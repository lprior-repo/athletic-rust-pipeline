# ADR-002 — Restate owns retries

Status: accepted.

## Context

Independent retry loops multiply physical traffic: three attempts at each of three layers can
produce 27 requests for one logical operation. Transport cannot own durable operation identity,
remaining budget and replay safety independently of the orchestrator.

## Decision

Restate is the sole automatic retry owner, with at most **three total attempts** per failed external
operation. HTTP, browser, adapter and model transports make one attempt per call. A journaled
physical action must not add an independent retry budget.

Retries retain logical operation identity and budget across restart. Exhaustion is retained as an
explicit failure; it is never evidence of athlete absence or `NO_MATCH`. Independent source work
continues. Human-required access conditions remain human-required, not an invitation to evade them.

## Consequences

No convenience retry wrapper, unbounded delay loop or new workflow key may reset the ceiling.
Retry classification and Retry-After handling remain explicit transport outcomes; scheduling is
Restate's responsibility. A completed `ctx.run` result is replayable, but an unfinished blocking job
has no automatic internal checkpoint.

[RESTATE_WORKFLOWS.md](../../RESTATE_WORKFLOWS.md) owns implemented handler policies, including pause
versus terminal-failure dispositions. [The fault catalog](../NATIONAL-CENSUS-FAULTS.md) owns required
restart, exhaustion and physical-request evidence; this ADR is not an execution certificate.
