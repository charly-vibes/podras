# podras

**podras** — *project status, distilled* — is a deterministic, per-repo CLI that answers "what's happening in this project?" for humans coming back cold and for AI coding agents, from one git-committed source.

## The 30-second pitch

Coming back to a project means 15 minutes of archaeology across the tracker, specs, git log, and session notes. Asking an AI agent for status means distrusting whatever it invents. podras collapses both problems: it **derives** a status brief from evidence that already exists (beads tracker, openspec changes, git history, whisper session notes) and adds the explicit human intent you recorded (`focus` / `park`). The result is:

- **`BRIEF.md`** — a fixed-section narrative for humans: purpose, focus, in-flight, blocked, next, parked, recent, health.
- **`BRIEF.json`** — a typed JSON envelope for agents: per-source freshness, stable IDs, explicit unknowns, deterministic bounds. A contract, not a prompt.

Design principle: **project status is a projection of evidence plus explicit human intent — never a document people maintain independently.** Beads stays authoritative for tasks; curation stays authoritative for intent; the brief is a trustworthy view, not another source of truth.

## Who it's for

| Who | Problem today | What podras gives them |
|---|---|---|
| Human returning to a project | 15 min of archaeology across tracker, specs, git log, session notes | One brief: current focus, next action, blockers, health — with freshness stamps |
| Person showing the project to someone else | Ad-hoc screenshots/prose | A generated, shareable brief artifact |
| AI coding agent | Distrusts or invents state; stale AGENTS.md conventions | A typed JSON envelope with per-source freshness, stable IDs, explicit unknowns |

## Install

podras is pre-release: install from source with `cargo install` until the first GitHub Release ships prebuilt binaries. See [Status](status.md).

```
cargo install podras
```

## Learn more

- [Getting started](getting-started.md) — a full `init → brief → focus/park → roadmap` walkthrough
- [CLI reference](cli.md) — every command, every flag
- [Design Specs](specs/brief-v1.md) — the normative v1 contract
