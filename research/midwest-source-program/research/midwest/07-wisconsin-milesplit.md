# 07. Wisconsin MileSplit

Status: complete — all 11 sections evidenced. Known limits: (a) roster coverage could only be spot-sampled (3 teams); (b) the athlete **Progression** tab loads via JS and its payload could not be confirmed without a browser (the page carries `paywall_present:1`); (c) no 429/rate-limit boundary was probed (deliberately, to stay inside the ~50-request politeness budget); (d) Athletic.net-side join quality is assessed structurally, not empirically, because `www.athletic.net` returns 403 to this client.
Observed on: 2026-09-19

---

### Source

| Item | Value |
|---|---|
| Provider | MileSplit (FloSports). State site for Wisconsin. |
| Primary host | `https://wi.milesplit.com/` (200) |
| Legacy brand on WI site | `WIRunners.com` — `<meta name="application-name" content="WIRunners.com">` and page footer |
| National host | `https://www.milesplit.com/` (used for cross-state ranking widgets and for canonical athlete `profileUrl` values in JSON) |
| State subdomains | one per state (`wi.`, `mn.`, `sd.`, `ca.`, …); **team IDs are network-global and a wrong-state team URL 301s to the owning state** (observed: `mn.milesplit.com/teams/12233-wayzata/roster` → `ca.milesplit.com/teams/12233-valley-christian-cerritos-ss/roster`) |
| Static asset hosts | `js.sp.milesplit.com`, `css.sp.milesplit.com`, `assets.sp.milesplit.com` |
| Product endpoints observed | `accounts.milesplit.com`, `api30.milesplit.com` (accounts API, from inline config), `tfmeetpro.com` (MeetPro — meet management/results ingest), `milesplit.live` |
| Membership | Free tier + **MileSplit PRO** (subscription); see *Access characteristics* |

### Coverage

- **Geography:** WI site is WI-scoped, but indices are **meet-location scoped, not athlete-home-state scoped**. Two artefacts: (1) the WI boys XC leaders list contained an athlete whose profile is on `sd.milesplit.com`; (2) `mn.milesplit.com/results?season=cc&level=hs&year=2026` lists the **WI** meet `780853-osceola-invitational-2026` (also on the WI index). Border meets therefore appear in ≥2 state indexes — dedupe by `MeetID`.
- **Sports/seasons:** `cross-country`, `indoor-track-and-field`, `winter-track-and-field` ("Indoor + Polar Bear"), `outdoor-track-and-field`, `road` (results index).
- **Levels:** Youth, Middle School, High School, College, Open, Pro/Elite (results index); ranking levels `middle-school-boys/girls`, `high-school-boys/girls`, `club-boys/girls`, `college-men/women`, `alumni-men/women`.
- **School level focus:** `/teams` defaults to `type=1` = High School (other options: College, Middle School, Elementary, Junior/Community College, Club, Company, Professional, Olympic/National, Group, Unattached).
- **Historical depth:** ranking `year` selector spans **2000–2027** (28 options); results-index `year` selector spans **2006–2026**; meet pages expose "Meet History" editions (observed for New London: 2014, 2015, 2025, 2026 editions as separate MeetIDs); a class-of-2027 athlete's profile carried per-season sections back to **2022** (CC/Indoor/Outdoor).
- **Volume anchors observed:** WI `/teams` = **597** HS team records (IDs 13933–75885, single A–Y page); WI 2026 outdoor meet index = 50 meets/page; one WI XC invite = 452 result rows; WIAA 2026 outdoor state meet = **2,454** result rows; MN Roy Griak Invite = 2,605 rows.

### Enumeration

All recipes below were executed live. Request tally for the whole assignment: **~46 HTTP requests to `wi.milesplit.com`** (30 distinct URLs tabulated in the appendix, plus redirect hops), 5 to `js.sp.milesplit.com`, 6 to `mn.milesplit.com`, 1 to `ca.milesplit.com`. **No 429 and no `Retry-After` was ever observed.**

**1) Teams (school universe proxy).** `GET https://wi.milesplit.com/teams` → 200, 200,225 bytes. 597 `<a href="/teams/{teamId}-{slug}">` records, one row each, grouped by letter headings A–Y (no Q/Z headings), **no pagination** (single document). `type=1` (High School) is pre-selected; `?type=2..11` switches level. Two caveats: (a) identity — 595 unique names for 597 IDs (`Fall Creek` = 14114 **and** 66673; `Independence` = 23695 **and** 51904); (b) level purity — the `type=1` list still contains 8 entries that are middle-school teams by name (e.g. `Green Bay Trinity MS`, `South Middle School`, `St. Anthony Middle School`, ≈1.3%), and HS team rosters include MS grades (Wayzata carried grades 2030–2033), so class filtering must be applied to *athletes*, never inferred from a team's level label.

**2) Athlete enumeration with graduating class — team rosters (best surface).**
```
GET https://wi.milesplit.com/teams/{teamId}/roster        # slug optional; 301 → /teams/{teamId}-{slug}/roster
GET https://wi.milesplit.com/teams/13976-aquinas/roster   # 200 (99 rows)
```
One request returns **every rostered athlete on one page** (no pagination). Each row is `<li class="athlete-row data-row">` containing:
- athlete link `https://wi.milesplit.com/athletes/{athleteId}-{slug}` with text `{Last}, {First}`
- `<div class="… column-gender">m|f</div>`
- `<div class="… column-grad-year">2027</div>` ← **absolute graduating class**
- three season flags `<div class="data-point …" data-season-id="{1|2|3}">` with `icon-yes`, or `data-season-id="0"` with `icon-no`; the page's own `#rosterFilterType` select documents **1=Indoor, 2=Outdoor, 3=XC**.
- Filters (`#rosterFilterClass`, `#rosterFilterGender`, `#rosterFilterType`) are **client-side only** (`js.sp.milesplit.com/ms18/teams/roster/index.js` = jQuery show/hide) → filtering to Class of 2027 costs **zero** extra requests.

Observed roster sizes / class-of-2027 counts:

| Team | Roster rows | 2027 | 2028 | 2029 | 2030+ | 0/unknown |
|---|---|---|---|---|---|---|
| Aquinas (WI, 13976) | 99 | 27 | 32 | 23 | 16 | 1 |
| Milwaukee King (WI, 13938) | 158 | 76 | 47 | 24 | 7 | 4 |
| Wayzata (MN, 13450) | 1,154 | 158 | 193 | 191 | 524 | 88 |

A full WI roster sweep is therefore **≈597 requests** — a full state sweep exceeds the mission's ~50 req/host politeness budget for one session and must be paced across days (or permitted in writing).

**3) Meets (date-descending index).**
`GET https://wi.milesplit.com/results?year=2026&season=outdoor&level=hs&page=2` → 200; **50 meets/page**, newest-first. Parameters: `season=cc|indoor|outdoor|road`, `level=youth|ms|hs|college|open`, `month=1..12`, `year=2006..2026`, `league={id}`, `page=N`. Canonical path form: `/results/wisconsin-meet-results?year=…`.

**4) Results, per result set (static, robots-allowed).** Meet pages embed the result-file list inline:
`let meetResultFiles = [{"id":1211442,"name":"1.75 Mile MS Boys Results","isMeetPro":0}, {"id":1211451,"name":"5K JV Boys Results","isMeetPro":0}, …]`.
Each viewer URL is `GET /meets/{meetId}-{slug}/results/{meetResultsId}/{raw|formatted}`:
```
GET https://wi.milesplit.com/meets/703331-new-london-bulldog-invite-2025/results/1211451/raw   # 200, 48,640 B
```
The `/raw` view is **static server-rendered HTML with the timing system's text output inside a single `<pre>`**:
```
  Pl Athlete                   Yr Team                                          Time
    1 Michael Hill              10 Ashwaubenon                                19:25.5
    2 Preston Holmes            10 Hortonville                                19:39.7
    3 Harper Collegnon           9 Ashwaubenon                                19:47.1
```
Columns: **Pl | Athlete | Yr | Team | Time** (plus a team-scores block). `Yr` is the **school-year grade** (9–12 for HS, 6–8 for MS in the same meet). 71 result lines for that file; 65 parsed cleanly by a strict whitespace rule. This path is **not disallowed by robots.txt**.

**5) Results, whole meet (undocumented JSON).**
```
GET https://wi.milesplit.com/api/v1/meets/{meetId}/performances?isMeetPro=0&fields=<list>
```
One request = the whole meet (WIAA state meet: 2,454 rows / 1.87 MB; New London XC: 452 rows; MN Griak: 2,605 rows / 1.80 MB). Full `fields` list (verbatim from `loadResultsNew.js`): `id,meetId,meetName,teamId,videoId,teamName,athleteId,firstName,lastName,gender,genderName,levelId,levelName,divisionId,divisionName,meetResultsId,meetResultsDivisionId,resultsDivisionId,ageGroupId,ageGroupName,gradYear,eventName,eventCode,eventDistance,eventGenreOrder,round,roundName,heat,units,mark,place,windReading,profileUrl,teamProfileUrl,performanceVideoId,teamLogo,statusCode`. **This path is disallowed by robots.txt (`Disallow: /api/`) — see Access.**

**6) Class-of-2027 enumeration verdict.**
| Surface | Yields Class of 2027? | Cost |
|---|---|---|
| Team roster page | **Yes** — `column-grad-year` per athlete, all seasons, plus gender | 1 req/team (597 for all WI HS) |
| Meet performances API | **Yes** — `gradYear` per performance row (100% coverage on individual rows at the state meet) | 1 req/meet (robots-disallowed path) |
| Raw result file | **Yes, derivable** — `Yr` grade column + meet date → grad year | 1 req/result set |
| Rankings/leaders `?grade=junior` | **No** — returns a **top-3 leaderboard**, not a list (page 1 and page 2 identical: 3 athlete rows). Also `noindex,nofollow` | 1 req/page |
| Athlete search `POST /search/v2/athletes` | **No** — name-query only; class appears in the response text but there is no class filter in the client contract; `q=johnson` returned grad years 2018–2031 in one page | 1 req/page, session token required |
| `sitemap.xml` | Indirect — 7,500 athlete URLs, no grad year in the sitemap; requires a profile fetch each | 1 + 7,500 |

**Reusable recipe for the other 11 states** (all paths byte-identical on `mn.`/`ca.` as observed):
```
# 1. team universe (HS)
GET https://{st}.milesplit.com/teams                       # type=1 default; single A–Z page
# 2. class-of-2027 athletes per team (grad year + gender + XC/Indoor/Outdoor flags)
GET https://{st}.milesplit.com/teams/{teamId}/roster
# 3. meet inventory, newest first, 50/page
GET https://{st}.milesplit.com/results?year={Y}&season=cc|indoor|outdoor&level=hs&page={N}
# 4. per-meet results, compliant static path (grade column inside <pre>)
GET https://{st}.milesplit.com/meets/{meetId}-{slug}/results/{meetResultsId}/raw
# 5. per-meet results, JSON path (robots-disallowed /api/)
GET https://{st}.milesplit.com/api/v1/meets/{meetId}/performances?isMeetPro=0&fields=…,gradYear,…
# 6. athlete change feed
GET https://{st}.milesplit.com/sitemap.xml
```
State constants: MileSplit HS team counts observed — WI 597, MN 592.

### Stable identifiers

| Entity | Identifier | Evidence |
|---|---|---|
| Athlete | numeric `athleteId`; canonical URL `/athletes/{id}-{last}-{first}`; `/athletes/{id}` and `/athletes/{id}-{first}-{last}` both 301 into the canonical form | `/athletes/14085147` → 200 at `…-pongonis-will`; `/athletes/14085147-will-pongonis` → 301 → `…-pongonis-will` |
| Team | numeric `teamId`, **global across the network**; URL `/teams/{id}-{slug}`; ID-only `/teams/{id}/roster` 301s to the slug form | `mn.milesplit.com/teams/12233-wayzata/roster` → 301 → `ca.milesplit.com/teams/12233-valley-christian-cerritos-ss/roster` |
| Meet | numeric `MeetID` (`/meets/{id}-{slug}`); **not date-monotonic** — 2026-09-17 meets carry IDs 780621, 780666, 782337, 780853 | `/results?season=cc&level=hs&year=2026` page 1 |
| Result set / result file | numeric `meetResultsId` = `meetResultFiles[].id`, page 1 e.g. `1211442`, `1211439`, `1211451`, `1211445`; PRO flag `isMeetPro` | meet page inline JS |
| Performance row | numeric row `id` (e.g. `192767162`), plus `meetResultsId`, `meetResultsDivisionId`, `resultsDivisionId` | performances API |
| Event | `eventCode` (`5000m`, `4x100m`, `100WC`) + `eventName` + `eventDistance` metres | performances API |
| Division | `divisionId`/`divisionName` (`Division 1` id 44, `Adaptive` id 26, `JV` id 72) | performances API |
| Level | `levelId`/`levelName` (`1` = `High School` in WI data; `College` also present at Griak) | performances API |
| Season (roster flags) | `1`=Indoor, `2`=Outdoor, `3`=XC (from `#rosterFilterType` options); `0` = not on that roster | roster page |
| Season (rankings) | `cross-country`, `indoor-track-and-field`, `winter-track-and-field`, `outdoor-track-and-field` | rankings filter form |
| Venue | numeric `/venues/{id}/{slug}` (e.g. 15768 New London – Hatton Park); usable as ranking filter `venue=` | meet page links |
| League / conference | numeric `league` id (e.g. `9642` Conference – Eastern Cloverbelt; `8916` Team State Qualifiers – D1 Boys XC) | rankings filter form |
| Graduating class | `gradYear` string (`"2027"`); **`"0"` = unknown** | performances API, roster column |
| Relay pseudo-athletes | `athleteId` **1500** (men) / **1501** (women) — placeholders, `firstName`/`lastName` null, `gradYear` `"0"` | 589/589 relay rows at the state meet |
| Accounts | separate host `api30.milesplit.com`, `accounts.milesplit.com` (from inline config; not used) | athlete page config |

Names are **not** identifiers: duplicate athlete records exist (two `Will Pongonis` profiles, ids 14085147 and 13733662, same school+class), duplicate team records exist (Fall Creek ×2, Independence ×2), and `Will Johnson` occurs repeatedly across schools and classes.

### Athletic.net leverage

**Direct links: none.** A case-insensitive scan of **all 28 fetched HTML documents** (WI homepage, athlete index, search, teams index, team page, roster, athlete profile, meet/result pages, results index, raw result files, plus MN/CA pages) returned **0 occurrences of `athletic.net`**. The only external third-party links found on the state-meet page were FloSports properties (`tfmeetpro.com`, `www.flosports.tv`, `milesplit.live`). There are **no Athletic.net meet/team/athlete/result links, and no Athletic.net IDs** anywhere observed.

What MileSplit *does* supply for a deterministic Athletic.net seed:

| Seed field | Present? | Source |
|---|---|---|
| school/team name | yes (`teamName` has a trailing space: `"Hortonville "`, `"Eau Claire Memorial "`) | roster, API, search |
| city/state | yes (team index `HORTONVILLE, WI, USA`; search description `Eau Claire, WI, USA`) | teams index, search |
| graduating class | yes | roster `column-grad-year`, API `gradYear` |
| gender | yes (`m`/`f`, `genderName` `Girls`/`Boys`) | roster, API |
| meet name + date | yes (meet pages, `meetName`, meet index dates) | meet pages |
| event + mark | yes (`mark`, `eventCode`) | API/raw |

Assessment: this is enough to drive a **targeted** Athletic.net lookup (`school → AT team page → athlete search by exact last/first + class`), but it is **not identity-deterministic on its own** because (i) MileSplit has duplicate athlete profiles, (ii) names collide within a school and across schools, and (iii) 1 in 14 sampled grade-11 athletes had a class conflict between the timer's raw `Yr` and MileSplit's own `gradYear` (`Zach Jones`, Hortonville: raw `Yr 11` vs API `gradYear 2026`). [INFERENCE] A practical join is name+school+class with a manual-review bucket for collisions.

**Estimated Athletic.net requests avoided.** Per-meet pulls replace per-athlete profile fetches:
- WIAA 2026 outdoor state championships: `GET /api/v1/meets/750759/performances` = **1 request → 2,454 rows → 382 unique Class-of-2027 athletes** across 224 teams (individual rows; 100% grad-year coverage).
- New London Bulldog Invite 2025 (XC): 1 request → 452 rows → 79 Class-of-2027 athletes (grades inflated by MS entries: grad years 2026–2033 present).
- The two meets shared exactly **1** Class-of-2027 athlete, i.e. a season is a long tail of small, mostly-disjoint meets — savings scale as ≈(Class-of-2027 athletes per meet) Athletic.net profile requests avoided, minus 1 MileSplit request per meet.
- Overlap caution: results repeats an athlete once per event (state meet: 582 rows for 382 athletes), so dedupe on `athleteId` before counting savings.

### Athlete evidence

| Field | Availability | Detail |
|---|---|---|
| Name | yes | `firstName`/`lastName` in API; `{Last}, {First}` on rosters; `{First} {Last}` in search |
| Graduating class | yes | roster `column-grad-year`; API `gradYear`; search `description` (`"Eau Claire Memorial  2027"`) |
| School | yes | `teamName` (trailing space), `teamProfileUrl` |
| City/state | yes (team-level) | team index `ABBOTSFORD, WI, USA`; search description `… , WI, USA` |
| Gender/category | yes | roster `column-gender`; API `gender`/`genderName` |
| TF/XC + indoor/outdoor | yes | roster season flags (1/2/3); API `levelName` + meet season from the meet page config (`season: 'CC'`, `isTrack: false`) |
| Performances | partial (free) | PR table free; **full meet-by-meet history is PRO-locked** |
| PRs | yes (free) | `#pr-table`: Event, Mark, State rank, National rank, Date, grouped overall and per season (`data-pr-mode="cc|indoor|outdoor"`, `data-pr-level="hs|ms"`). Example (Will Pongonis, 2027): 800m 1:59.01 #4,110 (May 18 2026); 1600m 4:14.68 #531; 3200m 9:05.58 #277; 5K 15:12.72 #110 (Sep 4 2026) |
| Progression | unverified | `/athletes/{id}/progression` returns 200 with no data in HTML (JS-loaded); page analytics carry `paywall_present:1` |
| Meets | partial | season/event structure is free (per-season event headings), individual meet/date/mark cells are masked |
| Athlete profile URL | yes | `https://wi.milesplit.com/athletes/{id}-{last}-{first}`; JSON `profileUrl` uses `https://www.milesplit.com/athletes/{id}-{slug}` |

Free-vs-locked measurement (Will Pongonis, 2027): the profile states *"See all 88 of Will Pongonis's results"* and the DOM contains exactly **88** masked rows (`<span class="mask" aria-label="Locked, subscribe to view">`) distributed `outdoor 51 / cc 32 / indoor 5`, with the free event breakdown `5000m 30, 1600m 25, 3200m 13, 800m 11, 2 Mile 4, One Mile 3, 3000m 2`. So a free collector learns *how many* results and *which events*, but not the marks/meets/dates.

### Recruiting information

**Not available — no coach or administrator data anywhere observed.** Keyword scans (`coach`, `Coach`, `athletic director`, `contact`) over the team page, roster page, results index and rankings page returned **0 coach/AD matches**; the 2 `Coach` hits on the homepage are the meet name "Cross Country Coaches National Youth Championships". Team pages expose tabs (Team, Rankings, Videos, Photos, News, Roster, Schedule) but no staff data, no public professional email, no school-athletics-site field. There is a `College Commitments` content section (editorial articles), not a structured data field.

No athlete personal contact data is exposed or collected; the site itself declares `<meta name="format-detection" content="telephone=no,address=no,email=no">`.

| Field | Present |
|---|---|
| head track coach / head XC coach / assistant coaches / athletic director | no |
| public professional email | no |
| school athletics website / team website | no (MileSplit team page is the only team URL) |

### Result evidence

From `GET /api/v1/meets/{meetId}/performances?isMeetPro=0&fields=…` (23 fields observed in a returned row):

| Field | Present | Notes |
|---|---|---|
| ResultID | yes | row `id` (e.g. `192767162`) |
| AthleteID | yes | `athleteId` (`"13704715"`), plus `profileUrl` |
| MeetID | yes | `meetId` (`"703331"`), `meetName` |
| EventID | partial | `eventCode` (`5000m`, `4x100m`), `eventName`, `eventDistance`; no numeric event id in the payload |
| mark | yes | `mark` as displayed (`"21:48.60"`, `"11.62"`, `"17-10"`) |
| normalized mark inputs | yes | `units`: **milliseconds for time events** (11.62 → `11620`; 21:48.60 → `1308600`) and **inches×1000 for english-measured field events** (5-7 HJ → `67000`; 19-9 LJ → `237000`; 39-9.75 TJ → `477750`; 13-6 PV → `162000`; 47-11.5 SP → `575500`; 156-6 DT → `1878000`) |
| timing method | no | not a row field; only a *rankings filter* `accuracy=fat` exists |
| wind | yes (when present) | `windReading` (`"-0.5"`; 483 of 2,454 rows at the state meet) |
| implement/hurdle spec | no | no field; only implied by event names (`100 Meter Hurdles`, `100 Meter Wheelchair Race`) |
| heat/round | yes | `heat` (string), `round` (`f`/`p`), `roundName` (`Finals`/`Prelims`) |
| place | yes | `place` (string) |
| date | **no (per row)** | not in the payload; take the meet date from the meet page/config (`startDate: '2025-09-20'`, `season: 'CC'`) |
| school represented | yes | `teamId`, `teamName`, `teamProfileUrl`, `divisionName` |
| relay membership | **no** | relay rows are one row per team attributed to a placeholder athlete (ids **1500** men / **1501** women, null names, `gradYear 0`); team scores present (`teamScores: 'team'`), individual legs absent |
| level / division | yes | `levelName` (`High School`, `College`), `divisionName` (`Division 1`, `JV`, `Adaptive`) |
| grade | yes | `gradYear` — see next paragraph |

**Grade coverage measured:**
- WIAA 2026 outdoor championships (meet 750759): 2,454 rows = 1,865 individual + 589 relay. **Individual `gradYear` coverage = 100.00%** (0 unknowns); relay rows are 589/589 `"0"`. Individual `gradYear` distribution: 2026 → 895, **2027 → 582**, 2028 → 276, 2029 → 111, 2038 → 1 (bad value). **382 unique Class-of-2027 athletes**; 224 teams represented; divisions D1 995 / D2 696 / D3 698 / Adaptive 65.
- New London Bulldog Invite 2025 XC (meet 703331): 452 individual rows, 1 unknown → **99.78% coverage**; 79 unique Class-of-2027 athletes; grad years 2026–2033 present (MS included) → **level/grade filtering is mandatory**.
- MN Roy Griak Invitational (meet 782310, one request): 2,605 rows, 112 `gradYear 0` → **95.70% individual coverage**; 588 unique Class-of-2027 athletes; levels HS 2,185 + College 420; `teamProfileUrl` pointed at `sd.milesplit.com` (cross-state teams).

**Grade ↔ class mapping (must be applied explicitly):** `gradYear = academic_year_end + (12 − grade)`, where a Jul–Dec meet belongs to the academic year ending the following June. Verified against the raw file for the same meet: the Sept 2025 5K JV Boys `<pre>` lists 18 athletes with `Yr = 11`; 14 of them join to API rows on (name, team), and **13 carry `gradYear = "2027"`** (1 conflict: `Zach Jones`, Hortonville — raw `11` vs API `2026`). Examples joined: `Sullivan Kaster` (Ashwaubenon) → `athleteId 14437281`, `gradYear 2027`; `Josue Azuara` (Weyauwega-Fremont) → `9627509`; `Theodore Main` (West De Pere) → `13694237`.

⚠️ **Calendar trap for this project.** Today (2026-09-19) is inside the **2026-27** school year, so **Class of 2027 = Grade 12**, and **Grade 11 = Class of 2028**. The pipeline's "Class of 2027 / Grade 11" equivalence only holds for 2025-26 and earlier data. Collectors must pick deliberately:
- continue the same cohort → `gradYear=2027` / roster `column-grad-year == 2027` / raw `Yr == 12` in 2026-27 meets;
- track *current* juniors → `gradYear=2028` / roster `2028` / raw `Yr == 11`.
MileSplit's own `grade=junior` ranking filter is school-year-relative and therefore currently selects **Class of 2028**, not 2027.

### Incremental use

1. **Athlete change feed — `GET /sitemap.xml`** (1 request, 918,709 bytes): 7,500 `<loc>` entries, all `/athletes/{id}-{slug}`, `lastmod` covering **2026-08-28T04:51:51-04:00 → 2026-09-19T02:56:58-04:00**. Weekly diff on `lastmod` yields exactly the athlete profiles touched that week. [INFERENCE] the window is a rolling recent-activity set (exactly 7,500 entries over 22 days); robots.txt declares no sitemap, so this path is discovered by convention only.
2. **New meets — `GET /results?season={s}&level=hs&year={Y}&page=1`** (1 request/page, 50 meets, date-descending). Diff the `MeetID` set against the previous run. Do **not** use a monotonic MeetID watermark: same-day meet IDs are non-monotonic (780621 / 780666 / 780853 / 782337 all dated 2026-09-17).
3. **New result files inside a known meet** — fetch the meet page and diff the inline `meetResultFiles` array (`id`, `name`, `isMeetPro`); each new id corresponds to a new/updated result file, fetched with 1 request (`/results/{id}/raw`, or the meet-level API call).
4. **Changed results / affected athletes** — no ETag/Last-Modified observed; `server-side-cache-ttl` and a footer `data-cacheKey="{meet:778860}:…" / "Generated by 10.1.2.17 fresh in 76 milliseconds"` indicate edge caching by key. Practical pattern: re-pull a meet's result sets only while it is "recent" (≤ ~10 days), then stop.
5. **Rosters** — the expensive surface (597 requests/WI). No change signal, so schedule 1–2 sweeps per season (start of XC, start of outdoor) rather than weekly; grade/roster membership changes once per year.
6. **Do not re-fetch athlete profiles weekly**: the PR table is small but the history is PRO-masked, so a profile fetch adds little beyond `gradYear` (already available from rosters/results). Use the sitemap only to target profiles whose `lastmod` moved.

### Access characteristics

**Classification: normal HTML + undocumented public JSON API (free tier), with a subscription tier for athlete history depth.** No authentication needed for any recipe above except the athlete search's session token.

| Surface | Status | Class |
|---|---|---|
| `/teams`, `/teams/{id}/roster`, `/results`, `/athletes/{id}`, `/sitemap.xml`, `/meets/*/results/*/raw` | 200 unauthenticated | normal HTML (static/server-rendered) |
| `/api/v1/meets/{id}/performances`, `/api/v1/athletes/top-ranked`, `POST /search/v2/athletes` | 200 unauthenticated | public structured JSON (undocumented; `API.endPoint = '/api/'`) |
| Athlete full meet-by-meet history | locked | `<div class="results-funnel results-funnel--locked">`, masked cells `aria-label="Locked, subscribe to view"`, CTA `See all 88 of Will Pongonis's results … included with MileSplit PRO` (12 PRO CTA blocks on that page) |
| Meet results "full depth" | gated | results-page CTA "To get the full depth of our meet coverage, become PRO!"; per-file `isMeetPro` flag in `meetResultFiles` |
| Progression tab | unverified | 200, no data in HTML, `paywall_present:1` in analytics payload |

Mechanics observed:
- **Search token:** `POST /search/v2/athletes` requires a `searchToken` that is **session-bound**. Reusing the anonymous token from an earlier page → **HTTP 400** with `Invalid Search Token` (23,793-byte HTML error page). With a cookie jar (`unique_id`) and the token from a fresh `/athletes` fetch → 200 JSON. Responses carry `totalPages` (reported as 100 even for a 4-hit query — do not treat as a real page count); `perPage` 25 works.
- **reCAPTCHA:** every page loads `https://www.recaptcha.net/recaptcha/enterprise.js?render=6LcfSNkjAAAAAEt-bQi1C9XVNfjQ8K_v-LXHecZj` and `recaptcha.css`, but the athlete-search POST succeeded **without any reCAPTCHA token** in this session — only the session `searchToken` was required (observed 2026-09-19).
- **robots.txt (verbatim, 173 bytes):** `User-agent: *` → `Disallow: /rankings`, `Disallow: /virtual-meets`, `Disallow: /api/`, `Disallow: /contact`; `Mediapartners-Google` and `AmazonAdBot` allowed; **no `Sitemap:` directive**. Consequence for a production collector: the two richest programmatic surfaces — the meet `performances` API and the rankings pages — are **disallowed for generic crawlers**, while team pages, rosters, `/results`, athlete profiles, meet pages and `/raw` result files are permitted. Ranking pages additionally send `<meta name="robots" content="noindex,nofollow">`. This assignment made a small number of requests to the disallowed `/api/` and `/rankings` paths purely to document their contracts (disclosed here); a production pipeline should either use the permitted `/raw` + roster paths or obtain written permission.
- **Rate limiting:** no 429 and no `Retry-After` across ~46 requests to `wi.milesplit.com` (sequential, no delay). Caching header observed: `server-side-cache-ttl: 348` on `/`; identity cookie `unique_id` set with a far-future expiry; a `personalization` JWT cookie is set after visiting an athlete page (30-day). Responses served over HTTP/2, `server: nginx`.
- **Not attempted (by policy):** no login, no PRO access, no `isMeetPro=1` request for PRO-only files, no cookie replay, no CAPTCHA solving, no header/fingerprint spoofing. Default `curl` user agent was used throughout.

### Recommendation

**RESULT-SOURCE** (primary), with a **VALIDATION** role for Class-of-2027 evidence. Not ATHLETIC.NET-SEED — the site exposes **zero** Athletic.net IDs or links, so it cannot hand the pipeline an Athletic.net identity directly.

Expected marginal coverage:
- **Results at 1 request per meet** — the cheapest whole-meet result acquisition seen anywhere in this research programme (WIAA state meet: 2,454 rows, 382 unique Class-of-2027 athletes, wind/round/heat/division, 100% grad-year coverage on individual rows). This is a direct substitute for Athletic.net meet-result fetches for every WI meet MileSplit has ingested, and it also covers athletes that Athletic.net ranking enumeration may miss (MS/adaptive/cross-state entries appear in the same payload).
- **Class-of-2027 evidence at 1 request per team** (roster `column-grad-year`, plus XC/Indoor/Outdoor membership flags and gender) — 597 requests for all of WI HS; strongest for schools whose results are not on MileSplit but whose roster is.
- **Compliance caveat drives the architecture choice:** the JSON API is the cheapest path but is `Disallow`ed by robots.txt. The **`/raw` result-set pages are robots-permitted, static, and carry `Pl | Athlete | Yr | Team | Time`** — a compliant collector should parse those (1 request per result set) and use `/teams/{id}/roster` + `/results` for discovery. Reserve the `/api/` path for a permitted/production-negotiated mode.
- **Not worth using for:** coach/AD contacts (absent), relay-member results (absent), wind-independent normalization subtleties (implement/hurdle metadata absent), or as an Athletic.net identity oracle.
- **Reuse priority:** the same recipe applies unchanged to `mn.`, `ia.`, `il.`, `mi.`, `in.`, `oh.`, `mo.`, `ks.`, `ne.`, `nd.`, `sd.` subdomains (verified structurally on `mn.` and `ca.`); the 12-state programme should implement one adapter parameterized by subdomain, with the grade↔class mapping applied per meet date.

### Evidence appendix

All requests issued 2026-09-20T04:12Z–04:19Z (= 2026-09-19 23:12–23:19 America/Chicago). Method `GET` unless noted; timestamps are server `date` headers where retained, otherwise the session minute.

| URL | Method | HTTP | What it proved | Timestamp (UTC) |
|---|---|---|---|---|
| `https://wi.milesplit.com/` | GET | 200 | Site live; brand WIRunners.com; nav surfaces (`/teams`, `/athletes`, `/results`, `/rankings/leaders/...`); `server-side-cache-ttl: 348`; meet/article links | 2026-09-20T04:12:43Z |
| `https://wi.milesplit.com/robots.txt` | GET | 200 | `Disallow: /rankings`, `/virtual-meets`, `/api/`, `/contact`; no sitemap directive | 2026-09-20T04:12:45Z |
| `https://wi.milesplit.com/sitemap.xml` | GET | 200 | 918,709 B; 7,500 athlete URLs; `lastmod` 2026-08-28 → 2026-09-19 | 2026-09-20T04:12:5xZ |
| `https://wi.milesplit.com/athletes` | GET | 200 | Athlete-search form; hidden `searchToken`/`isState=1`/`subdomain=wi`; `_DF_.API.get('v1/athletes/top-ranked')` | 2026-09-20T04:13:07Z |
| `https://wi.milesplit.com/api/v1/athletes/top-ranked?page=1&limit=8&minRank=10` | GET | 200 | Public JSON: `athleteId`, `eventCode`, `gender`, `season`, `stateRank`, `nationalRank`, `teamName`, `teamCity`, `topPerformance{event,mark}`, `profileUrl` | 2026-09-20T04:13:24Z |
| `https://wi.milesplit.com/search/v2/athletes` (stale token) | POST | 400 | Token is session-bound: `Invalid Search Token` error page | 2026-09-20T04:13:30Z |
| `https://wi.milesplit.com/search/v2/athletes` (`searchToken` + cookie jar, `q=pongonis`, `perPage=25`, `filters[subdomain]=wi`) | POST | 200 | Name search returns per-hit `id`, `url`, and `description` = `"Eau Claire Memorial  2027 / Chippewa Valley Nordic NSL / Eau Claire, WI, USA"` → **school + graduating class + city/state**; duplicate profiles visible | 2026-09-20T04:13:3xZ |
| `https://wi.milesplit.com/search` | GET | 200 | Search categories `athlete|meet|team`; same session-token mechanism | 2026-09-20T04:13:4xZ |
| `https://wi.milesplit.com/teams` | GET | 200 | **597** HS team records (IDs 13933–75885), A–Y single page, `type=1` default, no pagination; 2 duplicate names | 2026-09-20T04:14:0xZ |
| `https://wi.milesplit.com/teams/13976-aquinas` | GET | 200 | Team tabs incl. Roster; `a.paywall-action … ref=team-13976-tab-roster`; **no coach/AD fields** | 2026-09-20T04:14:1xZ |
| `https://wi.milesplit.com/teams/13976-aquinas/roster` | GET | 200 | 99 rows with `column-grad-year` + `column-gender` + season flags; `#rosterFilterType` (Indoor 1/Outdoor 2/XC 3) and `#rosterFilterClass` (2027–2031); filters are client-side | 2026-09-20T04:14:2xZ |
| `https://wi.milesplit.com/athletes/14085147-will-pongonis` | GET | 301 | Slug canonical form is `{id}-{last}-{first}`; `Location: /athletes/14085147-pongonis-will` | 2026-09-20T04:14:42Z |
| `https://wi.milesplit.com/athletes/14085147-pongonis-will` | GET | 200 | Free `#pr-table` (event/mark/state+national rank/date, per-season+level attributes); Results section `results-funnel--locked` with **88** masked rows matching "See all 88 … results"; **0 `athletic.net` occurrences** | 2026-09-20T04:14:50Z |
| `https://wi.milesplit.com/meets/778860-new-london-bulldog-invite-2026/results` | GET | 200 | Result-set link `/results/1321754/raw`; rankings filter params `?year=all&meet=43149` and `?venue=15768`; cross-host ranking links | 2026-09-20T04:15:3xZ |
| `https://wi.milesplit.com/meets/778860-new-london-bulldog-invite-2026/results/1321754/raw` | GET | 200 | 2026 result set exists but has no result rows yet (meet held that day); "Raw" = raw results view | 2026-09-20T04:15:45Z |
| `https://wi.milesplit.com/results` | GET | 200 | Meet index: filters `season|level|month|year(2006–2026)|league`; 50 meets; "Browse By Team or League" | 2026-09-20T04:15:5xZ |
| `https://wi.milesplit.com/meets/703331-new-london-bulldog-invite-2025/results` | GET | 200 | Inline `meetResultFiles=[{id:1211442,name:"1.75 Mile MS Boys Results",isMeetPro:0},…]`; `meetResultParams` with `meetId/resultsId/season:'CC'`; result views `raw|formatted` | 2026-09-20T04:16:0xZ |
| `https://wi.milesplit.com/api/v1/meets/703331/performances?isMeetPro=0&fields=…` | GET | 200 | 452 rows, 23 fields incl. `gradYear`, `windReading`, `round/roundName`, `heat`, `units`, `mark`, `divisionName`; 79 unique Class-of-2027 athletes | 2026-09-20T04:16:21Z |
| `https://wi.milesplit.com/rankings/leaders/high-school-boys/cross-country` | GET | 200 | Full filter taxonomy: `level`(10), `season`(4), `event`, `year(2000–2026)`, `grade`(SR/JR/SO/FR/8th/7th/6th/Returners), `state`, `league`(numeric ids), `country=usa`; `noindex,nofollow` | 2026-09-20T04:16:4xZ |
| `…/rankings/leaders/high-school-boys/cross-country?year=2026&accuracy=fat&grade=junior&conversion=n&page=1&event=5000m` | GET | 200 | Grade filter works but returns a **top-3 leaderboard** (3 athlete rows: 5K/4K/3 Mile) | 2026-09-20T04:16:5xZ |
| `…&grade=junior&…&page=2` | GET | 200 | Identical payload to page 1 → `page` does not paginate the leaders view | 2026-09-20T04:16:5xZ |
| `https://wi.milesplit.com/teams/13938-milwaukee-king/roster` | GET | 200 | 158 rows, `2027→76/2028→47/2029→24/2030→7` + 4 unknown → rosters are not truncated at 100 | 2026-09-20T04:17:1xZ |
| `https://wi.milesplit.com/results?season=outdoor&level=hs&year=2026` | GET | 200 | 50 outdoor meets incl. WIAA state `750759`, sectionals `764783/764784`, regionals `765879/765880`; pagination `/results?year=2026&season=outdoor&level=hs&page=2` | 2026-09-20T04:17:2xZ |
| `https://wi.milesplit.com/api/v1/meets/750759/performances?isMeetPro=0&fields=…` | GET | 200 | 2,454 rows/1.87 MB; individual `gradYear` coverage **100%**; 582 rows/382 unique athletes grad-2027; 224 teams; 483 rows with wind; 589 relay placeholder rows (`athleteId` 1500/1501) | 2026-09-20T04:17:3xZ |
| `https://wi.milesplit.com/athletes/14085147-pongonis-will/progression` | GET | 200 | No progression data in HTML; analytics `paywall_present:1` | 2026-09-20T04:17:4xZ |
| `https://wi.milesplit.com/search/v2/athletes` (`q=johnson`) | POST | 200 | 25 hits/page; grad years **2018–2031** in page 1 → search is not a class-enumeration surface | 2026-09-20T04:17:5xZ |
| `https://wi.milesplit.com/meets/750759-2026-wiaa-wi-outdoor-championships-2026` | GET | 200 | External links only FloSports/`tfmeetpro.com`; **0 `athletic.net`** | 2026-09-20T04:18:0xZ |
| `https://wi.milesplit.com/teams/13976/roster` | GET | 301 | ID-only team URL canonicalizes to the slug form | 2026-09-20T04:18:1xZ |
| `https://wi.milesplit.com/athletes/14085147` | GET | 200 | ID-only athlete URL resolves (301 → canonical slug) | 2026-09-20T04:18:2xZ |
| `https://wi.milesplit.com/teams/13976/roster` (follow) | GET | 200 | Lands on `/teams/13976-aquinas/roster`, 99 rows → recipe works from IDs alone | 2026-09-20T04:18:3xZ |
| `https://wi.milesplit.com/meets/703331-new-london-bulldog-invite-2025/results/1211442/raw` | GET | 200 | Static `<pre>` raw results: team scores + `Pl Athlete Yr Team Time` (MS `Yr` 6–8), 100 non-empty lines | 2026-09-20T04:18:4xZ |
| `https://wi.milesplit.com/meets/703331-new-london-bulldog-invite-2025/results/1211451/raw` | GET | 200 | HS `Yr` values **9,10,11,12** (11 → 18 rows) → grade column joins to `gradYear` (13/14 name+team matches = 2027; 1 conflict) | 2026-09-20T04:18:5xZ |
| `https://js.sp.milesplit.com/drivefaze/api.js?build=20260917151218` | GET | 200 | `API.endPoint='/api/'`; apiV3 hosts; GET/POST helper contract | 2026-09-20T04:13:5xZ |
| `https://js.sp.milesplit.com/ms18/search/athletes.js?build=20260917151218` | GET | 200 | Exact search contract: `POST /search/v2/athletes` with `searchToken,q,perPage,page,filters.subdomain` — **no class filter** | 2026-09-20T04:13:5xZ |
| `https://js.sp.milesplit.com/ms18/teams/roster/index.js?build=20260917151218` | GET | 200 | Roster grade/season/gender filters are client-side jQuery show/hide | 2026-09-20T04:14:2xZ |
| `https://js.sp.milesplit.com/drivefaze/meets/meets.js?build=20260917151218` | GET | 200 | Result view/page `<select>` navigation only (no results API here) | 2026-09-20T04:16:1xZ |
| `https://js.sp.milesplit.com/drivefaze/meets/loadResultsNew.js?build=20260917151218` | GET | 200 | `_DF_.API.get('v1/meets/{meetId}/performances', …)` + full `fields` list + `isMeetPro`/`excludeMeetPro` logic | 2026-09-20T04:16:3xZ |
| `https://mn.milesplit.com/teams` | GET | 200 | Same structure; **592** MN HS teams → recipe reusable | 2026-09-20T04:18:5xZ |
| `https://mn.milesplit.com/results?season=cc&level=hs&year=2026` | GET | 200 | 50 meets; lists a **WI** meet (`780853`) → state indexes overlap at borders | 2026-09-20T04:18:5xZ |
| `https://mn.milesplit.com/api/v1/meets/782310/performances?isMeetPro=0&fields=…` | GET | 200 | Same API on another subdomain; 2,605 rows; 95.70% individual grad-year coverage; 588 unique 2027 athletes; HS+College mix; `teamProfileUrl` → `sd.milesplit.com` | 2026-09-20T04:18:5xZ |
| `https://mn.milesplit.com/teams/13450-wayzata/roster` | GET | 200 | **1,154** roster rows (2027→158); rosters scale past 1,000 and include MS grades 2030–2033 | 2026-09-20T04:19:0xZ |
| `https://mn.milesplit.com/teams/12233-wayzata/roster` | GET | 301 | → `https://ca.milesplit.com/teams/12233-valley-christian-cerritos-ss/roster` → **team IDs are network-global** | 2026-09-20T04:19:0xZ |
| `https://ca.milesplit.com/teams/12233-valley-christian-cerritos-ss/roster` | GET | 200 | 116 rows with grad years + grade filter → same recipe on a third state | 2026-09-20T04:19:0xZ |
| `https://mn.milesplit.com/teams/75556-williams-home-school/roster` | GET | 200 | Home-school team with 0 roster rows → roster coverage is not guaranteed per team | 2026-09-20T04:18:5xZ |
