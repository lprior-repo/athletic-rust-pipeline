#!/usr/bin/env python3
"""Count forbidden constructs in production-reachable code and check size budgets.

Production-reachable means: not after the first `#[cfg(test)]` attribute in a file, and not
inside a file whose name or directory marks it as test code (`tests.rs`, `*/tests/*`).
This mirrors the measurement methodology recorded in docs/HARDENING-PROGRAM.md so the
ratchet in tools/gate.sh compares like with like.

Emits JSON on stdout:

    {
      "crates": {"<crate dir>": {"assert_family": .., "panic": .., "expect": ..,
                                 "unwrap": .., "unsafe": .., "indexing": .., "as_cast": ..,
                                 "todo": .., "production_lines": .., "files": ..}},
      "structure": {"files_over_300_lines": [..], "functions_over_60_lines": ..,
                    "functions_over_25_logical_lines": ..}
    }
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent

CRATES = {
    "athletic-rust-pipeline": REPO / "src",
    "midwest-census": REPO / "crates" / "midwest-census" / "src",
}

PATTERNS = {
    "unsafe": re.compile(r"\bunsafe\s*(\{|fn|impl|trait|extern)"),
    "unwrap": re.compile(r"\.unwrap\(\)"),
    "expect": re.compile(r"\.expect\("),
    "panic": re.compile(r"\bpanic!\("),
    "unreachable": re.compile(r"\bunreachable!\("),
    "todo": re.compile(r"\b(todo!|unimplemented!)\("),
    "assert_family": re.compile(r"\b(assert!|assert_eq!|assert_ne!)\("),
    "dbg": re.compile(r"\bdbg!\("),
    "indexing": re.compile(r"\[\s*[a-zA-Z0-9_]+\s*\]"),
    "as_cast": re.compile(r"\bas\s+(u8|u16|u32|u64|usize|i8|i16|i32|i64|isize|f32|f64)\b"),
}

FILE_LINE_BUDGET = 300
FN_LINE_BUDGET = 60
FN_LOGICAL_BUDGET = 25

FN_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|async\s+|unsafe\s+)*fn\s+([a-zA-Z0-9_]+)")


def is_test_file(path: pathlib.Path) -> bool:
    return "tests" in path.name or "tests" in path.parts


def production_lines(path: pathlib.Path) -> list[str]:
    lines = path.read_text().splitlines()
    cut = next((i for i, line in enumerate(lines) if line.strip() == "#[cfg(test)]"), len(lines))
    return lines[:cut]


def scan_functions(path: pathlib.Path, production: list[str]) -> tuple[int, int]:
    over_60 = 0
    over_logical = 0
    index = 0
    while index < len(production):
        match = FN_RE.match(production[index])
        if not match:
            index += 1
            continue
        depth = 0
        end = index
        while end < len(production):
            depth += production[end].count("{") - production[end].count("}")
            if depth <= 0 and "{" in "".join(production[index:end + 1]):
                break
            end += 1
        body = production[index:end + 1]
        if end - index + 1 > FN_LINE_BUDGET:
            over_60 += 1
        logical = sum(1 for line in body if line.strip() and not line.strip().startswith("//"))
        if logical > FN_LOGICAL_BUDGET:
            over_logical += 1
        index = end + 1
    return over_60, over_logical


def main() -> int:
    report: dict[str, object] = {"crates": {}, "structure": {
        "files_over_300_lines": [], "functions_over_60_lines": 0,
        "functions_over_25_logical_lines": 0}}
    structure = report["structure"]
    assert isinstance(structure, dict)

    for crate, root in CRATES.items():
        counts = {name: 0 for name in PATTERNS}
        production_total = 0
        files = 0
        for path in sorted(root.rglob("*.rs")):
            if is_test_file(path):
                continue
            files += 1
            total_lines = len(path.read_text().splitlines())
            if total_lines > FILE_LINE_BUDGET:
                structure["files_over_300_lines"].append(
                    f"{crate}:{path.relative_to(REPO)} ({total_lines})")
            production = production_lines(path)
            production_total += len(production)
            for name, pattern in PATTERNS.items():
                counts[name] += sum(1 for line in production if pattern.search(line))
            over_60, over_logical = scan_functions(path, production)
            structure["functions_over_60_lines"] += over_60
            structure["functions_over_25_logical_lines"] += over_logical
        counts["production_lines"] = production_total
        counts["files"] = files
        report["crates"][crate] = counts

    structure["files_over_300_lines"].sort()
    json.dump(report, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
