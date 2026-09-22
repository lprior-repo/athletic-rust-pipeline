# CAPTURES - state-assoc-greatlakes (IL, IN, MI, OH, WI)

**This lane performed no network fetches.** Every byte below was captured by the Midwest TF/XC
research wave on 2026-09-19..21 and already exists on disk; this file maps each capture this report
leans on to its path and size, and - where the wave kept a manifest - to the URL, HTTP status and
fetch timestamp recorded at capture time. Nothing here is re-fetched or re-timestamped by this lane.

Verification split, stated once and true for every row:
- **bytes**: measured by this lane 2026-09-21 with `os.walk` + `os.path.getsize` over the exact paths
  below (command at the bottom). Where a manifest also logs a byte count, both are shown rather than
  averaged, and a disagreement is called out.
- **URL / status / timestamp**: quoted from the owning wave manifest, *named in the row*
  (`fetch-log.jsonl`, `appendix-table.md`, `appendix.md`, ...) or marked `no retained manifest`.

Files this report cites or that back a quantified claim are listed **individually**; bulk directories
are listed as one row with a file count plus the manifest that indexes them, so nothing here is a
black box and nothing is duplicated.

## A. Repo fixtures (in-repo, read-only)

| Path (repo-relative) | Bytes | Backs |
|---|---:|---|
| `crates/midwest-census/tests/fixtures/ihsa/staff2_coach_rich.json` | 5387 | IHSA API shape: `v1_schools.json` (828 member rows), `staff2_coach_rich.json` (role rows + `HasEmail`), `staff2_office_only.json` (office-only variant) - report section 2 |
| `crates/midwest-census/tests/fixtures/ihsa/staff2_office_only.json` | 1347 | IHSA API shape: `v1_schools.json` (828 member rows), `staff2_coach_rich.json` (role rows + `HasEmail`), `staff2_office_only.json` (office-only variant) - report section 2 |
| `crates/midwest-census/tests/fixtures/ihsa/v1_schools.json` | 2019 | IHSA API shape: `v1_schools.json` (828 member rows), `staff2_coach_rich.json` (role rows + `HasEmail`), `staff2_office_only.json` (office-only variant) - report section 2 |
| `crates/midwest-census/tests/fixtures/ohsaa/ad_centerville.html` | 2594 | OH portal HTML shapes incl. the negative fixtures (`*_malformed.html`, `search_no_results.html`) - section 5 |
| `crates/midwest-census/tests/fixtures/ohsaa/ad_dublin_coffman.html` | 3275 | OH portal HTML shapes incl. the negative fixtures (`*_malformed.html`, `search_no_results.html`) - section 5 |
| `crates/midwest-census/tests/fixtures/ohsaa/ad_malformed.html` | 313 | OH portal HTML shapes incl. the negative fixtures (`*_malformed.html`, `search_no_results.html`) - section 5 |
| `crates/midwest-census/tests/fixtures/ohsaa/search_dublin_coffman.html` | 1112 | OH portal HTML shapes incl. the negative fixtures (`*_malformed.html`, `search_no_results.html`) - section 5 |
| `crates/midwest-census/tests/fixtures/ohsaa/search_duplicate_rows.html` | 2667 | OH portal HTML shapes incl. the negative fixtures (`*_malformed.html`, `search_no_results.html`) - section 5 |
| `crates/midwest-census/tests/fixtures/ohsaa/search_no_results.html` | 393 | OH portal HTML shapes incl. the negative fixtures (`*_malformed.html`, `search_no_results.html`) - section 5 |
| `crates/midwest-census/tests/fixtures/ohsaa/sports_centerville.html` | 13463 | OH portal HTML shapes incl. the negative fixtures (`*_malformed.html`, `search_no_results.html`) - section 5 |
| `crates/midwest-census/tests/fixtures/ohsaa/sports_dublin_coffman.html` | 14294 | OH portal HTML shapes incl. the negative fixtures (`*_malformed.html`, `search_no_results.html`) - section 5 |
| `crates/midwest-census/tests/fixtures/ohsaa/sports_malformed.html` | 342 | OH portal HTML shapes incl. the negative fixtures (`*_malformed.html`, `search_no_results.html`) - section 5 |
| `crates/midwest-census/tests/fixtures/wiaa/directory_letter_a.html` | 20565 | WIAA directory letter + 3 `GetDirectorySchool` detail pages (orgID 1 / 135 / 5151) - section 6 |
| `crates/midwest-census/tests/fixtures/wiaa/school_org135_gale_ettrick_trempealeau.html` | 54061 | WIAA directory letter + 3 `GetDirectorySchool` detail pages (orgID 1 / 135 / 5151) - section 6 |
| `crates/midwest-census/tests/fixtures/wiaa/school_org1_abbotsford.html` | 39262 | WIAA directory letter + 3 `GetDirectorySchool` detail pages (orgID 1 / 135 / 5151) - section 6 |
| `crates/midwest-census/tests/fixtures/wiaa/school_org5151_sails_charter.html` | 19393 | WIAA directory letter + 3 `GetDirectorySchool` detail pages (orgID 1 / 135 / 5151) - section 6 |
| `crates/midwest-census/tests/fixtures/wiaa_results/d1boysstateresults-dash.htm` | 17337 | WIAA result-file formats: Hy-Tek `.htm`, HTML `.htm`, legacy `.txt`, seed column - section 6 |
| `crates/midwest-census/tests/fixtures/wiaa_results/d1boysstateresults-dash.txt` | 3972 | WIAA result-file formats: Hy-Tek `.htm`, HTML `.htm`, legacy `.txt`, seed column - section 6 |
| `crates/midwest-census/tests/fixtures/wiaa_results/d1boysstateresults-sections.htm` | 16143 | WIAA result-file formats: Hy-Tek `.htm`, HTML `.htm`, legacy `.txt`, seed column - section 6 |
| `crates/midwest-census/tests/fixtures/wiaa_results/racinesectionalb-finish-list.htm` | 33560 | WIAA result-file formats: Hy-Tek `.htm`, HTML `.htm`, legacy `.txt`, seed column - section 6 |
| `crates/midwest-census/tests/fixtures/wiaa_results/seed-column-regional.htm` | 10176 | WIAA result-file formats: Hy-Tek `.htm`, HTML `.htm`, legacy `.txt`, seed column - section 6 |
| `crates/midwest-census/tests/fixtures/wiaa_results/trackside-regional.htm` | 3286 | WIAA result-file formats: Hy-Tek `.htm`, HTML `.htm`, legacy `.txt`, seed column - section 6 |
| `crates/midwest-census/tests/fixtures/milesplit/wi_roster_52649.html` | 47647 | MileSplit team index + roster page carrying `column-grad-year` (team 52649) - sections 2/6 |
| `crates/midwest-census/tests/fixtures/milesplit/wi_teams_index.html` | 4801 | MileSplit team index + roster page carrying `column-grad-year` (team 52649) - sections 2/6 |
| `crates/midwest-census/tests/fixtures/mshsl/**` (11 files) | 68672 | **MN, not one of this lane's five jurisdictions** - kept only because the section 7.3 ledger measured it; cited in none of the five state sections |
| `crates/midwest-census/tests/fixtures/coach_contacts_sample.csv` | 2926 | coach-CSV importer fixture (shared, cross-lane) |

## B. Midwest-corpus captures retained on disk (bytes re-measured; provenance from wave manifests)

### B1. Wisconsin - `research/midwest/evidence/gaps/35/` (report [35]; index `pdf-extract/summary-v2.json`)

| Path (corpus-relative) | Bytes | What it is / what it backs |
|---|---:|---|
| `evidence/gaps/35/pdfs/instr-track-athleticnet-2026.pdf` | 239369 | retained raw WIAA tournament PDF (section 6 result files) |
| `evidence/gaps/35/pdfs/instr-xc-entries-2026.pdf` | 81950 | retained raw WIAA tournament PDF (section 6 result files) |
| `evidence/gaps/35/pdfs/tr2026beaverdamsectionalindiv.pdf` | 2646780 | retained raw WIAA tournament PDF (section 6 result files) |
| `evidence/gaps/35/pdfs/tr2026hilbertsectionalindiv.pdf` | 1064667 | retained raw WIAA tournament PDF (section 6 result files) |
| `evidence/gaps/35/pdfs/tr2026madisonregional.pdf` | 524477 | retained raw WIAA tournament PDF (section 6 result files) |
| `evidence/gaps/35/pdfs/tr2026marathonsectional.pdf` | 1566480 | retained raw WIAA tournament PDF (section 6 result files) |
| `evidence/gaps/35/pdfs/trb2026stratfordregional.pdf` | 1012583 | retained raw WIAA tournament PDF (section 6 result files) |
| `evidence/gaps/35/pdf-extract/summary-v2.json` | 11054 | per-file extract stats for the 26-file 2026 sample: `grade11_rows` sums to **2653** over 26 entries (23 non-zero). This is the retained number this report prints beside [35]'s tolerant-matcher 2,786 (section 6.1) |
| `evidence/gaps/35/pdf-extract/summary.json` | 6783 | earlier 16-entry summary (all `grade11_rows` 0; superseded by summary-v2) |
| `evidence/gaps/35/pdf-extract/instr-track-athleticnet-2026.txt` | 2744 | `pdftotext` extract of `instr-track-athleticnet-2026.pdf` - no summary-v2 entry |
| `evidence/gaps/35/pdf-extract/instr-xc-entries-2026.txt` | 2347 | `pdftotext` extract of `instr-xc-entries-2026.pdf` - no summary-v2 entry |
| `evidence/gaps/35/pdf-extract/sample-final.json` | 10919 | `pdftotext` extract of `sample-final..pdf` - no summary-v2 entry |
| `evidence/gaps/35/pdf-extract/tr2026beaverdamsectionalindiv.txt` | 7 | `pdftotext` extract of `tr2026beaverdamsectionalindiv.pdf` - grade-11 rows **0**, pages 7, verdict `MOJIBAKE/raster` |
| `evidence/gaps/35/pdf-extract/tr2026chiltonsectionalindiv.txt` | 115018 | `pdftotext` extract of `tr2026chiltonsectionalindiv.pdf` - grade-11 rows **154**, pages 19, verdict `MOJIBAKE/raster` |
| `evidence/gaps/35/pdf-extract/tr2026colfaxregional.txt` | 82253 | `pdftotext` extract of `tr2026colfaxregional.pdf` - grade-11 rows **166**, pages 10, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026franklinregional.txt` | 65803 | `pdftotext` extract of `tr2026franklinregional.pdf` - grade-11 rows **105**, pages 13, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026graftonregional.txt` | 81024 | `pdftotext` extract of `tr2026graftonregional.pdf` - grade-11 rows **121**, pages 15, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026greendaleregional.txt` | 80761 | `pdftotext` extract of `tr2026greendaleregional.pdf` - grade-11 rows **150**, pages 27, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026hilbertregional.txt` | 103469 | `pdftotext` extract of `tr2026hilbertregional.pdf` - grade-11 rows **248**, pages 32, verdict `MOJIBAKE/raster` |
| `evidence/gaps/35/pdf-extract/tr2026hilbertsectionalindiv.txt` | 57254 | `pdftotext` extract of `tr2026hilbertsectionalindiv.pdf` - grade-11 rows **1**, pages 12, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026homesteadsectional.txt` | 59771 | `pdftotext` extract of `tr2026homesteadsectional.pdf` - grade-11 rows **69**, pages 12, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026horiconsectionalindiv.txt` | 138046 | `pdftotext` extract of `tr2026horiconsectionalindiv.pdf` - grade-11 rows **206**, pages 20, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026kielregionalindiv.txt` | 87926 | `pdftotext` extract of `tr2026kielregionalindiv.pdf` - grade-11 rows **182**, pages 13, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026madisonregional.txt` | 69231 | `pdftotext` extract of `tr2026madisonregional.pdf` - grade-11 rows **144**, pages 9, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026marathonsectional.txt` | 123327 | `pdftotext` extract of `tr2026marathonsectional.pdf` - grade-11 rows **0**, pages 22, verdict `MOJIBAKE/raster` |
| `evidence/gaps/35/pdf-extract/tr2026mondoviregional.txt` | 86146 | `pdftotext` extract of `tr2026mondoviregional.pdf` - grade-11 rows **5**, pages 13, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026mukwonagosectional.txt` | 61656 | `pdftotext` extract of `tr2026mukwonagosectional.pdf` - grade-11 rows **65**, pages 12, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026neenahregional-.txt` | 80470 | `pdftotext` extract of `tr2026neenahregional-.pdf` - grade-11 rows **116**, pages 15, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026pittsvilleregional.txt` | 87925 | `pdftotext` extract of `tr2026pittsvilleregional.pdf` - grade-11 rows **218**, pages 13, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026ricelakesectional.txt` | 82182 | `pdftotext` extract of `tr2026ricelakesectional.pdf` - grade-11 rows **137**, pages 19, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026shorewoodregional.txt` | 36517 | `pdftotext` extract of `tr2026shorewoodregional.pdf` - grade-11 rows **51**, pages 5, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026southmilwsectional.txt` | 60224 | `pdftotext` extract of `tr2026southmilwsectional.pdf` - grade-11 rows **59**, pages 11, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026stratfordregionalb.txt` | 0 | `pdftotext` extract of `tr2026stratfordregionalb.pdf` - grade-11 rows **0**, pages 0, verdict `MOJIBAKE/raster` |
| `evidence/gaps/35/pdf-extract/tr2026suringregional.txt` | 70858 | `pdftotext` extract of `tr2026suringregional.pdf` - grade-11 rows **115**, pages 8, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026veronasectional.txt` | 48731 | `pdftotext` extract of `tr2026veronasectional.pdf` - grade-11 rows **87**, pages 7, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026waupacaregional.txt` | 59948 | `pdftotext` extract of `tr2026waupacaregional.pdf` - grade-11 rows **98**, pages 8, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/tr2026westdeperesectionalindiv.txt` | 53766 | `pdftotext` extract of `tr2026westdeperesectionalindiv.pdf` - grade-11 rows **117**, pages 7, verdict `extractable` |
| `evidence/gaps/35/pdf-extract/trb2026altoonaregional.txt` | 45169 | `pdftotext` extract of `trb2026altoonaregional.pdf` - grade-11 rows **39**, pages 13, verdict `MOJIBAKE/raster` |
| `evidence/gaps/35/pdf-extract/trb2026stratfordregional.txt` | 57678 | `pdftotext` extract of `trb2026stratfordregional.pdf` - no summary-v2 entry |
| `evidence/gaps/35/raw/bTFarchive.html` | 519413 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/bTFtournament.html` | 224999 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/bXCarchive.html` | 528717 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/bXCtournament.html` | 231064 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/boys-cross-country-assignments.html` | 175727 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/boys-cross-country-schedules.html` | 182729 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/boys-cross-country-scores.html` | 181792 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/boys-cross-country-standings.html` | 181837 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/boys-track-field-assignments.html` | 175719 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/boys-track-field-schedules.html` | 182924 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/boys-track-field-scores.html` | 181821 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/boys-track-field-standings.html` | 181866 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/gTFarchive.html` | 514445 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/gTFtournament.html` | 235629 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/gXCarchive.html` | 520101 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/gXCtournament.html` | 223620 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/girls-cross-country-assignments.html` | 175731 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/girls-cross-country-schedules.html` | 182770 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/girls-cross-country-scores.html` | 181838 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/girls-cross-country-standings.html` | 181883 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/girls-track-field-assignments.html` | 175723 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/girls-track-field-schedules.html` | 182970 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/girls-track-field-scores.html` | 181867 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/girls-track-field-standings.html` | 181912 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/home.html` | 210234 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/my-wiaa-coaches.html` | 185636 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/sitemap-p1.xml` | 363293 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/sitemap-p2.xml` | 334016 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/sitemap-p3.xml` | 332614 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/sitemap-p4.xml` | 331352 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/sitemap-p5.xml` | 331518 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/sitemap-p6.xml` | 237765 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/sitemap.xml` | 1048 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/tourn-boys-cross-country-assignments.html` | 179680 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/tourn-boys-track-field-assignments.html` | 180259 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/tourn-girls-cross-country-assignments.html` | 179710 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/raw/tourn-girls-track-field-assignments.html` | 179771 | WIAA archive/sitemap capture (section 6 "104 result links/season"; section 6.1 file counts) |
| `evidence/gaps/35/schools/contact-info.html` | 3769 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/contact-names.json` | 396 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/h1.txt` | 748 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/h1b.txt` | 754 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/h222.txt` | 756 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/hr.txt` | 756 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/page-org1.html` | 181100 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/page-org222.html` | 218703 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/report.html` | 406192 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/reports-index.html` | 1233 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/searchcontacts.html` | 119123 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/schools/sportlist.html` | 121940 | WIAA `GetDirectorySchool` / directory-letter capture - section 6 coach lane (9-school sample) |
| `evidence/gaps/35/ocr/**` (20 files) | 5765352 | tesseract OCR of the 2 non-extractable PDFs (1.06 s/page claim) |

### B2. Illinois - `research/midwest/evidence/gaps/38/` (report [38]; provenance `appendix-table.md`, URL/status/timestamp per fetch)

| Path (corpus-relative) | Bytes | What it is / what it backs |
|---|---:|---|
| `evidence/gaps/38/appendix-table.md` | 11792 | the URL / method / HTTP status / what-it-proved / timestamp table behind section 2.1 (staff2 sample, cc-qualifiers ids 688-693, `/v1/schools` 828 rows) |
| `evidence/gaps/38/cc-qualifiers-2024-25-691.json` | 88 | archived-term XC qualifiers (empty-array proof) |
| `evidence/gaps/38/cc-qualifiers-2026-27-691.json` | 50 | current-term 404 body (archive not available for term 2026-27) |
| `evidence/gaps/38/cc-qualifiers-no-tid.txt` | 32 | 400 missing-tournamentId body - XC ids are not API-enumerable |
| `evidence/gaps/38/coach-rates.json` | 1099 | coach email-reveal response rates (section 2.2 staff2 lane) |
| `evidence/gaps/38/coach-sample-ids.json` | 364 | the school ids sampled for staff2 |
| `evidence/gaps/38/girls-xc-tournament-info.json` | 1183 | current-term girls XC final info (Nov 7 2026, Detweiler Park) |
| `evidence/gaps/38/ihsa-girls-xc-xhr-capture.json` | 1112 | headless-browser XHR capture proving the page calls the cc-qualifiers ids |
| `evidence/gaps/38/reveal-hasemail-false-probe.json` | 117 | `HasEmail:false` probe |
| `evidence/gaps/38/staff2-0120.json` | 4535 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-0219.json` | 4621 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-0302.json` | 2617 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-0337.json` | 6104 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-0415.json` | 12132 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-0523.json` | 4627 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-0705.json` | 3348 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-0809.json` | 2948 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-1103.json` | 3030 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-1232.json` | 8855 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-1329.json` | 5732 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-1366.json` | 2358 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-1504.json` | 10069 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-1618.json` | 2129 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-1803.json` | 4243 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-1903.json` | 3139 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-1941.json` | 5335 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-2205.json` | 5264 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-2330.json` | 6126 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-2720.json` | 4315 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-2759.json` | 5804 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-2817.json` | 7657 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-2924.json` | 3283 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/staff2-2966.json` | 1123 | staff2 sample payload (role rows + `HasEmail`) |
| `evidence/gaps/38/tfrrs-list-4524.html` | 1626863 | TFRRS list page (IL DAT/TFRRS lane) |
| `evidence/gaps/38/il-schools.json` | 451188 | **the IL school-universe payload itself**: 828 rows = 801 `full member` + 26 `approved school` + 1 `associate member`; 604 `Boundary` / 224 `Non-Boundary`; 677 `public` / 151 `private`; 126 `isCPS`; `SchoolID` 0101-7097; minified, no trailing whitespace, md5 `498179f68c66793396dd4c6123a4a4bc`. **Byte disagreement kept as-is:** this lane's ledger records the same fetch as 451182 B (6 B fewer) while the file and [13] both say 451188 - printed, not averaged, cause not established from the retained evidence. Byte-identical copy: `tools/a13-ihsa/schools.json` |
| `evidence/gaps/38/ledger.jsonl` | 13010 | **machine-readable fetch ledger for this whole dir**: 65 entries of `{url, host, method, status, bytes, out, ts}` (e.g. `api.ihsa.org/v1/schools` 200 at `2026-09-20T09:08:06-0500`) - the URL/status/timestamp source behind the staff2 + cc-qualifiers rows below |
| `evidence/gaps/38/classification-2026-27-fall.json` | 2271933 | fall classification payload = the `?season=F` response [13] measured (2271933 B) - the CCB 539 / CCG 507 sponsorship counts |
| `evidence/gaps/38/classification-2026-27-spring.json` | 1789503 | spring classification payload = the `?season=S` response [13] measured (1789503 B) - the TRB 632 / TRG 624 sponsorship counts |
| `evidence/gaps/38/cc-qualifiers-2025-26-691.json` | 79632 | XC qualifiers for the **one** term that exists (2025-26, Class 1A tournament 691); the 2026-27 request returned the 50 B 404 body listed above |
| `evidence/gaps/38/coach-survey.json` | 211189 | staff2 survey aggregate over the 24-school sample (`{"sample":[…24 SchoolIDs…],"schools":{id:{status,roles:{role:{n,has_email,…}}}}}`) - the IL coach-role/email yield behind section 2.1 |
| `evidence/gaps/38/**` (remaining 17 of 57 files, 8146413 B) | - | `da-*` + `ihsa-*-bundle.js` assets, the remaining `cc-qualifiers-2025-26-*` terms, and further staff2/email samples; indexed by `ledger.jsonl` and `appendix-table.md` |

### B3. Michigan - `research/midwest/evidence/gaps/39/` (report [39]; machine-readable provenance `fetch-log.jsonl`)

`fetch-log.jsonl` is the strongest manifest in this lane: one JSON object per fetch carrying `ts`,
`url`, `method`, `status`, `bytes`, `sha256` (16-hex prefix), `saved` and `secs`. Two lines quoted
verbatim from it (the 2026 regional-results page the section 4 tournament map rests on, and a 404
control):

```json
{"ts": "2026-09-20T09:05:59-0500", "url": "https://www.mhsaa.com/sports/boys-track-field/2026-mhsaa-track-field-regional-results", "method": "GET", "status": 200, "bytes": 138502, "ctype": "text/html; charset=UTF-8", "last_modified": null, "etag": null, "sha256": "c42ab2447a31fe96", "saved": "www.mhsaa.com_sports_boys-track-field_2026-mhsaa-track-field-regional-results", "error": null, "secs": 1.3}
{"ts": "2026-09-20T09:06:00-0500", "url": "https://www.mhsaa.com/sports/girls-track-field/2026-mhsaa-track-field-regional-results", "method": "GET", "status": 404, "bytes": 0, "ctype": "text/html; charset=UTF-8", "last_modified": null, "etag": null, "sha256": null, "saved": null, "error": "HTTP 404", "secs": 0.77}
```

| Path (corpus-relative) | Bytes | What it is / what it backs |
|---|---:|---|
| `evidence/gaps/39/2024-upd2girls.txt` | 27305 | UP D2 girls finals text extract (2024) |
| `evidence/gaps/39/2025-upd2girls.txt` | 23220 | UP D2 girls finals text extract (2025) |
| `evidence/gaps/39/appendix-extra.md` | 1886 | MI appendix supplement |
| `evidence/gaps/39/appendix.md` | 18325 | MI appendix (URL / status / what it proved / timestamp per fetch) |
| `evidence/gaps/39/es-michiana-indoor-count.json` | 161 | Michiana indoor meet count |
| `evidence/gaps/39/es-michiana-mi.json` | 163 | Michiana MI bucket count |
| `evidence/gaps/39/es-michiana-sample.json` | 5232 | Michiana Timing Elasticsearch sample (MI timer lane) |
| `evidence/gaps/39/es-michiana-total.json` | 163 | Michiana total bucket count |
| `evidence/gaps/39/fetch-log.jsonl` | 43680 | the 124-line MI fetch manifest itself (URL/status/bytes/sha256/ts per request) |
| `evidence/gaps/39/live.michianatiming.com_robots.txt` | 192 | Michiana Timing robots.txt (pre-fetch compliance) |
| `evidence/gaps/39/mhsaa_an_link_inventory.json` | 6574 | MHSAA to Athletic.net MeetID inventory for the 2026 tournament set (section 4) |
| `evidence/gaps/39/milesplit-indoor-mits-meets.json` | 6569 | MileSplit MITS indoor meet list (cross-lane) |
| `evidence/gaps/39/s-gke-usc1-nssi3-33.firebaseio.com_meet_12574_event_summary.json_ns_trackmeet-io` | 13831 | trackmeet.io Firebase event summary (MI timer lane) |
| `evidence/gaps/39/s-gke-usc1-nssi3-33.firebaseio.com_meet_30103_event_summary.json_ns_trackmeet-io` | 10884 | trackmeet.io Firebase event summary (MI timer lane) |
| `evidence/gaps/39/s-gke-usc1-nssi3-33.firebaseio.com_meet_43298_event_summary.json_ns_trackmeet-io` | 12779 | trackmeet.io Firebase event summary (MI timer lane) |
| `evidence/gaps/39/s-gke-usc1-nssi3-33.firebaseio.com_meet_43408_event_summary.json_ns_trackmeet-io` | 17472 | trackmeet.io Firebase event summary (MI timer lane) |
| `evidence/gaps/39/s-gke-usc1-nssi3-33.firebaseio.com_meet_43498_event_summary.json_ns_trackmeet-io` | 14023 | trackmeet.io Firebase event summary (MI timer lane) |
| `evidence/gaps/39/s-gke-usc1-nssi3-33.firebaseio.com_meet_4942_event_summary.json_ns_trackmeet-io` | 8934 | trackmeet.io Firebase event summary (MI timer lane) |
| `evidence/gaps/39/s-gke-usc1-nssi3-33.firebaseio.com_meet_59759_event_summary.json_ns_trackmeet-io` | 13670 | trackmeet.io Firebase event summary (MI timer lane) |
| `evidence/gaps/39/s-gke-usc1-nssi3-33.firebaseio.com_meet_60442_event_summary.json_ns_trackmeet-io` | 14646 | trackmeet.io Firebase event summary (MI timer lane) |
| `evidence/gaps/39/s-gke-usc1-nssi3-33.firebaseio.com_meet_61710_event_summary.json_ns_trackmeet-io` | 19932 | trackmeet.io Firebase event summary (MI timer lane) |
| `evidence/gaps/39/s-gke-usc1-nssi3-33.firebaseio.com_meet_61819_event_summary.json_ns_trackmeet-io` | 24943 | trackmeet.io Firebase event summary (MI timer lane) |
| `evidence/gaps/39/up-girls-grade-counts-2024-2026.json` | 305 | UP finals grade counts across 2024-2026 (the grade-bearing-PDF proof) |
| `evidence/gaps/39/up-girls-grade-counts.json` | 575 | UP finals grade counts |
| `evidence/gaps/39/www.mhsaa.com_sites_default_files_Track_20Field-Boys_2024_Finals_UP-D2-Girls.pdf` | 581244 | MHSAA finals PDF capture (section 4 grade evidence) |
| `evidence/gaps/39/www.mhsaa.com_sites_default_files_Track_20Field-Boys_2025_Finals_Up-D2-Girls.pdf` | 437674 | MHSAA finals PDF capture (section 4 grade evidence) |
| `evidence/gaps/39/www.mhsaa.com_sites_default_files_Track_20Field-Boys_2026_Finals_2026-UP-Girls-D2-Finals.pdf_time_1780175792370` | 203062 | MHSAA finals PDF capture (section 4 grade evidence) |
| `evidence/gaps/39/www.mhsaa.com_sites_default_files_Track_20Field-Boys_2026_Finals_2026-UP-Girls-D2-Finals.pdf_time_1780175792370.txt` | 32781 | MHSAA finals PDF capture (section 4 grade evidence) |
| `evidence/gaps/39/www.mhsaa.com_sites_default_files_Track_20Field-Boys_2026_Finals_2026-UP-Girls-D3-Finals.pdf_time_1780175792370` | 269086 | MHSAA finals PDF capture (section 4 grade evidence) |
| `evidence/gaps/39/www.mhsaa.com_sites_default_files_Track_20Field-Boys_2026_Finals_2026-UP-Girls-D3-Finals.pdf_time_1780175792370.txt` | 57816 | MHSAA finals PDF capture (section 4 grade evidence) |
| `evidence/gaps/39/**` (remaining 74 of 104 files, 5271461 B) | - | further MI probes/season JSON; every network fetch is indexed in `fetch-log.jsonl` |

### B4. Ohio - `research/midwest/evidence/gaps/45/` (report [45]) and `tools/ohio-independent/raw/` (report [20])

| Path (corpus-relative) | Bytes | What it is / what it backs |
|---|---:|---|
| `evidence/gaps/45/bluefox-sample.txt` | 43 | Bluefox paywall-boundary sample |
| `evidence/gaps/45/myohsaa-100-ad.body` | 9811 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-1000018-ad.body` | 11158 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-1000027-ad.body` | 10514 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-102-ad.body` | 9833 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-1026-ad.body` | 11214 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-1036-ad.body` | 10508 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-105-ad.body` | 9848 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-106-ad.body` | 11788 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-112-ad.body` | 11201 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-1752-ad.body` | 9817 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-828-ad.body` | 9828 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-892-ad.body` | 11497 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-9483-ad.body` | 10526 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-977-ad.body` | 9811 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-9823-ad.body` | 10200 | myOHSAA AthleticDirector payload (stress sample) |
| `evidence/gaps/45/myohsaa-sample-summary.json` | 7577 | myOHSAA 3-request path summary (SearchSchool, SportsInformation, AthleticDirector) behind the section 5 coach lane |
| `evidence/gaps/45/oatccc-Coaches_root.body` | 118 | OATCCC Coaches page body |
| `evidence/gaps/45/oatccc-doc-1KrDtc5eRxffu7FONjVr3esEA3FTRe_Tz_lcWn6bcouo.txt` | 2999 | OATCCC published contact doc (district reps) |
| `evidence/gaps/45/oatccc-doc-1Si1qAoTeQSVBpQmouvP-r_8OjnOO3BvO4Iv88m4oIGo.txt` | 2842 | OATCCC published contact doc (undifferentiated contacts) |
| `evidence/gaps/45/oatccc-robots.body` | 146 | OATCCC robots.txt body |
| `evidence/gaps/45/oatccc-robots.txt.body` | 146 | OATCCC robots.txt body (variant capture) |
| `evidence/gaps/45/oatccc-sitemap-urls.txt` | 44596 | OATCCC sitemap URL list |
| `evidence/gaps/45/otm-sample-result.body` | 10403 | timer sample result (Hy-Tek/PDF lane) |
| `evidence/gaps/45/otm-timerhub.body` | 5323 | Ohio timer hub page |
| `evidence/gaps/45/ohsaa-enrollment.html` | 228567 | **the OH school-universe payload**: `GET https://www.ohsaa.org/school-resources/school-enrollment`, HTTP 200 / 228,567 B (logged in [45] appendix and [19] line 71). One `<table>`; 816 `<tr>` = 1 header + **815 data rows**, 815 distinct `OhsaaSchoolId` (range 100-1000027); class cell 267 `AAA` + 267 `AA` + 267 `A` + 14 blank. This is the denominator re-derivation, not the [19] quote |
| `evidence/gaps/45/**` (remaining 50 of 76 files, 12781361 B) | - | further myOHSAA/OATCCC/timer probes (the dir's 76 files total 13241675 B) |
| `tools/ohio-independent/raw/ohsaa-state-tf-2026.pdf` | 816147 | **OH 2026 state-final PDF** - the `OHSAA_State_2026_Final_Results.pdf` this report cites; the report quotes 816,147 B and this file measures 816,147 B |
| `tools/ohio-independent/raw/**` (all 98 files, 9960957 B) | - | OH independent-source capture dir: Baum's Page, Finish Timing, Buckeye/CantStop/FTR timers, roster JSONs (report [20]) |

### B5. Indiana - `tools/scratch-28/` (reports [17], [18], [28])

**Read this first - a name collision bites here.** The corpus dir `tools/scratch-11/` is the **Iowa**
lane's (report [11] = `11-iowa-ihsaa-ighsau.md`) and Iowa's boys association is *also* called IHSAA
(`iahsaa.org`). Its `ihsaa-*` files are Iowa's, not Indiana's, and are filed under **B5-IA** below.
No Indiana fact in SOURCE_REPORT section 3 is drawn from them: that section cites [17]/[18]/[28] only,
and the "391 declared slots across 25 sectionals" figure is [17]'s own text.

| Path (corpus-relative) | Bytes | What it is / what it backs |
|---|---:|---|
| `tools/scratch-28/in-listdata-jr-f.html` | 1166719 | Indiana DAT listdata (junior girls): 4,453 `Indiana` tokens, 0 `Iowa`; Warren Central 46 · Noblesville 30 · Carmel 27 · Fishers 24 - the [18] performance-list / Co2027 evidence |
| `tools/scratch-28/in-hsr-2026i.html` | 1880743 | Indiana HSR indoor 2026: 7,806 `Indiana` tokens; Carmel 94 · Noblesville 86 · Fishers 69 · Warren Central 64 - the HSR 3,978 + 2,555 athlete-list evidence |
| `tools/scratch-28/ss-state-Indiana.html` | 87728 | DAT state page for Indiana (314 `Indiana` tokens) |
| `tools/scratch-28/tfrrs-sites.html` | 46660 | TFRRS site index (3 `Indiana` tokens) - the `indiana.tfrrs.org` pointer |
| `tools/scratch-28/tfrrs-search-IN-xc-202609.html` | 48882 | TFRRS search response for the IN XC 2026-09 query. **State identity is NOT in the bytes** (0 `Indiana`, 0 `Iowa`, plain TFRRS shell) - filed as IN on filename + [18] provenance only; never quote an IN fact from this file alone |
| `tools/scratch-28/` DAT team 1428/1429 family (15 files, 12861466 B): `cap-1428-JR-5000.html` 1414313, `cap-1428-JR-1000.html` 1414313, `dat-list-1428_5490-JR.html` 1337553, `dat-list-1428_5490.html` 1210584, `trap-1429_5490.html.html` 1210584, `trap-9999_5490.html.html` 1210584, `dat-list-1429_5491-JR.html` 877774, `g-1429_5491-none.html` 885685, `g-1428_5490-m.html` 804948, `g-1428_5490-f.html` 622649, `g-1429_5491-m.html` 479319, `g-1429_5491-f.html` 419650, `fuseDriver_Upcoming.js` 354456, `tfrrs-athlete-8429704.html` 312710, `upcoming_meets.json` 306344 | - | the DAT/TFRRS Indiana capture family behind [28]'s "1,816 team records in 2 requests" and the listdata route (`tools/scratch-28/` holds 91 files / 21750011 B in total; the rest are vendor bundles, fonts and other states' pages) |

### B5-IA. [11] Iowa lane (NOT Indiana) - `tools/scratch-11/`, `tools/iowa-11/`

Every row below is **IOWA**; the state is in the bytes (`iahsaa` links, literal "Iowa" occurrences,
Iowa schools). Listed because a name-based search for "ihsaa" lands here.

| Path (corpus-relative) | Bytes | What it is (all IA) |
|---|---:|---|
| `tools/scratch-11/ihsaa-members.csv` | 26522 | Iowa IHSAA member table, **379 rows** (`School, Nickname, School Colors, Conference`; conferences River Valley / SEISC / Mississippi Valley; 95 `Iowa` tokens). **Not** the Indiana 413, which is [17]'s 408 full + 5 provisional from the school-directory fetch |
| `tools/scratch-11/p-ihsaa-track.html` | 244246 | Iowa IHSAA boys T&F page (435 `iahsaa` links, 0 `Indiana`) |
| `tools/scratch-11/p-ihsaa-xc.html` | 245203 | Iowa IHSAA boys XC page |
| `tools/scratch-11/p-ihsaa-track-class.html` | 236876 | Iowa track classification page |
| `tools/scratch-11/ihsaa-track-class-2026.csv` | 8554 | Iowa classes/beds (Valley 2236, Johnston 1796, Southeast Polk 1757, ... Waukee Northwest 1599) |
| `tools/scratch-11/bound-ihsaa-boystrack-2026-04-24.html` | 115581 | Iowa boys T&F on the Bound platform (page title `Bound - High school sports - Boys Track & Field 2025-26`; 32 `Iowa`, 0 `Indiana`) |
| `tools/scratch-11/bound-ihsaa-boystrack-2026-05-21.html` | 127973 | Iowa boys T&F on Bound (46 `Iowa`) |
| `tools/scratch-11/bound-ihsaa-boystrack-2026-05-14.html` | 200516 | Iowa boys T&F on Bound (49 `Iowa`) |
| `tools/scratch-11/bound-ihsaa-boystrack-teams-2025-26.csv` | 66619 | Iowa Bound team ids/names (CAL Cadets, Cedar Rapids Prep Bison, ... Waukee, Dowling) |
| `tools/scratch-11/bound-ihsaa-boyscrosscountry-2026-09-17.html` | 107491 | Iowa boys XC on Bound (28 `Iowa`; ACGC, River Valley). **Does not carry Indiana's 391-slot / 25-sectional figure** - that is [17] text |
| `tools/scratch-11/ighsau-beds-alpha-2026-27.pdf` | 69599 | Iowa GIRLS union (IGHSAU) classification PDF |
| `tools/scratch-11/ighsau-tf-final-classes.pdf` | 283865 | IGHSAU T&F final classes |
| `tools/scratch-11/ighsau-xc-final-classes.pdf` | 182054 | IGHSAU XC final classes |
| the rest of the Iowa lane's captures in `tools/scratch-11/`: `b-iahsaa.org_member-schools_.html` 268218, `b-ia-schools.html` 638229, `b-school-harlan_directory.html` 86258, `b-ames-directory.html` 81902, `p-ihsaa-school-ames.html` 205795, `bound-ia-schools.csv` 17786, `ihsaa-members-without-bound-track.csv` 3300, `bound-track-vs-ihsaa-join.csv` 16165, `ihsaa-members-missing-bound-track.csv` 1550, `ihsaa-members-no-bound-track-2025-26.txt` 2133, `ihsaa-no-bound-boystrack-2025-26.txt` 960, `ighsau-members-beds-2026-27.csv` 9773 | - | owner report [11]. `tools/iowa-11/` (11 files, 291140 B) is a **partly overlapping** copy: `ihsaa-members.csv`, `bound-ia-schools.csv`, `bound-ihsaa-boystrack-teams-2025-26.csv`, `ihsaa-track-class-2026.csv`, `ighsau-members-beds-2026-27.csv` plus 6 files not in scratch-11 |

### B6. Cross-cutting (robots discipline, MileSplit sitemap, entries/timing probes)

| Path (corpus-relative) | Bytes | What it is / what it backs |
|---|---:|---|
| `evidence/gaps/33/robots/il.txt` | 173 | MileSplit `il` robots.txt (pre-fetch compliance check; the 173 B shape cited in section 7) |
| `evidence/gaps/33/robots/il.headers` | 203 | `il` robots.txt response headers |
| `evidence/gaps/33/robots/in.txt` | 173 | MileSplit `in` robots.txt (pre-fetch compliance check; the 173 B shape cited in section 7) |
| `evidence/gaps/33/robots/in.headers` | 203 | `in` robots.txt response headers |
| `evidence/gaps/33/robots/mi.txt` | 173 | MileSplit `mi` robots.txt (pre-fetch compliance check; the 173 B shape cited in section 7) |
| `evidence/gaps/33/robots/mi.headers` | 203 | `mi` robots.txt response headers |
| `evidence/gaps/33/robots/oh.txt` | 173 | MileSplit `oh` robots.txt (pre-fetch compliance check; the 173 B shape cited in section 7) |
| `evidence/gaps/33/robots/oh.headers` | 203 | `oh` robots.txt response headers |
| `evidence/gaps/33/robots/wi.txt` | 173 | MileSplit `wi` robots.txt (pre-fetch compliance check; the 173 B shape cited in section 7) |
| `evidence/gaps/33/robots/wi.headers` | 203 | `wi` robots.txt response headers |
| `evidence/gaps/33/sitemap-window-summary-2026-09-20.json` | 3706 | MileSplit sitemap recency-window summary ([33]) |
| `evidence/gaps/34/evidence-appendix.md` | 11692 | discovery-vs-validation appendix incl. join metrics ([34]) |
| `evidence/gaps/34/fetch.log` | 4321 | [34] fetch log |
| `evidence/gaps/32/fetch-log.tsv` | 13887 | paywall-boundary fetch log ([32]) |
| `evidence/gaps/32/parsed-summary.json` | 16985 | paywall-boundary parsed summary |
| `evidence/gaps/32/c09-il-results-index.body` | 142153 | IL results-index body from the paywall-boundary probe |
| `evidence/gaps/31/probe31-phase1.log` | 3472 | MileSplit entries/timing probe log ([31]) |
| `evidence/gap-closure/milesplit-sitemap-2026-09-20.xml` | 930488 | MileSplit sitemap capture |
| `evidence/gap-closure/milesplit-sitemap-summary.json` | 488 | its summary |
| `evidence/atn/navinfo.json` | 78469 | Athletic.net nav info (ATN lane; the 168416 national divList ancestry) |

## C. Cited by the report but **not retained** in the searched corpus

These are the fetch-log names the Midwest reports use. A corpus-wide `os.walk` (command below) found
**no file bearing them**, so this report does not pretend to hold their bytes. Where a related
artifact *is* retained it is named - without asserting the two are the same fetch unless bytes or
URLs prove it.

| Cited name | Owning report | Local status | Where the fetch evidence lives |
|---|---|---|---|
| `2025lpgd1final.pdf` (LP D1 XC finals 2025) | [15] | **bytes not retained**; the fetch itself is documented: `GET https://www.mhsaa.com/sites/default/files/Cross%20Country-Boys/2025/Finals/2025lpgd1final.pdf`, HTTP **200**, 187 finishers / 47 grade-11, `2026-09-19T23:17:55-05:00`. `gaps/39` retains *different* MHSAA finals captures (`..._2024_Finals_UP-D2-Girls.pdf` 581,244 B, `..._2025_Finals_Up-D2-Girls.pdf` 437,674 B) - identity not established | [15] appendix (URL/status/ts quoted) |
| `2026-UP-Boys-D1-Finals.pdf` | [15] | **bytes not retained**; fetch documented: `GET https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Boys-D1-Finals.pdf?time=1780175792370`, HTTP **200**, 103 graded rows / 30 grade-11, `2026-09-19T23:17:54-05:00`; the `?time=` value decodes to the 2026-05-30T21:16:32Z upload timestamp | [15] appendix |
| `26-27-Enrollment-List.pdf` | [15] | **bytes not retained**; fetch documented: `GET https://www.mhsaa.com/sites/default/files/Enrollment%20and%20Classification/26-27-Enrollment-List.pdf`, HTTP **200**, "755 member HS: MHSAA id, name, enrollment total, classification enrollment", `2026-09-19T23:14:48-05:00` - this is the provenance for the section 0.3 / section 4 **755**-school denominator | [15] appendix |
| `hscoop.pdf` | [15] | **bytes not retained**; fetch documented: `GET https://www.mhsaa.com/sites/default/files/Enrollment%20and%20Classification/hscoop.pdf`, HTTP **200**, approved co-op programs with ids in a separate space (`(1804) Ada Forest Hills Eastern(735)` vs school id 4506), 33 XC + 33 TF varsity entries, `2026-09-19T23:14:50-05:00` | [15] appendix |
| `tr2026homesteadsectional.pdf` | [35] | **PDF not retained; its text extract is**: `evidence/gaps/35/pdf-extract/tr2026homesteadsectional.txt`, 59,771 B, 69 grade-11 rows per `summary-v2.json` | `gaps/35/pdf-extract/summary-v2.json` |
| `live.pttiming.com/xc-ptt.html?mid=5127` | [06] | **not retained**; PrimeTime lane evidence is report-level ([08], [31]) | [06] appendix |
| MHSAA `sitemap.xml?page=1..7` | [15] | **not retained** (MHSAA sitemap). NB the *MileSplit* sitemap **is** retained: `evidence/gaps/35/raw/sitemap*.xml`, `evidence/gap-closure/milesplit-sitemap-2026-09-20.xml` | [15] appendix |
| `staff2_coach_rich.json`, `staff2_office_only.json`, `v1_schools.json` | [13], [38] | **retained in-repo** as fixtures - see section A, not missing | repo fixtures + [38] `appendix-table.md` |

## Verification commands (run 2026-09-21 by this lane)

```bash
# byte sizes + inventories (python, no repo writes, no fetches):
python3 -c "import os; [print(os.path.getsize(os.path.join(dp,f)), os.path.join(dp,f))
  for dp,dn,fn in os.walk(ROOT) for f in fn]"          # ROOT = each directory named above

# the name search proving section C non-retention (1 hit, and it is the .txt extract not the .pdf):
python3 -c "import os; [print(os.path.join(dp,f)) for dp,dn,fn in os.walk(CORPUS)
  for f in fn if 'homestead' in f.lower()]"
  -> research/midwest/evidence/gaps/35/pdf-extract/tr2026homesteadsectional.txt

# WI sample yield re-derivation (section 6.1):
#   python3 -c "import json; s=json.load(open(SUMM2)); print(len(s), sum(int(e['grade11_rows'] or 0) for e in s))"
#   -> 26 2653

# IN member-table row count (the 379 vs 413 distinction):
#   python3 -c "import csv; print(len(list(csv.DictReader(open(CSV)))))"   -> 379

# OH state-final byte match (section 5 quotes 816,147 B):
#   python3 -c "import os; print(os.path.getsize(PT))"                    -> 816147
```

Byte totals here are point-in-time measurements of files this lane did not create; the Midwest wave
owns them and may have changed them since 2026-09-21.
