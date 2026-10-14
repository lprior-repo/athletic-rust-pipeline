# Parser-envelope contract review

**Static contract verdict: no unresolved source blocker found in the adopted scope. Executable acceptance is pending Root's integration and Moon evidence.**

Reviewer: independent sol-reviewer, 2026-10-14. No build, test, formatter, lint, benchmark, browser, GitHub action, live source edit, commit, or publication was performed by this reviewer. This is a scoped source/API/test review, not full-project async or release approval.

## Coverage and identity

- Baseline commit: `49ad9922a9ba24148bd4e73dbb46993ee7f39fce`.
- Reviewed revised proposal `context.patch`: SHA256 `f5b3988005370686f93c351949d804c0b8b72b3c6c6cd2f895cbd11c0d4b8aba`.
- Reviewed `source-pins.json`: SHA256 `42935289902b3a5bafb49f43cede4ffcb2bda70e17f993e22f7ce9abc1e92f44`.
- All 36 changed/new source hashes were independently checked and copied into `reviewed-candidate/`. Exact paths/base/candidate hashes are retained in `source-pins.json` and `coverage.json`.
- The shared Root-owned `read.rs` constructor/accessor delta was applied only to this review's detached copy of final archive-lock candidate source. Exact combined hash: `bc9eb2ccc2a167b363721ef2b6598dd9bb708756d761e50c3ae67d26310f1170`.
- Shared new lock-order test accessor delta was likewise combined only in detached review artifacts. Exact combined hash: `c2e2984af298a2835c8f008f54e236d038fa995dbc3480dd276d4d2e4544604e`.
- Shared patch hashes: read `9c224782dda5d0ee1bea8048796cb7b9eef009fac4906c424f525313ffc716ef`; lock test `3be68476fcefc2657f0f5ffeee9307955d314adac0bc77a41b935fba6174c3c3`.
- `review-context-hashes.json` pins 14 supporting source/command/contract files. `named-api-context-hashes.json` pins 30 files matching the narrow interface inventory. The modified archive-lock implementation itself is outside this review; only preserving its `read.rs` fence and adapting getters is covered.
- `contract-reference-hashes.json` identifies the adopted PARSER-ENVELOPE contract, four required skill files and exact supporting references read. AGENTS.md and ARCHITECTURE.md were read; their Moon-only, root-ownership, no-comments and historical-identity rules govern.

Line references below are relative to `crates/census-crawl/src` and the detached reviewed source unless another crate is named. Root formatting can move these lines; pin and re-review semantic deltas before reusing this report.

## Contract findings

Applied rule IDs: `illegal_states_unrepresentable`, `parse_dont_validate`, `types_as_spec`, `explicit_workflows`, `boundary_parsing_required`, `domain_error_taxonomy`, `no_panic_paths`, `allocation_budget`, `two_phase_async_effects`, `capability_gated_effects`, and `resource_obligation_ledger`. Scope-specific runtime evidence remains pending as stated below.

### PE-1: transport capture and authority remain cohesive

Disposition: `fixed_with_evidence` (static source evidence only).

`net/acquisition.rs:20-99` separates the constructible borrowed `CaptureProvenanceView` from opaque crate-private `CaptureProvenance` with owner-private state. `CapturedFetch` fields are module-private. The public methods borrow or consume into raw `FetchOutcome`; there is no mutable getter, DerefMut, AsMut, Clone/Default/Deserialize trusted-envelope derive, or retained builder outside the owner.

The crate-private consuming `into_parser_parts` exposes the raw body and opaque provenance but supplies no inverse. The narrow named-interface inventory finds only two callers, both in `milesplit/owned/outcome.rs`: the production interpreter and cfg(test) historical factory. Moving or changing a raw capture obtained through erasure cannot restore retained authority through the reviewed API.

Retained producers remain bounded to these original owning routes:

1. `net/acquisition/read.rs:20-55`: Store's strict `VerifiedRetainedOccurrence` is consumed; actual owned bounded body bytes are read and `Capture::verify_body` succeeds before retained provenance is minted. The verified Vec is then moved into the envelope.
2. `net/execute/cache_writer.rs:10-55` and `net/acquisition/retention.rs:11-58`: fresh bytes and canonical metadata are passed to retain, checked, archived and completed; the same `body_vec` and metadata are moved into FetchOutcome. Acquisition's successful return couples this response with completed session provenance at `net/acquisition.rs:216-220`.
3. `net/acquisition/retention.rs:61-108`: conditional replay moves the body returned by the checked pinned-cache compare, retains distinct occurrence/direct original, and preserves actual date/metadata.
4. `net/acquisition/read.rs:58-98`: Prepared completion/strict read uses the same checked-body outcome function and retains the Root-owned archive-before-eligible fence.
5. `milesplit/fetch.rs:179-185`: legacy permission check precedes historical construction; a digest or older archive does not confer attempt authority.

This is a source proof of the actual envelope producers, not proof that every Fetcher API is an authority boundary.

### PE-2: interpreted verdict is derived from the same sealed input

Disposition: `fixed_with_evidence` (static source evidence only).

`milesplit/owned/outcome.rs:6-52` owns private capture/provenance/verdict fields. Its production constructor consumes intact CapturedFetch, checks the historical/durable mode before parsing, and derives Parsed/Malformed/Refused from exactly its body/status. No caller-supplied verdict is accepted. Both `fetch_owned_meet` (`milesplit/fetch.rs:128-141`) and `read_owned_meet` (`milesplit/owned/effect.rs:31-59`) use this constructor.

Resource exhaustion returns the original CapturedFetch with the typed original resource failure. It does not destructure/erase authority or clone the body prematurely. Callers explicitly erase only for their existing resource retention paths. Recording-resource failure consumes the interpreted capture. No new journal protocol or error-payload shape is introduced.

`release_body` (`outcome.rs:78-80`) is MileSplit-restricted and replaces only the resident Vec. It preserves bytes count, digest, time, URLs, context metadata and verdict. The result remains an interpretation with metadata after release; there is no conversion back to a complete retained CapturedFetch. `AcquiredMeet::new` still accepts only the sealed outcome.

### PE-3: synthetic historical fixtures cannot mint retained authority

Disposition: `fixed_with_evidence` (source and newly added test definition; test NOT RUN).

`outcome.rs:82-93` gates the only verdict-taking factory with `#[cfg(test)]`, restricts it to MileSplit, and unconditionally calls CapturedFetch::historical. No feature enables it in production. All old literal fixture families migrate to it; date/from-cache/final-URL variants are made in raw capture values before constructing an outcome. Existing v3 parser revision fixtures remain explicitly synthetic historical data.

The first candidate lacked direct factory-authority regression. Worker added the sixth test in response to review: `synthetic_historical_verdict_never_grants_durable_capture_authority` (`outcome/tests.rs:162-182`). It uses a real acquired 200 capture plus deliberately fabricated Refused418 verdict, proves HistoricalAttemptUnknown, refuses DurableV1 with exact CacheOccurrenceMismatch, and checks journals unchanged. It must pass after integration.

An external integration test builds its dependency library without cfg(test), so this is not an integration-test support API. Compiler probes supplied here cover the absence/privacy boundary without creating a public test feature.

### PE-4: same-variant arbitrary internal context is deliberately not checked

Disposition: `owner_approved_no_action`.

`outcome.rs:96-104` checks only LegacyMetadata versus DurableV1. An internal MileSplit caller can supply intact retained envelope A with DurableV1 context B and pass this check. The interpreter also accepts a separately supplied parsed u64 meet ID. **This guard does not establish exact owner, store, work, year, profile or meet equality.** Opaque fields do not establish that additional property.

Root explicitly directed retaining the adopted variant-only contract and disclosing this limit after being informed of it. No identity-binding API expansion is included.

Actual production binding was traced rather than assumed:

- Both production interpretation calls immediately use the result of `fetch_owned_capture` and the same immutable ctx/reference.
- `context/capture.rs:41-56` checks target site/meet, verifies the owner's store/cache root and constructs the scoped work before consuming its retry grant.
- `census-store/src/acquisition/capability.rs:96-161` binds the owner's work/scope/store; `transition/work.rs:9-75` checks owner, grant scope/slot and exact persisted work identity.
- `net/acquisition.rs:124-164` checks request representation/cache intent before the admitted/replay acquisition routes.
- `milesplit/results.rs:81-102` creates one private Run and passes the same immutable ctx through every request. `results/run.rs:160-180` reuses the active meet on its site/meet key. Run is private to the results module; no production caller switches contexts on that Run. Tests intentionally exercise repeated result sets and new Run replay.

Thus no actual production envelope-injection/context-switch path was found within the named caller graph. A future internal caller that holds envelopes from multiple owners must not infer exact binding from this variant guard; such adoption needs an explicit stronger contract. A new test asserting rejection of arbitrary retained A/DurableV1 B would contradict the Root-approved scope, so none is requested.

## Caller migration and persistence

A read-only effective-overlay scan across Rust under crates, xtask and fuzz found 133 named-interface lines in 30 files; all new/revised sources override the baseline only in review artifacts. No named external integration-test caller exists. This is a named-interface negative result, not a whole-repository audit.

`remaining-field-scan.txt` has seven superficial matches. Six are unrelated RosterOutcome accesses in the qualification roster example; the seventh is coach verification RowOutcome. Their source was inspected. No remaining direct caller field access to the two sealed types was found outside owner modules. Compiler all-target coverage is still required to catch inference/alias/macro cases a textual scan cannot prove absent.

Persistence preservation is supported statically:

- FetchOutcome's serde definition is unchanged, including skipped body and optional response_url behavior (`net/types.rs:170-184`).
- The new trusted runtime types remain unserialized. Pure OwnedMeetVerdict/Page types retain existing serde; their deserialization is untrusted and grants no envelope construction authority.
- Changed journal/receipt/window sites replace `&outcome.capture` with `outcome.capture()` and `&outcome.verdict` with `outcome.verdict()`. JSON keys, phase names, key functions, manifest/chunk logic, typed rows and schema/policy IDs remain unchanged.
- Archive-lock shared delta preserves its fence and held-lock argument. It changes retained construction syntax and read access only.
- Historical persistence tests retain all original payload/value assertions. Static sameness is not an executed byte-for-byte persisted compatibility certificate; run the exact parity regressions in `REGRESSIONS.md`.

## Async/resources and scan classification

This slice adds no production await point, task, lock, retry, network call, parser algorithm/budget or body clone. Interpretation is extracted from the same synchronous post-fetch boundary. No concurrency or throughput optimization is claimed; this review does not certify pre-existing async hot-path performance or the separate archive-lock repair.

`static-pattern-scans.txt` records the exact reviewed-source searches. No forbidden extraction/unsafe/panic macro match was found. Serialization matches are existing raw page/verdict types, not the sealed envelopes. Async-pattern matches are test error trait objects, a pre-existing fixture clock and Root-owned lock-observer test Rc/RefCell values. None is a new production escape or spawn. No static match is represented as compiler evidence. New production owner source files remain below 300 physical lines (acquisition.rs 245; outcome.rs 108); Root's trusted AST scan owns logical callable-size certification.

## Executed versus not executed

Executed: read-only file inspection; scoped named-type/direct-field/forbidden-pattern searches; baseline commit/status reads; SHA256 checks of all 36 candidate files; successful application of two shared deltas only to this review's detached artifacts and resulting hash checks; preparation of eight compiler-probe sources. Exact source/hash manifests and scan output are retained alongside this report.

Not run: Moon compilation, Nextest, fmt, source lint, production AST scan, compiler negative probes, benchmarks, fuzz/property campaigns, native Restate, faults, release gate, persistence readback or publication. Root prohibited reviewer builds/tests/live edits. The review does not relabel these unexecuted obligations as passes.

Required next evidence: integrate the full patch and both shared deltas without replacing unrelated archive-lock work; capture final source pins; run the six new tests, the focused producer/persistence/resource/buffer regressions, all-target check and source gates; run or explicitly retain as unexecuted the API probes. Root alone owns live execution. No unconditional runtime/release approval is issued here.
