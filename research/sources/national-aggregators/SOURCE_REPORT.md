# National aggregators beyond Athletic.net / MileSplit — SOURCE_REPORT

Lane: `research/sources/national-aggregators` (agent `ResNationalAggregators`).
Capture window: **2026-09-22T03:55Z – 2026-09-22T04:10Z** (UTC), plus package data files dated 2026-09-21.
Access discipline: anonymous `curl` GETs, **≤1 request/second per host** (all batches `sleep 1.2`–`1.3` between same-host calls), no login, no auth bypass, no paywall. Browser (managed Chromium via CDP) used **only** where curl is blocked by a challenge; those pages are marked `[browser]`.
Compliance: `robots.txt` fetched and stored for every host before/with content fetches (see §Compliance). Where a host publishes no policy, the absence is recorded with the raw response.

## Scope and exclusions

In lane: DirectAthletics + TFRRS (+ high-school TFRRS instances), AthleticLIVE (tenants/ES/blob/RTDB), RunnerSpace/DyeStat (+ ResultsCentral), MaxPreps, World Athletics, USATF (+AAU), and other multi-state athletic databases qualified in the window (Cross Country Ratings; screened: NCSA, FieldLevel, Baumspage, TrackScoreboard/FinishedResults).

Out of lane (owned by sibling agents, referenced here only for join keys): Athletic.net (`ResAthleticNet`), MileSplit (`ResMilesplit`), coach directories (`ResCoachDirectories`), timing providers (`ResTimingProviders`), state associations (`ResState*`).

## Provider ranking (marginal coverage / cost / effort)

| rank | provider | expected marginal Class-of-2027 athletes + performances | request cost (measured) | effort | recommendation |
|---|---|---|---|---|---|
| 1 | **AthleticLIVE** (ES + Azure blob) | Highest in lane: 18-state meet-doc counts measured (IL 14,599 · CA 12,703 · MI 10,775 · NY 9,664 · MN 7,502 · IA 7,169 · PA 5,937 · ND 5,222 · OH 4,542 · NE 4,534 · WI 3,902 · TX 3,454 · SD 3,188 · MO 2,517 · IN 2,233 · KS 1,698 · FL 425 · GA 206; all-time, `samples/athleticlive/al-es-msearch-state-counts.json`). Result rows carry **grade + Athletic.net athlete/team ids** (`samples/athleticlive/al-blob-ind-res-2254285.json`), so performances arrive pre-joined to the corpus we already hold. Athlete-level count not yet measured — see Open Q1 | 1 sitemap + 1 `_msearch` for all **256** tenant indices (44,948 B) + 1 ES query per state + 1 blob doc per (tenant, meet, event); 256 tenants are enumerated in the package CSV | MED (undocumented single-letter-key wire format; blob path templates must be reverse-engineered) | **PRIMARY** |
| 2 | **DirectAthletics (DAT)** | Discovery-grade: 946 upcoming meets in **one 340 KB JS file** across 49 venue states (`dat-fusedriver-upcoming.js`); results index renders 100 meets/page with links to either DAT or TFRRS (`dat-results-index-oh-track.html`: 100 `class="meetLink"` rows); HS event sheets carry a `Year` (grade) column (`dat-hs-event-sheet-74867_4545019.html`). DAT-hosted **HS** results are sparse relative to college (prior lane measurement: `data/dat-provider-coverage.csv`, `hs-marked`=0 for WI/MN samples) | LOW per request (100 meets per results-index page; one JS file = 946 meets); HIGH total page count for full state coverage (OH league page alone lists 1,749 team ids) | MED | **DISCOVERY_SOURCE** (DAT-hosted HS result pages: **CONDITIONAL**) |
| 3 | **TFRRS** | Performance lists with a **grade filter/column** (SR/JR/SO/FR/8/7/6) and per-athlete class year; IN "HSR All School Performance List (2010)" contains **1,671 distinct athletes** in one 1.88 MB page (`indiana-tfrrs-list-5489.html`). High-school coverage is only 3 states (FL, IN, NH — `tfrrs-sites.html`), so marginal Co2027 = those states' HS + all college athletes (college = out of census scope) | LOW: 1 request per list page (1.9 MB); 1 request per athlete page; `results_search_page.html` lists meets by state+sport+year | LOW-MED | **RESULT_SOURCE** (HS instances) / **VALIDATION_SOURCE** (national college) |
| 4 | **RunnerSpace / DyeStat / ResultsCentral** | Grade codes (`SR/JR/SO/FR`) are printed in NXN championship result tables (`rs-nxn-2025-results-title187.html`); ResultsCentral is a weekly **meet-name → live-results link** hub (153 news items in 2026) rather than a results database. Cloudflare blocks curl (403) — browser required, `Crawl-delay: 10` | 1 request per title/results page; meet universe is small (NXN/NXR regionals, national-class meets) | LOW (browser mandatory) | **DISCOVERY_SOURCE** (ResultsCentral) / **CONDITIONAL** (meet results) |
| 5 | **USATF (Junior Olympics)** | Not a results host for us: JO pages link live results to `finishedresults.trackscoreboard.com/meets/14258/events` and registration to **athletic.net meet 644030** — i.e. it *confirms* Athletic.net meet ids for national JO meets. 101 junior-olympic URLs in the sitemap | 1 request per page; 3,890 URLs in sitemap | LOW | **VALIDATION_SOURCE** |
| 6 | **World Athletics** | Elite/international only: `searchAthletes` is competition-scoped, `getAthlete(urlSlug)`, `AthleteNewData` carries `birthDate`/`countryCode`; US HS athletes appear only when they have an international-class result | LOW: GraphQL POSTs with a public `x-api-key`; introspection enabled (15 KB captured) | LOW | **VALIDATION_SOURCE** |
| 7 | **Cross Country Ratings** | 9-state XC-only database (IA, NE, MN, ND, SD, IL, WI, MO, KS) with **`Class of 20XX` + grade word** on runner pages; 240 runner links / 101 meet links on `/individuals` / `/results`. Small universe, robots-allowed with `Crawl-delay: 1` | 1 request per runner/meet page | LOW | **CONDITIONAL** (validation/spot-check) |
| 8 | **MaxPreps** | No grade, no TF performance data on the paths that are crawlable: `robots.txt` disallows `/school/`, `/team/`, `/discovery/`, `/careerprofile/`, `/m/team/`, `/m/school/` (191 Disallow rules); team roster pages expose no athlete ids/grades; athlete pages expose `height`/`position`/`weight` keys but no graduating class in the HTML | n/a | n/a | **REJECT** |
| 9 | **AAU** | `www.aausports.org/sports/track-field` → 403 Cloudflare challenge ("Attention Required! | Cloudflare", `aau-sports-track.html`) to curl; `aautrackandfield.org` → 301 → `aausports.org/track-and-field` → 403 | n/a | n/a | **REJECT** (blocked) |
| 10 | **Screened, not qualified**: NCSA (robots open, 2.7 KB policy; recruiting paywall), FieldLevel (robots open, 110 B; recruiting profiles), Baumspage (`/robots.txt` 404; OH-centric results host) | — | — | — | **REJECT** (out of class) for NCSA/FieldLevel; Baumspage belongs to the timing-provider lane |

---

## 1. DirectAthletics (DAT)

1. **Source name** — DirectAthletics (`www.directathletics.com`), Ruby app ("filterrific" + Turbo frames); also the owner/operator of TFRRS (§2) and the operator of the `live.*`-style meet registration used by college conferences and some HS associations. Page title on the league page is literally `DirectAthletics` (`samples/directathletics-tfrrs/dat-league-track-43-oh.html`).
2. **Geographic coverage** — multi-state: league pages are per state (`/leagues/track/43.html` = Ohio; Wisconsin = 115 per `data/dat-team-index.csv`). Global upcoming-meet index spans **49 `venue_state` values** incl. PR/VI (`samples/directathletics-tfrrs/dat-fusedriver-upcoming.js`: FL 357, PA 50, NY 44, NH 37, NC 36, VA 34, IN 32, OH 31 …).
3. **Sports** — `track` + `xc` only (`sports: ['track','xc']` in the same file).
4. **Historical depth** — meet dates in the captured HS event sheet go back to at least 2022-05-05 (`dat-hs-event-sheet-74867_4545019.html`); the results index is filtered by `with_date_from`/`with_date_to` (form controls in `dat-results-index-oh-track.html`, `max="2026-09-22"`); the upcoming index covers `2026-09-21 → 2027-10-01`.
5. **Discovery mechanism** — four independent ones, all verified: (a) **state league → team list** `/leagues/track/<league_id>.html` (1,749 distinct `/teams/track/<id>` links on league 43/OH); (b) **results index** `/legacy_da/results?filterrific[with_states]=OH&filterrific[with_sports]=track` — server-rendered 100 rows/page (`100` × `class="meetLink"`, `102` × `class="col-date"`), paginated `page=2`, `page=3`; (c) **global upcoming-meet JSON-in-JS** `/scripts/fuseDriver_Upcoming.js` (946 objects: `venue_state`, `name`, `date_begin`, `meet_hnd`, `sport`, `hs|college|jhs|jcollege|club`, `reg_status`); (d) **performance-list directory** `/rankings.html` (274 distinct `/lists/track/<a>_<b>` ids in the captured page, 1.95 MB).
6. **Stable identifiers** — `meet_hnd` (string, e.g. `"27326"`, `"27325"`, `"28499"`); meet page `/results/track/<meet_id>.html` (e.g. `96459`); event sheet `/results/track/<meet_id>_<sheet_id>.html` (e.g. `74867_4545019`); team `/teams/track/<team_id>` (e.g. `42070`); league `/leagues/track/<league_id>.html`; list `/lists/track/<league>_<list>.html` (e.g. `1428_5490`) with `?year=JR|SR|SO|FR` filter.
7. **Pagination** — results index: `page=N`, 100 meets/page; performance lists: `limit=100` (used in the captured `lists/track/1428_5490.html?year=JR&limit=100`).
8. **Athlete fields** — on HS event sheets: `Name` (`Woody, Isabelle`), `Year` (grade: SR/JR/SO/FR), `Team` (`Iowa City-Liberty High`), plus per-event performance and `Score`. No athlete id is exposed on the sheet.
9. **Meet fields** — `Meet name` (`MVC Girls Divisional Meet`), `Date` (`May 5, 2022`), `Venue` (`Western Dubuque - Epworth, IA`); in the results index rows: date, meet name, sport (`TF`), state (`OH`); upcoming index: name, date_begin, venue_state, sport, registration status/message.
10. **Result fields** — event name (`Girls High Jump Varsity`), round (`Finals:`), `Place`, `Overall`, mark (`5' 2"`), `Score`, team.
11. **Grade/class evidence** — **yes**, explicit `Year` column with SR/JR/SO/FR values on DAT HS result sheets (`dat-hs-event-sheet-74867_4545019.html`; token counts in the raw page: FR×3, SO×3, JR×3, SR×2 in the captured event's rows); the performance-list URL supports `year=JR` as a first-class filter.
12. **Coach/contact fields** — none observed on any captured page (no coach/AD/email field in the league, results-index, list or event-sheet captures). `[INFERENCE]` DAT stores coach contacts for registration but they are login-gated (not verified — no login attempted).
13. **Public API availability** — no public API. One static data file acts as an index: `/scripts/fuseDriver_Upcoming.js` (`application/javascript`, 340,501 B).
14. **Static file availability** — yes, the above JS index; all meet/team/list pages are server-rendered HTML (no JS needed to read rows: the results-index rows are plain `<tr>` with `<a class="meetLink">`).
15. **Browser requirement** — **no** for the captured surfaces (league, results index, event sheet, list, JS index all fetched with curl at HTTP 200, no challenge). Turbo frames are progressive enhancement only.
16. **Request cost** — league list page: 1 request per state×sport (then 1 per team if team pages are needed); results index: 1 request per state×sport per page (100 meets/page); JS index: **1 request for 946 meets**; event sheet: 1 request per event; performance list: 1 request per list×grade filter.
17. **Published rate limits** — none published. `/robots.txt` returns **HTTP 200 `text/html` 29,639 B** (the site's ASCII-art brand/404 page, `samples/directathletics-tfrrs/robots-www.directathletics.com_robots.txt`) → **no machine-readable robots policy exists**. We therefore self-limit to ≤1 req/s.
18. **Known blocks** — none observed for HTML pages (all captures 200). `[INFERENCE]` None of the captures required cookies; registration/login areas were not probed.
19. **Cross-source join keys** — **direct**: DAT's results index rows link to `tfrrs.org/results/<id>/<slug>/` for TFRRS-hosted meets and to DAT's own `/results/track/<id>.html` for DAT-hosted meets (`dat-results-index-oh-track.html` contains both link families — e.g. `tfrrs.org/results/92703/6th_Annual_Mike_Becraft_Invitational/` and `directathletics.com/results/track/96459.html`), so one crawl of DAT gives the TFRRS meet ids for the same meet. To Athletic.net: **no direct key observed** (no `athletic.net` link or id anywhere in the captured pages).
20. **Estimated marginal coverage** — discovery-level: 946 upcoming meets in one file (national, 49 states) + per-state results indexes (100 meets/page). For the 12-state Midwest, prior lane work already indexed **22,527 DAT team records** (`data/dat-team-index.csv`: OH 3,428 · IL 3,340 · MI 3,230 · MO 1,919 · WI 1,912 · IN 1,816 · KS 1,669 · IA 1,592 · NE 1,266 · MN 874 · SD 758 · ND 723). Marginal *Class-of-2027 performances* are unmeasured: `[INFERENCE]` low-to-moderate, because DAT's HS result volume is thin relative to Athletic.net/MileSplit (prior measurement `data/dat-provider-coverage.csv` shows `hs-marked` = 0 for the WI/MN sample and "index page only (95 rows, 0 HS)" for WI).
21. **Recommendation** — **DISCOVERY_SOURCE** (meet/team/list discovery + TFRRS id mapping). Use `CONDITIONAL` for DAT-hosted HS result harvesting pending a per-state HS-result-density measurement.

## 2. TFRRS (Track & Field Results Reporting System)

1. **Source name** — TFRRS (`www.tfrrs.org`), a DirectAthletics product ("Copyright © 2026 DirectAthletics, Inc." — `samples/directathletics-tfrrs/tfrrs-sites.html`).
2. **Geographic coverage** — national (college) + **three high-school association instances**: `florida.tfrrs.org` ("Florida High School Track & Field Rankings and Meet Results"), `indiana.tfrrs.org`, `nh.tfrrs.org` ("NHIAA Track & Field Rankings and Meet Results"). Probed-and-refused: `texas.tfrrs.org`, `ohio.tfrrs.org`, `california.tfrrs.org` → DNS NXDOMAIN (curl exit 6, recorded in `samples/directathletics-tfrrs/CAPTURES.log`).
3. **Sports** — indoor T&F, outdoor T&F, cross country (`xc.tfrrs.org` facility subdomain; nav: TEAMS / CONFERENCES / MEET RESULTS / PERFORMANCE LISTS).
4. **Historical depth** — performance lists are season-scoped and archived (IN list "All School Performance List (**2010**)", `indiana-tfrrs-list-5489.html`); indoor/outdoor archives exposed in nav; NH home page lists current-week XC meets (Sep 19 2026 Manchester Invitational etc.).
5. **Discovery mechanism** — `/sites.html` (association list), `MEET RESULTS` + `results_search_page.html?with_states=IN&with_sports=xc&with_year=2026` (state+sport+year meet search, server-rendered), team/conference pages, performance lists `/lists/<list_id>/<slug>/<year>/<season>` (e.g. `/lists/5489/HSR_All_School_Performance_List/2026/i`), athlete pages `/athletes/<id>.html`.
6. **Stable identifiers** — athlete `/athletes/<numeric_id>.html` (e.g. `8429704`); meet `/results/<meet_id>/<slug>/` (e.g. `96753/River_States_Conference_Track__Field_Championships/`); event within meet `/results/<meet_id>/<event_id>/<slug>/<Event-Name>` (e.g. `96753/5981923/.../Womens-10000-Meters`); list `/lists/<list_id>/...` (e.g. `5489`).
7. **Pagination** — performance lists expose a display-size control (`Top 5 | Top 10 | Top 20 | Top 25 | Top 30 | Top 50 | Top 100 | Top 200 | Top 500`) and an event/year filter (SR/JR/SO/FR/8/7/6) — i.e. client-page-size selection over one large HTML page (the IN list capture is 1,880,743 B).
8. **Athlete fields** — name (`EVAN WILLIAMS`), class year in parentheses (`(SR)`), team (`LAWRENCE CENTRAL`), conference (IH…), plus per-event best marks listed with meet name and date (`tfrrs-athlete-8429704.html`).
9. **Meet fields** — meet name, date, venue (`NH` home: `Sep 19 | Manchester Invitational | Derryfield Park`), and per-event result pages with place/name/grade/team/mark.
10. **Result fields** — `Place`, `Athlete`, `Year` (grade), `Team`, `Mark`, `Meet`, `Date` (e.g. rows from `indiana-tfrrs-list-5489.html`: `5 | Rylan Hainje | SR | Franklin Central | 6.84 | Warren Central HSR Qualifier #1 | Mar 4, 2026`).
11. **Grade/class evidence** — **yes, two ways**: (a) grade column in performance lists and a `Year` filter (SR/JR/SO/FR/8/7/6), (b) class year printed after the athlete name on the athlete page (`EVAN WILLIAMS (SR)`).
12. **Coach/contact fields** — none observed in the captured page set (no coach/email fields on home, list, event, meet, or athlete pages).
13. **Public API availability** — none observed; HTML only. No JSON endpoints in the captures.
14. **Static file availability** — n/a (server-rendered HTML).
15. **Browser requirement** — **no**: every captured TFRRS page returned 200 to plain curl (www + all three HS subdomains).
16. **Request cost** — 1 request per list (up to ~1.9 MB), per meet, per event, per athlete page; `results_search_page.html` is 1 request per state×sport×year.
17. **Published rate limits** — none; `/robots.txt` is a 99-byte comment-only file (`# See https://www.robotstxt.org/...`, no directives — `robots-www.tfrrs.org_robots.txt`). Self-limit 1 req/s.
18. **Known blocks** — none observed (all 200).
19. **Cross-source join keys** — **directAthletics → TFRRS**: DAT results-index rows link straight to TFRRS meet URLs (§1.19). TFRRS → Athletic.net: **no key observed**; TFRRS → MileSplit: none observed. `[INFERENCE]` The HS instances (FL/IN/NH) are association-run mirrors of state meet data that also exists on those states' association sites, so name+school+date joins are possible but not key-based.
20. **Estimated marginal coverage** — HS: 3 states only. Measured: 1,671 distinct athlete ids in one IN performance list; each list page carries hundreds-to-thousands of marks. `[INFERENCE]` Marginal Co2027 value = FL/IN/NH HS athletes (a genuine gap-closer for those 3 states because the grade column is explicit) + college athletes (out of census scope).
21. **Recommendation** — **RESULT_SOURCE** for the three HS instances; **VALIDATION_SOURCE** for the national/college surface.

## 3. AthleticLIVE (Athletic.net's live-results platform)

1. **Source name** — AthleticLIVE: SPA at `live.athletic.net`, per-tenant "livestatic" hosts (`livestatic.athletic.net`), a public Elasticsearch at `search.athletic.live`, an Azure blob container `athleticlive.blob.core.windows.net/$web/…`, a Firebase RTDB (`…firebaseio.com` with `ns=trackmeet-io`, i.e. the legacy TrackMeet.io), and per-tenant vanity domains (e.g. `results.wayzatatiming.com`).
2. **Geographic coverage** — national + international: 256 tenants in the package inventory (`data/athleticlive-tenant-inventory.csv`), plus non-US tenants visible in the SPA list (`athletics_canada`, `athleticsnewzealand`, `athleticsontario`, `athleticsnsw`, `athleticsauckland`, … — `samples/athleticlive/al-spa-tenant-list.txt`, 258 entries incl. `base-site`). Measured per-state meet-doc counts (all-time, `match` on `lsa`, `*_meet_list`): IL 14,599 · CA 12,703 · MI 10,775 · NY 9,664 · MN 7,502 · IA 7,169 · PA 5,937 · ND 5,222 · OH 4,542 · NE 4,534 · WI 3,902 · TX 3,454 · SD 3,188 · MO 2,517 · IN 2,233 · KS 1,698 · FL 425 · GA 206 (`al-es-msearch-state-counts.json`; counts are full-text matches on the text field `lsa`, so they are upper bounds — see Open Q2). Restricted to `sdy >= 2025-08-01` the same states return OH 2,512 · IL 4,664 · MI 2,914 · IN 768 · WI 1,577 · MN 2,079 · IA 2,025 · MO 1,154 · KS 368 · NE 1,212 · ND 2,048 · SD 1,161.
3. **Sports** — track & field (indoor/outdoor) and cross country; event docs carry `ab`/`un` (e.g. `1600m`/`1600`), `peg` (e.g. `Distance`), `runm` (`Finals`), `ec` (`Individual`), `es` (`r`), `xc: false`.
4. **Historical depth** — deep and date-queryable: wildcard `match_all` over `*_meet_list` returns **153,774** meet docs (`al-es-msearch-state-counts.json`, last msearch response), which matches the package inventory's tenant sum of 153,524 meet docs (`data/athleticlive-tenant-inventory.csv`: 256 tenants, 153,524 docs, 129,203 with an Athletic.net meet id, 84.2%). Date filter field observed: `sdy` (`range: {sdy: {gte: "2025-08-01"}}`).
5. **Discovery mechanism** — (a) **Elasticsearch** at `POST https://search.athletic.live/_msearch` with index patterns `<tenant>_meet_list` / `*_meet_list` (index list includes `live_results_meet_list`, `athleticlive_meet_list`, `wayzata_meet_list`, …); (b) the **tenant inventory CSV** (package) / the SPA tenant list; (c) **Azure blob** `…/$web/<index>/_doc/<id>` (e.g. `ind_res_list/_doc/2254285`, `ind_res_list/_doc/2254280`) and `meet_<id>/event_summary.json` (404 for the probed meet — path template differs from the RTDB one); (d) **Firebase RTDB** `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_61710/event_summary.json?ns=trackmeet-io` (200, 19,932 B); (e) `https://live.athletic.net/sitemap.xml` (only **17** `<loc>` entries: route templates such as `/meet-list`, not per-meet URLs); (f) `livestatic.athletic.net/assets/sites/<tenant>/config.json`.
6. **Stable identifiers** — AthleticLIVE meet id (`i` / `mi`, e.g. `61710`, `59382`), **Athletic.net meet id** (`athleticnet_meet_id` in `al-es-sample-2026.json`; package inventory counts 129,203 of 153,524 docs = 84.2% carrying one), event id (`i: 2254285`), team id (`t.i: 1469981`), athlete id (`a.i: 44251790`), **Athletic.net athlete id** (`a.ani: 15728090` — observed directly in `al-blob-ind-res-2254285.json`), tenant slug (`wayzata`, `michiana`, `live_results`, …), sport-season id (`se.i: 103434`, `se.n: "Track - Day 1"`).
7. **Pagination** — ES: `size`/`from` + aggregations (default-size probes returned `hits.total` with `hits: []` for `size: 0`); blob docs are single JSON documents per event (one GET returns the whole event, 30,436 B and 17,175 B in the two captures); `_msearch` batches many tenant queries in one request (our 256-tenant and 31-query batches both used it).
8. **Athlete fields** — from the blob result row (`al-blob-ind-res-2254285.json`, `r[0].a`): `i` 44251790, `n` "Edward Mugisha", `fn` "Edward", `l` "Mugisha", **`y` "12" (grade!)**, `g` "Male", `ag` "", `mi` 61710, `ani` 15728090 (Athletic.net athlete id), `t` {`i` 1469981, `n`/`f` "Unattached", `lg` team logo URL}.
9. **Meet fields** — meet id + name (`n` "MITS" / `sn` "Boys 1600m (MITS)"), `sdy`/`se.ds` dates (`2026-02-14`), `se.i`/`se.n` session, `mms` "RunMeet", `us` (live-results link, e.g. `http://fatresults.com/nsvii3`), tenant, and in the ES meet doc the Athletic.net meet id, `city_state`, `state`, `start`, `end`, `has_results` (see `data/athleticlive-midwest-2026-meets-all.csv` columns).
10. **Result fields** — `r[]` rows: `p` place ("1"), `s` time ("4:32.33"), `m` ("4:40.80"), `hn` heat, `ro` round, `w` wind, `gap`, `pt`, `im`/`vm`, `hl`, `mps` (split list), plus `irds`/`itds` on the event doc (result/athlete counters).
11. **Grade/class evidence** — **yes, explicit grade per athlete per result** (`a.y: "12"`), plus event-level grade buckets (`ry: ["3","5","7","8","9","10","11","12"]` on the event doc, i.e. the grade range present in that event). This makes AthleticLIVE the only national source in this lane that stamps grade onto each performance row.
12. **Coach/contact fields** — none observed in the ES/blob captures. `[INFERENCE]` Tenant-level contact info may exist in `livestatic…/config.json`; the two captured configs (base-site, wayzata) were not exhaustively scanned for contacts — unverified.
13. **Public API availability** — the ES endpoint requires **no credentials at all**: an anonymous `POST` with only `Content-Type: application/json` returns 200 for the count query (`samples/athleticlive/al-es-anon-probe.json`: `hits.total.value = 75526`, captured 2026-09-22T04:10:01Z, no auth headers). There is no documented API. The only place an API-key identifier appears in our captures is the error body of `GET https://search.athletic.live/robots.txt`, which returns an **ES `security_exception` as JSON with status 403** naming key id `CD3BXYEBTUeJujAXUcKH` (`al-search-robots.txt`) — i.e. auth is enforced for admin/index-metadata paths, not for `_search`. Note also that the shipped bundle does **not** contain the string `search.athletic.live` (`grep -c` = 0 in `al-main-js.js`), so the index host was found by observation, not by reading the bundle.
14. **Static file availability** — yes: Azure blob `$web` container (public read) and per-tenant `livestatic` JSON configs; the whole app bundle `main-5ADGVJIV.js` (295,358 B) is public.
15. **Browser requirement** — **yes for the HTML surface, no for the data**: `https://live.athletic.net/meets/59382` returns a 50,184 B Angular shell (`<title>AthleticLIVE</title>`) whose rows are fetched client-side; a tenant URL redirects to the tenant's own host (`results.wayzatatiming.com/meets/59382`, same 50,184 B shell). The ES/blob/RTDB data path is curl-reachable without a browser.
16. **Request cost** — 1 request per tenant-index `_msearch` (batch all tenants: 1 request for 256 tenants / 44,948 B response) + 1 request per per-state/date-filtered count + 1 blob GET per event doc; a per-meet harvest is 1–2 GETs (event summary via RTDB/blob + event result docs via blob).
17. **Published rate limits** — none published. `live.athletic.net/robots.txt` (186 B): `Disallow: /admin*`, `/meets/*/athletes*`, `/meets/*/live*`, `/meets/*/teams*`, `/meets/*/follow*` + `Sitemap: …/sitemap.xml`; tenant hosts publish the same policy (e.g. `live.michianatiming.com/robots.txt`, identical text). `livestatic.athletic.net/robots.txt` → 404. `athleticlive.blob.core.windows.net/robots.txt` → 400 `OutOfRangeInput` (no policy).
18. **Known blocks** — none: ES queries (200), blob docs (200), RTDB (200), tenant config (200), SPA shell (200). Robots **does** forbid the HTML `/meets/*/athletes|teams|live|follow` paths, so an implementation must use the data path (or negotiate permission) rather than crawling those pages. Compliance note: the ES endpoint answers **anonymous** queries (verified: `al-es-anon-probe.json`) and is therefore public in practice, but it is **not** a documented or published API — treat as CONDITIONAL, see Open Q3.
19. **Cross-source join keys** — **to Athletic.net: direct, two ways** — (a) the meet doc field `athleticnet_meet_id` (84.2% of tenant meet docs carry one; e.g. `55274 → 259955`), (b) the per-athlete field `ani` (Athletic.net athlete id) and team id `t.i` (Athletic.net team id space — the blob team object mirrors Athletic.net team shape with `lg` static logo URLs). To MileSplit: none observed. Cross-tenant duplication is real and must be de-duplicated by `athleticlive_meet_id` (the package's Midwest file has 20,254 rows for 122 tenants; the same meet appears under several tenants, e.g. `55274` under both `live_results` and `palatine`).
20. **Estimated marginal coverage** — highest in the lane. `[INFERENCE]` Marginal Co2027 = the subset of the 153,774 meet docs whose blob event rows carry `a.y` in {9,10,11,12} for Co2027-relevant seasons and whose meet/athlete ids are not already in the Athletic.net corpus; the package's `meet_docs_with_ani` (129,203) overstates marginal value because those docs are already representable on Athletic.net — the true marginal value is per-*athlete* rows only obtainable here (grade + marks) for meets whose Athletic.net profile coverage is incomplete. Not yet measured (Open Q1).
21. **Recommendation** — **PRIMARY** (data path), with the `CONDITIONAL` compliance caveat in Open Q3.

## 4. RunnerSpace / DyeStat / ResultsCentral

1. **Source name** — RunnerSpace (`www.runnerspace.com`), DyeStat (`www.dyestat.com`), Nike meet subdomains (`nxn.runnerspace.com`, `nxrnw.…`, `nxrhl.…`), ResultsCentral (`resultscentral.runnerspace.com`).
2. **Geographic coverage** — national (event-branded microsites: NXN + 8 regional NXR sites; DyeStat = HS news/hubs), plus college/pro/road-racing hubs ("HUBS HIGH SCHOOL COLLEGE PRO ROAD RACING TRAINING STATE/PROV").
3. **Sports** — XC and T&F (event microsites), road racing, plus video/news.
4. **Historical depth** — NXN results go back to 2004 ("2004 NTN Results"); the captured title page lists 2004–2025 result sets (`rs-nxn-2025-results-title187.html`: 2025, 2024, 2023 … 2004).
5. **Discovery mechanism** — per-event `eprofile.php?do=title&title_id=<id>&event_id=<id>` pages (e.g. `nxn.runnerspace.com/eprofile.php?do=title&title_id=187&event_id=13`), linked from each microsite's RESULTS/PHOTOS/VIDEOS nav; ResultsCentral is a **newsfeed of "2026 Results - <meet>" posts**, each linking to the meet's live-results host (153 news items in 2026).
6. **Stable identifiers** — `title_id` (e.g. 187/197), `event_id` (e.g. 13 = NXN), region site + its `event_id` (248 = NXR Northwest, 300 = NXR Heartland per the captured links).
7. **Pagination** — none needed: each title page is a self-contained results document (captured page = 120,248 B of HTML incl. full result tables).
8. **Athlete fields** — results rows: `Place`, grade code (`SR/JR/SO/FR`), `NAME` (LASTNAME caps), `Time`, club/team (`Herriman`, `Niwot`, `SE Individuals -2`).
9. **Meet fields** — meet title ("2025 RESULTS"), date, venue (in the results document), and the microsite brand/region.
10. **Result fields** — place, grade, name, mark, team; results are **finals-only tables** copied into the page (no splits).
11. **Grade/class evidence** — **yes**: grade code column in the NXN result tables (captured rows: `1 SR Jackson SPENCER 15:01.1 Herriman`, `2 JR Yohanes VAN MEERTEN …`, `10 SO Jack MCGOVERN …`).
12. **Coach/contact fields** — none observed.
13. **Public API availability** — none observed; HTML only.
14. **Static file availability** — none observed.
15. **Browser requirement** — **yes**: `www.runnerspace.com` and `www.dyestat.com` return **403** (Cloudflare challenge HTML, 5,346/5,342 B) to plain curl (`rs-home.html`, `ds-home.html`); the same pages and the NXN results page load normally in a managed Chromium session (captured).
16. **Request cost** — 1 browser request per result document; small universe (≈9 event microsites + regionals × seasons).
17. **Published rate limits** — `robots.txt` on all four probed hosts is the same 245-byte policy: `Disallow: /staging/ /custom/ /forum/ /attach.php /rss.php /ajax.php`, `Crawl-delay: 10`, plus a deny-all for `Amazonbot`/`ltx71`/`SEOkicks`. Respect ≤0.1 req/s on the HTML surface.
18. **Known blocks** — Cloudflare bot challenge on HTML (403 for curl). Robots additionally requires `Crawl-delay: 10`.
19. **Cross-source join keys** — **to Athletic.net/MileSplit: none observed** (no ids or cross-links in the captured pages). The only join path is name+school+meet-text. ResultsCentral posts link outward to live-results hosts (frequently AthleticLIVE or timing-company domains), which is a **text-level** discovery bridge to AthleticLIVE tenants.
20. **Estimated marginal coverage** — `[INFERENCE]` low for census purposes: national-championship fields only (≈200–500 athletes/yr at NXN + regionals), but those are exactly the athletes already indexed by Athletic.net/MileSplit, so marginal *unique* returns ≈0 for the census; value is grade-verified validation of national-class marks and a meet→live-host link map.
21. **Recommendation** — **DISCOVERY_SOURCE** (ResultsCentral event→host map, `Crawl-delay: 10` respected); **CONDITIONAL** for result documents.

## 5. MaxPreps

1. **Source name** — MaxPreps (`www.maxpreps.com`, CBS Sports/Paramount).
2. **Geographic coverage** — national, per-state sections (`/tx/track-field/`, `/oh/track-field/`) and per-school pages.
3. **Sports** — many HS sports; track & field is a first-class section (`/track-field/` with Teams/Athletes/States/Stat leaders nav).
4. **Historical depth** — 2026-27 season pages plus career pages (`/athletes/<slug>/?careerid=…`); TF "stat" depth is minimal (see 10).
5. **Discovery mechanism** — state → schools index (`/tx/track-field/schools/`, 359,208 B), team pages (canonical `/{state}/{city}/{team-slug}/track-field/`), national/state athlete directories (`/track-field/athletes/`, `/oh/track-field/athletes/` — 200 and 78 athlete links respectively on page 1/2 captures), legacy mobile URL that 301s to the canonical team URL.
6. **Stable identifiers** — athlete career URL `/{state}/{city}/{team}/athletes/<athlete-slug>/?careerid=<token>` (e.g. `gage-baker` / `careerid=9qdc8bm62h599`); team `/{state}/{city}/{team-slug}/` (legacy `schoolid`/`allseasonid` GUIDs redirect into it — capture `mp-mobile-team-aliedo.html`); no numeric athlete id observed.
7. **Pagination** — `/track-field/athletes/?page=2` observed (200 athlete links per page).
8. **Athlete fields** — name, school, class year *only on career pages rendered by JS* `[INFERENCE — no class/grade field found in the served HTML; see 11]`; `height`/`position`/`weight` JSON keys exist in the career page payload.
9. **Meet fields** — schedules per team; **no meet results/entries** in the captured TF pages.
10. **Result fields** — TF "stat leaders" are event lists, not meet results; no per-performance rows with marks were found in the captured pages.
11. **Grade/class evidence** — **not in served HTML**: the captured career page contains no `Class of 20XX`/`graduatingClass` string (`mp-athlete-gage-baker.html`); the athletes directory exposes year tokens 2023–2028 only as part of links/labels, not as an athlete attribute. The roster page for a real team (`mp-aledo-roster.html`) contains **no athlete ids and no grades**.
12. **Coach/contact fields** — staff page captured (`mp-aledo-staff.html`, 168,932 B) but no coach name/email extracted from it; per prior package notes TF staff data is thin. `[INFERENCE]` coach names may exist for other sports; TF coverage unverified.
13. **Public API availability** — none documented; the site is Next.js with server-rendered payloads.
14. **Static file availability** — none observed.
15. **Browser requirement** — the crawled pages render server-side (curl 200, no challenge; notice the `meta name="targeting"` Next.js metadata) but athlete career data appears JS-driven.
16. **Request cost** — 1 request per state/school/athlete page; national athlete directory paginates by 200.
17. **Published rate limits** — `robots.txt` is a full policy (5,094 B, **191 `Disallow:` rules**) and it explicitly disallows the paths a census would need: `/school/`, `/team/`, `/discovery/`, `/careerprofile/`, `/association/`, `/m/team/`, `/m/school/`, `/*/admin/`.
18. **Known blocks** — robots-prohibited surface; no technical block observed (pages fetched with curl returned 200 before the robots review — recorded here for the record; no further fetches performed).
19. **Cross-source join keys** — **none** to Athletic.net/MileSplit (no ids or cross-links observed). Only name+school+city joins.
20. **Estimated marginal coverage** — ≈0 for performances (none exposed); possible roster-name overlap only, which is not a Class-of-2027 performance source.
21. **Recommendation** — **REJECT** (robots-prohibited for the relevant paths; no grade; no results).

## 6. World Athletics

1. **Source name** — World Athletics (`worldathletics.org`), GraphQL API at `graphql-prod-4895.edge.aws.worldathletics.org/graphql`.
2. **Geographic coverage** — global (190+ member federations); US entries are international-class athletes only.
3. **Sports** — track & field, road running, XC, race walking (i.e. not HS-sport scoped).
4. **Historical depth** — deep (career-long athlete profiles with birth dates and competition results).
5. **Discovery mechanism** — athlete directory (`/athletes`, 33,382 B page + `athletes-*.js` chunk) backed by GraphQL queries: `getAthletes`, `searchAthletes`, `getAthlete(urlSlug|id)`, `getTrendingAthletes`, plus `getAllCompetitions`/`getCalendarCompetitions` families (query field list captured in `wa-graphql-queryfields.json`).
6. **Stable identifiers** — `urlSlug` (e.g. `malaika-mihambo-14377384`), numeric `id` (e.g. 14377384 = internal, 252802 = `iaafId` for the same athlete) — see the trap in 18.
7. **Pagination** — not exercised; `searchAthletes` takes `eventId` + `searchValue` and returned `[]` for `eventId: 0` (`wa-graphql-search-probe.json`), so paging semantics for a global name search were not established (Open Q4).
8. **Athlete fields** — `AthleteNewData` type (introspected, 974 B): `id`, `iaafId`, `firstName`, `lastName`, `fullName`, `friendlyName`, `sexCode`, `sexName`, `countryCode`, `countryName`, `birthDate`, `birthPlace`, `birthPlaceCountryName`, `urlSlug`, `biography`, `primaryMedia`, representative fields.
9. **Meet fields** — competition calendar entities (`getAllCompetitions`, `getAllSchedules`, `getCalendarCompetitions…` in `wa-graphql-queryfields.json`).
10. **Result fields** — `getAthleteResults` / all-time-list queries exist in the schema (`getAllTimeList`); not exercised in this window.
11. **Grade/class evidence** — **no grade/class** (no HS-grade concept); `birthDate` is present (e.g. `1994-02-03T00:00:00.000Z`), from which a *nominal* class year can only be inferred, poorly for US HS cohorts.
12. **Coach/contact fields** — `getAthleteRepresentativeDirectory`/`…Profile`/`…AthleteSearch` query fields exist (agent) — coach-level contacts for elite athletes only; not exercised.
13. **Public API availability** — **yes, undocumented but reachable**: POST GraphQL with header `x-api-key: da2-q7toieeiobcjxbov4abq3abk5u` **and** `referer: https://worldathletics.org/`; introspection is enabled (full introspection response captured, 15,163 B). Without the key/referer the endpoint returns **503** (`wa-graphql-metadata-curl.json`, 1,019 B HTML).
14. **Static file availability** — none relevant (sitemap index 302; `worldathletics.org/_next/static/chunks/...` JS only).
15. **Browser requirement** — **no** for the GraphQL API; the `/athletes` HTML page itself is server-rendered.
16. **Request cost** — 1 POST per query; batching multiple fields in one query is possible (relay-style).
17. **Published rate limits** — none. `robots.txt` is 106 B and effectively permissive: `User-agent: *`, `Allow: /sitemap`, `Allow: /`.
18. **Known blocks** — the **numeric-id trap**: `getAthlete(id: 252802)` and `getAthlete(id: 14377384)` both return a placeholder record (`{"id":7911,"fullName":" ","countryCode":"FRA",…}` — `wa-graphql-athlete-252802.json`, `wa-graphql-athlete-14377384.json`), while `getAthlete(urlSlug:"malaika-mihambo-14377384")` returns the real athlete. An implementation keyed on the numeric ids would silently import a bogus athlete.
19. **Cross-source join keys** — **none** to Athletic.net/MileSplit. Names + birth year are the only join path.
20. **Estimated marginal coverage** — `[INFERENCE]` very low for the Co2027 US HS census (a handful of top juniors may hold international entries); value is cross-checking elite marks and birth years.
21. **Recommendation** — **VALIDATION_SOURCE** (never a discovery or result source for the census).

## 7. USATF (Junior Olympics)

1. **Source name** — USATF (`www.usatf.org`) national-office pages for the Junior Olympic program.
2. **Geographic coverage** — US: national championships + 56 associations/16 regions as described on the event pages.
3. **Sports** — T&F and XC (JO program); age divisions, not HS grade divisions.
4. **Historical depth** — per-year event pages (2026 captured); sitemap holds 3,890 URLs, 101 of them `junior-olympic` pages (`usatf-sitemap-www.xml`).
5. **Discovery mechanism** — sitemap (`https://usatf.org/sitemap.xml` → 301 → `https://www.usatf.org/sitemap.xml`, 744,058 B) + event pages `/events/<year>/<slug>` with per-section subpages.
6. **Stable identifiers** — none of our ids; the pages instead **point at other platforms** (see 19).
7. **Pagination** — n/a (event pages are single pages).
8. **Athlete fields** — none on the national pages (entries live on the linked platforms).
9. **Meet fields** — event name, date window, venue, timetable and standards documents (e.g. "Last Updated 08/01/2026 at 4:05 p.m. PT").
10. **Result fields** — none hosted by USATF here; results live on TrackScoreboard/FinishedResults (timing-provider lane).
11. **Grade/class evidence** — none (JO divisions are age-based).
12. **Coach/contact fields** — none observed.
13. **Public API availability** — none observed.
14. **Static file availability** — sitemap XML only.
15. **Browser requirement** — **no** (all captured pages returned 200 to plain curl).
16. **Request cost** — trivial: 1 request for the sitemap, 1 per event page.
17. **Published rate limits** — `usatf.org/robots.txt` is 96 B: `user-agent:*`, `Sitemap: …/sitemap.xml`, `Disallow: /admin`, `Disallow: /cmsdesk` → permissive.
18. **Known blocks** — none on USATF. (AAU is the blocked youth-program source — see §8.)
19. **Cross-source join keys** — **direct**: the JO T&F page's own registration link is `https://www.athletic.net/TrackAndField/meet/644030/register` (Athletic.net MeetID **644030**) and its "Live Results" link is `https://finishedresults.trackscoreboard.com/meets/14258/events` (TrackScoreboard meet **14258**) — one page yields a verified USATF meet ↔ Athletic.net meet ↔ live-results mapping. The JO XC page instead points at `usatf.sport80.com/register/membership?id_add_on=941` and carries no results link.
20. **Estimated marginal coverage** — JO athletes are age-division youth and largely not Co2027 `[INFERENCE: youth divisions overlap HS ages only partially]`; value = meet-id crosswalks + knowing which national meets use which results host.
21. **Recommendation** — **VALIDATION_SOURCE**.

## 8. AAU

1. **Source name** — AAU (`aausports.org`, `www.aausports.org`, `aautrackandfield.org`).
2. **Geographic coverage** — not observable (blocked). `[INFERENCE]` US national districts, youth age divisions.
3. **Sports** — not observable; `aautrackandfield.org` 301-redirects into the association's track & field hub (name evidence only).
4. **Historical depth** — not observable (blocked).
5. **Discovery mechanism** — not observable (blocked).
6. **Stable identifiers** — not observable (blocked).
7. **Pagination** — not observable (blocked).
8. **Athlete fields** — not observable (blocked).
9. **Meet fields** — not observable (blocked).
10. **Result fields** — not observable (blocked).
11. **Grade/class evidence** — not observable (blocked).
12. **Coach/contact fields** — not observable (blocked).
13. **Public API availability** — none observed; `/robots.txt` returns 404 with an empty body (`other-www.aausports.org_robots.txt`, 0 B).
14. **Static file availability** — not observable (blocked).
15. **Browser requirement** — a browser might pass the Cloudflare challenge, but that is a bypass of an access control and was **not attempted**.
16. **Request cost** — n/a.
17. **Published rate limits** — none (no robots policy published; 404).
18. **Known blocks** — `www.aausports.org/sports/track-field` → **403 Cloudflare challenge** (`<title>Attention Required! | Cloudflare</title>`, 5,787 B, `aau-sports-track.html`); `aautrackandfield.org` → 301 → `aausports.org/track-and-field` → **403** (`aau-track-hub.html`, `aau-track-hub-followed.html`). No bypass attempted.
19. **Cross-source join keys** — none reachable (blocked).
20. **Estimated marginal coverage** — **0 as measured** (no content reachable anonymously). `[INFERENCE]` AAU youth results would duplicate Athletic.net/timing-provider coverage where they exist at all.
21. **Recommendation** — **REJECT** (403 to anonymous clients; not worth a bypass).

## 9. Cross Country Ratings

1. **Source name** — Cross Country Ratings (`www.crosscountryratings.com`).
2. **Geographic coverage** — 9 states: Iowa, Nebraska, Minnesota, North Dakota, South Dakota, Illinois, Wisconsin, Missouri, Kansas (site nav, `ccr-home.html` / `ccr-runner.html`).
3. **Sports** — cross country only (nav: `Cross Country / Ratings / Results / Rankings / Individuals / Teams / Analytics`; `Meet Forecast 🔒`, `Goal Setting 🔒`, `Search 🔒` show a partial paywall for analytics features, not for results).
4. **Historical depth** — current 2026 season with per-athlete meet history (links to `/event/<id>`).
5. **Discovery mechanism** — `/individuals` (240 runner links), `/teams`, `/results` (101 event links), `/runner/<id>`, `/school/<id>`.
6. **Stable identifiers** — runner `11669`, school `18`, event `1010` (numeric, path-stable).
7. **Pagination** — DataTables client-side paging/sorting over server-rendered tables (no server pagination observed).
8. **Athlete fields** — name (`Kuma Gutema`), grade word + town ("Junior • Sioux City, North"), **`Class of 2028`**, season, per-meet marks (`14:30.8`, `14:53.2`, …).
9. **Meet fields** — meet name, date, event id.
10. **Result fields** — 5K XC times per meet; team/individual ranking tables on `/individuals` and `/teams`.
11. **Grade/class evidence** — **yes**, explicit `Class of 20XX` plus the grade word on the runner page.
12. **Coach/contact fields** — none observed.
13. **Public API availability** — none; `robots.txt` explicitly disallows `/api/` ("Block admin and API endpoints").
14. **Static file availability** — sitemap declared (`Sitemap: https://crosscountryratings.com/sitemap.xml`).
15. **Browser requirement** — **no** (all captures curl 200, content server-rendered).
16. **Request cost** — 1 request per runner/team/event page.
17. **Published rate limits** — `robots.txt` (1,374 B): `User-agent: * / Allow: / ; Crawl-delay: 1`; disallows `/admin/`, `/api/`, one PDF; explicit `Allow:` for `/individuals`, `/teams`, `/results`, `/search`, `/help/`, `/runner/`, `/school/`, `/event/`, `/race/`; per-bot `Crawl-delay: 5–10` for AI crawlers. **Crawl-delay: 1 must be respected.**
18. **Known blocks** — none on allowed paths.
19. **Cross-source join keys** — none to Athletic.net/MileSplit (no ids or cross-links observed); name+town+school text joins only.
20. **Estimated marginal coverage** — `[INFERENCE]` small but grade-explicit: XC-only, 9 states whose meets overlap heavily with Athletic.net/MileSplit; value is spot-checking Co2027 XC class years and marks.
21. **Recommendation** — **CONDITIONAL** (validation/spot-check; low effort, robots-allowed).

## 10. Screened and rejected candidates (required fields, compact)

Three further multi-state candidates were screened in the window and rejected before qualification. Because they were rejected at the first step, most required fields are "not qualified"; the table records exactly what was and was not established.

| field | NCSA (`www.ncsasports.org`) | FieldLevel (`www.fieldlevel.com`) | Baumspage (`www.baumspage.com`) |
|---|---|---|---|
| 1. Source name | NCSA recruiting network | FieldLevel recruiting network | Baumspage (Ohio results host) |
| 2. Geographic coverage | not qualified (robots only) | not qualified (robots only) | not qualified `[INFERENCE]` Ohio-centric |
| 3. Sports | not qualified | not qualified | not qualified |
| 4. Historical depth | not qualified | not qualified | not qualified |
| 5. Discovery mechanism | not qualified | not qualified | not qualified |
| 6. Stable identifiers | not qualified | not qualified | not qualified |
| 7. Pagination | not qualified | not qualified | not qualified |
| 8. Athlete fields | not qualified (paywalled recruit profiles) | not qualified (recruit profiles) | not qualified |
| 9. Meet fields | not qualified | not qualified | not qualified |
| 10. Result fields | not qualified | not qualified | not qualified |
| 11. Grade/class evidence | not qualified | not qualified | not qualified |
| 12. Coach/contact fields | not qualified | not qualified | not qualified |
| 13. Public API availability | none observed | none observed | none observed |
| 14. Static file availability | unknown | unknown | unknown |
| 15. Browser requirement | unknown | unknown | unknown |
| 16. Request cost | n/a | n/a | n/a |
| 17. Published rate limits | `robots.txt` 200 (2,673 B) — policy retained, no content rules read | `robots.txt` 200 (110 B) — policy retained | `robots.txt` 404 (1,510 B HTML) — no policy |
| 18. Known blocks | paywalled athlete surface; not a results source | coach-side product; no meet results | belongs to the timing-provider lane |
| 19. Cross-source join keys | none observed | none observed | none observed |
| 20. Estimated marginal coverage | 0 for performances | 0 for performances | 0 for this lane (owned by the timing-provider lane) |
| 21. Recommendation | **REJECT** | **REJECT** | **REJECT** (lane mismatch) |

## Cross-source join keys (summary)

| from → to | key | evidence |
|---|---|---|
| AthleticLIVE → Athletic.net (meet) | ES meet-doc field `athleticnet_meet_id` (84.2% of tenant docs) | `samples/athleticlive/al-es-sample-2026.json`, `data/athleticlive-tenant-inventory.csv`, `data/athleticlive-midwest-2026-meet-seeds.csv` |
| AthleticLIVE → Athletic.net (athlete) | blob row `a.ani` (e.g. 15728090) | `samples/athleticlive/al-blob-ind-res-2254285.json` |
| AthleticLIVE → Athletic.net (team) | blob row `a.t.i` (e.g. 1469981) + team object shape (`lg` logo URL) | same |
| DAT → TFRRS (meet) | DAT results-index row link → `tfrrs.org/results/<meet_id>/<slug>/` | `samples/directathletics-tfrrs/dat-results-index-oh-track.html` |
| DAT → TFRRS (list) | DAT `/rankings.html` → `/lists/<league>_<list>` ids; TFRRS `/lists/<list_id>/…` | `dat-rankings.html`, `indiana-tfrrs-list-5489.html` |
| USATF → Athletic.net (meet) | registration link `/TrackAndField/meet/644030/register` on the JO T&F page | `samples/usatf-aau/usatf-jo-tf.html` |
| USATF → TrackScoreboard | live-results link `finishedresults.trackscoreboard.com/meets/14258/events` | same |
| ResultsCentral → live-results hosts | per-post "View Live Results Here" outbound links (AthleticLIVE/timing hosts) | `samples/runnerspace-dyestat/rs-resultscentral-home.html` |
| AthleticLIVE tenant → tenant domain | `live.athletic.net/meets/<id>` 301s to the tenant's own host (`results.wayzatatiming.com/meets/<id>`) | `samples/athleticlive/al-meet-page-59382.html` + CAPTURES log |
| TFRRS/MaxPreps/WA/CCR → Athletic.net or MileSplit | **no key observed** (name+school+date joins only) | negative result across all captures in this lane |

## Compliance record (robots.txt + observed blocks)

| host | `/robots.txt` result | operative rules | our behavior |
|---|---|---|---|
| www.directathletics.com | 200 `text/html` 29,639 B (brand/404 page) | none published | self-limit ≤1 req/s |
| www.tfrrs.org (+florida/indiana/nh) | 200 `text/plain` 99 B, comment only | none | self-limit ≤1 req/s |
| live.athletic.net | 200 186 B | `Disallow: /admin*, /meets/*/athletes*, /meets/*/live*, /meets/*/teams*, /meets/*/follow*`; sitemap declared | HTML meet pages only used to observe the SPA/redirect; data taken from ES/blob/RTDB, not from disallowed paths |
| live.michianatiming.com (tenant sample) | 200 192 B | same policy as above | not crawled |
| search.athletic.live | 403 ES `security_exception` JSON | none published (endpoint, not a crawlable site); `_search` answers **anonymous** queries (verified, `al-es-anon-probe.json`) | only count/doc `_search`/`_msearch` queries at ≤1 req/s |
| athleticlive.blob.core.windows.net | 400 `OutOfRangeInput` | none published | only captured doc GETs |
| livestatic.athletic.net | 404 | none | 2 config GETs |
| www.runnerspace.com / www.dyestat.com / nxn.runnerspace.com / resultscentral.runnerspace.com | 200 245 B each | `Disallow: /staging/ /custom/ /forum/ /attach.php /rss.php /ajax.php`; **`Crawl-delay: 10`** | browser reads only, 2 result pages captured, no sweep attempted |
| www.maxpreps.com | 200 5,094 B, 191 Disallow rules | team/school/discovery/careerprofile blocked | no further fetches after review; recommendation REJECT |
| worldathletics.org | 200 106 B | `Allow: /` | GraphQL POSTs at ≤1 req/s |
| www.usatf.org | 200 96 B | `Disallow: /admin`, `/cmsdesk` | page + sitemap reads |
| www.aausports.org | **404, 0 bytes** | none published | fetched once (403 Cloudflare challenge) → REJECT, no bypass attempted |
| www.crosscountryratings.com | 200 1,374 B | `Crawl-delay: 1`, `/api/` and `/admin/` disallowed | 1 req/1.3 s |
| www.ncsasports.org / www.fieldlevel.com | 200 (2,673 B / 110 B) | permissive | robots only (no content fetches) |
| www.baumspage.com | 404, 1,510 B | none published | robots only |

## Data files copied from the research package (originals untouched)

All copied with `cp -p`; md5 verified identical to `~/Downloads/midwest-tfxc-source-research/data/…`:

| file in `data/` | rows | md5 (matches Downloads original) |
|---|---|---|
| `athleticlive-tenant-inventory.csv` | 256 tenants (153,524 meet docs; 129,203 with Athletic.net meet id = 84.2%) | `c7c0e1dd359d3802c668f17ed1c8ebdc` |
| `dat-team-index.csv` | 22,527 team records, 12 states (OH 3,428 · IL 3,340 · MI 3,230 · MO 1,919 · WI 1,912 · IN 1,816 · KS 1,669 · IA 1,592 · NE 1,266 · MN 874 · SD 758 · ND 723) | `4d80df7cfc2b0494a0acebe9019dce22` |
| `dat-provider-coverage.csv` | 12 states × per-state DAT meet/HS-result notes | `96d24b4ebfab2eebc00f993d51a7abff` |
| `athleticlive-midwest-2026-meets-all.csv` | 20,254 rows, 122 tenants (IL 4,648 · MI 2,892 · OH 2,502 · MN 2,077 · IA 2,015 · WI 1,571 · NE 1,198 · MO 1,146 · IN 764 · SD 753 · KS 364 · ND 324) | `8468f7ded354d4dd635a5293b58bfcc4` |
| `athleticlive-midwest-2026-meet-seeds.csv` | 20,254 rows + `sport_flags_tf`, `timer_credit` | `3ec78fd5bb65a8b9f84301440c50cede` |
| `source-coverage-matrix.csv` | 66 rows (state × source_family × role × ids × grade evidence) | `83ed4f928263315b8882c17fc7a8295c` |
| `adapter-ranking.csv` | 10 ranked adapters with effort/request estimates | `ad485890dd57e4c293590734ca6b6be8` |

## Open questions / unverified

1. **Athlete-level marginal coverage for AthleticLIVE** — not measured. What exists: meet-doc counts (153,774), per-state meet counts, and one event doc with 43 graded result rows. What is missing: a sweep of `ind_res_list` docs per meet to count distinct `a.ani`/`a.y` for Co2027 seasons. Cheap next step: 1 blob GET per (meet, event) for the 20,254-row Midwest meet set.
2. **`lsa` semantics** — the state counts use `match` on a **text** field (`aggregations are disabled for text fields`, `al-es-agg-states.json` = HTTP 400 `illegal_argument_exception`), so the per-state totals are upper bounds (e.g. "Ohio" can match "Ohio Valley"). A `.keyword` field or an exact-term variant would firm the numbers; the field name for a term query was not established.
3. **AthleticLIVE compliance** — the ES endpoint answers anonymous `_search` calls (no credentials, verified), but it is not a documented API and the HTML paths a naive implementation would use are robots-disallowed. Before a production crawl, this lane recommends written permission or a legal read; the recommendation stays CONDITIONAL until then.
4. **World Athletics pagination** — `searchAthletes(eventId, searchValue)` returned `[]` for `eventId: 0`; the working `eventId` values and paging parameters were not established (only the `urlSlug` athlete fetch and `getTrendingAthletes` were exercised).
5. **TFRRS HS instances beyond 3** — `sites.html` lists florida/indiana/nh; probes for texas/ohio/california NXDOMAIN. A complete subdomain enumeration (e.g. remaining states) was not performed; `[INFERENCE]` the three are the set because `sites.html` is the product's own list.
6. **DAT HS result density** — not re-measured this window beyond the 2022 Iowa event sheet + the OH results index (100 meets/page). Whether DAT-hosted HS meets carry grades at scale is the deciding question for a CONDITIONAL → RESULT_SOURCE upgrade.
7. **MaxPreps grade data** — whether a JS-rendered career page exposes a class year was not verified (robots prohibits the path; no browser fetch attempted out of compliance discipline).
8. **USATF JO XC results** — the XC page carries no results link (only `usatf.sport80.com` registration); where JO XC results live in 2026 was not established in this window.
