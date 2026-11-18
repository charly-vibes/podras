# Design: roadmap lens read path — stage ⑥

## Context

`add-brief-v1-read-path` landed the deterministic now-lens, the typed agent
envelope, and the `focus` / `park` / `init` curation verbs; `add-brief-v1-1`
landed harvest, the declared freshness table, and the drift gate. The deployed
`brief-v1` corpus fixes the now-lens and must not move. The roadmap lens was
parked as `podras-5y8` behind an external gate; the `value-prop-roadmap`
synthesis (12 providers, 2026-11-18) satisfied it and `docs/design.md`
decisions 16–26 fix the lens's shape.

This change is the contract for **stage ⑥ only**: the read path. Curation verbs
for bets are stage ⑦ and the harvest extension is stage ⑧.

The lens's rules are the corpus's: separation of WHO/WHY from NOW, a
human-owned git-committed state file, a closed lifecycle with dated decisions,
identity linked rather than generated, warnings instead of verdicts, and a
strictly additive envelope.

## Goals / Non-Goals

Goals:

- Render identity, bets, and no-gos as their own lens over one human-owned file.
- Keep `focus` and `parked` exactly where they are: curation intent in
  `curation.toml`, never inferred from a bet and never copied into the roadmap.
- Surface commitment decay as named advisory warnings carried by the lens.
- Add the lens to the brief envelope without changing `schema_version`, the
  brief's narrative, or its health verdict.
- Give agents a surface that states constraints and enums, not pitch prose.

Non-Goals:

- Any verb that admits, defers, kills, or rewrites a bet
  (`bet add|admit|defer|kill` is stage ⑦).
- Harvest candidates for identity review or retirement (stage ⑧).
- A second parked list, auto-kill, auto-promote, auto-admit, or any tracker
  write.
- New `SignalKind` variants, a brief narrative change, or a `schema_version`
  bump.
- Portfolio / multi-repo or org-mode rendering, per-repo roadmap thresholds.

## Decisions

### D1. The lens payload is an optional top-level `roadmap{}`

The brief envelope gains `roadmap: Option<RoadmapView>` with
`#[serde(skip_serializing_if = "Option::is_none")]`. `RoadmapView` carries
`identity`, `bets[]`, `no_gos[]`, `review_due{identity, roadmap}`, `warnings[]`,
and `truncation`; the sub-views use typed enums (`Horizon`, `BetStatus`,
`Enforcement`, `RoadmapWarningKind`) with `snake_case` serialization, and dates
use `#[schemars(with = "String")]` exactly as the existing date fields do.

`schema_version` stays the literal `"brief.v1"` (`SCHEMA_VERSION` unchanged,
matching the published schema): the object is additive, introduces no new
required field, and a consumer that ignores unknown fields sees v1 unchanged.
This is deliberate rather than a major bump — additive-only-within-major.
`src/brief/sections.rs` constructs the section assembly with `roadmap: None`, so
every existing caller and fixture keeps compiling and rendering exactly as
before; only `roadmap::attach` sets the object and recomputes `source_hash` over
the attached brief.

### D2. `podras roadmap` reuses the brief read path and emits the same envelope

`podras roadmap [--json] [--as-of <date>] [--write]` collects the same
`BriefInputs` as `podras brief`, loads `.podras/roadmap.toml`, reads tracker
records, assembles the view, attaches it, and prints the same
`Envelope<Brief>` as the brief (no second schema, no wrapper). `--write` targets
`ROADMAP.md` / `ROADMAP.json` in the repository root (or `--out-dir`), through
the same generated-artifact path as the brief renders
(`write::write_roadmap_renders_to`). The roadmap narrative carries no whisper
content, so the brief's `publish_whisper` gate is inherited untouched rather
than re-implemented.

Assembly is pure: `roadmap::assemble(&inputs, &file, &records, evaluation_date)`
takes the evaluation date as an argument, reads no clock, no filesystem, and no
process environment. `roadmap::file::load(repo)` is the only filesystem read,
and it is the reader's job, not the assembler's.

### D3. A malformed state file is a hard error; an absent one is legal

Three cases, deliberately distinct:

| file state | `podras brief` | `podras roadmap` |
|---|---|---|
| absent | no `roadmap{}` object, exit 0 | `roadmap_missing` domain error, exit 1 |
| present, zero bets | present lens (`bets: []`) + identity row | renders the present lens |
| TOML syntax error, unknown key, unknown enum member | `roadmap_parse_error`, exit 1 | `roadmap_parse_error`, exit 1 |
| semantic violation (slug, uniqueness, title length, appetite, required/forbidden `review_or_kill` and `revive_when`, empty required prose, unparseable date, empty `match_terms` entry) | `roadmap_invalid`, exit 1 | `roadmap_invalid`, exit 1 |

An absent file is not a degraded source: absence means "this repository has not
adopted the roadmap lens", which the brief renders by omission. A malformed file
is not degraded either: it is a human-owned file that contradicts its declared
shape, and silently rendering a partial lens would be exactly the manufactured
narrative the corpus forbids. Validation aggregates every violation into one
`roadmap_invalid` message joined by `"; "`, with a suggestion pointing at
`docs/design.md` decision 17, so a hand-editing human fixes the file in one
pass. Slug legality is checked with a manual ASCII loop (`^[a-z0-9][a-z0-9-]*$`)
rather than a new dependency, and dates go through
`crate::curation::parse_human_date` so the file accepts the same ISO-8601 forms
as `curation.toml`.

### D4. The circuit-breaker factor is a module constant

`CIRCUIT_BREAKER_FACTOR: f64 = 1.25` lives in `src/roadmap/warnings.rs` and is
documented as the config key `roadmap.circuit_breaker_factor`. Decision 23
records the corpus's competing proposals (1.5×, 2×) and names 1.25 as the
shipped default; no config layer exists for the roadmap lens, so the constant is
the single declaration point rather than a half-wired setting. For an active bet
(`committed` or `building`), `age = evaluation_date - admitted_at` in days and
`appetite = appetite_weeks * 7`: `age > appetite * 1.25` escalates to
`circuit_breaker_due`, otherwise `age > appetite` emits
`bet_age_exceeds_appetite`. The codes are mutually exclusive by construction —
the escalation replaces the lesser warning instead of stacking with it.

### D5. Deterministic order and declared bounds

Ordering is total and stable, applied before the bound:

- bets: status rank `building`, `committed`, `deferred`, `shaping`, `shipped`,
  `killed`; then `admitted_at` ascending; then `id`;
- no-gos: `id` ascending;
- warnings: code declaration order (`RoadmapWarningKind`'s enum order is the
  order), then `subject`, then `detail`.

Bounds are the shared constants `ROADMAP_BETS_BOUND = 5` and
`ROADMAP_NO_GOS_BOUND = 5`; warnings stay unbounded. Whenever a bound drops an
entry the view records `truncation.applied = true` and, only for non-zero
counts, `omitted_counts["bets"]` / `omitted_counts["no_gos"]` — the same
never-silently-truncate rule the brief applies. `review_due` is not a computed
deadline: `identity` is `identity.next_review` and `roadmap` is the minimum
`review_or_kill` over active bets, each `null` when nothing is outstanding.
Because the assembler is a pure function of `(inputs, file, records,
evaluation_date)`, two runs at the same `--as-of` produce byte-identical
semantic output.

### D6. No verb writes `roadmap.toml` in this stage

The read path never scaffolds, rewrites, or normalizes the state file, and no
command surface here writes it: `podras roadmap --write` writes only the two
renders. `podras init` continues to scaffold curation alone, and its AGENTS.md
managed block gains exactly the two decision-25 lines so drift checking covers
them. Admission, deferral, and kill are human decisions recorded in the file by
hand until stage ⑦ adds verbs — which is also why the lens reports
`roadmap_review_overdue` and `identity_review_overdue` as advisories rather
than acting on them.

## Risks / Trade-offs

- **Hard failure on a malformed file** can block `podras brief` in a repository
  that adopted the lens carelessly. Accepted: the file is human-owned and
  small, the aggregate error lists every violation, and degraded rendering would
  hide a contradiction between declared boundaries and reality.
- **Duplicated bounds reasoning** with the brief: both sides apply the same
  truncation discipline. The rule is shared; the section constants are not,
  because the brief's bounds are per-section and the roadmap's are per-table.
- **A constant factor instead of configuration** ignores the corpus's
  alternatives (1.5×, 2×). Accepted for stage ⑥: the constant is documented as
  the config key it will become, so enabling configuration later is not a
  contract change.
- **`schema_version` stays `brief.v1`** while the payload grows. Additive-only
  is the recorded rule; a consumer relying on exact object equality would
  notice, one relying on the version alone will not. That is the intended
  trade.