# Root-owned verification plan

None of these build/test/format commands was run by the proposal worker. Run only after Root integrates the full envelope slice plus both shared-file deltas. All commands below are rooted at `/workspace/shared/athletic-rebuild-20261013` and use Moon. Reject zero matched tests; retain exact tool output, source pins, and exit status.

## New behavior regressions

All six added tests live in `milesplit::owned::outcome::tests` and acquire their original retained envelope through the existing local TLS HTTP transport and real qualification store. No test mints retained refs or uses a production test feature.

1. `sealed_interpretation_preserves_checked_bytes_and_metadata_through_body_release`: Given real retained parsed bytes, interpret that intact envelope. Assert the original Vec pointer and exact body, fetched date, serialized metadata, parsed verdict, and retained occurrence/original pair. Release the resident body; assert zero length and capacity while serialized capture metadata/verdict/provenance remain unchanged. Consume the released interpretation into raw metadata and verify the bytes remain absent.
2. `sealed_interpretation_resource_returns_the_original_retained_envelope`: Given a real captured 20,001-value extension array, assert ArrayValues limit 20,000, requested 20,001, exact offending byte locator, and Unrecorded retention from the pure interpreter. Assert the original buffer pointer, exact bytes/metadata, retained provenance, and proof-erasing move preserve the one allocation. Assert acquisition payloads and interpretation journal stay unchanged after interpretation and only one request occurred.
3. `sealed_interpretation_refuses_historical_authority_before_resource_parsing`: Erase a real retained capture and wrap the raw value as HistoricalAttemptUnknown, retaining its digest/date. Supply a DurableV1 context and resource-invalid body. Assert CacheOccurrenceMismatch rather than parser resource classification, no new source/owned journals, and one physical request. This proves digest/metadata cannot restore durable authority.
4. `sealed_interpretation_refuses_retained_authority_in_historical_context`: Supply a real retained resource-invalid body with legacy context. Assert the same exact mismatch and no interpretation or archive journal effects.
5. `erased_capture_can_only_reenter_as_historical_unknown`: Deliberately erase a retained envelope and parse its raw value via the explicit historical constructor with legacy context. Assert historical unknown provenance, exact capture serialization/body, parser parity, and one physical request.

6. `synthetic_historical_verdict_never_grants_durable_capture_authority`: Give the cfg(test) historical factory a real 200 capture and an intentionally unrelated Refused(418) verdict. Assert it keeps the raw metadata/body, always returns HistoricalAttemptUnknown, rejects DurableV1 through the exact existing mismatch error, and leaves journals unchanged.

Suggested first command:

`timeout 600 tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & test(milesplit::owned::outcome::tests)'`

Expected new test count: 6.

## Existing real transport and persistence regressions

`timeout 1200 tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & (test(net::acquisition::tests) | test(milesplit::results::run::owned_tests::durable_capture))'`

Required covered behavior includes actual HTTPS acquisition and replay, distinct same-byte occurrences, exact original/date on 304, eligible-pointer integrity refusal, retained-body integrity, original Prepared recovery and missing-backing refusal without refetch, cooldown and retry exhaustion, active-meet grant reuse, and projection replay. Include newly integrated lock-order tests; do not replace them with fabricated envelopes.

## Parser/error/receipt parity and migrated historical fixtures

`timeout 1200 tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & (test(milesplit::owned::effect) | test(milesplit::map::owned::tests) | test(milesplit::results::run::receipt) | test(milesplit::results::run::owned_tests::resource_recovery))'`

Key named oracles:

- `fetch_and_owned_reader_agree_on_resource_origin_and_preserve_the_acquired_body`: exact resource origin and NotRequested/Retained distinctions, body locator, original metadata, no parsed verdict receipt.
- `genuine_malformed_fetch_and_owned_read_keep_exact_historical_verdict_bytes`: exact JSON verdict bytes and phase/key behavior.
- `resource_capture_network_metadata_to_cache_replay_preserves_every_original_payload`: unchanged original resource receipts across cache replay/reopen.
- `partial_persistence_network_metadata_to_cache_replay_is_an_exact_noop` and `partial_persistence_retains_new_acquisition_without_restaging_identical_source_rows`: create separate raw variants before historical factory construction; unchanged persistence expectations.
- `xc_interpretation_allocates_v4_and_preserves_v3_rows_and_v1_capture`: original historical parser revision remains synthetic unknown, and existing v3/v1 payloads survive current parsing.
- `relay_interpretation_replays_beside_unchanged_v1_history_and_capture_archive`: source/receipt replay parity.
- Existing receipt ownership/final URL and cache-replay identity tests retain all assertions.

## Existing bounded resident-body evidence

`timeout 1800 tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & test(milesplit::results::run::owned_tests::bounded_buffers)'`

Run both thousand-result-set/same-capture and 10/100/1000 distinct-capture tests. Retain exact fact/peak and replay assertions. The added pointer/capacity regression supplements these bounded collector tests and does not replace them.

## Static/API checks and final integration

After Root-owned formatting, run `timeout 600 tools/moon-local run pipeline:fmt`, `timeout 1200 tools/moon-local run pipeline:check`, and `timeout 1200 tools/moon-local run pipeline:lint-src`. Full release acceptance remains subject to repository release gates; these scoped tests do not claim release/fault qualification.

Source review must verify no mutable deref/getter, exported retained constructor, retained From/Default/Deserialize, generic caller-verdict constructor, or feature-enabled fixture escape exists. `into_parser_parts` must have only the owning interpreter and cfg(test) historical factory callers. All live retained construction must remain acquisition-owner-local checked transport/session/read paths. The public descriptive view is deliberately constructible but no promotion API accepts it.

For negative Rust API evidence if required by the reviewer, Root should stage isolated temporary compile probes through the existing Moon check task, one case at a time, and verify the exact privacy/type rejection before removal: external CapturedFetch or OwnedMeetOutcome struct literals; mutating a value from capture()/verdict(); attempting the historical fixture from a non-cfg(test) library consumer; constructing retained CaptureProvenance; attempting promotion from a manually built view. Do not add doc comments, test features, trybuild dependencies, or public authority escapes merely to support these probes. These probes have not been executed.

## Mutation and property review

Reasoned mutation targets: deleting pre-parse context admission must fail tests 3/4; swapping/discarding capture body must fail tests 1/2; cloning the resource body must fail pointer equality; retaining released capacity must fail capacity==0; altering date/digest during release must fail exact serialized metadata; accepting the descriptive view cannot be introduced without changing the reviewed API. Existing parser/property/fuzz suites continue to own raw parsing semantics; no parser algorithm or budget is changed here. Repository owner retired mandatory mutation execution, so no new mutation campaign or Kani/fuzz command is introduced for this ownership refactor.
