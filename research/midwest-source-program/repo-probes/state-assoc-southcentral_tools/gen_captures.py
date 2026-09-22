#!/usr/bin/env python3
"""Generate samples/CAPTURES.md from samples/.captures.tsv plus derived-file records.

Usage: python3 tools/gen_captures.py
"""
from __future__ import annotations

import os
import re
import subprocess
import sys

LANE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SAMPLES = os.path.join(LANE, "samples")
UA = "ad-law-scrape-source-research/0.1 (anonymous; respectful 1rps; contact: repo owner)"

# extra captures made directly with curl (not through probe.sh), with the exact command used
EXTRA = {
    "wayback-cdx-misshsaa-results.txt": (
        "https://web.archive.org/cdx/search/cdx?url=misshsaa.com&matchType=domain&fl=original,timestamp,statuscode&collapse=urlkey&filter=original:.*(state|cross|track|champ|result).*&filter=statuscode:200&to=2023&limit=100",
        "curl -sS --max-time 120 -A '<UA>' \"<url>\" -o samples/wayback-cdx-misshsaa-results.txt -w 'http=%{http_code} bytes=%{size_download}\\n'",
    ),
    "wayback-cdx-misshsaa-xc.txt": (
        "https://web.archive.org/cdx/search/cdx?url=misshsaa.com&matchType=domain&fl=original,timestamp,statuscode&collapse=urlkey&filter=original:.*cross.country.*&filter=statuscode:200&limit=40",
        "curl -sS --max-time 120 -A '<UA>' \"<url>\" -o samples/wayback-cdx-misshsaa-xc.txt -w 'http=%{http_code} bytes=%{size_download}\\n'",
    ),
    "tn-champ-result-2025-file.cfm": (
        "https://tssaasports.com/event/file.cfm?championshipid=20258001&type=results",
        "curl (probe.sh)  ->  404 (control probe: bogus championship id)",
    ),
}

# local derivations (no network); command operates on an already-captured sample
DERIVED = {
    "ok-memberschools-2025-26.txt": "pdftotext -layout ok-memberschools-2025-26.pdf ok-memberschools-2025-26.txt",
    "la-2026-state-champ-results.txt": "pdftotext -layout la-2026-state-champ-results.pdf la-2026-state-champ-results.txt  (source of the 1749 result rows / 306 distinct school tokens in ../coverage.json LA.enumeration)",
    "la-coaches-directory-2025-26.txt": "pdftotext -layout la-coaches-directory-2025-26.pdf la-coaches-directory-2025-26.txt",
    "tx-alignment-26-28-alpha.txt": "pdftotext -layout tx-alignment-26-28-alpha.pdf tx-alignment-26-28-alpha.txt",
    "tx-alignment-26-28-rank.txt": "pdftotext -layout tx-alignment-26-28-rank.pdf tx-alignment-26-28-rank.txt",
    "tx-tf-school-codes-2026.txt": "pdftotext -layout tx-tf-school-codes-2026.pdf tx-tf-school-codes-2026.txt",
    "tn-champ-result-2025-d1aaa.pdf": "cp tn-champ-result-2025-d1aaa.cfm tn-champ-result-2025-d1aaa.pdf  (the .cfm response body is a PDF)",
    "tn-schools-extracted.tsv": "python3 - <<'PY' (regex over tn-portal-directory.html: source:[{id:'N',name:'X'}]) — see tools/extract_tn.py",
    "ky-schools-arbiterlive.tsv": "python3 - <<'PY' (regex over ky-member-school-directory.html: arbiterlive.com/Teams?entityId=N) — see tools/extract_ky.py",
    "al-milesplit-teams-ids.txt": "grep -o -E 'href=\"https://al\\.milesplit\\.com/teams/[0-9]+-[^\"]+\"' al-milesplit-teams.html | sed 's/.*teams\\///; s/\"//' | sort -u",
    "ms-milesplit-teams-ids.txt": "grep -o -E 'href=\"https://ms\\.milesplit\\.com/teams/[0-9]+-[^\"]+\"' ms-milesplit-teams.html | sed 's/.*teams\\///; s/\"//' | sort -u",
    "la-milesplit-teams-ids.txt": "grep -o -E 'href=\"https://la\\.milesplit\\.com/teams/[0-9]+-[^\"]+\"' la-milesplit-teams.html | sed 's/.*teams\\///; s/\"//' | sort -u",
}


def main() -> int:
    tsv = os.path.join(SAMPLES, ".captures.tsv")
    rows = []
    with open(tsv, encoding="utf-8") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line.strip():
                continue
            ts, code, nbytes, url, cmd = line.split("\t")
            rows.append((ts, code, nbytes, url, cmd))
    rows.sort()

    out = [
        "# Captures — `research/sources/state-assoc-southcentral/samples/`",
        "",
        "All captures are **anonymous** (`curl`, no cookies, no auth, no JS). Per-host spacing was",
        "≥1 s (requests were issued one at a time, ≥1.2 s apart per host); every host's `robots.txt`",
        "was fetched first and is preserved verbatim in this directory (see the per-state notes in",
        "`../SOURCE_REPORT.md`). `curl` writes the response body byte-exact to the listed sample file.",
        "",
        f"- User-Agent: `{UA}`",
        "- Harness: `../probe.sh <sample-name> <url>` → writes `samples/<sample-name>` and appends a row to `samples/.captures.tsv`.",
        "- `*.curlerr` sidecars hold each request's `curl` stderr (empty = clean transfer).",
        "",
        "## Network captures",
        "",
        "| UTC timestamp | HTTP | bytes | sample file | URL | exact command |",
        "|---|---|---|---|---|---|",
    ]
    for ts, code, nbytes, url, cmd in rows:
        m = re.search(r"samples/(\S+)$", cmd)
        name = m.group(1) if m else "?"
        shown = (f"curl -sS -o \"$out\" -w '%{{http_code}}' --max-time 40 -A \"$UA\" '{url}' "
                 f"2>\"$out.curlerr\"   [probe.sh <sample-name> '{url}']")
        if " -e " in cmd:
            shown += "   (+ referer header)"
        out.append(f"| {ts} | {code} | {nbytes} | `{name}` | `{url}` | `{shown}` |")

    covered = set(re.findall(r"samples/(\S+)", "\n".join(r[4] for r in rows)))
    missing = sorted(
        f for f in os.listdir(SAMPLES) if f not in covered and not f.endswith(".curlerr") and f != "CAPTURES.md"
    )

    out += ["", "## Captures issued directly with `curl` (not through the harness)", "", "| file | URL | command |", "|---|---|---|"]
    for name in missing:
        if name in EXTRA:
            url, cmd = EXTRA[name]
            out.append(f"| `{name}` | `{url}` | `{cmd.replace('<url>', url)}` |")
    out += ["", "## Derived files (produced locally from the captures above; no network)", "", "| file | command |", "|---|---|"]
    for name in missing:
        if name in DERIVED:
            out.append(f"| `{name}` | `{DERIVED[name]}` |")
    leftovers = [n for n in missing if n not in EXTRA and n not in DERIVED]
    if leftovers:
        out += ["", "## Unclassified files", ""]
        for n in leftovers:
            mt = subprocess.run(["stat", "-c", "%y", os.path.join(SAMPLES, n)], capture_output=True, text=True).stdout.strip()
            out.append(f"- `{n}` (mtime {mt})")
    out += [""]
    with open(os.path.join(SAMPLES, "CAPTURES.md"), "w", encoding="utf-8") as fh:
        fh.write("\n".join(out))
    print(f"rows={len(rows)} missing_classified={len([n for n in missing if n in EXTRA or n in DERIVED])} leftovers={leftovers}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
