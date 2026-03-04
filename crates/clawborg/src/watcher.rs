use crate::cache::AppCache;
use crate::types::{CronJobsFile, FileChangeEvent, SessionEntry};
use chrono::Utc;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::sync::broadcast;
use tokio::time::Duration;

// ─── Debounce window ─────────────────────────────────────────────────────────
//
// Events are accumulated in a pending set and flushed every DEBOUNCE_MS.
// This decouples event receipt from cache I/O, preventing two problems:
//
//   1. Event loop blocking: previously, reload_agent_sessions was awaited
//      INLINE inside the notify_rx receive loop. During heavy agent activity
//      this blocked the loop, filled the 512-event channel, caused
//      blocking_send to stall the FSEvents/kqueue OS thread, and eventually
//      killed the watcher silently on macOS.
//
//   2. Write stampede: rapid events for the same file previously triggered
//      N concurrent cache.write().await calls. With tokio::sync::RwLock,
//      pending write-lock waiters block NEW readers — so API handlers
//      (cache.read().await) were starved until all writers cleared.
//      With debouncing, the same file collapses to a single reload per window.

const DEBOUNCE_MS: u64 = 500;

// ─── Internal reload tracking ────────────────────────────────────────────────

#[derive(Hash, Eq, PartialEq, Debug)]
enum PendingReload {
    /// Reload sessions for a specific agent (path = sessions.json)
    AgentSessions { agent_id: String, path: PathBuf },
    /// Reload cron jobs (path = jobs.json)
    CronJobs { path: PathBuf },
}

/// Start watching the entire openclaw directory for file changes.
///
/// Returns `Ok(())` if the underlying watcher exits cleanly, or `Err` if setup
/// fails. The caller (server.rs) is responsible for restarting on exit — this
/// function is designed to be supervised.
pub async fn start_watching(
    openclaw_dir: PathBuf,
    tx: broadcast::Sender<FileChangeEvent>,
    cache: AppCache,
) -> anyhow::Result<()> {
    if !openclaw_dir.exists() {
        tracing::warn!("openclaw_dir does not exist, skipping watcher");
        return Ok(());
    }

    let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel::<Event>(512);
    let watch_dir = openclaw_dir.clone();

    // Set up the OS-level watcher on a blocking thread so it doesn't hold a
    // tokio worker during setup. The returned RecommendedWatcher is kept alive
    // by _watcher for the duration of start_watching.
    let _watcher = tokio::task::spawn_blocking(move || -> anyhow::Result<RecommendedWatcher> {
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| match result {
                Ok(event) => {
                    // blocking_send is safe here: the channel is 512-deep and
                    // events are consumed quickly by the debounce loop below.
                    // If the channel is full we drop the event rather than
                    // blocking the FSEvents callback thread indefinitely.
                    if notify_tx.try_send(event).is_err() {
                        tracing::warn!("File watcher event channel full — event dropped");
                    }
                }
                Err(e) => {
                    tracing::warn!("File watcher notify error: {e}");
                }
            },
            Config::default(),
        )?;

        watcher.watch(&watch_dir, RecursiveMode::Recursive)?;
        tracing::info!("👁 Watching: {} (recursive)", watch_dir.display());
        Ok(watcher)
    })
    .await??;

    // ── Debounce loop ────────────────────────────────────────────────────────
    //
    // We use tokio::select! between:
    //   - notify_rx.recv()   → accumulate events into `pending` (no I/O)
    //   - debounce_interval  → flush pending set with actual disk reads
    //
    // This keeps the event-receive path non-blocking and collapses N rapid
    // writes to the same file into a single cache reload.

    let mut pending: HashSet<PendingReload> = HashSet::new();
    let mut debounce_interval =
        tokio::time::interval(Duration::from_millis(DEBOUNCE_MS));
    debounce_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    debounce_interval.tick().await; // discard the immediate first tick

    loop {
        tokio::select! {
            biased; // prioritise draining events over flushing

            event = notify_rx.recv() => {
                let Some(event) = event else {
                    // All senders dropped — watcher has been released
                    tracing::warn!("File watcher channel closed; watcher exited");
                    break;
                };

                let event_type = match event.kind {
                    EventKind::Create(_) => "created",
                    EventKind::Modify(_) => "modified",
                    EventKind::Remove(_) => "removed",
                    _ => continue,
                };

                for path in &event.paths {
                    let (agent_id, file_name) = extract_agent_info(path);
                    let fname = file_name.as_deref().unwrap_or("");
                    let path_str = path.to_string_lossy().to_string();

                    // Queue the appropriate cache reload (deduplicates by key)
                    if fname == "sessions.json" && path_str.contains("/sessions/") {
                        if let Some(aid) = agent_id.clone() {
                            pending.insert(PendingReload::AgentSessions {
                                agent_id: aid,
                                path: path.clone(),
                            });
                        }
                    } else if fname == "jobs.json" && path_str.contains("/cron/") {
                        pending.insert(PendingReload::CronJobs { path: path.clone() });
                    }

                    // Broadcast the raw event immediately (lightweight, no I/O)
                    let change_event = FileChangeEvent {
                        event_type: event_type.to_string(),
                        path: path_str,
                        agent_id,
                        file_name,
                        timestamp: Utc::now(),
                    };
                    let _ = tx.send(change_event);
                }
            }

            _ = debounce_interval.tick() => {
                if pending.is_empty() {
                    continue;
                }
                // Drain pending and perform cache reloads sequentially.
                // Each reload holds the write lock for a minimal duration
                // (the actual file read finishes before the lock is taken).
                let reloads: Vec<PendingReload> = pending.drain().collect();
                tracing::debug!("Flushing {} pending cache reload(s)", reloads.len());
                for reload in reloads {
                    match reload {
                        PendingReload::AgentSessions { agent_id, path } => {
                            reload_agent_sessions(&path, &agent_id, &cache).await;
                        }
                        PendingReload::CronJobs { path } => {
                            reload_cron_jobs(&path, &cache).await;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Reload sessions for a single agent into the cache.
///
/// File I/O completes before the write lock is acquired, keeping the
/// critical section (lock held) as short as possible.
async fn reload_agent_sessions(path: &Path, agent_id: &str, cache: &AppCache) {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => match serde_json::from_str::<HashMap<String, SessionEntry>>(&content) {
            Ok(map) => {
                let mut c = cache.write().await;
                c.sessions.insert(agent_id.to_string(), map);
                tracing::debug!("💾 Cache refreshed: sessions for agent {agent_id}");
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse sessions.json for agent {agent_id}: {e}"
                );
            }
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                let mut c = cache.write().await;
                c.sessions.remove(agent_id);
                tracing::debug!("💾 Cache cleared: sessions for agent {agent_id}");
            } else {
                tracing::warn!(
                    "Failed to read sessions.json for agent {agent_id}: {e}"
                );
            }
        }
    }
}

/// Reload cron jobs from disk into the cache.
///
/// File I/O completes before the write lock is acquired.
async fn reload_cron_jobs(path: &Path, cache: &AppCache) {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => match serde_json::from_str::<CronJobsFile>(&content) {
            Ok(file) => {
                let count = file.jobs.len();
                let mut c = cache.write().await;
                c.cron_jobs = file.jobs;
                tracing::debug!("💾 Cache refreshed: {count} cron jobs");
            }
            Err(e) => {
                tracing::warn!("Failed to parse cron/jobs.json: {e}");
            }
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                let mut c = cache.write().await;
                c.cron_jobs.clear();
                tracing::debug!("💾 Cache cleared: cron jobs");
            } else {
                tracing::warn!("Failed to read cron/jobs.json: {e}");
            }
        }
    }
}

/// Extract agent ID and filename from a file change event path.
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
