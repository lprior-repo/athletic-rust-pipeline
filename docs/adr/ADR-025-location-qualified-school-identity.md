# ADR-025 — School identity is location-qualified; location disagreement is a retained conflict

**Status:** Accepted (2026-10-06); implemented under `tq18`, gate green 2026-10-06, review and landing pending.

## Context

`CanonicalSchool::mint` derived its id from exactly two parts —
`Id::mint("sch", &[state.code(), compressed(normalized_name)])`
([`crates/census-domain/src/model/school.rs`](../../crates/census-domain/src/model/school.rs)) — and
`NaturalKey for CanonicalSchool::same_natural_key` compared the same two fields
([`natural_key.rs`](../../crates/census-domain/src/model/natural_key.rs)). The entity merge guard in
[`crates/census-store/src/entities/canonical.rs`](../../crates/census-store/src/entities/canonical.rs)
records a `RetainedConflict` only when two rows that share an id disagree on their natural key; with
the id keyed by the same fields as the natural key, that guard was unreachable for schools. Two
distinct programs that publish one name in one state therefore collapsed into a single canonical
school with no conflict recorded: the first row's `city` won, and later observations' aliases,
`source_identities`, `evidence` and `postal_addresses` were unioned onto it. Once collapsed, the
address join (ADR-021/ADR-023) has one school to match against the corpus, so a postal claim can be
attached to the school of the wrong city — the corruption the reported defect describes.

## Decision

1. **City participates in the minted id when the observing row publishes one.** `mint`/`new` take
   `city: Option<&str>`; a non-empty city adds a third id part, its ASCII-alphanumeric lowercase
   form (`city_key`). Empty or absent city keeps the two-part id. Minting stays pure, deterministic
   and independent of arrival order.
2. **The natural key compares location compatibly.** Same state and same compressed name are
   required; when both rows publish a city they must agree on `city_key`, and a row that publishes
   no city does not contradict one that does. Two rows can share a city-qualified id only when their
   cities agreed, or both were absent.
3. **Disagreement on a shared id is a retained conflict, never a blend.** `Entity::merge` keeps its
   existing guard; with the key change it now fires for schools whose rows were minted without a
   city and later carry different ones (legacy rows, readback, direct entity use). The dropped row's
   facts are not unioned onto the kept school, and the conflict names both natural keys and sources.
4. **A school observed without a city keeps the two-part id and stays its own unresolved identity.**
   It is never collapsed into a located school by name; a later located observation is a distinct
   canonical school. Reconciling those two ids — aliases, campuses, co-ops and readback agreement —
   is the open school-identity mapping task, not this decision.
5. **No historical-store migration.** The census acquires each run afresh (ADR-013) and historical
   stores, exports and seals are preserved without import (ADR-010). Ids minted before this decision
   remain readable as opaque strings; the publication verifier compares ids read from the store, not
   re-derived ones. Fresh runs are the acceptance surface.

## Consequences

- Two same-name schools in different cities are two canonical schools and both publish their own
  postal claims; the join can match each to its own corpus entry instead of tying on one row.
- Two observations of one school that publish the same city still merge into one school, and a
  second observation carrying the city for a school first seen without it produces the unresolved
  pair described in §4 rather than silently promoting its city onto the located school.
- City spelling variation (`St. Louis` vs `Saint Louis`) produces two ids. That is visible by
  construction (two school rows) and is the reconciliation task's input, not a silent merge.
- `city_key` is the identity discriminator; the school's `city` field still preserves the source's
  spelling, and the workbook publishes it unchanged.
- Adapter call sites pass the city they already carry at construction; the post-construction
  `school.city = …` assignment that merely repeated it is removed.

## Evidence

`cargo test -p census-domain --lib model_tests::general_model` (city and state separate identity,
same city agrees), `cargo test -p census-store --lib entities` (conflicting cities on one id retain a
conflict; compatible cities merge), and the school-address join's same-name two-city lane. Dated
command results for the landing are recorded in
[VERIFICATION-EVIDENCE.md](../VERIFICATION-EVIDENCE.md).
