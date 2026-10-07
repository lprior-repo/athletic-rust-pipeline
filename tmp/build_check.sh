#!/bin/bash
cd /home/lewis/src/ad-law-scrape/athletic-rust-pipeline
cargo check -p census-service --all-targets 2>&1 | tail -100