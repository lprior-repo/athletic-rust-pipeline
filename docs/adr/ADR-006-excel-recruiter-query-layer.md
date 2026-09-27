# ADR-006 — Excel is the recruiter query layer

Status: accepted; shared-generation publication remains a delivery obligation, not assumed complete.

## Context

Recruiters filter a workbook; the pipeline must retain observations, source identity and conflicts.
Making Excel the evidence store or building a second interactive database would confuse those roles.

## Decision

Publish `Athletes`, `PRs`, partitioned `Performances_*`, `Coaches`, `Schools`, `Meets`, `Sources`,
`Coverage`, `Conflicts`, `Review` and `Run Metrics` from the durable census. Athlete rows must
separate accepted identities from retained review cases. PR rows are per athlete and **compatible
comparison key**, not merely display event name. Partition performance sheets at Excel's row limit.

All output formats share one input generation and semantic derivation. Preserve exact record IDs,
source references, coverage limits and historical affiliation. Independently read back and reconcile
the artifacts before publication and sealing. Verify semantic records and PR winners; byte-identical
XLSX files alone do not establish correctness.

Core evidence scope is an independence diagnostic excluding Athletic.net and its AthleticLIVE
derivative, not the default product population. Its filtered evidence must not be confused with
geographic or cohort eligibility.

## Consequences

The workbook and sidecars form one verified atomic bundle; a stale or failed exporter cannot replace
a valid published generation. [Architecture §9](../../ARCHITECTURE.md#9-target-data-and-publication-contracts)
owns the publication contract. [ADR-013](ADR-013-fresh-national-source-census.md) makes Excel output
only; public result spreadsheets remain eligible captured sources.
