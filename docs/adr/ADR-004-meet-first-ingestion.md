# ADR-004 — Shared meet results before repeated athlete fetches

Status: accepted strategy; source capabilities and measured coverage determine execution.

## Context

A whole-meet result document can serve many athletes with shared event, round, affiliation and mark
context. Repeating those acquisitions through individual profiles spends requests on duplicate
information. This is a structural cost argument, not a measured throughput claim.

## Decision

Prefer shared meet/result and roster acquisition. Deduplicate source work by qualified source-object,
capture and parser identity; reuse acquired evidence across applicable athletes. Prioritize valuable
meets and qualifiers, then fill missing history through rosters and linked profiles.

Profiles remain necessary when they add cohort evidence, source identifiers, TF/XC linkage, school
history or performances outside acquired meets. Meet-first does not authorize omitting athletes who
lack performances, assuming every source enumerates meets, or declaring coverage complete from a
single platform. Syndicated copies are not independent corroboration.

## Consequences

Adapters parse whole relevant documents and retain partial failures, rounds/heats/attempts and
coverage limits. Profile-declared PRs remain separate from calculated best observed marks.
Measure useful verified records per physical request; do not assert a numerical speedup from this
strategy alone. [SOURCE_ADAPTER_GUIDE.md](../../SOURCE_ADAPTER_GUIDE.md) owns adapter integration;
[source research](../../research/README.md) owns dated capability evidence.
