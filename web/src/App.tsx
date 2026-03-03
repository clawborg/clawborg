import { useState, useEffect } from "react";
import { Routes, Route, NavLink } from "react-router-dom";
import {
  LayoutDashboard,
  Activity,
  HeartPulse,
  Settings,
  Radio,
  Menu,
  X,
  DollarSign,
  Clock,
  AlertTriangle,
  XCircle,
  Info,
} from "lucide-react";
import { fetchAlerts } from "@/api/client";
import type { Alert } from "@/lib/types";
import Dashboard from "./pages/Dashboard";
import AgentDetail from "./pages/AgentDetail";
import Sessions from "./pages/Sessions";
import Health from "./pages/Health";
import Config from "./pages/Config";
import Usage from "./pages/Usage";
import Crons from "./pages/Crons";

const navItems = [
  { to: "/", icon: LayoutDashboard, label: "Agents" },
  { to: "/usage", icon: DollarSign, label: "Usage" },
  { to: "/sessions", icon: Radio, label: "Sessions" },
  { to: "/crons", icon: Clock, label: "Crons" },
  { to: "/health", icon: HeartPulse, label: "Health" },
  { to: "/config", icon: Settings, label: "Config" },
];

function AlertsBanner({ alerts, onDismiss }: { alerts: Alert[]; onDismiss: () => void }) {
  if (alerts.length === 0) return null;

  const severityIcon = (s: string) => {
    if (s === "critical") return <XCircle size={14} className="text-red-400 shrink-0" />;
    if (s === "warning") return <AlertTriangle size={14} className="text-yellow-400 shrink-0" />;
    return <Info size={14} className="text-blue-400 shrink-0" />;
  };

  const hasCritical = alerts.some((a) => a.severity === "critical");
  const borderColor = hasCritical ? "border-red-800 bg-red-950/40" : "border-yellow-800 bg-yellow-950/30";

  return (
    <div className={`mx-4 mt-4 sm:mx-6 lg:mx-8 border rounded-lg px-4 py-3 ${borderColor}`}>
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-medium text-gray-300 uppercase tracking-wide">
          {alerts.length} Alert{alerts.length !== 1 ? "s" : ""}
        </span>
        <button onClick={onDismiss} className="text-gray-500 hover:text-gray-300">
          <X size={14} />
        </button>
      </div>
      <div className="space-y-1.5">
        {alerts.slice(0, 5).map((alert) => (
          <div key={alert.id} className="flex items-start gap-2 text-sm">
            {severityIcon(alert.severity)}
            <span className="text-gray-300">
              <span className="font-medium">{alert.title}:</span>{" "}
              <span className="text-gray-400">{alert.message}</span>
            </span>
          </div>
        ))}
        {alerts.length > 5 && (
          <div className="text-xs text-gray-500 pl-6">
            +{alerts.length - 5} more alert{alerts.length - 5 !== 1 ? "s" : ""}
          </div>
        )}
      </div>
    </div>
  );
}

export default function App() {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [alertsDismissed, setAlertsDismissed] = useState(false);

  useEffect(() => {
    fetchAlerts().then(setAlerts).catch(() => {});
    const interval = setInterval(() => {
      fetchAlerts().then(setAlerts).catch(() => {});
    }, 60_000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="min-h-screen flex bg-gray-950 text-gray-100">
      {/* Mobile overlay */}
      {sidebarOpen && (
        <div
          className="fixed inset-0 bg-black/60 z-30 lg:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      {/* Sidebar — always fixed so it doesn't scroll with page content */}
      <aside
        className={`
          fixed inset-y-0 left-0 z-40 w-56 bg-gray-900 border-r border-gray-800 flex flex-col
          transform transition-transform duration-200 ease-in-out
          lg:translate-x-0
          ${sidebarOpen ? "translate-x-0" : "-translate-x-full"}
        `}
      >
        <div className="p-4 border-b border-gray-800 flex items-center justify-between">
          <div>
            <h1 className="text-lg font-bold flex items-center gap-2">
              <img src="/logo-128.png" alt="ClawBorg" className="w-8 h-8 rounded" />
              <span>ClawBorg</span>
            </h1>
            <p className="text-xs text-gray-500 mt-1">v0.2 · Agent Fleet Dashboard</p>
          </div>
          <button
            onClick={() => setSidebarOpen(false)}
            className="lg:hidden p-1 text-gray-400 hover:text-white"
          >
            <X size={18} />
          </button>
        </div>

        <nav className="flex-1 p-2 space-y-1">
          {navItems.map(({ to, icon: Icon, label }) => (
            <NavLink
              key={to}
              to={to}
              end={to === "/"}
              onClick={() => setSidebarOpen(false)}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${
                  isActive
                    ? "bg-claw-600/20 text-claw-400"
                    : "text-gray-400 hover:bg-gray-800 hover:text-gray-200"
                }`
              }
            >
              <Icon size={18} />
              {label}
            </NavLink>
          ))}
        </nav>

        <div className="p-4 border-t border-gray-800">
          <div className="flex items-center gap-2 text-xs text-gray-500">
            <Activity size={12} className="text-claw-500 animate-pulse" />
            <span>Live</span>
          </div>
        </div>
      </aside>

      {/* Main content — lg:pl-56 offsets the fixed sidebar width */}
      <div className="flex-1 flex flex-col min-w-0 lg:pl-56">
        {/* Mobile header */}
        <header className="lg:hidden sticky top-0 z-20 bg-gray-900/95 backdrop-blur border-b border-gray-800 px-4 py-3 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <button
              onClick={() => setSidebarOpen(true)}
              className="p-1 text-gray-400 hover:text-white"
            >
              <Menu size={20} />
            </button>
            <span className="text-sm font-semibold flex items-center gap-2">
              <img src="/logo-128.png" alt="ClawBorg" className="w-6 h-6 rounded" /> ClawBorg
            </span>
          </div>
          <div className="flex items-center gap-2 text-xs text-gray-500">
            <Activity size={12} className="text-claw-500 animate-pulse" />
            <span>Live</span>
          </div>
        </header>

        {/* Smart Alerts Banner */}
        {!alertsDismissed && (
          <AlertsBanner alerts={alerts} onDismiss={() => setAlertsDismissed(true)} />
        )}

        <main className="flex-1 overflow-auto">
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/agents/:id" element={<AgentDetail />} />
            <Route path="/usage" element={<Usage />} />
            <Route path="/sessions" element={<Sessions />} />
            <Route path="/crons" element={<Crons />} />
            <Route path="/health" element={<Health />} />
            <Route path="/config" element={<Config />} />
          </Routes>
        </main>
      </div>
    </div>
  );
}
