use crate::AppState;
use crate::model::api_config::ApiConfig;
use crate::model::articles::ArticleBlock;
use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
pub struct AiService {
    client: Client,
    pub target_lang: String,
    pub model: String,
    pub config: ApiConfig,
}

#[derive(Serialize)]
struct DeepLXRequest {
    text: String,
    source_lang: String,
    target_lang: String,
}

#[derive(Deserialize)]
struct DeepLXResponse {
    code: i32,
    data: String,
}
#[derive(Serialize, Deserialize)]
struct OpenaiMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenaiRequest {
    model: String,
    messages: Vec<OpenaiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct OpenaiResponse {
    choices: Vec<OpenaiChoice>,
}

#[derive(Deserialize)]
struct OpenaiChoice {
    message: OpenaiMessage,
}

impl AiService {
    pub fn new(target_lang: String, model: String, config: ApiConfig) -> Self {
        // 使用数据库中配置的超时时间
        let timeout_secs = config.timeout_seconds as u64;

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to create reqwest client");

        Self {
            client,
            target_lang,
            model,
            config,
        }
    }

    /// 翻译整篇文章并使用事务写入结果
    pub async fn translate_article(
        &self,
        state: &AppState,
        user_id: i64,
        article_id: i64,
    ) -> Result<()> {
        // 1. 获取（且不仅是需要翻译的）整篇文章的所有文本块
        let blocks = sqlx::query_as::<_, ArticleBlock>(
            r#"
            SELECT article_id, block_index, raw_text, trans_text
            FROM article_blocks
            WHERE article_id = ? AND user_id = ?
            ORDER BY block_index ASC
            "#,
        )
        .bind(article_id)
        .bind(user_id)
        .fetch_all(&state.db)
        .await?;

        if blocks.is_empty() {
            return Ok(());
        }

        // 2. 调用翻译实现，带数据库重试机制
        let mut attempts = 0;
        let max_attempts = if self.config.retry_enabled {
            self.config.retry_count as i32 + 1
        } else {
            1
        };
        let mut last_err;

        let translated_texts = loop {
            attempts += 1;
            let result = match self.config.api_type.as_str() {
                "deeplx" => self.handle_deeplx_translation(&blocks).await,
                "openai" => self.handle_openai_translation(&blocks).await,
                _ => {
                    return Err(anyhow!("目前不支持该 API 类型: {}", self.config.api_type));
                }
            };

            match result {
                Ok(texts) => break texts,
                Err(e) => {
                    last_err = e;
                    if attempts >= max_attempts {
                        return Err(
                            last_err.context(format!("经过 {} 次尝试，翻译依然失败", attempts))
                        );
                    }
                    tracing::warn!(
                        "(article_id={}) 翻译第 {} 次尝试失败 ({}ms 后将重试): {}",
                        article_id,
                        attempts,
                        self.config.retry_interval_ms,
                        last_err
                    );
                    tokio::time::sleep(Duration::from_millis(self.config.retry_interval_ms as u64))
                        .await;
                }
            }
        };

        // 3. 校验返回数量是否一致，确保能顺利按顺序写回
        if translated_texts.len() != blocks.len() {
            return Err(anyhow!(
                "翻译结果数量不匹配: 预期 {} 行, 实际返回 {} 行",
                blocks.len(),
                translated_texts.len()
            ));
        }

        // 4. 开启事务回写数据库
        let mut tx = state.db.begin().await?;

        for (block, trans) in blocks.iter().zip(translated_texts.iter()) {
            sqlx::query("UPDATE article_blocks SET trans_text = ? WHERE article_id = ? AND user_id = ? AND block_index = ?")
                .bind(trans)
                .bind(block.article_id)
                .bind(user_id)
                .bind(block.block_index)
                .execute(&mut *tx)
                .await?;
        }

        // 提交事务
        tx.commit().await.context("提交翻译事务失败")?;
        tracing::info!("(article_id={}) 翻译事务提交成功", article_id);

        Ok(())
    }

    /// 批量翻译所有需要翻译且尚未翻译的文章标题
    pub async fn translate_titles_batch(
        &self,
        state: &AppState,
        user_id: i64,
    ) -> Result<usize> {
        // 1. 获取最多 50 个尚未翻译标题的文章
        let untranslated = sqlx::query_as::<_, (i64, String)>(
            r#"
            SELECT a.id, a.title 
            FROM articles a
            JOIN subscriptions s ON s.feed_id = a.feed_id
            WHERE s.user_id = ? AND s.need_translate = 1
              AND NOT EXISTS (
                  SELECT 1 FROM article_blocks b 
                  WHERE b.article_id = a.id AND b.user_id = ? AND b.block_index = -1
              )
            ORDER BY a.published_at DESC
            LIMIT 50
            "#,
        )
        .bind(user_id)
        .bind(user_id)
        .fetch_all(&state.db)
        .await?;

        if untranslated.is_empty() {
            return Ok(0);
        }

        // 2. 构造 ArticleBlock 进行翻译，block_index 固定为 -1 代表标题
        let blocks: Vec<ArticleBlock> = untranslated
            .into_iter()
            .map(|(id, title)| ArticleBlock {
                article_id: id,
                block_index: -1,
                raw_text: title,
                trans_text: None,
            })
            .collect();

        // 3. 调用翻译器
        let translated_texts = match self.config.api_type.as_str() {
            "deeplx" => self.handle_deeplx_translation(&blocks).await?,
            "openai" => self.handle_openai_translation(&blocks).await?,
            _ => return Err(anyhow!("目前不支持该 API 类型: {}", self.config.api_type)),
        };

        if translated_texts.len() != blocks.len() {
            return Err(anyhow!("翻译返回结果数量不匹配"));
        }

        // 4. 将翻译好的标题存写入 article_blocks
        let mut tx = state.db.begin().await?;

        for (block, trans) in blocks.iter().zip(translated_texts.iter()) {
            sqlx::query(
                r#"
                INSERT INTO article_blocks (user_id, article_id, block_index, raw_text, trans_text)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT (user_id, article_id, block_index) DO UPDATE SET trans_text = excluded.trans_text
                "#
            )
            .bind(user_id)
            .bind(block.article_id)
            .bind(block.block_index)
            .bind(&block.raw_text)
            .bind(trans)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        tracing::info!("(user_id={}) 批量翻译了 {} 个标题", user_id, blocks.len());

        Ok(blocks.len())
    }

    /// 总结文章内容并写入数据库
    pub async fn summarize_article(
        &self,
        state: &AppState,
        _user_id: i64,
        article_id: i64,
    ) -> Result<()> {
        // 1. 获取文章所有原始文本
        let blocks = sqlx::query_scalar::<_, String>(
            "SELECT raw_text FROM article_blocks WHERE article_id = ? ORDER BY block_index ASC",
        )
        .bind(article_id)
        .fetch_all(&state.db)
        .await?;

        if blocks.is_empty() {
            return Ok(());
        }

        let full_text = blocks.join("\n\n");
        let prompt = format!(
            "Please provide a concise summary of the following text in {}. \
            The summary MUST be under 150 characters. \
            Respond ONLY with the summary text, no extra explanation.",
            self.target_lang
        );

        // 2. 调用 AI 获取总结 (借用 OpenAI 逻辑或类似)
        let summary = match self.config.api_type.as_str() {
            "openai" => self.get_openai_summary(&full_text, &prompt).await?,
            _ => return Err(anyhow!("目前仅支持 OpenAI 进行文章总结")),
        };

        // 3. 更新文章表
        sqlx::query("UPDATE articles SET summary = ? WHERE id = ?")
            .bind(summary)
            .bind(article_id)
            .execute(&state.db)
            .await?;

        tracing::info!("(article_id={}) 文章总结生成成功", article_id);
        Ok(())
    }

    async fn get_openai_summary(&self, text: &str, system_prompt: &str) -> Result<String> {
        let base_url = self
            .config
            .base_url
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/");
        let api_key = self
            .config
            .api_key
            .as_deref()
            .ok_or_else(|| anyhow!("未配置 API Key"))?;

        let mut full_url = base_url.to_string();
        if !full_url.ends_with("/chat/completions") {
            if !full_url.ends_with('/') {
                full_url.push('/');
            }
            full_url.push_str("chat/completions");
        }

        let request = OpenaiRequest {
            model: self.model.clone(),
            messages: vec![
                OpenaiMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                OpenaiMessage {
                    role: "user".to_string(),
                    content: text.to_string(),
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(1024), // 总结通常不需要太长
        };

        let response = self
            .client
            .post(&full_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await?;

        let body: OpenaiResponse = response.json().await?;
        let content = body
            .choices
            .get(0)
            .ok_or_else(|| anyhow!("AI 未返回结果"))?
            .message
            .content
            .clone();

        Ok(content.trim().to_string())
    }

    /// DeepLX 翻译实现的具体处理逻辑
    async fn handle_deeplx_translation(&self, blocks: &[ArticleBlock]) -> Result<Vec<String>> {
        let base_url = self
            .config
            .base_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("DeepLX 翻译配置缺失：base_url 未设置"))?;

        // 将所有区块按行合并，DeepLX 通常按行保留对应关系
        let combined_text = blocks
            .iter()
            .map(|b| b.raw_text.replace("\n", " ")) // 内部换行转为空格，防止行数错乱
            .collect::<Vec<String>>()
            .join("\n");

        let target_lang_code = match self.target_lang.to_lowercase().as_str() {
            "chinese" | "zh" => "ZH",
            "english" | "en" => "EN",
            "japanese" | "ja" => "JA",
            "french" | "fr" => "FR",
            _ => "ZH", // 默认中文
        };

        let mut full_url = base_url.to_string();
        if !full_url.contains("/translate") {
            if !full_url.ends_with('/') {
                full_url.push('/');
            }
            full_url.push_str("translate");
        }

        let response = self
            .client
            .post(&full_url)
            .json(&DeepLXRequest {
                text: combined_text,
                source_lang: "auto".to_string(),
                target_lang: target_lang_code.to_string(),
            })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("DeepLX API 请求失败: {}", response.status()));
        }

        let body: DeepLXResponse = response.json().await?;

        if body.code != 200 {
            return Err(anyhow!(
                "DeepLX 返回错误码: {}, 信息: {}",
                body.code,
                body.data
            ));
        }

        // 按行拆分回 Vec<String>
        let translated_lines: Vec<String> = body.data.lines().map(|s| s.to_string()).collect();

        Ok(translated_lines)
    }

    /// OpenAI 翻译实现的具体处理逻辑 (按 4096 Tokens 分批发送)
    async fn handle_openai_translation(&self, blocks: &[ArticleBlock]) -> Result<Vec<String>> {
        let mut final_results = vec![String::new(); blocks.len()];
        let mut current_batch = Vec::new();
        let mut current_tokens = 0;
        let mut total_estimated_tokens = 0;
        let token_limit = 4096;

        for (i, block) in blocks.iter().enumerate() {
            let block_tokens = self.estimate_tokens(&block.raw_text);
            
            if !current_batch.is_empty() && current_tokens + block_tokens > token_limit {
                tracing::info!("(OpenAI) 发送批次翻译：包含 {} 个区块，预估 {} tokens", current_batch.len(), current_tokens);
                let batch_results = self.process_openai_batch(&current_batch).await?;
                for (idx, trans) in batch_results {
                    final_results[idx] = trans;
                }
                current_batch.clear();
                current_tokens = 0;
            }
            
            if block_tokens > token_limit {
                tracing::warn!("(OpenAI) 单个区块预估 {} tokens，超过限制 {}，可能会导致翻译失败", block_tokens, token_limit);
            }

            current_batch.push((i, block));
            current_tokens += block_tokens;
            total_estimated_tokens += block_tokens;
        }

        if !current_batch.is_empty() {
            tracing::info!("(OpenAI) 发送最后一批翻译：包含 {} 个区块，预估 {} tokens", current_batch.len(), current_tokens);
            let batch_results = self.process_openai_batch(&current_batch).await?;
            for (idx, trans) in batch_results {
                final_results[idx] = trans;
            }
        }

        // 检查并记录总消耗
        tracing::info!("(OpenAI) 文章翻译完成，总预估消耗: {} tokens", total_estimated_tokens);

        // 填充漏掉的翻译（理论上不应该发生，除非 AI 返回的 JSON 包含错误的 ID）
        for (i, res) in final_results.iter_mut().enumerate() {
            if res.is_empty() {
                 *res = blocks[i].raw_text.clone(); 
            }
        }

        Ok(final_results)
    }

    fn estimate_tokens(&self, text: &str) -> usize {
        let mut tokens = 0.0;
        for c in text.chars() {
            if (c >= '\u{4e00}' && c <= '\u{9fff}') || // CJK 统一汉字
               (c >= '\u{3040}' && c <= '\u{309f}') || // 平假名
               (c >= '\u{30a0}' && c <= '\u{30ff}') || // 片假名
               (c >= '\u{ac00}' && c <= '\u{d7af}')    // 谚文 (韩文)
            {
                tokens += 1.5;
            } else if c.is_ascii_alphanumeric() {
                tokens += 0.3;
            } else if c.is_whitespace() {
                tokens += 0.2;
            } else {
                tokens += 0.5;
            }
        }
        tokens as usize + 150 // 给 Prompt 和 JSON 结构预留 150 tokens 的空间
    }

    async fn process_openai_batch(&self, batch: &[(usize, &ArticleBlock)]) -> Result<HashMap<usize, String>> {
        let base_url = self
            .config
            .base_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("OpenAI 翻译配置缺失：base_url 未设置"))?;

        let api_key = self
            .config
            .api_key
            .as_deref()
            .ok_or_else(|| anyhow!("未配置 OpenAI API Key"))?;

        let mut full_url = base_url.to_string();
        if !full_url.ends_with("/chat/completions") {
            if !full_url.ends_with('/') {
                full_url.push('/');
            }
            full_url.push_str("chat/completions");
        }

        let settings: serde_json::Value =
            serde_json::from_str(&self.config.settings).unwrap_or_default();
        let max_tokens = settings
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(4096);

        let mut input_map = HashMap::new();
        for (idx, block) in batch {
            input_map.insert(idx.to_string(), block.raw_text.as_str());
        }

        let input_json = serde_json::to_string(&input_map).unwrap_or_default();

        let system_prompt = format!(
            "You are a professional translator. \
            Task: Translate the provided JSON object into {}. \
            Format: Respond ONLY with a JSON object {{\"ID\": \"translation\", ...}}. \
            Respond with valid JSON only.",
            self.target_lang
        );

        let request = OpenaiRequest {
            model: self.model.clone(),
            messages: vec![
                OpenaiMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                OpenaiMessage {
                    role: "user".to_string(),
                    content: input_json,
                },
            ],
            temperature: Some(0.1),
            max_tokens: Some(max_tokens),
        };

        let response = self
            .client
            .post(&full_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("发送 OpenAI 请求失败 (URL: {}): {}", full_url, e))?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API 请求失败: {}, {}", status, err_text));
        }

        let body: OpenaiResponse = response.json().await.context("解析 OpenAI 响应失败")?;

        let translated_content = body
            .choices
            .get(0)
            .ok_or_else(|| anyhow!("OpenAI 未返回任何翻译结果"))?
            .message
            .content
            .clone();

        let cleaned = translated_content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let batch_result: HashMap<String, String> = serde_json::from_str(cleaned).map_err(|e| {
            anyhow!(
                "解析结果非有效JSON对象: {}. 响应内容: {}",
                e,
                translated_content
            )
        })?;

        let mut results = HashMap::new();
        for (idx_str, trans) in batch_result {
            if let Ok(idx) = idx_str.parse::<usize>() {
                results.insert(idx, trans);
            }
        }

        Ok(results)
    }
}
