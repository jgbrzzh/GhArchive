use crate::{
    db::{Store, Task},
    secrets, Failure, Result,
};
use base64::Engine;
use chrono::Utc;
use fs2::FileExt;
use futures::{stream, StreamExt};
use ghboost_core::GitOptions;
use rusqlite::params;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

pub fn identity(url: &str) -> Result<(String, String)> {
    let normalized = ghboost_core::prepare_git_url(url)?;
    let parsed = url::Url::parse(&normalized).map_err(|e| Failure::new(1, e))?;
    let parts: Vec<_> = parsed.path().trim_matches('/').split('/').collect();
    if parts.len() != 2 {
        return Err(Failure::new(1, "备份需要普通 GitHub owner/repo 仓库链接"));
    }
    let owner = parts[0].to_owned();
    let repo = parts[1].trim_end_matches(".git").to_owned();
    for name in [&owner, &repo] {
        let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
        if name.ends_with('.')
            || [
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9",
            ]
            .contains(&stem.as_str())
        {
            return Err(Failure::new(1, "仓库名称不能作为 Windows 目录名"));
        }
    }
    Ok((owner, repo))
}
fn reject_links(path: &Path) -> Result<()> {
    for p in path.ancestors() {
        if let Ok(m) = std::fs::symlink_metadata(p) {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if m.file_attributes() & 0x400 != 0 {
                    return Err(Failure::new(
                        2,
                        "备份路径不能经过 junction、符号链接或其他重解析点",
                    ));
                }
            }
            if m.file_type().is_symlink() {
                return Err(Failure::new(2, "备份路径不能经过符号链接"));
            }
        }
    }
    Ok(())
}
fn target(t: &Task, s: &Store) -> Result<PathBuf> {
    let (owner, repo) = identity(&t.input.repo_url)?;
    let root = PathBuf::from(&t.input.backup_dir);
    reject_links(&root)?;
    std::fs::create_dir_all(&root)?;
    let root = dunce::canonicalize(root)?;
    let parent = root.join(owner);
    reject_links(&parent)?;
    std::fs::create_dir_all(&parent)?;
    let p = parent.join(format!("{repo}.git"));
    reject_links(&p)?;
    if fs2::available_space(&parent)? < s.get("minimum_free_bytes")?.as_u64().unwrap() {
        return Err(Failure::new(2, "备份磁盘剩余空间不足"));
    }
    let probe = parent.join(format!(".gharchive-probe-{}", uuid::Uuid::new_v4()));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)?;
    f.write_all(b"write-test")?;
    f.sync_all()?;
    drop(f);
    std::fs::remove_file(probe)?;
    Ok(p)
}
fn config_entry(env: &mut HashMap<String, String>, key: &str, value: String) {
    let i = env
        .get("GIT_CONFIG_COUNT")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    env.insert(format!("GIT_CONFIG_KEY_{i}"), key.into());
    env.insert(format!("GIT_CONFIG_VALUE_{i}"), value);
    env.insert("GIT_CONFIG_COUNT".into(), (i + 1).to_string());
}
async fn prepare(
    t: &Task,
    s: &Store,
    token: Option<&str>,
) -> Result<(
    String,
    HashMap<String, String>,
    crate::acceleration::Connection,
)> {
    let mirror = s.get("mirror_url")?.as_str().unwrap().to_owned();
    if t.input.use_mirror && mirror.is_empty() {
        return Err(Failure::new(1, "请先在设置中配置镜像地址"));
    }
    let options = GitOptions {
        use_proxy: t.input.use_proxy,
        mirror: if t.input.use_mirror {
            Some(mirror)
        } else {
            None
        },
        private_repository: token.is_some(),
    };
    let url = ghboost_core::prepare_git_url_with_options(&t.input.repo_url, &options)?;
    if s.get("require_hosts")? == true {
        ghboost_core::ensure_hosts_applied()?;
    }
    let domain = url::Url::parse(&ghboost_core::prepare_git_url(&t.input.repo_url)?)
        .map_err(|e| Failure::new(1, e))?;
    let automatic = crate::acceleration::Connection::prepare(
        s,
        t.id,
        options.use_proxy && options.mirror.is_none(),
        domain.host_str().unwrap(),
    )
    .await?;
    let mut env = automatic.git_env(&options)?;
    // 限定每次命令的配置，避免旧的 insteadOf 或凭据助手向镜像传送 Token。
    env.insert("GIT_CONFIG_NOSYSTEM".into(), "1".into());
    env.insert(
        "GIT_CONFIG_GLOBAL".into(),
        if cfg!(windows) { "NUL" } else { "/dev/null" }.into(),
    );
    env.insert("GIT_TERMINAL_PROMPT".into(), "0".into());
    env.insert("GIT_ASKPASS".into(), String::new());
    env.insert("SSH_ASKPASS".into(), String::new());
    config_entry(&mut env, "credential.helper", String::new());
    config_entry(
        &mut env,
        "core.hooksPath",
        if cfg!(windows) { "NUL" } else { "/dev/null" }.into(),
    );
    config_entry(&mut env, "protocol.allow", "never".into());
    config_entry(&mut env, "protocol.https.allow", "always".into());
    config_entry(
        &mut env,
        "http.followRedirects",
        if token.is_some() { "false" } else { "initial" }.into(),
    );
    if let Some(token) = token {
        let value =
            base64::engine::general_purpose::STANDARD.encode(format!("x-access-token:{token}"));
        config_entry(
            &mut env,
            "http.extraHeader",
            format!("Authorization: Basic {value}"),
        );
    }
    Ok((url, env, automatic))
}
// 持续排空管道并截断保存内容，避免 Git 大量输出占满内存或阻塞。
async fn capture<R: AsyncRead + Unpin>(mut reader: R) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buf = [0; 8192];
    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        let take = n.min(1024 * 1024usize - output.len());
        output.extend_from_slice(&buf[..take]);
    }
    Ok(output)
}
async fn git(
    args: &[String],
    env: &HashMap<String, String>,
    timeout: u64,
    token: Option<&str>,
) -> Result<(i32, String, String)> {
    let mut c = Command::new("git");
    c.args(args)
        .envs(env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    // 取消继承的追踪和注入环境变量；认证只在子进程的配置环境中。
    for (k, _) in std::env::vars() {
        if k.starts_with("GIT_") || k == "SSH_ASKPASS" {
            c.env_remove(k);
        }
    }
    c.envs(env);
    #[cfg(windows)]
    c.creation_flags(0x08000000);
    let mut child = c.spawn()?;
    let stdout = tokio::spawn(capture(child.stdout.take().unwrap()));
    let stderr = tokio::spawn(capture(child.stderr.take().unwrap()));
    let code = match tokio::time::timeout(Duration::from_secs(timeout), child.wait()).await {
        Ok(r) => r?.code().unwrap_or(-1),
        Err(_) => {
            #[cfg(windows)]
            if let Some(pid) = child.id() {
                let _ = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .creation_flags(0x08000000)
                    .output()
                    .await;
            }
            let _ = child.kill().await;
            let _ = child.wait().await;
            -1
        }
    };
    let out = tokio::time::timeout(Duration::from_secs(5), stdout)
        .await
        .map_err(|_| Failure::new(5, "Git stdout 管道关闭超时"))?
        .map_err(|e| Failure::new(5, e))??;
    let err = tokio::time::timeout(Duration::from_secs(5), stderr)
        .await
        .map_err(|_| Failure::new(5, "Git stderr 管道关闭超时"))?
        .map_err(|e| Failure::new(5, e))??;
    Ok((
        code,
        secrets::redact(&String::from_utf8_lossy(&out), token),
        secrets::redact(&String::from_utf8_lossy(&err), token),
    ))
}
fn size(path: &Path) -> u64 {
    let Ok(m) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if m.file_type().is_symlink() {
        return 0;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if m.file_attributes() & 0x400 != 0 {
            return 0;
        }
    }
    if m.is_file() {
        return m.len();
    }
    std::fs::read_dir(path)
        .map(|it| it.filter_map(|e| e.ok()).map(|e| size(&e.path())).sum())
        .unwrap_or(0)
}
pub fn lock(s: &Store, id: i64) -> Result<File> {
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(s.dir.join("locks").join(format!("task-{id}.lock")))?;
    f.try_lock_exclusive()
        .map_err(|_| Failure::new(1, "该任务已有备份在运行"))?;
    Ok(f)
}
fn reserve(s: &Store, t: &Task) -> Result<Option<i64>> {
    let mut db = s.db()?;
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let active: u64 = tx.query_row(
        "SELECT COUNT(*) FROM backup_runs WHERE status='running'",
        [],
        |r| r.get(0),
    )?;
    if active >= s.get("concurrency")?.as_u64().unwrap() {
        return Ok(None);
    }
    let now = Utc::now().to_rfc3339();
    tx.execute(
        "INSERT INTO backup_runs(task_id,started_at,status) VALUES(?1,?2,'running')",
        params![t.id, now],
    )?;
    let id = tx.last_insert_rowid();
    let next = if t.input.enabled {
        Some(crate::db::next_time(&t.input.schedule_time, Utc::now())?)
    } else {
        None
    };
    tx.execute("UPDATE backup_tasks SET last_run_at=?2,last_status='running',last_error=NULL,next_run_at=?3 WHERE id=?1",params![t.id,now,next])?;
    tx.commit()?;
    Ok(Some(id))
}
struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
    bytes: u64,
    attempts: u32,
    error: Option<Failure>,
}
async fn perform(s: &Store, t: &Task) -> Outcome {
    let mut outcome = Outcome {
        code: -1,
        stdout: String::new(),
        stderr: String::new(),
        bytes: 0,
        attempts: 0,
        error: None,
    };
    let mut acceleration = None;
    let mut result: Result<()> = async {
        if s.get("accepted_notice")? != true {
            return Err(Failure::new(
                2,
                "请先阅读并接受安全声明（config set accepted_notice true）",
            ));
        }
        let token = secrets::load(s)?;
        let token = token.as_deref();
        let p = target(t, s)?;
        let (owner, repo) = identity(&t.input.repo_url)?;
        let directory_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(
                p.parent()
                    .unwrap()
                    .join(format!(".{owner}-{repo}.gharchive.lock")),
            )?;
        directory_lock
            .try_lock_exclusive()
            .map_err(|_| Failure::new(1, "此备份目录正在被另一个任务使用"))?;
        let retries = s.get("retries")?.as_u64().unwrap();
        let delay = s.get("retry_delay_seconds")?.as_u64().unwrap();
        let mut preparation_attempt = 0;
        let (url, env, automatic) = loop {
            outcome.attempts = preparation_attempt as u32 + 1;
            match prepare(t, s, token).await {
                Ok(connection) => break connection,
                Err(e) if e.exit == 3 && preparation_attempt < retries => {
                    outcome.stderr.push_str(&format!(
                        "\nGhBoost 准备第 {} 次失败：{}；{} 秒后重试\n",
                        preparation_attempt + 1,
                        e.message,
                        delay
                    ));
                    preparation_attempt += 1;
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                }
                Err(e) => return Err(e),
            }
        };
        if let Some(port) = automatic.port() {
            outcome.stdout.push_str(&format!(
                "GhBoost 内置加速：DNS/HTTPS 优选完成，任务代理 127.0.0.1:{port}\n"
            ));
        }
        acceleration = Some(automatic);
        let timeout = s.get("timeout_seconds")?.as_u64().unwrap();
        if p.exists() {
            let check = git(
                &[
                    "--git-dir".into(),
                    p.to_string_lossy().into(),
                    "rev-parse".into(),
                    "--is-bare-repository".into(),
                ],
                &env,
                timeout,
                token,
            )
            .await?;
            if check.0 != 0 || check.1.trim() != "true" {
                return Err(Failure::new(
                    1,
                    "目标目录已存在，但不是有效的 bare Git 仓库；未覆盖",
                ));
            }
            // 只更新 origin。禁止从用户额外配置的远程地址发送凭据。
            let remotes = git(
                &[
                    "--git-dir".into(),
                    p.to_string_lossy().into(),
                    "remote".into(),
                ],
                &env,
                timeout,
                token,
            )
            .await?;
            if remotes.0 != 0 || remotes.1.lines().any(|r| r != "origin") {
                return Err(Failure::new(
                    1,
                    "目标仓库包含非 origin 远程，请使用专用备份目录",
                ));
            }
        }
        for attempt in 0..=retries {
            outcome.attempts = attempt as u32 + 1;
            let stage = p
                .parent()
                .unwrap()
                .join(format!(".gharchive-clone-{}", uuid::Uuid::new_v4()));
            let commands = if p.exists() {
                let prefix = vec!["--git-dir".into(), p.to_string_lossy().into_owned()];
                [
                    vec![
                        "remote".into(),
                        "set-url".into(),
                        "origin".into(),
                        url.clone(),
                    ],
                    vec![
                        "fetch".into(),
                        "--all".into(),
                        "--prune".into(),
                        "--tags".into(),
                    ],
                    vec!["remote".into(), "update".into(), "--prune".into()],
                ]
                .into_iter()
                .map(|v| [prefix.clone(), v].concat())
                .collect::<Vec<_>>()
            } else {
                vec![vec![
                    "clone".into(),
                    "--mirror".into(),
                    "--".into(),
                    url.clone(),
                    stage.to_string_lossy().into_owned(),
                ]]
            };
            let mut success = true;
            for args in commands {
                outcome.stdout.push_str(&format!(
                    "\nAttempt {}: git {}\n",
                    attempt + 1,
                    args.join(" ")
                ));
                let (code, out, err) = git(&args, &env, timeout, token).await?;
                outcome.code = code;
                outcome.stdout.push_str(&out);
                outcome.stderr.push_str(&err);
                if code != 0 {
                    success = false;
                    break;
                }
            }
            if success {
                if stage.exists() {
                    std::fs::rename(&stage, &p)?;
                }
                outcome.bytes = size(&p);
                return Ok(());
            }
            // 仅清理本轮自己创建的临时 clone，绝不删除已完成的备份。
            if stage.exists() {
                reject_links(&stage)?;
                std::fs::remove_dir_all(&stage)?;
            }
            if attempt < retries {
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
        }
        outcome.bytes = size(&p);
        Err(Failure::new(
            3,
            if outcome.code == -1 {
                "Git 操作超时或被终止"
            } else {
                "Git 备份失败，请查看运行日志"
            },
        ))
    }
    .await;
    if let Some(automatic) = acceleration {
        if let Err(e) = automatic.close().await {
            outcome
                .stderr
                .push_str(&format!("\n内置 GhBoost 清理失败：{}\n", e.message));
            if result.is_ok() {
                result = Err(e);
            }
        }
    }
    if let Err(e) = result {
        outcome.error = Some(e);
    }
    // 日志容量有上限，stderr 始终保留末尾用于诊断。
    for text in [&mut outcome.stdout, &mut outcome.stderr] {
        if text.len() > 4 * 1024 * 1024 {
            let mut start = text.len() - 4 * 1024 * 1024;
            while !text.is_char_boundary(start) {
                start += 1;
            }
            *text = text[start..].into();
        }
    }
    outcome
}
pub async fn run(s: &Store, id: i64) -> Result<Value> {
    let _lock = lock(s, id)?;
    let t = s.task(id)?;
    let run_id = loop {
        if let Some(id) = reserve(s, &t)? {
            break id;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    let o = perform(s, &t).await;
    let now = Utc::now().to_rfc3339();
    let state = if o.error.is_none() {
        "success"
    } else {
        "failed"
    };
    let error = o.error.as_ref().map(|e| secrets::redact(&e.message, None));
    let mut db = s.db()?;
    let tx = db.transaction()?;
    tx.execute("UPDATE backup_runs SET finished_at=?2,exit_code=?3,stdout=?4,stderr=?5,size_bytes=?6,status=?7,error=?8,attempts=?9 WHERE id=?1",params![run_id,now,o.code,o.stdout,o.stderr,o.bytes,state,error,o.attempts])?;
    tx.execute(
        "UPDATE backup_tasks SET last_status=?2,last_error=?3,updated_at=?4 WHERE id=?1",
        params![id, state, error, now],
    )?;
    tx.commit()?;
    let log = json!({"run_id":run_id,"task_id":id,"timestamp":now,"status":state,"exit_code":o.code,"error":error,"stdout":o.stdout,"stderr":o.stderr,"size_bytes":o.bytes,"attempts":o.attempts});
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(s.dir.join("logs").join(format!(
            "backup-{}.log",
            chrono::Local::now().format("%Y-%m-%d")
        )))?;
    file.lock_exclusive()?;
    writeln!(file, "{log}")?;
    if let Some(e) = o.error {
        return Err(e);
    }
    Ok(log)
}
pub async fn run_all(s: &Store) -> Result<Value> {
    let tasks = s
        .tasks()?
        .into_iter()
        .filter(|t| t.input.enabled)
        .collect::<Vec<_>>();
    let results=stream::iter(tasks).map(|t|async move {let result=run(s,t.id).await;json!({"task_id":t.id,"success":result.is_ok(),"data":result.as_ref().ok(),"error":result.as_ref().err()})}).buffer_unordered(16).collect::<Vec<_>>().await;
    if results.iter().any(|r| r["success"] != true) {
        return Err(Failure::new(
            3,
            format!("部分任务失败；详情见 history。结果：{}", json!(results)),
        ));
    }
    Ok(json!(results))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn windows_path_boundary() {
        assert!(identity("o/CON").is_err());
        assert!(identity("o/r.").is_err());
        assert!(identity("o/repo").is_ok());
    }
    #[tokio::test]
    async fn failed_preflight_is_recorded() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_in(dir.path().into()).unwrap();
        let t = s
            .save_task(
                None,
                crate::db::TaskInput {
                    name: "test".into(),
                    repo_url: "o/r".into(),
                    backup_dir: dir.path().to_string_lossy().into(),
                    schedule_time: "03:00".into(),
                    enabled: true,
                    use_proxy: false,
                    use_mirror: false,
                    notes: String::new(),
                },
            )
            .unwrap();
        assert!(run(&s, t.id).await.is_err());
        let h = s.history(1, None).unwrap();
        assert_eq!(h[0]["status"], "failed");
        assert!(h[0]["finished_at"].is_string());
        assert_eq!(s.task(t.id).unwrap().last_status.as_deref(), Some("failed"));
    }
}
