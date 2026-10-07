# GhBoost 核心接入

GhArchive 从 GhBoost 现有独立核心复制本地依赖，不修改 GhBoost。来源提交固定在 `ghboost-core.lock.json`，目录 `.deps/ghboost-core` 被 Git 忽略，不公开核心源码。

```powershell
./scripts/prepare-core.ps1 -GhBoostPath D:\Github\GhBoost
```

应用 `src-tauri/Cargo.toml` 通过 `ghboost-core = { path = "../.deps/ghboost-core" }` 引用。GUI、CLI、调度器的每个 Git 子进程共用备份模块，调用 `prepare_git_url_with_options`、`prepare_git_env_with_options`，需要已有 Hosts 时调用 `ensure_hosts_applied`（只读检查）。URL 验证也调用 `prepare_git_url`。

默认任务通过 `src-tauri/src/acceleration.rs` 调用核心 `network::speedtest`（DNS/HTTPS 优选）和 `proxy::serve`（本机 CONNECT 代理），再调用核心 URL/环境接口执行 Git。无需外部 GhBoost 软件运行。这里只管理现有核心的生命周期，不重新实现 DNS、路由或加速算法。

每个任务的核心状态存放在 GhArchive 数据目录的 `ghboost/task-<id>/`，监听随机本机端口，不借用外部 GhBoost 的状态，不设置系统 PAC、系统代理或 Hosts。正常结束通过核心 `proxy::stop` 关闭并等待结束；取消时终止拥有的代理任务，核心文件锁使残留状态不会被误认为仍在运行。检查生成的 Git 环境，防止代理意外停止后静默转成直连。

关闭任务加速或选择镜像时不启动本机代理，但仍使用核心 URL/环境接口。镜像必须单独启用，配置 Token 时禁止镜像。

Actions 通过 Secret `GHBOOST_READ_TOKEN` 获取固定提交；没有凭据时明确跳过 Rust 检查，指定版本打包则立即报错。核心源码和私有凭据不进入提交或构建产物。GPL 完整对应源码交付需由分发者另行安排，详见 README。
