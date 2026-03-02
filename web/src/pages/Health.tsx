import { useState, useEffect } from "react";
import {
  HeartPulse,
  CheckCircle,
  AlertTriangle,
  XCircle,
  RefreshCw,
} from "lucide-react";
import { fetchHealth } from "@/api/client";
import type { HealthReport } from "@/lib/types";
import PageLayout from "@/components/PageLayout";

const severityIcon: Record<string, React.ReactNode> = {
  critical: <XCircle size={14} className="text-red-400 shrink-0" />,
  warning: <AlertTriangle size={14} className="text-yellow-400 shrink-0" />,
  info: <CheckCircle size={14} className="text-blue-400 shrink-0" />,
};

const statusBg: Record<string, string> = {
  healthy: "border-green-900/50 bg-green-900/10",
  warning: "border-yellow-900/50 bg-yellow-900/10",
  critical: "border-red-900/50 bg-red-900/10",
};

export default function Health() {
  const [report, setReport] = useState<HealthReport | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = () => {
    setLoading(true);
    fetchHealth()
      .then(setReport)
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
  }, []);

  return (
    <PageLayout
      title="Health Audit"
      icon={<HeartPulse size={24} className="text-claw-400" />}
      subtitle={
        report && (
          <span>
            {report.totalAgents} agents scanned — {report.healthyAgents}{" "}
            healthy — {report.totalIssues} issues
          </span>
        )
      }
      action={
        <button
          onClick={refresh}
          disabled={loading}
          className="flex items-center gap-2 text-sm bg-gray-800 hover:bg-gray-700 px-3 py-2 rounded-lg disabled:opacity-50 transition-colors"
        >
          <RefreshCw size={14} className={loading ? "animate-spin" : ""} />
          Re-scan
        </button>
      }
    >
      {loading && !report ? (
        <div className="text-gray-500">Running health audit...</div>
      ) : !report ? (
        <div className="text-red-400">Audit failed</div>
      ) : report.totalIssues === 0 ? (
        <div className="bg-green-900/10 border border-green-900/50 rounded-xl p-6 text-center">
          <CheckCircle size={32} className="text-green-400 mx-auto mb-2" />
          <p className="text-green-300 font-medium">All agents healthy!</p>
        </div>
      ) : (
        <div className="space-y-3">
          {report.agents.map((agent) => (
            <div
              key={agent.agentId}
              className={`border rounded-xl p-4 ${statusBg[agent.status] || ""}`}
            >
              <div className="flex items-center gap-2 mb-2">
                {agent.status === "healthy" && (
                  <CheckCircle size={16} className="text-green-400" />
                )}
                {agent.status === "warning" && (
                  <AlertTriangle size={16} className="text-yellow-400" />
                )}
                {agent.status === "critical" && (
                  <XCircle size={16} className="text-red-400" />
                )}
                <span className="font-semibold text-sm">{agent.agentId}</span>
              </div>

              {agent.issues.length === 0 ? (
                <p className="text-xs text-gray-500 ml-6">All checks passed</p>
              ) : (
                <div className="ml-6 space-y-1">
                  {agent.issues.map((issue, i) => (
                    <div
                      key={i}
                      className="flex items-start gap-2 text-xs text-gray-300"
                    >
                      {severityIcon[issue.severity]}
                      <span>{issue.message}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </PageLayout>
  );
}
