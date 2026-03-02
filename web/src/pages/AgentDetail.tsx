import { useState, useEffect, useMemo } from "react";
import { useParams, Link } from "react-router-dom";
import {
  ArrowLeft,
  FileText,
  Folder,
  Save,
  AlertTriangle,
  CheckCircle,
  XCircle,
  Edit3,
  Eye,
} from "lucide-react";
import { fetchAgent, fetchFile, updateFile } from "@/api/client";
import type { AgentDetail as AgentDetailType } from "@/lib/types";
import PageLayout from "@/components/PageLayout";

/** Standard OpenClaw files — shown first in tab order if they exist */
const PRIORITY_FILES = [
  "AGENTS.md",
  "SOUL.md",
  "IDENTITY.md",
  "USER.md",
  "TOOLS.md",
  "HEARTBEAT.md",
  "MEMORY.md",
];

/** Files that can be edited inline */
const EDITABLE_EXTENSIONS = [".md"];

export default function AgentDetail() {
  const { id } = useParams<{ id: string }>();
  const [agent, setAgent] = useState<AgentDetailType | null>(null);
  const [activeFile, setActiveFile] = useState<string | null>(null);
  const [fileContent, setFileContent] = useState("");
  const [editContent, setEditContent] = useState("");
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Sort files: priority files first, then alphabetical
  const sortedFiles = useMemo(() => {
    if (!agent) return [];
    const fileNames = Object.keys(agent.files);
    return fileNames.sort((a, b) => {
      const ai = PRIORITY_FILES.indexOf(a);
      const bi = PRIORITY_FILES.indexOf(b);
      if (ai !== -1 && bi !== -1) return ai - bi;
      if (ai !== -1) return -1;
      if (bi !== -1) return 1;
      return a.localeCompare(b);
    });
  }, [agent]);

  // Load agent
  useEffect(() => {
    if (!id) return;
    setLoading(true);
    fetchAgent(id)
      .then((data) => {
        setAgent(data);
        // Auto-select first available file
        const files = Object.keys(data.files);
        const first =
          PRIORITY_FILES.find((f) => files.includes(f)) || files[0] || null;
        setActiveFile(first);
      })
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, [id]);

  // Load file content
  useEffect(() => {
    if (!id || !activeFile) return;
    setEditing(false);
    fetchFile(id, activeFile)
      .then((data) => {
        setFileContent(data.content);
        setEditContent(data.content);
      })
      .catch(() => {
        setFileContent("(file not found)");
        setEditContent("");
      });
  }, [id, activeFile]);

  const handleSave = async () => {
    if (!id || !activeFile) return;
    setSaving(true);
    try {
      await updateFile(id, activeFile, editContent);
      setFileContent(editContent);
      setEditing(false);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Save failed");
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <PageLayout title="Loading...">
        <div className="text-gray-500">Loading agent...</div>
      </PageLayout>
    );
  }

  if (error || !agent) {
    return (
      <PageLayout title="Error">
        <div className="text-red-400">{error || "Agent not found"}</div>
      </PageLayout>
    );
  }

  const isEditable =
    activeFile && EDITABLE_EXTENSIONS.some((ext) => activeFile.endsWith(ext));
  const fileInfo = activeFile ? agent.files[activeFile] : null;

  return (
    <PageLayout
      title={agent.name || agent.id}
      subtitle={
        <div className="flex flex-wrap items-center gap-2 text-xs">
          {agent.model && (
            <span className="bg-gray-800 px-2 py-0.5 rounded font-mono text-gray-400">
              {agent.model.split("/").pop()}
            </span>
          )}
          {agent.isDefault && (
            <span className="bg-gray-800 px-2 py-0.5 rounded text-gray-500">
              default
            </span>
          )}
          <span className="text-gray-500">
            {sortedFiles.length} files
            {agent.directories.length > 0 &&
              ` · ${agent.directories.length} dirs`}
          </span>
        </div>
      }
      icon={
        <Link
          to="/"
          className="text-gray-500 hover:text-gray-300 transition-colors"
        >
          <ArrowLeft size={20} />
        </Link>
      }
    >
      {/* Health issues */}
      {agent.health.issues.length > 0 && (
        <div className="mb-4 space-y-2">
          {agent.health.issues.map((issue, i) => (
            <div
              key={i}
              className={`flex items-start gap-2 px-3 py-2 rounded-lg text-sm ${
                issue.severity === "critical"
                  ? "bg-red-900/20 text-red-300"
                  : issue.severity === "warning"
                    ? "bg-yellow-900/20 text-yellow-300"
                    : "bg-blue-900/20 text-blue-300"
              }`}
            >
              {issue.severity === "critical" ? (
                <XCircle size={14} className="shrink-0 mt-0.5" />
              ) : issue.severity === "warning" ? (
                <AlertTriangle size={14} className="shrink-0 mt-0.5" />
              ) : (
                <CheckCircle size={14} className="shrink-0 mt-0.5" />
              )}
              <span>{issue.message}</span>
            </div>
          ))}
        </div>
      )}

      {/* Summary row: tasks + directories */}
      <div className="flex flex-wrap gap-3 mb-4 text-sm">
        {agent.tasks && (
          <>
            <span className="text-yellow-400">
              {agent.tasks.pending} pending
            </span>
            <span className="text-blue-400">
              {agent.tasks.approved} approved
            </span>
            <span className="text-green-400">{agent.tasks.done} done</span>
          </>
        )}
        {agent.directories.length > 0 && (
          <span className="text-gray-500 flex items-center gap-1">
            <Folder size={12} />
            {agent.directories.join(", ")}
          </span>
        )}
      </div>

      {/* File tabs — scrollable on mobile */}
      {sortedFiles.length > 0 ? (
        <>
          <div className="flex gap-1 mb-4 border-b border-gray-800 pb-px overflow-x-auto scrollbar-none">
            {sortedFiles.map((fname) => {
              const info = agent.files[fname];
              return (
                <button
                  key={fname}
                  onClick={() => setActiveFile(fname)}
                  className={`shrink-0 px-3 py-2 text-xs rounded-t-lg transition-colors flex items-center gap-1.5 ${
                    activeFile === fname
                      ? "bg-gray-800 text-white"
                      : "text-gray-500 hover:text-gray-300"
                  } ${!info?.exists ? "opacity-40" : ""}`}
                >
                  <FileText size={12} />
                  {fname}
                  {info?.isEmpty && info?.exists && (
                    <span className="w-1.5 h-1.5 rounded-full bg-red-500" />
                  )}
                </button>
              );
            })}
          </div>

          {/* File viewer / editor */}
          <div className="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden">
            <div className="flex items-center justify-between px-4 py-2 border-b border-gray-800 bg-gray-900/50">
              <span className="text-xs text-gray-500 font-mono">
                {activeFile}
                {fileInfo && !fileInfo.isEmpty && (
                  <span className="ml-2 text-gray-600">
                    {fileInfo.sizeBytes > 1024
                      ? `${(fileInfo.sizeBytes / 1024).toFixed(1)} KB`
                      : `${fileInfo.sizeBytes} B`}
                  </span>
                )}
              </span>
              <div className="flex gap-2">
                {isEditable && !editing && (
                  <button
                    onClick={() => setEditing(true)}
                    className="flex items-center gap-1 text-xs text-claw-400 hover:text-claw-300"
                  >
                    <Edit3 size={12} /> Edit
                  </button>
                )}
                {isEditable && editing && (
                  <>
                    <button
                      onClick={() => {
                        setEditing(false);
                        setEditContent(fileContent);
                      }}
                      className="flex items-center gap-1 text-xs text-gray-400 hover:text-gray-200"
                    >
                      <Eye size={12} /> Cancel
                    </button>
                    <button
                      onClick={handleSave}
                      disabled={saving}
                      className="flex items-center gap-1 text-xs bg-claw-600 hover:bg-claw-500 text-white px-3 py-1 rounded disabled:opacity-50"
                    >
                      <Save size={12} /> {saving ? "Saving..." : "Save"}
                    </button>
                  </>
                )}
              </div>
            </div>

            {editing ? (
              <textarea
                value={editContent}
                onChange={(e) => setEditContent(e.target.value)}
                className="w-full h-96 p-4 bg-gray-950 text-gray-200 font-mono text-sm resize-none focus:outline-none"
                spellCheck={false}
              />
            ) : (
              <pre className="p-4 text-sm text-gray-300 font-mono whitespace-pre-wrap overflow-auto max-h-[32rem]">
                {fileContent}
              </pre>
            )}
          </div>
        </>
      ) : (
        <div className="bg-gray-900 rounded-xl border border-gray-800 p-8 text-center text-gray-500">
          No files found in workspace
        </div>
      )}
    </PageLayout>
  );
}
