import { useState, useEffect, useMemo } from "react";
import { useParams, Link } from "react-router-dom";
import {
  ArrowLeft,
  FileText,
  Folder,
  FolderOpen,
  Save,
  AlertTriangle,
  CheckCircle,
  XCircle,
  Edit3,
  Eye,
  Code2,
  ChevronRight,
} from "lucide-react";
import { marked } from "marked";
import { fetchAgent, fetchFile, fetchDirListing } from "@/api/client";
import type { AgentDetail as AgentDetailType, DirListing, FileInfo } from "@/lib/types";
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

/** Files that can be edited inline (workspace only) */
const EDITABLE_EXTENSIONS = [".md"];

// ─── Section descriptor ────────────────────────────────────────────────────

interface Section {
  label: string;
  /** "workspace" or the named dir label */
  sectionKey: string;
  files: Record<string, FileInfo>;
  directories: string[];
  editable: boolean;
}

// ─── Component ────────────────────────────────────────────────────────────

export default function AgentDetail() {
  const { id } = useParams<{ id: string }>();
  const [agent, setAgent] = useState<AgentDetailType | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Active section (index into sections array)
  const [activeSectionIdx, setActiveSectionIdx] = useState(0);
  // Breadcrumb path within the active section
  const [currentPath, setCurrentPath] = useState<string[]>([]);
  // Directory listing when navigated into a subdir
  const [dirListing, setDirListing] = useState<DirListing | null>(null);
  const [dirLoading, setDirLoading] = useState(false);

  // File viewer state
  const [activeFile, setActiveFile] = useState<string | null>(null);
  const [fileContent, setFileContent] = useState("");
  const [editContent, setEditContent] = useState("");
  const [editing, setEditing] = useState(false);
  const [viewMode, setViewMode] = useState<"preview" | "code">("preview");
  const [saving, setSaving] = useState(false);
  const [fileLoading, setFileLoading] = useState(false);
  const [previewHtml, setPreviewHtml] = useState<string | null>(null);

  // ── Load agent ─────────────────────────────────────────────────────────

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    fetchAgent(id)
      .then((data) => {
        setAgent(data);
        const files = Object.keys(data.files);
        const first =
          PRIORITY_FILES.find((f) => files.includes(f)) || files[0] || null;
        setActiveFile(first);
      })
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, [id]);

  // ── Build sections list ────────────────────────────────────────────────

  const sections: Section[] = useMemo(() => {
    if (!agent) return [];
    const result: Section[] = [
      {
        label: "Workspace",
        sectionKey: "workspace",
        files: agent.files,
        directories: agent.directories,
        editable: true,
      },
    ];
    for (const s of agent.extraSections) {
      result.push({
        label: s.label,
        sectionKey: s.label,
        files: s.files,
        directories: s.directories,
        editable: false,
      });
    }
    return result;
  }, [agent]);

  const activeSection = sections[activeSectionIdx] ?? sections[0];

  // ── Directory navigation ───────────────────────────────────────────────

  // Which files/dirs to display: dir listing when navigated, section root otherwise
  const displayFiles = dirListing?.files ?? activeSection?.files ?? {};
  const displayDirs = dirListing?.directories ?? activeSection?.directories ?? [];

  const sortedFileNames = useMemo(() => {
    const names = Object.keys(displayFiles);
    if (activeSectionIdx === 0 && currentPath.length === 0) {
      // Workspace root: priority sort
      return names.sort((a, b) => {
        const ai = PRIORITY_FILES.indexOf(a);
        const bi = PRIORITY_FILES.indexOf(b);
        if (ai !== -1 && bi !== -1) return ai - bi;
        if (ai !== -1) return -1;
        if (bi !== -1) return 1;
        return a.localeCompare(b);
      });
    }
    return names.sort((a, b) => a.localeCompare(b));
  }, [displayFiles, activeSectionIdx, currentPath]);

  // Navigate into a subdirectory
  const navigateInto = (dirName: string) => {
    const newPath = [...currentPath, dirName];
    setCurrentPath(newPath);
    setActiveFile(null);
    setDirListing(null);
    setDirLoading(true);
    fetchDirListing(id!, newPath.join("/"), activeSection?.sectionKey)
      .then(setDirListing)
      .catch(() => setDirListing(null))
      .finally(() => setDirLoading(false));
  };

  // Navigate to a breadcrumb segment (index = -1 = section root)
  const navigateTo = (targetDepth: number) => {
    if (targetDepth < 0) {
      setCurrentPath([]);
      setDirListing(null);
      setActiveFile(
        activeSectionIdx === 0
          ? PRIORITY_FILES.find((f) => Object.keys(activeSection?.files ?? {}).includes(f)) ??
              Object.keys(activeSection?.files ?? {})[0] ??
              null
          : null
      );
      return;
    }
    const newPath = currentPath.slice(0, targetDepth + 1);
    setCurrentPath(newPath);
    setActiveFile(null);
    setDirLoading(true);
    fetchDirListing(id!, newPath.join("/"), activeSection?.sectionKey)
      .then(setDirListing)
      .catch(() => setDirListing(null))
      .finally(() => setDirLoading(false));
  };

  // Reset navigation when switching sections
  const switchSection = (idx: number) => {
    setActiveSectionIdx(idx);
    setCurrentPath([]);
    setDirListing(null);
    setActiveFile(null);
    setFileContent("");
    setPreviewHtml(null);
    setEditing(false);
  };

  // ── File loading ───────────────────────────────────────────────────────

  const fullFilePath = useMemo(
    () =>
      activeFile
        ? [...currentPath, activeFile].join("/")
        : null,
    [activeFile, currentPath]
  );

  useEffect(() => {
    if (!id || !fullFilePath) return;
    setEditing(false);
    setViewMode("preview");
    setFileContent("");
    setPreviewHtml(null);
    setFileLoading(true);
    const section =
      activeSectionIdx === 0 ? undefined : activeSection?.sectionKey;
    fetchFile(id, fullFilePath, section)
      .then((data) => {
        setFileContent(data.content);
        setEditContent(data.content);
      })
      .catch(() => {
        setFileContent("(file not found)");
        setEditContent("");
      })
      .finally(() => setFileLoading(false));
  }, [id, fullFilePath, activeSectionIdx, activeSection?.sectionKey]);

  // Markdown parsing (handles Promise return from marked)
  useEffect(() => {
    if (!fileContent || !activeFile?.endsWith(".md")) {
      setPreviewHtml(null);
      return;
    }
    Promise.resolve(marked.parse(fileContent)).then(setPreviewHtml);
  }, [fileContent, activeFile]);

  // ── Save ───────────────────────────────────────────────────────────────

  const handleSave = async () => {
    if (!id || !fullFilePath) return;
    setSaving(true);
    try {
      const { updateFile } = await import("@/api/client");
      await updateFile(id, fullFilePath, editContent);
      setFileContent(editContent);
      setEditing(false);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Save failed");
    } finally {
      setSaving(false);
    }
  };

  // ── Render ─────────────────────────────────────────────────────────────

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
    activeSection?.editable &&
    activeFile &&
    EDITABLE_EXTENSIONS.some((ext) => activeFile.endsWith(ext));
  const fileInfo = activeFile ? displayFiles[activeFile] : null;

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
            {Object.keys(agent.files).length} files
            {agent.directories.length > 0 &&
              ` · ${agent.directories.length} dirs`}
            {agent.extraSections.length > 0 &&
              ` · ${agent.extraSections.length} extra section${agent.extraSections.length > 1 ? "s" : ""}`}
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

      {/* Task summary */}
      {agent.tasks && (
        <div className="flex flex-wrap gap-3 mb-4 text-sm">
          <span className="text-yellow-400">{agent.tasks.pending} pending</span>
          <span className="text-blue-400">{agent.tasks.approved} approved</span>
          <span className="text-green-400">{agent.tasks.done} done</span>
        </div>
      )}

      {/* Section tabs — only shown when extra sections exist */}
      {sections.length > 1 && (
        <div className="flex gap-1 mb-3 flex-wrap">
          {sections.map((s, idx) => (
            <button
              key={s.sectionKey}
              onClick={() => switchSection(idx)}
              className={`px-3 py-1.5 text-xs rounded-lg font-medium transition-colors ${
                activeSectionIdx === idx
                  ? "bg-claw-600 text-white"
                  : "bg-gray-800 text-gray-400 hover:text-gray-200"
              }`}
            >
              {idx === 0 ? <FolderOpen size={11} className="inline mr-1" /> : <Folder size={11} className="inline mr-1" />}
              {s.label}
            </button>
          ))}
        </div>
      )}

      {/* Breadcrumb — shown when navigated into a subdir */}
      {currentPath.length > 0 && (
        <nav className="flex items-center gap-1 text-xs text-gray-500 mb-3 flex-wrap">
          <button
            onClick={() => navigateTo(-1)}
            className="hover:text-gray-300 transition-colors"
          >
            {activeSection?.label ?? "workspace"}
          </button>
          {currentPath.map((segment, i) => (
            <span key={i} className="flex items-center gap-1">
              <ChevronRight size={10} />
              {i === currentPath.length - 1 ? (
                <span className="text-gray-300">{segment}</span>
              ) : (
                <button
                  onClick={() => navigateTo(i)}
                  className="hover:text-gray-300 transition-colors"
                >
                  {segment}
                </button>
              )}
            </span>
          ))}
        </nav>
      )}

      {/* File viewer panel */}
      <div className="bg-gray-900 rounded-xl border border-gray-800 overflow-hidden flex flex-col min-h-[calc(100vh-20rem)]">
        {/* Toolbar */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-gray-800 bg-gray-900/50 shrink-0 gap-3 flex-wrap">
          {/* Directory chips */}
          <div className="flex items-center gap-2 min-w-0 flex-wrap">
            {dirLoading ? (
              <span className="text-xs text-gray-600">Loading...</span>
            ) : (
              displayDirs.map((dir) => (
                <button
                  key={dir}
                  onClick={() => navigateInto(dir)}
                  className="flex items-center gap-1 text-xs text-gray-400 hover:text-claw-300 bg-gray-800 hover:bg-gray-700 px-2 py-1 rounded transition-colors"
                >
                  <Folder size={11} />
                  {dir}
                </button>
              ))
            )}
            {activeFile && (
              <span className="text-xs text-gray-500 font-mono ml-1">
                {fullFilePath}
                {fileInfo && !fileInfo.isEmpty && (
                  <span className="ml-1 text-gray-600">
                    {fileInfo.sizeBytes > 1024
                      ? `${(fileInfo.sizeBytes / 1024).toFixed(1)} KB`
                      : `${fileInfo.sizeBytes} B`}
                  </span>
                )}
              </span>
            )}
          </div>

          {/* View controls */}
          <div className="flex items-center gap-2 shrink-0">
            {isEditable && !editing && (
              <div className="flex rounded border border-gray-700 overflow-hidden text-xs">
                <button
                  onClick={() => setViewMode("preview")}
                  className={`flex items-center gap-1 px-2.5 py-1 transition-colors ${
                    viewMode === "preview"
                      ? "bg-gray-700 text-gray-100"
                      : "bg-gray-900 text-gray-500 hover:text-gray-300"
                  }`}
                >
                  <Eye size={11} /> Preview
                </button>
                <button
                  onClick={() => setViewMode("code")}
                  className={`flex items-center gap-1 px-2.5 py-1 border-l border-gray-700 transition-colors ${
                    viewMode === "code"
                      ? "bg-gray-700 text-gray-100"
                      : "bg-gray-900 text-gray-500 hover:text-gray-300"
                  }`}
                >
                  <Code2 size={11} /> Code
                </button>
              </div>
            )}
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

        {/* File tab strip */}
        {sortedFileNames.length > 0 && (
          <div className="flex gap-1 px-3 pt-2 border-b border-gray-800 overflow-x-auto scrollbar-none shrink-0">
            {sortedFileNames.map((fname) => {
              const info = displayFiles[fname];
              return (
                <button
                  key={fname}
                  onClick={() => setActiveFile(fname)}
                  className={`shrink-0 px-3 py-1.5 text-xs rounded-t-lg transition-colors flex items-center gap-1.5 ${
                    activeFile === fname
                      ? "bg-gray-800 text-white"
                      : "text-gray-500 hover:text-gray-300"
                  } ${!info?.exists ? "opacity-40" : ""}`}
                >
                  <FileText size={11} />
                  {fname}
                  {info?.isEmpty && info?.exists && (
                    <span className="w-1.5 h-1.5 rounded-full bg-red-500" />
                  )}
                </button>
              );
            })}
          </div>
        )}

        {/* Content area */}
        {!activeFile && sortedFileNames.length === 0 && displayDirs.length === 0 && !dirLoading ? (
          <div className="flex-1 flex items-center justify-center text-gray-600 text-sm">
            No files found
          </div>
        ) : !activeFile ? (
          <div className="flex-1 flex items-center justify-center text-gray-600 text-sm">
            Select a file to view
          </div>
        ) : fileLoading ? (
          <div className="flex-1 flex items-center justify-center text-gray-600 text-sm">
            Loading…
          </div>
        ) : editing ? (
          <textarea
            value={editContent}
            onChange={(e) => setEditContent(e.target.value)}
            className="flex-1 w-full p-4 bg-gray-950 text-gray-200 font-mono text-sm resize-none focus:outline-none"
            spellCheck={false}
          />
        ) : viewMode === "preview" && previewHtml ? (
          <div
            className="md-prose flex-1 p-5 overflow-auto text-sm"
            dangerouslySetInnerHTML={{ __html: previewHtml }}
          />
        ) : (
          <pre className="flex-1 p-4 text-sm text-gray-300 font-mono whitespace-pre-wrap overflow-auto">
            {fileContent}
          </pre>
        )}
      </div>
    </PageLayout>
  );
}
