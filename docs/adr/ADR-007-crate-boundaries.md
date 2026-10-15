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

## Browser ownership amendment — 2026-10-15

The October 14 JSON/runtime separation is superseded for current physical acquisition. An RPC
disconnect can leave the remote browser job running after its caller releases resource reservations.

Use one in-process bounded actor shared by acquisition and browser controls. Before enqueue,
transfer the real prepared token, armed dispatch guard and same-grant reservations. The actor's
issued slot retains them outside the job future until response transfer or completed physical
cleanup and reaping, including panic and cancellation. Move response bytes; do not serialize live
capabilities or add a second budget.

Allow `athleticnet-browser` → Store resource-ownership types and `census-crawl` → browser actor.
No Store → browser dependency. Remove the superseded physical RPC bridge and migrate every caller;
retain historical bytes without executing a fallback. This is an ownership requirement, not DTO
deduplication or a new acquisition engine.

Status: accepted target amendment; integration and runtime qualification remain open. Acceptance
requires real dispatch-before-network, same-grant lifetime and cancel/panic/drain evidence; source
inspection or a generic panic-result test is insufficient.
