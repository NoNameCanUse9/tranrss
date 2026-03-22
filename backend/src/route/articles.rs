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
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .unwrap_or(false);

    let mut stitched_content = detail
        .content_skeleton
        .as_deref()
        .unwrap_or_default()
        .to_string();
    for block in &blocks {
        let replacement = if is_need_translated {
            if let Some(ref trans) = block.trans_text {
                // 双语模式：原文 + 斜体翻译
                format!(
                    "{}<br><em class=\"trans-text\">{}</em>",
                    block.raw_text, trans
                )
            } else {
                // 尚未翻译，仅显示原文
                block.raw_text.clone()
            }
        } else {
            // 不需要翻译则直接用原文
            block.raw_text.clone()
        };
        stitched_content =
            stitched_content.replace(&format!("[[TEXT_{}]]", block.block_index), &replacement);
    }

    // 如果有翻译内容，注入内联 CSS
    if is_need_translated && blocks.iter().any(|b| b.trans_text.is_some()) {
        let inline_style = r#"<style>
em.trans-text {
  display: block;
  font-style: italic;
  color: inherit;
  opacity: 0.6;
  font-size: 0.95em;
  margin-top: 0.25em;
  padding-left: 0.75em;
  border-left: 2px solid #7986CB;
}
</style>"#;
        stitched_content = format!("{}{}", inline_style, stitched_content);
    }

    // 如果有 AI 生成的摘要，将其以绿色引用块的形式插入文章最前面
    if let Some(ref summary_text) = detail.summary {
        if !summary_text.trim().is_empty() {
            let summary_html = format!(
                r#"<style>
.ai-summary {{
  position: relative;
  background: linear-gradient(135deg, rgba(34,197,94,0.08) 0%, rgba(16,185,129,0.05) 100%);
  border-left: 3px solid #22c55e;
  border-radius: 0 8px 8px 0;
  padding: 1em 1.2em 1em 1.4em;
  margin: 0 0 1.8em 0;
  color: inherit;
  font-size: 0.97em;
  line-height: 1.7;
}}
.ai-summary::before {{
  content: '“';
  display: block;
  font-size: 2.4em;
  line-height: 0.8;
  color: #22c55e;
  font-family: Georgia, serif;
  margin-bottom: 0.1em;
  opacity: 0.85;
}}
.ai-summary::after {{
  content: '”';
  display: block;
  font-size: 2.4em;
  line-height: 0.5;
  color: #22c55e;
  font-family: Georgia, serif;
  text-align: right;
  margin-top: 0.2em;
  opacity: 0.85;
}}
</style>
<div class="ai-summary">{}</div>"#,
                summary_text
            );
            stitched_content = format!("{}{}", summary_html, stitched_content);
        }
    }

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
