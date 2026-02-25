use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Feed {
    pub id: i64,
    pub feed_url: String,
    pub site_url: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub last_fetched_at: Option<DateTime<Utc>>,
    pub etag: Option<String>,
    pub icon_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFeedRequest {
    pub feed_url: String,
    pub site_url: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFeedRequest {
    pub feed_url: Option<String>,
    pub site_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    pub guid: String,
    pub feed_id: i64,
    pub title: String,
    pub link: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<i64>, // Unix timestamp

    pub content_skeleton: Option<String>,
    pub is_read: bool,
    pub is_starred: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ArticleBlock {
    pub article_guid: String,
    pub block_index: i32,
    pub raw_text: String,
    pub summary: Option<String>,
    pub trans_text: Option<String>,
    pub user_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleDetail {
    #[serde(flatten)]
    pub article: Article,
    pub blocks: Vec<ArticleBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListArticlesRequest {
    pub feed_id: Option<i64>,
    pub unread_only: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateArticleRequest {
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
}
