#!/usr/bin/env python3
"""Stage 3: fetch the association school-directory URL for each state and measure
which contact fields it actually exposes.

Field probes are literal substring counts against the raw bytes:
  school_name  - an <h1>/row anchor is present (heuristic, verified per state in report)
  coach        - "coach" occurrences (case-insensitive)
  head_coach   - "head coach" occurrences
  athletic_dir - "athletic director" / "activities director" occurrences
  mailto       - mailto: links actually present in the HTML
  xc_tf        - "cross country" / "track and field" / "track & field" occurrences
Every measurement is reproducible from the saved sample with the count_* helpers.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fetch import LANE, fetch, parallel  # noqa: E402

# state -> (directory URL, short slug, what we expected to find there)
DIRS: dict[str, tuple[str, str, str]] = {
    "AK": ("https://asaa.org/about/member-schools/", "member-schools", "tier1-assoc"),
    "AZ": ("https://aiaonline.org/schools", "schools", "tier1-assoc"),
    "CO": ("https://schools.chsaa.org", "schools", "tier1-assoc"),
    "GA": ("https://ghsa.net/school-directory", "school-directory", "tier1-assoc"),
    "HI": ("https://hhsaa.org/schools", "schools", "tier1-assoc"),
    "IA": ("https://www.iahsaa.org/member-schools/", "member-schools", "tier1-assoc"),
    "KY": ("https://khsaa.org/08-16-2024-khsaa-school-directory-available-via-arbiterlive/",
           "arbiterlive-note", "tier1-assoc"),
    "LA": ("https://www.lhsaa.org/school-directory", "school-directory", "tier1-assoc"),
    "MA": ("https://miaa.net/schools", "schools", "tier1-assoc"),
    "MD": ("https://www.mpssaa.org/school-directory/", "school-directory", "tier1-assoc"),
    "MI": ("https://mhsaa.com/schools", "schools", "tier1-assoc"),
    "MN": ("https://mshsl.org/schools", "schools", "tier1-assoc"),
    "MT": ("https://www.mhsa.org/schools/directory/", "directory", "tier1-assoc"),
    "NC": ("https://www.nchsaa.org/schools/", "schools", "tier1-assoc"),
    "ND": ("https://ndhsaa.com/schools", "schools", "tier1-assoc"),
    "NE": ("https://nsaahome.org/schools/", "schools", "tier1-assoc"),
    "NH": ("https://www.nhiaa.org/about-nhiaa/schools/", "schools", "tier1-assoc"),
    "NJ": ("https://njsiaa.org/schools", "schools", "tier1-assoc"),
    "NM": ("https://www.nmact.org/member-schools/", "member-schools", "tier1-assoc"),
    "OR": ("https://www.osaa.org/schools/full-members", "full-members", "tier1-assoc"),
    "RI": ("https://riil.org/Directory.aspx", "directory", "tier1-assoc"),
    "SC": ("https://schsl.org/schsl-directory", "schsl-directory", "tier1-assoc"),
    "TN": ("https://tssaa.org/directory", "directory", "tier1-assoc"),
    "UT": ("https://uhsaa.org/school-directory-new/", "school-directory", "tier1-assoc"),
    "VT": ("https://vpaonline.org/membership/", "membership", "tier1-assoc"),
    "WA": ("https://wiaa.finalforms.com/state_schools", "state_schools", "tier1-assoc"),
    "WI": ("https://schools.wiaawi.org/Directory/School/List", "school-list", "tier1-assoc"),
    "WV": ("https://www.wvssac.org/school-resources/school-directory/", "school-directory", "tier1-assoc"),
    # reference tiers already proven in the Midwest study (re-fetched here for provenance)
    "KS": ("https://kshsaa-api.kshsaa.org/directory/search/name/a/", "kshsaa-api-letter-a", "tier1-assoc"),
    "IL": ("https://api.ihsa.org/v1/schools", "ihsa-schools", "tier1-assoc"),
    "OH": ("https://officials.myohsaa.org/Outside/SearchSchool", "search-school", "tier1-assoc"),
    "SD": ("https://www.gobound.com/sd/associations/sdhsaa/schools", "bound-sd-schools", "tier1-assoc"),
    "IN": ("https://www.ihsaa.org/schools/ihsaa-school-directory", "ihsaa-school-directory", "tier1-assoc"),
    "ME": ("https://www.mpa.cc/", "home", "tier1-assoc"),
    "DC": ("https://www.dcsaa.org/", "home", "tier1-assoc"),
    "NV": ("https://niaa.fpsports.org/", "home", "tier1-assoc"),
    "CT": ("https://ciac.fpsports.org/", "home", "tier1-assoc"),
    "ID": ("https://idhsaa.org/", "home", "tier1-assoc"),
    "PA": ("https://piaa.org/", "home", "tier1-assoc"),
    "NY": ("https://nysphsaa.org/", "home", "tier1-assoc"),
    "VA": ("https://www.vhsl.org/", "home", "tier1-assoc"),
    "AL": ("https://ahsaa.com/", "home", "tier1-assoc"),
    "MO": ("https://www.mshsaa.org/", "home", "tier1-assoc"),
    "CA": ("https://cifstate.org/", "home", "tier1-assoc"),
    "FL": ("https://fhsaa.com/", "home", "tier1-assoc"),
    "MS": ("https://misshsaa.com/", "home", "tier1-assoc"),
    "DE": ("https://diaa.org/", "home", "tier1-assoc"),
    "OK": ("https://ossaa.com/", "home", "tier1-assoc"),
    "TX": ("https://uiltexas.org/", "home", "tier1-assoc"),
    "WY": ("https://whsaa.org/", "home", "tier1-assoc"),
    "AR": ("https://ahsaa.org/", "home", "tier1-assoc"),
}

RX = {
    "coach": re.compile(r"coach", re.I),
    "head_coach": re.compile(r"head\s+coach", re.I),
    "athletic_dir": re.compile(r"athletic\s+director", re.I),
    "activities_dir": re.compile(r"activities\s+director", re.I),
    "mailto": re.compile(r"mailto:[^\"'>\s]+", re.I),
    "xc_tf": re.compile(r"cross[\s-]?country|track\s*(?:&amp;|and|&)?\s*field", re.I),
    "email_protection": re.compile(r"email-protection|cf_email|cloudflare", re.I),
    "phone": re.compile(r"\(\d{3}\)\s*\d{3}-\d{4}|\d{3}-\d{3}-\d{4}", re.I),
    "json": re.compile(r"application/json|__NEXT_DATA__|\"data\":\s*\["),
}


def measure(path: Path) -> dict:
    raw = path.read_bytes() if path.exists() else b""
    text = raw.decode("utf-8", "replace")
    out = {"bytes": len(raw)}
    for k, rx in RX.items():
        hits = rx.findall(text)
        out[k] = len(hits)
        if k == "mailto" and hits:
            out["mailto_sample"] = sorted(set(hits))[:5]
    return out


def main() -> int:
    only = sys.argv[1:] or sorted(DIRS)
    jobs, meta = [], []
    for st in only:
        url, slug, tier = DIRS[st]
        name = f"dir/{st}__{slug}.html"
        jobs.append((url, name))
        meta.append((st, slug, tier, name))
    res = parallel(jobs, workers=8)
    rows = []
    for (st, slug, tier, name), rec in zip(meta, res):
        m = measure(LANE / "samples" / name)
        rows.append({"state": st, "slug": slug, "tier": tier, "url": rec.get("url"),
                     "status": rec.get("status"), "effective": rec.get("effective"),
                     "sample": f"samples/{name}", **m,
                     "error": rec.get("error", "")})
    out = LANE / "tools" / "dir-probe.json"
    prev = json.loads(out.read_text()) if out.exists() else []
    keep = {r["state"] for r in rows}
    out.write_text(json.dumps([r for r in prev if r["state"] not in keep] + rows,
                              indent=1, sort_keys=True), encoding="utf-8")
    for r in sorted(rows, key=lambda r: r["state"]):
        print(f"{r['state']} {r['status']!s:>5} {r['bytes']:>8}B coach={r['coach']:<5} "
              f"AD={r['athletic_dir']:<4} mailto={r['mailto']:<4} xctf={r['xc_tf']:<5} "
              f"{r['sample']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
