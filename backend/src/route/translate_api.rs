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
        .route("/{id}", get(get_api).put(update_api).delete(delete_api))
}

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
