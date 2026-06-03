use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct AccessKey {
    pub id: i64,
    pub user_id: i64,
    pub key: String,
    pub name: String,
    /// JSON 数组，如 ["articles:read","subscriptions:*"]
    pub permissions: String,
    pub created_at: Option<String>,
    pub last_used_at: Option<String>,
}

/// 列表返回时隐藏完整 key，只显示前缀
#[derive(Serialize, ToSchema)]
pub struct AccessKeyInfo {
    pub id: i64,
    pub name: String,
    pub key_prefix: String,
    pub permissions: Vec<String>,
    pub created_at: Option<String>,
    pub last_used_at: Option<String>,
}

impl AccessKey {
    /// 解析 permissions JSON 为 Vec<String>
    pub fn parse_permissions(&self) -> Vec<String> {
        serde_json::from_str(&self.permissions).unwrap_or_default()
    }

    /// 返回脱敏的 key 前缀
    pub fn key_prefix(&self) -> String {
        if self.key.len() > 12 {
            format!("{}...{}", &self.key[..8], &self.key[self.key.len() - 4..])
        } else {
            self.key.clone()
        }
    }

    /// 转换为对外展示的 AccessKeyInfo
    pub fn to_info(&self) -> AccessKeyInfo {
        AccessKeyInfo {
            id: self.id,
            name: self.name.clone(),
            key_prefix: self.key_prefix(),
            permissions: self.parse_permissions(),
            created_at: self.created_at.clone(),
            last_used_at: self.last_used_at.clone(),
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct CreateAccessKeyRequest {
    pub name: String,
    pub permissions: Vec<String>,
}

/// 创建成功后返回完整 key（仅此一次）
#[derive(Serialize, ToSchema)]
pub struct CreateAccessKeyResponse {
    pub id: i64,
    pub name: String,
    pub key: String,
    pub permissions: Vec<String>,
}

/// 检查权限列表中是否匹配指定 resource:action
/// 支持格式: "articles:read", "articles:*", "*"
pub fn check_permission(permissions: &[String], resource: &str, action: &str) -> bool {
    for perm in permissions {
        let parts: Vec<&str> = perm.splitn(2, ':').collect();
        match parts.as_slice() {
            ["*"] => return true,
            [res, "*"] if *res == resource => return true,
            [res, act] if *res == resource && *act == action => return true,
            _ => {}
        }
    }
    false
}
