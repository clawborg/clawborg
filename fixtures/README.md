# ClawBorg Mock Fixtures

Mock data for developing and testing ClawBorg without a live OpenClaw installation.

## Usage

```bash
cargo run -- --dir ./fixtures/mock-openclaw
```

## Structure

Follows the **standard OpenClaw directory layout** documented at
[docs.openclaw.ai](https://docs.openclaw.ai/concepts/agent-workspace):

```
mock-openclaw/
├── .env.example                              # Env vars (secrets stay here, NOT in config)
├── openclaw.json                             # Gateway config (no secrets, uses ${ENV_VAR})
│
├── workspace/                                # Default agent workspace (id: "main")
│   ├── AGENTS.md                             # Operating instructions
│   ├── SOUL.md                               # Persona, boundaries, tone
│   ├── IDENTITY.md                           # Name, vibe, emoji
│   ├── USER.md                               # User profile
│   ├── TOOLS.md                              # Tool notes and conventions
│   ├── HEARTBEAT.md                          # Heartbeat checklist
│   ├── MEMORY.md                             # Curated long-term memory
│   ├── memory/                               # Daily logs (append-only)
│   │   ├── 2026-02-28.md
│   │   └── 2026-03-01.md
│   └── skills/                               # Workspace-scoped skills
│       └── web-search/SKILL.md
│
├── workspace-coder/                          # Multi-agent workspace (id: "coder")
│   ├── AGENTS.md, SOUL.md, IDENTITY.md, ...
│   ├── memory/
│   └── tasks/                                # Optional: file-based task queue
│       ├── pending/task-015.md               # Stale (>48h, triggers warning)
│       └── done/task-012.md
│
├── workspace-writer/                         # Multi-agent workspace (id: "writer")
│   ├── AGENTS.md                             # Nearly empty (<50 bytes, triggers critical)
│   ├── SOUL.md, IDENTITY.md
│   └── (no TOOLS.md, no MEMORY.md)
│
└── agents/                                   # Session state (standard OpenClaw path)
    ├── main/sessions/
    │   ├── agent:main:telegram:dm:123456789.jsonl
    │   └── agent:main:cron:heartbeat.jsonl
    ├── coder/sessions/
    │   └── agent:coder:cli:local.jsonl
    └── writer/sessions/
        └── agent:writer:telegram:dm:123456789.jsonl
```

## Health states

| Agent | Status | Why |
|-------|--------|-----|
| main | 🟢 Healthy | All standard files present and populated |
| coder | 🟡 Warning | Stale pending task (>48h old) |
| writer | 🔴 Critical | AGENTS.md nearly empty (<50 bytes) |

## Security

- **No secrets in config.** `openclaw.json` uses `${ENV_VAR}` references for tokens.
- **No real API keys anywhere.** `.env.example` shows the expected variables.
- ClawBorg itself never needs API keys — it reads the filesystem only.

## What this tests

- [x] Standard workspace layout (AGENTS.md, SOUL.md, IDENTITY.md, USER.md, TOOLS.md, HEARTBEAT.md)
- [x] Standard workspace paths (`workspace/` and `workspace-<id>/`)
- [x] Standard JSONL session files (`agents/<id>/sessions/<key>.jsonl`)
- [x] Auto-discovery of .md files (no hardcoded list)
- [x] Health checks: healthy, warning, critical
- [x] Optional directories (memory/, skills/, tasks/)
- [x] Config with `${ENV_VAR}` references (no inline secrets)
- [x] Agent identity resolution (config identity > IDENTITY.md)
- [x] Session key parsing (`agent:<id>:<channel>:<type>:<peer>`)
