# 30. Cross-source coverage / Pareto consolidation (12-state Midwest)

Status: complete — all 29 peer reports read as written (25, 26 and 29 landed mid-consolidation and are
integrated), `data/*.csv` read, `synthesis/atn-endpoint-groundtruth.md` used as the ATN baseline. **Zero new
external fetches were made** (the whole report is a re-derivation over local artifacts — see Evidence appendix),
so the ≤25-fetch budget for this assignment is untouched.

Observed on: 2026-09-19. Scope: WI MN IA IL MI IN OH MO KS NE ND SD + the five national Athletic.net reports.

Method note: every number below is either (a) quoted from a report with its `[NN]` reference, (b) read from a
`data/*.csv`, or (c) arithmetic performed *on* those numbers, in which case it is tagged `[INFERENCE]`/`[CALC]`
and the arithmetic is shown. Where two reports disagree, both values are stated and one is selected with a reason
(§ Contradictions).

---

### Source

This report is the consolidation layer: it does not re-collect. Its inputs are the 29 peer reports
(`04`–`29` state/source reports plus `01`–`05` Athletic.net endpoint reports), the five CSVs in `data/`, and
Main's HAR-derived `synthesis/atn-endpoint-groundtruth.md`. Its job is to answer one question: **which external
sources change what the production pipeline can discover, verify and contact, and at what request cost**, so the
pipeline owner can choose the next adapters.

### Coverage

**Headline Pareto.** Of the 29 sources examined, **four families carry almost all of the value and all of them
are free (no auth, no browser)**:

| Family | Reach | What it uniquely supplies | Cost |
|---|---|---|---|
| **MileSplit team rosters** `[27]` | 12/12 states, 6,633 HS teams | Class-of-2027 grad year per athlete, enrolment-independent of ATN | 1 request/school (`/teams/<id>/roster`), robots-allowed |
| **AthleticLIVE blobs/RTDB** `[10][12][25]` | MN, IA, ND + 258 tenant origins | Athlete-level **Athletic.net IDs + grade + marks** in one document | 1 request/event doc, no Cloudflare, no browser |
| **Association APIs** `[13][09][23][24][06][19][29]` | IL, MN, KS, NE, WI, OH (+MI, ND names) | School universe, grade oracles, and **public professional coach/AD contacts** | 1–3 requests/school or 1 request/state |
| **TFRRS Indiana** `[18][28]` | IN | 1,861 Co2027 athletes with grade for 2 requests (`&year=JR`; 6,533 unfiltered); ≈12,300 Co2027 from ≈901 roster requests (≈0.07 req/athlete) `[INFERENCE]` | 2 requests + 1/team |

Everything else is either a narrow seed (MHSAA's 89 postseason MeetIDs `[15]`), a validation-only oracle
(MSHSAA state PDFs `[21]`), a dead end (DAT outside Indiana `[28]`), or explicitly off-limits
(`live.pttiming.com` ToU `[22]`, MileSplit `/api/`+`/rankings` robots `[27]`).

**Measured scale of the target.** The retained corpus is 142,705 US boys grade-11 athletes; the 12-state subset
is **40,695** by direct enumeration `[04]`; `[27]`'s per-state corpus attribution values sum to **41,145** while
that report prints a total of 46,145 (see Contradictions). That is 28.5 % of the national corpus in 12 of 50 states.

**Coverage by source family (12-state roll-up).**

| Family | States with measured coverage | Enumerate | Grade | Results | Coaches | Access |
|---|---:|---|---|---|---|---|
| Athletic.net (baseline) `[01]–[05]` | 12 | ✅ divListId/state, 40,695 G11 | ✅ GradeID 11 (server filter) | ✅ whole meet = 3 XHR | ❌ none | browser-session API (CF-403 to curl) |
| State associations `[06][09][11][13][15][17][19][21][23][24][25][26]` | 12 | ✅ all (169–828 schools) | ⚠️ state-meet cohorts only | ⚠️ postseason-heavy | ⚠️ 8 states have names, 6 have emails | mixed: open HTML/JSON, S3, Google Docs |
| MileSplit `[07][16][18][20][27]` | 12 | ✅ 6,633 teams + rosters | ✅ grad-year column | ⚠️ `/api/` robots-disallowed | ❌ | robots-allowed rosters/meets only |
| DirectAthletics/TFRRS `[14][18][23][28]` | 12 registry, 1 live | ✅ 22,527 team records / 5,933 schools | ⚠️ FR/SO/JR/SR sheets; IN live | ❌ 17 HS-marked idx rows of ≥921 | ❌ | open HTML, no CF |
| AthleticLIVE + timers `[08][10][12][16][17][20][22][23][24][25]` | 7 with evidence | ⚠️ via roster/team docs | ✅ per-row `y`/`Yr` | ✅ per-event docs | ❌ | public blobs, ES, RTDB; some ToU-restricted |
| Coach graph `[29]` | 10 (MO, IN none) | ✅ 874 schools / 2,298 rows | n/a | n/a | ✅ 951 AD + 1,347 coach rows | curl-friendly APIs + school HTML |

### Enumeration

**Comparative cost of enumerating one Class-of-2027 athlete (12-state scope).**

| Source | Requests | Yield | req/athlete | Method note |
|---|---:|---:|---:|---|
| TFRRS Indiana HSR lists `[18][28]` | 2 | 1,861 Co2027 (`&year=JR`; 6,533 unfiltered) | **0.0011** | measured, both lists, disjoint; **scope caveats**: HSR indoor qualifiers only (not a season), and 429 of 1,176 default-limit rows are relay-team rows with no `AthleteID`, so relay-only athletes are invisible on this surface `[18]` |
| ATN rankings sweep `[04]` | 2,728 | 40,695 | **0.067** | measured, 462 terminating pages + 24 overhead |
| AthleticLIVE state XC `[10]` | 6 | 242 Co2027 (959 rows) | 0.025 | measured, one meet, all events |
| MileSplit rosters `[27]` | 6,645 (`[CALC]`: 6,633 rosters + 12 team-list pages) | 41,145–62,400 `[INFERENCE]` | 0.106–0.162 | 716 Co2027 from 30 schools (36 roster requests, 6 empty); per-team median 18 |
| Association state-meet PDFs `[13][19][21][23][26]` | 1–20 | 89–1,214 grade-verified | 0.01–0.06 | cohort only, not enumeration |
| Coach directories `[29]` | ~500 | 2,298 contact rows | 0.22 req/row | new capability, no ATN equivalent |
| DAT parent-domain registry `[28]` | 24 | 5,933 schools (identity only) | n/a | 1 request/state league page |

**Parser traps for anyone building the DAT/TFRRS adapter `[18]`:** (a) list pages end with a constant **7-row
"Useful Links" footer table** — a row count that does **not** change when `?limit` changes is the tell that you are
counting page chrome, not data (this artefact already fooled one row-accounting pass); (b) **relay share is a
property of the view, not the platform** — on the same large-school list, relay-team rows with no `AthleteID` are
**34.9 %** of the default-limit *data* rows (400/1,146) but **14.7 %** at `?limit=1000` (775/5,287), so never apply
one cross-state relay fraction (and note the trap inside the trap: the pre-correction 429/1,176 figure counted the
same header/footer chrome warned about in (a)); across states the share tracks the **relay event count**, not
extraction quality — IN **14.7 %** of rows against 4 relay events of 11 `[18]` vs IL **10.2 %** (1,193/11,720)
against 3 of 15 `[14]`, i.e. a meet-format difference; (b2) **the relay block saturates at the default limit**: at
`Top 50` the relay sections are already complete — exactly **400 relay rows on *both* the Large and Small lists**
(4 relay events × 2 genders × 50) — so relay enumeration needs **no** `?limit`, while every individual event is
Top-50-truncated without it `[18]`; (c) the list id space has silent fallbacks (`1429_5490` returns the *Large* list), so
validate every fetch by its rendered title, not by id arithmetic.

**Two facts drive adapter design.** (1) The ATN rankings sweep is *cheaper per athlete* than the MileSplit roster
sweep (0.067 vs ≥0.106 req/athlete): MileSplit is a **coverage and grade** play, not a request-saving device.
(2) The cheapest whole-meet acquisition anywhere is MileSplit's raw result set (1 request/meet) `[07][20]`, but it
lives on the robots-disallowed `/api/` path `[27]` — the compliant substitutes are the timer blobs (AthleticLIVE,
6 requests/meet `[10]`) and association result files.

### Stable identifiers

**Cross-source identifier map and join keys** (the reconciliation spine):

| Identifier | Source | Namespace / note | Joins to |
|---|---|---|---|
| `AthleteID` | ATN `[04]` | National person key; 142,705 rows = 142,705 ids | everything ATN (`IDResult`, `TeamID`, `MeetID`) |
| `ani` / `a.ani` | AthleticLIVE `[10][12][25]` | **Is** the ATN AthleteID (959/959, 136/136, RTDB `ani`) | ATN profile URL composable with 0 requests `[12]` |
| `athlete.athleticNetId` | IHSA `[13]` | ATN athlete id on every state-finals finisher + relay member | ATN, via no search |
| `TeamID` (= `SchoolID`) | ATN `[05]` | School-instance id; bio `SchoolID` ≡ rankings `TeamID` (verified 3204) | ATN team page, division tree |
| `RelayTeamID` / bio `relayTeamMembers[].TeamID` | ATN `[05]` | Squad id space 29.1 M–32.7 M, **disjoint** from school TeamIDs | not a school key — naming trap |
| `divListId` / `BaseDivID` | ATN `[01][04]` | Season-scoped list vs stable state key (WI BaseDivID 638) | state/division scoping |
| `AthleteID` (MileSplit) | MileSplit `[27]` | **Independent namespace; no crosswalk to ATN** | join by name+school+state+grad year (65.9 % measured match) |
| `AthleteID` / `TeamID` (DAT/TFRRS) | DAT `[18][28]` | Independent namespace; 901 IN gender-teams / 452 slugs | join by name+school; IN rosters carry numeric ids |
| `SchoolID`, `ohsaaId`, `orgID`, `schoolId`, `Identifier` | associations `[13][19][06][23][24][25][26][29]` | per-state school keys (KS: `KSS0307`, MI: 5792, OH: 474) | school reconciliation, never ATN |
| Bound `comp id` ↔ AN `MeetID` | SDHSAA `[26]` | same calendar row: `ID 4931063` ↔ MeetID `278813` | association platform ↔ ATN meets |
| AL `MeetID` ↔ AN `MeetID` | ND `[25]`, IA `[12]` | AL 55421 ↔ AN 261403; AL docs carry the AN meet id | meet reconciliation without search |

**Unjoined today:** MileSplit↔ATN, DAT↔ATN (outside IN), association athletes↔ATN athletes (name+school only),
coach person ids (none published, by design).

### Athletic.net leverage

**Request model of the baseline (measured, `[02][03][04]`).** Discovery sweep of 12 states = 2,728 requests for
40,695 athletes. Profile history = 1 XHR/athlete (`GetAthleteBioData`), 3 resources/athlete under the current repo
plan (`Bio{tf}`, `Bio{xc}`, `ProfileHtml`). Whole meet = 3 XHR regardless of participant count. `athletic.net` is
Cloudflare-403 to every non-browser client in this entire programme; nothing external removes that constraint, but
the AthleticLIVE family removes the need for most profile fetches.

**Where external sources remove requests (measured unit costs, arithmetic shown).**

| Replacement | Arithmetic | Avoided (requests) |
|---|---|---|
| AthleticLIVE state-meet blob vs profile refresh | 242 Co2027 obtained for 6 requests → 242 − 6 | **236 per state XC meet** `[10]` |
| AthleticLIVE IA state T&F | 1,265 G11 rows for ~4 requests → 1,265 − 4 | **~1,261** `[12]` |
| Association + AL grade oracles (state-meet cohorts) | 3,993 measured grade-bearing Co2027 rows (table in Q4) at a real cost of ~35–70 artifact fetches → proxy cost 1 ATN profile fetch each | **~3,920–3,960** `[13][19][21][23][26][25][10][12][07]` |
| Postseason meet-id seeds | MHSAA 12 HTML → 89 MeetIDs; NSAA 1 page → 28; WIAA 1 archive+1 PDF → ~60; SD Bound rows embed MeetID (13) | **~190 meet identities for ~20 requests** `[15][24][06][26]` |
| Whole-meet pull vs per-athlete refresh | 3 XHR/meet ≈ N/4 profile refreshes (N=1,000 rows → ~250) | **~250 per 1,000-row meet** `[03]` |
| Repo profile-plan fix (not external) | 1 Bio instead of 3 for TF-only athletes: 2 × 40,695 | **81,390** `[02]` |
| Coach layer | 2,298 rows from ~500 requests; ATN has 0 coach fields | new capability, ~4.6 rows/request `[29]` |

**Net planning number `[INFERENCE]` (disjoint parts, not summed):** the state-meet oracle set alone retires
**~3,920–3,960 profile-class ATN requests** per season at a cost of ~35–70 artifact fetches (the AL rows inside
that set — IA 1,265, MN 242, ND 185 — are the same rows counted from the AL side in lines 109-110); the meet-id
seeds retire **~190 meet lookups**; every additional AL-covered regular-season meet retires **~120–236**
profile-class requests at 1 request per meet, but the number of AL-covered HS meets per season is `[UNVERIFIED]`
(only the state-meet floor is measured). Against this, the MileSplit roster pass *adds* 6,645 requests that the
current ATN sweep (2,728) covers more cheaply per athlete. The
honest Pareto conclusion: **external sources buy grade verification, coach contacts and non-ATN athletes — not raw
request reduction.**

### Athlete evidence

- **Grade evidence quality ladder** (best → worst): association state-meet PDFs/JSON with a printed grade column
  (IL, OH, WI, MO, SD, KS-XC, ND-`y`, NE, MI-finals, MN-archive) `[13][19][06][21][26][23][25][24][15][09]`;
  AthleticLIVE per-row `y`/`Yr` (MN, IA, ND) `[10][12][25]`; MileSplit roster `gradYear` + profile `Class of YYYY`
  `[27]`; Athletic.net `GradeID==11` in a 2026-season row `[04][05]`; DAT `FR/SO/JR/SR` event sheets `[28]`.
- **ATN profile existence**: 100 % by construction for the 40,695-athlete corpus; **65.9 %** (CI 60.2–71.1) of
  sampled MileSplit Co2027 boys are in it `[27]`; 100 % of AthleticLIVE-sampled rows carry `ani` `[10][12]`.
- **Independent grade corroboration is a state-meet phenomenon**: measured association-grade cohorts total 3,993
  Co2027-grade rows (see Q4) ≈ 9.8 % of the corpus `[CALC]`. Regular-season athletes normally have exactly one
  grade source (ATN or MileSplit).
- **Grade semantics trap (route- and season-dependent — read before joining anything)**: the *same token* means
  different cohorts on different surfaces. At-meet grade: `JR` on a 2025-26 result list ⇒ Co2027 (`[18]`'s
  `&year=JR` finger; proven by a transition test), `SR` there ⇒ Co2026. Current grade (roster sense, **observed only on
  Athletic.net so far**): ATN `GradeID==11` on a 2026-season row ⇒ Co2027 `[04]` — no 2026-27 roster has been observed
  outside ATN; DAT/TFRRS rosters still serve 2025-26, where `JR` is the Co2027 token `[18]`. **Illinois is worse than
  "not season-scoped": there the team-page `Year` column is an era-relative ordinal, not a grade** — Lane Tech's
  histogram is `13`×3, `14`×13, `15`×10 plus one `SO`, and the `13/14/15` codes map to `SO/JR/SR` of the *2024*
  3A Top Times list for the same AthleteIDs, so a literal reading mis-assigns a cohort by ~11 years `[14]`. Super-index
  rule: a team-page roster yields `Name → AthleteID` in both IN and IL, but a directly usable class token **only in
  Indiana**; on Illinois the class token must come from the competition lists `[14]`. Literal graduation year: MileSplit
  `Class of 2027` / roster `gradYear` `[27]`. Association PDFs print the in-year token (`Yr=11`, `Year`, `y`) for the
  season being published `[06][19][21][25][26]`. A join that mixes two of these is off by one class — **key the rule
  on the season the artifact covers, never on the host, path, parameter name or today's date**. `[18]` paid for this
  three times in one assignment; the documented rule is now: **2025-26 season ⇒ `JR` = Co2027** (proven independently on
  parent-domain HSR lists — 1,861 via `&year=JR`, `SR` there being Co2026 — *and* on subdomain team rosters, whose
  `config_hnd` season selector still shows `2026 HSR Indoor` selected with no 2026-27 option); **2025 outdoor season
  ⇒ `SR` = Co2025** (subdomain `list_data/5328`, meet dates Apr-Jun 2025). **The trap is assuming "today" implies the
  current academic year** — the live pages serve *last* season, which is exactly how an off-by-one class error gets
  published (`[18]`'s own roster estimate was corrected from ≈9,800 to **≈12,300 Co2027 from ≈901 roster requests,
  ≈0.07 req/athlete** `[INFERENCE, 28.6% JR over 419 sampled roster rows]`). **Worst offender:**
  AthleticLIVE `a.y`, where the vocabulary is also **per meet** (`"SR"/"JR"/"SO"/"FR"` or `"12"/"11"/…`) — read the
  selector off the document: on the 2025-26 MSHSL state XC doc `JR` is the Co2027 token (242 rows) `[10]`, and on the
  2026 Iowa state T&F doc `"11"` is (1,265 rows) `[12]`. Practical rule for the pipeline: never store a grade token
  without (`season_year`, `season_type`, `token`) and resolve the class at load time.
- **Privacy**: no source in this programme exposes athlete email/phone/address; all reports verified absence
  `[02][10][25][27]`.

### Recruiting information

- **Coach coverage is bimodal** `[29]`: (a) association-keyed AD registries — KS 526/526 with email from **one
  request**, MI 14/14, OH 13/13, IL 4/4, MN 7/9, WI 5/5 (AD rows); (b) school-published rosters — Bound IA/SD and
  NDHSAA where names are complete but email is per-school opt-in (Rapid City Stevens 36/43 rows had email; Albia
  32/121; 8 of 10 sampled schools 0).
- **Sport-scoped TF/XC coach with email is available in 5 states**: WI (`GetDirectorySchool`, 17/17 rows with
  email) `[06][29]`, IL (`staff2` + per-person email, 12/12) `[13][29]`, OH (myOHSAA, 15/18) `[19][29]`,
  IA/SD (Bound, names always; email per-school opt-in — in the 5-school samples the emails came from a *single*
  school each: Albia (IA), Rapid City Stevens (SD)) `[11][26][29]`, MN (coach names + email via `/api/coaches/<nid>`;
  4/10 records carry email) `[09]`.
- **MO and IN have no collectible official contact source at all** — verified dead ends, not pending work
  (`mshsaa.org` school pages carry no contact fields; `myIHSAA` is login-only, `/schools` 404) `[29][21][17]`.
- **Coaches are the only channel to athletes absent from ATN enumeration** — the 34.1 % MileSplit-only complement
  `[27]` can only be reached by a human contact or by a source that names them (rosters).

### Result evidence

- **State series**: WIAA tournament PDFs+HTML `[06]`; MSHSL archive PDFs 2017–2025 `[09]`; IHSA event summaries
  (with `athleticNetId`) `[13]`; MHSAA finals PDFs + 89 AN meet links `[15]`; IHSAA Hy-Tek PDFs + TFRRS hubs `[17]`;
  OHSAA state/district PDFs `[19]`; MSHSAA state PDFs only `[21]`; KSHSAA championship history 2000–2026 `[23]`;
  NSAA district HTML + S3 mirror `[24]`; NDHSAA via AthleticLIVE RTDB `[25]`; SDHSAA yearbook PDFs `[26]`.
- **Regular season**: MN/IA/WI/ND/KS/OH have working timer paths (AthleticLIVE blobs, PrimeTime JSON, timer
  files) `[10][12][08][23][20]`; MO's best path is ToU-restricted `[22]`; SD's Bound results are empty `[26]`;
  NE is district-HTML only `[24]`; MI and IL regular-season results exist only on ATN/MileSplit `[15][14]`.
- **ATN meet channel** `[03]`: 3 XHR delivers every row of a meet with `AthleteID`, `Grade/AgeGrade`, `TeamID`,
  `IDResult` — the correct refresh primitive once a MeetID is known; the missing piece is a **meet-calendar
  enumerator** (external seeds supply 190+/season; see gaps).

### Incremental use

| Cadence | Action | Requests |
|---|---|---|
| Weekly (season) | AL ES/blob delta on known meets (`frua`/`ua` conditional) + IATC/timer index diff | ~2–5/meet + 1/index `[12]` |
| Weekly (season) | MileSplit `sitemap.xml` meet delta (12 states) | 12/day `[27]` |
| Monthly | Association meet-id pages (MHSAA hubs, NSAA page, WIAA archive) | ~20/season `[15][24][06]` |
| Post-season | State-meet PDFs/JSON × 12 (grade + validation) | ~35–50 `[13][19][21][23][24][25][26]` |
| Per season | MileSplit rosters (new schools / grade changes) | 6,633 first pass, then deltas `[27]` |
| Quarterly | Coach/AD directories (rolling per school) | 500–2,900 depending on state `[29]` |
| Never | `/api/` MileSplit, `live.pttiming.com`, member logins | 0 — policy `[27][22][29]` |

### Access characteristics

| Class | Sources | Constraint |
|---|---|---|
| Public structured JSON/XML (curl) | KHSAAT API, IHSA API, MSHSL JSON:API, AthleticLIVE blobs+ES, ND RTDB, TFRRS RSS, PrimeTime JSON | none observed; no 429 anywhere in the programme `[23][13][09][10][12][25][18][08]` |
| Public HTML (curl) | WIAA schools, NDHSAA, Bound, NSAA S3, MHSAA, kshsaachamps, OHSAA portal | Cloudflare fronts without challenges; NSAA's WordPress host 403s (use S3 or the form host) `[06][25][26][24][15][23][19]` |
| Browser-session only | **Athletic.net** (all of it), SDHSAA's ATN-side checks | CF-403/`cf-mitigated: challenge` to every non-browser client `[01][04][05][26]` |
| Robots-restricted | MileSplit `/api/` + `/rankings` (paywalled) | `/teams`, `/teams/<id>/roster`, `/results`, `/meets` remain allowed `[27]` |
| ToU-prohibited | `live.pttiming.com` + its realtime DB (Karmarush v1.0, 2026-09-03); PT Timing MI | do not automate `[22][16]` |
| Login/paywall | `myIHSAA`, `collect.ihsa.org`, Clell Wade directory, IIAAA FinalForms | not attempted `[29][17]` |

### Recommendation

**Build the coach layer and the AthleticLIVE bridge first; treat MileSplit rosters as a grade/coverage pass, not a
saving; and keep Athletic.net for identity.** Concretely:

1. **COACH-DIRECTORY (new capability, highest certainty).** KSHSAA (1 request → 526 schools with AD emails), IHSA
   staff (emails), WIAA detail (sport-scoped coach emails), OHSAA portal, Bound IA/SD, NSAA/NDHSAA names,
   MHSAA admin API. ATN cannot supply any of this `[29][23][13][06][19][24][25][15]`.
2. **ATHLETIC.NET-SEED / RESULT-SOURCE (AthleticLIVE).** `ani` = AthleteID with grade and marks, no browser;
   6 requests per state meet `[10][12]`; ND state series on RTDB `[25]`.
3. **PRIMARY for IN.** TFRRS Indiana HSR lists: 1,861 Co2027 for 2 requests (`&year=JR`) `[18][28]`, with the
   gender-team roster sweep as the broad-coverage complement (≈901 requests ⇒ ≈12,300 Co2027 ≈ 0.07 req/athlete
   `[INFERENCE]`; the roster pages serve the 2025-26 season, so `JR` — not `SR` — is the Co2027 token). Caveat `[18]`: the
   meet-results surface collapsed in 2026 (56 track meets vs 1,022 in 2025; the IHSAA outdoor tournament is absent
   from the 2026 list) — build this adapter for the athlete/grade instrument and re-measure the meet surface from
   November 2026.
4. **PRIMARY/VALIDATION for the 12 associations** at the state-meet cohort level (grade oracles) `[13][19][21][23][24][25][26]`.
5. **Grade/coverage pass (not a saving).** MileSplit rosters at 6,645 requests for the full network `[27]`.
6. **DISCOVERY-ONLY/REJECT**: DAT outside IN `[28]`, PrimeTime live `[22]`, MileSplit `/api/` `[27]`,
   mtecresults API (authenticated) `[10]`.

### State coverage matrix

Legend: **E** = enumerate, **G** = grade evidence, **R** = results, **C** = coaches; ★ = publicly collectible
contact (email), ○ = names only, ✗ = none. `ATN` column = baseline corpus size and state `divListId` `[04]`.

| State | ATN baseline `[04][05]` | State association | MileSplit `[27]` | DAT/TFRRS `[28]` | AthleticLIVE / timers | Coach graph `[29]` |
|---|---|---|---|---|---|---|
| **WI** | 3,924 G11; divList 170770 | **E** 516 schools, TeamID, 1,796 team-season rows; **G** tournament `Yr=11`; **R** tournament PDFs+104 links; **C** ★ coach+AD table `[06]` | 597 teams; roster grad year; state meet 2,454 rows/382 Co2027 (robots-disallowed API) `[07]` | 976/936 records, 492 schools; no live HS `[28]` | PrimeTime JSON 319 (2025)/255 (2026) meets; K2/TrackSide tenants; exEntryLink→AN MeetID `[08]` | ★ 17/17 coach rows w/ email; 5/5 AD `[29]` |
| **MN** | 3,780 G11; divList 169465 | **E** 664 schools, team nid, MeetID 666673; **G** roster FR..SR + archive PDFs; **R** archive 2017–2025; **C** ★ coach names+emails via `/api/coaches/<nid>` `[09]` | 592 teams; roster grad year `[27]` | 753/121 records, 380 schools; 1 legacy result page `[28]` | AL blob: 6 req → 959 rows/242 Co2027/959 `ani` `[10]` | ★ AD 7/9 w/ email; coach email 4/10 records `[29][09]` |
| **IA** | 2,619 G11; divList 169178 | **E** 379 members, 365 TF programs; **G** Bound JR=Co2027; **R** Bound comp pages; **C** ○ staff names `[11]` | 411 teams `[27]` | 808/784 records, 408 schools; JR/SR sheets 2022 `[28]` | AL ES: 598 IA meets (306/306 w/ AN meet URL); state T&F 1,265 G11; `t.ani` 136/136 `[12]` | ○ 63 coach rows (3 w/ email); AD 2/20 `[29]` |
| **IL** | 6,009 G11; divList 169070 | **E** 828 schools (1 req); **G** 838 Jr T&F entries / 358 G11 XC; **R** event summaries w/ `athleticNetId`; **C** ★ staff emails `[13]` | 849 teams `[27]` | 1,744/1,596 records, 880 schools; 336 Co2027 from 2024 lists `[14][28]` | IHSA state live on `live.athletic.net/meets/74003` `[13]` | ★ 12/12 coach rows w/ email; 4/4 AD `[29]` |
| **MI** | 5,984 G11; divList 169407 | **E** 755 school IDs + 89 AN MeetIDs/season; **G** finals PDFs only; **R** postseason PDFs; **C** ✗ host AD only `[15]` | 895 teams; roster Class=grad year `[16][27]` | 1,612/1,618 records, 814 schools; no live HS `[28]` | Superior Timing archive; PT Timing ToU-prohibited; ~14 AN MeetIDs from MITCA `[16]` | ★ AD 14/14 w/ email (no coach rows) `[29]` |
| **IN** | 2,681 G11; divList 169137 | **E** 413 members, 91 TF/XC nodes; **G** Hy-Tek PDFs w/ grade; **R** state PDFs + TFRRS hubs; **C** ✗ none `[17]` | 533 teams (1 request); ≥101 outdoor+HS meets on pages 1–2 `[18][27]` | **PRIMARY (athlete layer)**: 1,816 numeric TeamIDs (929 TF + 887 XC)/452 slugs; **1,861 Co2027 in 2 requests** (`&year=JR`; 6,533 unfiltered); 1,180 rows/312 teams; **2026 meet surface collapsed** (56 vs 1,022 in 2025) `[18][28]` | Timing MD live page `[17]` | ✗ none — verified dead end `[29][17]` |
| **OH** | 6,361 G11; divList 170050 | **E** 815 schools, 2,353 div rows; **G** 561 G11 (1,067 w/ relays); **R** PDFs + AN delegation; **C** ★ portal emails `[19]` | 977 teams; roster `gradYear`; 41 Co2027 in a 10-team sample `[20]` | 1,749/1,679 records, 875 schools `[28]` | Finish Timing publishes 29 AN MeetIDs (id = "20"+ANID) `[20]` | ★ 15/18 coach rows w/ email; AD 13/13 `[29]` |
| **MO** | 3,621 G11; divList 169578 | **E** 646 orgs (592 full), no ids beyond school; **G** state PDFs `Year`; **R** state only; **C** ✗ none `[21]` | 700 teams `[27]` | 980/939 records, 492 schools `[28]` | PrimeTime files (14→9 meets/yr); `live.pttiming.com` **ToU-prohibited** `[22]` | ✗ none — dead end `[29][21]` |
| **KS** | 2,037 G11; divList 169215 | **E** 348 classified (+394 names); **G** 89 G11 XC rows; **R** champs 2000–2026 (20,887 rows); **C** ★ AD 339/339 `[23]` | 413 teams; state 2,331 athletes/325 teams `[23][27]` | 865/804 records, 436 schools `[28]` | Midwest Timing Supabase (336 events 2026) `[23]` | ★ 526 schools/1 request, 100 % AD email `[29][23]` |
| **NE** | 1,980 G11; divList 169689 | **E** 312 schools, 283 TF/253 XC; **G** grade per athlete in district HTML; **R** S3 mirror + district HTML; **C** ○ 274/275 TF, 243/240 XC `[24]` | 317 teams `[27]` | 634/632 records, 319 schools `[28]` | Black Squirrel; onlineraceresults state XC `[24]` | ○ 1,217 coach rows, 0 emails; AD 311 rows `[29][24]` |
| **ND** | 762 G11; divList 170039 | **E** 169 schools + co-op sheet (885 rows); **G** AL `y` (year in school); **R** state series via RTDB (meet 55421 ↔ AN 261403); **C** ○ 91 % of schools have ≥1 coach name `[25]` | 153 teams (smallest) `[27]` | 361/362 records, 181 schools `[28]` | Hero's Timing tenant; AL RTDB + ES index `[25][10]` | ○ names, 0 emails in 22 sampled pages `[25][29]` |
| **SD** | 937 G11 (148 team labels); divList 170271 | **E** 176 schools; **G** yearbook PDFs (106 G11 boys XC); **R** state PDFs only; **C** ○ Bound head coaches `[26]` | 196 teams `[27]` | 380/378 records, 190 schools `[28]` | no AL tenant evidenced; regular season results absent `[26]` | ○ 20 coach rows (4 w/ email); AD 2/10 `[29][26]` |

### Answers to the nine acceptance questions

**Q1 — Class-of-2027 athletes discovered per source (per state where known).**

| Source | 12-state yield | Per-state detail |
|---|---|---|
| ATN rankings sweep `[04]` | **40,695** | WI 3,924 · MN 3,780 · IA 2,619 · IL 6,009 · MI 5,984 · IN 2,681 · OH 6,361 · MO 3,621 · KS 2,037 · NE 1,980 · ND 762 · SD 937 |
| MileSplit rosters `[27]` | 41,145 (corpus) to ~62,400 `[INFERENCE]` | per-team measured: median 18, range 3–89; 716 Co2027 / 30 schools |
| AthleticLIVE (per meet) `[10][12][25]` | not summed; per-meet measured | 242 Co2027 (MN state XC), 1,265 G11 (IA state T&F), 185 ND state XC Class-A-Boys entries |
| TFRRS Indiana `[18][28]` | 1,861 (IN only) | 2 requests (`&year=JR`), disjoint lists (Large 1,155 + Small 706); `&year=SR` = 1,678 is **Class of 2026** |
| Association grade oracles `[13][19][21][23][26][25]` | 3,993 grade-bearing Co2027 rows `[CALC]` | see Q4 table |

**Q2 — Unique athletes after deterministic reconciliation.**
Dedupe keys, in priority order: (1) ATN `AthleteID`; (2) `ani`/`athleticNetId` (AthleticLIVE, IHSA) → ATN id,
exact; (3) association school id + name+grad year (school-level reconciliation); (4) normalized
name+school+state+grad year (MileSplit/DAT only).
Result: the ATN corpus 40,695 `[04]` is already deduped (142,705 ids = 142,705 rows `[05]`). AthleticLIVE and IHSA
add **zero** new athletes (every row carries an ATN id) `[10][13]`. MileSplit adds the measured complement:
34.1 % (CI 28.9–39.8) of 287 sampled Co2027 are absent from the corpus `[27]`. Complement model: MileSplit
population ≈ 41,145 / 0.659 ≈ **62,435**, of which ≈ 21,290 (0.341 × 62,435) are MileSplit-only; union with the
measured ATN corpus ≈ **62,000–62,400** `[CALC, INFERENCE]` — the 40,695-vs-41,145 corpus ambiguity and the
34.1 %-vs-39.8 % CI tail swamp that difference, so quote the union as **"≈ 62,000, ± 5,000"**, never as a
measurement. DAT outside IN adds ~336 name-only
athletes (IL) `[14]`; TFRRS IN is already inside the corpus by identity but not by grade.
**Uniquely unreconcilable:** MileSplit↔ATN has no id crosswalk, so its 34.1 % complement can only be *guessed* to be
new; a 100-athlete manual audit is the cheapest way to firm this up (next actions).

**Q3 — % with Athletic.net profiles.** 100 % of the 40,695-athlete corpus (by construction — `AthleteID` ⇒
`/athlete/<id>/…` URL `[02]`); 100 % of AthleticLIVE-covered rows carry `ani` (959/959 `[10]`, 136/136 `[12]`,
RTDB rows carry `ani`+`anli` `[25]`); 100 % of IHSA state-finals finishers carry `athleticNetId` `[13]`;
**65.9 %** (CI 60.2–71.1) of sampled MileSplit Co2027 boys `[27]`; unknown/0 for DAT-only and MileSplit-only
records (no crosswalk).

**Q4 — % with independent corroboration.** Definition: a second, non-ATN source carrying grade evidence for the
same athlete-season. Measured state-meet grade-bearing cohorts (2025 XC + 2026 TF mixes, stated per row):

| State | Co2027-grade rows | Artifact | Ref |
|---|---:|---|---|
| IL | 838 (TF) + 358 (XC) | IHSA entries/qualifiers | `[13]` |
| IA | 1,265 | AthleticLIVE state T&F | `[12]` |
| OH | 561 (1,067 w/ relays) | OHSAA state-final PDF | `[19]` |
| WI | 382 | MileSplit at WIAA state | `[07]` |
| MO | 300–350 `[INFERENCE]` | MSHSAA state PDFs | `[21]` |
| MN | 242 | AthleticLIVE state XC | `[10]` |
| ND | 185 | AL RTDB state XC | `[25]` |
| SD | 106 (XC boys) + TF grade rows | SDHSAA yearbooks | `[26]` |
| KS | 89 (XC) | kshsaachamps | `[23]` |
| **Total** | **3,993** `[CALC]` | | |

Corroboration rate ≈ 3,993 / 40,695 = **9.8 %** `[CALC]` for grade; the *identity* linkage rate is far higher
where `ani` exists (100 % of AL-covered rows). MI (finals PDFs only) and NE (district top-15 HTML) add an
unquantified few hundred.

**Q5 — % with an identified track/XC coach.** Athlete-weighted over the 40,695-athlete corpus, using per-state
"school has a named TF/XC coach" rates (WI 1.00 `[06][29]`, MN 1.00 `[09]`, IA 1.00 `[11][29]`, IL 0.75 `[13]`,
MI 0.00, IN 0.00, OH 0.59 `[19]`, MO 0.00, KS 0.00 (AD only), NE 1.00 `[24]`, ND 0.91 (20/22 schools, systematic sample) `[25]`, SD 1.00 (4-school `[26]` and 5-school `[29]`
samples) `[26][29]`):

$$\frac{22{,}193}{40{,}695} = 54.5\%$$

`[CALC]` — unweighted school mean 60.4 %. The gap is the three no-coach states (MI/IN/MO) plus KS's AD-only
directory. By *schools* rather than athletes: 8 of 12 states publish coach names; ND's per-sport rates are
77 %/73 %/86 %/86 % (B-XC/G-XC/B-TF/G-TF) on a 22/169-school systematic sample `[25]`.

**Q6 — % with a public professional coach contact.** Athlete-weighted, same weights:

$$\text{coach email}: \frac{14{,}407}{40{,}695} = 35.4\% \qquad \text{(AD email, for contrast)}: \frac{27{,}713}{40{,}695} = 68.1\%$$

`[CALC]` **Basis note:** coach email is a *school-clustered, opt-in* property, so the per-state rate is taken at
school level (rows-per-school would understate reach; both bases are shown where they differ). Per-state: WI 1.00
(17 rows across 4 of 5 schools `[29]`, plus 9/9 `[06]`), IL 0.75 (12 rows, 3 of 4 schools `[13][29]`), OH 0.59
(cell rate; 15 rows across 5 of 5 schools `[19][29]`), MN 0.40 (midpoint of the 30–60 % band `[09]`), IA 0.20 and
SD 0.20 (**1 of 5 sampled schools each** — IA: 3 rows, all Albia; SD: 4 rows, all Rapid City Stevens — n=5, so
treat as a band: ≈1–72 % at 95 %, not a rate) `[29]`; the 4-school SDHSAA/Bound sample independently found 0 public
emails on the non-www paths, consistent with per-school opt-in rather than a platform rule `[26]`. MI/IN/MO/KS/NE/ND
0.00 for coach emails (KS/MI still supply AD emails at 100 % of sampled rows `[29]`). The `data/coach-contacts.csv` corpus itself: **2,298 rows / 874 schools
/ 10 states**, of which 1,347 coach rows (51 with email) and 951 AD rows (573 with email) `[29]`. **Definitional
note (recount of the CSV, confirmed by `[29]`'s author):** rows carrying a non-empty `coach_name` number
**1,333** (51 with `public_professional_email`); rows carrying a non-empty `ad_name` number **2,207** (620 with
`ad_email`) because 1,264 sport rows deliberately carry *both* a sport coach and the school's AD for joins; the
AD-only rows (`ad_name` set, `coach_name` empty) number 943. The 1,347→1,333 delta is 14 IA Bound sport rows
published with a blank name (G-XC 7, B-TF 3, B-XC 2, G-TF 2); the `ad_name`-2,207 vs AD-role-951 gap is the same
mechanism in reverse. Both accounting bases are consistent — quote whichever matches the consumer's role
definition.

**Q7 — Athletic.net requests avoided through external discovery (arithmetic).**
Baseline (ATN-only, 12-state G11 boys): discovery sweep **2,728** requests for 40,695 athletes (0.067 req/athlete)
`[04]`, plus 1 profile request per athlete = 40,695 → **43,423** requests; under the repo's 3-resource profile plan
**124,813** (fixing that to 1 Bio saves 81,390 `[02]`).
External replacement, measured units:

- AthleticLIVE state XC: `242 − 6 = 236` profile-class requests avoided per state meet `[10]`;
- AthleticLIVE IA state T&F: `1,265 − 4 ≈ 1,261` `[12]`;
- Association grade oracles: 3,993 grade rows at ~35–70 real fetches → `3,993 − (35…70) ≈ 3,920–3,960` proxy profile requests `[CALC]`;
- Meet seeds: `89 + 28 + ~60 + 13 ≈ 190` meet identities for ~20 requests → ~190 meet lookups avoided `[15][24][06][26]`;
- Whole-meet pull: 3 XHR replaces ≈ N/4 profile refreshes (N=1,000 → ~250) `[03]`.

**Net `[INFERENCE]`: ~3,920–3,960 profile-class requests retired by the state-meet oracle set + ~190 meet
lookups**, with an unmeasured additional retirement of ~120–236 per AL-covered regular-season meet; against an
added 6,645-request MileSplit roster pass. The genuine savings lever remains ATN-side (1 Bio instead of 3;
whole-meet pulls).

**Q8 — states/providers accounting for the remaining gaps.**
1. **Coach contacts: MO and IN have none** (verified dead ends) — 6,302 of 40,695 athletes (15.5 %) sit in
   coach-less states `[CALC]`; KS/MI supply AD-only.
2. **Regular-season results: MO** (ToU), **SD** (Bound result column empty), **NE** (district HTML only),
   **MI/IL** (ATN/MileSplit only), **KS** (no weekly schedules; `meetsData` empty) `[22][26][24][15][14][23]`.
3. **Grade depth: MO/KS/NE/MI** state-meet-only cohorts `[21][23][24][15]`.
4. **ND's remaining gap is documentation, not access** — 22/169 schools sampled for coach coverage `[25]`.
5. **Athlete coverage: the 34.1 % MileSplit-only complement** and the 15,724 unresolved rows in the repo corpus
   (identity, not discovery) `[27][02]`.
6. **No free state-level Co2027 census** outside ATN — MileSplit per-state numbers are 3-team-sample bands `[27]`.

**Q9 — the 5–10 production adapters with the best verified-coverage/engineering ratio.**
Full detail in `data/adapter-ranking.csv`; ranked summary:
**1** MileSplit team-roster (12 states, 6,645 req, Co2027 census w/ grad year) · **2** IHSA API (IL: 828 schools
per request, `athleticNetId` per finisher, 100 % of sampled coach rows with email) · **3** MSHSL JSON API (MN:
grade + coach name/email, 664 schools) · **4** KSHSAA directory API (1 request → 526 schools with AD emails) ·
**5** AthleticLIVE blob/ES/RTDB bridge (ATN ids + grade + marks, no browser) · **6** TFRRS Indiana (1,861 Co2027 /
2 requests) · **7** OHSAA portal (sport-scoped coach emails) · **8** WIAA schools DB (1 request/school, coach
emails + ATN MeetID seed) · **9** NSAA directory + S3 (1,528 contact rows; 28 district MeetIDs) · **10** long-tail
official directories (Bound IA/SD, MHSAA admin API, NDHSAA pages).

### Source-role table

| Source | Role | IDs yielded | ATN requests avoided | Access class | Ref |
|---|---|---|---|---|---|
| ATN endpoints (rankings/meet/profile) | PRIMARY (identity) + baseline | AthleteID, TeamID, MeetID, IDResult, GradeID | n/a (is ATN) | browser session | `[01]–[05]` |
| MileSplit rosters | RESULT-SOURCE (roster) / grade pass | TeamID, AthleteID, MeetID, grad year | up to 3 searches/athlete `[02]`; no net saving vs sweep | robots-allowed HTML | `[07][16][20][27]` |
| MileSplit `/api/` + `/rankings` | REJECT (robots) | — | — | disallowed/paywalled | `[27]` |
| AthleticLIVE blobs/ES/RTDB | ATHLETIC.NET-SEED + RESULT-SOURCE | `ani`(=AthleteID), `t.ani`, AL MeetID, grade | 236/state meet; ≈1,261 IA state T&F | public JSON | `[10][12][25]` |
| IHSA API | PRIMARY (IL) + COACH-DIRECTORY + grade oracle | SchoolID, PersonID, athleticNetId, MeetId | 1,196 grade rows for ~10 req | public JSON | `[13]` |
| MSHSL API | PRIMARY (MN) + COACH-DIRECTORY + grade oracle | schoolId, team nid, MeetID 666673, grade | 1,494 rosters w/o ATN | public JSON | `[09]` |
| KSHSAA API | COACH-DIRECTORY + VALIDATION | schoolId, Identifier, AD email | 526 school searches | public JSON | `[23][29]` |
| TFRRS Indiana | PRIMARY (IN athlete layer) | AthleteID, TeamID, grade | 1,861 profile fetches | public HTML | `[18][28]` |
| TFRRS/DAT other 11 states | REJECT as result source | team identity only | 5,933 team searches once | public HTML | `[28][14]` |
| OHSAA (myOHSAA + blob + PDFs) | COACH-DIRECTORY + VALIDATION + seed | ohsaaId, MeetIDs, grade | 561 profile fetches | public HTML/blob | `[19]` |
| WIAA (schools DB + tournament) | COACH-DIRECTORY + seed + VALIDATION | TeamID/TeamSeasonID, orgID, MeetIDs | ≥1,796 team requests; 46–49 meet searches | public HTML/POST | `[06]` |
| MHSAA (hubs + PDFs) | ATHLETIC.NET-SEED + VALIDATION | 755 school IDs, 89 AN MeetIDs | 89 meet searches; 755 school searches | public HTML/PDF | `[15][29]` |
| IHSAA (IN) | VALIDATION only | tournament nodes, PDF grade | 0 athlete, 0 coach | public HTML/JSON | `[17]` |
| MSHSAA | VALIDATION only (grade) | org ids, none else | ~300–350 profile fetches | public PDF/HTML | `[21]` |
| KSHSAA/kshsaachamps | VALIDATION + DISCOVERY-ONLY | AssociationYearID, activity ids | 89 grade rows | public JSON/HTML | `[23]` |
| NSAA + S3 | COACH-DIRECTORY + seed + VALIDATION | 312 schools, 28 MeetIDs, grade rows | ≥300 meet searches | public bucket/forms | `[24]` |
| NDHSAA + ND RTDB | COACH-DIRECTORY (names) + seed + VALIDATION | 169 schools, AN MeetIDs, `y` grade, `ani` | per-meet profile fetches (185 entries) | public JSON/HTML | `[25]` |
| SDHSAA + Bound | COACH-DIRECTORY (names) + VALIDATION | 176 schools, Bound↔AN MeetID, grade | 176 school searches | public HTML/PDF | `[26]` |
| IA/IHSAA + Bound + IATC | PRIMARY (IA structure) + coach names | 365 programs, comp ids | ranking enumeration for 365 programs | crawl-delay 10 | `[11]` |
| PrimeTime (WI files / MO) | RESULT-SOURCE (WI) / DISCOVERY-ONLY (MO) | event ids, AN MeetIDs via exEntryLink | per-athlete profile fetches | public bucket; live host ToU | `[08][22]` |
| Athletic.net timers (Finish, Superior, Midwest, Black Squirrel, Hero's) | RESULT-SOURCE (per state) | AN MeetIDs, timer ids | meet searches | public HTML/JSON | `[20][16][24][23][25]` |
| Coach-graph directories | COACH-DIRECTORY | 2,298 rows / 874 schools | n/a (new capability) | mixed | `[29]` |

### Recommended acquisition flow

Mapping the plan's preferred flow (official indexes → candidate schools → MileSplit/DAT/result sources → known
Athletic.net ids → targeted ATN → canonical reconciliation) onto measured reality, with a minimum 12-state budget:

| Stage | Reality | Minimum budget (12-state) | Where reports contradict the flow |
|---|---|---|---|
| 1. Official indexes | ✅ abundant — associations, KSHSAA/IHSA/MSHSL APIs, S3 mirrors | ~60 requests (12 states, 1–10 each) | IN/MO indexes carry no ids; MI's division list is JS-only `[17][21][15]` |
| 2. Candidate schools | ✅ complete (6,000+ schools across sources) | 24 (DAT leagues) + 4–26 per association | MileSplit slug↔association slug mismatch (IA) `[29]` |
| 3. MileSplit/DAT/result sources | ⚠️ MileSplit yes; **DAT no outside IN** | 6,645 (MileSplit rosters) + 2 (TFRRS IN) | DAT is not a discovery source in 11 states `[28]` |
| 4. Known ATN ids | ✅ where AL/IHSA/ATN meet seeds exist; ❌ from MileSplit/DAT | ~0 extra (ids come with stages 3/5) | MileSplit has *no* ATN links at all `[27]` |
| 5. Targeted ATN | ✅ meet-level (3 XHR/meet); ❌ no calendar API | 3 × (meets not already seeded) | meet discovery is the open gap `[03]` |
| 6. Canonical reconciliation | ⚠️ exact on ATN ids; name-based otherwise | free | 34.1 % MileSplit complement unresolvable by id `[27]` |
| 7. Coach layer | ✅ 10 states, best built first | ~500–2,900 (directory dependent) | MO/IN have no source at all `[29]` |

### Canonical model mapping

| Canonical entity | Fields | Primary source(s) | Join key | Unjoined / caveat |
|---|---|---|---|---|
| **CanonicalSchool** | name, city, state, enrollment, class/division, co-op, website | associations `[06][09][13][15][17][19][21][23][24][25][26]`, DAT registry `[28]`, MileSplit teams `[27]` | association school id (MI 5792, KS KSS0307, OH ohsaaId, IL 0101, ND `<id>`, WI orgID) | no ATN TeamID from associations except OH (policy) and AL `t.ani`; DAT's 5,933 canonical rows have empty ATN columns `[28]` |
| **CanonicalCoach** | role, name, sport, gender side, professional email | `[29]` winners + `[06][13][19][09][11][26][24][25][15]` | school + sport + role (no person id published) | MO/IN absent; emails school-opt-in in IA/SD; ND/H names-only |
| **CanonicalAthlete** | name, school, grad year, gender, `AthleteID`, profile URL | ATN `[04][05]`; AthleticLIVE (`ani`) `[10][12][25]`; IHSA `athleticNetId` `[13]`; MileSplit roster (grad year) `[27]` | **ATN AthleteID** where present; else normalized name+school+state+grad year | 34.1 % MileSplit complement has no ATN id `[27]`; DAT/MileSplit athlete ids are separate namespaces |
| **CanonicalMeet** | name, date, venue, division/class, source meet id | association seeds `[15][24][06][26][25]`, AL `[10][12]`, MileSplit `[27]`, DAT idx `[28]` | ATN MeetID where exposed (MHSAA/WIAA/SD/AL/ND); else name+date+state | no ATN meet-calendar enumerator `[03]`; MileSplit MeetIDs are not date-monotonic `[27]` |
| **CanonicalPerformance** | event, mark, wind, round/heat, FAT, place, result id, date | ATN `[03][04]`; AL blobs `[10][12][25]`; association PDFs `[13][19][21][23][26]`; MileSplit `/api/` (disallowed) `[27]` | ATN `IDResult` / (`AthleteID`,`MeetID`,`EventID`) | relay leg identity lives in `relayTeamMembers`/`Members[].IDAthlete`; AL relays carry legs with grade `[12][25]` |

### Remaining gaps + ranked next actions

Ranked by marginal verified coverage per engineering effort:

1. **Build the coach adapter set** (KSHSAA → IHSA → WIAA → OHSAA → Bound IA/SD → NSAA/NDHSAA → MHSAA). Closes the
   Q6 gap (35.4 % → potentially ~60–68 % where AD emails count) and creates the only channel to non-ATN athletes.
   Effort: days. Evidence: `[29]`.
2. **Build the AthleticLIVE bridge** (ES + blob + ND RTDB). Turns every timer-hosted meet into ATN ids + grade +
   marks with no browser. Effort: ~1 week; needs the 258-tenant origin list and one verification of the `ani`
   interpretation per state `[10][12][25]`.
3. **Run the MileSplit roster pass** for the 12 states (6,645 requests, paced under robots) to (a) confirm grad
   years and (b) sample-audit the 34.1 % complement: **audit 100 MileSplit-only athletes against ATN by hand** —
   this single measurement decides whether MileSplit is a discovery source or a validation source. `[27]`
4. **Close the ATN meet-calendar gap**: capture `/events` + a division home in a browser session; if no calendar
   exists, keep assembling MeetIDs from the 190/season association seeds `[03][15][24][06][26]`.
5. **Add the state-meet grade oracles as a scheduled validation job** (~35–50 requests/season) to corroborate the
   ~4,000-row state-meet cohort `[13][19][21][23][24][25][26]`.
6. **Qualify MO and IN contacts as out-of-scope** (dead ends) and record it, rather than leaving them "pending" `[29]`.
7. **Backfill ND** beyond the 22-school sample (169 school pages, 1 request each) and finish the ND coach table
   coverage census `[25]`.
8. **Feed the repo's 15,724 unresolved rows** through the `ani`-carrying channels (AL/IHSA/ND RTDB) before any new
   ATN sweep — identity resolution there is free `[10][12][13][25]`.

**A follow-up capture/qualification session must fetch** (in order): (a) one Athletic.net meet response
(`GetMeetData`/`GetAllResultsData`) to confirm the 3-XHR whole-meet shape `[03]`; (b) one AthleticLIVE state meet
end-to-end to verify `ani`→profile for 3 athletes `[10]`; (c) one MileSplit grad-year page for a state not yet
checked (only WI/OH/SD were verified) `[27]`; (d) one Athletic.net XC rankings call (`xcRankings`) since all five
ATN reports are TF-outdoor-scoped `[04]`; (e) one PrimeTime WI meet file to confirm grade parsing outside the two
sampled meets `[08]`.

### Contradictions and how they were resolved

1. **Midwest corpus size**: `[27]` lists per-state Grade-11 corpus counts (WI 3,961 · MN 3,845 · IA 2,643 ·
   IL 6,136 · MI 6,041 · IN 2,710 · OH 6,390 · MO 3,661 · KS 2,059 · NE 1,993 · ND 767 · SD 939) that sum to
   **41,145** (checked by addition), yet the same sentence prints the total as **46,145**. `[04]` measures
   **40,695** by direct enumeration (its own per-state table). Resolution: use **40,695** (measured sweep) as the
   working corpus number; treat 41,145 as the sum of `[27]`'s per-state primary-state attribution values; treat the
   printed 46,145 as an uncorrected +5,000 slip (the delta is exactly 5,000, and the per-state list is the only
   self-consistent decomposition). Confidence: high on 40,695/41,145, high on the slip.
   *Revision note:* `[27]`'s per-state values changed between the version first read (which summed to 41,145 with the
   same printed total) and the version read at 23:38 — same conclusion, different distribution.
2. **MN coach emails**: `[29]` records MN as "AD only"; `[09]` documents head-coach name + `field_email` via
   `/api/coaches/<nid>` (4/10 records on one team, both head coaches with email). Resolution: coach emails **do**
   exist in MN (better-evidenced for that specific claim, with a raw endpoint), but the sample is 2 teams — carried
   as a 30–60 % band `[09]`.
3. **MileSplit results API**: `[07][20]` use `/api/…/performances` (1 request/meet); `[27]` shows robots.txt
   disallows `/api/`. Resolution: the measurement stands, the path is **not usable in production**; use AL blobs or
   association files instead `[27]`.
4. **NSAA access**: `[24]` reports `nsaahome.org` 403 to curl; `[29]` uses
   `secure.nsaahome.org/nsaaforms/direxportscreen.php` successfully. Resolution: the WordPress host blocks, the
   forms host and the S3 mirror do not — both are correct `[24][29]`.
5. **SD regular season**: `[26]` finds Bound result columns empty while `[11]` shows IA Bound results working.
   Resolution: platform works, SD publishing is the gap `[26]`.
6. **Report revisions during consolidation** (not a source contradiction; recorded for traceability). `[18]` was
   revised twice while this consolidation ran: (i) at 23:37 the earlier "1,789 Co2027 in 2 requests" became a
   `&year=SR` count, and (ii) at 23:41 — after agent 28 challenged the class attribution — the Class-of-2027 figure
   was corrected to **1,861 via `&year=JR`** (Large 1,155 + Small 706, disjoint), with **1,678 (`&year=SR`) = Class of
   2026**. Evidence for the correction: the `Year` column is the **at-meet grade**, proven by a transition test (of
   1,366 athletes on both the 2025 and 2026 HSR lists, 1,330 advanced exactly one grade; `JR ∩ SR = 0` on the Large
   list), and cohort completeness is proven by the gender split summing exactly (`f 393 + m 607 = 1,000` on the SR
   view; `year=JR` reaches 1,155 > 1,000, so no blanket cap binds). This report carries the corrected values.
   *Residual wart, since fixed:* `[18]`'s evidence-appendix row for the Small-list SR fetch initially still labelled
   the 678 as "Class-of-2027" while its own body said Co2026; the author corrected that row at 23:42, so the file is
   now self-consistent (re-verified by grep at 23:43). `[18]` now also carries its own 14-item **"Corrections and
   retractions log"** (lines 884-910, after its evidence appendix), recording each retraction with the check that
   settled it; four of the 14 are the same season/grade-token mistake — independent corroboration of the pipeline
   rule below.
   `[27]` was likewise refreshed (per-state corpus values changed, same 41,145 sum, same printed 46,145).
   **Generalisation carried into §Athlete evidence: grade labels are scoped to the season the artifact covers, not to
   the host or to today's date — on the same platform the same `&year=SR` maps to Co2026 on a 2025-26 artifact and to
   Co2025 on a 2025-outdoor artifact, and the live roster pages still serve 2025-26 (no 2026-27 option exists), so
   "current grade" is a trap.**

### Evidence appendix

**External fetches made by this report: none.** Every number above is traceable to
`research/midwest/01-…29-*.md`, `data/{atn-id-fields,coach-contacts,dat-team-index,milesplit-coverage-matrix,school-alias-map}.csv`,
`synthesis/atn-endpoint-groundtruth.md`, or arithmetic on those (§ Q5–Q7 show the arithmetic).

**Peer confirmations (IRC, 2026-09-19 23:40–23:45) — status of the inputs, not new data.** `[18]` (final: 1,861 Co2027 via `&year=JR`, 6,533 unfiltered, plus the two scope caveats and the `&year=SR` = Co2026 correction), `[25]` (final; corrected attribution: 762 G11 belongs to `[04]`, ND coach rates are a
22/169-school sample), `[26]` (final; corrected coach accounting: 6 role rows/school, 0 public emails in its
4-school sample), `[29]` (final; confirmed the 1,347/951 role-accounting vs 1,333/2,207 column-accounting split,
including the 14 blank-name IA rows). Two of these corrections changed cells in this report and in
`data/source-coverage-matrix.csv`; the files were re-read after the corrections, and every load-bearing number was
re-grepped against the current file contents at 23:39–23:41 (`[27]`'s separators and per-state distribution had
also changed — see Contradiction 1's revision note).

| Artifact | Method | What it proved |
|---|---|---|
| 29 reports in `research/midwest/` | read (extract + section reads) | all per-state facts cited above |
| `data/coach-contacts.csv` (2,298 rows) | CSV parse | coach/AD yields per state (1,347/51, 951/573) |
| `data/milesplit-coverage-matrix.csv`, `dat-team-index.csv`, `school-alias-map.csv`, `atn-id-fields.csv` | CSV parse / header check | platform counts (6,633 teams, 22,527 DAT records, 5,933 schools, 45 id fields) |
| `synthesis/atn-endpoint-groundtruth.md` | read | ATN endpoint/payload facts, 15,724 unresolved rows |
| Arithmetic (Q5/Q6 weighting, Q4 sum, Q7 savings) | computed in-kernel, shown inline | all `[CALC]` figures |
| `ls`/dir listings of `research/midwest/` and `data/` | bash | confirmed 25/26/29 present and the 5 input CSVs |
