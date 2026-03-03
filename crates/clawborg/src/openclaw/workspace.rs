use crate::types::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;


/// Build an AgentSummary from a ResolvedAgent
pub fn build_agent_summary(agent: &ResolvedAgent) -> AgentSummary {
    let ws = &agent.workspace_path;
    let files = discover_workspace_files(ws);
    let has_tasks = ws.join("tasks").exists();
    let pending_tasks = if has_tasks {
        count_tasks(ws, "pending")
    } else {
        0
    };
    let session_count = count_sessions(agent);
    let health = check_agent_health(agent);

    AgentSummary {
        id: agent.id.clone(),
        name: resolve_agent_name(agent),
        model: agent.model.clone(),
        workspace_path: agent.workspace_path.to_string_lossy().to_string(),
        file_count: files.len(),
        has_tasks,
        pending_tasks,
        session_count,
        health,
        is_default: agent.is_default,
    }
}

/// Build a full AgentDetail from a ResolvedAgent
pub fn build_agent_detail(agent: &ResolvedAgent) -> AgentDetail {
    let ws = &agent.workspace_path;
    let files = discover_workspace_files(ws);
    let has_tasks = ws.join("tasks").exists();
    let tasks = if has_tasks {
        Some(TaskCounts {
            pending: count_tasks(ws, "pending"),
            approved: count_tasks(ws, "approved"),
            done: count_tasks(ws, "done"),
        })
    } else {
        None
    };
    let directories = discover_directories(ws);
    let health = check_agent_health(agent);

    AgentDetail {
        id: agent.id.clone(),
        name: resolve_agent_name(agent),
        model: agent.model.clone(),
        fallbacks: agent.fallbacks.clone(),
        workspace_path: agent.workspace_path.to_string_lossy().to_string(),
        files,
        tasks,
        directories,
        health,
        is_default: agent.is_default,
    }
}

/// Discover ALL .md files in workspace root (not hardcoded)
fn discover_workspace_files(workspace_path: &Path) -> HashMap<String, FileInfo> {
    let mut files = HashMap::new();

    if !workspace_path.exists() {
        return files;
    }

    let Ok(entries) = std::fs::read_dir(workspace_path) else {
        return files;
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "md" || ext == "json" || ext == "jsonl" || ext == "txt" {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    files.insert(fname, get_file_info(&path));
                }
            }
        }
    }

    files
}

/// Discover subdirectories in workspace (memory/, skills/, tasks/, etc.)
fn discover_directories(workspace_path: &Path) -> Vec<String> {
    let mut dirs = Vec::new();

    if !workspace_path.exists() {
        return dirs;
    }

    let Ok(entries) = std::fs::read_dir(workspace_path) else {
        return dirs;
    };

    for entry in entries.filter_map(|e| e.ok()) {
        if entry.path().is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden directories
            if !name.starts_with('.') {
                dirs.push(name);
            }
        }
    }

    dirs.sort();
    dirs
}

/// Get metadata about a file
fn get_file_info(path: &Path) -> FileInfo {
    match std::fs::metadata(path) {
        Ok(meta) => {
            let size = meta.len();
            let modified = meta
                .modified()
                .ok()
                .map(DateTime::<Utc>::from);
            FileInfo {
                exists: true,
                size_bytes: size,
                is_empty: size < 50,
                modified,
            }
        }
        Err(_) => FileInfo {
            exists: false,
            size_bytes: 0,
            is_empty: true,
            modified: None,
        },
    }
}

/// Count task files in a task subdirectory
fn count_tasks(workspace_path: &Path, folder: &str) -> usize {
    let task_dir = workspace_path.join("tasks").join(folder);
    if !task_dir.exists() {
        return 0;
    }
    std::fs::read_dir(&task_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "md" || ext == "json")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

/// Count sessions for an agent
fn count_sessions(agent: &ResolvedAgent) -> usize {
    let sessions_dir = &agent.sessions_dir;
    if !sessions_dir.exists() {
        return 0;
    }
    std::fs::read_dir(sessions_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "jsonl" || ext == "json")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

/// Try to resolve a display name for an agent.
/// Priority: config name > IDENTITY.md > AGENTS.md first heading > None
fn resolve_agent_name(agent: &ResolvedAgent) -> Option<String> {
    // 1. Config provides a name
    if agent.name.is_some() {
        return agent.name.clone();
    }

    // 2. Try IDENTITY.md
    let identity_path = agent.workspace_path.join("IDENTITY.md");
    if let Ok(content) = std::fs::read_to_string(&identity_path) {
        // Look for "Name:" or first # heading
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(name) = trimmed.strip_prefix("Name:") {
                let name = name.trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
            if let Some(heading) = trimmed.strip_prefix("# ") {
                let heading = heading.trim();
                if !heading.is_empty() {
                    return Some(heading.to_string());
                }
            }
        }
    }

    None
}

/// Run health checks on a single agent.
/// Uses generic detection rather than hardcoded file expectations.
pub fn check_agent_health(agent: &ResolvedAgent) -> AgentHealth {
    let mut issues = Vec::new();
    let ws = &agent.workspace_path;

    // ── Check 1: Workspace exists? ──
    if !ws.exists() {
        issues.push(HealthIssue {
            severity: IssueSeverity::Critical,
            message: format!(
                "Workspace directory not found: {}",
                ws.to_string_lossy()
            ),
            file: None,
        });
        return AgentHealth {
            status: HealthStatus::Critical,
            issues,
        };
    }

    // ── Check 2: Has any bootstrap/instruction files? ──
    // Standard OpenClaw uses AGENTS.md + SOUL.md. Some setups use variants.
    let has_agents_md = ws.join("AGENTS.md").exists();
    let has_soul_md = ws.join("SOUL.md").exists();
    let has_any_instruction = has_agents_md || has_soul_md;

    if !has_any_instruction {
        issues.push(HealthIssue {
            severity: IssueSeverity::Critical,
            message: "No instruction files found (AGENTS.md or SOUL.md)".to_string(),
            file: None,
        });
    }

    // ── Check 3: Empty instruction files ──
    // Standard OpenClaw bootstrap files that should not be empty
    let instruction_files = [
        "AGENTS.md", "SOUL.md",
    ];
    for fname in instruction_files {
        let fpath = ws.join(fname);
        if fpath.exists() && file_is_empty(&fpath) {
            issues.push(HealthIssue {
                severity: IssueSeverity::Critical,
                message: format!("{fname} exists but is empty (<50 bytes)"),
                file: Some(fname.to_string()),
            });
        }
    }

    // ── Check 4: Identity check ──
    let has_identity = ws.join("IDENTITY.md").exists();
    if !has_identity && agent.name.is_none() {
        // Only warn if no identity source at all
        issues.push(HealthIssue {
            severity: IssueSeverity::Info,
            message: "No IDENTITY.md and no name in config".to_string(),
            file: Some("IDENTITY.md".to_string()),
        });
    }

    // ── Check 5: Memory files ──
    let has_memory_md = ws.join("MEMORY.md").exists() || ws.join("memory.md").exists();
    let has_memory_dir = ws.join("memory").exists();
    if !has_memory_md && !has_memory_dir {
        issues.push(HealthIssue {
            severity: IssueSeverity::Info,
            message: "No memory files found (MEMORY.md or memory/ directory)".to_string(),
            file: None,
        });
    }

    // ── Check 6: Stale pending tasks (if task queue exists) ──
    let pending_dir = ws.join("tasks").join("pending");
    if pending_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&pending_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        let age = std::time::SystemTime::now()
                            .duration_since(modified)
                            .unwrap_or_default();
                        if age.as_secs() > 48 * 3600 {
                            issues.push(HealthIssue {
                                severity: IssueSeverity::Warning,
                                message: format!(
                                    "Stale pending task: {} (>48h old)",
                                    entry.file_name().to_string_lossy()
                                ),
                                file: Some(
                                    entry.file_name().to_string_lossy().to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    // ── Check 7: Session health ──
    if agent.sessions_dir.exists() {
        let session_count = count_sessions(agent);
        if session_count == 0 {
            issues.push(HealthIssue {
                severity: IssueSeverity::Info,
                message: "No session files found".to_string(),
                file: None,
            });
        }
    }

    // ── Determine overall status ──
    let status = if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Critical)) {
        HealthStatus::Critical
    } else if issues.iter().any(|i| matches!(i.severity, IssueSeverity::Warning)) {
        HealthStatus::Warning
    } else {
        HealthStatus::Healthy
    };

    AgentHealth { status, issues }
}

fn file_is_empty(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() < 50)
        .unwrap_or(true)
}

/// Read a file from an agent's workspace
pub fn read_workspace_file(
    workspace_path: &Path,
    filename: &str,
) -> anyhow::Result<String> {
    // Security: prevent path traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        anyhow::bail!("Invalid filename: path traversal not allowed");
    }

    let file_path = workspace_path.join(filename);
    if !file_path.exists() {
        anyhow::bail!("File not found: {filename}");
    }
    Ok(std::fs::read_to_string(file_path)?)
}

/// Write content to a workspace file.
/// By default, only .md files in the workspace root are writable.
/// Creates auto-backup before overwrite.
pub fn write_workspace_file(
    workspace_path: &Path,
    filename: &str,
    content: &str,
) -> anyhow::Result<()> {
    // Security: prevent path traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        anyhow::bail!("Invalid filename: path traversal not allowed");
    }

    // Only allow .md files
    if !filename.ends_with(".md") {
        anyhow::bail!("Only .md files are writable from ClawBorg");
    }

    let file_path = workspace_path.join(filename);

    // Auto-backup before overwrite
    if file_path.exists() {
        let backup_path = file_path.with_extension("md.bak");
        std::fs::copy(&file_path, &backup_path)?;
    }

    std::fs::write(&file_path, content)?;
    Ok(())
}
