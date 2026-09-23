# Operations runbook

Three processes, and only one of them writes the canonical store. `midwest-census` is the batch CLI
(acquisition, into the staging store); `midwest-serve` is the same adapters, store and reports exposed
as a Restate endpoint; `restate-server` is the node that holds the journal, the ingress and the admin
API. A crash of the endpoint resumes at the last recorded step because the journal lives in the node,
not in the endpoint process: an endpoint that dies mid-run leaves the invocation durably recorded,
and the restarted endpoint replays it instead of starting over.

```
midwest-serve --listen 127.0.0.1:9080 --data-dir var/midwest-census --max-concurrent 8 --drain-timeout 30
```

Everything lives under `--data-dir` (`var/midwest-census` by default): the Fjall store, the fetch
cache with its content hashes, the consolidated `out/*.jsonl` snapshots and the reports.

## Weekly incremental refresh

`meets` enumerates each state's published meets into `source_meets` (about one index request per
fifty meets, journaled per page, and the walk ends on the index's own repeat signal rather than a
page bound). `provider milesplit_results` then reads those meets whole: one results-page request per
meet, which lists every result file the meet has, plus one `/raw` request per file — result sets are
journaled by id, so a weekly run reads only what it has not read before. `--limit` bounds the meets
taken per state; the census does not filter meets by level, so a middle-school meet listed under `hs`
costs its requests and yields no canonical athlete.

```
midwest-census --store <dir> teams --states WI,MN,... --refresh
midwest-census --store <dir> meets --states WI,MN,... --year 2026
midwest-census --store <dir> provider milesplit_results --states WI,MN,... --limit 200
midwest-census --store <dir> collect --states WI,MN,... --school-year 2027
midwest-census --store <dir> consolidate
midwest-census --store <dir> index
midwest-census --store <dir> report --print
midwest-census --store <dir> bests
midwest-census --store <dir> workbook
```

`index` rewrites the derived index tables — source-object identities, retained conflicts and review
cases, coverage, and one snapshot of the pass — replacing their rows rather than appending, so the
chain can run daily without growing them. `run` chains every step above in one command.

`teams --refresh` is the only step that re-reads association indexes; `collect` walks rosters and the
current season's result pages. No step re-reads the full historical corpus: every fetch is
content-hash cached, per-host paced (2 rps) and robots-checked. `deploy/systemd/midwest-census-collect.service`
`.timer` runs exactly this chain weekly, against the **staging** store: the canonical store is written
only by an endpoint deployment, so a collector run can neither race the national run for the Fjall
writer lock nor write past the journal. Routing acquisition through the `Ingest` service is the
replacement for the staging hop; no CLI subcommand drives it yet.

**Flag surface (critical):** The batch CLI uses `--store` (not `--data-dir`, which is the
`midwest-serve` unit's flag). The `collect` subcommand uses `--school-year` (not `--grad-year`).
`--data-dir` and `--grad-year` are rejected by the batch CLI with exit code 2.

## Shutdown and the drain certificate

`midwest-serve` drains on SIGTERM: it stops accepting work, finishes or cancels in-flight requests
within `--drain-timeout`, finalizes the store, and prints

```
drained: accepted=<n> completed=<n> cancelled=<n> timed_out=<n> aborted=<n> panicked=<n>
```

Keep `TimeoutStopSec` above `--drain-timeout` (60 vs 30 in the unit) or systemd SIGKILLs mid-drain
and the certificate is never printed. A non-zero `panicked` or `aborted` count means a task ended
outside its own result path: the worker logs `browser job panicked; requesting shutdown` /
`browser job aborted; requesting shutdown`, pending callers receive
`browser worker task panicked` / `browser is unavailable`, and the browser is restarted rather than
reused. Treat any such run as a bug report, not an incident to retry into silence.

## Ingress posture

The Restate endpoint binds to loopback only (`127.0.0.1:9080` in the systemd unit). This is
enforced in code (`bootstrap.rs:NonLoopbackListen`) and must not be changed until owner decision 4
lands. Remote workers access the endpoint through a reverse proxy or tunnel — never by changing the
bind address.

## Session-pool sizing

The browser session pool uses `tabs × 4` as the queue capacity (where `tabs` is the validated
`--max-concurrent` count, validated at 1..=8). For a unit running at `--max-concurrent 8`, the
queue holds 32 pending requests. When the queue is full, `BrowserError::Unavailable` is returned
immediately. Plan concurrent consumers so they do not exceed `tabs × 4` in steady state.

## Observability

- `RUST_LOG=info` (or `debug`) is the switch; spans exist for `browser.actor`, `browser.handler`,
  `browser.job` (with `slot` and `nonce`) and the fetch operations, so a slow crawl can be attributed
  to a specific page slot.
- Browser health is a state machine — `Ready`, `Challenged`, `CoolingDown`, `HumanRequired`,
  `Restarting`, `Stopped`. A `HumanRequired` latch is deliberate: it means a challenge page was
  served and the tool must not try to defeat it. Fix the access path (different egress, slower pace)
  instead of clearing the flag.
- `fjall-stats` prints per-table observation counts and on-disk footprint; it is the cheapest
  "is the store growing the way the reports say" check.

## Sealing a census

A run is not finished when the workbook exists; it is finished when the census is sealed. The seal
assembles §70's evidence from the store (cohort counts, coverage gaps, conflicts, exhausted
retries), reads the exported workbook back, and either completes the census with a digest or refuses
and names the acceptance item that blocked it (§17, ADR-007).

    midwest-census workbook --grad-year 2027                    # build the export first
    midwest-census seal --grad-year 2027 --write                # certify the newest out/*.xlsx
    midwest-census seal --grad-year 2027 --workbook out/midwest-census-2026-09-21.xlsx

Exit code 0 prints the seal digest and what it covers; exit code 1 prints the unmet item and the
numbers behind it, for example `Run Metrics cohort 307652 != store 307653`. Sealing a census whose
phase ladder has not reached `exporting` is refused rather than granted: a store with no workbook,
no coverage classification or no consolidated snapshots cannot be sealed at all.

`--write` records the sealed state in `out/seal.json`. What the seal checks: the phase ladder
against the store's own artifacts, the cohort the workbook's `Run Metrics` sheet names against the
store's count, the `Coverage` sheet's jurisdiction rows against the classifier, and the workbook's
bytes (sha256). What it does not check: the cell contents of the multi-million-row performance
sheets — that reconciliation belongs to the workbook verifier, and the seal records
`workbook_rows: 0` rather than a count it did not take.

## Backups

Cold copy only: stop the unit (drain certificate printed), copy the whole `--data-dir`, start again.
The store and the cache are a matched pair — a store copy without the cache only costs re-fetching,
but a cache copy without the store is worthless. `midwest-census import-legacy` migrates pre-Fjall
JSONL journals one time; keep the old journals until a `report` matches the pre-migration numbers.

### Backup drill

`tools/ops-backup-drill.sh <store-dir>` automates a cold-copy verification: it copies the store,
runs `fjall-stats` on both copies, runs `consolidate` on the copy, and compares report row counts.
Emits `PASS: backup drill completed successfully` on success or `FAIL: <reason>` on mismatch.

### Failure modes

| Condition | Effect | Mitigation |
|---|---|---|
| Lock held | Fjall returns a lock error on the copy | Stop the source unit (drain certificate printed) before copying |
| Cache absent | Store copy works but re-fetches all uncached URLs | Acceptable; re-fetch cost is the price of cache-only backup |
| Marker present | `import-legacy` re-imports on next open, duplicating observations | Clear the `meta` marker (`--store <dir> import-legacy` handles this) or use a fresh copy |

## Deploy artifacts

Five files ship in `deploy/`:

| Unit / file | Purpose |
|---|---|
| `systemd/restate-server.service` | the Restate **node**: journal, metadata, ingress (18095) and admin (19095), loopback only, base-dir `/var/lib/midwest-census-restate` |
| `restate.toml` | the node's config, deployed to `/etc/midwest-census/restate.toml`; the only file that knows the ports. It used to live only on the deployed machine, where nothing could review it |
| `systemd/midwest-serve@.service` | the census **endpoint**, one instance per release: `midwest-serve@<release>` serves `releases/<release>/bin/midwest-serve` against `/var/lib/midwest-census/<release>` and logs to `/var/log/midwest-census/<release>/`. Two releases therefore never share a Fjall writer lock, and a release is identified by the commit it serves |
| `endpoint.env.example` | the per-release environment file, deployed to `/etc/midwest-census/endpoint-<release>.env`; it carries `CENSUS_LISTEN`, the one value an instance cannot derive from its name |
| `systemd/midwest-census-collect.{service,timer}` | weekly incremental acquisition into the **staging** store. The canonical store is written only by an endpoint deployment: a batch job writing it would race the national run for the same writer lock and bypass the journal |

Install with `systemctl enable --now restate-server.service`, then
`systemctl enable --now midwest-serve@<release>.service`, then register the endpoint with the node:

```
curl -X POST http://127.0.0.1:19095/deployments -H 'content-type: application/json' \
  -d '{"uri":"http://127.0.0.1:9080/"}'
```

Rolling forward is starting the new release's instance and retiring the old one. Both are separate
deployments in `curl http://127.0.0.1:19095/deployments`, and only the new one is redelivered to;
this is also how a rollback happens, by starting the previous release's instance again.

Every service declares its own retention in `crates/midwest-census/src/restate_services/*.rs`:
90 days of journal, 180 days of workflow completion, 30 days of idempotency. The server's own
defaults are one day, which is shorter than a national census can run.

## Quality gates

`tools/gate.sh` runs fmt, `check --all-targets`, the strict clippy set, tests, the panic-macro scan,
the forbidden-construct scan and the ratchet, then the optional cargo subcommand lanes (`audit`,
`deny`, `vet`, `geiger`, `machete`); absent tools print SKIP with the install command instead of
failing. `tools/quality-baseline.json` records the remaining debt and `cargo xtask ratchet` fails the
gate on any increase, so the numbers in `docs/HARDENING-PROGRAM.md` can only move down.
