# ADR-011 — A census is sealed, not declared

Status: accepted. Existing seal code is not proof that the fresh-run acceptance contract is met.

## Context

An operator-controlled “complete” label cannot distinguish a finished census from partial output.
Completion must carry evidence and explain refusal; it cannot rely on filenames or non-empty tables.

## Decision

Use an explicit phase progression through discovery, acquisition, reconciliation, review, gap
resolution and export. Completion requires seal evidence, not a boolean or a skipped phase.

Every required jurisdiction/source obligation, cohort decision and identity candidate has a terminal
declared outcome. Retain terminal gaps, conflicts, access restrictions and exhausted failures inside
the evidence; those findings are not successful acquisition and must not disappear to obtain a seal.
Open work, missing durable evidence, unreproducible calculations or unverified output blocks sealing.
An unmeasured open-work count is unknown, not zero.

Read the published artifacts and independently reconcile exact record identities, cohort population,
PR winners, coverage and metrics against their frozen input generation. Counts alone cannot prove
row-level correctness. The seal binds the run, immutable inputs, verified artifact digests and
retained findings. Reapplying identical evidence returns the same seal; time alone cannot create a
new completion claim.

## Consequences

A seal certifies the declared scope and disclosed limits, not Internet-wide exhaustive knowledge.
[ADR-013](ADR-013-fresh-national-source-census.md) prevents historical seals from certifying the fresh
run. [OPERATIONS.md](../OPERATIONS.md) owns current seal commands and limitations;
[the delivery plan](../NATIONAL-CENSUS-PLAN.md) owns missing oracle/integration work, including
refusal for every unmet acceptance item. Implementation heuristics are not this decision's proof.
