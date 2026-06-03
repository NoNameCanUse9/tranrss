use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ApiAction {
    /// 列出 API 配置
    List,

    /// 添加 API 配置
    Add {
        /// 配置名称
        name: String,
        /// 类型 (openai/deeplx)
        #[arg(long)]
        r#type: String,
        /// API 端点
        #[arg(long)]
        url: String,
        /// API Key
        #[arg(long)]
        key: String,
    },

    /// 删除 API 配置
    Delete {
        /// 配置 ID
        id: i64,
    },
}

pub async fn run(action: ApiAction) -> Result<()> {
    match action {
        ApiAction::List => println!("列出 API 配置..."),
        ApiAction::Add { name, .. } => println!("添加 API: {}", name),
        ApiAction::Delete { id } => println!("删除 API: {}", id),
    }
    Ok(())
}
