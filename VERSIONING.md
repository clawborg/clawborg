# Versioning and Release

ClawBorg follows [Semantic Versioning 2.0](https://semver.org/): `MAJOR.MINOR.PATCH`

## Version Bump Rules

### PATCH (0.1.0 -> 0.1.1)

No new features. No breaking changes. Safe to update blindly.

Examples: bug fix, typo in UI, dependency update, CSS tweak, docs correction.

### MINOR (0.1.0 -> 0.2.0)

New feature or meaningful improvement. Existing config and API still work.

Examples: add cost dashboard, add cron monitor, new page in UI, new API endpoint.

### MAJOR (0.x -> 1.0.0)

Breaking change to config format, API endpoints, or CLI flags. Alternatively, the "this is stable" declaration.

Examples: rename API routes, change `openclaw.json` parsing, restructure CLI args.

### Pre-1.0 Note

While ClawBorg is `0.x`, minor bumps may include small breaking changes. This is normal for pre-release software. Once `1.0.0` ships, the contract is strict.

## Roadmap Mapping

| Version | Milestone |
|---------|-----------|
| 0.1.x   | MVP: agents, files, sessions, health, config |
| 0.2.0   | Cost dashboard, cron monitor, smart alerts, embedded frontend (SHIPPED) |
| 0.3.0   | Skills viewer, sub-agent tracker, theme toggle, export health report |
| 0.4.0   | Gateway WebSocket integration (live status from `openclaw health --json`) |
| 1.0.0   | Stable API surface, CLI interface, and config format |

## Release Checklist

```bash
# 1. Make sure everything builds clean
cargo clippy --all-targets
cargo test
cd web && pnpm lint && pnpm build && cd ..

# 2. Bump version in all three places
#    - Cargo.toml (workspace.package.version)
#    - web/package.json (version)
#    - README.md status line

# 3. Update CHANGELOG.md
#    - Move [Unreleased] items into new version section
#    - Add date
#    - Add comparison link at bottom

# 4. Commit and tag
git add -A
git commit -m "release: v0.1.1"
git tag v0.1.1
git push origin main --tags

# 5. Create GitHub Release
#    - Title: v0.1.1
#    - Body: copy from CHANGELOG.md
#    - Attach binary if applicable
```

## Version Locations

These files contain the version string and must stay in sync:

| File | Field |
|------|-------|
| `Cargo.toml` | `workspace.package.version` |
| `web/package.json` | `version` |
| `README.md` | Status line |
| `CHANGELOG.md` | Latest `## [x.y.z]` header |

## Commit Convention

Prefix commits so changelogs are easy to generate:

| Prefix | Meaning | Bump |
|--------|---------|------|
| `feat:` | New feature | MINOR |
| `fix:` | Bug fix | PATCH |
| `docs:` | Documentation only | PATCH |
| `style:` | UI/CSS, no logic change | PATCH |
| `refactor:` | Code change, no feature/fix | PATCH |
| `perf:` | Performance improvement | PATCH |
| `test:` | Add/fix tests | PATCH |
| `chore:` | Build, deps, tooling | PATCH |
| `security:` | Security fix | PATCH |
| `release:` | Version bump commit | - |
| `BREAKING:` | Breaking change (prefix any type) | MAJOR |

Example commits:
```
feat: add cost dashboard with per-model breakdown
fix: file watcher crash on symlinked workspaces
docs: update README with pnpm instructions
security: sanitize file paths in agent detail API
BREAKING: feat: rename /api/health to /api/audit
```
