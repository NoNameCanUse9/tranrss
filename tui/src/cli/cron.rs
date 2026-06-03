use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum CronTask {
    /// 同步所有订阅
    Sync,

    /// 翻译待翻译文章
    Translate,

    /// 生成待摘要文章
    Summarize,
}

pub async fn run(task: CronTask) -> Result<()> {
    // TODO: 调用后端 API 执行定时任务
    match task {
        CronTask::Sync => {
            println!("执行订阅同步...");
            // 调用 POST /api/feeds/sync_all
        }
        CronTask::Translate => {
            println!("执行文章翻译...");
        }
        CronTask::Summarize => {
            println!("执行文章摘要...");
        }
    }
    Ok(())
}
