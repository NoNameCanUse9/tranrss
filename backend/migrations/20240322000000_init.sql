-- users 表
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL
);

-- feeds 表
CREATE TABLE IF NOT EXISTS feeds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    feed_url TEXT NOT NULL UNIQUE,
    site_url TEXT,
    title TEXT NOT NULL,
    description TEXT,
    last_fetched_at DATETIME,
    etag TEXT,
    icon_url TEXT,
    icon_base64 TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_status_code INTEGER,
    last_error TEXT,
    consecutive_fetch_failures INTEGER DEFAULT 0
);

-- folders 表
CREATE TABLE IF NOT EXISTS folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_folders_user_title ON folders(user_id, title);

-- subscriptions 表
CREATE TABLE IF NOT EXISTS subscriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    feed_id INTEGER NOT NULL,
    folder_id INTEGER,
    custom_title TEXT,
    need_translate BOOLEAN DEFAULT 0,
    need_summary BOOLEAN DEFAULT 0,
    target_language TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    num INTEGER DEFAULT 200,
    refresh_interval INTEGER DEFAULT 30,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE,
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions(user_id);

-- api_configs 表
CREATE TABLE IF NOT EXISTS api_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    api_type TEXT NOT NULL,
    api_key TEXT,
    base_url TEXT,
    settings TEXT NOT NULL,
    timeout_seconds INTEGER DEFAULT 180,
    retry_count INTEGER DEFAULT 3,
    retry_interval_ms INTEGER DEFAULT 1000,
    retry_enabled BOOLEAN DEFAULT 1,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE
);

-- articles 表
CREATE TABLE IF NOT EXISTS articles (
    id INTEGER PRIMARY KEY,
    original_guid TEXT NOT NULL UNIQUE,
    feed_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    link TEXT,
    author TEXT,
    published_at INTEGER,
    content_skeleton TEXT,
    is_read INTEGER DEFAULT 0,
    is_starred INTEGER DEFAULT 0,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    summary TEXT,
    crawl_time INTEGER,
    FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_articles_feed_unread ON articles(feed_id, is_read);
CREATE INDEX IF NOT EXISTS articles_feed_id ON articles(feed_id);

-- article_blocks 表
CREATE TABLE IF NOT EXISTS article_blocks (
    user_id INTEGER NOT NULL,
    article_id INTEGER NOT NULL,
    block_index INTEGER NOT NULL,
    raw_text TEXT NOT NULL,
    trans_text TEXT,
    PRIMARY KEY (user_id, article_id, block_index),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (article_id) REFERENCES articles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_article_blocks_lookup ON article_blocks(user_id, article_id);
CREATE INDEX IF NOT EXISTS idx_blocks_article_ordered ON article_blocks(article_id, block_index);
CREATE INDEX IF NOT EXISTS idx_blocks_untranslated ON article_blocks(user_id) WHERE trans_text IS NULL;

-- user_setting 表
CREATE TABLE IF NOT EXISTS user_setting (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL UNIQUE,
    translate_api_id INTEGER,
    summary_api_id INTEGER,
    default_api_id INTEGER,
    greader_api BOOLEAN,
    api_proxy BOOLEAN,
    api_proxy_url TEXT,
    app_mode BOOLEAN DEFAULT 0,
    log_num_limit INTEGER DEFAULT 300,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- inactive_feeds 表
CREATE TABLE IF NOT EXISTS inactive_feeds (
    user_id INTEGER NOT NULL,
    feed_id INTEGER NOT NULL,
    reason TEXT,
    disabled_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, feed_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE
);

-- Apalis SQL Jobs table (frame-default name in 0.7.4)
-- Apalis Workers table (required by apalis-sql 0.7.4)
CREATE TABLE IF NOT EXISTS Workers (
    id TEXT NOT NULL UNIQUE,
    worker_type TEXT NOT NULL,
    storage_name TEXT NOT NULL,
    layers TEXT,
    last_seen INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
CREATE INDEX IF NOT EXISTS Idx   ON Workers(id);
CREATE INDEX IF NOT EXISTS WTIdx ON Workers(worker_type);
CREATE INDEX IF NOT EXISTS LSIdx ON Workers(last_seen);

-- Apalis Jobs table (schema must match apalis-sql 0.7.4 exactly)
CREATE TABLE IF NOT EXISTS Jobs (
    job TEXT NOT NULL,
    id TEXT NOT NULL UNIQUE,
    job_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 25,
    run_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_error TEXT,
    lock_at INTEGER,
    lock_by TEXT,
    done_at INTEGER,
    priority INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(lock_by) REFERENCES Workers(id)
);
CREATE INDEX IF NOT EXISTS TIdx  ON Jobs(id);
CREATE INDEX IF NOT EXISTS SIdx  ON Jobs(status);
CREATE INDEX IF NOT EXISTS LIdx  ON Jobs(lock_by);
CREATE INDEX IF NOT EXISTS JTIdx ON Jobs(job_type);
