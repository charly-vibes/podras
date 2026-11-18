# Design: brief v1.1 — harvest, freshness hardening, and the render drift gate

## Context

`add-brief-v1-read-path` deployed the deterministic read path, the genesis
agent envelope with committed `BRIEF.md` / `BRIEF.json`, and the
`focus` / `park` / `init` curation verbs. The deployed corpus fixes the intent,
constraints, model, and 14 properties in `openspec/specs/brief-v1/spec.md`;
this change adds four implementation-facing requirements and five scenarios
without modifying the deployed corpus.

The three v1.1 tickets were deliberately deferred:

- `podras-o47` — harvest verb (build stage 4). `src/harvest.rs` exists only as
  a seam returning `Error::NotYetImplemented`.
- `podras-7jd` — freshness hardening and the expanded `--as-of` contract
  (build stage 5). Horizons are five separate constants; the green verdict has
  no mandatory-source guard; `--as-of` reproducibility is not pinned in tests.
- `podras-mgd` — the committed-render drift gate, parked behind `--write`.

This design keeps v1's rules: evidence over inference, curation beats
inference, reads are side-effect free, never silently truncate, never turn
unknown into green, and never let discovery become commitment.

## Goals / Non-Goals

Goals:

- Give discovery a landing place: `podras harvest` surfaces typed candidates
  with resolvable provenance, files nothing, and changes nothing.
- Make per-source freshness auditable: one declared threshold table, and a
  verdict that stale/missing mandatory inputs cannot paint green.
- Pin `--as-of` as the evaluation clock: identical inputs at the same date
  produce identical semantics; only `generated_at` moves.
- Detect stale committed renders without writing the working tree, as an
  opt-in gate.

Non-Goals:

- Filing, editing, closing, or commenting on tracker issues from harvest.
- Promoting, replacing, or parking focus from harvest; writing curation.
- New health signal kinds, envelope schema changes, or a project-wide
  staleness timestamp.
- Harvest-driven committed artifacts, scheduled harvests, or note
  summarization beyond deterministic reference extraction.
- The roadmap lens / portfolio view (`podras-5y8`), org render, or per-repo
  threshold configuration.
- Any change to the deployed spec corpus.

## Decisions

### D1. Harvest is a read verb with a typed candidate contract

CLI: `podras harvest [--json] [--as-of <YYYY-MM-DD>]`. There is deliberately no
`--write`, `--out-dir`, or curation flag: harvest cannot commit an artifact or
mutate state.

`src/model.rs` (serialized contract, shared so harness tests and the envelope
stay identical):

```rust
pub struct Harvest {
    pub schema_version: String,          // "harvest.v1"
    pub generated_at: DateTime<Utc>,     // volatile; never part of the candidate set
    pub sources: Vec<SourceStatus>,      // same rows/statuses as the brief
    pub candidates: Vec<Candidate>,
    pub warnings: Vec<Warning>,          // per-source degradation, never truncated
}

pub struct Candidate {
    pub kind: CandidateKind,
    pub summary: String,                 // one deterministic line
    pub evidence: Vec<Evidence>,         // non-empty by construction
}

pub struct Evidence {
    pub kind: EvidenceKind,              // Git | Notes
    pub reference: String,               // serialized as `ref`
}

pub enum CandidateKind {
    NoteReferenceUnresolved,             // declaration order == sort order
    OrphanCommit,
    UnlinkedSpecChange,
}

pub enum EvidenceKind {
    Git,                                 // full 40-char sha
    Notes,                               // `<notes path>#<entry timestamp>`
}
```

`podras harvest --json` emits `genesis::envelope::Envelope<Harvest>` with
`envelope_kind = ok`, no wrapper warnings, and `schema_version = "harvest.v1"`.
The human render groups candidates by `kind` and prints each summary with its
evidence refs; empty groups render an explicit `none` line, never a silent
omission.

Candidate kinds and evidence:

| kind | fires when | evidence |
|---|---|---|
| `note_reference_unresolved` | the latest whisper branch entry names a tracker id absent from the full beads export, or a commit token that `git rev-parse --verify <sha>^{commit}` cannot resolve | `notes`: `<notes.md path>#<entry timestamp>` |
| `orphan_commit` | a commit inside the harvest window (the existing `SIGNAL_HORIZON_DAYS = 14` window) names no tracker id and touches no open work-item path | `git`: commit sha |
| `unlinked_spec_change` | an `openspec/changes/*` entry inside the window links no tracker id in its id or title | `git`: newest commit touching `openspec/changes/<id>` |

Reference extraction is deliberately conservative: tracker tokens must match
the prefixes observed in the beads export, and a token that already resolves is
not a candidate. A candidate whose evidence cannot be resolved is dropped, not
emitted with empty evidence. False negatives are acceptable; false positives
are not, because a candidate is a human triage prompt.

Ordering is total and stable: `(kind declaration order, first evidence.ref,
summary)`. Detection uses `BTreeSet`/`BTreeMap` internally, never hash iteration
order. `generated_at` is excluded from idempotency: two runs over unchanged
inputs at the same `--as-of` produce the same candidate list, byte for byte.

### D2. Harvest reads, never writes

`src/harvest.rs` holds the public candidate contract and the assembler;
detectors live beside it: `src/harvest/detectors.rs` (tracker vocabulary, shared
predicates, ordering), `src/harvest/notes.rs` (latest note entry),
`src/harvest/git.rs` (orphan commits plus the bounded git queries),
`src/harvest/specs.rs` (unlinked spec changes), and `src/harvest/render.rs`
(narrative and envelope). The module has no write path: it does
not call `curation::save`, the verbs, `bd`, or any file-writing helper. Its only
output is stdout (human or envelope). Idempotency follows from purity: the
function takes `&BriefInputs` plus the evaluation date and returns `Harvest`.

Source reads stay in the existing readers; harvest adds read-only queries only:

- `src/harvest/git.rs`: `commit_touching(path) -> Option<String>` (one bounded
  `git log -1 --format=%H -- <path>`) and `commit_exists(token) -> bool`
  (`git rev-parse --verify --quiet <token>^{commit}`). Both are read-only and
  run per candidate, bounded by the candidate count.
- `src/readers/whisper.rs`: `parse_latest_entry` additionally returns the
  latest entry body. `WhisperReading` gains `latest_entry_body: Option<String>`,
  an internal field the brief never renders; the committed-render privacy gate
  is unchanged (`write.rs` still passes whisper content only on demand and only
  with `publish_whisper`).
- `src/brief.rs`: `BeadsSummary` gains `all_ids: BTreeSet<String>` (every id in
  the export, not only open items), so note references to closed ids do not
  become candidates. Not serialized.

Shared detection rules are not duplicated: the orphan-commit and unlinked-
spec-change predicates move next to the tracker-linking vocabulary in
`src/health/links.rs` (as pure helpers) and are consumed by both
`src/health/git.rs` / `src/health/specs.rs` (summarized into one signal) and
`src/harvest/*` (one candidate per item, with provenance). The health signal
keeps its existing shape and reason.

Degraded sources behave like the brief: missing beads/git/specs/whisper rows
become warnings, candidate families that need the missing source are skipped,
and harvest exits 0. Exit codes: `0` success (including degraded reads), `2`
CLI usage errors (bad date, unknown flag), `1` hard errors only (repository
root resolution).

### D3. Freshness thresholds are one declared table

`src/freshness.rs` replaces the five constants and the `match` arm with one
public table. Every staleness verdict reads it; no caller declares a horizon.

```rust
pub struct Horizon { pub source: SourceName, pub days: i64 }

/// The only declaration of per-source freshness horizons.
pub const HORIZONS: [Horizon; 5] = [
    Horizon { source: SourceName::Beads,    days: 7 },
    Horizon { source: SourceName::Git,      days: 7 },
    Horizon { source: SourceName::Openspec, days: 14 },
    Horizon { source: SourceName::Whisper,  days: 14 },
    Horizon { source: SourceName::Curation, days: 14 },
];

fn horizon_days(source: SourceName) -> i64 {
    HORIZONS
        .iter()
        .find(|horizon| horizon.source == source)
        .map(|horizon| horizon.days)
        .expect("every declared source has exactly one horizon")
}
```

`SourceName::ALL: [SourceName; 5]` is added in `src/model.rs` so tests can
assert the table covers every declared source exactly once. `evaluate`
semantics stay v1's: age is `evaluation_date - observed_at.date_naive()`; an
observation exactly on its horizon is `fresh`, one day past is `stale`, and an
absent `observed_at` is `unknown`. Thresholds stay documented in the module
doc comment and in the table itself; they are not serialized into the envelope,
which keeps per-source `staleness` and adds no project-wide field (the deployed
`per_source_freshness` invariant and the v1 envelope schema are unchanged).

### D4. Stale mandatory inputs cannot yield green

The verdict gains one guard, evaluated after the existing signal set. In
`src/freshness.rs`:

```rust
pub fn impaired_mandatory(sources: &[SourceStatus]) -> Vec<SourceName> {
    sources
        .iter()
        .filter(|row| matches!(row.name, SourceName::Beads | SourceName::Git))
        .filter(|row| row.status != SourceState::Present || row.staleness != Staleness::Fresh)
        .map(|row| row.name)
        .collect()
}
```

`src/brief.rs` computes this before `health::resolve_health` and passes it to
`health::layer`. The green branch keeps v1's requirements (fresh human verdict,
no firing signal) and adds a third: no impaired mandatory source. A fresh human
green with an impaired mandatory source becomes `status = amber`,
`basis = human`, `reason = "human green downgraded by unusable mandatory
sources: beads (stale)"`; the human amber/red and no-fresh-verdict paths are
unchanged. No `SignalKind` is added, so the envelope enum and schema do not
change. Openspec and whisper stay best-effort: they report their own staleness
and warn, but do not veto a human verdict. Curation missing already forces
`unknown` (no fresh human verdict), so it needs no extra rule.

Fault injection is proof, not narrative: `tests/health.rs` and the
`podras-4i2` experiment suite (`tests/experiments/staleness.rs`) kill the
tracker, age the beads export past 7 days, close the focus silently, and edit a
spec after harvest. Each injection must surface in `sources[]`, signals, or the
verdict; none may render `green`.

### D5. `--as-of` is the pinned evaluation clock

No model change is needed: `BriefInputs.as_of` already flows to freshness,
resurface triggers, and human-verdict freshness, and `generated_at` stays
wall-clock. v1.1 pins the contract in tests rather than trusting it:

- `tests/determinism.rs::as_of_reproducible_render` runs `podras brief --json
  --as-of 2026-11-18` twice and `podras brief --write --as-of 2026-11-18`
  twice. Payloads are compared after deleting `generated_at`; `BRIEF.json`
  renders must differ only in `generated_at`; `BRIEF.md` renders must differ
  only in the `generated_at` value inside the `Freshness:` stamp line.
- The fixture places a human verdict and a parked resurface trigger exactly on
  their freshness boundaries, so an off-by-one evaluation clock flips the
  verdict and fails the test.
- An unparseable date is already a clap usage error (exit 2); the test asserts
  no `BRIEF.md` / `BRIEF.json` is written.
- Harvest accepts the same flag; its window decisions and candidate order are
  functions of `(inputs, as_of)` only, which is what makes the idempotency
  scenario meaningful across a date boundary.

### D6. Render drift gate

`podras brief --write` currently always targets the repository root, so a gate
cannot regenerate without overwriting the artifacts it is comparing. Two
surfaces are added, both deliberately narrow:

```
podras brief [--json] [--write] [--as-of <date>] [--out-dir <DIR>]
```

- `--out-dir <DIR>` requires `--write` and writes `BRIEF.md` / `BRIEF.json`
  into an existing directory; the default remains the repository root. A
  missing or unwritable directory is a hard error (exit 1) and writes nothing.
- `src/write.rs` gains `write_renders_to(dir, ...)`; `write_renders(repo, ...)`
  becomes a thin call with `dir = repo`. Privacy gating (`publish_whisper`) and
  overwrite-never-merge semantics are unchanged.

`scripts/guards/brief-drift.sh`:

1. `set -euo pipefail`; resolve the repository root with
   `git rev-parse --show-toplevel`; require `jq` and a `podras` binary
   (`${PODRAS_BIN:-podras}`).
2. No-op (exit 0, message) when neither `BRIEF.md` nor `BRIEF.json` exists;
   one without the other is a hard error (exit 1).
3. Evaluation date: `$1` / `PODRAS_DRIFT_AS_OF` when supplied, else the date
   prefix of `BRIEF.json`'s `.data.generated_at`. The committed render records
   its own evaluation date, so the gate compares the same clock the render was
   built with instead of drifting with today. Validate `YYYY-MM-DD`; a bad
   value exits 2.
4. `mktemp -d` plus `trap` cleanup; run
   `podras brief --write --as-of "$as_of" --out-dir "$tmp"`.
5. Normalize before diffing. Volatile and provenance-only fields are removed;
   semantic content is compared exactly:
   - JSON: delete `.data.generated_at`, `.data.source_hash`,
     `.data.warnings`, `.meta.author`; delete `observed_at` and `staleness`
     from every `.data.sources[]` row.
   - Markdown: truncate at the `## Sources & freshness` heading (the final
     section) and delete the `Freshness: generated_at ...` line. The narrative
     sections above it, including health status, signals, focus, parked,
     recent, and truncation accounting, are compared unchanged.
6. `diff -u` the committed and regenerated normalized bytes. Any difference
   prints the artifact name and the hunk and exits 1; both matches exit 0.
   Snapshot `git status --porcelain` before and after and fail if the working
   tree changed — the gate itself must never write the tree.

The normalization boundary is explicit: the gate detects *content* drift, not
freshness drift. Freshness stays visible in the committed render and in health;
observed mtimes are clone-local and cannot be compared across machines.

Wiring is opt-in. `justfile` gains a `brief-drift` recipe. `lefthook.yml` gets a
commented stanza outside the `ddl:managed` blocks so `ddl init --gates` never
owns it; adopting repositories uncomment it. This repository has no CI
directory, so the script is the CI unit: run `scripts/guards/brief-drift.sh`
directly in any pipeline that has committed renders.

## Ticket-to-file map (locked)

| ticket | deliverable | scenarios |
|---|---|---|
| `podras-o47` | `src/harvest.rs`, `src/harvest/{notes,commits,specs}.rs`, `src/model.rs` (harvest types + `SourceName::ALL`), `src/brief.rs` (`all_ids`, whisper body), `src/readers/git.rs`, `src/readers/whisper.rs`, `src/commands.rs`, `src/main.rs`, `src/render/harvest.rs`, `tests/harvest.rs`, `README.md`, `src/verbs/init.rs` | `harvest_candidates_never_decisions`, `harvest_idempotent_provenance` |
| `podras-7jd` | `src/freshness.rs` (table + guard), `src/brief.rs` (guard call), `src/health/verdict.rs`, `src/health/links.rs` (shared predicates), `tests/freshness.rs`, `tests/health.rs`, `tests/experiments/staleness.rs`, `tests/determinism.rs` | `freshness_thresholds_declared`, `as_of_reproducible_render` |
| `podras-mgd` | `scripts/guards/brief-drift.sh`, `src/write.rs`, `src/commands.rs`, `src/main.rs`, `tests/drift_gate.rs`, `lefthook.yml`, `justfile`, `README.md` | `drift_gate_detects_stale_render` |
| shared | `.espectacular/changes/add-brief-v1-1/brief-v1/*.toml` | all five |

Beads metadata names `src/verbs/harvest.rs`, `src/render/envelope.rs`, and
`src/sources/mod.rs`; the locked layout above wins, as in v1.

## Risks / Trade-offs

- **Candidate false positives erode trust.** Reference extraction is
  conservative (known prefixes, resolvable evidence, full-export id set) and
  candidates are advisory. A missed mismatch is cheaper than a wrong prompt;
  harvest never acts, so a false positive costs a human glance, not a state
  change.
- **Harvest reads more note text than the brief.** The latest entry body is
  loaded for extraction. Mitigation: the body stays internal to `BriefInputs`,
  harvest has no `--write`/`--out-dir`, and the existing `whisper_privacy`
  test still proves committed renders carry no note content.
- **A green downgrade changes verdicts in adopting repositories.** A stale
  beads export now yields amber where a fresh human said green. That is the
  point of the guard: freshness is evidence, and stale evidence cannot support
  a healthy claim. The reason names the impaired source so the fix is obvious.
- **The drift gate ignores freshness.** Normalizing observed_at/staleness makes
  the gate clone-portable but unable to detect "the render's freshness stamp is
  old". That is already visible in the committed render and in the health
  verdict; the gate is a content-regeneration check.
- **Opt-in gate can be forgotten.** Being opt-in is deliberate (adopting repos
  without renders must stay green); a stale gate is caught by code review and
  by the rendered health section, not by a default hook.
- **`--out-dir` widens the CLI.** It is gated on `--write`, writes exactly two
  files into an existing directory, and leaves the default path untouched.

## Migration Plan

1. Land harvest types, detectors, and the verb behind `podras harvest`; no
   repository state changes and no new committed files.
2. Land the freshness table and verdict guard; adopting repositories may see
   `amber` where a stale mandatory source previously coexisted with green —
   the correct signal, reported with the impaired source.
3. Land `--out-dir`, the gate script, and opt-in wiring. Regenerate
   `BRIEF.md` / `BRIEF.json` once (`podras brief --write --as-of <today>`)
   before enabling the gate, so the baseline is fresh.
4. Re-run `podras init` in adopting repositories to refresh the AGENTS.md
   managed block with the harvest rule; the run stays idempotent and preserves
   existing bytes.
5. Rollback: harvest adds no state to remove; the guard is a pure function;
   the gate and wiring are delete-only. Committed renders remain valid v1
   artifacts.

## Open Questions

- Whether harvest should ever propose a draft bead body (explicitly excluded
  here: discovery is not commitment).
- Whether per-repo threshold overrides belong in `.podras/curation.toml`
  (deferred; thresholds stay code constants until a repository needs otherwise).
- Whether the drift gate should also compare the exact `sources[]` provenance
  when both runs happen on the same machine (deferred; opt-in strict mode
  would require a render-time environment fingerprint).