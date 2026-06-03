use anyhow::Result;
use tracing_subscriber::prelude::*;
use tranrss_backend::utils::broadcast_layer::BroadcastLayer;

pub async fn run(port: u16, with_tui: bool, no_cron: bool) -> Result<()> {
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

    // 3. 数据库初始化
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:/app/data/data.database".to_string());
    let pool = tranrss_backend::init_db(&database_url).await?;
    tranrss_backend::auto_init_db(&pool).await?;

    // 4. 构建应用状态
    let state = tranrss_backend::build_app_state(pool, event_tx).await?;

    // 5. 启动后台 Workers
    tranrss_backend::services::jobs::start_workers(state.clone()).await?;

    // 6. 构建路由
    let app = tranrss_backend::build_router(state);

    // 7. 启动服务器
    let addr: std::net::SocketAddr = ([0, 0, 0, 0], port).into();
    tracing::info!("🚀 TranRSS 启动于 http://{}", addr);

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
