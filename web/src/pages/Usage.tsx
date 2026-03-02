import { useState, useEffect } from "react";
import { fetchUsage } from "@/api/client";
import type { UsageSummary } from "@/lib/types";
import { DollarSign, TrendingUp, Cpu, AlertTriangle } from "lucide-react";
import PageLayout from "@/components/PageLayout";

/* ─── Helpers ─── */

function fmtCost(n: number): string {
  if (n >= 1) return `$${n.toFixed(2)}`;
  if (n >= 0.01) return `$${n.toFixed(3)}`;
  if (n > 0) return `$${n.toFixed(4)}`;
  return "$0.00";
}

function fmtTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return n.toString();
}

function modelShort(model: string): string {
  const parts = model.split("/");
  return parts[parts.length - 1] || model;
}

/* ─── Bar component for simple inline bars ─── */

function CostBar({ value, max, color }: { value: number; max: number; color: string }) {
  const pct = max > 0 ? Math.min((value / max) * 100, 100) : 0;
  return (
    <div className="w-full bg-gray-800 rounded-full h-2">
      <div className={`h-2 rounded-full ${color}`} style={{ width: `${pct}%` }} />
    </div>
  );
}

/* ─── Daily Trend Sparkline ─── */

function DailyTrendChart({ data }: { data: { date: string; cost: number }[] }) {
  if (data.length === 0) {
    return <div className="text-gray-500 text-sm py-8 text-center">No daily data yet</div>;
  }

  const maxCost = Math.max(...data.map((d) => d.cost), 0.01);
  const barWidth = Math.max(4, Math.min(24, Math.floor(600 / data.length) - 2));

  return (
    <div className="overflow-x-auto scrollbar-none">
      <div className="flex items-end gap-1 h-32 min-w-fit px-1">
        {data.map((d) => {
          const h = Math.max(2, (d.cost / maxCost) * 100);
          return (
            <div key={d.date} className="flex flex-col items-center group relative">
              {/* Tooltip */}
              <div className="absolute -top-8 hidden group-hover:block bg-gray-800 border border-gray-700 text-xs px-2 py-1 rounded whitespace-nowrap z-10">
                {d.date}: {fmtCost(d.cost)}
              </div>
              <div
                className="bg-claw-500 rounded-sm hover:bg-claw-400 transition-colors"
                style={{ width: barWidth, height: `${h}%` }}
              />
            </div>
          );
        })}
      </div>
      {/* Date labels */}
      <div className="flex justify-between text-xs text-gray-600 mt-1 px-1">
        <span>{data[0]?.date.slice(5)}</span>
        <span>{data[data.length - 1]?.date.slice(5)}</span>
      </div>
    </div>
  );
}

/* ─── Usage Page ─── */

export default function Usage() {
  const [usage, setUsage] = useState<UsageSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchUsage()
      .then(setUsage)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <PageLayout title="Usage & Cost" icon={<DollarSign size={24} className="text-claw-400" />}>
        <div className="flex items-center justify-center h-64 text-gray-400">
          Loading usage data...
        </div>
      </PageLayout>
    );
  }

  if (error || !usage) {
    return (
      <PageLayout title="Usage & Cost" icon={<DollarSign size={24} className="text-claw-400" />}>
        <div className="bg-red-900/20 border border-red-800 rounded-lg p-4 text-red-400">
          {error || "Failed to load usage data"}
        </div>
      </PageLayout>
    );
  }

  const totalTokens = usage.totalInputTokens + usage.totalOutputTokens;
  const maxModelCost = Math.max(...usage.byModel.map((m) => m.cost), 0.01);
  const maxAgentCost = Math.max(...usage.byAgent.map((a) => a.cost), 0.01);

  return (
    <PageLayout
      title="Usage & Cost"
      icon={<DollarSign size={24} className="text-claw-400" />}
      subtitle={`Total spend: ${fmtCost(usage.totalCost)} · ${fmtTokens(totalTokens)} tokens`}
    >
      {/* KPI Row */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-6">
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <div className="flex items-center gap-2 text-gray-500 text-xs mb-2">
            <DollarSign size={12} /> Today
          </div>
          <div className="text-2xl font-bold text-white">{fmtCost(usage.todayCost)}</div>
        </div>
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <div className="flex items-center gap-2 text-gray-500 text-xs mb-2">
            <TrendingUp size={12} /> Last 7 Days
          </div>
          <div className="text-2xl font-bold text-white">{fmtCost(usage.weekCost)}</div>
        </div>
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <div className="flex items-center gap-2 text-gray-500 text-xs mb-2">
            <DollarSign size={12} /> All Time
          </div>
          <div className="text-2xl font-bold text-white">{fmtCost(usage.totalCost)}</div>
        </div>
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <div className="flex items-center gap-2 text-gray-500 text-xs mb-2">
            <Cpu size={12} /> Total Tokens
          </div>
          <div className="text-2xl font-bold text-white">{fmtTokens(totalTokens)}</div>
          <div className="text-xs text-gray-500 mt-1">
            {fmtTokens(usage.totalCacheReadTokens)} cache reads
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        {/* Daily Cost Trend */}
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <h3 className="text-sm font-medium text-gray-300 mb-4">Daily Cost Trend</h3>
          <DailyTrendChart data={usage.dailyTrend} />
        </div>

        {/* Per-Model Breakdown */}
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <h3 className="text-sm font-medium text-gray-300 mb-4">Cost by Model</h3>
          {usage.byModel.length === 0 ? (
            <div className="text-gray-500 text-sm py-4">No model data</div>
          ) : (
            <div className="space-y-3">
              {usage.byModel.map((m) => (
                <div key={m.model}>
                  <div className="flex items-center justify-between text-sm mb-1">
                    <span className="text-gray-300 font-mono text-xs truncate max-w-48">
                      {modelShort(m.model)}
                    </span>
                    <span className="text-white font-medium">{fmtCost(m.cost)}</span>
                  </div>
                  <CostBar value={m.cost} max={maxModelCost} color="bg-claw-500" />
                  <div className="flex gap-3 text-xs text-gray-500 mt-1">
                    <span>{fmtTokens(m.inputTokens)} in</span>
                    <span>{fmtTokens(m.outputTokens)} out</span>
                    <span>{m.turnCount} turns</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Per-Agent Breakdown */}
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <h3 className="text-sm font-medium text-gray-300 mb-4">Cost by Agent</h3>
          {usage.byAgent.length === 0 ? (
            <div className="text-gray-500 text-sm py-4">No agent data</div>
          ) : (
            <div className="space-y-3">
              {usage.byAgent.map((a) => (
                <div key={a.agentId}>
                  <div className="flex items-center justify-between text-sm mb-1">
                    <span className="text-gray-300">
                      {a.agentName || a.agentId}
                      <span className="text-gray-600 ml-2 text-xs">
                        {a.sessionCount} session{a.sessionCount !== 1 ? "s" : ""}
                      </span>
                    </span>
                    <span className="text-white font-medium">{fmtCost(a.cost)}</span>
                  </div>
                  <CostBar value={a.cost} max={maxAgentCost} color="bg-blue-500" />
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Bloated Sessions */}
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <h3 className="text-sm font-medium text-gray-300 mb-4 flex items-center gap-2">
            <AlertTriangle size={14} className="text-yellow-500" />
            Bloated Sessions
          </h3>
          {usage.bloatedSessions.length === 0 ? (
            <div className="text-gray-500 text-sm py-4">
              No bloated sessions (&gt;500KB). All clear.
            </div>
          ) : (
            <div className="space-y-2">
              {usage.bloatedSessions.map((s) => (
                <div
                  key={`${s.agentId}-${s.sessionKey}`}
                  className="flex items-center justify-between bg-yellow-900/10 border border-yellow-900/30 rounded-lg px-3 py-2"
                >
                  <div>
                    <span className="text-sm text-gray-300 font-mono text-xs">
                      {s.sessionKey.length > 40 ? s.sessionKey.slice(0, 40) + "…" : s.sessionKey}
                    </span>
                    <span className="text-xs text-gray-500 ml-2">{s.agentId}</span>
                  </div>
                  <span className="text-yellow-400 text-sm font-medium">{s.sizeDisplay}</span>
                </div>
              ))}
              <p className="text-xs text-gray-500 mt-2">
                Large session files increase token costs. Reset with <code className="bg-gray-800 px-1 rounded">/new</code>
              </p>
            </div>
          )}
        </div>
      </div>
    </PageLayout>
  );
}
