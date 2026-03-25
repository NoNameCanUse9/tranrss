use crate::model::feed::CreateFeedRequest;
use anyhow::Result;
use feed_rs::parser;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use base64::{Engine as _, engine::general_purpose::STANDARD};

pub async fn fetch_feed_preview(url: &str) -> Result<CreateFeedRequest> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
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
    let mut icon_url = feed
        .logo
        .map(|l| l.uri)
        .or_else(|| feed.icon.map(|i| i.uri));

    // 如果 RSS 没提供 icon，尝试从网站主页嗅探
    if icon_url.is_none() {
        if let Some(ref s_url) = site_url {
            if let Some(sniffed) = sniff_icon_from_site(&client, s_url).await {
                icon_url = Some(sniffed);
            }
        }
    }

    let icon_base64 = if let Some(ref url) = icon_url {
        download_icon_base64(&client, url).await
    } else {
        None
    };

    Ok(CreateFeedRequest {
        feed_url: url.to_string(),
        site_url,
        title,
        description,
        icon_url,
        icon_base64,
    })
}

/// 从主机名获取图标 URL (优化版)
async fn sniff_icon_from_site(client: &reqwest::Client, site_url: &str) -> Option<String> {
    let parsed_url = reqwest::Url::parse(site_url).ok()?;
    let domain = parsed_url.host_str()?;

    // 1. 优先使用 Google 的 favicon 服务获取 128x128 的高清图标
    // 这种方式利用主机名直接获取，速度最快且无需爬取对方 HTML
    let google_icon = format!(
        "https://www.google.com/s2/favicons?domain={}&sz=128",
        domain
    );

    if let Ok(resp) = client.head(&google_icon).send().await {
        if resp.status().is_success() {
            return Some(google_icon);
        }
    }

    // 2. 备选方案：尝试主机根目录的 favicon.ico
    let mut base = parsed_url.clone();
    base.set_path("/favicon.ico");
    base.set_query(None);
    let favicon_url = base.to_string();

    if let Ok(resp) = client.head(&favicon_url).send().await {
        if resp.status().is_success() {
            return Some(favicon_url);
        }
    }

    // 3. 保险方案：使用 DuckDuckGo 的图标代理逻辑作为最后兜底
    Some(format!("https://icons.duckduckgo.com/ip3/{}.ico", domain))
}

/// 发起 HTTP 请求下载图标并转为 Base64 data URL
async fn download_icon_base64(client: &reqwest::Client, url: &str) -> Option<String> {
    if let Ok(resp) = client.get(url).send().await {
        if resp.status().is_success() {
            let mime = resp.headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("image/x-icon")
                .to_string();
            
            if let Ok(bytes) = resp.bytes().await {
                // 限制 64KB 以内的图标，防止数据库膨胀
                if bytes.len() > 48 * 1024 {
                    return None;
                }
                
                let encoded = STANDARD.encode(bytes);
                return Some(format!("data:{};base64,{}", mime, encoded));
            }
        }
    }
    None
}

/// 将 HTML 内容转换为简洁的段落级结构
///
/// 流程：HTML → Markdown（中间层）→ 识别段落 → 每段转回 HTML
///
/// 返回 (skeleton, blocks):
/// - skeleton: 简洁的 HTML，段落位置用 [[TEXT_N]] 占位
/// - blocks: HashMap<index, 段落纯 HTML 内容>
fn extract_blocks_from_html(raw_html: &str) -> (String, HashMap<usize, String>) {
    // 1. HTML → Markdown（仅作为中间层，用于识别段落边界和简化结构）
    let markdown = html2md::parse_html(raw_html);

    // 2. 按空行分段
    let paragraphs: Vec<&str> = markdown
        .split("\n\n")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // 3. 构建 HTML 骨架和文本块
    let mut skeleton = String::new();
    let mut blocks = HashMap::new();
    let mut counter = 0;

    for para in &paragraphs {
        // 判断段落类型并提取内容
        let (wrapper_open, wrapper_close, _content) = detect_block_type(para);

        // 如果 detect_block_type 已经识别出整个段落就是一个图片块 ( wrapper_open 为空)
        // 则直接按照原有逻辑处理（写入骨架，不作为翻译块）
        if wrapper_open.is_empty() && wrapper_close.is_empty() {
            skeleton.push_str(para);
            skeleton.push('\n');
            continue;
        }

        // 使用正则提取所有图片并保持其相对于文本的顺序
        let re_img = regex::Regex::new(r"!\[.*?\]\(.*?\)").unwrap();
        let mut last_pos = 0;
        let mut current_para_skeleton = String::new();
        current_para_skeleton.push_str(&wrapper_open);
        
        for mat in re_img.find_iter(para) {
            let start = mat.start();
            let end = mat.end();
            
            // 处理图片前的文本
            let before = &para[last_pos..start];
            if before.chars().filter(|c| !c.is_whitespace()).count() >= 5 {
                let html_content = md_inline_to_html(before.trim());
                blocks.insert(counter, html_content);
                current_para_skeleton.push_str(&format!("[[TEXT_{}]]", counter));
                counter += 1;
            } else if !before.is_empty() {
                // 如果文本太短或是纯空格，转义为 HTML 后直接放入骨架
                current_para_skeleton.push_str(&md_inline_to_html(before));
            }
            
            // 将图片代码直接转为 HTML 并放入骨架（不翻译）
            let img_markdown = &para[start..end];
            let img_html = md_inline_to_html(img_markdown);
            current_para_skeleton.push_str(&img_html);
            
            last_pos = end;
        }
        
        // 处理最后剩下的文本
        let remaining = &para[last_pos..];
        if remaining.chars().filter(|c| !c.is_whitespace()).count() >= 5 {
            let html_content = md_inline_to_html(remaining.trim());
            blocks.insert(counter, html_content);
            current_para_skeleton.push_str(&format!("[[TEXT_{}]]", counter));
            counter += 1;
        } else if !remaining.is_empty() {
            current_para_skeleton.push_str(&md_inline_to_html(remaining));
        }
        
        current_para_skeleton.push_str(&wrapper_close);
        skeleton.push_str(&current_para_skeleton);
        skeleton.push('\n');
    }

    (skeleton.trim_end().to_string(), blocks)
}

/// 从 Markdown 段落语法判断块类型，返回 (开标签, 闭标签, 内容文本)
fn detect_block_type(para: &str) -> (String, String, String) {
    // 标题: # ~ ######
    if let Some(rest) = para.strip_prefix("######") {
        return (
            "<h6>".into(),
            "</h6>".into(),
            rest.trim_matches(|c: char| c == '#' || c == ' ')
                .to_string(),
        );
    }
    if let Some(rest) = para.strip_prefix("#####") {
        return (
            "<h5>".into(),
            "</h5>".into(),
            rest.trim_matches(|c: char| c == '#' || c == ' ')
                .to_string(),
        );
    }
    if let Some(rest) = para.strip_prefix("####") {
        return (
            "<h4>".into(),
            "</h4>".into(),
            rest.trim_matches(|c: char| c == '#' || c == ' ')
                .to_string(),
        );
    }
    if let Some(rest) = para.strip_prefix("###") {
        return (
            "<h3>".into(),
            "</h3>".into(),
            rest.trim_matches(|c: char| c == '#' || c == ' ')
                .to_string(),
        );
    }
    if let Some(rest) = para.strip_prefix("##") {
        return (
            "<h2>".into(),
            "</h2>".into(),
            rest.trim_matches(|c: char| c == '#' || c == ' ')
                .to_string(),
        );
    }
    if let Some(rest) = para.strip_prefix("# ") {
        return (
            "<h1>".into(),
            "</h1>".into(),
            rest.trim_end_matches('#').trim().to_string(),
        );
    }

    // 引用块: >
    if let Some(rest) = para.strip_prefix("> ") {
        return (
            "<blockquote><p>".into(),
            "</p></blockquote>".into(),
            rest.to_string(),
        );
    }

    // 图片: ![alt](src)
    if para.starts_with("![") {
        // 直接用 pulldown-cmark 把整个 Markdown 图片转成 <img> HTML
        let img_html = md_inline_to_html(para);
        return (String::new(), String::new(), img_html);
    }

    // 分隔线: --- / *** / ___
    if para == "---" || para == "***" || para == "___" {
        return (String::new(), String::new(), "<hr>".to_string());
    }

    // 列表项: - / * / 1.
    if para.starts_with("- ") || para.starts_with("* ") {
        let rest = &para[2..];
        return ("<li>".into(), "</li>".into(), rest.to_string());
    }
    if para.len() > 2 && para.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        if let Some(rest) = para.split_once(". ") {
            return ("<li>".into(), "</li>".into(), rest.1.to_string());
        }
    }

    // 默认：普通段落
    ("<p>".into(), "</p>".into(), para.to_string())
}

/// 将 Markdown 内联格式转换为 HTML
/// 处理 **bold**, *italic*, [link](url), `code` 等
fn md_inline_to_html(md: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    let parser = Parser::new_ext(md, Options::all());
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    // pulldown-cmark 会自动包裹 <p>，但我们在骨架中已经有包裹标签，
    // 所以需要去掉最外层的 <p></p>
    let trimmed = html_output.trim();
    let result = if trimmed.starts_with("<p>") && trimmed.ends_with("</p>") {
        trimmed[3..trimmed.len() - 4].to_string()
    } else {
        trimmed.to_string()
    };
    result
}

/// 从指定的 ID 抓取并处理 Feed，更新到数据库
pub async fn fetch_and_process_feed(db: &SqlitePool, user_id: i64, feed_id: i64) -> Result<()> {
    tracing::info!("📡 开始同步 feed ID: {}", feed_id);
    let (feed_url, icon_url, icon_base64): (String, Option<String>, Option<String>) = 
        sqlx::query_as("SELECT feed_url, icon_url, icon_base64 FROM feeds WHERE id = ?")
        .bind(feed_id)
        .fetch_one(db)
        .await?;

    tracing::info!("🌐 正在请求 Feed: {} (ID: {})", feed_url, feed_id);

    // 2. 发起请求
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .build()?;

    let mut success = false;
    let mut last_status = None;
    let mut last_err_msg = None;

    for attempt in 1..=3 {
        match client.get(&feed_url).send().await {
            Ok(response) => {
                let status = response.status();
                last_status = Some(status.as_u16() as i32);

                if !status.is_success() {
                    last_err_msg = Some(format!("HTTP {} - Feed 获取失败 ({})", status.as_u16(), status.canonical_reason().unwrap_or("Unknown")));
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    }
                } else {
                    let xml_content = match response.text().await {
                        Ok(text) => text,
                        Err(e) => {
                            last_err_msg = Some(format!("Error reading response body: {}", e));
                            if attempt < 3 {
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                continue;
                            }
                            break;
                        }
                    };

                    sqlx::query("UPDATE feeds SET last_status_code = ?, last_error = NULL, consecutive_fetch_failures = 0 WHERE id = ?")
                        .bind(status.as_u16() as i32)
                        .bind(feed_id)
                        .execute(db)
                        .await?;

                    // 如果曾被拉黑，现在复活了，则从拉黑名单移除
                    let _ = sqlx::query("DELETE FROM inactive_feeds WHERE user_id = ? AND feed_id = ?")
                        .bind(user_id)
                        .bind(feed_id)
                        .execute(db)
                        .await;

                    tracing::info!(
                        "🚀 抓取成功 (feed_id={}, attempt={})，开始解析 XML ({} bytes)...",
                        feed_id,
                        attempt,
                        xml_content.len()
                    );

                    // 3. 处理并同步到数据库
                    process_xml_content(db, &xml_content, user_id, feed_id).await?;

                    // 4. 如果 icon_base64 为空，尝试同步获取一次
                    if icon_base64.is_none() {
                        if let Some(ref url) = icon_url {
                           if let Some(b64) = download_icon_base64(&client, url).await {
                               let _ = sqlx::query("UPDATE feeds SET icon_base64 = ? WHERE id = ?")
                                   .bind(b64)
                                   .bind(feed_id)
                                   .execute(db)
                                   .await;
                           }
                        }
                    }

                    success = true;
                    break;
                }
            }
            Err(e) => {
                last_err_msg = Some(e.to_string());
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
            }
        }
    }

    if !success {
        // 更新连续失败计数
        sqlx::query(
            "UPDATE feeds SET last_error = ?, last_status_code = ?, consecutive_fetch_failures = consecutive_fetch_failures + 1 WHERE id = ?"
        )
        .bind(last_err_msg.as_ref())
        .bind(last_status)
        .bind(feed_id)
        .execute(db)
        .await?;

        // 判定失效逻辑：如果最后一次成功抓取（或创建时间）超过 2 天，则标记为失效
        let should_disable: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM feeds 
                WHERE id = ? 
                  AND datetime('now') > datetime(IFNULL(last_fetched_at, created_at), '+2 days')
            )
            "#
        )
        .bind(feed_id)
        .fetch_one(db)
        .await?;

        if should_disable.0 {
            let _ = sqlx::query("INSERT OR IGNORE INTO inactive_feeds (user_id, feed_id, reason) VALUES (?, ?, ?)")
                .bind(user_id)
                .bind(feed_id)
                .bind(format!("连续超过2天抓取失败: {}", last_err_msg.as_deref().unwrap_or("未知错误")))
                .execute(db)
                .await;
        }

        return Err(anyhow::anyhow!(last_err_msg.unwrap_or_else(|| "Unknown error".to_string())));
    }

    tracing::info!("feed {} 同步完成", feed_id);
    Ok(())
}

/// 解析后的单篇文章数据（纯同步，Send 安全）
struct ParsedArticle {
    id: i64,
    origin_guid: String,
    title: String,
    link: Option<String>,
    author: Option<String>,
    published_at: Option<i64>,
    skeleton: String,
    blocks: HashMap<usize, String>,
}

/// 同步解析 XML，返回 Send 安全的数据结构
fn parse_feed_entries(xml: &str, _feed_id: i64) -> Result<Vec<ParsedArticle>> {
    let feed = parser::parse(xml.as_bytes())?;
    let mut results = Vec::new();

    for entry in feed.entries {
        let mut origin_guid = entry.id;
        if origin_guid.is_empty() {
            // 如果 GUID 为空，尝试使用链接作为 GUID，再不行就用标题
            origin_guid = entry.links.first().map(|l| l.href.clone())
                .unwrap_or_else(|| entry.title.as_ref().map(|t| t.content.clone()).unwrap_or_default());
        }

        if origin_guid.is_empty() {
            tracing::warn!("(feed_id={}) 文章没有 ID/链接/标题，跳过", _feed_id);
            continue;
        }

        let mut hasher = DefaultHasher::new();
        origin_guid.hash(&mut hasher);
        // 取模 2^53 - 1 确保 ID 在 JavaScript 的安全整数范围内 (Number.MAX_SAFE_INTEGER)
        let id = (hasher.finish() % 9_007_199_254_740_991) as i64;

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

        let (skeleton, blocks) = extract_blocks_from_html(&raw_html);

        results.push(ParsedArticle {
            id,
            origin_guid,
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
    tracing::info!(
        "📦 (feed_id={}) 解析 XML 成功，得到 {} 篇文章",
        feed_id,
        articles.len()
    );

    // 2. 开始事务异步写入数据库
    let mut tx = db.begin().await?;
    tracing::info!("💾 (feed_id={}) 数据库事务开启", feed_id);

    for article in &articles {
        sqlx::query(
            r#"
            INSERT INTO articles (id, original_guid, feed_id, title, link, author, published_at, content_skeleton, crawl_time)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, strftime('%s', 'now'))
            ON CONFLICT(original_guid) DO UPDATE SET
                title = excluded.title,
                link = excluded.link,
                author = excluded.author,
                published_at = excluded.published_at,
                content_skeleton = excluded.content_skeleton,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(article.id)
        .bind(&article.origin_guid)
        .bind(feed_id)
        .bind(&article.title)
        .bind(&article.link)
        .bind(&article.author)
        .bind(article.published_at)
        .bind(&article.skeleton)
        .execute(&mut *tx)
        .await?;

        for (idx, text) in &article.blocks {
            let idx_i32 = *idx as i32;
            sqlx::query(
                r#"
                INSERT INTO article_blocks (user_id, article_id, block_index, raw_text)
                VALUES (?, ?, ?, ?)
                ON CONFLICT(user_id, article_id, block_index) DO UPDATE SET
                    raw_text = excluded.raw_text
                "#,
            )
            .bind(user_id)
            .bind(article.id)
            .bind(idx_i32)
            .bind(text)
            .execute(&mut *tx)
            .await?;
        }
    }

    // 3. 检查 num 限制并清理旧文章
    // 获取当前订阅的 num 限制
    let num_limit: i64 =
        sqlx::query_scalar("SELECT num FROM subscriptions WHERE user_id = ? AND feed_id = ?")
            .bind(user_id)
            .bind(feed_id)
            .fetch_one(&mut *tx)
            .await?;

    // 删除超过 num 限制的旧文章（保留最新的 num 篇）
    // 使用嵌套子查询绕过 SQLite 对 LIMIT 在 IN/NOT IN 中的限制
    sqlx::query(
        r#"
        DELETE FROM articles 
        WHERE feed_id = ? AND id NOT IN (
            SELECT id FROM (
                SELECT id FROM articles 
                WHERE feed_id = ? 
                ORDER BY published_at DESC, id DESC 
                LIMIT ?
            )
        )
        "#,
    )
    .bind(feed_id)
    .bind(feed_id)
    .bind(num_limit)
    .execute(&mut *tx)
    .await?;

    // 4. 更新最后同步时间
    sqlx::query("UPDATE feeds SET last_fetched_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(feed_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    tracing::info!("✅ (feed_id={}) 数据库事务提交成功，同步文章完成", feed_id);
    Ok(())
}
