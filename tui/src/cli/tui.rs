use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::api_client::ApiClient;
use crate::config::Config;
use crate::tui_app::App;

pub async fn run() -> Result<()> {
    let config = Config::load()?;

    if config.server.is_empty() || config.api_key.is_empty() {
        eprintln!("请先配置服务器地址和 API Key:");
        eprintln!("  tranrss config set server http://your-server:8000");
        eprintln!("  tranrss config set api_key trss_xxx");
        return Ok(());
    }

    let client = ApiClient::new(&config)?;

    // 测试连接
    match client.get_subscriptions().await {
        Ok(subs) => eprintln!("✓ 连接成功，共 {} 个订阅", subs.len()),
        Err(e) => {
            eprintln!("✗ 连接失败: {}", e);
            return Ok(());
        }
    }

    // 初始化终端
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 运行 TUI
    let mut app = App::new(client);
    let result = app.run(&mut terminal).await;

    // 恢复终端
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
