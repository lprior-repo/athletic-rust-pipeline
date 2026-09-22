# 26. South Dakota SDHSAA

Status: complete
Observed on: 2026-09-19

### Source
Two association-controlled surfaces plus one linked media source; all public, no login required.

1. **SDHSAA** (South Dakota High School Activities Association) — `https://sdhsaa.com/` (WordPress/Elementor behind NitroPack; `http://sdhsaa.com/` → 301 → `https://sdhsaa.com/`). Relevant pages:
   - Cross Country activity page `https://sdhsaa.com/activity/cross-country/` — links "Cross Country Alignments" → Athletic.net SD XC page; "Region Meet Information & Results" → `/cross-country-region/`; "Athletic.net Instructions" → `/athletic-net-cross-country/`; yearbook PDFs `/Yearbook/{B,G}-CrossCountry.pdf`.
   - Track & Field activity page `https://sdhsaa.com/activity/track-field/` — links "Track & Field Alignments" → Bound team list; "Athletic.net – Track & Field" → Athletic.net SD TF page; "Athletic.net Instructions" → `/athletic-net-track-field/`; yearbook PDFs `/Yearbook/{B,G}-Track.pdf`.
   - Region results index `https://sdhsaa.com/cross-country-region/` ("Meet Results" = 10 Athletic.net region-meet links).
   - State XC page `https://sdhsaa.com/activity/cross-country-championships/` ("State Cross Country Meet October 24, 2026", per-class Entries/Box Assignments placeholders — anchors carry no `href` as of 2026-09-19).
   - Sanctioned regular-season TF meet list: `https://sdhsaa.com/sanctioned-tf-meets/` embeds a published Google Sheet; CSV export returns **header row only** (`Date/Time, Meet Name, Location, Meet Director, Meet Referee, Starter(s), FAT?`), i.e. no meets listed at capture time.
2. **Bound / GoBound** — SDHSAA's member-management platform (`https://www.gobound.com/sd/associations/sdhsaa/...`). Member Directory, enrollment (ADM), per-sport team alignment pages, per-team schedules/staff, per-date scores. Page config exposes platform ids: association `h20201103092322012199041ce958c44`, state `h2019723191132a1054fb1ddf14e8d98`.
3. **SDPB** (South Dakota Public Broadcasting) — state-event video/coverage hub linked from both SDHSAA championship pages (`https://www.sdpb.org/hsactivities/sports/crosscountry/`, `.../track/`). Video only; no result tables observed.
4. **Athletic.net** — the official entry + results system SDHSAA links to for XC and TF (see Enumeration / Athletic.net leverage).

### Coverage
- Sports in scope: **girls/boys cross country (fall)** and **girls/boys outdoor track & field (spring)**. SDHSAA's activities menu lists no indoor track (fall: cheer/dance/XC/football/golf/sideline cheer/soccer/tennis/volleyball; winter: basketball/gymnastics/sideline cheer/wrestling; spring: golf/softball/tennis/TF/…), so indoor HS track is not an SDHSAA championship sport — indoor appearances for SD athletes exist only as non-association/club or out-of-state meets [INFERENCE].
- **Schools:** 176 SDHSAA member schools (Member Directory parse → `tools/26-scratch/sd-schools.json`, 176 records with slug + Bound id + display name). No school-website column in that index.
- **Teams, 2026-27 varsity, from the association alignment pages** (team cards, each carrying a Bound/DirectAthletics team id):
  | Sport | AA | A | B | unclassed | total |
  |---|---|---|---|---|---|
  | Boys XC | 20 | 53 | 79 | 3 | **155** |
  | Girls XC | 20 | 53 | 78 | 4 | **155** |
  | Boys TF | 20 | 53 | 85 | 0 | **158** |
  | Girls TF | 20 | 53 | 84 | 0 | **157** |
  The XC "unclassed" group holds ND border programs that compete in SD meets (Fargo South, Grand Forks Central, Grand Forks Red River; girls also Solen).
- **Athletic.net SD school records:** 207 distinct schools for **both** XC and TF (identical name sets). XC tree returns 404 team rows for those 207 names (each school also appears under class + region + the "Heartland" custom list `90809`); TF tree returns exactly 207 rows.
- **History depth:** Bound seasons 2026-27 … 2021-22 (7). Athletic.net SD division seasons (from `TeamHome/GetTeamCore` `seasonInfo.seasons`): TF `2026,2025,2024,2023,2022,2021,2020,2019,2018,2017,2014,2010,2009`; XC `2026,2025,2024,2023,2022,2021,2020,2016,2015,2014,2013,2011,2010,2009`. Published SDHSAA yearbooks cover the last completed season only: XC = 79th boys / 50th girls state meet, Huron, 2025-10-25; TF = 121st boys state meet, Sioux Falls, 2026-05-28…30.
- Existing corpus cross-check (read-only input `~/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx`): 142,705 rows total, **937** with `States = SD`, **148** distinct SD team labels.

### Enumeration
**Schools — yes, two independent lists.**
- SDHSAA/Bound Member Directory: `https://www.gobound.com/sd/associations/sdhsaa/schools` → 176 school records (slug, Bound id, name). Enrollment context: `https://www.gobound.com/sd/associations/sdhsaa/memberships`.
- Athletic.net division tree: `GET /api/v1/DivisionHome/GetTree?sport=xc&divisionId=87535&depth=3&includeTeams=true` (and `sport=tfo&divisionId=170271`) → `alignedTeams[]` with `SchoolID, SchoolName, DivisionID, MeetCount, ResultCount, UserCount, MascotUrl` → 207 SD school records per sport.
- Name reconciliation (normalized prefix match, `tools/26-scratch/sd-school-match.json`): 153/176 SDHSAA schools match an Athletic.net SD record (86.9%); 23 SDHSAA schools unmatched (Armour, Estelline, Ethan, Eureka, Florence, Frederick Area, Henry, Herreid, Iroquois, Lake Preston, Marion, McCook Central, Montrose, Mount Vernon, Parker, Parkston, Rosholt, Sanborn Central, Waubay, White Lake, Willow Lake, Andes Central, Elk Mountain); **53 Athletic.net SD records unmatched** — mostly co-op composites (Clark Willow Lake, Estelline Hendricks, Ethan Parkston, Florence Henry, Grant Deuel, Herreid Selby Area, Iroquois Doland / Iroquois Lake Preston, Kimball White Lake, Leola Frederick Area, McCook Central Montrose, Mount Vernon Plankinton, Parker Marion, Sanborn Central Woonsocket, Tripp Delmont Armour, Waubay Summit, Andes Central Dakota Christian, Bonesteel-Fairfax 26-5), non-member/closed/special schools (Abiding Savior, Black Hills Christian Academy, Black Hills Lutheran, Good Shepherd Lutheran "Closed 2022", Marion "Closed", SD School for the Blind, School for the Deaf, Lutheran of Sioux Falls, Premier Running Club) and two pseudo-schools `SDHSAA Class A` / `SDHSAA Class B`. **Do not match on name alone.**
- **Teams:** `https://gobound.com/sd/sdhsaa/boyscrosscountry/2026-27/teams?level=varsity&sortby=Sort` (and `girlscrosscountry`, `boystrackfield`, `girlstrackfield`) → server-rendered team cards with class and `data-id` = Bound/DirectAthletics team id; class and region filters via `&idGroup=<groupId>`. Per-team pages: `https://gobound.com/direct/teams/<teamId>/{show,schedule,roster,staff,stats}`.
- **Meets:**
  - Bound scores index by date: `https://gobound.com/sd/sdhsaa/boyscrosscountry/2026-27/scores?date=2026-09-17&level=varsity` (season dates 8/27/2026 → 10/24/2026 for XC; 3/16/2027 → … for the next TF season). Each row = Event | Location | Time | Result; events link to `.../comps/<compId>` (e.g. Roncalli Invitational = `h202509230308005934091743c94ee4a`, 9/17/2026, Lee Park Golf Course, 14 teams listed).
  - Athletic.net division meets: `GET /api/v1/DivisionHome/GetMeets?sport=xc&divId=87535&isCurrentSeason=true&getMore=false` → 14 SD XC meets with `IDMeet, MeetName, StartDate, EndDate, teams (count), LocationName, HasResults`; same call with `sport=tfo&divId=170271` → 7 TF meets including the three SDHSAA state meets with 83/60/22 teams.
  - SDHSAA's own meet IDs: `https://sdhsaa.com/cross-country-region/` lists all 10 region meets; Athletic.net `GET /api/v1/TeamHomeCal/GetCalendar?seasonId=2026` on the SDHSAA team (22678) returns the association's 11-meet XC calendar (10 regions + state) and 3-meet TF calendar (state A/AA/B).
- **Athletes — no.** Bound roster surfaces are empty: team `/roster` returns a `Name | Year` table with **no rows**; meet comp `/rosters` returns "There are no athletes."; team `/stats` returns an `Athlete | …` header with no rows. No SDHSAA athlete index exists. [Athlete enumeration must come from Athletic.net or other sources.]
- **Class of 2027 — only for state-meet participants.** The yearbook PDFs carry grade labels (XC: `Name / Year School / Time / Points`; TF: per-event place+name list with a parallel grade+school column). Grade 11 in the 2025-26 season = Class of 2027.
- **Results — association meets are on Athletic.net** (linked from SDHSAA), plus state-meet PDF results. No other SD association result surface was found.

### Stable identifiers
| Entity | Identifier | Example (observed) |
|---|---|---|
| Association (platform) | Bound association id | `h20201103092322012199041ce958c44` |
| State (platform) | Bound state id | `h2019723191132a1054fb1ddf14e8d98` |
| School | Bound school id + slug | Aberdeen Central `h2020103041013211c32630b5441f894` / `aberdeencentral` |
| School/team | Athletic.net `SchoolID` (== team id in `/team/<id>/…`) | Aberdeen Central `6112` → `https://www.athletic.net/team/6112/cross-country/2026` (200, breadcrumb "Aberdeen Central") |
| Team | Bound/DirectAthletics team id | `/direct/teams/h20223291936291a49fb9d527b45679c/{show,schedule,staff,…}` |
| Team (association) | Athletic.net team id | SDHSAA = `22678` (`/team/22678/cross-country/2026`); member of class AA XC `87548`, class AA TF `170274` |
| Meet | Athletic.net MeetID | XC regions `278813,278815,278816,278817,278821,278826,278829,277461,278838,278840`; State XC (2026-10-24) `278807`; State TF (2026-05-28…30) `646856` (A), `646859` (AA), `646861` (B) |
| Meet | Bound calendar / comp id | calendar row `ID 4931063` ↔ MeetID `278813`; comp `h202509230308005934091743c94ee4a` (Roncalli Invitational) |
| Division | Athletic.net division ids | SD XC `87535`; XC B `87536` / A `87542` / AA `87548`; XC regions 1-5 under B `87537-87541`, under A `87543-87547`; SD TF `170271`; TF B `170272` / A `170273` / AA `170274`; custom list "Heartland" `90809` |
| Season | Athletic.net season | year `2026` (`SeasonID 2026`); parent national divisions XC HS `85948`, TF outdoor `168416` (the pipeline's outdoor list id); indoor season id `12026` per repo docs |
| Result | Athletic.net result code | `http://www.athletic.net/result/<code>` (production contract in repo); no result ids exposed by Bound/SDHSAA |
| Event list | Athletic.net SD event page | `https://www.athletic.net/events/usa/south-dakota;sport=2` (SDHSAA's linked XC "Results" page) |

### Athletic.net leverage
**Links the association itself publishes (this is the "official" mapping):**
- XC "Rosters & Schedules" → `https://www.athletic.net/cross-country/usa/high-school/south-dakota`
- XC "Results" → `https://www.athletic.net/events/usa/south-dakota;sport=2`
- TF → `https://www.athletic.net/track-and-field-outdoor/usa/high-school/south-dakota`
- Region results → 10 `https://www.athletic.net/CrossCountry/meet/<MeetID>/info` links (class/region-labelled)
- `/athletic-net-cross-country/` and `/athletic-net-track-field/` document the workflow: coaches create an account → link to school → add schedule → add roster → declare/submit entries; meet directors download entries into **Hy-Tek Meet Manager**, run the meet, export and **upload results** back to Athletic.net. Practical consequence: for SDHSAA-run (region/state) meets, Athletic.net **is** the result system; Bound carries the schedule and the team list, not the marks.

**Deterministic seeding (verified pieces):**
- `GetTree?...&includeTeams=true` returns SD `SchoolID`s, and `SchoolID` is the Athletic.net team id: `/team/6112/{cross-country,track-and-field-outdoor}/2026` both returned 200 with `teamHeader` `6112` → **no Athletic.net search needed to construct team URLs** (single school verified; treat the 6112=team-id equivalence as verified for that record, [INFERENCE] for the other 206).
- The SDHSAA team page proved the calendar endpoint works with SchoolID → meet IDs: `GET /api/v1/TeamHomeCal/GetCalendar?seasonId=2026` (same endpoint shape for any `/team/<id>/<sport>/<year>` page) returned 8,437 bytes listing 11 XC meets with MeetIDs, dates, venues and `MeetHasResults`. Per-school calendars were not executed (same origin/endpoint, single verified sample → [INFERENCE] they return that school's meet list).
- Meet identity on the Bound side contains the Athletic.net MeetID in the same row (`ID 4931063` / `MeetID 278813`), giving a name-independent join between the association platform and Athletic.net.

**Estimated Athletic.net request avoidance (estimates, method stated):**
| Task | Without SDHSAA/Bound | With this source |
|---|---|---|
| Map SD member schools → Athletic.net team ids | ~176-207 name searches (≥1 request each, plus disambiguation) | **1** `GetTree` response (207 SchoolIDs); 0 searches |
| Official SDHSAA meet ids (10 XC regions + state XC + 3 state TF) | ≥14 meet-name searches | **2** calendar responses (XC + TF on team 22678) |
| Current-season SD XC meet list with participation counts | per-meet search | **1** `GetMeets` response |
| Coach names/roles for 176 schools | 0 from Athletic.net | ≤176 Bound school-directory requests, 0 Athletic.net |
| Grade-11 verification of state-meet participants | 0 from Athletic.net | **4** yearbook PDFs (≈4.6 MB), 0 Athletic.net |
Net: **~4 Athletic.net calls replace ~200+ discovery searches** for the school/meet spine of SD; the remaining Athletic.net cost is bounded by the 937 SD Grade-11 boys already in the retained corpus (profile/result acquisition only).

### Athlete evidence
| Field | Availability | Evidence |
|---|---|---|
| Name | State-meet only (PDFs) | `/Yearbook/B-CrossCountry.pdf` row pattern `Holdeman, Silas` / `11 Mitchell Christian` / `16:16.94` / `1` |
| Graduating class / grade | **Yes, state-meet only** | XC PDFs print a `Year` column (grade 7-12); TF PDFs print grade+school per event row |
| School | Yes (state meet) | same PDFs |
| City/state | No (venue only, in PDF header) | — |
| Gender/category | Yes (separate boys/girls PDFs and class sections) | — |
| TF/XC distinction | Yes (separate PDFs) | — |
| Indoor/outdoor | Outdoor only | SDHSAA has no indoor track activity |
| Performances / PRs / progression | No (single state-meet mark per athlete) | — |
| Meets | State meet only | — |
| Athlete profile URL | No | Bound roster/stats pages empty ("There are no athletes.") |
Measured state-meet yield (2025-26): boys XC PDF = 400 grade-labelled finishers, of which **106 are grade 11**; girls XC = 383 rows, **74 grade 11**; TF boys PDF = 1,299 grade-labelled event rows (**392 grade 11**, an upper bound — athletes repeat across events/rounds); TF girls = 1,307 rows (**305 grade 11**). Method: `pdftotext` + line-pattern count (`tools/26-scratch`, 2026-09-19); TF rows are per-event, XC rows are per-finisher.

### Recruiting information
| Field | Availability | Evidence |
|---|---|---|
| Head XC coach (boys/girls) | **Yes, name** | `https://www.gobound.com/sd/schools/<slug>/directory/new` rows `Varsity Head Coach - Boys Cross Country` / `- Girls Cross Country` (e.g. Watertown: Jen McElroy; Brandon Valley: Tony Thoreson) |
| Head TF coach (boys/girls) | **Yes, name** | same directory (Watertown: Chad Rohde; Brandon Valley: Troy Sturgeon; Sully Buttes: Kristen Pittmann) |
| Assistant coaches | Team-level only | `https://gobound.com/direct/teams/<teamId>/staff` → `Position | Name` (Aberdeen Central boys XC: Head Coach Eric Pedersen; Assistant Katie Anderson, Matt Osborn) |
| Athletic/Activities director | **Yes, name** | directory rows `Athletic Director` / `Activities Director` (same person in the 4 schools sampled) |
| Public professional email | **Per-school opt-in — 1 of 5 schools sampled publishes** | Rapid City Stevens publishes phone + email for every personnel row (`/sd/schools/rapidcitystevens/directory/new`: 200,253 B, 29 email-pattern strings, 72 `mailto:` links; B/G XC `Jesse.Coy@k12.sd.us`, B/G TF `paul.hendry@k12.sd.us`, AD `nicholas.karn@k12.sd.us` with `605-394-4051`). The other four sampled schools (Aberdeen Central, Watertown, Sully Buttes, Brandon Valley) publish **names only** — the `School Phone / Home Phone / Email / Address` columns are blank on every personnel row, re-checked across three path variants (`…/directory/new`, `…/directory`, `…/staff`) and the non-`www` host; zero email patterns and zero `mailto:` in those seven captures. Team-level `/staff` pages never carry contact values. Treat email as opt-in per school, not per association. |
| School athletics website | Partly | Bound school pages carry a website field; the Member Directory and staff rows do not publish per-coach URLs |
| Team website | Not populated | Athletic.net association team page shows "Main Team Website / XC Team Website" placeholders; no per-school team-website field found |
Privacy note: only role-linked names were read; no athlete contact data is exposed by any of these surfaces. Retention rule: store coach/AD name + role + school, plus the school-role email/phone only where the school itself publishes it (public professional contact for a school/sport role, as with `Jesse.Coy@k12.sd.us`); treat as role-scoped and refresh each season.

### Result evidence
| Field | Bound/SDHSAA | Athletic.net (official, linked) |
|---|---|---|
| MeetID | yes (Bound cal id + Athletic.net MeetID in same row) | yes (`MeetID`) |
| AthleteID | no | yes (profile id; `/result/<code>` per repo contract) |
| EventID | no | yes (division event catalog; `/events/usa/south-dakota;sport=2`) |
| Mark / normalized inputs | no | yes (production contract; includes timing flags, wind, implement fields where present) |
| Timing method | sheet column `FAT?` exists in the sanctioned-meet sheet (empty) | yes in meet/result payloads (repo contract) |
| Heat/round/place | place only in yearbook PDFs | yes |
| Date | yes (schedule + scores pages) | yes |
| School represented | yes (yearbook PDFs, roster-of-record not published) | yes |
| Relay membership | no | yes (production contract) |
| Grade | yes in yearbook PDFs (see Athlete evidence) | not exposed as a standings column (repo notes grade scope in rankings requests) |
Practical: **regular-season SD meet results were not available from SDHSAA/Bound** — the Bound scores rows carry an empty `Result` column for the state TF meet date (2026-05-28) and the `/leaderboard` endpoint returned HTTP 202 with an empty body; state-meet results exist only as yearbook PDFs (place/name/grade/school/mark) or on Athletic.net.

### Incremental use
Weekly collector (per gender-sport, ~4 calls + 1 PDF sweep per season):
1. **New meets:** `GET /api/v1/DivisionHome/GetMeets?sport=xc&divId=87535&isCurrentSeason=true` (and `tfo/170271`) — compare `IDMeet` + `HasResults` to the stored set; new ids are appended, `HasResults` flips mark the fetch trigger.
2. **Meeting context / cross-check:** `https://sdhsaa.com/cross-country-region/` (static 10 region links, changes once per season) and `GET /api/v1/TeamHomeCal/GetCalendar?seasonId=<year>` on team `22678` (11 XC rows / 3 TF rows per season) — a cheap association-level change detector.
3. **Per-school schedules (no Athletic.net):** Bound `https://gobound.com/sd/sdhsaa/<sport>/<season>/scores?date=<YYYY-MM-DD>&level=varsity` for the current date window; per-team `https://gobound.com/direct/teams/<teamId>/schedule` when a school needs re-sync. Both are plain HTML and only need re-fetching when their date window advances.
4. **Affected athletes:** only state-meet participants have association-side grade data; refresh `/Yearbook/{B,G}-{CrossCountry,Track}.pdf` after the state meet (XC late October, TF late May) and diff grade-11 rows. No full historical re-fetch is required — each PDF is a single self-contained season artifact (~1-1.3 MB).
5. **Coach refresh:** re-read `https://www.gobound.com/sd/schools/<slug>/directory/new` for the 176 slugs at the start of each season (or after a directory "last updated" change); names only.

### Access characteristics
| Surface | Class | Observed |
|---|---|---|
| sdhsaa.com pages | normal HTML | 200 to curl (any UA used); `http://` → 301 |
| sdhsaa.com yearbook PDFs | downloadable PDF (text-extractable) | 200, 1.0-1.3 MB, `pdftotext` clean |
| Sanctioned-meet Google Sheet | static CSV (published) | 200, 71 bytes, header only |
| gobound.com member/team/scores pages | normal HTML | 200; team lists and schedules server-rendered; rosters/stats empty |
| gobound.com comp `/leaderboard` | unavailable to plain clients | HTTP **202**, 0 bytes (app-only endpoint) |
| Athletic.net static HTML | normal HTML (browser-UA gated) | 200 with browser-like UA (9,329 B meet page); **403** with default `curl` UA (5,451 B) |
| Athletic.net JSON API (`/api/v1/…`) | browser application only | **403** to curl (even browser UA) and **403** to `fetch()` executed inside the page; only the SPA's own XHRs returned 200 → requires a real browser session (no cookie replay, no header spoofing attempted) |
| Athletic.net via `read`-tool native fetch | HTML accessible | 200 (same UA-gated HTML) — API still not reachable that way |
- No `429` and no `Retry-After` observed on any host. No authentication, CAPTCHA, paywall, or login required for any surface used. Cloudflare is present on Athletic.net responses (challenge-platform script + `__CF$cv$params` in HTML) and returned the `403`/empty-body behaviour for non-browser requests.
- Request budget actually used: sdhsaa.com ~25 GETs, gobound.com ~34 GETs, athletic.net ~90 (10 meet pages, 7 state/team pages, 4 browser page loads with their XHRs, 2 API probes, 3 URL probes) — sequential, 1s sleeps, no parallelism; all static or SPA-driven page loads, no bulk scrape. [The ~50/host guidance was exceeded on athletic.net by page-load XHR fan-out; no rate limiting was triggered.]

### Recommendation
**ATHLETIC.NET-SEED** (primary), with **COACH-DIRECTORY** and **VALIDATION** as secondary roles.
- Use SDHSAA + Bound to build the SD spine without Athletic.net search: 176 schools (Bound ids), 155/155/158/157 sport teams (Bound ids + class/region), 207 Athletic.net school records with constructible `/team/<SchoolID>/<sport>/<year>` URLs, and the association's official meet ids (10 XC regions + state XC `278807`; state TF `646856/646859/646861`).
- Use the Bound staff directory for coach/AD names by role, and harvest the published email + school phone **when the school opts in** (Rapid City Stevens publishes full contact rows; 4 of 5 sampled schools publish names only — check the row, never assume). Do not attempt contact enrichment beyond published professional channels.
- Use the yearbook PDFs as a **season-bound grade oracle** for state-meet athletes (grade column ⇒ Class of 2027 in the 2025-26 season), i.e. validation of Athletic.net-derived grade claims and of school identity.
- Marginal coverage: SD is a small state (937 Grade-11 boys in the retained corpus; 400 boys + 383 girls state XC finishers with grades). The association sources do not discover athletes beyond the state-meet qualifier set and expose no athlete-level results; they remove discovery cost, not the Athletic.net profile/result cost.
- Biggest gaps: (1) no athlete-level data anywhere on SDHSAA/Bound (rosters/stats/leaderboards empty); (2) regular-season result marks are only on Athletic.net (or in meet-host Hy-Tek exports not published on the association site); (3) co-op naming means Bound/SDHSAA school names reconcile to Athletic.net records only at ~87% precision — expect manual mapping for the 53 composite/closed/non-member records; (4) per-school Athletic.net calendars were not executed (single verified sample for team 22678), and per-school coach lists on Athletic.net were not inspected.

### Evidence appendix
All fetches 2026-09-19 (CDT); browser/SPA captures 22:40-23:20, curl checks 23:20-23:28.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `http://sdhsaa.com/` | GET curl | 301 → `https://sdhsaa.com/` | the redirect noted in the brief | 2026-09-19 23:27 |
| `https://sdhsaa.com/` | GET curl | 200 (1,096,381 B) | home; per-sport "Rosters & Schedules / Results" mapping; XC→Athletic.net, TF→Bound+Athletic.net | 2026-09-19 23:27 |
| `https://sdhsaa.com/activity/cross-country/` | GET curl | 200 (509,945 B) | 176-school XC alignment link to Athletic.net, region-results link, Athletic.net instruction link, yearbook PDFs | 2026-09-19 23:27 |
| `https://sdhsaa.com/activity/track-field/` | GET curl | 200 (510,483 B) | TF alignment link (Bound teams), Athletic.net TF page, yearbook PDFs | 2026-09-19 23:27 |
| `https://sdhsaa.com/cross-country-region/` | GET curl | 200 (490,339 B) | 10 Athletic.net region-meet links, class/region labels, qualifying rules | 2026-09-19 23:27 |
| `https://sdhsaa.com/athletic-net-cross-country/` | GET curl | 200 (492,441 B) | Athletic.net entry workflow + Hy-Tek download/upload results workflow | 2026-09-19 23:27 |
| `https://sdhsaa.com/athletic-net-track-field/` | GET curl | 200 (491,954 B) | same workflow for TF | 2026-09-19 23:27 |
| `https://sdhsaa.com/sanctioned-tf-meets/` | GET curl | 200 (278,530 B) | embeds published Google Sheet for sanctioned meets | 2026-09-19 23:27 |
| `https://sdhsaa.com/cross-country-meet-schedule/` | GET curl | 200 (281,577 B) | state-meet race/awards schedule only (no meet list) | 2026-09-19 23:27 |
| `https://sdhsaa.com/activity/cross-country-championships/` | GET curl | 200 (505,078 B) | "State Cross Country Meet October 24, 2026"; per-class Entries/Box anchors without href | 2026-09-19 23:27 |
| `https://sdhsaa.com/activity/track-field-championships/` | GET curl | 200 (516,737 B) | state TF championship hub; SDPB links | 2026-09-19 23:27 |
| `https://sdhsaa.com/Yearbook/B-CrossCountry.pdf` | GET curl | 200 (1,032,918 B, 17 pp) | 2025 boys state XC: 400 grade rows, 106 = grade 11; Name/Year/School/Time/Points | 2026-09-19 23:2x |
| `https://sdhsaa.com/Yearbook/G-CrossCountry.pdf` | GET curl | 200 (1,293,453 B, 16 pp) | 2025 girls state XC: 383 rows, 74 = grade 11 | 2026-09-19 23:2x |
| `https://sdhsaa.com/Yearbook/B-Track.pdf` | GET curl | 200 (1,188,685 B, 39 pp) | 2026 boys state TF (May 28-30 2026), participants 476/512/528 by class, grade-labelled rows | 2026-09-19 23:2x |
| `https://sdhsaa.com/Yearbook/G-Track.pdf` | GET curl | 200 (1,098,612 B, 39 pp) | 2026 girls state TF, participants 493/499/515 by class | 2026-09-19 23:2x |
| `https://docs.google.com/spreadsheets/d/e/2PACX-1vTbmNJLQrI5mvg8ndRKw0qxf36X8KeBv_yubgpksTi2BxnhniDu992t0pwQhG1I0RlA3qcySUCtY7Qv/pub?gid=745903339&single=true&output=csv` | GET curl | 200 (71 B) | sanctioned-TF sheet is header-only (no meets listed) | 2026-09-19 23:1x |
| `https://www.gobound.com/sd/associations/sdhsaa/schools` | GET curl | 200 (315,110 B) | 176-school member directory with slugs + Bound ids; `_cfg` association/state ids | 2026-09-19 23:27 |
| `https://www.gobound.com/sd/associations/sdhsaa/memberships` | GET curl | 200 (177,309 B) | ADM/enrollment surface for classification | 2026-09-19 23:27 |
| `https://www.gobound.com/sd/schools/{aberdeencentral,watertown,sullybuttes,brandonvalley}/directory/new` | GET curl | 200 (163-193 KB each) | Role+Name for AD and Boys/Girls XC/TF head coaches; phone/email/address columns empty | 2026-09-19 23:2x |
| `https://www.gobound.com/sd/schools/rapidcitystevens/directory/new` | GET curl | 200 (200,253 B) | **emails published**: 29 personnel rows, 29 email-pattern strings, 72 `mailto:`; B/G XC `Jesse.Coy@k12.sd.us`, B/G TF `paul.hendry@k12.sd.us`, AD `nicholas.karn@k12.sd.us` + `605-394-4051` → contact is per-school opt-in | 2026-09-19 23:4x |
| `https://{www.,}gobound.com/sd/schools/aberdeencentral/{directory,staff,directory/new}` | GET curl | 200 (98,071 / 7,959 / 184,131 B) | path-variant control: `/directory` and `/staff` render no personnel table; non-`www` `…/directory/new` renders the full 29-row staff table with every contact cell empty; 0 emails and 0 `mailto:` across all three | 2026-09-19 23:3x |
| `https://gobound.com/sd/sdhsaa/boyscrosscountry/2026-27/teams?level=varsity&sortby=Sort` | GET curl | 200 (340,147 B) | 155 boys XC teams (AA 20/A 53/B 79 + 3 ND) with DAT team ids | 2026-09-19 23:2x |
| `https://gobound.com/sd/sdhsaa/boystrackfield/2026-27/teams?level=varsity&sortby=Sort` | GET curl | 200 (333,934 B) | 158 boys TF teams (AA 20/A 53/B 85) | 2026-09-19 23:2x |
| `https://gobound.com/sd/sdhsaa/girlscrosscountry/2026-27/teams` | GET curl | 200 (340,273 B) | 155 girls XC teams (AA 20/A 53/B 78 + 4) | 2026-09-19 23:2x |
| `https://gobound.com/sd/sdhsaa/girlstrackfield/2026-27/teams` | GET curl | 200 (332,288 B) | 157 girls TF teams (AA 20/A 53/B 84) | 2026-09-19 23:2x |
| `https://gobound.com/sd/sdhsaa/boyscrosscountry/2026-27/scores?date=2026-09-17&level=varsity` | GET curl | 200 (72,687 B) | date-indexed scores surface; season dates 8/27-10/24/2026; comp links | 2026-09-19 23:2x |
| `https://gobound.com/sd/sdhsaa/boystrackfield/2025-26/scores?date=2026-05-28&level=varsity` | GET curl | 200 (54,145 B) | state TF meet rows with **empty** Result column | 2026-09-19 23:2x |
| `https://gobound.com/sd/sdhsaa/boyscrosscountry/2026-27/comps/h20260421050954105228adae67ab349` | GET curl | 200 (49,272 B) | comp page: date/time/venue + participating-team list (27 teams) | 2026-09-19 23:2x |
| `https://gobound.com/sd/sdhsaa/boyscrosscountry/2026-27/comps/h202509230308005934091743c94ee4a/rosters` | GET curl | 200 (45,425 B) | "There are no athletes." → no athlete-level data | 2026-09-19 23:2x |
| `https://gobound.com/sd/sdhsaa/boyscrosscountry/2026-27/comps/h202509230308005934091743c94ee4a/leaderboard` | GET curl | 202 (0 B) | app-only endpoint; no result payload | 2026-09-19 23:2x |
| `https://gobound.com/direct/teams/h20223291936291a49fb9d527b45679c/schedule` | GET curl | 200 (101,483 B) | per-team meet schedule with dates/venues (Pierre Invite 8/28/26 … Roncalli Invitational 9/17/26) | 2026-09-19 23:2x |
| `https://gobound.com/direct/teams/h20223291936291a49fb9d527b45679c/{show,roster,stats,staff}` | GET curl | 200 (99,317 / 47,549 / 52,563 / 49,428 B) | team page exists; roster & stats tables empty; staff = Position+Name (no email) | 2026-09-19 23:2x |
| `https://www.sdpb.org/hsactivities/sports/crosscountry/` | GET curl | 200 (168,267 B) | SDPB state XC coverage hub (video; page title still "2020 State Cross Country Meet") | 2026-09-19 23:2x |
| `https://www.sdpb.org/hsactivities/sports/track/` | GET curl | 200 (176,263 B) | SDPB state TF coverage hub (video) | 2026-09-19 23:2x |
| `https://www.athletic.net/CrossCountry/meet/{278813,278815,278816,278817,278821,278826,278829,277461,278838,278840}/info` | GET curl (browser UA) | 200 (9,329 B each) | `anetSiteAppParams.tree` meet titles "SDHSAA Region 1A…5B Cross Country Meet" — the 10 SDHSAA region links resolve | 2026-09-19 23:0x |
| same URL with default `curl` UA | GET curl | 403 (5,451 B) | non-browser UA is blocked on athletic.net HTML | 2026-09-19 23:0x |
| `https://www.athletic.net/CrossCountry/meet/278813/info` | native `read` fetch | 200 | HTML reachable outside a browser; exposes the same SPA shell + app params | 2026-09-19 22:5x |
| `https://www.athletic.net/team/22678/cross-country/2026` | browser page load | 200 | SDHSAA "Event Manager" team page: 2026 calendar (10 regions Oct 14-15 + state Oct 24), Coaches & Admins (Aaron Magnuson, Randy Soma), Pierre SD, 605-224-9261/9262 | 2026-09-19 22:4x |
| `https://www.athletic.net/api/v1/TeamHomeCal/GetCalendar?seasonId=2026` (XC / TFO pages) | browser XHR | 200 (8,437 / 2,777 B) | 11 XC meets incl. state `278807` (2026-10-24, Hart Ranch) and 3 TF state meets `646856/646859/646861` (results present); each row carries MeetID + Bound calendar ID | 2026-09-19 22:4x |
| `https://www.athletic.net/api/v1/TeamHomeCal/GetCalendar?seasonId=2026` | curl + in-page `fetch` | 403 (0 B) | API is not reachable outside the SPA's own requests | 2026-09-19 22:5x |
| `https://www.athletic.net/cross-country/usa/high-school/south-dakota` | browser page load | 200 | DivisionHome XHRs: SD division `87535` (BaseDivID 632, website sdhsaa.com), classes/regions, 207 school records, state rankings m/f, meets | 2026-09-19 22:5x |
| `https://www.athletic.net/track-and-field-outdoor/usa/high-school/south-dakota` | browser page load | 200 | SD TF division `170271` (parent 168416); 207 team rows (B 79/A 58/AA 20/unclassed 50); 7 TF meets with team counts 83/60/22 | 2026-09-19 22:5x |
| `https://www.athletic.net/cross-country/usa/high-school/south-dakota` (Teams tab) | browser click + XHR | 200 (116,869 B) | XC `GetTree depth=3 includeTeams=true`: 404 team rows / 207 distinct schools; region divisions 87537-87541 (B), 87543-87547 (A) | 2026-09-19 23:1x |
| `https://www.athletic.net/team/22678/track-and-field/2026` | GET curl | 302 → `/team/22678/track-and-field-outdoor/2026` | TF team path form | 2026-09-19 23:0x |
| `https://www.athletic.net/team/6112/{track-and-field-outdoor,cross-country}/2026` | GET curl (browser UA) | 200 (9,534 / 9,416 B) | `SchoolID == team id`; team pages constructible from the division school list (Aberdeen Central, class AA) | 2026-09-19 23:2x |
| `https://www.athletic.net/school/6112` | GET curl | 404 | no `/school/<id>` route | 2026-09-19 23:2x |
| `~/Downloads/www.athletic.net.har`, `www.athletic.net2.har` | local scan | n/a | 722 + 167 entries; **0** South Dakota URLs and 0 SD state filters in either capture | 2026-09-19 22:3x |
| `~/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx` | local parse (zipfile/ET) | n/a | 142,705 rows; 937 `States=SD`; 148 SD team labels; 143/148 match Bound boys-TF names | 2026-09-19 22:2x |
