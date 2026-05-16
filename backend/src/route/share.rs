use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use rss::{ChannelBuilder, ItemBuilder, GuidBuilder};
use std::sync::Arc;
use crate::AppState;
use crate::services::articles;

/// 获取共享的 RSS 订阅源（带翻译内容）
#[utoipa::path(
    get,
    path = "/share/{feed_title}",
    params(
        ("feed_title" = String, Path, description = "Feed title or ID")
    ),
    responses(
        (status = 200, description = "Success (Returns RSS XML)", body = String),
        (status = 404, description = "Feed not found")
    ),
    tag = "Share"
)]
pub async fn share_feed(
    State(state): State<Arc<AppState>>,
    Path(feed_title): Path<String>,
) -> impl IntoResponse {
    // 1. 查找 Feed 信息以及一个代表性的用户（用于获取翻译内容）
    // 优先尝试按标题精确匹配，如果失败则尝试作为 ID 匹配
    let feed_info: Option<(i64, i64, String, Option<String>)> = sqlx::query_as(
        r#"
        SELECT f.id, s.user_id, f.title, f.site_url
        FROM feeds f
        JOIN subscriptions s ON s.feed_id = f.id
        WHERE (f.title = ? OR CAST(f.id AS TEXT) = ?) AND s.is_shared = 1
        LIMIT 1
        "#
    )
    .bind(&feed_title)
    .bind(&feed_title)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let (feed_id, user_id, title, site_url) = match feed_info {
        Some(info) => info,
        None => return (StatusCode::NOT_FOUND, "Feed not found").into_response(),
    };

    // 2. 获取该用户的文章列表（包含翻译后的标题）
    let articles_list = articles::list_articles(&state.db, user_id, Some(feed_id), None, None)
        .await
        .unwrap_or_default();

    let mut items = Vec::new();

    for art_item in articles_list {
        // 获取详情以拼合正文
        if let Ok(detail) = articles::get_article_detail(&state.db, user_id, art_item.id).await {
            let blocks = articles::get_article_blocks(&state.db, user_id, art_item.id)
                .await
                .unwrap_or_default();
            
            let content = articles::stitch_article_content(
                detail.content_skeleton.as_deref().unwrap_or_default(),
                &blocks,
                detail.summary.as_deref(),
                detail.need_translate,
            );

            let item = ItemBuilder::default()
                .title(Some(detail.title))
                .link(detail.link)
                .description(Some(content))
                .pub_date(detail.published_at.and_then(|ts| chrono::DateTime::from_timestamp(ts, 0)).map(|dt| dt.to_rfc2822()))
                .guid(Some(GuidBuilder::default().value(art_item.id.to_string()).build()))
                .build();
            items.push(item);
        }
    }

    let channel = ChannelBuilder::default()
        .title(format!("TranRSS: {}", title))
        .link(site_url.unwrap_or_default())
        .description(format!("Translated feed for {} (provided by TranRSS)", title))
        .items(items)
        .build();

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        channel.to_string(),
    )
    .into_response()
}
