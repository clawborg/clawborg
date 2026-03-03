# Changelog

All notable changes to ClawBorg will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-03-03

### Added
- Debug logging for serde parse errors in cron.rs — failures now print to stderr with file path and error detail instead of silently returning an empty list
- Configurable alert thresholds via `~/.clawborg/config.toml` (`[alerts]` section with `dailySpendThreshold` and `dailySpendWarning` fields)

### Fixed
- `sessions.json` parsing aligned with real OpenClaw format: flat `HashMap` keyed by session key, camelCase fields (`sessionId`, `updatedAt`, `inputTokens`, etc.), no cost field — cost is now calculated from tokens using a per-model pricing table
- `cron/jobs.json` parsing with polymorphic schedule object: `kind: "every"` (interval) and `kind: "cron"` (cron expression) variants now correctly deserialised via serde internally-tagged enum
- `CronSchedule` fields made optional (`every_ms`, `anchor_ms`, `expr`, `tz`) to handle real-world jobs that omit these fields without failing deserialization
- `lastDelivered` in `CronJobState` changed to `Option<serde_json::Value>` to accept both boolean (`true`) and timestamp (`u64`) values present in real data
- `is_overdue` no longer returns `true` for every job when `every_ms` is `None` — interval-unknown jobs are treated as `ok`, not overdue
- Frontend null guard on cron page: `lastRun` and `nextRun` are properly handled when `null`; shows "Never" instead of crashing on `.toString()`
- `CronRunInfo` TypeScript type aligned with actual API response (`durationMs` + `lastStatus` instead of non-existent `cost`/`tokens`)
- Sidebar is now `position: fixed` at all viewport sizes — no longer scrolls away with long page content (e.g. the Crons list)
- WebSocket ping/pong heartbeat: server sends a ping every 30 s to keep connections alive through proxies and load balancers; disconnected clients are now cleaned up immediately via `AbortHandle`
- Daily spend threshold moved from `openclaw.json` to `~/.clawborg/config.toml` — ClawBorg no longer writes non-standard fields into OpenClaw's config file
- Alert threshold logging reduced to once at startup ("Loaded config" or "No config found, using defaults") instead of per-request
- File watcher consolidated from per-agent path enumeration (30+ OS watches) to a single recursive watch on `openclaw_dir`, preventing inotify/kqueue limit exhaustion on large agent setups

## [0.2.0] - 2026-03-02

### Added
- **Cost & Token Dashboard** (`/usage`): Today/weekly/all-time cost, per-model breakdown with bars, per-agent cost comparison, daily trend sparkline chart, bloated session warnings (>500KB)
- **Cron Job Monitor** (`/crons`): Schedule display, last run info with cost/tokens, next run estimation, status detection (ok/overdue/disabled/unknown), ephemeral job indicator
- **Smart Alerts Banner**: Persistent top banner surfacing critical issues — high daily cost, bloated sessions, overdue crons, failed health checks. Dismissable per session. Auto-refreshes every 60s.
- **Embedded Frontend** (rust-embed): React build compiled into Rust binary. Single `clawborg` executable serves both API and UI. No Node.js runtime needed.
- **New API Endpoints**: `GET /api/usage` (cost/token aggregation from JSONL), `GET /api/crons` (cron config + status), `GET /api/alerts` (smart alerts from all data sources)
- JSONL cost parsing: Reads `usage.cost.total`, `usage.input_tokens`, `usage.output_tokens`, `usage.cache_read_input_tokens` from session transcripts
- Cron schedule parser with human-readable descriptions (e.g., "Every 30 minutes", "Daily at 06:00")
- Dev mode HTML fallback when no frontend is embedded (shows API links)

### Changed
- Dashboard KPI cards now show Today's Cost + Week Cost instead of raw token counts
- Dashboard Health KPI shows healthy/total ratio instead of workspace file count
- Sidebar navigation: added Usage and Crons entries
- Version bump to 0.2.0

### Fixed
- (Carries forward all v0.1.0 fixes: responsive layout, dynamic file loading, URL corrections)

## [0.1.0] - 2026-03-02

### Added
- Agent overview page with health status, file count, session count, task count
- Agent detail with dynamic file discovery (auto-detects all .md files, no hardcoded lists)
- Inline BRIEF/STATUS editor with auto-backup before overwrite
- Session list page across all agents with channel detection and status
- Workspace health audit with 7-point checks per agent
- Config viewer with automatic secret redaction
- Real-time file watching via WebSocket (notify + debounce)
- Responsive design: mobile (<640px), tablet (640-1023px), laptop (1024-1279px), desktop (1280px+)
- Consistent `PageLayout` component across all pages
- KPI summary cards on dashboard (agents, sessions, cost, health)
- CLI subcommands: `clawborg health`, `clawborg agents`, `clawborg version`
- OpenClaw standard structure support (AGENTS.md, SOUL.md, IDENTITY.md, etc.)
- Multi-agent config: agents.list[], agents.defaults, agent singular form, filesystem detection fallback
- Collapsible mobile sidebar with hamburger menu and overlay

### Security
- Path traversal prevention on file read/write endpoints
- Only .md files writable from dashboard
- API key / token / password redaction in config viewer
- No hardcoded secrets — all via environment variables
