## ADDED Requirements

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