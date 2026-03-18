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

use sqlx::sqlite::SqlitePool;
use std::net::SocketAddr;
use std::sync::Arc;

mod model;
mod route;
mod services;

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
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        // 智能路径检测：优先使用当前目录或上级目录存在的 rssdata.db
        let potential_paths = ["rssdata.db", "../rssdata.db"];
        let mut chosen_path = "../rssdata.db"; // 默认回退
        
        for path in potential_paths {
            if let Ok(metadata) = std::fs::metadata(path) {
                if metadata.is_file() && metadata.len() > 0 {
                    chosen_path = path;
                    break;
                }
            }
        }
        format!("sqlite:{}", chosen_path)
    });
    
    tracing::info!("📡 正在连接数据库: {}", database_url);
    let pool = SqlitePool::connect(&database_url).await?;

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
        .nest("/api/subscriptions", route::subscriptions::router())
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
                if path == "index.html" { "no-cache" } else { "public, max-age=31536000, immutable" },
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
            .body(Body::from("Frontend not embedded. Build with --features embed-frontend"))
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
    // 建表（IF NOT EXISTS，幂等，始终安全执行）
    sqlx::query(r#"CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL
    )"#).execute(pool).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS feeds (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        feed_url TEXT NOT NULL UNIQUE,
        site_url TEXT, title TEXT NOT NULL, description TEXT,
        last_fetched_at DATETIME, etag TEXT, icon_url TEXT,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        last_status_code INTEGER, last_error TEXT
    )"#).execute(pool).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS folders (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        user_id INTEGER NOT NULL,
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    )"#).execute(pool).await?;

    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_folders_user_title ON folders(user_id, title)")
        .execute(pool).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS subscriptions (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        user_id INTEGER NOT NULL, feed_id INTEGER NOT NULL, folder_id INTEGER,
        custom_title TEXT, need_translate BOOLEAN DEFAULT 0,
        need_summary BOOLEAN DEFAULT 0, target_language TEXT,
        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
        NUM INTEGER DEFAULT 200, refresh_interval INTEGER DEFAULT 30,
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
        FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE,
        FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE SET NULL
    )"#).execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions(user_id)")
        .execute(pool).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS api_configs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL, api_type TEXT NOT NULL, api_key TEXT,
        base_url TEXT, settings TEXT NOT NULL,
        timeout_seconds INTEGER DEFAULT 180,
        retry_count INTEGER DEFAULT 3, retry_interval_ms INTEGER DEFAULT 1000,
        retry_enabled BOOLEAN DEFAULT 1,
        user_id INTEGER REFERENCES users(id) ON DELETE CASCADE
    )"#).execute(pool).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS articles (
        id INTEGER PRIMARY KEY, original_guid TEXT NOT NULL UNIQUE,
        feed_id INTEGER NOT NULL, title TEXT NOT NULL,
        link TEXT, author TEXT, published_at INTEGER, content_skeleton TEXT,
        is_read INTEGER DEFAULT 0, is_starred INTEGER DEFAULT 0,
        updated_at DATETIME DEFAULT CURRENT_TIMESTAMP, summary TEXT,
        FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE
    )"#).execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_feed_unread ON articles(feed_id, is_read)")
        .execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS articles_feed_id ON articles(feed_id)")
        .execute(pool).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS article_blocks (
        user_id INTEGER NOT NULL, article_id INTEGER NOT NULL,
        block_index INTEGER NOT NULL, raw_text TEXT NOT NULL, trans_text TEXT,
        PRIMARY KEY (user_id, article_id, block_index),
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
        FOREIGN KEY (article_id) REFERENCES articles(id) ON DELETE CASCADE
    )"#).execute(pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_article_blocks_lookup ON article_blocks(user_id, article_id)")
        .execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_article_ordered ON article_blocks(article_id, block_index)")
        .execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_untranslated ON article_blocks(user_id) WHERE trans_text IS NULL")
        .execute(pool).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS user_setting (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        user_id INTEGER NOT NULL UNIQUE,
        translate_api_id INTEGER, summary_api_id INTEGER,
        greader_api BOOLEAN, api_proxy BOOLEAN, api_proxy_url TEXT,
        app_mode BOOLEAN DEFAULT 0, log_num_limit INTEGER DEFAULT 300,
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    )"#).execute(pool).await?;

    // Apalis 任务队列表
    sqlx::query(r#"CREATE TABLE IF NOT EXISTS Workers (
        id TEXT NOT NULL UNIQUE, worker_type TEXT NOT NULL,
        storage_name TEXT NOT NULL, layers TEXT,
        last_seen INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
    )"#).execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS Idx   ON Workers(id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS WTIdx ON Workers(worker_type)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS LSIdx ON Workers(last_seen)").execute(pool).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS Jobs (
        job TEXT NOT NULL, id TEXT NOT NULL UNIQUE, job_type TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'Pending',
        attempts INTEGER NOT NULL DEFAULT 0,
        max_attempts INTEGER NOT NULL DEFAULT 25,
        run_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
        last_error TEXT, lock_at INTEGER, lock_by TEXT, done_at INTEGER,
        priority INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY(lock_by) REFERENCES Workers(id)
    )"#).execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS TIdx  ON Jobs(id)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS SIdx  ON Jobs(status)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS LIdx  ON Jobs(lock_by)").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS JTIdx ON Jobs(job_type)").execute(pool).await?;

    sqlx::query(r#"CREATE TABLE IF NOT EXISTS _sqlx_migrations (
        version BIGINT PRIMARY KEY, description TEXT NOT NULL,
        installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        success BOOLEAN NOT NULL, checksum BLOB NOT NULL,
        execution_time BIGINT NOT NULL
    )"#).execute(pool).await?;

    // 如果没有任何用户，自动创建 admin/admin
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool).await?;

    if user_count == 0 {
        tracing::info!("首次启动：自动创建默认账号 admin/admin");
        let hash = bcrypt::hash("admin", bcrypt::DEFAULT_COST)?;
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (username, password_hash) VALUES ('admin', ?) RETURNING id"
        )
        .bind(&hash)
        .fetch_one(pool).await?;

        sqlx::query("INSERT INTO user_setting (user_id) VALUES (?)")
            .bind(user_id)
            .execute(pool).await?;

        tracing::info!("✅ 默认账号已创建 — 用户名: admin  密码: admin（请登录后尽快修改密码）");
    } else {
        tracing::debug!("数据库已初始化，跳过自动建表");
    }

    Ok(())
}
