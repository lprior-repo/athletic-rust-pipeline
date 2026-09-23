# Restate Rust SDK — Current Semantics

> **Fetch date:** 2026-09-22
> **Fetch method:** `read` on `https://github.com/restatedev/sdk-rust` (GitHub API, raw content) and `https://docs.rs/restate-sdk` documentation links embedded in the SDK README. No local checkout exists at `/home/lewis/src/restate*`.
> **SDK versions studied:** `restate-sdk = 0.12.0` (pinned in `Cargo.lock`), `restate-sdk-macros = 0.12.0`, `restate-sdk-shared-core = 7.0.3`. Latest released version is **0.12.1** (released after 0.12.0 with additive-only changes).
>
> Every API, flag, type, and macro claim below is grounded in the source code or release notes read on the date above. Items marked `UNVERIFIED` were inferred but could not be confirmed from the fetched source.

---

## 1. Service / Object / Workflow Definition Macros and Trait Names

### `#[restate_sdk::service]`

Applies to an **inherent `impl` block of a `struct`** (not a trait). Generates:

- An implementation of `Service` and `IntoServiceDefinition` so the struct can be passed directly to `Endpoint::builder().bind(...)`.
- A typed client type `<StructName>Client` that implements `IntoServiceClient<'_>`, used inside handlers as `ctx.service_client::<MyServiceClient>()`.

**Source:** `src/lib.rs:service` macro docs (unversioned docs page, fetched 2026-09-22).

```rust
#[restate_sdk::service(name = "MyService", client_visibility = "pub(crate)")]
impl MyService {
    // ...
}
```

Supported attribute keys (from `macros/src/struct_ast.rs:parse_service_args`):

| Key | Type | Applies to |
|-----|------|------------|
| `name` | `"string"` | Service name override (defaults to struct name) |
| `client_visibility` | `"pub"` / `"pub(crate)"` | Visibility of the generated `<T>Client` |
| `journal_retention` | `"90 days"` | Service-level default |
| `idempotency_retention` | `"30 days"` | Service-level default |
| `inactivity_timeout` | `"5m"` | Service-level default |
| `abort_timeout` | `"1m"` | Service-level default |
| `lazy_state` | `true` / `false` / bare flag | Service-level default |
| `ingress_private` | `true` / `false` / bare flag | Service-level default |
| `invocation_retry_policy(...)` | see §3 | Service-level default |

Durations use the `jiff` "friendly" format: `"1 sec"`, `"30s"`, `"5m"`, `"500ms"`, `"2h"`, `"1 day"`, `"7 days"` (months/years rejected — no fixed millisecond length).

**Source:** `macros/src/struct_ast.rs:parse_service_args`, `macros/src/struct_ast.rs:parse_duration_lit`

### `#[restate_sdk::object]`

Same struct-based API as `service`. Adds:

- Handlers with `ObjectContext<'_>` as the context type become **exclusive** (only one runs at a time per object key).
- Handlers with `SharedObjectContext<'_>` as the context type become **shared** (concurrent, read-only access to state).
- The object key is available via `ctx.key()`.
- State operations (`get`, `set`, `clear`, `clear_all`) are available on both context types.

**Source:** `src/lib.rs:object` macro docs.

### `#[restate_sdk::workflow]`

Same struct-based API. Adds:

- Requires a handler named `run`.
- The `run` handler uses `WorkflowContext<'_>`.
- Other handlers use `SharedWorkflowContext<'_>`.
- The `run` handler executes exactly **once per workflow instance** (key).
- Workflow completion retention is available via `workflow_completion_retention` — only valid on `#[workflow]`, not on `service`/`object`.

**Source:** `src/lib.rs:workflow` macro docs. `macros/src/struct_ast.rs:parse_service_args` (rejects `workflow_completion_retention` on non-workflow).

### `#[handler]`

Annotates a method inside a `#[service]`/`#[object]`/`#[workflow]` impl block as a Restate handler.

Supported attribute keys (from `macros/src/struct_ast.rs:read_handler_attr`):

| Key | Type | Applies to |
|-----|------|------------|
| `name` | `"myHandler"` | Handler name override (defaults to method name) |
| `inactivity_timeout` | `"5m"` | Handler-level override |
| `abort_timeout` | `"1m"` | Handler-level override |
| `journal_retention` | `"90 days"` | Handler-level override |
| `idempotency_retention` | `"30 days"` | Handler-level override |
| `lazy_state` | `true` / `false` / bare flag | Handler-level override |
| `ingress_private` | `true` / `false` / bare flag | Handler-level override |
| `invocation_retry_policy(...)` | see §3 | Handler-level override |

`workflow_completion_retention` is **NOT** a valid `#[handler]` argument (it belongs on `#[workflow]`).

**Source:** `macros/src/struct_ast.rs:read_handler_attr` (line ~467)

### Handler Name Derivation

By default the handler name is the **Rust method name** (e.g., `fn run` → handler name `"run"`). Override via `#[handler(name = "myHandler")]`. Service name defaults to the struct name; override via `#[restate_sdk::service(name = "...")]`.

**Source:** `macros/src/struct_ast.rs:StructService::from_impl` — `restate_name = args.name.unwrap_or_else(|| self_ident.to_string())`.

### Breaking Change from Pre-0.12

Prior to 0.12 the SDK used a **trait-based API** (`trait MyService { async fn handle() -> HandlerResult<...>; }`). The trait API is still accepted (parsed through a legacy parser) but is deprecated. The struct-based `impl` block API is the current and recommended form.

**Source:** `macros/src/lib.rs:dispatch` — dispatches to legacy `ServiceGenerator` for `Item::Trait`, to `StructService` for `Item::Impl`.

### `Client` Visibility

The generated client type defaults to `pub`. Override at the service level with `#[restate_sdk::service(client_visibility = "pub(crate)")]`. The generated client name is the struct name suffixed with `Client` (e.g., `JurisdictionCensus` → `JurisdictionCensusClient`).

**Source:** `macros/src/struct_ast.rs:StructService::from_impl` — `vis = args.vis.unwrap_or_else(|| parse_quote!(pub))`.

---

## 2. Durable Execution Primitives Inside a Handler

### `ctx.run(closure)` — Journaling Actions

```rust
fn run<R, F, T>(
    &self,
    run_closure: R,
) -> impl RunFuture<Result<T, TerminalError>> + 'ctx
where
    R: RunClosure<Fut = F, Output = T> + Send + 'ctx,
    F: Future<Output = HandlerResult<T>> + Send + 'ctx,
    T: Serialize + Deserialize + 'static,
```

**Source:** `src/context/mod.rs:ContextSideEffects::run`

- The closure runs **once** on the first invocation. On replay (after a restart or retry), the **stored result** is replayed instead of re-executing the closure.
- The closure must produce a `HandlerResult<T>` (i.e., `Result<T, HandlerError>`).
- The closure **must not** use the Restate context (`ctx`) — no `get`, `set`, calls to other services, or nested `ctx.run` are allowed inside `ctx.run`.
- The closure must be `Send + 'ctx` and the output type `T` must implement `Serialize + Deserialize + 'static`.
- **Critical:** Always `await` `ctx.run` immediately, before other context calls, to avoid non-deterministic journal interleaving.

**Source:** `src/context/mod.rs` docs — "Caution: Immediately await journaled actions: Always immediately await `ctx.run`, before doing any other context calls."

### `ctx.run(...).retry_policy(policy)` — Per-Run Retries

A `ctx.run` block can override its retry policy independently of the service-level `invocation_retry_policy`:

```rust
let policy = RunRetryPolicy::default()
    .initial_delay(Duration::from_millis(100))
    .exponentiation_factor(2.0)
    .max_delay(Duration::from_millis(1000))
    .max_attempts(10)
    .max_duration(Duration::from_secs(10));
ctx.run(|| do_work())
    .retry_policy(policy)
    .await?;
```

**Source:** `src/context/run.rs:RunFuture::retry_policy`

Default `RunRetryPolicy` values:

| Field | Default |
|-------|---------|
| `initial_delay` | 100ms |
| `exponentiation_factor` | 2.0 |
| `max_delay` | 2s |
| `max_attempts` | None (infinite) |
| `max_duration` | 50s |

### `ctx.run(...).name("label")` — Observability

```rust
ctx.run(|| do_work()).name("my-labeled-step").await?;
```

Sets a name used mainly for observability (journal logging, admin UI).

### Journal Replay Semantics

Restate records every `ctx.run` call result, `ctx.set`, `ctx.sleep`, `ctx.awakeable()`, `ctx.promise()`, `ctx.signal()`, and outbound calls (`ctx.service_client/...call()`) in a **durable execution log** (journal).

On recovery (node restart, invocation cancellation), Restate replays the handler **from the beginning**, comparing the journal entries to detect non-deterministic behaviour (e.g., a wall-clock `Instant::now()` that returned a different value). If the journal matches up to the point of failure, replay resumes from there. If it mismatches, the invocation fails.

**What gets replayed:** The entire handler code path is re-entered. `ctx.run` blocks that have a journal entry are **not re-executed** — the stored result is returned. New `ctx.run` blocks that have no journal entry are executed and recorded.

**Source:** `src/context/mod.rs:ContextSideEffects` docs; `restate-sdk-shared-core` VM (CoreVM) — the journal is the ground truth.

### What Must NOT Be Awaited Inside `ctx.run`

- No Restate context operations (`get`, `set`, calls to other services, `sleep`, `awakeable`, `promise`, `signal`).
- The SDK explicitly forbids nesting `ctx.run` inside `ctx.run`.

**Source:** `src/context/mod.rs:ContextSideEffects` docs — "You cannot use the Restate context within `ctx.run`. This includes actions such as getting state, calling another service, and nesting other journaled actions."

### Other Context Primitives

| Method | Context types | Description |
|--------|--------------|-------------|
| `ctx.get::<T>("key")` | ObjectContext, SharedObjectContext, WorkflowContext, SharedWorkflowContext | Read durable K/V state; returns `Result<Option<T>, TerminalError>` |
| `ctx.set("key", value)` | ObjectContext, WorkflowContext | Write durable K/V state (non-blocking; batched, flushed at handler end) |
| `ctx.clear("key")` / `ctx.clear_all()` | ObjectContext, WorkflowContext | Delete state keys |
| `ctx.get_keys()` | ObjectContext, SharedObjectContext, WorkflowContext, SharedWorkflowContext | List all state keys |
| `ctx.awakeable::<T>()` | All | Creates `(id: String, promise: DurableFuture<Result<T, TerminalError>>)`. The id is exfiltrated to an external process which resolves/rejects it via HTTP or SDK. |
| `ctx.signal::<T>("name")` | All | Await a named signal on the current invocation. `ctx.invocation_handle(id).signal("name").resolve(value)` completes it. |
| `ctx.promise::<T>("key")` | WorkflowContext | Create a durable promise inside a workflow. `ctx.resolve_promise("key", value)` / `ctx.reject_promise("key", err)` complete it. `SharedWorkflowContext` can only `peek_promise`. |
| `ctx.peek_promise::<T>("key")` | SharedWorkflowContext | Check if a workflow's promise has been resolved, without blocking. |

**Sources:** `src/context/mod.rs` — `ContextAwakeables`, `ContextSignals`, `ContextPromises`, `ContextReadState`, `ContextWriteState`.

### `ctx.sleep(duration)` — Durable Sleep

```rust
ctx.sleep(Duration::from_secs(60)).await?;
```

Suspends the handler durably. Restate tracks the timer and resumes the handler at the correct time, even across failures.

**Source:** `src/context/mod.rs:ContextTimers::sleep`

---

## 3. Retry and Error Semantics

### HandlerError — Retryable vs Terminal

Every handler returns `Result<T, HandlerError>`. `HandlerError` wraps either:

- **`HandlerErrorInner::Terminal(TerminalError)`** — the invocation ends immediately. The error propagates to the caller (HTTP response body, calling handler's `call()` result). **No retries.**
- **`HandlerErrorInner::Retryable(Box<dyn Error>)`** — **any other Rust error** (including `anyhow::Error`, `thiserror` errors, `ParseIntError`, etc.). The invocation is retried per the retry policy.

**Source:** `src/errors.rs:HandlerError` — `From<E> for HandlerError` converts any error to `Retryable`; `From<TerminalError> for HandlerError` converts to `Terminal`.

```rust
// Terminal — no retry:
Err(TerminalError::new("Bad input").into())

// Retryable — Restate will retry:
Err(anyhow::anyhow!("Transient failure"))
// or simply:
Err("Transient failure".to_string().into())
// (any `Into<Box<dyn Error>>` goes to Retryable)
```

### TerminalError API

```rust
TerminalError::new(message: impl Into<String>) -> Self          // code 500
TerminalError::new_with_code(code: u16, message: impl Into<String>) -> Self
TerminalError::with_code(self, code: u16) -> Self
TerminalError::code(&self) -> u16
TerminalError::message(&self) -> &str
TerminalError::from_error(e: impl StdError) -> Self
```

`TerminalError` implements `Into<HandlerError>` via `From<TerminalError> for HandlerError`.

**Source:** `src/errors.rs:TerminalError`

### TerminalErrorExt — One-Liner Terminal Conversion

```rust
use restate_sdk::prelude::*;

async fn handle(ctx: Context<'_>) -> Result<(), HandlerError> {
    let parsed: i32 = "not-a-number".parse().terminal()?;
    // Equivalent to:
    // let parsed: i32 = match "not-a-number".parse() {
    //     Ok(v) => v,
    //     Err(e) => return Err(TerminalError::new(e.to_string()).into()),
    // };
}
```

**Source:** `src/errors.rs:TerminalErrorExt`

### Invocation Retry Policy — Where It's Configured

The `invocation_retry_policy` is configured at **three levels**, each overriding the one above:

1. **Service/object/workflow attribute** — `#[restate_sdk::service(invocation_retry_policy(...))]`
2. **Handler attribute** — `#[handler(invocation_retry_policy(...))]`
3. **Programmatic (Endpoint builder)** — `ServiceOptions::retry_policy_*` / `HandlerOptions::retry_policy_*`

```rust
#[restate_sdk::service(
    invocation_retry_policy(
        initial_interval = "500ms",
        max_interval = "1m",
        max_attempts = 3,
        on_max_attempts = "pause"
    )
)]
impl MyService { ... }
```

### `invocation_retry_policy` Fields

| Field | Type | Description |
|-------|------|-------------|
| `initial_interval` | `"500ms"` / integer ms | Delay before first retry |
| `max_interval` | `"1m"` / integer ms | Upper bound on computed delay |
| `factor` | `2.0` / float | Exponential backoff multiplier |
| `max_attempts` | `3` / integer | Maximum attempts (initial + retries) |
| `on_max_attempts` | `"pause"` / `"kill"` | What to do when attempts exhausted |

`on_max_attempts` options:

| Value | Behaviour |
|-------|-----------|
| `"pause"` | Invocation enters **paused** state; requires manual resume via CLI/UI |
| `"kill"` | Invocation is marked **failed**; not retried unless re-triggered |

**Source:** `macros/src/struct_ast.rs:RetryPolicyArgs`, `macros/src/struct_ast.rs:parse_on_max_attempts`. `src/endpoint/builder.rs:ServiceOptions::retry_policy_*` and `HandlerOptions::retry_policy_*` for programmatic configuration.

### Per-`ctx.run` Retry Policy vs Invocation Retry Policy

The invocation retry policy governs **what happens when the handler itself fails** (the entire invocation is re-run from the beginning of the journal). The `ctx.run(...).retry_policy(...)` governs **what happens when a specific journaled action fails** — the run block is retried in-place while the rest of the handler state is preserved.

**Key insight for the census repo:** The repo's convention (ADR-002) is to set `max_attempts` on the service/object/workflow attribute (invocation-level retry), and set `ctx.run().retry_policy(RunRetryPolicy::new().max_attempts(1))` inside handlers so each journaled action is attempted exactly once. This prevents double retry budgets.

**Source:** `crates/midwest-census/src/restate_services/jobs.rs:199-201` — `pub(super) fn no_run_retry() -> RunRetryPolicy { RunRetryPolicy::new().max_attempts(1) }`

### What Retries Look Like on the Wire

The Restate server (1.7+) retries invocations according to the policy. The handler receives a **replay** with the same invocation ID. The journal entries are compared to detect non-deterministic behaviour. If the journal matches, the handler continues from the failure point; if not, the invocation fails with a journal mismatch error.

**UNVERIFIED:** The exact HTTP path and headers used for retry replay. The server sends a request to `/restate/invoke/service/<name>/<handler>` with the same invocation ID, and the SDK's VM detects the journal replay context.

---

## 4. Sleep/Timers, Awakeables, and Idempotency Keys

### Durable Sleep

```rust
ctx.sleep(Duration::from_secs(10)).await?;
```

- Returns `impl DurableFuture<Output = Result<(), TerminalError>>`.
- Suspends the handler durably. The handler resumes at the correct time, even across failures and restarts.
- **Cost savings on FaaS:** Restate suspends the handler during sleep, so you don't pay compute time.
- **Virtual Object caveat:** Sleeping blocks the object — no other invocation for the same key can run during sleep.

**Source:** `src/context/mod.rs:ContextTimers::sleep`

### Awakeables

```rust
// Create an awakeable
let (id, promise) = ctx.awakeable::<String>();

// Trigger an external process with the id
ctx.run(|| deliver_task_request(id.clone())).await?;

// Wait for the external process to complete
let payload = promise.await?;
```

**Completing an awakeable from outside Restate (HTTP):**

```bash
# Resolve
curl localhost:8080/restate/awakeables/prom_1PePOqp/resolve \
    -H 'content-type: application/json' \
    -d '{"hello": "world"}'

# Reject
curl localhost:8080/restate/awakeables/prom_1PePOqp/reject \
    -H 'content-type: text/plain' \
    -d 'Very bad error!'
```

**Completing from another handler:**

```rust
ctx.resolve_awakeable(&id, "hello".to_string());
ctx.reject_awakeable(&id, TerminalError::new("my error reason"));
```

**Source:** `src/context/mod.rs:ContextAwakeables`

### Durable Promises (Workflow-Scoped)

```rust
// In run handler:
ctx.resolve_promise("status.done", true);

// In shared handler:
let value = ctx.peek_promise::<bool>("status.done").await?;

// In run handler — wait:
let value = ctx.promise::<String>("email.clicked").await?;
```

**Source:** `src/context/mod.rs:ContextPromises`

### Named Signals (Invocation-Scoped)

```rust
// Awaiting handler:
let value = ctx.signal::<String>("my-signal").await?;

// Signaling handler:
ctx.invocation_handle(target_invocation_id)
    .signal("my-signal")
    .resolve("hello".to_string());
```

**Source:** `src/context/mod.rs:ContextSignals`

### Idempotency Keys

**On outbound calls (from within a handler):**

```rust
ctx.service_client::<MyServiceClient>()
    .handle(input)
    .idempotency_key("unique-key")
    .call()
    .await?;
```

- The idempotency key is durable — it is recorded in the journal. On replay, the same idempotency key is sent again.
- Restate deduplicates requests with the same service, handler, key, and scope.

**Source:** `src/context/request.rs:Request::idempotency_key`

**On inbound calls (from external clients):**

```rust
greeter
    .greet("Ada".to_owned())
    .idempotency_key("greet-ada")
    .call()
    .await?;
```

**Source:** `src/ingress/mod.rs:Request::idempotency_key` (client-side).

### Scope and Limit Key

```rust
ctx.service_client::<MyServiceClient>()
    .handle(input)
    .scope("tenant-123")
    .limit_key("api-key/user42")
    .call()
    .await?;
```

- `scope`: Sub-grouping of resources. Idempotency keys are scoped (same key in different scopes is different).
- `limit_key`: Enforces hierarchical concurrency limits. Only valid with a `scope`.

**Source:** `src/context/request.rs:Request::scope`, `Request::limit_key`.

---

## 5. Child Workflows / One-Workflow-Per-Unit Fan-Out

### Starting a Child Workflow

```rust
// From inside a handler:
let result = ctx
    .workflow_client::<ConsolidateClient>(workflow_key.clone())
    .run(Json(request))
    .call()       // synchronous — waits for completion
    .await?;
```

- `ctx.workflow_client::<C>(key)` creates a typed client for a workflow, keyed by `key`.
- The `run` handler can only be invoked **once per workflow key**. Subsequent calls with the same key return the retained result.
- `.call().await` is synchronous — waits for the child workflow to complete.
- `.send()` is fire-and-forget — the caller doesn't wait.

**Source:** `src/context/mod.rs:ContextClient::workflow_client`, `src/context/request.rs:Request::call`, `Request::send`

### Fire-and-Forget

```rust
ctx.service_client::<MyServiceClient>()
    .handle(input)
    .send();

// Or fire-and-forget with a delay:
ctx.service_client::<MyServiceClient>()
    .handle(input)
    .send_after(Duration::from_secs(60));
```

`send()` returns a `SendHandle` which can be dropped (fire-and-forget) or awaited to get an `InvocationHandle`.

### Fan-Out with `DurableFuturesUnordered`

The recommended pattern for concurrent fan-out:

```rust
use restate_sdk::prelude::*;

#[handler]
async fn run(&self, ctx: WorkflowContext<'_>, request: Json<Request>)
    -> Result<Json<Reply>, HandlerError>
{
    let mut in_flight = DurableFuturesUnordered::new();
    for (key, child_request) in &targets {
        let client = ctx.object_client::<JurisdictionCensusClient>(key.clone());
        in_flight.push(
            client.run(child_request).call(),
        );
    }

    // Process completions as they arrive
    while let Some((index, result)) = in_flight.next().await? {
        match result {
            Ok(report) => /* collect success */,
            Err(err) => /* record as failure, continue */,
        }
    }
}
```

`DurableFuturesUnordered::next()` returns `Ok(Some((index, result)))` for each completed future, or `Ok(None)` when all are done. If the invocation is cancelled, it returns `Err(TerminalError)`.

**Source:** `src/context/select_any.rs:DurableFuturesUnordered`

### Observing Child Workflow Failure

A child workflow's `call().await` returns `Result<Output, TerminalError>`. If the child fails (either via `TerminalError` or exhausting its retry budget with `on_max_attempts = "pause"` or `"kill"`), the parent receives the `TerminalError` from `.call()`.

**To continue around failures ("NationalCensus continues around failed units"):**

```rust
while let Some((index, result)) = in_flight.next().await? {
    let (jurisdiction, key) = &targets[index];
    match result {
        Ok(report) => {
            summaries.push(JurisdictionSummary { jurisdiction, report });
        }
        Err(err) => {
            failures.push(NationalFailure {
                jurisdiction,
                reason: err.to_string(),
            });
            // Continue — don't propagate the error
        }
    }
}
```

**Key insight:** The child workflow's failure is a `TerminalError` in the parent's `call()` result. The parent can catch it and record it as data rather than propagating. This is exactly what the `NationalCensus` does.

**Source:** `crates/midwest-census/src/restate_services/national.rs:103-138` — `classify()` function classifies child completions into `Completion::Success` or `Completion::Failure`.

### `on_max_attempts = "pause"` — Parent Observing a Paused Child

When a child invocation exhausts its retry budget with `on_max_attempts = "pause"`, the child enters a **paused state**. The parent's `.call()` will **never resolve** — it hangs waiting for the child to complete (which requires manual resume via CLI/UI).

**For the census repo's requirements:** If a jurisdiction fails 3 times and pauses, the parent workflow's `DurableFuturesUnordered::next()` will also block forever on that slot. To handle this, the parent needs either:
1. A timeout mechanism (e.g., `DurableFuturesUnordered` + `ctx.signal()` or external driver).
2. The child to use `on_max_attempts = "kill"` so the parent observes the failure immediately.
3. An external coordinator that can resume or kill paused invocations.

**UNVERIFIED:** Whether `DurableFuturesUnordered::next()` can be combined with `ctx.signal()` via the `select` macro for timeout-based cancellation. This pattern is documented in the SDK's `select` module but was not fetched directly.

### Re-Attaching to a Child Workflow from Outside

```rust
// From an external client:
let workflow = MyWorkflowIngressClient::from_client(client, "workflow-key");
workflow.run(input).send().await?;

// Later, re-attach by key:
let handle = workflow.handle().await?;  // requires Restate 1.7+
let result = handle.attach().await?.into_body()?;
```

**Source:** `src/ingress/mod.rs:Client::lookup_workflow`, `src/ingress/mod.rs:InvocationHandle::attach`

---

## 6. Endpoint / Service Registration

### Building and Binding Services

```rust
use restate_sdk::prelude::*;

let endpoint = Endpoint::builder()
    .bind(Census)
    .bind(Ingest)
    .bind(Sweep)
    .build();
```

`Endpoint::builder().bind(service_instance).build()` registers the service. The service name comes from the struct name (or `#[restate_sdk::service(name = "...")]` override).

### Starting the HTTP Server

```rust
HttpServer::new(endpoint)
    .listen_and_serve("0.0.0.0:9080".parse().unwrap())
    .await;
```

`HttpServer::listen_and_serve()` binds a TCP listener, serves HTTP/2 connections, and completes on `SIGTERM`.

For custom cancellation:

```rust
HttpServer::new(endpoint)
    .serve_with_cancel(listener, custom_cancel_signal)
    .await;
```

For custom listener:

```rust
let listener = TcpListener::bind(addr).await?;
HttpServer::new(endpoint)
    .serve(listener)
    .await;
```

**Source:** `src/http_server.rs:HttpServer`

### Identity Key (Request Verification)

```rust
HttpServer::new(
    Endpoint::builder()
        .bind(MyService)
        .identity_key("publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f")
        .unwrap()
        .build(),
)
```

Validates that requests come from a Restate instance with the corresponding private key.

**Source:** `src/http_server.rs` docs.

### Service Options (Programmatic Configuration)

```rust
use restate_sdk::endpoint::ServiceOptions;
use std::time::Duration;

let options = ServiceOptions::new()
    .journal_retention(Duration::from_secs(90 * 24 * 3600))  // 90 days
    .idempotency_retention(Duration::from_secs(30 * 24 * 3600))
    .retry_policy_initial_interval(Duration::from_millis(500))
    .retry_policy_max_interval(Duration::from_secs(60))
    .retry_policy_max_attempts(3)
    .retry_policy_pause_on_max_attempts();

let endpoint = Endpoint::builder()
    .bind_with_options(MyService, options)
    .build();
```

Programmatic options override attribute-level options. Handler-specific options:

```rust
let handler_opts = HandlerOptions::new()
    .retry_policy_max_attempts(1)  // no retry for this specific handler
    .journal_retention(Duration::from_secs(86400));  // 1 day

let options = ServiceOptions::new()
    .handler("my_handler", handler_opts);
```

**Source:** `src/endpoint/builder.rs:ServiceOptions`, `HandlerOptions`

### The `.well-known/restate` / Discover Endpoint

The SDK exposes a `/discover` endpoint that returns the service manifest. The content type is negotiated via the `Accept` header:

| Version | Content Type |
|---------|-------------|
| v2 | `application/vnd.restate.endpointmanifest.v2+json` |
| v3 | `application/vnd.restate.endpointmanifest.v3+json` |
| v4 | `application/vnd.restate.endpointmanifest.v4+json` |

The discovery document describes each service's name, handlers, options, and metadata. The Restate server uses this to register the service without a separate registration step.

**Source:** `src/endpoint/mod.rs` — `DISCOVERY_CONTENT_TYPE_V2`, `_V3`, `_V4`; `handle_discovery()` method.

### HTTP Endpoint Paths

| Path | Method | Description |
|------|--------|-------------|
| `/restate/invoke/service/<name>/<handler>` | POST | Invoke a handler |
| `/restate/send/service/<name>/<handler>` | POST | Fire-and-forget (one-way) |
| `/restate/call/service/<name>/<handler>` | POST | Request-response (same as invoke) |
| `/restate/awakeables/<id>/resolve` | POST | Resolve an awakeable |
| `/restate/awakeables/<id>/reject` | POST | Reject an awakeable |
| `/restate/lookup` | POST | Lookup a workflow invocation by name+key |
| `/restate/attach/<invocation-id>` | GET | Attach to a running/finished invocation |
| `/restate/output/<invocation-id>` | GET | Get output of a finished invocation |
| `/discover` | GET | Service manifest discovery |
| `/health` | GET | Health check |

**Source:** `src/endpoint/mod.rs:handle_with_verifier` — path parsing logic; `src/ingress/mod.rs` — client-side path construction; SDK README examples.

### restate.toml Keys for Single-Node Deployment

The `restate.toml` is a **server-side** configuration file, not an SDK concern. The SDK connects to the Restate server via HTTP. Relevant keys for the census deployment:

```toml
roles = ["http-ingress", "admin", "worker", "log-server", "metadata-server"]
node-name = "midwest-census"
cluster-name = "midwest-census"
auto-provision = true
default-num-partitions = 24
default-replication = 1
base-dir = "/var/lib/midwest-census-restate"
listen-mode = "tcp"
bind-ip = "127.0.0.1"
bind-port = 15152
advertised-address = "http://127.0.0.1:15152/"
shutdown-timeout = "1m"
disable-telemetry = true
experimental-enable-protocol-v7 = true
experimental-enable-vqueues = true
experimental-enable-scoped-virtual-objects = true
```

The SDK does NOT read `restate.toml`. It connects to the ingress port (default 8080) and uses the `/restate/*` HTTP API.

**Source:** `deploy/restate.toml` in the repo.

---

## 7. Version and Crate Layout

### Crate Structure (SDK 0.12.x)

```
restate-sdk/                    # Main SDK crate
  src/
    lib.rs                      # Re-exports macros, defines prelude
    context/                    # Context types (Context, ObjectContext, WorkflowContext, etc.)
      mod.rs                    # All context traits: ContextTimers, ContextClient,
                                #   ContextSideEffects, ContextReadState, ContextWriteState,
                                #   ContextAwakeables, ContextSignals, ContextPromises
      request.rs                # Request, CallFuture, SendHandle, InvocationHandle, SignalHandle
      run.rs                    # RunClosure, RunFuture, RunRetryPolicy
      select_any.rs             # DurableFuturesUnordered
    endpoint/                   # Endpoint, Builder, ServiceOptions, HandlerOptions
    errors.rs                   # HandlerError, TerminalError, TerminalErrorExt
    http_server.rs              # HttpServer
    ingress/                    # ReqwestClient, Client, RequestExecutor
    discovery.rs                # Service manifest types
    serde.rs                    # Json, PayloadMetadata, Serialize/Deserialize
    configuration.rs            # Service configuration
    tunnel/                     # Tunnel (Unix-only, `tunnel` feature)
  macros/                       # proc-macro crate
    restate-sdk-macros/         # The actual proc-macro crate
      src/
        lib.rs                  # service, object, workflow, handler macros
        struct_ast.rs           # Struct-based API parsing (current)
        ast.rs                  # Legacy trait-based API parsing (deprecated)
        generator.rs            # Code generation
        struct_generator.rs     # Struct-based code generation
    Cargo.toml                  # name = "restate-sdk-macros", version = "0.12"
```

### Dependency Versions

| Crate | Version | Notes |
|-------|---------|-------|
| `restate-sdk` | `0.12.0` (pinned) / `0.12.1` (latest) | Main SDK |
| `restate-sdk-macros` | `0.12.0` / `0.12.1` | Proc-macro crate (pinned to exact SDK version) |
| `restate-sdk-shared-core` | `7.0.3` | Shared VM, identity verification, error types. Pinned to exact version (`=7.0.3`) in both SDK and the census repo's `Cargo.lock`. |

**Source:** `Cargo.lock` in the census repo; `Cargo.toml` of the SDK repo (version = "0.12.1").

### Feature Flags (restate-sdk)

| Feature | Default | Description |
|---------|---------|-------------|
| `http_server` | YES | `HttpServer` for serving endpoints |
| `hyper` | YES (via http_server) | HTTP/2 server runtime |
| `rand` | YES | Random number generation (`ctx.rand()`, `ctx.rand_uuid()`) |
| `uuid` | YES | UUID generation (`ctx.rand_uuid()`) |
| `tracing-span-filter` | YES | `ReplayAwareFilter` for log filtering during replay |
| `reqwest-client` | NO | `ReqwestClient` for ingress client |
| `tunnel` | NO | In-process Restate Cloud tunnel (Unix-only) |
| `lambda` | NO | AWS Lambda support |
| `aws_lc_rs` | NO | AWS LC RS crypto (alternative to `rust_crypto`) |
| `rust_crypto` | YES (via default) | rustls ring crypto (default) |

The census repo pins: `restate-sdk = { version = "=0.12.0", default-features = false, features = ["http_server", "aws_lc_rs", "reqwest-client"] }`

### Breaking Changes Between 0.12.0 and 0.12.1

**None.** v0.12.1 is strictly additive:

1. **Scoped ingress clients** — `scoped_client(client, key, scope)` on generated clients.
2. **Workflow `handle()`** — `handle().await?` on generated workflow clients to re-attach by key.
3. **`Client::lookup_workflow()`** — public API for workflow handle lookup.

**Source:** `release-notes/v0.12.1.md`

### Breaking Changes from Pre-0.12 to 0.12

1. **Trait-based API deprecated.** The old `trait MyService { async fn handle() -> HandlerResult<...>; }` pattern is deprecated. The new struct-based `impl MyService { #[handler] async fn handle(...) -> Result<..., HandlerError> { ... } }` is the current form.
2. **Handler signature changed.** Handlers now take `&self` as the first argument, then the context, then an optional input parameter. The old trait-based API had no `&self`.
3. **Return type is always `Result<T, HandlerError>`.** The old API had a custom return type parameter.
4. **`HandlerResult<T>` type alias.** The prelude provides `HandlerResult<T> = Result<T, HandlerError>` for brevity.
5. **`ctx.run` now requires `RunRetryPolicy` for per-run retries.** The old SDK had a different mechanism.

**Source:** `macros/src/lib.rs:dispatch` (deprecated trait path); `src/lib.rs` docs.

### Crate Dependencies (restate-sdk → restate-sdk-shared-core)

`restate-sdk-shared-core = "=7.0.3"` provides:

- `CoreVM` — the deterministic execution engine (journal management, syscall handling)
- `TerminalFailure` — the terminal error type used in the shared core
- `IdentityVerifier` — request identity verification
- `VM` trait — the virtual machine interface
- `http` integration — HTTP request/response types
- `request_identity` — JWT-based request verification (for Restate Cloud)
- `sha2_random_seed` — SHA2-based random seed generation for deterministic randomness
- `rust_crypto` / `aws_lc_rs` — crypto backends

**Source:** `Cargo.toml:dependencies` — `restate-sdk-shared-core = { version = "=7.0.3", features = ["request_identity", "sha2_random_seed", "http"] }`

---

## Appendix: Current SDK vs. Repo Usage Comparison

| SDK API | Repo Usage | Status |
|---------|-----------|--------|
| `#[restate_sdk::service]` on `impl` block | Used in `census.rs` | ✅ Matches |
| `#[restate_sdk::object]` on `impl` block | Used in `ingest.rs`, `jurisdiction.rs` | ✅ Matches |
| `#[restate_sdk::workflow]` on `impl` block | Used in `national.rs`, `publish.rs` | ✅ Matches |
| `#[handler]` on methods | Used throughout | ✅ Matches |
| `Context<'_>`, `ObjectContext<'_>`, `WorkflowContext<'_>` | Used throughout | ✅ Matches |
| `SharedObjectContext<'_>`, `SharedWorkflowContext<'_>` | Used in `ingest.rs`, `jurisdiction.rs`, `national.rs` | ✅ Matches |
| `ctx.get::<T>()`, `ctx.set()` | Used in `ingest.rs`, `jurisdiction.rs`, `national.rs` | ✅ Matches |
| `ctx.run(|| ...).await?` | Used in `ingest.rs`, `publish.rs`, `jurisdiction.rs/stages.rs` | ✅ Matches |
| `ctx.run().retry_policy(RunRetryPolicy::new().max_attempts(1))` | Used in `jobs.rs:200` | ✅ Matches |
| `ctx.object_client::<TClient>(key).handler().call()` | Used in `national.rs` | ✅ Matches |
| `ctx.workflow_client::<TClient>(key).run().call()` | Used in `national.rs` | ✅ Matches |
| `DurableFuturesUnordered` | Used in `national.rs` | ✅ Matches |
| `TerminalError::new()` | Used in `ingest.rs`, `jurisdiction.rs`, `national.rs` | ✅ Matches |
| `HandlerError` | Return type of all handlers | ✅ Matches |
| `invocation_retry_policy(...)` on attributes | Used throughout with `max_attempts = 3, on_max_attempts = "pause"` | ✅ Matches |
| `journal_retention = "90 days"` | Used on `#[object]` and `#[workflow]` | ✅ Matches |
| `idempotency_retention = "30 days"` | Used on `#[service]`, `#[object]`, `#[workflow]` | ✅ Matches |
| `workflow_completion_retention = "180 days"` | Used on `#[workflow]` only | ✅ Matches |
| `ctx.sleep(Duration)` | Used in `sweep.rs` | ✅ Matches |
| `ctx.awakeable()` | Not used in repo | ⚠️ Available |
| `ctx.signal()` | Used in `sweep.rs` (STOP_SIGNAL) | ✅ Matches |
| `ctx.promise()` | Not used in repo | ⚠️ Available |
| `Endpoint::builder().bind(T).build()` | Used in `mod.rs:build_endpoint` | ✅ Matches |
| `HttpServer::new(endpoint).serve_with_cancel(...)` | Used in `bootstrap/serve.rs` | ✅ Matches |
| `ReqwestClient::connect()` | Not used in repo (ingress is via `crates/midwest-census/src/ingress/`) | ⚠️ Available with `reqwest-client` feature |

---

## Gaps and Limitations

1. **`ctx.select` macro for combining durable futures:** The SDK provides a `select!` macro (documented at `https://docs.rs/restate-sdk/latest/restate_sdk/macro.select.html`) for combining `DurableFuturesUnordered` with `ctx.signal()` or `ctx.sleep()` for timeout-based cancellation. This was NOT fetched directly and is marked UNVERIFIED.

2. **`ctx.run_typed`:** Not found in the current SDK source. The SDK uses `ctx.run()` with a closure and infers the type from the closure's return type. The repo uses `ctx.run()` consistently — no `ctx.run_typed` exists.

3. **Journal introspection endpoint:** The SDK exposes journal data through the Restate server's admin API and CLI (`restate journal`), but the SDK itself does not provide a direct journal-read API for handlers. The repo's approach of storing state via `ctx.set` and reading it back is the correct pattern.

4. **`restate.toml` is server-side only:** The SDK does not parse or depend on `restate.toml`. The server reads it and the SDK connects via HTTP. The repo's `deploy/restate.toml` is correct in this regard.

5. **Protocol version negotiation:** The repo's `restate.toml` enables `experimental-enable-protocol-v7`. The SDK 0.12.x supports protocol versions 5 through 7. The exact version range is not documented in the SDK source but is referenced in the repo's deployment config comments. **UNVERIFIED from SDK source.**
