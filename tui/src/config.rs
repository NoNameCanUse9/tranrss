use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 数据库模式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum DatabaseMode {
    /// 连接远程 TranRSS 实例（HTTP API）
    Remote {
        server: String,
        api_key: String,
    },
    /// 连接本地已有 SQLite 文件（寄生模式）
    Local {
        db_path: String,
    },
    /// 全新实例，指定目录创建 SQLite
    Fresh {
        data_dir: String,
    },
}

impl Default for DatabaseMode {
    fn default() -> Self {
        DatabaseMode::Remote {
            server: String::new(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// 数据库模式配置
    #[serde(default)]
    pub database: DatabaseMode,

    /// 旧版兼容：server 字段
    #[serde(default)]
    pub server: String,

    /// 旧版兼容：api_key 字段
    #[serde(default)]
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
            let mut config: Config = toml::from_str(&content)
                .with_context(|| format!("解析配置文件失败: {:?}", config_path))?;

            // 兼容旧版配置：如果 database 是默认的 Remote 但 server/api_key 有值
            if let DatabaseMode::Remote { server, .. } = &config.database {
                if server.is_empty() && !config.server.is_empty() {
                    config.database = DatabaseMode::Remote {
                        server: config.server.clone(),
                        api_key: config.api_key.clone(),
                    };
                }
            }

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

    /// 获取数据库模式
    pub fn database_mode(&self) -> &DatabaseMode {
        &self.database
    }

    /// 获取 API base URL（仅 Remote 模式）
    pub fn api_base(&self) -> String {
        match &self.database {
            DatabaseMode::Remote { server, .. } => server.trim_end_matches('/').to_string(),
            _ => String::new(),
        }
    }

    /// 获取 API Key（仅 Remote 模式）
    pub fn api_key_value(&self) -> String {
        match &self.database {
            DatabaseMode::Remote { api_key, .. } => api_key.clone(),
            _ => String::new(),
        }
    }

    /// 获取 SQLite 数据库路径（Local/Fresh 模式）
    pub fn db_path(&self) -> Option<PathBuf> {
        match &self.database {
            DatabaseMode::Local { db_path } => Some(PathBuf::from(db_path)),
            DatabaseMode::Fresh { data_dir } => Some(PathBuf::from(data_dir).join("data.database")),
            _ => None,
        }
    }

    /// 是否是寄生模式（连接已有实例）
    pub fn is_parasitic(&self) -> bool {
        matches!(self.database, DatabaseMode::Local { .. })
    }

    /// 是否是全新模式
    pub fn is_fresh(&self) -> bool {
        matches!(self.database, DatabaseMode::Fresh { .. })
    }

    /// 是否是远程模式
    pub fn is_remote(&self) -> bool {
        matches!(self.database, DatabaseMode::Remote { .. })
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
        assert!(config.is_remote());
        assert!(!config.is_parasitic());
        assert!(!config.is_fresh());
    }

    #[test]
    fn test_remote_mode() {
        let toml_str = r#"
[database]
mode = "remote"
server = "http://example.com"
api_key = "trss_abc"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.is_remote());
        assert_eq!(config.api_base(), "http://example.com");
        assert_eq!(config.api_key_value(), "trss_abc");
    }

    #[test]
    fn test_local_parasitic_mode() {
        let toml_str = r#"
[database]
mode = "local"
db_path = "/var/lib/tranrss/data.database"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.is_parasitic());
        assert_eq!(config.db_path(), Some(PathBuf::from("/var/lib/tranrss/data.database")));
    }

    #[test]
    fn test_fresh_mode() {
        let toml_str = r#"
[database]
mode = "fresh"
data_dir = "/home/user/tranrss-data"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.is_fresh());
        assert_eq!(config.db_path(), Some(PathBuf::from("/home/user/tranrss-data/data.database")));
    }

    #[test]
    fn test_backward_compatibility() {
        let toml_str = r#"
server = "http://old-server.com"
api_key = "trss_old"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        // 旧版配置应该兼容
        assert_eq!(config.server, "http://old-server.com");
        assert_eq!(config.api_key, "trss_old");
    }

    #[test]
    fn test_serialize_remote() {
        let config = Config {
            database: DatabaseMode::Remote {
                server: "http://example.com".to_string(),
                api_key: "trss_test".to_string(),
            },
            ..Default::default()
        };
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("mode = \"remote\""));
        assert!(toml_str.contains("server = \"http://example.com\""));
    }

    #[test]
    fn test_serialize_local() {
        let config = Config {
            database: DatabaseMode::Local {
                db_path: "/data/tranrss.db".to_string(),
            },
            ..Default::default()
        };
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("mode = \"local\""));
        assert!(toml_str.contains("db_path = \"/data/tranrss.db\""));
    }

    #[test]
    fn test_serialize_fresh() {
        let config = Config {
            database: DatabaseMode::Fresh {
                data_dir: "/home/user/data".to_string(),
            },
            ..Default::default()
        };
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("mode = \"fresh\""));
        assert!(toml_str.contains("data_dir = \"/home/user/data\""));
    }

    #[test]
    fn test_api_base_trailing_slash() {
        let config = Config {
            database: DatabaseMode::Remote {
                server: "http://localhost:8000/".to_string(),
                api_key: "test".to_string(),
            },
            ..Default::default()
        };
        assert_eq!(config.api_base(), "http://localhost:8000");
    }

    #[test]
    fn test_api_base_local_mode() {
        let config = Config {
            database: DatabaseMode::Local {
                db_path: "/data/db".to_string(),
            },
            ..Default::default()
        };
        assert_eq!(config.api_base(), "");
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let config = Config {
            database: DatabaseMode::Fresh {
                data_dir: "/tmp/tranrss".to_string(),
            },
            ..Default::default()
        };

        let content = toml::to_string_pretty(&config).unwrap();
        std::fs::write(&config_path, content).unwrap();

        let loaded: Config = toml::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        assert!(loaded.is_fresh());
        assert_eq!(loaded.db_path(), Some(PathBuf::from("/tmp/tranrss/data.database")));
    }
}
