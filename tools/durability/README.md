# Durability harness — seventeen failure-injection scenarios

Run with `tools/durability/run.sh`.  Each scenario is a self-contained script under
`tools/durability/scenario-NN-*.sh` (where NN is zero-padded).  The runner executes every
scenario in order, prints a PASS / FAIL / SKIPPED table, and exits nonzero on any FAIL or SKIPPED.

## Shared preconditions

Every scenario expects the census-service binary to be built.  Most scenarios skip without it.

| Precondition | How to verify |
|---|---|
| `census-service` binary | `"$BINARY" --help` prints the clap usage |
| `census-serve` binary | `"$SERVE_BINARY" --help` |
| Pinned Restate 1.7.10 at `$RESTATE_SERVER_BIN` or `$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server` | `"$RESTATE_SERVER_BIN" --help` |

The runner exports `SCRATCH_STORE`, `TMPDIR`, `BINARY`, `SERVE_BINARY`, `RESTATE_BINARY`,
`RESTATE_SERVER_BIN`, `CORPUS_FIXTURE`, `ADMIN_PORT`, `SERVICE_PORT`, `REPO_ROOT`,
`BINARY_AVAILABLE`, `RESTATE_AVAILABLE` to each scenario script.  Scenario scripts **must not**
edit Rust sources, `Cargo.toml`, or anything under `crates/`.  They may create and destroy
directories under `$SCRATCH_STORE`.

## Setup: obtaining the pinned Restate server

The census lanes are qualified against Restate 1.7.10. Scenario 01 starts and reaps
its own isolated Restate processes on ephemeral ports; no pre-existing live server
is required. Set `SCRATCH_STORE` to a local-disk directory, not tmpfs.

1. **Download** the pinned build from the Restate releases page, checking it against the published
   digest. The asset for x86-64 Linux is the musl tarball, and the binary sits one directory deep:
   ```
   base=https://github.com/restatedev/restate/releases/download/v1.7.10
   artefact=restate-server-x86_64-unknown-linux-musl.tar.xz
   curl -LO "$base/$artefact" && curl -LO "$base/$artefact.sha256"
   sha256sum -c "$artefact.sha256"          # 870fdc42…c83355e
   dir="$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10"
   mkdir -p "$dir" && tar xJf "$artefact" -C "$dir"
   mv "$dir/restate-server-x86_64-unknown-linux-musl/restate-server" "$dir/"
   "$dir/restate-server" --version          # restate-server 1.7.10
   ```
   `crates/census-service/tests/restate_kill_restart.rs` looks in `$RESTATE_SERVER_BIN`, then at
   exactly `$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server`, then on PATH,
   and panics rather than skipping when it finds none — it exists to prove a real server resumes
   a run whose endpoint was killed. `.github/workflows/gate.yml` installs it the same way.
2. **Place** the binary where the runner looks: `RESTATE_BINARY` for scenarios that need it, or
   the directory above.

Do not commit the binary to the repository.

## Scenario table

| # | Script | Status | Evidence |
|---|---|---|---|
| 1 | `scenario-01-endpoint-kill.sh` | Invokes tests | Endpoint crash recovery and Restate restart preserving the same paused workflow invocation; snapshots and observation counts reconcile |
| 2 | `scenario-02-restate-kill-during-fanout.sh` | SKIPPED | No isolated NationalCensus mid-fanout crash scenario; scenario 01 does not prove this |
| 3 | `scenario-03-reboot-with-full-census.sh` | SKIPPED | No whole-machine reboot scenario; process restart does not prove this |
| 4 | `scenario-04-rolling-upgrade.sh` | SKIPPED | No two-version rolling-upgrade scenario |
| 5 | `scenario-05-http-error-taxonomy.sh` | SKIPPED | No browser HTTP fault-server scenario for rate limits, failures and challenges |
| 6 | `scenario-06-no-duplicate-evidence.sh` | SKIPPED | No crash-injected evidence replay assertion; observation counts alone are insufficient |
| 7 | `scenario-07-domain-dedup.sh` | SKIPPED | No concurrent cross-workflow domain-dedup scenario |
| 8 | `scenario-08-global-budget.sh` | SKIPPED | No multi-endpoint global-budget scenario |
| 9 | `scenario-09-disk-full-fjall.sh` | SKIPPED | No isolated bounded-filesystem ENOSPC injector for Fjall |
| 10 | `scenario-10-disk-full-restate.sh` | Invokes processes | Private bounded tmpfs; actual Restate storage ENOSPC; acknowledged workflow survives restart; duplicate refused and fresh workflow returns baseline output |
| 11 | `scenario-11-parent-exit.sh` | SKIPPED | `ATHLETIC_FAULT_HTTP_EXIT` seam not implemented |
| 12 | `scenario-12-cross-midnight.sh` | SKIPPED | No clock-manipulation seam |
| 13 | `scenario-13-ai-review-failures.sh` | Invokes tests | Real model HTTP client against isolated peers: 503, malformed/empty answers, header/body timeouts, truncated body, and valid verdict |
| 14 | `scenario-14-seal-refuses.sh` | Invokes CLI | Builds an empty-store workbook, then requires a nonzero seal refusal with named unmet criteria |
| 15 | `scenario-15-full-backup-restore.sh` | Invokes test | `crates/census-service/tests/backup_restore.rs` — Fjall cold copy integrity |
| 16 | `scenario-16-golden-census-determinism.sh` | Invokes test | `crates/census-service/tests/parity_pipeline.rs` — byte-identical pipeline output |
| 17 | `scenario-17-recovery-tests.sh` | Invokes test | `crates/census-service/tests/recovery.rs` — SIGKILL mid-batch, service restart, drain deadline, journal-append gap, jurisdiction walk resume |

Scenario 13 verifies the model transport boundary. It does not kill a GPU model
process or prove review-checkpoint recovery. Skipped scenarios remain unverified;
a passing subset is not completion of the full durability requirement.

Scenario 10 requires Linux user, mount and PID namespaces plus `mount`, `curl`,
`jq`, `python3` and `timeout`. Only its isolated Restate data directory is on the
256 MiB tmpfs; endpoint storage and logs remain under owned local-disk scratch.
Bounded high-entropy request bodies exhaust preallocated storage. A filler error
or failed HTTP request alone cannot pass: the Restate log must contain an actual
OS disk-full error. Both child services are reaped and the private mount vanishes
with its namespace. This is process/storage recovery, not power-loss durability.

## Design principles

1. **No green results from absence of faults.**  Every scenario either passes with observable
   evidence or skips with a precise reason.
2. **SKIPPED names the missing seam.**  E.g. `"SKIPPED: no fault-injection abort point after the Fjall commit in census-service"`.
3. **Deterministic over time.**  Where clock manipulation is involved, the script advances time
   rather than waiting.
4. **Idempotent.**  Running the same scenario twice from a fresh `$SCRATCH_STORE` must produce
   the same result.
5. **No Rust edits.**  Scripts may create directories, kill processes, inject errors via
   environment variables or configuration, but must not modify `crates/`, `Cargo.toml`, or
   any `.rs` file.
6. **Scratch-only.**  All file operations use `$SCRATCH_STORE` or `mktemp -d`. No production
   directories are ever read, written, or deleted.
7. **Integration test delegation.**  Scenarios that cannot exercise a seam at the CLI level
   invoke the corresponding production integration test rather than running fake empty stores
   or sharing live ports.

## Running

```bash
# Quick run (all scenarios, defaults)
tools/durability/run.sh

# Custom store and binaries
SCRATCH_STORE=/tmp/my-scenarios \
BINARY=target/debug/census-service \
SERVE_BINARY=target/debug/census-serve \
RESTATE_SERVER_BIN=/opt/restate/restate-server \
tools/durability/run.sh

# One scenario only
tools/durability/run.sh scenario-01-endpoint-kill
```

## Exit codes

| Exit code | Meaning |
|---|---|
| 0 | All scenarios PASS |
| 1 | One or more scenarios FAIL or SKIPPED (unverified seams) |
| 2 | Invalid arguments or missing required tool |
