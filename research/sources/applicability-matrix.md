# Source Applicability Matrix — 48 Continental States + DC = 49 Jurisdictions

**Date:** 2026-09-22
**Purpose:** One row per jurisdiction (48 states + DC) with per-source capability verdicts, deciding the smallest source set that covers the most states for a Class-of-2027 census. Appendices A and B carry the dated host-level and timing-provider survey records behind these verdicts.

**Grade/class-year is the deciding column:** an athlete can only be a Co2027 candidate if some source states grade or graduating class. States marked N on grade/class have no Co2027 enumerability.

**Historical cohort convention (2026-09-22):** these lanes used grade 11 in the 2025–26 school year (`grades: [11]`) as their Class-of-2027 research frame. This dated convention is not the sole current cohort admission rule: [ARCHITECTURE.md §§8–9](../../ARCHITECTURE.md) requires source-owned grade/class evidence interpreted with its academic year and contradictions; “junior” or grade 11 in another year is not automatically Co2027.

**Consolidated 2026-09-27.** Appendix A preserves the unique dated observations of the deleted `SOURCES_SURVEY.md` (2026-09-20 source survey), Appendix B those of the deleted `timing-provider-inventory.md` (2026-09-22). The other two deleted root documents carried no per-jurisdiction verdicts: `PROFILE_REPLICATION.md` (2026-09-20 profile-replication plan) and `COLLECTOR_PATTERNS.md` (2026-09-20 transport-gap inventory) were already marked superseded, and the Class-of-2027 definition they shared is the one stated above. `tools/athletic-net-pr-population/README.md`'s endpoint findings moved to `athleticnet/CORPUS_BUILD_2026-09-20.md`; the dataset probes to `national-aggregators/OPEN_DATASETS_2026-09-20.md`.

**Historical boundary:** the matrix, Pareto recommendations and appendices below retain the original research observations, including `verified-this-run` for that prior wave, not this session. Current implementation versus executed coach evidence is separately inventoried in [Appendix C](#appendix-c--coach-implementation-versus-executed-coverage-2026-10-02). Neither old availability estimates nor registered adapters establish a fresh national acquisition.

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

---

## Appendix C — Coach implementation versus executed coverage (2026-10-02)

**Bead:** `athletic-rust-pipeline-8bs`. **Inventory author/model:** `openai-codex/gpt-6.1-sol`.
This is a static source/capture/report inventory, not a new acquisition or command result.
No commands, tests, scenarios, live database reads or external requests were performed for this
appendix. Main owns integrated validation and the eventual fresh run. Historical source/store/export
evidence remains unchanged. The national denominator is **48 continental states + DC = 49**;
the 51-location survey includes AK/HI only as excluded historical selectors.

### Counting contract and reconciliation

Count provider families, not directory names, state selectors, CLI modes, files, people or emails.
Here a family shares an acquisition/schema contract: DragonFly is one across its associations;
CIAC/MPA/RIIL share the FusionPoint family; NDHSAA and NSAA are independent families even though
one `plain_names` slug dispatches both. The coach CSV importer is a delivery surface over other
providers' evidence, not another acquired provider. Association-native sources and delegated
platforms retain distinct provenance (e.g. GHSA PDF versus DragonFly GHSA).

| Inventory / denominator | Count | What the count does and does not establish |
|---|---:|---|
| User-reported original implementations | **50 reported; owning manifest unresolved** | The recovered external prototype snapshot has **63 nonempty parser modules + empty `__init__.py` = 64 Python files**, and **48 distinct registered `custom:` keys + `html` = 49 stage-one kinds**. Neither is a 50-entry manifest. All 63 parser keys and the generic dispatch are mapped below; the report's original revision/counting unit remains missing, not replaced by these snapshot counts. |
| User-reported acquired families | **8 reported; exact eight-family receipt unresolved** | Recovered dated reports identify eight stage-two **lanes** collapsing to four families, and eight then-completed DragonFly **states** belonging to one family. Neither proves eight acquired families. `report.json` aggregates detail lanes by state, losing family identity; the earlier eight-family durable-caller snapshot is also not an acquisition receipt. |
| Recovered prototype stage-one / parser inventory | **49 kinds / 63 parser keys** | Static enumeration of external `sources.py::_sources` and `parsers/*.py`, inspected 2026-10-02; 48 registered custom modules and 15 unregistered detail/helper/older modules. Four parser modules unconditionally emit no rows. These are implementation keys, not 63 coach providers or proof of original50 membership. |
| Current registry descriptors, all capabilities | **24 = 11 + 10 + 3** | `registry/table/{through_milesplit,from_mshsl,directories}.rs`, chained by `mod.rs::descriptors`; includes results, discovery, artifacts and addresses, not 24 coach families. |
| Coach/contact-advertising registry slugs | **15 / 24** | `ciac, chsaa, coach_contacts, ihsa, ks, pa_piaa, coach_directories, mpa, mshsl, ohsaa, plain_names, riil, wiaa, arbiter_orgs, tssaa`. Capability flags are declarations, not fetched/eligible contacts. |
| Present provider-backed coach implementation families | **13** | The 15 slugs minus importer, collapse three FusionPoint slugs to one, split `plain_names` into two: **15 − 1 − 2 + 1 = 13**. All 13 have coded provider CLI and durable routes; CT within FusionPoint remains CLI-only. Implementation/wiring is not qualification or acquired coverage. |
| Provider CLI coach collector families | **13 through 14 collector slugs** | `ciac, chsaa, ihsa, ks, pa_piaa, coach_directories, mpa, mshsl, ohsaa, plain_names, riil, wiaa, arbiter_orgs, tssaa`; collapse the three FusionPoint slugs and split `plain_names`. The separate CSV importer is excluded. CLI availability is not durable execution. |
| Durable teams coach families | **13 through 13 coach slugs** | `wiaa, mshsl, plain_names` (NDHSAA + NSAA), `ihsa, ks, coach_directories, arbiter_orgs, ohsaa, mpa, riil, pa_piaa, chsaa, tssaa`; collapse the two durable FusionPoint slugs and split `plain_names`. Static applicability reaches **28 / 49 jurisdictions**, not fetched contacts. CIAC remains CLI-only. |
| DragonFly association selectors | **51 = 49 + 2 excluded** | `coach_directories/survey.rs::ASSOCIATIONS`; a selector is not a separate implementation. |
| DragonFly historical staff-publishing / registered associations | **15 / 49** | `survey.rs::VERIFIED` and `coach_directories/mod.rs::REGISTERED` agree on the 15 states; the former's school/page/sample constants are historical, not executed-command evidence. |
| Historical coach lane tier-1 TF/XC-name jurisdictions | **8 / 51 historical surveyed locations** | `coach-directories-national/coverage.json::counts.coach_name_available_tier1`; this is **jurisdictions**, not the reported eight provider families. Historical tier-1 coach-email jurisdictions are **3 / 51**. Do not silently rebase those counts onto a new run. |
| Current executed postal qualifier | **1 NC school; 2 postal claims; 16 raw coach contexts; 0 athletes** | `var/qualification-postal-sol-20261002-actual-02/qualification.json`; retained 2026-09-27 captures replayed offline, not fresh contact acquisition. Most coach roles are `unknown`; contexts are not 16 eligible/current contacts or 16 distinct people. |
| Current fresh national submissions / acquired coach jurisdictions | **0 / 49 submitted; acquired denominator unmeasured** | No current fresh49run exists. Never-submitted is not failed, successful-empty or evidence that a jurisdiction has no coaches. |

The **28 static durable-applicable jurisdictions** are AL, AR, CO, DC, DE, GA, ID, IL, KS, KY,
MD, ME, MN, MS, MT, NC, ND, NE, NH, NM, OH, PA, RI, SC, TN, WI, WV, WY. None is thereby
certified complete. Exact slugs, family mappings and applicability keys are retained in the
[keyed inventory evidence](../../var/source-keyed-inventory-sol-20261002.json).
The other nine registry descriptors are `athleticlive, athleticlive_athletes, athleticnet,
milesplit, tfrrs, wiaa_results, wayzata, nces, state_ed`: they advertise no coach-directory capability.
`ihsa_tournament`, `wiaa_results`, `milesplit_results` and AthleticLIVE modes are not extra
coach families. `private_assoc`, NCES and state-ED school-address readers and `directory` helpers
do not supply a missing coaching roster; the private-association saved-file caller is
`census-service/src/school_address/read.rs::lanes`, not a coach acquisition arm.

### Common family, wiring, status and evidence keys

**Wiring:** `D` = provider CLI + durable teams caller; `C` = provider CLI only, no durable coach
stage; `L` = library implementation without service acquisition caller; `H` = historical/report
obligation without a present matching collector; `I` = saved CSV import only.

**Evidence status (always source- and date-scoped):** `E` = historical captured/acquired evidence,
bounded to the named sample/report; `F` = historical failed/refused acquisition at the named surface
(HTTP/TLS/auth/then-policy), not a negative coach census; `U` = unqualified for coach intake
(schema/roles/scope/caller obligation unresolved); `?` = unknown, no successful coach evidence for
that surface; `Q` = executed current offline postal qualifier only; `N` = never submitted to a
**current fresh49run**. These keys are orthogonal: a family can be implemented `D`, historically
`E`, currently unexecuted `?`, and fresh-run `N` simultaneously. An old robots refusal is historical
policy evidence; Architecture §6 now owns pacing/admission. It is not an instruction to revive the
retired policy or an authorization to bypass authentication/challenges.

**Report locators:** all relative report paths below are under `research/sources/`.
`CN/ST` means `coach-directories-national/SOURCE_REPORT.md` heading `ST` plus
`coach-directories-national/coverage.json#/jurisdictions/ST` (particularly `evidence`, `tier1.sample`,
`tier1.url`, `baseline_rows_verified/refused`); capture paths there are relative to that lane.
`GL` = `state-assoc-greatlakes`, `MA` = `state-assoc-midatlantic`, `PL` = `state-assoc-plains`,
`SE` = `state-assoc-southeast`, `SC` = `state-assoc-southcentral`, `MT` = `state-assoc-mountain`,
`WC` = `state-assoc-westcoast`, `NA` = `national-aggregators`; each key denotes its
`SOURCE_REPORT.md` state/provider section and `coverage.json` state/provider entry, with exact
capture URLs/dates in its `samples/CAPTURES.md`. `MA` JSON uses `jurisdictions_detail`, `PL`/`MT`
use `jurisdictions`; `SE`/`SC`/`WC` have top-level state entries. `GL` is a consolidation of older
Midwest evidence, not a new fetch. Contradictory lane verdicts stay attributed, not averaged.

**Production path locators:** `P` = `crates/census-service/src/cli/provider.rs::run_provider`;
`A` = actual children `cli/provider/arms/{association_sources,native_associations}.rs`
and reexports in `arms/mod.rs`; the native association child supplies CHSAA and TSSAA;
`T` = `crates/census-service/src/restate_services/teams_arms.rs::{TEAMS_ARMS,team_source}`;
`J` = `restate_services/jurisdiction.rs::DISPATCHED`. Applicability is the actual
`crates/census-crawl/src/applicability/table/data.rs`, with the four-org Arbiter state constant in
`crates/census-crawl/src/applicability/table.rs::ARBITER`.
These are source-read locators, not assertions that any binary ran them.

### Present coach implementations, deduplicated

All crawl implementation paths in this table are under `crates/census-crawl/src/`.
`A::<slug>_report` below calls that adapter's `collect` unless explicitly noted.
No listed family establishes present tenure, accepted school/program identity or an allowed
recruiting mailbox merely by emitting a canonical coach row.

| Family key / provider | Present slugs, implementation and actual callers | Applicable scope / wiring | Role, sport, side and acquisition limits | Exact historical evidence locator |
|---|---|---|---|---|
| `DF` DragonFly | `coach_directories/{mod,collect,map,row,survey}.rs`; P → A::coach_directories_report → collect; T::walk_coach_directories, J | 15 registered states; D | Team-owned XC/indoor/outdoor and boys/girls; unlabeled/Mixed/Unified becomes mixed, not proof of both sides. Head/assistant titles recognized; other titles emit Unknown; AD also emitted. Census level/person/vendor admission precedes dedup. 1,000 directory rows/page; 64-page ceiling; failed/malformed summaries do not complete owners. | CN “DragonFly Athletics association platform (measured 2026-09-29)”; recovered prototype `notes/dragonfly.md::{The 2026-09-29 survey,Resulting artifacts,Representative captures}`, `out/dragonfly_probe.json`, `out/extra/<ST>-dragonfly.jsonl` (paths located, staff rows not opened). Repo `tests/fixtures/coach_directories/PROVENANCE.json`, directory/summary pair and `probe/{AL,GA,WY}/PROVENANCE.json` retain bounded samples, not national current yield. |
| `AR` Arbiter organisation API | `arbiter/{mod,collect,map,parse}.rs`, `collect/`; P → A::arbiter_orgs_report → collect; T::walk_arbiter_orgs, J | NH 2132, KY 2507, MT 4497, WV 4223; D | XC/indoor/outdoor, recognized head/assistant roles, non-varsity/ski excluded; absent side becomes mixed, not both. Member primary-contact role is parsed separately. Captured coach API shape does not prove an email field. Token/source refusal explicit; 200 rows/page, 64-page ceiling. OK 106940 is not registered. | `tests/fixtures/arbiter/PROVENANCE.json`: real NH `nh_children_p1.body`, `alvirne_coaches_p1.body`, `coaches_empty.body`, `nh_all_coaches_p11.body`; bundle/token fixtures are **derived-redacted**, `not_json.body` **synthetic**, not additional acquisitions. `arbiter/README.md` cites four historical prototype records (2,932 rows), but repo capture manifest substantiates NH specifically; KY/MT/WV per-org raw mapping remains owed. |
| `IL` IHSA | `ihsa/{collect,map,staff}.rs`; P → A::ihsa_report; T::walk_ihsa, J | IL; D | AD (not assistant AD), head/assistant/Unknown coach titles; XC/indoor/outdoor from title, side explicit or mixed. Staff2 per school, email reveal per person; HasEmail is not itself an address. | CN/IL; GL §2; CN `samples/validate/IL_Abingdon_Avon_0101_staff2.html`, `IL_0101_staff_40964_email.json` (separate revealed-address capture). |
| `KS` KSHSAA | `ks/collect.rs`, `ks/{parse,wire}.rs`; P → A::ks_report; T::walk_ks, J | KS; D | **AD-only**, no sport, mixed; per-sport coaching absent in captured JSON. School/JH directory rows are not a TF/XC program denominator. Personal mobile columns not a contact-coverage claim. | CN/KS; PL/KS; CN `samples/api/KS__directory_a.json` and `tools/validation-ks.json`; historical 526 single-letter/baseline rows and schema's 746 A–Z directory universe are different measurements, not current counts. |
| `WI` Wisconsin WIAA | `wiaa/{collect,collect_schools,map}.rs`; P → A::wiaa_report; T::walk_wiaa, J | WI only, not Washington; D | AD; head/assistant/Unknown coaching; XC/indoor/outdoor; side from label, absent side Unknown. Published school contacts incl. decoded email; index and school details bounded by collector. | CN/WI; GL §6; CN `samples/validate/WI_Abbotsford_orgID_1_coach_AD.html`, `WI_Madison_East_orgID_219_coach_AD.html`. |
| `MN` MSHSL | `mshsl/{collect,map,teams}.rs`, `collect/`; P → A::mshsl_report; T::walk_mshsl, J | MN; D | AD/assistant activities-director categories; team-page XC/indoor/outdoor and boys/girls/mixed; published coaching levels filtered. Historical school-page AD-name/no-email capture is distinct from team-coach endpoint evidence. | CN/MN `samples/validate/MN_albany.html` etc.; PL/MN report/coverage school + team endpoint recipes. CN schema `mshsl_school_html` has no sport coaches or AD emails; do not reinterpret it as the later team endpoint. |
| `ND` NDHSAA | `plain_names/{nd_walk,nd_coaches,nd}.rs`; P → A::plain_names_report; T::walk_plain_names, J | ND; D | AD names; XC/indoor/outdoor names, side from label or mixed; sport-offering rows emit **Unknown role**, not inferred head coach. No email field. | CN/ND; PL/ND; CN `samples/validate/ND_Minot_North_1307.html`, `ND_Bismarck_Legacy_1131.html`, `ND_West_Fargo_Sheyenne_1045.html`. |
| `NE` NSAA | `plain_names/{nsaa_walk,nsaa_coaches,nsaa}.rs`; same P/A/T/J as ND | NE; D | AD and mapped head-coach names; XC/Track (mapped outdoor), boys/girls/mixed; Unified excluded. No email field; indoor-specific scope not established by this mapper. POST school selector is name-valued, not a numeric identity guarantee. | CN/NE; PL/NE; CN `samples/validate/NE_nsaa_Adams_Central.html`, `NE_nsaa_Gothenburg.html`, `NE_nsaa_Omaha_Gross_Catholic.html`, `tools/validation-ne-wholesale.json`. |
| `OH` OHSAA / myOHSAA | `ohsaa/{collect,map,pages,parse}.rs`, `collect/`; P → A::ohsaa_report; T::TeamsSource → associations::ohsaa; J | OH; D | Matching published school owner and structured sports/AD tables are required. Sports and AD appointments retain their own actual capture URL/time/full digest; office slots are not source person IDs. Changed captures reach content-bound fact guards; full completion binds both URLs/digests, school owner and projection revision, not evaluation/acquisition restamps. HTTP-200 refusal/malformed/foreign-owner pages remain failed or partial; only matching-owner labelled N/A/TBA certifies AD absence. No current-tenure, assistant-role or indoor-specific qualification claim. | CN/OH; GL/OH; CN `samples/validate/OH_Centerville_336_TF_coach.html`, `OH_Dublin_Coffman_474_AD.html`; namespace `myohsaa`. Integrated 38-test regression PASS on 2026-10-02; current live search transport refused, zero facts. See `docs/VERIFICATION-EVIDENCE.md`. |
| `FP` FusionPoint directories | `ciac/`, `mpa/`, `riil/` collectors/maps; P → A::{ciac,mpa,riil}_report; T::TeamsSource → associations::{mpa,riil}; J; CIAC remains CLI-only | CT/C; ME, RI/D. NV historical template is not registered | XC/indoor/outdoor named rows mapped HeadCoach; no email column; historical AD fields are not emitted by these sport-coach mappers. CIAC role cell is discarded during parse. MPA `Coed` maps Mixed, explicit Boys/Girls remain distinct; its school/directory and coach/staff captures retain independent provenance. RIIL retains phone, actual directory URL/time/full digest, and school/content-bound atomic receipts; identical replay is stable and changed content retains new evidence. HTTP-200 refusal/schema failures remain unfinished, not successful vacancy. No invented person IDs, mailbox, postal or current-tenure claims. | CN/{CT,ME,RI}, `schema.json::families.fpsports_school_directory_html`; CN `samples/dir/{CT,ME,RI}__directory.html`. Historical ME/RI 543 staged rows and CT expired-TLS refusal are not current coverage. Current bounded ME replay: `var/qualification-mpa-sol-20261002-01/out/qualification.json`, no requests/current tenure. Actual RIIL fresh directory qualification: `var/qualification-riil-live-sol-20261002-01`, 55 schools/258 appointments/one request/zero errors, verified persisted capture binding; see dated `docs/VERIFICATION-EVIDENCE.md`. |
| `PA` PIAA | `pa_piaa/{collect,map,parse}.rs`; P → A::pa_piaa_report; T::TeamsSource → associations::piaa; J | PA; D | **AD-only**, no sport, mixed; no TF/XC coach rows or officials acquisition. Linked A–W + Y letters, details by ID; Z echoes A. Scope does not convert letters/detail samples into statewide coverage. | MA/PA Rust-port + integration sections; `tests/fixtures/pa_piaa/PROVENANCE.json`, `directory_alpha_a.html`, `directory_alpha_b.html`, `details_12048.html`; historical 2026-10-01 live A-letter/detail qualification in `docs/VERIFICATION-EVIDENCE.md` “PIAA live directory read, replay lane and index repair smoke”. |
| `CO` CHSAA / CHSAANow | `chsaa/{collect,map,parse}.rs`; registered `chsaa`, exported by crawl `lib.rs`; P → A::chsaa_report → collect; T::TeamsSource → associations::chsaa; J | CO; D | Recognized head/assistant names; XC or Track (mapped outdoor), boys/girls/mixed; no email. Captured activity data is richer than old staff-tab shell. Indoor distinction and current tenure unqualified. | MT/CO plus `tests/fixtures/chsaa/PROVENANCE.json`: `directory.html`, `school_cherry_creek.html`, `robots.txt`; 378 school entries and 50 prototype coach rows are historical fixture measurements, not current contacts. |
| `TN` TSSAA native reader | `tssaa/{collect,parse,fields,map,staff_year}.rs`; P → A::tssaa_report; T::TeamsSource → associations::tssaa; J | TN; D | Public indexed-school discovery and owner-checked detail acquisition; head/assistant/AD exact titles, XC or Track (outdoor), side from cards, published obfuscated mail, separately labelled mailing/physical/shipping postal claims. Current tenure requires the complete matching-school published consecutive staff year plus agreeing administration header; timestamp/run year never substitutes. Missing context remains unknown, malformed/foreign/conflicting context remains partial without success receipt; revision `tssaa_schools_v2`. | `tests/fixtures/tssaa/SOURCE.md`, `directory_id3.html`, `directory_id407.html`, `robots.txt`; historical 456 pages/14,962 all-staff rows is not eligible contact coverage. Actual Page High School qualification: `var/qualification-tssaa-live-sol-20261002-02`, one school/two requests/11 appointments/zero errors, persisted explicit 2026–2027 tenure and three postal claims verified; see dated `docs/VERIFICATION-EVIDENCE.md`. |
| `CSV` imported contact artifact (not a provider family) | `coach_contacts/import.rs::import_csv`, `wire.rs`, `entities.rs`; P → A::coach_contacts_report requires `--input`; `cli/gather.rs` also calls import_csv; no T/J | Historical applicability table names 22 states; I | Original source URLs/last_observed/role/sport/side must be inspected; repeated imported providers never count twice. Import reuses historical evidence, not fresh discovery. Rows without exact role/time/program support remain unqualified contacts. | CN baseline validation: 2,298 artifact rows, `tools/{baseline-validation,validation-ks,validation-ne-wholesale,validation-retractions}.json`; applicability `data.rs` exact 22-state list. Bound retractions and MN AD-email provenance gap survive import. |

### Historical-only / missing acquisition obligations

These are not extra counted implementations. `AS/ST` identifies the native association/report
surface for a state with no matching current coach collector; it is not a fabricated adapter slug.
`DF:U` for the other 34 scoped selectors means the historical probe sampled four summaries with
no staff and the collector does not register them. It is not evidence of no coach anywhere in that state.

| Family / obligation key | Applicable jurisdiction and historical evidence | Present disposition and role/sport/side limit |
|---|---|---|
| `HC` Home Campus / widget | FL: SE `samples/{homecampus-school-directory.html,homecampus-school-details-2397.json,homecampus-schools-get.json}`; recovered prototype `notes/fl-fhsaa.md` (2026-09-27), `notes/ca-cif.md` (2026-09-27), `notes/verification.md` §3 | H/E/U: no matching current coach collector/caller. The SE FL sample has one named AD and five null-name coaches; the separate prototype report records 336/879 successful detail IDs, 1,153 AD and 552 TF/XC posts, with 543 HTTP405 gaps. CA section lists alone are discovery; prototype section detail JSON does name TF/XC coaches with side/title/email. Keep those later captured surfaces distinct from earlier null/school-only samples; no fresh or statewide qualification. |
| `GA-PDF` GHSA native directory | GA: SE `samples/ghsa-directory-feed.pdf`, `coverage.json::GA.enumerates.coach_directory` | H/E: school-scoped sport codes 5/14, B/G and head marker `*`; one school-level general email, not per-person mailbox. No current PDF coach collector; keep separate from DF/GHSA. |
| `AZ-AIA` AIA native details | AZ: MT `samples/aia-school-detail-58.html` | H/E: AD and sport-scoped coach names/phone, **no email**; no current AIA coach collector. |
| `ID-AD` Idaho native directory | ID: MT `samples/idhsaa-directory.html` | H/E: AD names/base64-obfuscated email, not sport coaches; old CN challenge is F at another capture/client. DF is a separate registered path. |
| `UT-UHSAA` Utah native directory | UT: MT `samples/{uhsaa-school-directory.html,uhsaa-school-alta.html}` | H/E: AD + per-sport coach names/mailto at one detail sample; no matching current collector; no statewide executed contacts or indoor/tenure guarantee. |
| `OR-OSAA` Oregon native details | OR: WC `samples/{osaa-schools-full-members.html,osaa-school-347.html}` | H/E: AD/admin email plus activity HeadCoach table, not proof of coach-specific email for every activity; no current OSAA coach collector. |
| `NJ-AD` NJSIAA native members | NJ: MA `samples/njsiaa-member-info.html` | H/E: first-page AD names/phone/address, no per-sport coaching; full pagination and mailbox support unqualified. |
| `BD` Bound directories | IA, SD: PL coach section versus CN `tools/validation-retractions.json`, `samples/validate/{IA_Bound_adm.html,SD_Bound_aberdeencentral.html}`, `tools/robots/gobound.com.txt` | H/F/U: older browser-derived contacts retained as prior-study evidence; coach lane withdrew its validation claims and captured 403/refusal. No matching collector/durable coach caller; school/season registry counts are not contacts. |
| `MI-AD` Michigan myMHSAA | MI: GL coach/AD endpoint citation; CN `coverage.json#/jurisdictions/MI/baseline_rows_refused`, `tools/robots/my.mhsaa.com.txt` | H/F: older AD-name/email reporting versus later refusal; no current native collector (CSV namespace recognition is not a fetcher). |
| `FF` FinalForms | WA: CN `samples/dir/WA__state_schools.html`; WC `samples/wiaa-finalforms-state-schools.html`. NV: WC `samples/niaa-finalforms-state-schools.html` | H/U: school addresses/offerings; WA generic “Contact Info” mailbox has **no labeled person/role**, not AD/coach. NV coach landing empty-202 is F, not a verified coach directory. |
| `LA-PDF` LHSAA directory | LA: SC `samples/{la-coaches-directory-2025-26.pdf,la-coaches-directory-2025-26.txt}`; recovered prototype `la_lhsaa_ocr.py::{parse_school_blocks,map_codes}`, `notes/la-lhsaa-ocr.md`, `raw/www.lhsaa.org__588515f1fbdf2264d59d61ea`, `out/extra/LA-lhsaa.jsonl` | H/E(bytes, historical OCR report)/U: the registered `la_lhsaa_coaches` URL is uncached/failed; a different preserved 2025–26 PDF has an OCR report of 292 containers/1,953 rows. The OCR note's uncertain `h`/`*` interpretation conflicts with current prototype `map_codes` head/assistant mapping; TK/TKI collapse to Track, both sides collapse to empty. No qualified current collector, exact indoor split or tenure claim. |
| `AR-other` unsupported Arbiter delegation | MA, OK, WA: CN delegation/robots entries; WC WA AD-center; OK 106940 noted in `arbiter/README.md` | H/U/F: not in four-org current API registration; old delegated-host refusal and an empty OK candidate member page do not establish statewide absence or another API family. Missing qualified org/capture/caller mapping. |
| `AS/ST` remaining native associations | State-specific CN/ST and regional reports below; recovered prototype key and auxiliary tables retain later public detail/report evidence | H/? or U/F unless specifically E in the recovered table: school list, shell, office/committee contact or resource hub is not a school TF/XC coaching roster. DC native details, MD AD details, MS/SC Knack, DE AD and coaching-association artifacts are separate historical obligations, not additional current13 families. |
| `NA` national/result aggregators | NA `SOURCE_REPORT.md` coach/contact fields for DAT, TFRRS, AthleticLIVE, RunnerSpace/DyeStat, MaxPreps; `coverage.json::per_source` | Not coach families: DAT/TFRRS/AL/RunnerSpace captures expose no public coach contacts; MaxPreps `samples/maxpreps/mp-aledo-staff.html` is an unqualified staff capture (path relative to NA as recorded by its report). MileSplit registry advertises no coach-directory capability; validators/registration pages do not discharge contact acquisition. |
| `legacy50` / `acquired8` | Reported counts retained; recovered prototype mappings and dated unit reconciliation immediately below | All available 63 parser keys, stage-one source-ID generators and identified auxiliary public lanes are accounted for. Still missing: original50's owning manifest/revision/counting rule, and acquired8's exact eight named families with run-bound successful capture/effect receipts. Located lane files, report counts, module counts and durable caller counts do not substitute for either manifest. |

### Recovered original implementation keys and callers (static recovery 2026-10-02)

**Historical evidence root `CP`:** `/home/lewis/src/ad-law-scrape/census-prototype/`, external
to this Rust workspace. Code, public acquisition report schemas/counts and preserved capture
metadata were read, never executed. No private spreadsheets, athlete population files, workbook
rows or `out/extra/*.jsonl` staff payloads were opened/imported. A listed detail file is a located
historical artifact, not a newly checked row set. Dates below are the report's stated historical
scope, not file mtimes or an invented fetch timestamp; `CP/out/report.json` has no run date/revision.

**Original caller contract:** `CP/sources.py::_sources` constructs source IDs, URLs, `kind`, `role`
and evidence class; `CP/run.py::attempt` passes `kind` to `CP/extract.py::extract` (lines 273–299).
`custom:k` imports `CP/parsers/k.py::parse`; despite the stale source-table docstring, actual
parsers return a dictionary, not a `(schools, coaches)` tuple. `R` below means registered
stage-one custom key. `X` means present module but not registered in `_sources`; it can be a
stage-two parser, helper or older unwired variant, not a missing provider inferred from its name.
`S2` means `CP/deepen.py::fetch_one` → `extract("custom:<parser>", body)` with an explicit parser
argument, URL field and label; it writes URL/status/bytes/join/schools/coaches containers.
`CP/run.py::load_extras` loads successful containers and creates `detail:<file-stem>` identities;
`main` then emits **`detail:<state>` aggregate report rows with empty URL and `kind: detail`**.
Thus aggregate state coach counts cannot recover acquired family lineage. `CP/lane_check.py`
checks retained merge keys/sides; it is not an acquisition caller or current qualification.

**Table contract:** each bare parser key means exactly `CP/parsers/<key>.py::parse` unless a
different function is named. Every one of the **63 nonempty modules** appears once below.
`S` = schools/addresses/classification/offerings only, no coach output; role/sport/side therefore
not applicable. `T` = prototype `Track`, which generally conflates indoor/outdoor; `XC` =
`CrossCountry`; blank gender is unknown/mixed, never proof of both sides. Head/assistant/AD
spelling describes the historical parser, not accepted recruiting eligibility. Current family
keys and D/C/L/H wiring are those of the current13 table, not claims of semantic parity.
`RPT/<id>` means exact `CP/out/report.json` row selected by `source_id`; generated IDs are fully
specified below. `AUD/<id>` means the same source's row in `CP/notes/verification.md` “Stage-one
name audit (2026-09-27)” table; that audit proves reported name-byte provenance, not correct role
or tenure. `E(S)` denotes captured school evidence only. Supplemental prototype evidence does
not overwrite conflicting earlier regional samples or change any current fresh-run N.

| Original implementation keys | Registered IDs / actual historical caller | Family / present coach wire | Historical role, sport, side and evidence disposition / exact locator |
|---|---|---|---|
| `dragonfly_directory` R; `dragonfly_school` X | `_sources` lines 95–105: `<st>_dragonfly` for the 15 staff associations, plus GA `_p2,_p3`, MS/SC/TN `_p2`; S2 summary via `detail_url`; `dragonfly_probe.py::probe` calls both parsers | DF/D | Directory S; summary head/assistant/Coach + AD, T/XC, Boys/Girls or blank, team level retained. E: `notes/dragonfly.md` (2026-09-29), `out/dragonfly_probe.json`; all 15 named `out/extra/<ST>-dragonfly.jsonl` located. Four-summary survey is not state completion. |
| `arbiter` X | `arbiter_crawl.py::{main,paged}` calls `parse_schools/parse_coaches`, org/state/label arguments; not `extract::parse` | AR/D for four registered orgs | AD member contact; head/assistant T/XC, side from sport suffix, non-varsity excluded. E: `notes/arbiter.md::Endpoints observed` NH2132 raw children/coach capture locators; `notes/verification.md` §3 names KY/MT/NH lanes. `out/extra/{NH,KY,MT,WV}-arbiter.jsonl` located; WV4223 auth failure in `notes/wv-wvssac.md` is an earlier surface. OK org106940 remains unregistered/U. |
| `ct_ciac`, `me_mpa`, `ri_riil` R | `ct_ciac_fpsports,ct_ciac_directory`; `me_mpa,me_mpa_directory`; `ri_riil,ri_riil_directory`; RPT/AUD for each | CT FP/C; ME, RI FP/D | School list branch S; directory branch AD + explicit head/assistant T/XC with side/phone, no email. E: AUD CT 1,287, ME 472, RI 349 historical directory staff rows (not all eligible TF/XC). Current Rust mappers have separate role/side limitations; parser-file count three is one family. |
| `co_chsaanow`, `co_school` R | `co_chsaanow,co_school`; S2 `detail_url`, located `out/extra/CO-schools.jsonl` | CO/D | Index S; activity coaches head/assistant, T/XC and prefix side, no email. E: AUD `co_school` 50 sample rows, `notes/co-chsaa.md`; current Rust collector has coded CLI and durable service callers, not executed statewide qualification. |
| `il_ihsa`, `il_ihsa_staff` R | `il_ihsa_schools,il_ihsa_staff`; S2 staff2 with school join, `out/extra/IL-schools.jsonl` | IL/D | Index S; prototype `_split_title` emits HeadCoach or AD only, T/XC with label side; **no assistant rows**, RoleID/HasEmail not an address. E: AUD `il_ihsa_staff` six rows; `notes/il-ihsa.md`; separate Rust revealed-email capture and richer role mapping in current13 table. |
| `wi_wiaawi` R; `wi_wiaawi_school` X | `wi_wiaawi_schools`; S2 school details, `out/extra/WI-schools.jsonl` | WI/D | SearchOrg branch S; details AD/head/assistant T/XC side, published mailto/decoded mail. Historical AD roles use HeadCoach/AssistantCoach with AD sport, not literal coaching posts. E: `notes/wi-wiaawi.md`, AUD index; not WA. |
| `mn_mshsl`, `mn_mshsl_school` R | `mn_mshsl`, `_p2`…`_p14`, `mn_mshsl_school`; S2 `detail_url`, `out/extra/MN-schools.jsonl` | MN/D | List S; prototype `ADMIN_LABELS` recognizes activities director and boys/girls sports representatives, all emitted as AD sport/role; **no assistant-director category or team coaches**. Published phone/decoded mail may be present. E: AUD and `notes/mn-mshsl.md`; current Rust team endpoint is separate, not recovered from school-page totals. |
| `nd_ndhsaa` R; `nd_ndhsaa_school` X | `nd_ndhsaa` labeled coaches but actually list S; S2 `detail_url`, `out/extra/ND-schools.jsonl` | ND/D | Details AD and offering names; historical `_offering_rows/parse` infers HeadCoach for offering rows and Head/Assistant for AD sport; current Rust preserves Unknown sport-role. T/XC with label side, no email. E(sample): CN/ND retained Minot/Bismarck/West Fargo detail captures in current13 table; CP lane located, not row-checked; `notes/dragonfly.md` records separate ND DF evidence. |
| `ne_nsaa` R | `ne_nsaa` labeled coaches; parser reads GET select options only | NE/D | S, **no POST details or coaches in this module**, despite the source note. E(S): AUD `ne_nsaa`; `notes/ne-nsaahome.md` records further endpoint study. Rust `plain_names` NSAA POST coach reader is a later implementation, not evidence of a prototype coach parser here. |
| `oh_ohsaa`, `oh_myohsaa` R | `oh_ohsaa_enrollment,oh_myohsaa_sports,oh_myohsaa_ad`; S2, `out/extra/OH-schools.jsonl` | OH/D | Enrollment S; details infer HeadCoach from sport row, AD separately but historical AD role also HeadCoach; T/XC, explicit Boys/Girls, mailto. E: AUD and `notes/verification.md` §3; `_sources` notes old `/Outside` route404 separately from saved captures/current collector routes. |
| `ks_kshsaa` R | `ks_kshsaa` | KS/D | AD-only; historical `sport: AthleticDirector` with fallback AssistantCoach role is not an assistant TF/XC appointment; no side. E: RPT/AUD, `notes/ks-kshsaa.md`; current Rust correct AD taxonomy stays distinct. |
| `pa_piaa`, `pa_piaa_details` R | `pa_piaa`, `_b`…`_w`, `_y` (no X/Z); `pa_piaa_details`; S2 `detail_url`, `out/extra/PA-schools.jsonl` | PA/D | Index S; detail AD-only, role from contact title, no sport/side. E: AUD/RPT `pa_piaa_details`, `notes/pa-piaa.md`; not officials or school sport coaches. |
| `homecampus` X; `cif_directory` R; `cif_school` X; `fl_fhsaa_widget`, `fl_fhsaa_api` R; `fl_fhsaa_profile`, `fl_fhsaa_school`, `fl_fhsaa` X | `homecampus::{parse_directory,parse_school}` shared helper; CA generated `ca_{cifss,cifncs,cifsjs,cifsds,cifcs,cif_la}_directory`; FL `fl_fhsaa_widget,fl_fhsaa_api`; S2 `cif_school/fl_fhsaa_profile`, XHR; older FL school/list variants not registered | HC/H | Lists S; shared details published head/assistant/AD T/XC, comma-label side, non-varsity filtered. Older `fl_fhsaa_school` assumes head/assistant defaults and lacks side, not additional lineage. E/U: `notes/{ca-cif,fl-fhsaa}.md` dated 2026-09-27, `notes/verification.md` §3; `out/extra/CA-cif*.jsonl`, `FL-schools*.jsonl` located. CA/FL use one family, not eight modules/families. |
| `ga_ghsa` R | `ga_ghsa_pdf`; PDF parsing `parse_pdf_text/_ghsa_stream`; `out/extra/GA-ghsa.jsonl` located | GA-PDF/H | AD/head/assistant sport codes T/XC with B/G, school email not person mailbox. E: AUD/RPT `ga_ghsa_pdf`, `notes/ga-ghsa.md`; distinct from DF/GHSA. |
| `az_aia`, `az_aia_school` R | `az_aia_search`, `az_aia_q_a`…`_z`, `az_aia_school_58`; S2 `detail_url`, `out/extra/AZ-schools.jsonl` | AZ-AIA/H | Search S, 20-row cap/query; details head/assistant/Coach/AD T/XC label side, telephone only. E: AUD/RPT, `notes/az-aia.md`; generic `Coach` is not inferred head status. |
| `ia_iahsaa` R; `ia_school` X | `ia_iahsaa`; S2 `detail_url`, `out/extra/IA-schools.jsonl` | Native IA school discovery H; BD separate H | Both S, expose school page/Bound slug, no coaching output. E(S): AUD, `notes/verification.md` §3 distinguishes `IA-bound`; identity bridge not itself an acquired coach family. |
| `md_mpssaa`, `md_mpssaa_school` R | `md_mpssaa,md_mpssaa_school`; S2, `out/extra/MD-schools.jsonl` | AS/MD native H (not DF) | List S, school details AD-name only, no T/XC or side. E: RPT/AUD, `notes/md-mpssaa.md`; earlier regional login captures remain separate. |
| `nj_njsiaa` R | `nj_njsiaa`, `_p1`…`_p8` | NJ-AD/H | AD-name/phone/address, no sport/side/email; historical role mislabeled HeadCoach with AD sport. E: RPT/AUD, `notes/nj-njsiaa.md`; nine pages in prototype registration do not alter earlier regional first-page scope. |
| `or_osaa` R; `or_osaa_school` X | `or_osaa`; S2 `detail_url`, `out/extra/OR-schools.jsonl` | OR-OSAA/H | List S; detail `parse/_table_by_header` emits AD/head-coach names T/XC, B/G side, admin email distinct from coach name. E(sample): WC/OR `samples/osaa-school-347.html` in historical obligations table; CP stage-one AUD and located detail lane do not certify statewide staff. Current collector absent. |
| `ut_uhsaa` R | `ut_uhsaa` | UT-UHSAA/H | **S only**, `data-school` grid; no coach parser in this key. E(S): RPT/AUD, `notes/ut-uhsaa.md`; regional Alta detail E is a different captured surface, not an implemented detail walker. |
| `tn_tssaa` R; `tn_tssaa_school` X | `tn_tssaa` labeled coaches but list S; S2 `detail_url`, `out/extra/TN-tssaa.jsonl` | TN/D | Details explicit head/assistant/AD, T/XC header side, decoded mail; assistant AD/system directors excluded by prototype. E: `notes/tn-tssaa.md::Sport mapping fix (Main, 2026-09-27)` 456 successful pages, 2,769 rows = 2,272 TF/XC + 497 AD. Earlier 14,962 all-staff rows were unclassified, not qualified coaches. |
| `la_lhsaa` R | `la_lhsaa_coaches` | LA-PDF/H | Registered URL F: RPT `RuntimeError: offline and not cached`; parser regex **guesses Track + HeadCoach**, no side. U even if generic text matched; separate OCR/correct PDF obligation below, not successful execution of this key. |
| `in_ihsaa` R | `in_ihsaa` | AS/IN H discovery | S: membership-history PDF current-member/city/spans, no coaches. E(S): RPT/AUD, `notes/in-ihsaa.md`; 413 current members among 1,258 historical rows is not a coaching population. |
| `ky_khsaa` R | `ky_khsaa` | Native KY H discovery; AR separate D | S: published membership CSV, addresses/classes/district, no coaches. E(S): RPT/AUD; no spreadsheet body opened in this recovery. `KY-arbiter` is the distinct coach acquisition lane. |
| `ok_ossaa` R | `ok_ossaa_pdf` | AS/OK H discovery; AR-other separate U | S: member-school/address PDF, no coaches. E(S): RPT/AUD, `notes/ok-ossaa-pdf.md`; neither a public school PDF nor the separate org106940 candidate qualifies an Arbiter coaching roster. |
| `ma_miaa` R | `ma_miaa_members` | AS/MA H discovery | S: school/address PDF. E(S): AUD, `notes/ma-miaa-members.md` dated 2026-09-27. MSTCA membership evidence below is neither this implementation nor MIAA coach qualification. |
| `mi_mhsaa_enrollment` R; `mi_mhsaa` X | `mi_mhsaa_enrollment`; older JSON/PDF enrollment module has no `_sources` registration | MI native H discovery (not MI-AD fetcher) | Both S, enrollment/ID/class; no coaches. E(S): AUD registered enrollment, `notes/mi-mhsaa.md` dated 2026-09-27; X module's claimed JSON shape not proven by registration. MITCA below is distinct. |
| `mo_mshsaa_wayback` R | `mo_mshsaa_wayback` | AS/MO H discovery | S: 2025 archived member listing, no coaches. E(S): RPT/AUD; source policy failure at live `mo_mshsaa` stays F, neither is fresh2026 coach evidence. |
| `nc_nchsaa` R | `nc_nchsaa` | AS/NC H discovery; DF separate D | S: school/classification/conference, no coaches/detail links. E(S): RPT/AUD, `notes/nc-nchsaa.md`; admin/conference contacts do not supply school coaching posts. |
| `nm_nmact` R | `nm_nmact` | AS/NM H discovery; DF separate D | S: map markers school/address, no staff. E(S): RPT/AUD, `notes/nm-nmact.md`; NMAA resource/office page is not coaching scope. |
| `finalforms` R | `nv_finalforms`, `_p2`…`_p9`; `wa_finalforms`, `_p2`…`_p50` | FF/H | S: school address/NCES/enrollment/offerings; no person/role/coach output. E(S): RPT/AUD, `notes/{nv,wa}-finalforms.md`; pagination is not a separate implementation or a TF/XC roster. |
| `wv_wvssac` R | `wv_wvssac` | AS/WV H discovery; AR separate D | School/enrollment HTML/PDF parser, no coaches. No rows at registered classification hub: `out/coverage.md::Sources that did not yield rows`, `notes/wv-wvssac.md` dated 2026-09-27. AR org4223 and office directory are separate obligations. |
| `de_diaa` R | `de_diaa` | AS/DE H; DF separate D | **Unconditional no-op** school/coach lists; registered attempt F/offline uncached, not observed Cloudflare proof. `notes/de-diaa.md::Not established` (2026-09-27) explicitly says the annotation alone claims Cloudflare. Public DE AD script below is another host/surface. |
| `mt_mhsa` R | `mt_mhsa` | AS/MT H; DF/AR separate D | **Unconditional no-op**; source school hub contains no list. Cached body but no rows: `out/coverage.md::Sources that did not yield rows`, parser lines 12–13. No coach census absence inferred. |
| `wy_whsaa` R | `wy_whsaa` | AS/WY H; DF separate D | **Unconditional no-op**; homepage class divisions/counts not named school/staff list. Cached no-yield row in `out/coverage.md`, parser lines 11–13, `notes/wy-whsaa.md`; separate DF school summaries E. |
| `milesplit_teams` R; `milesplit_team` X | `<st>_milesplit` for all49 plus excluded AK/HI; `enrich.py::enrich_one` → `extract("custom:milesplit_team",…)`, `run.py::load_enriched` | MileSplit discovery, not current13 coach family | Both S, discovery-only team index/profile identity/address/offerings; no coach truth. E(S): all state RPT IDs, `coverage.py::build`, `out/enriched-summary.json` locator. Team pages differ from athlete roster acquisition; no athlete payload opened. |
| `athleticnet` R | `us_athleticnet_teams`, evidence `discovery_only` | Athletic.net discovery, not current13 coach family | **Unconditional no-op** schools/coaches/vacancies; browser-gated root not canonical coach field. `out/coverage.md` no-yield row, parser lines 15–16. Athletic.net Rust athlete/results implementation does not prove this old key acquired coaches. |
| `ak_asaa` R | `ak_asaa`, `in_scope=False` | Excluded historical AK discovery | S: member-school/address/enrollment. `sources.py` lines 322–324, `notes/ak-asaa.md`; implementation retained, **excluded**, not an extra national family or N-failure. |
| `bound_staff` X | `bound_crawl.py::{main,crawl,record}` → `extract("custom:bound_staff",…)`; `out/extra/{IA,SD}-bound.jsonl` | BD/H | Parser emits person/printed role; caller supplies school/sport/side from page path, checks school title; varsity T/XC. E(report)/U with separate CN refusal/retractions: `notes/verification.md` §3 IA-bound and §13 “Bound lanes completed (Iowa and South Dakota), and the one-off rate override”; `notes/sd-sdhsaa.md`; KS stub is not a staffed Bound lane. |

**Generic dispatch and source-ID closure:** `html` is the 49th registered stage-one kind,
`extract.py::school_rows_from_html`, always schools-only. Its exact IDs in `sources.py` are
`al_ahsaa,ar_ahsaa,ca_cifstate,ca_cifss,dc_dcsaasports,id_idhsaa,mi_mhsaa_schools,mo_mshsaa,
ms_misshsaa,nh_nhiaa,ny_nysphsaa,sc_schsl,tx_uil,va_vhsl,vt_vpa,us_bound_index`.
`id_idhsaa` is labeled `coaches` and its public page has AD evidence, but generic dispatch emits
**no coaches**; a source `role` label is not parser capability. These IDs are RPT-locatable and
map respectively to native AS/ST/ID-AD school-only or failed obligations (and Bound discovery
for `us_bound_index`), not additional current13 implementations. The six other generic branches
`html_finalforms,json_ihsa,json_kshsaa,json_aia,json_fhsaa,pdf_generic` are present in
`extract.py::extract` but unregistered as kinds in `_sources`, all schools-only; their shared
helpers used by custom modules are not new providers. Empty `parsers/__init__.py` is packaging,
not a 64th implementation. Generated letters/pages/section hosts and 51 state selectors preserve
all registrations without pretending each page/state is another family.

**Removed key recovered from dated notes:** `sd_sdhsaa` / `custom:sd_sdhsaa` is named in
`CP/notes/verification.md` §12 “South Dakota's registered source was dead” (2026-09-27).
Its `/schools` URL returned404, **no parser existed**, and the registration was removed.
Disposition H/F/U, no role/sport/side output; SD's actual delegated Bound lane is mapped above.
It is neither one of the63 available modules nor evidence that adding a conjectural50th kind
reconstructs the reported50 manifest. Historical removed registrations stay distinct from this
snapshot's implemented-key count.

### Auxiliary historical public lanes outside the parser registry

These caller/script keys were also identified in `CP` and are not silently counted among
the 63 parser modules or current13 families. `run.py::load_extras` can consume their converted
containers; that generic ingestion is not a present Rust acquisition arm. Located artifacts
alone retain U; dated source/metadata reports can establish bounded historical E, never freshness.

| Original script / lane keys and caller | Family / current coach disposition | Historical role/sport/side and exact evidence locator |
|---|---|---|
| `sc_knack.py::{schools,staff,map_roles}`, `to_extra.py::LANES/convert`; `SC-schsl-knack`; earlier `SC-knack`, `MS-knack` report lanes | Knack/H, association-specific app ownership; not DF | SC mapper head/assistant/AD T/XC Boys/Girls/blank. `notes/sc-schsl.md::Endpoints observed` proves school519 and earlier staff401, not later success. MS `notes/ms-misshsaa.md` proves separate app61324393 staff7,703 with1,205 TF/XC positions (705 varsity),383 AD; salted raw p1 `api.knack.com__99f0fe5c5f06f9a6fa6de379`…p8 `__962d1798a2b8738a4ef7e9dd`. `notes/verification.md::DragonFly Athletics lane` defect6 records separate Mississippi Knack/California school-site gender contradictions in the merge; no current Rust Knack collector. |
| `dc_directory.py::{school_links,map_role,main}`, `to_extra.py::LANES`; `DC-dcsaa` | Native DC/H, separate from DF | AD + T/XC published side/mailbox, **head status inferred**, not explicit source rank. `notes/dc-dcsaasports.md::Endpoints observed/Detail page/Gaps`: directory54, sample raw `www.dcsaasports.com__275cbddac07dab26bc6c1499`; role distinction unqualified. Earlier MA list-only evidence is not this detail capture. |
| `de_ad_directory.py::main`, `to_extra.py::LANES`; `DE-diaa-ad-directory` | Native DE AD/H, separate from no-op `de_diaa` | AD-only, no sport/side; public education.delaware.gov AD-directory browser table. Script URL/conversion contract and located `out/extra/DE-diaa-ad-directory.jsonl` establish a lane, not successful current acquisition; owning capture manifest/date remains U. |
| `la_lhsaa_ocr.py::{parse_school_blocks,map_codes,main}`; `LA-lhsaa` | LA-PDF/H/U | Historical 2025–26 directory OCR, AD/head/assistant by code; TKI and TK conflate Track, both sides collapse blank. `notes/la-lhsaa-ocr.md` reported1,953 rows/292 containers; contrary prefix assumptions remain explicit. Raw PDF `www.lhsaa.org__588515f1fbdf2264d59d61ea`, `ocr/page-*.txt` locators only; not executed or row-qualified here. |
| `mitca_lane.py::{pdf_pairs,contact_pairs,main}`; `MI-mitca` | MITCA/H/U, **not myMHSAA** | `notes/mi-mhsaa.md::Coach names (added 2026-09-28)` names 2021–23 awards,2022 meet staff and committees; report205 coach rows/141 schools. Historic honors/staff do not establish current tenure; head/assistant/discipline defaults and blank side must not become fresh contacts. |
| `mstca_lane.py::{rows_on_page,main}` → `mstca_to_extra.py::main`; `mstca`, `MA-mstca` | MSTCA/H/U, **not MIAA or Arbiter** | `notes/ma-miaa-members.md::Coach names (added 2026-09-28)` reports1,417 active members,415 school labels,382 joined rows/149 schools; `out/evidence/mstca/page-NNN.txt` locators. Membership establishes no head/assistant, exact XC/TF/indoor scope or side; converter's Coach/Track is an umbrella, not source appointment evidence. |
| `tx_uil_chairs.py::{rows_of,main}`; `TX-uil-chairs` | UIL district-chair/H/U | Explicit AD/coach title; sport from XC/spring-meet page, blank side/email. Script `SPORT_OF_PAGE` and `out/extra/TX-uil-chairs.jsonl` locator; a district chair is not automatically the school's current program coach. No current coach collector. |
| `maxpreps_lane.py::{coach_from,main}`, `integrate_lanes.py::convert`; `maxpreps*` | MaxPreps/H/U | Embedded coach name/gender search; sport/HeadCoach inferred by crawl/converter context, not exact title/time. `notes/maxpreps-lane.md`, `notes/us-maxpreps.md`, located `out/extra/maxpreps*.jsonl`; NA sample remains unqualified. Not a current13 family or proof of either side/indoor completeness. |
| `platform_sweep.py::{try_platform,parse_coaches,main}`; `platform-{a,b,c}` | Public platform-probe obligations/H/U, not three provider families | Generic regex assumes Track/HeadCoach/Varsity with blank side; domain labels alone do not own a role. `notes/platform-sweep-{a,b,c}.md`, located `out/extra/platform-{a,b,c}.jsonl`; source-specific provenance/role/time qualification missing. |
| `school_sites.py::{analyse_tables,analyse,main}` → `school_sites_to_extra.py::main`; `<ST>-school-sites` | School/CMS contact obligations/H/U, provider-specific qualification required | Raw coach/AD signal contexts, guessed head/Track/mailbox joins and side from text; no accepted source appointment merely by emitting containers. Script contracts and located `out/extra/<ST>-school-sites.jsonl`; `BI-school-sites` anomalous label appears in historical coverage, not an added jurisdiction. No signal/staff payload opened. |
| `ok_directory.py::main`, `mo_schools.py`, `site_seeds.py`, `osm_sites.py`, `osm_sites_tiled.py`, `ccd_frame.py`, `pss_frame.py`; named address/frame extras | School/site/address discovery only; no coach family count | File/label locators from CP directory listing and `out/extra/{CCD-fill,PSS-fill}.jsonl` plus enrichment labels do not prove coaching acquisition. Public school-directory workbook schema in `ok_directory.py` is site discovery, **no workbook opened**. Private-school enrichment/PSS files and their data were excluded from evidence intake; not evidence of a fresh population. |

### Historical original50 / acquired8 comparison

**Original50:** static `CP/sources.py` inspection yields 48 distinct `custom:` registrations,
plus generic `html` =49 registered stage-one kinds; `CP/parsers/*.py` yields63 nonempty
implementation modules plus empty initializer =64 files. Relative to reported50, those units
differ by **−1 kinds, +13 implementation modules, +14 files**. Fifteen modules are outside the
stage-one registration, and auxiliary lanes add further code keys without supplying a dated
50-entry membership list. ADR-015's approximate “~40 extractor kinds” is not such a list.
Nothing here identifies the original50 revision or licenses reconstructing50 by dropping keys.

**Acquired8:** `CP/notes/verification.md` §3 “Stage-two rows trace to their own captures”
names exactly `CA-cif,CA-cif-xhr2,FL-schools,OH-schools,IA-bound,KY-arbiter,MT-arbiter,NH-arbiter`
and reports16,509 name-token-attested rows. These are **eight lanes → four families**
(**HC** three lanes, **OH** one, **BD** one, **AR** three), not eight providers or eligibility
proof. The dated 2026-09-29 DragonFly defect section separately names **eight then-completed
states** `NC,AL,WY,MD,DE,AR,NM,DC`, all **one DF family**. Its later `notes/dragonfly.md::Resulting
artifacts` table lists15 states/11,194 summaries/26,566 mapped rows; this is another historical
scope, not evidence of acquired8. DC row counts differ inside that report (118 in one table,
124 in another), and its before/after state counts differ from the preserved later summary;
retain the scopes, do not manufacture a common run snapshot.

**Searched locators:** CP root, complete `parsers/*.py`, `sources.py`, `run.py`, `extract.py`,
`deepen.py`, `coverage.py`, `dragonfly_probe.py`, `lane_check.py`, public auxiliary scripts;
`notes/*.md` keyword search for50 implementations/sources/parsers/families/adapters,
8/eight acquired families and `manifest`; named report schemas/counts
`out/{coverage.md,report.json,state-summary.json}`, `out/*{report,summary,probe}*`,
`out/extra/*` filenames; recursive CP `*manifest*`, `*inventory*`, `*registry*` filenames.
No owning50/eight-family manifest was found in that bounded search. Notes cite
`/tmp/probe_lane2_manifest.py` and `/tmp/probe_lane2_manifest.out`; `/tmp/probe_lane2_manifest*`
had no matching preserved file. Archive/worktree revisions outside CP were not searched.
The exact missing owner artifact is therefore the **revision-bound original50 key manifest**
and the **run/date-bound acquired8 family → source unit → successful capture/effect receipt
manifest**, not missing executable code or permission to rerun the legacy engine.

**Receipt limit:** `CP/polite.py::{_cache_url,_key,_response}` stores raw files by host and
SHA256(cache URL)[:24], including POST-body hash and optional salt. Metadata contains
`url,status,bytes` only, **no fetched_at, content hash, run/revision or effect ID**.
The retained NC directory and summary `.meta.json` files were read: URL/status200/bytes800064
and671271 match the named report locators, but metadata alone cannot date them. Knack app salt
and POST identities are mandatory to interpret provenance; URL-only aggregates cannot recover
them. Current fixture manifests/NC qualification carry their own explicit dates/digests; replay
does not advance freshness. `coverage.py::detail_counts/build` sums successful detail payload
rows and source yields, not unique qualified people or complete programs; `state-summary.json`
is a merged snapshot, not a family acquisition manifest. Historical code/tests/audit statements
are quoted as retained claims, not commands run in this recovery.

### Updated reported50 / acquired49 comparison

The user's newer **50 implemented / 49 acquired** observation is preserved separately from the
older acquired8 inquiry above. Its original implementation keys/revision and successful acquisition
keys/run have not been recovered; no unique missing source can be selected from the two totals.
The subsequent bounded recovery inspected the sibling worktrees, current and historical `var/`
roots and `/tmp/piaa-live`, not just the prototype registry.

The [keyed inventory evidence](../../var/source-keyed-inventory-sol-20261002.json) retains
the exact current 24 registry descriptors, 24 canonical provider CLI keys plus the
`wayzata_schedule` alias, 18 durable dispatch entries resolving to 17 distinct keys,
and 14 actual `TeamsSource` acquisition arms. It also records 270 dated prototype
source IDs and each historical report root separately. `var/run-2027-v2/out/report.json`
and the Midwest core report genuinely contain 50 grade-evidence namespace keys:
49 jurisdiction-qualified MileSplit labels plus `wiaa_results`. The Midwest
all-sources report contains 51 keys, adding `athleticlive_athletes`; the export-1
root has 47 and the Athletic.net pilot has one. These retained observation
namespaces are not a revision/run-bound implementation/acquisition manifest pair.
The uniquely missing member of the reported50/49 remains unknown.

`/home/lewis/src/ad-law-scrape/arh-coach-acquisition/target/capture-census.{md,json}` records an
offline inventory generated **2026-09-29T19:02:17Z**: 270 source rows, 264 available bodies and
successful parses, 49 registered kinds, 47 kinds with available bodies and 13 kinds yielding contact
rows (6,534 rows, including administrators). The script makes no network calls and records no
implementation revision. Its body selection permits metadata-free digest fallback and non-200
captures; body hashes in that inventory are truncated. These are not 49 fresh acquired coach
families. The exact missing in-scope source IDs are
`al_ahsaa,de_diaa,la_lhsaa_coaches,mo_mshsaa,va_vhsl`; out-of-scope `ak_milesplit` is the sixth missing
row. These differences belong to that inventory, not to the user's unidentified 50/49 manifests.

The prototype `out/coverage.md` has positive historical TF/XC yields for every lower48+DC
jurisdiction. Its `sources-ok` predicate means school/contact output, not transport success or
qualified provider completion, and the report has no generation/revision/run identity.
`var/midwest-census/out/seal.json` separately reports **50 jurisdiction buckets** on 2026-09-25.
Neither artifact establishes that the user's two counts use geography rather than source keys.

Real retained publisher response witnesses were located for all 13 currently inspected provider
families, including CHSAANow and native TSSAA despite their missing callers in the inspected
pre-integration snapshot. Examples are
`var/midwest-census/http/583e31ea5bce0e14a82f30ad4718b866` (CHSAANow, 2026-09-22T16:10:23Z),
`var/midwest-census/http/078e90211a0d5d3319742c16aa84949c` (TSSAA, 2026-09-22T16:10:49Z),
`var/census-service/http/4e0bfe2b0fced2b0df9ed5ef48434fd4` (MPA, 2026-09-24T22:35:18Z), and
`/tmp/piaa-live/http/bb69990e690fe8b47080864fc6366e52` (PIAA, 2026-10-01T05:35:21Z).
The MPA body differs from the fixture qualifier's body at the same URL. PIAA retains a public
school contact/administrator response, not a TF/XC appointment or a canonical export; its `out/`
directory is empty. Historical HTTP success does not establish current tenure, admission, fresh
acquisition, durable effect or national coverage.

The unresolved prerequisite is the **revision-bound 50-key implementation manifest** and the
**run/revision/date-bound 49-key successful acquisition manifest**, joined on stable source keys
with request identity, full body digest and qualified effect/role criteria. The five caller gaps
identified in the earlier snapshot were OHSAA, FusionPoint (MPA/RIIL), PIAA, CHSAANow and TSSAA.
Current `restate_services/teams_arms.rs`, `teams_arms/associations.rs` and jurisdiction dispatch
contain those durable `TeamsSource` acquisition arms. That implementation is not their native
qualification or a fresh national acquisition manifest; CIAC remains a separate CLI-only arm.

### Every scoped jurisdiction: applicability versus execution

**Reading the table:** the family/wiring column is the present provider-backed coach route, not
a statement that it ran. The CSV column explicitly accounts for all 22 applicability entries:
`I` means historical import applicability only, `—` means not declared there.
DF status is explicit for **all 49** selectors; `E` refers only to its historical 15-state report,
not the `VERIFIED` constant alone. Native alternatives retain separate E/F/U/? statuses.
Current qualifier `?` means no executed coach qualification is evidenced in this session;
`F-native` is the unrelated OH fault qualification, **not** an OHSAA coach acquisition failure.
Every `N` means never submitted to the absent current fresh49run.

| St / DragonFly selector | Present coach family / wiring | CSV | Historical DF status | Other historical coach evidence / exact locator | Current executed qualifier | Current fresh49run |
|---|---|---|---|---|---|---|
| AL / AHSAA | DF/D | — | E | AS/AL F: native-host then-policy refusal, CN/AL; DF probe repo `probe/AL/PROVENANCE.json` | ? | N |
| AR / ArkAA | DF/D | — | E | AS/AR U: shell, CN/AR `samples/dir/AR__dragonfly.html`; SC/AR | ? | N |
| AZ / AIA | none | I | U | AZ-AIA E: names/phone only, MT/AZ `samples/aia-school-detail-58.html`; old CN/AZ U shell | ? | N |
| CA / CIF | none | I | U | AS/CA F/U: old empty-202, CN/CA; WC/CA section widgets school-only, no coach roster | ? | N |
| CO / CHSAA | CO/D | I | U | CO E: `tests/fixtures/chsaa/PROVENANCE.json`; old CN/CO F login and MT/CO U tab shell are other surfaces | ? | N |
| CT / CIAC | FP/C | — | U | FP E: CN/CT `samples/dir/CT__directory.html`; F: CN 2026-09-24 adapter expired TLS; MA/CT | ? | N |
| DC / DCSAA | DF/D | I | E | AS/DC U: MA/DC `samples/dcsaasports-school-directory.html` has no contacts; old CN/DC host is not authoritative association identity | ? | N |
| DE / DIAA | DF/D | — | E | AS/DE F: challenge, CN/DE `samples/dir/DE__home.html`, MA/DE `samples/diaa-home.html` | ? | N |
| FL / FHSAA | none | I | U | HC E/U: SE/FL `samples/homecampus-school-details-2397.json`, AD-only named sample; five null-name coaches | ? | N |
| GA / GHSA | DF/D | I | E | GA-PDF E: SE/GA `samples/ghsa-directory-feed.pdf`; DF repo `probe/GA/PROVENANCE.json`; different row universes | ? | N |
| IA / IoHSAA | none | I | U | BD F/U: CN/IA `samples/validate/IA_Bound_adm.html`, retractions; PL/IA prior browser citations not fresh | ? | N |
| ID / IdHSAA | DF/D | — | E | ID-AD E: MT/ID `samples/idhsaa-directory.html`; CN/ID F Turnstile at older native-host capture | ? | N |
| IL / IHSA | IL/D | I | U | IL E: CN/IL staff2 + revealed-email captures, GL §2; no DF collector for IHSA | ? | N |
| IN / InHSAA | none | I | U | AS/IN U: CN/IN `samples/dir/IN__ihsaa-school-directory.html` handoff; DF repo `in_staff_summary_qwugx2.json` no staff | ? | N |
| KS / KSHSAA | KS/D | I | U | KS E: AD-only, CN/KS `samples/api/KS__directory_a.json`, PL/KS | ? | N |
| KY / KHSAA | AR/D | — | U | AR E(report)/U(raw mapping): four-record prototype citation; SC/KY `samples/ky-schools-arbiterlive.tsv` is IDs, not fetched coaches; CN/KY F old delegated-host refusal | ? | N |
| LA / LHSAA | none | — | U | LA-PDF E(bytes)/U(parse): SC/LA `samples/la-coaches-directory-2025-26.pdf`; CN/LA shell | ? | N |
| MA / MIAA | none | — | U | AR-other F/U: CN/MA delegated-host refusal; MA/MA `samples/miaa-school-list.pdf` contains no coaches | ? | N |
| MD / MPSSAA | DF/D | — | E | AS/MD F/U: MA/MD `samples/mpssaa-ad-directory.html` login; CN/MD search/school identity only | ? | N |
| ME / MPA | FP/D | — | U | FP E: CN/ME `samples/dir/ME__directory.html`; 2026-09-24 staged adapter rows, no email; MA/ME alternate school-list surface | Q offline capture-date replay only | N |
| MI / MichHSAA | none | I | U | MI-AD F: CN/MI `baseline_rows_refused`, `tools/robots/my.mhsaa.com.txt`; GL/MI earlier reported sample not fresh | ? | N |
| MN / MSHSL | MN/D | I | U | MN E: CN/MN AD-name school samples; PL/MN team-coach endpoint evidence; AD-email provenance discrepancy retained | ? | N |
| MO / MSHSAA | none | I | U | AS/MO F: CN/MO then-policy native-host refusal; PL/MO school/result universe is not coaching acquisition | ? | N |
| MS / MHSAA | DF/D | — | E | AS/MS F: CN/MS `samples/dir/MS__home.html` 403; SC/MS | ? | N |
| MT / MHSA | DF/D; AR/D | — | E | AR E(report)/U(raw mapping): four-record citation; MT/MT `samples/mhsa-cross-country.html` committee email is not school coach | ? | N |
| NC / NCHSAA | DF/D | — | E | DF E: repo NC directory/summary pair; SE/NC `samples/schools-nchsaa.html` conference admins are not school ADs | Q postal replay only | N |
| ND / NDHSAA | DF/D; ND/D | I | E | ND E: CN/ND `samples/validate/ND_Minot_North_1307.html`, PL/ND; names/Unknown sport-role, no email | ? | N |
| NE / NSAA | NE/D | I | U | NE E: CN/NE `samples/validate/NE_nsaa_Adams_Central.html`, wholesale ledger; PL/NE; no emails | ? | N |
| NH / NHIAA | AR/D | — | U | AR E: repo `tests/fixtures/arbiter/PROVENANCE.json` NH body captures; CN/NH and MA/NH native shells unqualified | ? | N |
| NJ / NJSIAA | none | I | U | NJ-AD E: MA/NJ `samples/njsiaa-member-info.html` first-page AD names; old CN/NJ shell verdict not overwritten | ? | N |
| NM / NMAA | DF/D | — | E | AS/NM U: MT/NM `samples/nmaa-for-coaches.html` resource/office contacts, CN/NM school-only list | ? | N |
| NV / NIAA | none | — | U | FP E/U: CN/NV `samples/dir/NV__directory.html` AD/admin only, no implemented NV arm; WC/NV F empty-202 coach landing | ? | N |
| NY / NYSPHSAA | none | — | U | AS/NY ?: MA/NY `samples/nysphsaa-schools-path.html` advertised ADS/Coaches but no roster; NY state-ED reader is address-only | ? | N |
| OH / OHSAA | OH/D | I | U | OH E: CN/OH `samples/validate/OH_Centerville_336_TF_coach.html`, GL/OH; native coach arm coded, execution unqualified | F-native; current CLI search transport failed, zero contact facts; offline 38 tests PASS | N |
| OK / OSSAA | none | — | U | AR-other U/F: candidate org106940 empty historically, not registered; CN/OK delegated-host refusal; SC/OK login/manuals | ? | N |
| OR / OSAA | none | I | U | OR-OSAA E: WC/OR `samples/osaa-school-347.html`; old CN/OR list-only evidence is another surface | ? | N |
| PA / PIAA | PA/D | — | U | PA E: repo `tests/fixtures/pa_piaa/PROVENANCE.json`; MA/PA later letter/details qualification, AD-only; old CN/PA shell retained | ? | N |
| RI / RIIL | FP/D | — | U | FP E: CN/RI `samples/dir/RI__directory.html`; historical 2026-09-24 rows are distinct from current capture | Fresh bounded directory: 55 schools/258 appointments, capture binding verified; no mailbox/current-tenure claim | N |
| SC / SCHSL | DF/D | — | E | AS/SC U: SE/SC `samples/directory-schsl.html` navigation, not contact roster; CN/SC shell | ? | N |
| SD / SDHSAA | none | — | U | BD F/U: CN/SD `samples/dir/SD__bound-sd-schools.html` 403/retractions; PL/SD prior browser evidence | ? | N |
| TN / TSSAA | DF/D; TN/D | I | E | TN E: repo `tests/fixtures/tssaa/SOURCE.md` detail captures; SC/TN old list-only capture differs | Fresh bounded Page High: 11 published appointments/mailboxes, explicit 2026–2027 tenure and three postal claims verified | N |
| TX / UIL | none | I | U | AS/TX U: CN/TX `samples/dir/TX__home.html` no coaching rows; NA MaxPreps staff capture unqualified for TF/XC | ? | N |
| UT / UHSAA | none | I | U | UT-UHSAA E: MT/UT `samples/uhsaa-school-alta.html`; old CN/UT shell is not current absence | ? | N |
| VA / VHSL | none | — | U | AS/VA F: CN/VA refusal; SE/VA cites MA `samples/robots-vhsl.txt`; no qualifying school coach evidence | ? | N |
| VT / VPA | none | — | U | AS/VT U: CN/VT `samples/dir/VT__membership.html` association-office mailto, MA/VT; not school-scoped coaches | ? | N |
| WA / WIAA | none | — | U | FF U: CN/WA `samples/dir/WA__state_schools.html` unlabeled mailbox; WC/WA no names; WI adapter must not be assigned to WA | ? | N |
| WI / WisIAA | WI/D | I | U | WI E: CN/WI `samples/validate/WI_Abbotsford_orgID_1_coach_AD.html`, GL §6 | ? | N |
| WV / WVSSAC | AR/D | — | U | AR E(report)/U(raw mapping): four-record citation; SE/WV `samples/school-directory-wvssac.html` confirms org4223 iframe, not coach payload | ? | N |
| WY / WHSAA | DF/D | — | E | DF E: repo `probe/WY/PROVENANCE.json`, summary SS28UB; MT/WY F native `/schools/`403; CN/WY unknown native roster | ? | N |

**Excluded historical selectors:** AK/ASAA and HI/HHSAA remain in
`survey.rs::ASSOCIATIONS`, but neither is in `REGISTERED` or the national denominator.
AK repo `coach_directories/probe/AK/PROVENANCE.json` and CN/AK
`samples/dir/AK__member-schools.html` preserve directory/no-staff evidence; WC/AK reports
school phone only. HI CN/HI `samples/dir/HI__schools.html` is an unqualified shell; WC/HI
`SOURCE_REPORT.md` describes no named school coach roster. Both are **excluded**, not two
additional fresh-run failures or completed jurisdictions.

### Current executed evidence and remaining obligations

- **NC postal qualifier:** `var/qualification-postal-sol-20261002-actual-02/qualification.json`
  records `status: passed`, `qualification_only: true`, `national_census_qualified: false`,
  `capture_origin: retained public fixtures, offline replay; not fresh network acquisition`,
  `public_athletes: 0`, `synthetic_athletes: 0`, `model_calls: 0`.
  Its capture array binds `/states/NCHSAA/directory/1` (800,064 bytes,
  SHA256 `4cca3a67f7eecd8e143ae02e18939f7f9aefb91d8d8106ccdada87b2e72cb814`) and
  `/schools/ZCUM49/summary` (671,271 bytes,
  SHA256 `ed24ab23e0a76bd8fb705bc7cd72eae771ba19bbfa4200dea498e299e8b43d2a`).
  Both `fetched_at` values are **2026-09-27**, not the qualifier's 2026-10-02 date.
  One school `sch_9baa494b8e4b67e0` / owner `ZCUM49` retains two source-owned postal claims.
  The raw coach array / retained `csv_cli.stdout` exposes 16 contexts; Unknown roles and
  repeated person/sport/side contexts prevent an eligible-contact count. The generation is
  `8824e277e39fd7f43751f278a3d3a0ff6f246df2ff5482560dea7af1e86a7150`, store identity
  `2d3ae586c50c168041d0d4f0ee7b2bbeada47a2dc9c65ab2e67de4968896ab61`.
  This appendix only reads the retained qualification JSON, not workbook contents or a live store.
- **OH native fault qualifier, not coach acquisition:** retained
  `var/qualification-native-teams-sol-20261002-actual-05/{qualification.json,qualification-oracles.json,qualification-error.json,production-request.json}`.
  Identity `jurisdiction:OH:2026-27:1`; native teams action executed three logical attempts
  and retained Failed; rosters persisted, meets did not persist, results not reached.
  Its historical plan explicitly refuses `ohsaa` as unwired for that qualification snapshot;
  current `TeamsSource` wiring is D. The native measurement
  therefore neither qualifies the OHSAA coach collector nor fails a fresh national contact census;
  `national_completeness: false` is retained. Physical admissions and native action attempts are
  separate counts. Main owns the transport/native proof and any subsequent qualification.
- **Recovered historical inventories, bounded missing manifests:** all63 prototype parser keys,
  the49 stage-one kinds/source-ID generators and identified public auxiliary lanes are mapped above.
  Reported50 remains unresolved against that snapshot (49 kinds/63 implementations/64 files);
  acquired8 remains an unknown eight-family receipt set, not the separately named eight lanes,
  eight DragonFly states or eight current durable callers. Exact searched/missing locators and
  date/schema limitations are recorded above. KY/MT/WV Arbiter lane files are located but their
  full per-org raw/effect manifests were not opened/verified; NH's report/fixture mapping does not
  certify them. Do not rerun historical acquisition or ingest old population to close these gaps.
- **Owned integration versus qualification:** current jurisdiction dispatch and `TeamsSource`
  code call the OHSAA, MPA, RIIL, PIAA, CHSAANow and TSSAA collectors. The earlier missing-caller
  snapshot above is historical, not current implementation. CIAC remains CLI-only and CSV import
  is not fresh-source acquisition. Qualify actual native execution and every provider-owned
  capture/effect before treating these coded arms as coverage. Historical-only alternatives in
  the preceding table still require a qualified collector/caller or explicit retained gaps.
- **Unexecuted scope:** all **49 N rows** still owe the fresh run's provider-specific submitted,
  attempted, captured, parsed, rejected, failed, completed and unknown outcomes with exact unit,
  capture/effect/receipt identities. Availability is not school/program coverage. Sample-empty,
  source refusal and no submission must never be folded into “no coaches.” Contact tenure,
  role/mailbox eligibility, canonical program joins and side/indoor distinctions remain the
  `athletic-rust-pipeline-rh3` / school-identity obligations, not claims from this inventory.

