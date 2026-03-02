use crate::openclaw::config;
use crate::types::FileChangeEvent;
use chrono::Utc;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::sync::broadcast;

/// Start watching all resolved workspace and session paths for file changes
pub async fn start_watching(
    openclaw_dir: PathBuf,
    tx: broadcast::Sender<FileChangeEvent>,
) -> anyhow::Result<()> {
    // Resolve all paths to watch from config
    let watch_paths = discover_watch_paths(&openclaw_dir);

    if watch_paths.is_empty() {
        tracing::warn!("No directories to watch");
        return Ok(());
    }

    let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel::<Event>(256);

    let _watcher = tokio::task::spawn_blocking(move || -> anyhow::Result<RecommendedWatcher> {
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let _ = notify_tx.blocking_send(event);
                }
            },
            Config::default(),
        )?;

        for path in &watch_paths {
            if path.exists() {
                match watcher.watch(path, RecursiveMode::Recursive) {
                    Ok(_) => tracing::info!("👁 Watching: {}", path.display()),
                    Err(e) => tracing::warn!("Failed to watch {}: {e}", path.display()),
                }
            }
        }

        Ok(watcher)
    })
    .await??;

    // Process events
    while let Some(event) = notify_rx.recv().await {
        let event_type = match event.kind {
            EventKind::Create(_) => "created",
            EventKind::Modify(_) => "modified",
            EventKind::Remove(_) => "removed",
            _ => continue,
        };

        for path in &event.paths {
            let (agent_id, file_name) = extract_agent_info(path);

            let change_event = FileChangeEvent {
                event_type: event_type.to_string(),
                path: path.to_string_lossy().to_string(),
                agent_id,
                file_name,
                timestamp: Utc::now(),
            };

            let _ = tx.send(change_event);
        }
    }

    Ok(())
}

/// Discover all directories that need watching.
/// Includes each agent's workspace + state/sessions dir.
fn discover_watch_paths(openclaw_dir: &Path) -> Vec<PathBuf> {
    let mut paths = HashSet::new();

    // Always watch the openclaw.json itself
    paths.insert(openclaw_dir.to_path_buf());

    // Try to resolve agents from config
    if let Ok(cfg) = config::read_config(openclaw_dir) {
        let agents = config::resolve_agents(&cfg, openclaw_dir);
        for agent in &agents {
            if agent.workspace_path.exists() {
                paths.insert(agent.workspace_path.clone());
            }
            if agent.sessions_dir.exists() {
                paths.insert(agent.sessions_dir.clone());
            }
        }
    }

    // Also watch common fallback directories if they exist
    let fallbacks = [
        openclaw_dir.join("workspaces"),
        openclaw_dir.join("agents"),
        openclaw_dir.join("workspace"),
    ];
    for dir in fallbacks {
        if dir.exists() {
            paths.insert(dir);
        }
    }

    paths.into_iter().collect()
}

/// Extract agent ID and filename from a file change event path.
/// Handles various path patterns:
///   ~/.openclaw/workspace-<agentId>/FILE.md
///   ~/.openclaw/workspaces/<agentId>/FILE.md
///   ~/.openclaw/agents/<agentId>/sessions/FILE.jsonl
fn extract_agent_info(path: &Path) -> (Option<String>, Option<String>) {
    let path_str = path.to_string_lossy();
    let file_name = path.file_name().map(|f| f.to_string_lossy().to_string());

    // Pattern: /workspaces/<agent_id>/...
    if let Some(idx) = path_str.find("/workspaces/") {
        let rest = &path_str[idx + "/workspaces/".len()..];
        if let Some(slash) = rest.find('/') {
            return (Some(rest[..slash].to_string()), file_name);
        }
        return (Some(rest.to_string()), file_name);
    }

    // Pattern: /workspace-<agent_id>/...
    if let Some(idx) = path_str.find("/workspace-") {
        let rest = &path_str[idx + "/workspace-".len()..];
        if let Some(slash) = rest.find('/') {
            return (Some(rest[..slash].to_string()), file_name);
        }
        return (Some(rest.to_string()), file_name);
    }

    // Pattern: /agents/<agent_id>/sessions/...
    if let Some(idx) = path_str.find("/agents/") {
        let rest = &path_str[idx + "/agents/".len()..];
        if let Some(slash) = rest.find('/') {
            return (Some(rest[..slash].to_string()), file_name);
        }
    }

    (None, file_name)
}
