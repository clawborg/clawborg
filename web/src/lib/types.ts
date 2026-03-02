// ─── API Response Types (matches Rust backend) ───

export interface AgentSummary {
  id: string;
  name: string | null;
  model: string | null;
  workspacePath: string;
  fileCount: number;
  hasTasks: boolean;
  pendingTasks: number;
  sessionCount: number;
  health: AgentHealth;
  isDefault: boolean;
}

export interface AgentDetail {
  id: string;
  name: string | null;
  model: string | null;
  fallbacks: string[];
  workspacePath: string;
  /** Auto-discovered .md files — NOT a hardcoded list */
  files: Record<string, FileInfo>;
  /** Task counts — null if agent has no task queue */
  tasks: TaskCounts | null;
  /** Subdirectories in workspace (memory/, skills/, tasks/, etc.) */
  directories: string[];
  health: AgentHealth;
  isDefault: boolean;
}

export interface FileInfo {
  exists: boolean;
  sizeBytes: number;
  isEmpty: boolean;
  modified: string | null;
}

export interface TaskCounts {
  pending: number;
  approved: number;
  done: number;
}

export interface AgentHealth {
  status: 'healthy' | 'warning' | 'critical';
  issues: HealthIssue[];
}

export interface HealthIssue {
  severity: 'critical' | 'warning' | 'info';
  message: string;
  file: string | null;
}

export interface SessionSummary {
  agentId: string;
  sessionKey: string;
  sessionId: string | null;
  channel: string | null;
  label: string | null;
  lastActive: number | null;
  status: 'active' | 'idle' | 'stale' | 'archived';
  inputTokens: number;
  outputTokens: number;
  contextTokens: number;
  model: string | null;
}

export interface HealthReport {
  totalAgents: number;
  healthyAgents: number;
  totalIssues: number;
  agents: AgentHealthReport[];
}

export interface AgentHealthReport {
  agentId: string;
  status: 'healthy' | 'warning' | 'critical';
  issues: HealthIssue[];
}

export interface FileChangeEvent {
  eventType: string;
  path: string;
  agentId: string | null;
  fileName: string | null;
  timestamp: string;
}

// ─── v0.2 Types ───

export interface UsageSummary {
  totalCost: number;
  todayCost: number;
  weekCost: number;
  totalInputTokens: number;
  totalOutputTokens: number;
  totalCacheReadTokens: number;
  byModel: ModelCost[];
  byAgent: AgentCost[];
  dailyTrend: DailyCost[];
  bloatedSessions: BloatedSession[];
}

export interface ModelCost {
  model: string;
  cost: number;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  turnCount: number;
}

export interface AgentCost {
  agentId: string;
  agentName: string | null;
  cost: number;
  inputTokens: number;
  outputTokens: number;
  sessionCount: number;
}

export interface DailyCost {
  date: string;
  cost: number;
  inputTokens: number;
  outputTokens: number;
}

export interface BloatedSession {
  agentId: string;
  sessionKey: string;
  sizeBytes: number;
  sizeDisplay: string;
}

export interface CronEntry {
  schedule: string;
  agent: string;
  task: string;
  enabled: boolean;
  deleteAfterRun: boolean;
  scheduleDisplay: string;
  lastRun: CronRunInfo | null;
  nextRun: string | null;
  status: 'ok' | 'overdue' | 'disabled' | 'unknown';
}

export interface CronRunInfo {
  timestamp: string;
  cost: number;
  tokens: number;
  durationMs: number | null;
}

export interface Alert {
  id: string;
  severity: 'critical' | 'warning' | 'info';
  category: string;
  title: string;
  message: string;
  timestamp: string;
}
