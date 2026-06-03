mod api;
mod app;
mod config;
mod ui;

use anyhow::Result;
use app::App;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 加载配置
    let config = Config::load()?;

    // 2. 如果没有配置，进入配置流程
    if config.server.is_empty() || config.api_key.is_empty() {
        println!("TranRSS TUI - 首次使用");
        println!("请先配置服务器地址和 API Key:");
        println!();
        println!("  tranrss-tui config --server http://your-server:8000 --api-key trss_xxx");
        println!();
        println!("或创建配置文件 ~/.config/tranrss/tui.toml:");
        println!("  server = \"http://your-server:8000\"");
        println!("  api_key = \"trss_xxx\"");
        return Ok(());
    }

    // 3. 测试连接
    let client = api::ApiClient::new(&config)?;
    match client.get_subscriptions().await {
        Ok(subs) => {
            println!("✓ 连接成功，共 {} 个订阅", subs.len());
        }
        Err(e) => {
            eprintln!("✗ 连接失败: {}", e);
            return Ok(());
        }
    }

    // 4. 启动 TUI
    let mut terminal = ui::init_terminal()?;
    let mut app = App::new(client);
    app.run(&mut terminal).await?;
    ui::restore_terminal(&mut terminal)?;

    Ok(())
}
