# Change: Add brief v1.1 — harvest, freshness hardening, and the render drift gate

## Why

Build stages 1–3 landed as `add-brief-v1-read-path`: the deterministic
nine-section brief, the typed agent envelope with committed renders, and the
`focus` / `park` / `init` curation verbs. The read path is now proven by its own
property suite and validation experiments, so the two stages deliberately
deferred as v1.1 (harvest, freshness hardening) and the drift gate parked behind
committed renders can be specified without destabilising v1.

Three gaps remain:

- **Discovery has no landing place.** The notes, git history, and spec store
  already contain mismatches — commits no issue claims, spec changes no bead
  links, note references to items that no longer resolve. Today a human finds
  them by accident; the temptation is to have the tool file issues and promote
  focus automatically. Discovery is not commitment: harvest must surface
  candidates with provenance and change nothing.
- **Freshness is asserted, not hardened.** v1 declares per-source horizons as
  separate constants and a stale tracker can still coexist with a green human
  verdict. There is no fault-injection proof that a killed tracker, a stale
  export, or a silently closed focus cannot masquerade as healthy, and the
  `--as-of` reproducibility contract is exercised only inside the fixture
  matrix.
- **Committed renders can go stale silently.** `podras brief --write` produces
  `BRIEF.md` / `BRIEF.json`, but nothing detects a render that no longer matches
  the repository it describes. A hand-edited or forgotten render is exactly the
  stale artifact the brief exists to replace.

## What Changes

- **`podras harvest [--json] [--as-of <YYYY-MM-DD>]`** — a read-only verb that
  extracts typed candidates from whisper session notes, git history, and the
  OpenSpec change store. Each candidate is `{kind, summary, evidence[]}` where
  evidence is `{kind: git|notes, ref}`; a candidate without resolvable evidence
  is not emitted. Candidate kinds: `orphan_commit`,
  `unlinked_spec_change`, `note_reference_unresolved`. Candidates order
  deterministically by kind, then evidence ref. Harvest is idempotent, writes
  no curation state, files/closes no tracker issues, and never selects,
  replaces, or parks focus. Human narrative and `--json` envelope carry the
  same candidates. Output is on-demand only: no committed artifact, no
  `--write` / `--out-dir` surface.
- **Freshness hardening** — one declared threshold table
  (beads 7d, git 7d, openspec 14d, whisper 14d, curation 14d) becomes the only
  source of per-source horizons; every staleness verdict reads it. The health
  verdict gains a mandatory-source guard: a missing, unreadable, or stale
  beads/git reading downgrades a green verdict to amber (with the impaired
  source named), so stale inputs can never be rendered healthy. Thresholds stay
  documented in code; the envelope keeps per-source `staleness` and adds no
  project-wide field.
- **Pinned `--as-of` reproducibility** — the existing flag is pinned in tests:
  with an identical `--as-of` and unchanged inputs, semantic payloads and
  committed renders differ only in `generated_at`; freshness, resurface, and
  human-verdict boundaries do not flip between runs; an unparseable date exits
  2 and writes nothing.
- **Render drift gate** — `scripts/guards/brief-drift.sh` regenerates
  `BRIEF.md` / `BRIEF.json` into a temporary directory at the committed
  render's evaluation date (or an explicit date), normalizes volatile and
  freshness-provenance fields, diffs against the committed renders, exits
  non-zero naming the stale artifact, and never writes the working tree. The
  CLI gains `brief --write --out-dir <DIR>` so regeneration can target a temp
  directory. The gate is an opt-in lefthook/CI command, not a default blocking
  hook; repositories without committed renders are a no-op.
- **Docs** — README documents harvest, the hardened freshness contract, and the
  gate; the `podras init` AGENTS.md managed block gains the
  discovery-not-commitment rule for harvest and is refreshed idempotently.

Out of scope for this change:

- the roadmap lens and portfolio view (`podras-5y8`, gated on the external
  `value-prop-roadmap` synthesis and the external value-prop investigation)
- harvest-driven writes: filing or editing beads issues, promoting or parking
  focus, writing committed candidate artifacts
- new health signal kinds, envelope schema changes, or a project-wide staleness
  field
- org-mode render, portfolio/multi-repo render, per-repo threshold
  configuration
- modifying the deployed `openspec/specs/brief-v1/spec.md` corpus

## Impact

- Affected specs: `brief-v1` — the delta under
  `openspec/changes/add-brief-v1-1/specs/brief-v1/spec.md` adds four
  requirements and five scenarios (`harvest_candidates_never_decisions`,
  `harvest_idempotent_provenance`, `freshness_thresholds_declared`,
  `as_of_reproducible_render`, `drift_gate_detects_stale_render`). The deployed
  corpus is not modified.
- Affected code: `src/harvest.rs` (replaces the v1 seam) plus
  `src/harvest/` detectors, `src/readers/git.rs` and
  `src/readers/whisper.rs` (bounded read-only queries), `src/brief.rs`
  (`all_ids` for reference resolution), `src/main.rs`, `src/commands.rs`,
  `src/render/` (harvest narrative), `src/freshness.rs` (threshold table),
  `src/health/verdict.rs` and `src/brief.rs` (mandatory-source guard),
  `src/write.rs` (`--out-dir` target), `scripts/guards/brief-drift.sh`,
  `lefthook.yml` (opt-in stanza), `justfile` (recipe), `README.md`,
  `src/verbs/init.rs` (managed block text), and
  `.espectacular/changes/add-brief-v1-1/brief-v1/` contract stubs.
- Runtime surface: `podras harvest [--json] [--as-of <date>]`;
  `podras brief --write --out-dir <dir>` (in addition to the v1 surface). Exit
  codes stay consistent: `0` success including degraded reads, `2` CLI usage
  errors, `1` hard errors (unwritable output directory, unresolved write).
- Privacy: harvest consumes whisper note text on demand only; candidate output
  (which may quote a note line) is never written to a committed artifact. The
  brief's existing whisper read is unchanged: latest entry timestamp and
  next-action line only.
- Board: closes no tracker state by itself; `podras-o47`, `podras-7jd`, and
  `podras-mgd` map to the tasks below and to the five scenarios.