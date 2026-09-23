# The routed-ingest wave and the 2026-09-23 seal

This is the record of the wave that made the service execute its own collection: every walk that
acquires rows now travels back to a durable Restate object instead of being discarded between
passes, and the census that ran on 2026-09-23 carries a seal in the store.

## What changed

| File | Change |
| --- | --- |
| `crates/census-crawl/src/recording.rs` | new — a `RowSink` the walks write through, plus the `take_recorded` batch the caller flushes |
| `crates/census-crawl/src/context.rs` | new — the per-pass context the walks borrow, so one signature threads the sink and the reader |
| `crates/census-service/src/restate_services/ingest_post.rs` | new — posts a recorded batch to the lane's `Ingest` object |
| `crates/census-service/src/census/meets.rs`, `meets/walk.rs` | the index walk is routed first and its `RowSink` is the recording; one `take_recorded` serves index plus arms |
| `crates/census-service/src/restate_services/jurisdiction/stage_runs.rs` | the stage writes the recorded batch to the store as the object's own acquisition, then posts it |
| `crates/census-service/src/cli/meets.rs` (via `cli/*`) | `census-service meets` passes no sink — the unrouted batch path is unchanged |

Before this wave the walk's rows were journal markers only: the objects stayed at zero observations
and the second §70 item stood open by name. After it, a pass that enumerates an index *acquires
from* it, which is what the item's rule (`observations > 0`) asks.

## Measured on the revision-3 pass

```
jurisdiction:WI:2026-27:3 finished 2026-09-23 · stages meets     140.16 seconds
teams 597 · rosters 0/597 skipped 597 · athletes 0 · co2027 0 (boys 0 girls 0)
sweepable 4 · owed 4 of 8 planned · athleticlive,athleticnet,athleticlive_athletes,coach_contacts
```

The object check that followed:

| endpoint | observations | windows | unreadable |
| --- | --- | --- | --- |
| `milesplit_wi` | **998** | 1 | false |
| `wiaa_results_wi` | 0 | 1 | false |
| `wayzata_wi` | 0 | 1 | false |

The index walk posted 998 rows to its `Ingest` object; the two result arms accepted their window
markers and found nothing new to publish, which is why they stay at zero and are not named in the
seal's object scope. `owed sweeps: 0` — the first item stays terminal.

## The seal

`census-service seal --season 2026 --revision 1 --source-object milesplit_wi --write`:

```
phase: complete
workbook: var/midwest-census/out/census-service-2026-09-23.xlsx
acceptance: every §70 item is satisfied
sealed 5ab49d85c232c26363e8c2e04695ab2c6394d84eb31590c70fed78ccba9ad233 on 2026-09-23
  cohort 579732 of 2225091 athletes, 31818 schools, 11016 meets, 23970 performances, 31488 coaches
  retained: 113 gaps, 4548 conflicts, 0 access conditions (0 hosts refused, 0 throttled)
```

`var/midwest-census/out/seal.json` carries the same evidence, with 3,859,887 observations and 23,970
calculations retained. The gap classes are retained findings, not refusals: the largest are
`missing_performance_history` and `missing_coach`, both 9,120 athletes, which is the Class 1 and
Class 2 deficit `13-gap-resolution-plan-2026-09-22.md` describes.

## The export

`cargo xtask export` rebuilt the workbook through the deployment in 96 seconds (2 sheets of §50-§54
families, the eleven named sheets, and the nine research sheets). `census-service export-data
--store-out var/midwest-census/out --data research/midwest-source-program/data` rewrote the data
products in 21 seconds:

| Metric | Value |
| --- | --- |
| athletes | 2,374,515 |
| athletes in more than one source | 72,544 |
| class of 2027 | 625,899 |
| class of 2027 with any coach email | 76,411 |
| class of 2027 with a school coach | 81,990 |
| coaches | 31,488 |

The `Athletes` sheet carries 579,732 data rows — the store's class of 2027 exactly. Its `Coverage
State` column:

| Coverage state | Rows |
| --- | --- |
| `identity-only` | 572,170 |
| `pr` | 5,193 |
| `performance` | 2,369 |

and its `Contact Coverage State` column over the same rows:

| Contact coverage state | Rows |
| --- | --- |
| `contact_source_not_attempted` | 495,462 |
| `professional_coach_email` | 47,416 |
| `professional_ad_email` | 20,884 |
| `coach_name_only` | 13,801 |
| `contact_conflict` | 2,169 |

The identity-only majority is the Class 1 deficit the gap plan names — athletes with a public
profile URL and no performance history in the store yet — and the sheet says exactly that instead
of implying more. 47,416 athletes carry a public professional coach email and 20,884 an athletic
director email; the rest say why not.

The sheet's columns are the §50 contract: `Athlete ID`, `Name`, `State`, `School`, `Graduation
Year`, `Current Grade`, `TF`, `XC`, `Indoor`, `Outdoor`, `Event list`, `Headline PR summary`,
`Performance count`, `Meet count`, `Athletic.net URL`, `MileSplit URL`, `Other profile URLs`,
`Head TF Coach`, `Head XC Coach`, `Coach Professional Email`, `Athletic Director`, `School
Athletics URL`, `Public Recruiting GPA`, `GPA Source`, `Sources Count`, `Identity Confidence`,
`Coverage State`, `Conflict Flag`, `Review Flag`, `School City`, `Head TF Coach Email`, `Head XC
Coach Email`, `AD Email`, `Preferred Recruiting Contact`, `Preferred Contact Role`, `Preferred
Contact Email`, `Contact Coverage State`. A row with no public professional contact says so in
`Contact Coverage State` rather than carrying a placeholder, and `GPA` stays empty unless
`GPA Source` names where it came from.

## The row-level check

`census-service verify --store var/midwest-census --workbook
var/midwest-census/out/census-service-2026-09-23.xlsx` — the offline route, which needs the
deployment stopped because the store locks while it holds it — passes:

```
verify: OK (5000 athletes sampled of 579732 rows, 5000 performances sampled of 202979 rows)
```

Run without `--store`, the same command reads the default store (`var/census-service`, a different
census) and reports the workbook's first row as missing:

```
athletes verification failed: athletes row 0: id ath_5d1cdacbbab9e8e2 not in store
```

That is the flag and nothing else. The sheet is written from the Midwest store
(`census-serve --data-dir var/midwest-census`), its 579,732 rows are that store's class of 2027, and
`crates/census-reconcile/src/verify/compare.rs` checks them against `Table::Athletes` row for row.
The seal's `ExportVerified` evidence was recorded by an offline pass; anyone re-running the check
must name `--store var/midwest-census` or they will verify the workbook against the wrong store.
