import { useState, useEffect } from "react";
import { fetchCrons } from "@/api/client";
import type { CronEntry } from "@/lib/types";
import { Clock, CheckCircle, AlertTriangle, XCircle, Pause, ChevronDown, ChevronUp } from "lucide-react";
import PageLayout from "@/components/PageLayout";

/* ─── Helpers ─── */

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

/* ─── Detail field row ─── */

function DetailRow({ label, value }: { label: string; value: string | number | null | undefined }) {
  if (value === null || value === undefined || value === "") return null;
  return (
    <div className="flex gap-2 text-xs">
      <span className="text-gray-500 w-32 shrink-0">{label}</span>
      <span className="text-gray-300 font-mono break-all">{String(value)}</span>
    </div>
  );
}

/* ─── Crons Page ─── */

export default function Crons() {
  const [crons, setCrons] = useState<CronEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  useEffect(() => {
    fetchCrons()
      .then(setCrons)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

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
        <div className="space-y-3">
          {crons.map((cron) => {
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
                        <h3 className="text-sm font-medium text-gray-200 truncate text-left">{cron.task}</h3>
                        <div className="flex items-center gap-2 mt-0.5">
                          <span className="text-xs text-gray-500">
                            Agent: <span className="text-gray-400">{cron.agent}</span>
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
                      <span className="text-xs text-gray-600 block font-mono">{cron.schedule}</span>
                    </div>
                    <div>
                      <span className="text-xs text-gray-500 block">Last Run</span>
                      {cron.lastRun ? (
                        <>
                          <span className="text-gray-300">{timeAgo(cron.lastRun.timestamp)}</span>
                          <span className="text-xs text-gray-600 block">
                            {cron.lastRun.lastStatus ?? "—"}
                            {cron.lastRun.durationMs !== null && cron.lastRun.durationMs !== undefined
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

                {/* Collapsible detail panel */}
                {isExpanded && (
                  <div className="border-t border-gray-800 px-4 py-3 space-y-2 bg-gray-950/50">
                    <p className="text-xs text-gray-500 uppercase tracking-wide mb-2">Job Details</p>
                    <DetailRow label="ID" value={cron.id} />
                    <DetailRow label="Session Target" value={cron.sessionTarget} />
                    <DetailRow label="Session Key" value={cron.sessionKey} />
                    <DetailRow label="Wake Mode" value={cron.wakeMode} />
                    {cron.payloadMessage && (
                      <div className="flex gap-2 text-xs">
                        <span className="text-gray-500 w-32 shrink-0">Payload</span>
                        <span className="text-gray-300 bg-gray-900 rounded p-2 border border-gray-800 flex-1 font-mono whitespace-pre-wrap break-all">
                          {cron.payloadMessage}
                        </span>
                      </div>
                    )}
                    <DetailRow label="Delivery Mode" value={cron.deliveryMode} />
                    <DetailRow label="Delivery Channel" value={cron.deliveryChannel} />
                    <DetailRow label="Delivery To" value={cron.deliveryTo} />
                    {(cron.consecutiveErrors ?? 0) > 0 && (
                      <DetailRow label="Consec. Errors" value={cron.consecutiveErrors} />
                    )}
                    {cron.lastError && (
                      <div className="flex gap-2 text-xs">
                        <span className="text-gray-500 w-32 shrink-0">Last Error</span>
                        <span className="text-red-400 font-mono break-all">{cron.lastError}</span>
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </PageLayout>
  );
}
