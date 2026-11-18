# in_flight: bound or page? — evidence and options analysis

- **Ticket**: podras-eg4 ("design decision: bound or page the in_flight section")
- **Date**: 2026-11-18
- **Status**: analysis for ratification. PODRAS rules (docs/design.md §Resolved decisions,
  brief-v1 spec) change brief narrative and specs only via a deliberate openspec
  change; this document is input to that change, not the change itself. **The
  decision belongs to the human.**
- **Method note**: all numbers below are read from local committed renders
  (`~/para/areas/dev/gh/charly/<repo>/BRIEF.json`, `as_of = 2026-11-18`, the same
  18-repo fleet the sweep validates) and from podras source at the current HEAD.
  Nothing was invented and no repo outside podras was modified.

## Evidence

### 1. Real in_flight row counts across the fleet

`in_flight` is the only computed queue without a declared bound. Committed
envelopes, as-of 2026-11-18:

| repo | in_flight | next | blocked | in_flight rows ≥30d old |
|---|---|---|---|---|
| wai | 83 | 5 | 4 | 52 (63%, max 189d) |
| dont | 73 | 5 | 5 | 69 (95%, median age 191d, max 197d) |
| specodelic | 52 | 5 | 5 | 22 |
| fabbro | 29 | 5 | 4 | 27 (median age 293d) |
| dulce-de-leche | 22 | 5 | 1 | 8 |
| wild | 22 | 5 | 2 | 14 |
| genesis | 19 | 5 | 1 | 16 |
| testaruda | 18 | 5 | 1 | 17 |
| incitaciones | 14 | 5 | 0 | 10 |
| bajan | 12 | 3 | 3 | 4 |
| pretender | 8 | 5 | 1 | 3 |
| funkalization | 5 | 4 | 1 | 1 |
| gently | 5 | 4 | 1 | 1 |
| vampiro | 5 | 4 | 1 | 0 |
| podras | 4 | 4 | 0 | 0 |
| whisper | 4 | 3 | 1 | 1 |
| espectacular | 1 | 1 | 0 | 0 |
| sad | 0 | 0 | 0 | 0 |

Fleet summary: n = 18, total 376 in_flight rows, min 0, **median 13**, mean 20.9,
max 83. **10/18 repos exceed 10 rows; 7/18 exceed 15.** 245/376 rows (65%) are
≥30 days old. wai carries 11 priority-1 rows in_flight; dont carries 4.

This confirms the sweep flag: the verbosity is real, but it is a tail problem —
the fleet median repo (13 rows) is fine; wai/dont/specodelic/fabbro are
outliers, and their tails are dominated by months-old open items (dont: 95%),
i.e. triage debt, not "work happening now".

### 2. What the envelope says today

Every committed envelope in the fleet reports `truncation.applied = true` (from
`recent`, `next`, and in some repos `blocked`), and **no envelope carries an
`in_flight` key in `omitted_counts`** — correct, since nothing is omitted, but
it means the truncation contract is silent about the one section that can be
arbitrarily large.

### 3. What the code does today

- `src/model.rs:551-554`: `in_flight: Vec<WorkItem>` — unbounded by construction.
- `src/brief/ordering.rs:66-79` (`apply_bounds`): truncates focus/recent/blocked/
  next/parked only; in_flight passes through whole.
- `src/brief/ordering.rs:22-32` (`sort_work_items`): priority ascending (None
  last), then id ascending — a **total, deterministic order**. This matters for
  every option below: any prefix slice of in_flight is a deterministic,
  `--as-of`-stable slice.
- `src/render/markdown.rs:34`: the narrative renders `In flight` with
  `omitted = None`, while focus/blocked/next/parked/recent all pass
  `omitted(brief, …)` and print "(N more omitted)" (`push_omitted`, :244). The
  machinery for a bounded narrative annotation already exists and is simply not
  wired for this section.
- `schema/brief-envelope.schema.json` → `$defs.Truncation.omitted_counts` is an
  **open map** (`additionalProperties: uint32`). An `in_flight` key requires
  **no schema change** and `schema_version` stays `"brief.v1"`.
- `scripts/validation/fleet_sweep.py:24-28`: the sweep's `BOUNDS` dict omits
  in_flight and comments it "intentionally unbounded per brief.v1
  bounded_defaults (focus/recent/blockers/next/parked only)"; the sweep notes
  print the in_flight count per repo — this is where the verbosity was flagged.

### 4. What the spec says today

- `bounded_defaults` invariant: "1 focus, 5 recent, 5 blockers, 5 next,
  5 parked — plus all warnings unbounded; bounds are declared by the artifact
  itself." **in_flight is simply absent from the list.**
- `never_silently_truncate` invariant: "unbounded is the exception, never the
  default lie" — warnings are the declared exception; in_flight is an
  *undeclared* one.
- Requirement "Bounded sections with declared truncation" + scenarios
  `bounds_declared` / `truncation_flagged` (fixture
  `.espectacular/brief-v1/truncation-flagged.toml`): same five sections.
- Requirement "Fixed-section brief read path": the narrative and the envelope
  SHALL carry the same semantic content — whatever bound policy is chosen
  applies to both BRIEF.md and BRIEF.json alike.
- Requirement "Reproducible as-of evaluation" / "Committed render drift gate":
  committed renders must be regenerable and byte-diffable; nothing may depend
  on render-time flags.

Nuance worth stating plainly: the status quo is often described as "unbounded
per spec", but the spec never *declares* in_flight unbounded — the bound list
just doesn't mention it, and `never_silently_truncate`'s own phrasing ("the
exception") is violated in spirit by an undeclared unbounded section. Even
option A below therefore arguably needs a one-line spec edit to be honest.

## Options

Dimensions per ticket: behavior, envelope impact (truncation-flag semantics),
determinism impact (as-of reproducibility), consumer impact (agents reading
BRIEF.json), spec change required.

### A. Keep unbounded, declare it

- **Behavior**: all open, unparked, unfocused, non-ready, non-blocked items
  render in both narrative and envelope. wai's BRIEF.md keeps 83 rows; BRIEF.json
  keeps growing with the tracker.
- **Envelope impact**: none today; to be principled, `bounded_defaults` gains an
  explicit "in_flight unbounded" clause (the one-line spec edit above).
- **Determinism**: trivially preserved (full list, total order).
- **Consumer impact**: agents get the complete census every render — nothing
  hidden, but wai/dont agents parse 83/73 rows to find "what's happening now",
  and 65% of those rows are ≥30d stale. The brief's cold-return purpose test
  (amnesia test: stranger + brief + 10 min) degrades on exactly the repos that
  need orientation most.
- **Spec change**: none in code; optionally a one-line invariant amendment.
- **Strongest argument for**: in_flight is the only *complete* computed census
  in the envelope; bounding it converts visible triage debt into a count, and
  dont's 95%-stale 73-row tail is arguably the truest fact about that repo.

### B. Hard bound + truncation-flag reuse (recommended)

- **Behavior**: a declared `IN_FLIGHT_BOUND` (default 15) slices the existing
  priority-then-id order; narrative renders the slice plus the existing
  "(N more omitted)" annotation; envelope records
  `omitted_counts.in_flight = total − 15`.
- **Envelope impact**: reuses the existing contract untouched —
  `truncation.applied` semantics, `omitted_counts` open map, `bounds_declared`
  and `truncation_flagged` properties all already cover "a bound removed
  items"; the change adds a section to an existing mechanism. No schema change,
  `schema_version` stays `"brief.v1"`.
- **Determinism**: prefix of a total order — identical inputs under the same
  `--as-of` give an identical slice; committed renders stay flag-free, so the
  drift gate (`scripts/guards/brief-drift.sh`) is unaffected. Note the sort has
  no date key, so no new `--as-of` boundary is introduced.
- **Consumer impact**: agents read ≤15 rows plus an honest omitted count —
  `omitted_counts.in_flight = 68` says "this repo has 78 in-flight items" more
  legibly than 78 rows do. Agents needing the full inventory go to the tracker
  (`bd list`), which is the declared lifecycle authority; the brief stays an
  orientation lens. Consistency argument: this is exactly the contract `next`
  already honors (wai omits 74 ready items today).
- **Spec change** (one openspec change): amend the `bounded_defaults` invariant
  text; extend the "Bounded sections with declared truncation" requirement;
  extend `bounds_declared` and `truncation_flagged` scenarios with an
  in_flight fixture variant; code follows (`model.rs` bound constant,
  `ordering.rs::apply_bounds`, `markdown.rs:34` wiring `omitted()`,
  `fleet_sweep.py` BOUNDS).
- **Honest cost, stated**: the prefix cut falls *inside* a priority class by
  id-lexicographic order, which is deterministic but arbitrary with respect to
  activity — e.g. a bound of 10 on wai keeps 10 of its 11 priority-1 rows and
  drops one p1 by id. A bound of **15** avoids that failure mode on today's
  fleet: it keeps the entire priority-1 class for the worst offender (wai: 11
  p1), leaves the median repo (13) fully rendered, and truncates only the 7
  genuine outliers. A bound of 5 (symmetric with other queues) would truncate
  10/18 repos and drop six of wai's p1 rows; 10 truncates 9/18 and drops one.
  Also: the tail is where stale debt lives, so bounding hides the individual
  undead items behind a count; if ratifiers want a named health advisory
  (e.g. an `in_flight_over_bound` / stale-WIP signal) to keep the debt loud,
  that is a deliberate addition to consider in the same change — not bundled
  here.

### C. Pager-style slicing

- **Behavior**: on-demand `podras brief --in-flight-page N [--page-size M]`
  renders one slice; the committed render pins a fixed policy (page 1 of a
  fixed size, or full).
- **Envelope impact**: new additive optional fields (e.g.
  `in_flight_page{page, page_size, total}`) — additive-within-major precedent
  exists (design.md decision 24), so `schema_version` can stay `"brief.v1"`,
  but this is a new surface to document, lint, and test.
- **Determinism**: deterministic *per flag value*; the committed render must
  stay flag-free to remain regenerable and drift-gate-clean — so for the
  committed artifact, C degenerates to B (a fixed slice) with extra machinery.
- **Consumer impact**: agents with CLI access could page through in_flight; but
  the committed render's stated audience (design.md decision 8: "visible to
  non-cloners and tool-less agents") cannot page at all — they see whatever
  fixed slice the committed render chose, i.e. B's outcome. Cloners with
  tooling who want the full list can already run `bd list` against the
  authoritative tracker. C buys paging nobody demonstrably needs, at the cost
  of two flags, schema fields, and fixture surface.
- **Spec change**: B's list *plus* envelope-field and CLI-flag contract rows —
  strictly larger.

## Comparison

| dimension | A unbounded | B bound 15 | C pager |
|---|---|---|---|
| narrative rows (wai) | 83 | 15 + "(68 more omitted)" | committed: fixed slice; on-demand: paged |
| envelope honesty | no signal about section size | `omitted_counts.in_flight` — uniform with all queues | new paging fields |
| schema change | none | none (open map) | additive fields |
| determinism / drift gate | trivially fine | prefix of total order; fine | fine only if committed render is flag-free |
| tool-less agent (forge UI) | full 83-row dump | bounded + count | fixed slice only — no paging possible |
| full inventory available via | the brief itself | tracker (`bd list`), authority unchanged | tracker + paging flag |
| spec change | optional one line | invariant + 1 requirement + 2 scenarios + fixture | B's list + paging contract |
| hides stale-debt items? | never | behind an honest count | behind a slice |

## Recommendation

**Option B: bound in_flight with the declared default of 15 and reuse the
existing truncation contract.**

Rationale, in order of weight:

1. **The spec's own philosophy demands it.** `bounded_defaults` bounds every
   computed queue; `never_silently_truncate` calls unbounded "the exception".
   in_flight is currently an undeclared exception — the status quo is a spec
   gap wearing the costume of a decision.
2. **Zero-cost honesty.** The `omitted_counts` map, the `bounds_declared` /
   `truncation_flagged` properties, and the narrative "(N more omitted)"
   machinery all exist; the change adds a section to existing mechanisms, with
   no schema or version impact.
3. **The brief is an orientation lens, not an inventory.** Its own purpose test
   (cold return in 10 minutes) fails on wai/dont today, and the data shows why:
   65% of fleet in_flight rows are ≥30d old. A bounded slice plus an omitted
   count serves the reader better than 73 mostly-dead rows; the tracker remains
   the full-list authority, which is the authority structure the corpus already
   declares.
4. **Determinism is preserved by construction.** The slice is a prefix of the
   declared priority-then-id total order; committed renders stay flag-free and
   drift-gate-clean; `--as-of` gains no new boundary.
5. **Nothing is foreclosed.** If agents later demonstrate a paging need, C's
   on-demand flag is additive on top of B; the reverse migration is not.

The bound number (15 recommended — keeps the median repo and the worst
offender's full priority-1 class intact; 5 or 10 are the tighter alternatives
with the within-priority id-cut caveat above) is a ratification detail for the
openspec change, not part of this recommendation's mechanism.

Known cost accepted: bounding trades per-item visibility of the stale tail for
a count. If the human wants the tail to stay loud, pair B with a health
advisory decision in the ratifying change.

## Ratification

This is **analysis, not a decision**. Per podras rules, any of options A–C
requires a deliberate openspec change (spec edits and brief narrative changes
are change-gated; roadmap/behavior admission is human). The recommended path if
ratified: one openspec change amending `bounded_defaults`, the "Bounded
sections with declared truncation" requirement, the `bounds_declared` /
`truncation_flagged` scenarios and fixture, plus the four code touch points
listed under option B.
