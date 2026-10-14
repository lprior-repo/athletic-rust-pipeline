# Proposed test inventory

All paths are under census-service/src/restate_services/jurisdiction/results_work/replay_tests. Status: written, uncompiled, unexecuted.

## replayed_capture_results_run_releases_unused_original_owner_admission

A real capture Store contains a retained original-owner work identity and one typed NotDispatched(AdmissionRefused) attempt. A complete SDK journal contains actual typed input, retained eager state, the Results Run and a recorded nonterminal ResultsStageOutcome.

Two sequential deliveries each invoke the real Results production path. Each must return byte-equal JSON outcome and emit the exact expected saved JurisdictionState, with the original identity, history window and source plan preserved. Neither may emit a new GetEagerState, Run or Run proposal. Acquisition key set, complete acquisition payloads, retained source meets, Store sequence and every FetchStats field must remain identical. Same-key admission through the exact shared Jobs instance must succeed after each replay, and every resource charge must return to baseline. Final TaskReport must be all zero.

Mutation sensitivity: executing rather than skipping the run changes worker accepted count and cache hits; leaking the unused closure's WorkAdmission blocks same-key admission or leaves a resource delta; changing replay serialization/state saving breaks exact output/state comparisons.

## capture_results_stage_finalizes_offline_attempt_with_only_stage_permit_then_replays

A lifecycle-4 Store contains one selected owned meet; only its synthetic meet-results page is cached. The actual Results worker runs under Spawner capacity 1 and reaches the owned capture path. Offline transport refuses the uncached owned API URL after a real attempt reservation, with no physical source HTTP request.

The emitted Run proposal must decode to one exact failed MileSplit source row with meets=1, rows/withheld/unresolved/resolution absent, errors=1, no pending date obligations, exactly the original meet URL unfinished, and one note identifying the uncached owned API URL. Capture roots must contain exactly one slot binding, one work identity, one attempt-1 reservation and one typed NotDispatched(AdmissionRefused) terminal; no dispatch or attempt 2. Exactly one meet-page cache hit and zero physical source requests are required.

The actual Run/proposal frames are recorded, then replayed through the same production path. Returned bytes and saved state must match, all acquisition/Store/HTTP witnesses must remain unchanged, and original-key admission must succeed. Final report must be accepted=completed=1 with every other field zero and full resource restoration.

Mutation sensitivity: recursive acquisition of the only stage permit cannot complete; deleting durable refusal removes the exact terminal; dropping the owned target prevents acquisition roots; replay execution creates another worker/cache hit; treating failed results as terminal success violates the asserted outcome.

## replayed_capture_results_failure_preserves_error_and_releases_original_owner_admission

A real original-owner capture root is present. The SDK journal records a Results Run terminal failure with code 409 and message "recorded capture owner conflict".

The actual production Results path must preserve that exact error in its output, emit no saved successful state or new Run/proposal, leave acquisition payloads/keys/Store/HTTP unchanged, promptly re-admit the exact same key, restore every resource, and drain with a completely zero TaskReport.

Mutation sensitivity: flattening/rewriting the replay error changes code/message; success conversion creates saved state; unused-closure admission leakage fails re-admission/resource checks.

## Scope and topology

The source Fetcher is explicitly offline. The only listener is the existing style of local SDK HTTP/2 frame endpoint. Each test uses its own temporary Store and Spawner. No sleep-based synchronization, environment-variable switch, unowned fixture task, or public API expansion is introduced. The capacity-1 worker path uses the actual dedicated worker and actual selector's Tokio blocking work. The fixture server and assertion future are joined and the region is drained even when the assertion future returns an error.

These are behavior regressions, not a temporal model, property proof, successful-source fixture, cancellation proof or native Restate restart qualification. The distinct live dispatch/archive caller-drop case remains uncovered.
