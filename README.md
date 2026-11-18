# podras

> "Tal vez te olvides de mí, tal vez me olvide de ti…" — from *No podrás*, Cristian Castro

**Project status, distilled** — a deterministic CLI that answers "what's happening in this project?" for humans coming back cold and for AI coding agents, from one git-committed source.

## Objective

Build a small, deterministic, per-repo CLI that:

- **brief** — renders a fixed-section project status (what's happening, what's next, what's blocked, what's parked) for a human and a machine-readable envelope for agents
- **focus / park** — records explicit intent (what we're committed to now, what we're deliberately not doing)
- **roadmap** — renders the separate why-lens (identity, bets, no-gos) read-only from `.podras/roadmap.toml`; `--write` emits `ROADMAP.md` and `ROADMAP.json`
- **harvest** — derives status from sources that already exist: beads tracker, openspec changes, git history, whisper session notes

Per-repo, git-committed state (`.beads`-style): clone a repo and its orientation context is there. Portfolio view (many repos) is computed on the fly, never stored.

## Value proposition

| Who | Problem today | What podras gives them |
|---|---|---|
| Human returning to a project | 15 min of archaeology across tracker, specs, git log, session notes | One brief: current focus, next action, blockers, health — with freshness stamps |
| Person showing the project to someone else | Ad-hoc screenshots/prose | A generated, shareable brief artifact |
| AI coding agent (pi, Claude Code, Codex) | Distrusts or invents state; stale AGENTS.md conventions | A typed JSON envelope with per-source freshness, stable IDs, explicit unknowns — a contract, not a prompt |

Design principle: **project status is a projection of evidence plus explicit human intent — never a document people maintain independently.** Beads stays authoritative for tasks; curation stays authoritative for intent; the brief is a trustworthy view, not another source of truth.

## Status

Implemented through v1.1 (brief read path, envelope, committed renders,
focus/park/init, harvest, freshness hardening, drift gate) and the roadmap
lens read path (stage ⑥: `podras roadmap`, `.podras/roadmap.toml`, optional
`roadmap` object in the envelope, `ROADMAP.md`/`ROADMAP.json` via `--write`).
Next stages: ⑦ roadmap curation verbs (`bet add|admit|defer|kill`) and ⑧ the
harvest extension. Design decisions are recorded in
[docs/design.md](docs/design.md); the lens change lives in
`openspec/changes/add-roadmap-lens-read-path/`.

## Naming

Named with a nod to *No podrás* (Cristian Castro) — "tal vez te olvides de mí…" — forgetting is fine; coming back cold should still tell you what's happening. Deterministic, per-repo, a lens you point at a project. (Resembles *podrá* — "it will be able to"; the tool tells you what the project can do next.)
