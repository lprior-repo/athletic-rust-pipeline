# ADR-014 — Persisted source-owner identity may be absent

Status: accepted historical-shape contract; does not authorize inventing provider identity.

## Context

The source-owned identity cutover (`99f88c9`) encountered historical athlete rows without provider
identity and performance rows written before `source_athlete` existed. Refusing those rows makes
retained evidence unreadable; synthesizing an owner fabricates evidence. Dated failures, counts and
recovery measurements are retained in [VERIFICATION-EVIDENCE.md](../VERIFICATION-EVIDENCE.md).

## Decision

`CanonicalAthlete.source` and `CanonicalPerformance.source_athlete` are optional in the persisted
representation. The athlete's checked/normal construction path takes a source owner; historical
absence remains `None`, not a fake identity.

The historical athlete `source_identities` array decodes to the first identity as `source` and the
remaining identities as `source_links`; an empty array yields no owner. A missing performance owner
also decodes as absent. Serialization uses the current `source`/`source_links` shape, skips absent
owners and does not write the historical key.

Keep this conversion at the domain deserialization boundary (`PersistedAthlete`), not as a second
store reader. A performance merge may fill missing ownership from actual incoming evidence;
absence itself is unknown, not a contradiction.

## Consequences

Consumers handle optional ownership explicitly and retain source provenance. Existing PR reduction
can use the athlete's known owner when the performance lacks it; this is not new corroboration or
proof that an unresolved subject is accepted. Do not infer that public optional fields enforce the
fresh-run acquisition invariant by themselves.

New schema changes require an explicit migration under [ADR-001](ADR-001-fjall-primary-store.md),
not more unversioned accepted shapes. [ADR-013](ADR-013-fresh-national-source-census.md) preserves old
stores without importing their population into the fresh census.
