# 16. Michigan athlete/result alternatives

Status: complete — all four assignment steps answered from live observation; five items could not be
observed and are flagged inline: (a) the public result-level fields behind MileSplit's JS/API result
views (robots-disallowed, see Access); (b) RaceResult and AthleticLIVE result-row fields (robots / JS
app); (c) PTTiming field detail (terms prohibit automated extraction, so no data endpoint was
touched); (d) whether the MileSplit Michigan sitemap's 7,500-entry window is a cap or a rolling
recency window (constant across two pulls, no sitemap index exists); (e) the official Michigan Indoor
Track Series (MITS) archive, which could not be located because every general web-search tool was
unavailable this session (see note under Access characteristics).
Observed on: 2026-09-19

No CAPTCHA, authentication, paywall or robots-disallowed data path was used to obtain any result in
this report. The `web_search` tool failed with `Sign up and repeat your request.` (verbatim, twice);
Mojeek answered with a CAPTCHA page (not solved); Bing returned only generic Michigan geography
results for a timer query. All discovery below is therefore navigation-based from primary sites.

## Source

Providers investigated (all Michigan-facing except where noted):

| Source | Role | Coverage observed | Stable IDs | Grad-year evidence |
|---|---|---|---|---|
| MileSplit Michigan (`mi.milesplit.com`) | PRIMARY athlete seeding | MI; HS/MS/college/club; XC + indoor + outdoor; calendar back to ≥2015 | athlete id, team id, meet id (`data-meet-id`), raw result-set id | **Yes — roster `Class` column and profile "Class of YYYY"** |
| MITCA (`mitca.org`) | Athletic.net MeetID seed + PDF/HTM archive | MI team state finals 2004–2022+, Meet of Champions, Champions of Champions | Athletic.net MeetIDs on page | No |
| Superior Timing LLC (`superiortiming.com`) | Results host, UP + mid-MI | Meet archive 2009–2026 with dates/city/discipline | Athletic.net MeetIDs, RaceResult event ids | Not observed |
| A2 Race Management (`a2racemanagement.com`) | Results host, Ann Arbor | TF 2012–2024, XC 2011–2024 | Athletic.net team id 42482 link | No |
| CHT Timing (`chttiming.com`) | Results host, mid-Michigan | Track/XC live results; Meet of Champions results | AthleticLIVE meet id (e.g. 59051) | No |
| PT Timing / Karma Race Management (`live.pttiming.com`) | Portage Invite results | Portage Invite 2023–2025 (6,000+ runners) | `mid` meet numbers (5836/7465/8160) | Not observable — terms prohibit automated extraction |
| RaceResult (`my.raceresult.com`) | Generic timing platform used by MI timers | Superior Timing XC/road meets 2026 | RaceResult event id | Not observed |
| RunMichigan (`runmichigan.com`) | News/video/photo archive | MHSAA finals coverage 2022–2026; road-race calendar | `view.php?id=N` | No |
| MichiganCrossCounty.com | Coaches' team rankings blog | Week-1 team rankings by LP division/region, Sept 14 2026 | none (team names) | No |
| Baum's Page (`baumspage.com`) | **Ohio**, not Michigan | OHSAA state/district tournament results | n/a for MI | n/a |
| MHSAA (`mhsaa.com`) | Out of scope — owned by agent 15 | — | — | — |

## Coverage

- **States:** Michigan only for every source here (MileSplit Michigan is the `mi` subdomain of the
  MileSplit/FloSports network; Superior Timing, A2RM, CHT, PT Timing are Michigan timing operators).
- **Sports:** cross country, indoor track, outdoor track. MileSplit marks indoor/outdoor/XC
  availability per athlete on the roster page; the meet calendar is queryable per season. Michigan
  timers split by discipline: Superior Timing ran XC meets (e.g. MSU Spartan XC Invitational, East
  Lansing, 2026-09-11) and road races; A2RM publishes both TF and XC year archives; CHT publishes
  track/XC live results; PT Timing times the Portage Invitational XC meet.
- **School levels:** MileSplit Michigan team directory separates High School (895 teams), Middle
  School (382), College/University, Elementary, Junior/Community College, Club, Company,
  Professional, Olympic/National, Group, Unattached
  (`/teams` level `<select>` options 1–11; `?type=1` = High School default, `?type=3` = Middle
  School — both observed).
- **Seasons and historical depth:**
  - MileSplit calendar: XC Aug 2026 = 83 meets, Sep 2026 = 254, Oct 2026 = 41, Nov 2026 = 7;
    outdoor May 2026 = 942 meets; indoor Jan 2026 = 40 meets; outdoor **May 2015 = 286 meets**
    (so the calendar reaches at least 11 years back).
  - Athlete PR tables carry old marks: a Class-of-2027 Saline athlete shows a 3200 m mark dated
    2021-09-18; Luka Hammond's PR table includes 2021 XC/2-mile seasons.
  - Superior Timing results archive is exposed by year 2009–2026; A2RM XC 2011–2024 and TF 2012–2024;
    MITCA hosts Team State PDFs from 2004–2022 and Athletic.net links for 2017/2021/2022.
- **Class-of-2027 relevance:** the roster `Class` column is the athlete's graduation year, so
  `Class = 2027` in the 2026-27 school year is exactly the target population; verified against
  profile pages that print `Class of 2027` (Landon Stukey, Saline; Luka Hammond, Grand Haven).

## Enumeration

Exact recipes (all GET, all server-rendered HTML unless noted):

1. **Schools/teams (MileSplit).** `https://mi.milesplit.com/teams` → one 288,057-byte page containing
   the complete High-School level list: **895 unique team links**. Level is a form select
   (`type` param); `?type=3` returned 382 middle-school teams, which confirms the param works and that
   the default list is the HS universe. No pagination control was present on either page.
2. **Team rosters / Class-of-2027 (MileSplit).** `https://mi.milesplit.com/teams/<teamId>-<slug>/roster`
   → 0.03–1.04 MB server-rendered page whose data set is
   `<ul id="rosterDataset">` with one `<li class="athlete-row data-row">` per athlete and columns
   `Athlete | Gender | Class | Indoor | Outdoor | XC`. Athlete cell links to
   `/athletes/<id>-<slug>`; `Class` is the four-digit graduation year (`0` = unknown); sport columns
   are `icon-yes` / `icon-no` SVGs. The three filter selects (`rosterFilterType`,
   `rosterFilterClass`, `rosterFilterGender`) have **no `name` attribute and no hrefs — filtering is
   client-side only**, so one request returns every grade and both genders.
3. **Athletes (MileSplit).** `https://mi.milesplit.com/athletes/<id>-<slug>` → server-rendered
   profile: name, school, city/state, `Class of YYYY`, and a Personal Records table
   (`Event | Mark | State | National | Date`) grouped by XC/Indoor/Outdoor. The full result list
   ("See all N of … results") is PRO-subscriber gated.
4. **Meets (MileSplit).** `https://mi.milesplit.com/calendar?season=<cc|indoor|outdoor>&year=YYYY&month=M`
   → server-rendered month list; each row is
   `<li class="meet-row" data-meet-id="NNNNN" data-level="hs,ms">` with a
   `https://www.milesplit.com/meets/<id>-<slug>` link, date and venue. `season=cc` = cross country;
   `season=indoor` (Jan 2026: 40 rows) and `season=outdoor` (May 2026: 942 rows) both work.
5. **New/changed athletes (MileSplit).** `https://mi.milesplit.com/sitemap.xml` → 918,360-byte
   sitemap with exactly **7,500 `<url><loc>…/athletes/<id>-<slug></loc><lastmod>…</lastmod></url>`
   entries**, sorted newest-first by `lastmod` spanning 2026-08-28T05:06:53-04:00 to
   2026-09-19T03:08:57-04:00. No sitemap index exists (`/sitemap_index.xml`, `/sitemap2.xml` both 404);
   no team or meet URLs are in it.
6. **Association/archive (MITCA).** `https://mitca.org/MITCA/mitca-team-state/team-state-results/`
   (8 Athletic.net meet links + 10 PDFs + 1 sportstats link);
   `https://mitca.org/MITCA/champions-of-champions/` (3 Athletic.net meet links);
   `https://mitca.org/MITCA/michigan-meet-of-champions/` (3 `.htm` result files + 1 AthleticLIVE link).
7. **Timers.** Superior Timing: `https://www.superiortiming.com/category/raceresults/` lists meets by
   year (`/category/raceresults/year/2026`) with date, city and discipline, and each row links to the
   result platform. A2RM: `https://a2racemanagement.com/results/{cross-country,track}/<year-slug>/`
   (year slugs are irregular, e.g. `2024-2/`; `2024-xc/` is a 404 — read the nav hrefs, don't guess).
   CHT: `https://chttiming.com/results` (1.7 KB page of live-result links). PT Timing: Portage
   Invitational site links `https://live.pttiming.com/xc-ptt.html?mid=<meet>`.
8. **Can we enumerate each target class?**
   - schools/teams: yes (MileSplit `/teams`).
   - meets: yes (MileSplit calendar by month/season; timers' year archives).
   - athletes: yes (rosters; sitemap for recent activity).
   - **Class of 2027 athletes: yes** (roster `Class=2027`; profile `Class of 2027`).
   - results/marks: **partially** — individual PR marks and dates on public athlete pages; full
     result rows/places/meets are PRO-gated (MileSplit) or restricted (PTTiming terms, RaceResult
     robots), or published as static files (MITCA `.htm`).

## Stable identifiers

- **MileSplit athlete id** — numeric, first path segment of `/athletes/<id>-<slug>`
  (e.g. 11141513 = Luka Hammond, 11183082 = Landon Stukey, 16889941 = Aletta Scholten). The slug is
  decorative: `GET https://mi.milesplit.com/athletes/16811686-luka-hammond` returned **200 after
  redirecting to `https://tn.milesplit.com/athletes/16811686-bennett-arant`** — the same numeric id
  resolves on a different state site, so the athlete id space is network-global and the slug must
  never be used as a key.
- **MileSplit team id** — `/teams/<id>-<slug>` (Saline 12144, Rockford 1228, Grand Haven 12203,
  Detroit Catholic Central 12439, Boyne Falls 12317, Chasséll-class small schools 12382/12728).
- **MileSplit meet id** — `data-meet-id` on calendar rows (e.g. 779295 Logger Invite 2026-09-19;
  782352 Blue Bolt – Panther Invite) and in meet URLs.
- **MileSplit raw result-set id** — `/meets/<meetId>/results/<rawId>/raw`
  (e.g. `/meets/779295-logger-invite-2026-2026/results/1322031/raw`); a second id layer whose
  content is served only to the JS app.
- **No event id / result id / season id** was visible in HTML: events appear as names
  (800m, 1600m, 5K), the roster's sport cells carry an empty `data-season-id`, and the athlete page's
  season selector is JS. MileSplit ids are therefore athlete/team/meet-level only.
- **Athletic.net MeetIDs surfaced by third parties:** 284165, 284166, 284167, 284168 (MITCA 2017
  D1–D4), 437258, 437267 (2021 D1/D4), 474685, 474690 (2022 D1/D4), 564539/529506/503468
  (Champions of Champions), live.athletic.net/meets/77232 and /77753 (Superior Timing),
  results.superiortiming.com/meets/77216, live.chttiming.com/meets/59051 (CHT). Athletic.net team id
  **42482** appears as A2RM's linked team.
- **Timer ids:** Superior Timing/RaceResult event ids (423470, 421834, 422902, 422929, 423878),
  PT Timing `mid` values (5836 = 2023 Portage, 7465 = 2024, 8160 = 2025), RunMichigan `view.php?id=N`
  (view.php ids 40079–40082 = 2026 MHSAA LP T&F finals, D4–D1), MITCA file paths (stable upload URLs).
- **Do not treat names as identifiers:** MileSplit slugs and Athletic.net slugs both change; two
  different Saline athletes appear as "Jack Klein" and "John (Jack) Klein", and A2RM/MileSplit meet
  names for the same event differ ("Blue Bolt - Panther Invite" vs "2026 Blue Bolt – Panther Invite").

## Athletic.net leverage

- **Direct links from MileSplit Michigan: none.** Across every MileSplit MI page captured (meet page,
  results page, raw page, 12 team/roster pages, 3 athlete profiles, 6 calendar months, team
  directory), a case-insensitive scan for `athletic.net` returned **zero** matches. MileSplit supplies
  no Athletic.net meet/team/athlete/result URLs.
- **Direct links from third parties: yes, meet-level only.**
  - MITCA: 8 Athletic.net meet links on the Team State Results page, 3 on Champions of Champions.
  - Superior Timing: 2 `live.athletic.net/meets/<id>` links on the home page + 1 on its calendar page,
    and its own branded results host `results.superiortiming.com/meets/<id>` shares the Athletic.net
    live-meet id space.
  - A2RM: `athletic.net/team/42482/{cross-country,track-and-field-outdoor}/2024` on its year pages.
  - CHT: `anet.live/6glmne` (Athletic.net live short-link) from its results page.
  - No source observed exposes Athletic.net **athlete** ids.
- **Deterministic seeding without Athletic.net search:** roster rows give
  `(school, athlete name, grad year, gender, sport flags)`. That is enough to pick the right Athletic.net
  team page and then one candidate athlete, but it is not an ID hand-off, so at best it converts a
  rankings-wide search into a single targeted team/profile request. Marked
  `[INFERENCE]` where used for pipeline sizing.
- **Request-avoidance estimate (Michigan):**
  - Roster sweep = **895 MileSplit requests** for the whole HS team universe; a 12-team sample
    (10 with data) yielded **442 Class-of-2027 athletes of 2,581 roster entries** → 44.2 per team
    (upper bound, see caveats). If the pipeline would otherwise spend one Athletic.net profile
    request per athlete (peer report 02 documents `GetAthleteBioData` as a per-athlete, per-sport
    call), the roster route pre-empts up to ~44 Athletic.net requests per team request, i.e. the
    Athletic.net spend for discovery drops to zero and only targeted verification remains.
  - Sitemap route = **1 request** returning 7,500 recently-updated athlete URLs with per-URL
    `lastmod` (window 2026-08-28 → 2026-09-19, 23 days) — the cheapest change feed observed anywhere
    in this assignment.
  - Meet route: MITCA/timer pages hand over ~14 Athletic.net MeetIDs for Michigan championship and
    invitational meets at a cost of 4 page fetches.
  - Caveat on the 44/team figure: roster `Class` counts include every athlete ever attached to the
    team (grades 2027–2033 appear on one roster) and 2 of 12 sampled teams returned an empty roster,
    so 44/team is an upper bound, not an active-athlete count. XC-flagged Class-of-2027 athletes
    average 15.5 per team (155/10), outdoor-flagged 40.1 (401/10).

## Athlete evidence

From MileSplit roster + public profile (verified on 12 rosters and 3 profiles):

- name — yes (roster `Last, First`, profile `First Last`).
- **graduating class/grade — yes** (roster `Class` column = graduation year; profile literal
  `Class of 2027`; verified for Landon Stukey and Luka Hammond).
- school — yes (roster context + profile `Saline`, `Grand Haven`).
- city/state — yes (profile `Saline, MI`; team page carries the school's street address, e.g.
  `01662 M-75 S Boyne Falls, MI`).
- gender/category — yes, roster column `f`/`m` (Luka Hammond `m`, Aletta Scholten `f`).
  **Data-quality note:** the JSON-LD `gender` field on an athlete page can contradict the roster
  (Landon Stukey's JSON-LD says `Female`, roster says `m`); use the roster column.
- TF/XC distinction — yes, per-athlete `Indoor`/`Outdoor`/`XC` yes/no icons on the roster.
- indoor/outdoor — yes as above; indoor coverage is thin in the sample (33/494 Saline athletes
  flagged indoor; 25/175 Howell; 37/430 Grand Haven).
- performances — yes, PR table (event, mark, date, national rank where applicable); e.g.
  Luka Hammond 800 m 1:51.29 (#89 national), 5K 14:50.10 (#30), 37 PR rows.
- PRs — yes (same table).
- progression — **no** on public pages (`Progression` tab and "College Progression Tracker" are PRO).
- meets — **no** on public pages (full result list, incl. meet names, is PRO-gated:
  "Full performance history, included with MileSplit PRO").
- athlete profile URL — yes, canonical `/athletes/<id>-<slug>`.
- Coverage counts observed (Class-of-2027 per single roster request): Boyne Falls 4, Sterling Heights
  23, Croswell-Lexington 28, Negaunee 28, Howell 32, Detroit Catholic Central 45, Woodhaven 57,
  Grand Haven 58, Saline 63, Rockford 104. Gender split is available (e.g. Saline 29 f / 34 m;
  Rockford 49 f / 55 m; DCC 45 m — boys school).
- `Class` value `0` (unknown grade) appears on every roster (Saline 38/494, Boyne Falls 17/55,
  Rockford 52/345) — those athletes cannot be Class-of-2027 verified from the roster alone.

## Recruiting information

- MileSplit Michigan exposes **no coach or athletic-director fields** on the pages sampled: the team
  page shows name, street address, city/state, "Claim Team" plus rankings/videos/photos/news/roster
  tabs; no staff section, no coach email. Coaches are out of scope for this source.
- MITCA is a coaches' association and publishes its own committee/contact structure
  (`/MITCA/about-mitca/mitca-committees-contacts/`) — that is MITCA governance contact information,
  not per-school coach/AD contact data, and it must not be treated as a school coach directory.
- No other source in this slice published coach or AD names, emails or school athletics website URLs
  (MichiganCrossCounty.com publishes team rankings only; RunMichigan publishes news/videos).
- Recommendation for the coach graph: use agent 29's hierarchy (state association → school district
  directory). Nothing in this Michigan alternative slice contributes coach contacts.
- Athlete personal contact data was not collected anywhere; nothing in these sources displayed it.

## Result evidence

| Field | MileSplit public HTML | MITCA `.htm` files | Timer platforms (RaceResult / AthleticLIVE / PTTiming) |
|---|---|---|---|
| ResultID | no | no (row position only) | not observable (robots/terms) |
| AthleteID | yes (MileSplit athlete id) | no (Bib No + name) | not observable |
| MeetID | yes (`data-meet-id`) | no (file path per meet) | yes (platform event id / `mid`) |
| EventID | no (event names only) | no (5K single-event race) | not observable |
| mark | yes — PR table (mark + date) | yes (1 Mile / 2 Mile / Last 1 / Time / Pace) | not observable |
| normalized mark inputs | no (display marks only) | partial (splits) | not observable |
| timing method | no | "Results by CHT Timing" credit | platform credit |
| wind | no | no | not observable |
| implement/hurdle spec | no | n/a (XC) | not observable |
| heat/round | no | no (final list only) | not observable |
| place | no (PR table has no place) | yes (Finish Place, Score) | not observable |
| date | yes (PR date) | yes (meet date header) | yes (meet date in title) |
| school represented | yes (school of record) | yes (`Team` column, but club names at the elite Meet of Champions: "AA Running Co", "Runners Athletic") | not observable |
| relay membership | no | no | not observable |

Key constraint: MileSplit meet result pages are an empty JS shell to non-browser clients — the raw
result page (`…/results/1322031/raw`, 59,133 bytes) contains no marks, and the data endpoint lives on
`api30.milesplit.com`, whose robots.txt disallows everything except `/api/experiences/web/`. So for
results the usable public artifacts are **PR rows on athlete pages** (MileSplit) and **static result
files published by associations** (MITCA `.htm`/PDF; Athletic.net MeetIDs for the rest).

## Incremental use

Weekly collector design that avoids re-fetching history:

- **Meet change feed (1 request/month/season):** fetch
  `/calendar?season=<cc|indoor|outdoor>&year=Y&month=M`, extract `data-meet-id` + name + date, and diff
  against the stored set. New/changed ids are the only meets to pull. Michigan XC 2026 so far:
  Aug 83 + Sep 254 + Oct 41 + Nov 7 rows; outdoor May 2026 had 942 rows, so budget one page per month
  per season, not per meet.
- **Athlete change feed (1 request):** `GET /sitemap.xml`; diff `<loc>` URLs and `lastmod`. Only new
  or newer `lastmod` athlete URLs need a profile fetch. Two pulls 4.5 minutes apart returned an
  identical 7,500-entry set, so the feed is stable at hour granularity.
- **Roster refresh (895 requests, weekly is wasteful):** rosters only change when teams add/remove
  athletes, which the sitemap already signals. Recommended cadence: one full roster sweep at the
  start of each season (Aug for XC, Mar for outdoor TF) and re-pull only teams whose athletes appear
  in the sitemap diff.
- **Timer/association archives:** MITCA and timer year pages are static HTML; poll the 4 association
  pages + Superior Timing's home "Recent Results" + `chttiming.com/results` weekly (≈6 requests).
  No `If-Modified-Since`/ETag was tested; the pages are small (1.7–77 KB).
- **State finals:** MHSAA/regionals finals are agent 15's surface; the Athletic.net MeetIDs surfaced
  by MITCA can be used to pull whole-meet results once per final instead of per athlete.

## Access characteristics

- **MileSplit Michigan — normal HTML (server-rendered) with a robots-published carve-out.**
  `https://mi.milesplit.com/robots.txt` (200) declares for `User-agent: *`:
  `Disallow: /rankings`, `Disallow: /virtual-meets`, `Disallow: /api/`, `Disallow: /contact`.
  `https://api30.milesplit.com/robots.txt` (200) declares `Disallow: /` with a single
  `Allow: /api/experiences/web/`. The pages this report relies on (`/teams`, `/teams/<id>/roster`,
  `/athletes/<id>`, `/calendar`, `/sitemap.xml`) are **not** under any Disallow. Result lists and
  rankings are effectively off-limits to automation, and result history is a paid PRO feature
  "Full performance history, included with MileSplit PRO"). No 429 response and no `Retry-After`
  header were observed; requests were issued sequentially (~40 for this host).
- **MITCA (`mitca.org`) — normal HTML + downloadable PDF/HTM.** `/robots.txt` returns 404 (no policy
  published). WordPress site (`/MITCA/…`), static pages, stable upload URLs. No limits observed.
- **A2 Race Management — normal HTML (WordPress).** robots.txt 200: `Disallow: /wp-admin/`,
  `Allow: /wp-admin/admin-ajax.php`, sitemap `wp-sitemap.xml`. Live results live on an AthleticLIVE
  white-label (`live.a2racemanagement.com/meet-list` returned the AthleticLIVE JS shell).
- **Superior Timing — normal HTML (WordPress).** `/robots.txt` 301 (redirect, no policy served at
  root). Results archive pages are static and small.
- **CHT Timing — normal HTML.** Static 1.7 KB results page; live results are an AthleticLIVE
  white-label (`live.chttiming.com/meets/59051` served the AthleticLIVE shell, 50,184 bytes).
- **PT Timing (`live.pttiming.com`) — machine-readable prohibition.** robots.txt preamble states:
  "Automated bulk extraction of results data, and any use of this site's content for training,
  fine-tuning, grounding or evaluating AI/ML systems, is PROHIBITED by the Terms of Use at
  /terms.html, whether or not a crawler is named in this file. … Want this data? We license it.
  legal@karmarush.com". The `User-agent: *` block allows general indexing but disallows `/lib/`,
  timekeeper tools, `/scoreboards/`, `/pwa/`. **Do not build a collector against PTTiming**; the
  only compliant route is a licence. (Disclosure: this report fetched only their robots.txt, the
  human-facing Portage results URL and, before reading the policy, one static bundle file under
  `/lib/`; no result data was extracted and no further requests were made.)
- **RaceResult (`my.raceresult.com`) — restrictive robots.** robots.txt 200 disallows
  `/*/*/certificates`, `/RREvents`, `/RRPublish`, `/RRRegStart`, `/*/*/list`, `/*/*/pdf`,
  `/*/*/view`, `/users`. The event root page (`/423878/`) is a JS shell; the result list/view/PDF
  paths a collector would need are disallowed. Treat as unavailable for automation.
- **AthleticLIVE (`live.athletic.net`, white-labels) — browser application.** 50,184-byte JS shell
  (bundle `livestatic.athletic.net/main-*.js`); no HTML data. `www.athletic.net` remains
  Cloudflare-403 to non-browser clients from this machine (mission brief, 2026-09-19).
- **RunMichigan — normal HTML.** robots.txt 301 (none). Result coverage is road races; HS pages are
  video/photo/news (e.g. `view.php?id=40082`, 2026 MHSAA LP T&F Finals D1, links YouTube).
- **Search engines were unusable this session** and are not a dependency of the above:
  built-in `web_search` → `Sign up and repeat your request.`; Mojeek → CAPTCHA (not solved);
  Bing → 200 but generic results; DuckDuckGo HTML → 202 with no organic results.

## Recommendation

**PRIMARY (athlete seeding) — MileSplit Michigan rosters + sitemap.** One request per team returns the
complete multi-grade roster with **graduation-year evidence**, gender and Indoor/Outdoor/XC flags, and
one request returns a 7,500-URL change feed with `lastmod`. This is the only source in this slice that
independently produces Class-of-2027 identities (not just names) for Michigan, and it costs no
Athletic.net requests. Expected marginal coverage: ~640–895 Michigan HS teams (2/12 sampled had empty
rosters) × ~44 Class-of-2027 per team (upper bound; ~15 XC-flagged) — i.e. the discovery layer for
the whole state, with Athletic.net then needed only for meet-by-meet verification.

**ATHLETIC.NET-SEED (meet ids) — MITCA + timers.** MITCA pages hand over 11 Athletic.net MeetIDs for
championship meets; Superior Timing/A2RM/CHT white-labels expose Athletic.net live meet ids for
regular-season invitationals. Use these to target whole-meet pulls instead of per-athlete profile
requests.

**RESULT-SOURCE (narrow) — MITCA static files.** MITCA `.htm`/PDF files give place, name, team and
splits for the Meet of Champions/Team State with no Athletic.net dependency, but they are elite
post-season events with club (not school) team labels and no grade column. Marginal coverage is small.

**DISCOVERY-ONLY — MichiganCrossCounty.com, RunMichigan.** Team rankings / video news, no athlete or
result records.

**REJECT — PTTiming (terms prohibit automated extraction; licensing required), RaceResult result
endpoints (robots), MileSplit `/rankings` + `/api/` (robots).** Baum's Page is Ohio-scoped and not a
Michigan source.

**Gaps and risks.** (1) No independent Michigan **indoor** result archive was found — MileSplit's
indoor calendar exists (Jan 2026: 40 meets) but MileSplit does not publish public result rows, and the
MITS site could not be located without a search engine. (2) **Grade is never available on result
files** — only on MileSplit rosters/profiles. (3) Roster `Class` counts are all-time unions and
include `0`-graded athletes; treat 44/team as an upper bound and expect Athletic.net corroboration for
the final Class-of-2027 assertion. (4) Whether all 895 teams have roster data is unverified (12-team
sample); a full sweep is 895 requests and should be staged. (5) MileSplit IDs are network-global and
slugs are not identifiers — key everything on the numeric id.

### Evidence appendix

All requests issued sequentially from this machine on 2026-09-19; times are local (UTC-5/-6 zone
clock of the workstation). `UA` = `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like
Gecko) Chrome/126.0 Safari/537.36` for HTML fetches; no cookies, no auth, no proxy, no CAPTCHA solving.

| URL | Method | HTTP status | What it proved | Timestamp |
|---|---|---|---|---|
| https://mi.milesplit.com/ | GET | 200 (103,114 B) | MileSplit Michigan live from this machine; meet/athlete/calendar URL shapes | 2026-09-19 23:13:05 |
| https://mi.milesplit.com/robots.txt | GET | 200 | Disallow `/rankings`, `/virtual-meets`, `/api/`, `/contact` for `*` | 2026-09-19 23:13:21 |
| https://mi.milesplit.com/meets/779295-logger-invite-2026-2026/results | GET | 200 (59,048 B) | Meet results page is a JS shell; JSON-LD SportsEvent only; no `athletic.net` refs | 2026-09-19 23:13:22 |
| https://api30.milesplit.com/robots.txt | GET | 200 | API host disallows all but `/api/experiences/web/` | 2026-09-19 23:13:33 |
| https://js.sp.milesplit.com/drivefaze/api.js | GET | 200 (6,877 B) | Drivefaze API client (base URL injected per page) | 2026-09-19 23:13:33 |
| https://js.sp.milesplit.com/drivefaze/meets/meets.js | GET | 200 (2,377 B) | Results view switcher is client-side only | 2026-09-19 23:13:33 |
| https://mi.milesplit.com/meets/779295-logger-invite-2026-2026/results/1322031/raw | GET | 200 (59,133 B) | "Results (Raw)" is also a JS shell — no marks in HTML | 2026-09-19 23:13:33 |
| https://mi.milesplit.com/teams/12317-boyne-falls | GET | 200 (42,146 B) | Team page: name/address/roster tabs; no coach fields; 0 server-side tables | 2026-09-19 23:13:43 |
| https://mi.milesplit.com/meets/779295-logger-invite-2026-2026/results | GET | 200 (59,048 B) | Same shell re-fetched; confirms JS-only results | 2026-09-19 23:13:44 |
| https://mi.milesplit.com/sitemap.xml | GET | 200 (918,360 B) | 7,500 athlete URLs with `lastmod` 2026-08-28 → 2026-09-19 | 2026-09-19 23:13:51 |
| https://mi.milesplit.com/calendar | GET | 200 (351,743 B) | 254 server-rendered meet rows with `data-meet-id` (Sept 2026) | 2026-09-19 23:13:52 |
| https://mi.milesplit.com/athletes/16889941-aletta-scholten | GET | 200 (124,943 B) | Profile exposes `Class of 2026`, PR table, PRO-gated results | 2026-09-19 23:13:57 |
| https://mi.milesplit.com/meets/779295-logger-invite-2026-2026/info | GET | 200 (41,219 B) | Meet info page is JS-loaded; no timing credit in HTML | 2026-09-19 23:14:14 |
| https://mi.milesplit.com/teams | GET | 200 (288,057 B) | 895 High-School team links, one page, no pagination | 2026-09-19 23:14:14 |
| https://mi.milesplit.com/teams/12317-boyne-falls/roster | GET | 200 (145,021 B) | Roster `<ul id="rosterDataset">` with Athlete/Gender/Class/Indoor/Outdoor/XC; 55 athletes, 4 × 2027 | 2026-09-19 23:14:29 |
| https://mi.milesplit.com/teams/12245-academy-of-oak-park-hs | GET | 200 (36,008 B) | Second team page: no coach/AD fields | 2026-09-19 23:14:30 |
| https://mi.milesplit.com/teams/12144-saline/roster | GET | 200 (1,038,448 B) | 494 athletes, 63 × 2027 (29 f / 34 m), 38 with class `0` | 2026-09-19 23:14:50 |
| https://mi.milesplit.com/teams/1228-rockford/roster | GET | 200 (736,892 B) | 345 athletes, 104 × 2027 | 2026-09-19 23:14:51 |
| https://mi.milesplit.com/athletes/11183082-landon-stukey | GET | 200 (137,185 B) | `Class of 2027`, Saline MI, public PR table (400/800/1600/3200/5K) | 2026-09-19 23:15:12 |
| https://mi.milesplit.com/timing-companies | GET | 404 | No MileSplit timing-company directory | 2026-09-19 23:15:40 |
| https://mi.milesplit.com/sitemap_index.xml | GET | 404 | No sitemap index (single sitemap) | 2026-09-19 23:18:02 |
| https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster | GET | 200 (359,834 B) | 161 athletes, 45 × 2027 (boys school) | 2026-09-19 23:18:02 |
| https://mi.milesplit.com/sitemap2.xml | GET | 404 | No second sitemap | 2026-09-19 23:18:04 |
| https://mi.milesplit.com/sitemap.xml (2nd pull) | GET | 200 (918,360 B) | Identical 7,500 entries → stable change feed | 2026-09-19 23:18:15 |
| https://mi.milesplit.com/teams?type=3 | GET | 200 (144,603 B) | 382 middle-school teams → level param works; HS list = 895 | 2026-09-19 23:18:23 |
| https://mi.milesplit.com/athletes (search page) | GET | 200 (54,655 B) | Athlete search box + top performances; not a browse directory | 2026-09-19 23:18:05 |
| https://mi.milesplit.com/teams/12275-saginaw-arthur-hill/roster | GET | 200 (29,725 B) | Empty roster (0 athletes) — team exists without data | 2026-09-19 23:18:33 |
| https://mi.milesplit.com/teams/12427-croswell-lexington/roster | GET | 200 (332,614 B) | 147 athletes, 28 × 2027 | 2026-09-19 23:18:34 |
| https://mi.milesplit.com/teams/12579-howell/roster | GET | 200 (387,338 B) | 175 athletes, 32 × 2027 | 2026-09-19 23:18:36 |
| https://mi.milesplit.com/teams/12734-negaunee/roster | GET | 200 (476,535 B) | 219 athletes, 28 × 2027 | 2026-09-19 23:18:37 |
| https://mi.milesplit.com/teams/12886-sterling-heights/roster | GET | 200 (271,814 B) | 117 athletes, 23 × 2027 | 2026-09-19 23:18:39 |
| https://mi.milesplit.com/teams/35004-kalamazoo-lakeside-academy/roster | GET | 200 (30,159 B) | Second empty roster | 2026-09-19 23:18:40 |
| https://mi.milesplit.com/teams/57151-woodhaven-brownstown/roster | GET | 200 (926,705 B) | 438 athletes, 57 × 2027 | 2026-09-19 23:18:45 |
| https://mi.milesplit.com/teams/12203-grand-haven/roster | GET | 200 (910,465 B) | 430 athletes, 58 × 2027; located Luka Hammond id 11141513 | 2026-09-19 23:19:39 |
| https://mi.milesplit.com/athletes/11141513-luka-hammond | GET | 200 (270,868 B) | `Class of 2027`, Grand Haven MI; 37 PR rows; 800 m 1:51.29 (#89 nat.), 5K 14:50.10 (#30); 104 results PRO-gated | 2026-09-19 23:19:50 |
| https://mi.milesplit.com/athletes/16811686-luka-hammond | GET | 200 → tn.milesplit.com/athletes/16811686-bennett-arant | Athlete id space is network-global; slug is not an identifier | 2026-09-19 23:19:33 |
| https://mi.milesplit.com/calendar?season=cc&year=2026&month=8 | GET | 200 (163,082 B) | 83 XC meets in Aug 2026 | 2026-09-19 23:19:56 |
| https://mi.milesplit.com/calendar?season=cc&year=2026&month=10 | GET | 200 (96,477 B) | 41 XC meets in Oct 2026 | 2026-09-19 23:19:57 |
| https://mi.milesplit.com/calendar?season=cc&year=2026&month=11 | GET | 200 (57,207 B) | 7 XC meets in Nov 2026 | 2026-09-19 23:19:59 |
| https://mi.milesplit.com/calendar?season=outdoor&year=2026&month=5 | GET | 200 (1,432,434 B) | 942 outdoor meets in May 2026 → `season=outdoor` works | 2026-09-19 23:20:01 |
| https://mi.milesplit.com/calendar?season=indoor&year=2026&month=1 | GET | 200 (107,393 B) | 40 indoor meets Jan 2026 → `season=indoor` works | 2026-09-19 23:20:10 |
| https://mi.milesplit.com/calendar?season=outdoor&year=2015&month=5 | GET | 200 (453,749 B) | 286 meets in May 2015 → ≥11 years of calendar depth | 2026-09-19 23:20:11 |
| https://www.superiortiming.com/ | GET | 200 (46,752 B) | Michigan timer; recent meets incl. MSU Spartan XC Invite; links `live.athletic.net/meets/77232,77753` and `results.superiortiming.com/meets/77216` | 2026-09-19 23:15:29 |
| https://www.superiortiming.com/category/raceresults/ | GET | 200 (77,073 B) | Static meet archive by year 2009–2026 with dates/city/discipline | 2026-09-19 23:19:03 |
| https://superiortiming.com/race-results/ | GET | 301 (0 B) | Path variant redirects; not a data path used | 2026-09-19 23:16:12 |
| https://results.superiortiming.com/meets/77216 | GET | 200 (50,184 B) | Branded results host = AthleticLIVE shell (Athletic.net infra) | 2026-09-19 23:16:25 |
| https://live.athletic.net/meets/77232 | GET | 200 (50,184 B) | AthleticLIVE shell; data behind JS (no marks in HTML) | 2026-09-19 23:16:26 |
| https://www.superiortiming.com/event-calendar/ | GET | 200 (33,853 B) | Calendar page empty ("Nothing Found") but carries recent-results links | 2026-09-19 23:16:47 |
| https://a2racemanagement.com/ | GET | 200 (65,708 B) | A2RM (Ann Arbor) nav: results by discipline/year 2011–2024, `live.a2racemanagement.com` | 2026-09-19 23:15:30 |
| https://a2racemanagement.com/results/cross-country/ | GET | 200 (50,866 B) | XC year index (2011–2024) | 2026-09-19 23:16:18 |
| https://a2racemanagement.com/results/cross-country/2024-xc/ | GET | 404 | Year slugs are irregular — must read nav hrefs | 2026-09-19 23:16:33 |
| https://a2racemanagement.com/results/track/2024-tf/ | GET | 404 | Same trap on the TF side | 2026-09-19 23:16:35 |
| https://a2racemanagement.com/results/cross-country/2024-2/ | GET | 200 (51,035 B) | 2024 XC page links Athletic.net **team 42482** (`/cross-country/2024`) | 2026-09-19 23:16:41 |
| https://a2racemanagement.com/results/track/2024-2/ | GET | 200 (51,000 B) | 2024 TF page links the same Athletic.net team id + `live.a2racemanagement.com/meet-list` | 2026-09-19 23:16:47 |
| http://live.a2racemanagement.com/meet-list | GET | 200 (50,184 B) | White-label AthleticLIVE shell (JS app; no HTML data) | 2026-09-19 23:19:03 |
| https://chttiming.com/ | GET | 200 (1,803 B) | CHT Timing, "Complete Chip Timing in Mid Michigan", track/XC live results | 2026-09-19 23:17:41 |
| https://chttiming.com/results | GET | 200 (1,714 B) | Links `anet.live/6glmne` (Athletic.net live) — results go to Athletic.net | 2026-09-19 23:19:01 |
| https://live.chttiming.com/meets/59051 | GET | 200 (50,184 B) | CHT live results = AthleticLIVE shell | 2026-09-19 23:17:40 |
| https://mitca.org/MITCA/ | GET | 200 (29,798 B) | MITCA official site (WordPress); nav incl. team-state/results/awards | 2026-09-19 23:17:07 |
| https://mitca.org/MITCA/mitca-team-state/team-state-results/ | GET | 200 (45,586 B) | 8 Athletic.net MeetIDs (284165-8, 437258, 437267, 474685, 474690), 10 PDFs, 1 sportstats link | 2026-09-19 23:17:24 |
| https://mitca.org/MITCA/results/ | GET | 404 | No top-level results page | 2026-09-19 23:17:26 |
| https://mitca.org/MITCA/michigan-meet-of-champions/ | GET | 200 (32,055 B) | 3 `.htm` result files on mitca.org + `live.chttiming.com/meets/59051` | 2026-09-19 23:17:31 |
| https://mitca.org/MITCA/champions-of-champions/ | GET | 200 (31,044 B) | 3 Athletic.net links (564539, 529506, 503468) | 2026-09-19 23:17:33 |
| https://mitca.org/MITCA/wp-content/uploads/2023/11/11-11-23MMOC-IndividualResults.htm | GET | 200 (159,195 B, text/html) | Static finisher list: O'all/Place/Score/Bib/Name/Team/1 Mile/2 Mile/Last 1./Time/Pace; "Results by CHT Timing"; club team names; no grade | 2026-09-19 23:20:16 |
| https://mitca.org/robots.txt | GET | 404 | No crawling policy published | 2026-09-19 23:19:32 |
| https://live.pttiming.com/xc-ptt.html?mid=8160 | GET | 200 (4,489 B) | Portage Invite 2025 results page is a JS shell ("Loading Data") | 2026-09-19 23:17:46 |
| https://live.pttiming.com/robots.txt | GET | 200 (3,287 B) | Preamble prohibits automated bulk extraction + AI/TDM use; licenses via legal@karmarush.com; `*` disallows `/lib/`, scoreboards, pwa | 2026-09-19 23:17:51 |
| https://live.pttiming.com/lib/xc-bundle.js?cb=09222024A | GET | 200 (947,396 B) | Fetched before reading the policy; used only to confirm the page loads data client-side; no result data extracted, no further PTTiming requests | 2026-09-19 23:17:51 |
| https://portageinvite.com/ | GET | 200 (117,644 B) | Portage Invite ("6,000+" runners) links results `mid=8160` (2025), 7465 (2024), 5836 (2023) on `live.pttiming.com` | 2026-09-19 23:16:57 |
| https://my.raceresult.com/423878/ | GET | 200 (12,501 B) | 2026 W.I.N. XC Meet (09/17/2026) on RaceResult; JS shell | 2026-09-19 23:16:57 |
| https://my.raceresult.com/423878/results | GET | 200 (12,421 B) | Results view is the same JS shell — no HTML rows | 2026-09-19 23:19:11 |
| https://my.raceresult.com/robots.txt | GET | 200 (317 B) | Disallows `/*/*/list`, `/*/*/pdf`, `/*/*/view`, certificates, users | 2026-09-19 23:19:25 |
| https://runmichigan.com/ | GET | 200 (89,049 B) | Race calendar/results site; HS content is news/video | 2026-09-19 23:15:31 |
| https://runmichigan.com/inner.php?k=hsnews | GET | 200 (116,979 B) | HS page indexes MHSAA finals media incl. `view.php?id=40079-40082` (2026 T&F finals, D4–D1) | 2026-09-19 23:19:02 |
| https://runmichigan.com/high-school/ | GET | 404 | Path guess wrong; nav uses `inner.php?k=hsnews` | 2026-09-19 23:17:32 |
| https://runmichigan.com/view.php?id=40082 | GET | 200 (69,914 B) | 2026 MHSAA LP T&F Finals D1 page is media (YouTube link), 0 result tables | 2026-09-19 23:19:19 |
| https://runmichigan.com/calendar/results.php | GET | 200 (53,012 B) | Results index is road-race oriented | 2026-09-19 23:17:10 |
| https://runmichigan.com/robots.txt | GET | 301 (285 B) | No policy at root | 2026-09-19 23:19:33 |
| https://michigancrosscountry.com/ | GET | 200 (76,092 B) | Coaches' team rankings (Week 1, Sept 14 2026) by LP division/region; "MHSAA – Athletic.net Help" nav; no athlete records | 2026-09-19 23:17:09 |
| https://michigancrosscountry.com/robots.txt | GET | 404 | No policy published | 2026-09-19 23:19:34 |
| https://www.baumspage.com/ | GET | 200 (16,620 B) | Baum's Page home page (track/XC/wrestling/golf sections) | 2026-09-19 23:15:28 |
| https://www.baumspage.com/track/index.php | GET | 200 (82,986 B) | Content is OHSAA state/district tournaments → not a Michigan source | 2026-09-19 23:16:13 |
| https://a2racemanagement.com/robots.txt | GET | 200 (121 B) | Only `/wp-admin/` disallowed; `wp-sitemap.xml` published | 2026-09-19 23:19:32 |
| https://superiortiming.com/robots.txt | GET | 301 (0 B) | No policy at root | 2026-09-19 23:19:33 |
| Probe: `everalracemgmt.com`, `mitstrack.com`, `mitca.us`, `micta.info`, `michigantrackcoaches.org`, `michiganindoortrack.com`, `mitsxc.com` | GET | DNS failure (000) | Candidate timer/MITS domains do not resolve — not sources | 2026-09-19 23:15–23:16 |
| https://micta.net/, https://micta.org/, https://mitca.net/ | GET | 200 | Unrelated sites (Spanish security vendor, empty LiteSpeed index, biomedical project) — MITCA is `mitca.org` | 2026-09-19 23:15–23:17 |
| https://www.mojeek.com/search?q=michigan+high+school+track+field+timing+company+results | GET | 202 (CAPTCHA page) | Search engine unusable; CAPTCHA not solved | 2026-09-19 23:15:00 |
| https://www.bing.com/search?q=michigan+high+school+track+timing+company+results | GET | 200 | Generic Michigan geography results only; search degraded for non-browser clients | 2026-09-19 23:15:54 |
| https://html.duckduckgo.com/html/?q=michigan+high+school+track+timing+company+results | GET | 202 (no organic results) | Search engine unusable | 2026-09-19 23:13:21 |
| `web_search` tool, queries "Michigan high school track and field timing company results" and "Michigan high school track timing company live results" | tool call | error | `Sign up and repeat your request.` (verbatim, both calls) | 2026-09-19 23:13 / 23:14 |
