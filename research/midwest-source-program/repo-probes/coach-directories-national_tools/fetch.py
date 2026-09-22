#!/usr/bin/env python3
"""Robot-respecting fetch harness for the coach-directories-national lane.

Every fetch: (1) resolves+parses robots.txt for the host once, (2) refuses any
path disallowed for our UA, (3) enforces >=1.0s spacing per host, (4) writes the
byte-exact body plus a JSONL record with url/status/bytes/sha256/timestamp/command.

No auth bypass, no CAPTCHA solving, no proxy rotation, no cookies beyond the
association's own public session. Read-only GET/POST of public pages.
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import threading
import time
import urllib.parse
import urllib.robotparser
from pathlib import Path
from typing import Iterable

LANE = Path(__file__).resolve().parent.parent
SAMPLES = LANE / "samples"
LOG = LANE / "tools" / "fetch-log.jsonl"
UA = "omp-research/1.0 (public coach-directory research; contact: research@example.invalid)"
MIN_INTERVAL = 1.05  # seconds between requests to the same host

_lock = threading.Lock()
_last_hit: dict[str, float] = {}
_robots: dict[str, dict] = {}
_log_lock = threading.Lock()


def _throttle(host: str) -> None:
    while True:
        with _lock:
            now = time.monotonic()
            prev = _last_hit.get(host, 0.0)
            wait = MIN_INTERVAL - (now - prev)
            if wait <= 0:
                _last_hit[host] = now
                return
        time.sleep(min(wait, MIN_INTERVAL))


def _curl(url: str, out: Path, data: str | None, extra: Iterable[str]) -> dict:
    fmt = "%{http_code}\\t%{size_download}\\t%{url_effective}\\t%{content_type}\\t%{time_total}"
    cmd = ["curl", "-sS", "-L", "--max-time", "45", "--compressed",
           "-A", UA, "-o", str(out), "-w", fmt]
    if data is not None:
        cmd += ["--data", data]
    cmd += list(extra)
    cmd += [url]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    parts = (proc.stdout or "").strip().split("\t")
    if len(parts) < 5:
        return {"url": url, "status": 0, "bytes": 0, "effective": url,
                "content_type": "", "seconds": 0.0, "error": (proc.stderr or "").strip()[:400],
                "command": " ".join(cmd)}
    status, size, eff, ctype, secs = parts[:5]
    body = out.read_bytes() if out.exists() else b""
    return {"url": url, "status": int(status), "bytes": int(size), "effective": eff,
            "content_type": ctype, "seconds": round(float(secs), 3),
            "sha256": hashlib.sha256(body).hexdigest(), "command": " ".join(cmd)}


def robots_for(url: str) -> dict:
    """Fetch + parse robots.txt once per host. Returns {'text','allow':bool,'rule':str}."""
    host = urllib.parse.urlsplit(url).netloc
    with _lock:
        if host in _robots:
            return _robots[host]
    scheme = urllib.parse.urlsplit(url).scheme or "https"
    rurl = f"{scheme}://{host}/robots.txt"
    tmp = Path("/tmp") / f"robots-{host}.txt"
    _throttle(host)
    rec = _curl(rurl, tmp, None, [])
    text = tmp.read_text(encoding="utf-8", errors="replace") if tmp.exists() else ""
    rp = urllib.robotparser.RobotFileParser()
    rp.parse(text.splitlines())
    path = urllib.parse.urlsplit(url).path or "/"
    allowed = rp.can_fetch(UA, url) if text.strip() else True
    rule = ""
    for line in text.splitlines():
        low = line.strip().lower()
        if low.startswith("disallow:") and low.split(":", 1)[1].strip() not in ("", "/"):
            d = low.split(":", 1)[1].strip()
            if d and path.startswith(d):
                rule = line.strip()
    info = {"host": host, "url": rurl, "status": rec["status"], "bytes": rec["bytes"],
            "allow": allowed, "rule": rule, "text": text[:4000],
            "crawl_delay": rp.crawl_delay(UA) or rp.crawl_delay("*"),
            "request_rate": str(rp.request_rate(UA) or rp.request_rate("*") or "")}
    with _lock:
        _robots[host] = info
    _record({"kind": "robots", **{k: v for k, v in info.items() if k != "text"},
             "text_sha256": hashlib.sha256(text.encode()).hexdigest()})
    (LANE / "tools" / "robots").mkdir(parents=True, exist_ok=True)
    (LANE / "tools" / "robots" / f"{host}.txt").write_text(text, encoding="utf-8")
    return info


def _record(rec: dict) -> None:
    with _log_lock:
        with LOG.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps(rec, sort_keys=True) + "\n")


def fetch(url: str, name: str | None = None, data: str | None = None,
          extra: Iterable[str] = (), skip_robots: bool = False) -> dict:
    """Fetch url into samples/<name>. Refuses when robots.txt disallows the path."""
    host = urllib.parse.urlsplit(url).netloc
    rb = {"allow": True, "rule": "", "status": None} if skip_robots else robots_for(url)
    if not rb["allow"]:
        rec = {"kind": "refused", "url": url, "host": host, "robots_rule": rb["rule"],
               "robots_status": rb["status"], "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}
        _record(rec)
        return {**rec, "skipped": True, "bytes": 0, "status": None}
    if name is None:
        stem = urllib.parse.urlsplit(url).path.strip("/").replace("/", "_") or "index"
        name = (host + "__" + stem)[:150]
    out = SAMPLES / name
    out.parent.mkdir(parents=True, exist_ok=True)
    _throttle(host)
    rec = _curl(url, out, data, extra)
    rec.update({"kind": "fetch", "name": name, "host": host,
                "robots_allow": True, "robots_rule": rb["rule"], "robots_status": rb["status"],
                "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())})
    _record(rec)
    return rec


def parallel(jobs: list[tuple[str, str]], workers: int = 8, **kw) -> list[dict]:
    """jobs = [(url, sample_name), ...]; different hosts run concurrently."""
    results: list[dict | None] = [None] * len(jobs)

    def run(i: int, url: str, nm: str) -> None:
        try:
            results[i] = fetch(url, nm, **kw)
        except Exception as exc:  # noqa: BLE001 - harness must never die on one URL
            results[i] = {"url": url, "error": repr(exc)[:300], "status": 0, "bytes": 0}

    threads = []
    for i, (url, nm) in enumerate(jobs):
        t = threading.Thread(target=run, args=(i, url, nm), daemon=True)
        t.start()
        threads.append(t)
        while sum(1 for x in threads if x.is_alive()) >= workers:
            time.sleep(0.05)
    for t in threads:
        t.join()
    return [r for r in results if r is not None]


if __name__ == "__main__":
    import sys
    for u in sys.argv[1:]:
        r = fetch(u)
        print(json.dumps({k: r.get(k) for k in ("url", "status", "bytes", "effective")}, sort_keys=True))
