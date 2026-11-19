## MODIFIED Requirements

### Requirement: Harvest candidates are proposals, never decisions

`podras harvest [--json] [--as-of <date>]` SHALL extract typed candidates from
whisper notes, git, and the spec store. Every candidate SHALL carry `kind`,
`summary`, and at least one `{kind: git|notes|roadmap, ref}` evidence entry;
evidence-free candidates SHALL NOT be emitted. Harvest SHALL be idempotent,
SHALL NOT write curation, create or close issues, or change focus/parked state,
and SHALL exit 0 on degraded sources. Output is on-demand only: no committed
artifact or `--write`/`--out-dir`.

#### Scenario: harvest_candidates_never_decisions

- **WHEN** `podras harvest --json` runs on a fixture repository whose notes, git history, and spec store contain an orphan commit, an unlinked spec change, and a note reference to a missing tracker id
- **THEN** every emitted candidate carries a kind, a summary, and at least one git-, notes-, or roadmap-sourced evidence ref; the curation file and tracker export bytes are unchanged; focus and parked entries are unchanged; and the command exits 0
- **VERIFIES** [[brief.v1.harvest_candidates_never_decisions]]

#### Scenario: harvest_state_mutation_is_a_violation

- **WHEN** `podras harvest` runs on a fixture repository with curated focus and parked entries and open tracker items
- **THEN** harvest exits 0 having written nothing — a run that changed focus, parked state, or tracker export bytes would violate candidates-never-decisions
- **VERIFIES** [[brief.v1.harvest_candidates_never_decisions]]

#### Scenario: harvest_idempotent_provenance

- **WHEN** `podras harvest` runs twice over unchanged inputs with the same `--as-of`
- **THEN** both runs emit the same candidate set in the same order, every evidence ref resolves to an existing git object, note entry, or roadmap state file, and no repository file changes between runs
- **VERIFIES** [[brief.v1.harvest_idempotent_provenance]]
