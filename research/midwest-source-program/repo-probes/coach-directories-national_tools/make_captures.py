#!/usr/bin/env python3
"""Generate samples/CAPTURES.md from the two fetch logs.

One line per captured file: URL, HTTP status, bytes, UTC timestamp, exact command.
Refused URLs (robots.txt disallow) are listed in their own section - they are
evidence of NOT fetching, which the contract requires.
"""
from __future__ import annotations

import json
from pathlib import Path

LANE = Path(__file__).resolve().parent.parent
LOGS = [LANE / "tools" / "fetch-log.jsonl", LANE / "tools" / "fetch-log-manual.jsonl"]
PRE_EXISTING = ["ghsa-school-directory.html", "nchsaa-schools.html", "nchsaa-ad-directory.html"]


def load() -> tuple[dict[str, dict], list[dict], list[dict]]:
    fetches: dict[str, dict] = {}
    robots, refused = [], []
    for lg in LOGS:
        if not lg.exists():
            continue
        for line in lg.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            r = json.loads(line)
            if r.get("kind") == "fetch" and r.get("name"):
                fetches[r["name"]] = r
            elif r.get("kind") == "robots":
                robots.append(r)
            elif r.get("kind") == "refused":
                refused.append(r)
    return fetches, robots, refused


def main() -> int:
    fetches, robots, refused = load()
    have = {str(p.relative_to(LANE / "samples")) for p in (LANE / "samples").rglob("*") if p.is_file()}
    lines = [
        "# Raw captures — provenance",
        "",
        f"{len(fetches)} logged fetches across "
        f"{len({(r.get('host') or r.get('url', '').split('/')[2]) for r in fetches.values()})} hosts; "
        f"every file below is byte-exact as received (curl, no post-processing). "
        "User-Agent for all logged fetches: `omp-research/1.0 (public coach-directory research; contact: research@example.invalid)`, "
        ">=1.05 s between requests to the same host, `robots.txt` fetched and honoured first for every host "
        "(see `tools/robots/<host>.txt` and the Refusals section).",
        "",
        "| # | sample file | URL | HTTP | wire bytes | on-disk bytes | captured (UTC) | command |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for i, name in enumerate(sorted(fetches), 1):
        r = fetches[name]
        cmd = (r.get("command") or "").replace("|", "\\|")
        url = (r.get("url") or "").replace("|", "\\|")
        disk = LANE / "samples" / name
        disk_sz = disk.stat().st_size if disk.exists() else "MISSING"
        lines.append(f"| {i} | `samples/{name}` | {url} | {r.get('status')} | {r.get('bytes')} | {disk_sz} | "
                     f"{r.get('ts')} | `{cmd}` |")
    lines += ["", "**Column note.** `wire bytes` is curl's `%{size_download}` - bytes on the wire. Every logged "
              "fetch used `--compressed`, so for gzip/brotli responses it is SMALLER than the on-disk file, which "
              "holds the decompressed body exactly as received. Where the two differ the body is still complete; "
              "the difference is transport encoding only. All field counts in `coverage.json` and "
              "`SOURCE_REPORT.md` are computed over the on-disk body."]
    missing = sorted(have - set(fetches))
    if missing:
        lines += ["", "## Files present without a fetch-log record", "",
                  "These were not fetched by this lane's harness. They are listed so the gap is explicit; "
                  "no URL/status is claimed for them.", ""]
        for m in missing:
            tag = "pre-existing in lane before this session; provenance not established by me" \
                if m in PRE_EXISTING else "written by a helper outside the logged harness"
            lines.append(f"- `samples/{m}` — {tag}")
    lines += ["", "## robots.txt refusals (evidence of NOT fetching)", "",
              "| URL | host | matching rule | robots.txt HTTP |", "|---|---|---|---|"]
    for r in refused:
        lines.append(f"| {r.get('url')} | {r.get('host')} | `{r.get('robots_rule') or '(Disallow: / for user-agent *)'}` | "
                     f"{r.get('robots_status')} |")
    lines += ["", "## robots.txt recorded per host", "",
              "| host | robots.txt URL | HTTP | bytes | crawl-delay | allowed our path | matching Disallow |",
              "|---|---|---|---|---|---|---|"]
    seen = set()
    for r in sorted(robots, key=lambda x: str(x.get("host"))):
        if r.get("host") in seen:
            continue
        seen.add(r["host"])
        rule = r.get("rule") or ""
        if not rule and r.get("allow") is False:
            rule = "Disallow: / (blanket group for user-agent *)"
        if not rule and r.get("status") in (0, 403, 404):
            rule = f"(no robots.txt served: HTTP {r.get('status')}) - defaulted to allow"
        lines.append(f"| {r.get('host')} | {r.get('url')} | {r.get('status')} | {r.get('bytes')} | "
                     f"{r.get('crawl_delay')} | {r.get('allow')} | `{rule}` |")
    (LANE / "samples" / "CAPTURES.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote samples/CAPTURES.md: {len(fetches)} fetches, {len(refused)} refusals, "
          f"{len(seen)} robots hosts, {len(missing)} unlogged files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
