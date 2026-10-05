# HELD classification — 278-source TAP registry vs. in-repo evidence

Evidence-only classification of `TAP-REGISTRY.json` (278 entries) against artifacts already present in the repo and sibling trees. **No network fetches were performed** at any point; the sweep was read-only. Category rules: **HELD** = a real file/capture exists whose name/content matches the source (path + size cited); **ADAPTER** = source implemented by an existing adapter with fixtures (SOURCE_REPORT + fixture dir cited); **DERIVED** = only derived/aggregate traces exist (e.g. `census-prototype/out/coaches.jsonl` rows) without the exact source body; **ABSENT** = nothing found after sweeping the locations below.

**DERIVED can overlap HELD**: a state's DERIVED rows mean derived coach rows exist in `census-prototype/out/coaches.jsonl` even where the exact source body is not held.

## Method / locations swept

Registry read in full (`TAP-REGISTRY.json`, 268.8 KB). Held locations checked: repo `var/` (incl. `var/http/store` meta+body captures, `var/tap-*` run dirs, `var/school-address-join-20261004/`), `/home/lewis/src/ad-law-scrape/data/` (NCES), `research/sources/*/SOURCE_REPORT.md` + `samples/`, bundle `probes/` + `extract/`, `crates/**/tests/fixtures/`, `samples/`, `docs/OPERATIONS.md`, `docs/VERIFICATION-EVIDENCE.md`, and sibling trees `{arh-*, census-prototype, world-athletics, reports, hs-address-pipeline, ...}`.

**Machine-readable form:** `HELD-CLASSIFY.json` (frozen final, 134,756 bytes, 278 rows, all unique). Raw backup of the generated content: `HELD-CLASSIFY.recovered.json`. Validation report: `audit/held-classify-index.txt`.

**Path integrity:** every evidence path in the final JSON was stat-verified to exist; after correcting stale/mis-cited paths (18 main-pass replacements applied with exact occurrence counts + 3 late dangling-path fixes for SRC-114, SRC-116, SRC-248), the final missing-path sweep = **0**.

Paths below are relative to `/home/lewis/src/ad-law-scrape/` unless stated otherwise. Sizes are from the evidence notes; rows marked `*` were sized from a fresh directory listing this session.

## Class counts (278 total)

| Class | Count |
|---|---|
| HELD | 73 |
| HELD+ADAPTER | 30 |
| DERIVED | 158 |
| ADAPTER+DERIVED | 12 |
| ADAPTER | 1 |
| ABSENT | 4 |

## Per-state / per-class counts

| State | HELD | HELD+ADAPTER | DERIVED | ADAPTER+DERIVED | ADAPTER | ABSENT | Total |
|---|---|---|---|---|---|---|---|
| AZ | 1 | 1 | 8 | 0 | 0 | 0 | 10 |
| CA | 2 | 9 | 5 | 1 | 0 | 0 | 17 |
| CT | 0 | 2 | 6 | 0 | 0 | 0 | 8 |
| FL | 1 | 2 | 12 | 0 | 0 | 0 | 15 |
| IA | 4 | 1 | 2 | 1 | 0 | 0 | 8 |
| IL | 3 | 2 | 3 | 0 | 0 | 0 | 8 |
| IN | 2 | 0 | 5 | 0 | 0 | 0 | 7 |
| KS | 5 | 0 | 2 | 1 | 0 | 0 | 8 |
| LA | 6 | 0 | 4 | 0 | 0 | 0 | 10 |
| MA | 2 | 2 | 6 | 0 | 0 | 0 | 10 |
| MA,CT,VT,NY | 0 | 0 | 1 | 0 | 0 | 0 | 1 |
| MI | 3 | 0 | 4 | 0 | 0 | 0 | 7 |
| MO | 1 | 1 | 5 | 1 | 0 | 0 | 8 |
| NATIONAL | 18 | 0 | 9 | 0 | 1 | 4 | 32 |
| NJ | 2 | 2 | 7 | 1 | 0 | 0 | 12 |
| NV | 3 | 0 | 5 | 0 | 0 | 0 | 8 |
| NY | 1 | 1 | 9 | 0 | 0 | 0 | 11 |
| OH | 1 | 0 | 5 | 3 | 0 | 0 | 9 |
| OK | 2 | 0 | 6 | 1 | 0 | 0 | 9 |
| OR | 0 | 2 | 7 | 0 | 0 | 0 | 9 |
| PA | 5 | 1 | 4 | 0 | 0 | 0 | 10 |
| SD | 5 | 0 | 2 | 0 | 0 | 0 | 7 |
| TX | 0 | 2 | 13 | 1 | 0 | 0 | 16 |
| UT | 3 | 0 | 8 | 0 | 0 | 0 | 11 |
| VA | 3 | 0 | 7 | 1 | 0 | 0 | 11 |
| VT | 0 | 2 | 4 | 0 | 0 | 0 | 6 |
| WA | 0 | 0 | 9 | 1 | 0 | 0 | 10 |
| **Total** | **73** | **30** | **158** | **12** | **1** | **4** | **278** |

## ABSENT (4) — no trace found

All four are NATIONAL sources; the sweep record cited is `census-prototype/out/coverage.md` (54.7 KB directory listing) plus filename sweeps that returned nothing.

- **SRC-262** NIAAA state athletic-administrator association directory — no `niaaa.org` host entry or capture found; checked `research/sources/**` (all trees/probes/extract), `census-prototype/{out,raw,store}`, repo `var/`, fixtures, docs, `data/`, sibling trees; filename sweep `*niaaa*` returned nothing.
- **SRC-263** NASO state resource guide — no `naso.org` capture found; same locations; sweep `*naso.org*`/`*naso-*` returned nothing.
- **SRC-264** NHSACA national coaches association — no `nhsaca.org` capture found; same locations; sweep `*nhsaca*` returned nothing.
- **SRC-276** SchoolDigger documented school API — no `schooldigger` capture or adapter found; same locations; sweep `*schooldigger*` returned nothing.

The single ADAPTER row is **SRC-277** GreatSchools NearbySchools documented API (`hs-address-pipeline/greatschools_enrichment.py` + `greatschools_crawler.py`; no capture fixtures found).

## Full HELD list (73 HELD + 30 HELD+ADAPTER = 103 rows)

- **SRC-001** `TX` HELD+ADAPTER — UIL sport alignments and school-code inventory — `athletic-rust-pipeline/research/sources/state-assoc-southcentral/samples/tx-alignments.html` — 65.8 KB
- **SRC-013** `TX` HELD+ADAPTER — MileSplit Texas team directory — `athletic-rust-pipeline/research/sources/milesplit-national/samples/teams-tx.html` — 707.4 KB `*`
- **SRC-017** `CA` HELD+ADAPTER — CIF / Home Campus public school and coaches directory — `research/sources/state-assoc-westcoast/samples/cifss-directory.html` — 211.6 KB
- **SRC-019** `CA` HELD+ADAPTER — MileSplit California team directory — `research/sources/milesplit-national/samples/teams-ca.html` — 606.6 KB `*`
- **SRC-021** `CA` HELD — CIF San Francisco high-school members — `research/sources/state-assoc-westcoast/samples/cifsf-high-schools.html` — 252.1 KB
- **SRC-022** `CA` HELD+ADAPTER — CIF Southern Section directory wrapper — `research/sources/state-assoc-westcoast/samples/cifss-directory.html` — 211.6 KB
- **SRC-023** `CA` HELD+ADAPTER — CIF Northern Section public Home Campus directory — `research/sources/state-assoc-westcoast/samples/cifns-directory.html` — 242.8 KB
- **SRC-024** `CA` HELD+ADAPTER — CIF San Diego Section public Home Campus directory — `research/sources/state-assoc-westcoast/samples/cifsds-school-directory.html` — 180.1 KB
- **SRC-025** `CA` HELD+ADAPTER — CIF North Coast Section official source — `research/sources/state-assoc-westcoast/samples/cifncs-home.html` — 234.8 KB
- **SRC-026** `CA` HELD — CIF Central Coast Section official source — `research/sources/state-assoc-westcoast/samples/cifccs-home.html` — 89.7 KB
- **SRC-027** `CA` HELD+ADAPTER — CIF Central Section official source — `research/sources/state-assoc-westcoast/samples/cifcs-home.html` — 337.5 KB
- **SRC-029** `CA` HELD+ADAPTER — CIF Los Angeles City Section — `research/sources/state-assoc-westcoast/samples/cif-la-home.html` — 86.9 KB
- **SRC-030** `CA` HELD+ADAPTER — CIF Sac-Joaquin member schools — `research/sources/state-assoc-westcoast/samples/cifsjs-member-schools.html` — 47.2 KB
- **SRC-034** `FL` HELD+ADAPTER — FHSAA public Home Campus school and coach directory — `research/sources/state-assoc-southeast/samples/homecampus-school-directory.html` — 225.0 KB
- **SRC-035** `FL` HELD+ADAPTER — MileSplit Florida / flrunners team directory — `research/sources/milesplit-national/samples/teams-fl.html` — 325.0 KB `*`
- **SRC-036** `FL` HELD — Florida DOE private-school contact export — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/SRC-036/rows.csv` — 552.0 KB
- **SRC-054** `NY` HELD — PSAL Cross Country school/team listing — `research/sources/state-assoc-midatlantic/samples/psal-xc.html` — 315.4 KB
- **SRC-058** `NJ` HELD+ADAPTER — NJSIAA Member Information — `research/sources/state-assoc-midatlantic/samples/njsiaa-member-info.html` — 232.5 KB
- **SRC-061** `NJ` HELD — New Jersey XC/TF Coaches Association resources — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/njxctfca/FINDINGS.md` — 9.0 KB
- **SRC-066** `NJ` HELD — Greater Middlesex Conference Track Coaches Association member schools — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/SRC-066/REPORT.md` — 7.2 KB
- **SRC-068** `MA` HELD — Massachusetts State Track Coaches Association current members — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/SRC-068/REPORT.md` — 15.5 KB
- **SRC-069** `MA` HELD+ADAPTER — MIAA Schools Directory and school profiles — `research/sources/state-assoc-midatlantic/samples/miaa-member-schools.html` — 67.3 KB
- **SRC-075** `MA` HELD — MIAA Track & Cross Country — `research/sources/state-assoc-midatlantic/samples/miaa-track-xc.html` — 109.5 KB
- **SRC-076** `CT` HELD+ADAPTER — CIAC high school coach directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/ciac-fpsports/directory.html` — 657.3 KB
- **SRC-082** `VT` HELD+ADAPTER — Vermont Principals Association athletics — `research/sources/state-assoc-midatlantic/samples/vpa-divisional-alignments.html` — 120.6 KB
- **SRC-086** `NY` HELD+ADAPTER — MileSplit NY team directory — `research/sources/state-assoc-midatlantic/samples/ny-milesplit-teams.html` — 409.9 KB
- **SRC-087** `NJ` HELD+ADAPTER — MileSplit NJ team directory — `research/sources/state-assoc-midatlantic/samples/nj-milesplit-teams.html` — 185.7 KB
- **SRC-088** `MA` HELD+ADAPTER — MileSplit MA team directory — `research/sources/state-assoc-midatlantic/samples/ma-milesplit-teams.html` — 167.8 KB
- **SRC-089** `CT` HELD+ADAPTER — MileSplit CT team directory — `research/sources/state-assoc-midatlantic/samples/ct-milesplit-teams.html` — 102.4 KB
- **SRC-090** `VT` HELD+ADAPTER — MileSplit VT team directory — `research/sources/state-assoc-midatlantic/samples/vt-milesplit-teams.html` — 61.8 KB
- **SRC-100** `OH` HELD — OATCCC current coach membership — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/SRC-100/REPORT.md` — 13.6 KB
- **SRC-106** `MI` HELD — MHSAA public school directory and coach tabs — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/sources/SRC-106/FINDINGS.md` — 6.8 KB
- **SRC-107** `MI` HELD — Michigan EEM public school data export — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/107-mi-eem/rows.csv` — 469.9 KB
- **SRC-108** `MI` HELD — MITCA track/XC coaches association — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/108-mitca/REPORT.md` — 2.2 KB
- **SRC-113** `IN` HELD — Indiana DOE downloadable school directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/sources/SRC-113/FINDINGS.md` — 4.7 KB
- **SRC-114** `IN` HELD — Indiana IHSAA member school directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/sources/SRC-114/FINDINGS.md` — 4.0 KB
- **SRC-120** `IL` HELD+ADAPTER — IHSA new public school directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/il-ihsa/manifest.json` — 5.6 KB
- **SRC-121** `IL` HELD+ADAPTER — IHSA head-coach season-summary database — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/il_ihsa_records/FINDINGS.md` — 12.3 KB
- **SRC-122** `IL` HELD — ISBE nightly public/private school directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/122-il-isbe/candidate-25.body` — 71.0 KB
- **SRC-124** `IL` HELD — IHSA public mobile school search — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/il-ihsa-mobile/FINDINGS.md` — 2.3 KB
- **SRC-125** `IL` HELD — IHSA conference directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/il-ihsa-conference/FINDINGS.md` — 1.7 KB
- **SRC-128** `IA` HELD+ADAPTER — Iowa IHSAA member school list and detail pages — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/sources/SRC-128/FINDINGS.md` — 6.3 KB
- **SRC-130** `IA` HELD — Iowa DOE public and nonpublic building directories — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/130-ia-doe/REPORT.md` — 3.9 KB
- **SRC-131** `IA` HELD — Iowa school buildings ArcGIS REST service — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/131-ia-arcgis/query-0-0.geojson` — 600.7 KB
- **SRC-132** `IA` HELD — Iowa Association of Track Coaches — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/iatrackcoaches/FINDINGS.md` — 7.8 KB
- **SRC-133** `IA` HELD — IATC track-and-field advisory board — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/sources/SRC-133/board.json` — 1.2 KB
- **SRC-146** `OR` HELD+ADAPTER — OSAA member-school profiles and coach directory — `research/sources/state-assoc-westcoast/samples/osaa-schools.html` — 31.1 KB
- **SRC-149** `OR` HELD+ADAPTER — OSAA statewide compact coaches directory — `research/sources/state-assoc-westcoast/samples/osaa-coaches.html` — 101.0 KB
- **SRC-158** `AZ` HELD+ADAPTER — AIA school-search JSON endpoint — `research/sources/state-assoc-mountain/samples/aia-search-limit500.json` — 7.5 KB
- **SRC-159** `AZ` HELD — AIA sport alignments — `research/sources/state-assoc-mountain/samples/aia-alignments.html` — 277.8 KB
- **SRC-164** `UT` HELD — UHSAA school profiles and coaches — `research/sources/state-assoc-mountain/samples/uhsaa-school-directory.html` — 472.3 KB
- **SRC-169** `UT` HELD — UHSAA schools by district and private/charter category — `research/sources/state-assoc-mountain/samples/uhsaa-realignment-2025-27.pdf` — 521.2 KB
- **SRC-171** `UT` HELD — UHSAA school participation matrix — `research/sources/state-assoc-mountain/samples/uhsaa-participation-numbers.pdf` — 304.8 KB
- **SRC-175** `NV` HELD — Southern Nevada Track & Cross Country Coaches Association directories — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/SRC-175/REPORT.md` — 11.4 KB
- **SRC-178** `NV` HELD — Southern Nevada Track coaches CSV — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/SRC-178/REPORT.md` — 9.8 KB
- **SRC-179** `NV` HELD — Southern Nevada XC coaches CSV — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/SRC-179/rows.csv` — 25.9 KB
- **SRC-184** `VA` HELD — Virginia Department of Education public-school alphabetical directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/184-va-vdoe-directory/rows.csv` — 304.2 KB
- **SRC-185** `VA` HELD — VISAA member-school directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/185-va-visaa/rows.csv` — 8.3 KB
- **SRC-186** `VA` HELD — VHSL alignment/classification hub — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/186-va-vhsl/qualified.json` — 401 B
- **SRC-195** `MO` HELD+ADAPTER — MSHSAA member-school listing and coaching-roster route — `research/sources/state-assoc-plains/samples/mshsaa-school-listing-2026-09-22.html` — 704.5 KB
- **SRC-198** `MO` HELD — MSHSAA boys cross-country coaches sample — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/sources/SRC-198/FINDINGS.md` — 5.1 KB
- **SRC-203** `OK` HELD — OSSAARankings school directory and sport schedules — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/sources/SRC-203/FINDINGS.md` — 7.3 KB
- **SRC-204** `OK` HELD — Oklahoma State School and District Directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/204-ok-directory/schools_full.json` — 1.5 MB
- **SRC-212** `LA` HELD — LHSAA 2025–2026 Coaches Directory — `research/sources/state-assoc-southcentral/samples/la-coaches-directory-2025-26.pdf` — 6.5 MB
- **SRC-213** `LA` HELD — Louisiana BESE-approved nonpublic-school list 2026–2027 — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/213-la-bese-nonpublic/rows.csv` — 69.7 KB
- **SRC-215** `LA` HELD — LHSAA registered XC coaches, September 22 2025 — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/SRC-215/REPORT.md` — 7.4 KB
- **SRC-216** `LA` HELD — LHSAA public school-directory search — `research/sources/state-assoc-southcentral/samples/la-school-directory.html` — 62.1 KB
- **SRC-217** `LA` HELD — LHSAA XC classifications and alignments hub — `research/sources/state-assoc-southcentral/samples/la-cross-country.html` — 65.2 KB
- **SRC-218** `LA` HELD — LHSAA outdoor-track hub — `research/sources/state-assoc-southcentral/samples/la-outdoor-track-field.html` — 64.1 KB
- **SRC-222** `SD` HELD — SDCCTFCA 2024–2025 membership roster — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/SRC-222/REPORT.md` — 9.9 KB
- **SRC-223** `SD` HELD — South Dakota Department of Education educational directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/extract/223-sd-edudir/rows.csv` — 92.2 KB
- **SRC-225** `SD` HELD — SDHSAA cross-country participation hub — `research/sources/state-assoc-plains/samples/sdhsaa-cross-country-2026-09-22.html` — 498.0 KB
- **SRC-227** `SD` HELD — SDHSAA athletics cooperatives — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/227-sdhsaa-cooperatives/REPORT.md` — 2.7 KB
- **SRC-228** `SD` HELD — SDCCTFCA membership/area-alignment hub — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/228-sdhsca-membership/REPORT.md` — 4.3 KB
- **SRC-229** `PA` HELD+ADAPTER — PIAA school directory — `athletic-rust-pipeline/crates/census-crawl/tests/fixtures/pa_piaa/directory_alpha_a.html` — 78.2 KB
- **SRC-230** `PA` HELD — Pennsylvania EdNA output files — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/230-pa-edna/REPORT.md` — 7.2 KB
- **SRC-231** `PA` HELD — PAISAA independent-school member directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/231-paisaa-members/REPORT.md` — 2.1 KB
- **SRC-233** `PA` HELD — EdNA private and nonpublic export form — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/230-pa-edna/pnp-export-2.body` — 732.3 KB
- **SRC-234** `PA` HELD — Track and Field Coaches Association of Greater Philadelphia handbook — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/234-tfcaofgp-handbook/REPORT.md` — 2.6 KB
- **SRC-235** `PA` HELD — Friends Schools League — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/235-fsl-athletics/REPORT.md` — 1.6 KB
- **SRC-239** `KS` HELD — Kansas Educational Directory Reports — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/239-ks-directory/captures/` — 252.5 KB
- **SRC-240** `KS` HELD — KSHSAA school leagues directory — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/ks-kshsaa/raw/directory-search-name-a.json` — 489,295 B
- **SRC-241** `KS` HELD — Kansas Cross Country and Track and Field Coaches Association — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/kcctfca/FINDINGS.md` — 11.9 KB
- **SRC-242** `KS` HELD — KSHSAA approved schools — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/ks-kshsaa/manifest.json` — 3.1 KB `*`
- **SRC-244** `KS` HELD — KCCTFCA 2026 winter clinic — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/kcctfca/FINDINGS.md` — 11.9 KB
- **SRC-247** `NATIONAL` HELD — NCES CCD downloadable files selector — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/247-nces-ccd-selector/REPORT.md` — 3.0 KB
- **SRC-248** `NATIONAL` HELD — NCES CCD 2024–25 school directory CSV/SAS ZIP — `athletic-rust-pipeline/var/school-address-join-20261004/ccd/ccd_sch_029_2526_w_0a_050626.csv` — 41,054,983 B (in-repo receipt); probe `probes/248-nces-ccd-zip/REPORT.md` 5.3 KB
- **SRC-249** `NATIONAL` HELD — NCES CCD 2024–25 school directory companion — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/249-nces-ccd-companion/raw/directory-companion.xlsx` — 53.1 KB
- **SRC-250** `NATIONAL` HELD — NCES EDGE public-school administrative REST layer — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/250-nces-edge-admin/manifest.json` — 7.9 KB
- **SRC-253** `NATIONAL` HELD — NCES PSS data and documentation index — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/253-nces-pss-index/raw/pssdata.html` — 57.3 KB
- **SRC-254** `NATIONAL` HELD — NCES PSS 2023–24 public-use CSV ZIP — `data/nces/pss/pss2324_pu.csv` — 39,521,631 B (in-repo receipt)
- **SRC-255** `NATIONAL` HELD — NCES PSS 2023–24 record layout — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/253-nces-pss-index/raw/layout2023-24.pdf` — 229.1 KB
- **SRC-257** `NATIONAL` HELD — NCES PSS 2023–24 methodological frame file — `data/nces/pss/2023-24_PSS_Frame_Data.csv` — 849,644 B (in-repo receipt)
- **SRC-261** `NATIONAL` HELD — NHSTFXCCA state track/XC associations — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/nhstfxcca/FINDINGS.md` — 15.3 KB
- **SRC-266** `NATIONAL` HELD — SIDEARM public staff-directory pattern: Miramonte — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/sidearm_miramonte/FINDINGS.md` — 13.3 KB
- **SRC-267** `NATIONAL` HELD — Mascot Media public staff-directory pattern: Richland Northeast — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/mascotmedia_rne/FINDINGS.md` — 12.5 KB
- **SRC-268** `NATIONAL` HELD — Finalsite public school athletics staff pattern: Starr’s Mill — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/finalsite_starrsmill/FINDINGS.md` — 24.2 KB
- **SRC-269** `NATIONAL` HELD — Edlio public athletics staff pattern: Del Norte — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/edlio_delnorte/FINDINGS.md` — 15.5 KB
- **SRC-270** `NATIONAL` HELD — School-owned directory pattern: Wellesley — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/schoolowned_wellesley/FINDINGS.md` — 14.3 KB
- **SRC-271** `NATIONAL` HELD — School-owned Google Sites coaching directory: Hopatcong — `athletic-rust-pipeline/research/sources/coach-coverage-bundle-20261004/probes/googlesites_hopatcong/FINDINGS.md` — 13.7 KB
- **SRC-273** `NATIONAL` HELD — DragonFly Public Directory documentation — `athletic-rust-pipeline/crates/census-crawl/tests/fixtures/coach_directories/PROVENANCE.json` — 11.5 KB
- **SRC-274** `NATIONAL` HELD — MaxPreps track & field school/team discovery — `research/sources/national-aggregators/samples/maxpreps/mp-track-field.html` — 214.5 KB `*`
- **SRC-275** `NATIONAL` HELD — MileSplit team search — `research/sources/milesplit-national/samples/teams-count-sweep.tsv` — 9.9 KB `*`

## Provenance / fix log

- Source content: generated against the registry sweep; raw generated content preserved as `HELD-CLASSIFY.recovered.json` (134,611 chars, restored deterministically from the session log), and the corrected final written as `HELD-CLASSIFY.json` (134,756 bytes).
- 18 main-pass replacements corrected stale directory names in first-path citations and note text (verified targets on disk before application; exact occurrence counts, one item covering 3 occurrences: `bound-gobound` → `probes/gobound/FINDINGS.md`).
- 3 late dangling-path corrections after the stat sweep: `extract/SRC-114/captures/` → `probes/sources/SRC-114/captures/` (covers SRC-114 and SRC-116) and `extract/SRC-248/REPORT.md` → `probes/248-nces-ccd-zip/REPORT.md`.
- Post-fix stat sweep over every evidence path in the final JSON: **missing = 0**.
- No network fetches; no repo code or existing artifacts modified by this classification.
