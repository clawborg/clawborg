import { useState, useEffect } from "react";
import { useAgents } from "../hooks/useAgents";
import { fetchSessions, fetchUsage } from "@/api/client";
import type { AgentSummary, SessionSummary, UsageSummary } from "../lib/types";
import { Link } from "react-router-dom";
import {
  LayoutDashboard,
  Radio,
  DollarSign,
  HeartPulse,
  AlertTriangle,
} from "lucide-react";
import PageLayout from "@/components/PageLayout";

/* ─── Helpers ─── */

function healthBadge(status: string) {
  const colors: Record<string, string> = {
    healthy: "bg-green-900/50 text-green-400 border-green-800",
    warning: "bg-yellow-900/50 text-yellow-400 border-yellow-800",
    critical: "bg-red-900/50 text-red-400 border-red-800",
  };
  return colors[status] || colors.healthy;
}

/* ─── KPI Card ─── */

function KpiCard({
  label,
  value,
  sub,
  icon,
}: {
  label: string;
  value: string | number;
  sub?: string;
  icon: React.ReactNode;
}) {
  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
      <div className="flex items-center gap-2 text-gray-500 text-xs mb-2">
        {icon}
        {label}
      </div>
      <div className="text-2xl font-bold text-claw-100">{value}</div>
      {sub && <div className="text-xs text-gray-500 mt-1">{sub}</div>}
    </div>
  );
}

/* ─── Agent Card ─── */

function AgentCard({ agent }: { agent: AgentSummary }) {
  const displayName = agent.name || agent.id;
  const model = agent.model?.split("/").pop() || "unknown";

  return (
    <Link
      to={`/agents/${agent.id}`}
      className="block bg-gray-900 border border-gray-800 rounded-xl p-4 hover:border-claw-700 transition-colors"
    >
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-claw-100 font-medium truncate">{displayName}</h3>
        <span
          className={`text-xs px-2 py-0.5 rounded border ${healthBadge(agent.health.status)}`}
        >
          {agent.health.status}
        </span>
      </div>

      <div className="space-y-1.5 text-sm text-gray-400">
        <div className="flex justify-between">
          <span>ID</span>
          <span className="text-gray-300 font-mono text-xs">{agent.id}</span>
        </div>
        <div className="flex justify-between">
          <span>Model</span>
          <span className="text-gray-300 text-xs">{model}</span>
        </div>
        <div className="flex justify-between">
          <span>Files</span>
          <span className="text-gray-300">{agent.fileCount}</span>
        </div>
        <div className="flex justify-between">
          <span>Sessions</span>
          <span className="text-gray-300">{agent.sessionCount}</span>
        </div>
        {agent.hasTasks && agent.pendingTasks > 0 && (
          <div className="flex justify-between">
            <span>Pending tasks</span>
            <span className="text-yellow-400 font-medium">
              {agent.pendingTasks}
            </span>
          </div>
        )}
      </div>

      {/* Footer: issues + default badge */}
      <div className="flex items-center justify-between mt-3 pt-3 border-t border-gray-800">
        {agent.health.issues.length > 0 ? (
          <span className="text-xs text-yellow-500 flex items-center gap-1">
            <AlertTriangle size={10} />
            {agent.health.issues.length} issue
            {agent.health.issues.length !== 1 ? "s" : ""}
          </span>
        ) : (
          <span />
        )}
        {agent.isDefault && (
          <span className="text-xs text-gray-500 bg-gray-800 px-1.5 py-0.5 rounded">
            default
          </span>
        )}
      </div>
    </Link>
  );
}

/* ─── Dashboard Page ─── */

export default function Dashboard() {
  const { agents, loading, error } = useAgents();
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [usage, setUsage] = useState<UsageSummary | null>(null);

  useEffect(() => {
    fetchSessions().then(setSessions).catch(() => {});
    fetchUsage().then(setUsage).catch(() => {});
  }, []);

  if (loading) {
    return (
      <PageLayout title="Agent Fleet" icon={<LayoutDashboard size={24} />}>
        <div className="flex items-center justify-center h-64 text-gray-400">
          Loading agents...
        </div>
      </PageLayout>
    );
  }

  if (error) {
    return (
      <PageLayout title="Agent Fleet" icon={<LayoutDashboard size={24} />}>
        <div className="bg-red-900/20 border border-red-800 rounded-lg p-4 text-red-400">
          {error}
        </div>
      </PageLayout>
    );
  }

  const healthy = agents.filter((a) => a.health.status === "healthy").length;
  const warning = agents.filter((a) => a.health.status === "warning").length;
  const critical = agents.filter((a) => a.health.status === "critical").length;
  const totalIssues = agents.reduce(
    (sum, a) => sum + a.health.issues.length,
    0
  );

  const activeSessions = sessions.filter((s) => s.status === "active").length;
  const todayCost = usage?.todayCost ?? 0;
  const weekCost = usage?.weekCost ?? 0;

  return (
    <PageLayout
      title="Agent Fleet"
      icon={<LayoutDashboard size={24} className="text-claw-400" />}
      subtitle={
        <span>
          {agents.length} agent{agents.length !== 1 ? "s" : ""} —{" "}
          <span className="text-green-400">{healthy} healthy</span>
          {warning > 0 && (
            <>
              , <span className="text-yellow-400">{warning} warning</span>
            </>
          )}
          {critical > 0 && (
            <>
              , <span className="text-red-400">{critical} critical</span>
            </>
          )}
        </span>
      }
    >
      {/* KPI row */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-6">
        <KpiCard
          label="Agents"
          value={agents.length}
          sub={totalIssues > 0 ? `${totalIssues} issues` : "All clear"}
          icon={<LayoutDashboard size={12} />}
        />
        <KpiCard
          label="Sessions"
          value={sessions.length}
          sub={`${activeSessions} active`}
          icon={<Radio size={12} />}
        />
        <KpiCard
          label="Today's Cost"
          value={todayCost >= 0.01 ? `$${todayCost.toFixed(2)}` : `$${todayCost.toFixed(4)}`}
          sub={`$${weekCost.toFixed(2)} this week`}
          icon={<DollarSign size={12} />}
        />
        <KpiCard
          label="Health"
          value={`${healthy}/${agents.length}`}
          sub={totalIssues > 0 ? `${totalIssues} issues found` : "All checks passing"}
          icon={<HeartPulse size={12} />}
        />
      </div>

      {/* Agent cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
        {agents.map((agent) => (
          <AgentCard key={agent.id} agent={agent} />
        ))}
      </div>
    </PageLayout>
  );
}
