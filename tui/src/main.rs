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
