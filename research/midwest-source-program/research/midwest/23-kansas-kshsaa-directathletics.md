# 23. Kansas — KSHSAA + DirectAthletics

Status: complete
Observed on: 2026-09-19

Scope of this report: (a) Kansas school/team universe from the state association (KSHSAA) plus the
KSHSAA championship-history site, (b) DirectAthletics Kansas team IDs and coverage, (c) the official
Kansas track & field / cross-country result surfaces and the timer landscape, (d) coach-information
availability, (e) access classification and Athletic.net request avoidance.

All network evidence below was collected 2026-09-19 23:12–23:22 CDT (America/Chicago) with `curl`
from this machine using a desktop browser User-Agent. No CAPTCHA/auth/paywall was bypassed; no cookie
or token was replayed; nothing outside `/home/lewis/Downloads/midwest-tfxc-source-research/` was
written. Total: ~40 requests to `kshsaa-api.kshsaa.org`, 2 to `www.kshsaa.org`, ~26 to
`www.kshsaachamps.org`, ~24 to `www.directathletics.com`, 7 to `ks.milesplit.com`, 5 to
`www.gobound.com`, 8 to `midwesttiming.com` + its public Supabase project — all sequential, all
within the ≤ ~50/host guideline; no 429 and no `Retry-After` was ever returned.

## Source

Provider/site list (all Kansas-specific):

| # | Source | URLs used | Role |
|---|---|---|---|
| 1 | KSHSAA public JSON API (the data layer behind the redesigned kshsaa.org React SPA) | `https://kshsaa-api.kshsaa.org/` — `directory/*`, `PublicClassifications`, `schedules/*`, `playoff-sites/*`, `ActivityMainPage/PageData/*`, `standings/*` | school universe, classifications, AD contacts, state-site metadata |
| 2 | KSHSAA web app shell | `https://www.kshsaa.org/` → `/react/assets/index-f33aafe3.js` | endpoint discovery (2.9 MB bundle; 1304-byte SPA shell) |
| 3 | KSHSAA championship history / results (separate host, own ID space) | `https://www.kshsaachamps.org/` — `/Shared/*`, `/StateChampions/*`, `/Results/*` | champion history, individual state/regional results, rosters **with grade** |
| 4 | DirectAthletics (Kansas slice) | `https://www.directathletics.com/` — `/legacy_da/results?filterrific[with_states]=KS`, `/results/{track,xc}/{id}.html`, `/teams/{track,xc}/{id}.html`, `/athletes/{track,xc}/{id}.html`, `/leagues/track/{id}.html` | meet/team/athlete IDs for the subset of Kansas meets DAT hosts |
| 5 | MileSplit Kansas | `https://ks.milesplit.com/` — `/meets/751194-kshsaa-state-championships-2026/entries`, `/timing`, `/timing/782/midwest-timing-and-results` | official state-meet entry list; Kansas timer index |
| 6 | Midwest Timing & Results (timer; Leavenworth, KS) | `https://midwesttiming.com/` (React SPA) + public Supabase project `wkyzbfnhunrtyqjewpna.supabase.co` | per-meet results metadata + compiled-result PDFs for the big Kansas share |
| 7 | Bound / gobound.com Kansas | `https://www.gobound.com/ks/schools`, `/ks/schools/{slug}`, `/direct/programs/{id}/show` | school athletics sites (Kansas schools redirect their athletics domains to Bound) |

Unreachable / blocked, recorded as findings: `https://www.kansascoaches.com/` (Kansas Coaches
Association, linked from KSHSAA nav) — connection timeout, `curl` exit 28 after 30 s, no HTTP status.
`https://www.athletic.net/` remains Cloudflare-403 to non-browser clients from this machine (per
brief; not re-tested). Web search was not usable: the `web_search` tool returned the literal error
`Sign up and repeat your request.` — after that quota/usage error I stopped using web search for this
assignment and relied only on direct HTTP evidence.

## Coverage

- **States**: Kansas only (this report). All three layers are Kansas-specific: KSHSAA/kshsaachamps are
  state association sources; DirectAthletics and Midwest Timing are cross-state but were filtered to
  Kansas here.
- **Sports**: boys/girls Cross Country, boys/girls Track & Field (outdoor); the KSHSAA championship
  site additionally carries **indoor** Track & Field activity IDs (67/68) — see Stable identifiers.
- **School level**: high school (KSHSAA "Member Schools", grade span `9-10-11-12`); the KSHSAA
  directory also contains Jr High/Middle schools (294 of the 526 name-search-"a" rows) which must be
  filtered out for HS work. DAT and Bound additionally carry middle-school teams.
- **Seasons / historical depth**:
  - KSHSAA live directory + classification: classification year 2026 = `2025-2026`, status `1`.
  - KSHSAA state-series metadata: TF state site for year 2026 (May 29–30 2026, completed);
    XC state sites for year 2027 = `2026-2027` (Oct 31 2026, upcoming).
  - kshsaachamps TF boys results: **2000–2026** in one request (20,887 individual-result rows);
    XC boys: **2000–2025** (1,311,297 bytes); champion history reaches **1911** (TF boys, 458 rows)
    and **1956** (XC boys, 345 rows).
  - DAT Kansas results index: 42 pages × 100 rows (all-time, all levels, state=KS).
  - Midwest Timing event metadata: 336 events for 2026, 279 with a published compiled-results PDF;
    their MileSplit timing page links meets back to **2007**.

## Enumeration

### Schools (KSHSAA members) — exact recipe, 3 requests

1. `GET https://kshsaa-api.kshsaa.org/PublicClassifications` → JSON, **348 classified member schools**
   (`6A 36, 5A 36, 4A 36, 3A 64, 2A 64, 1A 112`), each
   `{schoolId (int), name, finalEnrollment, classId, moved}`. 6A enrollment range min 1323 / max 2481.
2. `GET https://kshsaa-api.kshsaa.org/directory/search/name/{term}/` — substring search, no
   pagination cap observed:
   - term `HS` → **339** records (338 `High School`, 1 `Jr High School`)
   - term `Academy` → 15, term `School` → 40
   - union of the three → **394 distinct school names**, and `comm -23` against the 348 classified
     names is **empty** → the 3-request union covers the whole classified member universe.
3. `GET https://kshsaa-api.kshsaa.org/directory/Id/{numericId}` → full school record for one school
   (all fields in Stable identifiers below).

Cross-check (Kansas state-championship ID space, 1 request):
`GET https://www.kshsaachamps.org/Shared/GetSchools` → **940** `{SchoolID, Name, City, State, Active}`
(historical, includes long-closed schools; different ID space — see Stable identifiers).

Bound (school athletics platform, 1 request): `https://www.gobound.com/ks/schools` renders **543**
Kansas school pages; 309 slugs end in `hs`, 112 in `ms/jh`, 122 are unsuffixed (academies, middle
schools, districts).

### Track/XC participation per school

`GET https://kshsaa-api.kshsaa.org/schedules/crossCountrySchedule/teamschedule/{schoolId}/{gender}`
(gender `1`=boys, `2`=girls) returns, for a real member school:

```
200 {"activityData":[…],"meetsData":[…],
     "schoolData":{"Activity":14,"Organization":735,"Gender":1,"GenderActivity":3,
                   "KATSSchoolName":"Shawnee Mission East HS","School":"Shawnee Mission East HS",
                   "Mascot":"Mascots/SM-EastHS.gif","SchoolType":1,
                   "GroupTitle":"Member Schools","Status":1}}
```

i.e. per school + gender it confirms KSHSAA-recognized XC participation (`Activity 14`,
`GenderActivity 3` boys / `4` girls, classification activity codes `CC` / `GCC` in `activityData`) and
would return that school's XC meet schedule. **`meetsData` was empty for every school probed on
2026-09-19** (season in progress; SME boys and girls both `[]`), so this endpoint currently proves
participation but not schedules.

KSHSAA does **not** publish TF/XC weekly schedules or XC standings:
`GET /schedules/weeklyschedule/onload/Football` → 200 (13,095 bytes, classData/yearData/weekData/
schoolData) while `.../onload/Cross country`, `.../onload/Track & field` and `.../onload/cross country`
all → **HTTP 500**; `GET /standings/CrossCountry/ByClassAndGender/6A/1` and `/1A/1` → 200 with
`standingsData: []`; `GET /standings/Track&Field/ByClassAndGender/6A/1` → **404**.
`GET /playoff-sites/regional-sites/Regional/cross%20country/60` → 200 but `siteData: []` (2026-27 XC
regional assignments not yet published as of 2026-09-19).

### Meets / state series

- `GET https://kshsaa-api.kshsaa.org/playoff-sites/state-sites/state/track%20&%20Field/11` → state TF
  site `ID 4135`, `PlayoffLevel: State`, `Class: "All Classes"`, `2026-05-29`→`2026-05-30`,
  `Wichita State University-University Stadium`, **`SiteURL: https://milesplit.live/meets/751194`**.
- `GET .../playoff-sites/state-sites/state/cross%20country/60` → six rows for `2026-10-31`:
  6A/5A/3A at `Lawrence-Rim Rock Farm`, 4A/2A/1A at `Wamego Country Club`, all `SiteURL: ""`.
- Activity hubs (`GET /ActivityMainPage/PageData/GetCardsAndLinksByActivity/{cross country|Track & Field}`)
  carry a "Meets & Results" card whose only link is
  `https://ks.milesplit.com/results?month=&year=&level=hs` (XC) / `https://ks.milesplit.com/results` (TF),
  plus "Champion History" links into `kshsaachamps.org` and a "Postseason Info" card linking the
  state/regional pages above.
- DirectAthletics Kansas meets: `GET /legacy_da/results?filterrific[with_states]=KS` → newest-first,
  100 rows/page, 42 pages; per-meet pages such as `/results/track/96776.html` (SCBL HS Track 2026) and
  `/results/xc/28178.html` (Wichita State JK Gold High School Classic).
- Midwest Timing: `events` table (public JSON) → 336 events for year 2026 with `name, date, location,
  status, milesplit_link, pdf_link`.

### Athletes / Class of 2027

- **MileSplit state-meet entries**:
  `GET https://ks.milesplit.com/meets/751194-kshsaa-state-championships-2026/entries` (1,956,659 bytes)
  yields **2,331 distinct athlete URLs** (`/athletes/{id}-{slug}`) and **325 distinct team URLs**
  (`/teams/{id}-{slug}`) for the 2026 KSHSAA state TF meet.
- **kshsaachamps individual results**: `RetrieveResultsByActivity` enumerates every state-meet
  individual result (name + school + mark + place) and for XC also **grade** (see Athlete evidence).
- **kshsaachamps rosters**: `GET /Shared/RetrieveRoster?activityYearClassID={}&schoolID={}` enumerates
  named athletes **with grade** — currently loadable only for the champion team of each class-year.
- No Kansas source observed exposes a "Class of 2027" search facet; the cohort is derived from
  grade/class-year fields (KSHSAA roster `Grade`, XC result `Grade`) or from `Year` on DAT rosters.

## Stable identifiers

### KSHSAA (kshsaa-api.kshsaa.org)

| ID | Where observed | Example | Notes |
|---|---|---|---|
| `Identifier` (string) | `directory/*` | `KSS0307` | KSHSAA's school code; stable across years |
| `Id` (int) | `directory/*` | `735` | **same space as `schoolId` in PublicClassifications** — verified: classifications row `{"schoolId":735,"name":"Shawnee Mission East HS"}` matches `directory/Id/735` (`Identifier KSS0307`). Use this as the join key |
| `League` / `LeagueName` | school record | `KSL0042` / `Sunflower` | league code + label |
| `classId` / `Class` / `FBClass` | classifications + school record | `6`, `6A` | numeric + label |
| `USD` | school record | `512` | school district number |
| Activity IDs | `activityData` / activity URLs | XC = `60` (state-tournament ActivityID), `14` (`GlobalActivityID`, MaxPreps `ActivityID`), `3`/`4` = boys/girls gender-activity; TF = `11` | multiple ID spaces — do not conflate |
| `yearData.Id` | `playoff-sites/*` | XC `2027` (`2026-2027`), TF `2026` (`2025-2026`) | KSHSAA school-year key |
| Site IDs | `playoff-sites/state-sites/*` | `4135` (TF state), `4179` (XC 6A state) | state/regional site records |
| `OrgActID` | `regional-sites` payload | `3` | school organization-activity key |

### kshsaachamps.org (separate ID space — do **not** merge with the above)

| ID | Example | Notes |
|---|---|---|
| `SchoolID` | `8` (Washburn Rural), `646` (Olathe North) | 940 rows from `/Shared/GetSchools`; distinct from KSHSAA `Id` |
| `ActivityID` | `50` TF girls, `51` TF boys, `56` XC girls, `57` XC boys, `67`/`68` indoor TF (B/G) | `/Shared/GetActivities`; also used as `?i=` on champion pages |
| `RecordActivityTypeID` | `5`/`6` TF state meet (B/G), `7`/`8` indoor, `9`/`10` outdoor-English | `/Shared/GetRecordActivities` |
| `ActivityYearID` | `2309` (2025 6A XC boys), `2346` (2026 TF boys) | year+activity key; also the id used by `SearchDetailPopup/{id}` |
| `ActivityYearClassID` | `8283` (2025 6A XC boys), `8465` (2026 6A TF boys) | year+activity+class key — the key for results and rosters |
| `ActivityYearClassIndividualResultID` | `187267` | individual result row |
| `ActivityYearClassTeamResultID` | `55257` | team result row |
| `ActivityYearClassRosterID` | `79358` | roster row (athlete or coach) |

### DirectAthletics

`/teams/{track|xc}/{teamId}.html` (separate per sport+gender; e.g. Argonia men's TF `3714`, women's TF
`3715`, Wichita-East women's XC `4058`), `/athletes/{track|xc}/{athleteId}.html` (e.g. `9115578`),
`/results/{track|xc}/{meetId}.html` (e.g. `96776`), event-level
`/results/{track|xc}/{meetId}_{eventResultId}.html` (e.g. `96776_5997090`), `/leagues/track/{leagueId}.html`
(Kansas league `1252` = "KSHSAA 1A"). Note the sport in the path is part of the identity — a team's
TF and XC pages have different IDs.

### Others

MileSplit: `/meets/{meetId}-{slug}` (state TF `751194`), `/athletes/{id}-{slug}`,
`/teams/{id}-{slug}`, `/timing/{timingCompanyId}/{slug}` (29 Kansas timing companies; Midwest Timing
= `782`). Midwest Timing: Supabase `events.id` = UUID, `state_meet_documents.id` = UUID, PDF paths
`results/{year}/{filename}.pdf` and `results/state-meet/{doc_type}-{class}.pdf`. Bound:
`/ks/schools/{slug}` and `/direct/programs/{programId}/show` (program id embeds a creation timestamp,
e.g. `h20260918033635217b68ae5ab080549`).

## Athletic.net leverage

**Observed: zero direct Athletic.net URL/ID exposure from any Kansas source tested.**
`grep -i 'athletic\.net|athleticnet|AthleticNet'` over the nine largest artifacts retrieved
(KSHSAA React bundle, MileSplit state-entries page, MileSplit timing index, DAT Kansas results index,
DAT JK Gold meet page, Midwest Timing JS bundle, Bound school + school-index pages) returned **0
matches**. The KSHSAA "Meets & Results" card points at MileSplit, the state TF `SiteURL` is
`milesplit.live/meets/751194`, and Bound/Midwest Timing pages reference only themselves, MileSplit and
TFRRS. So Kansas association/timer sources can **seed** Athletic.net lookups but never hand over an
Athletic.net TeamID/MeetID/ResultID.

Deterministic seeding that *is* available:

- school → Athletic.net team lookup: KSHSAA `SchoolName` + `PhysicalCity`/`PhysicalState` + class
  (e.g. "Shawnee Mission East HS", "Prairie Village", KS) — 348 classified schools = 348 seeds.
- meet → Athletic.net meet lookup: Midwest Timing `events.name` + `date` + `location` (336 rows for
  2026), DAT `results/{sport}/{id}` name + date.
- athlete → Athletic.net profile lookup: name + school + grade from kshsaachamps XC results/rosters,
  or name + school + `Year` from DAT rosters.

Estimated Athletic.net request avoidance (all inputs observed above; arithmetic is `[INFERENCE]`):

| Lever | Observed input | Avoided Athletic.net work (estimate) |
|---|---|---|
| School universe | 348 classified schools from 1 request | 348 Athletic.net team-search requests |
| KS XC state grade verification, 2025 season (Class-of-2027 = grade 11) | 143 boys graded rows (50 grade 11) + 143 girls graded rows (39 grade 11) = **89 grade-11 records** from 2 requests | 89 Athletic.net profile fetches whose only purpose was grade/identity confirmation |
| KS TF state validation, 2026 | 20,887 boys individual rows 2000–2026 (and 2 more activity IDs for girls); name+school+mark+place, **no grade** | name/school/meet validation without Athletic.net; ~2,300 state-meet entries |
| Champion rosters with grade | 1 team per class-year (`RosterExists=true` for 1 of 10–12 teams/class); 18 athletes for 2026 6A TF, 10 for 2025 6A XC | ~60–110 athletes/activity-year whose grade is association-verified |
| Weekly new-meet discovery | Midwest Timing `events` JSON (336 rows/yr, 279 PDFs) + DAT KS index page 1 | weekly Athletic.net meet-hunting requests |

Net: Kansas gives a cheap **association-grade-verified** seed set of roughly 90–200 Class-of-2027
identities per season and a complete 348-school seed list, but no Athletic.net IDs, so Athletic.net
acquisition still has to be driven by the browser pipeline (live Athletic.net is 403 from this
machine).

## Athlete evidence

| Field | KSHSAA API (kshsaa-api) | kshsaachamps results | kshsaachamps roster | DAT | MileSplit KS | Midwest Timing |
|---|---|---|---|---|---|---|
| Name | – (school+AD only) | ✅ individual string (e.g. `"Jacob D'Souza"`) | ✅ `FirstName`/`LastName`/`FullName` | ✅ team roster + athlete page | ✅ entries | ✅ results PDFs |
| Graduating class / grade | – | ✅ **XC only** (`Grade`; boys XC 2025: 143 graded rows, 50 = grade 11) | ✅ `Grade` (9–12) | ✅ `Year` on roster page (`FR/SO/JR/SR`); athlete page `Class:` | not observed in HTML | – |
| School | ✅ | ✅ + `SchoolID` | ✅ + `SchoolID` | ✅ team identity | ✅ team link | ✅ in PDFs |
| City/state | ✅ physical + mailing address, `USD` | city via `/Shared/GetSchools` | – | – | meet `Wichita, KS` | ✅ `location` |
| Gender/category | ✅ gender-keyed endpoints (`Gender` 1/2, `GenderActivity` 3/4) | ✅ separate activity IDs per gender | ✅ per activity | ✅ sport+gender team IDs | ✅ | ✅ |
| TF/XC distinction | ✅ (Activity 11 vs 60/14) | ✅ activity IDs | ✅ `ActivityID` | ✅ path `track` vs `xc` | ✅ | ✅ event `sport` |
| Indoor/outdoor | indoor optional (activity `MaxPrepsWidget`/indoor activities) | ✅ activities 67/68 indoor vs 50/51 outdoor | – | ⚠️ indoor results possible, not proven | – | ✅ DAT index shows `Indoor`/`Outdoor` filters |
| Performances / PRs | – | ✅ mark in `Statistic1` + `PlaceFinish` | – | ✅ PR table (indoor/outdoor bests per event) | ✅ results tabs | ✅ compiled PDFs |
| Progression | – | ✅ year over year (2000–2026) | – | ✅ full result history on athlete page | – | – |
| Meets | XC team schedule endpoint (empty in Sept) | ✅ + regional ("Regional Champions & Results") | – | ✅ Latest Results list | ✅ | ✅ 336 events |
| Athlete profile URL | – | – | – | ✅ `/athletes/{sport}/{id}.html` | ✅ `/athletes/{id}-{slug}` | – |

Worked example (grade-11 XC cohort, 2025): `Jacob D'Souza | Blue Valley Northwest | 15:42.50 | p1 |
grade=11` from activityID 57; `Charlotte Hardy | Shawnee Mission East | 18:23.50 | p1` from
activityID 56. Both are Class-of-2027 grade evidence published by the state association.

## Recruiting information

- **Head track coach / head XC coach / assistants**: not published in the KSHSAA member directory —
  `directory/Id/{id}` contains `PrincipalName`, `Email` (principal), `ADName`, `ADEmail` and **no
  coach field**. Coach names do appear in two KSHSAA-adjacent places:
  - kshsaachamps champion history `Coach`: XC 2025 rows carry role prefixes —
    `"HC: Matthew Swedlund; AC: Ian Cropp, Elizabeth Sigvaldson, Donnie Palmer"` (6 of 345 XC rows are
    in this HC/AC format; TF rows are a bare name, e.g. `"Levi Huseman"` for 2026 6A) — 422 of 458 TF
    rows have a non-empty coach string.
  - kshsaachamps rosters: older seasons include explicit coach rows
    (`{"FullName":"Huseman, Levi (Coach)","Coach":true,"CoachIndicator":"Yes"}`, 2015 Olathe North XC;
    `"Swedlund, Matthew (Coach)"`, 2019 Washburn Rural XC).
- **Athletic director**: ✅ `ADName` + `ADEmail` for **339/339** high-school records in the sampled
  search (`KSS0307 → "Ryan Johnson" / ryanjohnson@smsd.org`). Public professional contact only.
- **School athletics website**: ✅ `WebSite` present for **329/339** sampled HS records
  (`https://www.smelancers.com` for Shawnee Mission East).
- **Team website**: not published by KSHSAA; Kansas schools often run their athletics site on Bound —
  `https://www.smelancers.com` redirected to `https://www.gobound.com/ks/schools/shawneemissioneasths`
  (Bound school page with Schedule/Roster/Staff tabs per program).
- KCA (`kansascoaches.com`), linked from the KSHSAA nav, was **unreachable** (connection timeout) — a
  coach-association directory could not be evaluated.
- Privacy hazard (do **not** ingest): the KSHSAA JSON carries `PrincipalCell`, `ADCell`, `PresCell`
  (personal mobile numbers) and `PresName`/`PresEmail`. Only `ADName` + `ADEmail` + school `WebSite` +
  school phone/fax are inside our contract; cell fields must be dropped at parse time.

## Result evidence

| Field | KSHSAA (kshsaa-api) | kshsaachamps | DirectAthletics | Midwest Timing | MileSplit KS |
|---|---|---|---|---|---|
| ResultID | – | `ActivityYearClassIndividualResultID` / `…TeamResultID` | event-result page id (`96776_5997090`) | `state_meet_documents.id` (UUID) | site-internal |
| AthleteID | – | – (name string only) | ✅ `/athletes/{sport}/{id}` | – | ✅ `/athletes/{id}-{slug}` |
| MeetID | state site `ID` (4135/4179) | `ActivityYearID` (2309/2346) | ✅ `96776`, `28178` | `events.id` (UUID) + `milesplit_link` | ✅ `751194` |
| EventID | – | `Category` string + `CategorySort` (no numeric event id observed) | event result id suffix | doc `doc_type` (`performance`/`heatsheet`) | site-internal |
| Mark | – | ✅ `Statistic1` (`"9:14.17"`, `"100"`, team scores) | ✅ time/mark on event page | ✅ in PDFs | ✅ |
| Normalized mark inputs | – | partial: unit is implied by `Category`; field events carry raw distance/height | not observed in HTML | not observed | not observed |
| Timing method | – | – | – | ✅ DAT/MileSplit "Timing/Results: Midwest Timing & Results" is the publish path (timing method text not published) | ✅ timer attribution on meet page |
| Wind | – | – | – | – | – |
| Implement/hurdle spec | – | – | – | – | – |
| Heat/round | – | `Category` encodes prelim/final only for some activities; TF rows are final placings | ✅ `Prelim/Final` column on athlete page | ✅ heatsheets per class (`heatsheet-{1a..6a,relay,wheelchair}.pdf`) | ✅ |
| Place | – | ✅ `PlaceFinish` | ✅ | ✅ | ✅ |
| Date | ✅ state site dates | ✅ `Year` (activity-year) | ✅ date column | ✅ `date` (e.g. `Apr 24`) | ✅ |
| School represented | ✅ school IDs | ✅ `SchoolID` + `School` | ✅ team page/school name | ✅ in PDFs | ✅ team links |
| Relay membership | – | ⚠️ relay rows exist (`"Boys 4x800 Meter Relay"`) but the `Student` string is **empty** for relays | ✅ relay event pages | ✅ relay heatsheet PDF | ✅ |

Practical consequence: Kansas state **team/individual** results are usable as validation and seeding,
but individual relay-leg membership is not resolvable from kshsaachamps (empty `Student` on relay
rows); relay legs would need DAT event pages or MileSplit instead.

## Incremental use

A weekly Kansas collector needs ~6–10 requests total:

1. **New/changed meets** — `GET https://wkyzbfnhunrtyqjewpna.supabase.co/rest/v1/events?select=id,name,date,location,status,year,milesplit_link,pdf_link&year=eq.2026`
   with the site's public anon key (~1 request, JSON). Diff on `id` + `status` (`upcoming` →
   `live` → `posted`) and on `pdf_link` becoming non-null; `milesplit_link` gives the join to MileSplit
   without a MileSplit search. Observed 336 rows for 2026 / 279 with PDFs.
2. **New KS meets not timed by Midwest Timing** — `GET https://www.directathletics.com/legacy_da/results?filterrific[with_states]=KS&page=1`
   (newest-first, 100 rows) diffed against the previous week's page 1.
3. **New state-series results** — after each state meet:
   `GET https://www.kshsaachamps.org/Results/RetrieveResultsByActivity?startYear=2027&endYear=2027&activityID=57|56|51|50`
   (one request per activity/year returns the complete set; filter rows by `Year`).
4. **Grade-verified cohort refresh** — same 2 requests for XC (activity 57/56, `Grade==11` in the
   season that matches the target graduating class).
5. **School-universe drift** — `GET /PublicClassifications` (1 request) + the 3 name searches, diffed
   by `schoolId`; per-school `DateModified` in `directory/Id/{id}` shows when a record changed.

No design here requires a historical re-fetch: every source returns a complete current-state document
in 1 request (classifications, activity-year results, events table, page 1 of the DAT index).

## Access characteristics

- **kshsaa-api.kshsaa.org — public structured JSON, undocumented API** (discovered from the public JS
  bundle). No auth, no key, CORS-open, `application/json`. One root `GET /` returns `OK`. Error paths
  return HTML/JSON stack traces that leak server internals (`"TypeError: Cannot read properties of
  undefined (reading 'GlobalActivity') at C:\home\site\wwwroot\controllers\Schedules\generalSchedule-routes.js:75"`;
  ASP.NET `KSHSAA.Controllers.SharedController` null-parameter pages) — recorded as a robustness
  observation, not used. No rate-limit headers observed; ~40 sequential requests produced no 429.
- **www.kshsaa.org — browser application** (React SPA, empty shell + 2.9 MB bundle).
- **www.kshsaachamps.org — normal HTML + public JSON endpoints** (ASP.NET MVC + Kendo; JSON reads under
  `/Shared/*`, `/StateChampions/*`, `/Results/*`). No auth. Large single-response payloads
  (5.9 MB for 27 years of TF boys results); `RetrieveRoster` requires both `activityYearClassID` and
  `schoolID` (missing → 500 HTML error page).
- **www.directathletics.com — browser application + normal HTML**; meet/team/athlete pages are
  server-rendered and crawlable; the results index is `/legacy_da/results` with `filterrific[...]`
  query params and plain `?page=N` pagination.
- **midwesttiming.com — browser application (React SPA) over public Supabase REST + public storage**;
  the anon key ships in the client bundle; `events`, `results` (storage bucket), `state_meet_documents`
  are readable anonymously, `schools` returned 0 rows to anon (RLS or empty). Published artefacts:
  15 state-meet documents (performance + heatsheet PDFs for 1A–6A, relay, wheelchair) and per-meet
  compiled-results PDFs; ranged `GET` of `performance-1a.pdf` → `206`, `application/pdf`,
  `content-length 415008`.
- **ks.milesplit.com — normal HTML / browser application** (entries and results pages are
  client-rendered; the timing index and meet metadata are server-rendered).
- **www.gobound.com — browser application**; `/ks/schools` renders 543 links server-side, roster rows
  load client-side (SME girls XC roster showed headers `Name | Year` with no rows at observation time).
- **www.kansascoaches.com — unavailable** (`curl` exit 28, connection timeout, no HTTP status).
- **No 429/`Retry-After` was observed from any host.** Documented limits: none published; the only hard
  constraint found is payload size (`RetrieveResultsByActivity` 2000–2026 = 5.9 MB).

## Recommendation

**DISCOVERY-ONLY + VALIDATION** for KSHSAA/kshsaachamps (with **COACH-DIRECTORY** for AD contacts),
**RESULT-SOURCE** for Midwest Timing and DAT, and **ATHLETIC.NET-SEED** for the deterministic
school/meet seeds.

Expected marginal coverage:

- **School layer (high confidence)**: complete Kansas HS universe — 348 classified members with
  enrollment/class/league, `Identifier` + numeric `Id`, AD name/email, school website — for **3
  requests**, and no Athletic.net usage at all. This is the strongest Kansas deliverable.
- **Athlete layer (narrow but authoritative)**: ~89 grade-11 XC records for the 2025 season plus
  ~60–110 champion-roster athletes with grades per activity-year, plus all state-TF individual results
  as name/school/mark validation. Kansas will not discover masses of Class-of-2027 athletes that
  Athletic.net misses — state-level results only cover qualifiers (top-20 per class in XC, state
  entries in TF) — but it supplies **association-grade evidence** for that subset, which is the one
  thing Athletic.net profiles are being fetched for today.
- **Meet/result layer (medium)**: Midwest Timing covers the largest share of Kansas HS meets and the
  state TF meet with a 1-request JSON event index plus public result PDFs; DAT covers a smaller,
  mostly Wichita-area/1A-league subset with clean IDs (`[INFERENCE]` from the two meet pages and one
  league page sampled, not a statewide census); MileSplit carries the official state entry list
  (2,331 athletes / 325 teams) and the timer index (29 Kansas timers).
- **Biggest gaps**: no coach names/emails from the association (AD only); no wind/implement/heat fields
  anywhere; relay legs lost in kshsaachamps; rosters with grade exist only for champion teams; DAT has
  no state-wide team index (only per-class league pages found for 1A); KCA unreachable.

### Evidence appendix

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.kshsaa.org/` | GET (`-L`, browser UA) | 200 (1304 B) | kshsaa.org is a React SPA shell; bundle path `/react/assets/index-f33aafe3.js` | 2026-09-19 23:12 CDT |
| `https://www.kshsaa.org/react/assets/index-f33aafe3.js` | GET | 200 (2,915,263 B) | API base `https://kshsaa-api.kshsaa.org`; routes `/school-directory`, `/approved-schools`; activity IDs XC=60, TF=11; state-results link `milesplit.live/meets/751194`; nav → kshsaachamps.org, kansascoaches.com, arbiter | 2026-09-19 23:13 |
| `https://kshsaa-api.kshsaa.org/` | GET | 200 `OK` | API host live, no auth | 2026-09-19 23:13 |
| `…/directory/search/name/Shawnee/` | GET | 200 (12,374 B, 13 rows) | school record schema incl. `Identifier`, `Id`, `League`, `USD`, `ADName/ADEmail`, `WebSite`; exposes `PrincipalCell`/`ADCell` (privacy hazard) | 2026-09-19 23:13 |
| `…/directory/Id/735` | GET | 200 (962 B) | single-school endpoint; `Identifier KSS0307`, `Id 735`, `Class 6A`, `SchoolType High School`, `SchoolSubType 9-10-11-12` | 2026-09-19 23:15 |
| `…/PublicClassifications` | GET | 200 (31,885 B) | 348 classified schools; classes 6A/5A/4A/3A/2A/1A = 36/36/36/64/64/112; `year 2026 = 2025-2026`; per-school `schoolId`,`name`,`finalEnrollment`,`classId`,`moved` | 2026-09-19 23:14 |
| `…/PublicClassifications?activityID=11` | GET | 200 (31,885 B, identical) | no per-activity classification filtering; general classification only | 2026-09-19 23:21 |
| `…/directory/search/name/HS/` | GET | 200 (339 rows) | 338 High School + 1 Jr High; class distribution 1A:111…6A:36; 339/339 with AD name+email, 329 with WebSite | 2026-09-19 23:16 |
| `…/directory/search/name/Academy/`, `…/name/School/` | GET | 200 (15 / 40 rows) | union with `HS` = 394 distinct names; `comm -23` vs 348 classified names is empty → 3-request full coverage | 2026-09-19 23:17 |
| `…/directory/leagues`, `…/directory/league`, `…/directory/league/KSL0042` | GET | 404 / 404 / 200 `[]` | no bulk league index; league endpoints need internal ids | 2026-09-19 23:14 |
| `…/directory/league-schools/KSL0042` | GET | 200 (13,103 B) | per-league school listing = 13,103 B of full school records (9–13 schools/league) | 2026-09-19 23:14 |
| `…/schedules/crossCountrySchedule/teamschedule/735/1` and `/2` | GET | 200 (946/947 B) | per-school XC participation: `Activity 14`, `GenderActivity 3` (boys) / `4` (girls), `GroupTitle "Member Schools"`, mascot path; `meetsData` empty | 2026-09-19 23:16 |
| `…/schedules/crossCountrySchedule/teamschedule/735/2027` and `/2026` | GET | 200 `{"meetsData":[],"schoolData":null}` | second path segment is **gender**, not year | 2026-09-19 23:14 |
| `…/schedules/generalSchedule/teamschedule/735/6A/2027` and `/2026` | GET | 500 (Node stack trace leak) | generic fallback route misused; leaks `C:\home\site\wwwroot\controllers\Schedules\generalSchedule-routes.js` | 2026-09-19 23:14 |
| `…/schedules/weeklyschedule/onload/Football` | GET | 200 (13,095 B) | weekly-schedule feature exists for team sports (class/year/week/school data) | 2026-09-19 23:18 |
| `…/schedules/weeklyschedule/onload/{Cross country, Track & field, cross country}` | GET | 500 (×3) | KSHSAA does **not** publish TF/XC weekly schedules | 2026-09-19 23:18 |
| `…/standings/CrossCountry/ByClassAndGender/6A/1`, `/1A/1` | GET | 200, `standingsData: []` | XC standings page exists but carries no data | 2026-09-19 23:18 |
| `…/standings/Track&Field/ByClassAndGender/6A/1` | GET | 404 | TF standings slug not resolvable | 2026-09-19 23:18 |
| `…/playoff-sites/state-sites/state/track%20&%20Field/11` | GET | 200 (2,122 B) | state TF 2026: Wichita State, May 29–30, **`SiteURL https://milesplit.live/meets/751194`**; site id 4135 | 2026-09-19 23:15 |
| `…/playoff-sites/state-sites/state/cross%20country/60` | GET | 200 (10,336 B) | state XC 2026-10-31: 6A/5A/3A Rim Rock Farm, 4A/2A/1A Wamego CC; no SiteURL yet | 2026-09-19 23:15 |
| `…/playoff-sites/regional-sites/Regional/cross%20country/60` | GET | 200, `siteData: []` | 2026-27 XC regional assignments not yet published | 2026-09-19 23:22 |
| `…/ActivityMainPage/PageData/GetCardsAndLinksByActivity/cross%20country` | GET | 200 | "Meets & Results" card → `https://ks.milesplit.com/results?month=&year=&level=hs`; GlobalActivityID 14; champion-history/champs link set | 2026-09-19 23:15 |
| `…/ActivityMainPage/PageData/GetCardsAndLinksByActivity/Track%20%26%20Field` | GET | 200 | "Meets & Results" card → `https://ks.milesplit.com/results`; activity id 11; state/regional/qualifying-standard links | 2026-09-19 23:15 |
| `https://www.kshsaachamps.org/Shared/GetSchools` | GET | 200 (162,200 B) | 940 championship-era schools with own `SchoolID`, city/state/active | 2026-09-19 23:18 |
| `…/Shared/GetActivities` | GET | 200 (55,305 B) | activity IDs 50/51 TF G/B, 56/57 XC G/B, 67/68 indoor TF; `IndividualResultsSectionApplicable=true` for all six | 2026-09-19 23:18 |
| `…/Shared/GetRecordActivities` | GET | 200 (1,690 B) | `RecordActivityTypeID` 5/6 TF state, 7/8 indoor, 9/10 outdoor-English | 2026-09-19 23:19 |
| `…/Shared/RetrieveRoster?activityYearClassID=8465&schoolID=646` | GET | 200 (4,969 B) | 2026 TF 6A champion roster: 18 athletes with `Grade` (10/11/12), `ActivityYearClassRosterID`; 8 were grade 11 | 2026-09-19 23:19 |
| `…/Shared/RetrieveRoster?activityYearClassID=8283&schoolID=8` | GET | 200 (2,724 B) | 2025 XC 6A champion roster: 10 athletes with `Grade` (9/10/12/11) | 2026-09-19 23:20 |
| `…/Shared/RetrieveRoster?activityYearClassID=8283&schoolID=648` | GET | **200, `[]`** | non-champion (Olathe West) has no roster → grade rosters are champion-only | 2026-09-19 23:22 |
| `…/Shared/RetrieveRoster?activityYearClassID=38&schoolID=646` (2015) and `=5770&schoolID=8` (2019) | GET | 200 / 200 | historical rosters exist and include **coach rows** (`"Huseman, Levi (Coach)"`, `"Swedlund, Matthew (Coach)"`, `Coach:true`); `Grade:null` for coaches | 2026-09-19 23:20 |
| `…/StateChampions/RetrieveChampionHistory?activityID=51` | GET | 200 (157,867 B) | 458 TF-boys champion rows, years **1911–2026**, 422 with coach text, `RosterCount`, `ActivityYearClassID` | 2026-09-19 23:19 |
| `…/StateChampions/RetrieveChampionHistory?activityID=57` | GET | 200 (117,543 B) | 345 XC-boys rows, 1956–2025; 2025 coach strings carry roles: `"HC: Matthew Swedlund; AC: Ian Cropp, Elizabeth Sigvaldson, Donnie Palmer"` | 2026-09-19 23:20 |
| `…/Results/RetrieveResultsByActivity?startYear=2000&endYear=2026&activityID=51` | GET | 200 (5,925,020 B) | 27 seasons of TF-boys state results, 20,887 individual rows, **0 rows with `Grade`** | 2026-09-19 23:18 |
| `…/Results/RetrieveResultsByActivity?startYear=2000&endYear=2025&activityID=57` | GET | 200 (1,311,297 B) | XC-boys seasons 2000–2025; ~120 individual rows/season with `Grade` except 2020/2021 | 2026-09-19 23:18 |
| `…/Results/RetrieveResultsByActivity?startYear=2025&endYear=2025&activityID=57` | GET | 200 (63,930 B) | 2025 XC-boys: 143 graded rows, **50 grade 11**; 6A individual results = top 20 at Rim Rock Farm + 4 regional champions | 2026-09-19 23:17 |
| `…/Results/RetrieveResultsByActivity?startYear=2025&endYear=2025&activityID=56` | GET | 200 (63,697 B) | 2025 XC-girls: 143 graded rows, **39 grade 11** (e.g. `Charlotte Hardy, Shawnee Mission East, 18:23.50`) | 2026-09-19 23:22 |
| `…/Results/RetrieveResultsByActivityYearClassID?activityYearID=2309` | GET | 200 (63,930 B) | per-activity-year-class result fetch returns team standings (place + score) and individual rows | 2026-09-19 23:19 |
| `…/Results/RetrieveResultsByCriteria?startYear=2020&endYear=2026&fullName=D'Souza&activityID=57&schoolID=0` | GET | 200 (897 B) | name/school search across state **and regional** results ("2025 / 6A … placed 1st for Blue Valley Northwest in Boys Regional Cross Country") | 2026-09-19 23:19 |
| `…/Results/SearchDetailPopup/2309/0/D%27Souza` | GET | 200 (3,340 B) | popup is a JS shell (no server-rendered results) | 2026-09-19 23:19 |
| `…/Scripts/kshsaa/ResultsQuery.js`, `ChampHistory.js`, `Shared.js` | GET | 200 / 200 / 200 | endpoint inventory (`/Results/RetrieveResultsBy*`, `/Shared/RetrieveRoster`, `/Shared/GetSchools`) | 2026-09-19 23:19 |
| `https://www.kshsaachamps.org/Results/` | GET | 200 (17,143 B) | results landing page is an empty shell ("Portions of this website do not yet contain complete KSHSAA historical information") | 2026-09-19 23:18 |
| `https://www.directathletics.com/` , `/results.html`, `/upcoming_meets.html`, `/search.html` | GET | 200 (31 KB / 52 KB / 32 KB / 13 KB) | Rails app with `filterrific[...]` filters, state + sport selects, pagination to page 792 site-wide | 2026-09-19 23:17 |
| `https://www.directathletics.com/legacy_da/results?filterrific[with_states]=KS` (+`&page=2`) | GET | 200 (52,649 B each) | 42 pages × 100 rows for Kansas; page 1+2 = 200 rows, 67 DAT-hosted + 133 tfrrs.org links; newest 2026-09-18 | 2026-09-19 23:18 |
| `https://www.directathletics.com/results/track/96776.html` | GET | 200 (20,660 B) | SCBL HS Track 2026 meet page: event result links `96776_5997090…`, 20 school team links `/teams/track/{id}.html` | 2026-09-19 23:19 |
| `https://www.directathletics.com/teams/track/3714.html` | GET | 200 (17,843 B) | Argonia Men's TF: `Leagues: KSHSAA 1A`, roster `Name | Year` (SO/JR/SR) | 2026-09-19 23:19 |
| `https://www.directathletics.com/athletes/track/9115578.html` | GET | 200 (20,854 B) | athlete page: `Class: 13`, PR table (800m 2:25.03 / 1600m 5:13.78 / 3200m 11:43.70), full result history with date/meet/event/prelim-final/mark | 2026-09-19 23:19 |
| `https://www.directathletics.com/leagues/track/1252.html` | GET | 200 (47,142 B) | `KSHSAA 1A` league index: 249 team pages / 125 distinct school names (men+women pairs) incl. JH/middle-school teams | 2026-09-19 23:21 |
| `https://www.directathletics.com/teams/xc/4058.html` (Wichita-East W XC) and `/teams/xc/2926.html` (Kapaun W XC) | GET | 200 / 200 | Kansas XC team pages carry rosters with `Year` = FR/SO/JR/SR and "Latest Results" meet lists | 2026-09-19 23:21 |
| `https://www.directathletics.com/results/xc/28178.html` | GET | 200 (216,448 B) | Wichita State JK Gold HS Classic 2026: 26 participating HS teams with XC team IDs | 2026-09-19 23:21 |
| `https://www.directathletics.com/teams.html?state=KS`, `/leagues/track.html`, `/legacy_da/teams` | GET | 200 (generic) / 404 / 404 | no state-filtered or global team index on DAT | 2026-09-19 23:21 |
| `https://ks.milesplit.com/meets/751194-kshsaa-state-championships-2026/entries` | GET | 200 (1,956,659 B) | 2,331 distinct athlete profile links + 325 team links; "Timing/Results: Midwest Timing & Results"; host = KSHSAA | 2026-09-19 23:19 |
| `https://ks.milesplit.com/timing` | GET | 200 (50,387 B) | **29 Kansas timing companies** with stable `/timing/{id}/{slug}` IDs | 2026-09-19 23:20 |
| `https://ks.milesplit.com/timing/782/midwest-timing-and-results` | GET | 200 (428,502 B) | Midwest Timing & Results (Leavenworth, KS), `midwesttiming.com`, 283 meet links, year selector back to 2007 | 2026-09-19 23:20 |
| `https://ks.milesplit.com/meets/751194-kshsaa-state-championships-2026/results` and `/results/1307550/formatted` | GET | 200 (69,172 B / 69,192 B) | results tab + `View Live Results` → `milesplit.live/meets/751194`; result bodies are client-rendered | 2026-09-19 23:20 |
| `https://midwesttiming.com/` + `/assets/index-DoqTOaas.js` | GET | 200 (3,045 B + 1,448,832 B) | React SPA over Supabase project `wkyzbfnhunrtyqjewpna`; public storage `results/**`; tables `events`, `results`, `schools`, `state_meet_documents` | 2026-09-19 23:20 |
| `…supabase.co/rest/v1/events?select=id,name,date,location,status,year,milesplit_link,pdf_link&year=eq.2026` | GET (public anon key from bundle) | 200 | structured meet metadata incl. `milesplit_link` and `pdf_link` (`…/results/2026/…Compiled Results.pdf`) | 2026-09-19 23:21 |
| `…/rest/v1/events?select=id&year=eq.2026` and `…&pdf_link=not.is.null` | GET | 200 (`content-range 0-0/336` and `0-0/279`) | **336** Kansas-area 2026 events, **279** with published compiled-results PDFs | 2026-09-19 23:21 |
| `…/rest/v1/state_meet_documents?select=doc_type,class,pdf_url` | GET | 200 (15 rows) | `performance`+`heatsheet` PDFs for classes 1A–6A plus `relay` and `wheelchair` | 2026-09-19 23:21 |
| `…/rest/v1/schools?select=*` | GET | 200 `[]` (count 0) | timer's school table not readable/empty via anon | 2026-09-19 23:21 |
| `https://wkyzbfnhunrtyqjewpna.supabase.co/storage/v1/object/public/results/state-meet/performance-1a.pdf` | GET (range 0-500) | 206, `application/pdf`, total 415,008 B | public result PDFs are anonymously fetchable | 2026-09-19 23:21 |
| `https://www.smelancers.com/` | GET (`-L`) | 200 → `https://www.gobound.com/ks/schools/shawneemissioneasths` | Kansas school athletics sites are hosted on Bound | 2026-09-19 23:22 |
| `https://www.gobound.com/ks/schools` | GET | 200 (675,337 B) | 543 Kansas school pages (309 `*hs`, 112 `*ms/jh`, 122 other) | 2026-09-19 23:22 |
| `https://www.gobound.com/direct/programs/h20260918033635217b68ae5ab080549/show` and `…/roster` | GET (`-L`) | 200 (45,020 B / 40,631 B) | per-program tabs Schedule/Practices/Roster/Staff/Stats; roster table header `Name | Year` but rows JS-loaded (empty at observation) | 2026-09-19 23:22 |
| `https://www.kansascoaches.com/` (KCA, from KSHSAA nav) | GET | **no HTTP status** — `curl` exit 28, connection timeout after 30 s | KCA coach directory unreachable from this machine | 2026-09-19 23:22 |
| `grep -i 'athletic\.net'` over 11 retrieved artifacts | local | n/a | **0** Athletic.net URL/ID references in KSHSAA, MileSplit-KS, DAT-KS, Midwest Timing or Bound Kansas pages | 2026-09-19 23:22 |
