use crate::AppState;
use crate::services::auth::AuthUser;
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct JobInfo {
    pub id: String,
    pub job_type: String,
    pub category: String,
    pub status: String,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub run_at: i64,
    pub done_at: Option<i64>,
    pub job_data: serde_json::Value,
    pub title_label: Option<String>,
    pub feed_id: Option<i64>,
    pub feed_title: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_jobs))
        .route("/clear_completed", axum::routing::post(clear_completed))
        .route("/{id}/retry", axum::routing::post(retry_job))
        .route("/trigger_refresh_all", axum::routing::post(trigger_refresh_all))
}

async fn trigger_refresh_all(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
) -> Result<StatusCode, (StatusCode, String)> {
    use crate::services::jobs::RefreshFeedsJob;
    use apalis::prelude::Storage;

    let mut storage = state.refresh_queue.clone();
    storage
        .push(RefreshFeedsJob)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

async fn clear_completed(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
) -> Result<StatusCode, (StatusCode, String)> {
    sqlx::query("DELETE FROM Jobs WHERE status = 'Done'")
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::OK)
}

async fn retry_job(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    sqlx::query("UPDATE Jobs SET status = 'Pending', attempts = 0, last_error = NULL, run_at = (strftime('%s', 'now')) WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::OK)
}

async fn get_jobs(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<JobInfo>>, (StatusCode, String)> {
    // 获取用户设置中定义的日志数量上限
    let log_num_limit: i32 =
        sqlx::query_scalar("SELECT log_num_limit FROM user_setting WHERE user_id = ?")
            .bind(auth.user_id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(300);

    // 2. 获取任务日志
    let rows = sqlx::query("SELECT id, job_type, status, attempts, last_error, run_at, done_at, job FROM Jobs ORDER BY run_at DESC LIMIT ?")
        .bind(log_num_limit)
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 预解析 job 数据以提取关联 ID
    let mut jobs_raw = Vec::new();
    let mut article_ids = std::collections::HashSet::new();
    let mut feed_ids = std::collections::HashSet::new();

    for row in rows {
        let job_str: String = row.get("job");
        let job_data: serde_json::Value = serde_json::from_str(&job_str).unwrap_or(serde_json::Value::Null);
        let job_type: String = row.get("job_type");

        if let Some(aid) = job_data.get("article_id").and_then(|v| v.as_i64()) {
            article_ids.insert(aid);
        }
        if let Some(fid) = job_data.get("feed_id").and_then(|v| v.as_i64()) {
            feed_ids.insert(fid);
        }
        jobs_raw.push((row, job_data, job_type));
    }

    // 3. 批量获取文章和订阅源信息
    let mut article_to_feed = std::collections::HashMap::new();
    let mut article_titles = std::collections::HashMap::new();
    if !article_ids.is_empty() {
        let sql = format!(
            "SELECT id, feed_id, title FROM articles WHERE id IN ({})",
            article_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",")
        );
        let rows = sqlx::query(&sql).fetch_all(&state.db).await.unwrap_or_default();
        for r in rows {
            let aid: i64 = r.get("id");
            let fid: i64 = r.get("feed_id");
            let title: String = r.get("title");
            article_to_feed.insert(aid, fid);
            article_titles.insert(aid, title);
            feed_ids.insert(fid);
        }
    }

    let mut feed_titles = std::collections::HashMap::new();
    if !feed_ids.is_empty() {
        let sql = format!(
            "SELECT id, title FROM feeds WHERE id IN ({})",
            feed_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",")
        );
        let rows = sqlx::query(&sql).fetch_all(&state.db).await.unwrap_or_default();
        for r in rows {
            let fid: i64 = r.get("id");
            let title: String = r.get("title");
            feed_titles.insert(fid, title);
        }
    }

    let jobs = jobs_raw
        .into_iter()
        .map(|(row, job_data, job_type)| {
            let mut title_label = None;
            let mut feed_id = None;
            let mut feed_title = None;

            if let Some(fid) = job_data.get("feed_id").and_then(|v| v.as_i64()) {
                feed_id = Some(fid);
            }

            if let Some(aid) = job_data.get("article_id").and_then(|v| v.as_i64()) {
                title_label = article_titles.get(&aid).cloned();
                if feed_id.is_none() {
                    feed_id = article_to_feed.get(&aid).cloned();
                }
            }

            if let Some(fid) = feed_id {
                feed_title = feed_titles.get(&fid).cloned();
                if job_type.contains("SyncFeedJob") {
                    title_label = feed_title.clone();
                }
            }

            // 如果任务已成功，则隐藏之前的错误消息
            let status: String = row.get("status");
            let mut last_error: Option<String> = if status.to_lowercase() == "done" {
                None
            } else {
                row.get("last_error")
            };

            // 处理 Apalis 序列化的错误消息 (常见于 {"Err": "..."})
            if let Some(err) = last_error.as_ref() {
                if let Ok(json_err) = serde_json::from_str::<serde_json::Value>(err) {
                    if let Some(msg) = json_err.get("Err").and_then(|v| v.as_str()) {
                        last_error = Some(msg.to_string());
                    } else if json_err.get("Ok").is_some() {
                        last_error = None; // 如果是 {"Ok": null} 这种表示法，也清除
                    }
                }
            }

            JobInfo {
                id: row.get("id"),
                job_type: job_type.clone(),
                category: categorize_job_type(&job_type),
                status: row.get("status"),
                attempts: row.get("attempts"),
                last_error,
                run_at: row.get("run_at"),
                done_at: row.get("done_at"),
                job_data,
                title_label,
                feed_id,
                feed_title,
            }
        })
        .collect();

    Ok(Json(jobs))
}

fn categorize_job_type(job_type: &str) -> String {
    if job_type.contains("SyncFeedJob") {
        "sync".to_string()
    } else if job_type.contains("TranslateArticleJob") {
        "translate".to_string()
    } else if job_type.contains("SummarizeArticleJob") {
        "summarize".to_string()
    } else if job_type.contains("RefreshFeedsJob") {
        "system".to_string()
    } else {
        "other".to_string()
    }
}
