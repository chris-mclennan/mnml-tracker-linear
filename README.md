# mnml-tickets-linear

Linear ticket viewer for [mnml](https://mnml.sh) — terminal TUI with
configurable tabs backed by Linear's GraphQL API. Same shape as
[mnml-tickets-jira](https://github.com/chris-mclennan/mnml-tickets-jira),
swapping JQL for Linear's filter input + saved views.

```
┌─ linear ─────────────────────────────────────────────────────────┐
│ ▸1.Mine (12)  2.Cycle (47)  3.Triage (3)                          │
└──────────────────────────────────────────────────────────────────┘
┌─ Mine ───────────────────────────────────────────────────────────┐
│ KEY       STATE         PRIORITY  ASSIGNEE         UPDATED   TITLE
│ ENG-1234  In Progress   Urgent    chrismclennan    2026-06-03 Fix…
│ ENG-1235  Backlog       Medium    andrew           2026-06-02 AI …
│ …                                                                │
└──────────────────────────────────────────────────────────────────┘
  1-9 tab · ↑↓/jk move · Enter/o open · r refresh · q quit
```

## Install

```sh
cargo install --git https://github.com/chris-mclennan/mnml-tickets-linear mnml-tickets-linear
```

(Homebrew tap + binary releases follow once the binary stabilises.)

## Setup

1. **Generate a Linear personal API key** at
   <https://linear.app/settings/api> ("Personal API keys" — *not*
   OAuth).

2. **Save the token** to `~/.config/mnml-tickets-linear/token`
   (`chmod 600`).

3. **Run once** to scaffold the config:
   ```sh
   mnml-tickets-linear
   ```
   This writes `~/.config/mnml-tickets-linear.toml` and exits with
   instructions. Edit the `[[tabs]]` list to taste.

4. **Re-run** — the TUI launches with your configured tabs.

5. **Verify** the resolved config + auth state:
   ```sh
   mnml-tickets-linear --check
   ```

## Tabs

Each `[[tabs]]` entry is one tab. Two backing modes:

```toml
# Filter object — passed verbatim to Linear's GraphQL
# `issues(filter: ...)` argument.
# Reference: https://developers.linear.app/docs/graphql/working-with-the-graphql-api/filtering
[[tabs]]
name = "Mine"
filter = { assignee = { isMe = { eq = true } }, state = { type = { nin = ["completed", "canceled"] } } }

[[tabs]]
name = "Cycle"
filter = { cycle = { isActive = { eq = true } }, team = { key = { eq = "ENG" } } }
```

```toml
# Saved Linear view — paste the id from the view's URL.
# linear.app/<workspace>/view/my-view-<id>
[[tabs]]
name = "Triage"
view_id = "abc123def456"
```

`filter` and `view_id` are mutually exclusive per tab. Set one or
the other.

## Keys

| Chord          | Action                                       |
|----------------|----------------------------------------------|
| `1`-`9`        | Switch to that tab                           |
| `Tab` / `BackTab` | Cycle tabs forward / back                 |
| `↑` / `k`, `↓` / `j` | Move selection                         |
| `PgUp` / `PgDn` | Jump 10 rows                                |
| `g` / `G`      | Top / bottom                                 |
| `Enter` / `o`  | Open focused ticket in browser               |
| `r`            | Refresh active tab                           |
| `q` / `Esc` / `Ctrl+C` | Quit                                |

## Status & roadmap

**v0.1 (this release):**
- Standalone TUI
- Configurable filter / view_id tabs
- 1-9 tab switching · ↑↓ navigation · open-in-browser · refresh
- Blit mode (`--blit <socket>`) so mnml/tmnl can host as a pane

**Planned (paralleling mnml-tickets-jira's v0.2):**
- Right-half ticket detail panel (description + comments)
- Filter editor overlay (`/`)
- Status transition picker (`t` — mutate workflowState)
- Watcher / subscribe toggle (`w`)
- Comment posting (`c`)
- Bulk-transition across selected rows
- Inline-edit assignee / project / cycle

## License

MIT.
