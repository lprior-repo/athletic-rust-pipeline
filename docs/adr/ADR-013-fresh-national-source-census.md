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

## Forward production-lifecycle amendment (2026-10-14)

[ADR-038](ADR-038-selected-profile-reusable-production-state.md) supersedes this record's fresh-only
lifecycle restriction solely for explicitly enrolled and qualified reusable production. Such
production may reuse retained qualified public evidence through immutable selected-profile and
source/input revision contracts, with original acquisition dates and explicit freshness/coverage.
Its first selected delivery is Class of 2027 in MI, MN, IA, IL and CO with source-backed UP priority.

Every decision above remains binding for isolated fresh qualification and historical fresh national
claims: fresh named store and unused namespace/revision, no corpus or receipt/seal import and the
49-jurisdiction national denominator. Public-source discovery, no private/workbook denominator,
single store ownership and historical preservation remain unchanged in both lifecycles. Old stores,
manifests, keys, receipts, workbooks and seals keep their original meaning and are not automatically
enrolled or rewritten. Production adoption is not fresh-acquisition certification, migration
permission or runtime qualification; Main owns the separate versioned enrollment/integration.
