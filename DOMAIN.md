# Domain contracts and current representations

`census-domain` owns pure meaning; adapters preserve source facts, Rust adjudicates decisions and
reporting projects them. This document owns semantics, not physical table layout or workflow APIs.
See [FJALL_SCHEMA.md](FJALL_SCHEMA.md) and [RESTATE_WORKFLOWS.md](RESTATE_WORKFLOWS.md) for those.
Required behavior below is not a claim that every current public struct enforces it.

## Observation, evidence, candidate, decision

These are distinct stages:

1. **Raw observation:** what a captured source actually says, including unknown/unparseable fields.
2. **Validated claim:** interpreted value with its source locator, time and validation outcome.
3. **Candidate:** a possible identity relationship retrieved for deterministic evaluation.
4. **Accepted decision:** evidence-backed, policy-bound and reversible Rust adjudication.

A URL or model answer is not durable evidence. Material facts must resolve to retained capture bytes,
parser revision and a precise row/element/page locator. Preserve conflicting and rejected claims;
absence, failed acquisition, successful-empty acquisition and unfinished work are different states.

## Identity and source ownership

Provider identifiers are namespaced. Identical ID text at two providers does not identify one person;
syndicated copies do not provide independent corroboration. School/name/class/category is a useful
candidate bucket, not a unique person key. Distinct same-name students remain distinct; a transfer
may connect the same student across different school affiliations when corroborated.

Current representations include `CandidateKey` and `CanonicalAthlete`. The latter carries a stable
ID, canonical/known names, graduation and grade observations, a school, gender/sports, source links,
evidence and retained conflicts. Its single `school` field is **not** a complete affiliation history.
Source-owned subject minting adds provider ownership to the candidate bucket; it does not itself
prove cross-source equivalence or protect against every source's identifier reuse.

`CanonicalAthlete.source` and performance `source_athlete` may be absent for retained historical
shapes under [ADR-014](docs/adr/ADR-014-athlete-owner-identity-optional.md). Do not invent ownership to
fill that absence. Accepted aliases, candidate statuses and rejected relationships must remain
separate. Cluster changes must preserve public identity or publish an atomic alias; transitive
relationships cannot conceal an A–C contradiction merely because A–B and B–C were proposed.

[ADR-005](docs/adr/ADR-005-ai-cannot-override-contradictions.md) defines model authority. Acceptance
requires admissible corroboration; name-only, cohort-only, a score or model agreement is insufficient.
Bind evidence identity to structured, attributed facts. Reordering a genuinely unordered set may
leave its identity unchanged; swapping which subject owns a fact must change it.

## Cohort and time

[ADR-003](docs/adr/ADR-003-graduation-year-cohort-identity.md) makes graduation year authoritative.
Current values include `GradYear`, `Grade`, `SchoolYear` and `ObservedGrade`; the observation retains
its source. `SchoolYear` denotes the academic year's starting year, with the current containing-date
rule changing at August. For grades 9–12, graduation year is starting year plus `13 - grade`.

Thus grade 11 in academic year 2025 and grade 12 in academic year 2026 both support 2027. A free
“junior” token, query filter, current clock or default school year cannot establish cohort membership.
Preserve season context and contradicting observations. Distinguish discovered, cohort-unresolved,
cohort-accepted and cohort-excluded populations; a graduation conflict is not automatically a
source-person identity conflict.

## Meets, events, performances and marks

`CanonicalEvent` carries meet, kind, category, division, round, source labels and evidence.
`CanonicalPerformance` carries athlete, team, event/meet, date, mark, wind, place, heat/round, timing,
grade observation, source key/owner and provenance. Context is distributed across these records;
`Mark` alone does not establish comparability.

Meet and performance dates carry the source's published precision: a calendar day (`YYYY-MM-DD`) or,
when the source publishes only the year (the RaceDay archives do), the four-digit year.

Current `Mark` variants are time in centiseconds, distance in centimetres, imperial field marks
with metric representation, points and retained raw text. `TimingMethod` distinguishes FAT, hand
and unknown. These representations do not by themselves preserve every source's precision.
Required normalization preserves exact units, raw source precision and rejected/ambiguous values;
never infer a persisted writer's unit version from plausible magnitude. Explicit historical migration
belongs to the storage contract, not opportunistic parsing during fresh acquisition.

A legitimate performance identity includes relevant source ownership, event, meet, athlete/team,
round, heat and attempt. Deduplication unions provenance without merging distinct rounds or attempts.
A named relay member without an individual split has participation evidence, not an individual PR.

Best-mark reduction compares only compatible event and conditions: surface/venue context, distance,
implement/hurdle specification, timing, wind, measurement type and XC course/context where relevant.
Unknown is not a wildcard. The current `PrKey` includes athlete, event kind, surface, wind class,
timing, measure and optional context; all consumers must share its validated semantics. For fixed
compatibility/tie policy, reduction must be deterministic, idempotent, associative and commutative.
Retain source-declared PRs separately from best observed marks when coverage is incomplete.

## Affiliation and public coaching contacts

Affiliation is time-scoped evidence linking a person to a school/program/season. Use the result's team
and date for historical performance context, not the athlete's present school. Keep unresolved joins
visible; missing athlete/event/meet rows cannot silently delete evidence or count as athlete absence.

Public coaching contacts require current role, school/program, sport, side/category and source
context. Unknown sport/side is not “both”; former staff are not current. Export an email only when
the permitted source binds that exact mailbox to the eligible role. Never construct addresses from
patterns or collect athlete personal contact information. Distinguish no attempt, failed/blocked
attempt, successful-empty result and a resolved contact; retain redirects and evidence provenance.

## Scope and population accounting

Geographic eligibility is [ADR-009](docs/adr/ADR-009-census-run-scope.md)'s 49-jurisdiction run scope.
`All`/`Core` evidence scope is a separate dimension: Core excludes Athletic.net and its AthleticLIVE
derivative to expose independent evidence. It must filter the relevant source/grade/link evidence,
not merely relabel an all-source row. Neither evidence scope establishes cohort or identity acceptance.

Athletes with no performances remain in population accounting. Count unique accepted identities
separately from observations, proposed candidates and aliases; repeated source records are not more
athletes. Every obligation has an explicit success, terminal finding or unfinished outcome. Unknown
locations and unresearched applicability are reported separately from verified empty coverage.

## Remaining representation gaps

Public mutable fields, optional owners, scalar school affiliation and fixed-point precision mean the
current structs are not a proof that all target states are legal. The shared export derivation must
preserve these distinctions instead of letting each sheet recreate them. Exact row/PR/contact/coverage
reconciliation is defined in [the delivery plan](docs/NATIONAL-CENSUS-PLAN.md); its named canaries are
the acceptance oracle. Changes migrate all callers through the existing domain, not mirrored types.
