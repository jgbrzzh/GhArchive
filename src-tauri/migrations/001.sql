PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS backup_tasks (
 id INTEGER PRIMARY KEY, name TEXT NOT NULL, repo_url TEXT NOT NULL,
 backup_dir TEXT NOT NULL, schedule_time TEXT NOT NULL, enabled INTEGER NOT NULL,
 use_proxy INTEGER NOT NULL, use_mirror INTEGER NOT NULL, notes TEXT NOT NULL DEFAULT '',
 last_run_at TEXT, last_status TEXT, last_error TEXT, next_run_at TEXT,
 created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS backup_runs (
 id INTEGER PRIMARY KEY, task_id INTEGER NOT NULL REFERENCES backup_tasks(id),
 started_at TEXT NOT NULL, finished_at TEXT, exit_code INTEGER,
 stdout TEXT NOT NULL DEFAULT '', stderr TEXT NOT NULL DEFAULT '',
 size_bytes INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL,
 error TEXT, attempts INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX IF NOT EXISTS one_active_run ON backup_runs(task_id) WHERE status='running';
CREATE INDEX IF NOT EXISTS run_history ON backup_runs(started_at);
