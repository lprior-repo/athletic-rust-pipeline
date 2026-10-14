# Typed parser, geocode and relocation verification

This entry records a local focused repair on base HEAD `87df6a681f19b6e2c2995ac44010a9617f687f73`, not full-spec, configured-run or release acceptance. The [source ledger](2026-10-14-parser-repair-files.json) pins 46 changed source files and raw-log digests. Local evidence does not establish publication to PR #29; Main owns any subsequent commit and update.

## Scope and behavior

- The real public owned parser separates nonserialized Malformed and Resource outcomes. Typed Limit, Arithmetic and Allocation causes keep original numeric fields, operation/source and byte/envelope locator. Existing small malformed bytes and historical v3/v4 shapes remain exact. Admission agrees with the decoder's 127-container limit; rendered diagnostics through 1024 bytes preserve exact history, while larger diagnostics refuse as Resource.
- Actual parser and recording-pressure paths retain acquired bytes, capture provenance and explicit receipt status before reporting refusal. The collector retains Partial/owed-work outcomes, blocks later intake and creates no canonical/projection success effects.
- Typed/capture resource writes now use `partial/resource-v2`, a stable original cause and slash-separated `archive/bodies/<digest>.body`. Current physical path and receipt state stay in operator output. Old absolute-path v1 payloads and plain generic Resource identities remain exact; no immutable equality check is weakened.
- Geocode explicitly disables reqwest protocol-NACK replay. A bounded real loopback HTTP/2 peer observes one repaired request stream while the default positive control still observes three. This is a per-dispatch guarantee; redirects remain unchanged.

## Executed evidence

| Lane | Observed result |
| --- | --- |
| Original typed parser | 86/86 passed, 2026 skipped; nextest `5fa55cb2-9bb4-4b2c-9c6a-e0e0e916a2b9` |
| Actual service parity integration binary | 2/2 passed, none skipped; `30e85cfd-190b-448b-a6fb-5f37249be762` |
| Geocode protocol red | 2 passed / 1 failed; geocode observed streams [1,3,5], expected [1]; `3433c6a5-610b-474e-869b-b41486c27ba0` |
| Geocode plus transport regressions | 21/21 passed, 1314 skipped; `748d26b2-af0d-4fbf-9f84-9d269af560ef` |
| Parser/geocode earlier basic | Strict source lint, formatting and all-target/all-feature compilation passed, overall exit 0, 1m41.595s |
| Physical relocation red | 1 failed, 1335 skipped; immutable owned receipt changed after real Store/cache movement; `5f3a3394-234a-4713-ac9d-a721f1eb5d5e` |
| First combined repair attempt, build jobs 2 | Compilation stopped on rustc SIGKILL 9, before tests; Cargo 101, Moon 1. Memory-pressure cause is unproven |
| Combined serial attempt, build jobs 1 | Compilation succeeded; 89 passes including all four new relocation tests; one cache-replay test remained unfinished. Main interrupted with exit 130; `8f9ae9ff-d77a-42c1-bcb3-6f6b09e01277` is inconclusive |
| Crawl-only isolated retry | Outer timeout 124 during changed-feature compilation; no nextest/test execution |
| Same-package isolated retry | 1/1 passed, 2116 skipped, 0.095 test seconds; `c8f7e925-d0db-4266-b25a-f3d72434deec` |
| Full repeated focused suite | 90/90 passed, 2027 skipped, 23.115 test seconds, Moon 0; `e670ed30-5734-4f62-a29f-6780aa5fe479` |
| Ordered final formatting | fmt-write passed, exit 0, 6.870s |
| Final basic after formatting | Strict source lint 1m05.693s, all-target/all-feature check 56.291s, format 8.187s; all three passed, overall exit 0, 2m10.590s |

Earlier Fetcher protocol red, two initial formatting failures, later corrections and admission regressions remain in the source ledger. They have not been replaced by the later green results. The original serial aggregate stall remains unexplained despite same-code isolated and aggregate passes. Unchanged test code does not establish that the stall was unrelated to modified production paths.

The full repeat uses the existing Moon scoped task with `--build-jobs 1 -p census-crawl -p census-service --lib -E 'test(milesplit::owned) | test(resource_recovery) | test(owned_resource_errors) | test(owned_parser)'`. Service parity uses the actual `parity_pipeline` integration binary, not a name-only library filter. Raw logs are retained under [evidence/2026-10-14-parser-repair](evidence/2026-10-14-parser-repair/). Exact shell context and per-input Moon hashes also remain in the recovery packet.

## Exact review alignment

The original 39-file parser proposal received changed-surface review and a separate integrated-format review. Of that integrated state, 31 current files match exactly; the eight differences belong to the separately reviewed 11-file relocation repair. Seven relocation files match their frozen review exactly. Main and the evidence packager read the complete four-file remaining delta: only formatting in one production write! and three test files. The five-file geocode proposal has four exact matches and only a trailing empty-line removal in its shared fixture. The geocode review covers the test-only lib.rs registration excluded from parser semantic coverage.

This composes static changed-surface coverage of all 46 current source files. The 90-test repeat ran before final formatting; the three final basic tasks ran after formatting. There is no claim of a newly executed post-format 90-test run.

## Spec mapping and limits

The source ledger maps named tests to selected requirements in spec §§6, 9, 10, 12 and 16: typed source outcomes, bounded admission, retained provenance, honest partial intake, immutable history, stable relocated identities, retry ownership and caller parity. It does not close whole sections or final acceptance checklist items by implication.

Relocation fixtures genuinely close handles, physically move Store/cache, prove old paths absent and reopen with exact full payload/body witnesses. They use offline seeded bodies, so they do not establish real network first acquisition, process-crash/power-loss/native Restate recovery or qualified cross-system backup/restore. Explicit fallible reservation failures do not establish allocator-wide graceful OOM. Geocode fixtures use synthetic credentials and no vendor APIs.

Full exact-build release gates, all seventeen native faults, bounded parser fuzzing, independent whole-product semantic qualification, resource/performance measurements, full interpretation identity, reusable production, annual rollover and selected/national publication remain unqualified here. Active-context truth-serum was unavailable to the evidence packager; its detached assurance decision remains UNVERIFIED. No gate waiver or merge approval is inferred.

## Final active-context audit and parity

After final formatting, the actual service parity binary passed again: nextest
`9e290a1b-905b-4354-977d-db0ef45e6a3f`, 2/2 passed, zero skipped, 10.895 test
seconds, Moon exit 0 (2m30.467s). This executes fixture ingestion, workbook
readback and rebuild semantic parity without any live census or production Store.

The integration owner subsequently retrieved the repository truth-serum skill
(blob `326e29265dd77e2c0b50f8a4eef4c68f8ebf2500`) and its adversarial-audit
reference and ran a changed-surface audit directly. All 46 file hashes match;
the source-change scope matches exactly; 73 original named tests remain represented
among 97 current named tests. The lexical forbidden-pattern matches are entirely
in modules gated by cfg(test), verified at their parent declarations. The scoped
verification-laundering pattern scan found no match. This is not complete
panic-freedom, mutation, coverage or release proof. Full acceptance remains
UNVERIFIED, including the preserved unexplained earlier aggregate stall.
