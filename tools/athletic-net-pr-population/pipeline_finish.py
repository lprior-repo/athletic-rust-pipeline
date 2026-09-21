#!/usr/bin/env python3
"""Wait for the bio fetch process to exit, then run the PR join automatically.

Writes JOIN_DONE on success. Logs to athletic-bio-fetch/supervisor.log
"""
import os, time, subprocess, datetime

PID = int(os.environ.get("FETCH_PID", "2564453"))
OUTDIR = "/home/lewis/Downloads/athletic-bio-fetch"
LOG = os.path.join(OUTDIR, "supervisor.log")
BIO = os.path.join(OUTDIR, "bio_prs.jsonl")
EXPECT = 142288

def log(m):
    with open(LOG, "a") as f:
        f.write(f"{datetime.datetime.now().isoformat(timespec='seconds')} {m}\n")
        f.flush()

log(f"supervisor: watching pid {PID}")
while os.path.exists(f"/proc/{PID}"):
    time.sleep(30)
log("fetch pid exited")

n = sum(1 for _ in open(BIO))
log(f"bio records written: {n} / {EXPECT}")
if n < EXPECT:
    log("WARNING: fetch ended early; join runs on partial data (rerun fetch_bios.py to resume, then re-run join_prs.py)")

r = subprocess.run(["python3", "/tmp/join_prs.py"], capture_output=True, text=True)
log(f"join rc={r.returncode}")
log(r.stdout[-4000:])
if r.returncode == 0:
    with open(os.path.join(OUTDIR, "JOIN_DONE"), "w") as f:
        f.write(f"records={n}\n")
    log("JOIN_DONE written")
