use crate::AppState;
use crate::model::user::User;
use crate::services::auth::{self, AuthUser};
use axum::http::header::CONTENT_TYPE;
use axum::{
    Form, Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// --- GReader 协议常量 ---
const STATE_READ: &str = "user/-/state/com.google/read";
const STATE_STARRED: &str = "user/-/state/com.google/starred";
const STATE_READING_LIST: &str = "user/-/state/com.google/reading-list";
const STATE_KEPT_UNREAD: &str = "user/-/state/com.google/kept-unread";
const STATE_FRESH: &str = "user/-/state/com.google/fresh";

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Auth
        .route("/accounts/ClientLogin", post(client_login))
        .route("/reader/api/0/token", get(get_token).post(get_token))
        // Info
        .route("/reader/api/0/user-info", get(user_info))
        // Subscription & Tags
        .route("/reader/api/0/subscription/list", get(subscription_list))
        .route("/reader/api/0/tag/list", get(tag_list))
        // Content
        .route("/reader/api/0/stream/contents/{*id}", get(stream_contents))
        .route("/reader/api/0/stream/items/ids", get(stream_items_ids))
        .route(
            "/reader/api/0/stream/items/contents",
            get(stream_items_contents).post(stream_items_contents),
        )
        // Actions
        .route("/reader/api/0/edit-tag", post(edit_tag))
        .route("/reader/api/0/mark-all-as-read", post(mark_all_as_read))
        .route("/reader/api/0/disable-tag", post(disable_tag))
        .route("/reader/api/0/rename-tag", post(rename_tag))
        // Subscription actions
        .route(
            "/reader/api/0/subscription/quickadd",
            post(subscription_quickadd),
        )
        .route("/reader/api/0/subscription/edit", post(subscription_edit))
}

// --- 辅助函数 ---

/// 将整数 ID 转换为 GReader 标准的 16 位十六进制格式
/// 格式: "tag:google.com,2005:reader/item/000000000000abcd"
fn item_id_to_greader(id: i64) -> String {
    format!("tag:google.com,2005:reader/item/{:016x}", id as u64)
}

/// 从 GReader item ID 解析回整数 ID
fn greader_to_item_id(greader_id: &str) -> Option<i64> {
    // 支持三种格式:
    // 1. "tag:google.com,2005:reader/item/000000000000abcd" (标准 GReader 格式)
    // 2. "000000000000abcd" (16位纯十六进制字符串，CapyReader 使用的格式)
    // 3. 纯数字十进制字符串（向后兼容某些老客户端）
    if let Some(hex) = greader_id.strip_prefix("tag:google.com,2005:reader/item/") {
        u64::from_str_radix(hex, 16).map(|i| i as i64).ok()
    } else if greader_id.len() == 16 && greader_id.chars().all(|c| c.is_ascii_hexdigit()) {
        u64::from_str_radix(greader_id, 16).map(|i| i as i64).ok()
    } else {
        greader_id.parse::<i64>().ok()
    }
}

// --- Auth Handlers ---

#[derive(Deserialize)]
struct GReaderLoginRequest {
    #[serde(alias = "email")]
    #[serde(rename = "Email")]
    email: String,
    #[serde(alias = "passwd")]
    #[serde(rename = "Passwd")]
    passwd: String,
}

async fn client_login(
    State(state): State<Arc<AppState>>,
    Form(payload): Form<GReaderLoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user: User =
        sqlx::query_as("SELECT id, username, password_hash FROM users WHERE username = ?")
            .bind(&payload.email)
            .fetch_one(&state.db)
            .await
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    "Error=BadAuthentication".to_string(),
                )
            })?;

    if !auth::verify_password(&payload.passwd, &user.password_hash) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Error=BadAuthentication".to_string(),
        ));
    }

    let token = auth::create_token(user.id, &user.username)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let body = format!("SID=ignored\nLSID=ignored\nAuth={}", token);
    
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        body,
    ))
}

async fn get_token(_auth: AuthUser) -> impl IntoResponse {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain")],
        "antigravity_token",
    )
}

// --- User Info ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GReaderUserInfo {
    user_id: String,
    user_name: String,
    user_email: String,
    is_blogger_user: bool,
    signup_time_sec: i64,
    is_multi_login_enabled: bool,
}

async fn user_info(auth: AuthUser) -> Json<GReaderUserInfo> {
    Json(GReaderUserInfo {
        user_id: auth.user_id.to_string(),
        user_name: auth.username.clone(),
        user_email: format!("{}@localhost", auth.username),
        is_blogger_user: false,
        signup_time_sec: 0,
        is_multi_login_enabled: false,
    })
}

// --- Subscription & Tags ---

#[derive(Serialize)]
struct GReaderSubscriptionList {
    subscriptions: Vec<GReaderSubscription>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GReaderSubscription {
    id: String,
    title: String,
    categories: Vec<GReaderCategory>,
    sortid: String,
    firstitemmsec: String,
    url: String,
    #[serde(rename = "htmlUrl")]
    html_url: String,
    #[serde(rename = "iconUrl")]
    icon_url: String,
}

#[derive(Serialize)]
struct GReaderCategory {
    id: String,
    label: String,
}

async fn subscription_list(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<GReaderSubscriptionList>, (StatusCode, String)> {
    // 自动刷新逻辑：GReader 客户端访问时触发过期 Feed 同步
    let state_thread = (*state).clone();
    let uid = auth.user_id;
    tokio::spawn(async move {
        let _ = crate::services::subscription::trigger_stale_syncs(uid, state_thread).await;
    });

    // feed_id, feed_url, title, site_url, icon_url, folder_title
    let rows: Vec<(i64, String, String, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        r#"
        SELECT s.feed_id, f.feed_url, COALESCE(s.custom_title, f.title), f.site_url, f.icon_url, fo.title as folder_title
        FROM subscriptions s
        JOIN feeds f ON s.feed_id = f.id
        LEFT JOIN folders fo ON s.folder_id = fo.id
        WHERE s.user_id = ?
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let subscriptions = rows
        .into_iter()
        .map(
            |(feed_id, feed_url, title, site_url, icon_url, folder_title)| {
                let mut categories = Vec::new();
                if let Some(ft) = folder_title {
                    categories.push(GReaderCategory {
                        id: format!("user/{}/label/{}", auth.user_id, ft),
                        label: ft,
                    });
                }
                let html_url = site_url.clone().unwrap_or_default();
                let icon = icon_url.unwrap_or_default();
                GReaderSubscription {
                    id: format!("feed/{}", feed_id),
                    title,
                    categories,
                    sortid: format!("{:08x}", feed_id),
                    firstitemmsec: "0".to_string(),
                    url: feed_url,
                    html_url,
                    icon_url: icon,
                }
            },
        )
        .collect();

    Ok(Json(GReaderSubscriptionList { subscriptions }))
}

#[derive(Serialize)]
struct GReaderTagList {
    tags: Vec<GReaderTag>,
}

#[derive(Serialize)]
struct GReaderTag {
    id: String,
    sortid: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    tag_type: Option<String>,
}

async fn tag_list(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<GReaderTagList>, (StatusCode, String)> {
    let folders: Vec<(String,)> = sqlx::query_as("SELECT title FROM folders WHERE user_id = ?")
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut tags = vec![
        GReaderTag {
            id: format!("user/{}/state/com.google/starred", auth.user_id),
            sortid: "00000001".into(),
            tag_type: Some("tag".into()),
        },
        GReaderTag {
            id: format!("user/{}/state/com.google/read", auth.user_id),
            sortid: "00000002".into(),
            tag_type: None,
        },
    ];

    for (i, (title,)) in folders.into_iter().enumerate() {
        tags.push(GReaderTag {
            id: format!("user/{}/label/{}", auth.user_id, title),
            sortid: format!("{:08x}", i + 10),
            tag_type: Some("folder".into()),
        });
    }

    Ok(Json(GReaderTagList { tags }))
}

// --- Content Handlers ---

#[derive(Deserialize)]
struct StreamQuery {
    s: Option<String>,  // Stream ID
    n: Option<i64>,     // Number of items
    xt: Option<String>, // Exclude target
    it: Option<String>, // Include target
    r: Option<String>,  // Order: "o" for oldest first
    ot: Option<i64>,    // Newer than timestamp
    nt: Option<i64>,    // Older than timestamp
    c: Option<String>,  // Continuation token
    #[allow(dead_code)]
    output: Option<String>,
}

#[derive(Serialize)]
struct GReaderStreamContents {
    id: String,
    title: String,
    direction: String,
    #[serde(rename = "self")]
    self_link: Vec<GReaderLink>,
    updated: i64,
    items: Vec<GReaderItem>,
    continuation: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GReaderItem {
    id: String,
    crawl_time_msec: String,
    timestamp_usec: String,
    published: i64,
    updated: i64,
    title: String,
    canonical: Vec<GReaderLink>,
    alternate: Vec<GReaderLink>,
    summary: GReaderContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<GReaderContent>,
    author: String,
    categories: Vec<String>,
    origin: GReaderOrigin,
}

#[derive(Serialize)]
struct GReaderLink {
    href: String,
}

#[derive(Serialize)]
struct GReaderContent {
    direction: String,
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GReaderOrigin {
    stream_id: String,
    title: String,
    #[serde(rename = "htmlUrl")]
    html_url: String,
}

/// 从 continuation 令牌（base64 编码的偏移量）解析 offset
fn parse_continuation(c: &str) -> i64 {
    use std::str;
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, c)
        .unwrap_or_default();
    str::from_utf8(&decoded)
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}

/// 将 offset 编码为 continuation 令牌
fn encode_continuation(offset: i64) -> String {
    base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        offset.to_string().as_bytes(),
    )
}

async fn stream_contents(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(stream_id): Path<String>,
    Query(params): Query<StreamQuery>,
) -> Result<Json<GReaderStreamContents>, (StatusCode, String)> {
    // 自动刷新逻辑：拉取内容时触发过期 Feed 同步
    let state_thread = (*state).clone();
    let uid = auth.user_id;
    tokio::spawn(async move {
        let _ = crate::services::subscription::trigger_stale_syncs(uid, state_thread).await;
    });

    let limit = params.n.unwrap_or(20).min(1000);
    let offset = params.c.as_deref().map(parse_continuation).unwrap_or(0);

    // 构建基础查询
    // (id, title, link, author, published_at, crawl_time, is_read, is_starred, feed_id, feed_title, site_url, content_skeleton, summary, updated_at, need_translate)
    let mut query_str = String::from(
        r#"
        SELECT 
            a.id, a.title, a.link, a.author, a.published_at, a.crawl_time,
            a.is_read, a.is_starred, a.feed_id, f.title as feed_title, f.site_url,
            COALESCE(a.content_skeleton, '') as skeleton, COALESCE(a.summary, '') as summary,
            a.updated_at, s.need_translate
        FROM articles a
        JOIN feeds f ON a.feed_id = f.id
        JOIN subscriptions s ON s.feed_id = f.id
        WHERE s.user_id = ?
        "#,
    );

    apply_stream_filters(&mut query_str, &stream_id, &params, auth.user_id);

    // 排序
    if params.r.as_deref() == Some("o") {
        query_str.push_str(" ORDER BY a.published_at ASC");
    } else {
        query_str.push_str(" ORDER BY a.published_at DESC");
    }

    // 分页: 多取 1 条用于判断是否有下一页
    query_str.push_str(&format!(" LIMIT {} OFFSET {}", limit + 1, offset));

    let rows: Vec<(
        i64,
        String,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<i64>,
        bool,
        bool,
        i64,
        String,
        Option<String>,
        String,
        String,
        String,
        bool,
    )> = sqlx::query_as(&query_str)
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let has_more = rows.len() as i64 > limit;
    let mut rows = rows;
    if has_more {
        rows.truncate(limit as usize);
    }

    if rows.is_empty() {
        return Ok(Json(GReaderStreamContents {
            id: stream_id.clone(),
            title: resolve_stream_title(&stream_id),
            direction: "ltr".to_string(),
            self_link: vec![GReaderLink {
                href: format!("/reader/api/0/stream/contents/{}", stream_id),
            }],
            updated: chrono::Utc::now().timestamp(),
            items: vec![],
            continuation: Some("".to_string()),
        }));
    }

    // 提取 ID 以便批量拉取 Blocks
    let item_ids: Vec<i64> = rows.iter().map(|r| r.0).collect();
    let placeholders = item_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 2))
        .collect::<Vec<_>>()
        .join(",");

    let block_query = format!(
        "SELECT article_id, block_index, raw_text, trans_text FROM article_blocks WHERE user_id = ?1 AND article_id IN ({}) ORDER BY article_id, block_index ASC",
        placeholders
    );
    let mut bq =
        sqlx::query_as::<_, crate::model::articles::ArticleBlock>(&block_query).bind(auth.user_id);
    for id in &item_ids {
        bq = bq.bind(id);
    }
    let all_blocks = bq
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    use std::collections::HashMap;
    let mut blocks_map: HashMap<i64, Vec<crate::model::articles::ArticleBlock>> = HashMap::new();
    for block in all_blocks {
        blocks_map.entry(block.article_id).or_default().push(block);
    }

    let items: Vec<GReaderItem> = rows
        .into_iter()
        .map(
            |(
                id,
                mut title,
                link,
                author,
                pub_at,
                crawl_time,
                is_read,
                is_starred,
                feed_id,
                feed_title,
                site_url,
                skeleton,
                summary,
                _updated_at,
                need_translate,
            )| {
                let ct = crawl_time.unwrap_or(0);
                let ts_pub = pub_at.unwrap_or(ct);
                let ts_sync = ct;

                // 尝试翻译标题 (index = -1)
                if let Some(article_blocks) = blocks_map.get(&id) {
                    if let Some(block) = article_blocks.iter().find(|b| b.block_index == -1) {
                        if let Some(ref trans_title) = block.trans_text {
                            title = trans_title.clone();
                        }
                    }
                }

                let content_html = crate::services::articles::stitch_article_content(
                    &skeleton,
                    blocks_map.get(&id).map(|v| v.as_slice()).unwrap_or(&[]),
                    if summary.trim().is_empty() {
                        None
                    } else {
                        Some(&summary)
                    },
                    need_translate,
                );

                let mut categories = vec![STATE_READING_LIST.to_string()];
                if is_read {
                    categories.push(STATE_READ.to_string());
                } else {
                    categories.push(STATE_KEPT_UNREAD.to_string());
                    if ct > chrono::Utc::now().timestamp() - 86400 {
                        categories.push(STATE_FRESH.to_string());
                    }
                }
                if is_starred {
                    categories.push(STATE_STARRED.to_string());
                }

                let link_str = link.clone().unwrap_or_default();

                GReaderItem {
                    id: item_id_to_greader(id),
                    crawl_time_msec: (ct * 1000).to_string(),
                    timestamp_usec: (ts_sync * 1_000_000).to_string(),
                    published: ts_pub,
                    updated: ts_pub,
                    title,
                    canonical: vec![GReaderLink {
                        href: link_str.clone(),
                    }],
                    alternate: vec![GReaderLink { href: link_str }],
                    summary: GReaderContent {
                        direction: "ltr".into(),
                        content: content_html.clone(),
                    },
                    content: Some(GReaderContent {
                        direction: "ltr".into(),
                        content: content_html,
                    }),
                    author: author.clone().unwrap_or_default(),
                    categories,
                    origin: GReaderOrigin {
                        stream_id: format!("feed/{}", feed_id),
                        title: feed_title.clone(),
                        html_url: site_url.clone().unwrap_or_default(),
                    },
                }
            },
        )
        .collect();

    let continuation = if has_more {
        Some(encode_continuation(offset + limit))
    } else {
        Some("".to_string())
    };

    let stream_title = resolve_stream_title(&stream_id);

    Ok(Json(GReaderStreamContents {
        id: stream_id.clone(),
        title: stream_title,
        direction: "ltr".to_string(),
        self_link: vec![GReaderLink {
            href: format!("/reader/api/0/stream/contents/{}", stream_id),
        }],
        updated: chrono::Utc::now().timestamp(),
        items,
        continuation,
    }))
}

fn resolve_stream_title(stream_id: &str) -> String {
    if stream_id.contains("reading-list") {
        "All Articles".to_string()
    } else if stream_id.contains("starred") {
        "Starred".to_string()
    } else if stream_id.starts_with("feed/") {
        stream_id[5..].to_string()
    } else {
        stream_id.to_string()
    }
}

fn apply_stream_filters(
    query_str: &mut String,
    stream_id: &str,
    params: &StreamQuery,
    user_id: i64,
) {
    // 按 stream_id 过滤
    let s = params.s.as_deref().unwrap_or(stream_id);
    if s.starts_with("feed/") {
        if let Ok(feed_id) = s[5..].parse::<i64>() {
            query_str.push_str(&format!(" AND a.feed_id = {}", feed_id));
        }
    } else if s.contains("state/com.google/starred") {
        query_str.push_str(" AND a.is_starred = 1");
    } else if s.contains("/label/") {
        if let Some(label) = s.split("/label/").last() {
            query_str.push_str(&format!(" AND f.id IN (SELECT feed_id FROM subscriptions s2 JOIN folders fo ON s2.folder_id = fo.id WHERE s2.user_id = {} AND fo.title = '{}')", user_id, label.replace("'", "''")));
        }
    }
    // user/-/state/com.google/reading-list → 不过滤（全部）

    // 排除已读
    if let Some(ref xt) = params.xt {
        if xt.contains("state/com.google/read") {
            query_str.push_str(" AND a.is_read = 0");
        }
    }

    // 包含过滤
    if let Some(ref it) = params.it {
        if it.contains("state/com.google/read") {
            query_str.push_str(" AND a.is_read = 1");
        } else if it.contains("state/com.google/starred") {
            query_str.push_str(" AND a.is_starred = 1");
        }
    }

    // ot = oldest time：只取比此时间戳**更新**的文章
    if let Some(ot) = params.ot {
        // GReader 协议中 ot 可能是秒、毫秒或微秒，这里做自适应
        let ot_sec = if ot > 10_000_000_000_000 {
            ot / 1_000_000
        } else if ot > 10_000_000_000 {
            ot / 1000
        } else {
            ot
        };
        query_str.push_str(&format!(
            " AND a.crawl_time >= {}",
            ot_sec
        ));
    }

    // nt = newest time：只取此时间戳**之前**的文章
    if let Some(nt) = params.nt {
        let nt_sec = if nt > 10_000_000_000_000 {
            nt / 1_000_000
        } else if nt > 10_000_000_000 {
            nt / 1000
        } else {
            nt
        };
        query_str.push_str(&format!(
            " AND a.crawl_time <= {}",
            nt_sec
        ));
    }

    let _ = user_id;
}

// --- Stream Items IDs ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GReaderStreamIds {
    #[serde(rename = "itemRefs")]
    item_refs: Vec<GReaderItemRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    continuation: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GReaderItemRef {
    id: String,
    timestamp_usec: String,
    direct_stream_ids: Vec<String>,
}

async fn stream_items_ids(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(params): Query<StreamQuery>,
) -> Result<Json<GReaderStreamIds>, (StatusCode, String)> {
    let limit = params.n.unwrap_or(1000).min(10000);
    let offset = params.c.as_deref().map(parse_continuation).unwrap_or(0);

    let mut query_str = String::from(
        r#"
        SELECT a.id, a.published_at, a.feed_id, a.crawl_time
        FROM articles a
        JOIN feeds f ON a.feed_id = f.id
        JOIN subscriptions s ON s.feed_id = f.id
        WHERE s.user_id = ?
"#,
    );

    apply_stream_filters(&mut query_str, params.s.as_deref().unwrap_or(""), &params, auth.user_id);

    if params.r.as_deref() == Some("o") {
        query_str.push_str(" ORDER BY a.published_at ASC");
    } else {
        query_str.push_str(" ORDER BY a.published_at DESC");
    }

    query_str.push_str(&format!(" LIMIT {} OFFSET {}", limit + 1, offset));

    let rows: Vec<(i64, Option<i64>, i64, Option<i64>)> = sqlx::query_as(&query_str)
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let has_more = rows.len() as i64 > limit;
    let rows = if has_more {
        &rows[..limit as usize]
    } else {
        &rows[..]
    };

    let item_refs = rows
        .iter()
        .map(|(id, _pub_at, feed_id, crawl_time)| {
            let ct = crawl_time.unwrap_or(0);
            GReaderItemRef {
                id: id.to_string(), // CapyReader expects numeric strings here
                timestamp_usec: (ct * 1_000_000).to_string(),
                direct_stream_ids: vec![format!("feed/{}", feed_id)],
            }
        })
        .collect();

    let continuation = if has_more {
        Some(encode_continuation(offset + limit))
    } else {
        None
    };

    Ok(Json(GReaderStreamIds {
        item_refs,
        continuation,
    }))
}

// --- Stream Items Contents (POST) ---
// ReadYou 有时会用 POST + item IDs 批量获取文章内容

#[derive(Deserialize, Debug, Default)]
struct StreamItemsContentsForm {
    #[serde(rename = "i", alias = "i[]", default)]
    ids: Vec<String>,
    #[serde(default)]
    _t: Option<String>,
}

async fn stream_items_contents(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(query_params): Query<StreamItemsContentsForm>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<GReaderStreamContents>, (StatusCode, String)> {
    let mut ids = query_params.ids;

    // 尝试从 Body 解析 ID (支持 Form 和 JSON)
    if !body.is_empty() {
        let ct = headers
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if ct.contains("application/json") {
            if let Ok(json_data) = serde_json::from_slice::<StreamItemsContentsForm>(&body) {
                ids.extend(json_data.ids);
            }
        } else {
            // 手动解析表单数据以兼容重复的 i=... 或 i[]=... 参数
            let body_str = String::from_utf8_lossy(&body);
            for part in body_str.split('&') {
                let mut kv = part.split('=');
                if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
                    if k == "i" || k == "i[]" {
                        if let Ok(decoded) = urlencoding::decode(v) {
                            ids.push(decoded.into_owned());
                        }
                    }
                }
            }
        }
    }

    let item_ids: Vec<i64> = ids.iter().filter_map(|s| greader_to_item_id(s)).collect();

    if item_ids.is_empty() {
        return Ok(Json(GReaderStreamContents {
            id: STATE_READING_LIST.to_string(),
            title: "Items".to_string(),
            direction: "ltr".to_string(),
            self_link: vec![],
            updated: chrono::Utc::now().timestamp(),
            items: vec![],
            continuation: Some("".to_string()),
        }));
    }

    // 构建 IN 子句
    let placeholders = item_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 2))
        .collect::<Vec<_>>()
        .join(",");
    let query_str = format!(
        r#"
        SELECT 
            a.id, a.title, a.link, a.author, a.published_at, a.crawl_time,
            a.is_read, a.is_starred, a.feed_id, f.title as feed_title, f.site_url,
            COALESCE(a.content_skeleton, '') as skeleton, COALESCE(a.summary, '') as summary,
            s.need_translate, s.target_language
        FROM articles a
        JOIN feeds f ON a.feed_id = f.id
        JOIN subscriptions s ON s.feed_id = f.id
        WHERE s.user_id = ?1 AND a.id IN ({})
        "#,
        placeholders
    );

    let mut q = sqlx::query_as::<
        _,
        (
            i64,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
            Option<i64>,
            bool,
            bool,
            i64,
            String,
            Option<String>,
            String,
            String,
            bool,
            Option<String>,
        ),
    >(&query_str)
    .bind(auth.user_id);
    for id in &item_ids {
        q = q.bind(id);
    }

    let rows = q
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 批量拉取所有文章的 Blocks
    let block_query = format!(
        "SELECT article_id, block_index, raw_text, trans_text FROM article_blocks WHERE user_id = ?1 AND article_id IN ({}) ORDER BY article_id, block_index ASC",
        placeholders
    );
    let mut bq =
        sqlx::query_as::<_, crate::model::articles::ArticleBlock>(&block_query).bind(auth.user_id);
    for id in &item_ids {
        bq = bq.bind(id);
    }
    let all_blocks = bq
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 组织 Blocks 数据以便拼合
    use std::collections::HashMap;
    let mut blocks_map: HashMap<i64, Vec<crate::model::articles::ArticleBlock>> = HashMap::new();
    for block in all_blocks {
        blocks_map.entry(block.article_id).or_default().push(block);
    }

    let items = rows
        .into_iter()
        .map(
            |(
                id,
                mut title,
                link,
                author,
                pub_at,
                crawl_time,
                is_read,
                is_starred,
                feed_id,
                feed_title,
                site_url,
                skeleton,
                summary,
                need_translate,
                _target_lang,
            )| {
                let ct = crawl_time.unwrap_or(0);
                let ts = pub_at.unwrap_or(ct);

                // 1. 处理标题翻译 (block_index = -1)
                if let Some(article_blocks) = blocks_map.get(&id) {
                    if let Some(block) = article_blocks.iter().find(|b| b.block_index == -1) {
                        if let Some(ref trans_title) = block.trans_text {
                            title = trans_title.clone();
                        }
                    }
                }

                // 2. 统一调用 Service 层进行拼合
                let content_html = crate::services::articles::stitch_article_content(
                    &skeleton,
                    blocks_map.get(&id).map(|v| v.as_slice()).unwrap_or(&[]),
                    if summary.trim().is_empty() {
                        None
                    } else {
                        Some(&summary)
                    },
                    need_translate,
                );

                let mut categories = vec![STATE_READING_LIST.to_string()];
                if is_read {
                    categories.push(STATE_READ.to_string());
                } else {
                    categories.push(STATE_KEPT_UNREAD.to_string());
                }
                if is_starred {
                    categories.push(STATE_STARRED.to_string());
                }

                let link_str = link.unwrap_or_default();

                GReaderItem {
                    id: item_id_to_greader(id),
                    crawl_time_msec: (ct * 1000).to_string(),
                    timestamp_usec: (ct * 1_000_000).to_string(),
                    published: ts,
                    updated: ts,
                    title,
                    canonical: vec![GReaderLink {
                        href: link_str.clone(),
                    }],
                    alternate: vec![GReaderLink { href: link_str }],
                    summary: GReaderContent {
                        direction: "ltr".into(),
                        content: content_html.clone(),
                    },
                    content: Some(GReaderContent {
                        direction: "ltr".into(),
                        content: content_html,
                    }),
                    author: author.unwrap_or_default(),
                    categories,
                    origin: GReaderOrigin {
                        stream_id: format!("feed/{}", feed_id),
                        title: feed_title,
                        html_url: site_url.unwrap_or_default(),
                    },
                }
            },
        )
        .collect();

    Ok(Json(GReaderStreamContents {
        id: STATE_READING_LIST.to_string(),
        title: "Items".to_string(),
        direction: "ltr".to_string(),
        self_link: vec![],
        updated: chrono::Utc::now().timestamp(),
        items,
        continuation: Some("".to_string()),
    }))
}

// --- Action Handlers ---

async fn edit_tag(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut ids_list = Vec::new();
    let mut add_tags = Vec::new();
    let mut rem_tags = Vec::new();

    let body_str = String::from_utf8_lossy(&body);
    for part in body_str.split('&') {
        let mut kv = part.split('=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            if let Ok(decoded) = urlencoding::decode(v) {
                let val = decoded.into_owned();
                if k == "i" || k == "i[]" {
                    ids_list.push(val);
                } else if k == "a" || k == "a[]" {
                    add_tags.push(val);
                } else if k == "r" || k == "r[]" {
                    rem_tags.push(val);
                }
            }
        }
    }

    let mut tx = state.db.begin().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    for item_id_str in ids_list {
        let item_id = match greader_to_item_id(&item_id_str) {
            Some(id) => id,
            None => continue,
        };

        if !add_tags.is_empty() {
            if add_tags.iter().any(|t| t.contains("state/com.google/read")) {
                sqlx::query("UPDATE articles SET is_read = 1 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                    .bind(item_id).bind(auth.user_id).execute(&mut *tx).await.ok();
            }
            if add_tags.iter().any(|t| t.contains("state/com.google/starred")) {
                sqlx::query("UPDATE articles SET is_starred = 1 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                    .bind(item_id).bind(auth.user_id).execute(&mut *tx).await.ok();
            }
            if add_tags.iter().any(|t| t.contains("state/com.google/kept-unread")) {
                sqlx::query("UPDATE articles SET is_read = 0 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                    .bind(item_id).bind(auth.user_id).execute(&mut *tx).await.ok();
            }
        }

        if !rem_tags.is_empty() {
            if rem_tags.iter().any(|t| t.contains("state/com.google/read")) {
                sqlx::query("UPDATE articles SET is_read = 0 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                    .bind(item_id).bind(auth.user_id).execute(&mut *tx).await.ok();
            }
            if rem_tags.iter().any(|t| t.contains("state/com.google/starred")) {
                sqlx::query("UPDATE articles SET is_starred = 0 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                    .bind(item_id).bind(auth.user_id).execute(&mut *tx).await.ok();
            }
        }
    }

    tx.commit().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok("OK")
}

// --- Mark All As Read ---

#[derive(Deserialize)]
struct MarkAllAsReadForm {
    #[serde(default)]
    s: String, // Stream ID
    #[serde(default)]
    ts: Option<i64>, // 时间戳：只标记此时间之前的文章
}

async fn mark_all_as_read(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Form(payload): Form<MarkAllAsReadForm>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let mut query_str = String::from("UPDATE articles SET is_read = 1 WHERE is_read = 0");

    if payload.s.starts_with("feed/") {
        if let Ok(feed_id) = payload.s[5..].parse::<i64>() {
            query_str.push_str(&format!(" AND feed_id = {}", feed_id));
        }
    } else if payload.s.contains("/label/") {
        if let Some(label) = payload.s.split("/label/").last() {
            query_str.push_str(&format!(" AND feed_id IN (SELECT feed_id FROM subscriptions s2 JOIN folders fo ON s2.folder_id = fo.id WHERE s2.user_id = {} AND fo.title = '{}')", auth.user_id, label.replace("'", "''")));
        }
    }

    if let Some(ts) = payload.ts {
        let ts_sec = if ts > 10_000_000_000_000 {
            ts / 1_000_000
        } else if ts > 10_000_000_000 {
            ts / 1000
        } else {
            ts
        };
        query_str.push_str(&format!(" AND published_at <= {}", ts_sec));
    }

    sqlx::query(&query_str)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok("OK")
}

// --- Subscription edits (QuickAdd, Edit) ---

#[derive(Deserialize)]
struct QuickAddForm {
    quickadd: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QuickAddResponse {
    query: String,
    num_results: i32,
    stream_id: String,
}

async fn subscription_quickadd(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Form(payload): Form<QuickAddForm>,
) -> Result<Json<QuickAddResponse>, (StatusCode, String)> {
    let feed_url = payload.quickadd.trim();
    if feed_url.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "empty url".into()));
    }

    // Insert feed if not exists
    let feed_id: (i64,) = sqlx::query_as("INSERT INTO feeds (feed_url, title) VALUES (?, 'New Feed') ON CONFLICT(feed_url) DO UPDATE SET feed_url=excluded.feed_url RETURNING id")
        .bind(feed_url)
        .fetch_one(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Insert subscription
    let _ = sqlx::query("INSERT INTO subscriptions (user_id, feed_id, custom_title) VALUES (?, ?, '') ON CONFLICT(user_id, feed_id) DO NOTHING")
        .bind(auth.user_id)
        .bind(feed_id.0)
        .execute(&state.db)
        .await;

    Ok(Json(QuickAddResponse {
        query: feed_url.to_string(),
        num_results: 1,
        stream_id: format!("feed/{}", feed_id.0),
    }))
}

#[derive(Deserialize, Default)]
struct SubscriptionEditForm {
    #[serde(default)]
    ac: String, // action: edit / unsubscribe
    #[serde(default)]
    s: String, // feed stream id
    #[serde(default)]
    t: Option<String>, // title
    #[serde(default)]
    a: Option<String>, // add folder
    #[serde(default)]
    r: Option<String>, // remove folder
}

async fn subscription_edit(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Form(payload): Form<SubscriptionEditForm>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if !payload.s.starts_with("feed/") {
        return Ok("OK");
    }
    let feed_id = payload.s[5..].parse::<i64>().unwrap_or(0);
    if feed_id == 0 {
        return Ok("OK");
    }

    if payload.ac == "unsubscribe" {
        sqlx::query("DELETE FROM subscriptions WHERE user_id = ? AND feed_id = ?")
            .bind(auth.user_id)
            .bind(feed_id)
            .execute(&state.db)
            .await
            .ok();
        return Ok("OK");
    }

    if payload.ac == "edit" {
        if let Some(title) = payload.t {
            if !title.is_empty() {
                sqlx::query(
                    "UPDATE subscriptions SET custom_title = ? WHERE user_id = ? AND feed_id = ?",
                )
                .bind(title)
                .bind(auth.user_id)
                .bind(feed_id)
                .execute(&state.db)
                .await
                .ok();
            }
        }

        if let Some(folder_id_str) = payload.a {
            if let Some(label) = folder_id_str.split("/label/").last() {
                let folder_rec: Result<(i64,), _> = sqlx::query_as("INSERT INTO folders (user_id, title) VALUES (?, ?) ON CONFLICT(user_id, title) DO UPDATE SET title=excluded.title RETURNING id")
                    .bind(auth.user_id)
                    .bind(label)
                    .fetch_one(&state.db)
                    .await;

                if let Ok((folder_id,)) = folder_rec {
                    sqlx::query(
                        "UPDATE subscriptions SET folder_id = ? WHERE user_id = ? AND feed_id = ?",
                    )
                    .bind(folder_id)
                    .bind(auth.user_id)
                    .bind(feed_id)
                    .execute(&state.db)
                    .await
                    .ok();
                }
            }
        } else if let Some(folder_id_str) = payload.r {
            if folder_id_str.contains("/label/") {
                // If removing folder, we just set folder_id to NULL
                sqlx::query(
                    "UPDATE subscriptions SET folder_id = NULL WHERE user_id = ? AND feed_id = ?",
                )
                .bind(auth.user_id)
                .bind(feed_id)
                .execute(&state.db)
                .await
                .ok();
            }
        }
    }

    Ok("OK")
}

// --- Tag Actions ---

#[derive(Deserialize)]
struct DisableTagForm {
    #[serde(default)]
    s: String,
}

async fn disable_tag(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Form(payload): Form<DisableTagForm>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if let Some(label) = payload.s.split("/label/").last() {
        // Delete folder
        let folder_rec: Result<(i64,), _> =
            sqlx::query_as("SELECT id FROM folders WHERE user_id = ? AND title = ?")
                .bind(auth.user_id)
                .bind(label)
                .fetch_one(&state.db)
                .await;

        if let Ok((folder_id,)) = folder_rec {
            sqlx::query(
                "UPDATE subscriptions SET folder_id = NULL WHERE user_id = ? AND folder_id = ?",
            )
            .bind(auth.user_id)
            .bind(folder_id)
            .execute(&state.db)
            .await
            .ok();

            sqlx::query("DELETE FROM folders WHERE id = ? AND user_id = ?")
                .bind(folder_id)
                .bind(auth.user_id)
                .execute(&state.db)
                .await
                .ok();
        }
    }
    Ok("OK")
}

#[derive(Deserialize)]
struct RenameTagForm {
    #[serde(default)]
    s: String,
    #[serde(default)]
    dest: String,
}

async fn rename_tag(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Form(payload): Form<RenameTagForm>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if let (Some(old_label), Some(new_label)) = (
        payload.s.split("/label/").last(),
        payload.dest.split("/label/").last(),
    ) {
        sqlx::query("UPDATE folders SET title = ? WHERE user_id = ? AND title = ?")
            .bind(new_label)
            .bind(auth.user_id)
            .bind(old_label)
            .execute(&state.db)
            .await
            .ok();
    }
    Ok("OK")
}
