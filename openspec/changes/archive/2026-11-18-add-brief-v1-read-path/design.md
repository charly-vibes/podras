# Design: brief v1 read path

## Context

The deployed corpus `openspec/specs/brief-v1/spec.md` fixes the intent, 14
invariants, 14 properties, and the curation-item FSM. `docs/design.md` records
the resolved decisions from grill sessions on 2026-11-17/18. This change covers
build stages 1–3 only: the fixed nine-section read path, the typed agent
envelope (including committed renders), and the `focus`/`park` curation verbs
with `init`. Harvest (stage 4) and freshness hardening (stage 5) are v1.1 and
must not be pulled forward.

The implementation is a new Rust CLI. Every ticket is driven red→green with
`cargo test <property_id>` and passes a Rule-of-5 review. Contract stubs live in
`.espectacular/changes/add-brief-v1-read-path/brief-v1/`; the declared cargo
test filter is the property id, so test function names MUST equal the property
ids exactly.

## Goals / Non-Goals

Goals:

- One deterministic, side-effect-free read path over declared sources.
- A human narrative and a typed envelope with identical semantics.
- Generated committed renders that a hand edit can never corrupt.
- Curation that beats inference: focus and parked state come only from
  `.podras/curation.toml`.
- Per-reader degradation: one broken source never aborts the brief.
- A health verdict that can be `unknown` and never manufactures green.

Non-Goals:

- Harvest/candidate extraction, provenance graphs, or note summarization.
- Freshness hardening beyond deterministic per-source stamps and `--as-of`.
- The roadmap lens, portfolio view, org render, and status dashboard.
- BRIEF.md regen-diff CI gating.
- Repairing the tracker, VCS, spec store, or notes; the brief reads only.

## Decisions

### D1. SCOPE-002 resolved: independent per-reader degradation

The read path is **not** a serial all-or-nothing graph. Each reader is invoked
independently and returns either a source read or a typed source failure:

```rust
struct SourceRead {
    status: SourceState,          // present | missing | unreadable
    observed_at: Option<DateTime<Utc>>,
    detail: Option<String>,
    payload: SourcePayload,       // reader-specific, consumed by brief.rs
}

struct SourceFailure {
    source: SourceName,
    status: SourceState,          // missing | unreadable
    code: &'static str,           // stable warning code
    detail: String,
}
```

`brief.rs` collects every reader's outcome into `sources[]` and `warnings[]`;
no reader error propagates as a hard error. A missing or unreadable source
renders a per-source warning row, keeps the source in `sources[]` with its
declared status, and the process exits 0. Mandatory means "always declared and
always attempted, absence is loud", not "absence aborts the render". This is
what satisfies `brief.v1.sources_declared` and the `missing_source_warns`
property.

Hard errors are reserved for behavior outside the read path: invalid CLI
usage (exit 2) and a failure to write a requested artifact (exit 1).

### D2. Crate layout

```
Cargo.toml
src/main.rs                 clap entrypoint; maps errors to exit codes
src/lib.rs                  public modules; brief orchestration entry
src/error.rs                thiserror enums + miette diagnostics
src/commands.rs             CLI adapter: one entry point per subcommand; delegates to verbs
src/model.rs                Brief, sources, work items, enums, ordering keys
src/brief.rs                assemble model from readers + curation + health
src/brief/tracker.rs        tracker items addressed by identifier
src/brief/curation_view.rs  curated focus ordering and curation stamps
src/brief/parking.rs        parked entries split into still-parked and expired
src/brief/health.rs         human health verdict layer
src/brief/sections.rs       computed sections and envelope assembly
src/brief/ordering.rs       deterministic sort keys and render-bound truncation
src/hash.rs                 semantic payload and source hash
src/readers/mod.rs          Reader trait, SourceRead/SourceFailure, dispatch
src/readers/beads.rs        .beads/issues.jsonl
src/readers/git.rs          HEAD, log window, current branch
src/readers/specs.rs        openspec/changes + openspec/specs observation
src/readers/whisper.rs      latest branch note entry + next-action line
src/curation.rs             .podras/curation.toml load/save/round-trip
src/health.rs               signal detection + layered verdict
src/freshness.rs            per-source observed_at -> fresh|stale|unknown
src/render/mod.rs           selected render target
src/render/markdown.rs      narrative, nine sections
src/render/envelope.rs      typed JSON envelope + JSON Schema emission
src/write.rs                atomic BRIEF.md/BRIEF.json write, privacy gate
src/verbs/mod.rs            verb dispatch
src/verbs/focus.rs          focus select-or-replace
src/verbs/park.rs           park with mandatory resurface trigger
src/verbs/init.rs           curation scaffold + AGENTS.md managed block
src/harvest.rs              reserved v1.1 seam; not wired in v1
tests/common/mod.rs         fixture repository builders
tests/*.rs                  one or more property test functions per file
```

`src/harvest.rs` is a declared v1.1 seam and MUST NOT add a `harvest` verb or
any v1 code path. `src/freshness.rs` is v1 code: it owns the staleness mapping.

### D3. Dependencies

- `genesis-vibes = "0.13"` (crates.io; binary is the genesis shared crate) for
  `managed_block` and the house CLI conventions.
- `clap = { version = "4", features = ["derive"] }`
- `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`,
  `toml = "0.8"`
- `schemars = "1"` (JSON Schema 2020-12 output)
- `miette = "7"`, `thiserror = "2"`
- `chrono = { version = "0.4", features = ["serde"] }`
- `sha2 = "0.10"` for the canonical `source_hash` required by D5.
- dev: `tempfile = "3"`, `assert_cmd = "2"`, `predicates = "3"`,
  `jsonschema = "0.61"` (D14 envelope validation).

`src/commands.rs` is the CLI adapter layer: `src/main.rs` parses arguments and
calls it, and it delegates mutations to `verbs/*`. It owns no policy of its
own.

### D4. Model

`src/model.rs` defines the serialized contract. Enum variants serialize
`snake_case`.

```rust
struct Brief {
    schema_version: String,                 // "brief.v1"
    generated_at: DateTime<Utc>,            // volatile; excluded from semantics
    sources: Vec<SourceStatus>,
    curation: CurationStamp,                // { updated_at, confirmed_at }
    purpose_lite: Option<String>,
    focus: Vec<WorkItem>,
    in_flight: Vec<WorkItem>,
    blocked: Vec<WorkItem>,
    next: Vec<WorkItem>,
    parked: Vec<ParkedItem>,
    needs_retriage: Vec<WorkItem>,
    recent: Vec<RecentEvent>,
    health: Health,
    truncation: Truncation,
    head_sha: Option<String>,
    source_hash: Option<String>,
    warnings: Vec<Warning>,
}

struct SourceStatus {
    name: SourceName,                       // beads|git|openspec|whisper|curation
    status: SourceState,                    // present|missing|unreadable
    observed_at: Option<DateTime<Utc>>,
    staleness: Staleness,                   // fresh|stale|unknown
    detail: Option<String>,
}

struct CurationStamp { updated_at: Option<DateTime<Utc>>, confirmed_at: Option<DateTime<Utc>> }

struct WorkItem {
    id: String,
    title: Option<String>,
    status: Option<String>,                 // tracker status as reported
    priority: Option<u8>,
    blocked_by: Vec<String>,
    age_days: Option<u32>,
    reason: Option<String>,                 // curation reason when curated
    confirmed_at: Option<NaiveDate>,
}

struct ParkedItem { id: String, reason: String, parked_at: NaiveDate,
                    confirmed_at: NaiveDate, resurface: Resurface }

struct RecentEvent { sha: String, committed_at: DateTime<Utc>, subject: String,
                     outcome: Option<String> }

struct Health { status: HealthStatus, basis: HealthBasis,
                signals: Vec<Signal>, owner: Option<String>, reason: Option<String> }

struct Signal { kind: SignalKind, reason: String }

struct Truncation { applied: bool, omitted_counts: BTreeMap<String, u32> }

struct Warning { source: SourceName, code: String, detail: String }
```

No project-wide staleness field exists. `staleness` appears only inside each
`SourceStatus`.

Curation types:

```rust
struct Curation {
    purpose_lite: String,
    max_focus: u32,               // default 1
    publish_whisper: bool,        // default false
    health: CurationHealth,       // { status, owner, confirmed_at, reason }
    focus: Vec<FocusEntry>,       // { id, reason, focused_since, confirmed_at }
    parked: Vec<ParkedEntry>,     // { id, reason, parked_at, confirmed_at, resurface }
}

enum Resurface { Date(NaiveDate), Event(String) }   // "YYYY-MM-DD" | "event:<name>"
```

### D5. Determinism and ordering

- Reads mutate nothing. No cache, lock, or state file is written by `brief`.
- `generated_at` is the only volatile field. The semantic payload is the `Brief`
  with `generated_at` removed (or set to a sentinel for hashing).
- Computed sections (`in_flight`, `blocked`, `next`, `needs_retriage`) sort by
  `(priority ascending, id ascending)`; absent priority sorts last.
- `recent` sorts by `(committed_at descending, sha ascending)`.
- `focus` sorts by `(focused_since ascending, id ascending)`; `parked` by
  `(resurface date ascending, id ascending)`, with event triggers after date
  triggers and then by event name.
- `warnings` sorts by `(source fixed order, code, detail)`.
- `sources` uses the fixed declaration order beads, git, openspec, whisper,
  curation.
- `source_hash` = SHA-256 over the canonical JSON of the semantic payload with
  `generated_at` and `source_hash` omitted. Canonical JSON means sorted object
  keys, no insignificant whitespace. `head_sha` participates when present.
- `--as-of <YYYY-MM-DD>` sets the evaluation clock used by staleness, resurface
  triggers, and health freshness. `generated_at` still records wall-clock time.
  An unparseable date is a usage error (exit 2).

### D6. Bounds and truncation

Constants: `FOCUS_BOUND = 1`, `RECENT_BOUND = 5`, `BLOCKED_BOUND = 5`,
`NEXT_BOUND = 5`, `PARKED_BOUND = 5`. Warnings are unbounded. Bounds apply to
the envelope and the committed JSON; the narrative uses the same bounded lists
so both renders agree. Whenever a bound drops items, `truncation.applied = true`
and `omitted_counts["<section>"]` records the dropped count. Sections that
never truncate MUST NOT appear in `omitted_counts`. A curated focus count above
`max_focus` is truncated to the bound and also fires `wip_over_cap`.

### D7. Curation file: `.podras/curation.toml`

Git-committed. Humans may hand-edit; verbs write it; agents are instructed
never to write it. Writes are atomic (temp file + rename) and serialize the
curation model with `toml` (verb writes normalize formatting and drop comments;
the commented template is only the initial scaffold written by `init`). Unknown
keys are rejected on load (`deny_unknown_fields`) and reported as a
per-source `unreadable` warning with defaults, never a hard failure. Absent
optional fields resolve through accessors: `max_focus` defaults to 1 and
`publish_whisper` to false; `purpose_lite` is a `String` once a file exists.

```toml
purpose_lite = "podras: status read path for a cold return"
max_focus = 1
publish_whisper = false

[health]
status = "unknown"            # green | amber | red | unknown (human verdict)
owner = "@sasha"
confirmed_at = "2026-11-18"
reason = "v1 kickoff"

[[focus]]
id = "podras-d2e"
reason = "proposal phase"
focused_since = "2026-11-18"
confirmed_at = "2026-11-18"

[[parked]]
id = "podras-o47"
reason = "harvest is v1.1"
parked_at = "2026-11-18"
confirmed_at = "2026-11-18"
resurface = "event:value-prop-roadmap"
```

`resurface` is mandatory for every parked entry: either a calendar date
(`YYYY-MM-DD`) or `event:<name>`. A missing or malformed trigger makes the
entry invalid; it is reported as a warning and excluded from the parked
section rather than silently persisting. `curation.updated_at` is the file
mtime of the last verb write; `curation.confirmed_at` is the newest human
`confirmed_at` among health and focus/parked entries.

### D8. Health: signals and layered verdict

Signals (each carries a human-readable `reason`):

| kind | fires when |
|---|---|
| `stale_focus` | a focus entry exists and `max(focused_since, confirmed_at)` is older than 14 days |
| `wip_over_cap` | `focus.len() > max_focus` |
| `stuck_project` | `in_flight` is non-empty and the newest git commit is older than 14 days |
| `focus_closed` | the focused id is absent from the tracker's open items |
| `parked_expired` | a parked entry's date trigger is `<= as_of`, or its event trigger has fired |
| `orphan_commits` | a commit in the 14-day window references no tracker id and touches no open work-item path |
| `spec_change_without_bead` | an openspec change modified in the 14-day window links no tracker id |

Verdict layering, evaluated in order:

1. Fresh human verdict (`confirmed_at` within 14 days): use its status. A human
   `green` with any firing signal is downgraded to `amber`, `basis = human`, and
   `reason` names the firing signal(s); a human `amber`/`red` stands unchanged.
2. No fresh human verdict, at least one signal: `amber`, `basis = signals`,
   `reason` names every signal.
3. Neither: `unknown`, `basis = none`. Silence is never green.

`health.owner` comes from the curation health table. `green` is reachable only
through case 1 with zero firing signals. Expired human verdicts are treated as
absent.

### D9. CLI surface and exit codes

```
podras brief [--json] [--write] [--as-of <YYYY-MM-DD>]
podras focus <id>
podras park <id> --reason <text> --resurface <YYYY-MM-DD|event:<name>>
podras init
podras schema
```

- `brief` default: narrative Markdown to stdout. `--json`: envelope to stdout.
  `--write`: additionally write `BRIEF.md` and `BRIEF.json`; with `--json`,
  print the envelope and write both, otherwise print the written paths.
- `focus`: select-or-replace. If another id is focused, demote it, print
  `focus: <old> -> <new> (previous focus demoted)`. If the id is already
  focused, refresh `confirmed_at` and print a confirmation. The id MUST resolve
  to an open tracker item; otherwise exit 1 with a diagnostic and no write.
- `park`: requires both flags (clap enforces; missing flag exits 2). Writes a
  parked entry, removes the id from `[[focus]]` if present, prints the entry
  and its trigger. The id MUST resolve to an open tracker item; else exit 1.
- `init`: idempotent; exit 0 on success.
- `schema`: emit the envelope JSON Schema (2020-12) to stdout; no repo read.

Exit codes: `0` success including degraded reads; `2` CLI usage errors; `1`
hard errors (unwritable artifact, unresolved id, invalid existing curation that
cannot be preserved).

### D10. Readers

All readers take a `ReadContext { repo_root, as_of, branch }` and return
`Result<SourceRead, SourceFailure>`; none take `&mut` state.

- **beads** (`src/readers/beads.rs`): read `.beads/issues.jsonl` line-delimited.
  Missing directory/file → `missing`, code `beads_missing`. Malformed lines are
  skipped and counted into one `beads_parse_error` warning; the source stays
  `present` with a detail. `observed_at` = newest `updated_at` or file mtime.
- **git** (`src/readers/git.rs`): `git rev-parse HEAD`, current branch, and
  `git log --max-count=200 --format=...`. Missing repo/HEAD → `unreadable`,
  code `git_unreadable`. `observed_at` = HEAD commit time. Recent events carry
  sha, committed_at, subject.
- **openspec** (`src/readers/specs.rs`): observe `openspec/changes/*` and
  `openspec/specs/*`; `observed_at` = newest mtime. Missing `openspec/` →
  `missing`, code `openspec_missing` (best-effort source). Used for
  `spec_change_without_bead` and the sources table; it never fails the brief.
- **whisper** (`src/readers/whisper.rs`): read
  `.whisper/branches/<branch>/notes.md`; fall back to `main`; take only the
  latest `## <timestamp>` entry, its timestamp, and the first line after
  `## Next action`. Missing → `missing`, code `whisper_missing`. The extracted
  next-action line is returned separately as `on_demand_content` and is never
  placed in the envelope.
- **curation** (`src/curation.rs`): load `.podras/curation.toml`; absent →
  in-memory defaults with `missing` and code `curation_missing`; parse error →
  defaults with `unreadable` and code `curation_parse_error`; the brief still
  renders and exits 0.

### D11. Render targets and privacy

`src/render/markdown.rs` renders the nine fixed sections in this order:
purpose/current state, focus, in flight, blocked/waiting, next/ready, parked,
recent, health + confidence, sources & freshness. Empty sections render an
explicit `none` line; sections are never silently omitted. In on-demand mode
the purpose section carries `Re-entry: <whisper next action>` when present.

`src/render/envelope.rs` serializes `Brief` exactly as defined in D4/D6, with
`schema_version = "brief.v1"`. `podras schema` prints
`schemars::schema_for!(Brief)` with `$schema` set to the JSON Schema 2020-12
dialect. No status is rendered as free-form prose; `detail`, `reason`, and
`subject` are data fields, not status fields.

Committed renders (`--write`):

- `BRIEF.json` = the envelope with the same semantic payload as on-demand; only
  `generated_at` may differ.
- `BRIEF.md` = the narrative with a freshness stamp line carrying
  `generated_at` and per-source `observed_at`/`staleness`.
- Whisper note content is excluded from both committed files unless
  `publish_whisper = true`. The whisper `observed_at` stamp is always present.
- Regeneration always replaces both files atomically. A hand edit is never
  merged; the next `--write` produces the fresh bytes. There is no drift check
  or merge path in v1.

### D12. `init`

`podras init`:

1. Creates `.podras/` if absent and writes `curation.toml` from the commented
   template only when the file does not exist. Existing bytes are preserved.
2. Injects or refreshes an AGENTS.md managed block through genesis
   `managed_block` with markers identifying podras and a provenance footer.
   The block states: run `podras brief --json` first; freshness before
   inference; beads owns lifecycle; focus is intent, not permission; never
   silently change focus or unpark; never hand-edit committed renders.
3. Second run changes nothing (byte-identical files) and exits 0.
4. No network access; no tracker mutation.

### D13. Error handling

`src/error.rs` uses `thiserror` enums (`UsageError`, `CurationError`,
`RenderError`, `IoError`) wrapped in a `miette::Report` at `main`. Source
failures are values, not errors. Panics are a bug; the binary never panics on
malformed external input.

### D14. Test strategy and property mapping

Fixtures live in `tests/common/mod.rs`: builders create a temp repo, git
commits, `.beads/issues.jsonl`, `openspec/` trees, whisper notes, and curation
files. Each property is a `#[test]` whose function name is exactly the property
id, so `cargo test <property_id>` selects it and the contract's cargo flags
select it in `ah check`.

| property | test owner |
|---|---|
| `determinism_payload` | `tests/determinism.rs` |
| `missing_source_warns` | `tests/sources.rs` |
| `whisper_privacy` | `tests/write.rs` |
| `focus_cap_signal` | `tests/curation.rs` |
| `park_expired_surfaces` | `tests/curation.rs` |
| `truncation_flagged` | `tests/envelope.rs` |
| `silence_is_unknown` | `tests/health.rs` |
| `hand_edit_render_overwritten` | `tests/write.rs` |
| `stale_source_flagged` | `tests/sources.rs` |
| `envelope_schema_conform` | `tests/envelope.rs` |
| `focus_not_inferred` | `tests/brief.rs` |
| `curation_file_shape` | `tests/curation.rs` |
| `bounds_declared` | `tests/envelope.rs` |
| `corpus_self_lint` | shell: `spk lint openspec/specs` |

`.espectacular/config.toml` declares `cargo = ["cargo", "test"]`. Each cargo
contract carries `flags = "<property_id>"`; the corpus contract carries the
shell command. Stubs stay `status = "active"` until a ticket lands its green
implementation.

### D15. Ticket-to-file map (locked layout)

The beads metadata predates this layout; where it names `src/sources/*` or
`src/render/narrative.rs`, the locked paths below win.

| ticket | deliverable | property |
|---|---|---|
| `podras-t9y` | `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/error.rs`, `justfile` | — |
| `podras-xqr` | `src/model.rs` | `determinism_payload` |
| `podras-l21` | `src/readers/beads.rs` | — |
| `podras-ocl` | `src/readers/git.rs` | — |
| `podras-3hw` | `src/readers/specs.rs` | `missing_source_warns` |
| `podras-c5w` | `src/readers/whisper.rs` | — |
| `podras-i45` | `src/curation.rs` | `curation_file_shape` |
| `podras-j13` | `src/health.rs` | `silence_is_unknown` |
| `podras-ci3` | `src/render/markdown.rs` | `focus_not_inferred` |
| `podras-msq` | `src/render/envelope.rs`, `schema/brief-envelope.schema.json` | `bounds_declared`, `truncation_flagged`, `envelope_schema_conform`, `stale_source_flagged` |
| `podras-8ha` | `src/write.rs`, `src/render/markdown.rs` (privacy path) | `hand_edit_render_overwritten`, `whisper_privacy` |
| `podras-3xw` | `src/verbs/focus.rs` | `focus_cap_signal` |
| `podras-l4v` | `src/verbs/park.rs` | `park_expired_surfaces` |
| `podras-jza` | `src/verbs/init.rs` | — |
| `podras-4i2` | `tests/common/mod.rs`, `tests/experiments/` | validation experiments |
| `podras-o47` | `src/harvest.rs` (v1.1, out of scope) | — |
| `podras-7jd` | `src/freshness.rs` hardening (v1.1, out of scope) | — |
| `podras-mgd` | `scripts/guards/brief-drift.sh` (v1.1, out of scope) | — |
| `podras-cs6` | `versions.ddl.toml` spk refresh (tooling chore, out of scope) | — |
| `podras-d2e` | these change artifacts | — |

## Risks / Trade-offs

- **Independent reads hide coupled failures.** If beads and git disagree, the
  brief shows both; it never reconciles them into a narrative. Mitigation: the
  health signals surface `orphan_commits` and `focus_closed`.
- **Per-source staleness thresholds are heuristics.** They are constants in
  `src/freshness.rs` with the values in D5/D8; v1.1 hardens them.
- **Atomic writes on filesystems without rename semantics.** Fall back to
  write-in-place only after a successful temp write; a failed write exits 1 and
  leaves the prior render intact.
- **`--write` in CI could mask drift.** v1 deliberately has no drift gate;
  `podras-mgd` owns the regen-diff check in v1.1.
- **Whisper parsing is line-oriented and best-effort.** Only the latest entry
  and one next-action line are read; deeper extraction is harvest work.

## Migration Plan

1. Land the crate scaffold and readers behind the `brief` command; no repo
   state changes.
2. Land envelope + narrative + `--write`; existing repositories get a generated
   `BRIEF.md`/`BRIEF.json` on first run.
3. Land `focus`/`park`/`init`; run `podras init` once per repository to create
   `.podras/curation.toml` and the AGENTS.md block.
4. No brief state exists to migrate. Hand-maintained status documents are not
   imported; the brief links identity docs rather than copying them.
5. Rollback is deletion of the generated renders and `.podras/`; the read path
   is side-effect free.

## Open Questions

- None blocking v1. The roadmap-lens contract, org render, and portfolio view
  are parked pending the external `value-prop-roadmap` synthesis and the brief
  v1 validation experiments.