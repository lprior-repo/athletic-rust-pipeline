#!/usr/bin/env python3
import json, urllib.request, gzip, time
URL = "https://www.athletic.net/api/v1/tfRankings/GetRankings"
HEADERS = {
    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36",
    "Accept": "application/json, text/plain, */*", "Accept-Language": "en-US,en;q=0.9",
    "Content-Type": "application/json", "Origin": "https://www.athletic.net",
    "Referer": "https://www.athletic.net/TrackAndField/rankings/list/168416/m",
    "sec-ch-ua": '"Chromium";v="151", "Not=A?Brand";v="99"', "sec-ch-ua-mobile": "?0",
    "sec-ch-ua-platform": '"Linux"', "sec-fetch-dest": "empty", "sec-fetch-mode": "cors",
    "sec-fetch-site": "same-origin", "anet-appinfo": "web:web:0:300",
}
def probe(list_id, ev, grades):
    body = {"reportType":"div","mode":"list","divListId":list_id,"indoor":None,"eventShort":ev,
            "gender":"m","qParams":{"grades":grades,"page":1},"qualifyingListKey":"","version":2,"debug":""}
    req = urllib.request.Request(URL, data=json.dumps(body).encode(), headers=HEADERS, method="POST")
    with urllib.request.urlopen(req, timeout=30) as r:
        raw = r.read()
    if raw[:2] == b'\x1f\x8b': raw = gzip.decompress(raw)
    d = json.loads(raw)
    gr = d.get("groupedRankings") or []
    return sum(len(g) for g in gr if isinstance(g, list))

tests = ["100m","1600m","3200m","shot","decathlon","outdoor pentathlon","60m","55m","weight","1mile","300m","1000m","60mh","indoor pentathlon","throws pentathlon","2miles","3000m","hammer"]
print(f"{'event':18} {'168416(out)':>12} {'173005(ind)':>12}")
for ev in tests:
    r_out = probe(168416, ev, [11])
    time.sleep(0.35)
    r_ind = probe(173005, ev, [11])
    time.sleep(0.35)
    print(f"{ev:18} {r_out:>12} {r_ind:>12}")
