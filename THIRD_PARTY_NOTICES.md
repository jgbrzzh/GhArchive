# 第三方依赖

GhArchive 应用代码为 GPL-3.0-only，完整许可见 LICENSE。依赖版本见 `package-lock.json` 和 `src-tauri/Cargo.lock`；以下是主要直接依赖，并非全部传递依赖清单。

| 组件 | 用途 | 许可证 |
| --- | --- | --- |
| GhBoost core | GitHub 地址、环境、镜像与代理准备 | GPL-3.0-only |
| Tauri / Tauri plugins | WebView2、托盘、自启、系统浏览器 | MIT / Apache-2.0 |
| Vue 3 | 界面 | MIT |
| Vite / TypeScript | 前端构建 | MIT / Apache-2.0 |
| Lucide | 界面图标 | ISC |
| Tokio / Futures | 异步调度和进程 | MIT / Apache-2.0 |
| rusqlite | SQLite 接口 | MIT |
| SQLite | 本地数据存储 | Public domain |
| chrono / clap / dirs / fs2 / serde / serde_json / uuid / url / base64 / windows-sys | 基础设施 | 各自的 MIT / Apache-2.0 等许可证 |

GhBoost 核心来源和提交固定在 `ghboost-core.lock.json`，私有源代码不包含在此仓库。分发链接该 GPL 核心的二进制时需安排完整对应源码及依赖许可的交付，不应声称本公开仓库已经包含完整对应源码。

系统 Git 和 WebView2 Runtime 由各自供应商提供，本应用没有重新授权这些系统组件。GitHub 图标用于指向项目的 GitHub 仓库，不表示 GitHub 背书。
