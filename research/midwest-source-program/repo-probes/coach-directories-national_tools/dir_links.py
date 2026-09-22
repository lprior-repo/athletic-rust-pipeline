#!/usr/bin/env python3
"""Stage 2: from each captured association homepage, rank candidate school-directory
URLs (link text + href containing school/directory/member/schools). Then fetch the
best candidate per association and measure contact fields.

Usage:
  python3 assoc_probe.py links     # rank candidate directory URLs from saved homepages
  python3 assoc_probe.py fetch     # fetch top candidate per state
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fetch import LANE, fetch, parallel  # noqa: E402

from assoc_probe import ASSOC  # noqa: E402

HOST_FIX = {  # hosts confirmed live after stage 1 (docs: tools/assoc-probe-home.json)
    "AL": "ahsaa.com", "CT": "ciac.fpsports.org", "DC": "www.dcsaa.org",
    "ME": "www.mpa.cc", "MO": "www.mshsaa.org", "NV": "niaa.fpsports.org",
    "VA": "www.vhsl.org",
}
PRIORITY = {
    # substring -> score (higher wins)
    "school": 6, "directory": 6, "member": 4, "coach": 5, "athletic-director": 6,
    "ad-": 4, "staff": 3, "find": 3, "search": 2, "schools": 7,
}
NEG = re.compile(r"(login|signin|sign-in|register|cart|store|donate|password|reset|"
                 r"\.pdf$|\.jpg|\.png|\.css|\.js$|facebook|twitter|instagram|youtube|"
                 r"ticket|live-stream|broadcast|shop|photos|video)", re.I)
A_RE = re.compile(r'<a\b[^>]*?href="([^"#]+)"[^>]*>(.*?)</a>', re.S | re.I)


def norm_text(s: str) -> str:
    return re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", s)).strip()


def scan_homepages() -> dict:
    out: dict[str, list[dict]] = {}
    for st, (_, hosts) in ASSOC.items():
        host = HOST_FIX.get(st, hosts[0])
        cands: dict[str, dict] = {}
        for f in sorted((LANE / "samples" / "assoc-home").glob(f"{st}__*.html")):
            html = f.read_text(encoding="utf-8", errors="replace")
            for href, text in A_RE.findall(html):
                if href.startswith(("mailto:", "tel:", "javascript:", "#")):
                    continue
                label = norm_text(text)[:90]
                score = sum(v for k, v in PRIORITY.items() if k in href.lower())
                score += sum(v for k, v in PRIORITY.items() if k in label.lower())
                if NEG.search(href) or score == 0:
                    continue
                key = href.split("#")[0]
                if key not in cands or score > cands[key]["score"]:
                    cands[key] = {"href": key, "label": label, "score": score,
                                  "from": f.name}
        out[st] = sorted(cands.values(), key=lambda d: -d["score"])[:8]
    (LANE / "tools" / "dir-candidates.json").write_text(
        json.dumps(out, indent=1, sort_keys=True), encoding="utf-8")
    return out


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) > 1 else "links"
    if mode == "links":
        cands = scan_homepages()
        for st in sorted(cands):
            print(f"== {st}  ({HOST_FIX.get(st, ASSOC[st][1][0])})")
            for c in cands[st][:5]:
                print(f"   {c['score']:>2}  {c['href'][:110]:<112} {c['label'][:60]}")
        return 0
    print("usage: assoc_probe.py links", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
