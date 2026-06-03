use anyhow::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::{App, AppState};

pub fn init_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    // 标题栏
    render_header(f, chunks[0], app);

    // 主内容区
    match app.state {
        AppState::Subscriptions => render_subscriptions(f, chunks[1], app),
        AppState::Articles => render_articles(f, chunks[1], app),
    }

    // 状态栏
    render_statusbar(f, chunks[2], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let title = match app.state {
        AppState::Subscriptions => "TranRSS TUI - 订阅列表",
        AppState::Articles => {
            if let Some(sub) = app.subscriptions.get(app.sub_index) {
                &format!("TranRSS TUI - {}", sub.title)
            } else {
                "TranRSS TUI - 文章列表"
            }
        }
    };

    let header = Paragraph::new(title)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}

fn render_subscriptions(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .subscriptions
        .iter()
        .enumerate()
        .map(|(i, sub)| {
            let style = if i == app.sub_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let category = if sub.category.is_empty() {
                String::new()
            } else {
                format!(" [{}]", sub.category)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{}{}", sub.title, category), style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("订阅源 (↑↓/jk 导航, Enter 查看, a 全部文章, r 刷新, q 退出)")
            .borders(Borders::ALL),
    );
    f.render_widget(list, area);
}

fn render_articles(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // 文章列表
    let items: Vec<ListItem> = app
        .articles
        .iter()
        .enumerate()
        .map(|(i, article)| {
            let style = if i == app.article_index {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if article.is_read {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            let star = if article.is_starred { "★ " } else { "  " };
            let read_mark = if article.is_read { "  " } else { "● " };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{}{}{}", star, read_mark, article.title),
                    style,
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("文章 (↑↓/jk 导航, 空格 已读, s 收藏, o 打开, Esc 返回)")
            .borders(Borders::ALL),
    );
    f.render_widget(list, chunks[0]);

    // 文章详情
    let detail = if let Some(article) = app.articles.get(app.article_index) {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("标题: ", Style::default().fg(Color::Yellow)),
                Span::raw(&article.title),
            ]),
            Line::from(""),
        ];

        if let Some(ref author) = article.author {
            lines.push(Line::from(vec![
                Span::styled("作者: ", Style::default().fg(Color::Yellow)),
                Span::raw(author),
            ]));
        }

        if let Some(ref feed_title) = article.feed_title {
            lines.push(Line::from(vec![
                Span::styled("来源: ", Style::default().fg(Color::Yellow)),
                Span::raw(feed_title),
            ]));
        }

        if let Some(pub_at) = article.published_at {
            let dt = chrono::DateTime::from_timestamp(pub_at, 0)
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();
            lines.push(Line::from(vec![
                Span::styled("时间: ", Style::default().fg(Color::Yellow)),
                Span::raw(dt),
            ]));
        }

        if let Some(ref link) = article.link {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("链接: ", Style::default().fg(Color::Yellow)),
                Span::styled(link, Style::default().fg(Color::Blue)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(
                "状态: {} {}",
                if article.is_read { "已读" } else { "未读" },
                if article.is_starred { "★ 已收藏" } else { "" }
            ),
            Style::default().fg(Color::Green),
        )));

        Paragraph::new(lines).wrap(Wrap { trim: true })
    } else {
        Paragraph::new("选择一篇文章查看详情")
    };

    let detail_block = detail.block(Block::default().title("详情").borders(Borders::ALL));
    f.render_widget(detail_block, chunks[1]);
}

fn render_statusbar(f: &mut Frame, area: Rect, app: &App) {
    let status = if app.loading {
        "加载中...".to_string()
    } else if let Some(ref msg) = app.message {
        msg.clone()
    } else {
        match app.state {
            AppState::Subscriptions => {
                format!("共 {} 个订阅", app.subscriptions.len())
            }
            AppState::Articles => {
                format!(
                    "共 {} 篇文章 | 已读 {} | 未读 {}",
                    app.articles.len(),
                    app.articles.iter().filter(|a| a.is_read).count(),
                    app.articles.iter().filter(|a| !a.is_read).count()
                )
            }
        }
    };

    let statusbar = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
    f.render_widget(statusbar, area);
}
