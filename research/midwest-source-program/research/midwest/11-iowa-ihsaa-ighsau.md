# 11. Iowa — IHSAA / IGHSAU (association school universe, Bound platform, results providers, coach directories)

Status: complete
Observed on: 2026-09-19

### Source

| Source | URL(s) | What it is |
|---|---|---|
| IHSAA (boys) | `https://www.iahsaa.org/` | Official boys association (WordPress). Member list, classifications, sport pages, state-meet central, S3-hosted archives. |
| IGHSAU (girls) | `https://ighsau.org/` (canonical; `https://www.ighsau.org/` 301 → bare domain) | Official girls association (WordPress). BEDs, classifications, sport pages, state results; nav contains "Bound Login". |
| Bound ("gobound.com") | `https://www.gobound.com/ia/ihsaa/…`, `https://www.gobound.com/ia/ighsau/…`, `https://www.gobound.com/ia/schools` | The operational platform for both associations: school index, team programs, rosters, stats, staff, scoreboards, meet (comp) results. Server-rendered HTML, no login needed for public pages. |
| IHSADA | `https://ihsada.org/` | Iowa HS Athletic Directors Association. District directors + executive board only; **no per-school AD directory**. |
| IATC | `https://www.iatrackcoaches.org/results/` | Iowa Association of Track Coaches. Weekly XC meet-results index + XC rankings (XC only, not T&F). |
| Wayzata Timing | `https://results.wayzatatiming.com/meets/<id>` | Times Iowa state T&F (2021–2025) and state XC (2024–2025); runs on AthleticLIVE. |
| Other Iowa timers | `dakotatiming.anet.live`, `bwracingservices.anet.live`, `aatiming.com`, `resultscm.com`, `results.blacksquirreltiming.com`, `www.crosscountryratings.com`, `results.truetimeracing.com` | Regular-season XC (and some T&F) result publishers, reached via the IATC weekly index. |

### Coverage

- **Sports**: boys/girls outdoor track & field, boys/girls cross country (both associations). Indoor T&F exists administratively (IGHSAU publishes indoor order-of-events and an IATO procedures manual) but no indoor competition program was found in Bound — [UNVERIFIED] for indoor results.
- **Member universe**: IHSAA member-school list = **379 schools** (`School | Nickname | School Colors | Conference`, 363 rows carry a `/schools/<slug>/` link). IGHSAU BEDs 2026-27 list = **379 schools** with grade 9/10/11 enrollment. Name join between the two lists: 368/379 match exactly after normalization (`Community/High/School` stripped, punctuation removed); the remaining 11 are format variants of the same schools (`ADM` ↔ `ADM, Adel`; `PCM, Monroe` ↔ `PCM`; `Wahlert Catholic` ↔ `Wahlert, Dubuque`; `Okoboji` ↔ `Okoboji, Milford`; `North Cedar` ↔ `North Cedar, Stanwood`; `Exira-EHK` ↔ `Exira Elk Horn-Kimballton`; `Mormon Trail` ↔ `Mormon Trail, Garden Grove`; `The Lighthouse` ↔ `The Lighthouse Schools`) plus 3 single-association entries in the IHSAA list (Holy Trinity Catholic Fort Madison, Mt. Pleasant Christian School, Southeast Christian Academy).
- **Classification (season labels as published)**:
  - IHSAA **FINAL T&F 2026 classifications** (page says "Latest update: March 26, 2026"): 4A = 48, 3A = 64, 2A = 96, 1A = 143 → **351 schools classed** (28 of 379 members unclassed in T&F).
  - IGHSAU T&F final classifications PDF: 4A = 48, 3A = 64, 2A = 96, 1A = 138 → **346 rows**.
  - IGHSAU XC final classifications PDF (2026-27 cycle): 4A = 48, 3A = 64, 2A = 72, 1A = 152 → **336 rows**.
- **Bound Iowa team programs (observed 2026-09-19)**: IHSAA boys T&F 2025-26 = **365** teams; IGHSAU girls T&F 2025-26 = **365**; IHSAA boys XC 2026-27 = **365**; IGHSAU girls XC 2026-27 = **369**.
- **Bound Iowa school index**: **444** entries = ~378 likely 9–12 high schools + 51 middle/junior-high entries + 15 club/college/commission/alternative-program entries (e.g. "Iowa Volleyball AAU", "Wartburg College Knights", "Iowa City Area Sports Commission", "Alternative HS Mason City Riverhawks"). It is *not* a clean member list; reconcile against the association lists.
- **Athlete universe reachable**: 365 IHSAA boys-T&F programs × ~25–30 roster rows each [INFERENCE from two observed rosters] ≈ ~8–11k boys T&F athletes per season with name + grade + school; girls T&F similar; XC comparable but only after meet entries post.
- **Historical depth**: Bound season selectors reach back to ~2009-10 (Harlan boys T&F listed seasons 2009-10 → 2027-28). Association state archives: IGHSAU state T&F 2021–2025 (Wayzata), 2026 (PDF), 2016–2017 (PDFs); IHSAA 2018–2019 state T&F (static HTML on S3).

### Enumeration

**A. Schools — associations (2 requests for the whole state)**
1. `GET https://www.iahsaa.org/member-schools/` → one HTML table, 379 rows; scrape `School, Nickname, Colors, Conference` and the `/schools/<slug>/` hrefs (363 of 379 rows have one; save the slug as the IHSAA school key).
2. `GET https://ighsau.org/ighsau-classifications` → BEDs PDFs: `…/classifications/26-27bedsalpha.pdf` (alphabetical, 379 rows: school + 9/10/11 enrollment) and `…/classifications/26-27bedsnumbers.pdf` (numerical). Text-extractable with any PDF reader.
3. IHSAA class labels: `GET https://www.iahsaa.org/classifications/track-field/` → inline HTML table (TablePress), 351 rows `School | Class | TotalBedsCount | Coops`. Equivalent per-sport pages: `/classifications/cross-country`, `/classifications/<sport>`.
4. IGHSAU class labels: PDFs `https://ighsau.nyc3.digitaloceanspaces.com/t&f-final-classifications-.pdf` and `https://ighsau.nyc3.digitaloceanspaces.com/cc-final-classifications.pdf`.

**B. Schools/teams — Bound (1 request per sport-season index)**
5. `GET https://www.gobound.com/ia/schools` → 444 school links `/ia/schools/<slug>` (school display names include mascot, e.g. "Harlan Community Cyclones Home").
6. `GET https://www.gobound.com/ia/<tenant>/<sport>/<season>/teams` →
   `tenant ∈ {ihsaa, ighsau}`, `sport ∈ {boystrack, girlstrack, boyscrosscountry, girlscrosscountry}`, `season ∈ {2025-26, 2026-27, …}`.
   Parse `div.team-card[data-id][data-classes]` → one card per program: Bound team ID, class-group ID array, display name ("CAL Cadets"), school logo GUID. 365/365/365/369 cards observed for the four combinations above.
7. Team page: `/ia/<tenant>/<sport>/<season>/<school-slug>/<level>` with `level ∈ {v, junior-varsity, freshman, 8th, 7th}`; sub-pages `/roster`, `/roster/<athleteId>`, `/stats`, `/staff`, `/scores`, `/schedule`.

**C. Athletes + Class of 2027**
8. `GET …/<slug>/v/roster` → `Name | Year` table, `Year ∈ {FR, SO, JR, SR}`. **Class of 2027 = `JR`** (results may encode it numerically as `11`).
9. `GET …/<slug>/v/stats` → 24 per-event tables (`Athlete | <EVENT>` with grade suffix), i.e. per-event season leaderboards (observed 1–16 rows per event).

**D. Meets / results**
10. Scoreboard by day: `GET /ia/<tenant>/<sport>/<season>/scores?date=YYYY-MM-DD` (1 request = 1 day, all meets/levels; group-filtered variants `/scores/<groupId>?date=…` also exist).
11. Meet page: `GET /ia/<tenant>/<sport>/<season>/meets/<compId>` → date title, events, divisions.
12. Results: `GET …/meets/<compId>/results` (renders the first event of the first division, exposes the `idMetric=` event list) and `GET …/meets/<compId>/results/all?idMetric=<metricId>` → **full finisher table for that event**.
    - Observed XC: event list had one event (`5k Run`); `results/all?idMetric=…` returned 55 finishers with `#, Name, YEAR School, mark` in a single 78 KB response.
    - Observed track: `/results` page for a co-ed meet listed 15 `idMetric` values for the current division; `results/all?idMetric=…` returned one event's field (37 rows). [INFERENCE] a full co-ed track meet therefore costs ~15 events × 2 divisions ≈ 30 result requests unless the meet's own scoreboard/team stats are reused.
13. State/postseason: IHSAA state T&F live results → `https://results.wayzatatiming.com/meets/73956` (2026) / `/meets/53818` (2025); IHSAA state XC → `/meets/58504` (2025) / `/meets/41636` (2024); IGHSAU 2026 state T&F → `https://ighsau.nyc3.digitaloceanspaces.com/2026-track/results.pdf` (1.2 MB PDF, 200); IHSAA state-qualifying XC meet results are per-meet PDFs under `https://www.iahsaa.org/wp-content/uploads/YYYY/MM/…pdf` (links observed on `/cross-country/state-meet-central/`).

**E. Coach names (not contacts)**
14. `GET …/<slug>/v/staff` → `Position | Name` table for Head Coach / Assistant Coach, 1 request per school+sport+season. Observed: Harlan boys T&F 2025-26 (Head Coach Sam Brummer; assistants Caleb Brouse, James Cairney, Steve Wilwerding) and Harlan girls XC 2026-27 (Head Coach Zach Klaassen; assistant Andrew Sandquist).

### Stable identifiers

| Entity | Identifier | Evidence |
|---|---|---|
| Bound team/program | `h`+32 hex, **season-scoped** — e.g. `h2022217542312fc27f4365ab4ff28f2` (CAL boys T&F 2025-26) vs `h20222175423238599226459041c1852` (CAL boys T&F 2026-27): 0 of 365 IDs shared between the two seasons | `div.team-card[data-id]` on `/ia/ihsaa/boystrack/{2025-26,2026-27}/teams` |
| Bound school | school GUID (UUID) — **stable across seasons** (CAL = `0F209AAD-EBFC-4C89-B023-D35820D4AE97` in both 2025-26 and 2026-27). Present on the 167/365 cards that carry a logo image | team-card `img src=https://d2uxtb165k2tu5.cloudfront.net/schools/<GUID>/logos/thumbnail.png` |
| Bound class / group | `h`+32 hex, **re-issued per season** — girls XC 2026-27 1A = `h202608030426590673bc8dd36b4b64f`; girls T&F 2025-26 4A = `h20260109042734623786e0c60ae7340`; boys T&F 2025-26 exposed 72 group IDs (4 class + district/qualifying groups) | `data-classes` array per team card; sizes 139/96/64/48 (girls T&F) vs official PDF 138/96/64/48 |
| Meet (comp) | `h`+32 hex — Clear Lake girls XC 2026-27 = `h202512040530246586f798314e5e247`; OABCIG boys T&F 2025-26 = `h20250423021853175a8c2f38f56f84a` | `/meets/<compId>` URL + page title carries the date |
| Event/metric | `h`+32 hex in `?idMetric=` — XC 5k Run = `h201996183655cb8a25548fd74d39a05` | results-page links |
| Result group (division) | `h`+32 hex, 2 per co-ed comp observed | `/results/<groupId>?idMetric=…` links |
| Athlete | **none** — roster rows are unlinked `Name \| Year` text | `/…/v/roster`; robots disallows `/profile/*` and `*/athletes/*` |
| IHSAA school | slug in `/schools/<slug>/` (e.g. `ames`, `adm-adel`) | member-schools table hrefs |
| AthleticLIVE meet | integer (e.g. `75990`), tenant-scoped origin | `https://live.athletic.net/meets/75990` → 200 at `https://dakotatiming.anet.live/meets/75990` |
| Season | path segment `YYYY-YY` | team/score URLs |

### Athletic.net leverage

- **Bound exposes zero Athletic.net links**: a text scan of all 41 saved Bound-origin pages (out of 60 saved HTML pages total) found 0 occurrences of `athletic.net`; the only pages in the set that mention it are the two AthleticLIVE shells (290 `livestatic.athletic.net` asset URLs each). Nothing in Bound's HTML references Athletic.net meets, teams, or athletes. Bound is a separate platform. What Bound gives is *identity context* (school + team + athlete name + grade + coach name + classification), which deterministically seeds an Athletic.net lookup without Athletic.net search.
- **The real cross-link is the timing chain.** Iowa regular-season and state meets are published by timing companies on **AthleticLIVE**, which is the same platform family as Athletic.net:
  - meet pages use the canonical host `https://live.athletic.net/meets/<id>` (string present in the AthleticLIVE JS bundle) and serve assets from `https://livestatic.athletic.net/…`;
  - `GET https://live.athletic.net/meets/75990` returned **200 at `https://dakotatiming.anet.live/meets/75990`** — i.e. `live.athletic.net` is a router keyed on meet ID, and the same meet ID resolves across tenants;
  - `https://results.wayzatatiming.com` (Iowa state T&F/XC) is itself in the AthleticLIVE origin map.
  - Not verified: a hard ID join into `www.athletic.net` (my single probe `https://www.athletic.net/meet/75990` returned 404 with a browser-like UA). Meet/athlete identity on AthleticLIVE is therefore an *Athletic.net-family* source, but the exact Athletic.net core URL for a given AthleticLIVE meet ID remains [UNVERIFIED] — hand this to assignments 1–5 (HAR/JS mining), since AI-meet-ID→anet-core-meet-ID mapping is the piece they own.
- **Timing-company tenancy is enumerable for free**: AthleticLIVE's SPA HTML embeds `var imagesByOrigin = {…}` listing **258** tenant origins. Saved at `tools/iowa-11/anetlive-origins.csv`. Iowa-relevant entries: `dakotatiming.anet.live`, `bwracingservices.anet.live`, `aatiming.com`, `results.wayzatatiming.com`; neighbouring states: `live.wiscotrack.com`, `results.windsorrunning.com`, `results.wingfootfinish.com`.
- **Free meet-discovery index**: `https://www.iatrackcoaches.org/results/` lists the current week's Iowa XC meets by date with the timing host link (observed 7 date labels 9/14–9/24/2026 and 9 direct meet links: dakotatiming ×4, aatiming ×3, bwracingservices ×1, blacksquirrel ×1), each of which is an AthleticLIVE meet ID.
- **Requests avoided (Iowa, [INFERENCE] arithmetic)**: Bound replaces Athletic.net ranking enumeration for 365 boys-T&F programs at ≤1 request/team/season for rosters (name+grade) and 1/team for staff; the anet.live route replaces Athletic.net result fetching at ~1–2 requests per XC meet (full field) and ~1 per track event. Net: the state's T&F/XC meet results and athlete identity can be assembled with **no Athletic.net requests at all**, at the cost of ~365/season team-page requests plus meet pages.

### Athlete evidence

- **Name**: yes (roster, stats, staff, results — results inline as `Alexis Mujica, SO Newman Catholic`).
- **Graduating class / grade**: yes — `Year` ∈ {FR, SO, JR, SR} on rosters and in stats tables; results append grade (`SR|JR|SO|FR`) but freshmen were observed encoded as numeric `09` on a 2026 spring-track result page, so accept `FR|SO|JR|SR|9|10|11|12`. **Class of 2027 = JR / 11.**
- **School**: yes (team-page context and in-line in results). **City/state**: not present (school name only); Iowa implies the state.
- **Gender / category / sport**: yes (separate tenant+sport programs: `ihsaa/boystrack`, `ighsau/girlstrack`, `ihsaa/boyscrosscountry`, `ighsau/girlscrosscountry`); adaptive/wheelchair events exist as their own event codes (`100W`, `SPA`, …) on the stats page.
- **TF/XC distinction**: yes (separate programs). **Indoor/outdoor**: not distinguished in Bound data.
- **Performances / PRs / progression**: performances yes (meet results with place and mark; team `stats` = season marks per event). No explicit PR flag and no progression series — reconstruct progression from per-meet results.
- **Meets**: yes (scoreboard + comp pages, both seasons).
- **Athlete profile URL**: **none** publicly (roster rows are plain text; Bound robots disallows `/profile/*` and `*/athletes/*`).
- **Observed counts**: Harlan boys T&F 2025-26 roster = 25 athletes (JR 8, SO 7, FR 8, SR 2); Harlan girls T&F 2025-26 = 30 (JR 10, SO 10, SR 4, FR 6); 2026-27 rosters were empty for both T&F (season not started) and XC (`There are no athletes.`), while XC **results** for 9/17/2026 already carried 55 finishers (51 with grade) for one race.

### Recruiting information

- **Head track coach**: names available (Bound `/staff`) — Harlan boys T&F 2025-26: Head Coach **Sam Brummer**, assistants Caleb Brouse, James Cairney, Steve Wilwerding. No email, no phone, no profile link.
- **Head XC coach**: names available — Harlan girls XC 2026-27: Head Coach **Zach Klaassen**, assistant Andrew Sandquist.
- **Assistant coaches**: yes, names + position labels only.
- **Athletic director**: **no** — not on IHSAA school pages (`/schools/ames/` shows colors, conference, county, school website, "Directory at Bound"; no AD), not on ighsau.org, not on IHSADA (`ihsada.org` = district directors + executive board + open positions only), not on Bound's public school/directory pages (no `mailto:`, no contact table in the rendered HTML; 1 request per school).
- **Public professional email**: **none published** by any Iowa association source checked. Only school-level phone links appear (e.g. Ames school page `tel:+15154322011`).
- **School athletics website**: yes — IHSAA school pages link the district site (Ames → `https://amescsd.org/`). That is the entry point for AD/coach email discovery (assignment 29 territory).
- **Team website**: no.
- Privacy: no athlete contact data exists in these sources; coach names are role-published (position + name). Nothing collected beyond that.

### Result evidence

| Field | Availability in Bound results |
|---|---|
| ResultID | not exposed (rows are text); key by `(compId, metricId, place)` [INFERENCE] |
| AthleteID | not exposed (name + grade + school in-line) |
| MeetID | yes — `h…` comp id in URL, human date in page title |
| EventID | yes — `idMetric=h…` (XC: one `5k Run` metric; track: per-event per-division metrics) |
| Mark | yes (`18:25.00`, `20-10.25`, `11.21`) |
| Normalized mark inputs | no (no raw value/unit/wind fields) |
| Timing method | no field; inferred from host for anet.live meets |
| Wind | not shown on observed pages |
| Implement/hurdle spec | not shown as data (hurdle distance in event name; implement weights only in rules PDFs) |
| Heat/round | yes — `Final` / `Prelim` section headers on track pages |
| Place | yes (`#` column) |
| Date | yes (page title + scoreboard date) |
| School represented | yes (in `Name, YEAR School`) |
| Relay membership | yes — relay event pages list member athletes per team row (4×100/4×200 pages fetched); row-level detail not re-verified this session |

Additional result surfaces: Wayzata/AthleticLIVE state meet pages (2021–2025 T&F; 2024–2025 XC), IGHSAU 2026 state T&F PDF (`…/2026-track/results.pdf`, 1.2 MB), IHSAA state-qualifying XC PDFs under `/wp-content/uploads/2025/10/…`, IHSAA 2018–2019 static HTML (`https://ihsaa-static.s3.amazonaws.com/track/<year>/index.htm`), and `results.truetimeracing.com` for at least one Iowa SQM (Pekin).

### Incremental use

- **New meets**: two cheap paths — (a) `GET /ia/<tenant>/<sport>/<season>/scores?date=YYYY-MM-DD`, 1 request per tenant-sport-day (4 tenant-sport combos in-season ⇒ 28 requests/week if swept daily); (b) `GET https://www.iatrackcoaches.org/results/` once per week → the week's XC meets with their anet.live timing links (9 links observed for one week). Prefer (b) for XC regular season, (a) for association-tracked dates (conference/qualifier/state).
- **Changed meets / new results**: re-fetch `…/meets/<compId>` and its `results/all?idMetric=…` pages. Diff finisher tables on `(name, school, grade)` — no IDs exist, so the natural key is name+school (+grade as a tiebreak).
- **Affected athletes**: the team `stats` page is the cheapest single refresh for season marks (1 request per team, all events); rosters flip per season and can be empty early (observed for fall XC), so treat *results* as the primary incremental signal and rosters as a periodic (weekly/seasonal) reconciliation.
- **Avoid full historical re-fetch**: Bound team IDs and class-group IDs are **re-issued every season** (0 of 365 boys-T&F team IDs are shared between 2025-26 and 2026-27; class groups 2026-27 `h20260803…` vs 2025-26 `h20251011…`), while the **school GUID is stable across seasons**. So key storage on `(schoolGuid | schoolSlug, sport, season)` and treat the team ID as season-local; re-read the `teams` index once per season opening (1 request per tenant-sport) and whenever `data-classes` changes. Class-group ↔ class-label mapping is safe: my join of Bound girls-XC 2026-27 class groups against the IGHSAU official XC class PDF agreed on 300 of 304 matched teams (the 4 mismatches are attributable to my fuzzy name matching, e.g. "Storm Lake St. Mary's" vs "Storm Lake" — [INFERENCE]).
- **Politeness math**: Bound's robots requires `Crawl-Delay: 10`. A full sweep of 365 teams × 2 sports at 10 s spacing is ≈ 2 hours wall-clock; schedule by priority (Class-of-2027-bearing programs first) and reuse the scoreboard diff to touch only teams that competed.

### Access characteristics

- **iahsaa.org** — normal HTML (WordPress); no blocking observed. `robots.txt`: `User-agent: *` with `Disallow: /wp-admin/`, `/search/`, `Disallow: /*?*` (avoid query strings — every recipe above is path-based) and a sitemap (`/sitemap.xml`, 775 URLs; no coach/AD directory pages exist in it).
- **ighsau.org** — normal HTML; `https://www.ighsau.org/` → 301 → `https://ighsau.org/` (200). Static PDFs/images are served from `ighsau.nyc3.digitaloceanspaces.com` (direct 200).
- **gobound.com** — normal HTML, **server-rendered**; no public JSON/XHR endpoint observed in the rendered pages (no `fetch(`, `XMLHttpRequest`, `.json` or `/api/…` references in the school/team/meet pages inspected). A `/api/` tree exists but `robots.txt` disallows it for all agents ⇒ classify as **unavailable** (no probing attempted).
  - Published limits (robots.txt, "Updated: April 21, 2026"): `User-agent: *` → `Crawl-Delay: 10`; `Disallow: /api/`, `/css/`, `/js/`, `/assets/`, `/*directory`, `/*expire`, `/*cache`, `/profile/*`, `*/athletes/*`, `/*/leaderlist*`, `/*/allleaderlist*`, `/*/leaders*`, `/*/calendar*`. Also blocks SemrushBot/AI crawlers (GPTBot, CCBot, anthropic-ai, …).
  - **Disclosure**: I fetched `/ia/schools/<slug>/directory` (2 URLs) before reading robots.txt; that path matches the disallowed `/*directory` rule. Both pages returned no contact data, and the recipe above drops directory paths entirely.
  - No 429 or `Retry-After` observed across ~55 sequential requests (3–10 s spacing). No CAPTCHA, login, or paywall encountered on public pages.
- **iatrackcoaches.org** — normal HTML, single weekly results page; no robots restrictions observed in use.
- **results.wayzatatiming.com / *.anet.live / live.athletic.net** — **browser application** (AthleticLIVE SPA): 50,184-byte HTML shell + `livestatic.athletic.net` JS; results load client-side, so a plain HTTP GET returns the shell, not data (no documented API found; the JS bundle contains only `live.athletic.net/meets/`). `live.athletic.net/meets/<id>` acts as a 301 router to the tenant origin.
- **www.athletic.net** — `GET /meet/75990` with a browser-like UA from this machine returned **404** (not 403), i.e. no block was triggered for that path this session. The brief's 2026-09-19 observation that Athletic.net returns Cloudflare 403 to non-browser clients stands as the platform-level constraint; the 404 is recorded as the observation actually made here.
- Source classes present: normal HTML (associations, Bound), downloadable PDF (classifications, state results, memos), static HTML archive (S3), browser application (AthleticLIVE), authenticated (Bound admin/coach login — not used).

### Recommendation

- **PRIMARY** — Bound Iowa tenants (`/ia/ihsaa`, `/ia/ighsau`) for Iowa school/team/athlete/grade/coach-name data: 365–369 team programs per sport-season with season-scoped team IDs, a stable school GUID/slug key, rosters carrying grade, per-event season stats, staff names, and full meet results. Pair it with the two association member/classification sources for class labels and the member universe. Expected marginal coverage: ~365 of 379 IHSAA members' boys T&F programs, 369 IGHSAU girls XC programs, all without touching Athletic.net.
- **RESULT-SOURCE** — AthleticLIVE tenants (Wayzata for state T&F/XC; Dakota Timing, BW Racing, AA Timing, ResultsCM for regular season), entered via the IATC weekly results index. Full meet fields per event in ~1–2 requests/meet; also the honest route to Athletic.net-family meet IDs.
- **DISCOVERY-ONLY** — association state-meet pages/PDFs, IATC weekly index, DO Spaces/S3 archives (enumerate meets/results; not athlete rosters).
- **REJECT** for coach contacts — no Iowa association source publishes AD or coach emails; only coach *names* (Bound `/staff`). Email discovery must go through school-district websites (hand to assignment 29's coach graph), using Bound's coach names as the search key.
- **Biggest gaps**: (1) 24 IHSAA members have no obvious Bound boys-T&F team (e.g. Coram Deo Academy, Des Moines Prep, Great River Christian, H-L-V Victor, Kingsley-Pierson, Ruthven-Ayrshire, Regina Iowa City, Scattergood Friends) — [INFERENCE] mostly small/Christian/co-op programs; (2) XC athlete lists are result-driven only (rosters empty in-season); (3) no athlete IDs anywhere, so reconciliation is name+school+grade; (4) full co-ed track meet extraction is ~30 requests meet⁻¹ unless team stats are used instead; (5) AthleticLIVE→Athletic.net core meet-ID mapping is unverified on this machine.

### Evidence appendix

All requests were `GET`, sequential, browser-like UA, from this machine; timestamps are America/Chicago on 2026-09-19 (file mtime of the saved response where the request was not printed).

| URL | Method | HTTP status | What it proved | Timestamp |
|---|---|---|---|---|
| `https://www.ighsau.org/` | GET | 301 → `https://ighsau.org/` | IGHSAU canonical host is the bare domain | 23:12:47 |
| `https://ighsau.org/` | GET | 200 | IGHSAU home; nav "Bound Login"; association site is WP | 23:12:50 |
| `https://www.iahsaa.org/` | GET | 200 | IHSAA home; WordPress; canonical confirmed by tag | 23:12:47 |
| `https://www.iahsaa.org/member-schools/` | GET | 200 | 379-row member table (School/Nickname/Colors/Conference), 363 `/schools/<slug>/` links | 23:12:57 |
| `https://www.gobound.com/ia` | GET | 200 | Bound Iowa landing; tenant structure | 23:12:58 |
| `https://www.gobound.com/ia/ighsau` | GET | 200 | IGHSAU tenant exists in Bound | 23:12:58 |
| `https://www.gobound.com/ia/schools` | GET | 200 (638 KB) | 444 Iowa school entries with `/ia/schools/<slug>` links | 23:13:09 / 23:23:55 |
| `https://www.iahsaa.org/track-field/` | GET | 200 | IHSAA T&F hub (state meet, classifications, tournament central) | 23:13:38 |
| `https://www.iahsaa.org/cross-country/` | GET | 200 | IHSAA XC hub | 23:13:38 |
| `https://www.iahsaa.org/classifications/track-field/` | GET | 200 | "FINAL TRACK & FIELD 2026 CLASSIFICATIONS" table: 351 rows, 4A 48/3A 64/2A 96/1A 143; class rules text | 23:13:40 / 23:23:40 |
| `https://ighsau.org/ighsau-classifications` | GET | 200 | BEDs PDFs (`26-27bedsalpha.pdf`, `26-27bedsnumbers.pdf`) | 23:13:40 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/scores` | GET | 200 | Daily scoreboard structure (date navigation) | 23:13:52 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/scores?date=2026-05-23` | GET | 200 | Date-scoped scoreboard lists meets/levels for one day | 23:13:59 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/scores/<groupId>?date=…` (4A group) | GET | 200 | Group-filtered scoreboard variant exists | 23:14:21 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/scores?date=2026-04-24` | GET | 200 | Meet discovery for a regular-season date | 23:14:28 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/meets/h20250423021853175a8c2f38f56f84a` | GET | 200 | Comp (meet) page: date in title, events, divisions | 23:14:32 |
| `…/meets/h20250423021853175a8c2f38f56f84a/results` | GET | 200 (66 KB) | Results root renders first event of a division; 15 `idMetric` links; 2 result-group IDs | 23:20:57 |
| `…/meets/<compId>/results/<groupId>?idMetric=<metric>` (100 m male) | GET | 200 | One event's rows: `#, Name, YEAR School, mark` | 23:14:38 |
| `…/meets/<compId>/results/<groupId>?idMetric=<metric>` (4×100 male) | GET | 200 | Relay rows present in same table shape | 23:14:46 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/harlan/v` | GET | 200 | Team page shell (level selector, seasons, links) | 23:14:46 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/harlan/v/staff` | GET | 200 | Coaching staff names+positions (Head: Sam Brummer; 3 assistants) | 23:14:56 / 23:24:06 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/harlan/v/roster` | GET | 200 | `Name \| Year` roster: 25 athletes (8 JR) | 23:14:56 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/teams` | GET | 200 (781 KB) | 365 `div.team-card` entries with team IDs + class-group arrays | 23:15:02 / 23:23:59 |
| `https://www.gobound.com/ia/ighsau/girlstrack/2025-26/teams` | GET | 200 (783 KB) | 365 IGHSAU girls T&F teams, class groups 139/96/64/48 | 23:15:20 |
| `https://www.gobound.com/ia/schools/harlan` | GET | 200 | School page lists boys/girls activities incl. Cross Country + T&F | 23:15:27 |
| `https://www.gobound.com/ia/schools/harlan/directory` | GET | 200 | Directory page renders **no** contacts (and is robots-disallowed — see disclosure) | 23:15:28 |
| `https://www.gobound.com/ia/schools/ames/directory` | GET | 200 | Same: no AD/coach contacts, no `mailto:` | 23:16:58 / 23:24:10 |
| `https://www.gobound.com/ia/ighsau/girlscrosscountry/2026-27/teams` | GET | 200 (757 KB) | 369 girls XC teams; class groups 160/72/64/48 | 23:15:56 / 23:24:02 |
| `https://www.gobound.com/ia/ighsau/girlscrosscountry/2026-27/scores?date=2026-09-18` | GET | 200 | Current-season XC scoreboard | 23:15:58 |
| `https://www.gobound.com/ia/ighsau/girlscrosscountry/2026-27/scores?date=2026-09-17` | GET | 200 | Live XC meet list for a specific day | 23:16:01 |
| `https://www.gobound.com/ia/ihsaa/boyscrosscountry/2026-27/scores?date=2026-09-17` | GET | 200 | Same mechanism for boys XC | 23:16:01 |
| `https://www.gobound.com/ia/ighsau/girlscrosscountry/2026-27/meets/h202512040530246586f798314e5e247` | GET | 200 | Comp page (Clear Lake, 9/17/2026): single `5k Run` event | 23:16:06 |
| `…/meets/h202512040530246586f798314e5e247/results` | GET | 200 (40 KB) | Event list + `/results/all?idMetric=h201996183655cb8a25548fd74d39a05` link; no finishers in this view | 23:16:09 |
| `…/meets/h202512040530246586f798314e5e247/results/all` (no metric) | GET | 200 | Renders same event-list view (no finishers) | 23:16:19 |
| `…/meets/h202512040530246586f798314e5e247/results/all?idMetric=h201996183655cb8a25548fd74d39a05` | GET | 200 (78 KB) | **55-finisher XC table** with place, `Name, YEAR School`, time | 23:25:10 |
| `https://www.gobound.com/ia/ighsau/girlscrosscountry/2026-27/harlan/v/staff` | GET | 200 | XC staff: Head Coach Zach Klaassen + assistant | 23:18:07 |
| `https://www.gobound.com/ia/ighsau/girlscrosscountry/2026-27/harlan/v/roster` | GET | 200 | "There are no athletes." → XC rosters empty in-season | 23:16:29 |
| `https://www.gobound.com/ia/ighsau/girlscrosscountry/2025-26/teams` | GET | 200 | Prior-season XC team index | 23:16:29 |
| `https://www.gobound.com/ia/ighsau/girlscrosscountry/2025-26/scores?date=2025-11-01` | GET | 200 | State XC day scoreboard → state meet comp page ("4A Girls Race", 11/1/2025) | 23:16:33 |
| `https://www.gobound.com/ia/ihsaa/boyscrosscountry/2026-27/scores?date=2026-09-17` (Sergeant Bluff) | GET | 200 | Boys XC meet comp page for the same day | 23:16:39 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/scores?date=2026-05-21` | GET | 200 | State T&F day scoreboard | 23:16:39 |
| `…/meets/<state compId>/results` (2026 state, class 4A girls events) | GET | 200 (51-row table) | State results rows: `Katie Willits, SR Waukee Northwest 11.66` etc. | 23:19:32 |
| `https://www.iahsaa.org/schools/ames/` | GET | 200 | School page: colors, conference, county, district website, "Directory at Bound"; **no AD/coach data**; `tel:+15154322011` | 23:16:50 / 23:23:34 |
| `https://www.iahsaa.org/track-field/state-meet-central/` | GET | 200 | Live results links → `results.wayzatatiming.com/meets/73956` (2026) and `/meets/53818` (2025) | 23:17:27 |
| `https://www.iahsaa.org/cross-country/state-meet-central/` | GET | 200 | State XC results (Wayzata `/meets/58504`, `/meets/41636`) + per-meet SQM result PDFs under `/wp-content/uploads/2025/10/…` + link to IGHSAU XC central | 23:17:29 |
| `https://ighsau.org/sports/track-field/` | GET | 200 | 2026 state T&F results PDF (DO Spaces), Bound-upload instructions link, IATO manual | 23:17:36 |
| `https://ighsau.org/sports/cross-country/` | GET | 200 | IATC results/rankings links, XC manual, `cc-final-classifications.pdf`, SQM sites | 23:17:37 |
| `https://ighsau.org/state-track-field-meet-archived-results` | GET | 200 | State T&F archive: 2021–2025 Wayzata meet IDs, 2016–2017 PDFs, 2018–2019 IHSAA S3 HTML | 23:19:59 |
| `https://www.gobound.com/ia/ighsau/girlstrack/2025-26/harlan/v/roster` | GET | 200 | Girls T&F roster: 30 athletes (10 JR) | 23:19:20 |
| `https://www.gobound.com/ia/ighsau/girlstrack/2026-27/harlan/v/roster` | GET | 200 | 2026-27 roster empty (season not started) | 23:19:42 |
| `https://www.ighsau.org/…` → `https://ighsau.nyc3.digitaloceanspaces.com/classifications/26-27bedsalpha.pdf` | GET | 200 | 379-school BEDs list with 9/10/11 enrollment (parsed → `ighsau-members-beds-2026-27.csv`) | 23:19:07 |
| `https://ighsau.nyc3.digitaloceanspaces.com/t&f-final-classifications-.pdf` | GET | 200 | IGHSAU T&F classes: 48/64/96/138 = 346 rows | 23:20:16 |
| `https://ighsau.nyc3.digitaloceanspaces.com/cc-final-classifications.pdf` | GET | 200 | IGHSAU XC classes: 48/64/72/152 = 336 rows | 23:20:23 |
| `https://www.iatrackcoaches.org/results/` | GET | 200 | Weekly XC index: 7 dates (9/14–9/24/2026) and 9 timing-host meet links | 23:19:59 / 23:23:49 |
| `https://results.wayzatatiming.com/meets/73956` | GET | 200 (50,184 B) | State T&F 2026 live results = AthleticLIVE SPA (same shell size/content-type as anet.live) | 23:19:12 / 23:23:45 |
| `https://dakotatiming.anet.live/meets/75990` | GET | 200 (50,184 B) | AthleticLIVE tenant meet page; **258-tenant origin map** extracted from the shell | 23:20:05 / 23:23:52 |
| `https://livestatic.athletic.net/main-5ADGVJIV.js` | GET | 200 (295 KB) | AthleticLIVE JS bundle contains `https://live.athletic.net/meets/` (canonical host); no REST API path exposed | 23:22 |
| `https://www.gobound.com/robots.txt` | GET | 200 (2,163 B) | `Crawl-Delay: 10`; disallows `/api/`, `/*directory`, `/profile/*`, `*/athletes/*`, `/*/leaders*`, … | 23:25:34 |
| `https://ighsau.nyc3.digitaloceanspaces.com/2026-track/results.pdf` | GET | 200, 1.22 MB, `application/pdf` | 2026 IGHSAU state T&F results are a directly downloadable PDF | 23:25:38 |
| `https://www.athletic.net/meet/75990` | GET | **404** | AthleticLIVE meet 75990 does **not** map to `www.athletic.net/meet/75990`; also no 403 for this UA/path | 23:25:43 |
| `https://live.athletic.net/meets/75990` | GET | 200 (301 → `https://dakotatiming.anet.live/meets/75990`) | `live.athletic.net` is a meet-ID router into the tenant origins | 23:25:47 |
| `https://www.iahsaa.org/robots.txt` | GET | 200 (146 B) | `Disallow: /*?*`, `/search/`; sitemap declared | 23:25:51 |
| `https://www.iahsaa.org/sitemap.xml` → `post-sitemap1.xml`, `page-sitemap1.xml` | GET | 200 ×3 | 775 site URLs; **no** coach/AD directory pages exist | 23:21 |
| `https://ihsada.org/` | GET | 200 | IHSADA home: district directors/executive board/open positions only — no per-school AD directory | 23:20:33 |
| `https://www.iahsaa.org/resources/directory/` | GET | 404 | No IHSAA directory page at the guessed path (sitemap confirms none) | 23:20:50 |
| `https://www.gobound.com/ia/ihsaa/boystrack/2026-27/teams` | GET | 200 (698 KB) | 365 teams for 2026-27 with `data-classes="[]"` (classes not yet applied) | 23:18:17 |
| `https://www.gobound.com/ia/ihsaa/boyscrosscountry/2026-27/teams` | GET | 200 (741 KB) | 365 boys XC teams; 32 class-group IDs | 23:17:57 |

URLs observed as links but **not** requested (source page in parentheses) — listed so Main can fetch them deliberately:
`https://ighsau.nyc3.digitaloceanspaces.com/classifications/26-27bedsnumbers.pdf` (IGHSAU classifications); `.../2026-iato-procedures-manual-schools-edition-(5).pdf`, `.../2026-tf-sqm-assignments-4.21.26.pdf`, `.../2026-tf-sqm-sites-3.25.26.pdf`, `.../26-schedule_1st-draft-2.pdf`, `.../26-sqm-sites.pdf`, `.../2026-xc-memo-2-8-31-26.pdf`, `.../postseason-track-manual.pdf`, `.../trackandfield2025/indoor-order-of-events-1744215003.pdf` (IGHSAU sport pages); `https://www.iahsaa.org/wp-content/uploads/2025/10/<per-SQM>-results.pdf` (IHSAA XC state-meet central); `https://results.truetimeracing.com/results.aspx?CId=16535&RId=1611&EId=3&dt=1&top=10` (IHSAA XC SQM result page); `https://ihsaa-static.s3.amazonaws.com/track/2018/index.htm`, `.../track/2019/index.htm` (IGHSAU/IHSAA archives); `https://ighsau.org/2025-cross-country-tournament-central` (IHSAA XC central).

Supporting artifacts (in-repo, read-only inputs): `tools/iowa-11/*.csv` — Bound Iowa school index (444), Bound team IDs for IHSAA boys T&F 2025-26 (365) / IGHSAU girls T&F 2025-26 (365) / boys XC 2026-27 (365) / girls XC 2026-27 (369), IHSAA members (379) + T&F classes (351), IGHSAU BEDs (379) + T&F classes (346) + XC classes (336), AthleticLIVE tenant origins (258). Raw pages and the parse scratch live in `tools/scratch-11/`.
