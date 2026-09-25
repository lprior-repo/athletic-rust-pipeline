# ADR-002: Restate owns retries; the transport performs one attempt

## Status

Implemented, in both runtimes. The root crate's transport owned the *classification* of a failure and
no budget (`src/runtime/source/retry.rs` (historical: root package deleted 2026-09-23): "no attempt budget, no delay ladder, no sleep inside the
invocation"), and the census fetcher attempts once for the same reason
(`crates/census-crawl/src/net/execute/attempt.rs`: "the transport attempts exactly once, as the ADR's
Decision states"). Restate owns every later attempt: each durable handler declares `max_attempts = 3`
with `on_max_attempts = "pause"`
(`crates/census-service/src/restate_services/{census,national,publish,sweep,ingest,jurisdiction}.rs`),
a journaled physical act runs under `RunRetryPolicy::max_attempts(1)` (`.../jobs.rs`), and the browser
session declares one attempt (`.../browser_session.rs`). `.../retry_policy_tests.rs` scans the tree's
own text and fails any ceiling outside the contract. The `OperationTerminal` type this ADR once cited
never existed in the tree; the vocabulary is per layer (`DOMAIN.md` §8).

## Context

A retry is where a pipeline quietly multiplies its own traffic. Three layers that each retry three
times turn one logical operation into 27 requests:

```text
Restate retries 3 x HTTP helper retries 3 x adapter retries 3 = 27 physical requests
```

The origin cannot tell which layer caused the multiplication, and the census's most valuable sources
are exactly the hosts that block an operator for it. `ARCHITECTURE.md` §9 ("Retry exhaustion is
evidence… A source failure is never equivalent to `NO_MATCH`") and `RESTATE_WORKFLOWS.md:511-512`
("Do not add a second convention beside an existing one — another store, queue or retry loop. Restate
is the only scheduler here") already state the rule.

## Decision

* **One retry owner per operation.** Restate — a workflow, or an SDK policy a handler declares —
  decides whether an attempt happens again, and holds the durable attempt count and terminal outcome.
* **The transport performs one attempt.** No hidden loop in the HTTP helper, no adapter-level retry
  wrapper: one call in, one outcome out.
* **A retry reuses the logical identity** of the operation, so a replay after a crash reads the
  journaled value instead of manufacturing new work.
* **Budgets are explicit and small**, and exhaustion is a recorded outcome, not an exception:
  `FailureCode::RetryExhausted` (historical: root package deleted 2026-09-23; `src/runtime/source/result/classification.rs:69`) on an
  `OperationFailure` carrying its attempt receipts (historical: `src/runtime/protocol.rs`).

* **A source failure is never `NO_MATCH`.** `Decision::CompleteSearchNoMatch` is reachable only when
  discovery is `SearchCompleteness::Complete` (historical: `src/domain/decision/pipeline.rs:100-104`; restated in
  historical: `src/domain/decision/review.rs:19-20`, pinned by historical: `src/domain/decision/tests.rs:206`).

## Consequences

* Physical requests are auditable: `RetryEvidence` records who owned the retries, the budget and the
  observed attempts (historical: `src/runtime/protocol.rs:92-115`), and an independent verifier re-reads that from
  the store instead of trusting the caller (historical: `src/result_verify/source_receipts.rs:154-165`).
* Budgets are typed and fail closed: `RetryCount::new(4)` is refused
  (historical: `src/domain/facts/facts_contract_tests.rs:41`) and the counter stops at three
  (historical: `src/domain/facts/scalars.rs:98`; historical: `src/domain/error.rs:16`).
* Adapter authors must not add convenience retries "just for this endpoint" — the traffic budget
  depends on that discipline.

## Alternatives considered

* **Retry in the transport or adapter.** Rejected: that layer cannot see the logical operation, so it
  cannot judge whether a retry is safe, and it compounds with the owner's retries — the 27× case.
* **No retries; every failure pages an operator.** Rejected: transient store locks, session desyncs and
  rate limits are what a journaled retry repairs
  (`crates/census-service/src/restate_services/support.rs:16-70`).
* **Unbounded exponential retry.** Rejected: the invocation retry caps automatic attempts at three and
  a blocked service pauses for an operator instead of replaying (`max_attempts = 3` /
  `on_max_attempts = "pause"` in `crates/census-service/src/restate_services/`), and the transport
  owns no budget of its own to extend (`src/runtime/source/retry.rs`, historical: root package deleted 2026-09-23).

## Evidence — what exists, what does not

Implemented, root runtime (historical: root package deleted 2026-09-23): `src/runtime/source.rs:3-5` ("one journaled attempt in `attempt`");

`src/runtime/source/attempt.rs` (historical: root package deleted 2026-09-23; `run_step` = one browser fetch inside `ctx.run`);
historical: `src/runtime/source/retry.rs` (the transport keeps the classification, not a budget: "no attempt
budget, no delay ladder, no sleep inside the invocation"); historical: `src/runtime/http_audit.rs:71,84` (recorded
budget `RetryCount::new(3)`).

Implemented, census crates: `crates/census-crawl/src/net/execute/attempt.rs` (`fetch_once` = one
attempt); `crates/census-crawl/src/net/execute/browser/refusal.rs` (a retryable refusal "leaves as a
retryable error for the durable layer to replay"); handler budgets in
`crates/census-service/src/restate_services/` (`max_attempts = 3, on_max_attempts = "pause"`);
physical acts (`.../jobs.rs`: `RunRetryPolicy::new().max_attempts(1)`); the browser session
(`.../browser_session.rs`: `max_attempts = 1, on_max_attempts = "pause"`); and the scan that holds
every ceiling to the contract (`.../retry_policy_tests.rs`).

Resolved since the first draft of this section: the `MAX_RETRIES = 3` transport loop and the
`retry_loop` that let the census fetcher own retries lived in the pre-migration
`crates/census-service/src/net/**`; that module is `crates/census-crawl/src/net/**` now and contains
neither. `ARCHITECTURE.md` §9 names the per-layer vocabularies instead of asserting an
`OperationTerminal` type. What remains open is bounded and different: the root runtime walked up to 64
admission steps per source operation (`crates/census-crawl/src/net/bridge/lane.rs`,
`crates/census-crawl/src/net/bridge/mod.rs:39`) (historical: root package deleted 2026-09-23), a loop the rankings path refused — 64 × the invocation budget is
reachable and unmeasured there.
