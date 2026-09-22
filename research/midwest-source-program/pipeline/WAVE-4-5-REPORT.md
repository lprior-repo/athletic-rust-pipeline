## 12. Wave 4/5 outcome (2026-09-22)

Wave 4 took the four surfaces wave 3's research ranked highest, and wave 5 paid for them with
adversarial verification rather than more code. Both landed against the same rule as §10 and §11:
every lane edits its own files, the integrator owns `mod.rs`/registry/CLI and runs the gate.

**Surfaces.**

| Lane | What landed | Acceptance evidence |
| --- | --- | --- |
| MileSplit Ohio roster walk | `sources/milesplit` Ohio team index + roster walk (`fixtures/milesplit/oh_*`) | 13 Milesplit tests, roster walk e2e against the captured index |
| IHSA tournament decode | `sources/ihsa/tournament/**` — state-final index, event documents, summaries, XC qualifier lists, grade parsing | 20 tests; 1,571 qualifier rows with 1,569 grades; 240 schools, 145 teams, 1,636 athlete rows, 20 performances in one walk; resume run spends 4 cached requests and re-mints nothing |
| TFRRS adapter | `sources/tfrrs/**` — per-state performance lists and team rosters, parse/`map` split | 9 tests over 3 captures; list page 1.88 MB read for 1 request |
| AthleticLIVE result plane | `sources/athleticlive/results.rs` + `map_rows/`, `absorb.rs` — the three wire documents folded into canonical rows | 22 tests; 136 performances and 41 teams from two captures, second run merges to the same 136 |
| Athletic.net whole-meet acquisition | `sources/athleticnet/meet.rs` — meet → results → optional event metadata, 2 requests per meet | 10 module tests + 3 offline end-to-end parity tests; registry row now `bulk_results: true` |

**Defects the wave found, and what fixed them.** These are the point of the wave; the surfaces are
what made them reachable.

| Id | Defect | Fix and evidence |
| --- | --- | --- |
| D1 | Five walk sites journaled a unit as done *before* its rows were in the store, so a kill between the two left the journal claiming rows that never landed (`ks`, `plain_names` ND/NSAA, IHSA top-level, Athletic.net; TFRRS had the same shape) | append-then-journal at each site; TFRRS defers every claim until `collect` has appended. `tests/recovery.rs` ladder: `claimed_without_rows_at_kill=0 missing_from_the_final_store=0` (was 4/4), restart equals the control store, and `cargo xtask source-check ks` — 31/31 — green |
| D2 | `bootstrap/serve.rs` armed the drain deadline before waiting for a stop request, so an endpoint that was never asked to stop was killed at the deadline and reported `ServerExit` | stop-watch → cancel → drain ordering. `recovery drain-deadline`: `still_answering=true`, drain report `accepted=2 completed=2 timed_out=0`, operator stop `stop_reason: Signal` |
| D3 | The Coverage sheet's `Note` row embedded the store root, and the workbook parity normaliser blanked the store path only for the `Store`/`Core note` labels, so `pipeline_publishes_the_same_bytes_from_a_rebuilt_store` failed on a digest that changed every run | `volatile_cell` normalises `Note` too; the Coverage sheet digest is now identical across three consecutive rebuilds |

Lane-local bugs the same tests caught: `athleticlive::results` counted refusals inside a memo
(`or_insert_with`), so 8 of 16 refused rows were uncounted and the row-count invariant silently
broke; `Run::new` minted a meet but never placed it in the accumulator, leaving 136 performances
pointing at an absent meet; IHSA's `limit` counted *walked* meets so a resumed run re-walked 97
live pages; `plain_names` passed a `Vec` to single-record `append`; TFRRS read every section's
gender as `None`.

**Verification on the frozen tree.**

| Check | Result |
| --- | --- |
| `tools/gate.sh` | PASS every lane: fmt, check, doc, tests (**726 passed, 2 skipped**), strict clippy on source targets (**0 diagnostics**), production scan, domain purity, module seams, debt ratchet, deny, audit, vet, machete, geiger, feature powerset |
| `cargo xtask scan` | root 305 files / 36,656 production lines, census 251 / 39,340, every forbidden counter 0; **0 files > 300 lines, 0 functions > 60 logical lines** |
| `cargo xtask source-check ks` | 31 passed, 0 failed — including `recovery ks_directory_walk_claims_units_the_kill_can_lose` |
| `tests/recovery.rs` | 8 scenarios green (kill ladder, worker/service SIGKILL, drain deadline, resume, flush) |
| Legacy parity, same store | `census-by-state-all-sources.csv` byte-identical to the pre-Rust (JS) output; `census-by-state{,-core}.csv` differ only in one label (`ALL` → `TOTAL`), every figure identical |
| Workbook | 20 sheets: the legacy vocabulary (`Athletes`, `PRs`, `Performances_001`, `Coaches`, `Schools`, `Meets`, `Sources`, `Coverage`, `Conflicts`, `Review`, `Run Metrics`) plus the nine summary pages; five summary sheets byte-identical to the pre-wave export, four grew with the new sources' states |

**Honest notes.**

* Goldens were reseeded for the pipeline corpus (`report-core`, `report-all_sources`,
  `census-by-state-*`, `workbook-shape`) and for the new Ohio Milesplit captures. The churn is
  additive, checked per sheet: `Goal & method`, `Summary`, `Best results`, `Evidence mix` and
  `Method notes` are byte-identical, and the report's growth is new states (AK…WY appear under the
  new sources), not changed rows.
* Fixtures were split per adapter — `tests/fixtures/ihsa_tournament/` and
  `tests/fixtures/athleticlive_results/` — because both parity corpora require every file in a
  source's directory to be a payload kind that corpus can parse; the new payload families now have
  their own directories, and the IHSA corpus test is green again on three files.
* The `ihsa_tournament` adapter gained a `provider` CLI arm. The AthleticLIVE result plane did
  **not**: its route needs operator-supplied captures *and* the meet they belong to, so its entry
  point stays `athleticlive::collect_results` (the registry row documents that), rather than a CLI
  arm that could only fail for want of a target.
* Two of the wave's lanes (athleticlive results, TFRRS) had to be given explicit exit criteria
  mid-flight: both were at risk of landing a compiling adapter with failing tests. One exit — fix
  or delete with a reason — is what kept the tree's red set empty at freeze.
