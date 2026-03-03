import { useState, useEffect, useMemo } from "react";
import { Radio, Cpu, Clock, ChevronDown, ChevronUp, Search } from "lucide-react";
import { fetchSessions } from "@/api/client";
import type { SessionSummary } from "@/lib/types";
import PageLayout from "@/components/PageLayout";

/* ─── Types ─── */

type SortBy = "date" | "agent" | "cost" | "tokens";

/* ─── Model pricing (USD per 1M tokens) ─── */

interface Pricing {
  input: number;
  output: number;
  cacheRead: number;
}

const MODEL_PRICING: Record<string, Pricing> = {
  "claude-opus-4":         { input: 15,   output: 75,  cacheRead: 1.5  },
  "claude-opus-4-5":       { input: 15,   output: 75,  cacheRead: 1.5  },
  "claude-opus-4-6":       { input: 15,   output: 75,  cacheRead: 1.5  },
  "claude-sonnet-4":       { input: 3,    output: 15,  cacheRead: 0.3  },
  "claude-sonnet-4-5":     { input: 3,    output: 15,  cacheRead: 0.3  },
  "claude-sonnet-4-6":     { input: 3,    output: 15,  cacheRead: 0.3  },
  "claude-haiku-4-5":      { input: 0.8,  output: 4,   cacheRead: 0.08 },
  "claude-3-5-sonnet":     { input: 3,    output: 15,  cacheRead: 0.3  },
  "claude-3-5-haiku":      { input: 0.8,  output: 4,   cacheRead: 0.08 },
  "claude-3-opus":         { input: 15,   output: 75,  cacheRead: 1.5  },
};

function getPricing(model: string | null): Pricing | null {
  if (!model) return null;
  // Strip provider prefix (e.g. "anthropic/claude-sonnet-4-5" → "claude-sonnet-4-5")
  const name = model.split("/").pop() ?? model;
  if (MODEL_PRICING[name]) return MODEL_PRICING[name];
  // Partial match
  const entry = Object.entries(MODEL_PRICING).find(([k]) => name.includes(k));
  return entry ? entry[1] : null;
}

function estimateCost(s: SessionSummary): number | null {
  const p = getPricing(s.model);
  if (!p) return null;
  return (
    (s.inputTokens / 1_000_000) * p.input +
    (s.outputTokens / 1_000_000) * p.output +
    ((s.cacheWrite ?? 0) / 1_000_000) * p.input +
    ((s.cacheRead ?? 0) / 1_000_000) * p.cacheRead
  );
}

/* ─── Helpers ─── */

const STATUS_STYLE: Record<string, string> = {
  active:   "bg-green-500/20 text-green-400",
  idle:     "bg-yellow-500/20 text-yellow-400",
  stale:    "bg-red-500/20 text-red-400",
  archived: "bg-gray-500/20 text-gray-400",
};

const STATUS_DOT: Record<string, string> = {
  active:   "bg-green-400",
  idle:     "bg-yellow-400",
  stale:    "bg-red-400",
  archived: "bg-gray-500",
};

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return n.toString();
}

function formatCost(usd: number | null): string {
  if (usd === null) return "—";
  if (usd < 0.001) return `<$0.001`;
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  return `$${usd.toFixed(3)}`;
}

function timeAgo(epochMs: number | null): string {
  if (!epochMs) return "—";
  const diff = Date.now() - epochMs;
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function fmtDate(epochMs: number | null): string {
  if (!epochMs) return "—";
  return new Date(epochMs).toLocaleString([], {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/* ─── Pagination ─── */

function Pagination({
  page,
  total,
  onChange,
}: {
  page: number;
  total: number;
  onChange: (p: number) => void;
}) {
  if (total <= 1) return null;
  return (
    <div className="flex items-center justify-between mt-4 text-xs text-gray-500">
      <button
        onClick={() => onChange(page - 1)}
        disabled={page === 1}
        className="px-3 py-1.5 bg-gray-800 rounded disabled:opacity-40 hover:bg-gray-700 transition-colors"
      >
        ← Prev
      </button>
      <span>
        Page {page} of {total}
      </span>
      <button
        onClick={() => onChange(page + 1)}
        disabled={page === total}
        className="px-3 py-1.5 bg-gray-800 rounded disabled:opacity-40 hover:bg-gray-700 transition-colors"
      >
        Next →
      </button>
    </div>
  );
}

/* ─── Token breakdown row ─── */

function TokenRow({ label, value, dimmed }: { label: string; value: number; dimmed?: boolean }) {
  if (!value) return null;
  return (
    <div className="flex justify-between text-xs">
      <span className={dimmed ? "text-gray-600" : "text-gray-500"}>{label}</span>
      <span className={`font-mono tabular-nums ${dimmed ? "text-gray-600" : "text-gray-300"}`}>
        {formatTokens(value)}
      </span>
    </div>
  );
}

/* ─── Session detail panel ─── */

function SessionDetail({ session }: { session: SessionSummary }) {
  const [rawOpen, setRawOpen] = useState(false);
  const cost = estimateCost(session);
  const totalUsed = session.inputTokens + session.outputTokens + (session.cacheRead ?? 0) + (session.cacheWrite ?? 0);

  return (
    <div className="border-t border-gray-800 bg-gray-950/60 px-4 py-4 space-y-4 text-xs">
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {/* Metadata */}
        <div className="space-y-1.5">
          <p className="text-gray-500 uppercase tracking-wide text-[10px] font-medium mb-2">
            Session Metadata
          </p>
          {session.sessionId && (
            <div className="flex gap-2">
              <span className="text-gray-500 w-24 shrink-0">Session ID</span>
              <span className="text-gray-300 font-mono break-all">{session.sessionId}</span>
            </div>
          )}
          <div className="flex gap-2">
            <span className="text-gray-500 w-24 shrink-0">Session Key</span>
            <span className="text-gray-300 font-mono break-all">{session.sessionKey}</span>
          </div>
          <div className="flex gap-2">
            <span className="text-gray-500 w-24 shrink-0">Agent</span>
            <span className="text-gray-300">{session.agentId}</span>
          </div>
          {session.model && (
            <div className="flex gap-2">
              <span className="text-gray-500 w-24 shrink-0">Model</span>
              <span className="text-gray-300 font-mono">{session.model.split("/").pop()}</span>
            </div>
          )}
          {session.modelProvider && (
            <div className="flex gap-2">
              <span className="text-gray-500 w-24 shrink-0">Provider</span>
              <span className="text-gray-300">{session.modelProvider}</span>
            </div>
          )}
          {session.channel && (
            <div className="flex gap-2">
              <span className="text-gray-500 w-24 shrink-0">Channel</span>
              <span className="text-gray-300">{session.channel}</span>
            </div>
          )}
          <div className="flex gap-2">
            <span className="text-gray-500 w-24 shrink-0">Last Active</span>
            <span className="text-gray-300">{fmtDate(session.lastActive)}</span>
          </div>
          <div className="flex gap-2">
            <span className="text-gray-500 w-24 shrink-0">Status</span>
            <span
              className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded ${STATUS_STYLE[session.status] ?? ""}`}
            >
              <span className={`w-1.5 h-1.5 rounded-full ${STATUS_DOT[session.status] ?? ""}`} />
              {session.status}
            </span>
          </div>
        </div>

        {/* Tokens + cost */}
        <div>
          <p className="text-gray-500 uppercase tracking-wide text-[10px] font-medium mb-2">
            Token Usage
          </p>
          <div className="space-y-1.5 bg-gray-900 rounded-lg p-3 border border-gray-800">
            <TokenRow label="Input" value={session.inputTokens} />
            <TokenRow label="Output" value={session.outputTokens} />
            {(session.cacheRead ?? 0) > 0 && (
              <TokenRow label="Cache Read" value={session.cacheRead} dimmed />
            )}
            {(session.cacheWrite ?? 0) > 0 && (
              <TokenRow label="Cache Write" value={session.cacheWrite} dimmed />
            )}
            {session.contextTokens > 0 && (
              <TokenRow label="Context" value={session.contextTokens} dimmed />
            )}
            <div className="border-t border-gray-800 mt-2 pt-2 flex justify-between font-medium">
              <span className="text-gray-400">Total Used</span>
              <span className="text-gray-200 font-mono tabular-nums">
                {formatTokens(totalUsed)}
              </span>
            </div>
          </div>

          {cost !== null && (
            <div className="mt-3 bg-gray-900 rounded-lg p-3 border border-gray-800">
              <div className="flex justify-between items-baseline">
                <span className="text-gray-500">Estimated Cost</span>
                <span className="text-gray-200 font-mono font-medium">{formatCost(cost)}</span>
              </div>
              <p className="text-gray-700 text-[10px] mt-1">
                Based on {session.model?.split("/").pop()} list pricing
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Raw JSON collapsible */}
      <div className="border border-gray-800 rounded-lg overflow-hidden">
        <button
          onClick={() => setRawOpen((v) => !v)}
          className="w-full flex items-center justify-between px-3 py-2 bg-gray-900 hover:bg-gray-800/60 transition-colors text-xs text-gray-500"
        >
          <span>Raw JSON</span>
          {rawOpen ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
        </button>
        {rawOpen && (
          <pre className="text-xs text-gray-300 font-mono whitespace-pre overflow-auto max-h-64 p-3 leading-relaxed bg-gray-950">
            {JSON.stringify(session, null, 2)}
          </pre>
        )}
      </div>
    </div>
  );
}

/* ─── Sessions Page ─── */

const PAGE_SIZE = 20;

export default function Sessions() {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedKey, setExpandedKey] = useState<string | null>(null);

  const [sortBy, setSortBy] = useState<SortBy>("date");
  const [filterAgent, setFilterAgent] = useState("all");
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);

  useEffect(() => {
    fetchSessions()
      .then(setSessions)
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    setPage(1);
  }, [sortBy, filterAgent, search]);

  const agentIds = useMemo(
    () => Array.from(new Set(sessions.map((s) => s.agentId))).sort(),
    [sessions]
  );

  const filtered = useMemo(() => {
    let result = [...sessions];

    if (filterAgent !== "all") {
      result = result.filter((s) => s.agentId === filterAgent);
    }
    if (search.trim()) {
      const q = search.toLowerCase();
      result = result.filter(
        (s) =>
          s.sessionKey.toLowerCase().includes(q) ||
          (s.sessionId ?? "").toLowerCase().includes(q) ||
          (s.label ?? "").toLowerCase().includes(q)
      );
    }

    result.sort((a, b) => {
      switch (sortBy) {
        case "date":
          return (b.lastActive ?? 0) - (a.lastActive ?? 0);
        case "agent":
          return a.agentId.localeCompare(b.agentId);
        case "cost": {
          const ca = estimateCost(a) ?? -1;
          const cb = estimateCost(b) ?? -1;
          return cb - ca;
        }
        case "tokens":
          return (
            b.inputTokens + b.outputTokens - (a.inputTokens + a.outputTokens)
          );
        default:
          return 0;
      }
    });

    return result;
  }, [sessions, sortBy, filterAgent, search]);

  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const pageItems = filtered.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  const active = sessions.filter((s) => s.status === "active").length;
  const idle = sessions.filter((s) => s.status === "idle").length;
  const stale = sessions.filter((s) => s.status === "stale").length;
  const totalTokens = sessions.reduce(
    (sum, s) => sum + s.inputTokens + s.outputTokens,
    0
  );

  const SORT_OPTIONS: { value: SortBy; label: string }[] = [
    { value: "date", label: "Date" },
    { value: "agent", label: "Agent" },
    { value: "cost", label: "Cost" },
    { value: "tokens", label: "Tokens" },
  ];

  return (
    <PageLayout
      title="Sessions"
      icon={<Radio size={24} className="text-claw-400" />}
      subtitle={
        !loading && (
          <div className="flex flex-wrap gap-3">
            <span>{sessions.length} total</span>
            <span className="text-green-400">{active} active</span>
            <span className="text-yellow-400">{idle} idle</span>
            <span className="text-red-400">{stale} stale</span>
            <span className="flex items-center gap-1">
              <Cpu size={12} />
              {formatTokens(totalTokens)} tokens
            </span>
          </div>
        )
      }
    >
      {loading ? (
        <div className="text-gray-500">Loading sessions...</div>
      ) : (
        <>
          {/* Sort / filter / search bar */}
          <div className="flex flex-wrap items-center gap-3 mb-4">
            {/* Search */}
            <div className="relative flex-1 min-w-[160px] max-w-xs">
              <Search
                size={12}
                className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500 pointer-events-none"
              />
              <input
                type="text"
                placeholder="Search by session key or ID…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="w-full pl-8 pr-3 py-1.5 text-xs bg-gray-800 border border-gray-700 rounded-lg text-gray-300 placeholder-gray-600 focus:outline-none focus:border-claw-600"
              />
            </div>

            {/* Agent filter */}
            {agentIds.length > 1 && (
              <select
                value={filterAgent}
                onChange={(e) => setFilterAgent(e.target.value)}
                className="text-xs bg-gray-800 border border-gray-700 rounded-lg px-2.5 py-1.5 text-gray-300 focus:outline-none focus:border-claw-600"
              >
                <option value="all">All agents</option>
                {agentIds.map((id) => (
                  <option key={id} value={id}>
                    {id}
                  </option>
                ))}
              </select>
            )}

            {/* Sort */}
            <div className="flex items-center gap-1.5 text-xs text-gray-500">
              <span>Sort:</span>
              <div className="flex rounded-lg border border-gray-700 overflow-hidden">
                {SORT_OPTIONS.map((o) => (
                  <button
                    key={o.value}
                    onClick={() => setSortBy(o.value)}
                    className={`px-2.5 py-1.5 transition-colors border-r border-gray-700 last:border-r-0 ${
                      sortBy === o.value
                        ? "bg-gray-700 text-gray-100"
                        : "text-gray-500 hover:text-gray-300 hover:bg-gray-800"
                    }`}
                  >
                    {o.label}
                  </button>
                ))}
              </div>
            </div>
          </div>

          {(filterAgent !== "all" || search.trim()) && (
            <p className="text-xs text-gray-600 mb-3">
              Showing {filtered.length} of {sessions.length} sessions
            </p>
          )}

          {/* Sessions table */}
          <div className="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden">
            {pageItems.length === 0 ? (
              <div className="text-center py-12 text-gray-600 text-sm">
                No sessions match the current filters
              </div>
            ) : (
              <div className="divide-y divide-gray-800/50">
                {pageItems.map((s) => {
                  const rowKey = `${s.agentId}-${s.sessionKey}`;
                  const isExpanded = expandedKey === rowKey;
                  const cost = estimateCost(s);

                  return (
                    <div key={rowKey}>
                      {/* Clickable row */}
                      <button
                        className="w-full text-left px-4 py-3 hover:bg-gray-800/30 transition-colors flex items-center gap-3"
                        onClick={() =>
                          setExpandedKey(isExpanded ? null : rowKey)
                        }
                      >
                        {/* Status dot */}
                        <span
                          className={`w-2 h-2 rounded-full shrink-0 ${STATUS_DOT[s.status] ?? "bg-gray-600"}`}
                        />

                        {/* Main content */}
                        <div className="flex-1 min-w-0 grid grid-cols-2 sm:grid-cols-4 gap-x-4 gap-y-0.5 text-sm">
                          <div className="min-w-0">
                            <div className="font-medium text-gray-300 text-xs truncate">
                              {s.agentId}
                            </div>
                            <div className="font-mono text-[10px] text-gray-600 truncate">
                              {s.label || s.sessionKey}
                            </div>
                          </div>
                          <div className="hidden sm:block">
                            {s.channel && (
                              <span className="bg-gray-800 px-1.5 py-0.5 rounded text-[10px] text-gray-400">
                                {s.channel}
                              </span>
                            )}
                            {s.model && (
                              <div className="font-mono text-[10px] text-gray-600 mt-0.5 truncate">
                                {s.model.split("/").pop()}
                              </div>
                            )}
                          </div>
                          <div className="text-right sm:text-left">
                            <div className="text-xs text-gray-400">
                              {formatTokens(s.inputTokens + s.outputTokens)} tok
                            </div>
                            {cost !== null && (
                              <div className="text-[10px] text-gray-600">
                                {formatCost(cost)}
                              </div>
                            )}
                          </div>
                          <div className="hidden sm:flex items-center gap-1 text-[10px] text-gray-600">
                            <Clock size={9} />
                            {timeAgo(s.lastActive)}
                          </div>
                        </div>

                        {/* Chevron */}
                        <div className="shrink-0 text-gray-600">
                          {isExpanded ? (
                            <ChevronUp size={13} />
                          ) : (
                            <ChevronDown size={13} />
                          )}
                        </div>
                      </button>

                      {/* Expanded detail panel */}
                      {isExpanded && <SessionDetail session={s} />}
                    </div>
                  );
                })}
              </div>
            )}
          </div>

          <Pagination page={page} total={totalPages} onChange={setPage} />
        </>
      )}
    </PageLayout>
  );
}
