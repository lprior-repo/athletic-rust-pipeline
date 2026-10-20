# ADR-006 — Excel is the recruiter query layer

Status: accepted; shared-generation publication remains a delivery obligation, not assumed complete.

## Context

Recruiters filter a workbook; the pipeline must retain observations, source identity and conflicts.
Making Excel the evidence store or building a second interactive database would confuse those roles.

## Decision

Publish `Athletes`, `PRs`, partitioned `Performances_*`, `Coaches`, `Schools`, `Meets`, `Sources`,
`Coverage`, `Conflicts`, `Review` and `Run Metrics` from the durable census. Athlete rows must
separate accepted identities from retained review cases. PR rows are per athlete and **compatible
comparison key**, not merely display event name. Every physical worksheet must stay within Excel's
1,048,576-row limit.

All output formats share one input generation and semantic derivation. Preserve exact record IDs,
source references, coverage limits and historical affiliation. Independently read back and reconcile
the artifacts before publication and sealing. Verify semantic records and PR winners; byte-identical
XLSX files alone do not establish correctness.

Stream at most 1,000,000 data rows plus the repeated header per page. Preserve the existing
`Performances_001` naming. Other logical families keep their base name on page one and use
`_002`, `_003`, etc. thereafter. Retain every ordered record; do not truncate to fit Excel.
Readback must reconcile all pages against one logical row budget and reject missing pages,
changed headers and extra records. Pagination does not change the frozen input or row semantics.

Normalized CSV readback streams up to512MiB per family. Decoded rows remain bounded
to1MiB, with exact expected record counts and generation/key checks. File growth
and oversized records still refuse; a larger file budget does not materialize it
in memory or authorize truncated verification.

Coach observation claims are grouped by school, then role, program and owner, and budgeted per owner
and per school before any claim or contact is adapted for a working sheet. A group over its budget is
refused whole with a typed error naming the group and the observed claim count; it is never truncated,
and rows reach the worksheet as a stream, so no sheet materialises its cells first.

Core evidence scope is an independence diagnostic excluding Athletic.net and its AthleticLIVE
derivative, not the default product population. Its filtered evidence must not be confused with
geographic or cohort eligibility.

## Consequences

The workbook and sidecars form one verified atomic bundle; a stale or failed exporter cannot replace
a valid published generation. [Architecture §9](../../ARCHITECTURE.md#9-target-data-and-publication-contracts)
owns the publication contract. [ADR-013](ADR-013-fresh-national-source-census.md) makes Excel output
only; public result spreadsheets remain eligible captured sources.
