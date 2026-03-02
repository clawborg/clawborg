use crate::types::*;
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;

/// Build a complete usage summary across all agents
pub fn build_usage_summary(agents: &[ResolvedAgent]) -> UsageSummary {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let today_date = NaiveDate::parse_from_str(&today, "%Y-%m-%d").ok();

    let mut total_cost: f64 = 0.0;
    let mut today_cost: f64 = 0.0;
    let mut week_cost: f64 = 0.0;
    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache_read: u64 = 0;

    let mut model_map: HashMap<String, ModelCost> = HashMap::new();
    let mut agent_costs: Vec<AgentCost> = Vec::new();
    let mut daily_map: HashMap<String, DailyCost> = HashMap::new();
    let mut bloated: Vec<BloatedSession> = Vec::new();

    let now = Utc::now();
    let week_ago_ts = (now - chrono::Duration::days(7)).timestamp() as f64 * 1000.0;

    for agent in agents {
        let mut agent_cost: f64 = 0.0;
        let mut agent_input: u64 = 0;
        let mut agent_output: u64 = 0;
        let mut session_count: usize = 0;

        if !agent.sessions_dir.exists() {
            agent_costs.push(AgentCost {
                agent_id: agent.id.clone(),
                agent_name: agent.name.clone(),
                cost: 0.0,
                input_tokens: 0,
                output_tokens: 0,
                session_count: 0,
            });
            continue;
        }

        let Ok(entries) = std::fs::read_dir(&agent.sessions_dir) else {
            continue;
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                session_count += 1;

                // Check for bloated sessions
                if let Ok(meta) = std::fs::metadata(&path) {
                    let size = meta.len();
                    if size > 500_000 {
                        let session_key = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        bloated.push(BloatedSession {
                            agent_id: agent.id.clone(),
                            session_key,
                            size_bytes: size,
                            size_display: format_bytes(size),
                        });
                    }
                }

                // Parse JSONL for usage data
                let turns = parse_jsonl_usage(&path);
                for turn in &turns {
                    total_cost += turn.cost;
                    agent_cost += turn.cost;
                    total_input += turn.input_tokens;
                    total_output += turn.output_tokens;
                    total_cache_read += turn.cache_read_tokens;
                    agent_input += turn.input_tokens;
                    agent_output += turn.output_tokens;

                    // Today's cost
                    if let Some(ref date) = turn.date {
                        if date == &today {
                            today_cost += turn.cost;
                        }
                    }

                    // Week cost
                    if turn.timestamp_ms >= week_ago_ts {
                        week_cost += turn.cost;
                    }

                    // Per-model
                    if let Some(ref model) = turn.model {
                        let entry = model_map.entry(model.clone()).or_insert(ModelCost {
                            model: model.clone(),
                            cost: 0.0,
                            input_tokens: 0,
                            output_tokens: 0,
                            cache_read_tokens: 0,
                            turn_count: 0,
                        });
                        entry.cost += turn.cost;
                        entry.input_tokens += turn.input_tokens;
                        entry.output_tokens += turn.output_tokens;
                        entry.cache_read_tokens += turn.cache_read_tokens;
                        entry.turn_count += 1;
                    }

                    // Daily trend
                    if let Some(ref date) = turn.date {
                        let daily = daily_map.entry(date.clone()).or_insert(DailyCost {
                            date: date.clone(),
                            cost: 0.0,
                            input_tokens: 0,
                            output_tokens: 0,
                        });
                        daily.cost += turn.cost;
                        daily.input_tokens += turn.input_tokens;
                        daily.output_tokens += turn.output_tokens;
                    }
                }
            }
        }

        agent_costs.push(AgentCost {
            agent_id: agent.id.clone(),
            agent_name: agent.name.clone(),
            cost: agent_cost,
            input_tokens: agent_input,
            output_tokens: agent_output,
            session_count,
        });
    }

    // Sort models by cost descending
    let mut by_model: Vec<ModelCost> = model_map.into_values().collect();
    by_model.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));

    // Sort agents by cost descending
    agent_costs.sort_by(|a, b| b.cost.partial_cmp(&a.cost).unwrap_or(std::cmp::Ordering::Equal));

    // Sort daily trend by date, keep last 30 days
    let mut daily_trend: Vec<DailyCost> = daily_map.into_values().collect();
    daily_trend.sort_by(|a, b| a.date.cmp(&b.date));

    if let Some(cutoff_date) = today_date.and_then(|d| d.checked_sub_signed(chrono::Duration::days(30))) {
        let cutoff = cutoff_date.format("%Y-%m-%d").to_string();
        daily_trend.retain(|d| d.date >= cutoff);
    }

    // Sort bloated by size descending
    bloated.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    UsageSummary {
        total_cost,
        today_cost,
        week_cost,
        total_input_tokens: total_input,
        total_output_tokens: total_output,
        total_cache_read_tokens: total_cache_read,
        by_model,
        by_agent: agent_costs,
        daily_trend,
        bloated_sessions: bloated,
    }
}

/// Individual turn usage extracted from JSONL
struct TurnUsage {
    cost: f64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    model: Option<String>,
    timestamp_ms: f64,
    date: Option<String>,
}

/// Parse a JSONL file and extract per-turn usage data
fn parse_jsonl_usage(path: &Path) -> Vec<TurnUsage> {
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let reader = std::io::BufReader::new(file);
    let mut turns = Vec::new();

    for line_result in reader.lines() {
        let Ok(line) = line_result else { continue };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };

        // Only process lines with usage data (assistant messages)
        let usage = match obj.get("usage") {
            Some(u) => u,
            None => continue,
        };

        let input_tokens = usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let output_tokens = usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let cache_read_tokens = usage
            .get("cache_read_input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Cost: try usage.cost.total first, then compute estimate
        let cost = usage
            .get("cost")
            .and_then(|c| c.get("total"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let model = obj.get("model").and_then(|v| v.as_str()).map(String::from);

        // Timestamp
        let timestamp_ms = obj
            .get("ts")
            .and_then(|v| v.as_f64())
            .or_else(|| {
                obj.get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(s)
                            .ok()
                            .map(|dt| dt.timestamp_millis() as f64)
                    })
            })
            .unwrap_or(0.0);

        let date = if timestamp_ms > 0.0 {
            let secs = (timestamp_ms / 1000.0) as i64;
            chrono::DateTime::from_timestamp(secs, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
        } else {
            None
        };

        turns.push(TurnUsage {
            cost,
            input_tokens,
            output_tokens,
            cache_read_tokens,
            model,
            timestamp_ms,
            date,
        });
    }

    turns
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
