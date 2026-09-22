# 03 — Adapter build order and refresh design (post-national-lane revision)

Written **after** the 12 national lanes landed. Supersedes the pre-lane `03-adapter-ranking.md`; every
superseded claim is listed in §0 with the lane that overturned it. Evidence tags follow the source
reports: `[sources/<lane>]` = `athletic-rust-pipeline/research/sources/<lane>/SOURCE_REPORT.md` (+
`coverage.json`, `samples/`), `[research/midwest/NN-*.md]` = `~/Downloads/midwest-tfxc-source-research/research/midwest/`,
`[data/*.csv]`, `[reports/*.csv]`, `[crates/...]` = `athletic-rust-pipeline/crates/midwest-census/src/...`.
`[INFERENCE]` marks arithmetic or reasoning, not capture.

## 0. How to read this document

- **Decision unit**: one production adapter = one source × one shape (e.g. "AthleticLIVE blob event doc",
  not "AthleticLIVE"). Cost is in HTTP requests, always per the *measured* unit the lane recorded.
- **All request counts are measured** unless the cell says `[CALC]` (arithmetic on measured units) or
  `[INFERENCE]`.
- **The incumbent baseline** every marginal column is measured against is the run of 2026-09-20:
  `[reports/census-by-state-core.csv]` = 146,858 Class-of-2027 athletes in core sources;
  `[reports/midwest-census-2026-09-20-athleticnet-marginal.csv]` = 190,087 Co2027 across all sources,
  43,229 of them **only** visible through Athletic.net, core share 77.3 %.
- **Grade-evidence mix of the incumbent** `[reports/midwest-census-2026-09-20-evidence-mix.csv]`:
  `athleticlive_athletes` 462,875 athlete-records (not distinct athletes), twelve `milesplit_<st>` tenants
  totalling 142,569 (re-summed here: OH 27,000 + IL 19,398 + MI 16,176 + WI 14,984 + MO 13,218 +
  IN 11,909 + MN 11,261 + KS 9,252 + IA 8,738 + NE 5,921 + SD 2,669 + ND 2,043), `wiaa_results` 13,573.
  Coach mix: `association_school:ihsa` 15,114,
  `mshsl_team_coach` 5,843, `association_school:nsaa` 1,528, `…mshsl` 760, `…kshsaa` 526, `bound` 78,
  `…ohsaa` 33, `…wiaa` 22, `…ndhsaa` 13, `…mhsaa` 6.
- **What already exists in the crate** (registry slugs) `[crates/midwest-census/src/sources/registry/table.rs]`:
  `athleticlive` (harvest CSV, meet_discovery only), `athleticlive_athletes` (ES `athlete_list`),
  `athleticnet` (bio API), `coach_contacts` (artifact CSV), `ihsa`, `ks`, `milesplit`, `mshsl`, `ohsaa`,
  `plain_names` (ND+NE), `wiaa`, `wiaa_results`. Plus unregistered-by-slug machinery: `hytek`, `compiled`,
  `raceday`, `xc`, `wayzata`, `result_file`. **Ranking below is "extend this", not "build from zero"** —
  the effort column says which.

### 0.1 Six pre-lane claims this document overturns

| Pre-lane claim | Measured now | Evidence |
|---|---|---|
| MileSplit roster is rank 1 because 6,645 requests is "the only measured route to the ~34 % complement" | The roster frame is **26,562 HS teams across 51 jurisdictions in 51 requests**, not a 12-state-only guess; and roster cost is ~42 requests per 1,000 Co2027 (~24 Co2027 per populated roster) against **1.07** for TFRRS-IN and **~0.5** for the AL athlete index. MileSplit moves to rank 4 on cost-per-athlete, and its 34.1 % complement is still an inherited sample figure | `[sources/milesplit-national]` §2/§16/§20, `[data/milesplit-coverage-matrix.csv]` |
| IHSA's "head-coach row present in 42/56 sampled school-sport-gender cells (75 %)" | **No corpus artifact carries a 42/56 IHSA figure.** The only 56-cell measurement is OHSAA's `SportsInformation` stress ticket (56 cells → 33 named, all 33 with `mailto:`); IHSA's measured equivalent is the `staff2` sample: 482 staff rows over 14 schools, TRB 11/14, TRG 12/14, CCB 9/14, CCG 10/14, AD 14/14, 49/49 track-XC coach rows `HasEmail: true` | `[research/midwest/19-ohio-ohsaa.md]` line 505, `[research/midwest/13-illinois-ihsa.md]` lines 398/209 (the 42/56 number is unverifiable in the corpus) |
| AthleticLIVE "5–6 requests per meet, or 1 ES query per 1,000 docs" | Platform census is **1 request ↔ 153,774 meet docs** (0.0065 per 1,000); the athlete index is **2,000 rows/request** with a 10,000 `from+size` window and 40 meets per query; the blob event doc is 1 request ↔ 136 rows | `[sources/timing-providers-national]` §3/§16, `[crates/midwest-census/src/sources/athleticlive_athletes/mod.rs]` (`PAGE_SIZE`, `MEETS_PER_BATCH`, `RESULT_WINDOW`) |
| Athletic.net whole meet = 3 XHR | **2 requests for results** (`GetEventDivisionData` skippable — settled 2026-09-21 by issuing step 3 with only step 1's `jwtMeet`), 3 only when per-event metadata (`isHurdle`, `FieldMeasureType`, `Type`) is wanted. 2 requests ↔ 758 individual + 288 relay rows = 1.91 per 1,000 rows | `[sources/athleticnet]` §16/§22.9 |
| "Long tail" coach band = IA + SD (Bound) + MI + ND | **IA and SD Bound are retracted** (`gobound.com/robots.txt` = `User-agent: *` + `Disallow: /*directory`, `Crawl-Delay: 10`) and the MI administration API is refused (`robots.txt` disallows `/DesktopModules/`). The buildable long tail is ND (already implemented as `plain_names`) | `[sources/coach-directories-national]` + `tools/validation-retractions.json`, `[sources/state-assoc-plains]` §0.3 |
| "AthleticLIVE = MN/IA/ND evidenced; 258 tenant origins" | 256 tenants in the package inventory (258 SPA entries incl. `base-site`), 153,524 meet docs summed, **129,203 (84.2 %) carrying an Athletic.net meet id**; verified live by two lanes independently | `[data/athleticlive-tenant-inventory.csv]` (re-summed in this document), `[sources/national-aggregators]` §3, `[sources/timing-providers-national]` §3 |

### 0.2 Known discrepancies, printed both ways (never averaged)

| Quantity | Value A | Value B | Retained | Why |
|---|---|---|---|---|
| AthleticLIVE meet docs | 153,524 (sum of `meet_docs` over 256 tenants) | 153,774 (ES wildcard `match_all` over `*_meet_list`) | **153,524** for per-tenant work; 153,774 quoted as the ES-side total | Two different measurements of the same population; 250-doc gap un-reconciled (`[data/athleticlive-tenant-inventory.csv]` vs `[sources/timing-providers-national]` §3) |
| ND meet docs | 875 (`terms` agg on `lsa.keyword`) | 5,222 (`match` on the text field `lsa`) | **875** | The text-field count is a full-text match and therefore an upper bound; the aggregators lane flags this itself (`[sources/national-aggregators]` Open Q2, `[sources/timing-providers-national]` §3) |
| SD meet docs | 1,869 (`lsa.keyword`) | 3,188 (text match) | **1,869** | same |
| NY meet docs | 6,917 (`lsa.keyword`) | 9,664 (text match) | **6,917** | same |
| Meet links usable for refresh | 9,747 rows with non-empty AN id + URL + AL ids | 9,746 after excluding the one `athleticnet_meet_id = 0` row (`TEST Sample Meet - Training 8/6`, IL) | **9,746 real / 9,747 non-empty** | Re-counted in this document; the lane's 9,747 is the non-empty count |
| Per-state meet-link counts in `[data/athleticnet-meet-seeds.csv]` | IL 2,159 / MI 1,433 … (lane table, = **all 9,844 file rows** per state) | IL 2,128 / MI 1,421 … (this document, restricted to non-empty non-sentinel AN ids) | **2,128 / 1,421** for planning | Re-counted here: 9,844 rows total, 9,747 with a non-empty AN id (identical set to "non-empty + URL + AL ids"), of which 1 is the `id = 0` test meet → **9,746 usable**; the 97-row difference is rows whose AN id is blank or a sentinel |

## 1. Ranked adapter table

Ordering rule: **rows 1–6 are athlete-bearing** and are ordered by measured requests per 1,000 verified
Class-of-2027 rows (cheapest first), then divided by engineering effort; **rows 7–10 buy no athlete rows**
(coach/contact layers) and are ordered by measured requests per new contact row. A source being large is
not a reason to rank it: Athletic.net is the largest identity spine in the corpus and still ranks 2,
because the cost that matters is requests per *new* row, and its rows are substantially already in the
core census.

| # | Adapter (shape) | Scope / states | First-build requests | Steady-state weekly requests | Engineering effort | What it uniquely buys | Identity channel (AN id / MS id / school id / grade) | Cost per 1,000 measured rows | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| 1 | **AthleticLIVE result plane — blob event doc** (`ind_res_list/_doc/<eventId>`) **+ RTDB standings** (`meet_<id>/liveRunStandings/<rui>`) — extend `athleticlive` from meet-discovery to `bulk_results` | 122 tenants of 256 already named on disk for the Midwest harvest; 20,254 harvest rows / 18,112 with `has_results=True`; all-time meet docs (text-match, aggregators lane; SD 1,869 / ND 875 are the retained keyword-agg values per §0.2): IL 14,599 · MI 10,775 · MN 7,502 · IA 7,169 · OH 4,542 · NE 4,534 · WI 3,902 · SD 1,869 · MO 2,517 · IN 2,233 · KS 1,698 · ND 875 | 1 request is the whole tenant census (`_msearch` over 256 indices, 44,948 B); **0 discovery requests for meets** (harvest CSV already names tenant + AL meet id); event docs 1 request each; RTDB 1 request per race | Per touched meet: 1 per event doc (`If-None-Match` → 304/0 B) or 1 per race on RTDB; a 40-meet batch on `athlete_list` is 1 request | **MED** — doc shapes captured and verified key-by-key over 136 rows; blob conditional GET proven; the one unverified link is **event-id discovery** (Open Q1, 1 request to test) | Marks for whole fields (`m`, `im` integer ms, `irs` splits, `avk`/`avm` pace, `er`/`pr` seed/PR, `p`/`pt`/`ro`) plus grade on every row — the only free channel that returns all three of mark + grade + Athletic.net id in one request | `a.ani` = AN athlete id, `t.ani` = AN team id, meet doc `ani`/`athleticnet_meet_id`, `y` grade (SR/JR/SO/FR or 9–12), `t.i`/`a.i` AL ids. **No** MileSplit id | **7.4** per 1,000 rows (1 request ↔ 136 rows); RTDB race 185 entries/request; census 0.0065 per 1,000 docs | `[sources/timing-providers-national]` §3/§16, `[sources/national-aggregators]` §3, `[research/midwest/12-iowa-wayzata-results.md]` (304 revalidation), `[data/source-coverage-matrix.csv]` ND row (RTDB) |
| 2 | **Athletic.net whole-meet pull** (`Meet/GetMeetData` → `GetAllResultsData`) — extend `athleticnet` from bio-only | 12 Midwest states; inputs already on disk: **9,746 usable AN meet links** (IL 2,128 / MI 1,421 / OH 1,227 / IA 993 / MN 980 / WI 775 / NE 585 / MO 564 / IN 372 / SD 358 / KS 181 / ND 162) | 2 requests per meet (3 with event metadata) | 2 × new meets in the week; with 9,746 historical links the one-off backfill is **19,492 requests** `[CALC]` | **LOW–MED** — the CF/session transport and JSON decoding for this host already exist; the meet payload shapes are the new part | Full division + round + relay legs (288 rows on the probe meet) + per-row `Grade` + `IDResult`; the only source with entry lists, wind/heat and `LiveID` | AN `AthleteID` + `TeamID` + `MeetID` + `IDResult` + row `Grade`; `LiveID` = the AL meet id (must be persisted) | **1.91** per 1,000 rows (2 requests ↔ 1,046 rows) vs **586** profile calls for the same meet (the lane's 1-per-athlete measurement; the crate's tf+xc loop makes it 1,172) | `[sources/athleticnet]` §16/§19/§23, `[data/athleticnet-meet-seeds.csv]`, `[data/canonical-meets.csv]` (9,593 rows with AN meet id) |
| 3 | **TFRRS high-school instance** (`indiana.tfrrs.org` performance lists + team rosters) — new adapter | IN only (FL and NH are the only other HS instances; `texas`/`ohio`/`california.tfrrs.org` = NXDOMAIN) | **2 requests → 1,861 Co2027** (`&year=JR`, HSR Large 1,155 + Small 706, disjoint); +1 request per list page unfiltered (6,533 athletes at `?limit=1000`); roster route ≈901 gender-team pages | 1 request per list page per season boundary; ~0 weekly (seasonal source) | **LOW** — plain server-rendered HTML, empty robots policy, no auth, no challenge; numeric `AthleteID`, grade column, team ids | The only source that exposes **graduating class as a first-class list filter** (`&year=JR` returns exactly the Class of 2027), with an independent numeric athlete id that is neither AN nor MS — 1,861 Co2027 in 2 requests | Grade column + `Year` filter, numeric athlete id, numeric TeamIDs (929 TF + 887 XC from the parent domain), meet ids — **no AN id, no MS id** | **1.07** per 1,000 Co2027 (2 requests ↔ 1,861) — the cheapest *verified Co2027 enumeration* per request in the corpus | `[sources/national-aggregators]` §2/§20, `[sources/timing-providers-national]` §5 (1 request ↔ 2,060 rows, 0.49/1,000), `[research/midwest/18-indiana-directathletics-milesplit.md]`, `[data/source-coverage-matrix.csv]` IN row |
| 4 | **MileSplit roster + `/raw` result file** (`/teams`, `/teams/<id>/roster`, `/meets/<id>/results/<RSID>/raw`) — extend `milesplit` from roster-only | **51 jurisdictions** (26,562 HS teams measured; 6,633 for the 12 Midwest states: OH 977 · MI 895 · IL 849 · MO 700 · WI 597 · MN 592 · IN 533 · IA 411 · KS 413 · NE 317 · SD 196 · ND 153) | 1 request per state `/teams` (51 total) + 1 per roster; 6,633 rosters for the Midwest; results index 1 per 50 meets; `/raw` 1 per result set | Rosters 2×/season (6,633 requests per Midwest pass); results 1 request per new result set; sitemap 1 per host as a change signal | **LOW** — the roster parser is implemented and tested; `/raw` is fixed-width text (80-row sample on disk, `[inherited]` files up to 5,500 rows) | The only source that enumerates athletes with an **absolute graduating class** on a robots-allowed surface for all 51 jurisdictions, and free grade-bearing result files (`Yr` column) that need no API | MileSplit `AthleteID`, `TeamID`, `MeetID`, `RSID`, `column-grad-year` (95.9 % row presence inherited); **0 occurrences of `athleticnet` in any sample** — the AN join is a tuple match, already materialized as 69,819 AN↔MS id pairs | **~42** per 1,000 Co2027 planned (1 request ↔ ~24 Co2027 inherited median); **10.4** per 1,000 on the best retained roster (96 Co2027 in 1 request, OH Mason, re-counted here); `/raw` **12.5** per 1,000 rows (1 request ↔ 80 rows, up to 5,500 inherited) | `[sources/milesplit-national]` §2/§8/§16/§19/§21, `[data/milesplit-coverage-matrix.csv]`, `[data/athleticnet-athlete-seeds.csv]` (69,819 pairs), `[crates/midwest-census/src/sources/milesplit/]` |
| 5 | **IHSA API — tournament results + entry grade** (extend `ihsa` beyond `/v1/schools` + `/staff2`) | IL only — the largest Midwest cohort: 26,488 Co2027 all sources, 19,392 core, **7,096 Co2027 visible only through Athletic.net** | ~10/season for universe + structure (1 request = 828 schools with membership/enrollment/class/coop), ~200/season for T&F operations, 828 for `staff2` **+ 1 per revealed address** (`/v1/schools/{id}/staff/{person}/email`) | 1 + Δ (server sends `cache-control: public, max-age=180`); tournament index 1 request per season; per-event summaries as they appear | **LOW** — module exists with parse/map/staff; the work is the tournament endpoints and the `athleticNetId` mapping | Per-finisher `athleticNetId` **and** `athleticLiveId` on state-final rows; grade on entries (3,342 entry lines → 2,555 unique triples, 838 Jr; XC 3A boys 1,214 qualifiers with 358 G11); coach emails at 12/12 sampled rows and 4/4 AD rows | `athleticNetId` per finisher + team `athleticNetId` (e.g. team 16352); `PersonID` for coaches; grade from the entry list. No MileSplit id | ~0.3 per 1,000 finals rows `[CALC]`: 1 request ↔ ~3,342 entry lines | `[research/midwest/13-illinois-ihsa.md]`, `[research/midwest/30-cross-source-pareto.md]` line 186 (IL 12/12 coach emails, 4/4 AD), `[data/source-coverage-matrix.csv]` IL row, `[crates/midwest-census/src/sources/ihsa/]` |
| 6 | **MSHSL JSON API — rosters with grade + coach email** (extend `mshsl`, and delete the artifact path that duplicates it) | MN only (664 schools) | 4 requests (team lists) + 664×2 roster calls + 1,494 coach calls + 664 school pages ≈ **3,490** `[CALC]` | 1/team/activity per season; **no backfill** (current-season only); coach slice on a rolling monthly schedule | **LOW–MED** — implemented; the effort is running it at scale and keeping only school-domain emails | School-published grade (07/08/FR/SO/JR/SR) + head and assistant coach names + professional email from one request family — the only association that returns grade *and* a coach email | `hostSchoolId`, team node `nid`, `activityId`, `yearId`, `MeetID 666673` — **no AN id, no MS id** on the roster surface | ~1.5 per 1,000 roster rows (1 request ↔ 664 rows in the school index; roster pages ≈ 100–300 rows) | `[research/midwest/09-minnesota-mshsl.md]`, `[sources/coach-directories-national]` (MN tier-1: AD name yes, AD email no), `[crates/midwest-census/src/sources/mshsl/]` |
| 7 | **KSHSAA directory** (already implemented as `ks`; rank is for the **run**, plus the sibling classification refresh) | KS only (348 classified members; 526 directory rows) | **1 request = 526 schools + AD name + AD email** (489 KB) | 1 per month; per-school `DateModified` (`directory/Id/{id}`) gives the delta key | **DONE** — build cost 0; run cost 1 request/refresh | The cheapest AD layer in the study: 100 % AD name + email on a wholesale-validated slice (526/526, both fields) | `kshsaa` school identity, `ADName`/`ADEmail`, class/enrollment/league; **0 sport-coach rows**, no athletes | **1,000** contact rows per 1,000 requests (1 request ↔ 526 AD rows) | `[sources/coach-directories-national]` baseline validation, `[data/coach-contacts.csv]` (526 KS rows), `[research/midwest/23-kansas-kshsaa-directathletics.md]` §DateModified |
| 8 | **myOHSAA portal at scale** (already implemented as `ohsaa`; the gap is coverage, not code) | OH only (815 schools; 267 AAA / 267 AA / 267 A / 14 blank) | 3 requests/school → **2,445 requests** for the full state `[CALC]` (~1,630 if the `ohsaaId` comes from the 18 retained PDFs) | 1/3 of schools per month (~272 schools × 3 requests ≈ **815 requests/month** `[CALC]`); pages are plain HTML + `mailto:` | **DONE** — build cost 0; run cost ~2,445 requests once + monthly slices | Closes the largest coach-coverage hole in the incumbent census: **456 of 31,708 OH Co2027 have a coach today (1.4 %)**; the portal gives AD + per-sport head coaches with emails (18 coach rows/15 with email, 13 AD/13 with email on a 5-school sample) | `ohsaaId` ↔ Athletic.net team-name key; `PersonID`-free but school-scoped; **no athlete ids, no grade** | **~20** contact rows per 1,000 requests (33 rows from 1,630–2,445 requests measured to date) | `[research/midwest/19-ohio-ohsaa.md]`, `[sources/coach-directories-national]`, `[reports/midwest-census-2026-09-20-evidence-mix.csv]` (`association_school:ohsaa` = 33) |
| 9 | **WIAA directory + tournament grade results** (already implemented as `wiaa` + `wiaa_results`) | WI only (516 members; 1,796 team-season rows; 629 enumerated schools) | 4 team universes + 26 letter queries + 629 school pages ≈ 659; 1 archive page + 1 PDF per tournament meet | 1/school per season for coaches (host sends `no-cache,no-store` → full reads); tournament pages 1–5/season | **DONE** — build cost 0 | Highest per-school coach-email yield measured (1 request/school, 17/17 coach rows and 5/5 AD rows with email) and the state's own tournament ATN MeetIDs + result files with a `Yr` grade column | `wiaa_directory` school identity; tournament `MeetID` ↔ AN meet id in the same row; grade from timer reports | **~33** contact rows per 1,000 requests (22 rows measured from a partial run) | `[research/midwest/06-wisconsin-wiaa.md]`, `[sources/state-assoc-greatlakes]` §6, `[crates/midwest-census/src/sources/wiaa_results/]` |
| 10 | **Timer-index discovery layer**: PrimeTime `results/current` API + DAT `fuseDriver_Upcoming.js` + TFRRS `results.rss` + Wayzata schedule (extend `wayzata`) | WI/MO (PrimeTime: 353 events in 2026, WI 245 · MO 24 · IL 12 · IA 8 · IN 8 · NE 6 · MN 2 · KS 2), national DAT (946 upcoming meets, 49 venue states), national TFRRS RSS (75-item rolling window) | PrimeTime **2 requests per season**; DAT **1 request = 946 meets**; wayzata 2 requests per sport-year; TFRRS RSS 1 request | 1–3 requests per state per week | **LOW** — all four are static files/JSON; `wayzata` already retains the `/links/<slug>` keys as non-core and deliberately never fetches them | New-meet discovery **with AN MeetIDs attached**, without any Athletic.net search: PrimeTime embeds AN entry links on 130/200 page-1 rows; DAT gives a national upcoming index; TFRRS RSS is a free change feed | PrimeTime: AN meet id via `computed.entryLinkHtml`; DAT: `meet_hnd`/`tfrrs.org/results/<id>` cross-links (no AN id); Wayzata: `/links/<slug>` → AL tenant | PrimeTime index **0.35** per 1,000 events (1 page ↔ 200 events) `[sources/timing-providers-national]` | `[sources/timing-providers-national]` §4/§5, `[sources/national-aggregators]` §1, `[research/midwest/22-missouri-primetime.md]`, `[crates/midwest-census/src/sources/wayzata/mod.rs]` |

### 1.1 Why this order — the arithmetic, not the adjectives

1. **Rank 1 over rank 2 is decided by one unverified link, not by cost.** Per row, the AN whole-meet pull is
   cheaper (1.91 vs 7.4 requests per 1,000 rows) and needs no event-id discovery — but it needs the CF
   session tier and yields 1,046 rows/meet, while the blob path is anonymous, revalidates at 0 bytes, and is
   keyed by an inventory we already hold (20,254 harvest rows, 18,112 with results). The flip condition is
   Open Q1: one `POST search.athletic.live/ind_res_list/_search`. If that 200s with a `mi`-filterable doc,
   rank 1 stays; if it 404s, **rank 2 becomes rank 1** because event ids would then cost a rendered SPA page
   per meet.
2. **Rank 3 over rank 4 is pure measured arithmetic.** TFRRS-IN returns 1,861 Co2027 in 2 requests
   (1.07 per 1,000). MileSplit returns ~24 Co2027 per roster request (≈42 per 1,000) and needs 6,633
   requests for one Midwest pass. MileSplit still beats TFRRS on *breadth* (51 jurisdictions vs 1 state) —
   which is why it stays in the top 5 rather than being demoted on ratio alone.
3. **Rank 5 is the only adapter that adds Athletic.net ids to rows the core census cannot see.** IL has
   7,096 Co2027 visible only through Athletic.net; IHSA is the one association that publishes
   `athleticNetId` per finisher *and* a coach email in the same API family, at ~10 requests per season for
   the universe.
4. **Ranks 7–9 are not ordered by athlete coverage because they buy none.** They are ordered by measured
   requests per new contact row (KSHSAA 1 request ↔ 526 AD rows; OHSAA ~2,445 ↔ ~815 schools; WIAA 1
   request ↔ 1 school) and by the size of the gap they close in the incumbent census (core coach
   coverage: OH **1.2 %** = 313/26,998, IL **33.2 %** = 6,432/19,392, WI **76.2 %** = 14,699/19,281)
   `[reports/census-by-state-core.csv]` — so OH buys the most, WI the least.
5. **What is deliberately *not* in the top 10**: a national coach-directory sweep. Only 3 of 51
   jurisdictions publish a coach email at tier 1 (IL, OH, WI) and 26 are client-rendered, so a national
   coach adapter would be 26 browser sessions for `unverified` fields `[sources/coach-directories-national]`.

### 1.2 Adapter detail cards (the numbers behind each rank)

Each card states the measured request unit, the marginal claim, the identity channel, and the single
condition that would move the adapter. Cards carry no claim that is not already in §1 or §4.

**Rank 1 — AthleticLIVE result plane (blob event doc + RTDB standings)**

| Field | Value | Evidence |
|---|---|---|
| Public access | ES `POST search.athletic.live/*_meet_list/_search` anonymous 200; Azure blob `$web` container public; RTDB `?ns=trackmeet-io` anonymous; SPA HTML is a 50,184 B shell | `[sources/national-aggregators]` §3.13/§3.15 |
| Measured unit | 1 request ↔ 136 result rows (159,531 B); 1 request ↔ 153,774 meet docs for the census; RTDB 1 request ↔ 1 race (185 entries in the ND Class A Boys race) | `[sources/timing-providers-national]` §3/§16, `[data/source-coverage-matrix.csv]` ND |
| Inventory already held | 20,254 AL meet rows (122 tenants, 18,112 `has_results=True`) from the harvest; 9,746 usable AN meet links; 256-tenant inventory with `meet_docs` and `meet_docs_with_ani` | `[data/athleticlive-midwest-2026-meets-all.csv]`, `[data/athleticnet-meet-seeds.csv]`, `[data/athleticlive-tenant-inventory.csv]` |
| Identity | `a.ani` + `t.ani` + meet `ani` + `y` grade; also `im` ms marks, `irs` splits, `pr`/`er` | `[sources/timing-providers-national]` §3 |
| What it does not do | The crate's `athleticlive` slug is `meet_discovery` only — the registry comment says the harvest carries `has_results`, "not the result rows" | `[crates/midwest-census/src/sources/registry/table.rs]` |
| Moving condition | Open Q1 (event-id discovery). If event ids cost a rendered page per meet, this card swaps places with rank 2 | §5 Q1 |

**Rank 2 — Athletic.net whole-meet pull**

| Field | Value | Evidence |
|---|---|---|
| Measured unit | 2 requests ↔ 758 individual + 288 relay rows (1,046); 3 requests when `GetEventDivisionData` metadata is wanted; the 45-request per-event fallback is the only path that adds `Wind`/`Heat`/`HeatPlace` | `[sources/athleticnet]` §16 |
| Comparison | 586 per-athlete profile calls for the same meet = 195.3× the meet cost; a national profile scan is 142,705 requests vs 4,233 for the rankings scan (33.7×) | `[sources/athleticnet]` §16 |
| Access tier | All census-affecting reads need the session tier: anonymous rows are masked past rank 5 (`blurAfterDepth: 5`), `GetNavInfo` is 403 to plain clients, and the same filter reports 982 (anonymous) vs 2,117 (session) | `[sources/athleticnet]` §16/§18 |
| Guard rails | `GradeID == 99` is a relay placeholder, never a grade; `Overseas (170162)` must be filtered as out of scope; totals must record the tier that produced them | `[sources/athleticnet]` §23 |
| Moving condition | Two one-lane captures: `/events/usa/<state>` calendar (Open Q9) and the division page roster endpoint (Open Q: school roster per state, still uncaptured) | §5 Q9 |

**Rank 3 — TFRRS high-school instance (Indiana)**

| Field | Value | Evidence |
|---|---|---|
| Measured unit | 2 requests ↔ 1,861 Co2027 (`&year=JR`: HSR Large 1,155 + HSR Small 706, disjoint); 1 request ↔ 6,533 athletes unfiltered at `?limit=1000`; 1 request ↔ 2,060 rows for a sampled HS meet | `[sources/national-aggregators]` §2.20, `[sources/timing-providers-national]` §5 |
| Grade trap | `&year=SR` on the 2025-26 lists returns 1,678 = **Class of 2026** (at-meet grade), not Co2027 | `[data/source-coverage-matrix.csv]` IN row |
| Roster route | 452 slugs / 901 gender-team pages ≈ 12,300 Co2027 `[INFERENCE]` at 28.6 % JR over 419 sampled rows | `[sources/national-aggregators]` §2.6 |
| Access | Plain HTML, `robots.txt` comment-only (99 B), no auth, no challenge; `results.rss` is a 75-item rolling change feed | `[sources/national-aggregators]` §2.17/§2.13 |
| Moving condition | Open Q6: whether IN's 2026-27 provider is still TFRRS (2026 track coverage collapsed to 56 meets vs 1,022 in 2025) | §5 Q6 |

**Rank 4 — MileSplit roster + `/raw`**

| Field | Value | Evidence |
|---|---|---|
| Frame | 26,562 HS team records across 51 jurisdictions, 1 request per state, no pagination; 12/12 exact agreement with the prior independent sweep on the 12 Midwest states | `[sources/milesplit-national]` §2/§16/§20 |
| Midwest cost | 6,633 rosters (OH 977 · MI 895 · IL 849 · MO 700 · WI 597 · MN 592 · IN 533 · IA 411 · KS 413 · NE 317 · SD 196 · ND 153) | `[data/milesplit-coverage-matrix.csv]`, summed here |
| Grade | `column-grad-year` free on every roster row; 100 % coverage on one OH roster (319 rows), 95.9 % over 50 inherited WI/MN rosters. Re-counted in this document on the retained OH capture (`samples/roster-oh-mason.html`): 319 rows, **96 Co2027** (2027:96 · 2028:88 · 2029:88 · 2030:46), XC flag on 180 rows — so 1 request buys **96** Co2027 on this roster vs the inherited ~24 median | `[sources/milesplit-national]` §8, `[data/milesplit-coverage-matrix.csv]` (median), sample re-counted here |
| Result path | `/meets/<id>/results/<RSID>/raw` = whole result set in one request incl. `Yr`; the formatted view is a JS shell with 0 rows | `[sources/milesplit-national]` §10/§14 |
| Hard limits | `Disallow: /api/` on every host; rankings render 1 usable row; the profile's 480 mask spans carry no text; `TeamID`, `MeetID`, `RSID`, `AthleteID` are the only keys — the `www`/state host is never the identity | `[sources/milesplit-national]` §18/§19 |
| Moving condition | Open Q3 (MI roster emptiness, 15 requests) and Q4 (discovery vs validation) | §5 Q3/Q4 |

**Rank 5 — IHSA tournament results + entry grade (Illinois)**

| Field | Value | Evidence |
|---|---|---|
| Universe | 1 request ↔ 828 member schools with membership/enrollment/class/coop; `SchoolID` zero-padded 4-char | `[research/midwest/13-illinois-ihsa.md]`, `[crates/midwest-census/src/sources/ihsa/mod.rs]` |
| Grade evidence | 3,342 state-final entry lines → 2,555 unique triples with Sr 1,295 / Jr 1,079 / So 636 / Fr 332 → 838 Jr; XC 3A boys 1,214 qualifiers with 358 G11 | `[data/source-coverage-matrix.csv]` IL row |
| AN ids | `MeetId` 74003/74002 carry `athleticNetId` + `athleticLiveId` per finisher; team `athleticNetId` 16352 | same |
| Coach layer | `staff2` + `/staff/<PersonID>/email`: 482 staff rows over 14 schools — TRB 11/14, TRG 12/14, CCB 9/14, CCG 10/14, AD 14/14; **49/49 track-XC coach rows `HasEmail: true`** (2 request families: list + per-person reveal) | `[research/midwest/13-illinois-ihsa.md]` lines 398/209, `[sources/coach-directories-national]` (IL = one of three tier-1 email jurisdictions), `[research/midwest/30-cross-source-pareto.md]` line 186 |
| Cache | `cache-control: public, max-age=180`, `vary: Origin`, no rate-limit headers | `[research/midwest/13-illinois-ihsa.md]` lines 300/408 |
| Moving condition | Open Q5 (`athleticNetId` on XC finals docs) | §5 Q5 |

**Rank 6 — MSHSL rosters with grade + coach email (Minnesota)**

| Field | Value | Evidence |
|---|---|---|
| Universe | 664 `/schools/` URLs + 1,097 `/tournaments/` URLs in one sitemap request; 1,494 team targets (386+388+361+359) across four sport-seasons | `[sources/state-assoc-plains]` §0.2, `[research/midwest/09-minnesota-mshsl.md]` |
| Grade | In-season roster grades `07/08/FR/SO/JR/SR`; state archive pages print Name/Grade/Mark | `[data/source-coverage-matrix.csv]` MN row |
| Coach layer | AD 9 rows / 7 with email on school pages; coach names free on the roster endpoint; emails via `/api/coaches/<nid>` (4/10 records carry email on the sample) | `[sources/coach-directories-national]` (MN: AD name yes, AD email no), `[research/midwest/09-minnesota-mshsl.md]` |
| Deliberate exclusion | Personal-domain emails are dropped by `accept_coach_email`; only school-domain addresses are kept | `[crates/midwest-census/src/sources/mshsl/map.rs]` (via registry comment) |
| Backfill | None — the rosters are current-season only | `[research/midwest/09-minnesota-mshsl.md]` |
| Moving condition | Open Q8 (validators) and the MN baseline question: the prior study's AD emails are not served by the MSHSL school page | `[sources/coach-directories-national]` open question 3 |

**Rank 7 — KSHSAA directory (already built; ranked for the run)**

| Field | Value | Evidence |
|---|---|---|
| Measured unit | 1 request ↔ 526 schools, each with AD name + AD email (489 KB); per-letter variants exist but are unnecessary | `[research/midwest/23-kansas-kshsaa-directathletics.md]`, `[crates/midwest-census/src/sources/ks/collect.rs]` |
| Validation | 526/526 rows agree on AD name and 526/526 on AD email against the baseline CSV; the negative control proves mutated strings fail | `[sources/coach-directories-national]` baseline validation |
| Delta key | Per-school `DateModified` at `directory/Id/{id}` | `[research/midwest/23-kansas-kshsaa-directathletics.md]` line 309 |
| Not collected | Sport coaches (0 rows), athletes, rosters, results; `ADCell`/`PrincipalCell` deliberately excluded as personal numbers | `[crates/midwest-census/src/sources/ks/]`, `[sources/coach-directories-national]` contact-scope contract |
| Duplicate channel | `PublicClassifications` (348 schools) duplicates class/enrollment already on the directory record | `[sources/state-assoc-plains]` §0.2 |
| Moving condition | Open Q12 (slice stability on a second pass) | §5 Q12 |

**Rank 8 — myOHSAA portal at scale (already built; the gap is coverage)**

| Field | Value | Evidence |
|---|---|---|
| Universe | 815 schools (816 `<tr>` incl. header) with 267 AAA / 267 AA / 267 A / 14 blank | `[research/midwest/evidence/gaps/45/ohsaa-enrollment.html]` via the pre-lane synthesis |
| Measured unit | 3 requests/school (`SearchSchool` → `ohsaaId`, `SportsInformation`, `AthleticDirector`); ~2,445 for the full state `[CALC]` | `[research/midwest/19-ohio-ohsaa.md]` |
| Yield on the sample | 5-school ticket: 18 coach rows (15 with email) + 13 AD rows (13 with email); 13-school stress ticket `SportsInformation?ohsaaId=…`: **56 cells → 33 named, all 33 with `mailto:`** (XC 18/28, T&F 15/28), 7 `TBA`, 16 `N/A` | `[research/midwest/19-ohio-ohsaa.md]` lines 505/506, `[research/midwest/30-cross-source-pareto.md]` line 186 (OH 15/18 coaches, 13/13 AD), `[sources/coach-directories-national]` |
| Why it matters now | OH has 31,708 Co2027 in the all-source census and only **456 with a coach** (1.4 %) — the largest coverage hole in the incumbent | `[reports/census-by-state.csv]` |
| Cross-key | `ohsaaId` ↔ Athletic.net team-name key; OHSAA policy mandates that every member has an AN team page | `[data/source-coverage-matrix.csv]` OH row |
| Moving condition | None open — the constraint is request volume, not access | §4.1 |

**Rank 9 — WIAA directory + tournament grade results (already built)**

| Field | Value | Evidence |
|---|---|---|
| Universe | 516 members; 1,796 team-season rows (462/462/437/435); enumeration needs 26 letter queries → 629 schools | `[data/source-coverage-matrix.csv]` WI row, `[research/midwest/06-wisconsin-wiaa.md]` |
| Measured unit | 1 request/school for the coach detail page; **17/17 coach rows with email** and 5/5 AD rows on the sample; earlier independent 9-school sample: 7–9 of 9 schools per role, 9/9 with at least one TF/XC email | `[research/midwest/30-cross-source-pareto.md]` line 184 (WI 17/17, AD 5/5), `[research/midwest/06-wisconsin-wiaa.md]` lines 446/512, `[sources/coach-directories-national]` |
| Result layer | WIAA publishes its own tournament AN MeetIDs + 104 result-file links/season with a `Yr` grade column; `wiaa_results` already parses them (13,573 rows in the incumbent) | `[data/source-coverage-matrix.csv]` WI row, `[reports/midwest-census-2026-09-20-evidence-mix.csv]` |
| Refresh cost | Full reads only (`no-cache, no-store`, no `Last-Modified`, no `ETag`, no update timestamp) | `[research/midwest/06-wisconsin-wiaa.md]` line 385 |
| Moving condition | None open | §4 |

**Rank 10 — Timer-index discovery layer (PrimeTime + DAT + TFRRS RSS + Wayzata)**

| Field | Value | Evidence |
|---|---|---|
| Measured unit | PrimeTime: 2 requests ↔ 353 events for the whole 2026 season (200 + 153 rows, `hasMore:false`); DAT: 1 request ↔ 946 upcoming meets across 49 venue states; wayzata: 2 requests per sport-year; TFRRS RSS: 1 request ↔ 75 latest results | `[sources/timing-providers-national]` §4/§1, `[sources/national-aggregators]` §1.5 |
| AN ids handed over | PrimeTime `computed.entryLinkHtml` embeds Athletic.net hrefs on 130 of 200 page-1 rows; ND's association page hands over 28 district AN MeetIDs from one request; MI MHSAA hands over 215 TF + 117 XC ids across 13 pages | `[sources/timing-providers-national]` §4, `[research/midwest/24-nebraska-nsaa.md]`, `[research/midwest/evidence/gaps/39/]` |
| File surface | PrimeTime 532 file links across 2026 (pdf 489 / htm 38 / html 5), immutable per publication; 88 of 353 events (25 %) have no result file at all — a coverage gap, not an error | `[sources/timing-providers-national]` §4 |
| Wayzata bridge | The adapter already retains `/links/<slug>` as `TimerMeet` keys and deliberately never fetches them; those slugs are the tenant keys for rank 1 | `[crates/midwest-census/src/sources/wayzata/mod.rs]` |
| Moving condition | Open Q10 (PrimeTime file-surface licensing) | §5 Q10 |

### 1.3 Where each adapter stands against the incumbent baseline

| Adapter | Rows/athletes the incumbent already holds from it | Marginal claim | Status of the claim | Evidence |
|---|---|---|---|---|
| AL blob/RTDB results | none — registry declares `athleticlive` as `meet_discovery` only, "No bulk_results: the harvest carries a `has_results` flag, not the result rows" | performances with grade + `ani` for 18,112 harvest meets | **unverified at athlete level** (Open Q1/Q2); the mark fields are verified on 136 rows | `[crates/.../registry/table.rs]`, `[sources/timing-providers-national]` §3/§16 |
| AN whole-meet | `athleticnet` bio adapter contributes the 43,229 AN-only Co2027 | full-meet rows + relay legs + divisions/rounds | endpoint list verified; result shapes verified anonymously for one meet | `[sources/athleticnet]` §16, `[reports/midwest-census-2026-09-20-athleticnet-marginal.csv]` |
| TFRRS IN | 0 | 1,861 Co2027 with independent grade | measured (2 requests) | `[sources/national-aggregators]` §2.20 |
| MileSplit | 142,569 athlete-records across 12 Midwest state tenants | the 34.1 % complement + 39 non-Midwest jurisdictions | complement is `[inherited]` (report 34); Michigan roster emptiness unresolved | `[reports/midwest-census-2026-09-20-evidence-mix.csv]`, `[data/milesplit-coverage-matrix.csv]` |
| IHSA | `association_school:ihsa` 15,114 coach records; no results | IL finals rows with `athleticNetId` | measured on entry lines; XC finals `athleticNetId` not yet captured | `[reports/midwest-census-2026-09-20-evidence-mix.csv]`, `[research/midwest/13-illinois-ihsa.md]` |
| MSHSL | 760 association school rows + 5,843 team-coach rows | MN grade + coach emails at 664-school scale | partially run | `[reports/midwest-census-2026-09-20-evidence-mix.csv]`, `[crates/.../mshsl/]` |
| KSHSAA | 526 | AD emails complete | wholesale-validated (526/526) | `[data/coach-contacts.csv]`, `[sources/coach-directories-national]` |
| OHSAA | 33 | ~815 schools of AD + coach emails | 5-school sample verified; full state not run | `[reports/midwest-census-2026-09-20-evidence-mix.csv]`, `[research/midwest/19-ohio-ohsaa.md]` |
| WIAA | 22 coach rows in evidence mix; 13,573 WIAA result rows | 629 schools of coaches + tournament grades | sample verified (17/17, 5/5) | `[reports/midwest-census-2026-09-20-evidence-mix.csv]`, `[research/midwest/30-cross-source-pareto.md]` line 184 |
| Timer indexes | 0 | new-meet discovery with AN ids | measured per endpoint | `[sources/timing-providers-national]` §4/§5 |

## 2. Ranked out — attractive sources that are not worth building

| Source | Lane verdict | Reason (one measured fact each) | Evidence |
|---|---|---|---|
| MaxPreps | REJECT | robots disallows `/school/ /team/ /discovery/ /careerprofile/ /m/team/ /m/school/` (191 rules); roster pages carry no athlete ids and no grade string | `[sources/national-aggregators]` §5 |
| AAU (`aausports.org`) | REJECT | 403 Cloudflare challenge to curl; no bypass attempted; no data captured | `[sources/national-aggregators]` §8 |
| World Athletics GraphQL | VALIDATION only | needs `x-api-key` + referer; `getAthlete(id:)` returns a placeholder record for any id — only `urlSlug` works; elite/international athletes only | `[sources/national-aggregators]` §6 |
| RunnerSpace / DyeStat | DISCOVERY / CONDITIONAL | 403 to curl (browser required), `Crawl-delay: 10`, **no ids of any kind** — join is name+school+meet text; universe is ~9 event microsites | `[sources/national-aggregators]` §4 |
| XCStats (CA) | REJECT | paywalled and `robots.txt` explicitly disallows ClaudeBot; no data plane captured | `[sources/timing-providers-national]` §12 |
| OpenTrack | REJECT | `robots.txt` itself sits behind a Cloudflare interstitial; 403 with curl UA and Chrome UA | `[sources/timing-providers-national]` §8 |
| MeetPro (DirectAthletics) | CONDITIONAL | 0 meets located; cost "cannot be estimated without a captured MeetPro meet" | `[sources/timing-providers-national]` §6 |
| RACE RESULT | CONDITIONAL | `my.raceresult.com` robots disallows `/RREvents`, `/RRPublish` — the result payload is off-limits; only metadata is reachable | `[sources/timing-providers-national]` §7 |
| Bound directory (IA, SD) | **retracted** | `robots.txt` = `User-agent: *` + `Disallow: /*directory`, `Crawl-Delay: 10`; the earlier 63 IA + 20 SD coach rows were browser fetches of disallowed paths and are withdrawn | `[sources/coach-directories-national]` retractions + `[sources/state-assoc-plains]` §0.3 |
| MSHSAA (MO) association surface | REJECT as crawler target | a second `user-agent: *` group closes with `disallow: /` → RFC 9309 group merge disallows the entire host; independently reproduced by the coach lane's evaluator | `[sources/state-assoc-plains]` §0.3.1, `[sources/coach-directories-national]` (MO = robots-disallowed) |
| MHSAA admin API (MI coaches) | NOT BUILDABLE | `my.mhsaa.com` robots disallows `/DesktopModules/`; the 14 AD rows in `[data/coach-contacts.csv]` are a legacy capture, not a refreshable adapter | `[sources/coach-directories-national]`, `[data/coach-contacts.csv]` (MI 14 rows, `my.mhsaa.com`) |
| ArbiterSports-backed directories (KY, OK, MA, MT) + MIAA | REJECT | publish `Disallow: /` on the delegated directory host | `[sources/coach-directories-national]` |
| `live.pttiming.com` | REJECT (host only) | robots names Claude 4× and states automated bulk extraction is prohibited by the Terms of Use; the `www` API + `data.pttiming.com` bucket remain usable | `[sources/timing-providers-national]` §4, `[research/midwest/22-missouri-primetime.md]` |
| MileSplit `/api/**` and `/rankings/**` | NEVER request | `Disallow: /api/` on every host sampled; the free rankings table renders **1 usable row** and 49 empty mask spans — the payload is absent from the bytes, not hidden | `[sources/milesplit-national]` §18 |
| Athletic.net anonymous rankings | Not a census channel | rows past rank 5 are masked (`blurAfterDepth: 5`); an anonymous page yields 5 usable rows, and the same filter reports 982 (anonymous) vs 2,117 (session) total | `[sources/athleticnet]` §16/§18/§22.3 |
| TFRRS national (college) surface | VALIDATION only | college athletes are out of census class; only FL/IN/NH have HS instances | `[sources/national-aggregators]` §2 |
| Cross Country Ratings | CONDITIONAL | 9-state XC-only DB with 240 runner links / 101 meet links — smaller than one MileSplit state | `[sources/national-aggregators]` §7 |
| NCSA, FieldLevel | REJECT (out of class) | recruiting paywalls; robots open but the product is profiles, not results | `[sources/national-aggregators]` §10 |
| USATF JO, TrackScoreboard | VALIDATION only | confirms AN meet ids (Athletic.net 644030 ↔ TrackScoreboard 14258) rather than carrying new rows | `[sources/national-aggregators]` §5, facts-of-record: JO XC crosswalk |
| KSHSAA `PublicClassifications` | Duplicate channel | 348 classified schools (31,885 B) duplicate class/enrollment already on the directory record the `ks` adapter parses | `[sources/state-assoc-plains]` §0.2, `[crates/midwest-census/src/sources/ks/]` |
| DirectAthletics HS results | REJECT as a Midwest result source | `hs-marked` = 0 for the WI/MN samples and "index page only (95 rows, 0 HS)" for WI; keep DAT for discovery only | `[data/dat-provider-coverage.csv]`, `[sources/national-aggregators]` §1.21 |

## 3. Zero-extra-request wins inside the existing pipeline

These are changes to code already running. The first five are data **already fetched and dropped**; the
last two are **duplicate work that can be deleted**. Each row names the symbol or column that proves it.

| # | Win | What the pipeline does today | What it should do | Evidence |
|---|---|---|---|---|
| 1 | **Stop paying one bio call per athlete × sport for a whole meet** | `athleticnet` builds its result channel out of per-athlete bio calls over `SCOPES = [TrackField, CrossCountry]` → **2 requests per athlete**, so the probe meet's 586-athlete union costs **1,172 requests** `[CALC]` | Pull `Meet/GetMeetData` once per meet, then `GetAllResultsData` with that token — 2 requests ↔ ~1,046 rows, vs **586×** the requests for the same meet `[CALC]` (the lane's pre-correction figure: 586 per-athlete calls vs 3 whole-meet requests = 195.3×) | `[crates/midwest-census/src/sources/athleticnet/mod.rs]` (`SCOPES`), `[crates/.../athleticnet/collect.rs]` (url built per athlete **per scope**), `[sources/athleticnet]` lines 301/325/332 |
| 2 | **Skip the third XHR** | Prior design assumed 3 requests per meet | 2 is measured sufficient for results; `GetEventDivisionData` adds only `isHurdle`/`FieldMeasureType`/`Type` | `[sources/athleticnet]` §22.9 (settled 2026-09-21) |
| 3 | **Persist `LiveID` on every meet** | Meet records carry `athleticnet_meet_id` but the AN→AthleticLIVE bridge field `LiveID` (73767 for meet 634313) is not retained | Store it: it converts an AL meet id into an AN meet id *without an ES query* | `[sources/athleticnet]` §19/§23(e) |
| 4 | **Don't re-derive `source_key` on the AL path** | AN rows are keyed `athleticnet:{athlete_id}-{IDResult}` | A whole-meet adapter must mint the **same** shape, or the same result observed via bio and via meet inserts twice | `[crates/.../athleticnet/absorb/rows.rs]` (both `store_tf_row`/`store_xc_row`), `[crates/.../athleticnet/map.rs]` (`store_performance` keys the map on the minted id) |
| 5 | **Use the harvest columns already on disk** | The 20,254-row harvest CSV already carries `tenant`, `athleticlive_meet_id`, `athleticnet_meet_id`, `has_results` | Filter `has_results=True` (18,112 rows, skipping 2,142) and go straight to the tenant named in the row — no discovery request per meet | `[data/athleticlive-midwest-2026-meets-all.csv]` (re-counted here) |
| 6 | **Delete the artifact duplicate of live coach adapters** | `coach_contacts` ingests a 2,298-row CSV whose `source_url` hosts are `api.ihsa.org` 16, `kshsaa-api.kshsaa.org` 526, `www.mshsl.org` 9, `ndhsaa.com` 39, `secure.nsaahome.org` 1,528, `officials.myohsaa.org` 31, `schools.wiaawi.org` 22, `www.gobound.com` 113, `my.mhsaa.com` 14 | Seven of ten source hosts are now covered by live adapters (`ihsa`, `ks`, `mshsl`, `plain_names`, `ohsaa`, `wiaa`); keep the artifact **only** for MI/IA/SD, or drop it — as-is every coach in those seven states is written twice from two evidence dates | `[data/coach-contacts.csv]` (host histogram re-counted here), `[crates/midwest-census/src/sources/registry/table.rs]` (`coach_contacts` + the live slugs) |
| 7 | **Never request roster filters that are client-side** | MileSplit `rosterFilterType`/`rosterFilterClass`/`rosterFilterGender` are applied in JS over rows already served | One `/teams/<id>/roster` request returns all sports/grades/genders (319 rows incl. 139 non-XC in the sample); a per-grade or per-season URL variant would triple requests for identical bytes | `[sources/milesplit-national]` §8, `[crates/midwest-census/src/sources/milesplit/fetch.rs]` (single roster URL, no query) |
| 8 | **Filter harvest sentinels instead of joining on them** | `athleticnet_meet_id` in the harvest contains `-1` ×118, `""` ×109, `0` ×6 | Coerce-or-reject before the join: 9,747 rows are non-empty, **one** of them is a test meet with id `0` (`TEST Sample Meet - Training 8/6`) | `[data/athleticlive-midwest-2026-meets-all.csv]`, `[data/athleticnet-meet-seeds.csv]` (checked in this document) |
| 9 | **Do not fetch MileSplit athlete profiles for marks** | The roster adapter retains `public_profile_urls` but never fetches them | Keep it that way: the profile's 96 result rows mask 480 cells and **0 carry text** — the payload is not in the bytes | `[sources/milesplit-national]` §8/§18.3, `[crates/.../milesplit/normalize.rs]` |
| 10 | **Do not build a per-provider meet map from the ES aggregator** | terms-agg on `tna` (timer name) fails on **250 of 256 shards** (`illegal_argument_exception`: text field without a keyword subfield) | Enumerate tenants instead (the inventory CSV already does it); a retry loop here burns requests for a deterministic error | `[sources/timing-providers-national]` §3 (mapping limits) |

## 4. Refresh design per adapter

Rule for every row: **the delta key is a source-published key, never a rank, a row number or a URL slug.**

| Adapter | Conditional GET / validators | Delta key | Cursor / cadence | Re-fetches history? | Evidence |
|---|---|---|---|---|---|
| **AL blob event doc** | **Yes** — `ETag` + `Last-Modified`, `If-None-Match` → **304, 0 bytes** (two independent probes), `Access-Control-Allow-Origin: *` | `ind_res_list/_doc/<eventId>`; doc carries `mi` (AL meet), `a.ani`, `y` | 1 request per event doc; blob per-document GET, no pagination | No — a 304 on an unchanged doc is 0 bytes | `[research/midwest/12-iowa-wayzata-results.md]` lines 340–342, 476–478, `[sources/timing-providers-national]` §3 |
| **AL Elasticsearch** | No validators (ES POST); cheap enough to treat as pull | meet-doc field `sdy` (e.g. `range: {sdy: {gte: "2025-08-01"}}`); tenant index name | `from`/`size` (2,000/page in the crate; `from+size` ≤ 10,000 → split); `_msearch` batches many tenants in one request | No — date-ranged queries only | `[sources/national-aggregators]` §3.4/§3.7, `[crates/.../athleticlive_athletes/mod.rs]` (`RESULT_WINDOW`, `PAGE_SIZE`, `MEETS_PER_BATCH`) |
| **AL RTDB standings** | No validators observed | `meet_<AL meet id>/liveRunStandings/<rui>` (`1-1`…`4-1` for a 4-race state XC) | 1 request per race; keyed by a meet id already in the harvest | No | `[data/source-coverage-matrix.csv]` ND row, `[sources/national-aggregators]` §3.5(d) |
| **Athletic.net** | **None** — `cache-control: no-store` on every captured API call; no `ETag`/`Last-Modified` on Team/GetDivChildren/GetNavInfo/GetRankings; the rankings list is a "season-bound best-mark snapshot" with no cursor | `IDResult` per row (already parsed); `MeetID`; `LiveID` | Deltas must be **client-side content digests**; the season map (`GetNavInfo`, 1 request per state) is the only cheap re-baseline | Yes by design: any refresh is a full payload transfer | `[research/midwest/01-athletic-net-team-universe.md]` line 367, `[research/midwest/02-athletic-net-profile-acquisition.md]` line 262, `[research/midwest/03-athletic-net-meet-acquisition.md]` line 235, `[research/midwest/04-athletic-net-rankings-discovery.md]` line 183 |
| **MileSplit** | No `ETag`/`Last-Modified` observed; the site publishes a `server-side-cache-ttl` and a footer `data-cacheKey="{meet:778860}:…"` | `MeetID` for meets, `RSID` for result sets, `AthleteID` for athletes (`column-grad-year` for grade) | `/results?season=&level=&year=&page=N` = 50 meets/page, date-desc → walk page 1 until a known meet id; `/sitemap.xml` (7,500 URLs/host) is a **change signal only**, `lastmod` is page-rewrite time not history | No — the index is date-ordered; rosters 2×/season | `[research/midwest/07-wisconsin-milesplit.md]` line 224, `[sources/milesplit-national]` §5/§7, `[research/midwest/33-milesplit-sitemap-recency.md]` |
| **TFRRS** | No validators; `robots.txt` is a 99-byte comment-only file; `/results.rss` is a rolling window of the latest results (75 items) | `AthleteID`, meet id `/results/<meet_id>/`, list id; `?year=JR` is a first-class filter | 1 request per list page per season; RSS as the weekly change feed | Season-scoped lists are archived (2010 reachable) — backfill is available but not needed | `[sources/timing-providers-national]` §5, `[sources/national-aggregators]` §2.17 |
| **IHSA** | `cache-control: public, max-age=180` (short) | `SchoolID`, tournament `MeetId`/doc index, per-finisher `athleticNetId` | Season schedule: universe 1 request; tournament index 1 request; per-event summaries as they appear | No — the API is current-season-first; historical tournament docs are indexed per season | `[research/midwest/13-illinois-ihsa.md]` lines 300/408 |
| **MSHSL** | Not verified (`unverified` — no header capture read in this synthesis) | `hostSchoolId`, team `nid`, `activityId`, `yearId` | 1 request per team per activity per season; **no backfill** (current-season only) | No — current season only, by design | `[research/midwest/09-minnesota-mshsl.md]`, `[data/source-coverage-matrix.csv]` MN row |
| **KSHSAA** | No validators observed | per-school **`DateModified`** via `directory/Id/{id}`; `OrgActID`; `ActivityYearID` | 1 request for all 526 in the name-search directory; per-school `DateModified` for the delta | No | `[research/midwest/23-kansas-kshsaa-directathletics.md]` line 309 |
| **OHSAA** | No validators; plain HTML + `mailto:` | `ohsaaId` (stable), derived from `SearchSchool` | 3 requests/school; rolling monthly slice (1/3 of 815) | No — coach churn is the only change | `[research/midwest/19-ohio-ohsaa.md]`, `[sources/coach-directories-national]` (OH = tier-1 with coach email) |
| **WIAA directory** | **None** — `no-cache, no-store` on conditional GET; no `Last-Modified`, no `ETag`, no update timestamp | `orgID` (stable, e.g. `GetDirectorySchool?orgID=1`); school node ids | 1 request/school; the archive is a per-season meet index | Yes — full reads are the only option | `[research/midwest/06-wisconsin-wiaa.md]` line 385, `[sources/state-assoc-greatlakes]` §6 |
| **NSAA S3 mirror** | **Yes at file level** — the S3 listing exposes `Last-Modified`/`ETag` | S3 object key under `/textfile/**`; district/state file names | 1 listing request + 1 per changed file; the school directory is a single 312-entry `<option>` list (1 GET) + 1 GET per school | No — object-level conditional refresh | `[sources/state-assoc-plains]` line 369, `[research/midwest/24-nebraska-nsaa.md]` |
| **MHSAA (MI)** | **Yes** — `cache-control: max-age=3600, public`, `last-modified`, `x-drupal-cache: HIT` | MHSAA numeric school id, meet-Info PDF names, 89 AN MeetIDs/season | ~12 HTML requests/season + PDFs | No — conditional GET is supported | `[research/midwest/15-michigan-mhsaa.md]` line 284, `[sources/state-assoc-greatlakes]` §4 |
| **PrimeTime** | No `ETag`/`Last-Modified`; file URLs are **immutable per publication** (hash names for track, epoch-ms slugs for XC) so new files appear as new URLs | event Supabase uuid inside `eventUrl` (**not** the numeric `id` — dual id space); file path `<uuid>/<epoch-ms>-<slug>` | `page`/`limit` + `hasMore`; 2 requests enumerate the whole 2026 season (200 then 153 rows) | No — the per-event file list grows, keyed by immutable URLs | `[sources/timing-providers-national]` §4/§16, `[research/midwest/22-missouri-primetime.md]` lines 149/200 |
| **Bound (IA/SD)** | n/a — **retracted** | n/a | n/a | Withdrawn for robots reasons; do not schedule | `[sources/coach-directories-national]` retractions |
| **DirectAthletics** | `cache-control: max-age=0, private, must-revalidate` **with an `etag`** → conditional GET is technically available | `meet_hnd` (string), meet `/results/track/<id>.html`, list `/lists/track/<a>_<b>.html` | 1 request = 946 upcoming meets (`fuseDriver_Upcoming.js`); results index 100 meets/page | No — the index is date-filtered (`with_date_from`/`with_date_to`) | `[sources/national-aggregators]` §1.16, `[research/midwest/14-illinois-directathletics.md]` lines 444–445 |
| **Wayzata schedule** | Host is UA-gated for `robots.txt` (CloudFront 403 to curl, 200 to a browser UA); the `/links/<slug>` pages are never fetched by design | `/links/<slug>` slug retained as `TimerMeet` key + `source_urls` | 2 requests per sport-year (track + xc) | No — the schedule is a per-season table | `[crates/midwest-census/src/sources/wayzata/mod.rs]`, `[sources/timing-providers-national]` §13 |

### 4.1 Weekly job set that follows from the table (requests are the measured units above)

| When | Job | Adapters | Requests per week `[CALC]` |
|---|---|---|---|
| Seasonal (Aug, Nov, Mar) | Roster/grade sweep | MileSplit rosters (6,633 Midwest), MSHSL rosters, TFRRS-IN lists | 6,633 + 1,328 + ~4 |
| Weekly | New-meet discovery | PrimeTime (2), DAT (1), TFRRS RSS (1), MileSplit `/results` page 1 per state (12), wayzata (2), association tournament pages (1–5 per state) | ~25–40 |
| Weekly | Result pull for touched meets | AL blob (1 per event doc, 304 when unchanged) or RTDB (1 per race); AN whole-meet (2 per meet) where no timer publishes | scales with meets: 100 new meets × 2 = 200 (AN) |
| Monthly (1/3 of the frame) | Coach refresh | KSHSAA (1), OHSAA (~815/3 ≈ 272), WIAA (~210 schools), IHSA (Δ per `max-age=180`), MSHSL (rolling) | ~500 |
| Never on a schedule | Provenance repair | `coach_contacts` artifact (frozen), SD/IA Bound (retracted) | 0 |

## 5. Open questions that flip a rank, each with its cheapest test

| # | Question | Current state | Cheapest test | Flips | Evidence |
|---|---|---|---|---|---|
| Q1 | Does the ES cluster expose `ind_res_list` as a searchable index (so event-id discovery is 1 request, not 1 rendered SPA page per meet)? | Unverified. Event docs were fetched by direct id (`…/ind_res_list/_doc/2254285`), and no capture shows how the id was obtained; the sibling `athlete_list` index **is** queried anonymously by the crate | 1 `POST https://search.athletic.live/ind_res_list/_search` with `{"size":1}` (or a `mi` filter). A 200 with a `mi`-bearing doc settles it; a 404/index-missing flips rank 1 ↔ rank 2 | **1 ↔ 2** (blob plane vs AN whole-meet) | `[sources/national-aggregators]` §3.5, `[sources/timing-providers-national]` §3, `[crates/.../athleticlive_athletes/mod.rs]` |
| Q2 | What share of AL result rows carry a non-null `y` (grade) and a non-null `a.ani`? | One doc verified (136 rows, 43 graded rows counted in the aggregator lane; `y` present on the whole sample) — network-wide rate unknown | 2 requests: one blob doc from a tenant whose meet has **no** AN meet id (the 15.8 % of 153,524), one from a linked tenant; count nulls | 1 (if grades are sparse, the blob plane's marginal Co2027 value falls below the AN whole-meet pull) | `[sources/timing-providers-national]` §3, `[data/athleticlive-tenant-inventory.csv]` (84.2 % with AN id) |
| Q3 | Is the Michigan roster emptiness a platform-wide anomaly or a MI-only artefact? | Report 27: 2 of 3 MI teams returned 0 athletes; not re-measured nationally | 15 requests: 3 teams × 5 states (MI, CA, TX, NY, IL) off `/teams` | 4 (a network-wide roster failure would kill the ranks-4 breadth claim) | `[sources/milesplit-national]` Open Q1, `[data/milesplit-coverage-matrix.csv]` MI row (`teams_sampled_with_nonempty_roster` = 1) |
| Q4 | Is MileSplit a *discovery* source or a *validation* source for the 34.1 % complement? | Inherited sample (283/579 loose, 300/579 alias-strict matches); no lane re-measured it | 100-athlete audit on the roster frame now measured (26,562 teams) — cheaper than before because teams are enumerated | 4 vs 2 (discovery ⇒ raise; validation-only ⇒ demote below IHSA) | `[sources/milesplit-national]` §19/§20 (`[inherited]` reports 27/34), `[data/milesplit-candidate-records.csv]` |
| Q5 | Does `athleticNetId` appear on IHSA **XC** result docs, not just T&F? | `athleticNetId` verified on T&F finishers and relay members; XC qualifier lists verified with grade but not with `athleticNetId` | 1 request to the IHSA tournament-results doc index for a 2026 XC finals doc | 5 (if yes, IHSA becomes the cleanest AN-id injector in the Midwest) | `[research/midwest/13-illinois-ihsa.md]`, `[data/source-coverage-matrix.csv]` IL row |
| Q6 | Is Indiana's 2026-27 provider still TFRRS, or does the MileSplit switch hold? | IN switched TFRRS→MileSplit in 2025-26; 2026 TFRRS IN coverage collapsed (56 track meets vs 1,022 in 2025) | 1 request to `indiana.tfrrs.org` `results_search_page.html?with_states=IN&with_sports=xc&with_year=2026` | 3 (a dead TFRRS-IN drops rank 3 to the MileSplit-IN row) | `[research/midwest/18-indiana-directathletics-milesplit.md]`, `[data/source-coverage-matrix.csv]` IN row |
| Q7 | Is the AthleticLIVE ES endpoint acceptable to the operator as a production channel (undocumented, anonymous)? | Lane verdict CONDITIONAL: anonymous `_search` returns 200 with no auth headers, but no published API and `robots.txt` on that host is an ES error body | 1 email to the platform owner; no request cost. Until answered, treat as CONDITIONAL and keep the 1 rps cap | 1 (a refusal removes the cheapest row channel in the study) | `[sources/national-aggregators]` §3.13/§3.18 (Open Q3) |
| Q8 | Do the msHSL pages or API send any validator (they are the only association surface in the top 10 without one)? | `unverified` — no header capture was read in this synthesis | 1 `curl -D` against `/api/team-data/schedule` for any MN school | 6 (validators ⇒ weekly coach deltas become near-free) | `[research/midwest/09-minnesota-mshsl.md]` |
| Q9 | Does the AN meet calendar exist as a servable endpoint (so new-meet discovery needs no timer index)? | `/events/usa/<state>/<date>` is a shell with 0 server-rendered ids; no API captured | 1 browser capture on `/events/usa/wisconsin/2026-09-19`, logging `/api/v1/**` calls | 10 (would make the timer index layer optional rather than load-bearing) | `[sources/athleticnet]` §22.2 |
| Q10 | Are the WI/MO PrimeTime **file** surfaces licensed for automated use? | `www.pttiming.com` has no robots policy (404); `live.pttiming.com` prohibits automated extraction in its ToU; report 22 already recorded the live-platform rejection | 1 email (`info@pttiming.com`); no request cost | 10 (a refusal removes ~353 events/season from discovery) | `[sources/timing-providers-national]` §4, `[research/midwest/22-missouri-primetime.md]` |
| Q11 | Does the `filters[subdomain]` parameter on MileSplit `POST /search/v2/athletes` actually narrow server-side? | `oh` returns only `oh` hits and `www` a mix — consistent with filtering, not proof | 1 request with `filters[subdomain]=ca` from a non-CA host | 4 (a name→AthleteID lookup that does not filter is unusable per state) | `[sources/milesplit-national]` §13/Open Q6 |
| Q12 | Is the KSHSAA `526/526` AD slice stable across a second pass (the 100 % validation rests on one pass)? | One wholesale pass, 100 % agreement on name and email | 1 request (the same `/directory/search/name/a/`) diffed against `[data/coach-contacts.csv]` | 7 (a drifting slice would demote the "1 request" claim) | `[sources/coach-directories-national]` baseline validation |

## 6. Proof boundaries — what this document does **not** establish

1. **No per-athlete marginal count exists for the AthleticLIVE result plane.** Meet-doc counts
   (153,524/153,774), one 136-row event doc and one algebra class are the entire evidence base; the
   "athlete-level marginal coverage" question is unmeasured in both national lanes (`[sources/national-aggregators]`
   Open Q1). Any number I could print here would be invention.
2. **No non-Midwest result density is proven.** Athletic.net's 51/51 jurisdiction *container* is verified;
   result density outside the Midwest is `[INFERENCE]` (`[sources/athleticnet]` §20).
3. **The school-universe figures for IN (413) and MI (755)** remain quoted-only: no retained capture
   re-produces them (the corpus member pages are SPA shells with 0 `<tr>`), as carried from the pre-lane
   synthesis.
4. **26 of 51 jurisdictions' coach directories are `unverified`**, because their tier-1 pages are
   client-rendered and the coach lane did not run a browser (`[sources/coach-directories-national]`).
5. **Every `[CALC]` cell in §1 and §4 is arithmetic on measured units, not a measured run.** The largest
   ones are the AN backfill (9,746 meets × 2 = 19,492 requests) and the OHSAA full state (815 × 3 = 2,445).
6. **The incumbent census numbers are a snapshot of the 2026-09-20 run**; adapters that have since been
   extended (`athleticlive_athletes`, `milesplit/*`) may have moved.
