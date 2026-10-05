# Group A state work orders — 15 largest absolute gaps

States: TX, CA, FL, NY, IL, MI, WA, AZ, VA, NJ, CO, IN, KY, LA, NE.
Zero network fetches; zero builds/tests; all paths below were read from the working tree 2026-10-04.

Sources of numbers:
- Matrix rows: `var/state-coverage-matrix-20261004.json` (= `STATE-COVERAGE-MATRIX.md`), columns ccd / any-coach / TFXC / email-with-school.
- Prototype yields: `census-prototype/out/coverage.md` §"Per jurisdiction" (lines 5-57); merged rows in `census-prototype/out/coaches.jsonl` (11.8 MB; HELD-INVENTORY.md records 61,083 rows) and per-state `census-prototype/out/<st>.jsonl`.
- Host body counts: `census-prototype/out/coverage.md` §"Hosts captured (681 hosts)" (lines 349-1033); bodies live at `census-prototype/raw/<host>__<sha256>.body|.meta.json`.
- Corpus tier-A: `research/sources/state-assoc-{southcentral,southeast,midatlantic,westcoast,mountain,plains}/samples/` (+ `CAPTURES.md`, `.captures.tsv`), `research/sources/milesplit-national/samples/teams-*.html`.
- Registry classes per state: `research/sources/coach-coverage-bundle-20261004/HELD-CLASSIFY.json` (278 rows; CO/KY/NE have no registry rows — state absent from that file's per-state table).
- Today's tier-C: `var/tap-*`, `var/*-20261004*`, bundle `probes/<n>-*`, `extract/<n>-*`.

---

## TX

1. **Matrix row**: ccd 9774 | any-coach 190 | TF-XC 54 | email 64. Prototype yield (`coverage.md`): 3,536 school rows / 0 addr / 0 zip / 166 TF-XC / 76 head / 181 AD rows / sources ok 4 / failed 0 / 1,256 stage-two pages. `out/tx.jsonl` 676.5 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-southcentral/samples/`: `tx-alignments.html` 65.8 KB + `.curlerr`; `tx-alignment-26-28-alpha.txt` 179.5 KB (+ `-rank.txt`, PDF twins); `tx-track-field-school-codes.html`; `tx-tf-school-codes-2026.txt/.pdf`; `tx-track-field-state.html`; `tx-track-field.html`; `tx-cross-country.html`; `tx-cross-country-state.html`; `tx-xc-2025-6a-boys-results.html`; `tx-xc-historical-archive.html`; `tx-xc-state-follow.html`; `tx-alignments-xc.html`; `tx-milesplit-home.html`; `robots-tx.milesplit.com.txt` (+ tree `CAPTURES.md`).
   - B — raw host dirs: `www.uiltexas.org` 40 responses, `tea.texas.gov` 10, `www.thsca.com` 4, `athletics.pisd.edu` 4, `www.lamarsilverfoxes.com` 4, `tx.milesplit.com` 2. Shared national: `www.milesplit.com` 47,648, `www.maxpreps.com` 7,676.
   - C — `var/st-TX.json` (jurisdiction plan; `teams` completed 2,423 records 2026-10-03; sweepable athleticnet, milesplit). No TX tap/probe/extract dir exists.
3. **Fill plan**: (1) reuse prototype merged rows (190/54/64); (2) run wired adapters `tx_uil` (alignments school-code inventory) and `tx_milesplit` on held bodies; (3) fresh extraction from held `tx-tf-school-codes-2026.*` + `tx-alignment-26-28-*` to emit the UIL member roster; (4) school-site coach pages via held generic pattern probes (sidearm/finalsite/edlio). Reachable: UIL member set from the 2026-28 alignments + 2,423 team records.
4. **True missing**: TEA AskTED personnel downloads (`tealprod.tea.state.tx.us`); TAPPS (`tapps.biz`); TEPSAC (`tepsac.org`); TCAF (`tcafellowship.com`); SPC (`spcsports.org`); TCSAAL (`texascharter.org`); CCCAT (`cccat.org`); TTFCA (`ttfca.org`); TGCA (`austintgca.com`); THSCA (`thsca.com`); Clell Wade Coaches Directory (`coachesdirectory.com`, licensed); Community ISD example (`communityisd.org`).
5. **Notes**: UIL bodies are school/alignment only (no coach emails); prototype TX carries 0 addresses/zips — mail needs a CCD join. Coach rows are name/school only.

## CA

1. **Matrix row**: ccd 10407 | any 1038 | tfxc 810 | email 1001. Yield: 6,694/975/0/3,380/3,177/6,274 / 11 ok / 0 fail / 1,970 pages. `out/ca.jsonl` 2.3 MB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-westcoast/samples/`: `cifstate-census.html`, `cif-census-2025-26.xlsx`, `cifstate-xc-index/-tf-past-results/-xc-past-results/-xc-results/-home`, `cifss-directory.html` 211.6 KB, `cifss-home.html`, `cifncs-home.html` + `cifncs-widget-directory.html` 73.4 KB, `cifns-directory.html` + `cifns-home.html`, `cifsds-school-directory.html` 180.1 KB + `cifsds-info-directory.html` 170.4 KB + `cifsds-home.html`, `cifcs-home.html` 337.5 KB + `cifcs-widget-directory.html` 67.1 KB, `cifccs-home.html` 89.7 KB + `cifccs-directory.html` 30.5 KB, `cif-la-home.html` 86.9 KB + `cif-la-widget-directory.html` 72.7 KB, `cifsjs-member-schools.html` 47.2 KB + `cifsjs-home.html` 66.2 KB + `cifsjs-cross-country.html`, `cifsf-high-schools.html` 252.1 KB + `cifsf-xc-guide.html` + `cifsf-home.html`, `robots-cif*.txt`.
   - B — raw: `cifsshome.org` 507, `cifncshome.org` 161, `cifcshome.org` 151, `cifsjshome.org` 131, `cifsdshome.org` 83, `cif-lahome.org` 82, `www.cifss.org` 5, `www.cifncshome.org` 4, 3× {`www.cifccs.org`,`www.cifsds.org`,`www.cifsf.org`,`www.cifsjs.org`,`www.cifsshome.org`,`www.cifstate.org`}, 2× {`www.cif-la.org`,`www.cifcs.org`,`www.cifncs.org`,`www.cifns.org`,`ca.milesplit.com`} ≈ 1,152 bodies.
   - C — `var/home-campus-CA-20261004/`: `pass2.log` = 1,717 schools / 6,758 coach rows, all with email, 0 errors (sections 1-9,13); `run.log` 503/2,258/78 err; `resume.log` 562/2,399/19; `slowpass.log` 623/2,563/120; sibling `nj.log`/`fl.log`; `store/{http,fjall,out}`. Also `probes/home_campus/`, `probes/edlio_delnorte/` (national Edlio pattern, Del Norte CA).
3. **Fill plan**: run existing `home_campus` adapter over held CIF widget bodies → 1,717 CA schools with emails (today's artifact); wired `ca_cifstate`/`ca_cifss`/`ca_cif{ss,ncs,sjs,sds,cs,la}_directory` + `ca_milesplit` for section rosters; fresh extraction from `cifstate-census.html` + `cif-census-2025-26.xlsx`. Reachable: ~1,717 schools directly; sections cover ~10k CCD if details route unblocks.
4. **True missing**: CIF Oakland Section (`cifoakland.org`, robots-only); PrepCalTrack SCVAL (`lynbrooksports.prepcaltrack.com`); USATF Pacific (`pausatf.org`); California Coaches Association; CIF-SS per-school details endpoint (405s today); CIF-SS directory sections 2-13 (405s today).
5. **Notes**: widget detail endpoint returns 405 for most schools — only section 1 details parsed; CA is the only group-A state with broad email coverage (1,001). `cifss-directory.html` body is platform widget (school+coach rows) rather than TF/XC-specific.

## FL

1. **Matrix row**: ccd 4314 | any 377 | tfxc 302 | email 354. Yield: 3,681/335/0/1,186/1,101/1,671 / 5 ok / 0 fail / 970 pages. `out/fl.jsonl` 997.3 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-southeast/samples/`: `homecampus-school-directory.html` 225.0 KB, `homecampus-schools-get-query.json` 26.0 KB, `homecampus-schools-get.json`, `homecampus-school-details-2397.json` 2.2 KB, `membership-fhsaa.html`, `home-fhsaa.html` 148.6 KB, `sport-cross-fhsaa.html` 142.4 KB, `sport-track-fhsaa.html` 120.5 KB, `tk-regions-fhsaa.html`, `xc-results-fhsaa.html`, `robots-fhsaa.txt`, `robots-fhsaa-homecampus.txt`, `tfrrs-fl-{teams,class-4a,state-series,archive-2026,meet-index,meets,list-5587-4a-region-1}.html`, `robots-florida-tfrrs.txt`.
   - B — raw: `fhsaa.homecampus.com` 420, `fl.milesplit.com` 2, `fhsaahome.org` 1, `www.fhsaa.com` 1.
   - C — `var/home-campus-CA-20261004/fl.log` = section 10 (FL) parsed 880 schools / 3,677 coach rows, all with email, 0 errors; `var/home-campus-FL-20261004/run.log` = 0 rows (misconfigured: fetched a cifsshome URL); `var/tap-home-campus-20261004/pass-fl.log` 28.6 KB; `extract/SRC-036/rows.csv` 552.0 KB + `REPORT.md` (FL DOE private-school contact export); `probes/home_campus/`.
3. **Fill plan**: `fl_fhsaa_widget` + `fl_fhsaa_api` wired on `fhsaa.homecampus.com` held bodies → 880 FL schools / 3,677 emails today; SRC-036 extract for private-school contacts; `fl_milesplit` for team pages; prototype merged (377/302/354).
4. **True missing**: FHSAA XC/TF classification news files (article not captured); FACA (`floridacoaches.org`); SSAA (`sunshinestateathletics`); FICAA (`ficaa.org`); FCC (`fccsports.net`); FACCS (`faccs.org`); FCIS (`fcis.org`); Cypress Creek HS example (`cchs.pasco`); Winter Springs HS (`winterspringshs`).
5. **Notes**: held FHSAA home-campus body is school+coach+email (not sport-partitioned); FHSAA sport pages are TF/XC-only but coachless.

## NY

1. **Matrix row**: ccd 4865 | any 59 | tfxc 16 | email 33. Yield: 1,894/0/0/39/38/61 / 3 ok / 0 fail / 647 pages. `out/ny.jsonl` 358.3 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-midatlantic/samples/`: `nysphsaa-schools-path.html`, `nysphsaa-schools.html` 162.9 KB, `nysphsaa-itrack.html`, `nysphsaa-otrack.html`, `nysphsaa-xc.html`, `nysphsaa-home.html`, `nysphsaa-sitemap.aspx`, `sitemap-nysphsaa.xml`, `robots-nysphsaa.txt`; `psal-xc.html` 315.4 KB, `psal-outdoor-track.html` 381.6 KB, `psal-profile.html`, `psal-home.html`, `sitemap-psal.xml`, `robots-psal.txt`; `ny-milesplit-teams.html` 409.9 KB.
   - B — raw: `www.nysphsaa.org` 10, `nystca.com` 8, `data.nysed.gov` 7, `sectionxi.org` 6, 4× {`www.section1ny.org`,`www.section6ny.org`,`www.section8ny.org`}, 2× {`www.psal.org`,`www.section1.org`,`www.section2.org`,`www.section3.org`,`www.section9athletics.org`,`www.sections710.org`,`www.sectionv.org`,`ny.milesplit.com`}, `www.section7.org` 1 (≈60 total).
   - C — no NY-specific tap; `var/st-NY.json` plan file present.
3. **Fill plan**: `ny_nysphsaa` (html) + `ny_milesplit` wired; PSAL held XC/track pages for NYC programs; fresh extraction from held NYSPHSAA section pages + `nystca.com` raw bodies (8) for coach association rows; prototype merged (59/16/33).
4. **True missing**: NYSED SEDREF (`p12.nysed.gov`); Section V (`sectionv.org`); Section IV (`sectionivathletics`); Section IX (`sectionixathletics`); NYSAIS (`nysais.org`); CHSAA NY (`chsaany.org`); NEPSAC (`nepsac.org`); Section II/III/VI/X/XI pages.
5. **Notes**: NY bodies are name/URL-heavy; coach emails come mostly from derived rows. `nystca.com` raw (8 bodies) is unparsed. Prototype NY has 0 addr/0 zip.

## IL

1. **Matrix row**: ccd 4438 | any 824 | tfxc 705 | email 20. Yield: 2,730/1,643/828/2,537/2,530/1,665 / 5 ok / 0 fail / 1,339 pages. `out/il.jsonl` 902.5 KB.
2. **Held evidence**:
   - A — `research/sources/milesplit-national/samples/teams-il.html`; IHSA fixtures `crates/census-crawl/tests/fixtures/ihsa/ihsa_tournament` (per SRC-121 note).
   - B — raw: `api.ihsa.org` 817, `il.milesplit.com` 2.
   - C — `probes/il-ihsa/`: `manifest.json` 6.6 KB; `raw/schools-list.json` 440.6 KB, `raw/school-0101-staff2.json` 5.3 KB, `raw/school-0101-staff-136817-email.json` 31 B, `raw/school-directory.html` 6.9 KB; `robots/{www.ihsa.org,api.ihsa.org}.txt`. `probes/il_ihsa_records/FINDINGS.md` 12.3 KB, `probes/il-ihsa-mobile/` (FINDINGS 2.3 KB, manifest 4.7 KB), `probes/il-ihsa-conference/` (FINDINGS 1.7 KB, manifest 2.6 KB), `probes/122-il-isbe/` (candidate-01..25 bodies up to 4.9 MB, `page.body` 122.7 KB, `candidate-results.json` 17.4 KB). Runs: `var/il-drain2-20261004/` (loop-run3.log 9.7 KB; attempts 1-47; attempt-47.log: 318 done, 508 left open, `api.ihsa.org rate_limited` + cooldown, exit=127 storms), `var/il-drain-20261004/`, `var/il-run-20261004/`, `var/il-run2-20261004/` (`provider.log` 88.7 KB, `il-names.txt` 26 B).
3. **Fill plan**: wired `il_ihsa_schools` + `il_ihsa_staff` against held `schools-list.json` (full ~825-school member list) — drain reached 318 schools before rate limiting; `il_milesplit` for team pages; ISBE candidate export (122-il-isbe) for non-IHSA schools; prototype merged (824/705/20).
4. **True missing**: ITCCCA (`itccca.com`); Chicago Catholic League; ISBE nonpublic registration (`isbe.net`); IHSA conference route (`conf.htm` probe); school staff pages.
5. **Notes**: TF-XC school coverage (705) far exceeds email (20) — email reveals hit 429/cooldown; `staff/.../email` endpoint is the bottleneck.

## MI

1. **Matrix row**: ccd 3508 | any 179 | tfxc 146 | email 18. Yield: 2,255/0/0/214/192/49 / 5 ok / 0 fail / 582 pages. `out/mi.jsonl` 363.7 KB.
2. **Held evidence**:
   - A — none in the state-assoc corpus trees; held only via bundle artifacts (below).
   - B — raw: `mitca.org` 30, `www.mhsaa.com` 10, `mhsaa.com` 9, `www.mitca.org` 2, `mi.milesplit.com` 2, `my.mhsaa.com` 1.
   - C — `extract/107-mi-eem/rows.csv` 469.9 KB (EEM public-school export); `var/tap-107-mi-eem/` (`report-viewer-response.bin` 2.6 MB, `dataset-response.bin` 5.7 KB, `post-get*.html` 30.3 KB, `store/`); `probes/sources/SRC-106/FINDINGS.md` 6.8 KB + `extract/SRC-106/REPORT.md` 4.2 KB; `probes/108-mitca/REPORT.md` 2.2 KB + `extract/108-mitca/rows.csv` 4.6 KB.
3. **Fill plan**: EEM CSV → school roster (address fields unverified); MHSAA school pages (SRC-106 capture) for directory/AD; MITCA rows for coach names (4.6 KB, small); `mi_mhsaa_schools` + `mi_mhsaa_enrollment` wired; prototype merged (179/146/18).
4. **True missing**: `my.mhsaa.com` leagues search; MHSCA (`mhsca.org`); CHSL (`chsl.com`); MI approved nonpublic list (`michigan.gov`); school athletics pages.
5. **Notes**: no tier-A corpus for MI; all held bodies are bundle extracts/probes. MHSAA bodies are coaching-tab HTML; MITCA is name-only.

## WA

1. **Matrix row**: ccd 2574 | any 376 | tfxc 365 | email 10. Yield: 2,076/1,463/745/2,257/2,255/16 / 53 ok / 0 fail / 1,059 pages. `out/wa.jsonl` 566.6 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-westcoast/samples/`: `wiaa-finalforms-p2.html` 141.8 KB, `wiaa-finalforms-p51.html`, `wiaa-finalforms-state-schools.html` 131.8 KB, `mywiaa-home.html` 559.8 KB, `mywiaa-ad-center.html`, `wiaa-events.html`, `wiaa-home.html`, `wiaa-state-xc-results.html`, `wiaa-xc-state-2026.html`, `robots-wiaa*.txt`.
   - B — raw: `wiaa.finalforms.com` 1,509 (50-page sweep), `www.mywiaa.wiaa.com` 4, `www.wiaa.com` 2, `wa.milesplit.com` 2.
   - C — no WA tap today.
3. **Fill plan**: `wa_finalforms` (pages 1-50 wired) + held finalforms bodies → member schools with addresses (prototype 1,463 addr / 745 zip); `wa_milesplit` for team pages; school-site coach pages. Reachable: ~1,463 schools with addresses, coaches only via sites.
4. **True missing**: WSCCCA (`wsccca.com`); WSTFCA (`wstfca.com`); WFIS (`wfis.org`); WA SBE private schools (`sbe.wa.gov`); OSPI Education Directory (`eds.ospi.k12.wa.us`); OSPI ArcGIS; KingCo (`kingcoathletics.com`); Seattle Public Schools coach pages (`seattleschools.org`).
5. **Notes**: WA addresses are the strongest of group A; email coverage is 10 — finalforms rows carry AD names, not coach emails. Note sample `wiaa-finalforms-p51.html` exists while wired pages stop at 50.

## AZ

1. **Matrix row**: ccd 2631 | any 299 | tfxc 237 | email 5. Yield: 1,503/711/503/779/771/476 / 31 ok / 0 fail / 565 pages. `out/az.jsonl` 303.9 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-mountain/samples/`: `aia-alignments.html` 277.8 KB + `aia-alignments-activities.html` 83.0 KB + `aia-alignments-operators.html`, `aia-school-detail-58.html`, `aia-school-detail-4.html`, `aia-activity-xc-boys.html`, `aia-search-limit500.json` 7.5 KB (SRC-158), `aia-2026-{tf,xc}-tournament-guide.pdf`, `aia-athleticnet-xc-entry.pdf`, `aia-tf-state-results-18666.{txt,pdf}`, `aia-state-xc-2025-pdf-page{1,2,3,12}.txt`, `azpreps365-results-tf-boys.html` 41.9 KB.
   - B — raw: `www.aiaonline.org` 345, `az.milesplit.com` 2.
   - C — `probes/158-aia-json/`: `captures/` 445 bodies (65-110 KB each), `profile-<id>.body/.headers` for ~205 school profiles, `profile-failures.json` 3 B; `extract/158-aia-json/` exists but is EMPTY; `var/aia-smoke-20261004/` (`out/canonical-coaches.csv` 1.2 KB, `canonical-schools.csv` 646 B, `search-chandler-live.json` 3.7 KB, `school{25,68,100,116}-live.html` 89-103 KB, `http/` 4 bodies, 64 MB fjall store).
3. **Fill plan**: `az_aia` search adapter (q_a..q_z + school_N wired) over held search JSON; parse the 445 captured profiles for coach rows; `az_milesplit` for teams; prototype merged (299/237/5).
4. **True missing**: AIA member/sport-coach directory route (`aiaonline.org/schools`); AIA authenticated admin email directory (`admin.aiaonline.org`, restricted); CAA (`azcaa.com`); CAA Bound (gobound refused — documented); ACEC (`azchristianschools`); AZPreps365 team pages.
5. **Notes**: AIA search JSON is school-level (TF/XC-agnostic); profiles held but no extraction run. `extract/158-aia-json` empty — note for reruns.

## VA

1. **Matrix row**: ccd 2159 | any 21 | tfxc 9 | email 9. Yield: 884/0/0/24/19/33 / 3 ok / 1 fail / 363 pages. `out/va.jsonl` 158.8 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-midatlantic/samples/robots-vhsl.txt` 252 B (site disallows crawling); `research/sources/state-assoc-southeast/samples/`: `teams-va-milesplit.html`, `meets-va-milesplit.html`, `home-va-milesplit.html`, `vhsl-class5-state-meet-va-milesplit.html`, `milestat-redirect.html`, `robots-va-milesplit.txt`.
   - B — raw: `www.vhsl.org` 1, `va.milesplit.com` 2, `www.acps.k12.va.us` 2, `www.fcps.edu` 2, `www.pwcs.edu` 2, `www.vbschools.com` 2.
   - C — `extract/184-va-vdoe-directory/rows.csv` 304.2 KB + `probes/184-va-vdoe-directory/REPORT.md` 2.8 KB; `extract/185-va-visaa/rows.csv` 8.3 KB + `probes/185-va-visaa/REPORT.md` 2.3 KB; `extract/186-va-vhsl/qualified.json` 401 B + `probes/186-va-vhsl/REPORT.md` 1.3 KB; runs `var/tap-184-va-vdoe/`, `var/tap-184-va-vdoe-directory/`, `var/tap-185-va-visaa/`, `var/tap-186-va-vhsl/`.
3. **Fill plan**: VDOE extract (304.2 KB) = public-school directory; VISAA extract (8.3 KB) = private members; VHSL qualified.json partial; `va_vhsl` (html) + `va_milesplit` wired; prototype merged (21/9/9).
4. **True missing**: VHSL member directory (`vhsl.org`, robots-disallowed); VHSL Reports (`vhslreports.com`); VISAA XC/TF participation docs; VCPE (`vcpe.org`); Virginia Metro Athletic Conference; Virginia Track Coaches Association (`runsignup.com` MemberOrg).
5. **Notes**: VA is coach-starved (21 any / 9 tfxc); VDOE/VISAA are school-lists only. Prototype VA has 0 addr/0 zip.

## NJ

1. **Matrix row**: ccd 2575 | any 454 | tfxc 5 | email 9. Yield: 1,959/899/450/7/7/12 / 12 ok / 0 fail / 573 pages. `out/nj.jsonl` 347.3 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-midatlantic/samples/`: `njsiaa-member-info.html` 232.5 KB, `njsiaa-schools.html` 163.0 KB, `njsiaa-xc.html`, `njsiaa-tf-outdoor.html`, `njsiaa-sports.html`, `njsiaa-home.html`, `sitemap-njsiaa.xml`, `robots-njsiaa.txt`, `nj-milesplit-teams.html` 185.7 KB.
   - B — raw: `www.njsiaa.org` 14, `nj.milesplit.com` 2.
   - C — `probes/nj-njsiaa/`, `probes/sources/SRC-058/`, `probes/njxctfca/` (`FINDINGS.md` 9.0 KB, `manifest.json` 8.2 KB, `raw/njxctfca.org__links.html` 42.8 KB, robots), `probes/googlesites_hopatcong/` (FINDINGS 13.7 KB), `extract/SRC-066/rows.csv` 108 B + REPORT 7.2 KB (GMC TCA member list ~empty); `var/tap-nj-20261004/` (`readback/canonical-coaches.csv` 88.0 KB, `canonical-schools.csv` 65.4 KB, athletes/meets/recruiting CSVs, `store/`); `var/home-campus-NJ-20261004/`; `var/tap-home-campus-20261004/pass-nj.log` = 452 NJ schools / 0 coach rows.
3. **Fill plan**: `nj_njsiaa` p0-p8 on held member-information bodies → 1,959 school rows / 899 addr / 450 zip; NJ Home Campus section-12 run gives 452 schools but 0 coach rows (widget lacks coach data); `njxctfca` links body → county associations; `nj_milesplit`; prototype merged (454/5/9).
4. **True missing**: NJSIAA per-school details (restricted); NJ DOE School Directory (`homeroom6`); Shore TCA (`shorecoaches.com`); South Jersey TCA (`sjtrack.org`); Morris County TCA (`mctrack.org`); NJAIS (`njais.org`); Home Campus NJ selector (untested per SRC-096).
5. **Notes**: NJ's dataset is AD/school-rich but TF/XC-poor (5): coach rows must come from county TCA sites and school pages. GMC TCA extract is effectively empty (108 B).

## CO (no TAP-REGISTRY rows; absent from HELD-CLASSIFY)

1. **Matrix row**: ccd 1917 | any 337 | tfxc 331 | email 6. Yield: 1,496/756/376/3,401/1,096/14 / 5 ok / 0 fail / 652 pages. `out/co.jsonl` 471.8 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-mountain/samples/`: `chsaa-schools-home.html`, `chsaa-schools-path.html`, `chsaa-schools-staff.html`, `chsaa-school-cherry-creek.html`, `chsaa-school-telluride.html`, `chsaa-school-academy.html`, `chsaa-sport-xc.html`, `chsaa-sport-tf.html`, `chsaa-sports.html`, `chsaa-results.html`, `chsaa-results-calamo.html`, `chsaa-news-state-champions.html`, `chsaa-recap-5a-boys-xc-2025.html`, `chsaa-recap-5a-boys-tf-2026.html`, `chsaa-xc-champions.html`, `chsaa-media-tf-state-results-5a-2026.pdf`, `chsaa-tf-state-results-5a-2026.pdf`, `co-state-xc-2025-results-index.html`, `co-milesplit-home.html`, `robots-schools-chsaa.txt`, `robots-co-milesplit.txt`.
   - B — raw: `chsaanow.com` 380, `co.milesplit.com` 2.
   - C — none (no CO tap/probe).
3. **Fill plan**: `co_chsaanow` (chsaanow.com/schools/) + `co_school` per-school wired — held `chsaa-schools-staff.html` + 3 school pages show the staff-body format; `co_milesplit` for teams; prototype merged (337/331/6).
4. **True missing**: CHSAA per-school detail pages beyond the 3 samples (`chsaanow.com/schools/*`); Colorado High School Coaches Association (CHSCA); Colorado TF/XC coaches association; school athletics staff pages.
5. **Notes**: 3,401 TF/XC source rows already yielded but only 331 schools mapped and 6 emails — parsing held CHSAA school/staff bodies is the shortest path.

## IN

1. **Matrix row**: ccd 1928 | any 12 | tfxc 7 | email 5. Yield: 1,217/0/0/8/8/6 / 4 ok / 0 fail / 401 pages. `out/in.jsonl` 169.4 KB.
2. **Held evidence**:
   - A — none in state-assoc corpus trees.
   - B — raw: `www.ihsaa.org` 15, `www.doe.in.gov` 6, `www.iatccc.org` 6, `iatccc.org` 4, `www.myihsaa.net` 3, `myihsaa.net` 1, `in.milesplit.com` 2, plus ~25 Indiana school athletics hosts @4 (e.g. `bishopchatardathletics.com`, `calumetathletics.com`, `crispusattucksathletics.com`, `evansvilleharrisonathletics.com`, `evansvillenorthathletics.com`, `indianaathletics.com`, `indianapolissports.com`, `munciesports.com`, `munciecentralathletics.com`, `purduesports.com`, `southbendathletics.com`, `southbendrileysports.com`, `southbendwashingtonathletics.com`, `decaturcentralathletics.com`).
   - C — `probes/sources/SRC-113/FINDINGS.md` 4.7 KB + `extract/SRC-113-in-doe/rows.csv` 192.1 KB + `school-import.csv` 305.7 KB; `probes/sources/SRC-114/FINDINGS.md` 4.0 KB (`captures/www.ihsaa.org__schools.rendered.txt`) + `extract/SRC-114/REPORT.md` 3.2 KB + `rows.csv` 108 B; `var/tap-in-20261004/` (`in-school-names.txt` 61.7 KB); `var/tap-115-iatccc/`.
3. **Fill plan**: IN DOE extract (192.1 KB rows + 305.7 KB import) = full school roster; IHSAA rendered school list for member names; `in_ihsaa` (Membership History PDF) + `in_milesplit` wired; IATCCC tap for coach names; prototype merged (12/7/5).
4. **True missing**: IHSAA per-school detail pages; Indiana Association of Christian Schools (`indianaacs.org`); Eventlink XC (`eventlink.com`); Hoosier Crossroads Conference; school coach pages.
5. **Notes**: IN is essentially unharvested (any 12); school roster exists but no coach extraction from held bodies (`extract/SRC-114/rows.csv` 108 B is effectively empty).

## KY (no TAP-REGISTRY rows; absent from HELD-CLASSIFY)

1. **Matrix row**: ccd 1559 | any 416 | tfxc 271 | email 0. Yield: 1,653/584/292/1,190/579/415 / 4 ok / 0 fail / 730 pages. `out/ky.jsonl` 394.7 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-southcentral/samples/`: `ky-member-school-directory.html`, `ky-schools-arbiterlive.tsv`, `ky-arbiter-live-org2507.html`, `ky-school-enrollments.html`, `ky-enrollment-listings.html`, `ky-2025-xc-state-results.html`, `ky-2026-3a-outdoor-track-results.html`, `ky-xc-results-entries.html`, `ky-xc-301target.html`, `ky-track.html`, `ky-track-field.html`, `ky-cross-country.html`, `robots-live.arbiter.io.txt`.
   - B — raw: `khsaa.org` 6, `www.khsaa.org` 2, `ky.milesplit.com` 2; `out/ky.jsonl` 394.7 KB.
   - C — none; `var/st-KY.json` (plan; `teams` owed; sweepable milesplit/athleticnet/arbiter_orgs).
3. **Fill plan**: `ky_khsaa` (Google Sheets export) for school roster; held Arbiter org-2507 page + TSV for teams; `ky_milesplit`; prototype merged (416/271/0).
4. **True missing**: KHSAA member-directory route (`khsaa.org`); Kentucky Track & Cross Country Coaches Association; TFRRS KY; school sites (emails).
5. **Notes**: 271 TF/XC schools already derived and ZERO emails — this state needs email capture (school/coach pages), not more school names.

## LA

1. **Matrix row**: ccd 1330 | any 287 | tfxc 287 | email 0. Yield: 827/0/0/1,715/736/238 / 3 ok / 1 fail / 560 pages. `out/la.jsonl` 645.8 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-southcentral/samples/`: `la-coaches-directory-2025-26.pdf` 6.5 MB + `.txt` 977.4 KB (SRC-212), `la-school-directory.html` 62.1 KB (SRC-216), `la-schools.html`, `la-schools-all-academic.html`, `la-cross-country.html` 65.2 KB (SRC-217), `la-outdoor-track-field.html` 64.1 KB (SRC-218), `la-champ-central.html`, `la-apply-membership.html`, `la-forms-resources.html`, `la-milesplit-teams.html` + `.txt`, `la-sitemap.xml`, `la-2026-state-champ-results.{txt,pdf}`, `robots-la.milesplit.com.txt`, `robots-lhsaaonline.org.txt`.
   - B — raw: `www.lhsaa.org` 11, `la.milesplit.com` 2.
   - C — `extract/SRC-212/REPORT.md` (coaches-directory extraction) + `extract/SRC-215/REPORT.md` 7.4 KB + `rows.csv` 98.7 KB (LHSAA registered XC coaches, 2025-09-22); `extract/213-la-bese-nonpublic/rows.csv` 69.7 KB; `probes/213-la-bese-nonpublic/`; `probes/217-la-lhsaa-xc/` (raw empty, redirect only); `probes/218-la-lhsaa-tf/`; `var/tap-213-la-bese/`.
3. **Fill plan**: SRC-215 extract (98.7 KB) → XC coach rows per school; LHSAA coaches-directory PDF+text (SRC-212) → coach names/schools; BESE rows (69.7 KB) + LHSAA school directory → school universe; `la_lhsaa_coaches` + `la_milesplit` wired; prototype merged (287/287/0).
4. **True missing**: LHSCA (`lhsaa.org/lhsca`); ACEL (`theacel.com`); Louisiana School Finder (`louisianaschools.com`); DirectAthletics LA teams (`directathletics.com` route); school sites (emails).
5. **Notes**: LA's 287 "any" rows are all TF/XC rows — coach data is TF/XC-specific, but email-less. `probes/217-la-lhsaa-xc` raw is empty (redirect only) — don't count it as a body.

## NE (no TAP-REGISTRY rows; absent from HELD-CLASSIFY)

1. **Matrix row**: ccd 1115 | any 293 | tfxc 293 | email 0. Yield: 896/0/0/1,062/1,062/0 / 4 ok / 0 fail / 518 pages. `out/ne.jsonl` 219.8 KB.
2. **Held evidence**:
   - A — `research/sources/state-assoc-plains/samples/`: `nsaa-directory-form-2026-09-22.html`, `robots-nsaahome.txt`, `robots-secure-nsaahome.txt`.
   - B — raw: `secure.nsaahome.org` 19, `nsaahome.org` 6, `www.nsaahome.org` 4, `ne.milesplit.com` 2.
   - C — none; `var/st-NE.json` (plan; sweepable athleticnet/milesplit/plain_names; `teams` not complete).
3. **Fill plan**: `ne_nsaa` wired source = `secure.nsaahome.org/nsaaforms/direxportscreen.php` (directory export) — held 29 NSAA bodies + form sample; parse for school + AD/coach rows; `ne_milesplit` for teams; prototype merged (293/293/0).
4. **True missing**: NSAA directory export (already wired but email-less); Nebraska Coaches Association (`nebraskacoaches.org`); school coach/staff pages for emails.
5. **Notes**: NE any = tfxc = 293 and head = 1,062 with ZERO AD rows and ZERO emails — NSAA bodies carry participation/coach facts but no contact columns; school pages are the only email route.

---

## Series notes (cross-state)

- **AD-only bodies**: WA finalforms (AD names, no coach emails), NJ NJSIAA member info (AD/school rows), FL FHSAA widget (school + coach rows with email — exception).
- **TF/XC-only, email-less**: LA LHSAA XC coaches extract + coaches-directory PDF, NE NSAA export, KY derived rows — names/affiliations only.
- **Name/URL-only sources**: TX UIL alignments/codes, CA CIF census/section pages, VA VDOE/VISAA/VHSL lists, IN DOE extract, CO CHSAA school pages.
- **No-registry states (CO, KY, NE)**: held evidence is corpus + prototype raw only; every fill-plan step must run via prototype parser or fresh extraction from those held bodies. No new fetches in this work order.
- **Zero-email cluster** (KY, LA, NE = 0; AZ, CO, IN 5-6; VA, NJ, IL, WA ≤ 20): email capture is the dominant gap for group A; school-site pages or association rosters with contact columns are the missing pieces.
