# 38. Illinois follow-ups (IHSA girls XC tournament ids, coach email coverage, DirectAthletics recency)

Status: complete — all three report 13/14 open items closed with live evidence, 2026-09-20
Observed on: 2026-09-20 (America/Chicago, 09:06–09:12)

Scope: this file closes only the three items named in the assignment. Everything else about IHSA
stays in `13-illinois-ihsa.md`; everything else about DirectAthletics stays in
`14-illinois-directathletics.md`. All probes were live and sequential (≥1 s between requests to one
host). **No request was sent to any `*.athletic.net` host** — the Athletic.net URLs quoted below are
read out of IHSA's own JavaScript text, never fetched. One headless-Chromium page load of the IHSA
girls XC page was used to observe its own XHRs.

---

### Source

| Source | Hosts | What this report used it for |
|---|---|---|
| IHSA (Illinois High School Association) | `www.ihsa.org`, `api.ihsa.org` | (1) girls XC state-series tournament ids; (2) coach/AD directory + email reveal |
| DirectAthletics / TFRRS | `www.directathletics.com`, `www.tfrrs.org` | (3) IL "Top Times" indoor performance-list recency |

Both are the sources already characterised in reports 13/14 (IHSA: undocumented public JSON, no auth;
DA: server-rendered Rails HTML). No new source was probed.

### Coverage

* **Item 1 — girls XC ids.** Report 13 fetched only boys ids 688–690 and left "girls use 691/692/693
  (not fetched; [UNVERIFIED])". Now verified three ways: the live site chunk's id map, the live page's
  own XHRs, and a school-id join against the 2026-27 XC classification data. Girls 2025-26 qualifier
  rosters are now measured: **1,177 athletes, 318 of them grade 11**.
* **Item 2 — coach coverage.** Fresh 24-school sample (≥20 required): every 34th `SchoolID` ascending
  from index 17 (`/v1/schools` sorted; offset chosen so the sample differs from report 13's "every
  60th" set). Denominator-stated **name-present** and **email-present** rates for TRB/TRG/CCB/CCG head
  coaches + both AD roles.
* **Item 3 — DirectAthletics recency.** The IL HS indoor/"Top Times" surface is **frozen at the
  2023-24 season**: the 2024 trio is the newest IL list the product publishes, and no 2025/2026 IL
  list exists (the only IL HS meet found after that season remains report 14's 03/07/2025 SIU HS
  Invite; today's IL results-index page 1 is collegiate-only). Exact URLs recorded below.
* Nothing in this report touches regular-season IHSA meets/results, indoor IHSA championships (IHSA
  has none), or any other state.

### Enumeration

**1. IHSA girls XC tournament ids — exact recipe.**

* Id set (current site chunk `StateSeries-w2fH3odu.js`, imported by the live `main-B2rKN2jy.js`):
  `{"boys-cross-country":{"1A":"688","2A":"689","3A":"690"},"girls-cross-country":{"1A":"691","2A":"692","3A":"693"}}`.
* URL pattern (identical for both genders, term-parameterised):
  `GET https://api.ihsa.org/v1/{term}/statefinal/cc-qualifiers?tournamentId={id}` — the site's own
  call is `fetch(`${pe(n)}/statefinal/cc-qualifiers?tournamentId=${x[sport][class]}`)`, where `pe()`
  builds `https://api.ihsa.org/v1/{term}`.
* Term behaviour, all probed today:
  | term | `tournamentId=691` | Meaning |
  |---|---|---|
  | `2025-26` | 200, 79,632 B roster | last completed XC season (rosters posted) |
  | `2024-25` | 200, `{"tournamentId":"691","boxAssignments":[],"teamQualifiers":[],"individualQualifiers":[]}` | id valid for older terms, roster not archived |
  | `2026-27` | 404 `{"error":"Archive not available for term 2026-27"}` | current-season roster not published yet (2026-27 girls XC state final = Sat 2026-11-07) |
* There is **no enumerable XC tournament index**: `…/statefinal/cc-qualifiers` without
  `tournamentId` → 400 `{"error":"Missing tournamentId"}`, and
  `/v1/sports/girls-cross-country/tournament-info` → 200 with `classes:"1A-3A"`, dates/site only, no
  ids. The chunk constant remains the only id source (same situation as report 13 found for boys).
* Live-page proof: the girls XC state-central page
  (`https://www.ihsa.org/sports/girls-cross-country/state-series/xc-state-central`) requested
  `…?tournamentId=691` on load and `…?tournamentId=692` / `=693` when its 2A/3A class tabs were
  clicked — captured from the page's own `request` events (evidence file
  `evidence/gaps/38/ihsa-girls-xc-xhr-capture.json`). Zero `athletic.net` requests during that load.
* Class-mapping cross-check (independent of the bundle): the `ihsaSchoolId` of each girls
  `teamQualifiers` row joined to the 2026-27 fall classification for `CCG` gives **691 → 28× 1A**
  (30 team qualifiers: 1 school not in the fall classification, 1 classified 2A), **692 → 28× 2A
  (28/28)**, **693 → 28× 3A (28/28)**; the boys control (688, `CCB`) gives 29× 1A of 30. The two 1A
  outliers are `0535` Elgin (Harvest Christian Academy) — no CCG row in 2026-27 — and `0524` Elmhurst
  (Timothy Christian) — 2A in 2026-27, i.e. a season-to-season class move, so the join is
  `2025-26 roster × 2026-27 class` and ±1 mismatch is expected.
* Roster counts (2025-26, `boxAssignments` = full athlete list, `teamQualifiers[].athletes` +
  `individualQualifiers[].athletes` reconciles exactly to it):

  | id | class | athletes | team qualifiers | individual qualifiers | grade-11 |
  |---|---|---|---|---|---|
  | 691 | girls 1A | 357 | 30 | 41 | 97 |
  | 692 | girls 2A | 397 | 28 | 31 | 96 |
  | 693 | girls 3A | 423 | 28 | 35 | 125 |
  | **girls total** | | **1,177** | 86 | 107 | **318** |
  | 688 (control) | boys 1A | 398 | 30 | 42 | 104 |

  (Report 13's boys totals: 1,214 athletes / 358 grade-11 across 688–690.)

**2. Coach sample — exact recipe.** `GET https://api.ihsa.org/v1/schools` (1 request, 828 rows) →
sort `SchoolID` ascending → take indices 17, 51, 85, … (every 34th) → 24 schools →
`GET /v1/schools/{SchoolID}/staff2` each (24 requests, all 200) → for the first T&F/XC head-coach row
per school, `GET /v1/schools/{SchoolID}/staff/{PersonID}/email` (12 reveals, capped at 12 for the
anti-spam gate) → sponsorship/entry join from
`/v2/classification/classifications?season={S,F}` (2 requests). Sampled schools:
`0120 0219 0302 0337 0415 0523 0705 0809 1103 1232 1329 1366 1504 1618 1803 1903 1941 2205 2330 2720
2759 2817 2924 2966`. Per-school role rows: 678 staff rows total (mean 28.3/school); raw JSON in
`evidence/gaps/38/coach-survey.json`.

**3. DirectAthletics — exact URLs (re-verified live 2026-09-20).**

| Purpose | URL | Observed |
|---|---|---|
| Performance-list index (full product enumeration) | `https://www.directathletics.com/rankings.html` | 5,584 list links; **exactly 30 IL Top Times links**, highest ids `4524/4525/4526`; max global list id 5775 (2026 lists) |
| IL Top Times 1A (newest) | `https://www.directathletics.com/lists/track/1467_4524.html` | title `Illinois Top Times (1A)`; 1,471 data rows (1,176 athlete rows + 295 relay rows — exactly report 14's default-view count), **all 2024** (Feb 1 – Mar 9) |
| IL Top Times 2A (newest) | `https://www.directathletics.com/lists/track/1468_4525.html` | title `Illinois Top Times (2A)`; 1,500 data rows (1,200 + 300 relay), all 2024 |
| IL Top Times 3A (newest) | `https://www.directathletics.com/lists/track/1469_4526.html` | title `Illinois Top Times (3A)`; 1,500 data rows (1,200 + 300 relay), all 2024 |
| League homes (current list pointer) | `/leagues/track/{1467,1468,1469}.html` | each `View Performance List` button → `list_hnd={4524,4525,4526}` — the 2024 lists **are** the leagues' current lists |
| TFRRS mirror | `https://www.tfrrs.org/lists/4524/Illinois_Top_Times_1A` | title `TFRRS \| Illinois Top Times (1A)`; same 2024 rows |
| Newest IL HS indoor championship meet sheets | `/results/track/84225.html` (2A, 2024-03-23), `/results/track/84224.html` (1A, 2024-03-22) | 2024 meet sheets, confirmed via the IL meet-index name filter |
| Meet index (freshness checks) | `/legacy_da/results?filterrific[with_states]=IL[&filterrific[search_query]=…]` | `Top Times` → newest 2024-03-23; `High School` → newest 2024-03-02; unfiltered page 1 → 78×2026 + 22×2025 dates, **all collegiate** |

### Stable identifiers

| Identifier | Value / shape | Evidence |
|---|---|---|
| IHSA XC `tournamentId` | boys 688/689/690, girls 691/692/693 (1A/2A/3A each) | chunk constant + live page XHRs + class join |
| IHSA XC request URL | `https://api.ihsa.org/v1/{term}/statefinal/cc-qualifiers?tournamentId={688..693}` | site chunk fetch call; live page called 691/692/693 |
| IHSA term string | `2025-26` (last posted XC season), `2024-25` (empty archive), `2026-27` (404 not yet posted) | direct probes |
| IHSA school id | `ihsaSchoolId` on every `teamQualifiers`/`individualQualifiers` row (reuses `SchoolID` from `/v1/schools`) | payloads + classification join |
| DA list id | composite `{league_id}_{list_id}`: `1467_4524`, `1468_4525`, `1469_4526` | list URLs + `/rankings.html` rows |
| DA league id | `1467` / `1468` / `1469` (IL Top Times 1A/2A/3A); `1355` CPS; `105` Illinois; `1088/1089` IHSA A/AA | league pages; report 14 |
| DA meet id | `84225` (2A championship, 2024-03-23), `84224` (1A, 2024-03-22) | meet-index name filter + fetched sheet |
| TFRRS list id | `4524` (mirrors DA `1467_4524`; same numeric space as the DA list id) | `/lists/4524/Illinois_Top_Times_1A` |

Names are not identifiers anywhere here; the DA roster `Year` ordinal trap from report 14 stands.

### Athletic.net leverage

* **Girls XC completes the IHSA XC seed.** The chunk's meet-link map is gender-shared per class —
  `268054` (1A), `268064` (2A), `268066` (3A) with `/entries` and `/results` variants for **both**
  `boys-cross-country` and `girls-cross-country` (read from `StateSeries-w2fH3odu.js`; not fetched).
  The 1,177 girls qualifier rows carry name + school + `yearInSchool` + coach names but **no
  Athletic.net athlete/team ids**, exactly like the boys (report 13). So the girls half of the XC
  cohort is a deterministic *seed* (class meet + school + name + grade → one targeted lookup), not an
  id handover: the 318 grade-11 girls are 318 avoidable discovery searches, not 318 instant profile
  matches. [INFERENCE on request-avoidance magnitude; the field set is measured.]
* **DA contributes no new A.net leverage.** Its IL HS indoor surface is frozen at 2024, so there is
  nothing new to convert; the 2024 lists remain the one-time C/O-2027 backfill of report 14 (FR rows
  of 2024 = Class of 2027; 1,580 freshmen at `?limit=1000`). Zeros: no athletic.net or milesplit
  string occurs on any DA page fetched (report 14, re-checked on today's captures).
* **Coach directory:** no Athletic.net surface at all — school-role contacts, not athlete records.

### Athlete evidence

| Field | Girls XC qualifiers (IHSA) | DA 2024 Top Times lists |
|---|---|---|
| Name | `FirstName` + `LastName` (box, team and individual rows) | yes (`Last, First`) |
| Grade / class | `yearInSchool` `"9".."12"` on team+individual athletes; 1 row in 691 lacks it | `Year` column `FR/SO/JR/SR` (per 2024 season) |
| School | `SchoolName` + `ihsaSchoolId` on team/individual rows | team link `/teams/track/<id>.html` |
| City/state | no; join `ihsaSchoolId` → `/v1/schools/{id}` | no city (report 14) |
| Gender/category | implicit by id (bundle map) + class join; **no gender field in the payload** | separate men's/women's lists |
| TF vs XC | XC only (this endpoint) | indoor T&F only |
| Performances | **none** — qualifier rosters have no marks | yes (mark + meet + date) |
| PRs / progression | no | "Best" per event (report 14) |
| Meets | class meet only (implicit) | meet string + date per row |
| Profile URL | no | `/athletes/track/<id>.html` |
| Coach names | yes, per qualifying school (`coaches[].coachName`, `coachType` Head/Assistant) — no emails | no |

The DA surface was re-verified as *unchanged* today at the default view; the athlete counts at
`?limit=1000` (7,333 athletes / 1,580 C/O-2027) come from report 14 and were **not** re-measured here
[not re-verified, no reason to doubt].

### Recruiting information

Fresh sample, 24 schools; `name-present` = a `staff2` row with that `RoleID`; `email-present` = such
a row carrying `HasEmail: true`. Rates are given under three denominators, because most of the
variance is "does this school field the sport at all", not "is the directory incomplete".

| Role | all 24 schools | has a 2026-27 T&F/XC classification row (n=17) | **entered** the sport 2026-27 (`EntryStatus=Y`) |
|---|---|---|---|
| `HCB-TRB` boys T&F | 15/24 = 62.5% name; 15/24 = 62.5% email | 15/17 = 88.2% / 88.2% | **14/14 = 100% / 100%** |
| `HCG-TRG` girls T&F | 15/24 = 62.5% / 14/24 = 58.3% | 15/17 = 88.2% / 82.4% | 13/14 = 92.9% / 12/14 = 85.7% |
| `HCB-CCB` boys XC | 11/24 = 45.8% / 45.8% | 11/17 = 64.7% / 64.7% | 9/10 = 90.0% / 90.0% |
| `HCG-CCG` girls XC | 11/24 = 45.8% / 45.8% | 11/17 = 64.7% / 64.7% | 8/9 = 88.9% / 88.9% |
| `C1-BoysAD` | 23/24 = 95.8% / 95.8% | 16/17 = 94.1% / 94.1% | — |
| `D1-GirlsAD` | 23/24 = 95.8% / 95.8% | 16/17 = 94.1% / 94.1% | — |
| `E1-ActivDir` (not T&F-specific) | 9/24 = 37.5% | 7/17 = 41.2% | — |

Wilson 95% CIs on the entered-school rows: TRB 78–100%, TRG name 69–99% / email 60–96%, CCB 60–98%,
CCG 56–98%, ADs 73–99% (all-24 rows: T&F 43–79%, XC 28–65%, AD 80–99%).

**Email semantics verified both directions.** 12 head-coach reveal calls (`…/staff/{PersonID}/email`)
all returned 200 with a non-empty address; the one sampled row with `HasEmail: false` (school `0809`,
`HCG-TRG`, PersonID 107373) returned 200 with `{"email":""}`. So `HasEmail` is an honest gate and
"email-present" is exactly "role row absent OR address withheld". Domain quality of the 12 revealed
addresses: 10 school/district domains, **2 `gmail.com`** (`0302` Cairo, `2205` Vienna) — school-reported
personal-domain addresses exist in the directory and should be flagged, not silently trusted.

Named gaps (all measured, none extrapolated):

* `1232` Glenbard East entered girls T&F and XC, has `HCB-TRB`/`HCB-CCB`/`HCG-CCG` rows but **no
  `HCG-TRG` row** — the single "entered-but-no-head-coach-row" case for T&F in the sample.
* `0809` Hebron (Alden-H.) has the girls T&F coach row but withheld the email.
* `0523` Elmhurst (IC Catholic) entered TRB/TRG/CCB/CCG but lists only `HCG-TRG` — the worst
  partial-listing case (3 of 4 roles missing).
* `1618` Payson (Seymour) carries TRB/TRG classification rows but entered neither, and has no coach
  rows; `2924` Chicago (Noble/Johnson) carries a TRB row but entered only FB/VBG while still listing
  T&F coach rows — classification rows ≠ entered teams ≠ published coach rows, in both directions.
* The 7 sampled schools with no T&F/XC classification at all (`0705 1103 1366 1803 1903 2720 2966`)
  simply don't sponsor these sports; they are correctly empty, not directory failures.

Retention rule unchanged: head T&F/XC coach + AD roles only, reveal at low volume (this report used
13 reveals total, 1 per school, never a full-staff sweep).

### Result evidence

* **IHSA girls XC: no results.** The qualifier payloads carry box/school/name/grade/coach and the
  team/individual qualifier lists only — identical in kind to the boys payloads in report 13 (no
  ResultID, no marks, no timing). The girls side changes nothing about IHSA's stated limits
  (no regular-season data, no history, no sectionals).
* **DirectAthletics: the newest IL HS indoor result evidence is the March 2024 championship.** The
  IL meet-index name filter returns `Illinois Top Times Indoor Track & Field Championships` for
  2016/2017/2018/2022/2023/**2024**, newest `2024-03-23` (`/results/track/84225.html`, 2A) and
  `2024-03-22` (`84224`, 1A). Nothing 2025/2026 exists. Scoped precisely: the `High School` name
  filter's newest row is the 2024-03-02 Cogdal HS Invite; report 14 separately recorded one 2025 IL
  HS meet—the 03/07/2025 SIU HS Invite, whose name omits "High School"—and today's IL index page 1
  (newest 100 rows) is 2025–2026 collegiate only. Row-level result fields on those sheets are
  unchanged from report 14 (place/name/year/team/time, rounds, no wind, no FAT flag, no row id).

### Incremental use

* **IHSA XC (6 requests/season, both genders):** once the fall season's rosters post,
  `GET /v1/2025-26|2026-27/statefinal/cc-qualifiers?tournamentId={688..693}` — 3 boys + 3 girls —
  covers both genders for one season. Controller states to encode: `404 Archive not available for
  term X` = season not posted yet (retry later, do **not** treat as absent id); `200` with empty
  arrays = term archived but no roster (e.g. 2024-25). No watermark fields → hash the payload and
  diff, as report 13 already prescribes.
* **Coach refresh:** 1 request/school for `staff2` (+1 per head-coach row you actually reveal). At
  the measured entered-school coverage you get ~14 T&F + ~13 girls-T&F + ~9 XC rows per 24 schools
  sampled; a 25-school/night sweep finishes the 828-school universe in ~5 weeks without touching the
  reveal endpoint in bulk.
* **DA (near-zero cost):** IL has nothing to refresh weekly. One `GET /rankings.html` per month is
  enough to detect resumption — the signal is simple: **any IL Top Times list id above 4526** (or any
  TFRRS-shaped `Illinois` list) means the source has restarted; then fetch the three lists with
  `?limit=1000`. This supersedes report 14's "monthly/seasonal check" with a concrete detector.

### Access characteristics

* **Classification:** IHSA = public structured JSON (undocumented `/v1`,`/v2`), no auth, no CAPTCHA,
  no Cloudflare, `robots.txt` `Allow: /` (report 13, unchanged today). DA/TFRRS = normal server-rendered
  HTML, no auth for the pages used. **Neither source is rate-limit-signalled**: no `429`, no `403`, no
  `Retry-After` observed in 65 logged requests today (63× HTTP 200, 1× 404, 1× 400 — the 404 and 400
  are both the *expected* semantics above).
* **Request ledger (politeness):** 65 logged probes — `api.ihsa.org` 48, `www.directathletics.com` 12,
  `www.ihsa.org` 4, `www.tfrrs.org` 1 — plus **1 headless page load** of the IHSA girls XC page whose
  captured XHRs were 8 `api.ihsa.org` calls (incl. one `POST /v1/click-track` the page itself issues);
  the load's static subresources are additional but come from the same site and its CDN cache.
  Sequential throughout, ≥1.2 s between requests to the same host, no retries, no concurrency.
  ≈73 counted requests in total.
* **One soft control to respect:** the `…/staff/{PersonID}/email` reveal is a deliberate anti-spam
  gate (UI: "email addresses are hidden by default"). It is public and unauthenticated, but this report
  capped itself at 1 reveal/school and sampled only head-coach roles.
* **Not observable / not attempted:** any `*.athletic.net` host (hard constraint) — the A.net meet-link
  map is quoted from IHSA's bundle text; IHSA's staff email values are recorded only as
  present/absent + domain, not transcribed, except the behavioural evidence that they are real.

### Recommendation

* **Item 1 → CLOSED. Girls XC ids are 691/692/693 (1A/2A/3A)** with the exact pattern
  `https://api.ihsa.org/v1/{term}/statefinal/cc-qualifiers?tournamentId={id}`. Add them to the
  acquisition plan next to 688–690: same 6-request seasonal pull, +1,177 athletes/season (318 grade
  11) of seed material, no Athletic.net calls. Treat `404 Archive not available` as "not posted yet".
* **Item 2 → CLOSED. IHSA `staff2` remains a viable COACH-DIRECTORY for Illinois, but only with the
  entered-school denominator.** Head-coach coverage is 14/14 (boys T&F), 13/14 (girls T&F), 9/10 (boys
  XC), 8/9 (girls XC) among schools that actually entered that sport in 2026-27, and 16/17 = 94% for
  both AD roles; email-present equals name-present for every role **except girls T&F** (12/14 = 86% vs
  13/14 = 93%; one school withheld the address), and 2/12 revealed addresses are non-district
  (`gmail.com`) — flag them. The all-schools figures (~62% T&F, ~46% XC) are mostly *non-sponsorship*,
  not missing data: 7 of 24 sampled schools don't field T&F/XC at all. Use the classification endpoint
  (`?season=S|F`) to build the denominator, then chase only the measured 0–11% listing gaps from
  school/district sites.
* **Item 3 → CLOSED. DirectAthletics' IL "Top Times"/indoor surface is 2024-only.** Newest lists:
  `/lists/track/1467_4524.html`, `/1468_4525.html`, `/1469_4526.html` (2023-24 indoor season,
  Jan–Mar 2024); newest IL HS indoor championship sheets: `/results/track/84224.html` (1A,
  2024-03-22) and `/results/track/84225.html` (2A, 2024-03-23). No 2025/2026 IL list exists (the last
  IL HS meet on DA remains report 14's 03/07/2025 SIU HS Invite), and the global list-id sequence has
  moved on (2025 window 4867–5350, 2026 up to 5775) while IL HS stops at 4526. Keep DA as
  **historical C/O-2027 backfill only** (report 14's 2024 FR cohort), drop it from any weekly IL
  cadence, and use "any IL Top Times list id > 4526" as the resumption detector.
* **Net effect on the plan:** the IHSA XC row of report 13's coverage table now covers both genders
  (2,391 qualifiers/season, 676 grade-11 [boys 1,214/358 + girls 1,177/318]); IHSA coach coverage is
  quantified per role; DA's IL role is unchanged (one-time backfill). No new Athletic.net request
  savings beyond report 13's state-final cohort and these XC seeds.

### Evidence appendix

All rows were fetched live on 2026-09-20 (America/Chicago) in the order shown (the last row is the browser capture, appended after the HTTP ledger); timestamps are the ledger `ts` values (MM-DDTHH:MM, local). Method is HTTP GET via `urllib` with a full desktop-Chrome UA unless stated. Ledger + raw captures: `research/midwest/evidence/gaps/38/` (`ledger.jsonl`, payload JSON/HTML files).

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| https://www.ihsa.org/assets/StateSeries-w2fH3odu.js | GET | 200 | current IHSA site chunk (imported by main-B2rKN2jy.js): `x={"boys-cross-country":{"1A":"688","2A":"689","3A":"690"},"girls-cross-country":{"1A":"691","2A":"692","3A":"693"}}`; site's fetch call `${pe(n)}/statefinal/cc-qualifiers?tournamentId=${id}`; girls XC-side asset constants use `/data/ccg/` | 09-20T09:06 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=691 | GET | 200 | girls 1A: 357 athletes / 30 teams / 41 individual qualifiers; 97 grade-11 | 09-20T09:06 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=692 | GET | 200 | girls 2A: 397 athletes / 28 teams / 31 individuals; 96 grade-11 | 09-20T09:06 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=693 | GET | 200 | girls 3A: 423 athletes / 28 teams / 35 individuals; 125 grade-11 | 09-20T09:06 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=688 | GET | 200 | boys 1A control: 398 athletes / 30 teams / 42 individual qualifiers, 104 grade-11 (reproduces report 13) | 09-20T09:06 |
| https://api.ihsa.org/v1/2026-27/statefinal/cc-qualifiers?tournamentId=691 | GET | 404 | 404 `{"error":"Archive not available for term 2026-27"}` — current-season roster not posted yet (2026-27 girls XC final = 2026-11-07) | 09-20T09:06 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers | GET | 400 | 400 `{"error":"Missing tournamentId"}` — XC ids are NOT enumerable from the API; they exist only in the site chunk | 09-20T09:06 |
| https://api.ihsa.org/v1/sports/girls-cross-country/tournament-info | GET | 200 | current-term girls XC state-final info: `classes:"1A-3A"`, Saturday November 7 2026, Detweiller Park — no tournament ids in this payload | 09-20T09:06 |
| https://api.ihsa.org/v1/2024-25/statefinal/cc-qualifiers?tournamentId=691 | GET | 200 | 200 with empty arrays — the id exists for older terms but no roster is archived (mirrors boys 688) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools | GET | 200 | 828 member-school rows — sample frame for the fresh coach sample | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0120/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0120/staff/21953/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0219/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0219/staff/48746/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0302/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0302/staff/6889/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0337/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0337/staff/7372/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0415/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0415/staff/21368/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0523/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0523/staff/40351/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0705/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0809/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0809/staff/137473/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1103/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1232/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1232/staff/108737/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1329/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1329/staff/11187/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1366/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1504/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1504/staff/12157/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1618/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1803/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1903/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1941/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1941/staff/34575/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2205/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2205/staff/98284/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2330/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2720/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2759/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2817/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2924/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2966/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v2/classification/classifications?season=S | GET | 200 | 3,121 spring rows — T&F sponsorship/entry join for the sampled schools | 09-20T09:09 |
| https://api.ihsa.org/v2/classification/classifications?season=F | GET | 200 | 3,960 fall rows — XC sponsorship/entry join for the sampled schools; also the girls-XC class-mapping cross-check | 09-20T09:09 |
| https://api.ihsa.org/v1/schools/0809/staff/107373/email | GET | 200 | HasEmail=false probe (0809 girls T&F head coach) → `{"email":""}` — `HasEmail` is an accurate gate | 09-20T09:09 |
| https://www.directathletics.com/rankings.html | GET | 200 | live list index: 5,584 performance-list links; exactly 30 IL Top Times links, highest ids 4524/4525/4526; max global list id 5775 (2026 lists); no TFRRS-shaped Illinois list | 09-20T09:09 |
| https://www.directathletics.com/lists/track/1467_4524.html | GET | 200 | title `Illinois Top Times (1A)`; 1,471 data rows (1,176 athlete + 295 relay, = report 14's default-view count), all 2024 (Feb 1 – Mar 9) | 09-20T09:09 |
| https://www.directathletics.com/lists/track/1468_4525.html | GET | 200 | title `Illinois Top Times (2A)`; 1,500 data rows (1,200 + 300 relay), all 2024 | 09-20T09:09 |
| https://www.directathletics.com/lists/track/1469_4526.html | GET | 200 | title `Illinois Top Times (3A)`; 1,500 data rows (1,200 + 300 relay), all 2024 | 09-20T09:09 |
| https://www.directathletics.com/leagues/track/1467.html | GET | 200 | league home — `View Performance List` → `list_hnd=4524` (the league's current list is the 2024 one) | 09-20T09:09 |
| https://www.directathletics.com/legacy_da/results?filterrific%5Bwith_states%5D=IL&filterrific%5Bsearch_query%5D=Top%20Times | GET | 200 | IL meet-index name filter `Top Times`: 20 rows, newest 2024-03-23 (2A, meet 84225) / 2024-03-22 (1A, meet 84224); nothing in 2025/2026 | 09-20T09:09 |
| https://www.directathletics.com/results/track/84225.html | GET | 200 | `Illinois Top Times Indoor Track & Field Championships (2A)` meet sheet, March 2024 | 09-20T09:09 |
| https://www.tfrrs.org/lists/4524/Illinois_Top_Times_1A | GET | 200 | TFRRS mirror `TFRRS \| Illinois Top Times (1A)`; 1,471 date-stamped rows, all 2024 (no athlete links server-side) | 09-20T09:09 |
| https://www.directathletics.com/lists/track/1355_290.html | GET | 200 | Chicago Public Schools league list: 360 rows, all May 2007 — historical | 09-20T09:10 |
| https://www.directathletics.com/legacy_da/results?filterrific%5Bwith_states%5D=IL | GET | 200 | IL results index page 1 (newest 100): 78×2026 + 22×2025 dates, all collegiate (TFRRS links) — no HS rows | 09-20T09:10 |
| https://www.directathletics.com/legacy_da/results?filterrific%5Bwith_states%5D=IL&filterrific%5Bsearch_query%5D=High%20School | GET | 200 | IL meet names containing `High School`: 83 rows, newest 03/02/2024 (Cogdal HS Invite) — no 2025/2026 row matches this name filter | 09-20T09:10 |
| https://www.directathletics.com/leagues/track/1468.html | GET | 200 | league home → `list_hnd=4525` | 09-20T09:10 |
| https://www.directathletics.com/leagues/track/1469.html | GET | 200 | league home → `list_hnd=4526` | 09-20T09:10 |
| https://www.ihsa.org/ | GET | 200 | entry HTML references `/assets/main-B2rKN2jy.js` (hash unchanged since report 13's capture) | 09-20T09:12 |
| https://www.ihsa.org/assets/main-B2rKN2jy.js | GET | 200 (×2) | entry bundle imports `./StateSeries-w2fH3odu.js` → the id-map chunk belongs to the live deployment (fetched twice: inventory + saved capture) | 09-20T09:12 |
| https://www.ihsa.org/sports/girls-cross-country/state-series/xc-state-central | browser load (headless Chromium, `request` listener) | page 200 / XHR 200 | live girls XC page requested `/v1/2025-26/statefinal/cc-qualifiers?tournamentId=691` on load and `=692`,`=693` on the 2A/3A class-tab clicks; zero `athletic.net` requests | 09-20T09:07 |
