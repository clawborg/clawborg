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

4. **Cost tracking from JSONL.** Session files contain `usage.cost.total`, `input_tokens`, `output_tokens` fields. Parser aggregates by model, agent, and date.

5. **Cron detection.** Reads `crons` array from `openclaw.json`, matches to session files with `:cron:` in their name, calculates overdue status.

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
- Cost $0.00 on real OpenClaw if JSONL format differs from expected fields
- Cron jobs not detected if config uses different structure than `crons` array
- Agent detail page can hang on agents with many sessions (needs pagination)
- Folders in workspace browser are not clickable (not recursive yet)
- Session list has no pagination or filtering

## Current Version

v0.2.0. See CHANGELOG.md and VERSIONING.md for history and roadmap.
