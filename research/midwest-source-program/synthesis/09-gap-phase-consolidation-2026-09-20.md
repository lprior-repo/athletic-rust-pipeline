# 09. Gap-phase consolidation - reports 31-46 (2026-09-20)

**What this document is.** The continuation of `synthesis/08-gap-closure-2026-09-20.md`: the 16 gap-phase
follow-up reports that 08 announced as "in flight" have landed, and this file folds every one of them into one
place so a reader does not have to open 16 files. It does not replace 08 and does not contradict it except
where explicitly flagged in §6.

**Reports covered (16, all present):** 31, 32, 33, 34 (MileSplit: entries/timing, paywall boundary, sitemap
recency, roster-join pilot); 35 (WI), 36 (MN), 37 (ND), 38 (IL), 39 (MI), 40 (MO), 41 (KS), 42 (NE), 43 (SD),
44 (IA), 45 (OH) state follow-ups; 46 (offline event-token mapping). Report 44 (Iowa) is included - it landed
before this consolidation. Sources report files 31-46 live at `research/midwest/<n>-*.md`; raw captures at
`research/midwest/evidence/gaps/<n>/`.

**Observation window.** All live gap-phase probing is 2026-09-20 (session-local America/Chicago, roughly
09:04-09:25 CDT = 14:04-14:25 UTC, with per-report request ledgers carrying exact seconds); report 46 is
offline and its inputs are dated 2026-09-18 21:27 (delivery workbook mtime) and 2026-09-19 23:14
(nav-catalog mtime). 08's own re-verification work is 2026-09-20 and is not repeated here.

**Standing constraint, honored throughout.** No request was sent to any `*.athletic.net` host during the gap
phase. Every report states this explicitly (e.g. `research/midwest/35-wisconsin-gap-followups.md` header,
`research/midwest/39-michigan-gap-followups.md` header, `research/midwest/43-south-dakota-gap-followups.md`
header, `research/midwest/40-missouri-trxctiming.md` header, `research/midwest/37-north-dakota-gap-followups.md`
header, `research/midwest/38-illinois-gap-followups.md` header). Every Athletic.net id, URL or date index
quoted anywhere in this document was read out of a non-Athletic.net page or file, never fetched from
Athletic.net.

**Evidence discipline used here.** Every number below comes from a report I read or from an evidence file I
opened and quote; the citation is inline. Where a report's own figure could not be reproduced from the
evidence bundle, that is stated in place and, where the report figure rests on nothing I could open, marked
`[UNVERIFIED-CLAIM]`. Uncertainty labels the reports themselves applied (`[INFERENCE]`, `[UNVERIFIED]`,
bounds stated as bounds) are preserved, not upgraded. Section §6 lists the disagreements found while
consolidating.

---

## §1 Per-report digest

### 31. MileSplit `/entries` and `/timing` coverage beyond report 27 - six untested states plus Ohio

- **Question (register C4):** does the `/entries` and `/timing` coverage report 27 measured on 4 states hold
  on the six states it never tested (MI, IA, MO, NE, ND, SD) plus Ohio?
- **Verdict:** `/entries` is a discovery-only pre-meet surface with no grade, `/timing` is a timer
  *attribution/directory* surface rather than a data lane, and the result workhorse stays
  `/meets/<id>/results/<RSID>/raw`.
- **Measured:** 19 meets probed across 7 hosts, 80 requests, all HTTP 200 (per-host mi 13, ia 13, mo 13,
  ne 13, nd 13, sd 14, oh 1) - `research/midwest/31-milesplit-entries-timing.md` §Evidence appendix. Coverage
  now: entries measured on 9/12 states, `/timing` roots on 12/12 (OH 107 orgs, IN 79 orgs fetched by peers).
- **Grade is per-meet, not per-platform** (same §, table): grade fill on the sampled raw pages - MI
  `779299` 0/971 rows filled (Yr column blank), IA `776880` 194/195 = 99.5% with 50 grade-11, MO `773711`
  539/576 = 93.6% with 81 `JR`, ND `775054` 480 rows 100% with 65, NE `777602` 347 rows 99.4% with 33, SD XC
  `774965` 522 rows 100% with 75, SD outdoor state `765035` 3,027 rows with **no grade column at all** (145
  teams, 168 event sections, 798 relay leg rows carrying all four names).
- **Evidence:** `evidence/gaps/31/raw-stats.json` (ia/mi/mo/nd/ne/sd per-page stats),
  `evidence/gaps/31/sd-765035-state-raw-stats.json` (rows 3027 / teams 145 / relay_leg_rows 798 /
  grade_col absent), 80 raw page captures.
- **Remaining uncertainty:** the `/meets?year=2026` upcoming-meet index route was not probed, so pre-meet
  discovery without the results browser is unproven; entries visibility was measured only on meets that
  already have published results, so the base rate on the whole meet population is unknown; whether a meet's
  hidden/visible state can flip was not observed (one read per meet); OH/IN per-meet `/entries` behaviour is
  unmeasured (OH: one page probed; IN: none). The bundle's `raw-stats.json` records zeros for the SD outdoor
  state page while the dedicated stats file records the 3,027 rows - see §6.

### 32. MileSplit free/locked boundary (open question C5)

- **Question:** what exactly does the MileSplit paywall hide on an anonymous profile, and is there a free
  route to the same payload?
- **Verdict:** there is **no truncation** - the server renders one row per result the athlete has but strips
  every cell of every row (5 mask cells/row, all empty) and locks state rank in the PR table; the identity
  header, a PR table (mark + date + national rank) and the free `/raw` files remain usable.
- **Measured:** 14 profiles on 10 state hosts (`research/midwest/32-milesplit-paywall-boundary.md` §Athlete
  evidence): result rows 3-88; masks = 5 x rows (Dina 11 rows / 55 spans, Kingston 48 / 240, Pongonis 440,
  Danica 235); 0 rows without mask in 14/14; PR state rank locked on 100% of PR rows (6/6 ... 28/28);
  national rank free wherever `data-has-rank="1"` (14/14); "Show all PRs" expands client-side with no network
  request; `/raw` files fully unlocked 6/6 (WI x3, MN, IL x2, OH).
- **Load-bearing correction:** the earlier C5 note's "44 locked rows on Dina" is wrong - 44 is the count of
  the four non-seed mask *cells* on her page (4 x 11); the page renders **11** result rows and **55** mask
  spans total (`evidence/gaps/32/parsed-summary.json` profile `a01-wi-16319157-dina.body`: result_rows 11,
  mask_spans_total 55, mask_per_row 5.0).
- **Negative control:** Kingston Penn's real marks (`10.75`, `10.90`, `22.57`) exist in the free `/raw` meet
  file and appear nowhere in his profile HTML (480 mask spans), so nothing is hidden behind CSS.
- **Evidence:** `evidence/gaps/32/parsed-summary.json` (per-profile row/mask counts), the browser render
  capture, the verbatim mask markup and inline funnel JS quoted in the report.
- **Remaining uncertainty:** no athlete with >100 results was sampled - the "funnel path renders all rows"
  reading rests on the inline-JS comment, marked `[INFERENCE]` by the report; two CTA text variants exist
  (A/B) and both are upsells; the progression tab is a static example page with no data path to parse.

### 33. MileSplit sitemap recency semantics + re-pull harness (C6/C2)

- **Question (C6):** is the 7,500-URL sitemap a rolling recency window or a hard cap, and what can a collector
  diff against it?
- **Verdict:** the window **rolls** and is exactly "top 7,500 by `lastmod`" - so two pulls minutes apart being
  identical says nothing about the change rate, and one weekly pull is not a week of changes.
- **Measured:** 13 hosts (www + 12 states), each exactly 7,500 `<loc>`/`<lastmod>` pairs = 97,500 URLs, all
  `/athletes/`, all AthleteIDs distinct, 0 of 78 host pairs sharing an ID; the two 2026-09-20 captures are
  byte-identical (`sha256 5a05ca12f2dd91c0...`, 930,488 B); turnover over ~9 h 40 m - wi +4,460/-4,460/kept
  3,040 = **59.5%**, in +7,172/-7,172/328 = **95.6%**, mi +3,300/-3,300/4,200 = **44.0%**; window reach from
  1.6 h (OH) to 68 d (ND); rule test "old entries above the new floor must survive" - wi 2,969 eligible / 2
  missing, mi 4,196 / 1, in 0 / 0; no `ETag`/`Last-Modified`/`Age` on the sitemap response, `/sitemap_index.xml`
  404, `?page=2` byte-identical to the bare URL (918,197 B)
  (`research/midwest/33-milesplit-sitemap-recency.md` §Coverage/§Incremental use;
  `evidence/gaps/33/sitemap-window-summary-2026-09-20.json`, verified: www count 7500 / unique 7500 /
  reach_hours 2.02 / all_athletes true).
- **Evidence:** `evidence/gaps/33/sitemap-window-summary-2026-09-20.json`,
  `evidence/gaps/33/baseline-milesplit-sitemaps-2026-09-20T0905.json` (8.7 MB, 13 x 7,500),
  `tools/gaps/milesplit_sitemap_recheck.py`.
- **Remaining uncertainty:** which hosts lose how much of an oversized nightly batch is `[INFERENCE]` (batch
  size is not observable from the sitemap itself); the register row's full closure still needs the 7-day
  recurrence check; cadence advice is per host class because `www`/`oh`/`in`/`mo` windows are pinned to the
  nightly batch.

### 34. MileSplit team-roster enumeration vs the delivered Athletic.net corpus (C2/C3 pilot)

- **Question:** how much does a MileSplit roster sweep add beyond the delivered corpus, and does a
  name+school+class join actually land on corpus identities?
- **Verdict:** DISCOVERY-ONLY with a secondary validation role - roughly half of roster-registered Class-of-2027
  boys are not in the corpus, the corpus cannot see girls at all, and it exposes no Athletic.net ids.
- **Measured (single source of truth):** frame WI 597 + MN 592 teams, systematic every-23rd draw -> 50 teams,
  50/50 HTTP 200, 4,883 athletes all grades, **1,029 Co2027 (579 boys / 450 girls)**, 38 teams with Co2027;
  strict match 268/1,029 = 26.0% (CI 23.5-28.8), loose 296 = 28.8%; boys loose 296/579 = 51.1% => beyond
  corpus 283/579 = **48.9%** (report's conservative bound 46.5-51.8%) with 269/283 names absent nationally;
  XC-only boys 68/76 = 89.5% beyond (CI 80.6-94.6), no-season-flag 41/41 = 100%, outdoor-flagged most covered
  at 65.3%; girls 0/450 vs 230.1 expected if the corpus were mixed-gender; cost 1 request/team, 1,189
  requests for the WI+MN frame
  (`evidence/gaps/34/final-tables.md`, `evidence/gaps/34/join-metrics.json` - verified: teams 50, athletes
  4,883, Co2027 1,029/579/450, teams_with_co2027 38, corpus rows 142,705).
- **Evidence:** `evidence/gaps/34/final-tables.md`, `join-metrics.json`, `name-presence-check.txt`,
  `rosters/{wi,mn}/*.html` (50 files), `school-alias-candidates.json`, `repo-corpus-probe.txt`.
- **Remaining uncertainty:** the join is against the delivered corpus snapshot, not live Athletic.net (hard
  constraint), so "beyond corpus" is not "not on Athletic.net"; the sample is systematic, not random, and
  covers 2 states only; the loose tier is the liberal bound (28 loose matches ignore a school mismatch); 4 of
  4 independently repo-matched boys landed on the same AthleteID, which the report offers as join validation.

### 35. Wisconsin WIAA - gap follow-ups (coach change-feed, 2026 track-PDF extraction, timing-provider links)

- **Question:** (1) is there an updated-since signal for the WIAA coach directory; (2) are 2026 RUNMEET-style
  track PDFs text-extractable; (3) which provider links do WIAA tournament pages carry?
- **Verdict:** no change feed of any kind exists (poll-per-school is the only route); PDF extractability is
  **file-specific** with an installed OCR fallback, correcting report 08's blanket "not text-extractable";
  WIAA's pages are not the provider index - its instruction PDFs and RunMeet-printed files are.
- **Measured:** 2026 T&F file union **134 links** (boys 103 + girls 97, union 134 = 131 `.pdf` + 3 `.htm`); a
  26-file sample gave **24/26 extractable (92.3%)**, 2 producer-side failures (0-font raster; Type 3 without
  ToUnicode); **2,786 grade-11 rows** across the 24 extractable files (`tr2026homesteadsectional.pdf` 69,
  `tr2026kielregionalindiv.pdf` 182, `trb2026stratfordregional.pdf` 133, `tr2026pittsvilleregional.pdf` 218);
  `pdftotext` 0.04 s/file; tesseract 5.5.3 at 200 dpi = 1.06 s/page (raster file end-to-end 11.4 s; Type-3
  file projects to ~35 s); coach page 181,100-218,703 B with `no-cache, no-store` and **no validators** (a
  conditional GET returns the full body), 516-school sweep ~103 MB / ~5.3 min, rolling 10/week ~1.9 MB;
  `/Reports/` 403; `RunSportListReport` contains 0 `Coach` strings; 28 WIAA pages scanned: **0/28
  `athletic.net` anchors**, 2/24 timer links (`live.pttiming.com/xc-ptt.html?mid=5127`), 5 distinct
  Athletic.net URLs inside two instruction PDFs (incl. previously unknown XC date indexes
  `.../events/usa/wisconsin/2026-10-23` and `.../2026-10-24`); 13/26 result PDFs are RunMeet exports, 2/26
  print an Athletic.net URL; `wistca.ihigh.com` DNS failure, `wisconsinrunner.com/wccca/` -> 404
  (`research/midwest/35-wisconsin-gap-followups.md` §Athlete evidence/§Recruiting information/§Result
  evidence; `evidence/gaps/35/pdf-extract/`, `ocr/`, `pdfs/`).
- **Remaining uncertainty:** one girls-archive PDF link is broken and was substituted by its boys counterpart -
  a stated probe gap; OCR output noise (marks treated as *reported*, grade/name as the primary claims); the
  season-wide OCR projection (~10 files / ~136 pages / ~2.5 min) is `[INFERENCE]`; no association-level coach
  feed exists, so the rolling-poll design is the ceiling.

### 36. Minnesota MSHSL - gap follow-ups (2025 XC PDF extraction, prior-year grades, grade-availability map)

- **Question:** close the MN register row: are the 2025 XC PDFs raster-only (OCR or skip), and are MSHSL
  participant nodes enumerable for prior years?
- **Verdict:** the 2025 XC files are **outline-flattened text, not scans** and OCR recovers them in seconds;
  MSHSL publishes **no prior-year athlete rosters at any endpoint**, but prior-year graded participation is
  recoverable from state result PDFs plus the AthleticLIVE `athlete_list` roster.
- **Measured:** `pdftotext` returns 0 words on all four 2025 XC files; censused on every file - 0 `BT`/`Tj`/`TJ`
  operators, 0 embedded fonts, 8 logo images, path-op counts 114,764-125,212 `l` / 91,429-96,943 `c` /
  313-329 `re`; OCR (300 dpi, psm 6) of the 2-page Class AAA boys file: 1,747 words, **156 grade tokens**,
  ~5.0 s; 142/159 printed places recovered (89.3%); of 149 unique (name, grade) records, 142 (95.3%) match the
  AthleticLIVE roster exactly and 147 (98.7%) at fuzzy >=0.86 (a strict full-row parse scores 102/143 = 71.3%,
  which the report calls parse noise); AL rosters - state XC 2019 845, 2021 1,177, 2022 1,181, 2023 1,167,
  2024 1,218, 2025 1,143 rows (2018 indexed with 0 rows), state T&F 2023 2,980, 2024 3,308, 2025 3,029, 2026
  2,473; free Athletic.net joins - state T&F `ani` 2022 475179, 2023 516328, 2024 563571, 2025 613966, 2026
  666673, XC athlete ids 2025 1,141/1,143 (99.8%), 2024 1,025/1,218 (84.2%), 2023 0/1,167, team `t.ani`
  2024/2025 100%; the 2026 T&F **program** PDF is a graded surface - 1,309 records / 276 schools / grades
  12->572, **11->415**, 10->240, 9->82, names resolving 1,309/1,309 against the `mi=74652` roster and grades
  agreeing 1,303/1,309 (99.5%); prior-year roster routes fail - 4th path segment 404, `?year=695` 200 but
  ignored (byte-identical), `/api/mshsl-state/roster-611-150` is `is_empty:true`
  (`research/midwest/36-minnesota-gap-followups.md` §Headline answers/§Athletic.net leverage/§Result evidence;
  `evidence/gaps/36/xc300-*.txt`, `es-count-mi-*.json`, `es-mshsl-state-xc-all.json` - verified: 7 state-XC
  docs).
- **Remaining uncertainty:** the 2025-26 T&F and pre-fall-2025 XC per-athlete rosters are gone from mshsl.org;
  OCR row recovery is partial by design (report recommends treating marks as reported); one 2026 finals link
  (`.../2026-06/2026-track-and-field-class-aaa-prelim-results.pdf`) is a 404 while the other five links are
  200, so fetch-and-verify is mandatory; an earlier "1,465 grade+school pairs" figure in the 2025 T&F PDF was
  not reproducible and the report replaced it with the verified 875 lines.

### 37. North Dakota gap follow-ups (NDHSAA qualifier lists, coach sample, `dakota` tenant, member universe)

- **Question (report 25's items):** do official ND surfaces publish qualifier/participant lists with grade,
  coach emails, and how much of the ND meet universe carries Athletic.net ids?
- **Verdict:** NDHSAA state-meet program PDFs carry grade, NDHSAA is a coach-**name** source with 0 emails on
  33/33 sampled pages, and the ND meet universe is **96.3%** `ani`-covered with zero Athletic.net requests.
- **Measured:** meet census - 875 docs -> **437 unique meets** (outdoor 288 / indoor 105 / xc 44), **421/437 =
  96.3%** with non-null `ani` (per index: live_results 421/437, athleticlive 239/244, heros 165/175, wayzata
  17/18, rpmtiming 1/1), 16 exceptions listed; state outdoor TF 2026 = `i 73476` / **`ani 652729`** ("NDHSAA
  Class A & B State Meet (2026)", also served by RTDB `meet_list/73476.json`, 200, 4,826 B, 20 events);
  2026 state XC **not yet indexed** (0 ND docs in `md` 2026-10-15..2026-11-10, re-confirmed twice);
  `dakota_meet_list` = 916 docs, 0 North Dakota (SD 622 / IA 253 / NE 31 / WY 2 / OR 1 / blank 7); NDHSAA =
  169 member high schools with team-universe counts XC-B 100, XC-G 96, TF-B 113, TF-G 114; 11 new schools
  sampled, **44/44 coach slots named, 0/33 cumulative pages contain any email**; NDHSAA `/programs` = 286
  state program PDFs, track program 20,055,554 B / 42 pp / 1,766 standalone grade-number lines
  (`research/midwest/37-north-dakota-gap-followups.md` §Athletic.net leverage/§Recruiting information/§Result
  evidence; `evidence/gaps/37/nd-census-1000.json` - verified: 875 docs, 837 with numeric `ani`).
- **Remaining uncertainty:** a coach-level lookup in the Grand Forks district directory returned 0 rows
  (search is AJAX) so coach-specific GFPS coverage is `[UNVERIFIED]`; Carrington's district host is a "Client
  Challenge" interstitial and was abandoned; the NDHSAA state pages' "PERFORMANCE LISTS (ATHLETIC.NET)" labels
  are pre-season placeholders with no hrefs today, so the state-meet page needs a re-check after the 2026-10
  meets.

### 38. Illinois follow-ups (IHSA girls XC tournament ids, coach email coverage, DirectAthletics recency)

- **Question:** report 13/14's three open items - resolve girls XC tournament ids, quantify IHSA coach
  coverage on a fresh sample, and settle DirectAthletics' IL recency.
- **Verdict:** girls XC ids are **691/692/693** (verified three ways); IHSA `staff2` is a viable coach
  directory **only against an "entered the sport" denominator**; DirectAthletics' IL surface is **frozen at
  2023-24**.
- **Measured:** girls 2025-26 XC qualifier rosters **1,177 athletes, 318 grade 11** (boys 1,214 / 358 =>
  2,391 qualifiers and 676 grade-11 per season); coach sample 24 schools with per-role denominators - boys T&F
  15/24 name+email -> **14/14** among entered schools; girls T&F 15/24 name, 14/24 email -> 13/14 and 12/14;
  boys XC 11/24 -> 9/10; girls XC 11/24 -> 8/9; both AD roles 23/24 -> 16/17; activities director 9/24 ->
  7/17; 12 email-reveal calls all 200 with non-empty addresses, the one sampled `HasEmail:false` row returned
  `{"email":""}`; 2 of 12 revealed addresses are `gmail.com`; DA newest IL lists `/lists/track/1467_4524.html`,
  `1468_4525.html`, `1469_4526.html` (Jan-Mar 2024) and championships `/results/track/84224.html`,
  `84225.html` (2024-03-22/23), while the global list-id sequence ran 4867-5350 in 2025 and up to 5775 in
  2026 - so "any IL list id > 4526" is the resumption detector
  (`research/midwest/38-illinois-gap-followups.md` §Coverage/§Recruiting information/§Recommendation;
  `evidence/gaps/38/ledger.jsonl` + payload JSON/HTML).
- **Remaining uncertainty:** classification rows, entered teams and published coach rows are three different
  sets (examples recorded, e.g. Elmhurst IC Catholic lists 1 of 4 roles; 7 of 24 sampled schools sponsor
  neither sport, so all-school rates are mostly non-sponsorship); two withheld/absent cases are named; the
  headless-Chromium load confirmed the girls XC page's own XHRs (`=691`, then `=692`/`=693` on tab clicks)
  made zero Athletic.net requests.

### 39. Michigan follow-ups - MHSAA UP girls finals naming, MITS indoor archive, MHSAA -> Athletic.net links

- **Question (reports 15/16 items):** official names for the MHSAA UP girls finals, what is public in the
  MITS indoor archive, and how many MHSAA pages link Athletic.net meets.
- **Verdict:** MHSAA HTML is a **direct Athletic.net meet-link source** (215 TF + 117 XC distinct ids, link
  extraction only), and MITS is a second independent id source with both the AthleticLIVE meet id and the
  Athletic.net MeetID for all 17 current-season meets.
- **Measured:** UP girls finals artifacts exist for 2021-2026, grade present 2024-2026 and absent 2021-2023;
  link inventory per page family - TF hubs 13 hrefs / 5 distinct ids each, TF regional pages 48/48/49/49,
  TF archive 79/78 anchors -> 52 distinct (20 `/meet/` + 36 `Print/EntryMeet`), XC regional 36/37/36/**0**
  (the 2026 XC page currently has zero regional links), XC archive 8 each; union **215 distinct TF meet ids,
  117 XC meet ids, 36 legacy `Meet=<id>` ids** (4 ids in both forms); MITS - michiana index holds 20 meets
  2019-12-14 -> 2026-02-14, `live_results_meet_list` holds 66 MITS-named meets, the current season is **17
  meets (2025-12-06 -> 2026-02-27)** each carrying `i` and `ani` (table in report; verified count in
  `evidence/gaps/39/es-mits-season-2526.json`: 17 hits); an event from 2019-12-14 is still served from blob
  storage (`Last-Modified: Wed, 21 Aug 2024`); MHSAA `robots.txt` disallows only `/core/`, `/profiles/`,
  `/README.md`; Firebase RTDB rejects HEAD with 405 and has no ETag revalidation
  (`research/midwest/39-michigan-gap-followups.md` §Coverage/§Athletic.net leverage/§Athlete evidence/
  §Result evidence).
- **Remaining uncertainty:** exact Athletic.net request savings are `[INFERENCE]` (the id count is exact: 17
  meets / 20 tenant meets / 66 docs); MITS grade is 100% on GVSU MITS #5, MITS 4 and 2025 CMU MITS but partial
  (~22-31%) on AQ MITS #2 2026 and absent on michiana events dated <=2025-12-13; the 2026 XC page must be
  polled during the season because its regional links appear per contested meet; `mitca.org` offers no
  coach/AD directory beyond the pages fetched.

### 40. Missouri TRXC Timing deep map

- **Question:** is TRXC Timing - the non-Athletic.net MO provider flagged by report 22 - a viable
  grade-bearing Missouri result source?
- **Verdict:** **yes** - RESULT-SOURCE with a validation role for the grade column and a discovery-only role
  for its upcoming-meet Athletic.net ids; the one open question that could flip it is licensing (the results
  tree publishes no terms).
- **Measured:** page inventory - 132 MO T&F meets in 2026, 111 in 2025, 127 in 2024, plus 55 MO XC meets per
  season and a 2010-2025 MSHSAA XC district archive (650 files); the upcoming-meets page carries 39
  Athletic.net anchors in 39 of 40 rows, **37 with meet ids** (36 XC `/info` links + `670780` `/register`),
  quoted verbatim in the report; result index pages carry **0** Athletic.net links; grade - `Year = 11` is
  printed per row, with a measured 1,933 grade-11 lines across the 11 HS T&F files the report lists (mean
  ~176/file, range 72-276) plus 18 in the Festus XC file; per-file examples: Henle Holmes 228, MSHSAA
  C4S2/C5S2 sectional 240, C2D2/C3D2 district 276, CMAC 225, Suburban Green Girls 119; MSHSAA re-check - its
  TF InformationCentral page links only PrimeTime Timing and Athletic.net, XC page links no external provider,
  and no MSHSAA page links TRXC; `robots.txt` 404 on both hosts, no sitemap, the RSS feed is empty (0 items),
  the 2018 privacy policy covers event registration only; archive.org returned 429 and the probe stopped
  (`research/midwest/40-missouri-trxctiming.md` §Athletic.net leverage/§Athlete evidence/§Recommendation;
  `evidence/gaps/40/live_*.txt`, `evidence/gaps/40/strict-stats.json` - see §6 for the recount difference).
- **Remaining uncertainty:** no terms-of-use exists for the result tree, which the report explicitly says is
  *not* a licence and recommends a licence inquiry before weekly sweeps; identity must be resolved on
  (name, school label, grade) with Hy-Tek school abbreviations aliased; collectors must filter club meets
  (print `Age`, not `Year`) and college rows (FR/SO/JR/SR), and tolerate the pre-~2019 class-code era; the
  season-wide totals (132 + 55 file fetches) are `[INFERENCE]`.

### 41. Kansas gap follow-ups (report 23 open items)

- **Question:** the four items report 23 left: is TF grade reachable anywhere in the KSHSAA championship
  system, what grade-bearing sources exist beyond champion rosters, do fresh schools still yield AD coverage,
  and what is the official classification structure?
- **Verdict:** the KSHSAA TF championship system exposes **no grade at all** (`IndividualResultsSectionFieldApplicable_Grade=false`
  at the activity level, for boys, girls, indoor and the 2015 season alike), but four other sources do - Kansas
  is **no longer a grade-poor state**.
- **Measured:** TF boys `activityID=51` 200 / 235,224 B / 864 rows with **864/864 `Grade: null`**; girls `50`
  873 rows / 0 graded; 2015 `51` 850 rows / 0 graded; the per-class view is byte-identical (235,224 B); the
  by-school view (3,815 B) has no grade column; indoor `68` 1,508 B / 0 rows; control XC boys `57` flag `true`
  with **143/143 rows graded, 50 grade-11**; champion rosters are first-place only (boys 18/25/29/15/15/12 =
  114 athletes, girls 18/23/30/20/11/7 = 109, each graded). Grade-bearing beyond champions: AthleticLIVE
  `athlete_list` - **842 distinct Kansas meets** (232 XC / 520 outdoor / 90 indoor, 2015-2026, 12 tenants),
  **354,733 athlete rows**; TF outdoor 2026 = 107 meets / 42,023 rows (grade-11 4,793, grade-12 4,063, 88.9%
  carrying an Athletic.net athlete id); XC 2026 = 21 meets / 5,882 rows (100% graded, grade-12 651); XC 2025 =
  45 meets / 11,132 rows (grade-11 1,637); 2022 state meet present but its grade column is empty/NBSP, i.e.
  coverage is meet-config-dependent. Midwest Timing PDFs - performance-1A 546 graded rows (11:164),
  performance-6A 544 (11:184), relay heatsheet **4,336 graded leg tokens across 36 relay events, grade-11
  1,192**; MileSplit KS rosters - 413 team records, Olathe North 235 athletes {2027: 86} and St. Thomas
  Aquinas 150 {2027: 61} = **147 Co2027 from 2 requests**; DirectAthletics `100887` 17 row tokens. Coaches -
  12/12 fresh schools AD name+email, whole directory 339/339 AD name+email and 329/339 website, **no coach
  field anywhere** (a known coach's surname search returns `[]`), and `DateModified` shows 152/339 (44.8%)
  last modified before 2023. Classification - 348 schools: 6A 36 / 5A 36 / 4A 36 / 3A 64 / 2A 64 / 1A 112,
  9 schools moved up and 9 down (fetching `?year=2025` or `?year=2027` returns the same 2025-26 payload)
  (`research/midwest/41-kansas-gap-followups.md` §Answers; verified in
  `evidence/gaps/41/derived-counts-summary.json`: 842 meets, 560 with an AN meet id, 354,733 rows, 348
  schools, 339 directory records, TF champion rows 864/0 and 873/0 graded, XC 2025 143/143 graded with
  grade-11 50).
- **Remaining uncertainty:** whether Bound can serve Kansas grade rosters is unresolved (roster rows are
  JS-loaded, no public JSON visible in the page source) - not needed, as three other sources answer it; the
  relay grade-token count is a strict-regex count with an allowed +/-~0.5% line-wrap tolerance; the 2022
  AthleticLIVE state meet grade column is empty for every row.

### 42. Nebraska gap follow-ups (report 24 open items)

- **Question:** (1) do NE school-district sites publish professional coach/AD emails and at what cost,
  (2) do NSAA S3 files expose grade on **live 2026** files, (3) which timing/result hosts do NE meets use?
- **Verdict:** split - S3 finals and district files carry grade on every row (verbatim proofs), the coach
  email layer is a district-site job with a 25% role-matched yield, and NE meets hand over Athletic.net ids
  almost universally in recent data.
- **Measured:** ~106 requests total (school sweep 50; `search.athletic.live` 12; S3 13; nsaahome 2; striv 5;
  prepcast/DDG probing ~19; timer hosts 9), no host above 12 requests, no 429/no CAPTCHA/no auth prompt, two
  recorded access findings; ES - 4,520 docs / **2,203 distinct meets** / 130 distinct timers / 34 distinct
  hosts, by sport outdoor 3,043 / xc 994 / indoor 483 (verified in `evidence/gaps/42/es_ne_final_aggs.json`);
  `ani` - the 60 most recent NE meet docs (2026-06-17 -> 2026-10-15) are **60/60** with a numeric `ani` (the
  report labels this a sampled window), `anis` link arrays on 98/528 unique docs, and whole-corpus coverage of
  419/575 = **72.9%**; avoided meet-id resolution ≈2,200-6,600 Athletic.net requests; grade on live 2026
  files - `abresults.pdf` 1,395 rows (11:457), `cdresults.pdf` 1,406 (11:419), `abres26.pdf` 395 (11:117),
  `agres26` 386 (11:135), with verbatim rows and relay legs carrying per-leg grades; `unifiedresults.pdf` is
  the exception (no grade, no points); XC district HTML is still the 2025 object
  (`Last-Modified: Thu, 16 Oct 2025`), cells like `J'Shawn Afuh (11)`; NSAA directory re-read: 1,085,584 B
  containing exactly **one** `@` string; 16-school sweep - **4/16 (25%)** produced a role-matched coach/AD
  email (8 addresses), 8/16 (50%) published at least one staff email in plain text, 9/16 (56%) including
  Elkhorn's Cloudflare-obfuscated contacts (Apptegy embedded JSON and `data-cfemail` decoding, neither an
  access-control bypass); Millard West origin 403; timer contacts come free from ES `qe` (219/528 docs, 19
  distinct addresses)
  (`research/midwest/42-nebraska-gap-followups.md` §Athletic.net leverage/§Recruiting information/§Result
  evidence; `evidence/gaps/42/live_abresults.txt`, `live_cdresults.txt`, `es_ne_recent.json`,
  `s3_headers_2026-09-20.txt`).
- **Remaining uncertainty:** the 60/60 figure is a sampled window, not a whole-corpus claim; the full-sweep
  extrapolation (~900-1,000 requests -> ~70-80 role-matched emails) is `[INFERENCE]`; the prepcast/Striv lead
  hunt ended without locating a results host; the 2026-season XC result surface is regular-season only until
  the district files rotate on 2026-10-15; MaxPreps feeds carry no grade.

### 43. South Dakota gap follow-ups (co-op naming, SD result sources, school universe)

- **Question (report 26's items):** how does SDHSAA name co-ops and how do non-matching Athletic.net team
  labels map; what actually carries SD TF/XC results given Bound has none; what is the official school
  universe?
- **Verdict:** AthleticLIVE tenant `dakota` is the SD result feed **and** grade oracle - the only SD surface
  with per-athlete results, a grade field and Athletic.net ids for XC and TF alike; Bound carries no SD T&F
  results; the co-op/naming rule resolves 188 of 207 non-identical labels.
- **Measured:** 622 SD meet docs (374 XC / 464 outdoor / 78 indoor overlapping; 313 dated 2026, 279 2025, 69
  with `sdy` >= 2026-09-01); `ani` present on **617/622 = 99.2%** (`evidence/gaps/43/es-dakota-ani-coverage.json`
  - verified: 622 docs, 617 with `ani`), 4 docs with no `ani` listed; Dakota Timing archive 1,688 events with
  **1,120 located ", SD"** across 138 venues, 452 exposing a `results.phase2in.com` viewer; SDHSAA yearbook
  PDFs - B-XC 379 name/grade rows (11:102), B-TF 341 rows under a strict regex (99 grade-11 tokens; a
  report-26-style parse gives 1,299 rows / 392 grade-11 as an upper bound), G-TF 1,035 rows; top-performance
  sheets - 40 tabs, every tab `#N/A` off-season ("Last Updated: 6/03/2026"); MileSplit SD - 220 team rows /
  199 team links; team-label mapping - 207 rows resolve as **161 EXACT-OFFICIAL + 11 MEMBER-MASCOT + 16
  LEGACY-COMPONENT = 188** (plus 8 NON-MEMBER, 2 AN-PSEUDO-TEAM, 9 CLOSED/MERGED) (`jq` over
  `evidence/gaps/43/sd-an-team-mapping-final.json`: EXACT-OFFICIAL 161, MEMBER-MASCOT 11,
  LEGACY-COMPONENT->CURRENT 16, NON-MEMBER 8, AN-PSEUDO-TEAM 2, CLOSED/MERGED 9, total 207); Bound check -
  the SDHSAA state-TF scores page returns 200 with an **empty Result column**; coach layer - none of the five
  SD surfaces adds a coach directory
  (`research/midwest/43-south-dakota-gap-followups.md` §Coverage/§Athletic.net leverage/§Result evidence).
- **Remaining uncertainty:** the report records an unexplained 1-doc difference between the two `ani`
  framings (617 vs 618), marked `[INFERENCE]` (a doc whose value is present but not indexed for `exists`);
  co-op guest attribution is `[INFERENCE]` - the feeds credit the host team, and re-attaching the guest school
  from the classification edge must be tested on one real co-op meet before pipeline use; yearbook loose-parse
  counts over-count (the report offers only the two boys PDFs as clean grade evidence).

### 44. Iowa gap follow-ups (IHSAA 24-school resolution, coach coverage beyond Bound, 2026 timer delta)

- **Question (reports 11/12 items):** resolve the "24 IHSAA members with no Bound T&F team", find coach
  coverage beyond Bound, and capture the 2026 timer delta.
- **Verdict:** the 24 resolves to an exact rule (33 members lack a name-identical Bound boys-T&F team, split
  346 / 10 / 18 / 5 across all 379 members); Iowa's official association directories publish **no school
  coach emails at all**; and the 11 AthleticLIVE timers close the 2026 delta.
- **Measured:** 379-member resolution - **bound-name match 346, alias 10, co-op guest 18, no-program 5**
  (`evidence/gaps/44/out-resolution.json` - verified exactly); report 11's 8 named examples decompose as 4
  variants + 3 co-op + 1 no-program, which is why the earlier figure was approximate; 2026 Iowa calendar -
  768 docs = 630 spring T&F + 138 XC, **758 unique Athletic.net MeetIDs from one anonymous request** (99% of
  docs carry `ani`), the cheapest AN meet-id recovery route found in the campaign; the 5 program-less schools
  would otherwise cost ~60 avoided team-resolution requests per season; coaches - IHSAA's contact page says
  "Find School Contacts at Bound" and yields 0 school coach emails, IGHSAU yields 0 (its three sitemaps 404,
  `robots.txt` allow-all 24 B), IATC lists **163 member schools** (1A 69 / 2A 37 / 3A 33 / 4A 24) and an
  advisory board of 9 named people with 0 emails; 11 district probes produced **1 head-coach email + 7
  role-labelled AD/activities addresses** (Cedar Falls 2, Waukee 4, Dubuque 1) from six directories that
  publish email at all (Cedar Falls directory itself carries 878 addresses, Waukee 176); Bettendorf served a
  3,036 B "Client Challenge" page; timers - 11 providers missing from report 12's census are AthleticLIVE
  publishers whose docs carry `ani`/`a.ani`
  (`research/midwest/44-iowa-gap-followups.md` §Athletic.net leverage/§Recruiting information;
  `evidence/gaps/44/out-providers.json`, `xc-sharing-23.json`, `tools/44-iowa/compute.py`).
- **Remaining uncertainty:** the co-op-guest attribution is `[INFERENCE]` (no secondary guest-school field in
  the observed result schema; the guest edge must be re-attached from the classification table and tested);
  one unexplored page `iatrackcoaches.org/members/` is flagged as a possible extra name source; Johnston's
  district site was not retrieved; sport-attributed head-coach emails are effectively not published by Iowa
  districts in directory form.

### 45. Ohio gap follow-ups - OATCCC indoor directory, timer ecosystem, OHSAA portal re-check

- **Question (reports 19/20 items):** is there an OATCCC member directory, do Ohio timers carry coach/AD
  contact surfaces or grade, and does the myOHSAA portal still hold on a fresh school sample?
- **Verdict:** OATCCC publishes only association-level contacts (no member lookup exists), Ohio timers expose
  no coach directories, and myOHSAA remains the strongest coach-email path - every named coach cell carried a
  published email.
- **Measured:** 81 live HTTP requests (cap 80 - one over, disclosed: a peer-requested `oh.milesplit.com/timing`
  probe for report 31's 12-state timer table; largest single-host count `officials.myohsaa.org` 30, sequential
  1.2 s apart); no 403/429/CAPTCHA anywhere. OATCCC - full sitemap enumeration gives **658 unique URLs** and no
  member (school-coach) lookup; `/Contact-Us/` 200 / 91,880 B publishes **32 role-published contacts with 100%
  email coverage** (administration + 16 district representatives); the indoor verification list is 654 name
  rows / **588 distinct school names** with 0 emails and 0 phone numbers; the girls indoor division sheet has
  808 rows (93/203/220/292); the OHSAA FAT contractor sheet has 69 rows / 64 timers, **68 with email** and 67
  with phone, 59 distinct emails (24 gmail); myOHSAA - 15 fresh schools, 30/30 pages 200, **15/15 AD name+email**
  (32 `mailto:` rows) and a 60-cell coach grid (**15 schools x {XC, T&F} x {boys, girls}**) with **33/60 named,
  33/33 credentialed, 19 TBA, 8 N/A** (evidence `evidence/gaps/45/myohsaa-sample-summary.json` - verified:
  15 schools, 60 cells, 19 TBA, 8 N/A, 33 cells with an email); 35 email strings / 25 distinct coach emails;
  result surfaces - Finish Timing carries a numeric `Year` column (verbatim
  `Whiteley, Pazeley 10 Unattached 15.31`), Baum's Page renders the `Year` header blank on every row, Blue Fox
  PDFs have no text layer (`pdftotext` 0 chars), the AthleticLIVE blob doc has no grade field, TimingSpot and
  RacePenguin show no HS grade rows; access outliers - `gcxctiming.com` DNS failure (HTTP 000), buckeye/cantstop
  are 2,437 B SvelteKit shells
  (`research/midwest/45-ohio-gap-followups.md` §Coverage/§Recruiting information/§Result evidence/§Evidence
  appendix).
- **Remaining uncertainty:** Blue Fox's "not extractable" verdict rests on a sample of 1 of 25 2026 PDFs
  (marked `[INFERENCE]` in the report); SEO Timing, TimingSpot and On The Mark live-score surfaces are
  live-only and were not verified for grade; per-school cost (2 requests) projects to ~1,484 requests for the
  742 competing schools, replacing report 19's ~2,226-page equivalent; OATCCC's lack of a member directory is
  a proved negative, not a gap.

### 46. Event-token mapping: the seven bare-numeric event tokens in the delivered grade-11 workbook

- **Question (register A10):** what do the 7 distinct bare-numeric tokens in the delivered workbook's `Events`
  column mean?
- **Verdict:** they are **not event tokens at all** - they are comma-fragments of distance-medley relay shorts
  produced by report 04's own tokenizer (`.replace(";", ",").split(",")`).
- **Measured:** token rows/occurrences in the 142,705-row delivery - `4` 3,588/3,606, `8` 3,617/3,636, `16`
  3,280/3,290, `2` 326/327, `6` 37/37, `10` 37/37, `12` 5/10
  (`evidence/gaps/46/numeral-summary.tsv` - verified line-for-line); union **3,627 athlete rows = 2.542%**;
  rows with a numeral but no medley parent = **0**; 48 rows (1.3%) list a medley as their only event; the
  tokenizer re-run yields **79** distinct tokens; report 04's "~11.3k athlete-rows" is unreconciled - the sum
  of per-token occurrences is **10,943** (exactly report 04's own `all-athletes-agg.json` `ev_total`) and the
  sum of distinct-row counts is 10,890, so the 11.3k sentence probably described occurrences, not rows;
  independent corroboration - `800m` appears in 67.4% of the rows carrying `8` vs 20.2% of all rows (3.33x);
  false-mapping trap - the numerals collide with unrelated nav event ids (`2`=200m, `4`=800m, `6`=3000m,
  `8`=4x400m, `10`=110mh, `12`=shot, `16`=pv); the repo's production matcher works on the full short
  (`src/runtime/rankings/catalog.rs:217-218`), so it is not at risk
  (`research/midwest/46-event-token-mapping.md` §1/§2/§3; `evidence/gaps/46/numeral-cooccurrence.tsv`,
  `token-counts.tsv`, `numeral-reconciliation.tsv`, `numeral-provenance-exact.tsv`).
- **Remaining uncertainty:** leg order is `[INFERENCE]` from convention plus the leg-sum check (the source
  never states an ordered roster); the producer of the delivered `Events` column was not located anywhere
  (not in the repo, not in `/home/lewis/Downloads`), so it is unverified whether another build could emit a
  genuinely bare numeral; report 04's "~11.3k" remains unreproducible for any convention tested; one nav
  catalog name (`distmed10,2,6,16`, "DMR 4300m" with legs summing 3400 m) shows nav names alone are not a
  safe source of truth; live Athletic.net confirmation is out of scope by rule.

---

## §2 Cross-report tables

### 2.1 Grade-11 evidence by state (which source yields per-athlete grade, for which season/sport)

| State | Grade-bearing source (measured in the gap phase) | Season / sport | Measured counts | Grade-less surfaces observed in the gap phase |
|---|---|---|---|---|
| WI | WIAA tournament result PDFs, `Yr` column [35] | outdoor T&F 2026 (tournament series) | 2,786 grade-11 rows across 24 extractable files of a 26-file sample (union 134 2026 links); per-file 69/182/133/218 examples; OCR recovers the 2 failures | none claimed - the report's negative is about extraction, not grade |
| MN | MSHSL state result PDFs + AthleticLIVE `athlete_list` `y` + the 2026 program PDF [36] | XC 2017-2025 (2025 via OCR), T&F 2023-2026; AL rosters 2019, 2021-2025 XC and 2023-2026 T&F | program PDF 1,309 records / **415 grade-11**; AL rosters 845-1,218 (XC) and 2,473-3,308 (T&F) rows; 2025 XC OCR 156 grade tokens (Class AAA boys, ~5 s) | 2024 XC souvenir program (0 grade tokens); "Team Participants thru 2025" PDFs are school-level only |
| ND | NDHSAA state-meet program PDFs (`Gr.`/`Grade`), plus AL meet rows [37] | XC + indoor/outdoor TF (state meets) | track program 42 pp / 20,055,554 B with **1,766 standalone grade-number lines**; XC program carries `Last / First / Gr.` (7-12) | NDHSAA coach pages (contact side) carry 0 emails - unrelated to grade |
| SD | AthleticLIVE `dakota` row field `a.y` [43] | XC + TF (indoor and outdoor), 2015-> | 622 meet docs, 617 with `ani`; every row embeds the athlete object with `ani` + `y` + team ids (verified on 32 rows of `ind_res_list` 2915796) | Bound (state-TF page has an empty Result column); SDHSAA top-performance sheets (off-season `#N/A`) |
| IL | IHSA state-series qualifier JSON [38], boys TF/XC from [13] | XC 2025-26, T&F (state finalists) | girls XC **1,177 athletes / 318 grade-11**; boys XC 1,214 / 358 => 2,391 qualifiers and 676 grade-11 per season; boys XC/TF grade-bearing counts from report 13 (838 TF + 358 XC) | DirectAthletics IL surface frozen at 2023-24 and dropped from the weekly cadence |
| MI | MHSAA UP finals PDFs [39]; MITS blob rows `a.y` [39] | UP girls finals 2021-2026 (grade 2024-2026 only); MITS indoor 2019-2026 | UP PDFs: grade present 2024-2026, absent 2021-2023; MITS `a.y` 100% on GVSU MITS #5 / MITS 4 / 2025 CMU MITS, ~22-31% on AQ MITS #2 2026, absent <=2025-12-13 michiana events | MileSplit MI raw page `779299` (Yr column blank on all 971 rows) [31] |
| MO | TRXC Hy-Tek result files, `Year = 11` [40] | MO HS T&F (2024-2026 corpora) + XC | 1,933 grade-11 lines across the 11 HS T&F files listed (mean ~176, range 72-276) + 18 in the Festus XC file; MSHSAA official TF results are Athletic.net-linked PDFs [21][40] | `unifiedresults.pdf` (no grade, no points); club meets print `Age`; college rows print class-year codes |
| KS | AthleticLIVE `athlete_list` `y`; Midwest Timing PDFs; MileSplit/DAT rosters [41] | XC 2025/2026, TF outdoor 2026, 2015-2026 corpus | AL: 842 meets / 354,733 rows; TF-2026 107 meets / 42,023 rows (grade-11 4,793); XC 2026 5,882 rows, 100% graded; XC 2025 grade-11 1,637; MT: 546 + 544 graded performance rows (11: 164 / 184) and 4,336 relay leg tokens (11: 1,192) | kshsaachamps TF (864/864 `Grade: null`, girls 873/0, 2015 850/0, indoor 0 rows); AL 2022 state-meet grade column empty; kshsaa.org meet results; Bound (JS-loaded tables) |
| NE | NSAA S3 finals PDFs + district XC HTML, `Year`/`(NN)` [42] | 2026 state TF finals, 2025-season district XC (until 2026-10-15), state XC 2008-2025 via onlineraceresults [24] | `abresults` 1,395 rows (11:457), `cdresults` 1,406 (11:419), `abres26` 395 (11:117), `agres26` 386 (11:135), 8 district XC HTML files with `(11)` suffixes | MaxPreps feeds (top-5 names, no grade); `unifiedresults.pdf` |
| OH | Finish Timing Hy-Tek text, numeric `Year` [45] | OH HS TF (indoor/outdoor) + XC, archives 2014->2026 depending on host | sample rows `Whiteley, Pazeley 10 ...`, `Klimp, Olivia 11 ...` (per-row grade, no aggregate count published) | Baum's Page (Year header, all rows blank); Blue Fox PDFs (no text layer); AthleticLIVE blob doc (no grade field); myOHSAA (no athlete data); OATCCC sheets (school lists only) |
| IA | MileSplit IA `/raw` page `776880` numeric grade; AthleticLIVE row `a.y` [31][12][44] | XC 2026 raw page; AL TF/XC result rows; Bound rosters [11] | IA raw page 195 rows, 194 graded (99.5%), **50 grade-11** [31]; AL rows carry `a.ani`/`y` per report 12 | MileSplit `/entries` (no grade, any state) [31][27] |

Report 31's own consequence stands: **grade is a per-meet property, not a platform property** - the same
platform served 0% on MI `779299` and 100% on ND `775054`/SD `774965`
(`research/midwest/31-milesplit-entries-timing.md` §Result evidence). Two notations must be normalised:
numeric (`6`-`12`, where a May-2026 `11` is Class of 2027 but a September-2026 `11` is Class of 2028) and class
labels (`FR/SO/JR/SR`), which are ambiguous across levels - the MO raw page mixes college and HS rows and both
print `JR`.

### 2.2 Athletic.net leverage (ids and URLs each gap-phase source hands over, with measured coverage)

| Source (report) | Athletic.net **meet** ids | Athletic.net **athlete** ids | Athletic.net **team** ids | Profile URLs | Measured coverage fraction |
|---|---|---|---|---|---|
| MileSplit `/entries`, `/timing`, `/raw`, rosters, sitemap (31/32/33/34) | no (0 `athletic.net` matches in 80 captures [31]; 0 in roster/roster pages [34]) | no (no AthleteID ever exposed [34]) | no | profile **URLs** exist on the site but are MileSplit's own `/athletes/<id>-<slug>`; the sitemap is 97,500 such URLs | 0% AN id coverage by construction |
| AthleticLIVE ES `*_meet_list` (37/41/42/43/44) + `athlete_list`/`ind_res_list` rows | yes, doc field `ani` | yes, row field `a.ani` | yes, `t.ani` | `anis[].u` full AN meet URLs where set | SD 617/622 = 99.2% meet `ani` [43]; ND 421/437 = 96.3% [37]; NE 60/60 recent, 419/575 = 72.9% whole corpus [42]; KS 560/842 meets [41]; IA 758 unique ids from 1 request, 99% of 2026 docs [44]; athlete-level `ani` e.g. 88.9% of KS TF-2026 rows [41], MN XC 2025 1,141/1,143 = 99.8% [36] |
| AthleticLIVE blob + Firebase RTDB (39/45) | yes (`mi` + `ani` on the meet doc; RTDB `anis[].u`) | yes on blob result rows (`r[].a.ani`) | yes (`a.t.ani`) | via `anis[].u` | MITS 17/17 current-season meets [39]; OH blob doc carries `r[]` rows but **no grade field** [45] |
| MSHSL site + AthleticLIVE `wayzata` index (36) | yes - `wayzata_meet_list` docs, 148 MSHSL-named 2018-2026 | yes, where the timer linked rows | yes, `t.ani` | not exposed | state T&F `ani` 2022-2026 (475179 / 516328 / 563571 / 613966 / 666673); state XC always `ani: -1`/null; athlete ids 2025 99.8%, 2024 84.2%, 2023 0% [36] |
| WIAA pages + PDFs (35) | 5 distinct AN URLs printed inside 2 instruction PDFs; 2/26 result PDFs print an AN URL; 13/26 are RunMeet exports | no | no | no | 0/28 pages carry an `athletic.net` anchor |
| IHSA (38; report 13) | meet-link map `268054`/`268064`/`268066` (gender-shared per class), entries + results variants | no | no | no | girls XC ids 691-693 fetched directly (3 divisions, 6 requests/season both genders) |
| MHSAA HTML (39) | yes, link extraction only | no | no | AN meet/team/reference links | 215 distinct TF meet ids / 117 XC ids / 36 legacy ids across 16 pages; XC 2026 page currently 0 links |
| MITS / tenants (39) | yes, `ani` per meet | yes (`a.ani` on blob rows) | yes (`a.t.ani`) | no | 17/17 current-season meets, 20 tenant MITS meets, 66 MITS-named docs |
| TRXC Timing (40) | yes - 39 AN anchors in 39/40 upcoming-meet rows, **37 with meet ids** | no | no | AN entry links to upcoming meets | future meets only; 0 AN links on result index pages; MSHSAA itself links AN for TF results (not TRXC) |
| NSAA / NE hosts (42) | yes (`ani`, and `anis[].u` full URLs) | not exposed (athlete ids live in the RTDB/`athlete_list` path) | not exposed | no | 60/60 recent + 98/528 with `anis`; ≈2,200-6,600 AN requests avoided |
| Dakota Timing archive (43) | via its own `fallback_link` JSON field only (never fetched by this campaign) | no | no | no | 452 of 1,120 SD events expose a `phase2in.com` viewer instead |
| KSHSAA / MT / DAT (41) | 560/842 AL meets carry `ani`; DAT rosters carry class-year only | no (MT/DAT); AL rows yes | no | no | as above for AL; MT/DAT contribute grade but no AN ids |
| myOHSAA / OATCCC / OH timers (45) | Finish Timing's id is derivable (`FT id = "20" + ANID`, measured in report 20); nothing new added here | no | no | no | no new independent id source; Blue Fox/Timing First serve AthleticLIVE SPA shells |
| Bound (43/44) | no AN ids in the SD/IA surfaces tested | no | no | no | IA Bound names 346 of 379 members; 5 program-less schools must be excluded |
| MaxPreps / school sites (42/44) | no | no | no | no | coach-email layer only: 4/16 role-matched (25%) at 2-4 requests/school |

### 2.3 Access classification per source touched in the gap phase

| Class | Sources (report) | Notes observed |
|---|---|---|
| Documented / semi-documented public API | AthleticLIVE Elasticsearch `search.athletic.live` (37/41/42/43/44) - anonymous JSON POSTs, undocumented but stable; KSHSAA `kshsaa-api.kshsaa.org/PublicClassifications` + `kshsaachamps` JSON (41) | `_mapping` 403 (no `view_index_metadata`); text-field aggs 400 x2; KSHSAA SPA deep link 404s to a plain client |
| Public structured JSON (other) | MSHSL `/api/`, `/jsonapi/` (36); IHSA `/v1`,`/v2` (38); Firebase RTDB `s-gke-...firebaseio.com` (37/39) | MSHSL roster 4th segment 404 and `?year` silently ignored; RTDB rejects HEAD (405) and has no ETag revalidation |
| Static file | NSAA S3 (`nsaa-static.s3.amazonaws.com`) HTML/PDF (42); WIAA upload dir PDF/HTM (35); MSHSL PDFs (36); NDHSAA CDN PDFs/XLSX (37); AthleticLIVE blob (39/45) | S3 objects publish `x-amz-meta-sha256` (free integrity check); WIAA file validators are advertised but not honored (conditional GET returns the full body) |
| Normal HTML (server-rendered) | MileSplit pages `/entries`, `/timing`, `/raw`, rosters (31/32/34); MileSplit sitemap XML (33); WIAA Drupal + AJAX fragments (35); IHSA/DA result pages (38); MHSAA pages (39); TRXC Hy-Tek corpus + WordPress (40); Bound (41/43/44); IHSAA/IGHSAU/IATC (44); officials.myohsaa.org (45); NDHSAA/ndhsaanow (37); NSAA pages + district sites (42) | MileSplit `robots.txt` Disallows the `/api/` path only; MHSAA `robots.txt` disallows `/core/`, `/profiles/`, `/README.md`; IHSA `Allow: /`; TRXC `robots.txt` 404 (no policy found at all); WIAA `/Reports/` 403 |
| Browser-only / client-rendered | AthleticLIVE white-label SPAs and tenant shells (39/45); TimingSpot Wix, RacePenguin `rtrt.me`, SEO Timing widget, On The Mark socket.io scores (45); Bound roster tables (41); Bettendorf/Apptegy client challenge (44); Aurora husky-timing (42) | all returned 200 with empty or non-parseable result rows for a plain client; no JS execution was needed by any recommended path |
| Blocked at the host | `search.athletic.live/.../_mapping` 403 (37); WIAA `/Reports/` 403 (35); Millard West origin 403 (42) | none worked around; all recorded as findings |
| Absent / dead | See §2.4 | - |

### 2.4 Blocked, dead ends and absent endpoints found in the gap phase (exact URL + status)

| URL | Status | What it proved |
|---|---|---|
| `https://search.athletic.live/dakota_meet_list/_mapping` | **403** (`security_exception`) | API key lacks `view_index_metadata`; field types derived empirically instead [37] |
| same index, agg on text fields (`ls`, `lsa`, `session`) | **400** x2 | fielddata disabled - shapes query design [37] |
| `https://www.gcxctiming.com/` | **000** (DNS failure, `Could not resolve host`) | host unavailable [45] |
| `https://www.bluefoxtiming.com/s/2026-0404-independence.pdf` | **404** (126,400 B) | guessed path wrong (recorded) [45] |
| `https://storage.googleapis.com/bluefoxtiming/2026/4042026.pdf` | 200 but **no text layer** (`pdftotext` 0 chars) | not a grade source without OCR [45] |
| `https://www.oatccc.com/Coaches/` | **302 -> /** | no section landing page; no member directory exists [45] |
| `http://www.carrington.k12.nd.us` | 301 -> 200 "Client Challenge" | bot interstitial -> host abandoned [37] |
| `https://www.mpsomaha.org/...` (Millard West) | **403** | 0 emails obtainable from origin [42] |
| `https://aurorahuskies.org/page/husky-timing` | Cloudflare **JS challenge** | recorded access finding [42] |
| Bettendorf district site | 200, 3,036 B "Client Challenge" | host classified browser-only/challenge; 0 content [44] |
| `http://wistca.ihigh.com/` | DNS does not resolve (curl exit 6) | no WI coach feed [35] |
| `http://www.wisconsinrunner.com/wccca/` | 301 -> 302 -> **404** | same [35] |
| `https://www.wiaawi.org/Reports/` | **403** | no bulk report export exists [35] |
| `https://www.ihsa.org/sports/...` linked 2026 prelim PDF | **404** (the other five links 200) | fetch-and-verify is mandatory [36] |
| `.../sitemap_index.xml`, `/news-sitemap.xml`, `/sitemap/`, `/team-sitemap.xml` (MileSplit hosts) | **404** | no alternative sitemaps; 7,500 is the whole surface [33] |
| `https://kshsaa.org/.../general/PublicClassifications` (SPA deep link) | **404** to a plain client | client-side route; the API is the data layer [41] |
| `https://kshsaa.org/schedules/trackFieldSchedule/teamschedule/735/1` | **404** | kshsaa.org carries no usable meet results [41] |
| `https://gobound.com/sd/sdhsaa/boystrackfield/2025-26/scores?...` | 200 with **empty Result column** | Bound carries no SD T&F results [43] |
| `https://www.mshsaa.org/...` (TF/XC InformationCentral, Home) | 200, **0 TRXC links** | TRXC discoverable only from itself; MSHSAA links PrimeTime + Athletic.net [40] |
| `https://trxctiming.com/robots.txt`, `/wp2/wp-sitemap.xml` | **404** | no policy and no sitemap; absence is not a licence [40] |
| `https://archive.org/...` (probe) | **429** | probe stopped [40] |
| ND 2026 state XC across `*_meet_list` (`md` 2026-10-15..2026-11-10) | **0 hits** (twice) | index lags publication [37] |
| `dakota_meet_list` (`lsa:"North Dakota"`, phrase) | **0 hits** (916 docs, SD/IA/NE/WY/OR only) | dead end for ND [37] |
| NSAA coach/AD emails in the 312-school directory | 1,085,584 B contains exactly 1 `@` string | NSAA publishes no coach/AD emails [42] |
| NDHSAA school pages (33 cumulative) | **0 email strings** | coach *names* only [37] |
| IHSAA/IATC official directories | **0 school coach emails** | association delegates schools to Bound [44] |
| DirectAthletics IL lists/championships | newest = 2024 (list ids <= 4526) | IL surface frozen [38] |
| MSHSL prior-year rosters (`/api/team-data/roster/611/varsity/124/695`, `?year=695`) | 404 / 200 ignored | no prior-year roster exists [36] |
| `https://www.bluefoxtiming.com/contact` | 200 but empty | no contact fields [45] |
| RacePenguin results | 200, road-race/age data | no HS grade [45] |
| prepcast/Striv host hunt (42) | no results host located | dead end recorded [42] |

---

## §3 Answers to the mission's acceptance questions updated by the gap phase

Only rows the gap reports changed are listed; everything else stands as written in
`synthesis/01-acceptance-answers.md` and `synthesis/00-executive-summary.md`.

| Acceptance question / register row | Earlier answer (doc + section) | Gap-phase update | Evidence |
|---|---|---|---|
| **C4** - `/entries` + `/timing` beyond 4 states | open; "meet-has-data vs meet-has-link" unmeasured (`synthesis/06-open-questions.md` §C, row C4) | **Resolved as a three-part result:** `/raw` = result workhorse (18/18 meets had a working raw page; 7/7 sampled raw requests 200); `/entries` = discovery-only, measured on 9/12 states, no grade; `/timing` = timer directory, roots now measured on 12/12 states | 31 §Coverage/§Recommendation |
| **C5** - Progression-tab payload (`paywall_present:1`) | "44 locked rows" on a WI profile (`synthesis/08-gap-closure-2026-09-20.md` §4 (C5 bullet) and `synthesis/06-open-questions.md` status line) | **Corrected and extended:** no truncation; 5 mask cells per row; the profile in question renders 11 rows / 55 masks (44 was 4 x 11 cells); state rank locked on 100% of PR rows; identity + PR table + `/raw` stay free | 32 §Athlete evidence; `evidence/gaps/32/parsed-summary.json` |
| **C6** - sitemap 7,500-cap semantics | "7,500 rolling `/athletes/` URLs, ~2 h lastmod window; 7-day recurrence check pending" (`synthesis/06-open-questions.md` status line) | **Semantics resolved:** rolling top-7,500-by-`lastmod`; turnover 44-96% in <10 h; reach 1.6 h (OH) to 68 d (ND); no validators so every check is a full transfer; the 7-day recurrence check is still the only open part | 33 §Incremental use; `evidence/gaps/33/sitemap-window-summary-2026-09-20.json` |
| **C2/C3** - ATN join quality + roster sweep | open; join "structural today, not measured" (`synthesis/06-open-questions.md` §C) | **Pilot measured:** 48.9% of sampled Co2027 boys sit beyond the corpus (46.5-51.8% range), 95.1% of those unmatched have names absent nationally, girls 0/450; 1,189 requests for the WI+MN frame | 34 §Recommendation; `evidence/gaps/34/final-tables.md` |
| **A10** - 7 unexplained numeric event tokens | "~11.3k rows in retained delivery" (`synthesis/06-open-questions.md` §A, row A10) | **Closed:** the 7 tokens are medley-relay comma-fragments from report 04's tokenizer; affected population 3,627 rows (2.542%); report 04's ~11.3k = token occurrences (10,943), not rows | 46 §1/§2; `evidence/gaps/46/numeral-summary.tsv` |
| **Q4** - grade-corroborated share (3,993 rows = 9.8%) (`synthesis/01-acceptance-answers.md` §Q4) | 9.8% with state-meet cohorts KS 89 / SD 106+ / MO ~300-350 etc. | **Per-state oracle set superseded in kind, not re-derived:** KS moves out of "grade-poor" (AL 354,733 rows; MT 546+544 graded performance rows + 4,336 relay leg tokens), MO gains TRXC's 1,933 grade-11 lines, SD gains the `dakota` grade oracle (617/622 meets with `ani`), NE gains finals/district grade counts, MN gains the 1,309-record program PDF (415 grade-11). **No gap report recomputed the 9.8% aggregate**, so the percentage stands until that arithmetic is redone | 41 §2, 40 §Athlete evidence, 43 §Athlete evidence, 42 §Result evidence, 36 §Athlete evidence |
| **Q6** - coach-email rates (WI 1.00, IL 0.75, OH 0.59, MN 0.40, IA 0.20...) (`synthesis/01-acceptance-answers.md` §Q6) | school-clustered bands | **Refined per state (unit differs from Q6's school-clustered basis and is not merged into it):** IA's official directories publish 0 school coach emails (11 district probes: 1 head-coach + 7 AD/activities addresses); OH 15/15 AD name+email and 33/33 named coach cells credentialed; IL entered-school denominators 14/14 (boys T&F) down to 8/9 (girls XC), ADs 16/17; WI unchanged at poll-per-school with measured cost; ND names only (district directories uneven). The corpus-weighted 35.4% / 68.1% figures were **not** recomputed | 44 §Recruiting information, 45 §Recruiting information, 38 §Recruiting information, 35 §Recruiting information, 37 §Recruiting information |
| **Q8** - states behind the remaining gaps | "SD (Bound empty), MO (ToU), MI/IL (Athletic.net/MileSplit only), KS (no weekly schedules), NE (district top-15 only)" (`synthesis/01-acceptance-answers.md` §Q8) | **Partially retired:** MO regular-season results now have a grade-bearing non-AN source (TRXC, licensing caveat); SD results + grade closed by the `dakota` tenant; KS grade depth closed (weekly schedules remain absent); MI/IL gain postseason AN-seed inventories (215 + 117 MI ids; IHSA XC both genders); NE gains 2026 finals/district grade files and 19 timer contacts. Still open in kind: MO licensing, MI/IL regular-season results outside AN/MileSplit, NE district top-15 depth | 40, 43, 41, 39, 38, 42 |
| **Q9** - best adapters | ten ranked adapters (`synthesis/03-adapter-ranking.md`) | **Amendments the gap reports ask for:** KS - adopt AthleticLIVE Kansas + MT PDFs + MileSplit/DAT rosters (23's "grade-poor" note retired); SD - adopt the `dakota` tenant as result feed and grade oracle; MO - add TRXC as result source with a licensing inquiry; MI - MHSAA pages as AN-seed; OH - myOHSAA 2-page-per-school contact path (~1,484 requests for 742 schools); IA - official directories rejected for coaches, district sites discovery-only | 41 §Recommendation, 43 §Recommendation, 40 §Recommendation, 39 §Recommendation, 45 §Recommendation, 44 §Recommendation |

Also closed without changing a numbered answer: **WI** register row (no coach change feed; 2026 PDF extraction is
file-specific with OCR) [35]; **MN** register row (2025 XC PDFs are outline-flattened and OCR-recoverable;
prior-year rosters do not exist) [36]; **ND** register row (state-meet programs carry grade; 0 emails on
NDHSAA) [37]; **IL** register row (girls XC 691-693; DA frozen) [38]; **MI** register row (UP girls naming;
MITS archive) [39].

---

## §4 Open items carried forward

| # | Open item | Owning report | Why it remains open |
|---|---|---|---|
| 1 | `/meets?year=2026` upcoming-meet index route unprobed; entries base rate on the full meet population unknown; meet publish-state flip unobserved; OH/IN per-meet `/entries` unmeasured | 31 | one probe set was scoped to the results browser; the sample was conditioned on meets with published results |
| 2 | Athlete with >100 results unsampled; the "funnel renders all rows" reading rests on an inline-JS comment | 32 | no such athlete was in the 14-profile sample; `[INFERENCE]` preserved |
| 3 | 7-day sitemap recurrence check (the remainder of C6) | 33 | needs two pulls a week apart; semantics already resolved |
| 4 | Other 10 states untested for the roster join; girls half unvalidatable against a boys-G11 corpus | 34 | pilot scope was WI+MN by design |
| 5 | Broken girls-archive PDF link (substituted); OCR marks treated as reported; season-wide OCR projection `[INFERENCE]` | 35 | link is site-side; OCR numeric noise is inherent |
| 6 | MSHSL prior-year per-athlete rosters unrecoverable; strict full-row OCR parse 71.3% vs name quality 95.3%; one 2026 finals link 404 | 36 | endpoints do not carry prior-year rosters; parse noise is a method limit |
| 7 | Grand Forks coach-level email lookup `[UNVERIFIED]` (AJAX search); NDHSAA state-meet "PERFORMANCE LISTS" hrefs to re-check after the 2026-10 meets; XLSX qualifier-entry template unparsed | 37 | search is AJAX; placeholders have no hrefs today |
| 8 | One school withheld the girls T&F email; 2 of 12 revealed addresses are non-district (`gmail.com`); DA resumption detector (`list id > 4526`) | 38 | directory contents; detector is a rule, not a measurement |
| 9 | MITS AQ MITS #2 2026 grade partial (~22-31%); 2026 XC regional links appear per contested meet; exact AN savings `[INFERENCE]` | 39 | timer-side data quality; season in progress |
| 10 | TRXC licensing/ToU inquiry before weekly sweeps; Hy-Tek school-alias dependency; club/college row filters; archive.org 429 unretried | 40 | no policy found; alias map is campaign-wide work |
| 11 | Whether Bound can serve Kansas grade rosters; 2022 AthleticLIVE KS state meet grade column empty; relay token count `+/-~0.5%` | 41 | roster tables JS-loaded; meet-config-dependent grade |
| 12 | NE school-site sweep extrapolation `[INFERENCE]`; prepcast/timer host hunt incomplete; XC district files rotate 2026-10-15 | 42 | measured rate applied to 312 schools; leads unresolved |
| 13 | 617 vs 618 `ani` framing difference unexplained; co-op guest attribution `[INFERENCE]` (needs one real co-op meet test); yearbook loose-parse over-counts | 43 | in-report uncertainty preserved |
| 14 | `iatrackcoaches.org/members/` unfetched; Johnston district site unretrieved; co-op guest attribution `[INFERENCE]` | 44 | probe budget; flagged by the report itself |
| 15 | Blue Fox "not extractable" is a 1-of-25 sample `[INFERENCE]`; SEO Timing/TimingSpot/On The Mark live-only grade unverified | 45 | sample size; live-only surfaces |
| 16 | Medley leg order `[INFERENCE]`; producer of the delivered `Events` column not located; report 04's "~11.3k" unreconciled | 46 | offline analysis found no artefact that settles them |
| 17 | Q4's 9.8% grade-corroboration aggregate not re-derived after the gap phase's new oracles | this consolidation (no owning gap report) | every gap report supplies source counts, none recomputes the campaign aggregate |

---

## §5 Contradictions and evidence caveats found while consolidating

1. **"44 locked rows" vs the measured mask geometry (report 32 vs synthesis 08 §3 (C5 row) and 06's status
   line).** 08 states "**44** result rows are replaced by `aria-label="Locked"` placeholders" and 06's status
   line repeats "44 locked rows". Report 32 explicitly corrects this: 44 is the count of the four non-seed
   mask *cells* (4 x 11) on Dina Abdel-Megid's page, which renders **11** result rows and **55** mask spans
   (`research/midwest/32-milesplit-paywall-boundary.md` §Athlete evidence;
   `evidence/gaps/32/parsed-summary.json`). Both figures are given here; the measured geometry (11 rows /
   55 masks) is the one backed by the parse output. A secondary disagreement in the same sentence: 08 calls
   the PR block "truncated", while 32 finds the full PR set is present in the HTML including
   `style="display:none"` per-season rows.
2. **Report 04's "~11.3k athlete-rows" vs report 46's 3,627 rows / 10,943 occurrences.** Report 46 states the
   11.3k figure is "unreconciled and (as stated) wrong for any convention tested"; the affected population is
   3,627 rows, and 10,943 is the sum of per-token occurrences - exactly report 04's own
   `all-athletes-agg.json` `ev_total` (`research/midwest/46-event-token-mapping.md` §2). Both figures are
   preserved; the token-level ledger in `evidence/gaps/46/` is the machine-readable authority.
3. **Report 34's corpus name count: 133,570 vs 132,884.** The report and its appendix cite 133,570 distinct
   normalized names for 142,705 rows (`research/midwest/34-milesplit-discovery-vs-validation.md` §Stable
   identifiers, backed by `evidence/gaps/34/name-presence-check.txt` line 1: "corpus rows=142705 distinct
   normalized names=133570"), while `evidence/gaps/34/join-metrics.json` records
   `"corpus_distinct_names": 132884`. Both are in the same evidence bundle; the difference is likely a
   normalisation difference between the two scripts, and nothing downstream depends on the count.
4. **Report 40's grade-11 total vs its own evidence file.** The report gives 1,933 grade-11 lines across the
   11 HS T&F files it lists; that total reproduces exactly from its own per-file list, but
   `evidence/gaps/40/strict-stats.json` covers 17 labelled files and sums to **2,685** grade-11 lines
   including the 18-line XC file (extra non-zero entries: 300, 354, 79; and the report's Lindbergh 191 is
   recorded there as 192). The report's season totals are already marked `[INFERENCE]` by the report itself.
   No `[UNVERIFIED-CLAIM]` marker was needed for the per-file numbers - each named file has a counterpart row -
   but the two totals are not the same measurement.
5. **Report 31's SD outdoor state page: two different stats files in one bundle.**
   `evidence/gaps/31/raw-stats.json` records `rows: 0`, `teams: 0`, `relay_leg_rows: 0` under key
   `765035_state`, while `evidence/gaps/31/sd-765035-state-raw-stats.json` records rows 3,027 / teams 145 /
   relay_leg_rows 798 / `grade_col: "absent"`. The report cites the latter (3,027 rows, 168 event sections,
   798 leg rows), and the aggregate raw-stats entry appears to be an unparsed-summary artifact. The report's
   figures are the ones with a dedicated evidence file.
6. **Report 43's 617 vs 618.** The report records "617/622 (99.2%) with `ani`" and a separate
   `must_not exists` framing returning 4 docs, with the 1-doc difference left unexplained and marked
   `[INFERENCE]` by the report; `evidence/gaps/43/es-dakota-ani-coverage.json` (617 `ani_present` of 622)
   matches the 617 figure. Preserved as an in-report uncertainty, not resolved here.
7. **Report 37's 421/437 at meet level vs 837/875 at doc level.** Both are correct measurements of different
   objects: the census file `evidence/gaps/37/nd-census-1000.json` holds 875 docs of which 837 carry a numeric
   `ani`; the report's 421/437 is the cross-tenant deduped meet-level fraction. Quoted as such, not as a
   contradiction.

`[UNVERIFIED-CLAIM]` markers added in this document: **none**. Every number above is either reproduced from a
report section I read or from an evidence file I opened; the five discrepancies in this section are stated
with both figures rather than silently resolved.

---

*Consolidation scope: `research/midwest/31..46` (16 files) + `research/midwest/evidence/gaps/{31..46}` and
`synthesis/08-gap-closure-2026-09-20.md`. No other file was modified. Report 44 (Iowa) is included; no gap
report remains pending.*
