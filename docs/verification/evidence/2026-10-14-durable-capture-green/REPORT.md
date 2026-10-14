# Additive durable-capture verification evidence

Focused execution is green: 158 composed library tests, five explicit CLI tests and two workbook-parity tests passed. Final source lint, workspace/all-target/all-feature compilation and formatting also passed. All nine disclosed current-byte static review gaps are reconciled. Technical release and full-product qualification remain **UNVERIFIED**.

This is a detached additive proposal. It does not rewrite the earlier checkpoint, erase failed logs, change a gate or publish anything. The separate contextual Architecture §11 correction is provided for Root's review and application.

## Exact source and publication scope

Tested local integration commit: `3708d6bab0b08c390887c562a89e8c6714255a5d`.
Parent: `5111a603cc1d945e2da3f85c00d149796552e43e`.
Tree: `b66f002bde5446f9ff1384d393737452655ea47a`.

The frozen checkpoint's 237 source-snapshot paths all matched their retained SHA-256 values, the integration commit blobs and the clean worktree at initial verification. The integration commit changed 318 paths: those 237 source-snapshot paths and 81 frozen checkpoint/evidence paths. `source-files.json`, `source-pins.sha256` and `commit-scope.json` contain the exact inventory and Git blob identities. This is not a claim that every changed path is production code or every repository file was independently reviewed.

After all six capture-verification lanes completed, Root began a separate roster test probe: a cfg-test module addition in `crates/census-service/src/census/sweep/roster/journal.rs` and a new `journal/` test file. That later probe delta is excluded and was subsequently reverted exactly; Root reported the worktree clean again at 07:35 UTC, independently rechecked by this worker. The evidence remains bound to commit `3708d6ba` and does not include that separate probe result.

The existing publication receipt, observed at 2026-10-14T07:29:18Z, records [PR #29](https://github.com/lprior-repo/athletic-rust-pipeline/pull/29) open and non-draft at remote head `fe2731bd83629d4c33f7f4dfd8222aacfa9291a1`, with the same complete tree as local `3708d6ba`. It verifies all 318 manifest paths and 6,396 untruncated recursive tree entries with no missing, extra or mismatched entries. Its check-run, workflow-run, status and review collections are empty; the combined status is pending, not passed. Main is unchanged at `1e20d764dcf4539b91975b87110736ce2ec52594`, and no merge was attempted. `publication-reference.json` is a compact extraction with the original receipt hash. This additive proposal is not part of that frozen publication.

## Actual terminal results

All numeric exits below were confirmed by Root, separately from this worker's raw-log inspection. Exact complete commands and explicit toolchain setup are in `commands.md`; machine records are in `verification-ledger.jsonl`.

| Lane | Observed result | Raw-log timing |
|---|---|---|
| Composed lib selection | Exit 0; 158 passed, 3,113 skipped; Nextest `ee61a899-74a7-4875-a662-e2a18dee8c21` | 4.230 s tests; Moon 7m 11.124s |
| Binary CLI + CLI integration | Exit 0; 5 passed, 127 skipped; Nextest `14ef0659-9b43-4ea1-9f5a-ebd9d339e709` | 0.089 s tests; Moon 3m 11.265s |
| Source lint | Exit 0; complete `pipeline:lint-src` | Moon 1m 27.636s |
| All-target compilation | Exit 0; complete `pipeline:check -- --jobs 1` | Moon 4m 38.149s |
| Format check | Exit 0; complete `pipeline:fmt` | Moon 10.552s |
| Workbook parity integration | Exit 0; 2 passed, 0 skipped; Nextest `6548ea10-9069-4911-9220-b3058aa7bd73` | 9.881 s tests; Moon 18.134s |

These durations describe the commands and test harness, not application latency, throughput, compiler speedup, cold-build performance or resource capacity. Moon's scoped task is uncached and the logs contain fresh Nextest IDs and actual selected results. Local dependency/compiler state reuse is not excluded.

### Counting without double credit

There are 165 distinct current focused test identities: 158 + 5 + 2, with no cross-run overlap. The composed run contains 58 crawl, five domain, 66 service and 29 store tests. The CLI lane has two binary parser/control tests and three actual CLI integration tests; the workbook lane has two distinct integration tests.

All 30 distinct historical core tests and all 144 distinct historical composed-second tests are included in the 158. Do not add 30 or 144 again. The older composed-second log prints its failing test again in the final failure section; that repeated line is not a 145th execution. Its historical result remains 143 passed and one failed.

Fourteen test identities are newly selected relative to composed-second: the three FetchError boxing regressions, private service-context layout, the two earlier transport-layout/boxed-summary regressions, SelectedHistory membership, the separate observed-response service retry cap, three Results SDK replay tests, the actual selector/DNS test, and two team-source replay regressions. "Newly selected" does not mean every test was newly authored.

The broad `acquisition::`/`capture_tests::` predicates also select pre-existing provider regressions for Arbiter, AthleticLIVE, Athletic.net, MPA, RIIL and UHSAA. Their passes retain their original fixture meanings; they do not establish new durable protocol adoption or all-provider qualification. The `national_fanout...49_owned_partitions` fixture proves its asserted partition/read behavior, not a completed national census. `selected-test-results.json` retains every exact name, binary, log line and declaration hash; `test-overlap.json` records the historical intersection and fourteen-name delta.

## Acceptance changes supported by these results

- The stale unknown-outcome service oracle now passes as `Interrupted`/`RetainedOutcomeUnknown`. A separate actual Store fixture records status 500 and `BodyIncomplete`, then proves `Exhausted`/`AcquisitionBudgetExhausted`. Both reject a fourth reservation and compare unchanged `work/` and `slot/` fingerprints. Unknown outcomes are not relabeled known exhaustion.
- The exact selector plus localhost/DNS fixture completes with one async worker and one Tokio blocking thread. All three real SDK Results protocol fixtures pass, including completed/error replay, unchanged durable state, same-key readmission and exact drain accounting. The offline-refusal case is not successful live-body acquisition or native Restate restart.
- CLI execution now covers the correct binary and integration targets rather than inferring CLI coverage from `--lib`. Fresh capture init/show and explicit serve opt-in remain distinct from legacy adoption and annual production lifecycle changes.
- The clock-race-corrected canonical envelope test, 42-root closure, 83-root original/revalidation join and oversize refusal pass. Those assertions do not establish total I/O or workload capacity.
- Typed error conversion, Display/source preservation, exact refusal/retryability and the strict `<128` error-size guards pass. The private service wrapper's `<= two machine words` guard passes. No exact byte size, total stack saving or workload performance number was printed or inferred.
- The two parity tests pass for catalog identity-versus-semantics and fixture-store rebuild semantics-versus-publication identity. This is regression coverage, not production workbook publication or full recruiting correctness.
- Final source lint and all-target compilation close the retained crawl/service lint and missing native `ServeOptions` initializer blockers for the tested commit. AST/size/safety/release and native execution obligations remain separate.

`acceptance-deltas.jsonl` preserves every original CAP/FULL requirement and its remaining gap, adding exact current test evidence. `finding-deltas.jsonl` preserves prior findings and clearly limits each closure. Prepared lost backing, capacity, native boundaries, broader source/AST/provenance and active-context audit findings are not silently closed.

## Nine-file static review closure

The exact seven-file `transport-error-size-review` report and its live hashes close seven gaps. The two-file private service review in `service-context-review/REVIEW.md` closes the other two against its prior reviewed base. Production proposal bytes match exactly; the added service test has only rustfmt line wrapping/trailing comma after the proposal. All final hashes match the frozen 237-path source manifest and tested commit.

The resulting 237/237 count means combined prior review of unchanged behavior plus review of the exact final deltas. It is not a new independent audit of all lines, all 318 commit files, or the repository. The transport error box adds one allocation per capture-store failure; private service context boxing adds one allocation per successful durable admission. Neither implies fallible allocation, strict no-post-init-allocation, reduced RSS or improved latency/throughput. No runtime-performance claim or waiver is supplied.

## Remaining qualification and preservation

Still open: Prepared lost/corrupt backing recovery; the 2 GiB acquisition inventory/writer-capacity mismatch; actual native dispatch/archive/ack/frontier/publication interruption and all 17 native scenarios; whole-workload memory/disk/throughput/RPO/RTO; named proof kernels, parser-fuzz and required security/dependency/release evidence; complete historical execution-time source/binary/corpus/tool receipts; active-context truth-serum; full selected-profile five-state/UP and national recruiting-product qualification. Lifecycle 1/2 historical meaning and lifecycle 3 enrollment-only restrictions remain; lifecycle 4 is fresh capture qualification only, and the lifecycle-5 proposal is excluded.

`home-verification-commands.md` gives full repository-root Moon commands for the user's home verification, without assuming a home path. The actual current release wrapper includes a native durability lane, contrary to the stale Architecture §11 wording. `architecture-doc-repair/changes.patch` corrects only that documentation with exact base/proposal hashes, preserving the separate parser-fuzz requirement and all section numbering. No native run is claimed by correcting the wording.

Every original frozen raw log, including failures and the then-incomplete composed-third snapshot, is retained under `historical-raw-logs/`; additive terminal logs are separate under `raw-logs/`. The earlier checkpoint and 724 protected existing evidence/publication files were verified unchanged. New publication-receipt creation by Root is separately recorded, not attributed to this worker. No Rust build, test, lint, formatter, browser operation, live repository write, commit or publication was performed by this evidence worker.

## Handoff

Root may review/apply the detached documentation patch and append these results as new evidence. Do not replace the existing historical checkpoint with this report or describe a publication receipt as release acceptance. `final-evidence-decision.md` remains UNVERIFIED. `output-pins.sha256` binds the completed proposal; `integrity-check.json` records content checks and the excluded post-verification probe.
