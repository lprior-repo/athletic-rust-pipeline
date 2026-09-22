# 17. Indiana IHSAA

Status: complete
Observed on: 2026-09-19

### Source

| Source | Role | URL |
|---|---|---|
| IHSAA (Indiana High School Athletic Association) | Association site: membership, classifications, tournament structure, entry/state-finals PDFs | https://www.ihsaa.org/ |
| IHSAA tournament JSON API | Archive index of every state-tournament node (all sports, all seasons) | https://www.ihsaa.org/api/tournaments |
| TFRRS Indiana (DirectAthletics platform, IHSAA-branded) | Official Indiana HS result/rankings portal: meet results with **grade**, athlete/team profiles, 2023–2025 tournament hub | https://indiana.tfrrs.org/ , https://in.tfrrs.org/tournament.html?year=2025 |
| MileSplit Indiana | Result target for the **2025-26 boys/girls TF** sectionals + regionals | https://in.milesplit.com/meets/735020/results |
| Timing MD / AthleticLIVE | 2026 TF state-finals live results (Athletic.net platform under a timer domain) | https://live.timingmd.net/meets/74720 |
| Hy-Tek MEET MANAGER PDFs on ihsaa.org | Official state-finals performance list / heat sheets / final results (grade + wind) | https://www.ihsaa.org/sites/default/files/documents/2025-26%20BTr%20State%20Results.pdf |
| myIHSAA | Coach/AD/official portal (authenticated) | https://www.myihsaa.net/ |
| IATCCC | Indiana coaches association: officer names, weekly XC polls, All-State PDFs | https://iatccc.org/ |
| EventLink (rSchoolToday) IHSAA instance | Official schedules/tickets/results *search UI*; sport GUIDs | https://ihsaa.eventlink.com/Schedules |

IHSAA site is Drupal 10.6.15 (generator meta + `/core/*` paths). Two IHSAA-side machine-readable
surfaces were found and are new relative to the brief: **`/api/tournaments`** (JSON) and a
**Google My Maps KML export** of the membership map.

### Coverage

* **Geography:** Indiana only (IHSAA jurisdiction). Member schools are grouped into three IHSAA
  districts — District I 132, District II 143, District III 138.
* **Membership (2026-27, verbatim from https://www.ihsaa.org/schools/ihsaa-school-directory):**
  Full Member Schools **408**; Current Provisional Members **5**; **Total 413 (356 public / 57
  non-public)**; Full Members gained 2026-27: 2 (Mooresville Christian, Seven Oaks Classical);
  Provisional gained: 1 (Horizon Christian); 0 consolidations, 0 members lost, 1 name change.
* **Sports:** boys/girls cross country (fall), boys/girls track & field (spring), unified T&F.
  Indoor exists at school level via the **Hoosier State Relays** (TFRRS Indiana carries a
  "2026 HSR Indoor" season, config handle `423`).
* **Class structure:** Indiana **does not class T&F or XC**. The only published enrollment
  classification files are for baseball, boys/girls basketball, football, boys/girls soccer,
  softball and girls volleyball (https://www.ihsaa.org/schools/enrollments-classifications lists
  exactly those eight sports; the shared file is "Four-Class Sport Classifications 2026-27 and
  2027-28"). The T&F/XC state series is **single-class and geographic**.
* **State-series shape (2025-26, parsed from the official pages):**
  * Boys T&F: **32 sectionals → 8 regionals → 1 state final** (May 21 / May 28 / Jun 6, 2026).
  * Girls T&F: **32 sectionals → 8 regionals → 1 state final** (May 19 / May 26 / Jun 5, 2026).
  * Boys & girls XC: **25 sectionals → 5 regionals → 1 state final** (Oct 18 / Oct 25 / Nov 1, 2025).
    Each XC sectional is one combined boys+girls meet (boys and girls pages link the *same* TFRRS
    meet id, e.g. `26194`).
  * 2026-27 XC (season in progress on 2026-09-19) repeats the 25/5 shape with dates
    **Oct 17 / Oct 24 / Oct 31, 2026**; the hosts are published with a per-site *declared* school
    count (391 slots across 25 sectionals) but **no school-name lists and no result links yet**.
* **Historical depth:**
  * `/api/tournaments` returns **91 T&F/XC nodes**: Boys T&F 19, Girls T&F 19, Boys XC 20, Girls XC 20,
    Unified T&F 13 — school years **2008-09 … 2026-27** (Unified: 2013-14 … 2025-26) plus an
    `Archived` node for each of the four varsity sports.
  * The `archived-tournament` node (e.g. `/sports/boys/track-field/archived-tournament`) is a single
    page carrying the older PDF sets; year labels observed: **1996-97 … 2007-08**, with 227 sectional
    PDFs, 59 regional PDFs and 45 state PDFs linked on that one page.
  * TFRRS Indiana keeps "TOURNAMENT" hubs for **2025 / 2024 / 2023** plus per-year All-Indiana
    ranking lists (e.g. list `5328` = 2025 IHSAA All-Indiana Official Rankings).

### Enumeration

**A. Schools (four independent paths, all verified).**

1. *Counts / membership profile* — `GET https://www.ihsaa.org/schools/ihsaa-school-directory`
   (HTML). Yields 408/5/413 as quoted above. No machine-readable table on the page.
2. *Name + coordinate list* — the "Membership Map" link on that page is a Google My Maps id;
   exporting KML works without auth:
   `GET https://www.google.com/maps/d/kml?mid=1YN9MMTXmb_Rf5kyQU4rm81vH1nS-3G4&forcekml=1`
   → **412 `<Placemark>` items, 412 with `<coordinates>`, 0 with descriptions**; map title
   "2025-26 IHSAA Member Schools". One request = whole school name universe with lat/lon.
3. *School + city + county + membership years* — `Membership History.pdf`
   (`/sites/default/files/documents/Membership%20History.pdf`, 526 KB, last modified 2026-09-08):
   "Member Schools by County" pages 5-16 give `School | City/Town | Membership Years`
   (e.g. `Adams Central | Monroe | 1949-present`). Authoritative for **school identity, city and
   current-vs-defunct status**; two-column layout makes exact automated extraction messy (a naive
   parse under-counted; no count from this file is claimed).
4. *School + enrollment by grade* — `Four-Class Sport Classifications 2026-27 and 2027-28.pdf`
   (linked from the enrollments page). The PDF is a two-column table (same school list ranked by
   enrollment, then alphabetically) whose text layer emits one word per line, so recovery is by word
   coordinates — `tools/17-parse-classifications.py` (pdftotext `-bbox-layout`, deterministic).
   Result: **402 school rows recovered per column, 399 of them cross-validated between the two
   columns with zero value mismatches**; 10 row fragments stayed unresolved by the layout, so the true
   list is ~402-410 rows (a cruder text-mode read returned 410 unique names).
   * **Grade 11 (the Class-of-2027 column) sums to 77,661 / 77,691** across the two columns; grade 9
     79,191-79,360, grade 10 78,697-78,765, grade 12 75,071-75,111; total enrollment 310,620-310,927.
     Treat as ≈**77.7k Class-of-2027 enrolment** statewide, ±1% for layout recovery.
   * Class distribution of the same set: 1A 123, 2A 100-101, 3A 98-100, 4A 79-80.
   * Row shape: `Rank School 9th 10th 11th 12th Total Class Prev` (e.g. `1 Carmel 1289 1316 1294 1307
     5206 4A 4A`). Published for the *classed* team sports, but it is the only statewide per-grade
     enrollment table, so it doubles as the school-identity and cohort-denominator source.
5. *State-series participation lists* (who actually fields T&F/XC) — each tournament page lists the
   schools assigned to each sectional, enumerable in one request per sport:
   * Boys T&F 2025-26: 32 sectionals, **394 distinct school names**.
   * Girls T&F 2025-26: 32 sectionals, **392 distinct school names**.
   * Boys XC 2025-26: 25 sectionals, **381 distinct school names**.
   * Girls XC 2025-26: 25 sectionals, **367 distinct school names**.
   * **Union of those four 2025-26 lists = 413 distinct name strings — exactly the declared membership
     count (408 full + 5 provisional).** Use it as a reconciliation invariant, not an id join: the
     lists are name strings (two different "21st Century" spellings appear), and the per-site `(N)`
     header counts drift from the listed names on some XC blocks (declared-header sums 394 / 391 / 364
     / 351 for boys TF / girls TF / boys XC / girls XC) because of markup quirks.
   * 345 of the 413 appear in all four lists; 24 schools field T&F but not XC.
   * 2026-27 boys XC (upcoming, sectionals 2026-10-17): 25 sectionals + 5 regionals published with
     hosts and per-site **declared** counts summing to **391**, but no school-name lists yet.

**B. Meets / tournaments.**

* Archive index: `GET https://www.ihsaa.org/api/tournaments` → **HTTP 200, `application/json`,
  82,358 bytes, 456 array elements** shaped
  `{title, field_school_year, field_sport, view_node}`. Discovered by reading
  `/themes/custom/ihsaa/dist/js/misc/state-tournaments/tournament-years.js` (the page's year
  `<select id="years" data-sport="...">` is populated by `fetch("/api/tournaments")`).
* Season page: `GET /sports/<gender>/<sport>/<season>-tournament?round=<round>` returns the **whole**
  page — all rounds live in `div.tournament-round[data-round=…]` (`sectionals`, `regionals`,
  `state-finals`/`state-finals-`); `?round=` only selects the visible tab (all three variants returned
  ~302 KB and identical content). One request per gender/sport/season yields hosts + school lists +
  result links for every round.
* Parser written and executed for this report:
  `tools/17-parse-tournament.py` (records site number, host, declared school count, ticket/result/
  performance-list links, member school list) and a request-recipe wrapper
  `tools/17-ihsaa-enumerate.py seasons|season <url>|schools`. The enrollment/classification
  PDF has its own coordinate-aware parser, `tools/17-parse-classifications.py`.

**C. Athletes.**
* TFRRS Indiana "list athlete search" JSON: `GET https://indiana.tfrrs.org/list_athlete_search/5328`
  → **HTTP 200, `application/json`, 1,427,981 bytes, 24,634 unique athlete objects**
  `{value:<AthleteID>, text:"Last, First - School"}` across **402 distinct schools** (identical to the
  402 team objects below), in one request.
  The query string is ignored (`?query=…` and `?term=…` returned byte-identical payloads) — it is a
  full-list dump, not a search.
* `GET https://indiana.tfrrs.org/list_team_search/5328` → **402 team objects** `{value:<TeamID>, text:<School>}`.
* Team pages carry the **roster with grade**: `https://indiana.tfrrs.org/teams/xc/<School>_<f|m>.html`
  (e.g. Lake Central Girls XC: NAME + YEAR rows `Bonchik, Alexis JR`, …) and cross-links to
  Boys XC / Boys T&F / Girls T&F pages; a season `<select name="config_hnd">` switches
  season handles (`423` 2026 HSR Indoor, `410` 2025 IHSAA Cross Country, `395` 2025 Outdoor).
* Meet pages: XC `https://www.tfrrs.org/results/xc/<id>` and `https://indiana.tfrrs.org/results/xc/<id>/<slug>`
  are fully server-rendered (one request = whole meet, all athletes, both genders for a sectional);
  T&F `…/results/<meetid>/<slug>` is an event index, `…/results/<meetid>/m/<slug>` is the compiled
  all-events page, `…/results/<meetid>/<eventid>/<slug>/<N>-Meters` is one event.
* State-finals PDFs on ihsaa.org list the qualifiers themselves (name, Yr, school, seed mark, bib).

**D. Class of 2027.** Two independent in-band markers:
* TFRRS result rows carry grade tokens: fall-2025 XC `JR` **= Class of 2027**; spring-2026 T&F `Yr 11`
  **= Class of 2027**. Example row (2025 XC Sectional 1): `1 Macey Thompson JR Lake Central 5:37.1 17:25.0`.
  Athlete pages show the same tag: `MACEY THOMPSON (JR) LAKE CENTRAL`.
* State-finals PDFs: `265 … Korte, Hayden 12 Lakewood Par 6-09.00`, `852 Delaney, Linkin 11 Wood Memoria 14-00.00`.
* Denominator: ≈**77.7k eleventh-graders statewide** (classification PDF grade-11 column, §A4).

**E. Results.** Meet → event → athlete rows as in §C; state finals additionally as Hy-Tek PDF.

### Stable identifiers

| Entity | Identifier | Evidence |
|---|---|---|
| IHSAA tournament node | URL path only, i.e. `/sports/<gender>/<sport>/<season>-tournament`; API field `view_node` | `/api/tournaments` (`view_node: "/sports/boys/cross-country/2026-27-tournament"`) |
| IHSAA sport | `field_sport` string ("Boys Track & Field") | `/api/tournaments` |
| IHSAA school | **none public.** The directory page's only search is `Member School Search → https://www.myihsaa.net/schools` (authenticated portal, `idsrv.myihsaa.net` identity server) | directory page HTML; `GET https://www.myihsaa.net/schools` → HTTP **404** SPA shell |
| EventLink sport | GUID: Cross Country `b7630efd-2350-455a-b82b-6ebbac635204`; Track & Field `d2370f1c-834d-4be3-bf75-17686ceafb96` | sport pages' "Schedules (via EventLink)" hrefs |
| EventLink ticket/event | `https://public.eventlink.com/tickets?t=188456` (regional), `https://ihsaa.eventlink.com/Tickets?c=…&l=Munster+High+School+…` (sectional) | tournament pages |
| TFRRS/DA athlete | numeric `AthleteID` (e.g. `8608546`) in `/athletes/<id>/<School>/<Name>.html` | meet page links; list JSON `value` |
| TFRRS/DA team | numeric `TeamID`; 402 in the series list; per-gender ids in meet filter selects (e.g. `40975` = Calumet (M)) | `list_team_search/5328`; `results/92370/…` team filter |
| TFRRS/DA meet | numeric, gender-scoped for T&F (`92370`), combined for XC (`26194`) | tournament hub + IHSAA pages |
| TFRRS/DA event | numeric (`5664942`) inside meet URL | `…/results/92370/5664942/IHSAA_Sectional_1_Boys/Boys-100-Meters` |
| TFRRS/DA list | numeric (`5328`) — the JSON endpoints key off it | `lists/5328/2025_IHSAA_All_Indiana_Official_Rankings` |
| TFRRS/DA league | numeric (`/leagues/xc/1085.html` = IHSAA Section 1; `/leagues/1662.html` = IHSAA Regional 1) | team page league links |
| TFRRS/DA season | config handle option value (`423`, `410`, `395`) in `select[name=config_hnd]` | team page |
| MileSplit | numeric meet id (`735020`), URL `/meets/<id>/results` | IHSAA 2025-26 T&F sectional links |
| Hy-Tek PDF | bib number + 4-letter school code (`852`, `BHSN`) — timer-assigned, not a school registry | state performance list / results PDF |
| Athletic.net | **not exposed by any page fetched** (0 occurrences across 21 official pages) | see Athletic.net leverage |

### Athletic.net leverage

* **Direct links: none.** A case-insensitive scan for `athletic.net` over every page fetched from
  ihsaa.org, indiana.tfrrs.org/tfrrs.org, in.milesplit.com and iatccc.org returned **0 occurrences**.
  Indiana's official pages link **MileSplit** (T&F) and **TFRRS** (XC / earlier T&F), never Athletic.net.
* **The result host for the T&F state series changed between the last two seasons** (both verified by
  counting anchors on the official tournament pages):
  * `2024-25` boys T&F (`/sports/boys/track-field/2024-25-tournament`): **40 distinct TFRRS meet
    ids** (`indiana.tfrrs.org/results/<id>`, span 92370-92469) across 32 sectionals + 8 regionals.
  * `2025-26` boys T&F: **40 distinct MileSplit meet ids** (`in.milesplit.com/meets/<id>/results`,
    span 734902-735020) + 8 "Performance Lists" links; girls 2025-26 spans 734888-735113 (40 ids).
  * `2025-26` XC: **31 distinct TFRRS XC ids** (`tfrrs.org/results/xc/26194…26249`: 25 sectionals
    26194-26219, regionals 26220/26221/26223/26224/26225, state final 26249).
  * Consequence for Main: the same round is addressable from **two** id spaces depending on season;
    key postseason rows by (season, gender, sport, round, site) and store whichever provider id the
    official page carries.
* **Indirect Athletic.net presence:** the 2026 T&F **state-finals live results** are served by
  AthleticLIVE at `https://live.timingmd.net/meets/74720`, and that app loads
  `https://livestatic.athletic.net/main-5ADGVJIV.js` and `polyfills-D2W4DALN.js` — i.e. Athletic.net
  infrastructure behind a timing-company domain. `livestatic.athletic.net` returned **HTTP 200** from
  this machine (2 sub-resources fetched) while `www.athletic.net` is Cloudflare-403 here. The
  AthleticLIVE data API was not located (runtime config only exposed Google Analytics ids); the
  meet page itself is a JS shell with no server-rendered results.
* **Deterministic seeding:** TFRRS Indiana supplies `(athlete name, school, gender, grade, season,
  event, mark)` — enough to drive one specific Athletic.net athlete lookup per candidate instead of
  any broad Athletic.net enumeration. No Athletic.net URL/TeamID/MeetID is exposed, so the join must
  go through name+school(+grade) reconciliation.
* **Request-avoidance estimate (measured inputs, [INFERENCE] on the mapping):**
  * Baseline to beat: the retained Athletic.net Grade-11 delivery
    (`~/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx`, 142,705 rows) contains
    **2,681 rows with `States = IN`** (column E; counted by streaming the sheet) — the current known
    Indiana Grade-11 boys cohort from Athletic.net, plus an unquantified girls cohort.
  * TFRRS cost to cover a full Indiana season: **1 request → 24,634 athlete ids + 402 schools**;
    **1 request → 402 team ids**; team rosters with grade ≈1 request per (school, gender, sport)
    ≈ 400–1,600; full results ≈ **508 meet pages** (distinct meet ids referenced by the 2025
    All-Indiana rankings page) + **31 XC state-series pages**. Order of 550–650 page fetches covers
    an Indiana season's results **with grade**, versus ~2,681 (boys alone) Athletic.net profile
    acquisitions through the browser session. Net: Indiana is a state where a non-Athletic.net
    result source can plausibly replace most Athletic.net profile work for the 2025-26 cohort, and
    can *seed* the remaining Athletic.net identities cheaply.

### Athlete evidence

| Field | Availability | Evidence |
|---|---|---|
| name | yes | TFRRS meet rows/profiles; Hy-Tek PDFs |
| graduating class/grade | **yes, in-band** | TFRRS `YEAR` column (`JR`,`SO`,`FR`,`SR`) and profile header `(JR)`; Hy-Tek `Yr` column (`11`,`12`) |
| school | yes | TFRRS `TEAM` column, team slug, list JSON `text`; Hy-Tek school |
| city/state | partial | TFRRS team page shows state (`Indiana`) and league/sectional; **city not shown** on the pages fetched — city comes from the IHSAA Membership-History PDF, or MileSplit's team index per agent 18 |
| gender/category | yes | separate boys/girls pages, team slugs `_m`/`_f` |
| TF/XC distinction | yes | URL space `/results/xc/<id>` vs `/results/<id>`; separate team pages per sport |
| indoor/outdoor | yes | TFRRS seasons `2026 HSR Indoor` / `2025 Outdoor` / `2025 IHSAA Cross Country`; indoor meet pages exist (e.g. `results/94159` Hoosier State Relays Finals) |
| performances | yes | per-event rows; XC rows carry mile split + finish |
| PRs | yes | athlete page "IHSAA Bests" block (`5K (XC) 17:18.4`, `3.1 MILE (XC) 17:25.0`) |
| progression | yes | athlete page sections `MEET RESULTS / EVENT HISTORY / SEASON HISTORY / PROGRESSION`; history back to 2023 for the sampled athlete |
| meets | yes | each performance row lists meet name + date + place |
| athlete profile URL | yes | `https://indiana.tfrrs.org/athletes/<id>/<School>/<Name>.html` |

Gap: **regular-season 2026 T&F results are thin in TFRRS** (the current-season corpus is small; the
2026 T&F postseason result links point at MileSplit, whose performance data is subscription-gated per
agent 18). For the 2025-26 cohort the rich layer is TFRRS (XC 2025 + T&F 2025 + 2026 indoor);
for the 2026 T&F regular season, MileSplit is the official pointer.

### Recruiting information

**Verdict: no statewide public coach/AD directory exists on the IHSAA side.** Details, with the
retention rule applied (name + role + school + published professional contact only; nothing else):

* `https://www.ihsaa.org/schools/ihsaa-school-directory` — the only directory affordance is
  "Member School Search → **Visit myIHSAA.net**" (`https://www.myihsaa.net/schools`, HTTP **404** SPA
  shell); the portal is identity-server backed (`https://idsrv.myihsaa.net/documents/myIHSAA%20Privacy%20Policy.pdf`)
  and the site footer addresses it to "Coaches, officials, and Administrators". Public coach/AD
  records therefore sit **behind authentication** — out of contract, not attempted.
* `https://www.ihsaa.org/schools/games-wanted` — postings are "posted and maintained by IHSAA-member
  schools **on the myIHSAA website**"; the only public contact route is a Cloudflare-obfuscated email
  link to the IHSAA Sports Information Director. No public per-school AD contact.
* `https://www.ihsaa.org/schools/athletic-conferences` — conference membership in school names only
  (no coach/AD fields, no websites).
* IATCCC (coaches association): `https://iatccc.org/officers-and-council/` publishes **officer names +
  schools** (2025-26: Paige Brunner, Oak Hill; Jared Turner, Yorktown; Andy Belloli, Fishers;
  Michael Clements, Penn) and the association homepage publishes one **public professional email**
  (`jalano@hse.k12.in.us`) as its contact. Weekly XC polls are team-level only (school names);
  All-State lists are PDFs by year. → useful for *contacts of record for the sport*, not a roster of
  400 coaches.
* School/district sites — sampled four fetches; result: **names yes, individual emails mostly no**:
  * `https://penn.phmschools.org/athletics/athletic-department-staff/` → "Athletic Director Jeff Hart,
    Assistant Athletic Director Bridget Williams, Assistant Athletic Director Marie Doan,
    Athletic Department Secretary Jennifer Dunderman" + department phone numbers; the only email on
    the page is a generic `info@phm.k12.in.us`.
  * `https://www.hseschools.org/about/staff-directory` → 0 emails in HTML (JS directory).
  * `https://fhs.hseschools.org/` → 0 emails (links to `/student-life/athletics`).
  * `https://www.phmschools.org/about-us/district-overview/penn-harris-madison-contacts/` → 0 emails.
  * Domain shape observed: staff mail domain `hse.k12.in.us` (from the IATCCC mailto) while the
    district *web* domain is `hseschools.org`; `www.hse.k12.in.us` does **not resolve** (DNS failure).
    [INFERENCE] Indiana districts commonly publish `<district>.k12.in.us` staff addresses, but this
    was not confirmed with a coach-level example in this slice — hand this to agent 29, who owns the
    cross-state coach graph.
* TFRRS host pages contain **no coach fields** (0 case-insensitive `coach` matches in the meet page,
  team page, rankings list page and tournament hub, apart from a meet named after a coach).

Practical recipe for agent 29 if it needs Indiana coaches: IHSAA gives no bulk path → per-school
lookups on district sites (≈400 fetches) plus `iatccc.org` for sport-level contacts of record;
expect names and department phones, treat individual emails as the exception, and never ingest
anything beyond role-scoped professional contact.

### Result evidence

| Field | Status | Evidence |
|---|---|---|
| ResultID | **no dedicated id** — rows are positional; stable composite key = meet id + event id + athlete id (TFRRS) or meet + event + bib (Hy-Tek) | TFRRS rows carry no id attribute; links exist for athlete/team only |
| AthleteID | yes | XC meet page links `…/athletes/8608546/Lake_Central/Macey__Thompson.html` |
| MeetID | yes | TFRRS `26194` (XC) / `92370` (T&F); MileSplit `735020` |
| EventID | yes (TFRRS T&F) | `…/results/92370/5664942/IHSAA_Sectional_1_Boys/Boys-100-Meters` |
| mark | yes | XC: mile split + finish (`5:37.1`, `17:25.0`); T&F compiled: several `TIME` columns (FAT splits); PDFs: `10.50` |
| normalized mark inputs | partial | T&F pages expose multiple unnamed `TIME` columns (splits) with no unit/segment header; PDFs give the canonical mark. Wind not present on TFRRS compiled/event pages |
| timing method | yes (prose + artifact) | IHSAA advancement text requires FAT for advancement; PDFs stamped "Timing MD - Contractor License … Hy-Tek's MEET MANAGER"; state-finals live feed is AthleticLIVE |
| wind | yes in PDF, no on TFRRS | PDF: `1 McMillian, Zarion 12 Anderson Hig 10.50Q2.8` (wind +2.8 appended); not observed in TFRRS HTML |
| implement/hurdle specification | not observed | not present on TFRRS compiled pages or PDFs sampled (only event names). [UNVERIFIED] |
| heat/round | yes | TFRRS: separate `Preliminaries`/`Finals` tables per event (38 `<thead>` blocks in one compiled page) and qualifier codes (`Q`, `q`) in PDFs |
| place | yes | `PL` column; XC `1st/2nd`; PDF `Place` |
| date | yes | meet page header (`May 22, 2025`, `Oct 18, 2025`) |
| school represented | yes | `TEAM` column incl. gender suffix in meet filter (`Calumet (M)`) |
| relay membership | yes | relay events listed as events (`4 x 100 Relay`, `4x400`); PDF has `Boys 4x100 Meter Relay` sections with member lists; agent 18/05 should treat relay athlete links as the join |

### Incremental use

* **New meets:** `GET https://indiana.tfrrs.org/results.rss` → HTTP 200, `application/xml`, 16,550
  bytes, **75 items**, each `{title, description=<meet date>, link}`. Observed items span
  **Dec 2025 → Aug 22, 2026** (mixed T&F/XC: 57 T&F, 18 XC of 75). Sorted roughly newest-first, and
  it is a **capped 75-item window** — diff against stored `link`s each week; do not assume it is the
  complete index.
* **New/updated tournament nodes:** `GET https://www.ihsaa.org/api/tournaments` (456 nodes, one
  request, includes `field_school_year` + `view_node`) — a new season appears as a new node; the
  current-season node grows results links as rounds are completed, so a weekly re-read of one
  tournament page per gender/sport is enough to detect new rounds.
* **New results for a known meet:** re-fetch that meet only — XC meet pages are single-shot
  (`results/xc/<id>/<slug>` = whole meet), T&F compiled page (`results/<meetid>/m/<slug>`) = whole
  gender's events; no `ETag`/`Last-Modified` observed on the responses sampled, so change detection is
  by content hash or by "row count grew".
* **Affected athletes:** candidate-keyed; the cheapest identity refresh is the per-season roster
  layer (team page per school/gender/sport, year filter) or the per-season list JSON.
* **Anti-pattern to avoid:** re-crawling `/lists/5328/…` (4.4 MB page embedding ~7,400 result links)
  weekly; that page is a bootstrap, not a delta feed.
* Do **not** build a weekly job on `in.tfrrs.org/tournament.html` — `?year=2026` returned
  **HTTP 500** on 2026-09-19 (2025/2024/2023 work).

### Access characteristics

* **https://www.ihsaa.org** — *normal HTML* + **public structured JSON** (`/api/tournaments`,
  `application/json`) + *downloadable PDF*. Cloudflare is in front (pages contain `cdn-cgi/content`
  and Cloudflare email-protection markup) but no challenge, 403 or 429 was observed in ~27 requests
  from this machine. `robots.txt` disallows only `/core/`, `/profiles/`, `/admin/`, `/search/`,
  `/user/*`, `/comment/reply/`, `/filter/tips`, `/node/add/`, oembed paths — none of the endpoints
  used here. No published rate limit; sequential 1.0-1.5 s spacing used throughout.
* **https://indiana.tfrrs.org / www.tfrrs.org / in.tfrrs.org** — *normal HTML* + *public JSON*
  (`list_athlete_search`, `list_team_search`) + *RSS*; no authentication needed for any surface read.
  One 500 (`tournament.html?year=2026`); three 404s (`/teams/xc/`, `/teams.html`, `/results.html`);
  `results_search.html` returns HTTP 200 with a "Not Found (404)" body (their catch-all).
* **https://ihsaa.eventlink.com** (rSchoolToday EventLink) — *browser application*: Schedules and
  Results are JS search forms keyed on ZIP + radius + date + sport + level; the same URL with query
  parameters returned identical, empty shells, so results are fetched client-side. `/Schools` is 404;
  `/SchoolSchedules/Schools/<GUID>` uses the **sport** GUID, not a school id.
* **https://in.milesplit.com/meets/<id>/results** — *browser application* ("Meet Manager", body
  renders "Loading"; 45 KB shell, no result rows, no Athletic.net links in HTML).
* **https://live.timingmd.net/meets/74720** — *browser application* (AthleticLIVE on
  `livestatic.athletic.net` assets). `www.timingmd.net` does **not resolve** (DNS failure).
* **https://www.myihsaa.net** — *authenticated* (Angular SPA, routes include `schools`,
  `athletic-directors`, `contacts`; identity server at `idsrv.myihsaa.net`). Not attempted.
* **Rate-limit observations:** ~60 requests total across hosts; **zero 429s, zero `Retry-After`
  headers** observed; no published limits. Failures observed: 1× HTTP 500 (in.tfrrs.org), 4× 404
  (myihsaa SPA path, indiana.tfrrs.org index paths, eventlink `/Schools`), 2× DNS failure
  (`www.timingmd.net`, `www.hse.k12.in.us`), 1× HTTP 202 anti-bot on a search engine (see gaps).

### Recommendation

**RESULT-SOURCE — primary for Indiana** (with `ihsaa.org` itself classified DISCOVERY-ONLY and the
TFRRS roster layer serving as ATHLETIC.NET-SEED). No COACH-DIRECTORY capability exists in this lane.

Reasoning and expected marginal coverage:

1. **Official spine (DISCOVERY-ONLY, ~40 requests/season):** `/api/tournaments` + one tournament page
   per gender/sport/season gives the authoritative school universe, the exact postseason site map
   (32→8→1 T&F, 25→5→1 XC) and the provider meet-id for every round, plus school lists that double as
   participation truth for reconciliation. Nothing else in the state gives this cheaply.
2. **Result + identity layer (RESULT-SOURCE):** TFRRS Indiana returns per-meet rows carrying
   `place, name, grade, school, mark` and stable `AthleteID`/`TeamID`/`MeetID`/`EventID`, with
   per-season rosters that list every athlete's year. One request yields the whole 2025 outdoor
   athlete index (24,634 ids / 402 schools). For the Class-of-2027 target this is a direct hit:
   fall-2025 `JR` and spring-2026 `Yr 11` **are** the cohort.
3. **Marginal coverage over Athletic.net:** the retained Athletic.net delivery holds only **2,681**
   Indiana Grade-11 boys; a single TFRRS season covers ≈508 T&F meets + 31 XC state-series meets with
   grade, i.e. roughly an order of magnitude more Indiana athlete-meet rows than the Athletic.net
   Grade-11 slice, for hundreds of requests instead of thousands of browser-session profile pulls —
   and it is reachable without the Cloudflare friction that blocks `www.athletic.net` from this box.
4. **Watch-outs:** (a) the T&F postseason provider flips between seasons (TFRRS 2024-25 → MileSplit
   2025-26) — store both id spaces; (b) MileSplit performance data is subscription-gated; (c) the
   TFRRS 2026 regular-season corpus is thin so far (one 2026 XC meet in the RSS as of 2026-09-19);
   (d) TFRRS exposes no coach data and no Athletic.net ids — grade+school+name is the join key.

### Evidence appendix

All requests below were issued from this machine on 2026-09-19 (America/Chicago) with
`curl/8.7.1 (research)` UA, sequentially, 1.0-1.5 s apart per host. Statuses are as reported by curl.
Local copies of every response were kept under `/tmp/ihsaa/` for the duration of the session; nothing
was written outside `…/midwest-tfxc-source-research/`.

| URL | method | HTTP | what it proved | timestamp |
|---|---|---|---|---|
| https://www.ihsaa.org/ | GET | 200 | Drupal 10.6.15 site; nav to Sports/Schools/Resources; no Athletic.net anywhere | 2026-09-19 23:12 |
| https://www.ihsaa.org/robots.txt | GET | 200 | only `/core/`, `/profiles/`, `/admin/`, `/search/`, `/user/*` disallowed; no crawl-delay, no API exclusions | 2026-09-19 23:12 |
| https://www.ihsaa.org/sitemap.xml (+?page=1,2) | GET | 200 | 2,255 `<loc>` entries; only 32 `/sports/*` and 6 `/schools/*` pages — no per-school pages exist | 2026-09-19 23:12 |
| https://www.ihsaa.org/schools/ihsaa-school-directory | GET | 200 | 408 full + 5 provisional = 413 members (356 public/57 non-public); districts 132/143/138; directory search delegates to myIHSAA | 2026-09-19 23:13:04 |
| https://www.ihsaa.org/schools/enrollments-classifications | GET | 200 | classifications published for 8 team sports only → **T&F/XC are not classed** | 2026-09-19 23:13:06 |
| https://www.ihsaa.org/sports/boys/track-field | GET | 200 | 2025-26 series dates, EventLink sports GUID, `?round=` links, IATCCC link, history/records links | 2026-09-19 23:13:09 |
| https://www.ihsaa.org/sports/boys/cross-country | GET | 200 | 2026-27 series dates + `?round=` links; "All-Time Semi-State Championships" history link | 2026-09-19 23:13:10 |
| https://www.ihsaa.org/sports/boys/track-field/2025-26-tournament?round=sectionals\|regionals\|state-finals- | GET ×3 | 200, 200, 200 | identical ~302 KB body: all rounds in `div.tournament-round`; 32 sectionals, 8 regionals; state-finals = PDFs + `live.timingmd.net/meets/74720` | 2026-09-19 23:13:35-38 |
| https://www.ihsaa.org/api/tournaments | GET | 200 (application/json, 82,358 B) | 456 tournament nodes; 91 T&F/XC nodes across 2008-09…2026-27 + Archived; `view_node` URL per season | 2026-09-19 23:14:31 |
| https://www.ihsaa.org/sports/girls/track-field/2025-26-tournament (+/sports/girls/track-field) | GET | 200, 200 | girls series = 32 sectionals/8 regionals (May 19 / May 26 / Jun 5 2026); **40 distinct MileSplit ids spanning 734888-735113** | 2026-09-19 23:14:37 |
| https://www.ihsaa.org/sports/boys/cross-country/2025-26-tournament | GET | 200 | XC 2025: **31 distinct TFRRS XC ids 26194-26249** (25 sectionals 26194-26219, regionals 26220/26221/26223/26224/26225, state 26249); 381 distinct school names in the sectional lists | 2026-09-19 23:14:38 |
| https://www.ihsaa.org/sports/girls/cross-country/2025-26-tournament | GET | 200 | girls XC links the **same** TFRRS meet ids as boys (combined meets); 367 distinct school names | 2026-09-19 23:14:40 |
| https://www.ihsaa.org/sports/boys/track-field/2016-17-tournament | GET | 200 | pre-switch format: 32 sectionals/8 regionals link **ihsaa.org PDFs** (e.g. `201617ValparaisoSectional.pdf`) | 2026-09-19 23:14:42 |
| https://www.ihsaa.org/sports/boys/track-field/archived-tournament | GET | 200 | archive node: year labels 1996-97…2007-08 (12 labels); 227 sectional + 59 regional + 45 state PDF links (336 distinct PDFs) | 2026-09-19 23:14:43 |
| https://www.ihsaa.org/sports/boys/track-field/2024-25-tournament | GET | 200 | 2024-25 sectionals/regionals link **TFRRS**: **40 distinct ids 92370-92469** → confirms the provider switch | 2026-09-19 23:25:23 |
| https://www.ihsaa.org/sports/boys/cross-country/2026-27-tournament | GET | 200 | current season: 25 sectionals / 5 regionals with hosts + per-site declared counts (sum 391); **no school-name lists, no result anchors** | 2026-09-19 23:23:46 |
| https://www.ihsaa.org/schools/athletic-conferences | GET | 200 | conference lists in school names only; no coach/AD data | 2026-09-19 23:18:39 |
| https://www.ihsaa.org/schools/games-wanted | GET | 200 | postings live on myIHSAA; public contact is a Cloudflare-obfuscated email to the IHSAA SID | 2026-09-19 23:21:32 |
| https://www.ihsaa.org/sites/default/files/documents/Membership%20History.pdf | GET | 200 (526,651 B) | all-time member schools by county with City/Town + membership years (`School \| City \| Years`) | 2026-09-19 23:18:43 |
| https://www.ihsaa.org/sites/default/files/documents/Four-Class%20Sport%20Classifications%202026-27%20and%202027-28.pdf | GET | 200 (180,145 B) | per-grade enrollment table (two columns, same list): 402 rows recovered per column, 399 cross-validated with 0 mismatches; grade-11 (Class of 2027) 77,661-77,691; total 310,620-310,927; classes 1A 123 / 2A 100-101 / 3A 98-100 / 4A 79-80 (via `tools/17-parse-classifications.py`) | 2026-09-19 23:19:11 |
| https://www.google.com/maps/d/kml?mid=1YN9MMTXmb_Rf5kyQU4rm81vH1nS-3G4&forcekml=1 | GET | 200 (text/xml, 109,360 B) | "2025-26 IHSAA Member Schools": 412 placemarks, all with coordinates | 2026-09-19 23:18:43 |
| https://www.ihsaa.org/sites/default/files/documents/2025-26%20BTR%20State%20Performance%20List.pdf | GET | 200 (290,950 B) | Hy-Tek performance list: `bib Name Yr School seed mark` (grade + school + seed) per event | 2026-09-19 23:22:35 |
| https://www.ihsaa.org/sites/default/files/documents/2025-26%20BTr%20State%20Results.pdf | GET | 200 (503,427 B) | Hy-Tek final results: `PL Name Yr School time+WIND` (e.g. `10.50Q2.8`), prelims/finals, team scores; "Timing MD - Contractor License" | 2026-09-19 23:22:39 |
| https://www.myihsaa.net/ | GET | 200 | Angular SPA shell; identity server referenced via `/documents/myIHSAA%20Privacy%20Policy.pdf` on `idsrv.myihsaa.net` | 2026-09-19 23:13 |
| https://www.myihsaa.net/schools | GET | **404** | the IHSAA directory's only search target is not publicly served | 2026-09-19 23:13 |
| https://www.myihsaa.net/main-OYVSLILK.js | GET | 200 | SPA routes include `schools`, `athletic-directors`, `contacts` → coach/AD data is behind login | 2026-09-19 23:16 |
| https://www.tfrrs.org/results/xc/26194 | GET | 200 (182,765 B) | XC sectional: 282 athlete links; rows carry grade (`JR`); 33 team pages | 2026-09-19 23:15:45 |
| https://indiana.tfrrs.org/ | GET | 200 | portal title "TFRRS \| IHSAA Track & Field Rankings and Meet Results"; latest results include 2026 indoor + 2026 XC | 2026-09-19 23:16:04 |
| https://indiana.tfrrs.org/athletes/8608546/Lake_Central/Macey__Thompson.html | GET | 200 | profile: name, **(JR)**, school, IHSAA bests, meet history 2023→2025, season history, progression | 2026-09-19 23:16:05 |
| https://indiana.tfrrs.org/teams/xc/Lake_Central_f.html | GET | 200 | roster `NAME + YEAR` per athlete; season select (`410` 2025 XC, `423` 2026 HSR Indoor, `395` 2025 Outdoor); league links | 2026-09-19 23:16:07 |
| https://indiana.tfrrs.org/results.rss | GET | 200 (application/xml, 16,550 B) | 75-item meet feed (57 T&F / 18 XC), newest 2026-08-22 → incremental meet discovery | 2026-09-19 23:16:34 |
| https://indiana.tfrrs.org/teams/xc/ ; /teams.html ; /results.html | GET ×3 | **404** | no browsable team/meet index at those paths | 2026-09-19 23:16:35-37 |
| https://indiana.tfrrs.org/results_search.html | GET | 200 (page body = "Not Found (404)") | search UI moved; the surviving search inputs are `team_search`, `conference_search` | 2026-09-19 23:16 |
| https://indiana.tfrrs.org/leagues/xc/1085.html | GET | 200 | league page for "IHSAA Section 1" (sectional as a league entity) | 2026-09-19 23:16:58 |
| https://indiana.tfrrs.org/lists/5328/2025_IHSAA_All_Indiana_Official_Rankings | GET | 200 (4,441,959 B) | official 2025 outdoor rankings page; embeds links to **508 distinct meet ids**; event list 100m…Discus incl. 1600/3200 | 2026-09-19 23:19:05 |
| https://indiana.tfrrs.org/list_athlete_search/5328 (?query=/&term=) | GET ×2 | 200 (application/json, 1,427,981 B, byte-identical) | **24,634 athlete objects** `{value:<AthleteID>, text:"Last, First - School"}` over **402** schools; query ignored | 2026-09-19 23:20:48 |
| https://indiana.tfrrs.org/list_team_search/5328 | GET | 200 (15,736 B) | **402 team objects** `{value:<TeamID>, text:<School>}` | 2026-09-19 23:20:57 |
| https://indiana.tfrrs.org/archives.html | GET | 200 | T&F historical rankings/performance lists by year | 2026-09-19 23:19:07 |
| https://indiana.tfrrs.org/indoor_lists.html | GET | 200 | indoor categories: HSR (All Schools), HSR Large, HSR Small | 2026-09-19 23:24 |
| https://in.tfrrs.org/tournament.html?year=2025 | GET | 200 | IHSAA tournament hub: 32+32 sectional and 8+8 regional TFRRS meet links, state qualifiers, performance-list PDFs | 2026-09-19 23:19:09 |
| https://in.tfrrs.org/tournament.html?year=2026 | GET | **500** | no 2026 hub; server error (published today: no TFRRS tournament page for 2026) | 2026-09-19 23:23:45 |
| https://indiana.tfrrs.org/results/92370/IHSAA_Sectional_1_Boys | GET | 200 | T&F meet index: date, host, team filter with **numeric team ids**, event list with event ids | 2026-09-19 23:21 |
| https://indiana.tfrrs.org/results/92370/m/IHSAA_Sectional_1_Boys | GET | 200 (395,915 B) | compiled all-events results: 375 rows, 316 with grade; headers `PL NAME YEAR TEAM TIME… SC` | 2026-09-19 23:21:21 |
| https://indiana.tfrrs.org/results/92370/5664942/IHSAA_Sectional_1_Boys/Boys-100-Meters | GET | 200 | single-event page: 44 graded rows, multiple TIME (split) columns, prelims/finals tables | 2026-09-19 23:21 |
| https://indiana.tfrrs.org/results/94159/Hoosier_State_Relays_Finals_Large_Schools | GET | 200 | indoor (HSR) meets are in the same system with the same event-index structure | 2026-09-19 23:24:40 |
| https://ihsaa.eventlink.com/Schedules?...searchSport=d2370f1c-… | GET | 200 | schedule search UI (ZIP/radius/date/sport/level); sport GUIDs; no school ids | 2026-09-19 23:13 |
| https://ihsaa.eventlink.com/Results?... (with params) | GET | 200 (identical shell) | results are client-side; no server-rendered result data | 2026-09-19 23:14 |
| https://ihsaa.eventlink.com/Schools ; https://ihsaa.eventlink.com/ | GET | **404** ; 200 | no public school directory; root redirects to Tickets | 2026-09-19 23:13 |
| https://in.milesplit.com/meets/735020/results | GET | 200 (45,158 B) | the official 2025-26 T&F sectional result target is a JS "Meet Manager" page: body "Loading", 0 result rows, **0 Athletic.net references** | 2026-09-19 23:23 |
| https://live.timingmd.net/meets/74720 | GET | 200 (50,184 B) | state-finals live results = AthleticLIVE; loads `livestatic.athletic.net/main-5ADGVJIV.js` | 2026-09-19 23:22:40 |
| https://livestatic.athletic.net/main-5ADGVJIV.js | GET | 200 (295,358 B) | AthleticLIVE is Athletic.net infrastructure; references `https://live.athletic.net/meets/`; no REST path exposed in the bundle | 2026-09-19 23:22:56 |
| https://www.timingmd.net/ | GET | **DNS failure (curl 6)** | the timer's marketing domain does not resolve; only `live.timingmd.net` responds | 2026-09-19 23:22 |
| https://iatccc.org/ | GET | 200 | coaches association; public professional contact email `jalano@hse.k12.in.us`; 2026 clinic/All-Star/All-State pages | 2026-09-19 23:18:08 |
| https://iatccc.org/officers-and-council/ | GET | 200 | 2025-26 officers with names + schools (Brunner/Oak Hill, Turner/Yorktown, Belloli/Fishers, Clements/Penn) | 2026-09-19 23:18 |
| https://iatccc.org/awards/all-state/ | GET | 200 | All-State lists as PDFs by year (state meet top-9 per event) | 2026-09-19 23:18 |
| https://iatccc.org/2026/09/14/boys-cross-country-poll-9-14/ | GET | 200 | weekly XC polls are **team-level** (school names decoded from a short list), not athlete rows | 2026-09-19 23:18:18 |
| https://penn.phmschools.org/athletics/athletic-department-staff/ | GET | 200 (272,275 B) | AD + assistant AD **names** and department phones; only generic `info@phm.k12.in.us` email | 2026-09-19 23:24:46 |
| https://www.phmschools.org/ , /about-us/…/penn-harris-madison-contacts/ , https://penn.phmschools.org/ | GET ×3 | 200 | district/school sites resolve; sampled pages publish 0 individual staff emails | 2026-09-19 23:24 |
| https://www.hseschools.org/ , /about/staff-directory , https://fhs.hseschools.org/ | GET ×3 | 200 | district + school sites resolve; staff directory page contains 0 emails in HTML | 2026-09-19 23:24 |
| https://www.hse.k12.in.us/ | GET | **DNS failure (curl 6)** | the staff **mail** domain (from the IATCCC email) is not the district web domain | 2026-09-19 23:18 |
| https://html.duckduckgo.com/html/?q=… | GET | **202 (no results)** | search-engine fallback unavailable from this machine (anti-bot); no web-search corroboration was obtainable | 2026-09-19 23:18 |

**Tool-level failure (reported verbatim, per brief rule):** the `web_search` tool returned
`Sign up and repeat your request.` on the single call attempted (2026-09-19 23:17). I stopped using
that tool and completed all corroboration with direct HTTP fetches to primary sources, which is what
the evidence standard requires anyway. Nothing in this report depends on secondary web results.

**Read-only inputs used (not written):**
`/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx` → 142,705 rows total,
**2,681 rows with `States = IN`** (column E, streamed count; column headers `AthleteID | Names |
GradeID | Teams | States | Events | Result Count | Other Observed Grades | Identity Match Status`).
The repository under `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline` was read only to confirm
it contains no Indiana-specific Athletic.net mapping beyond a state-code table
(`.worktrees/authorized-alpha-continued/src/{alpha_url.rs,extract.rs}` map `"IN" → "Indiana"`).

**Unproven / open items** (explicit, so Main does not treat them as settled):
1. Whether 2026-27 XC results will again come from TFRRS — the 2026-27 XC page carries no result
   anchors as of 2026-09-19 (sectionals are 2026-10-17). [INFERENCE: TFRRS, as in 2025-26.]
2. The AthleticLIVE data API (would give live/structured results for the TF state finals) — not
   located; runtime config exposed only analytics ids.
3. MileSplit's meet payload API for the 2026 T&F sectional/regional targets — owned by agent 18.
4. Whether IHSAA/EventLink expose a public numeric **school id**: none observed on any public page;
   the only directory with school records is authenticated myIHSAA. Absence here is an observation,
   not proof of non-existence.
5. Individual coach e-mail availability at scale (only 4 school/district pages sampled, all negative
   for individual addresses) — agent 29 owns the cross-state answer.
6. Implement/hurdle specifications and wind on TFRRS pages were not observed; PDFs carry wind only.
