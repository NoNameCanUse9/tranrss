-- Access Keys: API key 认证，支持精细权限控制
CREATE TABLE IF NOT EXISTS access_keys (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key          TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    permissions  TEXT NOT NULL DEFAULT '[]',
    created_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_used_at DATETIME
);

CREATE INDEX idx_access_keys_user ON access_keys(user_id);
CREATE INDEX idx_access_keys_key  ON access_keys(key);
