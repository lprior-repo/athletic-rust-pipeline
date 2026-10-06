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
| [021](ADR-021-school-address-join-durable-stage.md) | The school-address join is one tested core, one durable stage | Accepted |
| [022](ADR-022-admissible-cross-source-corroboration.md) | Admissible cross-source identity corroboration: attested provider keys, independent documents, withholding reasons | Accepted; implemented 2026-10-04 (`46r`); link-document source qualification open under `b0f` |
| [023](ADR-023-school-link-mapping.md) | School-link extension beyond the landed join core: provider/state-record keys, durable review cases, campus and co-op rules | Accepted; implemented and closed 2026-10-04 (`7vk`); no source publishes `co_op` or member aliases yet |
| [024](ADR-024-coach-contact-tenure-admission.md) | Coach-contact tenure from a source-published staff listing: one claim binds role/program to mailbox; current tenures are the run's season; page-bound evidence | Accepted 2026-10-04; implementation open under `2b1`, `0hx`, `df2`, `jb2` |
| [025](ADR-025-location-qualified-school-identity.md) | School identity is location-qualified: city participates in the minted id and natural key; location disagreement is a retained conflict, never a blend | Accepted 2026-10-06; implemented under `tq18`, gate green, review and landing pending |
| [026](ADR-026-derived-generations-and-store-schema.md) | Derived state is published by generation with one atomic pointer flip; the store names its schema and refuses what it cannot interpret; decisions fence on the evidence generation | Accepted 2026-10-06 |

Changes record explicit supersession rather than silently rewriting prior decisions. Source-specific
research and dated command results belong in their evidence references, not additional ADR copies.
