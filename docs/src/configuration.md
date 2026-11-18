# Configuration

podras keeps per-repo, git-committed state under `.podras/`. Two files, different owners.

## `.podras/curation.toml` — intent

Written by the `focus` and `park` verbs; humans may hand-edit it; agents never do (the AGENTS.md block `podras init` generates says so). Every key is optional — the scaffolded file is all comments. Dates are `YYYY-MM-DD` (RFC 3339 accepted).

```toml
purpose_lite = "Deterministic per-repo project status briefs"  # one-line purpose
max_focus = 1                # focus cap; defaults to 1
publish_whisper = false      # whisper notes in the committed render; opt-in

[health]                     # human verdict — owned by a person, never inferred
status = "green"             # green | amber | red
owner = "charly"
confirmed_at = "2026-11-18"  # verdicts expire; freshness matters
reason = "on plan for the lens"

[[focus]]
id = "podras-abc"
reason = "current bet"
focused_since = "2026-11-18"
confirmed_at = "2026-11-18"

[[parked]]
id = "podras-def"
reason = "blocked on the sync design"
resurface = "2027-01-15"     # date, or "event:<name>"
parked_at = "2026-11-18"
confirmed_at = "2026-11-18"
```

Rules the file encodes: the focus cap is N=1 by construction; a human verdict is layered above computed signals and expires via `confirmed_at`; parked items stay parked until the resurface trigger fires.

## `.podras/roadmap.toml` — the why-lens

A **human decision record**: write it yourself (or via the `bet` verbs once stage ⑦ lands). Agents never edit it. A missing file is legal — the brief carries no `roadmap` object at all; a present file with zero bets is still a present lens.

Three tables, all dates ISO-8601, all enums closed (`horizon`: `now|next|later`; `status`: `shaping|committed|building|deferred|shipped|killed`; `enforcement`: `automated|human_review`):

```toml
[identity]
path = "README.md#roadmap-state"   # link, never generated prose
last_reviewed = "2026-11-18"
next_review = "2027-05-18"         # optional; absent ⇒ review unscheduled

[[bets]]
id = "roadmap-lens"                # slug ^[a-z0-9][a-z0-9-]*$, unique
title = "Ship the roadmap lens"    # ≤ 60 chars, imperative
outcome = "A cold reader predicts the next bet from the lens alone"
horizon = "now"
appetite_weeks = 6
owner = "charly"
epic = "podras-abc"                # optional; exactly one bead id
status = "building"
admitted_at = "2026-11-18"
last_reviewed = "2026-11-18"
review_or_kill = "2026-12-30"      # required for committed/building

[[no_gos]]
id = "no-tracker-writes"
reason = "podras never mutates the tracker; harvest proposes only"
scope = "all verbs"
enforcement = "automated"
revoke_by = "never — requires a new design decision"
match_terms = ["auto-close", "auto-promote"]  # whole-token match; empty ⇒ human_review only
```

Field requirements: `[identity]` needs `path` and `last_reviewed`. Each `[[bets]]` needs `id`, `title`, `outcome`, `horizon`, `appetite_weeks` (≥ 1), `owner`, `status`, `admitted_at`, `last_reviewed`; `epic` is optional; `review_or_kill` is required for `committed`/`building`; `revive_when` is required for `deferred`. Each `[[no_gos]]` needs `id`, `reason`, `scope`, `enforcement`, `revoke_by`; `match_terms` is optional.

Degradation: a malformed or unreadable `roadmap.toml` makes `podras brief` omit the `roadmap` object and add exactly one curation warning (never a failure), while `podras roadmap` reports it as a hard error — which is what you want when the lens itself is what you asked for.

## What podras does not store

Briefs are generated artifacts, never stored independently — the only committed renders are the `--write` pairs. The portfolio view (many repos) is computed on the fly, never stored. Authorities are never duplicated: beads owns task lifecycle, openspec owns specs, git owns implemented changes, curation owns intent, the brief is derived.
