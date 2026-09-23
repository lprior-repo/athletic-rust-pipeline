# Athletic.net — national source report (lane `athleticnet`)

Scope of this file: the whole lane contract for `www.athletic.net` as a **national** (51-jurisdiction) TF/XC
source. Every number below is either re-derived from a file in `samples/` (byte-exact captures, see
`samples/CAPTURES.md`) or explicitly marked `[INFERENCE]` / `[CITED]`. `[CITED]` means the value comes from
the retained research corpus under `~/Downloads/midwest-tfxc-source-research/` and was not re-measured here.

The derived numbers and identifier examples were produced by `python3 samples/build-schema.py` (it failed
loudly if a sample no longer matched this document); the 52 state rows by
`python3 samples/checks-enum-crosscheck.py`. Both scripts, and every other `samples/*.py` probe this report
names, were deleted from the tree by the contract commit `ea81c56` — the commands are provenance, not a
reproduction recipe. What remains runnable here is the retained evidence: `samples/`, `schema.json` and
`coverage.json`.

| Field | Value |
|---|---|
| **Lane** | `research/sources/athleticnet` |
| **Generated** | 2026-09-21 (probe captures 2026-09-22 UTC) |
| **Samples** | 52 files on disk, 2.07 MiB (58 at capture time; the 6 `samples/*.py` probes were deleted by `ea81c56`) — 17 live response bodies + 9 header files, 13 HAR extracts + request log, 8 copies of retained corpus captures, `refetch.sh`, 3 derived; every one listed in `samples/CAPTURES.md` |
| **Machine-readable** | `schema.json` (76 verified identifier examples), `coverage.json` (52 state rows + accounting + entitlement + request model + handoffs) |
| **Credentials** | none persisted; `jwtTFTopReport` values and `set-cookie` lines are masked by `samples/redact-samples.py` (idempotent, verified by hash) |

---

## 1. Source name

**Athletic.net** (`www.athletic.net`). Corporate ownership not verified by this lane (not needed for the
census; no ownership claim is recorded here). SPA (ASP.NET + Angular shell),
Cloudflare in front (`server: cloudflare`, `cf-ray` in every captured response header), data served from
`/api/v1/**` JSON endpoints.

Page shells are small and client-rendered: the rankings page document is **6,975 B**
(`https://www.athletic.net/TrackAndField/rankings/list/170770/m`) and the athlete page document is **9,771 B**
(`https://www.athletic.net/athlete/28872883/track-and-field`) — both `entry 0` of the two retained HARs,
`_resourceType: document`, status 200, re-checked 2026-09-21 against
`~/Downloads/www.athletic.net{,2}.har` (the HAR request log in `samples/` holds XHR entries only). All data
arrives over XHR.

## 2. Geographic coverage

National, self-published, and enumerable in one call:

* `GET /api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m&recordSetId=0&locationId=0&teamId=0&indoor=false`
  returns 52 `divType == "State"` nodes in `regions[]` plus the selected region echoed into `tree[0]`
  (`samples/har1-getnavinfo-wi-2026-hs-boys.json`, 78,468 B).
* 51 of those 52 carry a USPS code in `state`; **all 51 are valid `UsJurisdiction` codes and the set is exactly
  the enum's code set** (`samples/checks-enum-crosscheck.txt`: `regions[] states not in enum: []`,
  `enum codes with no nav entry: []`).
* The 52nd node is **`Overseas` (id 170162, `state: null`, `subDivType: "Schools"`)** — reported as out of
  scope, not dropped: it is a `State`-divType region with no USPS code and `UsJurisdiction` has no variant for
  it (`coverage.json` → `jurisdiction_coverage.out_of_scope`).
* The full 52-row table (nav path, nav id, USPS code, enum variant, sub-div type) is
  `samples/checks-state-divs.tsv` and is repeated machine-readably in `coverage.json`.
* Node taxonomy per state is not uniform and matters for discovery: `subDivType` is `Divisions` (WI), `Classes`
  (IN, KS, WY, …), `Associations` (AL, …), `Regions` (AK). Counts by value are in `coverage.json`.
* Second tier ships in the same payload for three states only — CA 10 nodes, TX 8, NY 5 — every other state's
  child nodes need their own call (`coverage.json` → `jurisdiction_coverage.second_tier_in_nav`).

## 3. Sports

Track & Field and Cross Country, each with indoor/outdoor season variants. The captured vocabulary is `tf`:
the meet payload's own top-level fields are `sport: "tf"`, `sport1: "tf"`, `sport2: "tfo"` (outdoor TF;
`samples/anon-meetdata-634313.json`). The nav payload publishes the event vocabulary: `events[]` holds **283
objects** (ids 1–660; id 1 = `100 Meters` / short `100m`, `t: T`, `m: M`) and `events_main` is the 36-id
main-HS shortlist — but only **18** of those 36 ids have an object in this capture's `events[]` (the other 18,
e.g. 19, 20, 21, 22, 67, have no object in the gender=m/2026 list), so event mapping must tolerate ids that
appear in `events_main` without a vocabulary object (`samples/har1-getnavinfo-wi-2026-hs-boys.json`; the
cross-source token map is the peer lane `46-event-token-mapping.md`).
XC is present in the source (athlete payload carries `resultsXC`, `distancesXC`) but **no XC endpoint was
captured in this lane** — see §22 gaps.

## 4. Historical depth

* **Per region**: one `GetNavInfo` call carries the region's full season map — for Wisconsin, **22 outdoor
  keys (2006 … 2027) and 21 indoor keys (12007 … 12027)**, each an independent list id
  (`samples/har1-getnavinfo-wi-2026-hs-boys.json` → `seasons`). Indoor keys are the outdoor id prefixed `1`
  (`12026` → 175082).
* **Per athlete**: `GetAthleteBioData` returns the athlete's whole career grouped by season; the captured
  athlete has 2 seasons and 53 results across 18 distinct meets (`allSeasons`, `meets`, `resultsTF` in
  `samples/live-getathletebiodata-28872883-2026-09-20.json`).
* **Per meet**: results are retained indefinitely per meet id; no year restriction was observed on
  `GetAllResultsData`.
* `[CITED]` the retained delivery workbook covers 2026 only.

## 5. Discovery mechanism

Five distinct routes, in decreasing order of cost:

1. **Rankings list → rows** (state/node scoped). Select a list id (state node or lower), then
   `POST /api/v1/tfRankings/GetRankings` per event/gender/grade/page. Rows carry `MeetID`, so rankings double
   as meeting-point discovery: one full page of 102 rows referenced **66 distinct meet ids** in
   `samples/har1-getrankings-wi-170770-m-100m-p2-n102.json`.
2. **Division tree walk** (anonymous). `GET /api/v1/SiteHeader/GetDivChildren?sport=tf&divId=<node>` returns
   the node's children in `divInfo[]`; one call per internal node. `divId=168416` (High School) returns all 52
   states in one request; WI's 3 divisions and 16 sectionals match the nav tree node-for-node
   (`samples/atn-probe-divchildren-168416.json`, `-170770.json`, `-170771.json`, `-170772.json`).
3. **Meet id → whole meet**. `GetMeetData` → `GetEventDivisionData` → `GetAllResultsData` (3 requests, see §16)
   gives every division, event, round, individual result and relay leg of one meet, each row carrying
   `AthleteID`, `TeamID`/`SchoolName`, `Grade`, `IDResult`, `ShortCode`.
4. **Athlete id → career**. `GetAthleteBioData` returns every result of one athlete plus a meet map with names
   and dates, i.e. a cheap MeetID seed when athlete ids are already known.
5. **External handoff**. State associations, AthleticLIVE timing tenants and timer sites publish Athletic.net
   URLs/ids directly — see the handoff section §21.

Search is **not** available to this lane: no search XHR was captured, and `/search.aspx` is disallowed by
robots (`samples/atn-robots.txt` line 37). A school-name → `IDSchool` resolver therefore has to be built from
the division tree plus team pages, or captured from the browser's own search UI.

## 6. Stable identifiers

Full table with one real example each, the file it came from, its JSON path and its join role: `schema.json`
(76 entries, every one verified at build time by `samples/build-schema.py`). The load-bearing ones:

| Id | Example | Where observed | Notes |
|---|---|---|---|
| `AthleteID` | 22874443 | rankings row `groupedRankings[0][0]` | person key, national, shared by all sports/levels |
| `IDAthlete` | 28872883 | bio `athlete.IDAthlete` | same integer, different spelling |
| `RelayTeamID` | 32663189 | `relayTeams.<IDResult>.RelayTeamID` | on relay rows `AthleteID == RelayTeamID` (verified 40/40) |
| `TeamID` | 19566 | rankings row | team record key |
| `IDSchool` / `SchoolID` | 3204 (Middleton) | allresults `teams[0]`, bio `athlete.SchoolID` | school key; one school can hold several team records (teamnav `linkedTeams` has Glacier Creek 57750, Kromrey 57810) |
| `MeetID` / `IDMeet` | 667477 / 610825 | rankings row / bio meet map | meet key = URL path segment |
| `IDResult` | 294093256 | rankings row | one athlete, one mark, one entry |
| `ResultID` | 294438034 | `GetResultsData3.relayLegs[0]` | leg-level id inside a relay result |
| `shortCode` / `ShortCode` | `2KiyYVPiwipw058tr` / `VPiXK5dHri4J1oNsm` | rankings row / meet result row | public `/result/<shortCode>` page; **key casing differs between payloads** |
| `divListId` | 170770 | nav `divListId` | rankings list id; also the `/TrackAndField/rankings/list/<id>` URL segment |
| `levelDivId` / `countryDivId` | 168416 / 167952 | nav | High School / United States nodes |
| `seasonId` | 2026 (indoor 12026) | nav `seasons`, meet `SeasonID` | season key, not a year string |
| `EventID` / `IDEvent` | 1 | rankings row / bio `eventsTF` | event key |
| `EventShort` | `100m` | rankings row + rankings POST body | the request key for rankings and per-event results |
| `IDMeetDiv` / `IDDiv` | 1366830 / 1 | meet `tfDivisions[]` | meet-scoped division (Varsity / Wheelchair) |
| `IDDivision` | 170771 (WI Division 1), 172497 (Big Eight) | `GetDivChildren.divInfo`, teamnav `customDivisions` | class/section/conference node |
| `IDBaseDiv` | 709 (WI D1) / 638 (WI state) | `GetDivChildren.divInfo[0]`, rankings `division.BaseDivID` | base-division identity of a node |
| `IDCalendar` | 7466322 | allresults `teams[0]` | per-season calendar behind a team page |
| `LiveID` | 73767 | meet `meet.LiveID` | **cross-source key into AthleticLIVE** |

## 7. Pagination

* `GetRankings` is the only paginated endpoint. `qParams.page` is 1-based; `settings.depth` is the page depth
  (**100** for a single-event list, **10** for the multi-event list) and `minCount` is the server's row count
  for the current filter — the pagination driver.
* Measured pages: page 2 of WI boys outdoor 100m = **102 rows, ranks 100–201** (2 rows beyond depth, tie spill);
  page 17 of the grade-11 filter = **17 rows** (last page, `minCount 1616` → `ceil(1616/100) = 17` pages).
  Because of tie spill the exact rows/page is not constant, so a crawler must page until `rows < depth` or the
  rank exceeds `minCount`, not assume a fixed row count.
* Multi-event mode (`eventShort: ""`, `qParams: {}`) returns 18 groups × depth 10 = **180 rows in one request**
  (`samples/har1-getrankings-wi-170770-m-multievent-n180.json`) — the cheapest way to see which events a state
  list carries.
* `GetDivChildren` ignores a `page` param: `divId=170770&page=2` returned a **byte-identical** payload to
  `divId=170770` (both sha256 `d44148702207693b`).
* Whole-meet pulls are unpaginated (all 49 event/division/round blocks in one 565,387 B response).
* `GetAthleteBioData` is unpaginated (complete career).

## 8. Athlete fields

From `samples/live-getathletebiodata-28872883-2026-09-20.json` (30,285 B, top-level keys: `athlete`,
`resultsTF`, `resultsXC`, `meets`, `allTeams`, `allSeasons`, `eventsTF`, `grades`, `relayTeamMembers`,
`distancesXC`, `synonyms`, `photos`, `unattachedCalendars`, `level`, `canEdit`, `hasOtherSport`):

* identity: `athlete.IDAthlete`, `FirstName`, `LastName`, `Gender`, `SchoolID`, `Handle`, `PhotoUrl`,
  `UsatfId` (present-but-null for this athlete), `isClaimed`, `age` (null here).
* per result: `IDResult`, `AthleteID`, `Result` (`"23.33a"`), `SortInt`/`SortIntRaw` (23,330 = 23.33 s in
  hundredths), `FAT`, `Place`, `Round`, `Wind` (null here), `SeasonBest`, `PersonalBest`, `Division`,
  `DivisionShort`, `SchoolID`, `EventID`, `EventTypeID`, `MeetID`, `SeasonID`, `ResultDate`, `shortCode`.
* meet map: `meets["610825"] = {IDMeet, MeetName, EndDate}` — name + date without a second request.
* grade: `grades = {"3204_2026": 11}` (school-membership + season → grade), `allSeasons[].IDSeason/Display`.
* name variants: `synonyms[]` (the retained HAR capture had one entry `{IDAthlete, FirstName, LastName}`;
  the live re-capture returned `[]` — the field is volatile, see §22).
* relay roster: `relayTeamMembers[] = {TeamID, AthleteID, Name, SortID}` (relay team id + leg order).
* **No coach, staff, AD or contact field exists in the payload** (verified absence).

Two independent captures of the same athlete agree exactly: 53 results, the same 53 `IDResult` values,
**0 field-level differences**, but a **different row order**, plus the `synonyms` delta above
(`samples/build-schema.py` compares the two; both files are in `samples/`). Ordering and `synonyms` are
therefore not stable, ids and marks are.

## 9. Meet fields

`samples/live-meetdata-634313.redacted.json` (18,790 B, 87 keys on `meet`):

* identity/geo: `ID`, `Name` (`Big 8 Conference`), `SeasonID` 2026, `MeetDate`/`StartDate`/`EndDate`,
  `LocationID` 93244, `Website`, `OwnerID`, `LiveID` 73767 (AthleticLIVE), `LevelMask`, `HasResults`.
* structure: `Sessions`, `ShowEvents`, `HasRounds`, `FieldMeasure`, `MaxUsedEventNum`, `MaxAthleteEvents`,
  `RelayAthletesRequired`, `RelayAlternates`, `EventEntriesPerTeam`, `RelayEntriesPerTeam`, `TeamScores`,
  `LockSeeds`/`LockSeedsTS`, `ScratchEnd`, `PublicEntries`, `RegStart`/`RegEnd`, `AcceptOverrides`.
* registrations: `meetRecordSetInfo` (object, redacted capture), `teamScores[]`.
* `GET /api/v1/Meet/GetEventDivisionData` adds `events[]` (per-meet event slots:
  `{Entry, Event, EventShort, Gender, ID, Measure, Result, Type, isHurdle, sort}`) and `tfDivisions[]`
  (meet-scoped divisions with entry limits; IDs `IDMeetDiv` 1366830 → `IDDiv` 1 Varsity, 1366831 → 2 Wheelchair).
* `GET /api/v1/Meet/GetAllResultsData` adds `teams[]` (12 rows / 11 schools for this meet) with
  `IDSchool`, `SchoolName`, `TeamCode`, `IDDivision`, `DivName`, `DivGender`, `IDCalendar`, `HasResults` —
  the school → meet-division membership map; `eventDivsWithResults[]` (`{e, d, t, pE, ge_p, cE}` = event,
  division, round, group-parent, …), `correctedResultIDs[]` (4 ids on this meet), `rounds[]`, `eventTypes[]`.

## 10. Result fields

`GetAllResultsData` (whole meet, 565,387 B, **758 individual rows in 49 event/division/round blocks + 288 relay
legs**): every row is flat and carries `IDResult`, `AthleteID`, `FirstName`, `LastName`, `Grade`, `AgeGrade`,
`TeamID`, `SchoolName`, `Result` (`"10.41a"`), `SortInt`/`SortIntRaw`, `Place`, `Round`, `EventID`,
`EventTypeID`, `IDDiv`, `Official`, `PersonalEvent`, `pr`/`sr` flags, `Score`, `ShortCode`, `dis*` display
strings, `MediaCount`.

**The whole-meet payload does not carry `Wind`, `Heat` or `HeatPlace`** — verified absent from all 758 rows.
`POST /api/v1/Meet/GetResultsData3` (one event × division × gender per request) does carry them
(`Wind: 1.0`, `Heat: 1`, `HeatPlace: "1"` in `samples/live-resultsdata3-100m-wind-615580.json`), plus `Handle`,
`SourceId`, `TeamMascot` and `isOwned`. Wind-corrected or heat-placement data therefore costs the per-event
path; see §16 for the request arithmetic.

Relay handling in both paths:

* whole meet: `relayLegs[] = {AthleteID, ID, Name, ResultID, ShortCode, ShortDesc, split, SortIntRaw}` —
  288 legs, 70 relay teams, 72 distinct result ids; 214 distinct leg athletes of which **45 appear only as legs**
  (so the meet's athlete union is **541 + 45 = 586**).
* rankings: relay rows are the 40 rows with `GradeID == 99`; each maps to `relayTeams[<IDResult>] =
  {IDResult, RelayTeamID, Members[{SortID, IDAthlete, AthleteName, Handle, GradeID}]}` and
  `AthleteID == RelayTeamID` on 40/40 rows. The row's `AthleteName` is an HTML-ish join of the legs
  (`"Presley Bencz<BR>Ryan Heiman<BR>Anthony Vetta<BR>Trey Resch "`, trailing space included).
* per-event: `relayLegs` (32 rows for one 4x100m: 8 teams × 4 legs) plus the relay result row whose
  `AthleteID` is again the relay team id and whose `FirstName` holds the `<BR>`-joined legs.

Grade evidence in results: `Grade` is a **string** (`"11"`, `"12"`, `"-"`), `AgeGrade` repeats it; **71 of the
758 meet rows publish `"-"`**, i.e. no grade evidence at the source for ~9% of that meet's rows.

## 11. Grade/class evidence

* **Rankings rows**: numeric `GradeID` (9–12 and the 99 sentinel) plus `Age`. Measured on one session page:
  grade 12 ×61, 11 ×24, 10 ×14, 9 ×3 (`samples/har1-getrankings-wi-170770-m-100m-p2-n102.json`).
* **`GradeID == 99` is overloaded** and must not be read as a grade: it is the relay placeholder (verified
  40/40 against `relayTeams`) **and** the marker on blurred rows in the anonymous view (96 rows, all with
  `AthleteID: 0`). The reliable discriminators are `blurred == true` / `AthleteID == 0` for masking and
  membership in `relayTeams` for relays.
* **Grade filters are server-side**: `qParams.grades: [11]` returned only grade 11 (plus the 99 relay rows).
  `POST /api/v1/tfRankings/GetAgesGrades` maps a division's grades to counts in one request (grade 9: 22,934;
  10: 21,535; 11: 19,725; 12: 16,436 for WI boys HS 2026) — a cheap pre-sizing primitive. Whether `GradeCount`
  counts rows or athletes is **unverified**.
* **Class/division evidence** comes from the division tree, not the result rows: `GetDivChildren.divInfo` gives
  the state → class/section/conference nodes with ids and `IDBaseDiv`; the team payload gives a school's whole
  ancestor chain for a season (`samples/har2-teamnav-3204.json` → `divisions[]`: US 167952 / HS 168416 /
  Wisconsin 170770 / Division 1 170771 / Sectional 2 170775 / Regional 2A 170776, plus `customDivisions`
  Big Eight 172497); meet team rows give the meet's division node per school (`IDDivision` 170776 for the
  captured regional meet). Whether a school's class label matches its *state-association* class is a cross-source
  question for the association lanes.
* Grade is **not** derivable from the team payload for a meet you have not pulled; it requires either rankings
  rows or meet rows.

## 12. Coach/contact fields

**None.** No staff, coach, AD, email or phone field exists anywhere in the captured payloads
(athlete bio, team nav, meet data, event division data, all-results, per-event results, division children,
breadcrumbs, states/countries). `meet.contactInfoExists` is a boolean on the meet payload but the data itself
was not returned. Coach/contact coverage must come from the association/directory lanes; Athletic.net can only
contribute school identity (`IDSchool`, `SchoolName`, `TeamCode`) as a join key.

## 13. Public API availability

Public JSON API under `/api/v1/**`, no key, no documented quota. Three access tiers were measured this wave:

| Tier | Endpoints verified | Result |
|---|---|---|
| Anonymous | `SiteHeader/GetDivChildren`, `public/GetStatesCountries2`, `SiteHeader/GetBreadcrumbs`, `Meet/GetMeetData`, `Meet/GetEventDivisionData`, `Meet/GetAllResultsData`, `Meet/GetResultsData3`, `AthleteBio/GetAthleteBioData`, `TeamNav/Team`, `General/GetRankings` | HTTP 200 with a plain `curl` + browser UA (`samples/atn-probe-*`) |
| Anonymous, but **degraded** | `tfRankings/GetRankings` | 200, but rows past `settings.blurAfterDepth = 5` are masked and `qParams.page` is ignored (§15) |
| Session-bound | `tfRankings/GetNavInfo` | **403** without the SPA's `anettokens` (`aud=jwtTFTopReport`) + `anet-site-roles-token` + `anet-appinfo` + session cookies `[CITED]` |
| Role-entitled | `tfRankings/DownloadRankings` | 403 anonymous `[CITED]` (bulk export; not re-tested here) |

Every API response sets an `ANETSettings` session cookie and is `cache-control: no-store`; the UI issues a
server-minted `jwtTFTopReport` inside `GetRankings` responses (masked in our samples, never persisted).

## 14. Static file availability

No static bulk files: no sitemap line in `robots.txt` (`samples/atn-robots.txt`, 1,080 B), no CSV/JSON dump
endpoint captured, no public data directory. The only "static" artifacts are the SPA bundles. The one bulk
export that exists (`DownloadRankings`) is role-entitled and anonymous-403.

## 15. Browser requirement

**Required for discovery; NOT required for results.** Measured split (fresh 3-request anonymous probe this
session, `samples/probe-anon-meet.py`, raw bytes `samples/anon-{meetdata,eventdiv,allresults}-634313.json`,
all HTTP 200 with browser-grade headers and no cookie jar):

* The **entire whole-meet pull runs anonymously**: `Meet/GetMeetData` mints its own `jwtMeet` in the public
  response (255 chars, present in `samples/anon-meetdata-634313.json`), and echoing it back as `anettokens`
  authorizes `Meet/GetEventDivisionData` (8,341 B, 36 events) and `Meet/GetAllResultsData` (506,882 B, 758
  result rows, 288 relay legs, 12 teams, 45 `eventDivsWithResults`). No login, cookies, CAPTCHA or browser.
  The fresh capture is structurally identical to the retained token-based capture
  (`samples/live-allresults-634313.redacted.json`): same 9 top-level keys, same 758/288/12/17/2/45/4 counts,
  first result row field-identical; the 58 KB byte gap is pure indentation (retained file is pretty-printed).
* Everything needed to enumerate jurisdictions, divisions and ids also works anonymously (§13 table 1).
* `GetNavInfo` (season map + full tree, 403 anonymous `[CITED]`) and unblurred `GetRankings` (pages past
  `blurAfterDepth`) need the SPA session: the rankings page must be booted in a real browser so the SPA mints
  its `anettokens`(aud=jwtTFTopReport), then the XHR streams can be replayed outside it.
* The browser is also required for anything the API does not expose as JSON: the meet calendar
  (`/events/usa/<state>/<date>` is a 7,430 B SPA shell with **0 server-rendered meet links**) and school search.

Net: the result-bearing lane (`meetId` → 3 calls) is browser-free and is what the Rust lane should implement;
the browser lane is needed only to *discover* `meetId`s and to read unblurred multi-page rankings.

## 16. Request cost

Integer request counts, each with the endpoint that produces it. Full table (including yield per request and
the citation for every row) is `coverage.json` → `requests.strategies`.

| Strategy | Endpoint(s) | Requests | Yield |
|---|---|---|---|
| Whole meet (3-step) | `Meet/GetMeetData` → `Meet/GetEventDivisionData` → `Meet/GetAllResultsData` | **3 per meet** | 758 individual + 288 relay legs (1,046 rows) for meet 634313 — re-measured anonymously `samples/anon-meet-probe-report.json` |
| Per-event fallback | `Meet/GetResultsData3` | **45 per meet** (the meet's `eventDivsWithResults`) | same rows, one event × division × gender per request, **adds Wind/Heat/HeatPlace** |
| Per-athlete profile | `AthleteBio/GetAthleteBioData` | **586 per meet** (the meet's distinct athletes) | 53 career rows for the captured athlete; 100% id-complete but 195× the meet cost |
| Rankings page (single event) | `tfRankings/GetRankings` + `page` | **1 per page** | 102 rows (depth 100 + tie spill); 17 pages for WI boys grade-11 100m (`minCount` 1616) |
| Rankings page (multi-event) | `tfRankings/GetRankings` (`eventShort: ""`) | **1 per page** | 180 rows (18 groups × depth 10) |
| Rankings, anonymous | `tfRankings/GetRankings` | 1 per request, page ignored | 101 rows of which **5 usable** (rest masked) |
| State list ids | `tfRankings/GetNavInfo` (session) | **1 per state** (51 for the country) | 52 state ids in one call, plus the selected state's 22 outdoor + 21 indoor season ids |
| Division tree | `SiteHeader/GetDivChildren` (anonymous) | **1 per internal node** — 21 for Wisconsin (level + state + 3 divisions + 16 sectionals) | 24 child rows for WI (3 + 8 + 4 + 4) |
| Grade sizing | `tfRankings/GetAgesGrades` (session) | **1 per division** | 11 grade rows (9: 22,934; 10: 21,535; 11: 19,725; 12: 16,436) |

Server-imposed parameters that set the arithmetic (all read off the captured `settings` blocks):

* single-event rankings page — `qParams.page=N`, `settings.depth = 100` → up to 100 rows (+tie spill: page 2
  returned **102**), so pages per event = `ceil(minCount / 100)`;
* multi-event rankings page — `eventShort: ""`, `settings.depth = 10` (server-forced, not requestable higher)
  → 18 groups × 10 = **180 rows in one request**, but never the deep tail;
* whole-meet results — `GetAllResultsData` needs `anettokens: jwtMeet`, which only `GetMeetData`
  mints, so the floor is 2 requests (meet object + results) and the measured metadata-complete sequence is 3.

Exact per-strategy costs:

| Ask | Endpoint + parameters | Requests |
|---|---|---|
| Results only for one meet | `Meet/GetMeetData?meetId=&sport=tf` then `Meet/GetAllResultsData?meetId=&sport=tf` | **2** |
| Results + event metadata for one meet | + `Meet/GetEventDivisionData?meetId=&sport=tf` | **3** |
| One event × one division × one gender, with wind/heat | `POST Meet/GetResultsData3?meetId=&sport=tf` body `gender:"m"` (lowercase) | **1** per event-div, **45** for meet 634313 |
| Per-athlete career history | `AthleteBio/GetAthleteBioData?athleteId=&sport=tf&level=0` | **1** per athlete, **586** for meet 634313's distinct athletes |
| Top-10 of all 18 events for one division/gender | `tfRankings/GetRankings` body `eventShort:""` (1 page) | **1** |
| Same 180 rows event-by-event at depth 10 | `tfRankings/GetRankings` body `eventShort:"100m"` … ×18 | **18** |
| Full depth of one event (WI boys 100m, `minCount` 2,117) | `tfRankings/GetRankings` `eventShort:"100m"` `qParams.page=1..N` | **22** = `ceil(2117/100)`; measured tail page 17 held 17 rows |

Derived trade-offs (all measured, no extrapolation):

* whole meet **3** vs per-athlete **586** → **195.3×** fewer requests for the same meet;
* whole meet **3** vs per-event **45** → **15.0×** fewer (the results payload alone: 45 vs 1);
* one multi-event rankings page **1** vs the same 180 rows event-by-event **18** → **18×** fewer, but the
  multi-event page is server-capped at `depth: 10` per event while a single-event page runs at `depth: 100`:
  the shallow scan is cheap, the deep scan is still per-event (**22** pages for one event's 2,117 rows);
* one rankings page **1** vs one profile **1**, but a page yields 102 ranked rows against 53 career rows;
* national rankings scan **4,233** vs national profile scan **142,705** → **33.7×** `[CITED]`
  (`research/midwest/04-athletic-net-rankings-discovery.md` tables 1–3, model validated within 0.6% of the
  repo's 4,256 retained receipts; not recomputed here);
* the *same* filter costs differently by tier: session `minCount` 2,117 vs anonymous 982 for WI boys 100m, and
  1,616 vs 1,108 for the grade-11 filter — record totals together with the tier that produced them.

## 17. Published rate limits

**None published and none observed.** `robots.txt` carries no `Crawl-delay` and no sitemap. No
`RateLimit-*`/`Retry-After` header appeared on any captured response (all headers are in `samples/*.headers`).
Responses are Cloudflare-fronted (`cf-cache-status: DYNAMIC` for API calls). Working self-imposed rule for this
lane: **≤1 request/second/host**, which is what the probe captures in `samples/CAPTURES.md` were taken at.
No throttle, 429 or `Retry-After` was observed at that rate (21 requests total across this lane's probes;
the fresh probe re-confirms it: 3 sequential requests at 1.2 s spacing, all 200 — `samples/anon-meet-probe-report.json`).
The corpus asserts no measured throttle either; the only related instruction is the mission brief's standing
"honor `Retry-After`, record published limits and any observed 429 behavior" rule.

## 18. Known blocks

* `GetNavInfo` and the role-entitled `DownloadRankings` are **403** to plain clients (`[CITED]`); the nav path
  is the only way to get a state's season map and full tree in one call.
* Blurring: anonymous ranking rows past rank 5 are masked (see §15/§11); the mask is signalled in-band by
  `settings.blurAfterDepth` and per row by `blurred`/`AthleteID: 0`.
* `robots.txt` (`samples/atn-robots.txt`) disallows `/TrackAndField/State/`, `/CrossCountry/State/`,
  `/trackandfield/print/`, `/trackandfield/report/`, `/search.aspx`, `/Admin/`, `/Account/`, `/User/`,
  `/Edit/`, `/Partner/`, `/Secure/`, `/home/`, plus several legacy `.aspx` paths. The `/api/v1/**`,
  `/TrackAndField/rankings/list/**`, `/TrackAndField/meet/**`, `/team/**`, `/athlete/**` and `/events/**`
  paths we use are **not** disallowed. `robots.txt` last-modified 2026-03-15, cached `max-age=31536000`.
* Captcha/paywall: none encountered. The only paywall-like wall is the entitlement blur, which is bypassed by a
  normal signed-in browser session — no credential sharing or authentication bypass was attempted.

## 19. Cross-source join keys

| Key type | Outbound (what Athletic.net gives) | Inbound (what other sources hand us) |
|---|---|---|
| Meet | `MeetID` → `/TrackAndField/meet/<id>[/results/all]`; `LiveID` → AthleticLIVE | association pages link `/TrackAndField/meet/<id>` directly; AthleticLIVE documents carry `ani` |
| Team/school | `TeamID` → `/team/<id>/<sport>/<season>`; `IDSchool`, `TeamCode`, `SchoolName`, `City`/`State` via team nav | association directories give school name + city + state (no id) |
| Athlete | `AthleteID` → `/athlete/<id>/track-and-field`; `Handle` slug | AthleticLIVE athlete documents carry `ani` |
| Result | `shortCode` → `/result/<code>` | nobody external publishes `shortCode` |
| Names | `AthleteName`/`FirstName`+`LastName`; `synonyms[]` | MileSplit row data is (name, school, state, grad year) with **no** Athletic.net id on the site side; the join already exists as an artifact, not as a live lookup — **69,819** rows of `data/athleticnet-athlete-seeds.csv` carry both an `athleticnet_athlete_id` and a `milesplit_athlete_id`, and **56,842** rows of `data/canonical-athletes-co2027.csv` do (verified 2026-09-21). Direct name-matching inside this source is still unproven: name+school is not unique in Athletic.net (`synonyms[]` exists precisely because athletes get renamed/merged) |

The `LiveID` field is the cleanest Athletic.net → AthleticLIVE join and was observed on the meet payload
(73767 for meet 634313); it should be persisted on every meet record.

## 20. Estimated marginal coverage

* **Jurisdictional**: complete by construction — 51/51 jurisdictions enumerable in one session-bound call, and
  the state list is the *only* place in this source where a USPS code is published as a first-class node
  attribute. This makes Athletic.net the natural **national spine** for the census even where state
  associations are richer.
* **Per-state depth is not uniform.** Measured anchors: Wisconsin (one selected region) carries 3 class
  divisions / 16 sectionals / 48 regionals for 2026 (`tree[]`, 68 nodes); its boys 100m open list holds 2,117
  rows and the grade-11 slice 1,616 rows. Texas's state node has 332 team accounts and 8 association children;
  California's has 10 children and 13 team accounts. How many meets/athletes each state contributes was **not**
  measured for any state, so any per-state total is `[INFERENCE]`.
* **Marginal over the 12 Midwest states**: Athletic.net already carries those states' meets. Two independent
  provenances: (a) `data/athleticnet-meet-seeds.csv` — every one of its 9,844 rows has
  `source = athleticlive-meet-harvest`, and **9,747** carry an AN meet id + AN URL + AthleticLIVE ids
  (IL 2,159 / MI 1,433 / OH 1,232 / IA 1,004 / MN 987 / WI 777 / NE 589 / MO 569 / IN 381 / SD 369 / KS 182 /
  ND 162); **471** of those rows are typed state/sectional/regional/district and their names are association
  meets (e.g. `IHSAA Girls State Track & Field Finals`, `MSHSAA Class 4 District 1 & 5 District 1`);
  (b) association pages themselves — MI MHSAA hands over 215 TF + 117 XC meet ids. So Athletic.net's marginal
  value in the Midwest is union-completion and national normalisation, not discovery. Caveat: provenance (a)
  reaches those meets *through* AthleticLIVE, so it does not show that the association pages link them
  (association-page linking is measured only for MI and the peer lanes).
* **Marginal outside the Midwest**: this lane's evidence shows the *container* (ids, lists, tree) exists for all
  51 jurisdictions; no capture yet proves result density for any non-Midwest state. Treat non-Midwest TF
  coverage as `[INFERENCE]` until one non-Midwest meet is pulled with the 3-request method.
* **Scale already in hand**: the athlete-seed artifact holds **189,705** rows / **167,277** distinct Athletic.net
  athlete ids over the 12 Midwest states, and the retained delivery workbook covers 142,705 national grade-11
  athletes (`[CITED]` `data/atn-id-fields.csv`) — so the national ceiling is at least the
  `4,233`-request rankings scan measured in `research/midwest/04-athletic-net-rankings-discovery.md`.
* **Primary use**: national identity spine (jurisdiction + school + meet ids) and ranked-result evidence; the
  state associations remain the better source for entry lists, assignments and per-state class semantics.

## 21. Cross-source handoff (what each peer source can hand the browser lane)

The browser lane never has to search if it consumes these handoffs. Every row below is verified against a file
on disk in this session; corpus-relative paths are under `~/Downloads/midwest-tfxc-source-research/`.

### 21.1 Verified id handoff artifacts (already on disk, no fetching needed)

| Artifact | Rows | What it carries | Verified fact |
|---|---|---|---|
| `data/athleticnet-meet-seeds.csv` | 9,844 | `athleticnet_meet_id`, `athleticnet_url`, `athleticlive_meet_ids`, `tenants` | **9,747** rows carry a non-empty AN meet id **and** an AN URL **and** AthleticLIVE meet ids — the complete 12-state meet join, by state: IL 2,159 / MI 1,433 / OH 1,232 / IA 1,004 / MN 987 / WI 777 / NE 589 / MO 569 / IN 381 / SD 369 / KS 182 / ND 162. Provenance of every row: `source = athleticlive-meet-harvest` |
| `data/canonical-meets.csv` | 9,667 | `athleticnet_meet_id` (+ `source_identities`, `evidence_sources`) | **9,593** rows carry an AN meet id; 9,562 distinct. The `athleticnet_url` column exists but is **empty** in this artifact — take URLs from the meet-seeds file above |
| `data/athleticnet-athlete-seeds.csv` | 189,705 | `athleticnet_athlete_id`, `athleticnet_url`, `name`, `grad_year`, `gender`, `state`, `school_name`, `city`, `sports`, `derived_from_sources`, `milesplit_athlete_id` | 189,705 rows all carry an AN athlete id; **167,277 distinct** ids; **69,819** rows also carry a MileSplit athlete id; sources = `athleticlive_athletes` + 10 MileSplit state tenants; 12 Midwest states |
| `data/canonical-athletes-co2027.csv` | 183,875 | `athleticnet_athlete_id` + `milesplit_athlete_id` + `profile_urls` | **93,619** rows carry an AN athlete id; **56,842** carry **both** AN and MileSplit ids — the measured cross-source join |
| `data/athleticnet-meet-seeds.csv` → `level` | — | meet level classification | 9,844 rows typed: invitational 3,991 / conference 475 / sectional 152 / dual 140 / regional 131 / district 118 / state 70 / unknown 4,767 (the `unknown` share is the gap this census closes) |

### 21.2 Per-source handoff (measured, by peer lane)

| Source | Hands over | Measured instance |
|---|---|---|
| State association sites (state-assoc lanes) | Athletic.net **MeetID** as a result link; sometimes a rankings list URL | MI MHSAA: **215** distinct TF + **117** distinct XC meet ids across 13 association pages, 0 overlap (`research/midwest/evidence/gaps/39/mhsaa_an_meetids_by_page.json`; 521 AN hrefs / 505 unique in `.../39/mhsaa_an_link_inventory.json`). Per-state instances for IL/OH/MN/NE/SD/WI are in the peer lane reports under `research/sources/state-assoc-*/SOURCE_REPORT.md` (landing in this wave) — this lane does not restate ids it did not open |
| Timing providers on AthleticLIVE | AN **MeetID** (`ani`), AN athlete id (`ani`), AN team id (`t.ani`) | the 9,747-row join above is the aggregate proof; in-repo the adapter already builds `https://www.athletic.net/TrackAndField/meet/{an_id}/info` (`crates/census-service/src/sources/athleticlive/meets.rs:45`, fixture `.../athleticlive/tests.rs:98` → `meet/259955/info`) |
| Timer sites (upcoming-meet tables) | AN MeetID + name/date/director per upcoming meet | trxctiming.com (MO) upcoming table: **40** meet rows → **39** AN links → **38** distinct URLs → **37** distinct meet ids (36 CrossCountry + 1 TrackAndField + 2 non-meet links), one page (`research/midwest/evidence/gaps/40/upcoming-table.json`) |
| MileSplit (milesplit-national lane) | a (name, school, state, grad year) tuple **plus** the 69,819 pre-joined athlete ids above; no AN ids from the site itself | the seeds file's `milesplit_athlete_id` column is the usable handoff; direct AN-side matching is not evidenced |
| Coach/directory lanes | school + city + state + sport + role; no AN ids → resolve `IDSchool`/`TeamID` once and cache | `[INFERENCE]` (no capture yet ties a directory row to an AN id) |
| National aggregators | unverified — no capture ties an aggregator page to an AN id | — |

### 21.3 Ready-to-use handoff (so the browser lane never searches)

| If the batch hands us | The browser lane loads (no search step) |
|---|---|
| an Athletic.net **MeetID** (from association pages, AthleticLIVE `ani`, timer tables, or either seeds CSV) | `GET /api/v1/Meet/GetMeetData?meetId=<id>&sport=tf` → `/api/v1/Meet/GetAllResultsData?meetId=<id>&sport=tf` (browser-free); human URL `https://www.athletic.net/TrackAndField/meet/<id>/info` |
| an **AthleticLIVE** meeting id or timer tenant | the seeds CSV row → `athleticnet_meet_id` + `athleticnet_url` columns; nothing to resolve |
| an **AthleteID** (189,705 rows in the athlete-seeds file) | `GET /api/v1/AthleteBio/GetAthleteBioData?athleteId=<id>&sport=tf&level=0`; human URL `/athlete/<id>/track-and-field` |
| a **MileSplit athlete id** | the same athlete-seeds / canonical rows give the paired `athleticnet_athlete_id` (69,819 / 56,842 verified pairs) |
| a **school name + state** (coach/directory lanes) | resolve once via `SiteHeader/GetDivChildren` on that state's node (`teams[]` carries `IDSchool`), then reuse `TeamID` — `IDSchool` values seen: 101718, 57750, 3204 |
| a **USPS state code** | the nav `divListId` for that state (all 51 present in `coverage.json` → `jurisdiction_coverage.entries`), then `/TrackAndField/rankings/list/<divListId>/<m/f>` |
| a **division/class label** (e.g. "WI Division 1") | the state node's children from `GetDivChildren` (3 class divisions for WI, ids in `samples/atn-probe-divchildren-170770.json`) |

Nothing in this table requires a search, a sitemap walk, or a calendar scrape — that is the point of the lane.

### 21.4 URL grammar the browser lane should use (all verified in captures)

```
meet      https://www.athletic.net/{TrackAndField|CrossCountry}/meet/<MeetID>[/info|/results|/results/all|/entries]
team      https://www.athletic.net/team/<TeamID>/<track-and-field-outdoor|track-and-field-indoor|cross-country>/<SeasonID>
athlete   https://www.athletic.net/athlete/<AthleteID>/track-and-field
result    https://www.athletic.net/result/<shortCode>
rankings  https://www.athletic.net/TrackAndField/rankings/list/<divListId>/<m|f>[/<EventShort>]?grades=<n>&page=<n>
calendar  https://www.athletic.net/events/usa/<state-slug>/<YYYY-M-D>     (SPA shell; needs the browser)
```

API call sequence for the browser lane (session-bound calls last): `SiteHeader/GetDivChildren` → division tree
(anonymous) → `tfRankings/GetNavInfo` (session; per state) → `tfRankings/GetRankings` (session; pages) →
`Meet/GetMeetData` → `Meet/GetAllResultsData` (jwtMeet in the same response) → `Meet/GetResultsData3`
(only when Wind/Heat is required).

## 22. Open questions (unverified, with the follow-up that would settle each)

1. **School roster per state** — no captured endpoint returns it; `GetDivChildren.teams[]` is a capped
   team-account list (0/1/5/13/150/332 measured, `page` ignored, empty for sectionals) and the nav's
   `Sectional → Schools` typing is not backed by an endpoint. Follow-up: open `/TrackAndField/division/<id>` in
   the browser and log the `/api/v1/**` calls.
2. **Meet calendar API** — the state events page is a shell with 0 server-rendered ids. Follow-up: same
   browser capture on `/events/usa/wisconsin/<date>`.
3. **Anonymous vs session totals** — the same filter reports 982 (anonymous) vs 2,117 (session) and 1,108 vs
   1,616. Which count a census should trust is unresolved.
4. **Grade semantics of `GetAgesGrades`** — rows or distinct athletes?
5. **Blur rule generality** — verified only at `blurAfterDepth = 5` for one list/event; unverified for
   girls/indoor/XC or for a token with non-empty `userRoles`.
6. **`GetDivChildren.teams[]` cap** — 150 (WI) looks like a cap, 332 (TX) does not; unverified.
7. **`synonyms[]` volatility** — present in the retained HAR capture, `[]` in the live re-capture (66 B vs 2 B).
8. **Girls / indoor / XC** — no capture in this lane; the skeleton of the season map (`12026`) and the same
   endpoints are expected to apply `[INFERENCE]`.
9. ~~**`GetEventDivisionData` skippability**~~ — **settled 2026-09-21**: step 3 of the anonymous probe was issued
   with only the `jwtMeet` from step 1 (never with step 2's output) and returned the complete payload
   (`flatEvents` 49, 758 rows, `relayLegs` 288, its own `eventTypes[]`). So the results lane is **2 requests per
   meet**, and `GetEventDivisionData` (8,341 B) is needed only for the per-event metadata that
   `GetAllResultsData` does not carry (`events[].isHurdle`, `FieldMeasureType`, `Type`) — evidence
   `samples/anon-meet-probe-report.json`, `samples/anon-allresults-634313.json`.

## 23. Implementation recommendation

**PRIMARY** — as the national identity spine and ranked-result source.

Justification from the measured evidence: 51/51 jurisdictions enumerable in one session-bound call and mapping
exactly onto `UsJurisdiction`; every entity (meet, school/team, athlete, result, event, division, season) has a
stable integer id and a URL grammar; a whole meet costs 3 requests for ~1,000 rows including relay legs and
per-row grade; per-request yield is 2–3 orders of magnitude better than profile walking; and the endpoint tier
that needs a browser is precisely the tier that also mints the season/state matrix the census needs anyway.

Conditions on "PRIMARY": (a) all census-affecting reads that need grades/ranks beyond rank 5 must run in the
session tier; (b) `Overseas (170162)` must be filtered as out of scope rather than coerced into a jurisdiction;
(c) `GradeID == 99` must never be read as a grade — check `blurred`/`AthleteID == 0` and `relayTeams`
membership instead; (d) totals must be recorded with the tier that produced them; (e) `LiveID` should be
persisted on meets for the AthleticLIVE join.
