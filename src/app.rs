//! App state — what's loaded, what's selected, the resolved filter
//! for each configured tab.

use crate::config::{Config, Tab};
use crate::linear::{Client, Issue};
use anyhow::Result;

pub struct App {
    pub cfg: Config,
    pub client: Client,
    pub tabs: Vec<TabState>,
    pub active_tab: usize,
    pub status: String,
}

pub struct TabState {
    pub name: String,
    pub source: TabSource,
    pub issues: Vec<Issue>,
    pub selected: usize,
    pub last_fetched: Option<std::time::Instant>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TabSource {
    Filter(serde_json::Value),
    View(String),
}

impl App {
    pub async fn new(cfg: Config, client: Client) -> Result<Self> {
        let mut tabs: Vec<TabState> = Vec::with_capacity(cfg.tabs.len());
        for t in &cfg.tabs {
            tabs.push(TabState {
                name: t.name.clone(),
                source: source_of(t),
                issues: Vec::new(),
                selected: 0,
                last_fetched: None,
                last_error: None,
            });
        }
        let mut app = App {
            cfg,
            client,
            tabs,
            active_tab: 0,
            status: String::new(),
        };
        app.refresh_active().await;
        Ok(app)
    }

    pub fn active(&self) -> &TabState {
        &self.tabs[self.active_tab]
    }
    pub fn active_mut(&mut self) -> &mut TabState {
        &mut self.tabs[self.active_tab]
    }

    pub fn switch_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active_tab = idx;
            if self.tabs[idx].last_fetched.is_none() {
                self.status = format!("loading {}…", self.tabs[idx].name);
            }
        }
    }

    pub fn move_selection(&mut self, delta: isize) {
        let len = self.active().issues.len();
        if len == 0 {
            return;
        }
        let s = self.active().selected as isize + delta;
        let new = s.clamp(0, len as isize - 1) as usize;
        self.active_mut().selected = new;
    }

    pub async fn refresh_active(&mut self) {
        let idx = self.active_tab;
        let source = self.tabs[idx].source.clone();
        self.status = format!("refreshing {}…", self.tabs[idx].name);
        let result = match source {
            TabSource::Filter(f) => self.client.search(f, 100).await,
            TabSource::View(id) => self.client.search_view(&id, 100).await,
        };
        match result {
            Ok(issues) => {
                self.tabs[idx].issues = issues;
                self.tabs[idx].last_fetched = Some(std::time::Instant::now());
                self.tabs[idx].last_error = None;
                self.tabs[idx].selected = self.tabs[idx]
                    .selected
                    .min(self.tabs[idx].issues.len().saturating_sub(1));
                self.status = format!(
                    "{} · {} issues",
                    self.tabs[idx].name,
                    self.tabs[idx].issues.len()
                );
            }
            Err(e) => {
                self.tabs[idx].last_error = Some(e.to_string());
                self.status = format!("error: {e}");
            }
        }
    }

    pub fn open_focused(&mut self) {
        let Some(issue) = self.active().issues.get(self.active().selected) else {
            return;
        };
        let url = issue.url.clone();
        match webbrowser::open(&url) {
            Ok(()) => self.status = format!("opened {} in browser", issue.identifier),
            Err(e) => self.status = format!("open failed: {e}"),
        }
    }
}

fn source_of(t: &Tab) -> TabSource {
    if let Some(view_id) = &t.view_id {
        TabSource::View(view_id.clone())
    } else if let Some(filter) = &t.filter {
        TabSource::Filter(filter.clone())
    } else {
        // Caught by Config::validate, but a no-op filter as a safety net.
        TabSource::Filter(serde_json::json!({}))
    }
}
