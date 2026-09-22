# Gap-resolution plan — from the 2026-09-22 sealed coverage sheet

Input: the `Coverage` sheet of the sealed workbook (`7b41a264`). Each gap class, what closes it, and
the order to run it. Numbers are the cohort athletes lacking that dimension, per state.

**Superseded input (2026-09-22, after 11:53).** The workbook was rebuilt after the coach-plane import and the
fragment union, so the on-disk workbook now digests `c63f7fb4f38f55167fc3019e722bff122bbac2fc156410552935d786c169da21`
and the `7b41a264` sheet this plan reads is no longer on disk. The coach gaps moved with it: on the 12-state
filter, Co2027 with an identified TF/XC coach 43,201 → **57,115** and with a professional coach email
33,340 → **46,185**, every one of the +735 email rows in Wisconsin. Re-read the coverage numbers from the
11:53 products (`reports/census-by-state{,-core}.csv`, `reports/report{,-core}.json`) before running the plan;
`data/acceptance-audit.json` carries the same note and is likewise a pre-import generation.

## Class 1 — `missing_performance_history` (dominant, 190k of 190k in most states)

Only Wisconsin carries performances (7,503 athletes) because the result adapters have been walked
for WI alone. Everything else in this class closes with the meet/result pass:

1. `meets --all-states --year 2026` — enumerates the published meet index into `source_meets`
   (running; ~10 pages per state, journaled per page so a re-run resumes).
2. `provider --name milesplit_results` — pulls the discovered meets' result blocks.
3. `provider --name athleticnet --meets <ids>` — the whole-meet route: **two requests per meet**
   (`Meet/GetMeetData` → `Meet/GetAllResultsData`), measured at 758 individual rows + 288 relay legs
   for one meet. This is the cheapest route into performance history that exists, and it is the one
   deliberate Athletic.net surface. The 9,593 meets already carrying an Athletic.net id are the
   priority list; the ids were obtained without Athletic.net.
4. `bests` → `workbook` → `seal --write`.

Ordering inside the pass: the twelve Midwest states first, then the rest.

## Class 2 — `missing_coach` (largest single-state deficits)

| State | Missing | Closing lane |
|---|---|---|
| OH | 28,639 | OH fragment landed (190 rows, 42 schools); re-import shrinks this |
| MI | 20,046 | MI fragment landed (106 rows, 26 schools); re-import shrinks this |
| IL | 18,288 | IL fragment landed (197 rows); re-import shrinks this |
| IA | 13,971 | IA fragment landed (93 rows, 25 schools); re-import shrinks this |
| IN | 13,796 | IN fragment landed (130 rows, 29 schools); re-import shrinks this |
| MO | 13,380 | MO fragment landed (61 rows, 27 schools); re-import shrinks this |
| WI | 5,485 | WI fragment (213 rows) + WIAA directory in the store |
| SD | 4,585 | SD lane running |
| MD/KS/NE/MN/ND | 1.1k–3.8k | KS lane running; MN second pass running; NE/MN/ND fragments landed |

The lever is bounded by what the sources publish: NDHSAA publishes no coach email at all (0 of 36
schools), so ND's coach *identity* closes but its *email* gap cannot. Every lane's report records
the hosts that refuse (JS shells, 403s, dead DNS) rather than working around them.

## Class 3 — `missing_profile` (4.2k worst case, WI)

WI 4,153 · IA 2,729 · IN 2,173 · OH 1,716 · NE 1,332. Closes with MileSplit state-roster walks
(the lane that answered 403 for ND/SD stays excluded) and the AthleticLIVE roster walk.

## Class 4 — `missing_pr_support` (90 athletes, WI only)

Athletes whose best mark has no supporting row on the PR scales. Closes with the same result pass as
Class 1, then a `bests` re-run.

## What is *not* worth another pass

- **Athletic.net profile discovery** — 93,619 profile URLs already arrive as ids published by other
  sources. Broad Athletic.net search is disallowed by robots and unnecessary.
- **ND/SD MileSplit rosters** — 403, recorded, never retried.
- **MHSAA bulk API** — `Disallow: /DesktopModules/`; no adapter ships.
- **OHSAA portal via rustls** — every request reset at connect; no bypass attempted.

## Re-run sequence once lanes land

```bash
python3 tools/merge_coach_fragments.py --fragments out/coach-fragments --out data/coach-contacts.csv
./target/release/midwest-census import-coaches ~/Downloads/midwest-tfxc-source-research/data/coach-contacts.csv --observed-on 2026-09-22
./target/release/midwest-census bests
./target/release/midwest-census workbook --grad-year 2027
./target/release/midwest-census seal --write
```
