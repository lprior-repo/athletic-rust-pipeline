# 20. Ohio independent athlete sources

Status: complete
Observed on: 2026-09-19

Scope: Ohio (OH) MileSplit, Ohio timing providers and independent result databases, and what they
add over OHSAA/Athletic.net. All live observations were made from this machine on 2026-09-19
(23:12–23:30 America/Chicago, rounded from capture mtimes); raw captures are in
`tools/ohio-independent/raw/`. Appendix timestamps are the saved-artifact mtimes rounded to the
minute.

Primary finding in one line: **Ohio MileSplit exposes an undocumented public JSON API
(`/api/v1/rosters/teams/<TeamID>/ranked`, `/api/v1/meets/<MeetID>/performances`) that returns
`gradYear` (Class of 2027) and full meet results without login or subscription, and Finish Timing's
public homepage publishes Athletic.net MeetIDs next to its own static result URLs.**

---

### Source

| Provider | Hosts | Role in Ohio |
|---|---|---|
| MileSplit Ohio (FloSports) | `oh.milesplit.com`, `www.milesplit.com`, API `oh.milesplit.com/api/v1/` | Largest Ohio athlete/team/meet database; team index, meet calendar, athlete profiles, JSON API; hosts OATCCC indoor state-meet pages |
| Finish Timing | `www.finishtiming.com` (SvelteKit app), `finishtimingresults.com` (static archive), `finishtiming.trackscoreboard.com` (live) | Ohio timer; meets observed on the 2026 homepage/archive sit in Dayton, Springfield, Cedarville, Tiffin, Cincinnati, Wilmington, Bowling Green, New Carlisle, Mechanicsburg, West Liberty, London/Madison-Plains and Ashland; also college and USATF meets |
| Baum's Page | `www.baumspage.com` | Ohio meet-management/entry host + result-file host (NW/central/east OH small-school meets; OHSAA district/regional archives 2003–2021) |
| Track Scoreboard | `finishtiming.trackscoreboard.com`, `api.trackscoreboard.com` | Live-results SPA used by Finish Timing meets (Firebase Realtime DB `track-scoreboard-default-rtdb.firebaseio.com`) |
| Buckeye Timing / Can't Stop Timing | `buckeye-timing.com`, `cantstoptiming.com` | Same SvelteKit build as Finish Timing — both serve the same 2,804-byte shell containing `finishtiming:stale-chunk-reload` (files not byte-identical, so the marker string is the evidence) — Finish Timing partners, not separate data planes |
| OATCCC | `www.oatccc.com` | Coaches association; publishes 2026 OATCCC indoor state-meet pages that link into MileSplit |
| MeetPro (DirectAthletics) | `tfmeetpro.com` | Meet-management software by DirectAthletics; linked from MileSplit meet pages; results are uploaded by the meet host/timer, not browsable as an Ohio index |
| Ohio Runner | `www.ohiorunner.com` | Road-race calendar only — no HS TF/XC result database |
| OHSAA | `www.ohsaa.org` | State series; 2026 state T&F runs on Athletic.net (assignment 19 owns the deep map) |

Reachability observed today: `oh.milesplit.com`, `www.finishtiming.com`, `finishtimingresults.com`,
`www.baumspage.com`, `www.ohsaa.org`, `tfmeetpro.com`, `www.oatccc.com`, `www.ohiorunner.com` all
HTTP 200. `www.neohstrack.com` fails TLS with `curl: (60) SSL: no alternative certificate subject
name matches target hostname 'www.neohstrack.com'` (unusable without ignoring cert validation; not
probed further). `finishtimingresults.com/` root returns 200 with an empty body; the archives are
reachable only under `/YYYY/`.

### Coverage

| Source | States | Sports / levels | Seasons | Historical depth observed |
|---|---|---|---|---|
| MileSplit OH | OH (plus OH teams at out-of-state meets; calendar mixes in KY/FL/NY/TN/MI meets) | HS, MS, club, college, unattached; TF (indoor/outdoor) + XC | Season select `indoor`, `outdoor`, `cc`; year select labelled 2016-2017 … 2026-2027 (11 school years) | 2016–2026 selectable per team; sampled Class-of-2027 athlete page carried 10 season blocks (2026→2022, hs+ms) |
| Finish Timing | tri-state OH/KY/IN + college/USATF | HS, MS, JH, elementary, open, college | `/2026/` (track) + `/2026/CC/` (XC); `/2025/CC/` present | 2025–2026 on the current host layout; older seasons under date-coded CC dirs |
| Baum's Page | OH | HS + MS, TF + XC | 2026 index + `archive.php` (track pre-2012, CC pre-2011) and OHSAA regional/district 2003–2021 | 2003–2026 |
| Track Scoreboard | meets it times | live only | current meet | live pages only |
| OATCCC | OH | Indoor HS + MS state meets | 2026 | links only, no own results DB |

Counts measured on 2026-09-19:
- MileSplit OH team index `/teams`: **977 unique high-school team IDs** (page renders a
  `<select name="type" id="teamType">` with 11 file types — `1=High School` … `11=Unattached`; the
  canonical URL is `https://oh.milesplit.com/teams?level=high-school`, which returned the same 977
  IDs (303,977 vs 304,039 bytes) — i.e. the bare `/teams` page is already the HS universe. The MS
  team `28988-st-paul-ms` and club team `53710-cleveland-youth-running-club` are **not** in the
  list, so 977 is the HS universe on MileSplit).
- MileSplit OH calendar, XC 2026 default (upcoming window): **294 meet rows / 294 unique MeetIDs**;
  `data-level` breakdown `hs`=100, `ms`=95, `hs,ms`=41, blank=30, remaining mixed → **156 meets
  include HS competition**.
- MileSplit OH calendar, XC 2026 October, `level=high-school`: **133 meet rows** (includes OHSAA
  district meets, e.g. MeetID 769837 "OHSAA Division 3 District - Columbus Grove", 769830 Div 4).
- Finish Timing `/2026/` autoindex: **325 result files** (275 plain 6-digit IDs, 38 `2000`+6-digit,
  2 `20`+6-digit, 10 other — timer-native IDs and slugged names such as `1000739707.html`,
  `690155-Kent-State-University-HS-Meet-3.html`) plus the `CC/` subdirectory; `/2026/CC/` holds
  **32 date-coded meet directories** (`08-01-CEN` … `09-22-MP`) and 3 loose files.
- Baum's Page 2026 indexes: CC **45 event links / 40 unique `peventid`** (5 IDs listed twice, once
  as `table=A` and once as `table=C`; 23 rows carry 2026 dates, 21 are red-flagged "not updated for
  2026" with 2025 dates, 1 is junk — `peventid=24` dated `1/11/71`); track **171 unique `peventid`
  links, all `table=C`, 160 of them dated `4/1/26`–`5/23/26`** (331 raw `peventid` string
  occurrences in the page); CC regional/district archive 2003–2021.

### Enumeration

**MileSplit — teams.** `GET https://oh.milesplit.com/teams` (server-rendered). Each row is
`href="https://oh.milesplit.com/teams/<TeamID>-<slug>"`, e.g. `55562-acad-for-urban-scholars` …
`9388-zanesville`. Team page carries `data-team-id="9360"`. No pagination observed; 977 links on the
single page. The filter is a `<select name="type" id="teamType">` (11 values, `1=High School`) and
the canonical URL form is `/teams?level=high-school` — the same 977 IDs.

**MileSplit — meets.** `GET /calendar?season=<cc|indoor|outdoor>&year=<YYYY>&month=<1-12>&level=<all-levels|high-school|middle-school|…>&type=<all-types|invitational|championship|…>`.
Server-rendered `<li class="meet-row" data-meet-id="710315" data-level="ms">` with
`meet-row__day`, `meet-row__name`, `meet-row__venue`. Month default is "Upcoming". Meet page URL
`/meets/<MeetID>-<slug>` (canonical for the sampled results page:
`https://oh.milesplit.com/meets/773056-seneca-easts-stars-stripes-and-lights-high-school-2026/results/1319069/formatted`).

**MileSplit — athletes and Class of 2027.** Two working paths:
1. `GET https://oh.milesplit.com/api/v1/rosters/teams/<TeamID>/ranked?formatRank=true&season=<cc|outdoor>&year=<YYYY>[&grade=<gradYear>][&gender=<M|F>]`
   → `data[]` rows carrying `athleteId`, `firstName`, `lastName`, **`gradYear`**, `teamId`,
   `eventCode`, `mark`, `units`, `stateRank`, `nationalRank`, `performanceId`, `profileUrl`.
   Two parsing gotchas measured on team 9412 (Versailles): `stateRank` is HTML, not an integer
   (`"2<span class='ordinal'>nd</span>"`), and `units` is the mark in **milliseconds** for running
   events (`mark: "14:54.90"` ↔ `units: 894900`; in meet performances the same convention holds —
   `1101700` = 18:21.70), while field-event rows carry millimetres.
   Season codes confirmed working: `cc`, `outdoor`. `grade=2027` filters to the 2026-27 senior class
   (= Class of 2027); it is a **graduation-year** filter, not a grade number (`grade=12` returns 0
   rows).
2. Athlete page `GET /athletes/<AthleteID>-<slug>` renders `<span class="grad-year">Class of 2027</span>`,
   school team link, a Personal Records table (event, mark, state/national rank, date) and one
   `<div class="season" data-season="cc|outdoor" data-level="hs|ms">` block per historical season,
   all server-side. For the sampled Class-of-2027 senior the page carried **10 season blocks**
   (`<h4>2026 - Cc</h4>`, `2026 - Outdoor`, `2025 …`, `2024 …`, `2023 - Cc`, `2023 - Outdoor` (ms),
   `2022 - Cc` (ms)). Marks inside those blocks are largely replaced by locked placeholders
   (`<span class="mask" role="img" aria-label="Locked, subscribe to view">`, 536 on the page), but the
   athlete+birth-class+team+season structure and the headline PRs are free.

Team-page filter selects document the contract: seasons `indoor`/`outdoor`/`cc`; years
`2026=2026-2027 … 2016=2016-2017`; grades `2027=2026-2027 … 2030=2029-2030`; gender `M|F`;
ranking filter `1=Top Rankings`.

**MileSplit — results.** `GET https://oh.milesplit.com/api/v1/meets/<MeetID>/performances` returns
the whole meet in one JSON document (no pagination links): `_embedded.meet` plus `data[]` of
performances with `id`, `meetId`, `place`, `teamName`, `teamProfileUrl`, `firstName`, `lastName`,
`profileUrl` (athlete), `eventType`, `units`, `millimeters`.

**Class-of-2027 enumeration does NOT require the paid leaderboards.** The public `/rankings/events/…`
and `/rankings/leaders/…` pages are paywalled past rank #1 (see Access characteristics); the roster
API above is the free, complete path per team.

**Finish Timing — meets/results.** Rendered homepage (`finishtiming.com`) lists "Upcoming Events" and
"Recent Results": each entry gives `athletic.net/…/meet/<ANID>/info`, `finishtiming.trackscoreboard.com/meets/20<ANID>/events`
and `finishtimingresults.com/<year>/20<ANID>.html`. Static archive: Apache autoindex
`https://finishtimingresults.com/2026/` (track files) and `/2026/CC/` → `/2026/CC/<MM-DD-CODE>/`
index pages linking per-race result files (XC: PDF; track: `.html` text).

**Baum's Page — meets/results.** `/cc/index.php` and `/track/index.php` list events as
`//www.baumspage.com/cc/ccevent.php?peventid=<N>&table=<A|C>` (CC) and
`//www.baumspage.com/track/trevent.php?peventid=<N>&table=C` (track). The event page then links
host-posted files, e.g. `//www.baumspage.com/cc/northmor/2026/hs boys results.htm`,
`http://www.baumspage.com/track/stjohns-delphos/2026/2026 Results.pdf`, and historical frames
`trframe.php?path=./stjohns-delphos/res11.htm`. Files are host-uploaded and can be non-results —
the 2026 "Shaker Heights Woodard-Richard Invitational" event page (`peventid=2`) linked only
`Meet Information.pdf` and `W9.pdf` (the latter an IRS W-9), i.e. no results at all.

### Stable identifiers

| Entity | MileSplit | Finish Timing | Baum's Page |
|---|---|---|---|
| Athlete | `athleteId` + `performanceId` (numeric, e.g. `10584778` in the roster API) and `id`; URL `/athletes/<id>-<slug>` (e.g. `…/athletes/12260210-jackson-spitzer`) | none — identity is name+school+grade | none |
| Team / school | `teamId` (numeric, e.g. `9412` Versailles) in URL and API | school name strings only | school name strings only |
| Meet | `meetId` (numeric, e.g. `773056`); canonical slug observed on the `oh.` host (`/meets/773056-seneca-easts-…/results/1319069/formatted`); no `www.milesplit.com/meets/…` link appeared in any capture | FT meet id = `"20" + Athletic.net MeetID` (28/28 checked); also `trackscoreboard.com/meets/<same id>/events`; native ids also exist (`649029`, `1000739707`, `2000631249`, `90000016`) | `peventid` (small int, scoped per sport and per `table=A|C`) |
| Result / performance | `performanceId` (`id` in performances API, e.g. `219122991`) | none (rows inside a per-race file) | none |
| Event | `eventCode` (observed set across the roster captures: `100H`, `100m`, `110H`, `1600m`, `200m`, `3000mSC`, `300H`, `3200m`, `400m`, `5000m`, `800m`, `D`, `HJ`, `LJ`, `Mile`, `S`, `TJ`), `eventType` (`R` running, `F` field, `T` relay/team) | Hy-Tek event numbers + event names inside files | Hy-Tek event numbers |
| Season | `season` (`cc`/`Indoor`/`outdoor`) + `seasonYear` (e.g. `2026`) | year directory (`/2026/`) + date-coded CC dir (`09-05-ASH`) | year directory in file path (`/2026/`) |
| Grade | `gradYear` (`2027`) in roster API; `Class of YYYY` in athlete HTML | numeric grade column (9–12) in results | Hy-Tek `Year` column usually **empty** |
| Athletic.net meet | not exposed (0 `athletic.net` references in every MileSplit page captured) | **exposed directly** as a link on the homepage; also encoded in FT/live meet ids | not exposed |

Names are not identifiers: MileSplit team names also appear as school abbreviations in results
(`Mass. Perry`, `Gah. Lincoln`), and Finish Timing uses abbreviations (`Wor. Kilbourne`,
`Lak. West`). The numeric IDs are the join keys.

### Athletic.net leverage

- **Direct Athletic.net links — Finish Timing only.** One fetch of the rendered Finish Timing
  homepage yielded **29 unique `athletic.net/…/meet/<ANID>/info` links**, and 28 of them were paired
  in the same page with an FT final-results URL. **All 28 pairs satisfy `FT id = "20" + ANID`**
  (e.g. AN 275818 → `/2026/20275818.html`; AN 275618 → `/2026/20275618.html`; AN 673916 →
  `/2026/20673916.html`). The same page's 27 `trackscoreboard.com/meets/<id>/events` links use the
  identical `20<ANID>` value. So for any meet Finish Timing serves, an Athletic.net MeetID is
  obtainable **without any Athletic.net request**.
  Verified against the saved rendered capture `tools/ohio-independent/raw/ft-home-rendered.txt`
  (20,777 B): 29 unique AN MeetIDs (83 link occurrences), 28 AN→FT rows, 0 rule violations, 27
  `20<ANID>` live links. The homepage is a rolling "upcoming/recent" list, so the roster of meets
  changes daily — the rule, not the count, is the durable finding. One observed variant: AN
  275821's "Live Results" link points at `athletic.net/CrossCountry/meet/275821/results` instead of
  Track Scoreboard, so the live link is not always FT-hosted.
- **Caveat, verified:** the homepage's XC "Final Results" links point at `/2026/20<ANID>.html` (seen
  again in the 23:27 re-render, e.g. Madison Plains XC → `finishtimingresults.com/2026/20275818.html`), but
  for XC the files actually live under `/2026/CC/<MM-DD-CODE>/` — `20275618.html` and
  `20275674.html` both returned 404 while `2026/CC/09-05-ASH/` returned 200 with race PDFs. Treat
  the `20<ANID>` id as the *join key*, not as a guaranteed fetchable path; only 2 of 325 files in
  `/2026/` match the `20`+6-digit shape.
- **MileSplit exposes no Athletic.net links.** Zero `athletic.net` occurrences across every MileSplit
  page captured (home, calendar, teams, team, meet, athlete, rankings). Joins must be built from
  school + meet name + date + venue. MileSplit's calendar does carry the same OHSAA tournament meets
  that OHSAA runs through Athletic.net (e.g. MeetID 769837 "OHSAA Division 3 District - Columbus
  Grove", Oct 24 2026), so (date, meet name) pairs can seed an Athletic.net meet lookup
  [INFERENCE — the pairing itself was not verified against Athletic.net because Athletic.net is
  Cloudflare-403 from this machine].
- **OHSAA state T&F 2026 is Athletic.net-native**: the OHSAA "2026 Track-Field State Coverage" page
  links `https://www.athletic.net/TrackAndField/meet/656920/info`, `…/656920/entries`,
  `https://live.athletic.net/meets/74520` and a Hy-Tek final-results PDF (`OHSAA_State_2026_Final_Results.pdf`).
  OHSAA publishes the AN URL itself; assignment 19 owns the full map.
- **Request avoidance (measured inputs, extrapolation marked).** One roster request returns a whole
  team-season; a 10-team XC-2026 sample returned 118 athlete rows including 41 with `gradYear=2027`
  (≈4.1 Class-of-2027 athletes per request). Over the 977 HS teams that is a **~977-request Ohio
  HS sweep producing ~4,000 Class-of-2027 XC athletes** [INFERENCE: sample of 10 teams, XC season
  only, 9/10 teams non-empty]. Each of those athletes needs no Athletic.net *search* — at most one
  targeted profile request, and grade evidence is already present. Meet-level: one
  `/performances` request returned 1,298 rows for a single XC meet (640,059 bytes) and 513 rows for
  an indoor state meet; a full Ohio HS XC season (~156 HS-inclusive meets in the Sep–Dec window)
  is therefore ~156 requests [INFERENCE: count of HS-inclusive calendar rows].

### Athlete evidence

| Field | MileSplit | Finish Timing | Baum's Page |
|---|---|---|---|
| Name | yes (roster, performances, profile) | yes (result rows) | yes |
| Graduating class/grade | **yes** — `gradYear` in roster API; `Class of YYYY` in profile HTML | **yes** — numeric grade column (9–12) for HS results | `Year` column exists but empty in every file sampled |
| School | yes (`teamId` + name + profile URL) | yes (name/abbrev.) | yes (name) |
| City/state | athlete page `Versailles, OH`; team API returns city/state/county/zip | meet venue in header | meet venue |
| Gender/category | yes — API `gender`; separate HS-boys/HS-girls datasets | from race/division labels | from race labels |
| TF/XC distinction | yes (`cc`, `Indoor`, `outdoor` seasons) | yes (CC dir vs track dir) | yes |
| Indoor/outdoor | yes at meet level (`season":"Indoor"` observed for the OATCCC D1 indoor state meet) | yes (trackscoreboard/live + year dirs) | partial (indoor meets appear in track index) |
| Performances | yes (`mark`, `units` ms) | yes | yes |
| PRs | yes — PR table with mark + date; state/national rank locked | derivable | derivable |
| Progression | yes — per-season result blocks (2022–2026 for the sampled athlete), "College Progression Tracker" section | per-season archives | per-season archives |
| Meets | yes (`meetId`, meet name, date) | yes | yes |
| Profile URL | yes `https://oh.milesplit.com/athletes/<id>-<slug>` | none | none |
| Result-row detail free? | **No** — per-meet rows on the athlete page are `<span class="mask" aria-label="Locked, subscribe to view">`; PR table is free | yes, fully free | yes, fully free |

### Recruiting information

None of the Ohio independent sources in this slice is a coach directory:
- MileSplit team pages/templates contain no coach blocks; the team API (`/api/v1/teams/<id>/schedules`)
  returns school-level `phone`, `address`, `zipCode`, `city`, `state`, `county` (institution data,
  not a coach contact) plus a `profileUrl`. No coach names observed.
- MileSplit team pages do link the school athletics/site where one is registered — e.g. team
  `9360-ada` links `http://www.ada.k12.oh.us`. That is a school-website seed, not a contact.
- Finish Timing/Baum's Page event pages link only the event host's contact form/meet manager.
- OATCCC publishes association-level material (awards, Hall of Fame, coaches poll, "OATCCC-Ohio-Recruiting-Guide.pdf",
  an athlete-facing recruiting guide) and links the **2026 OATCCC D1–D4 indoor state meets to
  MileSplit**, but no per-school coach directory was observed.
→ Coach/AD contacts must come from OHSAA/school/district sources (assignments 19 and 29).

### Result evidence

| Field | MileSplit `/api/v1/meets/<id>/performances` | Finish Timing static results | Baum's Page files |
|---|---|---|---|
| ResultID | `id` (e.g. `219122991`) | no | no |
| AthleteID | embedded in `profileUrl` (`/athletes/12260210-…`); explicit `athleteId` only via roster API | no | no |
| MeetID | `meetId` | AN MeetID embedded in the id/URL for FT meets | `peventid` (page), not in the result file |
| EventID | `eventCode` / `eventType` (`R`,`F`,`T`) — no event-number id | Hy-Tek event numbers | Hy-Tek event numbers |
| Mark | `units` (ms for running events; mm for field events, with an extra `millimeters` field e.g. `66000`/`1680`) | mark as printed (`14:54.90`, `16:07.1`) | mark as printed |
| Normalized inputs | `units` + `millimeters` (strings) | text only | text only |
| Timing method | not labelled per result; leaderboard filter exposes `accuracy=fat|…` | header "Licensed to Finish Timing" | header names the timing host |
| Wind | not observed in any field sampled | not observed in sampled files | not observed |
| Implement/hurdle spec | not observed | event name only (`Womens 100 Meter Hurdles Open`) | event name only |
| Heat / round | not observed (no heat/round field); `place` present | **yes** — `Prelims`/`Finals` sections and `H#` columns | same as Finish Timing (same Hy-Tek format) |
| Place | `place` | yes | yes |
| Date | `_embedded.meet.dateStart/dateEnd` (meet level) | meet date + page timestamp | meet date |
| School represented | `teamName` + `teamProfileUrl` | school column | school column |
| Relay membership | **team-level only** — relay rows have `eventType:"T"`, null `firstName`/`lastName`, `teamName` + team link; no leg names | **leg names present** in relay blocks (individual leg rows) | leg names present |
| Grade in results | not in performances (only roster API / athlete page) | **yes, numeric 9–12 column** | column exists but empty |

### Incremental use

Weekly collector paths, all delta-friendly (no historical re-fetch):

1. **MileSplit new/changed meets** — one request per (season, month):
   `GET /calendar?season=cc&year=2026&month=10&level=high-school` → 133 rows (Oct 2026) with
   `data-meet-id`; diff against stored IDs to detect new meets. Then one request per changed meet:
   `GET /api/v1/meets/<MeetID>/performances`. Every API response carries `_meta` — observed
   `{"created": 1789877853, "cache": {"fresh": true, "ttl": 0}, "status_code": 200}` — giving a
   server-side freshness stamp per response.
2. **MileSplit athlete refresh** — one request per team-season:
   `GET /api/v1/rosters/teams/<TeamID>/ranked?season=cc&year=2026&grade=2027`. Re-polling only the
   teams whose meet results changed bounds work to the affected schools.
3. **Finish Timing** — one rendered-homepage fetch gives "Upcoming Events" + "Recent Results" with
   AN MeetIDs and FT/live URLs (the weekly new-meet feed). For the archive, `GET
   https://finishtimingresults.com/2026/CC/` returns an Apache autoindex whose entries carry
   `Last modified` timestamps (e.g. `09-22-MP/ 2026-09-19 16:52`): one request yields the full delta
   of newly added meet directories; per-meet index then lists race PDFs.
4. **Baum's Page** — one request each to `/cc/index.php` and `/track/index.php` returns the full 2026
   event list with dates; new `peventid`s are the delta; the event page then reveals host-posted files.
   There is no per-file timestamp on the index, so re-polling a known event page is the only way to
   see updated result files.

### Access characteristics

| Source | Class | Notes |
|---|---|---|
| MileSplit team index / calendar / team page / athlete page | **normal HTML** (server-rendered) | no auth; rankings pages carry `<meta name="robots" content="noindex,nofollow">` |
| MileSplit `/api/v1/...` | **public structured JSON** (undocumented) | discovered from `js.sp.milesplit.com/ms18/teams/index.js` (`v1/rosters/teams/<id>/ranked`, `v1/teams/<id>/schedules`) and `drivefaze/loadResultsNew.js` (`v1/meets/<id>/performances`); no key, cookie or token required; responses `application/json` |
| MileSplit ranked leaderboards (`/rankings/events/…`) | **subscription-restricted in practice** | page advertises a **1,000-performance universe** and 50 rows/page; page 1 = 1 free row (rank 1) + 49 `<tr class="rankings-row lk" aria-hidden="true">` masked rows carrying `<span class="redact">` / `<span class="mask" aria-label="Locked">` (49/294 spans on p1); page 5 (rank 201+) has 50/50 masked; page 60 renders identically to page 1 (clamped). So only the overall rank-1 row per event/filter is free |
| MileSplit leader pages (`/rankings/leaders/…`) | **free HTML** | no masking at all (0 `redact`/`mask` spans): one leader row per event group with grade (`SR`) and date; the group rows (e.g. `5K`, `Other`) are readable |
| MileSplit athlete "Results" blocks / state+national ranks | **subscription-restricted** | 536 `<span class="mask" aria-label="Locked, subscribe to view">` placeholders on the sampled profile; PR table, season structure and `Class of YYYY` are free |
| Finish Timing homepage | **browser application** (SvelteKit; raw HTML is a 2,437-byte shell) | content needs JS or a rendering reader; no login |
| `finishtimingresults.com` archive | **static HTML / downloadable PDF** | Apache autoindex; XC pages are PDFs, track pages are Hy-Tek plain text in `<pre>` |
| Track Scoreboard live | **browser application** | Angular SPA over Firebase Realtime DB (`track-scoreboard-default-rtdb.firebaseio.com`), plus `api.trackscoreboard.com` and `/api/live/session` seen in the bundle; live-only value |
| Baum's Page | **normal HTML + static files** | event pages, Hy-Tek `.htm`, host-posted PDFs |
| OATCCC | **normal HTML** | links to MileSplit meet pages |
| Ohio Runner | **normal HTML** | road-race calendar only → out of scope |

Published rate limits: none observed on any of these hosts (no `Retry-After` seen, no 429 responses).
Observed server errors/limits today: `finishtimingresults.com/2026/20<ANID>.html` → 404 for XC meets;
`www.neohstrack.com` → TLS hostname mismatch; `oh.milesplit.com/meets` (no id) → 404, and
`/rankings` → 301 to `/rankings/leaders/high-school-boys/cross-country` (so the season slug in nav is
`cross-country` / `indoor-track-and-field` / `outdoor-track-and-field`, not `/outdoor-track`).

Politeness accounting (counted from the saved captures below plus probes noted without a saved body):
`oh.milesplit.com` **43 saved captures + ≈4 probe requests ≈ 47** — the only host anywhere near the
~50-request guidance, so remaining Ohio MileSplit work should be batched (roster calls are the ones
worth spending); `finishtimingresults.com` 16; `www.baumspage.com` 9; `js.sp.milesplit.com` 8;
`www.ohsaa.org` + `ohsaaweb.blob.core.windows.net` 7; `finishtiming.trackscoreboard.com` 3;
`www.finishtiming.com` 1 direct + 1 via a rendering reader; `buckeye-timing.com` /
`cantstoptiming.com` 2 each; `tfmeetpro.com` 2; `www.oatccc.com` 2; every other host ≤2. All
requests were sequential with ≥1 s spacing; **no 429 and no `Retry-After` was observed on any
host.**

Tool failure note (verbatim, per brief rule): a `web_search` call for additional Ohio timing
providers returned `Sign up and repeat your request.` — no provider list from web search is included;
all provider claims here come from direct HTTP observations.

### Recommendation

- **MileSplit Ohio → ATHLETIC.NET-SEED (primary).** It is the only Ohio source that yields a
  per-athlete identity *plus* verified graduating class *plus* school, at one request per
  team-season, and it can also serve as **RESULT-SOURCE** through
  `/api/v1/meets/<MeetID>/performances`. Expected marginal coverage: 977 HS teams; 10-team sample
  gave 41 Class-of-2027 athletes in the 2026 XC season (≈4.1/team-season) → roughly 4,000
  Class-of-2027 XC athletes from ~977 requests, plus the outdoor-track equivalent from the same
  teams [INFERENCE]. It also reaches school programs whose meets Athletic.net never logged, because
  MileSplit's own results come from timers (TFMeetPro etc.), not from Athletic.net.
- **Finish Timing → RESULT-SOURCE.** Free, grade-bearing results (numeric grade column) for Ohio
  invites, and the only source observed that hands over Athletic.net MeetIDs for free (29 IDs from
  one homepage fetch; `20<ANID>` join verified 28/28). Use it to seed Athletic.net meet lookups and
  to fill meet result gaps without touching Athletic.net. Footprint measured today:
  `/2026/CC/` holds **32 XC meet directories for the season to date** (Aug 1 – Sep 22) and `/2026/`
  holds **325 result files** across all levels (HS/MS/JH/college/USATF/other) for the year to date,
  so a full-year HS-only Ohio footprint in the low hundreds of meets is plausible but **not**
  measured [INFERENCE].
- **Baum's Page → RESULT-SOURCE (secondary) / DISCOVERY-ONLY.** Broad small-school meet
  enumeration with 2003–2026 archives, but the Hy-Tek `Year` column is empty in every sampled file,
  so it cannot verify Class of 2027 on its own. Worth keeping for meet discovery in NW/central OH.
- **Track Scoreboard → VALIDATION (live only).** Useful during a meet weekend; the static archive
  from Finish Timing supersedes it afterwards.
- **OATCCC → DISCOVERY-ONLY.** Its indoor state-meet pages point into MileSplit (MeetIDs 718766 D1,
  718840 D2, 718841 D3, 718842 D4, 718843 seated — plus a stale 2023 throws-camp link 509094) and
  its coaches poll is a team-ranking seed.
- **Ohio Runner → REJECT** for HS TF/XC (road races only).
- **Coach contacts: not available from any source in this slice** — route to OHSAA/school/district
  directories (assignments 19/29).

Biggest gaps in this slice: (1) no Ohio independent source exposes coach contacts; (2) MileSplit's
free data stops at rank #1 on leaderboards and at masked per-meet rows on athlete pages, so
"progression" beyond PRs is paid; (3) MileSplit's roster endpoint returned zero rows for `season=indoor`
on two programs (Walnut Hills, Sycamore), so indoor athlete enumeration must go through indoor
meet `/performances` instead [INFERENCE: indoor meet results do work — 513 rows for MeetID 718766];
(4) Finish Timing's XC "Final Results" URL pattern is inconsistent with its own directory layout, so
the `20<ANID>` id must be resolved through `/YYYY/CC/` rather than fetched directly.

---

### Evidence appendix

| URL | method | HTTP status | what it proved | timestamp (local) |
|---|---|---|---|---|
| `https://oh.milesplit.com/` | curl GET | 200 | Ohio MileSplit live; home page is server-rendered with meet/team/article links | 2026-09-19 23:12 |
| `https://oh.milesplit.com/rankings` | curl GET | 301 → `/rankings/leaders/high-school-boys/cross-country` | Season slug for XC is `cross-country` | 2026-09-19 23:12 |
| `https://oh.milesplit.com/meets` | curl GET | 404 | No bare meet index; meets come from `/calendar` and `/meets/<id>-<slug>` | 2026-09-19 23:13 |
| `https://oh.milesplit.com/calendar` | curl GET | 200 (416,645 B) | 294 `li.meet-row[data-meet-id]` rows; filters season/year/month/level/type | 2026-09-19 23:13 |
| `https://oh.milesplit.com/teams` | curl GET | 200 (303,979 B) | 977 unique HS TeamIDs; no pagination; filter select `name="type" id="teamType"` with option `1=High School` | 2026-09-19 23:13 |
| `https://oh.milesplit.com/teams?level=high-school` | curl GET | 200 (304,039 B; same 977 IDs) | Canonical `level=` form of the team index resolves to the same HS universe | 2026-09-19 23:13 |
| `https://oh.milesplit.com/teams/9360-ada` | curl GET | 200 | `data-team-id="9360"`; filter option values (`indoor/outdoor/cc`, `2027=2026-2027`, `M/F`, `1=Top Rankings`); school site link `ada.k12.oh.us` | 2026-09-19 23:14 |
| `https://oh.milesplit.com/api/v1/rosters/teams/9360/ranked?formatRank=true` | curl GET | 200 JSON | Roster API exists and returns `data[]` | 2026-09-19 23:14 |
| `https://oh.milesplit.com/api/v1/teams/9360/schedules?season=cross-country&year=2026` | curl GET | 200 JSON | Team metadata: name, city, state, county, phone, address, zip, gender, profileUrl | 2026-09-19 23:14 |
| `https://oh.milesplit.com/api/v1/rosters/teams/9412/ranked?formatRank=true&season=cross-country&year=2026` | curl GET | 200 JSON | 28 rows; `gradYear` present (2027×11, 2028×7, 2029×5, 2030×5) | 2026-09-19 23:17 |
| `https://oh.milesplit.com/api/v1/meets/773056/performances` | curl GET | 200 JSON (640,059 B) | Whole-meet results: 1,298 rows, `id`/`place`/`teamName`/`profileUrl`/`units`; `_embedded.meet` metadata | 2026-09-19 23:17 |
| `https://oh.milesplit.com/api/v1/meets/718766/performances` | curl GET | 200 JSON | Indoor meet results work (513 rows); `eventType` R=242, F=181, T=90; relay rows are team-level with null names | 2026-09-19 23:24 |
| `…/api/v1/rosters/teams/9425/ranked?...&season=outdoor&year=2026` | curl GET | 200 JSON | 78 outdoor-2026 athletes, 28 with `gradYear=2027` | 2026-09-19 23:21 |
| `…/rosters/teams/9425/ranked?...&season=cc&year=2026&grade=2027` | curl GET | 200 JSON | `grade=2027` returns 12/12 rows with `gradYear=2027` — grad-year filter works | 2026-09-19 23:22 |
| `…/rosters/teams/9425/ranked?...&grade=12` | curl GET | 200 JSON (0 rows) | `grade=` is a graduation year, not a grade number | 2026-09-19 23:21 |
| `…/rosters/teams/9425/ranked?...&season=indoor&year=2026` and `…10107…&season=indoor&year=2025|2026` | curl GET | 200 JSON (0 rows each) | Indoor not served by the roster endpoint for two sampled programs | 2026-09-19 23:21–23:23 |
| 10 × `…/api/v1/rosters/teams/<id>/ranked?...&season=cross-country&year=2026` (9884, 9323, 9845, 9575, 9702, 9524, 10026, 10170, 10071, 9425) | curl GET | 200 JSON | 118 athletes, 41 with `gradYear=2027`; 9/10 teams non-empty (Shroder empty) | 2026-09-19 23:21 |
| `https://oh.milesplit.com/rankings/events/high-school-boys/cross-country/5000m?year=2026&grade=senior&page=1|5|60` | curl GET | 200 (213 KB each) | Pager text `1 – 49 of 1000` (50 rows/page, 1,000-row universe); page 1 = rank 1 real ("1 Jackson Spitzer Versailles SR … 14:54.90") + 49 masked rows; page 5 starts at rank 201 and is 50/50 masked; page 60 re-renders the rank-1 row (clamped to page 1) | 2026-09-19 23:16 |
| `…/rankings/events/…` (row markup) | parse | — | Masked rows carry 49 `<span class="redact">` + 294 `<span class="mask">` spans and 4 `paywall` markers on page 1; only the overall rank-1 row is unmasked | 2026-09-19 23:16 |
| `https://oh.milesplit.com/rankings/leaders/high-school-boys/cross-country?year=2026&accuracy=fat&grade=senior…` | curl GET | 200 | Leaders page = 6 `<tr>` (2 group headers + leader rows); **0 `redact`/`mask` spans → unmasked**; leader row shows `5K Jackson Spitzer Versailles SR … 14:54.90`; grade filter select values `senior/junior/…/returners` | 2026-09-19 23:16 |
| `https://oh.milesplit.com/athletes/12260210-jackson-spitzer` | curl GET | 200 (268,994 B) | `<span class="grad-year">Class of 2027</span>`, school link, PR table (5K 14:54.90); 10 server-rendered season blocks (`<div class="season" data-season=cc|outdoor data-level=hs|ms>` with `<h4>2026 - Cc</h4>` … `2022 - Cc`); 536 `<span class="mask">` placeholders (`aria-label="Locked, subscribe to view"`) | 2026-09-19 23:17 |
| `https://oh.milesplit.com/meets/773056/results` | curl GET | 200 | Meet page canonical `/meets/773056-…/results/1319069/formatted`; links `tfmeetpro.com` (results provider) | 2026-09-19 23:14 |
| `https://oh.milesplit.com/calendar?season=cc&year=2026&month=10&level=high-school` | curl GET | 200 (215,892 B) | 133 HS-labeled Ohio meets in Oct 2026 incl. OHSAA district meets 769837/769830 | 2026-09-19 23:22 |
| `https://oh.milesplit.com/calendar?season=cc&year=2026&month=11&level=high-school` | curl GET | 200 | No OHSAA state/regional rows matched in the November window | 2026-09-19 23:22 |
| `https://js.sp.milesplit.com/ms18/teams/index.js` | curl GET | 200 JS | Endpoint contracts `v1/rosters/teams/<id>/ranked` + `v1/teams/<id>/schedules`; paywall branch on `stateRank`/`nationalRank` | 2026-09-19 23:15 |
| `https://js.sp.milesplit.com/drivefaze/loadResultsNew.js` | curl GET | 200 JS | Endpoint `v1/meets/<meetId>/performances` | 2026-09-19 23:15 |
| `https://js.sp.milesplit.com/drivefaze/api.js` | curl GET | 200 JS | API base `/api/` (site-relative) and `https://api.prod.milesplit.com/int/v3/`; the latter 404s for the v1 roster path | 2026-09-19 23:15 |
| `https://www.finishtiming.com/` | curl GET + render | 200 (2,437 B raw) | SvelteKit shell; rendered capture saved as `raw/ft-home-rendered.txt` (20,777 B): 29 unique `athletic.net/…/meet/<ANID>/info` links, 28 rows also carrying an FT results URL, 27 `trackscoreboard.com/meets/20<ANID>/events` links | 2026-09-19 23:13 (re-rendered and re-verified at 23:27, identical counts) |
| `https://finishtimingresults.com/2026/` | curl GET | 200 (59,293 B) | Apache autoindex: 325 result files with `Last modified` (275 six-digit, 38 `2000`+6, 2 `20`+6, 10 other); `CC/` subdir present | 2026-09-19 23:13 |
| `https://finishtimingresults.com/2026/CC/` | curl GET | 200 (6,877 B) | 32 date-coded meet dirs (`08-01-CEN`…`09-22-MP`) + 3 loose files | 2026-09-19 23:20 |
| `https://finishtimingresults.com/2026/CC/09-05-ASH/` | curl GET | 200 | Per-meet index: 6 race PDFs (`boys-hs-div-1-2.pdf` …) with start times; title "Ashland Arrows Cross Country Invitational, September 5, 2026" | 2026-09-19 23:20 |
| `https://finishtimingresults.com/2026/CC/09-19-TRI/` | curl GET | 200 | 4 race PDFs (hs/ms boys+girls) for the Sept 19 Triad meet | 2026-09-19 23:20 |
| `https://finishtimingresults.com/2026/CC/09-05-ASH/boys-hs-div-1-2.pdf` | curl GET + pdftotext | 200 (737,479 B) | Results carry `Place|Name|Grade|Team|Time|Points` with grades 9–12 (e.g. Matthew Scipione, 12, Solon, 16:07.1) | 2026-09-19 23:21 |
| `https://finishtimingresults.com/2026/1000739707.html` | curl GET (capture saved as `raw/ft-1000739707.html`, 41,721 B) | 200 | Track result = Hy-Tek plain text, header `Licensed to Finish Timing`, `Name Year School Prelims H# Points` / `Finals H# Points` columns and `Prelims`/`Finals` sections (35 hits) with **numeric grade** `Year` values (e.g. `1 Whiteley, Pazeley 10 Unattached 15.31 2`) | 2026-09-19 23:30 |
| `https://finishtimingresults.com/2026/20670782.html`, `…/2000631249.html`, `…/649029.html` | curl GET | 200 | Id variants: USATF Masters, Larry Young JH Invitational, Northeast Ohio Open (college) — explains the different id shapes | 2026-09-19 23:13–23:14 |
| `https://finishtimingresults.com/2026/20275618.html`, `…/20275674.html`, `…/20275818.html` | curl GET | 404 (355 B each) | Homepage "Final Results" pattern `20<ANID>` is not a fetchable XC path | 2026-09-19 23:13–23:20 |
| `https://finishtiming.trackscoreboard.com/` and `/meets/20275818/events` | curl GET | 200 (74,310 B) | Angular SPA; `main-ZSI2ZECE.js` references `api.trackscoreboard.com`, Firebase RTDB and `/api/live/session` | 2026-09-19 23:13–23:22 |
| `https://www.baumspage.com/cc/index.php` | curl GET | 200 (40,551 B) | 45 `peventid` links / 40 unique (5 listed twice as `table=A`+`table=C`); 23 rows dated 2026, 21 red-flagged "not updated for 2026" (2025 dates), 1 junk (`peventid=24`, `1/11/71`); archive link "Archived Results prior to 2011" + regional/district years 2003–2021 | 2026-09-19 23:22 |
| `https://www.baumspage.com/track/index.php` | curl GET | 200 (82,986 B) | 171 unique `peventid` links (all `table=C`, 331 raw `peventid` occurrences), 160 dated `4/1/26`–`5/23/26`; "2011 Track - Archived Results prior to 2012"; indoor meets listed in the same index | 2026-09-19 23:22 |
| `https://www.baumspage.com/cc/ccevent.php?peventid=10&table=C` | curl GET | 200 | Event page model: host-posted files `//www.baumspage.com/cc/northmor/2026/hs boys results.htm` + per-year archive links | 2026-09-19 23:13 |
| `https://www.baumspage.com/cc/northmor/2026/hs boys results.htm` | curl GET | 200 | Hy-Tek text in `<pre>`; 104 athlete lines; **`Year` column empty** | 2026-09-19 23:18 |
| `https://www.baumspage.com/track/trevent.php?peventid=108&table=C` + `res11.htm` | curl GET | 200 / 200 | Track event hosts arbitrary files (incl. race PDFs and `trframe.php?path=./stjohns-delphos/res11.htm`); archived `res11.htm` is Hy-Tek text with relay legs and empty `Year` | 2026-09-19 23:23 |
| `https://www.baumspage.com/track/trevent.php?peventid=2&table=C` | curl GET | 200 (21,924 B; byte-identical to the 23:18 capture) | `peventid=2` = "Shaker Heights Woodard-Richard Invitational" 2026, linking only `Meet Information.pdf` + `W9.pdf` → event pages can carry zero results | 2026-09-19 23:29 |
| `https://www.ohsaa.org/Sports-Tournaments/Track-Field/2026-Track-and-Field/2026-Track-Field-State-Coverage` | curl GET | 200 | State T&F → `athletic.net/TrackAndField/meet/656920/info`, `live.athletic.net/meets/74520`, Hy-Tek final-results PDF | 2026-09-19 23:18 |
| `https://ohsaaweb.blob.core.windows.net/files/Sports/Track-Field/2026/OHSAA_State_2026_Final_Results.pdf` | curl GET + read | 200 (816,147 B) | Hy-Tek Meet Manager output with athlete grade numbers (`Celia Schulte 12`, `Maren Barnett 10`) — free grade evidence for state qualifiers | 2026-09-19 23:19 |
| `https://www.ohsaa.org/sports/cc` | curl GET | 200 | OHSAA XC page links "Athletic.net How To & Usage Policy"; no result provider listed | 2026-09-19 23:18 |
| `https://www.oatccc.com/Indoor-Track-Field/High-School-State-Meet/` | curl GET | 200 | 2026 OATCCC indoor state meets link to `oh.milesplit.com/meets/718766` (D1), `718840` (D2), `718841` (D3), `718842` (D4), `718843` (seated), plus a stale 2023 throws-camp link `509094`; links finishtiming.com; no coach directory | 2026-09-19 23:20 |
| `https://tfmeetpro.com/` | curl GET | 200 | MeetPro = DirectAthletics meet-management software; results uploaded by host; `results.php` is not an Ohio index (404) | 2026-09-19 23:19 |
| `http://www.ohiorunner.com/` | curl GET | 200 | Road-race calendar only → REJECT for HS TF/XC | 2026-09-19 23:19 |
| `https://www.neohstrack.com/` | curl GET | 000 (TLS error 60) | Hostname certificate mismatch; source unusable without cert bypass | 2026-09-19 23:22 |
| `web_search` (Ohio timing providers) | tool call | n/a | Tool returned `Sign up and repeat your request.` verbatim; no web-search-derived provider list used | 2026-09-19 23:21 |
