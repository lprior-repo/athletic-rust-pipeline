# 05. Source matrix — what is reachable, per jurisdiction and per source family

Condensed from the completed research corpus: the 46-file Midwest research set (`research/midwest/*.md`),
the 12 national lanes (`research/sources/<lane>/{SOURCE_REPORT.md,schema.json,coverage.json}`), the data
products (`data/*.csv`) and the pipeline outputs (`reports/`). This file **supersedes**
`synthesis/0{0..4}-*.md` wherever the lanes disagree; every superseded number is named in §5.

Citation convention: `[state-assoc-greatlakes]` = `research/sources/state-assoc-greatlakes/coverage.json`
(same for the other 11 lanes); `[research/midwest/NN-slug.md]` = the numbered report; `[data/x.csv]`;
`[synthesis/NN-*]`; `[reports/*]`. `[INFERENCE]` marks anything not directly captured or counted.
`unknown` = the corpus does not contain the fact. No number here was re-fetched; all reads are from disk.

Baseline facts used throughout:

- AthleticLIVE tenant plane: **256 tenants**, 153,524 meet docs in the package inventory, **129,203 (84.2 %)
  carry an Athletic.net meet id**; wildcard ES count 153,774 (250-doc delta vs the package = index churn
  since the package was built, both printed) `[national-aggregators]`.
- Athletic.net's own API is anonymously reachable with browser-grade headers (2026-09-20 re-verification;
  supersedes the earlier blanket-403 note) `[synthesis/08-gap-closure-2026-09-20.md §1]`.
- Coach outreach ceiling from association directories alone: **3 tier-1 jurisdictions publish a coach email**
  (IL, OH, WI); **5 publish an AD email** by the lane's counts block vs **4 named in its per-jurisdiction
  `fields` blocks** (IL, KS, OH, WI) — both numbers retained, see §5 `[coach-directories-national, jq over
  coverage.json]`.

---

## 0. What the authorities are, and how to read the numbers

### 0a. The 12 national lanes (each is a directory with `SOURCE_REPORT.md` + `schema.json` + `coverage.json` + `samples/`)

| Lane | Jurisdictions | What it establishes | Anchor numbers |
|---|---|---|---|
| `athleticnet` | all 51 | Athletic.net's own API grammar, ids, entitlement blur, robots | 53 state nodes / 51 mapped; `blurAfterDepth=5`; robots 1,080 B, `/api/` not disallowed |
| `milesplit-national` | all 51 | Per-state hosts, teams index, roster grad-year semantics, paywall boundary | 51/51 hosts 200; 26,562 HS teams; 7,500-URL rolling sitemap |
| `national-aggregators` | national | AthleticLIVE tenant plane + DAT + TFRRS + RunnerSpace + MaxPreps + AAU + World Athletics + USATF + ccr | 256 tenants; 153,524 docs / 129,203 with AN id; DAT 22,527 team rows; TFRRS 3 HS instances |
| `timing-providers-national` | national (16 provider families) | Per-provider hosts, formats, request costs, join keys | AL state census 70 buckets; TFRRS IN list 1,671 ids; FINISH `20+ANID` |
| `coach-directories-national` | all 51 | Tier-1 directory status per association | 22 collected; coach email 3; AD email 5 (counts) vs 4 (fields); 26 client-rendered; 3 robots-disallow |
| `state-assoc-greatlakes` | IL IN MI OH WI | Consolidation of the Midwest corpus into the national contract (no new fetches) | IL 828 re-derived; OH 815 re-derived; MI 755 / IN 413 quote-only |
| `state-assoc-plains` | IA KS MN MO ND NE SD | Same, for the plains set | MN 664; ND 169; NE 312; KS 348 + 526; MO 1,006 |
| `state-assoc-mountain` | AZ CO ID MT NM UT WY | Association directories, athlete surfaces, blocks | AZ 287; CO 378; ID 174; MT 184; NM 126/156; UT 162; WY 70 |
| `state-assoc-midatlantic` | DC DE MD NJ NY PA CT MA RI NH VT ME | Association sweeps; where universes are unenumerable | DC 45; MD 201; MA 381; CT 440; RI 128; ME 152 |
| `state-assoc-southeast` | FL GA NC SC VA WV | Association sweeps + grade oracles | FL 880; GA 456/457 + 23,435 staff rows; NC 451 + 77 admins; SC 229; WV 119 |
| `state-assoc-westcoast` | CA OR WA AK HI NV | Association sweeps + participation files | CA 1,608; OR 299; WA 752; AK 217; HI 95; NV 127 |
| `state-assoc-southcentral` | AL AR KY LA MS OK TN TX | Association sweeps + result/coach artifacts | AR 502; KY 290; OK 482; TN 456; TX 1,566; LA 306 (derived) |

### 0b. How to read §1 and §2

- "Re-derivable from retained bytes" means a file on disk re-produces the count; quote-only counts say so.
- Results columns list the **official** surfaces first, then timers, then the AN linkage. A timer inside
  AthleticLIVE is a read path **into** the Athletic.net ecosystem, not independent corroboration
  `[synthesis/00-executive-summary.md]`.
- Coach/email columns distinguish published professional emails from names-only and from refused paths;
  privacy filters applied everywhere are listed in `synthesis/05-compliance-and-risks.md §2`.
- `unknown` means the corpus does not contain the fact; it is never a zero and never a guess.

## 1. Jurisdiction matrix

### 1a. The 12 target Midwest states

| State | Association(s) | School-universe enumerator · measured count · re-derivable from retained bytes? | Coach/AD directory: fields, which are public professional emails, robots | Official result/meet surfaces (association, timers, AN linkage) | Grade evidence available | Athletic.net relationship (link/seed/absent) | Best first adapter |
|---|---|---|---|---|---|---|---|
| **WI** | WIAA (wiaawi.org; schools.wiaawi.org) `[state-assoc-greatlakes]` | 516 published; NOT enumerated (1 of 26 directory letters fetched → 39 schools); team-seasons with TeamIDs = **1,796**, returned in 4 requests/season (462 boys TF / 462 girls TF / 437 boys XC / 435 girls XC) `[state-assoc-greatlakes]` — **no** re-derivation (recipe only, 1 letter captured) | `GetDirectorySchool?orgID=`: Superintendent, Principal, AD, City-Wide AD, Assistant AD, per-sport head coaches **with published emails**; 9-school sample any TF/XC email 9/9 (BXC 8/9, GXC 9/9, BTF 7/9, GTF 8/9); ~195 KiB/school; no change signal; no robots refusal `[state-assoc-greatlakes]` | WIAA tournament file set **104 result-file links/season**; formats = AN export, Hy-Tek, HTML, legacy .txt (63 on the girls XC archive alone, incl. 1999); PrimeTime JSON archive 2017–2026 (2026 YTD 255 meets/200 PDFs, 2025 319/266; entry links 2026: AN 195 / MS 0 / none 60 vs 2025 MS 163 / AN 76 / none 80) — live layer off-limits without licence `[state-assoc-greatlakes][research/midwest/08-wisconsin-timing-providers.md]` | Tournament PDFs carry `Yr`: 26-file 2026 sample = **2,653–2,786 grade-11 rows** (two matchers, range retained); D1 Regional 5A 234 athletes / 71 g11; 24/26 pdftotext-OK, 2 need OCR (1.06 s/page) `[state-assoc-greatlakes]` | Tournament result files **are AN exports**; AN MeetIDs in instruction PDFs + RUNMEET titles 13/26 sampled; `/events/usa/wisconsin/2026-5-26` date index; no AN ids in the school directory `[state-assoc-greatlakes][athleticnet]` | **WIAA schools DB** (rank 8): 4 requests for team-seasons, 1/school for coach+AD emails `[synthesis/03-adapter-ranking.md]` |
| **MN** | MSHSL (mshsl.org) `[state-assoc-plains]` | **664** unique `/schools/<slug>` in sitemap page 1 (fresh, 341,086 B) → 1,328 roster targets (664 × 2 XC+TF); **yes**, re-derivable from that sample | School page AD rows + `/jsonapi/views/teams/list_school` team nodes + `/api/coaches/<nid>` per team: head/assistant **names + school-domain professional emails**; `Non-MSHSL`/`Sub-Coach` levels filtered; `FieldWorkPhone` ignored `[state-assoc-plains]` | `/api/team-data/scores` and `/api/team-data/schedule/<school>/<level>/<activity>` (schedule JSON carries the **AN MeetID**, e.g. 666673 = 2026 state T&F); regular season is NOT published by MSHSL — it lives with timers; state XC via AthleticLIVE blob (6 requests → 959 rows → 242 Co2027) `[state-assoc-plains][synthesis/02-state-playbook.md]` | `/api/team-data/roster/<school>/<level>/<activity>` carries `student_grade` — **in-season only**; state-meet grade via AthleticLIVE rows `[state-assoc-plains]` | Strongest per-meet join of the seven plains states: MeetID in the schedule JSON; AN Live meet 666673 linked from the state-meet page `[state-assoc-plains][research/midwest/09-minnesota-mshsl.md]` | **MSHSL JSON API** (rank 3): grade + coach name/email + AN MeetID in one family `[synthesis/03-adapter-ranking.md]` |
| **IA** | IHSAA (iahsaa.org — IOWA) + IGHSAU `[state-assoc-plains]` | Fresh measure **362** distinct `/schools/<slug>` (380 `<tr>`, 363 link rows) vs report figure **379** (one day earlier) — both printed; IGHSAU BEDs 379 with grade 9/10/11 enrollment (368/379 join); Bound index 444 entries = 378 HS + 51 middle/JH + 15 other (not a member list); 351 schools classed in FINAL T&F 2026 `[state-assoc-plains][research/midwest/11-iowa-ihsaa-ighsau.md]` — **yes** for 362 | Association publishes School/Nickname/Colors/Conference only — no AD or coach field exists. Best source is Bound `/ia/schools/<slug>/directory/new`, but `gobound.com` robots has `Disallow: /*directory` and the compliant GET returned a 118-byte 403: **refused**; the earlier [29] browser-session claims are retracted by the lane `[coach-directories-national][state-assoc-plains]` | State T&F 2021–2025 + state XC 2024–2025 on Wayzata AthleticLIVE (73956 / 53818 / 58504 / 41636); IGHSAU 2026 state T&F PDF (1.2 MB); IHSAA state-qualifying XC PDFs; IATC weekly meet index; regular season timer-fragmented (8+ hosts) `[state-assoc-plains]` | AthleticLIVE rows carry grade: state T&F meet 73956 → **1,265 grade-11 athletes with AN athlete ids** `[synthesis/02-state-playbook.md][research/midwest/12-iowa-wayzata-results.md]` | **Zero** occurrences of `athletic.net` across all 41 saved Bound-origin pages; 100 % of 306 indexed IA HS meets carry the AN meet URL; AL meet id → AN core meet id **unverified for IA** (one probe, 404) `[state-assoc-plains][synthesis/02-state-playbook.md]` | **Bound (rag, 10 s delay) + AthleticLIVE tenant chain** for grade + `ani`; association = validation only. No adapter module exists (`grep` for iahsaa/ighsau = 0 Rust matches) `[state-assoc-plains]` |
| **IL** | IHSA (ihsa.org; api.ihsa.org) `[state-assoc-greatlakes]` | **828** = 801 full + 26 approved + 1 associate (604 boundary / 224 non-boundary; 677 public / 151 private; 126 CPS); **yes** — `evidence/gaps/38/il-schools.json`, md5 `498179f68c66793396dd4c6123a4a4bc`, parses to exactly 828 rows (ledger 451,182 B vs file 451,188 B — §5) | `/staff2` + `/staff/<pid>/email`: per-sport head coaches and ADs, per-row email reveal; 49/49 sampled rows HasEmail; `data/coach-contacts.csv` 12 coach + 16 AD emails; census 125/125 Co2027 with coach email `[state-assoc-greatlakes][data/coach-contacts.csv]` | T&F **state finals only** via IHSA API (`/v1/track-field/events/<id>/summary`); XC qualifier API 1,214 boys (358 g11); DAT Top Times is 2023/2024 legacy = 336 Co2027 at the default cut, 1,580 at `limit=1000`; DA IL has no live HS data `[state-assoc-greatlakes][research/midwest/14-illinois-directathletics.md]` | 6 static qualifier pages = **3,342 entries with grade** (838 junior); XC boys g11 358; class splits 1A/2A/3A g11 97/96/125 `[state-assoc-greatlakes]` | **Association publishes AN ids directly**: `athleticNetId` on every finisher and every relay member, plus team `athleticNetId`; meet 74003 (live) / 74002; hard-coded meets 663319–663324; rankings lists 168416 / 169070 (6,009 g11) `[state-assoc-greatlakes][athleticnet]` | **IHSA API** (rank 2): 828 schools/1 request + qualifier grades + coach emails `[synthesis/03-adapter-ranking.md]`; adapter implemented `[state-assoc-greatlakes]` |
| **MI** | MHSAA (mhsaa.com; my.mhsaa.com) `[state-assoc-greatlakes]` | **755** schools with stable MHSAA ids (co-op program ids are a separate id space); **NO** re-derivation — the enrollment-list PDF bytes were not retained and both corpus `/schools` copies (145,587 B each) are SPA shells with 0 `<tr>` `[state-assoc-greatlakes]` | No statewide directory (school page 404; AD/coach services login-gated). AD API `my.mhsaa.com/.../AdministrationDirectory?SchoolId=` is **refused**: robots `Disallow: /DesktopModules/`, 2 refusals logged; the 14 AD-with-email rows in `coach-contacts.csv` come from the earlier study; pipeline census 6 coach rows; the adapter was removed from the crate `[state-assoc-greatlakes][coach-directories-national][synthesis/10-measured-census.md]` | MHSAA finals PDFs (XC 14/season, UP TF 6) with `Year`/`Yr`; **no MHSAA-hosted regional results**; host-school Meet-Info PDFs ~84 opportunities/yr (35/48 TF, 18/36 XC to date) `[state-assoc-greatlakes][research/midwest/15-michigan-mhsaa.md]` | Finals PDFs only: 2025 LPG D1 187 rows / 47 g11; 2026 UP boys D1 103 / 30; annual ≈2,000 rows `[INFERENCE: extrapolated from two sampled PDFs]`; regional grade evidence does not exist `[state-assoc-greatlakes]` | **89 AN MeetIDs/season** published (48 TF regionals + 5 finals + 36 XC regionals) from ~12 HTML requests; AN-side inventory: 215 TF + 117 XC distinct AN MeetIDs across 13 captured pages; MITCA adds 11 `[state-assoc-greatlakes][athleticnet]` | **MHSAA tournament pages for AN meet seeds** (≈20 requests), then MileSplit rosters for Co2027 `[synthesis/02-state-playbook.md][synthesis/03-adapter-ranking.md]` |
| **IN** | IHSAA (ihsaa.org — INDIANA; do not confuse with Iowa) `[state-assoc-greatlakes]` | **413** = 408 full + 5 provisional (356 public / 57 non-public; districts 132/143/138); **NO** re-derivation — retained copies are SPA shells with 0 `<tr>`; no public numeric school id; 391 declared XC postseason slots across 25 sectionals `[state-assoc-greatlakes]` | **None public**: myIHSAA is behind authentication and was not attempted; 0 coach rows in `coach-contacts.csv` `[state-assoc-greatlakes][synthesis/10-measured-census.md]` | TFRRS Indiana (indiana.tfrrs.org, one of only 3 HS instances) certified meet results (508 meet ids via list 5328) + HSR lists (A 3,978 / B 2,555 / union 6,533); Timing MD live page for the 2026 state final (no grade on page); DAT IN = 1,816 team records, the only state with live HS performance lists `[state-assoc-greatlakes][national-aggregators]` | TFRRS `Year` column + `&year=` filter: `JR` = **1,861 Co2027** in 2 requests (`SR` = 1,678 = Co2026); HSR list 5489 = 1,671 athlete ids with grade + Year filter `[research/midwest/18-indiana-directathletics-milesplit.md][national-aggregators]` | **Zero** Athletic.net links in every captured IHSAA page; TFRRS/DAT carry no AN ids; provider switch 2024-25 TFRRS → 2025-26 MileSplit ids must both be stored `[state-assoc-greatlakes][synthesis/02-state-playbook.md]` | **TFRRS Indiana** (rank 6): 2 requests → 1,861 Co2027 with grade, independent of AN `[synthesis/03-adapter-ranking.md]` |
| **OH** | OHSAA (ohsaa.org; officials.myohsaa.org) `[state-assoc-greatlakes]` | **815** schools; **yes** — `evidence/gaps/45/ohsaa-enrollment.html` (228,567 B) holds one table with 816 `<tr>` = 1 header + 815 data rows and 815 distinct `OhsaaSchoolId` (100–1000027); class cell 267 AAA / 267 AA / 267 A / 14 blank; 742 have a deterministic AN path; division rows xc 556/468, tf 685/644 (as printed) `[state-assoc-greatlakes]` | myOHSAA, **3 requests/school**: `SearchSchool` → `SportsInformation` (XC + T&F head coaches, `mailto:` emails, Div-I..V tags) → `AthleticDirector` (name + email + assistants); 56 sampled cells / 33 named, **100 % of named cells carry email**, AD 5/5; OATCCC association 32 contacts all with email; census 456/456 Co2027 with coach email. **Caveat:** the census run's rustls client was connection-reset by the host (openssl handshake succeeds with ALPN, fails without → edge filters ClientHello); no impersonation attempted, so the `ohsaa` provider contributed 0 rows `[state-assoc-greatlakes][synthesis/10-measured-census.md]` | State finals via OHSAA; Hy-Tek district/regional PDFs (36 historical + 9 current on one DAB); **Finish Timing** grade-bearing results and **29 unique AN MeetIDs from 1 homepage fetch** (FT id = `"20" + ANID`, 28/28 verified); Baum's Page enumerates 2003–2026 but its `Year` column is empty in every sampled file; Track Scoreboard is live-only; Ohio Runner = REJECT (road races) `[state-assoc-greatlakes][research/midwest/20-ohio-independent-sources.md]` | State-final PDF 816,147 B, 178 event×division combos: **561 grade-11 individual rows, 1,067 including relay legs**; page count 83 (analysis) vs 82 (artifact table) — both printed `[state-assoc-greatlakes]` | OHSAA **publishes literal AN URLs** (team 22671, meets 656920 / 656686, live 74520, custom list 50249); all 815 schools have AN team pages `[state-assoc-greatlakes][research/midwest/19-ohio-ohsaa.md]` | **OHSAA portal** for coach names+emails (rank 7) and **Finish Timing's homepage** for free AN MeetIDs; adapter implemented `[synthesis/03-adapter-ranking.md]` |
| **MO** | MSHSAA (mshsaa.org) `[state-assoc-plains]` | **1,006** `<tr data-organizationtype>` rows (316 senior-high + 360 junior-high + 330 combined 7–12 → **646 HS-level**; 719 member / 257 affiliate / 30 HSA; 592 full members HS-level; `MySchool/?s=<id>` space 47.4 % sparse) — measured, but robots reads **disallow-all** under RFC 9309 group merging (two independent parsers; refused 2026-09-22T03:57:12Z); one documented universe fetch was performed `[state-assoc-plains]` | **NOTHING public** — verified on two school pages: zero coach/AD fields, zero AD/coach links; 0 rows in `coach-contacts.csv` `[state-assoc-plains]` | State-meet PDFs only (Hy-Tek, literal `Year` column); districts/sectionals publish no results; PrimeTime `data.pttiming.com` (event ids 2874 / 9011 / 619619) is **conditional** (ToU bans automated access); TRXC upcoming page = **37 AN meet links in 1 request**; 2026 PDF corpus only 7 % HTML (28/352) vs 53 % in 2025 (244/452), some image-only `[state-assoc-plains][research/midwest/22-missouri-primetime.md][research/midwest/40-missouri-trxctiming.md]` | State finals only: C1 boys TF 92 athletes / 33 g11; C1 boys XC 175 / 39 `[state-assoc-plains][research/midwest/21-missouri-mshsaa.md]` | Path URL only (`/track-and-field-outdoor/usa/high-school/missouri/mshsaa`), no ids; PrimeTime `exEntryLink` sometimes carries an AN MeetID (619619) `[state-assoc-plains][athleticnet]` | **TRXC/PrimeTime (conditional)** for meet seeds; coach = **verified dead end** `[synthesis/02-state-playbook.md]` |
| **KS** | KSHSAA (kshsaa.org; kshsaa-api.kshsaa.org) `[state-assoc-plains]` | **348** schools in `PublicClassifications` 2026 (classId 6:36, 5:36, 4:36, 3:64, 2:64, 1:112) fresh; directory **526** records with AD name + email (A–Z letter queries concatenated) matching `[data/coach-contacts.csv]`; `/Shared/GetSchools` = 940 rows in a **different id space** (never sum the two) `[state-assoc-plains]` | Directory API publishes **AD name + AD email on every record** (526/526, validated wholesale at 100 % agreement); **no coach fields**; deliberately ignored: ADCell, PrincipalCell, SchoolPhone/Fax, principal name/email, Twitter `[state-assoc-plains][coach-directories-national]` | `kshsaachamps` `RetrieveResultsByActivity` (TF 2000–2026 = 20,887 rows; XC activityID 57/56 → 143 graded boys rows, 50 g11); `RetrieveRoster` is champion-only (10 athletes with `Grade`; non-champion schoolID returns `[]`); Midwest Timing Supabase 336 meets 2026 / 279 PDFs; DAT KS `[state-assoc-plains][research/midwest/23-kansas-kshsaa-directathletics.md]` | XC graded rows + champion-team rosters with grade; **TF grades are null** `[state-assoc-plains]` | **None observed** (0 AN matches across 11 artifacts); MileSplit state meet carries 2,331 athlete profile links + 325 teams `[state-assoc-plains][synthesis/02-state-playbook.md]` | **KSHSAA directory API** (rank 4): 1 request = 526 schools with AD email; adapter implemented `[synthesis/03-adapter-ranking.md]` |
| **NE** | NSAA (nsaahome.org; secure.nsaahome.org) `[state-assoc-plains]` | **312** schools in one POST to `direxportscreen.php` (314 `<option>` minus the prompt and the "View all" control); robots `Crawl-delay: 5`; fixture holds 8 school blocks; **yes**, re-derivable from the 11,585 B sample + fixture `[state-assoc-plains]` | Directory export publishes school facts + one staff row per role (Superintendent/Principal/AD/AD Secretary/Assistant AD) + one coach per sport: **1,528 rows / 1,217 names, validated at 100 % wholesale, zero email fields anywhere** `[state-assoc-plains][coach-directories-national]` | NSAA S3 static corpus (district XC HTML top-15 with grade in parentheses; state T&F `abres26.txt`); Precision Race Results on `onlineraceresults.com` gives the **full state XC field with 1m/2m splits** (race_id 79870, 33,461 B, 96 finishers); state T&F live on AthleticLIVE 72610 / 72612 `[state-assoc-plains][research/midwest/24-nebraska-nsaa.md]` | District XC top-15 with grade + state XC full field (timer) + state T&F Hy-Tek `Year` `[state-assoc-plains]` | **28 AN district MeetIDs 645702–645756**, all of the form `/TrackAndField/meet/<id>/results/all`; mapping table not implemented `[state-assoc-plains][research/midwest/24-nebraska-nsaa.md]` | **NSAA directory + S3** (rank 9): 1 POST = 312 schools (AD + coach names); adapter implemented via `plain_names` `[synthesis/03-adapter-ranking.md]` |
| **ND** | NDHSAA (ndhsaa.com) `[state-assoc-plains]` | **169** distinct `/schools/<id>/<slug>` links on the single-page index (fresh, 97,754 B, recounted); robots 24 B; **yes**, re-derivable from that sample `[state-assoc-plains]` | School pages carry a per-sport coach table (**names only — no email field exists**) + AD/Activities Director names; 22-school sample: boys XC 17/22, girls XC 16/22, boys TF 19/22, girls TF 19/22; 39 rows in `coach-contacts.csv`, 0 emails `[state-assoc-plains][coach-directories-national]` | AthleticLIVE Hero's tenant: ES `search.athletic.live/heros_meet_list/_search` (1,117 meets; 175 ND; 6 ND XC meets indexed for 2026-08-15..2026-11-30) → RTDB `liveRunStandings` (state XC 2025: AL meet 55421 / AN 261403, Class A boys 185 entries in 234,753 B); yearbook PDFs for the last completed season `[state-assoc-plains][research/midwest/25-north-dakota-ndhsaa.md]` | Firebase rows carry `y: SR/JR/SO/FR` (or numeric) — grade per athlete row `[state-assoc-plains]` | AN ids appear **only inside AthleticLIVE payloads**; 6 verified AL ↔ AN pairs (76088/277450, 76109/277021, 76112/276762, 76117/272168, 76139/278116, 76140/278115); co-op attribution needs the coop sheet `[state-assoc-plains][athleticnet]` | **NDHSAA school pages** (implemented: `plain_names`) + **AthleticLIVE RTDB** for graded results (rank 5/10) `[synthesis/03-adapter-ranking.md]` |
| **SD** | SDHSAA (sdhsaa.com) `[state-assoc-plains]` | **176** members (Bound association-directory parse — **report citation, not re-derivable**); AN SD tree holds 207 schools, identical name set for XC and TF; only **153 matched** by normalized prefix (86.9 %), 23 unmatched, 53 AN records unmatched (co-op composites); the **10 region links are re-derivable** from the fresh 286,293 B capture `[state-assoc-plains][research/midwest/26-south-dakota-sdhsaa.md]` | Association publishes nothing directly; coaches come from Bound. Both Bound paths are **refused** (gobound robots `Disallow: /*directory`; the association index itself answered 118 B / 403 to that lane): 20 names / 4 coach emails / 2 AD emails in `coach-contacts.csv` carry prior-study provenance `[state-assoc-plains][coach-directories-national]` | Yearbook PDFs, **last completed season only** (≈4.6 MB for 4 files); Bound competition pages are empty for state T&F (no Result column); state XC/T&F live on AthleticLIVE; 10 AN region links on the region page `[state-assoc-plains][research/midwest/26-south-dakota-sdhsaa.md]` | Yearbook `Year` column: boys XC 400 rows / 106 g11 (girls 383 / 74); TF 1,299 boys event rows / 392 g11 and 1,307 girls / 305 (upper bounds — athletes repeat per event/round) `[state-assoc-plains]` | **10 region meet ids** (277461, 278813, 278815–278817, 278821, 278826, 278829, 278838, 278840) + Bound calendar row `4931063 ↔ AN 278813` (name-independent join); division 87535 (XC) / 170271 (TF); 207 AN school records usable as `/team/<id>/...` `[state-assoc-plains]` | **sdhsaa.com region pages** for free AN ids + **yearbook PDFs** for grade; Bound for coach names at ≥10 s `[synthesis/02-state-playbook.md]` |

### 1b. The other 39 jurisdictions the national lanes covered (one line each)

All numbers are as measured by the named lane file; rows are grouped by the measuring lane, and `unknown`
is used wherever the lane recorded no value.

#### Mountain lane — 7 jurisdictions `[state-assoc-mountain]`

Three of the seven expose person-level data (NM: championship rosters with grade; ID and UT: AD/coach
emails); WY has zero anonymously reachable result data; UT's result host was down (503 twice).

| State | Association | Universe (count; re-derivable?) | Coach/AD | Results · grade | Athletic.net linkage | Lane verdict |
|---|---|---|---|---|---|---|
| **AZ** | AIA | 287 alignment operators (yes, sample) | name/role/tel, **no emails** | state XC as one 22.4 MB PDF/season; XC grade yes, TF no | entries delegated to AN; no AN ids captured | DISCOVERY `[state-assoc-mountain]` |
| **CO** | CHSAA | 378 (363 Member + 7 Preliminary + 8 Activity; yes) | partial names, no emails; `schools.chsaa.org` is auth-only | Hy-Tek TF PDF with grade; XC not measured | Rapid Results / TFMeetPro own the athlete layer | DISCOVERY `[state-assoc-mountain]` |
| **ID** | IDHSAA | 174 rows vs headline 173 (discrepancy printed) | **AD name/phone/email published**, base64-obfuscated via `document.write(atob())` | only via AN links, which answer a Cloudflare challenge to curl | AN state-meet pages referenced | PRIMARY (universe + AD) `[state-assoc-mountain]` |
| **MT** | MHSA | 184 incl. 2 out-of-state schools (enrollment PDF) | none — Arbiter SPA shell only (4,278 B) | Competitive Timing renders client-side | none | CONDITIONAL `[state-assoc-mountain]` |
| **NM** | NMAA | XC 126 / TF 156 (classification PDF) | none (`/for-coaches/` is a resource hub) | championship roster PDFs **with grade** (4A/5A boys 361 entries; hist 8:15, 9:41, 10:82, 11:110, 12:113) | liverunningresults.com live carries a non-commercial restriction | PRIMARY + RESULT `[state-assoc-mountain]` |
| **UT** | UHSAA | 162 (server-rendered anchors + alignment PDF) | **AD + per-sport coach + mailto emails** (25 mailto targets on the Alta sample) | results host `results.runnercard.com` returned **503 twice** | none | CONDITIONAL `[state-assoc-mountain]` |
| **WY** | WHSAA | 70, class counts only, names not published | none | MileSplit Live meet 681838; **0 AN matches in captured pages** | 0 | zero anonymously reachable result data `[state-assoc-mountain]` |

#### Midatlantic lane — 12 jurisdictions `[state-assoc-midatlantic]`

The largest lane block; DE, NY and PA remain unenumerable (Cloudflare, robots-disallowed `/documents/`,
client-rendered rows), and MA is the only one in this block with a formal AN entry partnership.

| State | Association | Universe (count; re-derivable?) | Coach/AD | Results · grade | Athletic.net linkage | Lane verdict |
|---|---|---|---|---|---|---|
| **DC** | DCSAA | 45 sanctioned schools | none | competition-level only | none | DISCOVERY `[state-assoc-midatlantic]` |
| **DE** | DIAA (expansion inferred) | count `unknown` — Cloudflare interstitial on every host path | none | MileSplit DE 82 team links | none | CONDITIONAL `[state-assoc-midatlantic]` |
| **MD** | MPSSAA | 201 directory pages (9 aggregates → 192 school pages) | AD directory login-gated (302 → /login) | athlete-level state championship pages | none | RESULT_SOURCE `[state-assoc-midatlantic]` |
| **NJ** | NJSIAA | 47 school rows on page 1 (pagination not exposed; total not asserted) | **AD name + school + address + phone** in the member table (no email) | none on the association host; MileSplit entry windows published | MileSplit only | DISCOVERY `[state-assoc-midatlantic]` |
| **NY** | NYSPHSAA + PSAL | count `unknown` — not enumerable: `/schools` 404 and `/documents/` robots-disallowed | none public | none located; PSAL publishes XC results as Google Sheets | MileSplit ad handler only; MaxPreps links | VALIDATION `[state-assoc-midatlantic]` |
| **PA** | PIAA | count `unknown` (220 sitemap nodes / 25 directory letters counted; **rows render client-side**) | `/officials/directory/` robots-disallowed | none in static HTML; PIAA points at pa.milesplit.com (PenntrackXC) | MileSplit coverage link | VALIDATION `[state-assoc-midatlantic]` |
| **CT** | CIAC | 440 `SchoolID` values (mixed levels) | **AD name + coach name, no emails** on the delegated directory `ciac.fpsports.org` (256,697 visible chars; `ad_title` 189, `phone` 3,628, `coach` 3,824 token hits, `mailto` 0); the association host itself publishes none | results host `ciacsports.com` has an **expired TLS certificate** (verification not disabled) | none | CONDITIONAL — two lanes, two surfaces: `[state-assoc-midatlantic]` found no public contact on the association host, `[coach-directories-national]` found names on fpsports |
| **MA** | MIAA | 381 member schools from the 2026-03-27 PDF (260 H / 97 MH / 24 EMH) | member PDF has no names | entries route through AN (MIAA states a formal AN entry partnership for XC/indoor/outdoor) | **AN partnership**; MSTCA rankings built from AN data | DISCOVERY `[state-assoc-midatlantic]` |
| **RI** | RIIL | 128 `SchoolID` values | **AD name + coach name, no emails** on `riil.org/Directory.aspx` (64,883 visible chars; `ad_title` 57, `coach` 1,116, `phone` 243, `mailto` 0) `[coach-directories-national]` | publishes MileSplit as its XC results host | ri.milesplit.com partner link | DISCOVERY `[state-assoc-midatlantic]` |
| **NH** | NHIAA | count `unknown` (42 directory pages; entries client-rendered → **not measured**) | rSchoolToday data layer not captured | no result page captured | none | DISCOVERY `[state-assoc-midatlantic]` |
| **VT** | VPA | count `unknown` (17 athletics pages; membership not enumerated) | none | scoreboards/tournament docs are Google Docs | none | VALIDATION `[state-assoc-midatlantic]` |
| **ME** | MPA | 152 `SchoolID` values | **AD name + coach name, no emails** on `mpa.cc` (100,560 visible chars; `ad_title` 149, `coach` 1,630, `phone` 582, `mailto` 0) `[coach-directories-national]` | entries posted to MileSplit and Sub5.com | me.milesplit.com partner link | DISCOVERY `[state-assoc-midatlantic]` |

#### Southeast lane — 6 jurisdictions `[state-assoc-southeast]`

FL and NC are the two grade-bearing association surfaces in this block; GA has the biggest coach-name
corpus (23,435 staff rows) but no grade; VA's association is robots disallow-all.

| State | Association | Universe (count; re-derivable?) | Coach/AD | Results · grade | Athletic.net linkage | Lane verdict |
|---|---|---|---|---|---|---|
| **FL** | FHSAA | 880 homecampus widget rows (association claims "over 850"; mixes levels) | `get-school-details` JSON exposes `athleticFaculties` + `coaches` **with emails** (requires `X-Requested-With`) | TFRRS FL lists carry `Year` = FR/SO/JR/SR (4A Region 1: 1,469 rows; JR 487, SR 529) + Half Mile / PrimeTime live | **0 AN references across 87 surfaces** | PRIMARY + RESULT_SOURCE `[state-assoc-southeast]` |
| **GA** | GHSA | 457 dropdown / 456 PDF members | **23,435 staff rows** (names + sport codes `5`/`14`; AD 672, P 443, AP 1721); **no per-coach emails** | 320 state XC PDFs 2000-01..2024-25; the sampled TF PDF's `Year` column is **empty** | 0 AN; MileSplit nav links | PRIMARY + RESULT (no grade) `[state-assoc-southeast]` |
| **NC** | NCHSAA | 451 schools + **77 conference administrators with emails** | no coach directory; the coach lane grades AD contact **partial** (`ad_name`/`ad_email` partial, `coach_name` no) | 16 XC result PDFs for 2025; **grade-bearing** (5A girls 146 rows: 9:29, 10:42, 11:44, 12:31 → 44 Co2027) | 0 AN; footer links nc.milesplit.com | PRIMARY + RESULT_SOURCE `[state-assoc-southeast]` |
| **SC** | SCHSL | 229 classified schools (5A 35/4A 45/3A 35/2A 44/1A 70) | none | nothing on host; championship post links a MileSplit results hub | 0 AN | PRIMARY (universe only) `[state-assoc-southeast]` |
| **VA** | VHSL | count `unknown` — **robots disallow-all**, 0 requests issued | none | MileSplit/MileStat is the operational host (meet 742518, Tidewater Timing) | 0 AN | CONDITIONAL `[state-assoc-southeast]` |
| **WV** | WVSSAC | 119 classified (AAAA 20/AAA 28/AA 32/A 39, enrollment 7–1753) | directory page is an Arbiter iframe shell (not driven) | no association results (WP REST search returns `[]`); runwv.com used by the pipeline | 0 AN | PRIMARY universe; REJECT results/grades/coaches `[state-assoc-southeast]` |

#### Westcoast lane — 6 jurisdictions `[state-assoc-westcoast]`

OR is the only association in the whole corpus publishing coach names **and** emails per activity; CA has
the strongest bulk universe (one xlsx, per-sport participation); WA and NV have no timing provider observed.

| State | Association | Universe (count; re-derivable?) | Coach/AD | Results · grade | Athletic.net linkage | Lane verdict |
|---|---|---|---|---|---|---|
| **CA** | CIF (state + 10 sections) | **1,608** schools from the 2025-26 census xlsx in 1 request; per-sport participation (XC 1,400 schools, TF 1,323) | none located at state or section level | finals PDFs: 66 XC archive links (2004-2025), 36 TF (2005-2026); sanctioned timer not published (Arbiter for officials) | no AN, no MileSplit linkage | PRIMARY `[state-assoc-westcoast]` |
| **OR** | OSAA | 299 `schools/{id}` anchors (participation workbook 301 rows — 2 more) | **coach names + emails + phones** per activity (the only association lane with both) + AD/assistant AD/activities director | Athletic Timing live (meet 58997); participation xlsx back to 2008-09 | **AN URLs published** (state + `team/22622`) | PRIMARY `[state-assoc-westcoast]` |
| **WA** | WIAA (Washington) | 752 from 51 pages of 15 (pagination arithmetic) | none (Arbiter/FinalForms human tooling only) | **no timing provider observed** | none | PRIMARY universe only `[state-assoc-westcoast]` |
| **AK** | ASAA | 217 (218 `<tr>` minus header); WordPress REST page id 19 | none (school phone only) | 12 XC 2025 PDFs + RaceResult live | publishes `athletic.net/team/22622/...` | PRIMARY `[state-assoc-westcoast]` |
| **HI** | HHSAA | 95 = BIIF 23 + ILH 20 + KIF 8 + MIL 13 + OIA 31 | none | MileSplit raw results + AN Live | `live.athletic.net/meets/58854` + hi.milesplit meet 730986 | PRIMARY `[state-assoc-westcoast]` |
| **NV** | NIAA | 127 (FinalForms, 9 pages) | PrestoSports `/coaches/Landing` → 202 + 0-byte body | `/sports/track` → 202 + 0-byte body | none | CONDITIONAL (unverified) `[state-assoc-westcoast]` |

#### Southcentral lane — 8 jurisdictions `[state-assoc-southcentral]`

The most heterogeneous block: three associations publish grade-bearing results (OK, TN, LA), three are
blocked or delegated (AL, MS, TX), and AR/KY hand over only MileSplit or ArbiterLive pointers.

| State | Association | Universe (count; re-derivable?) | Coach/AD | Results · grade | Athletic.net linkage | Lane verdict |
|---|---|---|---|---|---|---|
| **AL** | AHSAA | count `unknown` — **robots `Disallow: /`** → blocked (MileSplit proxy: 562 teams) | none reachable | Xpress Timing delegates live results to `xt.anet.live` | AN Live | CONDITIONAL `[state-assoc-southcentral]` |
| **AR** | AAA | **502** member schools (self-published counter) | AD directory = Google Form; roster platform DragonFly (login) | association delegates meets/entries/results to MileSplit (letter of partnership) | MileSplit only | DISCOVERY `[state-assoc-southcentral]` |
| **KY** | KHSAA | **290** members with stable ArbiterLive `entityId` links | ArbiterLive per-school pages (JS-rendered; JSON endpoint **not identified**) | results delegated to MileSplit | none | VALIDATION `[state-assoc-southcentral]` |
| **LA** | LHSAA | 306 derived from the state-meet PDF; membership count **unverified** (fonts are subset-glyph permuted, 108 pages) | **coaches/AD directory PDF** (school, class, phone, AD, principal, per-sport coach names + emails) — text layer font-encoded | state-meet Hy-Tek PDFs 2003–2026 with `Year` + wind | none | RESULT_SOURCE `[state-assoc-southcentral]` |
| **MS** | MHSAA (Mississippi) | count `unknown` — **403 WAF** on apex and www (no bypass) | none | unknown | none | CONDITIONAL `[state-assoc-southcentral]` |
| **OK** | OSSAA | **482** members from the Member Schools PDF | `/athletic-directors/` is handbooks only; member login gated | per-class state results with **grade + splits** (MeetPro); explicit MileSplit partnership | MileSplit partnership | PRIMARY `[state-assoc-southcentral]` |
| **TN** | TSSAA | **456** member schools as an **inline JS array** `{id,name}` on `/common/directory/` | coaches hub only (no directory) | 151 championship PDFs with grade + top-8 depth | none captured | PRIMARY `[state-assoc-southcentral]` |
| **TX** | UIL | 1,566 schools in the 2026-28 realignment (1A 225 …) + 1,366 unique 4-letter school codes | UIL Portal login only; staff contacts only | association delegates stats to MaxPreps, not MileSplit | none | CONDITIONAL — every source PDF sits under robots-disallowed `/files/`; owner decision needed `[state-assoc-southcentral]` |

### 1c. What the pipeline actually measured per target state — 2026-09-20 snapshot

Source: `[synthesis/10-measured-census.md]` (built from `var/census-service/out/report.json` + `data/*.csv`),
cross-checked against `[reports/census-by-state.csv]`. This is the **measured** column set; the research
estimates live in `[synthesis/01-acceptance-answers.md]` and are order-of-magnitude bands.

| State | schools | athletes (all grades) | Co2027 | Co2027 multi-source | meet rows in store | coach rows (by-state col.) | coach email (by-state col.) |
|---|---:|---:|---:|---:|---:|---:|---:|
| WI | 1,027 | 73,202 | 18,136 | 5,185 | 777 | 12,766 | 12,766 |
| MN | 860 | 64,275 | 14,420 | 5,315 | 987 | 2,473 | 2,305 |
| IA | 913 | 62,201 | 14,076 | 2,820 | 1,003 | 105 | 21 |
| IL | 1,392 | 92,973 | 26,488 | 10,740 | 1,994 | 125 | 125 |
| MI | 1,493 | 114,289 | 22,073 | 9,148 | 1,429 | 0 | 0 |
| IN | 1,108 | 54,475 | 16,931 | 3,188 | 381 | 0 | 0 |
| OH | 1,504 | 106,136 | 31,708 | 9,891 | 1,232 | 456 | 456 |
| MO | 1,037 | 50,830 | 15,168 | 5,552 | 569 | 0 | 0 |
| KS | 836 | 33,628 | 10,077 | 1,937 | 181 | 0 | 0 |
| NE | 677 | 44,983 | 9,200 | 3,066 | 589 | 6,059 | 0 |
| ND | 195 | 3,511 | 1,946 | 0 | 161 | 0 | 0 |
| SD | 365 | 7,430 | 3,652 | 0 | 364 | 209 | 46 |

Totals for the 12: 11,407 schools, 707,933 athletes, 183,875 Co2027, 56,842 Co2027 with two independent
sources, 9,667 meets of which **9,593 carry an Athletic.net meet id**, 6,235 coaches/ADs of which 3,716
carry a published professional email `[synthesis/10-measured-census.md]`.

- **Do not compare the `schools` column to §1a.** The census mints canonical schools from *every* source
  (timer rosters, MileSplit team indexes, association lists), so it counts middle/junior-high programs and
  co-op composites too: IL 1,392 here vs **828** IHSA members in §1a, OH 1,504 vs **815**, MI 1,493 vs
  **755**. Both are correct at their own scope `[INFERENCE]` for the compositional reason, measured numbers
  `[synthesis/10-measured-census.md][state-assoc-greatlakes]`.
- **Coach columns disagree inside the same document.** The by-state table above gives WI 12,766 coach rows
  with email, while the doc's "Coach coverage" table gives WI 2,550 rows / 2,550 with email and MN 1,490 /
  584 (vs 2,473 / 2,305 above); `data/canonical-coaches.csv` holds **6,235** rows, matching the doc total
  `[data/canonical-coaches.csv, 6,236 lines incl. header]`. The two tables measure different joins; both are
  printed and neither is averaged (§5).
- Athletic.net identities minted **without querying Athletic.net**: 169,230 distinct athlete ids published by
  timer rows / MileSplit embeds, and 9,566 distinct meet ids from the timer meet index
  `[synthesis/10-measured-census.md]`; the same seed files hold 189,704 athlete rows and 9,844 meet rows
  `[data/athleticnet-athlete-seeds.csv][data/athleticnet-meet-seeds.csv]`.
- Identity coverage inside `data/canonical-athletes-co2027.csv` (183,875 rows): 93,619 carry an AN athlete id
  (50.9 %), 137,823 carry a MileSplit id, **56,842 carry both** `[synthesis/10-measured-census.md]`.
- 99 providers published the meet universe; the largest ten are `live_results` 9,705, `athleticlive` 2,432,
  `wayzata` 519, `michiana` 440, `dakota` 421, `iptt` 403, `palatine` 349, `blacksquirrel` 345, `heros` 329,
  `aatiming` 311 `[synthesis/10-measured-census.md]`.

---

## 2. Source-family matrix

One row per family. "AN" = Athletic.net.

| Family | Enumerates | Stable ids | Athlete fields | Result fields | Coach fields | Access class | robots + rate limits observed | Join keys to AN |
|---|---|---|---|---|---|---|---|---|
| **State associations** (51 jurisdictions, all 7 association lanes) | Member schools/classifications in 43 of 51; per-sport participation in AZ, CO, CA, OR, UT, NE, KS, SD, IA; athlete rosters in NM, NC, GA(no grade), LA, TN, OK; 22 tier-1 directories collected `[coach-directories-national]` | Per-state: WI `orgID`, IL 4-digit `SchoolID` + `athleticNetId`, OH `OhsaaSchoolId` (100–1000027), MI numeric MHSAA id, KS `Id`, ND `<id>`, NE/IA slugs, KY ArbiterLive `entityId`, TN id, WV/SC class lists `[state-assoc-greatlakes][state-assoc-plains][state-assoc-southcentral]` | Almost never: association lanes carry **no athlete ids**; they carry name + grade (`Yr`/`Year`) or roster PDFs; TN/NC/GA publish names only | State-series only; formats Hy-Tek PDF/HTML, plain text, xlsx, Google Docs, live SPA; regular season is delegated to timers/MileSplit/AN in most states | Name, role, sport, school, sometimes email; **coach email in 3 of 22 tier-1 jurisdictions (IL, OH, WI)**; AD email in 5 by the counts block vs 4 named in the fields blocks (IL, KS, OH, WI) `[coach-directories-national]` | HTML / static CSV-XLSX / PDF / browser app / **blocked** (AHSAA, VHSL, MS, DE, WY, MSHSAA-disallow-all, UIL `/files/`, NYSPHSAA `/documents/`, CT TLS) `[state-assoc-southcentral][state-assoc-midatlantic][state-assoc-mountain][state-assoc-plains]` | robots varies per host; refusals logged verbatim (gobound `/*directory`, my.mhsaa `/DesktopModules/`, bd MSHSAA disallow-all); MSHSL publishes a 2,385 B file that declares `/jsonapi/`, `/api/coaches/<nid>`, `/api/team-data/**`; RIIL/MPA/CIAC share an identical 247 B file; observed rate: anonymous, ≤1 rps in every lane, MSHSL bulk-PDF crawl disallowed, NSAA `Crawl-delay: 5`, VPA 10 s, GHSA 10 s, Bound 10 s; no 429/`Retry-After` observed anywhere `[state-assoc-plains][coach-directories-national][synthesis/05-compliance-and-risks.md]` | Association-published AN ids/URLs (IL, OH, MI, SD, NE, MN, WI); AN TeamID must be resolved once per school and cached where the lane hands over names only `[athleticnet]` |
| **DirectAthletics** (directathletics.com, tfmeetpro.com) | State league → team list (`/leagues/{sport}/{league_id}.html`); results index by state+sport **100 meets/page**; **global upcoming index = 946 meets across 49 states in one JS file**; performance-list directory = 274 list ids; MeetPro = DA's meet-management publisher `[national-aggregators][timing-providers-national]` | Team id, meet id, list id, athlete profile URL `/athletes/{sport}/{id}.html`; **no athlete ids in any capture**; `data/dat-team-index.csv` = 22,527 rows (OH 3,428 / IL 3,340 / MI 3,230 / MO 1,919 / WI 1,912 / IN 1,816 / KS 1,669 / IA 1,592 / NE 1,266 / MN 874 / SD 758 / ND 723) `[data/dat-team-index.csv]` | Name + school; grade only as `Year` (SR/JR/SO/FR) on HS event sheets and via `year=` on lists | Mark/place/meet; TFRRS profile links per row; live HS performance lists exist **only in Indiana** `[state-assoc-greatlakes]` | Meet-director contact only (dropped for privacy) `[synthesis/05-compliance-and-risks.md §2]` | HTML + undocumented JSON; no documented API; no published robots policy (`/robots.txt` serves brand/404 HTML) `[national-aggregators]` | No rate limit observed; the earlier Midwest passes ran 34.9/35.3/28.8/29.6 MB POSTs (4 site-search calls) `[state-assoc-greatlakes]` | **No AN crosswalk**: in `data/school-alias-map.csv`, **5,931 of 5,933** rows record "not exposed on DAT (no athletic.net links found on any sampled page)" and only **2 rows** carry an AN team id (from league page `track/114.html`) `[data/school-alias-map.csv, column `athletic_net_team_id_source`]` |
| **MileSplit / FloSports** | Per-state hosts `<code>.milesplit.com` (51/51 HTTP 200) with `/teams` (HS filter), `/teams/<id>/roster`, `/results` (50 meets/page), `/calendar/<state>-<season>-meet-calendar` (OH 294 meet rows), `/athletes/<id>`, `/meets/<id>/results/<RSID>/raw`, and `POST /search/v2/athletes` (100 hits/page, 100-page cap, needs a fresh `searchToken` + `unique_id` cookie) `[milesplit-national]` | TeamID, MeetID, AthleteID, RSID — network-global athlete ids; state-canonical host redirects `[milesplit-national]` | Name, school, grad year as an **absolute** value (`column-grad-year` on rosters, verified against profile bio; `Class of YYYY` on profiles), gender, season flags `[state-assoc-greatlakes][synthesis/02-state-playbook.md]` | Raw Hy-Tek text (`/raw` capture: 80 rows, `Yr` values 7,8); meet results HTML; **the JSON model with `gradYear` + performances sits under robots-disallowed `/api/`** `[milesplit-national][synthesis/05-compliance-and-risks.md]` | None — no coach directory; `/contact` is disallowed `[milesplit-national]` | Static HTML / browser app / **subscription** for locked result rows (WI profile: 44 `aria-label="Locked"` placeholders; OH profile 480 mask spans; OH event rankings 49 of 50 rows masked) `[milesplit-national][synthesis/08-gap-closure-2026-09-20.md §4]` | `Disallow: /rankings, /virtual-meets, /api/, /contact` on all sampled hosts; `sitemap.xml` = **7,500 athlete URLs, all `lastmod` inside a ~2 h window** (rolling recency feed, not a census); no 429 observed; 163 roster pages were refused (403) one day and recovered on a second pass — no refusal bypassed `[milesplit-national][synthesis/10-measured-census.md]` | **None**: 0 `athletic.net` matches across 28 WI captures and across the 80 captures of report 31; MileSplit hands over a tuple (name, school, state, grad year) that needs an AN-side lookup `[athleticnet][research/midwest/07-wisconsin-milesplit.md][research/midwest/31-milesplit-entries-timing.md]` |
| **AthleticLIVE tenants** (`<tenant>.anet.live`, `live.athletic.net`, `search.athletic.live`, Azure blob, Firebase RTDB) | 256 tenant indices in 1 aggregation request (296 ES buckets); per-tenant meet indices via `_search`/`_msearch`; per-state meet counts via `lsa.keyword` (70 buckets) or `match` on `lsa`; per-event result docs via blob `$web/ind_res_list/_doc/<id>`; per-meet event summaries via RTDB `meet_<id>/event_summary.json?ns=trackmeet-io`; tenant config via `livestatic.athletic.net/assets/sites/<tenant>/config.json`; tenant inventory 153,524 meet docs, 129,203 (84.2 %) with an AN id `[national-aggregators][state-assoc-plains]` | `i` (AL meet id), `ani` (AN MeetID), `aci` (AN calendar id), `atr` (timer credit), event/document ids, `t.ani` (AN team id), athlete doc `ani` (AN athlete id) `[synthesis/08-gap-closure-2026-09-20.md §3][athleticnet]` | Name, grade (`y` as SR/JR/SO/FR or numeric), team, **real AN athlete id** — e.g. `r[0].a = {i 44251790, n 'Edward Mugisha', y '12', ani 15728090}` | Whole-event blob docs (136 rows / 159 KB in 1 request); state XC fields 185–959 rows; relay legs are enumerated; blob supports `ETag`/`Last-Modified` + 304 revalidation `[national-aggregators][synthesis/08-gap-closure-2026-09-20.md §3]` | None | Public JSON (anonymous) with **no documented API**; the tenant SPA does not use `api.athletic.live`; blob container listing is disabled (404) `[national-aggregators]` | HTML paths `/meets/*/athletes`, `/meets/*/teams`, `/meets/*/live`, `/meets/*/follow` are robots-disallowed; `live.athletic.net/sitemap.xml` has 17 route-level locs only; per-provider aggregation on `tna` fails on 250/256 shards (text field, no keyword subfield); PrimeTime/Karmarush live ToU bars automated access — treat `livestatic.athletic.net`/`edge.athletic.net` as AN for compliance; ~59 requests to `search.athletic.live` disclosed, no adverse response `[national-aggregators][synthesis/05-compliance-and-risks.md]` | **The primary AN identity channel**: `ani` on meet, athlete and team docs (959/959 and 136/136 sampled athlete rows carry it); 4,586 documents with a non-null `ani` across the captured ES extracts `[athleticnet]` |
| **TFRRS** (tfrrs.org, DirectAthletics) | `/sites.html` association list; state+sport+year meet search; performance lists per association/season with grade + page-size filters; athlete pages by numeric id. **High-school instances exist for exactly 3 states: FL, IN, NH** (`texas/ohio/california.tfrrs.org` = NXDOMAIN) `[national-aggregators]` | Numeric athlete id, list id, team id (IN: 929 TF + 887 XC teams, 1,816 team records; list 5328 = 508 meet ids; list 5489 = 1,671 athlete ids) `[state-assoc-greatlakes][national-aggregators]` | Name, school, grade via `Year`/`(SR)` suffix; IN lists carry FR/SO/JR/SR class labels | Marks/places; list pages carry **0 team anchors and 0 meet anchors** (names are plain text) `[state-assoc-southeast]` | None | Static HTML; **no JSON/API endpoints observed**; robots is "comment-only" on the FL instance; no sitemap published `[national-aggregators]` | No published rate limit; 2 requests yield the IN Co2027 cohort (`&year=JR`) `[synthesis/02-state-playbook.md]` | **None**: no AN or MileSplit cross-links; the join is name+school+grade; `&year=SR` returns Co2026 (1,678) not Co2027 `[national-aggregators][research/midwest/18-indiana-directathletics-milesplit.md]` |
| **Timing companies** (AthleticLIVE tenants + independent publishers) | Per-meet result pages + season indexes: PrimeTime (WI 245 / MO 24 / IL 12 / FL 11 / IA 8 / IN 8 / NE 6 … meets for 2026), FlashResults (14 current schedule rows, 2019 index = 60 meet links), Finish Timing (`/2026/` = 325 files; `/2026/CC/` = 32 date-coded meet dirs), Hero's Timing (256 meets on the 2026 page, 15 season pages), TRXC (MO, upcoming table), GSE (MN XC), RACE RESULT (MI XC event 423470), Wayzata (MN core + IA/WI/ND/IL) `[timing-providers-national]` | Provider-native meet id; **Finish Timing id = `"20" + ANID`**; Track Scoreboard `meets/20<ANID>`; AthleticLIVE blob doc ids; FlashResults has **no cumulative meet index** `[research/midwest/20-ohio-independent-sources.md][timing-providers-national]` | Name, school, grade (Finish Timing numeric grade 9–12; FlashResults `athleteList.htm` brackets class/grade; Heritage/TFMeetPro vary) `[research/midwest/20-ohio-independent-sources.md]` | Place, mark, round/heat (`Prelims`/`Finals`, `H#`), wind where recorded; whole-meet cost ≈3 AN XHR or 1 blob doc per event vs 0.33 requests/1,000 rows on a grade-roster path `[timing-providers-national]` | None (meet-director contacts only) | Static HTML/PDF/plain-text archives, browser apps (SvelteKit, Angular-over-Firebase), some authenticated upload tools; **REJECT** classes: OpenTrack, xcstats, Baum's Page (Year empty), Ohio Runner (road only) `[timing-providers-national][state-assoc-greatlakes]` | robots: FlashResults disallows `/*.pdf$` `/*.csv$` `/*.js$` + `/flashwest/` `/flashtexas*`; RACE RESULT disallows result payloads (metadata OK); Wayzata CMS is UA-gated; PrimeTime live is ToU-blocked; OpenTrack blocks automation; RunnerSpace HTML 403 to curl with `Crawl-delay: 10`; no 429 observed on any timer host `[timing-providers-national][synthesis/05-compliance-and-risks.md]` | **Rich**: `ani` via the tenant plane; Finish Timing publishes AN MeetIDs next to its own ids (29 from 1 request, 28/28 satisfy `20+ANID`); TRXC upcoming page = 37 AN meet links; FlashResults rows carry TFRRS athlete links, **no AN ids** `[research/midwest/20-ohio-independent-sources.md][research/midwest/40-missouri-trxctiming.md]` |
| **National aggregators** | MaxPreps state/sport sections + athlete directories (200/page); RunnerSpace/DyeStat event microsites (NXN 22 result sets 2004–2025; ResultsCentral 153 news items in 2026); USATF sitemap (3,890 URLs, 101 junior-olympic); crosscountryratings (`/individuals`, `/results`, `/runner/<id>`, `/school/<id>`, XC only, 9 states); AAU (unavailable); World Athletics GraphQL `getAthletes/searchAthletes/getAthlete(urlSlug)` `[national-aggregators]` | ccr runner/school/event ids; USATF event URLs; MaxPreps **no** athlete ids and no grade string; World Athletics `getAthlete(id:)` returns a placeholder record regardless of id — **only the urlSlug works** `[national-aggregators]` | ccr `Class of 20XX` + grade word; RunnerSpace grade codes SR/JR/SO/FR in championship tables; World Athletics birthDate only (no HS grade concept) | No meet results on MaxPreps; RunnerSpace results are national-championship fields only; USATF hosts nothing itself | None | HTML (some browser-only); **REJECT**: MaxPreps, AAU (403 Cloudflare, no bypass attempted), NCSA, FieldLevel, World Athletics (validation only) `[national-aggregators]` | MaxPreps robots disallows `/school/`, `/team/`, `/discovery/`, `/careerprofile/`, `/m/team/`, `/m/school/`; ccr disallows `/api/`, `/admin/`; xcstats disallows AI crawlers and is paywalled; RunnerSpace 403 to curl + `Crawl-delay: 10` (browser render succeeds) `[national-aggregators]` | **One verified crosswalk**: USATF Junior Olympic T&F — Athletic.net MeetID **644030** + TrackScoreboard meet **14258**. No other aggregator capture in the corpus ties a page to an AN id `[national-aggregators]` |
| **Athletic.net** (the target namespace) | `GetNavInfo` returns all 51 jurisdictions of the High-School level (`168416`) in one session-bound call (78 KB; 53 state nodes, 52 in regions, 51 mapped, "Overseas" out of scope); `GetDivChildren` gives whole-country states in 1 request (WI full tree = 21); `GetRankings` per `divListId` with a **server-side `grades:[11]` filter** and `qParams.page` pagination; `GetAthleteBioData` self-seeds MeetIDs; `TeamNav/Team` returns State + grades + divisions `[athleticnet][synthesis/atn-endpoint-groundtruth.md]` | `AthleteID`, `TeamID` (= `IDSchool`, global, one id per school, `TeamID 0` = unattached), `MeetID`, `IDResult`, `EventID`/`shortCode`, `divListId`, meet `jwtMeet`; URL grammar: `/athlete/<id>/<sport>`, `/team/<id>/<sport-season>`, `/{TrackAndField,CrossCountry}/meet/<id>[/results/all]`, `/rankings/list/<divListId>/<m,f>` `[athleticnet]` | Name, `Grade`/`AgeGrade` per row, gender, `SchoolID` per performance, `allTeams`, `UsatfId`, handle | `Result`, `SortInt`, `FAT`, `Wind`/`Heat`/`HeatPlace` (per-event call only), `Round`, `Place`, `PersonalBest`, `MeetID`, `SeasonID`, implement/hurdle spec in `eventTypes[]`, relay legs resolved in `relayLegs[]` `[synthesis/08-gap-closure-2026-09-20.md §1]` | None | Undocumented public JSON: anonymous 200 with browser-grade headers (meet/bio/rankings/public endpoints); `GetNavInfo` is session-bound (403 to a plain client); `DownloadRankings` is role-entitled and 403 anonymous; entitlement blur: `GetRankings` mints `jwtTFTopReport` and with `userRoles` empty sets `blurAfterDepth=5` — measured 96 blurred rows, only ranks 1–5 unblurred `[athleticnet][synthesis/08-gap-closure-2026-09-20.md §1]` | `robots.txt` 200, 1,080 B, no sitemap, **nothing under `/api/` disallowed**; default/short UAs get a Cloudflare challenge; no `ETag`/`Last-Modified` (`no-store`) → use content-digest deltas; every lane kept ≥1.2 s spacing `[athleticnet][synthesis/05-compliance-and-risks.md]` | n/a (it is the namespace every other row joins into); never persist `jwtMeet`, `anettokens` or cookie values `[synthesis/08-gap-closure-2026-09-20.md §1]` |

### 2b. Family notes — what each family is actually for, and where it stops

**State associations.** The only source that yields the *membership denominator* (43 of 51 jurisdictions
enumerable) and, in 22 cases, a tier-1 directory. Cost: 1-4 requests for a universe (NE 1 POST,
CA 1 xlsx, TN 1 page, ND 1 page, KS 1 for 526 directory records), 3 requests/school for one of the two rich
directories (OH myOHSAA),
9 requests/school for the other (WI WIAA `GetDirectorySchool`). Posture (measured outcomes over all 51, `[coach-directories-national]`): 3 jurisdictions are
robots-disallowed and were not collected (AL, MO, VA), 26 tier-1 directories are client-rendered and
unverified, 12 were fetched **with partial contact fields**, 10 were fetched with **no** contact fields, and
4 hosts answered an HTTP block to this client. They never hand over athlete ids. Verdict: PRIMARY for the universe; RESULT_SOURCE
only where they publish grade (`Yr`/`Year` in Hy-Tek output) `[coach-directories-national][state-assoc-*]`.

**DirectAthletics / MeetPro.** Discovery of meets, team indexes and (via MeetPro) championship result pages.
`/leagues/{sport}/{league_id}.html` enumerates teams; the global upcoming index is 946 meets in one JS file;
the performance-list directory is 274 list ids; `data/dat-team-index.csv` holds 22,527 team rows across the
12 states (OH 3,428 / IL 3,340 / MI 3,230 / MO 1,919 / WI 1,912 / IN 1,816 / KS 1,669 / IA 1,592 / NE 1,266 /
MN 874 / SD 758 / ND 723). It exposes **no AN ids**: all 5,933 rows of `data/school-alias-map.csv` carry `athletic_net_team_id_source` =
"not exposed on DAT (no athletic.net links found on any sampled page)" and **0** rows carry a non-empty
`athletic_net_team_id` (re-counted with the `csv` module on 2026-09-21) —
so any DAT→AN link is a name tuple. Verdict: DISCOVERY_SOURCE `[national-aggregators][data/dat-team-index.csv]`.

**MileSplit / FloSports.** The only surface observed publishing an **absolute** class value on a
robots-allowed page (`column-grad-year` on `/teams/<id>/roster`, verified against profile bios), plus the
51-host team index (26,562 HS teams) and a 7,500-URL rolling sitemap. It publishes **no coach field at all**
(0 occurrences of "coach" in the captured OH roster sample), and its `/api/**` — the only JSON carrying
`gradYear` + `windReading` + `AthleteID` — is robots-disallowed on oh, www, ca, tx, mi, wy and va. Rankings
below row 1 are server-side masked for anonymous clients. Verdict: the primary Co2027 discovery +
validation surface `[milesplit-national]`.

**AthleticLIVE tenants.** The fast, allowed read path around Athletic.net's own front door: 256 tenants,
153,524 meet docs, 129,203 (84.2 %) carrying an AN meet id, per-tenant ES anonymous, Azure blob result rows
carrying grade **and** the real AN athlete id, RTDB live standings keyed by meet. Two structural limits: it
is **not independent** corroboration (it is AN's white-label platform), and per-provider counts cannot be
aggregated across the shared cluster — a `terms` agg on `tna` fails on 250/256 shards; `lsa.keyword` works
and returns 70 state buckets. PrimeTime/Karmarush *live* (WI, Franklin Firebase) is off-limits without a
licence. Verdict: PRIMARY `[national-aggregators][timing-providers-national]`.

**TFRRS.** High-school instances exist for exactly three states (FL, IN, NH; `texas/ohio/california.tfrrs.org`
are NXDOMAIN). Where it exists it is the cleanest grade oracle: a `Year` column readable per row and
filterable via `&year`, no login; the IN HSR list is 1,671 athlete ids, list 5328 is 508 meet ids. Its limit
is structural: list pages carry **0 team anchors and 0 meet anchors**, and the national corpus carries no AN
ids. Verdict: RESULT_SOURCE for FL/IN/NH, VALIDATION_SOURCE nationally `[national-aggregators][state-assoc-greatlakes]`.

**Timing companies.** 16 provider families; near-real-time results, and the only family that publishes ids
that *arithmetically* imply AN meet ids (Finish Timing `"20" + ANID`, 28/28 verified; Track Scoreboard
`meets/20<ANID>`). Sizes: PrimeTime 353 events / 532 result files (489 PDF, 38 htm, 5 html) for 2026, with WI 245 and MO 24 by
jurisdiction; Finish Timing `/2026/` 325 files (the lane records 2 directories there; `[research/midwest/20-ohio-independent-sources.md]`
counted 32 date-coded dirs under `/2026/CC/` — different levels, both printed); GSE 608 meets in one
`<select>` (2005-09-08 → 2025-10-25); Hero's 256 meets on the 2026 page with 15 season pages; FlashResults has
no cumulative meet index (14 current schedule rows, a 60-link 2019 index). robots varies per host: RunnerSpace
answers 403 to curl (browser render works, Crawl-delay 10); Baumspage is REJECT because its `Year` column is
empty in **every** sampled file. Verdict: PRIMARY for the meet universe, CONDITIONAL for grade
`[timing-providers-national]`.

**National aggregators.** crosscountryratings is XC-only, 9 states, with `Class of 20XX`; RunnerSpace/DyeStat
event microsites cover NXN (22 result sets 2004-2025) and ResultsCentral (153 news items in 2026); USATF
sitemap is 3,890 URLs of which 101 are junior-olympic (the JO crosswalk is verified: AN 644030 ↔
TrackScoreboard 14258); World Athletics GraphQL needs `x-api-key`+referer and its `getAthlete(id:)` returns a
placeholder for **every** id — only `urlSlug` works; MaxPreps, AAU, NCSA Sports and FieldLevel are all REJECT.
Verdict: VALIDATION_SOURCE / CONDITIONAL only `[national-aggregators]`.

**Athletic.net.** The target namespace, and after the 2026-09-20 re-verification also a directly readable
one: anonymous 200 with browser-grade headers on meet, bio and rankings endpoints; a whole meet costs 3
requests (`GetMeetData` → `anettokens` echo → `GetEventDivisionData` + `GetAllResultsData`); rankings rows
self-seed AthleteID/IDResult/MeetID/EventID/SeasonID/GradeID; `TeamNav/Team` resolves a TeamID to its state.
Limits: `GetNavInfo` needs the SPA session, `DownloadRankings` stays 403 anonymous, `blurAfterDepth=5`
applies to some surfaces, the anonymous `minCount` disagrees with the session one (982 vs 2,117), and there
is still **no** school-roster or meet-calendar endpoint captured. Verdict: PRIMARY where reachable, otherwise
read through AthleticLIVE `[athleticnet][synthesis/08-gap-closure-2026-09-20.md]`.

---

## 3. Join-key table — every deterministic route into an Athletic.net identity

| # | Route | From → to | Mechanism (evidence) | Confidence | Failure mode |
|---|---|---|---|---|---|
| 1 | `ani` on AthleticLIVE **meet** docs | AL meet id → AN MeetID | ES meet doc carries both: `{i:38426, ani:562093}` (wayzata tenant); 129,203 of 153,524 tenant docs (84.2 %) carry `ani` `[synthesis/08-gap-closure-2026-09-20.md §3][national-aggregators]` | high | `ani` is `null` for meets the tenant never linked; per-provider counts unobtainable in one request (`tna` agg fails on 250/256 shards) `[national-aggregators]` |
| 2 | `ani` on AthleticLIVE **athlete** rows | AL row → AN AthleteID | Blob result rows: `r[0].a.ani = 15728090`; 959/959 and 136/136 sampled rows carry it; MSHSL state XC 959 rows → 242 Co2027 (100 % with id) `[synthesis/00-executive-summary.md][research/midwest/12-iowa-wayzata-results.md]` | high | none observed on sampled meets; XC-only coverage where only XC meets stream to AL |
| 3 | `t.ani` on AthleticLIVE team docs | AL team → AN TeamID | Team docs expose the AN team id alongside the tenant's own `[athleticnet]` | medium (single capture) | teams unattached to a tenant account carry no `ani` `[INFERENCE]` |
| 4 | `athleticNetId` field (IHSA) | IL finisher/team → AN AthleteID/TeamID | IHSA state-final event summaries carry `athleticNetId` for **every finisher and relay member**, plus a team `athleticNetId` `[state-assoc-greatlakes][research/midwest/13-illinois-ihsa.md]` | high (association-published) | state series only; regular-season meets absent from the IHSA API |
| 5 | Association-published AN **MeetIDs** | meeting → AN MeetID | MHSAA 89/season (≈12 requests); NSAA 28 district ids 645702–645756; SDHSAA 10 region ids; MSHSL schedule JSON (666673); OH team pages (656920/656686) + custom list 50249; WIAA instruction PDFs (13/26 sampled) `[state-assoc-greatlakes][state-assoc-plains][athleticnet]` | high where published | per-state harvesting required; MI regional grade evidence does not exist even though the meet id does `[state-assoc-greatlakes]` |
| 6 | Timer id arithmetic `"20" + ANID` | Finish Timing id → AN MeetID | 28/28 verified pairs on the rendered OH homepage (AN 275818 → `/2026/20275818.html`); same value used by `finishtiming.trackscoreboard.com/meets/20<ANID>` `[research/midwest/20-ohio-independent-sources.md]` | high (arithmetic, 0 violations) | the `20<ANID>` URL is **not** a fetchable XC path (`/2026/20275618.html` → 404); resolve through `/YYYY/CC/` instead |
| 7 | Timer-published AN links | TRXC upcoming row → AN MeetID | trxctiming.com upcoming page: **37 AN meet links (36 XC + 1 TF) in 1 request** `[research/midwest/40-missouri-trxctiming.md]` | high | MO only; upcoming season only |
| 8 | Bound calendar row carries both ids | Bound calendar id → AN MeetID | `calendar 4931063 ↔ MeetID 278813` in the same row (SD); name-independent `[research/midwest/26-south-dakota-sdhsaa.md]` | medium-high (single verified pair) | Bound directory paths are robots-disallowed; only allowed server-rendered pages usable at ≥10 s |
| 9 | `exEntryLink` in PrimeTime payloads | PrimeTime meet → AN MeetID | `computed.entryLinkHtml` resolves to `www.athletic.net` on **130 of 200** page-1 rows of `/api/results/current?year=2026`; e.g. meet 619619 `[timing-providers-national][research/midwest/22-missouri-primetime.md]` | medium (130/200 measured) | only a subset of events exposes it; the PrimeTime **live** platform is ToU-blocked without a licence (static API + files are the safe lane) `[timing-providers-national]` |
| 10 | URL composition from ids | AthleteID/TeamID/MeetID → canonical URLs | `/athlete/<id>/track-and-field`, `/team/<id>/<sport>/<season>`, `/{sport}/meet/<id>/results/all` — same integers appear as `AthleteID`/`TeamID`/`MeetID` in payloads `[athleticnet]` | high | requires the id in the first place (this table's other rows supply it) |
| 11 | `jwtMeet` mint-and-echo | AN meet → AN divisions/events/results | `GetMeetData` (anonymous) mints a 255 B `jwtMeet`; echo it as `anettokens` for `GetEventDivisionData`, `GetAllResultsData`, `GetResultsData3` — 3 requests per whole meet, `Grade` per row `[synthesis/08-gap-closure-2026-09-20.md §1]` | high (live-verified) | not a cross-source join (same meet); `GetResultsData3` is POST-only, needs lowercase `gender`, and throws use `shot`/`discus` shorts; never persist the token |
| 12 | `live.athletic.net/meets/<id>` routing | AL meet id → tenant host → AN core meet id | `75990 → dakotatiming.anet.live/meets/75990` (200) `[state-assoc-plains]` | medium for the tenant hop | the AN core meet id from an AL meet id **remains unverified for IA** (single probe returned 404); use route 1 instead |
| 13 | Tuple match (name + school + state + grad year) — MileSplit | MileSplit athlete → AN AthleteID | 65.9 % (CI 60.2–71.1) of sampled MileSplit Co2027 boys resolve into the AN grade-11 corpus; the complement (34.1 %, CI 28.9–39.8) does **not** `[synthesis/01-acceptance-answers.md §Q2/Q3][research/midwest/34-milesplit-discovery-vs-validation.md]` | medium (measured, probabilistic) | no crosswalk exists for the complement; school/name normalization is the error source |
| 14 | Tuple match — DAT/KS | name+school+class+grade → AN profile | KS: 89 grade-11 rows obtained via AN profile fetches from name tuples; DAT IL Top Times: 336 Co2027, **name-only** `[state-assoc-plains][state-assoc-greatlakes]` | low | DAT exposes no AN ids at all (all 5,933 alias rows: `athletic_net_team_id_source` = "not exposed on DAT (no athletic.net links found on any sampled page)"; 0 rows carry an id, re-counted on 2026-09-21); no city field to disambiguate `[data/school-alias-map.csv]` |
| 15 | Association path URLs (no id) | MO MSHSAA → AN state hub | `/track-and-field-outdoor/usa/high-school/missouri/mshsaa` (403 to the study client) `[athleticnet]` | low | a path is not an id; no MeetID exposed |
| 16 | USATF JO crosswalk | USATF JO registration/live → AN meet | AN MeetID **644030** + TrackScoreboard meet **14258** `[national-aggregators]` | high (single verified instance) | applies to USATF Junior Olympic T&F only; no HS state-series value |
| 17 | Self-seeding from a rankings row | AN rankings row → AthleteID + IDResult + MeetID + EventID + SeasonID + GradeID | one Grade-11-filtered page returns all of them (`{"IDResult":292277081,"AthleteID":28872883,"GradeID":11,"EventID":1,"MeetID":…}`) `[synthesis/atn-endpoint-groundtruth.md]` | high | session/entitlement blur past rank 5 applies to some ranking surfaces (`blurAfterDepth=5`, 96 blurred rows) `[athleticnet]` |
| 18 | AN TeamID from school name (one-time resolve + cache) | school name → AN TeamID | `TeamNav/Team?team=<id>&sport=tf&season=<y>` returns State and the state's `divListId`; required because coach/directory lanes hand over **no AN ids** `[athleticnet][research/midwest/06-wisconsin-wiaa.md]` | medium | name collisions (co-ops, abbreviations such as `Mass. Perry`, `Lak. West`) `[research/midwest/20-ohio-independent-sources.md]` |

### 3b. Routes that were tested and **rejected** — do not re-attempt without a decision

| Route attempted | Observed result | Why it is not a join (or not allowed) | Ref |
|---|---|---|---|
| MileSplit `/api/**` | robots `Disallow` on oh, www, ca, tx, mi, wy (+ va from prior phase) | it is the only JSON carrying `gradYear` + `windReading` + `AthleteID`; not requested by policy | `[milesplit-national]` |
| MileSplit rankings beyond row 1 | server-side masked for anonymous clients | the XHR path (`/api/v1/teams/<id>/rankings` / `/best` / `/records`) is robots-disallowed | `[milesplit-national]` |
| Athletic.net `DownloadRankings` | 403 anonymous | unverified whether it would collapse a state list to 1 request — not bypassed | `[athleticnet]` |
| Athletic.net `/TrackAndField/events` HTML | SPA shell, 0 server-rendered meet ids | no meet-calendar endpoint captured anywhere → weekly discovery still unsolved | `[athleticnet]` |
| Athletic.net `GetNavInfo` to a plain client | 403 | the nav path needs the SPA session; do not scrape the SPA | `[athleticnet]` |
| AL meet id → AN **core** meet id (IA probe) | single probe returned 404 | unverified; use the `ani` field on the same doc instead | `[state-assoc-plains]` |
| AthleticLIVE per-provider meet count via `tna` terms agg | `illegal_argument_exception` on 250/256 shards, no keyword subfield | text field, `fielddata=true` refused; use tenant enumeration as the provider proxy | `[timing-providers-national]` |
| AthleticLIVE `lsa` text agg | same failure; `lsa.keyword` works (70 buckets) | print both when quoting state counts (§5) | `[national-aggregators]` |
| MaxPreps | robots disallows `/school/`, `/team/`, `/discovery/`, `/careerprofile/`; no grade string; no athlete ids | REJECT in the aggregator lane | `[national-aggregators]` |
| AAU | 403 Cloudflare | no bypass attempted | `[national-aggregators]` |
| World Athletics `getAthlete(id:)` | returns a **placeholder record for every id** | only `urlSlug` resolves; ids are not a join key | `[national-aggregators]` |
| NCSA Sports, FieldLevel, Baumspage | REJECT (Baumspage: `Year` column empty in every sampled file) | no usable grade or id | `[national-aggregators]` |
| TFRRS `<state>.tfrrs.org` for TX/OH/CA | NXDOMAIN | HS instances exist for exactly FL, IN, NH | `[national-aggregators]` |
| RunnerSpace HTML via curl | 403 | browser render succeeds; robots `Crawl-delay: 10` | `[national-aggregators]` |
| Bound `/ia/schools/<slug>/directory/new`, `/sd/...` | 118-byte 403 | gobound robots `Disallow: /*directory` → **refused**; retracts the IA/SD Bound coach claim | `[coach-directories-national][state-assoc-plains]` |
| MI AD API `my.mhsaa.com/.../AdministrationDirectory` | 2 refusals logged | robots `Disallow: /DesktopModules/` | `[state-assoc-greatlakes]` |
| MSHSAA robots read | disallow-all by two independent parsers (RFC 9309 group merging); refused 2026-09-22T03:57:12Z | blocks a 1,006-row universe; one documented fetch was made before the read | `[state-assoc-plains]` |
| AHSAA (AL) and VHSL (VA) robots | `Disallow: /` / disallow-all → 0 requests issued | MileSplit proxies used instead (562 / — teams) | `[state-assoc-southcentral][state-assoc-southeast]` |
| TX UIL PDFs under `/files/` | robots-disallowed | every source PDF is public but disallowed → owner decision open | `[state-assoc-southcentral]` |
| UHSAA results host `results.runnercard.com` | 503 twice | UT athlete results unobtainable this pass | `[state-assoc-mountain]` |
| CIAC results host | expired TLS certificate | verification was **not** disabled | `[state-assoc-midatlantic]` |
| NIAA PrestoSports `/coaches/Landing`, `/sports/track` | 202 + 0-byte body | conditional/unverified | `[state-assoc-westcoast]` |
| MD / NJ / PA / NY direct directories | 302 → `/login`, page 1 only, client-rendered rows, `/documents/` disallowed | login-gated or unenumerable | `[state-assoc-midatlantic]` |
| IHSA regular-season calendar (`/v1/sports/boys-track-field/calendar`) | 404 | regular season stays behind the MileSplit paywall in IL | `[state-assoc-greatlakes]` |
| OH `officials.myohsaa.org` from the census client | connection reset (rustls); an `openssl s_client` handshake succeeds **with** ALPN and fails without | the edge filters ClientHello; no impersonation attempted, so the census run contributed 0 coach rows | `[synthesis/10-measured-census.md]` |
| MI `/schools` copies in the corpus | 145,587 B SPA shells, 0 `<tr>` | cannot re-derive the 755 count; enrollment PDF bytes not retained | `[state-assoc-greatlakes]` |
| WIAA `/Reports/` | 403 | no HTTP validator/change signal for the coach table either | `[state-assoc-greatlakes]` |

---

## 4. Coverage gaps — what no corpus source covers

| State | Gaps no corpus source closes |
|---|---|
| **WI** | Regular-season grade beyond tournament files (sectionals: ~a third of uploads are image-only); indoor; no HTTP validator/change signal for the coach table; `/Reports/` returns 403; PrimeTime live unreachable without a licence `[state-assoc-greatlakes]` |
| **MN** | Regular-season results at the association level; rosters are in-season only (a July run returns nothing); coach emails are per-coach opt-in — many rows are phone-only `[state-assoc-plains]` |
| **IA** | Any association-side coach/AD publication; Bound exposes no AN ids; AL→AN core meet id unverified; regular season fragmented across 8+ timer hosts `[state-assoc-plains]` |
| **IL** | Regular-season meets (no calendar endpoint; `/v1/sports/boys-track-field/calendar` = 404); XC tournament ids exist only in a site JS chunk; results beyond the state series sit behind the MileSplit paywall; no IL-specific timing-provider report exists in the corpus `[state-assoc-greatlakes]` |
| **MI** | Grade evidence at regionals (does not exist at MHSAA); coach rows of any kind; per-sport school participation; regular-season meets; the named finals PDFs have no retained bytes (`CAPTURES.md` section C) `[state-assoc-greatlakes]` |
| **IN** | Any coach/AD contact; AN ids of any kind; the sectional school-name lists behind the 391 declared slots; TFRRS regular-season corpus is thin mid-September (56 track meets vs 1,022 in 2025; 1 XC vs 128) — re-measure in November `[state-assoc-greatlakes][state-assoc-plains]` |
| **OH** | No results API; the OHSAA-approved team-page listing sits behind a 403 API under an Angular shell; district PDFs only partially sampled (45) and carry no HTTP validators; coach attributes still depend on a TLS-fingerprint-sensitive portal `[state-assoc-greatlakes][synthesis/10-measured-census.md]` |
| **MO** | District/sectional results (no association artifact); association school-universe crawl is blocked by a disallow-all robots read; some 2026 PDFs are image-only scans (OCR out of scope); **no coach/AD data anywhere** `[state-assoc-plains]` |
| **KS** | Coach names beyond AD; TF grades (null); non-champion team rosters (`RetrieveRoster` returns `[]`); weekly schedules; association directories include junior-high rows (~526 vs ~348 classified HS) and `DateModified` is stale `[state-assoc-plains]` |
| **NE** | Emails of any kind; XC fields beyond the district top-15 (state XC needs the timer site); district AN meet mapping table not implemented; per-sport counts at the association `[state-assoc-plains]` |
| **ND** | Coach emails (none exist); co-op team attribution without the coop sheet; association-published results beyond yearbooks; AL meet enumeration and district AN meet ids not collected `[state-assoc-plains]` |
| **SD** | Regular-season results from SDHSAA/Bound; state-meet results only as yearbook PDFs (last season) or on AN; athlete ids of any kind in the association lane; compliant Bound access is refused by robots `[state-assoc-plains][coach-directories-national]` |
| **Non-Midwest (39 jurisdictions)** | None of them has a measured statewide Class-of-2027 population; DE, MS, AL, WI(WA) and NY have no anonymously enumerable school universe; WY, SC, WV, VA, DC and NV have no association-side athlete results; FL/GA/SC/VA/WV have no numeric grade from association artifacts; TX's whole source set sits under robots-disallowed `/files/`; LA's membership count is unreadable without solving a font permutation; NH/NJ/PA/UT/CT/NV have client-rendered or 202-blocked directories; **no lane observed an AN id on a non-Midwest association page** `[state-assoc-mountain][state-assoc-midatlantic][state-assoc-southeast][state-assoc-westcoast][state-assoc-southcentral][national-aggregators]` |

---

### 4b. Every cell the corpus leaves `unknown` — and the capture that would fill it

| Jurisdiction | Unknown cell | Why the corpus has no value | Capture that would fill it |
|---|---|---|---|
| MO | coach/AD contact at any level | association publishes none; verified on two school pages; 0 rows in `data/coach-contacts.csv` | none known — treat as a dead end unless MSHSAA later publishes a directory `[state-assoc-plains]` |
| IN | coach/AD contact | myIHSAA is authenticated and was not attempted | an authenticated myIHSAA session (owner decision) `[state-assoc-greatlakes]` |
| MI | coach/AD contact, per-sport participation | no statewide directory; AD API robots-refused | a change of the `/DesktopModules/` robots rule; otherwise earlier-study rows only `[state-assoc-greatlakes]` |
| KS | coach names beyond the AD; T&F grades | directory has no coach fields; `RetrieveResultsByActivity` T&F rows carry null grade | a coach field on the directory, or MeetPro/DA event sheets `[state-assoc-plains]` |
| NE | any email address | NSAA export carries zero email fields (1,528 rows / 1,217 names) | a district-published contact sheet, state by state `[state-assoc-plains]` |
| ND | any coach email; co-op attribution | association publishes names only; co-op sheet not fetched | the NDHSAA co-op sheet named in report 25 `[state-assoc-plains]` |
| SD | membership list re-derivation; per-coach emails beyond 4 | 176 is a Bound parse citation; Bound directory paths robots-refused | a robots-allowed SDHSAA member page (none observed) `[state-assoc-plains]` |
| IA | AD/coach publication; AL→AN core meet id | no field exists at the association; Bound refused | Bound under a licence, or the IA timer set `[state-assoc-plains]` |
| WI | regular-season grade beyond tournament files; indoor | `/Reports/` 403; sectionals ~1/3 image-only | PrimeTime licence, or the Winter/Indoor MeetID set `[state-assoc-greatlakes]` |
| IL | regular-season meets; T&F tournament ids in crawlable form | calendar endpoint 404; ids live in a site JS chunk | a MileSplit entitlement, or the JS chunk harvested once `[state-assoc-greatlakes]` |
| OH | grade at district/regional level; results API | district PDFs sampled only (45); team-page listing behind a 403 API under an Angular shell | Finish Timing `/2026/CC/` harvest + OHSAA portal session `[state-assoc-greatlakes]` |
| MN | regular-season results; phone-only coach rows | MSHSL publishes schedule/AN MeetID but not results; email is opt-in per coach | timer-host harvest (GSE, Hero's, Wayzata) `[state-assoc-plains]` |
| LA | membership count per class | 306 derived from the state-meet PDF; PDF fonts are subset-glyph permuted (108 pages) | solve the font encoding, or an HTML member list (the `/school-directory` page is a nav shell) `[state-assoc-southcentral]` |
| MS | school universe; timing provider | 403 WAF on apex and www | an IP/agent change decision, untested `[state-assoc-southcentral]` |
| DE | school universe | Cloudflare interstitial on every host path | none attempted `[state-assoc-midatlantic]` |
| NH | school universe (42 pages, entries client-rendered) | rSchoolToday data layer not captured | drive the client renderer `[state-assoc-midatlantic]` |
| PA | school rows | rows render client-side; 220 sitemap nodes / 25 letters counted instead | drive the client renderer `[state-assoc-midatlantic]` |
| NY | school universe | `/schools` 404 and `/documents/` robots-disallowed | none — blocked by policy `[state-assoc-midatlantic]` |
| NJ | total school count | page 1 shows 47 rows; pagination not exposed | find the pagination parameter `[state-assoc-midatlantic]` |
| WA (WA) | coach/AD, results, timing | Arbiter/FinalForms human tooling only | none observed `[state-assoc-westcoast]` |
| NV | coach/AD, results | PrestoSports 202 + 0-byte body on two paths | an endpoint that returns a body `[state-assoc-westcoast]` |
| MT | coach/AD, results | Arbiter SPA shell (4,278 B); Competitive Timing renders client-side | drive the renderer `[state-assoc-mountain]` |
| WY | school names; athlete results | class counts only, names not published; MileSplit Live has 0 AN matches | none observed; 0 anonymously reachable `[state-assoc-mountain]` |

## 5. Conflicts, supersessions and retained duplicates

| Item | Value A | Value B | Which is retained |
|---|---|---|---|
| IL school-list bytes | 451,182 B (ledger `gaps/38/ledger.jsonl`, `[state-assoc-greatlakes]`) | 451,188 B (on disk, md5 498179f68c66793396dd4c6123a4a4bc) | **both printed**; the 828-row parse holds under either; never averaged `[state-assoc-greatlakes]` |
| OH state-final PDF pages | 83 (analysis) | 82 (artifact table) | **both printed**; unresolved `[state-assoc-greatlakes]` |
| WI grade-11 rows (26-file 2026 sample) | 2,786 (matcher in report 35) | 2,653 (retained `summary-v2.json`) | **range 2,653–2,786**, not a point value `[state-assoc-greatlakes]` |
| IL DAT team ids | 4,964 (site search) | 3,340 (retained CSV slice) | **both printed** — different artifacts, never summed `[state-assoc-greatlakes]` |
| AthleticLIVE meet docs | 153,774 (wildcard `_msearch`) | 153,524 (tenant inventory sum) | **both printed**; 250-doc delta = index churn since the package build `[national-aggregators]` |
| SD / ND all-time meet docs | SD 3,188, ND 5,222 (`match` on `lsa`) | SD 1,869, ND 875 (`lsa.keyword` aggregation) | **both printed**; the `lsa` numbers are upper bounds, the keyword counts are exact per that index `[national-aggregators][timing-providers-national]` |
| IA member schools | 362 distinct slugs (fresh capture) | 379 (report figure, one day earlier) | **both printed**; use 362 with the capture path `[state-assoc-plains]` |
| WIAA `champs`/MI `LPG` coaches vs pipeline | research 9-school / 14-AD samples | pipeline census rows (WI 2,550 coaches/2,155 emails; MI 6) | **both printed**; census numbers are one snapshot (`core = Athletic.net off`) `[state-assoc-greatlakes][reports/census-by-state.csv]` |
| IA/SD Bound coach claims | `[29]`/prior synthesis: Bound `/staff` gives names (IA) and names for 176 schools (SD) | coach-directories lane: **refused**, robots `Disallow: /*directory`; the IA path answered 118 B/403 | **the refusal is the compliance verdict**; the rows in `data/coach-contacts.csv` keep prior-study provenance and are marked unverified by that lane `[coach-directories-national][state-assoc-plains]` |
| AD-email jurisdictions | counts block: `ad_email_available_tier1: 5` | per-jurisdiction `fields`: exactly **4** — IL, KS, OH, WI (verified by enumerating all 51 `fields` blocks with `jq` on 2026-09-21; `coach_email` = 3 = IL, OH, WI, matching the counts block) | **both printed**; the fifth jurisdiction is not named anywhere in the corpus → `unknown`; the counts block is the outlier `[coach-directories-national]` |
| Athletic.net network boundary | pre-2026-09-20: "headed session required, curl → 403" | 2026-09-20: anonymous 200 with browser-grade headers on meet/bio/rankings endpoints | **the 2026-09-20 re-verification supersedes**; only `GetNavInfo` and `DownloadRankings` remain gated `[synthesis/08-gap-closure-2026-09-20.md §1]` |
| Pipeline gender split vs Co2027 total | e.g. IL 26,472 | IL 26,488 (Co2027) | **never print a total as boys+girls**; the split does not sum in IL/IN/OH/WI `[state-assoc-greatlakes]` |
| `synthesis/02-state-playbook.md` (pre-lane seed) | its WI/MI/MO/KS/NE numbers agree with the lanes; its IA/SD Bound-coach rows are the retracted ones; it has no national-lane data | lanes: IL 828 re-derived, OH 815 re-derived, MI 755 and IN 413 quote-only, 256 tenants, 129,203/153,524 AN-linked docs | **lanes supersede** for every count they re-derived; the seed's per-state adapter recipes stand `[state-assoc-greatlakes][state-assoc-plains][national-aggregators]` |
| CT / ME / RI contact verdicts | midatlantic lane on the association hosts: "none public" | coach-directories lane on the delegated platforms (`ciac.fpsports.org`, `mpa.cc`, `riil.org/Directory.aspx`): `coach_name` **yes**, `ad_name` **yes**, emails **no** | **the coach lane supersedes for these three** — names exist, emails do not; both lanes cited in §1b `[state-assoc-midatlantic][coach-directories-national]` |
| OR / ID coach verdicts | westcoast lane: OR publishes **coach names + emails per activity**; mountain lane: ID publishes **AD name/phone/email** (base64-obfuscated) | coach-directories lane: `osaa.org/schools/full-members` and `idhsaa.org` home grade `coach_name`/`ad_name` = **no** | **different surfaces, not a contradiction**: the per-activity OR pages and the ID AD-directory page are the stronger captures and stand; the two laning results are keyed to different URLs `[state-assoc-westcoast][state-assoc-mountain][coach-directories-national]` |
| Tier-1 directory count | "22 directories collected" (counts block) | outcomes: 12 partial + 10 no-contact = 22 fetched, 26 client-rendered, 3 robots-disallowed (sum 51) | **both consistent** — 22 is the *collected* set, not the *useful* set; only IL, OH, WI yield coach emails `[coach-directories-national]` |
| Corpus filing hazard | `tools/scratch-11/`, `p*-ihsaa-*`, `bound-ihsaa-*` are **Iowa's** (iahsaa.org) files | Indiana's association is also `IHSAA` (ihsaa.org) | report 11 = 11-iowa-ihsaa-ighsau.md; do not re-introduce the collision `[state-assoc-greatlakes]` |

---

## 6. Open questions that block a decision

| # | Question | Jurisdiction(s) | Blocks | Cost to answer |
|---|---|---|---|---|
| 1 | Does the MSHSAA disallow-all robots read reflect operator intent? | MO | a 1,006-row school universe | one email to MSHSAA `[state-assoc-plains]` |
| 2 | Is a mailto request for the AHSAA classification list acceptable? | AL | universe | owner decision + 1 email `[state-assoc-southcentral]` |
| 3 | Are UIL `/files/` PDFs permitted, or only the HTML index? | TX | universe + results | owner decision `[state-assoc-southcentral]` |
| 4 | Can ND co-op teams be resolved from the sport-cell annotation alone? | ND | team attribution | 1 fetch of the coop sheet named in report 25 `[state-assoc-plains]` |
| 5 | What is the AN core meet id for a given AthleticLIVE meet id? | IA (and every state) | join route 12 | retest via the doc's `ani` field instead of the probe `[state-assoc-plains]` |
| 6 | OH state-final PDF: 83 or 82 pages? | OH | evidence packaging | re-read the retained artifact `[state-assoc-greatlakes]` |
| 7 | WI grade-11 yield: 2,786 or 2,653? | WI | yield estimate | re-run the matcher against `summary-v2.json` `[state-assoc-greatlakes]` |
| 8 | IL DAT team ids: 4,964 or 3,340? | IL | discovery scope | re-run the site search with the retained CSV `[state-assoc-greatlakes]` |
| 9 | Where are the MI finals PDF bytes (`2025lpgd1final.pdf`, `2026-UP-Boys-D1-Finals.pdf`)? | MI | any regional-grade claim | re-fetch 2 PDFs (`CAPTURES.md` section C) `[state-assoc-greatlakes]` |
| 10 | Which schools fill the 391 IN sectional slots? | IN | postseason enumeration | 25 sectional pages `[state-assoc-greatlakes]` |
| 11 | Does the NDHSAA `/schools` index hide schools behind a filter/pager? | ND | universe completeness | inspect the Alpine markup (no pager control found) `[state-assoc-plains]` |
| 12 | Is the MileSplit Co2027 complement (34.1 %, CI 28.9-39.8) real athletes or matching error? | 12 Midwest states | MileSplit = discovery source vs validation source | 100-athlete manual audit `[synthesis/03-adapter-ranking.md]` |
| 13 | Would `DownloadRankings` collapse a state list to one request? | all 51 | request-cost budget | 403 anonymous — needs an entitlement decision, not more probes `[athleticnet]` |
| 14 | What is the weekly new-meet discovery route? | all 51 | `07-weekly-incremental-design.md` | 1 browser session on `/events` + a division home page `[athleticnet][synthesis/07-weekly-incremental-design.md]` |
| 15 | How formal is the MIAA AN entry partnership (reliable enough to depend on)? | MA | entry/result route | 1 email `[state-assoc-midatlantic]` |
| 16 | LHSAA membership count per class | LA | universe | solve the subset-font permutation in the 108-page PDF `[state-assoc-southcentral]` |
| 17 | Is the MS 403 IP/agent-based or a global shield? | MS | the whole state | one agent-change probe (never attempted) `[state-assoc-southcentral]` |
| 18 | What is the ArbiterLive per-school JSON endpoint? | KY | rosters + coaches | inspect the network tab once `[state-assoc-southcentral]` |
| 19 | Does the TN roster page expose athlete names publicly? | TN | Co2027 discovery | capture one per-school roster output `[state-assoc-southcentral]` |
| 20 | Do PrimeTime/Karmarush grant a data licence? | WI, MO, MN, IA, ND | live-result ingestion | `info@pttiming.com`, `legal@karmarush.com` `[synthesis/03-adapter-ranking.md]` |

## 7. Provenance — the corpus files behind the numbers

Row counts are `wc -l` minus one header line, measured on this machine; the column names are the real
headers, so a reader can re-derive any figure in §1-§6 without re-fetching anything.

| File | Rows | Real header (abbreviated) |
|---|---:|---|
| `data/source-coverage-matrix.csv` | 66 | `state, source_family, role, enumeration_ids, grade_evidence, result_evidence, coach_evidence, access_class, report_ref, confidence` — the machine-readable form of §2 |
| `data/atn-id-fields.csv` | 45 | `id_name, sample_value, source_request, notes` — every AN id field with a sample value |
| `data/adapter-ranking.csv` | 10 | `rank, adapter, states_covered, athletes_est, coach_est, requests_est, effort_est, ratio_notes, report_refs` |
| `data/milesplit-coverage-matrix.csv` | 12 | per-state `hs_team_records_on_milesplit`, `sampled_c2027_*`, `est_c2027_total_order_of_magnitude_low/high`, `athnet_corpus_boys_g11`, `sampled_c2027_boys_outdoor_match_pct`, rankings/raw-results free flags |
| `data/milesplit-candidate-records.csv` | 12 | `state, athlete_name_*, gender, grad_year, class_of_2027_evidence, school, ms_team_id, ms_athlete_id, confidence` |
| `data/dat-provider-coverage.csv` | 13 | `state, dat_t&f_team_records, dat_xc_team_records, canonical_schools, meets_idx, hs-marked, dat_live_hs_performance_lists, strongest_hs_result_artifact_inspected` |
| `data/athleticlive-tenant-inventory.csv` | 256 | `tenant, meet_docs, meet_docs_with_ani, pct_with_ani` (sums to 153,524 / 129,203) |
| `data/coach-contacts.csv` | 2,298 | `school, city, state, sport, role, coach_name, public_professional_email, ad_name, ad_email, source_url, last_observed` |
| `data/school-alias-map.csv` | 5,933 | `state, canonical_school, canonical_key, alias_variants, dat_*_team_ids, athletic_net_team_id, athletic_net_team_id_source, verification` (all 5,933 rows record `not exposed on DAT…`; 0 carry an id) |
| `data/canonical-coaches.csv` | 6,235 | `coach_id, name, role, sport, gender, school_id, school_name, professional_email, evidence_sources` |
| `data/canonical-meets.csv` | 9,667 | `meet_id, state, date, name, location, level, sports, athleticnet_meet_id, athleticnet_url, source_identities` |
| `data/athleticnet-meet-seeds.csv` | 9,844 | `state, date, level, athleticnet_meet_id, athleticnet_url, athleticlive_meet_ids, tenants, source` |
| `data/canonical-schools.csv` | 11,407 | `school_id, name, state, association, classification, enrollment, co_op, athleticnet_team_id, milesplit_school_id, aliases, identity_count` |
| `data/athleticlive-midwest-2026-meet-seeds.csv` | 20,145 | `tenant, athleticlive_meet_id, athleticnet_meet_id, name, state, start, end, has_results, sport_flags_tf, timer_credit` |
| `data/athleticlive-midwest-2026-meets-all.csv` | 20,254 | same minus `sport_flags_tf`/`timer_credit` |
| `data/dat-team-index.csv` | 22,527 | `state, dat_league_name, dat_league_id, dat_team_id, team_gender, dat_team_name_raw, level_guess, verification` |
| `data/canonical-athletes-co2027.csv` | 183,875 | `athlete_id, name, grad_year, school_id, sports, identity_confidence, athleticnet_athlete_id, milesplit_athlete_id, profile_urls, source_namespaces, observed_grades` |
| `data/recruiting-co2027.csv` | 183,875 | adds `head_track_coach(_email)`, `head_xc_coach(_email)`, `athletic_director(_email)` |
| `data/athleticnet-athlete-seeds.csv` | 189,705 | `athleticnet_athlete_id, athleticnet_url, name, grad_year, gender, state, school_name, derived_from_sources, milesplit_athlete_id` |
| `research/midwest/evidence/gaps/38/il-schools.json` | 828 parsed rows | IL school universe (md5 `498179f68c66793396dd4c6123a4a4bc`) |
| `research/midwest/evidence/gaps/45/ohsaa-enrollment.html` | 816 `<tr>` = 1 header + 815 | OH enrollment/`OhsaaSchoolId` table |
| `reports/census-by-state.csv` | 13 + header | per-state census — the only source of §1c |
| `reports/best-results-co2027.csv` | 7,743 + header | best result per Co2027 athlete (1.5 MB; sibling `best-results-co2027.jsonl` 3.2 MB) |
| `reports/report.json` / `report-core.json` | 12,719 B / 9,263 B | pipeline run outputs (`core` = Athletic.net off) |
| `reports/census-service-2026-09-21.xlsx` | 662,980 B | spreadsheet export of the census |
