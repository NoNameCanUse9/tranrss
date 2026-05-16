use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::NaiveDateTime;
use utoipa::ToSchema;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ApiUsage {
    pub id: i64,
    pub user_id: i64,
    pub api_config_id: i64,
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiUsageStats {
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_tokens: i64,
    pub usage_by_model: Vec<ModelUsage>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ModelUsage {
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct TimeSeriesUsage {
    pub date: String,
    pub api_config_id: i64,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}
