use log::error;
use serde::Serialize;

/// HTTP client for communicating with the running daemon.
pub struct DaemonClient {
    base_url: String,
    client: reqwest::Client,
    json_output: bool,
    auth_token: Option<String>,
}

impl DaemonClient {
    pub fn new(port: u16, json_output: bool) -> Self {
        // Load auth token so CLI subcommands work when auth is enabled
        let auth_token = crate::auth::load_token()
            .ok()
            .flatten();
        Self {
            base_url: format!("http://127.0.0.1:{port}/api"),
            client: reqwest::Client::new(),
            json_output,
            auth_token,
        }
    }

    /// GET /api/health + GET /api/agents/active, display formatted status.
    pub async fn status(&self) -> i32 {
        let health = match self.get::<serde_json::Value>("health").await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Failed to connect to daemon: {e}");
                return 1;
            }
        };

        let active_agents = self
            .get::<Vec<serde_json::Value>>("agents/active")
            .await
            .unwrap_or_default();

        let workflows = self
            .get::<Vec<serde_json::Value>>("workflows")
            .await
            .unwrap_or_default();

        let runs = self
            .get::<Vec<serde_json::Value>>("runs")
            .await
            .unwrap_or_default();

        if self.json_output {
            let output = serde_json::json!({
                "health": health,
                "active_agents": active_agents.len(),
                "workflows": workflows.len(),
                "active_runs": runs.len(),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
        } else {
            let version = health.get("version").and_then(|v| v.as_str()).unwrap_or("unknown");
            let pid = health.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
            let workspace = health.get("workspace-root").and_then(|v| v.as_str()).unwrap_or("unknown");

            println!("Branchdeck Daemon");
            println!("  Version:      {version}");
            println!("  PID:          {pid}");
            println!("  Workspace:    {workspace}");
            println!("  Workflows:    {}", workflows.len());
            println!("  Active runs:  {}", runs.len());
            println!("  Active agents: {}", active_agents.len());
        }
        0
    }

    /// POST /api/runs to trigger a workflow.
    pub async fn trigger(
        &self,
        workflow: &str,
        task_path: Option<&str>,
        worktree_path: Option<&str>,
    ) -> i32 {
        let task = task_path.unwrap_or(workflow);

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct CreateRunRequest {
            task_path: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            worktree_path: Option<String>,
        }

        let body = CreateRunRequest {
            task_path: task.to_string(),
            worktree_path: worktree_path.map(String::from),
        };

        match self.post::<_, serde_json::Value>("runs", &body).await {
            Ok(resp) => {
                if self.json_output {
                    println!("{}", serde_json::to_string_pretty(&resp).unwrap_or_default());
                } else {
                    let session = resp
                        .get("sessionId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("(pending)");
                    let status = resp
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    println!("Triggered workflow: {workflow}");
                    println!("  Session:  {session}");
                    println!("  Status:   {status}");
                }
                0
            }
            Err(e) => {
                eprintln!("Failed to trigger workflow: {e}");
                1
            }
        }
    }

    /// GET /api/runs, display run list.
    pub async fn list_runs(&self) -> i32 {
        match self.get::<Vec<serde_json::Value>>("runs").await {
            Ok(runs) => {
                if self.json_output {
                    println!("{}", serde_json::to_string_pretty(&runs).unwrap_or_default());
                } else if runs.is_empty() {
                    println!("No runs.");
                } else {
                    println!("{:<24} {:<12} {:<24}", "SESSION", "STATUS", "STARTED");
                    println!("{}", "-".repeat(62));
                    for run in &runs {
                        let session = run
                            .get("sessionId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("(none)");
                        let status = run
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let started = run
                            .get("startedAt")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        println!("{session:<24} {status:<12} {started:<24}");
                    }
                }
                0
            }
            Err(e) => {
                eprintln!("Failed to list runs: {e}");
                1
            }
        }
    }

    /// POST /api/runs/{id}/cancel to cancel a run.
    pub async fn cancel_run(&self, id: &str) -> i32 {
        let path = format!("runs/{id}/cancel");
        match self.post::<_, serde_json::Value>(&path, &serde_json::json!({})).await {
            Ok(resp) => {
                if self.json_output {
                    println!("{}", serde_json::to_string_pretty(&resp).unwrap_or_default());
                } else {
                    println!("Cancelled run: {id}");
                }
                0
            }
            Err(e) => {
                eprintln!("Failed to cancel run {id}: {e}");
                1
            }
        }
    }

    /// Stub for update checking.
    pub fn update(&self) -> i32 {
        if self.json_output {
            let output = serde_json::json!({
                "status": "not-implemented",
                "message": "Update checking not yet implemented",
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
        } else {
            println!("Update checking not yet implemented.");
        }
        0
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let url = format!("{}/{path}", self.base_url);
        let mut req = self.client.get(&url);
        if let Some(token) = &self.auth_token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        let resp = req
            .send()
            .await
            .map_err(|e| {
                error!("GET {url} failed: {e}");
                format!("request failed: {e}")
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!("GET {url} returned {status}: {body}");
            return Err(format!("HTTP {status}: {body}"));
        }

        resp.json::<T>().await.map_err(|e| {
            error!("Failed to parse response from {url}: {e}");
            format!("parse error: {e}")
        })
    }

    async fn post<B: Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let url = format!("{}/{path}", self.base_url);
        let mut req = self.client.post(&url).json(body);
        if let Some(token) = &self.auth_token {
            req = req.header("Authorization", format!("Bearer {token}"));
        }
        let resp = req
            .send()
            .await
            .map_err(|e| {
                error!("POST {url} failed: {e}");
                format!("request failed: {e}")
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            error!("POST {url} returned {status}: {body}");
            return Err(format!("HTTP {status}: {body}"));
        }

        resp.json::<T>().await.map_err(|e| {
            error!("Failed to parse response from {url}: {e}");
            format!("parse error: {e}")
        })
    }
}
