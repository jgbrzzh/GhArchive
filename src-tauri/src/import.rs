//! Offline text recognition and atomic task import. Core owns URL normalization.
use crate::{
    db::{Store, TaskInput},
    Failure, Result,
};
use chrono::Utc;
use rusqlite::{params, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const MAX_REPOSITORIES: usize = 500;

#[derive(Debug, Serialize)]
pub struct Candidate {
    pub repo_url: String,
    pub name: String,
    pub exists: bool,
}
#[derive(Debug, Serialize)]
pub struct Rejected {
    pub link: String,
    pub reason: String,
}
#[derive(Debug, Serialize)]
pub struct Preview {
    pub repositories: Vec<Candidate>,
    pub duplicates: usize,
    pub rejected: Vec<Rejected>,
}
#[derive(Deserialize)]
pub struct ImportInput {
    pub repositories: Vec<String>,
    pub settings: TaskInput,
}
#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub created_ids: Vec<i64>,
    pub skipped: Vec<String>,
}

pub fn preview(s: &Store, text: &str) -> Result<Preview> {
    if text.len() > 1024 * 1024 {
        return Err(Failure::new(1, "粘贴文字不能超过 1 MiB"));
    }
    // Markdown escapes include ':' in user-pasted ChatGPT links. Preserve other text.
    let mut chars = text.chars().peekable();
    let mut plain = String::with_capacity(text.len());
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek().is_some_and(|c| c.is_ascii_punctuation()) {
            plain.push(chars.next().unwrap());
        } else {
            plain.push(c);
        }
    }
    let existing: HashSet<_> = s
        .tasks()?
        .into_iter()
        .map(|t| crate::repository::key(&t.input.repo_url))
        .collect();
    let mut seen = HashSet::new();
    let mut rejected = HashSet::new();
    let mut out = Preview {
        repositories: vec![],
        duplicates: 0,
        rejected: vec![],
    };
    for token in
        plain.split(|c: char| c.is_whitespace() || "[]()<>\"'`{}|,;，；。！？、：".contains(c))
    {
        let lower = token.to_ascii_lowercase();
        let start = ["https://", "http://", "ssh://", "git@github.com:"]
            .into_iter()
            .filter_map(|p| lower.find(p))
            .min();
        let candidate = if let Some(start) = start {
            token[start..].trim_end_matches(['.', '!', '?'])
        } else {
            // Bare owner/repo is accepted only as a complete standalone token.
            let parts: Vec<_> = token.split('/').collect();
            if parts.len() != 2
                || parts.iter().any(|p| {
                    p.is_empty()
                        || !p
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                })
            {
                continue;
            }
            token.trim_end_matches('.')
        };
        match crate::repository::normalize(candidate).and_then(|url| {
            let (_, name) = crate::backup::identity(&url)?;
            Ok((url, name))
        }) {
            Ok((repo_url, name)) => {
                let key = crate::repository::key(&repo_url);
                if !seen.insert(key.clone()) {
                    out.duplicates += 1;
                    continue;
                }
                if out.repositories.len() >= MAX_REPOSITORIES {
                    return Err(Failure::new(1, "每次最多识别 500 个仓库，请分批导入"));
                }
                out.repositories.push(Candidate {
                    exists: existing.contains(&key),
                    repo_url,
                    name,
                });
            }
            Err(e) => {
                // Do not echo credentials supplied in an invalid URL.
                let link = if let Ok(mut url) = url::Url::parse(candidate) {
                    let _ = url.set_username("");
                    let _ = url.set_password(None);
                    url.set_query(None);
                    url.set_fragment(None);
                    url.to_string()
                } else if candidate.contains('@') {
                    "无效地址（凭据信息已隐藏）".into()
                } else {
                    candidate.into()
                };
                let link = crate::secrets::redact(&link, None);
                if rejected.insert(link.clone()) {
                    out.rejected.push(Rejected {
                        link,
                        reason: e.message,
                    });
                }
            }
        }
    }
    Ok(out)
}

pub fn apply(s: &Store, input: ImportInput) -> Result<ImportResult> {
    if input.repositories.is_empty() || input.repositories.len() > MAX_REPOSITORIES {
        return Err(Failure::new(1, "请选择 1 至 500 个仓库"));
    }
    // Validate everything before writing; an invalid entry never leaves a partial batch.
    let tasks = input
        .repositories
        .into_iter()
        .map(|repo_url| {
            let mut task = input.settings.clone();
            task.repo_url = repo_url;
            task.name.clear();
            s.validate_task(task)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut db = s.db()?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut existing: HashSet<String> = {
        let mut query = tx.prepare("SELECT repo_url FROM backup_tasks")?;
        let rows = query.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .map(|u| crate::repository::key(&u))
            .collect()
    };
    let mut out = ImportResult {
        created_ids: vec![],
        skipped: vec![],
    };
    let now = Utc::now();
    for t in tasks {
        if !existing.insert(crate::repository::key(&t.repo_url)) {
            out.skipped.push(t.repo_url);
            continue;
        }
        let next = if t.enabled {
            Some(crate::db::next_time(&t.schedule_time, now)?)
        } else {
            None
        };
        tx.execute("INSERT INTO backup_tasks(name,repo_url,backup_dir,schedule_time,enabled,use_proxy,use_mirror,notes,next_run_at,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)", params![t.name,t.repo_url,t.backup_dir,t.schedule_time,t.enabled,t.use_proxy,t.use_mirror,t.notes,next,now.to_rfc3339()])?;
        out.created_ids.push(tx.last_insert_rowid());
    }
    tx.commit()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_pasted_repository_collection() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_in(dir.path().into()).unwrap();
        let p = preview(&s, include_str!("../testdata/import-links.md")).unwrap();
        assert_eq!(p.repositories.len(), 30);
        assert_eq!(p.duplicates, 30);
        assert_eq!(p.rejected.len(), 0);
        assert!(p
            .repositories
            .iter()
            .all(|r| r.repo_url.ends_with(".git") && !r.repo_url.contains('?')));
    }
    fn settings(s: &Store) -> TaskInput {
        TaskInput {
            name: "".into(),
            repo_url: "".into(),
            backup_dir: s.dir.join("backups").to_string_lossy().into(),
            schedule_time: "03:00".into(),
            enabled: false,
            use_proxy: true,
            use_mirror: false,
            notes: "batch".into(),
        }
    }
    #[test]
    fn escaped_markdown_and_mixed_text_are_normalized_and_deduplicated() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_in(dir.path().into()).unwrap();
        let text = r"正文[https\://github.com/Variante/endfield_research_kit.git]\(https\://github.com/Variante/endfield_research_kit.git?utm_source=chatgpt.com) Git@invalid
git@github.com:SQwatermark/endfield_research_kit.git
ssh://git@github.com/Dr-hydra/Better-Endfield.git
链接：https://github.com/Variante/Cpp2IL-Endfield/tree/main。 VARIANTE/endfield_research_kit
https://git.xeondev.com/LR/S.git http://github.com.evil.test/o/r.git https://user:secret@github.com/o/r.git";
        let p = preview(&s, text).unwrap();
        assert_eq!(p.repositories.len(), 5);
        assert_eq!(p.duplicates, 2);
        assert_eq!(p.rejected.len(), 2);
        assert!(!serde_json::to_string(&p).unwrap().contains("secret"));
        assert_eq!(
            p.repositories[0].repo_url,
            "https://github.com/Variante/endfield_research_kit.git"
        );
    }
    #[test]
    fn batch_is_atomic_and_reimport_skips_existing_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_in(dir.path().into()).unwrap();
        let run = |repos: Vec<&str>| {
            apply(
                &s,
                ImportInput {
                    repositories: repos.into_iter().map(str::to_owned).collect(),
                    settings: settings(&s),
                },
            )
        };
        assert!(run(vec!["octocat/Hello-World", "https://elsewhere.test/o/r"]).is_err());
        assert!(s.tasks().unwrap().is_empty());
        let out = run(vec![
            "octocat/Hello-World",
            "OCTOCAT/hello-world",
            "Variante/endfield_research_kit",
        ])
        .unwrap();
        assert_eq!(out.created_ids.len(), 2);
        assert_eq!(out.skipped.len(), 1);
        assert_eq!(
            run(vec!["octocat/Hello-World"]).unwrap().created_ids.len(),
            0
        );
        let p = preview(&s, "octocat/Hello-World").unwrap();
        assert!(p.repositories[0].exists);
        assert!(s
            .tasks()
            .unwrap()
            .iter()
            .all(|t| !t.input.enabled && t.input.use_proxy && t.input.notes == "batch"));
    }
    #[test]
    fn invalid_settings_and_limits_never_create_tasks() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_in(dir.path().into()).unwrap();
        let mut input = settings(&s);
        input.schedule_time = "25:00".into();
        assert!(apply(
            &s,
            ImportInput {
                repositories: vec!["o/r".into()],
                settings: input
            }
        )
        .is_err());
        assert!(preview(&s, &"x".repeat(1024 * 1024 + 1)).is_err());
        let text = (0..501)
            .map(|i| format!("o/r{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(preview(&s, &text).is_err());
        assert!(s.tasks().unwrap().is_empty());
    }
    #[test]
    fn other_host_paths_are_case_sensitive_and_keep_host_identity() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_in(dir.path().into()).unwrap();
        let p = preview(&s, "https://git.xeondev.com/LR/S.git https://git.xeondev.com/LR/s.git https://gitlab.com/LR/S.git").unwrap();
        assert_eq!(p.repositories.len(), 3);
        let out = apply(
            &s,
            ImportInput {
                repositories: p.repositories.into_iter().map(|r| r.repo_url).collect(),
                settings: settings(&s),
            },
        )
        .unwrap();
        assert_eq!(out.created_ids.len(), 3);
        assert!(s
            .tasks()
            .unwrap()
            .iter()
            .all(|t| !t.input.use_proxy && !t.input.use_mirror));
    }
}
