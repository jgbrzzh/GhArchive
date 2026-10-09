# 验收记录

## 2026-10-09：核心更新与按需选路（开发版）

- 远端 main 核实为 `52aeac9fcfac0b50241bf1aa9a82ceec8327bc63`，核心版本仍为 0.1.2。使用锁定提交的 Git archive 导出到忽略目录，全部源码文件 SHA256 与提交快照一致；不复制 GhBoost 工作区中另一任务正在维护的未提交修改。prepare-core 脚本也改用提交导出，包含核心测试并移除过时源码文件。
- 移除强制 90 秒完整测速前置条件，直接使用核心 CONNECT 按需选路；健康缓存、DNS 后台刷新、失败冷却、IPv4/IPv6 并发候选回退由核心负责。Git 命令和重试共用任务代理；更新检查同样使用该接入。代理就绪日志不再宣称已经完成优选。
- 应用 24 个库测试、核心 22 个单元测试及 4 个离线集成测试通过。核心中需管理员/真实 SNI 网络的 2 个显式测试保持 ignored；没有启用 SNI、安装 CA 或使用 loopback Hosts 模式。新增应用回归验证启动不依赖测速，且非法域名不能启动加速。
- `npm run build`、前端格式、CLI smoke、Tauri `build --no-bundle -- --locked` 通过。仅构建本地开发产物，未创建版本标签或发布安装包。
- 使用隔离数据目录、默认任务代理、无 Token/镜像，`octocat/git-consortium` 首次 mirror clone 与增量 fetch/remote update 均一次成功，退出码 0，耗时约 3 秒和 4 秒。
- 用户指定的 `Variante/endfield_research_kit` 首次 mirror clone 与增量同样一次成功，退出码 0，耗时约 9 秒和 3 秒，最终镜像 57,850,468 字节。四轮运行证据在忽略的 `output/core-52aeac9-validation/`。未调整 Clash 或全局 Git 配置；本轮成功不保证其他网络环境始终连通，未重新操作原生 GUI/托盘或 Windows 登录验收。

## 2026-10-08：批量导入与其他 Git 服务（开发版）

- 文字识别复用 GhBoost 的 GitHub 地址接口；增加其他服务的 HTTPS `.git` 地址校验。用户提供的 30 个地址全部识别，Markdown 标签/目标合并 30 次重复，清除查询参数；不联网判定仓库是否存在。文本上限 1 MiB，每批最多 500 个仓库。
- 22 个 Rust 库测试通过，覆盖整批失败不写入、重复/已有任务跳过、GitHub 大小写无关去重、其他服务路径大小写保留、URL 指纹避免 Windows 镜像路径混用、SSH/转义/参数、目录安全与 GitHub Token 不发送到其他站点。
- CLI 构建、all-targets check、格式及前端生产构建通过。Windows PowerShell CLI smoke 在隔离目录导入 30 项、重复导入全部跳过、非法时间不改变任务；测试任务全部禁用。UTF-8 文件与 stdin 预览可用。
- `https://git.xeondev.com/LR/S.git` 实际首次 mirror clone 及后续增量 fetch/remote update 均退出码 0，镜像 HEAD `936df33f69f7af2d2ddf52e650521e2667df0256`。证据在 output/xeondev-validation，未修改 GhBoost 或系统网络配置。
- Playwright 用真实 CLI 识别输出作为测试 IPC 响应，验证预览、全选/清空、取消单项、统一参数、确认提交与关闭弹窗；这是前端交互测试，未在原生 WebView 中操作。截图在 output/playwright/batch-import-*.png。
- 本地 Tauri 构建 `build --no-bundle -- --locked` 通过。未创建版本标签或发布新安装包；v0.1.1 Release 保留原产物。
- Vite 开发预览曾因监听 Rust 编译产物遇到 EBUSY，已将 src-tauri、私有依赖及产物目录加入 watch 忽略列表。

日期：2026-10-07，北京时间。平台：Windows x64，Rust 1.98.1、Node.js 24.19.0，当前应用版本 0.1.1。以下保留旧版本历史记录。

当时依赖同步到 GhBoost 远端提交 `5533732011b90dbc681056a74bff3a3b9edc0ee7`，核心 0.1.2。核心副本仅在忽略的 `.deps` 内，GhArchive 未改动 GhBoost。该版本默认任务自动调用核心 DNS/HTTPS 优选和任务专用代理，不依赖外部 GhBoost 软件，不修改系统 PAC/代理或 Hosts，也不自动启用 SNI/安装根证书。

下面的真实网络和 GUI/托盘验收记录来自首轮 `e289a0c00f23bb547fa26e374b6fb012ec0dcf20` 构建；新核心的构建与 CLI 复验单独记录，不能据此声称重新操作了 GUI 或重启电脑。

## 0.1.1 签名更新与核心 0.1.2

- 14 个应用库测试通过，新增签名篡改拒绝、官方更新 URL 限制、24 小时检查频率、跨进程锁互斥回归。CLI smoke、fmt、all-targets check、前端格式和生产构建通过。
- 本地 `scripts/build.ps1 -Version 0.1.1 -SignUpdates` 完成 NSIS EXE、MSI 与两个版本绑定的更新签名；`prepare-update-manifest.ps1` 在 Windows PowerShell 5.1 生成无 BOM 的 latest.json，包含 NSIS/MSI 平台与默认 NSIS 回退项，只选择当前版本产物。
- 通过 Tauri MockRuntime 及本机测试 HTTP 服务，使用实际安装包验证 updater 的 check/download：正常包签名验证通过、篡改字节失败、清单版本与签名版本不一致失败。测试 HTTP 仅存在于忽略的诊断程序，生产配置仍只接受 HTTPS；没有执行安装器，不据此声称安装重启已验收。
- 关于页浏览器预览显示 v0.1.1、核心 v0.1.2 与提交来源，以及“检查更新”按钮。浏览器按钮按设计禁用，不连接 Rust；新版生产 WebView 未再次操作。预览结束后已停止测试浏览器和 Vite，避免 Windows 构建占用 esbuild.exe。
- 核心 `3935ecc` 下（与最终固定的 `5533732` 核心文件逐一 SHA256 比对一致，后者只更新 GhBoost 打包脚本），22:28:15–22:28:34 首次 clone 小仓库成功。随后禁用重试的增量优选全失败、指定大仓库增量出现 Schannel 握手失败。恢复默认 3 次重试后，22:30:00–22:30:16 小仓库增量、22:30:16–22:30:31 大仓库增量均一次成功。网络仍会波动，未改变 Clash、Hosts 或全局 Git 配置。
- 私钥只在忽略目录、当前用户 DPAPI 备份和 GitHub Secret 保存；公钥与测试签名可公开。没有配置 GHBOOST_READ_TOKEN 时云端 Rust 仍明确跳过，云端打包缺私有核心权限会失败；本次发布使用本机固定核心构建，不使用宽权限个人 Token 作为私有核心 Secret。

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
| `cargo test --manifest-path src-tauri/Cargo.toml --locked` | 当前 14 个库测试通过，CLI/GUI 测试目标和 doc-tests 通过 |
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
