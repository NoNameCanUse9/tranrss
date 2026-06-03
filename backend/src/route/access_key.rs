use crate::AppState;
use crate::model::access_key::{
    AccessKey, AccessKeyInfo, CreateAccessKeyRequest, CreateAccessKeyResponse,
};
use crate::services::auth::AuthUser;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use rand::RngCore;
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_access_keys).post(create_access_key))
        .route("/{id}", get(get_access_key).delete(delete_access_key))
}

/// 列出当前用户的所有 Access Key
#[utoipa::path(
    get,
    path = "/api/user/access-keys",
    responses(
        (status = 200, description = "Success", body = Vec<AccessKeyInfo>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("jwt" = [])
    ),
    tag = "AccessKey"
)]
async fn list_access_keys(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<AccessKeyInfo>>, (StatusCode, String)> {
    let keys: Vec<AccessKey> =
        sqlx::query_as("SELECT id, user_id, key, name, permissions, created_at, last_used_at FROM access_keys WHERE user_id = ? ORDER BY created_at DESC")
            .bind(auth.user_id)
            .fetch_all(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(keys.iter().map(|k| k.to_info()).collect()))
}

/// 获取单个 Access Key 详情
#[utoipa::path(
    get,
    path = "/api/user/access-keys/{id}",
    params(
        ("id" = i64, Path, description = "Access Key ID")
    ),
    responses(
        (status = 200, description = "Success", body = AccessKeyInfo),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found")
    ),
    security(
        ("jwt" = [])
    ),
    tag = "AccessKey"
)]
async fn get_access_key(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<AccessKeyInfo>, (StatusCode, String)> {
    let key: Option<AccessKey> =
        sqlx::query_as("SELECT id, user_id, key, name, permissions, created_at, last_used_at FROM access_keys WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match key {
        Some(k) => Ok(Json(k.to_info())),
        None => Err((StatusCode::NOT_FOUND, "Access key not found".to_string())),
    }
}

/// 创建新的 Access Key
#[utoipa::path(
    post,
    path = "/api/user/access-keys",
    request_body = CreateAccessKeyRequest,
    responses(
        (status = 201, description = "Created", body = CreateAccessKeyResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("jwt" = [])
    ),
    tag = "AccessKey"
)]
async fn create_access_key(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateAccessKeyRequest>,
) -> Result<(StatusCode, Json<CreateAccessKeyResponse>), (StatusCode, String)> {
    // 生成 key: "trss_" + 32 字节 hex
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let key = format!("trss_{}", hex::encode(bytes));

    let permissions_json =
        serde_json::to_string(&payload.permissions).unwrap_or_else(|_| "[]".to_string());

    let res = sqlx::query(
        "INSERT INTO access_keys (user_id, key, name, permissions) VALUES (?, ?, ?, ?)",
    )
    .bind(auth.user_id)
    .bind(&key)
    .bind(&payload.name)
    .bind(&permissions_json)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let id = res.last_insert_rowid();

    Ok((
        StatusCode::CREATED,
        Json(CreateAccessKeyResponse {
            id,
            name: payload.name,
            key,
            permissions: payload.permissions,
        }),
    ))
}

/// 删除 Access Key
#[utoipa::path(
    delete,
    path = "/api/user/access-keys/{id}",
    params(
        ("id" = i64, Path, description = "Access Key ID")
    ),
    responses(
        (status = 200, description = "Deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not Found")
    ),
    security(
        ("jwt" = [])
    ),
    tag = "AccessKey"
)]
async fn delete_access_key(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM access_keys WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Access key not found".to_string()));
    }

    Ok(StatusCode::OK)
}
