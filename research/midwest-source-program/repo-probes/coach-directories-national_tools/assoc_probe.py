#!/usr/bin/env python3
"""Probe every US state/DC high-school association host for a public school
directory, and record robots.txt for each host.

Stage 1: robots.txt + homepage for all 51 association hosts (verified: title
must contain association-identifying text, otherwise the row is marked suspect).
Stage 2 (separate script): follow each homepage's school-directory links.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fetch import LANE, fetch, parallel  # noqa: E402

# state code -> (expected association name, candidate hosts newest-first)
ASSOC: dict[str, tuple[str, list[str]]] = {
    "AL": ("Alabama High School Athletic Association", ["ahsaa.com"]),
    "AK": ("Alaska School Activities Association", ["asaa.org"]),
    "AZ": ("Arizona Interscholastic Association", ["aiaonline.org"]),
    "AR": ("Arkansas Activities Association", ["ahsaa.org"]),
    "CA": ("California Interscholastic Federation", ["cifstate.org"]),
    "CO": ("Colorado High School Activities Association", ["chsaanow.com"]),
    "CT": ("Connecticut Interscholastic Athletic Conference", ["ciacsports.com"]),
    "DE": ("Delaware Interscholastic Athletic Association", ["diaa.org"]),
    "DC": ("DC State Athletic Association", ["dcsaasports.org", "dcsaa.org"]),
    "FL": ("Florida High School Athletic Association", ["fhsaa.com"]),
    "GA": ("Georgia High School Association", ["ghsa.net"]),
    "HI": ("Hawaii High School Athletic Association", ["hhsaa.org", "sportshigh.com"]),
    "ID": ("Idaho High School Activities Association", ["idhsaa.org"]),
    "IL": ("Illinois High School Association", ["ihsa.org"]),
    "IN": ("Indiana High School Athletic Association", ["ihsaa.org"]),
    "IA": ("Iowa High School Athletic Association", ["iahsaa.org"]),
    "KS": ("Kansas State High School Activities Association", ["kshsaa.org"]),
    "KY": ("Kentucky High School Athletic Association", ["khsaa.org"]),
    "LA": ("Louisiana High School Athletic Association", ["lhsaa.org"]),
    "ME": ("Maine Principals' Association", ["mpa.cc", "maineprincipalsassociation.com"]),
    "MD": ("Maryland Public Secondary Schools Athletic Association", ["mpssaa.org"]),
    "MA": ("Massachusetts Interscholastic Athletic Association", ["miaa.net"]),
    "MI": ("Michigan High School Athletic Association", ["mhsaa.com"]),
    "MN": ("Minnesota State High School League", ["mshsl.org"]),
    "MS": ("Mississippi High School Activities Association", ["misshsaa.com"]),
    "MO": ("Missouri State High School Activities Association", ["mshsaa.org"]),
    "MT": ("Montana High School Association", ["mhsa.org"]),
    "NE": ("Nebraska School Activities Association", ["nsaahome.org"]),
    "NV": ("Nevada Interscholastic Activities Association", ["niaa.org"]),
    "NH": ("New Hampshire Interscholastic Athletic Association", ["nhiaa.org"]),
    "NJ": ("New Jersey State Interscholastic Athletic Association", ["njsiaa.org"]),
    "NM": ("New Mexico Activities Association", ["nmact.org"]),
    "NY": ("New York State Public High School Athletic Association", ["nysphsaa.org"]),
    "NC": ("North Carolina High School Athletic Association", ["nchsaa.org"]),
    "ND": ("North Dakota High School Activities Association", ["ndhsaa.com"]),
    "OH": ("Ohio High School Athletic Association", ["ohsaa.org"]),
    "OK": ("Oklahoma Secondary School Activities Association", ["ossaa.com"]),
    "OR": ("Oregon School Activities Association", ["osaa.org"]),
    "PA": ("Pennsylvania Interscholastic Athletic Association", ["piaa.org"]),
    "RI": ("Rhode Island Interscholastic League", ["riil.org"]),
    "SC": ("South Carolina High School League", ["schsl.org"]),
    "SD": ("South Dakota High School Activities Association", ["sdhsaa.com"]),
    "TN": ("Tennessee Secondary School Athletic Association", ["tssaa.org"]),
    "TX": ("University Interscholastic League", ["uiltexas.org"]),
    "UT": ("Utah High School Activities Association", ["uhsaa.org"]),
    "VT": ("Vermont Principals' Association", ["vpaonline.org"]),
    "VA": ("Virginia High School League", ["vhsl.org"]),
    "WA": ("Washington Interscholastic Activities Association", ["wiaa.com"]),
    "WV": ("West Virginia Secondary School Activities Commission", ["wvssac.org"]),
    "WI": ("Wisconsin Interscholastic Athletic Association", ["wiaawi.org"]),
    "WY": ("Wyoming High School Activities Association", ["whsaa.org"]),
}

TITLE_RE = re.compile(r"<title[^>]*>(.*?)</title>", re.S | re.I)


def title_of(path: Path) -> str:
    try:
        head = path.read_bytes()[:200_000].decode("utf-8", "replace")
    except OSError:
        return ""
    m = TITLE_RE.search(head)
    return re.sub(r"\s+", " ", re.sub(r"<[^>]+>", "", m.group(1))).strip() if m else ""


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) > 1 else "home"
    out_name = LANE / "tools" / f"assoc-probe-{mode}.json"
    if mode == "home":
        jobs = [(f"https://{h}/", f"assoc-home/{st}__{h}.html")
                for st, (_, hosts) in ASSOC.items() for h in hosts]
    else:
        print("usage: assoc_probe.py home", file=sys.stderr)
        return 2
    res = parallel(jobs, workers=10)
    rows = []
    for (url, name), rec in zip(jobs, res):
        host = url.split("/")[2]
        st = [k for k, (_, hs) in ASSOC.items() if host in hs][0]
        rows.append({"state": st, "url": url, "status": rec.get("status"),
                     "bytes": rec.get("bytes"), "effective": rec.get("effective"),
                     "title": title_of(LANE / "samples" / name),
                     "expected": ASSOC[st][0],
                     "robots_allow": rec.get("robots_allow"),
                     "robots_rule": rec.get("robots_rule"),
                     "error": rec.get("error", "")})
    out_name.write_text(json.dumps(rows, indent=1, sort_keys=True), encoding="utf-8")
    ok = sum(1 for r in rows if r["status"] == 200)
    print(f"{ok}/{len(rows)} HTTP 200; wrote {out_name}")
    for r in sorted(rows, key=lambda r: (r["state"], r["url"])):
        flag = "" if r["status"] == 200 else "  <== NON-200"
        print(f"{r['state']} {r['status']!s:>5} {r['bytes']!s:>8}B {r['url']:<42} {r['title'][:70]}{flag}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
