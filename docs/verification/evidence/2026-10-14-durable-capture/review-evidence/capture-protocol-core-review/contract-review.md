# Capture protocol core: independent static review

Frozen review: 2026-10-14 06:17 UTC, core-freeze-20261014-0615. Static contract blockers identified during this review are resolved. The frozen core is suitable for Root-controlled integration and executable verification, subject to the explicit acceptance exclusions below. This is not runtime qualification or permission to run a census/production service.

## Scope and evidence

The live repository is read-only to this reviewer. Root alone integrates and runs Moon builds, tests, formatting and gates. This reviewer has run no builds/tests, made no live-source changes, and made no production, service, census, enrollment or security changes. Writes are confined to this detached review directory.

The review follows AGENTS.md, ARCHITECTURE.md, ADRs 001/002/007/012/026/027/028/037/038, the capture-admission and durable-attempt contracts, the active API.md, and the repository Rust, Holzmann, async, architectural-drift and Scott DDD review guidance. Architecture and contract pins are recorded separately. The reviewer independently verified every one of the73 proposal SHA256SUMS entries, including51 source files, and copied the exact reviewed bytes into frozen-core-20261014-0615/. reviewed-source.sha256 pins this retained review snapshot. The proposal lists30 tests; none were executed by this reviewer.

## Invariant conclusions

### Ownership and domain boundaries

- The explicit lifecycle-4 opener alone produces the writer. Writer checks use the actual Store appends Arc by pointer identity and the exact retained StoreIdentity. A parsed qualification declaration or occurrence reference grants no dispatch authority.
- StoreIdentity is the existing case-preserving value, not the new lowercase acquisition digest type. Its exact bytes participate in qualification, slot, work and reference relationships.
- RetryOwnerGrant, owned work, reservation, armed dispatch, dispatch observation, observed response and prepared capture are non-cloneable/nonserialized runtime capabilities. Every derived capability owns the original work token. The immutable scoped owner and writer can clone; transport does not receive the writer.
- Slot identity excludes representation, refresh/cache intent, pass/retry and physical attempt. Immutable work binds the exact slot, retained manifest digest/run, existing HistoryWindow, source-plan witness, request representation/effective UA and initial cache intent. A changed work under the same slot conflicts rather than resetting the three-attempt ceiling.
- Store verifies retained manifest/scope membership and runtime origin. The service must independently bind the actual WorkflowIdentity/state/plan and retain its real WorkAdmission guard through the supervised worker's drain. Linear Store tokens cannot replace that service cancellation fence. The service slice has separate review and is not qualified by this report.

### Bounded closure and serialization

- Maximum single-work journal closure is 42: slot binding + identity + 3 × (reservation + 5 armed + 5 response + prepared + terminal) + result.
- The complete two-work reader cap is 84 journal roots. The actually reachable maximal settled304 join is 83: its current work has no final prepared root, so 41 + 42. A fixture adding prepared to a304 terminal is invalid, even to manufacture an84-root maximum.
- Receipts are separate physical table reads, at most six for two complete works. They consume encoded bytes, not journal-root count. The conservative encoded allowance is 90 × (65,536 + 4,096) bytes. This is not a bound of84 total physical I/O operations: metadata/lineage probes, key scans, size probes and absent-key checks are additional bounded operations.
- The selected occurrence reader verifies all logical roots/receipts in current and at most one original work, but only the selected body's backing. Prior retryable bodies are distinct occurrences rather than dependencies of the selected body. Inventory/open/backup verify every retained terminal body's backing using the shared inventory budget.
- Complete journal envelopes are measured with a65,536-byte bounded canonical serializer before staging. Typed metadata length alone would not suffice because JSON byte-array encoding inflates it. The second serialization pass cannot exceed its measured allocation. No oversized committed row is justified by the typed input bound alone.
- Readers preflight journal, receipt, lineage and retained-manifest value sizes before materialization. Original occurrence traversal is a direct leaf join, not recursive. Per-operation backing memoization is cleared at each public Store operation; consumable counters are not reset.

### Retention, retry and response truth

- Reuse and304 originals admit only reusable200 ResponseRetained occurrences with matching request representation and StoreIdentity. A304 has its own actual observation and terminal while retaining the original body's occurrence, metadata and date. It cannot point through a revalidation chain or fabricate freshness.
- Response disposition is persisted and independently checked against the shared status taxonomy. Retained429/5xx responses have an occurrence and receipt but no settled work/result, allowing only a later original-owner continuation inside the same three attempts.
- Actual request construction, adopted DNS/destination checking and admission/cooldown precede arm in the inspected transport caller. The consumed observation token is passed immediately before the one-shot execute. A count-attempt await after arm can conservatively leave Armed on cancellation, but does not falsely claim dispatch completion.
- Generic dispatch failure accepts only Transport/Timeout and records OutcomeUnknown. A dedicated actual-response checkpoint refusal retains checked status and actual URL with a closed UnrepresentableResponse reason, requires the exact Armed checkpoint, and validates the actual URL against the armed request. It is excluded from generic NotDispatched and observed-response failure paths.
- Generic journal batches, DerivedStage publication journal entries, generic receipt commit/credit, and receipt pruning contain capture-family guards. These prevent bypasses around the typed transition writer.

### Lifecycle and preservation

- Explicit fresh initialization accepts only a nonexistent or genuinely empty root, stages ordinary seeds, existing lineage, the strict qualification declaration and lifecycle4 together, and never enrolls/adopts a legacy root.
- Ordinary opens/control and migration must refuse capture execution before ordinary recovery mutations. Existing-only open checks all required keyspaces before opening them. A shared preflight now examines only existing lifecycle/capture namespaces and requires complete keyspaces before generic opening can create missing ones; migration and backup both use it.
- Historical preservation detection first probes lifecycle/capture/receipt namespaces and returns without interpreting unrelated historical formats. Lifecycle3 remains enrollment-only and historical1/2 preservation behavior is not intentionally redefined.
- Cache-v2 inventory verifies the original reusable200 closure, matches canonical sorted-pretty original metadata bytes using the same recipe as transport, and verifies the live eligible body's digest/length. A renamed or mismatched live cache is additionally refused by the real transport's URL/representation checks.

## Findings and resolution status

Resolved and rechecked against the frozen source:

1. Removed recursive original-reference traversal.
2. Added receipt, lineage and run-manifest size-of preflights.
3. Removed stage value rereads/budget resets: stage is absent-root-only; equal registration/reserved replay flushes retained bytes. Current/conditional joined reads share one operation snapshot/budget.
4. Added conservative physical-key headroom to the default encoded allowance. The earlier omission was a bound-proof gap, not a demonstrated legal-input failure.
5. Bounded physical keys before cloning/decoding and charged inventory scan keys.
6. Replaced unconditional full format interpretation in backup preservation with a minimal capture detector.
7. Added same-StoreIdentity checks on304 completion, joined reads and backup edges.
8. Moved migrate ensure_dirs after capture refusal.
9. Excluded retained retryable responses from work/result success.
10. Cleared backing verification memo authority between separate public operations while keeping counters consumable.
11. Replaced generic Armed known-failure exceptions with the dedicated actual-response checkpoint-refusal transition; decoder requires exact Armed and actual URL binding.

12. Added shared migration/backup preflight for missing capture keyspaces before keyspace creation.
13. Added strict unknown-field decoding for the nested response-refusal variant and a direct regression.
14. Pinned, hashed and debited the inventory run manifest once before scanning work.
15. Added positive mixed-case StoreIdentity preservation tests; API now names actual-response URL.

16. API service-worker wording now describes the supervised worker and its approved unchanged owning WorkflowIdentity guard, without freezing the superseded blocking-worker design.
17. Split format/open and backup HTTP metadata helpers so every raw production Rust file in the frozen slice is at most300 physical lines. This is only a pre-format scan; formatting can expand compressed source, so trusted logical function/file gates remain Root-owned.

No unresolved static blocker remains within the reviewed core scope. Root's build, formatting, lint and integrated fault tests remain required; neither the source pins nor this review establish that those gates pass.

Withdrawn concerns are not findings: the existing archive writer synchronizes directory ancestors during creation, so no unsupported ancestor-fsync crash defect is asserted. A suspected same-work result pointing to non-retained failures is already rejected by ImmutableAttemptResult::occurrence(). Generic write/receipt guards are present in the detached source; an earlier live-baseline reading was corrected.

## Capacity and recovery acceptance limits

The acquisition inventory uses common limits of1,048,576 root debits,512MiB encoded reads,2GiB unique backing-address verification and64MiB memo accounting. Exact immutable path/digest/length tuples deduplicate only inside one operation. This fixes repeated archive verification for many aliases, but does not align every legal writer output with future reopen/backup admission. Live eligible paths may still require an additional verification of bytes already seen under an archive address.

The existing HTTP archive policy accepts a configurable positive-u64 CENSUS_ARCHIVE_CAPACITY_BYTES value, default100MiB. OPERATIONS'200GiB figure describes an individual publication artifact, not an HTTP archive maximum. The new2GiB acquisition inventory ceiling is therefore a distinct support limit without an open/backup/restore override. A writer can produce a larger configured archive than the validator accepts. Fail-closed integrity behavior is sound but does not establish required archive/workbook scale or restorability. The API now explicitly calls this unresolved scale acceptance rather than corruption or production qualification.

Prepared with missing/corrupt original backing remains fail-closed without allocating a new attempt. IDs and retained facts remain unchanged. The API explicitly calls this an unresolved durable-recovery acceptance gap; a lost-backing terminal/retry policy needs adoption before a full crash-recovery claim.

The new acquisition inventory budget is not a whole-backup I/O or allocation proof. Existing independent backup tree/HTTP validation scans remain outside this new budget and should not be represented as covered by the84-root or2GiB acquisition claims.

## Required evidence before qualification

Root must execute the repository-approved tests/gates on the integrated consumer, including maximal42-root history and83-root joined304 under84 cap, encoded/backing exhaustion, extra roots/receipts, nested unknown fields, positive uppercase StoreIdentity round trip, strict fresh/lifecycle refusals without mutation, backup/restore historical compatibility, actual-response checkpoint refusal, all terminal replay/corruption cases, and cancellation/panic/prepared recovery in the real worker. The current source includes exact42/41 root admission/refusal, mixed-case identity, nested unknown-field, stopped relocation/backup/restore and corrupt restore fixtures. Joined83/default84 and maximum-envelope inflation evidence still needs confirmation in the integrated suite. Static inspection of test names is not execution evidence.

## Exact freeze pins

- Version: core-freeze-20261014-0615
- MANIFEST.json: 9118acb0850f91c3facd17e3f78f0614de63d991515c609bbe8d22ac2b16c998
- capture-protocol-core.patch: bbdd23de99f5c9abbc3ff9156fd561632dfec42ad5f3dec31b5417a14467dd86
- API.md: d58e2b93c66cb87c3d2687cd0fa09e57189824be42d375e95524253ca603aa3b
- TEST_INVENTORY.json: 8349a4def470c2c79d6f44f9ed79575a5c619207b25a8e6a02aee1079ebadfd4

Per-file source hashes, including qualification, closure, lifecycle guards, codecs and all tests, are in reviewed-source.sha256. The frozen manifest records each original base and proposed source hash. If Root changes any reviewed file while integrating or fixing gates, these pins apply only to the original frozen slice; the delta needs its own review. The mutable proposal HANDOFF is not part of this review's source identity.

## Root compile-only delta reviewed at 06:18 UTC

Read-only comparison of the51 frozen source paths against the integrated repository found exactly five changed files. The changes are: explicit Vec<(Armed, Option<Observed>)> inference in closure.rs; removal of unused imports in init.rs and transition.rs; one borrowed backup PathBuf in each lifecycle.rs and preservation.rs test. No protocol behavior or persisted bytes changed. These five exact files are separately retained under root-compile-delta-20261014-0618 and pinned in reviewed-compile-delta.sha256. This static delta review does not assert build/test success and does not cover later Root changes.
