#!/bin/bash
cd /home/lewis/src/ad-law-scrape/athletic-rust-pipeline
set +e
output=$(cargo check -p census-service --all-targets 2>&1)
echo "$output"
if [ $? -ne 0 ]; then
    echo "=== CARGO CHECK FAILED ==="
fi