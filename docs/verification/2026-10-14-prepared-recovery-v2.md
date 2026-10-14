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
- CLI/bootstrap explicitly select v2 on fresh qualification roots. V1 remains the default.

Review: Core corrections cover post-callback corruption, persistent multi-target owner binding,
occupied-metadata precedence and exact historical/domain errors. Crawl corrections preserve
both primary and cleanup causes and exhaustive classification. A concrete duplicate envelope
parse now charges both traversals. General transitive allocation bounds remain unqualified.

Executed: initial Moon format failed on duplicate module visibility; corrected format passed.
Three all-target checks failed in sequence on five Core visibility errors, two Crawl error
conversions, then five Service reexport visibility errors plus one HandlerError formatting
error. Fourth check reached test compilation and failed on three anyhow downcasts and
one missing fallible-test return type. All reported errors and unused-source warnings have
source corrections; the final corrections have not been rechecked. No v2 tests or source lint
have run yet.

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
