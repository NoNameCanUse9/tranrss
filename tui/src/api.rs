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
            server: config.server.trim_end_matches('/').to_string(),
            api_key: config.api_key.clone(),
        })
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    pub async fn get_subscriptions(&self) -> Result<Vec<Subscription>> {
        let url = format!("{}/api/feeds", self.server);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .with_context(|| format!("请求失败: {}", url))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("API 错误 {}: {}", status, body);
        }

        let subs: Vec<Subscription> = resp
            .json()
            .await
            .with_context(|| "解析订阅列表失败")?;
        Ok(subs)
    }

    pub async fn get_articles(
        &self,
        feed_id: Option<i64>,
        is_read: Option<bool>,
    ) -> Result<Vec<Article>> {
        let mut url = format!("{}/api/articles", self.server);
        let mut params = Vec::new();
        if let Some(fid) = feed_id {
            params.push(format!("feed_id={}", fid));
        }
        if let Some(read) = is_read {
            params.push(format!("is_read={}", read));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let resp = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .with_context(|| format!("请求失败: {}", url))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("API 错误 {}: {}", status, body);
        }

        let articles: Vec<Article> = resp
            .json()
            .await
            .with_context(|| "解析文章列表失败")?;
        Ok(articles)
    }

    pub async fn mark_read(&self, article_id: i64, read: bool) -> Result<()> {
        let url = format!("{}/api/articles/{}/read", self.server, article_id);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&serde_json::json!({ "read": read }))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("标记已读失败: {}", resp.status());
        }
        Ok(())
    }

    pub async fn mark_starred(&self, article_id: i64, starred: bool) -> Result<()> {
        let url = format!("{}/api/articles/{}/star", self.server, article_id);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&serde_json::json!({ "starred": starred }))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("标记收藏失败: {}", resp.status());
        }
        Ok(())
    }
}
