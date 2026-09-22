#!/usr/bin/env python3
"""Validate sampled rows of ~/Downloads/midwest-tfxc-source-research/data/coach-contacts.csv
against the live official source, per host stratum.

Each check re-fetches the row's own source_url (or the per-school URL derived from
the school key it names) and reports whether the baseline coach_name / ad_name /
emails still appear in the live payload. Output: tools/baseline-validation.json.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
import time
import urllib.parse
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fetch import LANE, UA, fetch, robots_for, _throttle  # noqa: E402

BASE_CSV = Path.home() / "Downloads/midwest-tfxc-source-research/data/coach-contacts.csv"

# (label, state, url, substrings that must appear in the live body, note)
CHECKS: list[dict] = []


def add(label: str, st: str, url: str, must: list[str], note: str = "", data: str | None = None,
        name: str | None = None) -> None:
    CHECKS.append({"label": label, "state": st, "url": url, "must": must, "note": note,
                   "data": data, "name": name})


def norm(s: str) -> str:
    return re.sub(r"\s+", " ", (s or "")).strip()


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] == "help":
        print("usage: validate_baseline.py <plan|run>")
        return 2
    plan = json.loads((LANE / "tools" / "validation-plan.json").read_text())
    results = []
    for c in plan:
        name = c.get("name") or ("validate/" + re.sub(r"[^A-Za-z0-9]+", "_", c["label"])[:80] + ".html")
        rec = fetch(c["url"], name, data=c.get("data"))
        body = (LANE / "samples" / name).read_text(encoding="utf-8", errors="replace") if (LANE / "samples" / name).exists() else ""
        found = {s: (norm(s) in norm(body)) for s in c["must"]}
        results.append({"label": c["label"], "state": c["state"], "url": c["url"],
                        "http": rec.get("status"), "bytes": rec.get("bytes"),
                        "sample": f"samples/{name}", "found": found,
                        "agree": all(found.values()) if found else None,
                        "note": c.get("note", "")})
        print(f"{c['label'][:56]:<58} http={rec.get('status')} {rec.get('bytes')}B "
              f"agree={results[-1]['agree']} {found}")
    out = LANE / "tools" / "baseline-validation.json"
    prev = json.loads(out.read_text()) if out.exists() else []
    seen = {(r["label"], r["url"]) for r in results}
    out.write_text(json.dumps([r for r in prev if (r["label"], r["url"]) not in seen] + results,
                              indent=1, sort_keys=True), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
