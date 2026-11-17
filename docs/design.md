# podras design

Working design record until openspec proposals replace it. Sources: grill
sessions 2026-11-17, catitas investigation `pm-status-tool-structure`
(12-provider synthesis at `../catitas/catitas/data/synthesized/pm-status-tool-structure.md`),
pinky ledger topics `pm-status-tool-structure` (synthesized) and
`value-prop-roadmap` (in flight).

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
5 recent, 5 blockers, 5 next, 5 parked + all warnings. Human render:
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

## Open questions (grill-part-2)

- T1: focus WIP cap — hard N=1 vs ≤3 (cap-exceeded as health signal either way)
- T3: health authority — deterministic thresholds vs signals+human judgment
- T5: curation file format/location (`.toml` vs `.json`; dir name)
- Pitch/roadmap contract — pending `value-prop-roadmap` investigation:
  value-prop likely links repo identity doc (README §Objective); roadmap
  likely curated in the workspace, linking down to beads epics, with
  bet-age-vs-appetite as a health signal
- Implementation stack (Rust to match turu house style vs other)
