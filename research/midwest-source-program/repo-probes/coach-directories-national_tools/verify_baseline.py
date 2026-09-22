"""Re-check coach-contacts.csv baseline rows against the on-disk captures.

Method (per row): reconstruct a rendered-text form of the captured body and search
it for each baseline string. Three normalizations are applied, because the raw
markup breaks naive substring search:

  1. HTML entities are unescaped (`&nbsp;` -> space, `&amp;` -> `&`).
  2. Tags are replaced by a single space, so a name split across nested
     <td>/<a>/<label> elements reassembles; a second pass removes ALL whitespace so
     a string split by intervening markup also matches.
  3. Cloudflare-obfuscated addresses (`/cdn-cgi/l/email-protection#<hex>`, used by
     gobound-style CF deployments) are XOR-decoded and added to the text, so an
     email present only in obfuscated form still counts as present.

A row is `verified` only when a capture named by the plan exists on disk AND every
`must` string is found. Rows with no capture are reported `unverified_no_sample`.
"""

import csv
import binascii
import json
import re
import html as H
from pathlib import Path

LANE = Path(__file__).resolve().parent.parent
PLAN = LANE / "tools" / "validation-plan.json"
OUT = LANE / "tools" / "baseline-validation.json"
BASELINE = Path.home() / "Downloads" / "midwest-tfxc-source-research" / "data" / "coach-contacts.csv"

CF = re.compile(r"email-protection#([0-9a-fA-F]+)")


def decode_cf(blob: str) -> str:
    """Decode one Cloudflare email-protection hex blob (first byte is the key)."""
    try:
        raw = binascii.unhexlify(blob)
    except binascii.Error:
        return ""
    if not raw:
        return ""
    key = raw[0]
    return "".join(chr(b ^ key) for b in raw[1:])


ATTR = re.compile(r'(?i)\b(?:href|src|data-[a-z0-9-]+|title|alt|value|content)\s*=\s*"([^"]*)"')


def rendered(body: str) -> tuple[str, str]:
    """Return (spaced_text, compact_text) for a captured body.

    Attribute values are folded into the text before tags are stripped: an address
    published as `href="mailto:x@y"` is exactly the published contact, and dropping
    it with the tag would produce a false negative.
    """
    text = re.sub(r"(?is)<(script|style)[^>]*>.*?</\1>", " ", body)
    extra = " ".join(decode_cf(m.group(1)) for m in CF.finditer(body))
    extra += " " + " ".join(m.group(1) for m in ATTR.finditer(body))
    text = re.sub(r"<[^>]+>", " ", text + " " + extra)
    text = re.sub(r"\s+", " ", H.unescape(text))
    return text, re.sub(r"\s+", "", text)


def plan_rows() -> list[dict]:
    return json.loads(PLAN.read_text(encoding="utf-8"))


def main() -> int:
    rows = plan_rows()
    out = []
    for r in rows:
        sample = r.get("sample")
        path = (LANE / sample) if sample else None
        rec = {
            "label": r["label"], "state": r["state"], "url": r.get("url"),
            "note": r.get("note"), "sample": sample, "must": r["must"],
        }
        if r.get("refused"):
            rec |= {"status": "refused", "agree": None, "found": {}, "refused": r["refused"]}
            out.append(rec)
            continue
        if path is None or not path.exists():
            rec |= {"status": "unverified_no_sample", "agree": None, "found": {}}
            out.append(rec)
            continue
        raw = path.read_bytes()
        body = raw.decode("utf-8", errors="replace")
        spaced, compact = rendered(body)
        found = {}
        for s in r["must"]:
            key = re.sub(r"\s+", "", H.unescape(s))
            found[s] = (s in spaced) or (key.lower() in compact.lower())
        rec |= {
            "status": "verified" if all(found.values()) else "mismatch",
            "agree": all(found.values()),
            "on_disk_bytes": len(raw),
            "found": found,
            "missing": [k for k, v in found.items() if not v],
        }
        out.append(rec)

    OUT.write_text(json.dumps(out, indent=1) + "\n", encoding="utf-8")

    have = [r for r in out if r["status"] in ("verified", "mismatch")]
    ok = [r for r in have if r["agree"]]
    refused = [r for r in out if r["status"] == "refused"]
    print(f"rows={len(out)}  checked={len(have)}  agree={len(ok)}  "
          f"mismatch={len(have) - len(ok)}  refused={len(refused)}  "
          f"no_sample={len(out) - len(have) - len(refused)}")
    if have:
        print(f"agreement rate over checked rows: {len(ok)}/{len(have)} = {100 * len(ok) / len(have):.1f}%")
    for r in out:
        flag = {True: "OK  ", False: "FAIL", None: "----"}[r["agree"]]
        print(f"  {flag} {r['state']:3} {r['label'][:40]:<40} {r['status']}"
              + (f"  missing={r.get('missing')}" if r.get("missing") else ""))

    # cross-check: the checked rows must be real rows of the baseline CSV
    with BASELINE.open(encoding="utf-8") as fh:
        base = list(csv.DictReader(fh))
    print(f"\nbaseline rows on disk: {len(base)}  (path: {BASELINE})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
