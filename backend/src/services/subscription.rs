use crate::model::subscriptions::{
    CreateSubscriptionRequest, SubscriptionDetail, UpdateSubscriptionRequest,
};
use crate::services::jobs::SyncFeedJob;
use apalis::prelude::Storage;
use sqlx::{Error, SqlitePool};

pub async fn list_subscriptions(
    db: &SqlitePool,
    user_id: i64,
) -> Result<Vec<SubscriptionDetail>, Error> {
    let subs = sqlx::query_as::<_, SubscriptionDetail>(
        r#"
        SELECT 
            s.id,
            s.feed_id,
            COALESCE(s.custom_title, f.title) as title,
            f.feed_url as url,
            COALESCE(fo.title, '未分类') as category,
            COALESCE(ac.total, 0) as article_count,
            COALESCE(ac.unread, 0) as unread_count,
            COALESCE(ac.starred, 0) as starred_count,
            f.last_fetched_at as last_sync,
            'active' as status,
            s.target_language,
            COALESCE(s.target_language, 'en') as language,
            s.need_translate as auto_translate,
            s.need_summary,
            s.is_shared,
            f.site_url,
            f.description,
            f.icon_url,
            f.icon_base64,
            s.refresh_interval,
            f.last_status_code,
            f.last_error
        FROM subscriptions s
        JOIN feeds f ON s.feed_id = f.id
        LEFT JOIN folders fo ON s.folder_id = fo.id
        LEFT JOIN (
            SELECT feed_id,
                   COUNT(*) as total,
                   SUM(CASE WHEN is_read = 0 THEN 1 ELSE 0 END) as unread,
                   SUM(CASE WHEN is_starred = 1 THEN 1 ELSE 0 END) as starred
            FROM articles
            GROUP BY feed_id
        ) ac ON ac.feed_id = f.id
        WHERE s.user_id = ?
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(subs)
}

pub async fn trigger_stale_syncs(
    user_id: i64,
    state: crate::AppState, // 传入 AppState 以便获取数据库和队列
) -> Result<(), Error> {
    let db = &state.db;
    let overdue_feeds = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT f.id 
        FROM feeds f
        JOIN subscriptions s ON f.id = s.feed_id
        WHERE s.user_id = ?
          AND NOT EXISTS (
            SELECT 1 FROM inactive_feeds inf 
            WHERE inf.user_id = s.user_id AND inf.feed_id = f.id
          )
          AND NOT EXISTS (
            SELECT 1 FROM Jobs j 
            WHERE j.job_type LIKE '%SyncFeedJob%' 
              AND json_extract(j.job, '$.feed_id') = f.id
              AND j.status IN ('Pending', 'Running')
          )
        GROUP BY f.id
        HAVING f.last_fetched_at IS NULL OR 
               datetime(f.last_fetched_at, '+' || s.refresh_interval || ' minutes') < datetime('now')
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    let mut storage = state.sync_queue.clone();
    for fid in overdue_feeds {
        let _ = storage
            .push(SyncFeedJob {
                feed_id: fid,
                initiator_user_id: Some(user_id),
            })
            .await;
    }

    Ok(())
}

pub async fn create_subscription(
    db: &SqlitePool,
    user_id: i64,
    payload: CreateSubscriptionRequest,
) -> Result<(i64, i64), anyhow::Error> {
    // 1. Find or create feed
    let feed_id: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO feeds (feed_url, site_url, title, description, icon_url, icon_base64) 
        VALUES (?, ?, ?, ?, ?, ?) 
        ON CONFLICT(feed_url) DO UPDATE SET 
            site_url = COALESCE(excluded.site_url, feeds.site_url),
            title = COALESCE(excluded.title, feeds.title),
            description = COALESCE(excluded.description, feeds.description),
            icon_url = COALESCE(excluded.icon_url, feeds.icon_url),
            icon_base64 = COALESCE(excluded.icon_base64, feeds.icon_base64)
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
    .bind(&payload.icon_base64)
    .fetch_one(db)
    .await?;

    // 2. Resolve folder_id from category if folder_id is not provided
    let mut resolved_folder_id = payload.folder_id;
    if resolved_folder_id.is_none() {
        if let Some(cat) = payload.category.as_ref() {
            if !cat.is_empty() && cat != "未分类" {
                // Find or create folder
                let folder_rec: (i64,) = sqlx::query_as(
                    "INSERT INTO folders (user_id, title) VALUES (?, ?) ON CONFLICT(user_id, title) DO UPDATE SET title=excluded.title RETURNING id"
                )
                .bind(user_id)
                .bind(&cat)
                .fetch_one(db)
                .await?;
                resolved_folder_id = Some(folder_rec.0);
            }
        }
    }

    // 3. Create subscription
    let result = sqlx::query("INSERT INTO subscriptions (user_id, feed_id, folder_id, custom_title, need_translate, need_summary, target_language, num, refresh_interval, is_shared) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(user_id)
        .bind(feed_id.0)
        .bind(resolved_folder_id)
        .bind(&payload.custom_title)
        .bind(payload.need_translate.unwrap_or(false))
        .bind(payload.need_summary.unwrap_or(false))
        .bind(payload.target_language.as_ref().unwrap_or(&"Chinese".to_string()))
        .bind(payload.num.unwrap_or(200))
        .bind(payload.refresh_interval.unwrap_or(30))
        .bind(payload.is_shared.unwrap_or(false))
        .execute(db)
        .await?;

    Ok((result.last_insert_rowid(), feed_id.0))
}

pub async fn update_subscription(
    db: &SqlitePool,
    user_id: i64,
    id: i64,
    payload: UpdateSubscriptionRequest,
) -> Result<(), anyhow::Error> {
    // 1. Resolve folder_id if needed
    let mut resolved_folder_id = payload.folder_id;
    if let Some(cat) = payload.category.as_ref() {
        if !cat.is_empty() && cat != "未分类" {
            let folder_rec: (i64,) = sqlx::query_as(
                "INSERT INTO folders (user_id, title) VALUES (?, ?) ON CONFLICT(user_id, title) DO UPDATE SET title=excluded.title RETURNING id"
            )
            .bind(user_id)
            .bind(&cat)
            .fetch_one(db)
            .await?;
            resolved_folder_id = Some(folder_rec.0);
        } else {
            resolved_folder_id = None;
        }
    }

    // 2. Perform update
    // We use a trick with COALESCE and a flag or just handle folder_id explicitly.
    // Since SubscriptionView always sends category, we can trust it if it's there.
    let (set_folder, target_folder_id) =
        if payload.category.is_some() || payload.folder_id.is_some() {
            (true, resolved_folder_id)
        } else {
            (false, None)
        };

    sqlx::query(
        r#"
        UPDATE subscriptions 
        SET folder_id = CASE WHEN ? THEN ? ELSE folder_id END,
            custom_title = COALESCE(?, custom_title),
            need_translate = COALESCE(?, need_translate),
            need_summary = COALESCE(?, need_summary),
            target_language = COALESCE(?, target_language),
            num = COALESCE(?, num),
            refresh_interval = COALESCE(?, refresh_interval),
            is_shared = COALESCE(?, is_shared)
        WHERE id = ? AND user_id = ?
        "#,
    )
    .bind(set_folder)
    .bind(target_folder_id)
    .bind(payload.custom_title)
    .bind(payload.need_translate)
    .bind(payload.need_summary)
    .bind(payload.target_language)
    .bind(payload.num)
    .bind(payload.refresh_interval)
    .bind(payload.is_shared)
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
        "DELETE FROM article_blocks WHERE user_id = ? AND article_id IN (SELECT id FROM articles WHERE feed_id = ?)"
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
            s.feed_id,
            COALESCE(s.custom_title, f.title) as title,
            f.feed_url as url,
            COALESCE(fo.title, '未分类') as category,
            (SELECT COUNT(*) FROM articles WHERE feed_id = f.id) as article_count,
            (SELECT COUNT(*) FROM articles WHERE feed_id = f.id AND is_read = 0) as unread_count,
            (SELECT COUNT(*) FROM articles WHERE feed_id = f.id AND is_starred = 1) as starred_count,
            f.last_fetched_at as last_sync,
            'active' as status,
            s.target_language,
            COALESCE(s.target_language, 'en') as language,
            s.need_translate as auto_translate,
            s.need_summary,
            s.is_shared,
            f.site_url,
            f.description,
            f.icon_url,
            f.icon_base64,
            s.refresh_interval,
            f.last_status_code,
            f.last_error
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

pub async fn list_user_feed_ids(db: &SqlitePool, user_id: i64) -> Result<Vec<i64>, Error> {
    let recs: Vec<(i64,)> = sqlx::query_as("SELECT feed_id FROM subscriptions WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(db)
        .await?;
    Ok(recs.into_iter().map(|r| r.0).collect())
}
