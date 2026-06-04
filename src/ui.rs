//! ratatui rendering + the main event loop. Run from `main.rs` with
//! a fully-initialized `App`.

use crate::app::App;
use crate::keys;
use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Tabs},
};
use std::io::Stdout;
use std::time::{Duration, Instant};

pub async fn run(app: &mut App) -> Result<()> {
    let mut stdout = std::io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = event_loop(&mut terminal, app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut last_refresh = Instant::now();
    loop {
        terminal.draw(|f| draw(f, app))?;

        if app.cfg.refresh_interval_secs > 0
            && last_refresh.elapsed().as_secs() >= app.cfg.refresh_interval_secs
        {
            app.refresh_active().await;
            last_refresh = Instant::now();
        }

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) if key.kind == event::KeyEventKind::Press => {
                    if let Some(action) = keys::handle(key, app) {
                        let quit = keys::apply(action, app).await;
                        if quit {
                            break;
                        }
                        last_refresh = Instant::now();
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
    Ok(())
}

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(size);

    draw_tabs(f, chunks[0], app);
    draw_table(f, chunks[1], app);
    draw_status(f, chunks[2], app);
}

fn draw_tabs(f: &mut Frame, area: Rect, app: &App) {
    let labels: Vec<Line> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let n = t.issues.len();
            let label = if t.last_fetched.is_some() {
                format!("{}.{} ({n})", i + 1, t.name)
            } else {
                format!("{}.{}", i + 1, t.name)
            };
            Line::from(label)
        })
        .collect();
    let tabs = Tabs::new(labels)
        .block(Block::default().borders(Borders::ALL).title(" linear "))
        .select(app.active_tab)
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, area);
}

fn draw_table(f: &mut Frame, area: Rect, app: &App) {
    let tab = app.active();
    if let Some(err) = &tab.last_error {
        let p = Paragraph::new(format!("error: {err}\n\nPress `r` to retry."))
            .style(Style::default().fg(Color::Red));
        f.render_widget(p, area);
        return;
    }
    if tab.issues.is_empty() && tab.last_fetched.is_some() {
        let p = Paragraph::new("(no issues match this query)")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    }
    if tab.issues.is_empty() {
        let p = Paragraph::new("loading…").style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("KEY"),
        Cell::from("STATE"),
        Cell::from("PRIORITY"),
        Cell::from("ASSIGNEE"),
        Cell::from("UPDATED"),
        Cell::from("TITLE"),
    ])
    .style(
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = tab
        .issues
        .iter()
        .map(|i| {
            let state = i.state.as_ref().map(|s| s.name.as_str()).unwrap_or("?");
            let state_kind = i.state.as_ref().map(|s| s.kind.as_str()).unwrap_or("");
            let priority = i.priority_label.as_deref().unwrap_or("—");
            let assignee = i
                .assignee
                .as_ref()
                .map(|a| {
                    if a.display_name.is_empty() {
                        a.name.as_str()
                    } else {
                        a.display_name.as_str()
                    }
                })
                .unwrap_or("—");
            let updated = i
                .updated_at
                .as_deref()
                .map(format_updated)
                .unwrap_or_else(|| "—".to_string());
            Row::new(vec![
                Cell::from(i.identifier.clone()).style(Style::default().fg(Color::Yellow)),
                Cell::from(state.to_string()).style(state_color(state_kind)),
                Cell::from(priority.to_string()).style(priority_color(i.priority)),
                Cell::from(assignee.to_string()),
                Cell::from(updated),
                Cell::from(i.title.clone()),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Length(14),
        Constraint::Length(10),
        Constraint::Length(18),
        Constraint::Length(12),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", tab.name)),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    let mut state = TableState::default();
    state.select(Some(tab.selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn draw_status(f: &mut Frame, area: Rect, app: &App) {
    let hint = " 1-9 tab · ↑↓/jk move · Enter/o open · r refresh · q quit ";
    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", app.status),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            hint,
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        ),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

/// ISO 8601 timestamp → `YYYY-MM-DD`.
fn format_updated(s: &str) -> String {
    s.split('T').next().unwrap_or(s).to_string()
}

/// Linear workflow state kinds: backlog / unstarted / started /
/// completed / canceled / triage. Map to a readable color.
fn state_color(kind: &str) -> Style {
    match kind {
        "completed" => Style::default().fg(Color::Green),
        "canceled" => Style::default().fg(Color::DarkGray),
        "started" => Style::default().fg(Color::Cyan),
        "triage" => Style::default().fg(Color::Yellow),
        "backlog" | "unstarted" => Style::default().fg(Color::White),
        _ => Style::default().fg(Color::Gray),
    }
}

/// Linear priority: 0 = no priority, 1 = urgent, 2 = high, 3 = medium, 4 = low.
fn priority_color(p: Option<i32>) -> Style {
    match p.unwrap_or(0) {
        1 => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        2 => Style::default().fg(Color::LightRed),
        3 => Style::default().fg(Color::Yellow),
        4 => Style::default().fg(Color::DarkGray),
        _ => Style::default().fg(Color::Gray),
    }
}
