# Primary next slice: publish the eligible 50, then fall back to 51

**Local-model assignment, not implemented.** Start from final publication `50b02cf25534acd25b5fcd79b3229ebebb66253a` (tree `c656422ca29b2032f0ee7db7249ea03fc8c94b0e`). Finish one executable end-to-end regression before expanding any authority framework. Resolve existing compile failures first; do not call source review acceptance.

## One target

Create a single controlled CO2027 athlete with three independently retained 400m results: 49.00, 50.00, 51.00 seconds. Use exact supported event/category/mark parsing and dates chosen from the real configured participation window. The raw 49 result is outside that window; 50 and51 are inside it. Keep all three original source rows, IDs, dates, schools and marks in the retained inventory.

G0 selects50, because49 has an actual source-backed `OutsideParticipationWindow` exclusion and51 is slower. Withdraw the sole reviewed attachment supporting50 through the real public attachment revoke operation, producing G1 selecting51. “Withdraw” here means withdrawing that documentary authority, not deleting the original result or inventing a performance-deletion API. Give50 its own source document/attachment so withdrawal cannot accidentally remove51. Do not leave independent alternative authority supporting50.

Current school comes from a separately reviewed current roster and may differ from the historical result school. Neither current school nor name equality supplies school-at-performance authority. All fixture captures/reviews must be explicitly synthetic; genuine May15 evidence must not be relabeled or edited to make this pass.

## Existing call chain to reuse

1. Create/open via `Store::create_public_identity_qualification_v3` and `Store::open_public_identity_qualification_v3`; perform actual graph enrollment and `PublicIdentityWriterV1::establish_source_bound`. Reuse the public enrollment patterns, not injected lifecycle metadata.
2. Feed compact original-shaped source bytes through existing parsers. Public Crawl recording is `record_provider_target_intake`, `record_retained_html_result_capture` (accepts a checked parsed document), and `record_independent_participation_packet`. Use recorded attachment-discovery flow, not direct Performance insertion as proof of ingestion. Preserve input hashes and original source span identities.
3. Public documentary attachment review uses `Store::inspect_evidence_subject_attachment_v1`, `prepare_evidence_subject_attachment_v1`, and `apply_evidence_subject_attachment_v1`. Reuse exact reviewed context construction from Crawl `recorded/tests/intake_reopen_acceptance/genuine_attachment/workflow/operations.rs`; do not synthesize a checked capability.
4. Separately obtain current roster authority through `record_provider_school_intake`, actual roster enrollment, `inspect_roster_source_issuance_v1` → `source_parts` → `assess_retained_roster_source_v2` / `assess_existing_roster_binding` → `prepare_roster_source_review_v1` → `apply_roster_source_review_v1`. Existing Report `source_authority/tests/issuance.rs` is the concrete complete example.
5. Capture via `ProfileSelectionV1::new` → Store snapshot → `ExportDataset::from_selected_snapshot` → `ensure_current_selected_write`. Call `verified_participation`, `verified_current_affiliations`, and `Derivation::selected`; assert the actual selected best, not a separately calculated minimum. Use existing production best-event selection reached by workbook derivation.
6. Export through `Censuses::of` → `workbook::build_from`. Save via `ExportDataset::save_frozen`. Verify with `workbook::publication::verify_published`, reopen via `ExportDataset::reopen_frozen_with_digest` and `workbook::publication::open_publication`. Existing publication/transfer.rs supplies the source-deletion/offline pattern, but its performance is currently Unresolved and cannot be copied as a positive eligibility oracle.
7. Withdraw50 via `AttachmentInspectionRequestV1::Revoke { prior }` and the same inspect/prepare/apply path. Report documentary `lifecycle/operation.rs::revoke_record` is the exact reusable example. Re-capture G1 with fresh selected authority, publish, then replay the old attachment command in `ReplayOnly` mode and prove no reactivation.

## Smallest missing production work

- Integrate the separately sealed parser school-span witness `fb2234` only after checking its exact seven paths against the final tree. It proves original statement ownership, not SchoolId correspondence.
- Complete only the source-owned historical school correspondence needed by this fixture: original root/row/span/digest plus checked installed school, date, sport/category/year and an explicit reviewed correspondence. Domain model exists; Store9 is an unregistered draft. Its startup and checked predecessor8 current/frozen continuation must be coherent before using it. No fake metadata, raw proof→authority constructor, or implicit school-name match.
- Wire that checked correspondence and existing documentary date/context facts into current participation assessment and its frozen proof path. The public school-correspondence issuance/admission call is presently missing; implement that narrow seam explicitly rather than pretending an existing enrollment API issues it. Unknown/withdrawn authority must withhold50 and leave independent51 usable.
- Preserve existing selector and export implementation. Change it only if the new regression exposes that qualified inputs are not consumed. No new service route, generic review framework, unrelated identity migration or full architecture rewrite is part of this slice.

## One integration test and completion condition

Proposed new Report test, under a reserved publication subtree: `controlled_three_marks_selects_eligible_best_then_withdraws_to_next_and_reopens`. This name is a target, not an existing runnable selector. Keep helpers bounded and use real public ingestion/review operations.

Required assertions in the same executed test:
- Physical inventory has exactly49/50/51 with original input IDs/context;49 is Excluded with exact source-backed window reason,50 and51 qualify initially. No accepted facts are test-injected after parsing.
- G0 selected PR is50 across dataset derivation, normalized output, recruiting output and actual workbook cell; current-school output independently matches current roster while historical school remains unchanged.
- Actual withdrawal receipt changes current authority; G1 selected PR is51 everywhere.49 remains excluded;50 remains retained and ineligible/unavailable for its precise withdrawn-authority reason. Exact old command replay returns the original receipt and cannot restore50.
- G0 bytes/digest and full publication inventory stay unchanged. Close/delete original Store and transient objects, reopen both generations independently, and recover50 in G0 and51 in G1 with identical reasons, references, public identity and current-school values.
- Missing/tampered school witness or removed required proof refuses or withholds, never silently promotes49/50. Use one focused negative mutation in this fixture; do not substitute a large unrelated unit-test matrix.

Acceptance requires exact final-source canonical fmt/check/lint and this full integration test executing successfully with nonzero test count, plus retained artifact inspection. Compilation, unit/model tests, constructing DTOs, an empty export, or a handwritten expected minimum do not complete the slice. If a needed public seam is absent, report that concrete function/contract blocker and keep the test visibly incomplete. Broader genuine-population, cross-build and service acceptance remain separate gates.
