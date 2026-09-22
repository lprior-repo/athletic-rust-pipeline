#!/usr/bin/env python3
"""Extract byte-exact Athletic.net samples for the athleticnet lane.

Reads the two retained browser HARs and the retained gap-closure evidence tree, and writes
`samples/*` next to this script. Only two transformations are applied:

  1. `jwtTFTopReport` values are replaced with `<redacted>` (never persist session tokens).
  2. nothing else: every other byte is the HAR `response.content.text` verbatim.

Run:  python3 extract-har-samples.py
"""

from __future__ import annotations

import base64
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
DOWNLOAD = pathlib.Path("/home/lewis/Downloads")
EVIDENCE = DOWNLOAD / "midwest-tfxc-source-research/research/midwest/evidence"

HAR1 = DOWNLOAD / "www.athletic.net.har"
HAR2 = DOWNLOAD / "www.athletic.net2.har"

# HAR entry index -> output file name. Indices are positional in `log.entries`.
HAR1_PICKS = {
    69: "har1-getbreadcrumbs-wi-170770.json",
    147: "har1-getrankings-wi-170770-m-multievent-n180.json",
    153: "har1-getagesgrades-wi-2026.json",
    154: "har1-getnavinfo-wi-2026-hs-boys.json",
    155: "har1-getstatescountries2.json",
    340: "har1-getstandards-empty.json",
    414: "har1-getrankings-wi-170770-m-100m-p2-n102.json",
    704: "har1-getrankings-wi-170770-m-100m-g11-p17-n17.json",
    718: "har1-getconvertevents.json",
}
HAR2_PICKS = {
    127: "har2-getathletebiodata-28872883.json",
    134: "har2-teamnav-3204.json",
    135: "har2-general-getrankings-28872883.json",
}
JWT = re.compile(rb'("jwtTFTopReport"\s*:\s*")[^"]*(")')

VERBATIM_COPIES = {
    "gap-closure/atn/athletebio-28872883.json": "live-getathletebiodata-28872883-2026-09-20.json",
    "gap-closure/atn/meetdata-634313.redacted.json": "live-meetdata-634313.redacted.json",
    "gap-closure/atn/eventdiv-634313.json": "live-eventdiv-634313.json",
    "gap-closure/atn/allresults-634313.redacted.json": "live-allresults-634313.redacted.json",
    "gap-closure/atn/resultsdata3-100m.json": "live-resultsdata3-100m.json",
    "gap-closure/atn/resultsdata3-100m-wind-615580.json": "live-resultsdata3-100m-wind-615580.json",
    "gap-closure/atn/resultsdata3-4x100m.json": "live-resultsdata3-4x100m.json",
    "gap-closure/atn/resultsdata3-shot-615580.json": "live-resultsdata3-shot-615580.json",
}


def content_bytes(entry: dict) -> bytes:
    content = entry["response"].get("content") or {}
    text = content.get("text") or ""
    if not text:
        return b""
    if content.get("encoding") == "base64":
        return base64.b64decode(text)
    return text.encode("utf-8")


def entries(path: pathlib.Path) -> list[dict]:
    return json.loads(path.read_text())["log"]["entries"]


def write_sample(name: str, payload: bytes) -> None:
    out = HERE / name
    out.write_bytes(payload)
    print(f"{len(payload):>9} B  {name}")


def write_request_log() -> None:
    """Every /api/v1/ request in both HARs: the audit behind the request-cost integers.

    `resp_bytes` is what the HAR retained; `content_size` is the size the browser reported
    for the response (so a missing body is distinguishable from an empty one).
    """
    lines = ["har\tentry\tmethod\tstatus\tresp_bytes\tcontent_size\tms\tstarted_utc\tpost_body\turl"]
    for har, rows in (("har1", entries(HAR1)), ("har2", entries(HAR2))):
        for index, entry in enumerate(rows):
            url = entry["request"]["url"]
            if "/api/v1/" not in url:
                continue
            body = (entry["request"].get("postData") or {}).get("text") or ""
            content = entry["response"].get("content") or {}
            lines.append("\t".join([
                har, str(index), entry["request"]["method"], str(entry["response"]["status"]),
                str(len(content_bytes(entry))), str(content.get("size")), str(round(entry.get("time") or 0)),
                entry["startedDateTime"], body.replace("\t", " "), url,
            ]))
    out = HERE / "har-atn-request-log.tsv"
    out.write_text("\n".join(lines) + "\n")
    print(f"          wrote {out.name} ({len(lines) - 1} api requests)")


def write_rankings_bodies() -> None:
    """The nine GetRankings POST bodies, with the page each one produced."""
    rows = []
    for index, entry in enumerate(entries(HAR1)):
        if not entry["request"]["url"].endswith("/api/v1/tfRankings/GetRankings"):
            continue
        body = (entry["request"].get("postData") or {}).get("text") or ""
        raw = content_bytes(entry)
        parsed = json.loads(raw) if raw else None
        groups = (parsed or {}).get("groupedRankings") or []
        rows.append({
            "har_entry": index,
            "started_utc": entry["startedDateTime"],
            "ms": round(entry.get("time") or 0),
            "status": entry["response"]["status"],
            "response_bytes_retained": len(raw),
            "response_content_size": (entry["response"].get("content") or {}).get("size"),
            "body": json.loads(body),
            "response_rows": sum(len(group) for group in groups),
            "response_min_count": (parsed or {}).get("minCount"),
            "response_depth": ((parsed or {}).get("settings") or {}).get("depth"),
        })
    out = HERE / "har1-getrankings-request-bodies.json"
    out.write_text(json.dumps(rows, indent=2) + "\n")
    print(f"          wrote {out.name} ({len(rows)} GetRankings calls)")


def main() -> int:
    for har, picks in ((HAR1, HAR1_PICKS), (HAR2, HAR2_PICKS)):
        rows = entries(har)
        for index, name in picks.items():
            raw = content_bytes(rows[index])
            redacted, count = JWT.subn(rb"\1<redacted>\2", raw)
            if count:
                print(f"          redacted {count} jwtTFTopReport value(s) in {name}")
            write_sample(name, redacted)
    for source, name in VERBATIM_COPIES.items():
        write_sample(name, (EVIDENCE / source).read_bytes())
    write_request_log()
    write_rankings_bodies()
    return 0


if __name__ == "__main__":
    sys.exit(main())
