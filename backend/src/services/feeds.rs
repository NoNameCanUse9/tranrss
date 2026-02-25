use crate::model::feed::{Article, ArticleBlock, CreateFeedRequest};
use anyhow::Result;
use feed_rs::parser;
use scraper::{ElementRef, Html, Node};
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn fetch_feed_preview(url: &str) -> Result<CreateFeedRequest> {
    let client = reqwest::Client::builder()
        .user_agent("TranRSS/0.1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(url).send().await?;
    let content = response.bytes().await?;

    let feed = parser::parse(&content[..])?;

    let title = feed
        .title
        .map(|t| t.content)
        .unwrap_or_else(|| "Unknown Feed".to_string());
    let description = feed.description.map(|d| d.content);
    let site_url = feed.links.first().map(|l| l.href.clone());
    let icon_url = feed
        .logo
        .map(|l| l.uri)
        .or_else(|| feed.icon.map(|i| i.uri));

    Ok(CreateFeedRequest {
        feed_url: url.to_string(),
        site_url,
        title,
        description,
        icon_url,
    })
}

/// 深度遍历 DOM 树，保留标签，仅提取文本节点并产生占位符
fn walk(node: ElementRef, sk: &mut String, tm: &mut HashMap<usize, String>, c: &mut usize) {
    for child in node.children() {
        match child.value() {
            Node::Text(text) => {
                let t = text.to_string();
                if !t.trim().is_empty() {
                    tm.insert(*c, t);
                    sk.push_str(&format!("[[TEXT_{}]]", *c));
                    *c += 1;
                } else {
                    sk.push_str(&t);
                }
            }
            Node::Element(elem) => {
                let tag = elem.name();
                let attrs = elem
                    .attrs()
                    .map(|(k, v)| format!(" {}=\"{}\"", k, v))
                    .collect::<String>();

                sk.push_str(&format!("<{}{}>", tag, attrs));
                if let Some(er) = ElementRef::wrap(child) {
                    walk(er, sk, tm, c);
                }
                // 处理非自闭合标签的闭合
                if tag != "img" && tag != "br" && tag != "hr" && tag != "link" && tag != "meta" {
                    sk.push_str(&format!("</{}>", tag));
                }
            }
            _ => {}
        }
    }
}

/// 从指定的 ID 抓取并处理 Feed，更新到数据库
pub async fn fetch_and_process_feed(db: &SqlitePool, user_id: i64, feed_id: i64) -> Result<()> {
    // 1. 获取订阅源 URL
    let feed_rec: (String,) = sqlx::query_as("SELECT feed_url FROM feeds WHERE id = ?")
        .bind(feed_id)
        .fetch_one(db)
        .await?;

    // 2. 发起请求
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .build()?;

    let response = client.get(&feed_rec.0).send().await?;
    let xml_content = response.text().await?;

    // 3. 处理并同步到数据库
    process_xml_content(db, &xml_content, user_id, feed_id).await
}

/// 解析后的单篇文章数据（纯同步，Send 安全）
struct ParsedArticle {
    guid: String,
    title: String,
    link: Option<String>,
    author: Option<String>,
    published_at: Option<i64>,
    skeleton: String,
    blocks: HashMap<usize, String>,
}

/// 同步解析 XML，返回 Send 安全的数据结构
fn parse_feed_entries(xml: &str, feed_id: i64) -> Result<Vec<ParsedArticle>> {
    let feed = parser::parse(xml.as_bytes())?;
    let mut results = Vec::new();

    for entry in feed.entries {
        let guid = entry.id;
        let title = entry
            .title
            .map(|t| t.content)
            .unwrap_or_else(|| "No Title".to_string());
        let link = entry.links.first().map(|l| l.href.clone());
        let author = entry.authors.first().map(|a| a.name.clone());
        let published_at = entry.published.map(|d| d.timestamp());

        let raw_html = entry
            .content
            .and_then(|c| c.body)
            .unwrap_or_else(|| entry.summary.map(|s| s.content).unwrap_or_default());

        let fragment = Html::parse_fragment(&raw_html);
        let mut skeleton = String::new();
        let mut blocks = HashMap::new();
        let mut counter = 0;

        walk(
            fragment.root_element(),
            &mut skeleton,
            &mut blocks,
            &mut counter,
        );
        // fragment (Html) is dropped here — no longer held across await

        results.push(ParsedArticle {
            guid,
            title,
            link,
            author,
            published_at,
            skeleton,
            blocks,
        });
    }

    Ok(results)
}

/// 将解析后的 XML 字符串转换为 Article 骨架和翻译任务包，并同步到数据库
pub async fn process_xml_content(
    db: &SqlitePool,
    xml: &str,
    user_id: i64,
    feed_id: i64,
) -> Result<()> {
    // 1. 同步解析（不涉及 await，scraper::Html 不会跨越 await 边界）
    let articles = parse_feed_entries(xml, feed_id)?;

    // 2. 异步写入数据库（全部是 Send 安全的类型）
    for article in &articles {
        sqlx::query(
            r#"
            INSERT INTO articles (guid, feed_id, title, link, author, published_at, content_skeleton)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(guid) DO UPDATE SET
                title = excluded.title,
                link = excluded.link,
                author = excluded.author,
                published_at = excluded.published_at,
                content_skeleton = excluded.content_skeleton,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&article.guid)
        .bind(feed_id)
        .bind(&article.title)
        .bind(&article.link)
        .bind(&article.author)
        .bind(article.published_at)
        .bind(&article.skeleton)
        .execute(db)
        .await?;

        for (idx, text) in &article.blocks {
            let idx_i32 = *idx as i32;
            sqlx::query(
                r#"
                INSERT INTO article_blocks (user_id, article_guid, block_index, raw_text)
                VALUES (?, ?, ?, ?)
                ON CONFLICT(user_id, article_guid, block_index) DO UPDATE SET
                    raw_text = excluded.raw_text
                "#,
            )
            .bind(user_id)
            .bind(&article.guid)
            .bind(idx_i32)
            .bind(text)
            .execute(db)
            .await?;
        }
    }

    // 3. 更新最后同步时间
    sqlx::query("UPDATE feeds SET last_fetched_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(feed_id)
        .execute(db)
        .await?;

    Ok(())
}
