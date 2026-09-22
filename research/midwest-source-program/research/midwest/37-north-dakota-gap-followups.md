# 37. North Dakota gap follow-ups (NDHSAA qualifier lists, coach sample, `dakota` tenant, member universe)

Status: complete
Observed on: 2026-09-20

Session window 09:05–09:12 CDT. 73 HTTP requests total: `search.athletic.live` 21 (1× 403, 4× 400 recorded
as findings), `ndhsaa.com` 16, `ndhsaanow.com` 9, `d2q0tptsfejku7.cloudfront.net` 3 (NDHSAA CDN), district/campus
hosts 21, `s-gke-usc1-nssi3-33.firebaseio.com` 1, `docs.google.com` 2. No 429, no `Retry-After`, no CAPTCHA on any
host. One bot challenge (`www.carrington.k12.nd.us`, "Client Challenge") → that host abandoned and recorded. **No
request was sent to any `*.athletic.net` host.** Raw captures: `research/midwest/evidence/gaps/37/`.

**Headline:** all four follow-up items closed with live evidence. (1) NDHSAA publishes **official state-meet
roster/program PDFs that carry grade** (XC program = `Last | First | Gr.`; track program = per-event
performance lists with Name/Grade/School/mark) plus an "Athletic.net Custom Format" **XLSX qualifier-entry
template whose format includes a `[Grade]` column**; the in-season "PERFORMANCE LISTS (ATHLETIC.NET)" labels on the
state-meet pages are pre-season placeholders with **no hrefs today**. (2) **11 new schools** (33 cumulative) sampled:
**0 emails on NDHSAA pages**; the only tested official source that publishes professional staff emails is the
**Grand Forks Public Schools district directory**. (3) `dakota_meet_list` exists (916 docs) but is **Dakota
Timing (South Dakota) with 0 ND meets**; all ND meets live in `live_results`/`athleticlive`/`heros`/`wayzata`/
`rpmtiming` = **437 unique meets, 421 (96.3%) with non-null `ani`**, and **report-25 open item (a) is closed:
state outdoor track `i=73476` = "NDHSAA Class A & B State Meet (2026)", ani `652729`**. (4) NDHSAA states
**169 member high schools**; official team-universe counts are **XC-B 100, XC-G 96, TF-B 113, TF-G 114**.

---
### Source
Follow-up to report 25 (`25-north-dakota-ndhsaa.md`). Surfaces exercised in this session:

| Surface | Host / path | Role |
|---|---|---|
| NDHSAA association site | `https://ndhsaa.com/` | Member schools, sport pages, qualifier spreadsheet link, standards/regulations |
| NDHSAA "Now" site | `https://ndhsaanow.com/` | State-tournament pages, team universes, **state-meet program PDF index (`/programs`)** |
| NDHSAA CDN | `https://d2q0tptsfejku7.cloudfront.net/` | XLSX template + program/result PDFs |
| NDHSAA state meet pages | `/tournaments/{track,crosscountry}-{boys,girls}` | "Performance Lists & Heat Sheets" section (labels only today); AthleticLIVE result links |
| AthleticLIVE search | `POST https://search.athletic.live/<tenant>_meet_list/_search` | Tenant docs; `dakota` tenant measured; cross-index ND census |
| Firebase RTDB | `GET https://s-gke-usc1-nssi3-33.firebaseio.com/<path>.json?ns=trackmeet-io` | Meet-node verification for the state outdoor track meet |
| District staff directories (coach-email test) | `staffdir.bismarckschools.org`, `gfschools.org/directory`, `fargo.k12.nd.us` Google-Sheet directory, `west-fargo.k12.nd.us`, `central-cass.k12.nd.us`, `carrington.k12.nd.us` | Whether public professional emails exist outside NDHSAA |

### Coverage
Unchanged from report 25 (NDHSAA = the whole state universe: 169 member high schools, Class A/B, boys+girls,
XC + indoor/outdoor TF, historical depth back to the 1960s in program PDFs). New coverage measured here:

* **State-meet program PDFs:** 286 program PDFs on `ndhsaanow.com/programs`, of which **14 Cross Country**
  (through `Cross_Country_{A,B}_State_Program_2025-26.pdf`) and **8 Track & Field**
  (through `Track_State_Program_2024-25.pdf`). Text-extractable (not scans).
* **Coach directory depth:** 33 of 169 schools sampled across reports 25 and 37 (11 new here).
* **Meet index depth (AthleticLIVE):** 437 unique ND meets, 2019–2027 (2026 = 146, 2025 = 119, 2024 = 84,
  2023 = 54, 2022 = 25, ≤2021 = 7, 2027 = 1 future-dated doc, 2222 = 1 sentinel-dated doc).

### Enumeration
**A. Qualifier/participant lists.**
1. *Track & field, official program PDFs (works today):*
   `GET https://ndhsaanow.com/programs` → filter
   `https://d2q0tptsfejku7.cloudfront.net/uploads/files/Publications/Programs/Track_Field/Track_State_Program_<season>.pdf`
   (seasons 2015-16 … 2016-17, 2017-18, 2020-21 … 2024-25; no 2018-19/2019-20 editions). The 2024-25 edition
   (42 pages, 20.06 MB) contains per-event blocks with `Name`, **`Grade` (numeric 7-12)**, school and entering
   marks — i.e. a printable qualifier/performance list. Extracted text contains 1,766 standalone grade-number
   lines (values 9–12) in the event blocks.
2. *Cross country, official program PDFs (works today):*
   `.../Programs/Cross_Country/Cross_Country_A_State_Program_2025-26.pdf` (10 pages, 4.17 MB) → sections
   `CLASS A BOYS ROSTERS` / `CLASS A GIRLS ROSTERS` in `Last | First | Gr.` layout (~**186** and ~**180** grade
   tokens = participant rows in the Class A program alone) plus a `COACHES & BOX ASSIGNMENTS` table
   (school → Boys Coach, Girls Coach). Class B editions exist (`Cross_Country_B_State_Program_2025-26.pdf`).
   No email strings anywhere in the programme (`@` count = 0).
3. *Track & field, Athletic.net submission spreadsheet (works today):*
   `https://ndhsaa.com/athletics/track-{boys,girls}` → anchor "Athletic.net spreadsheet for manual entry of
   state qualifiers" → `.../2018_19_Sports_And_Activities/Sports/Track/Athletic-Net-Custom-Format-Handheld-TR.xlsx`
   (112,191 B). Sheets: `ANET_tfCSV1.1`, `Template Spreadsheet`, `SAMPLE`, `Help Videos`; column set includes
   `E, Gender, [Division], Event, [Place], LastName, FirstName, [Grade], Team, Result, [Score], [Heat/Flight],
   [Wind], R` and relay-leg groups `L1…L8 LastName/FirstName/Grade`. **Grade is representable** (optional
   `[Grade]`, bracketed = optional in Athletic.net custom format).
4. *State-meet page labels (not yet fetchable):* `ndhsaanow.com/tournaments/track-boys` and `-girls` carry the
   section `2027 PERFORMANCE LISTS & HEAT SHEETS` with lines "2027 NDHSAA Class A/B Boys/Girls Performance List
   Link (Athletic.Net)" — the labels exist but **no `href` is attached as of 2026-09-20** (pre-season; TF opens
   2027-03-01). Expected to become live links during the season — re-check after March 2027.
5. *XC state-meet pages:* no qualifier/heat-sheet section exists (only apparel, champions list, past results);
   the XC program PDF (item 2) is the participant list.

**B. Coach sample (item 2).** 11 new schools, one GET each: `https://ndhsaa.com/schools/<id>/<slug>`; parse the
`Sport/Activity Offering | Coaches` table + AD/Activities Director lines + the Website field.
**C. Meet census (item 3).** One cross-index query:
`POST https://search.athletic.live/*_meet_list/_search` with
`{"size":1000,"_source":["i","ani","n","md","o","e","tna","ls"],"query":{"match_phrase":{"lsa":"North Dakota"}}}`
(875 docs returned; dedupe by `i`). Note `lsa` is a text field: a plain `match` on `"North Dakota"` is
**OR-tokenized** and falsely matches "South Dakota" (622 hits) — use `match_phrase` or `lsa.keyword`.
**D. Member/team universe (item 4).** `GET /schools` (169), `GET ndhsaanow.com/teams/<sport>` ×4.

### Stable identifiers
Unchanged from report 25 (NDHSAA school ids, ndhsaanow per-sport team ids, `i`/`ani`/`rui`/`eventId`). New here:

| Identifier | Example | Source | Notes |
|---|---|---|---|
| State-meet program PDF URL | `.../Programs/Cross_Country/Cross_Country_A_State_Program_2025-26.pdf`, `.../Programs/Track_Field/Track_State_Program_2024-25.pdf` | ndhsaanow/programs | Stable, season-versioned, unauthenticated |
| Qualifier-entry template | `.../2018_19_.../Athletic-Net-Custom-Format-Handheld-TR.xlsx` | ndhsaa.com track pages | Athletic.net Custom Format 1.1; format (not data) |
| ND meets sans `ani` | `i` 7949, 11983, 12273, 12276, 12277, 12282, 12293, 15325, 17873, 18439, 23171, 27099, 28280, 36933, 49664, 51677 | ES | 16 meets with null `ani` (mostly collegiate indoor/XC; includes canceled + test meets) |
| Dummy docs to filter | ND census: `i` 66992 "NEW Eliot Local" (`md` 2222-04-15), `i` 51677 "Test Meet 2025"; also `i` 55481 "XC Sample Meet" (`md` 2222-09-01) **in the `dakota` index** | ES | Sentinel/sample docs; exclude by `md` sanity range |

### Athletic.net leverage
* **`ani` coverage of the ND meet universe is now measured at meet level: 421/437 = 96.3%** (deduped across
  tenants; per-index: live_results 421/437, athleticlive 239/244, heros 165/175, wayzata 17/18, rpmtiming 1/1).
  The 16 exceptions are listed above and are mostly collegiate meets that the HS pipeline does not need.
* **Report-25 open item (a) is closed.** `live.athletic.net/meets/73476` is served by the
  `athleticlive`/`live_results` tenant, and the meet doc carries **`ani = 652729`** →
  `https://www.athletic.net/TrackAndField/meet/652729`; the RTDB node `meet_list/73476.json?ns=trackmeet-io`
  (200, 4,826 B) carries `anis[0].u` with the same URL and `abs` = 20 events (100m…4x800mR, DT, HJ, JAV, LJ,
  PV, SP, TJ), divisions `Class A`,`Class B`.
* **`dakota_meet_list` is a dead end for ND**: 916 docs, all Dakota Timing (SD/IA/NE/WY/OR), 0 ND.
* Same conclusion as report 25 applies: ND needs **zero Athletic.net requests** for meet identity (meet-level
  `ani`) and none for state-meet participation (the state-meet program PDFs supply name+grade+school).

### Athlete evidence
* **State-meet program PDFs supply name + school + grade without any Athletic.net call.** XC program: `Last`,
  `First`, `Gr.` (7-12) per school; track program: `Name`, `Grade`, school, mark per event. Format is
  PDF-table; deterministic extraction requires a PDF table reader (or `pdftotext` + column heuristics — the
  column ordering in the extracted text is name block → grade block → school block, see captured sample).
* Nothing about these files is Class-of-2027-specific: grade is the *then-current* grade, so a class year must be
  derived from the meet season (a 2025-10 program "Gr. 10" = Class of 2027; a 2025-05 program "Gr. 10" = Class
  of 2026). Same rule as report 25's `y` field.
* No athlete contact data appears in any file fetched in this session.

### Recruiting information
Verdict for item 2: **NDHSAA is a coach-*name* source, never a coach-*email* source; public professional
emails, where they exist, come from district directories, and coverage is uneven.**

11 new schools (all pages 200; all 44 coach slots named = 100% on this sample, which skews to larger/known
programs). Verbatim coach names as published:

| # | School (id) | Boys XC | Girls XC | Boys TF | Girls TF | AD / Activities Dir. | Site on NDHSAA page |
|---|---|---|---|---|---|---|---|
| 1 | Bismarck Century (6) | Brad Lies | Kate Fox | Justin Miller | Brennan Doan | Ben Lervick, Dave Zittleman | chs.bismarckschools.org |
| 2 | Fargo South (27) | Dustin Swanson | Jason Swier | Alex Koppy | Mike Grant | Mike Beaton, Sarah Link | fargo.k12.nd.us/south |
| 3 | Grand Forks Red River (33) | Richard Dafoe | Nicole Kopff | Jeff Bakke | Nicole Kopff, Adam Eckert | Mike Biermaier, Tyler Nelson | gfschools.org/pages/rrhs |
| 4 | Jamestown (41) | Ken Gardner | Ken Gardner | Ken Gardner | Michael Dietz | Jim Roaldson, Jaidyn Hust | jhs.jamestown.k12.nd.us |
| 5 | Minot (55) | Lance Gehring | Carla Wahlund | Disa Julius, Joshua Knutson | Disa Julius | Matt Ruhland | minot.k12.nd.us |
| 6 | Northern Cass (63) | Jennifer Johnson | Jennifer Johnson | Jennifer Johnson, Darin Eller | Jennifer Johnson, Nikki Gibson | Bryce Laxdal | northerncassschool.org |
| 7 | Thompson (77) | Lindsay George | Lindsay George | Jeremy Anderson | Jeremy Anderson | Brady Schwab, Jason Schwabe | (none on page) |
| 8 | West Fargo (87) | Brad Amundson | Brad Amundson | Darin McKinnon | Darin McKinnon | Logan Midthun, Justin Behm, Dawn Petersen | west-fargo.k12.nd.us |
| 9 | Williston (89) | Shane Wahlstrom | Chase Gregory | Tyler Quilling | Chase Gregory | Robert Conley, Colby Simonsen | (none on page) |
| 10 | Carrington (94) | Brinklyn Johnson | April Hoggarth | April Hoggarth | April Hoggarth | Karla Michaelson | carrington.k12.nd.us |
| 11 | Central Cass (95) | Marv Roeske | Marv Roeske | Tommy Butler | Alex Kingsley | Travis Lemar, Katie McCullough | central-cass.k12.nd.us |

Email presence, by official source (measured, not assumed):

| Source | Result |
|---|---|
| NDHSAA school pages (11 new; 33 cumulative with report 25) | **0/33 pages contain any email string** (no `mailto:`, no address-like text) |
| Grand Forks Public Schools district directory `https://www.gfschools.org/directory` | **Publishes staff emails** (1,747 constituents; entries carry Titles, Locations, Email). Emails are spam-obfuscated client-side (`FS.util.insertEmail(...,"gro.sloohcsfgym","011rekaad",true)` = reversed `mygfschools.org`/`daaker110`); a sample entry verified: TEACHER @ COMMUNITY HIGH SCHOOL with email `daaker110@mygfschools.org`. Coach-level lookup by GET query params returned 0 rows (search is AJAX) → **coach-specific verification [UNVERIFIED]** |
| Bismarck Public Schools `https://staffdir.bismarckschools.org/` | Directory app exists with an **E-Mail column in its result template**, but returned **0 rows** for `CHS`+`COACH` both by last name and unfiltered, with and without session cookies → no coach emails retrievable over plain HTTP |
| Fargo Public Schools "Secondary School Directory" (official Google Sheet linked from `fargo.k12.nd.us/about-us/staff-directory/...`) | 53 rows of school address/phone only; **0 emails** |
| West Fargo PS `.../departments/activities/athletics` | **0 emails**, no coach directory |
| Central Cass HS `https://www.central-cass.k12.nd.us/apps/staff/` (small district) | Full staff list (names + titles, e.g. "Thomas Butler — Physical Education", "Alex Kingsley — Technology Coordinator"); **0 emails** |
| Carrington district site | **Blocked** — "Client Challenge" interstitial; host abandoned, not assessed |
| Thompson / Williston | No school website field on the NDHSAA page at all |

Privacy note: only role-published professional data was read; no athlete contact information was encountered.
The GFPS address form (`<initial><last><number>@mygfschools.org`) is a staff-domain convention, and the two
addresses found on the district page belong to district officials (HR/Title IX, Assistant Superintendent) —
not athletes.

### Result evidence
New result-source facts (all 200 unless noted; `i` = AthleticLIVE meet id, `ani` = Athletic.net MeetID):

| What | Value |
|---|---|
| ND state outdoor track 2026 | `i=73476`, **`ani=652729`**, "NDHSAA Class A & B State Meet (2026)", Bismarck ND, `md` 2026-05-21..23, `o=outdoor`, `ua` 2026-05-24; found in **`live_results_meet_list` + `athleticlive_meet_list`**; RTDB `meet_list/73476` = 200 (4,826 B, `abs` 20 events) |
| ND state outdoor track 2025 | `i=53808` (also present in `live_results` + `athleticlive`; found via `term i:53808`) |
| ND state XC 2026 (Oct 23-24) | **Not yet indexed** — 0 ND meet docs of any type in `md` 2026-10-15..2026-11-10 across all `*_meet_list` (re-confirms report 25's "index lags publication") |
| ND XC meets 2026 (regular season) | 6 meets, 2026-08-22 … 2026-10-10 (Orriginals 76088/277450, Breckenridge-Wahpeton 76109/277021, Blue&White 76112/276762, Border Battle 76117/272168, EDC 76139/278116, Class B East 76140/278115) — all `e=heros`, all with `ani` |
| ND meet universe (all sports) | 437 unique meets; outdoor 288 / indoor 105 / xc 44; XC ani coverage 40/44 |
| Origin-tenant split (`e` field) | athleticlive 244 (243 of them inside the `live_results` mirror), heros 175, wayzata 18, rpmtiming 1 (1 meet double-listed: `i=53058`) |
| Timers seen in ND meet docs (`tna`) | Hero's Timing 130, Metro Timing 51, Ryan McCrady 26, Bismarck Public Schools 24, Randy Votava/Grand Forks PS 22+15, University of Mary 12, Wayzata Results 11+7, FPS Timing 5, Mandan HS 5, others ≤5 |
| `dakota_meet_list` sample (8 meets, md desc) | `76064` DII Central Region 2026-11-21 SD ani=279295 · `76063` NSIC Conf 2026-11-07 SD **ani=null** · `76061` SDHSAA State XC 2026-10-24 ani=278807 · `76060` Mt Marty XC 2026-10-22 ani=279294 · `76367` IA 1A State Qualifier 2026-10-22 ani=280587 · `76059` IA 2A State Qualifier **ani=null, ls missing** · `76058` IA 3A State Qualifier **ani=null, ls missing** · `55481` "XC Sample Meet" md 2222-09-01 ani=261522 (dummy) → **5/8 non-null ani** |

### Incremental use
Unchanged report-25 loop (ES by date window → RTDB per event). Additions from this session:
1. **State-meet program PDFs** are season-static artifacts: fetch once per state meet (1 request) and cache by URL;
   detect new editions by scraping `/programs` (1 request/season) rather than polling.
2. **Coach refresh** stays 1 request/school/season; the co-op annotation lives inline in the sport cell, so
   co-op identity changes (`26-27`/`27-28` rows in the published coop sheet) remain the only reconciliation input.
3. **Do not use `dakota_meet_list` for ND** — it is SD/Iowa/Nebraska. ND meet discovery stays on
   `heros_meet_list` plus the global mirrors (`live_results`/`athleticlive`), which is where the state
   outdoor track meet actually lives.
4. Filter dummy docs by `md` year: exclude year ≥ 2100 (measured sentinels: `i=66992` in the ND set,
   `i=55481` in `dakota`) and titles containing `Sample`/`Test`/`CANCELED` if a clean meet list is required.

### Access characteristics
* All NDHSAA/ndhsaanow/CDN fetches: **200**, plain HTML/PDF/XLSX, no auth, no browser, no cookies.
* **403 (recorded, not worked around):** `GET https://search.athletic.live/dakota_meet_list/_mapping` →
  `security_exception`, API key lacks `view_index_metadata`. Field types were therefore derived empirically
  (`lsa`/`ls`/`o`/`tna` are text; only `lsa.keyword` and `e.keyword` aggregations succeeded).
* **400 (recorded as findings):** aggregations on `ls`, `o`, `tna.keyword` fail with
  `illegal_argument_exception … Text fields are not optimised for … field data`; use `<field>.keyword` where it
  exists, or count client-side from a size-N response (used here: `*_meet_list` size 1000, 194,636 B).
* **Bot challenge:** `www.carrington.k12.nd.us` → HTTP 200 "Client Challenge" interstitial; treated as a block.
* Politeness: ≥1.3 s spacing inside every batch via the shared helper (`tools/gaps/37-probe.sh`); sequential
  only; 73 requests total, below the 80-request budget; the busiest host (`search.athletic.live`) took 21.
* `www.gfschools.org/directory` is a public directory with client-side email obfuscation — the obfuscation is an
  anti-spam measure, not access control; nothing was bypassed (no auth/CAPTCHA/paywall), and only a single
  staff entry was read to prove the mechanism.

### Recommendation
Keep report 25's three-adapter verdict; the follow-ups sharpen it:
1. **NDHSAA school/coach adapter — keep (COACH-DIRECTORY, names only).** 33/33 sampled pages have zero emails;
   plan for name + school switchboard, and treat district directories (GFPS-style) as the optional email layer,
   not as a guaranteed one.
2. **State-meet program PDF adapter — add (RESULT-SOURCE, XC/TF, grade-bearing).** 1 request per season gives an
   official participant list with grade (XC) / performance list with grade and marks (TF), and the XC program
   doubles as a per-school coach list for the state-meet qualifiers. This is the cheapest known Grade-11
   verification for ND that touches no Athletic.net host.
3. **AthleticLIVE meet-index adapter — keep, but scope it to `heros` + `live_results`/`athleticlive`;** do not
   build a `dakota` adapter (916 SD/IA/NE docs, 0 ND). The state outdoor TF meet (73476/652729) lives in the
   `athleticlive`/`live_results` tenant, so the 2026 state track payload is reachable now, not only after
   Hero's indexes it.
4. **Open item carried forward:** re-check the state-meet page `PERFORMANCE LISTS (ATHLETIC.NET)` hrefs after
   March 2027 and re-check ES for the 2026 state XC meet (Oct 23-24 2026) — it is not indexed as of today.

---
### Evidence appendix
All timestamps 2026-09-20 America/Chicago (CDT). `search.athletic.live` rows are POSTs unless noted.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://search.athletic.live/dakota_meet_list/_search` `{"size":0,"track_total_hits":true,"query":{"match_all":{}}}` | POST | 200 | `dakota_meet_list` exists with **916 docs** (matches the 2026-09-20 tenant-bucket count) | 09:05:29 |
| same index, agg `ls` | POST | 400 | `ls` is a text field — fielddata disabled (recorded; shapes query design) | 09:05:31 |
| same index, `{"size":8,"sort":[{"md":"desc"}],"_source":[i,ani,n,md,ls,lsa,o,...]}` | POST | 200 | 8-meet sample: tenant content is SD/IA/OR; **5/8 non-null `ani`**; dummy doc `55481` (`md` 2222-09-01) exposed | 09:05:32 |
| same index, `agg lsa`+`agg session` | POST | 400 | text-field aggregation error (recorded) | 09:05:40 |
| same index, `match lsa:"North Dakota"` | POST | 200 | **622 hits — false positive** (OR-tokenized; matched "South Dakota"); teaches `match_phrase`/`.keyword` | 09:05:41 |
| same index, `{"size":1,"_source":["*"]}` | POST | 200 | 95-field doc shape; `e=dakota`, `tna="Dakota Timing"`, `us=http://dakota.anet.live/6j52kk`, `atr` credits dakotatiming.com; `xcp[]` event list with ids | 09:05:47 |
| same index, `match_phrase ls:"," ND"` | POST | 200 | 0 hits (tokenization; query retired) | 09:05:49 |
| `https://search.athletic.live/dakota_meet_list/_mapping` | GET | **403** | `security_exception` — API key lacks `view_index_metadata`; mapping not obtainable (recorded, not bypassed) | 09:06:00 |
| same index, `agg lsa.keyword` | POST | 200 | State mix: **SD 622, IA 253, NE 31, (blank) 7, WY 2, OR 1 = 916; 0 North Dakota** | 09:06:02 |
| same index, `match_phrase lsa:"North Dakota"` | POST | 200 | **0 hits** — tenant carries no ND meets (phrase query is the correct form) | 09:06:04 |
| `https://search.athletic.live/*/_search` — filters: ND phrase agg by `_index`; `term i:73476`; `term i:53808` | POST | 200 | ND meets by index: **live_results 437, athleticlive 244, heros 175, wayzata 18, rpmtiming 1**; **`i=73476` and `i=53808` found in live_results + athleticlive** | 09:06:14 |
| `https://search.athletic.live/*/_search` — ND filter, per-index `exists(ani)` | POST | 200 | ani coverage: live_results 421/437, athleticlive 239/244, heros 165/175, wayzata 17/18, rpmtiming 1/1 | 09:06:24 |
| `live_results_meet_list/_search` ND sample (10, md desc) | POST | 200 | ND docs in the global index incl. 2026 XC meets + RF meets; each with `ani` | 09:06:25 |
| `live_results_meet_list/_search` `term i:73476` | POST | 200 | Meet doc: **"NDHSAA Class A & B State Meet (2026)", Bismarck ND, 2026-05-21..23, outdoor, `ani=652729`** | 09:06:27 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_list/73476.json?ns=trackmeet-io` | GET | 200 (4,826 B) | RTDB serves the state TF meet: `ani 652729`, `anis[0].u=https://www.athletic.net/TrackAndField/meet/652729`, `abs` 20 events, `dvs` Class A/B | 09:06:37 |
| `northstar_meet_list/_search` (8) | POST | 200 | 7 docs, all **North Star Timing (NH/MA)** — ND-looking name ruled out | 09:06:38 |
| `dakota_meet_list` agg `o` + `tna.keyword` | POST | 400 | text-field agg error (recorded) | 09:06:40 |
| `live_results_meet_list` ND + `o=xc` + `md 2026-10-15..2026-11-05` | POST | 200 | **0 hits** — 2026 state XC not indexed | 09:06:41 |
| `https://search.athletic.live/*_meet_list/_search` ND census `size 1000` | POST | 200 (194,636 B) | **875 docs → 437 unique meets**; outdoor 288 / indoor 105 / xc 44; 421 with `ani`, 16 without; origin tenants athleticlive 244 / heros 175 / wayzata 18 / rpmtiming 1 | 09:06:50 |
| `https://ndhsaa.com/athletics/track-boys` | GET | 200 (119,541 B) | Links the "Athletic.net spreadsheet for manual entry of state qualifiers" (XLSX); text "Coaches: State Qualifying Reporting Directions"; 2027 rules-clinic/open dates | 09:07:07 |
| `https://ndhsaa.com/athletics/track-girls` | GET | 200 (119,563 B) | Same structure (girls) | 09:07:09 |
| `https://ndhsaa.com/athletics/crosscountry-boys` / `-girls` | GET | 200 (113,118 / 113,192 B) | XC pages carry rules/PDFs only — **no qualifier/entry list** | 09:07:10–12 |
| `https://ndhsaanow.com/tournaments/track-boys` / `-girls` | GET | 200 (315,441 / 319,949 B) | "2027 PERFORMANCE LISTS & HEAT SHEETS" + "…Performance List **Link (Athletic.Net)**" labels exist, **no href attached**; results chain 2026→`live.athletic.net/meets/73476/events`, 2025→`/meets/53808`, ≤2024→CDN PDFs | 09:07:14–16 |
| `https://ndhsaanow.com/tournaments/crosscountry-boys` / `-girls` | GET | 200 (298,605 / 298,387 B) | XC state pages: apparel store, champions links, past results; **no participant list section** | 09:07:38–41 |
| `https://d2q0tptsfejku7.cloudfront.net/uploads/files/2018_19_Sports_And_Activities/Sports/Track/Athletic-Net-Custom-Format-Handheld-TR.xlsx` | GET | 200 (112,191 B) | Official qualifier-entry spreadsheet; sheets `ANET_tfCSV1.1`/`Template Spreadsheet`/`SAMPLE`/`Help Videos`; columns include `[Grade]` and `L1..L8Grade` → **grade representable** | 09:07:42 |
| `https://ndhsaa.com/schools` | GET | 200 (97,754 B) | **"The NDHSAA is currently made up of 169 member high schools"** (verbatim page text); 169 unique `/schools/<id>/<slug>` links parsed | 09:07:57 |
| `https://ndhsaanow.com/teams/crosscountry-boys` | GET | 200 (413,580 B) | **100** XC boys teams (ids in hrefs, names in `title`) | 09:08:02 |
| `https://ndhsaanow.com/teams/crosscountry-girls` | GET | 200 (409,536 B) | **96** XC girls teams | 09:08:07 |
| `https://ndhsaanow.com/teams/track-boys` | GET | 200 (434,328 B) | **113** boys TF teams | 09:08:12 |
| `https://ndhsaanow.com/teams/track-girls` | GET | 200 (435,806 B) | **114** girls TF teams | 09:08:17 |
| `https://ndhsaa.com/schools/{6,27,33,41,55,63,77,87,89,94,95}/...` | GET ×11 | 200 ×11 | 11 new school pages: coach table 11/11 with all 4 sports filled (44/44 names), AD/Activities Director present 11/11, **0 emails**; website field present 9/11 | 09:08:36–53 |
| `https://www.bismarckschools.org/` | GET | 200 (97,469 B) | Links `staffdir.bismarckschools.org` + `/bpsdirectory` | 09:09:09 |
| `https://www.west-fargo.k12.nd.us/` | GET | 200 (108,571 B) | Activities/Athletics section paths found | 09:09:10 |
| `https://www.gfschools.org/` | GET | 200 (76,724 B) | Links district `/directory` | 09:09:12 |
| `https://www.fargo.k12.nd.us/` | GET | 200 (194,344 B) | Links `/about-us/staff-directory/secondary-school-directory` | 09:09:13 |
| `https://staffdir.bismarckschools.org/` | GET | 200 (12,950 B) | BPS "Staff Directory Lite" search form (`buildingchoice`, `rolechoice=COACH`, `lastname`, `firstname`) | 09:09:24 |
| `https://www.fargo.k12.nd.us/about-us/staff-directory/secondary-school-directory` | GET | 200 (58,415 B) | School contact links only; page references official Google-Sheet directory | 09:09:26 |
| `https://www.gfschools.org/directory` | GET | 200 (84,501 B) | **1,747 constituents** with Titles/Locations/**Email** (client-side obfuscated); sample entry verified TEACHER@COMMUNITY HIGH SCHOOL → `daaker110@mygfschools.org` | 09:09:30 |
| `https://www.west-fargo.k12.nd.us/departments/activities/athletics` | GET | 200 (83,741 B) | No emails, no coach directory | 09:09:33 |
| `https://www.gfschools.org/directory?const_search_last_name=Dafoe` (+ full param set; + `const_search_keyword=Dafoe`) | GET ×3 | 200 ×3 (0 constituents each) | Directory search is AJAX-only over GET → coach-level lookup **not verified** | 09:09:56, 09:10:04, 09:12:01 |
| `https://staffdir.bismarckschools.org/?mode=formsearch` (POST: CHS/COACH/Lies) | POST | 200 (930 B frameset) | Form works; results frame is `default.asp?mode=urlsearch…` | 09:10:12 |
| `…/default.asp?mode=urlsearch&b=CHS&r=COACH&…&ln=Lies` and unfiltered, with and without cookie jar | GET ×3 | 200 (706 B each) | **0 rows** — BPS coach search returns only the result-table header (E-Mail column present but unpopulated) | 09:10:18, 09:10:28, 09:10:44 |
| `https://docs.google.com/spreadsheets/d/1acRzN_jk7_65btep3YodB78EQr80y916ND3Rw1rE6XY/export?format=csv` | GET | 307 → 200 (3,842 B, 53 rows) | Fargo "Secondary School Directory" = addresses/phones; **0 emails** | 09:10:19, 09:10:26 |
| `http://www.carrington.k12.nd.us` | GET (-L) | 301 → 200 "Client Challenge" | Bot-challenge interstitial → host abandoned | 09:10:53 |
| `http://www.central-cass.k12.nd.us/` | GET (-L) | 302 → 200 (71,568 B) | Site links `/apps/staff/` directory | 09:10:54 |
| `https://ndhsaanow.com/programs` | GET | 200 (412,700 B) | **286 state program PDFs** incl. Cross_Country ×14 (to 2025-26) and Track_Field ×8 (to 2024-25) | 09:10:57 |
| `https://www.central-cass.k12.nd.us/apps/staff/` | GET | 200 (150,002 B) | Staff names + titles (incl. the two TF coaches' names under their non-coach titles); **0 emails** | 09:11:06 |
| `…/Programs/Cross_Country/Cross_Country_A_State_Program_2025-26.pdf` | GET | 200 (4,166,037 B, 10 pp) | **`CLASS A BOYS/GIRLS ROSTERS` = `Last` / `First` / `Gr.` columns** (~186 + ~180 grade tokens) + `COACHES & BOX ASSIGNMENTS`; 0 emails | 09:11:20 |
| `…/Programs/Track_Field/Track_State_Program_2024-25.pdf` | GET | 200 (20,055,554 B, 42 pp) | Per-event performance lists with **Name, Grade, School, marks** (1,766 standalone grade-number lines) | 09:11:22 |
| `https://search.athletic.live/live_results_meet_list/_search` ND + agg `e.keyword` | POST | 200 | live_results ND docs' origin tenants: athleticlive 243 / heros 175 / wayzata 18 / rpmtiming 1 (= 437) → **merged mirror** | 09:12:19 |
| `https://search.athletic.live/*_meet_list/_search` ND + `md 2026-10-15..2026-11-10` | POST | 200 | **0 meets** — 2026 state XC still absent (any sport, any tenant) | 09:12:20 |

Raw captures kept under `research/midwest/evidence/gaps/37/` (24 JSON query/response snapshots, 45 HTML page
captures, the qualifier XLSX, both program PDFs, the Fargo directory CSV and the `pdftotext` extractions; no
cookie or token values retained).
