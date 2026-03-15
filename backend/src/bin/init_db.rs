//! init_db — TranRSS 数据库初始化工具
//!
//! 用法：
//!   cargo run --bin init_db
//!   或者 cargo run --bin init_db -- --db ../mydata.db --user admin --pass secret
//!
//! 功能：
//!   1. 创建所有数据表（幂等，IF NOT EXISTS）
//!   2. 创建默认管理员账号（默认 admin/admin，已存在则跳过）

use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- 解析命令行参数 ---
    let args: Vec<String> = env::args().collect();
    let db_path   = get_arg(&args, "--db",   "../rssdata.db");
    let username  = get_arg(&args, "--user", "admin");
    let password  = get_arg(&args, "--pass", "admin");

    let db_url = format!("sqlite:{}", db_path);
    println!("📂 数据库路径: {}", db_path);

    // --- 连接数据库（自动创建文件） ---
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
    use std::str::FromStr;
    let opts = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(opts).await?;

    // --- 建表 ---
    println!("🔧 正在创建数据表...");
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            username      TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL
        )
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS feeds (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            feed_url        TEXT NOT NULL UNIQUE,
            site_url        TEXT,
            title           TEXT NOT NULL,
            description     TEXT,
            last_fetched_at DATETIME,
            etag            TEXT,
            icon_url        TEXT,
            created_at      DATETIME DEFAULT CURRENT_TIMESTAMP,
            last_status_code INTEGER,
            last_error      TEXT
        )
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS folders (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            title   TEXT NOT NULL,
            user_id INTEGER NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_folders_user_title
        ON folders(user_id, title)
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS subscriptions (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id          INTEGER NOT NULL,
            feed_id          INTEGER NOT NULL,
            folder_id        INTEGER,
            custom_title     TEXT,
            need_translate   BOOLEAN DEFAULT 0,
            need_summary     BOOLEAN DEFAULT 0,
            target_language  TEXT,
            created_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
            NUM              INTEGER DEFAULT 200,
            refresh_interval INTEGER DEFAULT 30,
            FOREIGN KEY (user_id)   REFERENCES users(id)        ON DELETE CASCADE,
            FOREIGN KEY (feed_id)   REFERENCES feeds(id)        ON DELETE CASCADE,
            FOREIGN KEY (folder_id) REFERENCES folders(id)      ON DELETE SET NULL
        )
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id
        ON subscriptions(user_id)
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS api_configs (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            name             TEXT NOT NULL,
            api_type         TEXT NOT NULL,
            api_key          TEXT,
            base_url         TEXT,
            settings         TEXT NOT NULL,
            timeout_seconds  INTEGER DEFAULT 180,
            retry_count      INTEGER DEFAULT 3,
            retry_interval_ms INTEGER DEFAULT 1000,
            retry_enabled    BOOLEAN DEFAULT 1,
            user_id          INTEGER REFERENCES users(id) ON DELETE CASCADE
        )
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS articles (
            id               INTEGER PRIMARY KEY,
            original_guid    TEXT NOT NULL UNIQUE,
            feed_id          INTEGER NOT NULL,
            title            TEXT NOT NULL,
            link             TEXT,
            author           TEXT,
            published_at     INTEGER,
            content_skeleton TEXT,
            is_read          INTEGER DEFAULT 0,
            is_starred       INTEGER DEFAULT 0,
            updated_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
            summary          TEXT,
            FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE
        )
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE INDEX IF NOT EXISTS idx_articles_feed_unread
        ON articles(feed_id, is_read)
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE INDEX IF NOT EXISTS articles_feed_id
        ON articles(feed_id)
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS article_blocks (
            user_id     INTEGER NOT NULL,
            article_id  INTEGER NOT NULL,
            block_index INTEGER NOT NULL,
            raw_text    TEXT NOT NULL,
            trans_text  TEXT,
            PRIMARY KEY (user_id, article_id, block_index),
            FOREIGN KEY (user_id)    REFERENCES users(id)    ON DELETE CASCADE,
            FOREIGN KEY (article_id) REFERENCES articles(id) ON DELETE CASCADE
        )
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE INDEX IF NOT EXISTS idx_article_blocks_lookup
        ON article_blocks(user_id, article_id)
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE INDEX IF NOT EXISTS idx_blocks_article_ordered
        ON article_blocks(article_id, block_index)
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE INDEX IF NOT EXISTS idx_blocks_untranslated
        ON article_blocks(user_id)
        WHERE trans_text IS NULL
    "#).execute(&pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS user_setting (
            id              INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
            user_id         INTEGER NOT NULL UNIQUE,
            translate_api_id INTEGER,
            summary_api_id  INTEGER,
            greader_api     BOOLEAN,
            api_proxy       BOOLEAN,
            api_proxy_url   TEXT,
            app_mode        BOOLEAN DEFAULT 0,
            log_num_limit   INTEGER DEFAULT 300,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
    "#).execute(&pool).await?;

    // Apalis 任务队列表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS Workers (
            id           TEXT NOT NULL UNIQUE,
            worker_type  TEXT NOT NULL,
            storage_name TEXT NOT NULL,
            layers       TEXT,
            last_seen    INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
        )
    "#).execute(&pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS Idx   ON Workers(id)").execute(&pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS WTIdx ON Workers(worker_type)").execute(&pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS LSIdx ON Workers(last_seen)").execute(&pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS Jobs (
            job          TEXT NOT NULL,
            id           TEXT NOT NULL UNIQUE,
            job_type     TEXT NOT NULL,
            status       TEXT NOT NULL DEFAULT 'Pending',
            attempts     INTEGER NOT NULL DEFAULT 0,
            max_attempts INTEGER NOT NULL DEFAULT 25,
            run_at       INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            last_error   TEXT,
            lock_at      INTEGER,
            lock_by      TEXT,
            done_at      INTEGER,
            priority     INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(lock_by) REFERENCES Workers(id)
        )
    "#).execute(&pool).await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS TIdx ON Jobs(id)").execute(&pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS SIdx ON Jobs(status)").execute(&pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS LIdx ON Jobs(lock_by)").execute(&pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS JTIdx ON Jobs(job_type)").execute(&pool).await?;

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS _sqlx_migrations (
            version        BIGINT PRIMARY KEY,
            description    TEXT NOT NULL,
            installed_on   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            success        BOOLEAN NOT NULL,
            checksum       BLOB NOT NULL,
            execution_time BIGINT NOT NULL
        )
    "#).execute(&pool).await?;

    println!("✅ 数据表创建完成");

    // --- 创建管理员用户 ---
    let existing: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = ?")
            .bind(&username)
            .fetch_optional(&pool)
            .await?;

    if existing.is_some() {
        println!("⚠️  用户 '{}' 已存在，跳过创建", username);
    } else {
        println!("👤 正在创建用户 '{}'...", username);
        let hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST)?;
        let user_id: i64 =
            sqlx::query_scalar("INSERT INTO users (username, password_hash) VALUES (?, ?) RETURNING id")
                .bind(&username)
                .bind(&hash)
                .fetch_one(&pool)
                .await?;

        // 同步创建 user_setting 行
        sqlx::query("INSERT INTO user_setting (user_id) VALUES (?)")
            .bind(user_id)
            .execute(&pool)
            .await?;

        println!("✅ 用户 '{}' 创建成功（id={}）", username, user_id);
    }

    println!("\n🎉 初始化完成！");
    println!("   数据库: {}", db_path);
    println!("   用户名: {}", username);
    println!("   密  码: {}", password);
    println!("\n   现在可以运行 `cargo run` 启动服务");

    Ok(())
}

fn get_arg(args: &[String], key: &str, default: &str) -> String {
    args.windows(2)
        .find(|w| w[0] == key)
        .map(|w| w[1].clone())
        .unwrap_or_else(|| default.to_string())
}
