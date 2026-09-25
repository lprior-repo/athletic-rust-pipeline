#!/usr/bin/env bash
# scenario-01-endpoint-kill.sh — endpoint consolidation kill recovery.
#
# Verifies the integration test crates/census-service/tests/restate_kill_restart.rs:
# a killed endpoint resumes its run and repeats no durable write.
# The test spawns a real restate-server, registers census-serve, submits a
# Consolidate run, SIGKILLs the endpoint after 300ms, restarts it, resumes
# the paused invocation, and asserts no duplicate durable writes.

set -euo pipefail

SCRATCH_STORE="${SCRATCH_STORE:-/tmp/durability-scenario/01}"
BINARY="${BINARY:-census-service}"
REPO_ROOT="${REPO_ROOT:-.}"

# Check for the pinned Restate server binary required by restate_kill_restart.rs
if [ -n "${RESTATE_SERVER_BIN:-}" ] && [ -x "$RESTATE_SERVER_BIN" ]; then
    echo "RESTATE_SERVER_BIN=$RESTATE_SERVER_BIN available"
else
    echo "SKIPPED: RESTATE_SERVER_BIN not set or not executable; restate_kill_restart requires a pinned restate-server 1.7.10 binary"
    exit 0
fi

# Run the integration test that proves kill-and-recovery
echo "Running crates/census-service/tests/restate_kill_restart.rs..."
cd "$REPO_ROOT"
cargo test -p census-service --test restate_kill_restart -- --nocapture 2>&1
TEST_RC=$?
cd - >/dev/null

if [ $TEST_RC -eq 0 ]; then
    echo "PASS: restate_kill_restart verifies endpoint consolidation kill recovery — killed endpoint resumes from journal, no duplicate durable writes"
    exit 0
else
    echo "FAIL: restate_kill_restart exited with code $TEST_RC"
    exit 1
fi
