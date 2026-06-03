use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::api_client::{ApiClient, Article, JobInfo, Subscription};

// ── Tab 定义 ──

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Articles,
    Subscriptions,
    Jobs,
    Api,
    Settings,
}

impl Tab {
    fn all() -> &'static [Tab] {
        &[Tab::Articles, Tab::Subscriptions, Tab::Jobs, Tab::Api, Tab::Settings]
    }

    fn title(&self) -> &'static str {
        match self {
            Tab::Articles => "1:文章",
            Tab::Subscriptions => "2:订阅",
            Tab::Jobs => "3:队列",
            Tab::Api => "4:API",
            Tab::Settings => "5:设置",
        }
    }

    fn index(&self) -> usize {
        match self {
            Tab::Articles => 0,
            Tab::Subscriptions => 1,
            Tab::Jobs => 2,
            Tab::Api => 3,
            Tab::Settings => 4,
        }
    }

    fn from_index(i: usize) -> Tab {
        match i {
            0 => Tab::Articles,
            1 => Tab::Subscriptions,
            2 => Tab::Jobs,
            3 => Tab::Api,
            4 => Tab::Settings,
            _ => Tab::Articles,
        }
    }
}

// ── 文章子视图 ──

enum ArticleView {
    List,
    Detail,
}

// ── App 状态 ──

pub struct App {
    // 通用
    tab: Tab,
    should_quit: bool,
    message: Option<String>,
    loading: bool,
    client: ApiClient,

    // 文章标签页
    subscriptions: Vec<Subscription>,
    articles: Vec<Article>,
    sub_index: usize,
    article_index: usize,
    article_view: ArticleView,
    article_content: Option<String>,  // 纯文本渲染后的内容
    article_detail_id: Option<i64>,

    // 订阅标签页
    sub_list_index: usize,

    // 队列标签页
    jobs: Vec<JobInfo>,
    job_index: usize,

    // 搜索
    search_query: String,
    search_mode: bool,
}

impl App {
    pub fn new(client: ApiClient) -> Self {
        Self {
            tab: Tab::Articles,
            should_quit: false,
            message: None,
            loading: false,
            client,
            subscriptions: Vec::new(),
            articles: Vec::new(),
            sub_index: 0,
            article_index: 0,
            article_view: ArticleView::List,
            article_content: None,
            article_detail_id: None,
            sub_list_index: 0,
            jobs: Vec::new(),
            job_index: 0,
            search_query: String::new(),
            search_mode: false,
        }
    }

    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        self.load_subscriptions().await;
        self.load_articles(None, None, None).await;
        self.load_jobs().await;

        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code).await;
                    }
                }
            }

            if self.should_quit { break; }
        }
        Ok(())
    }

    async fn handle_key(&mut self, code: KeyCode) {
        // 搜索模式
        if self.search_mode {
            match code {
                KeyCode::Esc => { self.search_mode = false; self.search_query.clear(); }
                KeyCode::Enter => { self.search_mode = false; }
                KeyCode::Char(c) => { self.search_query.push(c); }
                KeyCode::Backspace => { self.search_query.pop(); }
                _ => {}
            }
            return;
        }

        // 全局快捷键
        match code {
            KeyCode::Char('q') => { self.should_quit = true; return; }
            KeyCode::Char('?') => { self.message = Some("1-5:切换标签 j/k:导航 Enter:选择 t:翻译 s:摘要空格:已读 *:收藏 o:浏览器 /:搜索 r:刷新 q:退出".into()); return; }
            KeyCode::Char('/') => { self.search_mode = true; self.search_query.clear(); return; }
            KeyCode::Char('r') => { self.reload_current_tab().await; return; }
            KeyCode::Char('1') => { self.tab = Tab::Articles; return; }
            KeyCode::Char('2') => { self.tab = Tab::Subscriptions; return; }
            KeyCode::Char('3') => { self.tab = Tab::Jobs; return; }
            KeyCode::Char('4') => { self.tab = Tab::Api; return; }
            KeyCode::Char('5') => { self.tab = Tab::Settings; return; }
            _ => {}
        }

        // 标签页特定快捷键
        match self.tab {
            Tab::Articles => self.handle_articles_key(code).await,
            Tab::Subscriptions => self.handle_subscriptions_key(code).await,
            Tab::Jobs => self.handle_jobs_key(code).await,
            _ => {}
        }
    }

    // ── 文章标签页 ──

    async fn handle_articles_key(&mut self, code: KeyCode) {
        match self.article_view {
            ArticleView::List => match code {
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.articles.is_empty() {
                        self.article_index = (self.article_index + 1) % self.articles.len();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !self.articles.is_empty() {
                        self.article_index = if self.article_index == 0 { self.articles.len() - 1 } else { self.article_index - 1 };
                    }
                }
                KeyCode::Enter => {
                    if !self.articles.is_empty() {
                        let article = &self.articles[self.article_index];
                        let id = article.id;
                        self.load_article_detail(id).await;
                        self.article_view = ArticleView::Detail;
                    }
                }
                KeyCode::Char(' ') => {
                    if !self.articles.is_empty() {
                        let article = &self.articles[self.article_index];
                        let new_read = !article.is_read;
                        let id = article.id;
                        if self.client.mark_read(id, new_read).await.is_ok() {
                            self.articles[self.article_index].is_read = new_read;
                            self.message = Some(if new_read { "已标记为已读" } else { "已标记为未读" }.into());
                        }
                    }
                }
                KeyCode::Char('*') => {
                    if !self.articles.is_empty() {
                        let article = &self.articles[self.article_index];
                        let new_starred = !article.is_starred;
                        let id = article.id;
                        if self.client.mark_starred(id, new_starred).await.is_ok() {
                            self.articles[self.article_index].is_starred = new_starred;
                            self.message = Some(if new_starred { "已收藏" } else { "已取消收藏" }.into());
                        }
                    }
                }
                KeyCode::Char('t') => {
                    if !self.articles.is_empty() {
                        let id = self.articles[self.article_index].id;
                        if self.client.translate_article(id).await.is_ok() {
                            self.message = Some("翻译任务已提交".into());
                        }
                    }
                }
                KeyCode::Char('s') => {
                    if !self.articles.is_empty() {
                        let id = self.articles[self.article_index].id;
                        if self.client.summarize_article(id).await.is_ok() {
                            self.message = Some("摘要任务已提交".into());
                        }
                    }
                }
                KeyCode::Char('o') => {
                    if !self.articles.is_empty() {
                        if let Some(ref link) = self.articles[self.article_index].link {
                            let _ = open::that(link);
                        }
                    }
                }
                _ => {}
            },
            ArticleView::Detail => match code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.article_view = ArticleView::List;
                    self.article_content = None;
                }
                KeyCode::Char('t') => {
                    if let Some(id) = self.article_detail_id {
                        if self.client.translate_article(id).await.is_ok() {
                            self.message = Some("翻译任务已提交".into());
                        }
                    }
                }
                KeyCode::Char('s') => {
                    if let Some(id) = self.article_detail_id {
                        if self.client.summarize_article(id).await.is_ok() {
                            self.message = Some("摘要任务已提交".into());
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    // TODO: 滚动内容
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    // TODO: 滚动内容
                }
                _ => {}
            },
        }
    }

    // ── 订阅标签页 ──

    async fn handle_subscriptions_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.subscriptions.is_empty() {
                    self.sub_list_index = (self.sub_list_index + 1) % self.subscriptions.len();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.subscriptions.is_empty() {
                    self.sub_list_index = if self.sub_list_index == 0 { self.subscriptions.len() - 1 } else { self.sub_list_index - 1 };
                }
            }
            KeyCode::Enter => {
                if !self.subscriptions.is_empty() {
                    let feed_id = self.subscriptions[self.sub_list_index].feed_id;
                    self.load_articles(Some(feed_id), None, None).await;
                    self.tab = Tab::Articles;
                    self.article_index = 0;
                }
            }
            KeyCode::Char('s') => {
                if !self.subscriptions.is_empty() {
                    let id = self.subscriptions[self.sub_list_index].id;
                    if self.client.sync_subscription(id).await.is_ok() {
                        self.message = Some("同步任务已提交".into());
                    }
                }
            }
            KeyCode::Char('S') => {
                if self.client.sync_all().await.is_ok() {
                    self.message = Some("全部同步任务已提交".into());
                }
            }
            _ => {}
        }
    }

    // ── 队列标签页 ──

    async fn handle_jobs_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.jobs.is_empty() {
                    self.job_index = (self.job_index + 1) % self.jobs.len();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.jobs.is_empty() {
                    self.job_index = if self.job_index == 0 { self.jobs.len() - 1 } else { self.job_index - 1 };
                }
            }
            KeyCode::Char('r') => {
                if !self.jobs.is_empty() {
                    let id = self.jobs[self.job_index].id.clone();
                    if self.client.retry_job(&id).await.is_ok() {
                        self.message = Some("重试任务已提交".into());
                    }
                }
            }
            KeyCode::Char('c') => {
                if self.client.clear_completed().await.is_ok() {
                    self.message = Some("已清除完成的任务".into());
                    self.load_jobs().await;
                }
            }
            _ => {}
        }
    }

    // ── 数据加载 ──

    async fn load_subscriptions(&mut self) {
        self.loading = true;
        match self.client.get_subscriptions().await {
            Ok(subs) => self.subscriptions = subs,
            Err(e) => self.message = Some(format!("加载订阅失败: {}", e)),
        }
        self.loading = false;
    }

    async fn load_articles(&mut self, feed_id: Option<i64>, is_read: Option<bool>, is_starred: Option<bool>) {
        self.loading = true;
        match self.client.get_articles(feed_id, is_read, is_starred).await {
            Ok(articles) => { self.articles = articles; self.article_index = 0; }
            Err(e) => self.message = Some(format!("加载文章失败: {}", e)),
        }
        self.loading = false;
    }

    async fn load_article_detail(&mut self, id: i64) {
        self.loading = true;
        match self.client.get_article_detail(id).await {
            Ok(detail) => {
                self.article_content = Some(html_to_text(&detail.content));
                self.article_detail_id = Some(id);
                // 自动标记已读
                if !self.articles[self.article_index].is_read {
                    let _ = self.client.mark_read(id, true).await;
                    self.articles[self.article_index].is_read = true;
                }
            }
            Err(e) => self.message = Some(format!("加载文章详情失败: {}", e)),
        }
        self.loading = false;
    }

    async fn load_jobs(&mut self) {
        self.loading = true;
        match self.client.get_jobs().await {
            Ok(jobs) => { self.jobs = jobs; self.job_index = 0; }
            Err(e) => self.message = Some(format!("加载任务失败: {}", e)),
        }
        self.loading = false;
    }

    async fn reload_current_tab(&mut self) {
        self.message = None;
        match self.tab {
            Tab::Articles => self.load_articles(None, None, None).await,
            Tab::Subscriptions => self.load_subscriptions().await,
            Tab::Jobs => self.load_jobs().await,
            _ => {}
        }
    }

    // ── 渲染 ──

    fn render(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
            .split(f.area());

        // 标签页头
        self.render_tabs(f, chunks[0]);

        // 主内容
        match self.tab {
            Tab::Articles => self.render_articles(f, chunks[1]),
            Tab::Subscriptions => self.render_subscriptions(f, chunks[1]),
            Tab::Jobs => self.render_jobs(f, chunks[1]),
            Tab::Api => self.render_api(f, chunks[1]),
            Tab::Settings => self.render_settings(f, chunks[1]),
        }

        // 状态栏
        self.render_statusbar(f, chunks[2]);
    }

    fn render_tabs(&self, f: &mut Frame, area: Rect) {
        let titles: Vec<Span> = Tab::all().iter().map(|t| {
            let style = if *t == self.tab {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Span::styled(format!(" {} ", t.title()), style)
        }).collect();

        let header = Paragraph::new(Line::from(titles))
            .block(Block::default().borders(Borders::ALL).title("TranRSS"));
        f.render_widget(header, area);
    }

    fn render_articles(&self, f: &mut Frame, area: Rect) {
        match self.article_view {
            ArticleView::List => self.render_article_list(f, area),
            ArticleView::Detail => self.render_article_detail(f, area),
        }
    }

    fn render_article_list(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(area);

        // 左侧：订阅树
        let sub_items: Vec<ListItem> = self.subscriptions.iter().enumerate().map(|(i, sub)| {
            let style = if i == self.sub_index {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(format!(" {} [{}]", sub.title, sub.category), style))
        }).collect();

        let sub_list = List::new(sub_items)
            .block(Block::default().title("订阅 (Enter:查看)").borders(Borders::ALL));
        f.render_widget(sub_list, chunks[0]);

        // 右侧：文章列表
        let items: Vec<ListItem> = self.articles.iter().enumerate().map(|(i, a)| {
            let style = if i == self.article_index {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if a.is_read {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            let star = if a.is_starred { "★" } else { " " };
            let read = if a.is_read { " " } else { "●" };
            let date = a.published_at
                .map(|ts| chrono::DateTime::from_timestamp(ts, 0)
                    .map(|d| d.format("%m-%d").to_string())
                    .unwrap_or_default())
                .unwrap_or_default();
            ListItem::new(Span::styled(
                format!("{}{} {} {}", star, read, a.title, date), style
            ))
        }).collect();

        let article_list = List::new(items)
            .block(Block::default()
                .title("文章 (Enter:详情空格:已读 *:收藏 t:翻译 s:摘要 o:浏览器)")
                .borders(Borders::ALL));
        f.render_widget(article_list, chunks[1]);
    }

    fn render_article_detail(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(0)])
            .split(area);

        // 顶部：文章元信息
        let meta = if let Some(article) = self.articles.get(self.article_index) {
            let date = article.published_at
                .map(|ts| chrono::DateTime::from_timestamp(ts, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default())
                .unwrap_or_default();
            vec![
                Line::from(vec![Span::styled("标题: ", Style::default().fg(Color::Yellow)), Span::raw(&article.title)]),
                Line::from(vec![
                    Span::styled("来源: ", Style::default().fg(Color::Yellow)),
                    Span::raw(article.feed_title.as_deref().unwrap_or("")),
                    Span::styled("  日期: ", Style::default().fg(Color::Yellow)),
                    Span::raw(date),
                ]),
                Line::from(vec![
                    Span::styled("状态: ", Style::default().fg(Color::Yellow)),
                    Span::raw(if article.is_read { "已读" } else { "未读" }),
                    Span::raw(if article.is_starred { " ★已收藏" } else { "" }),
                ]),
            ]
        } else {
            vec![]
        };
        let meta_widget = Paragraph::new(meta).block(Block::default().borders(Borders::ALL));
        f.render_widget(meta_widget, chunks[0]);

        // 下方：文章内容（纯文本）
        let content = self.article_content.as_deref().unwrap_or("加载中...");
        let content_widget = Paragraph::new(content)
            .wrap(Wrap { trim: false })
            .block(Block::default().title("内容 (Esc:返回 t:翻译 s:摘要)").borders(Borders::ALL));
        f.render_widget(content_widget, chunks[1]);
    }

    fn render_subscriptions(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.subscriptions.iter().enumerate().map(|(i, sub)| {
            let style = if i == self.sub_list_index {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let trans = if sub.need_translate.unwrap_or(false) { "翻译" } else { "" };
            let summ = if sub.need_summary.unwrap_or(false) { "摘要" } else { "" };
            ListItem::new(Span::styled(
                format!(" {} [{}] {} {}", sub.title, sub.category, trans, summ), style
            ))
        }).collect();

        let list = List::new(items).block(Block::default()
            .title("订阅 (Enter:查看文章 s:同步 S:全部同步)")
            .borders(Borders::ALL));
        f.render_widget(list, area);
    }

    fn render_jobs(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.jobs.iter().enumerate().map(|(i, job)| {
            let style = if i == self.job_index {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let status_icon = match job.status.as_str() {
                "Running" => "⏳",
                "Pending" => "🕐",
                "Done" => "✅",
                "Failed" => "❌",
                _ => "❓",
            };
            let title = job.title_label.as_deref().unwrap_or(&job.id);
            ListItem::new(Span::styled(
                format!("{} {} [{}] {}", status_icon, title, job.job_type, job.status), style
            ))
        }).collect();

        let list = List::new(items).block(Block::default()
            .title("任务队列 (r:重试 c:清除已完成)")
            .borders(Borders::ALL));
        f.render_widget(list, area);
    }

    fn render_api(&self, f: &mut Frame, area: Rect) {
        let p = Paragraph::new("API 配置管理\n\n使用 CLI 命令管理:\n  tranrss api list\n  tranrss api add <name> <type> <url> <key>\n  tranrss api delete <id>")
            .block(Block::default().title("API 配置").borders(Borders::ALL));
        f.render_widget(p, area);
    }

    fn render_settings(&self, f: &mut Frame, area: Rect) {
        let p = Paragraph::new("设置管理\n\n使用 CLI 命令管理:\n  tranrss config show\n  tranrss config set <key> <value>\n  tranrss setup (生成 systemd/cron 配置)")
            .block(Block::default().title("设置").borders(Borders::ALL));
        f.render_widget(p, area);
    }

    fn render_statusbar(&self, f: &mut Frame, area: Rect) {
        let status = if self.loading {
            "加载中...".to_string()
        } else if let Some(ref msg) = self.message {
            msg.clone()
        } else if self.search_mode {
            format!("/{}", self.search_query)
        } else {
            match self.tab {
                Tab::Articles => format!("共 {} 篇文章", self.articles.len()),
                Tab::Subscriptions => format!("共 {} 个订阅", self.subscriptions.len()),
                Tab::Jobs => format!("共 {} 个任务", self.jobs.len()),
                _ => String::new(),
            }
        };
        let bar = Paragraph::new(status).style(Style::default().fg(Color::DarkGray));
        f.render_widget(bar, area);
    }
}

// ── HTML → 纯文本转换 ──

fn html_to_text(html: &str) -> String {
    html2text::from_read(html.as_bytes(), 80)
        .unwrap_or_else(|_| html.to_string())
        .lines()
        .map(|line: &str| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}
