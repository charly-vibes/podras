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

## Open questions (parked, non-blocking)

- Roadmap-lens contract — separate future lens, after `value-prop-roadmap`
  synthesis lands (value-prop links identity doc; roadmap curated in
  workspace, linking to beads epics; bet-age-vs-appetite as health signal)
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
