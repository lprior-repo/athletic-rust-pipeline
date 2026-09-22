# Midwest TF/XC source program — the research this pipeline is built from

The working corpus lives outside the repository (it carries large raw assets): it is
`~/Downloads/midwest-tfxc-source-research/`, 1.9 GB, and this directory mirrors its distillates so
the reasoning the adapters implement is reviewable next to the code that implements it.

## What is mirrored here

| Path | Files | What it holds |
| --- | --- | --- |
| `brief/` | 4 | the mission brief, the 30 agent assignments, the coach-lane brief, the consolidation brief |
| `research/midwest/` | 58 | the 49 assignment reports plus their evidence notes (`evidence/gap-closure`, `evidence/gaps/**`) |
| `synthesis/` | 17 | the executive summary, the nine acceptance answers, the state playbook, adapter ranking, canonical mapping, compliance and risks, source matrix, open questions, weekly-incremental design, both gap plans, the measured census, the live pipeline state, the acceptance audit, and the Athletic.net endpoint ground truth |
| `data/` | 18 | the products under the 25 MB cap: adapter ranking, source-coverage matrix, school alias map, canonical schools/coaches/meets, DirectAthletics indexes, MileSplit coverage, coach contacts, the AthleticLIVE meet inventory and seeds |
| `reports/` | 16 | the measured scopes, by-state CSVs, best results for the class of 2027, and the run reports |
| `scripts/` | 1 | the fragment merge script the coach lane calls |

## What is deliberately not mirrored

Four products exceed the 25 MB cap and stay in the working corpus:

| Size | File |
| --- | --- |
| 188.1 MB | `data/canonical-athletes-co2027.csv` |
| 114.2 MB | `data/recruiting-co2027.csv` |
| 72.8 MB | `reports/midwest-census-2026-09-22.xlsx` |
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
the tier list in `crates/midwest-census/README.md` puts the association, MileSplit, vendor-artifact
and timer surfaces ahead of the mirror, and why the whole-meet pass is ordered by expected yield
(championship and qualifier meets first) rather than by list position.
