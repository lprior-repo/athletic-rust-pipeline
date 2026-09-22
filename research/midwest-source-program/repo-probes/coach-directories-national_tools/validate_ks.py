"""Measure the coach-contacts baseline against the live KSHSAA JSON directory.

The KSHSAA directory endpoint is a bulk-per-letter JSON API, so the whole state is
26 requests. That lets this check cover EVERY baseline KS row instead of a sample:
for each `state == "KS"` row of coach-contacts.csv, look up its school in the live
directory and compare the published AD name and AD email.

Only contract-allowed fields are read: ADName and ADEmail. The same payload carries
`ADCell` and `PrincipalCell` (personal mobile numbers); they are deliberately never
extracted and never written to any artifact of this lane.
"""

import csv
import json
import re
import string
from pathlib import Path

LANE = Path(__file__).resolve().parent.parent
OUT = LANE / "tools" / "validation-ks.json"
BASELINE = Path.home() / "Downloads" / "midwest-tfxc-source-research" / "data" / "coach-contacts.csv"
SUFFIX = re.compile(r"(?i)\s*\b(?:hs|h s|high school|high|sr high|jr sr high|jr-sr high|county high)\b\.?\s*$")


def key(name: str) -> str:
    """Normalize a school name for lookup: lowercase, drop generic school suffixes."""
    n = re.sub(r"[^a-z0-9 ]+", " ", name.lower())
    n = re.sub(r"\s+", " ", n).strip()
    prev = None
    while prev != n:
        prev = n
        n = SUFFIX.sub("", n).strip()
    return n


def load_directory() -> dict[str, dict]:
    schools: dict[str, dict] = {}
    for c in string.ascii_lowercase:
        p = LANE / "samples" / "api" / f"KS__directory_{c}.json"
        rows = json.loads(p.read_text(encoding="utf-8"))
        for r in rows:
            k = key(str(r.get("SchoolName", "")))
            if k:
                schools.setdefault(k, r)
    return schools


def main() -> int:
    schools = load_directory()
    with BASELINE.open(encoding="utf-8") as fh:
        rows = [r for r in csv.DictReader(fh) if r["state"] == "KS"]

    recs = []
    for r in rows:
        hit = schools.get(key(r["school"]))
        rec = {
            "school": r["school"], "city": r["city"], "sport": r["sport"], "role": r["role"],
            "baseline_ad_name": r["ad_name"], "baseline_ad_email": r["ad_email"],
            "baseline_coach": r["coach_name"], "baseline_coach_email": r["public_professional_email"],
        }
        if hit is None:
            rec |= {"matched": False, "agree_ad_name": None, "agree_ad_email": None}
        else:
            live_name = str(hit.get("ADName", "")).strip()
            live_mail = str(hit.get("ADEmail", "")).strip()
            rec |= {
                "matched": True,
                "kshsaa_identifier": hit.get("Identifier"),
                "live_ad_name": live_name,
                "live_ad_email": live_mail,
                # name compare ignores punctuation/whitespace; email compare is exact-modulo-case
                "agree_ad_name": bool(re.sub(r"[^a-z]", "", live_name.lower())
                                      and re.sub(r"[^a-z]", "", live_name.lower())
                                      == re.sub(r"[^a-z]", "", str(r["ad_name"]).lower())),
                "agree_ad_email": bool(live_mail)
                and live_mail.lower() == str(r["ad_email"]).strip().lower(),
                "live_has_coach_field": any(
                    re.search(r"(?i)coach", f"{k}") for k in hit.keys()),
            }
        recs.append(rec)

    OUT.write_text(json.dumps(recs, indent=1) + "\n", encoding="utf-8")

    matched = [r for r in recs if r["matched"]]
    nm = [r for r in matched if r["agree_ad_name"]]
    em = [r for r in matched if r["agree_ad_email"]]
    print(f"baseline KS rows          : {len(recs)}")
    print(f"school matched in live dir: {len(matched)}")
    print(f"AD name agrees            : {len(nm)}/{len(matched)} = {100 * len(nm) / max(1, len(matched)):.1f}%")
    print(f"AD email agrees           : {len(em)}/{len(matched)} = {100 * len(em) / max(1, len(matched)):.1f}%")
    print(f"live dir school count     : {len(schools)}")
    print("\nfirst 5 unmatched baseline schools:")
    for r in recs:
        if not r["matched"]:
            print("   ", r["school"], "/", r["city"])
    print("\nfirst 5 sampled agreements:")
    for r in matched[:5]:
        print(f"    {r['school'][:26]:<26} name={r['agree_ad_name']} email={r['agree_ad_email']}  "
              f"{r['live_ad_name'][:22]:<22} {r['live_ad_email'][:34]}")
    print("\nfirst 5 sampled disagreements (name or email):")
    for r in [x for x in matched if not (x["agree_ad_name"] and x["agree_ad_email"])][:5]:
        print(f"    {r['school'][:24]:<24} base=({r['baseline_ad_name'][:18]},{r['baseline_ad_email'][:26]}) "
              f"live=({r['live_ad_name'][:18]},{r['live_ad_email'][:26]})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
