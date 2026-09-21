# Operations runbook

Two processes, one store. `midwest-census` is the batch CLI; `midwest-serve` is the same adapters,
store and reports exposed as a Restate endpoint so a crash resumes at the last recorded step.

```
midwest-serve --listen 127.0.0.1:9080 --data-dir var/midwest-census --max-concurrent 8 --drain-timeout 30
```

Everything lives under `--data-dir` (`var/midwest-census` by default): the Fjall store, the fetch
cache with its content hashes, the consolidated `out/*.jsonl` snapshots and the reports.

## Weekly incremental refresh

```
midwest-census --data-dir <dir> teams --states WI,MN,... --refresh
midwest-census --data-dir <dir> collect --states WI,MN,... --grad-year 2027
midwest-census --data-dir <dir> consolidate
midwest-census --data-dir <dir> report --print
midwest-census --data-dir <dir> bests
midwest-census --data-dir <dir> workbook
```

`teams --refresh` is the only step that re-reads association indexes; `collect` walks rosters and the
current season's result pages. No step re-reads the full historical corpus: every fetch is
content-hash cached, per-host paced (2 rps) and robots-checked. `deploy/systemd/*.service`
`.timer` runs exactly this chain weekly.

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

## Backups

Cold copy only: stop the unit (drain certificate printed), copy the whole `--data-dir`, start again.
The store and the cache are a matched pair — a store copy without the cache only costs re-fetching,
but a cache copy without the store is worthless. `midwest-census import-legacy` migrates pre-Fjall
JSONL journals one time; keep the old journals until a `report` matches the pre-migration numbers.

## Quality gates

`tools/gate.sh` runs fmt, `check --all-targets`, the strict clippy set, tests, the panic-macro scan,
the forbidden-construct scan and the ratchet, then the optional cargo subcommand lanes (`audit`,
`deny`, `vet`, `geiger`, `machete`); absent tools print SKIP with the install command instead of
failing. `tools/quality-baseline.json` records the remaining debt and `tools/ratchet.py` fails the
gate on any increase, so the numbers in `docs/HARDENING-PROGRAM.md` can only move down.
