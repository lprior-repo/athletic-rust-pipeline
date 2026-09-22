"""Count sport-scoped XC/TF head-coach rows in the shared fpsports/mpa/riil template.

`ciac.fpsports.org`, `www.mpa.cc` and `riil.org/Directory.aspx` render the same
school-directory layout: one <table> per school whose rows are

    [Sport/Activity, Role, Name, Phone]

with Role in {Head Coach, Athletic Director, Principal, ...}. The sport lives in the
FIRST cell, so TF/XC head coaches are directly addressable at tier 1. This script
counts, per state, the schools that carry at least one such row and the number of rows.
"""

import json
import re
import html as H
from pathlib import Path

LANE = Path(__file__).resolve().parent.parent
SRC = {
    "CT": "samples/dir/CT__directory.html",
    "ME": "samples/dir/ME__directory.html",
    "RI": "samples/dir/RI__directory.html",
    "NV": "samples/dir/NV__directory.html",
}
XC_TF = re.compile(r"(?i)\b(cross[ -]?country|indoor track|outdoor track|track (?:&|and) field|track|xc)\b")


def tables(body: str) -> list[list[list[str]]]:
    out = []
    for tm in re.finditer(r"(?is)<table[^>]*>(.*?)</table>", body):
        rows = []
        for rm in re.finditer(r"(?is)<tr[^>]*>(.*?)</tr>", tm.group(1)):
            cells = [re.sub(r"\s+", " ", H.unescape(re.sub(r"<[^>]+>", "", c))).strip()
                     for c in re.findall(r"(?is)<t[dh][^>]*>(.*?)</t[dh]>", rm.group(1))]
            if cells:
                rows.append(cells)
        if rows:
            out.append(rows)
    return out


def main() -> int:
    report = {}
    for st, rel in SRC.items():
        body = (LANE / rel).read_text(encoding="utf-8", errors="replace")
        ts = tables(body)
        schools = coaches = ad_rows = 0
        examples = []
        for t in ts:
            hit = [r for r in t if len(r) >= 4 and r[1] == "Head Coach" and XC_TF.search(r[0])]
            if hit:
                schools += 1
                coaches += len(hit)
                if len(examples) < 3:
                    examples.append(hit[0][:4])
            ad_rows += sum(1 for r in t if len(r) >= 4 and r[1] == "Athletic Director")
        report[st] = {
            "sample": rel, "bytes": len(body), "tables": len(ts),
            "schools_with_xc_tf_head_coach": schools,
            "xc_tf_head_coach_rows": coaches,
            "athletic_director_rows": ad_rows,
            "example_rows": examples,
            "phone_column_populated_examples": [e for e in examples if e[3]],
        }
    print(json.dumps(report, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
