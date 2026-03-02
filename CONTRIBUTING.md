# Contributing to ClawBorg

Thanks for your interest in contributing! Here's how to get started.

## Development Setup

### Prerequisites
- Rust 1.75+ (`rustup install stable`)
- Node.js 20+ and pnpm (`npm i -g pnpm`)
- An OpenClaw installation at `~/.openclaw/`, or use the mock fixtures

### Build & Run

```bash
# Clone
git clone https://github.com/clawborg/clawborg
cd clawborg

# Backend (Rust)
cargo run -- --dir ./fixtures/mock-openclaw

# Frontend (React) -- in a separate terminal
cd web
pnpm install
pnpm dev   # Vite dev server on :3103, proxies API to :3104
```

### Project Structure

```
crates/clawborg/src/     # Rust backend
  main.rs                # CLI entry point
  server.rs              # Axum server setup
  routes/                # API route handlers
  openclaw/              # OpenClaw filesystem readers
  watcher.rs             # File system watcher (notify)
  ws.rs                  # WebSocket handler

web/src/                 # React frontend
  pages/                 # Page components
  components/            # Shared UI components (PageLayout, etc.)
  hooks/                 # Custom React hooks
  api/                   # API client + WebSocket
```

## Guidelines

1. **Filesystem-first** -- No database. Read/write `~/.openclaw/` directly.
2. **Standard OpenClaw** -- Follow official structure from docs.openclaw.ai, not custom patterns.
3. **Write safety** -- Only markdown files are writable. Always validate paths.
4. **Token redaction** -- Never expose API keys or tokens through the API.
5. **Responsive** -- All UI changes must work across mobile, tablet, laptop, desktop.

## Commits

Use conventional commit prefixes. See [VERSIONING.md](./VERSIONING.md) for the full table.

```
feat: add cost dashboard
fix: file watcher crash on symlinks
docs: update setup instructions
```

## Pull Requests

- Fork, branch from `main`, PR back to `main`
- Run `cargo clippy` and `cargo fmt` before submitting
- Run `cd web && pnpm lint && pnpm build` for frontend checks
- Include tests for new features
- Update CHANGELOG.md under `[Unreleased]`

## Releases

See [VERSIONING.md](./VERSIONING.md) for version bump rules and release checklist.

## License

By contributing, you agree that your contributions will be licensed under AGPL-3.0.
