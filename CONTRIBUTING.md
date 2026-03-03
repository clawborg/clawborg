# Contributing to ClawBorg

Thanks for your interest in contributing to ClawBorg! This guide will help you get started.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone git@github.com:YOUR_USERNAME/clawborg.git`
3. Run the setup script: `bash scripts/setup.sh`
4. Create a branch: `git checkout -b fix/your-change`

## Prerequisites

- Rust (install via https://rustup.rs)
- Node.js 18+ and pnpm (install via `npm i -g pnpm`)
- Git

## Development Workflow

```bash
# Start the API server with mock data
cargo run -- --dir ./fixtures/mock-openclaw

# In another terminal, start the frontend dev server
cd web && pnpm dev

# Open http://localhost:3103 (Vite proxies API to 3104)
```

## Making Changes

### Backend (Rust)

Source code is in `crates/clawborg/src/`. After changes:

```bash
cargo check          # fast type check
cargo clippy         # lint
cargo run -- --dir ./fixtures/mock-openclaw   # test
```

### Frontend (React)

Source code is in `web/src/`. The Vite dev server auto-reloads on save. Before committing:

```bash
cd web
pnpm tsc --noEmit    # type check
```

### Both

When changing both backend and frontend, rebuild the full binary to verify the embedded frontend works:

```bash
cd web && pnpm build && cd ..
cargo build --release
./target/release/clawborg --dir ./fixtures/mock-openclaw
```

## Commit Messages

We use conventional commits:

```
feat: add session pagination
fix: daily cost trend timezone alignment
chore: update dependencies
docs: improve quickstart guide
```

## Pull Requests

1. Make sure your branch is up to date with main
2. One logical change per PR
3. Include a clear description of what changed and why
4. PRs are reviewed by Greptile (automated) and a maintainer
5. Squash merge into main

## Fixtures

`fixtures/mock-openclaw/` contains mock data. If your change needs new fixture data, add it there and document what you added.

## Code Review

All PRs go through automated review via Greptile plus manual review. Common things we look for:

- No panics in production code paths (use proper error handling)
- TypeScript strict mode compliance (no `any` types)
- Changes work with both mock fixtures and real OpenClaw installations
- Frontend changes are responsive

## License

By contributing, you agree that your contributions will be licensed under the AGPL-3.0 license.

## Questions?

Open an issue or reach out on Twitter: [@clawborgdev](https://x.com/clawborgdev)
