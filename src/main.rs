mod app;
mod auth;
mod config;
mod keys;
mod linear;
mod theme;
mod ui;

use anyhow::{Context, Result};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "mnml-tickets-linear",
    version,
    about = "Linear ticket viewer for mnml"
)]
struct Cli {
    /// Print the resolved config + auth state and exit.
    #[arg(long)]
    check: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let token = auth::load_token()
        .with_context(|| format!("couldn't load token from {}", auth::token_path().display()))?;
    let cfg = config::load()?;

    if cli.check {
        println!("config: {}", config::config_path().display());
        println!(
            "token: {} (loaded, {} chars)",
            auth::token_path().display(),
            token.len()
        );
        println!("refresh_interval_secs: {}", cfg.refresh_interval_secs);
        for (i, t) in cfg.tabs.iter().enumerate() {
            let source = if let Some(id) = &t.view_id {
                format!("view_id={id}")
            } else {
                format!(
                    "filter={}",
                    t.filter.as_ref().map(|v| v.to_string()).unwrap_or_default()
                )
            };
            println!("  tab {} ({}): {source}", i + 1, t.name);
        }
        return Ok(());
    }

    let client = linear::Client::new(&token)?;
    let mut app = app::App::new(cfg, client).await?;

    ui::run(&mut app).await
}
