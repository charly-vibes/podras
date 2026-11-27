#!/usr/bin/env python3
"""Record lag gate: fail when the record's capture frontier falls behind.

Purpose: detect a record that has stopped tracking the repository, without
re-importing anything and without a second ledger parser.
Responsibilities: read the facts from one `kaya check` envelope, count commits
since `last_capture_commit` that the git adapter imports, and compare the count
to a repository-owned bound.
Rationale: `kaya` reports facts and evaluates no thresholds; the bound and the
mode (warn on pull requests, fail on main and releases) belong to the caller.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys

# The git adapter records every commit except the ones carrying the
# self-identifying capture prefix (`--exclude-message-prefix`, mirroring the
# justfile's `record-capture` import). The lag count must therefore be
# path-independent: workflows, scripts, and tracker exports all become ledger
# events, so restricting the count to source paths undercounts real lag.
SELF_CAPTURE_PREFIX = "^chore(record):"


def check_facts() -> tuple[dict, str | None]:
    """Run one check and return its data plus the capture frontier."""
    try:
        process = subprocess.run(
            ["kaya", "check"],
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as exc:
        return {}, f"kaya cannot be run: {exc}"
    try:
        envelope = json.loads(process.stdout)
    except json.JSONDecodeError:
        return {}, f"kaya check printed no envelope: {process.stdout.strip()}"
    if not envelope.get("ok"):
        error = envelope.get("error") or {}
        return {}, f"kaya check failed: {error.get('label')}: {error.get('message')}"
    data = envelope.get("data") or {}
    frontier = data.get("last_capture_commit")
    if frontier is not None and not isinstance(frontier, str):
        return data, "kaya check reported a non-string last_capture_commit"
    return data, None


def commits_since(frontier: str) -> int | str:
    """Commits the git adapter imports since `frontier`, or an error message."""
    try:
        process = subprocess.run(
            [
                "git",
                "rev-list",
                "--count",
                "--grep",
                SELF_CAPTURE_PREFIX,
                "--invert-grep",
                f"{frontier}..HEAD",
            ],
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as exc:
        return f"git cannot be run: {exc}"
    if process.returncode != 0:
        return f"git cannot walk from {frontier}: {process.stderr.strip()}"
    try:
        return int(process.stdout.strip())
    except ValueError:
        return f"git reported a non-integer commit count: {process.stdout.strip()}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bound", type=int, default=25, help="allowed commits of lag")
    parser.add_argument(
        "--mode",
        choices=("warn", "fail"),
        default="fail",
        help="warn exits 0 above the bound; fail exits 1",
    )
    args = parser.parse_args()
    if args.bound < 0:
        print("record lag: --bound must be non-negative", file=sys.stderr)
        return 2

    data, error = check_facts()
    if error is not None:
        print(f"record lag: {error}", file=sys.stderr)
        return 2

    frontier = data.get("last_capture_commit")
    if frontier is None:
        message = "record lag: the record has no capture frontier"
        print(message, file=sys.stderr)
        return 1 if args.mode == "fail" else 0

    count = commits_since(frontier)
    if isinstance(count, str):
        print(f"record lag: {count}", file=sys.stderr)
        return 2

    short = frontier[:12]
    if count > args.bound:
        print(
            f"record lag: {count} commits since {short} (bound {args.bound})",
            file=sys.stderr,
        )
        return 1 if args.mode == "fail" else 0
    print(f"record lag: {count} commits since {short} (bound {args.bound})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
