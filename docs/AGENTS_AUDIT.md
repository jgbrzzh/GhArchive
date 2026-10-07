# AGENTS.md 要求自检

2026-10-07。按项目本地指令检查当前代码与实际验收记录。本地 AGENTS.md 本身不进入公开仓库。历史 GUI 验证与本轮 CLI/核心验证分开记录，详见 [VALIDATION.md](VALIDATION.md)。

| 要求 | 实现与验收情况 |
| --- | --- |
| GhArchive 独立项目，禁止改动 GhBoost | 符合。只读取远端核心快照；GhBoost 没有新增、修改或推送文件。 |
| 私有核心复制到 `.deps`，固定来源提交 | 符合。`ghboost-core.lock.json` 锁定 `00710891d5cc882681538167be060e7e48707aed`，核心 0.1.1；Cargo 路径依赖，`.deps` 被忽略。 |
| 自动调用内置 GhBoost 获取仓库 | 此前存在缺口，本轮修复。默认 GUI/CLI/调度任务调用核心优选、启动任务代理、生成 Git URL/环境；不依赖外部 GhBoost 软件。 |
| 不重复实现加速、不静默写 Hosts | 符合。DNS/HTTPS 优选和代理路由来自核心；应用只管理生命周期。不修改系统 PAC/代理，Hosts 接口只做已有配置检查。 |
| 四种仓库输入格式 | 符合。统一经核心规范化为 HTTPS，再验证 owner/repo 和 Windows 目录名。 |
| 任务字段、mirror clone、增量更新 | 符合。SQLite 保存名称、地址、目录、HH:mm、启用、代理、镜像和备注；实际验证首次 clone 与随后 fetch/remote update。 |
| 默认失败重试 3 次，间隔 30 秒 | 符合。Git 操作及核心准备阶段的网络错误均支持配置重试；参数、安全检查错误直接报告。历史定时任务实际验证第四次 Git 尝试成功。 |
| 运行记录、每日日志、敏感信息脱敏 | 符合。SQLite 与数据目录 logs 保存开始/结束、退出码、stdout/stderr、大小、状态和次数；Token 脱敏测试通过。 |
| 并发默认 2、立即运行、CLI、每日任务 | 符合实现。SQLite 即时事务限制跨入口运行数，文件锁阻止同任务重复执行。历史 GUI 隐藏后定时触发、本轮内置核心路径另行实测。 |
| 开机恢复任务 | 已实现到期补跑、异常运行恢复与调度器唯一锁；开机自启注册表此前实际验证，未重启用户电脑，登录后的端到端验收仍待验证。 |
| 托盘菜单、关闭隐藏、自启与启动隐藏开关 | 已实现；此前实际验证关闭窗口进程保持运行和隐藏后任务触发。新版 GUI 尚未重新操作验收。 |
| 仪表盘、任务增删改查、历史导出、设置与主题 | 已实现 Vue/WebView2 页面；此前 GUI 操作通过。本轮修改了内置加速文案并重新构建，未据此冒充新版 GUI 操作验收。 |
| 所有 CLI 命令、JSON 封装与退出码 | 已实现。独立 smoke 验证帮助/describe、增删、启停、配置、status/history、失败记录、run-all 和错误退出码；真实 run 另外做网络验证。 |
| SQLite 两张业务表 | 符合。backup_tasks、backup_runs 包含要求字段；额外 settings 保存配置，runs 增加 attempts。 |
| 删除确认、DPAPI、权限与磁盘空间、首次声明 | 符合。删除任务需确认/--yes，始终保留镜像，不提供删除备份功能；拒绝 junction/符号链接；DPAPI 往返、脱敏和前置失败记录测试通过。 |
| 右上角 GitHub 图标、默认浏览器 | 符合实现。点击经 Tauri opener 打开项目公开仓库，此前 GUI 动作已验证。 |
| GPL-3.0-only、公开仓库资料 | LICENSE、README、贡献/行为/安全说明、变更记录、第三方说明、Issue/PR 模板、CODEOWNERS 已收录；公开仓库无法缺少私有核心独立构建，README 已说明依赖与分发边界。 |
| 不推 AGENTS.md/核心/凭据/本地产物 | 符合当前跟踪清单和忽略规则检查。测试、日志、截图、安装包位于忽略目录。 |
| Actions 节省用量、普通提交只检查 | 符合。路径过滤、取消过期检查、普通提交只做前端/Rust 检查；版本标签或手动明确版本才执行 EXE/MSI 构建。不自动发布 Release。 |
| 私有核心凭据缺失明确报告 | 符合工作流。没有 GHBOOST_READ_TOKEN 时明确跳过 Rust；版本构建缺凭据直接失败。目前仓库没有配置该 Secret，云端 Rust 与安装包构建未验收。 |
| 测试、前端构建、Windows 安装包 | 结果记录于 VALIDATION.md。本轮增加代理并发、取消清理和禁止静默直连测试；安装包不等同于全新机器安装/卸载通过。 |
| 不引入 HTTP 管理 API 或 MCP | 符合。只有 Tauri IPC 与 CLI；本机 CONNECT 传输来自既有核心，不新增应用管理服务。 |

## 尚未完成的运行环境验收

- 新版生产 GUI 的实际操作、真实 Windows 重新登录后的自启与定时运行。
- 配置专用私有核心只读凭据后，云端 Rust 检查和显式版本安装包构建。
- 全新 Windows 环境安装/卸载、签名及 SmartScreen 验收。当前产物未签名。
- 本轮真实公开仓库成功不代表私有 Token、第三方镜像、Git LFS/子模块或 Release 附件得到完整备份；LFS 对象、子模块内容、附件不在此 Git 镜像备份范围。

文件树见 [FILE_TREE.md](FILE_TREE.md)，依赖关系见 [GHBOOST_INTEGRATION.md](../GHBOOST_INTEGRATION.md)，运行、打包与 CLI 示例见 [README.md](../README.md)。
