# 41. Kansas gap follow-ups (report 23 open items)

Status: complete — all four follow-up items answered with live evidence; the only sub-item left
unresolved is whether Bound (gobound.com) can serve Kansas grade rosters (roster rows are
JS-loaded and no public JSON endpoint was visible in the page source; not needed, since three
other sources answer the question).
Observed on: 2026-09-20 (America/Chicago, 09:05:43–09:13 CDT)

Scope: the four open items carried in report 23 (`23-kansas-kshsaa-directathletics.md`) that the
Kansas slice had not closed:

1. `kshsaachamps` returns `Grade: null` for every TF row — do alternate pages/params (per-class
   pages, results vs participants views) expose grade/Year for TF?
2. Find grade-bearing rosters **beyond champions**: MileSplit KS team rosters, kshsaa.org meet
   results, any AthleticLIVE Kansas tenant.
3. Coach/AD coverage re-check on ≥10 **fresh** Kansas schools from the official KSHSAA directory.
4. Record the KSHSAA classification structure (classes, counts) from official pages.

Method: plain `curl` with a desktop-Chrome User-Agent plus `read`/`jq`/`pdftotext` for parsing;
77 HTTP requests total, sequential per host with ≥1.2 s spacing, none to any `*.athletic.net`
host; no auth/CAPTCHA/paywall bypass, no cookie replay; no 429 and no `Retry-After` observed
anywhere. Raw captures: `research/midwest/evidence/gaps/41/`; scratch parsing happened in
`/tmp/gap41` (outside the campaign tree, not part of the deliverable). The repo was not touched.

---

## Answers to the four follow-up items

### 1. TF grade in the KSHSAA championship system: **no page, param or view exposes it** (confirmed negative)

| Probe | Result |
|---|---|
| `RetrieveResultsByActivity?activityID=51` (TF boys) 2026 | 200, 235,224 B — 864 individual rows, **864/864 `Grade: null`**; `ActivityConfiguration.IndividualResultsSectionFieldApplicable_Grade = false` |
| `RetrieveResultsByActivity?activityID=50` (TF girls) 2026 | 200, 237,810 B — 873 rows, **0 graded**; same flag `false` |
| `RetrieveResultsByActivity?activityID=51` **2015** | 200, 278,229 B — 850 rows, 0 graded → the gap is activity-level, not a per-year defect |
| `RetrieveResultsByActivityYearClassID?activityYearID=2346` ("per-class" view) | 200, **235,224 B — byte-identical payload** to the full-year call; 864 rows, 0 graded. The UI's per-class grouping is client-side; the endpoint takes `activityYearID` (year+activity), not a class |
| `RetrieveResultsByCriteria?...activityID=51&schoolID=646` ("results/participants by school") | 200, 3,815 B — 12 HTML placement strings ("…placed 1st… coached by Levi Huseman"); **no grade column, no participants section** |
| `RetrieveResultsByActivity?activityID=68` (indoor TF girls) 2026 | 200, 1,508 B — config present, `…FieldApplicable_Grade=false`, 0 rows |
| Endpoint inventory from the site's own JS (`ResultsQuery.js`, `Shared.js`, `ChampHistory.js`) | only `RetrieveResultsByActivity`, `RetrieveResultsByActivityYearClassID`, `RetrieveResultsByCriteria`, `RetrieveRoster`, `RetrieveChampionHistory`, `SearchDetailPopup`; grep for `Participant` = 0 hits |
| Control: `RetrieveResultsByActivity?activityID=57` (XC boys) 2025 | 200 — flag `true`, **143/143 rows graded**, 50 grade-11 → the flag is a real per-activity data gate, not a rendering accident |

The **only** TF grade inside the KSHSAA/kshsaachamps system is the champion-team roster
(`/Shared/RetrieveRoster`), which the payload flags via `RosterCount>0` / `RosterExists=true` on
**first place only** (2 negative tests: TF 6A 2nd-place school 660 → `[]`; XC 6A 2nd-place → `[]`).
2026 TF champion rosters: boys 18/25/29/15/15/12 = 114 athletes, girls 18/23/30/20/11/7 = 109
athletes, each roster graded (e.g. girls 6A Olathe North: 18 rows, grades 9:2 / 10:5 / 11:3 / 12:8).

**Where TF grade actually is** (item 2): Midwest Timing's official state-meet PDFs (a `Year`
column per athlete), AthleticLIVE Kansas state meets 2017–2021 (the 2022 state rows export no
grade values), and MileSplit/DAT rosters.

### 2. Grade-bearing rosters/results beyond champions: **four working sources, measured**

| Source | Grade surface | Measured coverage |
|---|---|---|
| **AthleticLIVE `athlete_list` (ES)** | per-row `y` = grade + `ani` = Athletic.net athlete ID + `t.n`/`t.ani` = school + AN team ID | **842 distinct Kansas meets** (232 XC / 520 outdoor / 90 indoor, 2015–2026) across 12 tenants; **354,733 athlete rows**; season-scoped: TF outdoor 2026 = 107 meets/42,023 rows (g11 4,793, g12 4,063, 88.9 % carry an AN athlete ID), XC 2026 = 21 meets/5,882 rows (100 % graded, g12 651 = Class of 2027), XC 2025 = 45 meets/11,132 rows (g11 1,637). Kansas state TF 2017/2018/2019/2021 are on this corpus **with grade** (2021: 3,699 rows, 1,079 grade-11); the 2022 state meet is present (3,695 rows) but its grade column is empty/NBSP for every row, so grade coverage is meet-config-dependent, not platform-wide |
| **Midwest Timing & Results PDFs** (timer of the KS state TF meet; 336 events in 2026, 279 with compiled PDFs) | state performance lists: `Year` column; compiled results: `YR` column; relay heatsheets: grade per leg | performance-1A = **546 graded entry rows** (9:74/10:114/**11:164**/12:194, 30 individual events, 18–22 entries each); performance-6A = **544 graded rows** (52/89/**184**/219); relay heatsheet = **4,336 graded leg tokens** (grade-11 = 1,192) across 36 relay events (4x100/4x400/4x800 × 2 genders × 6 classes); regular-season example Erie XC 2026-09-05 = `Athlete / YR / # / Team / Score / Time` with FR/SO/JR/SR for varsity+JV |
| **MileSplit KS team rosters** (`/teams/<id>-<slug>/roster`, server-rendered, free) | `column-grad-year` = graduating class | 413 KS HS team records; measured Olathe North = 235 athletes {2027:86, 2028:76, 2029:58, 2030:15}, St. Thomas Aquinas = 150 {2027:61, 2028:47, 2029:29, 2030:11, unset:2} — **147 Class-of-2027 athletes from 2 requests** |
| **DirectAthletics KS team pages** | roster `Year` column (FR/SO/JR/SR) | fresh sample `/teams/track/100887.html` = 17 row tokens (JR 7 / SR 6 / SO 3 / one `13`); league 1252 ("KSHSAA 1A") page enumerates 249 team pages |
| kshsaa.org meet results | **none** — `/schedules/trackFieldSchedule/teamschedule/735/1` → 404; XC standings empty and weekly schedules 500 (report 23); "Meets & Results" cards link to MileSplit only | 0 |
| Bound (gobound.com/ks) | roster rows JS-loaded (report 23 saw an empty `Name | Year` table); page source exposes no public JSON endpoint | not pursued (covered by the four sources above) |

### 3. Coach/AD re-check on fresh schools: **AD coverage is 100 %, and there is still no coach field**

- 12 **fresh** schools (never individually fetched in report 23), one/two per class, ids
  458/464/465/467/469/470/480/537/559/800/1185/1229 (Abilene, Andale, Andover, Argonia,
  Arma-Northeast, Ashland, Baxter Springs, Atchison Co Community, Anderson County, Blue Valley
  North, Blue Valley Northwest, Andover Central): **12/12** with `ADName`, `ADEmail`, `WebSite`,
  `SchoolPhone`, `PrincipalName`, `Identifier`; identical 39-field keyset on every record.
- Whole-directory re-measure: `directory/search/name/HS/` = 339 records, **339/339 AD name+email**,
  **329/339 WebSite** (matches report 23/report 29).
- No coach field exists anywhere in the record (fields: `ADName/ADEmail/ADCell/ADFax`,
  `PrincipalName/Email/PrincipalCell`, `PresName/PresEmail/PresCell`, addresses, `USD`, `Class`,
  `FBClass`, `League`, `WebSite`, `TwitterUserName`, `DateModified`). Searching a known head
  coach's surname (`directory/search/name/Huseman/`, Olathe North's coach) returns **`[]`**; the
  2.9 MB SPA bundle builds only `directory/search/name/<q>/` and `.../city/<q>`.
- Freshness signal: `DateModified` across the 339 HS records spans 2009→2026;
  **152/339 (44.8 %) were last modified before 2023** (2009:6, 2010:13, 2011:7, 2012:2, 2013:1,
  2014:3, 2015:24, 2016:4, 2018:3, 2019:16, 2020:6, 2021:51, 2022:16) → diff on `DateModified`,
  don't trust "current" AD records.
- Sport-level coach names remain available only through kshsaachamps champion metadata: 2026 TF
  boys **6/6** champions carry a coach string, 2026 TF girls **5/6**, 2025 XC state champions
  **6/6** carry `HC:`/`AC:` role strings; the 2025 XC regional section (23 team rows, 4/4/4/4/4/3
  per class) carries **0**.

### 4. KSHSAA classification structure (official)

- Official data layer: `GET https://kshsaa-api.kshsaa.org/PublicClassifications` (the endpoint the
  official page `kshsaa.org → Directories → Classifications → General` calls; the SPA deep link
  `/general/PublicClassifications` itself is **404 to a plain client** — it is client-side routed).
  Year **2026 = 2025-2026**, `status 1`:

| Class | Member schools | Enrollment range (min–max) |
|---|---|---|
| 6A | 36 | 1323–2481 |
| 5A | 36 | 814–1283 |
| 4A | 36 | 308–680 |
| 3A | 64 | 169–301 |
| 2A | 64 | 107–168 |
| 1A | 112 | 12–106 |
| **Total** | **348** | — |

  `moved` vs the previous cycle: same 330 / up 9 / down 9. `?year=2025` and `?year=2027` both
  return the identical 2025-26 payload — the API exposes only the current cycle.
- Football is classified separately (official PDF `FootballClassifications_Current.pdf`, "2026 &
  2027", enrollment = grades 9-11 as of 2025-09-22): 6A 32 (1919–1067), 5A 32 (1037–686),
  4A 32 (672–318), 3A 40 (317–184), 2A 40 (181–124), 1A 37 (124–69), 8-Player Division I
  40 (104–70), 8-Player Division II 50 (69–28). Note the 1A / 8-Player enrollment ranges overlap —
  8-player is a separate opt-in division `[INFERENCE from the overlapping ranges]`.
- State-series usage of the 6 general classes: 2026 XC state runs as two 3-class sites
  (3A/5A/6A at Rim Rock Farm; 1A/2A/4A at Wamego CC, both Oct 31 2026) and 2026 TF state runs all
  six classes at one site (Crossland Stadium, WSU, May 29–30 2026) with wheelchair events — both
  from official KSHSAA PDFs.
- 348 classified schools vs 339 records in the `name/HS/` search: the 9-school delta is the
  non-"HS"-suffixed members (academies etc.); report 23 verified the `HS`+`Academy`+`School`
  union covers the whole classified set.

---

## Source

| # | Source | URLs used | Role in this follow-up |
|---|---|---|---|
| 1 | AthleticLIVE (Athletic.net white-label live-results platform) | `https://search.athletic.live/*_meet_list`, `/athlete_list/_search`; `https://athleticlive.blob.core.windows.net/$web/*` | **primary new find** — Kansas meet universe + per-athlete grade + AN athlete/team IDs |
| 2 | Midwest Timing & Results (Kansas timer) | `https://midwesttiming.com/` (SPA) + `https://wkyzbfnhunrtyqjewpna.supabase.co/rest/v1/*` + public storage PDFs | grade-bearing state-meet and regular-season results PDFs |
| 3 | MileSplit Kansas | `https://ks.milesplit.com/teams/<id>-<slug>/roster`, `/teams?type=1`, `/meets/751194-…/entries` | free Class-of-2027 team rosters |
| 4 | KSHSAA championship history + results | `https://www.kshsaachamps.org/Results/*`, `/Shared/RetrieveRoster`, `/Scripts/kshsaa/*.js` | TF grade question (item 1) + champion rosters |
| 5 | KSHSAA public JSON API | `https://kshsaa-api.kshsaa.org/PublicClassifications`, `/directory/*` | classification structure + fresh-school AD re-check |
| 6 | KSHSAA website assets | `https://www.kshsaa.org/react/assets/index-f33aafe3.js`, `/Public/{CrossCountry,Track,Football}/PDF/*.pdf` | official route/PDF discovery (schedule + football classification) |
| 7 | DirectAthletics Kansas slice | `https://www.directathletics.com/leagues/track/1252.html`, `/teams/track/100887.html` | DAT roster `Year` re-check |
| 8 | Bound Kansas | `https://www.gobound.com/ks/schools/shawneemissioneasths` | roster surface check (negative) |

## Coverage

- **States**: Kansas only. AthleticLIVE meets are classified by the `lsa` field; all counts below
  are `lsa.keyword = "Kansas"`.
- **Sports**: boys/girls XC, indoor TF, outdoor TF. Kansas AthleticLIVE corpus: **842 distinct
  meets** — XC 232, outdoor 520, indoor 90 — 2015-04-25 → 2026-10-06 (newest is a future-dated
  October meet; one `TEST 637441 Collin1` record dated 2222-04-15 excluded from year counts).
- **Tenants serving Kansas**: `blacksquirrel` 472, `reddirt` 211, `athleticlive` 88, `runaround`
  28, `letsgoruntiming` 13, `enduro` 12, `kcrc` 12, `aatiming` 2, `adkinstrak` 1,
  `athletictiming` 1, `michiana` 1, `wayzata` 1 (deduped by meet id; the global
  `live_results_meet_list` also carries all 842).
- **School levels inside the corpus**: mixed — HS invitationals, JH/MS meets (grades 5–8 rows are
  numerous: XC `8`:7,732 / `7`:6,689 rows), plus college meets (NJCAA/Region VI/MIAA under
  `blacksquirrel` with FR/SO/JR/SR as *college* class years). Any collector must filter by meet
  (name/tenant) and/or row grade, not assume "Kansas meet = HS".
- **Historical depth**: AthleticLIVE Kansas 2015–2026; per-year meet counts 2017:23, 2018:24,
  2019:23, 2020:25, 2021:46, 2022:109, 2023:144, 2024:153, 2025:158, 2026:134 (partial).
  Midwest Timing: 336 events for 2026 (279 with PDFs), their MileSplit timing page reaches 2007.
  KSHSAA classification: current cycle only (2025-2026).
- **Class-of-2027 math**: in the 2025-26 season Grade 11 = Class of 2027; in 2026-27 (current)
  Grade 12 = Class of 2027. All counts below are stated as raw grade values so the mapping is
  explicit.

## Enumeration

**Kansas meets (AthleticLIVE) — 1 request.**
```
POST https://search.athletic.live/*_meet_list/_search
{"size":2000,"_source":["i","n","sd","o","e","ani"],
 "query":{"term":{"lsa.keyword":"Kansas"}},"sort":[{"sd":{"order":"desc"}}]}
```
→ 1,694 hits → `unique_by(.i)` = **842 meets**; fields: `i` AthleticLIVE meet id, `n`/`ln` name,
`sd`/`ed` start/end date, `o` sport (`xc|indoor|outdoor`), `e` tenant, `ani` Athletic.net MeetID,
`ls`/`lsa` city+state, `tna` timer name.

**Athletes of those meets — 1–N ES queries.**
```
POST https://search.athletic.live/athlete_list/_search
{"size":0,"query":{"terms":{"mi":[<meet ids>]}},"aggs":{...}}
```
with `mi` = the meet id list (or `{"term":{"mi":<id>}}` for one meet). Row fields: `i` row id,
`n`/`fn`/`l` name, **`y` grade** (keyword: `"9"…"12"` or `FR|SO|JR|SR`), `g` gender, `mi` meet id,
**`ani` Athletic.net athlete ID**, `t{ i, n, f, mi, ani, cco, lg }` team (school name + AN team id).
Kansas-wide: 354,733 rows for the 842 meets; `terms` list is capped at 65,536 values, and results
paginate at 10,000 hits (`from/size`, scroll, or partition by tenant/year).

**KSHSAA class structure — 1 request** (`GET /PublicClassifications`, see item 4).

**Schools/ADs — 2 requests**: `GET /directory/search/name/HS/` (339 HS records with AD contact in
one response) or `GET /directory/Id/{id}` per school (full record).

**MileSplit KS rosters — 1 request/school**: `GET https://ks.milesplit.com/teams/<TeamID>-<slug>/roster`
(the landing page without `/roster` carries no roster data); enumerate teams via
`GET /teams?type=1` (413 KS HS team links, 1 request).

**Midwest Timing PDFs — 2 requests**: `GET /rest/v1/events?select=id,name,date,status,pdf_link&year=eq.2026`
(public anon key shipped in `midwesttiming.com/assets/index-DoqTOaas.js`; request with
`apikey`/`Authorization: Bearer <anon>` headers) and `GET /rest/v1/state_meet_documents?select=doc_type,class,pdf_url`
(15 rows). PDFs are public storage objects (`/storage/v1/object/public/results/...`, ranged GETs
work).

**kshsaa.org PDFs — direct**: `/Public/CrossCountry/PDF/Schedule.pdf`,
`/Public/Track/PDF/StateSchedule.pdf`, `/Public/Football/PDF/FootballClassifications_Current.pdf`.

**Not enumerable**: Atlas/kshsaa.org TF/XC meet results (no endpoint: TF schedule route 404,
standings empty, no participants view); kshsaachamps TF grade (activity-level flag false).

## Stable identifiers

| ID | Source | Example | Notes |
|---|---|---|---|
| AthleticLIVE meet id `i` | ES `*_meet_list` | `77536` | also the blob `ind_res_list` key space for events, not meets (blob `_doc/76773` 404 on `meet_list`/`event_list`) |
| `ani` (in meet docs) | ES `*_meet_list` | `284391` | **Athletic.net MeetID**, present on 560/842 Kansas meets, 21/21 of the 2026 XC season so far |
| `mi` | ES `athlete_list` | `76773` | same space as meet `i` — the join key for athlete rows |
| `ani` (in athlete rows) | ES `athlete_list` | `18958902` | **Athletic.net athlete ID** |
| `t.i`, `t.ani` | ES `athlete_list` | `1764343` / `17956` | AthleticLIVE team id / **Athletic.net TeamID** |
| Kansas tenant keys | ES index names | `blacksquirrel`, `reddirt`, `athleticlive` | 256 tenant `*_meet_list` indices network-wide; 12 serve Kansas |
| KSHSAA `Id` / `Identifier` | kshsaa-api | `458` / `KSS0001` | school join key (classifications `schoolId` = directory `Id`) |
| KSHSAA `Class` / `FBClass` / `USD` / `League` | kshsaa-api | `4A` / `4A` / `435` / `KSL0033` | classification + district + league |
| champs `ActivityYearClassID` / `SchoolID` | kshsaachamps | `8465` / `646` | champion roster key (own id space) |
| MileSplit `TeamID` | ks.milesplit.com | `15615` | 413 KS HS team records |
| DAT team id | directathletics.com | `100887` | per sport+gender |
| MT Supabase `events.id` / doc `class` | midwesttiming.com | UUID / `1a`…`6a`, `relay`, `wheelchair` | PDF path includes a version `?v=<epoch-ms>` |

## Athletic.net leverage

**Kansas AthleticLIVE meets hand over the AN MeetID with zero Athletic.net requests** — the same
lever report 20/12 found for the MN/IA tenants, now measured for Kansas: **560/842 meets carry
`ani`**, and for the in-flight 2026 XC season it is **21/21**. Additionally, the athlete rows carry
the AN athlete ID and AN team ID, so grading/school confirmation does not require a profile fetch:
TF outdoor 2026 = **37,369/42,023 rows (88.9 %)** with `ani`; individual HS meets can be 100 %
(NCKL HS Track Championship 2026-05-14: 403/403; Tonganoxie Invite 2026-09-19: 147/147).

Estimated avoidance `[INFERENCE — arithmetic on the measured rows]`:

| Lever | Measured input | Athletic.net work avoided |
|---|---|---|
| Kansas meet→AN join | 560 meets with `ani` (1 ES query) | 560 meet lookups |
| Kansas grade-11 identity, 2025-26 TF outdoor | 4,793 grade-11 rows in 107 meets (one ES sweep; `ani` on 88.9 %) | up to ~4.3 k profile fetches whose only purpose was grade |
| Kansas Class-of-2027 (grade 12, 2026-27 XC so far) | 651 rows in 21 meets; 2,940/5,882 rows carry `ani` | ~650 profile fetches |
| Kansas 2025-26 XC grade-11 | 1,637 rows in 45 meets | ~1.6 k profile fetches |
| State TF grade (timer PDFs) | 2026 performance lists: 546 (1A) / 544 (6A) graded entry rows per class document; relay heatsheet 4,336 graded legs | state-meet grade confirmation for ~4.3 k relay legs + the individual entries of every performance-list class ([INFERENCE: 546/544 measured for 1A/6A; ×6 classes ≈ 3.3 k entries]), 8 PDF requests |
| MileSplit KS Class-of-2027 | 147 C2027 athletes from 2 roster requests (86+61) | per-school grade verification (413 schools statewide = 413 requests) |
| KSHSAA classification/ADs | 348 classified schools + 339 AD records, 2 requests | school identity + AD outreach with no Athletic.net call |

None of the Kansas sources expose an Athletic.net *URL*; the join is by ID (`ani`) from
AthleticLIVE, by name+school elsewhere. `grep -i athletic.net` over the new captures (KSHSAA
bundle, champs payloads, MT PDFs, MileSplit/DAT pages) = **0** (KSHSAA bundle: 0 occurrences in
2,914,688 bytes).

## Athlete evidence

| Field | AthleticLIVE `athlete_list` | MT PDFs | MileSplit roster | DAT roster | champs champion roster |
|---|---|---|---|---|---|
| Name | ✅ `n`/`fn`/`l` | ✅ "Last, First" | ✅ roster row | ✅ | ✅ |
| Grade / class | ✅ `y` (9-12 or FR/SR, 100 % of 2026 XC rows) | ✅ `Year` (state) / `YR` (compiled) | ✅ `column-grad-year` (2027…) | ✅ `Year` (FR/SO/JR/SR) | ✅ `Grade` 9-12 |
| School | ✅ `t.n` (+ AN team id) | ✅ team column | ✅ page scope | ✅ team page | ✅ `SchoolID` |
| City/state | ✅ `t.cco` (country only) | ✅ meet line | ✅ team list "CITY, ST" | – | via `/Shared/GetSchools` |
| Gender | ✅ `g` (Male/Female) | ✅ per race/event | ✅ `column-gender` | ✅ per team | ✅ per activity |
| TF/XC, indoor/outdoor | ✅ meet `o` | ✅ meet header | ✅ sport flags | ✅ path sport | ✅ activity id |
| Performances/PRs | ✅ marks (per-event rows) | ✅ | partial | ✅ PR table | mark + place |
| Progression | via meet history (`mi` per year) | per-season PDFs | – | ✅ | year-over-year results |
| Athlete profile URL | – (ids only: `ani`) | – | ✅ `/athletes/<id>-<slug>` | ✅ `/athletes/<sport>/<id>.html` | – |
| Class-of-2027 use | grade column (11 in 2025-26, 12 in 2026-27) | same | `Class = 2027` directly | `Year = JR` | `Grade = 11` in 2025-26 |

Caveat, measured: AthleticLIVE `y` uses **numeric grades for HS meets and `FR/SO/JR/SR` for many
college/indoor meets** (Kansas indoor rows: FR 15,358 / SO 11,030 / JR 5,642 / SR 3,776 vs numeric
`13`–`18` tokens 1,668) — token values are class years only within the right context, so join on
meet identity before interpreting them.

## Recruiting information

- **Unchanged from report 23 and re-confirmed**: the KSHSAA directory has **no coach field** and no
  coach-name search; it provides AD name + school-domain email for essentially every member
  (`dberns@abileneschools.org` for Abilene, 12/12 fresh schools). Privacy contract: `ADCell`,
  `PresCell`, `PrincipalCell` are personal numbers and were not read into any output.
- New sources add **no** coach data: AthleticLIVE `athlete_list`/`meet_list` rows carry no coach or
  contact fields; MileSplit rosters have 0 coach occurrences (report 27); DAT team pages publish
  roster, not staff; MT PDFs are results documents. Bound school pages have a Staff tab but the
  content is JS-loaded and no public JSON endpoint is visible in the page source.
- Sport-level coach names for Kansas remain limited to kshsaachamps champion metadata (6/6 TF-2026
  boys champions, 6/6 XC-2025 state champions with `HC:`/`AC:` role strings), i.e. ~12–18
  coach contacts per activity-year.
- Net: Kansas is an **AD-directory state**; coach-level outreach needs the school-site/Bound layer
  (report 29 tier-3) or the champion-metadata trickle.

## Result evidence

| Field | AthleticLIVE `athlete_list` (ES) | MT compiled/state PDFs | MileSplit `/raw` (report 27) | DAT |
|---|---|---|---|---|
| ResultID | row `i` (AthleticLIVE result id) | – (PDF) | site-internal | event page ids |
| AthleteID | ✅ `ani` (AN) + row `i` | comp# (state meet) | ✅ MileSplit AthleteID | ✅ |
| MeetID | ✅ `mi` (+ `ani` in meet docs) | PDF header | ✅ | ✅ |
| EventID | – at athlete_list level (marks per event live in blob `ind_res_list`/`ind_heat_list`) | event/race title | ✅ | ✅ |
| mark / place | ✅ | ✅ | ✅ | ✅ |
| wind / heat | not in `athlete_list` (blob event docs carry `w`/`hn`) | – | heat `H#`, no wind | prelim/final |
| grade | ✅ `y` | ✅ `YR`/`Year` | roster only | roster only |
| relay membership | `rel_res_list` blob docs (`rm[]` legs, report 12) | ✅ relay heatsheet lists every leg with grade | partial | relay events |
| date / school | ✅ meet `sd`, team `t.n` | ✅ | ✅ | ✅ |

Kansas relay grade, measured: MT `heatsheet-relay.pdf` contains every leg as
`1) #2514 Esparza, Stephany 12 …` — **4,336 leg grade tokens** across 36 relay events, grade-11 = 1,192 (strict-regex count over `#<comp>`-delimited legs; line-wrap boundaries allow ±~0.5 %).
This closes the report-23 relay-leg gap for the state meet without touching Athletic.net.

## Incremental use

- **New/changed Kansas meets** (1 ES query): `POST /*_meet_list/_search` filtered on
  `lsa.keyword = "Kansas"` sorted by `sd` desc; diff on `i` and on `ani`/`ed`. For a short window,
  filter by `sd` range instead of the full list.
- **New results per meet** (1 ES query per batch of meets): `terms: {mi: [...]}` on `athlete_list`
  with `_source` limited to `y,g,t,ani`; diff row `i` sets. For per-event detail (wind/heat), the
  blob `ind_res_list/_doc/<EventID>` supports `If-None-Match` → 304.
- **State-meet grade refresh** (8 requests/year): re-read `state_meet_documents` (15 rows) and fetch
  the performance + relay-heatsheet PDFs when the `?v=` version changes.
- **Regular-season PDFs** (1 request/week): `events?year=eq.<y>&status=eq.posted` diffed on
  `pdf_link`; 279/336 in 2026.
- **MileSplit rosters**: refresh once per season boundary (grad years roll annually); 413 requests
  for the state, or 1 request per school on demand.
- **kshsaa.org school drift** (2 requests): `PublicClassifications` diff by `schoolId` + `moved`,
  and `directory/search/name/HS/` diffed on `DateModified`.
- No design here requires a full historical re-fetch, and **no Athletic.net request is needed**.

## Access characteristics

- **search.athletic.live — public structured JSON (Elasticsearch, undocumented)**. Anonymous POSTs
  with no Origin/Authorization; 200s throughout; browser UA not required but used. Two 400s were
  query-authoring errors (aggregating on a `text` field without `.keyword`), not access blocks.
- **athleticlive.blob.core.windows.net — public structured JSON**; per-document GETs, container
  listing disabled (report 12/08), ETag/Last-Modified + 304.
- **wkyzbfnhunrtyqjewpna.supabase.co — public structured JSON + public storage**, anon key shipped
  in the timer's own client bundle (no account, no secret of a user); `Range` requests work
  (`206`), 279 compiled PDFs + 15 state-meet docs readable.
- **www.kshsaachamps.org — normal HTML + public JSON endpoints** (ASP.NET MVC); large payloads
  (5.9 MB for 27 seasons), `RetrieveRoster` requires both keys else 500.
- **kshsaa-api.kshsaa.org — public structured JSON, undocumented**; no auth; 18 requests, no 429.
- **www.kshsaa.org — SPA shell + static PDFs**; deep links 404 (`/general/PublicClassifications`),
  which is why the API is the observable layer.
- **ks.milesplit.com — normal HTML, server-rendered roster/entries** (team landing page is
  JS-driven; `/roster` is not); robots-allowed surfaces only (report 27).
- **midwesttiming.com — React SPA over public Supabase**; PDFs are static files.
- **www.directathletics.com — normal HTML Rails app**; **www.gobound.com — browser application**
  (roster JS).
- **milesplit.live — browser application (Angular)**; bundle points at
  `www.milesplit.com/api/v1/*` (robots-disallowed family), so no grade API was consumed.
- Observed rate-limit behaviour across all hosts: **no 429, no `Retry-After`**; 77 requests
  sequential per host with ≥1.2 s spacing.

## Recommendation

Amend report 23's Kansas recommendation as follows (Kansas is no longer a grade-poor state):

- **AthleticLIVE Kansas tenants → RESULT-SOURCE (PRIMARY for grade-bearing results).** 842 meets,
  354,733 athlete rows, grade + AN athlete/team IDs, one ES query per meet batch, no Cloudflare.
  Tenants to watch: `blacksquirrel` (KS XC + college), `reddirt` (KS HS TF/XC), `athleticlive`
  (state TF 2017/2018/2019/2021).
- **Midwest Timing PDFs → VALIDATION + RESULT-SOURCE.** The state TF `Year` column and the
  compiled-results `YR` column are the only association-adjacent TF grade evidence for Kansas;
  8 requests cover the state meet, 1 request/week covers new regular-season PDFs.
- **MileSplit KS rosters → PRIMARY Class-of-2027 roster enumeration** (per report 27) — measured
  again here: free, 1 request/school, 413 KS team records, 147 C2027 from 2 samples.
- **KSHSAA (classifications + directory) → DISCOVERY-ONLY + COACH-DIRECTORY (AD only)**; unchanged,
  now with the classification table and the 44.8 % stale-record caveat.
- **kshsaachamps → narrow VALIDATION**: XC grade (143 graded rows/season) and champion rosters
  (~223 graded athletes/activity-year for TF 2026); no TF grade will ever come from it.
- **kshsaa.org meet results, Bound KS rosters → REJECT** (no data / not accessible as structured
  output).

## Evidence appendix

All requests 2026-09-20 America/Chicago (CDT, −05:00), `curl` with desktop-Chrome UA unless noted;
ES = `search.athletic.live` POST with JSON body.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://search.athletic.live/midwesttiming_meet_list/_search` (agg on `lsa`) | POST | **400** | Text field cannot be aggregated (query error) → `.keyword` needed | 09:05:43 |
| `https://search.athletic.live/midwesttiming_meet_list/_search` (agg `lsa.keyword`) | POST | 200 | Tenant `midwesttiming` = 256 meets, Indiana 206/Ohio 14/… , **0 Kansas** — the Kansas Midwest Timing is not this tenant | 09:05:51 |
| `https://search.athletic.live/*_meet_list/_search` (`term lsa.keyword=Kansas`) | POST | 200 | **1,694 docs → 842 distinct Kansas meets across 13 indices** | 09:05:57 |
| `https://search.athletic.live/blacksquirrel_meet_list/_search` (Kansas, newest first) | POST | 200 | KS 2026 XC meets with `ani` AN MeetIDs (Tonganoxie 284391, Hiawatha 284384, Joe Schrag 281941 …); cross-check of the 21 XC-2026 meet docs: **21/21 carry `ani`** | 09:06:06 |
| `https://search.athletic.live/field_app_round_list/_search` | POST | 200 | Index exists but carries only `rounds` | 09:06:06 |
| `https://athleticlive.blob.core.windows.net/$web/session_list/_doc/76773` | GET | 200 | Session doc keyed by session id (74 B), not a usable meet index | 09:06:12 |
| `https://athleticlive.blob.core.windows.net/$web/division_list/_doc/76773` | GET | 200 | Division doc (57 B) | 09:06:12 |
| `https://athleticlive.blob.core.windows.net/$web/meet_list/_doc/76773` | GET | 404 | No blob `meet_list` doc — meet data comes from ES, not blob | 09:06:12 |
| `https://athleticlive.blob.core.windows.net/$web/event_list/_doc/76773` | GET | 404 | Event ids are not addressable by meet id | 09:06:12 |
| `https://search.athletic.live/athlete_list/_search` (`term mi=76773`) | POST | 200 | 590 rows; schema `n/fn/l/y=12/g/Male/mi/ani=18958902/t{ani=17956,n:"Topeka-West"}` — grade + AN athlete + AN team ids in one place | 09:06:18 |
| `https://search.athletic.live/*_meet_list/_search` (`size:2000`, Kansas) | POST | 200 | Full Kansas meet export: 1,694 hits, 842 unique; per-tenant and per-sport splits | 09:06:31 |
| `https://www.kshsaachamps.org/Scripts/kshsaa/ResultsQuery.js` | GET | 200 (32,945 B) | Endpoint inventory; grade column rendered only if `IndividualResultsSectionFieldApplicable_Grade` | 09:05:43 |
| `https://www.kshsaachamps.org/Results/RetrieveResultsByActivity?startYear=2026&endYear=2026&activityID=51` | GET | 200 (235,224 B) | TF boys 2026: 864 rows all `Grade:null`; flag `false`; 6 champions w/ RosterCount+Coach | 09:06:31 |
| `https://search.athletic.live/athlete_list/_search` (KS meet grade agg, `y`) | POST | **400** | `y` is text; keyword subfield required | 09:06:42 |
| `https://search.athletic.live/athlete_list/_search` (KS meet grade agg, `y.keyword`) | POST | 200 | **354,733 rows** across 842 meets; XC 61,020 / outdoor 234,047 / indoor 59,666; outdoor g11 23,099 | 09:06:51 |
| `https://www.kshsaachamps.org/Results/RetrieveResultsByActivity?startYear=2025&endYear=2025&activityID=57` | GET | 200 (63,930 B) | XC control: flag `true`, 143/143 graded, 50 grade-11; state 120 rows + regional 23 | 09:07:22 |
| `https://www.kshsaachamps.org/Results/RetrieveResultsByActivityYearClassID?activityYearID=2346` | GET | 200 (235,224 B) | "Per-class" endpoint returns the byte-identical ungraded payload | 09:07:24 |
| `https://www.kshsaachamps.org/Results/RetrieveResultsByCriteria?startYear=2026&endYear=2026&fullName=&activityID=51&schoolID=646` | GET | 200 (3,815 B) | Per-school placement HTML incl. coach name, no grade, no participants view | 09:07:26 |
| `https://www.kshsaachamps.org/Results/RetrieveResultsByActivity?startYear=2026&endYear=2026&activityID=68` | GET | 200 (1,508 B) | Indoor TF girls: config `…Grade=false`, 0 rows | 09:07:27 |
| `https://www.kshsaachamps.org/Scripts/kshsaa/Shared.js` | GET | 200 (21,352 B) | Endpoint inventory (`RetrieveRoster`, `RetrieveMedia`, `RetrieveUserFilter`) | 09:07:28 |
| `https://www.kshsaachamps.org/Scripts/kshsaa/ChampHistory.js` | GET | 200 (4,086 B) | Champion-history + popup endpoints only | 09:07:30 |
| `https://www.kshsaachamps.org/Shared/RetrieveRoster?activityYearClassID=8465&schoolID=660` | GET | 200 `[]` | TF 6A **2nd place has no roster** → champion-only | 09:08:01 |
| `https://www.kshsaachamps.org/Shared/RetrieveRoster?activityYearClassID=8283&schoolID=1` | GET | 200 `[]` | XC 6A 2nd place has no roster | 09:08:02 |
| `https://www.kshsaachamps.org/Shared/RetrieveRoster?activityYearClassID=8465&schoolID=646` | GET | 200 (4,969 B) | TF 6A champion roster: 18 rows, grades 10:5/11:8/12:5 | 09:08:04 |
| `https://kshsaa-api.kshsaa.org/PublicClassifications` | GET | 200 (31,885 B) | **348 schools; 6A 36 / 5A 36 / 4A 36 / 3A 64 / 2A 64 / 1A 112**, enrollment ranges, `moved` 330/9/9 | 09:05:43 |
| `https://kshsaa-api.kshsaa.org/PublicClassifications?year=2027` | GET | 200 (31,885 B) | `year` param ignored — still 2025-2026 | 09:08:04 |
| `https://kshsaa-api.kshsaa.org/PublicClassifications?year=2025` | GET | 200 (31,885 B) | Same payload | 09:08:05 |
| `https://kshsaa-api.kshsaa.org/directory/search/name/HS/` | GET | 200 (311,856 B) | 339 HS records; **339/339 AD name+email**, 329 WebSite; 39-field keyset has **no coach field** | 09:08:02 |
| `https://kshsaa-api.kshsaa.org/directory/Id/{458,464,465,467,469,470,480,537,559,800,1185,1229}` | GET ×12 | 200 ×12 | **12/12 fresh schools** with AD name+email+website+phone+principal; one identical keyset; `DateModified` 2019–2026 | 09:08:15–09:08:33 |
| `https://www.kshsaa.org/react/assets/index-f33aafe3.js` | GET | 200 (2,914,688 B) | Official routes/PDF paths; classification page route; directory search builds `directory/search/{name}/{city}`; **0** `athletic.net` references | 09:08:14 |
| `https://www.kshsaa.org/Public/CrossCountry/PDF/Schedule.pdf` | GET | 200 (84,879 B) | Official 2026 XC state schedule: 3A/5A/6A Rim Rock; 1A/2A/4A Wamego; per-class times | 09:08:55 |
| `https://www.kshsaa.org/Public/Track/PDF/StateSchedule.pdf` | GET | 200 (184,927 B) | Official 2026 TF state schedule: all 6 classes, wheelchair events, WSU | 09:08:57 |
| `https://search.athletic.live/athlete_list/_search` (top meets by g11) | POST | 200 | Top Kansas meets by grade-11 rows are the **state TF championships 2017–2021** (1,079 in 2021) | 09:08:55 |
| `https://ks.milesplit.com/meets/751194-kshsaa-state-championships-2026/entries` | GET | 200 (1,956,653 B) | 2,372 distinct athlete URLs / 325 team URLs for the 2026 state meet; still no grade, no AthleteID | 09:09:29 |
| `https://ks.milesplit.com/teams/15615-olathe-north-high-school` | GET | 200 (44,815 B) | Team landing page carries no roster data (JS widgets) | 09:09:43 |
| `https://ks.milesplit.com/teams/14601-blue-valley-northwest-high-schoo` | GET | **301** | Slug moved; the roster path must be used with the canonical slug | 09:09:44 |
| `https://search.athletic.live/athlete_list/_search` (per-meet stats, 10 meets) | POST | 200 | Per-meet grade/ani detail: 2021 state TF (i 9777) 3,699 rows g11 1,079; NCKL 2026 TF (i 73566) 403 rows **403 with ani**, g11 91; Joe Schrag (76773) 590/566; Hiawatha (77528) 450/434; Tonganoxie (77536) 147/**147** | 09:09:17 |
| `https://ks.milesplit.com/teams/15615-olathe-north-high-school/roster` | GET | 200 (522,969 B) | **235 athletes with `column-grad-year`: 2027:86 / 2028:76 / 2029:58 / 2030:15**; gender 123f/112m | 09:09:57 |
| `https://ks.milesplit.com/teams/10769-st-thomas-aquinas-high-school/roster` | GET | 200 (345,222 B) | 150 athletes: 2027:61 / 2028:47 / 2029:29 / 2030:11 / unset:2 | 09:10:20 |
| `https://ks.milesplit.com/teams?type=1` | GET | 200 (155,678 B) | **413 Kansas HS team records** (matches report 27's matrix) | 09:10:34 |
| `https://milesplit.live/meets/751194` | GET | 200 (36,991 B) | KSHSAA's linked live-results page is an Angular "MileSplit Live Results" SPA (not AthleticLIVE) | 09:09:17 |
| `https://milesplit.live/main.701a67b97a6fcdf5.js` | GET | 200 (2,373,449 B) | SPA calls `www.milesplit.com/api/v1/*` (robots-disallowed family); no anonymous grade API found | 09:10:56 |
| `https://www.directathletics.com/leagues/track/1252.html` | GET | 200 (47,140 B) | League page titled "**KSHSAA 1A**" (`title_text` span) enumerating 249 per-team roster links | 09:09:41 |
| `https://www.directathletics.com/teams/track/100887.html` | GET | 200 (19,770 B) | Fresh DAT roster (Wichita-Central Christian, Men's T&F, league KSHSAA 1A): 17 row tokens — JR 7 / SR 6 / SO 3 / one `13`; page also lists 10 past meets 2023–2026 | 09:10:20 |
| `https://search.athletic.live/athlete_list/_search` (`term mi=73566`, size 2) | POST | 200 | Raw row sample: `y:"12"`, `g:"Female"`, `ani:30987335`, `t:{n:"Abilene", ani:17624}` | 09:10:34 |
| `https://search.athletic.live/athlete_list/_search` (21 XC-2026 meets) | POST | 200 | **5,882 rows, 5,882 graded**; g12 651 (C2027), g11 679; AN athlete ids 2,940; 393 teams | 09:10:36 |
| `https://kshsaa-api.kshsaa.org/schedules/trackFieldSchedule/teamschedule/735/1` | GET | **404** | KSHSAA publishes no TF team schedule/meet-results route either | 09:10:20 |
| `https://kshsaa-api.kshsaa.org/directory/search/name/Huseman/` | GET | 200 `[]` | Coach surnames are not searchable in the directory | 09:10:22 |
| `https://www.kshsaa.org/general/PublicClassifications` | GET | **404** | Official SPA deep link is not server-served (client-side routing) → API is the observable layer | 09:10:20 |
| `https://www.kshsaa.org/Public/Football/PDF/FootballClassifications_Current.pdf` | GET | 200 (335,499 B) | Football 2026 & 2027: 6A 32 / 5A 32 / 4A 32 / 3A 40 / 2A 40 / 1A 37 + 8-P D-I 40, D-II 50 (9-11 enrollment) | 09:10:21 |
| `https://www.gobound.com/ks/schools/shawneemissioneasths` | GET (-L) | 200 (449,707 B) | Bound school page: roster rows not in source; no public JSON endpoint visible | 09:10:45 |
| `https://midwesttiming.com/assets/index-DoqTOaas.js` | GET | 200 (1,448,832 B) | Supabase project URL + client-shipped anon key (public read path) | 09:11:06 |
| `https://wkyzbfnhunrtyqjewpna.supabase.co/rest/v1/state_meet_documents?select=doc_type,class,pdf_url&limit=50` | GET (anon) | 200 (2,629 B) | **15 state-meet docs**: performance + heatsheet for 1a–6a, wheelchair, heatsheet-relay, each a public PDF URL | 09:11:11 |
| `https://wkyzbfnhunrtyqjewpna.supabase.co/rest/v1/events?select=id,name,date,status,pdf_link&year=eq.2026&pdf_link=not.is.null&order=date.desc&limit=3` | GET (anon) | 200 (824 B) | Regular-season 2026 compiled-result PDFs (Erie XC 2026-09-05, Anderson County 2026-09-03 …) | 09:11:13 |
| `https://…supabase.co/storage/v1/object/public/results/state-meet/performance-1a.pdf` | GET | 200 (415,008 B) | **Hy-Tek performance list with a `Year` column**: 546 graded entry rows over 30 individual events (36 events incl. 6 relays), 9:74/10:114/**11:164**/12:194 | 09:10:56 |
| `https://…supabase.co/storage/v1/object/public/results/state-meet/performance-6a.pdf` | GET | 200 (418,609 B) | 544 graded entry rows, 9:52/10:89/**11:184**/12:219, same 36-event structure | 09:11:17 |
| `https://…supabase.co/storage/v1/object/public/results/2026/1788625549432-2026-09-05%20Erie%20Invitational%20XC%20Results.pdf` | GET | 200 (958,018 B) | Compiled results carry `Athlete / YR / # / Team / Score / Time` (FR/SO/JR/SR for varsity+JV, numeric 5–8 for MS) | 09:11:19 |
| `https://wkyzbfnhunrtyqjewpna.supabase.co/rest/v1/events?select=id&year=eq.2026` (`count=exact`) | GET (anon) | 206 | `content-range 0-0/336` — 336 Kansas events in 2026 | 09:11:51 |
| `…/events?select=id&year=eq.2026&pdf_link=not.is.null` (`count=exact`) | GET (anon) | 206 | `content-range 0-0/279` — 279 with compiled PDFs | 09:11:53 |
| `https://…supabase.co/storage/v1/object/public/results/state-meet/heatsheet-relay.pdf` | GET | 200 (578,182 B) | **Relay legs with grade**: 4,336 leg grade tokens, grade-11 1,192, 36 relay events | 09:11:55 |
| `https://search.athletic.live/athlete_list/_search` (season groups 2025-26/2026-27) | POST | 200 | TF outdoor 2026: 42,023 rows, g11 4,793, g12 4,063, AN ids 37,369; XC 2025: 11,132 rows, g11 1,637; XC 2026: g12 651 | 09:12:29 |
| `https://www.kshsaachamps.org/Results/RetrieveResultsByActivity?startYear=2026&endYear=2026&activityID=50` | GET | 200 (237,810 B) | TF girls 2026: 873 rows, **0 graded**; champions RosterCount 18/23/30/20/11/7 (=109) | 09:12:16 |
| `https://www.kshsaachamps.org/Shared/RetrieveRoster?activityYearClassID=8459&schoolID=646` | GET | 200 (5,023 B) | Girls 6A champion roster: 18 rows, grades 9:2/10:5/11:3/12:8 | 09:12:17 |
| `https://www.kshsaachamps.org/Shared/RetrieveRoster?activityYearClassID=<1A-2026>&schoolID=485` | GET | 200 (1,901 B) | Girls 1A champion roster: 7 rows, grades 9:2/10:2/12:3 | 09:12:19 |
| `https://www.kshsaachamps.org/Results` | GET | 200 (17,143 B) | Results landing shell: nav only, no participants/grade surface | 09:12:50 |
| `https://www.kshsaachamps.org/Results/RetrieveResultsByActivity?startYear=2015&endYear=2015&activityID=51` | GET | 200 (278,229 B) | TF boys 2015: 850 rows, 0 graded → the TF grade gap is activity-wide, not 2026-specific | 09:12:52 |
| `https://www.kshsaachamps.org/Results/RetrieveResultsByActivity?startYear=2026&endYear=2026&activityID=51` (row/grade audit) | local parse | n/a | 61 team rows, 6 with coach, 6 with `RosterExists=true` (first place only) | 09:12 |
| `grep -i 'athletic\.net'` over KSHSAA bundle + new captures | local | n/a | **0** Athletic.net URLs; the Kansas↔AN join is by `ani` id from AthleticLIVE, not by link | 09:13 |

Request totals by host (sequential, ≥1.2 s spacing, no 429/`Retry-After` anywhere):
`search.athletic.live` 14 · `athleticlive.blob.core.windows.net` 4 · `www.kshsaachamps.org` 16 ·
`kshsaa-api.kshsaa.org` 18 · `www.kshsaa.org` 5 · `ks.milesplit.com` 6 · `milesplit.live` 2 ·
`midwesttiming.com` 1 · `wkyzbfnhunrtyqjewpna.supabase.co` 8 · `www.directathletics.com` 2 ·
`www.gobound.com` 1 — **77 total**.

**Post-hoc audit pass (2026-09-20 09:14–09:20, local only — zero new requests).** Every number in
this report was re-derived from the raw captures with fresh `jq`/Python parses before delivery;
that pass corrected three items against the first draft: `ani` coverage 563→**560**/842 meets, the
performance-list row counts (a 288-row partial parse → **546/544** measured rows over the full
`pdftotext` output), and the relay leg count (→ **4,336**). Derived summaries live in
`research/midwest/evidence/gaps/41/derived-counts-summary.json`
plus the reconstructed counts in the appendix rows above; file sizes were checked against the
stored captures byte-for-byte.
