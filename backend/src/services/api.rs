use crate::model::api_config::{ApiConfig, CreateApiConfigRequest, UpdateApiConfigRequest};
use sqlx::SqlitePool;

/// 创建新的 API 配置
pub async fn create_config(pool: &SqlitePool, req: CreateApiConfigRequest) -> anyhow::Result<i64> {
    // 将 settings JSON 对象转换为字符串存储到数据库
    let settings_json = req.settings.unwrap_or(serde_json::json!({})).to_string();

    let id = sqlx::query("INSERT INTO api_configs (name, api_type, api_key, base_url, settings, timeout_seconds, retry_count, retry_interval_ms, retry_enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(req.name)
        .bind(req.api_type)
        .bind(req.api_key)
        .bind(req.base_url)
        .bind(settings_json)
        .bind(req.timeout_seconds.unwrap_or(180))
        .bind(req.retry_count.unwrap_or(3))
        .bind(req.retry_interval_ms.unwrap_or(1000))
        .bind(req.retry_enabled.unwrap_or(true))
        .execute(pool)
        .await?
        .last_insert_rowid();

    Ok(id)
}

/// 更新现有的 API 配置
pub async fn update_config(
    pool: &SqlitePool,
    id: i64,
    req: UpdateApiConfigRequest,
) -> anyhow::Result<()> {
    // 1. 先查出旧数据，用于部分更新
    let old: ApiConfig = sqlx::query_as("SELECT * FROM api_configs WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;

    // 2. 只有新字段有值时才覆盖，否则保留旧值
    let name = req.name.unwrap_or(old.name);
    let api_type = req.api_type.unwrap_or(old.api_type);
    let api_key = req.api_key.or(old.api_key);
    let base_url = req.base_url.or(old.base_url);
    let timeout_seconds = req.timeout_seconds.unwrap_or(old.timeout_seconds);
    let retry_count = req.retry_count.unwrap_or(old.retry_count);
    let retry_interval_ms = req.retry_interval_ms.unwrap_or(old.retry_interval_ms);
    let retry_enabled = req.retry_enabled.unwrap_or(old.retry_enabled);

    // 如果传入了新的 settings JSON，则更新，否则保持旧的（保持字符串形式）
    let settings_str = if let Some(new_settings) = req.settings {
        new_settings.to_string()
    } else {
        old.settings
    };

    sqlx::query("UPDATE api_configs SET name = ?, api_type = ?, api_key = ?, base_url = ?, settings = ?, timeout_seconds = ?, retry_count = ?, retry_interval_ms = ?, retry_enabled = ? WHERE id = ?")
        .bind(name)
        .bind(api_type)
        .bind(api_key)
        .bind(base_url)
        .bind(settings_str)
        .bind(timeout_seconds)
        .bind(retry_count)
        .bind(retry_interval_ms)
        .bind(retry_enabled)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

/// 获取单个配置
pub async fn get_config(pool: &SqlitePool, id: i64) -> anyhow::Result<ApiConfig> {
    let config = sqlx::query_as("SELECT * FROM api_configs WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(config)
}

/// 列出所有配置
pub async fn list_configs(pool: &SqlitePool) -> anyhow::Result<Vec<ApiConfig>> {
    let configs = sqlx::query_as("SELECT * FROM api_configs")
        .fetch_all(pool)
        .await?;
    Ok(configs)
}

/// 删除配置
pub async fn delete_config(pool: &SqlitePool, id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM api_configs WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
