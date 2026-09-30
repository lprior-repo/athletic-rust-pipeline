# Architecture decision register

Each decision has one authoritative record. This index contains no second copy of its requirements.
[ARCHITECTURE.md](../../ARCHITECTURE.md) owns binding product/engineering contracts;
[the delivery plan](../NATIONAL-CENSUS-PLAN.md) owns unfinished implementation and acceptance.
Accepted policy is not proof of implementation or successful execution.

| ADR | Decision | Status |
|---|---|---|
| [001](ADR-001-fjall-primary-store.md) | Fjall is the system of record | Accepted |
| [002](ADR-002-restate-owns-retries.md) | Restate alone owns retries; three total attempts | Accepted |
| [003](ADR-003-graduation-year-cohort-identity.md) | Graduation year, not timeless grade | Accepted |
| [004](ADR-004-meet-first-ingestion.md) | Reuse shared meets/rosters before repeated profile fetches | Accepted strategy |
| [005](ADR-005-ai-cannot-override-contradictions.md) | Local AI advises; Rust adjudicates | Accepted |
| [006](ADR-006-excel-recruiter-query-layer.md) | Excel is the recruiter projection | Accepted |
| [007](ADR-007-crate-boundaries.md) | Explicit workspace crate boundaries | Accepted; extraction complete |
| [008](ADR-008-rust-only-tooling.md) | Rust-only pipeline/tooling logic | Accepted |
| [009](ADR-009-census-run-scope.md) | 48 contiguous states plus D.C. = 49 jurisdictions | Accepted; reaffirmed 2026-09-27 |
| [010](ADR-010-preserve-existing-corpus.md) | Preserve historical evidence; explicitly migrate affected stores | Fresh-run import prerequisite superseded by 013 |
| [011](ADR-011-census-seal.md) | Evidence-backed completion, not a declared flag | Accepted; verification obligations remain |
| [012](ADR-012-single-acquisition-plane.md) | One crawl/browser acquisition plane | Accepted; root pipeline removed |
| [013](ADR-013-fresh-national-source-census.md) | Fresh public-source census, no seed workbook or old corpus | Accepted; proposed 51-jurisdiction expansion withdrawn |
| [014](ADR-014-athlete-owner-identity-optional.md) | Preserve historical absent source ownership | Accepted persisted-shape contract |
| [015](ADR-015-port-prototype-acquisition-to-rust.md) | Port prototype acquisition to Rust; captured parity fixtures are the evidence | Accepted |
| [016](ADR-016-merge-and-identity-semantics.md) | Row hygiene ports to a shared helper; identity stays lossless, census scope filters are explicit | Accepted |
| [017](ADR-017-domain-canonical-json.md) | The domain owns its canonical JSON encoder; digests stay byte-equal with serde_json | Accepted |
| [018](ADR-018-identity-and-evidence-framing-escapes.md) | Identity and evidence framing escapes its delimiter bytes | Accepted |
| [019](ADR-019-export-derivation-single-home.md) | Export consumers share one dataset and derivation | Accepted; renumbered from duplicate 015 |
| [020](ADR-020-school-address-corpus-port.md) | The address pipeline and the TSSAA reader port to Rust as a school-directory corpus | Accepted; ADR-008's admitted-reader amendment resolved |

Changes record explicit supersession rather than silently rewriting prior decisions. Source-specific
research and dated command results belong in their evidence references, not additional ADR copies.
