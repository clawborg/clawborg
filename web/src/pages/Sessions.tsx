import { useState, useEffect } from "react";
import { Radio, Cpu, Clock } from "lucide-react";
import { fetchSessions } from "@/api/client";
import type { SessionSummary } from "@/lib/types";
import PageLayout from "@/components/PageLayout";

const statusStyle: Record<string, string> = {
  active: "bg-green-500/20 text-green-400",
  idle: "bg-yellow-500/20 text-yellow-400",
  stale: "bg-red-500/20 text-red-400",
  archived: "bg-gray-500/20 text-gray-400",
};

const statusDot: Record<string, string> = {
  active: "bg-green-400",
  idle: "bg-yellow-400",
  stale: "bg-red-400",
  archived: "bg-gray-500",
};

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return n.toString();
}

function timeAgo(epochMs: number | null): string {
  if (!epochMs) return "—";
  const diff = Date.now() - epochMs;
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export default function Sessions() {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchSessions()
      .then(setSessions)
      .finally(() => setLoading(false));
  }, []);

  const active = sessions.filter((s) => s.status === "active").length;
  const idle = sessions.filter((s) => s.status === "idle").length;
  const stale = sessions.filter((s) => s.status === "stale").length;
  const totalTokens = sessions.reduce(
    (sum, s) => sum + s.inputTokens + s.outputTokens,
    0
  );

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
        <div className="bg-gray-900 rounded-xl border border-gray-800 overflow-x-auto">
          <table className="w-full text-sm min-w-[640px]">
            <thead>
              <tr className="border-b border-gray-800 text-gray-500 text-xs uppercase">
                <th className="text-left px-4 py-3">Agent</th>
                <th className="text-left px-4 py-3">Session</th>
                <th className="text-left px-4 py-3">Channel</th>
                <th className="text-left px-4 py-3">Status</th>
                <th className="text-left px-4 py-3">Model</th>
                <th className="text-right px-4 py-3">Tokens</th>
                <th className="text-right px-4 py-3">Last Active</th>
              </tr>
            </thead>
            <tbody>
              {sessions.map((s) => (
                <tr
                  key={`${s.agentId}-${s.sessionKey}`}
                  className="border-b border-gray-800/50 hover:bg-gray-800/30"
                >
                  <td className="px-4 py-3 font-medium">{s.agentId}</td>
                  <td className="px-4 py-3 font-mono text-xs text-gray-400 max-w-[200px] truncate">
                    {s.label || s.sessionKey}
                  </td>
                  <td className="px-4 py-3">
                    {s.channel && (
                      <span className="bg-gray-800 px-2 py-0.5 rounded text-xs">
                        {s.channel}
                      </span>
                    )}
                  </td>
                  <td className="px-4 py-3">
                    <span
                      className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs ${statusStyle[s.status] || ""}`}
                    >
                      <span
                        className={`w-1.5 h-1.5 rounded-full ${statusDot[s.status] || ""}`}
                      />
                      {s.status}
                    </span>
                  </td>
                  <td className="px-4 py-3 font-mono text-xs text-gray-500">
                    {s.model?.split("/").pop() || "—"}
                  </td>
                  <td className="px-4 py-3 text-right text-xs text-gray-400">
                    {formatTokens(s.inputTokens + s.outputTokens)}
                  </td>
                  <td className="px-4 py-3 text-right text-xs text-gray-500">
                    <span className="inline-flex items-center gap-1">
                      <Clock size={10} />
                      {timeAgo(s.lastActive)}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </PageLayout>
  );
}
