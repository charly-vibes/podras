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

## Roadmap state

The lens reads one hand-written, git-committed file: `.podras/roadmap.toml`,
the sibling of `.podras/curation.toml`. It is a human decision record — write
it yourself, or through the `bet` verbs once stage ⑦ lands; **agents never
edit it**. A missing file is legal and means the `brief` carries no `roadmap`
object at all; a present file with zero bets is still a present lens.

`podras brief` degrades: a malformed or unreadable file omits the `roadmap`
object and adds exactly one curation warning instead of failing. `podras
roadmap` reports the same file as a hard error, which is what you want when
the lens itself is what you asked for.

Dates are ISO-8601 (`YYYY-MM-DD` or RFC 3339) and the enums are closed
(`horizon`: `now|next|later`; `status`:
`shaping|committed|building|deferred|shipped|killed`; `enforcement`:
`automated|human_review`). A complete file:

```toml
[identity]
path = "README.md#roadmap-state"
last_reviewed = "2026-11-18"
next_review = "2027-05-18"

[[bets]]
id = "roadmap-lens"
title = "Ship the roadmap lens"
outcome = "A cold reader predicts the next bet from the lens alone"
horizon = "now"
appetite_weeks = 6
owner = "charly"
epic = "podras-abc"
status = "building"
admitted_at = "2026-11-18"
last_reviewed = "2026-11-18"
review_or_kill = "2026-12-30"

[[no_gos]]
id = "no-tracker-writes"
reason = "podras never mutates the tracker; harvest proposes only"
scope = "all verbs"
enforcement = "automated"
revoke_by = "never — requires a new design decision"
match_terms = ["auto-close", "auto-promote"]
```

`[identity]` needs `path` and `last_reviewed`; `next_review` is optional and
absent means the review is unscheduled. Each `[[bets]]` entry needs `id`,
`title` (at most 60 characters), `outcome`, `horizon`, `appetite_weeks`
(≥ 1), `owner`, `status`, `admitted_at`, and `last_reviewed`; `epic` is
optional, `review_or_kill` is required for `committed` and `building`, and
`revive_when` is required for `deferred`. Each `[[no_gos]]` entry needs `id`,
`reason`, `scope`, `enforcement`, and `revoke_by`, with optional `match_terms`
matched as whole tokens against open bead titles and labels.

`podras roadmap --write` emits two artifacts with different scopes:

- `ROADMAP.md` — the lens narrative only (identity, bets, no-gos, and the
  lens's own warnings).
- `ROADMAP.json` — the full brief envelope with the `roadmap` object attached,
  so an agent reading it also gets `sources[]`, `health`, `truncation`, and
  `source_hash`.

## Naming

Named with a nod to *No podrás* (Cristian Castro) — "tal vez te olvides de mí…" — forgetting is fine; coming back cold should still tell you what's happening. Deterministic, per-repo, a lens you point at a project. (Resembles *podrá* — "it will be able to"; the tool tells you what the project can do next.)
