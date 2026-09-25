# Durability harness — fifteen failure-injection scenarios

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

The census lanes are qualified against Restate 1.7.10.  To run scenarios that depend on a
live Restate node (scenarios 01–08, 12, 15, 16):

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

| # | Name | What it breaks | Pass observable |
|---|---|---|---|
| 1 | endpoint-kill-before | SIGKILL endpoint before any HTTP request to Restate | The CLI command exits non-zero with a connection-refused error; re-run with a fresh invocation produces an identical result. Fjall effect count stays exactly 0 (no partial writes). |
| 2 | endpoint-kill-during | SIGKILL endpoint while a Restate `ctx.run` is executing | Re-invocation under the same key attaches to the existing invocation; the `NationalReport` shared handler returns the same `NationalReport` with the same `jurisdiction_summaries` count. Fjall effect count stays 1. |
| 3 | endpoint-kill-after-response | SIGKILL after the HTTP response body is fully written but before the TCP FIN | No duplicate effect: Fjall effect count stays 1; the report JSON is byte-identical to a clean run. |
| 4 | endpoint-kill-after-fjall-commit | SIGKILL immediately after `fdatasync` but before Restate step acknowledges | No duplicate evidence: `census-service report --print` shows exactly one set of athlete observations; no doubled `observations` in `fjall-stats`. |
| 5 | endpoint-kill-before-step-complete | SIGKILL at the last `ctx.run` await, before the step returns | Re-invocation replays from the journal; identical final artifacts (same `NationalReport` JSON, same `fjall-stats` observation counts). |
| 6 | restate-kill-during-fanout | SIGKILL Restate node mid-`NationalCensus/run` fanout across jurisdictions | After restart, every child `JurisdictionCensus` invocation resumes its last stage (not recreated from scratch); the run completes with `jurisdictions done` equal to the original request. |
| 7 | reboot-with-full-census | Reboot the host (simulated by killing Restate + endpoint simultaneously) during a 20-source `NationalCensus` with AI reviews, cooldown timers, and delayed retries | After full recovery (Restate restart + endpoint restart), the `NationalReport` shared handler returns a report with the same `jurisdiction_summaries` count; Fjall effect count matches the pre-crash count. |
| 8 | rolling-upgrade-v1-v2 | Start v1, launch a long `JurisdictionCensus`, then start v2 alongside v1 | The old invocation continues on v1 (same `JurisdictionReport` result); new invocations use v2; after draining v1, the final result is identical to a clean v2 run. |
| 9 | http-error-taxonomy | Inject 429+Retry-After, 403, 404, 500, timeout, connection reset, invalid HTML, corrupted JSON, oversized bodies via a local proxy | Each injected error produces the exact terminal error taxonomy the crawl lane defines (e.g., `CrawlError::RateLimited` for 429, `CrawlError::Forbidden` for 403, etc.); no retry exhaust leaks into the report. |
| 10 | no-duplicate-evidence | Kill endpoint after Fjall commit but before Restate acknowledgment | Post-recovery: `census-service report --print` shows exactly one set of observations; `fjall-stats` reports the same counts as the pre-crash run. No duplicate athlete rows. |
| 11 | domain-dedup | Submit the same logical operation twice via two different ingress requests | The domain operation id deduplicates the second submission: the `NationalReport` is identical to a single invocation, even after Restate's 180-day idempotency retention expires (verified via a mock timestamp). |
| 12 | global-budget-across-versions | Run two immutable endpoint versions simultaneously; each submits against the same source | Both versions enforce one global remote-origin budget: the total number of concurrent source fetches across both versions never exceeds `--max-concurrent`; no source is throttled by one version's load affecting the other. |
| 13 | disk-full-fjall | Force disk-full on the Fjall volume via a tmpfs mount (~50 MB); write until `ENOSPC`, verified via `trap` cleanup | The Fjall write fails with a storage error (not a silent completion); the pipeline state remains `Discovering` or `Acquiring` with no phantom `Complete` phase. Restate journal is intact. |
| 14 | disk-full-restate | Force disk-full on the Restate volume via a tmpfs mount (~50 MB); write until `ENOSPC`, verified via `trap` cleanup | The Restate worker fails to persist the bifrost journal; no invocation silently completes; the admin API reports paused invocations (not terminal failures). |
| 15 | parent-exit-on-http-kill | Kill the endpoint HTTP server task without terminating the parent supervisor process (requires `ATHLETIC_FAULT_HTTP_EXIT` env var seam) | The parent process (the census-serve binary) exits with a nonzero status; systemd's `Restart=on-failure` picks it up. Verified by checking the process tree: the supervisor PID is gone after the HTTP task dies. |
| 16 | cross-midnight-replay | Start a long workflow; advance the system clock forward by 24 hours; let the workflow complete | All journaled timestamps in the Fjall `journal` keyspace are byte-identical to what they would have been without the clock advance; the `NationalReport` is semantically identical. |
| 17 | ai-review-failures | Inject AI timeouts, malformed JSON responses, hallucinated case ids, conflicting verdicts, and a proposed match across a deterministic contradiction | Each failure mode produces the exact terminal error the review lane defines; the seal refuses with the appropriate `RetainedFindings` item named; no hallucinated case id leaks into the canonical entities. |
| 18 | seal-refuses-incomplete | Leave one jurisdiction incomplete, one source object unresolved, and one cohort decision unresolved | The seal command exits non-zero with `SealError::ItemUnmet` naming `JurisdictionSweepsTerminal`, `SourceObjectsTerminal`, and `CohortDecisionsTerminal` respectively (one at a time). Each refusal names the exact unmet item and its detail count. |
| 19 | full-backup-restore | Back up both Restate state and Fjall store; destroy both working directories; restore from backups; resume an unfinished workflow | After resume, the `NationalReport` shared handler returns a report identical to the pre-backup run (same `jurisdiction_summaries`, same `failures` count). The Fjall store's `fjall-stats` match the backup manifest. |
| 20 | golden-census-determinism | Run a full small-state golden census from a fresh store twice; compare all artifacts | The two runs produce byte-identical canonical JSON reports, the workbook has identical semantic content (sheet names, row counts, seal digest), identical work receipts, and identical seal digest prefix (`census-seal-v4` or later). |

### Mapping: scenarios to scripts

The fifteen scenario numbers in the task description map to the scripts below.  Some task
scenarios (e.g. scenario 1 with five sub-variants) are collapsed into a single script with
multiple fault points.  Where a seam does not exist, the script prints `SKIPPED` with the
specific missing seam named.

| Task scenario | Script | Notes |
|---|---|---|
| 1 (five kill points) | `scenario-01-endpoint-kill.sh` | Five sub-scenarios: before, during, after-response, after-fjall-commit, before-step-complete |
| 2 | `scenario-02-restate-kill-during-fanout.sh` | Requires Restate kill/restore seam |
| 3 | `scenario-03-reboot-with-full-census.sh` | Simulated reboot via simultaneous SIGKILL |
| 4 | `scenario-04-rolling-upgrade.sh` | Requires dual-version endpoint seam |
| 5 | `scenario-05-http-error-taxonomy.sh` | Requires HTTP error-injection proxy |
| 6 | `scenario-06-no-duplicate-evidence.sh` | Kill after Fjall commit, before Restate ack |
| 7 | `scenario-07-domain-dedup.sh` | Requires domain id dedup verification |
| 8 | `scenario-08-global-budget.sh` | Requires two concurrent endpoint versions |
| 9 | `scenario-09-disk-full-fjall.sh` | Uses tmpfs mount (~50 MB) to force ENOSPC on Fjall volume |
| 10 | `scenario-10-disk-full-restate.sh` | Uses tmpfs mount (~50 MB) to force ENOSPC on Restate volume |
| 11 | `scenario-11-parent-exit.sh` | Requires `ATHLETIC_FAULT_HTTP_EXIT` env var seam (not yet implemented) |
| 12 | `scenario-12-cross-midnight.sh` | Requires clock manipulation |
| 13 | `scenario-13-ai-review-failures.sh` | Requires AI review injection |
| 14 | `scenario-14-seal-refuses.sh` | Requires incomplete-state seal verification |
| 15 | `scenario-15-full-backup-restore.sh` | Requires full backup/restore of both stores |
| — | `scenario-16-golden-census-determinism.sh` | Two fresh runs, identical artifacts |

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
| 1 | One or more scenarios FAIL |
| 2 | Invalid arguments or missing required tool |
