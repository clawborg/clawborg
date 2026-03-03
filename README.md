# <img src="assets/logo.png" width="32" height="32" alt="ClawBorg" /> ClawBorg

**Dashboard for OpenClaw AI agent fleets.**

ClawBorg gives you visibility into your [OpenClaw](https://openclaw.ai) installation — agents, workspaces, sessions, health, and config — from a single local dashboard.

> **Status:** v0.2.1 released. See [CHANGELOG.md](./CHANGELOG.md) for details.

<p align="center">
  <img src="assets/screenshot-dashboard.png" width="800" alt="Agent Fleet Dashboard" />
</p>

<details>
<summary>More screenshots</summary>

**Usage & Cost** — per-model and per-agent cost breakdown, token tracking
<img src="assets/screenshot-usage.png" width="800" alt="Usage & Cost Dashboard" />

**Cron Jobs** — schedule monitoring, overdue detection, run cost tracking
<img src="assets/screenshot-crons.png" width="800" alt="Cron Job Monitor" />

**Agent Detail** — workspace file viewer, inline editor, task queue status
<img src="assets/screenshot-agent-detail.png" width="800" alt="Agent Detail View" />

</details>

## Features

### v0.2 — The One You Ship
- **Cost & Token Dashboard** — today/weekly/all-time spend, per-model and per-agent breakdown, daily trend chart, bloated session warnings
- **Cron Job Monitor** — schedule viewer, last/next run, status detection (ok/overdue/disabled)
- **Smart Alerts Banner** — persistent banner surfacing high cost, overdue crons, health issues
- **Single Binary Distribution** — React frontend embedded in Rust binary via rust-embed. No Node.js needed.

### v0.1 — Foundation
- **Agent overview** — auto-discovers agents from `openclaw.json`
- **Workspace viewer** — browse all files in each agent's workspace
- **Inline editor** — edit `.md` files (AGENTS.md, SOUL.md, etc.) directly
- **Health audit** — detects missing files, empty instructions, stale tasks
- **Session monitor** — view active/idle/stale sessions with token counts
- **Config viewer** — redacted view of your OpenClaw config
- **Real-time updates** — WebSocket file watching for live changes
- **CLI health check** — `clawborg health` for quick terminal diagnostics

## How it works

ClawBorg is a **read-only observer** of your OpenClaw installation. It reads:

- `~/.openclaw/openclaw.json` — discovers agents and their workspace paths
- Agent workspaces — scans all `.md` files (auto-discovered, not hardcoded)
- `~/.openclaw/agents/*/sessions/*.jsonl` — session transcripts
- Optional: `tasks/` directories if your agents use file-based task queues

ClawBorg does **not** modify your OpenClaw config or interfere with the gateway. Write operations (editing `.md` files) are opt-in and create auto-backups.

## Quick start

```bash
# Install
cargo install clawborg

# Run (auto-detects ~/.openclaw/)
clawborg

# Or point to a specific directory
clawborg --dir /path/to/.openclaw

# CLI health check
clawborg health

# List discovered agents
clawborg agents
```

## Compatibility

ClawBorg supports any OpenClaw configuration:

| Setup | Config pattern | Supported |
|-------|---------------|-----------|
| Single agent | `agent.workspace` or `agents.defaults.workspace` | ✅ |
| Multi-agent | `agents.list[]` with per-agent workspaces | ✅ |
| Standard files | AGENTS.md, SOUL.md, IDENTITY.md, USER.md, TOOLS.md | ✅ |
| Custom files | Any `.md` files in workspace root | ✅ (auto-discovered) |
| JSONL sessions | `~/.openclaw/agents/<id>/sessions/*.jsonl` | ✅ |
| Task queues | `<workspace>/tasks/{pending,approved,done}/` | ✅ (optional) |
| Skills | `<workspace>/skills/` | ✅ |
| Memory | `memory/YYYY-MM-DD.md` + `MEMORY.md` | ✅ |

## Development

```bash
# Clone
git clone https://github.com/clawborg/clawborg.git
cd clawborg

# Run with mock fixtures (no OpenClaw install needed)
cargo run -- --dir ./fixtures/mock-openclaw

# Frontend dev
cd web && pnpm install && pnpm dev
```

## Architecture

```
clawborg
├── crates/clawborg/       # Rust backend (Axum)
│   └── src/
│       ├── main.rs         # CLI + server startup
│       ├── openclaw/       # Config parsing, workspace reader, sessions
│       ├── routes/         # REST API handlers
│       ├── server.rs       # Axum server setup
│       ├── watcher.rs      # Filesystem watcher (notify)
│       └── ws.rs           # WebSocket for real-time events
├── web/                    # React frontend (Vite + Tailwind)
└── fixtures/               # Mock data for development
```

## Configuration

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--dir` | `OPENCLAW_DIR` | `~/.openclaw` | OpenClaw directory path |
| `--port` | — | `3104` | Dashboard port |
| `--readonly` | — | `false` | Disable write operations |
| `--no-watch` | — | `false` | Disable file system watching |

## License

AGPL-3.0 — see [LICENSE](LICENSE).

## Links

- [OpenClaw](https://openclaw.ai) — the AI agent framework
- [OpenClaw Docs](https://docs.openclaw.ai) — official documentation
- [ClawBorg Issues](https://github.com/clawborg/clawborg/issues)
