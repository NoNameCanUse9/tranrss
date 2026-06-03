use anyhow::Result;
use clap::Subcommand;

use crate::api_client::ApiClient;
use crate::config::Config;

#[derive(Subcommand)]
pub enum ArticleAction {
    /// 列出文章
    List {
        /// 按订阅 ID 或名称筛选
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
    let config = Config::load()?;
    let client = ApiClient::new(&config)?;

    match action {
        ArticleAction::List { feed, unread, starred } => {
            // 解析 feed 参数（可能是 ID 或名称）
            let feed_id = if let Some(ref f) = feed {
                match f.parse::<i64>() {
                    Ok(id) => Some(id),
                    Err(_) => {
                        // 按名称查找
                        let subs = client.get_subscriptions().await?;
                        subs.iter()
                            .find(|s| s.title.to_lowercase().contains(&f.to_lowercase()))
                            .map(|s| s.feed_id)
                    }
                }
            } else {
                None
            };

            let is_read = if unread { Some(false) } else { None };
            let is_starred = if starred { Some(true) } else { None };

            let articles = client.get_articles(feed_id, is_read, is_starred).await?;

            println!("{:<8} {:<6} {:<40} {:<20} {:<6} {:<4}", "ID", "收藏", "标题", "来源", "已读", "日期");
            println!("{}", "-".repeat(90));
            for a in &articles {
                let date = a.published_at
                    .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                    .map(|d| d.format("%m-%d").to_string())
                    .unwrap_or_default();
                println!("{:<8} {:<6} {:<40} {:<20} {:<6} {:<4}",
                    a.id,
                    if a.is_starred { "★" } else { "" },
                    truncate(&a.title, 38),
                    truncate(a.feed_title.as_deref().unwrap_or(""), 18),
                    if a.is_read { "✓" } else { "●" },
                    date,
                );
            }
            println!("\n共 {} 篇文章", articles.len());
        }
        ArticleAction::Read { id } => {
            let detail = client.get_article_detail(id).await?;
            let text = html2text::from_read(detail.content.as_bytes(), 80)
                .unwrap_or_else(|_| detail.content.clone());
            println!("{}", text);
        }
        ArticleAction::Translate { id } => {
            client.translate_article(id).await?;
            println!("翻译任务已提交: 文章 {}", id);
        }
        ArticleAction::Summarize { id } => {
            client.summarize_article(id).await?;
            println!("摘要任务已提交: 文章 {}", id);
        }
        ArticleAction::Star { id } => {
            // 先获取当前状态再切换
            let articles = client.get_articles(None, None, None).await?;
            if let Some(a) = articles.iter().find(|a| a.id == id) {
                client.mark_starred(id, !a.is_starred).await?;
                println!("文章 {} {}", id, if a.is_starred { "已取消收藏" } else { "已收藏" });
            } else {
                println!("文章 {} 不存在", id);
            }
        }
        ArticleAction::MarkRead { id } => {
            client.mark_read(id, true).await?;
            println!("文章 {} 已标记为已读", id);
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
