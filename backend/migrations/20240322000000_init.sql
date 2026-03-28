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
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_folders_user_title ON folders (user_id, title);

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
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (feed_id) REFERENCES feeds (id) ON DELETE CASCADE,
    FOREIGN KEY (folder_id) REFERENCES folders (id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_user_id ON subscriptions (user_id);

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
    user_id INTEGER REFERENCES users (id) ON DELETE CASCADE
);

-- articles 表
CREATE TABLE IF NOT EXISTS articles (
    id INTEGER PRIMARY KEY,
    original_guid TEXT NOT NULL UNIQUE,
    feed_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    link TEXT,
    author TEXT,
    published_at INTEGER DEFAULT (strftime('%s', 'now')),
    content_skeleton TEXT,
    is_read INTEGER DEFAULT 0,
    is_starred INTEGER DEFAULT 0,
    updated_at INTEGER DEFAULT (strftime('%s', 'now')),
    summary TEXT,
    crawl_time INTEGER DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (feed_id) REFERENCES feeds (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_articles_published ON articles (published_at DESC);

CREATE INDEX IF NOT EXISTS idx_articles_feed_unread ON articles (feed_id, is_read);

CREATE INDEX IF NOT EXISTS articles_feed_id ON articles (feed_id);

-- article_blocks 表
CREATE TABLE IF NOT EXISTS article_blocks (
    user_id INTEGER NOT NULL,
    article_id INTEGER NOT NULL,
    block_index INTEGER NOT NULL,
    raw_text TEXT NOT NULL,
    trans_text TEXT,
    PRIMARY KEY (
        user_id,
        article_id,
        block_index
    ),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (article_id) REFERENCES articles (id) ON DELETE CASCADE
);

-- idx_article_blocks_lookup is redundant as the PRIMARY KEY (user_id, article_id, block_index) already covers it.

CREATE INDEX IF NOT EXISTS idx_blocks_article_ordered ON article_blocks (article_id, block_index);

CREATE INDEX IF NOT EXISTS idx_blocks_untranslated ON article_blocks (user_id)
WHERE
    trans_text IS NULL;

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
    custom_trans_style TEXT DEFAULT 'display: block;
font-style: italic;
opacity: 0.6;
font-size: 0.95em;
margin-top: 0.3rem;
padding-left: 0.75rem;
border-left: 2px solid rgba(var(--v-theme-primary), 0.4);',
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

-- inactive_feeds 表
CREATE TABLE IF NOT EXISTS inactive_feeds (
    user_id INTEGER NOT NULL,
    feed_id INTEGER NOT NULL,
    reason TEXT,
    disabled_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, feed_id),
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (feed_id) REFERENCES feeds (id) ON DELETE CASCADE
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

CREATE INDEX IF NOT EXISTS Idx ON Workers (id);

CREATE INDEX IF NOT EXISTS WTIdx ON Workers (worker_type);

CREATE INDEX IF NOT EXISTS LSIdx ON Workers (last_seen);

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
    FOREIGN KEY (lock_by) REFERENCES Workers (id)
);
-- Add api_usage table to track token usage
CREATE TABLE IF NOT EXISTS api_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    api_config_id INTEGER NOT NULL,
    model TEXT NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    total_tokens INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (api_config_id) REFERENCES api_configs (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS system_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS TIdx ON Jobs (id);

CREATE INDEX IF NOT EXISTS SIdx ON Jobs (status);

CREATE INDEX IF NOT EXISTS LIdx ON Jobs (lock_by);

CREATE INDEX IF NOT EXISTS JTIdx ON Jobs (job_type);

CREATE INDEX IF NOT EXISTS idx_api_usage_user_config ON api_usage (user_id, api_config_id);

-- ---------------------------------------------------------
-- 性能优化索引
-- ---------------------------------------------------------

-- 优化“进入某个源”时的文章列表加载速度 (覆盖排序和过滤)
CREATE INDEX IF NOT EXISTS idx_articles_feed_published ON articles (feed_id, published_at DESC);

-- 优化“查看未读/置顶”列表
CREATE INDEX IF NOT EXISTS idx_articles_read_published ON articles (is_read, published_at DESC);
CREATE INDEX IF NOT EXISTS idx_articles_starred_published ON articles (is_starred, published_at DESC);

-- 优化背景同步任务：快速通过 feed_id 找到所有订阅用户
CREATE INDEX IF NOT EXISTS idx_subscriptions_feed_user ON subscriptions (feed_id, user_id);

-- 优化文章块查询（虽然主键已涵盖此顺序，但增加显式索引有助于复杂查询优化）
CREATE INDEX IF NOT EXISTS idx_article_blocks_meta ON article_blocks (article_id, user_id);

-- 优化爬虫调度：优先查找待更新的源
CREATE INDEX IF NOT EXISTS idx_feeds_fetch_priority ON feeds (last_fetched_at ASC, consecutive_fetch_failures DESC);

-- 优化用量统计查询速度
CREATE INDEX IF NOT EXISTS idx_api_usage_created ON api_usage (created_at DESC);