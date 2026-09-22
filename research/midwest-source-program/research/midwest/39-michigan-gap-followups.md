# 39. Michigan follow-ups — MHSAA UP girls finals naming, MITS indoor archive, MHSAA→Athletic.net link inventory

Status: complete — all three assignments answered from live observation; nothing was left unobserved.
Every claim below carries the URL and status code that produced it (see Evidence appendix); anything
not directly observed is marked `[INFERENCE]`.

Observed on: 2026-09-20 (America/Chicago). All probing done from this workstation with a Chrome 140
browser UA, ≥1.2 s spacing per host, no *.athletic.net request of any kind (the probe tool hard-refuses
those hosts), no auth/CAPTCHA/paywall bypass.

This report closes the open items of reports 15 (`15-michigan-mhsaa.md`) and 16
(`16-michigan-alternatives.md`):

1. **MHSAA UP girls finals D1/D2/D3** — official page names + URLs, six seasons deep, with grade counts.
2. **MITS indoor archive** — what is public on michianatiming.com, the AthleticLIVE tenants
   (`michiana_meet_list`), trackmeet-io Firebase and the blob store, and whether grades appear.
3. **MHSAA regional/state pages → Athletic.net link inventory** — how many link directly, with URL patterns.

## Source

| Surface | Host / pattern | What it is |
|---|---|---|
| MHSAA (Michigan High School Athletic Association) | `https://www.mhsaa.com` (Drupal, Fastly CDN) | Official state association: sport hubs, per-season regional-result pages, results archive, UP finals artifacts |
| AthleticLIVE search cluster | `https://search.athletic.live/<tenant>_meet_list/_search` | Anonymous ElasticSearch over 256 tenant meet indices (campaign census, see report 12); **this is the MITS meet index** |
| trackmeet-io Firebase | `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_<i>/event_summary.json?ns=trackmeet-io` | Per-meet event index (event ids + abbreviations) |
| AthleticLIVE blob store | `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<EventID>` | Whole event (all rows) as JSON, public GET, ETag/Last-Modified, 304 supported |
| Tenant white-labels (michiana) | `https://fatresults.com`, `https://live.michianatiming.com` | AthleticLIVE SPA shells for the `michiana` tenant; the SPA asset map inside the shell contains `live.michianatiming.com → sites/michiana` |
| Michiana Timing WordPress | `https://michianatiming.com` | News/registration/archive-of-post-titles site (not a results store) |
| MITCA | `https://mitca.org` | Coach association; **no** indoor/MITS content |
| MileSplit Michigan | `https://mi.milesplit.com` | Public indoor calendar; athlete profiles (Class of, PRs); result rows PRO-gated |

Tenant short-host map observed inside blob/Firebase data (recorded as identifiers only — never fetched):
`fst.anet.live` = fstiming (GVSU/SVSU meets), `cht.anet.live` = chttiming (CMU), `dl.anet.live` =
dlprotiming (MITS/MITCA state champs), `lab.anet.live` = labtiming, `fatresults.com` = michiana
(Aquinas College meets), `anet.live` = shared default host (Hillsdale Charger MITS meets).

## Coverage

- **State**: Michigan only (MITS is a Michigan series; the `michiana` tenant also times northern-Indiana
  meets, but every MITS meet on it is Michigan).
- **Sports**: boys/girls outdoor TF, boys/girls XC, boys/girls indoor TF.
- **Seasons / historical depth**
  - MHSAA outdoor **TF**: archive year sections **2016, 2017, 2018, 2019, 2021, 2022, 2023, 2024, 2025, 2026**,
    with a **2020** section that MHSAA itself labels *"2020 tournament cancelled due to COVID-19"*; older
    sections additionally link "Qualifiers" entry lists (36 legacy `Print/EntryMeet` ids).
  - MHSAA **XC**: archive carries 2015-2025 finals artifacts (naming varies per year); live regional
    pages exist for **2023, 2024, 2025, 2026** (2026 pre-season).
  - **UP finals (girls, TF)**: artifacts verified for **2021, 2022, 2023, 2024, 2025, 2026**.
  - **MITS indoor**: michiana tenant index holds **20 MITS meets, 2019-12-14 → 2026-02-14**;
    the cross-tenant `live_results_meet_list` holds **66 MITS-named meets, 2019 → 2026-02-27**;
    the complete current season is **17 meets (2025-12-06 → 2026-02-27)**.
- **School levels**: MHSAA varsity regionals/finals + a separate JH/MS regional block on the same pages;
  MITS has a high-school varsity division plus open/unattached/club entrants (unattached and club rows
  are the majority on AQ/GVSU meets).
- **Historical blob depth**: an event from **2019-12-14** is still served (200, `Last-Modified:
  Wed, 21 Aug 2024`), i.e. the AthleticLIVE store keeps the whole MITS run since the 2019-20 season.

## Enumeration

### 1. MHSAA UP girls finals (D1/D2/D3) — official page names

There is **no dedicated UP page per gender**; the official artifacts hang off the year section of the
results-archive page, and both the boys and the girls archive link the *same* girls files. Verified URLs
(`sites/default/files/...` under `www.mhsaa.com`), all HTTP 200:

| Season | D1 | D2 | D3 | Format / grades |
|---|---|---|---|---|
| 2026 | `Track%20Field-Boys/2026/Finals/2026-UP-Girls-D1-Finals.pdf` | `...2026-UP-Girls-D2-Finals.pdf` | `...2026-UP-Girls-D3-Finals.pdf` | Hy-Tek PDF, `Yr` column populated (D2: 25 × G11; D3: 45 × G11) |
| 2025 | `Track%20Field-Boys/2025/Finals/UP-D1-Girls.pdf` | `.../Up-D2-Girls.pdf` | `.../Up-D3-Girls.pdf` | Hy-Tek PDF, `Yr` populated (D2: 17 × G11). **Case matters**: D2/D3 are `Up-`, D1 is `UP-`; `UP-D2-Girls.pdf` = 404 |
| 2024 | `Track%20Field-Boys/2024/Finals/UP-D1-Girls.pdf` | `.../UP-D2-Girls.pdf` | `.../UP-D3-Girls.pdf` | Hy-Tek PDF, `Yr` populated (D2: 40 × G11) |
| 2023 | `Track%20Field-Boys/2023/UP%20D1Girls.htm` | `.../UP%20D2Girls.htm` | `.../UP%20D3Girls.htm` | Hy-Tek HTML; header says `Name Year School` but the **year column is empty** (0 grade values) |
| 2022 | `2022-07/22girlstfupd1finals.txt` | `...22girlstfupd2finals.txt` | `...22girlstfupd3finals.txt` | Hy-Tek text; year column present but empty (0 grade values) |
| 2021 | `2022-07/21trackupd1girlsfinal.txt` | `...21trackupd2girlsfinal.txt` | `...21trackupd3girlsfinal.txt` | Hy-Tek text (same layout as 2022) |

Grade counts use one explicit method: every line beginning with a place number whose next standalone
token is 7-12 immediately followed by a school token (Python token scan over `pdftotext -layout`
output). Row counts therefore are event-entry rows, not de-duplicated athletes (a multi-event athlete
appears once per event).

**Where they are linked**: `https://www.mhsaa.com/sports/{boys|girls}-track-field/results-archive` →
year section (`2026 Tournament`, `2025 Upper Peninsula`, `2024 Tournament`, `2023 Finals - June 3`,
`2022 Finals - June 4`, …). The 2026 files are additionally linked from both TF **hubs**; 2021-2025
files only from the two archive pages. The girls hub itself never links them (report 15's observation
stands) — the archive does.

**UP boys** use the same year+spelling scheme with `Boys` in the file name; the 2025 archive section links
`2025/Finals/UP-D1-Boys.pdf`, `.../UP-D2-Boys.pdf`, `.../Up-D3-Boys.pdf` (the case flip lands on a
different division than for the girls files), and the 2026 boys D2 file
(`2026-UP-Boys-D2-Finals.pdf`) was probed 200. **UP XC girls** naming per season (link evidence from
the XC archives; 2020/2022/2023 probed 200): `2022-07/20upxcd<N>girls_0.pdf` (2020),
`Cross%20Country-Boys/2022/Finals/2022%20UP%20D<N>%20Girls.pdf` (2022),
`Cross%20Country-Girls/2023/UP%20Girls%20D<N>.pdf` (2023), `2022-07/21upgd<N>.pdf` (2021).

**UP finals AN meet ids** (from the hub/archive links): 2026 = **622974** (one meet for all UP divisions,
boys+girls), 2025 = **571117**. 2024 and 2023 UP finals have **no** Athletic.net link on the archive page —
the UP artifact set there is PDFs only.

### 2. MITS — exact navigation

```
# meet discovery (anonymous POST, no key)
POST https://search.athletic.live/live_results_meet_list/_search
{"size":50,"track_total_hits":true,"_source":["i","ani","n","md","o","us"],
 "query":{"bool":{"must":[{"bool":{"should":[{"match_phrase":{"n":"MITS"}},
                                            {"match_phrase":{"n":"Indoor Track Series"}}],
                                       "minimum_should_match":1}}],
                  "filter":[{"range":{"md":{"gte":"2025-12-01"}}}]}},
 "sort":[{"md":"asc"}]}
#   i   = AthleticLIVE meet id        ani = Athletic.net MeetID (nullable)
#   n   = meet name                   md  = meet date
#   us  = tenant result short-URL     o   = tenant/other short-URL

# per-tenant view of the same catalog
POST https://search.athletic.live/michiana_meet_list/_search     # tenant MITS view (tenant index holds 1,995 docs per the campaign census)

# event index for a meet
GET  https://s-gke-usc1-nssi3-33.firebaseio.com/meet_<i>/event_summary.json?ns=trackmeet-io
#   -> {"Individual-2254285": {"abn": "B 1600m", ...}, "Relay-...": {...}}

# whole event (all rows)
GET  https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<EventID>
```

Row/athlete fields present in the blob document (verified on 2026 events):
`r[].i` result id, `r[].p` place, `r[].hn`/`r[].hl` heat/lane, `r[].ro` round, `r[].m` mark, `r[].im`
integer mark, `r[].co` metric conversion, `r[].w` wind, `r[].pt` points, `r[].er`/`ern` season-best/PR
flags, `r[].hs[]` field-event attempt series, `r[].a.{i,n,fn,l,y,g,ani}` athlete id/name/**grade**/gender/
**Athletic.net athlete id**, `r[].a.t.{i,n,ani}` team/club with **Athletic.net team id**. Event doc
fields include `i`, `mi` (meet id), `n`/`sn`/`abn`, `ec`/`g`/`gl` (category/gender), `runm`/`rui` (round),
`nrds` (row count), `ua` (a timestamp — appears to be the last update; `[INFERENCE]`).

### 3. MileSplit MI indoor (public surface only)

`https://mi.milesplit.com/calendar?season=indoor&year=2025|2026&month=12|1|2` yields the indoor calendar
rows with `data-meet-id`; the extraction taken for this report holds **37 indoor meets Dec 2025 - Feb 2026,
17 of them MITS-named**, e.g. `647922` → `https://www.milesplit.com/meets/647922-gvsu-mits-1-2025`.
`https://mi.milesplit.com/results?season=indoor` (200, "Michigan Indoor Race Results") is a JS shell:
0 result rows in HTML, 27 `PRO` markers. A MITS state-championship meet page returns a meet-manager
shell; the entries page exposes only JSON-LD `SportsEvent` metadata (name/date/location) with entries
locked. So MileSplit is usable for **discovery of Michigan indoor meet ids/names/dates**, not for rows.

### 4. Non-routes (recorded so they are not retried)

- `michianatiming.com/results/results-archive/` (200, 206,603 B) = a list of post titles, and a sample
  MITS post body (`/january-22-aq-mits-meet/`, 200) is an **entrant/bib list** for a 2022 meet — no marks,
  no grades. It is a news/registration archive, not a results store.
- `mitca.org/MITCA/` (200) contains 0 occurrences of `MITS` or `Indoor` — MITCA publishes awards and
  team-state pages, not indoor results.
- No official MITS site could be found: `mitsindoor.com`, `michiganindoortrackseries.com`, `mits.run`,
  `mitsmichigan.com` all fail DNS (no A record). `[INFERENCE]` the series lives only on the AthleticLIVE
  stack + Athletic.net + MileSplit calendar.

## Stable identifiers

| Entity | Field / pattern | Example | Notes |
|---|---|---|---|
| MHSAA meet → AN MeetID | `athletic.net/TrackAndField/meet/<MeetID>/...` inside MHSAA HTML | 622922 (2026 regional 1) | Extractable with no AN request |
| MHSAA legacy AN link | `athletic.net/TrackAndField/Print/EntryMeet.aspx?Meet=<MeetID>&t=<token>&show=all` | 481008 | "Qualifiers" entry lists; token is MHSAA's own published artifact |
| MHSAA meet grouping | region number + host school | `Regional 1 Mt Pleasant HS` | Region numbers are stable per season and disambiguate divisions |
| MHSAA file artifacts | `/sites/default/files/Track%20Field-Boys/<year>/...` | UP finals PDFs | Not content-addressed; `?time=<epoch-ms>` cache-buster appears in archive hrefs |
| MITS meet | AthleticLIVE `i` (int) | 61819 | Monotonic-ish, tenant-scoped |
| MITS meet → AN MeetID | `ani` on the same ES doc | 621555 | Present for all 17 meets of 2025-26; absent for michiana 2019/2022/2023 rows |
| MITS event | event id = blob doc key (`Individual-<id>` / `Relay-<id>` in Firebase) | 2254285 | Same integer as ES/AthleticLIVE event id |
| MITS result | `r[].i` | 35006768 | Stable per row |
| MITS athlete | AthleticLIVE `a.i` **and** Athletic.net `a.ani` | 44385233 / 18064122 | Two ids on every 2026 row |
| MITS team | `a.t.i` / `a.t.ani` | 1477365 / 35189 | AN team id available |
| MITS tenant | `us` / `o` short URL host | `http://fst.anet.live/qjp44f` | Identifies the timing tenant; recorded, never fetched |
| MileSplit meet | numeric id in `data-meet-id` | 647922 | Public on the calendar |
| MileSplit athlete | numeric id in `/athletes/<id>-<slug>` | 11141513 | **Different namespace from Athletic.net athlete ids** — AN athlete id `17984073` resolves on MileSplit to a Florida athlete (Class of 2030), and `27105091-...` is a 404 |

Region numbers, meet names and school names are **not** identifiers: MHSAA region numbering restarts each
season and divisions are enumerated within the page; use the AN MeetID or `(sport, year, region N)` pairs.

## Athletic.net leverage

**MHSAA HTML is a direct AN-meet-link source (link extraction only; no AN request was made).**

| Page family | Pages probed | athletic.net hrefs | Distinct AN meet ids | URL pattern |
|---|---|---|---|---|
| TF hub (boys, girls) | 2 | 13 each | 5 each (2026 finals D1 622966, D2 622969, D3 622972, D4 622973, UP 622974; 3 href forms each) | `/sports/{boys|girls}-track-field` |
| TF regional page | 4 (2023, 2024, 2025, 2026) | 48 / 48 / 49 / 49 | 48 / 48 / 49 / 49 | `/sports/{boys|girls}-track-field/{YYYY}-mhsaa-track-field-regional-results` (2023 = `…-regional-info-and-results`) |
| TF results archive | 2 | 79 / 78 | 52 each (20 `/meet/` + 36 `Print/EntryMeet` ids) | `/sports/{boys|girls}-track-field/results-archive` |
| XC hub | 2 | 2 each | 0 (only the middle-school index link) | `/sports/{boys|girls}-cross-country` |
| XC regional page | 4 (2023-2026) | 36 / 37 / 37 / 1 | 36 / 37 / 36 / 0 (2025 has one meet id linked twice) | `/sports/boys-cross-country/{YYYY}-mhsaa-cross-country-regional-info-and-results` (2026 = `2026-mhsaa-lp-cross-country-regional-info-and-results`) |
| XC archive | 2 | 14 / 14 | 8 each | `/sports/{boys|girls}-cross-country/results-archive` |

Union across the 16 captured pages: **215 distinct TF meet ids, 117 distinct XC meet ids, 36 legacy
`Meet=<id>` ids** (4 ids appear in both forms). The **cross-country 2026 page currently has zero regional
links** (37 on the 2025 page) — the results links are added per regional as meets are contested, so the
page must be polled during the season.

URL patterns to extract (all five observed):
`https://www.athletic.net/TrackAndField/meet/<id>/results` · `/results/all` · `/teamscores` ·
`https://www.athletic.net/CrossCountry/meet/<id>/results/all` ·
`https://www.athletic.net/TrackAndField/Print/EntryMeet.aspx?Meet=<id>&t=<token>` (legacy, 36 ids that
do not appear in `/meet/` form) · plus non-meet reference links (`/TrackAndField/Michigan/`,
`/CrossCountry/State/Archive.aspx?State=<id>`, `/cross-country/usa/middle-school/michigan`).

**MITS is a second, independent AN-id source**: every one of the **17 meets of the 2025-26 season** carries
both an AthleticLIVE meet id and an **AN MeetID**:

| Date | AL meet id `i` | AN MeetID `ani` | Name | Tenant host |
|---|---|---|---|---|
| 2025-12-06 | 59467 | 621518 | GVSU MITS #1 | fst.anet.live |
| 2025-12-13 | 59710 | 621519 | GVSU MITS #2 | fst.anet.live |
| 2025-12-13 | 59759 | 624916 | AQ MITS #1 | fatresults.com |
| 2026-01-10 | 60088 | 618891 | MITS #1 @ SVSU | fst.anet.live |
| 2026-01-10 | 60156 | 621520 | GVSU MITS #3 | fst.anet.live |
| 2026-01-10 | 60228 | 627657 | Hillsdale Charger Winter Open 1- MITS | anet.live |
| 2026-01-17 | 60348 | 618892 | SVSU - MITS #2 | fst.anet.live |
| 2026-01-17 | 60442 | 626526 | AQ MITS #2 | fatresults.com |
| 2026-01-24 | 60490 | 618894 | SVSU - MITS #3 | fst.anet.live |
| 2026-01-24 | 60543 | 621554 | GVSU MITS #4 | fst.anet.live |
| 2026-01-24 | 60707 | 627664 | Hillsdale Charger Winter Open 2- MITS | anet.live |
| 2026-01-31 | 60961 | 631644 | AQ MITS #3 | fatresults.com |
| 2026-02-08 | 61242 | 630073 | CMU MITS Meet | cht.anet.live |
| 2026-02-14 | 61710 | 634419 | MITS 4 | fatresults.com |
| 2026-02-21 | 61819 | 621555 | GVSU MITS #5 | fst.anet.live |
| 2026-02-21 | 61931 | 628710 | SVSU - MITS #4 | fst.anet.live |
| 2026-02-27 | 62107 | 619960 | MITS/MITCA Indoor TF Champs | dl.anet.live |

Request avoidance: the AN MeetID for each of these 17 meets, plus the athlete-level AN ids and the
per-row grade, all come from non-AN hosts. Under the pipeline's own historical cost model (report 15/16:
1-2 AN requests per meet just to *find/validate* it), that removes the discovery requests for the whole
Michigan indoor season, and the blob rows can replace AN result reads for meets whose grades are populated.
`[INFERENCE]` the exact request savings depend on how many of the 17 meets the pipeline would otherwise
acquire from AN; the count of ids obtained without AN is exact (17 meets / 20 tenant meets / 66 docs).

## Athlete evidence

| Field | MHSAA UP girls finals (2024-2026 PDFs) | MITS blob rows (2026 events) | MileSplit MI profile |
|---|---|---|---|
| name | yes (`Last, First` or `First Last` per season format) | yes (`a.n`, `fn`, `l`) | yes |
| graduating class / grade | **yes (2024-2026)**; absent 2021-2023 | `a.y` — 100% on GVSU MITS #5 + MITS 4 + 2025 CMU MITS; partial on AQ MITS #2 2026 (~22-31%); absent ≤2025-12-13 michiana events | profile shows `Class of <year>` (free); rosters show `Class` column |
| school | yes (team name per row / team scores block) | yes (`a.t.n`, `a.t.ani`) — club or `Unattached` for many indoor entries | yes |
| city/state | school only | `a.cco`, `a.cm`, team `cco` = country/community id | yes |
| gender / category | one document per gender+division | `g`/`gl` on the event, `a.g` on the athlete | yes |
| TF/XC + indoor/outdoor | outdoor finals only | indoor season | calendar separates indoor |
| performances | finals marks, place, points, wind for sprints | marks (`m`), conversions (`co`), wind (`w`), attempts series (`hs[]`) | PR table free |
| progression / PRs | no | `er`/`ern` = season best / PR flags per row | PR table free; full results PRO |
| meets | one meet per document | meet via `mi`/`o`/`us` | meet pages |
| profile URL | n/a (documents) | none (no athlete page on the blob route) | `/athletes/<id>-<slug>` |

## Recruiting information

Not applicable / not observable on these routes. The MHSAA UP finals PDFs, the MITS blob documents and the
AthleticLIVE tenant SPAs contain **no coach, AD or athletic-department contact fields**. The only Michigan
coach-adjacent site touched was `mitca.org` (association pages: awards, team-state, power rankings — read
200, no directory on the pages fetched); report 29 owns the coach-contact graph. Per the mission privacy
contract, no athlete contact data was collected (and none is exposed).

## Result evidence

MHSAA UP finals: ResultID no · AthleteID no · MeetID via the sibling AN link (2025/2026 only) · event name
in the document · mark, place, points, wind (Hy-Tek) · timing method not stated in the file · implement/
hurdle spec not stated · round = `Finals` · date in the document header · school per row · relay membership
via relay teams/legs.

MITS blob rows: ResultID **yes** (`r[].i`) · AthleteID **yes** (AL `a.i` + AN `a.ani`) · MeetID **yes**
(`mi` + AN `ani` on the meet doc) · EventID **yes** (doc key) · mark **yes** (`m` + integer `im` + metric
`co`) · normalized mark inputs: `im` is the integer form (e.g. 200m 24.91 → 24902, throws in mm) ·
timing method: not present on the row; event doc carries a `ts`/`mms` tuple (`mms: "RunMeet"`) `[INFERENCE]`
timing-system marker · wind **yes** (`w`) · implement weight: present in the event name (`B Weight Throw`)
but not as a structured field on the sampled rows · heat/round **yes** (`hn`, `hl`, `ro`, `runm`) · place
**yes** (`p`) · date via the meet doc/`md` · school represented via `a.t` (club/`Unattached` indoors) ·
relay membership via `Relay-*` event docs.

## Incremental use

- **MITS new/changed meets** — one ES query per week: `range md >= <watermark>` on
  `live_results_meet_list` (+ the tenant index for the header/canonical row). New meets are exactly the
  docs above the watermark (no full re-fetch). Verified: `md >= 2025-12-01` returns 17 for the season.
- **MITS new/changed results** — for each new/changed meet: `meet_<i>/event_summary.json` (one request)
  lists every EventID, then one conditional GET per event doc. Unchanged docs return **304 with 0 bytes**
  (verified with `If-None-Match: 0x8DE6C38AB71852E` on doc 2254285); changed docs carry a fresh
  `Last-Modified`/ETag. Firebase itself does **not** support ETag/HEAD revalidation (HEAD = 405; a bogus
  `If-None-Match` still returns 200), so use the blob ETag, not Firebase.
- **MHSAA** — archive/hub/regional pages are `cache-control: max-age=3600, public` with `Last-Modified`
  and no ETag: re-fetch at most hourly, hash the page, and only parse on change. Detect new season results
  by counting AN links per region row (2026 XC page is the pre-season state: 36 region rows, 0 result
  links; 2025 page: 36 distinct result-meet links across 37 hrefs). Archive pages grow new year sections (and UP PDFs within a season),
  so a re-read of the archive page each week is the update channel; PDFs carry a `?time=` cache-buster
  that changes when a file is replaced.
- **MileSplit** — the indoor calendar is the only cheap delta surface (month pages + `data-meet-id`); it
  is discovery only, since result rows are JS/PRO-gated.

## Access characteristics

- **MHSAA** — normal HTML + static files. `robots.txt` 200: only `/core/`, `/profiles/`, `/README.md`
  disallowed. No auth, no rate-limit headers, Fastly edge cache with hourly TTL. Politeness note: the
  shared agent-39 probe log holds **59 MHSAA requests across both sessions (27 in this resumed run)** —
  slightly above the brief's ~50/host aim because the four-year × two-gender page matrix was fetched to
  settle the regional-page naming question; probing stopped there and no further MHSAA requests are needed.
- **search.athletic.live** — documented-ish public ElasticSearch: anonymous `POST /<tenant>_meet_list/_search`
  returns 200 with no key, 256 tenant indices, `track_total_hits` supported. (Report 12 owns the index census.)
- **AthleticLIVE blob** — public GET per doc; container listing disabled (report 15/16); ETag +
  `Last-Modified` + 304 confirmed here.
- **Tenants (fatresults.com, live.michianatiming.com)** — AthleticLIVE SPA shells (50,184 B, no rows).
  `live.michianatiming.com/robots.txt` (200, 192 B) disallows `/admin*`, `/meets/*/athletes*`,
  `/meets/*/live*`, `/meets/*/teams*`, `/meets/*/follow*` and publishes a sitemap: a collector must not
  use those tenant paths — the ES/blob routes above are unaffected and carry no such policy.
- **michianatiming.com** — WordPress, ordinary HTML, news posts + registration pages; no structured
  results archive.
- **MileSplit MI** — normal HTML for calendar/rosters/profiles; **browser application + subscription-
  restricted** for result rows and entries (PRO markers), with `/api/` robots-disallowed (report 16/27).
- **mitca.org** — plain HTML, no indoor content. **upx/MITindoor domain guesses** — unavailable (DNS).

## Recommendation

**ATHLETIC.NET-SEED** for the Michigan postseason, with two supporting roles:

- **MHSAA (PRIMARY seeding)** — ≈53 TF AN meet ids/season (48 regionals + 5 finals, one link per meet,
  both genders) + 36-37 XC regional ids, each with region number, host school and division context, plus
  the whole 2016-2026 archive (215 TF / 117 XC distinct ids harvested from 16 pages). No AN request is
  needed to obtain or trust an id. Marginal coverage is state-postseason-only, but it is complete for
  Michigan and it also carries the *unlinked* UP girls finals artifacts (a grade oracle, see below).
- **MITS indoor (RESULT-SOURCE)** — 17 meets of the 2025-26 season with AN MeetIDs, full row-level data
  from the blob (grade + AN athlete id + AN team id on modern meets). Grade population is the caveat:
  100% on GVSU/SVSU/CMU/MITS-4-style events, ~22-31% on the January AQ meets, 0% before 2026-02 on the
  michiana tenant. Treat `a.y` as *evidence when present*, never as coverage.
- **UP girls finals PDFs (VALIDATION)** — independent grade source for UP G11 athletes (2024-2026 have
  `Yr`), useful for cross-checking AN-derived grades without an AN request.
- **MileSplit MI indoor — DISCOVERY-ONLY** — calendar with meet ids/names/dates; result rows are not
  a free source.

Biggest gaps: (a) no Michigan indoor source that is *both* deep and grade-complete — before the 2025-26
season the MITS archive is names+marks only, and the January 2026 AQ meets are partially graded; (b) the
MHSAA regional pages are single-season snapshots published under an alternating gender slug, so the URL
must be re-derived from the hub/archive each season (`[INFERENCE]` the slug alternation pattern will
continue; two of four observed seasons used each gender) ; (c) the 2023 regional page and 2026 XC page
show that *absence of links* is time-dependent, not structural.

## Evidence appendix

Every URL queried while producing this report (both agent-39 sessions share one probe log; unique URLs,
method, status, what it proved). DNS failures show status 0. No request was sent to any `*.athletic.net`
host.

| URL | method | HTTP | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.mhsaa.com/sports/boys-track-field` | GET | 200 | 2026 TF hub: Tracking-the-Tournament block links 5 finals meets (D1 622966, D2 622969, D3 622972, D4 622973, UP 622974) in 13 hrefs | 2026-09-20T09:05:35-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/results-archive` | GET | 200 | TF results archive (boys): year sections 2016-2026 (2020 labelled "cancelled due to COVID-19"); 20 /meet/ ids + 36 legacy Print/EntryMeet ids; links every UP girls finals artifact | 2026-09-20T09:05:36-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/results-archive` | GET | 200 | TF results archive (girls): byte-identical link set (same 52 meet ids + same UP girls files) | 2026-09-20T09:05:40-0500 |
| `https://www.mhsaa.com/sports/girls-track-field` | GET | 200 | Girls TF hub: same 5 finals meet ids; its "2026 Regional Results" link points at the boys-* regional page | 2026-09-20T09:05:41-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2026-mhsaa-track-field-regional-results` | GET | 200 | 2026 TF regional page: 48 HS regionals + 1 JH/MS zone link = 49 athletic.net hrefs; one Results link per regional | 2026-09-20T09:05:59-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/2026-mhsaa-track-field-regional-results` | GET | 404 | 404 — confirms 2026 regional page is published only under boys-* | 2026-09-20T09:06:00-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2025-mhsaa-track-field-regional-results` | GET | 404 | 404 — MHSAA publishes one TF regional page per season; 2025 lives under the girls-* slug | 2026-09-20T09:06:01-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/2025-mhsaa-track-field-regional-results` | GET | 200 | 2025 TF regional page (published under girls-* slug): 49 athletic.net hrefs = 48 regionals + 1 JH/MS | 2026-09-20T09:06:03-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2024-mhsaa-track-field-regional-results` | GET | 200 | 2024 TF regional page: 48 athletic.net hrefs | 2026-09-20T09:06:04-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2023-mhsaa-track-field-regional-info-and-results` | GET | 200 | 2023 TF regional page (older slug form): 48 athletic.net hrefs | 2026-09-20T09:06:05-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country` | GET | 200 | XC hub: links only the JH/MS Middle-School index on athletic.net (no meet ids) | 2026-09-20T09:06:10-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country` | GET | 200 | XC hub (girls): same; links the boys-* regional page for 2025/2026 results | 2026-09-20T09:06:11-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2026-mhsaa-lp-cross-country-regional-info-and-results` | GET | 200 | 2026 LP XC page pre-season state: 36 region rows, only 1 athletic.net href (JH/MS index) — regional Results links appear after each regional is contested | 2026-09-20T09:06:12-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2025-mhsaa-cross-country-regional-info-and-results` | GET | 200 | 2025 XC regional page: 37 athletic.net hrefs / 36 distinct XC meet ids | 2026-09-20T09:06:14-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2024-mhsaa-cross-country-regional-info-and-results` | GET | 200 | 2024 XC regional page: 37 athletic.net hrefs | 2026-09-20T09:06:15-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/results-archive` | GET | 200 | XC archive (boys): 8 XC meet ids (2022 section) + UP girls XC finals PDF names for 2015/2016/2020/2021/2022/2023 | 2026-09-20T09:06:22-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country/results-archive` | GET | 200 | XC archive (girls): same UP girls file set (8 XC meet ids) | 2026-09-20T09:06:25-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Girls-D1-Finals.pdf` | HEAD | 200 | UP girls D1 finals 2026 artifact exists (PDF, Hy-Tek) | 2026-09-20T09:06:56-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Girls-D2-Finals.pdf` | HEAD | 200 | UP girls D2 finals 2026 (203,062 B downloaded): 92 graded rows, 25 grade-11 | 2026-09-20T09:06:57-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Girls-D3-Finals.pdf` | HEAD | 200 | UP girls D3 finals 2026 (269,086 B downloaded): 240 graded rows, 45 grade-11 | 2026-09-20T09:06:58-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Boys-D2-Finals.pdf` | HEAD | 200 | Boys sibling naming checked for the same season | 2026-09-20T09:07:00-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2025/Finals/UP-D1-Girls.pdf` | HEAD | 200 | UP girls D1 finals 2025 exists | 2026-09-20T09:07:01-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2025/Finals/Up-D2-Girls.pdf` | HEAD | 200 | UP girls D2 finals 2025 exists under the `Up-` spelling (lowercase p); `UP-D2-Girls.pdf` 404s | 2026-09-20T09:07:02-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2025/Finals/Up-D3-Girls.pdf` | HEAD | 200 | UP girls D3 finals 2025 exists under the `Up-` spelling | 2026-09-20T09:07:03-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2024/Finals/UP-D2-Girls.pdf` | HEAD | 200 | UP girls D2 finals 2024 (581,244 B downloaded): 96 graded rows, 40 grade-11 | 2026-09-20T09:07:05-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2024/Finals/UP-D3-Girls.pdf` | HEAD | 200 | UP girls D3 finals 2024 exists | 2026-09-20T09:07:06-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2023/UP%20D2Girls.htm` | HEAD | 200 | UP girls D2 finals 2023 (22,303 B): has a "Name Year School" header but the year column is empty — no grades | 2026-09-20T09:07:07-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2023/UP%20D3Girls.htm` | HEAD | 200 | UP girls D3 finals 2023 exists | 2026-09-20T09:07:08-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2023-mhsaa-cross-country-regional-info-and-results` | GET | 200 | 2023 XC regional page: 36 athletic.net hrefs | 2026-09-20T09:07:35-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country/2025-mhsaa-cross-country-regional-info-and-results` | GET | 404 | 404 — XC regional page is published once per season (boys-* path) and linked from both hubs | 2026-09-20T09:07:36-0500 |
| `https://search.athletic.live/michiana_meet_list/_search` | POST | 200 | Anonymous POST 200: tenant MITS filter returns 20 docs | 2026-09-20T09:08:03-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_list/61710.json` | GET | 200 | Firebase meet metadata for MITS 4 (2026-02-14) | 2026-09-20T09:08:32-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_61710/event_summary.json` | GET | 200 | MITS 4 (2026-02-14) Firebase event index | 2026-09-20T09:08:34-0500 |
| `http://fatresults.com/` | GET | 200 | AthleticLIVE SPA shell (50,184 B) for tenant michiana; hostname→site-asset map contains "live.michianatiming.com" = sites/michiana | 2026-09-20T09:08:34-0500 |
| `https://michianatiming.com/` | GET | 200 | Michiana Timing home: WordPress news site; links `fatresults.com` (tenant SPA) and `mtresults.com`; 0 embedded result tables | 2026-09-20T09:08:35-0500 |
| `https://search.athletic.live/*_meet_list/_search` | POST | 200 | Anonymous POST 200: cross-tenant MITS filter returns 133 docs across 7 indices | 2026-09-20T09:08:45-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2254285` | GET | 200 | 2026-02-14 MITS 4 B 1600m: 43/43 rows graded (7 grade-11 shown), 43/43 AN ids | 2026-09-20T09:08:54-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2254286` | GET | 200 | 2026-02-14 MITS 4 G 60mH: 21/21 rows graded (4 grade-11), 21/21 AN ids | 2026-09-20T09:08:55-0500 |
| `https://michianatiming.com/results/` | GET | 200 | Michiana Timing results index (13,005 B): four menu links (live-results, results-archive, category) - post titles, no rows | 2026-09-20T09:08:56-0500 |
| `https://michianatiming.com/results/results-archive/` | GET | 200 | Michiana Timing WordPress results/archive index (206,603 B) — post titles only | 2026-09-20T09:08:57-0500 |
| `https://mi.milesplit.com/calendar` | GET | 200 | Indoor calendar source: 37 indoor meets Dec 2025-Feb 2026 with data-meet-id (17 MITS-named) | 2026-09-20T09:09:09-0500 |
| `https://michianatiming.com/january-22-aq-mits-meet/` | GET | 200 | AQ MITS post body = entrant/bib list; no marks, no grades, no embedded results tables | 2026-09-20T09:09:32-0500 |
| `https://michianatiming.com/results/live-results-track-and-field/` | GET | 200 | "Live Results: Track and Field" page: points at `fatresults.com`; no structured results on the WordPress host | 2026-09-20T09:09:33-0500 |
| `https://mi.milesplit.com/meets/722191-mitsmitca-indoor-tf-state-championships-2026/results` | GET | 200 | Results page: meet-manager shell; result rows not public | 2026-09-20T09:09:43-0500 |
| `https://mi.milesplit.com/meets/722191-mitsmitca-indoor-tf-state-championships-2026/entries` | GET | 200 | Entries page: JSON-LD SportsEvent only; entries locked (PRO) | 2026-09-20T09:09:44-0500 |
| `https://mi.milesplit.com/athletes/17984073-david-castrejon` | GET | 200 | AN athlete id 17984073 resolves to a different MileSplit athlete (Quantravious Broussard, Class of 2030, FL) — id namespaces differ | 2026-09-20T09:10:00-0500 |
| `https://mi.milesplit.com/athletes/27105091-olivia-hawkins` | GET | 404 | 404 — MileSplit ids are not AN athlete ids | 2026-09-20T09:10:01-0500 |
| `https://mi.milesplit.com/results` | GET | 200 | `/results` without the season filter - same JS shell behaviour as `?season=indoor` | 2026-09-20T09:10:12-0500 |
| `https://mi.milesplit.com/athletes/11141513-luka-hammond` | GET | 200 | Public profile: Class of <year> + PR table (marks/dates) free; full result history PRO | 2026-09-20T09:10:13-0500 |
| `https://fatresults.com/assets/sites/michiana/config.json` | HEAD | 200 | 200 — confirms fatresults.com is the white-label of the michiana tenant | 2026-09-20T09:10:28-0500 |
| `https://mitsindoor.com/` | HEAD | 0 | DNS failure (no A record) | 2026-09-20T09:10:28-0500 |
| `https://michiganindoortrackseries.com/` | HEAD | 0 | DNS failure (no A record) | 2026-09-20T09:10:28-0500 |
| `https://mits.run/` | HEAD | 0 | DNS failure (no A record) | 2026-09-20T09:10:28-0500 |
| `https://www.mitsmichigan.com/` | HEAD | 0 | DNS failure (no A record) | 2026-09-20T09:10:28-0500 |
| `https://live.michianatiming.com/robots.txt` | GET | 200 | Tenant robots: Disallow /admin*, /meets/*/athletes*, /meets/*/live*, /meets/*/teams*, /meets/*/follow* | 2026-09-20T09:10:34-0500 |
| `https://live.michianatiming.com/meet-list` | GET | 200 | AthleticLIVE SPA shell (50,184 B) — no server-rendered meet rows | 2026-09-20T09:10:35-0500 |
| `https://mitca.org/MITCA/` | GET | 200 | MITCA site: 0 MITS/indoor mentions; not an indoor archive | 2026-09-20T09:10:36-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_4942/event_summary.json` | GET | 200 | AQ MITS 1 (2019-12-14): 18 events — 2019 season still indexed | 2026-09-20T09:10:47-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/179592` | GET | 200 | 2019 AQ MITS G 800m: 23 rows, 0 grades, 0 AN ids | 2026-09-20T09:10:51-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_12574/event_summary.json` | GET | 200 | AQ MITS #2 (2022-01-22): 27 events | 2026-09-20T09:10:55-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_30103/event_summary.json` | GET | 200 | AQ MITS #3 (2024-02-03): 22 events | 2026-09-20T09:10:57-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/402116` | GET | 200 | 2022 AQ MITS #2 G LJ: 9 rows, 0 grades, 0 AN ids | 2026-09-20T09:11:00-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/1082624` | GET | 200 | 2024 AQ MITS #3 G SP: 7 rows, 0 grades, 0 AN ids | 2026-09-20T09:11:01-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_43298/event_summary.json` | GET | 200 | AQ MITS #2 (2025-01-25): 24 events | 2026-09-20T09:11:05-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_61819/event_summary.json` | GET | 200 | GVSU MITS #5 (2026-02-21): 37 events (31 individual, 6 relay) | 2026-09-20T09:11:07-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/1572722` | GET | 200 | 2025-01-25 AQ MITS #2 G LJ: 14 rows, 0 grades, 0 AN ids | 2026-09-20T09:11:10-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267002` | GET | 200 | 2026-02-21 GVSU MITS #5 G 200m: 6/6 graded, 6/6 AN ids; row carries result id, heat/lane, SB/PR flags, athlete+team AN ids | 2026-09-20T09:11:11-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_43498/event_summary.json` | GET | 200 | AQ MITS #3 (2025-02-01): 26 events | 2026-09-20T09:11:16-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_43408/event_summary.json` | GET | 200 | CMU MITS (2025-02-02, chttiming): 30 events | 2026-09-20T09:11:18-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/1584309` | GET | 200 | 2025-02-01 AQ MITS #3 B 60m: 77 rows, 0 grades, 0 AN ids | 2026-09-20T09:11:21-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/1577188` | GET | 200 | 2025-02-02 CMU MITS B 1600m (chttiming): 43/43 rows graded, 43/43 AN ids | 2026-09-20T09:11:22-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_59759/event_summary.json` | GET | 200 | AQ MITS #1 (2025-12-13): 24 events | 2026-09-20T09:11:29-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2183932` | GET | 200 | 2025-12-13 AQ MITS #1 B 1 Mile: doc exists but 0 rows | 2026-09-20T09:11:32-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2183950` | GET | 200 | 2025-12-13 AQ MITS #1 B 60m: 32 rows, 0 grades, 2 AN ids | 2026-09-20T09:11:40-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2183940` | GET | 200 | 2025-12-13 AQ MITS #1 G 200m: 14 rows, 0 grades, 0 AN ids | 2026-09-20T09:11:42-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/2023-mhsaa-track-field-regional-info-and-results` | GET | 404 | 404 — old slug exists only under boys-* | 2026-09-20T09:12:29-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/2024-mhsaa-track-field-regional-results` | GET | 404 | 404 — confirms single per-season regional page | 2026-09-20T09:12:30-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country/2023-mhsaa-cross-country-regional-info-and-results` | GET | 404 | 404 — same single-page pattern | 2026-09-20T09:12:32-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country/2024-mhsaa-cross-country-regional-info-and-results` | GET | 404 | 404 — same single-page pattern | 2026-09-20T09:12:33-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country/2026-mhsaa-lp-cross-country-regional-info-and-results` | GET | 404 | 404 — same single-page pattern | 2026-09-20T09:12:36-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2025/Finals/UP-D2-Girls.pdf` | HEAD | 404 | UP girls D2 finals 2025 exists (437,674 B downloaded): 96 graded rows, 17 grade-11 | 2026-09-20T09:13:44-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2025/Finals/UP-D3-Girls.pdf` | HEAD | 404 | UP girls D3 finals 2025 exists | 2026-09-20T09:13:45-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2024/Finals/UP-D1-Girls.pdf` | HEAD | 200 | UP girls D1 finals 2024 exists | 2026-09-20T09:13:46-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2023/UP%20D1Girls.htm` | HEAD | 200 | UP girls D1 finals 2023 (Hy-Tek HTML) | 2026-09-20T09:13:47-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/22girlstfupd1finals.txt` | HEAD | 200 | UP girls D1 finals 2022 (Hy-Tek text) | 2026-09-20T09:14:20-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/22girlstfupd2finals.txt` | HEAD | 200 | UP girls D2 finals 2022 | 2026-09-20T09:14:21-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/22girlstfupd3finals.txt` | HEAD | 200 | UP girls D3 finals 2022 (28,825 B): team rankings + event rows, no grade column | 2026-09-20T09:14:22-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/21trackupd1girlsfinal.txt` | HEAD | 200 | UP girls D1 finals 2021 | 2026-09-20T09:14:23-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/21trackupd2girlsfinal.txt` | HEAD | 200 | UP girls D2 finals 2021 | 2026-09-20T09:14:24-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/21trackupd3girlsfinal.txt` | HEAD | 200 | UP girls D3 finals 2021 | 2026-09-20T09:14:26-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267006` | GET | 200 | GVSU MITS #5 G Weight Throw: 13/13 graded | 2026-09-20T09:16:05-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267008` | GET | 200 | GVSU MITS #5 B Triple Jump: 4/4 graded | 2026-09-20T09:16:06-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267010` | GET | 200 | GVSU MITS #5 G 60m: doc exists (977 B), 0 rows | 2026-09-20T09:16:07-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267012` | GET | 200 | GVSU MITS #5 G Triple Jump: 10/10 graded | 2026-09-20T09:16:09-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267013` | GET | 200 | GVSU MITS #5 B Weight Throw: 18/18 graded; full field-series and conversion fields present | 2026-09-20T09:16:10-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_60442/event_summary.json` | GET | 200 | AQ MITS #2 (2026-01-17): 24 events | 2026-09-20T09:16:15-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2210117` | GET | 200 | 2026-01-17 AQ MITS #2 B 400m: 51 rows, 11 graded (~22%), 51/51 AN ids | 2026-09-20T09:16:18-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2210118` | GET | 200 | 2026-01-17 AQ MITS #2 G 1600m: 44 rows, 11 graded (25%), 44/44 AN ids | 2026-09-20T09:16:19-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2210121` | GET | 200 | 2026-01-17 AQ MITS #2 G 60m: 39 rows, 12 graded (~31%), 39/39 AN ids | 2026-09-20T09:16:20-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/20upxcd2girls_0.pdf` | HEAD | 200 | UP girls XC D2 finals 2020 (file name pattern "<YY>upxcd<N>girls_0.pdf") | 2026-09-20T09:17:23-0500 |
| `https://www.mhsaa.com/sites/default/files/Cross%20Country-Boys/2022/Finals/2022%20UP%20D2%20Girls.pdf` | HEAD | 200 | UP girls XC D2 finals 2022 (naming "2022 UP D<N> Girls.pdf") | 2026-09-20T09:17:24-0500 |
| `https://www.mhsaa.com/sites/default/files/Cross%20Country-Girls/2023/UP%20Girls%20D2.pdf` | HEAD | 200 | UP girls XC D2 finals 2023 (naming "UP Girls D<N>.pdf") | 2026-09-20T09:17:25-0500 |
| `https://search.athletic.live/michiana_meet_list/_search` | POST | 200 | 20 MITS docs in the michiana tenant index (`total.value=20`, `relation=eq`) | 2026-09-20T09:16:30-0500 |
| `https://search.athletic.live/*_meet_list/_search` | POST | 200 | 133 MITS docs across 7 indices (`live_results` 66, `fstiming` 22, `michiana` 20, `athleticlive` 20, `dlprotiming` 2, `chttiming` 2, `labtiming` 1) | 2026-09-20T09:16:34-0500 |
| `https://search.athletic.live/live_results_meet_list/_search` | POST | 200 | 66 cross-tenant MITS docs with tenant short hosts (fst.anet.live 21, fatresults.com 20, anet.live 20, dl 2, cht 2, lab 1); **17 meets since 2025-12-01** = the complete 2025-26 MITS slate | 2026-09-20T09:16:55-0500 |
| `https://search.athletic.live/live_results_meet_list/_search` (md >= 2025-12-01) | POST | 200 | `total.value=17` - season-scoped count used for the 2025-26 MITS table | 2026-09-20T09:17:02-0500 |
| `https://www.mhsaa.com/robots.txt` | GET (curl) | 200 | Drupal stock policy: only `/core/`, `/profiles/`, `/README.md` disallowed - results pages are crawlable | 2026-09-20T09:17:20-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2026-mhsaa-track-field-regional-results` | HEAD (curl) | 200 | `cache-control: max-age=3600, public`, `last-modified`, `x-cache: HIT` (Fastly), **no ETag** - change detection via Last-Modified/content hash | 2026-09-20T09:17:22-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2254285` | GET + `If-None-Match: 0x8DE6C38AB71852E` (curl) | 304 | 0-byte revalidation of an unchanged event doc - weekly sweeps are free | 2026-09-20T09:15:40-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_61819/event_summary.json?ns=trackmeet-io` | HEAD (curl) | 405 | Firebase RTDB rejects HEAD; a bogus `If-None-Match` still returns a full 200 - no ETag revalidation on this route | 2026-09-20T09:17:25-0500 |
