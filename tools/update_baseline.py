#!/usr/bin/env python3
"""Rewrite tools/quality-baseline.json from current measurements.

Usage: update_baseline.py <baseline> <clippy.tsv> <scan.json> [--allow-increase]

Refuses to raise any number without --allow-increase: the baseline is a ratchet, and a burndown
is the only legitimate reason for it to move down.
"""

from __future__ import annotations

import json
import pathlib
import sys


def main() -> int:
    baseline_path = pathlib.Path(sys.argv[1])
    clippy_path = pathlib.Path(sys.argv[2])
    scan_path = pathlib.Path(sys.argv[3])
    allow = "--allow-increase" in sys.argv[4:]

    new_clippy: dict[str, int] = {}
    for line in clippy_path.read_text().splitlines():
        if not line.strip():
            continue
        crate, lint, count = line.split("\t")
        new_clippy[f"{crate}\t{lint}"] = int(count)
    scan = json.loads(scan_path.read_text())
    old = json.loads(baseline_path.read_text()) if baseline_path.exists() else {}

    raised: list[str] = []
    if not allow and old:
        old_clippy = old.get("clippy", {})
        for key, value in new_clippy.items():
            if value > old_clippy.get(key, 0):
                raised.append(f"clippy {key}: {old_clippy.get(key, 0)} -> {value}")
        for crate, counts in scan["crates"].items():
            for name, value in counts.items():
                before = old.get("scan", {}).get(crate, {}).get(name, 0)
                if isinstance(value, int) and value > before:
                    raised.append(f"scan {crate}.{name}: {before} -> {value}")
        for key in ("files_over_300_lines", "functions_over_60_lines",
                    "functions_over_25_logical_lines"):
            before = len(old.get("structure", {}).get(key, [])) if key == "files_over_300_lines" \
                else old.get("structure", {}).get(key, 0)
            after = len(scan["structure"][key]) if key == "files_over_300_lines" \
                else scan["structure"][key]
            if after > before:
                raised.append(f"structure {key}: {before} -> {after}")
    if raised:
        print("refusing to raise the baseline without --allow-increase:")
        for item in raised:
            print(f"  {item}")
        return 1

    baseline_path.write_text(json.dumps({
        "note": "Debt baseline for tools/gate.sh. Numbers may only shrink; "
                "refresh with tools/gate.sh --update-baseline after a burndown.",
        "clippy": new_clippy,
        "scan": scan["crates"],
        "structure": scan["structure"],
    }, indent=2, sort_keys=True) + "\n")
    print(f"baseline updated: {baseline_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
