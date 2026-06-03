use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ArticleAction {
    /// 列出文章
    List {
        /// 按订阅 ID 筛选
        #[arg(long)]
        feed: Option<String>,
        /// 只看未读
        #[arg(long)]
        unread: bool,
        /// 只看收藏
        #[arg(long)]
        starred: bool,
    },

    /// 查看文章内容（纯文本）
    Read {
        /// 文章 ID
        id: i64,
    },

    /// 触发翻译
    Translate {
        /// 文章 ID
        id: i64,
    },

    /// 触发摘要
    Summarize {
        /// 文章 ID
        id: i64,
    },

    /// 切换收藏
    Star {
        /// 文章 ID
        id: i64,
    },

    /// 标记已读
    MarkRead {
        /// 文章 ID
        id: i64,
    },
}

pub async fn run(action: ArticleAction) -> Result<()> {
    // TODO: 实现 CLI 逻辑
    match action {
        ArticleAction::List { feed, unread, starred } => {
            println!("列出文章: feed={:?}, unread={}, starred={}", feed, unread, starred);
        }
        ArticleAction::Read { id } => println!("查看文章: {}", id),
        ArticleAction::Translate { id } => println!("翻译文章: {}", id),
        ArticleAction::Summarize { id } => println!("摘要文章: {}", id),
        ArticleAction::Star { id } => println!("收藏文章: {}", id),
        ArticleAction::MarkRead { id } => println!("标记已读: {}", id),
    }
    Ok(())
}
