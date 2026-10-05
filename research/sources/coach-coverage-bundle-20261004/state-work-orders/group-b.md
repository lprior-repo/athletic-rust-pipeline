# State work orders — group B (17 states)

States: **OH GA NC WI MN TN PA MO OK IA MD MA CT SC AL AR MS**
Created 2026-10-04 by `StateOrderB2`. This file is a read-only synthesis: **no network fetches, no builds/tests, no edits outside this file.** Every path was verified to exist this session (directory listing or read); every count is copied from an on-disk artifact, or attributed to the document that holds it.

## 0. Method, evidence tiers, and global caveats

**Matrix rows** are copied from `var/state-coverage-matrix-20261004.json` (mirror of `research/sources/coach-coverage-bundle-20261004/STATE-COVERAGE-MATRIX.md`). Fields: `ccd` = NCES CCD 2024-25 in-scope schools; `any` = distinct schools with any coach row in `census-prototype/out/coaches.jsonl`; `tfxc` = schools with Track/Cross-Country rows; `email` = schools with ≥1 row carrying an email. The matrix's own caveat applies: **today's tap rows are NOT merged** (scratch stores), so OH's SRC-100 import and MA's SRC-068 import do not move those numbers.

**Prototype rollup** numbers ("proto schools / tf_xc / head / admins") are copied from `/home/lewis/src/ad-law-scrape/census-prototype/out/state-summary.json` (per-state objects, read this session).

**Evidence tiers** (from `HELD-INVENTORY.md`):
- **A — corpus (2026-09-21/22/27)**: `research/sources/state-assoc-{greatlakes,midatlantic,plains,southcentral,southeast}/samples/**` + each tree's `samples/CAPTURES.md`; `research/sources/coach-directories-national/samples/**` (`assoc-home/`, `dir/`, `api/`).
- **B — prototype (2026-09-29)**: `/home/lewis/src/ad-law-scrape/census-prototype/raw/` (56,294 bodies / 682 hosts; host response counts cited from `.../out/coverage.md`); `.../out/coaches.jsonl` (61,083 rows); `.../out/coverage.md` wired-source table; `.../out/state-summary.json`.
- **C — today (2026-10-04/05)**: `var/tap-*/` run dirs; bundle `probes/<n>-*/` and `extract/<n>-*/`.
- **Midwest local corpus** (referenced by the greatlakes tree): root `/home/lewis/Downloads/midwest-tfxc-source-research/` — **verified this session**; the `research/midwest/...` paths in `research/sources/state-assoc-greatlakes/samples/CAPTURES.md` resolve under it (e.g. `.../research/midwest/evidence/gaps/45/` listed). It is outside the repo; cite as-is.
- **Registry classification**: `HELD-CLASSIFY.json` / `HELD-CLASSIFY.md` (278 sources). Of this group, only **OH (9), PA (10), MA (11), MO (8), OK (9), IA (8), CT (8)** have registry rows; **GA, NC, WI, MN, TN, MD, SC, AL, AR, MS have none** (`grep '"state":"(GA|NC|WI|MN|TN|SC|AL|AR|MS|MD)"'` → no matches; the only composite-state row touching this group is SRC-095 `MA,CT,VT,NY` NEPSTA).

**Extraction routes available** (named per state below): prototype-wired adapters (verified rows in `census-prototype/out/coverage.md`), the prototype raw bodies for fresh extraction, the bundle `extract/*/rows.csv` sets (schema-exact `coach_contacts`), and the census-crawl registry table helpers (`crates/census-crawl/src/registry/table/{directories.rs,from_mshsl.rs,through_milesplit.rs}`).

**"Estimated schools reachable" is an estimate, labeled as such**; it is the count of schools in the held universe body (or the prototype-discovered count), not a claim of coach-email yield. Bodies noted "no email" genuinely publish none.

---

## OH — Ohio

### 1. Matrix row
`ccd 3611 / any 788 / tfxc 747 / email 779` (21.6% any-coach coverage).

### 2. Held-evidence inventory
- **Proto rollup**: 1155 schools / 1698 tf_xc / 1698 head / 791 admins.
- **Tier C, today (highest-value)**:
  - `extract/SRC-100/` — OATCCC "2026 Members" Google Sheet: `rows.csv` 143.7 KB = **2,154 data rows** (2,141 importable; 13 excluded), `excluded_rows.csv`, `sweep.json`, `REPORT.md`, `captures/` (4 files incl. the 75,095 B CSV export). Main import: rows=2141, **schools=630**, coaches=0, with_email=0 (school observations only; sheet publishes no role/email).
  - `var/tap-097-oh-ohsaa/`, `var/tap-101-oh-oeds/`; `probes/oh-ohsaa/` (manifest+raw+transport+robots), `probes/097-oh-ohsaa-seed/` (REPORT+manifest+captures), `extract/097-oh-ohsaa-seed/`.
- **Tier B, prototype**: `officials.myohsaa.org` **1,564 responses**; `oatccc.com` 6; OH derived rows in `coaches.jsonl`. Wired adapters: `oh_ohsaa_enrollment` (OHSAA enrollment table), `oh_myohsaa_sports`, `oh_myohsaa_ad`, `oh_milesplit` (discovery-only).
- **Tier A, corpus (midwest root)**: `.../research/midwest/evidence/gaps/45/` — `ohsaa-enrollment.html` **228,567 B = 815 OHSAA school rows** (school universe/denominator); `myohsaa-*-ad.body` ×15 + `myohsaa-*-si.body` ×15 (~10 KB each; AD + sports-info payload samples for OhsaaSchoolIds 100, 102, 105, 106, 112, 828, 892, 977, 1026, 1036, 1752, 9483, 9823, 1000018, 1000027) + `myohsaa-sample-summary.json` 7,577 B + `myohsaa-sample-selection.json`; `oatccc-doc-*.txt` ×2 (district reps 2,999 B; contacts 2,842 B); `oatccc-sitemap-urls.txt` 44,596 B; `tools/ohio-independent/raw/**` 98 files / 9,960,957 B incl. `ohsaa-state-tf-2026.pdf` 816,147 B. Index: `state-assoc-greatlakes/samples/CAPTURES.md` §B4.
- **Fixtures**: `crates/census-service/tests/fixtures/ohsaa/*.html` (7 files; portal shapes + malformed/no-result negatives).
- **Registry**: SRC-097…SRC-105 (9). HELD: SRC-100 (OATCCC). ADAPTER+DERIVED: 097/098/099. DERIVED: 101–105 (OEDS tap contents uninspected; gclc blocked; Central District XC page unverified; OATCCC leadership unverified).

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `extract/SRC-100` rows.csv (2,141) | import-coaches → school observations (**done**, `var/extract-import-100/store`) | **630 schools** |
| `gaps/45/ohsaa-enrollment.html` (815 rows) | `oh_ohsaa_enrollment` adapter / fresh extract | 815 school universe |
| `officials.myohsaa.org` 1,564 proto bodies | `oh_myohsaa_sports` + `oh_myohsaa_ad` parsers | participating schools' coach/AD rows (15-school AD sample held locally) |
| `tools/ohio-independent/raw/**` | results/athlete lane (finals, timers) | cross-check only, no coach contacts |

### 4. True missing (names only; NO fetching)
`gclc.gclsports.com` (blocked, SRC-104) · `education.ohio.gov` nonchartered nonpublic list (SRC-102) · `oeds.education.ohio.gov/dataextract` (SRC-101; `var/tap-101-oh-oeds/` contents uninspected) · OHSAA Central District XC page (SRC-105) · OATCCC leadership/regional-reps route (SRC-103) · live myOHSAA coverage for all 815 `OhsaaSchoolId`s (only 15 AD bodies sampled).

### 5. Notes
OATCCC rows carry **no role/email** → school observations only. myOHSAA bodies are AD/sports-info payloads (no TF/XC role guarantee). OHSAA enrollment is identity/classification only. Matrix OH counts exclude the SRC-100 import.

---

## GA — Georgia

### 1. Matrix row
`ccd 2349 / any 954 / tfxc 845 / email 636`.

### 2. Held-evidence inventory
- **Proto rollup**: 3074 / 4923 / 972 / 974.
- **Tier B**: `www.ghsa.net` 6 responses. Wired adapters: `ga_ghsa_pdf` (GHSA directory feed PDF), `ga_dragonfly` + `ga_dragonfly_p2` + `ga_dragonfly_p3` (maxinfosite GHSA directory pages 1–3), `ga_milesplit` (discovery-only).
- **Tier A, corpus** (`state-assoc-southeast/samples/`, verified listing): `ghsa-directory-feed.pdf` (+ `.meta.txt`), `home-ghsa.html`, `school-directory-ghsa.html`, `coaching-ghsa.html`, `cross-country-ghsa.html`, `track-field-ghsa.html`, `state-championships-ghsa.html`, `ghsa-xc-state-meet-results.html`, `ghsa-state-track-meet-results.html`, `ghsa-results-records.html`, `prior-years-results-ghsa.html`, `hytek-codes-ghsa.html`, `maxpreps-stats-ghsa.html`, `ghsa-2026-girls-6a-track-results.pdf`, `robots-ghsa.txt` (~16 files).
- **Tier C**: none for GA today.
- **Registry**: none.

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `ghsa-directory-feed.pdf` | `ga_ghsa_pdf` custom parser (school universe) | all GHSA members |
| DragonFly GHSA pages 1–3 (proto/adapters) | `ga_dragonfly*` current_staff | GHSA schools' staff; 3 pages ≈ full directory |
| `ghsa-*` results/results-records bodies | identity cross-check only | — |

### 4. True missing
`gatfxcca.com` (GA TF/XC coaches association; held name: `probes/nhstfxcca/FINDINGS.md` E1 #4; unheld) · GA private/independent athletics association (no name present in held evidence) · school athletic staff pages for emails (email 636 vs any 954).

### 5. Notes
DragonFly = current_staff (role rows), not TF/XC-specific. GHSA PDF = school universe, no coaches. `tfxc 845` is the best TF/XC dimension in this group after OH.

---

## NC — North Carolina

### 1. Matrix row
`ccd 2766 / any 463 / tfxc 438 / email 455`.

### 2. Held-evidence inventory
- **Proto rollup**: 968 / 2343 / 1047 / 905.
- **Tier B**: `www.nchsaa.org` 13 responses. Wired: `nc_nchsaa` (schools route), `nc_dragonfly` (NCHSAA directory/1), `nc_milesplit` (discovery-only).
- **Tier A, corpus**: southeast tree — `schools-nchsaa.html`, `ad-directory-nchsaa.html`, `coaches-nchsaa.html`, `sitemap-nchsaa.xml`, `wpjson-search-nchsaa.json`, `wpjson-types-nchsaa.json`, `about-nchsaa.html`, `sport-track-nchsaa.html`, `sport-cross-nchsaa.html`, `track-champs-nchsaa.html`, `xc-champs-2025-nchsaa.html`, `xc-2025-5a-girls-nchsaa.pdf`, `record-books-nchsaa.html`, `champ-*-nchsaa.html` (~15 files); national tree — `coach-directories-national/samples/assoc-home/NC__nchsaa.org.html`, `.../dir/NC__schools.html`, `.../dir/NC__athletic-directors.html` (2026-09-22; CAPTURES.md rows 96, 151–152).
- **Tier C**: none for NC today.
- **Registry**: none.

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `schools-nchsaa.html` + `dir/NC__schools.html` | `nc_nchsaa` custom parser → school rows | full NCHSAA membership list |
| `ad-directory-nchsaa.html` + `dir/NC__athletic-directors.html` | AD-directory parser | member ADs |
| `nc_dragonfly` page 1 (proto) | DragonFly current_staff | staff rows |
| `coaches-nchsaa.html` / wpjson bodies | fresh extraction | event/coach-adjacent rows |

### 4. True missing
NCISAA-class independent-school directory (no name present in held evidence) · `nc.milesplit.com` full teams crawl (adapter discovery-only) · school athletic sites for staff/email. NOTE: NC email coverage is already near its any-coverage (455/463); the gap is schools **without any row** (463/2766).

### 5. Notes
The NCHSAA schools + AD bodies are held; the 2,343 proto tf_xc rows vs matrix tfxc 438 shows the merged-output TF/XC school join is weak. No coach roster on the association beyond DragonFly.

---

## WI — Wisconsin

### 1. Matrix row
`ccd 2232 / any 605 / tfxc 511 / email 597`.

### 2. Held-evidence inventory
- **Proto rollup**: 804 / 1225 / 1223 / 673.
- **Tier B**: `schools.wiaawi.org` **710 responses**, `wiaa.finalforms.com` **1,509**, `www.mywiaa.wiaa.com` 4. Wired: `wi_wiaawi_schools` (SearchOrg directory), `wi_milesplit` (discovery-only).
- **Tier A, corpus (midwest root)**: `.../evidence/gaps/35/` — `schools/` 12 files: `searchcontacts.html` 119,123 B, `sportlist.html` 121,940 B, `report.html` 406,192 B, `page-org1.html` 181,100 B, `page-org222.html` 218,703 B, `contact-info.html` 3,769 B, `contact-names.json` 396 B, `reports-index.html`, `h1/h1b/h222/hr.txt`; `raw/` ~30 WIAA archive/sitemap bodies (`bTFarchive.html` 519,413 B, `gXCarchive.html` 520,101 B, `my-wiaa-coaches.html` 185,636 B, `sitemap-p1..p6.xml` 363,293/334,016/332,614/331,352/331,518/237,765 B, assignments/schedules/scores/standings ×16, `tourn-*` ×4, `home.html` 210,234 B); `pdfs/` 7 + `pdf-extract/` ~40 (26-file 2026 sample; `grade11_rows` sum 2,653); `ocr/**` 20 files 5,765,352 B. Index: `state-assoc-greatlakes/samples/CAPTURES.md` §B1.
- **Fixtures**: `crates/census-service/tests/fixtures/wiaa/` (`directory_letter_a.html` 20,565 B + 3 `GetDirectorySchool` pages) + `wiaa_results/` (6 files) + `milesplit/wi_teams_index.html`, `wi_roster_52649.html`.
- **Tier C**: none for WI today.
- **Registry**: none.

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| WIAA `schools/searchcontacts.html` + `sportlist.html` | fresh extraction (WIAA coach-contact list) | highest-leverage coach rows held |
| `schools/contact-names.json` + `page-org*.html` | directory-detail parser; `wi_wiaawi_schools` adapter | org pages (710 proto responses ≈ membership) |
| `wiaa.finalforms.com` 1,509 proto bodies | fresh extraction (roster/registration surface) | participating schools |
| `gaps/35/raw` results bodies | athlete/results lane | cross-check only |

### 4. True missing
`wistca.org` (WI T&F coaches assoc; nhstfxcca E1 #32) · `wisccca.com` (WI XC assoc; E1 #33) · `www.wiaa.com/schools` (403 per ALL-STATE note) · school athletic sites for emails.

### 5. Notes
`searchcontacts.html` (119 KB) is the only held WIAA body explicitly named as a contact list; prioritize it. `wiaa.finalforms.com` (1,509) is the largest held WI count but is a registration platform, not a coach directory.

---

## MN — Minnesota

### 1. Matrix row
`ccd 2859 / any 662 / tfxc 4 / email 636`.

### 2. Held-evidence inventory
- **Proto rollup**: 848 / 4 / 4 / 831 (admin-heavy).
- **Tier B**: `www.mshsl.org` **750 responses**. Wired: `mn_mshsl` + `mn_mshsl_p2`…`p14` (schools pages 0–13), `mn_mshsl_school` (per-school detail, ex. adrian-high-school), `mn_milesplit` (discovery-only).
- **Tier A, corpus**: plains tree — `mshsl-sitemap-page1-2026-09-22.xml`, `robots-mshsl.txt`; SOURCE_REPORT (`state-assoc-plains/SOURCE_REPORT.md`): **664 schools `[V]` stream**, 665 in store, 1,097 tournaments, Drupal 11 + JSON:API, PRIMARY "school+team+AD/coach", adapter `sources/mshsl` ✅.
- **Fixtures**: `crates/census-service/tests/fixtures/mshsl/**` **11 files / 68,672 B**.
- **Registry/code**: `crates/census-crawl/src/registry/table/from_mshsl.rs`.
- **Tier C**: none for MN today.

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `www.mshsl.org` 750 proto bodies | `mn_mshsl*` + `mn_mshsl_school` parsers; `from_mshsl.rs` | 664–750 schools (near-full MN membership) incl. AD/coach rows |
| `mshsl-sitemap-page1` | enumeration/verification | sitemap universe |

### 4. True missing
`mshsca.org/track` and `mshsca.org/crosscountry` (MN T&F/XC coaches associations; nhstfxcca E1 #13/14; unheld) · school athletic staff pages for sport-level coach emails.

### 5. Notes
Proto `tfxc 4` understates: state-summary shows 831 admin/school rows while the raw MSHSL bodies carry team/AD/coach data per the plains SOURCE_REPORT. **This is a re-classification/parse gap on held bodies, not a fetch gap.**

---

## TN — Tennessee

### 1. Matrix row
`ccd 1928 / any 499 / tfxc 411 / email 491`.

### 2. Held-evidence inventory
- **Proto rollup**: 2056 / 1492 / 821 / 562.
- **Tier B**: `portal.tssaa.org` **915 responses**; `tssaa.org` 2. Wired: `tn_tssaa` (portal.tssaa.org/common/directory/), `tn_dragonfly` + `tn_dragonfly_p2`, `tn_milesplit` (discovery-only).
- **Tier A, corpus** (southcentral): `tn-coaches.html`, `tn-portal-directory.html`, `tn-directory.html`, `tn-classification.html`, `tn-schools-extracted.tsv`, `tn-2025-xc-d1aaa-results.html`, `tn-champ-result-2025-d1aaa.pdf` (+`.cfm`), `tn-champs-*.html` ×5, `tn-milesplit-start.html`, `tn-sports-index.html`, `tn-track-field.html`, `tn-track-boys.html`, `tn-xc-boys.html`, `robots-portal.tssaa.org.txt`; assoc-home `TN__tssaa.org.html`. SOURCE_REPORT: **456 schools with TSSAA ids**, 151 championship PDFs since 1960-61 (AllTrax/HY-TEK).
- **Tier C**: none for TN today.
- **Registry**: none.

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `portal.tssaa.org` 915 proto bodies | `tn_tssaa` directory parser → coach rows | 456 TSSAA member schools (ids), full portal directory |
| `tn_dragonfly` p1–2 (proto) | current_staff | staff rows |
| `tn-coaches.html` / `tn-portal-directory.html` / `tn-schools-extracted.tsv` | fresh extraction/identity | cross-check |
| TSSAA result files | results lane | — |

### 4. True missing
`tntccca.com` (TN TF/XC coaches assoc; nhstfxcca E1 #26; unheld) · school athletic staff pages for email (491 email vs 499 any — email coverage is already near its any-coverage) · remaining TN schools outside TSSAA ids.

### 5. Notes
The 915-body TSSAA portal capture is the strongest single held body in this group's south-central lane; TN `tfxc 411` tracks `any 499` well.

---

## PA — Pennsylvania

### 1. Matrix row
`ccd 2949 / any 1432 / tfxc 1 / email 1415`.

### 2. Held-evidence inventory
- **Proto rollup**: 2148 / 8 / 0 / 1544 (AD-heavy; `head_coaches 0`).
- **Tier B**: `www.piaa.org` **1,475 responses**. Wired: `pa_piaa`, `pa_piaa_b`…`pa_piaa_y` (alpha batches A–Y incl. details), `pa_piaa_details` (ex. ID=12048), `pa_milesplit` (discovery-only).
- **Tier A, corpus** (midatlantic): `piaa-school-directory.html`, `piaa-school-directory-a.html`, `piaa-membership.html`, `piaa-sitemap.aspx.html`, `piaa-championship-details-xc.html`, `piaa-xc-championships.html`, `pa-milesplit-teams.html`, `robots-piaa.txt`; assoc-home `PA__piaa.org.html`.
- **Fixtures**: `crates/census-crawl/tests/fixtures/pa_piaa/` (`directory_alpha_a.html` 78.2 KB + alpha_b/z + `PROVENANCE.json` 2.2 KB).
- **Tier C, today**: `probes/230-pa-edna/` (17 bodies: `pnp-export-2.body` 732.3 KB, `public-export.body`, forms/children/robots), `probes/231-paisaa-members/` (2 captures; page 53,322 B), `probes/234-tfcaofgp-handbook/`, `probes/235-fsl-athletics/`; `extract/230-pa-edna/schools_full.json` 3.3 MB, `extract/231-paisaa-members/rows.csv` 2.9 KB (**25 rows**), `extract/234-tfcaofgp-handbook/rows.csv` 965 B, `extract/235-fsl-athletics/qualified.json` 497 B; `var/tap-230-pa-edna/`, `var/tap-231-paisaa/`, `var/tap-231-paisaa-members/`, `var/tap-234-tfcaofgp/`, `var/tap-234-tfcaofgp-handbook/`, `var/tap-235-fsl-athletics/`.
- **Registry**: SRC-229…SRC-238 (10). HELD: 229 (PIAA, HELD+ADAPTER), 230, 231, 233 (PNP export), 234, 235. DERIVED: 232 (public EdNA form), 236 (Inter-Academic), 237 (`ptfca.org`), 238 (PCL `aopathletics.org`).

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `www.piaa.org` 1,475 bodies | `pa_piaa*` alpha + details parsers | PIAA membership (public+private HS/JH) |
| `extract/230-pa-edna/schools_full.json` 3.3 MB + `pnp-export-2.body` | fresh extraction (public + private/nonpublic universe) | EdNA entities (large) |
| `extract/231-paisaa-members` (25) | rows import → independent seeds | 25 member schools |
| `extract/234-tfcaofgp` / `extract/235-fsl` | rows import | regional coach seeds (small) |

### 4. True missing
`ptfca.org` (PA TF/XC coaches assoc; registry SRC-237 DERIVED; proto 4 responses held; nhstfxcca E1 #22) · `interacathletics` (SRC-236) · `aopathletics.org` (Philadelphia Catholic League, SRC-238) · PIAA `details.aspx` coach tabs beyond sample ID=12048 · EdNA **public**-school extract form (SRC-232; only the PNP form was executed).

### 5. Notes
PA is the strongest school/email state here (1432 any, 1415 email) but `tfxc 1` is a **classification failure**, not a body gap: PIAA rows land as AD/admin (proto `head_coaches 0`), and the TF/XC-specific sources (ptfca, leagues) are exactly the ones not held. Fix = role/sport classification + ptfca/league extraction.

---

## MO — Missouri

### 1. Matrix row
`ccd 2488 / any 21 / tfxc 5 / email 14`.

### 2. Held-evidence inventory
- **Proto rollup**: 1112 / 26 / 12 / 23.
- **Tier B**: `www.mshsaa.org` 1 response; `www.mtccca.org` **10** + `mtccca.org` **6** responses. Wired: `mo_mshsaa` (HTML SchoolListing) + `mo_mshsaa_wayback` (web.archive.org SchoolListing 2025) + `mo_milesplit` (discovery-only).
- **Tier A, corpus**: plains tree — `mshsaa-school-listing-2026-09-22.html` **704.5 KB**, `robots-mshsaa.txt` (blanket `Disallow: /` for `*`). SOURCE_REPORT (`state-assoc-plains/SOURCE_REPORT.md`): **1,006 org rows / 646 HS-level / 592 full members `[V]`**, no coach source, CONDITIONAL (robots-disallowed host).
- **Tier C, today**: `probes/sources/SRC-198/FINDINGS.md` 5.1 KB + robots/manifest/sweep — coaches route (`/MySchool/Coaches.aspx?alg=11&s=85`) **robots-refused** (merged-group `Disallow: /`; refusal independently on record from 2026-09-22 `coach-directories-national/samples/CAPTURES.md` and `tools/fetch-log.jsonl`). A wave-2 bare `mshsaa` adapter was removed by Main as a misread (see FINDINGS §5).
- **Registry**: SRC-195…SRC-202 (8). HELD+ADAPTER: 195 (school listing). HELD: 198. ADAPTER+DERIVED: 199. DERIVED: 196 (`mtccca.org`), 197 (`moqualityschools.com`, 265 nonpublic stated), 200/201 (DESE), 202 (`mocsaa.com`).

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `mshsaa-school-listing-2026-09-22.html` (704.5 KB) + wayback route | `mo_mshsaa_wayback` adapter (live host refused) | 592–646 schools (full members) |
| `mtccca.org` 16 proto responses | fresh extraction (association contacts/coaches) | MTCCCA membership (unknown count) |
| `moqualityschools.com` (unheld) | future nonpublic seed | 265 nonpublic stated |

### 4. True missing
`mtccca.org` live membership list (only proto responses held) · `moqualityschools.com/member-listing1.html` · `mocsaa.com` · `dese.mo.gov` + `apps.dese.mo.gov` · `www.mshsaa.org` coach routes (robots `Disallow: /`; documented refusal — do not fetch).

### 5. Notes
MO is the worst-covered state in this group (any 21/2488). Live MSHSAA crawling is refused by robots; the school universe must come from the wayback adapter + held listing. Email held: 14 schools only.

---

## OK — Oklahoma

### 1. Matrix row
`ccd 1791 / any 42 / tfxc 7 / email 21`.

### 2. Held-evidence inventory
- **Proto rollup**: 831 / 16 / 5 / 42.
- **Tier B**: `www.ossaa.com` 4, `ossaa.com` 2, `ossaaillustrated.com` 2, `oklahoma.gov` 6 responses. Wired: `ok_ossaa_pdf` (member PDF), `ok_milesplit` (discovery-only).
- **Tier A, corpus**: southcentral tree — `ok-memberschools-2025-26.pdf` **164.9 KB** (+ `.txt`), `ok-classifications-2025-26.pdf`, `ok-athletic-directors.html`, `ok-cross-country.html`, `ok-track.html`, `ok-state-champions.html`, `ok-cc-2025-6a-state-results.pdf`, `ok-xc-page-links-note.txt`, robots (`www.ossaa.com` 302, `ossaaillustrated.com` 200). SOURCE_REPORT: **482 member schools (PDF)**, PRIMARY.
- **Tier C, today**: `probes/204-ok-directory/` (REPORT 3.5 KB; `extract/204-ok-directory/schools_full.json` **1.5 MB**), `probes/ok-ossaarankings/` (FINDINGS 4.7 KB; rows.csv 268 B; manifest/transport/raw/robots; SHA256SUMS), `probes/206-ok-xctf-coaches/` (`ohstrack.com`: **24 advisory rows, 172 evidence candidates** incl. 123 HOF honorees; SSL cert mismatch on HTTPS, HTTP robots 200/138 B empty Disallow; 9 requests exit 0), `probes/210-ok-christian-academy/`; `var/tap-204-ok-directory/`, `var/tap-ok-20261004/`.
- **Registry**: SRC-203…SRC-211 (9). HELD: 203 (OSSAARankings school tree + team pages: `Head Coach : <name>` server-rendered, no emails; route verdict "needs-adapter"), 204 (state directory). DERIVED: 205 (`ncpsa.org/directory/oklahoma/`, 55 private stated), 206 (`ohstrack.com`), 207 (OCCTCA board PDF), 208 (OSSAA classifications), 209 (`heartlandathletics`), 210 (`ocacademy.org`), 211 (`opsac.org`).

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `ok-memberschools-2025-26.pdf` | `ok_ossaa_pdf` parser | **482 member schools** |
| `ok-ossaarankings` school tree → team pages | fresh extraction/adapter (`ctl53_lblHeadCoach`) | ~490 schools; coach names, no emails; budget ≈490+490+4,000–5,000 pages per FINDINGS (only 2 team pages verified) |
| `extract/204-ok-directory/schools_full.json` (1.5 MB) | fresh extraction (identity) | school universe |
| `ohstrack.com` probe bodies | association metadata/advisory rows only | small |

### 4. True missing
`ohstrack.com` membership directory (none published; info/empty form only) · `facebook.com/OCCTCA` (association contact, E1 #21) · `ncpsa.org/directory/oklahoma/` (55 private) · `heartlandathletics`, `opsac.org`, `ocacademy.org` (DERIVED) · school sites for email; an **adapter** for the OSSAA team-page coach-name route (probe-only today).

### 5. Notes
OK coach **names** are anonymously available (Head Coach label) but **no emails anywhere**; `ohstrack.com` HTTPS is SSL-broken (HTTP only).

---

## IA — Iowa

### 1. Matrix row
`ccd 1339 / any 366 / tfxc 358 / email 11`.

### 2. Held-evidence inventory
- **Proto rollup**: 549 / 2189 / 1045 / 14.
- **Tier B**: `www.iahsaa.org` **241 responses**. Wired: `ia_iahsaa` (member-schools), `ia_milesplit` (discovery-only). SOURCE_REPORT (plains): **379 members `[V]`**, 365/369 Bound team programs 2025-26, Bound school index 444; associations publish **no coach/AD contacts**; Bound directory sample: 63 coach rows (3 with email), 20 AD rows (2 with email) across 5 schools.
- **Tier A, corpus**: plains tree — `iahsaa-member-schools-2026-09-22.html`, `robots-iahsaa.txt`/`robots-iahsaa-apex.txt`. Midwest corpus `tools/scratch-11/` (Iowa lane; index `state-assoc-greatlakes/samples/CAPTURES.md` §B5-IA): `ihsaa-members.csv` 26,522 B (**379 rows**), `p-ihsaa-track.html` 244,246 B, `p-ihsaa-xc.html` 245,203 B, `p-ihsaa-track-class.html` 236,876 B, `b-iahsaa.org_member-schools_.html` 268,218 B, `b-ia-schools.html` 638,229 B, `b-school-harlan_directory.html` 86,258 B, `b-ames-directory.html` 81,902 B, `p-ihsaa-school-ames.html` 205,795 B, `bound-ia-schools.csv` 17,786 B, `bound-track-vs-ihsaa-join.csv` 16,165 B, `ihsaa-members-without-bound-track.csv` 3,300 B, `ihsaa-members-missing-bound-track.csv` 1,550 B, `ihsaa-members-no-bound-track-2025-26.txt` 2,133 B, `ihsaa-no-bound-boystrack-2025-26.txt` 960 B, **IGHSAU** `ighsau-beds-alpha-2026-27.pdf` 69,599 B + `ighsau-tf-final-classes.pdf` 283,865 B + `ighsau-xc-final-classes.pdf` 182,054 B; plus `tools/iowa-11/`.
- **Tier C, today**: `probes/130-ia-doe/` (REPORT 3.9 KB; `extract/130-ia-doe/rows.csv` **170.3 KB**), `probes/131-ia-arcgis/` (11 bodies; `extract/131-ia-arcgis/schools_full.json` **911.1 KB**; `query-0-0.geojson` 600.7 KB), `probes/ia-ihsaa/` (`rows.csv` 26.7 KB, `school-import.csv`, `provenance.csv`), `probes/iatrackcoaches/` (FINDINGS 7.8 KB; `seeds.json` 11.9 KB; `officers.json`), `probes/gobound/FINDINGS.md` 6.1 KB (refused/blocked), `probes/sources/SRC-128/FINDINGS.md` 6.3 KB, `probes/sources/SRC-133/board.json` 1.2 KB; `var/tap-130-ia-doe/`, `var/tap-131-ia-arcgis/`, `var/tap-ia-20261004/`, `var/tap-115-iatccc/`.
- **Registry**: SRC-128…SRC-135 (8). HELD+ADAPTER: 128. HELD: 130, 131, 132 (IATC), 133 (IATC board). ADAPTER+DERIVED: 129 (Bound). DERIVED: 134 (IHSAA committees), 135 (`ighsau.org`).

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `iahsaa-member-schools-2026-09-22.html` + `scratch-11/ihsaa-members.csv` (379) | `ia_iahsaa` adapter / fresh extract | 379 IHSAA members |
| `extract/131-ia-arcgis/schools_full.json` (911 KB) + `extract/130-ia-doe/rows.csv` (170 KB) | fresh extraction (identity + DOE nonpublic) | statewide building universe |
| IGHSAU PDFs ×3 + `ighsau-member*` captures | fresh extraction (girls-union school universe) | IGHSAU members |
| `iatrackcoaches/seeds.json` + `officers.json` | association-only rows | small |
| Bound (`gobound.com/ia`) | blocked (documented refusal) | — |

### 4. True missing
`ighsau.org` (girls union; site uncaptured — only PDFs held) · `gobound.com/ia/{ihsaa,ighsau}` (refused to this client) · `iowarunjumpthrow.com` (IATC assoc; E1 #7; probe seeds held, site uncaptured) · school sites for email (**email 11/1339 is the standout gap**).

### 5. Notes
IA has a large proto tf_xc row count (2,189) but only 366 schools and 11 email schools; Bound was the only observed coach-role+email surface (3 emails/63 rows sample) and it is refused.

---

## MD — Maryland

### 1. Matrix row
`ccd 1414 / any 253 / tfxc 45 / email 87`.

### 2. Held-evidence inventory
- **Proto rollup**: 450 / 211 / 5 / 240.
- **Tier B**: `www.mpssaa.org` **203 responses**. Wired: `md_mpssaa` (school-directory), `md_mpssaa_school` (per-school AD, ex. `/school-directory/aberdeen/`), `md_dragonfly` (MPSSAA directory/1), `md_milesplit` (discovery-only).
- **Tier A, corpus** (midatlantic): `mpssaa-ad-directory.html`, `mpssaa-members.html`, `mpssaa-school-directory.html`, `mpssaa-cross-country.html`, `mpssaa-fall-championships.html`, `mpssaa-xc-2025-results.html`, `mpssaa-xc-region-schedule.html`, `sitemap-mpssaa.xml`, `sitemap-mpssaa-page.xml`, `sitemap-mpssaa-school.xml`, `md-milesplit-teams.html`, `robots-mpssaa.txt`; assoc-home `MD__mpssaa.org.html`.
- **Tier C**: none for MD today.
- **Registry**: none.

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `www.mpssaa.org` 203 bodies | `md_mpssaa` + `md_mpssaa_school` adapters | MPSSAA membership (school + AD rows) |
| `mpssaa-ad-directory.html` | fresh extraction (AD rows) | member ADs |
| `md_dragonfly` page 1 (proto) | current_staff | staff rows |

### 4. True missing
Maryland private/independent athletics association (no name present in held evidence) · school athletic staff pages for TF/XC + email (matrix tfxc 45, email 87) · `md.milesplit.com` full teams crawl.

### 5. Notes
Proto rollup shows 211 tf_xc and 240 admins for 450 schools vs matrix tfxc 45 — a merged-output classification/join artifact. MPSSAA bodies are AD-directory heavy (proto `head_coaches 5`).

---

## MA — Massachusetts

### 1. Matrix row
`ccd 1844 / any 185 / tfxc 158 / email 20`.

### 2. Held-evidence inventory
- **Proto rollup**: 622 / 407 / 17 / 42.
- **Tier B**: `mstca.org` 4 + `www.mstca.org` 4 responses; MA derived rows in `coaches.jsonl`. Wired: `ma_miaa_members` (`miaa.net/media/824` membership PDF), `ma_milesplit` (discovery-only).
- **Tier A, corpus** (midatlantic): `miaa-member-schools.html`, `miaa-school-list.pdf`, `miaa-track-xc.html`, `miaa-scores.html`, `sitemap-miaa.xml`, `robots-miaa.txt`, `ma-milesplit-teams.html`; assoc-home `MA__miaa.net.html`. ALL-STATE note: MIAA listing = 32-page PDF, landing says updated 2024-07-18; profile pages show principal + AD, not track coaches.
- **Tier C, today**: `extract/SRC-068/` — MSTCA members via Sanity API `n3gp1igb`: `rows.csv` 55.4 KB = **153 rows (154 lines)**, `excluded_rows.csv` (147 school-less), 7 raw JSON captures, `REPORT.md` 15.5 KB, `sweep.json`. Slice = members `[0...300]` of **1,417 total**; emails: 300/300 filled, **145 are `unknown@mstca.org` placeholders**; no role/sport fields. Main import: rows=153, **schools=104**, coaches=0.
- **Registry**: SRC-068…SRC-075, 088, 092, 095 (11). HELD: 068 (MSTCA), 069/070/071 (MIAA, HELD+ADAPTER / DERIVED), 075 (MIAA T&XC). ADAPTER: 088 (MilesSplit MA, HELD+ADAPTER). DERIVED: 072 (DESE), 073/074 (MassGIS), 092 (NEPSAC), 095 (NEPSTA composite).

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `extract/SRC-068` (153 rows) | import → school observations (**done**, `var/extract-import-20261004/store`) | **104 schools** |
| MSTCA Sanity API (captures 1–7) | fresh extraction of further slices (300–1417 needs new fetch) | 1,417 members total |
| `miaa-member-schools.html` + `miaa-school-list.pdf` + `media/824` | `ma_miaa_members` parser | MIAA membership |
| `miaa-track-xc.html` | identity cross-check | — |

### 4. True missing
MSTCA membership **slice 300–1417** (only 0–300 held) · `profiles.doe.mass.edu` (SRC-072) · `mass.gov` MassGIS + `services1.arcgis.com` (SRC-073/074) · `nepsac.org` (SRC-092) · NEPSTA (SRC-095) · school athletic sites for email (matrix email 20).

### 5. Notes
MSTCA rows have **no role** → school observations only; 48.3% of captured emails are placeholders. MIAA body shows AD/principal, not track coaches. Matrix MA counts exclude the SRC-068 import.

---

## CT — Connecticut

### 1. Matrix row
`ccd 1021 / any 199 / tfxc 187 / email 12`.

### 2. Held-evidence inventory
- **Proto rollup**: 579 / 642 / 641 / 255.
- **Tier B**: `ciac.fpsports.org` 9 responses. Wired: `ct_ciac_fpsports` (SchoolPages/School.aspx), `ct_ciac_directory` (Directory.aspx), `ct_milesplit` (discovery-only).
- **Tier C, today**: `probes/ciac-fpsports/` — `directory.html` **657.3 KB** + `manifest.json` 4.3 KB + `robots.txt`; `probes/ciac-dragonfly/` — `directory-p1.json` 277.7 KB, `directory-p2.json` (audit row notes "undated capture records" for this tree).
- **Tier A, corpus** (midatlantic): `ciac-school-list.aspx`, `ciac-fpsports-home.html`, `ciac-xc-boys.aspx`, `ciac-outdoor-track.aspx`, `casciac-home.html` (splash; `sitemap-casciac.xml` is a 404 page), `sitemap-ciac.xml`, `ct-milesplit-teams.html`, `robots-casciac.txt`.
- **Registry**: SRC-076…SRC-081, 089, 093, 095 (8). HELD+ADAPTER: 076 (CIAC coach directory). HELD: 089 (MilesSplit CT). DERIVED: 077 (`casciac.org` MobileDir), 078/079 (`public-edsight.ct.gov`), 080 (`chsca.org`), 081 (`fciac.net`), 093 (NEPSAC), 095 (NEPSTA).

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `probes/ciac-fpsports/directory.html` (657 KB) | `ct_ciac_directory` / `ct_ciac_fpsports` parsers (school, address, sport, role, head coach, **phone not email**) | CIAC membership (public+private) |
| `probes/ciac-dragonfly/directory-p1/p2.json` | DragonFly staff extraction | staff rows |
| `ciac-school-list.aspx` corpus body | identity cross-check | — |

### 4. True missing
`chsca.org/officers.html` (CT coaches assoc XC/track committees; SRC-080; unheld) · `fciac.net` 2021-22 coach directory PDF (SRC-081; historical, unheld) · `public-edsight.ct.gov` (SRC-078/079) · `nepsac.org` (SRC-093) · school sites for **email (12/1021)**.

### 5. Notes
CIAC source publishes phone, not email (matrix email 12). The 657 KB directory body is the strongest held CT artifact; CIAC's live UI had a login gate historically (ALL-STATE note), but the directory body is held.

---

## SC — South Carolina

### 1. Matrix row
`ccd 1290 / any 495 / tfxc 288 / email 487`.

### 2. Held-evidence inventory
- **Proto rollup**: 1619 / 1042 / 541 / 761.
- **Tier B**: `schsl.org` 12 + `www.schsl.org` 4 responses. Wired: `sc_schsl` (HTML), `sc_dragonfly` + `sc_dragonfly_p2` (SCHSL directory 1–2), `sc_milesplit` (discovery-only).
- **Tier A, corpus** (southeast): `directory-schsl.html`, `sitemap-schsl.xml`, `home-schsl.html`, `cross-country-schsl.html`, `track-field-schsl.html`, `xc-championships-schsl.html`, `xc-state-championships-2025-schsl.html`, `xc-state-champions-2026-schsl.html`, `reclassification-2026-2028-schsl.html`, `brackets-schsl.html`, `search-xc-results-schsl.html`, `class-a-schsl.html`, `class-aaaaa-schsl.html`, `rewind-schsl.html`, `champ-info-category-schsl.html`, `robots-schsl.txt`; assoc-home `SC__schsl.org.html`.
- **Knack lane evidence**: `research/sources/applicability-matrix.md` — SCHSL directory is a Knack app; `sc_knack.py` mapper covers **head/assistant/AD T/XC Boys/Girls/blank**, with proven `school519` and earlier `staff401` (later success unproven).
- **Tier C**: none for SC today.
- **Registry**: none.

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| SCHSL Knack directory (mapped lane) | `sc_knack.py` mapper over the Knack app | member schools with head/assistant/AD + T/XC Boys/Girls roles |
| `sc_dragonfly` p1–2 (proto) | current_staff | staff rows |
| `directory-schsl.html` | `sc_schsl` parser (fallback/identity) | membership |

### 4. True missing
`carolinaxc.com` (SC TF/XC assoc; nhstfxcca E1 #24; unheld) · Knack app full-staff crawl (only school519/staff401 proven) · `sc.milesplit.com` full teams crawl · school sites for email (487/495 already — email is high).

### 5. Notes
SC's Knack mapper is the highest-leverage route in the state (sport × role × side mapping); DragonFly page 2 is held.

---

## AL — Alabama

### 1. Matrix row
`ccd 1541 / any 704 / tfxc 398 / email 689`.

### 2. Held-evidence inventory
- **Proto rollup**: 990 / 2067 / 797 / 1236.
- **Tier B**: `www.ahsaa.com` 1 response (site refuses anonymous crawls), `ahsaasports.com` 4. Wired: `al_ahsaa` (HTML home; **live crawl refused — `PermissionError: robots.txt disallows https://www.ahsaa.com/`**, coverage.md line 62), `al_dragonfly` (maxinfosite AHSAA directory/1), `al_milesplit` (discovery-only).
- **Tier A, corpus** (southcentral): `al-milesplit-teams.html` + `al-milesplit-teams-ids.txt`, `al-ahsaa-2025-xc-state-live.html`, `al-xpresstiming-home.html`, `al-xpresstiming-archived-results.html`, `al-xpresstiming-results-index.html` (404), `robots-al.milesplit.com.txt`, `robots-xpresstiming.com.txt` (404), `robots-alabamarunners.com.txt` (000/empty), `wayback-cdx-ahsaa-members.txt` (39,693 B), `wayback-cdx-ahsaa-classif.txt` (9,303 B), `wayback-cdx-ahsaa-recent.txt` (000). Robots: `ahsaa.com` apex 200/**389 B** — blanket `Disallow: /` documented in the corpus notes.
- **Tier C**: none for AL today.
- **Registry**: none. **assoc-home AL is absent** from `coach-directories-national/samples/assoc-home/` (verified listing).

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `al_dragonfly` (AHSAA directory/1) | DragonFly current_staff parser | AHSAA member staff rows |
| `al-milesplit-teams.html` + ids list | MilesSplit team-index parser | school identity |
| `wayback-cdx-ahsaa-*.txt` | candidate-URL lists only (no archived bodies held) | pointers |

### 4. True missing
`www.ahsaa.com` school coach/staff pages (robots `Disallow: /` — blocked; no archived bodies held) · `www.alabamarunners.com` (robots 000/unreachable) · `www.xpresstiming.com` (results routes 404; robots 404) · Alabama private/independent athletics association (no name present in held evidence) · school athletic sites for remaining ~837 uncovered schools (any 704/1541).

### 5. Notes
AL's association site is robots-refused; current coverage rides on DragonFly + MileSplit + prototype-derived rows. The wayback CDX lists are pointers, **not** content bodies.

---

## AR — Arkansas

### 1. Matrix row
`ccd 1117 / any 494 / tfxc 293 / email 449`.

### 2. Held-evidence inventory
- **Proto rollup**: 661 / 1283 / 634 / 782.
- **Tier B**: `www.ahsaa.org` 9 responses (home body 1,444,036 B), `ahsaa.finalforms.com` 6, `ar.finalforms.com` 6. Wired: `ar_ahsaa` (HTML home), `ar_dragonfly` (ArkAA directory/1), `ar_milesplit` (discovery-only).
- **Tier A, corpus** (southcentral): `ar-schools-page.html`, `ar-aaa-sitemap.xml`, `ar-cross-country.html`, `ar-track-and-field.html`, `ar-dragonfly.html` (1,448,913 B), `ar-documents.html`, `ar-postseason-info.html` (1,456,020 B), `ar-milesplit-calendar.html`, assoc-home `AR__ahsaa.org.html`.
- **Tier C**: none for AR today.
- **Registry**: none.

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `ar-schools-page.html` + `www.ahsaa.org` proto bodies | `ar_ahsaa` HTML parser → member school list | AHSAA (AR) membership |
| `ar_dragonfly` (ArkAA directory/1) | current_staff parser | staff rows |
| `ahsaa.finalforms.com` / `ar.finalforms.com` 12 proto responses | fresh extraction (registration surface) | participating schools |

### 4. True missing
`artrackcoaches.com` (AR TF/XC coaches assoc; nhstfxcca E1 #1; unheld) · AR private/parochial athletics association (no name present in held evidence) · school athletic sites for email (449/494).

### 5. Notes
The 1.44 MB AHSAA home body is a home page, not a roster; the `ar-schools-page.html` body is the identity route for the school universe.

---

## MS — Mississippi

### 1. Matrix row
`ccd 1050 / any 521 / tfxc 316 / email 509`.

### 2. Held-evidence inventory
- **Proto rollup**: 1473 / 1571 / 565 / 740.
- **Tier B**: `misshsaa.com` **absent from the proto host table** (site blocked at crawl time). Wired: `ms_misshsaa` (HTML `https://www.misshsaa.com/`; WAF-blocked), `ms_dragonfly` + `ms_dragonfly_p2` (MHSAA directory 1–2), `ms_milesplit` (discovery-only).
- **Tier A, corpus** (southcentral): `ms-milesplit-teams.html` + `ms-milesplit-teams-ids.txt` (**463 team ids**), `ms-2019-1a-girls-xc-results-wayback.html`, `wayback-cdx-misshsaa-members.txt` (20,280 B; notes `sgcaptcha` bot-shield interstitials), `robots-misshsaa.com.txt` (**403**), `robots-www.misshsaa.com.txt` (403), `ms-misshsaa-home.html` (403/57 B), `ms-misshsaa-home-apex.html` (403/57 B); assoc-home `MS__misshsaa.com.html` (**403/57 B**). SOURCE_REPORT: proxy = 463 MilesSplit MS team pages; MHSAA site "blocked (HTTP 403); UNVERIFIED".
- **Knack lane note** (`applicability-matrix.md`): earlier MS Knack app 61324393 with **7,703 staff rows / 1,205 TF/XC positions (705 varsity) / 383 AD** — salted raw `api.knack.com__99f0fe5c5f06f9a6fa6de379` locator only; success not established here.
- **Tier C**: none for MS today.
- **Registry**: none.

### 3. Fill plan
| Held body | Route | Estimated reach |
|---|---|---|
| `ms_dragonfly` p1–2 (proto) | DragonFly current_staff parser | MHSAA member staff rows |
| `ms-milesplit-teams-ids.txt` (463) | MilesSplit identity join | 463 team programs |
| `wayback-cdx-misshsaa-members.txt` | candidate-URL list only (sgcaptcha) | pointers |
| MS Knack app (locator only) | future Knack parser if reacquired | 7,703 staff rows per earlier lane note |

### 4. True missing
`misshsaa.com` / `www.misshsaa.com` (robots 403 + home 403 **WAF/bot-shield**; Wayback shows sgcaptcha interstitials) · MS school athletic sites for coach/email at scale (matrix email 509/521 is already near any-coverage, but any 521/1050 = half the state uncovered) · a coach-association seed (MS absent from nhstfxcca E1).

### 5. Notes
MS is a documented-blocked state: the association site is the least reachable in this group; DragonFly is the only current-staff route held, and the earlier Knack app is a locator, not a held body.

---

## Group closing numbers

| st | ccd | any | tfxc | email | proto schools | proto tf_xc | Held state body (largest) | Today's taps |
|---|---:|---:|---:|---:|---:|---:|---|---|
| OH | 3611 | 788 | 747 | 779 | 1155 | 1698 | OATCCC 2,141 rows → 630 schools; OHSAA 815 rows; myOHSAA 1,564 | SRC-100, 097-oh-ohsaa-seed, tap-101-oh-oeds |
| GA | 2349 | 954 | 845 | 636 | 3074 | 4923 | GHSA feed PDF + DragonFly p1-3 | — |
| NC | 2766 | 463 | 438 | 455 | 968 | 2343 | NCHSAA schools/AD dir | — |
| WI | 2232 | 605 | 511 | 597 | 804 | 1225 | WIAA 710 org pages + finalforms 1,509 | — |
| MN | 2859 | 662 | 4 | 636 | 848 | 4 | MSHSL 750 bodies (664 schools) | — |
| TN | 1928 | 499 | 411 | 491 | 2056 | 1492 | TSSAA portal 915 bodies (456 ids) | — |
| PA | 2949 | 1432 | 1 | 1415 | 2148 | 8 | PIAA 1,475 bodies; EdNA 3.3 MB | 230/231/234/235 |
| MO | 2488 | 21 | 5 | 14 | 1112 | 26 | MSHSAA listing 704.5 KB (wayback) | SRC-198 (refusal) |
| OK | 1791 | 42 | 7 | 21 | 831 | 16 | OSSAA PDF 482 schools; rankings probe | 204, ok-20261004 |
| IA | 1339 | 366 | 358 | 11 | 549 | 2189 | IHSAA 241 + scratch-11 379 rows | 130, 131, ia-20261004, 115 |
| MD | 1414 | 253 | 45 | 87 | 450 | 211 | MPSSAA 203 bodies | — |
| MA | 1844 | 185 | 158 | 20 | 622 | 407 | MIAA PDF; MSTCA 153 rows → 104 schools | SRC-068 |
| CT | 1021 | 199 | 187 | 12 | 579 | 642 | CIAC directory 657 KB | ciac-fpsports, ciac-dragonfly |
| SC | 1290 | 495 | 288 | 487 | 1619 | 1042 | SCHSL Knack lane + DragonFly p1-2 | — |
| AL | 1541 | 704 | 398 | 689 | 990 | 2067 | DragonFly AHSAA p1 | — |
| AR | 1117 | 494 | 293 | 449 | 661 | 1283 | AHSAA schools page + 9 proto bodies | — |
| MS | 1050 | 521 | 316 | 509 | 1473 | 1571 | DragonFly MHSAA p1-2 + 463 teams | — |

**Cross-state notes**
1. **Do not fetch blocked hosts**: `www.mshsaa.org` (MO, robots `Disallow: /`), `www.ahsaa.com` (AL, robots `Disallow: /`), `misshsaa.com` (MS, 403 WAF + sgcaptcha), `gobound.com/ia` (IA, refused). Wayback routes exist for MO (`mo_mshsaa_wayback` adapter) and CDX lists for AL/MS (pointers only).
2. **Classification, not fetch, for several gaps**: MN (tfxc 4 vs 831 admins), PA (tfxc 1 vs 1,415 emails), MD (tfxc 45 vs 211), MO/OK/AL/AR `tfxc≈0–7` — the held bodies carry roles/sports but the merged-output TF/XC join under-counts. Re-parse held bodies first.
3. **Email deserts**: CT (12/1021), MA (20/1844), IA (11/1339), MO (14/2488), MD (87/1414) — sources publish phones/names only; email closure needs school athletic sites, not association bodies.
4. **Coach-role import blocker**: SRC-100 (OH) and SRC-068 (MA) both imported as **school observations** (`rows_without_coach_role`), because the source rows carry no role; do not count them as coach rows.
5. **Unheld TF/XC coach-association seeds** (from `probes/nhstfxcca/FINDINGS.md` E1): AR artrackcoaches.com · GA gatfxcca.com · IA iowarunjumpthrow.com · MN mshsca.org (×2) · MO mtccca.org · OK facebook.com/OCCTCA · PA ptfca.org · SC carolinaxc.com · TN tntccca.com · WI wistca.org + wisccca.com. No seed in E1 exists for AL, NC, MD, MS, CT.
