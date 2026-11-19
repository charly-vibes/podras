# Change: Fix stale stage framing left by the stage ⑦/⑧ ships

## Why

Rule-of-5 review finding CORR-001 (HIGH, escalate-to-human) and CLAR-001
(MEDIUM), tracked as podras-6gq: spec narrative still describes the roadmap
state file and the bet verbs as future work. `bet add|admit|defer|kill` (stage
⑦) and the harvest extension (stage ⑧) shipped on 2026-11-19
(`add-bet-lifecycle-verbs`, `add-harvest-roadmap-candidates`), so roadmap-v1's
Purpose claim "No verb writes it in this stage" is now false, and brief-v1's
Non-Goals bullet frames the bet verbs as "roadmap-v1 stage ⑦" — a forward
pointer to something that exists.

## What Changes

- roadmap-v1 Purpose: replace the stale "No verb writes it in this stage"
  sentence with the shipped truth — the bet verbs are the only writers of
  `.podras/roadmap.toml`; harvest only proposes advisory candidates.
- roadmap-v1 requirement "Read-only lens with an agent-facing surface": name
  the writers by verb (`bet add|admit|defer|kill`) instead of bare stage
  attribution, keeping the requirement consistent with the rewritten Purpose.
- brief-v1 Non-Goals: reword the bet-verbs bullet from stage framing to
  ownership framing — a roadmap-v1 capability, no bet mutations in the brief
  envelope.

Normative behavior is unchanged: constraints, properties, scenarios, and the
espectacular contract tomls already state the shipped truth.

## Impact

- Affected specs: roadmap-v1 (Purpose narrative + one requirement clause),
  brief-v1 (Non-Goals narrative).
- Affected code: none — narrative-only; renders and tests unaffected.
