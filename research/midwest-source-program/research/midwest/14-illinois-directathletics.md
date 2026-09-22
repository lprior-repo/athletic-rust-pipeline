# 14. Illinois DirectAthletics

Status: complete
Observed on: 2026-09-19

Scope note: everything below was fetched live from `directathletics.com` (53 sequential requests,
~1.5 s apart, 23:12–23:38 America/Chicago, zero 4xx/5xx) plus three requests to `tfrrs.org` hosts
(two 200s, one DNS NXDOMAIN). The mission's "athletic-local / cache-cutover / evidence-repairs / fixture-native"
profiles and the pipeline repo were not touched. No authenticated, CAPTCHA, or paywalled surface
was used; the one page that requires login is flagged as such.

---

### Source

**Provider:** DirectAthletics, Inc. — `https://www.directathletics.com/` (HTTP/2 200, server
`nginx/1.21.6`, fronted by CloudFront: `via: 1.1 ...cloudfront.net`, `x-cache: Hit from cloudfront`).
It is the same company as **TFRRS** (`https://www.tfrrs.org/`), which shares the meet/result data
model; DA is the entry/registration + HS side of the platform, TFRRS is the collegiate results side.

Surface map actually observed (every pattern below was fetched or extracted from a fetched page):

| Purpose | URL pattern | Example |
|---|---|---|
| Results index (server-rendered, filterable) | `/legacy_da/results?filterrific[with_states]=..&filterrific[with_sports]=track\|xc&filterrific[search_query]=..&filterrific[with_date_from]=YYYY-MM-DD&filterrific[with_date_to]=YYYY-MM-DD&page=N` | `?filterrific[with_states]=IL` |
| Upcoming-meet JSON (undocumented but public) | `/scripts/fuseDriver_Upcoming.js` | 987 meet objects |
| Performance-lists index | `/rankings.html` | 5,584 lists |
| Team index search (POST) | `/site_search.html` (`query=`, `type=meets\|teams_leagues\|athletes`) | `type=teams_leagues&query=Chicago` |
| League / class home | `/leagues/<sport>/<league_id>.html` | `/leagues/track/105.html` |
| Performance list | `/lists/track/<league_id>_<list_id>.html` | `/lists/track/1467_4524.html` |
| Team | `/teams/track/<team_id>.html`, `/teams/xc/<team_id>.html` | `/teams/track/43268.html` |
| Athlete | `/athletes/track/<athlete_id>.html`, `/athletes/xc/<athlete_id>.html` | `/athletes/track/8407278.html` |
| Meet result sheet | `/results/track/<meet_id>.html`, `/results/xc/<meet_id>.html` | `/results/track/88976.html` |
| Event result sheet | `/results/<sport>/<meet_id>_<event_id>.html` | `/results/track/88976_5435184.html` |
| Meet (registration) page | `/meets/<sport>/<meet_handle>.html` | `/meets/track/96943.html` |
| Meet event list (popup) | `/events/<sport>/<meet_handle>` | `/events/track/88976` |
| TFRRS mirror of an HS league | `https://www.tfrrs.org/leagues/<id>.html` | `/leagues/1467.html` |
| TFRRS mirror of an HS list | `https://www.tfrrs.org/lists/<list_id>/<slug>` | `/lists/4524/Illinois_Top_Times_1A` |

---

### Coverage

**States / levels.** The results index exposes 65 US+CA state/province codes (plus an "All States"
option = 66 `<option>` values), but high-school
meets are concentrated in a handful of states. From the upcoming-meet JSON (987 meets,
2026-09-19 → 2027-10-01) the `hs=1` flag appears for: **FL 334, NH 22, MI 11, AL 9, IN 8, TX 3,
VA 3, MO 3, PA 3, GA 3, NY 2, MA 2, NE 1, CA 1, KS 1, LA 1, WA 1 — and IL 0.** The site's own
"Performance Lists" index confirms the same shape: live HS sections exist only for **Florida,
Indiana, New Hampshire**; everything else (including Illinois) sits in an "Other" bucket of
*historical* lists.

**Illinois specifically — HS coverage is legacy, not live.** Method: state-filtered index windows
(100 rows/page, page count read from the pagination markup):

| Window | IL meets listed | High-school meets identified |
|---|---|---|
| 2025-01-01 → 2025-03-31 | 40 | **1** — `35th Annual SIU HS Invite` (03/07/2025, meet 88976) |
| 2026-01-01 → 2026-03-31 | 41 | 0 (all collegiate: CCIW, NCC, Lewis, Illini …) |
| 2026-04-01 → 2026-06-30 | 24 | 0 (all collegiate) |
| 2025-08-01 → 2025-11-30 (XC) | 33 | 0 (all collegiate: CCIW, GLVC, Bradley Pink …) |
| Upcoming 2026-09-19 → 2027-04-08 | 19 (11 XC / 8 TF) | 0 (`hs=0` on all 19) |

Historical depth is real, however: the IL results index runs to **47 pages ≈ 4,676 meets**
(46 pages × 100 + 76 rows on the last page), oldest observed row **01/17/2003**; the 2003–2006 tail
contains Illinois HS indoor/outdoor HS meets (`Charleston HS Boys Indoor Invitational` 2006-03-18,
`NTC Jr. High Conference Meet` 2005-05-10, `Apollo Conference Track Meet` 2005-05-09).

**Illinois HS performance lists (the actual athlete-bearing HS product for IL).** IL appears in the
performance-list index under `IL Top Times-Class 1A/2A/3A` (leagues 1467/1468/1469) plus
`Chicago Public Schools` (1355) and the older `2020 Illinois HS Indoor Performance List`
(`/lists/track/0_2891.html`). Newest IL list in the index is `1467_4524` =
**2024 indoor season** (rows dated Feb–Mar 2024; verified with `1467_4070` = 2023 season, 1,487
date stamps all 2023). Nothing IL-HS is newer: a results-index name query for `Top Times` returns
IL Top Times Championship meets only for 2016/2017/2018/2022/2023/**2024** and the newest is
`03/23/2024` → **no 2025 or 2026 IL HS indoor list/meet exists on DirectAthletics.**

**Composition of the performance-list index (relevant to IL's odd shape).** Parsing all 5,584
`pl-entry` rows on `/rankings.html`: **2,848** are TFRRS-shaped links
`https://<host>/lists/<list_id>/<Slug>/<year>/<i|o>` (hosts: `www.tfrrs.org`, `florida.`, `indiana.`,
`newhampshire.`), **274** are DirectAthletics-shaped `https://www.directathletics.com/lists/track/
<league_id>_<list_id>.html` with **no year segment**, and 2,462 are other (`/archived_lists/`,
`/leagues/`). The year-segmented TFRRS lists span 2002–2026 (2025: 461, 2026: 420). The IL HS lists
are all in the DA-shaped group: every Illinois HS list found is
`directathletics.com/lists/track/<league_id>_<list_id>.html` (Top Times `146[7-9]_*`, CPS `1355_290`,
older `0_2891`), and the Top Times list IDs descend 4524 (2024) → 1147, i.e. **no IL HS list carries
a 2025/2026 year segment because none of them is TFRRS-shaped at all**. Strict check on the other
direction: of the 2,848 TFRRS-shaped list links, **zero** contain `Illinois` and **zero** match a
strict `IHSA(?!A)` slug (the apparent IHSA hits are Indiana's `IHSAA_*` sectional lists). So
Illinois has the year-less shape *only* — unlike Indiana, which carries both shapes (peer claim,
[UNVERIFIED] by me; I did not fetch IN lists). Cross-check (peer agent 18,
Indiana): their global count of 2,848 TFRRS list links matches my parse exactly, and their roster
HTML parser reproduces the same 2,848 + 274 split; their inference that
"the state subdomain is where HS lives" does **not** hold for Illinois —
`illinois.tfrrs.org` is `NXDOMAIN` (verified, curl error 6) and the IL HS lists live on the DA
domain instead.

**School-level depth.** 8 Illinois track leagues: `105 Illinois` (876 men / 868 women teams),
`1088 IHSA Class A` (437/435), `1089 IHSA Class AA` (349/347), `1432 Illinois Charter/Other`
(33/29), `1355 Chicago Public Schools` (87/86), `1467/1468/1469 IL Top Times 1A/2A/3A`
(315/204/190 men). At the page's **default `Top 50`** view the 2024 Top Times lists reach 2,692
distinct IL athletes across 3,576 rows (1A 1,176 rows / 856 athletes; 2A 1,200 / 929; 3A 1,200 / 908).
That default is a *cut*, not the content: with the form's `limit` raised (`?limit=1000`) the same
three lists return **10,527 athlete-linked rows / 7,333 athletes / 753 team IDs** (11,720 rows
including the 1,193 relay rows that carry no athlete ID — see Enumeration) (1A 3,498 rows / 2,287 athletes;
2A 3,891 / 2,763; 3A 3,138 / 2,284) — **2.7× the athletes** — and `?limit=5000` adds nothing: it
returns the *same 3,498 rows* as `limit=1000` (identical row multiset; the only differences are the
CSRF token and 24 tie-order swaps among equal marks), so the IL lists saturate at or below 1000, and
row order is **not** stable enough to diff on. **The three class lists are
disjoint, not a redundant multi-view**: athlete overlap 1A∩2A = **1**, 1A∩3A = 0, 2A∩3A = 0 at both
depths (union 2,692 athletes / 702 team IDs at Top-50; 7,333 / 753 at limit=1000), so a per-season IL capture costs **three** list fetches —
contiguous list IDs (`4524/4525/4526`) mean "created together", not "same data". The single overlap is
a classification anomaly worth knowing: athlete `8118466` (`Engle, Avery`, SR, `Galena`, team `43530`)
appears on the 1A list with a *regular-season* mark (`FASST Indoor Classic`, 02/17/2024, 12:00.70) and
on the 2A list with the 2A **championship** mark (11:46.00, 03/23/2024) — i.e. list membership follows
the ranking list's own class assignment, while the authoritative class is named in the meet string
(`… Championships (2A)`).

---

### Enumeration

**Schools/teams — two independent paths, both countable.**

1. *League pages (class-based).* `GET /leagues/track/<league_id>.html` renders a Men's/Women's team
   table where every row is `<A HREF=".../teams/track/<id>.html">Name</A>`. Across the 8 IL leagues:
   **1,746 unique IL team IDs, 877 distinct team names**. Teams are per gender and per sport
   (`/teams/track/` vs `/teams/xc/` have separate ID sequences — A-C Central: T&F M 43268, T&F W
   43269, XC M 27271, XC W 27272).
   *Parser caveat (costs a silent zero if ignored):* these anchors are **uppercase** `<A … HREF=` with
   **absolute** URLs — verbatim markup: `<A class="pLinks" HREF="https://www.directathletics.com/
   teams/track/43268.html" …>A-C Central</A>` — and the rows are `<tr class="even">`/`<tr class="odd">`.
   A lowercase `href=` scan or a root-relative `href="/teams/track/` scan matches **0 of 619** anchors
   on `lg1467.html` while the extraction above matches all 619. Verification of both directions is in
   the tool: a full-page case-insensitive anchor census equals the script's extract per league
   (1467: 619, 1468: 403, 1469: 375; 0 missing).
   *Content check (a 200 is not a contract):* every list URL cited here was confirmed by its own page
   `<title>` and row dates, not by status code alone — `1467_4524` → title `Illinois Top Times (1A)`,
   all rows 2024; `1468_4525` → `(2A)`, 2024; `1469_4526` → `(3A)`, 2024; `1467_4070` → `(1A)`, 2023.
2. *Site search (level-classified, whole state).* `POST /site_search.html` with
   `type=teams_leagues&query=<substring>` returns one row per team × sport × gender with columns
   **Team | Sport | Gender | Type | State** and the team link. The search is a substring match, is
   **not paginated**, and is uncapped: `query=a` returned 150,990 rows (34.9 MB), `e` 153,862
   (35.3 MB), `i` 124,167 (28.8 MB), `o` 128,372 (29.6 MB) = 557,391 rows total. Filtering those to `State=IL` +
   `Type=high school` gives **12,447 rows → 4,964 distinct IL HS team IDs → 893 distinct school
   names**, of which **882** have a boys T&F team, **825** a boys XC team, **823** have all four
   T&F+XC teams (method: union of 4 single-vowel substring queries; residual risk = a school name
   containing none of a/e/i/o).

**Meet level classification (past meets too).** Every meet page carries
`MEET INFORMATION → Types of Entrants:` — verified on the past IL HS meet `/meets/track/88976.html`
(`Types of Entrants: High Schools`, 200) and on an upcoming MI HS meet (`Individual Athletes,
High Schools, Clubs, Colleges, Junior Colleges`). So any historical index row can be classified
HS-vs-college with one extra request, and the page also exposes `View All Entries: Available`
(entry lists not fetched — unreachable in budget, so their exact field set is unverified).

**Meets.** `/legacy_da/results` is the complete, server-rendered meet index; filters are
`with_states`, `with_sports` (track/xc/swimming), `with_date_from`, `with_date_to`,
`search_query`, plus `page=N` (100 rows/page). Row = date | meet link | sport | state. IL has 47
pages. `search_query` performs a name substring match (e.g. `Meet of Champions` → 25 rows for IL,
1998–2026; `Top Times` → 25 rows). Sorting is date-descending, so page 1 is a cheap "new results"
poll.

**Athletes.** Three paths: (a) team page roster table (`Name | Year` + athlete link), (b) performance
lists (`Place | Athlete | Year | Team | Time | Meet | Meet Date`), (c) event result sheets
(`Place | Overall | Name | Year | Team | Time | Score`). Plus a name search:
`POST /site_search.html type=athletes&query=<last name>` → rows **Athlete | Gender | Team | Sport**
with the athlete URL (e.g. `Dewalt` → 49 rows, athletes in TX/MI/IL …). The search-name column is
truncated at 30 characters, so canonical school names must come from team/league pages.

**A team page is one request carrying four payloads** (verified on both IL team pages fetched,
2026-09-19). `GET /teams/track/<team_id>.html` renders:

1. a **Team Roster** table `Name | Year` where every name is an athlete link — Lane Tech (11939)
   28 roster rows / 28 distinct AthleteIDs; A-C Central (43268) 17 / 17;
2. `General Info → Leagues:` as printed link text — A-C Central lists `Illinois`, `IHSA Class A`,
   `IHSA Class AA`, `Chicago Public Schools`, `Illinois Charter/Other`, `IL Top Times-Class 1A | 2A |
   3A` — so a team's class and league membership are recovered without any search request;
3. **sibling team links to the other three school surfaces in the same page**, and the ID adjacency
   holds for both samples: Lane Tech men's T&F (11939) → `/teams/track/11940.html` Women's T&F,
   `/teams/xc/15964.html` Men's XC, `/teams/xc/15965.html` Women's XC; A-C Central (43268) → track
   `43269`, XC `27271` / `27272`. One fetch per school yields all four team IDs and the cross-sport
   join;
4. **`Latest Results` with direct meet links** (`results/track/<meet_id>.html`, 10 distinct on Lane
   Tech), consistent with the dead-IL-product pattern: A-C Central's block reads `No recent results.
   Upcoming Meets No upcoming meets.`

**Roster `Year` is an era-relative ordinal, not a grade — and the roster is not season-scoped.** Three
checks, all offline:

* *No season selector exists.* Neither IL team page contains a single `<form>` or `<select>` (0 of
  each on both pages; 0 `config_hnd`/`seasonSelect` references) — unlike the Indiana TFRRS subdomain,
  whose roster is season-keyed by a `seasonSelect()` form (peer agent 18 found its select pinned to
  `2026 HSR Indoor` with no 2026-27 option, making IN's token a season-scoped class). IL's roster is an
  **un-scoped, mixed-era table**: Lane Tech's `Year` histogram is `13`×3, `14`×13, `15`×10, `SO`×1,
  `SR`×2, and its `Latest Results` block ends in 2019.
* *The codes order by class but do not name it.* Three of Lane Tech's 28 roster IDs also appear in the
  2024 3A Top Times list, and each maps consistently: roster `13` → list `SO`
  (`8823301 Howard, Daniel`), `14` → `JR` (`8823343 Reynolds, Nolan`), `15` → `SR`
  (`8823354 Sevig, Oliver`, `70 | Sevig, Oliver | SR | Lane Tech | 12.61mh | 41' 4.5" | Proviso West
  Boys Invitational | Feb 24, 2024`). So within one roster the code rises with seniority
  (13 = youngest, 15 = oldest of that season's group) but is **not** a graduating year (those athletes
  graduate 2026/2025/2024, not 2013/2014/2015) and **not** a current grade — and with no season key
  the roster gives no way to say which season the codes belong to. The same encoding appears on a
  second surface: athlete `8434425` (`Curtis, Reece`, St. Anne) has profile `Class: 15` and is a `SR`
  in the 2024 1A list — i.e. the profile `Class:` field is the *same* ordinal code, not a cohort year.
* *Consequence for the mission:* this column is exactly the off-by-one-class trap in code form. Any
  adapter must take class standing from the competition lists/result sheets (which do carry
  `FR/SO/JR/SR` per season) and use the roster only for `Name → AthleteID`; reading `13/14/15` as
  grades or as graduation years mis-assigns the cohort by 9–11 years.

**This is the relay-leg resolution route** (peer agent 18's suggestion, now evidenced for IL): a relay
row carries school + meet + date + four leg names, and the school's team page maps `Name → AthleteID`,
so a leg resolves when the name is unique within that school's roster. The caveat is bounded
resolution, not free ids: the roster is **un-scoped and mixed-era** (no season selector; Lane Tech's
table spans `13`/`14`/`15`/`SR` values with results ending 2019 while still listing a 2024 competitor),
so it gives an un-timestamped name→ID map rather than a season-keyed one — resolution must be validated
against list/result evidence and must stay a seed, never an id join. Sample: both IL team pages fetched
carry a roster; whether all ~4,900 do is untested [INFERENCE].

**Row shapes on list pages (four variants — positional parsing silently corrupts meet/date).** Cells per
row are `Place | Athlete | Year | Team | Mark | [Conversion] | Meet | Meet Date | [Wind]`, and the
IL Top Times pages mix four shapes: **7 cells** (5,819 rows), **8 with a trailing wind cell** (1,353),
**8 with a conversion column instead** (1,878, field events), **9 with both** (1,477). The wind cell
always reads `NWI`; the conversion column holds the imperial equivalent of a metric field mark.
Locate `Meet`/`Meet Date` by shape (the last cell matching `Mon D, YYYY`), never by fixed index — a
naive `tds[6]` parser assigns the *meet name* as the date on 3,355 of 10,527 rows.

**The layout is a property of the event, and it never varies within an event** (0 of the 24
athlete-bearing sections are mixed; peer agent 18 measured the same on Indiana), and it follows a
single rule: **`width = 7 + conversion? + wind?`** — where `conversion` means the event's mark is
metric with a derived imperial cell, and `wind` means the event is wind-eligible (200m, horizontal
jumps). The event name is declared in each section's `<h3>`, so an adapter derives the width from the
event rather than sniffing rows:
| Layout | Events (row counts, 2024 trio) |
|---|---|
| 7 cells, no extra columns | 60m (1,575), 400m (1,044), 800m (1,061), 1600m (951), 3200m (473), 60m Hurdles (715) |
| 8 cells = `+Wind` | **200m only** (1,353) |
| 8 cells = `+Conversion` | High Jump (560), Pole Vault (322), Shot Put (996) |
| 9 cells = `+Conversion +Wind` | Long Jump (878), Triple Jump (599) |
So "does this source carry wind?" is event-conditional: a wind column exists for 200m, Long Jump and
Triple Jump — never for 60m, the most wind-sensitive sprint — while the imperial conversion exists for
jumps and throws. Which of the four widths a *state's* lists exhibit is an artifact of the meet
format's event menu, not of a state-level convention (Indiana's HSR list has no 200m section at all, so
it shows three widths; its Long Jump is 9 cells, i.e. Indiana does emit wind — it simply has no
wind-eligible event *without* a conversion column).

**Six of every list's 30 sections are invisible to athlete enumeration (relay sections).** Each IL class list
carries **30 sections, not 24**: the 12 individual events × 2 genders plus **4x200 / 4x400 / 4x800 ×
2 genders**, and every relay section has **zero athlete links** — 0 across all six sections in 1A
(75–92 team links each). Relay rows have their own 6-cell layout
`Place | Team | Mark | Leg names | Meet | Meet Date`, where the legs are an unlinked comma string
(`Anderson, Milder, Newman, Milder`) with **no AthleteID and no grade**. Relay-only athletes are
therefore invisible to this source, and an athlete who appears only as a leg is indistinguishable from
one who never competed. Peer agent 18 reports the identical structure on Indiana (**400 relay rows of
1,146** data rows at their default cut, i.e. 34.9%; 775 of 5,287 = 14.7% at depth — their earlier
published 429 was contaminated by 22 `<th>` header rows and 7 footer-table rows, which they found and
corrected; row share tracks relay *event* share in both states: 20% of events in IL, 36% in IN, so a
single cross-state "relay fraction" would be wrong). Team-level relay data is still usable: the team
link, mark, meet and date are present.
**Exact totals for the 2024 trio at `limit=1000`: 11,720 rows = 10,527 athlete-linked rows (7,333
athletes) + 1,193 relay rows** (1A 399 / 2A 429 / 3A 365). Every athlete/class figure in this report is
computed over the athlete-linked subset; relay rows are excluded by construction.

**Season mapping (why FR = Class of 2027 here).** All 10,527 rows of the 2024 trio carry a parsed
meet date, and every one falls in **January–March 2024** (Jan 26 / Feb 1,657 / Mar 8,844) — the
2023-24 indoor season. The `Year` column is the **at-meet grade**, so an `FR` on these lists was in
9th grade in 2023-24 and graduates in 2027 (`2024 + (12 − 9) = 2027`) = **Class of 2027**, the
mission's cohort. No row falls outside that school year, so there is no boundary leakage. This is not
just date arithmetic: the same athlete ID across the two adjacent seasons shows the label advancing by
exactly one grade — `8434425` (`Curtis, Reece`, St. Anne) is `JR` in the 2023 1A list
(`7.08 | 7th | Illinois Top Times Championship (1A) | Mar 24, 2023`) and `SR` in the 2024 1A list
(`6.93 | 1st | Cabin Fever Invitational | Mar 2, 2024`), which measures the at-meet-grade semantics
rather than inferring it. (Peer
agent 18 independently established the at-meet-grade semantics on 2025-26 lists, where the target
cohort is `JR`, not `SR`; that off-by-one-class trap does **not** apply to this 2024-based arithmetic.)
Class arithmetic of this kind is only safe on season-keyed surfaces: across this evidence set the class
token agrees between surfaces exactly when both are season-keyed (list `Year` vs team-roster code after
the mapping above) and disagrees the moment the season key is dropped — the failure mode peer agent 18
hit three times on Indiana and resolved by pinning their roster's `2026 HSR Indoor` season selector,
and the same trap this report's roster-code finding warns about.

**Class of 2027.** The class-year signal is the `Year` column (`FR/SO/JR/SR`) and the list's own YEAR
filter (`?year=FR`, server-side). At the page's **default `Top 50`** limit the three 2024 IL Top Times
lists give 3,576 rows, class split SR 1,369 / JR 1,063 / SO 718 / **FR 426 rows → 336 distinct
freshmen → 184 schools**. At `?limit=1000` the same three lists give **10,527 rows / 7,333 athletes**,
class split SR 2,841 / JR 2,738 / SO 2,664 / **FR 2,283 rows → 1,580 distinct freshmen → 301 schools**
(disjoint per class: 1A **594** / 2A **629** / 3A **357** FR athletes; 2,283 = 915+897+471 rows, i.e.
**all three fetches are required**. Separately, one 3A row's `Year` cell reads `2` instead of a class
code — a source data quirk, not part of the FR counts, and a reason for adapters to whitelist
`FR/SO/JR/SR`). A 2024 freshman is Class of 2027, so those rows are directly usable
C/O-2027 evidence (name, school, mark, meet, date, DA athlete ID, DA team ID).
**The `limit` parameter is the single largest lever on this source** (the filter form is
`GET /lists/track/<list_id>/list_data` with `limit` — select options 5/10/20/25/30/50/100/200/500,
default 50 — plus `event_type`, `year`, `gender`; larger values are accepted than the select offers).
Directionally the effect is ~2.7× on rows and ~4.7× on the C/O-2027 subset between the default and
`limit=1000`; a peer measuring an Indiana list saw a similar ~5× jump ([UNVERIFIED] — their fetch,
not mine).

---

### Stable identifiers

| Entity | Identifier | Evidence |
|---|---|---|
| Athlete | `/athletes/track/<athlete_id>.html` (separate `/athletes/xc/<id>.html` namespace) | `8407278` (Dewalt, Jaylen), `8823352` (Anderson, Quinn), `6626461`, `8736748`, `4104615` (XC) |
| Athlete (TFRRS host) | `https://www.tfrrs.org/athletes/<athlete_id>.html` — **same numeric namespace as DA**, no sport segment | `https://www.tfrrs.org/athletes/8434425.html` → 200, title `Reece Curtis - DirectAthletics`, `Class: 15`, `Team: St. Anne`, bests `60m 7.08 / 200m 23.50 / 400m 51.45 / HJ 1.75m`, results incl. `03/22/24 Illinois Top Times … (1A)`. ID `8434425` is exactly the anchor used by the DA-side 2024 Top Times 1A list |
| Team | `/teams/track/<team_id>.html`, `/teams/xc/<team_id>.html`; men's/women's distinct IDs | `43268`/`43269`/`27271`/`27272` (A-C Central), `11939`/`11940` (Lane Tech) |
| Meet (DA) | `/results/<sport>/<meet_id>.html` and meet handle `/meets/<sport>/<meet_handle>.html` | `88976` (2025 SIU HS Invite) = `/meets/track/88976.html`-style handle |
| Meet (TFRRS) | `https://www.tfrrs.org/results/<id>/<slug>/` — **different ID sequence**; IL HS leagues only carry TFRRS ids for *recent/related* meets | `/results/94985/2026_98th_Clyde_Littlefield_Texas_Relays_Univ_College/` |
| Event | `<meet_id>_<event_id>` | `88976_5435184` (B) 60m Dash; 30 events per meet; `/events/track/88976` lists event names |
| League (class/conference) | `/leagues/track/<league_id>.html` | 105, 1088, 1089, 1355, 1432, 1467, 1468, 1469 |
| Performance list | `/lists/track/<league_id>_<list_id>.html`; TFRRS form `tfrrs.org/lists/<list_id>/<slug>` | `1467_4524`, `1468_4525`, `1469_4526` |
| Season | **no season ID.** Season = meet dates / list identity. TFRRS list URLs embed it: `.../2024/i` (indoor) | `newhampshire.tfrrs.org/lists/4362/.../2024/i` |
| Result row | **no ResultID.** Identity is (meet_id, event_id, athlete_id, round/heat) | event sheet rows carry no row id |

Names are *not* identifiers: the search index truncates team names at 30 characters
(`Academy of Scholastic Achievem`, `IL Sch. for Visually Impaired`) and carries alias/duplicate
forms — `Chicago Military` vs `Chicago Military Academy`, `Illinois Math and Science` vs
`Illinois Math and Science Acad`, `TEAM ILLINOIS` vs `Team Illinois`, `Ottawa Township` vs
`Ottawa Township JV`, `Byron JV`, `DeLand-Weldon JHS` (JV/JHS squads are typed `high school`).

---

### Athletic.net leverage

* **No Athletic.net surface at all.** Across every fetched DA page (results, team, athlete,
  performance list, meet, events, rankings, search) there are **zero** occurrences of `athletic.net`
  and zero of `milesplit`. The only outbound links are the meet's own site (e.g.
  `gvsulakers.com`), Twitter, and Google/GTM.
* **Deterministic seed material instead of links.** DA supplies school name + state (+ sport and
  gender) and athlete name + school + class year + marks. That is enough to *request one specific*
  A.net candidate (team page by school+state, profile by name+school+class), never a broad ranking
  sweep. Cavities for the join: DA carries **no city** for schools (team page shows name + leagues
  only) and its names are normalized short forms (`Chicago Alcott`, `Lane Tech`), so any
  reconciliation must match on normalized name + state, expecting `Chicago (<Neighborhood>)`-style
  A.net names to differ. `[INFERENCE]`
* **Request-avoidance estimate (IL).** There is **no ongoing avoidance**, because DA has no live IL
  HS meets or lists for 2025-26: a weekly DA refresh for IL (1 upcoming-JSON + 3 index pages) yields
  **0 new IL HS athletes**. What DA can avoid is *historical re-acquisition*:
  2024 IL Top Times gives **7,333 distinct athletes (1,580 of them C/O-2027)** with school + mark +
  meet + date + athlete ID at `limit=1000`, and 1,746–4,964 IL team IDs. Rough ceiling for the
  C/O-2027 cohort: **~1,580 discovery+profile A.net requests** that DA evidence could replace or
  pre-verify (≈2 requests/athlete), plus ~5,700 for the older classes in the same lists; this is a
  one-time saving, not a recurring one. Team-level: DA's
  882 boys-T&F IL school names could seed an A.net TeamID map once, but AA (the official association
  index) is the better seed — DA is redundant there.
* **The roster route is the one live athlete-ID surface, and its cost is measurable.** A team page
  (1 request) returns the school's `Name | Year` roster with AthleteIDs, the other three team IDs
  (track/XC × men/women), league memberships and meet links. Two ways to use it: (a) resolve relay legs
  (name + school) to AthleteIDs — the only route that recovers the 1,193 relay rows' legs; (b) enumerate
  an IL school's current T&F athletes without any A.net call. Cost: ~4,900 requests for the whole IL HS
  universe at 1/school (vs ~0 for a name-only seed, but name-only cannot produce an ID). Bound the
  claim honestly: the roster is **un-scoped and mixed-era** — no season selector exists on the page,
  and Lane Tech's table mixes `SR` with `13`/`14`/`15` while still listing an athlete who competed in
  2024 — so it yields an un-timestamped name→ID map: leg resolution works for names listed and fails
  for those absent, and no row tells you which season it belongs to. No bulk/delta feed exists; any
  roster refresh is 1 request per school. `[INFERENCE]` on the fleet-wide cost.

---

### Athlete evidence

Observed on `/athletes/track/8407278.html` (Jaylen Dewalt, Belleville East) and
`/athletes/track/8823352.html` (Quinn Anderson, Lane Tech):

| Field | Available? | Evidence |
|---|---|---|
| Name | yes (`Last, First`) | `Dewalt, Jaylen` |
| Graduating class / grade | partial — **use the list/result `Year` column only** | profile header `Class:` (`Senior`; also a bare number `13`); per-result `Year` column (`FR/SO/JR/SR`, season-scoped and verifiable); team-roster `Year` column (`13`/`14`/`15`/`23`–`26` on the two pages fetched) is an **era-relative ordinal**, contradicted as a grade by list cross-check (see Enumeration) |
| School | yes (team link `/teams/track/8600.html`) | `Belleville East` |
| City/state | **no** (state only, in search rows) | team page shows name + leagues, no city |
| Gender/category | yes (implicit: team is Men's/Women's T&F or XC) | `Men's Track & Field` |
| TF/XC distinction | yes (URL path + team sport) | `/athletes/track/…` vs `/athletes/xc/…` |
| Indoor/outdoor | yes, split columns | summary table `Event \| Indoor \| Outdoor \| Best` |
| Performances | yes | `60m Dash 7.15 (ind) / 100m 11.54 (out)`, all results list with `P`/`F` round |
| PRs | yes ("Best" column computed across seasons) | `24.16` 200m indoor best, 2023–2025 rows |
| Progression | yes (results sorted date-desc across seasons) | 2023 St. Clair County → 2025 SIU HS Invite |
| Meets | yes, each row links the meet/event sheet | `/results/track/88976_5435184.html`, `/results/track/79441_4863754.html` |
| Profile URL | yes | `/athletes/track/8407278.html` |

Grade-signal warning (observed contradiction, matters for C/O-2027 classification): the profile
header `Class:` field is **not reliable** — Dewalt is listed `Class: 13` while his own result rows
show `JR` in March 2025; Anderson shows `Class: Senior` while his roster row also reads `SR`.
Use the per-result / per-list `Year` column (`FR/SO/JR/SR`) or the list's YEAR filter, not the
profile `Class:` number.

---

### Recruiting information

**None.** No coach, AD, or staff directory exists anywhere on DA; team pages contain only
`General Info → Leagues` and `More Teams`. What *does* exist is a **MEET CONTACT** block on the
registration meet page (`/meets/track/96943.html`: `Name:` + `Phone:`) and a
`/contact_meet_director.html?meet_hnd=…&sport=…` form. That is a meet-director role contact, not a
school/sport-role coach contact, and it includes a telephone number — per the privacy contract I
**did not transcribe the value** and recommend not ingesting meet-director phones at all. No school
athletics website is exposed either (only the meet host's own `Meet Site:` URL, e.g.
`gvsulakers.com`, plus `fstiming.com` seen as a timer reference).

---

### Result evidence

From `/results/track/88976_5435184.html` (2025 SIU HS Invite, (B) 60m Dash, 98 athlete links,
17 prelim heats + final) and `/results/track/58092_3679792.html` (2019 Gene Armer Invitational):

| Field | Available? | Evidence |
|---|---|---|
| ResultID | **no** (no row identifier) | rows are `<td>` values only |
| AthleteID | yes | `/athletes/track/8407278.html` |
| MeetID | yes (from page URL/anchor) | `88976`, `58092` |
| EventID | yes | `88976_5435184`, `58092_3679792`; event names from `/events/track/88976` |
| mark | yes (`Time`/`Mark` column, e.g. `7.15`, `13.95m (h)`, `8:10.89`) | event sheet + team "top marks" table |
| normalized mark inputs | **partly, on performance lists only** | list rows for field events add an imperial **conversion** column (`1.94mh` → `6' 4.25"`); result sheets have none |
| timing method | **no** (`FAT`/hand never appears) | keyword scan of meet+event+index pages = 0 hits |
| wind | **no usable value** — the surface differs by page type | event result sheets: no wind column at all. Performance lists: many rows carry a trailing wind cell, but **all 2,830 occurrences read `NWI`** (= no wind information) across the three 2024 IL lists; zero numeric readings |
| implement/hurdle spec | **no** (event names only: `60m Hurdles`, no `39"`) | `/events/track/88976` |
| heat/round | yes | `Preliminaries: Heat #1..17`, `Finals: Heat #1`, anchors `#roundN_heatM` |
| place | yes (`Place` + `Overall` columns) | `1 | 5 | Dewalt, Jaylen | JR | Belleville East | 7.15 | -` |
| date | yes (meet date on sheet; not per row) | `March 7-8, 2025` |
| school represented | yes (team link per row) | `/teams/track/8600.html` |
| relay membership | partial — relay events appear per athlete with the relay mark (`4 x 100m Relay … 43.22` on the athlete page); relay event sheets were not fetched, so per-leg membership is unverified | athlete page 8407278 |
| team scoring | yes | `Rank | Team Name | Score` block on the meet sheet (e.g. Bloom Twp. 76) |

Duplicate-row caution: one meet name can exist multiple times in the index with different IDs
(`Augie Alt`/`Augie Alternative`; `Meet of Champions` 2016, 2018 ×2, 2019) — key meets by ID, never
by name+date alone.

---

### Incremental use

* **New meets (cheap, 1 request):** GET `/scripts/fuseDriver_Upcoming.js` → JS array of
  `{date_begin, name, sport, venue_state, meet_hnd, reg_status, hs, jhs, college, jcollege, club,
  public_registration}`. Diff `meet_hnd` per week; the `hs` flag is the level filter. For IL this
  currently yields 19 meets, 0 HS.
* **New results (cheap, 1 request/sport):** `/legacy_da/results?filterrific[with_states]=IL` page 1
  is date-descending; new rows appear at the top. For a target state, page 1 + (occasionally) page 2
  is sufficient for a weekly delta.
* **Changed meets:** meet sheet `/results/<sport>/<meet_id>.html` links every event sheet; event
  sheets are immutable-looking but carry no per-event timestamp, so change detection requires
  diffing the event sheets (or the event count/list from `/events/<sport>/<handle>`). No
  `Last-Modified`-based protocol was observed; responses are `cache-control: max-age=0, private,
  must-revalidate` with an `etag`, so conditional GETs (`If-None-Match`) are technically possible but
  were not tested.
* **Historical IL HS athlete backfill (3 requests/season):** the `IL Top Times` class lists —
  `/lists/track/1467_4524.html`, `/1468_4525.html`, `/1469_4526.html` for 2024 — are one fetch per
  class (they are disjoint; see Coverage). Three requests cover one season — **always request
  `?limit=1000`**: at the default Top-50 the 2024 trio yields 2,692 athletes, at `limit=1000` it
  yields **7,333** (and 1,580 C/O-2027). The index carries 10 seasons of trios (30 class lists)
  descending from 2024, with 2024 and 2023 date-verified here; `?year=FR` gives a server-side class
  filter, but a peer found that view hard-capped near 1,000 rows per list ([UNVERIFIED] for IL — I
  did not fetch a `year=` view here).
* **Affected athletes:** team pages render `Latest Results`; athlete pages render `All Results`
  date-desc — but both require an ID, so athlete-level deltas must be driven from the meet/event
  delta above (meet → event → athlete IDs).
* **Not available:** any bulk export, sitemap, delta feed, or "modified since" API. There is no
  robots.txt either (see below), and no `/sitemap.xml` was attempted (out of budget).
* **For Illinois HS specifically:** there is nothing to refresh weekly — the observable IL HS
  products (Top Times lists, IL HS meet results) are historical. A monthly/seasonal check of
  `/rankings.html` (one request) is sufficient to detect if DA resumes publishing IL HS lists.

---

### Access characteristics

* **Markup quirk for any adapter:** team and athlete anchors on league/list/meet pages are
  **uppercase** `<A … HREF="…">` with **absolute** URLs (see Enumeration); a lowercase `href=` scan
  with a root-relative assumption silently returns zero. A `<title>` check on every list URL is also
  required — a 200 alone does not prove the requested class/season was served (peer agent 18 observed
  a wrong-list 200 elsewhere on the same host).
* **Class: normal HTML** (server-rendered Rails with Turbo frames; the only JS-rendered page is
  `/upcoming_meets.html`, whose data comes from the static `/scripts/fuseDriver_Upcoming.js`).
  Secondary class: **public structured JSON-ish** for that one artifact (served as
  `content-type: text/html`, body = `var data = [ {…}, … ]` + a Fuse.js driver; parse the bracketed
  array).
* **Auth:** none for results, teams, athletes, lists, leagues, search, or the upcoming JSON.
  Registration/entry (`/signup.html`, `/contact_meet_director.html`) requires an account — not used.
* **Robots:** `GET /robots.txt` returns HTTP 200 with the site's HTML shell (29,631 bytes, no
  `Disallow` directives at all, zero matches for `^disallow`). No crawl policy is published; treat
  the site as unspecified and stay polite.
* **Rate limiting:** no 429 and no `Retry-After` observed anywhere in 53 sequential requests
  (~1.5 s spacing). Response latency ~0.1–7 s depending on payload; `x-runtime` on the root page
  was 0.005 s.
* **Payload hazard:** `/site_search.html` broad queries return unbounded result sets in one
  response (124k–154k rows, 28.8–35.3 MB each for a single letter). Letter queries are therefore *not*
  cheap; prefer specific prefixes (`query=Chicago`, `query=ILLINOIS`) or the paginated league pages.
* **Cloudflare/anti-bot:** none encountered from this host; CloudFront caching only. The
  browser-blocked host in this mission (`www.athletic.net`) is unrelated to DA.
* **`illinois.tfrrs.org` does not exist** — `curl: (6) Could not resolve host` (NXDOMAIN), while
  `florida.tfrrs.org`, `indiana.tfrrs.org`, `newhampshire.tfrrs.org` are linked from the
  performance-list index — the entire TFRRS host set referenced by that index (anchor counts there:
  www 1,960 / florida 717 / indiana 133 / newhampshire 38).
* **But TFRRS still serves Illinois HS athlete profiles** — reachability is by ID, not by state
  host: `https://www.tfrrs.org/athletes/8434425.html` → **200**, rendering the same `Reece Curtis`
  (St. Anne, `Class: 15`) profile and 2024 IL Top Times results as the DA side. So the state-subdomain
  pattern is irrelevant for athlete-level work; it only gates the state *list/roster* surfaces.
* **TFRRS side detail:** on `www.tfrrs.org/leagues/1467.html` the IL HS teams are rendered with
  placeholder links (`/teams/tf/_m.html`, `/teams/tf/_f.html`) — i.e. TFRRS has no real team IDs for
  these HS teams; the usable team IDs live on the DA side, while the athlete ID is shared by both.

---

### Recommendation

**VALIDATION + limited ATHLETIC.NET-SEED (with DISCOVERY-ONLY value for non-IL states).** Not
PRIMARY, not RESULT-SOURCE, not COACH-DIRECTORY.

Expected marginal coverage:

* **Direct score for Illinois: ~0 live.** No 2025-26 IL HS meets (0 of 19 upcoming IL meets are HS;
  0 HS meets in the 2026 indoor and 2026 outdoor windows; the sole 2025-26 IL HS result set observed
  is `35th Annual SIU HS Invite`). Any plan that relies on DA for weekly IL HS discovery will fail.
* **Historical/validation value: moderate — and larger than the default view suggests.** 2003→2024 IL
  meet history (≈4.7k meets), the 2024 (and 2023) IL Top Times indoor lists — **7,333 athletes in 2024
  alone at `limit=1000`**, 10 seasons of class-trios on the index — and 4,964 IL HS team IDs with a
  high-school/middle-school/club level flag. Use it to (a) pre-verify or replace historical A.net
  profile fetches for IL indoor athletes, (b) supply **1,580 measured Class-of-2027 IL athletes**
  (2024 lists at `limit=1000`; 336 if you take the default Top-50 cut) with school + marks + meet
  dates, (c) cross-check AA/IHSA school naming.
* **Best mechanical uses to actually build:** (1) the one-request upcoming-meet JSON as a *meet*
  discovery feed for any state with HS flags (MI 11, IN 8, MO 3, KS 1, NE 1 events in the current
  window) — this is where the marginal coverage lives, not in IL; (2) the team-index search as a
  cheap state-wide school/level census (893 IL HS names, 882 with boys T&F) for reconciliation
  against AA/A.net; (3) leave the athlete/result surfaces unbuilt for Illinois until DA resumes
  publishing IL HS dates.
* **Relay-only athletes are invisible.** Six of each list's 30 sections are relays whose legs carry no
  AthleteID and no grade (1,193 rows in the 2024 trio), so a freshman who only ran legs cannot be
  discovered here at all — the C/O-2027 set is a *individual-event* set, and sprinters/middle-distance
  athletes who raced only relays are excluded. Team-level relay marks (team + mark + meet + date) are
  still usable: the team link, mark, meet and date are present, and leg names are resolvable to
  AthleteIDs through the school's team-page `Name | Year` roster in one request per school — subject
  to the roster being un-scoped and mixed-era (no season key; a `Year` value can contradict the
  athlete's measured class) and to name uniqueness within the school (see Enumeration: "relay-leg
  resolution route"). Treat as a seed, never as an id join.
* **C/O-2027-specific ceiling:** 1,580 IL freshmen (2024 indoor, `limit=1000`; 336 at the default
  cut) is real but is a depth-capped view of one indoor season, not a full grade cohort, and it carries
  no outdoor or XC evidence — treat it as a corroboration set, not a discovery primary.

---

### Evidence appendix

All timestamps 2026-09-19 America/Chicago (UTC−05:00). Every row was fetched with a single polite
`curl` (UA `Mozilla/5.0 … Chrome/128.0 Safari/537.36`, `-L`, ≤30–120 s, sequential, ~1.5–2 s gaps);
no cookies, tokens, or auth were used. Raw captures live in `tools/da-scratch/`; the reproducing
parser is `tools/14-da-il-enumerate.py` (offline mode parses those captures).

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.directathletics.com/` | GET | 200 | Provider identity, nav surface, CloudFront/nginx stack, `x-runtime 0.005s` | 23:12:42 |
| `https://www.directathletics.com/robots.txt` | GET | 200 (HTML shell) | No robots directives published (0 `Disallow`) | 23:12:46 |
| `https://www.directathletics.com/results.html` | GET | 200 | `/legacy_da/results` filter form: sports (track/xc/swimming), 66 state codes, date range, `search_query`, `page=2..792` pagination; meet links are `/results/xc/<id>.html` or `tfrrs.org/results/xc/<id>/<slug>/` | 23:12:48 |
| `https://www.directathletics.com/upcoming_meets.html` | GET | 200 | JS-driven meet list; level filters (open/future/closed, xc/tf/indoor/outdoor, college/jcollege/hs/jhs/club); row template `/meets/<sport>/<meet_hnd>.html`; data array named `meetsRaw` | 23:12:49 |
| `https://www.directathletics.com/rankings.html` | GET | 200 | Performance-lists index: 5,584 lists; live HS sections only FL (828)/IN (140)/NH (71); "Other" 269 historical lists incl. `2020 Illinois HS Indoor Performance List`, `IL Top Times-Class 1A/2A/3A` (leagues 1467/1468/1469) and `Chicago Public Schools` (1355) | 23:12:50 |
| `https://www.directathletics.com/search.html` | GET | 200 | Search form: POST `site_search.html` with `query` + `type ∈ {meets, teams_leagues, athletes}` | 23:12:52 |
| `https://www.directathletics.com/leagues/track/1467.html` | GET | 200 | `IL Top Times-Class 1A` league: 315 men / 304 women team links `/teams/track/<id>.html`; meet sidebar links to TFRRS ids | 23:13:33 |
| `https://www.directathletics.com/leagues/track/1355.html` | GET | 200 | `Chicago Public Schools`: 87/86 teams (Lane Tech 11939/11940) | 23:13:35 |
| `https://www.directathletics.com/leagues/track/1569.html`, `/1570.html` | GET | 200 | `Metro East/Metro West Conference` are **Florida** conferences (Oak Ridge, Winter Park, Dr. Phillips) — name collision, not IL | 23:13:37–39 |
| `https://www.directathletics.com/leagues/track/1468.html`, `/1469.html` | GET | 200 | `IL Top Times-Class 2A` (204/199), `3A` (190/185) | 23:13:55–57 |
| `https://www.directathletics.com/teams/track/43268.html` | GET | 200 | Team page shape: name, sport, leagues, `More Teams` cross-links (W T&F 43269, XC 27271/27272), roster `Name | Year` with athlete links, `Latest Results`, `Upcoming Meets` | 23:13:58 |
| `https://www.directathletics.com/teams/track/11939.html` | GET | 200 | Roster with `SR/13/14/15` year column; latest results 2017–2019; top-marks table `Event | Season | Athlete | Year | Time/Mark` (incl. `13.95m (h)`) | 23:14:00 |
| `https://www.directathletics.com/legacy_da/results?filterrific[with_states]=IL` | GET | 200 | IL index page 1: 100 rows, 2025-09→2026-09, **all collegiate**; 47 pages total | 23:14:13 |
| `https://www.directathletics.com/scripts/fuseDriver_Upcoming.js` | GET | 200 (354,456 B) | Public meet JSON: 987 meets, 2026-09-19→2027-10-01, flags `hs/jhs/college/jcollege/club`, `meet_hnd`, `venue_state`; HS: FL 334, NH 22, MI 11, AL 9, IN 8 … **IL 0** | 23:14:36 |
| `https://www.directathletics.com/results/track/58092.html` | GET | 200 | 2019 Gene Armer Invitational (Urbana, IL — HS): meet name/date/venue, 30 event links `58092_<eventid>`, men's/women's team scoring | 23:14:57 |
| `https://www.directathletics.com/athletes/track/8823352.html` | GET | 200 | Athlete page fields: `Anderson, Quinn` / `Class: Senior` / `Team: Lane Tech`; summary table empty (stale roster) | 23:14:58 |
| `https://www.directathletics.com/meets/track/96943.html` | GET | 200 | Meet page: entrant types (Individuals/High Schools/Clubs/Colleges/Junior Colleges), `Meet Site:` external URL, `MEET CONTACT` name+phone (not transcribed), `/events/track/<hnd>` popup, login-gated signup | 23:15:00 |
| `https://www.directathletics.com/results/track/58092_3679792.html` | GET | 200 | Event sheet columns `Place|Overall|Name|Year|Team|Time|Score`, Prelim heats + Finals anchors, athlete & team links → IDs | 23:15:10 |
| `.../legacy_da/results?filterrific[with_states]=IL&page=47` | GET | 200 | Oldest IL rows (`01/17/2003`), 76 rows → total ≈4,676 IL meets; 2003–2006 rows include IL HS indoor/outdoor + Jr. High meets | 23:15:23 |
| `.../legacy_da/results?filterrific[with_states]=IL&filterrific[with_date_from]=2019-01-01&...to=2019-12-31` | GET | 200 | 100 rows / 4 pages for 2019; contains HS-style names (`LTC F/S Conf 2019`, `Lincoln Prairie Conference`) | 23:15:25 |
| `https://www.directathletics.com/lists/track/1467_4524.html` | GET | 200 (1.41 MB) | `Illinois Top Times (1A)` performance list, 2024 season, 1,176 rows: `Place|Athlete|Year|Team|Time|Meet|Meet Date`, YEAR filter SR/JR/SO/FR/8/7/6, `limit` default `Top 50`, 15 event anchors | 23:15:28 |
| `.../legacy_da/results?...&filterrific[search_query]=Meet of Champions` | GET | 200 | Name search works on the index; 25 IL rows 1998–2026 | 23:15:56 |
| `.../legacy_da/results?...&filterrific[search_query]=Top Times` | GET | 200 | Newest IL Top Times meet with results = **03/23/2024** (2A) & 03/22/2024 (1A); none for 2025/2026 | 23:15:58 |
| `https://www.tfrrs.org/leagues/1467.html` | GET | 200 | TFRRS mirror of the IL HS league; HS team links are placeholders `/teams/tf/_m.html`; TFRRS list URL `/lists/4524/Illinois_Top_Times_1A` | 23:16:03 |
| `.../legacy_da/results?filterrific[with_states]=IL&...2026-01-01..2026-03-31` | GET | 200 | 2026 indoor window: 41 IL meets, **0 HS** | 23:16:16 |
| `.../legacy_da/results?filterrific[with_states]=IL&...2025-01-01..2025-03-31` | GET | 200 | 2025 indoor window: 40 IL meets, **1 HS** (`35th Annual SIU HS Invite`, 03/07/2025, meet 88976) | 23:16:18 |
| `https://www.directathletics.com/teams/xc/27271.html` | GET | 200 | IL HS XC team page exists but empty: `No athletes on roster. No recent results.` | 23:16:20 |
| `https://www.directathletics.com/leagues/track/105.html` | GET | 200 | Master `Illinois` league: 876 men / 868 women teams | 23:16:37 |
| `https://www.directathletics.com/leagues/track/1088.html`, `/1089.html`, `/1432.html` | GET | 200 | `IHSA Class A` 437/435, `IHSA Class AA` 349/347, `Illinois Charter/Other` 33/29 | 23:16:39–43 |
| `https://www.directathletics.com/results/track/88976.html` | GET | 200 | 2025 SIU HS Invite (Carbondale, IL — HS, Mar 7–8 2025): 30 event links → current-era IL HS results do exist in DA | 23:16:45 |
| `https://www.directathletics.com/meets/track/88976.html` | GET | 200 | Past-meet handle pattern confirmed; `Types of Entrants: High Schools` = per-meet level classification for historical meets; entries list `Available` (not fetched); `MEET CONTACT` name/phone/fax (not transcribed) | 23:22:16 |
| `https://www.directathletics.com/results/track/88976_5435184.html` | GET | 200 | Current HS event sheet: 98 athlete links, place/overall/name/year/team/mark/score, 17 prelim heats + final | 23:17:05 |
| `.../legacy_da/results?filterrific[with_states]=IL&...2026-04-01..2026-06-30` | GET | 200 | 2026 outdoor window: 24 IL meets, **0 HS** | 23:17:07 |
| `.../legacy_da/results?filterrific[with_states]=IL&filterrific[with_sports]=xc&...2025-08-01..2025-11-30` | GET | 200 | 2025 IL XC window: 33 meets, all collegiate | 23:17:09 |
| `https://www.directathletics.com/lists/track/1467_4070.html` | GET | 200 (1.45 MB) | Second-newest IL list = **2023** season (1,487 date stamps all 2023) → list→season mapping confirmed | 23:17:24 |
| `https://www.directathletics.com/athletes/track/8407278.html` | GET | 200 | Athlete profile fields incl. Indoor/Outdoor `Best` table + `All Results` (2023–2025); `Class: 13` contradicts result-year `JR` | 23:17:26 |
| `https://www.directathletics.com/site_search.html` (POST `query=Chicago&type=teams_leagues`) | POST | 200 | Team index: 250 rows, columns `Team|Sport|Gender|Type|State`, team links, IL rows for HS/club; no pagination | 23:17:28 |
| `.../site_search.html` (POST `query=ILLINOIS&type=teams_leagues`) | POST | 200 | 204 rows incl. club teams; confirms state+level columns drive filtering | 23:17:49 |
| `.../site_search.html` (POST `query=Dewalt&type=athletes`) | POST | 200 | Athlete index: `Athlete|Gender|Team|Sport` + `/athletes/track|xc/<id>.html` (49 rows) | 23:17:52 |
| `.../site_search.html` (POST `query=a\|e\|i\|o&type=teams_leagues`) | POST | 200 ×4 (34.9/35.3/28.8/29.6 MB) | 557,391 rows total; IL rows 26,916; IL `high school` 12,447 → **4,964 team IDs / 893 school names**; levels: high school 12,447, middle school 10,656, club 2,808, college 627, junior college 335 | 23:18:06–23:19:11 |
| `https://www.directathletics.com/lists/track/1468_4525.html`, `/1469_4526.html` | GET | 200 ×2 | 2A/3A 2024 lists (all rows dated 2024); combined 1A/2A/3A = 3,576 rows, 2,692 athletes, FR 426 rows → **336 C/O-2027 athletes, 184 schools** | 23:19:45–49 |
| `https://www.directathletics.com/events/track/88976` | GET | 200 | Meet event list popup: event names by gender/division (`(B) 60m Dash`, `Men's 4 x 800m Relay Open`, …); no implement/hurdle specs | 23:19:51 |
| `https://illinois.tfrrs.org/` | GET | **no response** — `curl: (6) Could not resolve host` | Illinois is not a TFRRS state subdomain (FL/IN/NH only) | 23:16:03 |
| `https://www.directathletics.com/lists/track/1467_4524.html?limit=1000` | GET | 200 (3.77 MB) | Full-depth 1A 2024 list: **3,498 rows / 2,287 athletes / 277 teams / 915 FR rows → 594 FR athletes** vs 1,176 rows at the page default → the bare page is a Top-50-per-event cut, not the list | 23:38:18 |
| `.../lists/track/1468_4525.html?limit=1000`, `/1469_4526.html?limit=1000` | GET | 200 ×2 (4.22 / 3.36 MB) | 2A: 3,891 rows / 2,763 athletes / 897 FR rows → 629 FR athletes; 3A: 3,138 / 2,284 / 471 FR → 357 FR. Trio at depth = **10,527 rows / 7,333 athletes / 753 teams / 1,580 C/O-2027 FR athletes at 301 schools** | 23:38:24–28 |
| `.../lists/track/1467_4524.html?limit=5000` | GET | 200 (3.77 MB) | **Saturation proof for IL**: same byte size as `limit=1000` and the **same 3,498-row multiset**; the 18,299 differing bytes are the CSRF token (cluster at offset 954) plus 24 rows whose order swaps among equal-mark ties → no depth gain above ~1000, and row order is not diff-stable | 23:38:32 |
| `https://www.tfrrs.org/athletes/8434425.html` | GET | 200 | **Athlete IDs are a shared DA/TFRRS namespace**: the same numeric ID used by the DA-side 2024 IL Top Times 1A list resolves on the TFRRS host to the identical profile (`Reece Curtis`, `St. Anne`, `Class: 15`) — IL HS athletes are reachable on TFRRS even though IL has no TFRRS subdomain and no IL HS team pages. **Its `Class: 15` is the same era-relative ordinal as the team-roster code**: the same ID is `SR` in the 2024 1A list and `JR` in the 2023 1A list, so `Class:` is an ordinal label (15 = that season's senior), not a cohort year | 23:31:15 |

| `tools/da-scratch/team-teams_track_{11939,43268}_html` (offline re-parse, no request) | — | — | **Team-page payload probe** (both IL team pages fetched earlier): each carries a `Team Roster` table with `Name` and `Year` columns and athlete links (Lane Tech 28 roster rows / 28 AthleteIDs, roster `Year` values `13|14|15|SO|SR`; A-C Central 17 / 17, values `23|24|25|26`), league membership as printed link text (`IHSA Class A/AA`, `Chicago Public Schools`, `IL Top Times-Class 1A/2A/3A`, `Illinois Charter/Other`), three sibling links to the school's other surfaces in each page (Lane Tech track/11940, xc/15964, xc/15965; A-C Central track/43269, xc/27271, xc/27272 — men's/women's and T&F/XC IDs are adjacent), and direct meet links (10 distinct `results/track/<meet_id>.html` on Lane Tech, 0 on A-C Central, matching its `No recent results` block). Roster `Year` mixes class labels with two-digit codes (`13`|`14`|`15`|`SO`|`SR` on Lane Tech; `23`|`24`|`25`|`26` on A-C Central) ⇒ not a grade. **No season scoping**: 0 `<form>`, 0 `<select>`, 0 `seasonSelect`/`config_hnd` refs on either page (contrast: IN's TFRRS roster is season-keyed). **The codes order by class but do not name it**: 3 of Lane Tech's 28 roster IDs appear in the 2024 3A list and map consistently `13`→`SO` (8823301), `14`→`JR` (8823343), `15`→`SR` (8823354) — so they are an era-relative ordinal, not graduation years and not current grades. **No footer contamination on the IL side**: the `Useful Links` tail carries 0 athlete links / 0 team links, so the 10,527 athlete rows and 1,193 relay rows exclude it, and each relay section's single `<th>` header row is excluded by the `<td>`-only count | 00:02–00:08 (re-parse) |

| `tools/da-scratch/*_l1000.html` (offline re-parse, no request) | — | — | **Row-shape + section-census discovery**: four athlete-row layouts (7 / 8+wind / 8+conversion / 9) obeying `width = 7 + conversion? + wind?`, wind always `NWI`, layout fixed per section (0 of 24 athlete-bearing sections mixed): 200m carries wind only, HJ/PV/Shot Put conversion only, LJ/TJ both, all other individual events 7 cells. Each list has **30** sections — the 6 relay sections (4x200/4x400/4x800 × 2) contain **zero athlete links** (legs are an unlinked comma string), i.e. trio = 10,527 athlete-linked rows + 1,193 relay rows = 11,720. A fixed-index parser mis-assigned the meet name as the date on 3,355 of 10,527 rows. Season mapping verified for **all 10,527 rows** (meet dates Jan–Mar 2024 only ⇒ at-meet `FR` = Class of 2027) | 23:44–23:58 (re-parse) |

Reproduction: `python3 tools/14-da-il-enumerate.py` (offline, parses the captures in
`tools/da-scratch/`); `--fetch` re-issues the same requests. Raw JSON extracts produced by this
assignment: `tools/da-scratch/il-all-team-ids.json` (1,746 IL league team IDs),
`tools/da-scratch/search-teams-union.json` (557k team-index rows),
`tools/da-scratch/il-toptimes-2024-all.json` (3,576 IL Top Times rows with athlete/team IDs, the
Top-50 default view). `python3 tools/14-da-il-enumerate.py --section classes` additionally reproduces
the disjointness and depth findings from the `*_l1000.html` captures (bare vs `limit=1000` per class,
cross-class overlap, FR counts, saturation check).
