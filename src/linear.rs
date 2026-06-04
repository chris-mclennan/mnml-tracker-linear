//! Minimal Linear GraphQL client — only the endpoints we need.
//!
//! Linear's API: https://developers.linear.app/docs/graphql/working-with-the-graphql-api
//! Endpoint: POST https://api.linear.app/graphql
//! Auth: `Authorization: <personal-api-key>` (no `Bearer ` prefix
//! for personal keys; that's the OAuth path).

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

const ENDPOINT: &str = "https://api.linear.app/graphql";

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    token: String,
}

impl Client {
    pub fn new(token: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent("mnml-tickets-linear/0.1.0")
            .build()?;
        Ok(Self {
            http,
            token: token.to_string(),
        })
    }

    /// Run a raw GraphQL query against the Linear API. Surfaces any
    /// `errors` in the response as an `anyhow::Error`. Caller is
    /// responsible for parsing `data` from `T`.
    async fn query<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T> {
        let body = serde_json::json!({
            "query": query,
            "variables": variables,
        });
        let resp = self
            .http
            .post(ENDPOINT)
            .header("Authorization", &self.token)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Linear GraphQL request failed")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Linear API HTTP {status}: {text}"));
        }
        let raw: serde_json::Value = resp
            .json()
            .await
            .context("parsing Linear GraphQL response")?;
        if let Some(errors) = raw.get("errors").and_then(|v| v.as_array())
            && !errors.is_empty()
        {
            let messages: Vec<String> = errors
                .iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()).map(String::from))
                .collect();
            return Err(anyhow!("Linear GraphQL errors: {}", messages.join(" / ")));
        }
        let data = raw
            .get("data")
            .ok_or_else(|| anyhow!("Linear response missing `data` field"))?;
        serde_json::from_value(data.clone())
            .with_context(|| "deserializing GraphQL `data` payload".to_string())
    }

    /// Fetch up to `first` issues matching `filter`. Pagination is
    /// not yet wired — v0.1 caps at the first 100 results.
    pub async fn search(&self, filter: serde_json::Value, first: u32) -> Result<Vec<Issue>> {
        let q = r#"
            query Issues($filter: IssueFilter, $first: Int!) {
              issues(filter: $filter, first: $first) {
                nodes {
                  id
                  identifier
                  title
                  url
                  priority
                  priorityLabel
                  updatedAt
                  state { name type color }
                  assignee { displayName name }
                  team { key }
                }
              }
            }
        "#;
        #[derive(Debug, Deserialize)]
        struct Wrapper {
            issues: IssueConnection,
        }
        #[derive(Debug, Deserialize)]
        struct IssueConnection {
            nodes: Vec<Issue>,
        }
        let wrapper: Wrapper = self
            .query(q, serde_json::json!({ "filter": filter, "first": first }))
            .await?;
        Ok(wrapper.issues.nodes)
    }

    /// Fetch issues from a saved Linear view. `view_id` is the slug
    /// at the end of the view's URL (`linear.app/<ws>/view/<id>`).
    pub async fn search_view(&self, view_id: &str, first: u32) -> Result<Vec<Issue>> {
        let q = r#"
            query ViewIssues($id: String!, $first: Int!) {
              customView(id: $id) {
                issues(first: $first) {
                  nodes {
                    id
                    identifier
                    title
                    url
                    priority
                    priorityLabel
                    updatedAt
                    state { name type color }
                    assignee { displayName name }
                    team { key }
                  }
                }
              }
            }
        "#;
        #[derive(Debug, Deserialize)]
        struct Wrapper {
            #[serde(rename = "customView")]
            custom_view: Option<View>,
        }
        #[derive(Debug, Deserialize)]
        struct View {
            issues: IssueConnection,
        }
        #[derive(Debug, Deserialize)]
        struct IssueConnection {
            nodes: Vec<Issue>,
        }
        let wrapper: Wrapper = self
            .query(q, serde_json::json!({ "id": view_id, "first": first }))
            .await?;
        let view = wrapper
            .custom_view
            .ok_or_else(|| anyhow!("Linear: view {view_id} not found or not accessible"))?;
        Ok(view.issues.nodes)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Issue {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub url: String,
    /// Linear uses a 0-4 priority scale: 0 = no priority, 1 = urgent,
    /// 2 = high, 3 = medium, 4 = low.
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default, rename = "priorityLabel")]
    pub priority_label: Option<String>,
    #[serde(default, rename = "updatedAt")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub state: Option<WorkflowState>,
    #[serde(default)]
    pub assignee: Option<User>,
    #[serde(default)]
    pub team: Option<Team>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct WorkflowState {
    pub name: String,
    /// "backlog" | "unstarted" | "started" | "completed" | "canceled" | "triage"
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub color: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct User {
    #[serde(default, rename = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Team {
    pub key: String,
}
