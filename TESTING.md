# TESTING.md — the test suite, its fixtures and the gate

Where tests live, what they assert, how to run one of them, and what the suite refuses to prove.
Structure only: this document makes no claim about the current pass/fail state of any lane. The
gate is the answer to "is it green"; `tools/gate.sh` prints it per lane.

Nine workspace members (`Cargo.toml`): `crates/athleticnet-browser`, `crates/census-domain`,
`crates/census-crawl`, `crates/census-reconcile`, `crates/census-report`, `crates/census-review`,
`crates/census-store`, `crates/census-service` and `xtask`. Test-bearing code lives in every one of
them. Tests are unit tests inside `#[cfg(test)]` modules, integration tests under `tests/`, and
generated-at-runtime workbook fixtures assembled into `tempfile::tempdir()`.

Counts below come from counting `#[test]` / `#[tokio::test]` attributes plus `proptest!` blocks under
`crates/*/src/**` and `crates/*/tests/**`, recounted 2026-09-21 while the acquisition root package
still existed: **598 attributes + 8 `proptest!` blocks across 104 files** — that root's `src/` 178
(41 files), `crates/census-service/src/` 253 (33), the root's `tests/` 75 + 1 `proptest!` block (15), census
`tests/` 66 + 7 `proptest!` blocks (13). The root's own 178 and 75 left with it; the totals above are from
that 2026-09-21 run and are not reproducible from the per-area breakdown (the areas sum to 572 / 102, leaving a
difference that cannot be accounted for after the root's deletion). Per-area counts in §2 carry the same date;
the "names and locations are the durable part" caveat applies only to rows whose current paths I verified
against the tree — the two root-crate sections (xlsx/workbook/store and network/browser/runtime) are not
durable, since the modules went with the deleted root. To recount the surviving crates, count
`#\[(test|tokio::test)` and `proptest!\s*\{` over `crates/*/src/**` and `crates/*/tests/**`.

---

## 1. How to run

| Purpose | Command |
|---|---|
| Every lane, whole workspace | `tools/gate.sh` |
| Rewrite `tools/quality-baseline.json` from current measurements | `tools/gate.sh --update-baseline` |
| Same, explicitly accepting debt growth | `tools/gate.sh --update-baseline --allow-increase` |
| All tests, both crates (nextest) | `cargo nextest run --workspace --all-features` |
| All tests without nextest installed | `cargo test --workspace --all-features` |
| One crate | `cargo nextest run -p census-service --all-features` |
| Unit tests only (inline `#[cfg(test)]` modules) | `cargo test --lib` (or `cargo nextest run --lib`) |
| One integration target | ~~`cargo nextest run --test result_verify --all-features`~~ (historical: `result_verify` test target belonged to the deleted root package; no such test exists in current crates) |
| Fixture-based adapter tests (captures are embedded at compile time, so no flags and no fixture server are needed) | `cargo nextest run -p census-service --all-features`, or one adapter: `cargo test -p census-service sources::mshsl::` |
| One test by name (nextest expression) | `cargo nextest run -E 'test(=store_round_trip_merges_observations_and_reports_stats)'` |
| One test by name (plain cargo) | ~~`cargo test --test result_verify verifies_deterministic_positive`~~ (historical: `result_verify` test target belonged to the deleted root package; no such test exists in current crates) |
| Census storage/service chain (in-process, no external server) | `cargo nextest run -p census-service --test fjall_restate_e2e` |
| Operator CLI against a fake ingress | ~~`cargo nextest run --test ingress_failure_surface`~~ (historical: `ingress_failure_surface` test target belonged to the deleted root package; no such test exists in current crates) |
| Opt-in CDP lane (2 ignored tests) | `cargo test --lib -- --ignored lane_smoke` |
| ~~Fixture origin + workbook for the lane~~ (historical: `examples/native_fixture.rs` belonged to the deleted root package; no such example exists in the workspace) | ~~`cargo run --release --example native_fixture -- --bind 127.0.0.1:21045 --output-dir var/native-fixture`~~ |

Notes that follow from the code, not from convention:

- **No `.config/nextest.toml` exists** (the directory does not exist anywhere in the repository), so
  nextest runs with defaults: no retries, no per-test timeout, no test groups, no serialisation
  overrides. `cargo nextest run` also does not run doc tests; `cargo test` does.
- The ignored CDP tests read two environment variables with loopback defaults:
  `ADLAW_LANE_FIXTURE` (default `http://127.0.0.1:21045/`) and `ADLAW_LANE_CDP` (default
  `http://127.0.0.1:9223`); see the `lane_smoke` module in
  ~~`src/runtime/browser/transport/rankings/`~~ (historical: path belonged to the deleted root crate; `crates/athleticnet-browser/src/transport/rankings/` is the current equivalent). They are `#[ignore = "requires a fixture origin and
  a CDP browser"]` and neither the gate nor a plain `cargo test` runs them.
- ~~`examples/native_fixture.rs` is the synthetic fixture origin: it refuses a non-loopback or
  zero port, writes a workbook plus `worker.toml` into `--output-dir`, prints a JSON summary with
  the bind URL and `/__fixture/{counters,control,reset}` control paths, then serves.~~ (historical:
  the example belonged to the deleted root package and no longer exists; the lane's origin must be
  supplied by the caller.)
- `tools/gate.sh` resolves its own path and `cd`s to the repository root, so it can be invoked from
  any working directory. It needs `bash`, `cargo` and `jq` unconditionally: only the
  cargo-subcommand lanes have a missing-tool guard (they print `SKIP: …` and pass).

### gate.sh flags and refusals

| Argument | Effect |
|---|---|
| none | Run every lane, compare debt against `tools/quality-baseline.json`, print a summary |
| `--update-baseline` | Rewrite the baseline from the current measurements, then stop (the summary exits before `deny`/`audit`/`machete`/`geiger`/bench lanes run) |
| `--allow-increase` | Only meaningful with `--update-baseline`: permits the baseline to rise |
| anything else | `unknown argument: …` on stderr, **exit 2** |

The gate refuses to:

- raise a baseline number without `--allow-increase` (`cargo xtask quality-baseline` prints
  `refusing to raise the baseline without --allow-increase:` and exits 1);
- run a single lane by name — there is no lane selector, so a single lane is run by running the
  lane's own command (table in §6);
- format, fix, or update anything: the fmt lane is `cargo fmt --all -- --check`, and the gate has
  no write path other than `--update-baseline`;
- run ignored tests, run doc tests under nextest, or execute benchmarks (a benchmark target is only
  *compiled* by the bench-presence lane);
- paper over a missing cargo subcommand — but the failure mode is a printed `SKIP`, not a red lane.

---

## 2. Test inventory by area

One row per area; "tests" is the 2026-09-21 attribute count, not a promise.

### Root crate — xlsx, workbook, store — **historical**: all paths belonged to the deleted root crate (`Cargo.toml` header + `ARCHITECTURE.md` §1)

| Area | Where | Tests | What the tests actually assert |
|---|---|---|---|
| xlsx streaming parse | `src/xlsx.rs` (`mod tests`), `src/xlsx_scope_tests.rs`, `src/xlsx_stream_tests.rs` | 15 | Column reference conversion (`column_index`/`column_name` round trips A/Z/AA/AC) and XML escaping; `ScanMode::FirstWorksheet` retains every row and field and treats empty first/last name cells as data; `ScanMode::Exhaustive` excludes the generated `Athletic Matches` sheet and errors when the first worksheet *is* generated; missing required headers are an error; sparse/rich/shared-string/inline rows keep their `excel_row` and raw field text (`" Alice Runner"`, `" Basketball "`); styled or explicitly-empty rows are not data rows (`xml_rows` 4 vs `actual_data_rows` 1). `visit_records` streams both source sheets with `source_key` `Export:5`/`Sheet1:9`, propagates a callback error without reading further rows, and rejects duplicate row positions and truncated worksheet XML. `BoundedReader` rejects real decompressed overflow, keeping the first 4 bytes. Writer test: `append_matches_sheet` keeps the source field text and writes a `HYPERLINK` to the profile URL into the appended sheet |
| zip container / record validation | `src/workbook_ingest/preflight/zip.rs` + `zip/records.rs` (no unit tests of their own), `tests/workbook_zip_layout.rs` | 4 | A `rust_xlsxwriter` workbook with streaming descriptors is accepted; a `zip64` (`large_file`) round trip preserves `Person Email`; a duplicated central-directory name is rejected only after the local and central names actually differ (the test asserts the byte positions first, so it cannot pass vacuously); a flipped byte in the local-file-header name is rejected |
| workbook verify (golden output) | `tests/workbook_verify.rs` | 5 | Coloured scalar semantics: sparse rows, text/number/bool/literal cells survive verification; `verify_fields` reports `source_sheet_count`, `matched_row_count`, `matched_field_count`, `appended_header_count` and source-hash matches before/after. Rejection cases cover missing, duplicated and moved rows, plus an expanded shared-string variant proving the digest is over the source *file*, not the parsed grid |
| annotation/bundle verify | `tests/bundle_verify.rs` | 1 | Annotation columns and the JSONL sidecar are checked against the source workbook + declared digest: tampering with either fails verification even though the original cells are preserved |
| artifact store round trip | `src/store/tests.rs` | 10 | Documents survive reopen and verify their digest; a tampered body yields `StoreError::DigestMismatch`; a re-published identical batch is idempotent but a changed row yields `SourceConflict`; ranking pages are addressable by `(collection, event_short, page)` with `RankingConflict` on a different payload and a droppable stale marker; source rows are isolated per workbook and ordered by sheet then numeric row (`Other:2`, `Sheet:2`, `Sheet:10`); paging resumes after gaps without crossing sheet boundaries and rejects a zero page limit; size/`MAX_BATCH_RECORDS` bounds and visitor errors are returned as typed errors; raw fields are retained without normalisation; a *missing* artifact is distinct from a corrupted one; and an existing store root with group/other access is refused with `StoreError::InsecureRoot` (unix-only) |
| store audit log | `src/store/audit.rs` | 1 | Two operations with identical effects stay distinct, durable and operation-bound (the retained record differs from the first write) |
| rankings storage | `tests/rankings_storage.rs` | 3 | Exact page replay preserves counts before and after sealing; a collection counts real athletes across variable-length event keys; an athlete lookup preserves every record in one checkpoint and reports `truncated` honestly |

### Root crate — network, browser, runtime — **historical**: all paths belonged to the deleted root crate (`Cargo.toml` header + `ARCHITECTURE.md` §1)

| Area | Where | Tests | What the tests actually assert |
|---|---|---|---|
| source request construction | `src/runtime/source/request/`, `src/runtime/source/tests.rs` | 13 | The search body is exact and percent-encoded; captured requests survive durable (de)serialization with `PartialEq`; unsafe search/profile inputs are rejected; the rankings query encodes the measured body byte-for-byte, emits `[]` (not `null`) for a missing grade, keeps the semantic UI URL (`/TrackAndField/rankings/list/<id>/<gender>/<event>/?page=&grades=`) separate from the physical API URL (`/api/v1/tfRankings/GetRankings`, empty query), and rejects page 0 and oversized event names; the team route retains the indoor season identifier `12025` |
| retry/backoff policy | `src/runtime/source/retry.rs`, `src/runtime/source/http/challenge.rs`, `src/runtime/source/result.rs`, `src/runtime/source/http/tests.rs` | 12 | `Retry-After` accepts delta-seconds and HTTP-date forms and fails closed on duplicate, signed or excessive values; only 429/5xx are retryable; a valid `Retry-After` overrides the 120 s rate-limit fallback while an absent header uses it; a receipt-less transport fault is retryable for rankings but any *observed* fault (403/429/challenge/parse with a receipt) never is; access-denied codes block even on HTTP 200; exhaustion retains every receipt and the final cooldown; the CF challenge header is definitive while a passive script tag alone is not, and challenge markers beyond the scan bound are ignored |
| browser request scheduling and state | `src/runtime/browser/request.rs`, `src/runtime/browser/state.rs`, `src/runtime/browser/lifecycle.rs`, `src/runtime/browser_session.rs`, `src/runtime/browser/transport/rankings_helper/` | 5 (+2 ignored) | `request.rs` and `state.rs` carry **no** `#[cfg(test)]` module; their behaviour is asserted indirectly: `lifecycle.rs::only_a_terminal_stopped_manager_is_rebuilt`, `browser_session.rs::stalls_and_challenges_escalate_while_settled_states_do_not` (which states can escalate), and the helper tests `terminal_page_null_is_absent_not_protocol_failure`, `invalid_page_values_do_not_become_terminal_pages`, `navigation_url_is_already_canonical`. The scheduler itself is covered only by the two ignored `lane_smoke` tests: one asserts that a served results capture costs **exactly one physical POST** (with status 200, the measured body, `next_page = 2` and an open gate), the other that a challenged page returns 403, revokes the gate and never advances pagination |
| CDP model compatibility | `tests/cdp_frame_compat.rs` | 1 | A captured (sanitized) Chromium 151 `Network.requestWillBeSentExtraInfo` frame deserializes, keeping `client_security_state` and cookie block reasons. This fails if the local `vendor/chromiumoxide_cdp` patch stops applying — the reason for the `[patch.crates-io]` entry |
| runtime workers / reviewer / export | `src/runtime/rankings/page/parse/main/`, `src/runtime/rankings/division.rs`, `src/runtime/reviewer/`, `src/runtime/reviewer/model.rs`, `src/runtime/reviewer/transport.rs`, `src/runtime/review_case.rs`, `src/runtime/row_worker/support.rs`, `src/runtime/profile_worker/team.rs`, `src/runtime/export/index.rs`, `src/runtime/export/projection/tests.rs`, `src/runtime/artifacts.rs`, `src/search/parser.rs` | 35 | Past-end pages parse as the exhausted list while any other page-identity conflict stays an error; masked relay rows are recorded unresolved; division list ids/revisions are pinned per (season, gender) and unsupported season years have no list; the reviewer verdict parser rejects invented ids, unsupported references, refusals, truncation and ambiguous shapes; `Retry-After` beyond the SDK delay becomes non-retryable with a cooldown; the reviewer case hash ignores the physical source location but changes with identity proof/model; export coverage reconciliation rejects dropped pending rows, differing resolutions, differing totals against sealed run reports and accepted-count overflow; the compact projection keeps selected-best provenance, moves an oversized profile to a detail sidecar without truncation, and reports newest-season school/grade; the search page parser refuses a row linking a different athlete, a missing athlete id, and row text grown past the bound |
| profile / bio parsers | `src/profile/parser.rs`, `src/profile/bio/events.rs` | 14 | Cohort text outside the requested athlete is ignored (including script and hidden text); a cross-athlete response is rejected; reordered canonical attributes cannot hide a foreign identity; embedded JSON escapes do not truncate identity; XC joins keep the source's own display units (`3 Miles`, `1.93 Miles`, `5000 Meters`) and never manufacture a distance from conflicting or missing metadata; equipment variants join exactly and an ambiguous implement resolves to `None`; oversized shared metadata is rejected before result fan-out |
| CLI arguments and ingress errors | `src/cli/args.rs`, `tests/ingress_failure_surface.rs` | 4 | Division flags without `--rankings` are rejected; `--rankings` alone keeps outdoor-boys defaults; explicit division selection survives parsing; and the operator CLI relays the ingress's own failure message (a fake loopback ingress answers 500 with a Restate JSON body, and `stderr` must contain `requested concurrency exceeds worker capacity`) |
| result verification | `tests/result_verify.rs`, `src/result_verify/rankings/`, `src/result_verify/rankings/verification.rs` | 21 | 14 integration cases over retained evidence: a deterministic positive verifies; forged selection and duplicate source keys are rejected; local acceptance without a review artifact is rejected; review rows with contradictory profiles are preserved without promotion; stale declared digests, rehashed canonical assessment flags, performance projections that differ from the retained results, duplicate report fields, indistinguishable local selection and profile identity swaps are all rejected; a reused query with case/stage variant is accepted; relay identity distinct from a confirmed member is accepted. Unit cases add: a frozen indoor girls collection is accepted, an outdoor scope must not accept an indoor page, a boys scope must not accept a girls page, and a results capture without its physical request is rejected |
| rankings parse / catalog / scope | `tests/rankings_parser.rs`, `tests/rankings_indoor.rs`, `tests/rankings_catalog.rs`, `tests/rankings_scope.rs` | 21 | An absent roster keeps the source row and counts missing without inventing a candidate; an unknown roster result is not a fake zero join; a negative row team id is `PageParseError::WrongRelayTeamId`; published individual ids keep the observation contribution; anonymous rows stay source rows; anonymous grade-11 rows count unresolved instead of failing. Catalog: level-scoped nav selects the indoor list (173005) or outdoor list (168416), a seasons map must point at the requested division, a missing season entry is reported with its key, nav outside the requested level is rejected. Scope: each supported division binds its own list and revision, an indoor scope cannot carry the outdoor list/revision, a girls scope cannot carry the boys revision, an unsupported gender is rejected, and a deserialized scope must revalidate |
| bounded HTML / merge properties | `tests/profile_html_bounds.rs`, `tests/profile_merge_bounds.rs`, `tests/search_html_bounds.rs` | 16 | Flat documents preserve facts without materializing nodes; an oversized unfinished attribute exhausts parser memory *explicitly* (the error text says so); deeply nested formatting tags are rejected; script rawtext is one DOM text node despite tag-like text; CDATA in foreign content stays extractable; malformed UTF-8 is rejected with `profile HTML is not UTF-8`; merges keep left-first order for distinct ids, ignore provenance for same-id equivalence while retaining conflicts, and keep duplicate evidence in stable order |
| parser properties (proptest) | ~~`tests/native_parser_properties.rs`~~ (historical: root `tests/` directory deleted 2026-09-23) + ~~`native_parser_properties.proptest-regressions`~~ (historical) | 9 | Deterministic property tests: search envelopes preserve the numeric count and next offset; profile HTML keeps only the requested athlete cohort; track numeric best flags stay opaque; XC boolean best and foreign identity are not conflated; invalid serialized performance state is rejected; time and points keep distinct numeric domains; source-row keys round trip; malformed and sparse OOXML are rejected by *both* ingestion boundaries while a synthetic workbook preserves source-row identity and fields. Config is pinned: 64 cases, ChaCha, fixed seed `0x4E41544956455052`, with past failures replayed from the regressions file |

### census-service

| Area | Where | Tests | What the tests actually assert |
|---|---|---|---|
| `net/` — robots, authorization, pacing, cache | `crates/census-crawl/src/net/` | 14 (10 + 4) | No host is authorized by default; a bare domain authorizes its subdomains (case-insensitively) but not lookalikes (`notathletic.net`, `athletic.net.evil.com`); an exact host never widens into its parent domain; the authorized-host spacing is a *floor*, not a target (the test asserts `MIN_AUTHORIZED_DELAY >= 500 ms`, so a 1 ms configured delay is still raised); robots rules honour longest match with allow-on-tie, an absent or empty robots.txt allows everything, rules outside a group do not apply, and named-agent groups (`GPTBot`) are ignored; jittered delay grows with attempt and is capped at 10 s; and `Fetcher::key_for` is pinned to exact digests because the key *is* the on-disk layout (`{key}.body` + `{key}.meta.json`). The authorization tests construct a fetcher on a throwaway cache and never issue a request |
| `store/` — Fjall round trip, streaming merge | `crates/census-store/src/` (`tests.rs` + `loom_tests.rs`), `crates/census-store/tests/bounded_memory.rs`, `crates/census-service/tests/fjall_restate_e2e.rs` | 14 (9 + 4 + 1) | Keys whose low sequence byte is zero still reopen (300 observations); consolidation merges evidence and source identities and fills an empty field; journal resume keys round trip; observations survive reopen without overwriting (one row, two evidence entries); legacy JSONL journals are imported exactly once — the e2e test proves the second `Store::open` is a no-op; oversized/empty ids are rejected before any write (stats stay 0); stats count observations per table and report every table; and the bounded-memory test streams a 3,200-observation table through `for_each_merged` and asserts that pass's resident growth stays at least half the collected rows' serialized size below the collecting scan's (each pass opens its own store, so the block cache cancels). The e2e test additionally drives `census::consolidate` → `report::build_census` (both scopes) → `bests::build` → `workbook::build` over a synthetic corpus, asserting counts at every step, one best mark per athlete resting on both performances, and that the published file is a complete zip container (`PK\x03\x04` … `PK\x05\x06`) |
| census model / domain types | `crates/census-domain/src/model.rs` + `crates/census-domain/src/jurisdiction/` (tests in `crates/census-domain/src/model_tests.rs`, `crates/census-domain/src/jurisdiction/tests.rs`) | 37 (22 + 15) |
| census reports, bests, index, supervisor, services | ~~`crates/census-service/src/report/`~~ (historical: now in `crates/census-report/src/report/`), ~~`…/bests/`~~ (historical: now in `crates/census-report/src/bests/`), ~~`…/school_index.rs`~~ (historical: now in `crates/census-domain/src/school_index.rs`) + `crates/census-service/src/bootstrap/tests.rs` (7), `crates/census-service/src/restate_services/tests.rs` (7) | 26 |
| census workbook (golden output) | ~~`crates/census-service/src/workbook/`~~ (historical: now in `crates/census-report/src/workbook/`) | 1 | The workbook carries the expected sheets (`Goal & method`, `Summary`, `By state - core`, `By state - all sources`, `Athletic.net marginal`, `Best results`, `Meets`, `Evidence mix`, `Method notes`), the best-mark reduction picks the fastest 400 m (`48.55`) and longest jump (`6.42 m`), and the sheet is read back with `calamine` to assert the header row contains `Best mark` and `Profile URL`; the `Performances_00N` sheets stream their rows through a school-name spill, and the module's own tests assert that the spilled rows equal the collected ones across range seams and partition identically |
| provider adapters | `crates/census-crawl/src/*` | 173 (18 files) | See the adapter table below |
| census identity scoring | `crates/census-reconcile/src/identity.rs`, `crates/census-reconcile/src/identity/tests.rs` (moved out of the deleted root package) | 12 |
| census orchestration / adapter dispatch | `crates/census-service/src/census/` and `crates/census-crawl/src/lib.rs` | — | No unit tests of their own; `census::consolidate` is exercised by the e2e and parity-pipeline tests, and the adapter registry by its own `registry/tests.rs` |
| golden-corpus parity harnesses | `crates/census-service/tests/parity_{wisconsin,north,mideast,national,pipeline}.rs` + `crates/census-service/tests/common/mod.rs`, goldens under `crates/census-service/tests/golden/` (112 files) | 26 | Every fixture of a covered source is parsed and compared byte-for-byte with a checked-in golden; per-source rollup digests fail when a fixture stops being walked. `GOLDEN_UPDATE=1` rewrites goldens (local seeding only; CI never sets it) |
| merge algebra + parser-roundtrip properties | `crates/census-service/tests/merge_properties.rs` (+ `merge_properties/`), `crates/census-service/tests/parser_roundtrip_properties.rs` (+ `parser_roundtrip_properties/`) | 36 attributes + 7 `proptest!` blocks | Merge laws (idempotency, union commutativity, first-writer-wins, identity preservation) and the `wiaa_results` parse seam (dispatch agreement, prefix stability, totality) |

| census route, replay and recovery | `crates/census-service/tests/{athleticnet_meet_parity,athleticnet_bio_replay,recovery,restate_kill_restart,backup_restore}.rs` | 19 | The whole-meet pull spends its two requests and stores every storable row of capture 634313 (758 published rows, 72 relay squads, 288 legs, 903 performances), spends the third only when asked, and a second run resumes from the journal and spends nothing; the per-athlete bio route serves both scopes from the seeded cache, absorbs 53 performances for one athlete, and its second run reads nothing and appends nothing; recovery covers store resume, adapter resume and the kill/restart boundary (a resumed run must not replay a durable write); backup/restore round trips a live store's rows and refuses a store still in use |

### Provider adapters (all in `crates/census-crawl/src/`)

| Adapter | Tests | Fixture-driven assertions (representative) |
|---|---:|---|
| `plain_names` (ND + NSAA) | 27 | 312 NSAA member schools from the form option list; per-block directory parse; multi-name cells split into one entity per person; AD rows map to `AthleticDirector`; office rows are never emitted but the same person's coaching row is kept; coop annotations are stripped; ND offerings and staff lines parse; each fixture reports its own fill rate; entities carry no email at all; malformed input yields no rows |
| `ohsaa` | 20 | Search dedupes duplicate rows; sports table extracts XC and T&F coaches with emails; `TBA` rows yield no coach; malformed search/sports/AD payloads yield empty results instead of panicking; AD office roles are excluded from the director; sport labels map strictly |
| `ihsa` | 21 | `parse_schools` over the `/v1/schools` capture (3 schools, ids/names/cities); staff fixtures split into people and rows (20 people / 6 office-only rows); coach titles map to sport+gender and non-coaching titles return `None`; email reveal parses only real addresses; AD-assistant and trainer roles are excluded; office roles are excluded from coaches; honorifics stripped; no cell phones or personal data in entities; malformed JSON errors, empty object parses empty lists |
| `hytek` | 13 | Trackside template parses place/grade/school/field marks; a seed column does not bleed into the school label or mark (22 placed throwers survive); uppercase pre-blocks and PDF page breaks parse; `3200 Meter Run` → `Track3200m`, `4x200 Meter Relay` → `Relay4x200`; published notation (`10.56`, `1:54.32`, `15:32.1`); header lines give name/date/timer; individual rows carry place/grade/school/mark/wind; field rows keep the imperial mark and flight; relay rows name the school and list legs with grades; team score lines are not results; a file without a meet header is skipped; plain-text reports parse identically to HTML |
| `mshsl` | 13 | Listing rows carry slug/name/city and pagination follows the real pager; `cfemail` decodes to the published addresses; a school page yields a canonical school with enrollment and identity; only ADs are emitted as admin rows; office roles never become coaches; published phones and consumer mailboxes never reach the store; team nodes filter to track/XC (39 for Wayzata); only school-domain coach levels survive; fixture AD email fill rate is 6/7; malformed payloads yield zero rows; honorifics stripped. `collect` runs end to end from a **seeded cache** with `report.requests == 0` and `from_cache == 5` |
| `wiaa` | 11 | Index parsing yields org id/level/city (6 rows); empty index fragments yield no rows; school page yields name/city/conference/AD/identity; one person with two sport-gender coach rows stays two rows; sport/role labels map strictly; office staff never become coaches or directors; a school with no coach rows yields none; malformed payloads yield zero rows; `cfemail` decoding matches published addresses; honorifics stripped; measured email fill rate asserted |
| `ks` | 11 | 5 fixture records; school canonicalisation; AD coach from record; honorific stripped; missing class/enrollment and missing website still parse; no phone/principal data in entities; empty AD name yields `None`; malformed JSON errors; empty array yields zero rows; fixture AD email fill rate |
| `wayzata` | 9 | Track schedule (13 rows) and XC schedule (10 rows) parse including month heading and provider slug; venue state only answers for unambiguous venues (two-state colleges → `None`); meet level read from the name; month → season mapping (Jan/Mar/Dec indoor); `collect` mints core meets from the provider schedule from a seeded cache and a journaled schedule is skipped on the next run |
| `athleticlive_athletes` | 8 | Grade tokens (`11`, ` 12 `, `JR`); grade interpreted against the meet school year; rows become canonical entities with Athletic.net seeds; one athlete seen at two meets stays one athlete and one team; meet targets dedupe by id and respect the state filter; implausible dates are skipped and counted; grades filter server-side; empty/malformed hits yield nothing |
| `athleticlive` | 6 | Harvest CSV rows keep only known states; quoted fields do not break column alignment (name stays intact); a missing required column is an error, not a panic; corrupt years are flagged; level inference only fires on explicit markers; tenants publishing one meet merge into one canonical meet |
| `athleticnet` | 6 | A registry line carries an id and optional state; a registry is refused rather than guessed between states; marks read as published (`1:17.80a` → 77.8 auto-timed); a no-mark row yields no performance; XC and track rows deserialize from their published shapes; the Athletic.net namespace stays outside the core scope |
| `wiaa_results` | 5 | Archive links carry year/stem/label/extension; formats decided by extension and sniffed body; current-season files found under the dated upload path; school year follows the sport boundary; meet levels from the published name |
| `milesplit` | 4 | Team index (40 teams, ids and names); roster rows with grad year and seasons (25 athletes, `roster_name` `Aguilera, Julian`); entities are canonical and source-independent; malformed HTML fails loudly |
| `coach_contacts` | 4 | Sport/gender labels parse; non-coaching roles (AD secretary, principal, superintendent) are not imported; coach rows become canonical entities with evidence; import dedupes schools and AD rows. The CSV fixture is read from a file written into a tempdir |
| `raceday` | 3 | Finish-list rows carry place/grade/school/final time for an XC meet; the team summary table never becomes athlete rows; divisions parse from both spellings and a gender-only label yields `None` |
| `compiled` | 3 | A regional export parses both blocks of a page (relay on the left, prelim heat on the right) with relay legs attached to the row above; print artifacts do not become part of the meet name; a file without a header is not claimed |
| `xc` | 3 | State-meet blocks carry place/grade/time; padded table rows parse with and without team points; AccuRace rows are read through the rule line |
| `result_file` | 0 | No tests, no `#[cfg(test)]` module in this file |

`compiled.rs` and `xc.rs` use verbatim layout excerpts embedded as `const` raw strings and parse them
through `hytek::lines_from_pdf_text`; the other adapters read checked-in fixture files (see §3).

---

## 3. Fixture policy

### Where fixtures live

| Root | Holds | Consumed by |
|---|---|---|
| `crates/census-crawl/tests/fixtures/` | 47 captured provider payloads across 11 provider directories: `mshsl/` (11), `ohsaa/` (9), `plain_names/` (6), `wiaa_results/` (6), `wiaa/` (4), `ihsa/` (3), `wayzata/` (2), `milesplit/` (2), `athleticlive/` (1 csv), `athleticlive_athletes/` (1 json), `ks/` (1 json), plus `coach_contacts_sample.csv` at the top level | `include_str!("../../tests/fixtures/<provider>/<file>")` inside the adapter's `mod tests`, or a tempdir write for the CSV |
| ~~`tests/fixtures/` (root)~~ (historical: root `tests/` directory deleted 2026-09-23) | ~~`cdp/request_will_be_sent_extra_info.json`~~, ~~`rankings/indoor-nav.json`~~, ~~`rankings/indoor-girls-100m-p1.json`~~ | ~~`tests/cdp_frame_compat.rs`~~ (historical: root `tests/` gone) and ~~`src/result_verify/rankings/`~~ (historical: root crate deleted) |
| `crates/census-service/tests/golden/` | 112 checked-in golden JSON files: `<source>__<case>.json` per parity case plus `<source>__rollup` and `<source>__corpus` digests | the parity harnesses under `crates/census-service/tests/parity_*.rs`; rewritten only by `GOLDEN_UPDATE=1`, which is a local seeding affordance and never set in CI |
| `fuzz/fixtures/retained_results_jsonl/` | 8 retained native-pipeline seeds plus `raw/<sha256>` bodies | ~~`tests/result_verify.rs`~~ (historical: root `tests/` gone) and ~~`src/result_verify/rankings/`~~ (historical: root crate deleted); the directory name is historical — there is no fuzz target crate (see §4) |
| generated at test time | xlsx workbooks, stores, caches | ~~`tempfile::tempdir()` + `zip::ZipWriter`~~ (the test names ~~`xlsx_scope_tests`~~, ~~`xlsx_stream_tests`~~, ~~`workbook_zip_layout`~~, ~~`workbook_verify`~~ belonged to the deleted root package) or ~~`rust_xlsxwriter`~~ (the test names ~~`workbook_zip_layout`~~, ~~`workbook_verify`~~, ~~`tests/bundle_verify.rs`~~ belonged to the deleted root package) |

### Determinism rules

- No checked-in binary workbook: every xlsx a test reads is either built in the test or generated by
  the fixture example; the repo has real `.xlsx` reports under `reports/` and `out-*/`, and no test
  reads them.
- Dates and observation labels are constants, not clocks: `OBSERVED_ON = "2026-09-20"` in the
  census adapters, `MEET_DATE = "2026-05-02"` in `fjall_restate_e2e.rs`, `day = "2026-09-21"` in
  the census workbook test, fixed `fetched_at` in seeded cache metadata.
- No test reads `var/` or `reports/`. Both directories exist at the repo root but only as pipeline
  runtime output (the CLI's default data directory, e.g. `var/census-service/{http,journal,entities,out}`,
  and exported reports, e.g. `reports/census-service-<date>.xlsx`); no fixture lives there and no test
  references those paths — every occurrence in code is a production default
  (`crates/census-service/src/bootstrap/options.rs:44`, `crates/census-service/src/cli/mod.rs:48`,
  `crates/census-service/src/cli/verify_coaches/mod.rs:30`,
  `crates/census-service/examples/coach_gate.rs:82`), a doc comment naming a run-evidence path
  (`crates/census-crawl/src/net/mod.rs:137`) or documentation. Test stores and caches are created
  inside a tempdir.
- Property tests are pinned: ~~`tests/native_parser_properties.rs`~~ (historical: root `tests/` directory deleted 2026-09-23) fixed `cases: 64`, ChaCha and
  `rng_seed: RngSeed::Fixed(0x4E41544956455052)`, and failed cases were replayed from
  ~~`native_parser_properties.proptest-regressions`~~ (historical) (one seed: `year = 1900, name = "a"`).
- Cache-seeded adapter tests write `{key}.body` / `{key}.meta.json` into a temp cache using the same
  key derivation as production (`sha256(method \x1f url \x1f body)[..16]`); `net/` pins that
  derivation, so a cache-format change fails a test rather than silently turning into a network call.
  The payoff is asserted, not assumed: `report.requests == 0`.

### Network-free requirement

No test may make an outbound request. Three deliberate exceptions, all loopback-only:

1. `crates/census-service/tests/fjall_restate_e2e.rs` binds an ephemeral `127.0.0.1` port for the
   Restate endpoint it drives in-process (the `/discover` route is the SDK's own).
2. ~~`tests/ingress_failure_surface.rs`~~ (historical: root `tests/` directory deleted 2026-09-23) bound a throwaway `127.0.0.1:0` listener that answered 500 so
   the CLI's error relay can be asserted.
3. The ignored `lane_smoke` tests expect a fixture origin and a CDP endpoint, both loopback by
   default; they are opt-in and never part of a gate run.

The `native_fixture` example enforces the same posture: it refuses to bind a non-loopback address or
port 0. Captured fixture bodies also carry privacy expectations, asserted per adapter:
`plain_names/` rejects any `@` in a serialized ND/NSAA entity,
`ihsa/tests.rs::no_cell_phones_or_personal_data_in_entities` and
`ks.rs::no_cell_phones_or_principal_data_in_entities` assert every coach phone is `None`, the MSHSL
deserializer never declares `field_work_phone` and drops personal-domain addresses, and the WIAA
parser reads no phone field at all.

---

## 4. What the suite does NOT cover

Stated plainly, from the tree as it stands:

- **No live-network test.** Adapters are driven either from pure parse functions over checked-in
  captures or from a seeded cache with `requests == 0`; `net/` authorization tests never fetch.
  Nothing in the suite proves a real HTTP fetch against a real host works.
- **The CDP lane is opt-in and not run by the gate.** The only tests that exercise the real browser
  scheduler and pin-and-verify transport are the two `#[ignore]`d `lane_smoke` tests, which need a
  fixture origin plus a CDP browser. Under `tools/gate.sh` that whole path is unverified.
- **No CAPTCHA / human-required end-to-end case.** Challenge *classification* and the escalation
  predicate are unit-tested; nothing drives a served challenge through the full browser recovery
  path except the ignored `challenge_response_revokes_the_gate_and_ends_pagination`.
- **Benchmarks exist as of 2026-09-21 but nothing gates them.** ~~`benches/` (root) holds `artifact_store`, `blocking_fanout` and `workbook_export`~~ (historical: root `benches/` directory deleted 2026-09-23); `crates/census-service/benches/core.rs` holds the census groups `census/parse`, `census/school_index` and `census/merge`. The gate's bench-presence lane compiles them (`cargo bench --workspace --no-run`) and passes; no lane compares their numbers to a threshold, so a regression is not caught. `crates/census-service/examples/` still holds the `bench_census` and `bench_store` harnesses.
- **Verification harnesses exist but are not run by the gate.** Kani harnesses live in
  `crates/census-domain/kani/` and `crates/census-store/kani/`, wired with
  `#[cfg(kani)] include!(...)` in each crate's `lib.rs`; the crate manifests declare the `kani` cfg
  (`Cargo.toml`, `[workspace.lints.rust] unexpected_cfgs`). Fuzz targets live in the standalone
  `fuzz/` workspace (`fuzz/fuzz_targets/{hytek,compiled,xc,raceday}.rs`, corpus under
  `fuzz/corpus/<target>/`). Loom models live in `crates/census-store/src/loom_tests.rs` and
  `crates/census-service/src/spawn/loom_tests.rs`, each behind its own crate's `loom` feature
  (`cargo test -p census-store --features loom --lib`, `cargo test -p census-service --features
  loom --lib`); a default `--all-features` gate run does test them, because `loom` is in each
crate's dev-dependency set and the feature is enabled by `--all-features`. None of the Kani or fuzz lanes is invoked by `tools/gate.sh`. ~~The root `fuzzing` feature only gates an entry point (`fuzz_parse_response` in `src/runtime/reviewer/model.rs`)~~ (historical: root package deleted; no `fuzzing` feature exists in any crate manifest) — nothing calls it.
- **CI exists.** `.github/workflows/gate.yml` runs `bash tools/gate.sh` on push to `main` and on pull
  requests, installing the pinned toolchain from `rust-toolchain.toml` and the optional gate tools
  best-effort.
- **No mutation testing lane.** `cargo-mutants` is installed on this workstation but `tools/gate.sh`
  does not invoke it.
- **Doc tests are not run by the gate** when nextest is installed — the tests lane prefers
  `cargo nextest run`, which does not run doc tests.
- **Declared but unused test tooling.** ~~The root manifest declares `mockito`, `axum` and `http` as dev-dependencies~~ (historical: root package deleted; none of these names appear in any current crate's `[dev-dependencies]`) — nothing calls them.
- **Modules with no tests of their own** (behaviour reachable only through another module's tests, a lane test, or nothing): ~~`src/workbook_export.rs`~~ (historical: root crate deleted 2026-09-23), ~~`src/bundle_verify/`~~ (historical), ~~`src/workbook_ingest.rs`~~ and its subtree (historical: root crate deleted) + ~~(`guards.rs`, `preflight.rs`, `stream.rs`, `preflight/zip.rs`, `preflight/zip/records.rs`, `preflight/zip/retired.rs`)~~ (historical), ~~`src/store/backend.rs`~~ (historical), ~~`src/runtime/browser/request.rs`~~ (historical; `crates/athleticnet-browser/src/request/` is the current equivalent), ~~`src/runtime/browser/state.rs`~~ (historical; `crates/athleticnet-browser/src/state.rs` is the current equivalent), ~~the two files under `src/runtime/run/`~~ (historical: root crate deleted), ~~`src/runtime/source/dispatch.rs`~~ (historical), ~~`src/runtime/source/admission.rs`~~ (historical), ~~`crates/census-service/src/census/` except `census/identity/tests.rs` (8 tests)~~ (`identity/` subdirectory deleted with root package 2026-09-23; `crates/census-service/src/census/` now has no tests at top level), `crates/census-crawl/src/result_file.rs`.
- **No happy-path CLI test against a real Restate ingress.** The single CLI integration test asserts
  a *failure* message relay; `fjall_restate_e2e.rs` drives `serve_until` in-process instead.
- **No fixture provenance README.** `AGENTS.md` §7 asks new adapter fixtures to ship with a short
  README naming the real URL and what the file proves. No such README exists under
  `crates/census-crawl/tests/fixtures/`; provenance currently lives in doc comments above each
  `include_str!` constant (byte ranges, capture date, HTTP status), and `plain_names/` documents
  the one edited fixture (`ND_PAGE_NO_AD`). Fixture *files* are otherwise verbatim captures.

---

## 5. Conventions for adding tests

Observed across both crates; follow the local file's idiom over anything written here.

- **Names are sentences, not method labels.** `a_bare_domain_authorizes_its_subdomains_but_not_lookalikes`,
  `robots_named_agent_groups_are_ignored`, `first_worksheet_preserves_sparse_rich_shared_and_inline_source_rows`,
  `office_rows_are_ignored_but_the_same_persons_coaching_row_is_kept`. No `test_` prefix, no `should_`.
  The name states the invariant; the case is not named after the function under test.
- **Placement.** Unit tests go in an inline `#[cfg(test)] mod tests` at the bottom of the module under
  test. When the module gets large, the suite moves to a child file: ~~`src/store/tests.rs`~~ (historical: now at `crates/census-store/src/tests.rs`), ~~`src/domain/marks/tests.rs`~~ (historical: root crate deleted), ~~`src/runtime/source/tests.rs`~~ (historical: root crate deleted), ~~`src/xlsx_scope_tests.rs`~~ (historical: root crate deleted) — the last
  two wired with `#[cfg(test)] #[path = "…"] mod tests;`. Cross-module and crate-boundary tests go
  under ~~`tests/<name>.rs` (root)~~ (historical: root `tests/` directory deleted) or `crates/census-service/tests/` (census); both are separate
  binaries, so anything they use must be exported from `lib.rs`.
- **Typed errors, not strings.** Assert error *variants* where the type exists —
  `matches!(store.get_bytes(&digest), Err(StoreError::DigestMismatch))`,
  `matches!(result, Err(PageParseError::WrongRelayTeamId))`,
  `matches!(transient, Err(JobError::Transient { .. }))`. String matching is reserved for
  operator-facing contracts where the message *is* the interface:
  `assert!(rejection(&actual, &expected, 90).contains("neither completed nor pending"))`, and the CLI
  test asserting the ingress message reaches `stderr`.
- **Assertions carry the reason.** `assert_eq!(report.requests, 0, "every response came from the
  seeded cache")`, `assert_eq!(coaches.len(), 4, "{slug}: one row per AD entry")`. Where a loop
  covers several fixtures, the message names the case.
- **Bounds are asserted as bounds.** Authorization/robots tests assert the negative case explicitly
  (`notathletic.net`, `athletic.net.evil.com`, `example.com` under an exact-host rule), not just the
  happy path.
- **Async tests** use `#[tokio::test]`; the drain test uses
  `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` because the supervisor spawns a
  runtime region.
- **Test code may panic; production may not.** Workspace lints deny `todo!`, `unimplemented!`,
  `dbg_macro` and `panic_in_result_fn` for every target, but the strict production set (no `unwrap`,
  `expect`, `panic`, indexing/slicing, `as` casts, unchecked arithmetic) is enforced by the gate on
  `--lib --bins --examples` only. Tests use `expect("reason")` and `unwrap()` freely, and some use
  slicing.
- **A new adapter needs**: an inline test module, a fixture directory (plus provenance note), pure
  parse tests, and — when the adapter has a `collect` entry point — one async test that seeds the
  cache and asserts the resulting `AdapterReport` and store contents, with `requests == 0`.

---

## 6. Gate semantics

`tools/gate.sh` runs lanes in this order, accumulates failures and prints `gate: FAIL -> <lanes>`
with exit 1, or `gate: PASS` with exit 0. `set -uo pipefail` is in force (no `set -e`), so every lane
runs even after one fails.

| Lane | Enforces | Single-lane command |
|---|---|---|
| fmt | rustfmt formatting, check-only | `cargo fmt --all -- --check` |
| check | whole workspace compiles, all targets, all features | `cargo check --workspace --all-targets --all-features` |
| doc | docs build without warnings/broken links | `cargo doc --workspace --all-features --no-deps` |
| tests | the whole suite | `cargo nextest run --workspace --all-features` (falls back to `cargo test --workspace --all-features --quiet` and prints `cargo-nextest absent: falling back to cargo test` when nextest is missing) |
| strict clippy (source targets) | `-D warnings` plus the doctrine set on production targets; counted per crate and lint code | `cargo clippy --workspace --lib --bins --examples --all-features --message-format=json -- <LINT_SET>` |
| production scan | forbidden constructs and size budgets in production-reachable lines | `cargo xtask scan` |
| domain type integrity | review candidates only (boolean signatures, primitive id parameters, structs with ≥2 `Option` fields) in `crates/census-domain/src/` (the single root in `xtask/src/integrity.rs`); **printed, not ratcheted** | `cargo xtask integrity` |
| debt ratchet | baseline comparison; fails on any increase | `cargo xtask ratchet tools/quality-baseline.json <clippy.tsv> <scan.json>` |
| domain purity | the census domain crate's *normal* dependency tree contains only `serde` + `sha2` | `cargo xtask domain-purity` |
| deny | `cargo deny check` (licenses, advisories, bans) | `cargo deny check` |
| audit | `cargo audit --quiet` | `cargo audit --quiet` |
| machete | unused dependencies | `cargo machete` |
| geiger | unsafe-surface report (JSON to /dev/null) | `cargo geiger --workspace --all-features --output-format Json > /dev/null` |
| bench presence | a benchmark target exists and compiles. `crates/census-service/benches/` exists (the root `benches/` directory went with the deleted root package; `tools/gate.sh` still tests both paths), so this lane runs `cargo bench --workspace --no-run`; the fallback message ("no benchmark target exists yet: …") only prints if both directories are gone | `cargo bench --workspace --no-run` |

`LINT_SET` (exactly as defined in `tools/gate.sh`, with `-D` for each):
`warnings`, `unsafe_code`, `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`,
`clippy::panic_in_result_fn`, `clippy::todo`, `clippy::unimplemented`, `clippy::dbg_macro`,
`clippy::indexing_slicing`, `clippy::string_slice`, `clippy::get_unwrap`,
`clippy::arithmetic_side_effects`, `clippy::as_conversions`, `clippy::let_underscore_must_use`,
`clippy::await_holding_lock`.

Mechanics worth knowing:

- Only the three tool lanes (`audit`, `machete`, `geiger`) are guarded: when the binary is absent
  they print `SKIP: <tool> is not installed (cargo install <tool>)` and count as a pass. `deny` is a
  normal lane, so a missing `cargo-deny` fails it.
- The production scan (`cargo xtask scan`) counts `unsafe`, `.unwrap()`, `.expect(`, `panic!`, `unreachable!`,
  `todo!`/`unimplemented!`, the `assert!`-family, `dbg!`, `as` casts to primitives, and real
  indexing (`expr[i]`, deliberately excluding slice/array types and attributes). It excludes files
  whose name or directory contains `tests` and cuts each file at the `#[cfg(test)]` that opens a
  *module* — a `#[cfg(test)]` gating only `use` re-exports (`crates/census-crawl/src/tfrrs/parse/mod.rs:34`) does not end the
  production region. Size budgets: 300 lines per file, 60 lines per function, 25 logical lines per
  function.
- The ratchet compares clippy counts per `(crate, lint)`, every scan counter, the two function-size
  totals, and the *set* of files over 300 lines — a new oversized file fails even when the count is
  unchanged (`new file over 300 lines: …`). Burndown prints `[DOWN]`; a non-zero delta in either
  direction is printed so progress is visible on a passing run.
- `--update-baseline` runs the same measurements, writes `tools/quality-baseline.json` and returns
  without the `deny`/`audit`/`machete`/`geiger`/bench lanes. `--allow-increase` without
  `--update-baseline` is accepted but has no effect on a normal run.
- Running a single gate means running that lane's command from the table above; the clippy lane's
  output is TSV (`crate \t lint \t count`) so `cargo xtask ratchet` and `cargo xtask quality-baseline` can
  consume it directly.
