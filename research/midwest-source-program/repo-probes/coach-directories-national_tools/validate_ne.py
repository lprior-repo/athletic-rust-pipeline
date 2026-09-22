"""Measure every baseline NE row against the live NSAA school directory.

`secure.nsaahome.org/nsaaforms/direxportscreen.php` selects one school at a time
(POST `school=<name>`), so covering the whole NE slice costs one request per distinct
school. The baseline names 311 distinct NE schools / 1,528 rows, so this script POSTs
each school name, stores the byte-exact body under samples/validate/ne/, then checks
every baseline row's coach name and AD name against that school's captured page.

Only role->name pairs published by the association are read (contract: no personal
contacts). NSAA publishes no email field at all, so nothing email-shaped is extracted.
"""

import csv
import json
import re
import sys
import unicodedata
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fetch import fetch  # noqa: E402

LANE = Path(__file__).resolve().parent.parent
OUT = LANE / "tools" / "validation-ne-wholesale.json"
BASE = Path.home() / "Downloads" / "midwest-tfxc-source-research" / "data" / "coach-contacts.csv"
URL = "https://secure.nsaahome.org/nsaaforms/direxportscreen.php"


def slug(s: str) -> str:
    s = unicodedata.normalize("NFKD", s).encode("ascii", "ignore").decode()
    return re.sub(r"[^a-z0-9]+", "_", s.lower()).strip("_")[:80]


def rendered(body: str) -> tuple[str, str]:
    import html as H
    t = re.sub(r"(?is)<(script|style)[^>]*>.*?</\1>", " ", body)
    attrs = " ".join(m.group(1) for m in
                     re.finditer(r'(?i)\b(?:href|value|title|alt)\s*=\s*"([^"]*)"', body))
    t = re.sub(r"<[^>]+>", " ", t + " " + attrs)
    t = re.sub(r"\s+", " ", H.unescape(t))
    return t, re.sub(r"\s+", "", t)


def norm(s: str) -> str:
    return re.sub(r"[^a-z]", "", unicodedata.normalize("NFKD", s).encode("ascii", "ignore").decode().lower())


def main() -> int:
    with BASE.open(encoding="utf-8") as fh:
        rows = [r for r in csv.DictReader(fh) if r["state"] == "NE"]
    schools = sorted({r["school"] for r in rows})
    print(f"NE baseline rows={len(rows)} distinct schools={len(schools)}", flush=True)

    bodies: dict[str, tuple[str, str, str]] = {}
    for i, s in enumerate(schools, 1):
        rec = fetch(URL, f"validate/ne/{slug(s)}.html", extra=["--data-urlencode", f"school={s}"])
        p = LANE / "samples" / "validate" / "ne" / f"{slug(s)}.html"
        body = p.read_text(encoding="utf-8", errors="replace") if p.exists() else ""
        bodies[s] = (body, *rendered(body))
        if i % 25 == 0 or i == len(schools):
            print(f"  fetched {i}/{len(schools)} (last status={rec.get('status')}, "
                  f"bytes={rec.get('bytes')})", flush=True)

    recs = []
    for r in rows:
        body, spaced, compact = bodies.get(r["school"], ("", "", ""))
        coach, ad = r["coach_name"].strip(), r["ad_name"].strip()
        rec = {
            "school": r["school"], "sport": r["sport"], "role": r["role"],
            "baseline_coach": coach, "baseline_ad": ad,
            "page_bytes": len(body),
            "agree_coach": bool(coach) and (coach in spaced or norm(coach) in compact.lower()),
            "agree_ad": bool(ad) and (ad in spaced or norm(ad) in compact.lower()),
        }
        recs.append(rec)

    OUT.write_text(json.dumps(recs, indent=1) + "\n", encoding="utf-8")

    pages_ok = sum(1 for b, _, _ in bodies.values() if len(b) > 2000)
    coach_rows = [r for r in recs if r["baseline_coach"]]
    ad_rows = [r for r in recs if r["baseline_ad"]]
    cm = [r for r in coach_rows if r["agree_coach"]]
    am = [r for r in ad_rows if r["agree_ad"]]
    print(f"\nschool pages captured (>2 KB)   : {pages_ok}/{len(schools)}")
    print(f"coach rows checked              : {len(coach_rows)}  agree {len(cm)} "
          f"= {100 * len(cm) / max(1, len(coach_rows)):.1f}%")
    print(f"AD rows checked                 : {len(ad_rows)}  agree {len(am)} "
          f"= {100 * len(am) / max(1, len(ad_rows)):.1f}%")
    print("\nfirst 6 rows:")
    for r in recs[:6]:
        print(f"  {r['school'][:24]:<24} {r['role'][:34]:<34} coach={r['agree_coach']} ad={r['agree_ad']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
