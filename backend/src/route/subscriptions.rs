use crate::AppState;
use crate::model::subscriptions::{
    CreateSubscriptionRequest, SubscriptionDetail, UpdateSubscriptionRequest,
};

use crate::model::feed::CreateFeedRequest;
use crate::services::auth::AuthUser;
use crate::services::{feeds, subscription};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_subscription).get(list_subscriptions))
        .route(
            "/{id}",
            get(get_subscription)
                .put(update_subscription)
                .delete(delete_subscription),
        )
        .route("/{id}/sync", post(sync_subscription))
        .route("/preview", post(preview_feed))
}

async fn create_subscription(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateSubscriptionRequest>,
) -> Result<(StatusCode, Json<i64>), (StatusCode, String)> {
    let id = subscription::create_subscription(&state.db, auth.user_id, payload)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(id)))
}

async fn list_subscriptions(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<SubscriptionDetail>>, (StatusCode, String)> {
    let subscriptions = subscription::list_subscriptions(&state.db, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(subscriptions))
}

async fn get_subscription(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<SubscriptionDetail>, (StatusCode, String)> {
    let sub = subscription::get_subscription_detail(&state.db, auth.user_id, id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    Ok(Json(sub))
}

async fn update_subscription(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateSubscriptionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    subscription::update_subscription(&state.db, auth.user_id, id, payload)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

async fn delete_subscription(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    subscription::delete_subscription(&state.db, auth.user_id, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
async fn sync_subscription(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    let feed_id = subscription::get_feed_id_by_subscription(&state.db, auth.user_id, id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, format!("订阅不存在: {}", e)))?;

    feeds::fetch_and_process_feed(&state.db, auth.user_id, feed_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("同步失败: {}", e),
            )
        })?;

    Ok(StatusCode::OK)
}

#[derive(serde::Deserialize)]
pub struct PreviewRequest {
    pub url: String,
}

async fn preview_feed(
    _auth: AuthUser,
    Json(payload): Json<PreviewRequest>,
) -> Result<Json<CreateFeedRequest>, (StatusCode, String)> {
    let preview = feeds::fetch_feed_preview(&payload.url)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(preview))
}
