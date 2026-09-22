#!/usr/bin/env python3
"""Extract link inventory / text from a captured sample.

Usage:
  python3 tools/extract.py links <file> [filter-regex]
  python3 tools/extract.py text  <file> [start-marker] [chars]
  python3 tools/extract.py pdf-find <pdf> <regex>
Read-only over samples/.
"""
import html
import re
import subprocess
import sys


def strip(h: str) -> str:
    h = re.sub(r"(?is)<(script|style)[^>]*>.*?</\1>", " ", h)
    return h


def links(path: str):
    h = open(path, encoding="utf-8", errors="replace").read()
    out = []
    for m in re.finditer(r'(?is)<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>', strip(h)):
        txt = html.unescape(re.sub(r"<[^>]+>", "", m.group(2)))
        out.append((re.sub(r"\s+", " ", txt).strip(), m.group(1)))
    seen = set()
    for t, u in out:
        k = (t, u)
        if k in seen:
            continue
        seen.add(k)
        print(f"{t[:70]!r}\t{u}")


def text(path: str, marker: str | None, n: int):
    h = open(path, encoding="utf-8", errors="replace").read()
    t = html.unescape(re.sub(r"(?s)<[^>]+>", " ", strip(h)))
    t = re.sub(r"\s+", " ", t)
    i = 0 if not marker else max(0, t.lower().find(marker.lower()))
    print(t[i : i + n])


def pdf_find(path: str, pat: str):
    txt = subprocess.run(
        ["pdftotext", "-layout", path, "-"], capture_output=True, text=True, check=True
    ).stdout
    for line in txt.splitlines():
        if re.search(pat, line, re.I):
            print(line.rstrip())


if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "links":
        filt = sys.argv[3] if len(sys.argv) > 3 else ""
        h = open(sys.argv[2], encoding="utf-8", errors="replace").read()
        for m in re.finditer(r'(?is)<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>', strip(h)):
            txt = html.unescape(re.sub(r"<[^>]+>", "", m.group(2)))
            line = f"{re.sub(r'\\s+', ' ', txt).strip()[:70]!r}\t{m.group(1)}"
            if not filt or re.search(filt, line, re.I):
                print(line)
    elif cmd == "text":
        text(sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else None, int(sys.argv[4]) if len(sys.argv) > 4 else 1200)
    elif cmd == "pdf-find":
        pdf_find(sys.argv[2], sys.argv[3])
    else:
        sys.exit("unknown cmd")
