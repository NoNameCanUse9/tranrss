use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum SubAction {
    /// 列出所有订阅
    List,

    /// 添加订阅
    Add {
        /// RSS 订阅地址
        url: String,
        /// 分类
        #[arg(long)]
        category: Option<String>,
        /// 启用自动翻译
        #[arg(long)]
        translate: bool,
        /// 启用自动摘要
        #[arg(long)]
        summary: bool,
    },

    /// 编辑订阅
    Edit {
        /// 订阅 ID
        id: i64,
        /// 自定义标题
        #[arg(long)]
        title: Option<String>,
        /// 分类
        #[arg(long)]
        category: Option<String>,
    },

    /// 删除订阅
    Delete {
        /// 订阅 ID
        id: i64,
    },

    /// 同步订阅（不传 id 则同步全部）
    Sync {
        /// 订阅 ID（可选）
        id: Option<i64>,
    },

    /// 查看失效订阅
    Inactive,
}

pub async fn run(action: SubAction) -> Result<()> {
    // TODO: 实现 CLI 逻辑
    match action {
        SubAction::List => println!("列出订阅..."),
        SubAction::Add { url, .. } => println!("添加订阅: {}", url),
        SubAction::Edit { id, .. } => println!("编辑订阅: {}", id),
        SubAction::Delete { id } => println!("删除订阅: {}", id),
        SubAction::Sync { id } => println!("同步: {:?}", id),
        SubAction::Inactive => println!("失效订阅..."),
    }
    Ok(())
}
