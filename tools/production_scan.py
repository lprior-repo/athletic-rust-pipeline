#!/usr/bin/env python3
"""Count forbidden constructs in production-reachable code and check size budgets.

Production-reachable means: not after the `#[cfg(test)]` attribute that opens a test module, and
not inside a file whose name or directory marks it as test code (`tests.rs`, `*/tests/*`). A
`#[cfg(test)]` that only gates `use` re-exports does not end the production region (`xlsx.rs`);
see `production_lines`. This mirrors the measurement methodology recorded in
docs/HARDENING-PROGRAM.md so the ratchet in tools/gate.sh compares like with like.

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
    "as_cast": re.compile(r"\bas\s+(u8|u16|u32|u64|usize|i8|i16|i32|i64|isize|f32|f64)\b"),
}

# Real Rust indexing: `expr[i]`, `expr[0]`, `expr[identifier]`, where the `[` follows an expression.
# The lookbehind requires an expression-ending character (identifier char, `)`, or `]`), which
# deliberately excludes slice and array types (`&[T]`, `&'a [T]`, `&mut [u8]`, `: [T]`, `-> [T]`,
# `[u8; 4]`), attributes (`#[must_use]`, `#[test]`, `#[cfg(test)]`) and prose or JSON (`: [1, 2]`).
# Whitespace before `[` is NOT allowed on purpose: the gate runs rustfmt, so `expr [i]` cannot occur
# in this repo, while allowing the space re-admits every type false positive above by letting the
# match start on the space itself (`&mut [u8]` matched once the engine began at the space after
# `mut`). Do not "simplify" this back to `\[\s*[a-zA-Z0-9_]+\s*\]`: that pattern scored slice types
# and `#[must_use]` as indexing (src/domain/decision.rs: 21 reported, 0 real index operators).
INDEXING = re.compile(r"(?<=[A-Za-z0-9_)\]])\[\s*[a-zA-Z0-9_]+\s*\]")

FILE_LINE_BUDGET = 300
FN_LINE_BUDGET = 60
FN_LOGICAL_BUDGET = 25

FN_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|async\s+|unsafe\s+)*fn\s+([a-zA-Z0-9_]+)")


def is_test_file(path: pathlib.Path) -> bool:
    return "tests" in path.name or "tests" in path.parts


ATTRIBUTE_LINE = re.compile(r"^\s*#\[")
TEST_ITEM_LINE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s")


def production_lines(path: pathlib.Path) -> list[str]:
    """Lines before the `#[cfg(test)]` attribute that opens the file's test module.

    A bare `#[cfg(test)]` ends the production region only when it gates a module. Files that
    cfg(test)-gate `use` re-exports (src/xlsx.rs: `pub(crate) use cells::{..}`) must keep every
    line in scope, otherwise the real production code below them is invisible to every count and
    to the size budgets.
    """
    lines = path.read_text().splitlines()
    for index, line in enumerate(lines):
        if line.strip() != "#[cfg(test)]":
            continue
        lookahead = index + 1
        while lookahead < len(lines) and (
            not lines[lookahead].strip() or ATTRIBUTE_LINE.match(lines[lookahead])
        ):
            lookahead += 1
        if lookahead < len(lines) and TEST_ITEM_LINE.match(lines[lookahead]):
            return lines[:index]
    return lines


class CodeMask:
    """Blanks Rust string literals and comments so the indexing count only sees code.

    HTML, CSS and JavaScript payloads live in string literals here and they read like indexing to
    a regex: `selector("a[href]")`, `headers[name] = value;` inside an injected script, and
    `r"(?is)<t[hd]..` capture patterns. None of those are Rust indexing. Only the indexing metric
    uses the mask; every other pattern keeps the historical per-line semantics so their recorded
    baselines do not move. Conservative by construction: anything not recognised as a string,
    character literal or comment stays visible, so an unparsed form can only keep a hit, never hide
    one. `/* */` depth and raw-string hashes carry across the lines of one file.
    """

    CHAR_LITERAL = re.compile(r"'\\(?:.|x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f]{1,6}\})'|'.'")

    def __init__(self) -> None:
        self.block_depth = 0
        self.raw_hashes: int | None = None

    def apply(self, line: str) -> str:
        masked = list(line)
        index = 0
        while index < len(line):
            if self.raw_hashes is not None:
                index = self._end_raw_string(line, masked, index)
            elif self.block_depth > 0:
                index = self._end_block_comment(line, masked, index)
            else:
                index = self._advance(line, masked, index)
        return "".join(masked)

    def _advance(self, line: str, masked: list[str], index: int) -> int:
        char = line[index]
        if char == "/" and line.startswith("//", index):
            self._blank(masked, index, len(line))
            return len(line)
        if char == "/" and line.startswith("/*", index):
            self.block_depth = 1
            self._blank(masked, index, index + 2)
            return self._end_block_comment(line, masked, index + 2)
        if char == "r" and not self._continues_identifier(line, index):
            return self._start_raw_string(line, masked, index)
        if char == '"':
            return self._end_string(line, masked, index)
        if char == "'":
            return self._end_char_literal(line, masked, index)
        return index + 1

    def _start_raw_string(self, line: str, masked: list[str], index: int) -> int:
        cursor = index + 1
        while cursor < len(line) and line[cursor] == "#":
            cursor += 1
        if cursor >= len(line) or line[cursor] != '"':
            return index + 1
        self.raw_hashes = cursor - index - 1
        self._blank(masked, index, cursor + 1)
        return self._end_raw_string(line, masked, cursor + 1)

    def _end_raw_string(self, line: str, masked: list[str], index: int) -> int:
        terminator = '"' + "#" * (self.raw_hashes or 0)
        end = line.find(terminator, index)
        if end < 0:
            self._blank(masked, index, len(line))
            return len(line)
        self.raw_hashes = None
        self._blank(masked, index, end + len(terminator))
        return end + len(terminator)

    def _end_block_comment(self, line: str, masked: list[str], index: int) -> int:
        start = index
        while index < len(line):
            nested = line.find("/*", index)
            close = line.find("*/", index)
            if close < 0 and nested < 0:
                break
            if 0 <= nested < close or close < 0:
                self.block_depth += 1
                index = nested + 2
                continue
            self.block_depth -= 1
            if self.block_depth == 0:
                self._blank(masked, start, close + 2)
                return close + 2
            index = close + 2
        self._blank(masked, start, len(line))
        return len(line)

    def _end_string(self, line: str, masked: list[str], index: int) -> int:
        cursor = index + 1
        while cursor < len(line):
            if line[cursor] == "\\":
                cursor += 2
            elif line[cursor] == '"':
                self._blank(masked, index, cursor + 1)
                return cursor + 1
            else:
                cursor += 1
        self._blank(masked, index, len(line))
        return len(line)

    def _end_char_literal(self, line: str, masked: list[str], index: int) -> int:
        literal = self.CHAR_LITERAL.match(line, index)
        if literal is None:
            return index + 1
        self._blank(masked, index, literal.end())
        return literal.end()

    @staticmethod
    def _continues_identifier(line: str, index: int) -> bool:
        return index > 0 and (line[index - 1].isalnum() or line[index - 1] == "_")

    @staticmethod
    def _blank(masked: list[str], start: int, end: int) -> None:
        for position in range(start, min(end, len(masked))):
            masked[position] = " "


def count_indexing(lines: list[str]) -> int:
    """Count lines carrying real indexing, ignoring matches inside literals and comments."""
    mask = CodeMask()
    masked = [mask.apply(line) for line in lines]
    return sum(1 for line in masked if INDEXING.search(line))


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
        counts["indexing"] = 0
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
            counts["indexing"] += count_indexing(production)
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
