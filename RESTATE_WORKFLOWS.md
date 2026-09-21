# Restate Workflows

How this repo uses [Restate](https://restate.dev) to make the browser/scrape side durable: which
process hosts which service, what each handler does, what state is written per invocation, and what
the code does (and does not) promise about retries and replay.

Everything below is read from source. Where it comes from:

| Path | Contents |
|---|---|
| `src/runtime/worker.rs`, `src/runtime/**` | Pipeline-worker endpoint and its 13 Restate definitions (`run/`, `row_worker/`, `rankings_collection/` hold submodules) |
| `crates/midwest-census/src/restate_services.rs` | `Census`, `Ingest`, `Sweep` and `build_endpoint` |
| `crates/midwest-census/src/bin/midwest-serve.rs`, `.../bootstrap.rs`, `.../store.rs` | Census endpoint process, supervisor and store |
| `src/cli.rs`, `src/cli/{args,transport,flow_control}.rs`, `src/runtime/{protocol,run_protocol,row_protocol}.rs` | Operator surface (ingress clients, deployment, flow control) and retry/revision vocabulary |

Two path notes. There is **no** `src/restate.rs` and no `src/restate_services.rs` in the root crate;
the census registrations live at `crates/midwest-census/src/restate_services.rs`. Line numbers are as
read on 2026-09-21, and other agents edit this tree: when a number and a name disagree, trust the
name.

## 1. Overview

Restate is the scheduler, journal and K/V store for work too long-lived and too failure-prone to own
in-process: headed-browser acquisitions that wait on humans, workbook rows that fan out across
duplicate athlete identities, multi-window collection sweeps. The worker performs the effects;
Restate decides when they run, what has already run, and what to re-run. The code states the split
directly: "Restate owns scheduling. Source workflows and model SDK policies have distinct retry
budgets. Unacknowledged effects may repeat during crash recovery" (`src/runtime/protocol.rs:93-94`).

Two processes serve two endpoints.

| Process | Started by | Binds | Serves |
|---|---|---|---|
| **Pipeline worker** | `athletic-rust-pipeline worker --config <toml> --bind 127.0.0.1:19181` — `Command::Worker` (`src/cli/args.rs:15-21`) → `runtime::worker::serve` (`src/cli.rs:33`) | default `127.0.0.1:19181` (`src/cli/args.rs:19-20`) | the 13 definitions in §2.1 |
| **Census service** | `midwest-serve --listen 127.0.0.1:9080 --data-dir var/midwest-census --max-concurrent 8 --drain-timeout 30` (`crates/midwest-census/src/bin/midwest-serve.rs`) → `bootstrap::serve` | default `127.0.0.1:9080` (`bootstrap.rs:256`); flags parsed by `ServeOptions::from_env` (`bootstrap.rs:264-265`) | `Census`, `Ingest`, `Sweep` |

`midwest-census serve` is **not** a third server: it prints the `midwest-serve` argv built from
`ServeOptions::default()`, so the printed command cannot drift from what the binary parses
(`crates/midwest-census/src/main.rs:100-101`, `:725-739`).

Registration is an operator step, not a boot step:
`athletic-rust-pipeline deploy --admin http://127.0.0.1:19070/ --endpoint http://127.0.0.1:19181/`
(`src/cli/args.rs:22-28`) runs `flow_control::ensure_source_scope`, then `POST`s `{"uri": <endpoint>}`
to the admin `deployments` resource (`src/cli/transport.rs`).

Both processes refuse a non-loopback bind: the worker with `bail!("worker must bind a loopback
address")` (`src/runtime/worker.rs:14-16`), the census endpoint because it "has no identity key
configured, so it must not be reachable from another host" (`bootstrap.rs:125-129`). The census side
also holds the store exclusively (`Store::open` takes an exclusive lock,
`crates/midwest-census/src/main.rs:5-7`), so `midwest-serve` cannot share a `--data-dir` with a batch
subcommand.

## 2. Service and object catalog

Wire names come from struct names (`restate_services.rs:1-5`), and the e2e test pins the census set to
`["Census", "Ingest", "Sweep"]` (`crates/midwest-census/tests/fjall_restate_e2e.rs:40`). Renaming a
struct is therefore a breaking API change; the SDK escape hatch is `#[handler(name = "...")]`.

### 2.1 Pipeline worker (`src/runtime/worker.rs:23-60`)

| Definition | Kind | Object key | Handlers | Visibility |
|---|---|---|---|---|
| `PipelineControl` | service | — | `prepare`, `run_and_export`, `rankings_progress`, `rankings_pause`, `rankings_resume` | public |
| `BrowserSession` | object | `BROWSER_SESSION_KEY` = `"profile-0"` (`browser_session.rs:11`) | `await_ready`, `status`, `capture_ready`, `recover` | public |
| `WorkbookImport` | object | `import_worker::key(request)` — fingerprint of `(INGESTION_REVISION, ImportRequest)` | `load` | `ingress_private` |
| `RunCoordinator` | object | `"global"` (any other key is terminal) | `run`, `status`, `page`, `snapshot` | public |
| `ExportWorker` | object | `ExportRequest::key()` | `publish` | public |
| `RowWorker` | object | `RowJob::key()` | `process` | `ingress_private` |
| `QueryWorker` | object | `QueryJob::key()` | `gather` | `ingress_private` |
| `ProfileWorker` | object | `ProfileJob::key()` | `identify`, `gather` | `ingress_private` |
| `ReviewCase` | object | `case_key(config, ReviewInput)` | `review` | `ingress_private` |
| `LocalReviewer` | object | `ModelLane::key()`, checked by `validate_lane` | `review` | `ingress_private` |
| `SourceGateway` | object | `"global"` (enforced by `fetch`) | `fetch`, `admit`, `await_admission`, `observe` | `ingress_private` |
| `SourceCache` | object | `SourceRequest::key()` | `fetch` | `ingress_private` |
| `RankingsCollectionState` | object | `collection_fingerprint(scope.revision, source_snapshot)` | `start_or_resume`, `step`, `pause`, `resume`, `progress`, `snapshot_ref` | `ingress_private` |

Definitions that block on a human (`BrowserSession`, `PipelineControl`, `RunCoordinator`) are public;
everything invoked only by a sibling handler is `ingress_private`.

### 2.2 Census service (`build_endpoint`)

| Definition | Kind | Handlers | Notes |
|---|---|---|---|
| `Census` | service | `status`, `consolidate`, `report`, `bests`, `workbook` | Holds the concurrency semaphore (`self.permit()`); every heavy step goes through `blocking(...)` |
| `Ingest` | object | `record`, `state`, `complete_window` | Key = endpoint string; the whole state is one value under `"state"` (§3.1); `state` is a shared (read-only) handler |
| `Sweep` | workflow | `run`, `interrupt` | `run` chains windows; `interrupt` is a shared handler that resolves `STOP_SIGNAL` on the target invocation |

## 3. The work-snapshot objects

Each object is a small durable state machine: a key, a value written per invocation, and enough
bookkeeping for a re-invocation to pick up exactly where the last committed write left off. Nothing
snapshots the browser or the store; the durable state is the Restate K/V value plus the journal.

### 3.1 `Ingest` — per-endpoint cursor and window bookkeeping (census)

All of an endpoint's durable state is written as a single value under one key:

```text
object = endpoint string
state  = "state"                                   # KEY_STATE (restate_services.rs:38)
value  = IngestState { endpoint, total_observations, cursor, last_appended_at, windows }
```

The code's stated reason: the endpoint's "whole durable state, written as one value so a partially
updated endpoint (cursor advanced but totals not, or the reverse) cannot exist"
(`restate_services.rs:36-38`). Per handler:

| Handler | Writes | Notes |
|---|---|---|
| `record` | one `ctx.set(KEY_STATE, ...)` | Appends observations, then advances `cursor`, `total_observations`, `last_appended_at`; converts the accepted count to `u64` with `try_from` (a non-fitting host is terminal, not clamped) |
| `state` | nothing | Shared (read-only) handler: returns the same value without writing |
| `complete_window` | one `ctx.set(KEY_STATE, ...)`, only on change | Rejects an empty label terminally; pushes the label only if absent, then sorts — window bookkeeping is a set, so a duplicate declaration is a no-op |

`appended` is produced inside `ctx.run`, so a replay of an acknowledged step reuses the journaled
count instead of re-appending.

### 3.2 `RunCoordinator` — per-run progress and sealed pages (pipeline)

The pipeline equivalent lives in the `global`-keyed `RunCoordinator`, spread over three slot families:

| Slot | Written by | Content |
|---|---|---|
| `progress:<run>` | `Results::{new, update_collection_ref, record, finish}` (`src/runtime/run/results.rs`) | `RunProgress { request, coverage, pages, pending_rows, complete, started_at_unix_ms, updated_at_unix_ms, summary, collection_ref }` |
| `page:<run>:<page>` | `Results::flush` | digest of the sealed `RunPage` |
| `result:<run>` | `Results::finish` | digest of the `RunSummary` |

`<run>` is `RunRequest::key()`, a fingerprint over `(RUN_REVISION, SOURCE_PARSER_REVISION, request)`
(`run_protocol.rs:9`, `:44-60`). Cursor discipline:

* `Results::record` appends one `RowReference` per completed row job and rewrites `progress:<run>`
  after every completion, so the cursor advances row by row;
* every `RESULT_PAGE_ROWS` (= 64, `run_protocol.rs:12`) pending rows, `flush` publishes a page
  artifact, stores its digest in the next page slot, then increments `progress.pages`;
* `finish` flushes the tail, publishes the summary, sets `complete = true`, and writes `result:<run>`;
  it refuses to complete early ("run ended before every selected row reached a terminal result");
* `run` short-circuits on the `result:<run>` slot, so a re-submitted finished run returns that digest;
  `snapshot` captures `progress` plus every sealed page digest (rejecting a key or page count that
  does not fit), treats a missing sealed page as terminal, and never lets later progress alter the
  captured prefix.

### 3.3 Other state-bearing objects

| Object | Keys | Purpose |
|---|---|---|
| `ExportWorker` | `owner-run` (`:107`), `stage-receipt` (`STAGE_STATE`, `:26`), `published-result` (`RESULT_STATE`, `:27`) | Two-phase publication: the staged bundle receipt is durable before installation; a missing commit receipt means "the two-file bundle is still in progress" |
| `RankingsCollectionState` | `state` | `CollectionState { source_snapshot, scope, phase, generation, events, current_event_index, final_snapshot, catalog_ref, plan_ref, absent_families, catalog_outcome, pause_reason, last_outcome, pause_detail }`; `step` is fenced by `generation` — a stale generation returns the current state instead of acting |
| `BrowserSession` | `status`, `cooldown-until-ms`, `cooldown-expired`, `challenge-started-ms`, `recovery-issued` | Readiness, cooldown and the one-shot recovery latch; `await_ready` clears `cooldown-expired` only after a navigation that reached `Ready` |
| `SourceGateway` / `SourceCache` | `not-before-ms`, `blocked` / `result` (non-rankings only) | Admission pacing and the retained source block (`blocked` is also read out of Restate's SQL interface by `flow_control::ensure_source_scope`); cached `FetchOutcome`. Ranking pages are deliberately not cached: "explicit retries of unvalidated pages must not replay an HTTP-200 response that failed domain parsing" |
| `WorkbookImport` | `manifest` | Import reuse; the cached manifest is re-verified against the requested workbook before reuse |
| `RowWorker`, `QueryWorker`, `ProfileWorker`, `ReviewCase`, `LocalReviewer` | `result`; also `probe` (`ProfileWorker`), `assignment` (`ReviewCase`), `blocked` (`LocalReviewer`) | Terminal-artifact short-circuits: each handler checks its result slot first and returns the stored digest |

### 3.4 What a re-invocation observes

* State: the last committed value under the key, read with `ctx.get`; a key never written reads as
  `None`/default, not an error. Completed pure steps return the journaled `ctx.run` value.
* Iteration: rebuilt, not restored. `SourceRows` is "Ephemeral iterator only. Restate journals calls
  and completion order; this is reconstructed from the immutable import on replay, never a checkpoint
  owner" (`run/source_rows.rs:23-25`); artifacts are re-read by content digest, so "verified immutable
  reads may repeat on replay; only references enter journals" (`artifacts.rs:7-8`).

## 4. Boot and restore

Nothing in either process scans for in-flight work at start. Both open local state, bind an endpoint,
and depend on the server to re-deliver open invocations; recovery is therefore two mechanisms —
server-side journal/state, and content-addressed local artifacts.

### 4.1 `midwest-serve` (`bootstrap::serve_until`)

1. `init_tracing()` (idempotent: a second call is a no-op, not a panic).
2. Open the store on the blocking pool — `create_dir_all(data_dir)`, then `Store::open`, because
   "Opening the store is synchronous, fsync-heavy work: it belongs on the blocking pool"
   (`bootstrap.rs:112-124`). `Store::open` is also where legacy recovery happens: pre-Fjall
   `entities/*.jsonl` and `journal/*.jsonl` are imported exactly once and marked under `meta`, and the
   sequence counter is "seeded from the last key present at open time" (`store.rs:1-40`).
3. Reject a non-loopback `--listen`; bind the listener; install the stop future (SIGINT/SIGTERM or
   the caller's `shutdown`, recording a `StopReason`), then build the endpoint with
   `restate_services::build_endpoint(store, max_concurrent)` and spawn exactly one task running
   `HttpServer::new(endpoint).serve_with_cancel(listener, stop)`.
4. Drain that region inside `drain_timeout` (`JoinSet` + deadline; survivors are aborted and counted,
   not leaked), then finalize: `store.flush()` (`PersistMode::SyncAll`) and drop the store — "after the
   region is empty: nothing can still be writing when the journal is synced".
5. Return `DrainReport { accepted, completed, cancelled, timed_out, aborted, panicked, stop_reason }`
   (`bootstrap.rs:83-92`), which `midwest-serve` prints as one line before exiting.

### 4.2 Pipeline worker (`runtime::worker::serve`)

1. Reject a non-loopback bind; install SIGTERM/SIGINT streams; bind the listener.
2. `Runtime::open(config)` — load `WorkerConfig`, open the `ArtifactStore`, build the bounded
   `reqwest` client (no retries, redirects or proxy), size the CPU semaphore, open the `TaskTracker`;
   the browser slot starts empty.
3. Build the endpoint by binding all 13 definitions of §2.1, each handed `runtime.clone()` (except
   `SourceCache`, a unit struct).
4. `HttpServer::new(endpoint).serve_with_cancel(listener, shutdown(terminate, interrupt))`, then
   `runtime.drain()` — shut down the browser manager, close the CPU semaphore and tracker, wait for
   tracked tasks. `Runtime::blocking` refuses new work once the tracker is closed ("worker is
   draining").

### 4.3 What boot does *not* do

There is no boot-time enumeration of runs, cursors or workflows, and no CLI verb to cancel a run: the
command set is `worker`, `deploy`, `start`, `status`, `browser-start`, `browser-status`, `export`,
`verify`, `rankings-status`, `rankings-pause`, `rankings-resume` (`src/cli/args.rs:14-79`). Recovery is
re-delivery plus re-submission:

* an open invocation is retried by the server under the definition's
  `invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")`;
  after four attempts it is **paused** for an operator, not failed forever;
* an operator-visible run is reattached by re-running the same command, which carries the same
  idempotency key and therefore rejoins the existing invocation;
* `start --output <file>` journals `RunCoordinator::run` and then `ExportWorker::publish` inside one
  handler, "so client disconnects cannot lose final publication".

## 5. Endpoint wiring

### 5.1 HTTP surface

Both endpoints use `restate_sdk::http_server::HttpServer` (`restate-sdk = "=0.12.0"` with
`http_server`, `aws_lc_rs`, `reqwest-client`: `Cargo.toml:29`, `crates/midwest-census/Cargo.toml:28`)
and shut down with `serve_with_cancel`, so a stop request stops intake and lets in-flight invocations
finish. Discovery is served by the SDK — the e2e test notes it "routes any path whose last segment is
`discover`" and asserts the manifest advertises `Census`, `Ingest`, `Sweep`
(`crates/midwest-census/tests/fjall_restate_e2e.rs:1-30`, `:513-519`); no `/discover` assertion exists
for the pipeline worker in `src/` or `tests/`.

Handler bodies take and return `Json<T>`; failures are `HandlerError`, where `TerminalError::new` (or
`new_with_code`) means "do not retry" and a bare `HandlerError` means "retry". The ingress clients
surface Restate's terminal message instead of a bare status: `transport::ingress_error` decodes
`{"code":…,"message":…}` so an operator sees *why* a call was rejected
(`src/cli/transport.rs:13-27`). Codes used in-repo include 404 (`run not found`), 408 (browser
readiness bound or deadline exhausted) and 409 (`browser is not ready; collection remains paused`).

### 5.2 CLI → handler map

| Command | Handler call | Idempotency key |
|---|---|---|
| `start` (no `--output`) | `PipelineControl::prepare` → `RunCoordinator::run` | `preparation_key(request)`, then the run key |
| `start --output <file>` | `PipelineControl::prepare` → `PipelineControl::run_and_export` | `preparation_key`, then `fingerprint(("run-and-export-v1", request))` |
| `status --run <digest>` | `RunCoordinator::status` | — |
| `browser-start` | `BrowserSession::await_ready { operator: true }` (submitted, not awaited) | — |
| `browser-status` | `BrowserSession::status` | — |
| `export --run <digest> --output <file>` | `ExportWorker::publish` | `fingerprint(("native-export-v4", request))`; the CLI then polls `InvocationHandle::output()` for up to 86 400 s |
| `rankings-status` / `-pause` / `-resume` | `PipelineControl::rankings_progress` / `rankings_pause` / `rankings_resume` | — |
| `deploy` | no handler: admin `POST /deployments` after flow-control checks | — |

Ingress clients are loopback-only by construction: `transport::local_origin` rejects any origin that
is not a loopback HTTP(S) origin without credentials, path, query or fragment; the admin/ingress
clients use a 300 s timeout with retries and redirects disabled.

### 5.3 Request/response shapes

* Pipeline: `PrepareRequest` → `RunRequest`; `RunRequest` → `EvidenceDigest`; `EvidenceDigest` →
  `Option<RunProgress>`; `PageRequest { run, page }` → `Option<EvidenceDigest>`; `EvidenceDigest` →
  `Option<ExportSnapshot>`; `ExportRequest` → `PublishedExport`; `RowJob` → `EvidenceDigest`;
  `SourceResource` → `FetchOutcome`; `ReadinessRequest` / nothing → `BrowserStatus`.
* Census: `StatusReply`, `Consolidate*`, `Report*`, `Bests*`, `Workbook*`, `IngestRequest` →
  `IngestReply`, `WindowRequest` → `IngestState`, `SweepRequest` → `SweepReport`.
* Counts on the wire are `u64`, not `usize` — "the wire shape must not change with the host pointer
  width" (`IngestReply`).

### 5.4 Timeouts and bounds

Definition attributes (omitted option = not declared, so the SDK/server default applies):

| Definitions | Attributes |
|---|---|
| `PipelineControl`, `WorkbookImport`, `RunCoordinator`, `ExportWorker`, `RowWorker`, `QueryWorker`, `ProfileWorker`, `ReviewCase` | `inactivity_timeout = "2h"`, `journal_retention = "30 days"`, `idempotency_retention = "30 days"`, retry `1s / 4 attempts / pause` (`SourceGateway` is identical minus `idempotency_retention`) |
| `RankingsCollectionState` | `inactivity_timeout = "2h"`, both retentions `"30 days"`, **no** `invocation_retry_policy` |
| `SourceCache` | `lazy_state`; no `inactivity_timeout`; both retentions `"30 days"`; retry declared without `initial_interval` (`max_attempts = 4, on_max_attempts = "pause"`) |
| `BrowserSession` | `inactivity_timeout = "26h"`, `journal_retention = "30 days"`, no `idempotency_retention`, retry `1s / 4 / pause` (a human may be clearing a challenge) |
| `LocalReviewer` | `inactivity_timeout = "10m"`, `journal_retention = "30 days"`, no `idempotency_retention`, retry `1s / 4 / pause` |
| `Census`, `Ingest`, `Sweep` | SDK/server defaults (bare macros) |

Handler-internal bounds:

| Where | Value |
|---|---|
| Long-wait loops | Browser readiness: `MAX_OBSERVATIONS` = 17 280 polls at `POLL_INTERVAL` = 5 s, hard deadline `READINESS_DEADLINE_MS` = 24 h. Sweep: `windows` default 1, `window_seconds` default 1, `MAX_SWEEP_WINDOWS` = 366, `MAX_SWEEP_ENDPOINTS` = 256 |
| `Ingest::record` | `MAX_ROWS_PER_REQUEST` = 50 000 |
| Row fan-out | request `concurrency` ≤ 256, bounded again by `row_concurrency`; `MAX_CANDIDATES` = 4 096; `MAX_PROFILE_BYTES_PER_ROW` = 8 MiB |
| Run size / paging / attempts | `MAX_RUN_ROWS` = 2 097 152, `RESULT_PAGE_ROWS` = 64; up to 64 attempts per source operation (`dispatch.rs`) |
| Client-side waits | `export` polls for up to 86 400 s and does not cancel the invocation on timeout; ingress/admin clients use a 300 s timeout with response bodies capped (65 536 bytes on admin reads) |
| Drains | census `--drain-timeout` default 30 s, `DEFAULT_MAX_CONCURRENT` = 8; `Runtime::drain` waits on tracked blocking work; browser `SHUTDOWN_TIMEOUT` = 15 s |

Live worker knobs come from the config file, e.g. `config.native.toml`: `source_interval_ms = 1000`,
`request_timeout_seconds = 120`, `cpu_workers = 16`, `row_concurrency = 64`, `[browser] tabs = 2`,
`challenge_wait_seconds = 30`. A Restate-level browser deadline is therefore much longer than the
transport timeout it wraps: the handler re-issues bounded physical acts instead of holding one socket
open.

### 5.5 Flow control

Outbound acquisition runs under the Restate flow-control scope `athletic-source`
(`src/runtime/source.rs:22`) with `SOURCE_CONCURRENCY = 16` (`:23`). `SourceGateway::fetch` refuses a
call whose key is not `"global"` or whose scope is not `SOURCE_SCOPE`, and `SourceCache` calls it with
`.scope(SOURCE_SCOPE)`. Before any deployment, `flow_control::ensure_source_scope` reads the server
version (1.7.3 or newer, with the flow-control features), refuses to continue while
`SourceGateway`/`global`/`blocked` exists in server state ("scope migration or deployment cannot
bypass retained source policy"), then validates or provisions the `athletic-source` rule plus the
stricter wildcard rule (`src/cli/flow_control.rs:13-14`, `:29-47`).

### 5.6 Where the browser pool is invoked

All physical browser acts go through `BrowserSession` — "Restate, not the browser driver, owns
challenge and human-wait policy" (`browser_session.rs:36`):

```text
BrowserSession::{await_ready,status,capture_ready,recover}
  -> browser_readiness::{act, now_ms, physical_status}
    -> Runtime::ensure_browser            (rebuilds a dead manager)
      -> BrowserManager (chromiumoxide actor + page pool, profile gate)
```

`act` wraps the physical call in `ctx.run(...)` with `RunRetryPolicy::new().max_attempts(1)`: the
observation is journaled, but a failed act is not silently re-issued by the SDK — the handler decides.
Shared handlers (`status`, `capture_ready`, `recover`) still act inside `ctx.run` ("Shared handler:
must use ctx.run for physical acts"). Other components reach the browser indirectly:
`SourceGateway::await_admission` waits on readiness (legacy) or takes a one-shot `capture_ready` check
(rankings), and `PipelineControl::rankings_resume` refuses to resume unless `BrowserSession::recover`
reports `Ready`.

## 6. Adding a handler or service

### 6.1 Pipeline worker

1. Put the definition in a module under `src/runtime/` (a submodule directory if it has helpers, as
   `run/`, `row_worker/`, `rankings_collection/` do) and export it from `src/runtime.rs`.
2. Write `pub struct X { pub runtime: Arc<Runtime> }`, `#[restate_sdk::object(...)]` (or
   `#[restate_sdk::service(...)]`) on the `impl` block, and `#[handler]` on each `pub async fn`.
3. Choose attributes deliberately: `lazy_state = true` when invocations touch few keys;
   `ingress_private = true` unless the CLI must reach it; an explicit `invocation_retry_policy` for
   anything that can block on a human; 30-day retentions when the result must stay replayable.
4. Derive the object key from content and **validate it in the handler**: existing objects compare
   `ctx.key()` against a recomputed or fixed key (`RunCoordinator`/`SourceGateway` require `"global"`,
   `BrowserSession` requires `BROWSER_SESSION_KEY`, `LocalReviewer` the lane key) and fail terminally
   on a mismatch. Keys are `identity::fingerprint(...)` (sha256 over the JSON encoding) or
   `identity::scoped_key(scope, value)` = `<scope>:<fingerprint>`.
5. Keep effects inside `ctx.run(...)`, pick `.retry_policy(...)` per effect (`max_attempts(1)` for
   irreversible or observation-only steps, `4` for staged publications), and read/write state with
   `ctx.get`/`ctx.set`; check the result slot first when re-entrant.
6. Bind it in `src/runtime/worker.rs`'s `Endpoint::builder()` chain — an unbound definition is simply
   not served, and nothing else in the tree will tell you. Unit-test the free functions that take
   `&Runtime`/`&Store`; handler bodies are thin by design.

### 6.2 Census endpoint

1. Add the struct, wire types and a free function taking `&Store` in `restate_services.rs`; keep the
   handler thin — "Handler bodies stay thin; the work sits in free functions that take `&Store`, so
   the interesting behaviour is testable without a Restate runtime".
2. Run heavy work as `ctx.run(|| async { blocking(|| …).await … })`: `blocking` puts it on
   `spawn_blocking` and classifies the outcome through `job_error` (`JobError::Transient` →
   retryable, `JobError::Terminal` → terminal; panic and cancel are terminal).
3. Size concurrency with the `Census` semaphore (`self.permit()`), not a new pool; a closed semaphore
   means shutdown and is terminal.
4. Add the binding to `build_endpoint` and update the test's `/discover` expectation (it asserts each
   expected name is advertised). Bound the surface the way the existing constants do — rows per
   request, sweep windows, sweep endpoints — so one invocation's memory and journal entry stay
   predictable.

### 6.3 Rules that apply to both

* **Never rename a struct or handler** without `#[handler(name = "...")]`: the name is the wire contract.
* A new outbound-fetch scope needs a matching flow-control rule (§5.5), or `deploy` will refuse and
  `fetch`'s scope check will reject the call.
* Changing a wire type's meaning means bumping the matching revision constant: `RUN_REVISION`,
  `PREPARE_REVISION` (`run_protocol.rs:9-10`), `ROW_PROTOCOL_REVISION` (`row_protocol.rs:8`),
  `EXPORT_PROTOCOL_REVISION` (`export_worker.rs:25`), `ACQUISITION_REVISION`, `INGESTION_REVISION`.
* Do not add a second convention beside an existing one — another store, queue or retry loop. Restate
  is the only scheduler here: "Row work uses a bounded, replenished durable fan-out; no application
  queue or lease exists" (`run.rs`).

## 7. Durability and idempotency contract

### 7.1 What is guaranteed: at-least-once

* Effects wrapped in `ctx.run` are journaled; a replay reuses the recorded value instead of repeating
  an acknowledged step — but effects whose acknowledgement was lost may repeat: "Unacknowledged effects
  may repeat during crash recovery" (`src/runtime/protocol.rs:93-94`), and "If the SDK loses the
  acknowledgement after this run, its kept stage may orphan" (`export_worker.rs`).
* The census store is append-only: "appending the same entity twice writes two rows, and
  `Store::consolidate` merges them through `Entity::merge`" (`store.rs`). A duplicate observation is
  visible in `total_observations` and in raw scans until the next consolidation; it is not deduplicated
  at append time.

### 7.2 Dedup keys and short-circuits

| Layer | Mechanism |
|---|---|
| Ingress submission | `idempotency_key(...)` on `prepare`, `run`, `run_and_export`, `export`, keys derived from the request fingerprint, retained `30 days` ("how long the result of an idempotent invocation is retained for deduplication" — SDK option doc) |
| Object identity | Content-derived keys: import key, source cache key, row job key, query job key, profile job key, review case key, export key, collection fingerprint |
| Re-entry | Result-slot short-circuits in `RunCoordinator::run`, `RowWorker::process`, `QueryWorker::gather`, `ProfileWorker::gather`, `SourceCache::fetch` (non-rankings), `WorkbookImport::load`, `ExportWorker::publish`, `ReviewCase::review` |
| Binding and paging | `ExportWorker` pins `owner-run` so one destination cannot be published by two runs; `ReviewCase` pins `assignment` and rejects a retained result whose lane contradicts the key; `RankingsCollectionState::step` is fenced by `generation`; `Ingest::complete_window` treats windows as a set; `RunCoordinator::snapshot` refuses to export unless every page digest of the captured progress is present, and that prefix is immutable |

### 7.3 Retention windows

Journal and idempotency retention are 30 days on the pipeline definitions that declare them; the
census definitions declare neither and inherit server defaults — the `Sweep` workflow carries no
retention attribute at all, so a completed sweep follows the SDK/server default. `BrowserSession`
keeps a 26 h inactivity window because a challenge can legitimately be waiting on a person.

### 7.4 Cancellation

* **Run fan-out failure**: `drive` in `run.rs` cancels every outstanding `RowWorker` invocation
  handle and drains the durable futures before returning the error, so a failed run leaves no orphan
  row work.
* **Sweep stop**: `Sweep::interrupt` resolves the `stop` signal on a target invocation; `wait_windows`
  is "cancellation-safe by construction: both select branches are durable Restate futures, so a drop
  mid-select replays the branch from its start instead of losing the wakeup". A stopped sweep still
  observes endpoints, writes its report, and returns `interrupted: true` with the windows it saw.
* **Rankings pause**: `RankingsCollectionState::pause` writes `Paused(Manual)` and stops
  re-scheduling; `resume` bumps the generation and requires a `Ready` browser.
* **Process shutdown**: `serve_with_cancel` stops intake and lets in-flight work finish; the drain
  deadline then aborts and *counts* what is left (`cancelled`/`timed_out`/`aborted`). Cancellation is
  not a CLI verb: a run stops when its source components stop or block (sweep interrupt, rankings
  pause).

### 7.5 What is not implemented: exactly-once

No handler implements an exactly-once effect: nothing carries a write-idempotency token into the
browser, the HTTP fetches, the local model or the workbook writer, and no transaction spans Restate
state and the artifact store. What exists is at-least-once delivery plus §7.2. Two consequences:

1. `Ingest::record` can append the same rows twice if the append commits and the acknowledgement is
   lost; those rows merge at the next `consolidate`, and `total_observations` counts both.
2. `ExportWorker` can leave a staged bundle that no commit receipt refers to; the code marks that as
   an orphan rather than pretending it cannot happen.

## 8. Failure modes

| Failure | What the code does |
|---|---|
| Endpoint not deployed / admin unreachable | Ingress calls fail client-side; `transport::ingress_error` prints Restate's message when a response body exists, otherwise the transport error. No CLI retry loop exists for submission (only `export` polls an already-submitted invocation). |
| Endpoint process dies mid-invocation | The handler task dies with it; the server retries per policy — `1s` initial, 4 attempts, then **paused** for an operator. Journaled `ctx.run` values replay instead of recomputing. |
| Task cancelled mid-flight (drain deadline, operator stop) | Work that committed its `ctx.run` result may not have written its state yet: for `Ingest`, rows can be appended while `cursor`/`total_observations` stay at the previous value, so a producer that resends the batch double-appends (§7.5). For `RunCoordinator`, `progress:<run>` may lag the sealed pages, and `snapshot` refuses to export while a claimed page is missing. |
| Browser challenged / human required / cooling down | `await_ready` never reports `Ready` on a stale observation: it writes `status`, arms `challenge-started-ms`, issues at most one recovery (`recovery-issued`), and ends the wait with `HumanRequired` or a 408 after the deadline. `SourceGateway` returns `BrowserUnavailable` for rankings instead of waiting; `rankings_resume` fails 409 until a recover reports `Ready`. |
| Local model blocked / unusable source row | `LocalReviewer::review` records `blocked` and returns `ReviewOutcome::Failed` with `request: None` on later calls instead of burning retries against a model that is down; `RowWorker::process` publishes a terminal `ReviewRequired` report for a row it cannot use (missing row, validation issue, empty query plan) instead of retrying |
| Panic or cancel inside `blocking` | `JobError::Terminal` with `format!("job panicked: {join}")` / `format!("job cancelled: {join}")`; only an ordinary `Err` becomes `Transient`. `job_error` is deliberately a function, not a `From` impl, so a terminal failure cannot take the SDK's blanket `From<E: StdError>` path and become retryable. |
| Census store held by another process | `Store::open` fails with the reason instead of interleaving writes; `midwest-serve` and batch commands must not share `--data-dir`. |
| Bad or oversized input | Terminal immediately — a retry cannot fix a typo, and the messages enumerate the accepted values — and refused before touching store or journal: rows over `MAX_ROWS_PER_REQUEST`, sweeps over 366 windows / 256 endpoints, runs over 256 concurrency or `MAX_RUN_ROWS`, labels over 128 bytes, admin responses over 64 KiB. |

## Appendix: verification and limits of this document

* Everything above was read from the listed files on 2026-09-21. No build, test, lint or `cargo`
  command was run while writing it, so no claim here is backed by a compile or test result.
* The **census** service set is covered by an executable test that queries `/discover` and asserts
  each name in `["Census", "Ingest", "Sweep"]` is advertised
  (`EXPECTED_SERVICES`, `fjall_restate_e2e.rs:40`); the pipeline worker's 13 bindings are covered only
  by the binding chain in `src/runtime/worker.rs`.
* Exactly-once, cross-store atomicity, and cancellation semantics under a drained endpoint are
  properties of the Restate server version in use, not of this repo; the comments quoted in §7 are the
  codebase's own statement of what it expects from that server.
