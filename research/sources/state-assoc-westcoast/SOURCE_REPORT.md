# SOURCE_REPORT.md — Tier-2 state association sources for CA, OR, WA, AK, HI, NV

Lane: `research/sources/state-assoc-westcoast/`
Captured: **2026-09-22T03:54:03Z .. 2026-09-22T04:00:17Z**
Raw evidence: `samples/**` (105 byte-exact captures, 14,615,289 bytes) — see `samples/CAPTURES.md`
Machine-readable: `schema.json`, `coverage.json`
Derived extracts: `derived/**`

---

## 0. Lane-level findings

**The three highest-value finds are Athletic.net / MileSplit links published directly by an
association.** Exact markup is quoted in §OR, §AK and §HI.

| Association | Publishes Athletic.net | Publishes MileSplit | Sanctioned timing provider |
|---|---|---|---|
| OR — OSAA | **yes**, 2 distinct URLs | no | Athletic Timing |
| AK — ASAA | **yes**, 1 team URL | no | RaceResult |
| HI — HHSAA | **yes**, 1 live-meet URL | **yes**, 1 raw-results URL | MileSplit Hawaii + Athletic.net Live |
| CA — CIF | none found | none found | none published (Arbiter for officials) |
| WA — WIAA | none found | none found | not identified |
| NV — NIAA | none found | none found | **unresolved** (bot-challenged) |

**Measured school universe, per jurisdiction:**

| Jurisdiction | Association | Schools | Enumeration | Requests to enumerate all |
|---|---|---|---|---|
| CA | CIF (state + 10 sections) | **1608** | bulk XLSX census | 1 |
| OR | OSAA | **299** | flat HTML directory | 1 |
| WA | WIAA | **752** | paginated FinalForms (1..51) | 51 |
| AK | ASAA | **217** | single HTML table | 1 |
| HI | HHSAA | **95** | 5 league pages | 5 |
| NV | NIAA | **127** | paginated FinalForms (1..9) | 9 |

**Two cross-cutting platform facts.**

1. `cifstate.org`, every PrestoSports CIF section host and `niaa.com` sit behind a CloudFront
   rule that returns **403 + a 919-byte error page** unless a browser-like `User-Agent` is sent.
   These are the only hosts where a stock non-browser UA was refused. Their PrestoSports
   `robots.txt` sets **`Crawl-Delay: 10`** for `User-agent: *` and disallows
   `/reports/ /admin/ /action/ /cgi-bin/ /_private/ /_vti_bin/`. Requests to those hosts were
   spaced ≥10 s and no disallowed path was fetched.
2. **OSAA disallows `/teams/`, `/contests/`, `/brackets/`, `/forms/`.** OSAA's per-school *team*
   and *contest* pages live under exactly those two first prefixes and were therefore **not
   fetched**. Everything OSAA-shaped in this report comes from `/schools/**`, `/activities/**`
   and `/coaches`, which are permitted.

**`niaa.org` is not Nevada's association.** It 301s and serves a 198 KB page titled
`Alumni | INROADS` (canonical `https://inroads.org/alumni/`). Nevada's association is
**`niaa.com`**. Evidence: `samples/robots-niaa.txt`, `samples/robots-niaa-follow.txt`.

---

## CA — California Interscholastic Federation (section-based; 10 sections)

| Field | Value |
|---|---|
| **Source name** | California Interscholastic Federation (CIF) — state office **plus 10 sanctioned sections** |
| **URL** | https://www.cifstate.org/ — deliberately **not** one directory; see the section table below |
| **Geographic coverage** | Entire state of California. Members are organised into 10 sections, not one body. |
| **Sports** | 24 sport variants in the census; XC and TF appear as `Cross Country`, `Track & Field`, plus `Adapted` and `Unified` variants |
| **Historical depth** | Participation census **2012-13 → 2025-26** (14 files, `.xls`/`.xlsx`). XC finals PDFs **2004 → 2025** (66 archive links). TF finals PDFs **2005 → 2026** (36 archive links; 2020/2021 absent). |
| **Discovery mechanism** | State census index → bulk XLSX. Per section: a PrestoSports "school directory" widget. |
| **Stable identifiers** | **None at school level.** The census keys schools by `School Name` + `City` strings only. Section membership is a `CIF Section` string. |
| **Pagination** | Census: none (single 1.8 MB file). Section widgets: none observed — a single flat response. |
| **Athlete fields** | **Not measured in this lane.** The census carries aggregate participant counts, never names. Athlete-level data exists only inside the result PDFs, which were not opened. `[INFERENCE]` those PDFs carry name/school/place/time. |
| **Meet fields** | **Not measured.** No meet table was opened. |
| **Result fields** | Aggregate only via census: per school, per sport, `- Boy Participants`, `- Girl Participants`, `- Total Number of Boy Teams`, `- Total Number of Girl Teams`. Example row (`ABLE Charter`, Stockton, SAC-Joaquin): CC boys 12, CC girls 4, TF boys 12, TF girls 8. |
| **Grade/class evidence** | **No** grade or class field anywhere in the census. Section is the only segmentation. |
| **Coach/contact fields** | **None published.** CIF state runs an Athletic Administrators Summit and an AD eligibility workshop but publishes no AD/coach directory. |
| **Public API availability** | None found. |
| **Static file availability** | **Excellent.** `.xlsx` participation census ×14, plus `.pdf` finals archives. |
| **Browser requirement** | Chrome-like UA required for the 403-gated CloudFront hosts; plain `curl` works against `cifss.org`, `cifsf.org`, `cif-la.org`, `cifsjs.org` (their own robots rules apply). |
| **Request cost** | 1 request for the whole CA school universe (`2025-26_census_web.xlsx`, 1,842,669 bytes). `[INFERENCE]` ~1 request per section for the widget directories. |
| **Published rate limits** | Presto sections + CIF state: `Crawl-Delay: 10`. `cif-la.org`: `Crawl-delay: 5`. `cifsf.org`: nothing (`Disallow:` empty). |
| **Known blocks** | (a) CloudFront 403 at non-browser UA on Presto hosts. (b) `cifsjs.org/robots.txt` serves a 47 KB SPA shell — no robots.txt exists. (c) `cifoakland.org` 302s to `ousd.org/oal`. (d) `/sports/cross_country/results/index` is **404** — the real path is `/sports/cross_country/past_results_records/index`. |
| **Cross-source join keys** | **Weak.** No NCES id, no state id. `School Name` + `City` is the only internal key. `[INFERENCE]` NCES name matching is the realistic CA join path. |
| **Estimated marginal coverage** | Very high for **participation** (1,400 schools with XC any, 1,323 with TF any); low for **results** in structured form (PDF only). |
| **Implementation recommendation** | **PRIMARY** — the census XLSX is the single best participation artefact found anywhere in this lane, but it must be paired with a result source for meet-level data. |

### CA section enumeration (10 sections)

| # | Section | Abbrev | URL | Publishes a school list? | Endpoint | Census schools |
|---|---|---|---|---|---|---|
| 1 | Central Coast Section | CCS | https://www.cifccs.org/ | no (JS/iframe) | `/schools/ccs_directory` | 155 |
| 2 | Central Section | CIFCS | https://www.cifcs.org/ | **yes** | `cifcshome.org/widget/school/directory` (**150**) | 139 |
| 3 | Los Angeles City Section | CIF-LA | https://www.cif-la.org/ | **yes** | `cif-lahome.org/widget/school/directory` (**173**) | 152 |
| 4 | North Coast Section | NCS | https://www.cifncs.org/ | **yes** | `cifncshome.org/widget/school/directory` (**179**) | 175 |
| 5 | Northern Section | CIFNS | https://www.cifns.org/ | **yes** | `/governance/directory/home` + `/landing/26-27_School_Member_List.pdf` | 67 |
| 6 | Oakland Section | CIF-OAK | https://www.cifoakland.org/ → https://www.ousd.org/oal | no | — | 29 |
| 7 | Sac-Joaquin Section | SJS | https://www.cifsjs.org/ | no (JS) | `/member-schools` (0 names in static HTML) | 201 |
| 8 | San Diego Section | CIFSDS | https://www.cifsds.org/ | **yes** | `/school-resources/CIF_School_Directory` | 129 |
| 9 | San Francisco Section | CIFSF | https://www.cifsf.org/ | **yes** | `/schools/high-schools/` | 18 |
| 10 | Southern Section | CIFSS | https://www.cifss.org/ | partial | `/directory/` (titled "School Directory", SearchWP client-side render, 0 names in static HTML) | 542 |

Census totals per section are from `samples/cif-census-2025-26.xlsx` (1,608 data rows). The three
measured widget counts are in `derived/cif-widget-directory-counts.json`. Widget counts exceed
census counts because the widget lists every directory school regardless of whether it filed a
census return — `[INFERENCE]`, not directly proven.

**Athletic.net linkage: NONE found.** No CIF state or section page captured in this lane links
Athletic.net or MileSplit. `cifsf.org`'s page titled *"The ultimate XC guide to the high school
season is here"* was fetched specifically to test this and contains **zero** Athletic.net or
MileSplit references (`samples/cifsf-xc-guide.html`).

---

## OR — Oregon School Activities Association (OSAA)

| Field | Value |
|---|---|
| **Source name** | Oregon School Activities Association (OSAA) |
| **URL** | https://www.osaa.org/ |
| **Geographic coverage** | Oregon, single statewide association (no sub-sections) |
| **Sports** | 30+ activities; XC and TF are first-class: `/activities/bxc`, `/activities/gxc`, `/activities/btf`, `/activities/gtf` |
| **Historical depth** | Participation XLSX **2008-09 → 2025-26** (43 files, Fall/Winter/Spring). XC results page is live-season only. |
| **Discovery mechanism** | `/schools/full-members` flat list → per-school detail page. Plus `/schools/{alphabetically,counties,regions,classifications-districts,participation}`. |
| **Stable identifiers** | **Yes — `osaa_school_id`.** Anchor `<a href="https://www.osaa.org/schools/347">Adrian High School</a>` → 299 distinct ids, 0 duplicates. |
| **Pagination** | None for the school list (single page). |
| **Athlete fields** | **Not measured.** OSAA's per-athlete surfaces are under `/teams/` and `/contests/`, both **disallowed by robots.txt**, so they were not fetched. |
| **Meet fields** | Meet *schedules* exposed at `/activities/{bxc,gxc}/meet-schedules` and `/activities/bxc-gxc/entries`; no meet fields were extracted. |
| **Result fields** | Live results are off-site (Athletic Timing). `derived/osaa-participation-2025-26-fall-summary.json` holds the participation shape: `BXC-B/G/X/T`, `GXC-B/G/X/T` where `-B`=boys, `-G`=girls, `-X`=co-ed, `-T`=total. |
| **Grade/class evidence** | **Class yes, grade no.** `Classification` column (`1A`…`6A`) in the participation workbook and in the school row. Example: `Adrian, 1A`. |
| **Coach/contact fields** | **The best in this lane.** Per-school staff table with Position/Name/Phone/**Email**: Superintendent, Principal, Athletic Director, Assistant AD, Activities Director — all with working emails (e.g. `george.ellsworth@adriansd.org`). Plus a per-activity **Head Coach** table (`Activity | Head Coach | League | Coop | Teams`), e.g. `Girls Track & Field | Reagan Shira | 1A-SD4 Special District 4 | | V`. Cost: **299 requests** for the full state. |
| **Public API availability** | None found. |
| **Static file availability** | Yes — 43 participation `.xlsx`, planbooks and memos as `.pdf`. |
| **Browser requirement** | No. Plain `curl` returns 200. |
| **Request cost** | 1 (school list) + 299 (staff/coach) + 1 (participation index) + 1 per participation workbook. |
| **Published rate limits** | `robots.txt`: `Disallow: /brackets/ /teams/ /contests/ /forms/ /mobile/ /demo/ /dev/`. No crawl-delay stated. |
| **Known blocks** | `/teams/` and `/contests/` are robots-disallowed — per-athlete and per-meet data is **out of reach by policy**, not by technique. |
| **Cross-source join keys** | `osaa_school_id` (internal only). School name strings match Athletic.net school names. `[INFERENCE]` no NCES id is exposed, so NFHS/NCES joins need name matching. |
| **Estimated marginal coverage** | 238 schools with boys XC (7,266 participants), 214 with girls XC (5,232) in 2025-26 Fall. |
| **Implementation recommendation** | **PRIMARY** for Oregon schools + coaches; **RESULT_SOURCE** via the published Athletic.net link. |

### OR — Athletic.net linkage (highest-value find, verbatim markup)

`samples/osaa-home.html` line 314, inside the *"Fall Individual Sports Schedules"* block that also
links OSAA's own XC schedules:

```html
<b><a href="https://www.osaa.org/activities/fall">Fall Individual Sports Schedules</b></a><br />
<small>
<a href="https://www.osaa.org/activities/gxc/meet-schedules" target="_blank">Girls Cross Country</a> |
<a href="https://www.osaa.org/activities/bxc/meet-schedules" target="_blank">Boys Cross Country</a> |
<a href="https://www.athletic.net/cross-country/usa/high-school/oregon" target="_blank">Athletic.net</a>
</small>
```

`samples/osaa-gxc.html` line 419-422 — a **second, different** URL, in the sport sub-nav next to
OSAA's own *Entries* and *Results* buttons. Note the markup is malformed upstream (the `</a>` is
missing before `</li>`); quoted exactly as served:

```html
<ul>
    <li><a href="https://www.osaa.org/activities/bxc-gxc/entries">Entries</a></li>
    <li><a href="https://www.osaa.org/activities/bxc-gxc/results">Results</a></li>
    <li><a href="https://www.athletic.net/CrossCountry/Oregon/" target="_blank">Athletic.Net</li>
</ul>
```

The same `https://www.athletic.net/cross-country/usa/high-school/oregon` link is repeated on
individual school pages (`samples/osaa-school-347.html`).

**OR sanctioned timing provider** — `samples/osaa-xc-results.html` line 464:

```html
<a class="live_results_button" href="https://live.athletictiming.net/meets/58997" style="font-size:9pt; margin-right: 8px;" target="_blank">Live Results</a>
```

→ **Athletic Timing** (`live.athletictiming.net`).

---

## WA — Washington Interscholastic Activities Association (WIAA)

| Field | Value |
|---|---|
| **Source name** | Washington Interscholastic Activities Association (WIAA) |
| **URL** | https://www.wiaa.com/ (site runs on `rschooltoday`/rst7) |
| **Geographic coverage** | Washington, single statewide association |
| **Sports** | Full slate; XC state event at `/events/2026-state-cross-country/`, XC tournaments at `/tournament-xch/?sportid=22` |
| **Historical depth** | Not measured — no result archive was enumerated. |
| **Discovery mechanism** | **FinalForms state directory** at `wiaa.finalforms.com`, linked from the WIAA homepage and repeated on the state XC page. |
| **Stable identifiers** | **Yes.** FinalForms row id (`state_school_109587`) → `https://wiaa.finalforms.com/state_schools/109587`. Also **NCES id** (`530003000007`) and a **state id** (`WA-14005-3476`). |
| **Pagination** | **Yes — 15 rows/page, last page link = 51.** URL: `?page={N}&state_schools.athletic_association_member_status_in=member&state_schools.is_archived_eq=false` |
| **Athlete fields** | **Not measured.** |
| **Meet fields** | **Not measured.** |
| **Result fields** | No third-party result host found. The state XC page (`samples/wiaa-state-xc-results.html`) links only `arbiter.io`, `finalforms.com`, `4nsp.com` photos, and the WIAA Live app. |
| **Grade/class evidence** | **Grade range yes, class yes.** `[9th - 12th]` on each row, and `Classifications: 2A` / `2B` (plus a `/state_partitions/{id}` link for each class). |
| **Coach/contact fields** | **No names.** WIAA's coach/AD surface is Arbiter (`mywiaa.wiaa.com/ad-center`, `/coaches`, `/finalforms`); the AD Center page serves PDF checklists and Google Forms, not a name roster. |
| **Public API availability** | None found as a documented API, but a JSON select2 endpoint is referenced: `/select2/state_records/state_partitions?athletic_association_abbreviation=WIAA&include_state_counties=true`. There is also a map view at `/state_schools/map?...`. |
| **Static file availability** | No participation bulk file found. |
| **Browser requirement** | No. Plain `curl` returns 200. |
| **Request cost** | **51 requests** for all member schools (1 for the base page to discover the last page, 51 for pages 1..51). |
| **Published rate limits** | **None — `www.wiaa.com/robots.txt` is 404.** `finalforms.com` publishes no robots policy for this path either. |
| **Known blocks** | None encountered. |
| **Cross-source join keys** | **Strong.** 12-digit **NCES id** + `NCES name` + WA state id. NCES is the cleanest cross-source key found in this entire lane. |
| **Estimated marginal coverage** | 752 member schools (50×15 + 2). Per-school `student_count` (e.g. Aberdeen = 919, Adna = 369) gives free enrollment weighting. |
| **Implementation recommendation** | **PRIMARY** — the richest per-school attribute set of the six, and the only one with a clean federal join key. |

Measured pagination proof: page 51 contains exactly **2** `tr.state_school` rows
(`samples/wiaa-finalforms-p51.html`), page 1 and page 2 contain **15** each. 50 full pages × 15 +
2 = **752**.

WIAA-published school row shape (verbatim field values from `samples/wiaa-finalforms-state-schools.html`):

```
student_count      919
name               Aberdeen
nces_name          J M Weatherwax High School
nces_id            530003000007
state_id           WA-14005-3476
grades             9th - 12th
district           Aberdeen School District   (/state_districts/18432)
league             Evergreen 2A/3A            (/state_partitions/14498)
classification     2A
badge              Combine Host
```

**Athletic.net linkage: NONE.** No WIAA page captured in this lane links Athletic.net or
MileSplit.

---

## AK — Alaska School Activities Association (ASAA)

| Field | Value |
|---|---|
| **Source name** | Alaska School Activities Association (ASAA) |
| **URL** | https://asaa.org/ (WordPress; `robots.txt` declares `https://asaa.org/wp-sitemap.xml`) |
| **Geographic coverage** | Alaska, statewide, organised into **6 regions** (R1–R6) |
| **Sports** | `/activities/cross-country-running/`, `/activities/track-field/`, plus fall/winter/spring slates |
| **Historical depth** | 2025 XC state result PDFs (12 files) and 2026 TF result PDFs present. Full archive not enumerated. |
| **Discovery mechanism** | `/about/member-schools/` — one TablePress table, all 217 schools on one page. |
| **Stable identifiers** | **No numeric school id.** Schools are identified by the `SCHOOL` name string. Phone, address, city, zip present. |
| **Pagination** | None — single table. |
| **Athlete fields** | **Not measured.** TF result PDFs are per-division individual + team results, so athlete names are `[INFERENCE]` present inside those files. |
| **Meet fields** | **Not measured.** |
| **Result fields** | Champions published as per-division PDFs: individual results, team results, girls variants. Example filenames: `2025-XCR-DI-ASAA-State-Championship-Results-.pdf`, `2025-XCR-DII-Girls-Team-ASAA-State-Championship-Results-.pdf`. |
| **Grade/class evidence** | **Class yes, grade no.** `CL` column: `1A` (152), `2A` (28), `3A` (15), `4A` (19), `3A/4A` (2), `4/A` (1). |
| **Coach/contact fields** | **School phone only.** The member table carries `PHONE` (e.g. `907-825-3616`). `/coaches/` and `/coaches/advisors/` carry policy/education content with **no named roster**. |
| **Public API availability** | **Yes, partially — WordPress REST.** `https://asaa.org/wp-json/wp/v2/pages/19` serves the member-schools page content. |
| **Static file availability** | Yes — 12 XC 2025 result PDFs and per-division TF result PDFs. |
| **Browser requirement** | No. Plain `curl` returns 200. |
| **Request cost** | 1 request for the entire 217-school universe. |
| **Published rate limits** | `robots.txt` disallows only `/wp-admin/` (with the standard `Allow: /wp-admin/admin-ajax.php`). No crawl-delay. |
| **Known blocks** | None encountered. |
| **Cross-source join keys** | `[INFERENCE]` school-name matching only, plus `CITY`/`ZIP`. One measured external key: **Athletic.net team id 22622**. |
| **Estimated marginal coverage** | 217 schools, 27,635 total enrollment across 212 numeric rows. Regions R1=109, R2=52, R5=25, R3=14, R6=9, R4=8. |
| **Implementation recommendation** | **PRIMARY** for the school universe; **RESULT_SOURCE** via RaceResult and the published PDF archive. |

### AK — Athletic.net and timing-provider linkage (verbatim markup)

`samples/asaa-track-field.html`, in the state championship *RESULTS* block, immediately above the
ASAA-hosted result PDFs:

```html
• <a href="https://www.athletic.net/team/22622/track-and-field-outdoor/2025">Visit Athletic.net to view results</a><br>
```

That is a **team-scoped** Athletic.net URL (team 22622), not a state landing page — a usable join
key into the Athletic.net graph, but it does not enumerate Alaska.

`samples/asaa-state-xc-results.html`, published twice (championship block and results block):

```html
• <a href="http://my.raceresult.com/363555/">Live Results Link</a><br>
```

→ **RaceResult** is ASAA's sanctioned live timing provider.

**MileSplit linkage: NONE found on ASAA pages.**

ASAA member-school table shape (verbatim first data row from `samples/asaa-member-schools.html`):

```
SCHOOL    Akiachak (Moses Peter)
PHONE     907-825-3616
EN        63
RG        R1
CL        1A
ADDRESS   PO Box 51189
CITY      Akiachak
ZIP       99551
DISTRICT  Yupiit School
```

217 data rows, 0 duplicate school names, 212 numeric `EN` values summing to 27,635.

---

## HI — Hawaii High School Athletic Association (HHSAA)

| Field | Value |
|---|---|
| **Source name** | Hawaii High School Athletic Association (HHSAA) |
| **URL** | https://www.hhsaa.org/ (Imagine CMS) |
| **Geographic coverage** | Hawaii, statewide, via **5 member leagues** |
| **Sports** | XC at `/sports/cross_country/tournament/2026`, TF at `/sports/track_field/tournament/2026` |
| **Historical depth** | Not measured beyond the 2025-26 season pages. |
| **Discovery mechanism** | `/schools` league index → 5 league pages → `/schools/<slug>` cards. |
| **Stable identifiers** | **Slug, stable and human-readable** (e.g. `hilo_high_school`). No numeric id. |
| **Pagination** | None — one page per league. **5 requests total.** |
| **Athlete fields** | **High-value and athlete-level.** `HHSAA-Championship-Performance-List.pdf`, `hawaii_2026_boys_track_top35.pdf`, `hawaii_2026_girls_track_top35.pdf`, plus public Google Sheets. Not opened in this lane. |
| **Meet fields** | Championship meet program, schedule (Saturday), campus map, games-committee decisions — all PDF. |
| **Result fields** | Published on **MileSplit in `?type=raw` mode** (`hi.milesplit.com/meets/730986-…/results/1304674?type=raw`) and **Athletic.net Live** (`live.athletic.net/meets/58854`). Also RSS result feeds. |
| **Grade/class evidence** | **No.** School cards carry only a display name and a logo image; no grade or class field observed. |
| **Coach/contact fields** | **No named coach directory.** `/resources/coaches` is an Imagine CMS page with 0 named coach entries in static HTML. Association-side coordinators are listed at `/about/officials-coordinators` and `/about/sport-coordinators`. |
| **Public API availability** | None found. **RSS** feeds are published per tournament, e.g. `https://hhsaa.org/rss/9939/Schedule%20&%20Results`. |
| **Static file availability** | Yes — a full PDF packet per championship (program, performance list, top-35, coaches packet, schedule). |
| **Browser requirement** | No. Plain `curl` returns 200. |
| **Request cost** | 5 requests for the school universe; 1 request per championship page for the result packet. |
| **Published rate limits** | **None.** `robots.txt` is 204 bytes in which the entire policy is **commented out** — nothing is disallowed. |
| **Known blocks** | None encountered. The 2026 XC tournament page carried **no** direct PDF result link at capture time; results surface through RSS. |
| **Cross-source join keys** | **Strongest in the lane.** HHSAA publishes a **MileSplit meet id + result id** (`730986` / `1304674`) and an **Athletic.net meet id** (`58854`) directly. |
| **Estimated marginal coverage** | 95 schools: OIA 31, BIIF 23, ILH 20, MIL 13, KIF 8. No single page states this total — it is the sum of five measured league lists with 0 overlap. |
| **Implementation recommendation** | **PRIMARY** for the school universe; **RESULT_SOURCE** — HHSAA is the only association in this lane that openly hands over both MileSplit raw results and Athletic.net live.**

### HI — Athletic.net and MileSplit linkage (verbatim markup)

`samples/hhsaa-tf-tournament-2026.html` — both links sit in the same `<ul>`, each with the
provider's logo as an image asset served from HHSAA's own domain:

```html
<li><a href="https://live.athletic.net/meets/58854" target="_blank"><img src="/assets/content/sports/track_field/tournament/2026/athletic-live.png?1777423348" alt="Athletic-live" width="275" height="40">HHSAA Live Results</a></li>
```

```html
<li><a href="https://hi.milesplit.com/meets/730986-hhsaa-championship-2026/results/1304674?type=raw" target="_blank"><img src="/assets/content/sports/track_field/tournament/2026/milesplit.png?1779492745" alt="Milesplit" width="195" height="52">HHSAA Championships Final Results</a></li>
```

League labels: **BIIF** = Big Island Interscholastic Federation, **ILH** = Interscholastic League
of Honolulu, **KIF** = Kauai Interscholastic Federation, **MIL** = Maui Interscholastic League,
**OIA** = Oahu Interscholastic Association.

---

## NV — Nevada Interscholastic Activities Association (NIAA)

| Field | Value |
|---|---|
| **Source name** | Nevada Interscholastic Activities Association (NIAA) — **at `niaa.com`, NOT `niaa.org`** |
| **URL** | https://www.niaa.com/ (PrestoSports) |
| **Geographic coverage** | Nevada, statewide |
| **Sports** | `/sports/xc` and `/sports/track` (bare-slug scheme, e.g. `bkb`, `fball`, `wrest`) |
| **Historical depth** | XC archive reaches back to **2009-10** with per-season, per-division files (2A/3A/4A) and per-region files (Sunrise/Sunset/Northern/Southern). |
| **Discovery mechanism** | `niaa.finalforms.com` — same FinalForms platform as WA — linked from `/members/Landing`. |
| **Stable identifiers** | **Yes.** FinalForms row id (`state_school_73153`) → `/state_schools/73153`. Also NCES **private-school (PSS)** ids for non-public schools (`00847227`). **No state id field** (unlike WA). |
| **Pagination** | **Yes — 15 rows/page, last page = 9.** The page states `127 Records`; page 9 holds 7 rows → 8×15 + 7 = **127**. |
| **Athlete fields** | **Not measured.** |
| **Meet fields** | **Not measured.** |
| **Result fields** | Archived results are a **mixed legacy bag**: `.pdf`, `.htm`, and `.mht` (MIME HTML). Examples: `/sports/xc/2010-11/files/2A_Boys.pdf`, `/sports/xc/2010-11/files/SR_Boys.htm`, `/sports/xc/2011-12/files/2011_4A_Sunrise_BXC_Individual_Results.mht`. |
| **Grade/class evidence** | **Grade range yes, class not observed on page 1.** `[PPPK - 12th]`, `[PPK - 12th]`; badge `Non-public School`. |
| **Coach/contact fields** | **UNRESOLVED.** `/coaches/Landing` returned **HTTP 202 with a zero-byte body** on both attempts. |
| **Public API availability** | None found. `members.niaa.org` and `highschoolofficials.com` (Arbiter) are linked but were not enumerated. |
| **Static file availability** | Yes, but old-format — `.mht` files are not machine-friendly without conversion. |
| **Browser requirement** | Chrome-like UA required (CloudFront 403 otherwise). |
| **Request cost** | **9 requests** for the school universe. |
| **Published rate limits** | `Crawl-Delay: 10` (PrestoSports boilerplate). |
| **Known blocks** | **PrestoSports bot challenge: HTTP 202 + empty body** on `/sports/track` and `/coaches/Landing`. Also note `/sports/cross_country` is **404** — the correct slug is `/sports/xc`. |
| **Cross-source join keys** | FinalForms id + NCES PSS id (private) / no NCES id for public rows on page 1. `[INFERENCE]` weaker than WA because the state-id column is absent. |
| **Estimated marginal coverage** | 127 schools. Page 1 alone is dominated by non-public schools, so NV's public-school coverage was not separately measured. |
| **Implementation recommendation** | **CONDITIONAL** — the school universe enumerates cleanly (127 schools, 9 requests), but the **timing provider and coach directory are unverified** because PrestoSports bot-challenged both paths. Do not treat NV as PRIMARY until those two surfaces are read with a browser. |

NV FinalForms row shape (verbatim, `samples/niaa-finalforms-state-schools.html`):

```
student_count   689
name            Adelson School
nces_name       THE DR. MIRIAM AND SHELDON G. ADELSON SCHOOL
nces_id         00847227        -> https://nces.ed.gov/surveys/pss/privateschoolsearch/school_detail.asp?Search=1&ID=00847227
grades          PPPK - 12th
badge           Non-public School
```

Note the field variance versus WA: NV private schools link the NCES **PSS** (private school)
search rather than **CCD**, and some rows use the literal string `--` in the student-count cell
(e.g. `American Heritage`).

**Athletic.net linkage: NONE found on NIAA pages.**

---

## Cross-source join keys (lane-wide)

| Key | Observed in | Example | Quality |
|---|---|---|---|
| `nces_id` (12-digit CCD) | WA, NV | `530003000007` | **Best.** Federal, stable, name-independent. |
| `state_id` | WA only | `WA-14005-3476` | State-scoped but authoritative. |
| FinalForms row id | WA, NV | `109587` / `73153` | Unique per tenant; **not** shared between states. |
| OSAA school id | OR | `347` | Unique per association. |
| HHSAA school slug | HI | `hilo_high_school` | Readable, unique. |
| Athletic.net team id | AK | `22622` | Direct bridge into the Athletic.net graph. |
| Athletic.net meet id | HI | `58854` | Direct bridge. |
| MileSplit meet/result id | HI | `730986` / `1304674` | Direct bridge. |
| OSAA `Coop` column | OR | `Nyssa / Adrian` | Names partner schools in combined XC/TF programmes — matters when reconciling shared squads. |

**No association in this lane publishes a documented JSON/REST API** for school enumeration. The
only machine-readable API observed is ASAA's WordPress REST page endpoint
(`https://asaa.org/wp-json/wp/v2/pages/19`) and WIAA's undereferenced select2 endpoint
(`/select2/state_records/state_partitions?athletic_association_abbreviation=WIAA&…`).

---

## What remains unproven in this lane

Stated plainly, so nobody downstream mistakes coverage for completeness:

1. **Athlete-level and meet-level fields were never extracted for any of the six.** Every result
   PDF, `.mht` archive, Google Sheet, MileSplit raw page and live-timing endpoint listed here was
   *located and its link verified*, but **not opened**. All athlete/meet schema statements are
   marked `[INFERENCE]`.
2. **WIAA 752 and NV 127 are pagination arithmetic**, not a total printed by the site. WA's 752
   has no corroborating declared count on the page; NV's 127 is corroborated by the page string
   `127 Records`.
3. **CIF-SS, CIF-SJS, CIF-CCS and CIF-SD school-list counts were not measured** — those directory
   pages render their lists client-side, so the static HTML carries 0–1 school names.
4. **NV timing provider and coach directory are unresolved** (HTTP 202 empty body from
   PrestoSports on `/sports/track` and `/coaches/Landing`). A browser session is required.
5. **WA timing provider is unidentified.** The state XC page exposes no third-party timing host;
   `wpanetwork.com/wiaa/brackets/tournament.php` is an xajax *team-bracket* index, not a timing
   service.
6. **HI's 95-school total is a sum of five league pages**, not a stated figure.
7. **CA's 1,608 school rows are not deduplicated by a stable id** — the census keys on
   `School Name` + `City` strings, so same-name schools in different cities are indistinguishable
   without fuzzy matching.
8. Historical depths for WA, AK and HI participation were not enumerated — only CA (2012-13→),
   OR (2008-09→), CA result archives (2004→) and NV's XC archive (2009-10→) were measured.
