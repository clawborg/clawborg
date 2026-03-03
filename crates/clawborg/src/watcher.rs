use crate::types::FileChangeEvent;
use chrono::Utc;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use tokio::sync::broadcast;

/// Start watching the entire openclaw directory for file changes.
/// Uses a single recursive watch instead of per-agent path enumeration,
/// which avoids exhausting OS inotify/kqueue limits on large setups.
pub async fn start_watching(
    openclaw_dir: PathBuf,
    tx: broadcast::Sender<FileChangeEvent>,
) -> anyhow::Result<()> {
    if !openclaw_dir.exists() {
        tracing::warn!("openclaw_dir does not exist, skipping watcher");
        return Ok(());
    }

    let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel::<Event>(512);
    let watch_dir = openclaw_dir.clone();

    let _watcher = tokio::task::spawn_blocking(move || -> anyhow::Result<RecommendedWatcher> {
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let _ = notify_tx.blocking_send(event);
                }
            },
            Config::default(),
        )?;

        watcher.watch(&watch_dir, RecursiveMode::Recursive)?;
        tracing::info!("👁 Watching: {} (recursive)", watch_dir.display());

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
