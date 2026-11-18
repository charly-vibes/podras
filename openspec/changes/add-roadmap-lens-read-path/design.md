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
(`write::write_roadmap_renders_to(dir, brief)`). The roadmap narrative carries
no whisper content, so the brief's `publish_whisper` gate is inherited untouched
rather than re-implemented.

Assembly is pure: `roadmap::assemble(&file, &records, evaluation_date)` takes
the evaluation date as an argument, reads no clock, no filesystem, and no
process environment. `roadmap::file::load(repo)` is the only filesystem read,
and it is the reader's job, not the assembler's.

### D3. The lens verb hard-fails on a malformed file; an absent one is legal

The cases, deliberately distinct:

| file state | `podras brief` | `podras roadmap` |
|---|---|---|
| absent | no `roadmap{}` object, exit 0 | `roadmap_missing` domain error, exit 1 |
| present, zero bets | present lens (`bets: []`) + identity row | renders the present lens |
| TOML syntax error, unknown key, unknown enum member | omits `roadmap{}`, one warning carrying `roadmap_parse_error`, exit 0 | `roadmap_parse_error`, exit 1 |
| unreadable bytes / non-`NotFound` IO error | omits `roadmap{}`, one warning carrying `roadmap_unreadable`, exit 0 | `roadmap_unreadable`, exit 1 |
| semantic violation (slug, uniqueness, title length, appetite, required/forbidden `review_or_kill` and `revive_when`, empty required prose, unparseable date, empty `match_terms` entry) | omits `roadmap{}`, one warning carrying `roadmap_invalid`, exit 0 | `roadmap_invalid`, exit 1 |

An absent file is not a degraded source: absence means "this repository has not
adopted the roadmap lens", which the brief renders by omission. For
`podras roadmap` a malformed file is not degraded either: it is a human-owned
file that contradicts its declared shape, and silently rendering a partial lens
would be exactly the manufactured narrative the corpus forbids. `podras brief`
must never be blocked by an optional lens, so it degrades instead (D1 in
"Review fixes"): the object is omitted and one curation-sourced warning carries
the domain error's code and message. Validation aggregates every violation into
one `roadmap_invalid` message joined by `"; "`, with a suggestion pointing at
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
never-silently-truncate rule the brief applies. The bound runs first: after the
sort, both tables are cut to their bounds and only then are `review_due` and
every warning computed, over the retained bets, so no warning and no review date
names a bet the reader cannot see in `bets` (D6 in "Review fixes"). `review_due`
is not a computed deadline: `identity` is `identity.next_review` and `roadmap`
is the minimum `review_or_kill` over retained active bets, each `null` when
nothing is outstanding.
Because the assembler is a pure function of `(file, records, evaluation_date)`,
two runs at the same `--as-of` produce byte-identical semantic output.

### D6. No verb writes `roadmap.toml` in this stage

The read path never scaffolds, rewrites, or normalizes the state file, and no
command surface here writes it: `podras roadmap --write` writes only the two
renders. `podras init` continues to scaffold curation alone, and its AGENTS.md
managed block gains exactly the two decision-25 lines so drift checking covers
them. Admission, deferral, and kill are human decisions recorded in the file by
hand until stage ⑦ adds verbs — which is also why the lens reports
`roadmap_review_overdue` and `identity_review_overdue` as advisories rather
than acting on them.

## Review fixes (Rule of 5)

A Rule-of-5 review of the delivered commit (`7d8c12c`) produced fourteen fixes,
all applied by this change. The labels below are the review's; they are
independent of the decision labels in "Decisions" above.

Contracts (mirrored verbatim in the deployed spec and this change's delta):

- **D1 — the brief degrades, the lens verb does not.** A malformed or unreadable
  `.podras/roadmap.toml` makes `podras brief` omit the `roadmap` object, append
  one warning, and exit 0; `podras roadmap` keeps the hard domain error. An
  optional lens must never block the frozen brief narrative.
- **D2 — the degradation warning's shape.** It is
  `Warning { source: SourceName::Curation, code: <the domain code, or
  "roadmap_unreadable">, detail: format!("roadmap: {message}") }`. The brief's
  `sources[]` rows stay the five declared sources; no sixth source is invented
  for the lens.
- **D3 — parked state reaches the lens without a second list.** `ROADMAP.md`'s
  "Not now (parked)" section renders the curation parked bullets first, then
  `brief.needs_retriage` (parked entries whose resurface trigger expired), each
  as `- <id> — <title or none> (needs re-triage: resurface trigger expired)`.
  The parked omission marker is unchanged, because parked rendering still
  belongs to the brief's truncation accounting.
- **D4 — `bet_epic_missing` scope.** It fires for `committed`, `building`, and
  `shipped` bets without an epic. A shipped commitment still owes one tracker
  epic; only terminal `killed` is exempt.
- **D5 — `horizon_epic_mismatch` scope.** It evaluates only non-terminal bets
  (`shaping`, `committed`, `building`, `deferred`); `shipped` and `killed` never
  fire it, because a closed outcome cannot be asked to move horizons.
- **D6 — truncation precedes warnings.** After sorting and applying the 5/5
  bounds, `review_due` and every warning are computed over the retained bets
  only, so every warning subject and review date is visible in `bets`. A
  consequence: the bound drops `shipped`/`killed` bets first, so a
  terminal-status positive can only surface in a fixture small enough to retain
  it. The spec's warnings scenario therefore uses small per-decision fixtures
  rather than one ten-bet repository, and the envelope scenario expects only the
  dropped `shipped` bet (not the dropped `killed` one) to have a suppressed
  `bet_epic_missing`.
- **D7 — the identity advisory covers passed reviews.** `identity_review_overdue`
  fires when `next_review` is absent **or** earlier than the evaluation date.
  Staleness rules are unchanged (`unknown` / `stale` / `fresh`), and the detail
  distinguishes "no next_review declared" from "next_review passed on <date>".
- **D8 — no-go tokens are Unicode-aware.** A token character is any Unicode
  alphanumeric plus `-` and `_`; typographic hyphens U+2010..=U+2015 and U+2212
  normalize to `-` on both the text and the term before matching, so a prose
  hyphen copied from a document still matches a hyphenated match term.

Implementation and documentation corrections:

- **D9 — unreadable bytes are a domain error.** A non-`NotFound` IO error while
  loading the state file is `Error::Domain { code: "roadmap_unreadable", … }`
  with a suggestion, not `Error::Io`; the operator sees a roadmap problem, not a
  generic filesystem failure.
- **D10 — the assembler drops its unused input.** The signature is
  `roadmap::assemble(file: &RoadmapFile, records: &[IssueRecord],
  evaluation_date: NaiveDate) -> RoadmapView`; the `BriefInputs` parameter was
  never read.
- **D11 — the render writer drops its unused parameter.** It is
  `write::write_roadmap_renders_to(dir: &Path, brief: &Brief) -> Result<()>`;
  the `repo` argument was never read.
- **D12 — the module re-exports only the read-path surface.**
  `src/roadmap.rs` re-exports `load`, `RoadmapFile`, and `ROADMAP_REL_PATH`;
  `validate`, `violations`, and the entry types stay crate-reachable through
  `roadmap::file::…`. The file structs drop the `JsonSchema` derive and keep
  `Serialize`/`Deserialize` — the roadmap file is input, not part of any
  published schema.
- **D13 — warning ordering has one implementation.** `src/brief.rs` gains
  `pub(crate) fn sort_warnings_in_place(warnings: &mut [crate::model::Warning])`
  delegating to a `pub(crate)` `brief::ordering::sort_warnings`; the key (source
  declaration order, code, detail) is unchanged, so the brief and the roadmap
  share one sort instead of two copies.
- **D14 — the state file is documented where humans look.** README gains a
  "Roadmap state" section with a `.podras/roadmap.toml` example, stating that
  `ROADMAP.md` is lens-only while `ROADMAP.json` is the full brief envelope; the
  `roadmap_missing` suggestion points at `README.md#roadmap-state` rather than
  at an internal design note.

### Measured false positive: archive duplication

The review predicted that landing the change would duplicate or drop deployed
requirement text when `openspec archive` merged the delta. Measured against the
real archive command: it deduped and reported
`Specs already in sync; no files changed`. No fix was made for it. The deployed
spec and the delta instead keep their requirement and scenario sections
byte-identical by construction, which is re-checked at review time rather than
asserted from tool behaviour.

### Accepted trade-off: `.beads/issues.jsonl` is parsed twice per brief run

The brief's health path already reads tracker records, and the roadmap lens
calls `readers::beads::read_records` itself. Threading the first read through
would change `BriefInputs` — a struct shared by the brief, the roadmap
assembler, and every fixture. The duplicate parse is one small file read on a
read-only path, and buying it back with contract churn was rejected for a
review-fix commit whose contract stays byte-identical. If a later stage adds a
second consumer of records, the change should go through `BriefInputs` once,
deliberately, not as a side effect of this review.

## Risks / Trade-offs

- **Hard failure on a malformed file in `podras roadmap`** can block the lens
  command in a repository that adopted it carelessly. Accepted: the file is
  human-owned and small, the aggregate error lists every violation, and degraded
  rendering would hide a contradiction between declared boundaries and reality.
  `podras brief` is never blocked: it omits the object and warns (D1).
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