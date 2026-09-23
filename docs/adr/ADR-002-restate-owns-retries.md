# ADR-002: Restate owns retries; the transport performs one attempt

## Status

Partially implemented. The root runtime implements the decision in the `SourceGateway` workflow
(`src/runtime/source/`); the census fetcher does **not** obey it yet, retrying inside the transport up
to `MAX_RETRIES = 3` (`crates/census-service/src/net/mod.rs:57`). The `OperationTerminal` vocabulary
quoted by `ARCHITECTURE.md:58-61` has no Rust definition in the tree.

## Context

A retry is where a pipeline quietly multiplies its own traffic. Three layers that each retry three
times turn one logical operation into 27 requests:

```text
Restate retries 3 x HTTP helper retries 3 x adapter retries 3 = 27 physical requests
```

The origin cannot tell which layer caused the multiplication, and the census's most valuable sources
are exactly the hosts that block an operator for it. `ARCHITECTURE.md:59-61` ("Failure is recorded as
evidence, never as absence") and `RESTATE_WORKFLOWS.md:371-373` ("Do not add a second convention
beside an existing one — another store, queue or retry loop. Restate is the only scheduler here")
already state the rule.

## Decision

* **One retry owner per operation.** Restate — a workflow, or an SDK policy a handler declares —
  decides whether an attempt happens again, and holds the durable attempt count and terminal outcome.
* **The transport performs one attempt.** No hidden loop in the HTTP helper, no adapter-level retry
  wrapper: one call in, one outcome out.
* **A retry reuses the logical identity** of the operation, so a replay after a crash reads the
  journaled value instead of manufacturing new work.
* **Budgets are explicit and small**, and exhaustion is a recorded outcome, not an exception:
  `FailureCode::RetryExhausted` (`src/runtime/protocol.rs:88`) on an `OperationFailure` carrying its
  attempt receipts (`:122-127`).
* **A source failure is never `NO_MATCH`.** `Decision::CompleteSearchNoMatch` is reachable only when
  discovery is `SearchCompleteness::Complete` (`src/domain/decision/pipeline.rs:100-104`; restated in
  `src/domain/decision/review.rs:19-20`, pinned by `src/domain/decision/tests.rs:206`).

## Consequences

* Physical requests are auditable: `RetryEvidence` records who owned the retries, the budget and the
  observed attempts (`src/runtime/protocol.rs:92-115`), and an independent verifier re-reads that from
  the store instead of trusting the caller (`src/result_verify/source_receipts.rs:154-165`).
* Budgets are typed and fail closed: `RetryCount::new(4)` is refused
  (`src/domain/facts/facts_contract_tests.rs:49`) and the counter stops at three
  (`src/domain/facts/scalars.rs:97-98`; `src/domain/error.rs:15-16`).
* Adapter authors must not add convenience retries "just for this endpoint" — the traffic budget
  depends on that discipline.

## Alternatives considered

* **Retry in the transport or adapter.** Rejected: that layer cannot see the logical operation, so it
  cannot judge whether a retry is safe, and it compounds with the owner's retries — the 27× case.
* **No retries; every failure pages an operator.** Rejected: transient store locks, session desyncs and
  rate limits are what a journaled retry repairs
  (`crates/census-service/src/restate_services/support.rs:16-70`).
* **Unbounded exponential retry.** Rejected: `MAX_RETRY_DELAY = 86_400 s` and `MAX_ATTEMPTS = 4` bound
  the workflow (`src/runtime/source/retry.rs:6-7`), and a blocked service pauses for an operator
  (`RESTATE_WORKFLOWS.md:209-211`).

## Evidence — what exists, what does not

Implemented, root runtime: `src/runtime/source.rs:3-5` ("one journaled attempt in `attempt`");
`src/runtime/source/attempt.rs:3-5` (`run_step` = one browser fetch inside `ctx.run`);
`src/runtime/source/workflow.rs:3,79,155-166` (`execute` walks at most `retry::MAX_ATTEMPTS`, and a
step past the budget is final even when classified retryable); `RESTATE_WORKFLOWS.md:317-318`
(physical acts use `max_attempts(1)`: "a failed act is not silently re-issued by the SDK — the handler
decides"); `src/runtime/browser_session.rs:35,79-80` (service policy 4-and-pause; status observed
with one attempt); `src/runtime/http_audit.rs:71,84,96,109` (recorded budget `RetryCount::new(3)`).

Conflicting or missing: `net/mod.rs:57` (`MAX_RETRIES = 3`) and `net/execute/attempt.rs:41-83`
(`retry_loop`) let the census fetcher own retries; no further retry exists under
`src/sources/**`/`src/cli/**`, and that path is not Restate-driven — 27× does not occur there, but the
one-attempt rule is unmet. `src/runtime/source/dispatch.rs:118` iterates `0..64` admission steps per
source operation (`RESTATE_WORKFLOWS.md:284`) while the rankings path refuses that loop
(`dispatch.rs:59-60`): 64 × 4 is reachable and unmeasured. `ARCHITECTURE.md:58-61` asserts
`OperationTerminal`; no such type exists — the real vocabularies are `RowResolution`
(`src/runtime/row_protocol.rs:106-113`), `Decision` (`src/domain/decision.rs:33-39`) and `FailureCode`
(`src/runtime/protocol.rs:87-89`).
