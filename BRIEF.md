# Project status brief

## Purpose / current state

none

## Focus

none

## In flight

- podras-23g — release: five-target binary matrix + checksums per standardization §1 (release.yml) (status: open; priority: 2; age: 0d)
- podras-e67 — publish podras to crates.io so ddl install podras cargo path works (registry entry already live in ddl) (status: open; priority: 2; age: 0d)
- podras-aqf — conformance: book + llms.txt + README status block + status page + ci.yml with dogfood installs (standardization §2/§5/§7) (status: open; priority: 3; age: 0d)
- podras-eg4 — design decision: bound or page the in_flight section (unbounded today — wai renders 83 rows, dont 73); fleet sweep flagged the verbosity, spec says unbounded by declared default (status: open; priority: 3; age: 0d)

## Blocked / waiting

none

## Next / ready

- podras-23g — release: five-target binary matrix + checksums per standardization §1 (release.yml) (status: open; priority: 2; age: 0d)
- podras-e67 — publish podras to crates.io so ddl install podras cargo path works (registry entry already live in ddl) (status: open; priority: 2; age: 0d)
- podras-aqf — conformance: book + llms.txt + README status block + status page + ci.yml with dogfood installs (standardization §2/§5/§7) (status: open; priority: 3; age: 0d)
- podras-eg4 — design decision: bound or page the in_flight section (unbounded today — wai renders 83 rows, dont 73); fleet sweep flagged the verbosity, spec says unbounded by declared default (status: open; priority: 3; age: 0d)

## Parked

none

### Needs re-triage

none

## Recent

- e4d374bab0ee86e8470284460a659152e154f29e test: fleet sweep pre-release gate — podras validated on 18 charly-vibes repos (2026-11-18T21:05:54Z)
- 2d9c811d4301945c0a2f5a21c2bf4201e6a87d6b beads: file release/crates-io/conformance follow-ups from the ddl dogfood onboarding (2026-11-18T21:00:27Z)
- a785bff36896f44b08dbed8c1f881a3648b0f3d1 fix: rule-of-5 review fixes for the roadmap lens (2026-11-18T20:25:14Z)
- 7d8c12c76d3e53f4af6ea7dff3f5997c42439290 feat: roadmap lens read path (stage 6) (2026-11-18T19:38:37Z)
- f534c2a1864c7dba5efd92e4b57c98b57794c894 design: rule-of-5 fixes for the roadmap-lens contract (2026-11-18T18:08:06Z)
(19 more omitted)

## Health + confidence

Health: **amber** — basis: signals — owner: none — reason: orphan_commits: 22 commits in the 14-day window link no tracker id or open work-item path: 0a00aab0571265ef1de08dd7f67e5ec744b7af37 (fix: v1/v1.1 rule-of-5 findings — drift-gate clock+normalization, out-dir contract, whisper reader reuse, stronger experiments); 0bdc5f68c90b6838306ed7dc779182aa300fcef4 (feat: brief v1 — readers, curation, health, renders, committed write, focus/park/init (read-path change)); 2d9c811d4301945c0a2f5a21c2bf4201e6a87d6b (beads: file release/crates-io/conformance follow-ups from the ddl dogfood onboarding); 47c73fd1fdfb7fd1a1a394748fb0de2a3bee84ed (spec: brief-v1 specodelic spec (14 constraints/14 properties/curation FSM) — spk lint gate non-vacuous; espectacular config: version bump + pytest runner); 502876c0327fceb77c9a7b057b1b25912304a5c5 (beads: issue-review fixes — j13+beads dep, property ownership (msq/j13/ci3), lane+complexity labels, metadata.files+base_commit on all tickets); 53f0a96db7c2cdd216c91bfe17e77f069c991950 (readme: No podrás epigraph (naming nod to Cristian Castro)); 7d8c12c76d3e53f4af6ea7dff3f5997c42439290 (feat: roadmap lens read path (stage 6)); 9dfea4252155c9ef3f4559de0633b0d81defd3cc (spec: brief v1 read-path change overlay (proposal/design/tasks, 14 property scenarios) + spk 0.14 pin); a785bff36896f44b08dbed8c1f881a3648b0f3d1 (fix: rule-of-5 review fixes for the roadmap lens); addb644fd6313be9da8e7189f7910ea658b6b887 (feat: brief v1.1 — harvest verb, freshness hardening, drift gate; v1 review fixes); b54ed5d241597c5e4c6cc1f62f73619d11856802 (chore: close implemented tickets (19 landed; 5y8 external gate remains)); b699c86db9bd3bce6e4fa12d2955eafa17ed0771 (init: espectacular config (ah init) so ah-check gate passes); c24c876325179d5c3cd134ab9055275464f7bf15 (fix: scaffold review fixes — as-of date parser, miette help, isolated git fixtures, verified deps; design amendments); cacdb5bf08ce906112ac92d9dacf4b0c9fa028ae (chore: init wai workspace); cc6d4429ef338bd6e9c841b232bbe6e925fc7c73 (design: roadmap-lens contract from the value-prop-roadmap synthesis); d8c011023ed374c7a311a313c1ce66cdf1bb962b (init: podras repo scaffold + design record); dbbb75b7cf2700a9abd29ea4c53616e10bb550b4 (spec: archive add-brief-v1-read-path and add-brief-v1-1 into brief-v1); e4d374bab0ee86e8470284460a659152e154f29e (test: fleet sweep pre-release gate — podras validated on 18 charly-vibes repos); f3a9e821d32302f5842f265f9cdfdd8665561633 (beads: implementation board — 19 tickets wired to brief.v1 spec (d2e approval gate blocks v1 path; harvest/freshness parked behind experiments)); f534c2a1864c7dba5efd92e4b57c98b57794c894 (design: rule-of-5 fixes for the roadmap-lens contract); fc9a44ad5b57cb84f01d10e7f3a9a0d58cabaecc (Initial commit); fd62e396a352bead37fa891c14eed9d41253074f (research: pm-status synthesis evaluation — CORR-001 resolved (7/12 strict count), synthesis accepted as design input); spec_change_without_bead: openspec changes link no tracker id: `add-roadmap-lens-read-path` changed 0 days ago
- signal: orphan commits — 22 commits in the 14-day window link no tracker id or open work-item path: 0a00aab0571265ef1de08dd7f67e5ec744b7af37 (fix: v1/v1.1 rule-of-5 findings — drift-gate clock+normalization, out-dir contract, whisper reader reuse, stronger experiments); 0bdc5f68c90b6838306ed7dc779182aa300fcef4 (feat: brief v1 — readers, curation, health, renders, committed write, focus/park/init (read-path change)); 2d9c811d4301945c0a2f5a21c2bf4201e6a87d6b (beads: file release/crates-io/conformance follow-ups from the ddl dogfood onboarding); 47c73fd1fdfb7fd1a1a394748fb0de2a3bee84ed (spec: brief-v1 specodelic spec (14 constraints/14 properties/curation FSM) — spk lint gate non-vacuous; espectacular config: version bump + pytest runner); 502876c0327fceb77c9a7b057b1b25912304a5c5 (beads: issue-review fixes — j13+beads dep, property ownership (msq/j13/ci3), lane+complexity labels, metadata.files+base_commit on all tickets); 53f0a96db7c2cdd216c91bfe17e77f069c991950 (readme: No podrás epigraph (naming nod to Cristian Castro)); 7d8c12c76d3e53f4af6ea7dff3f5997c42439290 (feat: roadmap lens read path (stage 6)); 9dfea4252155c9ef3f4559de0633b0d81defd3cc (spec: brief v1 read-path change overlay (proposal/design/tasks, 14 property scenarios) + spk 0.14 pin); a785bff36896f44b08dbed8c1f881a3648b0f3d1 (fix: rule-of-5 review fixes for the roadmap lens); addb644fd6313be9da8e7189f7910ea658b6b887 (feat: brief v1.1 — harvest verb, freshness hardening, drift gate; v1 review fixes); b54ed5d241597c5e4c6cc1f62f73619d11856802 (chore: close implemented tickets (19 landed; 5y8 external gate remains)); b699c86db9bd3bce6e4fa12d2955eafa17ed0771 (init: espectacular config (ah init) so ah-check gate passes); c24c876325179d5c3cd134ab9055275464f7bf15 (fix: scaffold review fixes — as-of date parser, miette help, isolated git fixtures, verified deps; design amendments); cacdb5bf08ce906112ac92d9dacf4b0c9fa028ae (chore: init wai workspace); cc6d4429ef338bd6e9c841b232bbe6e925fc7c73 (design: roadmap-lens contract from the value-prop-roadmap synthesis); d8c011023ed374c7a311a313c1ce66cdf1bb962b (init: podras repo scaffold + design record); dbbb75b7cf2700a9abd29ea4c53616e10bb550b4 (spec: archive add-brief-v1-read-path and add-brief-v1-1 into brief-v1); e4d374bab0ee86e8470284460a659152e154f29e (test: fleet sweep pre-release gate — podras validated on 18 charly-vibes repos); f3a9e821d32302f5842f265f9cdfdd8665561633 (beads: implementation board — 19 tickets wired to brief.v1 spec (d2e approval gate blocks v1 path; harvest/freshness parked behind experiments)); f534c2a1864c7dba5efd92e4b57c98b57794c894 (design: rule-of-5 fixes for the roadmap-lens contract); fc9a44ad5b57cb84f01d10e7f3a9a0d58cabaecc (Initial commit); fd62e396a352bead37fa891c14eed9d41253074f (research: pm-status synthesis evaluation — CORR-001 resolved (7/12 strict count), synthesis accepted as design input)
- signal: spec change without bead — openspec changes link no tracker id: `add-roadmap-lens-read-path` changed 0 days ago

## Sources & freshness

| source | status | observed_at | staleness | detail |
| --- | --- | --- | --- | --- |
| beads | present | 2026-11-18T21:05:48Z | fresh | none |
| git | present | 2026-11-18T21:05:54Z | fresh | none |
| openspec | present | 2026-11-18T20:20:21Z | fresh | none |
| whisper | present | 2026-11-18T18:09:41Z | fresh | none |
| curation | present | 2026-11-18T21:12:09Z | fresh | none |

Curation: updated_at 2026-11-18T21:12:09Z; confirmed_at none.

Freshness: generated_at 2026-11-18T21:12:09Z; per-source observed_at and staleness are listed above.
