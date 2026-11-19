# Change: Add harvest roadmap candidates — stage ⑧

## Why

Stage ⑥ gave podras a read-only roadmap lens whose warnings name stale
commitments, but the advisory never reaches the one verb agents actually run
cold: `podras harvest`. A human returning to a project sees an orphan commit
prompt yet no prompt that the identity review has lapsed or that a bet has
outlived its appetite. Harvest is derive-only by contract, so the fix is two
new candidate kinds — prompts for a human, never decisions — reusing the lens's
own warning predicates so the lens and harvest can never disagree
(`docs/design.md` decisions 4, 21, 22, 23; review fix D7).

## What Changes

- **`identity_review` candidate** — emitted iff `identity.next_review` is
  strictly earlier than the evaluation date (honouring `--as-of`). An absent
  schedule means staleness is unknown, not overdue (decision 21), so it fires
  nothing; equal or later is fresh. At most one candidate, ordered last.
- **`retire_or_reshape` candidate** — one per non-terminal bet
  (`shaping`|`committed`|`building`|`deferred`; `shipped` and `killed` exempt)
  that trips any of: `circuit_breaker_due` (evaluation − `admitted_at` >
  `appetite_weeks`×7×1.25), `bet_no_transition` (linked epic bead
  `updated_at` older than fourteen days), `bet_epic_closed_not_retired`
  (linked epic closed while the bet is active). The summary names the bet id
  and every triggering reason. Predicates are extracted from
  `src/roadmap/warnings.rs` and shared, not reimplemented.
- **No lens bound** — candidates compute over every bet in the file; the lens's
  5-bet envelope bound does not apply to harvest, because a dropped candidate
  is a lost prompt and the candidate list is already unbounded.
- **Evidence union extends** to `{kind: git|notes|roadmap, ref}`; roadmap
  entries cite `.podras/roadmap.toml`. This is a MODIFIED delta of the
  brief-v1 requirement "Harvest candidates are proposals, never decisions";
  the existing no-mutation and idempotence scenarios keep passing because a
  roadmap candidate run still writes nothing.
- **Degradation, never failure** — an absent roadmap file yields no candidates
  and no warning; a malformed or unreadable one yields zero roadmap candidates
  plus one `SourceName::Curation` warning carrying the domain code, mirroring
  the brief's degrade philosophy (decision D1), at exit 0.
- **Deterministic order** — bets by lens status rank (building, committed,
  deferred, shaping, shipped, killed), then `admitted_at`, then id; the single
  `identity_review` candidate last. Two runs at one `--as-of` emit the same
  set and order.
- **Render parity** — `--json` and the human narrative both carry the new
  kinds via the existing typed candidate flow; `serde` names are snake_case.
  `src/main.rs`, the brief read path, the envelope, and the schema dir are
  untouched: harvest stays on-demand only, with no committed artifact.

Out of scope for this change:

- any curation verb, tracker write, or roadmap.toml write from harvest; no
  auto-kill and no auto-promote (decision 23)
- roadmap-v1 Non-Goals cleanup — that section still lists the harvest
  extension as a stage-⑧ non-goal owned by the stage-⑦ sibling change; the
  cleanup happens at archive time, when both changes land
- new `SignalKind` variants, schema bumps, portfolio renders, or per-repo
  thresholds

## Impact

- Affected specs: delta `specs/brief-v1/spec.md` (MODIFIED "Harvest candidates
  are proposals, never decisions" — evidence wording only; Constraints and
  Properties rows updated with identical generator names) and delta
  `specs/roadmap-v1/spec.md` (ADDED "Harvest proposes roadmap candidates,
  never decisions" with five scenarios, one constraint pair, and six property
  rows whose generators are the new test functions).
- Affected code: `src/harvest/roadmap.rs` (new detector module),
  `src/harvest.rs` (two `CandidateKind` members, `EvidenceKind::Roadmap`,
  assembly wiring), `src/harvest/render.rs` (renders the new kinds through the
  existing `CandidateKind::ALL` loop), `src/roadmap/warnings.rs` +
  `src/roadmap.rs` (extracted shared predicates
  `circuit_breaker_overrun`/`epic_record`/`epic_idle_days`), and
  `tests/harvest_roadmap.rs` (new).
- Runtime surface: no new flags; `podras harvest [--json] [--as-of <date>]`
  gains the two candidate kinds. Exit codes unchanged.
- Privacy: candidates cite `.podras/roadmap.toml`, human-authored state with
  no whisper-derived content; the render surface is unchanged.
- Board: closes `podras-1jj` (proposal) and `podras-0k4` (implementation);
  stage ⑧ only.
