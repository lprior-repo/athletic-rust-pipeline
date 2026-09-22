#!/usr/bin/env python3
"""Cross-check the 52 Athletic.net `divType=="State"` nav entries against `UsJurisdiction`.

Two tables are compared, both read from files on disk (no network, no cargo):

  * the domain enum's `code()` arms in `crates/census-domain/src/jurisdiction.rs`, and
  * the `state` field of every `divType=="State"` node in the captured `GetNavInfo` payload.

`Overseas` has `state: null`, so it can never be a jurisdiction: it is reported as out of scope
rather than silently dropped.

Run:  python3 checks-enum-crosscheck.py
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

LANE = pathlib.Path(__file__).resolve().parent.parent
REPO = LANE.parents[2]
DOMAIN = REPO / "crates/census-domain/src/jurisdiction.rs"
NAVINFO = LANE / "samples/har1-getnavinfo-wi-2026-hs-boys.json"

CODE_ARM = re.compile(r"^\s*Self::\w+\s*=>\s*\"([A-Z]{2})\",\s*$", re.MULTILINE)


def enum_codes() -> list[str]:
    text = DOMAIN.read_text()
    block = text.split("pub const fn code(self) -> &'static str {", 1)[1].split("\n    }", 1)[0]
    return CODE_ARM.findall(block)


def nav_states(nav: dict) -> list[tuple[str, dict]]:
    found: list[tuple[str, dict]] = []

    def walk(node: object, path: str) -> None:
        if isinstance(node, dict):
            if node.get("divType") == "State":
                found.append((path, node))
            for key, value in node.items():
                walk(value, f"{path}.{key}" if path else key)
        elif isinstance(node, list):
            for index, value in enumerate(node):
                walk(value, f"{path}[{index}]")

    walk(nav, "")
    return found


def enum_variants() -> dict[str, str]:
    """USPS code -> Rust variant name, from the `code()` arms."""
    text = DOMAIN.read_text()
    block = text.split("pub const fn code(self) -> &'static str {", 1)[1].split("\n    }", 1)[0]
    return {code: variant for variant, code in re.findall(r"Self::(\w+)\s*=>\s*\"([A-Z]{2})\"", block)}


def main() -> int:
    codes = enum_codes()
    variants = enum_variants()
    nav = json.loads(NAVINFO.read_text())
    states = nav_states(nav)
    regions = [(p, n) for p, n in states if p.startswith("regions[")]
    duplicates = [(p, n) for p, n in states if not p.startswith("regions[")]

    print(f"enum: {DOMAIN.relative_to(REPO)} -> {len(codes)} code() arms")
    print(f"navinfo: {NAVINFO.name} -> {len(states)} divType=='State' nodes"
          f" ({len(regions)} under regions[], {len(duplicates)} elsewhere)")
    print()
    print(f"{'nav_path':<14}{'nav_id':>9}  {'state':<7}{'enum':<26}{'ok':<4}subDivType")
    for path, node in states:
        raw = node.get("state")
        matched = [c for c in codes if raw is not None and c.lower() == str(raw).lower()]
        verdict = "yes" if matched else "OUT"
        print(f"{path:<14}{node['id']:>9}  {str(raw):<7}{(matched[0] if matched else '<none>'):<26}{verdict:<4}"
              f"{node.get('subDivType')}")
    region_ids = {node["id"] for _, node in regions}
    out_of_scope = [(node["id"], node["name"]) for _, node in regions if not node.get("state")]
    print()
    print(f"regions[] divType=='State': {len(regions)} ({len(regions) - len(out_of_scope)} mapped,"
          f" {len(out_of_scope)} out of scope)")
    print(f"divType=='State' outside regions[]: "
          f"{[(p, n['id'], n['name']) for p, n in duplicates]} (selected region echoed in tree[])")
    for node_id, name in out_of_scope:
        print(f"out of scope: {name} ({node_id}) - state=null, not one of the 51 jurisdictions")
    region_states = {node.get("state") for _, node in regions if node.get("state")}
    print(f"distinct USPS codes in regions[]: {len(region_states)}")
    print(f"enum codes with no nav entry: {sorted(set(codes) - region_states)}")
    print(f"regions[] states not in enum: {sorted(region_states - set(codes))}")
    print(f"tree[] ids not in regions[]: {sorted({n['id'] for _, n in duplicates} - region_ids)}")
    tsv = pathlib.Path(__file__).resolve().parent / "checks-state-divs.tsv"
    lines = ["nav_path\tnav_id\tstate\tenum_variant\tsub_div_type\tin_scope"]
    for path, node in regions:
        raw = node.get("state")
        lines.append("\t".join([
            path, str(node["id"]), raw or "", variants.get(raw or "", ""),
            node.get("subDivType") or "", "yes" if raw in variants else "no",
        ]))
    tsv.write_text("\n".join(lines) + "\n")
    print(f"wrote {tsv.name} ({len(regions)} data rows)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
