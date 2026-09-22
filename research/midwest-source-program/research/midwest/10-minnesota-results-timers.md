# 10. Minnesota results/timers

Status: complete
Observed on: 2026-09-19

Assignment: identify the major Minnesota HS track/XC result providers and archives; qualify meet
enumeration, result format, athlete/grade/school fields, stable IDs, historical depth and whether
BULK results are downloadable; determine state/section/championship availability and direct
Athletic.net cross-links; classify access; estimate Athletic.net request avoidance.

**Headline:** Minnesota high-school meet results are produced by a small set of timers that publish
into **one shared platform — AthleticLIVE — which is Athletic.net's own live-results product.** Four of
the six MN timers observed (Wayzata, Hero's, Fast Finish, and the partially-absorbed GSE) resolve to
`*.anet.live` / `live.athletic.net` shells. That is *not* the dead end it looks like, because
AthleticLIVE exposes an **unauthenticated public JSON store on Azure Blob Storage** —
`https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<eventId>` — that returns a whole
event's results (place, mark, wind, PR, grade, team, **Athletic.net athlete ID `ani`**, Athletic.net
team ID) in **one request with no Cloudflare, no browser and no token**. Measured on the 2025 MSHSL
state XC championships: **6 requests → 959 result rows → 242 Juniors (Class of 2027), 242/242 carrying
their Athletic.net athlete ID.** Raceberryjam.com remains the only long-horizon MN archive (XC 1991+),
and MSHSL publishes per-class state-meet PDFs whose text-layer status is inconsistent (2026 T&F: text +
`Yr` grade column; 2025 XC: raster-only, OCR required).

---

### Source

| # | Provider | Own surface (verified) | Role |
|---|---|---|---|
| 1 | **Wayzata Results, LLC / Wayzata Timing** (Edina/Maple Grove area, MN) | `https://www.wayzataresults.com/` (PrestoSports CMS) → `https://results.wayzatatiming.com/` (**AthleticLIVE tenant**) | Dominant MN HS XC/T&F timer and the state-meet timer of record |
| 2 | **AthleticLIVE** (Athletic.net / RunnerSpace) | `https://live.athletic.net`, `https://livestatic.athletic.net`, `https://athleticlive.blob.core.windows.net` | The platform all MN live results land on; tenant map includes `*.anet.live` origins |
| 3 | **Apple Raceberry JaM** (Edina, MN) | `http://www.raceberryjam.com/` | Legacy archive/aggregator + scoring software vendor; MN XC archives 1991–2022 |
| 4 | **GSE Timing** | `https://gsetiming.com/` | MN HS XC 2005–2025 (608 meets). **T&F results 302-redirect to `https://athletic.net`** — merged into Athletic.net |
| 5 | **Hero's Timing** (Bemidji-area MN) | `https://www.herostiming.com/` → `https://live.herostiming.com/` (**AthleticLIVE tenant**) | MN + ND meets, year pages 2012–2026 |
| 6 | **Fast Finish Results** | `https://fastfinishresults.com/` → `https://live.fastfinishresults.com/` (**AthleticLIVE tenant**) | Own WordPress archive; two 2026 events link straight to `live.athletic.net` |
| 7 | **MTEC Results** (ChampionChip) | `https://www.mtecresults.com/` | MN road/XC; `${event}/leaderboard/<id>` pages; API docs are login-gated |
| 8 | **Pickle Events** (Northfield, MN) | `https://www.pickleevents.com/results/` | Meet director/consultant, mostly roads; index 2001–2026 pointing at MTEC/Athletic.net |
| 9 | **MileSplit Minnesota** (FloSports) | `https://mn.milesplit.com/results`, `/meets/<id>-<slug>/results` | Aggregator; full result depth is **PRO-gated** |
| 10 | **MSHSL** (state association) | `https://www.mshsl.org/tournaments/state-tournament-archives/**` | Official state-meet result PDFs per class; archive pages back to 2017 |
| 11 | *Hy-Tek MEET MANAGER / MeetPro* | format, not a host | Output format of MSHSL PDFs and of the raceberryjam archive pages |

`mtrack.timing`-style aggregators and Rainbow Racing (a race-number vendor named on raceberryjam) were
not observed as MN HS result providers.

### Coverage

| Provider | States | Sports | Levels | Historical depth (verified) | 2025-26 volume (verified) |
|---|---|---|---|---|---|
| Wayzata / AthleticLIVE | MN primary; also IA, WI, ND, IL meets | XC, indoor T&F, outdoor T&F | HS + MS + college in the same feed | season schedule pages resolve for **2023, 2024, 2025, 2026** | **distinct meets with results** — XC: 136 (2025), 124 (2024), 116 (2023), 49 (2026, season in progress). T&F: 319 (2025), 334 (2024), 305 (2023), 339 (2026) |
| GSE Timing | MN | XC only (T&F → athletic.net) | HS + MS | **608 XC meets, 2005–2025** in one index page | 42 XC meets in 2025; 270 since 2021 |
| Hero's Timing | MN + ND | XC, T&F | HS + MS | year pages **2012 … 2026** | **256** `live.herostiming.com/meets/<id>` links on the 2026 page |
| Fast Finish Results | MN | XC, T&F | HS | archives 2023, 2024, 2025 | 2026 page links both `live.fastfinishresults.com/meets/<id>` and `live.athletic.net/meets/<id>` |
| Raceberryjam | MN (upper Midwest) | XC + T&F (roads too) | HS + NCAA DIII + MIAC | **XC archive years 1991–2022, "over 500 meets"**; MSHSL championships since 1991 | **current-season index stale since 2025-10-08** — see caveat |
| MTEC Results | MN + ND/SD/IA/WI (road-heavy) | XC, roads, some T&F | HS + open | per-event pages; `/documentation/exports` exists but is login-gated | event ids in the 6000s observed |
| Pickle Events | MN (+NW WI) | XC, roads | HS + open | results index **2001 … 2026** | 2026 page: 98 rows, 90 → `mtecresults.com/race/leaderboard/<id>`, 36 → RunSignup, **1 → athletic.net** |
| MileSplit MN | MN (+ border states) | XC, indoor/outdoor T&F | HS + MS + college | `/results` month index for **2026 and 2025** | Sept 2026 XC index: Osceola, Roy Griak, Somerset, Pleasant Valley, Oxbow Lake, … |
| MSHSL (PDFs) | MN | XC, T&F (+ all activities) | HS only | archive pages **2017 … 2026** per sport, plus records/champions PDFs | 3 class PDFs per state meet (A/AA/AAA), boys + girls |

**Caveat on composition:** every Wayzata season count is *all events that timer scored*, not MN HS only.
Of the 50 XC event rows (49 distinct meets) on the 2026 page, **35 are Minnesota high-school meets**; the
rest are Iowa HS (5), Wisconsin HS (4) and collegiate/club (6 rows, incl. Roy Griak and Nike Heartland).

**Coverage gap — MSHSL section championships are split across timers.** MSHSL organises sections as
8 geographic sections × 3 competition classes (`A`, `AA`, `AAA`) = 24 section championships per sport
[naming scheme observed directly: `1A…8AAA`; the existence of all 24 is INFERENCE from the scheme plus
the 3 verified state-meet classes], and **no single MN timer scores all of them**. Wayzata's own schedule
contains only a subset:

| Season | MSHSL section championships in Wayzata's schedule | MSHSL State meet | True Team rows |
|---|---|---|---|
| XC 2025 | **12 distinct** — 1A, 1AA, 1AAA, 2AA, 2AAA, 3A, 4AA, 4AAA, 6A, 6AA, 6AAA, 8AAA (missing 2A, 3AA, 3AAA, 4A, 5A, 5AA, 5AAA, 7A, 7AA, 7AAA, 8A, 8AA) | 1 (Nov 1 2025 → MeetID 58505) | 0 |
| T&F 2025 | 10 distinct — 1A, 1AA, 1AAA, 2AA, 4AA, 4AAA, 5AAA, 6A, 7AAA, 8AAA | 3 days (Jun 10–12 2025) | 12 |
| T&F 2026 | 11 distinct — 1A, 1AA, 1AAA, 2AA, 3A, 4AA, 4AAA, 5AA, 5AAA, 7AAA, 8AAA | 3 days (Jun 4–6 2026 → MeetID 74652) | 15 |

Verified complementarity: **Section 5A XC 2025 is absent from Wayzata's schedule and is scored by GSE
Timing** (`gsetiming.com` `meet_id=1367` → "Section 5A CC 2025"). So a Wayzata-only collector silently
misses roughly half the MN championship-qualifying-meet line; the remaining sections must be sourced from
GSE, Hero's Timing, Fast Finish, MTEC or Athletic.net (the timers for those sections were not enumerated
in this slice — see Caveats 10). MSHSL state meets themselves are covered: all three
Athletic.net class links and all six event documents were reachable from the single state XC MeetID.

### Enumeration

**1. Timers' own season schedules — the cheapest entry point (static HTML, 1 request/season).**

`GET https://www.wayzataresults.com/sports/{xc|track|rr}/{YYYY}/schedule` returns a server-rendered
table. Each meet row is one anchor whose `aria-label` carries date + meet name + venue and whose `href`
is a short link:

```
<a aria-label=" event: November 1 12:00 AM: MSHSL State Cross Country Championships at Les Bolstad Golf Course: Results"
   href="/links/q31ha2" ...>Results</a>
```

Counts of distinct `/links/<slug>` hrefs per page (one request each, 2026-09-19). Note the mapping is
**many-to-one**: `/links/<slug>` identifies a *MeetID*, so a two-day meet listed twice on the schedule
shares one slug (2026 XC: 50 rows, 49 slugs — Roy Griak Day 1 and Day 2 both → `z7wnx4` → MeetID 77476).
The table counts distinct slugs; the parenthesised figure is raw anchor occurrences.

| Page | distinct `/links/` (occurrences) | | Page | distinct `/links/` (occurrences) |
|---|---|---|---|---|
| `/sports/xc/2023/schedule` | 116 (117) | | `/sports/track/2023/schedule` | 305 (340) |
| `/sports/xc/2024/schedule` | 124 (125) | | `/sports/track/2024/schedule` | 334 (379) |
| `/sports/xc/2025/schedule` | 136 (138) | | `/sports/track/2025/schedule` | 319 (367) |
| `/sports/xc/2026/schedule` | 49 (50) | | `/sports/track/2026/schedule` | 339 (379) |

**2. Short link → meet ID.** `GET https://www.wayzataresults.com/links/<slug>` redirects to
`https://results.wayzatatiming.com/meets/<meetId>`. Verified: `q31ha2 → 58505` (2025 MSHSL state XC),
`7b7yqe → 58374` (2025 Section 2AAA), `c9g8oy → 74652` (2026 MSHSL state T&F), `oerb45 → 77477`
(Toni St. Pierre 2026), `zsjlfs → 77061` (Bauman/Rovn 2026). Slugs are opaque and stable; there is no
bulk slug→id list.

**3. Meet → events.** `https://results.wayzatatiming.com/meets/<id>` is an **Angular SPA**: the HTML is a
byte-identical 50,184-byte shell for every meet id, containing no meet data and no event ids. Rendered,
the page exposes per-event anchors `/meets/<id>/events/individual/<eventId>` and
`/meets/<id>/events/relay/<eventId>`, plus `/teams`, `/athletes`, `/live`, `/reports/winners`.
The meet's own structure is read at runtime from a **Firebase Realtime Database** namespace
(`trackmeet-io`) — observed request:
`https://s-gke-usc1-nssi3-33.firebaseio.com/.lp?dframe=t&id=<id>&ns=trackmeet-io`. (Token values are
deliberately not reproduced here.) Practical consequence: **event-ID discovery needs one browser render
per meet**, after which the result payloads are plain public JSON.

**4. Event → results (bulk, no browser).** The SPA itself fetches
`https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<eventId>` — an Elasticsearch-style
document with `_source.r[]` holding every result row. `event_list/_doc/<eventId>` returns the event
header. Both indices are keyed by **event** id (verified: `_doc/60118` and `_doc/77477` resolve to
unrelated old meets, proving the key is not a meet id). Container listing is disabled
(`$web?restype=container&comp=list` → 404 `ResourceNotFound`), so there is no index dump; event ids must
come from the rendered meet page.

**5. Raceberryjam.** `http://www.raceberryjam.com/` → `maincc.html` (XC) / `mainod.html` (T&F) for the
current season; `xcarchives.html` / `archives.html` for the archive, with year anchors 1991…2022 and
per-meet relative links (`2022/boysaaa.html`). A POST search form (`xcarchives.php` → `xcarchives2.php`,
`select name="meet"`) browses "over 500 meets" by name.

**6. GSE Timing.** `GET https://gsetiming.com/results/cc_rslts/cc_rslts.asp?sport=cc` returns a `<select
name="meets">` with **609 options** (= meet ids + a blank). Selecting one POSTs to the same ASP and
302-redirects to
`https://gsetiming.com/results/cc_rslts/adv_rslts.asp?meet_id=<id>&race_id=<id>&sport=Cross-Country`.
The individual results table is DataTables-loaded from
`https://gsetiming.com/results/cc_rslts/adv_source.asp?race_id=<raceId>`, which returns a JSON array.

**7. MileSplit MN.** `https://mn.milesplit.com/results` is a month-grouped index (2026, 2025) of
`/meets/<id>-<slug>/results` links; the results table itself is client-rendered.

**8. MSHSL.** Per-sport, per-year archive pages list 3 class PDFs each, e.g.
`/tournaments/state-tournament-archives/state-tournament-archive-boys-cross-country-2025` →
`sites/default/files/2025-11/2025-class-{aaa,aa}-boys-cross-country-results.pdf` and
`sites/default/files/2025-11/results.pdf` (Class A), plus champions/participants PDFs. School universe:
`/schools?page=0..8` (≈450 schools; report 09 owns the JSON:API detail).

**Class of 2027 specifically:** yes, deterministically. AthleticLIVE result rows carry a grade token
(`a.y`) and the MSHSL state XC 2025 six-event set contains **242 Juniors**, i.e. Class of 2027.

### Stable identifiers

| ID | Where | Stability | Evidence |
|---|---|---|---|
| `MeetID` (AthleticLIVE) | `/meets/<id>`, blob `mi` | Stable per tenant; global numeric (74,652 / 77,477 / 58,505 observed across 2022–2026) | `_source.mi` = 77477 for the 2026 Toni St. Pierre meet |
| `EventID` (AthleticLIVE) | `/meets/<m>/events/{individual,relay}/<id>`, blob doc key | Stable; **the join key for bulk result fetch** | `ind_res_list/_doc/2912840` → `i:2912840, mi:77061` |
| `ResultID` | `_source.r[].i` | Stable per row | `45752881`, `33845830` |
| `AthleteEntryID` | `_source.r[].a.i` | **Meet-scoped** (carries `mi`) | `a.i=43632615, a.mi=60118` |
| **`AthleteID` (Athletic.net)** | `_source.r[].a.ani` | **Global** — same namespace as Athletic.net profile URLs | 242/242 Juniors in state XC carried one (e.g. `23733333`) |
| `TeamEntryID` | `_source.r[].a.t.i` | Meet-scoped | `t.i=1771139, t.mi=77477` |
| **`TeamID` (Athletic.net)** | `_source.r[].a.t.ani` | Global (e.g. Eastview `12638`, Blaine `12275`) | Plus `a.t.f` = team name string |
| `SessionID` | `_source.se.i`, with `.ds` date | Per meet/day | `se={ds:"2026-06-04", i:139820, mi:74652, n:"Class AAA Prelims"}` |
| `DivisionID` | `_source.dv.i` | Per meet division | `dv={i:86700, mi:60118, n:"Future Tracksters"}` |
| GSE `meet_id` / `race_id` | `adv_rslts.asp?meet_id=&race_id=` | Stable numerics | `meet_id=1367, race_id=7784`; also `Bib` per athlete |
| MileSplit `MeetID` | `/meets/<id>-<slug>/...` | Stable numeric + slug | `779650-toni-st-pierre-invitational-2026` |
| MSHSL | slug only | No numeric ids in result PDFs | `/schools/<slug>`, `/tournaments/...-2025` |
| Raceberryjam | year + filename only | No ids at all | `2022/boysaaa.html` |

Marks and "normalized mark inputs": every AthleticLIVE row carries both the display mark (`m`, e.g.
`"10.75Q"`) and an integer time in **milliseconds** (`im`, e.g. `10743`) or, for field events, in
0.0001-inch-like units (`im:6400800` with `m:"21-00.00"`). That is a pre-normalized input, not a string
to re-parse.

### Athletic.net leverage

**Direct Athletic.net links — yes, first-class.** Every rendered AthleticLIVE meet page has a
`View on AthleticNET` anchor pointing at the canonical Athletic.net meet URL, *per class* where the
timer splits a state meet:

| AthleticLIVE meet | Athletic.net target | Observed |
|---|---|---|
| 58505 — MSHSL State XC, 2025-11-01 | `https://www.athletic.net/CrossCountry/meet/268358` (Class A), `.../268359` (AA), `.../268360` (AAA) | rendered |
| 74652 — MSHSL State T&F, 2026-06-04..06 | `https://www.athletic.net/TrackAndField/meet/666673` | rendered |
| 77061 — Bauman/Rovn Invitational, 2026-09-10 | `https://www.athletic.net/CrossCountry/meet/280000` | rendered |
| 77477 — Toni St. Pierre Invitational, 2026-09-18 | `https://www.athletic.net/CrossCountry/meet/284283` | rendered |

**Athletic.net athlete/team links — better than links: the raw IDs.** `r[].a.ani` and `r[].a.t.ani` are
the Athletic.net athlete and team identifiers (in the same numeric namespace the repo already keys on).
So this source does not merely *seed* an Athletic.net lookup — for every athlete at a covered meet it
supplies the exact identifier, meaning zero Athletic.net search requests are needed for those athletes.
Not observed anywhere in this slice: relay-leg membership in the public JSON (`relay_res_list`,
`relay_list`, `relay_results` all 404) — treat relay membership as unqualified.

**Estimated avoidance (measured, not modelled).**

- 2025 MSHSL state XC, boys+girls, A/AA/AAA: **6 blob requests → 959 rows → 959 Athletic.net athlete
  IDs → 242 Class-of-2027 records.** ≈ **0.025 requests per Class-of-2027 athlete** vs. ≥1 (usually
  ≥2–3) Athletic.net requests per profile refresh. This is the entire state-qualifier cohort of MN XC.
- Bauman/Rovn Invitational 2026 (one regular-season meet, boys varsity 5000 m): **1 request → 121 rows,
  121 `ani`, 41 Juniors.**
- Whole-season extrapolation [INFERENCE]: Wayzata's 2025 XC schedule lists **136 distinct meets** with
  results links; at ~4 event documents/meet that is ≈ **540 blob requests for the full MN XC season's
  results**, all with grade + Athletic.net IDs, and zero requests to `www.athletic.net`. (Cross-check on
  the per-meet assumption: Bauman/Rovn 2026 has 6 events but Toni St. Pierre 2026, a college meet, has
  only 2 — so ~4 may be an over- rather than an under-estimate.)
- The *same* data via Athletic.net would require per-athlete profile/meet requests against a
  Cloudflare-gated host. On the retained 2025-26 boys grade-11 corpus (142,705 athletes, mission brief
  §2), the state-meet slice alone replaces ~242 profile refreshes with 6 requests; a full MN HS XC
  season replaces an estimated 15k–25k athlete-meet result rows with ~550 requests [INFERENCE, driven by
  the measured 959 rows/6 requests and 121 rows/1 request rates].

**Countervailing note (do not let this be lost):** AthleticLIVE *is* Athletic.net infrastructure
(`livestatic.athletic.net`, "© 2026 RunnerSpace.com … Athletic", Elasticsearch indices literally named
`wayzata`, `wayzata_reports`). Using it avoids Cloudflare and the `/api/v1/**` search surface, but it is
not a third-party source; treat it as an **unblocked Athletic.net read path**, and check it against the
pipeline's own terms-of-use position before productionising.

### Athlete evidence

| Field | AthleticLIVE blob JSON | GSE `adv_source.asp` | Raceberryjam archive | MSHSL PDF | MileSplit MN |
|---|---|---|---|---|---|
| name | `a.n` + `a.fn`/`a.l` | `First`, `Last` | `<pre>` "Name, Gr" | INDIVIDUAL RESULTS | `<pre>` raw block |
| grade / class | **`a.y`** — `"SR"/"JR"/"SO"/"FR"` **or** `"12"/"11"/"9"/"8"/"7"` (vocabulary is per-meet!) | **`Gr`** column (`"11"`, `"12"`) | grade token on every finisher (`Jr`, `So`, `Sr`) | **`Yr`** column (2026 T&F); absent from the 2025 XC raster | `Yr` column |
| school | `a.t.f` / `a.t.n` + Athletic.net team id | `Team` (abbrev, e.g. `LIT`) | team blocks per school | School column | `Team` |
| city/state | `a.cco` (country) only; city comes from the meet location | — | meet venue + city in the header line | meet site in header | meet site |
| gender/category | `a.g`, event `gl`/`sg` | `M/F` column | "Girls"/"Boys" headers | "Boys/Girls … Class X" | Girls/Boys sections |
| TF vs XC | `xc` boolean on the event doc | XC index only | separate XC/T&F archives | per sport | per meet |
| indoor/outdoor | not a field — inferred from date/venue; `mms` carries the timing program (`"RunMeet"`, `"HY-FTP"`) | n/a | XC archive vs T&F archive | per sport | per meet |
| performances | `r[].m` + `r[].im` | `Time` | mark | mark | mark |
| PRs | **`r[].pr`** (athlete PR at that moment) | — | — | — | athlete pages (PRO) |
| progression | interval splits `r[].irs[]` + `avk`/`avm` pace | — | 1-mile/2-mile splits on state files | 1/2-mile splits (2024 XC) | — |
| meets | `/meets/<id>/athletes`, `/meets/<id>/teams` routes + the schedule pages | index page | archive index | archive pages | month index |
| profile URL | athletic.net/athlete/`<ani>` (id supplied; URL pattern is the repo's known one) [INFERENCE on URL shape] | — | — | — | `mn.milesplit.com/athletes/**` |

Grade vocabulary is the single biggest parsing hazard: the *same* platform emits `"JR"` for one meet and
`"11"` for another (both observed on MSHSL state meets one season apart). Normalise both.

### Recruiting information

**No MN result provider exposes coach or AD data.** Checked and negative on: Wayzata
(`wayzataresults.com` is a schedule/results CMS with an `/about/aboutus` page only), AthleticLIVE meet
pages, GSE Timing, Hero's Timing, Fast Finish, MTEC, Pickle Events, MileSplit MN meet pages, and the
MSHSL result PDFs (which name the *timing company*, not coaches). Raceberryjam's "Coaching
Opportunities" and "Summer Camps" pages are job/camp postings, not a directory.

The authoritative MN coach surface is MSHSL itself — `mshsl.org/schools/<slug>` renders an
**Activities Director** block for Ada-Borup-West High School, and the page ships a Drupal state object
with per-school coach flags (`is_fine_arts_coach`). Coach name/role/professional-email extraction is
**report 09's assignment**; this report deliberately does not duplicate it.

### Result evidence

Availability per field, from the AthleticLIVE `ind_res_list` document (the richest surface here):

| Field | Present | Notes |
|---|---|---|
| ResultID | yes | `r[].i` |
| AthleteID | yes | `a.i` (meet-scoped) **and** `a.ani` (Athletic.net, global) |
| MeetID | yes | `mi` (top level and inside athlete/team objects) |
| EventID | yes | `i` on the event document; also the doc key |
| mark | yes | `r[].m` display, `r[].im` integer ms (time) or 1e-4-in units (field) |
| normalized mark inputs | yes | `im`; field events additionally expose attempts in `r[].hs[]` with `att`/`co`/`im` per attempt |
| timing method | **no explicit field** — inferred: `mms` names the timing program (`RunMeet`, `HY-FTP`); MSHSL PDFs say "Hy-Tek's MEET MANAGER" | [INFERENCE] |
| wind | **yes** — `r[].w` (`"-2.1"` on the 2026 state 100 m prelims) | |
| implement/hurdle spec | partially: `un`/`ab` (`"100 - Prelims"`, `"LJ"`, `"55m"`), `peg` event group (`Sprint`, `Field`) — no shot-put weight / hurdle-height field observed | |
| heat/round | yes | `hn` heat, `hl` lane, `ro` round order, `runm` (`Finals`, `Preliminaries`), `rui` (`"40-1"`) |
| place | yes | `r[].p`, points `r[].pt` |
| date | yes | `se.ds` (session date), `sd` (start), `ua` (last update) |
| school represented | yes | `a.t.f`/`a.t.n` + Athletic.net team id |
| relay membership | **not confirmed** — `relay_res_list`/`relay_list`/`relay_results` all 404; relay events exist as `/events/relay/<id>` but the public JSON index for them was not found | gap |
| PR / season context | yes | `r[].pr`, `er` (entry/seed), `gap`/`iv` (delta to next) |

Other providers: GSE gives Pl/Bib/First/Last/Team/Gr/M-F/Time + team scoring table (no wind, no rounds
beyond the race, no ids). Raceberryjam gives Hy-Tek-style place/name+grade/splits/finish and a full
team-scoring pre-block (no ids). MSHSL 2026 T&F PDFs give Name/`Yr`/bib/school/finals-mark/place/points.
MSHSL 2025 XC PDFs give nothing machine-readable.

### Incremental use

1. **New meets** — `GET /sports/xc/{YYYY}/schedule` (and `/sports/track/`) is one static request per
   sport per season. Compare the `/links/<slug>` set against the previous week's set: new slugs = new
   meets; the `aria-label` supplies date + name + venue + state inference. Two requests/week covers
   both sports for the current season.
2. **Changed meets** — event documents carry `ua` (last update), `frua`/`fruam` (first result update),
   `sd` (start), `nu` (result count) and `nrds`/`ane`. Because those are inside the event document, a
   change check costs one request per event. For a season-sized watch list (~540 XC event documents) that
   is ~540 conditional GETs/week — cheap in absolute terms, but it is a full re-read of the corpus, so
   gate it: only poll events whose meet date is within the last N days, or index `ua` once per meet at
   the end of the season.
3. **New results only** — `r[].i` (ResultID) is stable; a diff against the stored row-id set per event
   yields exactly the added rows. No re-derivation needed.
4. **Affected athletes** — each row already carries `a.ani`, so "affected athletes" is a set union over
   changed rows; no name/school re-resolution.
5. **Cheap completeness check** — `nu` (number of results) vs. the stored row count detects silent
   edits, not just additions.
6. **Avoid**: re-fetching the historical archive (`xcarchives.html`, 1991–2022) — it is static and can
   be ingested once.

### Access characteristics

| Host | Class | Observed |
|---|---|---|
| `athleticlive.blob.core.windows.net` | **public structured JSON** (Azure Blob, unauthenticated) | 50 requests, all 200, application/json, ~0.1–0.5 s each; no 429, no `Retry-After`, no Cloudflare. Container listing disabled (404) |
| `results.wayzatatiming.com` / `live.herostiming.com` / `live.fastfinishresults.com` / `live.athletic.net` | **browser application** (Angular SPA, identical 50,184-byte shell) | 200 to curl; data requires a browser render; per-event JSON obtainable without one |
| `www.wayzataresults.com` | normal HTML (PrestoSports) — **UA-gated** | curl default UA → **403**; browser UA → 200 (2.7 MB landing). 22 requests with browser UA, all 200, no 429 |
| `raceberryjam.com` | normal HTML + static archive files | 12 requests, all 200, Apache; `Last-Modified` present |
| `gsetiming.com` | normal HTML + ASP JSON | 7 requests, all 200 for `sport=cc`; `sport=tf` → **302 to `https://athletic.net`**; note a POST+`-L` combination returns HTTP 411 |
| `www.herostiming.com` | normal HTML (Weebly) | 2 requests, 200; year pages 2012–2026 |
| `fastfinishresults.com` | normal HTML (WordPress) | 3 requests, 200 |
| `www.mtecresults.com` | normal HTML (Bootstrap) | API documented at `/documentation/api` but **login-gated** → treat the API as authenticated |
| `www.pickleevents.com` | normal HTML | 2 requests, 200 |
| `mn.milesplit.com` | normal HTML + browser app; **PRO subscription** for full result depth | 200; results table client-rendered; page loads reCAPTCHA + ~20 ad/analytics origins |
| `api30.milesplit.com` | documented-ish host, routes not discoverable anonymously | `GET /v3/meets/779650`, `/v3/meets/779650/results`, `/v1/meets/779650`, `/v3/meets/779650/events` → **404** (Symfony "No route found") |
| `www.mshsl.org` | normal HTML (Drupal 11) + downloadable PDF | 13 requests, all 200 |
| `www.athletic.net`, `athletic.net` | **UA-gated, not blanket-blocked** | default curl UA → **403** (5,343 B); browser UA → **200** (28,026 B marketing root). Consistent with this report's other UA-gated hosts; not used for content here |
| `static.trackmeetio.com` | downloadable PDF | `meetFiles/<meetId>/<hash>.pdf`, e.g. 58505 → `690682f91381a.pdf` |

No `429` or `Retry-After` was observed on any host. All fetching was sequential with ≥0.3 s spacing,
≤50 requests per host.

### Recommendation

| Provider | Verdict | Why / expected marginal coverage |
|---|---|---|
| **AthleticLIVE blob JSON** (`athleticlive.blob.core.windows.net` + timer schedules) | **RESULT-SOURCE — PRIMARY implementation target** | Measured: 959 rows / 242 Class-of-2027 / 959 Athletic.net athlete IDs for 6 requests on one state meet; every row already carries grade + Athletic.net athlete and team IDs. Covers Wayzata (the state-meet timer), Hero's Timing, Fast Finish and (for live data) GSE. This is also the answer to report 03's open question — AthleticLIVE is now **qualified**, not "unqualified". |
| **Wayzata Results schedule pages** (`wayzataresults.com/sports/**`) | **DISCOVERY-ONLY** (mandatory front-end to the above) | 1 request/sport/season enumerates the meet list; the `/links/<slug>` → MeetID redirect is the join key. No results of its own. |
| **MSHSL state-tournament archive** (`mshsl.org`) | **VALIDATION** | Authoritative, independent of the timer, but PDF-only. 2026 T&F PDFs are text-extractable with a `Yr` grade column — usable for state-qualifier cross-checks. |
| **Raceberryjam** (`raceberryjam.com`) | **VALIDATION / DISCOVERY-ONLY** | Only long-horizon MN archive (XC 1991–2022, MSHSL championships since 1991) and grade-tagged; ideal for Class-of-2027 *back-fill* (2024–2025 files) and timer attribution. **But its live index is dead** — `maincc.html` last modified 2025-10-08 and carries no 2025 state-meet result; do not design a weekly collector on it. |
| **GSE Timing** (`gsetiming.com`) | **RESULT-SOURCE (XC only)** | 608 MN XC meets 2005–2025 with `Gr` + Bib in one JSON endpoint per race. Independent of AthleticLIVE. Its T&F surface is gone (302 → athletic.net), so XC only. |
| **MileSplit MN** (`mn.milesplit.com`) | **DISCOVERY-ONLY** | Good meet/date index and a `Yr`-bearing raw results block for some meets, but full depth is PRO-gated and the results table is client-rendered. Belongs to reports 07/27; use as a *cross-check of meet existence*, not as the bulk pipe. |
| **MTEC Results / Pickle Events** | **DISCOVERY-ONLY** | Mostly roads and non-HS; MTEC's API is login-gated. Pickle's index is a useful map of which vendor timed which meet. |
| **MSHSL 2025 XC PDFs** | **REJECT (as machine input)** | Raster-only: 0 fonts, 0 text operators, `pdftotext` → 2 bytes. OCR cost exceeds the value when AthleticLIVE has the same data as JSON. |

**Smallest useful MN set:** Wayzata schedule pages (2 requests/week) → AthleticLIVE event documents
(~6/meet) → MSHSL PDFs for state validation. That chain yields, for every covered MN HS XC/T&F meet,
athlete name + school + grade + Athletic.net athlete ID + mark, with no request to `www.athletic.net`.

## Caveats that must survive into the synthesis

1. **AthleticLIVE is Athletic.net.** This source removes the Cloudflare/search cost, not the Athletic.net
   relationship. Anyone reading "avoid Athletic.net" should see that sentence next to this one.
2. **Event-ID discovery still needs a browser render per meet.** ~1 render/meet is the residual cost; the
   result payloads themselves are curl-able.
3. **Grade vocabulary is inconsistent across meets** (`JR` vs `11`); normalise both, plus `7`/`8`.
4. **Counts are not MN-HS-only.** 35 of the 50 XC event rows (49 distinct meets) on the 2026 page are
   Minnesota high-school meets; the rest are Iowa/Wisconsin HS or college.
5. **Relay membership is unqualifed** in the public JSON (three relay index names all 404).
6. **Raceberryjam's live index is stale** (2025-10-08); its value is historical, not incremental.
7. **`ani` cross-meet stability is [INFERENCE].** Within-meet consistency and the namespace match to
   Athletic.net athlete ids were observed; a same-athlete-appearing-at-two-meets check was not run.
8. **Tool limitation encountered:** every vision-model call (`?q=` image query) failed with
   `429 Token Plan usage limit reached: Upgrade your Token Plan or purchase Credits for more usage. (2056)`.
   The 2025 XC raster-PDF finding therefore rests on non-visual evidence (font table, text-operator
   scan, `pdftotext` byte count), not on reading the rendered page.
9. **`www.athletic.net` returned 200 to a browser UA on this machine** (403 to a default curl UA). The
   mission brief's "Cloudflare-403 to non-browser clients" is UA-conditional; recorded here because it
   changes what future agents will see. No Athletic.net content was scraped for this report.
10. **The MN section-meet line is not one collector.** Wayzata covers 12 of the 24 MSHSL XC sections and
    10–11 of the T&F sections; the rest were timed by providers not enumerated in this slice. Closing
    that gap needs a per-section timer sweep (start from `raceberryjam.com/maincc.html`'s timer list,
    which names Wayzata, GSE, Hero's, Fast Finish, MTEC and Pickle, and from the MSHSL section pages).
    Total MeetIDs to chase is bounded: ≤24 sections × 2 sports × 12 years.

### Evidence appendix

| URL | method | HTTP status | what it proved | timestamp (CDT) |
|---|---|---|---|---|
| `http://www.raceberryjam.com/` | GET | 200 (6,623 B) | Root; RainberryJaM is an Edina MN scoring company; nav to `indexod/indexcc/today/archives` | 2026-09-19 23:13 |
| `.../maincc.html` | GET/HEAD | 200 (30,241 B); `Last-Modified: Wed, 08 Oct 2025 02:30:38 GMT` | XC index; timer links (Wayzata, GSE, Hero's, Fast Finish, MTEC, Pickle, Rainbow); body says "Last updated October 7, 2025"; **no 2025 state-meet results present** | 23:14 |
| `.../mainod.html` | GET/HEAD | 200 (11,989 B); `Last-Modified: Wed, 05 Mar 2025` | Outdoor T&F index; body says "Last updated June 15, 2023"; state T&F page names "Wayzata Timing" | 23:26 |
| `.../today.html` | GET/HEAD | 200 (59,255 B); `Last-Modified: Sat, 08 Aug 2026 03:36:07 GMT` | "Live" results page is still maintained in 2026 (Upsala Heritage 5K, Aug. 8) — the site is dormant per-sport, not abandoned | 23:14 |
| `.../xcarchives.html` | GET | 200 (614,641 B) | XC archive index: year anchors 1991–2022, "over 500 meets", 244 internal result-file links | 23:15 |
| `.../archives.html` | GET | 200 (111,596 B) | Results archive: MSHSL XC + T&F, NCAA DIII, MIAC; "MSHSL Championships since 1991" | 23:15 |
| `.../xcarchives.php` | GET | 200 (6,436 B) | POST search form (`action="xcarchives2.php"`, `select name="meet"`) over the archive | 23:15 |
| `.../xcarchives.php3` | GET | 404 | Dead relative link inside `xcarchives.html` (not a working surface) | 23:16 |
| `.../2022/boysaaa.html` | GET | 200 (28,445 B) | MSHSL Class AAA boys XC 2022 in Hy-Tek-style `<pre>`: `PLACE SCORE FINISHER` rows like `Dominic Haugland, Sr … 16:05.3`, plus team scoring and 1/2-mile split blocks — grade is embedded in the finisher token | 23:15 |
| `https://www.wayzataresults.com/` | GET (default curl UA) | **403** (919 B) | Reproduces the brief's `wayzataresults.com` 403 for non-browser clients | 23:14 |
| `https://www.wayzataresults.com/landing/index` | GET (browser UA) | 200 (2,729,258 B) | Root is a PrestoSports CMS; 451 distinct `/links/<slug>` meet links; outbound nav to `/sports/{xc,track,rr}/2026/schedule` | 23:14 |
| `.../sports/xc/2026/schedule` | GET | 200 (373,440 B) | **50** `/links/` event rows / **49 distinct slugs** for the 2026 XC season (35 Minnesota HS meets); Roy Griak Day 1 + Day 2 share slug `z7wnx4` | 23:16, re-verified 23:27 |
| `.../sports/track/2026/schedule` | GET | 200 (1,031,235 B) | **339** distinct `/links/` links (379 rows); **11 distinct MSHSL sections** (1A,1AA,1AAA,2AA,3A,4AA,4AAA,5AA,5AAA,7AAA,8AAA) + 3 MSHSL State T&F day rows + 15 True Team rows | 23:16, re-verified 23:27 |
| `.../sports/track/2025/schedule` | GET | 200 (987,195 B) | 367 rows / **319 distinct slugs**; **10 distinct MSHSL sections** (1A,1AA,1AAA,2AA,4AA,4AAA,5AAA,6A,7AAA,8AAA) + 3 State T&F day rows (Jun 10–12 2025) + 12 True Team rows | 23:22, re-verified 23:27 |
| `.../sports/xc/2025/schedule` | GET | 200 (404,878 B) | 138 rows / **136 distinct slugs**; contains **12 distinct MSHSL Section championships** (1A,1AA,1AAA,2AA,2AAA,3A,4AA,4AAA,6A,6AA,6AAA,8AAA — 14 rows incl. JV/Varsity pairs) + 2 section previews + 1 MSHSL State XC row + 3 out-of-state sectionals (IHSA, WIAA ×2) | 23:22, re-verified 23:27 |
| `.../sports/xc/2024/schedule`, `.../sports/xc/2023/schedule` | GET | 200 (369,799 B / 356,711 B) | **124** / **116** links — season archives are static HTML back to at least 2023 | 23:22 |
| `.../sports/track/2025|2024|2023/schedule` | GET | 200 (987,195 / 1,017,941 / 918,232 B) | **319 / 334 / 305** links | 23:22 |
| `https://www.wayzataresults.com/links/{q31ha2,7b7yqe,c9g8oy,oerb45,zsjlfs,z7wnx4,fnyotn,qxdbh0,yzez24}` | GET (-L) | 200; redirects | `/links/<slug>` → `results.wayzatatiming.com/meets/<id>`: 58505, 58374, 74652, 77477, 77061, 77476, 76863, 74479, 60118 | 23:16–23:31 |
| `https://results.wayzatatiming.com/meets/60118` | GET | 200 (50,184 B) | Title **"AthleticLIVE"**; scripts from `livestatic.athletic.net`; tenant config `elasticSearchIndex: "wayzata"`; footer "© 2026 RunnerSpace.com … Athletic" | 23:14 |
| `https://results.wayzatatiming.com/meets/77477` (and `/meets/60118`) | GET | 200, byte-identical 50,184 B | Meet pages are a pure SPA shell — **no** meet id, no event ids, no data in the HTML | 23:16 |
| `https://results.wayzatatiming.com/meets/77477` | browser render | 200 | Toni St. Pierre Invitational, Sep 18 2026, River Oaks GC, Cold Spring MN; `View on AthleticNET → https://www.athletic.net/CrossCountry/meet/284283`; events 2918044/2918045 | 23:31 |
| `https://results.wayzatatiming.com/meets/77061` | browser render | 200 | Bauman/Rovn Invitational, Sep 10 2026, Gale Woods Farm, Minnetrista MN; `→ https://www.athletic.net/CrossCountry/meet/280000`; 6 events 2912839–2912844; complete-results PDF at `static.trackmeetio.com/meetFiles/77061/6aa34c8a30c87.pdf` | 23:33 |
| `https://results.wayzatatiming.com/meets/58505` | browser render | 200 | 2025 MSHSL State XC: **three** Athletic.net links (`CrossCountry/meet/268358|268359|268360`); events 2152386–2152391; `Compete Results` PDF `static.trackmeetio.com/meetFiles/58505/690682f91381a.pdf` | 23:38 |
| `https://results.wayzatatiming.com/meets/74652` | browser render | 200 | 2026 MSHSL State T&F (Jun 4–6, 2026, St. Michael-Albertville): `→ https://www.athletic.net/TrackAndField/meet/666673`; 31 event links under `/events/individual/<id>` and `/events/relay/<id>`; reports routes `/scores`, `/records`, `/mvp`, `/winners` | 23:40 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2202020` | GET | 200 (3,391 B) | Event document schema: `i` eventId, `mi` meetId, `n`, `gl`, `xc`, `runm`, `se{}`, `dv{}`, `r[]` with `hn/i/p/s/im/m/w` and `a{i,n,y,g,ani,cm,t{...}}` | 23:17 |
| same, `_doc/2912840` | GET | 200 (109,316 B) | Bauman/Rovn Boys Varsity 5000 m: **121 rows**; grades SR 31 / JR 41 / SO 29 / FR 17 / "8" 3; **121/121 rows carry `a.ani`**; teams incl. Eastview `ani=12638` | 23:34 |
| same, `_doc/2918044` | GET | 200 (114,177 B) | Toni St. Pierre men's 8K, 111 rows; `a.ani=23911947`, `a.y="SR"`, `a.t.ani=21404`, `r[].pr`, 4-way `irs[]` splits, `avk`/`avm` | 23:33 |
| same, `_doc/2152386…2152391` | GET ×6 | 200 (177 KB…1.4 MB… ) | **2025 MSHSL State XC, all six classes: 959 rows, grades SR 293 / JR 242 / SO 193 / FR 133 / "8" 68 / "7" 30; 242 Juniors, 242/242 with `ani`; 959 unique Athletic.net athlete ids** | 23:39 |
| same, `_doc/2799753` | GET | 200 (17,981 B) | 2026 state T&F Boys 100 m AAA prelims: `w="-2.1"` wind, `m="10.75Q"`, `im=10743`, `runm="Preliminaries"`, `a.y="12"`, `a.ani=24966653`, `a.cm=788` (bib) | 23:41 |
| `$web/event_list/_doc/60118` and `_doc/77477` | GET | 200 (1,301 B / 1,757 B) | `event_list` restates the event header **without** `r[]` — cheaper existence/count probe (`nu`) | 23:19 |
| `$web/{meet_list,meet,meets,team_list,team,athlete_list,athlete,meet_info,meet_data,meet_events,meet_event_list,teams_list,athletes_list,team_res_list,xc_list,meet_summary,relay_res_list,relay_list,relay_results,events,events_list}/_doc/{77477,502165}` | GET ×~25 | 404 (215 B each) | **No meet-level or relay-level index exists** under those names; `ind_res_list` and `event_list` are the only two found, both keyed by event id | 23:20–23:42 |
| `$web?restype=container&comp=list&maxresults=20` | GET | 404 `ResourceNotFound` | Blob container listing is disabled — no index dump | 23:17 |
| `https://livestatic.athletic.net/assets/sites/wayzata/config.json` | GET | 200 | Tenant config: `siteName "Wayzata Results"`, `siteUrl https://results.wayzatatiming.com`, `elasticSearchIndex "wayzata"`, contact `results@wayzataresults.com` | 23:18 |
| `https://livestatic.athletic.net/assets/sites/base-site/config.json` | GET | 200 (820 B) | `primaryBaseUrl https://live.athletic.net` | 23:19 |
| `https://live.athletic.net/meets/75375` | GET | 200 (50,184 B) | Confirms `live.athletic.net` is the same AthleticLIVE app (link target used by Fast Finish) | 23:24 |
| `https://gsetiming.com/results/cc_rslts/cc_rslts.asp?sport=cc` | GET | 200 (60,896 B) | **609 `<option>` meet entries (608 dated), 2005–2025**, by year: 6,6,7,9,10,9,7,7,10,16,24,36,41,48,52,50,56,57,57,58,42; ids are numeric | 23:20 |
| `.../cc_rslts.asp?sport=tf` | GET | **302 → `https://athletic.net`** | GSE Timing's track & field results now live on Athletic.net | 23:21 |
| `.../cc_rslts.asp?sport=Cross-Country` | POST `meets=1367` | 302 → `adv_rslts.asp?meet_id=1367&race_id=7784&sport=Cross-Country` | Meet id + race id are the stable keys | 23:21 |
| `.../adv_rslts.asp?meet_id=1367&race_id=7784&sport=Cross-Country` | GET | 200 (74,867 B) | Section 5A CC 2025: team scoring table + individual table; DataTables ajax URL `adv_source.asp?race_id=7784` | 23:22 |
| `.../adv_source.asp?race_id=7784` | GET (XHR header) | 200 (87,823 B, body is JSON) | JSON array columns `Pl, Tm, Adv, Bib, First, Last, Team, Gr, M/F, Time, %, Diff` — e.g. `"5100","Judah","Allen","LIT","11","M","15:49.6"` | 23:21 |
| `https://www.herostiming.com/` and `/2026-results.html` | GET | 200 (96,468 / 235,773 B) | Year pages 2012–2026; 2026 page carries **256 `live.herostiming.com/meets/<id>` links** | 23:36 |
| `https://live.herostiming.com/meets/61470` | GET | 200 (50,184 B) | AthleticLIVE tenant (title + assets) | 23:36 |
| `https://fastfinishresults.com/`, `/schedule-results/`, `/results-archive/` | GET | 200 (129,630 / 213,810 / 120,729 B) | WordPress; 2026 page links `live.fastfinishresults.com/meets/<id>` **and** `live.athletic.net/meets/75375|75621|75656`; archive years 2023/2024/2025 | 23:37 |
| `https://live.fastfinishresults.com/meets/59668` | GET | 200 (50,184 B) | AthleticLIVE tenant | 23:37 |
| `https://www.mtecresults.com/documentation/api` | GET | 200 (18,968 B) — page title "Login - MTEC Results" | API exists but is **authenticated** | 23:38 |
| `https://www.mtecresults.com/event/show/6084/2026_Minnesota_Lutheran_Grade_School_State_XC_Meet` | GET | 200 (28,816 B) | Event URL pattern `/event/show/<id>/<slug>`; leaderboard at `/event/leaderboard/<id>/<slug>`; also `/documentation/exports` | 23:38 |
| `https://www.pickleevents.com/results/` | GET | 200 (37,426 B) | Year index **2001–2026**; 2026 table = 98 rows → 90 `mtecresults.com/race/leaderboard/<id>`, 36 RunSignup, **1 `athletic.net/CrossCountry/meet/260161`** | 23:41 |
| `https://mn.milesplit.com/` and `/results` | GET | 200 (103,019 / 92,997 B) | Month-grouped meet index for 2026 + 2025 with MileSplit meet ids (`/meets/779650-toni-st-pierre-invitational-2026/results`) | 23:19 |
| `https://mn.milesplit.com/meets/779650-toni-st-pierre-invitational-2026/results` | browser render | 200 | Raw `<pre>` results block with header `Athlete / Yr / Team / Mark / H#` (`Ingrid Norquist SR Macalester 21:58.7`) | 23:35 |
| `https://mn.milesplit.com/meets/778009-ada-borup-west-booster-club-invite-2026/results` | browser render | 200 | HS meet page exposes Gender/Level/Division/Event filters and **"MileSplit PRO … To get the full depth of our meet coverage, become PRO!"** → full results depth is subscription-gated | 23:36 |
| `https://api30.milesplit.com/{v3,v1}/meets/779650{,/results,/events}` | GET | 404 ×4 (`No route found for "GET …"`) | The API host is live (Symfony) but the route set is not anonymously discoverable; report 27 owns this | 23:24 |
| `https://www.mshsl.org/tournaments/state-tournament-archives/state-tournament-archive-boys-cross-country-2025` | GET | 200 (190,449 B) | Links 3 class result PDFs + champions/participants PDFs; archive pages also exist for 2024, 2023, … 2017 | 23:18 |
| `.../state-tournament-archive-boys-track-field-2026` | GET | 200 (236,456 B) | 2026 state T&F: `2026-track-state-meet-class-a-final-results.pdf`, `2026-state-track-class-aa-finals.pdf`, `2026-track-and-field-state-meet-class-aaa-finals.pdf` | 23:35 |
| `https://www.mshsl.org/sites/default/files/2025-11/2025-class-aaa-boys-cross-country-results.pdf` | GET | 200 (1,395,683 B, PDF 1.4, 2 pages) | **No text layer**: `pdffonts` empty, 0 `BT`/`Tj` operators in decompressed streams, `pdftotext` → **2 bytes**; content is raster (8 image XObjects, Nitro PDF Pro 14) → OCR required | 23:18 |
| `https://www.mshsl.org/sites/default/files/2025-11/results.pdf` | GET | 200 (1,408,206 B) | Class A XC 2025 — same: `pdftotext` → 2 bytes | 23:19 |
| `.../2024-11/2024-class-aaa-boys-cross-country-state-results.pdf` | GET | 200 (207,562 B) | **Text-based** (Helvetica, 30,065 B extracted); header "Timing: Wayzata Results, LLC"; `INDIVIDUAL RESULTS` with columns `Athlete … YR … # … Team … Score … Time … 1 Mile … 2 Mile` (`1 Robert Mechura SR 706 Roseville Area (1) 1 15:03.7`) | 23:20 |
| `.../2026-06/2026-track-and-field-state-meet-class-aaa-finals.pdf` | GET | 200 (158,656 B, 14 pages) | Hy-Tek `MEET MANAGER 7:23 PM 6/6/2026`, "Wayzata Results, LLC - Contractor License"; text-extractable (101,322 B); per-event blocks with `Name Yr School Finals` (`1 #788 Jackson Ziebarth 12 Cambridge-Isanti 10.37C 12`) | 23:36 |
| `.../2026-state-track-class-aa-finals.pdf`, `.../2026-track-state-meet-class-a-final-results.pdf` | GET | 200 (155,390 / 152,099 B) | Text-extractable (93,215 / 78,593 B), same `Yr`-bearing Hy-Tek layout | 23:36 |
| `https://www.mshsl.org/schools` and `/schools/ada-borup-west-high-school` | GET | 200 (221,979 / 175,910 B) | School directory `/schools/<slug>`, pager `?page=0..8` (~50/page); school page renders an **Activities Director** block; coach extraction is report 09's slice | 23:42 |
| `https://www.mshsl.org/tournaments/competitive-sections/list?field_activity_target_id=124&field_year_target_id=232` | GET | 200 (171,950 B) | Page loads but contains no `Section NX` text for that activity/year pair — **not confirmed** as an XC section list | 23:22 |
| `https://www.athletic.net/` | GET (default curl UA / browser UA) | **403** (5,343 B) / **200** (28,026 B) | Access is UA-conditional, not a blanket Cloudflare block; recorded without scraping content | 23:23 |
| `https://athletic.net` | GET (via GSE 302, browser UA) | 200 (28,026 B) | Apex and `www` serve the same marketing root | 23:21 |
| `https://results.wayzatatiming.com/meets/77477` | browser (all XHR) | — | Data source identified as Firebase RTDB namespace `trackmeet-io` (`s-gke-usc1-nssi3-33.firebaseio.com/.lp?dframe=t&id=…&ns=trackmeet-io`); this is why meet pages contain no data at rest. Credential values intentionally not reproduced | 23:44 |
| `.../meets/77477/events/xc/2918044` | browser (all XHR) | — | The page's only data request is `fetch GET https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2918044` — the public JSON path | 23:33 |

Per-host request totals (sequential, ≥0.3 s spacing): `athleticlive.blob.core.windows.net` ≈50 · `www.wayzataresults.com` 22 ·
`www.mshsl.org` 13 · `www.raceberryjam.com` 12 · `gsetiming.com` 7 · `results.wayzatatiming.com` 3 (+6 browser renders) ·
`livestatic.athletic.net` 4 · `mn.milesplit.com` 5 (+3 browser renders) · `herostiming.com`/`live.herostiming.com` 3 ·
`fastfinishresults.com`/`live.fastfinishresults.com` 4 · `mtecresults.com` 3 · `pickleevents.com` 2 · `api30.milesplit.com` 4 ·
`athletic.net` 3. No 429s, no `Retry-After`, no access-control bypass attempted; all requests were unauthenticated GETs (the two
GSE POSTs are the site's own documented form).
