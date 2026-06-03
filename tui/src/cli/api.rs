use anyhow::Result;
use clap::Subcommand;

use crate::api_client::ApiClient;
use crate::config::Config;

#[derive(Subcommand)]
pub enum ApiAction {
    /// 列出 API 配置
    List,

    /// 添加 API 配置
    Add {
        /// 配置名称
        name: String,
        /// 类型 (openai/deeplx)
        #[arg(long)]
        r#type: String,
        /// API 端点
        #[arg(long)]
        url: String,
        /// API Key
        #[arg(long)]
        key: String,
    },

    /// 删除 API 配置
    Delete {
        /// 配置 ID
        id: i64,
    },
}

pub async fn run(action: ApiAction) -> Result<()> {
    let config = Config::load()?;
    let client = ApiClient::new(&config)?;

    match action {
        ApiAction::List => {
            let configs = client.get_api_configs().await?;
            println!("{:<6} {:<20} {:<10} {:<40}", "ID", "名称", "类型", "端点");
            println!("{}", "-".repeat(80));
            for c in &configs {
                println!("{:<6} {:<20} {:<10} {:<40}",
                    c.id,
                    crate::cli::sub::truncate(&c.name, 18),
                    c.api_type,
                    c.base_url.as_deref().unwrap_or("(默认)"),
                );
            }
            println!("\n共 {} 个配置", configs.len());
        }
        ApiAction::Add { name, r#type, url, key } => {
            // TODO: 调用 POST /api/translate-configs
            println!("添加 API 配置:");
            println!("  名称: {}", name);
            println!("  类型: {}", r#type);
            println!("  端点: {}", url);
            println!("  Key: {}...", &key[..8.min(key.len())]);
        }
        ApiAction::Delete { id } => {
            // TODO: 调用 DELETE /api/translate-configs/{id}
            println!("删除 API 配置: {}", id);
        }
    }
    Ok(())
}
