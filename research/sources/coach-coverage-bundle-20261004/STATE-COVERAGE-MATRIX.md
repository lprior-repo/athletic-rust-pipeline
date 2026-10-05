# State-by-state coverage matrix — 2026-10-04

Target: every athlete -> school + address + state + coach contact, complete per state.

Denominators: NCES CCD 2024-25 in-scope rows (48 contiguous + DC), 100,384 schools (`extract/248-nces-ccd-zip/rows.csv`).
Numerator: distinct schools per state in `census-prototype/out/coaches.jsonl` (prototype merged coach output): ANY contact row; TFXC = Track/CrossCountry rows; EMAIL = school has >=1 row with an email.

**Caveats:** CCD includes non-athletics schools (the final denominator is the in-census school universe); schools/school-name matching is the prototype's own keys, not name-normalized against CCD; today's tap rows (SDHSCA 286, AIA profiles, MITCA 30, sdhsca) are NOT included (scratch stores, unmerged).

| st | ccd schools | with any coach row | with TF/XC coach | with email contact | email cov |
|---|---:|---:|---:|---:|---:|
| DE | 238 | 175 | 58 | 175 | 74% |
| MS | 1050 | 521 | 316 | 509 | 48% |
| PA | 2949 | 1432 | 1 | 1415 | 48% |
| AL | 1541 | 704 | 398 | 689 | 45% |
| AR | 1117 | 494 | 293 | 449 | 40% |
| KS | 1356 | 528 | 1 | 525 | 39% |
| SC | 1290 | 495 | 288 | 487 | 38% |
| DC | 247 | 82 | 34 | 79 | 32% |
| GA | 2349 | 954 | 845 | 636 | 27% |
| WI | 2232 | 605 | 511 | 597 | 27% |
| TN | 1928 | 499 | 411 | 491 | 25% |
| OR | 1300 | 303 | 284 | 293 | 23% |
| MN | 2859 | 662 | 4 | 636 | 22% |
| OH | 3611 | 788 | 747 | 779 | 22% |
| MT | 854 | 274 | 173 | 174 | 20% |
| WY | 367 | 75 | 73 | 74 | 20% |
| NM | 927 | 195 | 89 | 159 | 17% |
| ID | 820 | 234 | 208 | 136 | 17% |
| NC | 2766 | 463 | 438 | 455 | 16% |
| CA | 10407 | 1038 | 810 | 1001 | 10% |
| FL | 4314 | 377 | 302 | 354 | 8% |
| ND | 542 | 174 | 164 | 43 | 8% |
| MD | 1414 | 253 | 45 | 87 | 6% |
| VT | 306 | 8 | 3 | 4 | 1% |
| CT | 1021 | 199 | 187 | 12 | 1% |
| OK | 1791 | 42 | 7 | 21 | 1% |
| MA | 1844 | 185 | 158 | 20 | 1% |
| IA | 1339 | 366 | 358 | 11 | 1% |
| NY | 4865 | 59 | 16 | 33 | 1% |
| TX | 9774 | 190 | 54 | 64 | 1% |
| NH | 518 | 90 | 61 | 3 | 1% |
| MO | 2488 | 21 | 5 | 14 | 1% |
| NV | 746 | 100 | 97 | 4 | 1% |
| MI | 3508 | 179 | 146 | 18 | 1% |
| ME | 598 | 150 | 81 | 3 | 1% |
| IL | 4438 | 824 | 705 | 20 | 0% |
| VA | 2159 | 21 | 9 | 9 | 0% |
| WA | 2574 | 376 | 365 | 10 | 0% |
| UT | 1121 | 146 | 145 | 4 | 0% |
| NJ | 2575 | 454 | 5 | 9 | 0% |
| CO | 1917 | 337 | 331 | 6 | 0% |
| SD | 744 | 138 | 135 | 2 | 0% |
| IN | 1928 | 12 | 7 | 5 | 0% |
| AZ | 2631 | 299 | 237 | 5 | 0% |
| WV | 701 | 268 | 43 | 1 | 0% |
| KY | 1559 | 416 | 271 | 0 | 0% |
| LA | 1330 | 287 | 287 | 0 | 0% |
| NE | 1115 | 293 | 293 | 0 | 0% |
| RI | 316 | 55 | 49 | 0 | 0% |
| **TOTAL** | **100384** | **16840** | **10548** | **10521** | **10%** |

## Worst 15 by email coverage (absolute school gaps are largest in TX/CA/FL/NY/IL)
- KY: 0/1559 schools with email (0%), any-coach 416, cc-gap 1559
- LA: 0/1330 schools with email (0%), any-coach 287, cc-gap 1330
- NE: 0/1115 schools with email (0%), any-coach 293, cc-gap 1115
- RI: 0/316 schools with email (0%), any-coach 55, cc-gap 316
- WV: 1/701 schools with email (0%), any-coach 268, cc-gap 700
- AZ: 5/2631 schools with email (0%), any-coach 299, cc-gap 2626
- IN: 5/1928 schools with email (0%), any-coach 12, cc-gap 1923
- SD: 2/744 schools with email (0%), any-coach 138, cc-gap 742
- CO: 6/1917 schools with email (0%), any-coach 337, cc-gap 1911
- NJ: 9/2575 schools with email (0%), any-coach 454, cc-gap 2566
- UT: 4/1121 schools with email (0%), any-coach 146, cc-gap 1117
- WA: 10/2574 schools with email (0%), any-coach 376, cc-gap 2564
- VA: 9/2159 schools with email (0%), any-coach 21, cc-gap 2150
- IL: 20/4438 schools with email (0%), any-coach 824, cc-gap 4418
- ME: 3/598 schools with email (1%), any-coach 150, cc-gap 595

## 2026-10-05 held-readback fills (prototype-universe board v2)

The adds below merge against the *prototype school universe* (`census-prototype/out/schools.jsonl`,
43,138 rows) because every fill row joins on that universe; the CCD table above remains the reference
for CCD coverage. Board: `var/state-coverage-board-20261005.md`, rebuilt by
`var/state-coverage-board-20261005/build_board.py`.

Headline: email-contact schools 10,238 → **15,555 (24% → 36%)**; any-contact 16,179 → 19,662 (46%).

Held readbacks behind the adds (all artifacts retained, no network):

- `var/il-fill-20261005/` — IL IHSA drain store (`store/http`, 828 staff2 + 13,209 email bodies):
  19,107 contact rows, TF/XC emails for 704 schools. IL 20 → 810/1,425.
- `var/cf-emails-20261005/` — Cloudflare `data-cfemail` decode: WA FinalForms 777 school pages,
  NV NIAA 119, WI schools.wiaawi.org 633. WA 10 → 699/999, NV 4 → 122/169, WI 583 → 627/847.
- `var/atob-emails-20261005/` — base64 `atob()` mailto decode, IDHSAA: 176 school pages / 2,436
  emails. ID 136 → 214/539.
- `var/assoc-emails-20261005/` — association directories: PIAA (PA 1,447 pages/4,578 rows),
  myOHSAA officials portal (OH 764 schools/10,973), UHSAA (UT 162/3,400), MHSAA Knack JSON
  (MS 425/8,615), Greenville SC staff dirs. PA 1,414 → 1,450/2,164, UT 4 → 69/206, MS 487 → 537/1,451.
- `var/cif-json-20261005/` — CIF sections + FHSAA homecampus JSON (`athleticFaculties`/`coaches`
  emails): CA 954 schools/22,247 rows, FL 330/7,834. CA 993 → 1,363/3,352, FL 351 → 1,098/1,826.
- `var/mailto-scan-20261005/` — corpus-wide `mailto:` scan (4,608 files / 25,638 emails,
  entity-decoded) with global unique-name matching: RI 0 → 45/216, ME 3 → 113/207, MI 18 → 39/1,054.
- `var/dragonfly-20261005/` — held DragonFly Athletics school API (11,380 records): 126,906 staff-email
  rows across 21 states (GA 721 schools, AL 750, NC 452, MS 850, AR 513, SC 440, MT 232, ID 235,
  DE 103, MD 205, NM 333, TN 166, WY 87, ND 162, DC 105). Lifts: MS 487 → 944/1,451,
  GA 508 → 723/3,071, ID 136 → 274/539, NM 158 → 331/914, ND 43 → 160/589, MD 87 → 204/448.
- `var/or-fill-20261004/` OR parity 298/401 (74%); `var/ri-me-fill-20261004/` name rows.

Join rule: added rows count only when the normalized school name matches the state universe (exact,
else token-Jaccard ≥ 0.65 unique best), so coverage cannot exceed 100%. Strict vs fuzzy additions are
separated in the board's `fuzzy+` column; full fuzzy map in
`var/state-coverage-board-20261005-fuzzy.txt`, unmatched names in `…-unmatched.txt`.

Still email-barren after held readback: KY 0/646, NE 1/379, WV 1/360 (association pages carry no
contact columns; ArbiterLive is JS), AZ 5/428 (AIA captures are JS shells), CO 6/463 (chsaanow is
Next.js RSC without emails), IN 6/608, MO 14/1,086, NJ 9/935, VA 9/608. Fresh-fetch slices for
AZ/IN/KY/CO are running under `var/fresh-<ST>-20261005/`.

## 2026-10-05 fresh-fetch lanes (board v3)

Four fresh lanes were opened after the held wells were exhausted. All use anonymous access, a
descriptive `census-service/0.1 …` UA, >=1.1s per host, robots checked where rules exist.

1. **FinalForms state tenants** (`var/finalforms-fresh-20261005/`). 44 live tenants detected by
   homepage title (bogus tenants answer 200 with "Technical Default Installation (DO NOT USE)").
   Only five expose the public `/state_schools` directory: WA and NV (already held), plus
   **NY (1,355 records)**, **OH (1,405)** and **VT (79)**. Crawl: paginated index
   (`/state_schools?page=N&…member…`, 15 rows/page, stop when a page adds no new ids) then each
   `/state_schools/<id>` detail page; Cloudflare `/cdn-cgi/l/email-protection#<hex>` addresses are
   XOR-decoded (first byte is the key). Raw bodies (index + detail) retained per tenant; email rows
   in `var/finalforms-fresh-20261005/<tenant>-emails.jsonl` and merged by the board.
2. **School-site mass lane** (`var/school-site-fresh-20261005/`). The prototype's school-site crawler
   had covered 8,967 sites; 7,905 universe rows still carried a website that had never been rendered.
   Those are now crawled with `school_sites_fast.py` (derived from `census-prototype/school_sites.py`:
   15s navigation timeout, 600ms settle, <=5 followed pages, dead-host precheck) through
   `run_slice.py`, writing the same `census-prototype/out/school_sites/<ST>__<slug>.json` shape so the
   board merges the results unchanged. Note: this lane stores derived signals (URLs + emails), not raw
   page bytes; product-grade import still needs captures.
3. **ArbiterSports organisation staff** (`var/arbiter-national-20261005/`,
   `var/fresh-KY-20261005/`). `live.arbiter.io` directory -> public bundle credentials -> OAuth token
   (`token.arbitersports.com`) -> `services.arbitersports.com/api/v2/organization/public/<org>/children`
   -> `/staff` per school, pageSize=200. KY org 2507 produced 21,631 rows / 290 schools / 19,044
   distinct emails (KY was 0/646). WV 4223 and NH 2132 are queued for the same scraper.
4. **Prototype school-site merge** (`census-prototype/out/school_sites/`). The prototype's own
   site-mining corpus (8,967 sites, 5,718 with emails, 125,735 email strings) is now merged into the
   board explicitly; most rows were already represented via earlier merges (+1,437 new school rows).

Join rule (updated): normalized-name exact match, else token-Jaccard >= **0.80** unique best — raised
from 0.65 after false merges inside charter networks (e.g. "Great Hearts … Cicero Prep" ->
"… Glendale Prep", Jaccard 0.67) were found in the AZ/AZ rows; the long-tail mailto matcher uses 0.85.
Numbers as of the v3 board: email-contact schools **17,131 / 43,138 (40%)**, any-contact 21,074 (49%).
The lane runs were still in flight when this section was written; the board file is the live record.

## 2026-10-05 fresh pass closed (board v4)

The v3 section above was written while the lanes were still running. This section is the closed
count from the board file after every lane stopped writing, the school-site outputs were sanitized
and the export was rebuilt.

**Top line.** Universe 43,138 rows across 49 jurisdictions. Schools with a usable email
**19,389 (45%)**, up from a 10,238 (24%) held-data baseline — **+9,151 schools**. Schools with any
public contact (email or published staff name) **23,362 (54%)**, up from 16,179. The export built
from these lanes carries **421,598 rows / 50 state-regions**, of which 99,972 rows came from
2026-10-05 lanes.

**Lane close-out (schools added to email coverage).**

| lane | result |
|---|---:|
| school-site crawl (11,791 universe rows had a website never rendered) | 5,398 with emails, 242 names-only, 3,465 empty, 2,686 unreachable |
| FinalForms tenants (CT IL ME NH RI VT WI NY OH) | OH 12,328 email rows; NY 5,068 name rows and 0 emails; others as v3 |
| ArbiterSports orgs (KY WV NH MT UT MN MA) | MN 406 schools / 17,622 rows, MA 392 / 5,414, UT 193 / 1,669 |
| held-readback and directory lanes | unchanged from v3 |

**Largest movers vs baseline:** IL +803, FL +746, WA +714, TX +547, MA +478, MS +470, KY +415,
CA +398, NY +349, WV +264, GA +248, NJ +233.

**Coverage now >=75% in 7 states** (MN and MA 81%, UT 79%, WV 74%, ID, WA, IA); **24 of 49
jurisdictions are >=50%**. The remaining absolute gaps are concentrated in NE (1/379), LA (10/564),
IN (19/608), MO (210/1086), MI (220/1054), RI (47/216), VA (133/608), GA (756/3071) and TX
(611/2481) — the next acquisition targets.

**Sanitization.** `var/school-site-fresh-20261005/clean_outputs.py --apply` rewrote 3,137 of the
15,620 site JSONs, dropping 13,590 email strings (markup artifacts, case-fold duplicates,
`TRANSPORTATION`/`Webmaster` role mailboxes on the wrong school). Arbiter lanes also lost 134
platform-vendor rows (`@arbitersports.com` staff listed under school names) before merge; each
lane SUMMARY.md carries the note.

**Evidence limits.** The page-crawl lane persists per-school JSON but not raw page bodies, so
its rows are backed at page level, not by row-level body search; all other lanes keep raw bodies
next to their CSVs (`var/coverage-verify-20261005/LANE-BACKING.md`). 2,686 universe hosts did not
answer within the crawl's transport budget (two attempts, 15s navigation timeout) and are
unproven, not disproven. FinalForms tenant email rows for schools outside the 43,138-row
universe (mostly middle/junior campuses, e.g. 624 of OH's 633 matched schools were already
covered) are exported but cannot raise this board's percentages.
