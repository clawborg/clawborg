import { useState, useEffect } from "react";
import { fetchUsage } from "@/api/client";
import type { DailyCost, UsageSummary } from "@/lib/types";
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

/* ─── Daily Trend Chart ─── */

type ViewMode = "daily" | "weekly";

interface ChartBar {
  label: string;
  tooltipLabel: string;
  cost: number;
  tokens: number;
}

const MONTHS = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
const DAYS_SHORT = ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"];

function fmtDateShort(isoDate: string): string {
  const [, m, d] = isoDate.split("-");
  return `${MONTHS[parseInt(m) - 1]} ${parseInt(d)}`;
}

function fmtDateFull(isoDate: string): string {
  const dt = new Date(isoDate + "T00:00:00Z");
  return `${DAYS_SHORT[dt.getUTCDay()]}, ${MONTHS[dt.getUTCMonth()]} ${dt.getUTCDate()}`;
}

function fmtYAxis(n: number): string {
  if (n === 0) return "$0";
  if (n >= 10) return `$${n.toFixed(0)}`;
  if (n >= 1) return `$${n.toFixed(1)}`;
  if (n >= 0.1) return `$${n.toFixed(2)}`;
  return `$${n.toFixed(3)}`;
}

function toDailyBars(data: DailyCost[]): ChartBar[] {
  return data.map((d) => ({
    label: d.date.slice(5).replace("-", "/"),
    tooltipLabel: fmtDateFull(d.date),
    cost: d.cost,
    tokens: d.inputTokens + d.outputTokens,
  }));
}

function toWeeklyBars(data: DailyCost[]): ChartBar[] {
  const weekMap = new Map<string, ChartBar>();
  for (const d of data) {
    const dt = new Date(d.date + "T00:00:00Z");
    const daysToMon = (dt.getUTCDay() + 6) % 7;
    const mon = new Date(dt);
    mon.setUTCDate(dt.getUTCDate() - daysToMon);
    const key = mon.toISOString().slice(0, 10);
    const existing = weekMap.get(key);
    if (existing) {
      existing.cost += d.cost;
      existing.tokens += d.inputTokens + d.outputTokens;
    } else {
      weekMap.set(key, {
        label: key.slice(5).replace("-", "/"),
        tooltipLabel: `Wk of ${fmtDateShort(key)}`,
        cost: d.cost,
        tokens: d.inputTokens + d.outputTokens,
      });
    }
  }
  return [...weekMap.entries()]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([, v]) => v);
}

const CHART_H = 180;
const Y_AXIS_W = 48;
const Y_FRACS = [0, 0.25, 0.5, 0.75, 1.0];

function DailyTrendChart({ data }: { data: DailyCost[] }) {
  const [view, setView] = useState<ViewMode>("daily");
  const [hovered, setHovered] = useState<number | null>(null);

  const bars = view === "daily" ? toDailyBars(data) : toWeeklyBars(data);
  const maxCost = bars.length > 0 ? Math.max(...bars.map((b) => b.cost), 0.01) : 0.01;
  const labelEvery = bars.length <= 10 ? 1 : bars.length <= 20 ? 2 : 5;
  const hoveredBar = hovered !== null ? bars[hovered] : null;

  return (
    <div>
      {/* Header: title + view toggle */}
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-medium text-gray-300">Daily Cost Trend</h3>
        <div className="flex rounded border border-gray-700 overflow-hidden text-xs">
          {(["daily", "weekly"] as ViewMode[]).map((v) => (
            <button
              key={v}
              onClick={() => setView(v)}
              className={`px-2.5 py-1 transition-colors capitalize ${
                view === v
                  ? "bg-gray-700 text-gray-100"
                  : "text-gray-500 hover:text-gray-300 hover:bg-gray-800"
              }`}
            >
              {v}
            </button>
          ))}
        </div>
      </div>

      {bars.length === 0 ? (
        <div className="text-gray-500 text-sm py-8 text-center">No data yet</div>
      ) : (
        <div className="flex items-start gap-2">
          {/* Y-axis labels */}
          <div
            className="flex flex-col justify-between text-right shrink-0"
            style={{ width: Y_AXIS_W, height: CHART_H }}
          >
            {[...Y_FRACS].reverse().map((f) => (
              <span key={f} className="text-xs text-gray-600 leading-none">
                {fmtYAxis(maxCost * f)}
              </span>
            ))}
          </div>

          {/* Bars + x-axis */}
          <div className="flex-1 min-w-0">
            {/* Bar area */}
            <div className="relative" style={{ height: CHART_H }}>
              {/* Horizontal grid lines */}
              {Y_FRACS.slice(1).map((f) => (
                <div
                  key={f}
                  className="absolute left-0 right-0 border-t border-gray-800"
                  style={{ top: `${(1 - f) * 100}%` }}
                />
              ))}

              {/* Bars */}
              <div className="absolute inset-0 flex items-end gap-px">
                {bars.map((bar, i) => {
                  const h = Math.max(1, (bar.cost / maxCost) * 100);
                  return (
                    <div
                      key={i}
                      className="relative flex-1 h-full flex items-end"
                      onMouseEnter={() => setHovered(i)}
                      onMouseLeave={() => setHovered(null)}
                    >
                      <div
                        className={`w-full rounded-t-sm transition-colors ${
                          hovered === i ? "bg-claw-400" : "bg-claw-500"
                        }`}
                        style={{ height: `${h}%` }}
                      />
                    </div>
                  );
                })}
              </div>

              {/* Hover tooltip */}
              {hoveredBar !== null && hovered !== null && (
                <div
                  className="absolute z-20 bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-xs pointer-events-none shadow-xl whitespace-nowrap"
                  style={{
                    bottom: `${Math.min(78, Math.max(8, (hoveredBar.cost / maxCost) * 100) + 6)}%`,
                    ...(hovered >= bars.length * 0.6
                      ? { right: `${Math.max(0, ((bars.length - 1 - hovered) / bars.length) * 100)}%` }
                      : { left: `${Math.max(0, (hovered / bars.length) * 100)}%` }),
                  }}
                >
                  <div className="font-medium text-gray-200 mb-0.5">{hoveredBar.tooltipLabel}</div>
                  <div className="text-claw-400 font-bold">{fmtCost(hoveredBar.cost)}</div>
                  <div className="text-gray-500 mt-0.5">{fmtTokens(hoveredBar.tokens)} tokens</div>
                </div>
              )}
            </div>

            {/* X-axis labels */}
            <div className="flex gap-px mt-1.5">
              {bars.map((bar, i) => (
                <div key={i} className="flex-1 overflow-hidden text-center">
                  {(i % labelEvery === 0 || i === bars.length - 1) && (
                    <span className="text-xs text-gray-600 block truncate">{bar.label}</span>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
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

  const totalTokens =
    usage.totalInputTokens +
    usage.totalOutputTokens +
    usage.totalCacheReadTokens +
    usage.totalCacheWriteTokens;
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
