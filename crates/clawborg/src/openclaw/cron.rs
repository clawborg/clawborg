use crate::types::*;
use std::path::Path;

/// Build cron entries from config with status info
pub fn build_cron_list(config: &OpenClawConfig, agents: &[ResolvedAgent]) -> Vec<CronEntry> {
    let crons = match &config.crons {
        Some(c) => c,
        None => return Vec::new(),
    };

    crons
        .iter()
        .map(|raw| {
            let agent_id = raw.agent.as_deref().unwrap_or("main");
            let task = raw.task.as_deref().unwrap_or("(unnamed task)");
            let schedule_display = describe_cron(&raw.schedule);

            // Find last run from session data
            let last_run = find_last_cron_run(agent_id, task, agents);

            // Determine status
            let status = if !raw.enabled {
                CronStatus::Disabled
            } else if let Some(ref run) = last_run {
                // Check if overdue based on schedule
                if is_overdue(&raw.schedule, &run.timestamp) {
                    CronStatus::Overdue
                } else {
                    CronStatus::Ok
                }
            } else {
                CronStatus::Unknown
            };

            // Next run (simple estimation)
            let next_run = if raw.enabled {
                estimate_next_run(&raw.schedule)
            } else {
                None
            };

            CronEntry {
                schedule: raw.schedule.clone(),
                agent: agent_id.to_string(),
                task: task.to_string(),
                enabled: raw.enabled,
                delete_after_run: raw.delete_after_run,
                schedule_display,
                last_run,
                next_run,
                status,
            }
        })
        .collect()
}

/// Find the most recent cron run by scanning session files for cron sessions
fn find_last_cron_run(
    agent_id: &str,
    _task: &str,
    agents: &[ResolvedAgent],
) -> Option<CronRunInfo> {
    let agent = agents.iter().find(|a| a.id == agent_id)?;

    if !agent.sessions_dir.exists() {
        return None;
    }

    let entries = std::fs::read_dir(&agent.sessions_dir).ok()?;
    let mut latest: Option<CronRunInfo> = None;
    let mut latest_ts: f64 = 0.0;

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let fname = entry.file_name().to_string_lossy().to_string();

        // Only look at cron session files
        if !fname.contains(":cron:") {
            continue;
        }

        if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
            if let Some((ts, run)) = parse_last_cron_run(&path) {
                if ts > latest_ts {
                    latest_ts = ts;
                    latest = Some(run);
                }
            }
        }
    }

    latest
}

/// Parse a cron session JSONL to find the last run info
fn parse_last_cron_run(path: &Path) -> Option<(f64, CronRunInfo)> {
    let content = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().rev().collect();

    let mut last_ts: f64 = 0.0;
    let mut last_cost: f64 = 0.0;
    let mut last_tokens: u64 = 0;
    let mut last_timestamp_str = String::new();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let ts = obj.get("ts").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if ts > last_ts {
                last_ts = ts;

                // Get timestamp string
                last_timestamp_str = obj
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| {
                        let secs = (ts / 1000.0) as i64;
                        chrono::DateTime::from_timestamp(secs, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default()
                    });
            }

            if let Some(usage) = obj.get("usage") {
                let input = usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let output = usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let cost = usage
                    .get("cost")
                    .and_then(|c| c.get("total"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);

                last_cost += cost;
                last_tokens += input + output;
            }
        }
    }

    if last_ts > 0.0 {
        Some((
            last_ts,
            CronRunInfo {
                timestamp: last_timestamp_str,
                cost: last_cost,
                tokens: last_tokens,
                duration_ms: None,
            },
        ))
    } else {
        None
    }
}

/// Convert cron expression to human-readable text
fn describe_cron(expr: &str) -> String {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return expr.to_string();
    }

    let (min, hour, _dom, _mon, dow) = (parts[0], parts[1], parts[2], parts[3], parts[4]);

    // Common patterns
    if min.starts_with("*/") && hour == "*" {
        let interval: u32 = min[2..].parse().unwrap_or(0);
        return format!("Every {interval} minutes");
    }

    if min == "0" && hour.starts_with("*/") {
        let interval: u32 = hour[2..].parse().unwrap_or(0);
        return format!("Every {interval} hours");
    }

    if dow != "*" {
        let day_name = match dow {
            "0" | "7" => "Sunday",
            "1" => "Monday",
            "2" => "Tuesday",
            "3" => "Wednesday",
            "4" => "Thursday",
            "5" => "Friday",
            "6" => "Saturday",
            _ => dow,
        };
        return format!("Every {day_name} at {hour}:{min:0>2}");
    }

    if hour != "*" && min != "*" {
        return format!("Daily at {hour}:{min:0>2}");
    }

    expr.to_string()
}

/// Simple check if a cron job is overdue
fn is_overdue(schedule: &str, last_run: &str) -> bool {
    let Ok(last_dt) = chrono::DateTime::parse_from_rfc3339(last_run) else {
        return false;
    };
    let now = chrono::Utc::now();
    let elapsed = now.signed_duration_since(last_dt);

    let parts: Vec<&str> = schedule.split_whitespace().collect();
    if parts.len() != 5 {
        return false;
    }

    let (min, hour, _dom, _mon, dow) = (parts[0], parts[1], parts[2], parts[3], parts[4]);

    // Determine expected interval
    let expected_hours = if min.starts_with("*/") {
        let interval: f64 = min[2..].parse().unwrap_or(60.0);
        interval / 60.0
    } else if hour.starts_with("*/") {
        let interval: f64 = hour[2..].parse().unwrap_or(1.0);
        interval
    } else if dow != "*" {
        // Weekly
        7.0 * 24.0
    } else {
        // Daily
        24.0
    };

    // Overdue if elapsed > 2x expected interval
    let overdue_threshold = expected_hours * 2.0;
    elapsed.num_minutes() as f64 / 60.0 > overdue_threshold
}

/// Simple next run estimation
fn estimate_next_run(schedule: &str) -> Option<String> {
    let parts: Vec<&str> = schedule.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }

    let now = chrono::Utc::now();

    let (min_part, hour_part, _dom, _mon, _dow) = (parts[0], parts[1], parts[2], parts[3], parts[4]);

    if min_part.starts_with("*/") {
        // Every N minutes
        let interval: i64 = min_part[2..].parse().unwrap_or(30);
        let current_min = now.format("%M").to_string().parse::<i64>().unwrap_or(0);
        let next_min = ((current_min / interval) + 1) * interval;
        let delta = next_min - current_min;
        let next = now + chrono::Duration::minutes(delta);
        return Some(next.to_rfc3339());
    }

    if hour_part != "*" && min_part != "*" {
        // Daily at specific time
        let h: u32 = hour_part.parse().unwrap_or(0);
        let m: u32 = min_part.parse().unwrap_or(0);
        let today = now.date_naive();
        let next_time = today.and_hms_opt(h, m, 0)?;
        let next_dt = next_time.and_utc();
        if next_dt > now {
            return Some(next_dt.to_rfc3339());
        }
        // Tomorrow
        let tomorrow = today.succ_opt()?;
        let next_time = tomorrow.and_hms_opt(h, m, 0)?;
        return Some(next_time.and_utc().to_rfc3339());
    }

    None
}
