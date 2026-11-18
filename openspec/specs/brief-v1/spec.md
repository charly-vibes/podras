---
id: brief.v1
kind: intent
statement: "WHEN a human returns cold or an agent starts cold in a git repository, THE brief-v1 capability SHALL render a fixed-section project status brief (narrative for humans, typed envelope for agents) derived only from the tracker, the VCS, the spec store, session notes, and curation state, with per-source freshness, bounded defaults, and a health verdict that is never manufactured green."
---

# brief-v1 Specification

## Purpose

podras answers one question — *what is happening in this project now, and
what deserves attention next* — for a human returning cold and for an AI
agent starting cold, from one git-committed source. The objective is
deliberately narrow (machine census 2026-11-18: 118 repos, 58 touched
within 7 days, 1,469 open tracker issues, 275/315 spec changes stale >14d;
roadmap/vision documents maintained in no repo; why-statement one-liners
already live in flagship READMEs). The why layer is the identity doc's job;
this capability owns only the *now* lens and links, never duplicates.

v1 spans build stages 1–3: the read path (fixed 9-section brief), the typed
agent envelope, and the `focus`/`park` curation verbs. Harvest (session-note
candidate extraction) and freshness hardening are v1.1 and out of scope
here; this spec states the contract those stages must not break.

Render targets: on-demand narrative + typed JSON envelope, plus a
**committed** `BRIEF.md`/`BRIEF.json` written by `brief --write` — a
generated artifact, never hand-maintained, visible to non-cloners and
tool-less agents. Whisper session notes are read lightly (latest branch
entry: timestamp and next-action line only). Note *content* never reaches
the committed render (on-demand only; per-repo `publish_whisper` opt-in).

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| derived_never_authored | invariant | `the brief is computed only from declared sources (tracker, VCS, spec store, session notes, curation file); no stored brief state exists — committed BRIEF.md/BRIEF.json are outputs of brief --write and are regenerable; a hand-edit to any committed render is overwritten by the next regeneration, never merged` | [[brief.v1]] |
| contract_not_prompt | invariant | `the agent envelope carries typed fields and enumerations only — schema_version, generated_at, sources[] (status, observed_at), curation{updated_at, confirmed_at}, staleness fresh|stale|unknown, truncation{applied, omitted_counts}, head_sha/source_hash, health{status, basis} — and reports no status as prose` | [[brief.v1]] |
| per_source_freshness | invariant | `every declared source reports its own status and observed_at; staleness is judged per source, never as one project-wide timestamp; the curation section carries updated_at and confirmed_at separately — unchanged focus is not stale, an unconfirmed-by-human focus is` | [[brief.v1]] |
| never_silently_truncate | invariant | `whenever a bound is applied to any section, the envelope records truncation.applied = true and per-section omitted_counts; unbounded is the exception, never the default lie` | [[brief.v1]] |
| curation_beats_inference | invariant | `focus and parked sections render exclusively from the curation file; focus is never inferred from tracker priority and progress is never inferred from commit counts; a parked item is never rendered as ready or next` | [[brief.v1]] |
| focus_singular | invariant | `at most max_focus (default 1) items are focused per repository; the focus verb is select-or-replace — selecting a new focus demotes the previous one with a printed notice; exceeding the cap (possible only by hand-editing the curation file) emits a WIP-over-cap health warning and never blocks a render` | [[brief.v1]] |
| park_requires_resurface | invariant | `every parked entry carries a resurface trigger (date or event); an expired trigger moves the item to the needs-re-triage section — parked items never silently persist` | [[brief.v1]] |
| unknown_never_green | invariant | `the health verdict is layered: a fresh human verdict (basis = human, unexpired confirmed_at) stands; otherwise computed signals decide (basis = signals, amber when any signal fires); otherwise status is unknown (basis = none) — a green verdict requires a fresh human confirmation and no firing signal, and is never synthesized from silence` | [[brief.v1]] |
| reads_side_effect_free | invariant | `brief reads mutate nothing; identical inputs under --as-of produce identical semantic output with defined ordering (priority then id); volatile metadata (generated_at) is separated from the semantic payload` | [[brief.v1]] |
| whisper_content_privacy | invariant | `whisper-derived content (next-action lines, session summaries) appears only in on-demand output; the committed render carries the whisper freshness stamp (observed_at) and no note content unless publish_whisper is explicitly enabled per repository` | [[brief.v1]] |
| sources_declared | invariant | `beads and git are mandatory sources; openspec is best-effort; whisper notes are light-read (latest branch entry only); a missing or unreadable source produces an explicit per-source warning row — never a render failure and never a silent omission` | [[brief.v1]] |
| bounded_defaults | invariant | `the agent envelope defaults are bounded: 1 focus, 5 recent, 5 blockers, 5 next, 5 parked — plus all warnings unbounded; bounds are declared by the artifact itself` | [[brief.v1]] |
| curation_location | invariant | `curation state lives in .podras/curation.toml, git-committed: a purpose-lite one-liner plus focus and parked entries (id, reason, focused_since/parked_at, confirmed_at, resurface trigger); the focus and park verbs write it; humans may hand-edit it; agents are instructed never to (AGENTS.md managed block)` | [[brief.v1]] |
| enforcement_routed | invariant | `every row states its tier: tier 1 — enforced by spk lint over this corpus (table schema, reference typing, graph shape); tier 2 — enforced by the property suite declared below once the CLI exists (red until implementation, per the house red→green discipline); tier 3 — runtime rows (envelope fields, committed-render policy) enforced via fixture fixtures through the compile/verify pipeline in v1.1` | [[brief.v1]] |

## Model

Curation-item lifecycle (one item of work in a repository):

### States
- `unfocused`
- `focused`
- `parked`
- `needs_retriage`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| select | unfocused | focused | [[brief.v1.focus_singular]] ∧ `human ran podras focus <id> and the id resolves to an open tracker item` |
| demote | focused | unfocused | [[brief.v1.focus_singular]] ∧ `a newer focus was selected (replacement notice printed) or the focused tracker item closed` |
| park | unfocused | parked | [[brief.v1.park_requires_resurface]] ∧ `human ran podras park <id> with reason and resurface trigger recorded` |
| unpark_focused | focused | parked | [[brief.v1.park_requires_resurface]] ∧ `human parked the current focus with reason and resurface trigger recorded` |
| expire | parked | needs_retriage | [[brief.v1.park_requires_resurface]] ∧ `resurface date passed or resurface event fired` |
| retriage_focus | needs_retriage | focused | [[brief.v1.focus_singular]] ∧ `human re-triaged and selected the item as focus` |
| retriage_drop | needs_retriage | unfocused | [[brief.v1.curation_beats_inference]] ∧ `human decided the item returns to the computed queue without focus` |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| determinism_payload | unit | [[brief.v1.reads_side_effect_free]] | `brief_run_twice_on_fixture_repo()` | `semantic_payload(run1) == semantic_payload(run2)` — generated_at may differ, nothing else |
| missing_source_warns | unit | [[brief.v1.sources_declared]] | `fixture_repo_without_spec_store()` | `envelope.sources[spec_store].status == missing ∧ warnings non-empty ∧ exit == 0` |
| whisper_privacy | unit | [[brief.v1.whisper_content_privacy]] | `fixture_repo_with_local_session_notes()` | `committed_render contains no note content ∧ committed_render.sources[whisper].observed_at present ∧ on_demand_output contains the next-action line` |
| focus_cap_signal | unit | [[brief.v1.focus_singular]] | `curation_file_with_two_focus_rows()` | `brief succeeds ∧ health warnings include wip_over_cap` |
| park_expired_surfaces | unit | [[brief.v1.park_requires_resurface]] | `curation_file_with_parked_entry_past_trigger()` | `needs_retriage section contains the item ∧ parked section does not` |
| truncation_flagged | unit | [[brief.v1.never_silently_truncate]] | `fixture_repo_with_more_than_five_recent_events()` | `envelope.truncation.applied == true ∧ omitted_counts.recent == total - 5` |
| silence_is_unknown | unit | [[brief.v1.unknown_never_green]] | `fixture_repo_with_no_human_verdict_and_no_firing_signals()` | `envelope.health.status == unknown ∧ envelope.health.basis == none` |
| hand_edit_render_overwritten | unit | [[brief.v1.derived_never_authored]] | `committed_render_hand_edited_then_regenerated()` | `regenerated render equals fresh render ∧ hand edit absent` |
| stale_source_flagged | unit | [[brief.v1.per_source_freshness]] | `fixture_repo_with_one_stale_source_and_one_fresh_source()` | `envelope.sources[stale].staleness == stale ∧ envelope.sources[fresh].staleness == fresh ∧ no project-wide staleness field exists` |
| envelope_schema_conform | unit | [[brief.v1.contract_not_prompt]] | `envelope_from_fixture_repo()` | `schema_validate(envelope, published_json_schema_2020_12) == valid ∧ every status field is an enum member` |
| focus_not_inferred | unit | [[brief.v1.curation_beats_inference]] | `fixture_repo_where_top_priority_differs_from_curation_focus()` | `brief.focus.id == curation focus id ∧ brief.focus.id != tracker top priority id ∧ parked items absent from next section` |
| curation_file_shape | unit | [[brief.v1.curation_location]] | `curation_toml_at_default_path_with_focus_and_parked()` | `parsed focus/parked entries round-trip equal to written entries ∧ purpose_lite present` |
| bounds_declared | unit | [[brief.v1.bounded_defaults]] | `fixture_repo_with_many_items_per_section()` | `envelope section counts == declared bounds (1 focus, 5 recent, 5 blockers, 5 next, 5 parked) ∧ warnings section unbounded` |
| corpus_self_lint | unit | [[brief.v1.enforcement_routed]] | `spk_lint_over_this_corpus()` | `check(openspec/specs) == passed` — the corpus must satisfy its own declared shape before any runtime tier is claimed |

## Requirements

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

### Requirement: Harvest candidates are proposals, never decisions

`podras harvest [--json] [--as-of <date>]` SHALL extract typed candidates from
whisper notes, git, and the spec store. Every candidate SHALL carry `kind`,
`summary`, and at least one `{kind: git|notes, ref}` evidence entry;
evidence-free candidates SHALL NOT be emitted. Harvest SHALL be idempotent,
SHALL NOT write curation, create or close issues, or change focus/parked state,
and SHALL exit 0 on degraded sources. Output is on-demand only: no committed
artifact or `--write`/`--out-dir`.

#### Scenario: harvest_candidates_never_decisions

- **WHEN** `podras harvest --json` runs on a fixture repository whose notes, git history, and spec store contain an orphan commit, an unlinked spec change, and a note reference to a missing tracker id
- **THEN** every emitted candidate carries a kind, a summary, and at least one git-or-notes evidence ref; the curation file and tracker export bytes are unchanged; focus and parked entries are unchanged; and the command exits 0
- **VERIFIES** [[brief.v1.harvest_candidates_never_decisions]]

#### Scenario: harvest_idempotent_provenance

- **WHEN** `podras harvest` runs twice over unchanged inputs with the same `--as-of`
- **THEN** both runs emit the same candidate set in the same order, every evidence ref resolves to an existing git object or note entry, and no repository file changes between runs
- **VERIFIES** [[brief.v1.harvest_idempotent_provenance]]

### Requirement: Declared freshness thresholds and fault-resistant verdicts

Staleness SHALL read one declared threshold table covering every source exactly
once: beads 7 days, git 7 days, openspec 14, whisper 14, curation 14. An
observation on its horizon SHALL be `fresh`, one day past `stale`, absent
`unknown`. A mandatory source (beads, git) that is missing, unreadable, or
stale SHALL prevent `green`: a fresh human green SHALL downgrade to `amber`
(`basis = human`), naming the impaired source. The envelope SHALL keep
per-source `staleness` only.

#### Scenario: freshness_thresholds_declared

- **WHEN** the freshness thresholds are read and a fixture repository is evaluated with one source observed exactly on its horizon, another observed one day past it, and beads fault-injected as stale under a fresh human green verdict
- **THEN** the table declares beads 7, git 7, openspec 14, whisper 14, and curation 14 days exactly once per source; the on-horizon source is `fresh`, the past-horizon source is `stale`; and health is `amber`, not `green`, with the impaired mandatory source named in the reason
- **VERIFIES** [[brief.v1.freshness_thresholds_declared]]

### Requirement: Reproducible as-of evaluation

`podras brief --as-of <date>` SHALL pin every date-sensitive decision:
staleness, resurface triggers, and human-verdict freshness. `generated_at`
SHALL remain wall-clock and the only volatile field; with the same `--as-of`
and inputs, payloads and committed renders SHALL differ only in
`generated_at`. An unparseable date SHALL exit 2 and write no render. Harvest
SHALL accept the same flag with the same semantics.

#### Scenario: as_of_reproducible_render

- **WHEN** `podras brief --json --as-of 2026-11-18` and `podras brief --write --as-of 2026-11-18` each run twice on unchanged inputs, including a human verdict and a parked trigger exactly on their freshness boundaries
- **THEN** the two payloads are equal after removing `generated_at`, the two `BRIEF.json` renders differ only in `generated_at`, the two `BRIEF.md` renders differ only in the `generated_at` value of the freshness stamp, and the boundary verdicts do not flip between runs
- **VERIFIES** [[brief.v1.as_of_reproducible_render]]

### Requirement: Committed render drift gate

The repository SHALL provide `scripts/guards/brief-drift.sh` to regenerate both
renders into a temporary directory at the committed render's evaluation date
(or an explicit date), normalize volatile and freshness-provenance fields, and
diff. It SHALL exit 0 on a match, non-zero naming the divergent artifact on
drift, never write the working tree, and no-op without committed renders.
`brief --write --out-dir <DIR>` SHALL target a directory. The gate SHALL be
opt-in in lefthook/CI.

#### Scenario: drift_gate_detects_stale_render

- **WHEN** `scripts/guards/brief-drift.sh` runs against a committed render, and again after the fixture repository gains a commit without re-rendering
- **THEN** the first run exits 0, the second exits non-zero and names the divergent render, and `BRIEF.md` and `BRIEF.json` are byte-identical before and after both runs
- **VERIFIES** [[brief.v1.drift_gate_detects_stale_render]]
