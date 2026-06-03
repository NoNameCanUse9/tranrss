use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub server: String,
    pub api_key: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("读取配置文件失败: {:?}", config_path))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("解析配置文件失败: {:?}", config_path))?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        dirs_or_default().join("tranrss").join("tui.toml")
    }
}

fn dirs_or_default() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
}

mod dirs {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        if let Ok(home) = std::env::var("HOME") {
            Some(PathBuf::from(home).join(".config"))
        } else {
            None
        }
    }
}
