# Focused capture-core split and lint-grouping review

Date: 2026-10-14 06:45 UTC.

## Verdict

No static blocker found in formatted-core-splits-v2 or the additive adversarial-bounds slice. The responsibility moves preserve the reviewed budget sharing, snapshot use, lock lifetime, terminal classifications, single settlement writer, and persisted work representation. The focused integrated differences observed after application are formatting, trailing commas, and import/module ordering only.

This review is separate from the original core-freeze-20261014-0615 review. It does not approve production execution, resolve the previously documented inventory-capacity/Prepared-backing recovery limits, or claim post-split test success. The reviewer made no live edits and ran no build, formatter, lint, test, service or census command.

## Exact evidence

- Split patch: e1ca81b075733f818539ebc4381df9ba2beb0f0779c149b40dc2553c56d504a6.
- Split MANIFEST.json: a1f5af78f7fb6f817d753b308548f228a8bd59708c11d93acd45bf7837f59f33.
- Bounds patch: 6facffb438d0e0fa2070fa2c088b457a417c3eec95287338ec04b3f1476253aa.
- Bounds MANIFEST.json: 6cab42f8950a72bfcc13160f1954ca2d070bb7e9e07e2483e908ad00e25649c1.

All listed proposal base/source hashes were independently verified. Exact proposal bytes are retained under formatted-splits-v2/ and adversarial-bounds-v1/, pinned by focused-delta-source.sha256. The observed integrated files are retained under focused-live-20261014-0644/, pinned by focused-live-source.sha256. focused-delta-live-comparison.json records both proposal and observed live hashes. Later changes are outside these pins.

## Responsibility moves

A literal-preserving comparison of all56 original functions across the four parent modules and their four child modules found no function added, removed, or behavior changed. Seventeen functions moved. Only inter-token whitespace and the necessary backing/cache module-path substitutions were normalized; string literal contents were retained. Visibility/import changes were reviewed separately. See formatted-splits-function-comparison.json.

- qualification/inventory.rs:4 functions moved. One inventory budget remains owned by the whole operation. The retained manifest is still read/hashed/debited once, and journal, receipts, original references, cache envelopes and backing checks share the same mutable budget. No nested inventory budget is minted.
- read/retained.rs:6 functions moved. The direct original-body leaf join remains nonrecursive, preserves StoreIdentity/request/original-reusable200 checks, and shares the caller's snapshot/budget. Re-exports preserve internal callers; helper visibility stays inside acquisition.
- terminal/failure.rs:5 functions moved. Each public transition retains the same original appends guard through snapshot checking and settlement. The child calls the existing parent settle function, with no new lock or alternate writer. Transport/Timeout remain OutcomeUnknown; only the dedicated actual-response refusal can create the narrow Armed FailedUncaptured variant. Reserved and observed failure exclusions remain unchanged.
- transition/work.rs:2 functions moved. Registration and reservation retain the same appends lock, grant/owner/work validation, immutable equality checks, replay flush, and original three-attempt ceiling. Dispatch/response/preparation helpers keep their borrowed operation budget and snapshot.

The observed integrated production files in this focused slice are between123 and271 physical lines. That is a static file-size observation, not execution of the repository's trusted logical-size or formatting gates.

## SelectedHistoryV1 and byte compatibility

Root approved grouping the existing HistoryWindow and selected year into SelectedHistoryV1. Its fields are private, the sole constructor checks the same history.years().contains(year) predicate and returns the same HistoryConflict on mismatch, and its getters return the unchanged values. It derives no Serialize/Deserialize or unchecked Default. ResultsCaptureScopeV1 remains nonserialized and receives the two values from this checked grouping.

All four actual ResultsCaptureScopeV1::new callers were found and checked: domain tests, Store support, crawl fixture, and service capture_admission. Each passes the same history and year as before. No consumer, source, run, owner, cache-intent or request selection changed.

The AcquisitionWorkV1 declaration and the entire following implementation/WorkWire region are byte-for-byte identical between this proposal's base and source. That unchanged region hashes to4d64c1b3dd7959047c412f8c75e96946d61e8b755fbe239c7006242f885270ed. Field order, checked deserialization, digest recipe, accessors and serialization remain unchanged. The valid input set is unchanged. For a caller supplying multiple invalid values, history validation now occurs while constructing SelectedHistoryV1, before the outer owner-key check; error precedence can change, but no previously invalid input becomes admissible.

The added selected_history_requires_membership_without_serialized_authority test covers valid membership and out-of-window years. No post-split execution evidence for that new test is asserted here.

## ExistingKeyspaces ownership grouping

The private bundle carries the same five owned Fjall resources into finish_existing and immediately destructures them. It introduces no clone, database/keyspace open, lock, recovery step or capability. Capture admission and legacy/install validation still precede ordinary recovery in the same order. The resulting Store fields and ownership are unchanged.

## Adversarial bounds review

The added joined-closure regression constructs six legal attempts through public APIs, five dispatches per attempt, with earlier retained retryable responses, an original reusable200 and a current actual304 pinned to that original. It verifies exact83-root success, default84 success and82-root refusal. It does not add an illegal prepared root to304 merely to reach84.

The two canonical-codec tests deliberately use a string payload to isolate the private generic codec. They establish complete-envelope65,536-byte admission, physical-key bytes charged in addition to the value, and65,537-byte refusal before any journal mutation. They do not assert that the string is a valid response or closure, and do not use it in the legal joined-root fixture. This is the appropriate scope of that schema-agnostic serialization-bound proof.

## Executed evidence, explicitly before these splits

The reviewer read and pinned Root's actual logs rather than running tests:

- core-second.log:30 tests passed,0 failed; test execution1.686s. SHA256 c31cffba298a12a5b0dab07e01e5467766449c7a5c19ca458f22b25986944159.
- composed-second.log: nextest run e91a3300-7512-40c5-98fd-37af6afe75a9;144 tests ran,143 passed,1 failed; test execution4.001s. SHA256 a1ee1a40098b7ce94e6290d9dcf234f05970957a7d5eae0f123708c2107a6e7c.
- The composed log contains33 passing domain/Store core-and-bounds tests, including all three additive bounds tests. The one failure is census-service restate_services::jurisdiction::capture_admission::tests::attempts::original_owner_pass_continuations_keep_one_slot_and_lifetime_cap. Root identifies its expectation as stale; this report does not declare the whole composed run successful.

Copies and hashes are under pre-splits-root-evidence/ and pre-splits-root-evidence.sha256. These runs precede the responsibility splits, SelectedHistoryV1/new membership test, and the pending clock-race repair. They close the earlier missing pre-split83/default84 and generic envelope-boundary execution evidence only. Post-split compile/tests/gates and service repair verification remain Root-owned and must be recorded separately.
