## MODIFIED Requirements

### Requirement: Roadmap health is named warnings, never a verdict

Warnings SHALL be `RoadmapWarningKind` values in `roadmap.warnings`, never
`SignalKind` variants in `health.signals`; the verdict SHALL NOT change.
`bet_epic_missing` SHALL cover `committed`, `building`, and `shipped` bets;
`horizon_epic_mismatch` SHALL apply only to non-terminal bets (`shaping`,
`committed`, `building`, `deferred`). Warnings SHALL sort by code, then subject,
SHALL never name a bet dropped by the bound, and SHALL stay advisory: exit 0, no
auto-kill or auto-promote. The brief's markdown Health section MAY surface the
first `no_go_violation_candidate` in warning order as its single
`- no-go risk: <subject> — <detail>` line; that line SHALL stay advisory, never
become a signal or verdict, and no warning SHALL reach `health.signals`.

#### Scenario: roadmap_warnings_not_verdict

- **WHEN** small fixture repositories, each with at most five bets so no bet is truncated, respectively hold a building bet past `appetite_weeks` times 1.25, another building bet past `appetite_weeks` but below that factor, a committed bet without an epic, a shipped bet without an epic, an active bet whose epic is closed, an active bet whose epic has not changed for 15 days, a `now` bet with no open epic, a `shipped` bet whose epic sits under `later`, an active bet whose `review_or_kill` has passed, and an identity whose `next_review` has passed
- **THEN** each repository's corresponding code appears in `roadmap.warnings`, the shipped bet with a mismatched horizon fires no `horizon_epic_mismatch`, `health.signals` carries no roadmap code and the verdict is unchanged by them, every command exits 0, and no roadmap, curation, or tracker byte changes
- **VERIFIES** [[roadmap.v1.roadmap_warnings_not_verdict]]

#### Scenario: promoting_a_warning_to_the_verdict_is_a_violation

- **WHEN** any roadmap warning fires
- **THEN** `health.signals` keeps its v1 members and the verdict is unchanged — promoting a roadmap warning into the health verdict is a layering violation, and the exit stays 0
- **VERIFIES** [[roadmap.v1.roadmap_warnings_not_verdict]]

#### Scenario: health_no_go_line

- **WHEN** `podras brief --as-of 2026-11-18` runs on a fixture repository whose roadmap file declares a no-go with a match term matching an open bead's title
- **THEN** the brief's Health section contains exactly one advisory `- no-go risk: <subject> — <detail>` line naming that candidate, `health.signals` stays free of roadmap codes, the exit code stays 0, and no roadmap, curation, or tracker byte changes
- **VERIFIES** [[roadmap.v1.roadmap_warnings_not_verdict]]
