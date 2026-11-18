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
