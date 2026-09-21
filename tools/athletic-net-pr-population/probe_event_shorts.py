#!/usr/bin/env python3
import json, urllib.request, urllib.error, gzip

URL = "https://www.athletic.net/api/v1/tfRankings/GetRankings"
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

def fetch(event, page=1):
    body = {"reportType":"div","mode":"list","divListId":168416,"indoor":None,
            "eventShort":event,"gender":"m","qParams":{"grades":[11],"page":page},
            "qualifyingListKey":"","version":2,"debug":""}
    req = urllib.request.Request(URL, data=json.dumps(body).encode(), headers=HEADERS, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            raw = r.read()
    except urllib.error.HTTPError as e:
        return None, f"HTTP {e.code}", None
    if raw[:2] == b'\x1f\x8b':
        raw = gzip.decompress(raw)
    try:
        d = json.loads(raw)
    except Exception as ex:
        return None, f"parse error {ex}", None
    return d, "ok", None

for ev in ["shot", "lj", "4x100m", "distmed12,4,8,16"]:
    d, status, err = fetch(ev)
    if d is None:
        print(f"=== {ev}: {status} ===")
        continue
    gr = d.get("groupedRankings") or []
    nrows = sum(len(g) for g in gr if isinstance(g, list))
    rt = d.get("relayTeams")
    print(f"=== {ev} === status={status} minCount={d.get('minCount')} rows={nrows} relayTeams={'present len='+str(len(rt)) if rt else 'none'}")
    # first row
    r0 = None
    for g in gr:
        if isinstance(g, list) and g:
            r0 = g[0]; break
    if r0:
        for k in ["AthleteID","AthleteName","GradeID","TeamID","TeamName","State","Event","EventShort","display","SortIntRaw","PersonalBest","PersonalEvent","Measure"]:
            print(f"    {k}: {r0.get(k)!r}")
    if rt:
        print("    relayTeams[0]:", json.dumps(rt[0])[:300])
