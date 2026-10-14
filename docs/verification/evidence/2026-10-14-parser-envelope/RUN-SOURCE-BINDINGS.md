# Source-state binding and evidence limits

The final current source is byte-pinned to commit `771072a1013017a4f8b4993751f801df16f789a2`, tree `1a811cfe9328a4e3eaf7b923f2c18671c540aaef`. The assembler independently read that commit and compared every one of the 38 scope paths with the preserved snapshot. The commit's changed-path set is exactly the same set. `COMMIT-RECEIPT.json` records those comparisons.

The first focused invocation used the initial formatted integration, before correcting the seven expected empty collection types. Its failing `outcome/tests.rs` is preserved in `proposal/compile-repair-01/before/` with SHA256 `376eeee42a5406f1e0147f554a188cb37681c6599ff9964881bab56c836c3da4`. The separately pinned after-image, SHA256 `4ac406d1b4e3719b3805f222dd55c724cba828f505fdbe73abb306d95bf71a8c`, matches the final committed file.

Root reported that source remained unchanged for attempts two through five; only invocation-level debug information requests differed as recorded. The raw logs independently show the actual compiler configuration where relevant. The command receipts provide Root's invocation/outer-status attestation. They do not embed a fresh complete source manifest for every process start, and there is no separately fingerprinted executable in this bundle. Attempts one through four failed before test execution; the fifth terminal log records the 107/107 pass against the unchanged Root-attested source.

The format-check, source lint and all-target check passed against the same scope. Commit equality, source review, a successful formatter/check and a command receipt are different evidence types. No artifact is represented as stronger than its recorded observation.

Temporary API probe examples/modules are outside the 38-path committed snapshot. Their staged hashes and exact compiler outcomes belong to API evidence; they do not redefine production source. A final cleanup/clean check must be independently recorded.
