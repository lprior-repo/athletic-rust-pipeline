# 24. Nebraska NSAA (Nebraska School Activities Association)

Status: complete
Observed on: 2026-09-19

Session window: 2026-09-19 22:45–23:20 CDT (UTC 2026-09-20 03:45–04:18Z). All fetches were sequential; total
≈12 requests to `nsaahome.org`/`secure.nsaahome.org`, ≈20 to the NSAA S3 bucket, ≈7 to onlineraceresults.com,
≈9 to maxpreps.com, ≈5 to blacksquirreltiming.com/anet.live, 1 to precisionraceresults.com. No 429 and no
`Retry-After` was ever returned.

**Tooling note (verbatim failure):** the `web_search` tool returned `Sign up and repeat your request.` on both
attempts (queries: "Nebraska School Activities Association NSAA official website member schools" and
"nsaahome.org Nebraska School Activities Association"). Search was therefore not used; the domain was identified
by direct probing and everything below is from direct HTTP fetches recorded in the appendix.

---

### Source

**Nebraska School Activities Association (NSAA)** — the single sanctioning body for NE high-school activities.
Three distinct hosts serve public data, and knowing which host serves what is the whole trick:

| Host | Role | Behaviour from this machine |
|---|---|---|
| `https://nsaahome.org/` | Current public site (WordPress + Elementor). Sport pages, district assignments, PDF links. | **403 to curl** (Cloudflare block page, 4,546 bytes) for both `https://` and `http://`, and for `/textfile/*.pdf` paths; **200** via the harness reader fetch (`read` tool, "native" method). This is a finding, not a dead end. |
| `https://secure.nsaahome.org/` | Legacy PHP application host behind the current site: `/nsaaforms/*`, `/distassign.php`, `/about/*`. | **200 to plain curl**, GET and POST, no cookies needed. |
| `https://nsaa-static.s3.amazonaws.com/` (and the alias `nsaa-static.s3.dualstack.us-east-1.amazonaws.com`) | **Public, unauthenticated S3 mirror of the entire `/textfile/**` corpus**: rulebooks, classifications, results PDFs, results HTML, state-meet programs. | **200 to plain curl**, `Last-Modified` + `ETag` present → conditional GETs work. |

`nsaa-state.org` does not resolve (curl exit 6, status `000`); `nebraskaschools.org`, `nebraskansaa.org`,
`nsaa.home.org`, `nebraskaschoolactivities.org` also fail to resolve. `nsaahome.org` **is** the current NSAA
domain (canonical `https://nsaahome.org/`, `og:site_name` = "Nebraska School Activities Association").

Championship/result infrastructure referenced by NSAA pages (provider map, details in Result evidence):
`athletic.net` (TF district meets), `results.blacksquirreltiming.com` / `anet.live` (AthleticLIVE instance used
for state TF live results; Black Squirrel Timing), static Hy-Tek PDFs on NSAA's own site (state TF results),
`onlineraceresults.com` + `precisionraceresults.com` (state XC results, 2008–2025), `maxpreps.com` (NSAA's
official partner; XC district-seeding feeds and a scores widget).

### Coverage

* State: Nebraska only. This is the NSAA's complete jurisdiction.
* Sports: all NSAA activities. Of interest here: **Track & Field (Boys/Girls)** and **Cross Country (Boys/Girls)**;
  also Unified Track & Field. No indoor track championship exists in NE (NSAA season is spring outdoor +
  fall XC); the sport-page URL `https://nsaahome.org/track-field/` exposes only outdoor.
* Levels: high school (grades 9–12) plus an NSAA Middle Level program (separate bylaws, no results surface found).
* File/coverage depth:
  * Member directory: **312 entries** live as of 2026-09-19, each with AD, per-sport head coaches, enrollment,
    conference, NSAA governance district, address, phone, fax, mascot, colors, school homepage.
  * Enrollment figures: 2026-27 (`2627enroll/boys/girls.pdf`) back to 2002-03.
  * TF classifications + state results: state results PDFs 1900s→2026 (link grid on `/track-field/`),
    all-class gold medal history, state records (`/nsaaforms/tr/staterecords.php`).
  * XC classifications + state results: 1960→2025 (boys), 1980→2025 (girls); plus district result HTML for the
    most recent season.
  * District assignments: current year only, in-season.
* Regular-season meets: **not covered by NSAA**. NSAA publishes only postseason (districts + state). Regular
  season results live on Athletic.net (TF) and MaxPreps/Athletic.net (XC). NSAA's own XC seeding rankings are
  MaxPreps-derived, confirming MaxPreps as the de-facto regular-season XC aggregator for NE.

### Enumeration

Counts below are exact, from the fetches in the appendix (not estimates).

**Schools — 312 directory entries. 1 request.**
```
POST https://secure.nsaahome.org/nsaaforms/direxportscreen.php
     session=  school=View all schools  submit=See School Info
  → 200, 1,085,584 bytes, one HTML block per school:
       <h1 class="mt-3">SCHOOL NAME</h1>
       <div class="col-sm border-right">address / city,NE zip / phone / Fax</div>
       <div class="col-sm border-right">Conference / NSAA District / Enrollment</div>
       <div class="col-sm">Mascot / Colors / Homepage</div>
       <table> <tr><td scope='row'>TITLE</td><td scope='row'>NAME</td></tr> ... </table>
       rows carrying class="table-info" = co-op sport (blue background per the page's own legend)
```
* Single school: `GET https://secure.nsaahome.org/nsaaforms/direxportscreen.php?session=&school=Adams%20Central`
  → 200, 15,579 bytes, identical field set (rung 12× smaller, so bulk POST is the right primitive).
* Caveat: the 312 entries include **4 district-level pseudo-entries** (`Bellevue Public Schools`,
  `Lincoln Public Schools`, `Millard Public Schools`, `Omaha Public Schools`) that are not schools. Use the
  classification lists below as the participation universe and the directory as the metadata/coach authority.
* By sport (13,937-byte/111,269-byte responses): `POST .../direxportscreensport.php  sport=tr_b|tr_g|cc_b|cc_g`
  → `School | School Website | Coach` — one row per school fielding that sport (used as a second, cheap
  participation check; sizes: tr_b 111,269 B, cc_b 97,874 B at request time).

**Participation universe (classification lists on the S3 mirror).**
* 2026 TF classifications `textfile/track/trbclass.pdf` and `trgclass.pdf`: **283 entries each** for boys and
  girls, classes A=32, B=60, C=88, D=103. Rows carry a state-wide enrollment rank, class rank, school name,
  and (for co-ops) the member enrollments, e.g. `C 56 Clarkson(60) / Leigh(51) 111`,
  `A 8 Creighton Preparatory School(802x2) 1604`.
* 2026 XC classifications `textfile/cc/ccclassifications.pdf` (posted 2026-08-03): **253 entries**,
  A=35, B=26, C=59, D=133. Thresholds printed in the file: Class A ≥850, Class B 849–320, Class C 319–125,
  remainder Class D. 4 rows have names wrapped across two lines (e.g. B#18 `Nebraska City (and Lourdes Central
  Catholic and Johnson-Brock)`) and are the only rows a simple line parser misses.
* Exact-name join directory→TF classification: **260/312**; directory→XC classification: **230/312**. The
  misses are co-op/compound names (`Bloomfield-Wausa`, `H&H = Heartland(76) / Hampton(29)`, `EMF`, `BDS`,
  `South Loup = Callaway / Arnold`, …) plus the 4 district pseudo-entries. **A co-op alias map is mandatory**
  for canonical school identity in NE.

**Teams:** the classification entries *are* the team universe (283 TF, 253 XC); no separate team roster exists.

**Meets:** only postseason is enumerable from NSAA.
* TF: the sport page lists the 2026 district meets as Athletic.net links — 28 meets, IDs in Stable identifiers.
* XC: `/distassign.php?sport=ccb|ccg` names the 20 districts (A-1..A-4, B-1..B-4, C-1..C-5, D-1..D-6) with
  date, site, host school, and director, and lists the C/D member schools per district;
  `textfile/cc/26Districts.pdf` gives the same 20 districts with girls/boys start times;
  `textfile/cc/26BDist.pdf` lists the B-1/B-2 vs B-3/B-4 team split (14 + 12).
  Two state meets per year (XC Oct 23 2026 in Kearney; TF May 2026 at Burke Stadium, Omaha).
* Regular season: not enumerable from NSAA. The MaxPreps feed is the only NSAA-endorsed regular-season surface
  (it names meets that have been entered: `Platte River Rumble - Large Class`, `Class of the Metro`,
  `Fremont Twilight`, `Kearney Invite`, `John Votta Invitational at Norris High School`, …).

**Athletes / Class of 2027:** not enumerable as a list. Athletes appear only as rows inside district/state
results (see Athlete evidence). ~100 finishers per class-gender race at state XC (Class A boys 2025 plain text
= 96 rows) and top-15 per district in NSAA's district HTML. **No grade filter exists** — grade must be read from
the results themselves.

**Results:** yes, structurally: (a) full state XC results via onlineraceresults.com (2008–2025 detected);
(b) full state TF results via Hy-Tek PDFs (1900s–2026, complete files from 2012 onward as HTML/PDF);
(c) XC district results as 8 static HTML files (top-15 + team scores); (d) TF district results link out to
Athletic.net (NSAA hosts nothing for regular-season or district TF beyond the link).

### Stable identifiers

**NSAA exposes no numeric school, team, athlete, meet, or result IDs in any public surface we could reach.**
Verified: the directory `<option>` elements carry no `value` attribute (name strings only); school pages show
name/address/metadata but no id; `distassign.php` lists school names as free text; classification PDFs carry
only *enrollment rank* and *class rank* positions (re-issued each classification cycle, so **not** stable IDs).
Identity in NE is therefore: **school name string**, plus **co-op composition** (the co-op name is the identity
used in results, e.g. `Bloomfield-Wausa`, `Tri County Northeast (TCNE)`, `H&H`, `EMF`, `BDS`).
Associated keys that *are* stable/usable:
* **NSAA governance district** (1–6) and **conference** name (e.g. `Central Conference`, `Niobrara Valley
  Conference`, `Lewis & Clark`) — directory fields.
* **Class** (A/B/C/D) per sport per season, with per-season validity (2026 TF = 2nd year of a two-year cycle;
  2026-27 is the 1st year of the XC two-year cycle).
* **External, exact IDs captured this session:**
  * Athletic.net TF district meet IDs (2026): A1 `645702`, A2 `645705`, A3 `645706`, A4 `645708`;
    B1 `645711`, B2 `662930`, B3 `645715`, B4 `645719`, B5 `645720`, B6 `645723`;
    C1 `645725`, C2 `645727`, C3 `645729`, C4 `645732`, C5 `645734`, C6 `645736`, C7 `645738`,
    C8 `645739`, C9 `645741`; D1 `645742`, D2 `645745`, D3 `645746`, D4 `645749`, D5 `645750`,
    D6 `645752`, D7 `645754`, D8 `645755`, D9 `645756` → 28 IDs, all of the form
    `https://www.athletic.net/TrackAndField/meet/<MeetID>/results/all`.
  * AthleticLIVE (Athletic.net live platform) meet ids for the 2026 state TF championships: **72610**
    (Class A&B, short link `bst.anet.live/oolpf1`) and **72612** (Class C&D, `bst.anet.live/aetvft`) →
    `https://results.blacksquirreltiming.com/meets/<id>`. The short links are on the NSAA sport page, so NSAA
    itself publishes these ids.
  * Online Race Results: `event_id` + 8 `race_id`s per season — 2025 state XC `event_id=25737`,
    races `79870–79877`; 2024 `event_id=25397`, races `78662–78669`; 2020 `event_id=23643`,
    races `72432–72439`; group `group_id=33` = "NSAA Events" (pagination-free index, 2008–2025).
  * MaxPreps: NSAA association id `0953b16f-fa82-4c44-a8a8-b27f7f1d6d11` (scores widget) and the public XC feed
    key `d2c04f01-4e70-4e85-aa67-e39455f00c65`.
  * Hy-Tek event numbers inside the state TF PDFs (e.g. `Event 102 Boys Long Jump CLASS A`).

### Athletic.net leverage

* **Meet links: yes, directly, for TF districts.** The NSAA `/track-field/` page is the cheapest known source of
  NE postseason Athletic.net MeetIDs: **28 meets from one page fetch**, labeled by district and site
  (Class A/B/C/D), e.g. `B2 – Elkhorn High School → meet/662930`.
* **Meet links for XC: no.** XC districts point at NSAA-hosted HTML (and the state meet at
  onlineraceresults.com / a PDF). No Athletic.net meet link was found on the XC page.
* **Team links: no.** NSAA never links `athletic.net/team/...`; school links point at school websites.
* **Athlete links: no.** No Athletic.net athlete URLs anywhere on the NSAA pages examined.
* **State TF on Athletic.net infrastructure: yes (live only).** State championships are run live on an
  AthleticLIVE instance (`results.blacksquirreltiming.com`, `meta` favicon/static assets served from
  `livestatic.athletic.net`), reachable from NSAA's short links → two meet ids per state meet.
* **Deterministic seeding: yes, strong.** School name + class + enrollment + city from NSAA give an unambiguous
  Athletic.net team lookup key for ~283 TF and ~253 XC schools; meet ids for the 28 TF district meets are given
  outright; the state TF meet is identifiable by its AthleticLIVE id; state XC is identifiable by
  onlineraceresults event/race ids and by name+date.

**Requests avoided (estimates, [INFERENCE] — measured request counts are from the appendix):**

| Job | NSAA cost | Athletic.net alternative avoided | Avoided requests |
|---|---|---|---|
| NE school/team universe + class + enrollment | 1 directory POST + 2 classification PDFs (+2 more for girls/enrollment) ≈ **4–6** | Per-school team search/resolve at ~1–3 requests × 283 TF schools | **≈280–850** |
| TF district meet id resolution | 1 sport-page fetch = **28 MeetIDs** | Meet search + resolve at 1–3 requests/meet | **≈28–84** |
| Coach/AD names for 312 schools | 1 directory POST (**0 extra**) | No Athletic.net equivalent (Athletic.net does not publish coach names/emails) | n/a — **new capability**, 2,000+ person-role pairs |
| State XC results incl. grade | 1 event page + 8 race fetches = **9** | Meet results on Athletic.net ≈ 8 meet fetches (+ Bio fetches for grade) | ~8–24 (comparable, not superior) — value is **independent verification + grade** |

### Athlete evidence

| Field | Availability | Source / caveat |
|---|---|---|
| Name | Yes | XC district HTML `Name (grade)`; state XC plain text `Name Year`; state TF Hy-Tek PDF `Name Year` |
| Graduating class / grade | **Yes, in results** — grade printed next to every athlete | `J'Shawn Afuh (11)` in `ccbClassAResults.html`; `Aiden Gehring 10` in state XC plain text; `Leo Cauble 11` in state TF PDF. Arithmetic: grade is as-of that meet's school year, so Co2027 = grade 11 in the 2025-26 season, i.e. grade 12 in the 2026-27 season; the 2026 XC district/state files (Oct/Nov 2026) will mark Co2027 as (12). No standalone class list exists. |
| School | Yes | All result formats print school/team |
| City/state | Yes for the school (directory `City, NE zip`); not per athlete | Join by school name |
| Gender/category | Yes — separate boys/girls races and files | Class A/B/C/D × Boys/Girls |
| TF vs XC distinction | Yes — different sports, files and seasons | |
| Indoor/outdoor | Outdoor only (NE has no NSAA indoor championship) | |
| Performances | Yes for postseason; regular season only via MaxPreps/Athletic.net | State XC plain text has 1m/2m rank+split, time, pace; TF PDF has full attempt series with per-attempt wind |
| PRs / progression | No | Requires joining multiple seasons (doable: 2008–2025 state XC, 2012–2026 state TF) |
| Meets | Postseason only (districts + state) | Regular-season meet names appear in the MaxPreps XC feed |
| Athlete profile URL | **No** | No NSAA athlete pages |
| MaxPreps XC feed extra | Per school: seeding time + for each reported meet the meet name + top-5 athlete names/times (no grade) | 2026-09-19 state: boys A 4 schools/63 athletes/11 meet-slots; boys B 16/138/12; girls A 4/67/10; girls B 15/129/9. Updated hourly per the feed's own text. |

**Privacy:** NSAA results contain athlete name, school, grade, marks — no athlete contact data. Nothing in this
source requires or contains athlete email/phone/address.

### Recruiting information

**Directory verdict: 312 schools, head coaches only, names but no emails.**

* Head **Track & Field coach** present for 298 (boys) / 300 (girls) of 312 entries; after removing co-op
  placeholders (`Jamee Smith (Co-op w/Litchfield)`, `Jenna Landgren Co-op w/Wheeler Central`, …) the usable
  person-name counts are **274 boys / 275 girls**.
* Head **Cross Country coach** present for 270 / 270 entries; usable person names **243 boys / 240 girls**.
  39 entries carry no XC coach row at all (schools that do not sponsor XC — e.g. `Elba`, `Giltner`, `Hampton`,
  plus the 4 district pseudo-entries).
* **Athletic Director** (or Activities Director) name for **311/312**; 10 entries have no TF coach row
  (non-sponsoring or pseudo-entries).
* **Assistant coaches: not exposed** — exactly one head per sport/activity.
* **Public professional email: NOT exposed.** Scanning all 312 schools × 42 role titles yields **zero** email
  addresses in coach/AD fields; one stray value appears in a `Principal` field for Bishop Neumann
  (`Bridget-doyle@cdolinc.net`). NE coach email therefore must come from school/district sites (Agent 29's
  graph), not from NSAA.
* **School athletics/school website: 312/312** (`Homepage` field, e.g. `http://www.adamscentral.us/`).
* School phone: 312/312; fax: 304/312; `City, NE zip`: 307/312; conference: 312/312; NSAA district: 312/312.
  160 of 312 entries carry the directory's co-op highlight (`class="table-info"`), every one of them on a
  Track & Field and/or Cross-Country row — i.e. slightly over half the NE TF/XC universe is a co-op team and
  must be resolved to its member schools (the classification PDFs name those members with enrollments).
* Extra directory fields usable for identity resolution: mascot, colors, enrollment, NSAA governance district,
  conference.

### Result evidence

| Field | NSAA XC district HTML | NSAA state XC (onlineraceresults) | NSAA state TF (Hy-Tek PDF) | TF district (Athletic.net) |
|---|---|---|---|---|
| ResultID | No | No (place-based rows) | No | depends on Athletic.net |
| AthleteID | No | No | No | — |
| MeetID | No (one file per class) | `race_id` (event_id) | No | **Yes — the 28 meet ids** |
| EventID | No | No (one race per file) | **Yes** — Hy-Tek event numbers | — |
| Mark | Yes (time) | Yes (time + 1m/2m splits + pace) | Yes (mark + attempt series) | — |
| Normalized mark inputs | Partial (5,000 m stated in header) | Yes (5,000 Meters stated) | Partial (metric/imperial per event) | — |
| Timing method | No | No | No (chip/FAT not stated; Hy-Tek output) | — |
| Wind | n/a (XC) | n/a | **Yes** — per attempt and final, e.g. `23-03.25 1.8 3 10` | — |
| Implement/hurdle spec | n/a | n/a | No (state/meet records show specs; implement list is a separate PDF `/textfile/track/availableimplements.pdf`) | — |
| Heat/round | No (merged top-15) | No | **Yes** — `H#` and Finals/Prelims sections | — |
| Place | Yes (top 15) | Yes (full field) | Yes | — |
| Date | In file caption (2025) | Yes (Oct 24, 2025) | Yes (meet dates in header) | — |
| School represented | Yes | Yes | Yes | — |
| Relay membership | n/a | n/a | Yes (relay events list members) | — |
| Grade | **Yes** `Name (11)` | **Yes** `Name  Year` | **Yes** `Name  Year` | — |

Detail: NSAA's XC district HTML files are one per class-gender (8 files: `ccbClassAResults.html`,
`ccgClassAResults.html`, …), each containing per-district "Top 15 Individuals" (place, name, grade, school,
time) plus team scores. The **state** meet is the only NE source we found that yields a full-field, grade-bearing
XC results file that can be parsed mechanically with plain curl:
`https://www.onlineraceresults.com/race/view_plain_text.php?race_id=79870` → 200, 33,461 bytes, `text/html`
wrapping the timer's fixed-width report; columns `Place Team Bib Name Year Team Rank 1m Rank 2m Time Pace`.
"Results by" on the event page names the timer: **Precision Race Results** (`id=37`).

### Incremental use

Everything NSAA publishes is a static object, so weekly refresh is conditional-GET based and cheap:

1. **School/staff refresh (weekly, 1 request):** the directory POST is regenerated on demand; diff by school
   name and coach-name fields. Wrapper: hash the response, or use the per-school GET for the handful of schools
   whose coach changed.
2. **S3 objects (weekly, ~6 conditional GETs):** `HEAD`/`GET` with `If-Modified-Since` or `If-None-Match`
   against `https://nsaa-static.s3.amazonaws.com/textfile/cc/ccbClassAResults.html` (+ 7 sibling files) and the
   classification PDFs. Observed headers prove the mechanism: the 2025 district results file is
   `Last-Modified: Thu, 16 Oct 2025 00:10:06 GMT`, `ETag: "d11cbaa485cefc0cd42efde78e44dc52"`; the 2026
   classifications PDF `Last-Modified: Mon, 03 Aug 2026 13:54:38 GMT`; the 2026 TF results PDF
   `Last-Modified: Thu, 28 May 2026 19:27:37 GMT`. A 304 costs nothing.
3. **Postseason window (Oct 14–Nov):** XC district result files are rewritten the day of each district meet
   (14–15 Oct 2026); the state meet is 23 Oct 2026 → poll the 8 HTML files (conditional) and, once published,
   `onlineraceresults` group `group_id=33` for a new `event_id`, then fetch the 8 `view_plain_text.php`
   races.
4. **Spring window (May):** the `/track-field/` page rotates its 28 Athletic.net district links and adds
   `abresults/cdresults.pdf`; the state TF live results appear under `bst.anet.live/<slug>` →
   `results.blacksquirreltiming.com/meets/<id>`.
5. **In-season XC deltas:** re-fetch the 4 MaxPreps ranking feeds (2 genders × Class A/B) hourly-to-daily; the
   feed is a single small HTML document (6.0–8.6 KB) containing per-school top-5 names/times and meet names —
   diff by (school, athlete, meet) to detect new meets and new athletes without touching Athletic.net.
6. No full-historical re-fetch is ever required: the history is static files and per-season event ids.

### Access characteristics

* **Normal HTML (public forms, no auth):** `secure.nsaahome.org/nsaaforms/direxportscreen.php` (GET single,
  POST bulk), `direxportscreensport.php` (GET/POST), `distassign.php`, `staterecords.php` — all 200 to plain
  curl, no cookies, no CSRF token beyond the empty `session` field.
* **Static PDF/HTML/XLSX-class downloads:** the `/textfile/**` mirror on `nsaa-static.s3.amazonaws.com` —
  public bucket, `ETag`/`Last-Modified`, no rate limiting observed.
* **Public structured-ish HTML:** MaxPreps affiliate feeds (`cross_country_rankings.ashx`, key published in the
  NSAA page source) and onlineraceresults `view_plain_text.php` fixed-width reports — both curl-fetchable.
* **Browser application:** `results.blacksquirreltiming.com` (AthleticLIVE SPA) — the meet URL renders only a
  JS shell to a non-browser client; treat as browser-only for live results.
* **Blocked-to-curl (finding):** the `nsaahome.org` WordPress origin returns **403** (Cloudflare block, 4,546
  bytes HTML) for curl on all paths including `/textfile/*.pdf`. Workarounds that are *not* bypasses: read the
  page through a normal reader fetch, or fetch the same asset from the public S3 mirror (identical bytes:
  `trbclass.pdf` 177,530 B on both paths when reachable).
* **Not available without auth:** `nsaadmin.com/login` (ADs/coaches/colleges), `secure.nsaahome.org/nsaaforms/
  officials/…`, `wrassessor.php`. No attempt was made to authenticate; no protected surface is needed for this
  work.
* **Published limits:** none found. Observed: no 429, no `Retry-After`. We self-limited to ≈50 requests/host,
  sequential.

### Recommendation

**Split verdict — use NSAA for the school/coach substrate and for posture discovery; it is not an athlete
corpus.**

1. **`DISCOVERY-ONLY` + `VALIDATION` for the NE school universe** (primary use): the directory + classification
   PDFs give the authoritative 312-school metadata set, 283 TF and 253 XC participating entries with class,
   enrollment, conference, NSAA district — all in ~5 requests. This is the anchor for `CanonicalSchool` in NE.
2. **`COACH-DIRECTORY` (names/roles/websites only)**: 274/275 TF head coaches and 243/240 XC head coaches as
   person names + AD for 311 schools + school website for 312, from a single request. Marginal coverage vs.
   other sources: nobody else publishes NE coach-role-per-sport at state scale. **It cannot supply emails** —
   pair with school-district directories for the email field.
3. **`ATHLETIC.NET-SEED` for TF** — 28 district MeetIDs from one page, plus the state meet's AthleticLIVE ids;
   this removes meet-search work and gives deterministic team seeding by school name + class.
4. **`RESULT-SOURCE` for postseason results** — state XC (onlineraceresults, 2008–2025, full field, grade
   printed) and state TF (Hy-Tek PDFs, 2012–2026 complete, grade + wind + attempt series) are directly usable
   bulk results with independent verification value. District XC HTML adds grade-bearing top-15 rows.
5. **`DISCOVERY-ONLY` for the MaxPreps XC feed** — the only NSAA-endorsed regular-season surface; useful for
   weekly new-athlete/new-meet deltas, but it exposes no grade and (as observed on 2026-09-19) only early-season
   partial coverage (4–16 schools per feed).
6. **`REJECT`**: NSAA calendar page (Sugar Calendar UI, no meet-level data), NSAA state-records page (elite-only
   records, no cohort utility), and any expectation of NSAA-issued school/team/athlete IDs (they do not exist).

**Expected marginal coverage:** ~312 NE schools canonicalised, ~283 TF + ~253 XC participating entries labeled
by class, ~1,000 coach/AD person-roles, 28 + 2 postseason meet ids, and full-field grade-bearing state results
for 2 sports × ~2 decades — for roughly 40 requests total, versus ≳300 Athletic.net requests to learn the
equivalent school/team/meet surface. Athlete-level discovery remains the gap: NSAA cannot enumerate Class-of-2027
athletes; only grade-stamped postseason results (roughly 800 XC + several thousand TF performance rows per
season) can be harvested, and regular-season athlete discovery must come from MaxPreps/MileSplit/Athletic.net.

### Evidence appendix

All times are CDT (UTC-5), 2026-09-19 unless noted. "read" = harness reader fetch (browser-ish reader mode,
the only method that succeeded on the WordPress origin); "curl" = `/usr/bin/curl` with default UA.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://nsaahome.org/` | read | 200 | NSAA is the current domain; site = WordPress/Elementor; `og:site_name` NSAA; MaxPreps scores widget with NSAA association id | 22:46 |
| `https://nsaahome.org/` | curl | 403 (4,546 B) | WordPress origin blocks non-browser clients (Cloudflare); recorded as access finding | 22:52 |
| `http://nsaahome.org/` | curl | 403 | same block over HTTP | 22:52 |
| `https://www.nsaahome.org/` | curl | 403 | same block; DNS shows Cloudflare IPs (2606:4700:20::…) | 22:53 |
| `https://nsaa-state.org/`, `http://nsaa-state.org/` | curl | 000 | legacy/other NSAA domains do not resolve (curl exit 6) | 22:52 |
| `https://nebraskaschoolactivities.org/`, `https://nebraskansaa.org/`, `https://nsaa.home.org/`, `https://nebraskaschools.org/` | curl | 000 | no such hosts — domain candidates ruled out | 22:52–22:53 |
| `https://nsaahome.org/schools/` | read | 200 | School Directory links (`direxportscreen.php`, `direxportscreensport.php`), enrollment PDFs 2002-03→2026-27, addresses/phones/colors/conferences/districts pages, participation numbers | 22:55 |
| `https://nsaahome.org/coaches/` | read | 200 | NSAA "Coaches" page is coaching-certification only — no coach directory | 22:56 |
| `https://nsaahome.org/track-field/` | read | 200 | 2026 TF classifications links; **28 Athletic.net district MeetIDs**; state TF live links `bst.anet.live/oolpf1` & `/aetvft`; state PDFs; records/history | 22:58 |
| `https://nsaahome.org/track-field/` (range 301-464) | read | 200 | past state TF results archive (1970s→2026) and programs | 22:59 |
| `https://nsaahome.org/cross-country/` | read | 200 | CC classifications path; 8 district-result HTML paths; 2025 state XC onlineraceresults race ids; MaxPreps ranking feeds; 1960→2025 result archive | 23:00 |
| `https://nsaahome.org/calendar/` | read | 200 | Sugar Calendar UI with sport filters; no meet-level schedule exposed to a fetch | 23:01 |
| `https://secure.nsaahome.org/nsaaforms/direxportscreen.php` | curl | 200 (11,585 B) | School directory form; school `<option>` list has **no value attributes** (no school ids) | 23:03 |
| `https://secure.nsaahome.org/nsaaforms/direxportscreensport.php` | curl | 200 (3,937 B) | Sport selector values incl. `tr_b`, `tr_g`, `cc_b`, `cc_g` | 23:03 |
| `.../direxportscreen.php?session=&school=Adams%20Central` | curl GET | 200 (15,579 B) | Single-school view: address, phone, conference, district, enrollment, mascot, colors, homepage, AD + per-sport head coaches; **no email fields** | 23:05 |
| `.../direxportscreen.php` POST `school=View all schools` | curl POST | 200 (1,085,584 B) | **All 312 school entries with 42 distinct role titles incl. TF/XC head coaches** — the bulk primitive | 23:06 |
| `.../direxportscreensport.php` POST `sport=tr_b` | curl POST | 200 (111,269 B) | School │ Website │ Coach rows for TF boys | 23:04 |
| `.../direxportscreensport.php` POST `sport=cc_b` | curl POST | 200 (97,874 B) | School │ Website │ Coach rows for XC boys | 23:04 |
| `https://secure.nsaahome.org/nsaaforms/tr/staterecords.php` | curl | 200 (121,747 B) | State records page exists and is curl-fetchable (elite-only; not a cohort source) | 23:11 |
| `https://nsaahome.org/distassign.php?sport=ccb` | read | 200 | 2026-27 XC boys districts: 20 districts with host/director/date/site; C/D school lists populated; A/B lists empty | 23:02 |
| `https://nsaahome.org/distassign.php?sport=trb` | read | 200 | TF district skeleton (both classes) with **empty school lists** as of 2026-09-19 (assignments posted in-season) | 23:12 |
| `https://nsaa-static.s3.amazonaws.com/textfile/track/trbclass.pdf` | curl | 200 (177,530 B) | Public S3 mirror serves the blocked PDF; 2026 boys TF classes = 283 (A32/B60/C88/D103) with enrollment | 23:07 |
| `.../textfile/track/trgclass.pdf` | curl | 200 (177,626 B) | 2026 girls TF classes = 283 | 23:07 |
| `.../textfile/about/2627enroll.pdf`, `2627boys.pdf`, `2627girls.pdf` | curl | 200 (332,706 / 324,378 / 331,313 B) | 2026-27 enrollment figures by school/gender exist on S3 | 23:07 |
| `.../textfile/cc/ccclassifications.pdf` | curl | 200 (246,643 B) | 2026 XC classes A35/B26/C59/D133 = 253 entries + thresholds; `Last-Modified: 2026-08-03` | 23:08 |
| `.../textfile/cc/ccbClassAResults.html` | curl | 200 (16,301 B) | 2025 Class A boys district results: per-district Top-15 `Name (grade)` + school + time + team scores; `Last-Modified: 2025-10-16`, `ETag "d11cbaa4…"` | 23:08 |
| `.../textfile/cc/ccgClassAResults.html` | curl | 200 (16,545 B) | Girls sibling file, same shape | 23:08 |
| `.../textfile/cc/26Districts.pdf` | curl | 200 (598,372 B) | 2026 district sites/times for all 20 XC districts; "results … posted on the NSAA Cross Country Districts page" | 23:10 |
| `.../textfile/cc/26BDist.pdf` | curl | 200 (130,799 B) | B-1/B-2 (14 teams) vs B-3/B-4 (12 teams) split | 23:14 |
| `.../textfile/track/abres26.pdf` | curl | 200 (142,365 B) | 2026 state TF Class A&B Hy-Tek results: `Name Year School Finals Wind H# Points` + attempt series; produced by "Black Squirrel Timing"; `Last-Modified: 2026-05-28` | 23:09 |
| `.../textfile/track/agres26.pdf`, `.../track/abheat.pdf` | curl | 200 (141,343 / 122,604 B) | Girls state results and boys A heat sheets also on S3 | 23:09 |
| `https://nsaa-static.s3.amazonaws.com/textfile/track/trbclass.pdf` (from `nsaahome.org`) | curl | 403 | The same PDF via the WordPress origin is blocked — proves the S3 mirror is the reliable path | 23:07 |
| HEAD on `.../cc/ccbClassAResults.html`, `.../cc/ccclassifications.pdf`, `.../track/abres26.pdf` | curl -I | 200 | `ETag` + `Last-Modified` on every object → conditional-GET incremental refresh | 23:15 |
| `https://www.onlineraceresults.com/race/view_race.php?race_id=79870` | read | 200 | 2025 NSAA state XC Class A Boys: event `25737`, races `79870–79877`, group `33`, timer "Precision Race Results"; results inline | 23:10 |
| `https://www.onlineraceresults.com/race/view_plain_text.php?race_id=79870` | curl | 200 (33,461 B, text/html) | Full-field fixed-width results: `Place Team Bib Name Year Team Rank 1m Rank 2m Time Pace` (96 rows for this race) | 23:15 |
| `https://www.onlineraceresults.com/event/group.php?group_id=33` | read | 200 | "NSAA Events" index 2008→2025: state XC meet per year (`event_id` 7637…25737) plus some district meets | 23:13 |
| `https://www.onlineraceresults.com/event/view_event.php?event_id=25397` | read | 200 | 2024 state XC: races `78662–78669` (proves the per-season id pattern) | 23:17 |
| `https://www.onlineraceresults.com/event/view_event.php?event_id=23643` | read | 200 | 2020 state XC: races `72432–72439` (deep history) | 23:18 |
| `https://www.maxpreps.com/feeds/affiliates/nsaa/cross_country_rankings.ashx?...&gender=boys&file=html` (+ `&Class=B`, girls ×2) | curl | 200 (6,067–8,648 B each) | 4 public XC feeds (gender × class): per school seeding time + per-meet meet name + top-5 athlete names/times; 2026-09-19 data = boys A 4 schools/63 athletes, boys B 16/138, girls A 4/67, girls B 15/129 | 23:11 |
| `https://www.maxpreps.com/widgets/scores.aspx?associationid=0953b16f-fa82-4c44-a8a8-b27f7f1d6d11&gendersport=boys,football&theme=2&streaming=1` | curl | 200 (189,938 B) | NSAA↔MaxPreps official partnership; association id captured | 23:16 |
| `https://www.maxpreps.com/feeds/affiliates/nsaa/track_rankings.ashx?...` | curl | 301 | No analogous TF feed at the guessed path (unverified/absent — recorded, not assumed) | 23:16 |
| `http://bst.anet.live/oolpf1` | curl | 301 → `results.blacksquirreltiming.com/meets/72610` | NSAA's state TF live link resolves to AthleticLIVE meet id 72610 | 23:11 |
| `http://bst.anet.live/aetvft` | curl | 301 → `…/meets/72612` | C&D state TF live meet id 72612 | 23:12 |
| `https://results.blacksquirreltiming.com/meets/72610` | read | 200 | Page is an AthleticLIVE SPA (assets/favicon from `livestatic.athletic.net`) → browser-application access class | 23:12 |
| `https://results.blacksquirreltiming.com/` | curl | 200 (50,184 B) | AthleticLIVE instance root reachable by curl (meet ids enumerable only in-browser) | 23:12 |
| `http://blacksquirrelresults.com/2026/outdoor/26NSAAGold.pdf` | curl | 200 (305,140 B) | Black Squirrel Timing hosts all-class gold-medal PDFs linked from NSAA | 23:13 |
| `http://precisionraceresults.com/` | curl | 200 → `www.precisionraceresults.com` (37,674 B) | State XC timer is a live NE provider site | 23:13 |
| Local parse: `tools/ne/parse_nsaa_directory.py` over the saved directory HTML | python3 | n/a | 312 schools; AD 311; TF-boys 298 non-empty (274 person names); TF-girls 300 (275); XC-boys 270 (243); XC-girls 270 (240); homepages 312; phonos 312; 0 emails in coach/AD fields; 160 entries have ≥1 co-op-marked sport row | 23:14 |
| Local parse: `trbclass.txt`/`trgclass.txt`/`ccclassifications.txt` + join to directory | python3 | n/a | TF 283/283 classes A32 B60 C88 D103; XC 253 (A35 B26 C59 D133); name-join directory→TF 260/312, →XC 230/312 | 23:15–23:18 |

Session artifacts (read-only inputs for Main; all inside `tools/ne/`):
`nsaa_directory_all_2026-09-19.html` (1,085,584 B captured directory), `ne_nsaa_schools.json`,
`ne_nsaa_schools.csv`, `ne_schools_2026.csv` (school × class(TF/XC) × coaches × AD × website),
`ne_cc_classifications_2026.json`, `trbclass/trgclass/ccclassifications/26Districts/26BDist .txt`,
`mp_boys_A|mp_boys_B|mp_girls_A|mp_girls_B.html`, `abres26.txt`, `parse_nsaa_directory.py`.
