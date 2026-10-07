# GhArchive

**把 GitHub 仓库按时备份到本地。** Windows 桌面应用，Tauri 2 / WebView2 + Vue 3 + Rust + SQLite，同时提供独立 CLI。GitHub 访问由 GhBoost 核心处理，GhArchive 不重新实现加速器。

[![Checks](https://github.com/jgbrzzh/GhArchive/actions/workflows/check.yml/badge.svg)](https://github.com/jgbrzzh/GhArchive/actions/workflows/check.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

[问题反馈](https://github.com/jgbrzzh/GhArchive/issues) · [版本构建](https://github.com/jgbrzzh/GhArchive/actions/workflows/release.yml) · [贡献说明](CONTRIBUTING.md) · [安全报告](SECURITY.md)

## 功能

- 每个仓库单独设置每日执行时间、目录、代理、镜像、备注和启用状态。
- 首次 `git clone --mirror`，随后更新 origin 并运行 `git fetch --all --prune --tags`、`git remote update --prune`。
- 默认最多同时运行 2 个任务；失败额外重试 3 次，每次间隔 30 秒。并发、重试和命令超时可配置。
- 仪表盘、任务管理、运行历史、JSON 导出、设置，以及右上角在默认浏览器打开的 GitHub 图标。
- 托盘常驻、关闭窗口隐藏、暂停/恢复定时、登录 Windows 后自启、启动隐藏。
- SQLite 与每日文件日志记录运行时间、退出码、stdout、stderr、大小和结果。
- GitHub Token 使用 Windows DPAPI 按当前用户加密保存，不回读明文、不写入仓库 URL；配置 Token 时禁止镜像。
- CLI 的成功、参数错误、帮助输出均支持统一 `--json` 封装。

## 使用前说明

仅备份有权访问的仓库。程序会按你配置的计划访问网络并写入本地目录，首次启动需接受说明。GhArchive 不静默修改系统 Hosts；需要 Hosts 时先在 GhBoost 中应用，再打开“要求已应用 Hosts”。

**这是 Git 仓库镜像备份**：包含分支、标签及 Git 对象。不会另外下载 Release 附件、Issues、讨论、Wiki 或 Git LFS 对象，也不是保留每一天快照的归档。远端删掉的引用可能随 prune 在下一次同步时消失，长期历史应另做文件快照。

## 依赖边界

GhArchive 应用源码公开，许可证为 **GPL-3.0-only**。GhBoost 核心仍为私有依赖，按项目所有者要求不收录到本公开仓库。核心复制到忽略的 `.deps/ghboost-core`，Cargo 通过路径引用它；具体来源提交固定在 [ghboost-core.lock.json](ghboost-core.lock.json)。本仓库没有私有 Token，也没有修改 GhBoost。

构建应用需要拥有该核心的合法访问权限。本公开仓库**无法在缺少核心的情况下独立构建 Rust 应用**。如果分发 GPL 打包程序，需要向接收者提供包含该核心在内的完整对应源码及许可证；这应由分发者安排。本工作流只上传构建产物，不自动创建或发布 GitHub Release。

## 本地开发

要求 Windows 10/11 x64、WebView2 Runtime、系统 Git、Node.js 24、Rust stable MSVC 和 Visual Studio Build Tools（Desktop development with C++，含 Windows SDK）。可用 `rustup default stable-x86_64-pc-windows-msvc` 选择工具链。

在 PowerShell 中：

```powershell
git clone https://github.com/jgbrzzh/GhArchive.git
Set-Location GhArchive
# 使用 ghboost-core.lock.json 指定提交的 GhBoost checkout；不修改原仓库。
./scripts/prepare-core.ps1 -GhBoostPath D:\Github\GhBoost
npm ci
npm run desktop
```

已有 `.deps/ghboost-core` 时可直接运行。若 GhBoost 工作区已更新到其他提交，可建立该固定提交的独立 clone，再传给 `-GhBoostPath`，不要重置正在开发的 GhBoost 工作区。

```powershell
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib --locked
cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked
cargo build --manifest-path src-tauri/Cargo.toml --no-default-features --bin gharchive --locked
./scripts/test-cli.ps1
```

GUI 使用 Windows 自带 WebView2 宿主，不启动 HTTP API 或 MCP 服务。`npm run dev` 只是前端开发服务器；浏览器预览不执行备份。安装后运行 `gharchive-desktop.exe`，命令行使用同目录的 `gharchive.exe`（可以将该目录加入 PATH）。

## CLI

首次 CLI 使用请阅读上述说明，然后接受：

```powershell
gharchive describe --json
gharchive config set accepted_notice true --json
gharchive add octocat/git-consortium --time 03:00 --dir D:\backups --json
gharchive list --json
gharchive run 1 --json
gharchive run-all --json
gharchive enable 1 --json
gharchive disable 1 --json
gharchive status --json
gharchive history --limit 20 --task-id 1 --json
gharchive config get --json
gharchive config set concurrency 2 --json
gharchive config set timeout_seconds 1800 --json
gharchive config set paused true --json
gharchive remove 1 --yes --json
gharchive --help --json
```

支持 `owner/repo`、HTTPS 仓库/网页链接、`git@github.com:owner/repo.git`。SSH 输入会转成 HTTPS，所以不使用 SSH 密钥。编辑任务使用 `gharchive edit <id> --input '<完整 TaskInput JSON>' --json`，字段见 `describe`；推荐 GUI 编辑，避免旧版 PowerShell 的引号传参问题。

私有仓库可在 GUI 填写 Token。CLI 请通过标准输入提供，避免把 Token 留在进程命令行或 PowerShell 历史中：

```powershell
$secret = Read-Host 'GitHub Token' -AsSecureString
$credential = New-Object System.Net.NetworkCredential('', $secret)
$credential.Password | gharchive config set token --stdin --json
$credential = $null
$secret = $null
# 清除：
'' | gharchive config set token --stdin --json
```

无 GUI 常驻：`gharchive daemon --json`，Ctrl+C 结束并输出最终 JSON。`add` 和 `run` 等一次性 CLI 命令不会启动后台定时器；定时需要 GUI 托盘或 daemon 持续运行，同一数据目录仅允许一个调度器。`autostart` 在 GUI 设置立即应用；CLI 改该设置后由下一次 GUI 启动同步至系统。

统一响应：

```json
{"success":true,"data":{},"error":null,"timestamp":"2026-01-01T00:00:00Z"}
```

| 退出码 | 含义 |
| --- | --- |
| 0 | 成功 |
| 1 | 参数错误或任务已在运行 |
| 2 | 权限不足、未接受说明或未确认删除 |
| 3 | Git 网络操作失败或超时 |
| 4 | 任务、文件或配置键不存在 |
| 5 | 内部错误 |

任务保存路径：`<根目录>/<owner>/<repo>.git`。删除任务需要 GUI 确认或 CLI `--yes`，同时移除该任务的运行历史；备份文件始终保留。没有删除备份的命令。

## 运行与存储

- 默认数据目录 `%LOCALAPPDATA%\GhArchive`，包含 `gharchive.db`、`token.dpapi`、`logs\backup-YYYY-MM-DD.log` 和任务锁。
- `GHARCHIVE_DATA_DIR` 可设置独立数据目录；`GHBOOST_DATA_DIR` 可选择 GhBoost 数据目录。
- 定时使用 Windows 本地时间，每 5 秒检查一次；重启会补跑过期任务一次，再安排下一日。电脑睡眠、退出和关机期间无法运行。
- “备份占用”是各任务最近成功运行所记录的大小之和；相同目标的多个任务会重复统计，不等同于实时全盘占用。
- 写入前检查目录权限和最低剩余空间（默认 100 MB），不是对完整仓库大小的估计。禁止 junction、符号链接和路径穿越。
- 活动任务与同一目标目录均加锁；默认并发数在同一数据目录的 GUI、CLI 和调度器之间共享。
- 超时按单个 Git 命令计算。Windows 超时时尝试终止 Git 子进程树；异常结束的运行在下次启动记为 interrupted。
- 更新只允许专用 bare 仓库及 origin 远程，拒绝其他远程地址，防止凭据泄露。Git 配置隔离，保留 TLS 证书校验。
- GhBoost 代理须先在 GhBoost 启动；未开启时核心按原站配置准备访问。镜像地址需你自行选择，程序不保证第三方服务可用。
- 每个 Git 输出管道最多保存 1 MB，每次运行累计日志最多保留 4 MB；历史界面显示最近 200 条，导出最多 1000 条。日志暂不自动轮转清理旧日期文件。

## Windows 打包

明确指定与三个版本文件一致的版本：

```powershell
./scripts/build.ps1 -Version 0.1.0
```

生成 `src-tauri/target/release/gharchive.exe`、`gharchive-desktop.exe`、`bundle/nsis/*-setup.exe` 和 `bundle/msi/*.msi`。两个安装器均包含 GUI 与 CLI。首次安装缺少 WebView2 时会下载 Runtime。程序未代码签名，Windows 可能显示 SmartScreen 提示。

## GitHub Actions：按需构建

普通 main 源码推送或 PR 只运行检查，文档改动不触发。只有以下情况执行 Windows GUI/CLI、NSIS EXE、MSI 打包：

1. 手动运行 **Explicit version Windows build**，填写准确版本；
2. 主动推送版本标签，例如 `v0.1.0`。

不会在初始化时创建标签，不自动发布 Release，不跑 OS/架构矩阵、不定时跑工作流；并发检查自动取消旧运行，构建产物仅保留 7 天。`v*` 标签会先校验格式及三个版本文件，不一致立即失败。

仓库管理员需在 **Settings → Secrets and variables → Actions** 添加 `GHBOOST_READ_TOKEN`，建议使用仅有 GhBoost 仓库 Contents read 权限的细粒度 Token。它只用于获取固定提交的私有依赖，Git checkout 不保存凭据。没有 Secret 时普通工作流明确注明 Rust 检查跳过、仍验证前端；指定版本打包立即报错。来自 fork 的 PR 不获得私有核心或 Secret。

注意：发布产物前应安排 GPL 对应源码交付；只上传二进制不替代许可证义务。

## 项目结构

```text
GhArchive/
├── .github/                  # 检查、按版本构建、Issue/PR 模板
├── .deps/ghboost-core/       # 本地私有核心，Git 忽略
├── docs/                    # 数据模型、验收记录
├── scripts/                 # 核心准备、版本检查、打包、CLI 验证
├── src/
│   ├── components/GitHubIcon.vue
│   ├── App.vue
│   ├── main.ts
│   └── style.css
├── src-tauri/
│   ├── capabilities/default.json
│   ├── icons/               # Windows 图标
│   ├── migrations/001.sql
│   ├── src/
│   │   ├── backup/mod.rs
│   │   ├── bin/gharchive.rs
│   │   ├── db/mod.rs
│   │   ├── scheduler/mod.rs
│   │   ├── cli.rs
│   │   ├── lib.rs
│   │   ├── main.rs
│   │   └── secrets.rs
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── build.rs
│   ├── installer-hooks.nsh
│   └── tauri.conf.json
├── ghboost-core.lock.json
├── package.json / package-lock.json
├── index.html / tsconfig.json / vite.config.ts
├── LICENSE / THIRD_PARTY_NOTICES.md
└── README.md / CHANGELOG.md / CONTRIBUTING.md / SECURITY.md / CODE_OF_CONDUCT.md
```

不提交 AGENTS.md、核心源码、Token、数据库、日志、node_modules、target、dist、安装包和个人配置。依赖锁文件与应用图标提交，保证应用侧版本可复现。

完整功能与验收边界见 [验收记录](docs/VALIDATION.md)。欢迎通过 Issue 报告可复现问题；请先脱敏日志与本机路径。
