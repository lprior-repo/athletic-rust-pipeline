#!/usr/bin/env python3
"""Fetch GetAthleteBioData for all US grade-11 athletes; extract all-time + 2026 PR per event.

Throttled (~0.4s + jitter), retry/backoff on 429/5xx, resumable via output JSONL.
Output: one JSONL line per athlete:
  {"aid": "<id>", "school": <SchoolID|null>,
   "ev": {"100m": {"pr": "10.05a", "prs": 10044, "y26": "10.05a", "y26s": 10044}, ...}}
`pr` = all-time best Result string, `prs` = its SortIntRaw; `y26` = best in 2026 (SeasonID 2026/12026).
"""
import json, urllib.request, urllib.error, gzip, time, random, os, sys, signal

OUTDIR = os.path.expanduser("~/Downloads/athletic-bio-fetch")
os.makedirs(OUTDIR, exist_ok=True)
OUT = os.path.join(OUTDIR, "bio_prs.jsonl")
ERR = os.path.join(OUTDIR, "bio_errors.tsv")

IDS = json.load(open("/tmp/athlete_ids.json"))
EMAP = json.load(open("/tmp/event_map.json"))
EID2SHORT = {v["eid"]: k for k, v in EMAP.items() if v}

BASE = float(os.environ.get("BASE_DELAY", "0.4"))
TARGET = float(os.environ.get("TARGET_PERIOD", "0.85"))  # adaptive: seconds per request
HEADERS = {
    "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36",
    "Accept": "application/json, text/plain, */*",
    "Accept-Language": "en-US,en;q=0.9",
    "sec-ch-ua": '"Chromium";v="151", "Not=A?Brand";v="99"',
    "sec-ch-ua-mobile": "?0",
    "sec-ch-ua-platform": '"Linux"',
    "sec-fetch-dest": "empty",
    "sec-fetch-mode": "cors",
    "sec-fetch-site": "same-origin",
    "anet-appinfo": "web:web:0:300",
}

def log(*a):
    print(f"[{time.strftime('%H:%M:%S')}]", *a, file=sys.stderr, flush=True)

_st = {"f": None}
def _sig(sig, frm):
    if _st["f"]:
        try:
            _st["f"].flush(); _st["f"].close()
        except Exception:
            pass
    log("signal received, flushed and exiting")
    sys.exit(0)
signal.signal(signal.SIGTERM, _sig)
signal.signal(signal.SIGINT, _sig)

def fetch(aid):
    url = f"https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId={aid}&sport=tf&level=0"
    h = dict(HEADERS)
    h["Referer"] = f"https://www.athletic.net/athlete/{aid}/track-and-field/all"
    last = None
    for attempt in range(6):
        try:
            req = urllib.request.Request(url, headers=h)
            with urllib.request.urlopen(req, timeout=40) as r:
                raw = r.read()
            if raw[:2] == b'\x1f\x8b':
                raw = gzip.decompress(raw)
            return json.loads(raw), None
        except urllib.error.HTTPError as e:
            if e.code == 404:
                return None, "404"
            if e.code == 429:
                ra = e.headers.get("Retry-After")
                w = float(ra) if ra and ra.replace('.', '', 1).isdigit() else min(120.0, 10.0 * (2 ** attempt))
                log(f"429 aid={aid} attempt={attempt+1} sleeping {w:.0f}s")
                time.sleep(w); last = "429"; continue
            if e.code >= 500:
                w = min(60.0, 3.0 * (2 ** attempt))
                log(f"{e.code} aid={aid} attempt={attempt+1} sleeping {w:.0f}s")
                time.sleep(w); last = f"HTTP{e.code}"; continue
            return None, f"HTTP{e.code}"
        except Exception as e:
            w = min(60.0, 3.0 * (2 ** attempt))
            time.sleep(w); last = repr(e)[:60]
    return None, last or "fail"

def parse(aid, d):
    ath = d.get("athlete") or {}
    best = {}
    for x in (d.get("resultsTF") or []):
        sh = EID2SHORT.get(x.get("EventID"))
        if not sh:
            continue
        si = x.get("SortIntRaw")
        if not si or si <= 0:
            continue
        res = x.get("Result")
        if not res or not any(ch.isdigit() for ch in res):
            continue   # skip ND / NH / FOUL / DNS / DNF / X
        seas = x.get("SeasonID")
        b = best.get(sh)
        if b is None:
            b = best[sh] = {"pr": None, "prs": None, "y26": None, "y26s": None}
        if b["prs"] is None or si < b["prs"]:
            b["pr"] = res; b["prs"] = si
        if seas in (2026, 12026) and (b["y26s"] is None or si < b["y26s"]):
            b["y26"] = res; b["y26s"] = si
    return {"aid": aid, "school": ath.get("SchoolID"), "ev": best}

def load_done():
    done = set()
    if os.path.exists(OUT):
        with open(OUT) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    done.add(json.loads(line)["aid"])
                except Exception:
                    pass
    return done

def main():
    limit = int(os.environ.get("LIMIT", "0")) or None
    done = load_done()
    todo = [a for a in IDS if a not in done]
    if limit:
        todo = todo[:limit]
    log(f"resume: {len(done)} done, {len(todo)} to fetch, base_delay={BASE}s")
    f = open(OUT, "a"); _st["f"] = f
    ef = open(ERR, "a")
    t0 = time.time(); n_ok = 0; n_empty = 0; n_err = 0
    for i, aid in enumerate(todo):
        t_req = time.time()
        d, err = fetch(aid)
        if d is None:
            n_err += 1
            ef.write(f"{aid}\t{err}\n"); ef.flush()
            log(f"ERR aid={aid}: {err}")
        else:
            rec = parse(aid, d)
            if not rec["ev"]:
                n_empty += 1
            f.write(json.dumps(rec) + "\n")
            n_ok += 1
            if n_ok % 5 == 0:
                f.flush()
        if (i + 1) % 200 == 0:
            f.flush()
            el = time.time() - t0
            rate = (i + 1) / el if el else 0
            eta = (len(todo) - (i + 1)) / rate / 3600 if rate else 0
            log(f"{i+1}/{len(todo)} ok={n_ok} empty={n_empty} err={n_err} rate={rate:.2f}/s ETA={eta:.1f}h")
        time.sleep(max(0.0, TARGET - (time.time() - t_req)) + random.uniform(0, 0.1))
    f.flush(); f.close(); ef.close()
    log(f"DONE ok={n_ok} empty={n_empty} err={n_err} in {(time.time()-t0)/3600:.1f}h")

if __name__ == "__main__":
    main()
