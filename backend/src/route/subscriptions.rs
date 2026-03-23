use crate::AppState;
use crate::model::subscriptions::{
    CreateSubscriptionRequest, SubscriptionDetail, UpdateSubscriptionRequest,
};

use crate::model::feed::CreateFeedRequest;
use crate::services::auth::AuthUser;
use crate::services::jobs::SyncFeedJob;
use crate::services::{feeds, subscription};
use apalis::prelude::Storage;
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
        .route("/sync_all", post(sync_all_subscriptions))
        .route("/{id}/sync", post(sync_subscription))
        .route("/preview", post(preview_feed))
        .route("/opml", get(export_opml).post(import_opml))
        .route("/inactive", get(list_inactive_feeds))
        .route("/inactive/activate", post(activate_inactive_feeds))
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct InactiveFeed {
    pub feed_id: i64,
    pub title: String,
    pub url: String,
    pub reason: Option<String>,
    pub disabled_at: String,
}

async fn list_inactive_feeds(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<InactiveFeed>>, (StatusCode, String)> {
    let inactive = sqlx::query_as::<_, InactiveFeed>(
        r#"
        SELECT f.id as feed_id, f.title, f.feed_url as url, inf.reason, inf.disabled_at
        FROM inactive_feeds inf
        JOIN feeds f ON inf.feed_id = f.id
        WHERE inf.user_id = ?
        ORDER BY inf.disabled_at DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(inactive))
}

#[derive(serde::Deserialize)]
pub struct ActivateRequest {
    pub feed_ids: Vec<i64>,
}

async fn activate_inactive_feeds(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<ActivateRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut tx = state.db.begin().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    for fid in payload.feed_ids {
        // 从失效表移除
        sqlx::query("DELETE FROM inactive_feeds WHERE user_id = ? AND feed_id = ?")
            .bind(auth.user_id)
            .bind(fid)
            .execute(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        // 重置失败计数
        sqlx::query("UPDATE feeds SET consecutive_fetch_failures = 0, last_error = NULL WHERE id = ?")
            .bind(fid)
            .execute(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
        // 立即触发一次同步任务以尝试恢复
        let mut storage = state.sync_queue.clone();
        let _ = storage.push(SyncFeedJob {
            feed_id: fid,
            initiator_user_id: Some(auth.user_id),
        }).await;
    }

    tx.commit().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

async fn create_subscription(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateSubscriptionRequest>,
) -> Result<(StatusCode, Json<i64>), (StatusCode, String)> {
    let (id, feed_id) = subscription::create_subscription(&state.db, auth.user_id, payload)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 使用任务队列自动拉取文章
    let mut storage = state.sync_queue.clone();
    let _ = storage
        .push(SyncFeedJob {
            feed_id,
            initiator_user_id: Some(auth.user_id),
        })
        .await;

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

    // 将同步任务加入任务队列
    let mut storage = state.sync_queue.clone();
    storage
        .push(SyncFeedJob {
            feed_id,
            initiator_user_id: Some(auth.user_id),
        })
        .await
        .map_err(|e| {
            tracing::error!("Failed to queue sync job: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to queue job: {}", e),
            )
        })?;

    Ok(StatusCode::OK)
}

async fn sync_all_subscriptions(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<StatusCode, (StatusCode, String)> {
    let feed_ids = subscription::list_user_feed_ids(&state.db, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 批量加入队列
    let mut storage = state.sync_queue.clone();
    for feed_id in feed_ids {
        let _ = storage
            .push(SyncFeedJob {
                feed_id,
                initiator_user_id: Some(auth.user_id),
            })
            .await;
    }

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
async fn export_opml(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<impl axum::response::IntoResponse, (StatusCode, String)> {
    let subscriptions = subscription::list_subscriptions(&state.db, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut opml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head>
    <title>TranRSS Export</title>
  </head>
  <body>
"#,
    );

    // Group by category
    let mut categories: std::collections::HashMap<String, Vec<SubscriptionDetail>> =
        std::collections::HashMap::new();
    for sub in subscriptions {
        categories
            .entry(sub.category.clone())
            .or_default()
            .push(sub);
    }

    for (cat_name, subs) in categories {
        if cat_name == "未分类" || cat_name.is_empty() {
            for sub in subs {
                opml.push_str(&format!(
                    r#"    <outline text="{}" title="{}" type="rss" xmlUrl="{}" htmlUrl="{}"/>
"#,
                    escape_xml(&sub.title),
                    escape_xml(&sub.title),
                    escape_xml(&sub.url),
                    escape_xml(&sub.site_url.unwrap_or_default())
                ));
            }
        } else {
            opml.push_str(&format!(
                r#"    <outline text="{}" title="{}">
"#,
                escape_xml(&cat_name),
                escape_xml(&cat_name)
            ));
            for sub in subs {
                opml.push_str(&format!(
                    r#"      <outline text="{}" title="{}" type="rss" xmlUrl="{}" htmlUrl="{}"/>
"#,
                    escape_xml(&sub.title),
                    escape_xml(&sub.title),
                    escape_xml(&sub.url),
                    escape_xml(&sub.site_url.unwrap_or_default())
                ));
            }
            opml.push_str("    </outline>\n");
        }
    }

    opml.push_str("  </body>\n</opml>");

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/xml"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"subscriptions.opml\"",
            ),
        ],
        opml,
    ))
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

async fn import_opml(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    mut multipart: axum::extract::Multipart,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut content = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        if field.name() == Some("file") {
            let data = field
                .bytes()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            content = String::from_utf8_lossy(&data).to_string();
            break;
        }
    }

    if content.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No file uploaded".to_string()));
    }

    // Very basic OPML parsing using regex for simplicity as we don't have a heavy XML parser yet
    // In a real app, use quick-xml or similar
    let re_outline = regex::Regex::new(r#"<outline[^>]+xmlUrl=["']([^"']+)["'][^>]*>"#).unwrap();
    let re_title = regex::Regex::new(r#"title=["']([^"']+)["']"#).unwrap();
    // For handling folders: this is tricky with regex. 
    // Let's just import all as top-level for now or try to match category if present
    
    let mut count = 0;
    for cap in re_outline.captures_iter(&content) {
        let url = &cap[1];
        let title = re_title.captures(&cap[0]).map(|c| c[1].to_string());
        
        let payload = CreateSubscriptionRequest {
            feed_url: url.to_string(),
            folder_id: None,
            category: None,
            custom_title: title,
            need_translate: Some(false),
            need_summary: Some(false),
            site_url: None,
            description: None,
            icon_url: None,
            icon_base64: None,
            target_language: None,
            num: Some(200),
            refresh_interval: Some(30),
        };

        if let Ok((_sub_id, feed_id)) = subscription::create_subscription(&state.db, auth.user_id, payload).await {
            // Queue sync
            let mut storage = state.sync_queue.clone();
            let _ = storage.push(SyncFeedJob {
                feed_id,
                initiator_user_id: Some(auth.user_id),
            }).await;
            count += 1;
        }
    }

    if count == 0 {
        return Err((StatusCode::BAD_REQUEST, "No subscriptions found in OPML".to_string()));
    }

    Ok(StatusCode::OK)
}
