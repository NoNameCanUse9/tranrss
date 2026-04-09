#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use axum::{
    Router,
    extract::State,
    http::{StatusCode, Uri},
    response::{IntoResponse, sse::{Event, Sse, KeepAlive}},
    routing::get,
};
#[cfg(feature = "embed-frontend")]
use axum::{
    body::Body,
    http::{Response, header},
};
use tower_http::trace::TraceLayer;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqliteSynchronous, SqlitePoolOptions};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tracing_subscriber::prelude::*;
use crate::utils::broadcast_layer::BroadcastLayer;

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
    pub event_tx: tokio::sync::broadcast::Sender<String>, // 全局事件发射器
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.event_tx.subscribe();

    let stream = async_stream::stream! {
        while let Ok(msg) = rx.recv().await {
            yield Ok(Event::default().data(msg));
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 先创建广播频道（日志层需要引用它）
    let (event_tx, _) = tokio::sync::broadcast::channel::<String>(256);

    // 2. 初始化日志（含 SSE 广播层）
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "tranrss_backend=info,tower_http=warn".into());

    let fmt_layer = tracing_subscriber::fmt::layer();
    let broadcast_layer = BroadcastLayer::new(event_tx.clone());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(broadcast_layer)
        .init();

    // 2. 数据库连接池初始化
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:/app/data/data.database".to_string());

    tracing::info!("📡 正在连接数据库: {}", database_url);

    let opts = SqliteConnectOptions::from_str(&database_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .create_if_missing(true)
        .pragma("cache_size", "-8000")        // 8MB 页缓存 (默认 ~2MB)
        .pragma("mmap_size", "268435456")      // 256MB 内存映射 I/O
        .pragma("temp_store", "memory")        // 临时表存内存
        .pragma("busy_timeout", "5000");       // 5秒忙等待，减少锁冲突

    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .min_connections(1)
        .idle_timeout(std::time::Duration::from_secs(300))
        .connect_with(opts)
        .await?;

    // 3. 首次启动自动初始化（建表 + 默认账号）
    auto_init_db(&pool).await?;

    // 4. 初始化任务队列存储
    let (sync_queue, translate_queue, summarize_queue, refresh_queue) =
        jobs::create_storages(pool.clone());
    // event_tx 已在 tracing 初始化前创建

    // 构建应用状态
    let state = Arc::new(AppState {
        db: pool.clone(),
        sync_queue,
        translate_queue,
        summarize_queue,
        refresh_queue,
        event_tx,
    });

    // 4. 启动后台 Workers
    jobs::start_workers(state.clone()).await?;

    // 5. 定义路由
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/events", get(sse_handler))
        .route("/api/internal/trigger_refresh_all", axum::routing::post(trigger_refresh_all_internal))
        .route("/users/count", get(get_user_count))
        .nest("/api/user", route::user::router())
        .nest("/api/translate-configs", route::translate_api::router())
        .nest("/api/feeds", route::subscriptions::router())
        .nest("/api/articles", route::articles::router())
        .nest("/api/jobs", route::jobs::router())
        .nest("/api/greader", route::greader::router())
        .nest("/api/fever", route::fever::router())
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

async fn trigger_refresh_all_internal(
    State(state): State<Arc<AppState>>,
) -> StatusCode {
    use crate::services::jobs::RefreshFeedsJob;
    use apalis::prelude::Storage;

    let mut storage = state.refresh_queue.clone();
    let _ = storage.push(RefreshFeedsJob).await;
    
    StatusCode::OK
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
        let fever_key = format!("{:x}", md5::compute("admin:admin"));
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (username, password_hash, fever_api_key) VALUES ('admin', ?, ?) RETURNING id",
        )
        .bind(&hash)
        .bind(&fever_key)
        .fetch_one(pool)
        .await?;

        sqlx::query("INSERT INTO user_setting (user_id, custom_trans_style) VALUES (?, ?)")
            .bind(user_id)
            .bind(
                "display: block;
font-style: italic;
opacity: 0.6;
font-size: 0.95em;
margin-top: 0.3rem;
padding-left: 0.75rem;
border-left: 2px solid rgba(var(--v-theme-primary), 0.4);",
            )
            .execute(pool)
            .await?;

        tracing::info!("✅ 默认账号已创建 — 用户名: admin  密码: admin（请登录后尽快修改密码）");
    } else {
        tracing::debug!("数据库已初始化，跳过自动建表");
    }

    // 仅在启动时做一次 VACUUM（如果 freelist 页过多）
    let freelist: i64 = sqlx::query_scalar("PRAGMA freelist_count")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    if freelist > 1000 {
        tracing::info!("🧹 检测到 {} 个空闲页，执行 VACUUM...", freelist);
        let _ = sqlx::query("VACUUM").execute(pool).await;
    }

    // --- JWT 密钥持久化逻辑 ---
    // 1. 尝试从数据库加载
    let db_jwt: Option<String> =
        sqlx::query_scalar("SELECT value FROM system_config WHERE key = 'jwt_secret'")
            .fetch_optional(pool)
            .await?;

    if let Some(ref hex_val) = db_jwt {
        if let Ok(bytes) = hex::decode(hex_val.trim()) {
            if bytes.len() >= 32 {
                let _ = crate::services::auth::init_jwt_secret(bytes);
                tracing::info!("JWT secret 已从数据库加载");
            }
        }
    }

    // 2. 如果没能初始化（数据库没记录或无效），则通过原有逻辑加载/生成，并存回数据库
    let final_secret = crate::services::auth::get_jwt_secret();

    // 🌟 这里是您的优化点：只有在查不到时才写入，避免只读报错
    if db_jwt.is_none() {
        let hex_val = hex::encode(final_secret);
        let _ = sqlx::query("INSERT OR REPLACE INTO system_config (key, value) VALUES ('jwt_secret', ?)")
            .bind(hex_val)
            .execute(pool)
            .await;
        tracing::info!("JWT secret 已生成并同步至数据库");
    }

    Ok(())
}
