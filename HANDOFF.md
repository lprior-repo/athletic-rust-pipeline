# Current design handoff

**Status: qualification pending.** This handoff records the current source contracts and the exact boundary between available evidence and unexecuted work. It does not claim live collection readiness, identity-matched delivery, or completed end-to-end replay.

## Freeze and ownership

This documentation refresh covers these existing files:

- `README.md`
- `HANDOFF.md`
- `CHROMIUM_DESIGN.md`
- `WORKFLOW_REVIEW.md`
- `SCOPE.md`

The prior GPU development lanes are frozen. Main owns integration, executable verification, native qualification, and subsequent publication. The local 3090 drafted the documentation; local 5090/3090 repair recommendations remain inputs requiring source review and execution, not proof.

The raw local drafting response is retained outside the repository at `native-rankings-1789772180206/final-development/3090/docs-draft-01.json`. It contains no workbook rows, identities, credentials, or hosted-model traffic. A local model draft is design input, not evidence of pipeline execution.
The drafting request used the local 3090 endpoint `http://127.0.0.1:11001/v1/chat/completions`, model `Qwen3.6-35B-A3B-UD-Q4_K_XL.gguf`, with `chat_template_kwargs.enable_thinking=false`. Additional storage/fixture recommendations were requested from that endpoint and the local 5090 at port `11000`, model `Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf`; responses are retained as `final-development/{5090,3090}/integration-repair-01.json` under the same private evidence root.

The user pushed the accumulated changes. Subsequent documentation updates must continue to distinguish implementation, executed qualification, and remaining work; a model response or successful compilation is not end-to-end proof.

Preserve original workbooks, profiles, active stores, journals, retained binaries, and evidence. Never replace a changed binary under its old journals. Never open a live Fjall store from a second process. For offline verification: owner-online export first, drain, stop the sole writer, then open the stopped store.

## Current source design

For the supplied two-sheet source shape, both source worksheets remain in scope; sport and grade do not select or discard rows. The completed consolidated rankings delivery contains 142,705 unique athletes on `AllAthletes`. It is separate from original-workbook identity matching.

The expanded roster target is **source-verified Grade 11/junior boys and girls**, within the existing USA/2026 scope, covering **indoor track, outdoor track, and cross-country**. It requires athlete URLs, competing-school history, all available high-school performances, and evidence-backed comparable PRs. The exact contract and current implementation gaps are in [SCOPE.md](SCOPE.md#expanded-roster-target--requested-not-yet-complete). Do not describe the existing boys rankings collector as this completed expanded product.

The deterministic core covers stable source keys, request construction, HTML/JSON/rankings parsing, candidate evidence, identity policy, event/mark calculations, provenance checks, and export projection. The effect shell covers workbook I/O, ranking-index persistence, Restate calls/timers/admission, Chromium/CDP capture, browser challenge state, retries and uncertain-effect evidence, cancellation/drain, durable checkpoints, collection controls, seal, and publication.

Rankings are optional discovery. The requested manifest contains 46 event families. Cataloging expands matching navigation entries to observed variants (current target: 95), excludes walk events, and records absent families. Individual Grade 11 rows and relay members joined through the relay roster can enter the discovery projection. The Grade 11 projection does not decide workbook eligibility.

The rankings parser is bounded separately from source transport: source bodies are capped at 32 MiB while the parser accounts at most 8 MiB of ranking capture state. Pages retain source row numbers, result IDs, candidate kind, and checkpoint digest. Stable public IDs and provenance bind collection, event, page, request, raw receipt, parsed observation, and index. A collection seals only after every planned event has a terminal page.

`SourceCache` retains successful outcomes. Failed ranking outcomes are deliberately not cached; a resumed gateway call receives a new audit identity and can make a fresh source attempt. Restate owns durable attempt/retry scheduling and checkpoints; external HTTP remains an uncertain-effect boundary.

## Current CLI and operations

The actual CLI includes `worker`, `deploy`, `start`, `status`, `browser-start`, `browser-status`, `export`, `verify`, `rankings-status`, `rankings-pause`, and `rankings-resume`. The removed alpha/exhaustive/restate-native commands and deleted `tools/restate-native.sh` are not valid continuation instructions.

`start --output NEW_PATH` submits the durable `run-and-export` workflow, so it queues collection and export publication automatically. Its response is `state: submitted`; it is not completion. Without `--output`, submit the run, inspect `status`, then invoke `export` with a new destination. Ranking controls pause/resume the durable collection and its browser recovery path; a pause is genuine, and resumption is explicit rather than an implicit polling retry.

Real Cloudflare handling is manual human interaction only in the existing headed profile. There is no evasion, cookie extraction/replay, webdriver patch, proxy/CAPTCHA service, or direct source-HTTP fallback. A challenge closes new admission profile-wide while issued requests drain. Resumption requires successful non-challenged document evidence.

## Evidence and remaining work

The current evidence record says:

- Fresh `cargo check` compiled production and the test library.
- Strict Clippy passed before the latest storage/request repairs; final current-tree Clippy remains due. The previously reported trivial conversion is fixed.
- Serialization measurement: 344,131 versus 13,131 allocations across 1,000 synthetic iterations, or 331 removed per iteration. This is not throughput evidence.
- Retained corpus qualification passed: 95 queries, 4,256 receipt digest checks, 340,238 individual results, 57,629 relay-member results, and 142,705 unique athletes. It explicitly retains 15,724 unresolved roster results. Private report: `native-rankings-1789772180206/native-corpus-cleanup-proof.json`.
- All 26 private public-API storage scenarios passed, including replay, conflicts, per-event/collection counts, multiple record references, roster reconciliation, bounds, and reopen persistence. One private oracle was corrected: source positions `1,2,1` contain two distinct positions, not three.
- Focused parser/storage/bundle regressions and the new request-serialization regression passed. `result_verify` passed 13/14 scenarios; `rejects_indistinguishable_local_selection_with_rehashed_evidence` still has an obsolete assessment enum spelling. The command stopped before executing the subsequent workbook targets.
- Native fixture addresses are Restate admin `21041`, ingress `21042`, fixture `21043`, and worker `21140`; the worker is not deployed.
- Full native automated collection -> matching -> export -> replay has not yet been executed to completion.

Remaining gates are final current-tree quality checks, the restored fixture repair, native worker deployment and execution, rankings pause/resume, owner-online immutable export, stopped-writer verification, and exact cached replay without new source effects. Expanded girls/indoor/XC roster coverage and complete performance-history publication are additional unfinished requirements. These must not be represented as readiness.

Historical synthetic and legacy-run material may remain in retained evidence, but must be labelled historical and must not be used as current-tree proof. Old live profiles/stores/journals and changed binaries remain preserved; they are not replaced or replayed.

## Quality command ledger

**Executed:** fresh production/test-library compilation; strict Clippy at the earlier integration checkpoint; retained-corpus qualification; all 26 private storage scenarios; focused parser/storage/bundle regressions; and a request-serialization regression that failed before the repair and passed afterward.

**Latest incomplete command:** `cargo test --test rankings_parser --test rankings_storage --test workbook_verify --test workbook_zip_layout --test result_verify --test bundle_verify` stopped on the remaining restored fixture failure described above. Do not count targets after that failure as executed.

**Next:** repair the remaining fixture, finish current-tree gates, deploy only a fresh native worker/store/journal identity, and exercise collection -> matching -> owner-online export -> stopped-writer verification -> cached replay. No broad unit suite or fuzz campaign is required by this handoff.
