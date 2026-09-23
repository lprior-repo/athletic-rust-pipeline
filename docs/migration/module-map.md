# Module → crate migration map

Read-only survey of `crates/census-domain`, `crates/census-service`, `crates/g1-audit` in
`/home/lewis/src/ad-law-scrape/athletic-rust-pipeline`, mapped onto the crate layout of
`ARCHITECTURE.md` §4 (`census-domain`, `census-store`, `census-crawl`, `census-reconcile`,
`census-review`, `census-report`, `census-service`), the keyspaces of `ARCHITECTURE.md` §7 and the
workflow identities of `ARCHITECTURE.md` §8. The order (seal, then extract) is frozen by
`docs/adr/README.md` ADR-007.

**Basis of measurement.** HEAD `ea81c56` plus the working tree as read during this pass; `git status
--porcelain` listed 11 modified files while the pass ran (`crates/census-domain/src/jurisdiction.rs`,
`crates/census-service/src/{census/mod.rs,cli/gather.rs,cli/mod.rs,cli/national/run.rs,report/coverage.rs,report/coverage/classify.rs,report/coverage/tests.rs,report/projection.rs,restate_services/national.rs,restate_services/tests.rs}`), so line counts are a snapshot, not a constant. The tree kept moving during the pass: the same
command at the end listed 15 modified files (adding `crates/census-service/src/bootstrap.rs`,
`bootstrap/serve.rs`, `bootstrap/stop.rs`, `store/legacy.rs`) and one untracked file not covered by
§1 (`crates/census-service/src/bootstrap/guard.rs`). Method: `read`,
`find`, `wc -l`, `grep`, one Python pass over the same files (module-edge resolution). No build, no
`cargo` command was run — `cargo xtask seams`, `seams.rs`'s `ALLOWED` table, is quoted from source,
not executed. Totals below are exact for the three crates' `src` trees at this revision:
`wc -l $(find crates/{census-domain,census-service,g1-audit}/src -name '*.rs')` = **73,209** lines
(census-service 68,252 · census-domain 3,574 · g1-audit 1,383).

**Anchor re-verification (2026-09-23).** The anchors in §5, §5.1 and §5.2 below were re-read against
the working tree at HEAD `95e89e9` — three commits after `ea81c56`, with uncommitted changes present —
and corrected where they had drifted; the numbers here are that snapshot (`census/identity.rs` 265
lines, `restate_services/{jurisdiction.rs 243, national.rs 292, census.rs 156, ingest.rs 147,
sweep.rs 187}`). A file that moves invalidates them again.

Terminology used throughout: **production** = a `.rs` file that is not `tests.rs`, is not under a
`tests/` directory, and — for mixed files — only the lines before the first `#[cfg(test)]`, which is
exactly `xtask/src/seams.rs`'s scope. **Test** = everything else.

---

## 1. Inventory

### 1.1 `census-domain` (10 files, 3,574 lines)

Dependencies (`crates/census-domain/Cargo.toml:12-14`): `serde`, `sha2`, `thiserror`. No tokio,
fjall, reqwest, chromiumoxide, writer, Restate or HTTP client — enforced by `xtask/src/purity.rs`
(`BANNED`, 24 packages, `purity.rs:27-52`), which reads
`cargo tree -p census-domain --edges normal` (`purity.rs:16-24`).

| file | lines | public types | public fns | external crates |
|---|---|---|---|---|
| `crates/census-domain/src/lib.rs` | 19 | — | — | — (re-exports; `#[cfg(kani)] include!("../kani/census_domain_wiring.rs")` at `lib.rs:19`) |
| `crates/census-domain/src/error.rs` | 37 | `DomainError` | 0 | `thiserror` (`error.rs:8`) |
| `crates/census-domain/src/jurisdiction.rs` | 612 | `UsJurisdiction`, `OutsideCensusScope`, `JurisdictionBucket`, `MeetState` | 11 (`code`, `name`, `from_code`, `parse`, `require_census_scope`, `is_in_census_scope`, …) | `serde` (`jurisdiction.rs:19`), `thiserror` |
| `crates/census-domain/src/model.rs` | 1,281 | `SchoolYear`, `ObservedGrade` (`model.rs:210`), `CanonicalSchool`, `CanonicalTeam`, `CanonicalCoach`, `CanonicalAthlete`, `CanonicalMeet`, `CanonicalEvent`, `CanonicalPerformance`, `SourceEntityKind`, `ReviewVerdict*`, … (50 pub types in the `model` tree) | 62 | `serde` (`model.rs:12`), `sha2` (`model.rs:16`) |
| `crates/census-domain/src/model/records.rs` | 418 | `SourceObjectIdentity`, `RetainedConflict`, `ReviewCase`, `ReviewState`, `CoverageScope` (`records.rs:187`), `CoverageRow`, `CollectionSnapshot`, `AccessBlockKind`, `SourceAccessCondition`, `SourceMeetRef` | 17 | `serde` (`model/records.rs:9`) |
| `crates/census-domain/src/model/review.rs` | 233 | `ReviewEvidenceFact`, `ReviewCaseFact`, `ReviewPacket`, `ReviewVerdictKind`, `ReviewVerdict`, `VerdictBatch`, `ReviewVerdictRecord` | 9 | `serde` |
| `crates/census-domain/src/model_tests.rs` | 423 | test-only (`#[cfg(test)] #[path = "model_tests.rs"] mod tests;` at `model.rs:1281`) | — | — |
| `crates/census-domain/src/model/records_tests.rs` | 170 | test-only (`model/records.rs:375`) | — | — |
| `crates/census-domain/src/model/review_tests.rs` | 144 | test-only (`model/review.rs:233`) | — | — |
| `crates/census-domain/src/jurisdiction_tests.rs` | 237 | test-only (`jurisdiction.rs:612`) | — | — |

Plus `crates/census-domain/kani/` (4 files: `census_domain_wiring.rs`, `gradyear.rs`, `id_mint.rs`,
`publish.rs`) reachable only through the `include!` at `lib.rs:19`.

### 1.2 `g1-audit` (1 file, 1,383 lines)

| file | lines | public items | external crates |
|---|---|---|---|
| `crates/g1-audit/src/main.rs` | 1,383 | none (`pub` items: 0 — binary crate) | `clap` (`main.rs:10`), `indexmap` (`:11`), `regex` (`:12`), `serde_json` (`:13`), `sha2` (`:14`) |

No tokio, fjall, reqwest, chromiumoxide, workbook writer, Restate or HTTP client: it reads
`raw/`, `evidence/`, `parsed/` directories (`main.rs:20-30`). No tests, no benches in this crate.

### 1.3 `census-service` top-level modules (387 files, 68,252 lines: 49,720 production / 18,532 test)

`crates/census-service/src/lib.rs:33-50` declares all 17 modules and re-exports
`census::{collect_milesplit, consolidate, CollectOptions, CollectReport}`, `net::{FetchOptions,
Fetcher}`, `store::{Store, Table}`. `prod/test` is a snapshot split: production = lines before the
first `#[cfg(test)]`; test = whole test files plus those tails.

| module | files | prod lines | test lines | public types | pub fns | external crates (first import site) |
|---|---|---|---|---|---|---|
| `sources/` | 190 | 26,371 | 9,589 | 132 (`ParsedMeet`, `CrawlResult`, `DocumentEntities`, `SourceDescriptor`, `AdapterContext`, `MeetRow`, …) | 223 | `serde_json` (`sources/athleticlive/docs/events.rs:8`), `serde`, `regex` (`sources/compiled/events.rs:4`), `futures` (`sources/wiaa/collect.rs:6`); reaches `net` and `store` |
| `cli/` | 54 | 6,487 | 1,565 | 1 (`VerifyArgs`) | 19 | `calamine` (`cli/seal/workbook.rs:13`), `restate_sdk` (`cli/ingress.rs:13`), `regex` (`cli/merge_coaches/mod.rs:8`), `url` (`cli/ingress.rs:15`), `csv` (`cli/export_data/coaches.rs:3`), `futures` (`cli/verify_coaches/mod.rs:12`), `clap` (`cli/census_doc/mod.rs:23`), `serde_json`, `sha2`, `anyhow` |
| `workbook/` | 24 | 3,891 | 1,663 | 1 (`Options`) | 1 | `rust_xlsxwriter` (`workbook/cells.rs:7`), `serde`; reaches `store`, `report`, `bests`, `sources` |
| `store/` | 19 | 2,046 | 869 | 10 (`Table`, `Entity`, `Store`, `StoreError`, `StoreStats`, `Consolidated`, `BackupReport`, `RestoreReport`, `IntegrityReport`, `IntegrityTable`) | 23 | `fjall` (`store/legacy.rs:4`), `serde`, `serde_json` (`store/keys.rs:7`), `sha2` (`store/backup/helpers.rs:9`), `anyhow` (`store/backup/copy.rs:6`), `loom` (dev-gated, `store/loom_tests.rs:13`) |
| `census/` | 21 | 2,003 | 989 | 16 (`Revision`, `WorkflowIdentity`, `MeetCensus`, `CollectOptions`, `CollectReport`, `AcceptanceItem`, `SealCounts`, `OpenWork`, `SealEvidence`, `SealedCensus`, …) | 21 | `tokio` (`census/sweep/units.rs:12`), `futures` (`census/sweep.rs:21`), `sha2` (`census/identity.rs:24`), `serde`; reaches `sources`, `store`, `net` |
| `restate_services/` | 11 | 1,973 | 259 | 32 (`Census`, `Ingest`, `Sweep`, `JurisdictionCensus`, `NationalCensus`, `JobError`, wire types) | 8 | `restate_sdk` (`restate_services/census.rs:3`), `tokio` (`:4`), `serde_json` (`restate_services/jobs.rs:6`), `serde` (`restate_services/wire/ingest.rs:7`) |
| `report/` | 14 | 1,512 | 1,081 | 5 (`Scope`, `CoreScoped`, `GapClass`, `CoverageGap`, `ReportError`) | 7 | `serde` (`report/coverage/gaps.rs:10`); reads `store`, `clock` |
| `net/` | 11 | 1,425 | 757 | 4 (`FetchError`, `FetchOptions`, `FetchOutcome`, `FetchStats`) | 7 | `reqwest` (`net/decode.rs:15`), `tokio` (`net/client.rs:8`), `thiserror` (`net/types.rs:11`), `serde` (`net/cache.rs:4`), `sha2` (`net/cache.rs:5`), `regex` (`net/robots.rs:4`) |
| `coachverify/` | 6 | 1,057 | 287 | 7 (`GateOptions`, `FragmentRow`, `Verdict`, `RowEvidence`, `RowOutcome`, `FragmentOutcome`, `Reconcile`) | 26 | `anyhow` (`coachverify/output.rs:9`), `regex` (`coachverify/mod.rs:47`), `sha2` (`coachverify/report.rs:176`); reaches `net` |
| `identity/` | 11 | 1,036 | 392 | 8 (`ModelClient`, `ModelOptions`, `ModelError`, `ReviewFamily`, `ReviewOptions`, `ReviewReport`, `Admitted`, `Refusal`) | 15 | `serde_json` (`identity/model/mod.rs:22`), `futures` (`identity/mod.rs:30`), `reqwest` (`identity/model/mod.rs:87` — OpenAI-compatible local server, `ModelOptions.endpoint` "e.g. `http://127.0.0.1:52080`", `identity/model/mod.rs:32`) |
| `bootstrap/` | 7 | 528 | 195 | 3 (`BootstrapError`, `ServeOptions`, `StopReason`) | 2 | `restate_sdk` (`bootstrap/serve.rs:9`), `tokio` (`bootstrap/drain.rs:20`), `anyhow` (`bootstrap/serve.rs:8`) |
| `bests/` | 7 | 417 | 133 | 1 (`Measure`) | 10 | `serde` (`bests/mod.rs:19`) |
| `spawn.rs` + `spawn/` | 4 | 339 | 293 | 3 (`Spawner`, `SpawnError`, `TaskReport`) | 3 | `tokio` (`spawn.rs:25`), `loom` (dev-gated, `spawn/loom_tests.rs:12`) |
| `index.rs` | 2 | 250 | 242 | 1 (`IndexReport`) | 2 | none directly; writes through `store::Table` |
| `school_index.rs` | 1 | 177 | 118 | 2 (`SchoolIndex`, `SchoolMatch`) | 4 | none — `std` + `census_domain` only |
| `outcome.rs` | 1 | 63 | 70 | 2 (`Outcome`, `DrainState`) | 2 | `tokio` (`outcome.rs:7`) |
| `clock.rs` | 1 | 34 | 30 | 2 (`Clock`, `SystemClock`) | 0 | `tokio` (`clock.rs:15`, `:30-31`), `chrono` (`clock.rs:23`) |
| `lib.rs` | 1 | 50 | 0 | 0 | 0 | — |
| `main.rs` | 1 | 18 | 0 | 0 | 0 | `anyhow` (`main.rs:13`); `cli::run()` |
| `bin/census-serve.rs` | 1 | 43 | 0 | 0 | 0 | `census_service::bootstrap::{serve, ServeOptions}` |

`clock.rs` is the **only** module in the three crates whose *proposed* home is `census-domain`
while carrying a banned dependency: `crates/census-service/src/clock.rs:15` declares
`fn now(&self) -> tokio::time::Instant` (impl at `clock.rs:30-31`), and `purity.rs:27-52` bans
`tokio` in the domain tree.
`chromiumoxide` belongs to the root package, not to these three crates: `Cargo.toml:35` declares it
and `Cargo.toml:122` patches `chromiumoxide_cdp` onto `vendor/chromiumoxide_cdp`; no file under
`crates/census-service/src` or `crates/census-domain/src` mentions it. The browser supervisor that
`ARCHITECTURE.md` §4 assigns to `census-crawl` therefore lives outside the three crates today.

### 1.4 `sources/` by family (190 files, 26,371 production lines, 469 public declarations)

Each family has an entry point `sources/<family>.rs` (docs + `mod` declarations + re-exports) and a
`sources/<family>/` directory. Families that write canonical entities are marked ●, with the exact
call site; the shared parse kernel `result_file.rs` is imported by `compiled` and `xc`.

| family | files | prod lines | test lines | pub decls | writes canonical tables (production) |
|---|---|---|---|---|---|
| `ihsa` (IHSA directory + tournament) | 20 | 2,867 | 1,505 | 53 | ● `ihsa/collect.rs:124,136`, `ihsa/tournament/map.rs:146-151` |
| `athleticnet` | 17 | 3,116 | 739 | 31 | ● `athleticnet/collect.rs:147-152` |
| `athleticlive` (+ `athleticlive_athletes`) | 19+7 | 2,667+939 | 1,006+274 | 63+24 | ● `athleticlive.rs:120`, `athleticlive/results.rs:129-134`, `athleticlive_athletes/batches.rs:159-161` |
| `milesplit` | 15 | 2,503 | 540 | 38 | ● `milesplit/results.rs:184-188` |
| `tfrrs` | 20 | 2,382 | 373 | 33 | ● `tfrrs/run.rs:175-180` |
| `plain_names` (NSAA/ND/MHSAA-style name+school lists) | 10 | 1,644 | 1,283 | 29 | ● `plain_names/nsaa_walk.rs:177-178`, `nd_walk.rs:178-179` |
| `wiaa_results` | 9 | 1,341 | 118 | 14 | ● `wiaa_results/run.rs:190-194` |
| `wiaa` | 7 | 1,298 | 478 | 23 | ● `wiaa/collect_schools.rs:140-141` |
| `mshsl` | 8 | 1,211 | 801 | 40 | ● `mshsl/collect/run.rs:159-160` |
| `ohsaa` | 6 | 1,119 | 370 | 28 | ● `ohsaa/collect.rs:245-249` |
| `hytek` (shared Hy-Tek kernel) | 8 | 1,069 | 423 | 11 | — (kernel; callers write) |
| `wayzata` | 5 | 741 | 435 | 12 | ● `wayzata/walk.rs:220` |
| `compiled` (WIAA "Compiled" timer export) | 7 | 655 | 130 | 3 | — (parses; caller writes) |
| `coach_contacts` | 6 | 615 | 176 | 10 | ● `coach_contacts/import.rs:134-135` |
| `xc` (WIAA cross-country) | 7 | 589 | 287 | 3 | — (parses; caller writes) |
| `registry` (`SourceDescriptor`, admission, transport) | 6 | 580 | 276 | 8 | — (pure data + policy) |
| `raceday` | 6 | 408 | 177 | 2 | — (parses; caller writes) |
| `ks` (KSHSAA directory) | 5 | 371 | 198 | 8 | ● `ks/collect.rs:164-166` |
| `mod.rs` (`AdapterContext`, `CrawlResult`, adapter trait) | 1 | 194 | 0 | 32 | carries `&Store` (`sources/mod.rs:100-106`) |
| `result_file.rs` (`ParsedMeet`/`ParsedEvent`/`ParsedRow`/`RelayLeg`) | 1 | 62 | 0 | 4 | — (shared row vocabulary) |

### 1.5 `cli/` by subcommand (54 files, 6,487 production / 1,565 test lines)

`Command` is defined in `crates/census-service/src/cli/command.rs` and dispatched in
`crates/census-service/src/cli/dispatch.rs`.

| subcommand(s) | module | files | lines (prod/test) | notable external crates |
|---|---|---|---|---|
| `Serve` | `cli/serve.rs` | 1 | 25 / 0 | restate (via `bootstrap`) |
| `Fetch` `Sites` `Teams` `Meets` `Collect` `ImportCoaches` | `cli/gather.rs` | 1 | 185 / 0 | — (adapters) |
| `Provider` | `cli/provider.rs` + `cli/provider/` | 4 | 517 / 0 | — |
| `Consolidate` `Index` `Report` `Bests` `Workbook` | `cli/publish.rs` | 1 | 137 / 0 | reaches `census`, `report`, `bests`, `workbook` (`cli/publish.rs:7`) |
| `Review` (local-model lane) | `cli/review.rs` | 1 | 118 / 0 | llama.cpp-style HTTP via `identity` |
| `Seal` | `cli/seal.rs` + `cli/seal/{workbook.rs,tests.rs}` | 3 | 439 / 213 | `calamine` (`cli/seal/workbook.rs:13`), `sha2` |
| `Verify` | `cli/verify.rs` | 1 | 206 / 0 | — |
| `Run` (full cycle) | `cli/cycle.rs` | 1 | 195 / 0 | `tokio` |
| `FjallStats` `ImportLegacy` `StoreBackup` `StoreRestore` `StoreIntegrity` | `cli/store.rs` | 1 | 120 / 0 | — |
| `National` `Jurisdiction` `NationalReport` | `cli/national/` | 5 | 464 / 265 | `restate_sdk` (`cli/ingress.rs:13`), `url` |
| `ExportData` | `cli/export_data/` | 8 | 1,158 / 0 | `csv` (`cli/export_data/coaches.rs:3`) |
| `QaReports` | `cli/qa_reports/` | 3 | 258 / 49 | — |
| `MergeCoaches` | `cli/merge_coaches/` | 5 | 690 / 602 | `regex` (`cli/merge_coaches/mod.rs:8`) |
| `SchoolNames` | `cli/school_names.rs` | 1 | 85 / 0 | — |
| `CensusDoc` | `cli/census_doc/` | 10 | 1,073 / 0 | `clap` (`cli/census_doc/mod.rs:23`), `serde_json`, `sha2` |
| `VerifyCoaches` | `cli/verify_coaches/` | 2 | 351 / 0 | `futures` (`cli/verify_coaches/mod.rs:12`) |
| envelope | `cli/mod.rs`, `cli/command.rs`, `cli/dispatch.rs`, `cli/ingress.rs`, `cli/tests.rs`, `cli/review_tests.rs` | 6 | 466 / 436 | `clap`, `restate_sdk` (`cli/ingress.rs:13`) |

Dispatch mapping was read from `crates/census-service/src/cli/dispatch.rs:24-75` (e.g.
`Command::Sites => gather::run_sites()` at `:26`,
`Command::NationalReport => national::run_national_report(args)` at `:67`); `VerifyCoaches` is
rejected in `dispatch` (`cli/dispatch.rs:73-74`) because `cli::run` handles it before dispatch
(`cli/mod.rs:94`).

---

## 2. Proposed target crate per module

Sizes are from §1. "Purity" flags a module that would violate `ARCHITECTURE.md` §4 / `xtask/src/purity.rs`
in its proposed home; the required edit is named.

| today | target crate | justification | conf. | purity flag |
|---|---|---|---|---|
| `census-domain` (all 10 files) | **census-domain** | Already the pure core; deps are `serde`/`sha2`/`thiserror` only (`crates/census-domain/Cargo.toml:11-14`). | high | clean |
| `school_index.rs` (177) | **census-domain** | Pure name→school matcher (`std` + `census_domain` only); leaf, consumed by 22 crawl files. | high | clean |
| `census/identity.rs` (187) | **census-domain** | Pure identity alphabet (`sha2` + `census_domain`, `census/identity.rs:24`); shared by the fan-out (service) and, per ADR-004, by per-meet/per-athlete issuance in crawl. Alternative home `census-service` if only the fan-out mints ids. | medium | clean |
| `clock.rs` (34) | **census-domain** | Port required by 6 modules (`store`, `net`, `report`, `census`, `spawn`, `bootstrap`); a capability belongs with the core. | high | **FAIL today**: `clock.rs:15` (`fn now(&self) -> tokio::time::Instant`), impl at `clock.rs:30-31`; `purity.rs:27-52` bans `tokio` in the domain tree. Required edit: `now()` returns `std::time::Instant` (3 production call sites: `net/execute.rs:111`, `census/sweep.rs:238`, `spawn.rs:182`) and `census-domain` gains `chrono` (`SystemClock::today/today_iso8601`, `clock.rs:22-28`) — `chrono` is not in the banned list |
| `store/` (2,046 prod) | **census-store** | Fjall engine, key encodings, observation merge, backup/restore/integrity. | high | n/a (fjall) |
| `net/` (1,425 prod) | **census-crawl** | Polite, cache-first `Fetcher` is the crawl's outbound capability (`reqwest` `net/decode.rs:15`, `tokio` `net/client.rs:8`). | high | n/a (reqwest/tokio) |
| `sources/` (26,371 prod) | **census-crawl** | All provider adapters, parse kernels, registry/admission; the crawl's substance. | high | n/a (serde_json, reqwest via `net`) |
| `coachverify/` (1,057 prod) | **census-crawl** | Re-derives coach rows from cited pages through the same `Fetcher` (`coachverify/fetch.rs:1`); no store edge. | medium | n/a |
| `census/{sweep.rs,sweep/,meets.rs,meets/,scope.rs}` (861 prod) | **census-crawl** | ADR-004 meet-first fan-out, roster walks, access/block policy; writes canonical rows today (`census/sweep/roster.rs:31-36`, `census/meets.rs:116`). | medium | n/a (tokio `census/sweep/units.rs:12`, futures `census/sweep.rs:21`) |
| `index.rs` (250 prod) | **census-reconcile** | The derive step: `SourceIdentities` → `Conflicts` → `ReviewCases` → `Coverage` → `Snapshots` (`index.rs:77-85`). Alternative: fold into `census-store`, which already owns `Store::consolidate_table` (`store/read/mod.rs:108`). | medium | n/a (writes via `Table`) |
| `identity/` (1,036 prod) | **census-review** | ADR-005's review lane: local OpenAI-compatible client (`identity/model/mod.rs:87`), packets, verdicts; writes `ReviewCases`/`IdentityVerdicts` (`identity/mod.rs:148-149`). | high | n/a (reqwest, serde_json) |
| `report/` (1,512 prod) | **census-report** | Measured census output (`report.json`, per-state CSV) and coverage classification. | high | not domain-legal: reads `store` |
| `workbook/` (3,891 prod) | **census-report** | The spreadsheet writer (`rust_xlsxwriter`, `workbook/cells.rs:7`); same read model as `report`. | high | n/a (writer) |
| `bests/` (417 prod) | **census-report** | Per-athlete best-mark reduction over the consolidated performances; consumed by `workbook` (`workbook/inventory.rs:7`). | high | not domain-legal: reads `store` |
| `census/{aggregate.rs,verify.rs,verify/,state.rs,state/}` (830 prod) | **census-report** | Acceptance artifacts: `CollectReport`, `SealEvidence`/`SealCounts`, workbook-row verification; they read the store and the workbook. Alternative home `census-service` if the seal is owned by the root run. | medium | not domain-legal: reads `store` |
| `outcome.rs` (63 prod) | **census-service** | `Outcome`/`DrainState` completion lattice for the region's tasks. | high | n/a (tokio) |
| `spawn.rs` + `spawn/` (339 prod) | **census-service** | The region's spawner (`spawn.rs:25`); edges are `bootstrap`, `restate_services` only. | high | n/a (tokio) |
| `bootstrap/` (528 prod) | **census-service** | Supervisor: cancel/drain/finalize, Restate endpoint binding. | high | n/a (restate-sdk, tokio) |
| `restate_services/` (1,973 prod) | **census-service** | The workflow/object layer and its wire types. | high | n/a (restate-sdk) |
| `cli/` (6,487 prod) | **census-service** | Composition root: clap surface, dispatch, every subcommand body; drives all six other crates. | high | n/a (calamine, restate-sdk) |
| `lib.rs`, `main.rs`, `bin/census-serve.rs` | **census-service** (bins) | Umbrella re-exports die in the cutover (`lib.rs:48-50`); binaries are re-pointed per ADR-007. | high | n/a |
| `g1-audit/src/main.rs` (1,383) | **no target crate** | Not one of the seven; it is a directory-reading audit gate over retained evidence, not a pipeline stage. Keep as its own tool binary (or fold into `xtask`). | medium | n/a |

Direction rule the table obeys: `census-domain` ← everything; every other crate may depend on
`census-domain`; `census-crawl` must not write canonical entities (`AGENTS.md`), which is the one
rule §3 shows today's code breaking.

---

## 3. Coupling risks

### 3.1 Complete measured cross-module edge list (production scope)

37 edges, resolved the way `xtask/src/seams.rs` resolves them (each `crate::<top>` in a production
line, comments and `#[cfg(test)]` tails excluded). `→` crosses a proposed crate boundary unless both
sides are in the same target crate.

| edge | refs / files | example site | crosses to | verdict |
|---|---|---|---|---|
| `sources → store` | 27 / 21 | `sources/athleticlive.rs:120` | crawl → store | **violation**: adapters write canonical rows; `AdapterContext.store` (`sources/mod.rs:100-106`); `seams.rs`'s own doc comment calls it a direction violation |
| `sources → school_index` | 22 / 22 | `sources/athleticlive/map.rs:27` | crawl → domain | fine once `school_index` is in `census-domain` |
| `sources → net` | 17 / 16 | `sources/athleticlive_athletes/batches.rs:47` | crawl internal | fine |
| `workbook → report` | 17 / 17 | `workbook/cells.rs:6` | report internal | fine |
| `census → sources` | 12 / 8 | `census/aggregate.rs:7` | depends where `census/*` lands | **risk R4**: `census/meets.rs:25` imports `sources::milesplit::{MeetRef, Season, Site}` (a provider family) |
| `census → store` | 9 / 9 | `census/aggregate.rs:8` | crawl/report → store | fine downstream; split module straddles two crates (R13) |
| `workbook → store` | 9 / 9 | `workbook/meta.rs:23` | report → store | fine; `Table` becomes census-store's public vocabulary (R8) |
| `census → net` | 8 / 7 | `census/meets.rs:25` | crawl internal | fine |
| `restate_services → clock` | 7 / 6 | `restate_services/census.rs:6` | service → domain | fine after the `clock` move |
| `workbook → bests` | 7 / 7 | `workbook/inventory.rs:7` | report internal | fine |
| `restate_services → spawn` | 6 / 6 | `restate_services/census.rs:7` | service internal | fine |
| `restate_services → store` | 6 / 6 | `restate_services/census.rs:8` | service → store | fine |
| `restate_services → census` | 6 / 6 | `restate_services/jobs.rs:8` | service → crawl+report | fine; the only split-crate edge set |
| `report → store` | 5 / 5 | `report/coverage.rs:28` | report → store | fine (R8) |
| `coachverify → net` | 3 / 3 | `coachverify/fetch.rs:1` | crawl internal | fine |
| `identity → store` | 3 / 3 | `identity/mod.rs:33` | review → store | fine; **R2** orphan rule on the `Entity` impl |
| `restate_services → net` | 3 / 3 | `restate_services/jobs.rs:9` | service → crawl | fine |
| `restate_services → report` | 3 / 3 | `restate_services/jobs.rs:10` | service → report | fine |
| `bests → report` | 2 / 2 | `bests/mod.rs:17` | report internal | fine |
| `bests → store` | 2 / 2 | `bests/reduce.rs:5` | report → store | fine |
| `bootstrap → spawn` | 2 / 2 | `bootstrap/drain.rs:8` | service internal | fine |
| `bootstrap → outcome` | 2 / 2 | `bootstrap/error.rs:10` | service internal | fine |
| `bootstrap → store` | 2 / 2 | `bootstrap/error.rs:11` | service → store | fine |
| `net → clock` | 2 / 2 | `net/execute.rs:7` | crawl → domain | fine after the `clock` move; **R5** capability not injected |
| `restate_services → sources` | 2 / 2 | `restate_services/jobs.rs:11` | service → crawl | fine |
| `spawn → outcome` | 2 / 2 | `spawn.rs:29` | service internal | fine |
| `bootstrap → restate_services` | 1 / 1 | `bootstrap/serve.rs:12` | service internal | fine (R10) |
| `bootstrap → clock` | 1 / 1 | `bootstrap.rs:47` | service → domain | fine after the move |
| `census → clock` | 1 / 1 | `census/sweep.rs:14` | crawl → domain | fine |
| `index → report` | 1 / 1 | `index.rs:26` | reconcile → report | **risk**: if `index` is reconcile and `report` does not depend on reconcile, the edge is one-way and legal — but it makes reconcile a client of report |
| `index → store` | 1 / 1 | `index.rs:27` | reconcile → store | fine |
| `index → workbook` | 1 / 1 | `index.rs:57` | reconcile → report | see above |
| `report → clock` | 1 / 1 | `report/projection.rs:11` | report → domain | fine |
| `restate_services → outcome` | 1 / 1 | `restate_services/mod.rs:29` | service internal | fine |
| `spawn → clock` | 1 / 1 | `spawn.rs:28` | service → domain | fine; `spawn.rs:182` reads `SystemClock.now()` for the drain deadline |
| `store → clock` | 1 / 1 | `store/write.rs:8` | store → domain | fine |
| `workbook → sources` | 1 / 1 | `workbook/meta/sources.rs:12` | report → crawl | **risk R3** (see register) |

Edges the survey expected but did **not** find: `restate_services → bests` / `→ workbook` (0 refs —
the workbook and best results are rendered by `cli/publish.rs:7`, which imports
`census_service::{bests, census, report, workbook}`); `spawn → store` (0); `bootstrap → net` (0).

### 3.2 Risk register

| # | risk | evidence | required decision |
|---|---|---|---|
| R1 | Crawl writes canonical entities | `AdapterContext.store` (`sources/mod.rs:100-106`); 67 production write sites across 22 files; e.g. `sources/athleticnet/collect.rs:147-152`, `sources/tfrrs/run.rs:175-180`, `census/sweep/roster.rs:31-36` | adapters must return entity batches and a reconcile/write owner must append them; `seams.rs` already names deleting the row as the ratchet |
| R2 | `Entity` impl lives with its type | `identity/records.rs:17 impl Entity for ReviewVerdictRecord` (the only impl outside `store/entities/`; the other 14 are `store/entities/canonical.rs:12-164`, `store/entities/derived.rs:15-99`) | keep it in `census-review` (local type + foreign trait is legal); moving it into `census-store` would be an orphan-rule error |
| R3 | Report reads the crawl's registry | `workbook/meta/sources.rs:12` uses `descriptors` (`sources/registry.rs:37`) | keep the pre-authorized `(workbook, sources)` edge (registry is pure data), or lift the descriptor table into `census-domain` |
| R4 | The fan-out reaches into one provider family | `census/meets.rs:25` `use crate::sources::milesplit::{MeetRef, Season, Site}`; `census/sweep/roster.rs` walks MileSplit rosters | resolved by ADR-004 (the fan-out *is* crawl), not by a new seam |
| R5 | `Fetcher` uses the ambient clock, not the capability | `net/types.rs:174-181` `now_iso8601()/today_iso()` call `SystemClock` directly; `net/execute.rs:111`, `report/projection.rs:11`, `net/client.rs:15` documents that `tokio::time::pause` drives the clock | make `Clock` a constructor parameter of `Fetcher`; then `FetchOutcome.fetched_at` (`net/types.rs:130`) is deterministic under test |
| R6 | Re-export shims mask the owning crate | `lib.rs:48-50`; family re-export blocks `sources/athleticlive.rs:36-38`, `sources/coach_contacts.rs:22-25`, `sources/ks.rs:23-25`, `sources/milesplit.rs:38-47`, `sources/raceday.rs:22`, `sources/compiled.rs:29`, `sources/xc.rs:31` | ADR-007 clean cutover: re-point every consumer at the new crate path and delete the shim, do not carry aliases — **resolved** in the crawl extraction (`docs/adr/README.md` ADR-007) |
| R7 | Table names are wire vocabulary | `restate_services/wire/ingest.rs:28` `IngestRequest.table: String`; `Table::from_wire` rejects unknown names (`store/table.rs:93-95`); names ride in observation keys (`store/keys.rs:28-35`) | any §7 rename (`snapshots` → `collection_snapshots`, `coverage` → `source_coverage`/`jurisdiction_coverage`, `source_identities` → `canonical_links`) is an API break for ingest clients and on-disk keys — one atomic migration step |
| R8 | `Table`/`Entity`/`scan` become a two-crate contract | 44 production files import `store::Table` (21 of them under `sources/`); readers: `workbook/recruiting/dataset.rs:6`, `report/coverage/classify.rs:6`, `workbook/performances/rows.rs:5`, `index.rs:5`, `workbook/meta.rs:4`, `report/projection.rs:4`, `identity/packets.rs:3` | freeze the store API before the split, or the split churns two crates at once |
| R9 | The report family is a three-way lattice | `bests → report` (`bests/mod.rs:17`), `workbook → bests` (`workbook/inventory.rs:7`), `workbook → report` (17 refs), `index → workbook` (`index.rs:57`) | all four must land in the same crate (`census-report`) or `index` must be re-homed with them |
| R10 | Supervisor must stay above the workflow layer | `bootstrap/serve.rs:12` imports `restate_services`; no reverse edge exists | keep the single direction; `bootstrap` is the top of `census-service`, never a library the workflow layer imports |
| R11 | A `clock` home in `census-service` would create a crate cycle | `store → clock` (`store/write.rs:8`), `report → clock` (`report/projection.rs:11`) while `census-service` depends on `census-store` and `census-report` | the trait+impl must live in `census-domain` (after the `Instant` change) — otherwise cargo rejects the graph |
| R12 | The seal reads the artifact the workbook wrote | `cli/seal/workbook.rs:13` (`calamine` reader) vs `workbook/cells.rs:7` (`rust_xlsxwriter` writer) | keep reader and writer in `census-report` so the round trip is one crate's test |
| R13 | One module, two target crates | `census/` splits crawl-side (`sweep*`, `meets*`, `scope`) from report-side (`aggregate`, `verify*`, `state*`) while `census/mod.rs:1-129` re-exports both | split `census/mod.rs` first; nothing else in the crate splits |
| R14 | `restate_services → census` keeps the whole split visible | 6 refs / 6 files (`restate_services/jobs.rs:8`, …) | acceptable for a composition root, but it is why `census` cannot be deleted without re-pointing the service layer |

---

## 4. Fjall surface today

### 4.1 Physical layout

| item | value | source |
|---|---|---|
| database directory | `fjall/` under the store root | `store/mod.rs:118` (`const DB_DIR`), `store/mod.rs:225` (`table_path`) |
| keyspaces | `entities`, `journal`, `meta` | `store/mod.rs:119-121`; doc block `store/mod.rs:9-14` |
| `entities` key | `<table>\0<entity-id>\0<sequence:u64 big-endian>` | `store/mod.rs:12`; encoder `store/keys.rs:28-35`, BE tail read positionally `store/keys.rs:41-53` |
| `entities` value | one serialized observation (JSON) per row; append-only, merged at read time by `Entity::merge` | `store/mod.rs:9-20`, `store/table.rs:99-114` |
| `journal` key | `<phase>\0<key>` → `{key, at, payload}` | `store/mod.rs:13`, encoder `store/keys.rs:91-96` |
| `meta` key | `<name>` → small JSON/scalar | `store/mod.rs:14`; legacy-import marker written by `store/legacy.rs` |
| entity id ceiling | 512 bytes (`MAX_ID_BYTES`) | `store/table.rs:19`; enforced in `store/keys.rs:66-89` |
| rows per table ceiling | 20,000,000 (`MAX_ROWS_PER_TABLE`) | `store/table.rs:15` |
| cache budget | 256 MiB unified LSM cache | `store/mod.rs:116` (`CACHE_BYTES`) |
| write API | `append_many`, `append`, `replace_many`, `replace` | `store/write.rs:25,57,72,92` |
| read API | `scan<T: Entity>`, `consolidate_table`, snapshot | `store/read/mod.rs:51,108`, `store/read/snapshot.rs:131` |
| raw HTTP bodies (not in Fjall) | `<cache>/{key}.body` + `<cache>/{key}.meta.json`, key = `sha256_prefix16(method \x1f url \x1f extra)`; meta carries status/sha256/fetched_at/last_modified/content_type | `net/cache.rs:28-40`, `net/types.rs:130` |
| backup manifest (not in Fjall) | `backup.json` listing every file with its sha256 digest, byte length, and per-table row counts | `store/backup/mod.rs:3-6` |

`Table` has **15** variants and the enum's own doc comment says "thirteen collections"
(`store/table.rs:21` vs `store/table.rs:23-52`) — a stale count, not a missing variant
(`Table::ALL: [Table; 15]`, `store/table.rs:73-89`).

### 4.2 Today's tables ↔ §7 targets

| §7 target | carrier today | key shape | value (row type) | production writer | verdict |
|---|---|---|---|---|---|
| `athletes` | `Table::Athletes` = `"athletes"` (`table.rs:58`) | `<table>\0<id>\0<seq BE>` | `CanonicalAthlete` (`store/entities/canonical.rs:97`) | 8 writers (`sources/athleticlive/results.rs:132`, `athleticnet/collect.rs:150`, `ihsa/tournament/map.rs:149`, `milesplit/results.rs:187`, `tfrrs/run.rs:178`, `wiaa_results/run.rs:192`, `athleticlive_athletes/batches.rs:161`, `census/sweep/roster.rs:36`) | **maps** |
| `schools` | `Table::Schools` = `"schools"` | same | `CanonicalSchool` (`canonical.rs:12`) | 13 writers (`sources/ihsa/collect.rs:124`, `ohsaa/collect.rs:245`, `ks/collect.rs:164`, `mshsl/collect/run.rs:159`, `plain_names/nsaa_walk.rs:177`, `nd_walk.rs:178`, `wiaa/collect_schools.rs:140`, `coach_contacts/import.rs:134`, `athleticnet/collect.rs:147`, `ihsa/tournament/map.rs:146`, `tfrrs/run.rs:175`, `athleticlive_athletes/batches.rs:159`, `census/sweep/roster.rs:31`) | **maps** |
| `coaches` | `Table::Coaches` = `"coaches"` | same | `CanonicalCoach` (`canonical.rs:61`) | 8 writers (`coach_contacts/import.rs:135`, `ihsa/collect.rs:136`, `ks/collect.rs:166`, `mshsl/collect/run.rs:160`, `ohsaa/collect.rs:249`, `plain_names/nsaa_walk.rs:178`, `nd_walk.rs:179`, `wiaa/collect_schools.rs:141`) | **maps** |
| `meets` | `Table::Meets` = `"meets"` | same | `CanonicalMeet` (`canonical.rs:131`) | 8 writers (`sources/athleticlive.rs:120`, `athleticlive/results.rs:129`, `athleticnet/collect.rs:148`, `ihsa/tournament/map.rs:147`, `milesplit/results.rs:184`, `tfrrs/run.rs:176`, `wayzata/walk.rs:220`, `wiaa_results/run.rs:190`) | **maps** |
| `performances` | `Table::Performances` = `"performances"` | same | `CanonicalPerformance` (`canonical.rs:164`) | 6 writers (`athleticlive/results.rs:134`, `athleticnet/collect.rs:152`, `ihsa/tournament/map.rs:151`, `milesplit/results.rs:188`, `tfrrs/run.rs:180`, `wiaa_results/run.rs:194`) | **maps** |
| `source_meets` | `Table::SourceMeets` = `"source_meets"` | same | `SourceMeetRef` (`derived.rs:62`) | `census/meets.rs:116` | **maps** (§30 semantics match) |
| `review_cases` | `Table::ReviewCases` = `"review_cases"` | same | `ReviewCase` (`derived.rs:35`) | `index.rs:79`, `identity/mod.rs:149` | **maps** |
| `conflicts` | `Table::Conflicts` = `"conflicts"` | same | `RetainedConflict` (`derived.rs:25`) | `index.rs:78` | **maps** |
| `collection_snapshots` | `Table::Snapshots` = `"snapshots"` | same | `CollectionSnapshot` (`derived.rs:89`) | `index.rs:85` | **rename**: on-disk/wire name differs from the §7 name (R7) |
| `source_coverage` + `jurisdiction_coverage` | `Table::Coverage` = `"coverage"`, discriminated by `CoverageScope` (`census-domain/src/model/records.rs:187-192`, slug at `:195`) | same | `CoverageRow` (`derived.rs:52`) | `index.rs:80` | **split**: one table with two scopes → two §7 tables (touches keys and the wire name) |
| `canonical_links` | `Table::SourceIdentities` = `"source_identities"` (§31 join comment, `table.rs:31-32`) | same | `SourceObjectIdentity` (`derived.rs:15`) | `index.rs:77` | **rename or alias**: if §7's `canonical_links` means the same provider-id→canonical edge, it is a rename; if it means something narrower, the row type needs a split |
| `source_observations` | `Table::SourceObservations` = `"source_observations"` (`table.rs:99`) | `<table>\0<id>\0<seq BE>` (`keys.rs:77`) | `SourceObservation` (`census-domain/src/model/records/source_observation.rs:24`) | the funnel `observe_schools_of` (`census-crawl/src/lib.rs:162`), called by 10 walks (`wiaa/collect_schools.rs:141`, `mshsl/collect/run.rs:161`, `ohsaa/collect.rs:246`, `ihsa/collect.rs:125`, `ihsa/tournament/map.rs:147`, `ks/collect.rs:165`, `plain_names/nd_walk.rs:179`, `plain_names/nsaa_walk.rs:178`, `athleticnet/collect.rs:148`, `census/sweep/roster.rs:32`) | **maps** (§30 semantics match) |
| `graduation_evidence` | none: grade evidence is inline — `ObservedGrade` (`census-domain/src/model.rs:210`) held as `CanonicalAthlete.observed_grades` (`model.rs:828`) | — | — | — | **no carrier**: ADR-003 requires time-scoped grade evidence; it exists as data, not as a table |
| `document_receipts` | none: no `Receipt` type exists in either crate; closest artifact is the cache sidecar `{key}.meta.json` | — | — | — | **no carrier** |
| `raw_documents` | none in Fjall: raw bodies are files `<cache>/{key}.body` | — | — | — | **no carrier** (on-disk cache, not a keyspace) |
| `identity_candidates` | none as a table: candidates are computed per packet (`identity/packets.rs:22` scans `ReviewCases`); `IdentityCandidatesTerminal` is an acceptance item (`census/state/evidence.rs:20`) | — | — | — | **no carrier** |
| `source_athletes`, `source_schools`, `source_results` | partial: `source_identities` rows typed `SourceEntityKind` (`census-domain/src/model/records.rs`), `source_meets`, and the **school** half of `source_observations` (`SourceSchoolObservation`, keyed by the provider's own object id); the athlete half is modelled (`SourceAthleteObservation`) but no walk writes it yet; result *files* are cached bodies, never stored as rows | — | — | — | **partial** |
| `athlete_performances`, `meet_performances`, `school_athletes`, `athlete_schools`, `school_coaches` | none: the relationships are embedded fields/ids on the canonical rows (`CanonicalPerformance.meet_id`, `CanonicalAthlete.school_id`, `CanonicalCoach.school_id` in `census-domain/src/model.rs`) rather than join tables | — | — | — | **missing** |

### 4.3 Carried today with no §7 name

| key | today | note |
|---|---|---|
| `teams` | `Table::Teams` = `"teams"`, `CanonicalTeam` (`canonical.rs:47`), 8 writers (`sources/athleticlive/results.rs:131`, `tfrrs/run.rs:177`, `wiaa_results/run.rs:191`, `athleticnet/collect.rs:149`, `ihsa/tournament/map.rs:148`, `milesplit/results.rs:186`, `athleticlive_athletes/batches.rs:160`, `census/sweep/roster.rs:33`) | no target name |
| `events` | `Table::Events` = `"events"`, `CanonicalEvent` (`canonical.rs:153`), 6 writers (`sources/athleticlive/results.rs:130`, …) | no target name |
| `source_access` | `Table::SourceAccess` = `"source_access"`, `SourceAccessCondition` (`derived.rs:99`), writer `census/sweep/access.rs:50` | blocked `(kind, host)` ledger |
| `identity_verdicts` | `Table::IdentityVerdicts` = `"identity_verdicts"`, writer `identity/mod.rs:148` | adjudications, one row per case |
| `journal` keyspace | `<phase>\0<key>` | resume journal; §7 has no equivalent |
| `meta` keyspace | `<name>` | legacy-import marker |

### 4.4 Writer/reader distribution today

67 production write call sites (`append`/`append_many`/`replace`/`replace_many` naming a `Table`)
across 22 files: 17 under `sources/` plus `census/sweep/{roster,access}.rs`, `census/meets.rs`,
`index.rs` and `identity/mod.rs`. A 23rd file holds the generic helper rather than a literal table:
`restate_services/jobs.rs:37` is `store.append_many(table, rows)` with `table` a parameter, called by
the census jobs for every table (`restate_services/jobs.rs:186` passes `Table::ALL`). Every canonical-table writer is under `sources/` or the crawl-side
`census/` files, and every derived table (`source_identities`, `conflicts`, `review_cases`,
`coverage`, `snapshots`) is written by exactly one production site (`index.rs:77-85`), except
`review_cases`/`identity_verdicts`, which also receive the review lane's own writes
(`identity/mod.rs:148-149`).

---

## 5. Restate surface today

Components bound in `crates/census-service/src/restate_services/mod.rs` (`build_endpoint`): service
`Census`, object `Ingest`, workflow `Sweep`, object `JurisdictionCensus`, workflow `NationalCensus`.
`jobs.rs` is a helper module (report writing), `wire.rs` + `wire/ingest.rs` are `PureData` wire types —
neither is a component.

| component | kind (site) | handlers | identity key today | who binds it (production) |
|---|---|---|---|---|
| `Census` | `#[service]` `census.rs:28,43` | `status` `:55`, `open_work` `:81`, `seal` `:102` (the job handlers `consolidate`/`report`/`bests`/`workbook` are the separate `Consolidate`/`Report`/`Bests`/`Workbook` workflows, `publish.rs:97,145,193,245`) | none (stateless service) | n/a |
| `Ingest` | `#[object]` `ingest.rs:15,65` | `state` `:77`, `record` `:83`, `complete_window` `:131` | one object per **endpoint string**: `IngestState.endpoint` is "Object key this state belongs to" (`wire/ingest.rs:12-14`); `SweepRequest.endpoints: Vec<String>` (`wire/ingest.rs:54-56`) | no in-repo binder; the key is whatever the caller passes |
| `Sweep` | `#[workflow]` `sweep.rs:18,36` | `run` `:56`, `wait_windows` `:113`, `observe_endpoints` `:147`, `interrupt` `:174` | not minted in-repo: the request carries endpoints, windows, window_seconds (`wire/ingest.rs:54-62`) | no production binder (`Sweep` reaches `IngestClient` by endpoint, `sweep.rs:155`) |
| `JurisdictionCensus` | `#[object]` `jurisdiction.rs:48,180` | `state` `:194`, `run` `:209` (+ owed stages `:89`) | `jurisdiction:{state}:{season}:{revision}` (`census/identity.rs:99-108`); object key comes from the client | `cli/national/run.rs:125,136`, `cli/national/observe.rs:104-106`; identity echoed in state (`restate_services/jurisdiction/stages.rs:207,226`) and reported at `restate_services/jurisdiction.rs:168` |
| `NationalCensus` | `#[workflow]` `national.rs:42,199` | `run` `:217`, `report` `:282` | `national:<season>:<scope>:<revision>` (`census/identity.rs:83-96`) | `cli/national/run.rs:84,99,162,164` |

### 5.1 Identity mapping against §8

| §8 target identity | constructor today | production binder | gap |
|---|---|---|---|
| `jurisdiction:{state}:{season}:{revision}` | `WorkflowIdentity::jurisdiction` (`census/identity.rs:99`) | `cli/national/run.rs:125`, `cli/national/observe.rs:104` | none |
| `source-sweep:{source}:{state}:{season}:{revision}` | `WorkflowIdentity::source_sweep` (`identity.rs:111`) | **none** (only `census/identity/tests.rs:51-65`) | the `Sweep` workflow is not addressed by this identity today (R: sweep is endpoint-keyed) |
| `meet:{source}:{source_meet_id}:{revision}` | `identity.rs:129` | **none** | ADR-004's per-meet workflow does not exist yet |
| `athlete:{source}:{source_athlete_id}:{revision}` | `identity.rs:134` | **none** | same |
| `school:{source}:{source_school_id}:{revision}` | `identity.rs:142` | **none** | same |
| `review:{evidence_digest}:{policy_revision}` | `identity.rs:150` | **none** | the review lane runs as a batch command (`cli/review.rs:10` → `identity::run_lanes`), not as a workflow |
| *(no §8 name)* `national:<season>:<scope>:<revision>` | `identity.rs:83` | `cli/national/run.rs:84,162` | the root run's identity is outside the §8 list |

Identity alphabet constraints that any new binder must respect: `MAX_IDENTITY_BYTES = 64`
(`census/identity.rs:33`), `MAX_PART_BYTES = 32` (`census/identity.rs:37`), oversize or unsafe parts
replaced by a digest (`census/identity.rs:234-247`); tests assert `:` inside a source id cannot forge
a boundary (`census/identity/tests.rs:92-94`).

### 5.2 Restate gaps

1. `Sweep` and `Ingest` are keyed by *endpoint strings*, not by source/state/season — §8's
   `source-sweep:{source}:{state}:{season}:{revision}` has a constructor but no binder.
2. `national:<season>:<scope>:<revision>` is bound but has no §8 name (a root-identity vocabulary gap).
3. Four of the seven constructors (`meet`, `athlete`, `school`, `review`) have zero production
   callers — they are the ADR-004 / review-workflow coordinates that later waves must bind.

---

## 6. Test, fixture and bench locations later waves must keep passing

| location | count | contents (what depends on it) |
|---|---|---|
| colocated test files under `crates/census-service/src` | 53 files named `tests.rs`/`*_tests.rs`; 58 files carry `#[cfg(test)]` blocks (59 hold `#[test]` fns) / 18,532 test lines | per-module unit tests; move with their module. Largest: `sources/plain_names/tests.rs` (1,203), `sources/mshsl/tests.rs` (799), `sources/athleticlive/results/tests.rs` (748), `workbook/recruiting/tests.rs` (630), `cli/merge_coaches/tests.rs` (599), `sources/ihsa/tournament/tests.rs` (545), `sources/athleticnet/meet/tests.rs` (542), `sources/milesplit/tests.rs` (535), `report/coverage/tests.rs` (510) |
| `crates/census-domain/src/*_tests.rs` | 4 files, 974 lines | `model_tests.rs`, `model/records_tests.rs`, `model/review_tests.rs`, `jurisdiction_tests.rs`, each wired by `#[cfg(test)] #[path = …] mod tests;` (`model.rs:1281`, `model/records.rs:375`, `model/review.rs:233`, `jurisdiction.rs:612`) |
| `crates/census-service/tests/` | 21 files | integration layer that spans future crates: `fjall_restate_e2e.rs` (store+service), `backup_restore.rs`, `recovery.rs`, `parity_{mideast,national,north,pipeline,wisconsin}.rs`, `parser_roundtrip_properties.rs`, `merge_properties.rs`, `athleticnet_meet_parity.rs`, and 9 `*_parser_properties.rs` / `*_directory_properties.rs` / `wayzata_schedule_properties.rs` proptest files |
| `crates/census-service/tests/fixtures/` | 72 files in 15 dirs | provider fixtures, e.g. `ihsa_tournament/` (11), `mshsl/` (11), `ohsaa/` (9), `milesplit/` (7), `plain_names/` (6), `wiaa_results/` (6), `athleticnet/` (4), `wiaa/` (4), `tfrrs/` (3), `ihsa/` (3), `athleticlive_results/` (2), `wayzata/` (2), `athleticlive/` (1), `athleticlive_athletes/` (1), `ks/` (1), plus `coach_contacts_sample.csv` |
| `crates/census-service/benches/` | 6 files | `core.rs` + `core/{fixtures,labels,lcg,merge}.rs` and `pipeline.rs`; `[[bench]] name = "core"` and `name = "pipeline"`, both `harness = false` (`crates/census-service/Cargo.toml:60-66`); criterion 0.8 dev-dep; `xtask` runs them via its bench verb |
| dev-dependency-only lanes | 1 each | `loom = "0.7"` (feature `loom`) drives `store/loom_tests.rs` and `spawn/loom_tests.rs`; `proptest = "1"` drives the 9 property files above; `tokio` (`test-util`) drives `net/execute/tests.rs`, `spawn/tests.rs` |
| formal harnesses | 7 files | `crates/census-domain/kani/{census_domain_wiring,gradyear,id_mint,publish}.rs` (wired by `lib.rs:19`), `crates/census-service/kani/{keys,merge,store_wiring}.rs` — the store harnesses must be re-pointed to `census-store` |
| outside the three crates but same gates | root `tests/` = 17 files | root package `tests/` + `tests/fixtures/` + `tests/native_parser_properties.proptest-regressions`, `benches/{artifact_store,blocking_fanout,workbook_export}.rs`, `fuzz/` (4 targets), `tools/gate.sh` + `tools/quality-baseline.json` |
| gates that must be **updated**, not merely kept green | 6 xtask modules | `xtask/src/seams.rs` (module-edge `ALLOWED` table → crate-edge graph), `xtask/src/purity.rs` (domain tree), `xtask/src/scan.rs` + `xtask/src/baseline.rs` (line budgets), `xtask/src/integrity.rs`, `xtask/src/templates.rs` + `xtask/src/scaffold.rs` + `xtask/src/source_fixture.rs` (adapter scaffolding paths), `xtask/src/dump_sheet.rs` (workbook consumer) |

---

## 7. Verification basis and open items

* Every line count, file count, public-item list, edge count and citation above was produced by
  reading the cited file at this revision (`read`/`find`/`wc -l`/`grep`, plus one Python pass for the
  edge list). No `cargo` command was executed, so **no compile-time claim** is made: the proposed
  targets are not yet buildable artifacts.
* `xtask/src/seams.rs`'s `ALLOWED` table is quoted from its doc comment only; the table body below
  the doc comment was not read this pass. Its own comment records `(sources, store)` at "24 refs" and
  the survey measures 27 at this revision — the comment, not the code, is stale.
* Public-surface counts exclude `pub(crate)`/`pub(super)` items and exclude test-only files; a
  type listed under one module may be re-exported by another through the shim blocks named in R6.
* Unverified by design (out of scope for a read-only pass): whether the target layout compiles, the
  cost of the `clock` signature change, and the migration order for the table renames in R7.
