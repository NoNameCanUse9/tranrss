use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::Config;

#[derive(Debug, Clone, Deserialize)]
pub struct Subscription {
    pub id: i64,
    #[serde(rename = "feedId")]
    pub feed_id: i64,
    pub title: String,
    pub url: String,
    #[serde(rename = "siteUrl")]
    pub site_url: Option<String>,
    pub category: String,
    #[serde(rename = "customTitle")]
    pub custom_title: Option<String>,
    #[serde(rename = "needTranslate")]
    pub need_translate: Option<bool>,
    #[serde(rename = "needSummary")]
    pub need_summary: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Article {
    pub id: i64,
    pub title: String,
    pub link: Option<String>,
    pub author: Option<String>,
    #[serde(rename = "feedId")]
    pub feed_id: i64,
    #[serde(rename = "feedTitle")]
    pub feed_title: Option<String>,
    #[serde(rename = "isRead")]
    pub is_read: bool,
    #[serde(rename = "isStarred")]
    pub is_starred: bool,
    #[serde(rename = "publishedAt")]
    pub published_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArticleDetail {
    pub detail: ArticleInfo,
    pub blocks: Vec<ArticleBlock>,
    pub content: String,
    #[serde(rename = "is_need_translated")]
    pub is_need_translated: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArticleInfo {
    pub id: i64,
    pub title: String,
    pub link: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    #[serde(rename = "publishedAt")]
    pub published_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArticleBlock {
    #[serde(rename = "blockIndex")]
    pub block_index: i32,
    #[serde(rename = "rawText")]
    pub raw_text: String,
    #[serde(rename = "transText")]
    pub trans_text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JobInfo {
    pub id: String,
    #[serde(rename = "jobType")]
    pub job_type: String,
    pub category: String,
    pub status: String,
    pub attempts: i32,
    #[serde(rename = "lastError")]
    pub last_error: Option<String>,
    #[serde(rename = "feedTitle")]
    pub feed_title: Option<String>,
    #[serde(rename = "titleLabel")]
    pub title_label: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfigInfo {
    pub id: i64,
    pub name: String,
    #[serde(rename = "apiType")]
    pub api_type: String,
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
}

pub struct ApiClient {
    client: reqwest::Client,
    server: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(config: &Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self {
            client,
            server: config.api_base(),
            api_key: config.api_key_value(),
        })
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.server, path);
        let resp = self.client.get(&url)
            .header("Authorization", self.auth_header())
            .send().await
            .with_context(|| format!("请求失败: {}", url))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("API 错误 {}: {}", status, body);
        }
        resp.json().await.with_context(|| "解析响应失败")
    }

    async fn post_empty(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.server, path);
        let resp = self.client.post(&url)
            .header("Authorization", self.auth_header())
            .json(&serde_json::json!({}))
            .send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("API 错误: {}", resp.status());
        }
        Ok(())
    }

    async fn post_json<T: serde::de::DeserializeOwned>(&self, path: &str, body: &serde_json::Value) -> Result<T> {
        let url = format!("{}{}", self.server, path);
        let resp = self.client.post(&url)
            .header("Authorization", self.auth_header())
            .json(body)
            .send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("API 错误: {}", resp.status());
        }
        resp.json().await.with_context(|| "解析响应失败")
    }

    // 订阅
    pub async fn get_subscriptions(&self) -> Result<Vec<Subscription>> {
        self.get("/api/feeds").await
    }

    pub async fn sync_subscription(&self, id: i64) -> Result<()> {
        self.post_empty(&format!("/api/feeds/{}/sync", id)).await
    }

    pub async fn sync_all(&self) -> Result<()> {
        self.post_empty("/api/feeds/sync_all").await
    }

    // 文章
    pub async fn get_articles(&self, feed_id: Option<i64>, is_read: Option<bool>, is_starred: Option<bool>) -> Result<Vec<Article>> {
        let mut params = Vec::new();
        if let Some(fid) = feed_id { params.push(format!("feed_id={}", fid)); }
        if let Some(read) = is_read { params.push(format!("is_read={}", read)); }
        if let Some(starred) = is_starred { params.push(format!("is_starred={}", starred)); }
        let query = if params.is_empty() { String::new() } else { format!("?{}", params.join("&")) };
        self.get(&format!("/api/articles{}", query)).await
    }

    pub async fn get_article_detail(&self, id: i64) -> Result<ArticleDetail> {
        self.get(&format!("/api/articles/{}", id)).await
    }

    pub async fn mark_read(&self, id: i64, read: bool) -> Result<()> {
        let url = format!("/api/articles/{}/read", id);
        self.post_json(&url, &serde_json::json!({ "read": read })).await
    }

    pub async fn mark_starred(&self, id: i64, starred: bool) -> Result<()> {
        let url = format!("/api/articles/{}/star", id);
        self.post_json(&url, &serde_json::json!({ "starred": starred })).await
    }

    pub async fn translate_article(&self, id: i64) -> Result<()> {
        self.post_empty(&format!("/api/articles/{}/translate", id)).await
    }

    pub async fn summarize_article(&self, id: i64) -> Result<()> {
        self.post_empty(&format!("/api/articles/{}/summarize", id)).await
    }

    // 任务
    pub async fn get_jobs(&self) -> Result<Vec<JobInfo>> {
        self.get("/api/jobs").await
    }

    pub async fn retry_job(&self, id: &str) -> Result<()> {
        self.post_empty(&format!("/api/jobs/{}/retry", id)).await
    }

    pub async fn clear_completed(&self) -> Result<()> {
        self.post_empty("/api/jobs/clear_completed").await
    }

    // API 配置
    pub async fn get_api_configs(&self) -> Result<Vec<ApiConfigInfo>> {
        self.get("/api/translate-configs").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_subscription() {
        let json = r#"{
            "id": 1,
            "feedId": 10,
            "title": "Hacker News",
            "url": "https://news.ycombinator.com/rss",
            "siteUrl": "https://news.ycombinator.com",
            "category": "tech",
            "customTitle": null,
            "needTranslate": true,
            "needSummary": false
        }"#;

        let sub: Subscription = serde_json::from_str(json).unwrap();
        assert_eq!(sub.id, 1);
        assert_eq!(sub.feed_id, 10);
        assert_eq!(sub.title, "Hacker News");
        assert_eq!(sub.category, "tech");
        assert_eq!(sub.need_translate, Some(true));
        assert_eq!(sub.need_summary, Some(false));
    }

    #[test]
    fn test_deserialize_article() {
        let json = r#"{
            "id": 42,
            "title": "Test Article",
            "link": "https://example.com/article",
            "author": "John Doe",
            "feedId": 10,
            "feedTitle": "Test Feed",
            "isRead": false,
            "isStarred": true,
            "publishedAt": 1704067200
        }"#;

        let article: Article = serde_json::from_str(json).unwrap();
        assert_eq!(article.id, 42);
        assert_eq!(article.title, "Test Article");
        assert!(!article.is_read);
        assert!(article.is_starred);
        assert_eq!(article.published_at, Some(1704067200));
    }

    #[test]
    fn test_deserialize_article_detail() {
        let json = r#"{
            "detail": {
                "id": 42,
                "title": "Test",
                "link": null,
                "author": null,
                "summary": null,
                "publishedAt": null
            },
            "blocks": [
                {
                    "blockIndex": -1,
                    "rawText": "Title",
                    "transText": "标题"
                },
                {
                    "blockIndex": 0,
                    "rawText": "Content",
                    "transText": null
                }
            ],
            "content": "<p>Hello</p>",
            "is_need_translated": true
        }"#;

        let detail: ArticleDetail = serde_json::from_str(json).unwrap();
        assert_eq!(detail.detail.id, 42);
        assert_eq!(detail.blocks.len(), 2);
        assert_eq!(detail.blocks[0].block_index, -1);
        assert_eq!(detail.blocks[0].trans_text, Some("标题".to_string()));
        assert!(detail.is_need_translated);
    }

    #[test]
    fn test_deserialize_job_info() {
        let json = r#"{
            "id": "abc-123",
            "jobType": "SyncFeedJob",
            "category": "sync",
            "status": "Done",
            "attempts": 1,
            "lastError": null,
            "feedTitle": "HN",
            "titleLabel": "Hacker News"
        }"#;

        let job: JobInfo = serde_json::from_str(json).unwrap();
        assert_eq!(job.id, "abc-123");
        assert_eq!(job.job_type, "SyncFeedJob");
        assert_eq!(job.status, "Done");
        assert_eq!(job.feed_title, Some("HN".to_string()));
    }

    #[test]
    fn test_deserialize_api_config() {
        let json = r#"{
            "id": 1,
            "name": "OpenAI",
            "apiType": "openai",
            "baseUrl": "https://api.openai.com/v1"
        }"#;

        let config: ApiConfigInfo = serde_json::from_str(json).unwrap();
        assert_eq!(config.id, 1);
        assert_eq!(config.name, "OpenAI");
        assert_eq!(config.api_type, "openai");
    }

    #[test]
    fn test_auth_header_format() {
        let config = Config {
            database: crate::config::DatabaseMode::Remote {
                server: "http://localhost:8000".to_string(),
                api_key: "trss_abc123".to_string(),
            },
            ..Default::default()
        };
        let client = ApiClient::new(&config).unwrap();
        assert_eq!(client.auth_header(), "Bearer trss_abc123");
    }

    #[test]
    fn test_client_new_trailing_slash() {
        let config = Config {
            database: crate::config::DatabaseMode::Remote {
                server: "http://localhost:8000/".to_string(),
                api_key: "trss_test".to_string(),
            },
            ..Default::default()
        };
        let client = ApiClient::new(&config).unwrap();
        assert_eq!(client.server, "http://localhost:8000");
    }
}
