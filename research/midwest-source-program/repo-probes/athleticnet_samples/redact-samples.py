#!/usr/bin/env python3
"""Redact credentials from the athleticnet lane samples.

The evidence-capture rule for this lane is: raw bytes verbatim, except credentials, which are
never persisted. Athletic.net returns a server-minted `jwtTFTopReport` inside GetRankings JSON
and sets session cookies in response headers, so both are masked here.

Run:  python3 redact-samples.py     (idempotent; reports what it changed)
"""

from __future__ import annotations

import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent

JSON_RULES = (
    (re.compile(rb'("jwtTFTopReport"\s*:\s*")[^"]*(")'), rb"\1<redacted-server-issued>\2"),
    (re.compile(rb'("jwtMeet"\s*:\s*")[^"]*(")'), rb"\1<redacted-server-issued>\2"),
)
HEADER_RULES = (
    (re.compile(rb"(?im)^(set-cookie:).*$"), rb"\1 <redacted>"),
    (re.compile(rb"(?im)^((?:anettokens|anet-site-roles-token|anet-appinfo|authorization|cookie):).*$"),
     rb"\1 <redacted>"),
)
JWT_SHAPE = re.compile(rb"eyJ[A-Za-z0-9_-]{10,}")


def redact(path: pathlib.Path, rules) -> int:
    raw = path.read_bytes()
    out, hits = raw, 0
    for pattern, replacement in rules:
        out, count = pattern.subn(replacement, out)
        hits += count
    if out != raw:
        path.write_bytes(out)
    return hits


def main() -> int:
    total = 0
    for path in sorted(HERE.iterdir()):
        if not path.is_file():
            continue
        if path.suffix == ".json":
            hits = redact(path, JSON_RULES)
        elif path.suffix == ".headers":
            hits = redact(path, HEADER_RULES)
        else:
            continue
        if hits:
            print(f"redacted {hits:>2} credential(s) in {path.name}")
            total += hits
    leaks = [p.name for p in sorted(HERE.glob("*.json")) if JWT_SHAPE.search(p.read_bytes())]
    cookie_leaks = [p.name for p in sorted(HERE.glob("*.headers")) if b"<redacted>" not in p.read_bytes() and b"set-cookie" in p.read_bytes().lower()]
    print(f"total redactions: {total}")
    print(f"files still containing a JWT-shaped string: {leaks or 'none'}")
    print(f"header files still carrying an unmasked set-cookie: {cookie_leaks or 'none'}")
    return 1 if leaks or cookie_leaks else 0


if __name__ == "__main__":
    sys.exit(main())
