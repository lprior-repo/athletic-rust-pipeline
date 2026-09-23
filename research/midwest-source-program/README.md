# Midwest TF/XC source program — the research this pipeline is built from

The working corpus lives outside the repository (it carries large raw assets): it is
`~/Downloads/midwest-tfxc-source-research/`, 1.9 GB, and this directory mirrors its distillates so
the reasoning the adapters implement is reviewable next to the code that implements it.

## What is mirrored here

| Path | Files | What it holds |
| --- | --- | --- |
| `brief/` | 4 | the mission brief, the 30 agent assignments, the coach-lane brief, the consolidation brief |
| `research/midwest/` | 58 | the 49 assignment reports plus their evidence notes (`evidence/gap-closure`, `evidence/gaps/**`) |
| `synthesis/` | 18 | the executive summary, the nine acceptance answers, the state playbook, adapter ranking, canonical mapping, compliance and risks, source matrix, open questions, weekly-incremental design, both gap plans, the measured census, the live pipeline state, the acceptance audit, the Athletic.net endpoint ground truth, and the routed-ingest and seal record |
| `data/` | 22 | the products under the 25 MB cap: adapter ranking, source-coverage matrix, school alias map, canonical schools/coaches/meets, DirectAthletics indexes, MileSplit coverage, coach contacts, the AthleticLIVE meet inventory and seeds, the acceptance audit, the coach-merge report, and the Athletic.net id-field inventory |
| `reports/` | 16 | the measured scopes, by-state CSVs, best results for the class of 2027, and the run reports |
| `scripts/` | 1 | the fragment merge script the coach lane calls |
| `out/` | — | the coach lane's outputs: per-state fragments, their verified re-derivations, the verified union, the merged contacts CSV, the freeze manifests and gate tables, and the acceptance-by-state note |
| `pipeline/` | — | the last full run's evidence: `report.json`, `report-core.json`, the derived coverage/conflicts/review-case streams, the gate log and `best-results-co2027.csv` |
| `repo-probes/`, `targets/` | — | the probe scripts and the target lists the lanes consumed |

The `data/` and `reports/` artifacts are force-added: the repository's `.gitignore` skips CSV and
JSONL by default, and these eighteen data products and sixteen reports are the products the program
exists to publish, not regenerable scratch.

One false alarm worth recording, because it will look like data loss on a re-read: merging the
verified union reports 6,237 rows "missing" against `data/coach-contacts.csv`, and they are not
missing. `merge-coaches` keeps one row per school/sport/role and reports the rest as duplicates —
WI 10,363 → 3,166 kept with 7,197 duplicates, MN 1,180 → 389 with 791, ND 15, MI 7, NJ 17 — plus 31
rows rejected for carrying no role label (DC 16, ND 15). The frozen CSV already is that merge.

## What is deliberately not mirrored

Four products exceed the 25 MB cap and stay in the working corpus:

| Size | File |
| --- | --- |
| 188.1 MB | `data/canonical-athletes-co2027.csv` |
| 114.2 MB | `data/recruiting-co2027.csv` |
| 72.8 MB | `reports/census-service-2026-09-22.xlsx` |
| 37.0 MB | `pipeline/census-service-2026-09-22.xlsx` (same run's workbook, committed instead as `report.json` + `report-core.json`) |
| 35.2 MB | `data/athleticnet-athlete-seeds.csv` |

The raw asset trees are outside the repository for the same reason: `research/` assets (413 MB —
captures, screenshots, raw payloads), `tools/` (882 MB), `scratch-*` (90 MB), `pipeline/` (45 MB),
`reports/` beyond the cap (75 MB), `out/` (13 MB), `targets/`, `repo-probes/`.

## The measured results these documents carry

Two scopes are published and never merged: **Core** (every source except the AthleticLIVE mirror) and
**All sources**. The 2026-09-21 measurement, quoted from `synthesis/01-acceptance-answers.md`:
787,584 athletes all grades / 190,087 in the class of 2027 across the twelve Midwest states; 93.1 % of
the class of 2027 carries a public profile URL; 59,269 rows are multi-namespace. The pipeline's own
store on 2026-09-22 holds 3.06 M athletes, 88 k schools, 207 k teams, 58 k coaches and 12.2 k meets.

`synthesis/12-acceptance-audit-2026-09-22.md` is the audit of those answers against the artifacts, and
`synthesis/13-gap-resolution-plan-2026-09-22.md` is the plan the collection lanes are executing:
Class 1 missing performance history (Wisconsin is the only state with results in the store),
Class 2 missing coach identity (largest deficits OH 28.6 k, MI 20.0 k, IL 18.3 k), Class 3 missing
profile URL (WI 4.2 k, IA 2.7 k, IN 2.2 k), Class 4 unsupported PRs (90 athletes).

## The one access fact that shapes the collection design

`synthesis/atn-endpoint-groundtruth.md` and this program's probes agree: Athletic.net gates the bulk
result endpoint. Measured 2026-09-22 with the pipeline's own fetcher, `provider athleticnet --meets
<ids>` reports `blocked www.athletic.net rate_limited` while `Meet/GetMeetData` still answers 200:
the whole-meet route's second request (`Meet/GetAllResultsData`) is the one that returns 429, and the
fetcher's policy then parks the host for six hours. That is why the adapters are ranked as they are —
the tier list in `crates/census-service/README.md` puts the association, MileSplit, vendor-artifact
and timer surfaces ahead of the mirror, and why the whole-meet pass is ordered by expected yield
(championship and qualifier meets first) rather than by list position.

## The 2026-09-23 seal

The census the lanes have been feeding is sealed in the store on 2026-09-23: digest
`5ab49d85c232c26363e8c2e04695ab2c6394d84eb31590c70fed78ccba9ad233`, 50 jurisdictions, 2,225,091
athletes, 579,732 in the class of 2027, 31,818 schools, 11,016 meets, 23,970 performances and
31,488 coaches, with 113 gaps and 4,548 conflicts retained and no host refused or throttled.
`synthesis/14-routed-ingest-and-seal-2026-09-23.md` records the wave that made it possible — the
walks now post what they acquire back to their own Restate objects, which is what closes the second
§70 item by measurement — along with the pass timings, the export counts, the workbook's sheet and
column contract, and the row-level check: `census-service verify --store var/midwest-census
--workbook var/midwest-census/out/census-service-2026-09-23.xlsx` passes (5000 of 579,732 athletes
sampled, 5000 of 202,979 performances). Drop the `--store` flag and the same command reads the
default store, a different census, and reports the workbook's first row as missing; the flag is the
whole difference.

The coach lane's provenance CSV (`research/midwest/47-coach-provenance.csv`) was synced in from the
working corpus, so the 47th report's evidence is reviewable next to the report that cites it.
