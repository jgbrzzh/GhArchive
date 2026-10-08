pub mod acceleration;
pub mod backup;
pub mod cli;
pub mod db;
pub mod import;
pub mod repository;
pub mod scheduler;
pub mod secrets;
pub mod update;
pub use ghboost_core::{Envelope, Failure, Result};
pub fn response(result: Result<serde_json::Value>) -> Envelope {
    let mut e = Envelope::from_result(result);
    if let Some(error) = e.error.as_mut() {
        error.hint = match error.exit {
            1 => "运行 gharchive describe --json 查看用法",
            2 => "检查目录权限、安全声明和删除确认；Token 需使用原 Windows 用户",
            3 => "检查 Git、网络和 GhBoost 配置；查看 history 中的 stderr",
            4 => "检查任务 ID、文件路径或配置键",
            _ => "查看脱敏日志并报告可复现问题",
        }
        .into();
    }
    e
}
pub const REPOSITORY: &str = "https://github.com/jgbrzzh/GhArchive";
pub const NOTICE: &str = "GhArchive 将按你设置的时间访问 GitHub 并写入本地备份。默认自动调用内置 GhBoost 做 DNS/HTTPS 优选及任务代理，不改系统 Hosts 或 PAC。镜像只用于公开仓库；私有仓库令牌使用当前 Windows 用户的 DPAPI 加密。请仅备份有权访问的内容。删除任务不会删除备份。关闭窗口后程序继续在托盘运行；退出或关机期间不执行任务，重启后补跑到期任务。";
