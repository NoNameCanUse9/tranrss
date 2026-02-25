use axum::{Router, extract::State, routing::get};
use tower_http::trace::TraceLayer;

use sqlx::sqlite::SqlitePool;
use std::net::SocketAddr;

use std::sync::Arc;

mod model;
mod route;
mod services;
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tranrss_backend=debug,tower_http=debug".into()),
        )
        .init();

    // 2. 数据库连接池初始化
    // 注意：SQLite 需要文件存在。如果不存在，sqlx 会报错，除非开启了 create_if_missing 参数
    let database_url = "sqlite:../rssdata.db";
    let pool = SqlitePool::connect(database_url).await?;

    // 3. 构建应用状态
    let state = Arc::new(AppState { db: pool });

    // 4. 定义路由
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/users/count", get(get_user_count))
        .nest("/api/user", route::user::router())
        .nest("/api/translate-configs", route::translate_api::router())
        .nest("/api/subscriptions", route::subscriptions::router())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // 5. 启动服务器
    let addr = SocketAddr::from(([127, 0, 0, 1], 8002));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// --- Handler 示例 ---

async fn health_check() -> &'static str {
    "OK"
}

async fn get_user_count(
    State(state): State<Arc<AppState>>,
) -> Result<String, (axum::http::StatusCode, String)> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(|e: sqlx::Error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(format!("Total users: {}", count.0))
}
