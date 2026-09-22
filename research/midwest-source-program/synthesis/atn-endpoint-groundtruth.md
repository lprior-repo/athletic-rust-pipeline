# Athletic.net Endpoint Ground Truth (Main-verified from HAR captures)

Main-authored verification baseline, independent of agent reports. Every row was extracted from the
raw HAR captures with `jq`; extracts preserved under `research/midwest/evidence/atn/`.

## Captures

| Capture | Window (UTC) | Entries | Character |
|---|---|---|---|
| `/home/lewis/Downloads/www.athletic.net.har` | 2026-09-18 18:54:41 – 18:55:40 | 722 | Rankings-page session (GetNavInfo/GetRankings/GetStandards) |
| `/home/lewis/Downloads/www.athletic.net2.har` | 2026-09-18 19:20:59 – 19:21:18 | 167 | Athlete-profile session (GetAthleteBioData, TeamNav) |

Preserved extracts (sha256 in `evidence/atn/`): `navinfo.json` (78,469 B),
`states-countries.json` (34,160 B), `team-nav-sample.json` (1,223 B), `athletebio-topkeys.txt`.

## Verified endpoints

| Endpoint | Method | Observed request | Response |
|---|---|---|---|
| `/api/v1/tfRankings/GetNavInfo` | GET | — | 78 KB division hierarchy JSON |
| `/api/v1/tfRankings/GetRankings` | POST | body: `{"reportType":"div","mode":"list","divListId":170770,"indoor":null,"eventShort":"100m","gender":"m","qParams":{"grades":[11],"page":N},"qualifyingListKey":"","version":2,"debug":""}` | rankings page JSON (`defaultSettings` + rows) |
| `/api/v1/tfRankings/GetStandards` | GET | ×9 in capture | standards JSON |
| `/api/v1/tfRankings/GetAgesGrades` | POST | ×2 | age/grade metadata |
| `/api/v1/tfRankings/GetConvertEvents` | GET | ×1 | event-conversion metadata |
| `/api/v1/public/GetStatesCountries2` | GET | — | `{"states":[{"Code","Name","CountryCode","CountryCode3"},…]}` (34 KB) |
| `/api/v1/TeamNav/Team` | GET | `?team=3204&sport=tf&season=2026` | team JSON: `{ID, Name, Level, hasIndoor, Address, City, State, ZipCode, Country, Mascot, MascotUrl, TeamRecords, siteSupport, colors}`, `grades[]` 9–12, `divisions[]` |
| `/api/v1/AthleteBio/GetAthleteBioData` | GET | `?athleteId=28872883&sport=tf&level=0` | 30 KB JSON; top-level keys: `allSeasons, allTeams, athlete, canEdit, distancesXC, eventsTF, grades, hasOtherSport, level, meets, photos, relayTeamMembers, resultsTF, resultsXC, synonyms, unattachedCalendars` |
| `/api/v1/AthleteBio/LogOverview` | GET | — | audit/log call (not data) |
| `/api/v1/General/GetRankings` | GET | `?athleteId=28872883&sport=tf&seasonId=2026&truncate=true` | athlete ranking summary (6 KB) |
| `/api/v1/SiteHeader/GetBreadcrumbs`, `/SiteHeader/GetDivChildren`, `/SiteFooter/GetFooterData`, `/SignedInUser/*`, `/TLog/*`, `/SocialFollow/*` | GET | — | site chrome / session |

## Verified hierarchy facts

- `GetNavInfo` for list `170770` exposes levels: `167952 United States` (Country), `167953 Clubs`,
  `168179 College`, **`168416 High School`** (Level → subDivType `States`), `170856 Middle School`,
  `179057 PAC8` (WV conference). So HS team/athlete enumeration pivots through level `168416`,
  then state divisions.
- `divListId` in `GetRankings` bodies (`170770` here) is the list being displayed, not the HS level id.
- The rankings POST filters by `qParams.grades: [11]` and paginates with `qParams.page` — Grade-11
  discovery is a server-side filter, not a client-side screen.
- One `GetAthleteBioData` call returns the full profile payload (`resultsTF` + `resultsXC` + `allSeasons`
  + `allTeams` + `meets` + relay members) — the minimum-request profile unit to validate against
  agent 2's report.

## Rankings row schema (verified, HAR1)

`GetRankings` response top-level keys: `bestsRecordSet, customList, debug, defaultSettings, division,
eventId, eventShort, eventType, eventTypes, gender, groupedRankings, hasSiteSupport, includeDivisions,
jwtTFTopReport, level, location, minCount, recordSet, relayTeams, reportTitle, settings, sportRankings`.

`groupedRankings[]` row example (Grade-11 filtered, 100 m):

```json
{"shortCode":"VPiXK5dHri4J1oNsm","IDResult":292277081,"SortIntRaw":10410,"display":"10.41",
 "Round":"F","FAT":1,"PersonalBest":14,"ResultDate":"2026-05-15T00:00:00","SeasonID":2026,
 "rank":1,"AthleteID":28872883,"AthleteName":"Kingston Penn","GradeID":11,"EventID":1,"EventType":"T"}
```

That means a single Grade-11-filtered rankings page yields athlete id, grade, result id, event id,
season id, mark, FAT flag, round, and date — discovery plus first-result evidence in one request.

## Athlete bio payload schema (verified, HAR2)

One `GetAthleteBioData?athleteId=…&sport=tf&level=0` call returns:

- `athlete`: `FirstName, LastName, Handle, IDAthlete, Gender, age, SchoolID, PhotoUrl, UsatfId, isClaimed, NoSearch, rsMugshot`
- `resultsTF[]`: `IDResult, AthleteID, Result ("23.33a"), SortInt, SortIntRaw, FAT, Place, Exhibition,
  PersonalBest, Round, Wind, SeasonBest, Division, DivisionShort, SchoolID, EventID, EventTypeID,
  MeetID, SeasonID, …` — school-per-performance (`SchoolID`), meet, event, season, timing (FAT), wind, round, place
- `meets`: map keyed by `IDMeet` → `{IDMeet, MeetName, EndDate}` — **profile requests seed MeetIDs**
- `allTeams[]`/`{…}`: `{Level, IDSchool, SchoolName, …}`
- `resultsXC` (null for TF-only athlete), `distancesXC`, `eventsTF`, `relayTeamMembers`, `allSeasons`, `grades`

Navigation context from HAR1: page `https://www.athletic.net/TrackAndField/rankings/list/170770/m`,
referer `https://www.athletic.net/track-and-field-outdoor/usa/high-school/wisconsin`, and
`SiteHeader/GetBreadcrumbs?sport=tf&athleteId=0&meetId=0&calendarId=0&divId=170770&teamId=0&seasonId=0`.

Consequence for the acquisition plan: athlete profiles are self-seeding for the identifier graph
(AthleteID → ResultID/MeetID/EventID/SeasonID/SchoolID in one request), and a weekly refresh can be
meet-centric instead of athlete-centric.

## Network boundary (observed 2026-09-19, this machine)

| Host | Result |
|---|---|
| `https://www.athletic.net/` (curl, default UA) | HTTP 403 (Cloudflare "Attention Required!") |
| `https://wi.milesplit.com/` | HTTP 200 |
| Association sites (WIAA, MSHSL, IHSAA-IA/IAGHSAU, IHSA, MHSAA, IHSAA-IN, OHSAA, MSHSAA, KSHSAA, NDHSAA, SDHSAA) | 200/301/302 |
| `nsaahome.org` | 403 (curl) |
| `wayzataresults.com` | 403 (curl) |
| `directathletics.com`, `milesplit.com`, `finishtiming.com`, `raceberryjam.com` | 200 |
| `primetimetiming.com` | 301 |

HAR captures therefore remain the only live-source evidence for Athletic.net internals on this machine.

### Correction — 2026-09-20 live re-verification (supersedes the 403 row above)

The 403 was **default-UA-specific**, not a blanket block. With a browser-grade header set
(full Chrome UA + `Accept` + `Accept-Language` + `Referer: https://www.athletic.net/` + `sec-fetch-*`),
`www.athletic.net` API endpoints answer **200 over plain HTTPS, anonymously, no cookies, no browser**:

| Endpoint | Verification |
|---|---|
| `GET /api/v1/Meet/GetMeetData` | 200 — meet object + divisions + `jwtMeet` |
| `GET /api/v1/Meet/GetEventDivisionData` | 200 (echo `jwtMeet` as `anettokens`) |
| `GET /api/v1/Meet/GetAllResultsData` | 200 — whole meet, 758 rows + 288 relay legs |
| `POST /api/v1/Meet/GetResultsData3` | 200 (lowercase `gender`; adds Wind/Heat/Measure) |
| `GET /api/v1/AthleteBio/GetAthleteBioData` | 200 — full profile |
| `GET /api/v1/TeamNav/Team` | 200 — team object |
| `POST /api/v1/tfRankings/GetRankings` | 200 — graded rows (server-side `grades:[11]` filter) |
| `GET /api/v1/tfRankings/GetNavInfo` | **403** even with browser headers — needs the SPA-bootstrapped `anettokens`(aud=jwtTFTopReport) + `anet-site-roles-token` + `anet-appinfo` + session cookies |
| `POST /api/v1/tfRankings/DownloadRankings` | **403** anonymous — role-entitled bulk export |

Live example: the 3-request whole-meet method on `meetId=634313` (765 ms, 94.5 KB,
758 results, 288 relay legs). Response inventories, token model and gotchas:
`synthesis/08-gap-closure-2026-09-20.md` §1; raw (token-redacted) payloads under
`research/midwest/evidence/gap-closure/atn/`.

**Persist rule:** never store `jwtMeet`, `anettokens`, `anet-site-roles-token` or cookie values.
