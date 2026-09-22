#!/usr/bin/env python3
"""Per-state tier-1 evidence collector.

For every captured association directory page compute what the ASSOCIATION itself
publishes, distinguishing:
  server_rendered - school list present in the HTML as text (>=15 "high school" hits)
  spa             - page is a JS shell; data arrives from an API not in the HTML
  fields          - which of {school, address, city, phone, AD name, AD email, coach name,
                    coach email, sport label} literally appear
All counts are literal substring/regex counts over the saved sample so any line in
SOURCE_REPORT.md can be re-derived with grep -c against the cited file.
"""
from __future__ import annotations

import html as H
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fetch import LANE  # noqa: E402

PROBES = {
    "school_word": re.compile(r"high\s+school|high\s+sch\.|\bHS\b", re.I),
    "street": re.compile(r"\b\d{1,6}\s+[A-Z][A-Za-z]+(?:\s+[A-Z][A-Za-z]+)*\s+(?:St|Street|Ave|Avenue|Rd|Road|Dr|Drive|Blvd|Way|Ln|Hwy|Pkwy)\b"),
    "city_state_zip": re.compile(r"[A-Z][A-Za-z.\- ]{2,28},\s*[A-Z]{2}\s+\d{5}"),
    "phone": re.compile(r"\(\d{3}\)\s*\d{3}-\d{4}|\d{3}[-.]\d{3}[-.]\d{4}"),
    "ad_title": re.compile(r"(?:athletic|activities)\s+director", re.I),
    "coach_word": re.compile(r"\bcoach\b", re.I),
    "sport_label": re.compile(r"cross[\s-]?country|track\s*(?:&amp;|and|&)?\s*field", re.I),
    "mailto": re.compile(r"mailto:", re.I),
    "cf_email": re.compile(r"cdn-cgi/l/email-protection#", re.I),
    "enrollment": re.compile(r"enrollment|enrolment", re.I),
    "classification": re.compile(r"classification|\bclass\s+[0-9A]\b|division|\bregion\b", re.I),
}


def vtext(raw: str) -> str:
    t = re.sub(r"(?is)<(script|style)[^>]*>.*?</\1>", " ", raw)
    t = re.sub(r"<[^>]+>", " ", t)
    return re.sub(r"\s+", " ", H.unescape(t).replace("\xa0", " ")).strip()


def collect(path: Path) -> dict:
    raw = path.read_text(encoding="utf-8", errors="replace") if path.exists() else ""
    vis = vtext(raw)
    counts = {k: len(rx.findall(vis)) for k, rx in PROBES.items()}
    counts["mailto"] = len(PROBES["mailto"].findall(raw))
    counts["cf_email"] = len(PROBES["cf_email"].findall(raw))
    schoolish = counts["school_word"]
    kind = "server_rendered" if schoolish >= 15 else ("spa_or_shell" if len(vis) < 6000 else "thin")
    return {"sample": str(path.relative_to(LANE)), "bytes": len(raw.encode()),
            "visible_chars": len(vis), "kind": kind, **counts}


def main() -> int:
    rows = []
    for f in sorted((LANE / "samples" / "dir").glob("*.html")):
        st = f.name.split("__")[0]
        slug = f.name.split("__", 1)[1].rsplit(".", 1)[0]
        if slug in ("home", "schools2", "school-pages", "kshsaa-api-letter-a"):
            continue
        rows.append({"state": st, "slug": slug, **collect(f)})
    rows += [{"state": "KS", "slug": "kshsaa-api-letter-a",
              "sample": "samples/dir/KS__kshsaa-api-letter-a.html", **collect(LANE / "samples/dir/KS__kshsaa-api-letter-a.html")},
             {"state": "IL", "slug": "ihsa-schools",
              "sample": "samples/dir/IL__ihsa-schools.html", **collect(LANE / "samples/dir/IL__ihsa-schools.html")}]
    (LANE / "tools" / "dir-evidence.json").write_text(json.dumps(rows, indent=1, sort_keys=True), encoding="utf-8")
    hdr = f"{'ST':<3}{'kind':<16}{'vis':>8}{'schools':>8}{'cityZIP':>8}{'phone':>7}{'AD':>5}{'coach':>6}{'sport':>6}{'mail':>5}{'cfmail':>7}  sample"
    print(hdr)
    for r in sorted(rows, key=lambda r: (r["state"], r["slug"])):
        print(f"{r['state']:<3}{r['kind']:<16}{r['visible_chars']:>8}{r['school_word']:>8}"
              f"{r['city_state_zip']:>8}{r['phone']:>7}{r['ad_title']:>5}{r['coach_word']:>6}"
              f"{r['sport_label']:>6}{r['mailto']:>5}{r['cf_email']:>7}  {r['sample'][14:]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
