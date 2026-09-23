# Great Lakes TF/XC source report — IL, IN, MI, OH, WI

Consolidation lane (national contract) of the Midwest TF/XC source research. It adds **no new
fetches**: every fact below is read from an existing capture or report on disk, and each claim names
the file it came from. Where the Midwest research measured a number, this report says so; where it
did not, the gap is called out explicitly (§0.3) instead of being papered over with an estimate.

## 0. Scope, vocabulary, and denominator policy

### 0.1 Jurisdictions and their primary sources

| State | Association (national lane) | Association URL | Supporting lane |
|---|---|---|---|
| IL | Illinois High School Association (IHSA) | `https://www.ihsa.org` / `https://api.ihsa.org` | MileSplit `il`, DAT/TFRRS |
| IN | Indiana High School Athletic Association (IHSAA) | `https://www.ihsaa.org` | TFRRS `indiana`, MileSplit `in` |
| MI | Michigan High School Athletic Association (MHSAA) | `https://www.mhsaa.com` / `https://my.mhsaa.com` | MileSplit `mi`, MITCA, Michigan timers |
| OH | Ohio High School Athletic Association (OHSAA) | `https://www.ohsaa.org` / `https://officials.myohsaa.org` | MileSplit `oh`, Finish Timing, Baum's Page, OATCCC |
| WI | Wisconsin Interscholastic Athletic Association (WIAA) | `https://www.wiaawi.org` / `https://schools.wiaawi.org` | PrimeTime Timing, Performance Timing, MileSplit `wi` |

### 0.2 Vocabulary mapping (Midwest report labels → national contract labels)

The Midwest corpus used `PRIMARY / ATHLETIC.NET-SEED / RESULT-SOURCE / COACH-DIRECTORY / VALIDATION /
DISCOVERY-ONLY / REJECT`. This report emits only the national tokens, mapped as follows:
`ATHLETIC.NET-SEED → DISCOVERY_SOURCE`, `COACH-DIRECTORY → COACH_SOURCE`,
`RESULT-SOURCE → RESULT_SOURCE`, `VALIDATION → VALIDATION_SOURCE`, `DISCOVERY-ONLY → DISCOVERY_SOURCE`,
`PRIMARY / CONDITIONAL / REJECT` unchanged. Both labels are shown where a source's own report used the
Midwest word, so nothing is silently retyped.

### 0.3 Denominator policy — measured vs. not measured

**Measured denominators** (a count someone actually produced from a fetched artifact; path given):
association member-school universes — **828** IL and **815** OH are **re-derived here from retained
bytes**: `research/midwest/evidence/gaps/38/il-schools.json` (451,188 B, minified, md5
`498179f68c66793396dd4c6123a4a4bc`, byte-identical copy `tools/a13-ihsa/schools.json`) parses to
exactly **828** rows — 801 full + 26 approved + 1 associate, 604 Boundary / 224 Non-Boundary, 677
public / 151 private, 126 CPS, `SchoolID` 0101–7097 — and its own fetch ledger records the same URL at
**451,182 B**, 6 B fewer than the retained file: both numbers are printed, the delta is not explained
away. `research/midwest/evidence/gaps/45/ohsaa-enrollment.html` (228,567 B, the `GET
https://www.ohsaa.org/school-resources/school-enrollment` capture) holds one table with 816 `<tr>` =
1 header + **815 data rows**, 815 distinct `OhsaaSchoolId` (100–1000027), class cell 267 AAA + 267 AA +
267 A + 14 blank. The in-repo fixture `ihsa/v1_schools.json` is a **3-row sample** of the IL payload —
schema evidence only, never the denominator. — **413** IN and **755** MI are **quoted from [17] /
[15]**, with no retained capture that re-produces them: the member pages in the corpus are SPA shells
with zero table rows (`tools/a01/assoc/mhsaa_schools.html` and `tools/a29-coach/in-school-directory.html`
both contain 0 `<tr>`), and the MI figure rests on an enrollment PDF whose bytes were not kept — so a
downstream national report must carry them as *the reporting lane's measurement*, not as something it can
re-check. — **516** WI is **published by WIAA, not enumerated** (only 1 of 26 directory letters was ever
fetched, yielding 39 schools); per-sport sponsorship counts where
the association publishes a classification matrix (IL, WI, OH), archive/file inventories (WIAA, MHSAA,
IHSA documents), provider team-registry counts (MileSplit team records; DAT team records), timer meet
counts (PrimeTime, Finish Timing), state-meet grade-bearing cohort counts (IL/OH/WI/MI), and the
pipeline's own census (`reports/census-by-state.csv`, `reports/report-core.json`).

**Not measured — never quotable as coverage:**
1. **No state in this lane has a measured statewide Class-of-2027 population.** No association
   publishes a count of juniors in TF/XC; therefore no percentage-of-Co2027-coverage figure exists for
   IL, IN, MI, OH, or WI. Any such figure must stay unstated.
2. **Bands are not counts.** `data/milesplit-coverage-matrix.csv` carries
   `est_c2027_total_order_of_magnitude_low/high` for each state — IL 5,943–30,564; IN 2,843–8,528;
   MI 895–895; OH 9,770–86,953; WI 16,119–39,999 — with the file's own note *"n=3 teams/state; NOT a
   point estimate, band only"*. The `sampled_c2027_*` columns are 3-team samples (MI sampled 3 teams
   with 1 non-empty roster and 3 Co2027); they are evidence about method, not coverage.
3. **Per-sport participation is unmeasured for MI and IN.** MHSAA publishes class sizes but no
   per-sport school counts ([15]); IHSAA publishes a declared sectional-slot total (391 across 25 XC
   sectionals, 2026) with no school-name lists and no result links yet ([17]).
4. **Pipeline snapshots differ from the research estimates and from each other.** Two measured
   snapshots exist — `synthesis/10-measured-census.md` (2026-09-20) and `reports/census-by-state.csv`
   plus `reports/report-core.json` (2026-09-21, with an explicit `core = Athletic.net off` scope) —
   and they disagree (e.g. WI athletes 73,202 then 125,844; IL coach rows 16 then 15,102). Numbers
   below are always attributed to one of those files and never averaged.
5. **The pipeline's gender split does not sum to its Co2027 total.** In `reports/census-by-state.csv`,
   `co2027_boys + co2027_girls < co2027` for IL (26,472 vs 26,488), IN (16,826 vs 16,931), OH (31,631
   vs 31,708) and WI (22,045 vs 22,074); MI is exact (22,073). The remainder is records the pipeline
   could not attribute to a gender, so a national report must not print the total as "boys + girls".

### 0.4 Corpus filing hazards (check before trusting a path)

1. **Two associations are named IHSAA.** Indiana's is `ihsaa.org`; Iowa's is `iahsaa.org` and Iowa's
   boys association is *also* the "IHSAA". The corpus dir `tools/scratch-11/` belongs to the **Iowa**
   lane (report [11] = `11-iowa-ihsaa-ighsau.md`), and its `ihsaa-members.csv` (379 rows: ACGC, ADM,
   AGWSR; conferences River Valley / SEISC), `p-ihsaa-*.html` (435 `iahsaa` links, 0 "Indiana") and
   `bound-ihsaa-*` files are all Iowa's. **No Indiana fact in this report is drawn from them** —
   section 3 cites [17]/[18]/[28] only, and the 391-slot / 25-sectional XC figure is [17]'s own text.
   Those files are relabelled in `samples/CAPTURES.md` §B5-IA so that a name search for "ihsaa" cannot
   walk a national report into Iowa data.
2. **`[NN]` is the Midwest report number, not a directory guarantee.** `research/midwest/NN-*.md` is
   the source; `tools/scratch-NN/` mostly follows the same numbering (11 = Iowa, 16 = Michigan
   alternatives) but shared super-index lanes reuse dirs (`tools/scratch-28/` serves reports 18, 28
   and others; `tools/gaps31/`-style dirs are cross-cutting). Identity was therefore checked per file
   by reading its bytes, and that check is recorded in `samples/CAPTURES.md`.
3. **A retained capture can disagree with its own ledger.** The IL schools payload measures 451,188 B
   on disk while `gaps/38/ledger.jsonl` records 451,182 B for the same URL. Both numbers are printed
   where the file is cited; nothing here is averaged.

## 1. Coverage at a glance (measured)

| State | Assoc. member HS | Sport sponsorship (measured) | Pipeline census, all sources (2026-09-21) | Pipeline census, core = AN off | Repo adapter in `crates/census-service/src/sources/` |
|---|---|---|---|---|---|
| IL | 828 [13] — **re-derived here** from `gaps/38/il-schools.json` (828 rows, §0.3) | TRB 632 · TRG 624 · CCB 539 · CCG 507 [13] | schools 1,889 · athletes 92,973 · Co2027 26,488 | Co2027 19,392 · coaches 15,102/3,235 email | `ihsa/` |
| IN | 413 (408 full + 5 provisional) [17] — **quoted, not re-derivable** (SPA shell, §0.3) | not published per sport; 391 declared sectional slots [17] | schools 1,108 · athletes 54,475 · Co2027 16,931 | Co2027 11,909 · coaches 0 | **none** |
| MI | 755 [15] — **quoted, not re-derivable** (PDF bytes not retained, §0.3) | not published per sport (classes A 188 / B 189 / C 189 / D 189) [15] | schools 1,493 · athletes 114,289 · Co2027 22,073 | Co2027 16,176 · coaches 6/6 | **none** (coach CSV import recognizes `mhsaa` host only) |
| OH | 815 [19] — **re-derived here** from `gaps/45/ohsaa-enrollment.html` (815 data rows, §0.3); 742 competed: 721 TF · 602 XC [19] | 556 B-XC + 468 G-XC team rows; 685+644 T&F rows [19] | schools 1,504 · athletes 106,136 · Co2027 31,708 | Co2027 26,998 · coaches 28/23 | `ohsaa/` |
| WI | 516 (WIAA-published, not enumerated — §0.3) [06] | BTF 462 · GTF 462 · BXC 437 · GXC 435 (1,796 team-seasons) [06] | schools 1,027 · athletes 125,844 · Co2027 22,074 | Co2027 19,281 · coaches 2,550/2,155 | `wiaa/` + `wiaa_results/` |

Census columns read from `~/Downloads/midwest-tfxc-source-research/reports/census-by-state.csv`
(all sources) and `.../reports/census-by-state-core.csv` / `report-core.json` (`"scope": "core"`,
`~/Downloads/midwest-tfxc-source-research/reports/census-service-2026-09-20-summary.csv` documents
the two scopes: `Core (Athletic.net off)` vs `All sources`). MileSplit/DAT team-registry numbers, the
per-state Athletic.net Grade-11 baseline, and coach-contact yields are tabulated per state in §2–§6.

## 2. Illinois — IHSA

### 2.1 Primary association fields (national contract §13)

| Field | Value |
|---|---|
| Source name | Illinois High School Association (IHSA), sole state association; single-association state [13] |
| Geographic coverage | Illinois only; `State: "IL"` on every school row [13] |
| Sports | `TRB`/`TRG` (spring T&F), `CCB`/`CCG` (fall XC); no IHSA indoor series; `AWD` activity + wheelchair class `WD` scored inside the main meet (4 wheelchair events in the 2026 boys final) [13] |
| Historical depth | Terms `2023-24 … 2026-27` for structure; T&F results API holds exactly **2 meets** (2026 boys/girls state finals); XC qualifiers exist for **2025-26 only**; static `/data/` qualifier pages are overwritten each season (no archive); heat-sheet PDFs: 4 for 2025-26, 0 for 2024-25 [13] |
| Discovery mechanism | `GET /v1/schools` (1 request, 828 rows) → `/v2/classification/classifications?season=F` / `?season=S` (2) → `/v1/terms`; T&F: `/v1/track-field/meets` → `/v1/track-field/events/{eventId}/summary`; XC: `/v1/{term}/statefinal/cc-qualifiers?tournamentId={688..693}` [13][38] |
| Stable identifiers | `SchoolID` (4-digit, `0101–7097`), `PersonID`, `RoleID`, `tournamentId` (XC 688–693), `MeetId` 74003/74002, Athletic.net meet ids in the page bundle [13][38] |
| Pagination | `/v1/schools` returns all 828 rows in one response; no paging observed on any exercised route [13] |
| Athlete fields | state-final finisher: name, grade (`year`), school, `athlete.athleticNetId`, `team.athleticNetId`, `athleticLiveId`, relay legs included; qualifier pages: name + grade + school; **no** athlete profiles/URLs, no contact data of any kind [13] |
| Meet fields | 2 state-final meets with ids; event summaries carry event identity; XC is class-level meet ids (268054/268064/268066, shared by both genders) [13][38] |
| Result fields | event summary payload (110 KB for a 20-finisher field event) with place, mark, grade, school, AN ids; no ResultID; no regular-season data [13] |
| Grade/class evidence | 2026 qualifier pages: 3,342 entry lines `Sr 1,295 / Jr 1,079 / So 636 / Fr 332` → **2,555 unique (name, grade, school) triples, 838 Jr**; XC: **1,214 boys qualifiers, 358 grade-11**; girls XC ids 691/692/693 closed in [38] |
| Coach/contact fields | `GET /v1/schools/{SchoolID}/staff2` grouped staff rows (`PersonID, Name, DefaultTitle, HasEmail, LastName, RoleID, Phone, Fax`); RoleIDs `HCB-TRB`,`HCG-TRG`,`HCB-CCB`,`HCG-CCG`,`C1-BoysAD`,`D1-GirlsAD`,`E1-ActivDir`; email via `/v1/schools/{SchoolID}/staff/{PersonID}/email` → `{"email":"…"}` (e.g. `jrakestraw@atown276.net`); 14-school sample: boys T&F HC 11/14, girls T&F 12/14, XC 9–10/14, AD 14/14, **all 49 T&F/XC head-coach rows `HasEmail: true`**; 24-school resample on entered schools: TRB 14/14 name+email, TRG 13/14, CCB 9/10, CCG 8/9, AD 23/24 [13][38] |
| Public API availability | **Yes** — public unauthenticated REST JSON `/v1` + `/v2` (undocumented; no OpenAPI) [13] |
| Static file availability | **Yes** — legacy `/data/` qualifier HTML (overwritten per season) + CDN heat-sheet PDFs [13] |
| Browser requirement | **No** for every recipe exercised (robots `Allow: /`) [13][38] |
| Request cost | ~10/season universe+structure; ~200 per T&F championship week; ~6/season XC; 3/school for the coach+email build (1 per revealed address) [13][38] |
| Published rate limits | None published; zero 429 / `Retry-After` observed across the report's requests [13][38] |
| Known blocks | No historical result depth; no bulk staff directory (per-school only); email reveal is a deliberate anti-spam gate — use at low volume (12–13 reveals used in the samples) [13][38] |
| Cross-source join keys | `SchoolID` ↔ canonical school; `PersonID` ↔ coach; AN meet ids → Athletic.net; `athleticNetId` → ATN athlete id [13] |
| Estimated marginal coverage | complete IL school universe with sponsorship truth (828 / 632 / 624 / 539 / 507); 2,555 T&F state-final triples (838 Jr) + 1,214 boys XC qualifiers (358 G11) per season with association-published grade; AN identity for every T&F state-final finisher and relay leg with zero Athletic.net requests [13] |
| Implementation recommendation | **PRIMARY** (universe + structure). Secondary roles: `COACH_SOURCE`, `RESULT_SOURCE` (state finals only), `VALIDATION_SOURCE` for grades |

### 2.2 Supporting lanes — Illinois

| Source | IDs observed | Measured | Recommendation (national token) |
|---|---|---|---|
| MileSplit `il.milesplit.com` | TeamID, AthleteID, MeetID; roster `column-grad-year` | 849 HS team records (`data/milesplit-coverage-matrix.csv`); 3-team roster sample: 58 Co2027 rows measured, 37.9 % of roster | **PRIMARY** (Co2027 identity/grade evidence, per [27]); **not** a coach source |
| DAT / TFRRS | team records (no athlete id on result rows); league ids `IL Top Times 1A/2A/3A = 1467/1468/1469`, CPS 1355, IHSA Class A/AA 1088/1089, Charter/Other 1432/993 [28] | 1,744 T&F + 1,596 XC team records · 880 canonical schools · two **different** team-id enumerations, never to be summed: **4,964** distinct IL HS team IDs from [14]'s site-search POST (12,447 `high school` rows → 893 school names) and **3,340** IL rows in the retained `data/dat-team-index.csv` slice (3,315 HS-or-unknown, 25 middle school; measured 2026-09-21); no live HS performance lists; `IL Top Times` lists are 2023/2024 only; 336 Co2027 at the page's default Top-50 cut, **1,580** at `?limit=1000` [14][28] | **VALIDATION_SOURCE** (identity/school registry); Midwest label was `VALIDATION + limited ATHLETIC.NET-SEED` |
| AthleticLive / IHSA state feed | `live.athletic.net/meets/74003` (boys) / `74002` (girls), also IHSA's `MeetId` [13] | 2 meets | **RESULT_SOURCE** (state finals live) |
| Athletic.net baseline | `divListId 169070` (IL 2026 outdoor) + 168416 national [13] | two separate measurements, **not** a range: **6,009** G11 athletes on `divListId 169070` across 44 variants (`data/source-coverage-matrix.csv`, from [04]) and **6,136** G11 boys in the retained ATN seed corpus (`data/milesplit-coverage-matrix.csv` `athnet_corpus_boys_g11`); IL's 5,943–30,564 is the §0.3 **band**, never a count [04][30][CSV] | reference only (ATN lane owns it) |

## 3. Indiana — IHSAA

### 3.1 Primary association fields

| Field | Value |
|---|---|
| Source name | Indiana High School Athletic Association (IHSAA) [17] — see §0.4: Iowa's association is also IHSAA and the corpus `tools/scratch-11/` files are Iowa's, not these |
| Geographic coverage | Indiana only; three districts I 132 / II 143 / III 138 [17] |
| Sports | boys/girls XC (fall), boys/girls T&F (spring), unified T&F; indoor only at school level via the Hoosier State Relays (TFRRS season handle `423`) [17] |
| Historical depth | `/api/tournaments`: 91 T&F/XC nodes (B-TF 19, G-TF 19, B-XC 20, G-XC 20, Unified 13) spanning 2008-09 … 2026-27, plus an `Archived` node per varsity sport; the archived page carries 1996-97 … 2007-08 with 227 sectional + 59 regional + 45 state PDFs; TFRRS hubs 2023/2024/2025 [17] |
| Discovery mechanism | `GET /api/tournaments` (public JSON) → one tournament page per gender/sport/season; the school directory page self-reports membership; TFRRS `results.rss` (75 items) for new meets [17][18] |
| Stable identifiers | Association: none beyond school names and tournament nodes; result providers: TFRRS `MeetID` (26194 XC / 92370 T&F), `EventID`, `AthleteID`, `TeamID`; MileSplit `MeetID` 735020 [17][18] |
| Pagination | not observed on `/api/tournaments`; the archived-tournament page is a single page [17] |
| Athlete fields | state-final Hy-Tek rows: place, name, grade (`Yr`), school, mark, wind; TFRRS rows add `AthleteID` and per-season roster year; **no coach/AD fields anywhere** [17][18] |
| Meet fields | tournament node + site + date; TFRRS meet id and event id; Hy-Tek meet header [17] |
| Result fields | Hy-Tek PDF: place/name/yr/school/mark/wind/round/relay legs; TFRRS compiled pages expose unnamed `TIME` split columns with no unit/segment header (parse hazard) [17] |
| Grade/class evidence | state-final PDFs carry grade **and** wind (`10.50Q2.8`); TFRRS rosters carry a season-scoped year — fall-2025 `JR` and spring-2026 `Yr 11` **are** Co2027, while `&year=SR` on the 2025-26 lists is Class of 2026 (1,678) [17][18][28] |
| Coach/contact fields | **None statewide** (verified dead end): myIHSAA is identity-server gated (`/schools` 404 SPA shell), games-wanted notices live on myIHSAA, IATCCC publishes officer names/schools and one public address (`jalano@hse.k12.in.us`), district sites give names + department phones with individual emails as the exception [17][29] |
| Public API availability | **Yes** — `/api/tournaments` JSON, no auth [17] |
| Static file availability | **Yes** — sectional/regional/state Hy-Tek PDFs, incl. the 1996-97…2007-08 archive page [17] |
| Browser requirement | **No** for the association + TFRRS recipes [17][18] |
| Request cost | ~40/season for the official spine; TFRRS: **2 requests → 1,861 Co2027** (`&year=JR`, lists disjoint 1,155 + 706); 1 request → whole 2025 outdoor athlete index (24,634 ids / 402 schools) [17][18] |
| Published rate limits | None published; ~60 requests across hosts, zero 429 / `Retry-After` [17] |
| Known blocks | myIHSAA behind authentication (out of contract, not attempted); TFRRS 2026 regular-season corpus thin mid-September (56 track meets vs 1,022 in 2025; 1 XC meet vs 128 in 2025) — re-measure in November [18] |
| Cross-source join keys | school name (association ← TFRRS ← MileSplit); TFRRS `TeamID` 929 TF + 887 XC; MileSplit `MeetID`; the 2026 XC declaration's 391 slots are a per-site count, not a school list [17][18] |
| Estimated marginal coverage | authoritative 413-school universe + exact postseason site map (32→8→1 T&F, 25→5→1 XC) + provider meet ids; 1,861 Co2027 in 2 requests (largest measured single-instrument Co2027 yield in the whole study); graded state-final rows; **zero** coach coverage [17][18][28] |
| Implementation recommendation | **DISCOVERY_SOURCE** (Midwest label `DISCOVERY-ONLY`). Secondary: `RESULT_SOURCE` via the TFRRS/state-PDF layer — that athlete layer is Indiana's **PRIMARY** source and the reason IN is not a dead end despite having no coach lane |

### 3.2 Supporting lanes — Indiana

| Source | IDs observed | Measured | Recommendation |
|---|---|---|---|
| DirectAthletics / `indiana.tfrrs.org` | numeric TeamID (929 TF + 887 XC), AthleteID, MeetID, EventID; grade tokens FR/SO/JR/SR; 4-digit class = grad year | 1,816 team records in 2 requests; HSR lists 3,978 + 2,555 distinct athletes → 6,533 union; `&year=JR` → 1,861 Co2027; roster route ≈12,300 Co2027 from ≈901 requests `[INFERENCE]`; **no Athletic.net id anywhere** [18][28] | **PRIMARY** (athlete/grade layer; Midwest label `RESULT-SOURCE primary + ATHLETIC.NET-SEED`) |
| MileSplit `in.milesplit.com` | TeamID, AthleteID, MeetID, roster grad year | 533 HS teams; ≥101 2026 outdoor HS meets on pages 1–2; sitemap = 7,500 athlete URLs; 3-team roster sample measured 32 Co2027 | **PRIMARY** (category-level, per [27]); robot-allowed rosters only |
| Timing MD | live meet page `live.timingmd.net/meets/74720` (2026 state final) | 1 page; no grade observed on it [17] | **RESULT_SOURCE** (narrow) |
| Athletic.net baseline | `divListId 169137` (IN 2026 outdoor) | **2,681** G11 athletes enumerated across 37 event variants (`data/source-coverage-matrix.csv`) | reference only (ATN lane owns it) |

Indiana's measured pipeline result is the gap: `coach-contacts.csv` has **0 IN rows** and
`reports/census-by-state.csv` reports **0 coaches / 0 Co2027 with a coach** for IN — consistent with
the research verdict that no public Indiana coach directory exists.

## 4. Michigan — MHSAA

### 4.1 Primary association fields

| Field | Value |
|---|---|
| Source name | Michigan High School Athletic Association (MHSAA); site Drupal 10, portal `my.mhsaa.com` (DNN) [15] |
| Geographic coverage | Michigan only (LP + UP) [15] |
| Sports | boys/girls outdoor T&F and XC; indoor is not an MHSAA tournament series (news coverage only) [15] |
| Historical depth | finals/AN links: TF regionals 2026/2025/2024 (48 each, 100 % linked), TF finals 2023-2026 + legacy 2016-2019 qualifier links, XC regionals 2023-2025 (36 each), XC finals PDF-only; machine-readable links start ~2016 and are complete 2023-2026; archive narratives reach the 1920s [15] |
| Discovery mechanism | 1 PDF/year for schools (`26-27-Enrollment-List.pdf`); per sport per season: hub → regional page → results archive (3 pages); `sitemap.xml?page=1..7` = 12,046 URLs but contains no school pages or results [15] |
| Stable identifiers | MHSAA school ID (755 unique, stable across years, e.g. `5792` East Kentwood), co-op program ID (`hscoop.pdf`, distinct id space), **Athletic.net MeetID from MHSAA's own hrefs**; MHSAA has no meet id, no event id, no season id [15] |
| Pagination | none needed (single pages + one PDF) [15] |
| Athlete fields | finals PDFs only: name, `Yr`/`Year`, school, mark, bib; no athlete ids, no profiles; regionals have no MHSAA-hosted results [15] |
| Meet fields | AN MeetID per regional/final; site + division; no MHSAA-native meet identity [15] |
| Result fields | XC finals `Pl · Bib · Pts · Name · Year · Time · Pace`; TF finals `Athlete · Yr · Team · Finals · Pts`; `Wind` present in 2023/2024 LP TF finals; Meet-Info PDFs state the FAT/.00 vs handheld +.24 normalisation rule [15] |
| Grade/class evidence | XC finals + UP TF finals carry grade; 2023/2024 LP TF finals too: **47/187 grade-11 in `2025lpgd1final.pdf`, 30/103 in `2026-UP-Boys-D1-Finals.pdf`**; regional grade evidence does not exist at MHSAA [15] |
| Coach/contact fields | **No directory exists** (`/schools/5792` 404; AD/coach services login-gated). Host-school Meet-Info PDFs give one school's AD/coach emails per meet: ~84 host opportunities/year, 2026 TF had 35/48 PDFs, XC 18/36 to date; my.mhsaa.com `AdministrationDirectory?SchoolId=` returned AD + email for every school sampled (`data/coach-contacts.csv`: 14 MI rows, 0 coach, 14 AD-with-email) [15][29] |
| Public API availability | **No public results API**; `my.mhsaa.com` MHSAA-Endpoint JSON services answer without auth (AD directory, SportsDirectory with `SportTeamID`) [15][29] |
| Static file availability | **Yes** — enrollment/classification PDFs, co-op PDF, finals PDFs, Meet-Info PDFs [15] |
| Browser requirement | **No** — Cloudflare fronts the site but does not challenge; Drupal pages render server-side [15] |
| Request cost | ~12 HTML requests/season + optional PDFs; 1 request/school for the AD API; conditional GETs supported (`cache-control: max-age=3600`, `last-modified`) [15] |
| Published rate limits | none published; lowest-friction host in the study [15] |
| Known blocks | no regular-season results; no athlete-level enumeration; XC finals stay PDF-only (no AN links); scanned Meet-Info PDFs need OCR [15] |
| Cross-source join keys | school **name** inside PDFs (MHSAA id only in the enrollment list); AN MeetID → Athletic.net meet; MITCA/timer pages add ~11–14 further AN meet ids; `SportTeamID` links school key to meet/scoreboard endpoints [15][16][29] |
| Estimated marginal coverage | 755-school universe with stable ids + 89 AN MeetIDs/season (48 TF regionals + 5 TF finals + 36 XC regionals) + graded finals rows ≈2,000 athlete-rows/year `[INFERENCE: extrapolated from two sampled PDFs]`; host-school contacts only [15] |
| Implementation recommendation | **DISCOVERY_SOURCE** (Midwest label `ATHLETIC.NET-SEED`). Secondary: `VALIDATION_SOURCE` for grades, marginal `COACH_SOURCE` via the AD API |

### 4.2 Supporting lanes — Michigan

| Source | IDs observed | Measured | Recommendation |
|---|---|---|---|
| MileSplit `mi.milesplit.com` | TeamID, AthleteID, MeetID; roster `Class` = grad year (`0` = unknown) | 895 HS teams (+382 MS) ; 12-team roster sample; `Class=2027` verified against profiles printing "Class of 2027"; calendar: XC Sep 2026 = 254 meets, outdoor May 2026 = 942, indoor Jan 2026 = 40; robots carve-out for rosters/meets, `/api/` + `/rankings` disallowed [16][27][32] | **PRIMARY** (athlete seeding) |
| MITCA + Michigan timers | MITCA `.htm`/PDF (Meet of Champions / Team State, no grade, club labels); Superior Timing archive 2009–2026; A2RM TF 2012–2024; CHT; PT Timing (ToU prohibits extraction) | ~11–14 AN MeetIDs from MITCA + timer white-labels; `[INFERENCE]` full-year HS Ohio-style footprints not measured here | **DISCOVERY_SOURCE** (AN meet ids); `RESULT_SOURCE` narrow (MITCA files); **REJECT** PT Timing |
| MITS indoor (AthleticLIVE tenant `michiana`) | AthleticLIVE meet id, `ani` (AN MeetID, nullable), event/row docs | 37 MileSplit MI indoor meets Dec 2025–Feb 2026; MITS ES query + blob rows; **no** grade observed; cross-lane (timing providers) [39] | **CONDITIONAL** (indoor only, and it is a timing-lane surface) |
| Athletic.net baseline | `divListId 169407` (MI 2026 outdoor, from `GetNavInfo` `regions[]`) | **5,984** G11 athletes across 48 event variants; 294 distinct `TeamID` in a 707-row sample (`data/source-coverage-matrix.csv`) | reference only (ATN lane owns it) |

## 5. Ohio — OHSAA

### 5.1 Primary association fields

| Field | Value |
|---|---|
| Source name | Ohio High School Athletic Association (OHSAA) + the myOHSAA member portal [19] |
| Geographic coverage | Ohio only; 6 districts; HS 9-12 (+ MS at state championships) [19] |
| Sports | boys/girls outdoor T&F and XC; indoor **not** OHSAA-sanctioned (OATCCC runs the indoor state meet) [19] |
| Historical depth | divisional PDFs carry 2023 enrollment + 2025-26/2024-25 divisions; the Central DAB page alone links **45 distinct result PDFs** (9 for 2026 + 36 historical 2022–2025); `/sports/track/history` and `/sports/cc/history` exist but were not fetched [19] |
| Discovery mechanism | 1 request → enrollment table (815 schools); `SearchSchool?Name=` → `ohsaaId`; 18 divisional PDFs (2,353 school-gender-division rows) [19] |
| Stable identifiers | `ohsaaId`; **`OHSAA Tournament Name`** (the exact string OHSAA sets AN team names from → deterministic OhsaaSchoolId↔AN team key); AN MeetIDs 656920/656686 published as literal URLs; Hy-Tek `Event 103` (meet-local only) [19] |
| Pagination | none observed [19] |
| Athlete fields | state-final PDF: name, grade, school, event, mark, place, wind, relay legs — **no athlete ids**; myOHSAA holds no athlete data [19] |
| Meet fields | AN meet URLs (state meet 656920/656686); division/level, date, site; no OHSAA-native meet id [19] |
| Result fields | Hy-Tek PDF: place, mark, wind (`Heat 1 Preliminaries Wind: -0.3`), round/heat, implement/hurdle in event name, relay legs, records/standards; marks are rendered strings — no structured normalisation [19] |
| Grade/class evidence | **561 distinct grade-11 names in individual events, 1,067 including relay legs** from the 2026 state-final PDF (`OHSAA_State_2026_Final_Results.pdf`, 816,147 B, **83 pages** in [19]'s analysis — the same report's artifact table says 82; unresolved — 178 event-division combos) — described in [19] as the cleanest Co2027 corroboration in the Midwest; expandable via the 45 district PDFs and other districts' equivalents [19] |
| Coach/contact fields | myOHSAA, 3 requests/school: `SearchSchool` → `ohsaaId`; `SportsInformation` → XC + T&F head coaches with `mailto:` emails and `(Div-I…V)` tags; `AthleticDirector` → AD name + email + assistants/secretaries; school page adds conference, colors, mascot, county, school type, website, phone/fax. Stress sample 56 coach cells: **33 named, 100 % of named cells carry a public email** (XC 18/28, TF 15/28, 7 `TBA`, 16 `N/A`); AD 5/5 name+email; `data/coach-contacts.csv`: 31 OH rows, 15 coach rows with email, 31 AD-with-email [19][29]. OATCCC `/Contact-Us/` adds 32 role-published contacts (100 % email) incl. 16 district reps — association tier only [45] |
| Public API availability | **No results API**; portal HTML + `mailto`; PDFs on an Azure blob; the OHSAA-approved team-page listing is an Angular shell backed by a 403 API [19] |
| Static file availability | **Yes** — divisional PDFs + district/state Hy-Tek PDFs [19] |
| Browser requirement | **Only** for the OHSAA-approved team-page listing; not for the school/coach/PDF recipes [19] |
| Request cost | ~2,226 for a full coach build (742 schools × 3); ~18 PDFs for divisional alignment; 1 state-final PDF; 45 district PDFs measured on Central alone [19] |
| Published rate limits | none published; 2 s spacing used throughout [19] |
| Known blocks | 2026-27 T&F divisional table blank as of 2026-09-19 (T&F alignment for 2027 not yet published); team-page listing blocked without a browser; some coach addresses are personal-domain (data-quality flag, still role-published) [19] |
| Cross-source join keys | `ohsaaId` ↔ canonical school; Tournament Name → AN team; `FT id = "20" + ANID` (verified 28/28) [19][20]; PDF school abbreviations (`Hil. Davidso`) need mapping [19] |
| Estimated marginal coverage | 815/815 schools identified; 742 with a deterministic AN path; 556+468 XC and 685+644 T&F division rows; the whole postseason AN MeetID set; coach contacts for 59 % of stress-sampled cells with 100 % email on named cells; AD for every school sampled; 561–1,067 grade-11 state-final rows [19] |
| Implementation recommendation | **COACH_SOURCE** (Midwest label `COACH-DIRECTORY` — call it the best contact source in the study). Secondary: `DISCOVERY_SOURCE` (AN team/meet identity) + `VALIDATION_SOURCE` (grades) |

### 5.2 Supporting lanes — Ohio

| Source | IDs observed | Measured | Recommendation |
|---|---|---|---|
| MileSplit `oh.milesplit.com` | TeamID, AthleteID, MeetID; roster `gradYear`; `/api/v1/meets/{id}/performances` | 977 HS teams; 10-team 2026 XC sample → 41 Co2027 (≈4.1/team-season → ~4,000 Co2027 from ~977 requests `[INFERENCE]`); OATCCC indoor MeetIDs 718766/718840-718843 [20][27] | **PRIMARY** (Co2027 identity + results) |
| Finish Timing | AN MeetIDs (29 on one homepage fetch; `FT id = "20"+ANID` verified 28/28); `/2026/CC/` 32 XC dirs; 325 files 2026 YTD | free grade-bearing result files; the only Ohio source handing over AN IDs for free [20] | **RESULT_SOURCE** (primary Ohio result lane) |
| Baum's Page | meet enumeration 2003–2026; Hy-Tek `Year` column empty in every sampled file | NW/central OH small-school meets [20] | **RESULT_SOURCE** (secondary) / `DISCOVERY_SOURCE` |
| Track Scoreboard (MileSplit Live) | live only | meet-weekend validation [20] | **VALIDATION_SOURCE** |
| OATCCC | association contacts (32, 100 % email); indoor MeetIDs → MileSplit | indoor is the only Ohio indoor championship; no member directory [45] | **COACH_SOURCE** (association tier) + `DISCOVERY_SOURCE` |
| Ohio Runner | — | road races only [20] | **REJECT** |
| Athletic.net baseline | `divListId 170050` (OH 2026 outdoor) | **6,361** G11 athletes across 51 event variants; all 815 schools have ATN team pages (`data/source-coverage-matrix.csv`) | reference only (ATN lane owns it) |

## 6. Wisconsin — WIAA

### 6.1 Primary association fields

| Field | Value |
|---|---|
| Source name | Wisconsin Interscholastic Athletic Association (WIAA); school database `schools.wiaawi.org` [06] |
| Geographic coverage | Wisconsin only; 7 administrative districts; divisions 1–4 + Wheelchair [06] |
| Sports | boys/girls T&F (tournament series), boys/girls XC; **no indoor** (sitemap-verified); **no regular season** (contest table empty for T&F/XC) [06] |
| Historical depth | boys TF archive 1,639 result-file links / 26 file-years (2000-2026); boys XC 1,777 / 74 years (1952-2025); girls XC 1,738 / 74; per-meet files from 2003 (36), 2004-05 (46/64), 60-135/year from 2011; 2026 boys T&F archive re-counted at **103 unique links** [06][35] |
| Discovery mechanism | 26 letter POSTs (recipe; only letter **A** was executed = 39 schools) → per-school detail by `orgID`; `RunSportListReport` + `GetSSList` = 4 requests/season → **1,796 team-seasons**; two pages per sport for result files (tournament page + state archive) [06] |
| Stable identifiers | `OrganizationID`, `TeamID`, `SportSeasonID` (BTF 1544, GTF 1558, BXC 1537, GXC 1549), `ContactID`; result **files** have no ids — only URLs [06] |
| Pagination | directory paged by letter; RunSportListReport returns whole sport-season lists [06] |
| Athlete fields | tournament result files only: name, `Yr` 9-12, school, mark, heat, round, place, relay legs; no athlete ids/profiles [06] |
| Meet fields | tournament meet set per sport/round; XC tournament page links `live.pttiming.com/xc-ptt.html?mid=5127`; T&F regionals exist on Athletic.net (2026 instructions PDF) [06][35] |
| Result fields | mark, heat/round, place, date, school, relay membership, tie-breaks to 3 decimals; wind in state T&F PDFs only; no timing method/implement spec [06]; formats: Athletic.net export, Hy-Tek, HTML, legacy `.txt` (63 on the girls XC archive alone, incl. 1999); ~a third of sectional uploads are image-only — measured 24/26 extract with `pdftotext`, the other 2 need OCR (tesseract recovers at 1.06 s/page) [06][35] |
| Grade/class evidence | `Yr` on tournament files: **2,786 grade-11 rows** recovered by [35] from a 26-file 2026 sample; the retained per-file summary behind that sample (`evidence/gaps/35/pdf-extract/summary-v2.json`, 26 entries) sums to **2,653** — [35]'s matcher is the more tolerant, so the sample's yield is a 2,653–2,786 range, not one number. Worked example `tr2026homesteadsectional.txt` = 69 grade-11 rows (matches [35]); single D1 Regional 5A file = 234 distinct athletes / 71 grade-11 [06][35] |
| Coach/contact fields | `GetDirectorySchool?orgID=` returns School Administrators (Superintendent, Principal, Athletic Director, City-Wide AD, Assistant AD) and Head Coaches per sport, all with published emails; 9-school sample: BXC 8/9, GXC 9/9, BTF 7/9, GTF 8/9, 9/9 with ≥1 TF/XC email; `data/coach-contacts.csv`: 22 WI rows → 17 coach rows with email + 22 AD-with-email. **No change signal exists** → poll per school (1 request ≈195 KiB; 516-school sweep ≈103 MB/≈5 min; recommend 10 schools/week) [06][29][35] |
| Public API availability | **No documented API**; ASP.NET POST fragments (directory letters, `RunSportListReport`, `ContactInfoResults`) + small JSON endpoints [06] |
| Static file availability | **Yes** — 104 result-file links/season per the coverage matrix (103 unique 2026 boys links measured in [35]); PDF/HTML/`.txt`, back to 1952 [06][35] |
| Browser requirement | **No** (no anti-forgery token on the POSTs) [06] |
| Request cost | 4/season team universes; 26 letter POSTs + 1 detail GET per school (a **516**-school sweep ≈ 542 requests); [29]'s cost model prints "~655" because it assumed **629** schools — that 629 has no captured count behind it and conflicts with [06]/[35]'s 516, so use 516; 1/school for contact refresh [06][29][35] |
| Published rate limits | none; no 429 / `Retry-After` in the whole session; `/Reports/` returns 403 [06][35] |
| Known blocks | no athlete enumeration; no indoor; no regular season; image-only PDFs; **no HTTP validators** (`no-cache, no-store` on conditional GET) so refreshes are full reads [06][35] |
| Cross-source join keys | `TeamID`/`SportSeasonID`/`OrganizationID`; meet name+date (no id) → AN/TFRRS by name; AN MeetIDs come from WIAA's own instructions PDFs (5 URLs incl. XC sectional date indexes `2026-10-23`/`2026-10-24`) and from RUNMEET titles in result files (13/26 sampled) [06][35] |
| Estimated marginal coverage | 516-school universe (WIAA-published; the 26-letter enumeration is a recipe, not a run) + all 1,796 T&F/XC team-seasons with stable TeamIDs; 104 result files/season with `Yr` grades (2,653–2,786 G11 rows from a 26-file sample, §6.1); coach+AD email table with no in-study peer; deterministic AN tournament seeds [06][35] |
| Implementation recommendation | **PRIMARY** (school/team universe). Secondary: `COACH_SOURCE` (highest unique marginal value), `VALIDATION_SOURCE` (tournament grades), `RESULT_SOURCE` (tournament series only) |

### 6.2 Supporting lanes — Wisconsin

| Source | IDs observed | Measured | Recommendation |
|---|---|---|---|
| PrimeTime Timing | Hy-Tek/PDF result files; `exEntryLink` → AN/MileSplit meet URL | JSON API 2017–2026; 319 WI rows 2025 / 255 2026; 266 WI result PDFs 2025 + 200 YTD 2026; `exEntryLink` split measured — 2026: Athletic.net 195 · MileSplit 0 · none 60; 2025: MileSplit 163 · Athletic.net 76 · none 80 [08]; `data/source-coverage-matrix.csv`'s WI row prints the coarser 271-of-302 AN-link count for the same source — different denominator, kept side by side, never averaged | **RESULT_SOURCE** (primary for WI) |
| Performance Timing LLC | WordPress REST `?after=`/`?search=`; `CId/RId` ids | XC PDFs carry No/name/school/grade/qualifier/time/points/pace; [08] called 2026 track PDFs non-extractable — [35] corrects that to **file-specific** (24/26 extract with `pdftotext`; the 2 failures are zero-font raster and Type-3-without-ToUnicode, both OCR-recoverable at 1.06 s/page) [08][35] | **RESULT_SOURCE** (secondary; XC preferred) |
| MileSplit `wi.milesplit.com` | TeamID, AthleteID, MeetID; roster `column-grad-year` | 597 HS teams; state meet 2,454 rows / 382 Co2027 (robots-disallowed API); roster grad year verified against the profile bio [07][27] | **RESULT_SOURCE** (primary) + `VALIDATION_SOURCE` |
| TrackSide (72797), K2 (74899), OnYourMarks | AN team pages as meet hubs; tenant ids | OnYourMarks lists 0 meets on MileSplit but is credited on the official 2025 D1 De Pere XC file → never use MileSplit timer counts as a completeness measure [08] | **DISCOVERY_SOURCE** (tenant link graph) |
| Karmarush / `ptt-franklin.firebaseio.com` live feed | athlete ids, event ids, live marks | richest structured source available — but its own terms prohibit automated access/bulk extraction/ML grounding | **REJECT** |
| Athletic.net baseline | `divListId 170770` (WI 2026 outdoor) | **3,924** G11 athletes across 39 event variants (`data/source-coverage-matrix.csv`) | reference only (ATN lane owns it) |

## 7. Cross-cutting notes

### 7.1 What no lane in this batch can answer (carried forward as gaps)

1. **Co2027 denominators** (§0.3) — the one number a national coverage report will want and the five
   states' own research did not measure.
2. **MI and IN per-sport participation counts** — MHSAA class sizes and IHSAA declared slots are not
   substitutes.
3. **Illinois timing providers** — the Midwest set has dedicated reports for WI [08], MI [16], OH [20]
   and the DAT/MileSplit super-indexes [27][28]; no Illinois timer report exists, so IL's timer layer
   is described only by the IHSA state feed and DAT league pages [13][14].
4. **Roster completeness per team** — MileSplit/DAT samples are 3–12 teams per state; team-registry
   counts are exact, per-team roster coverage is not.
5. **Grade on the regular season** — only WI (tournament files) gives association-published grade
   outside the state meet; IL/OH state finals and MI finals PDFs are championship-scoped.

### 7.2 Repo adapter status (snapshot 2026-09-21, sources read-only)

Verified by directory enumeration of `crates/census-service/src/sources/` (26 entries) and a
repo-wide search for `mhsaa|ihsaa|tfrrs|directathletics`:

| Jurisdiction | Adapter now in repo | Notes |
|---|---|---|
| IL | `src/sources/ihsa/` (`collect.rs`, `parse.rs`, `staff.rs`, `map.rs`, `tests.rs`) | fixtures `crates/census-service/tests/fixtures/ihsa/{v1_schools.json, staff2_coach_rich.json, staff2_office_only.json}` |
| OH | `src/sources/ohsaa/` (`collect.rs`, `parse.rs`, `pages.rs`, `map.rs`, `tests.rs`) | fixtures `.../fixtures/ohsaa/` (9 files) |
| WI | `src/sources/wiaa/` + `src/sources/wiaa_results/` | fixtures `.../fixtures/wiaa/` (4), `.../fixtures/wiaa_results/` (6), `.../fixtures/milesplit/wi_roster_52649.html` |
| IN | **none** — no `ihsaa`/`tfrrs` module anywhere in the crate | Hy-Tek PDFs can still be read by the generic `hytek/` format parser; `milesplit/` covers `in.milesplit.com` via `Site::for_jurisdiction(UsJurisdiction::Indiana)` |
| MI | **none** — no `mhsaa` module; `src/sources/coach_contacts/wire.rs` recognizes the `mhsaa` host for CSV-import source attribution, and `tests/golden/coach_contacts__*` carry `my.mhsaa.com` AD endpoint rows | MHSAA finals PDFs would go through the generic `hytek/` parser; `milesplit/` covers `mi.milesplit.com` |
| All five | `src/sources/milesplit/` (per-jurisdiction `Site`, host = `<lowercase code>.milesplit.com`), `hytek/`, `compiled/`, `xc/`, `result_file/`, `raceday/` | generic format lanes; `compiled/` = Athletic.net "Compiled" export layout |

Nothing in this lane was modified; the only writes are the four files in this directory.

### 7.3 Evidence ledger (commands run for this report)

```
# CSV slices (Eval/IPython kernel, no repo writes)
python: csv.DictReader over ~/Downloads/midwest-tfxc-source-research/data/{school-alias-map,
  dat-team-index, canonical-schools, coach-contacts, source-coverage-matrix, milesplit-coverage-matrix}.csv
  → per-state school/DAT-team/coach row counts and the 5-state source-role rows
# byte sizes of cited captures
bash: stat -c '%s %n' ... (evidence/ gaps/38, gaps/39, gaps/45, gaps/33, gaps/32, gap-closure,
  crates/census-service/tests/fixtures/{wiaa,ihsa,ohsaa,mshsl,milesplit,wiaa_results})
# adapter inventory
read/glob: crates/census-service/src/sources listing + recursive **/mhsaa* **/tfrrs* glob (empty)
grep: (?i)(tfrrs|ihsaa|mhsaa|directathletics) over the repo
# 2026-09-21 re-derivation pass (csv.DictReader in the same kernel, no repo writes)
python: data/coach-contacts.csv -> 2,298 rows / 874 distinct schools / 51 public_professional_email /
  620 ad_email, role histogram, per-state rows+mail counts (IL 16/12/16, IN 0/0/0, MI 14/0/14,
  OH 31/15/31, WI 22/17/22)
python: reports/census-by-state.csv -> coaches / coaches_with_email = IL 15,102/3,235, OH 28/23,
  WI 2,550/2,155, MI 6/6, IN 0/0
python: data/milesplit-coverage-matrix.csv -> IL athnet_corpus_boys_g11 = 6,136, band 5,943-30,564
grep: 13-illinois-ihsa.md -> TRB 632 / TRG 624 / CCB 539 / CCG 507 and "~200 requests per T&F
  championship week"; 17-indiana-ihsaa.md -> "~40 requests/season"; 6,009 -> row IL of
  data/source-coverage-matrix.csv ("divListId 169070; 6,009 G11 athletes (44 variants)")
```

Raw numbers used above, as printed by those commands, are reproduced in `coverage.json`
(`denominator_policy.measured_universes` plus the per-jurisdiction count blocks) and
`samples/CAPTURES.md` (paths + bytes).

**Self-check sweep (run against this lane's own deliverables, 2026-09-21):**

```
python: parse every `| path | bytes |` row of samples/CAPTURES.md, os.path.getsize each
  → 248/248 single-path byte claims match (25 repo-relative, 223 corpus-relative); 0 mismatch, 0 missing
python: aggregate totals for the multi-path rows
  → scratch-28 DAT 1428/1429 family 15 files / 12,861,466 B; gaps/38 remainder 17 of 57 / 8,146,413 B;
    gaps/45 remainder 50 of 76 / 12,781,361 B; scratch-28 dir 91 files / 21,750,011 B - all match the
    figures printed in CAPTURES.md (one first-pass arithmetic slip, 8,036,413 vs 8,146,413, was caught
    by this check and corrected)
python: re-derive the two re-derivable denominators from retained bytes
  → gaps/38/il-schools.json: 828 rows, md5 498179f68c66793396dd4c6123a4a4bc, minified, no trailing
    whitespace; 801 + 26 + 1 members; 604/224 boundary; 677/151 public+private; 126 CPS - its ledger
    records 451,182 B vs 451,188 B on disk (6 B, unexplained, both printed)
  → gaps/45/ohsaa-enrollment.html: 1 <table>, 816 <tr> = 1 header + 815 data rows, 815 distinct
    OhsaaSchoolId (100-1000027), class cell 267 AAA / 267 AA / 267 A / 14 blank
python: per-file state identity of every `tools/scratch-11/*ihsaa*` file this lane had attributed to
  Indiana
  → ALL IOWA (task-relevant correction): `iahsaa` links / literal "Iowa" counts in the bytes, 0
    "Indiana"; schools Waukee, Dowling, Ankeny Centennial, ACGC, AGWSR; conferences SEISC / River
    Valley. The whole `tools/scratch-11/` dir is the [11] Iowa lane's (`11-iowa-ihsaa-ighsau.md`), so
    CAPTURES B5 was retitled and the 13 Iowa files moved to a new B5-IA section, with the Indiana rows
    rebuilt from the content-verified `tools/scratch-28/` captures (4,453 / 7,806 `Indiana` tokens)
python: modal pipe-count per markdown table in SOURCE_REPORT.md and samples/CAPTURES.md
  → 0 malformed rows in both (5 embedded-pipe rows found and repaired across the sweep: 2 in
    SOURCE_REPORT.md, 3 in CAPTURES.md)
python: json.load(schema.json), json.load(coverage.json) → both parse; coverage.json carries
  all five jurisdictions with adapter status, universes, ATN divListId, enumerates/excludes
python: crates/census-service/tests/fixtures/ihsa/v1_schools.json → **3 rows**, i.e. a sample of
  the 828-row payload, not the payload; §0.3 / §1 and the coverage.json provenances state this
  explicitly so no downstream report can mistake the fixture for the IL denominator
grep: crates/census-service/{src,tests} for "mhsaa" → `src/sources/coach_contacts/wire.rs:64-65`
  (host→`mhsaa` mapping) + `tests/golden/coach_contacts__*` (`my.mhsaa.com` AD URLs, "state":"MI")
  — the only MHSAA touchpoints in the crate, as the §7.2 MI row claims
```


### 7.4 Measured coach-contact yield in `data/coach-contacts.csv`

Whole file: **2,298 rows, 874 distinct schools, 51 rows carrying a coach address
(`public_professional_email`), 620 carrying an AD address (`ad_email`)** — `csv.DictReader` counts over
`~/Downloads/midwest-tfxc-source-research/data/coach-contacts.csv`, 2026-09-21. (An earlier `awk -F,`
pass read 626 AD rows; `awk -F,` miscounts on quoted commas, so the quote-safe reader above is the
number this report stands behind.) Rows are one-per-role: 611 `Athletic Director`, 300 + 298 T&F head
coaches (girls/boys), 270 + 270 XC head coaches, 286 `Activities Director`.

| State | rows | coach email | AD email | reading |
|---|---:|---:|---:|---|
| IL | 16 | 12 | 16 | IHSA `staff2` sample slice — not the full IL directory |
| IN | 0 | 0 | 0 | consistent with "no public Indiana coach directory exists" (§3) |
| MI | 14 | 0 | 14 | my.mhsaa.com AD endpoint only; MHSAA publishes no coach rows at all |
| OH | 31 | 15 | 31 | myOHSAA 3-request path on the stress sample |
| WI | 22 | 17 | 22 | `GetDirectorySchool` sample over 9 schools |

These are **sample-scale** rows, not a coverage claim. The operational numbers are the pipeline census
columns (`census-by-state.csv` `coaches` / `coaches_with_email`): IL 15,102/3,235 · OH 28/23 · WI
2,550/2,155 · MI 6/6 · IN 0/0. Note that MI's 6 coach rows and IN's 0 are *pipeline* facts, not
research estimates — MI's 6 arrive via the `mhsaa` host branch of the coach-CSV importer.

## 8. Source index (paths cited as `[NN]`)

`[06]` `research/midwest/06-wisconsin-wiaa.md` · `[07]` `07-wisconsin-milesplit.md` ·
`[08]` `08-wisconsin-timing-providers.md` · `[13]` `13-illinois-ihsa.md` ·
`[14]` `14-illinois-directathletics.md` · `[15]` `15-michigan-mhsaa.md` ·
`[16]` `16-michigan-alternatives.md` · `[17]` `17-indiana-ihsaa.md` ·
`[18]` `18-indiana-directathletics-milesplit.md` · `[19]` `19-ohio-ohsaa.md` ·
`[20]` `20-ohio-independent-sources.md` · `[27]` `27-milesplit-super-index.md` ·
`[28]` `28-directathletics-super-index.md` · `[29]` `29-coach-contact-graph.md` ·
`[30]` `30-cross-source-pareto.md` · `[32]` `32-milesplit-paywall-boundary.md` ·
`[35]` `35-wisconsin-gap-followups.md` · `[38]` `38-illinois-gap-followups.md` ·
`[39]` `39-michigan-gap-followups.md` · `[45]` `45-ohio-gap-followups.md` —
all under `~/Downloads/midwest-tfxc-source-research/`; CSV citations are paths under
`~/Downloads/midwest-tfxc-source-research/data/` and `.../reports/`.
