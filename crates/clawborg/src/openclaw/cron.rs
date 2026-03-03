use crate::types::*;
use std::path::Path;

/// Build cron entries by reading ~/.openclaw/cron/jobs.json.
/// Last-run info comes from the `state` object embedded in each job.
pub fn build_cron_list(openclaw_dir: &Path, _agents: &[ResolvedAgent]) -> Vec<CronEntry> {
    let jobs_path = openclaw_dir.join("cron").join("jobs.json");

    let content = match std::fs::read_to_string(&jobs_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[clawborg] Failed to read {}: {e}", jobs_path.display());
            return vec![];
        }
    };

    let jobs: Vec<CronJobEntry> = match serde_json::from_str::<CronJobsFile>(&content) {
        Ok(f) => f.jobs,
        Err(e) => {
            eprintln!("[clawborg] Failed to parse {}: {e}", jobs_path.display());
            return vec![];
        }
    };

    jobs.iter().map(job_to_entry).collect()
}

fn job_to_entry(job: &CronJobEntry) -> CronEntry {
    let schedule_display = describe_schedule(&job.schedule);
    let schedule_str = schedule_to_string(&job.schedule);

    let last_run = job.state.as_ref().and_then(state_to_run_info);

    let status = if !job.enabled {
        CronStatus::Disabled
    } else if let Some(ref state) = job.state {
        match state.last_run_at_ms {
            None => CronStatus::Unknown,
            Some(_) if is_overdue(&job.schedule, state) => CronStatus::Overdue,
            Some(_) => CronStatus::Ok,
        }
    } else {
        CronStatus::Unknown
    };

    let next_run = if job.enabled {
        estimate_next_run(&job.schedule, job.state.as_ref())
    } else {
        None
    };

    CronEntry {
        id: job.id.clone(),
        schedule: schedule_str,
        agent: job.agent_id.clone(),
        task: job.name.clone(),
        enabled: job.enabled,
        schedule_display,
        last_run,
        next_run,
        status,
    }
}

/// Convert job state to a CronRunInfo for the API response.
fn state_to_run_info(state: &CronJobState) -> Option<CronRunInfo> {
    let last_run_ms = state.last_run_at_ms?;
    let secs = (last_run_ms / 1000) as i64;
    let timestamp = chrono::DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default();

    Some(CronRunInfo {
        timestamp,
        duration_ms: state.last_duration_ms,
        last_status: state.last_status.clone(),
    })
}

/// Canonical string representation of a schedule (used in CronEntry.schedule field).
fn schedule_to_string(schedule: &CronSchedule) -> String {
    match schedule {
        CronSchedule::Every { every_ms, .. } => match every_ms {
            Some(ms) => format!("every:{ms}"),
            None => "every:unknown".to_string(),
        },
        CronSchedule::Cron { expr, .. } => expr.clone().unwrap_or_else(|| "N/A".to_string()),
    }
}

/// Human-readable schedule description for the UI.
fn describe_schedule(schedule: &CronSchedule) -> String {
    match schedule {
        CronSchedule::Every { every_ms, .. } => match every_ms {
            Some(ms) => describe_interval(*ms),
            None => "N/A".to_string(),
        },
        CronSchedule::Cron { expr, .. } => {
            expr.as_deref().map(describe_cron_expr).unwrap_or_else(|| "N/A".to_string())
        }
    }
}

fn describe_interval(every_ms: u64) -> String {
    let secs = every_ms / 1000;
    if secs < 60 {
        format!("Every {secs} seconds")
    } else if secs < 3600 {
        let mins = secs / 60;
        format!("Every {mins} minutes")
    } else if secs < 86400 {
        let hours = secs / 3600;
        if hours == 1 {
            "Every hour".to_string()
        } else {
            format!("Every {hours} hours")
        }
    } else {
        let days = secs / 86400;
        if days == 1 {
            "Every day".to_string()
        } else {
            format!("Every {days} days")
        }
    }
}

fn describe_cron_expr(expr: &str) -> String {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return expr.to_string();
    }

    let (min, hour, _dom, _mon, dow) = (parts[0], parts[1], parts[2], parts[3], parts[4]);

    if min.starts_with("*/") && hour == "*" {
        let interval: u32 = min.strip_prefix("*/").and_then(|s| s.parse().ok()).unwrap_or(0);
        return format!("Every {interval} minutes");
    }

    if min == "0" && hour.starts_with("*/") {
        let interval: u32 = hour.strip_prefix("*/").and_then(|s| s.parse().ok()).unwrap_or(0);
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

/// Check if a job is overdue based on schedule and last-run state.
/// Returns false when the schedule interval is unknown (every_ms/expr missing).
fn is_overdue(schedule: &CronSchedule, state: &CronJobState) -> bool {
    let Some(last_run_ms) = state.last_run_at_ms else {
        return false;
    };
    let Some(expected_ms) = schedule_interval_ms(schedule) else {
        return false;
    };
    let now_ms = chrono::Utc::now().timestamp_millis() as u64;
    let elapsed_ms = now_ms.saturating_sub(last_run_ms);
    elapsed_ms > expected_ms * 2
}

/// Expected interval in milliseconds. Returns None when the interval cannot be determined.
fn schedule_interval_ms(schedule: &CronSchedule) -> Option<u64> {
    match schedule {
        CronSchedule::Every { every_ms, .. } => *every_ms,
        CronSchedule::Cron { expr, .. } => {
            expr.as_deref().map(cron_expr_interval_ms)
        }
    }
}

/// Rough interval estimate for a cron expression (used for overdue detection).
fn cron_expr_interval_ms(expr: &str) -> u64 {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return 24 * 3600 * 1000;
    }
    let (min, hour, _, _, dow) = (parts[0], parts[1], parts[2], parts[3], parts[4]);

    if min.starts_with("*/") {
        let n: u64 = min.strip_prefix("*/").and_then(|s| s.parse().ok()).unwrap_or(60);
        return n * 60 * 1000;
    }
    if hour.starts_with("*/") {
        let n: u64 = hour.strip_prefix("*/").and_then(|s| s.parse().ok()).unwrap_or(1);
        return n * 3600 * 1000;
    }
    if dow != "*" {
        return 7 * 24 * 3600 * 1000;
    }
    24 * 3600 * 1000
}

/// Estimate the next run time.
/// For intervals: last_run + interval. For cron exprs: simple time math.
fn estimate_next_run(schedule: &CronSchedule, state: Option<&CronJobState>) -> Option<String> {
    match schedule {
        CronSchedule::Every { every_ms, .. } => {
            let interval = (*every_ms)?;
            let last_ms = state
                .and_then(|s| s.last_run_at_ms)
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() as u64);
            let next_ms = last_ms + interval;
            let secs = (next_ms / 1000) as i64;
            chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.to_rfc3339())
        }
        CronSchedule::Cron { expr, .. } => expr.as_deref().and_then(estimate_next_cron),
    }
}

fn estimate_next_cron(expr: &str) -> Option<String> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return None;
    }

    let now = chrono::Utc::now();
    let (min_part, hour_part, ..) = (parts[0], parts[1], parts[2], parts[3], parts[4]);

    if min_part.starts_with("*/") {
        let interval: i64 = min_part
            .strip_prefix("*/")
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        let current_min = now.format("%M").to_string().parse::<i64>().unwrap_or(0);
        let next_min = ((current_min / interval) + 1) * interval;
        let next = now + chrono::Duration::minutes(next_min - current_min);
        return Some(next.to_rfc3339());
    }

    if hour_part != "*" && min_part != "*" {
        let h: u32 = hour_part.parse().unwrap_or(0);
        let m: u32 = min_part.parse().unwrap_or(0);
        let today = now.date_naive();
        let next_time = today.and_hms_opt(h, m, 0)?;
        let next_dt = next_time.and_utc();
        if next_dt > now {
            return Some(next_dt.to_rfc3339());
        }
        let tomorrow = today.succ_opt()?;
        return Some(tomorrow.and_hms_opt(h, m, 0)?.and_utc().to_rfc3339());
    }

    None
}
