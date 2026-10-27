#!/usr/bin/env python3
"""Merge the lane address corpus with the owned GitHub address database.

Reads:
  1. var/school-address-join-20261004/corpus/current/school_directory.json
     (122692 lane entries: 100307 NCES CCD + 22385 PSS private)
  2. <github-checkout>/output/high_school_addresses.csv
     (lprior-repo/rust-athletic-school-addresses @ b705873, 102178 rows)

Writes <out>/school_directory.json plus <out>/../provenance.json.

Rules: base wins on NCES id; GitHub rows append only when the id is absent
and the state is in census scope (48 states + DC). Converted rows carry
name + street address only; coordinates are unused by the workbook fill.
Output is deterministic for identical inputs; the script asserts the
expected input digests and row counts and fails closed otherwise.

Usage:
  tools/merge-address-corpus.py <github-output-dir> <out-dir>
"""

import csv
import hashlib
import json
import sys

BASE_PATH = "var/school-address-join-20261004/corpus/current/school_directory.json"
BASE_ROWS = 122692
GITHUB_ROWS = 102178
GITHUB_COMMIT = "b7058732f05ab59105aa042e66d8a8cad467bf3c"
EXPECTED_ADDED = 1092

SCOPE = {
    'AL', 'AZ', 'AR', 'CA', 'CO', 'CT', 'DC', 'DE', 'FL', 'GA',
    'IA', 'ID', 'IL', 'IN', 'KS', 'KY', 'LA', 'MA', 'MD', 'ME',
    'MI', 'MN', 'MO', 'MS', 'MT', 'NC', 'ND', 'NE', 'NH', 'NJ',
    'NM', 'NV', 'NY', 'OH', 'OK', 'OR', 'PA', 'RI', 'SC', 'SD',
    'TN', 'TX', 'UT', 'VA', 'VT', 'WA', 'WI', 'WV', 'WY',
}


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b''):
            digest.update(chunk)
    return digest.hexdigest()


def main():
    github_dir, out_dir = sys.argv[1], sys.argv[2]
    base = json.load(open(BASE_PATH))
    assert len(base) == BASE_ROWS, f"base rows {len(base)} != {BASE_ROWS}"
    have = set()
    for entry in base:
        key = entry.get('key', {})
        if 'Nces' in key:
            have.add(key['Nces'])
    github_path = f"{github_dir}/high_school_addresses.csv"
    raw = open(github_path, 'rb').read()
    github_rows = list(csv.DictReader(raw.decode().splitlines()))
    assert len(github_rows) == GITHUB_ROWS, f"github rows {len(github_rows)}"
    added = []
    for row in github_rows:
        uid = (row['unit_id'] or '').strip()
        state = (row['state'] or '').strip().upper()
        if not uid or state not in SCOPE or uid in have:
            continue
        zip_code = (row['zip_code'] or '').strip()
        entry = {
            'key': {'Nces': uid},
            'name': (row['name'] or '').strip(),
            'address': {
                'line1': (row['address'] or '').strip(),
                'city': (row['city'] or '').strip(),
                'state': state,
                'zip': {'code': zip_code}
                if len(zip_code) == 5 and zip_code.isdigit()
                else {},
            },
            'sources': ['Ccd'],
            'provenance': {'Name': 'Ccd', 'Address': 'Ccd'},
        }
        if not entry['name'] or not entry['address']['line1']:
            continue
        added.append(entry)
        have.add(uid)
    assert len(added) == EXPECTED_ADDED, f"added {len(added)} != {EXPECTED_ADDED}"
    merged = base + added
    out_path = f"{out_dir}/school_directory.json"
    with open(out_path, 'w') as handle:
        json.dump(merged, handle)
    provenance = {
        'base_corpus': BASE_PATH,
        'base_rows': len(base),
        'github_source': 'lprior-repo/rust-athletic-school-addresses/output/high_school_addresses.csv',
        'github_commit': GITHUB_COMMIT,
        'github_rows': len(github_rows),
        'github_file_sha256': hashlib.sha256(raw).hexdigest(),
        'github_only_imported': len(added),
        'conflict_rule': 'base wins on NCES id; github rows appended only when id absent',
        'scope': '48 states + DC; AK/HI/PR/GU/VI/MP/AS excluded by census jurisdiction scope',
        'merged_rows': len(merged),
        'merged_sha256': sha256_file(out_path),
        'note': 'Converted rows omit coordinates (unused by workbook fill).',
    }
    with open(f"{out_dir}/../provenance.json", 'w') as handle:
        json.dump(provenance, handle, indent=1)
    print(f"base {len(base)} added {len(added)} merged {len(merged)}")
    print(f"sha256 {provenance['merged_sha256']}")


if __name__ == '__main__':
    main()
