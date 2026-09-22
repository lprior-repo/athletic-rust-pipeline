# SOURCE_REPORT — MileSplit (national)

Lane: `research/sources/milesplit-national/`. Scope: the whole MileSplit network (51 state subdomains + `www`), not one state.
Wave 1 = captures already on disk when this lane started (prior phase). Wave 2 = this lane, `2026-09-22T03:54Z–04:00Z`, 83 HTTP requests (38 logged in `samples/fetch-log-wave2.tsv` + 45 unkept `/teams` bodies counted in `samples/teams-count-sweep.tsv`), all HTTP 200 except the four failures listed in `samples/CAPTURES.md`.
Everything below cites a file in `samples/` or a named outside path; `[INFERENCE]` marks reasoning, `[inherited]` marks prior-phase facts not re-measured here.

---

## 1. Source name

MileSplit (FloSports). Hosts: `https://<usps-code-lowercase>.milesplit.com` per state and `https://www.milesplit.com` for the national channel. Asset hosts `js.sp.milesplit.com` / `css.sp.milesplit.com` / `assets.sp.milesplit.com`.
Evidence: `samples/homepage.html` (`milesplit_site_code":"www"`, `site_id":"67"`), `samples/robots-oh-milesplit.txt`, `samples/state-subdomain-probe.txt` (51/51 codes answer 200 on `/teams`; `newengland`, `dcmdva`, `national` answer 500).

## 2. Geographic coverage

- **All 51 U.S. jurisdictions have a working state host and a populated HS team index**: measured, one `/teams` GET per code, 26562 HS team records in total (`samples/teams-count-sweep.tsv`; every row HTTP 200).
- `www` is a real but *complementary* channel, not a superset: an athlete page requested on `www` canonicalises to the state host (`samples/athlete-www-13966799.html` → canonical `https://ny.milesplit.com/athletes/13966799-abigail-burt`), and the national athlete search on `www` returns hits whose own `subdomain` is a state code (first 25 hits of `q=Fuller`: `ga` 4, `tx` 2, `al` 2, `ma` 2, … — `samples/search-v2-athletes-www.json`), i.e. `www` is a router over the state hosts rather than a separate corpus. `[inherited]` report 33 found `www`'s 7,500 sitemap slots resolve to jurisdictions with no state host (DC, Canada).
- Coverage is **unequal in practice**, not just in team counts: `[inherited]` report 27 measured Michigan rosters as effectively empty (2 of 3 sampled teams returned 0 athletes) while WI/OH/IA/MO/NE/SD sampled cleanly.
- Per-jurisdiction numbers: `coverage.json` (51 rows: teams measured, prior-CSV cross-check, sampling evidence, gaps).

## 3. Sports

Cross country, indoor track & field, outdoor track & field, all on the same templates.
Evidence: the results index exposes `ddSeason ∈ {cc, indoor, outdoor, road}` (`samples/results-oh.html`); the calendar is per-season (`samples/calendar-oh.html` → `/calendar/ohio-cc-meet-calendar`); rankings URLs are `/<level>/<season>[/<event>]` (`samples/js-rankings.js`); a profile renders cc/indoor/outdoor season blocks (`samples/athlete-oh-15005189.html`).
Levels and types, verified in the captures: the results index `ddSeason` options are `{cc, indoor, outdoor, road}` plus an empty default (`samples/results-oh.html`); its `ddLevel` options are `{youth, ms, hs, college, open, pro}` plus empty; calendar rows carry `data-level` values `hs` (101), `ms` (97), `hs,ms` (41), empty (27), `hs,ms,youth` (6), `ms,youth` (6), `hs,ms,open` (3), `ms,youth,open` (3), `hs,open` (3), `hs,ms,youth,open` (3), `youth` (1), `youth,open` (1), `ms,open` (1), `open` (1) (`samples/calendar-oh.html`).
Team types are a `/teams` filter, not a row attribute: `<select name="type" id="teamType">` offers 11 options — `1` High School (**selected by default**), `2` College/University, `3` Middle School, `4` Elementary, `5` Junior/Community College, `6` Club, `7` Company, `8` Professional Team, `9` Olympic/National Team, `10` Group, `11` Unattached (`samples/teams-oh.html`). Consequence: the 26,562-record national sweep in §2 is the **High School slice only**; every other type is a separate request (`/teams?type=N`).

## 4. Historical depth

- Meet/result archive reaches back **20 years**: the results index `ddYear` carries 22 options — an empty default plus `2006`–`2026` (`samples/results-oh.html`). The large numeric option values on the same page (`2092`, `2094`) belong to `ddLeague` and are league IDs (Greater Western Ohio Conference), **not** years. The page also carries `ddSeason`, `ddLevel`, `ddMonth` selects.
- A single athlete profile keeps **every season they competed** with per-season headings (sample: 2022 cc → 2026 cc/indoor/outdoor, 12 season blocks in `samples/athlete-oh-15005189.html`), and every result row is masked, so the profile's *history* is free only as counts, not as marks.
- Result files themselves: a free `/raw` file carries the whole result set including `Yr` (grade) where the timer supplied it (`samples/raw-oh-770621-rs1321880.txt`: 80 result rows in 2 sections, 40 boys + 40 girls, every row carrying place / name / `Yr` / team / mark). `[inherited]` report 32: 5 of 6 sampled raw files carry `Yr`; the IHSA state file does not.
- Sitemap `lastmod` is **not** history: `[inherited]` report 33 (13 hosts, 7,500 URLs each, `/athletes/` only) — "Historical depth: none… `lastmod` describes page rewrite time".

## 5. Discovery mechanism

Ranked by measured cost:

1. **`GET /teams` per state (1 request, no pagination)** — the complete HS team frame with `TeamID`. 26562 records across 51 jurisdictions (`samples/teams-count-sweep.tsv`).
2. **`GET /teams/<TeamID>-<slug>/roster` (1 request per team)** — the Class-of-2027 enumeration surface: every rostered athlete with `AthleteID`, grad year, gender and Indoor/Outdoor/XC flags (`samples/roster-oh-mason.html`, 319 rows). `[inherited]` report 34 sampled 50 rosters in WI/MN: 4,883 athletes, 1,029 Class of 2027, grad year present on 95.9 % of rows.
3. **`GET /results?season=&level=&year=&page=N` (1 request per 50 meets)** — date-ordered meet index with `data-meet-id` and the results URL (`samples/results-oh.html`); `GET /calendar/<state>-<season>-meet-calendar` is the forward-looking twin, 294 meet rows in `samples/calendar-oh.html`.
4. **`POST /search/v2/athletes` (robots-allowed JSON, anonymous)** — name → `AthleteID` lookup, national or per-state. See §13; this corrects `[inherited]` report 27, which assumed the search shortcut sat behind the disallowed `/api/`.
5. `GET /sitemap.xml` (7,500 `/athletes/` rows per host, `lastmod`) — change signal only `[inherited]` report 33.

## 6. Stable identifiers

| Entity | Identifier | Shape / example | Where it appears (free) |
|---|---|---|---|
| Team | `TeamID` | `9573` — `https://oh.milesplit.com/teams/9573-fairless` | team index, roster `athlete` row's team link, meet/result HTML, rankings rows |
| Athlete | `AthleteID` | `15005189` — `https://oh.milesplit.com/athletes/15005189-brice-fuller` | roster row, search JSON `hits[].data.id`, rankings rows, profile URL |
| Meet | `MeetID` | `771577` / `770621` — `/meets/<id>-<slug>` | results index `data-meet-id`, calendar `data-meet-id`, meet page |
| Result set | `RSID` | `1321880` — `/meets/<id>/results/<RSID>/{raw,formatted}` | meet results page `meetResultFiles[].id` |
| Season participation | `data-season-id` | `1`=Indoor, `2`=Outdoor, `3`=XC (sibling order in the captured header) | roster row |
| Site | `milesplit_site_id` / `milesplit_site_code` | `"36"`/`"oh"`, `"67"`/`"www"`, `"33"`/`"ny"` | page-call metadata on every page |
| Deployment | `?build=` | `20260921153357` | every `js.sp.milesplit.com` script tag |

Slugs are **not** identifiers (they change); the numeric prefix is. Raw result files carry **no** IDs — names + team strings only.

## 7. Pagination

| Surface | Mechanism | Measured |
|---|---|---|
| `/teams` | none — single page per state | 977 OH team rows in one response (`samples/teams-oh.html`); client-side `#txtFilter` only |
| `/teams/<id>/roster` | none — whole roster in one page | 319 rows (`samples/roster-oh-mason.html`) |
| `/results` | `?&page=N`, 50 meets/page, `rel=next` | capture shows page 1 with `href="/results?&page=2"` (`samples/results-oh.html`) |
| `/calendar` | month buckets `data-month`: `2026-09`, `2026-10`, `2026-11`, `2026-12`, `2027-10` (an XC calendar spans this fall plus a stray next-October bucket) | 294 rows, one response |
| `/search/v2/athletes` | `page` + `perPage`; **`totalPages` caps at 100** | `q=Fuller` and `q=Smith` both report `totalPages: 100`; `perPage=100` is honoured (100 hits returned) → `[INFERENCE]` the reachable ceiling is 100 × 100 = 10,000 hits per query, since a 101st page was not requested (`samples/search-v2-athletes-oh-perpage100.json`, `…-oh-fuller-p2.json`, `…-oh-smith-p1.json`) |
| `/rankings/...` | `?page=N` in the URL pattern | `page=1` present in both rankings canonicals; row 1 free, everything else masked (§18) |
| `/sitemap.xml` | none — hard 7,500-URL window | `[inherited]` report 33 |

## 8. Athlete fields

Free on a roster row: `<li class="athlete-row data-row">` with 6 data points in header order **Athlete | Gender | Class | Indoor | Outdoor | XC** (`data-point data-heading` labels): athlete name + `AthleteID` link (`"Adesanya, Inioluwa"` → `/athletes/16846610-inioluwa-adesanya`), `column-gender` (`"f"`), `column-grad-year` (`"2029"`), then one season cell per sport — the cell carries `data-season-id="1|2|3"` + `<svg class="icon icon-yes">` when the athlete is active in that season, and `data-season-id="0"` + `<svg class="icon icon-no">` when not (over the 319 rows: 426 `icon-yes` / 531 `icon-no`, and the yes counts per id are 90 indoor, 156 outdoor, 180 XC). Sample file `samples/roster-oh-mason.html`.
**The page's three filters are client-side only** — `samples/js-ms18-teams-roster-index.js` calls `filter()` on load and shows/hides the already-served `<li>` rows (`$rosterFilterClass` compares `column-grad-year` text; `$rosterFilterType` tests `[data-season-id="<val>"]` presence). So **one request returns the whole roster regardless of sport, grade or gender**; the captured page loads with `rosterFilterType=3` (XC) selected, i.e. a JS-enabled view would hide the 139 non-XC athletes among the 319. Filter vocabularies are still useful as field domains: `rosterFilterType` = `''|1 Indoor|2 Outdoor|3 XC`, `rosterFilterClass` = `''|2027|2028|2029|2030`, `rosterFilterGender` = `''|F|M`.
Free on a profile: `#athleteName` (`"Brice Fuller"`), `class="grad-year"` (`"Class of 2028"`), `class="current-school"` (Fairless + TeamID link), `class="city-state"` (`"Brewster, OH"`), 12 season blocks (`<div class="season" data-season="cc|indoor|outdoor" data-level="hs|ms">` each with an `<h4>` heading — `2022 - Cc` … `2026 - Cc/Indoor/Outdoor`), per-event headings (`class="event-heading"`), and the PR table (`pr-td-event` `"800m"`, `pr-td-mark` `"2:00.70"`, national rank `"#6,790"`, state rank locked).
**Not free anywhere:** the athlete's per-result mark/place/round/meet/date. The profile's result listing is 96 rows of `<div class="record row " … data-season data-level data-event>`, and each row masks 5 cells (`seed`, `position`, `round`, `location`, `date`) — 480 empty `<span class="mask …">` elements in total, **0 of which carry text** (`samples/athlete-oh-15005189.html`; reproduce: `grep -o 'class="record row "' … | wc -l` → 96, `grep -o 'class="mask' … | wc -l` → 480, `grep -o 'class="mask[^"]*"[^>]*></span>' … | wc -l` → 480).
Search JSON gives `title` (name), `description` (`"Ellet 2016 / Akron, OH, USA"` — school + a year in free text), `subdomain`, `url`, `id`. No labelled grad-year, gender or team field.
No personal contact data on any captured surface.

## 9. Meet fields

| Field | Example | File |
|---|---|---|
| `MeetID` + slug URL | `771577` → `/meets/771577-wooster-xc-invitational-hs-2026/info` | `samples/meet-oh-771577.html` |
| name | `"Wooster XC Invitational-HS"` (JSON-LD `name`, `<h1>`) | same |
| dates | `"startDate": "2026-09-12"`, `"endDate": "2026-09-12"`; visible `<time>Sep 12, 2026</time>` | same |
| host team | `"homeTeam": "Wooster"` | same |
| sport | `"sport": "Cross Country"` | same |
| venue | `"name": "Wooster HS Track"`, `addressLocality "Wooster"`, `addressRegion "OH"` | same |
| index row | `<li class="meet-row" data-meet-id="770621" data-filter-text="beaver eastern invite beaver oh">` → `<span class="meet-row__day">Sep 19</span>`, `<a class="meet-row__name" href="https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results">Beaver Eastern Invite</a>`, `<span class="meet-row__venue">Beaver, OH</span>`, `<a class="meet-row__results" …>Results</a>`, grouped by `<section class="meet-month" data-month="2026-09">` | `samples/results-oh.html` |
| calendar row | `<li class="meet-row" data-meet-id="778691" data-level="hs,ms" data-filter-text="caldwell fall classic caldwell oh">` → `<span class="meet-row__day">Sep 19</span>`, `<a class="meet-row__name" href="https://www.milesplit.com/meets/778691-caldwell-fall-classic-2026">Caldwell Fall Classic</a>`, `<span class="meet-row__venue">Caldwell, OH</span>`, empty `<span class="meet-row__links">` | `samples/calendar-oh.html` |
| result files | `meetResultFiles = [{"id":1321880,"name":"Results","isMeetPro":0}]` + `meetResultParams {meetId, resultsId, isTrack:false, startDate:"2026-09-19", season:"CC", phantomRunnerRule:false, teamScores:"team", followedAthleteIds:[], followedTeamIds:[], teamLogoFallback}` | `samples/meet-oh-770621-results.html` |

Absent on meet pages: coach/contact fields, entry lists in HTML (`/entries` is a JS shell — `samples/meet-oh-770621-entries.html`, 0 rows), division metadata in HTML.
`[inherited]` report 27: `/meets/<MeetID>/entries` is free and unlocked but carries **no** `AthleteID` and no grade.

## 10. Result fields

The free result payload is the **`/raw` text file**, one request per result set:
header `     Athlete                   Yr Team                                          Mark      H#`, rows `    1 Jeydyn Fields              8 Jackson                                    12:40.6` — place, name, `Yr` (grade), team, mark, heat (`samples/raw-oh-770621-rs1321880.txt`: 80 result rows — 40 boys + 40 girls in two `Boys/Girls Middle School 3000 Meter` sections — so identity, school and grade are all present, free, in one request per result set).
Missing from `/raw`: `AthleteID`, `TeamID`, wind for XC, structured division identity.
The structured equivalent (with `athleteId`, `gradYear`, `windReading`, `performanceVideoId`, `statusCode`, …) exists only on the disallowed JSON path — its **field list is quoted from the site's own bundle**: `js-loadresultsnew.js` requests `v1/meets/<meetId>/performances` with `fields: 'id,meetId,meetName,teamId,videoId,teamName,athleteId,firstName,lastName,gender,genderName,levelId,levelName,divisionId,divisionName,meetResultsId,meetResultsDivisionId,resultsDivisionId,ageGroupId,ageGroupName,gradYear,eventName,eventCode,eventDistance,eventGenreOrder,round,roundName,heat,units,mark,place,windReading,profileUrl,teamProfileUrl,performanceVideoId,teamLogo,statusCode'`.
The formatted view itself contains 0 rows in HTML — it is a JS shell (`samples/meet-oh-770621-results.html`).
`[inherited]` report 32: all 18 result files seen across 7 meet pages carried `isMeetPro: 0`.

## 11. Grade/class evidence

Four independent encodings, in decreasing reliability:

1. **Roster `column-grad-year`** — `"2029"`, one cell per athlete, present on 319/319 rows in the OH capture; `[inherited]` report 34 measured 95.9 % presence over 50 rosters and distribution `2027:1029 · 2028:1067 · 2029:922 · 2030:746 · 2031:519 · 2032:273 · 2033:112 · 2034:15` of 4,883 rows.
2. **Profile `class="grad-year"`** — `"Class of 2028"` for athlete 15005189 — i.e. the OH #1 XC leader in the rankings capture is **not** Class of 2027, so grade must be read per athlete, never assumed from ranking position.
3. **`/raw` `Yr` column** — numeric `8` in the OH capture, present only when the timer's file has it.
4. Roster **season flags** (`data-season-id` 1/2/3) tell *which sport* an athlete is active in, which is a different question from grade.

Behind the paywall the same value is named `gradYear` / `hsGraduationYear` in the site's own request/response handling (`samples/js-loadresultsnew.js`, `samples/js-ms18-teams-rankings.js`).

## 12. Coach/contact fields

**None.** Zero occurrences of `coach` (case-sensitive) in `samples/roster-oh-mason.html` (the 688 KB roster capture); `contact`/`email` appear only in site chrome. No coach, AD, email or phone field was observed on any captured surface in either wave.
Do not route coach-directory work here (`[inherited]` report 27 reaches the same conclusion, and this lane's captures contain 0 `athletic.net` references and no contact data).

## 13. Public API availability

Three distinct classes, and the important one is the free one:

**(a) Anonymous JSON, robots-allowed — `POST /search/v2/athletes` (VERIFIED 200).**
Template: `POST https://<host>/search/v2/athletes` with form body `searchToken=<from GET /athletes>&q=<name>&perPage=<n>&page=<n>[&filters[subdomain]=<st>]`.
- `oh` host: 200 `application/json`, 25 hits (`samples/search-v2-athletes-oh-2.json`); `page=2` works with the same token (`…-fuller-p2.json`); `perPage=100` honoured (`…-perpage100.json`); `totalPages` = 100 for `q=Fuller` and `q=Smith`.
- `www` host, no subdomain filter: 200, hits carry other states' hosts (`samples/search-v2-athletes-www.json`).
- Precondition, measured both ways: token **and** the `unique_id` cookie from the same `/athletes` page load; empty token + no cookie → 400 HTML (`samples/search-v2-athletes-oh-notoken.json`).
- `robots.txt` does not disallow `/search/**` on any of the seven hosts sampled.

**(b) JSON used by the site's own widgets, robots-DISALLOWED — `/api/**`. Not requested in this wave; reachability unverified by design.** Templates read out of the bundles:

| Endpoint template | Parameters (verbatim option keys) | Bundle |
|---|---|---|
| `GET /api/v1/meets/<MeetResults.meetId>/performances` | `isMeetPro` (0/1), `fields`, `resultsId`, `teamScores` (when `!isTrack`); PRO files are requested as `{resultsId:<file id>, isMeetPro:1}`, non-PRO as `{resultsId:'', isMeetPro:0, excludeMeetPro:true}` | `samples/js-loadresultsnew.js` |
| `GET /api/v1/teams/<teamId>/schedules` | `season`, `year` | `samples/js-ms18-teams-index.js` |
| `GET /api/v1/rosters/teams/<teamId>/ranked` | `formatRank:true`, `gender`, `grade`, `ranking`, `season` | `samples/js-ms18-teams-index.js` |
| `GET /api/v1/teams/<teamId>/rankings` | `season` (default `"all"`) | `samples/js-ms18-teams-rankings.js` |
| `GET /api/v1/teams/<teamId>/best` | `gender` (default `"m"`), `grade` (default `"all"`), `season` | `samples/js-ms18-teams-rankings.js` |
| `GET /api/v1/teams/<teamId>/records` | `season`, `gender`, `eventType` | `samples/js-ms18-teams-rankings.js` |

Base is set by `API.endPoint = '/api/'` (on-host, not a separate API host); `api.js` also carries an unused v3 base `https://api.prod.milesplit.com/int/v3/` that is only selected when `API.useApiV3(true)` is called — no captured page calls it. Every `/api/` call sends `appName`/`appToken`/`userId`/`userToken` headers taken from `Drivefaze.Core` (`samples/js-api.js`).

**(c) No public per-event rankings API exists** — see §18.

## 14. Static file availability

- **Meet result files**: `/meets/<MeetID>/results/<RSID>/raw` is a static fixed-width text payload (wrapped in one `<pre>` inside the site chrome), robots-allowed, no JS needed — `curl` and parse (`samples/raw-oh-770621-rs1321880.txt`).
- **Result-set inventory**: the meet results page embeds `meetResultFiles` JSON in an inline `<script>`, so RSID discovery is one HTML GET per meet (`samples/meet-oh-770621-results.html`).
- **Bundle sources** are static and cache-keyed by `?build=20260921153357`: the capture set references **35 distinct first-party bundle paths** (18 `<script src>` tags on `samples/rankings-oh-boys-xc.html`, 17 on `samples/rank-event-oh-xc-5000m.html`). **16 of the 35 were fetched** — 4 in wave 1, 12 in wave 2 — all HTTP 200; they were chosen because they carry a request template or the grade/premium logic. The remaining 19 are widgets (modal, carousel, segment, skin, ad slots, handlebar templates) with no data path.
- **Sitemaps**: `/<host>/sitemap.xml`, 7,500 athlete URLs each `[inherited]` report 33.

## 15. Browser requirement

**None for the collector surface.** Every recommended surface returns its payload in the server-rendered HTML: team index (977 rows), roster (319 rows), results index (50 rows/page), calendar (294 rows), meet page (JSON-LD + inline `meetResultFiles`), `/raw` (fixed-width text), athlete profile header/PR table, and the search endpoint is plain form-POST JSON.
JS is required only for surfaces this lane recommends **against**: `/meets/<id>/results/<RSID>/formatted` (0 rows in HTML), `/teams/<id>` schedule + "Ranked Performances" widgets (they call `/api/`), `/teams/<id>/rankings` (0 rows in HTML, `samples/team-oh-10002-rankings.html`), `/meets/<id>/entries` (0 rows, "Loading"), and the rankings table (§18).
`[inherited]` report 32 reached the same conclusion with a headless-Chromium control (the rendered DOM equalled the server HTML for the locked profile).

## 16. Request cost

Measured per-surface costs:

| Surface | Requests | Yield |
|---|---|---|
| `/teams` | 1 per jurisdiction | whole HS team frame (e.g. TX 2424, CA 2029, OH 977 teams) |
| `/teams/<id>/roster` | 1 per team | whole roster with grades: 319 rows in the OH sample; `[inherited]` ~24 Class-of-2027 athletes per populated roster |
| `/results` | 1 per 50 meets | meet IDs + results URLs for a season/year |
| `/calendar` | 1 per season | 294 meet rows in the OH XC capture |
| `/meets/<id>/results` | 1 per meet | RSID list + `isMeetPro` |
| `/meets/<id>/results/<RSID>/raw` | 1 per result set | whole result set incl. `Yr` (80 rows here; `[inherited]` up to 5,500) |
| `/search/v2/athletes` | 1 per (query, page) | ≤25 (or ≤100 with `perPage=100`) named hits |
| `/athletes/<id>` | 1 per athlete | identity + PR marks + result *counts only* |

Full Class-of-2027 roster sweep over the measured frame: `1 per state + 1 per team` = **51 + 26,562 = 26,613 requests** for one pass of all 51 jurisdictions (team counts from `samples/teams-count-sweep.tsv`). Midwest only (12 states) = **6,633 requests** (`[inherited]` report 27's figure, reproduced by this lane's counts: 597+592+411+849+895+533+977+700+413+317+153+196 = 6,633).

## 17. Published rate limits

**None published.** `robots.txt` carries no `Crawl-delay` and no rate language (byte-identical 173 B file on all seven hosts, `samples/robots-*.txt`); no rate-limit documentation was observed on any captured page.
Observed behaviour instead: 83 requests in wave 2 (all hosts, ≤1 req/s per host, sequential) → **zero 429, zero 403, zero `Retry-After`, zero CAPTCHA**; `[inherited]` report 32 reported the same for 56 requests + 2 browser loads. Six hosts were swept in a single pass with no challenge.
Working assumption to keep: ≤1 request/second/host, sequential, browser UA.

## 18. Known blocks

**The rankings open question is answered — and the premise was wrong.** The prior phase recorded that "the free rankings table is server-rendered but paywalled past row 1" and left `js-rankings.js` uncaptured as a suspect XHR source.

1. **`js-rankings.js` contains no XHR at all.** The captured bundle (`samples/js-rankings.js`, 7,915 B, HTTP 200) is a filter-form navigator: it reads `#rankingsFilters` selects (`ddCountry`, `ddState`, `ddLevel`, `ddEvent`, `ddSeason`, `ddAccuracy`, `ddYear`, `ddGrade`, `ddAgeGroup`, `ddLeague`), reassembles a URL and assigns `document.location = url`. It performs **zero** `_DF_.API` calls, zero `$.ajax`, zero `fetch`. There is no per-event rankings endpoint to quote because the page has no rankings data path: navigation is a full page load.
2. **The event rankings table is server-rendered and truncated server-side, exactly as documented — measured precisely:** `samples/rank-event-oh-xc-5000m.html` holds 50 `<tr class="rankings-row…">` rows: **1 free row** carrying the real payload (`<tr class="rankings-row">` → text `1 Brice Fuller Fairless JR Wooster XC Invitational-HS Sep 12, 2026 14:54.10`, cells `col-rank / col-name / col-grade / col-meet / col-time`), and **49 locked rows** (`<tr class="rankings-row lk" aria-hidden="true">`) whose entire text content is the rank number: each locked row contains exactly 7 placeholder elements — `<span class="team-logo team-logo--mask">`, `<span class="redact" style="width:168px">`, `<span class="mask" style="width:150px;height:9px">`, `<span class="mask mw-grade">`, `<span class="mask blk" style="width:130px">`, `<span class="mask blk mw-date">`, `<span class="mask mw-time">` — all empty (7 × 49 = 343 placeholders, 0 with text; reproduce: `grep -o 'class="redact\|class="mask\|team-logo--mask' … | wc -l` → 343). The locked names are *absent from the bytes*, not hidden: there is no name, school, meet or time anywhere in those rows, no `aria-label` text at all (0 occurrences) and no unlock logic in any inline script. The page's own page-call says `"subpage_category":"Locked","paywall_present":1` (`"paywall_present":0` on the leaderboard), and the PRO CTA block is `<p>To see these rankings, <span>subscribe to</span></p><figure><strong>MileSplit PRO</strong></figure>` followed by `<a class="join button segment-tracked" … href="/join?ref=rankings-2026-cc-hs-m-5000m&next=…&p=1"><span>Join Now</span></a>` and `<a class="login link" href="/login?ref=…"> Already a PRO member? LOG IN </a>` — i.e. a rendered sentence split across elements, with `drivefaze/pro/paywall.js` loaded. The leaderboard page (`/rankings/leaders/...`, `paywall_present:0`, 8 free rows — one leader per event, grouped under `<tr class="rankings-group-header">`) is the one rankings surface that is fully free.
3. **A team-scoped rankings path exists on a robots-ALLOWED URL** — `/teams/<TeamID>-<slug>/rankings` is not under `/rankings`, so `robots.txt` permits it (`samples/robots-oh-milesplit.txt`) — but its HTML carries **0 rows**; the data arrives only from `/api/v1/teams/<id>/{rankings,best,records}` (`samples/team-oh-10002-rankings.html` + `samples/js-ms18-teams-rankings.js`), i.e. the robots-disallowed `/api/`.
4. **So what a free/anonymous request can and cannot get:** *can* get — rank 1 of any event (and all 8 leaderboard rows), all rosters, all `/raw` result sets, all meet/team/result-index/calendar HTML, and named athlete lookup with IDs via `/search/v2/athletes`. *Cannot* get — ranks 2+ of any event rankings table, any athlete's own result marks (480 mask spans for 96 rows), state ranks on PR rows, and anything on `/api/**`.
5. **Not attempted, by policy:** no login, no PRO entitlement, no cookie replay of a paid session, no `/api/` request, no CAPTCHA work, no `/rankings` re-fetch beyond the two captures already on disk (which are therefore analysis-only), and no re-capture of the empty `rank-event-oh-out-1600m.html`. The unauthenticated reachability of `/api/` is **unverified and will stay unverified**: `Disallow: /api/` on every host sampled makes testing it a robots violation, and the pipeline must not use it.

Other blocks: the athlete result payload is *absent from the bytes*, not CSS-hidden (mask spans are empty; there is nothing to reveal client-side); `/search/v2/athletes` requires a session token + cookie and caps at 100 pages; `sitemap.xml` is capped at 7,500 URLs and carries no grade; `/entries` carries no IDs or grades `[inherited]`.

## 19. Cross-source join keys

- **MileSplit exposes no Athletic.net identifier.** Measured: **0** files in `samples/` contain the string `athletic.net`/`athleticnet`. `[inherited]` report 27 says the same.
- Therefore the join is a **tuple match**, strongest to weakest: `(normalized name, school name, state)` → `[inherited]` report 34: 283/579 = 48.9 % loose match, 300/579 = 51.8 % alias-aware strict match against the delivered Athletic.net Grade-11 boys corpus, with 269 of the 283 unmatched absent from the national corpus by name.
- **MileSplit-side keys that make the join safe:** `AthleteID` (stable, numeric, appears on the roster row and in the search JSON), `TeamID`, `MeetID`, `RSID`, plus grade (`column-grad-year`) and state (`milesplit_site_code`, city-state string). A MileSplit-only census therefore needs no Athletic.net involvement to be internally consistent.
- **Source-side id traps found in this lane:** the same athlete can be reached via two hosts (`www` → state canonical, `samples/athlete-www-13966799.html`); calendar rows link `www` meet URLs while the results index links state-host meet URLs — verified on the **28 `MeetID`s that appear in both OH captures** (`https://www.milesplit.com/meets/715694-…` in `samples/calendar-oh.html` vs `https://oh.milesplit.com/meets/715694-…` in `samples/results-oh.html`) — so join on `MeetID`, never on the URL host. Rosters render `"Last, First"` while profiles and the search API render `"First Last"`.
- Events are named inconsistently across surfaces (`5K` in the leaderboard row with `data-event="5000m"` vs `5000 Meter Run` on the profile) — join events on the `data-event`/`eventCode` token, not the display label.

## 20. Estimated marginal coverage

- **Frame measured this lane:** 26,562 HS team records across all 51 jurisdictions (`samples/teams-count-sweep.tsv`), with **12/12 exact agreement** against the prior independent sweep on the 12 states that CSV covers (`~/Downloads/midwest-tfxc-source-research/data/milesplit-coverage-matrix.csv`, `hs_team_records_on_milesplit`: OH 977, WI 597, MN 592, IA 411, IL 849, MI 895, IN 533, MO 700, KS 413, NE 317, ND 153, SD 196). DC (80) and WY (81) have no prior to compare with — they rest on this lane's captures `samples/teams-dc.html` / `samples/teams-wy.html` and on the sweep row.
- **Class-of-2027 body count** is not measurable from team counts alone; the only per-state numbers on disk are the prior phase's n=3-teams-per-state sample: e.g. OH 977 teams → order-of-magnitude band 9,770–86,953 Class-of-2027 (`est_c2027_total_order_of_magnitude_low/high`, column source above), WI 16,119–39,999, MI 895–895 (the degenerate band caused by 2 of 3 sampled teams being empty). Treat these as bands, not estimates.
- **Marginal value vs Athletic.net:** `[inherited]` report 27/34 — 34.1 % (CI 28.9–39.8 %) of sampled outdoor-competing Class-of-2027 boys are absent from the Athletic.net Grade-11 boys corpus; and where a meet exists on MileSplit, one `/raw` request replaces many Athletic.net profile round-trips.
- **New this lane:** the coach lane gets nothing (0 contact fields), and the national picture is now a *measured frame* rather than an extrapolation — the team index is complete for all 51 jurisdictions, so per-state gating decisions can be made on counts instead of guesses (the Michigan roster-emptiness caveat `[inherited]` still applies to rosters, not to the index).

## 21. Implementation recommendation

**PRIMARY** — for *Class-of-2027 athlete identity, grade and school attribution*, on the whole network (51 jurisdictions + `www`), with **RESULT_SOURCE** (meet `/raw` files) and **DISCOVERY_SOURCE** (`/teams`, `/results`, `/calendar`, `/search/v2/athletes`) as declared secondary roles. Explicitly **not** COACH_SOURCE, and not VALIDATION_SOURCE as a primary role.

Why: the enumeration frame is complete and measured (26,562 teams, 51/51 hosts, one request per jurisdiction), the grade evidence is free and explicit (`column-grad-year`, 95.9 % presence `[inherited]`), the athlete-ID lookup is free JSON, and the results are free and grade-bearing via `/raw`. The one thing the free tier cannot do — an athlete's own cross-meet history — is exactly what Athletic.net supplies, so the two sources are complements rather than competitors.

**Collector rules (do / never):**

- **DO** enumerate `/teams` → `/teams/<id>/roster` for identity + grade; `/results` + `/calendar` → `/meets/<id>/results` → `/meets/<id>/results/<RSID>/raw` for marks; `POST /search/v2/athletes` (with a session token + `unique_id` cookie, ≤100 pages/query) for name → `AthleteID`; `/athletes/<id>` for identity/PR/count validation only.
- **NEVER** request `/api/**` (robots), `/rankings/**` (robots + locked beyond row 1), `/virtual-meets`, `/contact`; never parse mask spans as data (they are empty); never treat ranking position or PR national rank as a grade; never assume the `www`/state host or the URL slug is the identity — join on `MeetID`/`TeamID`/`AthleteID`.
- **Respect** the measured cost model (1 req/s/host, sequential) and the `isMeetPro` flag on result files (skip `1`).

---

### Field-to-evidence index

| Field | Primary evidence |
|---|---|
| source name, hosts | `samples/state-subdomain-probe.txt`, `samples/homepage.html`, `samples/robots-*.txt` |
| geographic coverage | `samples/teams-count-sweep.tsv`, `coverage.json`, `samples/teams-dc.html`, `samples/teams-wy.html`, `samples/teams-oh.html` |
| sports / levels | `samples/results-oh.html`, `samples/calendar-oh.html`, `samples/js-rankings.js`, `samples/athlete-oh-15005189.html` |
| historical depth | `samples/results-oh.html` (years), `samples/athlete-oh-15005189.html` (season blocks), `samples/raw-oh-770621-rs1321880.txt`, `[inherited]` reports 32/33 |
| discovery | `samples/teams-oh.html`, `samples/roster-oh-mason.html`, `samples/results-oh.html`, `samples/calendar-oh.html`, `samples/search-v2-athletes-oh-2.json` |
| identifiers | `samples/roster-oh-mason.html`, `samples/search-v2-athletes-oh-2.json`, `samples/meet-oh-770621-results.html`, `samples/teams-oh.html` |
| pagination | `samples/results-oh.html`, `samples/search-v2-athletes-oh-perpage100.json`, `samples/teams-oh.html` |
| athlete fields | `samples/roster-oh-mason.html`, `samples/athlete-oh-15005189.html`, `samples/search-v2-athletes-oh-2.json` |
| meet fields | `samples/meet-oh-771577.html`, `samples/meet-oh-770621-results.html`, `samples/results-oh.html`, `samples/calendar-oh.html` |
| result fields | `samples/raw-oh-770621-rs1321880.txt`, `samples/js-loadresultsnew.js` (field list), `samples/meet-oh-770621-results.html` |
| grade evidence | `samples/roster-oh-mason.html`, `samples/athlete-oh-15005189.html`, `samples/raw-oh-770621-rs1321880.txt`, `samples/js-ms18-teams-rankings.js` |
| coach/contact | `samples/roster-oh-mason.html` (0 occurrences) |
| public API | `samples/search-v2-athletes-*.json`, `samples/js-api.js`, `samples/js-loadresultsnew.js`, `samples/js-ms18-teams-index.js`, `samples/js-ms18-teams-rankings.js` |
| static files | `samples/raw-oh-770621-rs1321880.txt`, `samples/meet-oh-770621-results.html` |
| browser requirement | `samples/team-oh-10002-rankings.html`, `samples/meet-oh-770621-entries.html`, `samples/meet-oh-770621-results.html` |
| request cost | `samples/teams-count-sweep.tsv`, `samples/roster-oh-mason.html`, `samples/results-oh.html` |
| rate limits | `samples/robots-*.txt`, `samples/fetch-log-wave2.tsv` |
| known blocks | `samples/js-rankings.js`, `samples/rank-event-oh-xc-5000m.html`, `samples/rankings-oh-boys-xc.html`, `samples/athlete-oh-15005189.html`, `samples/robots-*.txt` |
| join keys | `samples/athlete-www-13966799.html`, `samples/calendar-oh.html`, `samples/results-oh.html`, `samples/roster-oh-mason.html`; `[inherited]` report 34 |
| marginal coverage | `samples/teams-count-sweep.tsv` + `~/Downloads/midwest-tfxc-source-research/data/milesplit-coverage-matrix.csv`; `[inherited]` reports 27/34 |
| recommendation | the above |

---

## Open questions (carry into the next wave)

1. **Does the roster page return a full roster outside OH?** This lane measured one OH roster (319 athletes, 100 % grade coverage) and inherited 50 WI/MN rosters (95.9 %); Michigan's roster-emptiness anomaly `[inherited]` report 27 was **not** re-measured here and remains the single biggest risk to a network-wide roster sweep. Cheapest next probe: 3 teams × 5 states (MI, CA, TX, NY, IL) = 15 requests.
2. **Are the per-state team counts time-stable?** All 51 counts are a single 04:00Z snapshot; no second pass exists in this lane, so the drift rate between snapshots is unknown (the prior CSV agrees exactly, which suggests stability rather than proving it).
3. **Is `/api/**` reachable unauthenticated?** Deliberately unverified (robots). If a future decision ever moves it into scope, that decision must be made by a human, because the field list in §10 shows the JSON carries `gradYear` + `windReading` + `athleteId` per row — i.e. the one thing the free tier lacks.
4. **What is the true Class-of-2027 count per jurisdiction?** Only order-of-magnitude bands exist (n=3 teams per state). Resolving it needs the roster sweep, not more indices.
5. **Do `/results` and `/calendar` enumerate every meet?** Both captures are single pages (50 and 294 rows) and both are demonstrably paginated/bucketed at the source; whether a season-complete walk exists (e.g. `?page=N` to the end) was not tested — one `page=2` fetch would confirm the pattern, and `data-month` buckets suggest the calendar is complete for its season range.
6. **Does the search endpoint's `filters[subdomain]` actually narrow server-side?** Only the `oh` and national (`www`) forms were exercised; the near-miss here is that `oh` returns only `oh` hits while `www` returns a mix, which is consistent with filtering but not proof of it (no cross-state query with an explicit `filters[subdomain]=ca` was sent).
