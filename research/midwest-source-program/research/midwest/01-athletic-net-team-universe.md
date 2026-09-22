# 01. Athletic.net Midwest school/team universe

Status: complete
Observed on: 2026-09-19

Everything below is derived from (a) the two retained HAR captures of a live, signed-in Chromium
session against `www.athletic.net` (2026-09-18), (b) the read-only production repo
`/home/lewis/src/ad-law-scrape/athletic-rust-pipeline` (docs + captured-response fixtures), (c) the
Angular bundles fetched inside those HARs (`angular.athletic.net/app/site-app/*.js`, 2.8 MB + 2.5 MB,
extracted under `tools/a01/js1|js2/`), and (d) live probes run 2026-09-19 23:14–23:20 CDT.
The origin is Cloudflare-challenged to non-browser clients from this machine (§ Access), so every
request shape named below is quoted from a capture or from the shipped client code, never invented.

## Source

Athletic.net (`https://www.athletic.net`), the US high-school/college/club track-and-field and
cross-country results platform. Team-facing surfaces used by this report:

| Surface | Form | Evidence |
|---|---|---|
| Rankings JSON API (current) | `POST/GET https://www.athletic.net/api/v1/tfRankings/*` | HAR 2026-09-18 |
| Navigation/division API | `GET /api/v1/tfRankings/GetNavInfo`, `GET /api/v1/SiteHeader/GetDivChildren`, `GET /api/v1/SiteHeader/GetBreadcrumbs` | HAR 2026-09-18 |
| Team identity API | `GET /api/v1/TeamNav/Team?team=<TeamID>&sport=<tf\|xc>&season=<SeasonID>` | HAR 2026-09-18 |
| Public helpers | `GET /api/v1/public/GetStatesCountries2`, `GET /api/v1/Public/Seasons_Team?team=`, `GET /api/v1/Public/TeamLogo?teamId=&width=`, `GET /api/v1/SiteHeader/GetTreeTeam_Sport?sport=&SchoolID=` | HAR (first two: `Seasons_Team`/`TeamLogo`/`GetTreeTeam_Sport` are in the shipped JS only, not captured) |
| Ratings index (newer) | `GET /api/v1/tfRankings/GetNavInfoTFRI?divId=`, `GetDivInfoTFRI?divId=`, `GetEventsTFRI?gender=`, `GET /api2/tfri/checkpoints?divId=` | shipped JS `chunk-DTfnMQG9.js`-class TFRI service |
| Live results | `https://live.athletic.net/meets/<id>` | OHSAA state-coverage page, 2026-09-19 |
| Legacy pages still routed | `/TrackAndField/rankings/list/<divListId>/<gender>`, `/CrossCountry/rankings/list/<divListId>` | captured HTML + `getDivisionLink()` in JS |

## Coverage

- **School levels** are a bitmask, decoded from the shipped client and verified in two independent
  chunks: `chunk-DPPELSDu2.js` (level table with `mask`/`name`/`abbrev`/`forUrl`:
  `1 Unattached /unattached`, `2 Middle School /middle-school`, `4 High School /high-school`,
  `8 Collegiate /collegiate`, `16 Club /club`, `1024 Event Manager /event-manager`,
  `2048 State Association /state-association` — the abbreviation pipe merges the last two to `EM`/`AA`)
  and `chunk-Cj6mJ0S2.js` (`TeamLevel` enum:
  `Unattached=1, MiddleSchool=2, HighSchool=4, Collegiate=8, Club=16, EventManager=1024,
  StateAssociation=2048`; helper `teamIsEligibleToReceivePoints` = `level < 1024`). High-school
  entities carry `Level: 4` in every payload observed (`TeamNav/Team` → `"Level":4`;
  `BaseDiv.Level":4`; `GetRankings` → `"level":4`).
- **States**: the High-School level division context enumerates **52 state-level divisions**
  (50 states + DC + one entry with `state: null`) in the 2026 outdoor season — counts from
  `GetNavInfo` `.regions[]` (76 entries total: 1 level row + 52 state rows + California section rows
  + smaller sub-rows).
- **Sports/seasons**: `tfRankings` (outdoor TF, indoor TF) and `xcRankings` (XC) are separate API
  families (`getMeets()` in JS: `/api/v1/${sport!=='xc'?'tfRankings':'xcRankings'}/GetMeets`).
  **Indoor is the same TF family with `SeasonID = year + 10000`** — decoded twice independently:
  the shipped breadcrumb component (`t.includes('indoor') && (o += 1e4)`) and the season map below
  (`12026 → 175082`).
- **Historical depth**: the `GetNavInfo` season map in the Wisconsin context returns **22 outdoor
  seasons (2006–2027) and 21 indoor seasons (12007–12027)**, each with its own `divListId`. So the
  team/division universe is reconstructable per season back to 2006.
- **Platform totals** (`GET /api/v1/SiteFooter/GetFooterData`, no auth token needed, 117 B —
  `{"stats":{"results":222379541,"recentResults":3138,"athletes":22012295,"activeCoaches":77271,
  "meetsUploaded":479681}}` at 18:54Z; the second capture minutes later returns
  `222383228 / 7003 / 22012944 / 77266 / 479689`, which also demonstrates the counters are live).

## Enumeration

### The model: seasonal division lists form a tree; teams attach to individual nodes

`GetNavInfo` returns the *whole* navigation context for one division list. Captured for Wisconsin
2026 outdoor (`seasonId=2026`, `level=4`, `gender=m`, `indoor=false`), 78,468 B uncompressed:

```
GET https://www.athletic.net/api/v1/tfRankings/GetNavInfo
      ?seasonId=2026&level=4&gender=m&recordSetId=0&locationId=0&teamId=0&indoor=false
      headers: anettokens: <jwtTFTopReport from the preceding GetRankings response>
```

Response keys: `divListId, levels, levelDivId, seasons, countries, countryDivId, regions,
regionDivId, tree, events, events_main, recordSets`. Observed values:

- `divListId = 170770`, `levelDivId = 168416` (High School), `countryDivId = 167952` (United States).
- `seasons` — the **season → division-list** map for this context:
  `{"2026":170770, "2025":159762, "2024":146624, ..., "2006":815, "12026":175082, "12025":164008,
  ..., "12007":1404, "2027":181951, "12027":187407}`.
  Repo fixture `tests/fixtures/rankings/indoor-nav.json` shows the same map in the *national*
  context: `{"2026":168416,"12026":173005}` — i.e. `divListId == levelDivId` when the level itself is
  the context. **This is the single most useful primitive: 1 request yields every season's list id
  for a state.**
- `regions` — the level's children for the *queried season*: **76 rows** =
  52 `divType:"State"` rows (the 50 states + DC + `Overseas`, id 170162 — the only State row with
  `state: null`) + 10 California `Section` rows (Central, Central Coast, Los Angeles, North Coast,
  Northern, Oakland, Sac-Joaquin, San Diego, San Francisco, Southern) + 7 `Association` rows
  (NY: CHSAA/MMAA/NYSAIS/NYSPHSAA/PSAL; TX: TAIAO/TCAF) + 6 `League` rows (all TX: HCAL/SPC/TAPPS/
  TCAL/TCSAAL/UIL) + 1 `divType:"Level"` row (High School, `state: null`, id 168416). **None of the
  12 target Midwest states has association or league sub-regions** — they are all plain `State` rows
  whose `subDivType` is their own first grouping (table below).
- `tree` — the sub-division structure of the queried state for the queried season, exactly
  **68 nodes for Wisconsin 2026 outdoor**, with `depth` 0–3:
  depth 0: 1 — `Wisconsin` (170770, `subDivType:"Divisions"`);
  depth 1: 3 — `Division 1` (170771) / `Division 2` / `Division 3` (`subDivType:"Sectionals"`);
  depth 2: 16 — `Sectional 1…8` for D1, `Sectional 1…4` for D2, `Sectional 1…4` for D3 —
  **8/4/4** (`subDivType:"Schools"`);
  depth 3: 48 — `Regional 1A - Menomonie`, `Regional 1B - Marshfield`, `Regional 2A - Sauk Prairie`,
  … i.e. 3 regionals per sectional (`subDivType:""`).
  Node fields: `{id, name, state, divType, subDivType, depth, customDivision}`. Ids are allocated so
  that a section and its regionals interleave (`170772 Sectional 1`, `170773 Regional 1A`,
  `170774 Regional 1B`, `170775 Sectional 2`, …), which is why node ids are not usable as a
  sequential walk.
- `events` / `events_main` — the event catalog, 283 and 36 entries in this context
  (`{id, name, short, t, m, r, h, w, so, fmt}`), e.g. `{"id":1,"name":"100 Meters","short":"100m",
  "t":"T","m":"M"}`. `recordSets` is `null` here.

### Team listing under a division node

```
GET https://www.athletic.net/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170771
→ 200, 1,962 B uncompressed (456 B gzip), no anettokens header required
```
Observed body: `{"teams":[{IDDivision, DivName, LastMajor, IDSchool, SchoolName, Website}],
"divInfo":[{StateName, Country, IDBaseDiv, IDDivision, SeasonID, DivName, LastMajor, HomePage,
Website, DivIDDivision, DivDivName, Level}]}`. For `divId=170771` (Wisconsin `Division 1`) the
payload contained **one team** (`IDSchool 101718`, `SchoolName "Carmen Southgate"`, `Website null`)
and **8 child rows**, one per sectional (170772 Sectional 1 … 170793 Sectional 8) — note the child
rows repeat the *parent's* `IDDivision`/`DivName`/`IDBaseDiv`/`SeasonID`/`Level` and put the child's
own identity in the separate `DivIDDivision`/`DivDivName` fields, which is a trap for a naive parser.

Two consequences, both measured rather than assumed:
1. `teams[]` is **not** subtree-aggregated: a node with 8 children returned a single team, so a
   consumer cannot stop at the top of the tree. Teams also appear at non-leaf depth — the one team
   observed sits directly on `Division 1` (`Carmen Southgate`, a recent school id). Therefore the
   complete enumeration is the **full 68-node walk** (2 + 68 = 70 requests); the "walk only the 48
   regionals" shortcut would risk losing exactly this kind of team and is not recommended.
2. The team's own path is confirmed by two independent captures: `TeamNav/Team?team=3204` returns
   `divisions[United States 167952 → High School 168416 → Wisconsin 170770 → Division 1 170771 →
   Sectional 2 170775 → Regional 2A - Sauk Prairie 170776]`, and `General/GetRankings?athleteId=…`
   returns rows stamped `IDDivision: 170776`, `DivName "Regional 2A - Sauk Prairie"`,
   `SchoolID: 3204` — i.e. the id families in `divInfo.DivIDDivision` and in the ranking rows are the
   same season-scoped node ids.

**Recipe (per state, per sport, per season).** `/track-and-field-outdoor/usa/high-school/<state>`
lineage confirmed externally (OHSAA, SDHSAA) and internally (har referrer):

1. `GET /TrackAndField/rankings/list/<stateDivListId>/m` (or the division-home page) or
   `POST /api/v1/tfRankings/GetRankings` with the exact captured body (all 8 fields observed):
   `{"reportType":"div","mode":"list","divListId":<stateDivListId>,"indoor":null,"eventShort":"",
   "gender":"m","qualifyingListKey":"","version":2,"debug":""}` plus **`qParams`**, which is where
   every filter lives: `{}` (default = top 10 in each of 18 events, 180 rows),
   `{"eventType":0}` (event filter), `{"page":N}` (paging, **102 rows/page** on the event-filtered
   100 m list), `{"grades":[11]}` (server-side grade filter — see § Athlete evidence). → **the request
   carries no `anettokens`**; the response mints one as `jwtTFTopReport` (payload keys `divListId,
   reportType, meetId, userRoles, tfRankings, aud, iss, iat, nbf, exp`; token value not copied per the
   privacy contract) alongside `.division` (incl. `BaseDiv`, `HomeSettings`), `.level` and the rows in
   `.groupedRankings` (an array of arrays: one inner array per `EventShort`).
2. `GET /api/v1/tfRankings/GetNavInfo?seasonId=<SeasonID>&level=4&gender=<m|f>&recordSetId=0
   &locationId=0&teamId=0&indoor=<true|false>` with `anettokens: <jwtTFTopReport>` → `regions[]`,
   `tree` (N nodes), `seasons`, `events`.
3. For **every** tree node (all 68 for Wisconsin — see the two measured consequences above)
   `GET /api/v1/SiteHeader/GetDivChildren?sport=<tf|xc>&divId=<node.id>` → `teams[]`
   (`IDSchool` = TeamID, `SchoolName`, `Website`) — **no token, no gender parameter: one walk serves
   boys and girls.**
4. Optional identity enrichment per team: `GET /api/v1/TeamNav/Team?team=<TeamID>&sport=tf&season=2026`
   (1 request per team) → city/state/zip/mascot/level/capability flags.

Not enumerable from this surface (checked in the shipped client, not assumed): the custom-list
accessors are `listType ∈ {teamList, athleteList, meetList, meetSimulation}` (component
`shared-rankings-list-items`, branches `listType==='athleteList' → this.athletes = …`,
`listType==='meetList' → this.meets = …`, `listType==='teamList' → this.teams = …`), and the
team-list endpoints present in the bundles are
`/api/v1/tfRankings/GetTeams`, `/api/v1/tfRankings/GetAthletes`, `/api/v1/Meet/GetTeams`,
`/api/v1/TLog/GetAthletes` — i.e. **list-scoped, not state-roster** accessors; they require a list id
that only the custom-list UI mints. The public state-wide roster API is not in either capture: the
route chunks `team-home.routes-D9MAu-sB.js` and `division-home.routes-C9Tno7ix.js` are referenced by
the main bundle but were **never fetched** by the capture browser (0 hits in both HAR URL lists), so
the team-home/teams-tab request shapes remain unknown. The only roster URL found in shipped code is
coach-scoped: `/edit/team/<TeamID>/<sport-string>/<year>/athletes/roster/<m|f>/<athleteId>`.

### Per-state feasibility (12 target states)

`divListId` values are the High-School, 2026 outdoor, per-state division lists read out of
`GetNavInfo.regions[]` (capture 2026-09-18). Sub-level type is the state's own first sub-division
(`subDivType`). Cost is `2 + N` requests (2 fixed = one rankings/nav bootstrap + navinfo; `N` = tree
nodes, each an independent `GetDivChildren`), times sports (TF-outdoor, TF-indoor, XC are separate
trees), and it is **gender-free**.

| State | 2026 TF-outdoor HS `divListId` | First sub-level (`subDivType`) | Measured `N` | Cost/state/season | Feasibility |
|---|---|---|---|---|---|
| Wisconsin | 170770 | Divisions | **68** (verified) | **70** | High — measured end-to-end |
| Illinois | 169070 | Class | unknown | 2 + N | High — same mechanism; N returned by request 2 |
| Indiana | 169137 | Regions | unknown | 2 + N | High |
| Iowa | 169178 | Class | unknown | 2 + N | High |
| Kansas | 169215 | Associations | unknown | 2 + N | High |
| Michigan | 169407 | Peninsula | unknown | 2 + N | High |
| Minnesota | 169465 | Classes | unknown | 2 + N | High |
| Missouri | 169578 | Associations | unknown | 2 + N | High |
| Nebraska | 169689 | Divisions | unknown | 2 + N | High |
| North Dakota | 170039 | Classes | unknown | 2 + N | High |
| Ohio | 170050 | Divisions | unknown | 2 + N | High — state association itself publishes the equivalent page |
| South Dakota | 170271 | Classes | unknown | 2 + N | High |

`N` for the other 11 states is [UNVERIFIED] — not observable from this machine (§ Access) and not
present in the captures. The honest bound: the exact `N` is returned by request 2 of the same state,
so the total is knowable, not guessed, at a cost of 2 requests per state. Wisconsin's shape
(3 divisions → 16 sectionals → 48 regionals) suggests Midwest states land in the same order of
magnitude (tens, not hundreds, of nodes) but that is an inference, not a measurement.

Because `regions[]` is season-scoped, the same table for another season needs one `GetNavInfo` in
that season's context; the `seasons` map then yields that season's `divListId` per state.

### Where the state id itself comes from (bootstrap)

`GET /api/v1/public/GetStatesCountries2` (200, 34,159 B) returns `{states:[{Code,Name,CountryCode,
CountryCode3}] (65),countries:{…} (USA/CAN/NZL/…),countriesLookup:[{Alpha2,Alpha3,Name,WA3,WA_Name,
WA_Area}] (249)}` — state **codes and slugs only, no Athletic.net ids**. The id-bearing map is
`GetNavInfo.regions[]`. The old-style ranked page URL contains the id literally
(`/TrackAndField/rankings/list/170770/m`, captured 200), and the new human URL contains the state
slug (`/track-and-field-outdoor/usa/high-school/wisconsin`, seen as the referrer of that capture), so
the slug→id resolution happens in the division-home page, not in a JSON lookup.

## Stable identifiers

| Identifier | Where observed | Semantics | Stability |
|---|---|---|---|
| `TeamID` / `IDSchool` / `SchoolID` | `GetRankings` rows; `TeamNav/Team?team=`; `GetDivChildren.teams[].IDSchool`; `GetAthleteBioData.allTeams` (object key); `General/GetRankings.SchoolID` | one and the same integer namespace for a school-level team entity (3204 = Middleton verified in all five payloads) | Stable; URL-visible (`/team/3204/track-and-field-outdoor`) |
| `TeamCode` | `GetAthleteBioData.allTeams["<TeamID>"].TeamCode` (`"MIDD"`) | 4-letter team abbreviation | Not verified unique platform-wide; treat as an alias, not a key |
| `divListId` | `GetNavInfo.divListId`, `GetRankings` body, `/rankings/list/<id>` | season-scoped division list (e.g. 170770 = Wisconsin 2026 outdoor; 175082 = Wisconsin 2026 indoor) | Re-minted every season; **never** a permanent state key |
| `BaseDiv.ID` / `BaseDivID` | `TeamNav/Team.divisions[].b`; `SiteHeader/GetBreadcrumbs[].BaseDivID`; `GetRankings.division.BaseDiv.ID`; `GetDivChildren.divInfo[].IDBaseDiv` | season-independent division identity (Wisconsin = **638**, US = 79, HS level = 2, Division 1 = 709, Sectional 2 = 713, Regional 2A - Sauk Prairie = 1902; breadcrumb chain World = 1, USA = 79, High School = 2) | Stable across seasons — the correct key for "this state/division" |
| `IDDivision` (node) | `GetDivChildren.teams[].IDDivision`, `General/GetRankings[].IDDivision` | season-scoped node id of the exact division/regional the entity belongs to | Re-minted per season |
| `SeasonID` | `GetRankings`, `TeamNav`, `GetAgesGrades`, `allSeasons[].IDSeason` | `year` for TF-outdoor/XC, `year+10000` for indoor (2026, 12026) | Convention, verified twice |
| `AthleteID`, `MeetID`, `IDResult`, `EventID`+`EventTypeID`, `GradeID`, `IDCustomList` | `GetRankings` rows; bio `resultsTF[]`; `/athlete/<AthleteID>/<sport>` | athlete/meet/result/event/grade identities (see assignments 2–5 for full graphs) | Stable |
| `RegionID` | `BaseDiv.RegionID` (=58 for WI) | grouping region for the 52 states | Stable |
| School name, mascot, city | `TeamNav`, `GetRankings.TeamName` | **not** identifiers | one capture already shows why: 10 co-op names print as `A/B` (`Argyle/Pecatonica`, `Chetek/Weyerhaeuser`, `Cudahy/St.Francis`, `Deerfield/Cambridge`, `Elmwood/Plum City`, `Gresham Community/Bowler`, `Kickapoo/La Farge`, `Richland Center/Ithaca`, `Turtle Lake/Clayton`, `Wisconsin Heights/Barneveld`), two same-town schools disambiguate by parentheses (`Martin Luther (Greendale)`, `Valley Christian (Oshkosh)`), and `Pulaski` (53055) coexists with `Milwaukee-Pulaski` (2950) |
| `TeamID 0` (sentinel) | `GetRankings` rows from USATF/national meets | **not a school** — an unattached bucket whose `TeamName` carries free text (`Unattached`, `Wausau-WI`, `Milton-WI`) and whose `State` is `null` (4 rows in the capture, from `USATF Minnesota Summer Meet #3/#4` and `Nike Outdoor Nationals & USATF U20 Championships`) | Constant; a collector MUST filter `TeamID != 0` and must not filter state on these rows |

**TeamID structure (measured, Wisconsin):** 293 distinct WI teams extracted from the captured
`GetRankings` payloads. Range **2836–101206**; **270 of 293 (92%) inside 2836–3390**; 23 outliers
(19566, 20135, 20142, 20300–20305, 29322, 37689, 47088, 47158, 53055, 71373, 74032, 74402, 76323,
80845, 83745, 83759, 95903, 99606, 101206). The dense block is near-alphabetical by school name
(`2836 Two Rivers … 2879 Whitewater … 2994 South Milwaukee … 3390 …`), which is consistent with a
one-time alphabetical import; the outliers are late additions, and include **clubs and non-school
programs** (`Wings of Glory TC` 47158, `Stallions Track Club` 76323, `Milwaukee Speed Academy` 83759,
`Salty Springs Sprinters` 101206), homeschool programs (`Washington County Homeschool` 20142,
`Tri State Homeschool` 29322) and post-2000 schools (`Sun Prairie West` 83745). Therefore:
- do **not** infer a TeamID range for a state, and do not infer the entity type from the id —
  always read `Level` (4 = HS) from `TeamNav/Team`;
- ids are allocated globally and chronologically and are not gender-scoped: one TeamID per school,
  boys and girls share it (`GetDivChildren.teams[]` has no gender field; `divisions[].gender = "x"`;
  gender is a parameter of rankings/reports only).
- `TeamNav/Team` also returns `linkedTeams[]` (`{IDSchool, SchoolName, Level}`) — feeder middle
  schools for the same site (Middleton → `Glacier Creek` 57750, `Kromrey` 57810, both Level 2) — and
  `customDivisions[]` for conferences (`Big Eight` 172497).

**Team identity fields available in one request** (`TeamNav/Team?team=3204&sport=tf&season=2026`,
1,222 B): `team{ID, Name:"Middleton", Level:4, hasIndoor:true, Address:"2100 Bristol St",
City:"Middleton", State:"WI", ZipCode:"53562-2699", Country:"USA", Mascot:"Cardinals", MascotUrl,
TeamRecords:1, siteSupport:0, colors:["Cardinal","White",null], coverPhoto}` plus `grades`
(`IDGrade` 9/10/11/12), `divisions[]`, `customLists`, `customDivisions[]`, `linkedTeams[]`.

## Athletic.net leverage

This report is about Athletic.net's *own* universe, so "leverage" means: how much the team universe
removes cost from the acquisition flow, and which official sources already point at it.

**Official state-association pages that link into Athletic.net (checked 2026-09-19, one or two GETs
per host, no bypass):**

| State | Official page | What it exposes | TeamIDs? |
|---|---|---|---|
| OH | `ohsaa.org/Sports-Tournaments/Track-Field/Athleticnet-How-To-Usage-Policy` | Anchor **"OHSAA Approved High School Team Pages"** → `https://www.athletic.net/track-and-field-outdoor/usa/high-school/ohio`; **"…Middle School Team Pages"** → `…/usa/middle-school/ohio`; `support.athletic.net`, `live.athletic.net/about` | No — state page, not per-school ids |
| OH | `ohsaa.org/Sports-Tournaments/Track-Field/2026-Track-and-Field/2026-Track-Field-State-Coverage` | `www.athletic.net/TrackAndField/meet/656920/info` ("Athletic.net Meet Page"), `…/meet/656920/entries` ("Qualifiers List"), `live.athletic.net/meets/74520` ("Athletic.net Live Results") | No (MeetID 656920) |
| MI | `mhsaa.com/sports/boys-track-field` | MHSAA 2026 finals: MeetIDs **622966** (D1), **622969** (D2), **622972** (D3), **622973** (D4), **622974** (UP), each with `/results/all` and `/teamscores` | No (5 MeetIDs) |
| NE | `nsaahome.org/track-field/` | 28 links `www.athletic.net/TrackAndField/meet/<id>/results/all` labelled by host site (e.g. 645702 Fremont HS … 645756 Bayard Public Schools, 662930 Elkhorn HS) | No (28 MeetIDs) |
| SD | `sdhsaa.com/activity/track-field/` and the SDHSAA XC article | `www.athletic.net/track-and-field-outdoor/usa/high-school/south-dakota`; XC page adds `…/cross-country/usa/high-school/south-dakota` ("Rosters & Schedules") and `…/events/usa/south-dakota;sport=2` ("Results") | No |
| ND | `ndhsaa.com/athletics/track-boys` | **no link**, but the page serves *"Athletic.net spreadsheet for manual entry of state qualifiers"* (`d2q0tptsfejku7.cloudfront.net/.../Athletic-Net-Custom-Format-Handheld-TR.xlsx`) and a "Register for a meet" video — NDHSAA feeds Athletic.net by hand rather than linking it | No |
| WI, MN, IA, IL, IN, MO, KS | WIAA (wiaa.com), MSHSL, IHSAA/IGHSAU, IHSA, IHSAA-IN, MSHSAA, KSHSAA | **no** athletic.net links found on the sports pages probed; these associations point at their own or other platforms (KSHSAA → `https://ks.milesplit.com/meets/751194-kshsaa-state-championships-2026/entries`; MSHSL → `results.wayzatatiming.com/meets/74652`; IAHSAA/IGHSAU → `gobound.com/ia/…`) | No |

No US Midwest state association publishes Athletic.net **TeamIDs**; the closest artefact is Ohio's
"Approved Team Pages", which is the Athletic.net state-level division page — i.e. the same surface
this report qualifies. Meet IDs, by contrast, are published outright by OH/MI/NE/SD.

**Request savings (measured + arithmetic, WI-calibrated):**
- Complete school/team universe for a state, one sport-season: **70 requests** (2 + 68) — the whole
  tree must be walked because `teams[]` is not subtree-aggregated (§ Enumeration). One walk covers
  boys and girls, and the heaviest two calls (`GetRankings` 16.7–191 KB + `GetNavInfo` 78 KB) are
  per-state, not per-team.
- The same 293 Wisconsin teams that the existing rankings path observed were only visible *because*
  707 athlete rows / 668 distinct athletes happened to surface them (and 668 is itself a lower bound:
  the capture only covered 18 event families — a top-10-per-event page plus paged, grade-filtered
  100 m). Seeding from the tree therefore replaces hundreds-to-thousands of incidental profile
  requests with 70 deliberate ones, and is complete where the rankings path is not (a team with no
  qualifying mark never appears in a ranking).
- 12 states × TF-outdoor 2026 ≈ **840 requests** at the measured Wisconsin shape (70 × 12, both
  genders included). A second pass for XC (and one for indoor, `SeasonID = year + 10000`) roughly
  doubles that; each is one season's re-mint, not a re-derivation, and the `seasons` map makes the
  next season a 1-request discovery per state.
- Per-team identity enrichment (`TeamNav/Team`) costs 1 request/school, so a full Midwest identity
  refresh is on the order of thousands of requests — but it is optional: it is only needed for the
  school-identity fields (city/state/mascot/level/feeder links), not to obtain ids.
- The alternative — resolving schools against Athletic.net by name — needs ≥1 search/navigation
  request per school and is not deterministic (`Pulaski` 53055 vs `Milwaukee-Pulaski` 2950 in the
  same state). For Wisconsin that is ≥293 requests to *guess* what 70 requests *prove*.

**Downstream seam.** `GetAthleteBioData` returns `allTeams` as an **object keyed by TeamID string**
(`{"3204":{"Level":4,"IDSchool":3204,"SchoolName":"Middleton","TeamCode":"MIDD","MascotUrl":…,
"PrefMetric":0,"PrefConvert":1,"Year":2026}}`), `allSeasons[]` as
`{SchoolID, IDSeason, Display, Selected, age}` and `grades` as
`{"<SchoolID>_<SeasonID>": <grade>}` (verified: `{"3204_2025":10,"3204_2026":11}`). So any athlete
already collected projects onto this TeamID universe with zero extra team requests — the team
universe is the join table, and the roster step is the only piece still missing (§ Recommendation).
The same payload also exposes `athlete{IDAthlete, FirstName, LastName, Gender, Handle, SchoolID,
UsatfId, isClaimed, NoSearch, PhotoUrl, age}` plus `synonyms`, `relayTeamMembers` and
`unattachedCalendars` — cross-identifier material for assignment 2, noted here only because it arrives
with the TeamID join.

## Athlete evidence

The team universe itself yields no athlete rows; it defines the container. What it *does* provide for
athlete work, all verified in the captures:

- **Team↔athlete join keys**: `AthleteID` + `TeamID/SchoolID` co-occur in every `GetRankings` row
  (`TeamID`, `TeamName`, `TeamMascot`, `State`, `Country`) and in `GetAthleteBioData.resultsTF[]`
  (`SchoolID` per historical result) — so school history per athlete is a projection of these two
  payloads, with the season stamped (`SeasonID`, `ResultDate`).
- **Grade/class signal**: `GradeID` in rankings rows and `grades:{"3204_2026":11}` in the bio.
  `GetAgesGrades` (POST, body
  `{recordSetId, locationId, teamId, listId, seasonId, restrict, includeEvents[{eventId,eventTypeId}]}`)
  returns `grades:[{ID:6..24, GradeCount}]` and `ages:[{Age, AgeCount}]` for the context; the
  Wisconsin 2026 capture counted `Grade 11: 19,725`, `Grade 10: 21,535`, `Grade 9: 22,934`,
  `Grade 12: 16,436`.
- **Server-side grade filter (Class-of-2027 relevant, verified)**: `qParams:{"grades":[11]}` on
  `GetRankings` restricts a division report to grade 11. Census over the captured pages: default
  page = 180 rows (18 `EventShort` groups × 10) with grade mix `{10:11, 11:42, 12:87, 99:40}`;
  the four grade-11-filtered 100 m pages returned 323 rows / **323 distinct athletes** (228 distinct
  meets) with grade mix `{11:322, 12:1}` — i.e. the filter is near-tight but **not airtight** (one
  GradeID-12 row appeared on page 10 of a `[11]` filter), so a consumer must still re-check `GradeID`
  and must expect `99` relay placeholders on unfiltered pages. This is a per-state, per-gender way to
  enumerate the target class without touching athlete profiles, and every row still carries `TeamID`,
  so it doubles as team discovery.
- Whole-capture census for the Wisconsin 2026 boys context (7 retained bodies): **707 rows**,
  705 distinct `IDResult`, **668 distinct `AthleteID`**, **294 distinct `TeamID`** (293 with
  `State:"WI"`, 1 belonging to the 4 rows whose `State` is null), 18 `EventShort`, 314 distinct
  `MeetID`; grade mix `{11:416, 12:193, 10:48, 99:40, 9:10}` — grade 11 is the modal grade in this
  sample (`wi_teams_observed.json` reproduces the 293-id set exactly).
- **Team page → roster** is the missing athlete-enumeration seam (chunk not captured; the only roster
  URL in shipped code is coach-scoped, see § Enumeration).
- Per-team season inventory is a public call: `GET /api/v1/Public/Seasons_Team?team=<TeamID>` →
  `{tf:[…Indoor flag…], xc:[…]}` (shipped `getSeasons()`), which answers "which seasons does this
  school actually contest" before spending any rankings request.

## Recruiting information

Not applicable to the team entity: `TeamNav/Team` exposes no coach, AD, email or website field
(coach data appears on the team *page*, which needs the uncaptured team-home chunk; the coach-side
API names visible in shipped code are `TeamCode/CoachPositions`, `TeamCode/AddChildrenToTeam`,
`manage-coaches` — all authenticated). The only public recruiting-adjacent signal on the team entity
is `siteSupport` and `Website` (null for the one school captured). Coach claiming/roster policy is
documented publicly by OHSAA (Google Doc `1R1BDB3GUFNVgZXmkITGZOiESmvIkKDb0Tbv3axblGdQ`, exported
2026-09-19): rosters are locked to grades 9–12 with 7/8 for middle school, and coach accounts are
free.

## Result evidence

Not applicable at team level (see assignment 3/5 for meet/result graphs). Two team-scoped result
seams are confirmed here because they carry TeamIDs: `POST /api/v1/Meet/GetTeamScoreData` (per-meet
team scores; the public page is `/TrackAndField/meet/<MeetID>/teamscores`, published by MHSAA) and
the `GetRankings` row schema itself — `IDResult, AthleteID, GradeID, EventID, EventType, EventShort,
EventTypeID, TeamID, TeamName, TeamMascot, State, Country, MeetID, MeetName, Round, FAT, Measure,
display, SortIntRaw/SortIntCalc/SortIntOrig, PersonalBest, MediaCountJson, ResultDate, SeasonID,
rank, rowNum, isFieldSeries`.

## Incremental use

Incremental design is constrained by a measured fact: **every API response observed carries
`cache-control: no-store, no-cache, max-age=0, private`, `cf-cache-status: DYNAMIC`, and no
`ETag`/`Last-Modified`** (`TeamNav/Team`, `GetDivChildren`, `GetNavInfo`, `GetRankings`). There is no
HTTP-level conditional revalidation, so a weekly collector must diff content digests.

Weekly plan that avoids re-fetching history:

1. **Season rollover only** (once per season, per sport): 1 `GetNavInfo` per state → new
   `divListId`s from the `seasons` map + new `tree` digest. Seasons are the only thing that
   invalidates ids, and the map hands them over in a single request.
2. **Weekly**: 1 `GetNavInfo` per state (78 KB, 2 requests incl. the bootstrap) and compare the
   `tree` digest. Unchanged digest ⇒ no team-list work at all. Changed ⇒ re-walk only the changed
   subtrees with `GetDivChildren` (node ids are in the tree, so the blast radius is explicit).
3. `TeamNav/Team` refresh is per-team and optional; where the pipeline already holds an athlete's
   `allTeams`/`grades` projection, no team request is needed for that school-season.
4. New *teams* (new schools, new clubs) appear as new nodes/rows in `GetDivChildren`; the
   `HomeSettings.teamsSettings.uncategorized:"withButton"` knob means uncategorized teams are
   presented as a drill-in group rather than mixed into the state list, so "unknown" teams should be
   harvested from the state node as well as the leaves.

## Access characteristics

- **Class**: browser application + public structured JSON behind a Cloudflare managed challenge.
  Endpoints are undocumented publicly (`support.athletic.net` is coach documentation; there is no
  public API doc).
- **Observed block (2026-09-19 23:15–23:23 CDT, `curl`, no special headers, 9 requests total to
  the host across two probe rounds)**: every non-`robots.txt` path returned **HTTP/2 403** with
  `cf-mitigated: challenge`, `server: cloudflare`, a nonce-bearing CSP and
  `<title>Just a moment...` — i.e. the managed-challenge interstitial, *not* an origin refusal.
  Bodies saved under `tools/a01/block*/`:

  | Path probed | Round 1 (23:15–23:20) | Round 2 (23:23, re-probe) | sha256 (round-1 body) |
  |---|---|---|---|
  | `/` | 403, 5,343 B | 403, 5,343 B | `ab410084a9bff6696993cd93b0ad335eededdcb49a7740b1aef4e68c4a50d070` |
  | `/api/v1/public/GetStatesCountries2` | 403, 5,484 B | 403, 5,484 B | (round-1 body not retained; size reproduced) |
  | `/api/v1/TeamNav/Team?team=3204&sport=tf&season=2026` | 403, 5,602 B | 403, 5,602 B | `e420af6b8887631b5ce0c23273323c8f2e7d320618122b87c24c7fe5de92f487` |
  | `/api/v1/tfRankings/GetTeams?listId=170770` | 403, 5,542 B | 403, 5,542 B | `d9c4fe25137d3ac9c33a2e2807995a0fe40ec3a034e8328886ca1d80e5786496` |

  Body **size** is reproducible across rounds; the **hash is not** (the page embeds a per-request
  nonce — round 2's four hashes differ from round 1's while sizes match), so treat size/`cf-mitigated`
  as the stable signature and the sha256 as a specimen fingerprint for the request recorded in the
  appendix. Note the interstitial is `Just a moment...`, which differs from the repo's earlier
  `Attention Required!` observation — the challenge form has changed since that note was written.
- `https://www.athletic.net/robots.txt` → **200**, 1,080 B, `cf-cache-status: HIT`,
  `cache-control: max-age=31536000`, `age: 234272`, `x-powered-by: ASP.NET`,
  `last-modified: Sun, 15 Mar 2026 19:18:17 GMT`; **no `Sitemap:` line** (byte-identical across the
  two rounds). It disallows `/Admin/ /admin/ /Account/ /User/ /Edit/ /Partner/ /Partner/PrepSportswear/
  /post/ /Secure/ /home/ /search.aspx` (all case variants), `/TrackAndField/State/`,
  `/CrossCountry/State/`, `/trackandfield/print/`, `/trackandfield/report/`,
  `/trackandfield/school_new.aspx`, `/trackandfield/meet/teamresults.aspx`,
  `/crosscountry/division.aspx`, `/crosscountry/results/{team,season}.aspx`,
  `/CrossCountry/Results/CourseHistory.aspx`, plus `MSIECrawler` and `AhrefsBot` fully. Note that
  `robots.txt` does **not** disallow any `/api/v1/...` path, and nothing in it blocks team or
  division pages.
- **Authentication surface** (measured header-by-header across all 889 captured requests; **no cookie
  or token value is reproduced here**): the capture session was signed in, yet `anettokens` — a
  319-char context JWT whose payload keys are
  `aud, divListId, exp, iat, iss, meetId, nbf, reportType, tfRankings, userRoles` — was presented on
  only **three** endpoints: `tfRankings/GetNavInfo` (GET, ×2), `tfRankings/GetAgesGrades` (POST, ×2)
  and `tfRankings/GetStandards` (GET, ×8). Everyone else goes **token-free**:
  `tfRankings/GetRankings` (the request that *mints* the token — all 9 captures carry no `anettokens`,
  and each retained response body contains exactly one JWT, the `jwtTFTopReport` field, 319 chars),
  `SiteHeader/GetDivChildren`, `TeamNav/Team`, `public/GetStatesCountries2`,
  `SiteFooter/GetFooterData`, `SiteHeader/GetBreadcrumbs`, `AthleteBio/GetAthleteBioData`,
  `General/GetRankings`. Minting chain verified numerically: 7 distinct body-minted tokens, 9 distinct
  header-presented tokens, overlap **7/7** — the 2 remainder belong to the two `GetRankings` responses
  whose bodies the HAR dropped. Practical consequence: **the team walk does not need the context
  token at all**, and the rankings POST that produces it does not need to send one.
  Whether the origin *requires* `anet-site-roles-token` / `anet-appinfo` (present on every request) or
  the `anettokens` on the nav endpoints cannot be established while the Cloudflare challenge stands.
- **Rate limits**: none published; none observed. No `429`, no `Retry-After` in either capture. The
  JSON is gzip-compressed (`GetDivChildren` 456 B over the wire vs 1,962 B body), which keeps the
  tree walk cheap. The harness politeness rule (≤~50 requests/host) is respected by design here:
  this report issued **9 requests to `www.athletic.net`** (5 in round 1, 4 in round 2, sequential,
  1 s apart) and 1–3 per association host.

## Recommendation

**ATHLETIC.NET-SEED** for the school/team layer (this is Athletic.net's own universe, so it is the
one "source" in this program that cannot reduce Athletic.net traffic by itself — it exists to make
every *other* source's school names resolve to ids deterministically).

Expected marginal value:

- One 2-request-per-state, ~70-request-per-state-sport-season walk converts any third-party school
  list (WIAA/MSHSL/IHSA/MHSAA/…) into `TeamID`s with city/state/level, replacing name-based searching
  and giving the upstream sources deterministic seeds.
- It closes the "team not in rankings" gap: teams with no qualifying mark or no event inside the
  95-family manifest simply do not exist in the rankings path, but they do exist in the tree.
- It yields `BaseDiv.ID` (stable) alongside the seasonal `divListId`, letting a collector survive
  season rollover with no rediscovery of state identity.
- Before relying on it: capture the two uncaptured chunks (`division-home.routes-*.js`,
  `team-home.routes-*.js`) with the existing headed profile and record the teams-tab and
  public-roster request/response shapes (both are the only remaining unknowns in this report). Until
  then, the roster step (team → athletes) must stay on the existing profile path, and the tree walk
  should be scheduled once per season per state with weekly digest checks.

### Evidence appendix

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.athletic.net/` | GET | 403 (`cf-mitigated: challenge`, 5,343 B; size reproduced in both rounds; sha256 ab410084… for the round-1 body) | origin is Cloudflare-challenged to non-browser clients from this machine; interstitial is `Just a moment...` | 2026-09-19 23:16 + 23:23 CDT |
| `https://www.athletic.net/api/v1/public/GetStatesCountries2` | GET | 403 (5,484 B both rounds) | the challenge covers the JSON API, not only HTML | 2026-09-19 23:16 + 23:23 CDT |
| `https://www.athletic.net/api/v1/TeamNav/Team?team=3204&sport=tf&season=2026` | GET | 403 (5,602 B; sha256 e420af6b… for the round-1 body) | as above, for the team endpoint | 2026-09-19 23:15 + 23:23 CDT |
| `https://www.athletic.net/api/v1/tfRankings/GetTeams?listId=170770` | GET | 403 (`cf-mitigated: challenge`, 5,542 B, sha256 d9c4fe25… for the round-1 body) | as above; also the point at which the "list id" accessor was probed | 2026-09-19 23:20 + 23:23 CDT |
| `https://www.athletic.net/robots.txt` | GET | 200 (1,080 B, `cf-cache-status: HIT`, `age: 234272`, `last-modified: Sun, 15 Mar 2026 19:18:17 GMT`, no `Sitemap:`; byte-identical in both rounds) | crawl policy; no sitemap shortcut for school enumeration; nothing under `/api/` is disallowed | 2026-09-19 23:15 + 23:23 CDT |
| `POST /api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m&recordSetId=0&locationId=0&teamId=0&indoor=false` | POST(captured, sent as GET by app) | 200, 78,468 B | `regions[]` = 76 rows (52 State incl. DC + `Overseas`, 10 CA sections, 7 NY/TX associations, 6 TX leagues, 1 HS level row) with ids for that season; 68-node Wisconsin `tree` (1/3/16/48); `seasons` map 2006–2027 outdoor + 12007–12027 indoor; 283 `events`, 36 `events_main` | 2026-09-18T18:54Z (HAR1) |
| `GET /api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170771` | GET | 200, 1,962 B | `teams[]` = `{IDDivision, DivName, LastMajor, IDSchool, SchoolName, Website}`; one directly-attached team for D1; 8 sectional children in `divInfo` | 2026-09-18T19:21Z (HAR2) |
| `GET /api/v1/TeamNav/Team?team=3204&sport=tf&season=2026` | GET | 200, 1,222 B | TeamID→school identity: Middleton, WI, Level 4, hasIndoor, address/city/zip/mascot/colors, grades 9–12, full division path, linked feeder schools, conference custom division | 2026-09-18T19:21Z (HAR2) |
| `GET /api/v1/SiteHeader/GetBreadcrumbs?sport=tf&athleteId=0&meetId=0&calendarId=0&divId=170770&teamId=0&seasonId=0` | GET | 200, 1,456 B | breadcrumb hierarchy with `id`+`url`+`BaseDivID` per level (World 166948 / USA 167952 / HS 168416 / Wisconsin 170770) | 2026-09-18T18:54Z (HAR1) |
| `POST /api/v1/tfRankings/GetRankings` body `{"reportType":"div","mode":"list","divListId":170770,"indoor":null,"eventShort":"","gender":"m",…,"version":2}` | POST | 200 (9 captures, 16.7 KB–191 KB) | rankings row schema with `TeamID/TeamName/TeamMascot/State/Country/GradeID/MeetID/IDResult`; division metadata incl. `BaseDiv{ID:638,State:"WI",Level:4,hasXC,hasTrack,featureIndoor}` and `HomeSettings.sections[…"teams"…]`, `teamsSettings.depth:3` | 2026-09-18T18:54–18:55Z (HAR1) |
| `GET /api/v1/General/GetRankings?athleteId=28872883&sport=tf&seasonId=2026&truncate=true` | GET | 200, 6,122 B | rows stamped `IDDivision:170776` ("Regional 2A - Sauk Prairie") + `SchoolID:3204`; confirms TeamID=SchoolID and that teams are carried at season-scoped node depth | 2026-09-18T19:21Z (HAR2) |
| `GET /api/v1/AthleteBio/GetAthleteBioData?athleteId=28872883&sport=tf&level=0` | GET | 200, 30,344 B | `allTeams[{key,value:{Level,IDSchool,SchoolName,TeamCode,MascotUrl,Year}}]`, `allSeasons[{SchoolID,IDSeason,Display,Selected}]`, `grades:{"3204_2025":10,"3204_2026":11}`, `resultsTF[].SchoolID` | 2026-09-18T19:21Z (HAR2) |
| `GET /api/v1/public/GetStatesCountries2` | GET | 200, 34,159 B | state/country code lists (65 states incl. CA provinces, 249 country lookups) — **no Athletic.net ids** | 2026-09-18T18:54Z (HAR1) |
| `GET /api/v1/SiteFooter/GetFooterData` | GET | 200, 117 B | `{"stats":{"results":222379541,"recentResults":3138,"athletes":22012295,"activeCoaches":77271,"meetsUploaded":479681}}`; the second capture returns higher counters (222,383,228 / 7,003) | 2026-09-18T18:54Z + 19:20Z (HAR1/HAR2) |
| `POST /api/v1/tfRankings/GetAgesGrades` body `{"recordSetId":0,"locationId":0,"teamId":0,"listId":0,"seasonId":2026,"restrict":false,"includeEvents":[…]}` | POST | 200, 1,080 B | grade/age universes with counts: `{9:22934, 10:21535, 11:19725, 12:16436}` + grades 6–8 and 21–24, `ages`: 31 entries | 2026-09-18T18:54Z (HAR1) |
| *(jq over both HARs)* all `request.headers` where `name==anettokens`, cross-checked against JWTs found in `tfRankings/GetRankings` response bodies | n/a | n/a | `anettokens` (319-char JWT; payload keys `aud,divListId,exp,iat,iss,meetId,nbf,reportType,tfRankings,userRoles`) is presented **only** by `GetNavInfo`/`GetAgesGrades`/`GetStandards`; `GetRankings` mints it and sends none; 7 body-minted tokens vs 9 header-presented, overlap 7/7 (2 bodies not retained) | 2026-09-19 23:24–23:30 CDT |
| *(jq over the 7 retained `GetRankings` bodies)* `groupedRankings` census + `qParams` variants | n/a | n/a | 707 rows / 668 distinct athletes / 294 distinct TeamID / 18 events / 314 meets; `qParams` carries `{page:N}` (102 rows/page), `{eventType:0}` and `{grades:[11]}`; the four grade-11-filtered pages give 323 rows and 323 distinct athletes with grade mix `{11:322, 12:1}` | 2026-09-19 23:30–23:40 CDT |
| *(grep over the captured Angular bundles, `tools/a01/js1`, `js2`)* `TeamLevel` enum + level table in `chunk-Cj6mJ0S2.js` / `chunk-DPPELSDu2.js`; endpoint literals; `1e4` indoor rule | n/a | n/a | bitmask `1/2/4/8/16/1024/2048` with names+slugs; `TeamLevel.StateAssociation=2048`; indoor `SeasonID = year + 1e4` (`track-and-field-indoor ? 1e4 : 0`) + `t.includes('indoor') && (o += 1e4)`; endpoint literals `Public/Seasons_Team`, `Public/Seasons_TeamReports`, `Public/TeamLogo?teamId=`, `SiteHeader/GetTreeTeam_Sport`, `tfRankings/GetTeams`, `tfRankings/GetAthletes`, `Meet/GetTeams`, `TLog/GetAthletes`; `team-home.routes-D9MAu-sB.js` and `division-home.routes-C9Tno7ix.js` referenced but never fetched | 2026-09-19 23:24–23:40 CDT |
| `https://www.ohsaa.org/Sports-Tournaments/Track-Field/Athleticnet-How-To-Usage-Policy` | GET | 200, 53,888 B | OHSAA 2026 Athletic.net tournament-entry transition; anchors "OHSAA Approved High School Team Pages" → `…/usa/high-school/ohio`, "…Middle School Team Pages" → `…/usa/middle-school/ohio` | 2026-09-19 23:13 CDT |
| `https://docs.google.com/document/d/1R1BDB3GUFNVgZXmkITGZOiESmvIkKDb0Tbv3axblGdQ/export?format=txt` | GET | 200, 17,193 B | OHSAA policy guide: approved team pages are the Athletic.net state pages; rosters locked to grades 9–12; all OHSAA tournament events hosted on Athletic.net; qualifier applications require Meet ID | 2026-09-19 23:14 CDT |
| `https://www.ohsaa.org/Sports-Tournaments/Track-Field/2026-Track-and-Field/2026-Track-Field-State-Coverage` | GET | 200, 75,868 B | OHSAA 2026 state meet MeetID 656920 (`/info`, `/entries`) + `live.athletic.net/meets/74520` | 2026-09-19 23:13 CDT |
| `https://www.mhsaa.com/sports/boys-track-field` | GET | 200, 168,411 B | 5 MHSAA 2026 finals MeetIDs linked directly (622966/622969/622972/622973/622974) with `/results/all` and `/teamscores` | 2026-09-19 23:13 CDT |
| `https://nsaahome.org/track-field/` | GET | 200, 265,118 B | 28 NSAA meet links `meet/<id>/results/all` labelled by host site | 2026-09-19 23:13 CDT |
| `https://sdhsaa.com/activity/track-field/` | GET | 200, 510,483 B | SDHSAA links `athletic.net/track-and-field-outdoor/usa/high-school/south-dakota` | 2026-09-19 23:13 CDT |
| `https://www.sdhsaa.com/` | GET | 200, 1,096,381 B | SDHSAA XC article links `…/cross-country/usa/high-school/south-dakota` ("Rosters & Schedules") + `…/events/usa/south-dakota;sport=2` ("Results") | 2026-09-19 23:12 CDT |
| `https://www.wiaa.com/`, `wiaawi.org`, `mshsl.org`, `iahsaa.org`, `ighsau.org`, `ihsa.org`, `mhsaa.com`, `ihsaa.org`, `ohsaa.org`, `mshsaa.org`, `kshsaa.org`, `nsaahome.org`, `ndhsaa.com` (homepages) | GET | all 200 | association baselines; only SDHSAA referenced athletic.net at root; note `nsaahome.org` answered 200 to a normal GET from this machine (curl-default-UA 403 recorded in the mission brief is UA-dependent) | 2026-09-19 23:12 CDT |
| `https://www.ohsaa.org/sports/track`, `mhsaa.com/sports/boys-track-field`, `mshsl.org/sports-and-activities/track-and-field-boys`, `iahsaa.org/track-field/`, `ighsau.org/sports/track-field`, `ihsa.org/assets/main-B2rKN2jy.js`, `ihsaa.org/sports/boys/track-field`, `mshsaa.org/sitemap.xml` (44 B), `kshsaa.org/react/assets/index-f33aafe3.js`, `nsaahome.org/track-field/`, `ndhsaa.com/athletics/track-boys`, `sdhsaa.com/activity/track-field/`, `wiaa.com/tournament-xch/?sportid=22`, `wiaawi.org/sports/boys-track-field` | GET | all 200 | sport-page sweep for athletic.net links: hits only on MHSAA (13 links / 5 MeetIDs) and NSAA (28 links); KSHSAA links `ks.milesplit.com/meets/751194-kshsaa-state-championships-2026/entries`; MSHSL links `results.wayzatatiming.com`; IA links `gobound.com` | 2026-09-19 23:13 CDT |
| `https://www.mhsaa.com/schools` | GET | 200, 145,587 B | MHSAA school directory renders school links client-side; no static school→athletic.net mapping available | 2026-09-19 23:18 CDT |
| HAR-internal assets: `angular.athletic.net/app/site-app/main-M36VMDMH.js` (+ ~150 chunks, 5.3 MB) | GET | 200 (captured) | shipped client code: route table, `GetDivChildren`/`TeamNav`/`GetTeams` call sites with parameters, level bitmask table, `SeasonID = year+1e4` indoor rule, team URL builders, `team-home.routes-*`/`division-home.routes-*` referenced but never fetched | 2026-09-18 (HAR1/HAR2) |
| `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/{README.md,SCOPE.md,CHROMIUM_DESIGN.md,HANDOFF.md}`, `tests/fixtures/rankings/*.json`, `fixtures/public/athletic-source-contract.json`, `src/runtime/profile_worker/team.rs` | read | n/a | production contract: division `168416` outdoor / `173005` indoor; `allTeams`→`TeamRequest{team_id,sport,season}`; live-source blocked by Cloudflare 2026-09-19 with retained block bodies | 2026-09-19 23:00–23:10 CDT |
