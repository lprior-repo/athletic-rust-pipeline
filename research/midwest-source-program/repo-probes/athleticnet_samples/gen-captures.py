#!/usr/bin/env python3
"""Emit samples/CAPTURES.md from the files actually present in samples/.

Sources of truth, in order:
  1. live captures made by this lane  -> URL + command from the table below
  2. HAR extracts (extract-har-samples.py) -> entry url from samples/har-atn-request-log.tsv
  3. copies of the retained corpus evidence tree -> original corpus path in the table below
"""
import csv
import datetime as dt
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).parent
ROOT = HERE.parent
CORPUS = Path.home() / "Downloads/midwest-tfxc-source-research"

UA = ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
      "Chrome/140.0.0.0 Safari/537.36")
C = 'curl -sS -A "%s" -H "Accept: application/json, text/plain, */*" ' % UA
REF = 'https://www.athletic.net'

# file -> (url, command, http, note)
LIVE = {
    "atn-robots.txt": (f"{REF}/robots.txt", 'bash refetch.sh   (curl -sS -A "<Chrome UA>" -D <f>.headers -o <f> <url>)', 200,
                       "no sitemap line; no Crawl-delay; last-modified 2026-03-15"),
    "atn-robots.headers": (f"{REF}/robots.txt", "same request as atn-robots.txt", 200, "response headers"),
    "atn-probe-statescountries-anon.json": (f"{REF}/api/v1/public/GetStatesCountries2", C + '-H "Referer: {ref}/TrackAndField/rankings" -o <f> "<url>"', 200,
                                            "65 US + 14 CA rows, 219 countries, 249 countriesLookup"),
    "atn-probe-statescountries-anon.headers": (f"{REF}/api/v1/public/GetStatesCountries2", "same request", 200, "response headers"),
    "atn-probe-divchildren-170770.json": (f"{REF}/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170770", C + '-H "Referer: {ref}/TrackAndField/rankings/list/170770" -o <f> "<url>"', 200,
                                          "WI state node tree: 3 class divisions + 24 child rows"),
    "atn-probe-divchildren-170771.json": (f"{REF}/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170771", "same shape as above, divId=170771", 200, "sectional -> 4 regional children"),
    "atn-probe-divchildren-170772.json": (f"{REF}/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170772", "same shape as above, divId=170772", 200, "smallest node (regionals)"),
    "atn-probe-divchildren-170305.json": (f"{REF}/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170305", "same shape as above, divId=170305", 200, "TX state node; teams[] 332"),
    "atn-probe-divchildren-168416.json": (f"{REF}/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=168416", "same shape as above, divId=168416", 200, "CA state node"),
    "atn-probe-divchildren-168546.json": (f"{REF}/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=168546", "same shape as above, divId=168546", 200, "CA child node"),
    "atn-probe-divchildren-170770-page2.json": (f"{REF}/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170770&page=2", "same shape as above, divId=170770&page=2", 200,
                                                "byte-identical to page 1 -> the page parameter is ignored"),
    "atn-probe-getrankings-anon-nopage.json": (f"{REF}/api/v1/tfRankings/GetRankings", C + ' -H "Content-Type: application/json" --data @<body> -o <f> "<url>"  (body with NO qParams.page key)', 200,
                                            "identical body to the page=1 request -> the field is optional and ignored"),
    "atn-probe-getrankings-anon.json": (f"{REF}/api/v1/tfRankings/GetRankings", C + ' -H "Content-Type: application/json" --data @<body> -o <f> "<url>"  (body: qParams.grades=[], page=1, eventShort=100m)', 200,
                                        "anonymous tier: 101 rows, blurAfterDepth 5; jwtTFTopReport server-issued (redacted)"),
    "atn-probe-getrankings-anon-g11-p2.headers": (f"{REF}/api/v1/tfRankings/GetRankings", "same request, qParams.grades=[11], page=2", 200, "response headers"),
    "atn-probe-getrankings-anon-g11-p2.json": (f"{REF}/api/v1/tfRankings/GetRankings", "same request, qParams.grades=[11], page=2", 200,
                                              "byte-identical body to page=2 vs page=1 -> page ignored"),
    "atn-probe-getrankings-anon.headers": (f"{REF}/api/v1/tfRankings/GetRankings", "same request as atn-probe-getrankings-anon.json", 200, "response headers"),
    "atn-probe-events-wisconsin-2026-5-26.html": (f"{REF}/events/usa/wisconsin/2026-5-26", C.replace("application/json, text/plain, */*", "text/html,application/xhtml+xml") + ' -o <f> "<url>"', 200,
                                                  "7,430 B SPA shell, 0 server-rendered meet links"),
    "atn-probe-events-wisconsin-2026-5-26.headers": (f"{REF}/events/usa/wisconsin/2026-5-26", "same request", 200, "response headers"),
    "anon-meetdata-634313.json": (f"{REF}/api/v1/Meet/GetMeetData?meetId=634313&sport=tf", "python3 probe-anon-meet.py", 200,
                                  "public response mints jwtMeet (255 chars, value redacted); eventDivsWithResults 45"),
    "anon-meetdata-634313.headers": (f"{REF}/api/v1/Meet/GetMeetData?meetId=634313&sport=tf", "python3 probe-anon-meet.py", 200, "response headers"),
    "anon-eventdiv-634313.json": (f"{REF}/api/v1/Meet/GetEventDivisionData?meetId=634313&sport=tf", "python3 probe-anon-meet.py   (anettokens = jwtMeet from GetMeetData)", 200, "36 events"),
    "anon-eventdiv-634313.headers": (f"{REF}/api/v1/Meet/GetEventDivisionData?meetId=634313&sport=tf", "python3 probe-anon-meet.py", 200, "response headers"),
    "anon-allresults-634313.json": (f"{REF}/api/v1/Meet/GetAllResultsData?meetId=634313&sport=tf&rawResults=false&showTips=false", "python3 probe-anon-meet.py", 200,
                                    "758 result rows, 288 relay legs, 12 teams, 45 eventDivsWithResults"),
    "anon-allresults-634313.headers": (f"{REF}/api/v1/Meet/GetAllResultsData?meetId=634313&sport=tf&rawResults=false&showTips=false", "python3 probe-anon-meet.py", 200, "response headers"),
    "anon-meet-probe-report.json": ("(derived)", "python3 probe-anon-meet.py", None, "machine-readable result of the 3-request anonymous whole-meet pull"),
}

# HAR-extracted files: url + status + time come from the request log
HAR = {
    "har1-getnavinfo-wi-2026-hs-boys.json", "har1-getagesgrades-wi-2026.json",
    "har1-getbreadcrumbs-wi-170770.json", "har1-getconvertevents.json",
    "har1-getstandards-empty.json", "har1-getstatescountries2.json",
    "har1-getrankings-request-bodies.json", "har1-getrankings-wi-170770-m-100m-p2-n102.json",
    "har1-getrankings-wi-170770-m-100m-g11-p17-n17.json",
    "har1-getrankings-wi-170770-m-multievent-n180.json",
    "har2-general-getrankings-28872883.json", "har2-getathletebiodata-28872883.json",
    "har2-teamnav-3204.json",
}

# Copies of the retained token-based corpus captures (not re-fetched by this lane)
COPIED = {
    "live-meetdata-634313.redacted.json": "research/midwest/evidence/gap-closure/atn/meetdata-634313.redacted.json",
    "live-eventdiv-634313.json": "research/midwest/evidence/gap-closure/atn/eventdiv-634313.json",
    "live-allresults-634313.redacted.json": "research/midwest/evidence/gap-closure/atn/allresults-634313.redacted.json",
    "live-resultsdata3-100m.json": "research/midwest/evidence/gap-closure/atn/resultsdata3-100m.json",
    "live-resultsdata3-100m-wind-615580.json": "research/midwest/evidence/gap-closure/atn/resultsdata3-100m-wind-615580.json",
    "live-resultsdata3-4x100m.json": "research/midwest/evidence/gap-closure/atn/resultsdata3-4x100m.json",
    "live-resultsdata3-shot-615580.json": "research/midwest/evidence/gap-closure/atn/resultsdata3-shot-615580.json",
    "live-getathletebiodata-28872883-2026-09-20.json": "research/midwest/evidence/gap-closure/atn/athletebio-28872883.json",
}

ARTIFACTS = {
    "build-schema.py": "generates ../schema.json from the sample bytes",
    "checks-enum-crosscheck.py": "asserts the 51 nav state nodes map to census-domain's UsJurisdiction enum; writes checks-state-divs.tsv",
    "checks-state-divs.tsv": "output of checks-enum-crosscheck.py (52 rows)",
    "extract-har-samples.py": "extracts the HTTP Archive entries into har1-*/har2-* files; writes har-atn-request-log.tsv",
    "redact-samples.py": "redacts jwtMeet/jwtTFTopReport/cookie values from every sample in place",
    "probe-anon-meet.py": "the 3-request anonymous whole-meet probe",
    "refetch.sh": "the exact curl command for every live sample (verified byte-identical on re-run, see below)",
    "har-atn-request-log.tsv": "URL + status + timestamp for every request in the two retained HARs",
    "gen-captures.py": "generates this file (CAPTURES.md) from the bytes actually present",
    "checks-enum-crosscheck.txt": "console output of checks-enum-crosscheck.py (51/51 enum match, Overseas out of scope)",
}

har_rows = []
log_path = HERE / "har-atn-request-log.tsv"
if log_path.exists():
    with log_path.open() as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            row["resp_bytes"] = int(row.get("resp_bytes") or 0)
            har_rows.append(row)

# file -> (distinctive token in the entry URL, exact response size in the log)
HAR_MATCH = {
    "har1-getnavinfo-wi-2026-hs-boys.json": ("tfRankings/GetNavInfo", 78468),
    "har1-getagesgrades-wi-2026.json": ("tfRankings/GetAgesGrades", 1080),
    "har1-getbreadcrumbs-wi-170770.json": ("SiteHeader/GetBreadcrumbs", 1456),
    "har1-getconvertevents.json": ("tfRankings/GetConvertEvents", 23613),
    "har1-getstandards-empty.json": ("tfRankings/GetStandards", 2),
    "har1-getstatescountries2.json": ("public/GetStatesCountries2", 34159),
    "har1-getrankings-wi-170770-m-100m-p2-n102.json": ("tfRankings/GetRankings", 89113),
    "har1-getrankings-wi-170770-m-100m-g11-p17-n17.json": ("tfRankings/GetRankings", 16709),
    "har1-getrankings-wi-170770-m-multievent-n180.json": ("tfRankings/GetRankings", 191003),
    "har2-general-getrankings-28872883.json": ("General/GetRankings", 6122),
    "har2-getathletebiodata-28872883.json": ("AthleteBio/GetAthleteBioData", 30344),
    "har2-teamnav-3204.json": ("TeamNav/Team", 1222),
}


def har_row_for(name):
    tok, size = HAR_MATCH.get(name, (None, None))
    if not tok:
        return None
    for row in har_rows:
        if tok in row.get("url", "") and row.get("resp_bytes") == size:
            return row
    return None


def meta(p: Path):
    b = p.read_bytes()
    ts = dt.datetime.fromtimestamp(p.stat().st_mtime, dt.UTC).strftime("%Y-%m-%d %H:%M:%SZ")
    return len(b), ts, hashlib.sha256(b).hexdigest()


def main():
    lines = [
        "# Athletic.net lane - sample captures",
        "",
        f"All live requests in this directory were made from this workstation on **2026-09-21/22 (UTC)** at",
        "**>=1.2 s spacing (<=1 req/s/host)**, with a desktop-Chrome user agent and no cookie reuse across",
        "samples. Tokens and cookies are redacted in place (`redact-samples.py`); never persisted.",
        "",
        "Two provenance classes: **live** (this lane's own requests) and **copied** (retained corpus captures,",
        "re-used instead of re-fetched; the original path is given per file).",
        "",
        "The **Bytes** column is the on-disk size (tokens already redacted); the un-redacted HTTP body is",
        "larger by the redaction delta, which `probe-anon-meet.py` prints as `bytes` in",
        "`anon-meet-probe-report.json` (e.g. GetMeetData: 16,644 B over the wire vs 16,403 B on disk, delta =",
        "the 255-char `jwtMeet` replaced by a fixed placeholder). Where a byte count is quoted in",
        "`../SOURCE_REPORT.md`, the wire value is used and the sample it came from is named.",
        "",
        "## Live captures",
        "",
        "| File | URL | HTTP | Bytes | Captured (UTC) | sha256[:16] | Command |",
        "|---|---|---|---|---|---|---|",
    ]
    for name in sorted(LIVE):
        p = HERE / name
        if not p.exists():
            continue
        url, cmd, http, _ = LIVE[name]
        n, ts, h = meta(p)
        lines.append(f"| `{name}` | `{url}` | {http if http else '-'} | {n} | {ts} | `{h[:16]}` | `{cmd}` |")

    lines += ["", "## HAR extracts (from the two retained sessions, not re-fetched)", "",
              "| File | URL | HTTP | Bytes | Extracted (UTC) | sha256[:16] | Command |",
              "|---|---|---|---|---|---|---|"]
    for name in sorted(HAR):
        p = HERE / name
        if not p.exists():
            continue
        n, ts, h = meta(p)
        row = har_row_for(name)
        if name == "har1-getrankings-request-bodies.json":
            url = "(derived) the 8 GetRankings POST bodies captured in HAR #1"
            st = "-"
        else:
            url = (row or {}).get("url", "see har-atn-request-log.tsv")
            st = (row or {}).get("status", "-")
        lines.append(f"| `{name}` | `{url}` | {st} | {n} | {ts} | `{h[:16]}` | `python3 extract-har-samples.py` |")

    lines += ["", "## Copied from the retained corpus evidence tree", "",
              "| File | Original path | Bytes | sha256[:16] |",
              "|---|---|---|---|"]
    for name, src in sorted(COPIED.items()):
        p = HERE / name
        if not p.exists():
            continue
        n, _, h = meta(p)
        lines.append(f"| `{name}` | `~/Downloads/midwest-tfxc-source-research/{src}` | {n} | `{h[:16]}` |")

    lines += ["", "## Scripts and derived files", "", "| File | Purpose |", "|---|---|"]
    for name, why in sorted(ARTIFACTS.items()):
        lines.append(f"| `{name}` | {why} |")

    lines += [
        "",
        "## Reproduction check (run 2026-09-21 23:11-23:12 UTC)",
        "",
        "`bash refetch.sh /tmp/atn-refetch` re-issued every live request; bodies were compared by sha256 against",
        "the stored samples:",
        "",
        "* **byte-identical** - `atn-robots.txt`, `atn-probe-statescountries-anon.json`, all six",
        "  `atn-probe-divchildren-*.json`, and the 3 whole-meet bodies (same 16,644 / 8,341 / 506,882 B, same",
        "  758/288/12/45 counts on the third run);",
        "* **differ, explained** - `atn-probe-events-wisconsin-2026-5-26.html` (+361 B: Cloudflare injects a",
        "  `beacon.min.js` tag and a per-response nonce), `atn-probe-getrankings-anon*.json` (+296 B each: the",
        "  server-issued `jwtTFTopReport` is present in the fresh body and was replaced by the redaction",
        "  placeholder in the stored one - the only differing JSON path in either file).",
        "",
        "The scratch directory holding the un-redacted re-fetch was deleted immediately (`rm -rf /tmp/atn-refetch`).",
        "",
        "## Request ledger",
        "",
        "The reproduction check issued exactly **14** live requests (11 from `refetch.sh` + 3 from",
        "`probe-anon-meet.py`). This directory holds 58 files (plus a `__pycache__/` bytecode dir): **17 live response bodies + 9 header files**",
        "(`anon-*`, `atn-probe-*`, `atn-robots.*`), **13 HAR extracts + 1 request log**, **8 copies** of the",
        "retained corpus captures, **7 scripts**, and 3 derived files (`CAPTURES.md`, `checks-state-divs.tsv`,",
        "`checks-enum-crosscheck.txt`). No 429, no `Retry-After`, no CAPTCHA, no login. `robots.txt` was read",
        "before the first request in every session.",
        "",
        "## Provenance notes",
        "",
        f"* The two HARs live outside the repo at `{Path.home()}/Downloads/www.athletic.net{{,2}}.har`",
        "  (722 + 167 entries, captured 2026-09-18 by the main agent); every `har*` file here is an extract.",
        "* `atn-probe-divchildren-170770-page2.json` is byte-identical to `atn-probe-divchildren-170770.json`:",
        "  the `page` parameter is ignored by `GetDivChildren`.",
        "* The `*.headers` files carry the response headers verbatim with `set-cookie` redacted; they are the",
        "  evidence for the rate-limit and Cloudflare claims in `../SOURCE_REPORT.md` §17.",
        "* Nothing in this directory contains a token, cookie, email or personal contact value.",
    ]
    (HERE / "CAPTURES.md").write_text("\n".join(lines) + "\n")
    print(f"wrote {HERE / 'CAPTURES.md'} ({len(lines)} lines)")
    print("har log rows:", len(har_rows))
    print("unmatched HAR files:", [n for n in sorted(HAR) if har_row_for(n) is None])
    print("files not classified:", sorted(
        p.name for p in HERE.iterdir()
        if p.is_file() and p.name not in LIVE and p.name not in HAR and p.name not in COPIED
        and p.name not in ARTIFACTS and p.name not in {"CAPTURES.md"} and not p.name.endswith(".headers")
        and p.name != "__pycache__"))


if __name__ == "__main__":
    main()
