import type {
  AgentSummary,
  AgentDetail,
  SessionSummary,
  HealthReport,
  UsageSummary,
  CronEntry,
  Alert,
} from "@/lib/types";

const BASE = "/api";

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || res.statusText);
  }
  return res.json();
}

async function put<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw new Error(err.error || res.statusText);
  }
  return res.json();
}

// ─── Agents ───

export const fetchAgents = () => get<AgentSummary[]>("/agents");

export const fetchAgent = (id: string) => get<AgentDetail>(`/agents/${id}`);

// ─── Files ───

export const fetchFile = (agentId: string, filename: string) =>
  get<{ filename: string; content: string }>(
    `/agents/${agentId}/files/${filename}`
  );

export const updateFile = (
  agentId: string,
  filename: string,
  content: string
) =>
  put<{ status: string }>(`/agents/${agentId}/files/${filename}`, { content });

// ─── Sessions ───

export const fetchSessions = () => get<SessionSummary[]>("/sessions");

export const fetchAgentSessions = (agentId: string) =>
  get<SessionSummary[]>(`/sessions/${agentId}`);

// ─── Health ───

export const fetchHealth = () => get<HealthReport>("/health");

// ─── Config ───

export const fetchConfig = () => get<Record<string, unknown>>("/config");

// ─── Usage / Cost (v0.2) ───

export const fetchUsage = () => get<UsageSummary>("/usage");

// ─── Crons (v0.2) ───

export const fetchCrons = () => get<CronEntry[]>("/crons");

// ─── Alerts (v0.2) ───

export const fetchAlerts = () => get<Alert[]>("/alerts");
