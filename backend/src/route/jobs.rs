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
    pub status: String,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub run_at: i64,
    pub done_at: Option<i64>,
    pub job_data: serde_json::Value,
    pub title_label: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_jobs))
        .route("/clear_completed", axum::routing::post(clear_completed))
        .route("/{id}/retry", axum::routing::post(retry_job))
}

async fn clear_completed(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, String)> {
    sqlx::query("DELETE FROM Jobs WHERE status = 'Done'")
        .execute(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::OK)
}

async fn retry_job(
    State(state): State<Arc<AppState>>,
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
            .unwrap_or(Some(300))
            .unwrap_or(300);

    // 获取任务日志（直接从数据库读取，不在此逻辑执行清理）

    // 1. 获取最近的任务
    let rows = sqlx::query("SELECT id, job_type, status, attempts, last_error, run_at, done_at, job FROM Jobs ORDER BY run_at DESC LIMIT ?")
        .bind(log_num_limit)
        .fetch_all(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 2. 获取所有的订阅源和文章标题用于映射
    let feed_titles: std::collections::HashMap<i64, String> =
        sqlx::query_as::<_, (i64, String)>("SELECT id, title FROM feeds")
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();

    let article_titles: std::collections::HashMap<i64, String> =
        sqlx::query_as::<_, (i64, String)>("SELECT id, title FROM articles")
            .fetch_all(&state.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();

    let jobs = rows
        .into_iter()
        .map(|row| {
            let job_str: String = row.get("job");
            let job_data: serde_json::Value =
                serde_json::from_str(&job_str).unwrap_or(serde_json::Value::Null);

            let job_type: String = row.get("job_type");
            let mut title_label = None;

            // 根据任务类型解析名称
            if job_type.contains("SyncFeedJob") {
                if let Some(id) = job_data.get("feed_id").and_then(|v| v.as_i64()) {
                    title_label = feed_titles.get(&id).cloned();
                }
            } else if job_type.contains("TranslateArticleJob")
                || job_type.contains("SummarizeArticleJob")
            {
                if let Some(id) = job_data.get("article_id").and_then(|v| v.as_i64()) {
                    title_label = article_titles.get(&id).cloned();
                }
            }

            // 处理 Apalis 序列化的错误消息 ({"Err":"..."} -> "...")
            let mut last_error: Option<String> = row.get("last_error");
            if let Some(err) = last_error.as_ref() {
                if let Ok(json_err) = serde_json::from_str::<serde_json::Value>(err) {
                    if let Some(msg) = json_err.get("Err").and_then(|v| v.as_str()) {
                        last_error = Some(msg.to_string());
                    } else if json_err.get("Ok").is_some() {
                        last_error = None; // 如果是 Ok: null，就不显示错误框
                    }
                }
            }

            JobInfo {
                id: row.get("id"),
                job_type,
                status: row.get("status"),
                attempts: row.get("attempts"),
                last_error,
                run_at: row.get("run_at"),
                done_at: row.get("done_at"),
                job_data,
                title_label,
            }
        })
        .collect();

    Ok(Json(jobs))
}
