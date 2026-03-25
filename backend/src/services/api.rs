use crate::model::api_config::{ApiConfig, CreateApiConfigRequest, UpdateApiConfigRequest};
use crate::model::api_usage::{ApiUsageStats, ModelUsage};
use crate::utils::crypto::{decrypt_safe, encrypt};
use sqlx::SqlitePool;

/// 创建新的 API 配置
pub async fn create_config(
    pool: &SqlitePool,
    user_id: i64,
    req: CreateApiConfigRequest,
) -> anyhow::Result<i64> {
    // 将 settings JSON 对象转换为字符串存储到数据库
    let settings_json = req.settings.unwrap_or(serde_json::json!({})).to_string();

    // 加密 API Key
    let api_key = req.api_key.as_deref().map(encrypt);

    let id = sqlx::query("INSERT INTO api_configs (user_id, name, api_type, api_key, base_url, settings, timeout_seconds, retry_count, retry_interval_ms, retry_enabled) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(user_id)
        .bind(req.name)
        .bind(req.api_type)
        .bind(api_key)
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
    user_id: i64,
    req: UpdateApiConfigRequest,
) -> anyhow::Result<()> {
    // 1. 先查出旧数据，用于部分更新，同时校验所有权
    let old: ApiConfig = sqlx::query_as("SELECT * FROM api_configs WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    let name = req.name.unwrap_or(old.name);
    let api_type = req.api_type.unwrap_or(old.api_type);

    // 加密新的 API Key，如果没有传则保留旧的
    let api_key = if let Some(ref new_key) = req.api_key {
        Some(encrypt(new_key))
    } else {
        old.api_key
    };

    let base_url = req.base_url.or(old.base_url);
    let timeout_seconds = req.timeout_seconds.unwrap_or(old.timeout_seconds);
    let retry_count = req.retry_count.unwrap_or(old.retry_count);
    let retry_interval_ms = req.retry_interval_ms.unwrap_or(old.retry_interval_ms);
    let retry_enabled = req.retry_enabled.unwrap_or(old.retry_enabled);

    let settings_str = if let Some(new_settings) = req.settings {
        new_settings.to_string()
    } else {
        old.settings
    };

    sqlx::query("UPDATE api_configs SET name = ?, api_type = ?, api_key = ?, base_url = ?, settings = ?, timeout_seconds = ?, retry_count = ?, retry_interval_ms = ?, retry_enabled = ? WHERE id = ? AND user_id = ?")
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
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// 获取单个配置 (含所有权校验)
pub async fn get_config(pool: &SqlitePool, id: i64, user_id: i64) -> anyhow::Result<ApiConfig> {
    let mut config: ApiConfig = sqlx::query_as("SELECT * FROM api_configs WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    // 解密给前端使用
    if let Some(ref k) = config.api_key {
        config.api_key = Some(decrypt_safe(k));
    }

    Ok(config)
}

/// 列出用户的所有配置
pub async fn list_configs(pool: &SqlitePool, user_id: i64) -> anyhow::Result<Vec<ApiConfig>> {
    let mut configs: Vec<ApiConfig> = sqlx::query_as("SELECT * FROM api_configs WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await?;

    // 全部解密
    for config in &mut configs {
        if let Some(ref k) = config.api_key {
            config.api_key = Some(decrypt_safe(k));
        }
    }

    Ok(configs)
}

/// 删除配置 (含所有权校验)
pub async fn delete_config(pool: &SqlitePool, id: i64, user_id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM api_configs WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 获取用户有效的 API ID（优先具体设置，次之默认设置，最后取序号最前的 API）
pub async fn get_effective_api_id(
    pool: &SqlitePool,
    user_id: i64,
    specific_api_id: Option<i64>,
) -> anyhow::Result<Option<i64>> {
    // 1. 如果有指定的 API ID，则直接映射 (如果指定的 ID 存在)
    if let Some(id) = specific_api_id {
        return Ok(Some(id));
    }

    // 2. 检查 user_setting 中的 default_api_id
    let default_api_id: Option<i64> =
        sqlx::query_scalar("SELECT default_api_id FROM user_setting WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(pool)
            .await?;

    if let Some(id) = default_api_id {
        return Ok(Some(id));
    }

    // 3. 回退逻辑：获取该用户序号最前（ID 最小）的 API
    let first_id: Option<i64> =
        sqlx::query_scalar("SELECT id FROM api_configs WHERE user_id = ? ORDER BY id ASC LIMIT 1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    Ok(first_id)
}

/// 获取用户 API 使用统计摘要
pub async fn get_usage_summary(
    pool: &SqlitePool,
    user_id: i64,
) -> anyhow::Result<ApiUsageStats> {
    // 1. 获取总代币使用情况
    let total: (i64, i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(prompt_tokens), 0), COALESCE(SUM(completion_tokens), 0), COALESCE(SUM(total_tokens), 0) FROM api_usage WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    // 2. 获取按模型分的统计情况
    let usage_by_model = sqlx::query_as::<_, ModelUsage>(
        "SELECT model, SUM(prompt_tokens) as prompt_tokens, SUM(completion_tokens) as completion_tokens, SUM(total_tokens) as total_tokens FROM api_usage WHERE user_id = ? GROUP BY model"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(ApiUsageStats {
        total_prompt_tokens: total.0,
        total_completion_tokens: total.1,
        total_tokens: total.2,
        usage_by_model,
    })
}

/// 获取用户的 Token 使用历史 (按天分组)
pub async fn get_usage_history(
    pool: &SqlitePool,
    user_id: i64,
) -> anyhow::Result<Vec<crate::model::api_usage::TimeSeriesUsage>> {
    let history = sqlx::query_as::<_, crate::model::api_usage::TimeSeriesUsage>(
        r#"
        SELECT 
            strftime('%Y-%m-%d', created_at) as date, 
            api_config_id, 
            model, 
            SUM(prompt_tokens) as prompt_tokens, 
            SUM(completion_tokens) as completion_tokens, 
            SUM(total_tokens) as total_tokens 
        FROM api_usage 
        WHERE user_id = ? 
        GROUP BY date, api_config_id, model
        ORDER BY date ASC
        "#
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(history)
}

