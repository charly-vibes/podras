# Change: Render the at-risk no-go in the brief's Health section

## Why

`no_go_violation_candidate` fires in `roadmap.warnings`, but the committed
`BRIEF.md` Health narrative never names it, so the at-risk boundary is invisible
on a cold read. Design decision 16 deferred this render; decision 23 keeps
roadmap warnings a distinct advisory enum, and decision 24 keeps the envelope
additive. This change lifts the deferral as a render-only concern: the brief's
markdown Health section surfaces the candidate without ever turning it into a
signal, a verdict, or a new envelope field.

## Goals / Non-Goals

- Goals: one deterministic Health line naming the at-risk no-go (subject +
  detail) whenever a candidate fires; byte-identical Health output otherwise.
- Non-Goals: no `SignalKind`, no `health.signals` change, no exit-code change,
  no envelope/schema change, no roadmap-lens render change, no enforcement.

## Decisions

- Decision: render the **first** candidate in the lens's deterministic warning
  order (code, then subject, then detail) as exactly one line. Rationale: the
  contract fixes the render at one line, and the lens already guarantees a
  total deterministic order, so "first" is reproducible. Alternatives: one line
  per candidate (breaks the one-line contract) or joining subjects onto one
  line (loses the subject/detail pairing).
- Decision: line shape `- no-go risk: <subject> — <detail>` after the signal
  lines, mirroring the existing `- signal: <kind> — <reason>` bullet style and
  staying inside the Health section. Alternative: a new section (changes the
  frozen nine-section narrative).
- Decision: read the warning through `brief.roadmap` inside the markdown
  renderer; when `roadmap` is absent the render path is untouched bytes.
  Alternative: teach the health evaluator about roadmap warnings (would couple
  the verdict layers decision 23 keeps separate).

## Risks / Trade-offs

- Two candidates render only the first → mitigated by the deterministic order
  and the roadmap lens still showing every candidate.
- The lens-separation guarantee ("brief never renders roadmap content") narrows
  to a single named line → recorded explicitly in the roadmap-v1 delta.

## Migration Plan

None: additive render behaviour on top of the existing optional `roadmap{}`.

## Open Questions

- None.
