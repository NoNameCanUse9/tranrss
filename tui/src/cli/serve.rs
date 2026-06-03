use anyhow::Result;
use tracing_subscriber::prelude::*;
use tranrss_backend::utils::broadcast_layer::BroadcastLayer;

use crate::config::{Config, DatabaseMode};

pub async fn run(port: u16, with_tui: bool, no_cron: bool) -> Result<()> {
    let config = Config::load()?;

    // 1. 创建广播频道
    let (event_tx, _) = tokio::sync::broadcast::channel::<String>(256);

    // 2. 初始化日志（含 SSE 广播层）
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "tranrss_backend=debug,tower_http=info".into());
    let fmt_layer = tracing_subscriber::fmt::layer();
    let broadcast_layer = BroadcastLayer::new(event_tx.clone());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(broadcast_layer)
        .init();

    // 3. 根据数据库模式初始化
    let database_url = match &config.database {
        DatabaseMode::Remote { .. } => {
            // 远程模式：使用环境变量或默认路径
            std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:/app/data/data.database".to_string())
        }
        DatabaseMode::Local { db_path } => {
            // 寄生模式：连接已有 SQLite
            tracing::info!("📦 寄生模式: 连接已有数据库 {}", db_path);
            format!("sqlite:{}", db_path)
        }
        DatabaseMode::Fresh { data_dir } => {
            // 全新模式：在指定目录创建新数据库
            let dir = std::path::PathBuf::from(data_dir);
            std::fs::create_dir_all(&dir)?;
            let db_path = dir.join("data.database");
            tracing::info!("🆕 全新模式: 创建新数据库 {:?}", db_path);
            format!("sqlite:{}", db_path.display())
        }
    };

    let pool = tranrss_backend::init_db(&database_url).await?;

    // 寄生模式不执行自动初始化（数据库已存在）
    if !config.is_parasitic() {
        tranrss_backend::auto_init_db(&pool).await?;
    } else {
        tracing::info!("📦 寄生模式: 跳过自动初始化，仅执行迁移");
        // 使用 auto_init_db 执行迁移（它会处理迁移和默认账号创建）
        tranrss_backend::auto_init_db(&pool).await?;
    }

    // 4. 构建应用状态
    let state = tranrss_backend::build_app_state(pool, event_tx).await?;

    // 5. 启动后台 Workers
    tranrss_backend::services::jobs::start_workers(state.clone()).await?;

    // 6. 构建路由
    let app = tranrss_backend::build_router(state);

    // 7. 启动服务器
    let addr: std::net::SocketAddr = ([0, 0, 0, 0], port).into();
    tracing::info!("🚀 TranRSS 启动于 http://{}", addr);

    match &config.database {
        DatabaseMode::Remote { server, .. } => {
            tracing::info!("📡 远程模式: {}", server);
        }
        DatabaseMode::Local { db_path } => {
            tracing::info!("📦 寄生模式: {}", db_path);
        }
        DatabaseMode::Fresh { data_dir } => {
            tracing::info!("🆕 全新模式: {}", data_dir);
        }
    }

    if !no_cron {
        tracing::info!("⏰ 定时任务已启用（系统调度器模式）");
    }

    if with_tui {
        tracing::info!("🖥️  TUI 模式已启用");
        // TODO: 在后台线程启动 TUI
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
