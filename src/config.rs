//! Config file at `~/.config/mnml-tickets-linear.toml`. First run
//! writes the scaffold + exits with instructions.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Polling interval. `0` disables auto-refresh; user can still
    /// press `r` to refresh the active tab. Default 60s.
    #[serde(default = "default_refresh")]
    pub refresh_interval_secs: u64,
    /// Tab list — at least one required.
    pub tabs: Vec<Tab>,
}

fn default_refresh() -> u64 {
    60
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    /// Human label shown in the tab strip.
    pub name: String,
    /// Mutually exclusive with `view_id`: a JSON filter object passed
    /// to Linear's GraphQL `issues(filter: ...)` argument. See
    /// https://developers.linear.app/docs/graphql/working-with-the-graphql-api/filtering
    /// for the available operators (`team`, `state`, `assignee`,
    /// `cycle`, `project`, …).
    #[serde(default)]
    pub filter: Option<serde_json::Value>,
    /// Mutually exclusive with `filter`: load a saved Linear view by
    /// id. The id is the slug at the end of the view's URL —
    /// `linear.app/<workspace>/view/my-view-<id>`.
    #[serde(default)]
    pub view_id: Option<String>,
}

impl Config {
    pub const EXAMPLE: &'static str = r##"# mnml-tickets-linear config. Edit and re-run.

# Auto-refresh in seconds. 0 disables; user can still press `r`.
refresh_interval_secs = 60

# ── Tabs ─────────────────────────────────────────────────────────
# Each `[[tabs]]` entry is one tab. Either:
#   filter  = <JSON filter object>     (substitute for Linear views)
#   view_id = "abc123def"              (saved Linear view id)
#
# Tabs are switched via 1-9 keys (or click) and ordered left→right
# as listed.
#
# Filter object reference:
#   https://developers.linear.app/docs/graphql/working-with-the-graphql-api/filtering

# All open issues assigned to me, most recent first.
[[tabs]]
name = "Mine"
filter = { assignee = { isMe = { eq = true } }, state = { type = { nin = ["completed", "canceled"] } } }

# This sprint's open issues — replace `team.key` with your team's
# 2-3 letter prefix (e.g. "ENG" for the team whose issues are
# `ENG-1234`).
[[tabs]]
name = "Cycle"
filter = { cycle = { isActive = { eq = true } }, team = { key = { eq = "ENG" } } }

# Issues in a specific saved view (paste the id from the URL).
# [[tabs]]
# name = "Triage"
# view_id = "abc123def456"
"##;

    pub fn validate(&self) -> Result<()> {
        if self.tabs.is_empty() {
            return Err(anyhow!("config: at least one [[tabs]] entry required"));
        }
        for (i, t) in self.tabs.iter().enumerate() {
            let label = format!("tab #{i} ({})", t.name);
            match (&t.filter, &t.view_id) {
                (Some(_), None) => {}
                (None, Some(_)) => {}
                (None, None) => {
                    return Err(anyhow!(
                        "{label}: needs either `filter = {{...}}` or `view_id = \"...\"`"
                    ));
                }
                (Some(_), Some(_)) => {
                    return Err(anyhow!(
                        "{label}: set EITHER `filter` OR `view_id`, not both"
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("mnml-tickets-linear.toml")
}

/// Load + validate the config. If the file is missing, write the
/// scaffold and return an error pointing at the file.
pub fn load() -> Result<Config> {
    let path = config_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, Config::EXAMPLE)?;
        return Err(anyhow!(
            "wrote config template to {} — edit it then re-run",
            path.display()
        ));
    }
    let text = std::fs::read_to_string(&path)?;
    let cfg: Config = toml::from_str(&text)?;
    cfg.validate()?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_parses_and_validates() {
        let cfg: Config = toml::from_str(Config::EXAMPLE).expect("example parses");
        cfg.validate().expect("example validates");
        // The scaffold has at least Mine + Cycle uncommented.
        assert!(cfg.tabs.len() >= 2);
    }

    #[test]
    fn validate_rejects_tab_with_neither_filter_nor_view_id() {
        let raw = r##"
[[tabs]]
name = "Bad"
"##;
        let cfg: Config = toml::from_str(raw).unwrap();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_tab_with_both_filter_and_view_id() {
        let raw = r##"
[[tabs]]
name = "Bad"
filter = {}
view_id = "abc"
"##;
        let cfg: Config = toml::from_str(raw).unwrap();
        assert!(cfg.validate().is_err());
    }
}
