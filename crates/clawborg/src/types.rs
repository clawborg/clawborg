use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::broadcast;

/// Application state shared across all routes
#[derive(Clone)]
pub struct AppState {
    /// Root OpenClaw directory (e.g. ~/.openclaw/)
    pub openclaw_dir: PathBuf,
    pub readonly: bool,
    pub file_events_tx: broadcast::Sender<FileChangeEvent>,
}

// ─── OpenClaw Config Types ───
// Supports BOTH single-agent and multi-agent setups.
// Standard OpenClaw uses "agent" (singular) or "agents.defaults".
// Multi-agent uses "agents.list[]".

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenClawConfig {
    #[serde(default)]
    pub agents: Option<AgentsConfig>,
    #[serde(default)]
    pub agent: Option<AgentSingleConfig>,
    #[serde(default)]
    pub models: Option<serde_json::Value>,
    #[serde(default)]
    pub channels: Option<serde_json::Value>,
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
    #[serde(default)]
    pub identity: Option<IdentityConfig>,
    #[serde(default)]
    pub bindings: Option<serde_json::Value>,
    #[serde(default)]
    pub gateway: Option<serde_json::Value>,
    #[serde(default)]
    pub session: Option<serde_json::Value>,
    #[serde(default)]
    pub memory: Option<serde_json::Value>,
    #[serde(default)]
    pub mcp: Option<serde_json::Value>,
    #[serde(default)]
    pub alerts: Option<AlertsConfig>,
}

/// ClawBorg-specific alert thresholds, read from the "alerts" key in openclaw.json.
/// Example:
///   "alerts": { "dailySpendThreshold": 50.0, "dailySpendWarning": 10.0 }
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlertsConfig {
    /// Daily cost that triggers a critical alert (USD). Default: $20.
    #[serde(default)]
    pub daily_spend_threshold: Option<f64>,
    /// Daily cost that triggers a warning alert (USD). Default: $5.
    #[serde(default)]
    pub daily_spend_warning: Option<f64>,
}

/// "agents" block — multi-agent or single with defaults
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentsConfig {
    #[serde(default)]
    pub list: Option<Vec<AgentEntry>>,
    #[serde(default)]
    pub defaults: Option<AgentDefaults>,
}

/// "agent" block — singular form used in many standard configs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSingleConfig {
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub model: Option<AgentModel>,
    #[serde(default)]
    pub skip_bootstrap: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefaults {
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub model: Option<AgentModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEntry {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub model: Option<AgentModel>,
    #[serde(default, rename = "default")]
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModel {
    #[serde(default)]
    pub primary: Option<String>,
    #[serde(default)]
    pub fallbacks: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityConfig {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub emoji: Option<String>,
}

// ─── Resolved Agent (post-config-parse) ───
// This is the canonical representation ClawBorg works with.
// All paths fully resolved, no ambiguity.

#[derive(Debug, Clone)]
pub struct ResolvedAgent {
    pub id: String,
    pub name: Option<String>,
    pub model: Option<String>,
    pub fallbacks: Vec<String>,
    /// Fully resolved workspace path on disk
    pub workspace_path: PathBuf,
    /// Sessions directory: ~/.openclaw/agents/<id>/sessions/
    pub sessions_dir: PathBuf,
    pub is_default: bool,
}

// ─── API Response Types ───

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummary {
    pub id: String,
    pub name: Option<String>,
    pub model: Option<String>,
    pub workspace_path: String,
    pub file_count: usize,
    pub has_tasks: bool,
    pub pending_tasks: usize,
    pub session_count: usize,
    pub health: AgentHealth,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHealth {
    pub status: HealthStatus,
    pub issues: Vec<HealthIssue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub file: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDetail {
    pub id: String,
    pub name: Option<String>,
    pub model: Option<String>,
    pub fallbacks: Vec<String>,
    pub workspace_path: String,
    /// All discovered .md files in workspace root (auto-scanned, not hardcoded)
    pub files: HashMap<String, FileInfo>,
    /// Task counts — only present if tasks/ directory exists
    pub tasks: Option<TaskCounts>,
    /// Discovered subdirectories in workspace (memory/, skills/, tasks/, etc.)
    pub directories: Vec<String>,
    pub health: AgentHealth,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub exists: bool,
    pub size_bytes: u64,
    pub is_empty: bool,
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Default)]
pub struct TaskCounts {
    pub pending: usize,
    pub approved: usize,
    pub done: usize,
}

// ─── Session Types ───
// Real OpenClaw sessions.json format:
//   Flat map of session_key → SessionEntry
//   e.g. { "agent:pa:telegram:group:123": { "sessionId": "...", "updatedAt": 1234, ... } }

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionEntry {
    #[serde(default)]
    pub session_id: Option<String>,
    /// Unix timestamp in milliseconds
    #[serde(default)]
    pub updated_at: Option<u64>,
    /// Model name only (e.g. "claude-sonnet-4-5"), no provider prefix
    #[serde(default)]
    pub model: Option<String>,
    /// Model provider (e.g. "anthropic", "openrouter", "openai")
    #[serde(default)]
    pub model_provider: Option<String>,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    /// Cache read tokens (prompt caching)
    #[serde(default)]
    pub cache_read: u64,
    /// Cache write tokens (prompt caching)
    #[serde(default)]
    pub cache_write: u64,
    #[serde(default)]
    pub context_tokens: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub agent_id: String,
    pub session_key: String,
    pub session_id: Option<String>,
    pub channel: Option<String>,
    pub label: Option<String>,
    /// Unix timestamp in milliseconds (f64 for JS compat and sort)
    pub last_active: Option<f64>,
    pub status: SessionStatus,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_tokens: u64,
    pub model: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Idle,
    Stale,
}

// ─── Health Audit Types ───

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub total_agents: usize,
    pub healthy_agents: usize,
    pub total_issues: usize,
    pub agents: Vec<AgentHealthReport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHealthReport {
    pub agent_id: String,
    pub status: HealthStatus,
    pub issues: Vec<HealthIssue>,
}

// ─── WebSocket Event Types ───

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChangeEvent {
    pub event_type: String,
    pub path: String,
    pub agent_id: Option<String>,
    pub file_name: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// ─── API Error ───

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

impl ApiError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
        }
    }
}

// ─── Usage / Cost Types ───

#[derive(Debug, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    /// Total cost across all sessions (USD)
    pub total_cost: f64,
    /// Cost for today
    pub today_cost: f64,
    /// Cost for last 7 days
    pub week_cost: f64,
    /// Total input tokens
    pub total_input_tokens: u64,
    /// Total output tokens
    pub total_output_tokens: u64,
    /// Total cache read tokens
    pub total_cache_read_tokens: u64,
    /// Per-model cost breakdown
    pub by_model: Vec<ModelCost>,
    /// Per-agent cost breakdown
    pub by_agent: Vec<AgentCost>,
    /// Daily cost trend (last 30 days)
    pub daily_trend: Vec<DailyCost>,
    /// Sessions flagged as bloated (>500KB)
    pub bloated_sessions: Vec<BloatedSession>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelCost {
    pub model: String,
    pub cost: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub turn_count: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentCost {
    pub agent_id: String,
    pub agent_name: Option<String>,
    pub cost: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub session_count: usize,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DailyCost {
    pub date: String,
    pub cost: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BloatedSession {
    pub agent_id: String,
    pub session_key: String,
    pub size_bytes: u64,
    pub size_display: String,
}

// ─── Cron Types ───
// Real OpenClaw cron/jobs.json format:
//   { "version": 1, "jobs": [...] }

/// cron/jobs.json top-level wrapper
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CronJobsFile {
    #[serde(default)]
    pub version: u32,
    pub jobs: Vec<CronJobEntry>,
}

/// A single cron job definition from cron/jobs.json
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CronJobEntry {
    pub id: String,
    pub agent_id: String,
    pub name: String,
    pub enabled: bool,
    pub schedule: CronSchedule,
    #[serde(default)]
    pub session_target: Option<String>,
    #[serde(default)]
    pub session_key: Option<String>,
    #[serde(default)]
    pub wake_mode: Option<String>,
    #[serde(default)]
    pub payload: Option<CronJobPayload>,
    #[serde(default)]
    pub delivery: Option<CronJobDelivery>,
    #[serde(default)]
    pub state: Option<CronJobState>,
    #[serde(default)]
    pub notify: Option<serde_json::Value>,
    #[serde(default)]
    pub created_at_ms: Option<u64>,
    #[serde(default)]
    pub updated_at_ms: Option<u64>,
}

/// Schedule definition — either a fixed interval or a cron expression.
/// Real OpenClaw kinds: "every" (interval) or "cron" (cron expression).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CronSchedule {
    /// e.g. { "kind": "every", "everyMs": 1800000, "anchorMs": 0 }
    Every {
        #[serde(default)]
        every_ms: Option<u64>,
        #[serde(default)]
        anchor_ms: Option<u64>,
    },
    /// e.g. { "kind": "cron", "expr": "0 8 * * *", "tz": "UTC" }
    Cron {
        #[serde(default)]
        expr: Option<String>,
        #[serde(default)]
        tz: Option<String>,
    },
}

/// Runtime state stored by OpenClaw after each job execution
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CronJobState {
    #[serde(default)]
    pub last_run_at_ms: Option<u64>,
    #[serde(default)]
    pub last_status: Option<String>,
    #[serde(default)]
    pub last_duration_ms: Option<u64>,
    #[serde(default)]
    pub consecutive_errors: u32,
    #[serde(default)]
    pub next_run_at_ms: Option<u64>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_delivered: Option<serde_json::Value>,
    #[serde(default)]
    pub last_delivery_status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CronJobPayload {
    pub kind: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub thinking: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct CronJobDelivery {
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub best_effort: Option<bool>,
}

/// API response type for a cron job (served to frontend)
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CronEntry {
    pub id: String,
    pub schedule: String,
    pub agent: String,
    pub task: String,
    pub enabled: bool,
    /// Human-readable schedule description
    pub schedule_display: String,
    /// Last known run info (from job state)
    pub last_run: Option<CronRunInfo>,
    /// Next expected run (estimated)
    pub next_run: Option<String>,
    pub status: CronStatus,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CronRunInfo {
    pub timestamp: String,
    pub duration_ms: Option<u64>,
    pub last_status: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CronStatus {
    Ok,
    Overdue,
    Disabled,
    Unknown,
}

// ─── Smart Alerts Types ───

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub category: String,
    pub title: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Critical,
    Warning,
}
