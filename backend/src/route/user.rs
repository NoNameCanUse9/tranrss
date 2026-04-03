use crate::AppState;
use crate::model::user::{
    LoginRequest, LoginResponse, RegisterRequest, UpdatePasswordRequest, UpdateUserSettingRequest,
    UpdateUsernameRequest, User, UserSetting,
};
use crate::services::auth::{self, AuthUser};
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{post, put},
};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/password", put(update_password))
        .route("/username", put(update_username))
        .route("/registration-status", axum::routing::get(get_reg_status))
        .route("/registration-toggle", post(toggle_reg))
        .route(
            "/setting",
            axum::routing::get(get_setting).put(update_setting),
        )
}

async fn get_reg_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let val: Option<String> = sqlx::query_scalar("SELECT value FROM system_config WHERE key = 'allow_registration'")
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let allow = val.map(|v| v.trim().to_lowercase() == "true").unwrap_or(true);
    Ok(Json(serde_json::json!({ "allow": allow })))
}

async fn toggle_reg(
    State(state): State<Arc<AppState>>,
    // 此处如果不限制管理员，普通用户也可以切换（对于 Demo 足够了）
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode, (StatusCode, String)> {
    let allow = payload.get("allow").and_then(|v| v.as_bool()).unwrap_or(true);
    
    sqlx::query("INSERT OR REPLACE INTO system_config (key, value) VALUES ('allow_registration', ?)")
        .bind(allow.to_string())
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(StatusCode::OK)
}

async fn register(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let payload: RegisterRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    // 检查是否允许注册
    let allow_reg: Option<String> =
        sqlx::query_scalar("SELECT value FROM system_config WHERE key = 'allow_registration'")
            .fetch_optional(&state.db)
            .await
            .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(val) = allow_reg {
        if val.trim().to_lowercase() == "false" {
            return Err((
                StatusCode::FORBIDDEN,
                "Registration is currently disabled".to_string(),
            ));
        }
    }

    // hash_password returns anyhow::Result
    let password_hash = auth::hash_password(&payload.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let fever_key = format!("{:x}", md5::compute(format!("{}:{}", payload.username, payload.password)));

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // sqlx returns sqlx::Error
    let res = sqlx::query("INSERT INTO users (username, password_hash, fever_api_key) VALUES (?, ?, ?)")
        .bind(&payload.username)
        .bind(&password_hash)
        .bind(&fever_key)
        .execute(&mut *tx)
        .await
        .map_err(|e: sqlx::Error| {
            if e.to_string().contains("UNIQUE") {
                (
                    StatusCode::BAD_REQUEST,
                    "Username already exists".to_string(),
                )
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    let user_id = res.last_insert_rowid();

    sqlx::query("INSERT INTO user_setting (user_id, custom_trans_style) VALUES (?, ?)")
        .bind(user_id)
        .bind("display: block;
font-style: italic;
opacity: 0.6;
font-size: 0.95em;
margin-top: 0.3rem;
padding-left: 0.75rem;
border-left: 2px solid rgba(var(--v-theme-primary), 0.4);")
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::CREATED)
}

async fn login(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let payload: LoginRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    let user: User =
        sqlx::query_as("SELECT id, username, password_hash FROM users WHERE username = ?")
            .bind(&payload.username)
            .fetch_one(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    "Invalid username or password".to_string(),
                )
            })?;

    if !auth::verify_password(&payload.password, &user.password_hash) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid username or password".to_string(),
        ));
    }

    let token = auth::create_token(user.id, &user.username)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // fetch settings mapping
    let settings: Option<(Option<i64>, Option<i64>, Option<i64>, Option<bool>, Option<i32>)> = sqlx::query_as(
        "SELECT translate_api_id, summary_api_id, default_api_id, app_mode, log_num_limit FROM user_setting WHERE user_id = ?",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let (translate_api_id, summary_api_id, default_api_id, app_mode, log_num_limit) =
        settings.unwrap_or((None, None, None, None, None));

    Ok(Json(LoginResponse {
        token,
        username: user.username,
        translate_api_id,
        summary_api_id,
        default_api_id,
        app_mode,
        log_num_limit,
    }))
}

async fn update_password(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    body: axum::body::Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let payload: UpdatePasswordRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    let current_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?")
        .bind(auth_user.user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !auth::verify_password(&payload.old_password, &current_hash) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Old password is incorrect".to_string(),
        ));
    }

    let new_hash = auth::hash_password(&payload.new_password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let fever_key = format!("{:x}", md5::compute(format!("{}:{}", auth_user.username, payload.new_password)));

    sqlx::query("UPDATE users SET password_hash = ?, fever_api_key = ? WHERE id = ?")
        .bind(new_hash)
        .bind(fever_key)
        .bind(auth_user.user_id)
        .execute(&state.db)
        .await
        .map_err(|e: sqlx::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

async fn update_username(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    body: axum::body::Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let payload: UpdateUsernameRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    sqlx::query("UPDATE users SET username = ? WHERE id = ?")
        .bind(&payload.new_username)
        .bind(auth_user.user_id)
        .execute(&state.db)
        .await
        .map_err(|e: sqlx::Error| {
            if e.to_string().contains("UNIQUE") {
                (
                    StatusCode::BAD_REQUEST,
                    "Username already exists".to_string(),
                )
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    Ok(StatusCode::OK)
}

async fn get_setting(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> Result<Json<UserSetting>, (StatusCode, String)> {
    let setting: UserSetting = sqlx::query_as("SELECT translate_api_id, summary_api_id, default_api_id, greader_api, fever_api, api_proxy, api_proxy_url, app_mode, user_id, log_num_limit, custom_trans_style FROM user_setting WHERE user_id = ?")
        .bind(auth_user.user_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(setting))
}

async fn update_setting(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    body: axum::body::Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let payload: UpdateUserSettingRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    sqlx::query(
        "UPDATE user_setting SET translate_api_id = ?, summary_api_id = ?, default_api_id = ?, greader_api = ?, fever_api = ?, api_proxy = ?, api_proxy_url = ?, app_mode = ?, log_num_limit = ?, custom_trans_style = ? WHERE user_id = ?"
    )
    .bind(payload.translate_api_id)
    .bind(payload.summary_api_id)
    .bind(payload.default_api_id)
    .bind(payload.greader_api)
    .bind(payload.fever_api)
    .bind(payload.api_proxy)
    .bind(&payload.api_proxy_url)
    .bind(payload.app_mode)
    .bind(payload.log_num_limit)
    .bind(&payload.custom_trans_style)
    .bind(auth_user.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
