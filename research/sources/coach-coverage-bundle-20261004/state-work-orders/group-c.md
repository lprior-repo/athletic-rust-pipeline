# State work orders — group C (17 states)

Scope: ND SD MT ID WY UT NV NM OR KS WV DE DC RI NH VT ME.
Constraints honored: zero network fetches, zero builds/tests, read-only elsewhere; every path below was read or listed in this session.
Sources of truth: `STATE-COVERAGE-MATRIX.md` + `var/state-coverage-matrix-20261004.json` (counts); `HELD-CLASSIFY.json` (registry subset per state);
`HELD-INVENTORY.md` (tiers); corpus trees `research/sources/*/samples/` (2026-09-22); `census-prototype/raw/` (56,294 bodies / 682 hosts) and
`census-prototype/out/{report.json,coverage.md,extra/}`; today's artifacts under `var/tap-*/` and bundle `probes/`, `extract/<n>-*/`.

Registry fact for this group (grep over `HELD-CLASSIFY.json`):
- **No TAP-REGISTRY entry at all** for: ND, MT, ID, WY, NM, WV, DE, DC, RI, NH, ME (grep `"state":"(…)"` → no match).
- With entries: SD (SRC-222…228), OR (SRC-146…154), UT (SRC-164…174), NV (SRC-175…182), KS (SRC-239…246), VT (SRC-082…085, 090, 094, 095).
- `var/tap-corpus-coverage-20261004.json` per_state screen (covered/total): SD 4/7, KS 3/8, OR 3/9, UT 5/11, NV 0/8, VT 2/6. Nothing for the other 11 states.

Adapter inventory relevant to this group (`crates/census-crawl/src/registry/table/{directories,from_mshsl}.rs` + module listings):
`plain_names` (NDHSAA+NSAA), `uhsaa`, `mpa` (ME, SCHOOL_COACH_NAMES), `riil` (SCHOOL_COACH_NAMES), KSHSAA directory descriptor, `coach_directories`
(DragonFly; registered states include ND/NDHSAA, MT/MHSA, ID/IdHSAA, NM/NMAA, WY/WHSAA, DC/DCSAA, DE/DIAA), `arbiter` (registered orgs NHIAA 2132, MHSA 4497,
WVSSAC 4223; `crates/census-crawl/src/arbiter/README.md`), `coach_contacts` (gobound → namespace `bound`), `milesplit`, `athleticnet`, `tfrrs`, `nces`.
No Rust adapter exists for OR/NV/NM/ID/MT/WY/WV/DE/DC/NH/VT/SD associations; their held bodies route through prototype parsers or fresh extraction.

Tier note for every state below: today's tap stores are scratch and unmerged (`STATE-COVERAGE-MATRIX.md` caveat); `census-prototype/out/extra/*.jsonl`
are parsed prototype outputs (reuse-only); `report.json` records are the 2026-09-29 merged-run receipts; `coverage.md` host counts are raw response counts under `census-prototype/raw/`.

---

## ND — North Dakota
### 1. Matrix row
ccd 542 | any-coach 174 | TF/XC 164 | email 43 (8%). (`STATE-COVERAGE-MATRIX.md`)

### 2. Held evidence
- A (corpus 2026-09-22): `research/sources/state-assoc-plains/samples/ndhsaa-schools-2026-09-22.html` 97,754 B — NDHSAA member-school index, **169 schools** (CAPTURES.md); `robots-ndhsaa.txt` 24 B; `athleticlive-heros-nd-xc-2026-09-22.json` 1,369 B — 6 ND XC meets (XC-only).
  `research/sources/coach-directories-national/samples/assoc-home/ND__ndhsaa.com.html` 104.9 KB on disk (30,222 wire B) — NDHSAA home; `samples/api/ND__*.js` bootstrap assets (CAPTURES.md rows 15–17).
- B (prototype): raw host counts `ndhsaa.com` 171 responses, `www.ndhsaa.com` 3, `ndhsaa.finalforms.com` 1, `athleticlive.com` 4 (`coverage.md`, read :367–1024).
  `census-prototype/out/report.json`: `nd_ndhsaa` cache 97,754 B → 169 schools, 0 coaches (index only); `nd_dragonfly` 165,214 B → **548 schools / 527 with address**; `nd_milesplit` 74,423 B → 152 teams.
  `census-prototype/out/extra/ND-dragonfly.jsonl` 264.9 KB (548 school rows); `ND-schools.jsonl` 167.1 KB = **169 lines** — per-school ndhsaa.com detail bodies (72–75 KB each) with Track/XC head-coach names + AD rows, **no emails** (sample read: Alexander/Ashley/Belfield); `ND-school-sites.jsonl` 3.6 KB.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| ndhsaa.com school detail bodies (ND-schools.jsonl, 169) | adapter `plain_names` (`crates/census-crawl/src/plain_names/{nd,nd_coaches,nd_walk}.rs`) | 169 member schools with TF/XC + AD names |
| `nd_dragonfly` directory | adapter `coach_directories` (NDHSAA registered) | 548 schools / 527 addresses |
| NDHSAA schools index (97,754 B) + ND-schools.jsonl | prototype `custom:nd_ndhsaa` parser (reuse) | reconciles 169 vs 548 keys |
| nd.milesplit 152 teams | adapter `milesplit` | school/team discovery |
Estimated: 169 NDHSAA members fully named (any-coach 174 ≈ done); address join lifts 548 rows; email lane is 0 from these bodies.
### 4. True missing
- NDHSAA school pages do not publish emails (COACH_CONTACTS_REFUSAL prose: "ND 0 emails") → need per-school district/school sites (e.g. West Fargo, Bismarck, Fargo Davies, Grand Forks Central, Minot, Williston) for emails.
- NDHSAA cooperative-team alignments (schools fielding combined teams) are not in the held index → NDHSAA co-op pages needed.
- `ndhsaa.finalforms.com` (1 body only) — full FinalForms school/coach rosters not held.
### 5. Notes
NDHSAA bodies are **name-only, no emails**; athleticlive capture is XC-only (6 meets); milesplit = discovery only; dragonfly rows carry addresses but no coach fields (report.json coaches=0).

---

## SD — South Dakota
### 1. Matrix row
ccd 744 | any-coach 138 | TF/XC 135 | email 2 (0%).

### 2. Held evidence
- Registry: SRC-222 (SDCCTFCA roster HELD, `extract/SRC-222/rows.csv` 57.5 KB), SRC-223 (SD DOE directory HELD, `extract/223-sd-edudir/rows.csv` 92.2 KB), SRC-224 (SDHSAA/Bound DERIVED — `probes/gobound/FINDINGS.md` 6.1 KB: gobound refused), SRC-225 (SDHSAA XC hub HELD), SRC-226 (track hub DERIVED), SRC-227 (cooperatives HELD), SRC-228 (SDCCTFCA membership HELD).
- A: `state-assoc-plains/samples/sdhsaa-cross-country-2026-09-22.html` 509,970 B; `sdhsaa-cross-country-region-2026-09-22.html` 286,293 B; `robots-sdhsaa.txt` 156 B. `coach-directories-national/samples/assoc-home/SD__sdhsaa.com.html` 1.0 MB on disk (117,030 wire).
- B: raw `sdhsaa.com` 3 + `www.sdhsaa.com` 2 + `www.gobound.com` 2,470 (multi-state). `extra/SD-bound.jsonl` 566.8 KB = **560 lines** — gobound per-school staff pages (`/sd/sdhsaa/{boys,girls}{trackfield,crosscountry}/2026-27/<slug>/v/staff`) with TF/XC head+assistant coach rows, `coaches_withheld` counters; `SD-school-sites.jsonl` 4.3 KB.
- C: `probes/227-sdhsaa-cooperatives/REPORT.md` 2.7 KB + `captures/` page 484,091 B + `extract/227-sdhsaa-cooperatives/qualified.json` 1.2 KB; `probes/228-sdhsca-membership/REPORT.md` 4.3 KB + `extract/228-sdhsca-membership/rows.csv` 59.5 KB; `var/tap-227-sdhsaa-cooperatives/` (4 store bodies `store/http/*.body` + 4 archive capture bodies under `archive/captures/*/bodies/`, `agreements.txt`, `terms.txt`); `var/tap-228-sdhsca/import-store/` (fjall + out; **286 coaches imported** per HELD-INVENTORY.md); `var/tap-223-sd-edudir/{store,import-store}` (3 http bodies under `store/http/`).
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| SD-bound.jsonl (560 staff pages) | adapter `coach_contacts` (namespace `bound`) | TF/XC staff rows for the ~140–200 SDHSAA schools with Bound pages |
| SDCCTFCA roster + membership (SRC-222/228; tap-228 imported 286 coaches) | reuse import-store + prototype parser | association coach rows incl. emails (prior study counted 4 emails) |
| `223-sd-edudir/rows.csv` | prototype `custom:*` directory parser | school addresses/city for join |
| SDHSAA XC + region + cooperatives bodies | fresh extraction (held HTML) | co-op pairings, XC participation |
Estimated: 138 any-coach today; Bound + rosters can reach all schools that field TF/XC; emails stay a handful.
### 4. True missing
- `www.gobound.com/sd/sdhsaa/...` staff pages for schools not among the 560 held rows (gobound live was refused/listed as blocked).
- SDHSAA member directory itself (SRC-224) — not held; school-side directory (SD DOE naming).
- School district sites for coach emails (SDHSAA publishes none).
### 5. Notes
Bound bodies = TF/XC-only, name-only; SDCCTFCA rosters are the only email-bearing SD lane; edudir = addresses only; tap-228 had no HTTP bodies (import came from frozen inputs).

---

## MT — Montana
### 1. Matrix row
ccd 854 | any-coach 274 | TF/XC 173 | email 174 (20%).

### 2. Held evidence
- Registry: none (no `"state":"MT"` in HELD-CLASSIFY.json).
- A: `state-assoc-mountain/samples/`: `home-mhsa.html` 692,849 B; `mhsa-member-schools.html` 337,334; `mhsa-schools-directory.html` 301,211; `mhsa-enrollment-numbers.html` 336,030; `mhsa-cross-country.html` 331,265; `mhsa-track-field.html` 332,737; `mhsa-xc-postseason.html` 317,372; `mhsa-tf-postseason.html` 337,610; `mhsa-search-athleticnet.html` 290,542; `mhsa-enrollment-alpha.pdf` 135,327; `mhsa-enrollment-classification.pdf` 137,357; `mhsa-arbiter-directory.html` 4,278; `mhsa-state-meet-results-competitive.html` 23,473; `robots-arbiter-live.txt` 4,278; `robots-competitivetiming.txt` 40. `coach-directories-national/samples/assoc-home/MT__mhsa.org.html` 678.9 KB on disk (136,340 wire).
- B: raw `www.mhsa.org` 2. `report.json`: `mt_mhsa` cache 289,162 B → **0 schools parsed**; `mt_dragonfly` 360,977 B → **358 schools / 310 addresses**; `mt_milesplit`. `extra/MT-dragonfly.jsonl` 391.4 KB; `MT-arbiter.jsonl` 197.8 KB = **220 lines** — Arbiter org 4497 children + AD rows (names, phone, enrollment; **no emails**); `MT-school-sites.jsonl` 1.9 KB.
- C: none today (arbiter adapter fixtures live in `crates/census-crawl/src/arbiter/`, README lists MHSA org 4497 = 215 schools).
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| MT DragonFly MHSA directory | adapter `coach_directories` (MHSA registered) | 358 schools / 310 addresses |
| `MT-arbiter.jsonl` (org 4497) | adapter `arbiter` (registered org) | 215 school AD contacts |
| mhsa.org corpus pages (member schools, enrollment PDFs, postseason) | fresh extraction from held HTML/PDF | school/class enrollment join |
Estimated: 274 any-coach now; 358 w/address after dragonfly; AD layer 215.
### 4. True missing
- MHSA school pages' coach contacts (TF/XC) — corpus pages are directories/enrollment, not staff.
- School-site staff directories (MT-school-sites only 1.9 KB).
- mhsa.org live routes beyond captured set (403/404 observed in corpus).
### 5. Notes
MT-arbiter rows are AD-only with no emails; dragonfly rows carry addresses but `coaches: 0` in report.json; enrollment PDFs give class but not contacts.

---

## ID — Idaho
### 1. Matrix row
ccd 820 | any-coach 234 | TF/XC 208 | email 136 (17%).

### 2. Held evidence
- Registry: none.
- A: `state-assoc-mountain/samples/`: `home-idhsaa.html` 45,804 B; `idhsaa-directory.html` 252,565; `idhsaa-coaches.html` 52,554; `idhsaa-cross-country.html` 41,643; `idhsaa-track-field.html` 43,453; `idhsaa-athletic-directors.html` 43,970; `athleticnet-idhsaa-xc-state-263030.html` 9,292; `robots-idhsaa.txt`. `coach-directories-national/samples/assoc-home/ID__idhsaa.org.html` 44.7 KB on disk (12,313 wire); `samples/api/ID__*.js` 56,879 + 44,371 B.
- B: raw `www.idhsaa.org` 354 responses. `report.json`: `id_idhsaa` cache 253,833 B → **177 schools** with AD name/phone/email — emails are JS-obfuscated (`document.write(window.atob('…'))`, sample decoded `lewisn@aberdeen58.org`); `id_dragonfly` 363,858 B → **461 schools / 449 addresses**; `id_milesplit`. `extra/ID-dragonfly.jsonl` 392.8 KB; `ID-school-sites.jsonl` 2.4 KB.
- C: none today.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| idhsaa.org directory body (253,833 B) | fresh extraction (must base64-decode atob emails), or new adapter mirroring `plain_names` shape | 177 schools with AD contacts+emails |
| ID DragonFly IdHSAA directory | adapter `coach_directories` (IdHSAA registered) | 461 schools / 449 addresses |
| `idhsaa-coaches.html` 52,554 B + `idhsaa-{cross-country,track-field}.html` | fresh extraction (held HTML) | sport-level coach listing |
Estimated: 208 TF/XC today; dragonfly join lifts address coverage to ~449; AD emails 177.
### 4. True missing
- Per-sport (TF/XC) coach names/emails — idhsaa coach pages are the association's own, not school staff.
- School-site staff directories (ID-school-sites 2.4 KB only).
- Idaho school district sites for email coverage beyond ADs.
### 5. Notes
IdHSAA directory emails require atob decoding (blocker for a naive parser); dragonfly = addresses only (`coaches: 0`); athleticnet capture is one state-meet page.

---

## WY — Wyoming
### 1. Matrix row
ccd 367 | any-coach 75 | TF/XC 73 | email 74 (20%).

### 2. Held evidence
- Registry: none.
- A: `state-assoc-mountain/samples/`: `home-whsaa.html` 21,570 B (two fetches, same file); `whsaa-2026-28-adms.pdf` 785,163 B — WHSAA administrators/AD directory + alignment PDF; `whsaa-track-state.html` 15,106; `whsaa-xc-state2.html` 5,838. `coach-directories-national/samples/assoc-home/WY__whsaa.org.html` 21.1 KB on disk (5,501 wire).
- B: raw `www.whsaa.org` 3; `whsaa.finalforms.com` 6; `wy.finalforms.com` 6; `edu.wyoming.gov` 6. `report.json`: `wy_whsaa` cache 22,411 B → **0 schools parsed**; `wy_dragonfly` 113,274 B → **93 schools / 88 addresses**; `wy_milesplit`. `extra/WY-dragonfly.jsonl` 138.1 KB; `WY-school-sites.jsonl` 1.2 KB.
- C: none today.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| WY DragonFly WHSAA directory | adapter `coach_directories` (WHSAA registered; fixture `coach_directories/probe/WY/directory-1.json` in-tree) | 93 schools / 88 addresses |
| `whsaa-2026-28-adms.pdf` 785,163 B | fresh extraction (held PDF) | AD names per school (WHSAA's best contact artifact) |
| finalforms bodies (whsaa/wy, 6+6 responses) | fresh extraction from held bodies | school roster pages |
Estimated: 75 any-coach now; 93 w/address; AD PDF adds administrators.
### 4. True missing
- WY school-site staff directories (WY-school-sites 1.2 KB) — the TF/XC coach/email lane.
- whsaa.org content beyond captured home/track/XC (live 403/404).
- Wyoming district sites (Laramie, Cheyenne, Casper, Gillette, Sheridan…) for emails.
### 5. Notes
whsaa.org home parse yielded 0 schools; ADMS PDF is the richest WY artifact held; wy/whsaa finalforms = school index only; dragonfly = addresses only.

---

## UT — Utah
### 1. Matrix row
ccd 1121 | any-coach 146 | TF/XC 145 | email 4 (0%).

### 2. Held evidence
- Registry: SRC-164 (UHSAA profiles HELD), SRC-165–168 (USBE DERIVED), SRC-169 (realignment HELD), SRC-170 (sanctioned events DERIVED), SRC-171 (participation matrix HELD), SRC-172 (UCCTCA DERIVED), SRC-173 (coach-assoc reps DERIVED), SRC-174 (Pine View DERIVED).
- A: `state-assoc-mountain/samples/`: `home-uhsaa.html` 547,726 B; `uhsaa-school-directory.html` 483,588; `uhsaa-school-alta.html` 456,682; `uhsaa-teams-xc-boys.html` 405,876; `uhsaa-cross-country.html` 456,073; `uhsaa-track-and-field.html` 456,597; `uhsaa-participation-numbers.pdf` 312,115; `uhsaa-realignment-2025-27.pdf` 533,693; `runnercard-meet-1001674.html` 86,565; `runnercard-uhsaa-state-results.html` 913; `robots-uhsaa.txt` 1,181. `coach-directories-national/samples/assoc-home/UT__uhsaa.org.html` 534.9 KB on disk (157,713 wire).
- B: raw `uhsaa.org` 328 + `www.uhsaa.org` 2. `report.json`: `ut_uhsaa` cache 483,323 B → **162 schools, 0 addresses**; `ut_milesplit`. `extra/UT-school-sites.jsonl` 9.3 KB = 9 lines (Bingham, Herriman, North Sanpete, Riverton, Taylorsville… school-site coach/email probes).
- C: `var/tap-uhsaa-20261004{,b,c,d}/store/http/` — 8 unique bodies total (`c75cbdcf`, `ed48031a`, `fa086d7f`, `9ea7d897` in a+b; c adds `2771755324`, `578ce667`, `37d1a87a`; d adds `7d5d7506`), per-school profile pages (meta read: `https://uhsaa.org/school-directory/?id=American%20Fork&Reg=3&schoolID=3`, 458,535 B); out CSVs per tap: `canonical-schools.csv` 618–804 B, `canonical-coaches.csv` 998 B–2.9 KB (only a few schools resolved; scratch/unmerged).
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| UHSAA school-directory body (483,323 B) | adapter `uhsaa` (registry slug "uhsaa", SCHOOL_COACH_CONTACT) | 162 listed schools |
| today's per-school profile bodies (tap-uhsaa a–d) | same adapter over held bodies (no refetch) | fills per-school profiles (American Fork seeded) |
| `UT-school-sites.jsonl` (9 rows) | fresh extraction/reuse | school-site coach emails (Jordan/Granite district samples) |
| participation/realignment PDFs | fresh extraction | class/region join |
Estimated: 162 UHSAA schools; addresses must come from USBE/NCES side (UHSAA has none).
### 4. True missing
- UHSAA per-school profiles for all 162 (only American Fork + partial in today's taps).
- USBE Utah Schools Directory / CACTUS endpoint (`schools.utah.gov`, SRC-165/167/168 — DERIVED only) for addresses.
- School-site staff directories for emails (Alpine, Davis, Nebo, Weber districts) — lane exists at 9.3 KB.
### 5. Notes
UHSAA bodies are name/sport only; today's taps are scratch stores and the STATE-COVERAGE-MATRIX caveat excludes them; runnercard bodies are meet results (XC timing), not coaches.

---

## NV — Nevada
### 1. Matrix row
ccd 746 | any-coach 100 | TF/XC 97 | email 4 (1%).

### 2. Held evidence
- Registry: SRC-175 (Southern NV TF/XC association HELD `extract/SRC-175/{REPORT.md 11.4 KB, rows.csv 58.8 KB}`), SRC-176 (NV DOE DERIVED), SRC-177 (Washoe school-sites DERIVED), SRC-178 (S. NV Track coaches CSV HELD, rows 30.7 KB), SRC-179 (S. NV XC coaches CSV HELD, rows 25.9 KB), SRC-180 (private dir DERIVED), SRC-181 (North Valleys DERIVED), SRC-182 (NIAA publications DERIVED — `niaa-members.html` corpus exists, prestosports route not captured).
- A: `state-assoc-westcoast/samples/`: `niaa-home.html` 40.8 KB; `niaa-members.html` 27.2 KB; `niaa-xc.html` 22.2 KB; `niaa-xc2.html` 36.0 KB; `niaa-xc-archive.html` 104.8 KB; `niaa-finalforms-state-schools.html` 136.4 KB; `niaa-finalforms-p9.html` 103.0 KB; `niaa-coaches.html` **0 B**; `niaa-sports-track.html` / `track2` **0 B**; `robots-niaa*.txt`. `coach-directories-national/samples/assoc-home/NV__niaa.org.html` 193.7 KB on disk (31,840 wire).
- B: raw `niaa.finalforms.com` 234 responses; `niaa.com` 10; `niaa.finalforms.com`. `report.json`: `nv_finalforms` p1–p9 (p1 141,373 B → **15 schools / 14 addresses**; 9 pages ≈ ~130 schools total); `nv_milesplit`. `extra/NV-school-sites.jsonl` 6.3 KB.
- C: extract CSVs SRC-175/178/179 (58.8+30.7+25.9 KB) — Southern Nevada coach rosters.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| niaa.finalforms.com pages 1–9 (234 raw responses) | prototype `custom:finalforms` parser (reuse) | ~130 schools w/address |
| SRC-175/178/179 rows.csv | reuse extract (prototype parser) | TF/XC coach names (Clark County-heavy) |
| `niaa-members.html` + `NV-school-sites.jsonl` | fresh extraction | member list + school-site lane |
Estimated: 100 any-coach now; finalforms lifts address coverage; coaches from S. NV CSVs.
### 4. True missing
- Clark County School District school-site staff directories (ccsd.net sites; SRC-177 Washoe lane same gap).
- NIAA publications/prestosports route (SRC-182) + `niaa-coaches` page is empty (0 B) → NIAA coach listing needed.
- Nevada private-school directory (SRC-180) for non-NIAA schools.
### 5. Notes
finalforms = school/address only (no coaches); S. NV CSVs are coach-name lanes (TF/XC); niaa.org sport/coaches pages captured empty — a fresh route is needed, not a refetch of 0-B pages.

---

## NM — New Mexico
### 1. Matrix row
ccd 927 | any-coach 195 | TF/XC 89 | email 159 (17%).

### 2. Held evidence
- Registry: none.
- A: `state-assoc-mountain/samples/`: `home-nmact.html` 599,330 B (nav variant 63,216 B overwritten); `nmaa-member-schools.html` 598,393; `nmaa-cross-country.html` 251,988; `nmaa-track-and-field.html` 258,853; `nmaa-for-coaches.html` 177,116; `nmaa-for-athletic-directors.html` 190,511; `nmaa-records.html` 613,634; `nmaa-section4-classification.pdf` 394,730; `nmaa-2025-cc-results.pdf` 357,394; `nmaa-2026-4a5a-boys-rosters.pdf` 486,524; `nmaa-2026-4a5a-final-results.pdf` 167,903; `robots-nmact.txt`. `coach-directories-national/samples/assoc-home/NM__nmact.org.html` 585.3 KB on disk (63,216 wire).
- B: raw `www.nmact.org` 4; `nmact.finalforms.com` 6; `nmaa.finalforms.com` 4; `nm.finalforms.com` 6. `report.json`: `nm_nmact` cache 599,315 B → **162 schools / 151 with address+city+zip**; `nm_dragonfly` 463,104 B → **751 schools / 741 addresses**; `nm_milesplit`. `extra/NM-dragonfly.jsonl` 479.8 KB; `NM-school-sites.jsonl` 13.0 KB.
- C: none today.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| NM DragonFly NMAA directory | adapter `coach_directories` (NMAA registered) | 751 schools / 741 addresses |
| `nm_nmact` member-schools body (599,315 B) | prototype `custom:nm_nmact` parser (reuse) | 162 schools w/address+zip |
| `NM-school-sites.jsonl` 13.0 KB | reuse | school-site coach rows |
| nmaa for-coaches/AD/records pages | fresh extraction | association-level contacts |
Estimated: 195 any-coach now; dragonfly completes addresses; coach emails remain the gap.
### 4. True missing
- NMAA per-school coach pages (for-coaches is association-level; sport pages are results/records).
- New Mexico school district sites beyond the 13 KB school-site lane (APS, Las Cruces, Rio Rancho…).
- NM PED school directory (not held in any tier).
### 5. Notes
NM-dragonfly is the largest school set in this group (751) but has `coaches: 0`; nmaa pages are sport/record-heavy; nmact gives addresses.

---

## OR — Oregon
### 1. Matrix row
ccd 1300 | any-coach 303 | TF/XC 284 | email 293 (23%) — richest of this group.

### 2. Held evidence
- Registry: SRC-146 HELD+ADAPTER (`osaa-schools.html` 31.1 KB, `osaa-schools-full-members.html` 57.1 KB, `osaa-home.html` 212.7 KB; `research/sources/osaa/SOURCE_REPORT.md` 2.7 KB), SRC-149 HELD+ADAPTER (`osaa-coaches.html` 101.0 KB), SRC-147/148/150/151/152/153/154 DERIVED.
- A: `state-assoc-westcoast/samples/`: `osaa-home.html` 212.7 KB; `osaa-schools.html` 31.1 KB; `osaa-schools-full-members.html` 57.1 KB; `osaa-coaches.html` 101.0 KB; `osaa-school-347.html` 95.5 KB (per-school profile sample); `osaa-xc-results.html` 38.6 KB; `osaa-gxc.html` 52.8 KB; `osaa-participation.html` 45.3 KB; `osaa-participation-2025-26-fall.xlsx` 45.9 KB; `robots-osaa.txt` 143 B. `coach-directories-national/samples/assoc-home/OR__osaa.org.html` 213.3 KB on disk (18,902 wire).
- B: raw `www.osaa.org` 302 responses. `report.json`: `or_osaa` cache 59,051 B → **299 full-member schools** (index only); `or_milesplit`. `extra/OR-schools.jsonl` 289.8 KB = **298 lines** — per-school OSAA profile bodies (~95–110 KB each) with coach rows incl. **emails for ADs** and TF/XC head coaches (sample: Adrian track `george.ellsworth@adriansd.org`); `OR-school-sites.jsonl` 14.9 KB.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| `OR-schools.jsonl` (298 profile bodies) | prototype `custom:or_osaa` parser (reuse) | 298 schools with TF/XC + AD coach rows |
| `osaa-coaches.html` + full-members index | reuse/fresh extraction | statewide coach-directory reconciliation |
| `OR-school-sites.jsonl` 14.9 KB | reuse | school-site email lane |
Estimated: 298 reachable → covers the 284 TF/XC gap; ~293 email rows already.
### 4. True missing
- OSAA index has 299 members; only per-school pages for a handful beyond the 298 JSONL rows may be missing (verify count).
- Private/independent schools outside OSAA membership (Oregon Federation of Independent Schools, SRC-148 — `ofisweb.org` not held).
- OACA (oregoncoach.org, SRC-150/151) association directory not held.
### 5. Notes
OR is the model lane: prototype parsed profile bodies already carry coach names+emails; emails are mostly ADs — TF/XC coach emails exist in some rows (Adrian, Amity examples).

---

## KS — Kansas
### 1. Matrix row
ccd 1356 | any-coach 528 | TF/XC 1 | email 525 (39%).

### 2. Held evidence
- Registry: SRC-239 (KSDE directory HELD, `probes/239-ks-directory/captures/` 252.5 KB), SRC-240 (KSHSAA leagues HELD, `probes/ks-kshsaa/raw/directory-search-name-a.json` 489,295 B), SRC-241 (KCCTFCA HELD, `probes/kcctfca/FINDINGS.md` 11.9 KB), SRC-242 (approved schools HELD), SRC-243 (KSDE annual PDFs DERIVED), SRC-244 (KCCTFCA winter clinic HELD), SRC-245 (Flint Hills DERIVED), SRC-246 (KCAA on AthleticNET ADAPTER+DERIVED).
- A: `state-assoc-plains/samples/kshsaa-publicclassifications-2026-09-22.json` 31,885 B — 348 classified schools (36/36/36/64/64/112 per CAPTURES.md); `robots-kshsaa.txt` 135; `robots-kshsaa-api.txt`. `coach-directories-national/samples/dir/KS__kshsaa-api-letter-a.html` 123,646 wire (489,381) — full directory search name/a; `assoc-home/KS__kshsaa.org.html` 790 wire (1,304).
- B: raw `kshsaa-api.kshsaa.org` 5. `report.json`: `ks_kshsaa` 489,375 B → **526 schools with address+city+zip, 526 AD-coach rows, TF/XC 0**; `ks_milesplit`. `extra/KS-school-sites.jsonl` 3.3 KB.
- C: `var/tap-239-ks-directory/` — `report.body` 185 B (302 redirect), `report.headers` (Location `/Directory_Rpts/Oops.aspx`), `report-form.txt` 128.7 KB (KSDE search form), `store/`; bundle probe raw JSON 489,295 B; KCCTFCA winter-clinic/awards pages in `probes/kcctfca/`.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| KSHSAA directory body (489,375 B; letter-a capture) | adapter/descriptor KSHSAA (`directories.rs`, StructuredApi) | 526 schools + AD names/addresses |
| `kshsaa-publicclassifications.json` | fresh extraction | 348 classified schools/classes |
| KCCTFCA probe bodies (clinic, awards, membership) | fresh extraction | TF/XC coach names for association members |
| `report-form.txt` (KSDE form) + `223`-style dir rows | fresh extraction | district/address join |
Estimated: 526 addressable now; the TF/XC=1 cell is the real gap — requires KCCTFCA/KSHSAA sport rosters.
### 4. True missing
- KSHSAA directory letters other than `a` (verify whether name/a returns the whole set — 526 rows suggests it may; if per-letter, b–z needed).
- KCCTFCA membership list (only clinic/awards pages held) — coach-name lane.
- KCAA (Christian schools) team directory via athleticnet (adapter lane).
### 5. Notes
KS email 39% = AD emails from KSHSAA; TF/XC coach coverage needs the coaches-association lane; tap-239 only confirmed the report route redirects.

---

## WV — West Virginia
### 1. Matrix row
ccd 701 | any-coach 268 | TF/XC 43 | email 1 (0%) — worst email cell in this group.

### 2. Held evidence
- Registry: none.
- A: `state-assoc-southeast/samples/`: `home-wvssac.html` 680.5 KB; `school-directory-wvssac.html` 266.3 KB; `classifications-wvssac.html` 267.9 KB; `classifications-2025-27-wvssac.pdf` 123.2 KB; `cross-country-wvssac.html` 417.2 KB; `track-wvssac.html` 418.5 KB; `programs-wvssac.html` 280.3 KB; `wpjson-page-8039-wvssac.json` 4.2 KB; `wpjson-search-track-wvssac.json` 2 B (empty); `wvmetronews-track-roundup-wvssac.html` 95.7 KB; `robots-wvssac.txt` 13 B. `coach-directories-national/samples/assoc-home/WV__wvssac.org.html` 680.6 KB on disk (136,475 wire).
- B: raw `www.wvssac.org` 21. `report.json`: `wv_wvssac` 274,397 B → **0 schools** (classifications page, role "files"); `wv_milesplit`. `extra/WV-arbiter.jsonl` 172.8 KB = **271 lines** — Arbiter org 4223 children (200/page) + AD rows, names/phone/enrollment, **no emails**; `WV-school-sites.jsonl` 700 B (1 row: Greenbrier East with `jhodge@k12.wv.us`).
- C: none today (arbiter README registers WVSSAC org 4223 = 269 schools).
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| `WV-arbiter.jsonl` (org 4223) | adapter `arbiter` (registered org) | 269 school AD contacts |
| `school-directory-wvssac.html` 266.3 KB + classifications PDF | fresh extraction | school list + classes/regions |
| `WV-school-sites.jsonl` (1 row) | reuse | school-site email lane seed |
| `cross-country`/`track` WVSSAC pages | fresh extraction | sport participation (not coaches) |
Estimated: 269 ADs; TF/XC 43 today; email 1 → the email lane is entirely school-site dependent.
### 4. True missing
- WV school-site staff directories (single held probe) — wvusd/k12.wv.us district sites for emails.
- WVSSAC per-school detail pages (directory page + classifications only).
- WVSSAC coach listings (association publishes none held).
### 5. Notes
Arbiter lane = AD-only names; WVSSAC bodies are rich (680 KB home) but parse as files/classifications; `wpjson-search-track` returned 2 B (empty).

---

## DE — Delaware
### 1. Matrix row
ccd 238 | any-coach 175 | TF/XC 58 | email 175 (74%) — best email coverage of the group.

### 2. Held evidence
- Registry: none.
- A: `state-assoc-midatlantic/samples/`: `diaa-home.html` 5.6 KB (Cloudflare block); `delaware-edu-diaa.html` 5.6 KB; `delaware-admincode-title14.html` 64.0 KB + `delaware-admincode-1001.shtml` 64.0 KB (regulation text); `de-milesplit-home.html` 106.3 KB; `de-milesplit-teams.html` 51.1 KB; `robots-education-delaware.txt` 311 B; `robots-diaa.txt` 0 B. `coach-directories-national/samples/assoc-home/DE__diaa.org.html` 403 (3,218 wire / 5,531). Raw `diaa-schools.rst7.rschooltoday.com` 12 responses.
- B: raw `diaa.finalforms.com` 6. `report.json`: `de_diaa` = **RuntimeError "offline and not cached: https://www.diaa.org/"** (coverage.md :63); `de_dragonfly` 112,667 B → **320 schools / 314 addresses**; `de_milesplit`. `extra/DE-dragonfly.jsonl` 319.6 KB; `DE-diaa-ad-directory.jsonl` 71.5 KB = **114 lines** — `education.delaware.gov/diaa/governance/organization/ad_directory/` AD rows with **emails** (sample `lamontz.hayman@redclay.k12.de.us`); `DE-school-sites.jsonl` 4.3 KB.
- C: none today.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| `DE-diaa-ad-directory.jsonl` (114 rows) | reuse | 114 schools AD name+email |
| DE DragonFly DIAA directory | adapter `coach_directories` (registered) | 320 schools / 314 addresses |
| `de-milesplit-teams.html` | adapter `milesplit` | team discovery |
Estimated: 320 w/address; 175 any-coach today; TF/XC 58 → coach-level gap (AD-dominated).
### 4. True missing
- DIAA member/school directory live (`diaa.org` Cloudflare-blocked; only AD directory held).
- School-site staff directories for TF/XC coaches (`DE-school-sites` 4.3 KB).
- `diaa.finalforms.com` full rosters (6 bodies only).
### 5. Notes
DE = AD-heavy: 74% email is AD coverage; dragonfly has `coaches: 0`; blocked diaa.org is why registry `de_diaa` errored.

---

## DC — District of Columbia
### 1. Matrix row
ccd 247 | any-coach 82 | TF/XC 34 | email 79 (32%).

### 2. Held evidence
- Registry: none.
- A: `state-assoc-midatlantic/samples/`: `dcsaasports-com-home.html` 726.9 KB; `dcsaasports-home.html` 159.1 KB; `dcsaasports-school-directory.html` 294.1 KB; `dcsaasports-about.html` 213.1 KB; `dcsaasports-otf.html` 321.4 KB; `dcsaasports-boys-xc.html` 318.3 KB; `dcsaa-home.html` 107.3 KB; `dcsaa-robots-404-check.txt` 109.3 KB; `robots-dcsaasports.txt` 13 B; `sitemap-osse-dc.xml` 556.3 KB; `dc-milesplit-teams.html` 51.8 KB. `coach-directories-national/samples/assoc-home/DC__dcsaasports.org.html` 159.1 KB? on-disk not listed here (wire 26,335 / 162,889); `DC__dcsaa.org.html` MISSING.
- B: raw `www.dcsaasports.com` 81 + `dcsaasports.com` 1 + `dcsaasports.rst7.rschooltoday.com` 2. `report.json`: `dc_dcsaasports` 301,245 B → **49 schools / 47 addresses**, AD name + email (sample `micheal.reid@k12.dc.gov`); `dc_dragonfly` 60,395 B → **112 schools / 103 addresses**; `dc_milesplit`. `extra/DC-dragonfly.jsonl` 81.6 KB; `DC-dcsaa.jsonl` 30.8 KB = **44 lines** — school-directory coach rows incl. XC/Track emails (Ballou: `Julius.West3@k12.dc.gov`); `DC-school-sites.jsonl` 27.9 KB.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| `DC-dcsaa.jsonl` (44 rows) | reuse | 44 schools w/ coach rows incl. sport emails |
| `dc_dcsaasports` directory (301,245 B) | fresh extraction (prototype `html` parser) | 49 schools AD+email |
| DC DragonFly DCSAA directory | adapter `coach_directories` (registered) | 112 schools / 103 addresses |
| `DC-school-sites.jsonl` 27.9 KB | reuse | school-site lane |
Estimated: 112 w/address; 44–49 schools with coach-level contacts.
### 4. True missing
- DCSAA member coaches for private/non-public schools (only 44 coach rows held).
- DC school sites beyond the 27.9 KB lane (OSSE sitemap 556 KB held only for discovery).
- `dcsaa.org` (old host) never captured — MISSING in corpus.
### 5. Notes
`dc_dcsaasports` = AD contacts (name+email); `DC-dcsaa.jsonl` adds sport coach rows; dragonfly = addresses only.

---

## RI — Rhode Island
### 1. Matrix row
ccd 316 | any-coach 55 | TF/XC 49 | email 0 (0%).

### 2. Held evidence
- Registry: none.
- A: `state-assoc-midatlantic/samples/`: `riil-home.html` 106.3 KB; `riil-school-list.aspx` 107.0 KB; `riil-xc-boys.aspx` 94.0 KB; `sitemap-riil.xml` 1.2 KB; `ri-milesplit-teams.html` 47.2 KB; `robots-riil.txt` 247 B. `coach-directories-national/samples/assoc-home/RI__riil.org.html` 106.2 KB on disk (24,863 wire).
- B: raw `riil.org` 7. `report.json`: `ri_riil` 111,222 B → **128 school pages, 0 coaches** (school list only); `ri_riil_directory` 201,040 B → **55 schools / 349 coaches / 258 TF-XC** (rich!); `ri_milesplit`. `extra/`: **no RI file exists** (confirmed against full `extra/` listing — no `RI-school-sites.jsonl`).
- C: none today.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| `ri_riil_directory` cache (201,040 B) | adapter `riil` (`registry/table/from_mshsl.rs`, SCHOOL_COACH_NAMES) | 55 schools / 349 coach rows / 258 TF-XC |
| `ri_riil` school pages (111,222 B) + `riil-school-list.aspx` | adapter reuse | 128-school list |
| ri.milesplit teams | adapter `milesplit` | team discovery |
Estimated: 55 → 128 schools enumerable; coach names 349; **emails 0**.
### 4. True missing
- RI school-site staff directories — no held lane at all (`extra/` lacks RI).
- RIIL per-school pages beyond the 128 list (directory has coaches for 55).
- Any email source for RI coaches (RIIL publishes names only; matrix says 0 emails).
### 5. Notes
RI = names-only lane; `riil` adapter maps to SCHOOL_COACH_NAMES (not contacts); today's taps: none.

---

## NH — New Hampshire
### 1. Matrix row
ccd 518 | any-coach 90 | TF/XC 61 | email 3 (1%).

### 2. Held evidence
- Registry: none.
- A: `state-assoc-midatlantic/samples/`: `nhiaa-home.html` 658.7 KB; `nhiaa-about-schools.html` 287.7 KB; `nhiaa-member-directory.html` 365.1 KB; `nhiaa-member-directory-p42.html` 354.0 KB; `nhiaa-boys-xc.html` 302.6 KB; `sitemap-nhiaa.xml` 47.1 KB; `nh-milesplit-teams.html` 58.9 KB; `robots-nhiaa.txt` 13 B. `coach-directories-national/samples/assoc-home/NH__nhiaa.org.html` 658.7 KB on disk (133,203 wire).
- B: raw `www.nhiaa.org` 1. `report.json`: `nh_nhiaa` 293,677 B → **0 schools parsed**; `nh_milesplit`. `extra/NH-arbiter.jsonl` 83.8 KB = **90 lines** — Arbiter org 2132 (89 schools; XC head coaches per school, names only, no emails — sample Alvirne/Bedford/Berlin); `NH-school-sites.jsonl` 3.5 KB.
- C: none today (arbiter README: NHIAA org 2132 = 89; test `the_schools_page_carries_all_89_nhiaa_schools`).
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| `NH-arbiter.jsonl` (org 2132) | adapter `arbiter` (registered) | 89 schools with XC coach names |
| `nhiaa-member-directory` bodies (365 KB + p42) | fresh extraction (client-rendered risk) | school list |
| `NH-school-sites.jsonl` 3.5 KB | reuse | school-site lane |
Estimated: 89 reachable from arbiter; emails remain 3.
### 4. True missing
- NH school-site staff directories for emails (3.5 KB lane; COACH_CONTACTS_REFUSAL notes NH is client-rendered).
- NHIAA member directory route itself (page held, parser yields 0 — needs a browser/JS route or fresh parser).
- NHIAA coach emails (association doesn't publish; arbiter has none).
### 5. Notes
Arbiter rows = XC-only names; nhiaa.org 658 KB home parsed to 0 schools; member-directory p42 exists but same parse gap.

---

## VT — Vermont
### 1. Matrix row
ccd 306 | any-coach 8 | TF/XC 3 | email 4 (1%) — lowest any-coach of the group.

### 2. Held evidence
- Registry: SRC-082 HELD+ADAPTER (VPA `vpa-divisional-alignments.html` 120.6 KB; `vpa-home.html` 183.6 KB), SRC-083/084/085 DERIVED (AOE), SRC-090 HELD+ADAPTER (`vt-milesplit-teams.html` 61.8 KB), SRC-094 DERIVED (NEPSAC), SRC-095 composite (NEPSTA) DERIVED.
- A: `state-assoc-midatlantic/samples/`: `vpa-home.html` 183.6 KB; `vpa-divisional-alignments.html` 120.6 KB; `vpa-tournaments.html` 137.3 KB; `vpa-tournament-pairings.html` 142.8 KB; `vpa-scoreboard.html` 125.2 KB; `sitemap-vpa.xml` 1.0 KB; `sitemap-vpa-page.xml` 27.9 KB; `vt-milesplit-teams.html` 61.8 KB; `robots-vpa.txt` 187 B. `coach-directories-national/samples/assoc-home/VT__vpaonline.org.html` 183.6 KB on disk (44,501 wire); `samples/dir/VT__membership.html` (referenced in `private_assoc/README.md`).
- B: raw `vpaonline.org` 12 + `www.vpaonline.org` 8. `report.json`: `vt_vpa` 131,115 B → **6 garbage rows** (nav cells: "Streaming, NFHS" etc.); `vt_milesplit`. `extra/VT-school-sites.jsonl` 4.8 KB.
- C: none today.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| `vpa-home.html` + membership body (188,000 B) | fresh extraction (VPA member list) | member schools (count unverified — parse currently yields nav) |
| `VT-school-sites.jsonl` 4.8 KB | reuse | school-site lane |
| vt.milesplit teams | adapter `milesplit` | team discovery |
Estimated: VPA membership size unverified from held bodies; 8 any-coach today → VT is essentially fresh-start.
### 4. True missing
- VPA member-school directory (membership route held but parse failed; `/athletics/` route not individually verified).
- VT school district staff directories for emails (4.8 KB lane).
- NEPSAC/NEPSTA private-school lanes (DERIVED; `nepsac.org` not held).
### 5. Notes
VT's vpaonline bodies are tournament/alignment-heavy; the only coach-ish lane is school-sites at 4.8 KB; SRC-082 class says "host-level; exact /athletics/ route not individually verified".

---

## ME — Maine
### 1. Matrix row
ccd 598 | any-coach 150 | TF/XC 81 | email 3 (1%).

### 2. Held evidence
- Registry: none.
- A: `state-assoc-midatlantic/samples/`: `mpa-home.html` 53.7 KB; `mpa-school-list.aspx` 98.3 KB; `mpa-xc-boys.aspx` 66.5 KB; `mpa-outdoor-track.aspx` 52.7 KB; `sitemap-mpa.xml` 1.2 KB; `mpa-robots.txt` 247 B; `mpaonline-home.html` 626 B; `me-milesplit-teams.html` 74.7 KB. `coach-directories-national/samples/assoc-home/ME__mpa.cc.html` **MISSING (0 wire)**, `ME__maineprincipalsassociation.com.html` **MISSING (0 wire)**.
- B: raw `www.mpa.cc` 14. `report.json`: `me_mpa` 102,465 B → **152 schools** (list); `me_mpa_directory` 288,570 B → **147 schools / 472 coaches / 310 TF-XC**; `me_milesplit`. `extra/ME-school-sites.jsonl` 3.9 KB.
- C: none today.
### 3. Fill plan
| Held body | Route | Reach |
|---|---|---|
| `me_mpa_directory` cache (288,570 B) | adapter `mpa` (`from_mshsl.rs`, SCHOOL_COACH_NAMES) + fixtures | 147 schools / 472 coach rows / 310 TF-XC |
| `me_mpa` school list (102,465 B) | adapter reuse | 152 schools |
| `ME-school-sites.jsonl` 3.9 KB | reuse | school-site lane |
Estimated: 147–152 schools enumerable; coach names 472 (310 TF/XC); emails 3.
### 4. True missing
- ME school-site staff directories for emails (3.9 KB lane; MPA publishes names only).
- mpa.cc live bodies (corpus assoc-home MISSING — prototype cache 288 KB is the body).
- Maine private schools outside MPA (association list needed).
### 5. Notes
ME = names-heavy/email-poor; the `mpa_directory` cache is the anchor; mpa.cc was unreachable at corpus time but data exists in the prototype cache.

---

## Cross-state roll-up

**Schools reachable per state from held bodies (upper bound, not de-duplicated across lanes):**
ND 169 (NDHSAA) / 548 dragonfly rows; SD ~560 Bound staff pages; MT 358 (dragonfly) + 215 arbiter ADs; ID 461 (dragonfly) + 177 AD directory;
WY 93 (dragonfly) + ADMS PDF; UT 162 (UHSAA); NV ~130 (finalforms p1–9); NM 751 (dragonfly) + 162 (nmact); OR 298 (school profiles); KS 526 (KSHSAA API);
WV 269 (arbiter ADs); DE 320 (dragonfly) + 114 AD emails; DC 112 (dragonfly) + 49/44 (directory lanes); RI 55 (directory) → 128 (school list);
NH 89 (arbiter); VT unknown (membership parse failed); ME 147–152.

**True-missing sites/sources (names only), grouped:**
1. School-site staff directories (emails) — needed for ND, SD, MT, ID, WY, UT, NV, NM, KS, WV, DE, DC, RI, NH, VT, ME. Held lane exists per state at `census-prototype/out/extra/<ST>-school-sites.jsonl` (sizes 700 B–27.9 KB; **no RI file**).
2. State education directories — USBE `schools.utah.gov` (UT), NV DOE (SRC-176), `edu.wyoming.gov` (WY, 6 bodies), VT AOE (SRC-083/84/85), KSDE (partial), NM PED (absent), SD DOE (held via SRC-223).
3. Association per-school coach pages — UHSAA profile pages (UT), NIAA/prestosports (NV), WHSAA ADMS (held), RIIL (names only), NHIAA member directory (parse gap), VPA membership (parse gap), WVSSAC school pages, KSHSAA b–z (verify), SDHSAA member directory (gobound), IdHSAA coach pages, MHSA staff pages, DIAA member directory (blocked), DCSAA non-public coaches.
4. Coaches-association rosters — KCCTFCA (KS, membership missing), SDCCTFCA (held 2×), OACA (OR, not held), UCCTCA (UT), NDHSAA co-ops.
5. Platform lanes for enrichment — DragonFly (registered/held), Arbiter orgs NH 2132 / MT 4497 / WV 4223 (held), Bound (SD, refused live), FinalForms (WY/NV/NM/DE partial), MileSplit team directories (all, discovery only).

**Notes on body types (repeat pattern):**
- AD-only bodies (names/emails, no sport coaches): DC `dc_dcsaasports`, DE `diaa-ad-directory` (114), WV/MT arbiter rows, KS KSHSAA directory, ID IdHSAA directory.
- TF/XC-only bodies: SD Bound staff pages, NH arbiter, WV/MT arbiter sport rows, OR school profiles (mixed), NV S. NV CSVs.
- Bodies without emails: NDHSAA detail pages (ND-schools.jsonl), MPA directory, RIIL directory, UHSAA directory, WY finalforms/ADMS, MT/ARB, NM dragonfly.
- Name/URL-only sources: MileSplit team indexes (2 raw responses per state site; `www.milesplit.com` 47,648 total), AthleticNET discovery, `teams-count-sweep.tsv`.
- Empty/refused held bodies: `niaa-coaches.html` 0 B, `niaa-sports-track*.html` 0 B, `wpjson-search-track-wvssac.json` 2 B, `ME__mpa.cc.html` MISSING, `DE__diaa.org` 403, `dcsaa.org` MISSING, `de_diaa` RuntimeError.

Method/limitations: all counts copied from the cited artifacts (paths above); no network, no builds, no stores touched; tap artifacts are scratch and excluded from the matrix numbers; "estimated reachable" figures are held-count upper bounds, not deduplicated joins.
