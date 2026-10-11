# samples/CAPTURES.md — state-assoc-plains

Raw captures taken by this lane on **2026-09-22T03:56:21Z – 03:58:00Z** (= 2026-09-21 evening,
America/Chicago), plus pointers to pre-existing captures that are **not** duplicated here.

Discipline applied to every request below:

- Requests were issued strictly sequentially with ≥1 s between requests to the same host (global rate
  ≤1 request/s); 9 stored captures plus one discarded connectivity check (10 requests total).
- User agent: `omp-research/1.0 (national census lane; contact via repo)`.
- Bodies are stored byte-exact as received (no reformatting; `curl -o`, never a shell redirect).
- The probe log `probe-2026-09-22.log` records timestamp / host / label / status / bytes / command for
  every request. It is the machine-readable companion to the table below.

## 1. Fresh captures (this lane)

| # | File | URL | HTTP | Bytes | UTC | Exact command |
|---|---|---|---|---|---|---|
| 3 | `kshsaa-publicclassifications-2026-09-22.json` | `https://kshsaa-api.kshsaa.org/PublicClassifications` | 200 | 31,885 | 03:56:30Z | `curl -sS -A "$UA" -o kshsaa-publicclassifications-2026-09-22.json https://kshsaa-api.kshsaa.org/PublicClassifications` |
| 5 | `ndhsaa-schools-2026-09-22.html` | `https://ndhsaa.com/schools` | 200 | 97,754 | 03:56:32Z | `curl -sS -A "$UA" -o ndhsaa-schools-2026-09-22.html https://ndhsaa.com/schools` |
| 7 | `iahsaa-member-schools-2026-09-22.html` | `https://www.iahsaa.org/member-schools/` | 200 | 268,201 | 03:56:54Z | `curl -sS -A "$UA" -o iahsaa-member-schools-2026-09-22.html https://www.iahsaa.org/member-schools/` |
| 9 | `mshsl-sitemap-page1-2026-09-22.xml` | `https://www.mshsl.org/sitemap.xml?page=1` | 200 | 341,086 | 03:57:00Z | `curl -sS -A "$UA" -o mshsl-sitemap-page1-2026-09-22.xml 'https://www.mshsl.org/sitemap.xml?page=1'` |
| 11 | `mshsaa-school-listing-2026-09-22.html` | `https://www.mshsaa.org/Schools/SchoolListing.aspx` | 200 | 721,371 | 03:57:02Z | `curl -sS -A "$UA" -o mshsaa-school-listing-2026-09-22.html https://www.mshsaa.org/Schools/SchoolListing.aspx` |
| 13 | `nsaa-directory-form-2026-09-22.html` | `https://secure.nsaahome.org/nsaaforms/direxportscreen.php` | 200 | 11,585 | 03:57:10Z | `curl -sS -A "$UA" -o nsaa-directory-form-2026-09-22.html https://secure.nsaahome.org/nsaaforms/direxportscreen.php` |
| 15 | `sdhsaa-cross-country-2026-09-22.html` | `https://sdhsaa.com/activity/cross-country/` | 200 | 509,970 | 03:57:12Z | `curl -sS -A "$UA" -o sdhsaa-cross-country-2026-09-22.html https://sdhsaa.com/activity/cross-country/` |
| 17 | `sdhsaa-cross-country-region-2026-09-22.html` | `https://sdhsaa.com/cross-country-region/` | 200 | 286,293 | 03:57:45Z | `curl -sS -A "$UA" -o sdhsaa-cross-country-region-2026-09-22.html https://sdhsaa.com/cross-country-region/` |
| 18 | `athleticlive-heros-nd-xc-2026-09-22.json` | `https://search.athletic.live/heros_meet_list/_search` (POST) | 200 | 1,369 | 03:58:00Z | `curl -sS -A "$UA" -H 'Content-Type: application/json' -X POST -o athleticlive-heros-nd-xc-2026-09-22.json 'https://search.athletic.live/heros_meet_list/_search' -d '{"size":20,"track_total_hits":true,"query":{"bool":{"filter":[{"match":{"ls":"ND"}},{"range":{"md":{"gte":"2026-08-15","lte":"2026-11-30"}}}]}},"sort":[{"md":"asc"}],"_source":["i","ani","n","md","ls","o"]}'` |

Notes:
- A connectivity check at 03:56:21Z (`ndhsaa.com/schools` → 200, 97,754 B) wrote to `/dev/null` and produced
  no file; row 5 is the stored capture of the same URL.

## 2. Derived counts and their exact commands

All counts quoted in `SOURCE_REPORT.md`/`coverage.json` were produced by these commands run in this
directory (outputs as shown in the lane transcript):

| Claim | Command |
|---|---|
| KS 348 classified; 36/36/36/64/64/112 | `python3 -c "import json,collections; d=json.load(open('kshsaa-publicclassifications-2026-09-22.json')); ..."` (sums `len(c['schools'])` over `d['classes']`) |
| ND 169 schools | `grep -o 'href="https://ndhsaa.com/schools/[0-9]*/[a-z0-9-]*"' ndhsaa-schools-2026-09-22.html \| sort -u \| wc -l` |
| IA 380 rows / 379 names / 364 links | `python3` regex count over `<tr>`/`<td>` in `iahsaa-member-schools-2026-09-22.html` |
| MN 664 schools + 1,097 tournaments | `grep -o 'https://www.mshsl.org/schools/[a-z0-9-]*' mshsl-sitemap-page1-2026-09-22.xml \| sort -u \| wc -l` and the same for `/tournaments/` |
| MO 1,006 / 646 / 592 | `python3` regex over `<tr data-organizationtype=… data-membertype=…>` in `mshsaa-school-listing-2026-09-22.html` |
| NE 314 options = 312 schools + 2 UI | `grep -o '<option[^>]*>[^<]*</option>' nsaa-directory-form-2026-09-22.html \| wc -l` |
| SD 10 Athletic.net region meet ids | `grep -o 'https://www\.athletic\.net/CrossCountry/meet/[0-9]*' sdhsaa-cross-country-region-2026-09-22.html \| sort -u \| wc -l` |
| ND 6 XC meets with AL+AN ids | `python3 -c "import json; d=json.load(open('athleticlive-heros-nd-xc-2026-09-22.json')); print(d['hits']['total']['value'])"` → `6` |

## 3. Pre-existing captures referenced (NOT duplicated in this directory)

### 3.1 Repository fixtures (`crates/census-service/tests/fixtures/`)

| Path | Used for |
|---|---|
| `ks/kshsaa_directory_a.json` | KSHSAA directory wire shape + example values (Abilene HS, `Identifier KSS0001`, `Id 458`) |
| `mshsl/school_detail_aitkin-high-school.html`, `school_detail_wayzata-high-school.html`, `school_detail_{academic-arts,foley}-high-school.html` | MSHSL school page: AD block, cfemail, enrollment/class |
| `mshsl/schools_listing.html`, `schools_listing_first_page.html` | MSHSL listing pagination rows |
| `mshsl/team_nodes_aitkin.json`, `team_nodes_wayzata.json` | MSHSL JSON:API team nodes (`nid`, path alias, sport) |
| `mshsl/coach_records_aitkin_track-and-field-boys.json`, `coach_records_aitkin_track-and-field-girls.json`, `coach_records_wayzata_track_boys.json` | MSHSL coach records (`name`, `coach_level`, `field_email`) |
| `plain_names/nd_schools_index.html` | NDHSAA `/schools` index (169 anchors) |
| `plain_names/nd_school_page.html`, `nd_school_page_no_ad.html` | NDHSAA school page: staff block + offering/coach table (West Fargo Sheyenne id 1045) |
| `plain_names/nsaa_directory_form.html` | NSAA directory form (option list) |
| `plain_names/nsaa_directory_export.html` | NSAA bulk export sample (8 school blocks incl. Adams Central: enrollment 215, NSAA District 4, AD Alan Frank, `Cross-Country (Boys) Toni Fowler`) |

### 3.2 Midwest research tree (`~/Downloads/midwest-tfxc-source-research/`)

| Path | Used for |
|---|---|
| `research/midwest/09,10,11,12,21,22,23,24,25,26,29-*.md` | every `[C …]` citation in the report |
| `data/source-coverage-matrix.csv` | the 7-state × source-family rows in `coverage.json` |
| `data/coach-contacts.csv` (2,298 rows) | coach/AD example rows and per-state yield counts |
| `data/adapter-ranking.csv` | ranked adapter choice (MileSplit roster adapter, IHSA API, plain-name walkers, athleticlive) |
| `tools/iowa-11/bound-ihsaa-boystrack-teams-2025-26.csv` | Bound team-card example (`h…`, nested `classes`, `school_guid`) |
| `tools/ne/*` (29 files incl. `abres26.txt`, `ccbClassAResults.html`, `nsaa_directory_all_2026-09-19.html`) | NE result/Hy-Tek examples and the 2025 Class A district top-15 grade row |
| `research/midwest/evidence/gaps/43/factsheet.txt` | SDHSAA factsheet "Member Schools 179" |
| `tools/26-scratch/*`, `tools/43-*`, `tools/a29-coach/*` | SD Bound id/AN link samples, coach-graph raw captures |
