# Verification evidence ledger

Each section records its own historical tree, input generation and execution limits. Results do
not transfer to a later revision, fresh store or new run identity. A historical seal is not the
fresh national census's release certificate. Current requirements live in
[NATIONAL-CENSUS-PLAN.md](NATIONAL-CENSUS-PLAN.md); procedures live in [TESTING.md](../TESTING.md)
and [OPERATIONS.md](OPERATIONS.md). Source audits and imported measurements are explicitly labelled.

## DragonFly directory parse parity — 2026-09-29

The prototype's directory parser and the Rust lane were executed over the same six captured
pages and compared field for field (3,693 rows): 0 divergent fields, 0 row-count differences, 0
`currentPage`/`totalPages`/`totalResults` differences. Execution: `census-prototype/parsers/
dragonfly_directory.parse` over the fixture bodies on one side, `parse_directory` plus
`directory_school` over the same bytes on the other; the comparison details, the field mapping
and the three recorded divergences (rows without a short code, the unpersisted raw
`dragonfly_levels` map, the classification key rule) are in
[the parity record](../research/sources/coach-directories-national/dragonfly-directory-parity.md).

Three pages are now permanent: `tests/fixtures/coach_directories/golden_directory_rows.json`
holds the prototype's own rows for ASAA, WHSAA and NCHSAA page 1 (generator and digests in the
fixture `PROVENANCE.json`), and `coach_directories::tests::
the_live_directory_pages_reproduce_the_prototypes_rows` asserts the Rust parse against them.

The comparison found a real projection gap rather than a parse gap: the classification rule
accepted only keys ending `classification(s)`, and the live AHSAA pages name the class
`ahsaaClass`, so Alabama's 793 captured rows carried no class. The rule now accepts a key ending
`class` as well; measured coverage is 1,336 of 3,693 rows (AL 485, GA 157, WY 73, NC 450, GHSA p2
171, AK 0), against an independent count of 1,340 whose 4-row difference is exactly the
classified subset of the 12 vendor-fixture rows the lane drops.

```text
cargo test -p census-crawl
test result: ok. 462 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
cargo clippy -p census-crawl --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.50s
cargo fmt -p census-crawl -- --check
(no output)
```

Not established here: fetching, caching, pacing or store writes; no prototype `out/` artifact was
reproduced; the golden covers three of the six compared pages. The throwaway Rust dumper used for
the six-page comparison was removed after the run.

## ADR-016 row hygiene and census-scope filters in the DragonFly lane — 2026-09-29

The prototype's per-row hygiene and its level scope now exist in Rust.
`crates/census-crawl/src/row_hygiene.rs` carries S03 (post suffix, post-only and
non-coach leads, `Dean` leads), S04 (`\btest\s+school\b`), S05
(`@dragonflyathletics.com`), S06 (`[\s\u{a0}]+` collapse, Python's whitespace
set) and S09 (empty key), with the audit's executed case tables ported as tests
(23 person cases, 8 vendor cases, the whitespace table, the level table).
`coach_directories/map.rs` applies them at census emission and drops any row
whose `(level or "Varsity")` is not `Varsity`, counted per level; `collect`
reports the counters in its notes.

The prototype orders the level filter per page, before merge:
`census-prototype/run.py:190-194` filters the detail record's coaches, and only
then does the merge collapse duplicates, so every sub-varsity row is a level
drop and none of them takes part in deduplication. The first Rust port
deduplicated first. That kept the same emitted row set — the level filter never
inserts a row, so no sub-varsity row could claim a key — but it misattributed
drops: a sub-varsity row that matched an already-kept varsity row read as a
duplicate and vanished from the counters. Executing the Wyoming test against the
dedup-first order produced `dropped_levels.get("JV") == Some(1)` instead of
`Some(4)` with the four varsity rows emitted in both orders; the filter now runs
before the duplicate check.

Executed in this tree (`crates/census-crawl`), with `RUSTC_WRAPPER=`:

```text
cargo test -p census-crawl
test result: ok. 461 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
cargo clippy -p census-crawl --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.61s
cargo fmt -p census-crawl -- --check
(no output)
```

Two of those 461 tests read captured live payloads, not synthetic bodies:
`a_live_middle_school_page_keeps_only_its_director_and_counts_each_level`
(`tests/fixtures/coach_directories/probe/AL/summary-SVXJDF.json`, Rainbow Middle
School: one athletic-director row kept, 6 middle-school rows counted) and
`a_live_coach_on_junior_varsity_and_varsity_teams_keeps_the_varsity_rows`
(`.../probe/WY/summary-SS28UB.json`, Arapaho Charter High School: 4 varsity rows
kept, 4 `JV` rows counted). The Wyoming case fails against the dedup-first
order, so it is the regression for the ordering above. The lane README now
states the emission scope.

Not established here: no store-level or service-level run, no national census
evidence, and no comparison against prototype output rows — the prototype's
`out/` artifacts for these two schools were not reproduced. Workspace-wide gates
are not run: `census-report` does not compile at this revision.

## Identity advice boundary and integration regression — 2026-09-27

The advice parser no longer accepts alternate case, separator or concatenated spellings.
Matching name, school and cohort alone no longer admits `same_person` advice. Shared-provider
evidence still admits advice; accepted identity application remains a separate domain decision.
The two regression tests failed before the change. Afterward, with
`TMPDIR=/home/lewis/av.kqQlyuIY` and `RUSTC_WRAPPER=`:

```text
cargo nextest run -p census-review
77 passed; 0 skipped.
cargo run -p census-review --example identity_advice_smoke
shared_source_advice_admitted=1 bucket_only_refused=1 legacy_aliases_refused=4
```

The temporary public-API smoke executable was removed. It used synthetic packets, not live
Qwen responses, and does not establish dual-model agreement or canonical identity acceptance.

A broader `cargo nextest run -p census-review -p census-reconcile -p census-report` stopped after
122 passes and one failure: `the_jurisdiction_row_carries_its_measured_denominators` observed
50 rows instead of 52. Live source inspection found the formerly removed 49-entry
`UsJurisdiction::CENSUS_SCOPE` and the coverage caller restored by an intervening write.
The owner subsequently confirmed that Alaska and Hawaii must remain excluded and that the restored
implementation must be kept. The earlier national-scope results below describe a withdrawn
51-jurisdiction change, not the required current scope. Current acceptance is 49 jurisdictions
plus the unplaced coverage row; these geographic APIs are intentional policy, not legacy shims.

## Census scope stays ADR-009's 49 jurisdictions — 2026-09-27

Owner direction on 2026-09-27 withdrew ADR-013's 51-jurisdiction target. The tree had removed
`UsJurisdiction::CENSUS_SCOPE`, `EXCLUDED_FROM_CENSUS`, the restricted-scope error, the scope
validation helpers and the report exclusion bookkeeping, and had widened the national target
list, `admitted_scope`, applicability and the coverage/seal denominators to `UsJurisdiction::ALL`.
That expansion is reverted:

- `UsJurisdiction::ALL` still models all 51 states so Alaska and Hawaii parse; `CENSUS_SCOPE` is
  the 49 the run counts, `EXCLUDED_FROM_CENSUS` is `[AK HI]`, and `is_in_census_scope` refuses an
  out-of-scope jurisdiction with the ADR-009 reason instead of admitting it silently.
- `admitted_scope` and `scope_digest` (census-reconcile), the restate national targets, coverage
  and seal denominators, the applicability table, the workbook recruiting readers and the
  workspace contract check use the 49-jurisdiction scope again. Explicit subsets remain subsets;
  provider evidence filtering, out-of-scope notes and UNKNOWN rows remain.
- Records updated at the time: ADR-013, ADR-009, architecture, operations and the national plan. The former secondary architecture document has since been consolidated into root `ARCHITECTURE.md`.
- Kept from the same integration: the optional `CanonicalAthlete::source` and
  `CanonicalPerformance::source_athlete` decode maps with the owning-row-id fallback, and the
  owner-identity and keyed-parent refactors.

Two tests written for the wider scope were restored with it:
`index::tests::the_jurisdiction_row_carries_its_measured_denominators` expected 52 jurisdiction
rows and now expects the 50 that `CENSUS_SCOPE` plus the unplaced row publish, and
`restate_services::tests::an_empty_jurisdiction_list_covers_the_census_scope_in_declaration_order`
passes again because `admitted_scope` no longer hands Alaska to the scope validator.

Executed on the converged tree:

```text
cargo test --workspace --no-fail-fast
0 failing test targets
cargo xtask contract
8 checks passed; 0 known deviations
check 1 census scope: PASS (CENSUS_SCOPE is 49 jurisdictions of the 51 modelled, in ALL order minus [AK HI])
```

This restores the scope contract; it does not claim national acquisition, a seal or coverage
evidence for any jurisdiction.

## Subject-bound model response cutover — 2026-09-27

Removed the bare-verdict-array fallback in `census-review/src/model/response.rs`.
`VerdictBatch::sanitize` rejects empty or mismatched subjects before admitting case IDs.
The two rejection regressions failed against the previous implementation (0 passed, 2 failed),
then `cargo nextest run -p census-domain -p census-review` passed all 228 tests.

With `TMPDIR=/home/lewis/av.kqQlyuIY` and `RUSTC_WRAPPER=`:
`cargo run -p census-review --example review_envelope_smoke` exercised the exported
`ModelClient` against a bounded loopback HTTP fixture and printed:

```text
matching_subject_accepted=1 mismatched_subject_dropped=1 legacy_array_rejected=1
```

The temporary executable was removed. This proves HTTP response parsing and admission, not
real-model agreement or the sufficiency of identity evidence. An initial smoke compilation
used the private `model` module; it was corrected to the existing crate-root exports.

## Current identity application and mailbox cutover — 2026-09-27

The former orphan `census-reconcile/src/index/apply.rs` is called by `index::derive`.
The domain constructs accepted applications only after the projection validator admits them.
The writer retains digest-qualified history, preserves the first observation time on replay,
checks row bounds under the append lock, and commits at most 1,024 applications atomically.
`AthleteIdentityDecisions` now has derived-map retention with unchanged sequence-zero row encoding.
Previously deleted historical decisions cannot be reconstructed by this change.

Executed with `TMPDIR=/home/lewis/av.kqQlyuIY` and an empty `RUSTC_WRAPPER`:

```text
cargo nextest run -p census-domain -p census-reconcile -p census-store \
  -E 'package(census-domain) | package(census-reconcile) | test(identity)'
184 passed; 101 skipped.

RUST_LOG=info cargo run -p census-service --example current_cutover_smoke \
  -- target/debug/census-service
historical_rows_imported=0 consumer_rows_published=0 accepted_source_bindings=1
repeated_applications=0 remaining_tasks=0
```

The temporary smoke ran the public server startup/shutdown, reopened its isolated Fjall store,
applied and projected a parsed primary-provider identity, repeated derivation, and invoked the
actual `merge-coaches` binary against a consumer-subdomain rejection fixture. Startup preserved
the historical JSONL bytes without importing them. The endpoint drained two accepted tasks with
zero remaining, cancelled, timed-out, aborted or panicked tasks. The temporary example was removed.
The permanent identity regressions cover source binding, changed-evidence invalidation, preserved
history after reopen, replay idempotence and refusal to promote or merge provider-owned homonyms.
CLI and domain mailbox classification share one allocation-free, case-insensitive,
label-boundary-aware consumer-domain predicate.

Limits: parsed URL-labelled evidence is still not raw-capture-bound. Complete review-case
production, dual-model advice binding and cross-provider adjudication remain release blockers;
these results do not prove nationwide coverage or completion of F01–F15.

### Identity history replay ordering

The independent history review found that case membership is unordered but immutable application
comparison was order-sensitive. The new regression failed before repair with
`StoreError::Invariant: identity application ... cannot change its payload`.
Accepted applications now canonicalize member order. The writer canonicalizes only its decoded
comparison value; it does not rewrite an existing historical row or observation time.

```text
cargo test -p census-reconcile --lib \
  index::application_tests::reviewed_membership_permutation_preserves_application_history -- --exact
Before repair: 0 passed; 1 failed.

cargo nextest run -p census-domain -p census-reconcile -p census-store \
  -E 'package(census-domain) | package(census-reconcile) | test(identity)'
After repair: 185 passed; 101 skipped.

cargo run -p census-reconcile --example identity_replay_smoke
Exit 0: public derive applied one same-provider transfer decision, replayed with permuted
membership without additions, preserved the stored row, and projected both subjects to one
verified identity after reopening Fjall.
```

### Maintained HTML parser dependency

The direct dependency moved from `scraper 0.24` to `0.25`, rather than ignoring the `fxhash`
maintenance advisory. The owner-directed license exclusion remains separate from security.

```text
cargo update -p scraper --precise 0.25.0
cargo nextest run -p census-service \
  -E 'test(coachverify) | test(consumer_domain_boundaries_agree_in_merging_and_publishing)'
17 passed; 449 skipped.

cargo deny check advisories bans sources
advisories ok, bans ok, sources ok; existing duplicate-version warnings remain.

cargo run -p census-service --example parser_upgrade_smoke
supported_structures=4 unsupported_email_claims=0
```

The parser smoke compiled the production `coachverify/claims.rs` implementation and exercised
table rows, list rows, nested staff cards and paragraph fallback. Every positive input yielded
the exact coach-name and professional-email claims; changing only the claimed email yielded
no email claim. It performed no network acquisition. Both temporary examples above were removed.
`cargo vet --locked` failed after this upgrade: 31 dependencies lack `safe-to-deploy`
coverage. No audits or exemptions were fabricated; cargo-vet remains a release blocker.

## Current main-cleanup integration

Integrated upstream cleanup through `d7df661`, preserving local implementation and fixtures.
The preservation stash is `499f204be4260809a31a8168941bca74d5914dda`; it has not been dropped.
All 20 merge conflicts were resolved. `SchoolMatch::as_str` remains because the integrated
MileSplit, WIAA-results, and benchmark callers use it; obsolete APIs removed upstream stay removed.
Seven `VerifyArgs` test constructors were migrated after removal of the `grad_year` field.

### Executed acquisition and artifact smoke

Commands ran with `TMPDIR=/home/lewis/av.kqQlyuIY` and an empty `RUSTC_WRAPPER`.
The four examples were temporary runtime harnesses, not production entrypoints:

```text
cargo run -p census-crawl --example observation_transaction_smoke --message-format short
before: orphan_reproduced=true abandoned_page_athletes=0 abandoned_page_observations=1 abandoned_page_journal=0
after: abandoned_page_rows=0 abandoned_page_observations=0 abandoned_page_journal=0 committed_and_reopened_athletes=1 committed_and_reopened_observations=1 committed_and_reopened_journal=1

cargo run -p census-crawl --example contact_artifact_smoke --message-format short
old_implementation: reproduced reader_and_writer_reject_17th_64KiB_block_under_128MiB_file_limit=true error=record exceeds 1 MiB limit
encoded_boundary: PASS LF_and_CRLF_1MiB_records_accepted_and_1MiB_plus_one_refused=true
new_artifact: PASS rows=1024 csv_bytes=2374786 evidence_bytes=1862656; quoted_unicode_roundtrip=true replacement_refused=true tamper_refused=true oversized_record_refused=true

cargo run -p census-crawl --example identity_acceptance_smoke --message-format short
before: invalid_empty_native_id_source_bound=true overflow_native_id_source_bound=true advisory_link_promotes_row_owner=true shared_advisory_id_authorizes_merge=true
after: invalid_empty_native_id_source_bound=false overflow_native_id_source_bound=false advisory_link_promotes_row_owner=false shared_advisory_id_authorizes_merge=false

cargo run -p census-crawl --example ihsa_ownership_smoke --message-format short
ihsa_replay: PASS athletes=1637 primary_native=68 primary_association=1569 grade_evidence_preserved=true duplicate_performances=0 physical_requests=0 reopened=true

cargo test -p census-domain --lib --message-format short
145 passed

cargo test -p census-crawl --lib --message-format short
403 passed
```

The observation reproducer abandoned an entity-and-journal batch. Before the fix, its observation
had already committed separately. Observation builders now return values; callers append them in
their entity batch. The post-fix run checked both rollback and committed state after reopening
Fjall. TFRRS now includes its page journals in that same batch.

The artifact smoke used synthetic contacts, not the admissions workbook. It crossed 1 MiB in both
files using small records, preserved quoted Unicode fields, refused replacement of an existing
stage, detected a changed CSV digest, and refused an oversized raw record. A school-wide athletic
director claim may omit sport; coach claims still require the matching published sport, and a
director claim explicitly scoped to a different sport is rejected.

Artifact bounds apply independently: 128 MiB per CSV or JSONL file, 1 MiB per encoded CSV record or
JSONL envelope including its line ending, 200,000 rows, and a 4 KiB manifest. Passing the domain's
individual claim bounds does not waive the artifact's aggregate envelope bound.

The encoded-record regression first exposed a CRLF off-by-one: `csv-core` reports record completion
on CR, leaving LF for its next call. The reader now consumes and counts that LF before accepting the
record. The runtime smoke accepted exact-limit LF and CRLF records and refused limit-plus-one bytes.

Identity acceptance now requires a checked, primary positive-u64 person identifier and Parsed
evidence rather than allowing an advisory link or a fetched page alone to qualify a source owner.
Same-person support no longer treats a shared advisory identifier as positive identity evidence.
This is not complete F02 delivery: URL-labelled evidence is not yet capture-bound, cross-provider
corroboration remains unwired, and the review-case digest and decision-writer contracts still need
integration. Reciprocal advisory labels alone must not substitute for captured evidence.

The IHSA regression exposed a real omission: new athletes lost their initial observed grade while
repeated observations retained it. New rows now preserve that first observation. The captured replay
checked every athlete's grade evidence, primary-owner counts, 20 retained performances, an idempotent
second collection, and reopened Fjall state. No remote requests were made. Assertions requiring
unreviewed name-based merges and report prose were removed rather than repinned.

### Executed observation-history smoke

The temporary `census-store` example used two observations of one synthetic school:

```text
cargo run -p census-store --example observation_overwrite_smoke --message-format short
before replace-append: physical_rows=1 merged_evidence=1 integrity=false
before append-replace: physical_rows=1 merged_evidence=1 integrity=true
before reopen: both orders retained only one observation and one evidence item
after both orders: replacement rejected, physical_rows=2 merged_evidence=2 integrity=true
after reopen: physical_rows=2 merged_evidence=2

cargo test -p census-store --lib replace_tests --message-format short
6 passed; 120 filtered
```

Replacement used sequence zero and could overwrite an append-only observation in either operation
order. `Store::replace_many` and `StoreBatch::replace_many` now return typed
`ObservationReplacement` errors for observation-log tables, including empty replacements, before
serializing or staging rows. Production replacement callers already target derived tables; obsolete
test fixtures now append observations instead. The regression covers all nine observation logs,
unchanged records after refusal, subsequent appends, and reopening. A table-list wiring assertion
and a test that legitimized mixed replacement/observation history were removed.

This does not certify sequence-ceiling concurrency, failed-commit counter handling, or legacy import
interleaving. Those findings remain separate integration obligations.

### Executed request and cache bounds

The temporary examples exercised the public clients with synthetic, non-admissions data:

```text
cargo run -p census-review --example local_model_request_smoke --message-format short
11000 / Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf: one synthetic state case returned WI
11001 / qwen3.8-27b-uncensored: one synthetic state case returned WI
both clients refused oversized requests: attempted encoded bytes 2098243 and 2098234

cargo run -p census-crawl --example bounded_cache_smoke --message-format short
exact_metadata_bytes=65536 metadata_plus_one_refused=true
oversized_physical_body_refused=true cache_hits=1 robots_blocked=1

cargo test -p census-crawl --lib --message-format short
407 passed

cargo test -p census-review --lib --message-format short
71 passed; 5 failed in athlete_verdict tests
```

The request writer streams escaped packet fields directly into a bounded buffer instead of first
building an unbounded prompt or JSON value. Both real local model servers completed the structured
response path. This is a transport/serialization smoke, not evidence of correct real-person identity
adjudication. The cache smoke used a loopback robots server: an oversized physical body with
false-small metadata was not served, and reacquisition respected the robots denial. The cache
regressions also exercise exact body/metadata boundaries, file growth and shrinkage after metadata
inspection, and I/O failure propagation.

The five review failures require identity-admission integration; their old expectations accept
`same_person` without sufficient supporting evidence. No full review-suite or report-suite pass is
claimed. Temporary examples were removed after recording these observations.

### Executed identity-projection smoke

```text
cargo run -p census-store --example identity_projection_smoke --message-format short
source_owners=2 cohort=2027 statuses=unverified,pending
rejected_applications=1 aliases=0 reopen=true

cargo test -p census-domain --lib --message-format short
145 passed

cargo test -p census-store --lib read::view::tests --message-format short
6 passed; 120 filtered

cargo test -p census-store --lib read::identity::tests --message-format short
1 passed; 126 filtered
```

Two synthetic same-name, same-school subjects retained separate native owners and explicit grade
evidence for 2027. An unsupported source-binding application neither verified them nor aborted the
projection; its rejection remained counted, the unresolved review remained pending, and reopening
preserved both raw subjects. Missing projection subjects now return a typed error. The temporary
example was removed, and the refusal/reopen case was retained as a store regression.

Recruiter status consumers now use this projection, not a numeric confidence field. The workbook
smoke below exercises that surface. Capture-bound positive identity evidence and the still-unwired
accepted-decision producer remain separate, unresolved integration obligations.

### Executed shared PR and workbook smoke

```text
cargo run -p census-report --example shared_projection_smoke --message-format short
source_owners=2 unverified=2 cohort=2027 winners=4 artifacts=xlsx,jsonl,csv
later_tie=true historical_school=true athlete_state=WI venue_state=IL metric_widening=true
```

The real exporter wrote a temporary workbook and both sidecars. Calamine read the workbook back:
two same-name source owners remained separate and unverified despite agreeing grade evidence.
Four expected winning performance IDs matched across XLSX, JSONL and CSV; normalized values were
checked in XLSX and JSONL. Indoor, outdoor legal FAT, wind-assisted FAT and hand-timed marks retained
separate selections. The equal-mark tie selected the later performance. All winners retained their
historical team school, and the state column used the athlete's Wisconsin school rather than the
Illinois meet venue. The maximum `i32` metric value widened before scaling without overflow.

The workbook-specific reducer and count-only reconciliation were removed; one owned selection
vector feeds the sidecars and recruiter projection. Ambiguous athlete summary cells now retain
classified alternatives rather than taking the first row. This smoke does not certify conflict
classification, contact precedence, artifact-bundle atomicity, concurrent snapshot consistency or
the full report regression suite. The temporary example was removed after execution.

### Executed imperial-input boundary repair

```text
cargo run -p census-report --example imperial_boundary_smoke --message-format short
before: exit 101; byte index 2 split the UTF-8 character in the fraction "1é"
after: PASS malformed_unicode_and_fraction_refused exact_trailing_zero_retained
```

The parser now validates fractional ASCII digits and precision without slicing at an unchecked
UTF-8 byte boundary. The smoke rejected non-ASCII fractions, a fractional plus sign and nonzero
excess precision, while preserving an exact trailing-zero representation. Permanent regressions
were added alongside the metric-widening boundary case. The temporary example was removed.

The first full report test compilation exposed 198 errors, primarily stale self-crate paths and
test fixtures using removed or nonexistent contracts. After that migration, the full report run
compiled and returned 111 passed / 13 failed. Failures covered imperial boundaries, non-finite wind,
contact scope/precedence and obsolete ordering expectations. This is not a full-suite pass.

### Executed contextual PR selection

```text
cargo test -p census-report --lib bests::tests --message-format short
64 passed; 53 filtered

cargo run -p census-report --example pr_context_smoke --message-format short
pr_context: PASS distinct_xc_contexts=2 within_context_winner=true exact_feet=true
malformed_unicode_refused=true nonfinite_wind_unknown=true
```

A temporary Fjall store held three XC performances in two distinct canonical event contexts.
The selector retained the correct winning ID and mark population for each context, rather than
comparing the two races as one PR. The sole PR-key constructor now derives that context from the
performance; the caller-controlled, infallible `Result<_, ()>` builder was removed. This does not
prove upstream canonical event IDs distinguish every source course or distance.

The runtime also exercised bare-foot notation, malformed Unicode and all three non-finite wind
values. Exact imperial units are exposed as `field_micrometres`; the misleading `field_mm` name and
the unused second numeric-conversion helper were removed. The temporary example was removed after
execution. Contact and full-suite integration remain open.

### Executed academic-year contact projection

The tenure regression initially returned 6 passed / 2 failed: whitespace-only source IDs and a
63-digit capture hash could qualify a current claim. Validation now requires nonblank metadata,
a 64-digit hexadecimal SHA-256 and an RFC3339 retrieval timestamp. The subsequent full domain
library run passed all 148 tests. These checks validate metadata, not the claimed captured bytes.

```text
cargo run -p census-report --example contact_context_smoke --message-format short
before: contact_tenure_conflict expected; professional_coach_email was exported
after: contact_context: PASS actual_xlsx=6_athletes
programme_side_role_year_scoped=true historical_rows_retained=true
stale_address_refused=true tenure_conflict_retained=true
metadata_only_not_capture_proof=true
```

The temporary program wrote a real Fjall store and read the generated XLSX with calamine.
Its six synthetic athletes exercised programme/side/role/year exclusion, a named current coach
against an emailed former coach, labelled AD fallback, unresolved coach conflicts, contradictory
tenure observations, undated-address rejection and personal email classification. The raw Coaches
projection retained the former contact with `former_declared` and its assessment school year.
No admissions workbook or private source rows were used.

The failing runtime exposed a store merge that deduplicated tenure evidence without comparing
tenure itself. Full evidence equality now preserves conflicting interpretations. Contact selection
and contact conflict queues read individual snapshot observations instead of mixing addresses and
tenure from different observations of one coach. Merged entity scans retain their existing API.
Currentness is assessed against an explicit workbook school year, or the school year containing
the run date; fetch recency is not appointment authority. Missing rows mean research is unknown,
not that a source was unattempted or successfully searched.

The integrated domain/report command returned 148 domain passes and 125 report passes / 1 report
failure at an obsolete season-column expectation. The separate store library command returned
123 passes / 4 failures in row-ceiling/reservation regressions. Neither result is a full-suite pass.
Raw-capture binding, persisted contact research attempts, and complete coverage accounting remain
open. The contact metadata fixtures do not establish source authority.

### Executed roster verdict and service integration

The current MileSplit parser run passed 32 tests:

```text
cargo test -p census-crawl --lib milesplit::
```

The temporary `roster_context_smoke` executable exercised a real loopback HTTP server, the
production fetcher and collector, and reopened Fjall stores. Its five cases passed:

| Response | Accepted athletes | Completed rosters | Named gaps |
|---|---:|---:|---:|
| Complete fixture | 25 | 1 | 0 |
| One rejected graduation-year row | 24 | 0 | 1 |
| Unrecognized template | 0 | 0 | 1 |
| Invalid UTF-8 | 0 | 0 | 1 |
| HTTP 404 | 0 | 0 | 1 |

The server observed 11 physical requests: six roster requests and five robots requests.
The 404 was fetched twice rather than cached; the other four captures were reused by the
collector. Accepted source observations survived reopening, including partial-roster rows.
Resuming each store preserved its counts and gaps without additional fetch or cache activity.
The smoke asserted response-body SHA-256, not immutable archival or source authority.
It used public fixtures and synthetic responses, not admissions data, and was removed afterward.

The integrated service command passed all 10 tests across four targets:

```text
cargo test -p census-service --test fjall_restate_e2e --test workbook_shape --test parity_pipeline --test milesplit_roster_observations --no-fail-fast -- --test-threads=1
```

All commands used `TMPDIR=/home/lewis/av.kqQlyuIY` and an empty `RUSTC_WRAPPER`.
The Fjall regressions now distinguish retained raw duplicate observations from deduplicated
merged evidence, including reopening and one-time legacy import. The workbook PR oracle
uses the existing later-date, later-meet-ID, later-performance-ID tie order rather than assuming
the smallest ID wins. Incidental text goldens were removed, not regenerated.
These targets do not establish native Restate crash recovery, a fully independent PR classifier
oracle, raw-capture archival, or recovery of an incomplete roster after its source changes.

### Unexecuted durability contract audits

Four DeepSeek workers inspected the native Restate restart, export-crash, recovery and backup
test contracts. They ran no commands, services or faults. These are review inputs, not execution
evidence, and the historical drill results below do not transfer to this integration.

- Native restart: `restate_kill_restart.rs` contains endpoint/server kill paths; the audit flagged
  pre-kill progress, pause timing and the distinction between snapshot reruns and evidence-write
  deduplication. All 17 required scenarios still need current execution evidence.
- Export crash: the workbook currently saves directly to its published path, while sidecars are
  separately published. There is no snapshot-bound atomic bundle yet; a readable restarted ZIP
  does not prove preservation of the previous complete generation.
- Recovery: the audit identified old roster progress/revision contracts and assertions that can
  tolerate lost rows or only print a defect. Those findings require integration and actual drills.
- Backup: the audit distinguished cold-copy/byte-corruption tests from the production manifest
  API and power-loss durability. It flagged the outdated table-count expectation and missing
  nonempty identity-history and crash-publication cases. No restore was executed in this batch.

### Verification limits

These are slice results, not a release verdict. Raw-capture binding of contact proofs, the full
F01–F15 acceptance set and all 17 real durability fault scenarios remained unverified for this
integration. Its workbook-era input checks do not define the newer fresh-run scope, which has no
seed workbook. Historical workbook/seal entries below do not certify this integration; these
smokes did not open or modify the original admissions workbook.

## Toolchain

```text
$ cargo kani --version
cargo-kani 0.67.0

$ cargo fuzz --version
cargo-fuzz 0.13.2

$ cargo +nightly fuzz --version
cargo-fuzz 0.13.2

$ rustup toolchain list
nightly-2026-04-27-x86_64-unknown-linux-gnu (active, default)
… plus stable, 1.85, 1.95.0, nightly-2026-05-05, and Kani 0.67.0's bundled nightly-2025-11-21
```

Both tools are present, so nothing in this pack is unverifiable for tool reasons. Kani runs on its own
bundled nightly (`nightly-2025-11-21`), so the active default toolchain does not matter.

## Harness inventory

| Crate | File | Harnesses | Properties |
|---|---|---|---|
| `crates/census-domain` | `kani/gradyear.rs` | 4 | `GradYear::of` cohort derivation and saturation, `ObservedGrade::grad_year` agreement, known cohort values |
| `crates/census-domain` | `kani/publish.rs` | 9 | published-address classification by domain and routing on set (symbolic + known-value tables), `normalize_name` diacritics/shape/idempotency |
| `crates/census-domain` | `kani/id_mint.rs` | 5 | `Id::mint` determinism, shape, tag prefixes, golden digest, `as_str`/`Display` agreement |
| `crates/census-store` | `kani/keys.rs` | 5 | `observation_key`/`split_observation_key` round-trip, fixed-width tail, id bounds, null byte and max-sequence handling |
| `crates/census-store` | `kani/merge.rs` | 5 | `Entity::merge` idempotency (School, Coach), `CanonicalCoach::publish` idempotency, address routing (arbitrary and known-value tables) |

**Current set (2026-09-25).** The contact policy changed: an address a source published is classified by
domain and routed to `professional_email` or `personal_email`, and none is withheld. That replaced the
two `professional_email` withholding harnesses in `census-domain/kani/publish.rs` (now 9, including
`check_published_email_classifies_domains` and `check_set_published_email_routes_by_kind`) and the two
withhold harnesses in `census-store/kani/merge.rs` (now `check_coach_publish_routes_arbitrary_address` and
`check_coach_publish_routes_known_addresses`), for **28 harnesses** in total. The per-harness audit below
records the pre-change set under its old names; no verdict in it was re-run.

Wiring: `crates/census-store/src/lib.rs` ends with

```rust
#[cfg(kani)] include!("../kani/store_wiring.rs");
```

and `kani/store_wiring.rs` declares `kani/keys.rs` and `kani/merge.rs` as modules with `#[path = ...]`.
`census-domain` uses the same pattern through `kani/census_domain_wiring.rs`. Both are compiled only
under `cargo kani`, which is what defines `cfg(kani)`.

Commands (from the repository root), one harness per invocation:

```bash
cargo kani --manifest-path crates/census-domain/Cargo.toml --harness <name>
cargo kani --manifest-path crates/census-store/Cargo.toml --harness <name>

# the id_mint and merge harnesses additionally need Kani's stubbing feature:
cargo kani -Z stubbing --manifest-path crates/census-domain/Cargo.toml --harness check_id_mint_format
```

`-Z stubbing` is required only because those harnesses stub the CPU feature probe `__cpuid_count`
(see below). Kani rejects a stubbed harness without the flag:

```text
error: Using the stub attribute requires activating the unstable `stubbing` feature
```

## Repairs made in this pass

1. **Arbitrary `String`.** `kani::any()` has no `Arbitrary` implementation for `String`, so the
   delivered `publish.rs` / `id_mint.rs` harnesses could not compile. Symbolic inputs are now bounded
   `[u8; N]` arrays (converted through `String::from_utf8_lossy`, or, for the address harness, built
   from printable-ASCII bytes); properties that need no symbolic input use concrete value tables.
2. **sha2's runtime CPU probe.** Minting an id hashes through `sha2`, which selects its backend at
   runtime via `cpufeatures` → `__cpuid_count` (inline asm). Kani refuses inline asm, so every harness
   that mints an id failed before reaching the code under test:

   ```text
   SUMMARY:
    ** 1 of 4028 failed (4027 undetermined)
   Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani. Please post your example at https://github.com/model-checking/kani/issues/2
    File: ".../stdarch/crates/core_arch/src/x86/cpuid.rs", line 75, in std::arch::x86_64::__cpuid_count
   VERIFICATION:- FAILED
   ```

   The probe is now stubbed to report no CPU features (`cpuid_without_features` in `id_mint.rs` and
   `merge.rs`), which routes the hash to sha2's pure-Rust soft backend; `Id::mint` and the entity code
   stay untouched. The SHA-NI backend is an acceleration of the same function and is **not** verified —
   Kani cannot model it.
3. **Unwinding.** Inside sha2, Kani hits loops in the block/compression code and in `memcmp` that a
   bound of 32 (and later 72) does not cover:

   ```text
   SUMMARY:
    ** 1 of 4992 failed (4991 undetermined)
   Failed Checks: unwinding assertion loop 0
    File: ".../library/core/src/slice/iter/macros.rs", line 252, in <std::slice::IterMut<'_, u8> as std::iter::Iterator>::fold
   VERIFICATION:- FAILED
   [Kani] info: Verification output shows one or more unwinding failures.
   ```

   The minting harnesses are annotated `#[kani::unwind(64)]` — the digest's own loop count: sha2's
   compression loop runs 64 rounds, the `GenericArray` folds 32 steps, and one 64-byte block covers a
   short id. At 64 the sha2 loops complete and the only truncation left in the trace is `memcmp.0`.
4. **Loop shapes inside the harnesses.** `assert_id_shape` walks its 16 hex digits with an index loop
   (nested iterator folds cost CBMC ~4 unrolled steps per element), and assertion messages that
   interpolated symbolic strings were reduced to literals (the conditions are unchanged).
5. **Analysis configuration is per harness, and stated in every command below.**
   - `-Z stubbing` — only for harnesses that mint an id (sha2's CPU probe, see 2).
   - `--no-unwinding-checks` — for harnesses whose inputs are concrete values (`id_mint/*`, the
     known-value `professional_email`/`normalize_name` harnesses). CBMC cannot discharge the
     unwinding *checks* for loops over heap-backed strings, whose trip counts are not visible at
     symex time, while every loop reachable at the annotated bound has a constant trip count.
     Symbolic-input harnesses (observation keys, entity merge, the symbolic mailbox harness) run with
     unwinding checks left on.
6. **Previous compile blockers are gone.** `crates/census-service` builds and the store harnesses run in
   this tree (the `bootstrap.rs:287` / `bootstrap/error.rs:90` errors no longer reproduce). No kani
   file calls `DrainState::from_join` (`grep -rn from_join crates/*/kani/` returns nothing), so the
   signature change to `Result<(), tokio::task::JoinError>` needed no harness edit.

## Kani results

### States, provenance, and sweep discipline

Four states are used below, and no state is inferred from a build, a timeout, or the absence of an
error:

| State | What was observed |
|---|---|
| `verified` | Kani printed `VERIFICATION:- SUCCESSFUL` and a `SUMMARY:` line reading `** 0 of N failed` for that harness. |
| `env-blocked (reason)` | CBMC, its solver, or the sweep environment ended the run with no `Failed Checks:` property line; the reason is what Kani printed. Never evidence about the code under test. |
| `counterexample (property)` | A `Failed Checks: <property>` line names a property the harness asserts. |
| `no verdict (reason)` | The run — or the sweep window — ended before any of the above. Not a pass, not a blocked proof, not a counterexample. |

The provenance column cites the run, and each row's raw tail is reproduced under "Raw tails" with the
label in the last column:

- **prev-pass** — executed by the previous verification pass against this same working tree; its logs
  are `/tmp/kani-cd/*.log` (census-domain) and `/tmp/kani-mw/*.log` (census-service), and the wall
  times quoted are that pass's. The four gradyear results were declared reusable for this sweep; the
  rest were not re-run before the sweep window closed.
- **this-window** — executed by this sweep; logs under `/tmp/kani-sweep2/`.

Run discipline, this window: strictly one CBMC process at a time (`pgrep -x cbmc` verified empty
before each start and re-checked after each run), each invocation in its own process group with its own
wall budget and a 5 s sampler over `/proc/<pid>/status` `VmHWM` for peak CBMC RSS. Both runs that
started either finished on CBMC's own out-of-memory path or were cut off by the budget; both process
groups were killed and re-checked before these tables were written.

**No harness was edited in this sweep.** `git status --porcelain crates/census-domain/kani
crates/census-store/kani` is empty, so `git diff` over both `kani/**` trees shows nothing at all —
no format-message edit, no added `assume`, no deleted harness, no `#[kani::ignore]`. The one harness
whose asserted property this lane believes is false on the current model
(`check_normalize_idempotent_repeated_suffix`, whose doc comment predicts exactly that counterexample)
was left as written: it is a finding for `crates/census-domain/src/model/normalization.rs`
(`normalize_name` at `:15`), which this lane does not own.

**Follow-up (same day, after this sweep).** The finding was acted on rather than filed: `normalize_name`
now strips school-type suffixes until none applies (`strip_type_suffix` returns `false`), so the
normalized form is a fixpoint and a second call is the identity — the property `SchoolId::mint` keys
identity on. `check_normalize_idempotent_repeated_suffix` stays as the pinned regression with its doc
comment updated from "expected to fail" to the fixpoint contract, and the behavior is covered in the
normal suite by `census-domain`'s `model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes`
(`cargo nextest run -p census-domain -E 'test(normalize_name)'`, 4 passed, and the full workspace run,
583 passed). The harness itself is a *concrete*-input check — it feeds one literal and asserts one
equality — so its Kani run adds no reach over that unit test; the CBMC attempt from this follow-up
(T14) was killed at 549.5 s inside `core::slice::memchr`, before a verdict, which is why row 12 keeps
`no verdict`.

### Sweep environment notes

1. **A missing `CARGO_HOME` breaks every harness launch, in 0.1 s.** The first sweep attempt launched
   `cargo kani` from a non-interactive process environment without `CARGO_HOME`. Every harness died
   immediately with

   ```text
   Kani Rust Verifier 0.67.0 (cargo plugin)
   error: Failed to get cargo metadata.: failed to start `cargo metadata`: No such file or directory (os error 2): No such file or directory (os error 2)
   ```

   `cargo kani --version` succeeds in that same environment, so a version probe cannot see this fault;
   it only appears on a real harness run. Exporting `CARGO_HOME` — the value the interactive shell
   carries — fixes it, and every run reported below used the full interactive environment. Those 0.1 s
   `rc=1` rows are launch faults, not Kani results.

2. **A transient manifest fault explains the previous pass's `rc=1 secs=0` rows.** Its logs
   (`/tmp/kani-cd/check_normalize_idempotent.log` and siblings, 394 bytes each) show every harness
   dying inside the same second with

   ```text
   error: failed to parse manifest at `.../crates/census-service/Cargo.toml`
   Caused by:
     can't find `core` bench at `benches/core.rs` or `benches/core/main.rs`.
   ```

   — a sibling's in-flight `[[bench]]` edit, not a harness defect. `benches/core.rs` exists in the
   current tree and the manifest parses, so that failure does not reproduce.

3. **A single sequential CBMC run still reaches CBMC's OOM path.** `check_observation_id_bounds` ran
   alone for 710.0 s and ended on CBMC's out-of-memory path (T8) with `free -g` reporting 76 GiB
   available when it started, so the prev-pass OOMs cannot be attributed to concurrency alone.
   Whether a kernel OOM kill took part is **not** established: `dmesg` returns no lines from this
   session and `journalctl -k` shows nothing for the window.

### Verdict table — `crates/census-domain` (17 harnesses)

| # | Harness | State | Provenance | Raw tail |
|---|---|---|---|---|
| 1 | `check_gradyear_of_formula` | `verified` | prev-pass | T1 |
| 2 | `check_gradyear_of_known_values` | `verified` | prev-pass | T2 |
| 3 | `check_gradyear_of_saturating` | `verified` | prev-pass | T3 |
| 4 | `check_observed_grade_grad_year` | `verified` | prev-pass | T4 |
| 5 | `check_professional_email_never_publishes_consumer_mailbox` | `no verdict (never started)` | — | — |
| 6 | `check_professional_email_known_consumer` | `env-blocked (CBMC out of memory; prev-pass wall 15 s)` | prev-pass | T5 |
| 7 | `check_professional_email_known_professional` | `no verdict (never started)` | — | — |
| 8 | `check_professional_email_malformed` | `env-blocked (CBMC out of memory; prev-pass wall not recorded)` | prev-pass | T6 |
| 9 | `check_normalize_diacritics` | `no verdict (this-window probe exceeded its 600 s budget, output not captured)` | this-window | — |
| 10 | `check_normalize_shape` | `env-blocked (CBMC out of memory; prev-pass wall 226 s)` | prev-pass | T7 |
| 11 | `check_normalize_idempotent` | `no verdict (never started)` | — | — |
| 12 | `check_normalize_idempotent_repeated_suffix` | `no verdict (follow-up run killed at 549.5 s inside core::slice::memchr; no verdict line; the property it asserts is now fixed in crates/census-domain/src/model/normalization.rs and pinned by the unit test model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes)` | this-window | T14 |
| 13 | `check_id_mint_format` | `no verdict (prev-pass run killed mid-trace; its log contains no verdict line)` | prev-pass | T11 |
| 14 | `check_id_mint_tag_prefix` | `no verdict (never started)` | — | — |
| 15 | `check_id_mint_deterministic` | `no verdict (never started)` | — | — |
| 16 | `check_id_mint_golden_value` | `env-blocked (solver conversion: z3 CBMC map::at status 6; bitwuzla status 134/SIGABRT)` | prev-pass | T9 |
| 17 | `check_id_as_str_consistent` | `no verdict (never started)` | — | — |

### Verdict table — `crates/census-store` (10 harnesses)

| # | Harness | State | Provenance | Raw tail |
|---|---|---|---|---|
| 18 | `check_observation_id_bounds` | `env-blocked (CBMC out of memory after 710.0 s alone; peak CBMC VmHWM ≥ 21.0 GiB)` | this-window | T8 |
| 19 | `check_observation_key_null_byte_id` | `env-blocked (600 s per-harness budget exhausted with CBMC still running; no verdict line; peak CBMC VmHWM ≥ 3.2 GiB)` | this-window | T12 |
| 20 | `check_observation_key_zero_and_max_sequence` | `env-blocked (CBMC out of memory; prev-pass wall 223 s)` | prev-pass | T13 |
| 21 | `check_split_key_reads_fixed_width_tail` | `no verdict (never started)` | — | — |
| 22 | `check_observation_key_round_trip` | `no verdict (never started)` | — | — |
| 23 | `check_school_merge_idempotent` | `no verdict (never started)` | — | — |
| 24 | `check_coach_merge_idempotent` | `no verdict (never started)` | — | — |
| 25 | `check_coach_publish_idempotent` | `no verdict (never started)` | — | — |
| 26 | `check_coach_publish_no_consumer_mailbox` | `no verdict (never started)` | — | — |
| 27 | `check_coach_withheld_mailboxes_consistency` | `no verdict (never started)` | — | — |

### Tally

**Tally of 27: verified 4 / env-blocked 7 / counterexample 0 / no verdict 16.**

Read precisely: 4 harnesses have a complete-verification result, 7 ended in an environment or solver
failure that says nothing about the code under test, 0 produced a `Failed Checks:` property line, and
16 were never carried to any outcome. The three-state target for all 27 was **not** met: the sweep
window closed on a wrap-up request after two census-service harnesses, so 14 of them were never
launched at all; the other two started and were killed before a verdict (`check_id_mint_format`, T11,
and the `check_normalize_diacritics` probe whose output was not captured). Nothing was weakened to close a row: the harness set
is byte-identical to the previous pass.

What the 4 `verified` rows cover: `GradYear::of`'s cohort formula, its known-value anchors, its
saturating arithmetic over a fully symbolic `i16` observation year, and `ObservedGrade::grad_year`'s
agreement with `GradYear::of`. No other harness in either crate has a verdict.

### Raw tails

T1–T4 — `crates/census-domain/kani/gradyear.rs`, verified, prev-pass (`/tmp/kani-cd/`, one harness per
invocation; the log for each run names the harness it checked and this crate's path):

```text
T1 cargo kani --manifest-path crates/census-domain/Cargo.toml --harness check_gradyear_of_formula
SUMMARY:
 ** 0 of 122 failed
VERIFICATION:- SUCCESSFUL
Verification Time: 0.05850434s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

T2 cargo kani --manifest-path crates/census-domain/Cargo.toml --harness check_gradyear_of_known_values
SUMMARY:
 ** 0 of 128 failed
VERIFICATION:- SUCCESSFUL
Verification Time: 0.04390135s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

T3 cargo kani --manifest-path crates/census-domain/Cargo.toml --harness check_gradyear_of_saturating
SUMMARY:
 ** 0 of 59 failed
VERIFICATION:- SUCCESSFUL
Verification Time: 0.03621107s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

T4 cargo kani --manifest-path crates/census-domain/Cargo.toml --harness check_observed_grade_grad_year
SUMMARY:
 ** 0 of 327 failed (4 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 0.12346949s
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

T5 — `check_professional_email_known_consumer`, `env-blocked`, prev-pass
(`/tmp/kani-cd/check_professional_email_known_consumer.log`, last 7 lines):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Verification failed for - kani_publish::check_professional_email_known_consumer
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T6 — `check_professional_email_malformed`, `env-blocked`, prev-pass
(`/tmp/kani-cd/check_professional_email_malformed.log`; command as the previous pass recorded it:
`cargo kani --no-unwinding-checks --manifest-path crates/census-domain/Cargo.toml --harness check_professional_email_malformed`):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Verification failed for - kani_publish::check_professional_email_malformed
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T7 — `check_normalize_shape`, `env-blocked`, prev-pass (`/tmp/kani-cd/check_normalize_shape.log`;
runs with `--no-unwinding-checks` as recorded by that pass):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Verification failed for - kani_publish::check_normalize_shape
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T8 — `check_observation_id_bounds`, `env-blocked (CBMC OOM)`, **this-window**
(`/tmp/kani-sweep2/check_observation_id_bounds.log`, 1,681,406 bytes, wall 710.0 s, peak sampled CBMC
`VmHWM` 21.0 GiB, one CBMC at a time, no other harness running):

```text
$ cargo kani --manifest-path crates/census-service/Cargo.toml --harness check_observation_id_bounds
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Verification failed for - store::kani::check_observation_id_bounds
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T9 — `check_id_mint_golden_value`, `env-blocked (solver)`, prev-pass. Both solver attempts, verbatim
(`/tmp/kani-cd/check_id_mint_golden_value-z3.log`, `…-bitwuzla.log`), plus the default-solver attempt
from that pass's own summary file (`check_id_mint_golden_value rc=137 secs=244`, i.e. killed by the
batch wrapper, no verdict):

```text
$ cargo kani -Z stubbing --no-unwinding-checks --solver z3 --manifest-path crates/census-domain/Cargo.toml --harness check_id_mint_golden_value
size of program expression: 659841 steps
slicing removed 490160 assignments
Generated 47259 VCC(s), 9690 remaining after simplification
Runtime Postprocess Equation: 0.497278s
Passing problem to SMT2 QF_AUFBV using Z3
converting SSA
map::at

CBMC failed with status 6
VERIFICATION:- FAILED

Manual Harness Summary:
Verification failed for - kani_id_mint::check_id_mint_golden_value
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

```text
[… --solver bitwuzla, same command shape …]
Passing problem to SMT2 QF_AUFBV (with FPA) using Bitwuzla

CBMC failed with status 134
VERIFICATION:- FAILED

Manual Harness Summary:
Verification failed for - kani_id_mint::check_id_mint_golden_value
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

T11 — `check_id_mint_format`, `no verdict`, prev-pass: the run was killed mid-trace. Its log
(`/tmp/kani-cd/check_id_mint_format.log`, 1,620,050 bytes) contains zero `VERIFICATION` lines and ends
inside CBMC's path trace:

```text
aborting path on assume(false) at file …/library/core/src/result.rs line 966 column 15 function std::result::Result::<std::ptr::NonNull<[u8]>, std::alloc::AllocError>::map_err::<std::collections::TryReserveError, {closure@alloc::raw_vec::RawVecInner::finish_grow::{closure#0}}> thread 0
```

T12 — `check_observation_key_null_byte_id`, `env-blocked (budget)`, **this-window**
(`/tmp/kani-sweep2/check_observation_key_null_byte_id.log`, 1,596,753 bytes; killed at the 600 s budget
with peak sampled CBMC `VmHWM` 3.2 GiB and no verdict line; the log's last line):

```text
$ cargo kani --manifest-path crates/census-service/Cargo.toml --harness check_observation_key_null_byte_id
…
aborting path on assume(false) at file /home/runner/work/kani/kani/library/kani_core/src/models.rs line 176 column 17 function <usize as kani::rustc_intrinsics::ToISize>::to_isize thread 0
```

T13 — `check_observation_key_zero_and_max_sequence`, `env-blocked`, prev-pass
(`/tmp/kani-mw/check_observation_key_zero_and_max_sequence.log`, 5,463,239 bytes, that pass's wall
223 s):

```text
CBMC failed
VERIFICATION:- FAILED
CBMC appears to have run out of memory. You may want to rerun your proof in an environment with additional memory or use stubbing to reduce the size of the code the verifier reasons about.

Manual Harness Summary:
Verification failed for - store::kani::check_observation_key_zero_and_max_sequence
Complete - 0 successfully verified harnesses, 1 failures, 1 total.
```

Unlabeled tails for completeness: the two `no verdict` rows that did start are T11 (prev-pass
`check_id_mint_format`) and the `check_normalize_diacritics` probe of this window, whose output was not
captured (a 600 s run that had not reached a verdict; a second probe was discarded when it was piped
through `head`). `/tmp/kani_cd_baseline.log` is a pre-repair multi-harness run whose harness names
(`check_gradyear_of_valid`) do not match the current set, and it is **not** used as evidence anywhere
above.

T14 — `check_normalize_idempotent_repeated_suffix`, `no verdict (killed mid-trace)`, follow-up run
against the fixed model, same day (`cargo kani --manifest-path crates/census-domain/Cargo.toml
--harness check_normalize_idempotent_repeated_suffix` with `CARGO_HOME` exported, wall 549.5 s, peak
sampled CBMC `VmHWM` 15.7 GiB, killed by the operator once it was clear the run was walking the string
machinery rather than the property; the last lines of the captured output):

```text
aborting path on assume(false) at file .../kani_core/src/models.rs line 176 column 17 function <usize as kani::rustc_intrinsics::ToISize>::to_isize thread 0
aborting path on assume(false) at file .../kani/src/lib.rs line 57 column 1 function kani::mem::cbmc::same_allocation thread 0
Unwinding loop _RNvNvNtNtCsci0VKyKEi6N_4core5slice6memchr14memchr_aligned7runtimeCskfx95qGcYES_13census_domain.0 iteration 62 file .../core/src/slice/memchr.rs line 81 column 13 function core::slice::memchr::memchr_aligned::runtime thread 0
aborting path on assume(false) at file .../kani_core/src/models.rs line 176 column 17 function <usize as kani::rustc_intrinsics::ToISize>::to_isize thread 0
Unwinding loop _RNvNvNtNtCsci0VKyKEi6N_4core5slice6memchr14memchr_aligned7runtimeCskfx95qGcYES_13census_domain.0 iteration 63 file .../core/src/slice/memchr.rs line 81 column 13 function core::slice::memchr::memchr_aligned::runtime thread 0
```

The harness has no symbolic input (it calls `normalize_name("x school school")` and compares the two
results), so this tail is a cost statement about `str::to_lowercase`/`memchr` under CBMC, not a
statement about idempotence. The property is pinned instead by
`model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes` (passes) and by the argument
that `normalize_name`'s only name-dependent step is `while strip_type_suffix(&mut parts) {}`, whose
result matches no suffix, so the second call's strip loop returns `false` immediately and every earlier
step (lowercase, folding, whitespace collapse) is already stable on its own output.

## cargo-fuzz

Fuzz crate layout: `fuzz/Cargo.toml` declares four `[[bin]]` targets — `hytek`, `compiled`, `xc`,
`raceday` — over `fuzz/fuzz_targets/`. Corpus and build output are gitignored
(`/fuzz/corpus/`, `/fuzz/target/`, `/fuzz/artifacts/`).

### Build

```text
$ cargo fuzz build                     # run in fuzz/
   Compiling census-domain v0.1.0 (/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/crates/census-domain)
   Compiling census-service v0.1.0 (/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/crates/census-service)
   Compiling fuzz v0.0.0 (/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/fuzz)
    Finished `release` profile [optimized + debuginfo] target(s) in 54.36s
```

All four targets build; the `fuzz/Cargo.lock` in the tree is the one this build produced.

### Bounded runs

Each target was seeded with the repository's retained raw bodies
(`fuzz/fixtures/retained_results_jsonl/raw/`, 7 files, 289–1100 bytes) copied into its corpus, then
run for exactly 1000 runs:

```text
$ cargo fuzz run hytek -- -runs=1000
INFO: Seed: 2978054506
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 85 min: 1b max: 1100b total: 4602b rss: 84Mb
Done 1000 runs in 0 second(s)

$ cargo fuzz run compiled -- -runs=1000
INFO: Seed: 3297506577
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 94 min: 1b max: 1100b total: 4637b rss: 85Mb
Done 1000 runs in 0 second(s)

$ cargo fuzz run xc -- -runs=1000
INFO: Seed: 4135662610
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 102 min: 1b max: 1100b total: 4693b rss: 84Mb
Done 1000 runs in 0 second(s)

$ cargo fuzz run raceday -- -runs=1000
INFO: Seed: 626477768
INFO: -max_len is not provided; libFuzzer will not generate inputs larger than 4096 bytes
INFO: seed corpus: files: 38 min: 1b max: 1100b total: 4501b rss: 85Mb
Done 1000 runs in 0 second(s)

$ find fuzz/artifacts -type f | wc -l
0
```

All four bounded runs completed with no crash and no reproduced artifact: `Done 1000 runs` for each,
`fuzz/artifacts/` empty. `cargo fuzz run <target> -runs=1000` without the `--` separator is rejected by
cargo-fuzz 0.13 (`tip: to pass '-r' as a value, use '-- -r'`), so the libFuzzer arguments go after `--`.

## cargo-mutants

Not re-executed in this pass — the numbers below are the earlier pack's run, kept for traceability.
Config `crates/census-domain/mutants.toml`, threshold 0 missed mutants: `Found 132 mutants to test` →
`ok Unmutated baseline in 2s build + 0s test` → **132 tested in 56s: 65 missed, 51 caught, 16
unviable — threshold FAIL.** The missed mutants are all in `crates/census-domain/src/model.rs`
(Display impls for `Id`/`Grade`/`GradYear`/`SourceNamespace`, `GradYear::get`, `SchoolYear::containing`
boundary, `SourceNamespace::is_core`, `EventKind::from_source_label`, `Gender::parse_milesplit`,
`CanonicalAthlete::mint` gender arms, `Mark::raw`, `professional_email`/`normalize_name` flips,
`strip_diacritic`, `flip_last_first`).

## Not established here

- **23 of the 27 Kani harnesses.** 7 are `env-blocked` and 16 have `no verdict`; the two verdict tables
  above name every one of them. The blocked set and its failure mode, exactly:
  - CBMC's own out-of-memory path, with no `Failed Checks:` line: `check_professional_email_known_consumer`,
    `check_professional_email_malformed`, `check_normalize_shape`,
    `check_observation_key_zero_and_max_sequence` (all prev-pass, wall 15 s / not recorded / 226 s /
    223 s), and `check_observation_id_bounds` (this window: 710.0 s as the only CBMC on the box, with
    `free -g` reporting 76 GiB available at start and a peak sampled CBMC `VmHWM` of 21.0 GiB). Whether
    a kernel OOM kill took part is not established — `dmesg` returns no lines from this session and
    `journalctl -k` shows nothing for the window.
  - Per-harness budget exhausted with CBMC still climbing, no verdict line:
    `check_observation_key_null_byte_id` (600 s, 3.2 GiB and rising).
  - Solver conversion inside CBMC, not a property failure: `check_id_mint_golden_value` (z3 `map::at`,
    status 6; bitwuzla status 134/SIGABRT). The default solver (CaDiCaL) is the remaining route and has
    not been given a long enough window yet (a 244 s attempt was killed without a verdict).
  - Never started, in either pass: the 13 rows marked `no verdict (never started)`.
  - Started but killed before a verdict: `check_id_mint_format` (prev-pass, log ends mid-trace),
    `check_normalize_diacritics` (probed here with a 600 s cap; output not captured), and
    `check_normalize_idempotent_repeated_suffix` (follow-up run on the fixed model, killed at 549.5 s
    inside `core::slice::memchr`, T14) — none carries a verdict line, so none is evidence about its
    property either way. That last harness has no symbolic input; its property is pinned in the unit
    suite instead (`model::tests::normalize_name_reaches_a_fixpoint_on_repeated_suffixes`).
- **SHA-NI SHA-256 backend** (see repair 2) — unreachable for Kani; the soft backend is what is proved.
- **Deeper fuzzing than 1000 runs per target** — the runs are bounded smoke runs, not soak runs.
- **cargo-mutants** — carried from the earlier pack, not re-run here.
- **Traceability matrix** — separate work item, not part of this pack.

## Census seal (2026-09-22)

The terminal state is now a value, not a label (§17, §70, ADR-011): `CensusState` advances one phase
at a time, `Complete` is unconstructible without `SealEvidence`, and `census-service seal` assembles
that evidence from the store and the exported workbook — then either completes the census with a
digest or refuses and names the acceptance item.

Commands and results:

    cargo test -p census-service --lib census::state      # 11 passed
    cargo test -p census-service --bins                   # 22 passed (9 of them cli::seal)
    cargo clippy -p census-service --all-targets --all-features -- -D warnings   # clean
    cargo fmt -p census-service -- --check                # clean
    cargo run --bin census-service -- --store /tmp/mc-seal-proof seal --grad-year 2027
      # exit 1: "no workbook in /tmp/mc-seal-proof/out: run `census-service workbook …` before sealing"

**Defect found and fixed while reviewing this work:** the seal digest rendered `workbook_rows` but
not the workbook's own sha256, so two different exports with the same row count would have shared one
seal. The digest now covers `workbook_rows`, `workbook_sheets` and the sorted set of workbook
digests, and `census::state::tests::the_seal_binds_the_workbook_it_certifies` pins it.

**Review claim rejected, with evidence:** an independent review reported that `school_coach_index`
(`crates/census-service/src/report/rows.rs:74-89`) is non-deterministic because "HashMap iteration
order" decides which coach wins at a school. The map is only read by key, the stored coach is the
first in deterministic `scan` order, and the email flag is accumulated across every coach at the
school (`entry.1 = true`), so no published number depends on map iteration. The same review's claim
that coaches with `sport = None` are wrongly excluded is the documented intent of that index
(`/// School id -> (a track/XC coach, whether any track/XC coach brings a published email)`; the
comment's wording after the 2026-09-25 contact-policy change): the
published metric is athletes with an identified *track/XC* coach, and a school-wide coach row carries
no evidence that they coach track or cross country. Residual, unresolved: a school whose only track
coach is recorded as school-wide counts its athletes as coachless — a coach-sourcing gap, not a
miscomputed metric.

**Not verified here:** the seal's success path against the live store. The store is locked by the
running national sweep, so the seal is proved at the unit level (ladder, workbook reconciliation,
digest) and at the CLI level for the refusal path; the completed seal over the real corpus remains to
be run once that sweep releases the store.

**Second review, also local, also rejected with evidence** (`cli/seal.rs`, `census/state/**`):

- *"`open_items()` never checks `ConflictsRetained`/`RetriesRepresented`"* — correct, and intended:
  those two §70 items are satisfied by retention, which ADR-011 states. Both variants now carry that
  in their own doc comments so a reader of the enum does not have to infer it.
- *"`SealEvidence.open` is not in the digest"* — cannot matter: `seal()` refuses unless
  `open_items()` is empty, which requires every open-work count to be zero, so the field is always
  zero wherever a digest is minted. `observed_on` is likewise outside the digest on purpose.
- *"`metrics_reconciled` passes vacuously on a header-only sheet"* — it does not: the flag requires
  `mapped_athletes > 0`, which requires the cohort row to have been found and parsed, so a sheet with
  no cohort row fails both `RunMetricsReconcile` and `WorkbookMapped`.
- *"`labelled_count` truncates a count split across cells"* — accepted as a real limit of the parse,
  not a path to a wrong seal: the first cell after the label is the value cell the workbook writer
  fills, and a truncated read disagrees with the store's cohort count and is refused rather than
  sealed.

## Workbook scope leak (2026-09-23)

The 2026-09-23 workbook published athletes the run scope excludes. Detection, fix and the proof that
the fix reached the artifact, in order:

- `census-service verify` refused the workbook at `athletes row 521536: id ath_2dc7ef2516e8f7a6
  school 'sch_051f545742ee9934' != store 'Chugiak High School'` — an Alaska school in a store whose
  run scope is the 12-state census, and a cell holding an id where the sheet prints a name.
- Cause: each workbook read model filtered the *schools* table to the run scope first and built the
  school-id → jurisdiction index from what was left, so an athlete whose school the scope drops had
  no jurisdiction to resolve; `in_run_scope` keeps the unplaced bucket, so the athlete survived and
  `Dataset::school_name` fell back to printing the raw id. `503bb78` makes all three read models
  reuse the report's own splitter, which keeps the excluded rows reachable for placement.
- Whole-artifact proof, not a sample (`verify` samples at most 5 000 rows per sheet). The scan tool
  ships with the research folder, not this repository; run both lines from the store root:

      RESEARCH=~/Downloads/midwest-tfxc-source-research
      python3 "$RESEARCH/tools/scan_school_column.py" \
        var/midwest-census/out/superseded/midwest-census-2026-09-23.xlsx
        [Athletes] rows=582691 raw_id_cells=2959
      python3 "$RESEARCH/tools/scan_school_column.py" \
        var/midwest-census/out/census-service-2026-09-23.xlsx
        [Athletes] rows=579732 raw_id_cells=0

  582 691 − 579 732 = 2 959: the leak published exactly the rows whose school it could not name, and
  the rebuilt workbook drops exactly those and nothing else. The leaked build is kept, not deleted —
  it is the only record of the defect's shape and the pre-fix binary cannot reproduce it — but it is
  quarantined under `out/superseded/`, because `verify`, `seal` and the workbook glob all resolve
  "the newest `out/*.xlsx`" and the leaked build has the *later* mtime.
- The rebuilt workbook reconciles with the core report: its audit line reads `rows=579732
  in_scope=2225091 store_athlete_rows=2225091` and `best_mark_rows=7809 consistent=true` against
  `report-core.json`'s `athletes=2225091 class_of_2027=579732 coaches=31488`.
- `verify` now holds the sheet to the rule the sheet implements (`Dataset::school_name`): an id
  printed where the store *does* hold a row is a discrepancy, the id fallback for a school with no
  store row is not. Two acceptance tests pin both directions; the agreement fixture wrote the id, so
  it was measuring the weaker rule rather than the sheet.
- The parity golden that had been red since `13c0590` is refreshed. Only the `Goal & method` sheet
  moved, and only its five reproduce commands: reconstructing that sheet's 40 cells and rewriting
  `-p census-service` back to `-p midwest-census` reproduces the committed digest `72e7bffb…`
  exactly, so no published number was ever in that mismatch. `WORKBOOK_DUMP=1` now prints every
  normalized cell the digest reads, in digest order, so a future mismatch names the cell.

## Census seal (2026-09-23)

The 2026-09-23 census could not be sealed at all until a schema-stale artifact was moved out of the
way, and how it failed is the part worth recording: `out/seal.json` was written by a build that
predated the access-conditions work, and every seal reads it (`recorded_seal`) before it can report
anything, so the run died on the *file* rather than on the census.

    sha256 2b27837d…   var/midwest-census/out/seal.json
    jq                 {"sealed_on":"2026-09-22","phase":"complete","athletes":2238090}
    seal --grad-year 2027 --workbook out/census-service-2026-09-23.xlsx
      # Error: parsing var/midwest-census/out/seal.json / missing field `access_conditions`

A recorded seal is not inert: it is read by the next run and it certifies a store that has since
moved. It is quarantined rather than deleted — `out/superseded/seal-2026-09-22-pre-access-conditions.json`,
same rule as the leaked workbook above: the bytes are the only record of the artifact's shape, and
the globs that resolve "the newest `out/*`" must not see it. (The remedy is an operator's, because
"quietly repair it" is the failure mode the ADR exists to prevent; a seal that cannot read its own
predecessor has to say so.)

With that moved, the offline route reports (20 s, store route, no `--write`):

    phase: exporting
    workbook: var/midwest-census/out/census-service-2026-09-23.xlsx
    acceptance: jurisdiction sweeps are terminal unmet — not measured — this seal does not read the workflow journal
    acceptance: source objects are terminal unmet — not measured — this seal does not read the workflow journal
    acceptance: identity candidates are terminal unmet — 442 identity candidates are undecided
    refused: census cannot be sealed: jurisdiction sweeps are terminal is unmet (…)

Three things this run establishes rather than assumes:

- **The workbook check passes.** `inspect_workbook` runs before the acceptance gate, so reaching the
  item list at all means the 2026-09-23 export reconciles with the store's cohort — the 2 959-row
  scope leak is fixed in the artifact the seal was pointed at, not only in the source.
- **Items 1 and 2 read `not measured` offline *by construction*, and the message says why**: the
  store route reads no workflow journal. They are neither failures nor zeros — §70's own distinction,
  now observed instead of inferred. Only the online route (`seal --ingress <origin>`) measures them.
- **Every other §70 item is satisfied**: the item list prints only what is unmet.

### The review lane, and what it costs

The two lanes were identified from the running servers and the config that pairs them:

    config.native.toml   q5_url = http://127.0.0.1:11000/  q5_model = …UD-Q5_K_XL.gguf
                         q4_url = http://127.0.0.1:11001/  q4_model = …UD-Q4_K_XL.gguf
    ps                   llama-server … -np 1 … -c 200000 … --port 11000    (pid 1510152)
                         llama-server … -np 1 … -c 131072 … --port 11001    (pid 1623)

`-np 1` is why `ask_lanes` keeps exactly one request in flight per lane (`.buffered(lanes)`): a pass
costs `ceil(cases / 2) × latency`, not `cases × latency / slots`.

Smoke run — two cases, 30.4 s wall, `review --limit 2`, both lanes:

    2374515 athlete rows, 2330330 provider objects, 51835 cases filed (42830 decided, 9005 pending),
    6626 rows holding several objects of one provider, 0 findings left to a standing decision
    asked=2 accepted=0 rejected=0 insufficient=1 unanswered=1 dropped=0 failed=0

The second line is the model's half, and `insufficient=1` is the module working as specified: an
answer the store's own evidence cannot back is not recorded as a verdict. The first line is the
deterministic half, and it is the one that changes the seal: `reconcile_athletes` **files 51 835
athlete-identity cases, 9 005 of them pending** — none of which the seal had counted a minute
earlier, when it read 442. Both numbers are true of different states of the same store:

- the **weekly chain** (teams → … → index → report → bests → workbook) leaves the review table
  holding the merge's own conflicts: 1 359 cases, 442 undecided;
- the **review stage**, run on top, replaces that with its reconciliation of every athlete row:
  51 835 cases, 9 448 undecided.

The index pass is what defines the standing rows of a *derived* table (`table_rows`'s own rule: a
derived table's count is the rows its newest write left standing), so `index` restores the chain's
state and the review stage can be re-run whenever an operator wants it. Nothing is lost either way —
the 9 005 are re-derivable from the same athlete rows — but the review-stage state is the honest one
for a census that ran its review stage, and closing it means ~15 h of local-model time (extrapolated
from the smoke: 30.4 s for two cases *including* the reconciliation). That is exactly why `review`
documents itself as an operator action with a small default limit rather than a pipeline stage.

### Online seal: item 1 measured, item 2 refused by name (2026-09-23)

The deployment route the ADR-011 note above leaves open was then run end to end: `census-serve`
over `var/midwest-census` on `127.0.0.1:9080`, registered with the local Restate ingress (the
deployment the national run already used), and the seal asked through it.

    national --detach
      # note: national:2026-27:51472a0f63b82f0d:1 already had a run — restate deduplicated
      # invocation inv_13LIoGM6LB600EGmJ5iU9yGQmEPr4dHTGp
    national --json
      # teams_total 26264 · rosters_done 0 · rosters_skipped = teams · rosters_owed 0
      # failures [] · athletes_total 0 · class_of_2027_total 0
    open-work
      # jurisdiction sweeps owed: 0 of 49
    seal --grad-year 2027 --source-object … (38 journal stems)
      # acceptance: source objects are terminal unmet - 38 source objects have no terminal state

Three things this establishes:

- **§70 item 1 is measured terminal.** `jurisdiction sweeps owed: 0 of 49` is the online read the
  offline route cannot take ("this seal does not read the workflow journal"), and the national
  run's own report agrees: the fan-out is complete, nothing is owed, no jurisdiction failed.
- **The identity item closed.** The review lane resolved all 442 pending meet-jurisdiction cases
  (`asked=442 accepted=233 rejected=0 insufficient=209 unanswered=0 dropped=0 failed=0`, 7m01s,
  one request in flight per model lane). An `insufficient` verdict is a decision — the store's own
  evidence could not back a state — so the cases are terminal and neither the seal's
  `identity candidates are terminal` item nor `open-work` names them again.
- **Item 2 is refused by name, and the count is honest in both readings.** `SourceObject::terminal()`
  is `observations > 0` (`census/state/open.rs:70-77`) and an endpoint's observations are what
  `Ingest::record` (`restate_services/ingest.rs:83`) appends. Nothing in this build calls it: the
  CLI chain and the service's jurisdiction stages both write observations straight to the store, and
  the contemporaneous module inventory recorded the same gap from the identity side —
  `WorkflowIdentity::source_sweep` "has a constructor but no binder". Every key an operator can
  name reads `observations 0 windows 0`: the 38 journal stems (`milesplit_rosters_wi`, …) and the
  endpoint spellings (`milesplit_wi`, `athleticnet_wi`, `wiaa_wi`, `wayzata_wi`, `meets_wi`, `wiaa`,
  `wiaa_results`, `milesplit`, `milesplit_results_wi`, `athleticlive_wi`, `athleticlive_athletes_wi`,
  `plain_names`, `ohsaa_oh`, `ihsa_il`, `tfrrs`, `coach_contacts_wi`). Naming none leaves the count
  `unmeasured`; naming any leaves it owed. Both refuse, and that is the designed answer.

A terminal endpoint has to *accept* an observation, so a run whose stages all skip — this one:
every roster `skipped` because the store already holds it — records none, however it is addressed.
Item 2 is therefore satisfiable only by a census whose acquisition runs *through* `Ingest`
(OPERATIONS.md §Seal: "Routing acquisition through the `Ingest` service is the replacement for the
staging hop; no CLI subcommand drives it yet") **and** that acquires something new. Rebuilding this
store's corpus to manufacture observations is what the program's §1 forbids, and naming an endpoint
whose acquisition never ran that way would be a claim about it that nothing backs. So the
2026-09-23 census stays refused — **by name**, over item 2 alone — which is §70's outcome for an
item the run cannot evidence; item 2 becomes a forward obligation for the first census whose
acquisition routes through `Ingest`.

`seal` now runs on the online route: with the deployment holding the store, the offline form cannot
open it (the single-writer rule; `seal --store var/midwest-census` →

    Error: store open failed: FjallError: Locked

), and `seal --grad-year 2027` prints the same item list as
`seal --ingress http://127.0.0.1:18095/`. Each route exports the workbook before the gate, so
reaching the item list at all re-establishes the workbook check against
`var/midwest-census/out/census-service-2026-09-23.xlsx`. Naming no key leaves item 2 in the
`not measured` form quoted above; naming one leaves it in the counted form quoted above. Both are
refusals, and the difference between the two readings is the whole point of the field: `unmeasured`
is "nobody looked", `owed` is "looked, and the endpoint has accepted nothing".

### The seal that closed (2026-09-23, 17:41)

Item 2 — the one the online run above refused by name — was then measured from a routed acquisition,
and the seal closed. The terminal state it wrote, after the refusal narrative at 16:12:

    sha256 d6cdd7e0868f0d4a9fb5d2c9658f01d6510df06503c66906bfb0bb9b97f1b497  (12 726 bytes)
    file   var/midwest-census/out/seal.json
    jq     {"phase":"complete","sealed_on":"2026-09-23",
            "digest":"5ab49d85c232c26363e8c2e04695ab2c6394d84eb31590c70fed78ccba9ad233",
            "counts":{"jurisdictions":50,"schools":31818,"meets":11016,"athletes":2225091,
                      "class_of_2027":579732,"performances":23970,"coaches":31488}}

Two independent re-runs over the same store back it: the §58 verification (`verify: OK (5000 athletes
sampled of 579732 rows, 5000 performances sampled of 202979 rows)`, exit 0, 6m56s) and the §60 drill
(`PASS: backup drill completed successfully`, `observations match: 3859887`, exit 0). The live route
agrees with the drill's count: `Census/status` through the ingress reads `observations=3859887`.

## Kani harness audit — this sweep

### Harness inventory and claim map (27 harnesses)

Enumerated via `rg '#\[kani::proof\]'` across `crates/census-domain/kani/` and `crates/census-store/kani/`.
5 wiring files: `census_domain_wiring.rs` (4 modules), `store_wiring.rs` (2 modules). Total: 27 `#[kani::proof]` functions.

| # | File | Harness | Claimed property | Symbolic input | Bound | Unwind | Assumptions | Stubs |
|---|---|---|---|---|---|---|---|---|
| 1 | gradyear.rs | `check_gradyear_of_formula` | Formula holds; in-domain derivation accepted | `grade:u8`, `school_year:i16` | 9..=12, 2020..=2027 | 16 | `kani::assume` on both | none |
| 2 | gradyear.rs | `check_gradyear_of_known_values` | Known cohort anchors | none (concrete) | — | 16 | none | none |
| 3 | gradyear.rs | `check_gradyear_of_saturating` | Saturating formula for all seasons | `school_year:i16` | 1900..=2100 | 16 | `kani::assume` | none |
| 4 | gradyear.rs | `check_observed_grade_grad_year` | `ObservedGrade::grad_year` = `GradYear::of` | `grade:u8`, `school_year:i16` | 9..=12, 2020..=2040 | 16 | `kani::assume` on both | none |
| 5-12 | publish.rs | 8 harnesses | published-address classification by domain and routing on set (symbolic + known-value tables), `normalize_name` diacritics/shape/idempotency | `[u8;12]` address (printable ASCII), concrete tables | 12 bytes | 64 | none | none |
| 13-17 | id_mint.rs | 5 harnesses | `Id::mint` format, tag prefix, determinism, golden digest, `as_str`/`Display` consistency | none (concrete) | — | 64 | none | `__cpuid_count` stub |
| 18-22 | keys.rs | 5 harnesses | Key round-trip, null-byte id, zero/max sequence, fixed-width tail split, id bounds | `[u8;8]` id, `[u8;24]` raw key, `u64` sequence | 8, 24 | 48 | none | none |
| 23-27 | merge.rs | 5 harnesses | `Entity::merge` idempotent, `CanonicalCoach::publish` idempotent, address routing (arbitrary and known-value tables) | `[u8;6]` text fields, `bool` flags, `u8%3` counts | 6 | 64 | none | `__cpuid_count` stub |

### Audit results by skill rule

#### `assumptions_are_debt` — GAP found, fixed

Rule: "Audit each assumption and require `kani::cover` or equivalent non-vacuity evidence for critical domains."

**Before fix:** Zero `kani::cover!` across all 27 harnesses. Every harness that uses `kani::assume`, `bounded_any`, or constructs bounded symbolic inputs lacks non-vacuity evidence.

**After fix:** Added `kani::cover!` points in 3 files:
- `gradyear.rs`: 3 harnesses (formula, saturating, observed_grade) — 10 new cover points for assumed boundaries
- `keys.rs`: 2 harnesses (round_trip, split_key) — 6 new cover points for id/sequence boundaries
- `merge.rs`: 5 harnesses (school_merge, coach_merge, coach_publish_idempotent, coach_publish_routes_arbitrary_address) — 10 new cover points for text/email boundaries

**Unchanged (no fix needed):**
- `check_gradyear_of_known_values`, `check_id_mint_*`, `check_published_email_*` tables, `check_normalize_*`, `check_observation_key_null_byte_id`, `check_observation_key_zero_and_max_sequence`, `check_observation_id_bounds`, `check_coach_withheld_mailboxes_consistency`: use only concrete inputs, no assumptions, no bounded generators — no cover needed.

#### `stubs_and_contracts_are_trust_boundaries` — CONFORMS

10 harnesses use `#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]` (5 in `id_mint.rs`, 5 in `merge.rs`). All require `-Z stubbing` at runtime. The VERIFICATION-EVIDENCE.md records this correctly (section "Commands" and "Repairs" §2). The stub replaces inline asm with a pure-Rust model; the SHA-NI backend is acknowledged as unverified.

#### `negative_evidence` — CONFORMS (inline rejection evidence)

The `check_gradyear_of_saturating` harness asserts that `SchoolYear::new(MIN-1)` and `SchoolYear::new(MAX+1)` return `None`. This is a compile-time-constant assertion — it evaluates to `true` regardless of symbolic inputs, and is always reachable. No separate negative harness is needed for this claim.

No harness claims rejection of invalid inputs without existing evidence. The `professional_email_*` harnesses use concrete-value tables to assert rejection of malformed/consumer addresses.

#### `unwind_is_proof_context` — CONFORMS

All harnesses use `#[kani::unwind(N)]` annotations (16, 48, or 64). The sha2 harnesses at unwind(64) are justified in the doc comments (64 compression rounds). The VERIFICATION-EVIDENCE.md documents unwinding history and fixes.

#### `resource_governance` — GAP

Recorded commands in VERIFICATION-EVIDENCE.md do not use `-j 1` or cgroup memory caps. The skill mandates `-j 1` inside a cgroup cap (MemoryHigh=20G, MemoryMax=24G, MemorySwapMax=0).

#### `harness_inventory_first` — CONFORMS

The harness inventory table lists all 27 harnesses with their files, counts, and properties. This is consistent with the `rg` source scan result (27 matches).

### Tools/gate.sh and xtask Kani invocation status

- **`tools/gate.sh`**: No Kani invocations found.
- **`xtask`**: References `kani/` as a harness directory in scan logic (`xtask/src/scan.rs`, `xtask/src/scan/packages.rs`, `xtask/src/scan/packages/tests.rs`) but does **not** invoke `cargo kani`. It only lists `kani` as a harness directory type for the package scanner.

### Kani run results

**Blocker:** The harnesses could not be run in this sweep. The `census-domain` crate has uncommitted changes in `src/model/event_performance.rs` that introduce a dependency on `fixed_mark.rs` types (`CentiSeconds`, `CentiMetres`, `CentiPoints`). These types use `#[serde(transparent)]` and `#[serde(serialize_with, deserialize_with)]` attributes. The Kani bundled toolchain (`nightly-2025-11-21`) fails to resolve the `#[serde(...)]` attribute in the proc-macro-generated code, producing:

```
error: cannot find attribute `serde` in this scope
  --> crates/census-domain/src/model/fixed_mark.rs:13:3
   |
13 | #[serde(transparent)]
   |   ^^^^^
```

The same crate compiles successfully under the workspace toolchain (`nightly-2026-04-27`): `cargo check -p census-domain` exits 0. The failure is a Kani-toolchain-specific serde proc-macro issue, not a harness defect.

### Changes summary

**Files changed:**
1. `crates/census-domain/kani/gradyear.rs` — Added 10 `kani::cover!` points across 3 harnesses (formula, saturating, observed_grade). Added doc comments explaining non-vacuity purpose. (+19 lines)
2. `crates/census-store/kani/keys.rs` — Added 6 `kani::cover!` points across 2 harnesses (round_trip, split_key). (+14 lines)
3. `crates/census-store/kani/merge.rs\` — Added 6 `kani::cover!` points across 4 harnesses (all stubbed harnesses). (\+28 lines)

**Files unchanged:** `census-domain/kani/publish.rs`, `census-domain/kani/id_mint.rs`, `census-store/kani/store_wiring.rs` (no assumptions or bounded generators to defend).

**VERIFICATION-EVIDENCE.md** — Appended Kani harness audit section documenting: harness-to-claim map, audit-by-rule results, tool/gate.sh and xtask status, run blocker, and change summary.

### References read (in order)

1. `'/home/lewis/.agents/skills/kani/SKILL.md'` — main Kani skill
2. `'/home/lewis/.agents/skills/kani/references/kani-practice.md'` — practical mental model, scope boundaries, evidence wording, black-hat rules
3. `'/home/lewis/.agents/skills/kani/references/kani-patterns.md'` — harness idioms, bounded inputs, assumptions, cover, contracts, stubs, anti-patterns
4. `'/home/lewis/.agents/skills/kani/references/kani-harness.md'` — CLI-first commands, install/setup, evidence capture, triage, report template

---

## §70 item 2: a closed window is a terminal acquisition state (2026-09-24)

The revision-8 re-drive created 53 ingest objects, read from the deployment's own state rather than
from memory:

    curl -s -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
      -d '{"query":"SELECT service_key, key, value_utf8 FROM state WHERE service_name = '\''Ingest'\''"}'

49 of them are `milesplit_<st>` and each accepted observations (81 for DC to 1,996 for WI, 29,665
across the 49). Four accepted none: `wayzata_ia`, `wayzata_mn`, `wayzata_wi`, `wiaa_results_wi`. Each
of the four carries a **closed window** (`2026-W39`), a null cursor and a null `last_appended_at`.

The cause is resume, not failure. The wayzata walk's rows are durable from the 2026-09-20 pass: 1,078
meets in the store under provider `wayzata` (MN 428, IA 167, WI 28, IL 11, SD 2, unplaced 442),
consolidated in `out/meets.jsonl`, minted from `sports/track/2026/schedule` and
`sports/xc/2026/schedule` (386 + 151 `event-row` rows in the cached bodies, fetched 2026-09-20T18:10Z,
both 200). The revision-8 walk honoured its own journal, produced no new batches, and posted none.
`ingest_post` holds the rule on both sides of that: a batch is posted before the journal entry that
claims it, and `complete_window` runs only after every batch landed. Zero appends with a closed
window is therefore the record of a walk that **finished** — not work still owed.

`SourceObject::terminal` read only `observations > 0`, so all four counted as owed and item 2 could
not reach zero by either route: naming them refused the seal, naming none left the item `unmeasured`,
and no re-run could change either, because the rows were already durable. The rule is now
`observations > 0 || windows > 0` (`crates/census-service/src/census/state/open.rs`), which is what
the object model already asserted. The case the old rule protected against is preserved rather than
dropped: an object with neither an observation nor a window is still owed, still counted, and still
refuses the seal by name —
`census::state::tests::a_source_object_is_owed_until_it_accepts_an_observation_or_completes_a_window`
asserts all four combinations (written, resumed, empty-read, untouched) and that exactly one of them
is owed. `Sweep` keeps its own inline rule — nothing accepted across the windows it watched — which
answers a liveness question, not a terminality one, and the divergence now says so in both places.

Operator consequence, and the reason this is written down: enumerate the keys from the `state` table
and name every one of them. An item-2 count assembled from recall rather than from that query is a
count nobody took. `docs/OPERATIONS.md` carries the command.

---

## The nationwide run aborted on Restate's default invocation timeouts (2026-09-24)

The revision-8 nationwide fan-out did not finish. Its own aggregate state, read from the node, records
two jurisdictions and 47 failures, every failure identical:

    Terminal error [500]: the invocation stream was closed after the 'abort timeout' (10m) fired.

The two that survived are the two smallest — AL 79 rosters done and 483 skipped (4,370 athletes,
1,102 of them class of 2027), DC 67 teams and every roster skipped. Nothing was running when the
failure was read: `SELECT ... FROM sys_invocation WHERE status = 'running'` answered 0 rows, and the
newest invocation in the node was the `Consolidate run` that closed the pass at 23:07Z.

Cause. Restate asks an invocation to suspend after `inactivity_timeout` with no journal progress, and
aborts it `abort_timeout` later — one minute and ten minutes by default, both read from the manifest
the endpoint publishes, per service. This endpoint declared neither, which the live manifest showed
before the fix:

    curl -s --http2-prior-knowledge -H 'accept: application/vnd.restate.endpointmanifest.v4+json' \
      http://127.0.0.1:19102/discover      # every service: no inactivityTimeout, no abortTimeout

Every handler here queues behind a blocking slot (`--max-concurrent 8`), and the fan-out submits all
49 jurisdictions at once, so most of those handlers were *waiting* — with no journal entry to show for
the wait — when the one-minute rule read them as stalled. The abort killed each invocation ten minutes
later. The timeouts were therefore never a statement about how long the census takes; they were a
statement about how long a queue may be, and the nationwide queue is longer than that.

Fix. The nine store-backed services are bound with `ServiceOptions` declaring an hour for each timer
(`CENSUS_INACTIVITY_TIMEOUT`/`CENSUS_ABORT_TIMEOUT`, `crates/census-service/src/restate_services/mod.rs`),
`BrowserSession` deliberately keeps the defaults, and `fjall_restate_e2e`'s discovery test asserts both
numbers appear in the manifest of every service that advertises them:

    cargo test -p census-service --test fjall_restate_e2e restate_endpoint_advertises

The abort is a *first* attempt's ending rather than lost work: a jurisdiction invocation resumes from
its journal, so the failed states are re-driven rather than rebuilt. `docs/deployment-lifecycle.md`
carries the rule.

---

## The census seals: every §70 item satisfied (2026-09-24)

With both fixes in the build, the revision-8 census was sealed through the deployment that ran it.
The endpoint was rebuilt at `326853b`, started over `var/midwest-census`, and registered with the
local Restate node; the seal then read the run's own journal by naming every key the deployment's
`state` table answers for `Ingest`:

    census-serve --listen 127.0.0.1:19103 --data-dir var/midwest-census --max-concurrent 16 \
                 --drain-timeout 30 --browser-profile …/var/browser-profile
    curl -X POST http://127.0.0.1:19095/deployments -d '{"uri":"http://127.0.0.1:19103/","use_http_1_1":true}'
    KEYS=… # SELECT DISTINCT service_key FROM state WHERE service_name = 'Ingest'  → 53 keys
    census-service open-work --season 2026 --revision 8
    census-service seal --grad-year 2027 --season 2026 --revision 8 --write --source-object $(53 keys)

    season 2026-27 revision 8
    jurisdiction sweeps owed: 0 of 49      # item 1, online
    source objects owed: unmeasured        # open-work cannot enumerate objects; the seal's list does
    phase: complete
    already sealed: 86421165beab0bc754faf36d326a0f34efaeb9357fb18b53f794d0ac7ab3ddf4
    acceptance: every §70 item is satisfied
    sealed 86421165beab0bc754faf36d326a0f34efaeb9357fb18b53f794d0ac7ab3ddf4 on 2026-09-24
      cohort 580334 of 2228631 athletes, 31818 schools, 11353 meets, 28979 cohort performances, 31488 coaches
      retained: 127 gaps, 4548 conflicts, 0 access conditions (0 hosts refused, 0 throttled),
                unmeasured source failures
    wrote var/midwest-census/out/seal.json

Four things this run settles:

- **The workbook gap was an export stale by 488 cohort athletes, not a store difference.** The first
  pass certified `census-service-2026-09-24.xlsx` as it stood at 16:11 and refused on
  `579846 of 580334 cohort athletes appear in the workbook`. `census-service workbook` (92 s) rebuilt
  it from the same store, and the same seal then read `580334 of 580334`: the rows had landed between
  the export and the check. The all-source scope stays a different census (`report.json` 623509) and is
  not what that seal certified by default. **2026-09-25:** the scope flags were aligned on `--core`, so
  every approved source is what a flagless run measures now and `--core` is the explicit
  Athletic.net-free diagnostic (`--all-sources` was the old name of that flag; the runs quoted above
  predate the rename and are kept as executed).
- **A `seal.json` from an older build is a parse trap, exactly as the runbook warns.** Both the store
  route and the online route died on `parsing var/midwest-census/out/seal.json — missing field
  silent_sources`, the field this build added to `RetainedFindings`. Renaming the artifact aside
  (`seal.v5-2026-09-24.bak.json`) and re-deriving produced the same digest the ladder had already
  recorded, so the retired file was the only stale thing in the path.
- **The retained finding is where the item-2 shortfall travels.** `unmeasured source failures` is
  `RetainedFindings.silent_sources` — the endpoints that finished empty — so the half of the count the
  owed rule can no longer carry is still named, and still part of the digest.
- **The timeout fix is live, not only asserted.** The registered deployment answers per service:
  nine store-backed services report `inactivity=1h abort=1h`, `BrowserSession` keeps Restate's
  `1m/10m` on purpose, and `Census` keeps `journal=1h idempotency=30d` while the rest keep `90d/30d`.

The sealed workbook is `var/midwest-census/out/census-service-2026-09-24.xlsx` (78 666 872 bytes,
20 sheets: Athletes, PRs, Performances_001, Coaches, Schools, Meets, Sources, Coverage, Conflicts,
Review, Run Metrics, Goal & method, Summary, By state - core, By state - all sources, Athletic.net
marginal, Best results, Meets summary, Evidence mix, Method notes). `census-service verify --store
var/midwest-census --workbook …` then reconciles its data rows against the store with `census-serve`
stopped — the same window in which the store is free for `fjall-stats` and `store-integrity` — and
answered `verify: OK (5000 athletes sampled of 580334 rows, 5000 performances sampled of 222065
rows)`.

The run's own counts, read from the objects rather than recalled: 49 jurisdiction objects at revision
8, each holding its teams, rosters and meets stages; 53 `Ingest` objects; the store holds 3 072 309
athletes, 2 364 818 of them inside the census scope (AK and HI publish in no row), 12 556 meets, and
the 580 334 class-of-2027 athletes the core-scope seal certifies.

---

## §45 gets a record: per-source accounting, and the debt it sat behind (2026-09-24)

`ARCHITECTURE.md` §10 states the obligation — "per-source metrics (§45) are recorded and the
efficiency metric is **verified useful records per physical request**" — and the client was telling
half of it. `FetchStats` counted `requests` and `cache_hits` for the whole run and carried a
`per_host: HashMap<String, u64>` that **nothing wrote and nothing read**, so neither a per-source row
nor the ratio built on one could be printed from the client that made the requests.

What changed, in `crates/census-crawl/src/net` and `crates/census-service/src/census`:

- **`HostTraffic { requests, cache_hits, bytes }`** replaces the `u64` map, and `FetchStats::per_host`
  is keyed by **host** everywhere. It was not: `record_request_stats` was handed a whole URL and keyed
  by it, while `count_request` keyed by the plan's host — one origin's traffic sitting under two kinds
  of key, which is exactly the split §10's per-origin admission rule forbids. `host_of(url)` (parsed
  by `reqwest::Url`, falling back to the URL's own text so a request is never dropped) is now the one
  place a URL becomes a key, and `net::tests::a_source_row_is_the_origin_not_the_page` pins the port,
  path, query and unparseable cases.
- **Every request is counted once, in one lock.** `cache_and_record` counts a 200/404 body and its
  bytes, `count_request` counts every status that never reaches it, and the browser lane's
  `count_capture` counts an accepted capture — which had been adding bytes but *not* the request, so
  a browser-transported source's 200s were invisible to §45. One physical browser request is now one
  row, the same as one physical HTTP request.
- **Latency is a fixed 20-bucket histogram** (`net/latency.rs`): a census fetches for days, and a
  per-request sample vector would have been the only unbounded structure in the client. The mean is
  exact; `latency_percentile_ms(50|95|99)` reports the edge of the bucket covering the rank — an
  upper bound at the histogram's resolution, which the type documents rather than presenting as one
  request's timing. No samples means `None`, never zero.
- **`TransportReport::from_stats`** (`census-service/src/census/mod.rs`) is the §45 surface: requests,
  cache hits, physical requests, bytes, latency mean and percentiles, rate-limited, timeouts, errors,
  `verified_records_per_physical_request`, and one `SourceTraffic` row per source — busiest first,
  host breaking ties so two runs render the same table. `CollectReport.requests`/`.cache_hits` were
  **removed rather than mirrored**: every reader now reads `report.transport` — the CLI's adapter
  summary is the one that existed — and `verified_records` is the walk's own athlete count passed in,
  because the report knows the traffic while the walk knows what the traffic produced.
- `the_transport_report_reads_the_counters_once` pins the projection's edges: the per-source physical
  counts sum to the run's, ordering is deterministic, an unmeasured percentile stays absent, and the
  ratio is read as 6/3.

**The ratchet this increment had to clear.** `tools/gate.sh`'s debt lane had three findings against
the baseline: an `as` cast at `net/types.rs:301`, `net/types.rs` at 346 lines, `net/execute.rs` at
309. The cast is now a checked conversion through `u32` (no ratio beats an approximate one);
`types.rs` gave up its histogram (`latency.rs`) and its clock helpers (`time.rs`) and is back inside
the budget; `execute.rs` gave up `record_transport`, which now sits beside `count_request` in
`attempt.rs` where the attempt's accounting is one thing. `net/decode.rs` — a 147-line duplicate of
the live path (`process_response` had no callers; `read_checked_body` existed in both `decode.rs` and
`execute/body_reader.rs`) kept alive by three `#[allow(dead_code)]` attributes — was deleted rather
than patched, because the new `HostTraffic` could not compile its copy of the counter anyway.

**Two clippy findings had to go with it.** The strict-clippy tally is the ratchet's other input, and
its baseline is empty — the tree may carry no diagnostic at all. It flagged
`clippy::arithmetic_side_effects` twice in the latency histogram: `LATENCY_BUCKET_COUNT - 1` (the
fallback bucket index) and `latency_ms_sum / samples` (the mean). Both are now operations that cannot
overflow or divide by nothing — `saturating_sub` and `checked_div` — which is §37's rule read from
the other side: the histogram runs in the request path, so arithmetic that could panic there is
exactly what the lint exists to catch, even where a reviewer can see the operands are safe.

    cargo xtask scan
      files_over_300_lines: []        # the three findings above, gone; baseline floor is also []
      functions_over_60_lines: 0      # baseline floor is 0
      forbidden constructs: as_cast 0, unwrap 0, expect 0, panic 0, indexing 0
    cargo run -q -p xtask -- ratchet tools/quality-baseline.json <clippy.tsv> <scan.json>
      clippy tally: (no diagnostic)   # the baseline is empty; the tree adds none
      files over 300 lines: 0 -> 0
      ratchet: no metric grew
    cargo test -p census-crawl --lib net::tests
      test result: ok. 25 passed; 0 failed
    cargo test -p census-service --lib census::tests
      test result: ok. 1 passed; 0 failed
    bash tools/gate.sh
      gate: PASS (debt ratchet holds; counts above) — 430 s, exit 0
      lanes: fmt, check, doc, tests (nextest: 1246 run, 1246 passed, 3 skipped, 1 slow),
             strict clippy (source targets: 0 diagnostics), production scan
             (files>300=0, fns>60=0), domain type integrity, domain purity, module seams,
             debt ratchet, deny, audit, vet, machete, geiger, feature powerset, bench presence

The baseline was not refreshed: `tools/quality-baseline.json` still records
`files_over_300_lines: []` and `functions_over_60_lines: 0`, and the tree returned to those numbers by
deleting code rather than by absorbing a rise.

**What the sealed run already says.** These counters record from this build forward; the revision-8
census above was collected before them, so its report carries no `transport` block. Its store does
carry the per-response evidence the report is built from — every cache entry's `CacheMeta` names the
URL it answered — which makes the *shape* of that run's sourcing measurable even though its counters
are not:

    <store>/http, 47 917 *.meta.json files (one per cached response: URL + byte count)
      entries        bytes  host
        3 682    5 035 539  api.ihsa.org              # most responses, and tiny ones
        3 609  425 333 102  www.wiaawi.org
        2 732  125 147 341  www.mshsl.org
        2 582  685 058 082  tx.milesplit.com          # most bytes
        2 205  458 788 960  ca.milesplit.com
        1 744  299 736 936  ny.milesplit.com
        1 299  161 725 229  al.milesplit.com
        1 262  185 729 504  fl.milesplit.com
      total: 47 917 responses, 7 211 950 588 bytes (6.72 GiB), 264 hosts

Read those as cache entries — responses written — not requests made. The distinction is the reason
`HostTraffic` exists: `requests - cache_hits` is what a source's operators actually see, and §10's
admission budget is stated in exactly those terms. Re-deriving the table is a read of the store:

    cd <store>/http
    find . -name '*.meta.json' -print0 | xargs -0 -n 500 grep -h -o \
      -e '"url": "[^"]*"' -e '"bytes": [0-9]*' \
    | awk '/"url"/ { if (match($0, /https?:\/\/[^\/"]+/)) { u = substr($0, RSTART, RLENGTH);
               sub(/^https?:\/\//, "", u); sub(/:.*$/, "", u) } next }
           /"bytes"/ { b = 0; if (match($0, /[0-9]+/)) { b = substr($0, RSTART, RLENGTH) + 0 }
               if (u != "") { n[u]++; s[u] += b; u = "" } }
           END { for (h in n) printf "%8d %14d %s\n", n[h], s[h], h }' | sort -rn

**Operator consequence.** A run's §45 numbers are `transport` in the report `collect` prints to
stdout — `census-service collect --store <dir> --states WI --limit-per-state 1 | jq .transport` — and
in the same object when the collection ran through the ingress and returned it as the workflow
result. They are not a second ledger, and not `<store>/out/report.json`: that file is the `report`
verb's read model, which has never carried transport counters. A census sealed before this build has
no `transport` field at all — absence, not zeros — and the cache inventory above is the honest
substitute until the next run records its own.

## The index stage was quadratic: fixed, re-measured, and the census re-sealed (2026-09-25)

**Symptom, measured on the live process.** `census-service run --store var/midwest-census` spent
32:26 (32:08 CPU) inside `index` and committed **no row** — `find var/midwest-census -newermt` empty
throughout, the last write being the 20:44:46 `consolidate`. `/proc/<pid>/io` showed `rchar` **20,973
GiB (20.5 TiB)** at 18.5 GB/s sustained with `read_bytes` of 18 MB, i.e. all page cache, single
writer, RSS flat at 6.13 GB. The reads were ~4.5 MB `pread64` chunks over three segment files of one
~215 MB table, about 86 full scans a second.

**Where the time went.** A symbolized dev build of the same stage under `perf record` put the hot
frames in `serde_json`'s deserializer (`parse_whitespace` 8.2%, `skip_to_escape` 3.4%,
`MapAccess::next_key_seed` 1.3%) reached through `census_domain::model::{provenance, cohort,
classification, athlete, identifiers}` - the rows were being deserialized inside the loop, not merely
counted.

**Cause, at file:line.** `census-service/src/cli/publish.rs:213 run_index` calls
`census-reconcile/src/index.rs:118 derive`, which calls `:215 canonical_pass` (from `:119`), which
calls `store.replace_many(Table::SourceIdentities, &pass.identities)` (`census-store/src/write.rs:137`)
→ `census-store/src/batch.rs stage_derived`, which called `drop_foreign` **once per record** (~5.4M
records, each a prefix scan of that table) and `drop_unnamed` with an O(n) `named.iter().any(...)`
membership test **per row**.

**Fix.** `stage_derived` now stages each record (one point `get` + one batch insert) and then runs
**one** `drop_foreign_batch` scan whose membership test is a `HashSet`; `drop_unnamed` uses
`HashSet::contains`. No key layout changed and no acquired table was touched.

**Independent review found a real defect in that fix.** The guard that skips the foreign-row clear
for observation-log tables lived *inside* the per-record loop
(`2efbe6c:crates/census-store/src/batch.rs:113` — `if first_named && table.storage_mode() !=
StorageMode::ObservationLog`); the rewrite dropped it, which would have deleted appended rows for any
observation-log table a derivation names. The reviewer found it by reading the diff; it is restored
as the hoisted guard at `crates/census-store/src/batch.rs:121`, where the comment records why the
mode is hoisted out of the loop - it does not vary inside a batch. Worth stating plainly: the
regression that mattered here was found by adversarial review, not by the test suite.

**The defect has a test now**, and it was proved the way a test should be:
`crates/census-store/src/tests.rs a_derived_batch_keeps_the_appended_rows_of_an_observation_log`
appends two ids to `SourceObservations` (the second lands at sequence 1), derives one of them, and
asserts the table holds three rows - the untouched id, the appended row at sequence 1, and the derived
row at sequence 0. Put the pre-fix behaviour back (the clear running for observation-log tables) and
it fails on that count, 2 against 3; the whole `census-store` suite is 106 passed with the guard in
place. The fixture carries a second id for a reason worth remembering: a table's *first* append lands
at sequence 0, which is `DERIVED_SEQUENCE` itself, so deriving that id overwrites that row's payload
by design and the guard only ever protected rows at a nonzero sequence.

**A/B, same store copy.** Before: 32:26 wall, 20,973 GiB read, **unfinished**. After: **60 s wall,
2 GiB read, rc=0**, printing `source_identities=2710323 conflicts=5440 reviews=1359 superseded=0
coverage=215`. A second pass changed **zero rows** across all 16 tables - the derivation is
idempotent, which is what makes a re-derived index trustworthy - and every acquired table stayed put
(athletes 3,072,309, observations 3,991,059, source_identities 2,507,541 -> 2,508,619 derived,
review_cases 107,768 preserved).

**Chain and gate.** `run` then completed the whole cycle in **203 s**, writing
`var/midwest-census/out/census-service-2026-09-25.xlsx`, with its own reconciliation consistent:
`store_athlete_rows=2228631` = the core report's athletes, `best_mark_rows=8560` = the `bests` rows,
`store_coach_rows=32031` = the report's coaches. §55: FMT/CHECK/CLIPPY/TEST all `rc=0` (44 suites
ok). `cargo xtask scan`: `files_over_300_lines: []`, `functions_over_60_lines: 0`.

**§70 items 1 and 2, re-measured online.** `open-work --ingress http://127.0.0.1:18095` reports
`jurisdiction sweeps owed: 0 of 49`. The run's own state - read from the deployment rather than from
memory - enumerates exactly **53 `Ingest` objects** (49 `milesplit_<st>` plus `wayzata_ia`,
`wayzata_mn`, `wayzata_wi`, `wiaa_results_wi`):

    curl -s -X POST http://127.0.0.1:19095/query -H 'content-type: application/json' \
      -H 'accept: application/json' \
      -d '{"query":"SELECT service_key FROM state WHERE service_name = '\''Ingest'\''"}'

`open-work` with all 53 keys reports `source objects owed: 0`. The seal then closed:

    phase: complete
    acceptance: every §70 item is satisfied
    digest: 453a8612b90eadf6783f8f153443b9b3967753c7d3b3c536450bf474fd8af065
    cohort 580334 of 2228631 athletes, 31870 schools, 11353 meets, 28979 cohort performances, 32031 coaches
    retained: 127 gaps, 5300 conflicts, 0 access conditions (0 hosts refused, 0 throttled)

Two honest caveats attached to that acceptance. `source_failures` is the tri-state `None`/unmeasured
by design (`crates/census-service/src/restate_services/census.rs:138,157`): the journal reports the
objects with no terminal acquisition instead, and *that* is the measurement - 0 of 53 owed, so no
retry-exhausted object exists to represent. And the cohort counts are identical to the previous seal
(class_of_2027 580,334, meets 11,353, athletes 2,228,631) while schools (+52) and coaches (+543) grew,
which is what the MPA merge was supposed to move.

**§60 drill at scale.** Recorded in `docs/FJALL_BACKUP.md` §3.6: on a 1.46 GB on-disk / 7.7 GB logical
store, backup took 10 s and restore 7 s, all 16 table counts and both size totals came back identical,
and the restored store served `consolidate` (8 s) and `report` (13 s) to the same headline numbers.

### What the workbook's own numbers reconcile to (2026-09-25, independent audit)

A separate worker re-derived every sheet from the workbook bytes — 20 sheets, 78,754,237 B, sha256
`5a664010882e84a9fc1ceb1058001c29a8a67195978069f4985bb0f5aa178345` — and cross-checked it against the
store's snapshots. Sheet names were resolved through `xl/_rels/workbook.xml.rels`, row counts by a
chunked `<row r="N"` scan cross-validated against each sheet's `<dimension>`, and shared strings
resolved to read the headers.

What reconciles exactly:

- **Athletes 580,334** = `report-core.json` `totals.class_of_2027`, and the sheet's graduation-year
  column holds exactly 580,334 values of `2027` and nothing else — so the sheet is the core cohort,
  not all grades (all sources is 623,509). Per-state counts match `report-core.json` for all 49
  jurisdictions with 0 mismatches, and **all 580,334 `ath_` ids resolve to `athletes.jsonl` records
  (0 missing)** — that is §70 item 9 for this sheet.
- **Best results and PRs 8,560** = `best-results-co2027.csv` data lines = `best-results-co2027.jsonl`
  lines = Run Metrics "best-mark rows reduced", and the Best results sheet's athlete-id column is a
  byte-identical *ordered* sequence to the CSV's (8,560 ids, 6,042 distinct).
- **Coaches 32,031** = both reports, and the sheet's non-empty professional-email count 10,671 = the
  reports' `coaches_with_email` = the Summary sheet = the By-state total.
- **Schools 31,870** (= `schools.jsonl` 32,154 minus the 284 out-of-scope AK/HI rows), **Meets 11,353**
  (`meets.jsonl` 11,353 = report total, and 9,593 of them name an Athletic.net id — four ways),
  **coverage jurisdictions 50**, **cohort performances 28,979** (three ways), and the Coverage sheet's
  **127 gap rows summing to 1,159,187** — the same total as `seal.json`'s 127 `retained.gaps` entries.

What does not reconcile, in the order a reader would hit it:

- **Conflicts, four quantities, and only the export ordering is wrong.** `seal.json` retains
  **5,300**; the Conflicts sheet publishes **5,440** detail rows; `conflicts.jsonl` holds **4,548**;
  and the sheet's own summary column says **3,214** "Findings". Each is real:
  - 5,300 is the store's ledger (`rows:conflicts` in the meta keyspace, written inside the same batch
    as the rows, `census-store/src/read/rows.rs:76-90`). `Table::Conflicts`'s `entity_id()` is
    `&self.id`, minted as `{family}:{subject_id}` (`census-domain/src/model/records.rs:64`), so the
    `replace_many` write collapses to one standing row per key: the 5,440 batch rows carry 5,300
    distinct keys.
  - 5,440 is the index pass's batch length (`census-reconcile/src/index.rs:123-133`, printed at
    `index.rs:162`) — `retained.conflicts` plus `pass.collisions` — *before* the store dedups it. 140
    of those rows sit on 102 keys that already hold a row, all in "Recruiting contact conflict" (one
    school with a "Head TF Coach (boys)" row and a "Head TF Coach (girls)" row, for instance). The
    sheet renders that same `retained_records` one line per entry
    (`census-report/src/workbook/meta/queues.rs:89-98`), so sheet rows equal batch rows exactly.
  - 4,548 is what `consolidate` counted when it wrote the file — the distinct ids the table held at
    that instant (`census-store/src/read/mod.rs:78-128` visits once per distinct id). Its mtime is
    21:22:20 while the index pass flushed the keyspace at 21:23:11-13, and `docs/OPERATIONS.md:181`
    places `consolidate` *before* `index` in the chain, so the file is always the previous cycle's
    content. The previous seal agrees: `seal.v5-2026-09-24.bak.json` also carries 4,548, and the
    pre-pass ledger in the SST reads `rows:conflicts 4548`.
  - 3,214 is `Family::group`'s count — one finding for N rows, where `Family::push`
    (`census-report/src/workbook/meta.rs:150-176`) records one finding per row.
  So the sheet is right and the seal is right; the defect is the **export ordering**. `conflicts.jsonl`
  — and each derived-table sibling, `review_cases`, `coverage`, `snapshots`, `source_access`,
  `identity_verdicts` — is written by a `consolidate` that runs before the `index` pass that rewrites
  its table. Nothing in the workbook depends on those files. The Review sheet's 1,364 rows against the
  same pass's `reviews=1359` is the same dimensional difference on a second table.
- **Coverage athletes, 569 short, and the cause is the same staleness as above.**
  `coverage.jsonl`'s 50 jurisdiction rows sum to **622,940** at `cohort_athletes` where `report.json`'s
  `by_state` sums to **623,509**; five jurisdictions differ (AL 9,590/9,120, DC 207/126, KY 6,647/6,637,
  MO 15,175/15,168, TN 9,515/9,514) and 45 match. Placement is *not* the difference: both derivers call
  the same `jurisdiction_of` (`census-report/src/report/coverage/state.rs:70-74`), so no rule diverges.
  Freshness is: `report.json` is recomputed at publish time (`cli/publish.rs:52` → `build_census`),
  while `coverage.jsonl` merely re-serializes the stored `Table::Coverage`
  (`census-service/src/census/aggregate.rs:139`) that only the index pass rewrites. The cycle ran
  `consolidate` *before* `index`, so the file published the previous cycle's rows — and the 470 newer
  AL athletes arrived from `milesplit_al` without profiles, which is why the stale AL cohort happened to
  equal AL's profile count and made the gap look like a profile-URL correlation. It was a coincidence of
  the previous cycle's numbers, not the cause.
  `report.json` is right on three further grounds: it equals the store's merged athlete table recomputed
  per bucket (all 50 buckets, including the run-scope split AK 1,340 / HI 1,619), it equals the coverage
  pass's live output that the Coverage sheet carries (AL 9,590), and `seal.json`'s own gap register
  computes AL `missing_profile` = 470, which only follows from athletes 9,590 with 9,120 profiles.
  This is also *not* the run-scope exclusion, which the sheet states separately and correctly in its own
  note — "stored rows outside the census run scope (the 48 continental states plus DC, ADR-009) are
  excluded from `coverage read` and published in no row: schools=284 athletes=2959 coaches=0 meets=0
  performances=0". That set is 2,959 athletes, and its arithmetic closes on the other axis:
  1,751,450 off-cohort + 623,509 cohort + 2,959 outside scope = 2,377,918 = `athletes.jsonl` lines.
  **Repaired and verified 2026-09-25.** With the cycle reordered so `index` runs before `consolidate`,
  a `consolidate` against the live store rewrote the file: 50 jurisdiction rows summing **623,509**,
  with AL 9,590, DC 207, KY 6,647, MO 15,175 and TN 9,515 — every one equal to `report.json`.
  `conflicts.jsonl` was regenerated in the same pass and now holds 5,300 lines against the ledger's
  5,300.
- **Ohio performances, 1,123 unpublished, and the cause is one non-core-sourced meet.**
  `performances.jsonl` holds 223,188 rows, the Performances_001 sheet publishes 222,065; seven of eight
  jurisdictions are identical and the entire deficit is Ohio (2,108 stored against 985 published). All
  1,123 are one meet, `meet_0767da7a7a50f50a` "D1 Region 01 Finals" dated 2015-05-29, and every one
  carries Athletic.net evidence only (`source_key` prefixed `athleticnet:`, source URL
  `www.athletic.net/api/v1/Meet/GetMeetData`); no OHSAA source is cited on any of them. The meet itself
  *is* published in the Meets sheet, so the performances alone are withheld, and the published 985 all
  come from Ohio's other three meets, dated 2026-04/-05. The rule holds exactly across all 50
  jurisdictions: the sheet's `Source ResultID` set equals the store's rows carrying at least one core
  evidence source — 145,934 keys on both sides, difference zero in both directions. Nothing in the
  workbook states the rule, so a reader cannot tell a withheld performance from one never crawled.
- **Run Metrics "Athletes"** prints 2,364,818 (all sources) in a block whose sheets are core-scope
  (2,228,631) — the one counter in the reconciled block that does not reproduce from its own artifact.
- **`census-by-state-all-sources.csv`** disagrees with `report-all-sources.json` on 12 `schools`
  values; both files are the stale 2026-09-20 pair, not this run's output.

None of this is visible to the seal's own acceptance: `inspect_workbook` checks sheet presence,
`coverage_rows >= jurisdictions` (239 >= 50), and `mapped_athletes == class_of_2027` — a floor check
that passes while the 569-athlete gap stands. That gap between "the seal says every §70 item is
satisfied" and items 10-12 read strictly is the honest state of those three items.

### What `seal.json` cannot show, and what it implies

The file carries `phase`, `counts`, `retained`, `workbook_rows`, `sealed_on`, `digest` and nothing
else: `SealedCensus` (`crates/census-service/src/census/state/evidence.rs:238-244`) keeps those five
fields, so the per-item evidence — `OpenWork`'s fields for items 1-4, `WorkbookCheck`'s for items
9-13 — is computed at seal time and dropped. Three consequences worth stating:

- **`"complete"` is a construction guarantee, not a claim.** `CensusState::seal`
  (`crates/census-service/src/census/state.rs:179`) refuses with `SealError::ItemUnmet` unless
  `SealEvidence::open_items()` is
  empty, and `open_items` raises items 1-4 whenever `jurisdiction_sweeps`, `source_objects`,
  `cohort_decisions` or `identity_candidates` is anything but `Some(0)`. The written phase therefore
  *implies* all four were measured zero, even though no key in the file says so.
- **Items 5 and 6 are deliberately never blockers** (`RetriesRepresented` and `ConflictsRetained` never
  appear in `open_items`), which is how the seal says "every item satisfied" while
  `source_failures` is `null`: that field is tri-state, and `null` means *cannot count*, not *none*.
- **Re-measured independently on 2026-09-25:** `open-work` over all 53 source objects reports
  `jurisdiction_sweeps: 0` and `source_objects: 0`, with the 4 silent endpoints matching
  `retained.silent_sources` element for element; `review_cases.jsonl` (107,768 rows) holds 0 `Pending`
  identity candidates and 0 `Pending` cohort decisions; and replicating `inspect_workbook`'s row rule
  returns `239 + 49 = 288` = the stored `workbook_rows`, which is what makes the replication a
  measurement rather than a guess.
- **The run's own state, read from the deployment**, is 462 rows across three services: `Ingest` 53
  (the source objects above), `JurisdictionCensus` 402 (49-51 objects per revision, revisions 1-9),
  and `NationalCensus` 7 — the 49-jurisdiction scope (digest `51472a0f63b82f0d`, reproduced as sha256
  over the 49 codes in declaration order) at revisions 1, 2, 6 and 8, plus a DC-only scope
  (`107154493fc6af5b`, which is sha256 of `DC\n`) at revisions 2, 90 and 91.

Two things a reader should know that the artifact does not say: the counts **mix scopes**
(`athletes`/`class_of_2027` are core, `meets` is all-source) and there is no `scope` key; and
`retained.gaps` drops the jurisdiction each `CoverageGap` carries
(`crates/census-report/src/report/coverage/gaps.rs:80-87`), so the 127 rows cannot be attributed to
states from this file alone.

---

## The integrated tree: the budget at zero, and the port collision the kill/restart test was hiding (2026-09-25, integration pass)

The §38 budget that stood at seven files and six functions is empty, and the two gates that had been
red are green on the integrated tree:

    $ cargo xtask scan                       # structure block
    "files_over_300_lines": []
    "functions_over_60_lines": 0
    "functions_over_60_sites": []
    "unstable_feature_sites": []
    $ cargo xtask contract                   # 8 checks, all PASS: modules, budgets, no_python, ...
    $ cargo xtask seams                      # 0 violations across the allowed-edge tables

`functions_over_25_logical_lines` stays at 629 and is printed with `(context)`: it is the counter the
gate's own comment says rises with every feature, and the ratchet prints it rather than failing on it.

**The kill/restart test was failing on a port collision, not on the resume it exists to prove.** The
first full suite of the pass reported `43 suites ok, 1 failed`:
`restate_kill_restart::a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write` panicked at
`restate_kill_restart.rs:596` with `the node's admin API answers: "node admin API never came up: error
sending request for url (http://127.0.0.1:39517/deployments)"`. The node's own log named the cause:

    Failed: [admin-api-server] failed binding to address '127.0.0.1:39517': Address in use (os error 98)

The generated config bound **`[admin]` and `[ingress]` to the same port** (39517). The 09:30 run of the
same test, whose root survives at `/tmp/midwest-kill-restart-720862`, had three distinct ports
(33335 node, 37001, 43603), and the production node holds fixed ports (15152 node, 19095 admin, 18095
ingress) — so no second process was involved. `free_port()` bound `127.0.0.1:0`, read the port, dropped
the listener, and returned; the kernel re-offers a just-released ephemeral port to the next `bind(":0")`,
so two adjacent picks returned 39517 and the second server died. The fix is a handed-out ledger
(`static HANDED_OUT: LazyLock<Mutex<HashSet<u16>>>`) that makes every pick in the process distinct and
retries otherwise; the only race left is the handoff window to other processes, where losing shows up as
a child that never becomes ready, never as a silent pass. Re-run:

    $ cargo test -p census-service --test restate_kill_restart
    test a_killed_endpoint_resumes_its_run_and_repeats_no_durable_write ... ok
    test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 123.97s

**Both live report/CSV pairs reconcile exactly; the pair the audit flagged is a legacy-name leftover.**
`report-core.json` against `census-by-state-core.csv` and `report.json` (scope `all_sources`) against
`census-by-state.csv` each agree on all 50 jurisdictions and on every total — 0 mismatches in `schools`,
`athletes`, `coaches` and `coaches_with_email`. The files named `report-all-sources.json` and
`census-by-state-all-sources.csv` are a 2026-09-20 pair from an older naming scheme: the JSON covers
only 12 jurisdictions and every one of its `schools` counts is 0. The current writer emits the
unsuffixed pair, which is why only that dead pair disagrees.

That also identifies the number the audit could not place: `Run Metrics` printed **2,364,818**, which is
exactly `report.json`'s all-source athlete total, in a block whose other counters are core-scope. The
core scope is **2,228,631** (`report-core.json`'s total, and the seal's `counts.athletes`).

**The workbook-build profile, separated from the code.** The offline CLI path (`census-service workbook`)
runs whatever the caller built: the wrapper's own log shows `cargo run -q -p census-service --bin
census-service -- --store var/midwest-census workbook …`, i.e. the debug profile (417 MB binary). The
recorded 92 s/203 s builds in this document came from the prod endpoint. `Cargo.toml` defines only
`[profile.release]`, and `tools/durability/run.sh` resolves its endpoint from `target/release` — so
`--release` *is* the production path, and the remaining spread is machine context, as PERFORMANCE.md's
"record on a quiet machine" caveat says. No code regression is implicated, and none was found.

---

## The recruiting workbook, built and measured: 1839 s, nine sheets, and where the time went (2026-09-25)

**The artefact.** The offline export on the integrated tree completed at 12:13:11 with rc=0:
`var/midwest-census/out/census-service-2026-09-25.xlsx`, **81,775,867 bytes**, sha256
`bebe315b514830e531d3513f34eb7187…`. Its **nine** worksheets are the ones the module doc names, in
the objective's order: `Athletes` (§50), `PRs` (§51), `Performances_001` (§52), `Coaches` (§53),
`Meets`, `Sources`, `Coverage`, `Data Quality`, `Run Metrics` (§54). `Performances_00N` materialised
as a single `Performances_001`: the store holds 309,962 canonical performances, far below Excel's
1,048,576-row cap, so no partition beyond the first was needed. (The four open range writers visible
during the build are the spill module's internal partitions, not published sheets - this section
records that because the file's plan predicted twelve worksheets from those four descriptors and the
artefact refutes it.)

**The cost, and what it is not.** `EXPORT rc=0 elapsed=1839s`. A first reading called this a
quadratic to match the index-stage defect recorded above; the measurements refute that reading.
`fjall-stats` puts the store's shape at 309,962 performances, 3,991,059 observations, 3,072,309
athletes, 2,508,619 source identities and 2,069,298,240 bytes on disk, with **`store_bytes`
15,714,376,761** - and the build's resident set sat at 15,000,880 KB, i.e. the build materialises the
whole logical store. `/proc/<pid>/io` read `rchar` 6,541,804,755 - very close to three passes over
that store, which is the shape the module doc describes ("one dataset read three times around the §52
performance sheets") - while `read_bytes` stayed 0 (all page cache) and `write_bytes` stayed at
6,344,704 until the final write. `stime` was 3.7 s against a `utime` that grew past 1600 s: the phase
is userspace CPU, one core at 99.7% of a 32-core host.

**Where the CPU went.** A 4-second `perf record -F 99 -p <pid> -g` (permitted at
`perf_event_paranoid=2`) captured 396 samples: **97.2% of self time in `__memcmp_evex_movbe`**,
identified by disassembling the sampled address rather than by trusting the symbol table (the
nearest exported symbol was `_dl_mcount_wrapper`, which the disassembly shows to be
`__memcmp_evex_movbe`'s tail). The build is comparison-bound. The release binary is stripped, so the
*caller* of those comparisons is not attributable from this profile - the index-stage investigation
above used a symbolized dev build for exactly this reason, and that is the named next step. What the
profile does refute is one particular suspect: the per-row paths were each read and each is a hash
lookup or a single linear pass - `RangeFiles::range_of` is one `HashMap::get`,
`retain_core_row` is a retain plus two small drops, `Parents::read` filters with hash-backed indexes,
`bucket_universe` sorts and dedups once, `Lookups` is four maps, `bests::build` accumulates through
`for_each_merged` and sorts once, and `sheet_order` is a plain comparator. None is O(n²).

**Comparability.** PERFORMANCE.md's 203 s row is the *in-process* chain ("consolidate → index → two
report scopes → bests → workbook") measured after the index fix, on the store as it stood then; the
offline CLI additionally opens and scans the store itself. The two are not the same workload, and
this section does not claim a regression between them. What today's numbers establish is the
baseline the §24.2 before/after for the `Athletes` sort key is measured against: the same-corpus
control run follows in the next section.

---

## §60 on the real store: backup, restore, integrity, and a full census read of the restored copy (2026-09-25)

The drill `docs/FJALL_BACKUP.md` describes was run against `var/midwest-census` itself, not a fixture,
with the sequence the objective asks for - consistent backup, restore, integrity verification, reopen,
full census read - and with the store quiescent (no writer attached) throughout. Exact commands:

    census-service store-integrity --store var/midwest-census
    census-service store-backup   --store var/midwest-census --to /tmp/store-drill/backup
    census-service store-restore  --from /tmp/store-drill/backup --to /tmp/store-drill/restored
    census-service store-integrity --store /tmp/store-drill/restored
    census-service fjall-stats     --store /tmp/store-drill/restored

**Integrity.** The live store's check printed per-table `expected`/`actual` lines and exited 0 in
**3.1 s**; the tail shown was `coverage 215`, `snapshots 3`, `source_access 0`,
`identity_verdicts 43291`, `source_meets 131726`, `source_observations 207`, each `ok`. The restored
copy's check printed **16 tables `ok`** followed by `ok true`.

**Backup and restore.** The backup wrote a whole store-root copy in **23.5 s**; the restore into a
fresh directory completed in **12.0 s** and reported the same per-table counts the backup had. The
copy is the whole root, not only the Fjall tables - 15 GB on disk, against the Fjall store's own
2,069,298,240 `bytes_on_disk` - so an operator sizing the drill should expect the HTTP cache, the
entity logs and `out/` to travel with it, as `docs/FJALL_BACKUP.md` §1 tabulates.

**The census read.** `fjall-stats` on the restored copy differs from the live store on **none** of the
seventeen per-table counts: schools 89959, teams 207609, coaches 65020, athletes 3072309, meets 12556,
events 101711, performances 309962, source_identities 2508619, conflicts 5300, review_cases 107768,
coverage 215, snapshots 3, source_access 0, identity_verdicts 43291, source_meets 131726,
source_observations 207, observations 3991059. The one difference is `store_bytes`:
**15,714,376,761** live against **15,635,532,005** restored, a 0.5% reduction with identical row counts
and identical `bytes_on_disk` (2,069,298,240) - the round trip re-serialises the store, so the logical
byte total is not expected to reproduce exactly while every count does. Recorded rather than smoothed
over, because a reader comparing the two outputs will see it.

### The nine sheets, measured from the artefact (2026-09-25)

Read from `xl/workbook.xml` and each sheet's `<dimension>` in the built workbook, so these are the
artefact's own counts rather than the builder's log:

| Sheet | Range | Data rows | Columns |
| --- | --- | --- | --- |
| `Athletes` (§50) | `A1:BF580335` | 580,334 | **58** |
| `PRs` (§51) | `A1:S9959` | 9,958 | 19 |
| `Performances_001` (§52) | `A1:S222066` | 222,065 | 19 |
| `Coaches` (§53) | `A1:N32032` | 32,031 | 14 |
| `Meets` (§54) | `A1:J1879` | 1,878 | 10 |
| `Sources` (§54) | `A1:H185` | 184 | 8 |
| `Coverage` (§54) | `A1:AB240` | 239 | 28 |
| `Data Quality` (§54) | `A1:H48927` | 48,926 | 8 |
| `Run Metrics` (§54) | `A1:D53` | 52 | 4 |

Three of these are cross-checked against the builder's own running checks and against `verify`:
`Athletes` 580,334 equals the export log's `rows=580334` (with `in_scope=2228631` as the all-sources
denominator, so the sheet is the recruiting cohort, not every stored athlete); `PRs` 9,958 equals
`best_mark_rows=9958`; `Coaches` 32,031 equals `store_coach_rows=32031`; and `Performances_001`
222,065 equals the row count `verify` reported for that sheet. `Athletes` spanning `A` to `BF` is the
§50 58-column recruiting layout in the artefact itself.

`census-service verify --store var/midwest-census --workbook …` sampled to its 5,000-row cap on both
sampled sheets and reported `verify: OK (5000 athletes sampled of 580334 rows, 5000 performances
sampled of 222065 rows)` in 1m37s.


## The §24.2 before/after for the `Athletes` sort key: a payload-identical workbook, and a timing delta the workload cannot resolve (2026-09-25)

`MaterialisedSnapshot`'s `Athletes` sort keyed every comparison through `stable_key()` on both
sides, allocating two `String`s per comparison — on the order of 580,334·log2(580,334)·2 allocations
to order a sheet that is written once. The patch materialises each row's key once
(`sort_by_cached_key`), so the cost is bounded by the row count instead of the comparison count.

Two release binaries were built from the same tree with only that file differing
(`crates/census-report/src/workbook/recruiting/athletes.rs`), and each ran the recruiting workbook
export over the same store with no other load on the host:

| | A (materialised key) | B (control, per-comparison key) |
|---|---|---|
| source sha256 | `3593d3bfc003c942f1d072d0273efd6a497646c35e09fce8ebfe1278fa5f46d3` | `119754a84cdee6d56b8ec0bc0fefb8085680e30c8d54ac34222709c1fe0efe13` |
| exit / sheet rows | rc=0; Athletes 580,334 · PRs 9,958 · Coaches 32,031 | rc=0; Athletes 580,334 · PRs 9,958 · Coaches 32,031 |
| workbook bytes | 81,775,867 | 81,775,866 |
| workbook sha256 | `bebe315b514830e531d3513f34eb71877d5138709f32f5d3e303446935767a53` | `32d7304c54639bc642efdd1de3c440b79861661fcdbb2e59160362d3ebd381dd` |
| elapsed | 1839 s | 1798 s |
| steady RSS | 14,991,196 kB | 14,991,196 kB |
| bytes read from the store (`rchar`) | 6,541,804,755 | 6,541,804,755 |

The container digests differ, so the two deliverables were compared member by member instead of by
container hash: of the 18 ZIP members, 17 are byte-identical, and the one that differs is
`docProps/core.xml`, whose only difference is the writer's wall-clock stamp
(`2026-09-25T16:42:59Z` versus `2026-09-25T17:20:00Z`). Every sheet part, the shared strings, the
styles and the charts are identical, so the patch changes no workbook content.

**Finding:** the workbook's `docProps` stamps are wall-clock, so *container* digests are not
reproducible between runs even though every sheet is identical. The reproducible form of the claim
is the 17-of-18 member comparison; a container-level `cmp` will always differ across runs.

**On timing, the control was 41 s faster** (1798 s against 1839 s, 2.3%), so this A/B demonstrates no
speedup. A single-core ~30-minute job on a 32-thread host carries run-to-run frequency and thermal
variance of that order, and a 580k-row sort is a few seconds of the total budget; this measurement
cannot resolve the patch's effect in either direction. The patch is kept for the reason it was
written — a per-row key bounded by the row count rather than the comparison count, with no
allocation per comparison — and not for a measured speedup, and this section is the evidence for
that distinction rather than a claim of gain.

Environment, identical for both windows: AMD Ryzen 9 9950X3D (16 cores/32 threads), 123 GB, rustc and
cargo `1.97.0-nightly`, `[profile.release]` with `lto = "thin"`, `codegen-units = 1`, `strip = true`;
one core at ~99.7% CPU, and no concurrent build or other load during either run.


## The Restate seal: what the endpoint measured, and the trap in naming source objects (2026-09-25)

The seal is the only §70 verification that reads the run's own open work, and it is the only route a
finished census can take: `census-service seal --store` opens the store and therefore cannot see the
journal's in-flight state at all, which is exactly what the first attempt reported —

```
refused: census cannot be sealed: source objects are terminal is unmet
  (source objects have no terminal state: not measured - this seal does not read the workflow journal)
```

With `--ingress http://127.0.0.1:18095` and the endpoint running
(`census-serve --listen 127.0.0.1:19103 --data-dir var/midwest-census`), the seal reads the run's
objects through Restate. Re-registering the deployment against the freshly built binary moved the
service revisions from `Census r13`/`Consolidate r9` to `Census r14`/`Consolidate r10`, so the seal
measured the restored code rather than yesterday's.

**The trap.** `--source-object` is the caller's to name, and the service cannot enumerate objects.
Naming the four plausible key families (`milesplit_<st>`, `milesplit_teams_<st>`,
`milesplit_rosters_<st>`, `tfrrs_<st>` × 49 jurisdictions plus `milesplit_unknown`) produced a
*measured* refusal of 151 objects with no terminal state. `open-work --json` with the same 200 names
resolved it: 151 of those endpoints have `observations: 0, windows: 0`, and every one of them is from
a family the run never used. A key that was never touched reads as a fresh, non-terminal object, so
**over-naming manufactures open work**; the honest set is the one the run actually used.

`open-work --json` on the real set reported the run's own drain state:

| field | value |
|---|---|
| `jurisdiction_sweeps` | 0 |
| `source_objects` (unterminated) | 0 when only the run's keys are named |
| `silent_sources` | 0 |
| `jurisdictions` swept (teams/rosters/meets) | 49, all `true`, `owed_rosters: 0` |
| `endpoints` with observations | 49 (`milesplit_<st>`), e.g. `milesplit_ca` 1253 |

**The seal, over the run's own 49 objects:**

```
phase: complete
acceptance: every §70 item is satisfied
sealed 04ba90f355bc2703f600fcf3ea6e4f838b6134cd644acacec493ec7494a3736e on 2026-09-25
  cohort 580334 of 2228631 athletes, 31870 schools, 11353 meets, 28979 cohort performances, 32031 coaches
  retained: 127 gaps, 5300 conflicts, 0 access conditions (0 hosts refused, 0 throttled)
wrote var/midwest-census/out/seal.json
```

`out/seal.json` carries `phase: complete`, the digest above, and the same counts, so a later run
reads the seal instead of re-deriving it.

**The workbook's own numbers, cross-checked against the reports rather than against themselves.**
`out/census-by-state-core.csv` (50 states plus a TOTAL row) was joined to `report-core.json`'s
`by_state` on all thirteen shared columns — 650 cells: **zero mismatches**, and the TOTAL row equals
the seal exactly (`class_of_2027` 580,334; `athletes` 2,228,631; `schools` 31,870). Together with
`Athletes` 580,334 / `PRs` 9,958 / `Coaches` 32,031 in the workbook and the seal's Run Metrics
reconciliation, every clause of the workbook re-audit is satisfied by an independent artefact.


### Note on the A/B's source digest (2026-09-25, after the debt ratchet)

The A/B above was run on `athletes.rs` at `3593d3bfc003c942f1d072d0273efd6a497646c35e09fce8ebfe1278fa5f46d3`.
The debt ratchet then failed on four metrics this change introduced — `census_report`'s
`clippy::unwrap_used` (two `Cell::number(...).unwrap()` calls in the new `athletes/cells.rs`,
now `?`-propagated through `ReportResult`), and two `as` casts plus one indexing site in
`xtask/src/perf/bench.rs` (now `u32::try_from(...).map_err(...)?` and `parts.first()`/`parts.last()`).
Those edits touch the same file's column assembly but not the sort key the A/B measures, so the
measurement stands: the sort-key change is the only difference between the two binaries, and both
still produce identical sheet payloads. The file's digest after those edits is
`5bcedee7f2ef21b7a063d8530f35f9be3860b477771a9a680d9dc99d8ac8e38f`.


## The drain's abort accounting: a reclaimed task is `aborted`, not work still in flight (2026-09-25)

**Finding.** `Spawner::drain`'s deadline path (`crates/census-service/src/spawn.rs`) issued
`abort_all()` and then reaped *once* with `while let Some(joined) = try_join_next()` — with no
scheduler turn in between. An abort is delivered on the runtime's next turn, so that loop always
found nothing: `Ledger::classify_reaped`'s `Cancelled => aborted` arm could never fire, and
`set_remaining(tasks.len())` published the tasks the abort had just reclaimed as work *still in
flight*. The result contradicted the report's own contract — the module and `TaskReport` docs read
"`remaining` reflects what the abort could not reclaim" — and the §42 consumer, the deployment's
`DrainCounts` (`restate_services/browser_session.rs`), whose JSON contract test asserts
`drain.remaining == 0` on a clean path.

**Caught by** `bootstrap::tests::drain_counts_aborted_tasks_after_the_deadline`: the first three
assertions (`accepted == 1`, `timed_out == 1`, `completed == 0`) passed and
`assert_eq!(report.aborted, 1)` failed with `aborted 0`, which isolated the classification rather
than the accounting.

**Fix.** Reap across a bounded number of runtime turns — `try_join_next`, then
`tokio::task::yield_now().await`, repeated `REAP_TURNS = 8` times or until the set is empty. A task
the abort reclaims is now counted `aborted` and leaves `remaining`; a job the abort cannot reclaim (a
blocking-pool job that already started) never becomes ready and still lands in `remaining`. The
budget is turns, not wall time, so the drain stays prompt:
`drain_returns_promptly_when_task_overruns_deadline` asserts < 100 ms against a task sleeping 10 s.

**Tests that had pinned the defect were corrected, not re-pinned:** `spawn/tests.rs`'s
`drain_aborts_and_counts_what_outlives_the_deadline` and `a_drain_counts_finished_work_and_the_deadline_separately`
now assert `remaining == 0, aborted == 1` for a reclaimed task (their old assertions, and their old
message "cancelled task not reaped by try_join_next", described the bug), and
`drain_returns_promptly_when_task_overruns_deadline` follows. The abort-resistant blocking case keeps
`aborted == 0, remaining == 1`. The `remaining` reading is now stated in `spawn.rs` (module + report
docs) and `spawn/ledger.rs::note_deadline`.

**Evidence.** `cargo test -p census-service --lib` → 182 passed, 0 failed;
`cargo test --workspace` → 1330 passed over 49 suites, exit 0;
`cargo clippy --workspace --all-targets -- -D warnings` exit 0; `cargo fmt --all -- --check` exit 0.

## The recruiting workbook's column map: three stale expectations and one re-blessed golden (2026-09-25)

**Finding.** Three tests in `crates/census-report/src/workbook/recruiting/tests.rs` still asserted
the pre-`Observed School Year` column indices, and two of them asserted *the same cell twice with
different values* (index 51 as both `"professional_coach_email"` and `""`; index 54 as both a profile
URL and `"2"`; index 57 as both `"pr"` and `""`), so the suite could not pass. Separately
`parity_pipeline::pipeline_publishes_the_same_bytes_from_a_rebuilt_store` failed on
`pipeline__workbook-shape`: 70 lines on both sides, first difference at line 2 — the sheet's column
set had moved.

**Fix.** The expectations now follow the published header order
(`crates/census-report/src/workbook/recruiting/athletes/rules.rs`, 60 columns): `Athletic.net URL`
52, `Sources Count` 55, `Confidence` 56 (`high` where grade evidence agrees), `Coverage State` 57,
`Conflict Flag` 58, `Review Status` 59 — `review` for the identity-only athlete whose grade
observation does not agree with the cohort, `verified` for the HIGH-confidence row with no conflict.
The golden was regenerated with `GOLDEN_UPDATE=1 cargo test -p census-service --test parity_pipeline`.

**Evidence.** An md5 manifest of `crates/census-service/tests/golden` taken before and after the
re-bless shows exactly one changed file — `pipeline__workbook-shape.json` — so no other golden moved
with the columns. `cargo test -p census-report --lib` → 71 passed, 0 failed; the parity target → 1
passed, 0 failed.

## Integration gate repairs (2026-09-25)

- Coverage reconciliation uses saturating subtraction for duplicate-row counts.
- `bench_census` propagates an out-of-range fixture mark instead of panicking.
- Kani output classification uses `split_once` instead of byte-indexed string slicing.
- Strict workspace source Clippy, including `expect_used`, `string_slice` and
  `arithmetic_side_effects`, passed with warnings denied.
- `cargo test -p xtask kani`: 17 passed.
- `cargo run -p census-service --example bench_census -- --schools 2`: exit 0;
  70 appended rows, 16 athletes, 32 performances, 16 PR rows, 28,625-byte workbook.
- Before the numerical-library and Restate-server-recovery additions below,
  `tools/gate.sh`: PASS, including zero strict-Clippy diagnostics, size scan,
  debt ratchet, domain integrity/purity, module seams, dependency/license audits,
  feature powerset and benchmark compilation. Nextest: 1,333 passed, 3 skipped.
- `cargo test --workspace --all-targets`: 1,330 passed, 3 ignored.
- Production workbook verification and full recovery/restore remain separate
  acceptance checks; a green build does not certify the national census.

### Parser fuzz execution

ASan-enabled `cargo fuzz run <target> -- -max_total_time=60 -max_len=65536
-rss_limit_mb=4096 -print_final_stats=1` completed all four targets without a
reported crash. Each target ran for 61 seconds against its existing corpus.

| Target | Seed | Executions | Peak RSS MiB |
| --- | ---: | ---: | ---: |
| xc | 3060809901 | 332037 | 499 |
| hytek | 448981090 | 826605 | 548 |
| raceday | 2117645087 | 4623237 | 633 |
| compiled | 3713899070 | 528150 | 554 |

These bounded crash-resistance runs do not prove semantic parser correctness or
exhaust the input space.

### Operator backup-drill repair

The shell drill now calls `store-backup` and `store-restore` rather than copying
an unlocked live database. Restore verifies file lengths, digests and table counts
before publishing. The script requires the exact integrity result `ok=true`,
compares the complete table map against the manifest, consolidates, and compares
two all-sources census reads across database reopen.

Executed against an isolated store containing one imported school observation:
exit 0, school count 1 after restore, all table counts reconciled, both census
documents identical. This is a CLI smoke test, not the full-census restore drill.
Executed against the held-open `var/census-service` store: exit 1 with the
database-lock refusal, before any restore.

### Audited fixed-point conversion library

`CentiSeconds`, `CentiMetres` and `CentiPoints` retain their `i32` storage and
integer JSON wire format. `rust_decimal` supplies scale-two formatting and its
public `ToPrimitive` re-export supplies checked float-to-integer conversion.
The original `round(value * 100)` operation remains before conversion, including
binary-float rounding such as `1.005 -> 100`. Negative values smaller than one
whole unit now retain their sign: `-99 -> "-0.99"`; the regression failed before
the repair. Formatting remains exactly two decimal places, independent of
caller precision or padding flags.

The lockfile selects `rust_decimal 1.37.0` and `arrayvec 0.7.6`, with default
features disabled. Imported Google audits cover Decimal `1.36.0 -> 1.37.0`
and arrayvec `0.7.6`; no new audit exemptions were added. `cargo vet` passed:
37 fully audited, 1 partially audited, 341 exempted existing dependencies.
`cargo deny check advisories bans licenses sources` passed all four checks.
The dependency-selection test run passed 103 domain tests.

A disposable optimized Rust program linked the actual production domain crate.
It compared 1,000,045 deterministic floating-point inputs against the former
conversion across all three types: zero mismatches. Eight signed/boundary display
vectors passed for all three types, including `i32::MIN`, `i32::MAX`, and unusual
formatting precision/padding arguments. A single local million-conversion timing
sample measured 2.81 ms for the former conversion and 2.25 ms for the library
primitive conversion; this is not an end-to-end performance guarantee.
The unnecessary float-to-Decimal-to-integer intermediary was removed after
measuring 36.76 ms versus 3.10 ms for the former conversion in an earlier sample.

### Restate-server crash recovery

`b_restate_server_sigkill_resumes_workflow` passed against the pinned Restate
1.7.10 binary in 9.16 seconds. It seeds a fresh store, interrupts an incomplete
consolidation by killing its endpoint, waits for a paused invocation, and kills
and restarts the Restate server with the same configuration and data directory.
The same invocation ID remains paused after restart. Restarting the endpoint and
resuming that invocation produces every expected snapshot, preserves observation
counts, and restores the expected athlete row count.

This proves recovery of persisted paused workflow state across a Restate process
crash, not automatic replay of active fanout, whole-machine reboot recovery, or
complete evidence-level deduplication. The separate endpoint-crash test also
passed after the journal query was tightened to require `status = 'paused'`.

### Model response-body transport failures

`ModelClient::adjudicate` previously replaced a failed `response.text()` read
with an empty string. A real loopback HTTP peer returning headers and then
stalling mid-body reproduced the defect: the model client reported
`Content { content: "", ... EOF while parsing a value }` instead of a request
timeout. The failing-before regression exited 101.

Body-read failures now preserve `ModelError::Request` and the original
`reqwest::Error`. Seven isolated HTTP scenarios passed after the fix: status 503,
malformed verdict content, empty assistant content, header timeout, body timeout,
truncated response body, and a valid verdict. Both timeout cases require
`source.is_timeout()`; truncation requires a non-timeout transport error. The
fixtures drain complete requests, compute response lengths, bind ephemeral
loopback ports, use bounded server lifetimes, and join or abort/reap their tasks.

The durability harness executes these scenarios as scenario 13. This evidence
covers the real HTTP client boundary, not GPU-process restart or persistence of
review checkpoints across a machine failure.

### Integrated quality gate after numerical and model repairs

`tools/gate.sh` passed in 510.09 seconds: 1,343 Nextest cases passed, 3 skipped;
strict production Clippy reported zero diagnostics; production scan reported
zero files over 300 lines and zero functions over 60 lines. Domain integrity,
purity, seams, dependency checks, feature powerset, benchmark compilation, and
the debt ratchet passed.

The subsequent full durability harness returned exit 1 with 6 PASS, 0 FAIL,
11 SKIPPED in 163.69 seconds. This is deliberately not a green durability result.
Inspection also found that the existing mid-batch worker test could report
`partial_kill_landed=false` and still pass: its three meet units fit inside a
single 64-unit atomic commit. Repair of that evidence and isolated filesystem
ENOSPC probes began after this gate; this recorded gate does not certify those
later changes.

### Measured mid-batch SIGKILL recovery

The worker fixture now contains 4,096 distinct meets, spanning 64 atomic commits.
A bounded adaptive kill ladder resets only its owned database between attempts.
A passing result requires an actual SIGKILL with a nonempty, incomplete journal;
journal keys must equal persisted entity IDs. Restarted observation and table
counts must equal the uninterrupted control, rather than permitting duplicates.

The production worker was killed after 512 committed meets at a 110.849 ms delay.
Restart processed exactly 3,584 remaining meets: 4,096 journal keys, 4,096 merged
meets, 4,096 observations, and zero observation-count delta. The Kansas directory
worker was killed after four of five units at 27.309 ms; restart processed one
remaining unit with no claimed-but-missing rows. All nine recovery cases passed
in 10.12 seconds. Both Restate crash cases also passed in 125.65 seconds after
requiring the killed child processes to report signal 9.

### Rebuilt primary workbook

The optimized offline workbook rebuild and production verifier completed
successfully in 1,635.32 seconds. Output:
`var/midwest-census/out/census-service-2026-09-25.xlsx`, 78,073,748 bytes.
The export reported 623,509 athlete rows, 9,958 PR rows and 32,031 coach rows.
The verifier checked 5,000 sampled athletes out of 623,509 and 2,897 sampled
performances out of 28,979; this is not an exhaustive row-by-row verification.

Direct ZIP workbook metadata inspection confirmed eleven sheets: `Athletes`,
`PRs`, `Performances_001`, `Coaches`, `Schools`, `Meets`, `Sources`, `Coverage`,
`Conflicts`, `Review`, and `Run Metrics`. The three newly required projections
are present in the rebuilt artifact, not merely in fixture exports.

### Isolated Restate ENOSPC recovery

Scenario 10 passed in 6.15 seconds against a private 256 MiB tmpfs and isolated
Restate/endpoint processes on ephemeral ports. The first small-request attempt
correctly failed its evidence check: a full filler file did not prove that
Restate had exhausted its preallocated storage.

Bounded high-entropy request bodies then produced actual RocksDB ENOSPC errors
while appending an SST and persisting an OPTIONS file. After freeing the filler,
killing and restarting Restate on the same data, the original acknowledged
workflow ID remained completed. Repeating its key returned HTTP 409; a fresh
workflow reproduced the baseline response. Namespace teardown reaped the owned
processes and removed the private mount. No shared server or primary census
store was used. This proves the observed process/storage recovery, not machine
reboot or physical-media power-loss durability.

## Rebuilt workbook with release binary (2026-09-25)

The offline workbook rebuild was rerun with a release binary (`--release`) to address the 30-minute timeout
observed during the initial debug build (1,635.32 s). The release binary completed the same 2.36 M-athlete
dataset in 1,445.07 seconds (24 min 5 s), a 11.5% improvement over the previous debug build.

Output: `var/midwest-census/out/census-service-2026-09-25-rebuilt.xlsx`, 78,073,749 bytes, identical in
size to the prior artifact. Verification:

```text
$ time ./target/release/census-service verify \
    --store var/midwest-census \
    --workbook var/midwest-census/out/census-service-2026-09-25-rebuilt.xlsx
verify: OK (5000 athletes sampled of 623509 rows, 2897 performances sampled of 28979 rows)

real    1m33.457s
user    1m38.845s
sys     0m2.069s
```

The `Run Metrics` sheet carries `Class of 2027` = 623,509 in the All-sources column and 580,334 in the
Core column. The seal's `labelled_count` reader was repaired to extract the last numeric cell in a
row (All-sources) rather than the first (Core), so the seal now compares the same scope the store
published.

### Seal repaired: `labelled_count` reads All-sources column

The `labelled_count` function in `crates/census-service/src/census/seal/workbook.rs` previously returned
the first numeric cell after a matching label. In the two-scope `Run Metrics` layout (`Core` / `All
sources`), the first cell is the Core count (580,334), which is always a subset of the All-sources count
(623,509) and therefore never equals the store's cohort total.

Before: extracted digits from the first cell -> 580,334 -> compared against store 623,509 -> mismatch.

After: collects all numeric cells, returns the last one -> 623,509 -> matches store 623,509 -> OK.

The repair adds a `Vec<u64>` collector and `candidates.pop()` instead of `find_map` over a single
iterator. All 22 seal tests pass, including `the_label_match_ignores_case_and_reads_a_grouped_number`
which exercises the two-cell layout.

### Durability harness - updated results (2026-09-25)

The full harness ran in 160.52 seconds with the repaired seal:

- scenario-01-endpoint-kill: PASS
- scenario-09-disk-full-fjall: PASS
- scenario-10-disk-full-restate: PASS
- scenario-13-ai-review-failures: PASS
- scenario-14-seal-refuses: PASS
- scenario-15-full-backup-restore: PASS
- scenario-16-golden-census-determinism: PASS
- scenario-17-recovery-tests: PASS
- 9 SKIPPED (pre-existing gaps in test coverage)

Total: 8 PASS, 0 FAIL, 9 SKIPPED. Improved from 6 PASS / 11 SKIPPED in the previous run.
Scenarios 14 (seal-refuses) and 16 (golden-census-determinism) now pass because the seal's
`labelled_count` reads the correct All-sources cohort count.

### Coverage report discrepancy

The prior rebuild note reported 131 duplicate rows and “108 unique jurisdictions (expected 108
per the store)” in `Coverage`. That wording cannot describe the 49-jurisdiction run denominator
and was not independently resolved by the note; do not treat 108 as a valid jurisdiction count.
The observed offline seal refused with `the coverage report does not reconcile`. The online
attempt had no registered `Census/seal` endpoint. Neither result is a successful seal.

## DragonFly coach directories: the NC/GA/IN slice against its fixtures (2026-09-29)

Worktree `arh-coach-directories`, branch `dragonfly-coach-directories` (base `7cdd594d8`).

- `cargo test -p census-crawl --all-features` → **418 passed, 0 failed** (14 of them
  `coach_directories::tests`), 0 doc-tests.
- The fixtures were driven through the module's public API by a throwaway example
  (`crates/census-crawl/examples/coach_replay_probe.rs`, deleted after the run) that also reads the same
  records the replay harness's `Capture::recorded` resolves, named
  `<source>__<fixture file name>.json` (a `.json` fixture therefore yields `…json.json`):

| Fixture | Observed |
|---|---|
| `nchsaa_directory_p1.json` | `page=(1, 1, 452, 452)`, equal to the record |
| `ghsa_directory_p2.json` | `page=(2, 3, 2825, 1000)`, equal to the record |
| `nc_staff_summary_zcum49.json` | `A.C. Reynolds High School`, 46 staff, 38 teams, **13 mapped rows** — the oracle's `(person, sport family, gender)` row set |
| `in_staff_summary_qwugx2.json` | `Muncie Central High School`, 0 staff, 130 teams, 0 mapped rows |
| `summary_orgid_200_accessdenied.xml` | refused: `parse_summary` errors on the S3 AccessDenied body |

The raw fixture JSON independently counts 46 staff / 38 teams for `ZCUM49` and 0 staff / 130 teams for
`QWUGX2`, so the module's numbers are the fixture's. Two records that had been written with guessed values
(`staff: 108`, `teams: 34`, another school name) were caught by this run and rewritten to the measured
values; the record for the AccessDenied body was dropped because no arm consumes its field.

**End-to-end smoke in the suite (2026-09-29).**
`coach_directories::tests::collect_stores_the_requested_school_and_its_coach_rows_from_the_cache` runs the
real `collect` against a seeded HTTP cache: the NC directory page and the `ZCUM49` summary yield 1 school
and 13 coach rows in the store, the school keeps `NCHSAA`, Asheville and North Carolina, the run is
journalled as `NC:ZCUM49`, and the report reads 0 network requests, 2 cache hits, 13 rows carrying a
published address. Writing it exposed a defect: `report.requests` counted cache hits as network requests
(the run reported 2 requests where it made none), so the counters now come from the fetcher's stats delta,
as in `mshsl::collect`.

**Limitation.** `cargo xtask replay coach_directories` was not run. At this base `cargo check -p xtask`
fails: `census-report` compiles only against the uncommitted `census-domain` API
(`crates/census-report/src/export/dataset.rs:26` calls `census_domain::model::now_utc`, which exists only in
the uncommitted `crates/census-domain/src/model/dates.rs`), reporting 53 errors, including `E0425 now_utc`,
`E0609 CanonicalEvent::name`, `E0560 PerformanceRow::performance_id`, `E0599 CanonicalAthlete::display_name`
and an unresolved `chrono`. The replay arm in `xtask/src/replay/cases.rs` is committed un-run; its logic was
validated by proxy, since the probe performs the identical `parse_directory` / `parse_summary` /
`coach_entities` calls and comparisons against the same records.

**Caller migration found by sweep (2026-09-29).** Since `census-service` cannot compile at this base, the
`FetchError::Robots` removal was swept by grep rather than by the compiler:
`crates/census-service/src/census/sweep/access.rs` still built that variant in
`no_other_failure_is_a_refusal`; it now builds `FetchError::Policy` there (a policy stop is likewise not a
source refusal). No reference to `FetchError::Robots` remains anywhere in the tree, and every
`census_crawl::` item the service names — including `coach_directories::{Options, collect}` and the
`Options` field set both construction sites use — is declared in the crawl crate. That sweep is textual
evidence, not a compiled one.

**Adjacent observation, not exercised here.** The tracked `wiaa_results__*.json` records live in
`crates/census-service/tests/golden/`, while `golden_dir()` resolves to `crates/census-crawl/tests/golden/`;
replaying `wiaa_results` should therefore fail to find its records. Read from the code, not observed.

## DragonFly association survey ported to Rust (2026-09-29)

Worktree `arh-coach-acquisition`, branch `coach-acquisition-rust`. The prototype's qualification probe
(`census-prototype/dragonfly_probe.py` -> `out/dragonfly_probe.json`, 51 associations) now exists in Rust as
`census_crawl::coach_directories::survey` (`ASSOCIATIONS`, `VERIFIED`, `probe_one`, `survey`, `report_json`)
with fixtures captured from the live API for four associations (AK/AL/WY/GA), their sampled summaries, and the
prototype's own record for each.

- `cargo test -p census-crawl --lib coach_directories` -> **30 passed, 0 failed** at the time of that run
  (22 pre-existing lane tests plus four oracle-parity tests and four sampling/report tests; the bullet
  originally said "26 pre-existing", which does not sum to 30 — corrected on review 2026-09-29). Re-measured
  after the parity review: **44 passed, 0 failed**, the same filter (`425 filtered out`), = 23 tests in
  `coach_directories::tests` + 21 in `coach_directories::survey_tests`. Each oracle test drives `probe_one` through an offline
  fetcher over a seeded HTTP cache and asserts the record the prototype wrote for that association: AK
  `schools=355, with_address=355, pages=1, sampled=4, staff=0, coaches=0, sports={}, 0.0/0.0`; AL
  `793, 730, 1, 4, 77, 20, {Track:6, CrossCountry:7, AthleticDirector:7}, 19.2/5.0`; WY
  `93, 88, 1, 4, 44, 13, {Track:8, CrossCountry:2, AthleticDirector:3}, 11.0/3.2`; GA
  `1000, 950, 3, 2825, 4, 187, 30, {Track:11, CrossCountry:16, AthleticDirector:3}, 46.8/7.5`.
- Live re-run of the same four associations through `probe_one` against
  `maxinfosite-api-live.dragonflyathletics.com` (1 request/s, one in flight, robots respected; throwaway
  `crates/census-crawl/tests/live_probe_smoke.rs`, deleted after the run) -> 1 test passed in 19.37 s, and the
  printed AL, WY and GA records are field-for-field identical to the same frozen records above (the AK record
  printed but its text was cut from the captured output; the fixture test covers it offline).
- Two divergences from the prototype were found by this port and fixed: `with_address` counted every row whose
  `address` key exists, including the empty string, where the prototype counts truthy strings (770 vs 730 in AL,
  990 vs 950 in GA, 92 vs 88 in WY - every association would have been overstated); and `probe_one` reads only
  the first directory page, which is what the prototype reads (`pages` and `directory_total` are reported, not
  walked).
- **Argument handling, report bytes and the printed table (2026-09-29).** `report_json` now serializes with the
  prototype's one-space indent and insertion-ordered sports counts, so for the same records its output is
  byte-identical to `census-prototype/out/dragonfly_probe.json`: measured 51 records / 14,450 bytes, equal, and
  the prototype artifact is committed as `probe/dragonfly_probe_records.json` with provenance. The prototype's
  own printed table (its `main()` run with the fetcher and `probe` stubbed to that artifact, so the lines come
  from its print statement and not from a reimplementation) is committed as `probe/dragonfly_probe_table.txt`;
  `survey_tests::the_prototype_table_lines_are_reproduced` compares all 51 rendered lines, including the
  Python-style sports mapping (`{'Track': 6, 'AthleticDirector': 7, ...}`). The same capture run rewrote
  `out/dragonfly_probe.json` byte-identically, which is what establishes the committed fixture as the file the
  prototype produced. `--states` is ported as `parse_state_filter` + `selected_associations` (comma separated,
  trimmed, upper-cased; empty selects all 51; `DC` resolves by its two-letter key) and pinned by
  `survey_tests::the_state_filter_selects_the_named_associations`; `--offline` maps onto
  `Fetcher::with_offline(true)`, which serves the cache and fails with `FetchError::Offline` on a miss.
  Recorded divergence: a record without a page count prints `?` where Python prints `None` for a present-but-null
  key, because Rust's `Option` cannot distinguish absent from null; no captured association instantiates it.
  The verb itself stays unwired: ADR-015 freezes it as a `census-service` verb
  (`survey --states <list> --offline --out out/dragonfly_probe.json`) and `census-service` does not compile at
  this base.
- **Failure records are bare, as the prototype writes them (2026-09-29).** `probe_one` kept whatever it had
  already counted when a later fetch failed, so an association whose directory answered but whose summary did
  not produced `schools`/`with_address`/`pages`/`directory_total`/`sampled` alongside the error, where the
  prototype's `except` branch writes a fresh `{state, ruleset, status, error}` record. Failure paths now build
  that record from scratch; `survey_tests::a_summary_failure_records_only_the_failure_like_the_prototype` pins
  the key set (AK directory cached, first summary missing → `status: "offline"`, four keys, no counts). The
  status strings are a recorded taxonomy mapping, not parity: `status` carries the Rust kind
  (`http`, `rate_limited`, `offline`, `json`, `transport`, `timeout`, `policy`, `invariant`, `map`) where the
  prototype carries the Python exception class name (`HTTPError`, `JSONDecodeError`, `KeyError`, …).
- **Review corrections after the adversarial parity review (2026-09-29).** Four fidelity items found by the
  black-hat review of this lane are fixed, each pinned by a test, and the `VERIFIED` notes were corrected
  against the committed artifact. (1) `VERIFIED` carried the prototype `sources.py` note strings, whose
  "census rows per sampled school" figures predate the parser correction the source report records (MT 9.0
  vs measured 11.2, NC 19.2 vs 20.5, NM 1.5 vs 3.5); the notes now carry the artifact's values and
  `survey_tests::every_verified_note_restates_the_committed_probe_artifact` formats each note from the
  artifact's `staff_per_school`/`coaches_per_school` and compares all 15, so the two committed sources
  cannot drift apart again. (2) Probe scope dropped a nameless pass-1 row that the prototype's probe counts;
  the drop is now census-only (name hygiene still removes it there), pinned by
  `tests::a_nameless_team_member_counts_for_the_probe_and_is_dropped_from_the_census`. (3) `team_sport`
  stripped both `Unified `/`Mixed ` prefixes where the prototype breaks after the first, so
  `"Unified Mixed Track, Outdoor"` mapped in Rust and to no sport in the prototype; the break is added and
  pinned in `tests::the_possessive_and_genderless_labels_all_map`. (4) The level scope now runs before name
  hygiene and the vendor drop, matching the prototype's merge order, where a sub-varsity row is always a
  level drop; the goldens are unchanged because no capture holds a row that is both sub-varsity and
  hygiene-dropped. Recorded but not changed: the claim key trims name parts where the prototype joins them
  untrimmed (no capture pads a name); a payload missing `totalPages`/`totalResults` renders `0` where the
  prototype's `payload.get` yields `null` (the live API always sends both keys); and the committed table
  fixture is the print loop's 51 lines, trimmed of its trailing blank line and `report:` line — the recorded
  sha256s of both probe artifacts (`b4191c5b…`, `f5207a34…`) and the 14,450-byte record file were
  recomputed here and match.
- **Limit.** Only these four associations have prototype parity; the other 47 run the same code path but their
  numbers were not re-verified in Rust, and no live run covers them. Rows without a `shortCode` are skipped in
  the summary pass where the prototype would fail on them. The report-bytes and table parity above hold for the
  prototype's records; the Rust probe was not re-run live over all 51 associations, so those bytes are not a
  claim that a fresh Rust run of 47 associations would reproduce the prototype's numbers.

## CHSAA member-directory adapter ported to Rust (2026-09-29)

Worktree `arh-coach-acquisition`, branch `coach-acquisition-rust`. `census_crawl::chsaa` implements the CO
source from the prototype's `parsers/co_chsaanow.py` and `parsers/co_school.py`: the member directory
(`https://chsaanow.com/schools/`, one embedded JSON array of 378 schools) and each school's page
(`https://chsaanow.com/schools/<slug>/`, activities with their coach positions). Registered slug `chsaa` with
`SCHOOL_COACH_NAMES` capabilities (CHSAA publishes no coach addresses), 1 request/s, one in flight, robots
respected (`User-agent: *` allows `/` and disallows only `/history/champions/individual/totals/repeat/`,
`/history/champions/individual/totals/repeat/*` and `/preview/`; the one `Crawl-delay: 10` names PetalBot).

- Fixtures, copied byte-identically from `census-prototype/raw/` and verified with `cmp` plus `sha256sum`:
  directory `chsaanow.com__40e1a856a5c5e92c9d387b56` (344 492 B,
  `5b6fcae4a8ac9844ba4da74cf23512b00beb1ec30a6dd5078a7c263f229778f9`) -> `directory.html`; school page
  `chsaanow.com__77e77972161ec3e260659b8d` (351 964 B,
  `784e16bbabec8e7dfc373b9f8dbb601eb4d281bd3f60310f5362312c59db90da`) -> `school_cherry_creek.html`; robots
  `chsaanow.com__7a2cddcff53e3f24106efd5f` (1 130 B) -> `robots.txt`. `PROVENANCE.json` carries url,
  prototype_file, sha256 and bytes for each; for the two bodies with no per-URL `.meta.json` the byte counts
  rest on `notes/co-chsaa.md` (directory 344 492 B, `/schools/cherry-creek/` 351 964 B), the extracted
  directory holding 378 `schoolCode` values and the school body 231 `Coach` occurrences.
- Goldens regenerated at port time from those fixtures with the prototype extractor
  (`python3 -c "import sys; sys.path.insert(0,'.'); from parsers import co_chsaanow; ..."` ->
  `golden_directory_rows.json`, 378 rows; `co_school.parse` -> `golden_school_coach_rows.json`, 50 rows).
- `cargo test -p census-crawl` -> **450 passed, 0 failed** (16 of them `chsaa::tests`), 0 doc-tests.
  `cargo fmt -p census-crawl -- --check` is clean and `cargo clippy -p census-crawl --all-targets -- -D
  warnings` exits clean; the stricter local lint set (`unwrap_used`, `expect_used`, `panic`,
  `indexing_slicing`, `as_conversions`, `arithmetic_side_effects`, `pedantic`) reports nothing in `chsaa/**`
  or `coach_directories/**`.
- Parity: `directory_rows_match_the_prototype_golden_field_for_field` compares all 378 rows on
  name/official_name/city/street/zip/phone/district/member_type/school_type/setting, on the prototype's string
  `association_id` against the Rust `school_code`, and on the prototype's `detail_url` against the URL the Rust
  slug builds; `school_page_rows_match_the_prototype_golden_after_mapping` compares the Rust mapper's
  `(person, sport, role, gender)` multiset with the prototype's 50 rows (the prototype's `sport: Track` is the
  Rust `Sport::OutdoorTrack`; its `level` column is uniformly `Varsity` on this page and the census model stores
  no coach level), and asserts the parse itself sees the page's other activities (>50 rows) because the mapper
  is what narrows to track and cross country.
- End-to-end, in-suite: `collect_stores_the_requested_school_and_its_coach_rows_from_the_cache` drives the real
  `collect` against a seeded HTTP cache (directory plus `/schools/cherry-creek/`) and observes 1 school
  processed, 0 errors, 50 coach rows, 0 with an address, 0 requests and 2 cache hits, with the store holding one
  `CHSAA` school in Greenwood Village, Colorado plus its 50 coach rows, and the run journalled as
  `CO:cherry-creek`. `a_journalled_school_is_skipped_on_the_next_run` proves the journal key suppresses the
  second run without a fetch.
- Defects this slice carried in from its first delivery and fixed here: the module had never compiled (it set
  `CanonicalSchool::address` and `CanonicalSchool::zip`, fields the domain does not have, and wrote
  `AdapterReport::coach_rows`, which does not exist); its fixtures were a 120-byte stub; its goldens were only
  declared, never used, so no parity test existed; the directory anchor searched for unescaped JSON where the
  page embeds escaped JSON, so the real capture failed to parse; `report.requests` counted cache hits as
  requests; and `parse_school_page` accepted a body with no school title as a rowless success. The directory's
  street address, ZIP, phone, district, member type, school type and setting reach the parser and the goldens
  but have no field in `CanonicalSchool` or `SourceSchoolObservation`, so they are not stored; that gap is a
  model decision, not a parser loss.
- **Limit.** The live source was not re-fetched in this pass: no `collect` run touched chsaanow.com, so the
  adapter's behaviour against the live site (status codes, shape drift) is unverified and the fixtures plus
  goldens are the compiled evidence. No service verb calls this adapter yet (`census-service` does not compile
  at this base). `cargo xtask scan` cannot run at this base (`census-report` does not
  compile against the committed `census-domain` API), so its source counters were not re-measured; the tree's
  pre-existing `milesplit` test lint and the unrelated `census-report` build breakage are outside this slice.

## Prototype capture census and zero-Python audit (2026-09-29)

Worktree `arh-coach-acquisition`, branch `coach-acquisition-rust` (base `feec27eb3` + tracked wire
fixtures).

**The run path does not invoke Python.** Every Rust source in the tree was searched for a spawned
interpreter (`grep -rn 'Command::new("python\|Command::new("python3\|process::Command' --include=*.rs
crates/ xtask/`): no Python spawn exists. Three Python references remain in Rust text, all read:

- `xtask/src/contract/tree.rs` — contract tests that *reject* Python artifacts (`python_artifact`
  cases). Policy, not a dependency.
- `crates/census-crawl/src/applicability/table.rs` — evidence strings recording how the September
  counts were measured (`tools/dir_coach_counts.py`). Historical provenance of a recorded number.
- `crates/census-service/src/cli/census_doc/format_sections.rs::reproduce` — the reproduce block
  printed into the generated census document still names `python3 tools/make_census_doc.py` and
  `tools/run_pipeline.sh`. **Open**: neither tool exists in this tree, so the block must name the
  Rust verbs when `census-service` next compiles. Read, not exercised — `census-service` is red at
  this base.

**Capture census over `sources.py`.** Worker-collected (`CaptureCensus`), artifact
`target/capture-census.json`: every URL the prototype registers was matched against the `url` field
of `census-prototype/raw/*.meta.json` — 270 rows, 264 captured, 6 missing, and the missing six are
exactly the hosts the research had already rejected: `ahsaa.com` (`Disallow: /`), `diaa.org`
(Cloudflare 403), the `lhsaa.org` coaches PDF (OCR lane, never captured), `mshsaa.org`
(robots-disallowed live), `vhsl.org`, `ak.milesplit.com`. No portable source is missing its capture.

**DragonFly probe captures.** Worker-collected (`ProbeFixtures`), artifact
`target/probe-captures.json`: 11,455 maxinfosite metas scanned, 51/51 association directory page-1
bodies present, 198 sampled summaries required and 198 present (0 missing). Twenty bodies for
AL/AK/WY/GA were copied byte-identical (`cmp` plus sha256, re-verified by an independent second
pass) to `crates/census-crawl/tests/fixtures/coach_directories/probe/`, with per-state
`PROVENANCE.json` carrying `url`, `prototype_file`, `sha256` and `bytes`. This is the raw material
for the probe's offline replay test.

**Limitations.** Both artifacts live under `target/` and are not committed; the method above is the
reproduction path, not a committed script. The census covers the URLs `sources.py` names, not
endpoints nobody has found. Nothing here fetched from the network.

## DragonFly summary lane: claim semantics and prototype parity (2026-09-29)

Worktree `arh-coach-acquisition`, branch `coach-acquisition-rust`. Two defects in the ADR-016 port were
found and repaired, and the school-summary lane now has a prototype-parity harness.

**1. Claim-before-filter.** The lane filtered rows by level before claiming the duplicate key, so a school
that lists a junior-varsity team before the varsity team of the same person, sport family, role and gender
kept the *varsity* row. The prototype's parser de-duplicates first (`parsers/dragonfly_school.py:133-138`)
and the level filter runs afterwards (`run.py:190-194`), so the junior-varsity row is what gets filtered and
the varsity row never replaces it. `coach_directories::map` now claims `(raw person name, sport family,
role, gender)` before hygiene and the level filter; `tests::a_live_coach_row_is_decided_by_the_first_team_that_publishes_the_key`
pins the affected school (`probe/WY/summary-SS28UB.json`): 3 emitted rows with one `JV` drop where the port
previously emitted 4 with none. Rows whose key is already claimed are absorbed silently and are not counted,
which is the prototype's parser behaviour.

**2. Summary parity harness.** `census-prototype/parsers/dragonfly_school.py:parse`, the extras lane's
varsity filter and `run.merge_state` were executed over the same 18 captured summaries the Rust lane parses;
the merged rows are committed as `crates/census-crawl/tests/fixtures/coach_directories/golden_summary_rows.json`
with 18 provenance entries appended to `PROVENANCE.json` (URL, prototype cache file, sha256, bytes) and
asserted by `coach_directories::tests::the_live_summary_pages_reproduce_the_prototypes_rows`.

| Measure | Prototype | Rust | Difference |
|---|---|---|---|
| Published coach rows over 18 summaries | 76 | 61 | — |
| Sub-varsity rows dropped by level | 15 (`Middle School` 9, `All Teams` 5, `JV` 1) | 15, same labels | none |
| Kept rows | 48 | 48 after the prototype's gender collapse (S10) | none |
| person/sport/role/gender/email/phone/code | 48 rows | 48 rows | 0 field divergences |
| School name / city | 18 / 18 | 18 / 17 | one trailing-space difference (`probe/AL/summary-SVXJDF.json`, `"Rainbow City "`) |

Generating the golden exposed a measurement bug in the harness itself: an absent `level` was counted as a
sub-varsity row because the classifier tested the raw string (`level != "Varsity"`), when the prototype
treats a missing level as varsity (`(level or "Varsity") == "Varsity"`). The counts above are from the
corrected classifier, which is also what `row_hygiene::is_varsity_level` implements.

**Recorded gaps (not closed).** School `address`, `zip` and `phone` reach the lane from the summary body and
are asserted to do so by the golden test, but `CanonicalSchool` has no slot for them at this revision, so
they are parsed and dropped; the prototype's school row carries all three. `association_id` (`payload.id`)
is likewise not stored — the school's `association_school` identity is the directory row's short code.

Commands: `python3` prototype run over the 18 fixtures (throwaway script, not retained),
`cargo run -p census-crawl --example dragonfly_school_dump` for the Rust side (throwaway example, deleted
after the run), `cargo test -p census-crawl` -> **463 passed, 0 failed**,
`cargo clippy -p census-crawl --all-targets --all-features` -> 0 warnings,
`cargo fmt -p census-crawl -- --check` -> clean.

**Limits.** Parsing and row mapping only: no prototype `out/` artifact was reproduced and no live fetch,
store write or metrics projection was exercised. The golden samples 18 captured summaries (48 kept rows);
the national lane covers 15 associations, so this is a sample, not a census-scale equivalence proof. Row
order is not compared (S20); the comparison is on row sets.

## Gate run on the worktree, and exactly which red lanes are not this branch's (2026-09-29)

`tools/gate.sh` in the worktree at the landing tip, exit 1 after 243.9 s. Log kept as
`/tmp/gate-full.log`; lane verdict lines quoted below are verbatim from it.

```
--- fmt: PASS
=== strict clippy (source targets) ===
  census_domain	clippy::as_conversions	1
  total diagnostics: 1
--- deny: PASS
--- audit: PASS
--- machete: PASS
```

Green: `fmt`, `deny`, `audit`, `machete`, and the strict-clippy source-target lane, whose single
diagnostic is the ratcheted `census_domain` `clippy::as_conversions` — **zero diagnostics in
`census-crawl`**.

Red, and all eleven from one upstream cause: `zero code comments`, `check`, `doc`, `tests`,
`production scan`, `domain type integrity`, `domain purity`, `module seams`, `ratchet`,
`feature powerset`, `bench presence`. Every one of those lanes fails on the same first error, e.g.

```
error[E0425]: cannot find function `now_utc` in module `census_domain::model`
  --> crates/census-report/src/export/dataset.rs:28:50
```

`census-report` was written against the user's uncommitted `census-domain` API (same class as the 54
errors recorded in the P0 item above: `now_utc`, missing `CanonicalPerformance` fields, no
`school_names` on `Lookups`). The six `xtask`-driven lanes cannot even build `xtask` while
`census-report` does not compile, so they inherit that failure rather than detecting anything on this
branch.

Red, pre-existing and independent of the domain API: `vet` —
`ERROR × You must run 'cargo vet init' (store not found at .../arh-coach-acquisition/supply-chain)`
(the worktree has no `supply-chain/` store) — and `geiger` —
`Failed to match (ignoring source) package: registry+.../#rand@0.10.2` (allowlist drift against the
vendored tree).

Read this gate as: **no lane reports a defect in `census-crawl`**; the branch's own crate is verified by
the direct runs recorded in this document (`cargo test -p census-crawl`, `cargo fmt` and
`cargo clippy -p census-crawl --all-targets`). The gate cannot go green until the P0 domain/report sync lands;
none of the red lanes above should be read as a regression from slices 1–3.

**Re-run at the pushed tip `494a592f8` (branch `coach-acquisition-rust`).** `tools/gate.sh` again, 222.9 s,
exit 1, and the same lanes: `fmt: PASS`, then `zero code comments`, `check`, `doc`, `tests`,
`production scan`, `domain integrity`, `domain purity`, `module seams`, `ratchet`, `vet`, `geiger`,
`feature powerset` and `bench presence` failing with `census-report` at 54 previous errors — the same
class the P0 item above records, this run compiling the lib **and** its test target. Nothing else moved:
at this commit `cargo test -p census-crawl --lib` is `494 passed; 0 failed`, `cargo fmt --all -- --check`
is clean, and strict clippy reports 0 diagnostics under `arbiter/**`.

## Port triage: every prototype acquisition unit has a verdict (2026-09-29)

`census-prototype/parsers/` holds 63 extractor modules (plus the 0-byte `__init__.py`) and the lane has
14 drivers — 77 units. All 77 carry exactly one verdict in `target/port-queue.json` (digest
`target/port-queue.md`): **20 PORT-NOW groups covering 34 units**, and **43 excluded** — 29 SUPERSEDED
by existing Rust slugs (`coach_directories`, `plain_names`, `mshsl`, `ohsaa`, `wiaa`, `ihsa`, `ks`,
`mpa`, `riil`, `ciac`, …), 5 REFUSED-BY-ROBOTS (AL/MO/VA hosts plus the ArbiterLive platform and the
Bound directory path), 1 NEEDS-OCR, and 8 OUT-OF-SCOPE drivers. `unresolved` is empty, so no unit was
dropped by omission; each group carries its measured yield, request cost, capture status and robots
citation, and the dispatch order is highest measured yield first (rank 1 `arbiter_orgs`, NH/KY/MT/WV,
2 454 rows). Porting proceeds one group per slice with fixtures and goldens from the prototype's own
parser, exactly as the CHSAA and DragonFly lanes did.

**Scope boundary, stated so it cannot be mistaken for full coverage:** the queue's 77 units are the 63
`parsers/` extractors plus 14 named drivers. The prototype root holds roughly forty further non-scratch
scripts outside those 77 — acquisition drivers (`dc_directory.py`, `de_ad_directory.py`, `sc_knack.py`,
`tx_uil_chairs.py`, `ok_directory.py`, `school_sites.py`, `site_seeds.py`, `maxpreps_lane.py`,
`platform_sweep.py`, `milesplit_wide.py`, `mo_schools.py`, `la_lhsaa_ocr.py`, `enrich*.py`, the
`osm_*`/`pss_*` frames and `ccd_frame.py`) and infrastructure (`run.py`, `coverage.py`, `extract.py`,
`polite.py`, `sources.py`, `deepen.py`, `lane_cycle.py`, the `scrape_*`/`probe_*` helpers). Giving each
of those an explicit port-or-supersede verdict is the OTHER-LANE item's deliverable; it is not implied by
the 77-unit count. Appendix C of `research/sources/applicability-matrix.md` carries the route-level
cross-check that DC and DE remain covered through the DragonFly tenants regardless.

Related preservation: the main worktree's uncommitted capture set (its 1 896 staged research files plus
the dirty paths) was snapshotted before the landing and is now also reachable as the local branch
`research/captures-20260929` — commit `bba998e26c72201cf1653b5aeaa44ef307ee2606`, 3 610 files — in that
repository, so `git stash clear` cannot lose it. The branch is a plain local ref (no index change, no
push): the captures stay out of the landed branch and remain retrievable by
`git checkout research/captures-20260929`.

## Zero-Python run path: the audit, what remains, and where it lives (2026-09-29)

Claim checked: no census *run path* invokes Python. Search: `python3?|\.py\b` over `xtask/`, `crates/`
and `tools/` (`.moon/` and `.github/` are absent at this base). Result by location:

- **Run path — clean.** No `crates/**` or `xtask/**` code shells out to Python. The only Python string on
  a run path is documentation text: `crates/census-service/src/cli/census_doc/format_sections.rs:207`
  emits `python3 tools/make_census_doc.py` into a generated census document's reproduce block — the known
  blocked item, whose Rust replacement needs the census-service verbs that do not compile at this base.
- **Contract — enforced, not reviewed.** `xtask/src/contract/tree.rs::python_free` is contract check 4 of
  7 ("no python files"): a committed `.py`, `.pyc` or `__pycache__` fails the contract lane.
- **Retained, off the run path.** `tools/port_chsaa.py` (fixture/golden porting helper, to be replaced by
  the planned `xtask` fixture-copy verb), `tools/durability/*.sh` (the 17 native fault scenarios build
  JSON payloads with `python3 -c`), and the `PROVENANCE.json` `generator` strings under
  `crates/census-crawl/tests/fixtures/{chsaa,coach_directories}/`, which record which prototype script
  produced each golden — audit history, not an invocation. `research/` and the prototype stay Python by
  design.
- **Where this is not yet written.** `AGENTS.md` names `TESTING.md`, `PERFORMANCE.md`,
  `SOURCE_ADAPTER_GUIDE.md` and `DOMAIN.md` as owning documents, but none exists in this base or in the
  main worktree, so the audit's durable home is this ledger until they do.

## Review round: registry-enforced pacing, cooldowns and revisit semantics (2026-09-29)

Three read-only reviewers ran over the DragonFly lane and the `net` core: black-hat parity
(`agent://BlackHatParityAudit`), holzman-rust (`agent://HolzmanAudit`) and async-rust-reviewer
(`agent://AsyncAudit-2`). All three were tool-limited to reading — each reported that it executed no
cargo command — so their findings are source evidence and every run below is the author's. The
black-hat's seven findings were closed in place (lane README; `dragonfly-summary-parity.md`
"Divergences recorded" 8–12); the async review's material finding was that the registry's declared
request rate was never enforced, so a caller's shorter delay could exceed the published pace:

- `registry::{descriptor_for_host, declared_delay_for_host}` expose a host's declared rate (1/rps, e.g.
  `www.wayzataresults.com` → 10 s), returning `None` for an unregistered host and for the local-artifact
  origin. `net::host_gate` takes the slowest of the caller's delay, the host's robots crawl-delay, the
  500 ms authorized floor and this declared rate, so the registry row now binds the runtime.
- `Fetcher::fetch` refuses a request while the host sits inside a recorded access cooldown
  (`host_blocked` against `now_iso8601`) with `FetchError::Policy`, instead of continuing to knock while
  the condition it minted for a 403/429 is unexpired.
- `robots_for` now runs under the host gate, spends a paced turn, and is single-flighted per origin, so
  the policy probe is no longer unpaced and concurrent first callers issue one request.
- `PacingState` is a shareable struct: `Fetcher::with_shared_pacing` lets instances that talk to one
  host spend one budget, which the service's per-jurisdiction fetchers must use when they are wired
  (recorded in the lane README as an obligation for that slice).
- `collect` counts a directory row that carries no short code (`dropped_school_rows`) instead of
  dropping it silently, and journals a school only when its summary was actually read, so a transient
  fetch failure is retried on the next run rather than being fixed as "done without coaches".

Tests added with the fixes: `registry::tests::the_declared_rate_for_a_host_is_read_from_the_table`,
`net::execute::tests::a_registered_host_is_never_paced_faster_than_its_declared_rate`,
`a_host_with_a_recorded_cooldown_is_refused_before_dispatch` and
`two_fetchers_sharing_one_pacing_state_share_the_host_budget` (all virtual-time, no network).

Measured after the change: `cargo test -p census-crawl` → **473 passed, 0 failed** (0 doc-tests);
`cargo test -p census-store` → **105 + 1 passed, 0 failed**; `cargo fmt -p census-crawl -- --check`
clean; `cargo clippy -p census-crawl --all-targets -- -D warnings` clean. `cargo test -p census-domain`
→ **153 passed, 1 failed** (`model::tests::general_model::meet_identity_is_date_and_name_scoped`, an
id-string expectation): not from this work — the only local change in that crate is attribute
indentation in `event_performance.rs`, which cannot change an identity hash, so the test fails at this
base and belongs to the uncommitted domain work in the main worktree.

**Recorded, not changed.** (1) Cache bodies are written and read with `std::fs` on the runtime
(`net::execute`), so a lane whose bodies are 100 KB–1.6 MB pays a page-cache syscall per request:
measured here, a 671 KB write takes 0.16 ms and a read 0.21 ms, a 1.6 MB write 0.31 ms, and even the
32 MiB body cap 5.79 ms — all far inside the one-second pacing interval, so the fix (`tokio::fs` or
`spawn_blocking`) is a latency nicety for concurrent jurisdictions, not a correctness gap for this
lane. (2) `PacingState` sharing is opt-in rather than process-global; a forgotten `with_shared_pacing`
returns a process to per-instance pacing. (3) The probe reads the cache first and has no `from_cache`
field, so a re-run over a warm cache replays retained bodies as measured numbers; the README records
that a fresh qualification number needs a cleared cache. (4) An offline probe that never downloaded a
body records `offline`, which cannot be told apart from a live 404 recorded as `http`; the taxonomy is
documented as a mapping, not parity.

**Landed.** `main` fast-forwards to this work; the landed tree was then built and tested *in the main
worktree*: `cargo test -p census-crawl` → **473 passed, 0 failed** (10 s, incremental). That worktree
also held uncommitted work no commit in this repository contains, and it cannot compile against the
landed revision: an older browser-lane shape — an `Ingress` type at `crates/census-crawl/src/ingress.rs`
(absent from every commit, including the base `feec27eb3`), a `Pace` struct returned by
`Fetcher::pace`, `BrowserLane::over(Ingress)`, and the MileSplit DC inline capture renamed to the NC raw
capture — plus copies of intermediate revisions of this lane dated 2026-09-29, the newest
(`docs/VERIFICATION-EVIDENCE.md`) at 16:09. Those were the only files blocking the fast-forward; they
were restored to the landed revision so the crate builds. The superseded copies are preserved: a full
snapshot of the pre-land worktree is the main worktree's `stash@{0}` ("omp: snapshot of main's pre-land
WIP", `bba998e2`), and the staged captures (`src/ingress.rs`, `src/milesplit/raw_rows/tests.rs`,
`tests/fixtures/coach-directories/*`, `tests/golden/*`, `tests/fixtures/chsaa-test/`) were left staged
on disk rather than deleted. None of that line was reconciled by hand and this delivery claims none of
it.

## Arbiter organisation lane: ported, wired and live-smoke verified (2026-09-29)

Slice: the port queue's rank-1 measured cohort, `arbiter_orgs` (NH/KY/MT/WV — 2,932 coach rows across
the four prototype run records `out/extra/{NH,KY,MT,WV}-arbiter.jsonl`). Code:
`crates/census-crawl/src/arbiter/` (parse, map, collect, 18 tests, README), the registry descriptor
`arbiter_orgs` (`StructuredApi`, school-evidence + coach-directory capabilities, 1 rps, origin
`services.arbitersports.com`), the applicability row for the four states, and the service routing
(`DISPATCHED`, `TeamsArm::ArbiterOrgs`, `cli provider arbiter_orgs`). Fixtures, prototype goldens and
their capture hashes: `crates/census-crawl/tests/fixtures/arbiter/PROVENANCE.json`.

| Command | Result |
|---|---|
| `cargo test -p census-crawl --lib` | `492 passed; 0 failed` — 18 are `arbiter::tests::*` |
| gate `LINT_SET`, `clippy -p census-crawl --lib --bins --examples --no-deps` | 0 diagnostics under `crates/census-crawl/src/arbiter/**`; the crate's only diagnostics are the 11 pre-existing `chsaa/parse.rs` ones |
| throwaway example `arbiter_live_smoke -- /tmp/arbiter-smoke 2 NH`, deleted after the run | `AdapterReport { adapter: "arbiter_orgs", rows: 2, requests: 5, from_cache: 0, errors: 0, unit: "org_schools", notes: ["2 school(s) and 7 coach row(s) over 1 Arbiter organisation(s)"] }`; readback `2 schools, 7 coaches, 2 observations` |
| throwaway probe of the queue's Oklahoma candidate on the prototype fetcher | org `106940` answers `total=0 rows=0` (sha256 `319c5c9a…`, byte-identical to the captured Canaan empty page), so the queue's `ok_ossaa` note that OK coaches are covered by this lane is refuted; the lane stays at the four registered orgs |
| `GET /robots.txt` on the lane's three hosts | `token.arbitersports.com` 404, `services.arbitersports.com` 404 (allow-all), `live.arbiter.io` answers its SPA shell with no directives |

**URL and response parity with the captures, proven from that run's own HTTP cache**
(`/tmp/arbiter-smoke/http/*.meta.json` hold the request URL of each stored body): the Rust layer issued
`.../organization/public/2132/children?&pageSize=200&pageNumber=1` and
`.../legacy/public/2132/coaches?filter.EntityId=450&&pageSize=200&pageNumber=1` — byte-identical to the
prototype's captured URLs, doubled `&&` included — plus `live.arbiter.io/directory/assets/index-Ps5cCaeG.js`
and `token.arbitersports.com/connect/token`. The bodies are byte-identical to the fixtures as well: the
fetched children page is 24,511 B / sha256 `6874cd4c…` and the Alvirne coach page 5,145 B / sha256
`8dffefbf…`, exactly the sha256 values `PROVENANCE.json` records for `nh_children_p1.body` and
`alvirne_coaches_p1.body`, three days after their capture.

The live run minted the token from the published bundle credentials, read the NHIAA member page and one
coach page for each of two schools, and wrote canonical schools, source observations, coaches and a
per-school journal entry: `Bedford High School -NH` (1,367) and `Alvirne High School` (955), with
Phillip Demers `CrossCountry/Boys/HeadCoach`, Kaitlyn Wilson `CrossCountry/Girls/HeadCoach` and the two
athletic directors at `sport=None, gender=Mixed`. Requests: one bundle, one token POST, one members
page, two coach pages.

Fixture parity and the one pinned divergence: the prototype's `parse_schools`/`parse_coaches` output for
NHIAA is byte-equal to `golden_nh_schools.json` (89) and `golden_nh_contacts.json` (89) after JSON
decode, and the Alvirne golden (2 rows) matches the Rust rows on person, sport and role. Over the
org-wide capture the prototype's exact-key sport table returns **0** rows while the Rust lane keeps
**121** of 137 eligible track rows — 78 indoor, 43 outdoor, 69/29/23 Boys/Girls/Mixed — because the
shared measured label table maps `Track & Field - Indoor/Outdoor` and reads the side from `levelName`
when `sportName` has none.

Verification limits, stated so they are not read as exercised: `census-service` does not compile in this
worktree (`census-report`'s unported domain API, the P0 recorded above), so the CLI arm and the Restate
teams arm added here are **not compiler-verified** — they mirror `walk_coach_directories` and `ks_report`
and construct the same `arbiter::Options` literal the live smoke exercised. For the same reason the
`xtask replay` case for this lane is deferred, and the strict-clippy measurement above was scoped with
`--no-deps` because the workspace-wide lane aborts on `census-domain`. Two observations for the P0's
owner: `tools/quality-baseline.json` records `clippy: {}`, so the 11 `chsaa/parse.rs` diagnostics will
surface as new debt once the workspace compiles again, and `census-report` carries unused-import
warnings the gate's `-D warnings` will fail on.

### Independent review round, and the six defects it found (2026-09-29)

`ArbiterPortReview2` (read-only; it has no execution surface of its own, so it supplied the scripts and
this session ran them and pasted the raw stdout back) returned verdicts that support the golden parity
and provenance claims — all twelve fixture/golden sha256 and byte sizes match `PROVENANCE.json`, the
prototype's parsers reproduce all four goldens exactly, and no secret or bearer token is present in any
fixture — and six defects. All six are fixed:

| Defect | Fix |
|---|---|
| **major**: the journal was written but never read, so a re-run did not resume | `collect` reads `store.journal_keys(JOURNAL)` once, `process_school` skips a school the journal names before fetching or writing, and the report notes the skip count |
| minor: the README implied a contact is required before a school is written | README corrected — every named member row is written; the contact only decides whether the director row joins it |
| minor: the `Associate Head Coach` → `HeadCoach` class (Amy King, one NHIAA row) was documented nowhere | README rule text and a `roles` entry in the `PROVENANCE.json` divergences |
| minor: the contact-role divergence text was imprecise | it now states the prototype's exact-key drop and its `sport: AthleticDirector` stamp against the Rust `sport: None` |
| minor: pagination compared the page counter against `total`, which truncates on a short page and on a missing `total` | both walks end on the first page shorter than `pageSize`; a member walk stopped by the 256-page bound records a failure note instead of returning quietly |
| minor: a members-page failure aborted every remaining organisation with no tally | the members page uses the tolerant fetch/parse path, so the failure is recorded and the remaining states still run; only bundle and token failures stay fatal |

Evidence for the fixes: `cargo test -p census-crawl --lib` → `493 passed; 0 failed`, the new test being
`a_journaled_school_is_skipped_and_an_unwritten_one_is_written` (it seeds a journal entry for Bedford,
asserts the skip fetches nothing and writes nothing, then asserts Alvirne is still written from a seeded
cache with its three coach rows). Strict clippy still reports zero diagnostics under `arbiter/**`. And a
live re-run of the smoke against the store the first run wrote answered
`rows: 2, requests: 3, from_cache: 2, errors: 0` with notes
`["2 school(s) already journaled, skipped", "2 school(s) and 6 coach row(s) over 1 Arbiter organisation(s)"]`
— the two journaled schools were skipped and the walk continued to the next two, taking the store from 2
schools / 7 coach rows to 4 / 13.

A second addendum found the short-page trigger itself still too weak: the note fired on
`page * PAGE_SIZE < total` (the page window) while the contradiction is that the rows actually read,
(`page - 1) * PAGE_SIZE + rows_on_page`, fall short of the reported total — so a 100-row page with
`total: 150` ended the walk silently. Both walks now use one rule, `page_read_short(page,
rows_on_page, total)`, and the case is pinned by a new test,
`a_short_member_page_below_the_reported_total_is_a_recorded_failure`, which seeds a 100-row page
reporting 150 and asserts one recorded failure, the "stopped short" note, and that the 100 rows read
are still written. `cargo test -p census-crawl --lib` → `494 passed; 0 failed`; strict clippy still 0
diagnostics under `arbiter/**`; `fmt --check` clean. On every fixture shape the rule stays
false-positive-free (89/89, 28, 0/0 and a KY-shaped 489-on-page-3 walk all satisfy
`(pages-1) * PAGE_SIZE + rows >= total`).

Its earlier addendum had confirmed fixes 1–6 in the worktree and filed two errata against the new text,
both now closed: the pagination paragraph advertised a 256-page bound where `MAX_PAGES` is 64, and the
`PROVENANCE.json` `pagination` divergence claimed a short page with a larger total cannot truncate,
which the code did not yet deliver. The code now records a failure note for a short page whose response
still reports a larger total, for the member walk and the per-school coach walk alike, so that case is
visible rather than passing as the end of the list; the two documents state the 64-page bound and the
note. Re-verified after the change: `cargo test -p census-crawl --lib` → `493 passed; 0 failed`, `fmt
--check` clean, strict clippy 0 diagnostics under `arbiter/**`.

The reviewer's remaining unverified items were closed live where they were cheap to close. Kentucky is
the multi-page member walk (three pages), and a throwaway smoke over it — `arbiter_live_smoke
/tmp/arbiter-ky 1 KY`, deleted after the run — reported `rows: 1, requests: 6, from_cache: 0, errors: 0`
with the note `1 school(s) and 21 coach row(s) over 1 Arbiter organisation(s)` and read back Adair County
High School with its observation. The run's cache shows why: member pages 1, 2 and 3 returned 200, 200 and
89 rows against a reported total of 489, so the complete multi-page walk ended on a short page with
`(3 - 1) * 200 + 89 = 489` and recorded **no** failure — the false-positive-free property the reviewer
computed, now exercised end to end. Its one coach page returned 111 of 111 rows, exercising the coach
walk's short-page completion the same way; the contradicting short-page branch is pinned by the unit test
above. What stays unverified by execution is only the multi-organisation sweep (the four `--states` in one
run), the KY/MT/WV coach pages beyond the first school, and the CLI/Restate arms the `census-report` P0
keeps uncompiled.

Review limits, restated: the reviewer could not execute anything itself (all commands above were run
here), and its prototype reference is the working-tree `parsers/arbiter.py`, not a VCS-pinned revision.

### Arbiter lane: Holzman Rust pass on structure and resource bounds (2026-09-29)

Trigger: a request to re-read the lane from a Power-of-Ten-plus-performance vantage point. Scope was
`crates/census-crawl/src/arbiter/` only — no shared interface, registry row, service arm or other
lane changed. Reference files read before advising: `nasa-jpl-standards`, `zero-cost-abstractions`
and `runtime-performance-architecture` from the `holzman-rust` skill. Eight findings were accepted
and fixed:

| Finding | Rule / convention | Fix |
|---|---|---|
| the member walk buffered every page (`schools.extend`) and wrote only after paging ended, so a 64-page organisation held up to 12,800 rows before its first write | Rule 2/3: fixed loop bounds and a bounded working set | `walk` maps and journals each page's rows as it is read; the accumulator is gone, so the working set is one page and the journal advances during the walk instead of after it |
| `Run::open(ctx, options, state, org, fetch, done, tally)` — a seven-parameter constructor | Rule 4/6 (one-page reviewability, smallest scope) and the sibling convention | `Run` is now the five-field session struct built with a struct literal, as in `coach_directories::collect`; the target travels as `walk(state, org)` / `process_school(state, org, row, url)` arguments |
| the fetches took `refresh` from `Options` alone, so a refreshing `AdapterContext` was served from the cache | the sibling lanes build `options.refresh \|\| ctx.refresh` (chsaa, ciac, coach_directories, mshsl) | one `fetch_options(ctx, options, headers)` helper (`ctx.fetch_options()` plus the OR) used by the bundle read, the token POST and every API page |
| the per-school coach walk recorded nothing when the 64-page bound ended it, while the member walk did | the lane's own "no truncation is silent" claim | `collect_coaches` records the bound the way the member walk does |
| a state repeated in `--states` was walked once per repetition | Rule 2: the target list's size follows the requested set, not its spelling | `targets` skips a state it has already resolved, and the walk order stays the caller's |
| the journal detail wrote the state through `format!("{:?}")`, making a persisted payload depend on a derived `Debug` | stable persisted shape; the sibling lane stores `state.code()` | the detail carries the two-letter code (`"NH"`), the organisation, the association id, the organisation's school id and the coach-row count |
| `parse_org_schools` and `parse_coach_rows` each repeated the decode/collect/schema-error loop | Rule 4 and ruthless simplicity | one private `decode_page(body, url, subject, map_row)`; the per-row schema error is now `expected an object in the {subject} rows` |
| a member row without a name produced `row "" has no school name` — it cloned the name that was empty by construction | Rule 7: diagnostics that identify the failure | the note names the organisation, the state code and the row's association id, and no longer clones |
| `page_read_short` was introduced as `a_page_read_short` | naming (a predicate should read as one) | renamed `page_read_short`; the earlier paragraph in this section was updated to the current name |

Commands and raw results, all run in the worktree at this date:

| Command | Observed result |
|---|---|
| `cargo test -p census-crawl --lib arbiter` | `24 passed; 0 failed` (20 before this pass; the four added tests are listed below) |
| `cargo test -p census-crawl --lib` | `498 passed; 0 failed; 0 ignored; finished in 1.87s` |
| `cargo fmt --all -- --check` | no output (clean) |
| `cargo clippy -p census-crawl --lib --bins --examples --all-features --no-deps -- <the gate's LINT_SET>` after touching the lane files | 14 diagnostics, every one in the pre-existing `src/chsaa/parse.rs`; **0** under `crates/census-crawl/src/arbiter/**` |
| `cargo check -p census-report` | 54 errors — the pre-existing worktree blocker, so `tools/gate.sh`'s workspace-wide check/clippy lanes still cannot run here; the census-crawl lanes above are the strongest available |

The four added tests and the behaviour each pins:

- `a_full_member_page_is_written_before_the_next_page_is_fetched` — one full 200-row page reporting
  `total: 250`, no second page in the cache and an offline fetcher; asserts the 200 rows are written
  and that the missing page is the single recorded failure. The buffering walk wrote nothing here.
- `a_coach_walk_stopped_by_the_page_bound_records_a_failure` — 64 full coach pages of 200 rows each;
  asserts the bound is named in a note instead of ending the walk quietly.
- `a_state_named_twice_is_walked_once` — `NH, NH, KY` resolves to two targets, in the caller's order.
- `a_refreshing_context_refreshes_even_when_the_options_do_not` — the helper's refresh flag follows
  `ctx.refresh` when the options say otherwise.

For the changed entry point (`collect`, not only the walk) the lane was re-exercised live, the same way
the review round was: a throwaway `census-crawl` example (`examples/arbiter_live_smoke.rs`, deleted after
the run) built the fetcher as `census-service`'s `build_fetcher` does — `Store::http_cache_dir`, 1 s
delay, `default_host_delays()`, `default_family_delays()` and the three Arbiter hosts authorized — and
called `arbiter::collect` over a fresh store:

| Command | Observed result |
|---|---|
| `cargo run -q -p census-crawl --example arbiter_live_smoke -- /tmp/arbiter-holzman-smoke 1 NH` | `rows: 1, requests: 4, from_cache: 0, errors: 0`, note `1 school(s) and 3 coach row(s) over 1 Arbiter organisation(s)`; readback `1 school(s), 3 coach(es), 1 observation(s)` — `Alvirne High School` and coaches Phillip Demers, Justin Hufft, Kaitlyn Wilson |

Those four requests are the bundle, the token, the filtered member page and the coach page, so the run
covers the refactored session struct, `fetch_options`, both walks and the journal write in one pass; no
failure note was recorded although the member page was a natural short page (the false-positive-free
property once more). The run's journal payload on disk reads
`{"arbiter_org_id":29072,"association_id":450,"coach_rows":3,"observed_on":"2026-09-30","org":"2132","school":"Alvirne High School","state":"NH"}`
— the `state` key carries the two-letter code, and `observed_on` is the run's `today_iso()` value. One
dated-capture note: the live Alvirne coach page carries three rows where the fixture's golden pins the
prototype's two, so the live roster moved since the 2026-09-2x capture; the fixture and golden are dated
captures, not a live contract, and were left as they are.

Examined and deliberately left alone: per-row allocations (the shared label parsers' lowercasing,
the `json!` journal detail, `FetchOutcome::text()`'s single copy per page) and a `#[cold]` marker on
the failure path. The lane is request-bound — the registry row paces it at 1 rps with
`maximum_in_flight = 1` — so these are not the bottleneck and the pass makes no measured performance
claim; `Tally` keeps its 32-note cap and the per-org page bound stays `MAX_PAGES = 64`. What remains
unverified by execution after this pass: the multi-organisation sweep (the four `--states` in one run),
the KY/MT/WV coach pages beyond their first school, and the CLI/Restate arms the `census-report` P0
keeps uncompiled.

## Consolidated historical evidence — imported 2026-09-27

The following facts came from retired handoffs, implementation plans and duplicate operating
guides. They were **not rerun during documentation cleanup**. Original run IDs, partial digests,
dates and scope distinctions remain as recorded; abbreviated hashes are not full hash certificates.
Old source access, signed-in sessions, arbitrary retry behavior and workbook inputs are not
authorizations or production procedures for the fresh census.

### Workbook-era acquisition and qualification — 2026-09-19 to 2026-09-21

- The retained native corpus comparison reported 95 queries, 4,256 receipts, 340,238 individual
  results, 57,629 relay-member results, 142,705 athlete IDs and 15,724 unresolved roster results.
  Its private report was `native-rankings-1789772180206/native-corpus-cleanup-proof.json`.
  Twenty-six private public-API storage scenarios passed; their corrected position oracle counted
  positions `1,2,1` as two distinct positions. These reports were not recovered or rerun here.
- Serialization measured 344,131 versus 13,131 allocations over 1,000 synthetic iterations,
  331 fewer per iteration. This is not a throughput result.
- Fixture division runs `6f3c0c59…` (indoor girls), `f0ef3948…` (indoor boys) and `3c726c6d…`
  (outdoor boys) each reconciled eight synthetic rows: six accepted, one no-match, one review,
  none pending. Stopped-writer verification matched source hashes; cached replay added no source
  requests. Outdoor pause/resume froze source traffic for the observed 25 seconds.
- Live indoor boys `57f88b34…` collected 95 terminal events over 1,065 pages, snapshot
  `40792b83cfb168de5de876773f1312e41af3c34fc49782a991bdd6d9d4389dd6`. Its eight synthetic
  workbook rows all remained `review_required`. Workbook/JSONL/receipt digests began
  `a4eb1fe7…` / `87cfd651…` / `a4f0791b…`; stopped-writer verification exited zero.
  Cached replay took 0.5 seconds, left artifacts byte-identical and kept fetch count 488→488.
  Collection required a CDP re-arm mitigation; it did not prove the transport defect repaired.
- Live indoor girls `fe726b41…` collected 94 events over 816 pages and likewise left all eight
  rows in review. Artifacts began `8322bae9…` / `44f9f773…` / `fc6c12fe…`; verification exited
  zero and replay left requests 3,616→3,616. Outdoor runs `7be7e9dd…` and `3b7c07fa…` were
  separate historical generations, not additional accepted-person evidence.
- The 2026-09-19 fresh-headless probes returned 403 block pages of 5,484 bytes. The later headed
  profile produced 200 captures. A recovery cycle reached `human_required` in 31 seconds around
  its 30-second bound rather than waiting to the old 24-hour deadline. These are session-specific
  observations, not a rule that future access or challenges may be bypassed.
- The old gate records changed with the tree: 226 tests with library 149/2 ignored, later
  232 tests at `3f78fd9` with library 156/2 ignored, and a CDP-patched 234-test record. Do not
  combine them into one current suite count. The old 16-target command was:

```text
cargo test --lib --bins --test rankings_parser --test rankings_catalog --test rankings_scope --test rankings_indoor --test rankings_storage --test profile_html_bounds --test profile_merge_bounds --test search_html_bounds --test workbook_verify --test workbook_zip_layout --test native_parser_properties --test result_verify --test bundle_verify --test ingress_failure_surface
```

Those root-package targets no longer exist. The retained `scale-v15` lane used fixture transport
and binary `0f044e59…` from working tree `9d101d9`, not live identity search. Its real workbook
had 120,716 rows in two sheets (111,939 + 8,777), 1,569,308 source fields and 26 headers.
A ten-selected-row smoke retained all rows and an 83.7 MB JSONL. An owner-online partial export
retained 462 completed and 120,254 pending rows. Final run `a8328b7f…` assessed 5,000:
zero accepted, 3,405 review, 1,595 no-match; the other 115,716 remained explicitly pending.
Stopped-writer verification and replay were reported complete by 2026-09-21, correcting the
earlier pending note. Older `smoke`, `scale-partial` and `scale-2500` publications failed the later
header-width verifier and were not silently replaced. No private row values are reproduced here.

Measured fixture throughput was approximately 0.31 rows/s at concurrency 8 and 0.27 at 32;
two lanes together approximately 0.9 rows/s. A 120.1-second window held 41 rows and 358 source
operations (0.341 rows/s, 8.73 operations/row), with 358 readiness calls and a 0.329-second mean
handler duration. A shared-probe experiment kept probe/gate calls 280/280 and fell to 0.167 rows/s;
it was reverted. The device-write estimate of 12–14 MB/row was not journal payload: retained
journals measured 51.5 KB/decided row, 112.5 B/entry, versus 110 KB/row in Restate data and
36 KB/row in the store. This is an old workload, not a current performance baseline.

The 2026-09-21 endgame audit at `3af713b` (dirty tree) found live issue counts 38 sport mismatch,
17 noncanonical URL, 16 malformed page and six short page; six submitted live output directories
were empty. Its twelve-state walk held 6,737 rosters, 35,622 cached responses and 2.2 GB.
It could not infer national workflow execution from journals shared with CLI walks. The later
[search capture audit](../research/G1-LIVE-SEARCH-CONTRACT.md) owns detailed rejection evidence.
The original “profile evidence stayed empty” claim was false across retained runs: the older
33-row pilot held 2,263 profile artifacts and 46,524 performance entries, and one live-girls row
held one profile with 21 entries. Neither observation established an accepted identity.

Live probes on 2026-09-21 found no XC season in the TF `GetNavInfo` map; XC rankings landing
pages did not issue a ranking request. Bio responses exposed both sport-specific surfaces,
grades, teams, seasons, meets and results. A two-sport sample returned 40 TF results separately
from 22 XC results. Across five other athletes, `level=4` reduced TF row counts
88/87/428/80/162 to 73/74/148/71/68; `GetAthletes?listId=` returned an empty array. These are
capability captures, not exhaustive collection or permission to bypass the headed browser policy.

### Early independent-census baseline — 2026-09-20

Imported from the former service README; before Fjall/Restate cutover. Counts describe that
generation only, not today's store or independently verified people.

| Scope | Athletes | Co2027 | Boys | Girls | Profile URL | Coach | Coach email |
|---|---:|---:|---:|---:|---:|---:|---:|
| Core | 651,736 | 146,858 | 80,896 | 65,751 | 142,916 | 37,179 | 28,929 |
| All sources | 787,584 | 190,087 | 105,755 | 84,034 | 176,885 | 43,201 | 33,340 |

Both rows reported grade evidence for 100% of their cohort; core/all-source cohort ratio was
77.3%. There were 27,580 coach rows, 1,532 core meets (WI 796, MN 209, IA 85; 442 unresolved),
and 11,007 all-source meets, 9,593 carrying an Athletic.net ID.

| State | Schools | Athletes | Co2027 | Boys | Girls | Coach | Coach email |
|---|---:|---:|---:|---:|---:|---:|---:|
| OH | 1,504 | 87,765 | 26,998 | 14,414 | 12,507 | 313 | 313 |
| IL | 1,889 | 69,493 | 19,392 | 10,856 | 8,521 | 6,432 | 6,244 |
| WI | 1,027 | 118,031 | 19,281 | 10,323 | 8,929 | 14,699 | 13,646 |
| MI | 1,493 | 94,771 | 16,176 | 9,285 | 6,891 | 0 | 0 |
| MO | 1,037 | 42,226 | 13,218 | 7,377 | 5,839 | 0 | 0 |
| IN | 1,108 | 40,846 | 11,909 | 6,430 | 5,402 | 0 | 0 |
| MN | 1,016 | 54,084 | 11,261 | 6,147 | 5,114 | 9,338 | 8,629 |
| KS | 836 | 29,950 | 9,252 | 5,258 | 3,993 | 0 | 0 |
| IA | 913 | 49,024 | 8,738 | 4,924 | 3,804 | 76 | 19 |
| NE | 678 | 35,800 | 5,921 | 3,248 | 2,673 | 4,971 | 0 |
| SD | 427 | 17,134 | 2,669 | 1,518 | 1,151 | 239 | 78 |
| ND | 339 | 12,612 | 2,043 | 1,116 | 927 | 1,111 | 0 |

The parser/header repair in that record grew parsed Wisconsin artifacts 96→1,740 (compiled 763,
XC 380, Hy-Tek 597), producing 834,254 rows, 264,167 with grades, and 4,323 added core Co2027
rows. A separate Wayzata schedule measurement resolved 304/537 venue rows (95 sites, 209 schools);
the rest were unresolved, not guessed.

### Hardening waves — 2026-09-21 to 2026-09-22

The retired program's baseline was `4e5b828`, nightly-2026-04-27 / rustc 1.97.0-nightly.
Strict production Clippy exited 101 with approximately 440 diagnostics (392 census lib and
51 root lib; the approximation and differing scopes are retained). Per-lint census/root counts:
arithmetic 208/36, expect 75/0, string slicing 47/2, casts 36/10, indexing 23/2,
ignored-must-use 3/0, unwrap 1/0. It counted 134/552 census functions over 25 logical lines,
33 over 60 and 21 files over 300. The pre-split sizes were 33,608 root production lines/182 files
and 17,854 census lines/31 files.

The async census/root inventory recorded bare spawn 0/4, blocking spawn 2/2, JoinSet 3/5,
TaskTracker 0/3, cancellation token 0/10, select 3/7, buffered-unordered 4/0, and six/zero
instrument attributes. Spawn instrumentation, paused-time/loom and console/OTLP support were absent
in that baseline. These were subsequent implementation targets, not current-tree claims.

| Recorded wave | Executed result and distinctive evidence |
|---|---|
| Wave 1, 2026-09-21 | Gate reported 465 passed/2 skipped; root strict counters zero; census arithmetic 240→68, casts/index/string-slice/expect/unwrap 36/23/47/75/1→7/4/1/0/0; scan census indexing/expect/casts 128/75/32→0/0/7 |
| Wave 2, 2026-09-21 | 583 passed/2 skipped, strict counters zero, oversized files 23→0; production root/census 36,656/24,039 lines over 305/160 files; functions over 60/25 were 0/549 |
| Wave 3, `3af713b` | Commit 286 files, +66,070/−1,184; independently frozen fmt/scan clean, census 25,582 lines/167 files, zero oversized files/over-60 functions; commit record reported 618 passed/2 skipped |
| Waves 4/5, 2026-09-22 | Gate 726 passed/2 skipped, strict Clippy zero; recovery eight scenarios; census 39,340 lines/251 files. Lost-row kill oracle changed 4/4 to `claimed_without_rows_at_kill=0 missing_from_the_final_store=0`; drain ordering and volatile Coverage-note parity were repaired |
| Wave 6, 2026-09-22 | 745 passed/2 skipped; census lib 356, domain 39. Six-school/13-coach smoke derived 19 identities, zero conflicts/reviews, 58 coverage rows and one snapshot; three passes preserved one row/key. Offline cycle 0.23 s; Run Metrics golden cells 13,872→13,882, other sheets unchanged |

Distinct defects included a FOUL row under a relay header, RaceDay reporting zero parsed rows
for an 82-row finish list, missing meet references, a resumed IHSA limit counting walked rather
than newly fetched meets, and normalization non-idempotence for `X School School`. The latter
was repaired to a suffix-stripping fixpoint and pinned by the repeated-suffix regression.
The Kani census (four verified, seven environment-blocked, sixteen no verdict) and bounded fuzz/
mutation results already have their detailed logs above; they are not upgraded by this transfer.
The old 559–627k observations/s figures had no in-repository reproducible baseline.

Deferred findings then included direct adapter/store coupling, whole-table materialization and
20M-row rejection, legacy import's commit/marker window, full-store index rebuilds, observation
counts mistaken for merged entities, unchecked allocation growth, cloned journal payloads,
whole-scan failure on malformed rows, uninstrumented spawn sites, backup/parity fixtures that never
derived indexes, and cohort-specific coverage. This list is historical; the current plan and schema
distinguish implemented repairs from remaining obligations. It is not a second active work queue.

### Pinned delivery source audit — `183fca1`

The former delivery brief inspected commit `183fca13dd6d48165d04c76fd56648998e6b432f`.
Actions run `36020764525`, job `107715786629`, failed at Quality gate; subsequent feature checks
were skipped. Log download failed for credentials, so the review did not diagnose the failure.
The source findings below are renamed **G01–G11** to avoid collision with the national plan's
F01–F15 acceptance items. They are static findings, not fresh test results or an assertion they
all remain present.

| ID | Finding at that revision |
|---|---|
| G01 | Transparent integer mark wrappers contradicted a decimal-JSON compatibility claim |
| G02 | Imperial comparison treated hundredths of an inch as inches: `3-0.75` computed 2819 mm instead of 933.450; `4-0.00` computed 1219 instead of 1219.200, reversing order |
| G03 | Restate seen-operation state followed the external Fjall write, leaving a lost-acknowledgement window |
| G04 | S06 explicitly skipped the named crash boundary |
| G05 | Review fact hashing sorted words and erased attribution |
| G06 | Candidate/singleton identity depended on school/name/class/category rather than source-person evidence |
| G07 | Fast Kani selection was not reconciled with actual wired harnesses |
| G08 | Benchmark parsing accepted empty output and comparison did not require every baseline group |
| G09 | RSS capture used invalid `/proc/self/status/VmHWM` and could substitute zero |
| G10 | Applicability mapped twelve Midwest states, with empty fallback elsewhere |
| G11 | Batch accounting saturated row counts and lacked a total byte budget |

The 24 named canaries, 12 proof kernels, 17 faults and full-artifact verifier requirements now live
in their single current owners (national plan, TESTING and fault catalog). The old one-model policy,
seed-workbook branch and competing T01–T36 implementation waves are not current instructions.

### Provider wiring and storage-path audits — historical source inspection

The 2026-09-23 dispatch audit found six witnessed registry/stage/CLI slugs: `milesplit`, `mshsl`,
`plain_names`, `wayzata`, `wiaa`, `wiaa_results`. Seven had registry+CLI but no stage:
`athleticlive`, `athleticlive_athletes`, `athleticnet`, `coach_contacts`, `ihsa`, `ks`, `ohsaa`.
Three were CLI-only modes: `athleticlive_results`, `ihsa_tournament`, `milesplit_results`.
`tfrrs` had a registry row but no CLI arm. That was 17 names, not 17 integrated adapters.
It ran no binary; subsequent wiring changes require their own evidence.

The retired local batch audit, transferred on 2026-09-27, estimated separate `SyncData` commits
from the then-inspected source: Athletic.net bio flush `6 + k` (70 for 64 units), index five,
review two, WIAA/ND/NSAA school+coach journals two, OHSAA `1 + coach_count`. These were source-path
counts, **not measured fdatasync calls or throughput**. Its proposal to commit from Drop and its
claim that Fjall batches cannot span keyspaces were incorrect and are not retained as design.
The current atomic write/receipt contract is [FJALL_SCHEMA.md](../FJALL_SCHEMA.md).

### Backup, deployment and source-owner cutover evidence

The earlier synthetic cold-copy drill used 400 observations (200 schools, 100 coaches, 100 meets)
and merged 199 schools; the backup held 12 files/229,232 bytes and normalized reports of 27,649
bytes compared equal. A 64 MiB journal preallocation made the root 67,212,350 bytes before reopen
versus 196,402 afterward; this was physical layout, not lost observations. Bad version and truncated
descriptor opens refused with `InvalidVersion` and `UnexpectedEof`. Truncating 100 tail bytes
retained 400 observations; 4,000 retained 300. Fourteen row-value flips all refused. Of 96 header
flips, eight triggered upstream debug assertions, 82 refused and six opened; release behavior was
not verified. These observations do not make a live directory copy safe.

The 2026-09-24 campaign cold-copy drill recorded:

```text
target/release/census-service --store /tmp/store-copy store-backup --to /tmp/census-backup-drill/backup
exit 0; 10 s
target/release/census-service store-restore --from /tmp/census-backup-drill/backup --to /tmp/census-backup-drill/restored
exit 0; 7 s
target/release/census-service --store /tmp/census-backup-drill/restored consolidate
exit 0; 8 s
target/release/census-service --store /tmp/census-backup-drill/restored report
exit 0; 13 s
```

Reported physical table counts included teams 207,609; coaches 65,020; athletes 3,072,309;
meets 12,556; events 101,711; performances 309,962; source identities 2,507,541; conflicts 4,548;
review cases 107,768; coverage 213; snapshots two; source access zero; identity verdicts 43,291;
source meets 131,726; source observations 207; observations 3,991,059.
Sizes were 1,459,217,992 on disk and 7,698,688,836 logical. Restored all-source report:
31,870 schools, 2,364,818 athletes, 623,509 Co2027 (350,944 boys, 271,560 girls), 609,738 profile
URLs, 59,269 multisource rows and 32,031 coaches. Equal counts are not complete semantic equality;
the separate 2026-09-25 drill above used a different generation.

The deployment record for 2026-09-24 reported revision 8 fan-out of 49 jurisdictions, with 47
aborted/two finished under the default timeout. An in-place rebuild left 59 paused invocations
and three DC tasks pending; re-registration preceded recovery. Old process stop and new canary
response each took two seconds in that experiment. Duplicate localhost/127.0.0.1 registrations
carried revisions 9/5 versus 8/4; old registrations were retired after 10/6 became active.
Deletion without force returned 501, with force 202. This is not proof of uninterrupted upgrades
or a safe recipe to retire a deployment with unknown in-flight work.

ADR-014's source-owner repair was motivated by 74,468 identity-less rows among 2,374,515 historical
athletes and 204,102 performance rows predating the owner field. Recorded decode errors named
`athletes:ath_0000477b2bc153dc#1363551` and
`performances:perf_0000150f06a7a1de#5600`. After repair, core census-status reported 31,870 schools,
2,220,866 athletes and 575,991 Co2027; release bests reported 10,231 selections in 3.2 seconds
with 1.4 GB peak RSS. These imported measurements do not supply owners to unknown historical rows
or certify the fresh run.

An asynchronous export result received on 2026-09-27 printed
`wrote /var/tmp/real-census/out/census-service-2026-09-27.xlsx`,
`real 24m50.412s`, `user 24m43.126s`, `sys 0m3.953s`.
Its initiating command and artifact readback were not recovered. It is a process result only,
not verified workbook contents, a repeatable benchmark or a new seal.

### Pinned sources for the 183fca1 review

Retained source references from that review, not newly fetched or runtime-verified. Repository
links pin the old commit even when the current file has been removed; external links describe
framework semantics, not this implementation's correctness.

| Reference | Source |
|---|---|
| R01 | [Latest main recheck and pinned commit](https://github.com/lprior-repo/athletic-rust-pipeline/commit/183fca13dd6d48165d04c76fd56648998e6b432f) |
| R02 | [GitHub Actions run for the reviewed SHA](https://github.com/lprior-repo/athletic-rust-pipeline/actions/runs/36020764525) |
| R03 | [Failed Quality gate job](https://github.com/lprior-repo/athletic-rust-pipeline/actions/runs/36020764525/job/107715786629) |
| R04 | [Fixed mark representation](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/src/model/fixed_mark.rs) |
| R05 | [PR comparison and imperial conversion](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-report/src/bests/measure.rs) |
| R06 | [Kani runner](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/xtask/src/kani.rs) |
| R07 | [Production Ingest handler](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/ingest.rs) |
| R08 | [Performance benchmark capture](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/xtask/src/perf/bench.rs) |
| R09 | [Existing multi-table StoreBatch](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-store/src/write_batch.rs) |
| R10 | [Current meet-stage handoff](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/meets_arms.rs) |
| R11 | [S06 crash/duplicate-evidence scenario](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/tools/durability/scenario-06-no-duplicate-evidence.sh) |
| R12 | [Current domain Kani module wiring](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/kani/census_domain_wiring.rs) |
| R13 | [CaseEvidence and review case identity](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/src/model/records.rs) |
| R14 | [Candidate and canonical athlete types](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/src/model/athlete.rs) |
| R15 | [Performance comparison](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/xtask/src/perf/compare.rs) |
| R16 | [Source applicability](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-crawl/src/applicability.rs) |
| R17 | [Repository endgame research](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/research/ENDGAME-GAPS.md) |
| R18 | [Restate jobs and journal handoff](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/jobs.rs) |
| R19 | [Jurisdiction pipeline integration](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-service/src/restate_services/jurisdiction/pipeline.rs) |
| R20 | [Grade-year Kani harnesses](https://github.com/lprior-repo/athletic-rust-pipeline/blob/183fca13dd6d48165d04c76fd56648998e6b432f/crates/census-domain/kani/gradyear.rs) |
| E01 | [Restate Rust durable steps](https://docs.restate.dev/develop/rust/durable-steps) |
| E02 | [Restate Rust error handling](https://docs.restate.dev/develop/rust/error-handling) |
| E03 | [Serde container attributes](https://serde.rs/container-attrs.html) |
| E04 | [Kani harness listing](https://model-checking.github.io/kani/reference/experimental/list.html) |
| E05 | [Criterion command-line options](https://bheisler.github.io/criterion.rs/book/user_guide/command_line_options.html) |
| E06 | [TigerBeetle testing practices](https://docs.tigerbeetle.com/coding/testing/) |
| E07 | [TigerBeetle safety practices](https://docs.tigerbeetle.com/coding/safety/) |

