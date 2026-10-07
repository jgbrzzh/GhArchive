# 安全政策

目前仅维护 0.1.x 开发版本，不保证生产环境安全审计。

请通过 [GitHub 私密漏洞报告](https://github.com/jgbrzzh/GhArchive/security/advisories/new) 报告漏洞。若仓库未开放此入口，请在 Issue 中只说明希望私下联系维护者，不披露漏洞细节或凭据。

报告包含版本、复现条件、影响范围和脱敏日志。不要发送 Token、DPAPI 文件、私有仓库内容或真实用户名路径。DPAPI 文件只绑定当前 Windows 用户，不能作为跨设备可用的配置备份。

镜像仅用于公开仓库；配置 Token 时禁止镜像。不关闭 TLS 验证，不把 Token 存入 remote URL，认证环境仅传给 Git 子进程。任何变更都应保留这些边界。
