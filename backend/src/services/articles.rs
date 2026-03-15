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
            a.id, a.title, a.author, a.published_at, a.is_read, a.is_starred,
            a.feed_id, f.title as feed_title
        FROM articles a
        JOIN feeds f ON a.feed_id = f.id
        JOIN subscriptions s ON s.feed_id = f.id
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
            a.id, a.title, a.link, a.author, a.published_at, 
            a.content_skeleton, a.is_read, a.is_starred, a.summary,
            s.need_translate
        FROM articles a
        JOIN subscriptions s ON s.feed_id = a.feed_id
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
