# ADR-024 — Coach-contact tenure from a source-published staff listing

**Status:** Accepted (2026-10-04). Implementation open: `athletic-rust-pipeline-2b1` (directory
emission), `athletic-rust-pipeline-0hx` (claim binding), `athletic-rust-pipeline-df2` (research
outcomes), `athletic-rust-pipeline-jb2` (cell traceability).

[ARCHITECTURE.md](../../ARCHITECTURE.md) §9 owns the binding contact-publication contract, §8 owns
identity review. [ADR-011](ADR-011-census-seal.md) requires every accepted claim to resolve to
retained bytes, [ADR-022](ADR-022-admissible-cross-source-corroboration.md) decides *identity*
corroboration (not contact admission), and [ADR-006](ADR-006-excel-recruiter-query-layer.md) makes
the workbook the recruiter projection. This ADR decides what makes a source-listed coaching
appointment an admissible *current* contact claim.

## Context

The domain expresses tenure as `CoachTenure::{Current { school_year }, Former { last_school_year },
Unknown}` with page-bound `CoachTenureEvidence { tenure, source, source_sha256, retrieved_at,
statement }`; `assess_coach_tenure` filters claims to the queried school year, refuses
current/former conflicts, and `validate_tenure_evidence` requires a source id, a 64-hex SHA256, an
RFC3339 retrieval time and a bounded statement (`crates/census-domain/src/model/contact_tenure.rs`).
The report's contact selection (`crates/census-report/src/workbook/recruiting/contact/heads.rs`)
publishes a mailbox only for an owner holding an eligible current row.

Three gaps break that chain (adversarial assessment of `rh3`, 2026-10-04):

- The directory adapter emits no tenure evidence (`crates/census-crawl/src/coach_directories/staff.rs`
  builds coaches without it), so a source-listed current varsity head coach with a public mailbox
  still resolves unknown and publishes nothing. The captured directory payloads state no academic
  year; the 2026-27 season exists only in the census run's own scope.
- Nothing binds a mailbox to the role/program claim it is attributed to, so a person-level address
  can qualify a contact without the source asserting that role for it.
- Title classification read any title containing "head coach" as current, including "Former Head
  Coach", and `is_director` accepted "Former Athletic Director" (both fixed 2026-10-04).

## Decision

1. **A contact is one claim.** Publication requires a single claim binding (person, school, role,
   program) to a public mailbox carried by the same retained capture, plus `CoachTenure::Current`
   evidence for that same (person, school, role, program). A mailbox the source does not bind to
   that role and program never qualifies a contact.

2. **The current tenures are the run's season.** A staff listing that names a person in a role for a
   team is the source's own current-appointment statement; the emitted school year is the census
   run's season (`SchoolYear`), never a year inferred from a retrieval date. A school-scoped role
   that names no team (Athletic Director) carries the school's athletics as its program; the listing
   must still state the role. A listing that states a different season, names no role, names a coach
   role with no team, or carries no season scope emits `CoachTenure::Unknown` and publishes no
   contact.

3. **Evidence is page-bound.** Tenure evidence carries the source URL, the retained capture's
   SHA256, an RFC3339 `retrieved_at` and a bounded statement naming the role and program. Retrieval
   alone never makes an appointment current, and an absent, blocked or unparsed capture yields no
   evidence.

4. **Former and unstated roles never classify as current.** `coach_role`/`is_director` refuse any
   title stating a former role (implemented 2026-10-04). An unknown-role row may remain raw history
   but never supplies a contact.

5. **Research outcomes stay distinct.** Successful-empty, blocked or failed, and never-attempted
   research are distinct projected states; no state invents a negative finding, and a missing
   contact stays explicit.

6. **Contact cells stay auditable.** A published athlete contact cell, or its row provenance, names
   the selected coach identity, the source and the observation date used to select it.

7. **Alternatives rejected.**
   - *Require a source-stated academic year.* No directory payload in the corpus states one; this
     would make directory acquisition publication-dead while adding no evidence.
   - *Emit `Current` from the retrieval date alone.* A stale page would publish an unbound contact.
   - *Require two independent sources per contact.* ADR-022 governs identity corroboration; a
     contact claim is single-source-attributable and auditable through its retained capture.
   - *Treat silence as current.* Rejected; it is the F06 gap the plan already names.

## Consequences

- `census-crawl` `coach_directories` emissions (`staff.rs::build_coach`, `map.rs::coach_entities`)
  gain the run's `SchoolYear` and attach page-bound `CoachTenureEvidence` per emitted row, using the
  capture SHA256 and retrieval time the harvest already records (`2b1`).
- `census-report` contact selection requires the mailbox and the tenure evidence to share the claim
  (`0hx`); the contact-state projection gains distinct blocked/failed and never-attempted states
  (`df2`); athlete contact cells carry the selected coach, source and date (`jb2`).
- Existing domain behaviour is reused unchanged: year filtering, conflict refusal and provenance
  validation.
- Stale-listing risk is bounded: the listing must name the role and program, the run is
  season-scoped, and every published contact resolves to retained bytes a reviewer can re-read.
- Tests to land with the implementation: a listed current head coach with a public mailbox publishes
  exactly one contact; a listing that states no role emits unknown; a mailbox not bound to the role
  claim does not publish; the three research outcomes project distinctly.

## References

- [ARCHITECTURE.md](../../ARCHITECTURE.md) §9 (publication contracts), §8 (identity review).
- [ADR-006](ADR-006-excel-recruiter-query-layer.md), [ADR-011](ADR-011-census-seal.md),
  [ADR-022](ADR-022-admissible-cross-source-corroboration.md).
- `crates/census-domain/src/model/contact_tenure.rs`;
  `crates/census-report/src/workbook/recruiting/contact/`;
  `crates/census-crawl/src/coach_directories/`.
