# DOMAIN.md — types, identities and evidence rules

The census answers one question for every athlete: *who is this, what did they run, and how do we
know?* The types in `crates/census-domain/src/model.rs` (plus `crates/census-domain/src/jurisdiction/` and
`src/error.rs`) exist to make the wrong answer hard to express. This document is the contract; the
code is the implementation.

## 1. Identity is minted, not borrowed

`Id<T>` is a locally minted, deterministic id for a canonical entity: `Id<CanonicalAthlete>`,
`Id<CanonicalSchool>`, `Id<CanonicalMeet>`, `Id<CanonicalTeam>`, `Id<CanonicalCoach>`. Provider ids
never become canonical ids. A provider id lives in `SourceIdentity { namespace, id, url }`
(`id` is the provider's own id, `url` optional) where `SourceNamespace` names the provider
(`MileSplit`, `Wiaa`, `Mshsl`, `AthleticNet`, …).

Consequences:

- One athlete with four provider profiles is one canonical athlete with four `SourceIdentity` rows.
- Deleting or re-importing a provider never renumbers canonical entities.
- Two providers that disagree stay disagreeing and visible; the merge does not average them.

A state is not a string either: `UsJurisdiction` (`crates/census-domain/src/jurisdiction/`)
declares the 50 states plus the District of Columbia. `UsJurisdiction::ALL` is the *modelled*
universe; `UsJurisdiction::CENSUS_SCOPE` — every one of them except Alaska and Hawaii — is what
every coverage denominator is decided over, so a jurisdiction in `ALL` but outside the scope is a
valid value that no run and no published row ever counts. Territories and freely associated states
are deliberately absent, so `"PR"` *fails to parse* instead of silently widening coverage; every
source id, journal phase, report row and workflow identity that needs a state formats it as the
USPS code (`Display` writes the code).

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

Only a school/sport role's published address is collected, with its source URL and observed date;
the address's domain decides whether it lands in the coach's `professional_email` or
`personal_email`, and only a malformed address is refused. Athlete personal email, personal
phone, home address and other unrelated personal data are outside the contract: if a directory
exposes them, they are not ingested.

## 8. Failure is a first-class value

There is no single `OperationTerminal<T>` type in this tree: earlier revisions of this document named
one, and no such type exists in the code. The vocabulary is per layer, and each layer names its own
causes:

- acquisition pipeline (deleted; see `ARCHITECTURE.md` §1): its per-layer vocabulary —
  `FailureCode` (`src/runtime/protocol.rs` — historical: root package deleted 2026-09-23), `InvalidInput`, `Transport`, `AccessDenied`,
  `BrowserChallenge`, `BrowserUnavailable`, `RateLimited`, `HttpFailure`, `PayloadLimit`,
  `ArtifactFailure`, `MalformedResponse`, `RetryExhausted`, `UncertainEffect` — went with the root
  package. What survives of it is `BrowserError` (`crates/athleticnet-browser/src/outcome.rs`),
  including `HumanRequired`, `Unavailable`, `TaskPanicked`, `Shutdown`; and `DomainError`
  (`crates/census-domain/src/error.rs`) for pure validation.
- census crate: `FetchError` (`crates/census-crawl/src/net/mod.rs`) for the transport
  (`Robots`, `Http{status}`, `RateLimited{retry_after_secs}`, `TooLarge`, `Transport`, `Cache`,
  `Timeout`, …) and `CrawlError` (`crates/census-crawl/src/lib.rs`) for the adapter
  layer.

Collapsing `RateLimited` or `SourceUnavailable`/`Unavailable` into "not found" is forbidden: it
silently corrupts coverage reporting, which is the artifact the whole census is judged on. Where an
outcome must cross an async boundary, keep the cause: `outcome::Outcome<T, E>`
(`crates/census-service/src/outcome.rs`) separates `Ok`/`Err` from `Cancelled`/`Timeout`/`Panicked`,
so panic and cancellation stay distinct from domain errors. Retry exhaustion is a Restate policy, not a domain value: invocations park with `on_max_attempts = pause` except `JurisdictionCensus`, which uses `on_max_attempts = kill` so `NationalCensus` folds a `NationalFailure` and continues — see `ARCHITECTURE.md` §5 (§9) and `RESTATE_WORKFLOWS.md` §7.4.
