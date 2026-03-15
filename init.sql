-- TranRSS Database Initialization Script

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL
);

-- Feeds table
CREATE TABLE IF NOT EXISTS feeds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    feed_url TEXT NOT NULL UNIQUE,
    site_url TEXT,
    title TEXT NOT NULL,
    description TEXT,
    last_fetched_at DATETIME,
    etag TEXT,
    icon_url TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Folders table
CREATE TABLE IF NOT EXISTS folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL, -- The query in subscription.rs uses fo.title
    user_id INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Subscriptions table
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
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE,
    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE SET NULL
);

-- Articles table
CREATE TABLE IF NOT EXISTS articles (
    guid TEXT PRIMARY KEY,
    feed_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    link TEXT,
    author TEXT,
    published_at INTEGER,
    content_skeleton TEXT,
    is_read BOOLEAN DEFAULT 0,
    is_starred BOOLEAN DEFAULT 0,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE
);

-- Article Blocks table
CREATE TABLE IF NOT EXISTS article_blocks (
    user_id INTEGER NOT NULL,
    article_guid TEXT NOT NULL,
    block_index INTEGER NOT NULL,
    raw_text TEXT NOT NULL,
    summary TEXT,
    trans_text TEXT,
    PRIMARY KEY (user_id, article_guid, block_index),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (article_guid) REFERENCES articles(guid) ON DELETE CASCADE
);

-- API Configs table
CREATE TABLE IF NOT EXISTS api_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    api_type TEXT NOT NULL,
    api_key TEXT,
    base_url TEXT,
    settings TEXT NOT NULL, -- JSON string
    timeout_seconds INTEGER DEFAULT 180,
    retry_count INTEGER DEFAULT 3,
    retry_interval_ms INTEGER DEFAULT 1000,
    retry_enabled BOOLEAN DEFAULT 1
);

-- 8. 索引 (优化查询性能)

-- 加快按用户查询订阅列表的速度
CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions(user_id);

-- 加快按 Feed 查询文章的速度
CREATE INDEX IF NOT EXISTS idx_articles_feed_id ON articles(feed_id);

-- 加快按发布时间排序的速度 (用于阅读器列表倒序展示)
CREATE INDEX IF NOT EXISTS idx_articles_published_at ON articles(published_at DESC);

-- 加快未读文章的过滤
CREATE INDEX IF NOT EXISTS idx_articles_unread ON articles(is_read) WHERE is_read = 0;

-- 加快文件夹按用户查询的速度
CREATE INDEX IF NOT EXISTS idx_folders_user_id ON folders(user_id);
