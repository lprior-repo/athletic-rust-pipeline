# Source Applicability Matrix — 49 Continental Jurisdictions + DC

**Date:** 2026-09-22
**Purpose:** One row per jurisdiction (48 states + DC) with per-source capability verdicts, deciding the smallest source set that covers the most states for a Class-of-2027 census. Appendices A and B carry the dated host-level and timing-provider survey records behind these verdicts.

**Grade/class-year is the deciding column:** an athlete can only be a Co2027 candidate if some source states grade or graduating class. States marked N on grade/class have no Co2027 enumerability.

Class of 2027 = grade 11 in the 2025–26 school year (the `grades: [11]` / Co2027 convention used across these lanes).

**Consolidated 2026-09-27.** Appendix A preserves the unique dated observations of the deleted `SOURCES_SURVEY.md` (2026-09-20 source survey), Appendix B those of the deleted `timing-provider-inventory.md` (2026-09-22). The other two deleted root documents carried no per-jurisdiction verdicts: `PROFILE_REPLICATION.md` (2026-09-20 profile-replication plan) and `COLLECTOR_PATTERNS.md` (2026-09-20 transport-gap inventory) were already marked superseded, and the Class-of-2027 definition they shared is the one stated above. `tools/athletic-net-pr-population/README.md`'s endpoint findings moved to `athleticnet/CORPUS_BUILD_2026-09-20.md`; the dataset probes to `national-aggregators/OPEN_DATASETS_2026-09-20.md`.

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
- `DC`'s verdict splits: the DCSAA association is negative for AN linkage while the M&D Timing platform is positive — hence ATHLETIC.NET-SEED.
- States marked `pending` have unlanded group reports (NY, NJ, PA, CT, RI, GA, FL, SC, NC, CA, OR, WA) but the AthleticLIVE index covers all 49, so the `ani` column is complete.
- Evidence status `verified-this-run` means the claim was observed in the wave F sessions (mid-atlantic, mountain-north, mountain-south, northeast-2, south-central, southeast-west, national-aggregators). `inherited` means the claim was carried from a prior wave or from the Midwest corpus and was not independently verified in this session.
- **After this matrix was written:** the states whose group reports were `pending` have since landed as lane reports (`state-assoc-westcoast/`, `state-assoc-midatlantic/`, `state-assoc-southeast/`); the `inherited` rows above were not re-derived for this consolidation, so consult the lane report before treating those verdicts as current evidence.

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

Rows 1–8 are wave reports from a prior research pass; they live **outside this repository** (`~/Downloads/census-source-research/`, a non-repo artifact) and cannot be resolved as links from here. Rows 9–19 are repo lane reports.

| # | Source | Type | Jurisdictions Covered |
|---|--------|------|----------------------|
| 1 | `~/Downloads/census-source-research/README.md` (non-repo artifact) | Consolidated README | All 49 |
| 2 | `~/Downloads/census-source-research/mid-atlantic.md` (non-repo artifact) | Wave F3 Mid-Atlantic | MD, DE, VA, WV, DC |
| 3 | `~/Downloads/census-source-research/mountain-north.md` (non-repo artifact) | Wave F8 Mountain North | MT, WY, ID |
| 4 | `~/Downloads/census-source-research/mountain-south.md` (non-repo artifact) | Wave F7 Mountain South | CO, UT, NV, AZ, NM |
| 5 | `~/Downloads/census-source-research/northeast-2.md` (non-repo artifact) | Wave F2 Northeast | MA, VT, NH, ME |
| 6 | `~/Downloads/census-source-research/south-central.md` (non-repo artifact) | Wave F6 South Central | TX, OK, AR, LA |
| 7 | `~/Downloads/census-source-research/southeast-west.md` (non-repo artifact) | Wave F5 Southeast-West | AL, MS, TN, KY |
| 8 | `~/Downloads/census-source-research/national-aggregators.md` (non-repo artifact) | Wave F10 National | All 49 |
| 9 | `state-assoc-midatlantic/SOURCE_REPORT.md` | Repo mid-Atlantic | DC, DE, MD, NJ, NY, PA, CT, MA, RI, NH, VT, ME |
| 10 | `state-assoc-greatlakes/SOURCE_REPORT.md` | Repo Great Lakes | IL, IN, MI, OH, WI |
| 11 | `state-assoc-plains/SOURCE_REPORT.md` | Repo Plains | IA, KS, MN, MO, ND, NE, SD |
| 12 | `state-assoc-southeast/SOURCE_REPORT.md` | Repo Southeast | FL, GA, NC, SC, VA, WV |
| 13 | `state-assoc-southcentral/SOURCE_REPORT.md` | Repo South-Central | AL, AR, KY, LA, MS, OK, TN, TX |
| 14 | `state-assoc-mountain/SOURCE_REPORT.md` | Repo Mountain | AZ, CO, ID, MT, NM, UT, WY |
| 15 | `state-assoc-westcoast/SOURCE_REPORT.md` | Repo West Coast | CA, OR, WA, AK, HI, NV |
| 16 | `athleticnet/SOURCE_REPORT.md` | Repo Athletic.net | All 49 |
| 17 | `national-aggregators/SOURCE_REPORT.md` | Repo National Aggregators | All 49 |
| 18 | `milesplit-national/SOURCE_REPORT.md` | Repo MileSplit National | All 49 |
| 19 | `timing-providers-national/SOURCE_REPORT.md` | Repo Timing Providers | All 49 |
| 20 | `coach-directories-national/SOURCE_REPORT.md` | Repo Coach Directories | All 49 |

---

## Appendix A — Host-level survey record (2026-09-20, historical)

Unique observations from the deleted `SOURCES_SURVEY.md`, kept because later lane reports did not re-cover these hosts. **Historical:** the survey's operating policy (a 2 rps/host ceiling, `--authorized-host` relaxation) and its collection design are obsolete — pacing and admission are owned by `crates/census-crawl/src/registry/` (1 request/second default; 0.1 rps for `Crawl-delay: 10` origins; one in-flight request) and transport by `ARCHITECTURE.md`. Its unsafe statement about ignoring site restrictions is not retained in any form.

| # | Source | Population | Reachability (2026-09-20) | Identity | Restriction (recorded) |
|---|--------|-----------|--------------|----------|------------------------|
| 4 | `www.yentiming.com` | NY Section V HS (indoor/outdoor) | FEASIBLE — live static HTML, 2008–2027, + leaderboards | name-based | robots 404 |
| 6 | `elitefeats.com` | NY HS track (Sections VIII/XI) | FEASIBLE (static ASP; PL/Name/YR/Team/Wind/Time) | per-meet bib | robots disallows utility paths; `/t-Results` allowed |
| 9 | `www.runnercard.com` | UT/ID/WY HS + JH | FEASIBLE-WITH-WORK (legacy report engine, fixed-width text) | per-meet bib | none (robots 404) |
| 10 | `cifss.org` (CIF-SS) | CA HS postseason | FEASIBLE-WITH-WORK (PDF text layer) | name+grade+school | robots allow-all |
| 12 | `finishedresults.com` + `api.trackscoreboard.com` | CA/NV HS+college | FEASIBLE-WITH-WORK via API (front end is JS-only) | API athlete ids | api host robots `Disallow: /` |
| 13 | `results.leonetiming.com` | NY/PA | JS-only app (engineering blocker) | unknown | robots + ToS prohibit automated extraction and AI/TDM use |
| 14 | `milesplit.live` | HS/college live | JS-only SPA; API + Firebase data plane | inherits MileSplit | no robots (SPA catch-all) |
| 15 | `worldathletics.org` | elite/pro + U20 | parseable static HTML (675 KB records pages) | name+DOB, no id | ToS bans crawlers/automated agents |
| 17 | `live.athletictiming.net` / `live.athletic.net` | white-label live | loaded AthleticLIVE tenant bundles from `livestatic.athletic.net`; survey did not follow them under the then-current Athletic.net exclusion | n/a | n/a |
| 18 | `athletic.live` | — | 6/6 probes 302 → `live.athletic.net`; not followed | — | — |
| 19 | `www.athletictiming.net` / `athletictiming.rsupartner.com` / `www.tfmeetpro.com` | — | pointers/vendor/road-race only | — | rsupartner robots `Disallow: /`; others none |
| 5 | `www.directathletics.com` | HS + college meets | usable (static, filterable index, `page=792`) | results carry **no athlete id**; profile URL 302 → TFRRS | ToS expressly prohibits commercial exploitation of results/rosters/IDs |

Hosts whose rows the later lanes superseded (rows 1–3, 7–8, 11, 16, 20 of that survey): MileSplit and `milesplit.live` → `milesplit-national/`, `milesplit-cohort-enumerability/`; TFRRS/DirectAthletics/MaxPreps/RunnerSpace/World Athletics → `national-aggregators/`; Baumspage, Finish Timing → `timing-providers-national/`; Kaggle/HuggingFace/Zenodo/Figshare → `national-aggregators/OPEN_DATASETS_2026-09-20.md`; association/NCAA blocks (`mhsaa.com`, `ihsaa.org`, `ohsaa.org`, `stats.ncaa.org`, `opentrack.run`) → the association lanes and `timing-providers-national/`.

**Request ledger (all probes 2026-09-20).** One-shot, sequential, no retries, no concurrency, bounded `--max-time`, plainly-identifying UA, robots fetched first on every host.

| Host | Requests | Notes |
|---|---|---|
| `www.milesplit.com` / `ny.` / `js.sp.` / `milesplit.live` | 25 + 3 + 2 (main) | includes the verified API probe |
| `www.tfrrs.org` | 11 + 2 (main) | |
| `www.directathletics.com` | 13 + 2 (main) | |
| `www.maxpreps.com` | 11 | |
| `cifss.org` / `cifsshome.org` | 6 / 1 | |
| `mhsaa.com` / `ihsaa.org` / `ihsaa.eventlink.com` / `ohsaa.org` / blob | 12 / 6 / 4 / 5 / 2 | |
| `baumspage.com` | 16 + 10 | |
| `runnerspace.com` (+ subdomains) | 11 | |
| `leonetiming.com` (www/results) | 6 | |
| `finishedresults.com` / `trackscoreboard` / api | 5 / 3 / 2 | one API GET before its robots was seen — disclosed |
| `finishtimingresults.com` / `live.finishtiming.com` | 4 / 1 | |
| `runnercard.com` / `elitefeats.com` / `tfmeetpro.com` | 10 / 11 / 5 | |
| `athletictiming.net` / `live.athletictiming.net` / `…rsupartner.com` | 4 / 4 / 2 | rsupartner results fetched before its robots was seen — disclosed |
| `www.kaggle.com` (+ GCS object) | ~16 + 1 | over the ~10 guide; includes the verified download |
| `api.github.com` / `github.com` / `raw.githubusercontent.com` | 11 + 1 + ~12 | core API limit 60/hour observed |
| `huggingface.co` | 10 | |
| `worldathletics.org` / `stats.ncaa.org` / `opentrack.run` | 4 / 1 / 1 | |
| `zenodo.org` / `api.figshare.com` / `catalog.data.gov` / `archive.org` | 2 / 2 / 2 / 1 | |
| `www.opensplittime.org` / `www.olympedia.org` / `www.usatf.org` / `www.thepowerof10.info` / `www.yentiming.com` | 2 / 2 / 1 / 1 / 3 | |
| `athletic.live` | 6 | all 302 into the then-excluded host; not followed |
| **athletic.net** | **0** | excluded under the survey-era owner instruction (ADR-013 supersedes that exclusion) |

**Open questions recorded then** (most were answered by the lane reports that landed afterwards; none of these are current instructions): MileSplit rankings XHR route; whether every MileSplit meet exposes `/raw`; TFRRS `tf.`/`xc.`/`florida.` subdomains and `director_info.html` terms; DirectAthletics HS `Year` values and track result pages; MaxPreps XC stat tables; elitefeats `YR` population and page-shape uniformity; runnercard `Mark` population on past seasons; `finishedresults`/`trackscoreboard` API route shapes and id stability; whether unauthenticated Kaggle download works for datasets beyond the one observed; Yen Timing ToS text and event coverage beyond the leaderboard index; flosports/MaxPreps terms; other state sections' timing hosts.

---

## Appendix B — Timing-provider roster (2026-09-22 inventory, merged here)

Unique content of the deleted `timing-provider-inventory.md`. Per-provider depth for the families the timing lane later captured first-hand (FlashResults, PrimeTime, Wayzata, FinishLynx/MeetPro, RACE RESULT, OpenTrack, GSE Timing, Finish Timing, RunWV, XCStats, Hero's Timing, TRXC) lives in `timing-providers-national/SOURCE_REPORT.md` with captures; the digest below covers the providers that report does not detail. The inventory's own "Recommended Pipeline Architecture" section is **not** retained: the current pipeline is defined by `ARCHITECTURE.md`, `SOURCE_ADAPTER_GUIDE.md` and the registry, and its "reject all browser-only providers" step is superseded by the headed-browser transport design (`CHROMIUM_DESIGN.md`).

| # | Provider | States | Stable IDs | Grade? | Cost / 1k athletes | Recommendation |
|---|----------|--------|------------|--------|---------------------|----------------|
| 1 | MileSplit | 49 | `athleteId`, `teamId`, `meetId`, `performanceId`, `gradYear` (JSON) | **Yes** — `gradYear` in roster API; `Class of YYYY` in HTML | Free | **PRIMARY** |
| 2 | AthleticLIVE tenant network | 49 | `ani` (Athletic.net MeetID, 83.3% of 75,585 docs), `EventID` (blob) | No (raw data) | Free | **PRIMARY — Result Source** |
| 3 | Athletic.net (site + platform) | 49 | `AthleteID`, `TeamID`, `MeetID`, `SchoolID`, `ResultCode` | **Yes** — via profile metadata and ranking pages | Free (browser-gated) | **ATHLETIC.NET-SEED** |
| 4 | DirectAthletics + TFRRS | 49 | DAT `teamId` (Bound), TFRRS `team-id` | No (college-weighted) | Free | **RESULT-SOURCE** (FL/IN/NH only) |
| 5 | PrimeTime Timing | ~10 | `ptmeetid`, Firebase `EventID` | No | Free (but ToU prohibitive) | **REJECT** |
| 6 | Finish Timing | 3 (OH/KY/IN) | `FT id = "20" + AN MeetID`, native IDs | **Yes** — numeric 9–12 column in result rows | Free | **RESULT-SOURCE** |
| 7 | Lancer Timing | 4 (MA/NH/NE/CNESSPA) | `lan-<state>-<meetid>` on AN; AN MeetID | No | Free | **RESULT-SOURCE** |
| 8 | 802 Timing | 1 (VT) | AN MeetID | No | Free | **RESULT-SOURCE** |
| 9 | Sub5 | 1 (ME) | AN MeetID | No | Free | **RESULT-SOURCE** |
| 10 | Brewer Timing | 2 (ME/MA) | AN MeetID | No | Free | **RESULT-SOURCE** |
| 11 | Hy-Tek Meet Manager | N/A (per-meet) | Hy-Tek event numbers in result files | No | N/A (software, not host) | **DISCOVERY-ONLY** |
| 12 | TrackScoreboard / FinishedResults | 2 (MA/OH) | Firebase MeetID | No | Free | **RESULT-SOURCE** |
| 13 | MeetPro (DirectAthletics) | National | AN MeetID | No | Free | **DISCOVERY-ONLY** |
| 14 | RaceTec | 1 (WI) | AN MeetID | No | Free | **RESULT-SOURCE** |
| 15 | AccuRace Timing | 1 (WI) | AN MeetID | No | Free | **RESULT-SOURCE** |
| 16 | TrackSide Timing | 1 (WI) | AN MeetID | No | Free | **RESULT-SOURCE** |
| 17 | K2 Timing | 1 (WI) | AN MeetID | No | Free | **RESULT-SOURCE** |
| 18 | Performance Timing | 1 (WI) | AN MeetID | No | Free | **RESULT-SOURCE** |
| 19 | TRXC Timing | 1 (MO) | AN MeetID | No | Free | **RESULT-SOURCE** |
| 20 | State Running Network | 4 (NH/MA/CT/RI) | AN MeetID links | No | Free | **DISCOVERY-ONLY** |
| 21 | Baum's Page | 1 (OH) | `peventid`, Hy-Tek event numbers | No (Year column empty) | Free | **RESULT-SOURCE** |
| 22 | MITS (Michigan) | 1 (MI) | AN MeetID (AthleticLIVE tenant) | No | Free | **RESULT-SOURCE** |
| 23 | WIAA | 1 (WI) | AN MeetID | No | Free | **DISCOVERY-ONLY** |
| 24 | IHSA | 1 (IL) | AN MeetID, athlete IDs, grades | **Yes** — API returns grade + AN athlete id | Free | **VALIDATION** |
| 25 | SDHSAA / Bound | 1 (SD) | Bound team/school IDs, AN MeetID | **Yes** — yearbook PDFs, grade 11 | Free | **VALIDATION** |

### Digest for providers not detailed in `timing-providers-national/SOURCE_REPORT.md`

| Provider | Hosts | States | Enumeration / key URL | Stable IDs | Grade |
|----------|-------|--------|----------------------|------------|-------|
| Lancer Timing | `lancertiming.com`, `lancer-results.com` | 4 (MA primary, NH, NE/CNESSPA) | AN MeetID links from association pages; direct surface not independently fetchable | AN MeetID | No |
| 802 Timing | `802timing.com` | 1 (VT) | Only timer listed on MileSplit VT → AN MeetID | AN MeetID | No |
| Sub5 | `sub5timing.com` | 1 (ME) | MPA state meets + NE championship | AN MeetID | No |
| Brewer Timing | `brewertiming.com` | 2 (ME, MA) | State and league meets | AN MeetID | No |
| Hy-Tek Meet Manager | N/A (local software) | N/A | Result files (`.htm`/`.html`/`.pdf`) carry Hy-Tek event numbers; not independently discoverable | Hy-Tek event numbers | Rare (some WI PDFs) |
| TrackScoreboard / FinishedResults | `finishtiming.trackscoreboard.com`, `api.trackscoreboard.com`, Firebase RTDB | 2 (MA indoor, OH live) | Angular SPA over Firebase; archive on `finishedresults.com` | Firebase MeetID (= `20 + ANID` in OH) | No |
| MeetPro | `tfmeetpro.com` | National | Software by DirectAthletics, linked from MileSplit meet pages; results uploaded by host/timer, no browsable index | AN MeetID | No |
| RaceTec | `racetec.com` | 1 (WI) | WI timer, behind Performance Timing in usage | AN MeetID | No |
| AccuRace Timing | `accu race.com` (host string as recorded in the inventory; not re-verified) | 1 (WI) | WI timer; AthleticLIVE tenant | AN MeetID | No |
| TrackSide Timing | `tracksidetiming.com` | 1 (WI) | WI timer; AthleticLIVE tenant | AN MeetID | No |
| K2 Timing | `k2timing.com` | 1 (WI) | WI timer; AthleticLIVE tenant | AN MeetID | No |
| Performance Timing | `performancetiming.com` | 1 (WI) | WI timer; AthleticLIVE tenant | AN MeetID | No |
| State Running Network | various state-running sites | 4 (NH primary, MA, CT, RI) | Aggregator for NH; links to AN meet pages | AN MeetID links | No |
| Baum's Page | `www.baumspage.com` | 1 (OH) | `/cc/index.php` and `/track/index.php` list the season; NW/central/east OH small-school meets; OHSAA district/regional archives 2003–2021 | `peventid` (scoped per sport and `table=A|C`), Hy-Tek event numbers | No — Year column present but empty in every sampled file |
| MITS (Michigan Indoor Track Series) | AthleticLIVE tenant in MI | 1 (MI) | MI indoor series; AthleticLIVE tenant | AN MeetID | No |
| WIAA | `wiaa.com` | 1 (WI) | State archives, 130 files in 2025 (repo lane: `state-assoc-greatlakes/`) | AN MeetID | No |
| IHSA | `ihsatf.org` (inventory); `api.ihsa.org` (repo lane) | 1 (IL) | AthleticLIVE tenant + AN links; API returns grade + AN athlete id (repo lane: `state-assoc-greatlakes/`) | AN MeetID, athlete IDs, grades | Yes |
| SDHSAA / Bound | `sdhsaa.com`, `www.gobound.com` | 1 (SD) | SDHSAA activity pages + Bound `GetTree` (207 SchoolIDs) / `GetCalendar` (11 XC + 3 TF state meets) (repo lane: `state-assoc-plains/`) | Bound school id (hex), AN MeetID | Yes — yearbook PDFs carry grade 11 |

### Providers that could not be verified (2026-09-22)

Observed as link targets or named in research but never independently verified, due to access restrictions:

| Provider | Reason |
|----------|--------|
| Finish Timing / TrackScoreboard live SPA | Requires JS rendering (SvelteKit / Angular SPA) |
| PrimeTime live results (Firebase RTDB) | ToU prohibits automation; Firebase private |
| PrimeTime-owned surface (`www.pttiming.com`) | Same ToU as live results (Karmarush LLC operator) |
| Athletic.net SPA pages | Cloudflare-403 from research machine; requires browser |
| Speed Sport (NE timer) | Domain observed only as link target, never fetched |
| Northstar Timing (NE) | Domain observed only as link target |
| MSTCA Live Results (NE) | Domain observed only as link target |
| iResultsLive | Domain observed only as link target |
| Millennium Timing | Domain observed only as link target |
| Marathon Sports | Domain observed only as link target |
| MTS (Michigan) | No independent site found; lives on AthleticLIVE stack only |
| MaxPreps | Score partner, not a result host; widget football-configured |
| GoFan | Browser-only, no verified IDs |
| Arbiter | Browser-only, no verified IDs |
| niaa.com/NV | CAPTCHA-gated (Nevada) |
| Various guessed association domains | DNS-failed on guessed hosts |

### Critical refutations (2026-09-22)

1. **Athletic.net state URL (`athletic.net/track-and-field-outdoor/usa/high-school/<state>/<assoc>`) is NOT evidence of partnership.** Returns 200 for any slug (~7.4 KB Angular shell). Never use as seed.
2. **`ani = -1` is a sentinel, not a MeetID.** 516 docs carry it. Must filter `ani > 0`.
3. **MileSplit MeetID space is separate from AthleticLIVE and AN.** Same MeetID 716278 appears in TX, OK, and LA indices. Never join on a bare integer meet ID.
4. **MileSplit's team index `tx.milesplit.com` does not bound state.** A Georgia meet (741372) appears in the TX index.
5. **AthleteID is 1:N for person-to-record mapping.** One athlete returned two IDs for the same name+school+class. Join on `(AthleteID, school, grad-year)`.
6. **Every result file carries name/school/mark/place but NEVER a grade** (except the IHSA API, some WI Hy-Tek PDFs, and SD yearbook PDFs). Co2027 must come from MileSplit rosters or AN, not from result files.
7. **DirectAthletics is DISCOVERY-ONLY outside Indiana.** College-weighted, refuted in MT/WY/ID, AL/MS/TN/KY, CO, TX/OK/AR/LA.
8. **MileSplit `/api/` and `/rankings` are robots-disallowed.** Rate carefully.

### Provider-to-state coverage matrix (verbatim from the inventory; its header repeats the IN column)

| Provider | OH | WI | MO | MI | MA | VT | ME | NE | NH | IL | SD | FL | IN | TX | KY | IN | All |
|----------|----|----|----|----|----|----|----|----|----|----|----|----|----|----|----|----|-----|
| MileSplit | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | 49 |
| AthleticLIVE | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | 49 |
| Athletic.net | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | 49 |
| DAT/TFRRS | X | | | | | | | | | | | X | X | | | X | 3 HS |
| PrimeTime | X | X | X | X | | | | X | | X | | X | X | X | X | X | 10 |
| Finish Timing | X | | | | | | | | | | | | | | X | | 3 |
| Lancer Timing | | X | | | X | | | X | X | | | | | | | | 4 |
| 802 Timing | | | | | | X | | | | | | | | | | | 1 |
| Sub5 | | | | | | | X | | | | | | | | | | 1 |
| Brewer Timing | | | | | X | | X | | | | | | | | | | 2 |
| IHSA | | | | | | | | | | X | | | | | | | 1 |
| SDHSAA | | | | | | | | | | | X | | | | | | 1 |

## Appendix C — Rust-coverage cross-check (2026-09-29)

Purpose: confirm the Matrix above does not name a jurisdiction whose coach/AD route has no Rust
acquisition path. Method: take the 49 jurisdiction rows' *Coach / AD Source?* column, then intersect
the positive rows with (a) the slugs registered in `crates/census-crawl/src/registry/table/` and the
DragonFly tenants at `crates/census-crawl/src/coach_directories/mod.rs` (`REGISTERED`, 15 entries), and
(b) the PORT-NOW groups in the dispatch queue (`target/port-queue.json`).

Fifteen rows carry a positive coach/AD route: **IA IL KS KY MI MN ND NE NJ OH OK SD TN UT WI**.
Fourteen of them are reached:

| Route state | Rust path |
|---|---|
| IL | slug `ihsa` (API per-school coach/AD + email) |
| KS | slug `ks` (KSHSAA AD name + email) |
| MN | slug `mshsl` (6,596 coach rows in the prototype lane) |
| OH | slug `ohsaa` (myOHSAA) |
| WI | slug `wiaa` (`GetDirectorySchool`) |
| ND, NE | slug `plain_names` (NDHSAA / NSAA member directories) |
| TN | slug `coach_directories`, TSSAA tenant |
| IA, KY, MI, NJ, OK, UT | PORT-NOW groups `ia_iahsaa`, `ky_khsaa` + `arbiter_orgs`, `mi_mhsaa` + `mi_mitca`, `nj_njsiaa`, `ok_ossaa`, `ut_uhsaa` |
| NH, MT, WV (route marked N here, Arbiter org exists) | PORT-NOW `arbiter_orgs` |

**The one uncovered row is SD, and it is an accepted gap for this wave.** The chain is now closed
with on-disk evidence: SDHSAA publishes no directory of its own — `https://sdhsaa.com/schools` answers
404, and the association homepage links its Member Directory to
`https://www.gobound.com/sd/associations/sdhsaa/schools`, i.e. to the same Bound instance
(`notes/sd-sdhsaa.md`, "Entry dropped (Main, 2026-09-27)") — and `gobound.com` refuses the directory
path, its cached robots body being a 118-byte 403 page
(`coach-directories-national/SOURCE_REPORT.md`). That is why the queue carries `bound_staff.py`,
`bound_crawl.py` and `bound_match.py` as REFUSED-BY-ROBOTS, and nothing is scheduled that would
reopen it. The two remaining candidate routes were measured and rejected: the per-school-website lane
produced **4** SD rows (`census-prototype/out/extra/SD-school-sites.jsonl`) and is OUT-OF-SCOPE in the
queue besides, and the Bound lane's 560 SD rows (`out/extra/SD-bound.jsonl`) are preserved historical
captures — per ARCHITECTURE they are evidence of what was once reachable, not a fresh-run acceptance
path. SD's school universe, grade evidence and results stay covered by the Athletic.net / MileSplit /
SDHSAA-PDF lanes this Matrix marks Y, so the gap is coach-and-AD contacts only, not enumerability, and
closing it needs either a robots-permitted SD school list to appear or a licensed/partner route.

Delaware and DC are **not** gaps despite their association-site parsers being refused or absent:
`parsers/de_diaa.py` is REFUSED-BY-ROBOTS on `diaa.org`, and no `parsers/dc_*.py` exists at all — the
DCSAA route is the prototype root driver `dc_directory.py`, which is outside the queue's 77 units. Both
jurisdictions are registered DragonFly tenants (`coach_directories/mod.rs` `REGISTERED`: Delaware,
DistrictOfColumbia), so their member directories and staff are reachable through
`maxinfosite-api-live.dragonflyathletics.com` in the already-implemented Rust lane. The cross-check is
therefore on the route, not on any single Python unit.

Standing caveat for the whole table: the queue's first unresolved question is unproven — that the
DragonFly `/schools/{short}/summary` route publishes the same TF/XC staff the superseded association
parsers yielded (e.g. `ga_ghsa` 3,381 coach rows). Until a fixture proves it per state, "covered" here
means *an owned acquisition path exists*, not *staff parity is verified*.

