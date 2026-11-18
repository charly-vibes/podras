# Tasks: add-brief-v1-1

Checklist maps every v1.1 board ticket to its deliverable files and the scenario
it owns. Test function names MUST equal scenario ids; each contract stub's
cargo filter is the scenario id. Nothing here is implemented — this change is
the proposal/contract phase. Mark items only when the ticket's Rule-of-5 review
has passed.

## 0. Change artifacts and gates

- [x] 0.1 (podras-o47, podras-7jd, podras-mgd) Delta spec at `openspec/changes/add-brief-v1-1/specs/brief-v1/spec.md`: four ADDED requirements for harvest, declared freshness thresholds with the mandatory-source guard, reproducible `--as-of`, and the committed-render drift gate. Scenario: — (five scenarios cover the change).
- [x] 0.2 (podras-o47, podras-7jd, podras-mgd) `proposal.md`, `design.md`, `tasks.md` under `openspec/changes/add-brief-v1-1/`. Scenario: —.
- [x] 0.3 (podras-o47, podras-7jd, podras-mgd) Five contract stubs at `.espectacular/changes/add-brief-v1-1/brief-v1/*.toml` with `[tests]` cargo flags = scenario id: `harvest-candidates-never-decisions`, `harvest-idempotent-provenance`, `freshness-thresholds-declared`, `as-of-reproducible-render`, `drift-gate-detects-stale-render`. Scenario: all five.
- [x] 0.4 (gate) `openspec validate add-brief-v1-1 --strict` exits 0 and `spk lint openspec/specs` exits 0. Scenario: `corpus_self_lint` (deployed v1 contract, rerun here).
- [x] 0.5 (gate) `ah check --changes add-brief-v1-1` structural findings 0; execution stays red until 1.1–3.3 land. Scenario: all five.

## 1. Harvest (podras-o47)

- [x] 1.1 red: `tests/harvest.rs::harvest_candidates_never_decisions` — fixture repo with an orphan commit, an unlinked spec change, and a note reference to an id absent from the tracker export; assert typed kind/summary/evidence, curation and tracker bytes unchanged, focus/parked unchanged, exit 0.
- [x] 1.2 red: `tests/harvest.rs::harvest_idempotent_provenance` — two `podras harvest` runs at the same `--as-of`; assert identical candidate ordering `(kind, evidence ref, summary)`, every ref resolves (`git rev-parse --verify`, note entry present), and no repository file changes between runs.
- [x] 1.3 `src/model.rs` + `src/harvest/model.rs`: `Harvest`, `Candidate`, `CandidateKind` (declaration order = sort order), `Evidence`, `EvidenceKind` (serde `ref`); `SourceName::ALL` for table coverage tests.
- [x] 1.4 `src/harvest.rs`: replace the `NotYetImplemented` seam with a pure assembler over `&BriefInputs` and the evaluation date; no write path, no verb calls, no `bd` invocation.
- [x] 1.5 `src/harvest/{notes,commits,specs}.rs`: detectors. Move the orphan-commit and unlinked-spec-change predicates next to the tracker-linking vocabulary in `src/health/links.rs`; `src/health/git.rs` and `src/health/specs.rs` keep their single summarized signal and harvest emits one candidate per item with provenance. Reuse `SIGNAL_HORIZON_DAYS = 14` as the candidate window.
- [x] 1.6 `src/readers/git.rs`: `commit_touching(path)` and `commit_exists(token)` bounded read-only queries. `src/readers/whisper.rs`: return the latest entry body in `WhisperReading::latest_entry_body` (internal; the brief never renders it). `src/brief.rs`: `BeadsSummary::all_ids` from the full export so closed ids do not become candidates.
- [x] 1.7 `src/commands.rs` + `src/main.rs`: `podras harvest [--json] [--as-of <date>]`; `src/render/harvest.rs` human narrative grouped by kind with explicit `none`; `--json` emits `Envelope<Harvest>` with `schema_version = "harvest.v1"` and the same `sources[]`/`warnings[]` rows as the brief. Exit 0 on degraded reads, 2 on usage errors.
- [x] 1.8 Privacy/mutation test: harvest has no `--write`/`--out-dir`; note-derived summaries never appear in any committed file; `whisper_privacy` stays green.
- [x] 1.9 Docs: README command list gains `podras harvest` as discovery-not-commitment and on-demand-only; `src/verbs/init.rs` block gains the harvest rule; second `podras init` stays byte-identical.

## 2. Freshness hardening and `--as-of` (podras-7jd)

- [x] 2.1 red: `tests/freshness.rs::freshness_thresholds_declared` — read the public table, assert beads 7, git 7, openspec 14, whisper 14, curation 14 exactly once per `SourceName::ALL` entry; on-horizon fresh, +1 day stale, absent unknown; no project-wide staleness field in the envelope.
- [x] 2.2 `src/freshness.rs`: replace the five constants and the `match` with `HORIZONS: [Horizon; 5]` and a `horizon_days` lookup; document thresholds in the module doc; `evaluate` keeps v1 boundary semantics.
- [x] 2.3 `src/freshness.rs::impaired_mandatory` + `src/brief.rs` + `src/health/verdict.rs`: pass impaired mandatory sources into `layer`; a fresh human green downgrades to `amber` (`basis = human`) naming each impaired source; no new `SignalKind`, no envelope schema change.
- [x] 2.4 `tests/health.rs` + `tests/experiments/staleness.rs` (from podras-4i2): fault injection — killed tracker, beads export aged past 7 days, silently closed focus, spec edited after harvest; each surfaces in `sources[]`/signals/verdict and none renders green.
- [x] 2.5 red: `tests/determinism.rs::as_of_reproducible_render` — two `--json --as-of` runs and two `--write --as-of` runs; payload equality after removing `generated_at`, `BRIEF.json` diff only in `generated_at`, `BRIEF.md` diff only in the stamp's `generated_at` value, boundary verdicts stable, invalid date exits 2 and writes nothing.
- [x] 2.6 Docs: README states the pinned `--as-of` semantics and the threshold table's single declaration point.

## 3. Render drift gate (podras-mgd)

- [x] 3.1 red: `tests/drift_gate.rs::drift_gate_detects_stale_render` — run `scripts/guards/brief-drift.sh` against a committed render (exit 0), add a commit without re-rendering (exit non-zero, names `BRIEF.json`/`BRIEF.md`), assert both artifacts byte-identical before and after both runs.
- [x] 3.2 `src/main.rs` + `src/commands.rs` + `src/write.rs`: `brief --write --out-dir <DIR>`; `--out-dir` requires `--write`, targets an existing directory, missing/unwritable directory exits 1 with no writes, default remains the repository root.
- [x] 3.3 `scripts/guards/brief-drift.sh`: root via `git rev-parse`; no-op without renders; evaluation date from `BRIEF.json` `.data.generated_at` (or `--as-of`/`PODRAS_DRIFT_AS_OF`); `mktemp -d` + `--out-dir` regeneration; normalize `generated_at`, `source_hash`, `warnings`, `meta.author`, per-source `observed_at`/`staleness`, and the narrative sources/freshness tail; `diff -u` normalized bytes; exit 0 match, 1 drift (artifact named), 2 usage/dependency; `git status --porcelain` unchanged.
- [x] 3.4 Wiring: `justfile` `brief-drift` recipe; `lefthook.yml` commented opt-in stanza outside the `ddl:managed` blocks. Never default-on.
- [x] 3.5 Docs: README gate usage, the normalization boundary (content drift, not freshness drift), and the local-vs-CI source caveat.

## 4. Out of scope for this change (tracked, not delivered here)

- [x] 4.1 (podras-5y8) Roadmap-lens contract: gated on the external `value-prop-roadmap` synthesis and the value-prop investigation. Scenario: —.
- [x] 4.2 Harvest-driven writes: filing/closing beads issues, promoting or parking focus, committed harvest artifacts. Scenario: —.
- [x] 4.3 New health signal kinds, envelope schema changes, or a project-wide staleness field. Scenario: —.
- [x] 4.4 Org-mode render, portfolio view, and per-repo freshness-threshold configuration. Scenario: —.

## Scenario coverage ledger

| scenario | owning ticket | contract stub | test owner |
|---|---|---|---|
| `harvest_candidates_never_decisions` | podras-o47 | `harvest-candidates-never-decisions.toml` | `tests/harvest.rs` |
| `harvest_idempotent_provenance` | podras-o47 | `harvest-idempotent-provenance.toml` | `tests/harvest.rs` |
| `freshness_thresholds_declared` | podras-7jd | `freshness-thresholds-declared.toml` | `tests/freshness.rs` |
| `as_of_reproducible_render` | podras-7jd | `as-of-reproducible-render.toml` | `tests/determinism.rs` |
| `drift_gate_detects_stale_render` | podras-mgd | `drift-gate-detects-stale-render.toml` | `tests/drift_gate.rs` |
## Implementation notes (as landed)

- Harvest detectors read the beads export, whisper branch note, and bounded git
  queries directly in `src/harvest/*`; the v1 `BriefInputs`/model contract was
  left untouched so the read path and its schema do not drift. Module layout as
  landed: `harvest.rs` (contract + assembler) plus
  `harvest/{detectors,notes,git,specs,render}.rs`.
- `--out-dir` lives in `src/write.rs` as `write_renders_to`; `write_renders`
  delegates with the repository root.
- The drift gate normalises volatile fields (`generated_at`, `source_hash`,
  per-source `observed_at`/`staleness`, genesis `meta.author`) and refuses to
  run when the working tree is not clean before/after.
