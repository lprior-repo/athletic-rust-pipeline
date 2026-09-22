#!/usr/bin/env python3
"""Dump sheets from the census workbook without openpyxl (zipfile + the shared string table).

    python3 xlsx_dump.py <workbook.xlsx> <sheet-name> [<sheet-name> ...]

Prints one line per non-empty row: `column=value` pairs in column order, truncated to 200 chars.
"""

from __future__ import annotations

import re
import sys
import zipfile

MIDWEST = {"WI", "MN", "IA", "IL", "MI", "IN", "OH", "MO", "KS", "NE", "ND", "SD"}


def shared_strings(z: zipfile.ZipFile) -> list[str]:
    raw = z.read("xl/sharedStrings.xml").decode("utf-8", "replace")
    return [
        "".join(re.findall(r"<t[^>]*>(.*?)</t>", si, re.S))
        for si in re.findall(r"<si>(.*?)</si>", raw, re.S)
    ]


def sheet_index(z: zipfile.ZipFile, name: str) -> int:
    wb = z.read("xl/workbook.xml").decode("utf-8", "replace")
    rels = z.read("xl/_rels/workbook.xml.rels").decode("utf-8", "replace")
    targets = dict(re.findall(r'Id="([^"]+)"[^>]*Target="([^"]+)"', rels))
    for sheet in re.findall(r"<sheet [^>]*/>", wb):
        found = re.search(r'name="([^"]+)"', sheet)
        rid = re.search(r'r:id="([^"]+)"', sheet)
        if not found or not rid or found.group(1) != name:
            continue
        target = targets.get(rid.group(1), "")
        number = re.search(r"sheet(\d+)\.xml", target)
        if number:
            return int(number.group(1))
    raise SystemExit(f"sheet {name!r} not found")


def rows_of(z: zipfile.ZipFile, strings: list[str], number: int) -> list[dict[str, str]]:
    raw = z.read(f"xl/worksheets/sheet{number}.xml").decode("utf-8", "replace")
    rows = []
    for row in re.findall(r"<row [^>]*>(.*?)</row>", raw, re.S):
        cells: dict[str, str] = {}
        for col, attrs, body in re.findall(r'<c r="([A-Z]+)\d+"([^>]*)>(.*?)</c>', row, re.S):
            value = re.search(r"<v>(.*?)</v>", body, re.S)
            if not value:
                continue
            text = value.group(1)
            if 't="s"' in attrs and text.isdigit() and int(text) < len(strings):
                text = strings[int(text)]
            cells[col] = text
        if cells:
            rows.append(cells)
    return rows


def main() -> int:
    path, names = sys.argv[1], sys.argv[2:]
    with zipfile.ZipFile(path) as z:
        strings = shared_strings(z)
        for name in names:
            rows = rows_of(z, strings, sheet_index(z, name))
            print(f"===== {name} ({len(rows)} rows)")
            for row in rows:
                line = " | ".join(f"{col}={row[col]}" for col in sorted(row, key=lambda c: (len(c), c)))
                print("  " + line[:240])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
