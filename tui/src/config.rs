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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.server.is_empty());
        assert!(config.api_key.is_empty());
        assert_eq!(config.log.output, "auto");
        assert_eq!(config.log.level, "info");
        assert!(config.log.file.is_none());
        assert_eq!(config.tui.theme, "dark");
        assert_eq!(config.tui.language, "zh");
    }

    #[test]
    fn test_api_base_trailing_slash() {
        let mut config = Config::default();
        config.server = "http://localhost:8000/".to_string();
        assert_eq!(config.api_base(), "http://localhost:8000");
    }

    #[test]
    fn test_api_base_no_trailing_slash() {
        let mut config = Config::default();
        config.server = "http://localhost:8000".to_string();
        assert_eq!(config.api_base(), "http://localhost:8000");
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = Config {
            server: "http://example.com".to_string(),
            api_key: "trss_abc123".to_string(),
            log: LogConfig {
                output: "file".to_string(),
                level: "debug".to_string(),
                file: Some("/var/log/tranrss.log".to_string()),
            },
            tui: TuiConfig {
                theme: "light".to_string(),
                language: "en".to_string(),
            },
        };

        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(deserialized.server, "http://example.com");
        assert_eq!(deserialized.api_key, "trss_abc123");
        assert_eq!(deserialized.log.output, "file");
        assert_eq!(deserialized.log.level, "debug");
        assert_eq!(deserialized.log.file, Some("/var/log/tranrss.log".to_string()));
        assert_eq!(deserialized.tui.theme, "light");
        assert_eq!(deserialized.tui.language, "en");
    }

    #[test]
    fn test_partial_config_defaults() {
        let toml_str = r#"
server = "http://example.com"
api_key = "trss_test"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server, "http://example.com");
        assert_eq!(config.log.output, "auto");
        assert_eq!(config.tui.theme, "dark");
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let config = Config {
            server: "http://test.com".to_string(),
            api_key: "trss_test".to_string(),
            ..Default::default()
        };

        let content = toml::to_string_pretty(&config).unwrap();
        std::fs::write(&config_path, content).unwrap();

        let loaded: Config = toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(loaded.server, "http://test.com");
        assert_eq!(loaded.api_key, "trss_test");
    }
}
