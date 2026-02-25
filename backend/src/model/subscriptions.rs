use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subscription {
    pub id: i64,
    pub user_id: i64,
    pub feed_id: i64,
    pub folder_id: Option<i64>,
    pub custom_title: Option<String>,
    pub need_translate: bool,
    pub need_summary: bool,
    pub target_language: Option<String>,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Folder {
    pub id: i64,
    pub name: String,
    pub user_id: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDetail {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub category: String,
    pub article_count: i64,
    pub last_sync: Option<DateTime<Utc>>,
    pub status: String,
    pub language: String,
    pub auto_translate: bool,
    pub need_summary: bool,
    pub site_url: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionRequest {
    pub feed_url: String,
    pub folder_id: Option<i64>,
    pub custom_title: Option<String>,
    pub need_translate: Option<bool>,
    pub need_summary: Option<bool>,
    pub site_url: Option<String>,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubscriptionRequest {
    pub folder_id: Option<i64>,
    pub custom_title: Option<String>,
    pub need_translate: Option<bool>,
    pub need_summary: Option<bool>,
    pub target_language: Option<String>,
}
