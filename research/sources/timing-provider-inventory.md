# Timing Provider Inventory — Midwest Class-of-2027 Census

**Date:** 2026-09-22
**Scope:** 49 states (all except AK via direct research; HI via national aggregator data)
**Purpose:** Enumerate every timing result source relevant to collecting Class-of-2027 (Grade 11 in 2025–26 or 2026–27) cross-country / track & field data. Providers ranked by states served.

---

## Ranking Table

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

---

## Provider Details

### 1. MileSplit (FloSports) — PRIMARY

| Field | Detail |
|-------|--------|
| **Hosts** | `oh.milesplit.com`, `tx.milesplit.com`, `wi.milesplit.com`, `www.milesplit.com` … (one per state) |
| **States served** | 49 (every state with a HS athletics presence) |
| **Enumeration method** | `GET <st>.milesplit.com/teams?type=1` → 977 HS team IDs (OH) / 26,265 total HS team records; `GET <st>.milesplit.com/calendar?season=<cc|indoor|outdoor>&year=<YYYY>` → meet rows with `data-meet-id`; `GET <st>.milesplit.com/api/v1/rosters/teams/<TeamID>/ranked?season=<cc|outdoor>&year=<YYYY>&grade=2027` → athlete rows with `gradYear: 2027` |
| **Result format** | JSON API (`/api/v1/meets/<MeetID>/performances`) returns full meet with `mark`, `units` (ms for running, mm for field), `place`, `eventType` (R/F/T), `eventCode` |
| **Stable IDs** | `athleteId` (numeric), `teamId` (numeric), `meetId` (numeric), `performanceId` (numeric per result), `gradYear` (numeric graduation year in roster API); separate MeetID space from AthleticLIVE/AN |
| **Grade/class-year** | **Yes** — `gradYear` field in roster API (e.g. `"gradYear": 2027`); `Class of 2027` in athlete profile HTML; `grade=2027` filters to Class of 2027 |
| **Historical depth** | 2016–2027 (11 school years) selectable per team; sampled athlete carried 10 season blocks (2026→2022) |
| **Incremental discovery** | Calendar monthly diffs; team roster API re-poll; server freshness stamp `{"created": <epoch>, "cache": {"fresh": true}}` |
| **Access characteristics** | Team index / calendar / team page / athlete profile: normal HTML, no auth. Roster API: public structured JSON, no key required. Leaderboard (`/rankings/events/…`): **subscription-restricted** — 1 free row (rank 1) + 49 masked rows per page. Results on athlete profile: masked (536 `<span class="mask" aria-label="Locked">` on sampled profile). **Robots disallowed** on `/api/` and `/rankings` paths. Rate limits: none observed; ~50 requests to OH host safe. |
| **Cost** | Free for roster API, team index, calendar, athlete profile metadata |
| **Key URL** | `https://oh.milesplit.com/teams`, `https://oh.milesplit.com/api/v1/rosters/teams/<id>/ranked`, `https://oh.milesplit.com/api/v1/meets/<id>/performances` |
| **Recommendation** | **PRIMARY** — the only free source carrying explicit Class-of-2027 grade across all 49 states with a team index |

---

### 2. AthleticLIVE Tenant Network — PRIMARY — Result Source

| Field | Detail |
|-------|--------|
| **Hosts** | `search.athletic.live` (Elasticsearch endpoint), `athleticlive.blob.core.windows.net` (blob storage), `*.athletic.live` (tenant sites) |
| **States served** | 49 (tenant ecosystem observed in all 49 states) |
| **Enumeration method** | `POST https://search.athletic.live/_search` (anonymous, no key) — returns meet docs with `ani` (Athletic.net MeetID) on 83.3% of 75,585 meet documents |
| **Result format** | Elasticsearch documents: `ani` (AN MeetID), `EventID` (numeric), blob path `ind_res_list/_doc/<EventID>`; blob rows carry `a.y` (year/grade) in some states |
| **Stable IDs** | `ani` = Athletic.net MeetID (516 docs carry `ani = -1` — sentinel, filter `ani > 0`); `EventID` (numeric, 6 digits) |
| **Grade/class-year** | **Rare** — `a.y` column in some states' blob rows; not consistent |
| **Historical depth** | ~75,585 meet documents in index; historical coverage varies by state |
| **Incremental discovery** | Elasticsearch search with time range filters; blob `ind_res_list` directory listing |
| **Access characteristics** | Anonymous POST to Elasticsearch endpoint, no auth key required. Returns JSON. Blob URLs are public. No rate limits observed. |
| **Cost** | Free |
| **Key URL** | `POST https://search.athletic.live/_search`, `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<EventID>` |
| **Recommendation** | **PRIMARY — Result Source** — the single highest-value meet result source across all 49 states; carries AN MeetID on 83.3% of meet docs |

---

### 3. Athletic.net — ATHLETIC.NET-SEED

| Field | Detail |
|-------|--------|
| **Hosts** | `www.athletic.net` (SPA), `live.athletic.net` (live results), `athletic.net/result/<code>` (result URLs) |
| **States served** | 49 (state division pages exist for every state) |
| **Enumeration method** | `GET /cross-country/usa/high-school/<state>` or `/track-and-field-outdoor/usa/high-school/<state>` — returns 200 with ~7.4 KB Angular shell; division API `GET /api/v1/DivisionHome/GetTree?sport=xc&divisionId=<id>&depth=3&includeTeams=true` returns `alignedTeams[]` with `SchoolID`; calendar `GET /api/v1/TeamHomeCal/GetCalendar?seasonId=<year>` returns meet list with `MeetID` |
| **Result format** | SPA-rendered pages (HTML not parseable by curl — 403 to default UA); JSON via browser XHR (`/api/v1/…`) |
| **Stable IDs** | `AthleteID`, `TeamID`, `MeetID`, `SchoolID` (== team id in `/team/<id>/…`), `ResultCode` (production contract: `/result/<code>`) |
| **Grade/class-year** | **Yes** — on profile pages and ranking pages; grad-year metadata in ranking responses |
| **Historical depth** | Varies by state: SD TF 2010–2026, SD XC 2009–2026 |
| **Incremental discovery** | Division tree API for school list; calendar API for meet list |
| **Access characteristics** | **Browser-gated** — returns 403 to `curl` default UA and `fetch()` from page; only SPA's own XHRs return 200. Requires a real browser session. Cloudflare challenge present. |
| **Cost** | Free (browser required) |
| **Key URL** | `https://www.athletic.net/cross-country/usa/high-school/<state>`, `POST https://search.athletic.live/_search` (AN MeetID) |
| **Recommendation** | **ATHLETIC.NET-SEED** — essential for athlete profiles, result codes, and state-meet results; browser-gated but provides the canonical result ID format (`/result/<code>`) |

---

### 4. DirectAthletics + TFRRS — RESULT-SOURCE (FL/IN/NH only)

| Field | Detail |
|-------|--------|
| **Hosts** | `directathletics.com` (registry), `tfrrs.org` (results), `www.gobound.com` (Bound platform), `tfmeetpro.com` (DirectAthletics meet software) |
| **States served** | 49 states in registry (Bound team list), but HS results only in **FL, IN, NH** |
| **Enumeration method** | Bound: `GET /sd/sdhsaa/<sport>/<season>/teams` → server-rendered team cards with DAT team ids; TFRRS: `GET https://lists.track/<l>_<l>.html?year=JR&limit=1000` (Indiana only for HS) |
| **Result format** | Bound: server-rendered HTML, scores by date, empty rosters/stats; TFRRS: HTML result tables (college-weighted); meet software on `tfmeetpro.com` |
| **Stable IDs** | Bound/DAT: `teamId` (hex string, e.g. `h20223291936291a49fb9d527b45679c`), AN MeetID embedded in meet rows; TFRRS: `team-id` (numeric) |
| **Grade/class-year** | **No** — college-weighted; HS results in FL/IN/NH carry no grade |
| **Historical depth** | Bound: 2021–2027 (7 seasons); TFRRS: extensive college history; HS limited |
| **Incremental discovery** | Bound scores by date; DAT team alignment pages |
| **Access characteristics** | Bound: normal HTML, no auth. Roster/stats/leaderboard endpoints: empty or HTTP 202 (app-only). TFRRS: normal HTML but college-focused. |
| **Cost** | Free |
| **Key URL** | `https://www.gobound.com/sd/sdhsaa/boyscrosscountry/2026-27/teams`, `https://lists.track/ind_ind.html` |
| **Recommendation** | **RESULT-SOURCE** for FL/IN/NH HS results; DISCOVERY-ONLY elsewhere (college-weighted, refuted in MT/WY/ID, AL/MS/TN/KY, CO, TX/OK/AR/LA as a HS source) |

---

### 5. PrimeTime Timing — REJECT

| Field | Detail |
|-------|--------|
| **Hosts** | `www.pttiming.com`, `live.pttiming.com`, Firebase RTDB `prime-time-results-<state>.firebaseio.com` |
| **States served** | ~10: WI (primary), MO (24 events in 2026), IL, FL, IA, IN, NE, KS, TX, PA |
| **Enumeration method** | `GET https://www.pttiming.com/api/results/current?year=YYYY&page=N` → current results; live results via Firebase Realtime DB |
| **Result format** | Live: Firebase RTDB JSON; archive: HTML result files |
| **Stable IDs** | `ptmeetid` (numeric), Firebase `EventID` |
| **Grade/class-year** | **No** on PrimeTime-owned surface; some WI Hy-Tek PDFs and IHSA API carry grade, not PrimeTime itself |
| **Historical depth** | Current season results visible |
| **Incremental discovery** | `/api/results/current` pagination |
| **Access characteristics** | **ToU explicitly prohibits automated access and recruiting database building** on `live.pttiming.com`. Firebase RTDB is private (no anonymous access). PrimeTime-owned surface (`www.pttiming.com`) is same operator (Karmarush LLC) — same ToU applies. |
| **Cost** | Free (but ToU blocks automation) |
| **Key URL** | `https://www.pttiming.com/api/results/current?year=2026` |
| **Recommendation** | **REJECT** — ToU prohibitions on automated access and recruiting database building; Firebase data is private |

---

### 6. Finish Timing — RESULT-SOURCE (OH/KY/IN)

| Field | Detail |
|-------|--------|
| **Hosts** | `www.finishtiming.com` (SvelteKit app), `finishtimingresults.com` (static archive), `finishtiming.trackscoreboard.com` (live) |
| **States served** | 3 (OH primary, KY, IN), plus college/USATF |
| **Enumeration method** | Homepage rendered SPA lists upcoming/recent meets; static archive at `GET https://finishtimingresults.com/2026/` (Apache autoindex); XC at `/2026/CC/<MM-DD-CODE>/` |
| **Result format** | Track: Hy-Tek plain text in `<pre>` tags; XC: PDF files |
| **Stable IDs** | **`FT id = "20" + AN MeetID`** — verified on 28/28 pairs; also native IDs (`649029`, `1000739707`, `2000631249`) |
| **Grade/class-year** | **Yes** — numeric grade column (9–12) in result rows for HS meets |
| **Historical depth** | 2025–2026 on current host layout; older seasons under date-coded dirs |
| **Incremental discovery** | Homepage fetch → new meets with AN MeetIDs; archive autoindex → `Last modified` timestamps per directory |
| **Access characteristics** | Homepage: browser SPA (SvelteKit, 2,437-byte shell); static archive: Apache autoindex, normal HTML/PDF. No login. |
| **Cost** | Free |
| **Key URL** | `https://finishtimingresults.com/2026/`, `https://finishtimingresults.com/2026/CC/09-05-ASH/` |
| **Important caveat** | Homepage "Final Results" links point at `/2026/20<ANID>.html` but XC files live under `/2026/CC/<MM-DD-CODE>/` — treat `20<ANID>` as the **join key**, not as a guaranteed fetchable path |
| **Recommendation** | **RESULT-SOURCE** — Ohio timer with free, grade-labeled results and deterministic AN MeetID mapping |

---

### 7. Lancer Timing — RESULT-SOURCE (MA/NH/NE/CNESSPA)

| Field | Detail |
|-------|--------|
| **Hosts** | `lancertiming.com`, `lancer-results.com` |
| **States served** | 4 (MA primary, NH, NE/CNESSPA) |
| **Enumeration method** | Lancer Timing results on Athletic.net; AN MeetID links from association pages |
| **Result format** | Hy-Tek PDFs / HTML results on Athletic.net |
| **Stable IDs** | AN MeetID (linked from MA/NE association pages) |
| **Grade/class-year** | **No** on Lancer surface |
| **Historical depth** | Multi-year results on AN state pages |
| **Incremental discovery** | AN state division pages for MA, NH, NE |
| **Access characteristics** | Results on Athletic.net (browser-gated); direct Lancer surface not independently fetchable |
| **Cost** | Free (results on AN) |
| **Recommendation** | **RESULT-SOURCE** — major New England timer, results accessible via AN |

---

### 8. 802 Timing — RESULT-SOURCE (VT)

| Field | Detail |
|-------|--------|
| **Hosts** | `802timing.com` |
| **States served** | 1 (VT) |
| **Enumeration method** | Only timer listed on MileSplit VT |
| **Result format** | AN MeetID results |
| **Stable IDs** | AN MeetID |
| **Grade/class-year** | **No** |
| **Incremental discovery** | MileSplit VT → AN MeetID |
| **Recommendation** | **RESULT-SOURCE** — VT's sole timer, accessible via AN |

---

### 9. Sub5 — RESULT-SOURCE (ME)

| Field | Detail |
|-------|--------|
| **Hosts** | `sub5timing.com` |
| **States served** | 1 (ME) |
| **Enumeration method** | MPA state meets + NE championship via Sub5 |
| **Result format** | AN MeetID results |
| **Stable IDs** | AN MeetID |
| **Grade/class-year** | **No** |
| **Recommendation** | **RESULT-SOURCE** — ME state meets timer |

---

### 10. Brewer Timing — RESULT-SOURCE (ME/MA)

| Field | Detail |
|-------|--------|
| **Hosts** | `brewertiming.com` |
| **States served** | 2 (ME, MA) |
| **Enumeration method** | State meets, league meets |
| **Result format** | AN MeetID results |
| **Stable IDs** | AN MeetID |
| **Grade/class-year** | **No** |
| **Recommendation** | **RESULT-SOURCE** — ME/MA state and league meets |

---

### 11. Hy-Tek Meet Manager — DISCOVERY-ONLY

| Field | Detail |
|-------|--------|
| **Hosts** | N/A (software installed on local machines at meets) |
| **States served** | N/A — per-meet tool used by timers nationwide |
| **Enumeration method** | Result files (`.htm`, `.html`, `.pdf`) carried in Hy-Tek event numbers |
| **Result format** | Hy-Tek event numbers + event names; Grade column in some WI PDFs |
| **Stable IDs** | Hy-Tek event numbers (numeric) |
| **Grade/class-year** | **Rare** — present in some WI Hy-Tek PDFs and PrimeTime/WI results, not in most |
| **Incremental discovery** | Not independently discoverable — found via result files in result archives |
| **Recommendation** | **DISCOVERY-ONLY** — not a data host, but the result format carried by many providers |

---

### 12. TrackScoreboard / FinishedResults — RESULT-SOURCE (MA/OH)

| Field | Detail |
|-------|--------|
| **Hosts** | `finishtiming.trackscoreboard.com`, `api.trackscoreboard.com`, Firebase Realtime DB `track-scoreboard-default-rtdb.firebaseio.com` |
| **States served** | 2 (MA indoor, OH live results) |
| **Enumeration method** | Live results via Angular SPA over Firebase; archived on `finishedresults.com` |
| **Result format** | Live: Firebase RTDB JSON; archive: HTML results |
| **Stable IDs** | Firebase MeetID (same as FT id = `20 + ANID` in OH) |
| **Grade/class-year** | **No** |
| **Recommendation** | **RESULT-SOURCE** — MA indoor meets, OH live results via Finish Timing partnership |

---

### 13. MeetPro (DirectAthletics) — DISCOVERY-ONLY

| Field | Detail |
|-------|--------|
| **Hosts** | `tfmeetpro.com` |
| **States served** | National (via DirectAthletics network) |
| **Enumeration method** | Meet management software by DirectAthletics; linked from MileSplit meet pages |
| **Result format** | Results uploaded by meet host/timer, not browsable as an index |
| **Stable IDs** | AN MeetID |
| **Recommendation** | **DISCOVERY-ONLY** — software platform, not a data source |

---

### 14. RaceTec — RESULT-SOURCE (WI)

| Field | Detail |
|-------|--------|
| **Hosts** | `racetec.com` |
| **States served** | 1 (WI) |
| **Enumeration method** | WI timer, behind Performance Timing in usage |
| **Result format** | AN MeetID |
| **Stable IDs** | AN MeetID |
| **Recommendation** | **RESULT-SOURCE** — WI timer |

---

### 15. AccuRace Timing — RESULT-SOURCE (WI)

| Field | Detail |
|-------|--------|
| **Hosts** | `accu race.com` |
| **States served** | 1 (WI) |
| **Enumeration method** | WI timer; AthleticLIVE tenant |
| **Result format** | AN MeetID via AthleticLIVE |
| **Stable IDs** | AN MeetID |
| **Recommendation** | **RESULT-SOURCE** — WI timer |

---

### 16. TrackSide Timing — RESULT-SOURCE (WI)

| Field | Detail |
|-------|--------|
| **Hosts** | `tracksidetiming.com` |
| **States served** | 1 (WI) |
| **Enumeration method** | AthleticLIVE tenant in WI |
| **Result format** | AN MeetID via AthleticLIVE |
| **Stable IDs** | AN MeetID |
| **Recommendation** | **RESULT-SOURCE** — WI timer, part of AthleticLIVE tenant ecosystem |

---

### 17. K2 Timing — RESULT-SOURCE (WI)

| Field | Detail |
|-------|--------|
| **Hosts** | `k2timing.com` |
| **States served** | 1 (WI) |
| **Enumeration method** | WI timer; AthleticLIVE tenant |
| **Result format** | AN MeetID via AthleticLIVE |
| **Stable IDs** | AN MeetID |
| **Recommendation** | **RESULT-SOURCE** — WI timer |

---

### 18. Performance Timing — RESULT-SOURCE (WI)

| Field | Detail |
|-------|--------|
| **Hosts** | `performancetiming.com` |
| **States served** | 1 (WI) |
| **Enumeration method** | WI timer; AthleticLIVE tenant |
| **Result format** | AN MeetID via AthleticLIVE |
| **Stable IDs** | AN MeetID |
| **Recommendation** | **RESULT-SOURCE** — WI timer |

---

### 19. TRXC Timing — RESULT-SOURCE (MO)

| Field | Detail |
|-------|--------|
| **Hosts** | `trxctiming.com` |
| **States served** | 1 (MO, St. Louis metro) |
| **Enumeration method** | MSHSAA district/sectional meets |
| **Result format** | AN MeetID |
| **Stable IDs** | AN MeetID |
| **Recommendation** | **RESULT-SOURCE** — St. Louis metro MO timer |

---

### 20. State Running Network — DISCOVERY-ONLY (NH/MA/CT/RI)

| Field | Detail |
|-------|--------|
| **Hosts** | Various state-running sites |
| **States served** | 4 (NH primary, MA, CT, RI) |
| **Enumeration method** | Aggregator for NH; links to AN meet pages |
| **Result format** | AN MeetID links |
| **Recommendation** | **DISCOVERY-ONLY** — aggregator, not a primary data host |

---

### 21. Baum's Page — RESULT-SOURCE (OH)

| Field | Detail |
|-------|--------|
| **Hosts** | `www.baumspage.com` |
| **States served** | 1 (OH) |
| **Enumeration method** | Meet-management/entry host + result-file host (NW/central/east OH small-school meets); OHSAA district/regional archives 2003–2021 |
| **Result format** | Host-uploaded Hy-Tek `.htm`, PDF result files |
| **Stable IDs** | `peventid` (small int, scoped per sport and per `table=A|C`), Hy-Tek event numbers |
| **Grade/class-year** | **No** — Year column exists in result files but empty in every file sampled |
| **Historical depth** | 2003–2026 |
| **Incremental discovery** | `GET /cc/index.php` and `/track/index.php` → full 2026 event list with dates; new `peventid`s are delta |
| **Access characteristics** | Normal HTML + static files; host-uploaded files can be non-results (e.g., IRS W-9s masquerading as meet files) |
| **Recommendation** | **RESULT-SOURCE** — OH small-school meets with deep historical archive (20+ years) |

---

### 22. MITS (Michigan Indoor Track Series) — RESULT-SOURCE (MI)

| Field | Detail |
|-------|--------|
| **Hosts** | AthleticLIVE tenant in MI |
| **States served** | 1 (MI) |
| **Enumeration method** | Michigan Indoor Track Series; AthleticLIVE tenant |
| **Result format** | AN MeetID via AthleticLIVE |
| **Stable IDs** | AN MeetID |
| **Recommendation** | **RESULT-SOURCE** — MI indoor track |

---

### 23. WIAA (Wisconsin) — DISCOVERY-ONLY

| Field | Detail |
|-------|--------|
| **Hosts** | `wiaa.com` |
| **States served** | 1 (WI) |
| **Enumeration method** | State archives, 130 files in 2025 |
| **Result format** | AN MeetID |
| **Recommendation** | **DISCOVERY-ONLY** — state association, links to results |

---

### 24. IHSA (Illinois) — VALIDATION

| Field | Detail |
|-------|--------|
| **Hosts** | `ihsatf.org` (AthleticLIVE + AN links) |
| **States served** | 1 (IL) |
| **Enumeration method** | AthleticLIVE tenant + AN links; **API with grade + AN athlete id** |
| **Stable IDs** | AN MeetID, athlete IDs, grades |
| **Grade/class-year** | **Yes** — IHSA API returns grade and AN athlete id |
| **Recommendation** | **VALIDATION** — provides grade and athlete ID for IL; validates AN data |

---

### 25. SDHSAA / Bound (South Dakota) — VALIDATION

| Field | Detail |
|-------|--------|
| **Hosts** | `sdhsaa.com`, `www.gobound.com` |
| **States served** | 1 (SD) |
| **Enumeration method** | SDHSAA activity pages + Bound platform; `GetTree` API returns 207 SchoolIDs; `GetCalendar` returns 11 XC meets + 3 TF state meets |
| **Stable IDs** | Bound school id (hex), AN MeetID (XC regions `278813…40`, state XC `278807`, state TF `646856/659/61`) |
| **Grade/class-year** | **Yes** — yearbook PDFs (`/Yearbook/{B,G}-{CrossCountry,Track}.pdf`) carry grade 11 column |
| **Historical depth** | Bound: 2021–2027; AN: 2009–2026; yearbook PDFs: per-season |
| **Recommendation** | **VALIDATION** — grade oracle for state-meet participants; removes discovery cost for SD spine |

---

## Providers That Could Not Be Verified

These providers were observed as link targets or named in research but **could not be independently verified** due to access restrictions:

| Provider | Reason |
|----------|--------|
| **Finish Timing / TrackScoreboard live SPA** | Requires JS rendering (SvelteKit / Angular SPA) |
| **PrimeTime live results (Firebase RTDB)** | ToU prohibits automation; Firebase private |
| **PrimeTime-owned surface (`www.pttiming.com`)** | Same ToU as live results (Karmarush LLC operator) |
| **Athletic.net SPA pages** | Cloudflare-403 from research machine; requires browser |
| **Speed Sport** (NE timer) | Domain observed only as link target, never fetched |
| **Northstar Timing** (NE) | Domain observed only as link target |
| **MSTCA Live Results** (NE) | Domain observed only as link target |
| **iResultsLive** | Domain observed only as link target |
| **Millennium Timing** | Domain observed only as link target |
| **Marathon Sports** | Domain observed only as link target |
| **MTS (Michigan)** | No independent site found; lives on AthleticLIVE stack only |
| **MaxPreps** | Score partner, not a result host; widget football-configured |
| **GoFan** | Browser-only, no verified IDs |
| **Arbiter** | Browser-only, no verified IDs |
| **niaa.com/NV** | CAPTCHA-gated (Nevada) |
| **Various guessed association domains** | DNS-failed on guessed hosts |

---

## Critical Refutations

### Assumption disproved by research

1. **Athletic.net state URL (`athletic.net/track-and-field-outdoor/usa/high-school/<state>/<assoc>`) is NOT evidence of partnership.** Returns 200 for any slug (~7.4 KB Angular shell). Never use as seed.

2. **`ani = -1` is a sentinel, not a MeetID.** 516 docs carry it. Must filter `ani > 0`.

3. **MileSplit MeetID space is separate from AthleticLIVE and AN.** Same MeetID 716278 appears in TX, OK, and LA indices. Never join on bare integer meet ID.

4. **MileSplit's team index `tx.milesplit.com` does not bound state.** A Georgia meet (741372) appears in TX index.

5. **AthleteID is 1:N for person-to-record mapping.** One athlete returned two IDs for same name+school+class. Join on `(AthleteID, school, grad-year)`.

6. **Every result file carries name/school/mark/place but NEVER a grade** (except IHSA API, some WI Hy-Tek PDFs, and SD yearbook PDFs). Co2027 must come from MileSplit rosters or AN, not result files.

7. **DirectAthletics is DISCOVERY-ONLY outside Indiana.** College-weighted, refuted in MT/WY/ID, AL/MS/TN/KY, CO, TX/OK/AR/LA.

8. **MileSplit `/api/` and `/rankings` are robots-disallowed.** Rate carefully.

---

## Provider-to-State Coverage Matrix

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

## Recommended Pipeline Architecture

Based on this inventory, the optimal data pipeline for Class-of-2027 census is:

1. **MileSplit PRIMARY** — team index + roster API with `gradYear` for all 49 states. This is the **only** free source carrying explicit Class-of-2027 grade nationwide.

2. **AthleticLIVE as Result Source** — `POST https://search.athletic.live/_search` for all 49 states, joining on `ani` (AN MeetID) to Athletic.net. 83.3% of 75,585 meet docs carry `ani > 0`.

3. **Athletic.net for athlete profiles and result codes** — browser-gated but canonical source for `/result/<code>` URLs and athlete profile metadata.

4. **Finish Timing for Ohio** — grade-labeled results, deterministic `20 + ANID` join key.

5. **State-specific timers (Lancer, 802, Sub5, Brewer, etc.)** — fill gaps not covered by MileSplit/AN, accessible via AN MeetID links.

6. **Validation sources (IHSA, SDHSAA/Bound)** — grade oracle and state-meet qualification lists.

7. **Reject** PrimeTime (ToU prohibitions), DirectAthletics outside FL/IN/NH (college-weighted), and all browser-only providers that cannot be automated without a real browser session.
