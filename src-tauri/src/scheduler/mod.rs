use crate::{backup, db::Store, Failure, Result};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    time::Duration,
};
pub fn daemon_lock(s: &Store) -> Result<File> {
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(s.dir.join("scheduler.lock"))?;
    f.try_lock_exclusive()
        .map_err(|_| Failure::new(1, "该数据目录已有定时器常驻；请关闭另一实例"))?;
    Ok(f)
}
pub fn recover(s: &Store) -> Result<()> {
    for t in s.tasks()? {
        if t.last_status.as_deref() == Some("running") {
            if let Ok(_lock) = backup::lock(s, t.id) {
                let mut db = s.db()?;
                let tx = db.transaction()?;
                tx.execute("UPDATE backup_runs SET status='interrupted',finished_at=?2,error='上次进程异常结束，任务未完成',exit_code=-1 WHERE task_id=?1 AND status='running'",rusqlite::params![t.id,Utc::now().to_rfc3339()])?;
                tx.execute("UPDATE backup_tasks SET last_status='interrupted',last_error='上次进程异常结束' WHERE id=?1",[t.id])?;
                tx.commit()?;
            }
        }
    }
    Ok(())
}
pub fn due(s: &Store, now: DateTime<Utc>) -> Result<Vec<i64>> {
    if s.get("paused")? == true || s.get("accepted_notice")? != true {
        return Ok(vec![]);
    }
    Ok(s.tasks()?
        .into_iter()
        .filter(|t| {
            t.input.enabled
                && t.next_run_at.as_deref().is_some_and(|n| {
                    DateTime::parse_from_rfc3339(n).is_ok_and(|n| n.with_timezone(&Utc) <= now)
                })
        })
        .map(|t| t.id)
        .collect())
}
pub async fn serve(s: Store, _lock: File) {
    let mut timer = tokio::time::interval(Duration::from_secs(5));
    let mut pending = HashSet::new();
    let mut jobs = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            _=timer.tick()=>{
                if let Ok(ids)=due(&s,Utc::now()) {for id in ids {if pending.insert(id){let store=s.clone();jobs.spawn(async move {let _=backup::run(&store,id).await;id});}}}
            }
            Some(result)=jobs.join_next(),if !jobs.is_empty()=>{if let Ok(id)=result {pending.remove(&id);}else{pending.clear();}}
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overdue_and_pause() {
        let dir = tempfile::tempdir().unwrap();
        let s = Store::open_in(dir.path().into()).unwrap();
        s.set("accepted_notice", serde_json::json!(true)).unwrap();
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
        s.db()
            .unwrap()
            .execute(
                "UPDATE backup_tasks SET next_run_at='2000-01-01T00:00:00+00:00' WHERE id=?1",
                [t.id],
            )
            .unwrap();
        assert_eq!(due(&s, Utc::now()).unwrap(), vec![t.id]);
        s.set("paused", serde_json::json!(true)).unwrap();
        assert!(due(&s, Utc::now()).unwrap().is_empty());
        let _lock = daemon_lock(&s).unwrap();
        assert!(daemon_lock(&s).is_err());
    }
}
