use crate::types::*;
use std::path::{Path, PathBuf};

/// Read and parse openclaw.json
pub fn read_config(openclaw_dir: &Path) -> anyhow::Result<OpenClawConfig> {
    let config_path = openclaw_dir.join("openclaw.json");
    if !config_path.exists() {
        anyhow::bail!(
            "openclaw.json not found at {}\n\n\
            ClawBorg needs an OpenClaw installation to read from.\n\
            \n\
            Options:\n\
              clawborg --dir /path/to/.openclaw      # point to your install\n\
              clawborg --dir ./fixtures/mock-openclaw # use mock data\n\
            \n\
            Set OPENCLAW_DIR to make it permanent:\n\
              export OPENCLAW_DIR=/path/to/.openclaw",
            config_path.display()
        );
    }

    let content = std::fs::read_to_string(&config_path)?;
    // Strip JS-style comments for JSON5 compatibility
    let cleaned = strip_json_comments(&content);
    let config: OpenClawConfig = serde_json::from_str(&cleaned)
        .map_err(|e| anyhow::anyhow!("Failed to parse openclaw.json: {e}"))?;
    Ok(config)
}

/// Resolve all agents from config into ResolvedAgent structs with full paths.
///
/// Handles three config patterns:
/// 1. Multi-agent: agents.list[] — each with own workspace
/// 2. Single with defaults: agents.defaults.workspace — one "main" agent
/// 3. Singular form: agent.workspace — one "main" agent
///
/// Falls back to ~/.openclaw/workspace if no workspace specified.
pub fn resolve_agents(
    config: &OpenClawConfig,
    openclaw_dir: &Path,
) -> Vec<ResolvedAgent> {
    let agents_state_dir = openclaw_dir.join("agents");
    let default_workspace = default_workspace_path(openclaw_dir);

    // Case 1: Multi-agent — agents.list[]
    if let Some(agents_cfg) = &config.agents {
        if let Some(list) = &agents_cfg.list {
            if !list.is_empty() {
                let default_ws = agents_cfg
                    .defaults
                    .as_ref()
                    .and_then(|d| d.workspace.as_ref())
                    .map(|ws| resolve_tilde(ws, openclaw_dir))
                    .unwrap_or_else(|| default_workspace.clone());

                let default_model = agents_cfg
                    .defaults
                    .as_ref()
                    .and_then(|d| d.model.as_ref());

                return list
                    .iter()
                    .enumerate()
                    .map(|(i, entry)| {
                        let ws_path = entry
                            .workspace
                            .as_ref()
                            .map(|ws| resolve_tilde(ws, openclaw_dir))
                            .unwrap_or_else(|| default_ws.clone());

                        let model = entry
                            .model
                            .as_ref()
                            .or(default_model);

                        let state_dir = agents_state_dir.join(&entry.id);
                        let sessions_dir = state_dir.join("sessions");

                        let is_default = entry.is_default.unwrap_or(i == 0);

                        ResolvedAgent {
                            id: entry.id.clone(),
                            name: entry.name.clone(),
                            model: model.and_then(|m| m.primary.clone()),
                            fallbacks: model
                                .and_then(|m| m.fallbacks.clone())
                                .unwrap_or_default(),
                            workspace_path: ws_path,
                            state_dir,
                            sessions_dir,
                            is_default,
                        }
                    })
                    .collect();
            }
        }

        // Case 2: agents.defaults.workspace only (single agent, no list)
        if let Some(defaults) = &agents_cfg.defaults {
            if defaults.workspace.is_some() {
                let ws_path = defaults
                    .workspace
                    .as_ref()
                    .map(|ws| resolve_tilde(ws, openclaw_dir))
                    .unwrap_or_else(|| default_workspace.clone());

                let agent_id = "main".to_string();
                return vec![single_agent(
                    &agent_id,
                    &ws_path,
                    defaults.model.as_ref(),
                    config,
                    &agents_state_dir,
                )];
            }
        }
    }

    // Case 3: agent.workspace (singular form)
    if let Some(agent_cfg) = &config.agent {
        let ws_path = agent_cfg
            .workspace
            .as_ref()
            .map(|ws| resolve_tilde(ws, openclaw_dir))
            .unwrap_or_else(|| default_workspace.clone());

        let agent_id = "main".to_string();
        return vec![single_agent(
            &agent_id,
            &ws_path,
            agent_cfg.model.as_ref(),
            config,
            &agents_state_dir,
        )];
    }

    // Fallback: try to detect agents from filesystem
    // Scan ~/.openclaw/agents/ for directories with sessions
    detect_agents_from_filesystem(openclaw_dir, &default_workspace)
}

/// Build a single "main" agent
fn single_agent(
    id: &str,
    workspace_path: &Path,
    model: Option<&AgentModel>,
    config: &OpenClawConfig,
    agents_state_dir: &Path,
) -> ResolvedAgent {
    let name = config
        .identity
        .as_ref()
        .and_then(|i| i.name.clone());

    let state_dir = agents_state_dir.join(id);
    let sessions_dir = state_dir.join("sessions");

    ResolvedAgent {
        id: id.to_string(),
        name,
        model: model.and_then(|m| m.primary.clone()),
        fallbacks: model
            .and_then(|m| m.fallbacks.clone())
            .unwrap_or_default(),
        workspace_path: workspace_path.to_path_buf(),
        state_dir,
        sessions_dir,
        is_default: true,
    }
}

/// Last resort: scan filesystem for agent directories
fn detect_agents_from_filesystem(
    openclaw_dir: &Path,
    default_workspace: &Path,
) -> Vec<ResolvedAgent> {
    let agents_state_dir = openclaw_dir.join("agents");
    let mut agents = Vec::new();

    // Check if there's a main workspace
    if default_workspace.exists() {
        let state_dir = agents_state_dir.join("main");
        agents.push(ResolvedAgent {
            id: "main".to_string(),
            name: None,
            model: None,
            fallbacks: vec![],
            workspace_path: default_workspace.to_path_buf(),
            state_dir: state_dir.clone(),
            sessions_dir: state_dir.join("sessions"),
            is_default: true,
        });
    }

    // Scan for additional agent state dirs
    if let Ok(entries) = std::fs::read_dir(&agents_state_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let id = entry.file_name().to_string_lossy().to_string();
            if id == "main" || !entry.path().is_dir() {
                continue;
            }
            let sessions_dir = entry.path().join("sessions");
            if sessions_dir.exists() {
                agents.push(ResolvedAgent {
                    id: id.clone(),
                    name: None,
                    model: None,
                    fallbacks: vec![],
                    workspace_path: openclaw_dir.join(format!("workspace-{id}")),
                    state_dir: entry.path(),
                    sessions_dir,
                    is_default: false,
                });
            }
        }
    }

    if agents.is_empty() {
        tracing::warn!("No agents discovered from config or filesystem");
    }

    agents
}

/// Resolve ~ and relative paths
fn resolve_tilde(path_str: &str, openclaw_dir: &Path) -> PathBuf {
    if path_str.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path_str[2..]);
        }
    }
    if path_str.starts_with("~/.openclaw/") {
        // Already has the prefix, resolve from home
        if let Some(home) = dirs::home_dir() {
            return home.join(&path_str[2..]);
        }
    }

    let path = PathBuf::from(path_str);
    if path.is_absolute() {
        path
    } else {
        // Relative paths resolve from openclaw_dir
        openclaw_dir.join(path_str)
    }
}

/// Default workspace path: ~/.openclaw/workspace
/// Or ~/.openclaw/workspace-<profile> if OPENCLAW_PROFILE is set
fn default_workspace_path(openclaw_dir: &Path) -> PathBuf {
    if let Ok(profile) = std::env::var("OPENCLAW_PROFILE") {
        if profile != "default" && !profile.is_empty() {
            return openclaw_dir.join(format!("workspace-{profile}"));
        }
    }
    openclaw_dir.join("workspace")
}

/// Find a resolved agent by ID
pub fn find_agent<'a>(agents: &'a [ResolvedAgent], id: &str) -> Option<&'a ResolvedAgent> {
    agents.iter().find(|a| a.id == id)
}

/// Get a redacted config (strip tokens, API keys, passwords)
pub fn read_config_redacted(openclaw_dir: &Path) -> anyhow::Result<serde_json::Value> {
    let config_path = openclaw_dir.join("openclaw.json");
    let content = std::fs::read_to_string(&config_path)?;
    let cleaned = strip_json_comments(&content);
    let mut value: serde_json::Value = serde_json::from_str(&cleaned)?;
    redact_value(&mut value);
    Ok(value)
}

fn redact_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                let key_lower = key.to_lowercase();
                if key_lower.contains("token")
                    || key_lower.contains("apikey")
                    || key_lower.contains("api_key")
                    || key_lower.contains("secret")
                    || key_lower.contains("password")
                    || key_lower.contains("credential")
                {
                    if val.is_string() {
                        *val = serde_json::Value::String("***REDACTED***".to_string());
                    }
                } else {
                    redact_value(val);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                redact_value(val);
            }
        }
        _ => {}
    }
}

/// Strip single-line (//) and multi-line (/* */) comments from JSON5-ish content
fn strip_json_comments(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escape_next = false;

    while let Some(c) = chars.next() {
        if escape_next {
            result.push(c);
            escape_next = false;
            continue;
        }

        if in_string {
            result.push(c);
            if c == '\\' {
                escape_next = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        if c == '"' {
            in_string = true;
            result.push(c);
            continue;
        }

        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    // Skip until end of line
                    for ch in chars.by_ref() {
                        if ch == '\n' {
                            result.push('\n');
                            break;
                        }
                    }
                }
                Some('*') => {
                    chars.next(); // consume *
                    let mut prev = ' ';
                    for ch in chars.by_ref() {
                        if prev == '*' && ch == '/' {
                            break;
                        }
                        if ch == '\n' {
                            result.push('\n');
                        }
                        prev = ch;
                    }
                }
                _ => result.push(c),
            }
        } else {
            result.push(c);
        }
    }

    result
}
