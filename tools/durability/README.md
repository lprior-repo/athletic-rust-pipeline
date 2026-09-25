
# Durability harness — sixteen failure-injection scenarios

Run with `tools/durability/run.sh`.  Each scenario is a self-contained script under
`tools/durability/scenario-NN-*.sh` (where NN is zero-padded).  The runner executes every
scenario in order, prints a PASS / FAIL / SKIPPED table, and exits nonzero on any FAIL.

## Shared preconditions

Every scenario that touches the pipeline expects the following seams to be available:

| Precondition | How to verify |
|---|---|
| Restate admin on port 19095 | `curl -sf http://127.0.0.1:19095/health` |
| Census endpoint on port 9080 | `curl -sf http://127.0.0.1:9080/health` |
| Fresh Fjall store at `$SCRATCH_STORE` (default `/tmp/durability-scenario/store`) | `rm -rf "$SCRATCH_STORE"; mkdir -p "$SCRATCH_STORE"` |
| Corpus fixture at `$CORPUS_FIXTURE` (default `fixtures/alpha/`) | `test -d "$CORPUS_FIXTURE"` — one of the scenario scripts ships a minimal fixture when the directory is absent |
| `census-service` binary on PATH or at `BINARY` env | `"$BINARY" --help` prints the clap usage |
| `census-serve` binary on PATH or at `SERVE_BINARY` env | `"$SERVE_BINARY" --help` |
| Restate 1.7.10 binary (or compatible) at `RESTATE_BINARY` | `"$RESTATE_BINARY" --help` |

The runner exports `SCRATCH_STORE`, `BINARY`, `SERVE_BINARY`, `RESTATE_BINARY`, and `CORPUS_FIXTURE`
to each scenario script.  Scenario scripts **must not** edit Rust sources, `Cargo.toml`, or anything
under `crates/`.  They may create and destroy directories under `$SCRATCH_STORE`.
## Setup: obtaining the pinned Restate server

The census lanes are qualified against Restate 1.7.10.  Scenarios 09 and 10 (disk-full testing)
do not require a running Restate node.  All other scenarios that check Restate connectivity
(01, 03, 06, 15) skip gracefully when it is unavailable.

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
   That directory is not arbitrary:
   `crates/census-service/tests/restate_kill_restart.rs` looks in `$RESTATE_SERVER_BIN`, then at
   exactly `$HOME/.local/share/athletic-rust-pipeline/restate/1.7.10/restate-server`, then on PATH,
   and fails rather than skipping when it finds none — it exists to prove a real server resumes a run
   whose endpoint was killed. `.github/workflows/gate.yml` installs it the same way before the gate,
   out of the same release.
2. **Place** the binary where the runner looks: the directory above, `RESTATE_BINARY` for these
   scenarios, or `/opt/athletic-rust-pipeline/vendor/restate-server-1.7.10/restate-server` on the
   production host.
3. **Configure** Restate: copy `deploy/restate.toml` to a temporary config file or
   `/etc/census-service/restate.toml` if running as systemd.  The config binds admin to
   port 19095 and ingress to port 18095 on loopback.
4. **Start** the node:
   ```
   mkdir -p /tmp/durability-restate
   restate-server --no-logo --config-file deploy/restate.toml --base-dir /tmp/durability-restate
   ```
5. **Register** an endpoint (optional — many scenarios just need the node running):
   ```
   curl -X POST http://127.0.0.1:19095/deployments \
     -H 'content-type: application/json' \
     -d '{"uri":"http://127.0.0.1:9080/"}'
   ```

Do not commit the binary to the repository.

## Disk-full testing

Scenarios 9 and 10 force disk-full safely using a small tmpfs mount or a fixed-size loopback
file.  Neither approach touches the host filesystem:

- **tmpfs**: `mount -t tmpfs -s <size> mnt`; write until `ENOSPC`; `umount` in a trap.
- **loopback**: `dd if=/dev/zero of=loop.img bs=1M count=<size>; losetup -f loop.img; mount ...`;
  write until `ENOSPC`; `losetup -d`; `rm loop.img` in a trap.

The tmpfs approach is simpler and fully contained in memory; the loopback approach tests the
actual block-device path but is slightly slower.  The harness scripts choose tmpfs by default.

## Parent-exit seam

Scenario 11 (parent-exit) requires a seam that the endpoint honours: a fault-injection
environment variable (e.g. `ATHLETIC_FAULT_HTTP_EXIT`) that makes the HTTP server task
return an error while the parent supervisor process continues running.  Until that seam
exists, the scenario is SKIPPED with the precise missing seam named.

## Scenario table

| # | Script | Status | Notes |
|---|---|---|---|
| 1 | `scenario-01-endpoint-kill.sh` | Partial | Sub 1 tests CLI connection-refused when endpoint is down; sub 2-5 SKIPPED (seam verified by `restate_kill_restart.rs`) |
| 2 | `scenario-02-restate-kill-during-fanout.sh` | SKIPPED | No Restate kill/restore seam at CLI level; verified by `restate_kill_restart.rs` |
| 3 | `scenario-03-reboot-with-full-census.sh` | SKIPPED | Requires active NationalCensus workflow; uses scratch base-dir only |
| 4 | `scenario-04-rolling-upgrade.sh` | SKIPPED | No dual-version endpoint registration seam |
| 5 | `scenario-05-http-error-taxonomy.sh` | SKIPPED | No HTTP error-injection proxy seam in chromiumoxide browser transport |
| 6 | `scenario-06-no-duplicate-evidence.sh` | SKIPPED | Requires active Restate invocations to query admin API |
| 7 | `scenario-07-domain-dedup.sh` | SKIPPED | Dedup is a Restate-level property, not observable via CLI |
| 8 | `scenario-08-global-budget.sh` | SKIPPED | `--max-concurrent` is per-process; two versions cannot coexist |
| 9 | `scenario-09-disk-full-fjall.sh` | Executable | tmpfs mount (~50 MB) in scratch directory; exercises Fjall write path |
| 10 | `scenario-10-disk-full-restate.sh` | Executable | tmpfs mount (~50 MB) in scratch directory; exercises Restate bifrost journal |
| 11 | `scenario-11-parent-exit.sh` | SKIPPED | Requires `ATHLETIC_FAULT_HTTP_EXIT` env var seam (not implemented) |
| 12 | `scenario-12-cross-midnight.sh` | SKIPPED | No clock-manipulation seam in census-service |
| 13 | `scenario-13-ai-review-failures.sh` | SKIPPED | No AI review fault-injection seam |
| 14 | `scenario-14-seal-refuses.sh` | Partial | Tests seal refusal on empty store; durable-run items SKIPPED without active Restate run |
| 15 | `scenario-15-full-backup-restore.sh` | SKIPPED | No pre-backup reference NationalReport; uses scratch base-dir only; covered by `backup_restore.rs` |
| 16 | `scenario-16-golden-census-determinism.sh` | SKIPPED | Empty stores produce tautological seal output; covered by `backup_restore.rs` |


## Design principles

1. **No green results from absence of faults.**  Every scenario either passes with observable
   evidence or skips with a precise reason.  A scenario cannot pass merely because the fault
   was not injected — the script must demonstrate that the fault was attempted.
2. **SKIPPED names the missing seam.**  E.g. `"SKIPPED: no fault-injection abort point after the Fjall commit in census-service"`.
3. **Deterministic over time.**  Where clock manipulation is involved, the script advances time
   rather than waiting.
4. **Idempotent.**  Running the same scenario twice from a fresh `$SCRATCH_STORE` must produce
   the same result.
5. **No Rust edits.**  Scripts may create directories, kill processes, inject errors via
   environment variables or configuration, but must not modify `crates/`, `Cargo.toml`, or
   any `.rs` file.
6. **Scratch-only.**  All file operations use `$SCRATCH_STORE` or `mktemp -d`. No production
   directories (`/var/lib`, `/etc/census-service`, etc.) are ever read, written, or deleted.

## Running

```bash
# Quick run (all scenarios, defaults)
tools/durability/run.sh

# Custom store and binaries
SCRATCH_STORE=/tmp/my-scenarios \
BINARY=target/debug/census-service \
SERVE_BINARY=target/debug/census-serve \
RESTATE_BINARY=/opt/restate/restate-server \
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
