use crate::{
    backup,
    db::{Store, TaskInput},
    scheduler, Failure, Result,
};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};

#[derive(Parser)]
#[command(
    name = "gharchive",
    version,
    about = "GhArchive · GitHub 仓库定时备份器"
)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Add {
        repo_url: String,
        #[arg(long, default_value = "03:00")]
        time: String,
        #[arg(long)]
        dir: Option<String>,
        #[arg(long, default_value = "")]
        name: String,
        #[arg(long)]
        disabled: bool,
        #[arg(long)]
        no_proxy: bool,
        #[arg(long)]
        mirror: bool,
        #[arg(long, default_value = "")]
        notes: String,
    },
    List,
    #[command(about = "从 UTF-8 文字文件或标准输入批量导入；--preview 仅识别")]
    Import {
        #[arg(long, conflicts_with = "stdin", required_unless_present = "stdin")]
        file: Option<std::path::PathBuf>,
        #[arg(long, conflicts_with = "file")]
        stdin: bool,
        #[arg(long)]
        preview: bool,
        #[arg(long, default_value = "03:00")]
        time: String,
        #[arg(long)]
        dir: Option<String>,
        #[arg(long)]
        disabled: bool,
        #[arg(long)]
        no_proxy: bool,
        #[arg(long)]
        mirror: bool,
        #[arg(long, default_value = "")]
        notes: String,
    },
    Edit {
        id: i64,
        #[arg(long, help = "完整 TaskInput JSON")]
        input: String,
    },
    Remove {
        id: i64,
        #[arg(long)]
        yes: bool,
    },
    Enable {
        id: i64,
    },
    Disable {
        id: i64,
    },
    Run {
        id: i64,
    },
    RunAll,
    Status,
    History {
        #[arg(long, default_value = "20")]
        limit: u32,
        #[arg(long)]
        task_id: Option<i64>,
    },
    Config {
        #[command(subcommand)]
        command: Config,
    },
    Describe,
    #[command(about = "常驻定时器（无 GUI）；Ctrl+C 退出，GUI 自带同一调度器")]
    Daemon,
}
#[derive(Subcommand)]
enum Config {
    Get {
        key: Option<String>,
    },
    Set {
        key: String,
        value: Option<String>,
        #[arg(long, help = "从标准输入读值，建议用于 Token")]
        stdin: bool,
    },
}
fn parsed_value(s: &str) -> Value {
    serde_json::from_str(s).unwrap_or_else(|_| json!(s))
}
pub fn describe() -> Value {
    json!({"name":"gharchive","version":env!("CARGO_PKG_VERSION"),"repository":crate::REPOSITORY,"notice":crate::NOTICE,
        "commands":["import --file UTF8-TEXT | --stdin [--preview] [--time HH:mm] [--dir PATH] [--disabled]","add <repo_url> --time HH:mm --dir <absolute-directory> [--name NAME] [--disabled] [--no-proxy] [--mirror] [--notes TEXT]","list","edit <id> --input <TaskInput-JSON>","remove <id> --yes","enable <id>","disable <id>","run <id>","run-all","status","history [--limit 20] [--task-id ID]","config get [key]","config set <key> [value] [--stdin]","describe","daemon","--help"],
        "import":"import --file UTF8-TEXT | --stdin [--preview] [--time HH:mm] [--dir PATH] [--disabled] [--no-proxy] [--mirror] [--notes TEXT]：识别文字中的仓库，去重并跳过已有任务；只创建任务，不立即备份。",
        "json":"所有命令支持 --json；响应含 success,data,error,timestamp。daemon 是常驻命令，退出时返回 JSON。",
        "exit_codes":{"0":"success","1":"invalid_argument","2":"permission_denied","3":"network_error","4":"not_found","5":"internal_error"},
        "task_schema":{"name":"string","repo_url":"string","backup_dir":"absolute path","schedule_time":"HH:mm","enabled":"bool","use_proxy":"bool","use_mirror":"bool","notes":"string"},
        "config_keys":["backup_root","concurrency","timeout_seconds","retries","retry_delay_seconds","minimum_free_bytes","mirror_url","require_hosts","paused","autostart","start_minimized","theme","accepted_notice","auto_check_updates","token"],
        "examples":["gharchive add octocat/Hello-World --time 03:00 --dir D:\\backups --json","gharchive config set accepted_notice true --json","gharchive run 1 --json","gharchive remove 1 --yes --json"]})
}
pub async fn action(s: &Store, name: &str, p: Value) -> Result<Value> {
    let id = || {
        p["id"]
            .as_i64()
            .ok_or_else(|| Failure::new(1, "缺少任务 ID"))
    };
    match name {
        "list" => Ok(json!(s.tasks()?)),
        "status" => s.status(),
        "config" => s.config(),
        "describe" => Ok(describe()),
        "import-preview" => Ok(json!(crate::import::preview(
            s,
            p["text"]
                .as_str()
                .ok_or_else(|| Failure::new(1, "缺少待识别文字"))?
        )?)),
        "import" => Ok(json!(crate::import::apply(
            s,
            serde_json::from_value::<crate::import::ImportInput>(p)
                .map_err(|e| Failure::new(1, e))?
        )?)),
        "save" => Ok(json!(s.save_task(
            p["id"].as_i64(),
            serde_json::from_value::<TaskInput>(p["task"].clone())?
        )?)),
        "remove" => s.remove(id()?, p["yes"] == true),
        "enable" => Ok(json!(s.enable(id()?, p["enabled"] == true)?)),
        "run" => backup::run(s, id()?).await,
        "run-all" => backup::run_all(s).await,
        "history" => s.history(
            p["limit"].as_u64().unwrap_or(20).min(1000) as u32,
            p["task_id"].as_i64(),
        ),
        "set" => {
            let key = p["key"]
                .as_str()
                .ok_or_else(|| Failure::new(1, "缺少配置键"))?;
            s.set(key, p["value"].clone())
        }
        _ => Err(Failure::new(1, "未知操作")),
    }
}
async fn execute(s: &Store, command: Commands) -> Result<Value> {
    match command {
        Commands::Add {
            repo_url,
            time,
            dir,
            name,
            disabled,
            no_proxy,
            mirror,
            notes,
        } => Ok(json!(s.save_task(
            None,
            TaskInput {
                name,
                repo_url,
                backup_dir: dir.unwrap_or_default(),
                schedule_time: time,
                enabled: !disabled,
                use_proxy: !no_proxy,
                use_mirror: mirror,
                notes
            }
        )?)),
        Commands::List => action(s, "list", json!({})).await,
        Commands::Import {
            file,
            stdin: _,
            preview,
            time,
            dir,
            disabled,
            no_proxy,
            mirror,
            notes,
        } => {
            use std::io::Read;
            let reader: Box<dyn Read> = if let Some(path) = file {
                Box::new(std::fs::File::open(path)?)
            } else {
                Box::new(std::io::stdin())
            };
            let mut text = String::new();
            reader
                .take(1024 * 1024 + 1)
                .read_to_string(&mut text)
                .map_err(|e| Failure::new(1, format!("请提供 UTF-8 文字：{e}")))?;
            let identified = crate::import::preview(s, text.trim_start_matches('\u{feff}'))?;
            if preview {
                return Ok(json!(identified));
            }
            let input = crate::import::ImportInput {
                repositories: identified
                    .repositories
                    .iter()
                    .map(|r| r.repo_url.clone())
                    .collect(),
                settings: TaskInput {
                    name: "".into(),
                    repo_url: "".into(),
                    backup_dir: dir.unwrap_or_default(),
                    schedule_time: time,
                    enabled: !disabled,
                    use_proxy: !no_proxy,
                    use_mirror: mirror,
                    notes,
                },
            };
            let imported = crate::import::apply(s, input)?;
            Ok(
                json!({"created_ids": imported.created_ids, "skipped": imported.skipped, "duplicates": identified.duplicates, "rejected": identified.rejected}),
            )
        }
        Commands::Edit { id, input } => {
            action(
                s,
                "save",
                json!({"id":id,"task":serde_json::from_str::<Value>(&input)?}),
            )
            .await
        }
        Commands::Remove { id, yes } => action(s, "remove", json!({"id":id,"yes":yes})).await,
        Commands::Enable { id } => action(s, "enable", json!({"id":id,"enabled":true})).await,
        Commands::Disable { id } => action(s, "enable", json!({"id":id,"enabled":false})).await,
        Commands::Run { id } => action(s, "run", json!({"id":id})).await,
        Commands::RunAll => action(s, "run-all", json!({})).await,
        Commands::Status => s.status(),
        Commands::History { limit, task_id } => s.history(limit, task_id),
        Commands::Describe => Ok(describe()),
        Commands::Config { command } => match command {
            Config::Get { key } => {
                if let Some(k) = key {
                    s.get(&k)
                } else {
                    s.config()
                }
            }
            Config::Set { key, value, stdin } => {
                let value = if stdin {
                    use std::io::Read;
                    let mut input = String::new();
                    std::io::stdin().take(65536).read_to_string(&mut input)?;
                    input.trim().into()
                } else {
                    value.ok_or_else(|| Failure::new(1, "缺少值；Token 请使用 --stdin"))?
                };
                s.set(&key, parsed_value(&value))
            }
        },
        Commands::Daemon => {
            let lock = scheduler::daemon_lock(s)?;
            tokio::select! {_=scheduler::serve(s.clone(),lock)=>{},r=tokio::signal::ctrl_c()=>{r?;}}
            Ok(json!({"stopped":true}))
        }
    }
}
pub async fn run() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args.iter().any(|s| s == "--json");
    // Clap 的 help/version 提前退出；包装它们以保证 --json 时 stdout 始终是 JSON。
    let parsed = Cli::try_parse_from(&args);
    let result = match parsed {
        Ok(cli) => match Store::open() {
            Ok(s) => {
                if let Err(e) = scheduler::recover(&s) {
                    Err(e)
                } else {
                    execute(&s, cli.command).await
                }
            }
            Err(e) => Err(e),
        },
        Err(e)
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            Ok(json!({"help":e.to_string(),"schema":describe()}))
        }
        Err(e) => Err(Failure::new(1, e.to_string())),
    };
    let code = result.as_ref().err().map_or(0, |e| e.exit);
    let envelope = crate::response(result);
    if json_mode {
        println!("{}", serde_json::to_string(&envelope).unwrap());
    } else if envelope.success {
        if let Some(help) = envelope.data["help"].as_str() {
            print!("{help}");
        } else {
            println!("{}", serde_json::to_string_pretty(&envelope.data).unwrap());
        }
    } else {
        eprintln!("{}", envelope.error.as_ref().unwrap().message);
    }
    code
}
#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn schema_and_commands() {
        Cli::command().debug_assert();
        assert!(
            Cli::try_parse_from(["gharchive", "--json", "add", "o/r", "--time", "03:00"]).is_ok()
        );
        assert!(describe()["commands"].as_array().unwrap().len() > 10);
    }
}
