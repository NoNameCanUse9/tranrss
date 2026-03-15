use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiConfig {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub api_type: String, // "openai", "deeplx", etc.
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub settings: String, // JSON string for extra params like "model"
    pub timeout_seconds: i32,
    pub retry_count: i32,
    pub retry_interval_ms: i32,
    pub retry_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiConfigRequest {
    pub name: String,
    pub api_type: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub settings: Option<serde_json::Value>,
    pub timeout_seconds: Option<i32>,
    pub retry_count: Option<i32>,
    pub retry_interval_ms: Option<i32>,
    pub retry_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApiConfigRequest {
    pub name: Option<String>,
    pub api_type: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub settings: Option<serde_json::Value>,
    pub timeout_seconds: Option<i32>,
    pub retry_count: Option<i32>,
    pub retry_interval_ms: Option<i32>,
    pub retry_enabled: Option<bool>,
}
