use crate::AppState;
use crate::model::user::User;
use crate::services::auth::{self, AuthUser};
use axum::{
    Form, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
        // Actions
        .route("/reader/api/0/edit-tag", post(edit_tag))
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GReaderLoginRequest {
    email: String,
    passwd: String,
}

// --- Auth Handlers ---

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

    Ok(format!("SID=ignored\nLSID=ignored\nAuth={}\n", token))
}

async fn get_token(_auth: AuthUser) -> impl IntoResponse {
    "your_token_here" // GReader expects a token for POST requests, but for simple ones any string works
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GReaderUserInfo {
    user_id: String,
    user_name: String,
}

async fn user_info(auth: AuthUser) -> Json<GReaderUserInfo> {
    Json(GReaderUserInfo {
        user_id: auth.user_id.to_string(),
        user_name: auth.username,
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
    let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as(
        r#"
        SELECT s.feed_id, f.title, folder.title as folder_title
        FROM subscriptions s
        JOIN feeds f ON s.feed_id = f.id
        LEFT JOIN folders folder ON s.folder_id = folder.id
        WHERE s.user_id = ?
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let subscriptions = rows
        .into_iter()
        .map(|(feed_id, title, folder_title)| {
            let mut categories = Vec::new();
            if let Some(ft) = folder_title {
                categories.push(GReaderCategory {
                    id: format!("user/{}/label/{}", auth.user_id, ft),
                    label: ft,
                });
            }
            GReaderSubscription {
                id: format!("feed/{}", feed_id),
                title,
                categories,
                sortid: feed_id.to_string(),
                firstitemmsec: "0".to_string(),
            }
        })
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
            id: format!("user/{}/state/com.google/read", auth.user_id),
            sortid: "1".into(),
        },
        GReaderTag {
            id: format!("user/{}/state/com.google/starred", auth.user_id),
            sortid: "2".into(),
        },
    ];

    for (title,) in folders {
        tags.push(GReaderTag {
            id: format!("user/{}/label/{}", auth.user_id, title),
            sortid: "10".into(),
        });
    }

    Ok(Json(GReaderTagList { tags }))
}

// --- Content Handlers ---

#[derive(Deserialize)]
struct StreamQuery {
    n: Option<usize>,      // Number of items
    xt: Option<String>,   // Exclude target (e.g. read items)
    r: Option<String>,    // Order (n for newest first)
}


#[derive(Serialize)]
struct GReaderStreamContents {
    id: String,
    title: String,
    items: Vec<GReaderItem>,
    updated: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GReaderItem {
    id: String,
    title: String,
    published: i64,
    updated: i64,
    canonical: Vec<GReaderLink>,
    alternate: Vec<GReaderLink>,
    summary: GReaderContent,
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
    content: String,
}

#[derive(Serialize)]
struct GReaderOrigin {
    stream_id: String,
    title: String,
    html_url: String,
}

async fn stream_contents(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(stream_id): Path<String>,
    Query(params): Query<StreamQuery>,
) -> Result<Json<GReaderStreamContents>, (StatusCode, String)> {
    let mut query_str = String::from(
        r#"
        SELECT 
            a.id, a.title, a.link, a.author, a.published_at, a.updated_at,
            a.is_read, a.is_starred, a.feed_id, f.title as feed_title, f.site_url,
            a.content_skeleton, a.summary
        FROM articles a
        JOIN feeds f ON a.feed_id = f.id
        JOIN subscriptions s ON s.feed_id = f.id
        WHERE s.user_id = ? 
        "#,
    );

    // Filter by stream_id
    if stream_id.starts_with("feed/") {
        let feed_id = stream_id[5..].parse::<i64>().unwrap_or(0);
        query_str.push_str(&format!(" AND a.feed_id = {}", feed_id));
    } else if stream_id.contains("state/com.google/starred") {
        query_str.push_str(" AND a.is_starred = 1");
    } else if stream_id.contains("state/com.google/reading-list") {
        // all items
    }

    // Exclude read items if xt=...read
    if let Some(ref xt) = params.xt {
        if xt.contains("state/com.google/read") {
            query_str.push_str(" AND a.is_read = 0");
        }
    }

    // Sort order
    if params.r.as_deref() == Some("o") {
        query_str.push_str(" ORDER BY a.published_at ASC");
    } else {
        query_str.push_str(" ORDER BY a.published_at DESC");
    }

    // Limit
    let limit = params.n.unwrap_or(20);
    query_str.push_str(&format!(" LIMIT {}", limit));

    let rows: Vec<(
        i64,
        String,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<String>,
        bool,
        bool,
        i64,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(&query_str)
        .bind(auth.user_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|(id, title, link, author, pub_at, _up_at, is_read, is_starred, feed_id, feed_title, site_url, skeleton, summary)| {
            let mut categories = Vec::new();
            if is_read {
                categories.push(format!("user/{}/state/com.google/read", auth.user_id));
            } else {
                categories.push(format!("user/{}/state/com.google/kept-unread", auth.user_id));
            }
            if is_starred {
                categories.push(format!("user/{}/state/com.google/starred", auth.user_id));
            }

            // Simple content reconstruction (placeholder since we don't want to fetch all blocks here for performance if not needed)
            // But GReader usually wants the content. For now we use summary or skeleton
            let content_text = skeleton.unwrap_or_else(|| summary.unwrap_or_default());

            GReaderItem {
                id: id.to_string(),
                title,
                published: pub_at.unwrap_or(0),
                updated: pub_at.unwrap_or(0),
                canonical: vec![GReaderLink { href: link.clone().unwrap_or_default() }],
                alternate: vec![GReaderLink { href: link.unwrap_or_default() }],
                summary: GReaderContent { content: content_text.clone() },
                content: Some(GReaderContent { content: content_text }),
                author: author.unwrap_or_default(),
                categories,
                origin: GReaderOrigin {
                    stream_id: format!("feed/{}", feed_id),
                    title: feed_title,
                    html_url: site_url.unwrap_or_default(),
                },
            }
        })
        .collect();

    Ok(Json(GReaderStreamContents {
        id: stream_id,
        title: "TranRSS".into(),
        updated: chrono::Utc::now().timestamp(),
        items,
    }))
}

// --- Action Handlers ---

#[derive(Deserialize)]
struct EditTagForm {
    i: Vec<String>, // Item IDs
    a: Option<Vec<String>>, // Add tags
    r: Option<Vec<String>>, // Remove tags
}

async fn edit_tag(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Form(payload): Form<EditTagForm>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    for item_id_str in payload.i {
        let item_id = item_id_str.parse::<i64>().unwrap_or(0);
        
        // Handle read/unread
        if let Some(ref add_tags) = payload.a {
            if add_tags.iter().any(|t| t.contains("state/com.google/read")) {
                sqlx::query("UPDATE articles SET is_read = 1 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                    .bind(item_id).bind(auth.user_id).execute(&state.db).await.ok();
            }
            if add_tags.iter().any(|t| t.contains("state/com.google/starred")) {
                sqlx::query("UPDATE articles SET is_starred = 1 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                    .bind(item_id).bind(auth.user_id).execute(&state.db).await.ok();
            }
        }
        
        if let Some(ref rem_tags) = payload.r {
            if rem_tags.iter().any(|t| t.contains("state/com.google/read")) {
                sqlx::query("UPDATE articles SET is_read = 0 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                    .bind(item_id).bind(auth.user_id).execute(&state.db).await.ok();
            }
            if rem_tags.iter().any(|t| t.contains("state/com.google/starred")) {
                sqlx::query("UPDATE articles SET is_starred = 0 WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)")
                    .bind(item_id).bind(auth.user_id).execute(&state.db).await.ok();
            }
        }
    }

    Ok("OK")
}
