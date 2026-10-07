# GhBoost 核心接入

GhArchive 从 GhBoost 现有独立核心复制本地依赖，不修改 GhBoost。来源提交固定在 `ghboost-core.lock.json`，目录 `.deps/ghboost-core` 被 Git 忽略，不公开核心源码。

```powershell
./scripts/prepare-core.ps1 -GhBoostPath D:\Github\GhBoost
```

应用 `src-tauri/Cargo.toml` 通过 `ghboost-core = { path = "../.deps/ghboost-core" }` 引用。GUI、CLI、调度器的每个 Git 子进程共用备份模块，调用 `prepare_git_url_with_options`、`prepare_git_env_with_options`，需要已有 Hosts 时调用 `ensure_hosts_applied`（只读检查）。URL 验证也调用 `prepare_git_url`。

核心共享 GhBoost 数据与已开启代理，不自行启动 GhBoost、后台写 Hosts 或重新实现 DNS/加速。镜像必须单独启用，配置 Token 时禁止镜像。

Actions 通过 Secret `GHBOOST_READ_TOKEN` 获取固定提交；没有凭据时明确跳过 Rust 检查，指定版本打包则立即报错。核心源码和私有凭据不进入提交或构建产物。GPL 完整对应源码交付需由分发者另行安排，详见 README。
