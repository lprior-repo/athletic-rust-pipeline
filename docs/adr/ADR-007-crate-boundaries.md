# ADR-007 — Explicit workspace crate boundaries

Status: accepted; the original extraction sequence is complete, not a second active plan.

## Context and decision

Separate pure domain semantics from storage, acquisition, deterministic reconciliation, model
advice, projection and service composition. [Architecture §4](../../ARCHITECTURE.md#4-crate-layout-17)
is the canonical responsibility table. `athleticnet-browser` owns the browser effect boundary;
`xtask` owns developer verbs, not business rules.

The extraction proceeded from domain contracts through store/crawl, reconciliation/review/report,
then service integration. Changes must still be coherent, gated slices with every caller migrated.
Do not preserve obsolete root paths through aliases or re-exports, introduce mirrored domain types,
or create a competing CLI/deployment workflow.

## Consequences

Main owns public contracts and final integration; narrow adapters own source differences.
[ADR-012](ADR-012-single-acquisition-plane.md) fixes the acquisition boundary. New crate extraction
requires an actual responsibility/consumer need, not a repeat of the superseded migration inventory.
The only unfinished delivery ordering lives in [the active plan](../NATIONAL-CENSUS-PLAN.md).

## Boundary clarification — 2026-10-14

The browser bridge deliberately separates the JSON consumer contract (`census-crawl` wire DTOs)
from browser runtime outcomes (`athleticnet-browser`). HTTP status/header objects and owned body
bytes are not the consumer's integer status, header pairs and base64 representation. Neither crate
imports the other's runtime outcome types; shared wire fixtures verify encoding/decoding parity.
This is an effect-boundary representation, not a second domain model or acquisition engine.

Do not extract a shared crate merely to remove these DTOs. Change the wire contract coherently at
both endpoints and retain unknown-variant/field refusal. Qualification remains execution evidence,
not this architectural decision.
