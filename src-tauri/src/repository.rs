//! Repository identity and safe local paths, not an acceleration implementation.
use crate::{Failure, Result};

fn component(name: &str) -> Result<()> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if name.is_empty()
        || name.len() > 100
        || name == "."
        || name == ".."
        || name.ends_with('.')
        || name.starts_with('-')
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        || [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ]
        .contains(&stem.as_str())
    {
        return Err(Failure::new(
            1,
            "仓库路径包含不安全或无法作为 Windows 目录的名称",
        ));
    }
    Ok(())
}

pub fn is_github(normalized: &str) -> bool {
    url::Url::parse(normalized)
        .ok()
        .is_some_and(|u| u.host_str() == Some("github.com"))
}
pub fn key(normalized: &str) -> String {
    if is_github(normalized) {
        normalized.to_ascii_lowercase()
    } else {
        normalized.into()
    }
}
pub fn mirror_name(normalized: &str, repo: &str) -> String {
    if is_github(normalized) {
        return format!("{repo}.git");
    }
    use sha2::{Digest, Sha256};
    // Windows folds path case; distinct URLs must never share a mirror by accident.
    format!("{repo}-{:x}.git", Sha256::digest(normalized.as_bytes()))
}

pub fn normalize(input: &str) -> Result<String> {
    let input = input.trim();
    if !input.contains("://") {
        return ghboost_core::prepare_git_url(input);
    }
    let mut parsed = url::Url::parse(input).map_err(|_| Failure::new(1, "Git 地址格式无效"))?;
    if matches!(
        parsed.host_str(),
        Some(
            "github.com"
                | "raw.githubusercontent.com"
                | "codeload.github.com"
                | "api.github.com"
                | "gist.github.com"
                | "gist.githubusercontent.com"
        )
    ) {
        return ghboost_core::prepare_git_url(input);
    }
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(Failure::new(
            1,
            "其他 Git 服务需要不含凭据的 HTTPS .git 链接",
        ));
    }
    // Check raw segments too: URL parsing otherwise removes literal '..'.
    let raw = input
        .split_once("://")
        .unwrap()
        .1
        .split(['?', '#'])
        .next()
        .unwrap();
    if raw
        .split('/')
        .skip(1)
        .any(|p| p == "." || p == ".." || p.contains('%') || p.contains('\\'))
    {
        return Err(Failure::new(1, "Git 路径不能包含遍历、转义或反斜杠"));
    }
    let path = parsed.path().trim_end_matches('/').to_owned();
    if !path.ends_with(".git") {
        return Err(Failure::new(1, "其他 Git 服务请提供以 .git 结尾的仓库地址"));
    }
    parsed.set_path(&path);
    parsed.set_query(None);
    parsed.set_fragment(None);
    let normalized = parsed.to_string();
    identity_normalized(&normalized)?;
    Ok(normalized)
}

fn identity_normalized(normalized: &str) -> Result<(String, String)> {
    let parsed = url::Url::parse(normalized).map_err(|_| Failure::new(1, "Git 地址格式无效"))?;
    let mut parts: Vec<_> = parsed
        .path()
        .trim_matches('/')
        .split('/')
        .map(str::to_owned)
        .collect();
    let repo = parts.pop().unwrap_or_default();
    let repo = repo.strip_suffix(".git").unwrap_or(&repo).to_owned();
    component(&repo)?;
    for part in &parts {
        component(part)?;
    }
    if is_github(normalized) {
        if parts.len() != 1 {
            return Err(Failure::new(1, "备份需要普通 GitHub owner/repo 仓库链接"));
        }
    } else {
        let host = parsed.host_str().unwrap_or("");
        component(host)?;
        let host = if let Some(port) = parsed.port() {
            format!("{host}__port{port}")
        } else {
            host.into()
        };
        parts.insert(0, host);
    }
    Ok((parts.join("/"), repo))
}

pub fn identity(input: &str) -> Result<(String, String)> {
    identity_normalized(&normalize(input)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generic_https_hosts_namespaces_and_ports_stay_distinct() {
        let url = normalize("https://git.xeondev.com/LR/S.git?utm_source=chatgpt.com").unwrap();
        assert_eq!(url, "https://git.xeondev.com/LR/S.git");
        assert_eq!(
            identity(&url).unwrap(),
            ("git.xeondev.com/LR".into(), "S".into())
        );
        assert_eq!(
            identity("https://gitlab.com/group/subgroup/repo.git")
                .unwrap()
                .0,
            "gitlab.com/group/subgroup"
        );
        assert_eq!(
            identity("https://git.example.com:8443/repo.git").unwrap().0,
            "git.example.com__port8443"
        );
        assert_eq!(identity("o/r").unwrap(), ("o".into(), "r".into()));
        assert_ne!(
            mirror_name(&url, "S").to_ascii_lowercase(),
            mirror_name("https://git.xeondev.com/LR/s.git", "s").to_ascii_lowercase()
        );
    }
    #[test]
    fn rejects_credentials_protocols_and_unsafe_paths() {
        for input in [
            "http://git.example.com/o/r.git",
            "https://u:secret@git.example.com/o/r.git",
            "file:///tmp/r.git",
            "https://git.example.com/o/../r.git",
            "https://git.example.com/o/%2e%2e/r.git",
            "https://git.example.com/o/CON.git",
            "https://git.example.com/o/r",
            "https://git.example.com/o//r.git",
        ] {
            assert!(normalize(input).is_err(), "{input}");
        }
    }
}
