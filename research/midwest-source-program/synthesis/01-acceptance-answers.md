# Acceptance answers — Midwest TF/XC Class-of-2027 source research (measured)

Nine questions, each answered from the newest artifacts on disk. Supersedes the estimate-only version of this
file: the numbers below are measured, not modelled, wherever a measurement exists.

**Provenance rule.** Every number carries a bracketed source. Four generations of numbers exist and they are never averaged: **(1) research-phase estimates** — `research/midwest/01..46-*.md`, `data/adapter-ranking.csv`: per-source yields measured per source but never summed into a census; **(2) measured census 2026-09-20** — `synthesis/10-measured-census.md`, `reports/midwest-census-2026-09-20-*.csv`, `data/*.csv`: the first pipeline run (Python-era store); **(3) measured census 2026-09-21** — `reports/report.json`, `reports/report-core.json`, `reports/census-by-state{,-core}.csv`, `reports/midwest-census-2026-09-21.xlsx`, `reports/best-results-co2027.csv`: the Rust port's regeneration of the same store, what the answers below used until the 2026-09-22 refresh; **(4) measured census 2026-09-22** — `reports/report.json`, `reports/report-core.json`, `reports/census-by-state{,-core}.csv`, `reports/midwest-census-2026-09-22.xlsx`, `reports/best-results-co2027.csv`, `data/*.csv`: the coach-plane import (store coaches 29,294 → 31,488, 12-state coach rows 28,724 → 29,968) plus the merged AthleticLIVE side store, the newest measurement and what every coach figure below uses.

**Two scopes, both published, never merged** (`reports/midwest-census-2026-09-20-summary.csv` names them `Core (Athletic.net off)` and `All sources`). Core drops every athlete whose only evidence is the AthleticLIVE timer plane — measured from the store as **135,848** all-grade / **43,229** Class-of-2027 rows (`var/midwest-census/out/athletes.jsonl`, evidence source set `== {athleticlive_athletes}`) against **145,323** rows stated by `reports/report-core.json` `notes[]`. Both numbers are printed wherever they matter; 135,848 is the figure this file re-derived from the store.

**Citation keys.** `[reports/…]`, `[data/…]`, `[research/midwest/…]` are paths under `~/Downloads/midwest-tfxc-source-research/`. `[lane: <name>]` is `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/research/sources/<name>/SOURCE_REPORT.md` with its `coverage.json`/`schema.json`. `[store: <path>]` is a file under `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/var/`. `[repo: <path>]` is a source file in the pipeline crate. `[INFERENCE]` marks reasoning; everything else was counted.

**Headline totals used throughout** (`reports/census-by-state.csv`, `reports/report.json`, 2026-09-21):

| metric | All sources | Core (A.net off) | 2026-09-20 CSV export |
|---|---:|---:|---:|
| schools | 12,267 | 12,267 | — |
| athletes, all grades | 787,584 | 651,736 | — |
| **Class of 2027** | **190,087** | **146,858** | **183,875** |
| Co2027 boys / girls / no gender | 105,755 / 84,034 / 298 | 80,896 / 65,751 / 211 | (export has no unknown-gender column) |
| Co2027 with any public profile URL | 176,885 (93.1 %) | 142,916 (97.3 %) | 174,600 (95.0 %) |
| Co2027 with grade evidence | 190,087 | 146,858 | 183,875 |
| Co2027 multi-namespace | 59,269 (31.2 %) | 0 | 56,842 (30.9 %) |
| Co2027 with an identified TF/XC coach | 43,201 (22.7 %) | 37,179 (25.3 %) | 22,183 (12.1 %) |
| Co2027 with a professional coach email | 33,340 (17.5 %) | 28,929 (19.7 %) | 15,338 (8.3 %) |
| coaches / coaches with email | 27,580 / 8,284 | 27,580 / 8,284 | — |
| meets / meets carrying an Athletic.net id | 11,007 / 9,593 | 1,532 / 0 | 9,667 / 9,593 |

**2026-09-22 refresh (12-state filter; `reports/census-by-state{,-core}.csv`, `reports/report{,-core}.json`).**
The athlete and school plane is unchanged inside the twelve states — the store's growth is national (2,374,515
all-grade athletes over 52 jurisdictions against 787,584 in the twelve) — so only the coach plane and the
national totals moved:

| metric | All sources 2026-09-21 | All sources 2026-09-22 | Core 2026-09-22 |
|---|---:|---:|---:|
| schools (12-state) | 12,267 | 12,272 | 12,272 |
| Co2027 with an identified TF/XC coach (12-state) | 43,201 (22.7 %) | **57,115 (30.0 %)** | 50,318 (34.3 %) |
| Co2027 with a professional coach email (12-state) | 33,340 (17.5 %) | **46,185 (24.3 %)** | 41,026 (27.9 %) |
| coach rows / rows carrying an email (12-state) | 27,580 / 8,284 | 29,968 / 9,970 | 29,968 / 9,970 |
| national store: coaches / with email, Co2027 / with coach email | 27,580 / 8,284 | 31,488 / 10,671, 625,899 / 58,260 | 31,488 / 10,671, 582,691 / 53,122 |

The national-store row is re-measured after the post-merge rebuild, from `[store: var/midwest-census/out/report.json]`
(all sources, `census.report --print` through `Report/run`) and `[store: var/midwest-census/out/report-core.json]`
(core, `census.report --core`); the workbook rebuilt from the same store carries the same figures on its
`Run Metrics` sheet (core athletes 2,238,191 / Co2027 582,691, all sources 2,374,515 / 625,899), so the two
surfaces agree rather than merely being copied.

The 12-state coach-email gain of +735 rows sits entirely in Wisconsin (`reports/census-by-state.csv`: WI coach
rows 2,719 → 3,963, WI rows with an email 2,302 → 3,242); every other state's coach columns are byte-identical
between the two generations.

The third column is `data/canonical-athletes-co2027.csv` + `data/recruiting-co2027.csv`, both 183,875 rows;
its coach figures are a **different join** from the census's (see Q5). Where the export and the census differ,
the census (`reports/`) is retained because it is the 2026-09-21 measurement and is re-derivable from
`var/midwest-census/out/athletes.jsonl` / `coaches.jsonl`; the discrepancy itself is printed, not averaged.

---

## 1. How many Class-of-2027 athletes each source discovers

**Answer: the census's 190,087 Co2027 rows come from exactly three athlete-bearing source families — AthleticLIVE
108,174 rows (56.9 %; `athleticlive_athletes`, with `athleticlive_meets_csv` ever only on rows that already carry
it), the twelve MileSplit state rosters 142,535 rows (75.0 %), WIAA tournament artifacts 7,503 rows (3.9 %) — and
Athletic.net 0 rows** (the census made zero Athletic.net requests); the seven association/coach modules
(`ihsa`, `ks`, `mshsl`, `plain_names`, `ohsaa`, `wiaa`, `coach_contacts`) contribute schools and coaches but
**zero** athletes, and the remaining crate modules are parsers.

Counted from `[store: var/midwest-census/out/athletes.jsonl]` (787,584 lines; 190,087 with `grad_year == 2027`),
evidence source set per athlete:

| evidence source (`evidence[].source.id`) | Co2027 athletes carrying it | grade-evidence rows (`report.json` → `providers.grade_evidence_sources`) | lane-side yield (source reports) |
|---|---:|---:|---|
| `athleticlive_athletes` | **108,174** | 462,875 | 256 tenant indexes / 153,774 meet docs / 129,203 with an Athletic.net meet id `[lane: national-aggregators §3.2,§3.5]`, `[lane: timing-providers-national §3]` |
| `milesplit_oh` | 26,998 | 27,000 | OH 977 HS team records `[lane: milesplit-national §2]` |
| `milesplit_il` | 19,392 | 19,398 | IL 849 teams (`data/milesplit-coverage-matrix.csv`) |
| `milesplit_mi` | 16,176 | 16,176 | MI 895 teams (same) |
| `milesplit_wi` | 14,958 | 14,984 | WI 597 teams (same) |
| `milesplit_mo` | 13,218 | 13,218 | MO 700 teams (same) |
| `milesplit_in` | 11,909 | 11,909 | IN 533 teams (same) |
| `milesplit_mn` | 11,261 | 11,261 | MN 592 teams (same) |
| `milesplit_ks` | 9,252 | 9,252 | KS 413 teams (same) |
| `milesplit_ia` | 8,738 | 8,738 | IA 411 teams (same) |
| `milesplit_ne` | 5,921 | 5,921 | NE 317 teams (same) |
| `milesplit_sd` | 2,669 | 2,669 | SD 196 teams (same) |
| `milesplit_nd` | 2,043 | 2,043 | ND 153 teams (same) |
| **MileSplit subtotal** | **142,535** | 142,569 | 26,562 HS team records nationally, 51/51 hosts `[lane: milesplit-national §2,§16]` |
| `wiaa_results` | **7,503** | 13,573 | 1,796 WI team-seasons; WIAA publishes 2026 T&F tournament Athletic.net MeetIDs + 104 result-file links/season `[research/midwest/06-wisconsin-wiaa.md]` |
| `athleticnet` (adapter) | **0** | 0 | zero requests; the 9,593 Athletic.net meet ids and 93,619 athlete ids arrived *as ids published by other sources* `[lane: athleticnet §21.1]` |
| association adapters (`ihsa`, `ks`, `mshsl`, `plain_names`, `ohsaa`, `wiaa`, `coach_contacts`) | **0 athletes** | 0 | they write `CanonicalSchool` + `CanonicalCoach` only — `[lane: state-assoc-plains §10]` "Does NOT collect: athletes" for KS/MN/ND/NE |
| TFRRS / DirectAthletics / MaxPreps / World Athletics / AAU / RunnerSpace / Cross Country Ratings / USATF | **0 in the census** | 0 | research-lane only; IN TFRRS 1,671 athletes in one HSR list, `[research/midwest/18-indiana-directathletics-milesplit.md]` |

Because sources overlap, the per-source numbers sum to more than 190,087: 142,535 + 108,174 + 7,503 = 258,212
attributions over 190,087 rows. Association **grade oracles** (state-meet cohorts, a research-phase measure, not
a census source) total 3,993 grade-bearing Co2027 rows `[research/midwest/30-cross-source-pareto.md]`, and the
research-phase Athletic.net rankings sweep measured 40,695 Co2027 in 12 states / 2,728 requests, national
142,705 in 4,233 requests `[research/midwest/04-athletic-net-rankings-discovery.md]`.

Unverified:
- The lane-side per-source yields are from separate capture windows (2026-09-19…22) than the census store (2026-09-20); no artifact reconciles a lane's sample yield with its census row count. Settling capture: a re-run of one lane inside the census's window, or a per-lane capture-window ledger.
- `athleticlive_athletes` has no retained request ledger of its own inside the census store — its cost lives in the second store (Q7). The number of distinct *meets* the 108,174 rows came from is not counted anywhere on disk; `[lane: national-aggregators]` Open Q1 (athlete-level count for AthleticLIVE) is still open. Settling capture: an AthleticLIVE harvest ledger naming the meet doc behind each athlete-plane row.
- ND and SD MileSplit yields are real (2,043 / 2,669), but `synthesis/05-compliance-and-risks.md` §6 records "ND and SD MileSplit `/teams` indexes (1 request each, HTTP 403) — recorded, never retried". The retained cache contradicts the *consequence*, not the event: `[store: var/midwest-census/http/]` holds **154 ND and 197 SD host responses, every one HTTP 200** — 153 + 196 of them `/teams/<id>-<slug>/roster` pages (one `/teams` index per state plus its rosters were all served), and the two sampled bodies carry 41 and 427 `athlete-row` blocks, each with a `column-grad-year` value (`nd.milesplit.com/teams/23515-beach-high-school/roster`, 114,699 B → 41; `sd.milesplit.com/teams/20920-aberdeen-central-high-school/roster`, 879,119 B → 427). Both the 403 note and the cache are printed; the cache is retained because it is the byte-countable artifact, and which request the 403 note refers to (the index page on a different client, later recovered) is not recorded on disk. Settling capture: the lane's request log for that 403, or the capture directory of the ND/SD `/teams` fetch.

## 2. How many unique athletes remain after deterministic reconciliation

**Answer: 190,087 Class-of-2027 athlete records (All sources) / 146,858 (Core) — but only 83,617 are unique by
Athletic.net id and 141,036 by MileSplit id, so the reconciled row count is not a unique-person count.**

`reports/report.json` `totals.class_of_2027` = 190,087; `reports/report-core.json` = 146,858;
`data/canonical-athletes-co2027.csv` = 183,875 rows = 183,875 distinct `athlete_id`. All grades: 787,584 rows.
The applied row key is **not** a vendor id — that is measurable rather than quoted: if Athletic.net `AthleteID`
were the primary key, no id could sit on two rows, yet 10,131 of them do (table below). What the corpus does
document is the join vocabulary between the two id spaces: `[lane: athleticnet §19]` gives the outbound keys
(`MeetID`/`LiveID`, `TeamID`/`IDSchool`, `AthleteID`/`Handle`, `shortCode`) and records the pairing artifact —
**69,819** rows of `data/athleticnet-athlete-seeds.csv` and **56,842** rows of
`data/canonical-athletes-co2027.csv` carry *both* an Athletic.net and a MileSplit id (both re-verified from the
CSVs here), with the same lane noting "name+school is not unique in Athletic.net (`synonyms[]` exists precisely
because athletes get renamed/merged)". Id reuse measured on the 190,087 Co2027 rows
(`[store: var/midwest-census/out/athletes.jsonl]`, `source_identities[].id`):

| namespace | distinct ids | athlete rows holding an id | ids attached to >1 canonical athlete | extra rows | multiplicity histogram |
|---|---:|---:|---:|---:|---|
| `legacy_athletic_net` | **83,617** | 95,236 | **10,131** | 11,619 | 2→8,888 · 3→1,047 · 4→159 · 5→27 · 6→9 · 8→1 |
| `milesplit_athlete` | **141,036** | 142,970 | **1,868** | 1,934 | 2→1,805 · 3→60 · 4→3 |

Consequences that must travel with any "unique athletes" figure:

1. `report.json` `providers.athletic_net_urls_known` = 93,619 counts athletes whose `public_profile_urls` contains `athletic.net`; the store holds **95,236** athletes with a `legacy_athletic_net` identity entry, so 1,617 more athletes carry an Athletic.net id than carry a composable Athletic.net URL. Two measures, both printed.
2. **10,131 Athletic.net athlete ids are each attached to two or more canonical Co2027 athletes** (worst case id `29753305` on 8). Under the documented dedupe priority (Athletic.net `AthleteID` first) those rows are duplicates; the store did not apply that key.
3. The research phase's complement model — "AthleticLIVE and IHSA add zero new athletes because every row carries an Athletic.net id" `[research/midwest/30-cross-source-pareto.md]` — still holds for **identity, not for rows**: 43,229 Co2027 rows rest on AthleticLIVE evidence alone (Q4) and bring grade + school without a second witness.

Unverified:
- Whether the 10,131 shared Athletic.net ids are one person (duplicate records) or several people (a shared account, a merged/renamed athlete, or a school-team placeholder) is not decidable from the store: `athletes.jsonl` keeps `known_names` but no merge/alias provenance. A capture of `AthleteBio/GetAthleteBioData` for 20 of the shared ids against their two school/name variants would settle it (`[lane: athleticnet §13]` shows the endpoint answers anonymously with the full career + school).
- The core-scope drop is stated twice and differently (`145,323` in `reports/report-core.json` `notes[]` vs
  **135,848** re-derived from the store). The 09-21 workbook's `Summary` sheet would show which the CLI printed;
  that sheet was not parsed here. Settling capture: parsing that sheet, or the CLI's 09-21 consolidate stdout.

## 3. What percentage have Athletic.net profiles

**Answer: 49.3 % — 93,619 of 190,087 Co2027 rows carry a composable Athletic.net profile URL
(63.7 % of the Core scope's 146,858). Any *public* profile URL (MileSplit or Athletic.net) covers 93.1 %.**

| measure | All sources | Core | 2026-09-20 export |
|---|---:|---:|---:|
| Co2027 with an `athletic.net` profile URL | 93,619 / 190,087 = **49.3 %** | 59,650 / 146,858 = **40.6 %** | 93,619 / 183,875 = 50.9 % |
| Co2027 with any public profile URL | 176,885 = 93.1 % | 142,916 = 97.3 % | 174,600 = 95.0 % |
| distinct Athletic.net ids behind those rows | 83,617 (Q2) | not counted | not counted |

Sources: `reports/report.json` → `totals.class_of_2027_with_profile_url` + `providers.athletic_net_urls_known`;
`reports/census-by-state-core.csv`; `data/canonical-athletes-co2027.csv` (`athleticnet_athlete_id` non-empty =
93,619). Per state (All sources):

| state | Co2027 | with AN profile URL | share | any profile URL | share |
|---|---:|---:|---:|---:|---:|
| OH | 31,708 | — (not reported per state) | — | 29,992 | 94.6 % |
| IL | 26,488 | — | — | 26,002 | 98.2 % |
| WI | 22,074 | — | — | 17,921 | 81.2 % |
| MI | 22,073 | — | — | 21,967 | 99.5 % |
| IN | 16,931 | — | — | 14,758 | 87.2 % |
| MO | 15,168 | — | — | 15,121 | 99.7 % |
| MN | 14,420 | — | — | 14,345 | 99.5 % |
| IA | 14,076 | — | — | 11,347 | 80.6 % |
| KS | 10,077 | — | — | 9,888 | 98.1 % |
| NE | 9,200 | — | — | 7,868 | 85.5 % |
| SD | 4,868 | — | — | 4,673 | 96.0 % |
| ND | 3,004 | — | — | 3,003 | 100.0 % |

The Athletic.net id is **published by AthleticLIVE's `a.ani` field**, not fetched from Athletic.net:
`data/athleticnet-athlete-seeds.csv` = 189,705 rows / **167,277 distinct ids**, every row
`derived_from_sources` ⊇ `athleticlive_athletes` (measured; the `milesplit_*` values on 69,819 rows are the
*paired* MileSplit id, not a second publisher of the Athletic.net id). `[lane: athleticnet §21.1]` states the
same artifact as the handoff.

Unverified:
- The report publishes `athletic_net_urls_known` only at total level, not per state, so the per-state Athletic.net share in the table above is genuinely absent from the corpus — the capture that would settle it is a per-state breakdown in `reports/report.json` (`providers` has no state dimension) or a query over `var/midwest-census/out/athletes.jsonl` joined to `schools.jsonl` (this file answered the *other* questions that way but did not re-run the Athletic.net-URL join per state).
- Whether an Athletic.net profile URL *resolves* was never tested: the census made zero Athletic.net requests
  (`README.md` compliance section) and `[lane: athleticnet §18]` records that Cloudflare 403s plain curl. Settling capture: a browser-rendered profile fetch for a sample (a curl test is refused at the edge and is not evidence about the 93,619 URLs).
- 1,617 athletes carry an Athletic.net id with no composable URL (Q2) — not resolvable from the artifacts on disk. Settling capture: the same store join without the `public_profile_urls` filter.

## 4. What percentage have an independently corroborating source

**Answer: 31.2 % (59,269 of 190,087) carry both an Athletic.net id and a MileSplit id; 77.3 % (146,858) carry at
least one evidence source other than AthleticLIVE — i.e. something other than Athletic.net's own live-results
platform, which is exactly the Core scope; and 22.7 % (43,229) rest on AthleticLIVE alone with no second witness.**

The five definitions, all measured on `[store: var/midwest-census/out/athletes.jsonl]`:

| definition | count | share | what it means |
|---|---:|---:|---|
| two identity **namespaces** (Athletic.net id AND MileSplit id) | 59,269 | 31.2 % | `report.json` `class_of_2027_multisource`; `data/canonical-athletes-co2027.csv` gives the same 56,842 on the 183,875-row export |
| ≥ 2 distinct **evidence sources** | 66,149 | 34.8 % | source sets of size 1→123,938 · 2→64,173 · 3→1,976 |
| **no** AthleticLIVE evidence at all | 81,913 | 43.1 % | MileSplit or WIAA only |
| ≥ 1 evidence source **other than `athleticlive_athletes`** | **146,858** | **77.3 %** | identical to the Core-scope total, which is why `reports/report-core.json` needs no separate athlete count |
| evidence from an **association artifact** (`wiaa_results`) | 7,503 | 3.9 % | 3,938 WIAA-only · 1,204 WIAA+MileSplit · 1,976 WIAA+MileSplit+AthleticLIVE · 385 WIAA+AthleticLIVE |
| `athleticlive_athletes` **only** | 43,229 | 22.7 % | no independent corroboration; this is exactly the Core-scope drop. Robust across definitions: the AthleticLIVE *family* (`athleticlive_athletes` ∪ `athleticlive_meets_csv`) covers the same 43,229 rows, because all 9,667 `athleticlive_meets_csv` observations sit on rows that already carry `athleticlive_athletes` |

The independence rule comes from the lanes, not from taste: `[lane: state-assoc-plains §2.1]` records
AthleticLIVE as "a **white-label of the same pipeline as Athletic.net** → excellent for coverage/joins, **not
independent corroboration**", and `[lane: national-aggregators §3.21]` reaches the same conclusion. Therefore an
Athletic.net id republished by AthleticLIVE is *identity linkage*, not corroboration.

Corroboration is very uneven by state (All sources; `reports/census-by-state.csv` `co2027_multisource`):

| state | Co2027 | multi-namespace | share | AL-only | AL-only share |
|---|---:|---:|---:|---:|---:|
| OH | 31,708 | 9,891 | 31.2 % | 4,710 | 14.9 % |
| IL | 26,488 | 10,740 | 40.5 % | 7,096 | 26.8 % |
| WI | 22,074 | 5,185 | 23.5 % | 2,793 | 12.7 % |
| MI | 22,073 | 9,148 | 41.4 % | 5,897 | 26.7 % |
| IN | 16,931 | 3,188 | 18.8 % | 5,022 | 29.7 % |
| MO | 15,168 | 5,552 | 36.6 % | 1,950 | 12.9 % |
| MN | 14,420 | 5,315 | 36.9 % | 3,159 | 21.9 % |
| IA | 14,076 | 2,820 | 20.0 % | 5,338 | 37.9 % |
| KS | 10,077 | 1,937 | 19.2 % | 825 | 8.2 % |
| NE | 9,200 | 3,066 | 33.3 % | 3,279 | 35.6 % |
| SD | 4,868 | 1,442 | 29.6 % | 2,199 | 45.2 % |
| ND | 3,004 | 985 | 32.8 % | 961 | 32.0 % |

For contrast, the research phase's only estimate was 3,993 grade-corroborated rows = **9.8 %** of a 40,695
corpus `[research/midwest/30-cross-source-pareto.md]`. That estimate is superseded: it counted state-meet grade
oracles as *rows*, while the measurement above counts athletes with two `source_identities` namespaces. Both are
printed; the 31.2 % is the retained number.

Unverified:
- The 59,269 cross-namespace pairs are **generated by the pairer, not published** by either site: `[lane: athleticnet §19]` shows MileSplit exposes no Athletic.net id (0 files in `samples/` contain the string) and `[lane: milesplit-national §19]` confirms it, so the pairing is tuple-based. Per-pair precision is not measured anywhere on disk. Settling capture: `data/athleticnet-athlete-seeds.csv` `identity_confidence` distribution joined back to the 287-athlete MileSplit audit `[research/midwest/34-milesplit-discovery-vs-validation.md]`.
- No artifact measures whether the two evidences describe the *same season*. `athletes.jsonl` `observed_grades` carries `school_year` per observation but no reconciliation asserts year equality. Settling capture: a per-observation `school_year` equality assertion in the reconciler.
- `report-core.json` reports `multisource_athletes: 0` because the Core scope drops the AthleticLIVE rows and only kept one namespace; a Core-scope corroboration rate is therefore not computable from `reports/`. Settling capture: none — the count is 0 by construction until the core filter keeps both namespaces.

## 5. What percentage have a school with an identified track/XC coach

**Answer (measured 2026-09-22, after the second coach wave): 30.0 % (57,115 of 190,087) in the All-sources
scope; 34.3 % in Core — up from 22.7 % / 25.3 % on 2026-09-21.** The predicate is exact: the
athlete's school has at least one coach whose sport is track or cross-country — `[repo:
crates/midwest-census/src/report/rows.rs:97-112]` (`school_coach_index` skips `!coach.sport.is_some_and(is_track_or_xc)`
at :103 and records `professional_email.is_some()` at :107; the athlete-side bump is `:174`).

| scope / artifact | Co2027 with an identified TF/XC coach | share |
|---|---:|---:|
| `reports/census-by-state.csv`, 12-state filter (All sources, 2026-09-22) | **57,115** | **30.0 %** |
| `reports/census-by-state-core.csv`, 12-state filter (Core, 2026-09-22) | 50,318 | 34.3 % |
| `reports/report.json` totals (national store scope, All sources, 2026-09-22) | 82,737 | 13.2 % |
| `reports/report-core.json` totals (national store scope, Core, 2026-09-22) | 75,940 | 13.0 % |
| same measurement on 2026-09-21 (12-state filter) | 43,201 | 22.7 % |
| `data/recruiting-co2027.csv` (2026-09-20 export) | 22,183 | 12.1 % |

**Scope note:** `reports/report.json` covered only the 12 Midwest states on 2026-09-21; the store has since
grown to 51 states with nonzero Co2027 (the extra coach research lanes covered 23 states, and the census store
now holds the whole country), so its `totals` block reports the national store scope (625,899 Co2027) and the
Midwest answer must be taken from the 12-state filter of `census-by-state*.csv`. The two artifacts agree
row-for-row on all 12 states (checked: `report.json.by_state` vs `census-by-state.csv`).

The 2026-09-22 gain is the coach wave described in `README.md` §Wave 6/7/8: `data/coach-contacts.csv` (now a
**verified** rebuild — 6,215 rows over 1,634 schools, carried into the store as 2,194 merges / 1,244 net new
coach rows across 1,190 schools, store coaches 29,294 → 31,488; see
`research/midwest/47-coach-fragment-provenance-audit.md`), 758 re-verified athletic-director rows, and a
Wisconsin rebuild straight from the WIAA `CoachList` table (WI coach rows 2,719 → 3,963, WI rows carrying an
email 2,302 → 3,242 — the entire 12-state email gain). The states that read exactly 0 on 2026-09-21 now
carry rows — MI 112, IN 130, MO 58, KS 565, IA 145, SD 25, ND 1,103 — and OH moved 28 → 210 rows.

Per state (`reports/census-by-state.csv`; `coach rows` = coach rows the store holds for that state, cross-checked against `report.json.by_state`, which agrees exactly on all 12):

| state | Co2027 | with coach | share | coach rows | rows with email |
|---|---:|---:|---:|---:|---:|
| MN | 14,420 | 11,147 | 77.3 % | 6,731 | 2,386 |
| WI | 22,074 | 16,561 | 75.0 % | 3,963 | 3,242 |
| NE | 9,200 | 6,396 | 69.5 % | 1,824 | 59 |
| ND | 3,004 | 1,909 | 63.5 % | 1,103 | 12 |
| IL | 26,488 | 8,170 | 30.8 % | 15,102 | 3,235 |
| IN | 16,931 | 2,980 | 17.6 % | 130 | 120 |
| OH | 31,708 | 4,310 | 13.6 % | 210 | 165 |
| IA | 14,076 | 1,785 | 12.7 % | 145 | 75 |
| KS | 10,077 | 946 | 9.4 % | 565 | 548 |
| MO | 15,168 | 1,097 | 7.2 % | 58 | 45 |
| MI | 22,073 | 1,531 | 6.9 % | 112 | 78 |
| SD | 4,868 | 283 | 5.8 % | 25 | 5 |
| **TOTAL** | **190,087** | **57,115** | **30.0 %** | 29,968 | 9,970 |

*2026-09-21 state, superseded by the import above; the per-state explanations still describe what each
directory publishes:* the zero-coach states are explained, not estimated: KS's 526 rows are **AD-only** (`[lane:
coach-directories-national §KS]` "the registry is school-level, not sport-level"; "not a coach source"), MI's 6
rows arrive through the `mhsaa` host branch of the coach-CSV importer (`[lane: state-assoc-greatlakes §7.4]`),
and IN/MO publish no coach directory at all (`[lane: coach-directories-national §IN]` "no per-school contact
rows"; `§MO` host-wide robots disallow).

The export/census split is a **different join, and it is explained**: `data/recruiting-co2027.csv` populates only four head-coach columns from the research contact graph — 22,183 rows with a head track or head XC coach name (IL 125 · MN 2,463 · NE 6,059 · WI 12,766 · OH 456 · SD 209 · IA 105) and 15,338 with an email on one of them (WI 12,766 · MN 1,924 · OH 456 · IL 125 · SD 46 · IA 21; verified predicate: either `head_*_coach_email` non-empty) — while the census's predicate is *any* track/XC-sport coach row at the school, which adds the IHSA staff adapter (15,114 observations, 15,102 IL coach rows) and the MSHSL team-coach endpoint (5,843 observations). Reproduce the census side from `report.json` `coach_sources` = `{association_school:ihsa 15,114 · mshsl_team_coach 5,843 · association_school:nsaa 1,528 · association_school:mshsl 760 · association_school:kshsaa 526 · bound 78 · association_school:ohsaa 33 · association_school:wiaa 22 · association_school:ndhsaa 13 · association_school:mhsaa 6}` = 23,923 observations of the 27,580 coach rows (a 3,657-row attribution gap, printed as-is; `coach_sports` covers all 27,580 rows).

The ceiling is set by the directories: `[lane: coach-directories-national §Headline]` measured **51/51 jurisdictions covered, 22 tier-1 directories fetched and parsed, 8 jurisdictions naming XC/TF coaches at tier 1, and only 3 publishing a coach EMAIL at tier 1**.

Unverified:
- `report.json` `coach_sources` sums to **31,697 attributions** over **31,488** coach rows (2026-09-22, post-import); the 209-row surplus is `[INFERENCE]` double attribution (209 rows carrying two observations), not row inflation — `[store: var/midwest-census/out/coaches.jsonl]` holds 31,488 lines and the report's `by_state` sums match its `totals` exactly (31,488 coaches, 82,737 Co2027 with a coach). The earlier 09:42 run showed the opposite sign (histogram 28,532 < total 30,243) because that export predated the 09:36 import by 36 minutes; both runs' attributions are printed, never reconciled silently.
- Whether a TF/XC coach row is *current* is not modelled: `[lane: coach-directories-national]` records "current
  only" for each directory, and no observed-on older than 2026-09-20 exists in the store. Settling capture: a second crawl window to diff against.

## 6. What percentage have a public professional coach contact

**Answer (measured 2026-09-22, after the second coach wave): 24.3 % (46,185 of 190,087) in the All-sources
scope; 27.9 % (41,026 of 146,858) in Core — up from 17.5 % / 19.7 % on 2026-09-21. The 12-state store now holds
9,970 coach rows carrying an email out of 29,968 coach rows (33.3 %); the 52-jurisdiction store scope is 10,671
of 31,488 (33.9 %).** Twelve states now contribute emails (the 2026-09-21 top four were IL 3,235, MN 2,333, WI
2,155, KS 523); the same scope note as Q5 applies — `reports/report.json` totals are national.

| scope / artifact | Co2027 with a professional coach email | share |
|---|---:|---:|
| `reports/census-by-state.csv`, 12-state filter (All sources, 2026-09-22) | **46,185** | **24.3 %** |
| `reports/census-by-state-core.csv`, 12-state filter (Core, 2026-09-22) | 41,026 | 27.9 % |
| `reports/report.json` totals (52 jurisdictions, All sources, 2026-09-22) | 58,260 | 9.3 % |
| `reports/report-core.json` totals (52 jurisdictions, Core, 2026-09-22) | 53,101 | 9.1 % |
| same measurement on 2026-09-21 (12-state filter) | 33,340 | 17.5 % |
| `data/recruiting-co2027.csv` (head TF/XC coach email union, 2026-09-22 export) | 56,220 | 9.0 % |
| `data/coach-contacts.csv` (verified research contact graph, 2026-09-22: 6,215 rows / 1,634 schools / 23 states) | 3,597 rows carry `public_professional_email` | — |

Per state (`reports/census-by-state.csv` `co2027_with_coach_email`; `report.json` `coaches_with_email`):

| state | Co2027 | with coach email | share | coach rows with email | email source measured by the lane |
|---|---:|---:|---:|---:|---|
| MN | 14,420 | 10,326 | 71.6 % | 2,386 | MSHSL `/api/coaches/<nid>` plus the `mshsl_team_coach` store adapter (303 fragment emails), domain-checked `[lane: coach-directories-national §MN]`, `[lane: state-assoc-plains §5]` |
| WI | 22,074 | 16,218 | 73.5 % | 3,242 | `schools.wiaawi.org` `GetDirectorySchool?OrgID=` coach table (1,429 fragment emails) plus the 2026-09-22 `CoachList` rebuild, CF-obfuscated + decodable `[lane: coach-directories-national §WI]` |
| IL | 26,488 | 7,958 | 30.0 % | 3,235 | `api.ihsa.org` `/v1/schools/<id>/staff2` + `/staff/<PersonID>/email` (2 requests per person) `[lane: coach-directories-national §IL]` |
| IN | 16,931 | 2,800 | 16.5 % | 120 | new 2026-09-22: school athletics sites; 39 of 94 fragment emails sit on Eventlink team pages (`sites.eventlink.com`) |
| OH | 31,708 | 3,997 | 12.6 % | 165 | `officials.myohsaa.org` `SportsInformation` + `AthleticDirector` (77 fragment emails) plus school athletics sites `[lane: coach-directories-national §OH]` |
| IA | 14,076 | 1,402 | 10.0 % | 75 | new 2026-09-22: school athletics sites (Ankeny `jaguars/hawks.ankenyschools.org`, `kennedy.crschools.us`) |
| NE | 9,200 | 663 | 7.2 % | 59 | NSAA directory-export names + Google Sites / `rschoolteams` team pages (53 emails); **only 59 of the 209 NE fragment rows re-verify against their cited page** (see Unverified) |
| KS | 10,077 | 636 | 6.3 % | 548 | KSHSAA AD emails (526 slice rows — school-level, not sport-level `[lane: coach-directories-national §KS]`) plus school sites (23) |
| MO | 15,168 | 918 | 6.1 % | 45 | new 2026-09-22: school athletics sites (`athletics.kirkwoodschools.org`, `lhsathletics.lps53.org`, `lshs.lsr7.org`) |
| MI | 22,073 | 1,188 | 5.4 % | 78 | new 2026-09-22: school athletics sites (51 emails) plus AD endpoint rows imported via the `mhsaa` host branch; the live directory is robots-disallowed `[lane: state-assoc-greatlakes §7.4]` |
| SD | 4,868 | 79 | 1.6 % | 5 | `data/coach-contacts.csv` rows from the pre-retraction Bound sample (4 with a coach email, all Rapid City Stevens, `www.gobound.com`) `[lane: coach-directories-national §Baseline]` |
| ND | 3,004 | 0 | 0.0 % | 12 | NDHSAA coach names (187 fragment rows on `ndhsaa.com`), 0 emails published `[lane: coach-directories-national §ND]` |
| **TOTAL** | **190,087** | **46,185** | **24.3 %** | 9,970 | |

Cross-check of the research contact graph, recounted from the CSV (quote-safe `csv.DictReader`, which `awk -F,` gets wrong — `[lane: state-assoc-greatlakes §7.4]` records the same 626-vs-620 slip): on 2026-09-22, **after the fragment union** (`out/coach-freeze-rust.log`: 33 fragment files / 15,072 verified rows → 14,275 shipped, 9,086 distinct identities, 6,215 merged rows, **0 merged rows with no verified counterpart**), `data/coach-contacts.csv` = **6,215 rows / 1,634 schools / 22 states**, 5,157 rows with a coach name (**3,597** with `public_professional_email`), 2,891 rows with an AD name (2,256 with `ad_email`). The same artifact read **3,444 / 1,180 / 23 states**, 2,348 (1,407) and 2,929 (2,317) before the union — the figures this file quoted earlier today; the pre-gate merge figure was 4,063 / 1,216 / 23 states, 2,891 (1,513) and 3,522 (2,665); before that 2,298 / 874 / 10 states, 1,333 (51) and 2,207 (620).
Tier-1 association ceiling `[lane: coach-directories-national §Headline]`: **3 of 51 jurisdictions publish a coach email at tier 1 (IL, OH, WI)**; 14 publish an AD name and 5 an AD email — which is why the 2026-09-22 gain comes from tier-2 school-athletics sites rather than from the associations.

Unverified:
- **NE fragment provenance (settled 2026-09-22, was open):** of 209 NE fragment rows, the verifier accepts
  **86** (70 `ok` + 16 role-context) and drops **123** — 92 `role_contradicted` (the cited page prints a
  different role next to the matched value) and 31 `render_required` (the NSAA export screen renders its rows
  client-side, so the fetched body carries no names). The pre-gate merge shipped those rows; the verified
  rebuild no longer contains them, so NE coach *names* in `data/coach-contacts.csv` are now verified-or-absent.
  IL, by contrast, re-verified 197/197 once both URLs in its two-URL cells were fetched (27 of the 197 are
  `role_contradicted` and dropped by the role rule).
- Which exact predicate produced `report.json` `coaches_with_email` for MI's AD rows (465 KB CSV import vs the
  `mhsaa` host branch) is not restated in any report; `[lane: state-assoc-greatlakes §7.4]` calls MI's first 6 rows "a
  *pipeline* fact, not a research estimate". MI now carries 112 rows / 78 emails, of which 51 emails come from the
  new school-site fragments. Settling capture: the coach-import log for the 09-22 run.
  This file treats the figures as *coach-role* emails because `[repo: crates/midwest-census/src/report/rows.rs:103,107,174]`
  requires `professional_email.is_some()` on a track/XC coach specifically.
- The `professional_email` withholding rule (consumer mailbox domains dropped) publishes its count only as
  `coaches_email_withheld` on the **consolidate step's stdout** (`[repo: crates/midwest-census/src/census/aggregate.rs:90]`,
  consumed by `tools/run_pipeline.sh:113` "the counts below report what it withheld"), and that stdout was not
  captured anywhere in the corpus — `README.md` points at `synthesis/05-compliance-and-risks.md` §2 and §6, but
  §2 lists the *exclusions* (MSHSL `Non-MSHSL Coach`/`MSHSL Sub-Coach` levels, KSHSAA `PrincipalCell/ADCell/PresCell`,
  Bound staff, DAT meet-director phone/fax, athlete height/weight) without a number, and §6's ledger has no such
  row. What can be measured: the store's `email_withheld` flag is **0** on every row of both
  `[store: var/midwest-census/out/coaches.jsonl]` (29,294 rows, 9,187 with a professional email; export written
  09:00, 36 min before the 09:36 import) and `[store: var/midwest-census/entities/coaches.jsonl]` (27,580 rows,
  8,284 with one; export written 07:38), so the 903-email difference between the two exports is the morning
  merge/dedupe step, not a visible withholding event — and neither export is the store state that `report`
  read at 09:42 (30,243 rows / 9,730 emails). Settling capture: the 09-22 consolidate log.
- No artifact measures the **bounce/validity** of any address; every figure above counts a published string. Settling capture: none on disk — validity needs a probe that sends mail, which this study did not do.

## 7. How many Athletic.net requests are avoided through external discovery

**Answer: 163,902–169,446 Athletic.net requests avoided, against measured external spend of 18,424 requests —
arithmetic below; the census itself made exactly 0 Athletic.net requests.**

### 7.1 The baseline: Athletic.net-only, per the lane's own request model

Units from `[lane: athleticnet §16]` (every row re-measured from captured payloads), with the citation for the
two corpus-sourced rows:

| baseline component | measured unit | source |
|---|---:|---|
| 12-state rankings discovery sweep (2026 outdoor boys, grade 11) | **2,728 requests → 40,695 Co2027** (0.067 req/athlete) | `[research/midwest/04-athletic-net-rankings-discovery.md]` table 1, model validated within 0.6 % of 4,256 retained receipts |
| same sweep, national scope | 4,233 requests (recorded: 4,256) → 142,705 athletes | same, table 2 |
| four-way 12-state scope (2 genders × outdoor+XC) | **5,456 measured floor .. ≈ 11,000** | `[research/midwest/04-…]` "all four, 12-state scope" row |
| per-athlete career/profile fetch | **1 request per athlete** (`AthleteBio/GetAthleteBioData`; 586 requests for meet 634313's 586 athletes) | `[lane: athleticnet §16]` |
| production profile plan in the repo | **3 resources per athlete** | `[research/midwest/02-athletic-net-profile-acquisition.md]` |
| whole meet (results) | **3 requests** (settled to **2** — `GetEventDivisionData` is needed only for per-event metadata) | `[lane: athleticnet §16, §22.9]` |
| rankings page (single event) | 102 rows/request; multi-event page server-capped at 180 rows | `[lane: athleticnet §7,§16]` |

### 7.2 The external side: measured request spend, from the response caches

Two stores, both on disk, both countable by `http/*.meta.json` (the cache key includes the request body, so
613 distinct bodies under one URL are 613 requests — verified: the second store has 1 distinct URL, 613 metas and
613 distinct `(bytes, sha256)` pairs, i.e. no two identical responses):

| store | responses | hosts | Athletic.net responses |
|---|---:|---:|---:|
| `[store: var/midwest-census/http/]` (census run, 2026-09-20T14:00–18:59Z) | **17,811** (17,808 distinct URLs; 200→17,720, 404→91) | 21 | **0** |
| `[store: var/midwest-athletes/http/]` (AthleticLIVE athlete harvest, 2026-09-20T14:xxZ) | **613** (all `POST search.athletic.live/athlete_list/_search`) | 1 (`search.athletic.live`, already in the census store) | **0** |
| **total** | **18,424** | 21 distinct | **0** |

Per family inside the census store: `api.ihsa.org` 3,682 · `www.wiaawi.org` 3,609 (91 of them 404) ·
`www.mshsl.org` 2,732 · twelve MileSplit hosts **6,643** (`oh` 977 · `mi` 896 · `il` 850 · `mo` 700 · `wi` 598 ·
`mn` 593 · `in` 535 · `ks` 414 · `ia` 412 · `ne` 317 · `sd` 197 · `nd` 154) · `schools.wiaawi.org` 655 ·
`secure.nsaahome.org` 313 · `ndhsaa.com` 170 · `search.athletic.live` 4 · `www.wayzataresults.com` 2 ·
`kshsaa-api.kshsaa.org` 1.

Three published request counts exist for this run and are **not** averaged: `synthesis/05-compliance-and-risks.md` §6 "Cached responses" = **7,355** (7,190 × 200, 165 × 403 "before the retry sweep"; refusals purged) across 15 hosts; `README.md` = "~7,400" across "15 hosts"; **the cache on disk, recounted here** = **17,811** (200→17,720, 404→91) across **21** hosts.

The ledger and the cache agree on the run window (all metas fall in 2026-09-20T14–18Z, the same five hours) but
not on volume; the extra hostnames are the gap-closure and retry-sweep families (both ND/SD MileSplit hosts,
`secure.nsaahome.org`, `ndhsaa.com`, `www.wayzataresults.com`). The on-disk cache is retained as the external
spend because it is byte-countable and carries no "purged" step. Both numbers are printed.

### 7.3 The subtraction

| product | Athletic.net-only cost | external cost | avoided |
|---|---:|---:|---:|
| Athletic.net athlete profile ids: 167,277 distinct ids (189,705 seed rows) | 167,277 × 1 `GetAthleteBioData` = **167,277** (or 3× = 501,831 under the repo's 3-resource plan) | **613** `athlete_list/_search` requests | **166,664** (1-Bio) / **501,218** (3-resource) |
| Athletic.net meet ids: 9,593 canonical meets carry one (`data/canonical-meets.csv`), 9,747 of 9,844 seed rows carry one (`data/athleticnet-meet-seeds.csv`) | 9,593 meet lookups (search is robots-disallowed to the lane, so an ATN-only route needs a calendar/division walk per meet) | included in the 17,811 census responses | **≈ 9,593** |
| Co2027 identity + grade for 190,087 rows | four-way 12-state discovery **5,456–11,000** | MileSplit roster enumeration **6,643** (the WIAA 4,264 and AthleticLIVE meet harvest add grade + corroboration on top and are charged in the total line, not here) | **−1,187 … +4,357** — MileSplit roster enumeration is *not* cheaper than the rankings sweep for pure discovery; it buys grad year and a second namespace, not request savings |
| coach/AD contacts: 27,580 coach rows, 8,284 emails | **not substitutable — Athletic.net publishes no contact field at all** (`[lane: athleticnet §12]`: "None. No staff, coach, AD, email or phone field exists anywhere in the captured payloads"; `[lane: milesplit-national §12]` agrees for MileSplit) | 11,162 association-directory responses (IHSA 3,682 + WIAA 4,264 + MSHSL 2,732 + NSAA 313 + NDHSAA 170 + KSHSAA 1) | undefined (∞) — a new capability, not a substitution |
| **total profile/meet substitution** | 182,326 … 187,870 | 18,424 | **163,902 … 169,446** |

The total is **conservative by construction**: the full external spend (18,424) is charged against only the profile and meet baselines, while the MileSplit and association-directory responses are charged without claiming their own credit (row 4 is a capability the Athletic.net-only baseline cannot produce at any price). Charging only the row-1 external cost against the row-1 baseline gives the single cleanest exchange in the study: **613 requests for 167,277 Athletic.net athlete ids.**

Sanity check against the research phase: it modelled 43,423 requests (2,728 discovery + 40,695 profiles) for the 12-state boys-outdoor cohort alone, or 124,813 under the 3-resource plan, and measured the *state-meet oracle set's* replacement cost as ~3,920–3,960 profile-class requests plus ~190 meet lookups `[research/midwest/30-cross-source-pareto.md]`.
Those ~4,110 requests cover one cohort's state meets, not the census's whole external spend; the 18,424 above covers the larger corpus (190,087 Co2027 rows, 167,277 distinct Athletic.net ids, 12 states of rosters and 27,580 coach rows). Both numbers are printed; they are different scopes, and the census's is the retained one for Q7.

Unverified:
- The **AthleticLIVE state-meet substitution units** of the research phase (236 profile-class requests avoided
  per state XC meet `[research/midwest/10-minnesota-results-timers.md]`; ≈1,261 for IA state T&F
  `[research/midwest/12-iowa-wayzata-results.md]`) are per-meet and were never summed; the census side has no
  per-meet request attribution, so the regular-season meet substitution is **not** in the 163,902–169,446 range.
  Settling capture: a per-meet request ledger from the AthleticLIVE meet harvest (the journal
  `[store: var/midwest-census/journal/athleticlive_meets.jsonl]`, 1,009 KB, is the input list, not a ledger).
- The 613-request AthleticLIVE athlete harvest has no retained per-state/page ledger; whether 613 requests were
  the whole harvest or a resumed slice is not decidable from the cache (all 613 are timestamped in one hour). Settling capture: a per-page cursor ledger from that harvest.
- The 17,811-response count includes 91 HTTP 404s on `www.wiaawi.org`; whether the completed run needed them or
  they are retry residue is not recorded. Settling capture: the WIAA retry-sweep log (not retained).

## 8. Which states/providers account for the remaining coverage gaps

**Answer: coach contact (MI, IN, MO publish nothing; KS/MI/NE/ND email-free) is the largest measured gap,
followed by Athletic.net-url depth (WI 81.2 %, IA 80.6 %, NE 85.5 % of Co2027) and by the 43,229 Co2027 rows
that rest on AthleticLIVE alone.**

Ranked by measured size:

| # | gap | measured size | states / providers responsible | citation |
|---|---|---|---|---|
| 1 | no coach-role data at all | **MI 22,073 + IN 16,931 + MO 15,168 + KS 10,077 = 64,249 Co2027 (33.8 %)** have `with_coach` = 0 (the research-phase estimate over its 40,695-athlete corpus was "6,302 athletes / 15.5 % sit in coach-less states" — MI+IN+MO only; superseded by the measured 64,249, which adds KS and the larger corpus) | MHSAA (robots `/DesktopModules/`), IHSAA/IN (no directory), MSHSAA (host-wide robots disallow), KSHSAA (AD-only) | `reports/census-by-state.csv`, `[lane: coach-directories-national §MI,§IN,§MO,§KS]`, `[lane: state-assoc-plains §0.3]`, `[research/midwest/30-cross-source-pareto.md]` §Q8 |
| 2 | coach names but **zero emails** | NE 9,200 + ND 3,004 = **12,204 (6.4 %)** | NSAA publishes no email field; NDHSAA names only | `reports/census-by-state.csv`, `[lane: coach-directories-national §NE,§ND]` |
| 3 | no independent corroboration | **43,229 rows (22.7 %)** AthleticLIVE-only; worst SD 45.2 %, IA 37.9 %, NE 35.6 %, ND 32.0 % | measured from the 2026 meet inventory: SD `live_results` 372/`dakota` 283/`athleticlive` 61 of 753 · IA `live_results` 1,007/`aatiming` 297/`athleticlive` 160/`dakota` 120/`wayzata` 119 of 2,015 · NE `live_results` 591/`athleticlive` 307/`blacksquirrel` 184 of 1,198 · ND `live_results` 162/`athleticlive` 110/`heros` 52 of 324 | Q4 table, `[data/athleticlive-midwest-2026-meets-all.csv]`, `[store: var/midwest-census/out/athletes.jsonl]` |
| 4 | thin public-profile depth | WI 17,921/22,074 = 81.2 % · IA 11,347/14,076 = 80.6 % · NE 7,868/9,200 = 85.5 % · IN 14,758/16,931 = 87.2 % | MileSplit `wi`/`ia`/`ne`/`in` roster coverage; ND/SD/KS/MO ≥ 96 % | `reports/census-by-state.csv` |
| 5 | no per-state Athletic.net-url figure | **unknown for all 12 states** | report.json has no state dimension for `athletic_net_urls_known` | Q3 Unverified |
| 6 | meets without an Athletic.net id | 11,007 − 9,593 = **1,414** (1,532 core meets carry 0) | the timer indexes that publish no `ani` | `reports/report.json` `meets` |
| 7 | meet venues that never resolved to a state | **442 of 11,007 (4.0 %)** filed under `??` (09-20 core scope: 537 Wayzata competition rows minted 536 meets and only 304 resolved to a state — 95 recurring sites, 209 schools; the two scopes are different runs and are not reconciled) | venue strings that match no state (Wayzata schedule rows and other timer calendars) | `reports/report.json` `meets.by_state`, `reports/midwest-census-2026-09-20-method-notes.csv` ("Unresolved venues", "Wayzata schedules") |
| 8 | unresolved identity rows in the reference corpus | **15,724** roster results with no athlete identity | AthleticLIVE/SCOPE corpus, not the census store | `[research/midwest/04-athletic-net-rankings-discovery.md]` evidence appendix |
| 9 | no free state-level Co2027 denominator | **all 12 states** | no association publishes a junior count; `data/milesplit-coverage-matrix.csv` carries 3-team-sample *bands* only (e.g. OH 9,770–86,953; MI 895–895) with the file's own note "NOT a point estimate" | `data/milesplit-coverage-matrix.csv`, `[lane: state-assoc-greatlakes §0.3]` |
| 10 | jurisdictions outside the 12 that the lanes reached but the census did not | CA 12,703 · NY 9,664 · PA 5,937 · TX 3,454 meet docs (all-time AthleticLIVE) | AthleticLIVE's non-Midwest tenants | `[lane: national-aggregators §3.2]` |

Access-level dead ends that are **not** documentation gaps (each was refused, not skipped):

| provider | verdict | evidence |
|---|---|---|
| MaxPreps | REJECT — robots disallows `/school/ /team/ /discovery/ /careerprofile/ /m/team/ /m/school/` (191 Disallow rules); no grade string, no athlete ids | `[lane: national-aggregators §5, §10]` |
| AAU `aausports.org` | REJECT — 403 Cloudflare, no bypass attempted | `[lane: national-aggregators §8]` |
| MHSAA `my.mhsaa.com` | refused — `Disallow: /DesktopModules/` for `*`; 2 compliant attempts logged | `[lane: coach-directories-national §MI]` |
| MSHSAA `www.mshsaa.org` | refused — second `User-agent: *` group with `Disallow: /`; groups merge under RFC 9309; two independent evaluators agreed | `[lane: state-assoc-plains §0.3]`, `[lane: coach-directories-national §MO]` |
| gobound.com directories (IA, SD) | refused — `Disallow: /*directory`; the earlier SD/IA table claims are **retracted** | `[lane: coach-directories-national §SD]`, `tools/validation-retractions.json` |
| `live.pttiming.com` | prohibited — ToU forbids bulk extraction and AI/ML use; Claude named 4×; 50 Disallow rules | `[lane: timing-providers-national §4]` |
| RunnerSpace / DyeStat | 403 to curl, browser render succeeds, `Crawl-delay: 10` | `[lane: national-aggregators §4]` |
| World Athletics GraphQL | VALIDATION only, with a trap: public `x-api-key` needed, and `getAthlete(id: 252802)` / `getAthlete(id: 14377384)` both return the same placeholder `{"id":7911,"fullName":" ","countryCode":"FRA",…}` while `getAthlete(urlSlug:"malaika-mihambo-14377384")` returns the real athlete — an implementation keyed on numeric ids would silently import a bogus athlete | `[lane: national-aggregators §18 (numeric-id trap), §3 row 6 (auth)]` |
| `niaa.org` | not Nevada's association — it 301s to a 198 KB page titled `Alumni | INROADS` (canonical `https://inroads.org/alumni/`); Nevada's association is `niaa.com` | `[lane: state-assoc-westcoast §0 Lane-level findings]` |

Unverified:
- **ATHLETIC.NET-URL DEPTH PER STATE IS ABSENT**, not merely small: `reports/report.json` `providers` has no state
  dimension, so the per-state column in Q3's table is empty by construction. Settling capture: a `report` run
  emitting `athletic_net_urls_known` per `by_state` entry, or the equivalent join over
  `var/midwest-census/out/athletes.jsonl` × `schools.jsonl`.
- The 15,724 unresolved identity rows are quoted from the retained Athletic.net corpus, not re-measured here;
  whether any of them are in the 190,087 is not established. Settling capture: a join of those 15,724 rows' meet/team keys against the census store.
- 442 `??` venues: `reports/midwest-census-2026-09-20-method-notes.csv` files them "rather than guessed", and no
  later artifact resolved them. Settling capture: a venue-normalization pass over the 442 strings.
- Whether ND's 43.8 % `with_coach` is a documentation limit or a sample limit: `[lane: state-assoc-plains §12]`
  carries the open question "ND state outdoor T&F tenant … unproven whether 2026 state XC (Oct 23-24) lands
  there"; the census did fetch 1 request per ND school (`ndhsaa.com` 170 responses) so the remaining gap is
  distribution, not access. Settling capture: the 2026 ND state XC tenant in AthleticLIVE once the Oct 23-24 meet lands.

## 9. Which five to ten production adapters give the best coverage/engineering ratio

**Answer: the ten below, in this order, with the measured yield and the measured request cost per adapter.
Ranks 1–5 carry the census; ranks 6–10 are association/contact layers that no Athletic.net request can replace.**

| # | adapter (production module) | coverage | measured yield in the census | measured request cost | why this rank |
|---|---|---|---|---|---|
| 1 | **`sources/milesplit/`** (12 state hosts) | 12/12 states | **142,535 Co2027 rows** (75.0 %), 141,036 distinct MileSplit ids; per-state 26,998 OH … 2,043 ND | **6,643** responses (1 team list + 1 roster per team; 26,562 HS records nationally) | breadth winner: grad year is free and explicit (`column-grad-year`, 95.9 % presence), robots-allowed, no paywall, no coach fields — `[lane: milesplit-national §16,§21]` |
| 2 | **`sources/athleticlive_athletes/`** + **`sources/athleticlive/`** | 12/12 states via tenants | **108,174 Co2027 rows**; 167,277 distinct Athletic.net athlete ids; 9,593 Athletic.net meet ids | **613** `_msearch`/`_search` responses for the athlete plane (+4 in the census store) | highest ratio in the study: **0.0037 requests per athlete** (613 requests standing in for 167,277 ids), and hands over `a.ani`/`a.t.ani`/meet `ani` with grade per row `[lane: national-aggregators §3.20,§3.21]` |
| 3 | **`sources/wiaa/` + `sources/wiaa_results/`** | WI only | **7,503 Co2027 rows** (3,938 of them WIAA-only, 3,180 also on `milesplit_wi`), 13,573 grade-evidence rows | 4,264 responses (3,609 `www.wiaawi.org` + 655 `schools.wiaawi.org`) | the only association artifact that materially raises athlete coverage; also the WIAA tournament Athletic.net MeetIDs and 104 result-file links/season `[research/midwest/06-wisconsin-wiaa.md]` |
| 4 | **`sources/ihsa/`** | IL only | **15,102 coach rows, 3,235 with email**; school universe 828 re-derived from retained bytes | 3,682 responses (1/school + 1/person with `HasEmail`) | largest coach-email corpus in the Midwest and the only one with a stable `PersonID`; IL is 98.2 % profile-URL covered (Q3) — `[lane: coach-directories-national §IL]`, `[lane: state-assoc-greatlakes §0.3]` |
| 5 | **`sources/mshsl/`** | MN only | **10,726 Co2027 with a coach, 9,905 with email**; 6,596 coach rows; grade from the school, not from Athletic.net | 2,732 responses (4 team lists + 664 school pages + 1,494 coach calls) | only association found returning grade + head-coach name + professional email in one request family `[lane: coach-directories-national §MN]`, `[lane: state-assoc-plains §5]` (lane recommendation: §1) |
| 6 | **`sources/coach_contacts/`** (CSV import) | 10 states | **2,224 coach rows (2,229 observations), 604 with a professional email**; the research graph behind it is 2,298 rows / 874 schools / 10 states | 0 network requests (import) | cheapest coach layer; carries the 51 coach emails and 620 AD emails that no site publishes in bulk `[lane: coach-directories-national §Baseline]` |
| 7 | **`sources/ks/`** (KSHSAA) | KS only | **526 coach rows in 1 request, 523 with email**; 526/526 AD name+email validated wholesale | **1** response | best single-request ratio measured anywhere in the study; carries no sport coach and no athlete `[lane: coach-directories-national §KS]`, `[lane: state-assoc-plains §10]` |
| 8 | **`sources/plain_names/`** (nsaa + ndhsaa) | NE, ND | NE 1,589 evidence rows / ND 922 — **names only, 0 emails** | 313 + 170 responses (1 POST/GET per school) | closes 12,204 Co2027 rows' coach-identification gap with names; email-free by design `[lane: coach-directories-national §NE,§ND]` |
| 9 | **`sources/coach_contacts/wire.rs` + `sources/wayzata/`** | MI (6 rows), MN (2 responses) | 6 MI AD rows; 2 Wayzata responses feeding 537 schedule rows / 536 core meets | 2 responses | the long tail that still contributes; both are single-purpose and cheap `[lane: state-assoc-greatlakes §7.4]`, `reports/midwest-census-2026-09-20-method-notes.csv` |
| 10 | **format parsers `hytek/`, `compiled/`, `xc/`, `result_file.rs`** | all states | 96 parsed artifacts before the vendor layouts landed → **1,740** after (Compiled 763, cross-country 380, Hy-Tek 597); 834,254 result rows, 264,167 grade-bearing; the corpus added **4,323 WI Co2027 to the 09-20 core (14,958 → 19,281)** | bundled inside the families above | the leverage is in the format layer, not the source count: one Hy-Tek parser covers every timer that emits it `reports/midwest-census-2026-09-20-method-notes.csv` |

**Deliberately not ranked** — `data/adapter-ranking.csv` (written before the 12 lanes and the census landed) ranks
ten adapters, and this list regroups them by *measured census yield* instead of estimated yield. The sources that
drop out of the top ten, with the measured reason:

| module | `data/adapter-ranking.csv` rank | measured census contribution | why it is not in the top ten |
|---|---:|---|---|
| `sources/ohsaa/` | 7 | OH 456 Co2027 linked, 28 coach rows (23 with email), 33 `association_school:ohsaa` observations | every request from the campaign's rustls client is reset at connect (`Connection reset by peer`) while curl/OpenSSL gets 200 on the same host and IP; the census did not impersonate a browser, so the provider contributes the research-compiled contact rows only `README.md` line 106, `synthesis/05-compliance-and-risks.md` §6 |
| DirectAthletics (`sources/`) | 6 (as "TFRRS/DirectAthletics Indiana") | 0 census rows | HS-marked index rows total **17 across 12 states** (IN 6 · MO 4 · KS 7) and `dat_live_hs_performance_lists` is non-"none" for IN only; team registries are 22,527 records (11,791 TF + 10,736 XC) `data/dat-provider-coverage.csv`, `[lane: national-aggregators §3 row 2]` |
| MileSplit `/api/**` and `/rankings/**` | — (never a module) | 0 | `Disallow: /api/` on every host sampled, and rankings are locked beyond row 1 (ranks 2+ need PRO; athlete result marks are absent from the bytes) — the build must use `/teams`, `/teams/<id>/roster`, `/raw` and `/search/v2/athletes` instead `[lane: milesplit-national §18,§21]` |
| `sources/athleticnet/` | 0 (it is the baseline) | 0 rows, 0 requests | kept for the seam it defines (id grammar, `LiveID` join, 2-request whole-meet pull), not for yield `[lane: athleticnet §23]` |
| `sources/raceday/`, `sources/registry/` | — | **0 observations** | the store's complete evidence-id vocabulary is 23 ids and neither appears: `athleticlive_athletes` 817,928 · `wiaa_results` 115,515 · `milesplit_{mi 95,681, oh 89,261, il 70,383, wi 63,272, mn 54,710, ia 49,521, mo 42,939, in 41,873, ne 36,120, ks 30,472, sd 17,330, nd 12,765}` · `ihsa` 19,526 · `mshsl` 13,104 · `athleticlive_meets_csv` 9,667 · `wiaa_directory` 3,179 · `coach_contacts_csv` 3,103 · `nsaa` 1,901 · `ndhsaa` 1,091 · `ks` 1,052 · `wayzata_schedule` 745 across `athletes/schools/coaches/meets.jsonl` |

Unverified:
- Ranks 1 and 2 are ordered by *distinct athletes*, not by *unique* athletes: they overlap (Q4: 31.2 % carry both
  namespaces), and no artifact measures their union-minus-overlap per state. A `report` run grouping
  `class_of_2027` by evidence source set per state would settle it.
- `sources/athleticnet/` exists in the crate and is ranked nowhere here **on purpose**: it contributed 0 rows
  (Q1) and 0 requests. Its measured value is the seam it defines (id grammar, `LiveID` join, 2-request whole-meet
  pull) — `[lane: athleticnet §23]` — not a yield. Settling capture: none — the rank follows from the measured 0 rows / 0 requests.
- The engineering-effort column in `data/adapter-ranking.csv` (`LOW`/`LOW-MED`/`MED`) is a judgement, not a
  measurement; this file therefore orders by measured yield ÷ measured requests, and no artifact on disk
  converts either into staff time. Settling capture: none on disk — it needs delivery timing from the repo's own work history.
- `data/adapter-ranking.csv` was written before the 12 lanes landed: its rank 1 "Co2027 boys census 41,145 …
  to ~62,400 [INFERENCE]" and rank 5 "258 tenant origins" disagree with the newest measurements (142,535
  MileSplit rows; 256 live tenant indexes measured in one aggregation, `[lane: timing-providers-national §3]`,
  which itself records the 258-vs-256 difference as "agreement within 2"). Both numbers printed; the census and
  the live aggregation are retained. Settling capture: none needed — both numbers are printed above and the ranking CSV is superseded.
