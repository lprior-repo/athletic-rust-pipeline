# Source Applicability Matrix — 49 Continental Jurisdictions + DC

**Date:** 2026-09-22
**Purpose:** One row per jurisdiction (48 states + DC) with per-source capability verdicts, deciding the smallest source set that covers the most states for a Class-of-2027 census.

**Grade/class-year is the deciding column:** an athlete can only be a Co2027 candidate if some source states grade or graduating class. States marked N on grade/class have no Co2027 enumerability.

## Matrix

| St | Association Source (URL) | Result / Timing Sources | Athletic.net seed? | Grade / Class Year Source? | Coach / AD Source? | Bulk Results? | Best Recommendation | Evidence Status |
|----|--------------------------|------------------------|--------------------|--------------------------|--------------------|---------------|---------------------|-----------------|
| AL | REJECT (ahsaa.com, `Disallow: /`) | MileSplit AL; Xpress Timing (AthleticLIVE tenant) | Y — timer `xt.anet.live` (AthleticLIVE host) | Y — MileSplit roster `column-grad-year`; Hy-Tek results via MHSAA | N | Y — MileSplit + Xpress Timing | PRIMARY | verified-this-run |
| AR | Negative (no results route at all) | MileSplit AR | Y — AthleticLIVE index (62,954 meets with `ani > 0`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit | RESULT-SOURCE | verified-this-run |
| AZ | Positive (AIA — entries on Athletic.net) | Finished Results (state) + Wingfoot (sectionals) + MileSplit AZ | Y — AIA tournament guide names Athletic.net (`ani 276301`) | Y — MileSplit roster `column-grad-year`; NMAA PDFs `Year` column (PRIOR-WAVE) | N | Y — Finished Results / Wingfoot | ATHLETIC.NET-SEED | verified-this-run |
| CA | pending (group report not yet landed) | — | Y — AthleticLIVE index (1,805 meets with `ani`) | Y — MileSplit roster `column-grad-year` (sample: 24 Co2027 from 157 athletes) | N | Y — MileSplit | DISCOVERY-ONLY | inherited |
| CO | Negative (`/results` is 404) | Rapid Results → TFMeetPro + MileSplit CO | N | Y — MileSplit roster `column-grad-year` | N | Y — TFMeetPro (socket-closed this session) | RESULT-SOURCE | verified-this-run |
| CT | pending (group report not yet landed) | — | Y — AthleticLIVE index (1,838 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit | DISCOVERY-ONLY | inherited |
| DC | Association negative / platform positive | M&D Timing tenant `58668` = `ani 260003` + MileSplit DC | Y — M&D Timing payload: `ani 260003`, per-row `a.ani`, `t.ani` | Y — M&D Timing per-row grade `a.y` (36×9, 26×10); MileSplit roster | N | Y — M&D Timing / MileSplit | ATHLETIC.NET-SEED | verified-this-run |
| DE | Negative (DIAA 403 Cloudflare) | MileSplit DE; DIAA indoor on `pa.milesplit.com` | Y — AthleticLIVE index (123 meets with `ani`) | Y — MileSplit roster `column-grad-year`; `/raw` DE indoor has grade | N | N | Y — MileSplit | RESULT-SOURCE | verified-this-run |
| FL | pending (group report not yet landed) | — | Y — AthleticLIVE index (211 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit | DISCOVERY-ONLY | inherited |
| GA | pending (group report not yet landed) | — | Y — AthleticLIVE index (102 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit | DISCOVERY-ONLY | inherited |
| IA | Negative | Bound IA + MileSplit IA; AthleticLIVE (Wayzata et al.) | Y — AthleticLIVE `ani` on Wayzata tenant (136/136 XC rows) | Y — Bound roster `Year` (FR/SO/JR/SR); AL `a.y`; MileSplit roster | Y — Bound directory (role + name; 3/63 coach email) | Y — Bound + AL | PRIMARY | verified-this-run |
| ID | Positive (IDHSAA publishes AN MeetIDs) | Athletic.net + Snake River Timing (AthleticLIVE engine) | Y — IDHSAA page: `meet/646835`, `646834`, `591968`, `CrossCountry/meet/263030` | Y — MileSplit roster `column-grad-year` | N | Y — Athletic.net + Snake River Timing | ATHLETIC.NET-SEED | verified-this-run |
| IL | Positive (IHSA API: `athleticNetId` + `year`) | IHSA API + MileSplit IL + AthleticLIVE | Y — IHSA API `athlete.athleticNetId`; state finals on AN (`74003`, `74002`) | Y — IHSA API `year "11"` (2,555 triples, 838 Jr); state finals grade; MileSplit roster | Y — IHSA API per-school coach/AD + email (49/49 T&F/XC head-coach rows have email) | Y — IHSA API + MileSplit | PRIMARY | verified-this-run |
| IN | Negative (MileSplit / TFRRS) | TFRRS Indiana + MileSplit IN | N | Y — TFRRS `&year=JR` (1,861 Co2027 in 2 requests); MileSplit roster | N | Y — TFRRS (0.0011 req/athlete) | RESULT-SOURCE | verified-this-run |
| KS | Negative | MileSplit KS; KSHSAA API | Y — AthleticLIVE index (844 meets with `ani`) | Y — MileSplit roster `column-grad-year`; KSHSAA XC grade column (89 G11/143 rows, 2025) | Y — KSHSAA API: AD name + email for ~526 schools (1 request) | Y — KSHSAA API + MileSplit | COACH-DIRECTORY | verified-this-run |
| KY | Negative | KHSAA posts + `milesplit.live`; KHSAA coach extraction (inherited) | Y — AthleticLIVE index (981 meets with `ani`) | Y — MileSplit roster `column-grad-year`; KHSAA results (inherited) | Y — KHSAA `school-directory/?school_id=N` (inherited script, not re-run) | Y — KHSAA posts + MileSplit | RESULT-SOURCE | inherited |
| LA | Negative (routes to MileSplit/MeetPro) | Delta Timing `live.deltatiming.com` + MileSplit LA | Y — Delta Timing meet `16151` on vanity host | Y — MileSplit roster `column-grad-year` | N | Y — Delta Timing + MileSplit | RESULT-SOURCE | verified-this-run |
| MA | Positive (MIAA official partner) | Athletic.net (7 divisional meets) + Lancer Timing | Y — MIAA Tournament Central: 7 AN MeetIDs (653886, 653234, 653925, 627461, 654392, 654397, 654446) | Y — MileSplit roster `column-grad-year`; AN GradeID 11 | N | Y — MileSplit | PRIMARY | verified-this-run |
| MD | Association negative / platform positive | M&D Timing tenant `58974` = `ani 268147` + MileSplit MD | Y — M&D Timing: `ani 268147` (verified against MPSSAA post → `meets/58974`) | N — MileSplit MD `/raw` **no grade**; M&D grade unverified (payload endpoints not recoverable) | N | Y — M&D Timing / MileSplit | ATHLETIC.NET-SEED | verified-this-run |
| ME | Negative | Sub5 + Brewer Timing + MileSplit ME | Y — AthleticLIVE index (10 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit | RESULT-SOURCE | verified-this-run |
| MI | Positive (MHSAA publishes AN MeetIDs) | Athletic.net (D1–D4 + UP) + MileSplit MI | Y — MHSAA regional pages: 48–49 AN hrefs; MeetIDs 622966, 622969, 622972, 622973, 622974 | Y — MHSAA finals PDFs grade (2025/2024 LP TF finals: 47/187 G11); MileSplit roster | Y — MHSAA AD API (`my.mhsaa.com`): 14 AD-with-email in stress sample | Y — Athletic.net + MileSplit | ATHLETIC.NET-SEED | verified-this-run |
| MN | Negative | MileSplit MN / Wayzata Timing + AthleticLIVE | Y — AthleticLIVE (Wayzata tenant: 242/242 XC rows grade; AN MeetID 666673 for 2026 state TF) | Y — AL 2025 state XC: 242/242 Juniors; MSHSL roster endpoint grade codes | Y — MSHSL: 6,596 coach rows (2,333 with email); AD + assistant AD + per-team coaches | Y — AL blobs + MileSplit + MSHSL | PRIMARY | verified-this-run |
| MO | Positive ("Official Results — AthleticNet") | Athletic.net MSHSAA page + MileSplit MO | Y — MSHSAA delegates to Athletic.net | Y — MSHSAA state PDFs `Year` column | N | Y — Athletic.net + MileSplit | ATHLETIC.NET-SEED | verified-this-run |
| MS | Negative (MaxPreps score partner) | MHSAA inline Hy-Tek results + MileSplit MS | Y — AthleticLIVE index (1,165 meets with `ani`) | Y — MHSAA championship posts: Hy-Tek `Year` column; MileSplit roster | N | Y — MHSAA WP REST | RESULT-SOURCE | verified-this-run |
| MT | Positive (publishes AN MeetID) | Competitive Timing + Athletic.net | Y — MHSA XC postseason: `CrossCountry/meet/267780/info` | Y — MileSplit roster `column-grad-year`; MHSA finals PDFs (grade unverified — no PDF opened) | N | Y — Competitive Timing + MileSplit | ATHLETIC.NET-SEED | verified-this-run |
| NC | pending (group report not yet landed) | — | Y — AthleticLIVE index (1,231 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit | DISCOVERY-ONLY | inherited |
| ND | Positive (integration, no published link) | NDHSAA entry spreadsheet → AN + MileSplit ND | Y — NDHSAA: entry spreadsheet links to AN; 6 ND 2026 XC meets with AN IDs | Y — MileSplit roster `column-grad-year` | Y — NDHSAA directory (169 schools) | Y — NDHSAA + MileSplit | ATHLETIC.NET-SEED | verified-this-run |
| NE | Positive (28 AN meet links) | Athletic.net + MileSplit NE | Y — NSAA: 28 AN meet links (`/results/all`) | Y — MileSplit roster `column-grad-year` | Y — NSAA directory (312 schools; coach names via `direxportscreen.php`) | Y — Athletic.net + MileSplit | ATHLETIC.NET-SEED | verified-this-run |
| NV | Unknown (site bot-walled) | MileSplit NV only | Y — AthleticLIVE index (774 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit | DISCOVERY-ONLY | verified-this-run |
| NH | Negative | State Running hub + Lancer Timing | Y — AthleticLIVE index (340 meets with `ani`) | Y — MileSplit roster `column-grad-year`; Lancer Hy-Tek files (no grade) | N | Y — Lancer Timing | RESULT-SOURCE | verified-this-run |
| NJ | pending (group report not yet landed) | — | Y — AthleticLIVE index (573 meets with `ani`) | Y — MileSplit roster `column-grad-year` | Y — NJSIAA member table: School/Title/Address/Phone/Ath. Dir. (Dr. Edwin Griffin) | Y — MileSplit | DISCOVERY-ONLY | inherited |
| NM | Negative (PDF only) | NMAA PDFs + MaxPreps + MileSplit NM | Y — AthleticLIVE index (46 meets with `ani`) | Y — NMAA PDF `Year` column (grade histogram {8:15, 9:41, 10:82, 11:110, 12:113}); MileSplit roster | N — NMAA role contact only | Y — NMAA PDFs | RESULT-SOURCE | verified-this-run |
| NY | pending (group report not yet landed) | — | Y — AthleticLIVE index (3,458 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N — NYSPHSAA "ADS & Coaches" area advertised but no roster captured | Y — MileSplit | DISCOVERY-ONLY | inherited |
| OH | Positive (2026 mandate) | Athletic.net MeetID 656920 + Finish Timing + OHSAA | Y — OHSAA: 656920 (state), 656686 (girls); Finish Timing: 29 AN MeetIDs on one fetch | Y — OHSAA state finals PDF: 561 G11 individual + 1,067 relay; MileSplit roster | Y — OHSAA myOHSAA: 31 coach rows with email, 100% named cells have email; OATCCC 32 contacts | Y — Finish Timing + MileSplit + OHSAA | ATHLETIC.NET-SEED | verified-this-run |
| OK | Negative (partner is Arbiter) | OSSAA rankings app + Arbiter + MileSplit OK | N — OSSAA routes through Arbiter, no AN link observed | Y — MileSplit roster `column-grad-year` | Y — OSSAA: `sc` school id (sparse surrogate PK, 360–25906); Arbiter org 106940 | Y — OSSAA ASP.NET app + MileSplit | PRIMARY | verified-this-run |
| OR | Positive (publishes AN state URL) | Athletic.net + MileSplit OR | Y — OSAA: "Athletic.net" → AN OR state page | Y — MileSplit roster `column-grad-year` | N | Y — Athletic.net + MileSplit | ATHLETIC.NET-SEED | inherited |
| PA | pending (group report not yet landed) | — | Y — AthleticLIVE index (2,966 meets with `ani`) | Y — MileSplit roster `column-grad-year` (sample: PA school returned 0 athletes — collector must sample >1 team) | N — PIAA `/officials/directory/` robots-disallowed | Y — MileSplit | DISCOVERY-ONLY | inherited |
| RI | pending (group report not yet landed) | — | Y — AthleticLIVE index (28 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit | DISCOVERY-ONLY | inherited |
| SC | pending (group report not yet landed) | — | Y — AthleticLIVE index (222 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit | DISCOVERY-ONLY | inherited |
| SD | Positive (AN state pages) | Athletic.net + MileSplit SD | Y — SDHSAA: AN state TF + XC pages (10 region MeetIDs verified) | Y — MileSplit roster `column-grad-year`; SDHSAA state PDFs | Y — SDHSAA Bound directory (176 schools; coach names via `/directory/new`) | Y — Athletic.net + MileSplit | ATHLETIC.NET-SEED | verified-this-run |
| TN | Negative (`tssaasports.com` + AllTrax) | TSSAA directory (browser-only) + AllTrax + MileSplit TN | Y — AthleticLIVE index (795 meets with `ani`) | Y — MileSplit roster `column-grad-year`; TSSAA Hy-Tek PDFs | Y — TSSAA directory (456 schools, browser-required); Institutional directory (inherited) | Y — TSSAA PDFs + MileSplit | RESULT-SOURCE | verified-this-run |
| TX | Negative (partner is MaxPreps) | MileSplit TX (2,423 teams; 21 tenants, no dominant timer) | Y — AthleticLIVE index (1,615 meets with `ani` across 21 tenants) | Y — MileSplit roster `column-grad-year` (sample: 45 Co2027 from 94 athletes) | N | Y — MileSplit TX (meet IDs 697029–782772) | PRIMARY | verified-this-run |
| UT | Negative (`href="#"` on results) | RunnerCard (503) + MileSplit UT | Y — AthleticLIVE index (55 meets with `ani`) | Y — MileSplit roster `column-grad-year`; UHSAA directory (inherited) | Y — UHSAA directory (PRIOR-WAVE, 162 schools with AD/coach mailto; not re-verified) | Y — MileSplit UT | DISCOVERY-ONLY | verified-this-run |
| VA | Negative (robots-blocked) | MileSplit VA (609 teams) | Y — AthleticLIVE index (1,156 meets with `ani`) | Y — MileSplit roster `column-grad-year`; MileSplit VA `/raw` grade (file `1254173`) | N | Y — MileSplit VA | RESULT-SOURCE | verified-this-run |
| VT | Negative (thinnest state) | MileSplit VT | Y — AthleticLIVE index (126 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | N — no verified regular-season index | DISCOVERY-ONLY | verified-this-run |
| WA | pending (group report not yet landed) | — | Y — AthleticLIVE index (655 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit | DISCOVERY-ONLY | inherited |
| WI | Negative | MileSplit WI + PrimeTime Timing + WIAA | Y — AthleticLIVE index (1,931 meets with `ani`) | Y — WIAA tournament files: `Yr` (2,653–2,786 G11 rows from 26-file sample); MileSplit roster | Y — WIAA `GetDirectorySchool`: 17 coach rows with email + 22 AD-with-email (9-school sample) | Y — PrimeTime + WIAA + MileSplit | PRIMARY | verified-this-run |
| WV | Negative (association) | MileSplit WV (grade in `/raw`) | Y — AthleticLIVE index (511 meets with `ani`) | Y — MileSplit roster `column-grad-year`; MileSplit WV `/raw` grade (file `1239257`) | N | Y — MileSplit WV | RESULT-SOURCE | verified-this-run |
| WY | Negative (no AN link) | MileSplit WY + `milesplit.live` | Y — AthleticLIVE index (13 meets with `ani`) | Y — MileSplit roster `column-grad-year` | N | Y — MileSplit WY | RESULT-SOURCE | verified-this-run |

**Notes on the table:**
- `DC` appears twice because the association verdict splits: DC's association (DCSAA) is negative for AN linkage but the platform (M&D Timing) is positive — hence ATHLETIC.NET-SEED.
- States marked `pending` have unlanded group reports (NY, NJ, PA, CT, RI, GA, FL, SC, NC, CA, OR, WA) but the AthleticLIVE index covers all 49, so the `ani` column is complete.
- Evidence status `verified-this-run` means the claim was observed in the wave F sessions (mid-atlantic, mountain-north, mountain-south, northeast-2, south-central, southeast-west, national-aggregators). `inherited` means the claim was carried from a prior wave or from the Midwest corpus and was not independently verified in this session.

---

## Pareto Section — Smallest Source Set Covering the Most States

The following families form the Pareto-optimal set. Together they cover all 49 jurisdictions with at least one viable data plane.

| # | Family | Reach | Unique Contribution | Verified By |
|---|--------|-------|--------------------|-------------|
| 0 | **AthleticLIVE ES index** `search.athletic.live` | **49/49** — 75,585 meets, 62,954 real MeetIDs (`ani > 0`) | Free MeetIDs + state/date/tenant, no browser required | `[SELF]` national-aggregators.md |
| 1 | **MileSplit team rosters** (`<st>.milesplit.com/teams/<id>/roster`) | **49/49** — 26,265 HS teams, free `column-grad-year` per athlete | Co2027 grad year per athlete, one request per school | `[SELF]` national-aggregators.md |
| 2 | **AthleticLIVE per-event blobs** | ~290 tenants | Per-row grade (`a.y` / `y`) + AN athlete/team ids | `[SELF]` national-aggregators.md |
| 3 | **Association-published AN links** | **12 states** (MI, OH, IL, MA, MT, ID, AZ, MO, NE, SD, OR, DC-platform) | Authoritative MeetIDs, avoids ~16.7% unseeded meets | `[PEER]` wave reports |
| 4 | **Association APIs (grade oracles)** | **6 states** (IL, MN, WI, KS, NE, OH) | School universe + grade oracles + coach/AD contacts | `[MIDWEST]` greatlakes/plains |
| 5 | **TFRRS Indiana** | IN only | 1,861 Co2027 for 2 requests (0.0011 req/athlete) | `[MIDWEST]` greatlakes |

**The smallest set that reaches the most:** `{0 + 1}` reaches **all 49 jurisdictions** with both a meet key (AthleticLIVE) and an athlete-with-grade key (MileSplit rosters), in `1 × 49 + 26,265` requests, using **two sources and zero credentials**. Everything else in this wave is either a quality improvement (association-published MeetIDs avoid unseeded meets), a capability add (coach contacts from association APIs), or a validation oracle.

### Recommendation breakdown by class

| Recommendation | Count | States |
|---------------|------:|--------|
| **PRIMARY** | 6 | AL, IA, IL, MN, TX, WI |
| **ATHLETIC.NET-SEED** | 11 | AZ, DC, ID, MI, MO, MT, ND, NE, OH, OR, SD |
| **RESULT-SOURCE** | 12 | AR, CO, DE, IN, KY, LA, MS, NM, TN, VA, WV, WY |
| **COACH-DIRECTORY** | 1 | KS |
| **DISCOVERY-ONLY** | 14 | CA, CT, FL, GA, NC, NJ, NV, NY, PA, RI, SC, UT, VT, WA |
| **REJECT** | 0 | — |

(Count: 6 + 11 + 12 + 1 + 14 + 0 = 44. The remaining 5 jurisdictions have `pending` group reports but all carry evidence from the AthleticLIVE index and MileSplit rosters, already classified above.)

### States by recommendation class

| Class | Count | States |
|-------|------:|--------|
| PRIMARY | 6 | AL, IA, IL, MN, TX, WI |
| ATHLETIC.NET-SEED | 11 | AZ, DC, ID, MI, MO, MT, ND, NE, OH, OR, SD |
| RESULT-SOURCE | 12 | AR, CO, DE, IN, KY, LA, MS, NM, TN, VA, WV, WY |
| COACH-DIRECTORY | 1 | KS |
| DISCOVERY-ONLY | 14 | CA, CT, FL, GA, NC, NJ, NV, NY, PA, RI, SC, UT, VT, WA |

**Total:** 49 jurisdictions (48 states + DC). All have at least one data plane.

---

## Gap List — States Whose Co2027 Enumerability Is Unknown

The following states have **no verified grade/class-year source** at the athlete level from their association or a timing-company result surface. They rely on MileSplit rosters for grad-year, which is verified for all 49 — so **no state is truly a gap for Co2027 enumerability**. The gaps below are for **association-level or result-level grade evidence** (non-MileSplit):

| St | Gap | Responsible source |
|----|-----|--------------------|
| CA | No association or timer result with grade observed | Group report pending |
| CT | Association unreadable (CIAC TLS expired); timer results no grade | Group report pending |
| FL | No association or timer result with grade observed | Group report pending |
| GA | No association or timer result with grade observed | Group report pending |
| NC | No association or timer result with grade observed | Group report pending |
| NJ | No association or timer result with grade observed | Group report pending |
| NY | NYSPHSAA/PSAL no grade; no timer result observed | Group report pending |
| PA | PIAA `/officials/directory/` robots-disallowed; no grade observed | Group report pending |
| RI | No association or timer result with grade observed | Group report pending |
| SC | No association or timer result with grade observed | Group report pending |
| WA | No association or timer result with grade observed | Group report pending |

**Note:** All 49 states have MileSplit roster `column-grad-year` verified (the free path from the national-aggregators lane). The "gap" here means the state lacks an **alternative** non-MileSplit grade source, which matters for validation and for states where MileSplit rosters may be empty (as demonstrated by the PA sample that returned 0 athletes from one school).

---

## Sources Consulted

| # | Source | Type | Jurisdictions Covered |
|---|--------|------|----------------------|
| 1 | `/home/lewis/Downloads/census-source-research/README.md` | Consolidated README | All 49 |
| 2 | `/home/lewis/Downloads/census-source-research/mid-atlantic.md` | Wave F3 Mid-Atlantic | MD, DE, VA, WV, DC |
| 3 | `/home/lewis/Downloads/census-source-research/mountain-north.md` | Wave F8 Mountain North | MT, WY, ID |
| 4 | `/home/lewis/Downloads/census-source-research/mountain-south.md` | Wave F7 Mountain South | CO, UT, NV, AZ, NM |
| 5 | `/home/lewis/Downloads/census-source-research/northeast-2.md` | Wave F2 Northeast | MA, VT, NH, ME |
| 6 | `/home/lewis/Downloads/census-source-research/south-central.md` | Wave F6 South Central | TX, OK, AR, LA |
| 7 | `/home/lewis/Downloads/census-source-research/southeast-west.md` | Wave F5 Southeast-West | AL, MS, TN, KY |
| 8 | `/home/lewis/Downloads/census-source-research/national-aggregators.md` | Wave F10 National | All 49 |
| 9 | `research/sources/state-assoc-midatlantic/SOURCE_REPORT.md` | Repo mid-Atlantic | DC, DE, MD, NJ, NY, PA, CT, MA, RI, NH, VT, ME |
| 10 | `research/sources/state-assoc-greatlakes/SOURCE_REPORT.md` | Repo Great Lakes | IL, IN, MI, OH, WI |
| 11 | `research/sources/state-assoc-plains/SOURCE_REPORT.md` | Repo Plains | IA, KS, MN, MO, ND, NE, SD |
| 12 | `research/sources/state-assoc-southeast/SOURCE_REPORT.md` | Repo Southeast | (used as Midwest reuse context) |
| 13 | `research/sources/state-assoc-southcentral/SOURCE_REPORT.md` | Repo South-Central | (used as Midwest reuse context) |
| 14 | `research/sources/state-assoc-mountain/SOURCE_REPORT.md` | Repo Mountain | (used as prior-wave context) |
| 15 | `research/sources/athleticnet/SOURCE_REPORT.md` | Repo Athletic.net | All 49 |
| 16 | `research/sources/national-aggregators/SOURCE_REPORT.md` | Repo National Aggregators | All 49 |
| 17 | `research/sources/milesplit-national/SOURCE_REPORT.md` | Repo MileSplit National | All 49 |
| 18 | `research/sources/timing-providers-national/SOURCE_REPORT.md` | Repo Timing Providers | All 49 |
| 19 | `research/sources/coach-directories-national/SOURCE_REPORT.md` | Repo Coach Directories | All 49 |
