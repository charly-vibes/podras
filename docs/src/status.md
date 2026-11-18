# Status

**Status:** experimental — podras is pre-release: implemented through v1.1 plus the roadmap lens read path (stage ⑥); stages ⑦ (roadmap curation verbs: `bet add|admit|defer|kill`) and ⑧ (harvest extension) are not built yet.

> **§1 deviation:** prebuilt release binaries are **not yet published** (no crates.io publish, no GitHub Release). Until the first release, install via `cargo install podras` from source. Once release.yml ships the standard five-target binary matrix, this deviation is removed.

## Command status

| Command | Status | Notes |
|---|---|---|
| `podras brief` | Implemented | v1 read path: beads + git mandatory, openspec best-effort, whisper light-read, curation read-if-present; nine fixed sections; envelope with per-source freshness and explicit unknowns |
| `podras brief --json` | Implemented | Typed envelope, `schema_version: "brief.v1"`; published schema via `podras schema` |
| `podras brief --write` | Implemented | Commits `BRIEF.md`/`BRIEF.json`; differs from on-demand only in `generated_at` (C3 determinism rules apply); `--out-dir` supported |
| `podras focus` | Implemented | Select-or-replace; cap N=1 by construction, exceeding cap is a warning, never a block |
| `podras park` | Implemented | Mandatory reason + resurface trigger (date or `event:<name>`) |
| `podras init` | Implemented | Idempotent scaffold of `.podras/curation.toml` + AGENTS.md managed block via genesis `managed_block` |
| `podras harvest` | Implemented (v1.1) | Mismatch candidates only, no state changes; idempotent; freshness-hardened |
| `podras roadmap` | Implemented (stage ⑥) | Read-only lens; `.podras/roadmap.toml`; `--write` emits `ROADMAP.md`/`ROADMAP.json`; advisory warnings, exit always 0 |
| `bet` verbs (`add/admit/defer/kill`) | In progress | Stage ⑦, not built |
| Harvest extension | In progress | Stage ⑧; deep whisper extraction beyond the light-read |
| Portfolio view (multi-repo) | Mapped to specs | Computed on the fly, never stored; timing left unvalidated by the corpus |
| Org-mode render target | Mapped to specs | Parked open question |
| Single-line Health render of `no_go_violation_candidate` | Mapped to specs | Corpus C6 exception deferred; tracked for a deliberate future change (review DOC-003) |

## Dogfooding

Per the standardization dogfooding matrix, podras is pre-release, so **every cell is opt-in** until its first publish:

- `ddl` dogfoods podras locally: `BRIEF.md`/`BRIEF.json` are committed at the ddl repo root.
- No ecosystem repo runs podras in required CI yet (2026-11-18 baseline: 18/18 repos pass with podras absent).
- podras itself consumes beads, openspec, git, and whisper notes as harvest inputs; its own CI install of ecosystem tools is opt-in per the matrix row for podras.

Validation gates before "done": amnesia test (stranger + brief + 10 min vs maintainer), agent cold-start, staleness fault injection, and the three roadmap experiments (cold-return, no-go discipline, determinism).
