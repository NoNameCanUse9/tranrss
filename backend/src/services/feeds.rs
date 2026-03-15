use crate::model::feed::CreateFeedRequest;
use anyhow::Result;
use feed_rs::parser;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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

    Ok(CreateFeedRequest {
        feed_url: url.to_string(),
        site_url,
        title,
        description,
        icon_url,
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
        let (wrapper_open, wrapper_close, content) = detect_block_type(para);

        // 图片等非文本元素：直接写入骨架，不作为可翻译 block
        if wrapper_open.is_empty() && wrapper_close.is_empty() {
            skeleton.push_str(content.as_str());
            skeleton.push('\n');
            continue;
        }

        // 过滤过短的噪音
        if content.chars().count() < 10 {
            skeleton.push_str(&format!("{}{}{}\n", wrapper_open, content, wrapper_close));
            continue;
        }

        // 将段落文本内容（Markdown）转回 HTML
        let html_content = md_inline_to_html(&content);

        blocks.insert(counter, html_content);
        skeleton.push_str(&format!(
            "{}[[TEXT_{}]]{}\n",
            wrapper_open, counter, wrapper_close
        ));
        counter += 1;
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
    tracing::debug!("开始同步 feed: {}", feed_id);
    // 1. 获取订阅源 URL
    let feed_rec: (String,) = sqlx::query_as("SELECT feed_url FROM feeds WHERE id = ?")
        .bind(feed_id)
        .fetch_one(db)
        .await?;

    tracing::debug!("正在抓取: {}", feed_rec.0);

    // 2. 发起请求
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .build()?;

    let response_result = client.get(&feed_rec.0).send().await;

    match response_result {
        Ok(response) => {
            let status = response.status();
            sqlx::query("UPDATE feeds SET last_status_code = ? WHERE id = ?")
                .bind(status.as_u16() as i32)
                .bind(feed_id)
                .execute(db)
                .await?;

            if !status.is_success() {
                let err_msg = format!("HTTP Error: {}", status);
                sqlx::query("UPDATE feeds SET last_error = ? WHERE id = ?")
                    .bind(&err_msg)
                    .bind(feed_id)
                    .execute(db)
                    .await?;
                return Err(anyhow::anyhow!(err_msg));
            }

            let xml_content = response.text().await?;
            sqlx::query("UPDATE feeds SET last_error = NULL WHERE id = ?")
                .bind(feed_id)
                .execute(db)
                .await?;

            tracing::debug!(
                "抓取成功 (feed_id={})，开始解析 XML ({} bytes)...",
                feed_id,
                xml_content.len()
            );

            // 3. 处理并同步到数据库
            process_xml_content(db, &xml_content, user_id, feed_id).await?;
        }
        Err(e) => {
            let err_msg = e.to_string();
            sqlx::query("UPDATE feeds SET last_error = ?, last_status_code = NULL WHERE id = ?")
                .bind(&err_msg)
                .bind(feed_id)
                .execute(db)
                .await?;
            return Err(e.into());
        }
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
        let origin_guid = entry.id;

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
    tracing::debug!(
        "(feed_id={}) 解析 XML 成功，得到 {} 篇文章",
        feed_id,
        articles.len()
    );

    // 2. 开始事务异步写入数据库
    let mut tx = db.begin().await?;
    tracing::debug!("(feed_id={}) 数据库事务开启", feed_id);

    for article in &articles {
        sqlx::query(
            r#"
            INSERT INTO articles (id, original_guid, feed_id, title, link, author, published_at, content_skeleton)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
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
    tracing::debug!("(feed_id={}) 数据库事务提交成功", feed_id);
    Ok(())
}
