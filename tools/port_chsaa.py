#!/usr/bin/env python3
"""Port CHSAA/CHSAANow fixtures and generate golden rows."""

import json
import hashlib
import os
import sys
import shutil

PROTOTYPE_RAW = "/home/lewis/src/ad-law-scrape/census-prototype/raw"
RUST_FIXTURES = "/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/crates/census-crawl/tests/fixtures/chsaa"

def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        data = f.read()
        h.update(data)
    return h.hexdigest(), len(data)

def copy_fixture(source_name, target_name):
    source = os.path.join(PROTOTYPE_RAW, source_name)
    target = os.path.join(RUST_FIXTURES, target_name)
    shutil.copy2(source, target)
    sha, size = sha256_file(target)
    return sha, size

def find_file_by_content(marker, files):
    for name in files:
        path = os.path.join(PROTOTYPE_RAW, name)
        try:
            with open(path, "r", encoding="utf-8", errors="replace") as f:
                content = f.read()
            if marker in content:
                return name
        except Exception:
            pass
    return None

os.makedirs(RUST_FIXTURES, exist_ok=True)

print("=== Copying CHSAA fixtures ===")

# Directory page - find by the embedded JSON array marker
print("Finding directory page...")
directory_file = find_file_by_content('[{\\\"schoolCode\\\"', os.listdir(PROTOTYPE_RAW))
if not directory_file:
    directory_file = find_file_by_content('[{"schoolCode"', os.listdir(PROTOTYPE_RAW))
if not directory_file:
    # Try another approach - the notes say the array starts at the marker
    directory_file = "chsaanow.com__0018db17820a243d447b3cc0"

print(f"Directory file: {directory_file}")
dir_sha, dir_size = copy_fixture(directory_file, "directory.html")
print(f"  Copied: directory.html ({dir_size} bytes, sha256={dir_sha[:16]}...)")

# Cherry Creek school page
print("Finding cherry-creek school page...")
cherry_creek_file = find_file_by_content("cherry-creek", [f for f in os.listdir(PROTOTYPE_RAW) if f.startswith("chsaanow.com__")])
if not cherry_creek_file:
    cherry_creek_file = "chsaanow.com__40e1a856a5c5e92c9d387b56"

print(f"Cherry Creek file: {cherry_creek_file}")
cc_sha, cc_size = copy_fixture(cherry_creek_file, "school_cherry_creek.html")
print(f"  Copied: school_cherry_creek.html ({cc_size} bytes, sha256={cc_sha[:16]}...)")

# Copy other school pages that exist
print("Copying school pages...")
chsaa_files = [f for f in os.listdir(PROTOTYPE_RAW) if f.startswith("chsaanow.com__") and f != directory_file and f != cherry_creek_file]
school_count = 0
for name in chsaa_files:
    try:
        with open(os.path.join(PROTOTYPE_RAW, name), "r", encoding="utf-8", errors="replace") as f:
            content = f.read(10000)
        # Check if this is a school page (has activityName in JSON)
        if 'activityName' in content or '"slug"' in content:
            # Extract school slug from the content
            import re
            slug_match = re.search(r'"slug"\s*:\s*"([^"]+)"', content)
            if slug_match:
                slug = slug_match.group(1)
                target_name = f"school_{slug}.html"
                copy_fixture(name, target_name)
                school_count += 1
    except Exception as e:
        pass

print(f"  Copied {school_count} school pages")

# Generate robots.txt
robots_content = """User-agent: *
Allow: /

# CHSAA/CHSAANow robots.txt (from prototype research, 2026-09-27)
Disallow: /history/champions/individual/totals/repeat/
Disallow: /preview/
"""
with open(os.path.join(RUST_FIXTURES, "robots.txt"), "w") as f:
    f.write(robots_content)
print("  Created robots.txt")

# Write provenance
provenance = {
    "fixtures": [
        {
            "fixture": "directory.html",
            "url": "https://chsaanow.com/schools/",
            "prototype_file": directory_file,
            "sha256": dir_sha,
            "bytes": dir_size
        },
        {
            "fixture": "school_cherry_creek.html",
            "url": "https://chsaanow.com/schools/cherry-creek/",
            "prototype_file": cherry_creek_file,
            "sha256": cc_sha,
            "bytes": cc_size
        },
        {
            "fixture": "robots.txt",
            "url": "https://chsaanow.com/robots.txt",
            "note": "documented from prototype research (co-chsaa.md); no raw capture exists",
            "bytes": len(robots_content.encode())
        }
    ],
    "captured_source_note": "Directory page: embedded JS-escaped JSON array of 378 school records starting at marker '[{\"schoolCode\"'. Per-school pages: embedded JSON with sports roster data including members and positions."
}

with open(os.path.join(RUST_FIXTURES, "PROVENANCE.json"), "w") as f:
    json.dump(provenance, f, indent=2)
print("  Created PROVENANCE.json")

print("\n=== Fixture inventory ===")
for name in os.listdir(RUST_FIXTURES):
    path = os.path.join(RUST_FIXTURES, name)
    if os.path.isfile(path):
        try:
            sha, size = sha256_file(path)
            print(f"  {name}: {size} bytes, sha256={sha[:16]}...")
        except Exception as e:
            print(f"  {name}: (error: {e})")
