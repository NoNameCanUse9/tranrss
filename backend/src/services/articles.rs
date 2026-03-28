use crate::model::articles::{ArticleBlock, ArticleDetail, ArticleListItem};
use sqlx::{Error, SqlitePool};

pub async fn list_articles(
    db: &SqlitePool,
    user_id: i64,
    feed_id: Option<i64>,
    is_read: Option<bool>,
    is_starred: Option<bool>,
) -> Result<Vec<ArticleListItem>, Error> {
    let mut query = String::from(
        r#"
        SELECT 
            a.id, COALESCE(b.trans_text, a.title) as title, a.author, a.published_at, a.is_read, a.is_starred,
            a.feed_id, f.title as feed_title
        FROM articles a
        JOIN feeds f ON a.feed_id = f.id
        JOIN subscriptions s ON s.feed_id = f.id
        LEFT JOIN article_blocks b ON b.article_id = a.id AND b.user_id = s.user_id AND b.block_index = -1
        WHERE s.user_id = ?
        "#,
    );

    if let Some(fid) = feed_id {
        query.push_str(&format!(" AND a.feed_id = {}", fid));
    }

    if let Some(read) = is_read {
        query.push_str(&format!(" AND a.is_read = {}", if read { 1 } else { 0 }));
    }

    if let Some(starred) = is_starred {
        query.push_str(&format!(
            " AND a.is_starred = {}",
            if starred { 1 } else { 0 }
        ));
    }

    query.push_str(" ORDER BY a.published_at DESC LIMIT 100");

    sqlx::query_as(&query).bind(user_id).fetch_all(db).await
}

pub async fn mark_starred(
    db: &SqlitePool,
    user_id: i64,
    id: i64,
    starred: bool,
) -> Result<(), Error> {
    sqlx::query(
        r#"
        UPDATE articles 
        SET is_starred = ? 
        WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)
        "#,
    )
    .bind(if starred { 1 } else { 0 })
    .bind(id)
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_article_detail(
    db: &SqlitePool,
    user_id: i64,
    id: i64,
) -> Result<ArticleDetail, Error> {
    sqlx::query_as(
        r#"
        SELECT 
            a.id, COALESCE(b.trans_text, a.title) as title, a.link, a.author, a.published_at, 
            a.content_skeleton, a.is_read, a.is_starred, a.summary,
            s.need_translate
        FROM articles a
        JOIN subscriptions s ON s.feed_id = a.feed_id
        LEFT JOIN article_blocks b ON b.article_id = a.id AND b.user_id = s.user_id AND b.block_index = -1
        WHERE a.id = ? AND s.user_id = ?
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(db)
    .await
}

pub async fn get_article_blocks(
    db: &SqlitePool,
    user_id: i64,
    article_id: i64,
) -> Result<Vec<ArticleBlock>, Error> {
    sqlx::query_as(
        r#"
        SELECT 
            article_id, block_index, raw_text, trans_text
        FROM article_blocks
        WHERE article_id = ? AND user_id = ?
        ORDER BY block_index ASC
        "#,
    )
    .bind(article_id)
    .bind(user_id)
    .fetch_all(db)
    .await
}

pub async fn mark_read(db: &SqlitePool, user_id: i64, id: i64, read: bool) -> Result<(), Error> {
    sqlx::query(
        r#"
        UPDATE articles 
        SET is_read = ? 
        WHERE id = ? AND feed_id IN (SELECT feed_id FROM subscriptions WHERE user_id = ?)
        "#,
    )
    .bind(if read { 1 } else { 0 })
    .bind(id)
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}

pub fn stitch_article_content(
    skeleton: &str,
    blocks: &[ArticleBlock],
    summary: Option<&str>,
    need_translate: bool,
) -> String {
    let mut stitched_content = skeleton.to_string();

    for block in blocks {
        if block.block_index < 0 {
            continue;
        }

        let raw_text = block.raw_text.trim();

        let replacement = if need_translate {
            if let Some(trans) = &block.trans_text {
                let trans_text = trans.trim();

                if raw_text.is_empty() && trans_text.is_empty() {
                    String::new()
                } else if raw_text.is_empty() {
                    format!(
                        "<em style=\"display:block;font-style:italic;opacity:0.8;margin-top:0.25em;padding-left:0.75em;border-left:2px solid #7986CB;\">{}</em>",
                        trans_text
                    )
                } else {
                    format!(
                        "{}<br><em style=\"display:block;font-style:italic;opacity:0.6;font-size:0.95em;margin-top:0.25em;padding-left:0.75em;border-left:2px solid #7986CB;\">{}</em>",
                        raw_text, trans_text
                    )
                }
            } else {
                raw_text.to_string()
            }
        } else {
            raw_text.to_string()
        };

        stitched_content =
            stitched_content.replace(&format!("[[TEXT_{}]]", block.block_index), &replacement);
    }

    // 注入 AI 摘要 (如果存在)
    if let Some(summary_text) = summary {
        if !summary_text.trim().is_empty() {
            let summary_html = format!(
                r#"<div style="background:rgba(34,197,94,0.08);border-left:3px solid #22c55e;padding:1em;margin-bottom:1.5em;border-radius:0 8px 8px 0;line-height:1.7;">
                    <strong style="color:#22c55e;display:block;margin-bottom:0.5em;font-size:1.1em;">AI 摘要</strong>
                    {}
                </div>"#,
                summary_text
            );
            stitched_content = format!("{}{}", summary_html, stitched_content);
        }
    }

    // 如果骨架为空且只有摘要，兜底处理
    if stitched_content.is_empty() && summary.is_some() {
        stitched_content = summary.unwrap_or_default().to_string();
    }

    stitched_content
}
