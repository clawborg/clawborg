# ClawBorg — Project Brief

## What
A local dashboard for OpenClaw AI agent fleets. Reads `~/.openclaw/` filesystem directly. No database, no cloud.

## Why
OpenClaw operators have zero visibility into their agent fleet health. Current workflow: SSH + cat files + grep sessions. ClawBorg replaces that with a single binary dashboard.

## Stack
- **Backend:** Rust (Axum) — single binary, <10MB RAM, <100ms startup
- **Frontend:** React (Vite) + Tailwind CSS + shadcn/ui
- **Data:** Filesystem (`~/.openclaw/`) — no database
- **Real-time:** notify crate → WebSocket → React

## MVP Scope (v0.1)
1. Agent overview dashboard
2. Agent detail + file viewer
3. Inline BRIEF/STATUS editor
4. Real-time file watching
5. Session monitoring
6. Workspace health audit
7. Config viewer (redacted)

## Non-Goals (v0.1)
- Authentication
- Database
- Cloud deployment
- Multi-user support
- Cost analytics (v0.2)
- Cron job monitoring (v0.2)

## Distribution
- `cargo install clawborg`
- GitHub Releases (binary)
- `brew install` (planned)

## License
AGPL-3.0 (open core model)
