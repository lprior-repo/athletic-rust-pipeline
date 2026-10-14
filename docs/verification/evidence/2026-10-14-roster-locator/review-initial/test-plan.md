# Positive roster locator regression plan

Status: specification and static review only; no build, test, parser runtime, network, browser, census, production or publication operation is run by this reviewer. Root owns integrated checks. This plan does not certify acceptance.

## Scope and baseline

The binding contract is `accepted-affiliation-bridge-review/IMPLEMENTATION-CONTRACT.md`. Production baseline is repository commit `3708d6bab0b08c390887c562a89e8c6714255a5d`, with exact production/test base files retained under `positive-roster-locator-proposal/base`. Documentation-only successor commits do not establish changed Rust behavior. The detached candidate is `positive-roster-locator-proposal/proposal`; Root separately integrates and formats it.

The change preserves a positive syntactic row's existing raw source coordinate with its unchanged owned payload. It does not create durable positive locators, a provenance ledger per canonical output, accepted affiliation, source authority, dated HS participation, or a new production lifecycle.

## Behavior inventory and scenarios

### LP01: exact positive bytes and payload

Given an input with a multibyte prefix, a non-ASCII name, and HTML entity syntax, when the actual public `parse_roster` runs, then one complete member has ordinal 0 and the exact byte offset/length of the literal inner `<li>` content. Checked offset plus length must safely slice the original UTF-8 input to the independently specified inner string. Assert all payload fields, including decoded name, provider ID, URL, published graduation year, gender and seasons. Character counts, normalized string lengths, outer-tag lengths and a second regex-based oracle are invalid substitutes.

Proposed test: `positive_roster_locator_preserves_exact_utf8_bytes_and_unchanged_payload`.

### LP02 and LP03: rejected gaps and duplicate occurrences

Given valid/rejected/identical-valid rows with additional multibyte bytes between rows, when parsing occurs, then positive ordinals remain 0 and 2, rejection ordinal remains 1, and the two equal payloads have distinct exact offsets. Assert rejected kind, retained provider ID and literal rejected span. Preserve two positive source occurrences; do not infer canonical dedup policy from this test.

Proposed test: `partial_roster_keeps_global_ordinals_and_duplicate_occurrence_positions`.

### LP04: boundaries and unchanged failure meaning

Given 20,000 valid rows under the 8 MiB body bound, parsing is Complete with 20,000 located members and the exact last locator. Given 20,001 rows, parsing is Partial with the same 20,000-member prefix and exact unfinished locator at original ordinal 20,000. No extra accepted row enters the vector. Verify 20,001-row generated fixture remains below body cap.

Retain exact existing rejection/quarantine behavior for unknown/empty templates, missing/invalid graduation, missing identity/name, field/row/body limits, and owner mismatches. The wrapper must not alter these branches or hide failures behind `is_err()` alone. Existing unchanged implementation plus existing executed tests is evidence only for branches they actually cover; missing boundary tests must remain a stated coverage limit rather than a fabricated pass.

Proposed test: `positive_roster_locator_capacity_keeps_only_the_existing_bounded_prefix`.

### LP05: actual Window to Records to consumers

Given 67 source rows with rejected ordinal 31 and a multibyte prefix, parsing produces 66 positives. Actual `Window::next` must return the original `[0..64]` and `[64..66]` slices, whose first source ordinals are 0 and 65. `Records::of` retains that exact slice and its accessor remains usable after the stack-local Window value leaves scope. Assert slice pointer identity, member count, every locator and payload provider ID, and safe body access.

Use `Counts::include(records.members(), records.teams())` and checked cursor addition through `records.members().len()`. The production `apply_roster` must use that same retained input, not an unused test-only field. Actual application twice must retain correct 66-positive, 23 Co2027, 23 boys, 0 girls and 1 team counts, rejected ordinal 31, Partial disposition, owed work and 66 canonical athlete facts.

Proposed tests: `positive_roster_windows_and_records_borrow_the_same_global_members`; `actual_roster_apply_keeps_partial_window_counts_and_legacy_replay`.

### LP06: output independence from raw locator coordinates

Given the same real roster fixture with and without an extra multibyte prefix, canonical school/athlete/team projection must be identical while every locator moves by the exact prefix byte length. Existing source observations, published graduation/no-invented-grade tests and different supplied seasons retain their original assertions. This differential test alone is not a pre-change canonical baseline.

Proposed test: `canonical_roster_projection_is_unchanged_by_positive_locator_offsets` plus existing `roster_entities_are_canonical_and_source_independent`, `published_roster_year_is_high_without_inventing_grade_from_the_supplied_season`, fixture roster counts and observation integration tests.

### LP07: historical bytes and effect identity

Use a deterministic pre-change probe executed against old production source, with nonempty school, two teams, two athletes, school observation, two athlete observations, rejected row and fixed source/date/run metadata. Export exact canonical tuple JSON, Journal JSON, staged fact digest and effect digest before the wrapper is applied. Pin the probe code, source hashes, execution log and output hash independently from the new-code test. Compare all four strings against that immutable export; never regenerate expected bytes using the changed implementation.

The old Journal field set/content and staging order remain unchanged. Positive Records members are absent from serialization. Unfinished values require explicit boundary coverage; a golden whose unfinished field is null alone does not exercise Some(locator) serialization, although the same unchanged serializer can be statically inspected.

Proposed test: `positive_roster_preserves_prechange_canonical_journal_and_effect_bytes`.

### LP08: replay and partial progress compatibility

Retain and execute actual existing `partial_roster_survives_reopen_and_corrected_body_adds_only_missing_facts`, `thousands_of_rows_resume_after_one_atomic_window_without_duplicate_facts`, resource-refusal, completion, failed-retry/quarantine and historical unknown-completion tests. Store reopen must preserve canonical facts, counts and owed work. These tests do not read positive source locators back from old Journal. Passing the same still-resident parsed RosterOutcome into apply after reopening Store is not durable locator recovery.

### LP09: ownership and all callers

Keep published/captured owner checks, final-response ownership and ownerless historical-fragment outcomes unchanged. The contextual prefix test must compare unchanged payloads plus exact shifted locator values because whole verdict equality intentionally changes when source offsets move. Verify all call sites in libraries, tests, integration modules, examples and benchmarks. Empty `&[]`, borrowed chunks and count/length consumers should remain source-compatible without cosmetic edits. Separate TFRRS `RosterAthlete` is excluded.

## Layer allocation and finite adversarial properties

This is a narrow borrowed-data representation change, so layers follow risk rather than a mechanical ratio: parser unit/contract cases; real parser→normalization and parser→Window→Records→Counts integrations; actual apply/replay with local temporary Store; all-target compiler and static contract gates. No new external E2E journey is authorized or needed to establish this in-memory property.

Bounded properties are: adding a UTF-8 prefix shifts offsets by byte length only; inserting rejected matched rows increases later ordinals without increasing positive count; duplicate payloads preserve source occurrence multiplicity; window concatenation reconstructs the original located sequence; canonical projection is independent of coordinate movement; dropped Window does not invalidate Records' retained original slice. Deterministic scenarios provide explicit adversarial seeds. No proptest/fuzz/Kani execution is claimed. Existing parser fuzz surface remains the target for later authorized bounded campaigns; no new codec boundary was introduced. No broad verifier is requested for accessor/lifetime changes. Repository owner retired mutation execution; use the following mutation thought experiments rather than overriding that policy.

## Mutation resistance checkpoints

Named tests should fail if ordinal uses positive-vector length, byte offsets use character counts, span covers outer tags, payload is paired by provider-ID lookup, duplicate occurrences collapse, overflow row enters positives, second window rebases coordinates, Records stores a cloned slice, accessor lifetime is tied to local Window, counting skips rejected-gap-adjacent positives, cursor advances by canonical dedup count, members enter Journal serialization, or canonical/staged output order changes. Compilation plus slice identity checks covers borrow shape; it is not a measured allocator/performance claim.

## Root execution plan, NOT_RUN by reviewer

After exclusive integration/formatting, pin final source and test bytes. Use only repository `tools/moon-local` entrypoints. Root should execute `pipeline:fmt`, `pipeline:check` (all targets), `pipeline:lint-src`, and the focused roster tests with nonzero observed test counts. Add affected named integration suites (`milesplit_roster_observations`, recovery/jurisdiction replay, parity pipeline and boundary points) using actual runner binary/test names. A name filter containing only `roster` does not prove that every affected module, example and benchmark was compiled or tested. Preserve exact commands, logs, exits, selected/skipped counts and final source hashes. Root decides broader required gates; neither focused green results nor this review certify release or publication.

## Review stop condition

Finish static review when the final formatted candidate has an exact path/hash inventory, all contract paths and caller migrations are checked, known test quality issues are resolved, and remaining execution requirements are reported to Root. Do not expand into durable acceptance, lifecycle work, authority interpretation, production, browser, network, census or publication.
