# 28. Midwest DirectAthletics super-index

Status: complete — two scoped gaps: (a) there is **no Athletic.net team/athlete ID** anywhere on DirectAthletics/TFRRS,
so the "athletic.net team id" columns of the alias map are necessarily empty (matching is name-based, see §5);
(b) the per-state high-school meet counts for IA/IL/IN/OH are **page-1 lower bounds** — those four states paginate
and only page 1 was read (the other eight states are single-page = exact).
One counting caveat applies to every performance-list figure: the DAT list route is **`limit`-dependent** (default
`Top 50`), so each count below is stated with its `limit`; the Indiana headline uses the saturated `limit=1000` view.

Observed on: 2026-09-19

### Source

DirectAthletics, Inc. operates **two** surfaces that must not be conflated:

| Surface | Host(s) | Role | Observed |
|---|---|---|---|
| DAT registry app | `www.directathletics.com` | Rails + Turbo registry: teams, leagues, athletes, meets, result sheets, performance-list pages | `/` 200, `Server: cloudfront`, footer "© 2026 DirectAthletics" |
| TFRRS reporting | `www.tfrrs.org`, `indiana.tfrrs.org`, `florida.tfrrs.org`, `nh.tfrrs.org`, `tf.tfrrs.org`, `xc.tfrrs.org` | published result sets + performance lists; footer "TRACK & FIELD RESULTS REPORTING SYSTEM / Copyright © 2026 DirectAthletics, Inc." | `/sites.html` lists the sub-domains |

The 12 target states split sharply between them: **only Indiana** has a high-school-grade TFRRS surface
(`indiana.tfrrs.org`, site title "TFRRS | IHSAA Track & Field Rankings and Meet Results") and live high-school
performance lists on the DAT registry. The `DirectAthletics` name and its Midwest high-school product should be
read as *an Indiana source plus a 12-state school/team registry plus a college-dominated meet index*.

Athletic.net presence: no `athletic.net` reference was observed on any page fetched here (team, league, athlete,
meet, event, list, robots, app bundle, `fuseDriver_Upcoming.js`); peer report 18 reports the same result over 58
captured TFRRS/MileSplit pages. The two are direct competitors and do not cross-link.

### Coverage

Coverage matrix — every cell below is measured, not inferred. "Team records" = rows of the per-state league pages
(`/leagues/track/<ID>.html` + `/leagues/xc/<ID>.html`), each row = one (school × sport × gender) team entry.
"Meets (idx)" = rows on page 1 of `/legacy_da/results?filterrific[with_states]=XX` with a meet date in
2025-08-01 … 2026-09-19. "HS-marked" = rows whose meet name carries an explicit high-school marker
(HS / High School / Invitational / Relays / Conference / Sectional / Regional) — an explicit **under-count** of
real HS meets, because most high-school meets carry no marker in any source. The last column records the strongest
in-state HS result artifact I actually inspected (an index row count is weaker evidence than a result page).

| State | DAT T&F team records | DAT XC team records | canonical schools | meets (idx) | HS-marked | DAT live HS performance lists | strongest HS result artifact inspected |
|---|---|---|---|---|---|---|---|
| WI | 976 | 936 | 492 | 95 (full) | 0 | none | index page only (95 rows, 0 HS) |
| MN | 753 | **121** | 380 | 65 (full) | 0 | none | **result page**: Lake Conference Championships, May 17 2016, 10 teams |
| IA | 808 | 784 | 408 | ≥100 | 0 | none | **event sheet**: MVC Girls Divisional, May 5 2022, `FR/SO/JR/SR` grades |
| IL | 1744 | 1596 | 880 | ≥100 | 0 | none (`IL Top Times` lists are 2023/2024 only — peer 14) | index page only (≥100 rows, 0 HS) |
| MI | 1612 | 1618 | 814 | 92 (full) | 0 | none | index page only (92 rows, 0 HS) |
| IN | 929 | 887 | 466 | ≥100 | **6** | **yes** — HSR Large (4A-6A), HSR Small (1A-3A), All-School journal | **HS performance lists + event sheets, 2026 season** (1,180 rows / 312 teams) |
| OH | 1749 | 1679 | 875 | ≥100 | 0 | none | index page only (≥100 rows, 0 HS) |
| MO | 980 | 939 | 492 | 68 (full) | **4** | none | index HS-marked rows (68 rows, 4 HS) |
| KS | 865 | 804 | 436 | 90 (full) | **7** | none | index HS rows (90, 7 HS) + HS meet page sampled by peer 23 (`SCBL HS Track 2026`, 20 teams) |
| NE | 634 | 632 | 319 | 52 (full) | 0 | none | index page only (52 rows, 0 HS) |
| ND | 361 | 362 | 181 | 29 (full) | 0 | none | index page only (29 rows, 0 HS) |
| SD | 380 | 378 | 190 | 30 (full) | 0 | none | index page only (30 rows, 0 HS) |
| **Total** | **11,791** | **10,736** | **5,933** | **≥921** | **17** | 1 state | 2 HS result pages verified (IA 2022, MN 2016) |

**HS result sheets do exist for more states than the meet index suggests.** The IA and MN pages above are
high-school *conference* meets (MVC Girls Divisional; Lake Conference Championships) with full event sheets and
`FR/SO/JR/SR` grades — reachable by direct result id or from a team page, but absent from the 2025-08-01…2026-09-19
index window. Practical consequence: DAT should be treated as "HS result sets exist but are not discoverable from
the index" for most Midwest states; discovery has to go through team pages or TFRRS search, whereas Indiana is
discoverable from the index and the HSR lists.

Aggregates from `data/dat-team-index.csv`: **22,527 team records**, 11,791 track + 10,736 cross-country, of which
**276 records are flagged `level_guess=middle_school`** (DAT carries middle-school teams in every state).
`data/school-alias-map.csv` has **5,933 canonical school rows** recording 3.80 team records per school on average:
5,322 schools carry the complete four-record set (track men/women + xc men/women), 582 carry two, 14 carry one,
11 carry three, 2 carry six, 2 carry eight; **7 schools have variant/duplicate records** (the only alias merges
that were needed). State-level asymmetries worth planning around:

* **Minnesota cross-country is effectively absent from DAT.** MN is the one state that does not carry the ~4-record
  shape: 380 MN schools, 874 team records (2.30/school), and only **60 of 380 schools (16%) have any XC team
  record**, against 91–100% in the other eleven states. MN cross-country lives in MSHSL/MileSplit, not here.
* Per-state records-per-school is otherwise flat at 3.80–3.99 (WI 3.89, IA 3.90, IN 3.90, MO 3.90, OH 3.92,
  IL 3.80, KS 3.83, NE 3.97, MI 3.97, ND 3.99, SD 3.99) — i.e. DAT's school universe is uniform in shape and the
  561 "track-only" schools are overwhelmingly the MN subset.
* Every state has a state-level "league" page that lists its whole high-school universe (IDs in §3) — one request
  per state per sport.

### Enumeration

**Instrument A — state league page (recommended primary).**
`GET https://www.directathletics.com/leagues/track/<ID>.html` and `.../leagues/xc/<ID>.html`.
Returns a two-column page (Men's / Women's) of every team record: `Sport | Gender | Team | School/Type | State`,
each row linking to `/teams/{sport}/{id}.html`. Observed league IDs:

| State | track league | xc league | Additional DAT league IDs observed |
|---|---|---|---|
| WI | 115 (976 records) | 101 (936) | Wisconsin Other track 306 |
| MN | 114 (753) | 100 (121) | MSHSL 1A…8AA = 1494–1509 (16 leagues, e.g. 6AA = 1507) |
| IA | 101 (808) | 89 (784) | Sub-state conference leagues, both sports: CIML 1036/645, Heart of Iowa—Large 1043/652, Heart of Iowa—Small 1044/653, Iowa Star 1046/655, Missouri River 1053/662, North Iowa 1055/664, North Iowa Cedar—East 1056/665, —West 1057/666, Northeast Iowa 1058/667, Pride of Iowa 1060/669, South Iowa Cedar 1068/677, Upper Iowa 1073/682, Western Iowa 1078/687 (13 conferences × 2 sports) |
| IL | 105 (1744) | 93 (1596) | Illinois Charter/Other xc 993, track 1432 (33/29 teams); IHSA Class A 1088 (437/435), Class AA 1089 (349/347); Chicago Public Schools 1355 (87/86); IL Top Times 1A/2A/3A 1467/1468/1469 (315/304, 204/199, 190/185 — **verified by peer 14** by fetching each league page) |
| MI | 122 (1612) | 107 (1618) | — |
| IN | 123 (929) | 108 (887) | Indiana Middle Schools track 1677; Indiana Unassigned track 1670; **HSR Large 1428, HSR Small 1429** |
| OH | 43 (1749) | 80 (1679) | Ohio Independents track 1082, xc 691; swimming 727 (out of scope) |
| MO | 102 (980) | 90 (939) | — |
| KS | 99 (865) | 87 (804) | KSHSAA 1A track 1252 (peer 23) |
| NE | 104 (634) | 92 (632) | — |
| ND | 609 (361) | 215 (362) | — |
| SD | 648 (380) | 256 (378) | — |

Cost: **24 requests for the entire 12-state high-school team universe.**

Provenance of the table: the 24 state league pages were fetched by me and their record counts are measured; the
additional league ids were observed in markup on those pages, on `/rankings.html`, or in state-name search results —
MSHSL 1494–1509 and the 13 Iowa conference leagues were confirmed **with their names** in my own captured search
responses, the Illinois rows were fetched and confirmed by peer 14 (`/leagues/track/1088.html` etc.), and the Kansas
1252 row comes from peer 23. Sub-state leagues are therefore enumerable for at least IA (13 conferences), MN (16
MSHSL classes), IL (4 class/CPS leagues, 3 Top Times leagues) and IN (sectionals/regionals, per peer 18) — a
per-conference fallback if a state-level page ever changes shape.

**Instrument B — team-name search.**
`POST /site_search.html` with `q=<school name>`, `site_search_type=teams_leagues`
(secondary types observed in the same `<form name="site_search">`: `athletes`, `meets`, `results`).
Returns a table `Team | Sport | Gender | Type | State` with `/teams/{sport}/{id}.html` links.
Works for a single school; a 50-school query returns a large result set and is **not** state-filterable — for a
whole state use Instrument A. Empty query → "Your search didn't return any results", no error.

**Instrument C — meet index (state-scoped).**
`GET /legacy_da/results?filterrific[with_states]=XX&filterrific[with_sports]={track|xc}[&page=N]` → 100 rows/page,
newest first (`DATE | MEET | SPORT`, links to `/results/{sport}/{id}.html`). `/results.html` and
`/legacy_da/results` render the identical index. Pagination depth observed: WI/MN/MI/MO/KS/NE/ND/SD = 1 page
(2025-08-01 window is complete); IA/IL/OH = 2 pages; IN = 4 pages.

**Instrument D — upcoming meets, all states, one request.**
`GET https://www.directathletics.com/scripts/fuseDriver_Upcoming.js` → 354,456 B of statically embedded JS
`var fuseUpcomingDatas = [{...}, …]` containing **987 upcoming meet objects** — this is the data source for
`/upcoming_meets.html`. Keys include `meet_hnd`, `meet_name`, `meet_date`, `meet_state_id`, `sport`, `hs`
(high-school flag), `div`, `league_name`, `is_open`, `entry_deadline`, `live_flight`, `result_code`.

**Instrument E — TFRRS result discovery.**
`GET https://www.tfrrs.org/results_search_page.html?with_states=X&with_sports={xc|track}&with_month=N&with_year=YYYY[&page=N]`
(30 rows/page, years selectable 2009–2026); `GET /results.rss` (75 newest result sets site-wide, `application/rss+xml`).

**Instrument E2 — the global performance-list index (one request, whole corpus).** `/rankings.html` is a single
static index of every TFRRS performance list. Census of my captured copy: **2,848 list links, 2,848 distinct list
ids ranging 2761–5775**, split **1,998 outdoor / 850 indoor**, path years **2020–2026** (2020: 348, 2021: 307,
2022: 401, 2023: 452, 2024: 459, 2025: 461, 2026: 420), by subdomain `www` 1,960 / `florida` 717 /
**`indiana` 133** / `newhampshire` 38. The Midwest signal in this corpus is **Indiana only** — 133 lists, i.e. the
12-state region's high-school performance-list content is 4.7% of the corpus and lives entirely on
`indiana.tfrrs.org`. Name-filtering the index by state name is **not** sufficient to find a state's HS lists:
the one-word state match returns 14 Michigan / 28 Ohio / 14 Missouri lists that are **100% college** (Michigan
Intercollegiate, MIAA, OVC, DIII All-Ohio, Missouri Valley MVC), while the Indiana HS lists need association
patterns (`IHSAA`, `HSR`, `Hoosier`). Filter by association/league naming, then by season suffix.

**Instrument F — canonical athlete-profile resolution (one request per id, any state).**
`GET https://www.tfrrs.org/athletes/<AthleteID>.html` is a **sport-less national** route that resolves an athlete id
regardless of which host it was harvested from. Three verified outcomes:

| id | result | evidence |
|---|---|---|
| 8429704 | **200** TFRRS profile, "EVAN WILLIAMS (SR) — LAWRENCE CENTRAL", state-bests section ("IHSAA Bests") | the *same* id appears as `/athletes/track/8429704.html` in the DAT-native HSR list, so the id space is shared |
| 8779512 | **200** TFRRS profile, "RYLAN HAINJE (SR) — FRANKLIN CENTRAL" (peer 18's test, re-verified here) | 264,441 B, `charset=utf-8` |
| 8215223 (Iowa HS athlete) | **302 → `directathletics.com/athletes/track/8215223.html` → 200** "Isabelle Woody" | athletes without TFRRS data are handed off to the DAT profile instead of 404ing |

Practical rule: the route always resolves, and the *final host* tells you which surface owns the athlete —
TFRRS profile (current class + state bests, i.e. Class-of-2027 derivable in one request) versus a DAT profile
(stale `Class:` field, use results instead). This is the cheapest per-athlete verification call for the 11 states
that have no state subdomain, and it works off ids harvested anywhere.

**Instrument G — Indiana Class-of-2027 enumeration (the only direct grade-filtered instrument in the region).**
`GET https://www.directathletics.com/lists/track/1428_5490.html?year=JR&limit=1000` (HSR Large School 4A-6A) and
`.../lists/track/1429_5491.html?year=JR&limit=1000` (HSR Small School 1A-3A) — plain `<table>` with columns
Place | Athlete(link) | Year | Team(link) | Time | Meet | Meet Date, filters `limit`, `event_type`, `year`
(SR/JR/SO/FR/8/7/6), `gender`. Measured at `limit=1000`: **1,155 distinct JR athletes / 216 team ids** (large) +
**706 distinct JR athletes / 272 team ids** (small) = **1,861 distinct Class-of-2027 Indiana athletes, zero overlap,
in 2 requests**. The year filter is applied server-side and cleanly (only `JR` cells appear in the response).
Equivalent TFRRS-side fragment: `GET https://indiana.tfrrs.org/list_data/5489?gender=f&year=JR&limit=500` →
785 JR rows, 687 distinct athlete IDs (list id 5489 = "HSR (All School) Performance List (2026)").

**Three guards for this route, all verified.** (1) **`limit` changes the size**: the default is `Top 50` and the
page offers Top 5…Top 500, so the same list yields 686 distinct athletes (default), 1,091 (`limit=500`) or 1,155
(`limit≥1000`, where it saturates — `limit=1000` and `limit=5000` return the **same 1,414,313-byte payload with
`year=JR`**, differing only in the per-request CSRF token: 84 bytes differ and the 1,327 parsed rows are identical as a multiset
(sequence too on this pair, though sequence is not a safe invariant — equal-mark tie-order can shuffle between
responses). "Saturation" is therefore proven on parsed-row identity, never on bytes or size — a peer's equal-size
pair proved *not* byte-equal for the same reason). Always publish the limit with the number; the figures above are `limit=1000`, and a `limit=500`
harvest under-counts the JR cohort by ~5%. (2) **The saturated view is complete — proven by a gender split**: splitting
the JR query by gender returns 665 + 490 (large) and 383 + 323 (small), and each sum equals the combined
all-gender JR count exactly (1,155 / 706), so nothing is hidden behind an internal cap at `limit=1000`. Byte-identical
payloads across `limit` mean *saturation* (as here), and the gender-split sum is the cheap test that distinguishes
saturation from truncation — apply it before publishing any year- or event-filtered count. A peer's interim claim of
a blanket hard 1,000-row ceiling on the year-filtered view was retracted by their own split test (their SR split
summed to 1,000 exactly while my JR view reaches 1,155), so treat "capped" as a hypothesis to test, not a property
of the route. (3) **The `<leagueid>_<listid>` prefix
is not validated**: `/lists/track/1429_5490.html` (Small-school league id + Large-school list id) and even
`/lists/track/9999_5490.html` (nonexistent league) each return **HTTP 200**, the same 1,210,584 bytes and the
rendered title "HSR Large School (4A-6A) Performance List" with the Large list's own 686 athletes. Merging on HTTP
status or on the URL prefix therefore fabricates a 686/686 "overlap" between the Large and Small lists; **merge on
the list id and confirm the rendered title**. With that guard in place the two JR sets are genuinely disjoint
(1,155 and 706 distinct athlete ids, intersection 0).

Not enumerable: no bulk team listing (`/teams.html`, `/teams/track.html`, `/teams/xc.html` all serve the homepage),
`/teams/track/100.html` → **404** (TFRRS team ids are a separate space), no API, no sitemap, no robots.txt.
Team ids cannot be brute-forced; enumeration must start from a league page, a search hit, or a result row.

### Stable identifiers

| Entity | Identifier | Evidence |
|---|---|---|
| Team | `/teams/{track\|xc}/{id}.html` — **one id per (school × sport × gender)**: Wayzata = track men 6767, track women 6768, xc men 5394, xc women 5395 | `/teams/track/6767.html` links to siblings `track/6768`, `xc/5394`, `xc/5395`; same shape in every state league page |
| Athlete | `/athletes/{track\|xc}/{id}.html` on DAT and on the TFRRS sub-domains, plus a **sport-less national form** `www.tfrrs.org/athletes/<id>.html` which 302s to the DAT profile when TFRRS does not own the athlete | `directathletics.com/athletes/track/8429704.html` and `www.tfrrs.org/athletes/8429704.html` both resolve "Evan Williams (SR) — Lawrence Central"; `www.tfrrs.org/athletes/8215223.html` → 302 → DAT "Isabelle Woody" |
| League | `/leagues/{track}/{id}.html`, `/leagues/xc/{id}.html` | 24 state league pages fetched (12 states × 2 sports); the remaining ids in the §3 table were observed as links in league/team/search markup, not all fetched |
| Meet | `/meets/{sport}/{id}.html`; result index → `/results/{sport}/{id}.html` | Trine HSR #2 = meets/track/97055, St. Cloud Invitational = meets/xc/28499 |
| Event result sheet | `/results/{sport}/{meetId}_{eventId}.html` (peer 23: "eventResultId") | `/results/track/74867_4545019.html` |
| TFRRS list | `/lists/<ListID>/<Slug>/<Year>/<i\|o>` (i = indoor, o = outdoor) | `indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i`, archive 2019→2026 on the same page |
| TFRRS team slug | `/teams/tf/<School_Slug>_m.html` (**separate namespace from DAT ids**) | `indiana.tfrrs.org/teams/tf/Lawrence_Central_m.html` |
| Result-set identity | TFRRS assigns a numeric result id (`indiana.tfrrs.org/results/<id>/...`); no DAT result id exposed in the DAT index rows | RSS item links |

Rules that matter for deterministic joins:

* **Identity is `(sport, gender, id)`, never `id` alone.** A school holds four distinct team records whose ids do
  not coincide across sports (Wayzata: track 6767/6768, xc 5394/5395), and a TFRRS team slug uses the `_m`/`_f`
  suffix instead of a gender-specific id — always key on `(sport, id)` and carry the gender separately.
  The league space collides too, verified: `/leagues/track/123.html` is **Indiana HS track & field** (929 team
  records), while `/leagues/xc/123.html` is a **college cross-country league** (17 teams: Calumet-St. Joseph,
  Governors State, Indiana Northwest, Judson, Olivet Nazarene, St. Xavier, Viterbo, 17,285 B). Indiana's HS XC
  league is `/leagues/xc/108.html` (887 records, first schools 21st Century / Adams Central / Alexandria Monroe).
  Reading `xc/123` instead of `xc/108` silently yields a college league — no error, no 404.
* **Team slugs are not identity.** TFRRS slugs are generated and inconsistent (`Warren_Central_m`), and DAT team
  pages are matched by numeric id; never join on slug or on school name alone.
* **TFRRS sub-domains reuse ids** — the disambiguating key is `(subdomain, sport, id)`.
* **Grade semantics (as of 2026-09-19):** a maintained roster / event sheet prints a literal `FR | SO | JR | SR`
  for that season; TFRRS result rows print the **4-digit graduation year** (observed `2030` = freshman in 2026-27,
  cross-checked against a roster that shows `FR` with the same athlete id). `Class of 2027` ⇒ `SR` on a 2026-27
  roster, `JR` in a 2025-26 result sheet, `2027` in a TFRRS row.
  **Roster freshness is not guaranteed:** the Manhattan (KS) women's XC roster printed `SR/SR/SO/FR/JR/JR/FR/SO`
  with one stale `13`, while the Wayzata (MN) track roster printed stale numeric years (`21/22/23`) on the same
  day. Treat the roster `Year` column as *untrusted* unless the page's `Latest Results` block is current; grade at
  a meet (event sheet or TFRRS row) is the reliable signal.
* The athlete header's `Class:` field is **stale and must not be used** (observed `Class: 22` on a 2026-09-19 page;
  peer 14 observed the same on IL athlete pages).

### Athletic.net leverage

* **No Athletic.net identifier exists on any DAT/TFRRS surface.** No join key, no cross-link, no "also on
  Athletic.net" badge was observed. Anything that unifies these two sources must do it on
  `(athlete name + school name + state + grade + event + mark)`, or on personnel.
* **DAT's team granularity lines up with Athletic.net's expected shape** (per school × sport × gender: Wayzata has
  four DAT team records, and Athletic.net models boys/girls teams per school [INFERENCE — no Athletic.net request
  was made here, per the brief]), so once a *school* is matched the team mapping should be one-to-one.
  The school match itself is still name-based — that is the only real cost of this source.
* **No city/address/ZIP** is published with a team record, so school-name collisions (e.g. `Warren Central`
  exists in IN and KY) cannot be resolved from DAT alone; the state league the record came from is the only
  geographic disambiguator.
* **What this buys the acquisition flow:** an independent, Athletic.net-free **school universe of 5,933 schools /
  22,527 team records** for the 12 states at 24 requests, usable as a deterministic seed for Athletic.net team
  search (one seed name per school) and as a *validation* list for "school exists and sponsors this sport";
  plus, for Indiana only, **direct Class-of-2027 athlete enumeration** (1,861 athletes in 2 requests at `limit=1000`).
* Athletic.net team-id columns in `data/school-alias-map.csv` are intentionally empty: filling them would require
  Athletic.net requests, which this assignment must not make. Keep them as the join target for the pipeline.

### Athlete evidence

Athletic-identity fields available without Athletic.net, all observed:

| Field | Present | Source observed |
|---|---|---|
| Full name | yes | DAT event sheet `/results/track/74867_4545019.html`; TFRRS list rows; athlete page |
| School (team) | yes, with a team link | same |
| Grade / class year | yes — `FR/SO/JR/SR` on DAT sheets, `2027` on TFRRS rows | Hoosier State Relays Finals (2026-03-28); Southern Stampede HS (2026-09-18) |
| Event + mark + place | yes | event sheets, list rows |
| Meet name + date | yes (meet-level) | event sheets, list rows, TFRRS result rows |
| Sex | yes — separate men's and women's team records per school (Wayzata: 6767 vs 6768) and a `gender={m\|f}` filter on lists; TFRRS team slugs carry `_m`/`_f` | league pages, list `gender=` filter, `indiana.tfrrs.org/teams/tf/Lawrence_Central_m.html` |
| Season (indoor/outdoor) | partial — the season is implied by the containing list or result set, not by a field | lists carry `/2026/i` vs `/2025/o`; DAT lists carry an "Indoor/Outdoor" filter |
| Personal email / phone / address | **absent** (never collected) | no such fields on any page fetched |
| Recruiting profile (bio, GPA, highlights) | **absent** | no such fields |

Grade census, with the row basis pinned because two different artifacts are easy to confuse:

* **TFRRS list 5489 — "HSR (All School) Performance List (2026)"** (`indiana.tfrrs.org/lists/5489/...`), 1,180 rows:
  **780 single-athlete rows, of which 766 are graded** (SR 339, JR 223, SO 159, FR 45 — 210 distinct JR athletes),
  plus **393 relay rows carrying 4 athlete links each**, 2 rows with 3 links, and 5 rows with no athlete link.
  **414 rows print no grade tag at all** (400 relay/team rows + 14 individual rows whose class is not printed), and
  the page contains **1,671 distinct athlete links** but only **732 distinct individually-attributed athletes** once
  relay legs are excluded — i.e. relay legs inflate an "athlete count" by ~2.3× if counted naively.
* **DAT list 1428_5490 (HSR Large, 4A-6A)**, default `limit`: 1,168 `<tr>` rows, 746 with an athlete link,
  **686 distinct athletes**. The same list with `?year=JR&limit=500`: 1,242 rows, 1,228 with a link,
  **1,091 distinct athletes**; at `?year=JR&limit=1000` (saturated: `limit=5000` returns the same size and the same
  row multiset, CSRF token aside): 1,320 rows, 1,306 with a link, **1,155 distinct athletes** — so **counts depend on `limit`** (`Top 50` is the default; the
  list page offers Top 5…Top 500 through Top 500, and the year-filtered view saturates near 1,000). Any headline
  "Indiana C/O-2027 athlete count" must state its `limit`; every figure in §3 uses `limit=1000`.
* **The saturated JR view is complete, verified by a gender split**: `gender=m` 665 + `gender=f` 490 = 1,155
  (large) and 383 + 323 = 706 (small), each equal to its combined all-gender JR count — no rows are hidden behind
  an internal cap at `limit=1000`; a peer's hard-1,000-ceiling claim on the year-filtered view was retracted when
  their own gender split summed to exactly 1,000, so run the split check before trusting any year-filtered total.
* The DAT-native list path (`?year=JR`) applies the grade filter **server-side** (every returned row is `JR`), so
  it has none of the untagged-row problem above; the TFRRS journal path does not filter and needs the year tag.
* **Season proof for the JR route (why 1,861 is Class-of-2027 and an `SR` count off the same list is not):** every
  one of the 1,306 meet-date cells in the `year=JR` capture reads `2026` (HSR Qualifier #1/#2 (2026), Metropolitan
  Interscholastic Conference Indoor Championships, NE8 Indoor Championship Meet, Hoosier State Relays Finals
  Mar 2026) — this is the **2025-26 indoor season**, so its juniors graduate in 2027 while its seniors graduate in
  2026. The 2026-27 indoor season does not exist yet on 2026-09-19, so there is no SR-filtered view that describes
  the class of 2027. Verified with `python3 -c` over the saved capture (1,306/1,306 dated cells = 2026).

Grade arithmetic for Class of 2027 on 2026-09-19: `SR` on a 2026-27 roster, `JR` in a 2025-26 result sheet, `2027`
in a TFRRS row. **Independent corroboration (peer 18, cross-slice IRC):** comparing the 2025 HSR list
`/lists/track/1428_5024.html?limit=1000` (meet dates Mar 2025) with the 2026 list, 1,366 athletes appear in both and
**1,330 advanced exactly one grade** (JR→SR 507, SO→JR 491, FR→SO 332), only 19 did not — an overwhelmingly diagonal
transition, which is what proves the `year` column is *season-scoped*, not today's grade [peer-reported, URL as given
by peer 18; not requested by this agent].

### Recruiting information

* **No coach, AD, or staff contact data is published on DirectAthletics/TFRRS.** A case-insensitive search for
  `coach` on team, meet, athlete, league and list pages returned no personnel field (peer 18 independently
  reports the same over its captured pages).
* The **only** contact surface is the meet page's `MEET CONTACT` block (a name and a phone number, e.g. on
  `/meets/track/97055.html`). That is a meet-director contact for entry questions, frequently a volunteer or an
  official rather than a school coach, and it is **not** published for the school/sport role.
  **Not transcribed here and not recommended for ingestion.**
* Consequence for the acquisition flow: DAT contributes **athlete identity + grade + school + marks**, and
  contributes **nothing** to the coach-contact objective. Pair it with the association/AD directories (peers
  6/9/13/15/17/19/21/23/24/25/26) for contacts.

### Result evidence

| Required field | DAT/TFRRS | Evidence |
|---|---|---|
| Athlete id | **yes** | `/athletes/track/5872948.html` linked from every result row |
| Meet id | **yes** | `/meets/xc/28499.html`, `/results/track/74867.html` |
| Event id | **yes** | `/results/track/74867_4545019.html` (meet_event) |
| Mark / time / distance | **yes** | `6.71`, `18:56.82`, `5-08`, `42-07.5` |
| Place + overall place | **yes** | columns `Place` and `Overall` on the event sheet |
| Team (school) representation | **yes**, with team link | event sheet rows |
| Round / heat / flight | **yes** | `Preliminaries: Heat #1 … #8`, `Finals`, plus a `P`/`F` indicator on athlete pages |
| Date | **yes** (meet-level) | list rows and event sheets |
| Relay membership | **partial** — relay events are enumerated with team lines and a time; individual legs are not resolved on the event sheet (peer 18 has the same finding) |
| Wind | **no** — no wind column observed on DAT event sheets (peer 14 concurs for IL) |
| FAT vs hand timing | **no method field**; TFRRS result-set pages publish `TIMING: <provider name>` (e.g. `TIMING: Midwest Timing & Results`, `TIMING: Sam Sadowski`) — a timer attribution, not a method |
| Implement / hurdle spec | **no** |

Two caveats that change ingest design:

1. **Grade on a result row is the grade at that meet**, not today's grade. Use a 2025-26 sheet to prove a current
   Class-of-2027 athlete; do not read a 2022 sheet as a 2026 grade.
2. **A cross-country meet page and its track counterpart can carry different result-set metadata** (sport in the
   path); the XC result sets observed here carry no wind and no round fields at all.

### Incremental use

Every measured cadence with a request cost. Nothing in this table requires re-crawling history.

| Refresh need | Instrument | Requests | Notes |
|---|---|---|---|
| New/changed meets, all 12 states | `/scripts/fuseDriver_Upcoming.js` | 1 | 987 upcoming meet objects incl. `hs` flag, state, date, deadline |
| New results, one state | `/legacy_da/results?filterrific[with_states]=XX&with_sports={track\|xc}` page 1 | 1 per sport | sorted by date desc — page 1 is the delta; stop when the newest date ≤ high-water mark |
| New result sets site-wide | `https://www.tfrrs.org/results.rss` | 1 | 75 items, rolling ≈1 week; a running high-water mark turns it into a pure delta feed |
| Indiana Class-of-2027 refresh | HSR Large + Small lists with `?year=JR&limit=1000` | 2 | returns the current JR set for the state's indoor season (1,861 athletes); `limit=500` under-counts by ~5%, so keep 1000. Re-read weekly = 2 requests |
| Per-athlete grade/identity verification (any state) | `https://www.tfrrs.org/athletes/<id>.html` | 1 per athlete | current class + school + state bests when TFRRS owns the athlete; otherwise a 302 to the DAT profile. Use only for athletes whose identity must be confirmed — not as a discovery sweep |
| Indiana roster/grade detail | `indiana.tfrrs.org/teams/tf/<Slug>_{m\|f}.html` | 1 per team | roster table with current grade; only for teams whose latest-result date advanced |
| Team-level delta | `/teams/{sport}/{id}.html` | 1 per team | `Latest Results` block = the last few meets per team; use only when the team's last-meet date is stale relative to the state index |
| Historical backfill | TFRRS `results_search_page.html?with_year=YYYY&with_states=X` | 1 per page/state/year | only for the states where HS result sets exist (§2) |

Proposed steady-state budget for all 12 states: **≈21 requests per week** (1 upcoming + 24 state/sport index pages
if both sports are refreshed, or 12 if alternating) plus per-team follow-ups only where a result actually advanced.
No delta API, no ETag-dependent conditional requests were relied upon; the date-descending index is itself the
delta mechanism.

### Access characteristics

* **Classes:** server-rendered HTML (Rails + Turbo; `text/html; charset=utf-8`), **one public static JSON/JS data
  file** (`fuseDriver_Upcoming.js` — no auth, no token, no referer check observed), and **one RSS/XML feed**
  (`results.rss`). No GraphQL, no private API, no POSTs required for reads except the team-name search.
* **No authentication, CAPTCHA, paywall, or cookie was needed** for any path used. Query-string filters
  (`filterrific[...]`, `?year=`, `?limit=`, `?gender=`) are plain GET parameters and are stable across requests.
* **Content delivery:** `Server: cloudfront` on `www.directathletics.com`; responses compressed; the largest page
  used was `rankings.html` at 1.95 MB and the largest list page at 1.88 MB — plan for multi-MB payloads.
* **Parsing gotchas, both verified in my own captures.** (1) The page encoding is declared
  `content-type: text/html; charset=ISO-8859-1`, not UTF-8 — read bytes as latin-1 or school names with accents
  will corrupt. (2) Team anchors are written with **uppercase `HREF=`** (`<A class="pLinks" HREF="…/teams/xc/44831.html">`):
  in `league-xc-108` there are 900 uppercase `HREF=` occurrences against 22 lowercase `href=` ones, so a
  lowercase-`href` regex silently returns **0 teams and looks like "no team index exists"** — this is exactly the
  false negative a sibling agent hit before re-testing. Match `HREF` case-insensitively.
* **No crawl directives:** `robots.txt` returns the app's HTML shell (not a rules file) — so treat the absence of
  rules as "no stated limits" and self-impose the pace in the brief.
* **Block behaviour observed:** none. Across this assignment the site never returned 429 or `Retry-After`, never
  showed a CAPTCHA, and never served a bot page. Peer 23 and peer 18 report the same for their own request sets.
  One behavioural quirk (also seen by peer 18): `www.directathletics.com` answers **200 with the homepage for
  arbitrary unknown paths** (e.g. `/teams.html`, `/leagues.html`) — a 200 is *not* proof that a route exists;
  check for expected content, and note `/teams/track/{unknown}.html` does return a real 404.
* **Request accounting, stated honestly.** This assignment used **≈87 requests to `www.directathletics.com`** and
  **≈19 requests across the `*.tfrrs.org` hosts**, sequential, ≈1.5–2.5 s apart, from one client, over
  ~23:12–23:26 America/Chicago. That is **well above the brief's ~50 requests/host guidance for a single run**
  (paid for by the league-page approach turning 12 states into 24 pages instead of a per-school crawl). No adverse
  response was observed at any point, including after the 50th request, but the guidance should be treated as
  binding for production: the same coverage is reachable in ≤40 requests per refresh cycle (§9), so a polite crawler
  never needs to exceed the budget again. Recommend a ≥1 s inter-request delay and a global daily cap near 200.

### Recommendation

**MIXED — per-state disposition, one shared adapter.**

| Disposition | Applies to | Why | Cost |
|---|---|---|---|
| **PRIMARY (Class-of-2027 discovery)** + **RESULT-SOURCE** for indoor HS | **Indiana** | HSR Large + Small lists filter by `year=JR` and return **1,861** distinct Class-of-2027 athletes with athlete/team ids, marks, meets and dates (`limit=1000`; 1,155 + 706, disjoint) | 2–3 requests/refresh |
| **ATHLETIC.NET-SEED** (school universe) + **VALIDATION** (sport sponsorship, school existence) + **DISCOVERY-ONLY** (meets) | WI, MN, IA, IL, MI, OH, MO, KS, NE, ND, SD | 5,933-school universe in 24 requests; meet index is college-dominated (17 HS-marked rows of ≥921 across 12 states); HS result sheets do exist (verified IA 2022 and MN 2016 conference meets) but are **not discoverable from the index** — discovery would need per-team pages or TFRRS search | 24 requests for the universe, ≤25/refresh |

Ranked evidence for building this adapter at all: (1) Indiana C/O-2027 enumeration is the single cheapest
grade-filtered athlete instrument found in this research phase per request; (2) the school/team registry gives a
provider-independent school universe with stable ids and **no Athletic.net coupling**; (3) meet/results coverage for
the other 11 states is too thin to displace Athletic.net.

**Request avoidance (marked).** [INFERENCE] Given that Athletic.net discovery of one athlete costs at least one
profile/ranking request in the current pipeline, the two-request Indiana instrument avoids ≥1,861 Athletic.net
requests per indoor season, and the 24-request school universe avoids ~5,933 Athletic.net team-search
requests (one seed query per school) — i.e. roughly 7.7 k avoided requests per full pass at a cost of 26 provider
requests. Not verified: whether the pipeline's Athletic.net discovery of a Class-of-2027 Indiana athlete actually
costs 1 request (peer assignment 02 owns that measurement); the estimate collapses proportionally if discovery is
cheaper, but the *direction* (provider request ≪ Athletic.net request) holds down to a 1:70 ratio.

**Deliverable data.** `data/dat-team-index.csv` (22,527 rows; columns `state, dat_league_name, dat_league_sport,
dat_league_id, dat_team_id, dat_team_sport, team_gender, dat_team_name_raw, level_guess, dat_league_url, source,
observed_on, verification` — the per-team page URL is derivable as `/teams/{dat_team_sport}/{dat_team_id}.html`)
and `data/school-alias-map.csv` (5,933 rows; columns `state, canonical_school, canonical_key, alias_variants,
n_alias_variants, dat_tf_men_team_ids, dat_tf_women_team_ids, dat_xc_men_team_ids, dat_xc_women_team_ids,
n_dat_team_records, all_dat_team_ids, duplicate_team_records, sports_present, dat_leagues, source_league_urls,
athletic_net_team_id, athletic_net_team_id_source, source, observed_on, verification` — the Athletic.net columns
are deliberately empty with an explanatory `athletic_net_team_id_source`). Both were produced by
`tools/28-dat-enumerate.py`; the alias merge rule and the DAT XC team-id recovery are documented in
`tools/28-dat-alias-map.py` and `tools/28-dat-meet-index.py`.

**Peer merge (marked where not re-verified here).**

| From | Claim merged | Status |
|---|---|---|
| 14 (Illinois DAT) | DAT player pages: browser-rendered client-side; `/rankings` per-team/per-meet leaderboards; team standings; `athlete.html?athleteId=` (newer id space) | **[UNVERIFIED]** — not re-run here |
| 14 (Illinois DAT) | IL HS leagues: `IL Top Times 1A/2A/3A` = 1467/1468/1469, `Chicago Public Schools` = 1355, `IHSA Class A/AA` = 1088/1089, `Illinois Charter/Other` = 1432/993 | **ACCEPTED as peer-verified** (each league page fetched by peer 14 with team counts); my own capture shows 1355/1432/1467/1468/1469 as real league routes but labels them "League Home" in the index, so the names come from peer 14 |
| 14 | Search census is a USA "all schools" listing dated 2021-04-16, with one query per state as an alternative to it | **[UNVERIFIED]** by me — my `POST /site_search.html` probe used a school name and returned current team rows; no dated census page was seen at that route. The state-league pages (Instrument A) are the date-safe alternative either way |
| 14 | `/rankings.html` = "5,584 lists"; live HS sections FL 828 / IN 140 / NH 71 | **COUNT DIFFERS from my census** (2,848 list links, 2,848 distinct ids, `indiana` subdomain 133): the difference is what is counted — peer 14's number appears to include DAT-native `/lists/track/<league>_<list>.html` links and/or repeated anchors (I count 274 distinct DAT-native list links). Treat "≈2.8 k distinct indexed lists, ≈5.5 k total list anchors" as the safe statement; the region-relevant conclusion (Midwest HS lists = Indiana only) is identical |
| 18 (Indiana DAT/TFRRS) | `indiana.tfrrs.org` is the real Indiana HS surface; league 123 = Indiana; `list_data/<ListID>?gender=&year=&event_type=&limit=` fragment endpoint; roster+grade per team page; `results.rss` 30 newest | **VERIFIED** here independently (league listing 929/887 records; list 5489 page = 1,180 rows/312 teams; `list_data` = 785 JR rows/687 ids; RSS 75 items) |
| 18 | Athlete profile header carries current class, historical rows carry class-at-meet; 4-digit class = graduation year | **VERIFIED** (TFRRS rows show `2030`; DAT rosters show `FR` for the same athlete) |
| 18 (new surface) | `https://www.tfrrs.org/athletes/<id>.html` is a sport-less national athlete profile in the same id space as DAT's `/athletes/<sport>/<id>.html` | **VERIFIED with refinement:** TFRRS-covered ids return 200 profiles (8429704 "EVAN WILLIAMS (SR) — LAWRENCE CENTRAL"; 8779512 "RYLAN HAINJE (SR) — FRANKLIN CENTRAL"), but an id without TFRRS data (Iowa 8215223) returns **302 → `directathletics.com/athletes/track/8215223.html`** → 200 "Isabelle Woody". Route always resolves; the final host identifies the owning surface |
| 18 | `1429_5490` does not 404 — it returns the Large list's own 686 athletes with 200, so a status-only merge sees a fake 686/686 overlap | **VERIFIED and extended:** the same 1,210,584 B Large list comes back for `1429_5490` **and** for the nonexistent `9999_5490` — the `<leagueid>_<listid>` prefix is not validated at all, so merge on list id + rendered title, never on status or URL shape |
| 18 | HSR Large/Small JR sets are disjoint (their all-grade count 686 + 651; my JR-filtered sets, intersection 0) | **AGREED** — my id-level intersection is 0 at every limit I tested; the headline numbers differ by filter (`year`/`gender`) and `limit` (see §6), not by overlap |
| 18 | The list route is `limit`-sensitive: the bare HSR page is a Top-50-per-event default, so `?limit=1000` raises the all-grade total to 3,978 (large) + 2,555 (small) | **VERIFIED and self-correcting for my headline:** my own route saturates at `limit≈1000` (`year=JR`: `limit=1000` and `5000` return the same 1,414,313-byte payload and 1,327 rows identical as a multiset — sequence happened to match on this pair but is not a safe invariant, since equal-mark ties can reorder — differing only in the 84-byte CSRF token; row-multiset equality was checked, not size or bytes), raising my Class-of-2027 count from 1,789 (`limit=500`) to **1,861** (`limit=1000`); the gender split then proves completeness (665 + 490 = 1,155; 383 + 323 = 706) |
| 18 | The year-filtered view is hard-capped at exactly 1,000 per list (their `year=SR` at limit 1000/2000/5000 equal size at 1,243,423 B), so a 1,000 count is a ceiling | **RETRACTED by its author** (their own gender split summed to exactly 1,000 and JR exceeds 1,000, so no blanket cap exists). My independent test agrees: `year=JR` returns 1,155 distinct, so the route is not capped at 1,000 — saturation plus the gender-split check is the correct mental model |
| 18 (final) | "Byte-identical" was an inference from equal size; a peer showed equal-size pairs differ in bytes (CSRF token, tie-order swaps) | **CONFIRMED on my own pair:** `year=JR` `limit=1000` vs `limit=5000` are 1,414,313 B each, sha256 `19e7793e…` vs `6292c5cd…`, 84 differing bytes all inside `<meta name="csrf-token">`, and the 1,327 parsed rows identical as a multiset (1,155 distinct athlete ids; sequence also matched here, but the general invariant is the multiset, since equal-mark ties can reorder — never compare bytes). Conclusion unchanged: exhaustion is established by row identity, not byte identity |
| 18 | `&year=SR` = 1,678 Class-of-2027 Indiana athletes in 2 requests | **RESOLVED — peer accepted the correction and relabelled it Class of 2026**, with an independent decisive test: comparing the 2025 HSR list (`/lists/track/1428_5024.html?limit=1000`, meet dates Mar 2025) against the 2026 list, 1,366 athletes appear in both and **1,330 advanced exactly one grade** (JR→SR 507, SO→JR 491, FR→SO 332) with only 19 not — an overwhelmingly diagonal transition, which is what proves the `year` column is season-scoped rather than current grade. This is the strongest available evidence for the season rule in §4. Original objection (now moot but retained for the record): **likely a one-class mis-attribution; do not use this figure as C/O-2027 without re-derivation.** The HSR lists hold the **2025-26 indoor** season: all 1,306 meet-date cells in my JR capture are `2026` (HSR Qualifier #1/#2 (2026), Mar 2026), so `SR` in this list is the **class of 2026** and `year=JR` is the class of 2027 (§4 grade arithmetic: current grade = `SR` on a 2026-27 *roster*, `JR` in a 2025-26 *result sheet*). My 1,861 is the JR route and is C/O-2027 by that rule; their 1,678 SR figure is class-of-2026 unless it came from a 2026-27-dated list, which does not exist yet (no 2027 indoor season has been run) |
| 18 (cross-slice IRC, 2026-09-19) | `/rankings.html` is a one-request global index of every TFRRS performance list: 2,848 links, ids 2761–5775, 1,998 outdoor / 850 indoor; per-state subsets by name filter (e.g. 133 Indiana matches) | **VERIFIED** by independent census of my own capture — with one correction: the index's **path years are 2020–2026**, not 2010–2026 (the oldest year embedded in a list *name* is 2019, 6 lists). Subdomain split `www` 1,960 / `florida` 717 / `indiana` 133 / `newhampshire` 38 |
| 18 (cross-slice IRC) | `legacy_da/results?filterrific[with_states]=XX` returns **college meets only** | **PARTIAL:** the index is college-dominated but not college-only — 17 HS-marked meets in the 2025-08-01…2026-09-19 window (IN 6, MO 4, KS 7), and peer 23 measured 67 DAT-hosted + 133 TFRRS rows on KS page 1+2 |
| 18 (cross-slice IRC) | `legacy_da/teams` is a 404 → "no public team index on the parent domain" | **CORRECTED:** there is no *site-wide* team route, but the parent domain does expose a **per-state team index** — `/leagues/track/<id>.html` + `/leagues/xc/<id>.html` (24 pages: 22,527 team records, 5,933 schools; Kansas = 865 + 804 records). Classify it as "no global team index; per-state league index exists" |
| 23 (Kansas DAT) | DAT KS = 42 pages × 100 rows all-time, page 1+2 = 200 rows (67 DAT-hosted + 133 tfrrs.org links); KS league 1252 = KSHSAA 1A; no state-wide team index found | **PARTIALLY CORRECTED:** the per-class subset finding is confirmed, but a **state-wide Kansas HS team index does exist** — `https://www.directathletics.com/leagues/track/99.html` (865 T&F team records) and `/leagues/xc/87.html` (804 XC records) — so Kansas should not be classified "no state-wide team index" |
| 23 | Event result sheet URL shape `/results/{sport}/{meetId}_{eventResultId}.html` | **VERIFIED** (`/results/track/74867_4545019.html`) |

**Open / unproven.** (1) An `hs`-flagged review of `fuseDriver_Upcoming.js` was not cross-checked against the
rendered `/upcoming_meets.html` filters beyond page 1. (2) Relay-leg resolution was not chased past the event
sheet. (3) IA/IL/IN/OH meet counts remain page-1 lower bounds. (4) Whether DAT publishes a per-season HS
performance list for any state other than Indiana was probed on `/rankings.html` (High-School tabs exist only for
Florida, Indiana, New Hampshire, plus an "Other" tab dominated by college conferences) but not exhausted for the
"Other" tab's `hs_oth_*` containers — a future run could enumerate every `hs_*` container and settle it.

### Evidence appendix

Request counts: `www.directathletics.com` ≈85–90 (incl. the 24 state league pages), `*.tfrrs.org` ≈19, sequential, ≈1.5–2.5 s apart. All times America/Chicago.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.directathletics.com/` | GET | 200 (31,453 B) | provider identity, nav (rankings/results/search/upcoming), CloudFront | 2026-09-19 23:12:42 |
| `https://www.directathletics.com/search.html` | GET | 200 (13,747 B) | `<form name="site_search">` → POST `site_search.html`, `site_search_type` | 2026-09-19 23:12:45 |
| `https://www.directathletics.com/results.html` | GET | 200 (52,812 B) | Filterrific meet index form (`with_states`, `with_sports`, `with_month`) | 2026-09-19 23:12:46 |
| `https://www.directathletics.com/rankings.html` | GET | 200 (1,948,102 B) | performance-list index; HS tabs only FL/IN/NH/Other → live HS lists = Indiana only in-region | 2026-09-19 23:12:51 |
| `https://www.directathletics.com/upcoming_meets.html` | GET | 200 (32,569 B) | upcoming-meet filters; confirms the page is fed by `fuseDriver_Upcoming.js` | 2026-09-19 23:12:52 |
| `https://www.directathletics.com/scripts/fuseDriver_Upcoming.js` | GET | 200 (354,456 B) | 987 meet objects incl. `hs`, `meet_state_id`, `meet_date` — one-request cross-state meet feed | 2026-09-19 23:13:22 |
| `https://www.directathletics.com/packs/js/application-*.js` | GET | 200 | Rails app bundle; no hidden authenticated JSON API for lists | 2026-09-19 23:13:56 |
| `POST /site_search.html` (`q=Wayzata`, `teams_leagues`) | POST | 200 | team-name search returns `Team\|Sport\|Gender\|Type\|State` + team links; Wayzata = team 6767 | 2026-09-19 23:13:56 |
| `https://www.directathletics.com/teams/track/6767.html` | GET | 200 | Wayzata M T&F: leagues MN + MSHSL 6AA, 4 sibling team records, roster `Name\|Year`, Latest Results | 2026-09-19 23:14:04 |
| `https://www.directathletics.com/leagues/track/114.html` | GET | 200 | Minnesota state HS league = 753 team records with (sport, gender, school, state) | 2026-09-19 23:14:13 |
| `https://www.directathletics.com/leagues/track/1507.html` | GET | 200 | MSHSL 6AA = 30 team records → league granularity below state exists | 2026-09-19 23:14:14 |
| `https://www.directathletics.com/athletes/track/5872948.html` | GET | 200 | athlete fields: name, team, `Class: 22` (stale), Indoor/Outdoor/Best summary, All Results rows | 2026-09-19 23:14:28 |
| `https://www.directathletics.com/leagues/track/115.html`, `/leagues/xc/101.html` | GET | 200 (1.24 MB / 317 KB) | Wisconsin HS universe = 976 T&F + 936 XC team records | 2026-09-19 23:16:10 |
| 23 further `/leagues/{track\|xc}/<ID>.html` pages | GET | 200 | per-state team universes (table in §3); no 404s, no rate limiting | 2026-09-19 23:14–23:17 |
| `https://www.directathletics.com/leagues/xc/123.html` | GET | 200 (17,285 B) | **college** XC league (17 teams) while `track/123` = Indiana HS T&F (929 records) → `(sport, id)` is the league identity; `xc/108` is Indiana HS XC (887 records) | 2026-09-19 23:29:5x |
| `https://www.directathletics.com/results/track/74867.html` | GET | 200 | Iowa **HS** meet "MVC Girls Divisional Meet" (May 5, 2022): event-result index + team scoring, 8 team links | 2026-09-19 23:17:29 |
| `https://www.directathletics.com/results/track/46341.html` | GET | 200 | MN **HS** meet "Lake Conference Championships" (May 17, 2016), 10 teams — 2016 HS data still live on DAT | 2026-09-19 23:17:31 |
| `https://www.directathletics.com/results/track/74867_4545019.html` | GET | 200 | event sheet "Girls High Jump Varsity" `Place\|Overall\|Name\|Year(SR/JR/SO/FR)\|Team\|mark` + athlete links + Preliminaries/Finals heats | 2026-09-19 23:17:37 |
| `https://www.directathletics.com/resultsRankingsFilter.html` | GET | 200 | list-index chooser route (alternate entry to HS lists) | 2026-09-19 23:17:49 |
| `https://www.directathletics.com/legacy_da/results?filterrific[with_states]=WI&with_sports=track` | GET | 200 | state+sport filter on the meet index; identical surface to `/results.html` | 2026-09-19 23:18:02 |
| 12 × `…/legacy_da/results?filterrific[with_states]=<XX>` | GET | 200 | per-state meet counts 2025-08-01…2026-09-19; 8 states single-page (exact), IA/IL/OH/IN paginate | 2026-09-19 23:18:xx |
| `https://www.directathletics.com/meets/track/97055.html` | GET | 200 | Trine HSR #2 (IN): `Types of Entrants: High Schools`, MEET CONTACT present (not transcribed), entry fees | 2026-09-19 23:15:0x |
| `https://www.directathletics.com/meets/xc/28499.html` | GET | 200 | St. Cloud Invitational meet page: meet ids resolve for XC as well | 2026-09-19 23:15:0x |
| `/teams.html`, `/teams/track.html`, `/teams/xc.html`, `/leagues.html`, `/leagues/track.html`, `/leagues/xc.html` | GET | 200 (homepage shell) | **no bulk routes**; unknown paths answer 200 with the homepage — 200 ≠ route exists | 2026-09-19 23:15:1x |
| `https://www.directathletics.com/teams/track/100.html` | GET | **404** | the DAT team id space is sparse; ids are not enumerable by brute force | 2026-09-19 23:15:3x |
| `https://www.directathletics.com/teams/xc/100.html` | GET | 302 → `nh.tfrrs.org` | the same numeric id can mean a different team in another sport/sub-domain → key on (sport, id) | 2026-09-19 23:15:3x |
| `https://www.directathletics.com/teams/xc/5394.html` | GET | 200 | legacy HS XC team page (Wayzata): accurate 2017-22 results, stale numeric class column, "No recent results" | 2026-09-19 23:15:4x |
| `https://www.directathletics.com/robots.txt` | GET | 200 (HTML shell) | no crawl directives published | 2026-09-19 23:16:0x |
| `https://www.directathletics.com/lists/track/1428_5490.html` | GET | 200 (1,210,584 B) | HSR Large School (4A-6A) list; filters `limit\|event_type\|year\|gender`; plain HTML table | 2026-09-19 23:23:5x |
| `https://www.tfrrs.org/athletes/8429704.html` | GET | 200 (312,710 B, `charset=utf-8`) | national sport-less profile resolves the DAT-side id: "EVAN WILLIAMS (SR) — LAWRENCE CENTRAL", state-bests section | 2026-09-19 23:31:xx |
| `https://www.tfrrs.org/athletes/8779512.html` | GET | 200 (264,441 B) | "RYLAN HAINJE (SR) — FRANKLIN CENTRAL" — confirms peer 18's route on a second id | 2026-09-19 23:31:xx |
| `https://www.tfrrs.org/athletes/8215223.html` | GET | **302** → `https://directathletics.com/athletes/track/8215223.html` → 200 (14,046 B) | non-TFRRS athlete ids hand off to the DAT profile ("Isabelle Woody") instead of 404ing | 2026-09-19 23:32:xx |
| `https://www.directathletics.com/lists/track/1428_5490.html?year=JR&limit=500` | GET | 200 (1,337,553 B) | 1,228 JR rows, **1,091 distinct athletes**, 216 team ids — under-counts vs the saturated view | 2026-09-19 23:24 |
| `…lists/track/1428_5490.html?year=JR&limit=1000` and `…&limit=5000` | GET | 200 (1,414,313 B each; sha256 `19e7793e…` vs `6292c5cd…`) | **1,320 rows / 1,155 distinct JR athletes — the year-filtered view saturates at `limit≈1000`.** Rows identical as a multiset (sequence matched too on this pair); the 84 differing bytes are all in `<meta name="csrf-token">`, so exhaustion is proven on parsed-row identity, not on bytes or size | 2026-09-19 23:41, re-diffed offline 2026-09-19 |
| `…lists/track/1428_5490.html?year=JR&limit=1000&gender=m` / `&gender=f` | GET | 200 (804,948 B / 622,649 B) | 665 + 490 = **1,155** = the combined count → the saturated view hides nothing | 2026-09-19 23:42 |
| `…lists/track/1429_5491.html?year=JR&limit=1000` (+ `&gender=m` / `&gender=f`) | GET | 200 (885,685 B / 479,319 B / 419,650 B) | **706 distinct JR athletes** (383 + 323 = 706, again complete), 272 team ids; intersection with Large = 0 → **1,861 total** | 2026-09-19 23:42 |
| saved `cap-1428-JR-1000.html` re-parsed locally | offline (`python3`, no request) | n/a | **season proof:** 1,306/1,306 meet-date cells are `2026` and every grade cell is `JR` → the list is the 2025-26 indoor season, so its `JR` = class of 2027 and its `SR` = class of 2026 | 2026-09-19 |
| `https://www.directathletics.com/lists/track/1429_5490.html` (Small league id + Large list id) | GET | 200 (1,210,584 B) | renders the **Large** list ("HSR Large School (4A-6A) Performance List", 686 athletes) — the league prefix is not validated | 2026-09-19 23:35 |
| `https://www.directathletics.com/lists/track/9999_5490.html` (nonexistent league) | GET | 200 (1,210,584 B) | same Large list again → route cannot be probed by status; merge on list id + rendered title | 2026-09-19 23:35 |
| `https://www.directathletics.com/lists/track/1429_5491.html?year=JR&limit=500` | GET | 200 (877,774 B) | HSR Small School (1A-3A), limit=500 view: 698 athletes, 270 team ids, 0 overlap with large | 2026-09-19 23:24 |
| `https://www.tfrrs.org/results.rss` | GET | 200 | 75 result-set items, newest 2026-09-18/19 — one-request delta feed | 2026-09-19 23:19:57 |
| `https://www.tfrrs.org/sites.html` | GET | 200 | TFRRS sub-domain list (florida, indiana, nh, tf, xc) — Indiana is the only Midwest one | 2026-09-19 23:19:52 |
| `https://www.tfrrs.org/results_search_page.html?…with_month=9&with_year=2026&with_sports=xc` (WI, MN, IL, IN, MO) | GET | 200 | Sept-2026 XC depth: WI 8, MN 7, IL 11, IN 11, MO 9 result sets | 2026-09-19 23:19:2x |
| `https://www.tfrrs.org/results/xc/27822/Southern_Stampede…HS/` | GET | 200 (833 KB) | 1,297 rows with `Place\|name+link\|2027…2030\|team+link\|time`; `TIMING:` provider published | 2026-09-19 23:19:24 |
| `https://www.tfrrs.org/results/xc/28517/Badger_Classic/`, `…/28349/Titan_Fall_Classic/` | GET | 200 | college result sets — the bulk of the Midwest feed is college, not HS | 2026-09-19 23:19:2x |
| `https://www.tfrrs.org/teams/xc/2982.html` | GET | 302 → `directathletics.com/teams/xc/2982.html` | TFRRS team pages are DAT team pages; HS XC roster shows current `FR/SO/JR/SR` | 2026-09-19 23:18:3x |
| `https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i` | GET | 200 (1,880,743 B) | 1,180 rows / 732 athletes / 312 teams; 223 JR rows (210 distinct); archive 2019→2026 on one page | 2026-09-19 23:23:27 |
| `https://indiana.tfrrs.org/list_data/5489?gender=f&year=JR&limit=500` | GET | 200 (1,166,719 B, `text/html`) | fragment endpoint returns 785 JR rows / 687 athlete ids — server-side grade filter | 2026-09-19 23:24:19 |
