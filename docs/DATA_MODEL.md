# 数据模型

SQLite 迁移文件为 `src-tauri/migrations/001.sql`，开启 WAL，每个连接启用外键并使用 busy timeout。

`backup_tasks` 保存任务字段、备注、上次运行结果和下一次执行时间。用户输入时间为 HH:mm，本机时区计算后以 RFC3339 UTC 时间戳持久化。编辑任务重新计算下一次时间；定时到点开始执行后推进至次日，提前手动执行则保留今天尚未到点的计划。禁用后 next_run_at 为空。

`backup_runs` 包含开始/结束时间、Git 退出码、脱敏 stdout/stderr、镜像目录大小、状态、错误、尝试次数。状态为 running、success、failed、interrupted。每个任务只能有一个 running 行，通过部分唯一索引限制。重试合并为一次运行，尝试次数与命令输出保留。

`settings` 保存 JSON 值。Token 不在 SQLite 中；Windows DPAPI 密文独立放在 token.dpapi。读取配置只返回 token_configured，不返回 Token 内容。

备份目录写权限、最低空间、认证准备等前置步骤失败仍记录一次 failed 运行。异常退出的 running 行在下一次应用/CLI 启动时检查任务锁；已无活动进程则记为 interrupted。删除任务也删除其历史，不删除镜像目录。
