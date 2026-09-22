# 40. Missouri TRXC Timing deep map

Status: complete
Observed on: 2026-09-20

Scope of this report: deep map of **TRXC Timing** (the non-Athletic.net Missouri provider flagged in
report 22 §Recommendation, `research/midwest/22-missouri-primetime.md`), plus a link-extraction re-check
of the MSHSAA official result surfaces for links into TRXC or Athletic.net. All probes were anonymous
HTTP with a Chrome UA (`Mozilla/5.0 … Chrome/140.0.0.0 Safari/537.36`), sequential, ≥1.2 s apart,
61 requests to `trxctiming.com`, 3 to `liveresults.trxctiming.com`, 4 to `www.mshsaa.org`, 1 to
`archive.org` (429 → stopped). No `*.athletic.net` request was sent (mission constraint); all
Athletic.net IDs quoted below were read from page HTML or report 22.

---

### Source

**TRXC Timing, LLC** — Bridgeton, MO (12727 Carrollton Ind. Ct., Bridgeton MO 63044; published contact
`inquiry@trxctiming.com`, (314) 522-6176; footer of `/wp2/` and the Privacy Policy PDF). It is a
FinishLynx/IPICO-based timing company that is the **largest Missouri HS timer outside PrimeTime
Timing** (report 22 compared the two). Two surfaces matter, and only the first one is in scope:

| Host | Role |
|---|---|
| `trxctiming.com` (apex `301 → https://trxctiming.com/wp2/`) | WordPress site at `/wp2/` (Sydney Pro II theme, PHP 8.1/Apache, TablePress result tables, LiteSpeed Cache) **plus** a web-root static result tree (`https://trxctiming.com/<Meet_Dir>/…`) that is *not* WordPress — raw Apache-served Hy-Tek output |
| `liveresults.trxctiming.com` | Create-React-App SPA ("TIMING | Live Results"), client-rendered; used only during live meets. Not needed for static ingestion and not probed beyond identifying it |

Discovery of the host: `https://trxctiming.com/` → `301` to `https://trxctiming.com/wp2/` (WordPress
`x-redirect-by: WordPress` header) → 200, 87,065 B, whose nav exposes exactly five sections: Home,
Live Results, Upcoming Meets, Results, Services, Contact. The Results hub
(`/wp2/results/`) is a link page pointing at the two per-season index trees plus the live SPA.

Compliance-relevant observations (all recorded in the appendix):

- `robots.txt` → **404 on both `trxctiming.com` and `www.trxctiming.com`** (the 404 body is the themed
  WP "Page not found", 52 KB). **No sitemap** (`/wp2/wp-sitemap.xml` → 404), no `X-Robots-Tag`, no
  AI-crawler directives. The site's RSS (`/wp2/feed/` → 200, 883 B) is an **empty post feed** (0
  `<item>` entries), not a results change feed.
- The only policy document found is the 2018 Privacy Policy PDF (`/TRXC_Timing/Privacy_Policy/Privacy_Policy.pdf`,
  `last-modified: 2018-12-17`), which covers **event registration data only** (registration data,
  Stripe payment data, under-13 rule). It contains no prohibition on automated reading of the results
  pages and no republication/recruiting clause of the kind found on the Karmarush live platform
  (report 22 §Access characteristics). Absence of a ToU is **not** a licence — see Recommendation.

### Coverage

Geographic: **Missouri** is the core market (St. Louis metro conferences, MSHSAA districts/sectionals,
mid-MO, the Bootheel, Springfield-area), with a **secondary Illinois (SW IL) footprint** encoded as a
`/Illinois/…` path prefix, occasionally college meets (GLVC, SLIAC, UCM, SIUC, Illinois College,
Principia, Greenville, SBU, UIS, Culver-Stockton) and club/racewalk/senior/youth meets. All meets are
reached from two per-season WordPress tables; the meet-type column is the filter.

Season corpus, counted from the live index tables on 2026-09-20 (rows = meet rows; "IL" = result URL
under `/Illinois/`; "non-IL" = the Missouri/in-scope remainder):

| Season page | Rows | IL rows | non-IL rows | Date span in table | Meet types observed |
|---|---|---|---|---|---|
| Track & Field 2026 | **166** | 34 | **132** | 8/29/26 → 1/23/26 (season in progress at observation) | HS Outdoor 117, MS Outdoor 18, College Outdoor 12, College Indoor 8, HS Indoor 4, HS/MS Outdoor 1, Youth 1, Club 2, Racewalk 2, Senior 1 |
| Track & Field 2025 | **146** | 35 | **111** | 12/05-06/25 → 1/24/25 | HS Outdoor 101, MS Outdoor 17, College Outdoor 10, College Indoor 10, HS Indoor 5, Senior 1, Para 1, Youth 1 |
| Track & Field 2024 | **160** | 33 | **127** | 12/06/24 → 1/19/24 | HS Outdoor 107, MS Outdoor 22, College Outdoor 11, College Indoor 9, HS Indoor 5, HS Indoor/Outdoor 2, Youth 3, Para 1 |
| Cross Country 2026 (YTD) | **17** | 2 | **15** | 9/19/26 → 8/29/26 | HS XC 11, High School 3 (label variant), College XC 3 |
| Cross Country 2025 | **64** | 9 | **55** | 11/14/25 → 8/29/25 | High School 47, College 12, Middle School 5 |
| Cross Country 2024 | **64** | 9 | **55** | 11/09/24 → 8/30/24 | High School 51, College 8, Middle School 5 |

Historical depth:

- **WordPress season pages**: T&F 2014–2026 (13 pages: `2014…2026-track-and-field-results`), XC 2014
  and 2016–2026 (12 pages; no `/2015-cross-country-results/` page exists, but 2015 XC files exist on
  disk — e.g. the Festus `menu.htm` links `/Festus/XC/Bowles/Results/2015/2015_results_girls_varsity.htm`).
  Enumerated from one REST call (see Stable identifiers).
- **Per-meet multi-year history on disk**: the T&F file for a recurring meet is
  `<Meet_Dir>/Results/<YYYY>.htm`, and older years stay live. Verified for Parkway Central Henle Holmes:
  `2014.htm` (109,800 B), `2018.htm` (129,879 B), `2021.htm` (53,065 B), `2026.htm` (109,778 B) all
  return 200. XC uses per-year directories `<Meet_Dir>/Results/<YYYY>/<race>.htm`; the Festus meet's
  `menu.htm` links **76 races across 13 seasons (2014–2026)**.
- **MSHSAA postseason**: TRXC times MSHSAA **district and sectional** T&F meets (2026 index carries 9
  MSHSAA rows: C4D5, C3D3, C2D2_C3D2 districts; C4S1_C5S1, C4S2_C5S2, C2S1_C3S1 sectionals; plus 2 para
  sectionals) and **MSHSAA XC districts** (`/MSHSAA_XC/…`). The MSHSAA **state** finals are not TRXC's:
  report 21 verified the state PDFs are licensed to PrimeTime Timing, and TRXC has no state-meet row.
- **MSHSAA XC district archive**: a single page, `https://trxctiming.com/MSHSAA_XC/menu.htm` (108,153 B),
  links **650 race files spanning 2010–2025** (boys 325 / girls 325; 64 files for 2010 at the archive
  root, ~64/yr 2011–2018, 32 in 2019, 18 in 2020, 4 in 2021–2024, 8 in 2025). This is the deepest single MSHSAA postseason artifact found in
  this campaign that is not a PDF.
- School levels: HS + middle/JH + college + open/club. The grade column distinguishes them (6–8 MS,
  9–12 HS, FR/SO/JR/SR college).

### Enumeration

Everything is reachable without search, pagination, or JS. Exact patterns:

1. **Site page inventory (1 request)**: `GET https://trxctiming.com/wp2/wp-json/wp/v2/pages?per_page=100&page=1&_fields=id,slug,link,title,parent,modified`
   → `X-WP-Total: 54`, `X-WP-TotalPages: 1`. This returns every WordPress page with its canonical link
   and `modified` date, i.e. the full season-page map (e.g. page `18071` = `/wp2/track-and-field/track-and-field-results/2026-track-and-field-results/`,
   page `18590` = `/wp2/cross-country/past-results/2026-cross-country-results/`).
2. **T&F season index (1 request/season)**: `GET https://trxctiming.com/wp2/track-and-field/track-and-field-results/<YYYY>-track-and-field-results/`
   → one server-rendered TablePress table, **no pagination**: columns `Date | (blank) | Meet Name | Meet Type | Results | Timing`.
   The `Results` cell holds one anchor per meet; 166 rows in the 2026 fetch (178,403 B).
3. **XC season index (1 request/season)**: `GET https://trxctiming.com/wp2/cross-country/past-results/<YYYY>-cross-country-results/`
   → 5-column table `Date | Meet Name | Meet Type | Results | Timing`; 17 rows for 2026 (65,484 B), 64
   for 2025/2024.
4. **Per-meet result files**: T&F link → `https://trxctiming.com/<Meet_Dir>/Results/<YYYY>.htm` (sometimes
   `results/2026.htm` lower-case, or a one-off name like `class_4_sectional_1_results.htm`). XC link →
   `…/Results/index.htm`, a 3-frame frameset (`header.htm`, `menu.htm`, `main.htm`); the real content is
   behind `menu.htm`, which lists every race file for every season of that meet.
5. **Upcoming meets (1 request)**: `GET https://trxctiming.com/wp2/upcoming-meet-information/` → 40 rows,
   columns `Date | Meet Name (Online Entry link) | Registration Opens | Registration Closes | Meet Type | Meet Contact | More Info`.
   Spot-checked: 39 of 40 rows carry an Athletic.net entry/meet link (see Athletic.net leverage) and 38
   of 40 rows carry a `Meet Info` PDF such as `https://trxctiming.com/Festus/XC/Bowles/Info.pdf`.
6. **Directory listing**: **disabled**. `…/MSHSAA/2026_TF/Sectional/C4S1_C5S1/Results/` → `403 Forbidden`
   (13 B). Requesting an XC `…/Results/` directory returns the frameset because `DirectoryIndex` maps to
   `index.htm` (200, 1,250 B), not a listing. Sibling files are therefore discoverable only through the
   WP index or `menu.htm`.
7. Not enumerable anywhere: schools, teams, athletes, coach/AD contacts. There is no school or athlete
   index on the site (route inventory: `/wp2/` pages above, plus the static tree). Also note the T&F
   postseason has **no** archive page — `https://trxctiming.com/MSHSAA/menu.htm` → 404 and
   `/MSHSAA/index.htm` → 200 with **0 bytes** — so MSHSAA T&F history is reconstructed from the per-season
   WP tables only.
8. Class of 2027: enumerable **only inside a result file**, by the `Year` column (`11`), exactly as
   report 22 found for PrimeTime. No site-level grade filter exists.

### Stable identifiers

| Entity | What exists | Quality / caveats |
|---|---|---|
| Meet | **No id field anywhere.** De-facto key = the static directory path, e.g. `/Parkway_Central/TF/Henle_Holmes/` or `/MSHSAA/2026_TF/District/C2D2_C3D2/` (160 distinct dirs across the 166-row 2026 T&F index — a few conferences contribute several rows: Suburban Conference appears 5×, Lutheran North 4×, etc.) | Stable across seasons for recurring meets; new meets get new slugs. Names are inconsistent by era (`North_Callaway/Track/…` vs `…/TF/…`) |
| Season | WP page slug `<YYYY>-track-and-field-results` / `<YYYY>-cross-country-results`; WP page id (`18071`, `18590`, …) | Stable, machine-readable via WP REST |
| Result file | Full URL is the identifier. T&F: `…/Results/<YYYY>.htm` (immutable per season, but **rewritten in place** during a season). XC: `…/Results/<YYYY>/<race>.htm` | ETag + Last-Modified present; 304 works (verified). XC race filenames vary by era (`varsity_girls.htm` 2019+, `girls_varsity.htm` 2018, `2017_results_girls_varsity.htm` ≤2017) → never guess, read `menu.htm` |
| Athlete | **None.** Zero `AthleteID` / `data-*` / HTML comments across 8 files scanned (Henle 2026, MSHSAA sectional, Suburban, C2D2/C3D2, CMAC, Henle 2021, Henle 2018, Festus XC race) | Identity resolution must be `(last, first) + school + grade` per file; Hy-Tek school labels are abbreviations ("Parkway C.", "F. Tolton", "F.Z.S.", "N.D. (C.G.)") needing alias resolution |
| Result / Event | **None global.** Hy-Tek `Event NN` numbers are per-meet only (`Event 2 (B) High Jump Varsity`, `Event 2 (G) 5k Run CC Varsity`); MSHSAA compiled files drop the event number entirely and use titles like `Girls 100 Meter Dash C4` | Not usable as cross-meet keys |
| Athletic.net MeetID | Appears **only on the upcoming-meets page** as part of the AN URL (37 meet links observed); **absent** from every result index and result file | For past meets the only join keys are (meet name, date, venue/host from the Hy-Tek header) |

### Athletic.net leverage

- **Upcoming meets page = an explicit TRXC → Athletic.net MeetID feed.** 39 Athletic.net anchors sit in
  39 of the 40 rows (37 are `/meet/<id>/…` links — 36 `/info` and one `/TrackAndField/meet/670780/register`
  — and 2 rows, Metro Invite and Cahokia Conference, carry a bare `https://www.athletic.net/` link with no
  meet id). The 37 meet IDs, verbatim from the page:
  CrossCountry — `259371, 273452, 273453, 275074, 275077, 276216, 276575, 276588, 276695, 276698,
  276700, 276701, 276702, 276705, 276706, 276707, 276708, 276710, 276711, 276712, 276713, 276714,
  276716, 276717, 276720, 276721, 276725, 276726, 276727, 276728, 276729, 277112, 277477, 281322,
  284465, 284647` (36); TrackAndField — `670780` (1 `/register` link).
  Example row: `Festus Bowles-Wright Invitational, 9/19/26, HS XC → https://www.athletic.net/CrossCountry/meet/276695/info`.
  One row uses DirectAthletics instead (`Greenville XC Classic, 10/16/26 → https://www.directathletics.com/meets/xc/28263.html`).
- **Result index pages carry zero Athletic.net links** (checked `tf2026`, `tf2025`, `xc2026` main content:
  only `i0.wp.com` image proxies and one absolute self-link). So the AN MeetID feed covers *future*
  meets only; it is a week-ahead discovery surface, not a historical lookup.
- **No AN team/athlete/result links exist** anywhere on the site (no such routes; anchor scans of the
  index, hub, upcoming, and result pages show only trxctiming.com, AN entry links, a DirectAthletics
  link, and social).
- **Reverse direction (MSHSAA re-check)**: `www.mshsaa.org` never links TRXC.
  - `/Content/TrackandField/InformationCentral.aspx` (200, 33,314 B): exactly two provider anchors —
    `Live Results - PrimeTime Timing → https://www.pttiming.com/` and
    `Official Results - AthleticNet → https://www.athletic.net/track-and-field-outdoor/usa/high-school/missouri/mshsaa`.
  - `/Content/CrossCountry/InformationCentral.aspx` (200, 37,143 B): **no external provider links at all**;
    the Championship Results block links internally to `PostseasonResult.aspx?alg=11|12&class=1..5&year=2025`.
  - `/Content/TrackandField/Home.aspx` (200, 31,022 B): links `State Championship Results` →
    `PostseasonResult.aspx?alg=52|53&year=2026`, `Sectional Info` → `SectionalResults.aspx?…`, and a
    PDF `resources/Activities/CrossCountry/Athletic.net%20helpful%20links.pdf` (MSHSAA promoting AN to
    schools). **No `trxctiming` link on any of the three pages.**
  - Consequence: TRXC's MSHSAA postseason coverage is discoverable *only* from TRXC itself (its own
    season indexes), and MSHSAA's own postseason results remain AN-linked T&F PDFs (report 21).
- **Seeding estimate.** For the 132 non-IL T&F meets of 2026 and 55 MO XC meets of 2025, there is no AN
  link; the deterministic seed is `(meet name, meet date, host/venue)` from the Hy-Tek header plus the
  grade column, which is already enough to (a) confirm Class-of-2027 independent of AN and (b) narrow
  an AN meet search to one candidate. Request avoidance versus per-athlete AN work: **1 static file GET
  replaces N profile fetches per meet** (measured: 500 grade-tagged athlete-lines in the Henle file, 468
  in the MSHSAA C4S2/C5S2 file, 883 in C2D2/C3D2) — on the order of 130 + 55 file requests per season
  to cover the MO corpus [INFERENCE for the season total; per-file counts are measured].

### Athlete evidence

Measured on 17 files fetched 2026-09-20 (11 files sampled uniformly from the 2026 T&F index + the 6
named files analysed in depth). Parser grammar used: place line `^\s*<place>\s+<name>\s{2,}<Year>\s+<School>\s{2,}<mark…>`
(2+ spaces before the grade, exactly one space after it — verified from raw bytes:
`'  1 Johnson, Jaidyn           11 Central (C.G.)           1.95m    6-04.75  10   '`).

| Field | Availability |
|---|---|
| Name | Yes — Hy-Tek "Last, First" (`Johnson, Jaidyn`, `Archambeault, Kelly`) |
| Grade | Yes — `Year` column. HS 2019+ = numeric `9/10/11/12`; the 2014–2018 era prints `SR/JR/SO/FR` (verified in the 2018 Henle file, `Johnson, Cara SR`; layout there differs — the centred name field is followed by the Year and only one space precedes the school: `  1       Johnson, Cara       SR M.I.C.D.S.   12.03   4  10`, 749 parsed lines = JR 259 / SR 239 / SO 166 / FR 85); MS meets print `6/7/8` (Sikeston JH file: 25×6, 71×7, 101×8); **college meets reuse the same column with FR/SO/JR/SR** (GLVC 2026: FR 343, SO 292, JR 300, SR 230) — so a naive "JR = grade 11" rule would miscount college rows |
| School | Yes — Hy-Tek short label in the same line (`Central (C.G.)`, `F. Tolton`, `F.Z.S.`, `Lafayette (W)`, `Nixa`) |
| City/state | **No** per athlete. Only the meet header carries a venue (`Larry Crites Park`, `Orchard Farm HS`, `Parkway Central High School`) |
| Gender / category | Yes — event titles carry `(B)/(G)` or `Boys/Girls`, plus class suffixes in MSHSAA files (`Girls 100 Meter Dash C4`) |
| TF/XC, indoor/outdoor | Meet-level only (the WP index `Meet Type` column and the file header date/name) |
| Performances | Yes — finals marks with thousandths where FAT recorded (`12.83 … 12.821`), field series (`1.65 1.75 1.80 … XXX`), XC times `18:53.55` |
| PRs / progression | **Not directly.** No athlete pages. Per-meet multi-year archives make manual progression possible (fetch `menu.htm` / `<YYYY>.htm` for prior seasons) but there is no PR field |
| Meets | One row per meet; the file header names the meet, venue and date |
| Athlete profile URL | **None** |

**Class-of-2027 yield (measured, not estimated).** A "grade-11 line" below = one printed Hy-Tek line
whose Year column is `11` (or `JR` in the pre-2019 class-code era); Hy-Tek prints two relay legs per
leg-line, so a relay leg-line counts once. Grade-11 row counts per file: Parkway Central Henle
Holmes **228**; MSHSAA C4S2/C5S2 sectional **240**; MSHSAA C2D2/C3D2 district **276**; CMAC conference
**225**; Suburban Green Girls **119**; sample10 HS files 165 (C3D3 district), 135 (Louisiana invite),
134 (Cunningham invite), 72 (North Callaway), 191 (Lindbergh), 148 (Fort Zumwalt West); Festus XC
varsity girls **18**. Total across the 11 HS T&F files: **1,933 grade-11 lines** (mean ≈176/file,
range 72–276); the one XC race file adds 18 [season-wide totals are INFERENCE: 132 MO T&F files × ~176 ≈
23k grade-11 *lines*, before de-duplicating athletes who appear at several meets]. Re-derived
independently from the same raw captures (`evidence/gaps/40/strict-stats.json`): the result-line counts
reproduce exactly for all six named files (500 / 468 / 424 / 883 / 700 / 59), and the grade-11 totals
agree within 2% (e.g. Henle 233 vs 228) — the residual is the attribution of a leg-line's single grade
to one of the two legs printed on it, which does not change file-level magnitudes.

### Recruiting information

**No coach or AD directory exists.** Checked: the site's five content sections, the results hub, both
season-index trees, result-file headers, and the WP page inventory (54 pages: About, Personnel,
Contact, Services, Jobs, Equipment, Results trees — no school pages).

The upcoming-meets table does publish a **Meet Contact** per meet: 38 of the 40 rows carry a named
person with a `mailto:` role address (column heading "Meet Contact"; 38 addresses, 32 distinct, on 29
domain strings = 28 distinct third-party domains after collapsing one `mexico.k12.mo.us/` typo, of
which 2 are `gmail.com`; the remaining address is TRXC's own `trxctiming.com` contact). Every
non-gmail domain is a school-district, school or college domain.

This is public, role-published *meet-director* contact information — not coach or AD information, and
not to be repurposed for recruiting outreach. No emails are reproduced in this report; the count and
domain shape are the finding.

### Result evidence

| Field | Availability (TRXC Hy-Tek output) |
|---|---|
| ResultID | **No** |
| AthleteID | **No** |
| MeetID | No numeric id; de-facto = directory path + `<YYYY>.htm`. Athletic.net MeetID only via the upcoming page |
| EventID | Per-meet Hy-Tek `Event NN` in MM-printed files; **absent** in the MSHSAA compiled layout (titles only) |
| Mark | Yes — `Finals` column; field marks often dual-unit (`1.95m  6-04.75`); XC `18:53.55` |
| Normalized mark inputs | Partial — thousandths where FAT exists (`12.83 … 12.821`); no wind/altitude adjustment; no explicit FAT/hand flag |
| Timing method | Implicit: `HY-TEK's Meet Manager` header, `Licensed to TRXC Timing, LLC - Contractor License`, `Entries and Results by: TRXC Timing`; TRXC is a FinishLynx/IPICO shop (site footer links) → FAT [INFERENCE, not a field in the file] |
| Wind | **No.** Zero `Wind` occurrences in the 2026 Henle file, the MSHSAA sectional file, and the 2026 XC race file; the only hit seen anywhere is the pre-2019-era mark annotation `Wind Aided` in the 2018 Henle file (a status flag, not a wind speed/reading — no wind column exists in any sampled file). (PrimeTime's compiled output *does* carry wind; TRXC does not) |
| Implement / hurdle specification | **Not observed** in any sampled file (no implement weights, no hurdle heights) |
| Heat / round | Yes — `H#` column when multiple heats exist (Henle 2026: 17 `H#` occurrences); `Finals` / `Section N` headers; sectional files are finals-only |
| Place | Yes, incl. `--` rows with `DNF`/`DNS`/`DQ`/`NH`/`FOUL` states |
| Date | Yes — header `Parkway Central Henle Holmes Invite - 4/16/2026 to 4/17/2026`; Hy-Tek build timestamp `4/17/2026 08:37 PM` |
| School represented | Yes (Hy-Tek label) |
| Relay membership | Yes — numbered legs with per-leg grade: `1) Lee Hughes 9  2) Devin Ferguson 11  3) Xzavir Jones 11  4) Kinyon Johnson 12`; leg lines counted separately (226 legs in the Henle file, 254 in the MSHSAA sectional) |

Format census: **11 of 11** sampled 2026 files are Hy-Tek text output embedded in a `<pre>`-style HTML
wrapper (f01 Club, f02–f10 HS/MS/college, f11 college indoor) — i.e. **no PDFs on this path at all**,
unlike PrimeTime 2026 (report 22: 1,349 PDFs vs 387 HTML). The only PDFs found are per-meet `Info.pdf`
handouts on the upcoming page.

### Incremental use

Design (all mechanisms verified live):

1. **New meets** — weekly `GET` of the two current-season index pages (2 requests). Diff the extracted
   row set on `(Date, Meet Name, Results URL)`. The WP pages return **no `ETag`, no `Last-Modified`, no
   `Cache-Control`** (verified on both index URLs), and the WP `modified` field does **not** track table
   edits (the 2026 XC page shows `modified: 2026-07-30` while its table already lists a 2026-09-19 meet —
   TablePress data is stored outside the page body), so the change token must be a content hash of the
   parsed rows, not a header or REST timestamp. A full-page byte hash also works (65–180 KB, no
   pagination); extract-then-hash avoids churn from cache-decorating markup.
2. **Changed / re-published files** — every static result file carries `ETag` + `Last-Modified`
   (Apache). Revalidate with `If-None-Match`; verified: `If-None-Match: "b412ad-1acd2-64fb21d9f0e00"`
   on the Henle file → **`304`, 0 bytes**. T&F files are rewritten in place as a meet is finalised
   (same URL, new ETag), so a changed ETag on an existing row is the "meet updated" signal. XC race
   files likewise (`last-modified: 2026-09-19 14:01:38` for a meet run that day).
3. **XC season growth** — for a known XC meet, `menu.htm` changes when a new season directory appears
   (`Last-Modified: 2026-09-17 17:28:41` on the Festus menu, i.e. when the 2026 races were added).
   Weekly: 1 conditional GET per known XC meet (≈55) instead of touching every race file.
4. **Affected athletes** — no athlete entity exists, so there is no athlete-level delta. A changed file
   must be re-parsed; identity diffs are then done by the consumer on `(<name>, school, grade)`.
5. **Cost** — a weekly MO refresh ≈ **2 index GETs + ~190 conditional GETs** (132 T&F + ~60 XC files,
   plus per-meet `menu.htm` for XC), almost all 304s; only genuinely new/changed files transfer bytes.
   Nothing requires a historical re-fetch; older `<YYYY>.htm` files can be revalidated monthly instead
   of weekly once their season is over.
6. **What does *not* work**: no sitemap (`/wp2/wp-sitemap.xml` → 404) and no results RSS (the only feed,
   `/wp2/feed/`, is an empty post feed — 883 B, 0 items, verified); no `updated` field in the WP tables;
   and no directory listing to enumerate files without the index (403).

### Access characteristics

- **Classification: static HTML result corpus (Hy-Tek ASCII wrapped in HTML) + normal HTML index pages
  (WordPress) + undocumented public JSON (WP REST v2) + downloadable PDF handouts.** The live surface
  (`liveresults.trxctiming.com`) is a **browser application** (React SPA, `main.6419a799.chunk.js`
  references only its own origin + `trxctiming.com` in the main bundle; data path not determined and not
  needed here). No authentication, no paywall, no CAPTCHA, no special headers — plain anonymous GETs;
  the **default `curl` UA also returns 200** on the result/index pages (re-verified: 65,483 B on the
  2026 XC index), so there is no UA gate here (contrast `www.mshsaa.org`, where report 21 saw
  browser-like UAs reset at the edge on 2026-09-19; this run's four Chrome-UA MSHSAA GETs all returned
  200, so that gate is at least intermittent). The static result tree also sends
  `access-control-allow-origin: *` with `GET, OPTIONS` allowed (observed on the 2018 file), so the files
  are readable from a browser context without page-level proxying.
- **Observed limits**: ~61 sequential requests to `trxctiming.com` (≥1.2 s apart) and 3 to
  `liveresults.trxctiming.com` produced **zero 429s and zero `Retry-After` headers**. No published rate
  limit found (no robots.txt, no terms page on the results host). `archive.org` returned **429** on the
  single wayback availability probe → that host was abandoned immediately (recorded, not retried).
- **Caveats**: no ToU exists for the results tree, so systematic weekly collection of the whole corpus
  is a judgement call, not a documented permission; the conservative path is a licensing inquiry to
  `inquiry@trxctiming.com` (the company also publishes a Privacy Officer contact). The 2018 privacy
  policy is registration-scoped and silent on result reuse. Also note the WP 404 page is returned with
  HTTP 200 for some paths (e.g. `/MSHSAA/index.htm` → 200/0 bytes), so status codes alone are not a
  reliable existence check.
- **Not probed / out of scope**: anything requiring login (none needed), the live SPA's data endpoints,
  and `www.trxctiming.com` beyond robots/PDF (apex serves the same content; one index row even links
  `http://www.trxctiming.com/GLVC/TF/Outdoor/Results/2026.htm`).

### Recommendation

**RESULT-SOURCE** (primary classification) — with **VALIDATION** as a secondary role for the grade
column, and **DISCOVERY-ONLY** for the upcoming-meets page's Athletic.net IDs.

**Verdict on the assignment question: yes, TRXC is a viable grade-bearing Missouri result source.**
Concretely:

- It publishes **Machine-readable, grade-tagged results for the whole MO HS corpus it times** — 132
  non-IL T&F meets in 2026, 111 in 2025, 127 in 2024, plus 55 MO XC meets per season, plus MSHSAA
  districts/sectionals and a 2010–2025 MSHSAA XC district archive (650 files) — as plain Hy-Tek text
  files, with `Year = 11` directly encoding Class of 2027 (1,933 grade-11 lines measured across 11
  sampled HS files). No PDF/OCR path is needed at all.
- It is **not** an identity source: no athlete/team/result/meet IDs, no profile pages, no city/state,
  no wind, no implement spec. Athlete identity must be resolved on `(name, school label, grade)` with
  Hy-Tek school abbreviations aliased (the campaign's `data/school-alias-map.csv` work is the right
  dependency).
- It carries **Athletic.net MeetIDs only for upcoming meets** (37 AN links today, XC-heavy), which makes
  it a worthwhile *week-ahead* feed for AN meet discovery during the fall, and a DirectAthletics
  alternative signal in one case. Historical AN joins must be seeded by `(meet name, date)`.
- Marginal coverage over Athletic.net is the mid-major MO market that AN only reflects through uploaded
  files: St. Louis metro conferences (Suburban, GAC, Metro League, CMAC per report 22), MSHSAA
  districts/sectionals, and the Bootheel/Springfield circuit. For these, TRXC yields per-meet file GETs
  instead of per-athlete profile fetches, and independent grade confirmation.
- Practical caveats for the collector: filter `Meet Type` (Club meets print **Age**, not `Year`), filter
  college rows (FR/SO/JR/SR ≠ HS grades), tolerate two grade-encoding eras (numeric vs class-year codes
  pre-~2019), and re-read `menu.htm` each season for XC (race filenames are not stable across eras).
- The one open question that could change this from RESULT-SOURCE to REJECT is licensing: the company
  publishes no terms for the results tree. Nothing observed (no robots.txt, no AI-crawler blocks,
  privacy policy silent on results reuse) forbids reading these pages, but a licence inquiry is
  advisable before the pipeline depends on weekly corpus sweeps.

---

### Evidence appendix

Timestamps are 2026-09-20 America/Chicago (CDT). Bracketed values are exact (from the command or the
response `Date` header); unbracketed are the probe minute within the observation window 09:05–09:13.
All requests: `curl`, Chrome UA unless noted, `-L` (redirects followed), sequential with ≥1.2 s gaps.
No `*.athletic.net` host was contacted at any point.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://trxctiming.com/` | GET | 301 → `https://trxctiming.com/wp2/` → 200 (87,065 B) | real home is the WordPress `/wp2/` site; nav exposes Live Results / Upcoming Meets / Results / Services / Contact | 09:05 [14:05:02 GMT] |
| `https://trxctiming.com/wp2/wp-json/` | GET | 200 (466,044 B) | WP REST v2 is public; route inventory (no custom result API) | 09:05 |
| `https://trxctiming.com/robots.txt` | GET | 404 (52,301 B) | no robots.txt; themed WP 404 body | 09:05 |
| `https://www.trxctiming.com/robots.txt` | GET | 404 (52,301 B) | same on `www` | 09:07 |
| `https://trxctiming.com/wp2/wp-json/wp/v2/pages?per_page=100&page=1&_fields=id,slug,link,title,parent,modified` | GET | 200, `X-WP-Total: 54`, `X-WP-TotalPages: 1` | full page inventory in 1 request: T&F season pages 2014–2026, XC 2014+2016–2026, Results hub, Upcoming Meets, Personnel, Contact, Jobs, Privacy link | 09:05 |
| `…/wp2/track-and-field/track-and-field-results/2026-track-and-field-results/` | GET | 200 (178,403 B) | 2026 T&F index: 166 meet rows (34 IL / 132 non-IL), types + one `.htm` file link per row | 09:05 |
| `…/wp2/track-and-field/track-and-field-results/2025-track-and-field-results/` | GET | 200 (163,194 B) | 2025 T&F index: 146 rows (35 IL / 111 non-IL) | 09:06 |
| `…/wp2/track-and-field/track-and-field-results/2024-track-and-field-results/` | GET | 200 (173,650 B) | 2024 T&F index: 160 rows (33 IL / 127 non-IL) | 09:09 |
| `…/wp2/cross-country/past-results/2026-cross-country-results/` | GET | 200 (65,484 B) | 2026 XC index: 17 rows (15 MO), all links `…/Results/index.htm` | 09:05 |
| `…/wp2/cross-country/past-results/2025-cross-country-results/` | GET | 200 (89,547 B) | 2025 XC index: 64 rows (55 MO); 4 MSHSAA district rows share `/MSHSAA_XC/index.htm` | 09:06 |
| `…/wp2/cross-country/past-results/2024-cross-country-results/` | GET | 200 (89,784 B) | 2024 XC index: 64 rows (55 MO) | 09:09 |
| `…/wp2/upcoming-meet-information/` | GET | 200 (85,640 B) | 40 upcoming meets; 39 Athletic.net anchors / 37 `/meet/<id>/` links (36 `/info`, 1 `/register`); 38 `Meet Info` PDFs; 38 meet-director name+email contacts; 1 DirectAthletics link | 09:05 |
| `…/wp2/results/` | GET | 200 (69,275 B) | Results hub = links to the 2 season trees + live SPA | 09:07 |
| `https://trxctiming.com/Parkway_Central/TF/Henle_Holmes/Results/2026.htm` | GET | 200 (109,778 B); `etag "b412ad-1acd2-64fb21d9f0e00"`, `last-modified 2026-04-18` | T&F result file = Hy-Tek ASCII in `<pre>`; `Year` column 9–12; 500 graded lines, 226 relay legs, 228 grade-11 lines; no Wind; `H#` heats present | 09:05 [14:05:50 GMT] |
| `https://trxctiming.com/Parkway_Central/TF/Henle_Holmes/Results/2026.htm` (repeat) | GET `If-None-Match: "b412ad…"` | **304**, 0 B | ETag revalidation works → zero-byte weekly re-checks | 09:10 |
| `https://trxctiming.com/Parkway_Central/TF/Henle_Holmes/Results/2021.htm` | GET | 200 (53,065 B) | 2021 era already uses numeric grades (409 numeric-grade lines, 1 SR/JR token) | 09:08 |
| `https://trxctiming.com/Parkway_Central/TF/Henle_Holmes/Results/2018.htm` | GET ×2 (09:07, 09:18 re-check) | 200 (129,879 B both times); `last-modified: Fri, 13 Apr 2018 03:12:04 GMT`, `etag "b40947-1fb57-569b23c850900"`, `access-control-allow-origin: *` | 2018 era prints `SR/JR/SO/FR` in the Year column (`Johnson, Cara SR`), centred name field, 749 parsed lines (JR 259 / SR 239 / SO 166 / FR 85); only `Wind Aided` annotation, no wind column; per-meet T&F history reaches at least 2014 at a stable URL | 09:07 / 09:18 [14:18:08 GMT] |
| `https://trxctiming.com/Parkway_Central/TF/Henle_Holmes/Results/2014.htm` | GET | 200 (109,800 B) | per-meet T&F history reaches at least 2014 at a stable URL | 09:07 |
| `https://trxctiming.com/MSHSAA/2026_TF/Sectional/C4S2_C5S2/Results/2026.htm` | GET | 200 (96,631 B) | MSHSAA sectional compiled layout: `Girls 100 Meter Dash C4`, `Name Year School Seed Finals Points`; 468 graded lines, 254 legs, 240 grade-11 | 09:05 |
| `https://trxctiming.com/MSHSAA/2026_TF/Sectional/C4S1_C5S1/Results/` (directory) | GET | **403** (13 B) | Apache directory listing disabled | 09:07 |
| `https://trxctiming.com/Festus/XC/Bowles/Results/` (directory) | GET | 200 (1,250 B) | returns the frameset `index.htm` via DirectoryIndex, not a listing | 09:07 |
| `https://trxctiming.com/Festus/XC/Bowles/Results/index.htm` | GET | 200 (1,250 B); `last-modified 2022-10-06` | XC entry file is a 3-frame frameset (header/menu/main) | 09:05 [14:05:52 GMT] |
| `https://trxctiming.com/Festus/XC/Bowles/Results/menu.htm` | GET / HEAD | 200 (13,105 B); `last-modified 2026-09-17` | 76 links = every race × 13 seasons (2014–2026); LM moves when a season is added → XC change token | 09:06 / 09:10 |
| `https://trxctiming.com/Festus/XC/Bowles/Results/2026/varsity_girls.htm` | GET / HEAD | 200 (8,306 B); `last-modified 2026-09-19 14:01:38` | XC race file: `Event 2 (G) 5k Run CC Varsity`, `Name Year School Finals Points`; 59 graded lines, 18 grade-11 | 09:06 / 09:10 |
| `https://trxctiming.com/Hickman/XC/COMO_Kick-off/Results/menu.htm` | GET | 200 (3,388 B) | menu pattern generalizes (2023–2026, `hs_boys.htm`/`hs_girls.htm`) | 09:08 |
| `https://trxctiming.com/MSHSAA_XC/index.htm` | GET | 200 (1,154 B) | shared MSHSAA XC district frameset | 09:08 |
| `https://trxctiming.com/MSHSAA_XC/menu.htm` | GET | 200 (108,153 B) | **650 MSHSAA XC district race files, 2010–2025** (325 boys / 325 girls; 64 for 2010) — deepest non-PDF postseason artifact found | 09:08 |
| `https://trxctiming.com/MSHSAA/menu.htm` | GET | 404 (52,306 B) | no T&F postseason archive page | 09:12 |
| `https://trxctiming.com/MSHSAA/index.htm` | GET | 200, **0 bytes** | empty file; status code alone is not an existence check | 09:12 |
| `https://www.trxctiming.com/TRXC_Timing/Privacy_Policy/Privacy_Policy.pdf` | GET | 200 (69,075 B, `application/pdf`) | only policy doc = 2018 registration/privacy text; no crawling/reuse clause | 09:06 [14:06:59 GMT] |
| `https://trxctiming.com/Suburban_Conference/TF/Green/Girls/Results/2026.htm` | GET | 200 (78,340 B) | conference file: 424 graded lines, 119 grade-11 | 09:09 |
| `https://trxctiming.com/MSHSAA/2026_TF/District/C2D2_C3D2/Results/2026.htm` | GET | 200 (160,993 B) | district file: 883 graded lines, 276 grade-11 (largest single file sampled) | 09:09 |
| `https://trxctiming.com/CMAC/TF/HS/Results/2026.htm` | GET | 200 (131,068 B) | conference file: 700 graded lines, 225 grade-11 | 09:09 |
| 11 files sampled across the 2026 T&F index (Run It Back #2, MSHSAA C3D3, GLVC, Louisiana Invite, Cunningham Invite, Sikeston JH, North Callaway, UCM Mule Relays, Lindbergh Flyer, Fort Zumwalt West, SLIAC Indoor) | GET | all **200** (10,076–291,906 B) | **11/11 are Hy-Tek text**; 10/11 have the `Name … Year … School` header (the Club meet prints `Age`, e.g. `36`, `45`); grades 6–8 MS, 9–12 HS, FR/SO/JR/SR college; aggregate 1,933 grade-11 HS lines (see Athlete evidence) | 09:10–09:11 |
| 5 XC `menu.htm` files (Festus, Lutheran North Crusader-Greyhound, Carbondale, Mexico Fall, Clayton Classic) | GET | all **200** (2,821–13,105 B) | menu pattern holds for every sampled 2026 XC meet; per-meet history depth 2–13 seasons | 09:11 |
| `liveresults.trxctiming.com/` | GET | 200 (3,023 B) | React SPA shell (CRA build), JS required | 09:06 |
| `liveresults.trxctiming.com/robots.txt` | GET | 200 (3,023 B, identical SPA shell) | SPA serves index for unknown paths | 09:06 |
| `liveresults.trxctiming.com/static/js/main.6419a799.chunk.js` | GET | 200 (382,090 B) | bundle references only its own origin + `trxctiming.com`; no external data API host in the main chunk | 09:07 |
| `https://www.mshsaa.org/Content/TrackandField/InformationCentral.aspx` | GET (Chrome UA) | 200 (33,314 B) | provider anchors: `Live Results - PrimeTime Timing → pttiming.com`; `Official Results - AthleticNet → athletic.net/track-and-field-outdoor/usa/high-school/missouri/mshsaa`; **no TRXC link** | 09:08 |
| `https://www.mshsaa.org/Content/CrossCountry/InformationCentral.aspx` | GET (Chrome UA) | 200 (37,143 B) | no external provider links; Championship Results are internal `PostseasonResult.aspx?alg=11|12&class=1..5&year=2025` | 09:09 |
| `https://www.mshsaa.org/Content/TrackandField/Home.aspx` | GET (Chrome UA) | 200 (31,022 B) | links `State Championship Results` (alg=52/53), `Sectional Info`, and the PDF `Athletic.net helpful links`; **no TRXC link** | 09:09 |
| `https://www.mshsaa.org/Activities/PostseasonResult.aspx?alg=52&year=2026` | GET (Chrome UA) | 200 (72,950 B) | T&F state-results surface resolves via group id (report 21: Hy-Tek PDF licensed to PrimeTime); no TRXC reference | 09:09 |
| `https://trxctiming.com/wp2/wp-sitemap.xml` | GET | 404 (52,309 B) | no sitemap | 09:12 |
| `https://trxctiming.com/wp2/feed/` | GET | 200 (883 B) | site RSS is an empty post feed (0 `<item>`) — no results change feed | 09:12 |
| `…/wp2/cross-country/past-results/2026-cross-country-results/` | GET (default `curl` UA) | 200 (65,483 B) | no UA gate on the results tree | 09:12 |
| `http://archive.org/wayback/available?url=trxctiming.com/wp2/upcoming-meet-information/` | GET | **429 Too Many Requests** | no historical snapshots check possible; host abandoned per politeness rule (no retry) | 09:11 |

Raw captures (page HTML/JSON used for every count above) are under
`research/midwest/evidence/gaps/40/` — index tables, the 17 sampled result files (incl. `henle2018.html`),
the 5 XC menus, the MSHSAA_XC menu, the four MSHSAA pages, and the parser outputs that back the grade
counts: `strict-stats.json` (authoritative), `sample10/sample10-stats.json`, `season-stats.json`.
`grade-sample-stats-SUPERSEDED-loose-parse.json` is an earlier, looser regex pass whose histogram leaked
mark digits into the grade column — retained only as a record; do not use its numbers.
