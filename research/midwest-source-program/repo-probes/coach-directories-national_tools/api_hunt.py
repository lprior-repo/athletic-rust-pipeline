#!/usr/bin/env python3
"""Locate the JSON endpoints behind client-rendered association directories.

For a saved SPA shell: extract same-origin <script src> bundles, fetch the largest
few, and report candidate API paths + any direct JSON endpoint already present in
the HTML. Bounded: max 3 bundles per state, >=1s per host.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from urllib.parse import urljoin, urlsplit

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fetch import LANE, fetch  # noqa: E402

PAT = re.compile(r"""["'`]((?:https?://[^"'`\s]{0,80})?/(?:api|v\d|graphql|_next/data|wp-json|rest)/
                      [A-Za-z0-9_\-./{}$:]{0,60})["'`]""", re.X)
SRC = re.compile(r"""<script[^>]+src=["']([^"']+\.js[^"']*)["']""", re.I)
JSONHINT = re.compile(r"""(?i)(?:json|api|endpoint)\s*[:=]\s*["']([^"']{4,120})["']""")


def main() -> int:
    states = sys.argv[1:]
    ev = json.loads((LANE / "tools" / "dir-evidence.json").read_text())
    todo = [e for e in ev if (not states or e["state"] in states)]
    out = []
    for e in sorted(todo, key=lambda x: x["state"]):
        shell = LANE / e["sample"]
        if not shell.exists():
            continue
        html = shell.read_text(encoding="utf-8", errors="replace")
        base = "https://" + shell.name.split("__")[0]  # unused fallback
        urls = [u for u in SRC.findall(html)]
        api_hits = sorted(set(PAT.findall(html)))
        # pick up to 3 bundles; bundle must be absolute or resolve against the real URL
        real = e.get("url", "")
        picked, scripts = [], []
        for u in urls:
            if u.startswith("http") and "//" in u:
                scripts.append(u)
        # prefer same-site bundles, then any
        scripts.sort(key=lambda u: (0 if urlsplit(u).netloc in REAL_HOSTS.get(e["state"], []) else 1, -len(u)))
        for u in scripts[:3]:
            picked.append(u)
        found_apis = set(api_hits)
        for u in picked:
            nm = "api/" + re.sub(r"[^A-Za-z0-9]+", "_", u)[-90:]
            rec = fetch(u, nm)
            if rec.get("status") == 200:
                js = (LANE / "samples" / nm).read_text(encoding="utf-8", errors="replace")
                found_apis |= set(PAT.findall(js))
                found_apis |= {m for m in JSONHINT.findall(js) if "/" in m and len(m) < 90}
        row = {"state": e["state"], "sample": e["sample"], "scripts": picked,
               "apis": sorted(a for a in found_apis if len(a) > 4)[:25]}
        out.append(row)
        print(f"{e['state']}: {len(picked)} bundles -> {row['apis'][:8] or 'NO API PATHS'}")
    prev = json.loads((LANE / "tools" / "api-hunt.json").read_text()) if (LANE / "tools" / "api-hunt.json").exists() else []
    keep = {r["state"] for r in out}
    (LANE / "tools" / "api-hunt.json").write_text(
        json.dumps([r for r in prev if r["state"] not in keep] + out, indent=1, sort_keys=True), encoding="utf-8")
    return 0


REAL_HOSTS: dict[str, list[str]] = {}
if __name__ == "__main__":
    raise SystemExit(main())
