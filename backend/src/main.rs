use axum::{
    Router,
    extract::State,
    http::{StatusCode, Uri},
    response::IntoResponse,
    routing::get,
};
#[cfg(feature = "embed-frontend")]
use axum::{
    body::Body,
    http::{Response, header},
};
use tower_http::trace::TraceLayer;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqliteJournalMode, SqliteSynchronous};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

mod model;
mod route;
mod services;
mod utils;

use apalis_sql::sqlite::SqliteStorage;
use services::jobs::{
    self, RefreshFeedsJob, SummarizeArticleJob, SyncFeedJob, TranslateArticleJob,
};

// ── 内嵌前端（发布模式）──────────────────────────────────────────
// 编译时需先构建前端：pnpm --filter frontend build
// 然后：cargo build --release --features embed-frontend
#[cfg(feature = "embed-frontend")]
#[derive(rust_embed::RustEmbed)]
#[folder = "../frontend/dist"]
struct FrontendAssets;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub sync_queue: SqliteStorage<SyncFeedJob>,
    pub translate_queue: SqliteStorage<TranslateArticleJob>,
    pub summarize_queue: SqliteStorage<SummarizeArticleJob>,
    pub refresh_queue: SqliteStorage<RefreshFeedsJob>,
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
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:/app/data/data.database".to_string());

    tracing::info!("📡 正在连接数据库: {}", database_url);

    let opts = SqliteConnectOptions::from_str(&database_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(opts).await?;

    // 3. 首次启动自动初始化（建表 + 默认账号）
    auto_init_db(&pool).await?;

    // 4. 初始化任务队列存储
    let (sync_queue, translate_queue, summarize_queue, refresh_queue) =
        jobs::create_storages(pool.clone());

    // 构建应用状态
    let state = Arc::new(AppState {
        db: pool.clone(),
        sync_queue,
        translate_queue,
        summarize_queue,
        refresh_queue,
    });

    // 4. 启动后台 Workers
    jobs::start_workers(state.clone()).await?;

    // 5. 定义路由
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/users/count", get(get_user_count))
        .nest("/api/user", route::user::router())
        .nest("/api/translate-configs", route::translate_api::router())
        .nest("/api/feeds", route::subscriptions::router())
        .nest("/api/articles", route::articles::router())
        .nest("/api/jobs", route::jobs::router())
        .nest("/api/greader", route::greader::router())
        // 内嵌前端：发布模式走 SPA fallback，开发模式无此路由（由 Vite dev server 提供）
        .fallback(get(serve_frontend))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // 6. 启动服务器
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("🚀 TranRSS 启动于 http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?; 



    Ok(())
}

// ── 前端静态资源 Handler ─────────────────────────────────────────

/// 发布模式：从内嵌资源中 serve 前端，支持 SPA history 路由
#[cfg(feature = "embed-frontend")]
async fn serve_frontend(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // 精确路径匹配（/assets/xxx.js 等）
    if let Some(asset) = FrontendAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .header(
                header::CACHE_CONTROL,
                if path == "index.html" {
                    "no-cache"
                } else {
                    "public, max-age=31536000, immutable"
                },
            )
            .body(Body::from(asset.data.to_vec()))
            .unwrap();
    }

    // fallback → index.html（SPA 客户端路由）
    match FrontendAssets::get("index.html") {
        Some(index) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::from(index.data.to_vec()))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(
                "Frontend not embedded. Build with --features embed-frontend",
            ))
            .unwrap(),
    }
}

/// 开发模式：不内嵌前端，所有未匹配路由返回 404（前端由 Vite dev server 在 :8001 提供）
#[cfg(not(feature = "embed-frontend"))]
async fn serve_frontend(_uri: Uri) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        "Dev mode: frontend served by Vite on :8001",
    )
}

// ── 其他 Handler ─────────────────────────────────────────────────

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

// ── 首次启动自动初始化 ────────────────────────────────────────────

async fn auto_init_db(pool: &SqlitePool) -> anyhow::Result<()> {
    tracing::info!("🏃 Running database migrations...");
    sqlx::migrate!("./migrations")
        .set_ignore_missing(true)
        .run(pool)
        .await?;

    // 数据库迁移由 sqlx::migrate! 处理，此处无需手写 ALTER TABLE

    // 如果没有任何用户，自动创建 admin/admin
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    if user_count == 0 {
        tracing::info!("首次启动：自动创建默认账号 admin/admin");
        let hash = bcrypt::hash("admin", bcrypt::DEFAULT_COST)?;
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (username, password_hash) VALUES ('admin', ?) RETURNING id",
        )
        .bind(&hash)
        .fetch_one(pool)
        .await?;

        sqlx::query("INSERT INTO user_setting (user_id, custom_trans_style) VALUES (?, ?)")
            .bind(user_id)
            .bind("display: block;
font-style: italic;
opacity: 0.6;
font-size: 0.95em;
margin-top: 0.3rem;
padding-left: 0.75rem;
border-left: 2px solid rgba(var(--v-theme-primary), 0.4);")
            .execute(pool)
            .await?;

        tracing::info!("✅ 默认账号已创建 — 用户名: admin  密码: admin（请登录后尽快修改密码）");
    } else {
        tracing::debug!("数据库已初始化，跳过自动建表");
    }

    Ok(())
}
