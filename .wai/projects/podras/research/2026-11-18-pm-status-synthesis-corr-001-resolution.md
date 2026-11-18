---
title: "pm-status-tool-structure investigation evaluation — CORR-001 resolution and sign-off"
report_id: "podras-pm-status-synthesis-corr-001-resolution-2026-11-18"
kind: research
generated_at: "2026-11-18"
generator: "pi agent session (podras repo, /var/home/sasha)"
provenance_standard: "corpus paths + commit refs only; no synthesis content invented here"
scope: "Evaluation of the catitas 12-provider synthesis feeding podras docs/design.md; resolves the open Rule-of-5 review finding CORR-001"
---

# pm-status-tool-structure: evaluation conclusion

## TL;DR

The investigation **passes evaluation**. The 12-provider synthesis
(`pm-status-tool-structure`) is internally honest (attribution-rich consensus,
declared contradictions T1–T5, fabrication flags, gaps section) and transfers
cleanly into `podras docs/design.md`. The one open review finding — **CORR-001:
the "8/12 beads-support" count contradicts the doc's own fabrication flags** —
is resolved by a one-line count note: openrouter's beads support is
pattern-level only (its beads specifics are fabricated), so the strict count is
**7/12**. Materiality: **low** — no design decision changes; `docs/design.md`
never cites the count.

## What was reviewed (state verified 2026-11-18)

- Synthesis: `ak/catitas/catitas/data/synthesized/pm-status-tool-structure.md`
  (a symlink into `ak/ryan/archive/catitas/data/` — one real file). 107 lines,
  12 valid members; meta failed; huggingface/perplexity/qwencloud pending
  canonical transcripts (synthesis itself notes an `--additive` re-run).
- Ledger: `ak/pinky/ledger/topics/pm-status-tool-structure.org`
  (status: synthesized) — consistent with the synthesis.
- Design record: `podras docs/design.md` — decisions 1–5 and the brief v1
  contract trace to synthesis C1–C7; open questions match T1/T3/T5 exactly.
- Prior review: a Rule-of-5 review ran in a previous session; only its
  handoff-line survives (`verified@0.45 < gate → REVIEW_REQUIRED` on the
  8/12 wording). The full review document was not recoverable — evaluated on
  the verifiable claim (see uncertainty below).

## CORR-001 — finding and verification

C2 claims beads is closest prior art in **8/12**: claude, gemini, grok, kimi,
mistral-work, openrouter, zai, chatgpt. But the same document's quality-flags
section says openrouter described a **non-real beads** (`beads orient`,
open/done/parked directories — fabricated). Counting a transcript whose beads
specifics are fabricated as "beads support" is self-contradictory. Also:
deepseek explicitly did not pick beads; qwen was not counted. Strict count:
**7/12**, or 8/12 with openrouter annotated as pattern-level only.

## Resolution

Amendment applied to the synthesis C2 (count note, references the quality
flags section), committed as **ryan `09c4f51c`** ("synthesis: pm-status CORR-001
count note — openrouter beads support is pattern-level only (strict 7/12)";
gitleaks gate green). catitas sees the fix through its `data` symlink — no
catitas commit needed. Ryan/catie/pinky unpushed-commits rule still applies:
**not pushed** (push authorized for podras only).

**Sign-off: CORR-001 closed with amendment. Verdict on the synthesis: accepted
as design input for podras.**

## What this does not change

- `docs/design.md` decisions stand as written (beads substrate is
  load-bearing regardless of 7 vs 8).
- T1 (focus cap), T3 (health authority), T5 (curation format) remain open →
  beads ticket `podras-fx1` (grill-part-2).
- Brief v1 proposal remains `podras-d2e`.

## Remaining caveats (candidates, not blockers)

- Three providers' canonical transcripts pending → optional `--additive`
  re-synthesis after `just download`; unlikely to flip any consensus.
- Portfolio/multi-repo aggregation unvalidated by the corpus → candidate
  follow-up investigation (flagged in ledger open questions).
- value-prop-roadmap job dispatched but not yet downloaded/analyzed → feeds
  the pitch/roadmap contract in `podras-fx1`.

## References

- `ak/ryan/archive/catitas/data/synthesized/pm-status-tool-structure.md` (post-`09c4f51c`)
- `ak/pinky/ledger/topics/pm-status-tool-structure.org`
- `podras docs/design.md` (brief v1 contract + build order + open questions)
- Whisper handoff `resume-podras-brief` (branch main, podras) — source of the CORR-001 verdict line

