# Changelog

All notable changes to ClawBorg will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2026-03-04

### Added
- CLI daemon subcommands: `clawborg start` (fork to background, write PID file), `clawborg stop` (SIGTERM + 5 s graceful wait), `clawborg log` (last 50 lines), `clawborg log -f` (follow mode)
- ASCII art banner with version and project URLs shown on startup (foreground and daemon mode)
- Animated startup sequence in foreground mode — per-step `▸ label... ✓` with real agent count and actual operation status (cache load, file watcher)
- Braille spinner animation for `clawborg start` daemon init
- Styled `clawborg stop` output: `▸ Stopping ClawBorg (PID: N)... ✓ stopped`
- Daily / Weekly / Monthly toggle on the Cost Trend chart
- Hover tooltip on Cost Trend chart bars (date, cost, session count)
- Dashboard alert banner now surfaces agent health warnings alongside cost and cron alerts
- Monthly cost aggregation view (groups daily data by calendar month)

### Changed
- Cost Trend chart renamed from "Daily Cost Trend" to "Cost Trend"
- Chart fills its card container — no dead whitespace when adjacent cards are taller
- Cost by Model: model names normalised case-insensitively (e.g. "Kimi-K2.5" and "kimi-k2.5" merge into one entry)
- Cost by Model / Cost by Agent: progress bars are proportional to the highest-cost entry rather than fixed width
- Cost by Model: `$0.00` / unknown model entries hidden
- Cost by Model / Agent: pluralisation fix — "1 turn" instead of "1 turns"
- Sidebar version label updated to v0.2.2
- All CLI output is TTY-aware: colours and animations are suppressed when stdout is piped or redirected
- Startup animation steps 3–4 ("Building session cache", "Starting file watcher") now reflect actual completion status, not optimistic pre-render

### Fixed
- Usage page daily / weekly / all-time cost figures now stay in sync with the selected view period
- Total token count in Usage summary now includes `cache_read` and `cache_write` tokens
- Cost Trend chart bars were rendering empty with real data — root cause: missing `h-full` on bar wrapper; fixed by replacing fixed `barWidth` with flex layout
- Haiku model pricing corrected: `(0.25, 1.25)` → `(0.80, 4.00)` per million tokens
- `calculate_cost` now accounts for `cache_write` tokens (billed at input rate) and `cache_read` tokens (billed at 10 % of input rate)
- Systemic "Load failed" on all pages after extended runtime — root cause: `reload_agent_sessions` was awaited inline inside the `notify_rx` receive loop; rapid writes filled the 512-event channel, causing `blocking_send` to stall the FSEvents/kqueue OS thread and silently kill the watcher; concurrent write-lock waiters then starved API read-lock requests. Fixed with a pending-set + 500 ms debounce interval that decouples event receipt from disk I/O and collapses N events for the same file into one reload.
- File watcher is now supervised: `start_watching` is restarted with exponential backoff (1 s → 60 s) if it exits for any reason (FSEvents restart, macOS sleep/wake, channel close)
- Watcher callback changed from `blocking_send` (blocks OS thread on full channel) to `try_send` (drops event, logs warning)
- `eprintln!` calls in `watcher.rs` and `cache.rs` replaced with `tracing::warn!` so errors are visible in both foreground and daemon log output

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
