#!/usr/bin/env python3
"""Measure every captured association directory for contact-field content.

For each samples/dir/<ST>__*.html produce a row:
  schools        - distinct <details><summary> blocks OR table row-groups seen
  tbl_rows       - <tr> count
  coach_rows     - rows whose row-label matches track/cross-country AND a coach role
  tf_rows        - rows labelled track/cross-country
  ad_rows        - rows labelled athletic/activities director
  mailto_n       - distinct mailto: addresses in the raw HTML
  tel_n          - distinct tel: numbers (NOT collected - counted only, see report)
  json_embedded  - page embeds a JSON payload (SPA/API-driven directory)
  conclusion     - auto-classification, always re-checked by hand in SOURCE_REPORT.md
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fetch import LANE  # noqa: E402

TF = re.compile(r"track|cross[\s-]?country", re.I)
COACH_ROLE = re.compile(r"head\s*coach|coach", re.I)
AD_ROLE = re.compile(r"(athletic|activities)\s*director", re.I)
TR = re.compile(r"<tr[^>]*>(.*?)</tr>", re.S | re.I)
CELL = re.compile(r"<t[dh][^>]*>(.*?)</t[dh]>", re.S | re.I)
SUMMARY = re.compile(r"<summary[^>]*>(.*?)</summary>", re.S | re.I)


def cells(row: str) -> list[str]:
    out = []
    for c in CELL.findall(row):
        txt = re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", c)).strip()
        out.append(txt)
    return out


def analyze(path: Path) -> dict:
    html = path.read_text(encoding="utf-8", errors="replace") if path.exists() else ""
    rows = [cells(r) for r in TR.findall(html)]
    tf = [r for r in rows if r and TF.search(r[0])]
    coach = [r for r in tf if len(r) > 1 and COACH_ROLE.search(r[1])]
    ad = [r for r in rows if len(r) > 1 and AD_ROLE.search(r[1])]
    mailtos = sorted(set(re.findall(r"mailto:([^\"'>\s?]+)", html, re.I)))
    tels = sorted(set(re.findall(r"tel:([^\"'>\s]+)", html, re.I)))
    embedded = bool(re.search(r"__NEXT_DATA__|application/json|\"data\":\s*\[|window\.__", html))
    return {
        "sample": str(path.relative_to(LANE)),
        "bytes": len(html.encode("utf-8", "replace")),
        "tbl_rows": len(rows),
        "schools_details": len(SUMMARY.findall(html)),
        "tf_rows": len(tf),
        "coach_rows": len(coach),
        "ad_rows": len(ad),
        "mailto_n": len(mailtos),
        "mailto_sample": mailtos[:3],
        "tel_n": len(tels),
        "json_embedded": embedded,
        "coach_eg": coach[0][:3] if coach else (tf[0][:3] if tf else []),
        "ad_eg": ad[0][:3] if ad else [],
    }


def main() -> int:
    pats = sys.argv[1:] or ["*"]
    files: list[Path] = []
    for p in pats:
        files += sorted((LANE / "samples" / "dir").glob(f"{p}__*.html" if "*" not in p else p))
    if not files:
        files = sorted((LANE / "samples" / "dir").glob("*.html"))
    out = [{"state": f.name.split("__")[0], **analyze(f)} for f in files]
    (LANE / "tools" / "dir-fields.json").write_text(
        json.dumps(out, indent=1, sort_keys=True), encoding="utf-8")
    print(f"{'ST':<3}{'rows':>7}{'schools':>8}{'tf':>6}{'coach':>7}{'AD':>5}{'mail':>6}{'tel':>6} json  sample")
    for r in sorted(out, key=lambda r: (-r["coach_rows"], -r["ad_rows"], r["state"])):
        print(f"{r['state']:<3}{r['tbl_rows']:>7}{r['schools_details']:>8}{r['tf_rows']:>6}"
              f"{r['coach_rows']:>7}{r['ad_rows']:>5}{r['mailto_n']:>6}{r['tel_n']:>6} "
              f"{'Y' if r['json_embedded'] else '.':<5} {r['sample']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
