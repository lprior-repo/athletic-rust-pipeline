#!/usr/bin/env python3
"""Type-integrity scan for domain modules (Scott Wlaschin doctrine, measured not asserted).

Counts candidates in the DOMAIN paths only:
  * `bool` parameters or return types in public signatures (boolean control flags);
  * primitive id parameters (`String`/`&str`/integer named `*id`/`*_id`) where a newtype belongs;
  * structs with two or more `Option<..>` fields (Option-as-state candidates).

These are review candidates, not verdicts: each hit must be either converted or justified. The
counts feed tools/quality-baseline.json so the number can only shrink.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent

DOMAIN_PATHS = {
    "root-domain": REPO / "src" / "domain",
    "census-domain": REPO / "crates" / "census-domain" / "src",
}

PUB_FN = re.compile(r"^\s*pub\s+(?:async\s+)?fn\s+([a-zA-Z0-9_]+)\s*(?:<[^>]*>)?\s*\(([^)]*)\)\s*(?:->\s*([^{]+))?")
ID_PARAM = re.compile(r"\b([a-zA-Z0-9_]*id)\s*:\s*(?:&(?:'[a-z]+\s+)?)?(String|&str|u8|u16|u32|u64|usize|i32|i64)")
BOOL_TOKEN = re.compile(r":\s*bool\b|->\s*bool\b")


def strip_test_cut(lines: list[str]) -> list[str]:
    cut = next((i for i, line in enumerate(lines) if line.strip() == "#[cfg(test)]"), len(lines))
    return lines[:cut]


def scan_file(path: pathlib.Path) -> dict[str, list[str]]:
    hits: dict[str, list[str]] = {"bool_in_signature": [], "primitive_id_param": [],
                                  "struct_with_many_options": []}
    lines = strip_test_cut(path.read_text().splitlines())
    for number, line in enumerate(lines, start=1):
        match = PUB_FN.match(line)
        if match:
            name, params, ret = match.group(1), match.group(2) or "", match.group(3) or ""
            if BOOL_TOKEN.search(params) or BOOL_TOKEN.search(ret):
                hits["bool_in_signature"].append(f"{path.name}:{number}:{name}")
            id_hit = ID_PARAM.search(params)
            if id_hit:
                hits["primitive_id_param"].append(f"{path.name}:{number}:{name}({id_hit.group(1)})")
        if re.match(r"^\s*pub\s+struct\s+([A-Za-z0-9_]+)", line):
            body: list[str] = []
            index = number
            while index < len(lines) and "}" not in lines[index]:
                body.append(lines[index])
                index += 1
            options = sum(1 for entry in body if re.search(r":\s*Option<", entry))
            if options >= 2:
                hits["struct_with_many_options"].append(
                    f"{path.name}:{number}:{options} option fields")
    return hits


def main() -> int:
    report: dict[str, dict[str, list[str]]] = {}
    for name, root in DOMAIN_PATHS.items():
        paths = sorted(root.rglob("*.rs")) if root.is_dir() else [root]
        combined: dict[str, list[str]] = {"bool_in_signature": [], "primitive_id_param": [],
                                          "struct_with_many_options": []}
        for path in paths:
            if not path.exists() or "tests" in path.name or "tests" in path.parts:
                continue
            for key, values in scan_file(path).items():
                combined[key].extend(values)
        report[name] = combined
    json.dump(report, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
