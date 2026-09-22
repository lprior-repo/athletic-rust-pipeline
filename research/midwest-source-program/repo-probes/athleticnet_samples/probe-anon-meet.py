#!/usr/bin/env python3
"""Anonymous whole-meet pull probe against Athletic.net (3 requests, 1 req/s).

Question: can the 3-endpoint whole-meet lane (GetMeetData -> GetEventDivisionData
-> GetAllResultsData) run in a plain HTTPS client with NO browser session?
Method: browser-grade headers only; jwtMeet comes from the public GetMeetData
response. Raw bytes saved per request; tokens redacted before write.
"""
import json
import re
import time
import urllib.request
from pathlib import Path

OUT = Path(__file__).parent
MEET = "634313"
UA = ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
      "Chrome/140.0.0.0 Safari/537.36")


def hdrs(extra=None):
    h = {
        "User-Agent": UA,
        "Accept": "application/json, text/plain, */*",
        "Accept-Language": "en-US,en;q=0.9",
        "Referer": f"https://www.athletic.net/TrackAndField/meet/{MEET}/info",
        "sec-fetch-dest": "empty",
        "sec-fetch-mode": "cors",
        "sec-fetch-site": "same-origin",
    }
    if extra:
        h.update(extra)
    return h


def get(name, url, extra=None):
    req = urllib.request.Request(url, headers=hdrs(extra))
    with urllib.request.urlopen(req, timeout=45) as r:
        body = r.read()
        status = r.status
        hh = dict(r.headers)
    time.sleep(1.2)  # <=1 req/s per host
    (OUT / f"{name}.json").write_bytes(_redact(body))
    (OUT / f"{name}.headers").write_text(
        "\n".join(f"{k}: {'<redacted>' if k.lower() == 'set-cookie' else v}"
                  for k, v in sorted(hh.items())) + "\n")
    return status, body, hh


def _redact(b: bytes) -> bytes:
    """Blank JWT-looking values; keep structure. Tokens are never persisted."""
    s = b.decode("utf-8", "replace")
    s = re.sub(r'(jwtMeet|jwtTFTopReport|anettokens)"?\s*:\s*"eyJ[^"]*"',
               r'\1":"<redacted-jwt>"', s)
    s = re.sub(r'eyJ[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}',
               "<redacted-jwt>", s)
    return s.encode()


def main():
    rep = {}
    # 1. public: no auth
    st, body, _ = get("anon-meetdata-634313",
                      f"https://www.athletic.net/api/v1/Meet/GetMeetData?meetId={MEET}&sport=tf")
    md = json.loads(body)
    jwt = md.get("jwtMeet") or md.get("JwtMeet") or ""
    rep["1_GetMeetData"] = {
        "http": st, "bytes": len(body),
        "top_keys": sorted(md.keys()),
        "jwtMeet_present_in_response": bool(jwt),
        "jwtMeet_len": len(jwt),
        "LiveID": md.get("LiveID"),
        "HasResults": md.get("HasResults"),
        "eventDivsWithResults_len": len(md.get("eventDivsWithResults") or []),
        "tfDivisions_len": len(md.get("tfDivisions") or []),
    }
    if not jwt:
        rep["abort"] = "no jwtMeet minted anonymously -> remaining calls not evidence"
        (OUT / "anon-meet-probe-report.json").write_text(json.dumps(rep, indent=2) + "\n")
        print(json.dumps(rep, indent=2))
        return

    # 2. anettokens: jwtMeet (mint from step 1)
    st, body, _ = get("anon-eventdiv-634313",
                      f"https://www.athletic.net/api/v1/Meet/GetEventDivisionData?meetId={MEET}&sport=tf",
                      {"anettokens": jwt})
    ed = json.loads(body)
    rep["2_GetEventDivisionData"] = {
        "http": st, "bytes": len(body),
        "top_keys": sorted(ed.keys()) if isinstance(ed, dict) else f"list[{len(ed)}]",
        "events_len": len(ed.get("events") or []) if isinstance(ed, dict) else None,
    }
    # 3. whole meet
    st, body, _ = get(
        "anon-allresults-634313",
        f"https://www.athletic.net/api/v1/Meet/GetAllResultsData?meetId={MEET}&sport=tf"
        "&rawResults=false&showTips=false",
        {"anettokens": jwt})
    ar = json.loads(body)
    flat = ar.get("flatEvents") or []
    rows = [r for e in flat for r in (e.get("results") or [])]
    legs = ar.get("relayLegs") or []
    rep["3_GetAllResultsData"] = {
        "http": st, "bytes": len(body),
        "top_keys": sorted(ar.keys()),
        "flatEvents_len": len(flat),
        "result_rows": len(rows),
        "relayLegs_len": len(legs),
        "teams_len": len(ar.get("teams") or []),
        "eventTypes_len": len(ar.get("eventTypes") or []),
        "rounds_len": len(ar.get("rounds") or []),
    }
    rep["request_headers"] = {k: v for k, v in hdrs().items()}
    (OUT / "anon-meet-probe-report.json").write_text(json.dumps(rep, indent=2) + "\n")
    print(json.dumps(rep, indent=2))


if __name__ == "__main__":
    main()
