# ADR-003 — Graduation year is the cohort identity

Status: accepted.

## Context

“JR”, “11”, “SR” and “12” describe grade relative to an academic year. Treating those tokens as a
permanent cohort changes meaning at season boundaries and can create false contradictions.

## Decision

Use graduation year as cohort identity. Preserve each grade observation with its academic-year and
source context. Grade 11 in 2025–2026 and grade 12 in 2026–2027 both support Class of 2027; a grade
without the necessary season context cannot establish that conclusion.

A directly published graduation year is a distinct `PublishedGraduation` observation, with its
exact publisher `SourceRef`; it is not reverse-calculated into a grade in the collector's chosen
season. `CanonicalAthlete.published_graduations` retains these claims separately from
`observed_grades`. Either matching typed evidence form supports HIGH cohort confidence; any
disagreeing published year or unsupported/contradicting grade observation yields LOW. An empty
typed evidence set remains unknown, even when the constructor year or an opaque evidence note
mentions 2027. The claims' source references resolve through the retained capture evidence and
locators, not an inferred roster/person join.

Historical persisted athletes missing `published_graduations` decode to an empty vector, and empty
vectors are omitted on serialization. Existing notes are not promoted into validated claims during
deserialization or indexing. Fresh qualified projection emits the typed claims; explicit historical
source-bound correction/migration remains separate. Canonical merges preserve distinct source
claims and contradictions, and the full athlete evidence digest fences stale identity applications.

Retain contradictory observations rather than choosing the convenient one. Discovery is allowed
without cohort proof; verified and unresolved cohort outcomes remain separate. Cohort agreement is
not proof that two source subjects are the same person.

Inference is fallible within the constrained graduation-year range. A legal grade and school year
whose inferred graduation year falls outside that range remain raw source evidence and a located
pending review case; they do not mint a canonical cohort or become an input-triggered panic.
When a canonical subject already has the same primary provider namespace and ID, frozen snapshot
reads include those raw grade observations. Advisory aliases and names do not establish ownership;
ambiguous primary owners retain contradictions without an identity merge. Ingestion does not
backpatch canonical rows based on the store's current contents.

## Consequences

Adapters preserve raw tokens and context; deterministic domain rules interpret them. Neither the
current clock nor a source query filter may silently establish graduation evidence. The current
`GradYear`, `Grade`, `SchoolYear`, `ObservedGrade` and `PublishedGraduation` are implemented in `census-domain`;
[Architecture §9](../../ARCHITECTURE.md#9-target-data-and-publication-contracts) owns the required
semantics, without a second illustrative wire schema here.
