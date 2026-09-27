# ADR-013 — Fresh nationwide source census, not workbook enrichment

Status: accepted owner correction; the geography clause is withdrawn (owner direction, 2026-09-27),
the fresh-run program and national execution remain required.

## Decision

Build the Class-of-2027 high-school Track & Field / Cross Country census from scratch through
qualified public sources. The population is discovered, not supplied by an admissions workbook,
a recruit list, an existing export, or the previous canonical corpus.

The census scope is ADR-009's: the 48 contiguous states plus D.C. — 49
jurisdictions. Owner direction on 2026-09-27 withdrew this ADR's 51-jurisdiction target: Alaska,
Hawaii and every territory stay outside the census, and no coverage, seal or workbook denominator
counts them. `UsJurisdiction` still models all 51 states so their identifiers parse and are refused
by scope instead of silently widening coverage.

Subset runs are explicitly labeled qualification runs, never the delivered national census.

Recruiter workbooks are generated outputs. Do not implement workbook intake, admissions matching,
source-row reconciliation against a private workbook, or an input-workbook coverage denominator.
Publicly published result spreadsheets remain eligible source documents, like public PDFs; they
are acquired with provenance and do not supply an operator-selected target population.

Start the delivered census in a fresh, separately identified store and durable run namespace or
revision. Do not import old observations, canonical entities, decisions, completion receipts,
coverage counts or seals into it. Reuse maintained Rust code, parser fixtures and qualified public
source entry points, not inherited claims of completed acquisition. Within the new run, immutable
captures and completed effects must be reused across subjects and retries.

Preserve existing stores, workbooks, fixtures and historical evidence. Fresh acquisition is not
authorization to delete user data or run two writers against one Fjall database.

## Superseded decisions

- ADR-009's 48-contiguous-states-plus-D.C. run restriction is superseded by the 51-jurisdiction target.
- ADR-010's requirement to populate the delivered census by migrating the old corpus is superseded
  for this fresh national run. Explicit schema migrations remain necessary when persisted formats
  change; they do not authorize importing the old census as the new population.
- Workbook-driven delivery requirements in prior plans are superseded. Excel remains a projection
  under ADR-006; the existing crate ownership and single acquisition plane of ADR-012 remain.

## Rationale

A workbook-enrichment pipeline searches a supplied list and can finish without discovering the
national population. That is the wrong product. National completeness must be measured against
jurisdictions, qualified source objects, programs, seasons, discovery and acquisition obligations,
not a spreadsheet's rows. Historical corpus size and old seals cannot certify a new acquisition.

## Scope

Owner direction on 2026-09-27 withdrew this ADR's 51-jurisdiction target. The run scope stays
ADR-009's: the 48 contiguous states plus D.C. `UsJurisdiction::CENSUS_SCOPE` holds those 49 and
`EXCLUDED_FROM_CENSUS` names Alaska and Hawaii, which parse and are refused by scope. The
fresh-run program below is unaffected: a fresh store, unused durable run identities and an
explicit revision are still required, and no historical corpus or seal certifies a new census.

A fresh directory alone is insufficient: existing Restate keys can replay old completed work.
Bind the new store and run revision explicitly, and prove that old completions cannot seed it.

The detailed target, dependency order, F01–F15 acceptance and all 17 fault scenarios are in
[the national census master plan](../NATIONAL-CENSUS-PLAN.md). It is an implementation contract,
not execution evidence. Current-state descriptions must retain named gaps until verified.
