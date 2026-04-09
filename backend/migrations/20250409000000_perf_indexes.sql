-- 优化 articles 表的聚合查询（subscription list 中的 COUNT）
CREATE INDEX IF NOT EXISTS idx_articles_feed_starred ON articles (feed_id, is_starred);

-- 优化 published_at 排序和时间范围过滤（GReader stream_contents, stream_items_ids）
CREATE INDEX IF NOT EXISTS idx_articles_published ON articles (published_at DESC);

-- 优化 Jobs 表按 status + job_type 的过滤查询（Cron 调度去重）
CREATE INDEX IF NOT EXISTS idx_jobs_status_type ON Jobs (status, job_type);
