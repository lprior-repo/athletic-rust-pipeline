#!/usr/bin/env python3
"""Debt ratchet: compare current measurements against tools/quality-baseline.json.

Usage: ratchet.py <baseline> <clippy.tsv> <scan.json>

Exits non-zero if any metric grew. Shrinking metrics are reported as burndown progress.
"""

from __future__ import annotations

import json
import pathlib
import sys


def main() -> int:
    baseline = json.loads(pathlib.Path(sys.argv[1]).read_text())
    new_clippy: dict[str, int] = {}
    for line in pathlib.Path(sys.argv[2]).read_text().splitlines():
        if not line.strip():
            continue
        crate, lint, count = line.split("\t")
        new_clippy[f"{crate}\t{lint}"] = int(count)
    scan = json.loads(pathlib.Path(sys.argv[3]).read_text())

    failures: list[str] = []
    keys = sorted(set(new_clippy) | set(baseline["clippy"]))
    for key in keys:
        value = new_clippy.get(key, 0)
        was = baseline["clippy"].get(key, 0)
        if value > was:
            failures.append(f"clippy {key}: {was} -> {value}")
        if value != was:
            print(f"  clippy {key}: {was} -> {value} [{'DOWN' if value < was else 'UP'}]")
    for crate in sorted(scan["crates"]):
        for name in sorted(scan["crates"][crate]):
            value = scan["crates"][crate][name]
            if not isinstance(value, int):
                continue
            was = baseline["scan"].get(crate, {}).get(name, 0)
            if value > was:
                failures.append(f"scan {crate}.{name}: {was} -> {value}")
            if value != was:
                print(f"  scan {crate}.{name}: {was} -> {value} [{'DOWN' if value < was else 'UP'}]")
    for key in ("functions_over_60_lines", "functions_over_25_logical_lines"):
        was = baseline["structure"].get(key, 0)
        value = scan["structure"][key]
        if value > was:
            failures.append(f"structure {key}: {was} -> {value}")
        if value != was:
            print(f"  structure {key}: {was} -> {value} [{'DOWN' if value < was else 'UP'}]")

    known_files = set(baseline["structure"].get("files_over_300_lines", []))
    current_files = set(scan["structure"]["files_over_300_lines"])
    # Set membership, not just the count: a new oversized file is debt even when another file shrank.
    for path in sorted(current_files - known_files):
        failures.append(f"new file over 300 lines: {path}")
    for path in sorted(known_files - current_files):
        print(f"  file over 300 lines resolved: {path} [DOWN]")
    print(f"  files over 300 lines: {len(known_files)} -> {len(current_files)}")

    if failures:
        print("ratchet failures (debt grew):")
        for item in failures:
            print(f"  {item}")
        return 1
    print("ratchet: no metric grew")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
