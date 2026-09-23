# Data source survey — findings and collection design

**Date:** 2026-09-20. **Scope:** alternative public data sources for the athlete-evidence pipeline (HS track & field + cross-country), required because the previous pipeline target (athletic.net) is avoided by owner instruction.
**Status of this file:** reconnaissance findings + operating policy + design direction. Every status,
byte count and quote below is a live probe result from 2026-09-20 and is not a code claim. Since that
date the census crate has landed adapters for MileSplit rosters and the association lanes
(`milesplit`, `wiaa`, `wiaa_results`, `mshsl`, `ihsa`, `ohsaa`, `ks`, `plain_names`, `wayzata`,
`coach_contacts`), the national lanes this survey ranked (`tfrrs`, `athleticlive`,
`athleticlive_athletes`, `athleticnet`), and vendor result-file parsers for the HY-TEK / RunData /
XC / RaceDay artifacts (`crates/census-crawl/src/{hytek,compiled,xc,raceday}`, dispatched by
`crates/census-crawl/src/result_file.rs`) — the tree is the current state; this file is the reconnaissance input
that chose them. `crates/census-crawl/src/registry/table.rs` is the authoritative slug list.
**Method note:** `web_search` was unavailable at the provider level during this survey (provider errors), and the HTML search engines it fell back to were captcha-gated or empty (DuckDuckGo 202/no links, Mojeek captcha, Bing empty). All discovery therefore ran through platform APIs (GitHub, Kaggle, HuggingFace, Zenodo, Figshare, data.gov CKAN, archive.org) plus direct page fetches.

---

## 0. Operating policy (owner decision, 2026-09-20)

> "Ignore all site restrictions here we will do like max 2 RPS here on all sites we will design this intelligently so we aren't hammering these sites."

*(Recorded 2026-09-20. The implemented rule is item 1 below: robots.txt is enforced by the fetcher
and the only relaxation is an operator naming a host on `--authorized-host`.)*

1. **robots.txt is enforced in code; this file only records it.** The fetcher
   (`crates/census-crawl/src/net/mod.rs`) returns `FetchError::Robots` for a disallowed path and
   counts it in `FetchStats::robots_blocked`. The one relaxation is an operator naming a host on
   `--authorized-host`: the rule is then counted as `robots_authorized` and the request proceeds
   under the 2 rps ceiling. ToS and crawl-delay entries below are survey observations for the
   ledger, not an enforcement decision.
2. **Hard pacing ceiling: 2 requests/second per host** — enforced by a per-host token bucket (500 ms minimum spacing). No host is ever contacted faster than that, regardless of what the host permits.
3. **Intelligent-design requirements** (§17): bounded per-host concurrency, conditional GETs where the host provides `ETag`/`Last-Modified`, whole-document fetches over per-row fetches, exponential backoff on 429/5xx, host cool-downs, and a projected per-host request budget written to the run log.
4. **Stated exception:** `athletic.net` and its subdomains remain **excluded** (earlier owner instruction). The surveys confirm several paths redirect *into* it (athletic.live, live.athletictiming.net, OHSAA/IHSAA delegations, RunnerSpace sign-ups); those targets were not followed. Flagged here only so the owner can reverse that single decision; **no other host is excluded**.
5. Collectors send a fixed, honest User-Agent and honor `Retry-After` when present.

## 1. Summary matrix

`Reachability` is the engineering verdict only (the policy above removes rights from the verdict).

| # | Source | Population | Reachability | Identity | Restriction (recorded) |
|---|--------|-----------|--------------|----------|------------------------|
| 1 | `www.milesplit.com` / `<state>.milesplit.com` | HS (ms/hs/college) | **PRIMARY — JSON API verified working** | `athleteId` + `gradYear` per row | robots: `/rankings`, `/api/`, `/virtual-meets`, `/contact` |
| 2 | `www.tfrrs.org` | college + HS meet results | **PRIMARY — static HTML, no JSON** | college id + class year; HS rows carry DirectAthletics ids | none (robots is a 99-byte comment) |
| 3 | `www.maxpreps.com` | HS | **STRONG — static SSR, per-meet rows w/ wind/timer/round** | `careerId` UUID + per-sport `athleteId` + `classYear` | robots disallows `/careerprofile/`, `/local/`, `/school/`, `/team/`, `/scores/`; athlete URLs allowed |
| 4 | `www.yentiming.com` | NY Section V HS (indoor/outdoor) | **FEASIBLE — live static HTML, 2008–2027, + leaderboards** | name-based | robots 404 |
| 5 | `www.directathletics.com` | HS + college meets | usable (static, filterable index, page=792) | results carry **no athlete id**; profile URL 302 → TFRRS | ToS expressly prohibits commercial exploitation of results/rosters/IDs |
| 6 | `elitefeats.com` | NY HS track (Sections VIII/XI) | **FEASIBLE** (static ASP; PL/Name/YR/Team/Wind/Time) | per-meet bib | robots disallows utility paths; `/t-Results` allowed |
| 7 | `www.baumspage.com` | Ohio HS (XC + TF + MS) | **FEASIBLE** (HY-TEK `<pre>` text) | name+school+grade | none found |
| 8 | `www.finishtimingresults.com` | Ohio HS/college | **FEASIBLE** (autoindex + HY-TEK `<pre>`; some PDFs) | name only | robots allows (`Crawl-delay: 3`) |
| 9 | `www.runnercard.com` | UT/ID/WY HS + JH | **FEASIBLE-WITH-WORK** (legacy report engine, fixed-width text) | per-meet bib | none (robots 404) |
| 10 | `cifss.org` (CIF-SS) | CA HS postseason | FEASIBLE-WITH-WORK (PDF text layer) | name+grade+school | robots allow-all |
| 11 | `*.runnerspace.com` | HS/college/pro per meet | FEASIBLE-WITH-WORK (static PHP, `&nbsp;` columns) | name-only | ToS = Athletic ToS (anti-scraping clauses) |
| 12 | `finishedresults.com` + `api.trackscoreboard.com` | CA/NV HS+college | FEASIBLE-WITH-WORK via API (front end is JS-only) | API athlete ids | api host robots `Disallow: /` |
| 13 | `results.leonetiming.com` | NY/PA | **JS-only app** (engineering blocker) | unknown | robots + ToS prohibit automated extraction and AI/TDM use |
| 14 | `milesplit.live` | HS/college live | JS-only SPA; API + Firebase data plane | inherits MileSplit | no robots (SPA catch-all) |
| 15 | `worldathletics.org` | elite/pro + U20 | parseable static HTML (675 KB records pages) | name+DOB, no id | ToS bans crawlers/automated agents |
| 16 | Kaggle / HuggingFace / Zenodo / Figshare | varies | **bulk download paths verified** (see §8) | varies | dataset licenses vary (one CC0 corpus verified) |
| 17 | `live.athletictiming.net` / `live.athletic.net` | white-label live | **excluded in practice** — AthleticLIVE tenant loading bundles from `livestatic.athletic.net` | n/a | n/a |
| 18 | `athletic.live` | — | **AVOID** — 6/6 probes 302 → `live.athletic.net` (excluded host); not followed | — | — |
| 19 | `www.athletictiming.net` / `athletictiming.rsupartner.com` / `www.tfmeetpro.com` | — | pointers/vendor/road-race only | — | rsupartner robots `Disallow: /`; others none |
| 20 | `mhsaa.com` / `ihsaa.org` / `ohsaa.org` / `stats.ncaa.org` / `opentrack.run` | associations, NCAA | AVOID/BLOCKED for athlete-level (finals recaps, delegations to athletic.net, empty widgets, `Disallow: /`, CF challenge) | — | see §10 |

## 2. MileSplit — primary HS plane (verified end-to-end)

**Hosts:** `www.milesplit.com`, `<state>.milesplit.com` (58 state/regional subdomains), `js.sp.milesplit.com`, `milesplit.live`.

### 2.1 The performance API is open JSON

```
GET https://<state>.milesplit.com/api/v1/meets/<meetId>/performances
    ?isMeetPro=&resultsId=<fileId>&teamScores=team
    &fields=id,meetId,athleteId,firstName,lastName,gender,gradYear,eventName,eventCode,round,heat,units,mark,place,windReading,profileUrl
```
Probed live (ny host, ESM Invitational 2026, results file 1321756): **HTTP 200, `application/json`, 146,703 bytes, 420 rows, all 16 requested fields present on every row.**

```json
{"id":219435694,"athleteId":"13966799","meetId":"774438","gender":"F","units":"1126700",
 "place":"1","heat":"1","windReading":null,"firstName":"Abigail","lastName":"Burt",
 "gradYear":"2029","eventCode":"5000m","eventName":"5000 Meter Run","round":"f",
 "mark":"18:46.70","profileUrl":"https://www.milesplit.com/athletes/13966799-abigail-burt"}
```

- `gradYear` populated on **420/420** rows — the evidence-backed grade/junior signal `SCOPE.md` requires (sampled athlete `Class of 2029` is exactly the 2027-focus population).
- `athleteId` is stable and joins to the profile URL; `windReading` exists as a field (null for XC, as expected); responses carry a `_meta`/`_links` envelope with `cache.ttl`.
- **No auth, no cookies, no JS.** A second endpoint was confirmed open by the ecosystem survey: `GET /api/v1/athletes/<id>/live/top50` → 200 JSON without auth.

### 2.2 Other verified surfaces

- **Meet `/raw`** (`/meets/<id>/results/<fileId>/raw`): static and unmetered — XC tab-separated `Place/Bib/Name/Grade/Team/Time`; track = **raw HY-TEK text** in `<pre>` (141 events, `Wind` columns, per-attempt series, `FOUL/NH/DNS/DNF/DQ` vocabulary). Cross-check / alternative to the API.
- **Meet discovery:** `/results` hub (~50 meets/page; `?season=cc&level=hs&year=2026&page=N`, year filter 2006–2026), `/sitemap.xml` (7,500 athlete URLs), `/meets/<id>/elites`, and the `meetResultFiles` / `meetResultParams` globals embedded in meet pages.
- **Athlete profiles:** id, name, `Class of <year>`, school, PR table (event/mark/national rank/date). Per-performance history is server-side masked for non-PRO (values absent from the HTML — not parser-recoverable).
- **Rankings:** `/rankings/leaders/high-school-{boys,girls}/<season>` with server params `?year=&accuracy=legal&grade=<all|junior|…>&conversion=n`. Verified 200 (105–126 KB) and a **Grade filter including Junior**, but **rows are not in the HTML** — they arrive by XHR. `/api/v1/athlete-rankings` with guessed params returned **501 Not Implemented**; the exact call still needs extraction from the rankings bundles (§16 item 1).
- **No challenge anywhere:** 25/25 requests 200 with plain curl; the `recaptcha` reference is a passive asset and the `cloudflare` match was a CDN `<script>` URL. No `cf-mitigated`, `retry-after`, or `x-ratelimit-*`.
- **Request math:** ≈3 requests per meet (hub page → meet page for file ids → one API call per results file, 1–3 files).

## 3. TFRRS — college + full athlete history (static, open)

- **robots.txt is a 99-byte comment** (no allow/disallow lines). 11 requests, all 200, no challenge, no rate limits; Rails + CloudFront with `ETag`s (`x-runtime` 0.008–0.053 s).
- **Zero JSON** — every table is in the first response; the only XHR-shaped surface is Turbo-frame HTML fragments (`/directory_tab.html?outdoor=0|1&tab=<div>&year=<YYYY>`), a second GET.
- **URL families:** XC meet `/results/xc/<id>/<slug>`; TF meet `/results/<id>/<slug>`; TF event `/results/<id>/<eventid>/<slug>/<Event-Slug>`; athlete `/athletes/<id>/<School>/<Name>.html`; team `/teams/{xc,tf}/<ST>_<college|jcollege>_<f|m>_<Team>.html`; search `/results_search.html` (30/page, pages to **1312 ≈ 39,000 meets**, year filters 2009–2026); lists `/indoor_lists.html`, `/outdoor_lists.html`.
- **One request per athlete career:** id, name, school, class year (`SO-2`), `College Bests`, 4 tabs, **per-performance mark + wind + place + round + meet + date** (`12.18 (0.4)`, `200 | 26.75 (-1.5) | 43rd (F)`, `DNS`/`DNF`); 72–98 performance rows observed on two athletes.
- **HS meets are hosted** (meet 27822, `…High_School_`, 833,212 bytes): HS rows carry **DirectAthletics athlete/team ids** and `YEAR` = **graduation year** [quote] `<td><a href="https://www.directathletics.com/athletes/track/9444606.html">Claire Grennan</a></td><td>2030</td>`; the same meet's college races carry 580 TFRRS athlete links / 635 team links.
- **Parser hazards:** the TF event summary table's 3 round columns are not reliably row-attributable (parse per-section tables with `W:` markers); athlete pages ship **4 duplicate bests tables** (`bests`/`xc_bests`/`indoor_bests`/`outdoor_bests`) — deduplicate or PRs triple-count.
- **Identity note:** HS athletes have no TFRRS id; HS identity lives at DirectAthletics (§5) or MileSplit.

## 4. MaxPreps — HS athlete seasons

- `RESTRICTION (recorded)` — robots disallows `/school/`, `/team/`, `/discovery/`, `/contest/`, `/careerprofile/`, `/local/` (athlete URLs allowed; internal rewrites to `/careerprofile/` were never requested); `terms-of-use` linked, not read.
- No challenge; Akamai/CloudFront; **stat pages are legacy ASPX server-rendered, no JS**.
- Verified row [quote]: `4/17/2026 | Gilbert District City | Final | 19th | | -1.10 | Electronic | 00:12.02` — **date, meet, round, place, wind, timing type, mark** per event in `mx-grid` tables; both seasons in one document (`data-year`), grade present (`Sophomore`), ids stable (`careerId` UUID, per-sport `athleteId`).
- Career pages (~523 KB, Next.js `self.__next_f` chunks) carry `careerData{classYear, graduatingClass, mostRecentSchoolId, …}`.
- Sitemaps: `Players-{gender}-Varsity-{Sport}-{Season}-2026-2027-{n}-Sitemap.xml`; one boys-XC file = **12,929 athlete URLs / 2.5 MB**. Send the literal `&` (percent-encoded `%26` 400s).
- Unproven: XC stat tables (sampled page had 0 tables); leaderboard endpoint `/list/leaderboard_list.aspx?...` (identified, not fetched); depth is school-submission dependent.

## 5. DirectAthletics — identity front door

- **No robots.txt** (apex and www serve homepage HTML at `/robots.txt`); no challenge/WAF/rate-limit across 13 requests.
- Results index `/results.html` + `/legacy_da/results?page=N` — 100 rows/page, links observed to **page=792** (~79,200 rows); filters by sport/state/date. Per-meet pages `/results/{xc|track}/{id}.html` are static and XHR-free.
- **Result rows contain no athlete id** (`grep -c 'athletes/' = 0`); profile URLs `/athletes/{sport}/{id}.html` **302 → `https://www.tfrrs.org/athletes/<id>`**. Search (`POST /site_search.html`) returns athlete rows with stable numeric ids. `rankings.html` (1.95 MB) points 10,601/11,164 links at `*.tfrrs.org`. Upcoming-meet payload `/scripts/fuseDriver_Upcoming.js` = `var data = [...]` (987 meets, indoor/outdoor flag, state, audience flags).
- `RESTRICTION (recorded)` — ToS verbatim: *"Any commercial or promotional distribution, publishing or exploitation of the DirectAthletics meet results, performance lists, roster data, roster names, roster teams, roster years, DirectAthletics IDs, TFRRS IDs, site, or any content, code, data or materials on the DirectAthletics site, is strictly prohibited unless you have received the express prior written permission…"*

## 6. Independent timing hosts

| Host | Reachability | Fields observed | Identity | Notes |
|---|---|---|---|---|
| `yentiming.com` (NY Section V) | **FEASIBLE** — `/results` → `/results/{2008…2027}`, `/leaderboard` (16 events + league/class filters), `files.yentiming.com/indoor/SV-AT.htm` | meet/LEAGUE/class structure; name-based rows | name | robots 404; no challenge; **upstream of the CC0 Kaggle corpus in §8** |
| `baumspage.com` (Ohio) | **FEASIBLE** — `/cc/index.php`, `/track/index.php`, `ccevent.php`/`trevent.php`, archives; HY-TEK `<pre>` | place, name, grade, school, seed, finals, heat, points | name+school | robots 404; no ToS found; archives partially stale |
| `finishtimingresults.com` (Ohio) | **FEASIBLE** — Apache autoindex (325 `.html` in `/2026/`) + HY-TEK `<pre>`; some PDFs | place, name, grade, school, mark, heat | name only | robots allows, `Crawl-delay: 3` |
| `runnercard.com` (UT/ID/WY) | **FEASIBLE-WITH-WORK** — `/results3/` index (1440 meets) → `GET /e/runner.Reports?goto=meetTeam&d=&event=<id>&meet=<id>` fixed-width `First Name\|Last Name\|Gender\|Year\|Team\|Mark\|Bib Num` | name, gender, grade, team, mark, bib | per-meet bib | Mark column unproven (reachable ids resolved to future editions) |
| `elitefeats.com` (NY VIII/XI) | **FEASIBLE** — `/results/s11.asp` → `/t-Results?ID=<meet>` → `/T-Detail.asp?ID=&Bib=` | place, name, grade, team, wind (empty in sample), time (exact value in `title`) | per-meet bib | YR empty in both sampled meets; per-meet page shape not uniform |
| `cifss.org` (CA CIF-SS) | **FEASIBLE-WITH-WORK** — `/results-sitemap.xml` → PDFs; `pdftotext` yields `1 Brown, Jaela 10 Millikan (SS) 11.79 +1.4 1` | name, grade, school, mark, wind, heat, place, round | none | Championship rounds only; upstream `files.finishedresults.com` |
| `*.runnerspace.com` | FEASIBLE-WITH-WORK — per-meet subdomains, `&nbsp;` columns; `Crawl-delay: 10` | place, name, grade, gender, mark, school | none | ToS = Athletic ToS |
| `finishedresults.com` | results = Angular SPA; data plane `api.trackscoreboard.com` (robots `Disallow: /`) exposing `/meets/{id}/athletes/{athlete}?idType=` | unknown (API) | API athlete ids | meet index (655 meets) is plain HTML |
| `results.leonetiming.com` | JS app only | — | — | robots + ToS prohibit automated extraction and AI/TDM use |
| `milesplit.live` | Angular SPA; data plane = MileSplit `/api/v1/*` + Firebase (`milesplit-live-results`; RTDB root 401) | live heats/rounds/scores | inherits MileSplit | live layer is a Phase-2 feature |

## 7. World Athletics (elite reference)

- robots allows (`Allow: /`); the robots-advertised sitemap is dead (302 → 404).
- **Server-rendered static HTML**: all-time toplists 675,964 B (19 tables, 133 rows/page; marks present in raw HTML, e.g. `9.58`), Olympic results 556,385 B. No `__NEXT_DATA__`, no data global, no `/api/`-shaped URL in the HTML.
- `RESTRICTION (recorded)` — ToS §4.1 bans *"any robot, spider, crawler, scraper, artificial intelligence system, machine learning model, large language model, automated agent, data mining tool, harvesting tool or other automated means to access, collect, copy, monitor, extract, reproduce, analyse, index, store, republish or otherwise use any content, data, materials, trademarks, databases"*; §3.1.1 bans database creation; §4.2 bans AI training use.
- Population is elite/professional + U20 — useful only as a reference/normalization source, not for US HS rosters.

## 8. Bulk and open datasets (verified)

| Dataset | License | Rows/size | Contents | Verdict |
|---|---|---|---|---|
| Kaggle `ferdinanddelgadophd/nys-section-v-100m-high-school-track-20082025` | **CC0** | **82,889 rows verified by download** (zip 905,779 B → CSV 8,050,287 B) | `athlete_id, team_id, graduation_year, sex, meet_name, meet_date, meet_year, meet_month, season, career_year, career_label, time_seconds, timing_type, meet_timing_status, timing_corrected` | **USABLE-WITH-WORK** — anonymized (no names/schools/ids joinable), 100m/outdoor only, but a **CC0 ground-truth corpus for grade + FAT/hand timing normalization** |
| Kaggle `jeannicolasduval/world-athletics-all-time-rankings` | CC0 | ~489k entries / 98.9 MB | `competitor, dob, nat, event, mark, mark_details, wind, venue, date, rank` | USABLE-WITH-WORK (elite only; no id; upstream-scraped provenance) |
| Kaggle `laurenainsleyhaines/paris-2024-…` | CC0 | 1.67 MB | Olympic TF results, `name/bib/country/event/mark/round/pos` | USABLE-WITH-WORK (contest-level) |
| Kaggle `heesoo37/120-years-of-olympic-history-…` | CC0 | 41.5 MB | medals only, no marks; frozen at 2016 | USABLE-WITH-WORK (low value) |
| GitHub `JPcheco/stonehill-tfrrs-scraper` (published CSV) | NO-LICENSE | `performances_normalized.csv` = 6,135,057 B | `athlete_id, Profile Link, Athlete Name, Gender, Meet, Meet Date, Event, Mark/Time, Wind, Conversion, meet_date_raw, meet_start_date, meet_end_date, year, season, season_type` | **schema reference only** (copy the data model, not the code) |
| GitHub reusable code | MIT: `reteps/rundata`, `JonathanOppenheimer/MileSplit-Ranks-Web-Scraper`; GPL-3.0: `nulxn/athletic-data`, `loup-brun/better-web-hytek` | — | HY-TEK/RunData/MileSplit tooling | USABLE under their licenses |
| HuggingFace datasets | — | — | 5 searches (`track and field`, `tfrrs`, `athlete_events`, …) → **no TF/XC corpus exists** | DEAD |
| Zenodo / Figshare / archive.org / data.gov CKAN | — | — | APIs live; 4.4M Zenodo hits are papers; data.gov CKAN endpoint 404s | DEAD |
| `stats.ncaa.org` | — | — | robots `Disallow: /` | BLOCKED (college-only anyway) |
| `opentrack.run` | — | — | robots itself returns **`cf-mitigated: challenge`** („Just a moment…") | BLOCKED without a browser |
| OpenSplitTime API | — | — | `/api/v1/events` → 401 "You need to sign in" (self-hostable OSS; no HS corpus) | RESTRICTED |

**Adversarial checks recorded:** the NYS dataset's description claims 82,898 results vs **82,889 shipped rows** (verified), and description v3 vs API `version 5 / 2026-04-19`. Kaggle's `/api/v1/datasets/view` returns `files: []` unauthenticated, so column lists for the three non-NYS sets come from uploader descriptions, not parsed files (the NYS schema *is* file-verified).

## 9. State associations, NCAA, and exclusions

- `mhsaa.com` — archive of 595 on-site files, but **finals-only `.txt` recaps** (top-6/8, no wind/grade/id) and **79 links point to athletic.net**.
- `ihsaa.org` — zero results in its 2,255-URL sitemap; Player Stats is an empty MaxPreps widget; tournament data is a JS-gated EventLink app.
- `ohsaa.org` — state-finals PDFs, heat sheets, divisional-alignment PDFs (a usable school ↔ OHSAA-id map); per-meet data delegated to athletic.net / live.athletic.net.
- `athletictiming.rsupartner.com` — RunSignup portal, road races only (wrong population).
- `www.athletictiming.net` (Wix) → live.athletictiming.net (AthleticLIVE tenant) → excluded.
- `www.tfmeetpro.com` — vendor site for DirectAthletics' MeetPro app (nothing to ingest; explains the HY-TEK-shaped outputs across the ecosystem).
- `athletic.live` — 6/6 probes 302 → `live.athletic.net`; `results.athletic.live` NXDOMAIN. Pure alias of the excluded host.

## 10. Cross-cutting findings

1. **One JSON schema can carry the primary collection.** MileSplit's `performances` response already contains every per-row column `SCOPE.md` requires (athlete id, name, gender, grad year, event, round, heat, mark, place, wind, profile URL) — verified live, 420 rows.
2. **HY-TEK `<pre>` text is the interchange format of HS track.** The same Meet-Manager layout appears on MileSplit `/raw`, baumspage, finishtimingresults (and as PDFs at CIF-SS). **One parser unlocks four hosts.**
3. **Identity is the hard problem, not access.** Only MileSplit (`athleteId`), TFRRS (college) and MaxPreps (`careerId`) give stable ids; every HY-TEK/regional host is name+school(+grade). Cross-host joins will be probabilistic.
4. **The ecosystem is consolidated.** TFRRS, DirectAthletics, MileSplit and RunnerSpace terms are all FloSports/DirectAthletics-flavoured (TFRRS footer: `Copyright © 2026 DirectAthletics, Inc.`; DA athlete URLs 302 into TFRRS; DA rankings point at TFRRS; MeetPro is DirectAthletics' app).
5. **Almost nobody presents a bot challenge.** ~170 requests across 25+ hosts: no CAPTCHA, no `cf-mitigated`, no WAF block, no 403/429, no `retry-after` (one benign `retry-after: 0` on a 400). The two exceptions found: `opentrack.run` (CF challenge) and `www.thepowerof10.info` (`wafrule: 5`, not probed further). Latency 0.04–1.3 s everywhere else.
6. **Several state portals and timing hosts delegate into athletic.net** (OHSAA, IHSAA, AthleticLIVE tenants, RunnerSpace coach sign-ups) — that delegation is where the excluded host appears.
7. **Paywalls are server-side omission**, not obfuscation: MileSplit PRO history values are absent from the HTML. No bypass attempted, none planned.
8. **Incidental:** MileSplit pages embed a Twitter client token/secret pair in cleartext page source.
9. **`web_search` is currently unusable** at the provider level — discovery must run through platform APIs (as it did here).

## 11. What the pipeline can get, per capability

| Capability | Primary source (verified) | Fallback |
|---|---|---|
| HS meet results, full field set, structured | **MileSplit `/api/v1/meets/{id}/performances`** | `/raw` HY-TEK; MaxPreps stats; elitefeats/baumspage/finishtiming/runnercard/yentiming |
| Grade / junior evidence | **MileSplit `gradYear`** (420/420 rows) | MaxPreps `classYear`; `/raw` grade column; CIF-SS PDF grade; TFRRS HS `YEAR` = graduation year |
| Stable athlete id + profile | **MileSplit `athleteId` + `profileUrl`** | TFRRS (college); DirectAthletics search ids; MaxPreps `careerId` |
| Full performance history per athlete | **TFRRS athlete page (one request)** | MaxPreps season stats; MileSplit PR table (history is PRO-masked) |
| Wind / round / timing type | **MaxPreps stat tables** (wind + `Electronic` + round) | MileSplit JSON `windReading`/`round`; TFRRS event pages; HY-TEK `Wind` columns |
| Timing normalization (FAT vs hand, +0.24 s) | **Kaggle NYS corpus (CC0, 82,889 rows, `timing_type`/`timing_corrected`)** | `meet_timing_status` columns; HY-TEK `Electronic` marker |
| Meet calendar / enumeration | **MileSplit `/results`**, DA `fuseDriver_Upcoming.js`, TFRRS `/results_search.html` | MaxPreps sitemaps |
| National rankings by grade/season | **MileSplit `/rankings/leaders/…?grade=junior`** — rows are XHR; endpoint TBD | TFRRS division lists (college) |
| Regional/independent coverage | yentiming, baumspage, finishtimingresults, runnercard, elitefeats, CIF-SS | RunnerSpace per-meet subdomains |
| Elite reference / normalization | World Athletics records pages | Kaggle CC0 Olympic/all-time sets |

## 12. Unverified / open items

1. **MileSplit rankings XHR endpoint** — page exposes `?year=&accuracy=legal&grade=<junior>`; rows arrive client-side; `/api/v1/athlete-rankings` returned 501 with guessed params. Extract the real call from the rankings bundles (bounded, 2–3 requests).
2. MileSplit: whether every meet exposes `/raw`; `isMeetPro` meets; whether a PRO session returns history (not attempted).
3. TFRRS: `tf.`/`xc.`/`florida.` subdomains; `/outdoor_lists.html`; directory fragments; `/top_performances` bodies; `director_info.html` terms; jumps/throws wind format; HS athlete profiles (none observed).
4. DirectAthletics: HS `Year` values (only a college meet sampled); track result pages (wind/round); `/view_list.html`; team rosters.
5. MaxPreps: XC stat tables; leaderboard endpoint.
6. elitefeats: `YR` population; per-meet page-shape uniformity. runnercard: `Mark` column population on past seasons.
7. `finishedresults`/`trackscoreboard` API route shapes and id stability.
8. Kaggle: whether unauthenticated download works for all datasets or only the one observed (the documented path needs `kaggle.json`); terms text unread.
9. Yen Timing: ToS text (none located); full event coverage beyond the leaderboard index.
10. flosports.tv ToS and MaxPreps terms — linked everywhere, read nowhere (record-keeping only under the new policy).
11. Other state sections' timing hosts were not enumerated.

## 13. Request ledger (all probes 2026-09-20)

| Host | Requests | Notes |
|---|---|---|
| `www.milesplit.com` / `ny.` / `js.sp.` / `milesplit.live` | 25 + 3 + 2 (main) | includes the verified API probe |
| `www.tfrrs.org` | 11 + 2 (main) | |
| `www.directathletics.com` | 13 + 2 (main) | |
| `www.maxpreps.com` | 11 | |
| `cifss.org` / `cifsshome.org` | 6 / 1 | |
| `mhsaa.com` / `ihsaa.org` / `ihsaa.eventlink.com` / `ohsaa.org` / blob | 12 / 6 / 4 / 5 / 2 | |
| `baumspage.com` | 16 + 10 | |
| `runnerspace.com` (+ subdomains) | 11 | |
| `leonetiming.com` (www/results) | 6 | |
| `finishedresults.com` / `trackscoreboard` / api | 5 / 3 / 2 | one API GET before its robots was seen — disclosed |
| `finishtimingresults.com` / `live.finishtiming.com` | 4 / 1 | |
| `runnercard.com` / `elitefeats.com` / `tfmeetpro.com` | 10 / 11 / 5 | |
| `athletictiming.net` / `live.athletictiming.net` / `…rsupartner.com` | 4 / 4 / 2 | rsupartner results fetched before its robots was seen — disclosed |
| `www.kaggle.com` (+ GCS object) | ~16 + 1 | over the ~10 guide; includes the verified download |
| `api.github.com` / `github.com` / `raw.githubusercontent.com` | 11 + 1 + ~12 | core API limit 60/hour observed |
| `huggingface.co` | 10 | |
| `worldathletics.org` / `stats.ncaa.org` / `opentrack.run` | 4 / 1 / 1 | |
| `zenodo.org` / `api.figshare.com` / `catalog.data.gov` / `archive.org` | 2 / 2 / 2 / 1 | |
| `www.opensplittime.org` / `www.olympedia.org` / `www.usatf.org` / `www.thepowerof10.info` / `www.yentiming.com` | 2 / 2 / 1 / 1 / 3 | |
| `athletic.live` | 6 | all 302 into the excluded host; not followed |
| **athletic.net** | **0** | excluded by owner instruction |

All requests: one-shot, sequential, no retries, no concurrency, bounded `--max-time`, plainly-identifying UA, robots fetched first on every host. Over-budget and order-of-operations disclosures are noted inline.

## 14. Collection design direction (≤2 RPS/host)

**Shape:** one durable collector per host behind a shared **per-host token bucket** (500 ms spacing, capacity 1–2) plus a small global worker pool (≤8 hosts concurrently). A single `HostPolicy` record per host carries `rps_ceiling: 2`, `min_spacing_ms: 500`, `max_inflight: 1`, `etag_cache`, `backoff: 1s→2s→4s→…→60s on 429/5xx`, `cooldown_until`, and a request budget written to the run log.

**Priority order (value per request):**

1. **MileSplit `performances` API** — 1 request = one complete meet file (420 rows, all target columns). ≈3 requests/meet.
2. **TFRRS athlete page** — 1 request = an entire career with wind and round.
3. **MaxPreps stat page** — 1 request = every season's per-meet rows with wind/timer/round.
4. Regional HY-TEK hosts (yentiming, baumspage, finishtiming, runnercard, elitefeats) — 1 request per meet file; name-based identity.
5. CIF-SS PDFs (postseason only); World Athletics (reference).

**Politeness multipliers:** conditional GETs (`If-None-Match`) on hosts that emit `ETag` (TFRRS, MileSplit meet pages); whole-document fetches over per-row fetches (one TFRRS athlete page instead of ~90 event pages); ETag-driven incremental runs; and a `--budget` dry-run that prints projected request counts per host for a season before any fetch.

**Bulk lane (different shape):** dataset ingestion is a one-shot download + file parse (no browser, no origin config) — e.g. the CC0 NYS corpus via Kaggle's signed-URL flow. It feeds the **timing/grade normalization** tables rather than the roster.

**Requests flow through the existing admission/gateway layer**; the only new activity types are (a) JSON-API ingest, (b) fixed-width HY-TEK text ingest, (c) PDF text extraction, (d) one-shot dataset download.

**Open choices to settle with the owner:** (a) MileSplit API vs `/raw` HTML as the HS primary; (b) whether DirectAthletics ids are worth a separate identity-resolution pass; (c) whether rankings are Phase 1 (needs §12.1); (d) whether `opentrack.run` (CF challenge) or `live.athletic.net` get a headed-browser path after all.
