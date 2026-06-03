use anyhow::Result;
use clap::Subcommand;

use crate::config::Config;

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
}

pub async fn run(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let config = Config::load()?;
            println!("当前配置:");
            println!("  server = {}", config.server);
            println!("  api_key = {}", mask_key(&config.api_key));
            println!("  log.output = {}", config.log.output);
            println!("  log.level = {}", config.log.level);
            println!("  tui.theme = {}", config.tui.theme);
            println!("  tui.language = {}", config.tui.language);
        }
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            match key.as_str() {
                "server" => config.server = value,
                "api_key" => config.api_key = value,
                "log.output" => config.log.output = value,
                "log.level" => config.log.level = value,
                "tui.theme" => config.tui.theme = value,
                "tui.language" => config.tui.language = value,
                _ => {
                    eprintln!("未知配置项: {}", key);
                    return Ok(());
                }
            }
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
