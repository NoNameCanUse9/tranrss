mod api_client;
mod cli;
mod config;
mod logging;
mod tui_app;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tranrss", about = "TranRSS - AI-powered RSS reader")]
#[command(version)]
struct Cli {
    /// 配置文件路径
    #[arg(long, default_value = "~/.config/tranrss/config.toml")]
    config: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动所有服务（Web + 定时任务）
    Serve {
        /// 端口
        #[arg(long, default_value = "8000")]
        port: u16,
        /// 同时启动 TUI
        #[arg(long)]
        with_tui: bool,
        /// 不启动定时任务
        #[arg(long)]
        no_cron: bool,
    },

    /// 进入 TUI 交互界面
    Tui,

    /// 订阅管理
    Sub {
        #[command(subcommand)]
        action: cli::sub::SubAction,
    },

    /// 文章操作
    Article {
        #[command(subcommand)]
        action: cli::article::ArticleAction,
    },

    /// 任务队列
    Job {
        #[command(subcommand)]
        action: cli::job::JobAction,
    },

    /// API 配置
    Api {
        #[command(subcommand)]
        action: cli::api::ApiAction,
    },

    /// 配置管理
    Config {
        #[command(subcommand)]
        action: cli::config::ConfigAction,
    },

    /// 定时任务执行入口（由系统调度器调用）
    Cron {
        /// 任务类型
        #[command(subcommand)]
        task: cli::cron::CronTask,
    },

    /// 初始化/安装（生成 systemd unit / crontab）
    Setup {
        /// 指定调度器类型
        #[arg(long)]
        scheduler: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    logging::init()?;

    match cli.command {
        Some(Commands::Serve { port, with_tui, no_cron }) => {
            cli::serve::run(port, with_tui, no_cron).await
        }
        Some(Commands::Tui) => {
            cli::tui::run().await
        }
        Some(Commands::Sub { action }) => {
            cli::sub::run(action).await
        }
        Some(Commands::Article { action }) => {
            cli::article::run(action).await
        }
        Some(Commands::Job { action }) => {
            cli::job::run(action).await
        }
        Some(Commands::Api { action }) => {
            cli::api::run(action).await
        }
        Some(Commands::Config { action }) => {
            cli::config::run(action).await
        }
        Some(Commands::Cron { task }) => {
            cli::cron::run(task).await
        }
        Some(Commands::Setup { scheduler }) => {
            cli::setup::run(scheduler).await
        }
        None => {
            // 无参数默认进入 TUI
            cli::tui::run().await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_serve_default() {
        let cli = Cli::try_parse_from(&["tranrss", "serve"]).unwrap();
        match cli.command {
            Some(Commands::Serve { port, with_tui, no_cron }) => {
                assert_eq!(port, 8000);
                assert!(!with_tui);
                assert!(!no_cron);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_serve_custom_port() {
        let cli = Cli::try_parse_from(&["tranrss", "serve", "--port", "9000"]).unwrap();
        match cli.command {
            Some(Commands::Serve { port, .. }) => assert_eq!(port, 9000),
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_serve_with_tui() {
        let cli = Cli::try_parse_from(&["tranrss", "serve", "--with-tui"]).unwrap();
        match cli.command {
            Some(Commands::Serve { with_tui, .. }) => assert!(with_tui),
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_serve_no_cron() {
        let cli = Cli::try_parse_from(&["tranrss", "serve", "--no-cron"]).unwrap();
        match cli.command {
            Some(Commands::Serve { no_cron, .. }) => assert!(no_cron),
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_cli_tui() {
        let cli = Cli::try_parse_from(&["tranrss", "tui"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Tui)));
    }

    #[test]
    fn test_cli_no_command() {
        let cli = Cli::try_parse_from(&["tranrss"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_sub_list() {
        let cli = Cli::try_parse_from(&["tranrss", "sub", "list"]).unwrap();
        match cli.command {
            Some(Commands::Sub { action }) => {
                assert!(matches!(action, cli::sub::SubAction::List));
            }
            _ => panic!("Expected Sub command"),
        }
    }

    #[test]
    fn test_cli_sub_sync_all() {
        let cli = Cli::try_parse_from(&["tranrss", "sub", "sync"]).unwrap();
        match cli.command {
            Some(Commands::Sub { action }) => match action {
                cli::sub::SubAction::Sync { id } => assert!(id.is_none()),
                _ => panic!("Expected Sync action"),
            },
            _ => panic!("Expected Sub command"),
        }
    }

    #[test]
    fn test_cli_sub_sync_one() {
        let cli = Cli::try_parse_from(&["tranrss", "sub", "sync", "42"]).unwrap();
        match cli.command {
            Some(Commands::Sub { action }) => match action {
                cli::sub::SubAction::Sync { id } => assert_eq!(id, Some(42)),
                _ => panic!("Expected Sync action"),
            },
            _ => panic!("Expected Sub command"),
        }
    }

    #[test]
    fn test_cli_article_list() {
        let cli = Cli::try_parse_from(&["tranrss", "article", "list"]).unwrap();
        match cli.command {
            Some(Commands::Article { action }) => match action {
                cli::article::ArticleAction::List { feed, unread, starred } => {
                    assert!(feed.is_none());
                    assert!(!unread);
                    assert!(!starred);
                }
                _ => panic!("Expected List action"),
            },
            _ => panic!("Expected Article command"),
        }
    }

    #[test]
    fn test_cli_article_list_with_feed() {
        let cli = Cli::try_parse_from(&["tranrss", "article", "list", "--feed", "Hacker News"]).unwrap();
        match cli.command {
            Some(Commands::Article { action }) => match action {
                cli::article::ArticleAction::List { feed, .. } => {
                    assert_eq!(feed, Some("Hacker News".to_string()));
                }
                _ => panic!("Expected List action"),
            },
            _ => panic!("Expected Article command"),
        }
    }

    #[test]
    fn test_cli_article_list_unread() {
        let cli = Cli::try_parse_from(&["tranrss", "article", "list", "--unread"]).unwrap();
        match cli.command {
            Some(Commands::Article { action }) => match action {
                cli::article::ArticleAction::List { unread, .. } => assert!(unread),
                _ => panic!("Expected List action"),
            },
            _ => panic!("Expected Article command"),
        }
    }

    #[test]
    fn test_cli_article_translate() {
        let cli = Cli::try_parse_from(&["tranrss", "article", "translate", "123"]).unwrap();
        match cli.command {
            Some(Commands::Article { action }) => match action {
                cli::article::ArticleAction::Translate { id } => assert_eq!(id, 123),
                _ => panic!("Expected Translate action"),
            },
            _ => panic!("Expected Article command"),
        }
    }

    #[test]
    fn test_cli_article_summarize() {
        let cli = Cli::try_parse_from(&["tranrss", "article", "summarize", "456"]).unwrap();
        match cli.command {
            Some(Commands::Article { action }) => match action {
                cli::article::ArticleAction::Summarize { id } => assert_eq!(id, 456),
                _ => panic!("Expected Summarize action"),
            },
            _ => panic!("Expected Article command"),
        }
    }

    #[test]
    fn test_cli_job_list() {
        let cli = Cli::try_parse_from(&["tranrss", "job", "list"]).unwrap();
        match cli.command {
            Some(Commands::Job { action }) => match action {
                cli::job::JobAction::List { status } => assert!(status.is_none()),
                _ => panic!("Expected List action"),
            },
            _ => panic!("Expected Job command"),
        }
    }

    #[test]
    fn test_cli_job_list_with_status() {
        let cli = Cli::try_parse_from(&["tranrss", "job", "list", "--status", "failed"]).unwrap();
        match cli.command {
            Some(Commands::Job { action }) => match action {
                cli::job::JobAction::List { status } => assert_eq!(status, Some("failed".to_string())),
                _ => panic!("Expected List action"),
            },
            _ => panic!("Expected Job command"),
        }
    }

    #[test]
    fn test_cli_job_retry() {
        let cli = Cli::try_parse_from(&["tranrss", "job", "retry", "abc-123"]).unwrap();
        match cli.command {
            Some(Commands::Job { action }) => match action {
                cli::job::JobAction::Retry { id } => assert_eq!(id, "abc-123"),
                _ => panic!("Expected Retry action"),
            },
            _ => panic!("Expected Job command"),
        }
    }

    #[test]
    fn test_cli_config_show() {
        let cli = Cli::try_parse_from(&["tranrss", "config", "show"]).unwrap();
        match cli.command {
            Some(Commands::Config { action }) => {
                assert!(matches!(action, cli::config::ConfigAction::Show));
            }
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_cli_config_set() {
        let cli = Cli::try_parse_from(&["tranrss", "config", "set", "server", "http://example.com"]).unwrap();
        match cli.command {
            Some(Commands::Config { action }) => match action {
                cli::config::ConfigAction::Set { key, value } => {
                    assert_eq!(key, "server");
                    assert_eq!(value, "http://example.com");
                }
                _ => panic!("Expected Set action"),
            },
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_cli_cron_sync() {
        let cli = Cli::try_parse_from(&["tranrss", "cron", "sync"]).unwrap();
        match cli.command {
            Some(Commands::Cron { task }) => {
                assert!(matches!(task, cli::cron::CronTask::Sync));
            }
            _ => panic!("Expected Cron command"),
        }
    }

    #[test]
    fn test_cli_setup() {
        let cli = Cli::try_parse_from(&["tranrss", "setup"]).unwrap();
        match cli.command {
            Some(Commands::Setup { scheduler }) => assert!(scheduler.is_none()),
            _ => panic!("Expected Setup command"),
        }
    }

    #[test]
    fn test_cli_setup_with_scheduler() {
        let cli = Cli::try_parse_from(&["tranrss", "setup", "--scheduler", "systemd"]).unwrap();
        match cli.command {
            Some(Commands::Setup { scheduler }) => assert_eq!(scheduler, Some("systemd".to_string())),
            _ => panic!("Expected Setup command"),
        }
    }
}
