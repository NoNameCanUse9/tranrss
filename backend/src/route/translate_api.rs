use crate::AppState;
use crate::model::api_config::{ApiConfig, CreateApiConfigRequest, UpdateApiConfigRequest};
use crate::services::api;
use crate::services::auth::AuthUser;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_apis).post(create_api))
        .route("/usage", get(get_usage))
        .route("/usage/history", get(get_usage_history))
        .route("/{id}", get(get_api).put(update_api).delete(delete_api))
}

/// 创建 API 配置
#[utoipa::path(
    post,
    path = "/api/translate-configs",
    request_body = CreateApiConfigRequest,
    responses(
        (status = 201, description = "Created", body = i64)
    ),
    security(
        ("jwt" = [])
    ),
    tag = "API Config"
)]
async fn create_api(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateApiConfigRequest>,
) -> Result<(StatusCode, Json<i64>), (StatusCode, String)> {
    let id = api::create_config(&state.db, auth.user_id, payload)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(id)))
}

/// 获取所有 API 配置
#[utoipa::path(
    get,
    path = "/api/translate-configs",
    responses(
        (status = 200, description = "Success", body = Vec<ApiConfig>)
    ),
    security(
        ("jwt" = [])
    ),
    tag = "API Config"
)]
async fn list_apis(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<ApiConfig>>, (StatusCode, String)> {
    let configs = api::list_configs(&state.db, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::debug!(
        "Found {} API configs for user {}",
        configs.len(),
        auth.user_id
    );
    Ok(Json(configs))
}

/// 获取单个 API 配置
#[utoipa::path(
    get,
    path = "/api/translate-configs/{id}",
    params(
        ("id" = i64, Path, description = "API Config ID")
    ),
    responses(
        (status = 200, description = "Success", body = ApiConfig),
        (status = 404, description = "Not Found")
    ),
    security(
        ("jwt" = [])
    ),
    tag = "API Config"
)]
async fn get_api(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<ApiConfig>, (StatusCode, String)> {
    let config = api::get_config(&state.db, id, auth.user_id)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                "API config not found or access denied".to_string(),
            )
        })?;

    Ok(Json(config))
}

/// 更新 API 配置
#[utoipa::path(
    put,
    path = "/api/translate-configs/{id}",
    params(
        ("id" = i64, Path, description = "API Config ID")
    ),
    request_body = UpdateApiConfigRequest,
    responses(
        (status = 200, description = "Success"),
        (status = 404, description = "Not Found")
    ),
    security(
        ("jwt" = [])
    ),
    tag = "API Config"
)]
async fn update_api(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateApiConfigRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    api::update_config(&state.db, id, auth.user_id, payload)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

/// 删除 API 配置
#[utoipa::path(
    delete,
    path = "/api/translate-configs/{id}",
    params(
        ("id" = i64, Path, description = "API Config ID")
    ),
    responses(
        (status = 204, description = "Deleted")
    ),
    security(
        ("jwt" = [])
    ),
    tag = "API Config"
)]
async fn delete_api(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    api::delete_config(&state.db, id, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// 获取 API 使用统计
#[utoipa::path(
    get,
    path = "/api/translate-configs/usage",
    responses(
        (status = 200, description = "Success", body = crate::model::api_usage::ApiUsageStats)
    ),
    security(
        ("jwt" = [])
    ),
    tag = "API Config"
)]
async fn get_usage(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<crate::model::api_usage::ApiUsageStats>, (StatusCode, String)> {
    let stats = api::get_usage_summary(&state.db, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(stats))
}

/// 获取 API 使用历史
#[utoipa::path(
    get,
    path = "/api/translate-configs/usage/history",
    responses(
        (status = 200, description = "Success", body = Vec<crate::model::api_usage::TimeSeriesUsage>)
    ),
    security(
        ("jwt" = [])
    ),
    tag = "API Config"
)]
async fn get_usage_history(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<crate::model::api_usage::TimeSeriesUsage>>, (StatusCode, String)> {
    let history = api::get_usage_history(&state.db, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(history))
}
