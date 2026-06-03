use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::api_client::ApiClient;
use crate::config::{Config, DatabaseMode};
use crate::tui_app::App;

pub async fn run() -> Result<()> {
    let config = Config::load()?;

    match &config.database {
        DatabaseMode::Remote { server, api_key } => {
            if server.is_empty() || api_key.is_empty() {
                eprintln!("请先配置服务器地址和 API Key:");
                eprintln!("  tranrss config remote --server http://your-server:8000 --api-key trss_xxx");
                return Ok(());
            }
        }
        DatabaseMode::Local { db_path } => {
            if db_path.is_empty() {
                eprintln!("请先配置 SQLite 数据库路径:");
                eprintln!("  tranrss config local --db-path /path/to/data.database");
                return Ok(());
            }
            if !std::path::Path::new(db_path).exists() {
                eprintln!("数据库文件不存在: {}", db_path);
                return Ok(());
            }
        }
        DatabaseMode::Fresh { data_dir } => {
            if data_dir.is_empty() {
                eprintln!("请先配置数据目录:");
                eprintln!("  tranrss config fresh --data-dir /path/to/data");
                return Ok(());
            }
        }
    }

    let client = ApiClient::new(&config)?;

    // 测试连接（仅远程模式）
    if config.is_remote() {
        match client.get_subscriptions().await {
            Ok(subs) => eprintln!("✓ 连接成功，共 {} 个订阅", subs.len()),
            Err(e) => {
                eprintln!("✗ 连接失败: {}", e);
                return Ok(());
            }
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
