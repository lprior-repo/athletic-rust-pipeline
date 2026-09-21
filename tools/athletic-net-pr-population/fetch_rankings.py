#!/usr/bin/env python3
"""Fetch grade-11 boys 2026 national (list 168416) rankings PRs for all 74 event shorts.

Polite throttle ~1.2s + jitter, exponential backoff on 429/5xx, checkpointed resume.
Output: JSONL lines of (athlete, event, mark, team) records + done-set + state.
"""
import json, urllib.request, urllib.error, gzip, time, random, os, sys, re

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

# (event_short, workbook_count) — individual first (size desc), then relay (size desc)
INDIVIDUAL = [
    ("100m", 50791), ("200m", 45767), ("400m", 33565), ("800m", 28873),
    ("shot", 26620), ("1600m", 24855), ("discus", 24776), ("lj", 23571),
    ("3200m", 14518), ("hj", 11341), ("tj", 9959), ("300mh", 9831),
    ("110mh", 9659), ("javelin", 8603), ("pv", 6149), ("1mile", 2640),
    ("1500m", 2090), ("400mh", 1937), ("3000m", 905), ("60m", 755),
    ("2miles", 532), ("300m", 401), ("2ksteeple", 400), ("3ksteeple", 318),
    ("600m", 305), ("hammer", 287), ("decathlon", 198), ("outdoor pentathlon", 174),
    ("60mh", 147), ("1000m", 84), ("55m", 63), ("weight", 54),
    ("100mh", 19), ("55mh", 13), ("500m", 10), ("throws pentathlon", 24),
    ("heptathlon", 1), ("indoor pentathlon", 3),
]
RELAY = [
    ("4x100m", 14133), ("4x400m", 13112), ("4x200m", 9690), ("4x800m", 9623),
    ("4x1600m", 978), ("4xmile", 87),
    ("distmed12,4,8,16", 3230), ("distmed2,2,4,8", 289), ("distmed4,4,8,16", 45),
    ("distmed4,6,8,10", 37), ("distmed10,2,4,8", 25), ("distmed8,2,4,16", 13),
    ("distmed4,8,12,12", 5), ("distmed13,4,8,mile", 4), ("distmed8,8,16,16", 1),
    ("sprintmed2248", 2696), ("sprintmed1124", 2598), ("sprintmed4224", 46),
    ("sprintmed600", 11), ("sprintmed400", 9), ("sprintmed6248", 7),
    ("sprintmed880", 4), ("sprintmed1400", 1),
    ("swedish1234", 281),
    ("110shuttleh", 569), ("100shuttleh", 36), ("55shuttleh", 33),
    ("120yshuttleh", 22), ("160mshuttleh", 11), ("70shuttleh", 9),
    ("60shuttleh", 8), ("110shuttlehd", 7), ("80yshuttleh", 2),
    ("102.5shuttleh", 1), ("300shuttleh", 1), ("65shuttleh", 1),
]

OUT = "/tmp/rankings_rows.jsonl"
DONE = "/tmp/rankings_done.txt"
STATE = "/tmp/rankings_state.json"

def log(*a):
    print(f"[{time.strftime('%H:%M:%S')}]", *a, file=sys.stderr, flush=True)

def load_done():
    s = set()
    if os.path.exists(DONE):
        with open(DONE) as f:
            for line in f:
                line = line.strip()
                if line:
                    s.add(line)
    return s

def load_state():
    if os.path.exists(STATE):
        with open(STATE) as f:
            return json.load(f)
    return {}

def save_state(st):
    with open(STATE, "w") as f:
        json.dump(st, f)

def fetch(event, grades, page):
    body = {"reportType": "div", "mode": "list", "divListId": 168416, "indoor": None,
            "eventShort": event, "gender": "m",
            "qParams": {"grades": grades, "page": page},
            "qualifyingListKey": "", "version": 2, "debug": ""}
    data = json.dumps(body).encode()
    last_err = None
    for attempt in range(7):
        try:
            req = urllib.request.Request(URL, data=data, headers=HEADERS, method="POST")
            with urllib.request.urlopen(req, timeout=40) as r:
                raw = r.read()
            if raw[:2] == b'\x1f\x8b':
                raw = gzip.decompress(raw)
            return json.loads(raw), None
        except urllib.error.HTTPError as e:
            raw = e.read()
            if raw[:2] == b'\x1f\x8b':
                raw = gzip.decompress(raw)
            if e.code == 429:
                ra = e.headers.get("Retry-After")
                wait = float(ra) if ra and ra.replace('.', '', 1).isdigit() else min(60.0, 5.0 * (2 ** attempt))
                log(f"429 on {event} p{page} (attempt {attempt+1}); sleeping {wait:.0f}s")
                time.sleep(wait)
                last_err = f"429"
                continue
            if e.code >= 500:
                wait = min(60.0, 5.0 * (2 ** attempt))
                log(f"{e.code} on {event} p{page} (attempt {attempt+1}); sleeping {wait:.0f}s")
                time.sleep(wait)
                last_err = f"HTTP {e.code}"
                continue
            # 4xx other than 429 -> fatal for this request
            log(f"HTTP {e.code} on {event} p{page}: {raw[:200]!r}")
            return None, f"HTTP {e.code}"
        except urllib.error.URLError as e:
            wait = min(60.0, 5.0 * (2 ** attempt))
            log(f"URLError on {event} p{page}: {e!r}; sleeping {wait:.0f}s")
            time.sleep(wait)
            last_err = f"URLError {e!r}"
        except Exception as e:
            log(f"Exception on {event} p{page}: {e!r}; sleeping 5s")
            time.sleep(5)
            last_err = f"Exception {e!r}"
    return None, last_err or "unknown"

def throttle():
    time.sleep(1.2 + random.uniform(0.0, 0.4))

def main():
    done = load_done()
    state = load_state()
    out_f = open(OUT, "a")
    done_f = open(DONE, "a")
    rows_written = 0
    reqs = 0
    t0 = time.time()

    # order: individual then relay
    plan = [(ev, "ind", c) for ev, c in INDIVIDUAL] + [(ev, "rel", c) for ev, c in RELAY]

    for ev, kind, wb_count in plan:
        grades = [] if kind == "rel" else [11]
        page = state.get(ev, {}).get("next_page", 1)
        total = state.get(ev, {}).get("rows", 0)
        if page is None:
            log(f"SKIP {ev}: already complete ({total} rows)")
            continue
        if page == 1:
            log(f"START {ev} ({kind}, workbook={wb_count})")
        else:
            log(f"RESUME {ev} page {page} ({kind})")
        while True:
            key = f"{ev}|{page}"
            if key in done:
                page += 1
                continue
            d, err = fetch(ev, grades, page)
            reqs += 1
            if d is None:
                log(f"ABORT {ev} at page {page}: {err} (after retries)")
                state[ev] = {"next_page": page, "rows": total}
                save_state(state)
                break
            gr = d.get("groupedRankings") or []
            rows = [r for g in gr if isinstance(g, list) for r in g if isinstance(r, dict)]
            if not rows:
                done_f.write(key + "\n"); done_f.flush()
                done.add(key)
                log(f"DONE {ev}: total rows={total} pages={page-1}")
                state[ev] = {"next_page": None, "rows": total}
                save_state(state)
                break
            n = 0
            if kind == "rel":
                rt = d.get("relayTeams") or {}
                for r in rows:
                    idr = r.get("IDResult")
                    members = rt.get(str(idr), {}).get("Members") or []
                    mark = r.get("display"); si = r.get("SortIntRaw")
                    team = r.get("TeamName"); tid = r.get("TeamID"); st = r.get("State")
                    for m in members:
                        if isinstance(m, dict) and m.get("GradeID") == 11:
                            aid = m.get("IDAthlete")
                            if aid:
                                out_f.write(json.dumps({
                                    "aid": aid, "ev": ev, "display": mark, "sortint": si,
                                    "team_id": tid, "team": team, "state": st,
                                    "relay": True}) + "\n")
                                n += 1
            else:
                for r in rows:
                    if r.get("GradeID") == 11:
                        out_f.write(json.dumps({
                            "aid": r.get("AthleteID"), "ev": ev,
                            "display": r.get("display"), "sortint": r.get("SortIntRaw"),
                            "team_id": r.get("TeamID"), "team": r.get("TeamName"),
                            "state": r.get("State"), "relay": False}) + "\n")
                        n += 1
            rows_written += n
            total += n
            out_f.flush()
            done_f.write(key + "\n"); done_f.flush()
            done.add(key)
            state[ev] = {"next_page": page + 1, "rows": total}
            page += 1
            if reqs % 25 == 0:
                save_state(state)
                el = time.time() - t0
                log(f"progress: {reqs} reqs, {rows_written} rows, {el/60:.1f} min, cur {ev} p{page-1} (+{n})")
            throttle()

    out_f.close(); done_f.close()
    save_state(state)
    el = time.time() - t0
    log(f"ALL DONE: {reqs} reqs, {rows_written} rows, {el/60:.1f} min")

if __name__ == "__main__":
    main()
