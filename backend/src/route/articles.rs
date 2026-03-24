use crate::AppState;
use crate::services::articles;
use crate::services::auth::AuthUser;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use apalis::prelude::*;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ListArticlesQuery {
    pub feed_id: Option<i64>,
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_articles))
        .route("/{id}", get(get_article))
        .route("/{id}/read", post(mark_read))
        .route("/{id}/star", post(mark_starred))
        .route("/{id}/translate", post(translate_article))
        .route("/{id}/summarize", post(summarize_article))
        .route("/translate-titles", post(batch_translate_titles))
}

async fn batch_translate_titles(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let ai = crate::services::jobs::get_default_ai_service_for_user(&state.db, auth.user_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let count = ai.translate_titles_batch(&state, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "translated_count": count
    })))
}

async fn translate_article(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    // 1. 获取有效 API ID (优先指定，次之默认，最后回退至最前)
    let translate_api_id: Option<i64> = sqlx::query_scalar("SELECT translate_api_id FROM user_setting WHERE user_id = ?")
            .bind(auth.user_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch user settings: {}", e)))?;

    let effective_id = crate::services::api::get_effective_api_id(&state.db, auth.user_id, translate_api_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if effective_id.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Please configure a translation API in settings first".to_string(),
        ));
    }

    // 5. 将任务推入异步队列
    let mut storage = state.translate_queue.clone();
    storage
        .push(crate::services::jobs::TranslateArticleJob {
            user_id: auth.user_id,
            article_id: id,
        })
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to enqueue translation task: {}", e),
            )
        })?;

    Ok(StatusCode::ACCEPTED)
}

async fn summarize_article(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    // 1. 获取有效 API ID
    let summary_api_id: Option<i64> = sqlx::query_scalar("SELECT summary_api_id FROM user_setting WHERE user_id = ?")
            .bind(auth.user_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch user settings: {}", e)))?;

    let effective_id = crate::services::api::get_effective_api_id(&state.db, auth.user_id, summary_api_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if effective_id.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Please configure a summary API in settings first".to_string(),
        ));
    }

    // 5. 将任务推入异步队列，这样就能在“运行日志”中看到并管理它
    let mut storage = state.summarize_queue.clone();
    let job = crate::services::jobs::SummarizeArticleJob {
        user_id: auth.user_id,
        article_id: id,
    };
    
    storage
        .push(job)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to enqueue summary task: {}", e),
            )
        })?;

    Ok(StatusCode::ACCEPTED)
}


async fn list_articles(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(params): Query<ListArticlesQuery>,
) -> Result<Json<Vec<crate::model::articles::ArticleListItem>>, (StatusCode, String)> {
    let articles = articles::list_articles(
        &state.db,
        auth.user_id,
        params.feed_id,
        params.is_read,
        params.is_starred,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(articles))
}

async fn mark_starred(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode, (StatusCode, String)> {
    let starred = payload
        .get("starred")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    articles::mark_starred(&state.db, auth.user_id, id, starred)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

async fn get_article(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let detail = articles::get_article_detail(&state.db, auth.user_id, id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    let blocks = articles::get_article_blocks(&state.db, auth.user_id, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 使用用户要求的方法获取是否需要翻译
    let is_need_translated: bool = sqlx::query_scalar(
        r#"
        SELECT s.need_translate
        FROM subscriptions s
        JOIN articles a ON a.feed_id = s.feed_id
        WHERE a.id = ? AND s.user_id = ?
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    // 2. 拼合内容
    let stitched_content = articles::stitch_article_content(
        detail.content_skeleton.as_deref().unwrap_or_default(),
        &blocks,
        detail.summary.as_deref(),
        is_need_translated,
    );

    Ok(Json(serde_json::json!({
        "detail": detail,
        "blocks": blocks,
        "content": stitched_content,
        "is_need_translated": is_need_translated
    })))
}

async fn mark_read(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode, (StatusCode, String)> {
    let read = payload
        .get("read")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    articles::mark_read(&state.db, auth.user_id, id, read)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
