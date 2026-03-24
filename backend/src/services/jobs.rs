use crate::AppState;
use crate::model::api_config::ApiConfig;
use crate::services::{ai::AiService, feeds};
use anyhow::Result;
use apalis::prelude::*;
use apalis::layers::WorkerBuilderExt;
use apalis_cron::{CronStream, Schedule};
use apalis_sql::sqlite::SqliteStorage;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::str::FromStr;
use std::sync::Arc;
// use std::time::Duration;

// --- 任务定义 ---
// 在 apalis 0.7.4 中，不需要实现 Job trait。
// 只需要派生 Serialize 和 Deserialize 即可。

/// 同步单个 Feed 的任务
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncFeedJob {
    pub feed_id: i64,
    pub initiator_user_id: Option<i64>,
}

/// 翻译文章的任务
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranslateArticleJob {
    pub user_id: i64,
    pub article_id: i64,
}

/// 总结文章的任务
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SummarizeArticleJob {
    pub user_id: i64,
    pub article_id: i64,
}

/// 定期检查所有 Feed 是否需要更新的任务
/// Cron 任务需要实现 Default
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RefreshFeedsJob;

// --- 处理器 (Handlers) ---

async fn sync_feed_handler(
    job: SyncFeedJob,
    data: Data<Arc<AppState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    tracing::info!("⬇️ 开始处理同步任务: [FeedID: {}]", job.feed_id);
    let user_id = if let Some(uid) = job.initiator_user_id {
        uid
    } else {
        match sqlx::query_scalar::<_, i64>(
            "SELECT user_id FROM subscriptions WHERE feed_id = ? LIMIT 1",
        )
        .bind(job.feed_id)
        .fetch_optional(&data.db)
        .await?
        {
            Some(uid) => uid,
            None => {
                tracing::warn!("Feed {} 没有订阅者，跳过同步", job.feed_id);
                return Ok(());
            }
        }
    };
    feeds::fetch_and_process_feed(&data.db, user_id, job.feed_id)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )) as Box<dyn std::error::Error + Send + Sync + 'static>
        })?;

    // 同步开启下游任务 (翻译/摘要)
    // 策略：仅对最近 1 小时更新的文章中、且由于刚同步产生的新文章触发任务。
    // 为了防止“爆发式”IO（如几天没开后一次性同步几百篇），这里对每个 Feed 每次同步自动触发的任务数做限制 (LIMIT 20)。
    let subscribers = sqlx::query(
        "SELECT user_id, need_translate, need_summary FROM subscriptions WHERE feed_id = ?",
    )
    .bind(job.feed_id)
    .fetch_all(&data.db)
    .await?;

    for sub in subscribers {
        let sub_user_id: i64 = sub.get("user_id");
        let need_translate: bool = sub.get("need_translate");
        let need_summary: bool = sub.get("need_summary");

        if need_translate {
            let to_translate = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT a.id FROM articles a
                WHERE a.feed_id = ? 
                  AND a.updated_at > datetime('now', '-1 hour')
                  AND NOT EXISTS (
                      SELECT 1 FROM article_blocks b 
                      WHERE b.article_id = a.id AND b.user_id = ? AND b.trans_text IS NOT NULL
                  )
                  AND NOT EXISTS (
                      SELECT 1 FROM Jobs j
                      WHERE j.job_type LIKE '%TranslateArticleJob%'
                        AND j.job LIKE '%"article_id":' || a.id || '%'
                        AND j.job LIKE '%"user_id":' || ? || '%'
                        AND j.status IN ('Pending', 'Running', 'Killed')
                  )
                ORDER BY a.published_at DESC
                LIMIT 20
                "#,
            )
            .bind(job.feed_id)
            .bind(sub_user_id)
            .bind(sub_user_id)
            .fetch_all(&data.db)
            .await?;

            let mut storage = data.translate_queue.clone();
            for aid in to_translate {
                let _ = storage
                    .push(TranslateArticleJob {
                        user_id: sub_user_id,
                        article_id: aid,
                    })
                    .await;
            }
        }

        if need_summary {
            let to_summarize = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT a.id FROM articles a
                WHERE a.feed_id = ? 
                  AND a.updated_at > datetime('now', '-1 hour')
                  AND a.summary IS NULL
                  AND NOT EXISTS (
                      SELECT 1 FROM Jobs j 
                      WHERE j.job_type LIKE '%SummarizeArticleJob%'
                        AND j.job LIKE '%"article_id":' || a.id || '%'
                        AND j.job LIKE '%"user_id":' || ? || '%'
                        AND j.status IN ('Pending', 'Running', 'Killed')
                  )
                ORDER BY a.published_at DESC
                LIMIT 20
                "#,
            )
            .bind(job.feed_id)
            .bind(sub_user_id)
            .fetch_all(&data.db)
            .await?;

            let mut storage = data.summarize_queue.clone();
            for aid in to_summarize {
                let _ = storage
                    .push(SummarizeArticleJob {
                        user_id: sub_user_id,
                        article_id: aid,
                    })
                    .await;
            }
        }
    }
    tracing::info!("✅ 同步任务处理完成: [FeedID: {}]", job.feed_id);
    Ok(())
}

async fn translate_article_handler(
    job: TranslateArticleJob,
    data: Data<Arc<AppState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let ai = get_ai_service_for_user(&data.db, job.user_id)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )) as Box<dyn std::error::Error + Send + Sync + 'static>
        })?;
    ai.translate_article(&data, job.user_id, job.article_id)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )) as Box<dyn std::error::Error + Send + Sync + 'static>
        })?;
    Ok(())
}

async fn summarize_article_handler(
    job: SummarizeArticleJob,
    data: Data<Arc<AppState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let ai = get_summary_ai_service_for_user(&data.db, job.user_id)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )) as Box<dyn std::error::Error + Send + Sync + 'static>
        })?;
    ai.summarize_article(&data, job.user_id, job.article_id)
        .await
        .map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )) as Box<dyn std::error::Error + Send + Sync + 'static>
        })?;
    Ok(())
}

async fn refresh_feeds_handler(
    _job: RefreshFeedsJob,
    data: Data<Arc<AppState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    tracing::info!("🕒 Cron: 开始检查需要刷新的 Feed...");
    let due_feeds = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT f.id 
        FROM feeds f
        JOIN subscriptions s ON f.id = s.feed_id
        WHERE NOT EXISTS (
            SELECT 1 FROM inactive_feeds inf 
            WHERE inf.user_id = s.user_id AND inf.feed_id = f.id
          )
          AND NOT EXISTS (
            SELECT 1 FROM Jobs j 
            WHERE j.job_type LIKE '%SyncFeedJob%' 
              AND json_extract(j.job, '$.feed_id') = f.id
              AND j.status IN ('Pending', 'Running')
        )
        GROUP BY f.id
        HAVING f.last_fetched_at IS NULL OR 
               datetime(f.last_fetched_at, '+' || MIN(s.refresh_interval) || ' minutes') < datetime('now')
        "#,
    )
    .fetch_all(&data.db)
    .await
    .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync + 'static>)?;

    let mut storage = data.sync_queue.clone();
    tracing::info!("🕒 Cron: 找到 {} 个待刷新 Feed", due_feeds.len());
    for fid in due_feeds {
        tracing::info!("🕒 Cron: 为 Feed {} 调度同步任务", fid);
        let _ = storage
            .push(SyncFeedJob {
                feed_id: fid,
                initiator_user_id: None,
            })
            .await;
    }
    Ok(())
}

// --- 辅助函数 ---

pub async fn get_ai_service_for_user(db: &SqlitePool, user_id: i64) -> Result<AiService> {
    let translate_api_id: Option<i64> =
        sqlx::query_scalar("SELECT translate_api_id FROM user_setting WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(db)
            .await?;

    let api_id = crate::services::api::get_effective_api_id(db, user_id, translate_api_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No translation API configured for user {}", user_id))?;
    let config: ApiConfig = sqlx::query_as("SELECT * FROM api_configs WHERE id = ?")
        .bind(api_id)
        .fetch_one(db)
        .await?;
    let settings: serde_json::Value = serde_json::from_str(&config.settings).unwrap_or_default();
    let model = settings
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-3.5-turbo")
        .to_string();
    let target_lang = settings
        .get("target_lang")
        .and_then(|v| v.as_str())
        .unwrap_or("Chinese")
        .to_string();
    Ok(AiService::new(target_lang, model, config))
}

/// 为总结任务获取 AI 服务，使用 summary_api_id
async fn get_summary_ai_service_for_user(db: &SqlitePool, user_id: i64) -> Result<AiService> {
    let summary_api_id: Option<i64> =
        sqlx::query_scalar("SELECT summary_api_id FROM user_setting WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(db)
            .await?;

    let api_id = crate::services::api::get_effective_api_id(db, user_id, summary_api_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No summary API configured for user {}", user_id))?;
    let config: ApiConfig = sqlx::query_as("SELECT * FROM api_configs WHERE id = ?")
        .bind(api_id)
        .fetch_one(db)
        .await?;
    let settings: serde_json::Value = serde_json::from_str(&config.settings).unwrap_or_default();
    let model = settings
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-3.5-turbo")
        .to_string();
    let target_lang = settings
        .get("target_lang")
        .and_then(|v| v.as_str())
        .unwrap_or("Chinese")
        .to_string();
    Ok(AiService::new(target_lang, model, config))
}

/// 获取用户默认的 AI 服务配置
pub async fn get_default_ai_service_for_user(db: &SqlitePool, user_id: i64) -> Result<AiService> {
    let api_id = crate::services::api::get_effective_api_id(db, user_id, None)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No default API configured for user {}", user_id))?;

    let config: ApiConfig = sqlx::query_as("SELECT * FROM api_configs WHERE id = ?")
        .bind(api_id)
        .fetch_one(db)
        .await?;
    let settings: serde_json::Value = serde_json::from_str(&config.settings).unwrap_or_default();
    let model = settings
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-3.5-turbo")
        .to_string();
    let target_lang = settings
        .get("target_lang")
        .and_then(|v| v.as_str())
        .unwrap_or("Chinese")
        .to_string();
    Ok(AiService::new(target_lang, model, config))
}


// --- 启动函数 ---

pub fn create_storages(
    pool: SqlitePool,
) -> (
    SqliteStorage<SyncFeedJob>,
    SqliteStorage<TranslateArticleJob>,
    SqliteStorage<SummarizeArticleJob>,
    SqliteStorage<RefreshFeedsJob>,
) {
    (
        SqliteStorage::new(pool.clone()),
        SqliteStorage::new(pool.clone()),
        SqliteStorage::new(pool.clone()),
        SqliteStorage::new(pool),
    )
}

pub async fn start_workers(state: Arc<AppState>) -> anyhow::Result<()> {
    // Set default max_attempts to 1.
    // This hands over retry control to the AiService's internal loop (based on api_configs.retry_count).
    // It prevents nested retries (e.g., 3 internal retries * 25 queue retries).

    let _ = sqlx::query("UPDATE Jobs SET max_attempts = 1 WHERE max_attempts = 25")
        .execute(&state.db)
        .await;

    tracing::info!("🏃 正在清理残留的 Pending 任务...");
    let cleanup_res = sqlx::query(
        r#"
        DELETE FROM Jobs 
        WHERE job_type LIKE '%SyncFeedJob%' 
          AND status = 'Pending'
          AND id NOT IN (
              SELECT MIN(id) 
              FROM Jobs 
              WHERE job_type LIKE '%SyncFeedJob%' 
                AND status = 'Pending'
              GROUP BY job
          )
        "#,
    )
    .execute(&state.db)
    .await;
    
    if let Ok(res) = cleanup_res {
        tracing::info!("🧹 已清理 {} 条重复 Pending 任务", res.rows_affected());
    } else if let Err(e) = cleanup_res {
        tracing::error!("❌ 清理任务失败 (可能数据库已锁定): {:?}", e);
    }

    let state_sync = state.clone();
    let storage_sync = state.sync_queue.clone();
    tokio::spawn(async move {
        Monitor::new()
            .register(
                WorkerBuilder::new("sync-worker")
                    .concurrency(1)
                    .data(state_sync)
                    .backend(storage_sync)
                    .build_fn(sync_feed_handler),
            )
            .run()
            .await
            .expect("Sync monitor failed");
    });

    let state_trans = state.clone();
    let storage_trans = state.translate_queue.clone();
    tokio::spawn(async move {
        Monitor::new()
            .register(
                WorkerBuilder::new("translate-worker")
                    .concurrency(1)
                    .data(state_trans)
                    .backend(storage_trans)
                    .build_fn(translate_article_handler),
            )
            .run()
            .await
            .expect("Translate monitor failed");
    });

    let state_sum = state.clone();
    let storage_sum = state.summarize_queue.clone();
    tokio::spawn(async move {
        Monitor::new()
            .register(
                WorkerBuilder::new("summarize-worker")
                    .concurrency(1)
                    .data(state_sum)
                    .backend(storage_sum)
                    .build_fn(summarize_article_handler),
            )
            .run()
            .await
            .expect("Summarize monitor failed");
    });

    let state_cron = state.clone();
    let schedule = Schedule::from_str("0 * * * * *")?; // 每分钟运行一次

    tokio::spawn(async move {
        Monitor::new()
            .register(
                WorkerBuilder::new("cron-worker")
                    .data(state_cron)
                    .backend(CronStream::new(schedule))
                    .build_fn(refresh_feeds_handler),
            )
            .run()
            .await
            .expect("Cron monitor failed");
    });

    Ok(())
}
