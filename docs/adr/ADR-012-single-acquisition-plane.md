# ADR-012 — One acquisition plane

Status: accepted; obsolete root pipeline removed on 2026-09-23.

## Context and decision

`census-crawl` owns source adapters, fetchers, admission and the browser bridge.
`athleticnet-browser` owns headed profile/CDP effects and browser failure classification.
`census-domain` remains pure. These are the maintained acquisition boundaries; do not introduce an
`acq-*` family, restore the root package, or retain competing acquisition/workflow implementations.

Sources differ in transport, format and supported evidence, not in identity authority, cohort rules,
mark normalization or output rules. Main owns shared contracts; adapters emit the common domain
observations. A future extraction requires demonstrated independent consumers and a new decision.

## Consequences

Migrate all callers during cutover and remove obsolete paths rather than adding shims. Enforce
purity and crate seams using the commands in [TESTING.md](../../TESTING.md). The current crate map
is [Architecture §4](../../ARCHITECTURE.md#4-crate-layout-17), not a historical module inventory.
