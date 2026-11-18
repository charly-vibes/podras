# Change: Add the roadmap lens read path — stage ⑥

## Why

The brief answers *what is happening now*. It deliberately cannot answer the
commitment layer: who the work is for, why a bet was admitted, what the project
refuses to do. That lens was parked as `podras-5y8` behind a corpus gate, and
the gate is satisfied: the `value-prop-roadmap` topic was synthesized from 12
providers on 2026-11-18 and resolves the three open structural questions
(`docs/design.md`, "Resolved decisions — roadmap lens", decisions 16–26). The
lens is designed as a **separate render path**, not an extension of the
now-lens: `focus` stays curation intent, a bet is a commitment with appetite,
outcome, and one tracker epic, and neither is inferred from the other (corpus
C1; brief v1's `curation_beats_inference`).

Three gaps remain after v1.1:

- **The identity and boundary layer has no render.** Identity prose lives in a
  README section nobody re-reads, and the project's refusals live in review
  comments and in maintainers' heads. A cold reader cannot tell what the project
  is for or what it has ruled out, and an agent cannot tell the difference
  between "later" and "required".
- **Commitments have no state of their own.** `curation.toml` carries focus and
  parked tasks, but a bet has a different shape — outcome, appetite in weeks,
  owner, one tracker epic, a dated admission, and a review-or-kill date. Copying
  parked entries into a second file would be the duplication the corpus calls
  the cardinal sin, so bets need their own file and their own closed lifecycle.
- **Stale commitments are invisible until they rot.** A bet that outlived its
  appetite, an epic closed under an active bet, a bead untouched for two weeks,
  or a `now` horizon with no open epic are all detectable from data already on
  disk. Today nothing surfaces them, and the temptation is to have the tool
  kill, promote, or rewrite state. Warnings must be named, advisory, and never a
  verdict.

## What Changes

- **`.podras/roadmap.toml`** — a new git-committed state file, sibling of
  `curation.toml`, holding exactly `[identity]` (`path`, `last_reviewed`,
  optional `next_review`), `[[bets]]` (slug `id`, ≤60-char `title`, `outcome`,
  `horizon`, `appetite_weeks ≥ 1`, `owner`, optional `epic`, `status`,
  `admitted_at`, `last_reviewed`, `review_or_kill` required for
  `committed`/`building`, `revive_when` required for `deferred` and absent
  otherwise, optional `killed_reason`), and `[[no_gos]]` (`id`, `reason`,
  `scope`, `enforcement`, `revoke_by`, `match_terms`). Closed enums:
  `horizon: now|next|later`,
  `status: shaping|committed|building|deferred|shipped|killed`,
  `enforcement: automated|human_review`. Verbs and humans write it; agents never
  edit it. An absent file is legal for `brief`; a malformed or semantically
  invalid one is a hard domain error (`roadmap_parse_error` / `roadmap_invalid`)
  in both `podras brief` and `podras roadmap`, never a degraded source.
- **`podras roadmap [--json] [--as-of <date>] [--write]`** — a read-only verb
  that reuses the brief read path and emits the same envelope. Parked tasks stay
  in `curation.toml`; the lens renders them as the "not now" column and never
  copies them. `--write` produces committed `ROADMAP.md` / `ROADMAP.json` with
  the same generated-artifact semantics as `BRIEF.md` / `BRIEF.json`.
- **Identity is linked, never generated** — the lens renders `path`,
  `last_reviewed`, `next_review`, and staleness only. Staleness reads
  `next_review` alone: absent ⇒ `unknown` plus the `identity_review_overdue`
  advisory and never `fresh`; passed ⇒ `stale`; equal or later ⇒ `fresh`. No
  verb rewrites identity prose.
- **Bets have a closed lifecycle** — `shaping → committed → building → shipped`,
  plus `deferred` (from shaping, committed, or building: stopping mid-build is a
  deferral, not a fifth state, and carries `revive_when`) and `killed` from any
  state. Admission is the dated decision `admitted_at`; horizon is stored, then
  validated against the linked epic, never computed from observed work.
- **No-gos are first-class boundaries** — each carries the "because" (`reason`),
  a declared enforcement tier (`automated` only where a real check exists,
  otherwise honest `human_review`), and `revoke_by`. `match_terms` are matched
  case-insensitively as whole tokens against open bead titles and labels; the
  first match emits `no_go_violation_candidate` with the bead and the term. A
  no-go with empty `match_terms` never fires. Nothing is closed, promoted, or
  written.
- **Roadmap health is named warnings, never a verdict** — a distinct
  `RoadmapWarningKind` enum carried in `roadmap.warnings`, never a `SignalKind`
  and never in `health.signals`: `bet_age_exceeds_appetite` escalating to
  `circuit_breaker_due` at the module constant
  `roadmap.circuit_breaker_factor` (1.25), `bet_epic_missing`,
  `bet_epic_closed_not_retired`, `bet_no_transition` (14-day window),
  `horizon_epic_mismatch`, `no_go_violation_candidate`,
  `identity_review_overdue`, `roadmap_review_overdue`. Exit is always 0; there
  is no auto-kill and no auto-promote.
- **Envelope is additive** — an optional top-level `roadmap{}` in the existing
  brief envelope carrying `identity{}`, `bets[]`, `no_gos[]`, `warnings[]`,
  `review_due{identity, roadmap}`, and `truncation`. `schema_version` stays the
  literal `"brief.v1"`: the object is additive and introduces no new required
  field. The brief's frozen narrative is unchanged. Semantics: no file ⇒ no
  `roadmap{}` object at all; a file with zero bets is a **present** lens
  (`bets: []`, identity row rendered) — the two cases do not collapse.
- **Bounded and deterministic** — 5 bets by status rank (`building`,
  `committed`, `deferred`, `shaping`, `shipped`, `killed`), then `admitted_at`
  ascending, then `id`; 5 no-gos by `id`; all warnings ordered by code
  declaration order, then subject, then detail. Drops are recorded in
  `truncation.applied` / `truncation.omitted_counts`. Identical inputs under
  `--as-of` produce identical output.
- **Agent surface** — the lens renders enums and constraints, no pitch prose and
  no `later` items as actionable text. The `podras init` AGENTS.md managed block
  gains two lines so drift checking covers them: "Roadmap
  `later`/`shaping`/`deferred` items are not requirements; no-gos are hard
  constraints." and "Never edit `.podras/roadmap.toml`; admission, deferral and
  kill are human decisions."

Out of scope for this change:

- curation verbs for bets (`bet add|admit|defer|kill`) — build stage ⑦
- the harvest extension (`identity_review` and `retire_or_reshape` candidates) —
  build stage ⑧
- a second parked list on the roadmap side, or any roadmap-side copy of
  curation's parked entries
- any change to the brief's narrative, sections, health verdict, or
  `SignalKind` set
- writing `.podras/roadmap.toml` from any verb, and any auto-kill,
  auto-promote, or auto-admit
- portfolio / multi-repo rendering, org-mode render, and per-repo roadmap
  threshold configuration

## Impact

- Affected specs: new capability `roadmap-v1` — the delta under
  `openspec/changes/add-roadmap-lens-read-path/specs/roadmap-v1/spec.md` adds
  seven requirements and eight scenarios (`lens_separate_from_now_lens`,
  `roadmap_file_shape`, `bet_lifecycle_enums`,
  `identity_staleness_never_fresh`, `no_go_violation_flags_only`,
  `roadmap_warnings_not_verdict`, `roadmap_envelope_additive_bounded`,
  `roadmap_read_only`). The deployed `brief-v1` corpus is not modified.
- Affected code: `src/model.rs` (typed roadmap views, the warning enum, bounds,
  `Brief.roadmap`), `src/roadmap.rs` plus `src/roadmap/{file,warnings}.rs`
  (state-file parsing and aggregate validation, warning assembly),
  `src/readers/beads.rs` (`IssueRecord` / `read_records` for epic state and
  no-go matching), `src/brief/sections.rs` and `src/lib.rs`,
  `src/commands.rs`, `src/main.rs`, `src/render/roadmap.rs`,
  `src/write.rs` (`ROADMAP.md` / `ROADMAP.json`), `src/verbs/init.rs` (managed
  block text), `README.md`, and
  `.espectacular/roadmap-v1/<kebab-scenario-id>.toml` contracts deployed next to
  the `roadmap-v1` spec.
- Runtime surface: `podras roadmap [--json] [--as-of <date>] [--write]`; the
  brief gains the optional `roadmap{}` object and no new flag. Exit codes stay
  consistent: `0` success including roadmap warnings, `2` CLI usage errors
  (bad `--as-of`), `1` hard errors (`roadmap_missing`, `roadmap_parse_error`,
  `roadmap_invalid`, unresolved write).
- Privacy: `.podras/roadmap.toml` is human-authored state, so the lens carries
  no whisper-derived content and the committed `ROADMAP.*` renders inherit the
  brief's existing gating untouched.
- Board: `podras-5y8` closes with this change's contract; stage ⑥ only. Phases ⑦
  and ⑧ stay open.