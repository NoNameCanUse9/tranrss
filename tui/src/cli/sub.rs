use anyhow::Result;
use clap::Subcommand;

use crate::api_client::ApiClient;
use crate::config::Config;

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
    let config = Config::load()?;
    let client = ApiClient::new(&config)?;

    match action {
        SubAction::List => {
            let subs = client.get_subscriptions().await?;
            println!("{:<6} {:<30} {:<15} {:<8} {:<8}", "ID", "标题", "分类", "翻译", "摘要");
            println!("{}", "-".repeat(75));
            for sub in &subs {
                println!("{:<6} {:<30} {:<15} {:<8} {:<8}",
                    sub.id,
                    truncate(&sub.title, 28),
                    truncate(&sub.category, 13),
                    if sub.need_translate.unwrap_or(false) { "是" } else { "否" },
                    if sub.need_summary.unwrap_or(false) { "是" } else { "否" },
                );
            }
            println!("\n共 {} 个订阅", subs.len());
        }
        SubAction::Add { url, category, translate, summary } => {
            // TODO: 调用 POST /api/feeds
            println!("添加订阅: {}", url);
            if let Some(cat) = category { println!("  分类: {}", cat); }
            println!("  翻译: {}", translate);
            println!("  摘要: {}", summary);
        }
        SubAction::Edit { id, title, category } => {
            // TODO: 调用 PUT /api/feeds/{id}
            println!("编辑订阅: {}", id);
            if let Some(t) = title { println!("  新标题: {}", t); }
            if let Some(c) = category { println!("  新分类: {}", c); }
        }
        SubAction::Delete { id } => {
            // TODO: 调用 DELETE /api/feeds/{id}
            println!("删除订阅: {}", id);
        }
        SubAction::Sync { id } => {
            match id {
                Some(feed_id) => {
                    client.sync_subscription(feed_id).await?;
                    println!("同步任务已提交: 订阅 {}", feed_id);
                }
                None => {
                    client.sync_all().await?;
                    println!("全部同步任务已提交");
                }
            }
        }
        SubAction::Inactive => {
            // TODO: 调用 GET /api/feeds/inactive
            println!("查看失效订阅...");
        }
    }
    Ok(())
}

pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
