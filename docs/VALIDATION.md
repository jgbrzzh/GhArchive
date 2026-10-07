# 验收记录

日期：2026-10-07，北京时间。平台：Windows x64，Rust 1.98.1、Node.js 24.19.0，应用版本 0.1.0。

当前依赖已同步到 GhBoost 远端提交 `00710891d5cc882681538167be060e7e48707aed`，核心 0.1.1，包含代理路由 TLS 验证。核心副本仅在忽略的 `.deps` 内，GhArchive 未改动 GhBoost。默认任务现已自动调用核心 DNS/HTTPS 优选和任务专用代理，不依赖外部 GhBoost 软件，不修改系统 PAC/代理或 Hosts。

下面的真实网络和 GUI/托盘验收记录来自首轮 `e289a0c00f23bb547fa26e374b6fb012ec0dcf20` 构建；新核心的构建与 CLI 复验单独记录，不能据此声称重新操作了 GUI 或重启电脑。

## `00710891` 内置核心自动调用复验

- `cargo test --locked` 的 10 个库测试、CLI/GUI 目标与 doc-tests 通过；代理并发独立停止、取消后释放端口并重启、代理意外停止后不静默直连三个新增测试通过。
- 最终本地 `scripts/build.ps1 -Version 0.1.0` 完成 release CLI、Vue/TypeScript、Tauri GUI、NSIS EXE 和 MSI 构建。首次构建曾因验收 CLI 占用待覆盖 exe 失败，结束测试并改用独立测试副本后重新构建通过。没有触发云端版本打包。
- 普通 CLI 默认加速配置下，21:26:06 开始、21:28:17 完成 `Variante/endfield_research_kit` 首次 mirror clone，一次成功、退出码 0。21:28:33–21:28:39 的增量 fetch/remote update 同样一次成功，57,850,468 字节（约 55.2 MiB）。无需诊断助手、外部 GhBoost 软件或 GHBOOST_DATA_DIR。
- 两轮 stdout 均记录了内置优选和任务代理，端口分别为 3977、9759；结束后端口无监听，proxy-state.json 已移除，无 system-proxy.json。Git `fsck --full` 通过，HEAD 为 `717c6a56cb3198d5bb7ed0588dd34ff53293bd55`。
- 最终 release CLI daemon 于 21:31:03 按临近时间自动触发 `octocat/git-consortium`，21:31:09 完成首次镜像（83,208 字节），一次成功，日志记录内置核心代理 11892。每日 03:00 任务启用时正确计算为 2026-10-08 03:00；没有等待到凌晨。
- 21:32:30–21:32:36，最终 release CLI `run-all` 同时运行上述两个任务，两个独立代理端口 14511、14512，均一次完成增量更新。测试后停止 daemon、禁用隔离任务，保留镜像与证据，不留下测试计划。
- 隔离验收将 retries=0、Git timeout=600，默认应用配置仍为 3 次重试、30 秒间隔。核心准备阶段的网络错误现也按该配置重试，权限/参数错误不反复重试。大仓库仍受核心当前每个代理连接 300 秒上限影响，不能把本轮成功当作任意大小仓库保证。
- 本轮新版生产 GUI 尚未重新操作、未重启 Windows；此前的 WebView/托盘和自启登记验证仍作为历史证据。逐项符合情况与未验收项见 [AGENTS_AUDIT.md](AGENTS_AUDIT.md)。

## `d4674b1` 核心更新复验

- 7 个应用库测试、CLI/GUI 测试目标与 doc-tests、fmt、all-targets check、前端格式检查和生产构建通过；Windows PowerShell 5.1 下的 release CLI smoke 验证通过。
- `scripts/build.ps1 -Version 0.1.0` 重新生成 CLI、GUI、NSIS EXE 和 MSI。没有创建版本标签或运行云端安装包工作流。
- 20:00:35–20:00:39，普通配置下的 release CLI 完成 `octocat/git-consortium` 首次 mirror clone（83,208 字节）及增量 fetch/remote update（90,061 字节），两轮均一次尝试成功、退出码 0。隔离测试将 retries=0、timeout_seconds=60，默认应用设置未改变。
- 为实际覆盖新核心代理策略，在被忽略的测试目录调用核心 `network::speedtest`，只测 github.com，HTTPS 优选成功；随后调用核心 `proxy::serve`，仅监听本机 127.0.0.1:17897，CLI 通过测试进程的 GHBOOST_DATA_DIR 使用该代理。20:00:45–20:00:49 的 mirror clone 与增量更新也均一次尝试成功、退出码 0。结束时通过核心 stop 正常停止，未修改系统 PAC/代理或 Hosts，没有遗留代理进程。
- 本轮没有修改 Clash 的 GitHub DIRECT 规则。普通配置下也恢复成功，且此次核心提交未修改 Git URL/环境接口，因此不能仅凭更新后成功把此前失败唯一归因于旧核心；此前将问题主要归到 Clash 的判断证据不足。新核心的 HTTPS 优选和代理路径实际可用，长期稳定性仍需持续观察。
- 详细结果位于忽略目录 `output/core-d4674b1-probe/{baseline,proxy,speedtest}.json`，私有核心及隔离诊断代码不进入公开仓库。

## `0555888` 核心更新复验

- `cargo test --locked` 的 7 个库测试、CLI/GUI 目标和 doc-tests 通过。
- `cargo fmt --check`、`cargo check --all-targets --locked`、前端格式检查和生产构建通过。
- Windows PowerShell 5.1 下的 release CLI smoke 验证通过。
- `scripts/build.ps1 -Version 0.1.0` 完成 CLI、GUI、NSIS EXE 和 MSI 全部构建。没有触发云端安装包构建或新增版本标签。
- 新核心下对 `octocat/git-consortium` 的两轮真实克隆均尝试 4 次后失败，Git 退出码 128，日志仍为 `schannel: failed to receive handshake`。第二轮期间任务被用户中断，后台最终结果也已读取；当前没有遗留验证进程。本轮没有完成首次克隆或增量更新，不能把首轮旧核心的成功记录当作本轮网络验收通过。

## 构建与命令行

| 项目 | 实际结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 当前 10 个库测试通过，CLI/GUI 测试目标和 doc-tests 通过 |
| `cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked` | 通过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | 通过 |
| `npm run build` | TypeScript 检查与 Vite 打包通过 |
| `cargo build` / CLI release 构建 | 通过 |
| `npm run tauri -- build --bundles nsis,msi -- --locked` | GUI、NSIS EXE 和 MSI 生成成功 |
| `scripts/test-cli.ps1`，使用 Windows PowerShell 5.1 和 release CLI | 通过：帮助/describe、参数错误、增删、启停、配置、status、history、run 前置失败和禁用任务的 run-all，校验 JSON 封装和退出码 |

单元测试覆盖时间与任务持久化、删除确认、Windows 文件名边界、失败前置检查记录、过期任务及暂停、调度器唯一锁、CLI schema、DPAPI 加密往返和 Token 脱敏。没有把真实网络稳定性当作离线测试保证。

## 真实 Git 与 WebView

- 16:27 最终 release CLI 再次完成 `octocat/git-consortium` 的首次 mirror clone（83,208 字节）和后续增量更新（90,061 字节），两次退出码均为 0。每日 03:00 的任务计划正确计算为 2026-10-08 03:00（北京时间）。复验单独设置 retries=0、timeout_seconds=90，以便快速反馈网络问题；应用默认重试配置未改变。
- 16:28:02 最终 release CLI daemon 按计划触发增量备份；此轮网络再次出现 schannel TLS 握手失败，16:28:34 记录退出码 128。离线测试与编译通过不能保证 GitHub 网络稳定。
- 恢复默认的 3 次重试、间隔 30 秒后，16:30:02 daemon 自动开始下一轮；前三次遇到相同 TLS 错误，第四次成功完成增量 fetch 和 remote update，16:31:49 结束，退出码 0，90,061 字节。因此最终版本的自动执行、重试及增量备份得到实际成功验证，网络仍可能波动。
- 最终生产 GUI 的远程调试启动命令被自动审批以 `blocked by policy` 拒绝，未补做该轮生产 WebView 操作；下面的 GUI/托盘记录来自此前实际验收。

- 测试仓库：`octocat/git-consortium`，公开小型仓库，无 Token。初期 CLI 验证完成 mirror clone（83,208 字节）及后续 `remote set-url`、`fetch --all --prune --tags`、`remote update --prune`（90,061 字节），退出码均为 0。
- 实际 GhArchive WebView2 中接受首次说明、新增任务，并点击立即备份：15:46:42 开始，15:46:43 完成 mirror clone，退出码 0，83,208 字节。
- GUI 已操作验证：保存/编辑任务、启用/禁用、查看历史及日志、设置保存、开机自启开关、GitHub 图标打开动作。删除时分别取消和确认二次确认框，确认后任务移除且镜像 HEAD 文件仍存在。
- 每日 03:00 配置已保存，并计算为次日 03:00 的计划；另设置临近的 15:47 以验证触发，不声称实际等待到了凌晨 03:00。
- 15:46:44 后发送主窗口关闭事件；原主窗口不可见且进程仍在运行。任务在 15:47:04 自动开始，证明隐藏窗口后调度器继续工作。
- **该次定时 Git 操作没有成功**：GitHub TLS 握手失败，记录 `schannel: failed to receive handshake`，总共 4 次尝试，约 111 秒后结束并写入 SQLite 和文件日志。此记录证明定时触发、默认 3 次重试和错误保留，不等同于自动备份网络验收成功。
- 开机自启开关在当前用户 Run 注册表中登记正确 GUI 路径；测试后关闭自启。未重启用户电脑，不声称完成了实际 Windows 登录后的验收。

## 打包与发布边界

NSIS 和 MSI 均包含 `gharchive-desktop.exe` 与 `gharchive.exe`，已检查 NSIS 生成脚本和 MSI File 表。不声称完成全新电脑的安装、卸载或 SmartScreen 验收；程序未签名。

开发端口为独立的 1427，Vite strictPort 阻止占用时静默换端口。生产构建使用内置页面。Tauri 主包名与 GUI 二进制名一致，避免将 CLI 误识别为主桌面二进制。

GhBoost 仓库由另一任务维护，此项目只复制已有固定提交核心到被忽略的 `.deps`，不另行修改或推送 GhBoost。

普通 GitHub 工作流不打包；没有配置 `GHBOOST_READ_TOKEN` 时只执行前端检查并明确写出 Rust 检查跳过。指定版本工作流需要该 Secret 才能读取私有核心。当前交付不创建版本标签，不自动发布 Release。

本地详细数据、日志和截图保存在忽略的 `output/` 与 `.playwright-cli/`，不会把测试目录、本机账户、私有核心或凭据推送到公开仓库。
