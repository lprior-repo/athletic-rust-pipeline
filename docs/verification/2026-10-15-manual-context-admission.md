# Manual context admission implementation slice

This source slice is not runtime acceptance. It was prepared in an isolated cloud checkout. Only repository Moon formatting was executed; no compilation, tests, live-store enrollment, native service work or GitHub writes were performed.

## Contract

The explicit `PublicIdentityWriterV1::enroll_manual_context_admission_v1` API requires verified current qualification, genesis identity history, no admission namespace and no context-object rows (including orphan chunks/pages). It never resets an existing header. There is no automatic enrollment or existing-store migration.

The new manual-only `manual-context-admission-v1` journal namespace has a versioned store-bound header and immutable operation-hash-keyed reservations. Each reservation carries the complete existing manual V2 record and two root charges. Generic journal writers reject this namespace. Missing or corrupt enrollment refuses preparations; historical replay does not consult admission.

Under the existing append lock, preparation validates qualification, head, entity revisions and request, captures immutable inputs, measures both roots, previews history/witness validity, and commits reservation plus increased header in one SyncData batch before staging. A private store-bound permit then limits the existing immutable writer to the reserved measured root. No second context encoder, collector, pruning or refund path is introduced.

Physical measurement uses the existing chunk/page/manifest wire encoders and includes encoded keys and values. Ready roots are zero new-object charge only after complete immutable-plan verification. Partial roots are fully charged. Both roots, reservation key/value bytes, and a permanent 4096-byte header key/value margin count against 128 MiB. The root allowance is at most 4096, counting both roots per operation. A separate 4096-reservation ceiling bounds audit work. Repeated partial roots across different operations may be conservatively charged twice; this tightens the bound. Charges are never released. These bounds describe retained encoded rows, not Fjall WAL/compaction/filesystem write amplification.

An exact operation retry requires unchanged request, original head and entity revisions, restores the original affected snapshot sequence, reconstructs both original roots, and rejects newly required physical bytes/roots exceeding its saved allowance. It returns the original record/timestamp/sequence. A completed retry makes no writes. Entity/head changes refuse, while journal-only staging growth does not change prepared identity.

The bounded header audit recomputes reservation counts and byte/root sums, rejects unknown namespace rows, validates store/schema/root bindings and retained physical bounds, and requires zero-charge roots to remain ready. This detects ordinary counter corruption and undercharged resumptions. It does not authenticate an adversary's coherent rewrite of every unsigned journal fact; no signing scheme or credentials were added.

## Authored regression coverage

- Exact repeated preparation returns identical record bytes without new rows.
- Request conflict, missing/corrupt enrollment and re-enrollment refuse before writes.
- Reservation-only reopen and interruption between roots retain original prepared bytes.
- Context-writer interruption after chunks and after an index page resumes the same root after reopen and retains full partial-object charge.
- Concurrent same-operation preparations share the original record.
- Physical byte charge equals exact persisted key/value sizes; verified ready reuse is zero.
- Foreign-store permits and changed reconstructed roots fail before writing.
- Orphan physical rows prevent enrollment.
- Equality and one-unit-over cap predicates, arithmetic overflow and a saturated-accounting fixture that refuses a new preparation without writes while the original admitted retry remains available.
- Consistently rewritten zero-charge reservation/counter fixture cannot resume a missing root.

The saturation test deliberately constructs a larger conservative retained allowance in an isolated fixture; it does not claim to simulate 128 MiB of generated payload or measure real storage-engine resource use.

## Remaining acceptance

Run repository all-target compilation, focused store/report tests, zero-comment/AST sizing/Clippy gates, native SIGTERM crash matrix across reservation/chunk/page/manifest boundaries, distinct-operation concurrency at the root/byte caps, corruption matrix (foreign store/schema/physical plan), and bounded memory/disk measurements on selected compute. Verify source/reviewed/manual historical replay and existing wire bytes are unchanged, including successful admitted retries after exhaustion. Native manual crash fixture explicitly enrolls before evidence installation; report correction fixture creation also needs explicit enrollment before its first source birth. Existing real stores remain ineligible until a separately approved bounded reconciliation contract exists.

## Additive shared-ledger wire revision 2

Root-approved membership withholding uses the same counters, root permits, physical measurement and finite allowance as manual splits. Admission header/policy/reservation are now explicitly version 2. The original protected namespace is retained, so creating a differently named quota cannot reset consumption. Version 1 admission bytes are refused before mutation; enrollment refuses any existing admission rows. No real stores were enrolled and no migration is implemented.

Reservation payloads are tagged `SplitV2` or `MembershipWithholdingV1`. Exact retry checks the tag, typed request, operation, head, entity generations, original snapshot sequence, both reconstructed roots and original byte/root allowance. Split prepared approval bytes and accepted witness5 remain unchanged. Withholding caller integration is delivered by its owning worker. A synthetic admission-only test exercises both tags against one saturated ledger and refuses cross-variant retries; it creates no accepted history or Applied rows. Genuine withholding approval/replay regressions remain the caller's responsibility.

The native crash fixture now separately retains the pre-reservation snapshot sequence and complete original prepared record. Retry checks the independently saved sequence and exact record bytes instead of incorrectly equating the reserved sequence with the current journal sequence.
