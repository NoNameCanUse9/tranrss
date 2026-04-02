use crate::model::feed::CreateFeedRequest;
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use feed_rs::parser;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use scraper::{Html, Node};
use ego_tree::NodeRef;

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
            let mime = resp
                .headers()
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
/// 返回 (skeleton, blocks):
/// - skeleton: HTML骨架，段落位置用 [[TEXT_N]] 占位
/// - blocks: HashMap<index, HTML片段内容>
fn extract_blocks_from_html(raw_html: &str) -> (String, HashMap<usize, String>) {
    let fragment = Html::parse_fragment(raw_html);
    let mut skeleton = String::new();
    let mut blocks = HashMap::new();
    let mut counter = 0;
    let mut acc = String::new();

    // 针对片段解析，直接遍历根节点下的所有直接子节点
    for child in fragment.tree.root().children() {
        process_node(child, &mut skeleton, &mut blocks, &mut counter, &mut acc);
    }
    
    flush_acc(&mut acc, &mut skeleton, &mut blocks, &mut counter);
    (skeleton.trim().to_string(), blocks)
}

fn flush_acc(
    acc: &mut String,
    skeleton: &mut String,
    blocks: &mut HashMap<usize, String>,
    counter: &mut usize,
) {
    if acc.is_empty() {
        return;
    }
    let t = acc.trim();
    if !t.is_empty() {
        skeleton.push_str(&format!("[[TEXT_{}]]\n", *counter));
        blocks.insert(*counter, acc.clone());
        *counter += 1;
    } else {
        skeleton.push_str(acc);
    }
    acc.clear();
}

fn open_tag(elem: &scraper::node::Element) -> String {
    let mut attrs_str = String::new();
    for (k, v) in elem.attrs() {
        let escaped_v = v.replace("\"", "&quot;");
        attrs_str.push_str(&format!(" {}=\"{}\"", k, escaped_v));
    }
    format!("<{}{}>", elem.name(), attrs_str)
}

fn process_node<'a>(
    node: NodeRef<'a, Node>,
    skeleton: &mut String,
    blocks: &mut HashMap<usize, String>,
    counter: &mut usize,
    acc: &mut String,
) {
    match node.value() {
        Node::Document | Node::Fragment => {
            for child in node.children() {
                process_node(child, skeleton, blocks, counter, acc);
            }
            flush_acc(acc, skeleton, blocks, counter);
        }
        Node::Text(text) => {
            let escaped = text.text.replace('<', "&lt;").replace('>', "&gt;");
            acc.push_str(&escaped);
        }
        Node::Element(elem) => {
            let tag = elem.name();
            if tag == "script" || tag == "style" || tag == "noscript" || tag == "iframe" || tag == "svg" || tag == "form" {
                return;
            }

            if is_pure_image_element(node) {
                flush_acc(acc, skeleton, blocks, counter);
                serialize_as_is(node, skeleton);
                skeleton.push('\n');
                return;
            }

            let is_block = [
                "p", "div", "h1", "h2", "h3", "h4", "h5", "h6", "ul", "ol", "li", "blockquote",
                "table", "tr", "td", "figure", "header", "footer", "main", "article", "section",
            ]
            .contains(&tag);
            
            let is_void = [
                "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
                "param", "source", "track", "wbr",
            ]
            .contains(&tag);

            if is_block || has_image_descendant(node) || is_void {
                flush_acc(acc, skeleton, blocks, counter);
                skeleton.push_str(&open_tag(elem));
                if !is_void {
                    for child in node.children() {
                        process_node(child, skeleton, blocks, counter, acc);
                    }
                    flush_acc(acc, skeleton, blocks, counter);
                    skeleton.push_str(&format!("</{}>\n", tag));
                }
            } else {
                serialize_as_is(node, acc);
            }
        }
        _ => {}
    }
}

fn is_pure_image_element(node: NodeRef<Node>) -> bool {
    if let Node::Element(elem) = node.value() {
        let tag = elem.name();
        if tag == "img" {
            return true;
        }
        if tag == "figure" {
            return node.children().any(|c| is_pure_image_element(c));
        }
        if tag == "a" {
            let mut has_img = false;
            let mut has_other = false;
            for child in node.children() {
                match child.value() {
                    Node::Element(e) if e.name() == "img" => has_img = true,
                    Node::Text(t) if t.text.trim().is_empty() => continue,
                    _ => has_other = true,
                }
            }
            return has_img && !has_other;
        }
    }
    false
}

fn has_image_descendant(node: NodeRef<Node>) -> bool {
    for child in node.children() {
        if let Node::Element(elem) = child.value() {
            if elem.name() == "img" {
                return true;
            }
        }
        if has_image_descendant(child) {
            return true;
        }
    }
    false
}

fn serialize_as_is(node: NodeRef<Node>, out: &mut String) {
    match node.value() {
        Node::Element(elem) => {
            let tag_name = elem.name();
            out.push_str(&open_tag(elem));
            let is_void = [
                "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
                "param", "source", "track", "wbr",
            ]
            .contains(&tag_name);

            if !is_void {
                for child in node.children() {
                    serialize_as_is(child, out);
                }
                out.push_str(&format!("</{}>", tag_name));
            }
        }
        Node::Text(text) => {
            out.push_str(&text.text.replace('<', "&lt;").replace('>', "&gt;"));
        }
        _ => {}
    }
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
                    last_err_msg = Some(format!(
                        "HTTP {} - Feed 获取失败 ({})",
                        status.as_u16(),
                        status.canonical_reason().unwrap_or("Unknown")
                    ));
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
                    let _ =
                        sqlx::query("DELETE FROM inactive_feeds WHERE user_id = ? AND feed_id = ?")
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
                                let _ =
                                    sqlx::query("UPDATE feeds SET icon_base64 = ? WHERE id = ?")
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
            "#,
        )
        .bind(feed_id)
        .fetch_one(db)
        .await?;

        if should_disable.0 {
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO inactive_feeds (user_id, feed_id, reason) VALUES (?, ?, ?)",
            )
            .bind(user_id)
            .bind(feed_id)
            .bind(format!(
                "连续超过2天抓取失败: {}",
                last_err_msg.as_deref().unwrap_or("未知错误")
            ))
            .execute(db)
            .await;
        }

        return Err(anyhow::anyhow!(
            last_err_msg.unwrap_or_else(|| "Unknown error".to_string())
        ));
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
    // 1. 预处理 XML：修复非标准的 pubDate 格式 (针对 rustcc.cn 等源)
    // 将 <pubDate>2024-03-27 06:40:41</pubDate> 转换为可被解析的标准格式
    let date_fixed_xml = regex::Regex::new(r"<pubDate>(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})</pubDate>")
        .unwrap()
        .replace_all(xml, "<pubDate>$1 +0800</pubDate>");

    let feed = parser::parse(date_fixed_xml.as_bytes())?;
    let mut results = Vec::new();

    for entry in feed.entries {
        let mut origin_guid = entry.id;
        if origin_guid.is_empty() {
            // 如果 GUID 为空，尝试使用链接作为 GUID，再不行就用标题
            origin_guid = entry
                .links
                .first()
                .map(|l| l.href.clone())
                .unwrap_or_else(|| {
                    entry
                        .title
                        .as_ref()
                        .map(|t| t.content.clone())
                        .unwrap_or_default()
                });
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
        
        // 尝试获取作者：优先从 entry.authors 获取
        let author = entry.authors.first().map(|a| a.name.clone());
        
        let raw_html = entry
            .content
            .and_then(|c| c.body)
            .unwrap_or_else(|| entry.summary.map(|s| s.content).unwrap_or_default());

        let published_at = entry.published.or(entry.updated).map(|d| d.timestamp());

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
            VALUES (?, ?, ?, ?, ?, ?, COALESCE(?, strftime('%s', 'now')), ?, strftime('%s', 'now'))
            ON CONFLICT(original_guid) DO UPDATE SET
                title = excluded.title,
                link = excluded.link,
                author = excluded.author,
                published_at = COALESCE(excluded.published_at, articles.published_at),
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

        // 写入正文块
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

        // --- 新增：写入标题块 (index = -1) 以供翻译引擎识别 ---
        sqlx::query(
            r#"
            INSERT INTO article_blocks (user_id, article_id, block_index, raw_text)
            VALUES (?, ?, -1, ?)
            ON CONFLICT(user_id, article_id, block_index) DO UPDATE SET
                raw_text = excluded.raw_text
            "#,
        )
        .bind(user_id)
        .bind(article.id)
        .bind(&article.title)
        .execute(&mut *tx)
        .await?;
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
