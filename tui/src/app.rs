use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::api::{ApiClient, Article, Subscription};

pub enum AppState {
    Subscriptions,
    Articles,
}

pub struct App {
    pub state: AppState,
    pub subscriptions: Vec<Subscription>,
    pub articles: Vec<Article>,
    pub sub_index: usize,
    pub article_index: usize,
    pub loading: bool,
    pub message: Option<String>,
    pub should_quit: bool,
    client: ApiClient,
}

impl App {
    pub fn new(client: ApiClient) -> Self {
        Self {
            state: AppState::Subscriptions,
            subscriptions: Vec::new(),
            articles: Vec::new(),
            sub_index: 0,
            article_index: 0,
            loading: false,
            message: None,
            should_quit: false,
            client,
        }
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        // 初始加载订阅
        self.load_subscriptions().await;

        loop {
            // 渲染
            terminal.draw(|f| crate::ui::render(f, self))?;

            // 处理事件
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code).await;
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    async fn handle_key(&mut self, code: KeyCode) {
        match self.state {
            AppState::Subscriptions => match code {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.subscriptions.is_empty() {
                        self.sub_index = (self.sub_index + 1) % self.subscriptions.len();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !self.subscriptions.is_empty() {
                        self.sub_index = if self.sub_index == 0 {
                            self.subscriptions.len() - 1
                        } else {
                            self.sub_index - 1
                        };
                    }
                }
                KeyCode::Enter => {
                    if !self.subscriptions.is_empty() {
                        let sub = &self.subscriptions[self.sub_index];
                        let feed_id = sub.feed_id;
                        self.load_articles(Some(feed_id)).await;
                        self.state = AppState::Articles;
                        self.article_index = 0;
                    }
                }
                KeyCode::Char('a') => {
                    self.load_articles(None).await;
                    self.state = AppState::Articles;
                    self.article_index = 0;
                }
                KeyCode::Char('r') => {
                    self.load_subscriptions().await;
                }
                _ => {}
            },
            AppState::Articles => match code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    self.state = AppState::Subscriptions;
                    self.articles.clear();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if !self.articles.is_empty() {
                        self.article_index = (self.article_index + 1) % self.articles.len();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !self.articles.is_empty() {
                        self.article_index = if self.article_index == 0 {
                            self.articles.len() - 1
                        } else {
                            self.article_index - 1
                        };
                    }
                }
                KeyCode::Char(' ') => {
                    if !self.articles.is_empty() {
                        let article = &self.articles[self.article_index];
                        let new_read = !article.is_read;
                        let id = article.id;
                        if self.client.mark_read(id, new_read).await.is_ok() {
                            self.articles[self.article_index].is_read = new_read;
                            self.message = Some(if new_read {
                                "已标记为已读".into()
                            } else {
                                "已标记为未读".into()
                            });
                        }
                    }
                }
                KeyCode::Char('s') => {
                    if !self.articles.is_empty() {
                        let article = &self.articles[self.article_index];
                        let new_starred = !article.is_starred;
                        let id = article.id;
                        if self.client.mark_starred(id, new_starred).await.is_ok() {
                            self.articles[self.article_index].is_starred = new_starred;
                            self.message = Some(if new_starred {
                                "已收藏".into()
                            } else {
                                "已取消收藏".into()
                            });
                        }
                    }
                }
                KeyCode::Char('o') => {
                    if !self.articles.is_empty() {
                        let article = &self.articles[self.article_index];
                        if let Some(ref link) = article.link {
                            let _ = open::that(link);
                        }
                    }
                }
                _ => {}
            },
        }
    }

    async fn load_subscriptions(&mut self) {
        self.loading = true;
        self.message = None;
        match self.client.get_subscriptions().await {
            Ok(subs) => {
                self.subscriptions = subs;
                self.sub_index = 0;
                self.message = None;
            }
            Err(e) => {
                self.message = Some(format!("加载失败: {}", e));
            }
        }
        self.loading = false;
    }

    async fn load_articles(&mut self, feed_id: Option<i64>) {
        self.loading = true;
        self.message = None;
        match self.client.get_articles(feed_id, None).await {
            Ok(articles) => {
                self.articles = articles;
                self.article_index = 0;
                self.message = None;
            }
            Err(e) => {
                self.message = Some(format!("加载失败: {}", e));
            }
        }
        self.loading = false;
    }
}
