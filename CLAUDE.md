# CLAUDE.md

This file provides guidance to Claude Code when working on the ClawBorg codebase.

## Project Overview

ClawBorg is a read-only dashboard for OpenClaw AI agent fleets. It reads `~/.openclaw/` directory and surfaces agent health, cost/token usage, cron job status, and alerts through a local web dashboard.

Single binary distribution: Rust backend serves an embedded React frontend. No runtime dependencies for end users.

## Architecture

```
clawborg/
├── crates/clawborg/src/     # Rust backend (Axum)
│   ├── main.rs              # CLI entry point (clap)
│   ├── server.rs            # Axum server + embedded frontend (rust-embed)
│   ├── openclaw/            # Core logic
│   │   ├── config.rs        # Parse openclaw.json
│   │   ├── workspace.rs     # Read agent workspaces
│   │   ├── sessions.rs      # Parse JSONL session files
│   │   ├── health.rs        # Health audit checks
│   │   ├── usage.rs         # Cost/token aggregation from JSONL
│   │   ├── cron.rs          # Cron job status detection
│   │   └── alerts.rs        # Alert generation (cost, cron, health)
│   ├── routes/              # API route handlers
│   ├── watcher.rs           # Filesystem watcher (notify crate)
│   └── ws.rs                # WebSocket for real-time events
├── web/                     # React frontend (Vite + Tailwind + shadcn/ui)
│   ├── src/
│   │   ├── api/client.ts    # API client functions
│   │   ├── lib/types.ts     # TypeScript type definitions
│   │   ├── pages/           # Page components
│   │   ├── hooks/           # Custom React hooks
│   │   └── App.tsx          # Router + layout + alerts banner
│   ├── index.html
│   └── vite.config.ts
└── fixtures/mock-openclaw/  # Mock data for development/testing
```

## Tech Stack

- Backend: Rust, Axum, serde, tokio, rust-embed
- Frontend: React 19, TypeScript, Vite, Tailwind CSS, shadcn/ui, Recharts
- Build: cargo (Rust), pnpm (frontend)
- License: AGPL-3.0

## Development Commands

```bash
# Full build (production)
cd web && pnpm install && pnpm build && cd ..
cargo build --release
./target/release/clawborg --dir ./fixtures/mock-openclaw

# Development mode (hot reload)
# Terminal 1:
cargo run -- --dir ./fixtures/mock-openclaw
# Terminal 2:
cd web && pnpm dev

# Type check frontend
cd web && pnpm tsc --noEmit

# Check Rust
cargo check
cargo clippy
```

## Ports

- 3103: Vite dev server (frontend development only)
- 3104: Axum server (API + embedded frontend in production)

## Key Design Decisions

1. **Read-only by default.** ClawBorg observes OpenClaw, it does not modify it. Write operations (md file editing) are opt-in and create .bak backups.

2. **Filesystem-first.** All data comes from reading `~/.openclaw/` directly. No database, no external services.

3. **Single binary.** React frontend is compiled into the Rust binary via rust-embed. End users run one file.

4. **Cost tracking from sessions.json.** OpenClaw writes per-agent cost/token summaries to `~/.openclaw/agents/<id>/sessions.json` (not individual JSONL files). The current `usage.rs` parser targets JSONL fields (`usage.cost.total`, `input_tokens`, `output_tokens`) which do not match the actual format — this is the root cause of $0.00 cost on real installs.

5. **Cron detection from cron/jobs.json.** Cron job definitions are stored at `~/.openclaw/cron/jobs.json`, not in a `crons` array inside `openclaw.json`. The current `cron.rs` reads the wrong file — this is the root cause of cron jobs never being detected on real installs.

## Code Style

- Rust: follow standard rustfmt, use `thiserror` for error types
- TypeScript: strict mode, no `any` types, functional components with hooks
- Commits: conventional commits (feat:, fix:, chore:, docs:)
- Branches: `feat/`, `fix/`, `chore/` prefixes from main
- Never push directly to main, always use PR

## Testing with Fixtures

`fixtures/mock-openclaw/` contains a complete mock OpenClaw setup with 3 agents (main, coder, writer), JSONL sessions with cost data, and cron config. Always test changes against fixtures before pushing.

## Known Issues (v0.2.0)

- Daily cost trend shows "No daily data yet" due to UTC timezone alignment
- **Cost always $0.00 on real OpenClaw** — root cause: `usage.rs` parses JSONL files looking for `usage.cost.total` / `input_tokens` / `output_tokens`, but actual cost data lives in `~/.openclaw/agents/<id>/sessions.json`. Needs full rewrite to read `sessions.json`.
- **Cron jobs never detected on real OpenClaw** — root cause: `cron.rs` reads a `crons` array from `openclaw.json`, but cron definitions are actually stored at `~/.openclaw/cron/jobs.json`. Needs to read the correct file.
- Agent detail page can hang on agents with many sessions (needs pagination)
- Folders in workspace browser are not clickable (not recursive yet)
- Session list has no pagination or filtering

## Current Version

v0.2.0. See CHANGELOG.md and VERSIONING.md for history and roadmap.
