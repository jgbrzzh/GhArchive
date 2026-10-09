//! GhArchive owns the lifetime of a GhBoost core proxy, never system networking.
use crate::{db::Store, Failure, Result};
use ghboost_core::{network, proxy, store::Store as CoreStore};
use serde_json::Value;
use std::{collections::HashMap, net::TcpListener, time::Duration};
use tokio::task::JoinHandle;

pub struct Connection {
    pub store: CoreStore,
    proxy: Option<OwnedProxy>,
}

impl Connection {
    pub async fn prepare(s: &Store, task_id: i64, enabled: bool, domain: &str) -> Result<Self> {
        // This directory belongs exclusively to GhArchive. Do not reuse GhBoost's
        // global proxy/PAC state, or stop a proxy started by another application.
        let store = CoreStore::open_in(s.dir.join("ghboost").join(format!("task-{task_id}")))?;
        if !enabled {
            return Ok(Self { store, proxy: None });
        }
        network::validate_domain(domain)?;
        // The core validates and races routes on CONNECT, reuses healthy routes,
        // refreshes DNS and cools down failures. A full speedtest is diagnostic,
        // not a prerequisite: its failure must not prevent route fallback.
        let owned = OwnedProxy::start(store.clone()).await?;
        Ok(Self {
            store,
            proxy: Some(owned),
        })
    }

    pub fn port(&self) -> Option<u16> {
        self.proxy.as_ref().map(|p| p.port)
    }

    pub fn git_env(&self, options: &ghboost_core::GitOptions) -> Result<HashMap<String, String>> {
        let env = ghboost_core::prepare_git_env_with_options(&self.store, options)?;
        if let Some(port) = self.port() {
            let expected = format!("http://127.0.0.1:{port}");
            let count = env
                .get("GIT_CONFIG_COUNT")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let configured = (0..count).any(|i| {
                env.get(&format!("GIT_CONFIG_KEY_{i}")).map(String::as_str) == Some("http.proxy")
                    && env.get(&format!("GIT_CONFIG_VALUE_{i}")) == Some(&expected)
            });
            if !configured {
                return Err(Failure::new(3, "内置 GhBoost 代理已停止，未回退到直连"));
            }
        }
        Ok(env)
    }

    pub async fn close(mut self) -> Result<()> {
        if let Some(owned) = self.proxy.take() {
            owned.close().await?;
        }
        Ok(())
    }
}

struct OwnedProxy {
    store: CoreStore,
    port: u16,
    task: Option<JoinHandle<ghboost_core::Result<Value>>>,
}

impl OwnedProxy {
    async fn start(store: CoreStore) -> Result<Self> {
        if store.get("proxy_tls_mode")? != "tunnel" {
            return Err(Failure::new(
                2,
                "GhArchive 自动代理仅使用端到端 TLS；请检查任务核心配置",
            ));
        }
        if proxy::status(&store)?["running"] == true {
            return Err(Failure::new(1, "该任务的内置 GhBoost 代理已有实例运行"));
        }
        if store.dir.join("system-proxy.json").exists() {
            return Err(Failure::new(
                2,
                "内置核心目录包含外部系统代理状态，未启动或修改它",
            ));
        }
        let reservation = TcpListener::bind(("127.0.0.1", 0))?;
        let port = reservation.local_addr()?.port();
        drop(reservation);
        let core = store.clone();
        let mut owned = Self {
            store,
            port,
            task: Some(tokio::spawn(async move { proxy::serve(core, port).await })),
        };
        for _ in 0..100 {
            if owned.task.as_ref().unwrap().is_finished() {
                let result = owned
                    .task
                    .take()
                    .unwrap()
                    .await
                    .map_err(|e| Failure::new(5, e))?;
                result?;
                return Err(Failure::new(3, "内置 GhBoost 代理提前退出"));
            }
            if let Ok(state) = proxy::status(&owned.store) {
                if state["running"] == true
                    && state["port"].as_u64() == Some(port as u64)
                    && state["pid"].as_u64() == Some(std::process::id() as u64)
                {
                    return Ok(owned);
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Err(Failure::new(3, "内置 GhBoost 代理未在5秒内就绪"))
    }

    async fn close(mut self) -> Result<()> {
        // No PAC/Hosts configuration is ever created in our private core store.
        let stopped = tokio::time::timeout(Duration::from_secs(6), proxy::stop(&self.store)).await;
        match stopped {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(Failure::new(5, "内置 GhBoost 代理停止超时")),
        }
        let mut task = self.task.take().unwrap();
        match tokio::time::timeout(Duration::from_secs(2), &mut task).await {
            Ok(result) => {
                result.map_err(|e| Failure::new(5, e))??;
            }
            Err(_) => {
                task.abort();
                let _ = task.await;
                return Err(Failure::new(5, "内置 GhBoost 代理未结束"));
            }
        }
        Ok(())
    }
}

impl Drop for OwnedProxy {
    fn drop(&mut self) {
        // Cancellation/panic must not leave the loopback listener alive.
        // Stale state after abrupt exit is ignored by the core's process lock.
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ghboost_core::{prepare_git_env_with_options, GitOptions};

    #[tokio::test]
    async fn proxy_readiness_does_not_require_network_speedtest() {
        let temp = tempfile::tempdir().unwrap();
        let app = Store::open_in(temp.path().to_path_buf()).unwrap();
        let connection = tokio::time::timeout(
            Duration::from_secs(6),
            Connection::prepare(&app, 1, true, "github.com"),
        )
        .await
        .expect("starting the local listener must not wait for DNS/HTTPS probes")
        .unwrap();
        assert!(connection.port().is_some());
        let env = connection.git_env(&GitOptions::default()).unwrap();
        assert_eq!(
            env["GIT_CONFIG_VALUE_0"],
            format!("http://127.0.0.1:{}", connection.port().unwrap())
        );
        assert_eq!(
            connection.store.history(1, Some("github.com")).unwrap(),
            serde_json::json!([])
        );
        assert!(!connection.store.dir.join("system-proxy.json").exists());
        assert!(!connection.store.dir.join("loopback-hosts.json").exists());
        connection.close().await.unwrap();
    }

    #[tokio::test]
    async fn invalid_domain_cannot_start_acceleration() {
        let temp = tempfile::tempdir().unwrap();
        let app = Store::open_in(temp.path().to_path_buf()).unwrap();
        assert!(Connection::prepare(&app, 1, true, "example.com")
            .await
            .is_err());
        let core = CoreStore::open_in(app.dir.join("ghboost/task-1")).unwrap();
        assert_eq!(proxy::status(&core).unwrap()["running"], false);
    }

    #[tokio::test]
    async fn stopped_proxy_does_not_silently_fall_back_to_direct() {
        let temp = tempfile::tempdir().unwrap();
        let store = CoreStore::open_in(temp.path()).unwrap();
        let owned = OwnedProxy::start(store.clone()).await.unwrap();
        let connection = Connection {
            store: store.clone(),
            proxy: Some(owned),
        };
        proxy::stop(&store).await.unwrap();
        assert_eq!(
            connection.git_env(&GitOptions::default()).unwrap_err().exit,
            3
        );
        connection.close().await.unwrap();
    }

    #[tokio::test]
    async fn concurrent_proxies_are_owned_and_stopped_independently() {
        let temp = tempfile::tempdir().unwrap();
        let a = CoreStore::open_in(temp.path().join("a")).unwrap();
        let b = CoreStore::open_in(temp.path().join("b")).unwrap();
        let one = OwnedProxy::start(a.clone()).await.unwrap();
        let two = OwnedProxy::start(b.clone()).await.unwrap();
        assert_ne!(one.port, two.port);
        let env = prepare_git_env_with_options(&a, &GitOptions::default()).unwrap();
        assert_eq!(
            env["GIT_CONFIG_VALUE_0"],
            format!("http://127.0.0.1:{}", one.port)
        );
        let port = one.port;
        one.close().await.unwrap();
        assert_eq!(proxy::status(&a).unwrap()["running"], false);
        assert!(TcpListener::bind(("127.0.0.1", port)).is_ok());
        assert_eq!(proxy::status(&b).unwrap()["running"], true);
        two.close().await.unwrap();
        assert!(!a.dir.join("proxy-state.json").exists());
        assert!(!a.dir.join("system-proxy.json").exists());
        assert!(!b.dir.join("system-proxy.json").exists());
    }

    #[tokio::test]
    async fn cancellation_releases_listener_and_allows_restart() {
        let temp = tempfile::tempdir().unwrap();
        let store = CoreStore::open_in(temp.path()).unwrap();
        let owned = OwnedProxy::start(store.clone()).await.unwrap();
        let port = owned.port;
        drop(owned);
        for _ in 0..100 {
            if proxy::status(&store).unwrap()["running"] == false {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(proxy::status(&store).unwrap()["running"], false);
        assert!(TcpListener::bind(("127.0.0.1", port)).is_ok());
        OwnedProxy::start(store)
            .await
            .unwrap()
            .close()
            .await
            .unwrap();
    }
}
