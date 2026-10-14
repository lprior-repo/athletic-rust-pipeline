# ADR-009 — Census run scope is 49 jurisdictions

Status: accepted. Reaffirmed by owner correction on 2026-09-27.

## Context and decision

The delivery covers the **48 contiguous states plus D.C.**, not all modeled jurisdictions.
`UsJurisdiction::ALL` contains the 50 states plus D.C.; `UsJurisdiction::CENSUS_SCOPE` contains the
49 jurisdictions required for this run. Alaska and Hawaii remain valid parsed locations but do not
satisfy or enlarge its denominator. Territories are not modeled.

Keep unknown/unplaced location explicit. A subset such as Wisconsin is qualification, not national
completion. A broad source-research survey may inspect excluded jurisdictions, but must identify
its own universe rather than silently changing census coverage.

## Consequences

Discovery, coverage, verification and seal requirements share the same declared run scope. Every
required jurisdiction must have an honest terminal status; an empty or unresearched applicability
list is not completion. [ADR-013](ADR-013-fresh-national-source-census.md) changes population intake
and freshness, not this scope: its proposed expansion to 51 was withdrawn on 2026-09-27.

## Forward profile scope clarification (2026-10-14)

[ADR-038](ADR-038-selected-profile-reusable-production-state.md) keeps this decision as the exact
49-jurisdiction denominator for the historical fresh census and the distinct future national
profile. It is not a hardcoded denominator for every selected production query. The first
production profile is Class of 2027 in MI, MN, IA, IL and CO with source-backed UP priority;
UP-only qualification and five-state delivery cannot certify national completion. Explicit
50-state-plus-D.C. configuration is a separate forward profile requiring qualified execution,
not a restoration of the withdrawn ADR-013 expansion or a reinterpretation of historical scope.
Unknown geography stays unknown; territories still need a separate model extension.
