# Operations runbook

Three processes, and only one of them writes the canonical store. `census-service` is the batch CLI
(acquisition, into the staging store); `census-serve` is the same adapters, store and reports exposed
as a Restate endpoint; `restate-server` is the node that holds the journal, the ingress and the admin
API. A crash of the endpoint resumes at the last recorded step because the journal lives in the node,
not in the endpoint process: an endpoint that dies mid-run leaves the invocation durably recorded,
and the restarted endpoint replays it instead of starting over.

```
census-serve --listen 127.0.0.1:9080 --data-dir var/census-service --max-concurrent 8 --drain-timeout 30
```

Everything lives under `--data-dir` (`var/census-service` by default): the Fjall store, the fetch
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
census-service --store <dir> teams --all-states --refresh
census-service --store <dir> meets --all-states --year 2026
census-service --store <dir> provider milesplit_results --all-states --limit 200
census-service --store <dir> collect --all-states --school-year 2026
census-service --store <dir> consolidate
census-service --store <dir> index
census-service --store <dir> report --print
census-service --store <dir> bests
census-service --store <dir> workbook
```
The `--school-year` flag names the roster's season start, never the cohort: 2026 = the 2026-27
season, the only season the class of 2027 is still enrolled in. Rosters are current-season
documents, so a 2027 label would file today's 2026-27 rosters under 2027-28 and compute grade 13
— not a grade — causing observations to be dropped.
`index` rewrites the derived index tables — source-object identities, retained conflicts and review
cases, coverage, and one snapshot of the pass — replacing their rows rather than appending, so the
chain can run daily without growing them. `run` chains every step above in one command.

Because the review table is one of those derived tables, the case state a seal reads is the state the
last stage left, and the two states are not close in size: the weekly chain leaves its own retained
conflicts (1 359 cases, 442 undecided on the 2026-09-23 store), while the review lane on top of it
files its reconciliation of every athlete row as well (51 835 cases, 9 448 undecided — the larger
number includes 9 005 athlete-identity cases the lane then has to ask about). Verdicts live in their
own table and survive an `index` pass, so a re-run re-mints cases but does not lose decisions; what
an operator chooses is how much of the review stage the census they seal claims. Nothing here is a
repair path — `index` re-deriving the table from the same rows *is* what keeps it bounded.

`teams --refresh` is the only step that re-reads association indexes; `collect` walks rosters and the
current season's result pages. No step re-reads the full historical corpus: every fetch is
content-hash cached, per-host paced (2 rps) and robots-checked. `deploy/systemd/census-service-collect.service`
`.timer` runs this chain weekly, appending `"$bin" --store "$out" fjall-stats` after `workbook`, against the
staging store at `/var/lib/census-service-staging`: the canonical store is written
only by an endpoint deployment, so a collector run can neither race the national run for the Fjall
writer lock nor write past the journal. Routing acquisition through the `Ingest` service is the
replacement for the staging hop, and the service drives it: the meet-index stage of a jurisdiction run
runs each planned source's walk with a recording and posts what it recorded through that source's
`Ingest` object, keyed `<slug>_<state>` and filed under the ISO week it was read (§3.1.1 of
`RESTATE_WORKFLOWS.md`). The walk's rows are still written to the store the run holds, so the meets a
later stage selects are the same rows they always were; the object is what makes the run's acquisition
measurable and durable outside the store. Nothing in the batch chain above routes yet, and the stages
that still write their own store — the state's own results index, the team-index and the roster
stages — are the ones left to route.

**Flag surface (critical):** The batch CLI uses `--store` (not `--data-dir`, which is the
`census-serve` unit's flag). The `collect` subcommand uses `--school-year` (not `--grad-year`).
`--data-dir` and `--grad-year` are rejected by the batch CLI with exit code 2.

**Jurisdiction default (critical):** with neither `--states` nor `--all-states`, the gather commands
(`teams`, `meets`, `collect`) cover **Wisconsin alone** (`crates/census-service/src/cli/mod.rs:239`,
pinned by `crates/census-service/src/cli/tests.rs:15`) — a one-state quick test, not the run scope.
The run scope is the 49 jurisdictions of `UsJurisdiction::CENSUS_SCOPE` (ADR-009): `--all-states`
selects it (`cli/mod.rs:238`), and the operational path is the service, which walks the whole scope on
its own (`crates/census-service/src/restate_services/open_work.rs:102`; a request naming a
jurisdiction outside it is refused at `restate_services/national.rs:73`). The `provider` subcommands
take the restriction form instead, where no flag means no restriction (`cli/mod.rs:252`, pinned at
`cli/tests.rs:34`).

## Shutdown and the drain certificate

`census-serve` drains on SIGTERM: it stops accepting work, finishes or cancels in-flight requests
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
The browser session pool uses `tabs × 4` as the queue capacity (where `tabs` is the `--max-concurrent`
count, which defaults to 8 and refuses only 0). For a unit running at `--max-concurrent 8`, the
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
and names the acceptance item that blocked it (§17, ADR-011).

    census-service workbook --grad-year 2027                    # build the export first
    census-service seal --grad-year 2027 --write                # certify the newest out/*.xlsx
    census-service seal --grad-year 2027 --workbook out/census-service-2026-09-21.xlsx

Exit code 0 prints the seal digest and what it covers; exit code 1 prints the unmet item and the
numbers behind it, for example `Run Metrics cohort 307652 != store 307653`. Sealing a census whose
phase ladder has not reached `exporting` is refused rather than granted: a store with no workbook,
no coverage classification or no consolidated snapshots cannot be sealed at all.

**Two of §70's items only the run's own objects can answer** — owed jurisdiction sweeps (§70 item 1)
and source objects that have accepted nothing (item 2) — and the store holds neither. The command
above therefore reads them as *unmeasured* and stays refused over them; that is the honest answer for
the store route, not a bug. A finished census seals through the deployment that ran it:

    census-service open-work                                    # what the run still owes, online
    census-service seal --ingress http://127.0.0.1:18095/ --write

`--season` and `--revision` name the run being certified (defaults: the 2026-27 season, revision 1)
— the run it *was submitted under*, never a new one. `--source-object <key>` names an ingest object
to read, repeatably, because an object key is the caller's to choose and the service cannot enumerate
them: naming none leaves item 2 unmeasured rather than reporting it as zero, and naming a key whose
acquisition never ran that way reads as an endpoint that never accepted an observation. The
meet-index stage routes each planned source's acquisition through `<slug>_<state>`, so an online seal
measures item 2 from the endpoints the operator names; a pass that ran before the route existed
leaves the same object readable and empty, and naming it is what refuses the seal rather than
certifying a count nobody took. The two routes otherwise assemble the same ladder, and an operator
reads one vocabulary either way.

`--write` records the sealed state in `out/seal.json`. What the seal checks: the phase ladder
against the store's own artifacts, the cohort the workbook's `Run Metrics` sheet names against the
store's count, the `Coverage` sheet's jurisdiction rows against the classifier, and the workbook's
bytes (sha256); it records the rows it read as `workbook_rows`. What it does not check: the cell
contents of the multi-million-row performance sheets — that reconciliation belongs to the workbook
verifier:

    census-service verify --store var/midwest-census --workbook var/midwest-census/out/census-service-2026-09-23.xlsx

Name the store the workbook was written from. Without `--store`, the verifier opens the default
store (`var/census-service`, a different census) and reports the workbook's first row as missing —
true of that store and nothing else. Because the verifier opens the store itself, the store must not
be held: stop `census-serve` for the check, then start it again.

A recorded seal is read by the next run, so a `seal.json` written by an older build is not inert:
where a field the current build requires was added later, every later seal dies parsing the file
rather than measuring the census (`Error: parsing out/seal.json / missing field access_conditions`,
2026-09-23). The remedy is an operator's — quietly repairing or ignoring the store's own record is
exactly what the seal exists to prevent — so move it aside rather than edit it: `out/superseded/` is
where this store keeps artifacts that must not be read as current, and the seal then runs.

## Backups

Cold copy only: stop the unit (drain certificate printed), copy the whole `--data-dir`, start again.
The store and the cache are a matched pair — a store copy without the cache only costs re-fetching,
but a cache copy without the store is worthless. `census-service import-legacy` migrates pre-Fjall
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
| `systemd/restate-server.service` | the Restate **node**: journal, metadata, ingress (18095) and admin (19095), loopback only, base-dir `/var/lib/census-service-restate` |
| `restate.toml` | the node's config, deployed to `/etc/census-service/restate.toml`; the only file that knows the ports. It used to live only on the deployed machine, where nothing could review it |
| `systemd/census-serve@.service` | the census **endpoint**, one instance per release: `census-serve@<release>` serves `releases/<release>/bin/census-serve` against `/var/lib/census-service/<release>` and logs to `/var/log/census-service/<release>/`. Two releases therefore never share a Fjall writer lock, and a release is identified by the commit it serves |
| `endpoint.env.example` | the per-release environment file, deployed to `/etc/census-service/endpoint-<release>.env`; it carries `CENSUS_LISTEN`, the one value an instance cannot derive from its name |
| `systemd/census-service-collect.{service,timer}` | weekly incremental acquisition into the **staging** store. The canonical store is written only by an endpoint deployment: a batch job writing it would race the national run for the same writer lock and bypass the journal |

Install with `systemctl enable --now restate-server.service`, then
`systemctl enable --now census-serve@<release>.service`, then register the endpoint with the node:

```
curl -X POST http://127.0.0.1:19095/deployments -H 'content-type: application/json' \
  -d '{"uri":"http://127.0.0.1:9080/"}'
```

Rolling forward is starting the new release's instance and retiring the old one. Both are separate
deployments in `curl http://127.0.0.1:19095/deployments`, and only the new one is redelivered to;
this is also how a rollback happens, by starting the previous release's instance again.

Every service declares its own retention in `crates/census-service/src/restate_services/*.rs`: most
services set 90 days of journal, 180 days of workflow completion, 30 days of idempotency; the
`Census` service overrides with `journal_retention = "1 hour"` and no workflow-completion retention.

## Quality gates

`tools/gate.sh` runs lanes in this order: `fmt`, `check`, `doc`, `tests`, `strict clippy`,
`production scan`, `domain type integrity`, `domain purity`, `module seams`, `debt ratchet`;
then tool-lanes `deny`, `audit`, `vet`, `machete`, `geiger`, `feature powerset`, `bench presence`;
`mutants` runs only with `--full`. The `ratchet` step compares clippy + scan debt against
`tools/quality-baseline.json`; absent tools print SKIP (or fail with `--release`).
