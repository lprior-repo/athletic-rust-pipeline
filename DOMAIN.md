# DOMAIN.md — types, identities and evidence rules

The census answers one question for every athlete: *who is this, what did they run, and how do we
know?* The types in `crates/midwest-census/src/model.rs` exist to make the wrong answer hard to
express. This document is the contract; the code is the implementation.

## 1. Identity is minted, not borrowed

`Id<T>` is a locally minted, deterministic id for a canonical entity: `Id<CanonicalAthlete>`,
`Id<CanonicalSchool>`, `Id<CanonicalMeet>`, `Id<CanonicalTeam>`, `Id<CanonicalCoach>`. Provider ids
never become canonical ids. A provider id lives in `SourceIdentity { namespace, source_id, url }`
where `SourceNamespace` names the provider (`MileSplit`, `Wiaa`, `Mshsl`, `AthleticNet`, …).

Consequences:

- One athlete with four provider profiles is one canonical athlete with four `SourceIdentity` rows.
- Deleting or re-importing a provider never renumbers canonical entities.
- Two providers that disagree stay disagreeing and visible; the merge does not average them.

## 2. Cohort is a graduation year; grade is time-scoped evidence

```rust
struct GradYear(i16);          // permanent cohort identity: Class of 2027 == GradYear(2027)
struct SchoolYear(i16);        // 2025 means the 2025-2026 academic year
struct Grade(u8);              // 9..=12, validated at construction
struct ObservedGrade { grade: Grade, school_year: SchoolYear, source: SourceRef }
```

`Junior`, `JR`, `11` and `SR`/`12` are *query parameters and label fragments*, not facts. They are
interpreted with the season they were observed in and recorded as `ObservedGrade`. An athlete
observed as grade 11 in 2025-2026 and grade 12 in 2026-2027 is the same Class-of-2027 athlete by
construction, and a source that only ever says "Senior" contributes an observation, never a cohort.

## 3. Evidence, not summary

```rust
enum EvidenceMethod { … }                 // how the fact was obtained (page, api, pdf, roster …)
struct Evidence { method: EvidenceMethod, source: SourceRef, … }
struct SourceRef { namespace: SourceNamespace, url: String, observed_on: … }
struct Confidence(u8);                    // bounded, explicit, never stringly typed
```

Everything durable is an observation with a source and a method. Readers merge; writers append. A
value without an `Evidence`/`SourceRef` is a bug: it cannot be audited later, and the whole product
promise is auditability.

## 4. Sport, event and mark

`Sport` distinguishes cross country from track; `CompetitionLevel` distinguishes high-school,
post-season and club; `EventKind` canonicalises known event families (sprints, distances, hurdles,
relays, jumps, throws, pole vault, combined events) while `SourceEventLabel` preserves the provider's
own literal text, including unusual variants. `Mark` carries the performance value together with its
comparability inputs (`TimingMethod`, wind, indoor/outdoor, implement and hurdle specification where
known). `TimingMethod` records whether a mark was FAT, hand-timed, or converted.

Never compare incomparable marks, and never flatten a variant into a neighbour to make an event
column tidy: an indoor 55 m dash is not a 60 m dash, and a wind-aided 10.74 is not a legal 10.74.

## 5. PRs

A PR is computed in Rust from performances that are comparable under §4: an event-specific ordering
over `Mark`, restricted to provably comparable performances, taking the best. Source-reported PRs
(from a provider profile page) are stored *alongside* the computed PR, not instead of it, so a
disagreement is a visible, auditable fact. AI never decides which of two marks is better.

## 6. Transfer and school history

A performance carries the school the athlete represented when it was run. When an athlete transfers,
new observations carry the new school; earlier observations keep the old one. `CanonicalAthlete`
therefore points at a school history, not a single mutable school field, and "current school" is a
derived, dated fact rather than an overwritten string.

## 7. Coaches and contacts

```rust
struct CanonicalCoach { … }
enum CoachRole { … }        // head track, head cross country, assistant, athletic director, …
```

Only publicly published professional contact information for a school/sport role is collected, with
its source URL and observed date. Athlete personal email, personal phone, home address and other
unrelated personal data are outside the contract: if a directory exposes them, they are not ingested.

## 8. Failure is a first-class value

`OperationTerminal<T>` distinguishes `Complete`, `NotFound`, `Incomplete`, `RateLimited`,
`HumanRequired`, `RetryExhausted`, `SourceUnavailable`, `PolicyBlocked`. Collapsing `RateLimited` or
`SourceUnavailable` into "not found" is forbidden: it silently corrupts coverage reporting, which is
the artifact the whole census is judged on. Where an outcome must cross an async boundary, keep the
cause: panic (`BrowserError::TaskPanicked`), cancellation and timeout are distinct from domain errors.
