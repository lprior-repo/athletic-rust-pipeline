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
