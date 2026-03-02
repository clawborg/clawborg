import { useState, useEffect } from "react";
import { Settings, Shield } from "lucide-react";
import { fetchConfig } from "@/api/client";
import PageLayout from "@/components/PageLayout";

export default function Config() {
  const [config, setConfig] = useState<Record<string, unknown> | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchConfig()
      .then(setConfig)
      .finally(() => setLoading(false));
  }, []);

  return (
    <PageLayout
      title="Configuration"
      icon={<Settings size={24} className="text-claw-400" />}
      subtitle={
        <span className="flex items-center gap-1">
          <Shield size={12} />
          Read-only — tokens and API keys are redacted
        </span>
      }
    >
      {loading ? (
        <div className="text-gray-500">Loading config...</div>
      ) : (
        <div className="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden">
          <div className="px-4 py-2 border-b border-gray-800 bg-gray-900/50">
            <span className="text-xs text-gray-500 font-mono">
              openclaw.json
            </span>
          </div>
          <pre className="p-4 text-sm text-gray-300 font-mono whitespace-pre-wrap overflow-auto max-h-[40rem]">
            {config
              ? JSON.stringify(config, null, 2)
              : "Failed to load config"}
          </pre>
        </div>
      )}
    </PageLayout>
  );
}
