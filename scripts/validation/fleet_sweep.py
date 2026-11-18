#!/usr/bin/env python3
"""Pre-release gate: run the installed podras across the charly-vibes fleet
(the 18 registered repos in REPOS below — ddl's registry is the authoritative
list; keep the two in sync) and check its output against ground truth (git,
beads export, curation).

Validates per repo: exit 0 · envelope contract (schema_version, five declared
source rows in fixed order, row status vs filesystem) · determinism (two runs
identical beyond generated_at) · head_sha vs git · declared bounds · health
layering (green requires basis=human) · curation authority (focus/parked only
from curation.toml, expired parks in needs_retriage) · beads truth (queues
never render closed beads, next never leaks curated ids, missing export warns).
`in_flight` is intentionally unbounded (brief.v1 bounded_defaults).

Usage: cargo install --path . && python3 scripts/validation/fleet_sweep.py
Exits non-zero when any repo fails. Local checkouts expected under
~/para/areas/dev/gh/charly/<name>.
"""
"""Fleet validation sweep: run the installed podras across the charly-vibes
repos and check its output against ground truth (git, beads export, curation)."""
import json, subprocess, sys, os, re, datetime, tomllib

BASE = "/var/home/sasha/para/areas/dev/gh/charly"
AS_OF = "2026-11-18"
REPOS = {
    "wai": "wai", "testaruda": "testaruda", "dont": "dont", "pretender": "pretender",
    "vampiro": "vampiro", "specodelic": "specodelic", "incitaciones": "incitaciones",
    "ah": "espectacular", "turu": "whisper", "funkalization": "funkalization",
    "genesis": "genesis", "ddl": "dulce-de-leche", "bajan": "bajan", "wild": "wild",
    "sad": "sad", "gently": "gently", "fabbro": "fabbro", "podras": "podras",
}
SOURCE_ORDER = ["beads", "git", "openspec", "whisper", "curation"]
# in_flight is intentionally unbounded per brief.v1 bounded_defaults (focus/recent/blockers/next/parked only)
BOUNDS = {"focus": 1, "blocked": 5, "next": 5, "parked": 5, "recent": 5}

def run_brief(repo):
    p = subprocess.run(["podras", "brief", "--json", "--as-of", AS_OF],
                       cwd=repo, capture_output=True, text=True, timeout=120)
    return p

def git(repo, *args):
    return subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True).stdout.strip()

def parse_date(raw):
    if not raw: return None
    m = re.match(r"(\d{4})-(\d{2})-(\d{2})", raw.strip())
    return m.group(0) if m else None

def check(repo_name, repo):
    fails, notes = [], []
    if not os.path.isdir(os.path.join(repo, ".git")):
        return ["not a git repository"], []
    p1 = run_brief(repo)
    if p1.returncode != 0:
        fails.append(f"exit={p1.returncode}: {p1.stderr.strip()[:200]}")
        return fails, notes
    try:
        env = json.loads(p1.stdout)
    except Exception as e:
        return [f"stdout is not JSON: {e}"], []
    d = env.get("data", {})
    if not env.get("ok"): fails.append("envelope ok != true")
    if d.get("schema_version") != "brief.v1": fails.append("schema_version != brief.v1")

    # determinism: two runs identical sans generated_at
    p2 = run_brief(repo)
    d2 = json.loads(p2.stdout).get("data", {})
    a, b = dict(d), dict(d2)
    a.pop("generated_at", None); b.pop("generated_at", None)
    if a != b: fails.append("nondeterministic payload (two runs differ beyond generated_at)")

    # sources: five declared rows, fixed order, status matches filesystem
    rows = d.get("sources", [])
    if [r["name"] for r in rows] != SOURCE_ORDER:
        fails.append(f"source rows {[r['name'] for r in rows]} != {SOURCE_ORDER}")
    fs_state = {
        "beads": os.path.exists(os.path.join(repo, ".beads/issues.jsonl")),
        "git": True,
        "openspec": os.path.isdir(os.path.join(repo, "openspec")),
        "whisper": os.path.isdir(os.path.join(repo, ".whisper")),
        "curation": os.path.exists(os.path.join(repo, ".podras/curation.toml")),
    }
    for r in rows:
        want = "present" if fs_state[r["name"]] else "missing"
        if r["status"] != want:
            fails.append(f"source {r['name']} status {r['status']} != {want}")
        if r["status"] == "missing" and r["staleness"] != "unknown":
            fails.append(f"missing source {r['name']} staleness {r['staleness']} != unknown")

    # head sha matches git
    head = git(repo, "rev-parse", "HEAD")
    if d.get("head_sha") != head:
        fails.append(f"head_sha {d.get('head_sha')} != {head}")

    # bounds
    for section, bound in BOUNDS.items():
        n = len(d.get(section, []))
        if n > bound: fails.append(f"{section} has {n} > bound {bound}")

    # health layering
    h = d.get("health", {})
    if h.get("status") == "green" and h.get("basis") != "human":
        fails.append(f"green without human basis ({h.get('basis')})")
    if h.get("status") not in ("green", "amber", "red", "unknown"):
        fails.append(f"health status {h.get('status')} not in enum")

    # curation authority
    cur_path = os.path.join(repo, ".podras/curation.toml")
    curation = None
    if os.path.exists(cur_path):
        try: curation = tomllib.load(open(cur_path, "rb"))
        except Exception as e: notes.append(f"curation unparsable: {e}")
    cur_focus = {f["id"] for f in (curation or {}).get("focus", [])}
    cur_parked = {p["id"]: p for p in (curation or {}).get("parked", [])}
    focus_ids = {i["id"] for i in d.get("focus", [])}
    parked_ids = {i["id"] for i in d.get("parked", [])}
    if focus_ids - cur_focus: fails.append(f"focus not from curation: {focus_ids - cur_focus}")
    if parked_ids - set(cur_parked): fails.append(f"parked not from curation: {parked_ids - set(cur_parked)}")
    if not curation and (focus_ids or parked_ids): fails.append("curated rows without a curation file")
    # expired parks must be in needs_retriage, not parked
    as_of = datetime.date.fromisoformat(AS_OF)
    expired = {pid for pid, p in cur_parked.items()
               if (parse_date(p.get("resurface")) or "9999") <= AS_OF}
    retriage_ids = {i["id"] for i in d.get("needs_retriage", [])}
    if expired & parked_ids: fails.append(f"expired parks still parked: {expired & parked_ids}")
    for pid in expired:
        if pid not in retriage_ids: fails.append(f"expired park {pid} missing from needs_retriage")
    for pid in retriage_ids - expired:
        fails.append(f"needs_retriage {pid} trigger not expired")

    # beads ground truth
    beads_path = os.path.join(repo, ".beads/issues.jsonl")
    bead_status = {}
    if os.path.exists(beads_path):
        for line in open(beads_path, encoding="utf-8", errors="replace"):
            line = line.strip()
            if not line: continue
            try: row = json.loads(line)
            except Exception: continue
            if row.get("_type") == "issue" and row.get("id"):
                bead_status[row["id"]] = row.get("status") or ""
        open_ids = {i for i, s in bead_status.items() if s.lower() not in ("closed", "done")}
        computed = set()
        for section in ("in_flight", "blocked", "next"):
            for item in d.get(section, []):
                computed.add(item["id"])
                if item["id"] not in open_ids:
                    fails.append(f"{section} contains non-open bead {item['id']}")
        # every open, unparked, unfocused bead is somewhere or absent is fine;
        # but a closed bead must never appear in computed queues
        for section in ("in_flight", "blocked", "next"):
            closed = [i["id"] for i in d.get(section, []) if bead_status.get(i["id"], "").lower() in ("closed", "done")]
            if closed: fails.append(f"{section} renders closed beads: {closed}")
        # next excludes parked and focus
        leak = {i["id"] for i in d.get("next", [])} & (parked_ids | focus_ids)
        if leak: fails.append(f"next leaks curated ids: {leak}")
    else:
        codes = [w["code"] for w in d.get("warnings", [])]
        if "beads_missing" not in codes:
            fails.append(f"missing beads export without beads_missing warning (have {codes})")

    notes.append(f"health={h.get('status')}/{h.get('basis')} signals={len(h.get('signals', []))} "
                 f"warnings={len(d.get('warnings', []))} beads={len(bead_status)} "
                 f"in_flight={len(d.get('in_flight', []))} next={len(d.get('next', []))} "
                 f"blocked={len(d.get('blocked', []))} parked={len(parked_ids)} focus={len(focus_ids)}")
    return fails, notes

results = {}
for name, rel in REPOS.items():
    repo = os.path.join(BASE, rel)
    if not os.path.isdir(repo):
        results[name] = (["missing local checkout"], [])
        continue
    try:
        results[name] = check(name, repo)
    except Exception as e:
        results[name] = ([f"sweep error: {e}"], [])

fail_count = 0
print(f"{'repo':<14} {'verdict':<8} detail")
print("-" * 100)
for name, (fails, notes) in results.items():
    verdict = "PASS" if not fails else "FAIL"
    if fails: fail_count += 1
    print(f"{name:<14} {verdict:<8} {notes[0] if notes else ''}")
    for f in fails:
        print(f"{'':<14} !! {f}")
print("-" * 100)
print(f"{len(results)} repos, {fail_count} failing")
sys.exit(1 if fail_count else 0)
