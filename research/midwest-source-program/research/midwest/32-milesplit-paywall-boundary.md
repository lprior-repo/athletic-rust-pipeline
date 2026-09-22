# 32. MileSplit free/locked boundary (open question C5)

Status: complete — measured live on 14 athlete profiles (10 state hosts), 16 team rosters (5 states),
6 raw result sets (4 states), 8 meet pages (incl. 3 national meets), 3 result indexes, 1 client results
script, the team-landing and progression routes,
and the two inline-JS control blocks that decide truncation; plus a headless-Chromium render of the
profile and progression pages. Everything below is first-hand unless marked `[INFERENCE]` or `[27]`
(inherited from report 27, not re-measured here).
Observed on: 2026-09-20 (14:05–14:14 UTC).

**One-paragraph answer.** MileSplit's paywall does **not** truncate the athlete result list to a free
"first N" — for every anonymous profile measured it server-renders **one row per result the athlete
has** (3 → 88 rows in this sample), but every cell of every row is an empty placeholder: the mark,
place, round, meet/location and date are simply **not in the HTML**. What is free on a profile is the
identity header (name / school / Class of YYYY / city-state), the PR table (event + mark + date +
**national** rank; **state** rank locked), and the results *skeleton* (season headings, event headings,
per-event row counts). Free surfaces elsewhere on the same site are unusually generous: `/teams/<id>/roster`
is 100 % unlocked (Class year, gender, season flags, athlete IDs, 1 request/team) and
`/meets/<id>/results/<RSID>/raw` is 100 % unlocked (full marks, places, heats, wind, and the `Yr`
grade column when the timing file carries it). The locked surfaces are: the per-athlete result payload,
the progression tab, the team "Ranked Performances" widget, and the rankings tree (the last is also
robots-disallowed). Collector rule in one line: **read results and grades from meets and rosters, never
from athlete profiles.**

---

### Source

| Item | Value |
|---|---|
| Provider | MileSplit / FloSports; state subdomains `<st>.milesplit.com` (`wi`, `mn`, `il`, `mi`, `oh`, `ia`, `mo`, `ks`, `ne`, `sd`, `nd`, …) |
| Paywall product | **MileSplit PRO** (subscription); CTAs link to `/join?ref=…`; login at `/login` |
| Surfaces examined | `/athletes/<id>[-slug]`, `/athletes/<id>/progression`, `/teams/<id>/roster`, `/teams/<id>` (landing), `/meets/<id>/results`, `/meets/<id>/results/<RSID>/{raw,formatted}`, `/results?…` index, `/teams` index |
| Inline paywall machinery | `css.sp.milesplit.com/drivefaze/pro/paywall.css`, `js.sp.milesplit.com/drivefaze/pro/paywall.js`, analytics flag `paywall_present:1`, classes `results-funnel--locked`, `funnel-masked`, `paywall-overlay-container paywall-active` |
| Evidence bundle | `research/midwest/evidence/gaps/32/` (raw HTML captures, `fetch-log.tsv`, `parsed-summary.json`); scripts in `tools/32-scratch/` |

### Coverage

- **Network-wide, not per-state.** The identical anonymous profile template (same CSS/JS build
  `20260920074803`, same `funnel-masked` + `results-funnel--locked` markup, same 5-mask-cell row shape)
  rendered on **10 different state hosts** in this run: WI, MN, IL, MI, OH, IA, MO, KS, NE, SD
  (plus WI/MN second samples). Report 27 `[27]` measured the same templates across all 12 Midwest
  subdomains. No state-specific paywall variation is visible in either sample.
- **Sports/seasons covered by the measurement:** outdoor track (WI/MN/IL/MI/OH/MO/KS/SD/NE profiles),
  indoor track (MN/IL/IA/SD rows present), cross-country (IA/SD/KS/MO rows present). The lock applies
  to all three seasons identically — e.g. Addie Bjork (IA, XC-heavy): 39/39 rows masked;
  Charlie Alberts (SD, XC-heavy): 32/32 masked.
- **Levels:** HS (`data-level="hs"`), plus MS rows inside the same profile (`data-level="ms"`) — both locked.
- **Historical depth of the locked block:** free season headings run back to 2021–2022 on the sampled
  profiles (Dina: 2025–2026; Charlie: 2021–2025); every historical row is masked the same way.

### Enumeration

What the boundary means for enumeration (all recipes executed live 2026-09-20):

| Surface | Status | Unlocked content | Cost |
|---|---|---|---|
| `/teams` (state index) | **free** | full HS team list, single page (WI 597, MN 592, IL 849, MI 895, OH 977 records — all 200) | 1 req/state |
| `/teams/<id>/roster` | **free, 100 %** | every rostered athlete: name, `AthleteID`, profile URL, gender, `column-grad-year`, indoor/outdoor/XC flags; client-side Class/gender/season filters | 1 req/team |
| `/results?year=&season=&level=hs` | **free** | date-ordered meet index with `data-meet-id` + results link | 1 req/page (50 meets) |
| `/meets/<id>/results` | **free** | `meetResultFiles[]` incl. per-file `isMeetPro`, `meetResultParams` (season/dates/teamScores) | 1 req/meet |
| `/meets/<id>/results/<RSID>/raw` | **free, 100 %** | whole result set as static fixed-width text | 1 req/result set |
| `/meets/<id>/results/<RSID>/formatted` | free HTML, **data path gated** | page shell only; rows arrive from the robots-disallowed `/api/` | 1 req (useless alone) |
| `/athletes/<id>` | partial | bio + PR table + result *skeleton* (no payload) | 1 req/athlete |
| `/athletes/<id>/progression` | **locked** | PRO upsell only | 1 req/athlete |
| `/rankings/events/…` lists | **locked** | rank 1 only; ranks 2+ empty `redact`/`mask` spans `[27]` | 1 req/page |
| `/rankings/pro/…` | **locked** | — | — |

**The enumeration consequence:** Class-of-2027 enumeration stays exactly where report 27 `[27]` put it —
team rosters — and this run re-verifies that the roster surface carries **zero** paywall machinery
(0 `mask`, 0 `aria-label="Locked"`, 0 `paywall_present` across 16 rosters in 5 states; the only
"paywall" byte on a roster page is the globally-included `paywall.css` `<link>`).

### Stable identifiers

All of these are on **free** surfaces (verified in this run):

| Entity | Identifier | Where free |
|---|---|---|
| Athlete | numeric `AthleteID` (`/athletes/<id>-<slug>`) | roster row `<a href>`, raw page has **no** ids, profile URL |
| Team | numeric `TeamID` (`/teams/<id>-<slug>`) | team index, roster page, meet page |
| Meet | numeric `MeetID` (`/meets/<id>-<slug>`) | results index (`data-meet-id`), meet page |
| Result set | numeric `RSID` (`/results/<RSID>/raw`) | meet page + `meetResultFiles[]` |
| Grade (cohort) | profile `<span class="grad-year">Class of 2027</span>`; roster `column-grad-year`; raw `Yr` column (letter grades `Sr/Fr` on some timer feeds) | profile + roster **free**; raw free when present |
| Season participation | roster `data-season-id="1|2|3"` (Indoor/Outdoor/XC) `icon-yes/no` | roster |
| Result-file PRO flag | `meetResultFiles[].isMeetPro` (`0|1`) | meet page |
| Event (profile skeleton) | `data-event="400 Meter Dash"` on each masked row | profile |

Inside the locked block there are **no identifiers to extract** — the mask spans carry only
`class`, `role="img"` and `aria-label`; no `data-*`, no hidden text (see *Athlete evidence*).

### Athletic.net leverage

The paywall sits on exactly the surface that would otherwise substitute for Athletic.net's
per-athlete history — so the boundary changes *which* MileSplit surface can carry the acquisition load:

- **Substitutable (free) work:** Class-of-2027 identity/grade (rosters, 1 req/school), whole-meet
  marks+grade+place+wind (raw, 1 req/result set), meet discovery (results index). Unchanged from
  report 27's accounting; this run adds no new AN-avoidance and removes none.
- **Not substitutable:** an athlete's *own* cross-meet history, PR progression, and state rank are
  PRO-only. If the pipeline wants a per-athlete performance list without Athletic.net, it must rebuild
  it by unioning meet raw files — which requires knowing which meets the athlete was in. MileSplit's
  free surfaces give that only indirectly (name+school appears in any raw file you already chose to fetch).
- **Cheap AN-side compensation (no AN requests needed):** the profile's free skeleton gives a
  **per-event result count and season boundaries** for the athlete, and the free PR table gives current
  PR marks + dates + national rank. That is a free *validation* signal for AN-sourced history
  (count/PR cross-check) without a single AN call.
- **Estimate:** for the WIAA 2026 state meet alone (report 07 evidence) 1 raw request carries 2,454
  API-equivalent rows / 382 unique Class-of-2027 athletes; where a meet exists on MileSplit, AN profile
  round-trips are avoidable in full. Where it does not, the profile page cannot fill the gap at any free cost.

### Athlete evidence

**Measured boundary per field (14 profiles, 10 state hosts).**

| Field | Free? | Evidence |
|---|---|---|
| Name | **yes** | `<h1 id="athleteName">` |
| School (+ team link) | **yes** | `<span class="current-school"><a href="/teams/14192-middleton">` |
| Class of YYYY | **yes** | `<span class="grad-year">Class of 2027</span>` on all 14 |
| City/state | **yes** | `<span class="city-state">Middleton, WI</span>` |
| Season headings | **yes** | `<div class="season" data-season="outdoor|indoor|cc" data-level="hs|ms"><h4>2026 - Outdoor</h4>` |
| Event headings | **yes** | `<p class="event-heading">400 Meter Dash</p>` |
| **Per-result row count** (the site's own total) | **yes** | the CTA line and the DOM agree: `See all N of <name>'s results` with N = DOM rows in **7/7** profiles where that CTA variant rendered: 7, 36, 39, 44, 45, 48, 88 |
| Row-level event/season tags | **yes** | `data-season`, `data-level`, `data-event` on each masked row |
| **Result payload** (mark, place, round, meet/location, date) | **no** | every row = 5 `<span class="mask" role="img" aria-label="Locked…">` cells, all empty; 0 rows without mask in 14/14 profiles |
| PR table: event, mark, date | **yes** | `#prTable` rows; full set is in the HTML (incl. `style="display:none"` per-season rows that contain older/extra PRs — e.g. Charlie Alberts: 5 visible unique PRs, 12 more unique PRs only in hidden rows) |
| PR table: **National** rank | **yes** | 4th cell: `#181` (Kingston 100m), `#16,238` (Amelia 60m); present exactly when `data-has-rank="1"` (count matched 14/14) |
| PR table: **State** rank | **no** | 3rd cell `<span class="pr-rank--locked" title="Subscribe to see State rank">` — locked on **100 %** of PR rows in 14/14 profiles (6/6 … 28/28) |
| "Show all PRs" expansion | **yes (client-side)** | verified in browser: toggling reveals in-DOM rows with **no** network request (no new `/athletes/*` or `/api/` fetch; DOM row count unchanged) |
| Results-section CTA text | n/a | two A/B variants render, both upsells: "See all N of <name>'s results — Full performance history, included with MileSplit PRO" or "To see all performances, subscribe to … Join Now" (`mile_pro_cta_contextual` cointoss) |
| Progression tab | **no** | `/athletes/<id>/progression` = PRO upsell page: headline "See how <name> is progressing", one **static example** `<svg class="prog-chart">` (the same "4:45 / 4:24 PR" illustration on every athlete), and "Join MileSplit PRO". Browser render: 0 data elements, 0 `<canvas>`; no request to any athlete data path |

**Exact lock mechanic (verbatim, one result row, Dina Abdel-Megid / WI):**

```html
<div class="record row " data-season="outdoor" data-level="hs" data-event="400 Meter Dash">
  <div class="seed col-4 col-md-2 pl-0"><span class="">
      <span class="mask" role="img" aria-label="Locked, subscribe to view"></span></span></div>
  <div class="position d-none d-md-block col-md-1"><span class="mask mask--sm" role="img" aria-label="Locked"></span></div>
  <div class="round d-none d-md-block col-md-2"><span class="mask" role="img" aria-label="Locked"></span></div>
  <div class="location col-7 col-md-4"><span class="mask mask--wide" role="img" aria-label="Locked"></span></div>
  <div class="date d-none d-md-block col-md-3 text-right"><span class="mask" role="img" aria-label="Locked"></span></div>
</div>
```

- 5 mask cells per row, **identical shape in 14/14 profiles**: `seed` (the mark) uses
  `aria-label="Locked, subscribe to view"`; `position`, `round`, `location` (meet), `date` use
  `aria-label="Locked"`. Total masks = 5 × rows (Dina 55, Kingston 240, Pongonis 440, Danica 235).
- **Correction to the earlier C5 note ("44 locked rows on Dina"):** 44 is the count of the four
  non-seed mask *cells* on her page (4 × 11); the page renders **11** result rows, and 55 mask spans total.
- **Truncation mechanic (inline JS, verbatim):**
  ```js
  // Record limit Split Test
  var isLoggedIn = '', isPremium = '', isFunnel = true;
  // Funnel users see ALL rows (masked) so the true performance count is visible;
  // only the legacy non-funnel preview is capped.
  if ((!isLoggedIn || !isPremium) && !isFunnel) { window.truncatePerformances(); }
  ```
  `window.truncatePerformances` = `performances.slice(8).remove()` per event. On every anonymous fetch
  in this run `isFunnel = true`, so the cap never fired; the DOM row count equals the site's own count
  line on all 7 profiles that print it (max observed 88). `[INFERENCE]` an athlete with >100 results was
  not sampled; the code comment is the authority that the funnel path renders all rows, and no evidence
  contradicts it.
- **Negative control (proves the lock is real, not CSS):** Kingston Penn's actual state-meet marks
  (`10.75`, `10.90`, `22.57` for 100m/200m; `1:26.47` relay) are all present in the free
  `/raw` meet file but **none** occur anywhere in his profile HTML. There is nothing to parse behind
  the masks and nothing hidden by CSS.
- Rendering in headless Chromium reproduced the server counts exactly (11 `.record`, 55 `.mask`,
  first row `innerText === ""`), i.e. client JS neither adds nor reveals payload.

### Recruiting information

Not applicable to this question and unchanged by it: none of the paywall surfaces (profile, roster,
meet results, progression) exposes coach/AD fields — the 16 roster captures in this run contain 0
occurrences of `coach` (case-sensitive), matching `[27]`, and the only contact-like string on meet
pages remains MileSplit's own `results@milesplit.com`.
No athlete personal contact data is exposed or collected.

### Result evidence

**Free `/raw` files are the result workhorse — 6/6 fully unlocked** (WI ×3, MN, IL ×2, OH):

| Result set | Meet | Status | Non-empty lines | Grade column | Locks |
|---|---|---|---|---|---|
| 1308252 | WI 2026 WIAA Outdoor Championships | 200 | 3,755 (2,523 place rows) | `Yr` 9–12 numeric | 0 masks |
| 1304175 | WI Big 8 Conference 2026 | 200 | 1,026 | `Yr` {9:94,10:167,11:219,12:206} by regex | 0 masks |
| 1308498 | MN Mini-Apple Night of Miles 2026 | 200 | 305 | `Yr` incl. `Sr`/`Fr`/`-` | 0 masks |
| 1307551 | IL IHSA Boys State Championships 2026 | 200 | 2,792 (2,443 place rows) | **no `Yr` column** (tab-delimited timer format; `Place Name Team Mark Heat Wind`) | 0 masks |
| 1310168 | IL AAU Region 13 Qualifier 2026 | 200 | 5,500 | `Yr` numeric (ages 2–12 at this youth meet) | 0 masks |
| 1309698 | OH 419 Flyers Invitational 2026 | 200 | 1,489 | `Yr` present (often `-` at this youth meet) | 0 masks |

- Free per row, when present in the timer's text: **place, athlete name, `Yr` (grade), team, mark,
  heat (`H#`), wind**, and `--`/`SCR`/`DNS` status. Relay rows print team + time, with leg detail only
  in formats that include it (IHSA state file lists the four leg names; the fixed-width MeetPro-style
  format prints a bare `1) A`).
- **Grade availability is a property of the timing file, not of the paywall**: 5/6 sampled files carry
  `Yr`; the IHSA state file does not. Collectors must join school+name to the roster for grade when `Yr`
  is absent.
- **The `/formatted` view is not a collector surface:** it returns a JS shell (0 `<tr>` in the server
  HTML) and loads `loadResultsNew.js`, whose data path is
  `_DF_.API.get('v1/meets/<id>/performances', {isMeetPro, resultsId, fields: 'id,meetId,…,gradYear,…,windReading,…'})`
  — i.e. the robots-disallowed `/api/` endpoint. Parse `/raw`.
- **PRO-flagged result files:** the page contract carries per-file `isMeetPro` (client sends
  `isMeetPro:1` + `excludeMeetPro` logic for mixed meets), but across **18 result files on the 7 meet
  pages that carried files** (AAU Junior Olympic Games 2026 ×5, adidas Track Nationals 2026 ×8, five
  Midwest state/regional meets ×1) **every flag was `0`** — no PRO-only result file was observed in the
  Midwest HS sample.
  `[INFERENCE]` PRO files exist mainly for very large national meets; treat the flag as a
  fetch-decision signal on the meet page.
- Locked equivalent (for completeness): the same data as *structured JSON with `gradYear`, `windReading`,
  `athleteId`, `ResultID`* is served only by `/api/v1/meets/<id>/performances` — robots-disallowed `[27]`
  and the path the paid "formatted" view itself uses.

### Incremental use

The boundary changes three decisions in the refresh loop:

1. **Profiles are not a change feed for results.** `lastmod`-driven profile re-fetching (sitemap) can
   only refresh identity/PR data; the result payload never appears for free. Use
   `(MeetID, RSID)` diffing on the results index + meet page, then fetch `/raw` only for new RSIDs.
2. **Roster sweep cadence** is unchanged (season boundaries, 1 req/team) — rosters are published in
   graders' terms, so one sweep at the start of each season covers the cohort.
3. **Free cross-check for PR drift:** the profile page's PR table + per-event row counts change when a
   new result is added, so a cheap profile re-fetch can *detect* an athlete's new activity without
   yielding the mark; the mark itself must come from the meet's `/raw`. This makes profiles an optional
   *signal*, never the *source*.

### Access characteristics

- **Classification:** normal server-rendered HTML for the whole recommended surface; the site also has
  an undocumented public JSON API (`/api/`) used by the paid views — `Disallow`ed by `robots.txt` `[27]`;
  and a **subscription tier (MileSplit PRO)** that removes masking server-side (data absent, not
  CSS-hidden, for anonymous clients).
- **robots.txt** (unchanged, `[27]`; re-consistent with all page behaviour here): `Disallow: /rankings`,
  `/virtual-meets`, `/api/`, `/contact`. Everything in the collector rules below is robots-allowed.
- **Rate limiting:** 56 curl requests in this run + 2 browser page loads; **all HTTP 200**, no 429, no
  `Retry-After`, no CAPTCHA. Max requests to one host = 13 (`wi`). Sequential with ≥1.2 s same-host gaps.
- **No bypass was attempted or needed:** no login, no PRO, no cookie replay, no CAPTCHA work, no
  `isMeetPro=1` data request, no requests to any `*.athletic.net` host (the browser's resource log for
  the two rendered pages contains ~110 third-party ad/analytics hosts — none of them `athletic.net`).

### Recommendation

**Keep MileSplit as RESULT-SOURCE + VALIDATION (as report 27 recommended), with an explicit
free/locked contract for the collector.** The paywall does not reduce the value of `/raw`, rosters,
entries, results index or meet pages at all; it only cancels the idea of per-athlete history from
profiles.

**Collector rules (do / never):**

- **NEVER fetch for data:** `/athletes/<id>` hoping for marks (payload is structurally absent — fetch it
  only for name/school/class/city-state/PR/counts); `/athletes/<id>/progression` (upsell only);
  `/rankings/events/…` or `/rankings/pro/…` (locked + robots-disallowed `[27]`);
  `/api/*` including `/api/v1/meets/<id>/performances` and the `/formatted` view (robots-disallowed);
  `/join`, `/login` (subscription gates — never attempt entitlement).
- **NEVER parse masked rows:** `<span class="mask">` contents are empty strings; a mask count is a
  *count*, not data. Do not treat `aria-label="Locked"` cells as fields.
- **FREE and preferred:** `/meets/<id>/results/<RSID>/raw` (marks/place/heat/wind/`Yr` — 1 req/result set);
  `/teams/<id>/roster` (AthleteID + Class + gender + season flags — 1 req/team);
  `/results?season=&level=hs&year=` (meet discovery); `/meets/<id>/results` (RSID + isMeetPro discovery);
  `/meets/<id>/entries` `[27]`; `/timing` `[27]`; `sitemap.xml` `[27]` (athlete change signal only).
- **Grades:** take `Yr` from `/raw` where the timer file has it; otherwise join name+school to
  `/teams/<id>/roster`'s `column-grad-year`. Never expect grade on `/api/`-fed views.
- **PR/identity validation:** profile `grad-year` + PR table (mark/date/national rank) are free and can
  corroborate Athletic.net history without an AN call; the *state* rank column is the one thing to leave alone.
- **PRO-file policy:** read `meetResultFiles[].isMeetPro` from the meet page and skip `isMeetPro:1`
  files (0 observed in the Midwest sample, but the contract exists).

---

### Free-surface capability matrix

| Surface | Recommended for collector | Name/ID | Class/grade | Marks | Meet/date | Wind/heat | Locks present | Cost |
|---|---|---|---|---|---|---|---|---|
| `/teams` index | yes | team id+name, city | – | – | – | – | none | 1/state |
| `/teams/<id>/roster` | **yes (primary census)** | athlete id + name | **yes (`column-grad-year`)** | – | – | – | **none (16/16)** | 1/team |
| `/teams/<id>` (landing) | no | team meta | – | – | – | – | *Ranked Performances* paywalled | 1/team |
| `/results` index | yes | meet id+name | – | – | date, venue | – | none | 1/page |
| `/meets/<id>/results` | yes | RSID, isMeetPro | – | – | meet dates | – | none | 1/meet |
| `/meets/<id>/results/<RSID>/raw` | **yes (primary results)** | team/athlete names (no ids) | `Yr` when timer file has it (5/6) | **yes (all)** | via meet page | heat; wind when present | **none (6/6)** | 1/result set |
| `/meets/<id>/results/<RSID>/formatted` | no | – | – | JS-only (`/api/`) | – | – | data path robots-disallowed | 1 |
| `/meets/<id>/entries` `[27]` | yes (pre-meet) | name+team (no id) | no | – | – | – | none | 1/meet |
| `/athletes/<id>` identity + PR | yes (validation) | id + name, school | **yes (`Class of YYYY`)** | PR only (mark+date) | PR dates | – | state-rank lock; results payload masked | 1/athlete |
| `/athletes/<id>` result skeleton | marginal | – | – | **no payload** | **no** | – | 5 empty mask cells/row (100 % rows) | same request |
| `/athletes/<id>/progression` | **never** | – | – | no | no | – | PRO upsell only | 1 |
| `/rankings/events/…`, `/rankings/pro/…` | **never** | – | – | =0 | – | – | rank 1 only; robots-disallowed `[27]` | – |
| `/api/v1/*` | **never** | (would carry payload) | yes | yes | yes | yes | robots-disallowed `[27]` | – |

### Evidence appendix

All requests 2026-09-20, method GET, from this workstation (browser UA), sequential with ≥1.2 s same-host
gaps; **all responses HTTP 200**; full raw captures in `research/midwest/evidence/gaps/32/` (file names
match the `capture` column; log = `fetch-log.tsv`).

| URL | method | HTTP | capture | what it proved | timestamp (UTC) |
|---|---|---|---|---|---|
| https://wi.milesplit.com/athletes/16319157-dina-abdel-megid | GET | 200 | a01 | free name/school/`Class of 2027`/city-state; 11 result rows all masked (44 plain + 11 seed cells = 55 masks); PR state-rank locked; `paywall_present:1`; `isFunnel=true` truncation guard present | 14:05:30 |
| https://wi.milesplit.com/athletes/16319414-kingston-penn | GET | 200 | a02 | 48 rows / 240 masks; "See all 48" = DOM rows; national rank `#181` free, state rank locked; `10.75` (his raw meet mark) absent → payload truly not delivered | 14:05:31 |
| https://mn.milesplit.com/athletes/11796978-regan-anderson | GET | 200 | a03 | 39 rows all masked; indoor+outdoor seasons both locked; 2 free national ranks | 14:05:32 |
| https://il.milesplit.com/athletes/11062721-amelia-benge | GET | 200 | a04 | 36 rows; PR table 23 DOM rows (5 visible/18 hidden incl. extra PRs) all present free; 9 free national ranks | 14:05:32 |
| https://mi.milesplit.com/athletes/16250380-darrell-foster | GET | 200 | a05 | small profile (3 rows) locked the same way — lock is not size-dependent | 14:05:32 |
| https://oh.milesplit.com/athletes/13426167-cameron-black | GET | 200 | a06 | 6 rows all masked (incl. a `cc` season block) | 14:05:33 |
| https://ia.milesplit.com/athletes/13856076-addie-bjork | GET | 200 | a07 | 39 rows; XC-heavy profile fully locked; 12 free national ranks | 14:05:33 |
| https://mo.milesplit.com/athletes/13058025-danica-ackerman | GET | 200 | a08 | 47 rows / 235 masks | 14:05:33 |
| https://ks.milesplit.com/athletes/13732171-kamdyn-affolter | GET | 200 | a09 | 44 rows; "See all 44" = DOM rows | 14:05:34 |
| https://ne.milesplit.com/athletes/15249012-emma-barnhill | GET | 200 | a10 | 7 rows; "See all 7" = DOM rows (smallest CTA sample) | 14:05:34 |
| https://sd.milesplit.com/athletes/11140853-charlie-alberts | GET | 200 | a11 | 32 rows; 28 PR DOM rows with 12 PRs only in hidden rows → full PR set is free | 14:05:34 |
| https://wi.milesplit.com/athletes/16319157-dina-abdel-megid/progression | GET | 200 | a12 | progression = PRO upsell; no data/API call; only a static example SVG | 14:05:35 |
| https://oh.milesplit.com/athletes/13426167-cameron-black/progression | GET | 200 | a13 | identical upsell on a second state host | 14:05:35 |
| https://mn.milesplit.com/teams | GET | 200 | b01 | 592 HS team records (index free) | 14:07:19 |
| https://il.milesplit.com/teams | GET | 200 | b02 | 849 HS team records | 14:07:19 |
| https://oh.milesplit.com/teams | GET | 200 | b03 | 977 HS team records | 14:07:20 |
| https://wi.milesplit.com/teams/14192-middleton/roster | GET | 200 | b04 | 324 rows w/ Class+gender+season flags; 0 masks/locks | 14:07:20 |
| https://wi.milesplit.com/teams/13976-aquinas/roster | GET | 200 | b05 | 99 rows free (27×2027) | 14:07:22 |
| https://wi.milesplit.com/teams/13938-milwaukee-king/roster | GET | 200 | b06 | 158 rows free (76×2027) | 14:07:23 |
| https://mn.milesplit.com/teams/13138-glencoe-silver-lake-high-school/roster | GET | 200 | b07 | 88 rows free | 14:07:23 |
| https://mn.milesplit.com/teams/13450-wayzata/roster | GET | 200 | b08 | 1,154 rows in one request, 0 locks (no truncation) | 14:07:25 |
| https://il.milesplit.com/teams/29127-abingdon-a-avon/roster | GET | 200 | b09 | 32 rows free | 14:07:25 |
| https://oh.milesplit.com/teams/55562-acad-for-urban-scholars/roster | GET | 200 | b10 | 36 rows free | 14:07:25 |
| https://mn.milesplit.com/teams/13264-minnetonka-high-school/roster | GET | 200 | c01 | 382 rows free | 14:07:42 |
| https://mn.milesplit.com/teams/13352-rosemount-high-school/roster | GET | 200 | c02 | 677 rows free | 14:07:44 |
| https://il.milesplit.com/teams/10953-naperville-neuqua-valley/roster | GET | 200 | c03 | 341 rows free | 14:07:44 |
| https://il.milesplit.com/teams/12207-hinsdale-central/roster | GET | 200 | c04 | 333 rows free | 14:07:46 |
| https://oh.milesplit.com/teams/9558-dub-coffman/roster | GET | 200 | c05 | 294 rows free | 14:07:46 |
| https://oh.milesplit.com/teams/10002-mason/roster | GET | 200 | c06 | 321 rows free | 14:07:47 |
| https://wi.milesplit.com/meets/763484-big-8-conference-2026/results | GET | 200 | c07 | `meetResultFiles=[{id:1304175,isMeetPro:0}]`; RSID discovery free | 14:07:48 |
| https://mn.milesplit.com/results?year=2026&season=outdoor&level=hs | GET | 200 | c08 | 50 meets/page free w/ `data-meet-id` | 14:07:48 |
| https://il.milesplit.com/results?year=2026&season=outdoor&level=hs | GET | 200 | c09 | same on IL | 14:07:48 |
| https://oh.milesplit.com/results?year=2026&season=outdoor&level=hs | GET | 200 | c10 | same on OH | 14:07:49 |
| https://wi.milesplit.com/meets/750759-2026-wiaa-wi-outdoor-championships-2026/results | GET | 200 | d01 | state-meet result file 1308252, isMeetPro 0 | 14:08:02 |
| https://mn.milesplit.com/meets/767110-mini-apple-night-of-miles-2026/results | GET | 200 | d02 | `meetResultFiles=[]` yet a `/results/1308498/raw` link exists — RSID discovery is not only via that array | 14:08:02 |
| https://il.milesplit.com/meets/749383-ihsa-boys-track-and-field-state-championships-2026/results | GET | 200 | d03 | file 1307551 "Complete", isMeetPro 0 | 14:08:02 |
| https://oh.milesplit.com/meets/767165-419-flyers-invitational-2026/results | GET | 200 | d04 | file 1309698, isMeetPro 0 | 14:08:03 |
| https://wi.milesplit.com/meets/750759-2026-wiaa-wi-outdoor-championships-2026/results/1308252/raw | GET | 200 | e01 | 3,755 lines / 2,523 place rows, `Yr` 9–12, wind column, 0 masks; relay legs render as `1) A` | 14:08:10 |
| https://wi.milesplit.com/meets/750759-2026-wiaa-wi-outdoor-championships-2026/results/1308252/formatted | GET | 200 | e02 | JS shell: 0 `<tr>` in HTML; loads `loadResultsNew.js` (API-driven) → not a collector surface | 14:08:12 |
| https://mn.milesplit.com/meets/767110-mini-apple-night-of-miles-2026/results/1308498/raw | GET | 200 | e03 | 305 lines; `Yr` incl. `Sr`/`Fr`/`-`; 0 masks | 14:08:12 |
| https://il.milesplit.com/meets/749383-ihsa-boys-track-and-field-state-championships-2026/results/1307551/raw | GET | 200 | e04 | 2,792 lines, tab-delimited, **no `Yr` column**; relay leg names present; 0 masks | 14:08:12 |
| https://oh.milesplit.com/meets/767165-419-flyers-invitational-2026/results/1309698/raw | GET | 200 | e05 | 1,489 lines w/ `Yr` column; 0 masks | 14:08:13 |
| https://www.milesplit.com/meets/768819-aau-junior-olympic-games-2026/results | GET | 200 | e06 | 5 result files, all `isMeetPro:0` | 14:08:13 |
| https://js.sp.milesplit.com/drivefaze/meets/loadResultsNew.js?build=20260920074803 | GET | 200 | f01 | `/formatted` → `API.get('v1/meets/<id>/performances', {isMeetPro, resultsId, fields:[…gradYear…windReading…]})`; PRO-file request logic (`isMeetPro:1`, `excludeMeetPro`) | 14:08:32 |
| https://wi.milesplit.com/athletes/14085147-pongonis-will | GET | 200 | g01 | 88 rows / 440 masks; "See all 88" = DOM rows (largest CTA sample) | 14:09:17 |
| https://il.milesplit.com/meets/768526-aau-region-13-qualifier-2026/results | GET | 200 | g02 | file 1310168, isMeetPro 0 | 14:09:17 |
| https://wi.milesplit.com/teams/14192-middleton | GET | 200 | g03 | team landing locks *Ranked Performances* (`paywall-overlay-container paywall-active`, "To unlock all rankings, subscribe to") while linking the fully-free roster | 14:09:17 |
| https://il.milesplit.com/meets/768526-aau-region-13-qualifier-2026/results/1310168/raw | GET | 200 | h01 | 5,500 lines, `Yr` present; 0 masks | 14:09:31 |
| https://www.milesplit.com/meets/704190-adidas-track-nationals-2026/results | GET | 200 | h02 | 8 result files, all `isMeetPro:0` (18 result files counted across the 7 meet pages that carried files; none PRO) | 14:09:31 |
| https://mn.milesplit.com/athletes/11174432-thomas-barrett | GET | 200 | i01 | 45 rows; "See all 45" = DOM rows; 15 free national ranks | 14:09:59 |
| https://mn.milesplit.com/athletes/13808215-valentina-bernini | GET | 200 | i02 | 33 rows all masked | 14:10:01 |
| https://wi.milesplit.com/meets/763484-big-8-conference-2026/results/1304175/raw | GET | 200 | j01 | conference-meet raw: grades {9:94,10:167,11:219,12:206} → ordinary HS meets are grade-bearing and free | 14:10:10 |
| https://mi.milesplit.com/teams | GET | 200 | k00 | 895 MI team records | 14:10:44 |
| https://mi.milesplit.com/teams/12731-muskegon/roster | GET | 200 | l01 | 263 rows free (MI rosters are populated here, contra the sparse MI sample in [27]) | 14:10:47 |
| https://mi.milesplit.com/teams/12144-saline/roster | GET | 200 | l02 | 494 rows free | 14:10:49 |
| https://mi.milesplit.com/teams/12800-ann-arbor-pioneer/roster | GET | 200 | l03 | 528 rows free | 14:10:50 |
| https://wi.milesplit.com/athletes/16319157-dina-abdel-megid/progression (headless Chromium) | GET | 200 | browser | rendered DOM: 0 masks, 0 canvases, 1 static example SVG; ~110 third-party hosts, **no `athletic.net`** | 14:12 |
| https://wi.milesplit.com/athletes/16319157-dina-abdel-megid (headless Chromium) | GET | 200 | browser | rendered DOM: 11 `.record`, 55 `.mask`, first row `innerText=""`; "Show all PRs" + season toggle fire **no** server request | 14:13 |

Cross-references: `[27]` = `research/midwest/27-milesplit-super-index.md` (robots policy, rankings-tree
lock, roster recipe, entries/timing/sitemap surfaces); `[07]` = `research/midwest/07-wisconsin-milesplit.md`
(performance API field list, state-meet row counts used above as the raw-vs-API completeness backdrop).
