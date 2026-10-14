# Prepared recovery v2, 2026-10-14

Scope: explicit fresh-only capture v2 through the existing domain/Store/Crawl/Service plane.
Historical v1 wrappers and bytes retain strict version dispatch; lifecycle remains 4. No
production enrollment, automatic migration, live census or historical rewrite is enabled.

- Exact original owner, slot, attempt and Prepared digest remain bound across recovery.
- Store owns the actual archive fence and append-held closure. Fresh fixed-leaf hashes reject
  corrupt restoration; missing backing closes atomically with a zero-credit terminal receipt.
- Genuine retained body ownership reaches parsing once; optional eligible publication has its
  own bounded loan and preserves durable retained truth if publication fails.
- Two named SDK runs freeze selection/discovery before execution and replay the original pin.
  Size limits, cumulative read/IO/allocation allowances and typed cleanup causes refuse excess.
- CLI/bootstrap explicitly select v2 on fresh qualification roots. General serving still defaults
  to LegacyFresh; historical v1 has explicit reopen/replay support.

Review: Core corrections cover post-callback corruption, persistent multi-target owner binding,
occupied-metadata precedence and exact historical/domain errors. Crawl corrections preserve
both primary and cleanup causes and exhaustive classification. A concrete duplicate envelope
parse now charges both traversals. General transitive allocation bounds remain unqualified.

Executed follow-up: Moon all-target/all-feature check passed (3m41s) after five failed
checks exposed and corrected visibility, error conversion, fixture and common-terminal enum
errors. Moon format passed. The focused Store acquisition suite passed **90/90**, with 591
skipped (nextest `9fd79bbe-dadf-4db8-aa38-c96fa1f266f5`). This includes genuine J0/J1 80/82-root,
encoded-byte and receipt-probe boundaries, historical-family/reopen and backup preservation.

The combined Crawl/Store/Service test build failed when the Crawl test compiler received
SIGKILL after 8m29s; no tests executed in that attempt. Cause is not established. An earlier
invocation was rejected for duplicate test-thread arguments before compilation. Crawl parser,
publication and Service SDK endpoint regressions compile but remain unexecuted; source lint
and full release qualification remain pending. Functional recovery iteration preserves the
existing bounded read/debit/EOF order; additional endpoint/receipt assertions are included.

Not run/accepted: full focused regression envelope, actual SDK cases, native-v7 frame limits,
crash/SyncData/cancellation scenarios, complete physical read/allocation/resource traces,
release/fuzz/security gates and full specification acceptance. V6 SDK fixture source is not
native-v7 execution evidence. Additional reviewer fixtures will follow as ordinary commits.

Specification mapping: durable incremental recovery and evidence-backed capture provenance
advance here; interpretation identity/reuse, reusable production lifecycle, stable cross-cohort
identities, selected-profile membership/completion and consistent selected workbook/normalized
publication remain incomplete. This PR is not nationwide-crawl readiness.

Home verification from repository root:

```sh
tools/moon-local run pipeline:fmt pipeline:check pipeline:lint-src
tools/moon-local run pipeline:tests -- --build-jobs 1
env -u CI tools/moon-local run pipeline:xtask -- scan
env -u CI tools/moon-local run pipeline:xtask -- seams
env -u CI tools/moon-local run pipeline:gate -- --release
```
