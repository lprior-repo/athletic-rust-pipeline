# Accepted affiliation bridge contract review

## Verdict

**Accept the narrowed in-memory locator-preservation contract for implementation planning. Do not treat the full proposed accepted-affiliation bridge as implementation-ready or qualified.** This is static contract/source review only. All build, test, fuzz, fault, performance, live-source and publication lanes are NOT_RUN by explicit assignment.

Baseline: repository HEAD `3708d6bab0b08c390887c562a89e8c6714255a5d`, clean at review start. Exact examined files and hashes are in `coverage.json`; a symbol search is not a whole-file or whole-repository audit. Root is the sole integrator. No repository file was changed.

Root explicitly adopted the smallest first slice during this review: `LocatedRosterAthlete` pairs the unchanged `RosterAthlete` with the already computed `SourceRowLocator`; preserve that pairing through the actual parser, normalizer, Window and Records callers; preserve canonical output and historical Journal bytes. No new durable family, lifecycle revision or acceptance authority belongs to this gate. `IMPLEMENTATION-CONTRACT.md` makes that choice concrete.

## Findings and dispositions

### F1. The proposal's independent checkpoint still bundles a persistence contract that the first gate does not need

Proposal `contract.md` (“Independent work”), `api-wire.md` (“Immediate located-row API option”) and test AB01 combine positive locator preservation with a separately versioned interpretation family, persisted missing-authority outcomes and commit/reopen membership. Those are different deliverables. Adding a wrapper does not require extending Store, protected phases, source activations, semantic revisions, Workbook wires or schema.

**Disposition: resolved in the narrowed contract by Root's explicit direction.** Implement the existing in-memory route first. Defer immutable body/member interpretation persistence, its strict wire and exact parser descriptor to an independently adopted retention contract. Do not install dormant public accepted APIs, new empty tables or unused lifecycle alternatives to prepare for future work.

### F2. AB01's reopen claim is not supported by the current roster route

`census-crawl/src/net/types.rs:171–183` serializes `FetchOutcome` metadata but skips `body`. `census-service/src/census/sweep/roster/journal.rs:109–128` serializes rejection/unfinished locators and counts, but no positive-row membership. `Records::stage` at lines 45–73 stages existing canonical entities and source observations. `commit.rs:41–68` commits that existing journal and facts.

Thus the accepted first slice preserves locators while the parsed `RosterOutcome` lives, including its borrowed service windows/records. It does **not** make positive locators recoverable by reopening a Store, does not certify the HTTP cache as an immutable archive, and does not create an exact interpretation manifest. Reparse of the same available bytes can recreate the locators; that is a new parse, not old journal membership recovery.

**Disposition: resolved for the first gate by a precise stopping condition.** Replace AB01 with two claims: exact in-memory member/locator preservation; unchanged durable canonical/journal/replay behavior. Durable positive-member readback remains unimplemented.

### F3. “Underlying borrowed athletes” needs an exact API decision

`Roster.athletes` is currently `Vec<RosterAthlete>` (`milesplit/wire.rs:103–107`), and `roster_entities` takes `&[RosterAthlete]` (`normalize.rs:36–43`). A slice of wrappers cannot be borrowed as a contiguous payload slice. A temporary cloned payload vector, parallel locator vector or implicit `Deref` would weaken the small contract or add avoidable work.

**Disposition: resolved by the proposed exact contract.** Change the one existing `roster_entities` signature to take `&[LocatedRosterAthlete]`; borrow each member's payload explicitly. Make the actual Window and Records hold the same located slice. All known callers already pass slices from parsed rosters (including the empty school-only example), so no second normalization API or raw compatibility shim is needed.

### F4. The source-backed positive bridge remains a real, separate authority gap

The positive-row parser computes the locator at `parse/roster.rs:57–63`, then `Parsed::admit` pushes only the athlete at lines 90–92. `normalize.rs:192–207` uses the caller's year and literal `high_school`; neither is a source statement. The current `AthleteAffiliation::new` only checks IDs, date and digest syntax. `affiliation_candidates/observed.rs:89–149` checks local structures/spans while its qualification/activation/binding references remain caller-supplied candidates. None is evidence of official publisher authority or independently dated HS status.

The proposal correctly keeps representation distinct from enrollment, affiliation distinct from HS status, source-effective dates distinct from acquisition, and current school distinct from historical performance school. Its refusal of inferred summer extension, source-name trust, unique-provider-ID-as-acceptance, omission-as-withdrawal and candidate-to-accepted casting is appropriate. The same official roster may support separate claims; “independent HS evidence” here means independent of the performance being admitted, not necessarily a second website.

**Disposition: blocker for positive acceptance, explicitly outside the first gate.** A genuinely observed source pack and Root-adopted authority, temporal and retained interpretation contracts are still missing. Existing Mason/Wisconsin fixtures prove provider-row structure only. Do not broaden the initial exact-day/explicit-interval predicate to guessed academic calendars to force a positive example.

### F5. The eventual consumer is real, but its selected branch is not implemented at this HEAD

`restate_services/publish.rs:205–260` accepts the scalar `WorkbookRequest` and calls `build_workbook`; `jobs.rs:139–151` loads `ExportDataset::for_job` and calls the existing workbook builder. `export/dataset/frozen/capture.rs:19–85` is the existing same-snapshot capture seam, and its streams contain no dated school-evidence family. The proposal properly says the selected branch is future work.

**Disposition: preserve this boundary.** No Workbook change is required for locator preservation. For the later positive bridge, add an explicit selected alternative at the same existing handler and freeze the necessary accepted evidence closure with the base from one StoreSnapshot, under new adopted meaning. Do not reinterpret scalar WorkbookRequest, policy 18, base frozen v3 or old Published receipts. The selected checker must refuse before Stage when any required role remains unqualified.

### F6. Minimize later history machinery to the adopted claim semantics

Spec v2 §5 explicitly permits a simple append-only assertion history plus current-value views and does not require a general-purpose event-sourcing system. The proposal's activation/decision/binding/receipt/invalidation requirements express real safety needs, but their many DTO names and separate conceptual ledgers are proposals, not requirements to create a generic platform now. Reuse existing Store ownership, append serialization, receipt and snapshot mechanisms. Introduce only durable objects needed by the actual accepted producer and its readback. A→B→A and reversible corrections still need explicit distinct accepted revisions; fewer types cannot justify losing that meaning.

**Disposition: Root design decision before the later durable gate.** The locator slice chooses no persistence option and no version number. The full positive bridge still needs Root to freeze an exact wire/capability, update/read/invalidation semantics, compatibility/older-binary refusal and migration boundary. Lifecycle 4 remains result-capture/fresh-qualification scope; the reviewed capture capability only constructs owned-meet work.

## Minimal actual producer-to-Workbook path

1. **Now:** existing `milesplit::fetch_roster` → `parse_captured_roster` → `RosterVerdict` containing located positive members → `census::sweep::roster::apply`/`apply_roster` → borrowed `Window` → `Records::of` → unchanged `Records::stage` and `Commit::window`. Preserve source locations in memory; canonical entities, observations, counts and old journal remain the current output.
2. **Separate retention gate:** retain exact body/context and every positive/rejected/unfinished member under a truthful, versioned roster interpretation with an actual reader/reparse consumer. Do not silently widen the owned-meet acquisition capability or attest historical summaries retroactively.
3. **Positive bridge only after a genuine specimen exists:** one narrow real school-program producer through the same acquisition plane; accepted person and institution/publisher binding checks; separately assessed dated representation and independently supported HS status; a specialized atomic commit using the existing Store owner and a complete dependency fence. No second crawler, publisher, population intake or person-allocation path.
4. **Existing publication consumer:** selected alternative of the existing Workbook handler → existing job composition → same-snapshot base plus adopted evidence closure → private selected checker → existing Stage only if all required selected roles qualify. Current school B supplies directory scope; historical marks retain school A. This path need not create a publishable workbook merely to demonstrate the school-evidence dependency: other missing roles remain typed refusals.

## Review limitations and skill use

Recovered rust-contract, Scott DDD, Holzman, async reviewer and architectural-drift guidance were consulted, including applicable type, bounds, allocation and crate-boundary references recorded in `coverage.json`. Binding AGENTS/architecture and the explicit no-build assignment override generic skill commands, whole-workspace scans and retired gate examples. No runtime/safety/performance approval is asserted. Current selected production files are below 300 physical lines in the inspected baseline; post-edit trusted callable/file gates remain Root's job. No new async protocol, spawn, clock, I/O, dependency or concurrency structure is recommended.

The proposal README says substantial unrelated dirty work remains, but its source-pins says clean and this review observed a clean checkout at the named HEAD. Use the exact repinned coverage and actual integration candidate rather than that stale prose. This is an evidence-description correction, not an implementation defect.
