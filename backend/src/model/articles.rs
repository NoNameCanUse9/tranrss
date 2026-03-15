use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ArticleListItem {
    pub id: i64,
    pub title: String,
    pub author: Option<String>,
    pub published_at: Option<i64>,
    pub is_read: bool,
    pub is_starred: bool,
    pub feed_id: i64,
    pub feed_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ArticleDetail {
    pub id: i64,
    pub title: String,
    pub link: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<i64>,
    pub content_skeleton: Option<String>,
    pub is_read: bool,
    pub is_starred: bool,
    pub summary: Option<String>,
    pub need_translate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ArticleBlock {
    pub article_id: i64,
    pub block_index: i32,
    pub raw_text: String,
    pub trans_text: Option<String>,
}
