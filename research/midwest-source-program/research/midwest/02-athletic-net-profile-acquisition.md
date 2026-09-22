# 02. Athletic.net profile acquisition

Status: complete — all five assignment steps answered from captured evidence. Two sub-questions are
not observable from this machine and are labelled `[UNVERIFIED]` in place: (a) anonymous
(no-session) behaviour of `GetAthleteBioData`, because Cloudflare answers every non-browser request
with a challenge before any application response; (b) the XC response body, because neither HAR
contains an XC athlete (both captures are TF).
Observed on: 2026-09-19

All HAR entry numbers below are 0-based positions in the capture's `log.entries` array
(HAR1 = `/home/lewis/Downloads/www.athletic.net.har`, HAR2 = `/home/lewis/Downloads/www.athletic.net2.har`).
HAR1 was captured 2026-09-18T18:54:41.860Z–18:55:40.818Z (722 entries, rankings list `170770`,
gender `m`). HAR2 was captured 2026-09-18T19:20:59.690Z–19:21:18.194Z (167 entries, athlete page
`/athlete/28872883/track-and-field`). Both captures came from the operator's own headed
Chromium 151 session. Parse helper: `tools/02-har-inspect.py` (read-only; redacts
`anet-site-roles-token`, `anettokens`, `pageguid`; neither HAR contains a cookie header or `log.cookies`
entry — `log.cookies` is empty in both).

## Source

Athletic.net (`https://www.athletic.net`), an Angular SPA (`anet-site-app` mount point,
`angular.athletic.net/app/site-app/*` bundles) over an ASP.NET (`x-aspnet-version: 4.0.30319`,
`x-powered-by: ASP.NET`) JSON API under `/api/v1/`. Cloudflare fronts it
(`server: cloudflare`, `cf-cache-status: DYNAMIC`).

Athlete-profile surface, all confirmed from the captures:

- Document: `GET https://www.athletic.net/athlete/{athleteId}/track-and-field` (also
  `/cross-country`, and `/…/{sport}/all`, `/…/{sport}/{level}`).
- Bio data: `GET https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId={id}&sport={tf|xc}&level={mask}`
  (HAR2 #127).
- Supporting same-origin calls made by the profile route in the same page load: `TeamNav/Team` (#134),
  `General/GetRankings` (#135), `AthleteBio/LogOverview` (#136), `SocialFollow/GetFollowInformation3`
  (#137), plus session/site chrome `SignedInUser/GetSignedInUser` (#66),
  `public/GetStatesCountries2` (#147), `TLog/GetInitialLogData2` (#149), `TLog/GetLogUserSettings`
  (#150), `SiteFooter/GetFooterData` (#160), `SiteHeader/GetDivChildren` (#166).

The route→request contract is machine-readable in the captured bundle and matches the repository's
existing implementation exactly:

- `angular.athletic.net/app/site-app/athlete-bio.routes-BYYQfvXR.js` (HAR2 #69, 80,123 bytes,
  sha256 `830128f8832d113e5b89af76882cfccc8fe7f4e4da3dec0947f86d31a253f000` — a **different build**
  from the one recorded in `fixtures/public/athletic-source-contract.json`
  (`athlete-bio.routes-BA3NJCer.js`, sha256 `95f7d36f…`, observed 2026-09-16), but the
  best-claim snippet `isPR(n){return(n.PersonalBest&2)===2}` is byte-identical in both).
  `getAthleteBioCore(n,e,i){let a={athleteId:n,sport:e,level:i}; return this.http.get('/api/v1/AthleteBio/GetAthleteBioData',{params:a})}`.
- Same bundle: `this.sport = n === 'cross-country' ? 'xc' : 'tf'`,
  `this.level = levels.find(i => i.forUrl === e)?.mask || 0`,
  `hasOtherSport && (this.otherSport = this.sport === 'tf' ? 'xc' : 'tf')`.
- Repository equivalents (read-only): `src/runtime/source/request.rs:112-119` builds exactly
  `athleteId`/`sport`/`level=0`; `src/runtime/profile_worker.rs:178-195` lists the per-athlete
  resource set; `fixtures/public/athletic-source-contract.json` names `GetAthleteBioData` as the
  authoritative field-metadata source.

## Coverage

- **Sports:** track & field (HAR-observed: outdoor only) and cross-country (`sport=xc`; body shape
  `[UNVERIFIED]`, see Result evidence). The endpoint is **per sport**: `sport=tf` returned
  `resultsTF: [53 rows]` with `resultsXC: null` and `distancesXC: null`; the other sport requires a
  second call.
- **Indoor/outdoor:** distinguished by `SeasonID`/`allSeasons[].Display`, not by a result flag.
  Observed here: `IDSeason 2026 → "2026 Outdoor"`, `IDSeason 2025 → "2025 Outdoor"`. Indoor seasons
  carry the `1xxxx` prefix (`12026` = 2026 indoor per repository evidence: `HANDOFF.md`, `SCOPE.md`,
  nav fixture) — not exercised by any athlete in either HAR.
- **Seasons / historical depth:** the request carries no season or date bound, so depth is whatever
  the athlete has. Observed for athlete `28872883`: 2 seasons (`2025`, `2026` outdoor), 53 results,
  18 meets, 5 event families, 1 school, grades for both seasons. A synthetic repository fixture
  (`src/profile/bio.rs`/`parser.rs` tests) models an 8-season `allSeasons` array plus a
  `SchoolID 0 / IDSeason 0` sentinel row — the sentinel is fixture-only and `[UNVERIFIED]` as real
  output.
- **States:** the endpoint is athlete-keyed and state-agnostic; the captured athlete is WI
  (`meta description` carries `- WI`, `allTeams[3204].SchoolName = "Middleton"`). No state coverage
  limit is imposed by the endpoint. **Sample size caution: every field claim in this report is
  grounded in exactly one real athlete profile (Wi, male, TF, grades 10→11) plus two real rankings
  pages.**
- **School levels:** `TeamNav/Team.grades` lists `IDGrade 9..12` for the school; the bio `level`
  query parameter is a school-level mask (`level=0` = all, observed). No athlete-level enrolment
  history beyond `grades`.

## Enumeration

This endpoint is **not** an enumerator — it is a point read keyed by `AthleteID`. What it does
enumerate is everything attached to one athlete. Assessment for the required list:

| Can we enumerate… | From the Bio endpoint | Exact mechanism |
|---|---|---|
| schools | No | one athlete's `allTeams` only (1 school observed) |
| teams | No | `allTeams` keys are the athlete's school TeamIDs; `relayTeamMembers[].TeamID` are relay-team instances |
| meets | Only the athlete's | `meets` dict (18 entries) / `resultsTF[].MeetID` |
| athletes | No | needs seeds; but each profile leaks other athletes (below) |
| Class-of-2027 athletes | Only for a known athlete | `grades["{SchoolID}_{SeasonID}"] == 11` |
| results | Yes, all of that athlete's | `resultsTF` (53 rows in one response) |

**Seed surfaces that produce `AthleteID`s without any profile call (this is the load-bearing finding
for cost):**

1. `POST /api/v1/tfRankings/GetRankings` (HAR1 #147; body
   `{"reportType":"div","mode":"list","divListId":170770,"indoor":null,"eventShort":"","gender":"m","qParams":{},"qualifyingListKey":"","version":2,"debug":""}`)
   returned 191,003 decoded bytes containing 18 event families. Row fields include
   `AthleteID: 28872883`, `AthleteName: "Kingston Penn"`, `GradeID: 11`, `TeamID: 3204`,
   `TeamName: "Middleton"`, `State: "WI"`, `MeetID: 634313`, `MeetName: "Big 8 Conference"`,
   `IDResult: 292277081`, `EventID/EventShort`, `SortIntRaw`, `ResultDate`, `SeasonID`,
   `PersonalBest: 14`, `rank`, `rowNum`, `shortCode`. Counts in that single response: 180 rows,
   **157 distinct `AthleteID`s**, `GradeID ∈ {10, 11, 12, 99}` (`99` = relay placeholder per
   repository rules), `State ∈ {WI, null}`. So one page = 157 athlete seeds *with source grade and
   school*.
2. `resultsTF[].shortCode` → public per-result link `http://www.athletic.net/result/{shortCode}`
   (repository contract `src/profile/bio.rs:723-726`; not fetched live — Cloudflare would block it).
3. `relayTeamMembers[]` → other athletes for free: 40 rows in this profile, **10 distinct
   `AthleteID`s, 9 of them not the subject** (`23113661, 24785889, 24832397, 25401381, 25820258,
   28044741, 31544778, 31545765, 23113668`), each with `Name` and relay `SortID`. The captured
   bundle renders these as `/athlete/{AthleteID}/track-and-field` links, so the seed is directly
   usable.
4. The profile document itself is server-rendered with a `BreadcrumbList` (see Athletic.net
   leverage).

Profile URL is fully deterministic from `AthleteID`: `https://www.athletic.net/athlete/{id}/track-and-field`
(`/cross-country` for the other sport) — no lookup request is ever needed to construct it.

## Stable identifiers

Observed in HAR2 #127 unless noted. "Result-level" ids are per result row; "profile-level" ids are
per response.

| Identifier | Where | Observed value(s) | Notes |
|---|---|---|---|
| `AthleteID` / `IDAthlete` | `athlete.IDAthlete`, `resultsTF[].AthleteID`, `relayTeamMembers[].AthleteID`, HAR1 ranking rows | `28872883`; relay members `23113661 …` | primary key; also the profile URL segment |
| `SchoolID` / `IDSchool` | `athlete.SchoolID`, `allTeams[*].IDSchool`, `resultsTF[].SchoolID`, `grades` key prefix | `3204` (Middleton) | school id equals the school's TF TeamID here (`/team/3204/track-and-field-outdoor/2026` per the profile breadcrumb) |
| TeamID | `allTeams` key; `TeamNav/Team.team.ID`; `relayTeamMembers[].TeamID` | `3204`; relay instances `29163596, 31801142, 31916897, 31918951, 32121231, 32123232, 32304060, 32442720, 32565897, 32663276` | two distinct namespaces: school team vs relay team — do not conflate |
| `IDMeet` | `meets` key and `meets[*].IDMeet`, `resultsTF[].MeetID` | 18 ids, e.g. `610825`, `634313`, `667524` | join key to meet acquisition (assignment 3) |
| `IDResult` | `resultsTF[].IDResult` | `258445410`, `258445370`, `261172931`, `292277081` | stable worldwide result id |
| `shortCode` | `resultsTF[].shortCode` | `BMiAAK8U3FOM3Apcg` (17 chars), 3 of 53 were 18 chars | opaque; builds `/result/{shortCode}` |
| `EventID` / `IDEvent` | `resultsTF[].EventID`, `eventsTF[].IDEvent` | `1`(100m) `2`(200m) `3`(400m) `7`(4x100) `50`(4x200) | join `eventsTF` on `IDEvent` |
| `EventTypeID` / `IDEventType` | `resultsTF[].EventTypeID`, `eventsTF[].IDEventType` | `0` for all 53 rows / all 5 events | join pair documented in `athletic-source-contract.json`; non-zero values (field-event variants) not observed |
| `SeasonID` / `IDSeason` | `resultsTF[].SeasonID`, `allSeasons[].IDSeason`, `grades` key suffix | `2026`, `2025` (outdoor); indoor = `12026` form per repo | indoor/outdoor discriminator |
| `Division` / `DivisionShort` | `resultsTF[]` | `"Varsity"`, `"V"` | display strings, not ids |
| `TeamCode` | `allTeams[3204].TeamCode` | `"MIDD"` | 4-char code, not an id |

## Athletic.net leverage

This assignment *is* the Athletic.net side, so "leverage" here means: what does the profile surface
give you for free, and what does it save elsewhere.

1. **No cheap "Athletic.net link" is exposed by the profile — the profile *is* the link.** The
   profile response contains no URLs at all pointing at meets/teams other than ids. Everything is
   id-based and must be URL-composed: `/athlete/{AthleteID}/track-and-field`,
   `/team/{TeamID}/track-and-field-outdoor/{year}`, `/result/{shortCode}`.
2. **The document fetch is a second, independent identity source and is under-used.** HAR2 #0
   (9,771 bytes, brotli) is server-rendered and contains, with zero JavaScript and zero API calls:
   - `<title>Kingston Penn - WI Track and Field Bio</title>` and
     `<meta name="description" content="Kingston Penn - WI  Track and Field results and photos on Athletic.net">`
   - `<link rel="canonical" href="https://www.athletic.net/athlete/28872883/track-and-field">` plus two
     `og:url` values (`…/track-and-field` and `…/track-and-field/all`) and `og:type="athlete"`
   - an inline `ld+json` **BreadcrumbList** that is athlete-specific:
     `Track & Field` → `World/166948` → `United States/167952` → `High School/168416` →
     `Wisconsin/170770` → `Division 1/170771` → `Sectional 2/170775` →
     `Regional 2A - Sauk Prairie/170776` → `Middleton` = `https://www.athletic.net/team/3204/track-and-field-outdoor/2026`.
     The tail matches this athlete's own 2026 postseason results (`D1 Regional 2A - Sauk Prairie`,
     `D1 Sectional 2 - Verona Area`), so it is per-athlete, not viewer chrome. This single fetch
     therefore yields **state + association division chain ids + TeamID + team URL slug**, which is
     also the natural seed for rankings-list enumeration (assignment 4).
   - **Negative finding:** the shell contains no results. `IDResult`, `shortCode`, `resultsTF` and
     `class of` each occur 0 times; `<anet-site-app></anet-site-app>` is an empty mount point. The
     repository's `parse_profile_html` cohort regex (`class of (\d{4})`) therefore matches nothing on
     this build; the HTML is only good for canonical identity + breadcrumb, and all grade/PR/history
     evidence must come from the API.
3. **Searches can be eliminated.** The repository's matching flow uses
   `https://www.athletic.net/Search.aspx/runSearch` with `max_candidates = 3`
   (`config.toml [discovery]`). Once an `AthleteID` arrives from a ranking row, relay roster,
   meet result, or third-party seed (MileSplit/DirectAthletics reports), the profile URL is known
   and **0 search requests** are needed for that athlete — up to 3 are avoided per source row
   against the current config.
4. **Quantified avoidance (arithmetic on retained corpus counts, not a new observation):** the
   retained outdoor-boys delivery covers 142,705 unique athletes (`SCOPE.md`, `HANDOFF.md`). The
   repository's per-athlete profile plan issues 3 profile resources
   (`Bio{tf}`, `Bio{xc}`, `ProfileHtml` — `src/runtime/profile_worker.rs:178-195`) plus derived
   `TeamNav/Team` calls. Requesting 1 Bio instead of 3 for the TF-only subset saves 2 requests per
   athlete; against 142,705 athletes that is **285,410 avoided profile requests** in the boys
   outdoor corpus alone, ignoring the school-metadata dedupe in the next table.

## Athlete evidence

Availability from the Bio response, field by field (observed values from HAR2 #127):

| Field | Source path | Observed | Assessment |
|---|---|---|---|
| name | `athlete.FirstName`, `athlete.LastName` | `Kingston` / `Penn` | present; repository requires the observed `IDAthlete` to equal the requested one (anti-mismatch guard) |
| graduating class / grade | `grades["{SchoolID}_{SeasonID}"]` | `{"3204_2025": 10, "3204_2026": 11}` | **present and season-bound** — Grade 11 in 2026 = Class of 2027 evidence |
| school | `athlete.SchoolID`, `allTeams[].SchoolName/.IDSchool/.Level/.TeamCode/.Year` | `3204`, `Middleton`, level 4, `MIDD`, 2026 | present (name + id + level + season) |
| city/state | **absent from the Bio**; state appears in the document `meta description` (`- WI`) and `TeamNav/Team.State` (`WI`), `TeamNav/Team.City` (`Middleton`) | — | requires the school-level call (1,222 B, cacheable per TeamID) or a school index |
| gender/category | `athlete.Gender` | `"M"` | present; drives `gender=m|f` on rankings |
| TF/XC distinction | `resultsTF` vs `resultsXC` (+`distancesXC`), `hasOtherSport`, `allSeasons[].Display` | `resultsTF[53]`, `resultsXC: null`, `hasOtherSport: false` | present per sport; **two calls for two-sport athletes** |
| indoor/outdoor | `SeasonID`/`IDSeason`, `allSeasons[].Display` | `"2026 Outdoor"`, `"2025 Outdoor"` | present (indoor prefix form from repo evidence only) |
| performances | `resultsTF[]`/`resultsXC[]` | 53 rows | present, full history, one response |
| PRs | `PersonalBest` bit mask; `SeasonBest` bit mask | 13/53 rows with `PersonalBest&2 == 2`; 15/53 with `SeasonBest&1 == 1`; raw values `{0:38, 14:13, 16:2}` and `{0:35, 7:14, 8:3, 15:1}` | present but **masked flags, not booleans** (16 & 2 == 0); publisher claim kept distinct from computed best (`SCOPE.md`) |
| progression | `resultsTF[]` ordered by `ResultDate` + `SeasonID`; `SortInt`/`SortIntRaw` per mark | 2025→2026 rows interleaved by event | derivable client-side |
| meets | `meets[{MeetID}]` + `resultsTF[].MeetID` | 18 meets with `MeetName` + `EndDate` | present; `resultsTF[].meetName` is **null**, so the join is mandatory |
| athlete profile URL | not in the response body | — | trivially composed: `https://www.athletic.net/athlete/28872883/track-and-field` (repo does exactly this) |
| social/recruiting profile | `athlete.Handle` (null here) and `isClaimed` | `Handle: null`, `isClaimed: false` | when set, the captured bundle links `/profile/{Handle||IDAthlete}/feed`; `structuredData.sameAs` includes `https://www.athletic.net/profile/{IDAthlete}` |
| photos / USATF id | `photos: []`, `athlete.UsatfId: null`, `athlete.PhotoUrl: null`, `rsMugshot: null` | all empty | not dependable |
| **trap: `synonyms`** | `synonyms[]` = `[{IDAthlete 987411, FirstName "…", LastName "…"}]` (name fields present but withheld — they are the capture operator's own name, not the subject's) | 1 entry, and `987411` is the *requesting* session's own athlete id (`GetSignedInUser.profile.AthleteId`) | the app uses `synonyms` for a same-name duplicate/merge prompt; in this capture the entry is viewer-coupled, not an alias of athlete 28872883. **Do not read `synonyms` as an identity or alias map.** |

No athlete email, phone, or address exists anywhere in the Bio response (only `SchoolID`); the
response's `athlete` object has no contact fields at all. The one place viewer PII appears is
`SignedInUser/GetSignedInUser` (`profile.Email`, `PhoneNumber`, `Birthdate` for the signed-in
user) — that is operator account data, not athlete data, and must never be ingested or retained.

## Recruiting information

**Not applicable / not observable — the profile surface exposes no coach or administrator
information.** Verified absence in HAR2 #127: the response has no coach, staff, AD, email or
website field; `allTeams[3204]` carries only `Level, IDSchool, SchoolName, TeamCode, MascotUrl,
PrefMetric, PrefConvert, Year`; `TeamNav/Team` for the same school carries `Name, Level, Address,
City, State, ZipCode, Country, Mascot, TeamRecords, siteSupport, colors, coverPhoto` plus
`divisions`, `customDivisions` and `linkedTeams` — no personnel and no URLs beyond the mascot image.
The only "contact-ish" value anywhere in either capture is the operator's own account email in
`GetSignedInUser`, which is out of contract. Coach/AD discovery belongs to assignment 29 and the
state associations; the profile acquisition step must not expect to feed it. `TeamNav/Team` does
give a school postal address, which is a school (institutional) address, not a personal address —
retain the `City`/`State` for school identity if needed and leave the rest alone.

## Result evidence

Every claim below is from HAR2 #127 (`resultsTF`, 53 rows) unless noted as repository-derived.

| Required field | Availability | Path / evidence |
|---|---|---|
| ResultID | yes | `resultsTF[].IDResult` (e.g. `258445410`) |
| AthleteID | yes | `resultsTF[].AthleteID` = `28872883` on all 53 rows |
| MeetID | yes | `resultsTF[].MeetID`, joined to `meets` for name/date |
| EventID | yes | `resultsTF[].EventID` + `EventTypeID`; metadata in `eventsTF` (join pair `IDEvent`/`IDEventType`) |
| mark | yes | `resultsTF[].Result` = display string, e.g. `"23.33a"`, `"11.48a"` (trailing `a` = auto-timed) |
| normalized mark inputs | yes | `SortInt` / `SortIntRaw` (`23330`, `10401`, …) + per-event `ConversionInt` (`240.0`, `140.0`) and `Measure` (`M`) in `General/GetRankings` rows |
| timing method | partial | `FAT: 1` on all 53 rows (flag only; no other method code observed) |
| wind | yes, sparse | `Wind`: `null` on 46/53, values `{0.4, -0.1, 1.0, 1.5, -2.0, -1.2, -1.0}` on 7 |
| implement/hurdle specification | **not observed** | `EventTypeID = 0` for all 53 rows and all 5 `eventsTF` entries; `FieldMeasureType` exists on `eventsTF` as `S`/`L` display format (see `athletic-source-contract.json`: "S/L controls display format; not a physical unit"). No implement weight or hurdle height field appears in this TF sprint sample |
| heat/round | partial | `Round`: `F` 40 / `P` 13 (final/prelim). No heat number or lane field |
| place | yes | `Place` — **string**, e.g. `"1"`, `"2"` |
| date | yes | `ResultDate` ISO `"2025-05-01T00:00:00"` (plus `ignoreDay`/`ignoreMonth`, both null here) |
| school represented | yes | `resultsTF[].SchoolID` (all 53 = `3204`); `Division`/`DivisionShort` (`Varsity`/`V`) |
| relay membership | yes | `relayTeamMembers[].{TeamID, AthleteID, Name, SortID}` (40 rows); `AthleteID == IDAthlete` marks the subject's own leg; `eventsTF[].PersonalEvent = false` for `4x100`/`4x200` |
| result URL | derivable | `/result/{shortCode}` per repository contract; not fetched live |
| XC-specific fields | `[UNVERIFIED]` | `resultsXC`/`distancesXC` are `null` for `sport=tf`. Repository code (`src/profile/bio.rs:347-350, 423-640`, `src/profile/bio/events.rs`) parses `resultsXC[].Distance` as a join key into `distancesXC[] = {Meters, Distance, Units}`. No real XC body exists in either HAR |
| boundedness | 30,344 decoded bytes / 5,876 gzip / 144.7 ms for 53 rows ≈ 572 bytes and ~111 gzip bytes per result (arithmetic) | a 400-result career ≈ 229 KB decoded / 44 KB gzip (arithmetic, `[INFERENCE]`) |

Aggregate counts observed in that one response: 53 results over 18 meets, 2 seasons, 5 event
families, 1 school, 13 PR-flagged and 15 SB-flagged rows, 7 rows with wind, 40 relay-member rows.

**No pagination exists as the app uses the endpoint.** The captured bundle builds the call with
exactly `{athleteId, sport, level}` and the repository builder appends exactly
`athleteId`/`sport`/`level=0`; no page/size/offset parameter appears in either. All 53 rows arrived
in one response. Whether the server silently truncates very large histories cannot be determined
from a 53-row sample — `[UNVERIFIED]`, and the reason a "compare count against a full-history
source" check is worth keeping.

## Incremental use

Facts that constrain refresh design:

- The Bio response is **uncacheable by HTTP**: `cache-control: no-store, no-cache, max-age=0, private`
  and `cf-cache-status: DYNAMIC` (HAR2 #127). Every refresh is a real origin request; there is no
  validator/ETag and no 304 path.
- The response is a full-history snapshot per sport, so "fetch again" always returns everything —
  the only question is *when* to fetch.
- New results are identifiable by `IDResult`, which is issued in date order: the 20 rows for
  season 2025 span `258445370`-`264166400` and the 33 rows for season 2026 span
  `280259476`-`295310276` - a clean separation. Within a season, `ResultDate` gives the ordering.
- The response gives **no `modified` timestamp**, so change detection must be content-derived.

Recommended weekly design (delta work, no full re-fetch):

1. **Keep a durable per-(AthleteID, sport) fingerprint** of the last accepted Bio:
   `{max(ResultDate), max(IDResult), result count, set(SeasonID)}`. A re-fetch that reproduces the
   fingerprint is a no-change and costs exactly one request with no downstream work.
2. **Drive the refresh list from meet-level sources, not from the athlete list.** Assignment 3
   (`GetMeetData`/`GetAllResultsData`/`GetResultsData3`) and the external timer/meet reports
   (assignments 8, 12, 22, and the state reports) identify *new meets* cheaply; the `AthleteID`s in
   those results form the exact set of profiles whose histories can have changed. Fetch a Bio only
   for athletes appearing in a newly published result set, plus a low-frequency backfill sweep.
3. **Fetch per-season, not per-week:** schedule a full re-read of an athlete's current-season Bio
   once at the end of each season (state meet) and on first discovery; between those, rely on step 2.
   Because the payload includes all seasons every time, the marginal cost of any fetch is the same —
   so fetching less often is the only lever.
4. **Deduplicate school metadata at the school level.** `TeamNav/Team` is keyed by
   `(team, sport, season)` and is identical for every athlete at that school. One cached fetch per
   school-season (≈700 WI high schools, not 142,705 athletes) supplies `SchoolName`, `City`, `State`,
   `Level`, `hasIndoor`, and the division list used to interpret `Division`/`DivisionShort`. The
   repository currently derives these requests per athlete (`authorized_requests_value` →
   `season_requests` over `allTeams` × `allSeasons`, bounded at `MAX_TEAM_REQUESTS = 4096`).
5. **Gate the XC call:** issue `sport=xc` only when the first response's `hasOtherSport` is true.
   `[UNVERIFIED, single sample]` whether `hasOtherSport` means "has XC results" or "has an XC profile
   row"; qualify it on a 20-athlete sample before relying on it, and fall back to issuing both if the
   flag proves unreliable.
6. **Never re-derive seeds:** persist the `AthleteID` alongside the discovery provenance
   (ranking row, relay roster, meet result, external source) so a refresh never re-runs search.

## Access characteristics

Classification: **public structured JSON behind a browser application (session-gated by a Cloudflare
managed challenge)**. Not documented, not authenticated, not subscription — but not reachable by a
plain HTTP client from this machine either.

- **Plan boundary, measured 2026-09-19 23:15 America/Chicago (2026-09-20T04:15Z), sequential,
  4 requests:**
  - `GET /robots.txt` → **200**, 1,080 bytes, `content-type: text/plain`.
  - `GET /api/v1/AthleteBio/GetAthleteBioData?athleteId=28872883&sport=tf&level=0` → **403**,
    5,707 bytes, `cf-mitigated: challenge`, body `<title>Just a moment...</title>` (Cloudflare
    managed-challenge interstitial, not the `Attention Required!` block page recorded in
    `HANDOFF.md` for 2026-09-19 — same status class, different interstitial).
  - `GET /athlete/28872883/track-and-field` → **403**, 5,481 bytes, `cf-mitigated: challenge`.
  - `GET /api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m` → **403**, 5,650 bytes,
    `cf-mitigated: challenge`.
  - Every 403 carried `server: cloudflare` and `set-cookie: _cfuvid=…` (a Cloudflare visitor id;
    value deliberately not reproduced). No `Retry-After` was returned on any of them.
- **What the persistent Chromium session adds:** the application-layer responses. With a
  JS-capable browser session that has cleared the challenge, all 11 same-origin API calls in HAR2
  returned `200` and the app-visible fields (`canEdit`, `isClaimed`, `synonyms`, `Handle`) were
  rendered for the authenticated viewer. The captures prove the *shape* and *content* of a
  successful response and prove that the origin serves them normally to a browser; they do **not**
  prove that a challenge-cleared, cookie-less session receives the same body, because the capture
  session was signed in (`GetSignedInUser` #66: `isAuthenticated: true`, `userId 960929`,
  `profile.AthleteId 987411`).
- **Anonymous access `[UNVERIFIED]`:** cannot be tested from this machine — the challenge fires
  before any application response, and solving/bypassing it is out of contract. Structural evidence
  that the data is not viewer-private: the bio response for *another* athlete (`28872883`, not the
  signed-in user's `987411`) was served in full, and the app builds a public
  `structuredData.sameAs = https://www.athletic.net/profile/{IDAthlete}` plus public `/profile/{handle}`
  routes. Do not infer anonymous access from that; qualify it in the owned headed profile instead.
- **Header set on the API call (HAR2 #127)** — values reproduced only where they are not secrets:
  `accept: application/json, text/plain, */*`, `accept-language: en-US,en;q=0.9`,
  `anet-appinfo: web:web:0:300`, `anet-site-roles-token: <JWT, redacted>` (decodes to
  `{userId:960929, userRoles:[], iss:"athletic.net", aud:"jwtUserRolesSiteWide"}` — empty roles),
  `pageguid: <redacted>`, `referer: https://www.athletic.net/athlete/28872883/track-and-field/all`,
  the usual `sec-ch-ua*` / `sec-fetch-*` client hints, and
  `user-agent: Mozilla/5.0 (X11; Linux x86_64) … Chrome/151.0.0.0 Safari/537.36`. A second token
  header, `anettokens` (319-byte JWT), is sent only on `tfRankings/GetStandards`, `GetAgesGrades`
  and `GetNavInfo` — **not** on `GetAthleteBioData`. Neither HAR contains a `cookie` request header
  and both have empty `log.cookies`, so the cookie values needed to clear the challenge are not
  recoverable from these captures at all (and must not be copied if they were).
- **Politeness / limits:** no 429, no `Retry-After`, no 4xx and no 5xx anywhere in either HAR.
  Combined status distribution over all 889 entries: `853 × 200`, `34 × 204`, `2 × 101`
  (the two Firebase websocket upgrades); `retry-after` response header count = 0. The repository
  paces at `source_interval_ms = 1000` with 2 tabs
  (`config.native.toml`), `search_delay_ms = 750`, `max_candidates = 3`, `delay_ms = 2000`,
  `respect_robots_txt = true`, and treats `403` as non-retryable while retrying `429`/`5xx` up to 4
  attempts with `Retry-After` honoured (`src/runtime/source/retry.rs`). Note `config.toml` carries
  `authorized_direct_fetch = true` next to the comment "Athletic.net currently prohibits scraping and
  automated spiders in its Terms. Enable only after obtaining authorization." — a policy
  contradiction between comment and value that Main should reconcile before scaling the profile
  phase.
- **`robots.txt` (fetched 2026-09-20T04:15:24Z, 200):** `User-agent: *` disallows ~20 specific
  paths and prefixes — `/Admin/`, `/Account/`, `/User/`, `/Edit/`, `/Partner/`, `/Secure/`,
  `/home/`, `/search.aspx`, `/Search.aspx`, `/trackandfield/print|report/`, `/CrossCountry/State/`,
  `/TrackAndField/State/`, `/crosscountry/results/coursehistory.aspx`, and others. **`/athlete/`,
  `/api/` and `/team/` are not disallowed for `*`**; `MSIECrawler` and `AhrefsBot` are disallowed
  site-wide. Note `/Search.aspx` *is* disallowed — the seeding move from search to direct profile
  reads is also the robots-compliant direction.

## Recommendation

**PRIMARY** for athlete identity/history — with the mandate that profile work be reduced to the
minimum request count proven above. Concretely:

Expected marginal coverage: this is the only source seen so far that returns, in **one** request,
name + school + season-bound grade + full per-sport result list with `IDResult`/`MeetID`/`EventID`/
wind/round/place/date + relay membership + publisher PR/SB claims. No other source in this
exploration can substitute for that history; the alternatives (MileSplit, DirectAthletics, state
association archives) are discovery/validation/seeding surfaces. The marginal value of *not*
re-architecting around it is the 142,705-athlete corpus that already exists.

**Request budget per athlete** (same-origin requests to `www.athletic.net`; "static" = JS/CSS/font/image):

| Profile-phase design | Requests per athlete | What it yields | Evidence basis |
|---|---|---|---|
| Observed page load in this capture | 1 document + 129 static + 11 API + 1 `cdn-cgi/rum` = **142** same-origin (plus 25 third-party) | everything the UI renders for one athlete, cold app | counted over HAR2's 167 entries |
| Repository design today | `Bio{tf}` + `Bio{xc}` + `ProfileHtml` = **3 profile**, plus 1–2 derived `TeamNav/Team` = **4–5** | both sports' bios, HTML identity, school location | `src/runtime/profile_worker.rs:178-195`; team requests derived per athlete × season |
| **Minimum verified (recommended)** | **1** for a TF-only athlete, **2** for a two-sport athlete | name, school, season-bound grade, seasons, meets, all results, events, relay members | HAR2 #127 alone satisfies every Athlete-evidence row except city/state |
| + school city/state | **+0 amortised**: one `TeamNav/Team` per `(TeamID, season)` cached for all athletes at that school (1,222 B) | `City`, `State`, `Level`, `hasIndoor`, `grades` | HAR2 #134 |
| + division chain and team URL slug | **+1** (`ProfileHtml`) only if the `ld+json` breadcrumb is actually consumed | state + division chain ids + `/team/{TeamID}/…` slug | HAR2 #0 |
| + PR/rank panel | **+1** `General/GetRankings?athleteId=` — derived from the same results, so normally skippable | per-event PR with `Position`/`IDDivName` scoping | HAR2 #135 |

Aggregate arithmetic on the retained corpus count (142,705 unique athletes, `SCOPE.md`): the current
3-request-per-athlete profile plan costs 428,115 profile requests for that corpus and up to 713,525
once per-athlete team lookups are included, versus 142,705–285,410 at the recommended minimum — a
saving of **285,410–428,115 requests**, before counting the up-to-3 eliminated `Search.aspx/runSearch`
requests per workbook row (`config.toml [discovery] max_candidates = 3`). These are arithmetic
projections over a retained count plus measured per-call costs, not new measurements.

Adopt, in order:

1. **1 request per athlete-sport, not 3 per athlete.** Replace the always-both-sports
   `initial_resources` (`Bio{tf}`, `Bio{xc}`, `ProfileHtml`) with `Bio{tf}`, then `Bio{xc}` only if
   `hasOtherSport`. Saving: 2 of 3 profile requests for a TF-only athlete (67%), 1 of 3 for a
   two-sport athlete (33%).
2. **Hoist school metadata to a per-(TeamID, season) cache** instead of per-athlete
   `TeamNav/Team` derivation. Measured: `TeamNav/Team` is 1,222 bytes and contains no athlete-specific
   data.
3. **Keep the document fetch only if the breadcrumb is used.** The shell is 9,771 bytes and contains
   no results, but it uniquely supplies the athlete's division chain in one request —
   world `166948` → country `167952` → high school `168416` → state `170770` → division `170771` →
   sectional `170775` → regional `170776` → team — plus the `/team/{TeamID}/…` URL slug. That is
   useful for seeding rankings enumeration; it is worthless for grade/PR evidence. If the chain is
   already derivable from `GetNavInfo` (captured in HAR1 #154) or the state reports, drop the fetch.
4. **Seed, never search.** With `AthleteID` from ranking rows, relay rosters, meet results, or
   MileSplit/DirectAthletics (assignments 3, 4, 7, 18, 27, 28), the profile URL is deterministic;
   `max_candidates = 3` search requests per source row become 0.
5. **Treat these as trap fields:** `PersonalBest`/`SeasonBest` are bit masks (`&2` / `&1`, so `16` is
   not a PR); `Place` is a string; `resultsTF[].meetName` is null (join `meets`); `Division` is a
   display string; `synonyms` is viewer-coupled and must not be read as an alias map;
   `resultsXC`/`distancesXC` are `null` (not `[]`) when the sport does not apply.
6. **Qualify before scaling:** (a) run `hasOtherSport` against a 20-athlete sample to confirm it
   predicts an XC profile; (b) confirm in the owned headed profile whether the Bio endpoint answers
   without a signed-in account, since `[UNVERIFIED]` and it changes whether `anet-site-roles-token`
   must be minted per session; (c) sample one 300+-result athlete to confirm the endpoint does not
   silently truncate.

### Evidence appendix

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.athletic.net/athlete/28872883/track-and-field` (HAR2 #0) | GET (Chrome 151) | 200 | server-rendered shell: title/`meta description`/`canonical`/`og:url`/`og:type=athlete`, athlete-specific `ld+json` BreadcrumbList with division chain + `/team/3204/…`; **no** results (`IDResult`/`shortCode`/`class of` = 0 hits); `<anet-site-app>` empty mount | 2026-09-18T19:20:59.690Z |
| `https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=28872883&sport=tf&level=0` (HAR2 #127) | GET | 200 | the profile payload: 30,344 B decoded / 5,876 gzip / 144.7 ms; `athlete`, `allSeasons[2]`, `allTeams{3204}`, `grades{3204_2025:10,3204_2026:11}`, `meets[18]`, `resultsTF[53]`, `eventsTF[5]`, `relayTeamMembers[40]`, `resultsXC:null`, `hasOtherSport:false`, `synonyms[1]`; `cache-control: no-store…private` | 2026-09-18T19:21:00.062Z |
| `https://www.athletic.net/api/v1/TeamNav/Team?team=3204&sport=tf&season=2026` (HAR2 #134) | GET | 200 | school identity + location at school granularity (City `Middleton`, State `WI`, Level 4, `hasIndoor:true`, `grades[4]`, `divisions[6]`, `linkedTeams[2]`); no personnel; 1,222 B | 2026-09-18T19:21:00.210Z |
| `https://www.athletic.net/api/v1/General/GetRankings?athleteId=28872883&sport=tf&seasonId=2026&truncate=true` (HAR2 #135) | GET | 200 | PR/rank panel for one athlete: 21 rows with `EventShort`, `SortInt`, `Position`, `IDDivision`/`DivName` (`Team`, sectional, `United States`); 6,122 B | 2026-09-18T19:21:00.220Z |
| `https://www.athletic.net/api/v1/AthleteBio/LogOverview?athleteId=28872883` (HAR2 #136) | GET | 200 | training-log visibility only (`canViewLog`, `logIsActive`, `workouts`); not result evidence; 773 B | 2026-09-18T19:21:00.220Z |
| `https://www.athletic.net/api/v1/SocialFollow/GetFollowInformation3?type=A&linkId=28872883&useRW=false` (HAR2 #137) | GET | 200 | social follow count only; 48 B | 2026-09-18T19:21:00.221Z |
| `https://www.athletic.net/api/v1/SignedInUser/GetSignedInUser` (HAR2 #66) | GET | 200 | capture session **was authenticated** (`userId 960929`, `isAuthenticated:true`, `profile.AthleteId 987411`) → anonymous behaviour of the Bio endpoint is not established; response also contains viewer account PII (excluded from this report) | 2026-09-18T19:21:00.028Z |
| `https://www.athletic.net/api/v1/TLog/GetInitialLogData2?athleteId=987411&urlTeamId=0` (HAR2 #149) | GET | 200 | viewer's own training log (id `987411` = viewer), confirming `synonyms` on #127 is viewer-coupled | 2026-09-18T19:21:00.305Z |
| `https://www.athletic.net/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170771` (HAR2 #166) | GET | 200 | division-tree chrome (`Division 1`, `StateName: wisconsin`, sectional children); 1,962 B | 2026-09-18T19:21:18.194Z |
| `https://angular.athletic.net/app/site-app/athlete-bio.routes-BYYQfvXR.js` (HAR2 #69) | GET | 200 | route→request contract: `getAthleteBioCore` builds `{athleteId,sport,level}` only (no pagination); `sport = 'cross-country' ? 'xc' : 'tf'`; `hasOtherSport → otherSport`; `isPR = (PersonalBest&2)===2`, `isSR = (SeasonBest&1)===1`; relay members link to `/athlete/{AthleteID}/track-and-field`; 80,123 B, sha256 `830128f8…` | 2026-09-18T19:21:00.038Z |
| `https://www.athletic.net/api/v1/tfRankings/GetRankings` (HAR1 #147) | POST `{"reportType":"div","mode":"list","divListId":170770,"eventShort":"","gender":"m",…}` | 200 | the seed interface: 18 event families, 180 rows, **157 distinct `AthleteID`**, `GradeID ∈ {10,11,12,99}`, plus `TeamID/TeamName/State/MeetID/MeetName/IDResult/EventShort/SortIntRaw/ResultDate/PersonalBest`; 191,003 B / 63,344 gzip / 611 ms | 2026-09-18T18:54:42.330Z |
| `https://www.athletic.net/api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m` (HAR1 #154) | GET | 200 | division-tree enumeration source (78,468 B); carried `anettokens`, not on the Bio endpoint | 2026-09-18T18:54:42.948Z |
| `https://www.athletic.net/api/v1/public/GetStatesCountries2` (HAR2 #147) | GET | 200 | filter vocabulary chrome (34,159 B) — page-level, cacheable, not per-athlete | 2026-09-18T19:21:00.298Z |
| `https://www.athletic.net/robots.txt` | GET (curl) | **200**, 1,080 B | `User-agent: *` disallows `/Search.aspx`, `/home/`, `/Account/`, state/print/report paths — **not** `/athlete/`, `/api/`, `/team/`; `MSIECrawler`/`AhrefsBot` disallowed site-wide | 2026-09-20T04:15:24Z |
| `https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=28872883&sport=tf&level=0` | GET (curl, no cookies) | **403**, 5,707 B, `cf-mitigated: challenge` | plain clients do not reach the API: Cloudflare managed challenge precedes the application; no `Retry-After` | 2026-09-20T04:15:27Z |
| `https://www.athletic.net/athlete/28872883/track-and-field` | GET (curl, no cookies) | **403**, 5,481 B, `cf-mitigated: challenge` | same boundary for the document route; `cf-ray` present, `server: cloudflare` | 2026-09-20T04:15:31Z |
| `https://www.athletic.net/api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m` | GET (curl, no cookies) | **403**, 5,650 B, `cf-mitigated: challenge` | the boundary is host-wide, not endpoint-specific; `set-cookie: _cfuvid=…` returned (value not reproduced) | 2026-09-20T04:15:34Z |
| `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/fixtures/public/athletic-source-contract.json` | file read | n/a | `GetAthleteBioData` named as field-metadata source; `isPR/isSR` snippet; `IDEvent/IDEventType` ↔ `EventID/EventTypeID` join; search may return TF and XC links for one identity | 2026-09-19 |
| `src/runtime/profile_worker.rs:178-195`, `src/runtime/source/request.rs:112-119`, `src/profile/bio.rs:347-350,423-640,709-726`, `src/runtime/source/retry.rs`, `config.toml`, `config.native.toml`, `SCOPE.md`, `HANDOFF.md`, `CHROMIUM_DESIGN.md` | file reads | n/a | current per-athlete resource set (3), exact request construction, XC distance join contract, `/result/{shortCode}` URL, retry/`Retry-After` policy, pacing (`source_interval_ms=1000`), 142,705-athlete retained corpus, 2026-09-19 live 403 record | 2026-09-19 |

Total live requests issued to `www.athletic.net` by this assignment: **8** (4 + 4 probe sets,
sequential, no cookies, no UA spoofing), all `GET`, ≤1 per ~2 s. All HAR evidence is from the two
read-only captures dated 2026-09-18. No other host was contacted.
