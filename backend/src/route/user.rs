use crate::AppState;
use crate::model::user::{
    LoginRequest, LoginResponse, RegisterRequest, UpdatePasswordRequest, UpdateUsernameRequest,
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
}

async fn register(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Result<StatusCode, (StatusCode, String)> {
    let payload: RegisterRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    // hash_password returns anyhow::Result
    let password_hash = auth::hash_password(&payload.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // sqlx returns sqlx::Error
    sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
        .bind(&payload.username)
        .bind(&password_hash)
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

    Ok(StatusCode::CREATED)
}

async fn login(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let payload: LoginRequest = serde_json::from_slice(body.as_ref())
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)))?;

    let user: (i64, String, String) =
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

    if !auth::verify_password(&payload.password, &user.2) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid username or password".to_string(),
        ));
    }

    let token = auth::create_token(user.0, &user.1)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LoginResponse {
        token,
        username: user.1,
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

    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(new_hash)
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
