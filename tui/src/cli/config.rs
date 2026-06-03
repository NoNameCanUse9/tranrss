use anyhow::Result;
use clap::Subcommand;

use crate::config::{Config, DatabaseMode};

#[derive(Subcommand)]
pub enum ConfigAction {
    /// 显示当前配置
    Show,

    /// 设置配置项
    Set {
        /// 配置键
        key: String,
        /// 配置值
        value: String,
    },

    /// 快速配置远程模式
    Remote {
        /// 服务器地址
        #[arg(long)]
        server: String,
        /// API Key
        #[arg(long)]
        api_key: String,
    },

    /// 快速配置寄生模式（连接本地已有 SQLite）
    Local {
        /// SQLite 数据库路径
        #[arg(long)]
        db_path: String,
    },

    /// 快速配置全新模式（指定目录创建新数据库）
    Fresh {
        /// 数据目录
        #[arg(long)]
        data_dir: String,
    },
}

pub async fn run(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let config = Config::load()?;
            println!("当前配置:");
            println!();
            match &config.database {
                DatabaseMode::Remote { server, api_key } => {
                    println!("  模式: 远程 (Remote)");
                    println!("  server: {}", server);
                    println!("  api_key: {}", mask_key(api_key));
                }
                DatabaseMode::Local { db_path } => {
                    println!("  模式: 寄生 (Local)");
                    println!("  db_path: {}", db_path);
                }
                DatabaseMode::Fresh { data_dir } => {
                    println!("  模式: 全新 (Fresh)");
                    println!("  data_dir: {}", data_dir);
                }
            }
            println!();
            println!("  log.output: {}", config.log.output);
            println!("  log.level: {}", config.log.level);
            println!("  tui.theme: {}", config.tui.theme);
            println!("  tui.language: {}", config.tui.language);
        }
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            match key.as_str() {
                "server" => {
                    if let DatabaseMode::Remote { server, .. } = &mut config.database {
                        *server = value;
                    } else {
                        config.database = DatabaseMode::Remote {
                            server: value,
                            api_key: String::new(),
                        };
                    }
                }
                "api_key" => {
                    if let DatabaseMode::Remote { api_key, .. } = &mut config.database {
                        *api_key = value;
                    } else {
                        eprintln!("当前不是远程模式，无法设置 api_key");
                        return Ok(());
                    }
                }
                "database.mode" => {
                    config.database = match value.as_str() {
                        "remote" => DatabaseMode::Remote { server: String::new(), api_key: String::new() },
                        "local" => DatabaseMode::Local { db_path: String::new() },
                        "fresh" => DatabaseMode::Fresh { data_dir: String::new() },
                        _ => {
                            eprintln!("未知模式: {} (可选: remote, local, fresh)", value);
                            return Ok(());
                        }
                    };
                }
                "database.db_path" => {
                    config.database = DatabaseMode::Local { db_path: value };
                }
                "database.data_dir" => {
                    config.database = DatabaseMode::Fresh { data_dir: value };
                }
                "log.output" => config.log.output = value,
                "log.level" => config.log.level = value,
                "tui.theme" => config.tui.theme = value,
                "tui.language" => config.tui.language = value,
                _ => {
                    eprintln!("未知配置项: {}", key);
                    eprintln!("可用配置项:");
                    eprintln!("  server, api_key (远程模式)");
                    eprintln!("  database.db_path (寄生模式)");
                    eprintln!("  database.data_dir (全新模式)");
                    eprintln!("  log.output, log.level");
                    eprintln!("  tui.theme, tui.language");
                    return Ok(());
                }
            }
            config.save()?;
        }
        ConfigAction::Remote { server, api_key } => {
            let mut config = Config::load()?;
            config.database = DatabaseMode::Remote { server, api_key };
            config.save()?;
        }
        ConfigAction::Local { db_path } => {
            let mut config = Config::load()?;
            config.database = DatabaseMode::Local { db_path };
            config.save()?;
        }
        ConfigAction::Fresh { data_dir } => {
            let mut config = Config::load()?;
            config.database = DatabaseMode::Fresh { data_dir };
            config.save()?;
        }
    }
    Ok(())
}

fn mask_key(key: &str) -> String {
    if key.len() > 12 {
        format!("{}...{}", &key[..8], &key[key.len() - 4..])
    } else if key.is_empty() {
        "(未设置)".to_string()
    } else {
        "***".to_string()
    }
}
