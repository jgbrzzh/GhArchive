use crate::{Failure, Result};
use chrono::{DateTime, Local, NaiveTime, TimeZone, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Store {
    pub dir: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskInput {
    pub name: String,
    pub repo_url: String,
    pub backup_dir: String,
    pub schedule_time: String,
    pub enabled: bool,
    pub use_proxy: bool,
    pub use_mirror: bool,
    #[serde(default)]
    pub notes: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    #[serde(flatten)]
    pub input: TaskInput,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
pub fn next_time(time: &str, now: DateTime<Utc>) -> Result<String> {
    if time.len() != 5 || time.as_bytes()[2] != b':' {
        return Err(Failure::new(1, "时间必须为 HH:mm"));
    }
    let t = NaiveTime::parse_from_str(time, "%H:%M")
        .map_err(|_| Failure::new(1, "时间必须为 HH:mm（00:00 至 23:59）"))?;
    let today = now.with_timezone(&Local).date_naive();
    for days in 0..3 {
        let date = today + chrono::Duration::days(days);
        if let Some(candidate) = Local.from_local_datetime(&date.and_time(t)).earliest() {
            if candidate.with_timezone(&Utc) > now {
                return Ok(candidate.with_timezone(&Utc).to_rfc3339());
            }
        }
    }
    Err(Failure::new(5, "无法计算下次执行时间"))
}
fn row_task(r: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: r.get(0)?,
        input: TaskInput {
            name: r.get(1)?,
            repo_url: r.get(2)?,
            backup_dir: r.get(3)?,
            schedule_time: r.get(4)?,
            enabled: r.get(5)?,
            use_proxy: r.get(6)?,
            use_mirror: r.get(7)?,
            notes: r.get(8)?,
        },
        last_run_at: r.get(9)?,
        last_status: r.get(10)?,
        last_error: r.get(11)?,
        next_run_at: r.get(12)?,
        created_at: r.get(13)?,
        updated_at: r.get(14)?,
    })
}
impl Store {
    pub fn open() -> Result<Self> {
        Self::open_in(
            std::env::var_os("GHARCHIVE_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    dirs::data_local_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join("GhArchive")
                }),
        )
    }
    pub fn open_in(dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(dir.join("logs"))?;
        std::fs::create_dir_all(dir.join("locks"))?;
        let s = Self { dir };
        let db = s.db()?;
        db.execute_batch(include_str!("../../migrations/001.sql"))?;
        for (k, v) in [
            (
                "backup_root",
                json!(dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("GhArchive-backups")),
            ),
            ("concurrency", json!(2)),
            ("timeout_seconds", json!(1800)),
            ("retries", json!(3)),
            ("retry_delay_seconds", json!(30)),
            ("minimum_free_bytes", json!(104857600u64)),
            ("mirror_url", json!("")),
            ("require_hosts", json!(false)),
            ("paused", json!(false)),
            ("autostart", json!(false)),
            ("start_minimized", json!(false)),
            ("theme", json!("dark")),
            ("accepted_notice", json!(false)),
            ("auto_check_updates", json!(true)),
            ("update_last_check", json!(0)),
        ] {
            db.execute(
                "INSERT OR IGNORE INTO settings VALUES(?1,?2)",
                params![k, v.to_string()],
            )?;
        }
        Ok(s)
    }
    pub fn db(&self) -> Result<Connection> {
        let db = Connection::open(self.dir.join("gharchive.db"))?;
        db.busy_timeout(std::time::Duration::from_secs(15))?;
        db.execute_batch("PRAGMA foreign_keys=ON;")?;
        Ok(db)
    }
    pub fn get(&self, key: &str) -> Result<Value> {
        if key == "token" {
            return Ok(json!({"configured":crate::secrets::load(self)?.is_some()}));
        }
        let v: Option<String> = self
            .db()?
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?;
        serde_json::from_str(&v.ok_or_else(|| Failure::new(4, "配置键不存在"))?).map_err(Into::into)
    }
    pub fn config(&self) -> Result<Value> {
        let db = self.db()?;
        let mut q = db.prepare("SELECT key,value FROM settings ORDER BY key")?;
        let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = serde_json::Map::new();
        for row in rows {
            let (k, v) = row?;
            out.insert(k, serde_json::from_str(&v)?);
        }
        out.insert(
            "token_configured".into(),
            json!(crate::secrets::load(self)?.is_some()),
        );
        Ok(Value::Object(out))
    }
    pub fn set(&self, key: &str, value: Value) -> Result<Value> {
        if key == "token" {
            crate::secrets::save(
                self,
                value
                    .as_str()
                    .ok_or_else(|| Failure::new(1, "Token 必须是字符串"))?,
            )?;
            return self.get(key);
        }
        let valid = match key {
            "backup_root" => value
                .as_str()
                .is_some_and(|s| PathBuf::from(s).is_absolute()),
            "concurrency" => value.as_u64().is_some_and(|n| (1..=16).contains(&n)),
            "timeout_seconds" => value.as_u64().is_some_and(|n| (1..=86400).contains(&n)),
            "retries" => value.as_u64().is_some_and(|n| n <= 10),
            "retry_delay_seconds" => value.as_u64().is_some_and(|n| (1..=3600).contains(&n)),
            "minimum_free_bytes" => value.as_u64().is_some(),
            "mirror_url" => value.as_str().is_some_and(|s| {
                s.is_empty()
                    || ghboost_core::network::convert(
                        "https://github.com/octocat/Hello-World.git",
                        "clone",
                        Some(s),
                    )
                    .is_ok()
            }),
            "require_hosts" | "paused" | "autostart" | "start_minimized" | "accepted_notice"
            | "auto_check_updates" => value.is_boolean(),
            "theme" => matches!(value.as_str(), Some("dark" | "light" | "system")),
            _ => false,
        };
        if !valid {
            return Err(Failure::new(1, format!("配置键或值无效: {key}")));
        }
        self.db()?.execute(
            "UPDATE settings SET value=?2 WHERE key=?1",
            params![key, value.to_string()],
        )?;
        Ok(value)
    }
    pub fn tasks(&self) -> Result<Vec<Task>> {
        let db = self.db()?;
        let mut q = db.prepare("SELECT * FROM backup_tasks ORDER BY id DESC")?;
        let rows = q.query_map([], row_task)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
    pub fn task(&self, id: i64) -> Result<Task> {
        self.db()?
            .query_row("SELECT * FROM backup_tasks WHERE id=?1", [id], row_task)
            .optional()?
            .ok_or_else(|| Failure::new(4, "任务不存在"))
    }
    pub(crate) fn validate_task(&self, mut t: TaskInput) -> Result<TaskInput> {
        t.repo_url = crate::repository::normalize(&t.repo_url)?;
        crate::backup::identity(&t.repo_url)?;
        if !crate::repository::is_github(&t.repo_url) {
            t.use_proxy = false;
            t.use_mirror = false;
        }
        if t.name.trim().is_empty() {
            t.name = t
                .repo_url
                .rsplit('/')
                .next()
                .unwrap_or("repository")
                .trim_end_matches(".git")
                .into();
        }
        if t.name.len() > 200 || t.notes.len() > 10000 {
            return Err(Failure::new(1, "名称或备注过长"));
        }
        if t.backup_dir.is_empty() {
            t.backup_dir = self.get("backup_root")?.as_str().unwrap().into();
        }
        if !PathBuf::from(&t.backup_dir).is_absolute() {
            return Err(Failure::new(1, "备份目录必须使用绝对路径"));
        }
        next_time(&t.schedule_time, Utc::now())?;
        Ok(t)
    }
    pub fn save_task(&self, id: Option<i64>, t: TaskInput) -> Result<Task> {
        let _lock = id.map(|id| crate::backup::lock(self, id)).transpose()?;
        let t = self.validate_task(t)?;
        let next = next_time(&t.schedule_time, Utc::now())?;
        let next = if t.enabled { Some(next) } else { None };
        let now = Utc::now().to_rfc3339();
        let db = self.db()?;
        let id = if let Some(id) = id {
            if self.task(id)?.last_status.as_deref() == Some("running") {
                return Err(Failure::new(1, "任务运行期间不能编辑"));
            }
            db.execute("UPDATE backup_tasks SET name=?2,repo_url=?3,backup_dir=?4,schedule_time=?5,enabled=?6,use_proxy=?7,use_mirror=?8,notes=?9,next_run_at=?10,updated_at=?11 WHERE id=?1",params![id,t.name,t.repo_url,t.backup_dir,t.schedule_time,t.enabled,t.use_proxy,t.use_mirror,t.notes,next,now])?;
            id
        } else {
            db.execute("INSERT INTO backup_tasks(name,repo_url,backup_dir,schedule_time,enabled,use_proxy,use_mirror,notes,next_run_at,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)",params![t.name,t.repo_url,t.backup_dir,t.schedule_time,t.enabled,t.use_proxy,t.use_mirror,t.notes,next,now])?;
            db.last_insert_rowid()
        };
        self.task(id)
    }
    pub fn enable(&self, id: i64, enabled: bool) -> Result<Task> {
        let mut t = self.task(id)?.input;
        t.enabled = enabled;
        self.save_task(Some(id), t)
    }
    pub fn remove(&self, id: i64, confirmed: bool) -> Result<Value> {
        if !confirmed {
            return Err(Failure::new(
                2,
                "删除任务需要二次确认或 --yes；备份文件不会删除",
            ));
        }
        let _lock = crate::backup::lock(self, id)?;
        let task = self.task(id)?;
        if task.last_status.as_deref() == Some("running") {
            return Err(Failure::new(1, "任务正在运行"));
        }
        let mut db = self.db()?;
        let tx = db.transaction()?;
        tx.execute("DELETE FROM backup_runs WHERE task_id=?1", [id])?;
        tx.execute("DELETE FROM backup_tasks WHERE id=?1", [id])?;
        tx.commit()?;
        Ok(json!({"removed":id,"backups_preserved":true}))
    }
    pub fn history(&self, limit: u32, task_id: Option<i64>) -> Result<Value> {
        let db = self.db()?;
        let mut q=db.prepare("SELECT id,task_id,started_at,finished_at,exit_code,stdout,stderr,size_bytes,status,error,attempts FROM backup_runs WHERE (?1 IS NULL OR task_id=?1) ORDER BY id DESC LIMIT ?2")?;
        let rows=q.query_map(params![task_id,limit.clamp(1,1000)],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"task_id":r.get::<_,i64>(1)?,"started_at":r.get::<_,String>(2)?,"finished_at":r.get::<_,Option<String>>(3)?,"exit_code":r.get::<_,Option<i32>>(4)?,"stdout":r.get::<_,String>(5)?,"stderr":r.get::<_,String>(6)?,"size_bytes":r.get::<_,u64>(7)?,"status":r.get::<_,String>(8)?,"error":r.get::<_,Option<String>>(9)?,"attempts":r.get::<_,u32>(10)?})))?;
        Ok(json!(rows.collect::<std::result::Result<Vec<_>, _>>()?))
    }
    pub fn status(&self) -> Result<Value> {
        let tasks = self.tasks()?;
        let db = self.db()?;
        let today = Local::now().date_naive();
        let mut q = db.prepare("SELECT finished_at FROM backup_runs WHERE status='success'")?;
        let rows = q.query_map([], |r| r.get::<_, String>(0))?;
        let successes = rows
            .filter_map(|r| r.ok())
            .filter(|s| {
                DateTime::parse_from_rfc3339(s)
                    .is_ok_and(|t| t.with_timezone(&Local).date_naive() == today)
            })
            .count();
        let size:u64=db.query_row("SELECT COALESCE(SUM(size_bytes),0) FROM backup_runs WHERE id IN (SELECT MAX(id) FROM backup_runs WHERE status='success' GROUP BY task_id)",[],|r|r.get(0))?;
        let next = tasks.iter().filter_map(|t| t.next_run_at.as_ref()).min();
        let running: u32 = db.query_row(
            "SELECT COUNT(*) FROM backup_runs WHERE status='running'",
            [],
            |r| r.get(0),
        )?;
        Ok(
            json!({"task_count":tasks.len(),"enabled_count":tasks.iter().filter(|t|t.input.enabled).count(),"today_success":successes,"size_bytes":size,"next_run_at":next,"running":running,"paused":self.get("paused")?,"data_dir":self.dir,"notice":crate::NOTICE}),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_and_schedule() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_in(dir.path().into()).unwrap();
        let t = s
            .save_task(
                None,
                TaskInput {
                    name: "test".into(),
                    repo_url: "octocat/Hello-World".into(),
                    backup_dir: dir.path().to_string_lossy().into(),
                    schedule_time: "03:00".into(),
                    enabled: true,
                    use_proxy: false,
                    use_mirror: false,
                    notes: "note".into(),
                },
            )
            .unwrap();
        assert!(t.next_run_at.is_some());
        assert_eq!(s.tasks().unwrap().len(), 1);
        assert!(s.remove(t.id, false).is_err());
        assert!(!s.enable(t.id, false).unwrap().input.enabled);
        assert!(next_time("24:00", Utc::now()).is_err());
        assert!(next_time("3:00", Utc::now()).is_err());
        s.remove(t.id, true).unwrap();
        assert!(s.task(t.id).is_err());
    }
}
