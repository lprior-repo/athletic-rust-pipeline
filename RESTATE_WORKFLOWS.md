# Restate workflow/API catalog

Implemented endpoint surface in `crates/census-service/src/restate_services/`.
[Architecture §5](ARCHITECTURE.md#5-workflow-backbone-7-9) owns orchestration invariants;
[ADR-002](docs/adr/ADR-002-restate-owns-retries.md) owns retry policy. Operations and deployment
procedures live in [the runbook](docs/OPERATIONS.md) and [lifecycle](docs/deployment-lifecycle.md).

## Bound services

`build_endpoint` binds nine store-backed definitions and optionally `BrowserSession`. Wire names
and serialized requests are public contracts; do not rename them or reinterpret existing keys
without an explicit compatible deployment/migration decision.

| Definition | Kind | Handlers | Identity/purpose |
|---|---|---|---|
| `Census` | Service | `status`, `open_work`, `seal` | Counts, durable open-work observations, seal/refusal |
| `Consolidate` | Workflow | `run` | Caller run key; consolidation job |
| `Report` | Workflow | `run` | Caller key bound to scope/generation; report job |
| `Bests` | Workflow | `run` | Caller key bound to scope/cohort/limit/generation |
| `Workbook` | Workflow | `run` | Caller key bound to cohort/scope/limit/generation |
| `Ingest` | Virtual object | `record`, `state`, `complete_window` | Source endpoint object; application/cursor/window bookkeeping |
| `Sweep` | Workflow | `run`, `interrupt` | Observe named ingest endpoints over bounded windows; durable stop promise |
| `JurisdictionCensus` | Virtual object | `state`, `run` | `jurisdiction:<state>:<season>:<revision>`; teams, rosters, meets, results |
| `NationalCensus` | Workflow | `run`, `report` | `national:<season>:<scope>:<revision>`; jurisdiction fan-out and report folding |
| `BrowserSession` | Virtual object | `fetch`, `status`, `start`, `stop` | Default `profile-0`; one endpoint-owned persistent profile |

National failures are retained in the report rather than erasing the jurisdiction. Reconciliation,
identity review, gap resolution and full generation verification are not separate bound durable
stages merely because batch functions or target stage names exist. Their end-to-end integration is
tracked in [the delivery plan](docs/NATIONAL-CENSUS-PLAN.md).

## Wire ownership and idempotency

The authoritative request/reply definitions are `wire.rs` and `wire/`; the catalog does not duplicate
their full JSON schema. Important contracts:

- `IngestRequest`: `table`, `rows`, `operation_id`, optional `cursor`.
- `IngestReply`: endpoint, newly appended count, total observations, cursor and last-append time.
- `IngestState`: endpoint, total observations, cursor, last-append time, completed windows and the
  existing seen-operation bookkeeping. Fjall receipts, not this bookkeeping alone, protect effects.
- `WindowRequest` identifies a completed source window. A successful-empty window must not be
  indistinguishable from never-attempted work.
- `SweepRequest` supplies endpoint keys, number of windows and interval; replies include observed
  endpoints, stale entries, interruption/report state and receipt-pruning counts.

The current ingest digest hashes table identity and serialized rows. The handler resolves the table,
decodes rows to its domain representation and calls the store application path. That is not proof
of cross-source identity acceptance or a complete capture/parser/run binding.
`StoreBatch::commit_once` atomically persists its staged rows/accounting/receipt; replay with identical
identity/digest reuses the receipt and adds zero rows. Changed payload under the same ID fails.
[FJALL_SCHEMA.md](FJALL_SCHEMA.md) owns exact storage semantics and retention limitations.

No distributed transaction spans Restate and Fjall. A crash after external commit and before a
reusable step acknowledgement can re-execute the closure. The receipt must close that effect window;
merge-on-read deduplication is not an adequate substitute. HTTP/browser calls are not exactly-once.

## Retry, retention and timeouts

Current declared invocation policies:

| Definition | Journal retention | Idempotency retention | Exhaustion |
|---|---|---|---|
| `Census` | 1 hour | 30 days | Three attempts, pause |
| `Consolidate`, `Report`, `Bests`, `Workbook`, `Ingest`, `Sweep`, `NationalCensus` | 90 days | 30 days | Three attempts, pause |
| `JurisdictionCensus` | 90 days | 30 days | Three attempts, **kill** so parent folding can continue |
| `BrowserSession` | 90 days | 30 days | One attempt, pause; caller owns the enclosing retry decision |

The three-attempt policies use 500 ms initial interval and 1 minute maximum. Workflow completion
retention is separately declared; do not equate it with receipt or journal age. The nine store-backed
services advertise 1-hour inactivity and 1-hour abort timeouts through service options; the browser
uses SDK defaults. Queueing before journal progress consumes inactivity time.

Heavy jobs run through the shared bounded `Jobs`/blocking path inside `ctx.run`, with the step's
`max_attempts(1)` avoiding a second local budget. A whole workbook inside one such closure is one
completed replay boundary, **not** resumable internal export partitions. Transport performs one
attempt. Invocation limits alone do not prove the physical-request ceiling across all child effects;
that requires the native request-counter faults in the acceptance catalog.

## Command routing

| Command | Durable surface |
|---|---|
| `census-service national`, `national-report` | `NationalCensus/run`, shared `report` |
| `census-service jurisdiction` | `JurisdictionCensus/run` |
| `census-service open-work`, serving `seal` | `Census/open_work`, `Census/seal` |
| `cargo xtask census-status` | `Census/status` |
| `cargo xtask coverage`, `export` | `Report/run`, `Workbook/run` |
| `census-service browser-session <action>` | `BrowserSession/<action>` |

Serving calls use loopback Restate ingress, default port 18095, without opening a second store.
Explicit offline routes are described in [the CLI reference](crates/census-service/README.md).
Neither a new store directory nor a different output filename makes an old durable key a fresh run.

## Cancellation and replay boundaries

`Sweep::interrupt` resolves its stop signal. Endpoint shutdown stops intake, drains owned work to a
deadline, classifies remaining outcomes and flushes storage; see the runbook for the drain certificate.
Cancelling an async waiter does not prove a started blocking effect was undone. Recovery must reconcile
committed effects and retained unfinished work, not infer success from process exit.

Completed journaled effects reuse recorded results; uncompleted effects may execute again. Keep
workflow branching, attributed evidence, time-derived queries and call order stable under the stored
run contract. Installing/removing a browser capability currently changes the plan fingerprint; handle
that as a deliberate compatible run transition, not an automatic retry under a new key.

## Adding/changing a handler

1. Main reviews wire types, logical identity, effect boundary and replay compatibility.
2. Use existing bounded admission and `Jobs`/supervisor ownership; no competing retry loop or pool.
3. Bind the definition in `build_endpoint` and migrate callers/discovery expectations together.
4. Exercise real same-key replay, terminal failure and reached crash boundaries before claiming
   durability. The handler catalog is not test evidence.
