#!/bin/bash
set -e

RAW="/home/lewis/src/ad-law-scrape/census-prototype/raw"
FIX="/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/crates/census-crawl/tests/fixtures/chsaa"

mkdir -p "$FIX"

# Copy directory page
cp "$RAW/chsaanow.com__0018db17820a243d447b3cc0" "$FIX/directory.html"
echo "Copied directory.html ($(stat -c%s "$FIX/directory.html") bytes)"

# Copy cherry-creek school page
cp "$RAW/chsaanow.com__40e1a856a5c5e92c9d387b56" "$FIX/school_cherry_creek.html"
echo "Copied school_cherry_creek.html ($(stat -c%s "$FIX/school_cherry_creek.html") bytes)"

# Find and copy other school pages (those with activityName in JSON)
school_count=0
for f in "$RAW"/chsaanow.com__*; do
    [ ! -f "$f" ] && continue
    name=$(basename "$f")
    [ "$name" = "chsaanow.com__0018db17820a243d447b3cc0" ] && continue
    [ "$name" = "chsaanow.com__40e1a856a5c5e92c9d387b56" ] && continue
    
    # Check if this is a school page with embedded JSON
    if head -c 10000 "$f" | grep -q 'activityName'; then
        # Extract slug
        slug=$(head -c 50000 "$f" | grep -o '"slug":"[^"]*"' | head -1 | sed 's/"slug":"//;s/"$//')
        if [ -n "$slug" ]; then
            cp "$f" "$FIX/school_${slug}.html"
            school_count=$((school_count + 1))
        fi
    fi
done
echo "Copied $school_count school pages"

# Verify fixture count
echo ""
echo "=== Fixture inventory ==="
ls -la "$FIX" | tail -n +2 | grep -v "^d" | grep -v "^total" | while read line; do
    name=$(echo "$line" | awk '{print $NF}')
    echo "  $name: $(stat -c%s "$FIX/$name") bytes"
done
