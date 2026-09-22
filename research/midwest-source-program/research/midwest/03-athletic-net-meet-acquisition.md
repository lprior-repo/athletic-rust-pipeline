# 3. Athletic.net meet acquisition

Status: complete — every endpoint named in the assignment is qualified from captured client code; no captured
*response body* exists for any meet endpoint (neither HAR contains a meet request), so all meet response shapes
are marked `derived` (from the client code that consumes them) and are not response-verified. Section
"Evidence gaps and the exact capture that closes them" lists the four captures required.
Observed on: 2026-09-19

Scope of evidence (all read-only):

| Artifact | What it is | Window (UTC) |
|---|---|---|
| `/home/lewis/Downloads/www.athletic.net.har` | 722 entries; document = `https://www.athletic.net/TrackAndField/rankings/list/170770/m` (Wisconsin, boys); 30 `/api/` calls | 2026-09-18T18:54:41.860Z – 18:55:40.818Z |
| `/home/lewis/Downloads/www.athletic.net2.har` | 167 entries; document = `https://www.athletic.net/athlete/28872883/track-and-field`; 11 `/api/` calls | 2026-09-18T19:20:59.690Z – 19:21:18.194Z |
| Retained Angular bundle `chunk-BXK99FuV2.js` (inside HAR #1, entry 58, 12,998 B) | Full `Meet` service class: every meet/result/entries/divisions call site | captured 2026-09-18 |
| Repo docs + fixtures (read-only) | `README.md`, `SCOPE.md`, `src/profile/**`, `tests/fixtures/rankings/*` | as of 2026-09-19 |

The decisive negative: **`/api/v1/Meet/*` request count in both HARs = 0.** No meet page was ever captured, so
nothing about meet payload size, division counts, or per-meet row counts is measured here; those are the named gaps.

### Source

Athletic.net, Inc. — `https://www.athletic.net` (Angular SPA + private JSON API `/api/v1/**`), with its
live-results sibling **AthleticLIVE** at `https://live.athletic.net` and `https://api.athletic.live` (a different
host, different Cloudflare posture — see Access characteristics).

API surface is not publicly documented; the route/service inventory below was extracted from the Angular bundles
that the two HAR captures retained (`angular.athletic.net/app/site-app/*`). Meet UI route entry points observed in
the route table of `main-M36VMDMH.js`:

| Route | Lazy chunk | Meaning |
|---|---|---|
| `TrackAndField/meet` | `tf-meet.routes-DAPMe4DX.js` | TF meet shell (not captured) |
| `CrossCountry/meet` | `xc-meet.routes-QTulWKwk.js` | XC meet shell (not captured) |
| `track-and-field-outdoor` / `track-and-field-indoor` / `cross-country` | `division-home.routes-C9Tno7ix.js` | division home (state/season index, not captured) |
| `events` | `events.routes-DXP3eD-y.js` | candidate public meet index (not captured) |
| `athlete` | `athlete-bio.routes-BYYQfvXR.js` | athlete profile |
| `team` | `team-home.routes-D9MAu-sB.js` | team home |

Human-readable URL patterns (strings in the bundles, verified): meet `/TrackAndField/meet/<MeetID>`;
XC meet `/CrossCountry/meet/<MeetID>`; meet result deep link `/TrackAndField/meet/<MeetID>/results/<gender>/<divId>/<EventShort>`
(built in `listenForResultUpdates`), XC `/CrossCountry/meet/<MeetID>/results/<idMeetDiv>`; team scores
`/TrackAndField/meet/<MeetID>/teamscores/`; athlete `/athlete/<AthleteID>/track-and-field|cross-country`;
team `/team/<TeamID>/track-and-field-outdoor|track-and-field-indoor|cross-country/<SeasonID>`.

### Coverage

- **Sports:** boys/girls track & field (indoor + outdoor) and cross-country. TF and XC are separate meet shells
  (`sport1` = `tf` | `xc`) with separate result payload keys (`resultsTF` / `resultsXC`).
- **States:** all 12 target states are first-class divisions. `tfRankings/GetNavInfo` returns `tree[68]` for the
  captured Wisconsin list; the same call with another state's `divListId` yields that state's tree. Observed ids:
  `168416` = High School, `170770` = Wisconsin, `170771` = Division 1, `170775` = Sectional 2,
  `170776` = Regional 2A - Sauk Prairie, `173005` = 2026 indoor, indoor `SeasonID = 12026`.
- **Levels:** `level=4` (high school) is the parameter the live capture used; the nav tree also carries
  `levels[6]` (Country → Level → State → Conference/Class → …).
- **Seasons:** meet pages are single-edition; `division.ID` values are season-scoped (2026 list `168416`).
  Historical depth is per meet: `GetMeetHistoryAndRecordSettings` returns a `history[]` array of prior editions
  (`history[i].SeasonID`, normalized client-side as `SeasonID % 1e4 = year`), and the captured athlete's `meets`
  map spans 2025 and 2026 (`EndDate` 2025-05-01 … 2026-06-06).
- **Not covered by this source path:** no roster/participation API for meets that never posted results.

### Enumeration

Enumerate **meets** — four channels are qualified, two of them verified end-to-end from captured payloads:

1. **Rankings rows carry the meet directly (verified).** Every row of `tfRankings/GetRankings` contains
   `MeetID` and `MeetName`. Measured in HAR #1: 707 parsed rows → **314 distinct `MeetID`, 668 distinct
   `AthleteID`, 294 distinct `TeamID`**; `MeetID`/`MeetName` each occur 180× in the single 191,003-byte
   all-event response. So a division×event×gender rankings sweep *is* a meet-discovery sweep, at no extra request.
   Row fields: `IDResult, AthleteID, AthleteName, GradeID, EventID, EventTypeID, Event, EventShort, TeamID,
   TeamName, State, Country, MeetID, MeetName, display, SortIntRaw/Calc/Orig, Round, FAT, PersonalBest,
   ResultDate, SeasonID, rank, rowNum, shortCode, Wind`(event-dependent), `isFieldSeries`.
2. **Athlete bio `meets` map (verified).** `AthleteBio/GetAthleteBioData?athleteId=&sport=tf&level=0` returns
   `meets: { "<IDMeet>": {IDMeet, MeetName, EndDate} }` plus `resultsTF[]` rows carrying `MeetID`. Sampled
   athlete: **18 meets (7 in 2025, 11 in 2026), all 18 referenced by the results array**. One request per athlete
   yields that athlete's whole meet-ID set.
3. **Division/season index (verified for the index; postseason seed is partial).** `SiteHeader/GetDivChildren?sport=tf&divId=<divId>`
   returns `divInfo[]` (child divisions with `IDDivision`, `DivName`, `DivIDDivision`, `Level`, `SeasonID`) and
   `teams[]` (`IDSchool`, `SchoolName`). Observed for `divId=170771` (Division 1): 8 Sectionals + 1 team row.
   The `GetNavInfo` tree (68 nodes for Wisconsin) is structured `Division N` → `Sectional N` → `Regional XA/B - <host school>`;
   48 of the 68 nodes are host-school-named regionals. Postseason divisions are therefore named after their meet,
   and the join is **exact for Regional meets only, 2/2 on the sampled athlete's regionals**:
   division `170776 Regional 2A - Sauk Prairie` = meet `667436 D1 Regional 2A - Sauk Prairie`, and
   division `170777 Regional 2B - Madison Memorial` = meet `613273 D1 Regional 2B - Madison Memorial` (both names
   normalized for the `D1 ` class prefix). Do **not** overclaim this channel: Sectional divisions are named
   `Sectional 2` with no host suffix (three different ids share that name — 170775/170802/170823), so a Sectional
   meet name like `D1 Sectional 2 - Verona` joins only by ambiguous prefix; and the two state-championship meets
   in the sample have **no** division at all (the tree stops at Regional; `/TrackAndField/State/` is even
   robots-disallowed). Net: this seed covers 2 of the 18 sampled meets exactly, and costs no extra request beyond
   the division tree you already walk for rankings.
4. **Meet index page (route exists, content unverified).** `/events` → `events.routes-DXP3eD-y.js`. The route table
   is real; the page's XHR is unknown. **No meet-calendar API endpoint appears anywhere in the captured client
   code** — the full `/api/v1/**` inventory extracted from the bundles contains no `GetCalendar`, `GetMeets`,
   `MeetList` or date-range meet search. The breadcrumb service accepts `meetId`/`calendarId` but only echoes nav
   state (`GetBreadcrumbs?sport=tf&athleteId=0&meetId=0&calendarId=0&divId=170770&teamId=0&seasonId=0`).
   A one-time capture of `/events` (or a division home) is required to know whether a browsable meet list exists.

Enumerate **schools/teams**: `GetDivChildren` → `IDSchool`/`SchoolName` per division; `TeamNav/Team?team=<TeamID>&sport=<tf|xc>&season=<SeasonID>`
returns team identity (`ID`, `Name`, `Level`, `hasIndoor`, `City`, `State`, `divisions[]`, `customDivisions[]`,
`linkedTeams[]`). Both are in scope of assignment 1; meet pulls reuse the same `TeamID` values.

Enumerate **athletes / Class-of-2027**: rankings rows expose `GradeID` (11 in the sampled rows) and meet result
rows expose `Grade`/`AgeGrade`; `GetAgesGrades` returns the grade dictionary. Meet pulls therefore produce a
grade-attributed athlete roster per meet, from which grade-11 (Class-of-2027 in 2026-27) athletes are derived
without an extra request.

Enumerate **results**: `GetAllResultsData` (whole meet) or `GetResultsData3` (one gender×division×event), below.

**Meet endpoint table.** All rows are under host `https://www.athletic.net`; `anettokens` = the per-meet token
`data.jwtMeet` returned by `GetMeetData` — a scope-limited JWT, not a user credential; the same mint-and-echo
pattern is verified empirically for rankings (`jwtTFTopReport` → `anettokens`, see Access characteristics).
"Response fields" is `derived` — no response body
for any of these endpoints exists in either HAR (verified: 0 requests to `/api/v1/Meet/*`); the fields listed are
the ones the shipped client code reads. "Read-only" marks calls a collector needs; the rest are admin/registration
calls listed for completeness because they appear in the same service.

| URL | method | params / body | response fields (derived) | ids carried |
|---|---|---|---|---|
| `/api/v1/Meet/GetMeetData` | GET, **no auth header attached** | `?meetId=<MeetID>&sport=<tf\|xc>` | `meet{ID, StartDate, EndDate, SetupState (JSON string), Settings (JSON string), HasResults, LiveID, DfpTag}`, `eventDivsWithResults[]`, `sport1`, `jwtMeet`, `hostIsSS` | MeetID, LiveID |
| `/api/v1/Meet/GetMeetRedirect` | GET, **no auth header attached** | `?meetId=&sport=` | redirect target (unverified) | MeetID |
| `/api/v1/Meet/GetEventDivisionData` | GET, anettokens | – | `events[{ID, Event, EventShort, Gender, Type}]`, `tfDivisions[{IDDiv, Division, …}]`, `xcDivisions[{IDMeetDiv, DivName, Gender, Entry}]` | EventID, DivisionID (`IDDiv`/`IDMeetDiv`) |
| `/api/v1/Meet/GetAllResultsData` | GET, anettokens | `?rawResults=<bool>&showTips=<bool>` (XC variant `getXCAllResultData()` sends no params) | whole-meet results (all divisions/events); keys unverified; XC sibling returns `resultsXC[]` | IDResult, AthleteID, MeetID, EventID, TeamID |
| `/api/v1/Meet/GetResultsData3` | POST, anettokens | TF body `{gender, divId, eventShort, rawResults, showTips, groupParentId}`; XC body `{divId}` | `{resultsTF[]}` / `{resultsXC[]}`; failure shape `{currentEventValid:false}`; group children via `groupParentId` | IDResult, AthleteID, EventID, EventTypeID, TeamID/SchoolID, DivisionID |
| `/api/v1/Meet/GetEventListData` | GET, anettokens | – | event list (unverified) | EventID |
| `/api/v1/Meet/GetEventMedia2` | GET, anettokens | `?gender=&divId=&eventShort=` | media rows | DivisionID, EventID |
| `/api/v1/Meet/GetEntriesData` | POST, anettokens | TF `{meetId, sport, gender, divId, eventShort, seasonId}`; XC `{meetId, sport, divId, seasonId}` | entries (declared athletes) per event/division | AthleteID, TeamID, SeasonID |
| `/api/v1/Meet/GetTeams` | GET, anettokens | – | teams entered in the meet | TeamID |
| `/api/v1/Meet/GetTeamScoreData` | GET, anettokens | – | team scores | TeamID, DivisionID |
| `/api/v1/Meet/GetSchedule` | GET, anettokens | – | meet schedule (track/field order) | EventID, DivisionID |
| `/api/v1/Meet/GetMeetHistoryAndRecordSettings` | GET, anettokens | – | `history[{SeasonID, …}]` + record settings | MeetID (lineage), SeasonID |
| `/api/v1/Meet/GetMeetRecordsForEvent` | GET | `?divisionId=&eventId=&eventTypeId=&meetStart=` | meet records for event | EventID, EventTypeID, DivisionID |
| `/api/v1/Meet/GetStandardsForEventTF` | GET | `?divisionId=&eventId=` | qualifying standards | EventID, DivisionID |
| `/api/v1/Meet/GetMeetRecordsForXCDistance` | GET | `?gender=&divisionId=&meters=&meetStart=` | XC course records | DivisionID |
| `/api/v1/Meet/GetRunSplits` | GET | `?divId=&eventId=&round=&heat=&relay=&isMultiEvent=` | run splits | DivisionID, EventID, heat, round, relay flag |
| `/api/v1/Meet/GetHorizontalFieldSeries` | GET | `?divId=&eventId=&round=&heat=&isMultiEvent=` | field series (horizontal) | DivisionID, EventID |
| `/api/v1/Meet/GetVerticalFieldSeries` | GET | `?divId=&eventId=&round=&heat=&isMultiEvent=` | field series (vertical) | DivisionID, EventID |
| `/api/v1/Meet/GetALiveDocIdTF` \| `GetALiveDocIdXC` | GET, anettokens | `?divId=[&eventId=&round=]` (`round` truncated to 1 char) | integer doc id → `https://live.athletic.net/meets/<LiveID>/<events\|xc-events\|relays>/<docId>` | LiveID, doc id, DivisionID |
| `/api/v1/Meet/GetMeetEventId` | GET, anettokens | `?eventId=&eventTypeId=&divId=` | runmeet event id | EventID, EventTypeID, DivisionID |
| `/api/v1/Meet/GetCorrectionInfo` | POST, anettokens | `{gender, meetId, sport, teamId, resultId}` | correction detail for a result | ResultID, MeetID, TeamID |
| `/api/v1/Meet/ValidateMark` / `InvalidateMark` | POST, anettokens | `{resultId}` | validation ack (mutates) | ResultID |
| `/api/v1/Meet/UAthleteDetails` | POST, anettokens | `{athleteIds:[<AthleteID>,…]}` | batch athlete details (admin/registration scope) | AthleteID |
| `/api/v1/Meet/GetMeetDataForUser` | GET, anettokens | – | per-user meet view (`isAuthenticatedWithGoodStanding` gate) | MeetID |
| `/api/v1/Meet/UserData`, `Cart`, `GetRegisterData`/`GetXCRegisterData`, `GetXCMoreData` | GET, anettokens | – | registration/cart/user state (mutating flows excluded from use) | MeetID, TeamID |
| `/api/v1/Meet/AddMeetToCal`, `AcceptInvite`, `IgnoreInvite` | GET | header `anettokens: <token from URL>` | ack only (mutating) | MeetID |

Context endpoints needed by the same flow (all captured live): `/api/v1/tfRankings/GetRankings` (yields `MeetID`
per row), `/api/v1/AthleteBio/GetAthleteBioData` (yields the `meets` map), `/api/v1/SiteHeader/GetDivChildren`
(division children + `teams[].IDSchool`), `/api/v1/SiteHeader/GetBreadcrumbs` (accepts `meetId`/`calendarId`),
`/api/v1/TeamNav/Team` (team identity + `hasIndoor`), `/api/v1/Public/Seasons_Team?team=` and
`/api/v1/Public/Seasons_TeamReports?team=&sport=` (team season/report indexes, request shape only),
`/api/v1/tfRankings/GetCustomListInfo?listId=` (account-scoped meet/team/athlete custom lists — the
`/team/<TeamID>/<sport>/custom-list/meets/<IDCustomList>` pages).

### Stable identifiers

| Identifier | Endpoint/field evidence | Notes / traps |
|---|---|---|
| MeetID | `rankings.MeetID`; bio `meets.<IDMeet>.IDMeet`; `GetMeetData?meetId=`; `meet.ID` | Numeric, 6 digits in 2025-26 samples (594374…667524). Same value is the URL path segment. |
| EventID | `rankings.EventID`; bio `eventsTF[].IDEvent`; `GetResultsData3` eventShort→row `EventID` | `EventTypeID` disambiguates type (0 = standard; 16 observed for a throw variant in repo fixture). |
| DivisionID | `tfDivisions[].IDDiv` (TF), `xcDivisions[].IDMeetDiv` (XC), request `divId`; `GetDivChildren.divInfo[].IDDivision` | Two id spaces: **rankings list division** (state/class, e.g. 170770/170771) vs **meet division** (`IDDiv`, e.g. Varsity/JV). `GetDivChildren` also returns `DivIDDivision` (child) — do not conflate. |
| ResultID | `IDResult` (rankings row, bio result row, `{resultId}` POST bodies for `InvalidateMark`/`ValidateMark`/`GetCorrectionInfo`) | Result-level correction key. |
| AthleteID | `AthleteID` (rankings row, bio result), `athleteId` query, `{athleteIds:[…]}` POST for `UAthleteDetails` | Batch endpoint exists (one request, many ids) — auth/meet-scope unverified. |
| TeamID / SchoolID | `TeamID` (rankings row), `TeamID`/`SchoolName` (result rows), `IDSchool` (`GetDivChildren`), `team=3204` query | Team row also carries `SchoolID` in the bio result rows; `IDSchool` in `GetDivChildren` is a *school* id — relationship unverified. |
| SeasonID | Bio `allSeasons[].IDSeason` (2026), `resultsTF[].SeasonID`, rankings row `SeasonID`, `GetNavInfo?seasonId=` | **Indoor seasons are offset by 10000**: indoor `SeasonID = 12026`; `seasonId=12026` selects the indoor list `173005`. Meet history uses `SeasonID % 1e4`. |
| shortCode | `IDResult`-adjacent opaque token (`shortCode`) in rankings and bio rows | Not an id; per-result opaque share/media key. Do not key on it. |
| LiveID / jwtMeet | `meet.LiveID` (AthleticLIVE meet id), `data.jwtMeet` (per-meet token attached as `anettokens` header) | `jwtMeet` is returned by the public `GetMeetData` and replayed as a header; it is **not** a user credential. Never persist it in reports. |

### Athletic.net leverage

This report *is* the Athletic.net side, so the question inverts: what do the other 28 sources have to supply, and
what does the meet channel save? Concretely:

- **Exposes directly:** meet links (`/TrackAndField/meet/<MeetID>`, `/CrossCountry/meet/<MeetID>`), team links
  (`/team/<TeamID>/<sport>/<SeasonID>`), athlete links (`/athlete/<AthleteID>/track-and-field`), result deep links
  (`…/meet/<MeetID>/results/<gender>/<divId>/<EventShort>`) and result ids (`IDResult`).
- **Deterministic seeds into Athletic.net** (no Athletic.net search required):
  - state-association tournament page → meet name + date → the post-season division of the same name
    (`GetDivChildren`) → division's rankings rows → exact `MeetID` (verified name-join above);
  - an existing athlete's bio (`GetAthleteBioData`) → that athlete's full `MeetID` set (18/18 verified) and
    `TeamID`;
  - a team page/`TeamNav` → `TeamID` → division membership → that division's rankings rows → `MeetID`s.
- **Requests avoided:** every `MeetID` obtained from any of those channels is one `GetMeetData` + result pull that
  does not have to be discovered by crawling Athletic.net. The measured density is the lever: **1 request
  (a single 100-row rankings page) returned 66–92 distinct `MeetID`s** (sample: 66, 77, 81, 80, 92 on five full
  pages — median 80.5, ratio 0.65–0.90 meets/row; the final 17-row page returned 16), i.e. ≈ 0.8 new meet ids per
  ranked row in deep Wisconsin 100m pages.

### Athlete evidence

From a **meet result pull** (fields verified in client code that formats them; payload not captured):

name (`AthleteName`, or `FirstName`+`LastName`), grade (`Grade`, `AgeGrade`; `AgeGrade||Grade` fallback),
school/team (`SchoolName`, `TeamName`, `TeamID`, `SchoolID`), gender (division/`events[].Gender`), TF vs XC
(separate payload keys and shells), indoor/outdoor (meet/division context, `hasIndoor` on team),
performance (`Result` display string + `SortIntRaw`/`SortIntCalc`/`SortIntOrig` normalized integers),
PR/SB flags (`PersonalBest`, `SeasonBest` — bit flags, see repo contract `(PersonalBest&2)===2` for PR,
`(SeasonBest&1)===1` for SB), meet identity (`MeetID`/`MeetName` + `ResultDate`), athlete profile URL
(`/athlete/<AthleteID>/track-and-field`). **Progression** is not a meet-pull field: build it by joining an
athlete's rows across meets (bio `resultsTF` remains the cheapest progression read: 53 rows in one 30,344-byte
response for the sampled athlete).

### Recruiting information

Not applicable / not observable. No coaching or administrator data appears in any meet, division, team-nav or
result endpoint captured. Meet result rows are athlete+school only. (Coach/AD work belongs to assignments 6, 9,
11, 13, 15, 17, 19, 21.)

### Result evidence

Endpoint request shapes are verified; response row fields marked `derived` come from the client code that consumes
them (`StandardizeResultText`, `getXCResultData`, `getAthleteName/getTeamName/getDisGrade`, `invalidateMark`,
`getRunSplits`) — no meet response body was captured.

| Requirement | Availability | Evidence |
|---|---|---|
| ResultID | Yes (`IDResult`) | `invalidateMark`: `{resultId: e.IDResult}` POST to `InvalidateMark`/`ValidateMark`; `GetCorrectionInfo` body includes `resultId`. |
| AthleteID | Yes | result-row formatter + `UAthleteDetails {athleteIds:[]}`. |
| MeetID | Yes (implicit) | payload is meet-scoped; `GetMeetData` returns `meet.ID`; rankings/bio rows carry `MeetID`. |
| EventID | Yes | `GetResultsData3` selects by `eventShort`; `GetRunSplits`/series endpoints take `eventId`. |
| mark | Yes | `Result` display + `SortIntRaw`; `Measure`, `display`. |
| normalized mark inputs | Yes | `SortIntRaw`, `SortIntCalc`, `SortIntOrig`, `ConversionInt`; division settings carry `primaryMeasure`/`secondaryMeasure`/`conversion`. |
| timing method | Yes | `FAT` flag on every rankings row (1 = FAT). |
| wind | Partial | `Wind` present on some rankings rows (present on the deep 100m page, absent on another page's rows); no wind field observed in the meet-result formatter code. Meet-level wind: unverified. |
| implement / hurdle specification | **Not observed** — no implement or hurdle-height field appears in any captured call site. `GetStandardsForEventTF`/`GetVerticalFieldSeries` exist but their payloads are uncaptured. Repo `SCOPE.md` lists this as a required target field, so this is a real gap: closes with fixture capture only. |
| heat / round | Round yes (`Round`: `F`/`P` observed = final/prelim); heat only as a *request* parameter (`GetRunSplits?round=&heat=`), so heat exists server-side but is not evidenced in a row payload. |
| place | Yes | `Place`, plus `display`/`rank`/`rowNum` in rankings; `disPlace` formatting marks non-numeric places. |
| date | Yes | `ResultDate` (rankings rows, bio rows); meet `StartDate`/`EndDate`. |
| school represented | Yes | `SchoolName`/`SchoolID`/`TeamID` (result rows), `TeamName`, `TeamMascot`. |
| relay membership | Partial | Relay rows are team rows: the formatter appends `" - <AthleteName>"` when `!PersonalEvent && AthleteName && TeamID>0`, and events/params carry a `relay` flag (`GetRunSplits`, `GetALiveDocId`, `GetALiveDocId…/relays/`). Whether individual legs are enumerated in `GetResultsData3` (the `groupParentId` parameter suggests grouped child rows) is unverified — the repo's 57,629 relay-member results were joined from rankings rosters, not from meet pulls. |

### Incremental use

Server-side caching is **not** available as a delta mechanism: all 41 captured `/api/` responses carry
`cache-control: no-store, no-cache, max-age=0, private` with `cf-cache-status: DYNAMIC`, `vary: Accept-Encoding`
and no `ETag`/`Last-Modified`. Every refresh is a full payload transfer, so the delta design must be client-side.

Cheapest weekly design implied by the evidence:

1. **Meet-level delta (new meets):** keep the previous `MeetID` set; refresh
   (a) each tracked division's rankings rows (`GetRankings` per event page — 66–92 distinct `MeetID`s per
   100-row page in the sampled deep pages) and (b) `meets` maps from `GetAthleteBioData` for already-tracked
   athletes. Anything unseen is a new meet; `meet.StartDate`/`EndDate` bound "this week".
2. **Changed-meet probe (1 request per candidate):** `GetMeetData?meetId=&sport=` — one of only two meet calls the
   client makes with **no auth header at all** (the other is `GetMeetRedirect`) — and it exposes
   `eventDivsWithResults` (TF) / `meet.HasResults` (XC), i.e. "results posted". Only if that flag is non-empty,
   pull results.
3. **Result delta:** re-pull the meet (`GetAllResultsData`) and diff on `IDResult`; `IDResult` is stable and
   monotonic per meet, so new ids = new rows (including corrections, which surface by changed `Result`/`FAT`/
   `PersonalBest` for an existing `IDResult`).
4. **Push channel (observed; most likely not anonymously usable):** the meet page subscribes to a Firebase
   realtime path `/<sport>/meets/<MeetID>/resultUpdate` (plus `entry_counts/{event,meet}`) over the socket
   `wss://s-gke-usc1-nssi3-31.firebaseio.com/.ws?v=5&ns=notify-20c67`, observed open (HTTP 101) in **both**
   captures. Its credentials come from the signed-in-user call:
   `SignedInUser/GetSignedInUser.firebaseTokens.fireNotify` is a Google identity-toolkit ID token whose issuer is
   `webserver@notify-20c67.iam.gserviceaccount.com` — the same `notify-20c67` namespace as the socket. Both
   captures were authenticated sessions, so this is a *user-scoped* channel; anonymous read is unlikely and using
   it would require an authenticated session, which this mission's rules exclude. Treated as out of scope, not as
   a candidate mechanism.
5. `GetMeetHistoryAndRecordSettings.history[].SeasonID` gives prior editions of the same meet — one request maps a
   current meet to its past editions (archive backfill without a calendar crawl).

### Access characteristics

Classification: **browser application + private structured JSON API** (not a documented API, not subscription-gated
for the surfaces above). Transport observed in HAR:

- All 41 `/api/` requests carry `accept: application/json, text/plain, */*`, `anet-appinfo: web:web:0:300`,
  `pageguid: <uuid>` (two distinct guids in the captures), `referer` of the page, HTTP/2 browser headers; 13
  requests additionally carry `anettokens`, 11 are POSTs with `content-type: application/json`. No `cookie`
  header is present in the exported HAR, so cookie usage is not evidenced either way. **No cookie or token value
  from the HAR is reproduced anywhere in this report.**
- **The `anettokens` mechanism is verified end-to-end (this is the key to reading the meet endpoints).** The
  server mints a *scope-limited* JWT in a response body and the client echoes it back as the `anettokens` header
  on follow-up calls. Measured: `tfRankings/GetRankings` (which itself sends **no** `anettokens`) returns
  `jwtTFTopReport` (319-char JWT); 10 of the 13 token-bearing calls in the capture then carry a JWT whose
  sha256 matches a `jwtTFTopReport` minted by a captured `GetRankings` response (`GetStandards` ×9, and the
  `GetNavInfo`/`GetAgesGrades` pair). The remaining 3 uses belong to a `GetRankings` response whose body is
  absent from the export. The meet service builds its header exactly the same way from `GetMeetData`'s
  `data.jwtMeet` — so `jwtMeet` **is** the meet-scope equivalent of `jwtTFTopReport`, not a user credential.
- **Both captures were authenticated sessions**, which bounds what "works anonymously" can be claimed:
  `SignedInUser/GetSignedInUser` was called in both (HAR #1 entry 66, 3,645 B: `isAuthenticated`,
  `isAuthenticatedWithGoodStanding`, `firebaseTokens{fireNotify, athleticApp}`, `jwtUserRolesSiteWide`), and
  HAR #1 additionally calls `GetUserAthletes2`/`GetUserTeams2`. Only three of the captured calls attach neither a
  user context nor a minted token by construction — `GetMeetData`, `GetMeetRedirect` (client code passes no
  headers) and `GetRankings` (no `anettokens` in 9/9 captured calls) — so those three are the anonymous-usable
  candidates; the rest were observed only inside a signed-in session ([INFERENCE], not server-verified).
- Latency: 41 API calls ranged 53.3–900.6 ms (median 78 ms; `GetRankings` slowest 199–900 ms; small metadata calls
  53–116 ms). **No 429 and no `Retry-After` header anywhere in either capture**; all 41 API responses were
  HTTP 200 (204s occurred only on analytics/telemetry hosts, never on `athletic.net`).
- Capture windows: HAR #1 = 59.0 s for 722 entries (30 API calls); HAR #2 = 18.5 s for 167 entries (11 API calls).
  Normal page load = *document (~7 KB HTML shell) + XHR fan-out*; e.g. the athlete page fired 11 API calls, of
  which 5 are athlete-scoped (`GetAthleteBioData`, `TeamNav/Team`, `General/GetRankings`, `LogOverview`,
  `SocialFollow/GetFollowInformation3`); the rankings page fired 30 (9 `GetRankings` + 9 `GetStandards` + 2
  `GetNavInfo` + 2 breadcrumbs + 2 `GetAgesGrades` + 6 chrome/signed-in-user calls).
- `robots.txt` (live, 2026-09-19): HTTP 200, 1,080 B, **no `Crawl-delay`** and no sitemap directive. Disallowed:
  `/Admin/`, `/Account/`, `/User/`, `/Edit/`, `/Partner/`, `/post/`, `/home/`, `/Secure/`, `/Search.aspx`,
  `/trackandfield/print/`, `/trackandfield/report/`, `/TrackAndField/State/`, `/CrossCountry/State/`,
  `/crosscountry/results/team.aspx`, `/crosscountry/results/season.aspx`, `/trackandfield/meet/teamresults.aspx`,
  `/CrossCountry/Results/CourseHistory.aspx`. **`/Search.aspx` is disallowed** even though the repo fixture records
  an athlete search through `https://www.athletic.net/Search.aspx/runSearch` — that channel is out of policy for a
  crawler. `/api/` is neither allowed nor disallowed (Cloudflare is the real gate).
- **Cloudflare boundary (live, my probes, 2026-09-19):** non-browser clients get the managed-challenge
  interstitial, not data — document `GET /TrackAndField/meet/634313` → **403**, 5,460 B, `<title>Just a moment...
  </title>`; API `GET /api/v1/Meet/GetMeetData?meetId=634313&sport=tf` → **403**, 5,575 B, same interstitial.
  `GET /robots.txt` → 200. This is the same boundary the repo recorded on 2026-09-19 (its live capture: first
  navigation and both indoor ranking requests 403 `Attention Required!`). Consequence: **the meet path is only
  usable through the already-human-cleared headed Chromium profile** the pipeline owns; there is no cheaper
  transport, and the repo's own rule ("no direct source-HTTP fallback") already encodes that.
- Third-party (different host, reachable, not a bypass): `live.athletic.net/robots.txt` → 200 (186 B) with a
  sitemap pointer and `Disallow: /admin*, /meets/*/athletes*, /meets/*/live*, /meets/*/teams*, /meets/*/follow*`;
  `live.athletic.net/sitemap.xml` → 200 (5,890 B, 17 URLs incl. `/meet-list`); `live.athletic.net/` and
  `/meet-list` → 200 with the *same* 50,184-byte client-rendered shell (no embedded data); its API host
  `api.athletic.live` → reachable (`/api/meets` 404 `Cannot GET /api/meets`, `/api/notices/my` 500). Client
  routes: `/meets/<id>`, `/meets/<id>/live`, `/meets/<id>/track-scores`, `/meets/<id>/events/{individual|xc|relay|CombinedEvent}/<eventId>`,
  `/meets/<id>/athletes/<id>`, `/meets/<id>/teams/<id>`; the Athletic.net meet's `meet.LiveID` is exactly the
  `/meets/<LiveID>` segment (`GetALiveDocIdTF|XC` resolves a doc id, then opens
  `https://live.athletic.net/meets/<LiveID>/<events|xc-events|relays>/<docId>`).

**Published limits:** none found on Athletic.net (robots.txt only, no rate statement). AthleticLIVE publishes
robots directives but no rate limit.

### Recommendation

**RESULT-SOURCE** — the meet channel should be the primary way the pipeline *refreshes results*, replacing
per-athlete profile refreshes wherever a meet can be named. Verdict and math:

**Whole-meet vs profile refresh — verdict: yes, whole-meet pulls replace most profile refreshes.**

| Path | Requests to obtain a meet's results | Data obtained |
|---|---|---|
| Whole meet | **3** XHR — `GetMeetData` + `GetEventDivisionData` + `GetAllResultsData` (+1 HTML doc; +2 optional `GetTeams`/`GetTeamScoreData`) | every result row of the meet, all divisions and events, each with `IDResult`, `AthleteID`, `Grade/AgeGrade`, `TeamID`/`SchoolName`, `EventID`, `Result`/`SortInt*`, `FAT`, `Place`, `Round`, `ResultDate` |
| Per-event (fallback when `GetAllResultsData` misbehaves or one event must be re-polled) | 2 + (genders × divisions × events). A 2-gender, 3-division, 18-event invite = **110** XHR | one event×division×gender per request |
| Per-athlete profile refresh | **1** essential XHR (`GetAthleteBioData`, 30,344 B for 53 results) — 11 XHR if you load the page, 5 athlete-scoped | that athlete's own history only (18 meets, 53 results over 2 seasons in the sample) |

Request-ratio: for a meet with **N participants**, whole-meet acquisition costs `N/4`-ish profile refreshes
(≈ **50–250× fewer requests** for N = 200–1,000, the plausible range for a WI invite [INFERENCE]) — and it also
returns the participants you do **not** yet know, which a profile refresh can never do. Per row of evidence, the
profile path is the most expensive of all three channels: 1 request ≈ 53 rows *for one athlete's whole TF career*
(one `sport=xc` request more for XC; vs. the retained rankings corpus's measured 80 rows per request, 340,238
results / 4,256 receipts).

Marginal coverage and the honest caveats:

- The meet channel needs a **seed**: a `MeetID`. Two seeds are verified here (rankings-row `MeetID`s at ~0.8 new
  meet ids per ranked row; bio `meets` maps at 18/18), and one is exact for Regional meets
  (division-name ≡ meet-name, 2/2 on the sampled athlete's regionals; Sectional and State stages do **not** join).
  The general **meet-calendar enumeration is the open
  gap** — no calendar API exists in the captured code, so `/events` + a division home capture decides whether
  weekly "new meets" discovery can be done without rankings sweeps.
- Freshness: `GetMeetData.eventDivsWithResults`/`meet.HasResults` gives a 1-request "has results posted" probe,
  and `IDResult` diffing makes meet re-pulls idempotent.
- Do **not** retire `GetAthleteBioData`: it is the cheapest progression/PR source (1 request/athlete) and the only
  captured source of `AllSeasons`/`grades` history. Keep it for the tracked-athlete core; move *result* acquisition
  to meets.
- AthleticLIVE is a candidate **second RESULT-SOURCE** (unblocked host, sitemap, meet list page) but explicitly
  unqualified: what is known is the host posture and URL structure; the data API (`api.athletic.live/api/**`) is
  uncaptured. One browser capture of `/meet-list` plus one meet's `/events/individual/<id>` would qualify or kill it,
  and it only helps for meets that used AthleticLIVE (the meet payload's `LiveID` is the join key).

Expected marginal coverage: replaces the per-athlete result refresh for every meet the pipeline can name (the
142,705-athlete corpus currently has no meet-level collector at all — the retained rankings receipts carry
`MeetID` per row but the delivered workbook drops it: sheet `All Athletes` columns are
`AthleteID | Names | GradeID | Teams | States | Events | Result Count | Other Observed Grades | Identity Match Status`).
Adding `MeetID` retention to the existing rankings parser is a zero-extra-request change that would immediately
yield the meet universe; the meet collector then converts *new meets* into complete result sets instead of
per-athlete discovery.

### Evidence gaps and the exact capture that closes them

No meet request exists in either HAR, so the following must be captured in the pipeline's already-cleared headed
Chromium profile with HAR recording on (one session, ≤ 6 navigations):

1. **`/TrackAndField/meet/<an outdoor invite with ≥2 divisions>`** — get a real id from a rankings row's
   `MeetID` (e.g. `634313`, `667509`, `667524` from the captured Wisconsin pages). Closes: `GetMeetData` response
   (meet object, `eventDivsWithResults`, `jwtMeet`), `GetEventDivisionData` (true `tfDivisions`/`events` counts →
   the per-event request math), `GetAllResultsData` (payload size, row fields, relay groups, `Wind`, implement).
2. **One results tab navigation** (`/TrackAndField/meet/<id>/results/<gender>/<divId>/<EventShort>`) — closes
   `GetResultsData3` response shape, `currentEventValid`, `groupParentId` grouping, and the heat/round fields.
3. **One XC meet** (`/CrossCountry/meet/<id>`) — closes `resultsXC` row shape, `IDMeetDiv`, `Distance`/`meters`,
   team scores (`GetTeamScoreData`) and the XC all-results path (`getXCAllResultData` with no params).
4. **`/events` and one division home** (`/track-and-field-outdoor/usa/high-school/wisconsin`) — closes MeetID
   discovery at scale (is there a browsable/filterable meet list, and does it XHR a calendar endpoint).

Optional fifth capture: `live.athletic.net/meet-list` + one `/meets/<id>/events/individual/<id>` to qualify or
reject AthleticLIVE as a RESULT-SOURCE independent of the Cloudflare-gated host.

### Evidence appendix

HAR rows are local captures (no live HTTP from this machine); their "status" is the recorded response status.
Live probes were single sequential `curl` requests with the default client identity, no cookies, no UA spoofing.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.athletic.net/TrackAndField/rankings/list/170770/m` | GET (HAR #1 entry 0) | 200 | Document under study: Wisconsin boys rankings; 6,975-byte Angular shell with `anetSiteAppParams` and **no** embedded API payload (no `ng-state`) | 2026-09-18T18:54:41Z |
| `https://angular.athletic.net/app/site-app/chunk-BXK99FuV2.js` | GET (HAR #1 entry 58) | 200 | Full meet service: all 30+ `Meet` endpoints, request shapes, header construction, result-row fields | 2026-09-18T18:54:5xZ |
| `https://angular.athletic.net/app/site-app/main-M36VMDMH.js` | GET (HAR #1 entry 18) | 200 | Route table (`TrackAndField/meet`, `events`, division homes), URL regexes, breadcrumb/URL-param parsing, ad-page meet detection | 2026-09-18T18:54:4xZ |
| `https://angular.athletic.net/app/site-app/tf-rankings.routes-CeE2uDyE.js` | GET (HAR #1 entry 70) | 200 | `getMeetUrl`/`getMeetResultUrl` patterns; rankings→meet links in the UI | 2026-09-18T18:54:5xZ |
| `https://www.athletic.net/api/v1/tfRankings/GetRankings` | POST ×9 (HAR #1 entries 147,348,414,478,539,587,622,656,704) | 200 (2 bodies absent from export) | 707 rows parsed → 314 `MeetID`, 668 `AthleteID`, 294 `TeamID`; row schema incl. `MeetID`/`MeetName`/`GradeID`; page size 100 + 2-row boundary overlap; `minCount` 2117 (all grades) / 1616 (grade 11) for 100m | 2026-09-18T18:54:5xZ |
| `https://www.athletic.net/api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m&recordSetId=0&locationId=0&teamId=0&indoor=false` | GET (HAR #1 entries 154,354) | 200 | Division tree (68 WI nodes), `levels[6]`, `regions[76]`, `countries[220]`, `events[283]`, list ids 168416/170770/170771/170776 | 2026-09-18T18:54:5xZ |
| `https://www.athletic.net/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170771` | GET (HAR #2 entry 166) | 200 | Division children (`divInfo[]`: 8 Sectionals) + `teams[]` (`IDSchool`, `SchoolName`); proves the division index walk | 2026-09-18T19:21:18Z |
| `https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=28872883&sport=tf&level=0` | GET (HAR #2 entry 127) | 200 | 30,344 B; `meets` map 18 (7×2025, 11×2026); `resultsTF` 53 rows with `IDResult/AthleteID/MeetID/EventID/SeasonID/FAT/Wind/Division/shortCode`; `eventsTF`, `relayTeamMembers`, `allSeasons`, `grades` | 2026-09-18T19:21:0xZ |
| `https://www.athletic.net/api/v1/SiteHeader/GetBreadcrumbs` ×2, `tfRankings/GetAgesGrades` ×2 | GET (HAR #1) | 200 | Browsing context only; breadcrumb service echoes `meetId`/`calendarId` but performs no meet lookup | 2026-09-18T18:54:4xZ |
| `https://www.athletic.net/api/v1/SignedInUser/GetSignedInUser` | GET (HAR #1 entry 66) | 200 | 3,645 B: `isAuthenticated`, `isAuthenticatedWithGoodStanding`, `firebaseTokens{fireNotify, athleticApp}`, `jwtUserRolesSiteWide` (259 ch) — proves the capture session was signed in and that `fireNotify` is the credential for the `ns=notify-20c67` socket | 2026-09-18T18:54:4xZ |
| `wss://s-gke-usc1-nssi3-31.firebaseio.com/.ws?v=5&ns=notify-20c67` | WS (HAR #1 entry 152; same URL HAR #2 entry 874) | **101** | Realtime socket actually opened in both captures — the channel the meet page's `resultUpdate` subscription rides on | 2026-09-18T18:54–19:21Z |
| `anettokens` chain: 13 header values vs. 7 `jwtTFTopReport` values minted by captured `GetRankings` responses | local sha256 comparison (no value reproduced) | n/a | 10 of 13 header values hash-match a minted `jwtTFTopReport` (319-char JWT); `GetRankings` sends no token itself. Proves the server-minted-scoped-JWT → `anettokens` echo pattern that the meet service implements with `jwtMeet` | 2026-09-19 23:50 CDT |
| Division tree structure + name join (`GetNavInfo` tree vs. bio `meets`) | local parse of HAR #1 entry 154 + HAR #2 entry 127 | n/a | 68 WI nodes = 3 Divisions → 8/8/8 Sectionals → 48 host-named Regionals (e.g. `170776 Regional 2A - Sauk Prairie`); exact normalized join to athlete meets = 2/2 regionals (`667436`, `613273`), ambiguous for Sectionals, none for the 2 State meets | 2026-09-19 23:50 CDT |
| 41 × `https://www.athletic.net/api/v1/**` responses (both HARs) | GET (30) / POST (11) | 200 (all 41) | Transport headers (`anet-appinfo: web:web:0:300`, `pageguid`, `anettokens` on 13 calls), `cache-control: no-store…` on all 41, `cf-cache-status: DYNAMIC`, `server: cloudflare`; latencies 53.3–900.6 ms (median 78); **no 429 / no Retry-After** | 2026-09-18T18:54–19:21Z |
| `https://www.athletic.net/robots.txt` | GET (live) | 200 | 1,080 B; no Crawl-delay; Disallow list incl. `/Search.aspx`, `/TrackAndField/State/`, `teamresults.aspx`, `CourseHistory.aspx`; `/api/` not listed | 2026-09-19 23:05 CDT |
| `https://www.athletic.net/TrackAndField/meet/634313` | GET (live) | **403** | Cloudflare managed challenge ("Just a moment...", 5,460 B) for a non-browser client — meet pages are unreachable without the cleared browser profile. Re-confirmed 2026-09-19 23:22 CDT (same 403, same interstitial) | 2026-09-19 23:05 CDT |
| `https://www.athletic.net/api/v1/Meet/GetMeetData?meetId=634313&sport=tf` | GET (live) | **403** | Same interstitial (5,575 B): the meet API itself is Cloudflare-gated, not merely the HTML. Re-confirmed 2026-09-19 23:22 CDT | 2026-09-19 23:05 CDT |
| `https://live.athletic.net/robots.txt` | GET (live) | 200 | 186 B; Disallow `/admin*`, `/meets/*/athletes*`, `/meets/*/live*`, `/meets/*/teams*`, `/meets/*/follow*`; Sitemap declared | 2026-09-19 23:10 CDT |
| `https://live.athletic.net/sitemap.xml` | GET (live) | 200 | 5,890 B, 17 URLs including `/meet-list` — a meet index page exists on the unblocked host | 2026-09-19 23:10 CDT |
| `https://live.athletic.net/` and `/meet-list` | GET (live) | 200 | Same 50,184-byte client-rendered shell for both paths (no server-side meet data); app bootstraps from `livestatic.athletic.net/main-5ADGVJIV.js` | 2026-09-19 23:10 CDT |
| `https://livestatic.athletic.net/main-5ADGVJIV.js`, `chunk-4TVDFI4C.js`, `chunk-CUIGWISF.js` | GET (live) | 200 | Route map (`meets` → `chunk-4TVDFI4C`), AthleticLIVE URL structure, API host string `https://api.athletic.live` | 2026-09-19 23:12 CDT |
| `https://api.athletic.live/api/meets` | GET (live) | 404 | Host reachable without Cloudflare; Express-style `Cannot GET /api/meets` (route set is not the Athletic.net `/api/v1` one) | 2026-09-19 23:15 CDT |
| `https://api.athletic.live/api/notices/my` | GET (live) | 500 | Endpoint taken from the app bundle; reachable, not anonymously answerable without app context | 2026-09-19 23:15 CDT |
| `/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx` (sheet `All Athletes`) | local read | n/a | Delivered rankings projection drops `MeetID` (columns `AthleteID, Names, GradeID, Teams, States, Events, Result Count, Other Observed Grades, Identity Match Status`) — meet ids exist upstream but are not retained downstream | 2026-09-19 23:05 CDT |
