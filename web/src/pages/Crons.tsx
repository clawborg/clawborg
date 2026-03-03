import { useState, useEffect, useMemo } from "react";
import { fetchCrons } from "@/api/client";
import type { CronEntry } from "@/lib/types";
import {
  Clock,
  CheckCircle,
  AlertTriangle,
  XCircle,
  Pause,
  ChevronDown,
  ChevronUp,
  Search,
} from "lucide-react";
import PageLayout from "@/components/PageLayout";

/* ─── Helpers ─── */

type SortBy = "name" | "status" | "nextRun" | "lastRun";
type StatusFilter = "all" | "ok" | "overdue" | "disabled" | "unknown";

const STATUS_ORDER: Record<string, number> = {
  overdue: 0,
  unknown: 1,
  ok: 2,
  disabled: 3,
};

function statusIcon(status: string) {
  switch (status) {
    case "ok":
      return <CheckCircle size={16} className="text-green-400" />;
    case "overdue":
      return <AlertTriangle size={16} className="text-yellow-400" />;
    case "disabled":
      return <Pause size={16} className="text-gray-500" />;
    default:
      return <XCircle size={16} className="text-gray-500" />;
  }
}

function statusBadge(status: string) {
  const styles: Record<string, string> = {
    ok: "bg-green-900/40 text-green-400 border-green-800",
    overdue: "bg-yellow-900/40 text-yellow-400 border-yellow-800",
    disabled: "bg-gray-800 text-gray-500 border-gray-700",
    unknown: "bg-gray-800 text-gray-500 border-gray-700",
  };
  return styles[status] || styles.unknown;
}

function timeAgo(isoStr: string): string {
  const diff = Date.now() - new Date(isoStr).getTime();
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function fmtDuration(ms: number | null): string {
  if (ms === null || ms === undefined) return "";
  if (ms < 1_000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1_000).toFixed(1)}s`;
  return `${(ms / 60_000).toFixed(1)}m`;
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

/* ─── Crons Page ─── */

const PAGE_SIZE = 20;

export default function Crons() {
  const [crons, setCrons] = useState<CronEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const [sortBy, setSortBy] = useState<SortBy>("status");
  const [filterStatus, setFilterStatus] = useState<StatusFilter>("all");
  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);

  useEffect(() => {
    fetchCrons()
      .then(setCrons)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  // Reset page when filters change
  useEffect(() => {
    setPage(1);
  }, [sortBy, filterStatus, search]);

  const filtered = useMemo(() => {
    let result = [...crons];

    if (filterStatus !== "all") {
      result = result.filter((c) => c.status === filterStatus);
    }
    if (search.trim()) {
      const q = search.toLowerCase();
      result = result.filter(
        (c) =>
          c.task.toLowerCase().includes(q) ||
          c.agent.toLowerCase().includes(q)
      );
    }

    result.sort((a, b) => {
      switch (sortBy) {
        case "name":
          return a.task.localeCompare(b.task);
        case "status":
          return (
            (STATUS_ORDER[a.status] ?? 4) - (STATUS_ORDER[b.status] ?? 4)
          );
        case "nextRun": {
          if (!a.nextRun && !b.nextRun) return 0;
          if (!a.nextRun) return 1;
          if (!b.nextRun) return -1;
          return (
            new Date(a.nextRun).getTime() - new Date(b.nextRun).getTime()
          );
        }
        case "lastRun": {
          if (!a.lastRun && !b.lastRun) return 0;
          if (!a.lastRun) return 1;
          if (!b.lastRun) return -1;
          return (
            new Date(b.lastRun.timestamp).getTime() -
            new Date(a.lastRun.timestamp).getTime()
          );
        }
        default:
          return 0;
      }
    });

    return result;
  }, [crons, sortBy, filterStatus, search]);

  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const pageItems = filtered.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  if (loading) {
    return (
      <PageLayout title="Cron Jobs" icon={<Clock size={24} className="text-claw-400" />}>
        <div className="flex items-center justify-center h-64 text-gray-400">
          Loading cron jobs...
        </div>
      </PageLayout>
    );
  }

  if (error) {
    return (
      <PageLayout title="Cron Jobs" icon={<Clock size={24} className="text-claw-400" />}>
        <div className="bg-red-900/20 border border-red-800 rounded-lg p-4 text-red-400">
          {error}
        </div>
      </PageLayout>
    );
  }

  const enabled = crons.filter((c) => c.enabled).length;
  const overdue = crons.filter((c) => c.status === "overdue").length;

  const statusCounts: Record<string, number> = {};
  for (const c of crons) {
    statusCounts[c.status] = (statusCounts[c.status] ?? 0) + 1;
  }

  const STATUS_FILTERS: { value: StatusFilter; label: string }[] = [
    { value: "all", label: `All (${crons.length})` },
    { value: "overdue", label: `Overdue (${statusCounts.overdue ?? 0})` },
    { value: "ok", label: `OK (${statusCounts.ok ?? 0})` },
    { value: "disabled", label: `Disabled (${statusCounts.disabled ?? 0})` },
    { value: "unknown", label: `Unknown (${statusCounts.unknown ?? 0})` },
  ];

  const SORT_OPTIONS: { value: SortBy; label: string }[] = [
    { value: "status", label: "Status" },
    { value: "name", label: "Name" },
    { value: "nextRun", label: "Next Run" },
    { value: "lastRun", label: "Last Run" },
  ];

  return (
    <PageLayout
      title="Cron Jobs"
      icon={<Clock size={24} className="text-claw-400" />}
      subtitle={
        <span>
          {crons.length} job{crons.length !== 1 ? "s" : ""} — {enabled} enabled
          {overdue > 0 && (
            <span className="text-yellow-400 ml-1">· {overdue} overdue</span>
          )}
        </span>
      }
    >
      {crons.length === 0 ? (
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-8 text-center">
          <Clock size={32} className="text-gray-600 mx-auto mb-3" />
          <p className="text-gray-400">No cron jobs configured</p>
          <p className="text-gray-600 text-sm mt-1">
            Add a <code className="bg-gray-800 px-1 rounded">crons</code> array to your openclaw.json
          </p>
        </div>
      ) : (
        <>
          {/* Sort / filter / search bar */}
          <div className="flex flex-wrap items-center gap-3 mb-4">
            {/* Search */}
            <div className="relative flex-1 min-w-[160px] max-w-xs">
              <Search size={12} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500 pointer-events-none" />
              <input
                type="text"
                placeholder="Search jobs or agents…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="w-full pl-8 pr-3 py-1.5 text-xs bg-gray-800 border border-gray-700 rounded-lg text-gray-300 placeholder-gray-600 focus:outline-none focus:border-claw-600"
              />
            </div>

            {/* Status filter */}
            <div className="flex rounded-lg border border-gray-700 overflow-hidden text-xs">
              {STATUS_FILTERS.filter((f) => {
                // Hide zero-count non-all filters
                if (f.value === "all") return true;
                return (statusCounts[f.value] ?? 0) > 0;
              }).map((f) => (
                <button
                  key={f.value}
                  onClick={() => setFilterStatus(f.value)}
                  className={`px-2.5 py-1.5 transition-colors border-r border-gray-700 last:border-r-0 ${
                    filterStatus === f.value
                      ? "bg-claw-700/60 text-claw-300"
                      : "text-gray-500 hover:text-gray-300 hover:bg-gray-800"
                  }`}
                >
                  {f.label}
                </button>
              ))}
            </div>

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

          {/* Results count when filtering */}
          {(filterStatus !== "all" || search.trim()) && (
            <p className="text-xs text-gray-600 mb-3">
              Showing {filtered.length} of {crons.length} jobs
            </p>
          )}

          {pageItems.length === 0 ? (
            <div className="text-center py-12 text-gray-600 text-sm">
              No jobs match the current filters
            </div>
          ) : (
            <div className="space-y-3">
              {pageItems.map((cron) => {
                const isExpanded = expandedId === cron.id;
                return (
                  <div
                    key={cron.id}
                    className={`bg-gray-900 border rounded-xl overflow-hidden ${
                      cron.enabled ? "border-gray-800" : "border-gray-800/50 opacity-60"
                    }`}
                  >
                    {/* Clickable header row */}
                    <button
                      className="w-full text-left p-4 hover:bg-gray-800/40 transition-colors"
                      onClick={() => setExpandedId(isExpanded ? null : cron.id)}
                    >
                      <div className="flex items-start justify-between gap-4 mb-3">
                        <div className="flex items-center gap-3 min-w-0">
                          {statusIcon(cron.status)}
                          <div className="min-w-0">
                            <h3 className="text-sm font-medium text-gray-200 truncate text-left">
                              {cron.task}
                            </h3>
                            <div className="flex items-center gap-2 mt-0.5">
                              <span className="text-xs text-gray-500">
                                Agent:{" "}
                                <span className="text-gray-400">{cron.agent}</span>
                              </span>
                            </div>
                          </div>
                        </div>
                        <div className="flex items-center gap-2 shrink-0">
                          <span
                            className={`text-xs px-2 py-0.5 rounded border whitespace-nowrap ${statusBadge(cron.status)}`}
                          >
                            {cron.status}
                          </span>
                          {isExpanded ? (
                            <ChevronUp size={14} className="text-gray-500" />
                          ) : (
                            <ChevronDown size={14} className="text-gray-500" />
                          )}
                        </div>
                      </div>

                      {/* Summary row */}
                      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
                        <div>
                          <span className="text-xs text-gray-500 block">Schedule</span>
                          <span className="text-gray-300">{cron.scheduleDisplay}</span>
                          <span className="text-xs text-gray-600 block font-mono">
                            {cron.schedule}
                          </span>
                        </div>
                        <div>
                          <span className="text-xs text-gray-500 block">Last Run</span>
                          {cron.lastRun ? (
                            <>
                              <span className="text-gray-300">
                                {timeAgo(cron.lastRun.timestamp)}
                              </span>
                              <span className="text-xs text-gray-600 block">
                                {cron.lastRun.lastStatus ?? "—"}
                                {cron.lastRun.durationMs !== null &&
                                cron.lastRun.durationMs !== undefined
                                  ? ` · ${fmtDuration(cron.lastRun.durationMs)}`
                                  : ""}
                              </span>
                            </>
                          ) : (
                            <span className="text-gray-500">Never</span>
                          )}
                        </div>
                        <div>
                          <span className="text-xs text-gray-500 block">Next Run</span>
                          {cron.nextRun ? (
                            <span className="text-gray-300">
                              {new Date(cron.nextRun).toLocaleTimeString([], {
                                hour: "2-digit",
                                minute: "2-digit",
                              })}
                            </span>
                          ) : (
                            <span className="text-gray-500">—</span>
                          )}
                        </div>
                        <div>
                          <span className="text-xs text-gray-500 block">Status</span>
                          <span className="text-gray-300">
                            {cron.enabled ? "Enabled" : "Disabled"}
                          </span>
                        </div>
                      </div>
                    </button>

                    {/* Collapsible detail panel — raw JSON */}
                    {isExpanded && (
                      <div className="border-t border-gray-800 bg-gray-950/50">
                        <p className="text-xs text-gray-500 uppercase tracking-wide px-4 pt-3 pb-2">
                          Job Details
                        </p>
                        <pre className="text-xs text-gray-300 font-mono whitespace-pre overflow-auto max-h-80 px-4 pb-4 leading-relaxed">
                          {JSON.stringify(
                            cron.raw ?? {
                              id: cron.id,
                              agent: cron.agent,
                              task: cron.task,
                              schedule: cron.schedule,
                              enabled: cron.enabled,
                              sessionKey: cron.sessionKey,
                              sessionTarget: cron.sessionTarget,
                              wakeMode: cron.wakeMode,
                              payloadMessage: cron.payloadMessage,
                              deliveryMode: cron.deliveryMode,
                              deliveryChannel: cron.deliveryChannel,
                              deliveryTo: cron.deliveryTo,
                              consecutiveErrors: cron.consecutiveErrors,
                              lastError: cron.lastError,
                            },
                            null,
                            2
                          )}
                        </pre>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}

          <Pagination page={page} total={totalPages} onChange={setPage} />
        </>
      )}
    </PageLayout>
  );
}
