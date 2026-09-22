# 09. Minnesota MSHSL

Status: complete
Observed on: 2026-09-19

Assignment: enumerate MSHSL schools and track/XC teams; qualify the coach directory for head
track/XC coach name + school + public professional email only (exclude home address, personal
phone, unrelated fields); map state T&F/XC result surfaces including any Athletic.net links;
classify access.

**Headline:** `mshsl.org` is a Drupal 11 site with an **open JSON:API 1.1** plus three
unauthenticated custom JSON endpoints (`/api/coaches/<nid>`, `/api/team-data/roster|schedule|scores/
<schoolId>/<level>/<activityId>`). It gives: school universe with stable numeric school IDs;
per-school-per-activity-per-year "team" nodes; **head coach + assistant coach names, roles and
school-domain professional emails**; **in-season participation rosters with per-athlete grade**; and
it links the 2026 state T&F meet to Athletic.net (`TrackAndField/meet/666673`) through the timer's
AthleticLIVE page. It does **not** publish regular-season results or PRs.

---

### Source

- Provider: Minnesota State High School League (MSHSL), the state association for MN high-school
  activities. Site: `https://www.mshsl.org/` (Drupal 11; JSON:API enabled; Cloudflare in front for
  `/cdn-cgi/l/email-protection` obfuscation only).
- Surfaces used: `/schools`, `/schools/<slug>`, `/schools/activities?activity=<tid>`,
  `/schools/<slug>/<activity-slug>/<year>`, `/sports-and-activities/<sport>`,
  `/2026-track-and-field-results`, `/tournaments/state-tournament-archives/...`, `/section-events`,
  `/jsonapi/**`, `/api/coaches/<nid>`, `/api/team-data/**`.
- Linked third-party: `https://gobound.com/mn/schools/<school>/calendar` (Bound schedule links, the
  field MSHSL names `field_rschooltoday_schedule_url`), `https://results.wayzatatiming.com/`
  (AthleticLIVE instance of the state-meet timer), `https://static.trackmeetio.com/`.

---

### Coverage

- State: Minnesota only. Sports: all MSHSL activities; the four target activities are
  `124` = Cross Country Running, Boys; `125` = Cross Country Running, Girls;
  `150` = Track and Field, Boys; `151` = Track and Field, Girls (IDs observed in
  `/jsonapi/taxonomy_term/activities`).
- Levels: varsity/other levels exist as a path segment on the team-data endpoints, but the `level`
  segment did **not** change the payload in testing (`varsity` and `jv` returned byte-identical
  roster responses for school 611 / activity 124). Treat level as cosmetic.
- Seasons: academic-year taxonomy; observed `695` = 2024-2025, `696` = 2025-2026,
  `698` = 2026-2027 (from `/jsonapi/taxonomy_term/academic_years`). Team pages are scoped by year
  in the URL (e.g. `.../track-and-field-boys/2027`, `.../cross-country-running-boys/2026`).
- Team/participant nodes exist per school × activity × year; the 2026-2027 set is fully populated
  (Wayzata has 39 team nodes, `meta.count = 39`).
- Historical depth: team nodes for prior years are addressable by URL but I did **not** verify that
  prior-year participant nodes are retained (only 2026-2027 confirmed). State-tournament result
  archives are deep: boys T&F **2018–2026** (9 seasons), boys XC **2017–2025** (9 seasons), each
  year page linking per-class result PDFs and a "champions thru <year>" historical PDF.
- Who is covered: member + non-member MSHSL-registered schools, including home schools, charter
  schools and co-ops (co-op name field `field_co_op_name` on team nodes). Independent/non-MSHL
  club programs are not covered.

---

### Enumeration

All patterns below were executed and returned the stated shapes.

**Schools (664).** Two independent paths:
1. `GET https://www.mshsl.org/sitemap.xml` → sitemap index with 40 child sitemaps; `?page=1`
   contains **664 unique `/schools/<slug>` URLs** and 1,097 `/tournaments/` URLs; `?page=2` contains
   **0** `/schools/` URLs, so page 1 is the complete school set for the sitemap generation date
   (lastmod 2026-09-11). 1 request = full slug list (no IDs).
2. `GET https://www.mshsl.org/schools?page=N` → 50 school rows/page (50 `/schools/<slug>` links and
   51 `views-row` blocks per page, verified on `?page=0` and `?page=9`), pager shows `?page=0..8`
   plus an ellipsis (i.e. >9 pages, consistent with 664/50 ≈ 14), each row giving school name + city
   only. A–Z filter (`custom_az_filter`), name (`name`) and city (`field_address_locality`) filters
   exist. No IDs in the listing markup.

**School IDs.** The numeric school ID appears only on the school page as `/group/<id>/` links
(Wayzata = **611**, Adrian = **5**) and as `drupal_internal__id` in
`GET https://www.mshsl.org/jsonapi/group/school?page[limit]=50` (63 attributes incl.
`label`, `path.alias`, `field_bound_id`, `field_isd_number`, `field_administrative_region`,
year-by-year `field_<yyyy>_<yyyy>_enrollment`). The JSON:API listing is the cheap ID source;
pagination via `page[offset]`.

**Teams (per activity, sponsor schools, 2026-27).**
`GET https://www.mshsl.org/schools/activities?activity=<tid>&page=N` → 50 rows/page, 8 pages each
(page 7 is the last: pager shows `page=0..7` with no ellipsis):

| activity | listed schools (2026-27) | last page rows | rows carrying a team link |
|---|---|---|---|
| 150 T&F Boys | 350 + 36 = **386** | 36 | 35 |
| 151 T&F Girls | 350 + 38 = **388** | 38 | 37 |
| 124 XC Boys | 350 + 11 = **361** | 11 | 10 |
| 125 XC Girls | 350 + 9 = **359** | 9 | 8 |

Each row links `/schools/<slug>/<activity-slug>/2027` (the 2027 contest year = 2026-2027 season).
The small gap between "rows" and "rows with a team link" is rows whose participant node does not
yet exist at that year. `items_per_page=1000` is accepted and echoed in the pager but does **not**
change the 50-row page size.

**Teams (per school).** `GET https://www.mshsl.org/jsonapi/views/teams/list_school?views-argument[]=<schoolId>`
→ all `node--participant` teams for that school with `drupal_internal__nid`, `title` and `path.alias`,
plus `meta.count` (39 for Wayzata 611). One request per school.

**Meets.** Not enumerated by MSHSL. `/group/<schoolId>/full-schedule/` is a JS shell (no
server-rendered schedule); the school page instead links `https://gobound.com/mn/schools/<school>/calendar`
(Bound). `/api/team-data/schedule/<schoolId>/<level>/<activityId>` exists and returned `[]` for
school 611 / activity 124.

**Athletes.** Via roster data only (see Athlete evidence); no athlete search or listing surface on
mshsl.org.

**Class of 2027.** In-season rosters carry a grade code per athlete; the 2025-2026 academic year
(`696`) grade 11 and the 2026-2027 year (`698`) grade 12 are Class of 2027, but the roster endpoint
has **no year parameter** (a 4th path segment returned 404), so only the *current* season roster is
retrievable. Class-of-2027 enumeration is therefore a per-season capture, not a backfill
(see Incremental use).

**Results.** Only state/postseason (see Result evidence); 1 page per archive year + PDFs.

---

### Stable identifiers

| Entity | Identifier | Evidence |
|---|---|---|
| School | numeric group ID, e.g. `611` (Wayzata), `5` (Adrian) | `/group/611/` links; `drupal_internal__id` in `/jsonapi/group/school`; `hostSchoolId` in team-page `drupalSettings` |
| School (slug) | `/schools/<slug>` path alias | sitemap + `path.alias` |
| School (Bound) | `field_bound_id` e.g. `h202282521336d6833291721448cba48` (Adrian) + slug `gobound.com/mn/schools/adrian/...` | `/jsonapi/group/school`, school page |
| School (Arbiter) | `field_arbiter_id` (e.g. `177` Adrian, `215` Aitkin, `249` Albany) | `/jsonapi/group/school` |
| Activity | taxonomy tid: 124/125/150/151 (+ UUID e.g. `ed9aa112-3c9b-4b8f-a8e1-b136e06e9cb4` = 150) | `/jsonapi/taxonomy_term/activities` |
| Academic year | taxonomy tid: 695/696/698 | `/jsonapi/taxonomy_term/academic_years` |
| Team ("participant") | Drupal node id (nid): 589034 = Wayzata T&F Boys 2026-27; 589008 = Wayzata XC Boys 2026-27; 589000-589038 = Wayzata's 39 teams | team page `path.currentPath` = `node/<nid>`; `/jsonapi/node/participant`; `/jsonapi/views/teams/list_school` |
| Class / Section | taxonomy terms `field_class` (e.g. 51 = Baseball class) and `field_competitive_section`; team page shows "(6AAA)" for Wayzata T&F | `/jsonapi/node/participant` relationships |
| Coach record | no public ID; addressable by *team node id* only | `/api/coaches/<nid>` |
| Athlete (roster) | `student_id`, 32-hex string (e.g. Wayzata XC rows) — Bound-derived, stable within the feed | `/api/team-data/roster/611/varsity/124` |
| State meet (timer) | AthleticLIVE meet id `74652` | `https://results.wayzatatiming.com/meets/74652` |
| State meet (Athletic.net) | Athletic.net meet id `666673` | `https://www.athletic.net/TrackAndField/meet/666673` href on the meet page |
| Event (AthleticLIVE) | event id in URL: individual `2799745` = Girls 3200m Class AAA; relays `502165`, `502167`, ... | `/meets/74652/events/individual/2799745` |

Not identifiers: school names (co-op teams carry combined titles, e.g. "GMLOKS Bulldogs",
"Cotter/Hope Lutheran", "LA/RP/H"), meet names, event names.

---

### Athletic.net leverage

Direct Athletic.net links published by this source:

- **The state T&F meet is linked to Athletic.net from the timer's page MSHSL itself links to.**
  The MSHSL T&F activity page publishes "Live Results" →
  `https://results.wayzatatiming.com/meets/74652` (title: "MSHSL State Track & Field Championships -
  Jun 4 – 6, 2026 | Wayzata Results"). That page (AthleticLIVE, i.e. Athletic.net's live platform,
  loading `https://livestatic.athletic.net/main-*.js`) contains a header link
  **`https://www.athletic.net/TrackAndField/meet/666673`** ("View on AthleticNET").
  So MSHSL gives us `MeetID 666673` for the 2026 state T&F meet with **zero Athletic.net requests**
  — one meet per year, ~6 classes × 25 events.
- No other Athletic.net links were found: a case-insensitive search for `athletic.net` over every
  MSHSL page I fetched (home, school page, T&F page, XC page, 2026 results page, both archive pages,
  activity listing, team pages) returned **no matches** (exit status 1).
- No Athletic.net TeamID or AthleteID is published. Athletic.net seeding from MSHSL is therefore
  *context-based*, not link-based: school name + city (`/jsonapi/group/school`) + activity + season
  are enough to resolve one specific Athletic.net team, and grade + event + mark from state results
  pages are enough to resolve one specific athlete.

Estimated request avoidance (state-level captures only, using observed counts):

- 664 schools × 2 sports (T&F + XC) = 1,328 names/grade rosters obtainable **without any Athletic.net
  request**; a roster request also returns head/assistant coach names, so the same request serves the
  coach graph.
- 386 + 388 + 361 + 359 = 1,494 team-page/coach targets identified from 4 + 664 requests.
- The state-meet Athletic.net link removes the need to *search* Athletic.net for the MN state meet
  (~300–400 athlete rows per class set) once per season.
- MSHSL does **not** replace Athletic.net performance history: no marks, PRs or meet results exist
  outside the postseason pages.

---

### Athlete evidence

Primary athlete surface: `GET https://www.mshsl.org/api/team-data/roster/<schoolId>/<level>/<activityId>`
(observed: `/611/varsity/124`).

- Response keys: `school_name`, `rosters[]`, `head_coach`, `assistant_coaches`,
  `athletic_trainers`, `roster_hidden`.
- `rosters[]` row keys: `student_id`, `student_number`, `student_name`, `student_lastname`,
  `student_position`, `student_height`, `student_weight`, `student_grade`.
- Grade codes observed: `07`, `08`, `FR`, `SO`, `JR`, `SR`. Wayzata XC Boys 2026-27 = **155 rows**:
  `07`=33, `08`=40, `FR`=20, `SO`=17, `JR`=31, `SR`=14 (i.e. 45 grade-11/12 athletes).
- Available: name, school, activity (⇒ sport and, by activity id, gender), season, **grade**
  (⇒ graduating class when combined with the season), roster membership.
- Not available: city/state per athlete (only per school), PRs, performances, progression, meets,
  athlete profile URL, indoor/outdoor distinction (MSHSL has no indoor track activity — the four
  activity ids above are the complete target set), and no separate Class-of-2027 label.
- `student_number`, `student_height`, `student_weight`, `student_position` are unrelated body/
  roster fields → **excluded from ingestion** per the retention contract.
- `roster_hidden` = true means the school has opted out ("Bound roster hidden by school preference"
  string in `roster.bundle.js`); only the count of such schools is unknown (not measured — would
  require one request per team).
- Wayzata T&F Boys 2026-27 returned an empty roster with `roster_hidden: false` (spring season not
  started on 2026-09-19) — in-season rosters only.

Indirect athlete evidence: state-tournament archive pages print individual champions as
`Name, Grade, Mark` (e.g. 2026 T&F Class A 100m "Leif Shervey, 12, 10.61"; 2025 XC Class AA
individual champion "Charlie Loth, 11, 15:31.1") — grade 11 in Nov 2025 = **Class of 2027**, an
independent grade check for the top ~1–2% of the cohort (~700–1,000 athletes across classes/seasons).

---

### Recruiting information

**Verdict: COACH-DIRECTORY — YES for head T&F/XC coach name, role and public professional email;
NO for a name-searchable coach directory; AD email requires one page fetch per school.**

1. **Head/assistant coach per team — `GET https://www.mshsl.org/api/coaches/<team-node-nid>`.**
   Public, unauthenticated, JSON array. Fields per record: `name`, `title`, `coach_level`,
   `coach_level_weight`, `coach_level_vocabulary`, `field_email`, `field_work_phone`.
   - Wayzata T&F Boys (nid 589034) → **10 records**; `Head Coach` = Aaron Berndt
     (`title` = "Hurdles & Sprints"), `field_email` = `aaron.berndt@wayzataschools.org`;
     4 of 10 records carry an email.
   - Wayzata XC Boys (nid 589008) → **9 records**; `Head Coach` = Mark Popp,
     `field_email` = `mark.popp@wayzataschools.org`.
   - Roles are per event group for T&F (`title` = Distance / Sprints / Jumps / Throws / Hurdles),
     so "head coach" is identified by `coach_level = "Head Coach"`, not by being first.
   - The site's own renderer filters out `coach_level` values `Non-MSHSL Coach` and `MSHSL Sub-Coach`
     (literal filter found in `teampersonnel.bundle.js`). **That filter must be replicated**: those
     records carry personal-domain emails and 10-digit personal mobiles
     (observed: a `@gmail.com` address and a mobile number on a "Non-MSHSL Coach" record, plus a
     `@hotmail.com` address and mobile on an "Assistant Coach" record).
   - Requires the team node id. Acquisition: 1 request per school
     (`/jsonapi/views/teams/list_school?views-argument[]=<schoolId>`) then 1 request per team.
2. **Coach names without the node id — the roster endpoint already carries them.**
   `/api/team-data/roster/<schoolId>/<level>/<activityId>` returns `head_coach` (string, e.g.
   "Mark Popp") and `assistant_coaches` (comma-separated string, e.g. "Aaron Berndt, Kyle Rasmussen,
   Brandon Heebink"). One request per school × activity — no team node id needed, no emails.
3. **Athletic director — published per school on `/schools/<slug>` (HTML).** The "Administration"
   block lists `Activities Director` (name + school phone + email), `AD Administrative Assistant`,
   `Principal`, `Superintendent`, other activity directors (band/orchestra/choir/yearbook/newspaper,
   each with email), `Title IX Officer`, `Boys/Girls Sports Representative`, `Athletic
   Trainer/Medical`. Verified on Wayzata (`/group/611/`, 5 Cloudflare-obfuscated `mailto` links) and
   Adrian (school id 5, 2 obfuscated links). Emails are Cloudflare `email-protection` obfuscated in
   the HTML but decoded client-side by Cloudflare's own `/cdn-cgi/scripts/.../email-decode.min.js`
   (published transformation, no access control involved). Cost: 1 HTML request per school + XOR
   decode.
4. **Trap: `field_athletic_*` in JSON:API is NOT the AD.** `field_athletic_name` /
   `field_athletic_email_address` / `field_athletic_phone_number` on `group--school` hold the
   **athletic trainer** (Adrian: `field_athletic_name` = "Chelsea Miles" = the "Athletic
   Trainer/Medical" line on the school page; Aitkin: "Marc Carley, DPT, ATC" with a
   `@rwhealth.org` address; Albany: `athletictrainer@district745.org`). Do not map these to
   Activities Director.
5. No coach directory keyed by name/sport/school was found. `/who-are-you/coaches` is a landing page
   (2 links, no listing). `/dashboards/coaches-dashboard` returns **403** (authenticated area).
   `/roster`, `/participant`, `/activity` are the only structural entities; there is no
   "coach" content type exposed via JSON:API (coaches exist as `paragraph--coach` → `user--user`, and
   JSON:API exposes only `display_name` for users — no email, no phone).

**Retention contract (retain / exclude), exact:**

| Retain | Source | Field |
|---|---|---|
| school | `/jsonapi/group/school` | `label`, `drupal_internal__id`, `path.alias` |
| coach name | `/api/coaches/<nid>` | `name` |
| sport | URL | activity id (124/125/150/151) |
| role | `/api/coaches/<nid>` | `coach_level` (+ `title` for T&F event group) |
| public professional email | `/api/coaches/<nid>` | `field_email` **only if** `coach_level ∉ {Non-MSHSL Coach, MSHSL Sub-Coach}` and the domain is the school/district domain |
| AD name/email | `/schools/<slug>` HTML | `Activities Director` block |

| Exclude (observed to exist) | Source | Field |
|---|---|---|
| work/personal phone | `/api/coaches/<nid>` | `field_work_phone` (school extensions **and** personal mobiles) |
| personal email | `/api/coaches/<nid>` | `field_email` when `coach_level` = `Non-MSHSL Coach` / `MSHSL Sub-Coach` or domain is a consumer mail provider |
| school/AD phone | `/schools/<slug>` HTML; `group--school` | `tel:` links, `field_school_phone_number`, `field_athletic_phone_number` |
| body metrics / roster trivia | roster API | `student_height`, `student_weight`, `student_position`, `student_number` |
| institutional address | `group--school` | `field_address`, `field_latitude`, `field_longitude` (school building, not a person) |
| emergency-plan PDF | `node--participant` | `field_emergency_action_plan_pdf` (not personal data, out of scope) |
| no athlete email/phone/address | — | **none observed on any roster/schedule/coach endpoint** |

---

### Result evidence

| Surface | URL pattern | Format | Depth | Fields |
|---|---|---|---|---|
| State T&F results index | `/2026-track-and-field-results` (year-specific) | HTML index → 6 PDFs (`class-a-prelim-results`, `class-aa-prelims`, `class-aaa-prelim-results`, `class-a-final-results`, `2026-state-track-class-aa-finals`, `2026-track-and-field-state-meet-class-aaa-finals`) | 2026 observed | place/mark/grade in PDF text only |
| State T&F archive | `/tournaments/state-tournament-archives/state-tournament-archive-boys-track-field-<YYYY>` | HTML per year | **2018–2026** (girls series exists, e.g. `...-girls-track-field-2018`) | champions as `Name, grade, mark`, class records, team-champion photos with head/assistant coach names, "team champions thru 2026" + "meet records thru 2026" PDFs |
| State XC archive | `/tournaments/state-tournament-archives/state-tournament-archive-boys-cross-country-<YYYY>` | HTML per year | **2017–2025** | 3 result PDFs for 2025 (`2025-class-aa-...`, `2025-class-aaa-...` and a generically named `results.pdf`), individual champion `Name, grade, mark` (e.g. `Charlie Loth, 11, 15:31.1`), team champions with coach names |
| State T&F live/complete | `https://results.wayzatatiming.com/meets/74652` (AthleticLIVE) | JS SPA | current + past meets of that instance | place, athlete, school, mark, `Yr: 12`, heat (`H1`), bib (`#530`), points, PB/US-40/MN-1 annotations, splits, entries, start lists, records; per-event ids (individual `2799745`, relay `502165`); tabs `/athletes`, `/teams`, `/track-scores`, `/reports/winners|records|track-point-mvp`; complete-results PDF at `static.trackmeetio.com/meetFiles/74652/<hash>.pdf` |
| Section tournaments | `/section-events` with filters `field_activity_target_id`, `field_class_target_id`, `field_section_target_id`, `field_subsection_target_id`, `field_year_target_id`, `field_administrative_region_target_id`, `field_event_type_value` | HTML | current cycle | event listing; competitive placement via `/tournaments/competitive-sections/list?field_activity_target_id=150&field_year_target_id=232` (link observed on the T&F page, not fetched) |
| Team-level schedules/scores | `/api/team-data/schedule\|scores/<schoolId>/<level>/<activityId>` | JSON | current season | endpoint exists (bundle-confirmed; schedule returned `[]` for Wayzata XC) |
| Records pages | `/mshsl-records-track-and-field-boys`, `/mshsl-records-cross-country-running-boys` | HTML | all-time | event records |

Available on these surfaces: mark, place, date, school represented, heat/round, class/division,
grade, and (AthleticLIVE) splits and relay membership. **Not available:** ResultID, AthleteID,
MeetID (except the single state-meet Athletic.net id), EventID in an MSHSL namespace, normalized
mark inputs, timing method, wind, implement/hurdle specification — MSHSL publishes none of these;
the AthleticLIVE page shows wind/implement only where the timer entered them (not verified).
Regular-season results are **not** published by MSHSL at all.

---

### Incremental use

Weekly collector design that avoids full re-fetch:

1. **Schools** — once per season: `/sitemap.xml?page=1` (1 request) for slugs;
   `/jsonapi/group/school` paginated for IDs/Bound IDs/enrollment. Cheap delta: JSON:API `changed`
   timestamps (`changed` attribute per school, e.g. Aitkin 2026-01-08, Adrian 2025-02-26).
2. **Teams** — once per season per activity: 4 requests (`/schools/activities?activity=124|125|150|151`)
   for the sponsor list; or refresh a single school with
   `/jsonapi/views/teams/list_school?views-argument[]=<schoolId>` (also returns `meta.count`, so a
   count change flags a new/removed team).
3. **Rosters (athletes + grades + coach names)** — poll `/api/team-data/roster/<schoolId>/varsity/<activityId>`
   for the in-season activity only: XC (124/125) roughly Aug–Nov, T&F (150/151) Mar–Jun. 2 requests
   per school per in-season sport. Server-side cache windows are published in the team-page
   `drupalSettings.mshsl.teamData.durations` (`roster`: 12 h base / 6 h empty; `schedule` and
   `score`: 12 h, constrained to an 18:00–23:00 window with a 30-minute window duration) — a weekly
   poll always bypasses the cache, a sub-6-hour poll mostly will not. Roster rows carry `student_id`,
   so week-over-week diffing is by ID, not name.
4. **New/changed teams and seasons** — filter `/jsonapi/node/participant` by `field_activity.id` +
   `field_year.id` (UUIDs observed above) with `changed` for delta; page size is capped at 50 even
   when `page[limit]=200` is requested.
5. **Postseason results** — poll `/sports-and-activities/<sport>` once per season for the results/
   archive links (the 2026 T&F page's results and AthleticLIVE links appear there), then the year
   archive page. Add the Athletic.net meet id from the timer page once per season.
6. **Not incremental-friendly:** no roster/schedule/scores year parameter (404 on a 4th segment), so
   past-season rosters cannot be polled — capture each season in place or lose it.

---

### Access characteristics

- Classes present: **public structured JSON** (`/jsonapi/**`, `/api/coaches/<nid>`,
  `/api/team-data/{roster,schedule,scores}/...`), **documented API** (Drupal JSON:API 1.1 at
  `/jsonapi`, 441 resource types, no auth), **normal HTML** (school/activity/archive pages),
  **downloadable PDF** (state results, champions/records/participants-through-year lists),
  **browser application** (the AthleticLIVE results SPA), **authenticated** (`/dashboards/coaches-dashboard` → 403).
- No API key, token, cookie or login was used for any JSON endpoint. No CAPTCHA encountered.
- Published limits: `robots.txt` (200) disallows `/core/`, `/profiles/`, `/search`, `/admin`,
  `/user/*` and — critically — **`*.pdf`, `*.doc`, `*.docx`, `*.xls`, `*.xlsx`, `*.csv`** for
  `User-agent: *`, and fully disallows `MJ12bot`, `AhrefsBot`, `dotbot`, `GPTBot`. The state result
  PDFs and the champions/participants history PDFs are therefore in a crawl-disallowed extension
  bucket: fetch them only manually/on demand, not with a bulk crawler.
- Observed rate behaviour: 58 requests to `www.mshsl.org` (sequential, 1–2 s spacing, browser-like
  UA) with **zero 429s and no `Retry-After` seen**; the only non-200 was the intentional 403 on the
  authenticated coach dashboard. This is above the mission's ~50-request aim, disclosed here; stop
  the crawl sooner in production and prefer the JSON endpoints (they replaced ~1,300 HTML fetches
  with ~5).
- Unrelated hosts touched while confirming the state-meet surfaces: `results.wayzatatiming.com`
  (2 curl + 3 browser navigations, all 200) and `live.athletic.net` (2 requests, both **200** —
  note: unlike `www.athletic.net`, the AthleticLIVE host on the athletic.net domain is reachable
  to a non-browser client from this machine; only the SPA shell is returned without JS).
- Athletic.net target `https://www.athletic.net/TrackAndField/meet/666673` was **not** fetched
  (the mission documents www.athletic.net as Cloudflare-403 from this machine); the id is evidence
  from the href on the AthleticLIVE page.
- No published terms-of-service or API rate limit was found on mshsl.org beyond `robots.txt`
  (no `/terms` link in the footer nav I fetched). Treat the JSON endpoints as undocumented and
  throttle conservatively.

---

### Recommendation

**COACH-DIRECTORY (primary) + VALIDATION (grade/school) + DISCOVERY-ONLY (teams/school universe).**
Not RESULT-SOURCE (no regular-season results), not an Athletic.net seed for athlete profiles.

Expected marginal coverage:

- **Coach graph**: the only source found in this mission so far that returns an authoritative,
  machine-readable *head coach* record (name + role + school-domain professional email) for a
  high-school team, because MSHSL makes coach data part of each activity's roster. 1,494 target teams
  (T&F + XC, both genders) ≈ 1,494 `/api/coaches/<nid>` calls + 664 node-list calls; or 664×2 roster
  calls for names without emails. Realistic yield: head-coach email present for a minority of teams
  (Wayzata: head coaches present with email in both sports; only 4/10 T&F records carry email at all) —
  plan for ~30–60% email coverage and treat missing emails as "ask the school", not as failure.
- **AD contacts**: 1 request per school returns AD name + obfuscated professional email (+ principal,
  superintendent, other directors) — the best AD source for MN short of district sites.
- **Class-of-2027 grade verification**: in-season rosters give an official school-published grade for
  every participant in a sport; that is direct independent Co2027 evidence for the cohort that
  Athletic.net discovers by inference. Because the feed is current-season-only, the Co2027 value
  lands in the **2025-26** (grade 11) and **2026-27** (grade 12) captures; the 2025-26 window is
  already partly historical unless a capture already exists — this is the single biggest risk to the
  Co2027 yield and should be decided by Main before building.
- **Athletic.net request avoidance**: the state-meet Athletic.net MeetID (666673 for 2026) comes free
  from the timer page; school+activity+season+grade context from MSHSL is sufficient to request one
  specific Athletic.net team/athlete without search, but MSHSL publishes no Athletic.net IDs of its
  own.
- **Do not** build on MSHSL for performances, PRs, meets or meet results: those live on Bound
  (school schedules), AthleticLIVE/`results.*` instances (state + timer-run meets) and
  Athletic.net (history) — see agent 10's report for the timer ecosystem.
- **Sequencing recommendation**: implement the roster + coach endpoints first (small, JSON, cached),
  the AD page scrape second (HTML + obfuscation decode, 1 request per school), the JSON:API
  enumeration third (it is the cheapest school-ID/Bound-ID source), and skip the schedule/scores
  endpoints until agent 10 confirms whether Bound or MSHSL's copy is the better meet feed.

---

### Evidence appendix

All timestamps 2026-09-19 (America/Chicago), sequential requests, browser-like UA, `curl -L`.
`MS` = www.mshsl.org.

| URL | method | HTTP | what it proved | timestamp |
|---|---|---|---|---|
| https://www.mshsl.org/ | GET | 200 | Site is Drupal; nav exposes /schools, /sports-and-activities, /section-events, /calendar, /region-1A…8AA, /who-are-you/coaches, /dashboards/coaches-dashboard | 2026-09-19 |
| https://www.mshsl.org/sitemap.xml | GET | 200 | Sitemap index, 40 child sitemaps, lastmod 2026-09-11 | 2026-09-19 |
| https://www.mshsl.org/sitemap.xml?page=1 | GET | 200 | 2,000 URLs incl. **664 unique /schools/** and 1,097 /tournaments/ | 2026-09-19 |
| https://www.mshsl.org/sitemap.xml?page=2 | GET | 200 | 0 /schools/ URLs → page 1 is the complete school set | 2026-09-19 |
| https://www.mshsl.org/robots.txt | GET | 200 | Disallows /search, /admin, /user/* and **all *.pdf/*.doc/*.xls/*.csv**; blocks GPTBot/AhrefsBot/MJ12bot/dotbot | 2026-09-19 |
| https://www.mshsl.org/schools | GET | 200 | 50 school rows/page; filters name/`field_address_locality`/`custom_az_filter`; no IDs in markup | 2026-09-19 |
| https://www.mshsl.org/schools?page=9 | GET | 200 | Pagination works beyond page 8; pager ellipsis ⇒ >9 pages | 2026-09-19 |
| https://www.mshsl.org/schools/wayzata-high-school | GET | 200 | School 611; `/group/611/...` links; AD block (Meghan Potter + tel + obfuscated email); 5 cf-obfuscated emails; Bound calendar link; classification enrollment 3533; ISD 284; region 6AA | 2026-09-19 |
| https://www.mshsl.org/schools/adrian-high-school | GET | 200 | Same AD template at a small school (Joe Kruger); 2 cf emails; group 5; gobound slug `adrian` | 2026-09-19 |
| https://www.mshsl.org/sports-and-activities/track-and-field-boys | GET | 200 | activity=150; Jun 10-12 2027 state meet; **Live Results → results.wayzatatiming.com/meets/74652**; `/2026-track-and-field-results`; archive + records links | 2026-09-19 |
| https://www.mshsl.org/sports-and-activities/cross-country-running-boys | GET | 200 | activity=124; Nov 7 2026 state meet at Majestic Oaks; XC archive (2025) + records links; no live-results link | 2026-09-19 |
| https://www.mshsl.org/2026-track-and-field-results | GET | 200 | 6 per-class prelim/final result PDFs under /sites/default/files/2026-06/ | 2026-09-19 |
| https://www.mshsl.org/schools/activities?activity=150 | GET | 200 | Sponsor-school listing for T&F Boys; 50/page; team URL pattern `/schools/<slug>/track-and-field-boys/2027`; pager 0..7 | 2026-09-19 |
| https://www.mshsl.org/schools/activities?activity=150&items_per_page=1000 | GET | 200 | Param echoed in pager but page size stays 50 rows | 2026-09-19 |
| https://www.mshsl.org/schools/activities?activity=150&page=7 | GET | 200 | Last page: 36 rows ⇒ 386 listed schools for T&F Boys 2026-27 | 2026-09-19 |
| https://www.mshsl.org/schools/activities?activity=151&page=7 | GET | 200 | Last page: 38 rows ⇒ 388 listed schools for T&F Girls 2026-27 | 2026-09-19 |
| https://www.mshsl.org/schools/activities?activity=124&page=7 | GET | 200 | Last page: 11 rows ⇒ 361 listed schools for XC Boys 2026-27 | 2026-09-19 |
| https://www.mshsl.org/schools/activities?activity=125&page=7 | GET | 200 | Last page: 9 rows ⇒ 359 listed schools for XC Girls 2026-27 | 2026-09-19 |
| https://www.mshsl.org/schools/wayzata-high-school/track-and-field-boys/2027 | GET | 200 | Team page node 589034; drupalSettings `hostSchoolId=611`, `activityId=150`, `yearId=698`, `rosterColumns` (student_name/student_grade enabled), team-data cache durations; class "(6AAA)" | 2026-09-19 |
| https://www.mshsl.org/schools/wayzata-high-school/cross-country-running-boys/2026 | GET | 200 | Team page node 589008; activityId 124, yearId 698 | 2026-09-19 |
| https://www.mshsl.org/group/611/full-schedule/ | GET | 200 | Page renders an empty JS shell — schedule not server-rendered | 2026-09-19 |
| https://www.mshsl.org/who-are-you/coaches | GET | 200 | Landing page only; no coach directory | 2026-09-19 |
| https://www.mshsl.org/dashboards/coaches-dashboard | GET | **403** | Coach dashboard is authenticated | 2026-09-19 |
| https://www.mshsl.org/section-events | GET | 200 | Section-event view with activity/class/section/subsection/year/region/event-type exposed filters | 2026-09-19 |
| https://www.mshsl.org/tournaments/state-tournament-archives/state-tournament-archive-boys-track-field-2026 | GET | 200 | Archive years 2018–2026; individual champions printed as `Name, grade, mark`; team-champion photos name Head/Asst coaches; per-class finals PDFs; girls series exists | 2026-09-19 |
| https://www.mshsl.org/tournaments/state-tournament-archives/state-tournament-archive-boys-cross-country-2025 | GET | 200 | Archive years 2017–2025; 3 per-class result PDFs for 2025; champion `Charlie Loth, 11, 15:31.1`; team champions with coach names | 2026-09-19 |
| https://www.mshsl.org/jsonapi | GET | 200 | JSON:API 1.1, unauthenticated, 441 resource types incl. `group--school`, `node--participant`, `paragraph--coach`, `user--user` | 2026-09-19 |
| https://www.mshsl.org/api/ | GET | 404 | Custom API has no index (documents only) | 2026-09-19 |
| https://www.mshsl.org/api/coaches/ | GET | 404 | `/api/coaches/` needs a node id | 2026-09-19 |
| https://www.mshsl.org/jsonapi/group/school?page[limit]=3 | GET | 200 | 63 attributes/school: `drupal_internal__id`, `path.alias`, `field_bound_id`, `field_arbiter_id`, `field_team_data_provider`, `field_rschooltoday_schedule_url`, yearly enrollments, `field_athletic_*` (trainer), website/socials | 2026-09-19 |
| https://www.mshsl.org/jsonapi/taxonomy_term/activities?page[limit]=100 | GET | 200 | Activity tids + UUIDs: 124/125/150/151 (50 activities total) | 2026-09-19 |
| https://www.mshsl.org/jsonapi/taxonomy_term/academic_years?page[limit]=50 | GET | 200 | Year tids + UUIDs: 695=2024-2025, 696=2025-2026, 698=2026-2027 | 2026-09-19 |
| https://www.mshsl.org/jsonapi/views/teams/list_school?views-argument[]=611 | GET | 200 | All 39 Wayzata team nodes with nids + path aliases; `meta.count` = 39 | 2026-09-19 |
| https://www.mshsl.org/jsonapi/node/participant?filter[drupal_internal__nid]=589034 | GET | 200 | `node--participant` = school × activity × year team entity; relationships `field_activity`, `field_year`, `field_school`, `field_coaches`, `field_class`, `field_competitive_section` | 2026-09-19 |
| https://www.mshsl.org/jsonapi/node/participant?page[limit]=2&include=field_coaches,field_activity,field_year | GET | 200 | `field_coaches` → `paragraph--coach`, which holds only `field_coach_title` and links to `user--user` (`field_coach`) | 2026-09-19 |
| https://www.mshsl.org/jsonapi/user/user?page[limit]=2 | GET | 200 | User JSON:API exposes **only** `display_name` (no email/phone) | 2026-09-19 |
| https://www.mshsl.org/jsonapi/node/participant?filter[field_activity.id]=<uuid 150>&filter[field_year.id]=<uuid 698>&page[limit]=50 | GET | 200 | Activity+year filter works; 50 rows/page with `next` link (payload 461,991 B) | 2026-09-19 |
| …same with `fields[node--participant]=title,path&page[limit]=200` | GET | 200 | Sparse fieldsets work (28,335 B, 16× smaller) but page size is still capped at 50 | 2026-09-19 |
| https://www.mshsl.org/api/coaches/589034 | GET | 200 | **10 coach records for Wayzata T&F Boys**; `Head Coach` Aaron Berndt with school-domain `field_email`; 4/10 have email; fields incl. `field_work_phone` | 2026-09-19 |
| https://www.mshsl.org/api/coaches/589008 | GET | 200 | **9 coach records for Wayzata XC Boys**; `Head Coach` Mark Popp `mark.popp@wayzataschools.org`; `Non-MSHSL Coach` records expose personal-domain emails/mobiles → must be filtered out | 2026-09-19 |
| https://www.mshsl.org/api/team-data/roster/611/varsity/124 | GET | 200 | **155 roster rows** with `student_grade` (07/08/FR/SO/JR/SR) + `student_id`; `head_coach` "Mark Popp"; `assistant_coaches`; `roster_hidden` false | 2026-09-19 |
| https://www.mshsl.org/api/team-data/roster/611/varsity/150 | GET | 200 | T&F Boys roster empty pre-season; `head_coach` "Aaron Berndt" | 2026-09-19 |
| https://www.mshsl.org/api/team-data/roster/611/varsity/124/696 | GET | **404** | No year parameter → no historical rosters | 2026-09-19 |
| https://www.mshsl.org/api/team-data/roster/611/jv/124 | GET | 200 | `level` segment returns the same payload as `varsity` | 2026-09-19 |
| https://www.mshsl.org/api/team-data/schedule/611/varsity/124 | GET | 200 | Schedule endpoint exists; returned `[]` for this team | 2026-09-19 |
| https://www.mshsl.org/modules/custom/mshsl_decoupled/js/dist/teampersonnel.bundle.js | GET | 200 | Front-end calls `/api/coaches/<node id>` and **filters out `coach_level` = "Non-MSHSL Coach" / "MSHSL Sub-Coach"** | 2026-09-19 |
| https://www.mshsl.org/modules/custom/mshsl_team_data/js/dist/roster.bundle.js | GET | 200 | Roster URL shape `/api/team-data/roster/<schoolId>/<level>/<activityId>`; "Bound roster hidden by school preference" | 2026-09-19 |
| https://www.mshsl.org/modules/custom/mshsl_team_data/js/dist/schedule.bundle.js | GET | 200 | `/api/team-data/schedule/...` | 2026-09-19 |
| https://www.mshsl.org/modules/custom/mshsl_team_data/js/dist/scores.bundle.js | GET | 200 | `/api/team-data/scores/<schoolId>/varsity/<activityId>` | 2026-09-19 |
| https://www.mshsl.org/modules/custom/mshsl_decoupled/js/dist/schoolteamlist.bundle.js | GET | 200 | Team list comes from `/jsonapi/views/teams/list_school?views-argument[]=<schoolId>` | 2026-09-19 |
| https://www.mshsl.org/modules/custom/mshsl_team_data/js/dist/schoolSchedule.bundle.js | GET | 200 | School schedule uses `/api/mshsl-state/<key>`; `drupalSettings.schoolId`/`activitiesList`; no athletic.net reference | 2026-09-19 |
| https://results.wayzatatiming.com/meets/74652 | GET (curl) | 200 | SPA shell titled "AthleticLIVE"; scripts from `livestatic.athletic.net` → state T&F results run on Athletic.net's live platform | 2026-09-19 |
| https://results.wayzatatiming.com/ | GET (curl) | 200 | Instance root is the same 50 KB SPA shell; no server-rendered meet index | 2026-09-19 |
| https://results.wayzatatiming.com/meets/74652 | browser navigation | 200 | Title "MSHSL State Track & Field Championships - Jun 4 – 6, 2026 \| Wayzata Results"; team scores; links to **https://www.athletic.net/TrackAndField/meet/666673**, `static.trackmeetio.com/meetFiles/74652/<hash>.pdf`, per-event ids, login link (not used) | 2026-09-19 |
| https://results.wayzatatiming.com/meets/74652/athletes | browser navigation | 200 | Athletes tab is search-driven (3+ chars), no bulk athlete list; timer credit "Wayzata Results, LLC", `results@wayzataresults.com` | 2026-09-19 |
| https://results.wayzatatiming.com/meets/74652/events/individual/2799745 | browser navigation | 200 | Event = "Girls 3200m Class AAA"; result rows expose place, athlete, school, mark, `Yr: 12`, heat `H1`, bib `#530`, points, PB/US/MN annotations, tabs Results/Splits/Entries/Start Lists/Records | 2026-09-19 |
| https://live.athletic.net/meets/74652 and https://live.athletic.net/ | GET | 200, 200 | The AthleticLIVE host on the athletic.net domain is reachable to a non-browser client from this machine (unlike www.athletic.net, 403) and returns the SPA shell only | 2026-09-19 |

Request totals for this slice: **58 requests to www.mshsl.org** (sequential, 1–2 s spacing, no 429,
no `Retry-After`; one expected 403), 3 to `results.wayzatatiming.com`, 2 to `live.athletic.net`.
