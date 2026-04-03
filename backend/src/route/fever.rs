use crate::AppState;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(fever_handler).get(fever_handler))
}

#[derive(Deserialize, Default)]
struct FeverQuery {
    #[serde(default)]
    _api: Option<String>,
    #[serde(default)]
    groups: Option<String>,
    #[serde(default)]
    feeds: Option<String>,
    #[serde(default)]
    items: Option<String>,
    #[serde(default)]
    favicons: Option<String>,
    #[serde(default)]
    unread_item_ids: Option<String>,
    #[serde(default)]
    saved_item_ids: Option<String>,
    #[serde(default)]
    mark: Option<String>,
    #[serde(default)]
    as_: Option<String>,
    #[serde(alias = "as")]
    as_real: Option<String>, // because `as` is a keyword
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    before: Option<i64>,
    #[serde(default)]
    since_id: Option<i64>,
    #[serde(default)]
    max_id: Option<i64>,
    #[serde(default)]
    with_ids: Option<String>,
}

#[derive(Deserialize, Default)]
struct FeverBody {
    #[serde(default)]
    api_key: String,
}

#[derive(Serialize)]
struct FeverBaseResponse {
    api_version: i32,
    auth: i32,
    last_refreshed_on_time: i64,
}

#[derive(Serialize)]
struct FeverGroupsResponse {
    #[serde(flatten)]
    base: FeverBaseResponse,
    groups: Vec<FeverGroup>,
    feeds_groups: Vec<FeverFeedGroup>,
}

#[derive(Serialize)]
struct FeverGroup {
    id: i64,
    title: String,
}

#[derive(Serialize)]
struct FeverFeedGroup {
    group_id: i64,
    feed_ids: String, // Comma-separated feed IDs
}

#[derive(Serialize)]
struct FeverFeedsResponse {
    #[serde(flatten)]
    base: FeverBaseResponse,
    feeds: Vec<FeverFeed>,
    feeds_groups: Vec<FeverFeedGroup>,
}

#[derive(Serialize)]
struct FeverFeed {
    id: i64,
    favicon_id: i64,
    title: String,
    url: String,
    site_url: String,
    is_spark: i32,
    last_updated_on_time: i64,
}

#[derive(Serialize)]
struct FeverFaviconsResponse {
    #[serde(flatten)]
    base: FeverBaseResponse,
    favicons: Vec<FeverFavicon>,
}

#[derive(Serialize)]
struct FeverFavicon {
    id: i64,
    data: String,
}

#[derive(Serialize)]
struct FeverItemIdsResponse {
    #[serde(flatten)]
    base: FeverBaseResponse,
    unread_item_ids: Option<String>,
    saved_item_ids: Option<String>,
}

#[derive(Serialize)]
struct FeverItemsResponse {
    #[serde(flatten)]
    base: FeverBaseResponse,
    items: Vec<FeverItem>,
    total_items: i64,
}

#[derive(Serialize)]
struct FeverItem {
    id: i64,
    feed_id: i64,
    title: String,
    author: String,
    html: String,
    url: String,
    is_saved: i32,
    is_read: i32,
    created_on_time: i64,
}

#[axum::debug_handler]
async fn fever_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FeverQuery>,
    body: String,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let api_key = if let Ok(form) = serde_urlencoded::from_str::<FeverBody>(&body) {
        form.api_key
    } else {
        "".to_string()
    };

    let user_id = match authenticate(&state.db, &api_key).await {
        Ok(id) => id,
        Err(_) => {
            return Ok(Json(serde_json::json!({
                "api_version": 3,
                "auth": 0,
            })).into_response());
        }
    };

    // Check if Fever API is enabled in user settings
    let is_enabled: bool = sqlx::query_scalar("SELECT fever_api FROM user_setting WHERE user_id = ?")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(Some(false))
        .unwrap_or(false);

    if !is_enabled {
        return Ok(Json(serde_json::json!({
            "api_version": 3,
            "auth": 0,
        })).into_response());
    }

    let now_ts = chrono::Utc::now().timestamp();
    let base = FeverBaseResponse {
        api_version: 3,
        auth: 1,
        last_refreshed_on_time: now_ts,
    };

    // groups
    if query.groups.is_some() {
        let (groups, feeds_groups) = get_groups_and_feeds(&state.db, user_id).await?;
        return Ok(Json(FeverGroupsResponse {
            base,
            groups,
            feeds_groups,
        }).into_response());
    }

    // feeds
    if query.feeds.is_some() {
        let feeds = get_feeds(&state.db, user_id).await?;
        let (_, feeds_groups) = get_groups_and_feeds(&state.db, user_id).await?;
        return Ok(Json(FeverFeedsResponse {
            base,
            feeds,
            feeds_groups,
        }).into_response());
    }

    // favicons
    if query.favicons.is_some() {
        let favicons = get_favicons(&state.db, user_id).await?;
        return Ok(Json(FeverFaviconsResponse {
            base,
            favicons,
        }).into_response());
    }

    // unread_item_ids
    if query.unread_item_ids.is_some() {
        let unread = get_item_ids(&state.db, user_id, false, false).await?;
        return Ok(Json(FeverItemIdsResponse {
            base,
            unread_item_ids: Some(unread),
            saved_item_ids: None,
        }).into_response());
    }

    // saved_item_ids
    if query.saved_item_ids.is_some() {
        let saved = get_item_ids(&state.db, user_id, true, false).await?;
        return Ok(Json(FeverItemIdsResponse {
            base,
            unread_item_ids: None,
            saved_item_ids: Some(saved),
        }).into_response());
    }

    // items
    if query.items.is_some() {
        let (items, total_items) = get_items(&state, user_id, &query).await?;
        return Ok(Json(FeverItemsResponse {
            base,
            items,
            total_items,
        }).into_response());
    }

    // mark
    if query.mark.is_some() {
        handle_mark(&state.db, user_id, &query).await?;
        // return just base
        return Ok(Json(base).into_response());
    }

    // fallback: just auth check
    Ok(Json(base).into_response())
}

async fn authenticate(db: &sqlx::SqlitePool, api_key: &str) -> std::result::Result<i64, ()> {
    if api_key.is_empty() {
        return Err(());
    }
    let user_id: Option<i64> = sqlx::query_scalar("SELECT id FROM users WHERE fever_api_key = ?")
        .bind(api_key.to_lowercase())
        .fetch_optional(db)
        .await
        .unwrap_or(None);
    user_id.ok_or(())
}

async fn get_groups_and_feeds(db: &sqlx::SqlitePool, user_id: i64) -> Result<(Vec<FeverGroup>, Vec<FeverFeedGroup>), (StatusCode, String)> {
    let folders: Vec<(i64, String)> = sqlx::query_as("SELECT id, title FROM folders WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(db)
        .await
        .unwrap_or_default();

    let mut groups = Vec::new();
    let mut fg_map = std::collections::HashMap::new();

    for (fid, title) in folders {
        groups.push(FeverGroup { id: fid, title });
        fg_map.insert(fid, Vec::new());
    }

    let subs: Vec<(i64, Option<i64>)> = sqlx::query_as("SELECT feed_id, folder_id FROM subscriptions WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(db)
        .await
        .unwrap_or_default();

    for (feed_id, folder_id_opt) in subs {
        if let Some(folder_id) = folder_id_opt {
            if let Some(list) = fg_map.get_mut(&folder_id) {
                list.push(feed_id.to_string());
            }
        }
    }

    let mut feeds_groups = Vec::new();
    for (group_id, feed_ids) in fg_map {
        feeds_groups.push(FeverFeedGroup {
            group_id,
            feed_ids: feed_ids.join(","),
        });
    }

    Ok((groups, feeds_groups))
}

async fn get_feeds(db: &sqlx::SqlitePool, user_id: i64) -> Result<Vec<FeverFeed>, (StatusCode, String)> {
    let rows: Vec<(i64, String, Option<String>, String, Option<i64>)> = sqlx::query_as(
        r#"
        SELECT s.feed_id, f.feed_url, f.site_url, COALESCE(s.custom_title, f.title), f.last_fetched_at
        FROM subscriptions s
        JOIN feeds f ON s.feed_id = f.id
        WHERE s.user_id = ?
        "#
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let feeds = rows.into_iter().map(|(id, url, site_url, title, updated)| {
        FeverFeed {
            id,
            favicon_id: id,
            title,
            url,
            site_url: site_url.unwrap_or_default(),
            is_spark: 0,
            last_updated_on_time: updated.unwrap_or(0),
        }
    }).collect();

    Ok(feeds)
}

async fn get_favicons(db: &sqlx::SqlitePool, user_id: i64) -> Result<Vec<FeverFavicon>, (StatusCode, String)> {
    let rows: Vec<(i64, Option<String>)> = sqlx::query_as(
        r#"
        SELECT s.feed_id, f.icon_base64
        FROM subscriptions s
        JOIN feeds f ON s.feed_id = f.id
        WHERE s.user_id = ?
        "#
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    let mut favicons = Vec::new();
    for (id, icon_opt) in rows {
        // Fever expects mime type logic. Usually base64 encoded png/jpg.
        // E.g. "image/png;base64,...". `f.icon_base64` typically doesn't contain the mime part if it's just raw base64.
        // Actually, some clients expect the full data URI.
        // So we format it if it isn't empty.
        let mut data = "image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7".to_string(); // 1x1 transparent
        if let Some(b64) = icon_opt {
            if b64.starts_with("data:image") {
                if b64.find(";").is_some() {
                    data = b64.clone().replacen("data:", "", 1);
                }
            } else if !b64.is_empty() {
                data = format!("image/png;base64,{}", b64);
            }
        }
        favicons.push(FeverFavicon { id, data });
    }

    Ok(favicons)
}

async fn get_item_ids(db: &sqlx::SqlitePool, user_id: i64, starred_only: bool, unread_only: bool) -> Result<String, (StatusCode, String)> {
    let mut query = String::from("SELECT id FROM articles WHERE feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)");
    if starred_only {
        query.push_str(" AND is_starred = 1");
    }
    if unread_only {
        query.push_str(" AND is_read = 0");
    }
    query.push_str(" ORDER BY id DESC"); // Fever expects ids or something, but order matters sometimes

    let ids: Vec<(i64,)> = sqlx::query_as(&query)
        .bind(user_id)
        .fetch_all(db)
        .await
        .unwrap_or_default();

    let id_strings: Vec<String> = ids.into_iter().map(|(id,)| id.to_string()).collect();
    Ok(id_strings.join(","))
}

async fn get_items(state: &Arc<AppState>, user_id: i64, query: &FeverQuery) -> Result<(Vec<FeverItem>, i64), (StatusCode, String)> {
    let mut q_str = String::from(
        r#"
        SELECT a.id, a.feed_id, a.title, a.author, a.link, a.is_saved, a.is_read, a.published_at, a.crawl_time, a.content_skeleton, a.summary, s.need_translate
        FROM articles a
        JOIN subscriptions s ON a.feed_id = s.feed_id
        WHERE s.user_id = ? 
        "#
    );

    // Filters
    if let Some(since_id) = query.since_id {
        q_str.push_str(&format!(" AND a.id > {}", since_id));
    }
    if let Some(max_id) = query.max_id {
        q_str.push_str(&format!(" AND a.id < {}", max_id));
    }
    if let Some(with_ids) = &query.with_ids {
        let valid_ids: Vec<i64> = with_ids.split(',').filter_map(|s| s.parse().ok()).collect();
        if !valid_ids.is_empty() {
            let id_list = valid_ids.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",");
            q_str.push_str(&format!(" AND a.id IN ({})", id_list));
        }
    }

    q_str.push_str(" ORDER BY a.id DESC LIMIT 50"); // Fetch limit

    let rows: Vec<(i64, i64, String, Option<String>, Option<String>, bool, bool, Option<i64>, Option<i64>, Option<String>, Option<String>, bool)> = 
        sqlx::query_as(&q_str)
        .bind(user_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    // extract IDs for fetching blocks
    let item_ids: Vec<String> = rows.iter().map(|r| r.0.to_string()).collect();
    
    // total count
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM articles WHERE feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
        .bind(user_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

    if item_ids.is_empty() {
        return Ok((vec![], total.0));
    }

    let id_list_str = item_ids.join(",");
    let p_blocks = format!("SELECT article_id, block_index, raw_text, trans_text FROM article_blocks WHERE user_id = ? AND article_id IN ({})", id_list_str);
    let blocks: Vec<crate::model::articles::ArticleBlock> = sqlx::query_as(&p_blocks)
        .bind(user_id)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    use std::collections::HashMap;
    let mut block_map: HashMap<i64, Vec<crate::model::articles::ArticleBlock>> = HashMap::new();
    for b in blocks {
        block_map.entry(b.article_id).or_default().push(b);
    }

    let mut items = Vec::new();
    for (id, feed_id, mut title, author, link, is_saved, is_read, published, crawl_time, skeleton, summary, need_translate) in rows {
        let ts = published.or(crawl_time).unwrap_or(0);

        if let Some(b_list) = block_map.get(&id) {
            if let Some(t_block) = b_list.iter().find(|b| b.block_index == -1) {
                if let Some(t_title) = &t_block.trans_text {
                    title = t_title.clone();
                }
            }
        }

        let summary_ref = summary.as_deref().filter(|s| !s.trim().is_empty());
        let html = crate::services::articles::stitch_article_content(
            skeleton.as_deref().unwrap_or_default(), 
            block_map.get(&id).map(|v| v.as_slice()).unwrap_or(&[]), 
            summary_ref, 
            need_translate
        );

        items.push(FeverItem {
            id,
            feed_id,
            title,
            author: author.unwrap_or_default(),
            html,
            url: link.unwrap_or_default(),
            is_saved: if is_saved { 1 } else { 0 },
            is_read: if is_read { 1 } else { 0 },
            created_on_time: ts,
        });
    }

    Ok((items, total.0))
}

async fn handle_mark(db: &sqlx::SqlitePool, user_id: i64, query: &FeverQuery) -> Result<(), (StatusCode, String)> {
    let mark_type = query.mark.as_deref().unwrap_or("");
    let as_status = query.as_real.as_deref().or(query.as_.as_deref()).unwrap_or("");
    let item_id = query.id.unwrap_or(0);
    let before = query.before.unwrap_or(0);

    if item_id == 0 {
        return Ok(());
    }

    match mark_type {
        "item" => {
            match as_status {
                "read" => {
                    sqlx::query("UPDATE articles SET is_read = 1 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                        .bind(item_id).bind(user_id).execute(db).await.ok();
                }
                "unread" => {
                    sqlx::query("UPDATE articles SET is_read = 0 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                        .bind(item_id).bind(user_id).execute(db).await.ok();
                }
                "saved" => {
                    sqlx::query("UPDATE articles SET is_starred = 1 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                        .bind(item_id).bind(user_id).execute(db).await.ok();
                }
                "unsaved" => {
                    sqlx::query("UPDATE articles SET is_starred = 0 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                        .bind(item_id).bind(user_id).execute(db).await.ok();
                }
                _ => {}
            }
        }
        "feed" => {
            if as_status == "read" {
                let mut q_str = "UPDATE articles SET is_read = 1 WHERE feed_id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)".to_string();
                if before > 0 {
                    q_str.push_str(&format!(" AND (published_at < {} OR (published_at IS NULL AND crawl_time < {}))", before, before));
                }
                sqlx::query(&q_str).bind(item_id).bind(user_id).execute(db).await.ok();
            }
        }
        "group" => {
            if as_status == "read" {
                // item_id is group_id (= folder_id). If 0, it means all items
                let mut q_str = if item_id == 0 {
                    "UPDATE articles SET is_read = 1 WHERE feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)".to_string()
                } else {
                    "UPDATE articles SET is_read = 1 WHERE feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ? AND folder_id = ?)".to_string()
                };

                if before > 0 {
                    q_str.push_str(&format!(" AND (published_at < {} OR (published_at IS NULL AND crawl_time < {}))", before, before));
                }

                let mut q = sqlx::query(&q_str).bind(user_id);
                if item_id != 0 {
                    q = q.bind(item_id);
                }
                q.execute(db).await.ok();
            }
        }
        _ => {}
    }

    Ok(())
}
