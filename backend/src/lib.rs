pub mod model;
pub mod route;
pub mod services;
pub mod utils;

use apalis_sql::sqlite::SqliteStorage;
use services::jobs::{
    RefreshFeedsJob, SummarizeArticleJob, SyncFeedJob, TranslateArticleJob,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous};
use std::str::FromStr;
use std::sync::Arc;

/// 应用状态，包含数据库连接池、任务队列和事件广播
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub sync_queue: SqliteStorage<SyncFeedJob>,
    pub translate_queue: SqliteStorage<TranslateArticleJob>,
    pub summarize_queue: SqliteStorage<SummarizeArticleJob>,
    pub refresh_queue: SqliteStorage<RefreshFeedsJob>,
    pub event_tx: tokio::sync::broadcast::Sender<String>,
}

/// 初始化数据库连接池
pub async fn init_db(database_url: &str) -> anyhow::Result<SqlitePool> {
    tracing::info!("📡 正在连接数据库: {}", database_url);

    let opts = SqliteConnectOptions::from_str(database_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .create_if_missing(true)
        .pragma("cache_size", "-8000")
        .pragma("mmap_size", "268435456")
        .pragma("temp_store", "memory")
        .pragma("busy_timeout", "5000");

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .idle_timeout(std::time::Duration::from_secs(300))
        .connect_with(opts)
        .await?;

    Ok(pool)
}

/// 首次启动自动初始化（建表 + 默认账号 + JWT 密钥）
pub async fn auto_init_db(pool: &SqlitePool) -> anyhow::Result<()> {
    tracing::info!("🏃 Running database migrations...");
    sqlx::migrate!("./migrations")
        .set_ignore_missing(true)
        .run(pool)
        .await?;

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

    // VACUUM
    let freelist: i64 = sqlx::query_scalar("PRAGMA freelist_count")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    if freelist > 1000 {
        tracing::info!("🧹 检测到 {} 个空闲页，执行 VACUUM...", freelist);
        let _ = sqlx::query("VACUUM").execute(pool).await;
    }

    // JWT 密钥持久化
    let db_jwt: Option<String> =
        sqlx::query_scalar("SELECT value FROM system_config WHERE key = 'jwt_secret'")
            .fetch_optional(pool)
            .await?;

    if let Some(ref hex_val) = db_jwt {
        if let Ok(bytes) = hex::decode(hex_val.trim()) {
            if bytes.len() >= 32 {
                let _ = services::auth::init_jwt_secret(bytes);
                tracing::info!("JWT secret 已从数据库加载");
            }
        }
    }

    let final_secret = services::auth::get_jwt_secret();

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

/// 构建应用状态
pub async fn build_app_state(pool: SqlitePool, event_tx: tokio::sync::broadcast::Sender<String>) -> anyhow::Result<Arc<AppState>> {
    let (sync_queue, translate_queue, summarize_queue, refresh_queue) =
        services::jobs::create_storages(pool.clone());

    Ok(Arc::new(AppState {
        db: pool,
        sync_queue,
        translate_queue,
        summarize_queue,
        refresh_queue,
        event_tx,
    }))
}

/// 构建 Axum 路由
pub fn build_router(state: Arc<AppState>) -> axum::Router {
    use axum::routing::get;
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::trace::TraceLayer;
    use utoipa::OpenApi;
    use utoipa_swagger_ui::SwaggerUi;

    // OpenAPI doc
    #[derive(OpenApi)]
    #[openapi(
        paths(
            route::user::get_reg_status,
            route::user::toggle_reg,
            route::user::register,
            route::user::login,
            route::user::update_password,
            route::user::update_username,
            route::user::get_setting,
            route::user::update_setting,
            route::articles::batch_translate_titles,
            route::articles::translate_article,
            route::articles::summarize_article,
            route::articles::list_articles,
            route::articles::mark_starred,
            route::articles::get_article,
            route::articles::mark_read,
            route::subscriptions::list_inactive_feeds,
            route::subscriptions::activate_inactive_feeds,
            route::subscriptions::create_subscription,
            route::subscriptions::list_subscriptions,
            route::subscriptions::get_subscription,
            route::subscriptions::update_subscription,
            route::subscriptions::delete_subscription,
            route::subscriptions::sync_subscription,
            route::subscriptions::sync_all_subscriptions,
            route::subscriptions::preview_feed,
            route::subscriptions::export_opml,
            route::subscriptions::import_opml,
            route::translate_api::create_api,
            route::translate_api::list_apis,
            route::translate_api::get_api,
            route::translate_api::update_api,
            route::translate_api::delete_api,
            route::translate_api::get_usage,
            route::translate_api::get_usage_history,
            route::jobs::trigger_refresh_all,
            route::jobs::clear_completed,
            route::jobs::retry_job,
            route::jobs::get_jobs,
            route::share::share_feed,
            route::fever::fever_handler,
            route::access_key::list_access_keys,
            route::access_key::create_access_key,
            route::access_key::delete_access_key,
        ),
        components(
            schemas(
                model::user::RegisterRequest,
                model::user::User,
                model::user::LoginRequest,
                model::user::LoginResponse,
                model::user::UpdatePasswordRequest,
                model::user::UpdateUsernameRequest,
                model::user::UpdateUserSettingRequest,
                model::user::UserSetting,
                model::articles::ArticleListItem,
                model::articles::ArticleDetail,
                model::articles::ArticleBlock,
                model::subscriptions::SubscriptionDetail,
                model::subscriptions::CreateSubscriptionRequest,
                model::subscriptions::UpdateSubscriptionRequest,
                route::subscriptions::InactiveFeed,
                route::subscriptions::ActivateRequest,
                route::subscriptions::PreviewRequest,
                model::api_config::ApiConfig,
                model::api_config::CreateApiConfigRequest,
                model::api_config::UpdateApiConfigRequest,
                model::access_key::AccessKey,
                model::access_key::AccessKeyInfo,
                model::access_key::CreateAccessKeyRequest,
                model::access_key::CreateAccessKeyResponse,
                model::api_usage::ApiUsageStats,
                model::api_usage::ModelUsage,
                model::api_usage::TimeSeriesUsage,
                route::jobs::JobInfo,
                model::feed::CreateFeedRequest,
            )
        ),
        tags(
            (name = "User", description = "User management APIs"),
            (name = "Articles", description = "Article management APIs"),
            (name = "Subscriptions", description = "RSS Subscription management APIs"),
            (name = "API Config", description = "AI API configuration management APIs"),
            (name = "Jobs", description = "Background jobs monitoring APIs"),
            (name = "Compatibility", description = "Third-party protocol compatibility APIs (Fever, GReader, etc.)"),
            (name = "AccessKey", description = "Access Key management APIs for CLI/TUI/Agent"),
        ),
        modifiers(&SecurityAddon)
    )]
    struct ApiDoc;

    struct SecurityAddon;
    impl utoipa::Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            if let Some(components) = openapi.components.as_mut() {
                components.add_security_scheme(
                    "jwt",
                    utoipa::openapi::security::SecurityScheme::Http(
                        utoipa::openapi::security::HttpBuilder::new()
                            .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                            .bearer_format("JWT")
                            .build(),
                    ),
                )
            }
        }
    }

    // SSE handler
    async fn sse_handler(
        axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    ) -> axum::response::Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
        let mut rx = state.event_tx.subscribe();
        let stream = async_stream::stream! {
            while let Ok(msg) = rx.recv().await {
                yield Ok(axum::response::sse::Event::default().data(msg));
            }
        };
        axum::response::Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
    }

    async fn health_check() -> &'static str { "OK" }

    async fn get_user_count(
        axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    ) -> Result<String, (axum::http::StatusCode, String)> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&state.db)
            .await
            .map_err(|e: sqlx::Error| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(format!("Total users: {}", count.0))
    }

    async fn trigger_refresh_all_internal(
        axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    ) -> axum::http::StatusCode {
        use apalis::prelude::Storage;
        let mut storage = state.refresh_queue.clone();
        let _ = storage.push(RefreshFeedsJob).await;
        axum::http::StatusCode::OK
    }

    // Frontend serving
    #[cfg(feature = "embed-frontend")]
    #[derive(rust_embed::RustEmbed)]
    #[folder = "../frontend/dist"]
    struct FrontendAssets;

    #[cfg(feature = "embed-frontend")]
    async fn serve_frontend(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
        let path = uri.path().trim_start_matches('/');
        if let Some(asset) = FrontendAssets::get(path) {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            return axum::response::Response::builder()
                .status(axum::http::StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, mime.as_ref())
                .header(axum::http::header::CACHE_CONTROL, if path == "index.html" { "no-cache" } else { "public, max-age=31536000, immutable" })
                .body(axum::body::Body::from(asset.data.to_vec()))
                .unwrap();
        }
        match FrontendAssets::get("index.html") {
            Some(index) => axum::response::Response::builder()
                .status(axum::http::StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")
                .header(axum::http::header::CACHE_CONTROL, "no-cache")
                .body(axum::body::Body::from(index.data.to_vec()))
                .unwrap(),
            None => axum::response::Response::builder()
                .status(axum::http::StatusCode::NOT_FOUND)
                .body(axum::body::Body::from("Frontend not embedded. Build with --features embed-frontend"))
                .unwrap(),
        }
    }

    #[cfg(not(feature = "embed-frontend"))]
    async fn serve_frontend(_uri: axum::http::Uri) -> impl axum::response::IntoResponse {
        (axum::http::StatusCode::NOT_FOUND, "Dev mode: frontend served by Vite on :8001")
    }

    axum::Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/health", get(health_check))
        .route("/api/events", get(sse_handler))
        .route("/api/internal/trigger_refresh_all", axum::routing::post(trigger_refresh_all_internal))
        .route("/users/count", get(get_user_count))
        .nest("/api/user", route::user::router())
        .nest("/api/user/access-keys", route::access_key::router())
        .nest("/api/translate-configs", route::translate_api::router())
        .nest("/api/feeds", route::subscriptions::router())
        .nest("/api/articles", route::articles::router())
        .nest("/api/jobs", route::jobs::router())
        .nest("/api/greader", route::greader::router())
        .merge(route::greader::router())
        .nest("/api/fever", route::fever::router())
        .route("/share/{feed_title}", get(route::share::share_feed))
        .fallback(get(serve_frontend))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state)
}
