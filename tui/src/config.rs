use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub server: String,
    pub api_key: String,

    #[serde(default)]
    pub log: LogConfig,

    #[serde(default)]
    pub tui: TuiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub output: String,  // auto / journald / syslog / file / stdout
    pub level: String,   // trace / debug / info / warn / error
    pub file: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            output: "auto".into(),
            level: "info".into(),
            file: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    pub theme: String,    // dark / light
    pub language: String, // zh / en
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            language: "zh".into(),
        }
    }
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
        println!("配置已保存到 {:?}", config_path);
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        home_dir().join(".config").join("tranrss").join("config.toml")
    }

    /// 获取 API base URL
    pub fn api_base(&self) -> String {
        self.server.trim_end_matches('/').to_string()
    }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
