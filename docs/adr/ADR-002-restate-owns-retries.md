# ADR-002: Restate owns retries; transport performs exactly one attempt

## Status

Accepted. Implemented in the browser/scrape runtime: durable work objects in `src/restate.rs`
and `src/restate_services/`, single-attempt HTTP in `src/runtime/source/http.rs` and the
browser transport in `src/runtime/browser/**`.

## Context

A retry is the classic place where a pipeline quietly multiplies its own traffic. Layered
retries are the specific failure this project must avoid:

```text
Restate retries 3 x HTTP helper retries 3 x adapter retries 3 = 27 requests
```

Each of those requests is visible to the remote origin, and origins do not care which layer
caused the multiplication. Athletic.net and the state associations are the census's most
valuable sources; careless retry layering is what gets an operator blocked.

## Decision

There is exactly one retry owner:

* **Restate (or the calling workflow) decides whether an operation is attempted again.** It
  holds the durable state, the attempt count and the terminal outcome.
* **The transport performs one attempt.** No hidden loops, no backoff inside the HTTP helper,
  no adapter-level retry wrapper. One call in, one outcome out.
* A retried operation **reuses the same logical identity** (`meet:{source}:{id}:{revision}`,
  `athlete:{source}:{id}:{revision}`, `source-sweep:{source}:{state}:{season}:{revision}`); a
  failed HTTP call never manufactures a new logical job.
* Maximum automatic attempts per external operation is **3**. Exhaustion is not an exception:
  it becomes evidence (`OperationTerminal::RetryExhausted(..)`), and it is reported in coverage
  as an unresolved gap rather than as absence of data.

## Consequences

* The number of physical requests is auditable: attempts counted by the workflow equal attempts
  seen by the origin (modulo genuine transport-level reconnects, which are logged).
* A source failure can never be represented as `NO_MATCH`; the outcome lattice keeps
  `NotFound`, `RateLimited`, `HumanRequired`, `RetryExhausted`, `SourceUnavailable` and
  `PolicyBlocked` distinct.
* Adapter authors must not add convenience retries "just for this endpoint" — the correctness of
  the traffic budget depends on that discipline, and the source-adapter review gate should
  reject it.
* Idempotency is required downstream: because an attempt can be replayed after a crash, every
  observation write must be keyed by evidence digest, not by arrival order.
