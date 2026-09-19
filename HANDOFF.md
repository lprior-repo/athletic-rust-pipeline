# Current design handoff

**Status: qualification pending.** This handoff records the current source contracts and the exact boundary between available evidence and unexecuted work. It does not claim live collection readiness, identity-matched delivery, or completed end-to-end replay.

## Freeze and ownership

The current documentation-only change updates exactly these existing files:

- `README.md`
- `HANDOFF.md`
- `CHROMIUM_DESIGN.md`
- `WORKFLOW_REVIEW.md`
- `SCOPE.md`

No Rust source is changed here. Main owns integration, executable proof, the remaining source Clippy conversion, native qualification, and final publication. The 5090 owns the ranking parser/storage/publication repair slice; this 3090 lane drafted the current documentation from the source interfaces and the explicit qualification facts. Do not accept model completion messages as proof.

The raw local drafting response is retained outside the repository at `native-rankings-1789772180206/final-development/3090/docs-draft-01.json`. It contains no workbook rows, identities, credentials, or hosted-model traffic. A local model draft is design input, not evidence of pipeline execution.
The one drafting request used the local 3090 endpoint `http://127.0.0.1:11001/v1/chat/completions`, model `Qwen3.6-35B-A3B-UD-Q4_K_XL.gguf`, with `chat_template_kwargs.enable_thinking=false`; it was one request within the six-request cap. No hosted model was used.

This documentation freeze is complete at these five files. Main must append only final, directly exercised checks after the source/qualification freeze; it must not turn this draft or retained historical evidence into a passing claim.

Preserve original workbooks, profiles, active stores, journals, retained binaries, and evidence. Never replace a changed binary under its old journals. Never open a live Fjall store from a second process. For offline verification: owner-online export first, drain, stop the sole writer, then open the stopped store.

## Current source design

For the supplied two-sheet source shape, both source worksheets remain in scope; sport and grade do not select or discard rows. The completed consolidated rankings delivery contains 142,705 unique athletes on `AllAthletes`. It is separate from original-workbook identity matching.

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
- Strict Clippy is not final: one known trivial conversion remains for Main to repair.
- Serialization measurement: 344,131 versus 13,131 allocations across 1,000 synthetic iterations, or 331 removed per iteration. This is not throughput evidence.
- Retained corpus/private 26-case storage qualification exposed bugs currently being repaired by Main/5090. No passing result may be stated.
- Native fixture addresses are Restate admin `21041`, ingress `21042`, fixture `21043`, and worker `21140`; the worker is not deployed.
- Full native automated collection -> matching -> export -> replay has not yet been executed to completion.

The following are therefore still open: fresh integrated source review, current-tree quality gates, native fixture execution against the repaired binary, rankings pause/resume and seal proof, owner-online immutable export, stopped-writer verification, and exact replay with zero new effects. Do not convert any of these into a readiness claim.

Historical synthetic and legacy-run material may remain in retained evidence, but must be labelled historical and must not be used as current-tree proof. Old live profiles/stores/journals and changed binaries remain preserved; they are not replaced or replayed.

## Quality command ledger

**Completed for the current handoff:** source/interface reading and the local 3090 drafting request, retained outside the repository.

**Reported but not final:** fresh production/test-library `cargo check`; one known strict-Clippy conversion remains.

**Planned by Main after the freeze:** repair that conversion; run the approved current-tree compilation/Clippy/targeted native scenarios; execute the private 26-case qualification and retained-result checks; then run owner-online export -> stop writer -> `verify`, followed by exact replay. No formatter, broad test suite, fuzz campaign, benchmark, service restart, or commit was run by this documentation lane.
