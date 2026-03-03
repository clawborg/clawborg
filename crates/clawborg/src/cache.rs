use crate::types::{CronJobEntry, CronJobsFile, ResolvedAgent, SessionEntry};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared in-memory cache for expensive disk reads.
pub type AppCache = Arc<RwLock<DataCache>>;

/// In-memory data cache. Loaded at startup and invalidated by the file watcher.
#[derive(Default)]
pub struct DataCache {
    /// sessions[agent_id] = flat sessions map (session_key → SessionEntry)
    pub sessions: HashMap<String, HashMap<String, SessionEntry>>,
    /// All cron job definitions from cron/jobs.json
    pub cron_jobs: Vec<CronJobEntry>,
}

/// Load initial cache from disk. Called once at startup.
/// Silently skips agents that have no sessions.json or unreadable data.
pub fn load_cache(agents: &[ResolvedAgent], openclaw_dir: &Path) -> DataCache {
    let mut cache = DataCache::default();

    for agent in agents {
        let sessions_json = agent.sessions_dir.join("sessions.json");
        if let Ok(content) = std::fs::read_to_string(&sessions_json) {
            match serde_json::from_str::<HashMap<String, SessionEntry>>(&content) {
                Ok(map) => {
                    cache.sessions.insert(agent.id.clone(), map);
                }
                Err(e) => {
                    eprintln!(
                        "[clawborg] Failed to parse sessions.json for agent {}: {e}",
                        agent.id
                    );
                }
            }
        }
    }

    let jobs_path = openclaw_dir.join("cron").join("jobs.json");
    if let Ok(content) = std::fs::read_to_string(&jobs_path) {
        match serde_json::from_str::<CronJobsFile>(&content) {
            Ok(file) => {
                cache.cron_jobs = file.jobs;
            }
            Err(e) => {
                eprintln!("[clawborg] Failed to parse cron/jobs.json for cache: {e}");
            }
        }
    }

    tracing::info!(
        "💾 Cache loaded: {} agents with sessions, {} cron jobs",
        cache.sessions.len(),
        cache.cron_jobs.len()
    );

    cache
}
