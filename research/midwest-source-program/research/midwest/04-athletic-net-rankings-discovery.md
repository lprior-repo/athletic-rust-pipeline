# 04. Athletic.net rankings discovery

Status: complete — capture/repo/retained-delivery evidence only; every live re-test from this machine
returned Cloudflare `403` (2 probes, recorded in Access characteristics). No access-control mechanism
was attempted.
Observed on: 2026-09-19

Scope of this report: what the Athletic.net **rankings list** can enumerate, what it costs, what it
omits, and what one ranking row yields. All live Athletic.net evidence is the two browser HAR captures
(2026-09-18) plus the repo's captured fixtures; all cost figures are anchored to the retained
grade-11 delivery workbook (2026-09-18).

---

### Source

Athletic.net, Track & Field / Cross Country rankings.

| Surface | URL | Evidence |
|---|---|---|
| Rankings list page | `https://www.athletic.net/TrackAndField/rankings/list/<divListId>/<gender>[/<eventShort>]` | HAR1[0] URL; `tf-rankings.routes` JS defines exactly these three routes |
| XC rankings page | `https://www.athletic.net/CrossCountry/rankings/list/<IDDivision>/<gender>/<Meters>?restrict=true` | HAR2[69] `athlete-bio.routes` template (`ZS(``, prefix, '/CrossCountry/rankings/list/', e.IDDivision, '/', genderParam, '/', e.Meters, '?restrict=true')`) |
| Rankings data API (TF) | `POST https://www.athletic.net/api/v1/tfRankings/GetRankings` | HAR1[147],[348],[414],[478],[539],[587],[622],[656],[704] |
| Navigation/event catalog | `GET /api/v1/tfRankings/GetNavInfo?seasonId=&level=&gender=&recordSetId=&locationId=&teamId=&indoor=` | HAR1[154],[354]; service code HAR1[138] |
| Grade/age counts | `POST /api/v1/tfRankings/GetAgesGrades` | HAR1[153],[353] |
| Qualifying standards | `GET /api/v1/tfRankings/GetStandards?recordsetId=&eventId=&eventTypeId=` | HAR1[340] (empty `[]`), [409]… |
| Event conversions | `GET /api/v1/tfRankings/GetConvertEvents` | HAR1[718] |
| Bulk export (exists, untested) | `POST /api/v1/tfRankings/DownloadRankings` → blob (`Content-Disposition`) | service code HAR1[138] |
| Custom-list lookups (exists, untested) | `GET /api/v1/tfRankings/GetAthletes?listId=`, `GET /api/v1/tfRankings/GetTeams?listId=`, `GET /api/v1/tfRankings/GetCustomListInfo?listId=` | HAR1[133],[138] |
| XC API family | `/api/v1/xcRankings/GetMeets`, `/api/v1/xcRankings/GetMeetSimulationTeams` | HAR1[133] (`t!=='xc'?'tfRankings':'xcRankings'`) |
| Sport crossover | `GET /api/v1/TFX/go?action=tftoxc\|xctotf\|xctotfi&id=<AthleteID>` | HAR1[133] |

**`divListId 170770` is the Wisconsin 2026 outdoor state list** — not a national/division-of-competition list.
`GetRankings` response `division` block (HAR1[147]): `{"ID":170770,"Name":"Wisconsin","ParentID":168416,
"BaseDivID":638,"SeasonID":2026,"Website":"http://www.wiaawi.org","Filter":4,"LastMajor":true,
"Settings":{"gender":"a"}}`. Outcome: **state-scoped lists exist** and are first-class (each has its own
`divListId`, own nav, own seasons map). HAR1[0] is the URL of that state page.

### Coverage

- **Geography:** US high-school level (`levelDivId` 168416 = "High School", parent 167952 "United States")
  with a 2026 state-division set for all 50 states + DC + Overseas. `GetNavInfo` `regions` (76 entries)
  carries depth-1 states and depth-2 sub-state divisions (CA sections, NY associations, TX leagues).
  The nav `tree` (68 entries) returns the full sub-tree of the requested state (WI = 3 divisions,
  8 sectionals, 34 regionals, ids 170770–170837).
- **Sports:** TF outdoors and indoors; XC is a separate app + API family (`xcRankings`) keyed by
  division **and distance in metres**, not by `eventShort`.
- **Seasons:** the nav `seasons` map is scoped to the requested division and spans 2007→2027 for WI
  (43 keys, e.g. `"2027":181951`, `"2026":170770`, `"2025":159762`, `"2024":146624`, `"12027":187407`,
  `"12026":175082`, `"12025":164008` …). Indoor keys are `1xxxx`; the app itself decides indoor by
  `seasonId > 1e4` or the `track-and-field-indoor` URL slug (service code HAR1[138]).
- **Levels:** middle school and college/club divisions have their own level divs (168179 College, 170856 Middle
  School) reachable with the same API; irrelevant here.
- **Depth:** only the **selected season** is rankable per request (`SeasonID` 2026 / indoor 12026).
  A Class-of-2027 athlete's Grade-10 marks live in the 2025 lists — the 2026 list contains 2026 results
  only (each row's `SeasonID: 2026`, `ResultDate` in the 2026 season).

### Enumeration

Exact request shape (verified twice in HAR1, and byte-identical to the production contract in
`src/runtime/source/request.rs:352`):

```http
POST /api/v1/tfRankings/GetRankings   HTTP/1.1
content-type: application/json
{"reportType":"div","mode":"list","divListId":170770,"indoor":null,"eventShort":"100m","gender":"m",
 "qParams":{"grades":[11],"page":2},"qualifyingListKey":"","version":2,"debug":""}
```

| What | Enumerable? | How |
|---|---|---|
| Schools / teams | **Not via rankings.** `GET GetTeams?listId=` exists but its `listId` is a *custom list* id (`customList` in the response schema), not a division; no listing endpoint was exercised in the captures. | Use assignment 1/5 routes. |
| Meets | Not via rankings for a division; `GET /api/v1/{tfRankings,xcRankings}/GetMeets?recordsetId=&locationId=&meetList=&meetIds=` exists (code only, never called in captures). A ranking row **does** carry `MeetID`/`MeetName` for the athlete's best mark. | Assignments 3/5. |
| Athletes | **Yes — per event, per list, per gender, page by page.** No "all athletes in a list" mode exists in the observed API. The list landing view (empty `eventShort`) returns only rank-ordered **top-N per main family** (18 groups × depth 10 = 180 rows in 1 request, HAR1[147]) and is not page-complete. | single-event paging |
| Class of 2027 / Grade 11 | **Yes, server-side:** `qParams.grades:[11]`. Verified: pages 2/10/14/17 of WI/m/100m returned `GradeID 11` for 101–102 of 102 rows each (one stray `GradeID 12` on page 10), and the terminal page's max `rank`/`rowNum` = 1616 = the response `minCount` (a coincidence of the
filtered query — `minCount` is page-dependent, risk #10). | `grades:[11]` |
| Results | **Yes, best-mark-per-athlete-per-event rows** (one row per athlete per event family), not every performance. | paging |
| Relay rows | Yes: relay events return one row per relay team with `GradeID: 99` (placeholder) and `AthleteName` = the 4 member names joined with `<BR>`; the same response carries `relayTeams` keyed by `IDResult` with each member's `IDAthlete`/`GradeID`. Grade-11 relay members are recovered by joining `relayTeams[IDResult].Members[]`. | HAR1[147] group 8 + `relayTeams` |
| XC | Not enumerated here: `/CrossCountry/rankings/list/<IDDivision>/<gender>/<Meters>` + `xcRankings` API exist, but no XC nav/list-id capture exists and live access is 403. | needs live test |

Navigation/mode mechanics (all from the app's own code, HAR1[138], cross-checked against the captures):

- `eventChanged()` sets `depth = eventId ? 100 : 10` → **depth 100 for a single event, depth 10 for the
  multi-event list overview**. The captures match exactly (overview `depth:10`; every single-event page
  `depth:100`).
- Pages carry **100–102 rows** (ties spill over the nominal depth: pages of 102 rows with `rank`
  100→201 were observed).
- The URL itself accepts filters: the page component parses `ages`, `grades`, `include`, `convert`
  query params (HAR1[133] `re(n)`), and the site's own canonical links add `?eventType=<EventTypeID>&restrict=true`
  (HAR2[69]).
- `settings` in each response echoes the applied filter state (`depth,page,grades,eventType,fat,
  official,wind:-1,restrict:false,visibility:10,trackType:"any",followed:"noFilter"`); `minCount` and
  `eventTypes[].Results` are source-side counters: `minCount` is **page-dependent** within one query
  shape, `eventTypes[].Results` is page-independent for a given query shape (risk #10).

### Stable identifiers

| Identifier | Where | Observed values |
|---|---|---|
| `AthleteID` | every ranking row, `relayTeams[].Members[].IDAthlete` | e.g. WI/m/100m p17 rows 24991423…; profile URL is derivable: `https://www.athletic.net/athlete/<AthleteID>/track-and-field` — verified end-to-end: HAR1[147]'s rank-1 row (`AthleteID 28872883`) is HAR2[0]'s captured profile URL `/athlete/28872883/track-and-field` |
| `IDResult` | every ranking row; also the relay-roster key | 292277081 (rank 1), 286325812 (rank 1600) |
| `TeamID` | every ranking row | 3204 Middleton, 3052 Wauwatosa West |
| `EventID` / `EventTypeID` / `EventShort` | every row | 1 / 0 / `100m`; `EventTypeID` 98 = "Relay Split", 99 = "Wheelchair" (HAR1[414] `eventTypes`) |
| `SeasonID` | every row + nav `seasons` map | 2026 outdoor, `12026` indoor, `>1e4 ⇒ indoor` |
| Division list id (`divListId` / `ID`) | request body + response `division.ID`, nav `divListId`/`tree`/`regions` | **2026 outdoor**: national 168416, WI 170770, IL 169070, IN 169137, IA 169178, KS 169215, MI 169407, MN 169465, MO 169578, NE 169689, ND 170039, OH 170050, SD 170271 (from `regions`, HAR1[154]); **2026 indoor**: national **173005** (repo fixture `tests/fixtures/rankings/indoor-nav.json`: `seasons {"2026":168416,"12026":173005}`), WI 175082 (HAR1[154] `seasons["12026"]`) |
| `shortCode` | every row | per-result opaque code, e.g. `VPiXK5dHri4J1oNsm` — not an id namespace we can dereference |
| Names | `AthleteName`, `TeamName` | display only; never a join key (relay rows concatenate 4 names) |

The non-WI state id → state mapping above is from the captured `regions` array read in a Wisconsin-scoped
call; WI is confirmed by `divListId`/`seasons["2026"]`. Treat the other 11 as **[INFERENCE]** until a
live nav confirms them (the ids are consistent with the WI pattern and with `ParentID 168416`).

### Athletic.net leverage

This source *is* Athletic.net, so it is not an external replacement; its leverage is inside the
acquisition flow:

- A ranking row seeds **AthleteID → profile URL** (verified chain above), **TeamID**, **IDResult**,
  **MeetID/MeetName**, school name, state, and grade — i.e. it removes the need for name/school search
  requests for every discovered athlete.
- Discovery cost is ~**0.03 requests per athlete** (4,256 page requests for 142,705 national grade-11
  boys; §Quantification), vs 1 `GetAthleteBioData` request per athlete for the profile route (33.7×
  more requests for the same population, and each bio is ~30 KB vs ~0.85 KB per ranking row).
- It cannot replace search for the *targeted* population: a rankings scan finds only athletes who have
  at least one ranked best mark in a probed event family in that season. Workbook-driven name matching
  is still required for rows with no ranked mark.

### Athlete evidence

One decoded `groupedRankings[0][i]` row (HAR1[704], page 17) — that row carries 35 keys; across the 527
rows recovered from the capture the key count ranges 34–39, the extra keys being sparse optionals such as
`Age` and `Wind`:

```
shortCode, mediaCount, blurred, IDResult, SortIntRaw, SortIntCalc, SortIntOrig, display, Round, FAT,
Measure, PersonalBest, MediaCountJson, ResultDate, SeasonID, rank, rowNum, AthleteID, AthleteName,
GradeID, EventID, EventType, Event, EventShort, PersonalEvent, ConversionInt, EventTypeID, TeamID,
TeamName, TeamMascot, State, Country, MeetID, MeetName, isFieldSeries
```

| Field | Available | Note |
|---|---|---|
| name | ✅ `AthleteName` | relay rows concatenate members |
| graduating class/grade | ✅ `GradeID` — present on all 527 observed rows (11 with the filter); `Age` is an optional extra present on only 104/527 | source-reported, per team-season (`GradeID` is not a graduating class) |
| school / city / state | school ✅ (`TeamID`+`TeamName`), state ✅ (`State`, `Country`) | no city, no address |
| gender / category | implicit (request `gender=m/f`); `eventTypes` carries Wheelchair/Relay-Split categories | — |
| TF/XC distinction | ✅ by surface (TF rankings vs `xcRankings`) | no cross-sport flag inside a TF row |
| indoor/outdoor | ✅ by list id / `SeasonID`/`indoor` param | indoor ids `1xxxx` |
| performances | best mark only (`SortIntRaw/Calc/Orig`, `display`, `Measure`, `Wind`, `FAT`, `Round`, `ResultDate`) | `SortInt*` = raw/calculated/original integer forms (100m 10.41 → 10410; `ConversionInt` 240 for 100m) |
| PRs | ❌ (only "is this the athlete's best mark in this event this season") | `PersonalBest` present on every observed row but semantics unverified: distribution over 527 rows `{14:416, 0:69, 30:29, 8:7, 16:6}`; the bio API uses `0` for a non-PB, so `0` here may or may not mean the same |
| progression | ❌ | 2026 season only |
| meets | partially: the best mark's `MeetID`/`MeetName` | not the athlete's meet list |
| profile URL | derivable from `AthleteID` (verified) | needs no extra request |

### Recruiting information

**None.** No coach, AD, email, website, or contact field exists anywhere in `GetRankings`,
`GetNavInfo`, `GetAgesGrades`, or the ranking row schema. `GetNavInfo` `division.Website` is the
governing association URL (e.g. `http://www.wiaawi.org` for WI) — an association link, not a school or
team site. Rankings cannot contribute to the coach-directory side of the mission at all.

### Result evidence

| Field | In a ranking row | Note |
|---|---|---|
| ResultID | ✅ `IDResult` | |
| AthleteID | ✅ `AthleteID` | relay rows: team-row id + `relayTeams` roster |
| MeetID | ✅ `MeetID` + `MeetName` | meet of the best mark only |
| EventID | ✅ `EventID`, `EventTypeID`, `EventShort` | `EventTypeID` 98 relay split, 99 wheelchair |
| mark | ✅ `SortIntRaw`/`SortIntCalc`/`SortIntOrig` + `display` + `Measure` | |
| normalized mark inputs | ✅ `ConversionInt` (per-event conversion factor), `Measure: "M"/"E"/" "` | |
| timing method | ✅ `FAT` (1/0) | |
| wind | ⚠️ sparse: `Wind` present on only 38 of 527 observed rows (all `FAT:1`, all 100m; e.g. `3.8`, `0.0`) — i.e. only where a wind reading exists, and **absent from most wind-event rows** | filter `windLimits:[2]`, `wind:-1` default; [INFERENCE] absence = no reading recorded |
| implement/hurdle spec | ❌ | event variant is implied by `EventID`/`EventShort` only |
| heat/round | ✅ `Round` (`F`, `P`) | heat number not present |
| place | ❌ in a ranking row (ranked position instead: `rank`, `rowNum`) | `Place` exists in the bio API |
| date | ✅ `ResultDate` | day precision (`2026-05-15T00:00:00`) |
| school represented | ✅ `TeamID`/`TeamName` | per best mark row |
| relay membership | ✅ via `relayTeams[IDResult].Members[]` (`SortID`,`IDAthlete`,`AthleteName`,`Handle`,`PhotoUrl`,`GradeID`) | relay row itself has `GradeID:99` and no member ids |

### Incremental use

- The list is a **season-bound best-mark snapshot**; there is no cursor, `ETag`, `Last-Modified`,
  `If-Modified-Since`, or delta parameter in any observed request (`qParams` observed keys: `grades`,
  `page`, `eventType`). Nothing in the captured JS supports incremental fetch.
- Row `ResultDate` (day precision) + `MIN(ResultDate)` per event page is the only usable staleness
  signal: a weekly collector that re-walks a division must re-fetch every page, but it can stop early
  **inside a page sequence** only if it trusts mark ordering — and it cannot: a new slow mark appears at
  the *end* of the list, so prefix-only re-scans permanently miss new slow performers and all
  late-season entrants.
- Cheap, honest weekly policy for a 12-state scope: re-walk the full state-scoped plan (≈2.7k requests
  for one division-season, §Quantification), or re-walk only the events whose page-**independent**
  counter moved. Measured candidate: the `eventTypes` entry with `EventTypeID 0` → `Results`
  (WI/m/100m: **7,361** on unfiltered pages 2 and 3, **1,846** on grade-11 pages 2/10/14/17 — identical
  within a query shape, so it moves when that event's result population moves). Do **not** key this on
  `minCount`: it varied 2,117 → 3,282 across two unfiltered pages of the same query (risk #10), and the
  repo documents `minCount` as a per-page figure that must not drive pagination
  (`CHROMIUM_DESIGN.md:71`). This is a design proposal, not an observed feature.
- Ranking rows are replaced when a better mark is set, so *changed* rows are indistinguishable from new
  ones except by re-reading and diffing `IDResult`/`SortIntRaw` client-side.
- A local `sportRankings:false` division also carries `LastMajor`, `TFRIRefreshInterval`,
  `TFRIStartDate/EndDate` fields on the division object (all `null` for WI) — if populated they would
  indicate source-side refresh cadence. Currently null ⇒ no scheduling signal.

### Access characteristics

Classification: **public structured JSON behind a TLS-level browser challenge (browser application)**.

- **Live probe from this machine (2026-09-19, plain `curl`, default UA, no evasion):**
  - `GET https://www.athletic.net/TrackAndField/rankings/list/168416/m/100m` → **403**, 5,530 bytes,
    body `<title>Just a moment...</title>` (Cloudflare interstitial).
  - `GET https://www.athletic.net/api/v1/tfRankings/GetRankings` → **403**, 5,472 bytes, same
    interstitial.
  - This matches the repo's own 2026-09-19 live-capture attempt (`403` / `Attention Required!`, README
    "Qualification status"). Two requests total were issued; no retry, no header/UA modification.
- In-browser (the HAR session) the API answers `200` with `application/json`; page bodies for a
  single-event page at depth 100 measured **84–89 KB** for 102 rows (≈850 B/row), terminal page 16.7 KB
  for 17 rows.
- Auth-relevant mechanics observed (header **names** only; no header values were read or copied): every
  request carries `anet-appinfo`, `anet-site-roles-token` and `pageguid`; `GetNavInfo` (HAR1[154]) and
  `GetAgesGrades` (HAR1[153]) additionally carry `anettokens`, while the `GetRankings` POST (HAR1[147])
  does **not**. The app obtains that token from the first `GetRankings` response field `jwtTFTopReport`
  (present, 319 chars, in HAR1[147]) and re-sends it as `anettokens` (service code HAR1[138]). Because
  the capture is a signed-in browser session, none of these headers can be assumed to work anonymously;
  the role hint in `anet-site-roles-token` was not decoded and is not relied on here.
- Entitlement seams seen in code: `showDepthUpgrade` (`!sportRankings && isDivReport`),
  `ANetRoles.Division_DownloadRankings`/`Division_Average3` gate the download button,
  `GetSSUpgradeLink()` (`/Partner/SiteSupporter/Upgrade.aspx?SchoolID=…`), `hasSS`/`hasAPlus` on the
  signed-in user. Implication: `DownloadRankings` may be role-gated — must be tested in a real session.
- Politeness: no `429`, no `Retry-After` observed in 722 + 167 HAR entries; the pipeline's own
  production default is a 2,000 ms inter-request delay (`delayMs` in the retained partial delivery's
  Coverage sheet). A 2.7k-request state walk at 2 s/request ≈ 90 min serial per division-season.
- **Privacy note for future HAR handling:** HAR2 contains a signed-in session payload
  (`SignedInUser/GetSignedInUser`) whose schema includes `Email`, `PhoneNumber` and address fields.
  No values were read or copied into this report, and analysis-only scripts must keep redacting them.

### Recommendation

**DISCOVERY-ONLY** (inside Athletic.net: the cheapest AthleteID/TeamID/MeetID/grade seed; not a source
that can replace Athletic.net, and not a coach source).

**Cheapest COMPLETE method — recommendation:** state-scoped, single-event enumeration with
`qParams:{"grades":[11],"page":N}`:

1. One `GetNavInfo` per state list (yields the season list ids *and* the observed variant catalog);
   one `GetAgesGrades` per (state, gender) for cheap sizing.
2. For each manifest family variant: page `N = 1,2,…` until a page carries no rows (the source's own
   termination signal); budget `ceil(rows/100) + 1` requests per variant.
3. Project grade 11 server-side (`grades:[11]`) for individual events. Relay rows carry the placeholder
   `GradeID 99`, so a grade filter cannot select them; production therefore requests relay variants
   with `qParams.grades = []` (`rankings_collection/helpers.rs:43` + `source/request.rs`) and recovers
   grade-11 members by joining `relayTeams[IDResult].Members[]` on `GradeID == 11`. *[UNVERIFIED: what
   the server returns for a grade-filtered relay query — no such request exists in either capture.]*
   Consequence: relay variants are fetched at all-grade breadth, so their page count is not bounded by
   the grade-11 numbers in Table 3.
4. Prefer the 12 state lists over the national list: **2,728 vs 4,233 requests** for the same 40,695
   Midwest grade-11 athletes (measured model below), and every fetched row is in-scope instead of 28.5%
   in-scope.

**Single-event vs multi-event:** multi-event (empty `eventShort`) is unusable for complete discovery —
it returns 18 rank-ordered top-10 groups (180 rows/request, depth fixed at 10 by the app) and carries
`minCount: 0`, so it cannot be paged to completeness and only ever exposes fast athletes. Use it as a
1-request smoke test per list (does the state have data? which families?), never as the discovery path.

**Two cheaper paths exist but are untested and must be verified in a real browser session before
adoption:** (a) `POST /api/v1/tfRankings/DownloadRankings` — a same-shaped request returning a blob
(`Content-Disposition` filename `Rankings_<reportType>_<divListId>_…`) which, if it returns the full
filtered list, collapses a division to ~95 requests; it is plausibly role/entitlement-gated
(`Division_DownloadRankings`). (b) `GET GetAthletes?listId=` — exercised only as a custom-list lookup in
the captured code; if a division list id were accepted it would be a single-request roster.

---

### Quantification

Model: rows are taken from the retained national grade-11 boys delivery
(`/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx`, 142,705 athlete rows, columns
`AthleteID|Names|GradeID|Teams|States|Events|Result Count|…`), counting each athlete once per event in
their `Events` cell. Reproduce with `tools/04-rankings-request-model.py` (read-only).

- page size = 100 rows (app's own single-event depth; 100–102 observed),
- requests per variant = `ceil(rows/100)` row-bearing pages **+ 1** terminating page,
- variants probed = the repo's 95-variant design target (46 requested families expanded); variants with
  no rows cost 1 empty probe each,
- per-division overhead = 1 `GetNavInfo` + 1 `GetAgesGrades`.

#### Table 1 — 12-state scope, 2026 outdoor boys grade 11 (list per state)

| state | grade-11 athletes | variants with rows | athlete-event rows | row-bearing pages | terminating pages | empty probes | total requests |
|---|---|---|---|---|---|---|---|
| WI | 3,924 | 39 | 12,945 | 155 | 39 | 56 | 250 |
| MN | 3,780 | 44 | 12,211 | 149 | 44 | 51 | 244 |
| IA | 2,619 | 38 | 8,061 | 108 | 38 | 57 | 203 |
| IL | 6,009 | 44 | 18,105 | 211 | 44 | 51 | 306 |
| MI | 5,984 | 48 | 19,476 | 230 | 48 | 47 | 325 |
| IN | 2,681 | 37 | 6,271 | 86 | 37 | 58 | 181 |
| OH | 6,361 | 51 | 20,372 | 239 | 51 | 44 | 334 |
| MO | 3,621 | 40 | 11,380 | 140 | 40 | 55 | 235 |
| KS | 2,037 | 34 | 4,811 | 72 | 34 | 61 | 167 |
| NE | 1,980 | 35 | 6,014 | 83 | 35 | 60 | 178 |
| ND | 762 | 26 | 2,733 | 44 | 26 | 69 | 139 |
| SD | 937 | 26 | 3,323 | 47 | 26 | 69 | 142 |
| **12 states** | **40,695** | 462 | **125,702** | **1,564** | **462** | **678** | **2,704 (+24 overhead = 2,728)** |

#### Table 2 — national single list 168416 (all states, same query shape)

| scope | variants with rows | athlete-event rows | row-bearing pages | terminating | empty probes | overhead | total requests |
|---|---|---|---|---|---|---|---|
| national 168416, boys, grade 11 | 79 tokens | 408,810 | 4,136 | 79 | 16 | 2 | **4,233** |

Repo-measured anchor for the same division: **4,256 source receipts** (README/SCOPE: "95 queries, 4,256
receipt digest checks, 340,238 individual results, 57,629 relay-member results, 142,705 unique
athletes"). The model's 4,233 is within 0.6% of the recorded count — the model is validated.

#### Table 3 — per-variant extremes (national, boys outdoor grade 11)

| variant | athletes (national) | pages | variant | athletes | pages |
|---|---|---|---|---|---|
| 100m | 50,791 | 508 | 4x800m | 9,623 | 97 |
| 200m | 45,767 | 458 | javelin | 8,603 | 87 |
| 400m | 33,565 | 336 | pv | 6,149 | 62 |
| 800m | 28,873 | 289 | 4x1600m | 978 | 10 |
| shot | 26,620 | 267 | 400mh | 1,937 | 20 |
| 1600m | 24,855 | 249 | 2miles | 532 | 6 |
| discus | 24,776 | 248 | hammer | 287 | 3 |
| lj | 23,571 | 236 | 4xmile | 87 | 1 |
| 3200m | 14,518 | 146 | decathlon | 198 | 2 |

Relay variants are fetched **unfiltered** by the production design (`helpers.rs`: `grade: if relay {None}
else {Some(11)}`, and `request.rs` serializes `None` as `"grades":[]`), so their page count is driven by
all-grade relay-team rows, not by the grade-11 member count above. The retained numbers do not separate
that share: 340,238 individual + 57,629 relay-member rows at depth 100 ≈ 3,979 row-bearing pages, so the
relay variants + tie spill + nav account for the remaining ~277 receipts of the 4,256.

#### Table 4 — all-division discovery

| division | list scope | requests | status |
|---|---|---|---|
| 2026 outdoor boys | 12 state lists | **2,728** | measured (Table 1) |
| 2026 outdoor boys | national 168416 | **4,233** (recorded 4,256) | measured + repo receipt count |
| 2026 outdoor girls | same 12 list ids, `gender=f` | roughly the boys figure, **not measured** | **[INFERENCE]** no girls ranking data exists in any artifact: the retained delivery is boys-only, the repo's only girls fixture is a synthetic 3-row file (`tests/fixtures/rankings/indoor-girls-100m-p1.json`, `minCount:4`, `eventTypes:[]`), and live access is blocked. Gender is a request parameter over the same lists, so the request *shape* is identical and the count scales with girls' ranked population per variant |
| 2026 indoor boys/girls | 12 indoor state lists (ids only known for WI 175082, national 173005) | not measurable | **unmeasured** — indoor event vocabulary is smaller (55m/60m/300m/500m/600m/1000m/weight/indoor pentathlon) but list ids and row counts are not in any captured artifact |
| **all four, 12-state scope** | | **≥ 5,456 measured-floor (2× 2,728) and up to ≈ 11k if indoor/outdoor and both genders all match the boys-outdoor figure** | |

Full-profile enumeration comparison: 142,705 national athletes × 1 `GetAthleteBioData`
(= 33.7× the national rankings scan) for the deliverable above; the Midwest subset alone would be
40,695 requests. Rankings discovery is ~0.03 requests/athlete; profile enumeration is ~1 request/athlete.
Transfer: ≈850 B/row × 125,702 rows ≈ 107 MB for the 12-state boys-outdoor scan [INFERENCE from the
measured 84–89 KB per 102-row page].

### Omissions and risks

1. **No ranked mark ⇒ invisible.** Any grade-11 athlete who has no best-mark row in a probed event
   family in the selected season cannot be discovered from that division list — TF-only lists cannot see
   XC-only athletes (XC is a separate, unmeasured surface), relay-only squads whose roster rows are
   missing stay unresolved, and anyone excluded by the request's default filter state
   (`official:true`, `fat:true`, `wind:-1`, `restrict:false`, `windLimits:[2]`) is filtered server-side.
   *[INFERENCE: the exact exclusion semantics of `official`/`fat`/`restrict`/`windLimits` are documented
   nowhere in the captures; they are plausible coverage levers and the site's own canonical links set
   `restrict=true`]*. Rankings can only be a seed, never a population.
2. **Relay-only athletes and unresolved rosters.** Relay rows carry `GradeID: 99` and a concatenated
   name; a grade-11 member is only recoverable through `relayTeams[IDResult].Members[]`. The retained
   delivery kept **15,724 unresolved roster results** (repo) rather than inventing members, and relay
   pages are fetched unfiltered, so both cost and completeness vary by relay support per state.
3. **Walk events.** The nav vocabulary contains exactly 23 race-walk/power-walk events whose shorts are
   `… rw` / `… pw` (name contains "walk": 23; short ends `rw`/`pw`: 23; no nav short contains the
   substring `walk`) (e.g. `60m rw`, `1500m pw`, `1609m rw`). The production `is_excluded` predicate is a
   case-insensitive substring test for `"walk"` on the **short** code (`src/runtime/rankings/types.rs:310`),
   which does **not** match `rw`/`pw`. The exclusion that actually holds is *family matching*: no
   46-family entry equals or prefixes those shorts (`catalog.rs:215`), so walk events never enter the plan.
   Risk: if a future nav ever publishes a walk short inside a requested family's namespace, the
   substring guard will not catch it — it is a no-op for every observed walk short.
4. **Variant drift.** 46 requested families expand to variants (95 design target); the retained delivery's
   `Events` column contains only **79 distinct tokens**, and 7 of them are bare numerals
   (`4`,`8`,`16`,`2`,`6`,`10`,`12`, together ~11.3k athlete-rows) whose mapping to nav shorts is not
   explained by any captured artifact (no nav short is numeric in the captured 283-event catalog).
   Any consumer joining on event tokens must treat unexplained tokens as variants, not families.
5. **Indoor/outdoor bleed.** The outdoor 2026 list already returns indoor-format events (`60m` 755,
   `600m` 305, `1000m` 84, `55m` 63, `weight` 54, `indoor pentathlon` 3 athlete-rows in the boys-outdoor
   delivery), so an "outdoor" scan is not a clean outdoor-only population, and an outdoor plan may carry
   indoor-family probes. Conversely indoor lists (173005/175082) are separate and unmeasured here.
6. **Grade is source-reported, not a roster fact.** `GradeID` comes from the athlete's team-season grade
   entry; observed vocabulary includes grade ids 6–12 plus 21–24 (`GetAgesGrades`, HAR1[153]) and the
   relay placeholder 99. A repeat/transfer athlete can be listed at a grade that does not match the
   school's Class-of-2027 roster. Grade 11 in rankings is discovery evidence, never eligibility.
7. **Page-window arithmetic is unsafe for completeness.** Pages carry 100–102 rows; `rank` is a
   *tie-aware competition rank* that repeats while `rowNum` keeps incrementing (of 527 observed rows, 489
   have `rank == rowNum` and the rest differ only at ties, e.g. `rank 107,107` with `rowNum 108,109`).
   Only the source's own termination (a page that carries no ranked rows) proves completeness — the
   repo's collector relies on exactly that. A consumer that reads `page*depth` rows and stops cannot tell
   a tie-spill page from a truncated list.
8. **Page cap semantics.** The production cap (`--max-pages-per-event`, default 10,000,
   `max 1..=10,000`) **pauses** the collection with reason `PageLimit` (`helpers.rs:169`) — it does not
   silent truncation. A low cap therefore shows up as an incomplete-but-paused division; in the 12-state
   plan the deepest single variant is MI boys 100m at 27 pages (WI boys 100m grade 11 = 17 row-bearing
   pages, HAR1[704] terminal page), so the default cap is not binding except at deliberately tiny values.
9. **Filter-state non-reproducibility.** The URL-addressable filter state (`grades`, `ages`, `include`,
   `convert`, `eventType`, `restrict=true` used by the site's own canonical links) means two consumers
   can legitimately see different row sets for the same `divListId`/`eventShort`. Discovery receipts
   must record the full request body + `settings` echo (the repo does) or the list is not reproducible.
10. **Counters are ambiguous, so do not size from them.** `minCount` behaved as a query total in one
    grade-filtered query (constant 1,616 across pages 2/10/14/17 = the terminal `rank`) but varied
    within one unfiltered query (2,117 at page 2 → 3,282 at page 3 with identical settings except
    `page`). The pipeline treats it as a *lower bound* (`sourceReportedLowerBound: 118847` for the
    national unfiltered 100m walk in the retained partial delivery, whose last page 116 carried
    `minCount 118847`, `rows 102`, `firstRow 11500`, `lastRow 11601`). `eventTypes[].Results` is also a
    source-side count (WI/m/100m unfiltered: 7,361 standard / 29 relay-split / 14 wheelchair; grade-11
    filtered: 1,846 / 5 / 4); it *is* page-independent within a query shape in all 6 bodies observed
    (7,361 twice unfiltered; 1,846 four times filtered), but it counts results, not pages, so it cannot
    be converted into a page budget. Either may be a sizing hint; neither may be used as a completeness
    proof.
11. **XC is a separate, unmeasured surface.** `/CrossCountry/rankings/list/<IDDivision>/<gender>/<Meters>`
    plus the `xcRankings` API family exist, but the XC division id set, its distances, and its row counts
    are in no captured artifact, and live access returned 403. Do not budget XC from the TF numbers:
    distance-keyed lists imply a page sequence per (division, gender, distance).
12. **Third-party/entitlement claims are unverified.** `DownloadRankings`, `GetAthletes?listId=`,
    depth upgrades, and coach/site-supporter gating are code observations only; no request of that shape
    appears in either capture.

### Evidence appendix

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.athletic.net/TrackAndField/rankings/list/170770/m` (HAR1 entry 0) | GET | 200 | Session page = **Wisconsin** state list (170770), boys; canonical `/rankings/list/<divListId>/<gender>` route | 2026-09-18T18:54:41Z |
| `https://www.athletic.net/api/v1/tfRankings/GetRankings` HAR1[147] body `{"reportType":"div","mode":"list","divListId":170770,"eventShort":"","gender":"m","qParams":{}…}` | POST | 200 | Multi-event mode: 18 `groupedRankings` groups × depth 10 = 180 rows, `minCount:0`; `division` block identifies 170770 = Wisconsin, ParentID 168416, SeasonID 2026; rank-1 row `AthleteID 28872883` | 2026-09-18T18:54:42Z |
| `https://www.athletic.net/api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m&recordSetId=0&locationId=0&teamId=0&indoor=false` HAR1[154] | GET | 200 | `divListId 170770`, `levelDivId 168416`, 6 levels, 220 countries, **76 regions incl. all 12 target state ids**, 68-node WI tree, `seasons` map 2007→2027 (WI outdoor 2026=170770, WI indoor 2026=175082), 283-event vocabulary incl. walk `rw/pw` shorts and 96 relays | 2026-09-18T18:54:42Z |
| `https://www.athletic.net/api/v1/tfRankings/GetAgesGrades` HAR1[153] body `{"recordSetId":0,"locationId":0,"teamId":0,"listId":0,"seasonId":2026,"restrict":false,"includeEvents":[18 main event ids]}` | POST | 200 | One-request sizing: grade counts (9:22,934 10:21,535 **11:19,725** 12:16,436, plus 6–8, 21–24) and age counts (sum 10,925); grade vocabulary includes non-12 grades | 2026-09-18T18:54:42Z |
| `…/GetRankings` HAR1[348]/[414]/[478]/[539] `{"eventShort":"100m","gender":"m","qParams":{"page":2…4}}` | POST | 200 (bodies captured for 414, 478 only) | Unfiltered pages: 102 rows each, `rank` 100→201 and 200→301, mixed grades 9–12; `minCount` 2,117 (p2) vs 3,282 (p3) with otherwise identical settings | 2026-09-18T18:54:53–55Z |
| `…/GetRankings` HAR1[587]/[622]/[656]/[704] `{"eventShort":"100m","gender":"m","qParams":{"grades":[11],"page":2\|10\|14\|17}}` | POST | 200 | Grade filter is server-side: 101–102/102 rows `GradeID 11`; terminal page 17 = 17 rows, `rank`/`rowNum` 1600→1616, `minCount 1616`; 35-field row schema incl. `IDResult, MeetID, MeetName, TeamID, State, FAT, Wind, Round` | 2026-09-18T18:55:10–20Z |
| `https://angular.athletic.net/app/site-app/chunk-yZ2mAgBc.js` HAR1[138] | GET | 200 | TF rankings service: exact `GetRankings`/`DownloadRankings` bodies; `getAgesGrades`, `GetStandards`, `CanEditRecords`, `GetCustomListInfo` shapes; `eventChanged()` sets depth 100 (event) / 10 (overview); `maxDepth = recordSet?.depth`; entitlement seams (`Division_DownloadRankings`, `showDepthUpgrade`) | 2026-09-18T18:54:42Z |
| `https://angular.athletic.net/app/site-app/chunk-DEV-xx7r2.js` HAR1[133] | GET | 200 | `/api/v1/xcRankings/GetMeets`, `tfRankings/GetAthletes?listId=`, `GetTeams?listId=`, `TFX/go?action=tftoxc|xctotf` ; filter query-param parser for `ages,grades,include,convert` | 2026-09-18T18:54:42Z |
| `https://angular.athletic.net/app/site-app/tf-rankings.routes-CeE2uDyE.js` HAR1[70] | GET | 200 | Route table: `list/:divListId/:gender[/:eventShort]`, `records/…`, `venue-records/:locationId/…`, `:qualifyingListKey/…` (qualifying mode) | 2026-09-18T18:54:42Z |
| `https://angular.athletic.net/app/site-app/main-M36VMDMH.js` HAR1[18] | GET | 200 | `/CrossCountry/rankings` route chunk `xc-rankings.routes-BMQjja26.js`; `getDivisionLink` builds `/CrossCountry/rankings/list/<id>` vs `/TrackAndField/rankings/list/<id>` | 2026-09-18T18:54:42Z |
| `https://www.athletic.net/athlete/28872883/track-and-field` HAR2[0] | GET | 200 | Profile URL form for an `AthleteID` seen in HAR1[147] rank-1 row ⇒ rankings→profile join verified | 2026-09-18T19:20:59Z |
| `https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=28872883&sport=tf&level=0` HAR2[127] | GET | 200 | One request = full history: `resultsTF` (53), `allSeasons`, `grades {school_season→grade}`, `meets` (12), `relayTeamMembers` (40), `resultsXC` (null for this TF-only athlete); 30,344 bytes | 2026-09-18T19:21:00Z |
| `https://angular.athletic.net/app/site-app/athlete-bio.routes-BYYQfvXR.js` HAR2[69] | GET | 200 | XC ranking link template `/CrossCountry/rankings/list/<IDDivision>/<gender>/<Meters>?restrict=true`; TF link adds `?eventType=<EventTypeID>&restrict=true` | 2026-09-18T19:21:00Z |
| `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/{README.md,SCOPE.md,HANDOFF.md,CHROMIUM_DESIGN.md}` | read | n/a | 46-family manifest, 95-variant target, walk exclusion, relay roster join, page cap semantics; `CHROMIUM_DESIGN.md:71` records `minCount` as a per-page figure that must never drive pagination (matches the 2,117/3,282 measurement above); **retained outdoor-boys division: 4,256 receipts, 340,238 individual results, 57,629 relay-member results, 15,724 unresolved roster results, 142,705 unique athletes** | 2026-09-19 |
| `tests/fixtures/rankings/indoor-nav.json` | read | n/a | National nav: `divListId 168416`, `levelDivId 168416`, `seasons {"2026":168416,"12026":173005}` ⇒ national indoor 2026 list = 173005 | 2026-09-19 |
| `src/runtime/source/request.rs`, `src/runtime/browser/transport/rankings.rs`, `src/runtime/rankings/{types,division,catalog}.rs`, `src/runtime/rankings_collection/helpers.rs`, `src/cli/args.rs` | read | n/a | Production request body identical to capture; relay pages sent `grades:[]`; `is_excluded` = substring "walk" on short; family match by exact short/prefix; page cap **pauses** (default 10,000); walk/pw/rw shorts not matched by "walk" | 2026-09-19 |
| `/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx` (10.1 MB, 1 sheet `All Athletes`, 142,705 data rows) | read (streamed) | n/a | Per-state and per-event row counts used for Tables 1–3; 12-state share 40,695/142,705 = 28.5%; WI 100m grade 11 = 1,596 rows | 2026-09-18 21:27 (file), read 2026-09-19 |
| `/home/lewis/Downloads/Athletic-2026-US-Boys-100m-PARTIAL_NOT_COMPLETE-…xlsx` sheet `Coverage` | read | n/a | National unfiltered 100m walk: `capturedPages 116`, `rawRows 11831`, `uniqueResultRows 11601`, `grade11ResultRows 4034`, `sourceReportedLowerBound 118847`, `lastPageObservation {page:116,depth:100,minCount:118847,rows:102,firstRow:11500,lastRow:11601}`, `stopReason user_scope_change`, `delayMs 2000`, `challenges 0`, `rateLimits 0` | 2026-09-18 16:24 (file), read 2026-09-19 |
| `https://www.athletic.net/TrackAndField/rankings/list/168416/m/100m` | GET (curl, default UA) | **403** | Cloudflare `Just a moment...` interstitial, 5,530 bytes — live access blocked for non-browser clients from this machine | 2026-09-19T23:18Z |
| `https://www.athletic.net/api/v1/tfRankings/GetRankings` | GET (curl, default UA) | **403** | Same interstitial, 5,472 bytes — API equally blocked; no evasion attempted | 2026-09-19T23:18Z |
| HAR1 `GetRankings` bodies → decoded files | `tools/04-har-rankings.py body /home/lewis/Downloads/www.athletic.net.har <idx> tools/scratch-04/gr-<idx>.json` (read-only helper; same script also dumped `navinfo-154.json`, `agesgrades-153.json`, `standards-340.json`) | 200 | Source of every row/field/count number above: 527 rows from entries 414, 478 (unfiltered pages 2–3) and 587, 622, 656, 704 (grade-11 pages 2/10/14/17). Entries **348** and **539** answered `200` with 88,676 / 87,425-byte JSON bodies but the HAR stored no `content.text` for them (cache-served), so those two pages are not part of the row analysis. 516 KiB extracted in total across those nine files | 2026-09-19T23:13Z |
| `tools/04-rankings-request-model.py` | run | n/a | Reproduces Tables 1–2: 12 states = 1,564 row-bearing pages + 462 terminators + 678 empty probes + 24 overhead = **2,728**; national = 4,136 + 79 + 16 + 2 = **4,233** | 2026-09-19T23:17Z |
