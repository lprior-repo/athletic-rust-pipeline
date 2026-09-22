# 27. Midwest MileSplit super-index

Status: complete — all 12 state subdomains probed live; enumeration recipes, identifier map,
per-state counts, and the Athletic.net corpus overlap are measured on this machine. `robots.txt` was
fetched and the compliant-vs-reachable boundary is now stated explicitly (the API and rankings tree
are disallowed; everything recommended is allowed). Four limits are recorded explicitly rather than
papered over: (a) no free state-level `Class of 2027` count exists, so the per-state totals are an
order-of-magnitude band from a 3-team sample per state, not a census; (b) `www.athletic.net` is
Cloudflare-403 from this machine, so the overlap is measured entirely on the read-only corpus
artifact, never on live Athletic.net; (c) the richest result model lives on the robots-disallowed
`/api/` path and is therefore reported as peer-captured evidence, not as a usable surface;
(d) meets-with-a-results-link vs meets-with-actual-data is not measured (one request per meet would
be needed), so result availability is treated as unknown rather than assumed.

Observed on: 2026-09-19
Platform build string seen on every page/asset: `20260917151218`

---

### Source

MileSplit (FloSports, Inc.), a 12-host state network on one shared platform. Site identity is
rebranded per state (Wisconsin renders as **WIRunners.com**, `meta name="application-name"`), but the
templates, API, paywall and identifiers are identical across subdomains — verified by the 12-state
sweep (all 12 `/teams?type=1` and all 36 `/roster` responses parsed with the same regexes).

| Host | Role | Evidence |
|---|---|---|
| `https://<st>.milesplit.com/` — `wi mn ia il mi in oh mo ks ne nd sd` | state site + same-origin JSON API at `/api/v1/…` | 200 on all 12 |
| `https://www.milesplit.com/` | national site; also serves cross-listed meets (`/meets/768819-…`) | 200 |
| `https://api30.milesplit.com` | advertised as `accounts.apiDomain` in the inline `_DF_.init({…})` block | page source |
| `https://api.prod.milesplit.com/int/v3/`, `https://api.stag.milesplit.com/int/v3/` | referenced by `drivefaze/api.js`; **not reachable** at the paths the frontend would build | 404 ×2 |
| `https://js.sp.milesplit.com/`, `https://css.sp.milesplit.com/`, `https://assets.sp.milesplit.com/` | static assets, public | 200 |

Per-state site metadata is embedded in every page as `_DF_.init({…site:{…}})`. Wisconsin example:
`siteId=51`, `regionName=Wisconsin`, `regionCode=USA-WI`, `sports="outdoor,indoor,cc,road"`,
`levels="ms,hs,college"`, `indoor="winter"`, `hsBoysCc="5000m"`, `hsGirlsCc="5000m"`.

### Coverage

- **States:** all 12 target states have a live, identically-structured subdomain. No target state is
  missing from the network.
- **Sports:** `outdoor`, `indoor` (labelled "Indoor"/"winter"), `cross-country`, `road` — taken from
  the embedded site config and from the rankings `season` selector
  (`cross-country | indoor-track-and-field | winter-track-and-field | outdoor-track-and-field`).
- **Levels:** rankings `level` selector = `middle-school-boys/girls`, `high-school-boys/girls`,
  `club-boys/girls`, `college-men/women`, `alumni-men/women`. Team list `type` selector =
  `1 High School, 2 College/University, 3 Middle School, 4 Elementary, 5 Junior/Community College,
  6 Club, 7 Company, 8 Professional Team, 9 Olympic/National Team, 10 Group, 11 Unattached`.
- **Historical depth:** rankings `year` selector runs `all, 2000 … 2026`; the results browser `year`
  selector runs `Any Year, 2006 … 2026`. Both observed on `wi.milesplit.com`, 2026-09-19.
- **School levels actually populated:** rosters mix grades. `column-grad-year` values observed across
  the sample span `2027 … 2034`, and Nebraska/Wisconsin rosters carry large `2030/2031` cohorts —
  i.e. some `type=1` team records include middle-school/youth athletes. Grade `0` appears too
  (unset grade). This must be filtered, not assumed.

### Enumeration

All recipes below are exact, verified request patterns. `{RSID}` = result-set id, `{TeamID}`,
`{MeetID}` = numeric ids.

**Schools / teams — yes, complete, 1 request per state.**

```
GET https://<st>.milesplit.com/teams?type=1
```
One alphabetised page, **no pagination** (`page=` never appears in the markup), `597` rows for WI.
Rows are `<tr><td><a href="https://<st>.milesplit.com/teams/<TeamID>-<slug>">Name</a></td>
<td>CITY, ST, USA</td></tr>`. Measured high-school (`type=1`) team records:

| state | teams | state | teams | state | teams | state | teams |
|---|---|---|---|---|---|---|---|
| OH | 977 | MI | 895 | IL | 849 | MO | 700 |
| WI | 597 | MN | 592 | IN | 533 | KS | 413 |
| IA | 411 | NE | 317 | SD | 196 | ND | 153 |

Total **6,633** high-school team records. WI has 597 records but only 595 distinct names
(`fall creek`, `independence` duplicated) — team records are not a 1:1 school key.

**Meets — yes, 50 per page, but the listing has a silent depth cap; partition by month.**

```
GET https://<st>.milesplit.com/results/<state-slug>-meet-results?year=<YYYY>&season=<cross-country|indoor|outdoor>&level=hs&month=<1-12>&page=<N>
```
`<state-slug>` observed as `wisconsin-meet-results`; `/results?year=…&season=…` redirects there. The
filter form exposes `season`, `level` (`youth|ms|hs|college|open|pro`), `month`, `year`, `league`.
Rows are `<li class="meet-row" data-meet-id="<MeetID>">` with `meet-row__day`, `meet-row__name`,
`meet-row__venue`, and (sometimes) a `meet-row__results` link.

- 50 rows/page. Page 5 is valid (WI outdoor 2026: May 18→May 14); **page 30 silently returns page 1**
  — out-of-range pages are clamped, not 404'd, so you cannot detect the end by status code.
- `month=<M>` pages deeper than the unpartitioned list: `month=5&page=6` still returned fresh rows
  (May 14→May 12). **Partition by month to enumerate a full season.**

**Athletes — yes, via team rosters. This is the only free Class-of-2027 enumerator.**

```
GET https://<st>.milesplit.com/teams/<TeamID>-<slug>/roster
```
One request returns the **entire** roster (no pagination; the largest observed was 436 athletes,
916 KB). Per athlete row:

| column | markup | values observed |
|---|---|---|
| Athlete | `<a href="https://<st>.milesplit.com/athletes/<AthleteID>-<slug>">Last, First</a>` | — |
| Gender | `div.column-gender` | `f`, `m` |
| **Class** | `div.column-grad-year` | `2027` … `2034`, and `0` when unset |
| Indoor / Outdoor / XC | three positional cells, each `icon-yes` or `icon-no` | booleans |

The `Class` filter in the page UI (`rosterFilterClass` = All Grades / 2027 / 2028 / 2029 / 2030) is
**client-side only** — the unfiltered HTML already contains every athlete, so one request per school
yields the whole grade cohort without paying for a filtered crawl.

**Class of 2027 — yes, directly.** `Class = 2027` in the roster is corroborated by the athlete
profile's own bio line. Verified on four states:

| athlete | roster class | profile bio |
|---|---|---|
| Dina Abdel-Megid (WI, `16319157`) | 2027 | `Middleton \| Class of 2027 \| Middleton, WI` |
| Cameron Black (OH, `13426167`) | 2027 | `Acad. For Urban Scholars \| Class of 2027 \| Youngstown, OH` |
| Charlie Alberts (SD, `11140853`) | 2027 | `Aberdeen Central High School \| Class of 2027 \| Aberdeen, SD` |
| Kingston Penn (WI, `16319414`) | JR on the leaders board | `Middleton \| Class of 2027 \| Middleton, WI` |

**Full rankings with a grade filter exist but are paywalled.**

```
GET https://<st>.milesplit.com/rankings/leaders/<level>/<season>?year=<YYYY>&accuracy=fat&grade=junior&conversion=n&page=1
```
The `grade` selector is `all | senior | junior | sophomore | freshman | 8th-grade | 7th-grade |
6th-grade | returners` — i.e. `grade=junior` **is** Grade 11 = Class of 2027. It is accepted and
returns HTTP 200, but the `leaders` view yields only **one row per event** (WI boys outdoor junior =
39 rows / 39 distinct events; WI boys XC junior = 3 rows: 5000m, 4000m, 3 Mile). It is a leaderboard,
not an enumeration.

Requesting the real per-event list redirects to the PRO route and is redacted:
`/rankings/events/high-school-boys/outdoor-track-and-field/100m?…` → **302** →
`/rankings/pro/high-school-boys/outdoor-track-and-field/100m?…`. Only **rank 1** is rendered; ranks
2+ are `<tr class="rankings-row lk" aria-hidden="true">` containing only
`<span class="redact" style="width:168px">` / `<span class="mask">` placeholders. **No data is
present in the HTML for the locked rows** — there is nothing to parse, and nothing here is bypassed.

**Athlete name search — yes (deterministic join, not enumeration).**
```
GET /search                                    (establishes session + searchToken)
GET https://<st>.milesplit.com/api/v1/search?category=athlete&q=<name>&searchToken=<token>&isState=1&subdomain=<st>&page=<N>
```
`searchToken` is session-bound: reusing a token from an earlier request without its cookie returns
`400 {"error":{"message":"Bad Request (400): Invalid search token"}}`. With a fresh cookie jar it
returns `{type, id, title, description, url}` where `description` is
`"<School> <GradYear> / <Team> / <City>, <ST>, USA"` — name + school + **grad year** in one call.

**Meet entry lists — yes, free, server-rendered, and richer than expected.**

```
GET https://<st>.milesplit.com/meets/<MeetID>-<slug>/entries
```
Robots-allowed (not in the `Disallow` list), HTTP 200, **1.96 MB, one request**, and crucially
**zero locked markers** (`Locked` count 0, `redact` count 0) — the entry list sits entirely on the
free side of the paywall. Measured on `ks.milesplit.com/meets/751194-kshsaa-state-championships-2026/entries`:
**4,595 `<tr>` rows, 216 event headings (108 boys / 108 girls), 2,599 unique `(athlete, team)` pairs.**
Per-event block structure:

```
<h>1A Boys 100 Meter Dash</h> <span>18 entries</span>
Athlete | Seed | Team          <- header row
Wright, Karsyn | 10.80 | Weskan High School
Carson, Malik  | 10.88 | Chase High School
```

Field limits, measured: the athlete cell is **plain text — no `<a href="/athletes/<id>">`** and there
is **no grade column** (`Class of`, `grad-year` and `>Yr<` all occur 0 times on the page). So an
entry list gives `name + seed mark + team + event + division` for a whole meet, but **not** an
AthleteID and **not** Class-of-2027 evidence. Its value is therefore: (a) a meet-sized
roster-equivalent for schools whose team page is empty, and (b) **pre-meet** athlete presence — entry
lists exist before results do (confirmed on a 2026 meet whose result sets still render "No Results
Yet"), which is earlier discovery than any results surface.

**Availability caveat — the route always answers, the list is often empty.** The `/entries` route
returned HTTP 200 for every meet tested, but populated content is meet-specific:

| meet | state | entries |
|---|---|---|
| `751194` KSHSAA State Championships 2026 | KS | **2,599 unique `(athlete, team)` pairs**, 216 event headings, 1.96 MB |
| `749383` IHSA Boys Track & Field State Championships 2026 | IL | **0 rows**, 51 KB |
| `764934` IHSA Class 1A TRB Sectional #01 Oregon 2026 | IL | **0 rows**, 36 KB |

Bare-numeric `/meets/<MeetID>/entries` transparently redirects to the slugged form, so the route is
safe to hit blind. 1 of 3 tested meets was populated (and it was populated *heavily*), so entries are
a **high-value, low-coverage** surface: cheap to probe, occasionally worth a whole meet's participant
list, never to be assumed present.

**Timer directory — yes, and it is a second meet-enumeration axis.**

```
GET https://<st>.milesplit.com/timing                 -> timer orgs: /timing/<TimerID>/<slug>
GET https://<st>.milesplit.com/timing/<TimerID>/<slug> -> that timer's meets
```
`wi.milesplit.com/timing` returned HTTP 200 with **28 timer organisations** as
`/timing/<TimerID>/<slug>` links (e.g. `/timing/1013/xm-timing`, `/timing/111/white-river-sports-timing`,
`/timing/1192/complete-timing`). Verified on three states — **WI 28, MN 21, IL 23** timer orgs, each
at HTTP 200 with a ~50 KB page — and matching the path peer report 23 observed for Kansas
(`/timing/782/midwest-timing-and-results`) and peer report 08 used for per-timer schedules, so it is
network-wide. For meet discovery it is **complementary to the month-partitioned `/results` sweep**:
the timer axis finds meets that the results browser's depth cap or `level=hs` filter would drop,
and timer IDs are a stable per-state identifier worth carrying in the meet model.

**Athlete sitemap — yes, but it is a rolling recent-changes window, not a census.**

```
GET https://<st>.milesplit.com/sitemap.xml
```
HTTP 200, 918 KB, robots-allowed, **exactly 7,500 `<loc>` + `<lastmod>` pairs, all `/athletes/<id>-<slug>`**,
ordered newest-first. Measured windows:

| subdomain | urls | `lastmod` range | span |
|---|---|---|---|
| `wi.milesplit.com` | 7,500 | 2026-08-28 → 2026-09-19 | ~22 days |
| `in.milesplit.com` | 7,500 | 2026-08-21 → 2026-09-19 | ~29 days |

The 7,500 cap is exactly hit on both, so this is a **truncated tail**: it exposes only the most
recently touched athlete records (IDs in the 18.7 M range — newer than the 11–17 M IDs on the sampled
rosters). It cannot enumerate Class of 2027 across a state, but it is the cleanest **change-detection**
surface on the whole site and is the correct basis for a refresh loop (see *Incremental use*).

**State-wide athlete browse — not available.** `/athletes` is a landing page only: a search box plus
`v1/athletes/top-ranked` cards. There is no state-level, grade-filtered athlete index anywhere on the
free surface.

### Stable identifiers

| identifier | where it appears | form | notes |
|---|---|---|---|
| **AthleteID** | `…/athletes/<AthleteID>-<slug>` on rosters, rankings rows, results, search API | numeric string | stable across pages; verified same id for the same athlete via roster + rankings + header |
| **TeamID** | `…/teams/<TeamID>-<slug>`, `data-team-id` on team pages | numeric string | 6,633 HS records |
| **MeetID** | `…/meets/<MeetID>-<slug>`, `data-meet-id` on results rows | numeric string | meets may be cross-listed on another state host, e.g. an OH-hosted row links to `pa.milesplit.com/meets/772832-…` and `www.milesplit.com/meets/768819-…` |
| **Result-set ID** | `…/meets/<MeetID>-<slug>/results/<RSID>/raw` | numeric string | 7 digits observed (`1304175`, `1307671`, `1311155`); a meet can hold several |
| **VenueID** | `…/venues/30927/sun-prairie-east-high-school` | numeric string | |
| **SiteID** | `_DF_.init` → `site.id` (WI = `51`) | numeric string | per-state; `site_id` also appears in page analytics blobs |
| **Season** | query param + `data-season` attr on results rows | `outdoor \| indoor \| cross-country \| road` | athlete result rows carry `data-season="outdoor" data-level="hs" data-event="100 Meter Dash"` |

Caution on AthleteID: name search returns **duplicate person records**. `q=Kingston Penn` returned
two ids — `16319414` (`Middleton 2027 / Cardinal Track and Field Club / Middleton, WI, USA`) and
`17981238` (`Middleton 2027 / WI, USA`). Ids are stable; the person↔record mapping is 1:N and needs
school+grad-year disambiguation.

### Athletic.net leverage

**MileSplit exposes no Athletic.net links or ids.** Ranking rows, roster rows, meet pages, team pages
and athlete profiles link only inside the `milesplit.com` network. There is no `athletic.net` href
anywhere in the pages fetched. Therefore MileSplit **cannot** hand over a TeamID, MeetID, AthleteID
or URL on the Athletic.net side.

It can, however, **seed a deterministic Athletic.net lookup** from
`(normalized name, school, state, grad year)`, and it replaces the *cost shape* of discovery rather
than the lookup:

- **Requests avoided (discovery):** one MileSplit request per **school** replaces up to *r*
  Athletic.net profile requests, where *r* is that school's Class-of-2027 headcount. Measured
  per-team Class-of-2027 counts in this sample run 3–89 (median 18 across the 30 non-empty sampled
  teams). Whole-network cost for the 12 states is **6,633 roster requests**.
- **Marginal discovery:** against the read-only Athletic.net corpus
  (`Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx`, 142,705 US boys Grade 11), **189 of 287**
  sampled MileSplit Class-of-2027 boys flagged as outdoor participants (**65.9 %**, 95 % Wilson CI
  **60.2 %–71.1 %**) are present in the corpus by normalized `name + state` (or `name + school`).
  The complement — **34.1 %**, CI **28.9 %–39.8 %** — is MileSplit-only. Interpretation: roughly a
  third of roster-registered, outdoor-competing Class-of-2027 boys have **no** Grade-11 boys
  Athletic.net record. `[INFERENCE]` part of that complement will be athletes whose Athletic.net
  profile exists but whose event/school was outside ranking enumeration; assignment 4 owns that
  question.
- **Requests avoided (results):** every meet whose raw result set is public is a whole meet of
  per-athlete marks, grades, places and schools for **one request**, with no Athletic.net calls.
- **Requests avoided (grad-year verification):** the roster `Class` column is a free Class-of-2027
  signal, so Grade-11 confirmation does not need an Athletic.net profile round-trip.

Midwest Athletic.net corpus boys Grade 11 (from the corpus, primary-state attribution):
WI 3,961 · MN 3,845 · IA 2,643 · IL 6,136 · MI 6,041 · IN 2,710 · OH 6,390 · MO 3,661 · KS 2,059 ·
NE 1,993 · ND 767 · SD 939 = **46,145** of the 142,705 national rows.

### Athlete evidence

| field | free? | where / how |
|---|---|---|
| name | yes | roster, rankings, results, profile `<h1 id="athleteName">` |
| graduating class | **yes** | roster `div.column-grad-year`; profile `<span class="grad-year">Class of 2027</span>` |
| school | yes | roster page scope; profile `<span class="current-school"><a href="…/teams/<TeamID>-…">` |
| city/state | yes | profile `<span class="city-state">` (e.g. `Middleton, WI`); team list `CITY, ST, USA` |
| gender/category | yes | roster `div.column-gender` (`f`/`m`); rankings level (`high-school-boys/girls`) |
| TF/XC distinction | yes | roster Indoor/Outdoor/XC boolean columns; rankings `season` |
| indoor/outdoor | yes | roster booleans; results `data-season="outdoor"`, event rows `data-level="hs"` |
| performances | partial | raw meet results are free and complete; an athlete's own list is PRO-gated |
| PRs | partial | profile "Personal Records" table is free but truncated (WI example shows 3 outdoor PRs + "Show all PRs"); includes state/national rank + date |
| progression | **no** | `/athletes/<id>/progression` exists in the nav; the underlying series is PRO |
| meets | partial | meet name + date + place appear free in raw results; the athlete's full meet list is PRO |
| athlete profile URL | yes | `https://<st>.milesplit.com/athletes/<AthleteID>-<slug>`; verified 200 in WI, OH, SD |

Paywall boundary, measured: on the Kingston Penn profile, the free render includes name, school,
`Class of 2027`, city/state and a truncated PR block; then *"See all 48 of Kingston Penn's results —
Full performance history, included with MileSplit PRO"* and the result rows are replaced by
`<span class="mask" role="img" aria-label="Locked">` placeholders (55 locked markers on the page).

#### Candidate record shape (athletic identity/evidence only — schema, not a collection)

Only fields that the free MileSplit surface actually supplies are included; no coach contact, no
athlete personal contact. `ms_*` ids are MileSplit-side; Athletic.net ids are deliberately absent
because MileSplit never exposes them (join is by `name + school + state`, see *Athletic.net leverage*).

```json
{
  "state": "WI",
  "athlete_name_roster": "Abdel-Megid, Dina",       // roster display form (Last, First)
  "athlete_name_normalized": "Dina Abdel-Megid",    // for cross-source join
  "gender": "f",                                    // roster div.column-gender: f|m
  "grad_year": 2027,                                // roster div.column-grad-year
  "class_of_2027_evidence": {
    "value": "2027",
    "source": "MileSplit team roster 'Class' column",
    "url": "https://wi.milesplit.com/teams/14192-middleton/roster",
    "corroborated_by_profile": true                 // profile <span class="grad-year">Class of 2027</span>
  },
  "school": {"name": "Middleton", "city_state": "Middleton, WI"},
  "ms_team_id": "14192",
  "ms_athlete_id": "16319157",
  "ms_profile_url": "https://wi.milesplit.com/athletes/16319157-dina-abdel-megid",
  "season_participation": {"indoor": false, "outdoor": true, "xc": false},
  "events_prs_availability": {
    "prs_free_truncated": true,                     // "Personal Records" table, limited rows
    "prs_full": "PRO",                              // "Show all PRs" is subscription-gated
    "results_history": "PRO",                       // locked: <span class="mask" aria-label="Locked">
    "meet_results_free": true                       // via /meets/<MeetID>/results/<RSID>/raw
  },
  "observed_on": "2026-09-19"
}
```

Twelve real samples, one per state, each taken as the first `Class = 2027` row in that state's
sampled roster (roster order, not ranked). Profile URLs are the identity evidence; `Y`/`—` are the
roster's per-sport participation flags.

| state | athlete_name (roster form) | gender | grad_year | school | city/state | MileSplit athlete id | profile URL | indoor | outdoor | XC |
|---|---|---|---|---|---|---|---|---|---|---|
| WI | Aguilera, Julian | m | 2027 | Abbotsford | ABBOTSFORD, WI | `14399169` | `https://wi.milesplit.com/athletes/14399169-julian-aguilera` | — | — | — |
| MN | Anderson, Regan | f | 2027 | Glencoe-Silver Lake High School | Glencoe, MN | `11796978` | `https://mn.milesplit.com/athletes/11796978-regan-anderson` | Y | Y | — |
| IA | Bjork, Addie | f | 2027 | A-D-M High School | Adel, IA | `13856076` | `https://ia.milesplit.com/athletes/13856076-addie-bjork` | Y | Y | Y |
| IL | Benge, Amelia | f | 2027 | Abingdon (A.-Avon) | Abingdon, IL | `11062721` | `https://il.milesplit.com/athletes/11062721-amelia-benge` | Y | Y | Y |
| MI | Foster, Darrell | m | 2027 | Muskegon Heights | Muskegon Heights, MI | `16250380` | `https://mi.milesplit.com/athletes/16250380-darrell-foster` | — | Y | — |
| IN | Butler, James | m | 2027 | 21st Century Charter (Gary) | Gary, IN | `17697445` | `https://in.milesplit.com/athletes/17697445-james-butler` | Y | Y | — |
| OH | Black, Cameron | m | 2027 | Acad. For Urban Scholars | Youngstown, OH | `13426167` | `https://oh.milesplit.com/athletes/13426167-cameron-black` | Y | Y | Y |
| MO | Ackerman, Danica | f | 2027 | Adrian High School | Adrian, MO | `13058025` | `https://mo.milesplit.com/athletes/13058025-danica-ackerman` | Y | Y | Y |
| KS | Affolter, Kamdyn | m | 2027 | Abilene High School | Abilene, KS | `13732171` | `https://ks.milesplit.com/athletes/13732171-kamdyn-affolter` | — | Y | Y |
| NE | Barnhill, Emma | f | 2027 | Adams Central | Hastings, NE | `15249012` | `https://ne.milesplit.com/athletes/15249012-emma-barnhill` | — | Y | Y |
| ND | Abentroth, Hadley | f | 2027 | Hillsboro/Central Valley HS | Hillsboro, ND | `12031026` | `https://nd.milesplit.com/athletes/12031026-hadley-abentroth` | Y | Y | — |
| SD | Alberts, Charlie | m | 2027 | Aberdeen Central High School | Aberdeen, SD | `11140853` | `https://sd.milesplit.com/athletes/11140853-charlie-alberts` | Y | Y | Y |

Three of these twelve profile URLs (OH, SD and WI `16319157`) were fetched and confirmed to render
`Class of 2027` with the matching school; the remaining nine are roster-derived and
`[INFERENCE]`-only at the profile level.

### Recruiting information

**None.** MileSplit exposes no coach or athletic-director data on any surface fetched:

- `wi.milesplit.com/teams/14192-middleton` — 0 occurrences of `coach`.
- `wi.milesplit.com/teams/14192-middleton/roster` — 0 occurrences of `coach`.
- No school athletics website or team website field; the team page's outbound links are MileSplit
  pages, plus a MileSplit results contact (`results@milesplit.com`) on meet pages.
- Per-state editorial contacts exist only in the site footer (e.g. "MileSplit Ohio Editor:
  Mark Dwyer") — an editorial contact, **not** a school/sport coach and not usable as one.

MileSplit is therefore **not** a COACH-DIRECTORY candidate for this mission. Assignment 29 owns that.

### Result evidence

The free `raw` view is a fixed-width plain-text block inside a single `<pre>`. Header per event:

```
Boys Varsity 100 Meters Finals
============================================================================================
     Athlete                   Yr Team                                          Mark      H#
============================================================================================
   1 Kingston Penn             11 Middleton                                   10.41a
   2 Antonio Jackson           12 Sun Prairie West                            10.41a
  -- Larry Boyd                12 Sun Prairie West                               DNS
```
Relay events use `Team | Time | H#` with a leg list rendered as `1) A` (empty leg names).

| field | available | notes |
|---|---|---|
| ResultID | **no** | no per-result id in the raw view |
| AthleteID | **no** | raw view prints names only; ids exist on the formatted/rankings surface |
| MeetID | yes | from the URL you already have (`data-meet-id` in the listing) |
| Result-set ID | yes | in the URL (`/results/<RSID>/raw`) |
| EventID | **no** | event is a text header (`Boys Varsity 100 Meters Finals`); the `data-event` code (`100m`) exists on rankings rows, not on raw results |
| mark | yes | e.g. `10.41a`, `3:23.93a`, `15:12.72` |
| normalized mark inputs | partial | `a` suffix marks FAT (also selectable as `accuracy=fat` on rankings); conversion flag `conversion=n` on rankings |
| timing method | partial | FAT implied by the `a` suffix / the `accuracy` filter; no structured field |
| wind | **not in raw** | wind is rendered on rankings rows as `<span class="wind">(+0.8)</span>`; absent from this raw block. The API result model does carry it (`windReading`) — see the two-tier note below |
| implement/hurdle spec | **no** | none observed |
| heat/round | partial | `H#` column present; round is in the event title (`Finals` / `Preliminaries`) |
| place | yes | leading integer, `--` for DNS |
| date | yes | meet-level date on the meet page (`May 15, 2026`) |
| school represented | yes | team column, e.g. `Middleton` — team name only, not a TeamID |
| relay membership | **no** | relay sections list `Team … 1) A` with empty legs (72 leg lines, 0 named) |

Scale check on a second meet (WIAA `D1 Sectional 7 - Beaver Dam`, MeetID `766429`, RSID `1307671`):
579 lines, **232 athlete rows**, `Yr` counter `{12: 99, 11: 64, 10: 41, 9: 28}` — a single free
request yields 64 Grade-11 entries with marks.

#### Two-tier result model — and the tier that matters is robots-disallowed

There are **two** result models on MileSplit, and they are not equivalent:

| | compliant view — `/meets/<MeetID>/results/<RSID>/raw` | structured JSON — `/api/v1/meets/<MeetID>/performances` |
|---|---|---|
| robots.txt | **allowed** | **`Disallow: /api/`** |
| ResultID | no | yes (`id`, e.g. `192767162`) |
| AthleteID | no (names are plain text) | yes (`athleteId` + `profileUrl`) |
| TeamID | no (team name string only) | yes (`teamId`, `teamProfileUrl`, `divisionName`) |
| grad year | `Yr` column (grade, not cohort) | yes — `gradYear`, the cohort value directly |
| wind | no | yes, when present (`windReading`) |
| round / heat | round in event title, `H#` column | `round`, `roundName`, `heat` |
| normalized mark | no | yes (`units`: ms for time, inches×1000 for field) |
| field-event spec | no | no |

The upstream measurement behind the right-hand column is peer report 07 (Wisconsin MileSplit), which
captured 23 fields from `/api/v1/meets/{meetId}/performances` and measured on the **WIAA 2026 outdoor
state championships** (`meet 750759`): 2,454 rows (1,865 individual + 589 relay), **100 % `gradYear`
coverage on individual rows**, distribution 2026 → 895, **2027 → 582**, 2028 → 276, 2029 → 111, plus
one bad `2038`; **382 unique Class-of-2027 athletes**, 224 teams, divisions D1 995 / D2 696 / D3 698 /
Adaptive 65. Two further meets: New London Bulldog Invite 2025 XC (452 rows, 99.78 % coverage, 79
unique C2027) and MN Roy Griak Invitational (`meet 782310`, 2,605 rows, 95.70 % coverage, 588 unique
C2027).

**This is the report's sharpest trade-off.** The API is where `AthleteID`, `gradYear`, `teamId`,
`windReading` and normalized marks live — i.e. everything needed for a clean join and a real result
model — and it is exactly the path `robots.txt` disallows, while the compliant `/raw` view loses
ResultID, AthleteID, TeamID, wind and normalization. A production collector should therefore:
parse `/raw` for compliant result capture; recover `AthleteID`/`gradYear`/`TeamID` from the
robots-allowed **roster** page (`/teams/<id>/roster`, which carries AthleteID + `column-grad-year` +
gender); and treat the API as **production-negotiation territory, not a scrape target**. The `/raw`
view is also over 10× smaller per meet (579 lines for a sectional vs 2,454 structured rows for a
state meet), which partially offsets its thinner field set.

**Calendar trap (adopted from peer report 07; affects this report's overlap result).** Today
(2026-09-19) sits inside the 2026-27 school year, so **Class of 2027 = Grade 12** and Grade 11 =
Class of 2028. MileSplit's `grade=junior` ranking filter is school-year-relative and currently
selects Class of 2028. Everything in *this* report is stated as **grad year (cohort)**, never as
current grade: the roster `column-grad-year` column, the profile's `Class of YYYY` line, and the CSV's
`c2027` columns all mean the cohort graduating in 2027. The Athletic.net corpus used for the overlap
test is **US boys outdoor Grade 11, 2025-26** — i.e. the same cohort — so the comparison is
cohort-consistent and does not inherit the trap.

**Availability caveat, measured:** a `meet-row__results` link does **not** imply data. Ohio
`770652 Ohio Rockets Classic` → `/results/1311155/raw` returned **200 with "No Results Yet"** and no
`<pre>` block. Count of meets-with-actual-result-sets vs meets-with-result-links is **not measured**
(would need one request per meet) — treat it as unknown rather than assume near-100 %.

### Incremental use

- **New meets:** one daily/weekly request per state per season, partitioned by month so the depth cap
  cannot hide old pages:
  `…/results/<state-slug>-meet-results?year=<YYYY>&season=<season>&level=hs&month=<M>&page=<N>`.
  Diff `data-meet-id` sets week over week. 50 meets/page; a month-partitioned sweep of 12 states is
  ~12 × (12 months × pages) requests, but only the current month's pages need re-reading weekly.
- **Changed meets / new results:** a meet's `meets/<MeetID>-<slug>/results` page names its
  `<RSID>`(s). Cache `(MeetID → RSID)`; re-fetch `/results/<RSID>/raw` **only** when a new RSID
  appears. Neither response family carries a validator: the raw results and roster responses send
  only `date, content-type, server, x-frame-options, set-cookie, server-side-cache-ttl` — no `ETag`,
  no `Last-Modified` (captured 2026-09-19 23:24 CDT). So change detection must key on the RSID set
  and on the presence/absence of `<pre>`, not on HTTP validators.
- **Affected athletes:** recompute from the changed raw block only. Athlete identity is resolved by
  **name+school within the meet** — the roster or entry page that carries the `AthleteID`. The
  `/api/v1/search` shortcut recorded earlier in this report (name → `AthleteID` + grad year in one
  call) sits on a **robots-disallowed path** and must not be used in the collector; treat the roster
  page as the identifier oracle instead (1 request per school, cached, ~24 Class-of-2027 athletes
  each).
- **Rosters:** class year only changes once per year (a cohort rolls over each summer), so a weekly
  full re-fetch of 6,633 rosters is wasteful. Refresh a state's rosters once per season boundary, and
  use per-school refresh only for schools where meet results surfaced a Class-of-2027 name absent
  from the cached roster.
- **Cheapest daily change signal — `sitemap.xml` (1 request per state per day).** 7,500
  `(athlete URL, lastmod)` pairs, newest-first, covering roughly the last three to four weeks. Store
  the per-state URL→`lastmod` map and diff it: URLs whose `lastmod` advanced are the athletes the site
  actually rewrote, so a weekly refresh only needs to re-fetch *those* profiles rather than crawl.
  Two caveats, both measured: the file is **capped at exactly 7,500 entries** so it is a tail, not a
  census (an athlete untouched for a month will never appear), and its URLs carry no grade — it is a
  *change* oracle, never a Class-of-2027 oracle.
- **Pre-meet discovery — `/meets/<MeetID>/entries`.** Entry lists publish before results and are
  free and unlocked, so adding them to the sweep finds a meet's participants at the moment entries
  close rather than at the moment results post. 1 request per meet; no `AthleteID` and no grade, so
  pair it with the roster page when identity is required.
- **Avoid full historical re-fetch:** the whole design keys on `(MeetID, RSID)` and `(TeamID)`, both
  cheap to cache, so no weekly full-corpus crawl is required.

### Access characteristics

**Class: normal HTML, server-rendered, no auth (robots-allowed) — plus a robots-disallowed JSON API
and a subscription tier layered over the same content.** The operative classification for a
production collector is the intersection of *reachable* and *permitted*, which is **plain HTML only**:
the structured JSON is reachable but `/api/` is `Disallow`ed, and `/rankings` is `Disallow`ed.

- Public, no auth, **robots-allowed** (→ the collector surface): team list `/teams?type=1`, team
  pages, roster `/teams/<id>/roster`, athlete profile bio + truncated PRs, meet results
  `/meets/<id>/results/<RSID>/raw`, meet entry lists `/meets/<id>/entries`, results browser
  `/results/…`, timer directory `/timing`, `sitemap.xml`.
- Public, no auth, **but `robots.txt`-disallowed** (→ observation only): `/rankings/*` including the
  free `/rankings/leaders/…` grade-filtered leaderboard, and `/api/v1/*`
  (`athletes/top-ranked`, `search`, `meets/{id}/performances`).
- Subscription-restricted (**MileSplit PRO**): full per-event rankings (`/rankings/pro/…`), an
  athlete's full result history, "Show all PRs", team "Ranked Performances". Rows are replaced by
  empty `redact`/`mask` spans — the data is not merely hidden by CSS. Note this tier sits *inside* the
  already-disallowed `/rankings` subtree, so the paywall is not the binding constraint — robots is.
- API headers observed on `GET /api/v1/athletes/top-ranked`:
  `content-type: application/json`, `access-control-allow-origin: *`,
  `access-control-allow-credentials: true`,
  `access-control-allow-methods: POST, GET, OPTIONS, DELETE, PUT, PATCH`,
  `set-cookie: unique_id=…; domain=.milesplit.com`.
- **Rate limits: none advertised and none triggered.** No `X-RateLimit-*`, no `Retry-After`, no `429`
  in any response. Automated sweep: **52/52 requests HTTP 200**, mean 0.6 s latency, sequential with
  a ≥2 s gap per host. Manual probes: **66 further requests**, of which only the two deliberate
  404s (`api.prod.milesplit.com`), the two deliberate negative controls (302 on
  `/rankings/events/…`, 400 on a stale `searchToken`) and the one deliberate 403 were non-200.
- `server-side-cache-ttl` header present (e.g. `344`) — server-side caching, not a client contract.
- **`robots.txt`: fetched, verified identical on three subdomains** (`wi`, `oh`, `ne`) —
  the policy is network-wide, not per-state:

  ```
  User-agent: *
  Disallow: /rankings
  Disallow: /virtual-meets
  Disallow: /api/
  Disallow: /contact
  ```
  No `Crawl-delay` directive. Consequences for a production collector:
  **`/rankings/*` and `/api/` are off-limits**, which removes the paywalled rankings surface *and*
  the undocumented JSON endpoints (including the `/api/v1/search` name lookup) from any compliant
  pipeline. The `/api/` results in this report are therefore **research observations, not a
  sanctioned integration path**. What remains — and is explicitly *not* disallowed — is the whole
  enumerable surface this report recommends: `/teams`, `/teams/<id>/roster`, `/results`,
  `/meets/<id>/results/<RSID>/raw`, `/meets/<id>/entries`, `/timing`, `/athletes/<id>`, `sitemap.xml`.
  The roster/results/entries paths are server-rendered HTML, so the compliant path is also the one
  that needs no reverse-engineered API. (The `/rankings/leaders/…` free leaderboard is disallowed
  too, so the `grade=junior` leaderboard finding above is likewise observation-only.)

  *Correction to peer report 18 (Indiana):* it states "MileSplit's per-team rosters are client-rendered
  from the robots-disallowed `/api/`, so a name+school join would require either one request per
  MileSplit athlete profile or an `/api/` call." **This is contradicted by direct measurement:**
  `/teams/<TeamID>-<slug>/roster` is server-rendered static HTML — 36/36 roster requests across 12
  states parsed 3,650 athletes with AthleteID, gender and `Class` from the raw bytes, with no API
  call and no JS. Peer 18 appears to have inspected the team *landing* page (`/teams/<id>-<slug>`),
  which does load its "Ranked Performances" and schedule widgets via JS; the sibling `/roster` page
  does not. This matters because it moves athlete enumeration from "policy decision, not a technical
  one" back onto a **robots-allowed, one-request-per-school** path.

### Recommendation

**PRIMARY** — scoped to *Class-of-2027 athlete identity, grade evidence and school attribution* — with
a secondary **RESULT-SOURCE** role, and explicitly **not** a coach directory.

**First, the compliant surface is narrower than the reachable surface — plan against the compliant
one.** `robots.txt` disallows `/rankings` and `/api/`, which removes both the paywalled rankings tree
and the undocumented JSON endpoints (including the richest result model, and the name → `AthleteID`
search shortcut) from a legitimate collector. Nothing recommended below touches either path. The
practical cost is one extra join: the API's `AthleteID`/`gradYear`/`TeamID` arrive instead from the
robots-allowed roster and entry pages. Everything the pipeline needs for Class-of-2027 discovery is
on the allowed side.

Expected marginal coverage:

1. **Roster enumeration is the cheapest Class-of-2027 roster-discovery surface found so far.**
   One request per school yields the complete roster with an explicit `Class` column and per-sport
   participation flags — 6,633 requests for the whole 12-state Midwest, and the sample produced 716
   Class-of-2027 athletes from 30 schools (36 roster requests; 6 rosters came back empty) — ~24
   Class-of-2027 athletes per populated request.
2. **It reaches athletes Athletic.net rank-enumeration does not.** 34.1 % (CI 28.9–39.8 %) of
   sampled outdoor-competing Class-of-2027 boys are absent from the Athletic.net Grade-11 boys
   corpus.
3. **It is not a county-wide census in every state.** Coverage is sharply unequal and must be gated
   per state: Michigan rosters were effectively empty (2 of 3 sampled team pages returned 0 athletes;
   the one populated roster held 52 athletes of whom just 3 were Class of 2027), and Kansas/North
   Dakota/Indiana each had 1 of 3 sampled teams with an empty roster. Wisconsin, Ohio, Iowa,
   Missouri, Nebraska and South Dakota sampled cleanly.
4. **Results are free and grade-bearing** where a result set exists — 232 athlete rows including 64
   Grade-11 entries from one request on a WIAA sectional — but relay leg identity, wind, and
   per-result ids are missing, so it complements rather than replaces a full result model.
5. **Three cheap additive surfaces beyond rosters and results:** `/meets/<MeetID>/entries`
   (free, unlocked, pre-meet; 2,599 unique `(athlete, team)` pairs for the Kansas state meet in one
   request — but no `AthleteID` and no grade), `/timing` (a per-state timer directory, 28 orgs in WI,
   giving a second meet-enumeration axis), and `sitemap.xml` (7,500 `(athlete URL, lastmod)` pairs
   per state, the cleanest change-detection signal on the site, capped so it is a tail rather than a
   census).
6. **No coach/AD data at all** — do not route assignment 29 here.

Not chosen: `REJECT` (coverage is real and measurable), `DISCOVERY-ONLY` (it also carries grade
evidence and free results), `VALIDATION` (its independent-corpus role is real but secondary to the
discovery role), `ATHLETIC.NET-SEED` (it exposes no Athletic.net identifiers, so the seed is a
name+school+state+grad-year tuple rather than a URL).

**Per-state enumeration recipes and counts:** see `data/milesplit-coverage-matrix.csv`
(12 rows × 27 columns: team counts, sample sizes, Class-of-2027 counts by sex and sport,
order-of-magnitude bands, corpus overlap, and per-state free/paywall flags).

---

### Evidence appendix

Automated sweep timestamps are from `tools/scratch/milesplit_sweep.log` (all `2026-09-19`, CDT).
Manual probes ran `2026-09-19 23:12–23:54 CDT`. Requests: **52 automated + 66 manual = 118 total**
across 20 hosts; the busiest host is `wi.milesplit.com` at **44**, so no host reached the ~50
politeness budget; zero `429` and zero `Retry-After` observed anywhere.

| URL | method | HTTP | what it proved | timestamp |
|---|---|---|---|---|
| `https://wi.milesplit.com/` | GET | 200 | state site on MileSplit platform, branded WIRunners.com; nav exposes `/athletes /rankings/leaders /results /teams`; `_DF_.init` site config (siteId 51, sports, levels) | 2026-09-19 23:12 |
| `https://www.milesplit.com/` | GET | 200 | national host, same platform render | 2026-09-19 23:12 |
| `https://wi.milesplit.com/athletes` | GET | 200 | `/athletes` is a landing page only — no state-wide, grade-filtered athlete index | 2026-09-19 23:13 |
| `https://wi.milesplit.com/api/v1/athletes/top-ranked?page=1&limit=8&minRank=10` | GET | 200 | same-origin JSON API works anonymously; CORS `*`; payload has `athleteId, gender, season, state, teamName, topPerformance, profileUrl` | 2026-09-19 23:13 |
| `https://wi.milesplit.com/search` | GET | 200 | search form: `category=athlete\|meet\|team`, `q`, session-bound `searchToken` | 2026-09-19 23:13 |
| `https://wi.milesplit.com/api/v1/search?category=athlete&q=Kingston%20Penn&searchToken=<old>` | GET | 400 | `searchToken` is session-bound: `Invalid search token` | 2026-09-19 23:15 |
| `https://wi.milesplit.com/api/v1/search?category=athlete&q=Kingston%20Penn&searchToken=<fresh>&page=1` | GET | 200 | returns `id, title, description="<School> <GradYear> / <Team> / <City>, <ST>, USA"`; **two** ids returned for one athlete (duplicate records) | 2026-09-19 23:15 |
| `https://wi.milesplit.com/rankings/leaders/high-school-boys/outdoor-track-and-field` | GET | 200 | filter form exposes `level, season, event, year, accuracy, grade, conversion, page`; grade values `all/senior/junior/sophomore/freshman/8th-grade/7th-grade/6th-grade/returners` | 2026-09-19 23:13 |
| `.../rankings/leaders/high-school-boys/outdoor-track-and-field?year=2026&accuracy=fat&grade=junior&conversion=n&page=1` | GET | 200 | `grade=junior` accepted free; 39 rows = 1 per event (leaderboard, not enumeration); `canonical` link confirms param names | 2026-09-19 23:14 |
| `.../rankings/leaders/high-school-boys/cross-country?year=2026&grade=junior&page=1` | GET | 200 | XC junior leaders = 3 rows (5000m/4000m/3 Mile) | 2026-09-19 23:17 |
| `.../rankings/events/high-school-boys/outdoor-track-and-field/100m?year=2026&grade=junior` | GET | 302→200 | redirects to `/rankings/pro/…`; rank 1 rendered, ranks 2+ are empty `redact`/`mask` spans → **MileSplit PRO paywall, no data in HTML** | 2026-09-19 23:14 |
| `https://wi.milesplit.com/meets/763484/results` | GET | 200 | meet results shell links to `/results/1304175/formatted` and `/results/1304175/raw` | 2026-09-19 23:14 |
| `https://wi.milesplit.com/meets/763484-big-8-conference-2026/results/1304175/raw` | GET | 200 | **free fixed-width results** with `Athlete \| Yr \| Team \| Mark \| H#`; relay legs rendered as `1) A` with no names | 2026-09-19 23:15 |
| `https://wi.milesplit.com/meets/766429/results` → `…-d1-sectional-7-beaver-dam-2026/results/1307671/raw` | GET | 200 | 579 lines, 232 athlete rows, `Yr` counter `{12:99, 11:64, 10:41, 9:28}` | 2026-09-19 23:32 |
| `https://wi.milesplit.com/results` | GET | 200 | results browser with `season/level/month/year/league` filters; `level` values `youth\|ms\|hs\|college\|open\|pro`; `year` 2006–2026 | 2026-09-19 23:15 |
| `…/results/wisconsin-meet-results?year=2026&season=outdoor&page=1` | GET | 200 | 50 `li.meet-row` entries with `data-meet-id`, name, venue, date | 2026-09-19 23:16 |
| `…&page=2` | GET | 200 | further 50 rows (May 27→26); a `page=3` link exists | 2026-09-19 23:34 |
| `…&page=5` | GET | 200 | deep pages valid (May 18→14) | 2026-09-19 23:36 |
| `…&page=30` | GET | 200 | **silently returns page 1** — out-of-range pages clamp, so depth cannot be detected by status | 2026-09-19 23:36 |
| `…&year=2026&season=outdoor&level=hs&page=1` | GET | 200 | `level=hs` is the correct filter value; drops youth/MS meets, keeps cross-listed national meets | 2026-09-19 23:41 |
| `…&year=2026&season=outdoor&month=5&page=6` | GET | 200 | month partition pages deeper than the unpartitioned cap (May 14→12) → use `month=` to enumerate a season | 2026-09-19 23:40 |
| `https://wi.milesplit.com/teams?type=1` | GET | 200 | complete alphabetised HS team list, **no pagination**, 597 WI records with city/state; `type` values 1–11 | 2026-09-19 23:16 |
| `https://wi.milesplit.com/teams/14192-middleton` | GET | 200 | team page; "Ranked Performances" paywalled; **0 occurrences of `coach`** | 2026-09-19 23:17 |
| `https://wi.milesplit.com/teams/14192-middleton/roster` | GET | 200 | **free full roster**: 324 athletes, 324 profile links, per-row `gender`, `column-grad-year`, Indoor/Outdoor/XC flags; grad-year dist `{2028:113, 2027:101, 2029:90, 2030:17, 0:3}` | 2026-09-19 23:17 |
| `https://wi.milesplit.com/athletes/16319414-kingston-penn` | GET | 200 | free bio `Middleton \| Class of 2027 \| Middleton, WI`; truncated PR block; full results PRO-locked (55 `Locked` spans) | 2026-09-19 23:21 |
| `https://wi.milesplit.com/athletes/16319157-dina-abdel-megid` | GET | 200 | roster `Class=2027` confirmed by profile `Class of 2027`; school link to `/teams/14192-middleton` | 2026-09-19 23:37 |
| `https://js.sp.milesplit.com/drivefaze/api.js?build=20260917151218` | GET | 200 | default endpoint is same-origin `/api/`; `apiV3Production='https://api.prod.milesplit.com/int/v3/'` | 2026-09-19 23:13 |
| `https://api.prod.milesplit.com/int/v3/v1/athletes/top-ranked` | GET | 404 | the frontend-referenced v3 host is not reachable at the `v1/…` path (`Cannot GET /v3/v1/athletes/top-ranked`) | 2026-09-19 23:13 |
| `https://api.prod.milesplit.com/int/v3/athletes/top-ranked` | GET | 404 | second v3 shape also 404 — no anonymous v3 API found; probing stopped | 2026-09-19 23:19 |
| `https://oh.milesplit.com/results?year=2026&season=outdoor` | GET | 200 | 50 OH rows incl. cross-state/cross-host meets (`pa.milesplit.com`, `www.milesplit.com`) | 2026-09-19 23:37 |
| `https://oh.milesplit.com/meets/770652-ohio-rockets-classic-2026/results/1311155/raw` | GET | 200 | **"No Results Yet"** — a results link does not imply a result set | 2026-09-19 23:38 |
| `https://oh.milesplit.com/athletes/13426167-cameron-black` | GET | 200 | cross-state profile resolves: `Acad. For Urban Scholars \| Class of 2027 \| Youngstown, OH` | 2026-09-19 23:39 |
| `https://sd.milesplit.com/athletes/11140853-charlie-alberts` | GET | 200 | cross-state profile resolves: `Aberdeen Central High School \| Class of 2027 \| Aberdeen, SD` | 2026-09-19 23:39 |
| `https://wi.milesplit.com/meets/763484-big-8-conference-2026/results/1304175/raw` (headers only) | GET | 200 | no `ETag`/`Last-Modified`; only `date, content-type, server, x-frame-options, set-cookie, server-side-cache-ttl: 3006` | 2026-09-19 23:24 |
| `https://wi.milesplit.com/teams/14192-middleton/roster` (headers only) | GET | 200 | no `ETag`/`Last-Modified`; `server-side-cache-ttl` empty | 2026-09-19 23:24 |
| `https://<st>.milesplit.com/teams?type=1` ×12 (`wi mn ia il mi in oh mo ks ne nd sd`) | GET | 200 ×12 | high-school team counts 153–977 per state, 6,633 total (see Coverage table) | 23:16:37–23:18:27 |
| `https://<st>.milesplit.com/teams/<TeamID>-<slug>/roster` ×36 | GET | 200 ×36 | roster records: name + AthleteID + profile URL, gender, `Class` year, Indoor/Outdoor/XC flags; 3,650 athletes; 716 Class-of-2027; **6 of 36** rosters empty | 23:16:39–23:18:34 |
| `https://{wi,oh,ne}.milesplit.com/robots.txt` | GET | 200 ×3 | **identical network-wide policy** `Disallow: /rankings, /virtual-meets, /api/, /contact`; no `Crawl-delay`; `/teams`, `/results`, `/meets`, `/athletes`, `/timing`, `sitemap.xml` not disallowed | 2026-09-19 23:44 |
| `https://{wi,in}.milesplit.com/sitemap.xml` | GET | 200 ×2 | **exactly 7,500** `/athletes/<id>-<slug>` `<loc>`+`<lastmod>` pairs, newest-first; WI window 2026-08-28→09-19, IN 2026-08-21→09-19 → rolling tail, not a census | 2026-09-19 23:45 |
| `https://wi.milesplit.com/sitemap.xml` (re-fetch to file) | GET | 200 | 918,709 bytes, `text/xml`; all 7,500 locs are athlete profiles; IDs in the 18.7 M range | 2026-09-19 23:45 |
| `https://wi.milesplit.com/timing` | GET | 200 | **timer directory**: 28 timer orgs as `/timing/<TimerID>/<slug>` (e.g. `/timing/1013/xm-timing`); second meet-enumeration axis | 2026-09-19 23:46 |
| `https://ks.milesplit.com/meets/751194-kshsaa-state-championships-2026/entries` | GET | 200 | **entry list fully free**: 1.96 MB, 4,595 `<tr>`, 216 event headings (108 boys/108 girls), 2,599 unique `(athlete, team)` pairs; `Athlete \| Seed \| Team` with **no athlete links and no grade**; 0 `Locked`/`redact` markers | 2026-09-19 23:47 |
| `https://{mn,il}.milesplit.com/timing` | GET | 200 ×2 | timer directory is network-wide: MN **21** timer orgs, IL **23** (WI 28) | 2026-09-19 23:52 |
| `https://il.milesplit.com/results/illinois-meet-results?year=2026&season=outdoor&level=hs&page=1` | GET | 200 | 50 IL meets with `data-meet-id` + name, incl. `749383 IHSA Boys State Championships` | 2026-09-19 23:53 |
| `https://il.milesplit.com/meets/749383/entries` | GET | 200→ | redirects to `/meets/749383-ihsa-boys-track-and-field-state-championships-2026/entries`; **0 rows** — route answers, list empty | 2026-09-19 23:53 |
| `https://il.milesplit.com/meets/764934/entries` | GET | 200→ | redirects to slugged form; **0 rows** — confirms entries are meet-specific, not state-wide absent | 2026-09-19 23:54 |

Peer reports consumed read-only, findings incorporated above: `07-wisconsin-milesplit.md` (API
`performances` field model + WIAA state-meet measurements + the Class-of-2027/Grade-11 calendar trap),
`08-wisconsin-timing-providers.md` (MileSplit per-timer schedules; PrimeTime `exEntryLink` pointing at
MileSplit meet pages in 2025), `18-indiana-directathletics-milesplit.md` (sitemap as a recency proxy —
its roster claim is contradicted and corrected above), `23-kansas-kshsaa-directathletics.md`
(`/timing/782/…` and the KSHSAA entries page). Other peer reports were surveyed for MileSplit content;
01/02/05/06/13/14/19/21/22/24 contain ≤6 incidental mentions each and added nothing not already here.
