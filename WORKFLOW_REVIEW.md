# Function-by-function workflow review packet

## Scope, provenance, and how to read this

This packet describes the **implemented native production path** at source baseline `fee6c82`, not a proposed browser crawler. `src/main.rs` calls `src/cli.rs`; `src/lib.rs` exports the production runtime, domain, parsers, storage, and workbook modules. Historical files elsewhere in the repository are not automatically part of that reachable path.

The user requested both local GPUs for this review. `GPU5090Workflow` uses the configured `qwen36-5090/Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf:high` reviewer; `GPU3090Workflow` uses `qwen36-3090/Qwen3.6-35B-A3B-UD-Q4_K_XL.gguf:high`. Main maps startup/import/coordinator and checks cross-slice claims. These are **static source reviews**, not pipeline matching calls or execution proof. No Golden records or credentials are supplied to the mapping agents. No unit suite, fuzz campaign, build, formatter, or linter is run for this documentation-only mapping.

Call notation:

- **L**: ordinary in-process function call or pure calculation.
- **D**: durable Restate service/object invocation through a generated client.
- **J**: effect enclosed by Restate `ctx.run`; completed result is journaled.
- **B**: `Runtime::blocking`, bounded offload to Tokio's blocking pool.
- **H**: external HTTP effect. HTTP itself is not a transactional participant in Restate.
- **S**: immutable artifact read/write in the worker-owned Fjall database.

File paths are relative to this repository. Line references describe the source baseline and may move after edits. The later sections name exact functions rather than treating similarly named helpers as interchangeable. Consult [HANDOFF.md](HANDOFF.md) for live service addresses, run IDs, frozen binaries, historical failures, raw evidence paths, and unfinished acceptance.

### Reading order

- Sections 1–7: contract, CLI, deployment, import, storage, coordinator.
- Sections 8–10: source admission/HTTP, discovery, profile evidence.
- Sections 11–12: deterministic matching, model review, selection-authorization finding.
- Sections 13–15: performance summaries, export publication, independent verification.
- Sections 16–19: complete call graph, identity/state map, failure boundaries, prioritized review questions.

This is a walkthrough of meaningful production function calls and their important helpers, not a dump of every accessor, generated SDK method, test helper, or historical unused module.

## 1. Product contract and ownership

The input workbook establishes membership, not an eligibility heuristic. The pipeline preserves original source fields, discovers candidate Athletic.net identities, obtains source evidence, conservatively resolves identity, optionally asks one local model about genuine ambiguity, and publishes an auditable XLSX/JSONL bundle.

There is **no junior/class/grade/graduation-year eligibility gate**. Missing school or sport is not automatic exclusion. A workbook address is source context, not proof of a candidate's residence or school location. Missing/incomplete/conflicting evidence must not silently become a verified match.

The important owners are deliberately separate:

| Owner | Responsibility | Not its responsibility |
|---|---|---|
| Native CLI | Validate command inputs, invoke local control API, observe results | Durable workflow scheduling |
| Restate | Durable calls, state, idempotency, timers, queueing, replay | Making external HTTP exactly once or granting source access |
| RunCoordinator | Selection, bounded row fan-out, result accounting | HTTP retry policy |
| SourceGateway + scoped admission | Source pacing, cooldown, bounded HTTP attempts, source block | Candidate matching |
| Query/Profile/Row workers | Cache-bound interpretation, coverage, identity resolution | Inventing missing source evidence |
| Local reviewer lanes | One local model's bounded review of eligible ambiguity | Overriding deterministic contradictions |
| Fjall ArtifactStore | Immutable source rows, bodies, reports, attempt evidence | Application work queue, lease, retry timer, or cross-process live database |
| ExportWorker + offline verifiers | Consistent captured result projection and original-field/evidence validation | Proving the external site's data is true |

Pinned dependencies relevant to review: Restate SDK 0.12.0, reqwest 0.13.4, Fjall 3.1.10, calamine 0.36.1, rust_xlsxwriter 0.99.1, lol_html 2.9.0, quick-xml 0.41.0, zip 8.6.0. Tokio is the async runtime. No Spider, Chromiumoxide, or WebDriver dependency is present in this source baseline.

## 2. Process startup and command dispatch

### `main` → `cli::run`

`src/main.rs:9`:

1. Read `RUST_LOG`; parse it as `EnvFilter`, defaulting to `info` when absent.
2. Invalid/non-Unicode filter input is an error, not ignored.
3. Initialize tracing to **stderr**.
4. **L** `cli::run().await`.

`src/cli.rs:27` parses `args::Cli` with Clap and dispatches:

| Command | Immediate function | Result |
|---|---|---|
| `worker` | `runtime::worker::serve(config, bind)` | Long-lived native SDK HTTP worker |
| `deploy` | `cli::transport::deploy(admin, endpoint)` | Registration JSON |
| `start` | `cli::start(Start)` | Submitted run digest + invocation ID, not completion |
| `status` | `RunCoordinatorIngressClient::status` on key `global` | `Option<RunProgress>`; null can mean not initialized |
| `export` | `ExportWorkerIngressClient::publish` then `await_export` | `PublishedExport` |
| `verify` | `existing_stopped_store` → `bundle_verify::verify_bundle` | Independent bundle verification report |

`cli::emit` serializes final JSON to stdout and adds a newline. Diagnostic tracing belongs on stderr.

### `worker::serve` → `Runtime::open`

`src/runtime/worker.rs:12`:

1. Reject a non-loopback bind address.
2. Install SIGTERM and SIGINT handlers.
3. Bind a TCP listener.
4. **L** `Runtime::open(config)`.
5. Bind eleven SDK implementations into one `Endpoint`: `PipelineControl`, `WorkbookImport`, `RunCoordinator`, `ExportWorker`, `RowWorker`, `QueryWorker`, `ProfileWorker`, `ReviewCase`, `LocalReviewer`, `SourceGateway`, `SourceCache`.
6. `HttpServer::serve_with_cancel(listener, shutdown(...))` serves Restate invocation traffic.
7. On shutdown, `Runtime::drain()` closes new CPU admission and awaits tracked blocking work.

`src/runtime.rs:43`, `WorkerConfig::load` and `WorkerConfig::validate` in `src/runtime/config.rs`:

- Read at most 64 KiB configuration, decode UTF-8/TOML, deny unknown fields.
- Validate CPU workers 1–32; configured row concurrency 1–128; request timeout 1–300 seconds; source interval at most one hour; absolute storage path.
- Endpoint origins must have no path beyond `/`, query, fragment, or embedded credentials.
- Live source is exactly `https://www.athletic.net/`; fixture source must be a loopback IP literal.
- Both model endpoints must be loopback IP literals. Model identifiers are nonempty, bounded to 256 bytes, and control-character-free.
- Load and validate optional private source session **before opening storage**.
- **S** `ArtifactStore::open`.
- Construct shared reqwest client: no built-in retries, decompression, proxy, or redirect following; configured timeout; authorized-research User-Agent unless source-specific session headers override it for a source request.
- Create CPU semaphore and `TaskTracker`.

`Runtime::blocking` (`src/runtime.rs:68`) acquires a tracked token, rejects draining state, obtains a CPU permit, then invokes `spawn_blocking`. The closure owns its token and permit until it finishes. Join errors are surfaced with context. `Runtime::drain` closes the semaphore/tracker and awaits tracked work; it is not itself a guarantee that all durable Restate invocations are completed or cancelled.

## 3. Deployment and global source-concurrency guard

`cli::transport::deploy` (`src/cli/transport.rs:43`):

1. `local_origin` validates admin/worker endpoints are loopback HTTP(S) origins.
2. Build a local control client: no proxy, retries, or redirects; 300-second request timeout.
3. **H, local admin** `flow_control::ensure_source_scope`.
4. **H, local admin** POST `/deployments` with worker URI.
5. Bound response at 64 KiB and decode JSON.

`ensure_source_scope` (`src/cli/flow_control.rs:35`) calls:

- `get_version` → `verify_server_capabilities`: require a supported non-prerelease server version >= 1.7.3 and advertised enabled `vqueues`, `protocol_v7`, `scoped_virtual_objects`.
- Query `SourceGateway/global` state for `blocked`; refuse deployment/scope migration if a block exists.
- `source_rules` → `validate_or_provision_source_rule`.
- Existing exact `athletic-source` rule must be enabled and between 1 and 16 concurrent operations. Unlimited/zero/over-ceiling is rejected.
- `reject_stricter_wildcard` prevents an exact rule from silently bypassing a stricter wildcard.
- If absent, provision an exact rule at min(16, enabled wildcard limit), using a does-not-exist precondition, then re-read and validate it.

This is Restate flow-control admission, not an in-process semaphore and not a requests-per-second guarantee. Deployment cannot be used to clear source policy.

## 4. Starting a run and constructing identities

`cli::start` (`src/cli.rs:141`):

1. Require exactly one selection mode: `Selection::PerSheet { rows }` or `Selection::All`.
2. Canonicalize the input path and parse the supplied SHA-256 as `WorkbookDigest`.
3. Build `ImportRequest { original, workbook }` and `PrepareRequest { source, selection, concurrency, snapshot_label, execution }`.
4. **L** `run_protocol::preparation_key` hashes `(PREPARE_REVISION, ACQUISITION_REVISION, request)`.
5. **D** `PipelineControl.prepare`, with that ingress idempotency key; wait for a prepared `RunRequest`.
6. **L** `RunRequest::key` validates execution label and protocol concurrency ceiling, then hashes `(RUN_REVISION, SOURCE_PARSER_REVISION, request)`.
7. **D** `RunCoordinator/global.run`, with run key as ingress idempotency key, using send/submission semantics.
8. Emit run key and actual invocation ID with `state: submitted`.

`identity::fingerprint` (`src/runtime/identity.rs:7`) streams JSON serialization into SHA-256. `scoped_key` formats `<scope digest>:<fingerprint>`. Domain digest types validate representation; they do not by themselves prove an artifact is present. Artifact reads independently verify bytes against the digest.

### `PipelineControl::prepare`

`src/runtime/control.rs:33`:

- Validate requested concurrency against worker capacity and 256 protocol ceiling; labels are 1–128 bytes without controls.
- **L** `import_worker::key(source)`.
- **D** `WorkbookImport/<import-key>.load(source)` → manifest digest.
- **J** `prepare source snapshot and run`, max one run-action attempt:
  - Construct `SourceSnapshot { revision: ACQUISITION_REVISION, source_origin, label }`.
  - **S/B** `Runtime::store_json(snapshot)`.
  - Return `RunRequest { manifest, snapshot, selection, concurrency, execution }`.

A source snapshot is a contract/origin/label identity, **not a frozen copy of the entire remote website**. Source receipts are acquired under that identity. Changing labels changes reuse boundaries; it does not override access policy.

## 5. Immutable workbook import

### `WorkbookImport::load`

`src/runtime/import_worker.rs:34` is an ingress-private exclusive virtual-object handler. Object key = fingerprint of `(INGESTION_REVISION, ImportRequest)`; the original path must be absolute.

1. Validate object key.
2. Read state `manifest`.
3. Cache hit: **J** `verify unchanged source for cached import`:
   - **S/B** load retained `SourceManifest`.
   - Verify ingestion revision, workbook digest, and original path.
   - **B** `snapshot::verify(original, expected_digest)` rereads and hashes the original.
   - Return manifest digest only if it still binds unchanged input.
4. Cache miss: **J** `import immutable workbook once`:
   - `import::import(runtime, request)`.
   - **S/B** store the resulting manifest JSON.
5. Set state `manifest` and return its digest.

### `import::import` → `snapshot::freeze` → `import_rows`

`src/runtime/import.rs:31` offloads one bounded blocking action:

1. `snapshot::freeze(original, storage_dir, expected_digest)`.
2. `import_rows(store, workbook_digest, frozen_path)`.
3. Re-verify original path/digest after ingestion.
4. Return `SourceManifest { original, frozen, workbook, ingestion_revision, stats }`.

`snapshot::freeze` (`src/runtime/snapshot.rs:13`) creates private `source-workbooks/`, chooses `<sha256>.xlsx`, and verifies an existing target before reusing it. For a new target, `copy_verified` hashes while copying a regular <=1 GiB file into a temporary file, verifies digest/length, fsyncs, and publishes with `persist_noclobber`. A concurrent already-existing target is verified rather than overwritten. `snapshot::verify` repeats regular-file/size/hash checks.

`import_rows` builds `ImportBatch`, calls `workbook_ingest::visit_records`, flushes, and rejects an empty source workbook. `ImportBatch::push` counts source field key/value bytes and flushes before the next record would exceed 512 records or the 8 MiB batching target. `flush` delegates to `ArtifactStore::put_source_batch`, whose own serialized-size bound remains authoritative.

### `workbook_ingest::visit_records` → `stream::visit_sheet`

`src/workbook_ingest.rs:22`:

- Preflight the XLSX archive/XML before calamine ingestion.
- Load OOXML sheet-path metadata to connect dimensions and XML row counts.
- Open calamine workbook and enumerate workbook-order sheet names.
- Exclude generated sheets named `Athletic Matches`, `Corrections`, `Summary` (case-insensitive).
- For each source sheet, **L** `stream::visit_sheet`, accumulating actual observed rows and sheet metadata.

`src/workbook_ingest/stream.rs:14` uses calamine's cell reader rather than materializing every worksheet row simultaneously:

- `SheetState::accept` validates Excel row/column bounds and strict cell ordering; detects duplicate/out-of-order positions.
- On a row transition, `finish_row` emits the previous row.
- Materialize cell values with row-byte accounting.
- The first nonempty row becomes headers through `build_headers`; headers cannot be empty or duplicated.
- Retained headers are bounded at 1 MiB across the workbook. Materialized row keys + values are bounded at 8 MiB.
- Empty rows are not source records; nonempty data in an unnamed column is rejected.
- Each data row emits `SourceRecord { source_key: "<sheet>:<physical-row>", sheet, excel_row, fields }` using physical one-based Excel row numbers.
- `SheetState::finish` flushes the last row and returns observed `SheetStats`.

Preservation here is a typed/textual source-cell contract, not a promise to retain every original workbook style, macro, or byte layout. Independent export verification checks the declared original-field contract.

## 6. Artifact storage and journal boundary

`ArtifactStore` (`src/store.rs`) wraps one shared `StoreInner`. `StoreInner::open` (`src/store/backend.rs:29`) opens Fjall with a 32 MiB database cache and manual journal persistence, plus keyspaces:

- `documents`: digest-addressed bodies and JSON artifacts.
- `source_index`: `(workbook, source row)` to source-record digest.
- `http_attempt_evidence`: operation/attempt index to immutable attempt artifact.

`put_bytes` validates <=32 MiB, computes digest, verifies any existing document at that key, otherwise atomically persists a new document with `PersistMode::SyncAll`. `get_bytes` bounds stored size, loads, and verifies the digest before returning bytes.

`put_source_batch` validates row identity, serialized document size, maximum 4,096 records and 32 MiB serialized bytes. `plan_record`/`plan_index` build an atomic `BatchPlan`; a source index pointing to different content is `SourceConflict`, not last-write-wins. `BatchPlan::commit` atomically writes documents and indexes with synchronous persistence. `source_record` and `visit_source` verify index, digest, sheet, and physical row consistency.

`Runtime::load_json` (`runtime/artifacts.rs:9`) performs verified immutable reads through `Runtime::blocking`; these reads may repeat during replay. `Runtime::store_json` uses bounded streaming JSON encoding and storage, and its callers should enclose derived publication in a journaled `ctx.run`. Journals carry references instead of entire workbook/source bodies.

`record_attempt` (`src/store/audit.rs:30`) allocates a fresh UUIDv7 attempt identity, serializes `{ operation, attempt_id, value }`, and atomically stores document + operation index. Identical response values in separate attempts remain separate evidence. `read_attempt` verifies digest, operation, and index binding. This store is evidence, **not** retry ownership or proof of every remote request start.

## 7. Coordinator: selection, fan-out, progress, and replay

### `RunCoordinator::run`

`src/runtime/run.rs:46`, exclusive object key `global`:

1. Reject any other admission key.
2. Derive `RunRequest::key`.
3. Return state `result:<run-key>` when already finished.
4. `validate_snapshot`: requested concurrency <= worker capacity; retained snapshot acquisition revision/origin match this worker; label is valid.
5. **S/B** load source manifest and require current ingestion revision.
6. **L** `SourceRows::new(manifest, request)`.
7. `Results::new(ctx, runtime, RunIdentity, selected)`.
8. `drive(ctx, runtime, request, rows, results)`.
9. `Results::finish` → summary digest.

The exclusive `global` run handler serializes run admissions. Shared `status`, `page`, and `snapshot` handlers remain available while work is running. Object/service decorators generally retain journals and idempotency 30 days and pause invocation retries after four attempts; those infrastructure attempts are **not** the source HTTP retry counter.

### `SourceRows::new` → `SourceRows::next` → `refill`

`src/runtime/run/source_rows.rs`:

- Validate 1–256 uniquely named sheets, nonzero consistent row accounting, and total <=2,097,152.
- `Selection::count` selects all rows or `min(available, requested_per_sheet)`.
- Build a per-sheet cursor with issued count, physical-row cursor, and small pending deque.
- `next` rotates sheets round-robin; this is not "finish sheet one before sheet two".
- `refill` **B/S** calls `source_key_page` for at most 64 immutable row keys.
- Return `RowJob { workbook, snapshot, source }` until the selected count is exhausted.

These cursors are **ephemeral deterministic replay state**, not an application checkpoint or independent persistent queue.

### `drive`: durable replenished fan-out

`src/runtime/run.rs:197`:

1. Create `DurableFuturesUnordered` and a map of admitted jobs/child invocation handles.
2. Refill while fewer than requested row concurrency are in flight.
3. For each job, derive `RowJob::key` and **D** call `RowWorker/<key>.process(job)`.
4. Obtain and retain the child invocation handle before recording its future index.
5. Await the next durable completion, recover its job, and call `Results::record(job, report_digest)`.
6. Replenish the freed slot; no need to wait for a whole batch.
7. On errors, cancel retained child handles and attempt to drain outstanding durable futures before returning the error.
8. Successful exit requires no unaccounted child jobs.

Review cancellation at SDK boundaries, not just Rust future destruction. A cancelled/dropped outer future is not sufficient evidence that remote child effects never occurred.

### `Results` owns accounting and sealed result pages

`src/runtime/run/results.rs`:

- `new`: `journal_time` observes clock inside **J** `observe run time`; initialize coverage and state `progress:<run-key>` with no terminal rows.
- `record`: **S/B** load `RowReport`, check row protocol revision and exact job key, then `Coverage::record` increments one outcome category and `completed` with overflow/selected-bound checks.
- Append `RowReference { source, report }` to `pending_rows`.
- At 64 references, `flush`: **J/S** publish immutable `RunPage`, set `page:<run-key>:<page-number>`, append page digest, increment page count.
- Refresh journaled time and publish current progress after each row.
- `finish`: require completed == selected; flush tail; publish `RunSummary`; set `complete = true`, summary digest, final progress, and `result:<run-key>`.

Coverage categories are deterministic acceptance, local-review acceptance, complete-search no-match, and review-required. `completed` means a row reached one terminal resolution; it does not mean all rows matched.

### Shared observation/export snapshot handlers

- `status` reads `progress:<run-key>` and returns optional `RunProgress`.
- `page` verifies requested page is within current sealed-page count and returns an optional page digest.
- `snapshot` captures a progress value, validates its run identity/page bound, and reads its already-sealed immutable page prefix. Pending rows belong to that captured progress. Later work can append pages without mutating the captured prefix.
- An export snapshot can intentionally represent an incomplete run. Preserve that distinction in output and acceptance reporting.

## 8. Source transport: exact call chain and three scopes

Every source request produced by a QueryWorker or ProfileWorker reaches:

```text
SourceRequest::key(snapshot, acquisition revision, resource)
  D SourceCache/<key>.fetch
    read state "result" → return cached FetchOutcome, including failures
    D SourceGateway/global.fetch in scope "athletic-source"
      L request::build
      L execute
        L http_audit::operation_key(invocation_id, "source-http")
        repeat at most four retained workflow attempts:
          D admission::acquire → SourceGateway.await_admission
            scope "athletic-source-admission", exclusive sleeping head
            D SourceGateway.admit
              scope "athletic-source-control", exclusive quick state operation
          J run_step (max one action attempt)
            H http::perform → send → reqwest GET/POST
            L body::read_body → challenge classification
            B/S receipt → store raw body
            B/S http_audit::record → record_attempt
            B/S http_audit::load → attempt_digests/read_attempt
            L result::finish_workflow → Finalized
          D publish_feedback → SourceGateway.observe (control scope)
          durable ctx.sleep before retry
        D publish_final_feedback
    set state "result"
```

### Cache identity versus evidence retention

`SourceRequest::key` (`runtime/source_cache.rs:17`) includes snapshot, acquisition revision, and `SourceResource`, but not parser revision. Query/Profile caches add parser revision. Consequently, a parser-only revision can reparse compatible retained source receipts without asserting they were newly fetched.

`SourceCache::fetch` is an exclusive object handler: same-key callers reuse one retained `FetchOutcome`. Its `result` is Restate object state. Raw bodies and audit documents live in Fjall. These are different ownership roles, **not two independent retry caches**. Failures are cached too, which is why an erroneous terminal acquisition result can contaminate that snapshot's later consumers.

### Exact resource → HTTP mapping

`request::build` (`runtime/source/request.rs:26`) converts a typed resource into `RequestSpec { url, semantic_url, body }`:

| Resource | Method and path | Input |
|---|---|---|
| `Search` | POST `/Search.aspx/runSearch` | JSON `{ q, fq: "t:a a:<sport>", start }` |
| `Bio` | GET `/api/v1/AthleteBio/GetAthleteBioData` | `athleteId`, `sport`, `level=0` |
| `ProfileHtml` | GET validated profile path on configured origin | Validated `ProfileUrl` |
| `Team` | GET `/api/v1/TeamNav/Team` | `team`, `sport`, `season` |

Search query text must be nonempty, <=512 bytes, and contain no controls; `start` <=1,000,000. Team ID and season must be nonzero. `endpoint` prevents origin escape and rejects unsafe paths; `safe` rejects embedded authority credentials and fragments. Search identity lives in the JSON body, not the endpoint URL alone; audits retain the request body.

### Three separate scope responsibilities

1. **`athletic-source`**: shared `fetch` calls; deployment concurrency ceiling 16. A fetch can hold this slot while waiting on admission or its retry cooldown. Thus scope usage 16 does **not** mean sixteen sockets currently transferring bytes.
2. **`athletic-source-admission`**: exclusive `await_admission`; one durable waiting head. This serializes eligibility to approach the quick control object, preventing pacing starvation.
3. **`athletic-source-control`**: exclusive `admit` and `observe`; reads/writes policy quickly and does not sleep, so already-admitted operations can report cooldown/block while the admission head waits.

All three require object key `global` and their expected scope. Distinct scopes deliberately separate execution/state ownership. Inspect the control scope for policy state.

`admission::admit` (`runtime/source/admission.rs:21`):

- State `blocked` present → `AdmissionDecision::Blocked` with retained failure.
- State `not-before-ms` in future → `Deferred { wait_ms }`.
- Expired deadline → clear it.
- If configured interval is nonzero, store journal-observed current time + interval.
- Return `Granted`.

`admission::observe`:

- Preserve the first `blocked` failure instead of replacing it.
- For nonzero cooldown, store `max(existing_deadline, journaled_now + cooldown_ms)`.
- Return only after issuing the state update; source `publish_feedback` uses `.send().await` and then `.attach::<()>().await` to await handler completion.

The deadline is an absolute Unix-time value observed inside a journaled action, **not a monotonic clock**. It is not shortened by cooldown feedback, but it can be cleared after expiry. Clock adjustment behavior is a review concern; do not describe this as a globally monotonically increasing deadline.

`admission::wait` calls quick `admit`, durably sleeps for a deferral, then rechecks. Its bound is 64 observations; exceeding that returns a retryable infrastructure error rather than a cached synthetic `RateLimited/NotAttempted` result. With the source ceiling, at most fifteen other admitted peers can extend the head's wait before their next attempts must queue behind it. That is the concurrency assumption behind the guard.

`admission::acquire` retains the admission child's invocation handle. On SDK code 409, it cancels that child and propagates cancellation. **409 is cancellation, not a source concurrency rejection.**

### Actual network attempt and body handling

`http::perform` (`runtime/source/http.rs:40`) records start time, calls `send`, captures status/media type/parsed Retry-After/header challenge flag, then `body::read_body`.

- `send` chooses POST JSON when a body exists, GET otherwise.
- Optional `SourceSession::authorize` checks exact scheme/host/effective port and expiry immediately before request construction is sent.
- `SourceSession::attach` adds sensitive Cookie/User-Agent only to that source request. They are not shared client defaults and are not fields of captured `RequestSpec`.
- No browser process, JavaScript interpreter, cookie renewal, CAPTCHA solver, or DOM interaction occurs.
- `body::read_body` checks declared size, streams with checked allocation/counting up to 32 MiB, and rejects Content-Length mismatch or transport failure. Oversized/partial bodies are not passed off as complete retained documents.
- `receipt` publishes complete raw bytes to Fjall and returns `DocumentReceipt { digest, source_url, http_status, media_type, bytes, fetched_at_unix_ms, elapsed_ms }`.
- Complete raw response headers are **not** stored in this receipt. Header-derived classifications and delay are retained in attempt results, but a missing historical `cf-ray` header cannot be reconstructed by assertion.

### Cloudflare detection and response classification

`http/challenge.rs`:

- `cf_header_challenge` recognizes `cf-mitigated: challenge` case-insensitively.
- `html_body_challenge` only inspects HTML/XHTML and at most the first 128 KiB.
- Requires combinations of title, platform path, form/error marker, JavaScript/cookie instruction, or `window._cf_chl_opt`.
- A passive `/cdn-cgi/challenge-platform/` script alone does not classify a normal page as denied.

`http::outcome`:

- Challenge + non-429 status → `AccessDenied`, even when status is 200.
- Unchallenged successful status → successful attempt.
- Complete challenged 429 → failed `RateLimited` attempt, eligible for the same bounded cooldown rules as other valid 429s; never parser success.
- 401/403 → access-denied code; 429 → rate-limited code; other unsuccessful statuses → HTTP-failure code.
- Retryable status class is 429 or 5xx, subject to valid bounded delay and earlier challenge/body handling.
- Session expiry/origin mismatch → AccessDenied without sending HTTP.
- Body transport failure may be retryable for successful/retryable statuses with valid delay; payload-limit and artifact failures remain fail-closed.

### HTTP retry versus infrastructure retry

`source::execute` owns at most four source attempt iterations. Before **every** attempt it re-enters admission. Each `run_step` encloses one HTTP/audit/finalization action with `max_attempts(1)`; hidden reqwest retry is disabled.

`retry::retry_after` accepts one unsigned-seconds or HTTP-date header, trimming horizontal surrounding whitespace. Duplicate, malformed, overflowed, or >24-hour delays are errors; past HTTP dates become zero. `next_delay` uses a positive accepted header delay directly; otherwise:

- HTTP 429: max(configured interval, 60/120/240 seconds).
- Other transient retry: max(configured interval, 1/2/4 seconds).

Before sleeping, `publish_feedback` durably extends the shared cooldown. The caller sleeps with Restate `ctx.sleep`, not a process-local retry timer. If admission subsequently blocks, `execute` returns its own prior finalized evidence when available, otherwise the admission block failure.

`run_step` records `CapturedAttempt { request, result }`, reloads retained attempts for the operation, and uses `finish_workflow` to identify the exact returned attempt digest. Nonselected complete responses remain `previous_responses`/failure evidence.

`result::finish_with_retries` blocks source admission on access denial, 401/403, exhausted retryable 429, artifact failure, and nonretryable zero-delay 429/503 cases. Exhausted non-429 transient failures still produce explicit terminal failed acquisition without necessarily globally blocking all source work. A successful final attempt produces no new cooldown; existing shared policy is not forcibly cleared.

Audit publication/retrieval uses internal terminal code 507. In the active caller, a non-cancellation `run_step` failure is converted by `artifact_finalized` into blocked `ArtifactFailure`, with unavailable workflow evidence rather than a fabricated zero-attempt claim. The result module also has generic effect-error branches; distinguish those branches from the actual current call path, which invokes normal finalization with a selected retained digest.

### Retry evidence is not exactly-once HTTP

`http_audit::operation_key` hashes invocation ID, a zero separator, and effect label. Source evidence is `WorkflowControlled`; model SDK retry evidence is a distinct `SdkControlled` variant. Both report maximum retries three, completed observed attempt count, and immutable attempt digests when available.

A crash after an external response but before acknowledged audit/journal publication can repeat the request on recovery. A four-iteration policy is not a proof of at most four physical network starts across uncertain crashes. `WorkflowEvidenceUnavailable`, `SdkEvidenceUnavailable`, `UncertainEffect`, and `NotAttempted` must not be conflated.

### Source-session security boundary

`SourceSession::load` (`runtime/source_session.rs:61`) requires an absolute regular non-symlink final path; Unix group/other permission bits must be zero; file <=16 KiB; strict JSON fields `origin`, `user_agent`, `cookie_header`, `expires_at_unix_ms`. Header values are nonempty, <=8 KiB, control-free, valid `HeaderValue`, and marked sensitive. Debug output redacts both header values.

Session values are loaded once, with startup freshness and per-use expiry checks. A clock failure fails expiry closed. No hot reload/renewal exists. The server can invalidate clearance before nominal local expiry. File metadata/open ordering and ancestor-path trust are legitimate security-review questions; do not overstate the final-component check as a complete hostile-filesystem proof.

## 9. Row workflow and exhaustive bounded discovery

`RowWorker::process` (`runtime/row_worker.rs:41`) is an ingress-private exclusive object keyed by `RowJob::key`. It validates the job, returns state `result` if cached, then:

1. **B/S** `load_source` calls `store.source_record(workbook, source)` and checks stored physical source identity.
2. Missing source, failed source validation, invalid query plan, or empty query plan → `support::publish_terminal`, retaining a review-required row rather than silently dropping it.
3. **L** `search::query_plan(source)`.
4. `execute_row` → `discovery::execute_queries`.
5. **J/S** `support::publish` stores `DiscoverySummary` with job, candidate IDs, query artifacts, completeness, and issues.
6. `discovery::execute_profiles` obtains explicit coverage for every bounded discovered ID.
7. Search is `Complete` only if both discovery and candidate-profile coverage are complete; otherwise construct `SearchCompleteness::Incomplete`.
8. **B** `assess_and_publish` → `decision::assess`; **J/S** publish assessment.
9. `review::resolve_assessment` chooses deterministic outcome or the narrow eligible model-review path.
10. Merge discovery/profile/limit issues; **J/S** `publish_report(RowReport)`; cache `result`.

### Query planning: `query_plan` → `source_name` → `query_variants` → `build_queries`

`src/search/plan.rs`:

- Read first/last name from canonical source headers, with `first_name`/`last_name` aliases in this planner.
- Stage 0: full name.
- Stage 1: full name + school when available; full name + combined city/region context, or whichever of city/region exists.
- Stage 2: normalized full name, reversed name, first initial + last name.
- For each variant, build both TF and XC queries, compact whitespace, validate <=512 bytes/control-free, and deduplicate by sport filter + ASCII-lowercased query text.
- Normalization uses NFKD, removes combining marks, and **replaces punctuation with spaces**, not concatenation.
- A `stage` label is planning metadata. `execute_queries` iterates the whole plan; it does not stop after a promising stage-0 candidate.

Only permitted name/school/city/region context enters source search requests. Email, street, postal, and other full-record fields are not added to these queries.

### `execute_queries` → `QueryWorker::gather`

`runtime/row_worker/discovery.rs:90` processes planned queries sequentially within one row, while different row workers run concurrently:

- Build `QueryJob`, derive snapshot/acquisition/parser/query-identity key.
- **D** `QueryWorker/<key>.gather`; retain child invocation handle.
- 409 → cancel child and propagate; other terminal call errors → retain issue and continue other planned queries.
- **S/B** load `QueryEvidence`, append query issues/failures, and `add_candidates` from every retained parsed page.
- Candidate IDs deduplicate in a `BTreeSet`, up to 4,096 unique IDs per row. Overflow sets `candidate_limit`, so partial coverage cannot become complete.
- `DiscoveryState::complete` requires no incompleteness flag, candidate overflow, or issues.

`QueryWorker::gather` (`runtime/query_worker.rs:30`) validates key, reuses `result`, otherwise `collect`, **J/S** publishes `query-evidence`, and sets `result`.

`collect`:

1. `SearchProgress::new(query)`.
2. While `next_offset()` exists, build `SourceResource::Search` and `SourceRequest`.
3. **D** local helper `fetch` → `SourceCache::fetch`.
4. `reconcile_page` handles the returned outcome:
   - Failed source acquisition: retain failure, stop this pagination.
   - Retrieved: **S/B** `load_response`; **B** `parse_response` → `search::parse_page`.
   - Distinguish artifact-load/offload failure from parser failure.
   - **J/S** publish parsed search page.
   - `SearchProgress::consume` validates/reconciles it.
   - Retain `QueryPage { response, parsed, retries, previous_responses }` **even if reconciliation fails**, so discovered rows do not disappear from evidence.
5. Query complete only if no failures and `SearchProgress::complete`.

### Search parser and pagination reconciliation

`search::parse_page` in `src/search/parser.rs` parses the source JSON envelope and embedded HTML results/pager through `src/search/parser/markup.rs` and `src/search/parser/stream.rs`. Search display names are discovery hints, **not sufficient name-exclusion proof**.

`SearchProgress::consume` (`search/progress.rs:37`) → `validate_page` → `record_page` → `add_candidates`/`reconcile_page`:

- Query cache identity and expected start must match.
- Repeated starts, duplicate athlete identities, inconsistent candidate counts, changed advertised count, and over-counting are rejected.
- Bounds: 1,000 pages and 100,000 advertised results per query.
- Zero advertised count is complete only with zero rows, no next page, and no issues.
- Reaching the exact advertised count is complete only without another page or issues.
- Before reaching count, a next offset and nonempty progress are required; malformed/unrelated result rows prevent completeness.

A 200 response is not automatically a valid/complete search. An empty/malformed/challenged response cannot silently become no-match.

## 10. Candidate probe, source-bound exclusion, and full profile acquisition

### `execute_profiles` per-candidate call chain

`runtime/row_worker/discovery.rs:178` iterates deduplicated athlete IDs in sorted order:

```text
L ProfileJob::key(snapshot, acquisition revision, parser revision, athlete ID)
D identify helper → ProfileWorker.identify
S/B load ProfileProbe within cumulative row artifact budget
L exclusion(source canonical name, probe)
  proven different name → record NameExcluded, skip TeamNav expansion
  not proven different → D gather helper → ProfileWorker.gather
S/B add_full_profile / load_artifact
  complete + no failures + profile present → Complete
  otherwise → Incomplete with retained refs/issues
```

An 8 MiB cumulative **serialized probe/full-artifact loading budget per row** bounds this layer. It is not an 8 MiB whole-process RSS guarantee and is not the source HTTP body limit. Budget excess keeps candidate identity and available references as `Incomplete`; it does not silently accept the acquired prefix. The raw artifact is loaded before this deserialization-budget decision, so allocation bounds at storage and row layers are distinct.

`identify`/`gather` helpers retain child handles and cancel on 409. Ordinary failures become explicit incomplete candidate coverage.

### Probe reuse: `ProfileWorker::identify` → `identify_probe` → `initial_phase`

`runtime/profile_worker.rs:72`:

- Validate `ProfileJob` object key.
- State `probe` hit returns a source-independent retained `ProfileProbe`.
- Miss: `initial_resources` constructs three requests: TF Bio, XC Bio, canonical TF/all profile HTML.
- `acquire` calls `fetch_one` → **D** `SourceCache.fetch` **sequentially** for those resources.
- `parse_sources` buffers up to eight parsing futures, each using bounded CPU offload. The constant named `FETCH_CONCURRENCY` here is used by parsing; it does not make `acquire` an eight-way HTTP fan-out.
- `parse_one`/`parse_document` load complete source bodies and dispatch:
  - Bio JSON → `profile::parse_bio_value` and `team::authorized_requests_value`.
  - HTML → `profile::parse_profile_html`.
- `absorb_initial` preserves receipts, failures, operations, identities, profiles, authorized follow-ups, and issues. Missing validated Bio identity makes the state incomplete.
- `profile_probe` packages it; **J/S** `publish_probe` stores `profile-identity-probe-publication` and sets `probe`.

### Name exclusion is not search-name comparison

`exclusion` (`discovery.rs:277`) requires a source canonical name, no probe failures, exactly two raw Bio identity observations, **and HTML present**. It calls `NameExclusion::new` with TF/XC identity evidence and scoped HTML identity.

The exclusion must bind to the requested athlete and the particular source-name context. Missing HTML, missing/empty raw name components, wrong IDs, or conflicting Bio/HTML identities cannot establish exclusion. `NameExcluded` retains probe digest and canonical source name; it is not a global athlete-negative cache and must not be reused for a different source name.

The constructor and its canonical-name checks are in `src/domain/name.rs` and its submodules. The independent verifier rechecks raw witnesses; a search display label alone cannot authorize skipping full work.

### Full acquisition: `ProfileWorker::gather` → `build` → `team_phase`

- `gather` returns cached `result` when present.
- `build` calls `identify_probe` again, which reuses the original probe; load via `state_from_probe`. Initial HTTP work and failures are not reset.
- `team::unique_requests` deduplicates `(team_id, sport, season)` and caps at 4,096; overflow records an issue/incomplete flag.
- Authorized requests originate from bounded `allSeasons` records joined to `allTeams` IDs in the Bio response. No guessed TeamNav IDs/seasons.
- `acquire` fetches TeamNav resources sequentially through SourceCache.
- `parse_teams` buffers up to eight parser futures; `parse_team` **B/S** invokes `team::parse_team_nav`.
- TeamNav requires JSON `team`, exact requested `team.ID`, nonempty valid name, valid optional location and level.
- `finalize_profile` → `assembly::assemble`.
- **J/S** `publish` stores `profile-acquisition-publication`, sets state `result`, and returns a `ProfileAcquisition` digest.

`ProfileAcquisition::complete` describes the requested acquisition contract, **not proof of the upstream athlete's entire career corpus**.

### Bio parsing and evidence construction

`profile::parse_bio_value` → `profile/bio.rs::parse_bio_value`:

1. `parse_identity`: require requested `IDAthlete`, parse raw first/last separately, retain field locators, and record incomplete-component issues. Do not derive one complete raw component from a concatenated display name.
2. `parse_teams`/`parse_team` and `parse_grades`: retain team/season/grade observations and source references.
3. `parse_results` chooses only requested `resultsTF` or `resultsXC`; missing, null, invalid-shape and empty data are distinct.
4. TF → `events::index`; XC → `index_distances`. Unrelated sport metadata is not required to validate the requested sport.
5. Build bounded meet/team/relay lookup context.
6. `parse_result` joins result, season, school, meet, event/distance metadata; preserves raw mark, units, timing, wind, date, source IDs, best flags, and evidence locator.
7. `attribution` yields `Individual` only for the requested personal athlete, `VerifiedRelayMember` only with membership evidence, otherwise `Unresolved`.
8. `best_claim` retains TF numeric flags as `OpaqueFlags`; XC boolean flags as claimed/not-claimed. Opaque numbers are not proof of a PR.
9. `availability` records observed sport availability; build `ProfileEvidence` and optional complete `BioIdentityObservation`.

Relevant limits include 32 MiB Bio input, 100,000 bounded items per relevant collection, and 8 MiB materialized joined-result evidence. Truncation retains issues and raw-document references, not a false completeness claim. XC display distance text/units are distinct from canonical `Meters` metadata.

### HTML and profile merge

`profile::parse_profile_html` (`profile/html.rs:36`):

- Bound to 32 MiB and valid UTF-8.
- `html::stream::parse` uses bounded lol_html handlers to capture canonical URLs, scoped text, and supported embedded state.
- `canonical_url` validates every retained canonical/og identity against requested athlete.
- `html::state::parse` parses supported embedded JSON state into scoped tree/identity hints; it does **not** execute JavaScript.
- `cohort_witnesses` parses explicitly scoped descriptive graduation text with locators. This does not introduce eligibility filtering.

`assembly::assemble`:

- `profile::merge_profiles` rejects cross-athlete merge; merges team, grade, graduation, sport and result observations while recording contradictions rather than overwriting them.
- `apply_html` joins HTML identity/URL/document, retains descriptive cohort witnesses, and marks non-descriptive issues or identity contradictions incomplete.
- `attach_team` requires an existing matching Bio team ID, compares name and overlapping location facts, retains source-specific observations, and marks contradictions incomplete.
- Compatible CityOnly/RegionOnly observations can corroborate the **same team ID** without fabricating a new combined-location witness. Different team IDs cannot be spliced into corroboration.

## 11. Deterministic assessment and the actual automatic-match gate

`assess_and_publish` → **B** `decision::assess` (`domain/decision.rs:160`):

1. Bound candidate entries (4,096), profile fragments (twice that), identity strings (4,096 bytes), and search reasons (64).
2. Validate each profile's athlete ID and any exclusion's canonical source-name binding.
3. `group_evidence` groups by athlete ID; mixed coverage for one ID becomes incomplete.
4. `matching::source_identity` extracts canonical first/last, meaningful normalized school, optional mailing city/region.
5. `matching::candidate_reason` → `candidate_facts`, evidence reasons, evidence digests, and a descriptive strength score.
6. `choose_decision` derives final `Assessment`, including private `VerifiedMatch` only for a permitted deterministic selection.

`candidate_facts` (`domain/decision/matching.rs:94`) defines:

```text
hard_eligible =
    coverage == Complete
    AND exact normalized name
    AND no conflicting profile names
    AND meaningful matching normalized school
    AND observed matching-school location corroboration
    AND independently attributed participation
    AND no recognized hard evidence conflict
```

`school_location_matches` requires at least one supplied city/region field and corroborates all supplied location components from matching-school observations, grouped by team ID. It does not treat a mailing mismatch as proof the athlete attended elsewhere. Missing meaningful school or all mailing geography prevents this positive gate.

`strength` counts four booleans into 0/25/50/75/100. It is not an ML confidence, ranking override, or a replacement for hard eligibility. Year/grade do not appear in the eligibility conjunction.

`choose_decision` (`decision.rs:345`) branches:

| Condition | Decision |
|---|---|
| Missing source canonical name or meaningful school; incomplete search; any incomplete candidate group | `EvidenceReview` |
| Exact-name + matching-school candidate exists but fails hard eligibility | `EvidenceReview` |
| Exactly one hard-eligible candidate after those checks | `DeterministicAccepted` |
| Two or more hard-eligible candidates | `IdentityReview` |
| None hard-eligible, with complete non-uncertain evidence | `CompleteSearchNoMatch` |

**Important distinction:** missing school does not stop source discovery, but the current decision code does not automatically match that row or send it to the ordinary model ambiguity path. A row can finish as review-required without any GPU call.

## 12. Local model review, lane assignment, and selection authorization

### `resolve_assessment` is a narrow gate

`runtime/row_worker/review.rs:18`:

- Deterministic acceptance → `RowResolution::Accepted { method: Deterministic }`, no model call.
- Complete-search no-match → corresponding terminal row, no model call.
- Anything other than `IdentityReview` with 2–64 hard-eligible candidates → `ReviewRequired`, no model call.
- Build each `ReviewCandidate` from its profile and reasons; require every eligible candidate to have a profile.
- `ReviewInput` includes the **entire source record**, including private Golden context, and structured supplied candidates. This goes only to configured loopback models, not remote development agents.
- **J/S** `publish_review_input` enforces the 64 KiB input bound.
- **B** compute `review_case::case_key`.
- **D** `ReviewCase/<case-key>.review(input_digest)`, retaining/cancelling the child handle on 409.
- Map model select/unresolved/failure into `ReviewChoice`.
- **L** `decision::apply_review`; rejected or unresolved selection → `ReviewRequired`; authorized selection → `Accepted { method: LocalReview }`.

### `ReviewCase` prevents duplicate model assignment

`runtime/review_case.rs`:

- `case_key` hashes review protocol revision, complete source **fields**, supplied candidates, and **both configured model IDs**.
- Physical source location is deliberately excluded, allowing equivalent duplicate source-field/candidate cases to share review. The prepared HTTP request still contains a full source record; reviewers should examine the intended equivalence boundary.
- `review` loads/bounds input, requires 2–64 distinct candidate IDs and nonempty eligibility reasons, validates key.
- `assigned_lane`: first 16 hexadecimal key characters modulo five; buckets 0,1,2 → Q5/5090; buckets 3,4 → Q4/3090. This is a deterministic **3:2 bucket allocation**, not a guarantee of exact workload proportions or a 5:2 split.
- State `assignment` is set once and checked against both expected assignment and any cached `result`.
- **D** `LocalReviewer/<lane.key()>.review(ReviewJob { input, lane })`.
- Cache the returned `ReviewOutcome` as `result`.

There is one chosen model per equivalent case, **not two-model consensus**. Two distinct exclusive model lane keys permit independent cases to use both GPUs; one lane serializes its own requests.

### `LocalReviewer::review` and actual model HTTP

`runtime/reviewer.rs:39`:

1. Validate object key matches assigned lane.
2. Existing state `blocked` → return failed review without HTTP.
3. **B/S** `reviewer/input::prepare`: verified input load, `model::build_request`, bounded encoded request, configured loopback `/v1/chat/completions` endpoint.
4. **J/S** `publish_request` → `input::store_request`, max one action attempt.
5. `execute` derives operation key with effect label `local-review-http`.
6. **J/H** `run_attempt` calls `transport::request_once`, records attempt evidence, and returns a retryable ordinary handler error only when `Attempt::retryable()` is true.
7. SDK run policy: initial delay 1 s, factor 2, max delay 4 s, max attempts four.
8. Separate **J/S/L** `local-review-http-evidence-finalization` reloads attempt evidence and calls `finalize`. It does not resend HTTP merely to repeat finalization.
9. Durably sleep for finalization's retained post-operation cooldown.
10. If artifact integrity failed, persist lane `blocked`; return outcome.

**Corrections to raw GPU-review wording:** retryable model failures do not mean 409/concurrency rejection; 409 propagates cancellation. A lane block is not a routine rate-limit marker and there is no success path that clears an already-blocked lane. Current persistent block paths are artifact-evidence failures. Rate-limit cooldown and persistent lane block are different concepts.

`request_once` (`reviewer/transport.rs:21`) performs one POST using the shared HTTP client without source-session attachment:

- Capture status, media type, parsed Retry-After.
- `bounded_body` limits model response to 32 KiB.
- Parse successful envelopes; store complete raw response bytes and receipt.
- Transport timeout/connect/body failures can retry.
- 401/403 are nonretryable denial; 429/5xx are potentially transient.
- Retry-After <=1 s can fit the SDK guaranteed schedule. Larger valid delays suppress immediate retry and become durable post-operation cooldown. Invalid/excessive delays are nonretryable.
- Malformed model output is retained `MalformedResponse`, not a guessed selection or automatic success.

### Model request and response validation

`reviewer/model.rs::build_request` creates two messages (fixed system contract + serialized ReviewInput), temperature 0, max_tokens 1,024, `reasoning_effort: none`, `enable_thinking: false`, and JSON-object response format. The system text treats all source/retrieved text as untrusted data and forbids invented identity, unsupplied IDs, or graduation-based eligibility.

`parse_response` → `AssistantVerdict::validate`:

- Exactly one choice, `finish_reason == stop`, no refusal, nonblank content.
- Strict tagged JSON `Select { athlete_id, reason, evidence }` or `Unresolved { reason }`; deny unknown verdict/reference fields.
- Nonempty reason <=4,096 bytes.
- Selection requires 1–32 evidence refs and a supplied candidate with eligibility reasons.
- Locator must be nonempty, <=1,024 bytes, and an exact supplied `(document, locator)` from that candidate's allowed team/location, graduation, or issue evidence.
- Convert with `into_protocol` only after validation.

This checks reference authorization, not the truth of an arbitrary free-text reason or whether cited evidence genuinely distinguishes candidates. Domain authorization and independent export verification remain separate gates.

`finalize` selects the acknowledged attempt digest, or the last retained attempt only for interpreting a failed/unacknowledged effect. A retained successful HTTP response without an acknowledged successful effect becomes `UncertainEffect`/artifact failure, **not** an accepted reviewed outcome. Successful finalization retains request digest, final response, previous response receipts, lane, verdict, and SDK retry evidence.

### High-priority review finding: local-review acceptance appears unreachable

**[INFERENCE from the current source conjunctions; not a new executed regression test.]**

`decision::apply_selection` (`domain/decision.rs:240`) requires complete search, an allowed decision state, and a hard-eligible supplied candidate. For `IdentityReview`, it additionally requires `selection_distinguishes`.

`selection_distinguishes` (`decision.rs:263`) counts hard-eligible candidates equal to the selected candidate on:

1. `exact_name`
2. `matching_school`
3. `location_corroborated`
4. `mailing_location_matches`

But `candidate_facts` requires the first three to be true for **every** hard-eligible candidate, and sets `location_corroborated = mailing_location_matches`. Therefore all four are true for every hard-eligible candidate. `IdentityReview` requires at least two such candidates, so the distinguishing count is at least two, while authorization requires exactly one.

Consequently, the normal source-derived `IdentityReview` path can spend a model call and validate its returned selection, yet still reject that selection as indistinguishable. The existence of the `AcceptanceMethod::LocalReview` variant is not proof the automatic path can produce it. An external reviewer should resolve whether the desired behavior is conservative human review only or a different evidence-backed distinguishability contract. **Do not fix this by simply deleting the safety check or trusting the model reason.** No production code is changed by this packet.

## 13. Performance normalization is local Rust, not LLM arithmetic

After identity selection, export `projection::build_pr_summary` calls `domain::performance_evidence::summarize_performances`. Raw performance evidence remains in the sidecar for all retained profiles; an accepted athlete's compact summary is a separate projection.

```text
summarize_performances(results)
  ├─ results.map(observe)
  │    ├─ source_event → EventName::parse_retained
  │    ├─ timing_for
  │    ├─ source_unit_for
  │    ├─ context
  │    └─ parse_mark → Performance::from_status / Performance::parse_retained
  ├─ results.flat_map(source_claims)
  └─ observed_bests
       ├─ eligible → known_context + comparable recorded mark
       ├─ comparable_context
       └─ improves → compare_performances
```

`Performance::parse` (`domain/marks.rs:134`) → `parse_value` → mark parser functions (`parse_time`, `parse_distance`, `parse_integer`). Time and distance use integral scaled values (milliseconds/nanometers), not model-generated arithmetic. Event-specific direction controls better/worse. Unsupported, malformed, DNS/DNF/no-mark and incompatible events cannot become numeric bests merely because their raw strings exist.

Context includes sport, event/distance, surface, event type, timing, units, implement/hurdle equipment, wind legality, and attribution. Only known comparable contexts and **individual** results enter observed-best grouping. Relay membership can support participation for identity without turning a relay team mark into the individual's personal best.

Important distinctions:

- `source_claims` preserves PersonalBest/SeasonBest observations separately. Numeric TF flags remain unknown booleans.
- `observed_bests` means best among the supplied comparable retained sample, not career PR.
- `Completeness` has `RetrievedSampleOnly`; a complete acquisition does not upgrade that to an authenticated complete career corpus.
- Known legal and illegal wind contexts remain separate; unknown wind can prevent grouping.
- Timing suffixes such as `a`/`h`/`c` require corroborating timing metadata before stripping.
- Missing source units or incompatible mark units remain unsupported rather than guessing.
- Raw evidence and locators survive even where normalized comparison is unavailable.

The implementation scans existing context groups for each observation. That can become costly with many distinct groups; bounded input is not a proof of acceptable worst-case CPU latency. Review actual distributions before replacing it with a more elaborate index.

## 14. Export is an owner-online snapshot followed by verified staged publication

### CLI → `ExportWorker::publish`

The `export` CLI does **not** open the live Fjall store. It sends `ExportRequest { run, destination }` to a durable export owner:

1. `ExportRequest::key` → `normalize_destination` → fingerprint `native-export-worker-v4` + normalized destination.
2. Destination must be absolute, UTF-8, control-free, named `.xlsx`, without `..` traversal. Normalization is lexical; the parent's canonical path is resolved later during staging.
3. CLI transport idempotency uses the separate `native-export-v4` request fingerprint.
4. `ExportWorkerIngressClient::publish(...).send()` gives an invocation handle.
5. `await_export` repeatedly checks SDK output up to the 24-hour observation bound, sleeping one second between NotReady responses. This addresses the prior 300-second client observation timeout; it is not a guarantee the export will finish within 24 hours.
6. Ready returns `PublishedExport`; terminal failure is surfaced. Client observation timeout does not cancel the server-side invocation.

`ExportWorker::publish` (`runtime/export_worker.rs:97`) is an exclusive object keyed by **destination, not run**:

- Validate key; state `owner-run` prevents another run claiming the same destination.
- State `published-result` returns a previous publication. Repeating the same destination does not refresh a partial export with later row progress; use a new destination for a new snapshot.
- **D** `RunCoordinator/global.snapshot(run)` obtains one consistent `ExportSnapshot { progress, page_digests }`.
- Reuse `stage-receipt` if present; otherwise **J/B** `stage_export`.
- Verify stage run/destination binding.
- **J/B** `publish_bundle`.
- State `published-result` stores returned report, verification, and commit path.

### `stage_export`

`runtime/export_worker.rs:170`:

1. Canonicalize destination parent.
2. Create adjacent private `.native-export-*` staging directory, mode 0700.
3. `export_to(store, snapshot.progress, snapshot.page_digests, stage/export.xlsx)`.
4. Load source manifest.
5. `bundle_verify::verify_bundle(original, staged_xlsx, workbook_digest, EXPORT_HEADERS, store)`.
6. Compare independently verified total/accepted/review/no-match/pending counts with export coverage.
7. Hash staged XLSX and JSONL.
8. Keep staging directory and return `StageReceipt`.

Staging uses the worker's existing store handle, not a competing database owner. If acknowledgement of a kept stage is lost, retry may leave an orphan stage directory; the code explicitly notes this. No automatic garbage collector is established here.

### `export_to` and per-row projection

`runtime/export.rs:70`:

- **S** load SourceManifest.
- `index::validate_progress` checks page/counter/manifest consistency.
- `index::build_report_index` reads sealed `RunPage` digests and pending tail into a source-key → report-digest map, validating duplicate/binding/count constraints.
- Construct `WorkbookExport`, temporary JSONL and initial coverage.
- `write_artifacts` → `stream::sources` → per-sheet `stream_sheet`.
- Iterate source index in manifest order, pages of at most 256 physical row keys. For **every source row**, not merely selected rows:
  1. `store.source_record`.
  2. If report exists, `projection::project_report`.
  3. Otherwise `details::pending_fields`.
  4. `WorkbookExport::write(ExportRow { source, extra_fields })`.
  5. `details::write_detail` emits the same source, report, discovery, probes, assessment, profile acquisitions and raw performance evidence into one newline-terminated JSONL record.
  6. `record_coverage`/`count_resolution` update accounting.
- Validate coverage against source manifest and captured progress.
- Flush/fsync/publish staged JSONL, then `WorkbookExport::finish`, then hash workbook.
- `CompletenessState::Complete` only if **all workbook rows** have no pending row. A completed 10,000-row selected run on a 120,716-row workbook still produces a partial whole-workbook export.

The source-key/report index uses memory proportional to completed reports. Constant-memory XLSX writing does not make every export data structure constant-sized.

`projection::project_report` loads retained report/assessment/discovery/probe/full-profile artifacts and builds eleven appended fields:

```text
native.source_key          native.terminal_status       native.athlete_id
native.profile_url         native.acceptance_method     native.row_report_digest
native.assessment_digest   native.candidate_count       native.eligibility_basis
native.evidence_strength   native.pr_summary
```

Terminal statuses are `ACCEPTED`, `COMPLETE_SEARCH_NO_MATCH`, `REVIEW_REQUIRED`, or `PENDING`. Only acceptance populates selected identity/profile/method/performance summary. `native.eligibility_basis` is `source_workbook_membership`.

`build_pr_summary` emits compact observed-best/source-personal-best context for the selected athlete. If serialized content exceeds **32,767 UTF-16 units**, it emits an explicit `detail_sidecar` reference with source key, report digest and selected athlete linkage. It does not cut a JSON string or silently omit evidence. `details::write_detail` retains the underlying profile/result evidence from which the full summary can be reconstructed.

### `WorkbookExport::{new, write, finish}`

`src/workbook_export.rs`:

- `new` verifies original source SHA, validates extra-header uniqueness/collisions and Excel limits, creates one constant-memory worksheet per source sheet.
- `write` → `advance_to_sheet` → `validate_row` → `write_values`; enforces source order/physical row identity and expected fields, writes original values and appended fields, updates counts.
- `finish` validates every sheet and aggregate count, rejects source/frozen-source destination collision, saves to a temporary XLSX, flushes/fsyncs it, rechecks original SHA, persists without clobbering and fsyncs parent.

Original source **values/headers/row identity** are preserved; this is not an OOXML byte-for-byte clone with every style, chart, macro, or formula representation retained. The input file itself is never rewritten.

### `publish_bundle` and commit semantics

`runtime/export_worker.rs:239`:

```text
validate private stage paths
→ final_report (rewrite output paths, retain digests)
→ install_artifact(staged XLSX, destination)
→ install_artifact(staged JSONL, destination.with_extension("jsonl"))
→ persist_commit(destination.with_extension("commit.json"))
→ PublishedExport
```

`install_artifact` first verifies staged regular-file hash, then hard-links without clobbering. Existing/raced target is accepted only if a non-symlink regular file with exact expected hash.

`persist_commit` writes/fsyncs JSON via `persist_noclobber`; an existing receipt must equal the expected bytes, then parent is fsynced. The receipt is published **last**. XLSX/JSONL visibility is not an atomic two-file rename: missing commit means publication is not committed even if one or both files exist.

Do not confuse commit `BundleState::Complete` (two files verified/published) with `ExportReport.completeness == Complete` (all source rows processed). A committed partial result bundle is valid and explicitly partial.

## 15. Independent verification: what is checked and what is not

### Entry modes

- Owner-online: ExportWorker runs `verify_bundle` against its own open store handle in private staging.
- Offline CLI: `verify` checks the existing-store path/version marker, opens it only after the writer is stopped, then uses `spawn_blocking` for `verify_bundle`.
- Never invoke offline verification against a currently worker-owned Fjall directory. Successful directory inspection is not coordination or permission to take a second ownership.

### Layer A: `workbook_verify::verify_fields`

`src/workbook_verify.rs:59`:

1. Hash original, require expected workbook SHA.
2. Validate appended headers.
3. OOXML preflight original and output.
4. `open_sheet_names`: original source-sheet order must exactly equal output sheets; original generated sheets are deliberately excluded.
5. Open original/output with calamine.
6. For each sheet, `verify_sheet` streams independent cell/row views, validates headers and compares source physical row keys and original fields.
7. Check sheet/row/field/header accounting.
8. Rehash original; fail if it changed.

This is a distinct field-preservation pass, not a hash of the export writer's own counters. It does **not** prove identity decisions. It shares calamine/preflight/header-accounting components with other paths, so “independent” does not mean implementation-diverse libraries.

### Layer B: `result_verify::verify_results`

`src/result_verify.rs:60` reads JSONL with a 128 MiB **per-line** limit, newline requirement, no blank lines, strict DetailRow schema, and unique source keys:

```text
verify_results
  → validate_line
  → checks::verify_embedded_artifacts
      → verify_typed_value / verify_stored_artifact
      → verify_assessment_artifact
      → verify_profile_artifacts
      → verify_performance_evidence
      → coverage::verify
          → verify_discovery → discovery::verify
          → verify_assessment
          → verify_candidate
              Complete → complete::verify + raw_profiles::verify
              NameExcluded → raw_profiles::verify_probe + exclusions::verify
              Incomplete → retain/check incomplete classification
  → checks::verify_row
      → source/report binding, pending-state validation
      → assessment::verify → decision::assess
      → positive::verify_positive, no-match, or review-state checks
  → increment verified terminal-state counters
```

Key guarantees/limits:

- Embedded typed JSON must round-trip to its exact typed representation and match retained hash-verified artifact bytes where referenced.
- Coverage binds discovery job, query refs, unique candidate IDs, candidate order, probe/full-artifact identities, and completeness.
- Discovery verifier reconstructs planned queries, validates query sequence, reparses retained raw pages and reconciles pagination rather than trusting `complete: true`.
- `source_receipts::source_origin` loads frozen source snapshot; `verify_operation`, `successful_operations`, and `verify_receipt` bind captured request route/body and retained receipt to expected resource and origin.
- Complete profile reconstruction verifies initial TF/XC/HTML operation order, exact probe prefix, authorized deduplicated TeamNav order, reparses raw bodies, calls `assembly::assemble`, and requires exact profile equality and completeness.
- Exclusion reconstruction rechecks raw Bio/HTML witnesses and source-name binding.
- **Incomplete** acquisitions remain retained review evidence; they do not receive the same complete-raw-corpus certification as `Complete`.
- `assessment::verify` replays the row's cumulative 8 MiB metadata budget and coverage semantics, recomputes `decision::assess`, and requires exact canonical assessment.
- Accepted selection requires a `Complete` candidate and positive identity/participation checks.
- Deterministic acceptance must match assessment's verified selection.
- Local acceptance must have a retained selected verdict and supported profile evidence refs; `verify_local_authorization` recomputes assessment and calls `decision::apply_review` again.

The last check shares the current distinguishability gate described in section 12. The verifier does not make an unreachable positive path reachable.

### Layer C: `bundle_verify::verify_bundle`

`src/bundle_verify.rs:42`:

1. Hash output XLSX and JSONL.
2. Run field and result verifiers.
3. Require JSONL row count equals preserved source row count.
4. `bind_rows` streams exported workbook and JSONL in lockstep, rejects missing/extra lines, and calls `check_binding`.
5. `check_binding` checks physical row identity, exact original field equality, source membership annotation, report/workbook/source binding, terminal state, selected athlete/profile/method, candidate count, strength, and recomputed compact performance summary/overflow reference.
6. Rehash both outputs and original to detect mutation during verification.

`verify_bundle` itself verifies XLSX + sidecar + original + store; it does **not** consume the publication commit receipt. Atomic publication ownership is a separate ExportWorker protocol. A caller requiring “committed output” must also observe that protocol/receipt, not just a standalone bundle-verifier success.

### Do not inflate these guarantees

Verification does not:

- Authenticate upstream documents cryptographically to Athletic.net beyond locally captured transport evidence.
- Prove the source site's facts are true or its search covers every real-world athlete.
- Establish a complete career record.
- Prove every recorded model verdict was semantically correct just because its references are authorized.
- Prove absence of common-mode bugs: raw reconstruction shares production parsers/assembly/domain calculations.
- Make Restate external HTTP exactly-once.
- Make two independent machines safe concurrent owners of one local Fjall directory.
- Prove live 10,000-row completion, throughput, matching accuracy, or both GPUs' actual matching execution.

## 16. Complete production call graph

```text
main
└─ cli::run
   ├─ worker
   │  └─ worker::serve
   │     ├─ Runtime::open → WorkerConfig::load/validate
   │     │                 → SourceSession::load
   │     │                 → ArtifactStore::open
   │     ├─ Endpoint builders → bind 11 services → HttpServer::serve_with_cancel
   │     └─ Runtime::drain
   │
   ├─ deploy
   │  └─ transport::deploy → ensure_source_scope → local admin registration
   │
   ├─ start
   │  └─ cli::start
   │     ├─ PipelineControl.prepare
   │     │  ├─ WorkbookImport.load
   │     │  │  └─ import::import → snapshot::freeze → import_rows
   │     │  │     └─ workbook_ingest::visit_records → ImportBatch.push/flush → ArtifactStore
   │     │  └─ publish SourceSnapshot; return RunRequest
   │     └─ RunCoordinator/global.run
   │        ├─ validate snapshot + manifest
   │        ├─ SourceRows → bounded round-robin physical row selection
   │        ├─ Results → initial progress
   │        └─ drive → bounded durable RowWorker.process calls
   │           └─ RowWorker.process
   │              ├─ load_source → query_plan
   │              ├─ execute_queries → QueryWorker.gather
   │              │  └─ collect → SearchProgress
   │              │     └─ fetch → SourceCache.fetch
   │              │        └─ SourceGateway.fetch
   │              │           ├─ request::build
   │              │           └─ execute (bounded 4-attempt source schedule)
   │              │              ├─ SourceGateway.acquire → wait → admit
   │              │              ├─ run_step → http::perform
   │              │              │  ├─ apply source session → HTTP
   │              │              │  ├─ body/challenge/status classification
   │              │              │  └─ raw receipt + http_audit::record
   │              │              └─ finalize evidence → observe cooldown/block
   │              │     → parse_response → search::parse_page
   │              │     → SearchProgress.consume → QueryEvidence
   │              ├─ publish DiscoverySummary
   │              ├─ execute_profiles → ProfileWorker.identify
   │              │  └─ identify_probe → initial_phase
   │              │     ├─ acquire TF Bio/XC Bio/HTML via same SourceCache path
   │              │     ├─ parse_sources → parse_bio_value / parse_profile_html
   │              │     └─ publish ProfileProbe
   │              │  → NameExclusion::new?
   │              │     ├─ yes → explicit source-name-bound exclusion
   │              │     └─ no → ProfileWorker.gather → build
   │              │              ├─ reuse probe
   │              │              ├─ team_phase → authorized TeamNav via SourceCache
   │              │              ├─ parse_teams → parse_team_nav
   │              │              └─ assembly::assemble → ProfileAcquisition
   │              ├─ decision::assess
   │              │  ├─ validation + grouping
   │              │  ├─ source_identity + candidate_reason/candidate_facts
   │              │  └─ choose_decision
   │              ├─ resolve_assessment
   │              │  └─ only eligible ambiguity → ReviewCase.review
   │              │     ├─ case_key + assigned_lane
   │              │     └─ LocalReviewer.review
   │              │        ├─ input::prepare → model::build_request
   │              │        ├─ publish_request
   │              │        ├─ execute/run_attempt → request_once → local GPU HTTP
   │              │        │  └─ parse_response → AssistantVerdict::validate
   │              │        └─ audit finalization + cooldown/block → ReviewOutcome
   │              │     → decision::apply_review
   │              └─ publish_report
   │           → Results::record → sealed pages/progress → Results::finish
   │
   ├─ status → RunCoordinator.status
   │
   ├─ export → ExportWorker.publish → RunCoordinator.snapshot
   │  ├─ stage_export
   │  │  ├─ export_to
   │  │  │  ├─ validate_progress + build_report_index
   │  │  │  └─ sources → project_report → build_pr_summary → summarize_performances
   │  │  │     ├─ WorkbookExport.write/finish
   │  │  │     └─ write_detail
   │  │  └─ verify_bundle
   │  └─ publish_bundle → install XLSX/JSONL → persist_commit
   │
   └─ verify → existing_stopped_store → verify_bundle
      ├─ verify_fields
      ├─ verify_results → coverage/raw reconstruction/canonical assessment
      └─ bind_rows + rehash
```

## 17. Identity, cache, and state map

| Identity/owner | Binding and durable state | Consequence |
|---|---|---|
| Workbook digest | SHA-256 of original XLSX; frozen file and source index | Input content is stable; physical duplicates still have different source-row keys |
| Source row | Sheet + physical Excel row | Equal source values are not deduplicated from workbook accounting |
| WorkbookImport | Preparation/import key; `manifest` | Import can be reused only after current source/snapshot checks |
| Run identity | Prepared manifest/snapshot/selection/concurrency under run revision | `start` submits a durable run, not a transient process-local loop |
| RunCoordinator | `global`; run-specific progress, pages and result entries | One durable owner; shared observations can read consistent snapshots |
| SourceSnapshot | Acquisition revision + configured source origin + caller-provided label | Not bound to workbook bytes, not a frozen upstream website, and not a guarantee of simultaneous page capture |
| SourceCache | Snapshot + acquisition revision + resource; `result` | Dedupe source effects within defined scope; retained failure is also a cached outcome |
| QueryWorker | Snapshot + acquisition/parser revisions + query cache identity; `result` | Parser change must invalidate parsed search evidence |
| ProfileWorker | Snapshot + acquisition/parser revisions + athlete ID; `probe`, `result` | Source-independent probe/full acquisition reused across rows |
| RowWorker | Snapshot + row/parser revisions + workbook + source row; `result` | Source-specific coverage/decision must not leak to another physical row |
| Source admission | Scope/object state `blocked`, `not-before-ms` | Persistent safety latch and durable future deadline, not an OS sleep queue |
| ReviewCase | Review revision + full source fields + candidates + both model IDs; `assignment`, `result` | Equivalent cases reuse one assigned lane; model changes invalidate case identity |
| LocalReviewer | One key per Q5/Q4 lane; `blocked` | Per-lane serialization; artifact-integrity block requires operator handling |
| ExportWorker | Export-worker revision + normalized destination; `owner-run`, `stage-receipt`, `published-result` | One immutable publication per destination; never refresh by overwriting |
| HTTP operation | Invocation identity + named effect hashed by `http_audit::operation_key` | Connects acknowledged/unacknowledged effects with retained attempt evidence |
| EvidenceDigest | SHA-256 of exact serialized artifact/raw bytes | Integrity reference, not upstream authenticity or a workflow ID |

Current revision strings:

```text
calamine-ooxml-v1
native-prepare-v1
native-run-workbook-eligible-v2
captured-source-request-response-v7
streaming-source-parsers-v13
athlete-row-golden-context-v7
review-protocol-golden-context-v3
native-export-v4
native-export-worker-v4
```

Changing a revision creates different identities; it does not migrate existing retained artifacts or make changed handler code safe for active journal replay. Compatibility is the caller's responsibility. Preserve old frozen deployments for old journals.

## 18. Failure/recovery semantics and actual operational status

### There are several different kinds of “retry”

| Boundary | Behavior | Avoid this mistaken conclusion |
|---|---|---|
| reqwest client | Automatic retries disabled | “The HTTP client secretly retries until success” |
| Source `execute` | Four explicitly bounded workflow attempts, every retry through source admission | “Each 429 causes unbounded request pressure” |
| Source HTTP `ctx.run` | One SDK effect attempt per source schedule step | “Nested SDK and application budgets are the same thing” |
| Model HTTP `run_attempt` | SDK-controlled four-attempt bounded retry policy | “Source and model Retry-After policies are identical” |
| Evidence finalization | Separate bounded work over retained attempts | “Finalization retry necessarily resends HTTP” |
| Handler invocation policy | Bounded retries then pause | “No active invocations means all rows succeeded” |
| Durable child cancellation | 409 propagated; owned child handles cancelled/drained | “409 means a normal model concurrency rejection” |
| CLI export observer | Waits on existing durable invocation up to 24 hours | “Client timeout cancelled the worker export” |

An HTTP effect can reach the source/model and lose acknowledgement before durable capture. The implementation distinguishes `UncertainEffect`, `ArtifactFailure`, malformed/limited response, access denial, and exhausted retries. Do not describe this as exactly-once HTTP. Report what was acknowledged, what was captured, and what is unavailable.

Source blocks are safety state, not a nuisance to clear until progress resumes. A non-429 challenge can deny even with HTTP 200; challenged 429s use the bounded source backoff path and may ultimately latch a block. Already-admitted physical requests may finish after a block is learned. Scope usage includes occupied durable operations waiting on timers, not only active TCP requests.

### State of the real workload at the documented handoff

Consult the timestamped [HANDOFF.md](HANDOFF.md), not this static packet, for current process observation. Its historical checkpoint reports an unfinished 10,000-row run; real whole-workbook qualification and real matching execution on both GPUs were **not established**.

The retained denied response was a 429 HTML **Cloudflare Managed Challenge** at the AthleteBio endpoint, with “Just a moment…”, `cType: managed`, challenge-platform script, and a JavaScript/cookie instruction. The site operator's exact triggering rule remains unknown; handoff includes the safe timestamp/Ray ID/body digest. Private athlete parameters, original challenge tokens and source cookies stay outside the repository.

A local Chromium homepage navigation loaded normally. That does not establish API authorization or sustained workload clearance. The current source transport is reqwest plus optional authorized session headers; it neither launches Chrome nor executes challenge JavaScript.

### Browser/Spider is a prospective change, not part of the above graph

If the next task is browser acquisition, first define the transport contract:

- Same exact HTTP method, origin, path, query/body and cookie/UA isolation.
- One bounded action per admitted durable effect; no nested crawler retries/recursive discovery.
- Complete raw response, status, media type and evidence capture remain authoritative.
- Browser/process ownership, bounded pages and cancellation/drain behavior must be explicit.
- Authentication renewal must not overwrite active source semantics or leak private session material into artifacts.
- A fetched HTML string does not execute its script; navigation and browser-context API fetch have different behavior.
- Access restrictions still apply. An authorized browser does not remove source rate limits.
- Use a fresh acquisition revision/deployment and frozen binary; no changed browser code under active reqwest-era journals.
- Compare a public/authorized bounded native scenario and retained-evidence verification before claiming throughput or scaling the real workbook.

This packet does not choose a crawler library, introduce a browser dependency, claim to solve a CAPTCHA, or bypass an access restriction.

## 19. Prioritized questions for the next AI reviewer

### Highest priority: correctness and qualification

1. **Selection reachability:** section 12's conjunction makes every model-eligible candidate identical on `selection_distinguishes`. Establish intended behavior and an independently evidenced, safe distinguishability contract before claiming `LocalReview` acceptance works.
2. **Real source access:** what exact site-approved access path survives the required workload? A public homepage, unexpired cookie, or successful tiny probe is not an API/load qualification.
3. **Product/decision parity:** membership is eligible, but missing school/geography causes review and no ordinary GPU call. Is that conservative terminal behavior acceptable for all source rows?
4. **Meaning of completeness:** distinguish query pagination completion, full requested profile acquisition, selected run completion, all-source export completion, committed bundle publication, and career-corpus completeness.
5. **Verifier trust boundary:** positive raw reconstruction shares parser/assembly/decision code. Identify common-mode failure risks and independent actual-behavior witnesses; do not label shared recomputation formal proof.
6. **Model audit coverage:** separately examine whether positive export verification needs to reconstruct model request/response/lane/operation evidence as deeply as source acquisition. Current `verify_local_review` checks retained selected verdict and supplied profile refs; canonical domain reauthorization is a different check.

### Durability, ownership, and bounded resource risks

7. **Global source ownership:** prove the three admission scopes avoid queue starvation under long cooldowns and 16 occupied operations; preserve the quick-control/waiting-head split. Do not combine them casually.
8. **Wall-clock behavior:** admission deadlines use journaled Unix time, not a monotonic distributed clock. Inspect restart, clock jump, expiry, and extension semantics without clearing safety state.
9. **Effect/acknowledgement windows:** inspect capture-before/after-ack failures and uncertainty propagation. Retained bodies do not necessarily mean an acknowledged successful invocation.
10. **Whole-process bounds:** row metadata, raw response, parsed JSON, lol_html, CAS document, task count and export indexes have different bounds. Determine actual peak RSS/CPU, not a claimed universal “8 MiB” limit.
11. **Request amplification:** all query variants run, each query has its own pagination bound, candidate IDs cap only the profile set, and candidate TeamNav expansion has its own cap. Bound and measure effective source volume before raising row concurrency.
12. **Sequential acquisition:** profile `acquire` is sequential; eight-way buffering is parsing. Decide intentional pacing versus missed throughput only after source authorization/cooldown is understood.
13. **Cached terminal failures:** SourceCache/Query/Profile/Row/ReviewCase cache results within revision/key scopes. Determine which recovery action requires a fresh identity versus replay of the same recorded outcome.
14. **Artifact lifecycle:** Restate journals/state, source snapshots, CAS documents, attempt indexes, retained stages and export commits must survive compatible lifetimes. There is no proof here of comprehensive GC or backup/recovery policy.
15. **Export destination aliases:** lexical object-key normalization and later canonical parent resolution can treat filesystem aliases differently. No-clobber prevents silent overwrite, but inspect ownership/race/error behavior under trusted-directory and hostile-filesystem assumptions.
16. **Stage cleanup:** successful publication keeps stage hard links; lost stage acknowledgement may orphan another private directory. Determine operator-safe reclamation criteria without deleting live retained artifacts.

### Identity, privacy, and representation

17. **Search and domain normalization:** planner aliases/context and canonical domain headers have intentionally different roles. Check punctuation/accent/name-component behavior against real allowed source formats without exposing private rows.
18. **Exclusion soundness:** require both sport-specific Bio component identities and scoped HTML agreement. Do not optimize it into a search-display-name comparison.
19. **Location evidence:** preserve source-specific partial observations; same normalized school name alone must not authorize cross-team city/region splicing.
20. **Model citation semantics:** exact authorized refs can still support an irrelevant reason. Define what supplied evidence can distinguish identities; never promote email-domain assumptions or graduation-based shortcuts.
21. **Session file boundary:** mode/regular-file/final-symlink checks do not by themselves prove hostile-ancestor/TOCTOU/ACL safety. Determine trusted-directory assumptions and operational renewal without sending credentials to either review agent.
22. **Performance claims:** keep unknown/opaque flags, incompatible event contexts, relays and observed-sample bests distinct. Review quadratic context grouping only with measured workloads.
23. **Pending and duplicate rows:** every physical input row remains represented, including unselected/missing-identity/duplicate-value records; semantic ReviewCase deduplication must not erase workbook rows.
24. **Deployment compatibility:** any implementation change must update affected identities and verifier contract, preserve old frozen journals, and demonstrate native recovery. A compile or code review is not a live-scale acceptance result.

### How the two local GPU reviews were used

Both reviewers supplied useful maps and questions, but their raw prose was **not accepted as evidence without checking source**. Corrections integrated into this packet include:

| Raw-review error or overstatement | Source-grounded disposition |
|---|---|
| Same `SourceGateway/global` key means all shared calls are one invocation | Shared handler calls are distinct; SourceCache provides the resource-key reuse |
| Cache and CAS are redundant double body storage | Restate retains outcome/state; Fjall retains bytes and evidence |
| 409 is a concurrency rejection | This code handles it as cancellation propagation |
| Admission uses a monotonic deadline | It uses journaled Unix time |
| Model assignment is 5:2 | Five buckets partition 3:2 |
| A successful review clears lane block | No such clear path exists |
| Probe exclusion requires missing HTML | HTML presence and validated identity evidence are required |
| Profile HTTP fan-out is eight | Acquisition is sequential; parser futures buffer eight |
| Planner emits dozens of queries and can paginate empty pages indefinitely | Six variant families at most before two-sport dedup; progress rejects empty continuation and has hard page/count bounds |
| A school ending in “High School” is entirely rejected | A meaningful name normalizes the suffix; generic “High School” is not a distinguishing school identity |
| Candidate IDs are lexicographic strings | Typed numeric athlete IDs drive the ordered set |
| macOS is a non-Unix exception to permission checking | macOS is Unix; do not infer its behavior from a Windows-oriented branch |
| Missing Retry-After means an immediate 429 retry | Source fallback backoff applies |
| Missing required positive source identity can be skipped | Canonical assessment and positive verification require the necessary identity/corroboration contract |

The GPU-utilization observation during this review showed both devices active (5090 89%, 3090 94% at the sampled moment). This proves reviewer activity only. It does **not** satisfy the outstanding requirement to demonstrate real workflow matching invocations on both lanes.

### Copyable review brief

> Review the implemented native pipeline at source baseline `fee6c82` using `WORKFLOW_REVIEW.md` and `HANDOFF.md` as maps, not authority over source. Start with the apparent unreachable local-review acceptance gate, source admission/uncertain-effect boundaries, raw evidence reconstruction, and export publication semantics. Separate confirmed defects, source-derived inferences, intentional conservative policies, and unexecuted acceptance. Cite file/function/line and give the smallest observable failure scenario. Preserve source membership, all original fields, private local-only matching context, conservative ambiguity handling, native Restate/Fjall ownership, and frozen-journal compatibility. Do not run unit suites or fuzz campaigns, open the live store from another process, restart GPU servers, clear blocks, replace active binaries, or invent completed 10,000-row/GPU qualification. Browser acquisition is not implemented and requires its own explicit transport/recovery contract. Propose fixes; do not quietly weaken safety gates to manufacture a positive outcome.
