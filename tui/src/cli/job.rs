use anyhow::Result;
use clap::Subcommand;

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
    match action {
        JobAction::List { status } => println!("列出任务: status={:?}", status),
        JobAction::Retry { id } => println!("重试任务: {}", id),
        JobAction::Clear => println!("清除已完成任务..."),
    }
    Ok(())
}
