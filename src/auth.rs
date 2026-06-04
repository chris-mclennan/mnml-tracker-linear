//! Linear personal API key loader. Reads
//! `~/.config/mnml-tickets-linear/token` (one line, chmod 600).
//!
//! Linear's API takes the raw token in the `Authorization` header
//! (no `Bearer ` prefix for personal keys — that's the OAuth path).
//! Generate at: https://linear.app/settings/api

use anyhow::{Context, Result, anyhow};
use std::path::PathBuf;

pub fn token_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("mnml-tickets-linear")
        .join("token")
}

pub fn load_token() -> Result<String> {
    let path = token_path();
    let s =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let token = s.trim().to_string();
    if token.is_empty() {
        return Err(anyhow!(
            "{} is empty — paste your Linear API key",
            path.display()
        ));
    }
    Ok(token)
}
