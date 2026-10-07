//! Signed desktop updates; backup and installer processes share an activity lock.
use crate::{db::Store, Failure, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};

pub fn activity_lock(s: &Store, install: bool) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(s.dir.join("locks/update-activity.lock"))?;
    let result = if install {
        file.try_lock_exclusive()
    } else {
        FileExt::try_lock_shared(&file)
    };
    result.map_err(|_| {
        Failure::new(
            1,
            if install {
                "备份任务正在运行或排队，请完成后安装更新"
            } else {
                "正在安装更新，请稍后运行备份"
            },
        )
    })?;
    Ok(file)
}

pub fn allowed_download(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url
            .path()
            .starts_with("/jgbrzzh/GhArchive/releases/download/")
}

pub fn check_due(s: &Store, now: u64) -> Result<bool> {
    let last = s.get("update_last_check")?.as_u64().unwrap_or(0);
    Ok(s.get("auto_check_updates")? == true && (last == 0 || now.saturating_sub(last) >= 86400))
}

#[cfg(feature = "desktop")]
pub async fn action<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    s: &Store,
    install: bool,
    force: bool,
) -> Result<serde_json::Value> {
    use crate::acceleration::Connection;
    use serde_json::json;
    use std::time::Duration;
    use tauri::Emitter;
    use tauri_plugin_updater::UpdaterExt;
    static UPDATE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _serial = UPDATE_LOCK
        .try_lock()
        .map_err(|_| Failure::new(1, "已有更新操作正在进行"))?;
    if s.get("accepted_notice")? != true {
        return Err(Failure::new(2, "请先接受首次运行说明"));
    }
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    if !install && !force && !check_due(s, now)? {
        return Ok(json!({"checked":false}));
    }
    let _activity = if install {
        Some(activity_lock(s, true)?)
    } else {
        None
    };
    if install && s.status()?["running"].as_u64().unwrap_or(0) > 0 {
        return Err(Failure::new(1, "仍有备份任务运行，请完成后安装更新"));
    }
    // Throttle automatic attempts, including unavailable releases and network failures.
    s.db()?.execute(
        "UPDATE settings SET value=?1 WHERE key='update_last_check'",
        [now.to_string()],
    )?;
    let connection = Connection::prepare(s, 0, true, "github.com").await?;
    let result: Result<_> = async {
        let env = connection.git_env(&ghboost_core::GitOptions::default())?;
        let proxy = url::Url::parse(&env["GIT_CONFIG_VALUE_0"]).map_err(|e| Failure::new(5, e))?;
        let updater = app.updater_builder().proxy(proxy).timeout(Duration::from_secs(30))
            .build().map_err(|e| Failure::new(5, e))?;
        let update = updater.check().await.map_err(|e| Failure::new(3, format!("检查更新失败（可能尚未发布更新）：{e}")))?;
        if let Some(update) = update {
            if !allowed_download(&update.download_url) { return Err(Failure::new(2, "更新下载地址不属于 GhArchive 官方 Release")); }
            if !install { return Ok((json!({"checked":true,"available":true,"version":update.version,"notes":update.body}), None)); }
            let mut downloaded = 0u64;
            let data = tokio::time::timeout(Duration::from_secs(240), update.download(
                |chunk, total| { downloaded += chunk as u64; let _ = app.emit("update-progress", json!({"phase":"download","downloaded":downloaded,"total":total})); },
                || { let _ = app.emit("update-progress", json!({"phase":"verify"})); }
            )).await.map_err(|_| Failure::new(3, "更新下载超时，请稍后重试"))?
                .map_err(|e| Failure::new(3, format!("更新下载或签名验证失败：{e}")))?;
            Ok((json!({"checked":true,"available":true}), Some((update, data))))
        } else { Ok((json!({"checked":true,"available":false}), None)) }
    }.await;
    // Windows updater exits the process; stop our proxy before launching installer.
    let closed = connection.close().await;
    let (response, install_data) = result?;
    closed?;
    if let Some((update, data)) = install_data {
        let _ = app.emit("update-progress", json!({"phase":"install"}));
        update
            .install(data)
            .map_err(|e| Failure::new(5, format!("启动更新安装失败：{e}")))?;
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configured_key_accepts_signed_data_and_rejects_tampering() {
        use base64::Engine;
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let key_text = base64::engine::general_purpose::STANDARD
            .decode(config["plugins"]["updater"]["pubkey"].as_str().unwrap())
            .unwrap();
        let sig_text = base64::engine::general_purpose::STANDARD
            .decode(include_str!("../testdata/update-fixture.txt.sig").trim())
            .unwrap();
        let key =
            minisign_verify::PublicKey::decode(std::str::from_utf8(&key_text).unwrap()).unwrap();
        let sig =
            minisign_verify::Signature::decode(std::str::from_utf8(&sig_text).unwrap()).unwrap();
        let original = include_bytes!("../testdata/update-fixture.txt");
        key.verify(original, &sig, false).unwrap();
        let mut changed = original.to_vec();
        changed[0] ^= 1;
        assert!(key.verify(&changed, &sig, false).is_err());
    }
    #[test]
    fn installer_excludes_backups_across_handles() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_in(dir.path().into()).unwrap();
        let a = activity_lock(&s, false).unwrap();
        let b = activity_lock(&s, false).unwrap();
        assert!(activity_lock(&s, true).is_err());
        drop(a);
        drop(b);
        let install = activity_lock(&s, true).unwrap();
        assert!(activity_lock(&s, false).is_err());
        drop(install);
        assert!(activity_lock(&s, false).is_ok());
    }
    #[test]
    fn update_checks_are_throttled_and_optional() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_in(dir.path().into()).unwrap();
        assert!(check_due(&s, 100000).unwrap());
        s.db()
            .unwrap()
            .execute(
                "UPDATE settings SET value='100000' WHERE key='update_last_check'",
                [],
            )
            .unwrap();
        assert!(!check_due(&s, 100001).unwrap());
        assert!(check_due(&s, 186400).unwrap());
        s.set("auto_check_updates", serde_json::json!(false))
            .unwrap();
        assert!(!check_due(&s, 200000).unwrap());
    }
    #[test]
    fn downloads_are_restricted_to_official_https_releases() {
        assert!(allowed_download(
            &url::Url::parse(
                "https://github.com/jgbrzzh/GhArchive/releases/download/v1.0.0/setup.exe"
            )
            .unwrap()
        ));
        for bad in [
            "http://github.com/jgbrzzh/GhArchive/releases/download/v1.0.0/a.exe",
            "https://github.com/other/GhArchive/releases/download/v1.0.0/a.exe",
            "https://github.com.evil.test/jgbrzzh/GhArchive/releases/download/a.exe",
            "https://user@github.com/jgbrzzh/GhArchive/releases/download/a.exe",
        ] {
            assert!(!allowed_download(&url::Url::parse(bad).unwrap()));
        }
    }
}
