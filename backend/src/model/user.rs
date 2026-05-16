use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub translate_api_id: Option<i64>,
    pub summary_api_id: Option<i64>,
    pub default_api_id: Option<i64>,
    pub app_mode: Option<bool>,
    pub log_num_limit: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateUsernameRequest {
    pub new_username: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateUserSettingRequest {
    pub translate_api_id: Option<i64>,
    pub summary_api_id: Option<i64>,
    pub default_api_id: Option<i64>,
    pub greader_api: Option<bool>,
    pub fever_api: Option<bool>,
    pub api_proxy: Option<bool>,
    pub api_proxy_url: Option<String>,
    pub app_mode: Option<bool>,
    pub log_num_limit: Option<i32>,
    pub custom_trans_style: Option<String>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct UserSetting {
    pub translate_api_id: Option<i64>,
    pub summary_api_id: Option<i64>,
    pub default_api_id: Option<i64>,
    pub greader_api: Option<bool>,
    pub fever_api: Option<bool>,
    pub api_proxy: Option<bool>,
    pub api_proxy_url: Option<String>,
    pub app_mode: Option<bool>,
    pub user_id: i64,
    pub log_num_limit: Option<i32>,
    pub custom_trans_style: Option<String>,
}
