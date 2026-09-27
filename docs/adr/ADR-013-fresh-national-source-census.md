# ADR-013 — Fresh source-discovered census, not workbook enrichment

Status: accepted owner correction. Geography amended on 2026-09-27: the proposed 51-jurisdiction
expansion was withdrawn; [ADR-009](ADR-009-census-run-scope.md) remains authoritative at 49.

## Context

Enriching a supplied recruit list can finish without discovering the national population. Historical
corpus size and old seals also cannot certify a newly acquired census. The required product is
source discovery, not workbook intake or a relabeled prior run.

## Decision

Discover the Class-of-2027 high-school TF/XC population from qualified public sources across the 48
contiguous states plus D.C. Label subset runs as qualification, never national completion.

Do not import an admissions/seed workbook, operator recruit list or previous canonical corpus.
Excel is generated output. Publicly published result spreadsheets are captured source documents,
like PDFs; they are not an operator-supplied population denominator.

Use a fresh named store bound to an unused durable run namespace/revision. Do not import old
observations, canonical identities, decisions, completion receipts, coverage totals or seals. A new
directory with old Restate keys is not fresh: those keys can replay old completed work.

Reuse maintained Rust code, parser fixtures and qualified public entry points, then acquire this
run's evidence afresh. Within the run, reuse immutable captures and completed effects across
subjects and recovery. Preserve historical stores, workbooks, fixtures and unique evidence.

## Supersession and consequences

- Supersedes [ADR-010](ADR-010-preserve-existing-corpus.md)'s old-corpus prerequisite for this fresh
  delivery; explicit schema migration/preservation obligations for historical stores remain.
- Supersedes workbook-driven population and original-input-row accounting in old plans.
- Does **not** supersede ADR-009, authorize data deletion, reinstate the deleted root pipeline,
  permit concurrent owning processes for a store, or waive evidence-backed completion.

[The national delivery plan](../NATIONAL-CENSUS-PLAN.md) owns implementation order, F01–F15, named
regressions and all 17 fault scenarios. Its requirements are not executed evidence.
