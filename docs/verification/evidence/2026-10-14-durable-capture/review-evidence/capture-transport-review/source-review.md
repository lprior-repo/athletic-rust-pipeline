# Frozen durable capture transport review

## Result

**Static source disposition: no remaining source-level blocker found in the reviewed transport slice.** Root can integrate this frozen proposal and run its owned gates. **Runtime/test acceptance remains unqualified**: no build, formatting, lint or test was run by this reviewer. Full service/Restate composition is not established by the transport fixtures.

Frozen proposal: `durable-capture-transport-proposal/changes.patch`.

- Patch SHA256: `b716c818490fde0f5bb052a82af20502b521ee7f4b55f4b8cd097ad515169f35`
- Proposal-pins SHA256: `a7e3a2e274b9807761b841ee44e5d5756147fd656e07372b77accfe7f7d23670`
- Output-pins SHA256: `a8316f9ccad8f3c452040d17ac0427791c4ae62717470a67bdd1469a08422025`
- 101 changed paths, including 23 new paths; core/service changes are outside this patch.
- The exact final source, patch and handoff artifacts are retained in `final-snapshot/`. All final output-source and artifact pins verified from that snapshot. Input-base pins also verified in the proposal base directory. See `final-snapshot-pin-verification.txt` and `final-pin-verification.txt`.
- Root separately reported that this patch applies cleanly with `-p2`; reviewer did not apply it to live source.

All source references below are relative to the frozen source root. The source/skills/ADR scope is recorded in `review-scope.md`.

## Findings resolved during review

### T01 — Mutable cache body read after 304 completion

Disposition: **fixed_with_evidence (static source and authored regression evidence only)**.

Initial code released the compare fence after committing a revalidation, then reread the mutable eligible body path. A legitimate concurrent replacement could produce a post-completion body mismatch. Final `net/cache/eligible.rs:58-72` verifies and owns the body while holding the cache lock, and `net/acquisition/retention.rs:48-68` carries those exact bytes through finalization and returns them without a second mutable read. This does not borrow a newer pointer or create a new body timestamp. `actual_304_keeps_original_occurrence_date_and_eligible_pointer` and the changed-metadata compare-fence regression are authored, unrun.

### T02 — Unrepresentable header lost the actual received response evidence

Disposition: **fixed_with_evidence (under Root's explicit narrowed checkpoint-refusal contract)**.

Final `net/acquisition/session.rs:68-78` passes actual status and actual response URL to `refuse_acquisition_response_checkpoint`. Matching core `acquisition/terminal.rs:52-63` constrains the URL/status and requires the observed URL to match the armed target before retaining the closed `ResponseCheckpointRefused` reason. It truthfully retains an Armed checkpoint plus received-response refusal, rather than fabricating a complete ResponseObserved header envelope. Generic pre-response dispatch failure remains Transport/Timeout only. `actual_overbound_response_header_keeps_typed_status_and_url_checkpoint_refusal` now inspects the exact typed terminal after Store reopen. Unrun.

### T03 — Header-checkpoint failure bypassed known 403/429 source admission state

Disposition: **fixed_with_evidence (static source and authored actual-response regression)**.

Final `net/execute/dispatch.rs:47-52` records the existing access condition from the actual response status/URL even when checkpoint representation fails. Successful checkpoint handling retains the existing final-response block path. The authored malformed-429 fixture asserts one source condition, cooldown, and no second dispatch from a distinct work scope. It uses the actual TLS response, not a mocked classification. Unrun.

### T04 — Retryable pre-arm timeout was permanently settled as NotDispatched

Disposition: **fixed_with_evidence (static source and explicitly injected boundary regression)**.

Final `net/acquisition/session.rs:97-100` leaves a Reserved attempt intact on a pre-arm Timeout; it neither creates a terminal nor automatically retries. A subsequent original-owner grant resumes the same ordinal. Once armed, the normal unknown-outcome rules still apply. `injected_prearm_timeout_reopens_reserved_without_consuming_attempt` asserts no arm/terminal, Store reopen, later ordinal 1, and one eventual request. This is a boundary-injection test, not native DNS-timeout evidence. Unrun.

### T05 — Durable consumer tests stopped at acquire_owned and used empty-selection-tolerant assertions

Disposition: **fixed_with_evidence for the crawl Run bridge only**.

`milesplit/results/run/owned_tests/durable_capture.rs` now checks concrete rows/IDs for two result sets sharing one grant. Its new `durable_run_request_projects_then_reopens_and_replays_the_same_capture` reaches `Run::request`, real owned parsing, metadata acquisition and projection. It asserts the three exact performance source keys/date, projection row count and meet/rsid, then compares serialized performance rows, result receipts and acquisition roots after Store reopen, with the same occurrence and request count 1. This closes the missing authored crawl-consumer bridge; it does not invoke the real service owner or Restate journal. Unrun.

The truncated-body regression's mistaken `/observed` key assertion was also corrected to the actual `/response` key before freeze.

## Positive contract checks

- One application execution path: whole-meet fetch → consumed target-bound grant → immutable registered work → common fetch/dispatch → one `Client::execute` per armed hop. Explicit reqwest NACK retries remain disabled; the existing actual HTTP/2 count test remains relevant but is not a durable-protocol test.
- DNS validation, source/host/family admission, origin ownership and pacing precede arm. No new transport retry loop, browser fallback, proxy or credential route was introduced. The preflight resolver checks at most 64 returned addresses.
- Response checkpoint precedes bounded body processing. Prepare uses exact canonical original metadata and its recorded observation timestamp; immutable archive publication precedes final relation/receipt; only then can a v2 eligible pointer be published.
- V2 cache reuse verifies the original non-revalidation occurrence and exact metadata/body before publishing a new work result. Historical/invalid/mismatched cache does not silently trigger network fallback. The metadata-mismatch test asserts that no second result root was created.
- 304 returns a distinct revalidation occurrence tied to the original body occurrence/date, with the original eligible pointer unchanged. The same-fence owned body avoids a mutable reread and stays within the existing body bound.
- Actual truncated Content-Length responses have an authored three-attempt/reopen regression: received checkpoint, retryable typed Transport uncaptured terminal, ordinals 1–3, and fourth-call exhaustion with no fourth request.
- Grant extraction is synchronous under a short mutex and is moved once; target/cache-root validation happens before grant extraction. The consumer stores retained provenance in the actual AcquiredMeet. Historical receipt serialization is unchanged.
- Test-only DNS mapping, injected clock/interruption and trusted TLS root are all behind `cfg(test)` reachability. No production certificate validation relaxation was found. The two added dependencies are dev dependencies already present in the lockfile.
- No spawn, dropped JoinHandle, unbounded channel, unsafe/unwrap/expect/panic/todo/unimplemented match was found in changed production candidates. The TLS server is joined with its client future; no orphan fixture thread/task was added. See `final-changed-production-structure-scan.txt`.
- Every changed Rust path was line-counted, including mechanical migrations. No changed production file exceeds 300 raw physical lines; maximum is `lib.rs` at 273, followed by `net/mod.rs` at 264. Test-only files account for all >300 results. Compact source is not a formatted AST/style certificate; Root must repeat after formatting. See `final-production-line-counts.txt` and `final-all-changed-line-counts.txt`.
- Protected parser/resource behavior remains intact against parser base `5111a603cc1d945e2da3f85c00d149796552e43e`: `owned/effect.rs` changes only acquisition wiring/provenance; stable origins, `partial/resource-v2` and relative `archive/bodies/...` payload locators are preserved. `owned/parse/error.rs`, `owned/resource_error.rs` and `results/run/resource.rs` are untouched. Updated resource-recovery test diffs are context/signature migration and formatting, not receipt meaning changes.

## Remaining acceptance boundary

1. Root must compile all migrated callers/examples/tests with matching core/service APIs, run formatting and production size/AST gates, then execute the named transport/Run regressions plus existing NACK, redirect, cache/archive and protected parser-resource suites. Zero-selected runs are not evidence.
2. The actual service `results_arms`/original Restate owner, capacity-one blocking composition, journal-skipped guard release, cancellation/drain ownership and native restart lanes remain independently required. The new TLS fixture creates legitimate Store capabilities directly and cannot prove those service boundaries. Existing legacy service replay tests cannot substitute.
3. Backup/restore v2 closure, capacity/I/O faults, concurrency schedules, full joined closure budgets and release/native fault obligations remain outside this source review's execution evidence. No blanket durable-recovery or release acceptance is given.

No live repository edits, build commands, tests, network acquisition, census work or production state mutation were performed by this reviewer.
