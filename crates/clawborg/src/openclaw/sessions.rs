use crate::types::*;
use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;

/// Read all sessions across all resolved agents
pub fn read_all_sessions(agents: &[ResolvedAgent]) -> Vec<SessionSummary> {
    let mut all_sessions = Vec::new();

    for agent in agents {
        let sessions = read_agent_sessions(agent);
        all_sessions.extend(sessions);
    }

    // Sort by last_active descending
    all_sessions.sort_by(|a, b| {
        b.last_active
            .unwrap_or(0.0)
            .partial_cmp(&a.last_active.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    all_sessions
}

/// Read sessions for a specific agent.
/// Tries multiple formats:
/// 1. Standard OpenClaw: ~/.openclaw/agents/<id>/sessions/*.jsonl (individual JSONL per session)
/// 2. Custom: sessions.json (aggregated JSON map or array)
pub fn read_agent_sessions(agent: &ResolvedAgent) -> Vec<SessionSummary> {
    let sessions_dir = &agent.sessions_dir;

    if !sessions_dir.exists() {
        return Vec::new();
    }

    let mut sessions = Vec::new();

    // ── Strategy 1: Scan for .jsonl files (standard OpenClaw) ──
    if let Ok(entries) = std::fs::read_dir(sessions_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();

            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                // Parse JSONL session transcript
                if let Some(summary) = parse_jsonl_session(&agent.id, &path) {
                    sessions.push(summary);
                }
            }
        }
    }

    // ── Strategy 2: Fall back to sessions.json (custom aggregated format) ──
    if sessions.is_empty() {
        let sessions_file = sessions_dir.join("sessions.json");
        if sessions_file.exists() {
            sessions.extend(parse_sessions_json(&agent.id, &sessions_file));
        }
    }

    sessions
}

/// Parse a single .jsonl session file (standard OpenClaw format).
/// JSONL files contain one JSON object per line — each is a message/event.
/// We extract metadata from the last few lines and aggregate token counts.
fn parse_jsonl_session(agent_id: &str, path: &Path) -> Option<SessionSummary> {
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);

    let session_key = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut last_timestamp: Option<f64> = None;
    let mut total_input_tokens: u64 = 0;
    let mut total_output_tokens: u64 = 0;
    let mut last_model: Option<String> = None;
    let mut line_count: u64 = 0;

    for line_result in reader.lines() {
        let Ok(line) = line_result else { continue };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        line_count += 1;

        // Try to parse each line as JSON
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) {
            // Extract timestamp
            if let Some(ts) = obj.get("timestamp").and_then(|v| v.as_f64()) {
                last_timestamp = Some(ts);
            }
            // Some formats use "ts" instead
            if let Some(ts) = obj.get("ts").and_then(|v| v.as_f64()) {
                last_timestamp = Some(ts);
            }

            // Extract token usage from assistant messages or usage blocks
            if let Some(usage) = obj.get("usage") {
                if let Some(input) = usage.get("input_tokens").and_then(|v| v.as_u64()) {
                    total_input_tokens += input;
                }
                if let Some(output) = usage.get("output_tokens").and_then(|v| v.as_u64()) {
                    total_output_tokens += output;
                }
            }

            // Extract model
            if let Some(model) = obj.get("model").and_then(|v| v.as_str()) {
                last_model = Some(model.to_string());
            }
        }
    }

    // If no lines parsed, skip
    if line_count == 0 {
        return None;
    }

    // Try file mtime as fallback for timestamp
    if last_timestamp.is_none() {
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(modified) = meta.modified() {
                let duration = modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                last_timestamp = Some(duration.as_millis() as f64);
            }
        }
    }

    let channel = extract_channel_from_key(&session_key);
    let status = determine_session_status(last_timestamp);

    Some(SessionSummary {
        agent_id: agent_id.to_string(),
        session_key,
        session_id: None,
        channel,
        label: None,
        last_active: last_timestamp,
        status,
        input_tokens: total_input_tokens,
        output_tokens: total_output_tokens,
        context_tokens: 0,
        model: last_model,
    })
}

/// Parse sessions.json — custom aggregated format.
/// Supports both map format { "key": { ...entry } } and array format [{ ...entry }]
fn parse_sessions_json(agent_id: &str, path: &Path) -> Vec<SessionSummary> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    // Try map format first
    if let Ok(sessions_map) = serde_json::from_str::<HashMap<String, SessionEntry>>(&content) {
        return sessions_map
            .into_iter()
            .map(|(key, entry)| entry_to_summary(agent_id, &key, entry))
            .collect();
    }

    // Try array format
    if let Ok(entries) = serde_json::from_str::<Vec<SessionEntry>>(&content) {
        return entries
            .into_iter()
            .enumerate()
            .map(|(i, e)| entry_to_summary(agent_id, &format!("session_{i}"), e))
            .collect();
    }

    Vec::new()
}

fn entry_to_summary(agent_id: &str, key: &str, entry: SessionEntry) -> SessionSummary {
    let channel = entry
        .origin
        .as_ref()
        .and_then(|o| o.provider.clone())
        .or_else(|| extract_channel_from_key(key));

    let label = entry.origin.as_ref().and_then(|o| o.label.clone());
    let status = determine_session_status(entry.updated_at);

    SessionSummary {
        agent_id: agent_id.to_string(),
        session_key: key.to_string(),
        session_id: entry.session_id,
        channel,
        label,
        last_active: entry.updated_at,
        status,
        input_tokens: entry.input_tokens.unwrap_or(0),
        output_tokens: entry.output_tokens.unwrap_or(0),
        context_tokens: entry.context_tokens.unwrap_or(0),
        model: entry.model,
    }
}

/// Determine session status based on last activity
fn determine_session_status(last_active: Option<f64>) -> SessionStatus {
    let now_ms = chrono::Utc::now().timestamp_millis() as f64;
    match last_active {
        Some(ts) => {
            let age_ms = now_ms - ts;
            let age_hours = age_ms / (1000.0 * 3600.0);
            if age_hours < 0.5 {
                SessionStatus::Active
            } else if age_hours < 24.0 {
                SessionStatus::Idle
            } else {
                SessionStatus::Stale
            }
        }
        None => SessionStatus::Stale,
    }
}

/// Extract channel from OpenClaw session key patterns.
/// Standard format: agent:<agentId>:<channel>:<type>:<id>
/// e.g. "agent:main:whatsapp:dm:+15551230001"
/// e.g. "agent:main:telegram:group:12345"
fn extract_channel_from_key(key: &str) -> Option<String> {
    // Standard OpenClaw key format: agent:<agentId>:<channel>:...
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() >= 3 && parts[0] == "agent" {
        return Some(parts[2].to_string());
    }

    // Fallback: pattern matching for custom keys
    let key_lower = key.to_lowercase();
    if key_lower.contains("telegram") || key_lower.contains("tg") {
        Some("telegram".to_string())
    } else if key_lower.contains("whatsapp") || key_lower.contains("wa") {
        Some("whatsapp".to_string())
    } else if key_lower.contains("discord") {
        Some("discord".to_string())
    } else if key_lower.contains("signal") {
        Some("signal".to_string())
    } else if key_lower.contains("slack") {
        Some("slack".to_string())
    } else if key_lower.contains("cron") || key_lower.contains("heartbeat") {
        Some("cron".to_string())
    } else if key_lower.contains("cli") {
        Some("cli".to_string())
    } else if key_lower.contains("web") {
        Some("web".to_string())
    } else {
        None
    }
}
