# ADR-003: GraduationYear is the cohort identity; grade is time-scoped evidence

## Status

Accepted. Implemented: `Grade`, `GradYear`, `ObservedGrade`, `SchoolYear` in
`crates/midwest-census/src/model.rs`; grade observations carry evidence in the adapter layer.

## Context

Every source states cohort membership differently, and almost all of them state it as a *grade
at a moment in time*: "JR", "Junior", "11", "Grade 11", "senior", "Class of 2027". Grade alone
is ambiguous the moment a season boundary or a source's own academic-year convention differs.
A source that says "11" in May 2026 and a source that says "12" in September 2026 may be
describing the same person; a naive model would treat them as a contradiction, or worse, as two
different cohorts.

The census's durable target is a recruiting cohort that outlives the observation:
**GraduationYear(2027)**.

## Decision

* Cohort identity is `GradYear` (graduation year), never a grade label.
* Grade is represented as an observation bound to the academic year in which it was stated:

  ```rust
  struct ObservedGrade {
      grade: Grade,
      school_year: SchoolYear,
      evidence: Evidence,
  }
  ```

* No raw source token (`JR`, `SR`, `11`, `12`, `class-of`) becomes a canonical cohort fact
  without season-aware interpretation. The adapter's job is to preserve the raw token and the
  academic year; the domain's job is to map it.
* Both `2025-2026 -> Grade 11` and `2026-2027 -> Grade 12` are evidence for the same
  `GradYear(2027)`, and both are retained; disagreement is surfaced as a conflict, not resolved
  by discarding the loser.
* Graduation evidence strength is a first-class, reportable property: an athlete can be
  discovered without cohort proof, and the workbook must show which athletes are
  Class-of-2027 *verified* versus merely *presumed*.

## Consequences

* Adapters cannot answer "is this athlete Class of 2027?" from a single field; they emit
  observations, and the cohort decision is a deterministic, auditable function of them.
* The workbook can state coverage honestly: `Class of 2027 verified` vs `unresolved` are
  separate counts, and no run may claim a national denominator it cannot defend.
* Time-scoped evidence makes cross-season joins correct (XC in fall and TF in spring of the
  same school year belong to one cohort), which would be impossible with a bare grade integer.
