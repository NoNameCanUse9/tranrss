use crate::model::subscriptions::{
    CreateSubscriptionRequest, SubscriptionDetail, UpdateSubscriptionRequest,
};
use sqlx::{Error, SqlitePool};

pub async fn list_subscriptions(
    db: &SqlitePool,
    user_id: i64,
) -> Result<Vec<SubscriptionDetail>, Error> {
    let subs = sqlx::query_as::<_, SubscriptionDetail>(
        r#"
        SELECT 
            s.id,
            COALESCE(s.custom_title, f.title) as title,
            f.feed_url as url,
            COALESCE(fo.title, '未分类') as category,
            (SELECT COUNT(*) FROM articles WHERE feed_id = f.id) as article_count,
            f.last_fetched_at as last_sync,
            'active' as status,
            'en' as language,
            s.need_translate as auto_translate,
            s.need_summary,
            f.site_url,
            f.description,
            f.icon_url
        FROM subscriptions s
        JOIN feeds f ON s.feed_id = f.id
        LEFT JOIN folders fo ON s.folder_id = fo.id
        WHERE s.user_id = ?
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(subs)
}

pub async fn create_subscription(
    db: &SqlitePool,
    user_id: i64,
    payload: CreateSubscriptionRequest,
) -> Result<i64, anyhow::Error> {
    // 1. Find or create feed
    let feed_id: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO feeds (feed_url, site_url, title, description, icon_url) 
        VALUES (?, ?, ?, ?, ?) 
        ON CONFLICT(feed_url) DO UPDATE SET 
            site_url = COALESCE(excluded.site_url, feeds.site_url),
            title = COALESCE(excluded.title, feeds.title),
            description = COALESCE(excluded.description, feeds.description),
            icon_url = COALESCE(excluded.icon_url, feeds.icon_url)
        RETURNING id
        "#,
    )
    .bind(&payload.feed_url)
    .bind(&payload.site_url)
    .bind(
        payload
            .custom_title
            .clone()
            .unwrap_or_else(|| "New Feed".to_string()),
    )
    .bind(&payload.description)
    .bind(&payload.icon_url)
    .fetch_one(db)
    .await?;

    // 2. Create subscription
    let result = sqlx::query("INSERT INTO subscriptions (user_id, feed_id, folder_id, custom_title, need_translate, need_summary) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(user_id)
        .bind(feed_id.0)
        .bind(payload.folder_id)
        .bind(&payload.custom_title)
        .bind(payload.need_translate.unwrap_or(false))
        .bind(payload.need_summary.unwrap_or(false))
        .execute(db)
        .await?;

    Ok(result.last_insert_rowid())
}

pub async fn update_subscription(
    db: &SqlitePool,
    user_id: i64,
    id: i64,
    payload: UpdateSubscriptionRequest,
) -> Result<(), Error> {
    sqlx::query(
        r#"
        UPDATE subscriptions 
        SET folder_id = COALESCE(?, folder_id),
            custom_title = COALESCE(?, custom_title),
            need_translate = COALESCE(?, need_translate),
            need_summary = COALESCE(?, need_summary),
            target_language = COALESCE(?, target_language)
        WHERE id = ? AND user_id = ?
        "#,
    )
    .bind(payload.folder_id)
    .bind(payload.custom_title)
    .bind(payload.need_translate)
    .bind(payload.need_summary)
    .bind(payload.target_language)
    .bind(id)
    .bind(user_id)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn delete_subscription(db: &SqlitePool, user_id: i64, id: i64) -> Result<(), Error> {
    // 1. 查出关联的 feed_id 和 folder_id
    let rec: (i64, Option<i64>) =
        sqlx::query_as("SELECT feed_id, folder_id FROM subscriptions WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_one(db)
            .await?;
    let feed_id = rec.0;
    let folder_id = rec.1;

    // 2. 删除该用户在此 feed 下所有文章的翻译块
    sqlx::query(
        "DELETE FROM article_blocks WHERE user_id = ? AND article_guid IN (SELECT guid FROM articles WHERE feed_id = ?)"
    )
    .bind(user_id)
    .bind(feed_id)
    .execute(db)
    .await?;

    // 3. 删除订阅记录
    sqlx::query("DELETE FROM subscriptions WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(db)
        .await?;

    // 4. 如果该 feed 没有其他订阅了，删除 feed（ON DELETE CASCADE 会自动清理 articles）
    let remaining: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE feed_id = ?")
        .bind(feed_id)
        .fetch_one(db)
        .await?;

    if remaining.0 == 0 {
        sqlx::query("DELETE FROM feeds WHERE id = ?")
            .bind(feed_id)
            .execute(db)
            .await?;
    }

    // 5. 如果原来有分类文件夹，且已无其他订阅使用它，也一并删除
    if let Some(fid) = folder_id {
        let folder_remaining: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM subscriptions WHERE folder_id = ?")
                .bind(fid)
                .fetch_one(db)
                .await?;

        if folder_remaining.0 == 0 {
            sqlx::query("DELETE FROM folders WHERE id = ?")
                .bind(fid)
                .execute(db)
                .await?;
        }
    }

    Ok(())
}

pub async fn get_subscription_detail(
    db: &SqlitePool,
    user_id: i64,
    id: i64,
) -> Result<SubscriptionDetail, Error> {
    let sub = sqlx::query_as::<_, SubscriptionDetail>(
        r#"
        SELECT 
            s.id,
            COALESCE(s.custom_title, f.title) as title,
            f.feed_url as url,
            COALESCE(fo.title, '未分类') as category,
            (SELECT COUNT(*) FROM articles WHERE feed_id = f.id) as article_count,
            f.last_fetched_at as last_sync,
            'active' as status,
            'en' as language,
            s.need_translate as auto_translate,
            s.need_summary,
            f.site_url,
            f.description,
            f.icon_url
        FROM subscriptions s
        JOIN feeds f ON s.feed_id = f.id
        LEFT JOIN folders fo ON s.folder_id = fo.id
        WHERE s.id = ? AND s.user_id = ?
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_one(db)
    .await?;

    Ok(sub)
}
pub async fn get_feed_id_by_subscription(
    db: &SqlitePool,
    user_id: i64,
    id: i64,
) -> Result<i64, Error> {
    let rec: (i64,) =
        sqlx::query_as("SELECT feed_id FROM subscriptions WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_one(db)
            .await?;
    Ok(rec.0)
}
