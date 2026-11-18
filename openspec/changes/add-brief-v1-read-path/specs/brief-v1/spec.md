## ADDED Requirements

### Requirement: Deterministic brief payload

`podras brief` SHALL compute status only from the declared sources and SHALL
NOT store brief state. Two runs over identical inputs with the same `--as-of`
SHALL produce identical semantic payloads, excluding volatile metadata such as
`generated_at`. Reads SHALL be side-effect free. Computed sections SHALL use
deterministic ordering (priority ascending, then id ascending); recent events
SHALL order by commit time descending, then sha; warnings SHALL order by
`(source, code)`.

#### Scenario: determinism_payload

- **WHEN** `podras brief --json` runs twice on an unchanged fixture repository with the same `--as-of`
- **THEN** the two semantic payloads are equal and only `generated_at` may differ
- **VERIFIES** [[brief.v1.determinism_payload]]

### Requirement: Fixed-section brief read path

The brief SHALL render nine fixed sections in cold-return order: purpose /
current state, focus, in flight, blocked / waiting, next / ready, parked,
recent, health + confidence, sources & freshness. Focus and parked SHALL render
exclusively from curation state; a parked item SHALL NOT appear in next / ready
or any computed queue. The narrative and the envelope SHALL carry the same
semantic content.

#### Scenario: focus_not_inferred

- **WHEN** a fixture repository has a tracker top-priority item that differs from the curation focus, and curation also parks an item
- **THEN** `brief.focus.id` equals the curated focus id, differs from the tracker top-priority id, and the parked item is absent from the next section
- **VERIFIES** [[brief.v1.focus_not_inferred]]

### Requirement: Declared sources with independent degradation

The brief SHALL declare beads and git as mandatory sources, openspec as
best-effort, whisper notes as a light-read, and curation as read-if-present.
Each read SHALL be isolated so one missing or unreadable source cannot abort
the render. A missing or unreadable source SHALL produce an explicit
per-source warning row with the source name, its `present` / `missing` /
`unreadable` status, and a stable code, while the remaining sources render and
the command exits 0.

#### Scenario: missing_source_warns

- **WHEN** `podras brief --json` runs on a fixture repository without a spec store
- **THEN** the openspec source status is `missing`, the warnings list is non-empty, and the process exits 0
- **VERIFIES** [[brief.v1.missing_source_warns]]

### Requirement: Per-source freshness

Every source entry in the envelope SHALL report its own `status`,
`observed_at`, `staleness` (`fresh` / `stale` / `unknown`), and `detail`.
Staleness SHALL be judged per source; the envelope SHALL NOT carry a
project-wide staleness field. The curation section SHALL carry `updated_at`
and `confirmed_at` separately: unchanged focus is not stale, while a focus
that no human has recently confirmed is.

#### Scenario: stale_source_flagged

- **WHEN** a fixture repository has one stale source and one fresh source
- **THEN** the stale source entry reports `staleness = stale`, the fresh source entry reports `staleness = fresh`, and the envelope exposes no project-wide staleness field
- **VERIFIES** [[brief.v1.stale_source_flagged]]

### Requirement: Bounded sections with declared truncation

The agent envelope SHALL apply bounded defaults: 1 focus, 5 recent, 5 blocked,
5 next, and 5 parked items, while the warnings list SHALL remain unbounded.
Whenever any bound removes items, the envelope SHALL set
`truncation.applied = true` and SHALL record the per-section removed count in
`truncation.omitted_counts`. Silent truncation SHALL NOT occur.

#### Scenario: bounds_declared

- **WHEN** a fixture repository has more items per section than the declared bounds
- **THEN** the envelope section counts equal the declared bounds (1 focus, 5 recent, 5 blocked, 5 next, 5 parked) and the warnings section remains unbounded
- **VERIFIES** [[brief.v1.bounds_declared]]

#### Scenario: truncation_flagged

- **WHEN** a fixture repository produces more than five recent events
- **THEN** `truncation.applied` is true and `omitted_counts.recent` equals the total recent events minus five
- **VERIFIES** [[brief.v1.truncation_flagged]]

### Requirement: Typed agent envelope

`podras brief --json` SHALL emit a typed envelope carrying only typed fields
and enumerations: `schema_version`, `generated_at`, `sources[]`,
`curation{updated_at, confirmed_at}`, `health{status, basis, signals, owner,
reason}`, `truncation{applied, omitted_counts}`, `head_sha`, `source_hash`,
`warnings[]`. Status values SHALL be enum members, never prose. `podras schema`
SHALL emit the envelope's JSON Schema 2020-12, and every envelope SHALL
validate against it.

#### Scenario: envelope_schema_conform

- **WHEN** `podras brief --json` runs on a fixture repository and its output is validated against the schema emitted by `podras schema`
- **THEN** validation succeeds under JSON Schema 2020-12 and every status field is an enum member
- **VERIFIES** [[brief.v1.envelope_schema_conform]]

### Requirement: Committed render is generated, never authored

`podras brief --write` SHALL write `BRIEF.md` and `BRIEF.json` as generated
artifacts. Committed renders SHALL differ from on-demand output only in
`generated_at` and SHALL carry the freshness stamp. A hand edit SHALL be
overwritten by the next regeneration, never merged. Whisper-derived content
SHALL appear only on demand: the committed render carries the whisper
`observed_at` stamp and no note content unless `publish_whisper` is enabled.

#### Scenario: hand_edit_render_overwritten

- **WHEN** a committed render is hand edited and `podras brief --write` runs again
- **THEN** the regenerated render equals a fresh render and the hand edit is absent
- **VERIFIES** [[brief.v1.hand_edit_render_overwritten]]

#### Scenario: whisper_privacy

- **WHEN** a fixture repository has local session notes and `publish_whisper` is disabled
- **THEN** the committed render contains no note content, its whisper source entry carries `observed_at`, and the on-demand output contains the next-action line
- **VERIFIES** [[brief.v1.whisper_privacy]]

### Requirement: Curation file shape and idempotent init

Curation state SHALL live in the git-committed `.podras/curation.toml` with
`purpose_lite`, `max_focus`, `publish_whisper`, `[health]`, `[[focus]]`, and
`[[parked]]` entries carrying ids, reasons, timestamps, and a date-or-event
resurface trigger. Humans MAY hand-edit it; agents SHALL be instructed never
to. `podras init` SHALL idempotently scaffold the file when absent, preserve
existing bytes, and refresh the AGENTS.md managed block via genesis
`managed_block`.

#### Scenario: curation_file_shape

- **WHEN** a curation file at the default path is parsed, then written back, with focus and parked entries
- **THEN** the parsed focus and parked entries round-trip equal to the written entries and `purpose_lite` is present
- **VERIFIES** [[brief.v1.curation_file_shape]]

### Requirement: Curation verbs for focus and park

`podras focus <id>` SHALL be select-or-replace with a demotion notice, capped
at `max_focus` (default 1). Exceeding the cap SHALL emit a `wip_over_cap`
health warning without blocking the render. `podras park <id>` SHALL require a
reason and a date-or-event resurface trigger; an expired trigger SHALL move the
item to needs-re-triage and out of parked. Parked items SHALL never render as
ready or next.

#### Scenario: focus_cap_signal

- **WHEN** the curation file contains two focus rows while `max_focus` is 1
- **THEN** `podras brief` succeeds and health warnings include the `wip_over_cap` signal
- **VERIFIES** [[brief.v1.focus_cap_signal]]

#### Scenario: park_expired_surfaces

- **WHEN** the curation file contains a parked entry whose resurface trigger has passed
- **THEN** the needs-re-triage section contains the item and the parked section does not
- **VERIFIES** [[brief.v1.park_expired_surfaces]]

### Requirement: Health verdict layering

The health verdict SHALL be layered: a fresh human verdict (`basis = human`)
stands; otherwise firing signals decide (`basis = signals`, `amber`); otherwise
status is `unknown` (`basis = none`). Green SHALL require a fresh human
confirmation and no firing signal, never silence. Signals SHALL be named with a
reason: `stale_focus`, `wip_over_cap`, `stuck_project`, `focus_closed`,
`parked_expired`, `orphan_commits`, `spec_change_without_bead`. Human verdicts
SHALL expire.

#### Scenario: silence_is_unknown

- **WHEN** a fixture repository has no human verdict and no firing signal
- **THEN** the envelope health status is `unknown` and the health basis is `none`
- **VERIFIES** [[brief.v1.silence_is_unknown]]

### Requirement: Corpus self-lint gate

The specification corpus SHALL satisfy its own declared shape before any
runtime tier is claimed: `spk lint openspec/specs` SHALL pass with no issues.

#### Scenario: corpus_self_lint

- **WHEN** `spk lint openspec/specs` runs over this repository
- **THEN** the lint check passes with exit 0 and reports no issues
- **VERIFIES** [[brief.v1.corpus_self_lint]]