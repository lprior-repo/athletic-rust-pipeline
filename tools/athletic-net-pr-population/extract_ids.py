#!/usr/bin/env python3
import zipfile, json, urllib.request, gzip, time, random
from xml.etree import ElementTree as ET

WB = "/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes-US-Only.xlsx"
NS = {'m': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}

z = zipfile.ZipFile(WB)
root = ET.fromstring(z.read('xl/worksheets/sheet1.xml'))
rows = root.findall('.//m:sheetData/m:row', NS)
def cv(c):
    v = c.find('m:v', NS)
    return v.text if v is not None else ''
hdr = [cv(c) for c in rows[0].findall('m:c', NS)]
ci = hdr.index('AthleteID')
ids = []
for row in rows[1:]:
    cells = row.findall('m:c', NS)
    vals = [cv(c) for c in cells]
    if len(vals) > ci and vals[ci]:
        ids.append(vals[ci])
print(f"header: {hdr}")
print(f"athlete count: {len(ids)}  first: {ids[:3]}  last: {ids[-3:]}")
json.dump(ids, open('/tmp/athlete_ids.json', 'w'))
print("saved /tmp/athlete_ids.json")

HEADERS = {
    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36",
    "Accept": "application/json, text/plain, */*", "Accept-Language": "en-US,en;q=0.9",
    "Referer": "https://www.athletic.net/athlete/1/track-and-field/all",
    "sec-ch-ua": '"Chromium";v="151", "Not=A?Brand";v="99"', "sec-ch-ua-mobile": "?0",
    "sec-ch-ua-platform": '"Linux"', "sec-fetch-dest": "empty", "sec-fetch-mode": "cors",
    "sec-fetch-site": "same-origin", "anet-appinfo": "web:web:0:300",
}
def bio(aid):
    url = f"https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId={aid}&sport=tf&level=0"
    req = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(req, timeout=30) as r:
        raw = r.read()
    if raw[:2] == b'\x1f\x8b':
        raw = gzip.decompress(raw)
    return json.loads(raw)

# sample 8 random athletes
sample = random.sample(ids, 8)
for aid in sample:
    try:
        d = bio(aid)
        rtf = d.get('resultsTF') or []
        ath = d.get('athlete') or {}
        s = json.dumps(d)
        print(f"aid={aid}: resultsTF={len(rtf)} name={ath.get('FirstName')} {ath.get('LastName')} school={ath.get('SchoolID')} claimed={ath.get('isClaimed')} blurred={s.count('Xxxxxx')}")
    except Exception as e:
        print(f"aid={aid}: ERROR {e!r}")
    time.sleep(0.4)
