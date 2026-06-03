use anyhow::Result;
use clap::Subcommand;

use crate::api_client::ApiClient;
use crate::config::Config;

#[derive(Subcommand)]
pub enum JobAction {
    /// 列出任务
    List {
        /// 按状态筛选
        #[arg(long)]
        status: Option<String>,
    },

    /// 重试任务
    Retry {
        /// 任务 ID
        id: String,
    },

    /// 清除已完成任务
    Clear,
}

pub async fn run(action: JobAction) -> Result<()> {
    let config = Config::load()?;
    let client = ApiClient::new(&config)?;

    match action {
        JobAction::List { status } => {
            let jobs = client.get_jobs().await?;

            let filtered: Vec<_> = if let Some(ref s) = status {
                jobs.iter().filter(|j| j.status.to_lowercase() == s.to_lowercase()).collect()
            } else {
                jobs.iter().collect()
            };

            println!("{:<12} {:<8} {:<30} {:<10}", "ID", "类型", "标题", "状态");
            println!("{}", "-".repeat(65));
            for job in &filtered {
                let title = job.title_label.as_deref().unwrap_or(&job.id);
                let status_icon = match job.status.as_str() {
                    "Running" => "⏳",
                    "Pending" => "🕐",
                    "Done" => "✅",
                    "Failed" => "❌",
                    _ => "❓",
                };
                println!("{:<12} {:<8} {:<30} {} {}",
                    &job.id[..12.min(job.id.len())],
                    job.job_type,
                    truncate(title, 28),
                    status_icon,
                    job.status,
                );
            }
            println!("\n共 {} 个任务", filtered.len());
        }
        JobAction::Retry { id } => {
            client.retry_job(&id).await?;
            println!("重试任务已提交: {}", id);
        }
        JobAction::Clear => {
            client.clear_completed().await?;
            println!("已清除完成的任务");
        }
    }
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:width$}", s, width = max_len)
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
