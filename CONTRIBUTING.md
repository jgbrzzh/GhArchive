# 贡献说明

先通过 Issue 说明问题和预期行为。小修复可直接提交 PR；重大变更先讨论。应用代码使用 GPL-3.0-only，贡献表示同意以此许可证发布你的代码。

按 README 准备私有 GhBoost 核心、安装依赖。提交前运行 `npm run build`、`cargo fmt --manifest-path src-tauri/Cargo.toml --check`、`cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib --locked`、`cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked` 和 `scripts/test-cli.ps1`。没有核心权限的贡献者可以做前端工作，须说明 Rust 检查未执行。

不要提交 AGENTS.md、`.deps`、构建产物、数据库、Token 或个人路径。保留 Cargo.lock 和 package-lock.json。网络测试只使用有权访问的仓库，隔离数据目录；不要修改机器的 Hosts 或全局 Git 配置。

普通 PR 不应触发安装包工作流。发布由维护者明确指定版本；不要为小变更额外创建版本标签。
