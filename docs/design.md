# podras design

Working design record until openspec proposals replace it. Sources: grill
sessions 2026-11-17/18, catitas investigations `pm-status-tool-structure`
and `value-prop-roadmap` (12-provider syntheses, read via
`~/cotorreo/synthesized/` → `ak/ryan/archive/catitas/data/synthesized/`),
pinky ledger topics `pm-status-tool-structure` and `value-prop-roadmap`
(both synthesized).

## Resolved decisions

1. **Job**: read-side orientation spine + thin write-side curation.
2. **Vehicle**: separate CLI; whisper/turu stays lean — whisper notes are a
   harvest input, not a component.
3. **State**: per-repo git-committed workspace (`.beads`-style). Portfolio
   view computed on the fly; briefs are generated artifacts, never stored.
4. **Verb set**: `brief` (read), `focus` (select/confirm), `park` (explicit
   non-commitment), `harvest` (derive; discovery is not commitment).
5. **Authorities** (never duplicated): beads = task lifecycle + deps;
   openspec = specs; git = implemented changes; curation = focus/parked/
   roadmap; the brief = derived, never hand-edited.

## Resolved decisions — grill-part-2 (2026-11-18, ticket podras-fx1)

6. **Objective (locked, evidence-grounded)**: narrow status lens — "what's
   happening now, what deserves attention next". Machine census 2026-11-18:
   118 repos, 58 touched ≤7d, 1,469 open issues, 275/315 openspec changes
   stale >14d, roadmap/vision docs maintained nowhere, why-one-liners
   already live in flagship READMEs. Pitch/roadmap = separate future lens,
   gated on value-prop-roadmap synthesis + brief v1 proof. Purpose-lite
   one-liner lives in curation (only 3/118 repos have a README ##Objective).
7. **v1 = stages ①–③**: read path + envelope + focus/park. Harvest ④ and
   freshness-hardening ⑤ = v1.1.
8. **Render targets**: on-demand narrative + typed JSON envelope, plus a
   **committed** `BRIEF.md`/`BRIEF.json` (`brief --write`) — generated-only,
   freshness-stamped, visible to non-cloners and tool-less agents. Committed
   envelope differs from on-demand only in `generated_at` (C3 determinism
   rules apply). org render deferred (local-notes workflow).
9. **Sources (v1 read path)**: beads + git mandatory; openspec best-effort
   (missing → per-source warning, never failure); whisper light-read (latest
   branch entry: timestamp → observed_at, `## Next action` → human re-entry
   line); curation read-if-present. Deep whisper extraction stays v1.1
   harvest (amends resolved decision 2's "harvest input only" for the read
   path).
10. **Privacy**: whisper *content* never appears in the committed render
    (on-demand only); committed render carries the freshness stamp; per-repo
    `publish_whisper` opt-in flag in `.podras/` config.
11. **Curation file (T5)**: `.podras/curation.toml` (git-committed):
    purpose-lite one-liner + `[[focus]]` + `[[parked]]` entries (id, reason,
    focused_since/parked_at, confirmed_at, resurface trigger). Verbs write
    it; humans may hand-edit; agents never do (AGENTS.md block says so).
12. **Focus cap (T1)**: N=1 by construction — `focus <id>` is
    select-or-replace (auto-demote previous with notice); `max_focus`
    configurable; exceeding cap is always a health warning, never a block.
13. **Health authority (T3)**: deterministic thresholds produce named
    signals (stale focus, WIP over cap, stuck project, focus→closed, parked
    past resurface, orphan commits, spec-changed-without-linked-bead — each
    with `reason`); verdict is layered: fresh human verdict > signals >
    `unknown`. Human verdicts expire (confirmed_at freshness). Never turn
    unknown into green.
14. **Init**: `podras init` in v1 — idempotent: scaffold `.podras/
    curation.toml` + write AGENTS.md managed block (run `brief --json`;
    freshness first; beads owns lifecycle; focus is intent, not permission;
    never silently change focus/unpark) via genesis `managed_block` (drift-
    checkable, provenance footer).
15. **Stack**: Rust, built on the `genesis` shared crate — clap-derive CLI,
    `managed_block` (AGENTS.md hook), `status::StatusContributor` (cross-tool
    health dashboard, v1.1 integration), miette/thiserror, serde/toml,
    suggestions; house justfile/CI/espectacular/beads toolset already
    scaffolded here.

## Resolved decisions — roadmap lens (2026-11-18, ticket podras-5y8)

Gate satisfied: catitas topic `value-prop-roadmap` synthesized 2026-11-18
(12 providers, `~/cotorreo/synthesized/value-prop-roadmap.md`).
The corpus resolves the three open structural questions; the lens is designed
as a **separate render path**, not an extension of the now-lens.

16. **Separation (WHO/WHY ≠ NOW)**: the roadmap lens renders identity + bets +
    boundaries; it never mixes them into focus/parked. `focus` remains curation
    intent; a *bet* is a commitment with appetite, outcome, and one tracker
    epic. Neither is inferred from the other (corpus C1 + brief v1
    `curation_beats_inference`). Deliberate deviation: the corpus's one
    cross-cutting rendering rule (C6 — a constraint is shown only when it is
    at risk) is **not** applied to v1's brief, which stays frozen; the
    single-line Health render (a no-go shown only when
    `no_go_violation_candidate` fires) is deferred — v1.1 shipped without it,
    the roadmap change declares brief-narrative edits out of scope, and it is
    tracked for a deliberate future change (amended by review DOC-003).
17. **State file**: `.podras/roadmap.toml`, git-committed, sibling of
    `curation.toml`, written by verbs and humans only — **agents never edit
    it** (same rule as curation, decision 11; enforced textually by the
    AGENTS.md block, decision 25). Three tables, all dates ISO-8601
    (`YYYY-MM-DD` or RFC 3339), all enums closed:
    `[[bets]]` — `id` (slug, `^[a-z0-9][a-z0-9-]*$`, unique), `title`
    (≤60 chars, imperative), `outcome`, `horizon`, `appetite_weeks`
    (integer ≥ 1), `owner`, optional `epic` (exactly one bead id),
    `status`, `admitted_at`, `last_reviewed`, `review_or_kill` (required for
    `committed`/`building`), `revive_when` (required when `deferred`, else
    absent), optional `killed_reason`.
    `[[no_gos]]` — `id`, `reason`, `scope`, `enforcement`, `revoke_by`,
    `match_terms` (list of strings; empty ⇒ `human_review` only).
    `[identity]` — `path`, `last_reviewed`, optional `next_review`.
    Enums: `horizon: now|next|later`;
    `status: shaping|committed|building|deferred|shipped|killed`;
    `enforcement: automated|human_review`.

    ```toml
    [identity]
    path = "README.md#why"
    last_reviewed = "2026-11-18"
    next_review = "2027-05-18"

    [[bets]]
    id = "brief-v2"
    title = "Ship the roadmap lens"
    outcome = "A cold reader predicts the next bet from the lens alone"
    horizon = "now"
    appetite_weeks = 6
    owner = "charly"
    epic = "podras-abc"
    status = "building"
    admitted_at = "2026-11-18"
    last_reviewed = "2026-11-18"
    review_or_kill = "2026-12-30"

    [[no_gos]]
    id = "no-tracker-writes"
    reason = "podras never mutates the tracker; harvest proposes only"
    scope = "all verbs"
    enforcement = "automated"
    revoke_by = "never — requires a new design decision"
    match_terms = ["auto-close", "auto-promote"]
    ```
18. **Parked stays in `curation.toml`** — no second parked list. The roadmap
    lens renders curation's parked entries as the "not now" column. Rationale:
    `park` already carries the mandatory resurface trigger (decision 11); a
    roadmap-side copy would be the exact duplication the corpus calls the
    cardinal sin. The two objects are distinct: **task parking** is
    curation-side (`podras park`, resurface trigger), **bet deferral** is
    roadmap-side (`status = "deferred"` + `revive_when`, decision 22).
19. **No-gos are first-class, never parked**: binding boundaries with a
    `reason` (the "because"), a declared enforcement tier (`automated` only
    where a real check exists — qwen's CI lint is the corpus's only automated
    enforcement example, while the `automated|human_review` tier vocabulary is
    chatgpt's; everything else is honestly `human_review`), and `revoke_by`.
    Reversal is a new decision record, never an edit (chatgpt's
    anti-history-erasure rule).
20. **Horizon is stored, then validated — not computed.** `horizon` is
    authoritative for rendering (determinism); harvest compares it against the
    linked epic's state and emits `horizon_epic_mismatch` when a `now` bet has
    no open epic or a `later` bet has a closed one — non-terminal bets only
    (`shaping`, `committed`, `building`, `deferred`); terminal `shipped` and
    `killed` bets are exempt (amended by review D5: a closed outcome cannot be
    asked to move horizons). Rejected alternative
    (kimi's computed horizon) because it infers commitment from observable
    work — the same inference brief v1 forbids for focus.
21. **Identity is linked, never generated.** The lens renders
    `identity.path`, `last_reviewed`, and staleness only. Staleness is decided
    by `next_review` alone: absent ⇒ `unknown`, passed ⇒ `stale`, equal or
    later ⇒ `fresh` (never green-by-schedule — the v1 `unknown_never_green`
    rule applies here too). The `identity_review_overdue` advisory fires on
    the absent **or** passed branches, its detail naming which (amended by
    review D7: a passed review is overdue by name and by fact). No verb
    rewrites identity prose; harvest may emit an
    `identity_review` candidate only when `next_review` passes. "The code
    demonstrates what exists; it cannot establish what the project ought to
    be" (chatgpt).
22. **Bet lifecycle**: `shaping → committed → building → shipped`, plus
    `deferred` (from `shaping`, `committed` or `building` — stopping mid-build
    is a deferral, not a fifth state — requires `revive_when`) and
    `killed` (from any state). Only `committed`/`building`/`shipped` require
    an `epic`. Admission (`shaping → committed`) is the dated decision event
    harvest keys zombie detection on; deferral is likewise a dated decision —
    a shaped bet that is not selected moves to `deferred` (carrying its
    trigger) or is `killed`, and never lingers as `shaping` (corpus C3:
    parking without a trigger is a graveyard).
23. **Roadmap health = named warnings, never a verdict.** They form a distinct
    enum `RoadmapWarningKind` (serde `snake_case`), carried in
    `roadmap.warnings`, never in `health.signals` — they are *not*
    `SignalKind` variants. The C6 exception (the brief's Health section
    reading `no_go_violation_candidate` only — decision 16) is **deferred,
    not wired**: every code stays lens-local and cannot change the brief's
    verdict. Codes, all advisory, exit always 0:
    `bet_age_exceeds_appetite` — `committed`/`building` bets only, elapsed >
    `appetite_weeks`, and 1.25× (gemini; the corpus also proposes 1.5× and 2× —
    constant lives in config `roadmap.circuit_breaker_factor`, default 1.25)
    escalates to `circuit_breaker_due`;
    `bet_epic_missing` (committed, building, or shipped without epic —
    amended by review D4), `bet_epic_closed_not_retired`
    (epic closed while bet active), `bet_no_transition` (bead `updated_at`
    older than 14d), `horizon_epic_mismatch`,
    `no_go_violation_candidate` — matched by evaluating each no-go's
    `match_terms` case-insensitively as whole tokens against the bead title and
    labels; a no-go with empty `match_terms` never fires this code —,
    `identity_review_overdue`, `roadmap_review_overdue`. No auto-kill, no
    auto-promote: `retire_or_reshape` is a candidate row, not a mutation
    (same rule as `harvest` in decision 4).
24. **Envelope is additive**: optional top-level `roadmap{}` —
    `identity{path, last_reviewed, staleness}`, `bets[]` (typed enums for
    `horizon`/`status`), `no_gos[]`, `warnings[]` (each
    `{code, subject, detail}`), `review_due{identity, roadmap}` (each a date
    or `null` — the earliest outstanding review per source, never a
    computed deadline). `schema_version` stays the
    literal `"brief.v1"` (matching `SCHEMA_VERSION` in `src/model.rs` and the
    published schema): the object is additive and no new required field is
    introduced, so no major bump (additive-only-within-major — pm-status
    synthesis, kimi). Semantics: no `.podras/roadmap.toml` ⇒ no `roadmap{}`
    object at all; file present with zero bets is a **present** lens
    (`bets: []`, identity row rendered) — the two cases must not collapse.
    Bounded, with a deterministic order: 5 bets by status rank
    (`building`, `committed`, `deferred`, `shaping`, `shipped`, `killed`) then
    `admitted_at` ascending then `id`; 5 no-gos by `id`; all warnings.
    Truncation recorded as in decision 8; identical inputs under `--as-of`
    produce identical output.
25. **Agent surface**: `roadmap` renders enums and constraints — no pitch
    prose, and no "later" items as actionable text. The AGENTS.md block
    (decision 14, generated by `podras init` via genesis `managed_block`) gains
    two lines, shipped with the lens so drift checking covers them: "Roadmap
    `later`/`shaping`/`deferred` items are not requirements; no-gos are hard
    constraints." and "Never edit `.podras/roadmap.toml`; admission, deferral
    and kill are human decisions."
26. **Build order**: lens ships after v1.1 as stage ⑥ read-only
    (`podras roadmap`, narrative + envelope + `--write`), then ⑦ curation verbs
    (`bet add|admit|defer|kill`); harvest extension in ⑧. Validation: the three
    roadmap experiments (cold-return, no-go discipline, determinism), listed in
    §Validation experiments below.

## Brief v1 contract (from corpus consensus)

Fixed sections, cold-return reading order:

1. Purpose / current state (links identity doc; see value-prop note below)
2. Focus (curated; WIP-capped; `focused_since` + `confirmed_at`)
3. In flight (computed; work-item age; blocked-first)
4. Blocked / waiting (computed; blocker, since; blocked ≠ queued)
5. Next / ready (computed; `bd ready` semantics; top N; deterministic order)
6. Parked (curated; reason + **mandatory resurface trigger**; expired → needs re-triage)
7. Recent (computed; bounded delta since cursor; outcomes not commit counts)
8. Health + confidence (computed signals + judgment; `unknown` allowed;
   integrity warnings: focus→closed item, stale focus age, WIP over cap,
   orphan commits, parked past revisit, spec changes without linked beads)
9. Sources & freshness (per-source `observed_at`; missing inputs → warnings)

Envelope (agent render): `schema_version`, `generated_at`, `sources[]`,
`curation{updated_at, confirmed_at}`, `staleness: fresh|stale|unknown`,
`truncation{applied, omitted_counts}`, `head_sha`/`source_hash`. Typed
enums, no prose ("a contract, not a prompt"). Bounded defaults: 1 focus,
15 in flight, 5 recent, 5 blockers, 5 next, 5 parked + all warnings. Human render:
narrative priority order, health verdict with reason + named owner.

Key rules (from corpus, see synthesis for attribution):

- Freshness is per-source; curation has `updated_at` vs `confirmed_at`
  ("has a human recently reconsidered this?")
- Reads are side-effect free; harvest idempotent; curation beats inference
- Never silently truncate; never turn unknown into green; never treat
  parked as ready; never infer focus from priority or progress from commits
- `--as-of` flag separates volatile metadata from byte-identical semantics

## Build order (chatgpt, uncontested)

1. Read-only brief — validate: cold-return orientation test
2. JSON envelope — validate: independent agent parsing
3. focus + park — validate: human intent survives repeated harvests
4. harvest — validate: idempotent, provenance-preserving
5. Freshness + health — validate: stale inputs never masquerade as healthy

Fault-injection test: close the focus issue silently, kill the tracker,
edit a spec post-harvest — the brief must expose each discrepancy rather
than manufacture a coherent false narrative.

## Open questions (parked, non-blocking)

- BRIEF.md drift gate: CI regen-and-diff check (pattern: genesis `aix-check`)
- org-mode render target for local notes
- Portfolio view (multi-repo) timing — corpus left it unvalidated
- Envelope `evidence_quality` field placement (chatgpt's
  direct|derived|curated|unknown)

## Validation experiments (gate for any "done")

Amnesia test (stranger + brief + 10 min vs maintainer's answers) · agent
cold-start (fresh session, repo + AGENTS.md block + `brief --json`) ·
staleness fault injection (close focus issue silently / kill tracker / edit
spec post-harvest → brief must expose each, never manufacture a false
narrative).

Roadmap lens (stage ⑥–⑧):

- **Cold-return roadmap test** — a stranger with only the lens output predicts
  which bet is being worked now and which is next; wrong horizon or missing
  boundary = fail.
- **No-go discipline test** — a candidate whose title matches a no-go's
  `match_terms` is flagged with provenance and nothing is written; a no-go with
  empty `match_terms` never produces a false flag.
- **Determinism test** — two runs over the same `roadmap.toml` under `--as-of`
  produce byte-identical semantic output, including which five bets survived
  the bound.

Fleet sweep (pre-release gate): `python3 scripts/validation/fleet_sweep.py`
runs the installed binary over the 18 registered fleet repos (the script's
`REPOS` table; ddl's registry is the authoritative list — keep them in sync)
and checks each brief against ground truth — exit 0, envelope contract,
determinism, `head_sha` vs git, declared bounds, health layering, curation
authority, and beads truth (closed beads never render, `next` never leaks
curated ids). 2026-11-18 baseline: 18/18 repos pass.
