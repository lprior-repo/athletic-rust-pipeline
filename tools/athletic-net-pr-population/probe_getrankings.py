#!/usr/bin/env python3
import json, urllib.request, urllib.error, gzip, io

URL = "https://www.athletic.net/api/v1/tfRankings/GetRankings"
BODY = {
    "reportType": "div", "mode": "list", "divListId": 168416,
    "indoor": None, "eventShort": "100m", "gender": "m",
    "qParams": {"grades": [11], "page": 1},
    "qualifyingListKey": "", "version": 2, "debug": "",
}
HEADERS = {
    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36",
    "Accept": "application/json, text/plain, */*",
    "Accept-Language": "en-US,en;q=0.9",
    "Content-Type": "application/json",
    "Origin": "https://www.athletic.net",
    "Referer": "https://www.athletic.net/TrackAndField/rankings/list/168416/m",
    "sec-ch-ua": '"Chromium";v="151", "Not=A?Brand";v="99"',
    "sec-ch-ua-mobile": "?0",
    "sec-ch-ua-platform": '"Linux"',
    "sec-fetch-dest": "empty",
    "sec-fetch-mode": "cors",
    "sec-fetch-site": "same-origin",
    "anet-appinfo": "web:web:0:300",
}

req = urllib.request.Request(URL, data=json.dumps(BODY).encode(), headers=HEADERS, method="POST")
try:
    with urllib.request.urlopen(req, timeout=30) as r:
        raw = r.read()
        status = r.status
        ct = r.headers.get("Content-Type")
except urllib.error.HTTPError as e:
    status = e.code
    raw = e.read()
    ct = e.headers.get("Content-Type")
    print("HTTPError", e.code, e.headers)
# handle gzip/br
if raw[:2] == b'\x1f\x8b':
    raw = gzip.decompress(raw)
print("STATUS:", status, "| content-type:", ct, "| bytes:", len(raw))
try:
    d = json.loads(raw)
except Exception as ex:
    print("JSON parse error:", ex)
    print(raw[:500])
    raise SystemExit
print("top keys:", sorted(d.keys()))
print("eventShort:", d.get("eventShort"), "| minCount:", d.get("minCount"), "| reportTitle:", d.get("reportTitle"))
gr = d.get("groupedRankings") or []
print("groupedRankings groups:", len(gr))
total_rows = 0
for g in gr:
    if isinstance(g, list):
        total_rows += len(g)
print("total rows:", total_rows)
# sample first row
def first_row():
    for g in gr:
        if isinstance(g, list):
            for row in g:
                if isinstance(row, dict):
                    return row
    return None
r0 = first_row()
if r0:
    print("--- first row (selected keys) ---")
    for k in ["AthleteID","AthleteName","GradeID","TeamID","TeamName","State","Event","EventShort","display","SortIntRaw","PersonalBest","SeasonID","ResultDate","MeetID","PersonalEvent"]:
        print(f"  {k}: {r0.get(k)!r}")
else:
    print("no rows; response sample:", json.dumps(d)[:800])
