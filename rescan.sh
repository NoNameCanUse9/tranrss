#!/bin/bash

# 配置数据库路径
DB_PATH="/home/choken/docker/demo-tranrss/data/data.database"

if [ ! -f "$DB_PATH" ]; then
    echo "❌ 数据库文件不存在: $DB_PATH"
    exit 1
fi

echo "🔍 正在体检数据库任务队列..."

# 1. 救活卡死的任务
echo "➡️ 恢复卡死在 'Killed' 状态的任务..."
sqlite3 "$DB_PATH" "UPDATE Jobs SET status = 'Pending', attempts = 0 WHERE status = 'Killed';"

# 2. 补漏翻译任务 (TranslateArticleJob)
echo "➡️ 补全缺失翻译的文章 (最近 30 天)..."
sqlite3 "$DB_PATH" <<EOF
INSERT INTO Jobs (id, job_type, job, status, attempts, max_attempts, created_at)
SELECT 
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(6))),
    'tranrss_backend::services::jobs::TranslateArticleJob',
    '{"article_id":' || a.id || ',"user_id":' || s.user_id || '}',
    'Pending',
    0,
    1,
    datetime('now')
FROM articles a
JOIN subscriptions s ON a.feed_id = s.feed_id
WHERE s.need_translate = 1
  AND a.published_at > datetime('now', '-30 days')
  AND NOT EXISTS (
      SELECT 1 FROM article_blocks b 
      WHERE b.article_id = a.id AND b.user_id = s.user_id AND b.trans_text IS NOT NULL
  )
  AND NOT EXISTS (
      SELECT 1 FROM Jobs j 
      WHERE j.job_type LIKE '%TranslateArticleJob%'
        AND json_extract(j.job, '$.article_id') = a.id
        AND json_extract(j.job, '$.user_id') = s.user_id
        AND j.status IN ('Pending', 'Running')
  )
LIMIT 500;
EOF

# 3. 补漏总结任务 (SummarizeArticleJob)
echo "➡️ 补全缺失摘要的文章 (最近 30 天)..."
sqlite3 "$DB_PATH" <<EOF
INSERT INTO Jobs (id, job_type, job, status, attempts, max_attempts, created_at)
SELECT 
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(6))),
    'tranrss_backend::services::jobs::SummarizeArticleJob',
    '{"article_id":' || a.id || ',"user_id":' || s.user_id || '}',
    'Pending',
    0,
    1,
    datetime('now')
FROM articles a
JOIN subscriptions s ON a.feed_id = s.feed_id
WHERE s.need_summary = 1
  AND a.summary IS NULL
  AND NOT EXISTS (
      SELECT 1 FROM Jobs j 
      WHERE j.job_type LIKE '%SummarizeArticleJob%'
        AND json_extract(j.job, '$.article_id') = a.id
        AND json_extract(j.job, '$.user_id') = s.user_id
        AND j.status IN ('Pending', 'Running')
  )
LIMIT 300;
EOF

echo "✅ 补全脚本执行完毕。请观察 Docker 容器日志，翻译任务应该已经开始跳动了！"
