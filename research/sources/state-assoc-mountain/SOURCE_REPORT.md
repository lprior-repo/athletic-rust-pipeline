# SOURCE_REPORT — state high-school association sources, Mountain region

Lane: `research/sources/state-assoc-mountain/` (AZ, CO, ID, MT, NM, UT, WY).
All captures are anonymous `curl` at <=1 req/s per host (azpreps365.com publishes
`Crawl-Delay: 10`, honoured with 11 s spacing). Every sample below is in `samples/` and is
listed with URL, status, bytes, UTC timestamp and exact command in `samples/CAPTURES.md`.

Field contract (objective §13): Source name; Geographic coverage; Sports; Historical depth;
Discovery mechanism; Stable identifiers; Pagination; Athlete fields; Meet fields; Result fields;
Grade/class evidence; Coach/contact fields; Public API availability; Static file availability;
Browser requirement; Request cost; Published rate limits; Known blocks; Cross-source join keys;
Estimated marginal coverage; Implementation recommendation.

Machine-readable companions: `schema.json` (observed keys + one real example each),
`coverage.json` (per-jurisdiction measured counts, enumerators, gaps).

---

## 1. AZ — Arizona Interscholastic Association (AIA)

- **Source name**: Arizona Interscholastic Association (AIA); site `https://aiaonline.org/`;
  championship results portal `https://azpreps365.com/`.
  Evidence: `samples/home-aiaonline.html` (200, 72790 B), `samples/azpreps365-results-xc-boys.html`.
- **Geographic coverage**: State of Arizona, all counties —
  "The Arizona Interscholastic Association has a membership of 287 schools from every county in
  Arizona." (`samples/aia-schools.html`).
- **Sports**: 30 aligned activities, including "Cross Country - Boy's", "Cross Country - Girl's",
  "Track - Boy's", "Track - Girl's" (`samples/aia-alignments-activities.html`).
- **Historical depth**: azpreps365 result-set dropdowns reach back to
  "2003 Track & Field 1A-5A State Championships" (30 sets; `samples/azpreps365-results-tf-boys.html`)
  and "2001 Cross Country 1A-3A State Championships" (33 sets;
  `samples/azpreps365-results-xc-boys.html`).
- **Discovery mechanism**: three full-roster views, no pagination —
  `/alignments/2026/` (Master Conferences, 287 unique `/alignments/2026/operators/<id>` links),
  `/alignments/2026/activities/` (per-sport school counts), `/alignments/2026/operators/`
  (By School index, 287 unique ids again). Counts and both independent enumerations are in
  `coverage.json`; sources `samples/aia-alignments.html`, `samples/aia-alignments-activities.html`,
  `samples/aia-alignments-operators.html`.
- **Stable identifiers**: AIA operator id (`/alignments/2026/operators/206` = Ash Fork, enrollment 97
  — `samples/aia-alignments.html`); directory id (`/schools/4`, `/schools/58`); result-set id
  (`/story?id=18609`, `?id=18666`); azpreps365 result-set option values (`18609`, `18666`).
- **Pagination**: none needed for the alignment views; the autocomplete JSON caps at 20 rows with no
  paging (`?limit=500` and `?page=2` both returned 20 identical-shape rows —
  `samples/aia-search-limit500.json`, `samples/aia-search-page2.json`).
- **Athlete fields**: XC state results PDF individual table is
  `Place | Name | Grade | Team | Time | Points`; real row `1  Taylor Drewry  12  Basha  18:05.8  1`
  (`samples/aia-state-xc-2025-pdf-page3.txt`).
- **Meet fields**: meet name, venue, race label + distance, date/time —
  "Arizona State Championships / Cave Creek Golf Course / Race 7 - Girls D1 - 5K / 11/15/2025 - 1:35pm"
  (`samples/aia-state-xc-2025-pdf-page1.txt`).
- **Result fields**: per-race PDF; team table `Team | Score | Team Time | Avg | Avg 5 Man Mile Gap`
  plus DNP rows (`aia-state-xc-2025-pdf-page1.txt`, `...-page2.txt`), individual table as above.
  TF state results are Hy-Tek Meet Manager pages: "Hy-Tek's MEET MANAGER Page 1 /
  AIA State Championship - 5/15/2026 to 5/16/2026 / Mesa Community College / Results"
  (`samples/aia-tf-state-results-18666.txt`).
- **Grade/class evidence**: yes for XC — explicit `Grade` column (grade 12 in the captured row);
  TF state result pages carry no grade column (`aia-tf-state-results-18666.txt`, no `Grade` match).
- **Coach/contact fields**: per-school staff blocks with name + role + `tel:` link —
  "Jeff Frazine | Athletic Director", "Teresa Laurean | Assistant Principal - Activities",
  "Selma Dillon | Athletic Secretary", "Tim Britt…" and a coach list
  "Robert Kochis Coach Cross Country - Girl's Division II South"
  (`samples/aia-school-detail-58.html`). No email addresses published on the page (0 `mailto:`).
- **Public API availability**: `GET /schools/search.json?q=<prefix-or-empty>` returns JSON
  (`id, name, full_name, mascot, logo.thumbnail, alignment.conference, alignment.region,
  address.line1/city/state/zip`), 20 rows max (`samples/aia-schools-search-emptystring.json`,
  `samples/aia-schools-search-a.json`). Endpoint and cap were read out of the site bundle
  (`samples/aia-app.js`: `src:"/schools/search.json"`, `limit:20`).
- **Static file availability**: yes — championship results are static PDFs behind
  `https://www.aiaonline.org/story?id=<n>` which 302/303-redirects to
  `/files/<id>/<slug>.pdf` (e.g. `…/files/18609/2025-cross-country-division-i-iv-state-championships.pdf`,
  22401579 B, sha256 `6a6c650cade2a40d135e065aa15e63d3cfbd3819f99ac0fb06bdc01bc8dbcb9e`).
- **Browser requirement**: none for HTML/JSON/PDF; the azpreps365 result picker submits a plain
  GET (`<form method="GET" action="https://www.aiaonline.org/story">`, option value = story id).
- **Request cost**: XC state results PDF is 22.4 MB for one season (43 pages); all other captures
  <=300 KB. `azpreps365.com` requires >=10 s between requests.
- **Published rate limits**: `azpreps365.com/robots.txt` = "User-agent: * / Crawl-Delay: 10"
  (`samples/azpreps365-robots.txt`); `aiaonline.org/robots.txt` = "User-agent: * / Disallow:"
  (`samples/robots-aiaonline.txt`).
- **Known blocks**: 20-row cap on `/schools/search.json` with no paging parameter; state meet
  XC results are one large PDF, not per-event files.
- **Cross-source join keys**: school full name + mascot + city/state/zip
  (`aia-schools-search-emptystring.json`), AIA conference/region, MileSplit team pages
  (`az.milesplit.com`, linked from `aia-sport-tf.html`).
- **Estimated marginal coverage**: 287 member schools; 252 XC-boys teams, 251 XC-girls, 267 TF-boys,
  266 TF-girls (`aia-alignments-activities.html`, corroborated per-sport at 252 operator ids in
  `samples/aia-activity-xc-boys.html`). Athlete-level rosters are NOT on the AIA side for XC —
  AIA delegates entries to Athletic.net — so the marginal contribution is the school/coach universe
  plus official state result files.
- **Implementation recommendation**: **DISCOVERY_SOURCE** for the school+coach universe
  (287 schools, AD and per-sport coach names/phones), **RESULT_SOURCE** for state championship
  results (PDF, grade column on XC), **CONDITIONAL** as the entry platform (Athletic.net owns XC
  entries; MileSplit/MaxPreps own TF).
- **Timing / result platform (measured, quoted)**:
  - XC state meet: "Chip timing will be used and provided by Finished Results from California."
  - XC sectionals: "Chip timing will be used and provided by Wingfoot Finish at all sectional meets"
  - Entries: "All entries will be completed on www.athletic.net." and
    "All athletes running at the State Meet must be entered with www.athletic.net."
    (`samples/aia-2026-xc-tournament-guide.pdf`, pdfinfo Author: Mary Wimmer).
  - TF state meet: "HyTek must be used to export a backup of the entire meet to Sue Hysong within 72
    hours after completion of the meet"; "instructions on how to set up a meet in az.milesplit.com and
    www.athletic.net"; rankings/rosters through MaxPreps (`samples/aia-2026-tf-tournament-guide.pdf`).
  - PDF provenance: XC state PDF was authored by "Finished Results" / Creator "Track Scoreboard"
    (jsPDF 2.3.1, Title `girls-d1.pdf`) vs TF state PDF Creator "Crystal Reports" containing
    "Hy-Tek's MEET MANAGER" pages — two different timing/result pipelines in the same association.
- **Published Athletic.net / MileSplit links (markup quoted verbatim)**:
  - `samples/aia-sport-xc.html`:
    `<a href="https://www.aiaonline.org/files/18387/athleticnet-cross-country-meet-entry-process.pdf" target="_blank">2026 Athletic.net State Entry Instructions</a>`
    (anchor text as rendered: "2026 Athletic.net State Entry Instructions";
    PDF Title: "Submitting Team Entries for a Cross Country Meet - Athletic.net Support").
  - `samples/aia-sport-xc.html`:
    `<a href="https://www.aiaonline.org/files/17555/how-to-claim-and-edit-your-team-information-in-milesplit.pdf" target="_blank">How to claim and edit your team information in Milesplit</a>`
  - `samples/aia-sport-tf.html`:
    `<a href="https://az.milesplit.com/articles/358467/entry-breakdown-mingus-invitational?utm_source=Iterable&amp;utm_medium=email&amp;utm_campaign=campaign_12649035" target="_blank">Hosting and Setting Up an Invitational</a>`

---

## 2. CO — Colorado High School Activities Association (CHSAA)

- **Source name**: CHSAA; site `https://chsaanow.com/` (Next.js, media on a DigitalOcean Spaces CDN);
  member portal `https://schools.chsaa.org/` (auth-gated).
  Evidence: `samples/home-chsaanow.html`, `samples/chsaa-schools-home.html`.
- **Geographic coverage**: State of Colorado; "Currently, the CHSAA has 378 active member schools
  across Colorado." (`samples/chsaa-schools-path.html`), independently repeated as
  "378 of 378 schools" on `samples/chsaa-schools-staff.html`.
- **Sports**: 27 sports pages incl. `/sports/cross-country` and `/sports/track-and-field`
  (`samples/chsaa-sports.html`); per-school alignment shows e.g. "Boys Cross Country 5A Single".
- **Historical depth**: championship history per sport — "Showing the 15 most recent of 209 titles"
  for Boys Cross Country, entries like `2025 5A Niwot … 28`, school totals back to 1976
  (`samples/chsaa-xc-champions.html`).
- **Discovery mechanism**: `/schools` embeds the full member list in the React-Server-Components
  payload — 378 records with keys `schoolCode, slug, href, name, officialName, city, memberType,
  schoolType, setting, logoUrl, districtName, streetAddress, zipCode, phone, mapUrl`
  (`samples/chsaa-schools-path.html`). Measured splits: memberType Member 363 / Preliminary 7 /
  Activity 8; schoolType Public 289 / Charter 48 / Non-Public 41.
- **Stable identifiers**: `schoolCode` (309 = Academy), `slug` (`/schools/academy`), MaxPreps
  `schoolid` UUID (`2ffb2809-94ad-4f52-9425-b918046fbeaf`) — `samples/chsaa-school-academy.html`.
- **Pagination**: none — all 378 records ship in the first HTML response; the staff directory page
  also enumerates all 378 schools (`samples/chsaa-schools-staff.html`).
- **Athlete fields**: state TF results list `Name | Year | School` where `Year` is the grade —
  "1 Case Matteson / 12 Legend / 10.74Q 0.5 2" (`samples/chsaa-media-tf-state-results-5a-2026.pdf`,
  `pdftotext` page 1). XC athlete results live on TFMeetPro per-race pages.
- **Meet fields**: TF: "COLORADO HIGH SCHOOL STATE TRACK CHAMPIONSHIPS 2026 - 5/14/2026 to 5/16/2026 /
  Jefferson County Stadium, Lakewood"; XC: "Colorado Springs, CO / Norris Penrose Event Center -
  New Course / 11/1/2025" with a Meet Officials block (`samples/tfmeetpro-state-xc-4a.html`).
- **Result fields**: XC (TFMeetPro): per-classification event list, team scoring summary
  `Score | Scoring Order | Total | Avg. | Spread`, plus "Timing: Rapid Results Timing"
  (`samples/tfmeetpro-state-xc-4a.html`). TF (PDF): Hy-Tek Meet Manager pages with prelims/finals,
  wind, heat and records blocks (`samples/chsaa-media-tf-state-results-5a-2026.pdf`).
- **Grade/class evidence**: yes — `Year` column in TF results; XC events are split by classification
  (1A–5A) and gender (`samples/tfmeetpro-state-xc-4a.html`).
- **Coach/contact fields**: `/schools/staff` states "Listings show names and positions only — no email
  addresses or phone numbers are published." Per-school pages expose "Staff directory ( 43 )" /
  "( 224 )" counts whose contents are fetched client-side on tab activation — the JSON endpoint was
  NOT captured (open question). School records carry a main `phone` (Academy "303-853-1210").
- **Public API availability**: none published for schools; a logo/media API exists at
  `https://feed.chsaa.co/api/v1/logos?filename=Academy.webp` (referenced in the school payload).
  School data needs no API because it is server-rendered into HTML.
- **Static file availability**: yes — championship results are static PDFs on the media CDN
  (`https://chsaa-media.sfo3.cdn.digitaloceanspaces.com/wp-content/uploads/2026/06/16211311/5a_state_results_2026.pdf`,
  212669 B, 55 pages, Producer "Powered By Crystal", header "Rapid Results Timing - Contractor
  License", body "Hy-Tek's MEET MANAGER"). The human URL
  `/documents/2026/6/16/5a_state_results_2026.pdf` returns an HTML viewer page (HTTP 200,
  `content-type: text/html`, `x-nextjs-prerender: 1`), not PDF bytes.
- **Browser requirement**: none for school/sport pages; the document viewer is JS but the CDN PDF URL
  is present in the HTML. Staff-tab content requires the SPA fetch.
- **Request cost**: 1 request for all 378 schools; 1 request per school detail page (378 total,
  ≈7 min at 1 rps); staff tab = extra per-school fetch (endpoint not captured).
- **Published rate limits**: `chsaanow.com/robots.txt` = `Allow: /` for `User-agent: *` with only
  `/history/champions/individual/totals/repeat/` and `/preview/` disallowed, plus named AI crawlers
  blocked (GPTBot, ClaudeBot, anthropic-ai, CCBot, PerplexityBot, …);
  `schools.chsaa.org/robots.txt` disallows `/api/`, `/dashboard/`, `/login`, `/logout`,
  `/compliance`, `/report/` (`samples/robots-chsaanow.txt`, `samples/robots-schools-chsaa.txt`).
- **Known blocks**: `schools.chsaa.org` redirects anonymous requests to
  `https://schools.chsaa.org/login?callbackUrl=%2Fdashboard` — the School Center is
  authentication-only and was not touched. Staff-directory contents are behind a client-side fetch.
  A Calaméo "Digital Program" link (`https://www.calameo.com/read/007737357e30cacdf7523`) now 404s
  (`samples/chsaa-results-calamo.html`).
- **Cross-source join keys**: `slug` / `officialName` / `city` / `schoolCode`, MaxPreps `schoolid`
  UUID, classification tags (5A/4A/…), MileSplit CO team pages.
- **Estimated marginal coverage**: 378 member schools (363 full members) with per-school sport
  alignment and classification. Measured samples: Cherry Creek 34 sports in the 2026–28 cycle incl.
  "Boys Cross Country 5A" + "Boys Track and Field 5A" (`samples/chsaa-school-cherry-creek.html`);
  Telluride 14 sports incl. "Boys Cross Country 2A" + "Boys Track and Field 2A"
  (`samples/chsaa-school-telluride.html`). Statewide headline on `samples/chsaa-sport-xc.html`:
  "more than 176000 students involved in sports activities in 2025-26". Athlete-level XC data is not
  published by CHSAA (it goes to TFMeetPro/Rapid Results); TF athlete data appears only in the
  end-of-meet PDF.
- **Implementation recommendation**: **DISCOVERY_SOURCE** (school universe, classification, sport
  alignment, MaxPreps school UUIDs) + **RESULT_SOURCE** (state meet results: TFMeetPro for XC,
  Hy-Tek PDF for TF) + **CONDITIONAL** for staff/coach data (names + positions only, SPA-fetched).
- **Timing / result platform (measured, quoted)**:
  - XC state championships: `Results by: <a href=https://rapidresultslive.com style=color:black>Rapid Results Timing</a>`
    with per-class links to `http://results.tfmeetpro.com/Rapid_Results_Timing/2A_Colorado_State_Championships25/`
    (2A/3A/4A in the captured page; source `samples/co-state-xc-2025-results-index.html`).
  - The TFMeetPro event page repeats "Timing: Rapid Results Timing" in its Meet Officials block
    (`samples/tfmeetpro-state-xc-4a.html`).
  - TF state championships: PDF produced by Rapid Results Timing on a Hy-Tek Meet Manager licence.
  - `results.tfmeetpro.com/robots.txt` was not readable: `curl: (35) Recv failure: Connection reset
    by peer` (`samples/robots-tfmeetpro.txt.curlerr`); the plain-HTTP results pages answer 200.
- **Published Athletic.net / MileSplit links (markup quoted verbatim)**:
  - MileSplit is a CHSAA corporate partner; the site payload carries
    `{"id":"cG9zdDo3NjQ4NA==","name":"MileSplit","logoUrl":"https://chsaa-media.sfo3.cdn.digitaloceanspaces.com/wp-content/uploads/2026/03/20192411/MileSplit_Logo.png","logoAlt":"MileSplit CO Logo","websiteUrl":"https://co.milesplit.com/","showOnFrontPage":false}`
    (`samples/chsaa-sport-xc.html`).
  - Recap result links:
    `<a target="_blank" rel="noopener noreferrer" … href="https://www.rapidresultslive.com/2025/chsaaxc25/index.htm">…<span class="line-clamp-1">Results</span></a>`
    (`samples/chsaa-recap-5a-boys-xc-2025.html`) and
    `[Results] -> /documents/2026/6/16/5a_state_results_2026.pdf`
    (`samples/chsaa-recap-5a-boys-tf-2026.html`).
  - `athletic.net` appears in **zero** CHSAA captures (checked across `samples/chsaa-*.html`); the
    per-sport external link of choice is MaxPreps:
    `https://www.maxpreps.com/local/team/schedule.aspx?schoolid=2ffb2809-94ad-4f52-9425-b918046fbeaf&gendersport=boys%2Ctrack&season=spring`
    (`samples/chsaa-school-academy.html`).

---

## 3. ID — Idaho High School Activities Association (IHSAA / IDHSAA)

- **Source name**: Idaho High School Activities Association, `https://idhsaa.org/`
  (site title "IHSAA - Idaho High School Activities Association").
  Evidence: `samples/home-idhsaa.html`.
- **Geographic coverage**: State of Idaho; members grouped into districts I–VI and classes 6A–1A.
- **Sports**: XC and TF pages exist (`/cross-country`, `/track-and-field`), classified as
  6A/5A and 4A/3A/2A for track, single XC championship
  (`samples/idhsaa-cross-country.html`, `samples/idhsaa-track-field.html`).
- **Historical depth**: XC page links "2024 State Results" and "State Records" (PDF
  `/asset/Year in Review/XC Records.pdf`); TF page links 2025 results, 2027 rules, and
  "Girls Records"/"Boys Records" PDFs (`idhsaa-track-field.html`). No year-by-year archive page.
- **Discovery mechanism**: `/directory` — page headline "Member High Schools (173)"
  (`samples/idhsaa-directory.html`). Measured: the same page renders **174 unique school rows**
  (no duplicates; classifications 1A 42, 2A 36, 3A 30, 4A 21, 5A 25, 6A 20 = 174). The 173-vs-174
  discrepancy is real and recorded as a gap: one school is in the table but not in the headline count.
- **Stable identifiers**: school name (no code published in the directory); class + district
  (e.g. "Aberdeen High School | 3A | V"); the site's own asset paths
  (`/asset/XC/…`, `/asset/TRACK/…`).
- **Pagination**: none — the whole directory is one HTML table (174 `<tr>` rows over one table).
- **Athlete fields**: not published by IDHSAA; state-meet athlete data is delegated to Athletic.net
  links (see below). XC/TF record PDFs list mark holders only.
- **Meet fields**: not published on-site; meet pages live on Athletic.net / Snake River Timing.
- **Result fields**: on-site none; result artifacts are external links (Athletic.net meet pages /
  `live.snakerivertiming.com` live-results pages).
- **Grade/class evidence**: class/classification per school (6A…1A) in the directory; per-athlete
  grade only via the external result platforms.
- **Coach/contact fields**: yes — AD directory with `School | Classification | District |
  AD Name | AD Phone | AD Email` (`samples/idhsaa-directory.html`). E-mails are published
  base64-obfuscated behind `document.write(window.atob('…'))`; e.g. Aberdeen row decodes to
  `<a href='mailto:lewisn@aberdeen58.org'>lewisn@aberdeen58.org</a>`; Alturas Preparatory lists two
  ADs. `/coaches` is a coaching-education page, not a coach directory
  (`samples/idhsaa-coaches.html`); `/athletic-directors` is a resource page with no table
  (`samples/idhsaa-athletic-directors.html`); `/rosters` returns 404 (`samples/idhsaa-rosters.html`).
- **Public API availability**: none observed.
- **Static file availability**: yes for rules/records PDFs under `/asset/...`; results are not
  static files on this host.
- **Browser requirement**: none for IDHSAA pages. Results links require a browser because they point
  into Athletic.net, which returns a Cloudflare challenge shell to curl:
  `https://www.athletic.net/CrossCountry/meet/263030/info` → HTTP 200, 9292 B, title
  "Track & Field, Cross Country Results, Statistics", body contains `cloudflare` + `challenge`
  (`samples/athleticnet-idhsaa-xc-state-263030.html`).
- **Request cost**: 1 request for the full directory; sport pages are <50 KB.
- **Published rate limits**: `idhsaa.org/robots.txt` = "User-agent: *" with no Disallow lines
  (`samples/robots-idhsaa.txt`).
- **Known blocks**: `/rosters` 404; directory headline count (173) disagrees with the rendered table
  (174 rows); state results sit behind Athletic.net's Cloudflare.
- **Cross-source join keys**: school name; class/district; Athletic.net meet ids
  (`CrossCountry/meet/263030`, `TrackAndField/meet/646835`, `/646834`, `/591968`, `/596626`).
- **Estimated marginal coverage**: 174 school rows with AD name/phone/email —
  the strongest AD contact surface in this lane. Per-school XC/TF sponsorship is NOT published;
  the state meet entries on Athletic.net are the participation proxy.
- **Implementation recommendation**: **COACH_SOURCE** (AD names/phones/e-mails, 174 schools)
  + **DISCOVERY_SOURCE** (school universe, class, district) + **CONDITIONAL** for results
  (Athletic.net links: no on-site results, Cloudflare on the target).
- **Timing / result platform (measured, quoted)**:
  - 2024 XC state results → `live.snakerivertiming.com` (Snake River Timing):
    `<a href="https://live.snakerivertiming.com/meets/41688" title="" target="_blank" style="color: rgb(153, 0, 51); font-size: 14px;">2024 State Results</a>`
    (`samples/idhsaa-cross-country.html`). [INFERENCE] Snake River Timing is an Idaho-based timer;
    no rate-limit or pricing page was captured for it.
  - 2026 XC state results and 2025/2026 TF state results → Athletic.net (see markup below).
- **Published Athletic.net / MileSplit links (markup quoted verbatim)**:
  - `samples/idhsaa-cross-country.html`:
    `<a href="https://www.athletic.net/CrossCountry/meet/263030/info" class="btn btn-primary edit" style="border-radius: 50px; color: rgb(255, 255, 255);" title="">2026 State Meet Results</a>`
  - `samples/idhsaa-track-field.html`:
    `<a href="https://www.athletic.net/TrackAndField/meet/646835/info" title="" style="font-size: 28px;">6A/5A RESULTS</a>`
    `<a href="https://www.athletic.net/TrackAndField/meet/646834/info" title="" style="font-size: 28px;">4A/3A/2A RESULTS</a>`
    `<a href="https://www.athletic.net/TrackAndField/meet/591968/info" title="" target="_blank"><span style="color: rgb(153, 0, 51);">2025 6A/5A Results</span></a>`
    `<a href="https://www.athletic.net/TrackAndField/meet/596626/info" title="" target="_blank"><span style="color: rgb(153, 0, 51);">2025 4A/3A/2A Results</span></a>`
  - No MileSplit link appears anywhere in the captured IDHSAA pages.

---

## 4. MT — Montana High School Association (MHSA)

- **Source name**: Montana High School Association, `https://www.mhsa.org/` (WordPress on the
  rSchoolToday platform; assets on `assets-rst7.rschooltoday.com`).
  Evidence: `samples/home-mhsa.html`.
- **Geographic coverage**: State of Montana, classes AA / A / B / C (XB classification list).
- **Sports**: `/sports-activities/fall/cross-country/` and `/sports-activities/spring/track-field/`
  (`samples/mhsa-cross-country.html`, `samples/mhsa-track-field.html`).
- **Historical depth**: XC history text — "Boys' Cross Country was first introduced as a fall sport
  in Montana in 1964…"; "Girls' Cross Country became an officially sanctioned sport in the fall of
  1971" (`samples/mhsa-cross-country.html`). Enrollment files are per-year PDFs
  ("2024-25 MONTANA HIGH SCHOOL *FALL ENROLLMENT BY CLASSIFICATION (SIZE)", SOURCE "*OPI ENROLLMENT
  FIGURES FALL 2024").
- **Discovery mechanism**: `/schools/directory/` does not list schools itself — it says
  "Click on the link below to view the updated, online directory of MHSA Member Schools via
  ArbiterSports" → `https://live.arbiter.io/org/4497` (`samples/mhsa-schools-directory.html`,
  `samples/mhsa-arbiter-directory.html`). Arbiter's route returns a 4278-byte JS SPA shell
  (`/directory/assets/index-BLYioIsx.js`), so the member list needs a browser.
  The measured school universe instead comes from `/enrollment-numbers/` →
  "2024-25-Fall-Enrollment-by-Classification.pdf" (`samples/mhsa-enrollment-classification.pdf`) and
  "2024-25-Fall-Enrollment-Alphabetically.pdf" (`samples/mhsa-enrollment-alpha.pdf`):
  **184 unique school entries** (184 name/enrollment pairs, no duplicates), total enrollment
  **45,205** students. Two entries are out-of-state schools that MHSA lists (Grenora, ND; Mullan, ID).
  `[INFERENCE]` the AA/A/B/C split is present in the same PDF; a per-class count needs
  layout-aware column parsing that was not completed (open item).
- **Stable identifiers**: school name; class (AA/A/B/C); Arbiter org id `4497`; Athletic.net ids
  (`CrossCountry/meet/267780`, `team/22658/track-and-field-outdoor/2026`).
- **Pagination**: none — single HTML/PDF artifacts.
- **Athlete fields**: none published on mhsa.org; athlete data is delegated to Athletic.net /
  Competitive Timing (see below).
- **Meet fields**: state XC meet block on the sport page — "WHEN October 24, 2026 / WHERE Amend Park
  – Billings, MT"; TF state meet listed as "2026 STATE AA-B TRACK & FIELD MEET, May 28-30, 2026,
  MCPS Stadium, 3100 South Ave. West, Missoula, MT 59804" (`samples/mhsa-cross-country.html`,
  `samples/mhsa-search-athleticnet.html`).
- **Result fields**: not on mhsa.org; the XC postseason page links
  `https://competitivetiming.com/mhsa-state-meet-results/` which renders client-side (captured shell
  shows only "All Events / Other Resources / MHSA State Meet",
  `samples/mhsa-state-meet-results-competitive.html`).
- **Grade/class evidence**: school class (AA/A/B/C) available; athlete grade not published.
- **Coach/contact fields**: no coach/AD directory on mhsa.org (the member directory is Arbiter-hosted
  and JS-only). Sport contacts are published as mailto links, e.g. XC contact "Kip Ryan" →
  `mailto: kryan@mhsa.org` (`samples/mhsa-cross-country.html`); association address
  "631 N Last Chance Gulch, Helena, MT 59601, Phone: 406-442-6010".
- **Public API availability**: none observed.
- **Static file availability**: yes — an extensive PDF library on the rSchoolToday asset host, e.g.
  `https://assets-rst7.rschooltoday.com/rst7files/uploads/sites/152/2025/09/19090658/2024-25-Fall-Enrollment-by-Classification.pdf`
  (137357 B) and state-meet qualifying forms
  (`…/2025/10/09111644/2026-Boys-MHSA-State-Cross-Country-Meet-Qualifying-Form.pdf`).
- **Browser requirement**: yes for the member directory (Arbiter SPA) and for Competitive Timing
  results (Next.js client render); no for the MHSA pages and PDFs.
- **Request cost**: low (pages ≤340 KB; PDFs 135-137 KB).
- **Published rate limits**: **no robots.txt** — `https://www.mhsa.org/robots.txt` returns
  `404 Not Found` (13 B, `samples/robots-mhsa.txt`); `https://assets-rst7.rschooltoday.com/robots.txt`
  also 404 (13 B, `samples/robots-rst7-assets.txt`); `competitivetiming.com/robots.txt` =
  "User-agent: * / Allow: / / Disallow: /admin" (`samples/robots-competitivetiming.txt`).
  Absence of a robots file means no published policy — the lane kept the global 1 req/s cap.
- **Known blocks**: MHSA directory behind ArbiterSports JS; Competitive Timing results client-rendered;
  no on-site athlete-level results at all.
- **Cross-source join keys**: school name + class; Arbiter org `4497`; Athletic.net meet/team ids.
- **Estimated marginal coverage**: 184 school entries with enrollment (45,205 students) — good for the
  school universe and class structure; **zero** athlete-level records; state results require
  Competitive Timing (XC) or Athletic.net (XC + TF).
- **Implementation recommendation**: **DISCOVERY_SOURCE** (school universe, class, enrollment) +
  **VALIDATION_SOURCE** (cross-check of school names/classes against other sources). Results are
  **CONDITIONAL** (external timers, one of them Athletic.net → Cloudflare).
- **Timing / result platform (measured, quoted)**:
  - XC state meet results: `competitivetiming.com` — page text "2026 STATE CROSS COUNTRY MEET
    Competitive Timing Link Athletic.net link"
    (`samples/mhsa-search-athleticnet.html`), link target
    `https://competitivetiming.com/mhsa-state-meet-results/`
    (`samples/mhsa-xc-postseason.html`).
  - TF state meet: no timer named; the postseason page's only result link is Athletic.net.
- **Published Athletic.net / MileSplit links (markup quoted verbatim)**:
  - `samples/mhsa-xc-postseason.html`:
    `[Competitive Timing Link] -> https://competitivetiming.com/mhsa-state-meet-results/`
    `[Athletic.net link] -> https://www.athletic.net/CrossCountry/meet/267780/info`
  - `samples/mhsa-tf-postseason.html`:
    `[Athletic.net Link] -> https://www.athletic.net/team/22658/track-and-field-outdoor/2026`
    (appears twice, once for the AA-B meet block and once for A-C).
  - No MileSplit link appears in any captured MHSA page.

---

## 5. NM — New Mexico Activities Association (NMAA)

- **Source name**: New Mexico Activities Association, `https://www.nmact.org/`
  (WordPress/Jupiter theme; Sucuri + Cloudflare in front — responses arrive gzip-encoded).
  Evidence: `samples/home-nmact.html`.
- **Geographic coverage**: State of New Mexico; 5 classifications A, 2A, 3A, 4A, 5A
  (football additionally 6A/8-man/6-man) per "SECTION IV CLASSIFICATION AND ALIGNMENT 2026-2028".
- **Sports**: `/sports/cross-country/` and `/sports/track-and-field/`
  (`samples/nmaa-cross-country.html`, `samples/nmaa-track-and-field.html`).
- **Historical depth**: `/records/` hosts per-year championship result PDFs —
  `2025_CC_Results.pdf`, `2024_*`, `2023_*`, `2022_*` for XC and `2022`–`2025` for track
  (`samples/nmaa-records.html`). The timer's own archive (`liverunningresults.com`) lists event sets
  back to 2021 incl. `events/2021nmaaxc/` (`samples/liverunningresults-home.html`).
- **Discovery mechanism**: `/member-schools/` renders no school list in HTML (JS-driven; 0 `<table>`
  rows). The measured enumeration comes from the classification PDF:
  `https://www.nmact.org/file/Section_4.pdf` (`samples/nmaa-section4-classification.pdf`), which
  states per sport — "CROSS COUNTRY: 5 CLASSES / 5A 25 SCHOOLS / 4A 29 / 3A 25 / 2A 26 / A 21 /
  **TOTAL: 126 SCHOOLS**" and "TRACK & FIELD: 5 CLASSES / 5A 25 / 4A 29 / 3A 28 / 2A 33 / A 41 /
  **TOTAL: 156 SCHOOLS**", each followed by district-by-district school name lists
  (e.g. 5A District 1: Cleveland, Farmington, Piedra Vista, Rio Rancho, Volcano Vista).
- **Stable identifiers**: school names in the alignment lists; athlete `Comp#` in state rosters;
  PDF paths under `/file/`; MileSplit onboarding URLs.
- **Pagination**: none — single PDF/HTML artifacts.
- **Athlete fields**: state championship **rosters are published as PDFs with grade**:
  `2026_4A5A_Boys_Rosters.pdf` = Hy-Tek roster export, columns
  `Name | Comp# | Sex | Year` grouped by school (e.g. "1. Carson Ames 500 M 11"; "7. Gage Conway 506
  M 12") — 361 entries measured, grade histogram {8:15, 9:41, 10:82, 11:110, 12:113}
  (`samples/nmaa-2026-4a5a-boys-rosters.pdf`). Girls rosters exist as a parallel file.
- **Meet fields**: "2026 NMAA State Track & Field Championships / Class 4A & 5A BlueCross BlueShield -
  5/15/2026 to 5/16/2026 / UNM Track & Field Complex"; XC: "New Mexico State Cross Country
  Championships / Hosted by New Mexico Activities Association / Albuquerque Academy, Albuquerque, NM -
  11/08/2025" (`nmaa-2026-4a5a-final-results.pdf`, `nmaa-2025-cc-results.pdf`).
- **Result fields**: XC individual table
  `Place | TmPl | Bib | Name | Year | School | Finals | Pace` (Year printed as SR/JR/SO/8 —
  e.g. "5 250 Carlos Ragland JR Pecos 16:23.2 5:17"); TF is Hy-Tek two-column results with relay
  legs and state-meet records (`samples/nmaa-2025-cc-results.pdf`,
  `samples/nmaa-2026-4a5a-final-results.pdf`).
- **Grade/class evidence**: **yes, in both directions** — roster PDFs (Year) and XC result PDFs
  (Year) let Grade-11 / Class-of-2027 athletes be identified directly. Both files are dated and
  official (Hy-Tek "Licensed To: Track Timing and Data Management LLC - Contractor License").
- **Coach/contact fields**: no coach or AD directory; `/for-coaches/` and
  `/for-athletic-directors/` are resource hubs (31 and 6 PDF links, 2 mailto each)
  (`samples/nmaa-for-coaches.html`, `samples/nmaa-for-athletic-directors.html`). NMAA publishes a
  staff directory at `/staff-directory/` and uses MaxPreps for team/stat content
  (`http://www.maxpreps.com/state/cross-country/new-mexico.htm` on the XC page).
- **Public API availability**: none observed; all data is HTML + PDF.
- **Static file availability**: yes, and this is the strong suit — `/file/*.pdf` covers rosters, heat
  sheets, day results and final results per class and gender
  (`2026_A3A_Day1_Results.pdf`, `2026_4A5A_Final_Results.pdf`, `2026_4A5A_Boys_Rosters.pdf`, …).
- **Browser requirement**: none for the PDFs; the member-school list and Heyzine "championship
  program" flip-books (`https://heyzine.com/flip-book/da170f00d8.html`) need a browser.
- **Request cost**: rosters 486 KB, TF final results 168 KB, XC results 357 KB (106 pages).
- **Published rate limits**: `nmact.org/robots.txt` = "User-Agent: * / Disallow:" (allow all,
  `samples/robots-nmact.txt`); `liverunningresults.com/robots.txt` returns **404** (13 B,
  `samples/robots-liverunningresults.txt`).
- **Known blocks**: the live-results host carries a licence restriction — "The results listed on this
  website are for the NON-COMMERCIAL use of the race participants and the host. … It is a violation of
  United States Copyright laws to use any results on this site for commercial purposes without the
  expressed written consent of the website owner" (`samples/liverunningresults-home.html`). Treat
  `liverunningresults.com` as display-only. The member-schools page has no server-rendered list.
- **Cross-source join keys**: school name; athlete name + school + grade; bib/`Comp#`;
  `nm.milesplit.com` athlete/team pages (`https://nm.milesplit.com/rankings/leaders/high-school-boys/outdoor-track-and-field`).
- **Estimated marginal coverage**: XC 126 schools, TF 156 schools in 2026-28 alignment; state-meet
  athlete-level rosters with grade (4A/5A boys alone: 361 athletes, 110 of them grade 11) — the best
  grade-bearing artifact set in this lane after Utah.
- **Implementation recommendation**: **RESULT_SOURCE** (official state results + rosters, grade-bearing)
  + **DISCOVERY_SOURCE** (school alignment by class/district, 126 XC / 156 TF schools)
  + **VALIDATION_SOURCE** (roster vs. MaxPreps/MileSplit team membership).
- **Timing / result platform (measured, quoted)**:
  - XC state championships: PDF footer "Wingfoot Finish * www.wingfootfinish.com"
    (`samples/nmaa-2025-cc-results.pdf`) — the same timer AIA uses for Arizona sectionals.
  - TF state championships: Hy-Tek Meet Manager export "Licensed To: Track Timing and Data Management
    LLC - Contractor License" (`samples/nmaa-2026-4a5a-boys-rosters.pdf`,
    `samples/nmaa-2026-4a5a-final-results.pdf`, Creator "Crystal Reports ActiveX Designer -
    tfmm6results2col.rpt").
  - Live results: `[2026 1A-3A Boys & Girls LIVE RESULTS] -> http://liverunningresults.com/`
    (`samples/nmaa-track-and-field.html`).
- **Published Athletic.net / MileSplit links (markup quoted verbatim)**:
  - `samples/nmaa-track-and-field.html`:
    `[MileSplit Coach Onboarding] -> https://support.milesplit.com/s/onboarding/nmaa-coach-onboarding`
    `[MileSplit Meet Manager Onboarding] -> https://support.milesplit.com/s/onboarding/nmaa-meet-manager-onboarding`
    `https://nm.milesplit.com/rankings/leaders/high-school-boys/outdoor-track-and-field`
  - The XC page links MaxPreps instead:
    `http://www.maxpreps.com/state/cross-country/new-mexico.htm`.
  - No `athletic.net` link appears in any captured NMAA page.

---

## 6. UT — Utah High School Activities Association (UHSAA)

- **Source name**: Utah High School Activities Association, `https://uhsaa.org/` (WordPress +
  Elementor). Evidence: `samples/home-uhsaa.html`.
- **Geographic coverage**: State of Utah, 1A–6A plus regions 1–22; "2025-27 UHSAA Alignment - All
  Activities Except Football" lists every school per region
  (`samples/uhsaa-realignment-2025-27.pdf`).
- **Sports**: XC `https://uhsaa.org/uhsaa-cross-country/` and TF
  `https://uhsaa.org/uhsaa-track-and-field/` (`samples/uhsaa-cross-country.html`,
  `samples/uhsaa-track-and-field.html`). A `uhsaa-track-field` slug returns 404
  (`samples/uhsaa-track-field.html`).
- **Historical depth**: every sport page offers "View All Past Results" →
  `https://uhsaa.org/v2/historical-cross-country-results/` and
  `https://uhsaa.org/v2/historical-track-field-results/` — **these paths are disallowed by the site's
  own robots.txt (`Disallow: /v2/`) and were therefore NOT fetched**. The timer's public archive goes
  back to 2015 ("RESULTS | 2026 | 2025 | 2024 | 2023 | 2022 | 2021 | 2020 | 2019 | 2018 | 2017 | More
  Results | 2016 | 2015", `samples/runnercard-meet-1001674.html`).
- **Discovery mechanism**: `/school-directory-new/` renders the complete member list server-side —
  **162 schools** as `data-school`/`data-category` anchors of the form
  `../school-directory/?id=Alta&Reg=6&schoolID=1` (`title='Alta Hawks' data-category='5A'`), measured
  distribution **2A 35, 4A 34, 1A 30, 5A 29, 6A 17, 3A 17 = 162**
  (`samples/uhsaa-school-directory.html`). The alignment PDF independently gives class→region→school
  tables with per-class counts (6A 17, 5A 29, 4A 34, 3A 17, 2A 35, 1A 30) reading down the left margin
  (`samples/uhsaa-realignment-2025-27.pdf`).
- **Stable identifiers**: `schoolID` + `id` + `Reg` tuple in the directory URL
  (`?id=Alta&Reg=6&schoolID=1`); class (5A) and region (6) as `data-category`; RunnerCard meet ids
  (`runner.Main?meet=1001674`); timer results path `results.runnercard.com/Results/results.jsp?meetid=<id>`.
- **Pagination**: none — one HTML page for all 162 schools, one PDF for the alignment.
- **Athlete fields**: not published by UHSAA on the association pages; athlete-level data belongs to
  RunnerCard (currently unreachable, see Known blocks).
- **Meet fields**: the sport pages carry state-meet blocks — TF: "State Information"
  `https://uhsaa.org/track/2027/TrackStateInfo.pdf`, "State Meet Schedule"
  `…/track/2027/StateTrackSchedule.pdf`, "Qualifying Standards" `…/track/2027/StateQual.pdf`; XC:
  `https://uhsaa.org/xc/2024/XCStateCourseMap.pdf` (`samples/uhsaa-track-and-field.html`,
  `samples/uhsaa-cross-country.html`).
- **Result fields**: association-side results are consumed from the timer; the TF page embeds the
  live/state meet link `[Runnercard State Meet] -> https://www.runnercard.com/e/runner.Main?meet=1001674`
  (`samples/uhsaa-track-and-field.html`). Live video is `https://kslsports.com/stream`.
- **Grade/class evidence**: **school-level class only** (5A/4A/…). The participation matrix is a
  per-school X-mark grid, not a per-athlete roster — no athlete grade is published by UHSAA.
- **Coach/contact fields**: **yes, and this is the strongest coach surface found in the lane** —
  each `school-directory/?id=…` page carries address, district, classification, region, colours,
  phone/fax, then a `Directory` block (`Role | Name`: Principal, Ast. Principal Over Athletics,
  **Athletic Director**, Athletic Trainer) and a `Coaches` block by sport with **mailto links**.
  Measured on Alta (5A): "Athletic Director | Jim Langford", "Boys Cross Country | Rebecca Bennion",
  "Girls Cross Country | Rebecca Bennion", "Football | Blake Burdette", 25 `mailto:` targets incl.
  `Rebecca.Bennion@canyonsdistrict.org` (`samples/uhsaa-school-alta.html`).
- **Public API availability**: none observed; `wp-json/oembed` only.
- **Static file availability**: yes — a PDF library (`/realignment/…`, `/track/2027/…`,
  `/bxcountry/RunnerCardInstructions.pdf`, `https://uhsaa.org/ParticipationNumbers.pdf`).
- **Browser requirement**: none for the directory, alignment or PDFs; the embedded "Results"
  accordions render `href="#"` placeholders and are JS-driven.
- **Request cost**: 1 request for all 162 school directory entries; 1 request per school detail page
  (162 total ≈ 3 min at 1 rps); `ParticipationNumbers.pdf` 312 KB; alignment PDF 534 KB.
- **Published rate limits**: `uhsaa.org/robots.txt` returns 200 (1181 B,
  `samples/robots-uhsaa.txt`) with `User-agent: *` and 30 `Disallow:` lines but **no `Crawl-delay`**
  — the rules cover `/administrator/`, `/media/`, `/mlr/`, `/phpGrid/`, `/js/`, and **`/v2/`**, which
  is the rule that excludes the historical-results indexes. RunnerCard's `robots.txt` is **404**
  (`samples/robots-runnercard.txt`, 431 B), i.e. no published policy.
- **Known blocks**: (a) robots-aware exclusion — the historical-results indexes live under `/v2/`
  which the site's robots.txt disallows; (b) **the Utah results host is down**: both
  `http://results.runnercard.com/Results/results.jsp?meetid=1001674` attempts returned
  **503 "No server is available to handle this request"** (107 B each,
  `samples/runnercard-results-jsp.html`, `samples/runnercard-results-jsp-retry.html`), and the
  fallback origin hard-coded in RunnerCard's own redirect stub
  (`http://66.29.162.100/Results/ViewResults.html?class=server&meetid=1001674`) failed to connect
  (curl exit non-zero, no body written). State-meet athlete results for Utah were therefore **not
  captured**.
- **Cross-source join keys**: school name; `schoolID`; class + region; RunnerCard `meetid`;
  MaxPreps (UHSAA links `https://www.maxpreps.com/` and a "How to use MaxPreps" page).
- **Estimated marginal coverage**: 162 schools × (AD + per-sport coach names + emails) = a complete
  coach/AD surface; 162-school class/region alignment; participation matrix giving per-school XC/TF
  sponsorship X-marks (`samples/uhsaa-participation-numbers.pdf`: 92 school rows carry marks,
  2109 marks in total, of which 74 schools are attributed to each of the four XC/TF columns —
  `[INFERENCE]` the four-column attribution carries ±1-column ambiguity because the sport headers are
  rotated; the 92 row / 2109 mark totals are exact). Athlete-level marginal coverage is **zero today**
  because RunnerCard is unavailable.
- **Implementation recommendation**: **COACH_SOURCE** (AD + head XC/TF coach names and district
  e-mails, 162 schools) + **DISCOVERY_SOURCE** (school universe, class, region, participation X-marks)
  + **CONDITIONAL** for results (RunnerCard 503 at capture time; no association-side athlete data).
- **Timing / result platform (measured, quoted)**:
  - RunnerCard is Utah's platform: `[Instructions for Schools to Enter Teams for State …] ->
    https://www.uhsaa.org/bxcountry/RunnerCardInstructions.pdf` (XC) and
    `[Create RunnerCard Meet Director's Account] -> https://www.uhsaa.org/btrack/RunnercardMeetInstructions.pdf`,
    `[Runnercard State Meet] -> https://www.runnercard.com/e/runner.Main?meet=1001674` (TF)
    (`samples/uhsaa-cross-country.html`, `samples/uhsaa-track-and-field.html`).
  - XC region results are submitted through `https://uhsaa.org/xc/Results`
    (`samples/uhsaa-cross-country.html`) — so even regional XC results route into RunnerCard.
  - Live broadcast partner: `[Watch Live] -> https://kslsports.com/stream`.
- **Published Athletic.net / MileSplit links (markup quoted verbatim)**: **none** — zero
  `athletic.net` and zero `milesplit` matches across every captured UHSAA page; UHSAA's external
  stats partner is MaxPreps (`[Rankings] -> https://uhsaa.org/uhsaa-rankings-home/`,
  `https://uhsaa.org/how-to-use-maxpreps/`, `https://www.maxpreps.com/`).

---

## 7. WY — Wyoming High School Activities Association (WHSAA)

- **Source name**: Wyoming High School Activities Association, `https://www.whsaa.org/` (static
  Apache/Plesk site with Bootstrap 4.4.1). Evidence: `samples/home-whsaa.html` (200, 21570 B).
- **Geographic coverage**: State of Wyoming, classes 4A/3A/2A/1A.
- **Sports**: **no `/sports/<sport>` routes exist** — `/sports/cross-country/` and
  `/sports/track-and-field/` both 404 (`samples/whsaa-cross-country.html`,
  `samples/whsaa-track-and-field.html`). Real sport pages live in short directories:
  `https://www.whsaa.org/track/statetrack.html` (200, `samples/whsaa-track-state.html`) and
  `https://www.whsaa.org/crosscountry/statecrosscountry.html` (200, `samples/whsaa-xc-state2.html`).
- **Historical depth**: not published as a browsable archive — `https://www.whsaa.org/archives/`
  returns **403 Forbidden** (directory listing denied, `samples/whsaa-archives.html`); record PDFs
  are referenced relatively (`../archives/track/class.pdf`, `../archives/track/overallstate.pdf`).
- **Discovery mechanism**: **no school list is retrievable anonymously** — `/schools/` returns
  **403**, `/whsaa/members.html` 404 (`samples/whsaa-schools.html`, `samples/whsaa-members.html`).
  The only measured universe figure is the association's own classification summary PDF
  `https://www.whsaa.org/forms/2026-28ADMs.pdf`, whose full text is
  "2A / 18 Schools … 4A / 16 Schools … 3A / 16 Schools … 1A / 20 Schools"
  (`samples/whsaa-2026-28-adms.pdf`, 1 page) → **4 classes, 70 schools total**. No school names are
  present in that file.
- **Stable identifiers**: school name only (no code published in captured pages); timer side ids
  (`https://milesplit.live/meets/681838`).
- **Pagination**: none.
- **Athlete fields**: none on the association site.
- **Meet fields**: state meet blocks — TF "STATE TRACK & FIELD May 20th - 22nd, 2027 Casper Kelly
  Walsh High School"; XC "STATE CROSS COUNTRY October 24th, 2026 Cheyenne"
  (`samples/whsaa-track-state.html`, `samples/whsaa-xc-state2.html`).
- **Result fields**: the XC state page publishes labelled result slots — page text reads
  "… RESULTS / STATE CROSS COUNTRY LIVE RESULTS / STATE CROSS COUNTRY LIVE STREAM / STATE CROSS
  COUNTRY RESULTS (PDF) …" but **every one of those anchors carries an empty `href=""`**
  (`grep -oE 'href="[^"]*"' samples/whsaa-xc-state2.html` shows only CSS, two parking PDFs, a Google
  Drive link and the merchandise/weather links). `[INFERENCE]` the 2026-10-24 meet has not happened
  yet, so the association has not yet published its results links.
- **Grade/class evidence**: class level only (4A/3A/2A/1A); no athlete grade published.
- **Coach/contact fields**: none captured. The homepage's only staff contact is a mailto
  (`mailto:jwalkerlat@gmail.com`); `/schools/` is 403, so no AD directory is reachable anonymously.
- **Public API availability**: none observed.
- **Static file availability**: yes but thin — a handful of PDFs
  (`forms/2026-28ADMs.pdf`, `Forms/CulEventStructure26-27-27-28.pdf`, `Officials/officials.html`,
  `golf/stategolf.html`, `tennis/statetennis.html`, `whsaa/boardminutes.html` are the only links on
  the homepage).
- **Browser requirement**: not tested beyond the 403s; the site is plain static HTML, so no JS
  dependency was observed on the pages that did return 200.
- **Request cost**: very low (homepage 21.5 KB; ADMs PDF 785 KB).
- **Published rate limits**: **no robots.txt** — `https://www.whsaa.org/robots.txt` returns
  **404 Not Found** (1094 B error page, `samples/robots-whsaa.txt`). No published policy; the lane
  kept 1 req/s.
- **Known blocks**: (a) `/schools/`, `/track/`, `/archives/` return **403 Forbidden** from the Plesk
  front end (directory URLs without a default document are refused rather than listed);
  (b) unknown paths return a 404 HTML error page with the server banner "Web Server at whsaa.org";
  (c) no member-school list, no AD/coach directory, no historical results archive.
- **Cross-source join keys**: school name; class (4A…1A); MileSplit Live meet id.
- **Estimated marginal coverage**: **70 schools across 4 classes** as the only enumeration figure
  (class-level, no names). Zero athlete records, zero coach records, zero result files reachable
  anonymously. This is the thinnest jurisdiction in the lane.
- **Implementation recommendation**: **REJECT** as a discovery/result/coach source in its current
  form. The one usable fact is the per-class school count from `forms/2026-28ADMs.pdf`; Wyoming
  school names and results must come from a different source (Wyoming state meets are published on
  MileSplit Live — see below — and MaxPreps holds team/roster pages).
- **Timing / result platform (measured, quoted)** — **the Athletic.net assumption is FALSE for the
  captured pages**:
  - `samples/whsaa-track-state.html` carries exactly one live-results link, labelled
    `[MILESPLIT LIVE RESULTS LINK] -> https://milesplit.live/meets/681838`
    (`grep -ci 'milesplit' samples/whsaa-track-state.html` = 1; `samples/whsaa-xc-state2.html` = 0).
  - `athletic.net` matches **zero** times in the WHSAA track page, the XC page and the homepage
    (`grep -ci 'athletic.net'` over `whsaa-track-state.html`, `whsaa-xc-state2.html`,
    `home-whsaa.html` = 0). Wyoming's state-meet result linkage observed here is **MileSplit Live**,
    not Athletic.net.
  - State-meet record documents are relative PDFs: `[STATE TRACK & FIELD CLASS RECORDS] ->
    ../archives/track/class.pdf`, `[STATE TRACK & FIELD OVERALL STATE RE…] -> ../archives/track/overallstate.pdf`.
  - NFHS/DragonFly are the compliance platforms linked from the homepage
    (`https://www.dragonflymax.com/`, `https://nfhslearn.com/`); officials register via
    `http://highschoolofficials.com/`.
- **Published Athletic.net / MileSplit links (markup quoted verbatim)**:
  - `samples/whsaa-track-state.html`:
    `<a href="https://milesplit.live/meets/681838" ...>MILESPLIT LIVE RESULTS LINK</a>` (label as
    rendered: "MILESPLIT LIVE RESULTS LINK").
  - Wyoming's state XC page publishes no external results link yet (empty `href=""` slots as quoted
    above).
