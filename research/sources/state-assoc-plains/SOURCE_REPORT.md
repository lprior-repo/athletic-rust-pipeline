# state-assoc-plains — consolidated state-association source report (IA, KS, MN, MO, ND, NE, SD)

- Lane: `research/sources/state-assoc-plains/` (national lane contract, consolidation mode).
- Jurisdictions: **Iowa, Kansas, Minnesota, Missouri, North Dakota, Nebraska, South Dakota**.
- Scope: convert the Midwest phase research (`~/Downloads/midwest-tfxc-source-research/`, 46 reports +
  synthesis + data products) into the national research-lane contract, and re-verify the headline counts
  first-hand. **No new research, no new sources, no repo code touched.**
- Fresh observation window: **2026-09-22T03:56:21Z – 04:01:39Z** (= 2026-09-21 evening America/Chicago).
  18 requests total, sequential, ≤1 request/second/host, `robots.txt` fetched before each host's content.

## 0. Evidence tags — read this first

| Tag | Meaning |
|---|---|
| `[V]` | Verified **in this lane** by a command recorded verbatim in `samples/CAPTURES.md`. |
| `[C <path>]` | Cited from an existing capture/report on disk (path given). Not re-fetched here. |
| `[I]` | Inference / reasoning. Explicitly **not** measured. |

Every count, id and URL below carries one of these tags. Where a Midwest report and this lane's fresh
capture disagree, both numbers are given with their provenance (§0.3).

### 0.1 Inputs read (all read in full or in the cited ranges)

- `~/Downloads/midwest-tfxc-source-research/research/midwest/09-minnesota-mshsl.md`,
  `10-minnesota-results-timers.md`, `11-iowa-ihsaa-ighsau.md`, `12-iowa-wayzata-results.md`,
  `21-missouri-mshsaa.md`, `22-missouri-primetime.md`, `23-kansas-kshsaa-directathletics.md`,
  `24-nebraska-nsaa.md`, `25-north-dakota-ndhsaa.md`, `26-south-dakota-sdhsaa.md`,
  `29-coach-contact-graph.md`
- `…/data/source-coverage-matrix.csv` (67 rows), `adapter-ranking.csv`, `coach-contacts.csv` (2,298 rows)
- `…/tools/iowa-11/bound-ihsaa-boystrack-teams-2025-26.csv`, `…/tools/ne/*` (29 files),
  `…/research/midwest/evidence/gaps/43/factsheet.txt`
- Repo fixtures: `crates/midwest-census/tests/fixtures/{ks,mshsl,plain_names}/**`
- Repo adapters: `crates/midwest-census/src/sources/{ks,mshsl,plain_names}/**`
- Repo run output: `var/midwest-census/out/{census-by-state.csv,report.json,schools.jsonl}` (run dated 2026-09-21)

### 0.2 First-hand re-verification (summary; commands + file-level detail in `samples/CAPTURES.md`)

| Jurisdiction | Endpoint | Status | Bytes | Count derived | Tag |
|---|---|---|---|---|---|
| IA | `www.iahsaa.org/member-schools/` | 200 | 268,201 | 380 `<tr>`, **379** distinct school names, 364 rows with a `/schools/<slug>/` link | `[V]` |
| KS | `kshsaa-api.kshsaa.org/PublicClassifications` | 200 | 31,885 | **348** classified schools = 36/36/36/64/64/112 by class | `[V]` |
| MN | `www.mshsl.org/sitemap.xml?page=1` | 200 | 341,086 | **664** `/schools/` URLs + 1,097 `/tournaments/` URLs | `[V]` |
| MO | `www.mshsaa.org/Schools/SchoolListing.aspx` | 200 | 721,371 | **1,006** org rows = 316 HS + 360 JH + 330 combined; 719 member / 257 affiliate / 30 hsa | `[V]` |
| ND | `ndhsaa.com/schools` | 200 | 97,754 | **169** school anchors (unique = total) | `[V]` |
| NE | `secure.nsaahome.org/nsaaforms/direxportscreen.php` | 200 | 11,585 | 314 `<option>` = **312** schools + 2 UI rows | `[V]` |
| SD | `sdhsaa.com/cross-country-region/` | 200 | 286,293 | **10** distinct Athletic.net region meet ids | `[V]` |
| ND | `POST search.athletic.live/heros_meet_list/_search` | 200 | 1,369 | **6** ND 2026 XC meets, each with AthleticLIVE + Athletic.net ids | `[V]` |

### 0.3 New findings and corrections to the Midwest phase

1. **MSHSAA (`www.mshsaa.org`) robots.txt ends with a blanket disallow for generic crawlers.** The file
   (36 lines, 670 B, sha256 `d863f28d…`) closes with
   `user-agent: *` / `disallow: /`. Under RFC 9309 §2.2.1 ("If more than one group matches … the groups
   MUST be combined") this disallows the **entire host** for `*`, not just the paths listed in the earlier
   `*` group. `[V]` Independently, the sibling lane `coach-directories-national` captured the same file
   (`tools/robots/mshsaa.org.txt`, 634 B, content-identical ignoring CR/BOM) and its fetch log recorded
   `{"host": "www.mshsaa.org", "kind": "refused", "allow": false, … , "ts": "2026-09-22T03:57:12Z"}`
   (`tools/fetch-log.jsonl:154-155`) — a second, independently written robots evaluator reached the same
   conclusion. Report 21's access section listed only the path-level disallows and the three named
   agents, so it understated the posture. **Consequence: Missouri's association surface is not
   crawlable as-is; my own probe of the listing page is recorded as a one-off, robots-flagged fetch, and
   no automated MO association collector should be scheduled without permission.**
2. **Missouri's high-school-level universe reconciles exactly.** Report 21 quotes "646 high-school-level
   organizations, 592 full members". My fresh parse of the same listing: 316 `orgtype=1` (senior HS) +
   330 `orgtype=3` (combined 7–12) = **646**, of which 262 + 330 = **592** are `membertype=member`.
   `[V]` The other 360 rows are junior-high orgs. Both figures are correct; they measure different slices
   of the same 1,006-row page.
3. **South Dakota member count has two defensible values.** Bound member directory = **176** schools
   `[C 26]`; the SDHSAA factsheet PDF (text extract `…/evidence/gaps/43/factsheet.txt:14-16`) prints
   "Member Schools **179**" (with 29 sports+activities, 32,107 participants). Use 176 for the Bound/Bound-id
   spine and 179 as the association's own membership figure; do not present either as "the" SD universe.
4. **Iowa member-row arithmetic (minor).** Fresh capture gives 380 `<tr>` / 379 distinct first-cell names /
   364 rows containing a `/schools/` link `[V]`; report 11 recorded 379 rows / 363 links. The one-row gap
   is unclassified (header/footer row handling); the 379-name figure agrees.
5. **North Dakota 2026 XC meets re-confirmed live.** The six AthleticLIVE meet docs with pre-attached
   Athletic.net ids reproduced exactly (76088/277450, 76109/277021, 76112/276762, 76117/272168,
   76139/278116, 76140/278115) `[V]` — the ES index path in report 25 still works unchanged.
6. **IHSAA apex robots redirects to `www`.** `https://iahsaa.org/robots.txt` → 200 via redirect to
   `https://www.iahsaa.org/robots.txt`, 146 B, identical content `[V]` (host-scoped robots evaluation is
   therefore consistent for IA).

## 1. Summary table — seven jurisdictions

| ST | Association (host) | School universe (measured) | Repo adapter | Recommendation (this lane) |
|---|---|---|---|---|
| **IA** | IHSAA `www.iahsaa.org` + IGHSAU `www.ighsau.org`; platform Bound `gobound.com/ia/{ihsaa,ighsau}` | **379** members (`[V]`), 365/369 Bound team programs 2025-26 `[C 11]`, Bound school index 444 entries (not a clean member list) `[C 11]` | **none** | **PRIMARY** (Bound IA tenants + association lists) |
| **KS** | KSHSAA `www.kshsaa.org` + `kshsaa-api.kshsaa.org`; results `kshsaachamps.org` | **348** classified members (`[V]`), **526** directory rows incl. JH (AD directory) `[C 23]` | `sources/ks` ✅ | **COACH_SOURCE** (AD-only, 1 request) + VALIDATION secondary |
| **MN** | MSHSL `www.mshsl.org` (Drupal 11 + JSON:API) | **664** schools (`[V]` stream), 665 in store, 1,097 tournaments `[V]` | `sources/mshsl` ✅ | **PRIMARY** (school+team+AD/coach) |
| **MO** | MSHSAA `www.mshsaa.org`; timer PrimeTime `data.pttiming.com` | **1,006** org rows / **646** HS-level / **592** full members (`[V]`) | **none** | **CONDITIONAL** (robots-disallowed host; school universe + grade oracle, manual or permissioned) |
| **ND** | NDHSAA `ndhsaa.com` + `ndhsaanow.com`; timer Hero's on AthleticLIVE | **169** members (`[V]`) | `sources/plain_names` (nd) ✅ | **COACH_SOURCE** (names only) + RESULT_SOURCE for state series |
| **NE** | NSAA `nsaahome.org` + `secure.nsaahome.org`; S3 mirror `nsaa-static.s3.amazonaws.com` | **312** directory entries (`[V]`) | `sources/plain_names` (nsaa) ✅ | **COACH_SOURCE** (names only) + VALIDATION_SOURCE (grade-bearing postseason files) |
| **SD** | SDHSAA `sdhsaa.com` + Bound `gobound.com/sd/associations/sdhsaa` | **176** Bound members `[C 26]` / **179** association factsheet `[C†]` | **none** | **CONDITIONAL** (seed + coach-name slice only; 176 requests; no athlete-level data) |

`[C†]` = `~/Downloads/midwest-tfxc-source-research/research/midwest/evidence/gaps/43/factsheet.txt`.

## 2. Shared platform facts that govern ≥3 plains states

### 2.1 AthleticLIVE — Athletic.net's own live-results product
`search.athletic.live` (Elasticsearch, public, no auth), `…/meet_<id>/liveRunStandings/<rui>` (RTDB JSON),
`athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<eventId>` (per-event result docs). Every doc
carries the **Athletic.net meet id (`ani`)**, and result rows carry `a.ani` (athlete) and `t.ani` (team).
Verified live for ND §7; Iowa/Kansas/Minnesota/Nebraska instances documented in `[C 12][C 10][C 23][C 24]`.
It is a **white-label of the same pipeline as Athletic.net** → excellent for coverage/joins, **not**
independent corroboration `[C 12]`.

### 2.2 Bound (GoBound / DirectAthletics) — IA, SD, plus KS/MN DAT records
School directory `/…/schools/<slug>/directory/new` (role + name; email is **per-school opt-in**), team cards
with `data-classes`, rosters with grade (`Year`), meet comp ids `h…`. `robots.txt` sets `Crawl-Delay: 10` `[C 11]`.
Team ids are **season-scoped** (0 of 365 shared between 2025-26 and 2026-27 for IA boys T&F) `[C 11]`;
school GUIDs are stable. Bound `/leaderboard` returned HTTP 202 with an empty body for SD `[C 26]`.

### 2.3 Athletic.net itself
Cloudflare-403 to plain curl; official/rankings surfaces need a browser session `[C 04]`. Consequence: the
plains lanes that require Athletic.net data must go through the Chromium lane, and everything else should
harvest ids from AthleticLIVE/Bound/association links instead (ND, SD, IA do this successfully).

### 2.4 MileSplit per-state sites
Grade-bearing rosters under `/teams/<id>/roster` are robots-allowed and are the **only** per-athlete grade
enumeration common to all seven states; `/api/` and `/rankings` are robots-disallowed. Owners: the
`milesplit*` lane — this lane only records the dependency (`inventory` in `coverage.json`).

### 2.5 Robots posture of the seven association hosts (fresh, 2026-09-22)

| Host | robots status | Key rules `[V]` |
|---|---|---|
| `www.iahsaa.org` | 200 (146 B) | disallow `/wp-admin/`, `/search/`, `/*?*`; sitemap present |
| `www.kshsaa.org` | 200 (135 B) | `Crawl-delay: 30` for `*`; disallow `/Arbiter/` |
| `kshsaa-api.kshsaa.org` | **404** (0 B) | none published |
| `www.mshsl.org` | 200 (2,385 B) | disallow `/core/`, `/profiles/`, `/search*`, `/admin/`, `/user/*`, `*.pdf|doc|docx|xls|xlsx|csv`; blocks GPTBot/AhrefsBot/MJ12bot/dotbot |
| `www.mshsaa.org` | 200 (670 B) | path-level disallows **plus a final `user-agent: * / disallow: /` group** → host-wide disallow for generic crawlers |
| `ndhsaa.com` | 200 (24 B) | `Disallow:` (nothing) |
| `nsaahome.org` | 200 (31 B) | `Crawl-delay: 5` |
| `secure.nsaahome.org` | 302 | `robots.txt` redirects to the apex host |
| `sdhsaa.com` | 200 (156 B) | `Crawl-delay: 3`; disallow `/*?`, `/calendar/action*`, `/events/action*` |

## 3. Iowa (IA)

**Association / sources.** IHSAA (boys) `https://www.iahsaa.org/` · IGHSAU (girls) `https://www.ighsau.org/`
· competition platform Bound `https://gobound.com/ia/ihsaa/…` and `/ia/ighsau/…` · results from AthleticLIVE
timers, led by Wayzata Timing.

**Adapter status: none.** `ls crates/midwest-census/src/sources/` has 12 adapter modules; none is IA
(`grep -rn "iahsaa\|ighsau" crates/ src/` → no matches `[V]`). In the 2026-09-21 store run, IA school rows come
from `athleticlive_athletes` (792 schools), `milesplit_ia` (410) and `coach_contacts_csv` (5) — **there is no
Iowa association adapter, and no Iowa coach directory**.

| §13 field | Value |
|---|---|
| Source name | IHSAA member/classification pages; IGHSAU BEDs + classification PDFs; **Bound Iowa** (`/ia/ihsaa`, `/ia/ighsau`); AthleticLIVE timers (Wayzata et al.) |
| Geographic coverage | Iowa only |
| Sports | B/G cross country (fall), B/G outdoor T&F (spring). Indoor exists administratively (IGHSAU order-of-events, IATO manual) with no competition program found in Bound `[C 11]` |
| Historical depth | Bound season selectors to ~2009-10 `[C 11]`; IGHSAU state T&F archive 2016,17,21–25 + 2026 PDF; IHSAA 2018–19 static HTML (S3); Wayzata IA HS-flagged meets 2019→2026 = 15/13/23/33/53/54/62/53 per year `[C 12]` |
| Discovery mechanism | 1 GET member table → `/schools/<slug>/`; Bound slug differs from IHSAA slug (`adm-adel`→`adm`) so 1 extra GET/school; Bound team lists per sport×class×season; AthleticLIVE ES per-timer indices (`wayzata_meet_list` …) and the network index; IATC weekly results index maps meets→timers |
| Stable identifiers | Bound team/comp/metric/group `h…` (team ids season-scoped), Bound school GUID (stable), IHSAA slug, AthleticLIVE meet id (int), Athletic.net MeetID (`ani`), `a.i`/`a.ani`, `t.i`/`t.ani`; **no athlete id in Bound rosters** `[C 11][C 12]` |
| Pagination | association table single page; Bound team lists single page per sport-season; AL ES paged via `size`/`from` |
| Athlete fields | Bound roster `Name \| Year` (FR/SO/JR/SR, unlinked, no id) `[C 11]`; AL rows: name, school, grade `a.y` (numeric 9–12 track / FR–SR XC), `a.ani` — 136/136 XC rows and 6/6 track rows populated `[C 12]` |
| Meet fields | Bound comp id + date in title; AL meet doc (`n`,`son`,`ln`,`sdy`,`md`,`upts`) |
| Result fields | Bound: place, mark, `Final`/`Prelim` section, relay membership; AL: `r[].i`, `m`, `im` (ms for times, µm for distances), `w` wind, `hn` heat, `runm` round, `pt` points, splits, series reports `[C 12]` |
| Grade/class evidence | Bound roster Year; AL `a.y`; association class lists 4A/3A/2A/1A (IHSAA T&F 351 schools classed, IGHSAU T&F 346, IGHSAU XC 336) `[C 11]` |
| Coach/contact fields | Bound `/directory/new`: role + name (63 coach rows, 3 with email; 20 AD rows, 2 with email across the 5-school sample) `[C 29]`; the associations publish **no** coach/AD contacts `[C 11]` |
| Public API availability | none from the associations; AthleticLIVE ES + blob JSON are public and unauthenticated; Bound is HTML only |
| Static file availability | `ihsaa-static.s3.amazonaws.com` (2018–19 state T&F); IGHSAU PDFs (2026 state T&F `results.pdf`, 1.2 MB); IATC-hosted PDFs `[C 11]` |
| Browser requirement | none for Bound/AthleticLIVE; **required** for Athletic.net core `[C 04]` |
| Request cost | 1 (member list) + 2/school (slug + directory) ≈ 724 sequential for a full contact build `[C 29]`; AL ≈1 index page + 1 per event doc |
| Published rate limits | Bound `Crawl-Delay: 10` `[C 11]`; IHSAA robots disallows query strings `[V]` |
| Known blocks | Bound robots disallows `/profile/*` and `*/athletes/*` `[C 11]`; Athletic.net CF-403 `[C 04]`; `iahsaa.org` robots disallows `/*?*` `[V]` |
| Cross-source join keys | Athletic.net MeetID/TeamID/AthleteID from AL docs; school identity by name+city; Bound GUID ↔ IHSAA slug (1:1, text differs) `[C 29]` |
| Estimated marginal coverage | 365 (B-TF) / 365 (G-TF) / 365 (B-XC) / 369 (G-XC) Bound programs 2025-26 `[C 11]`; ~8–11k boys T&F athletes per season `[I, from 2 observed rosters]`; 1,265 grade-11 athletes at the 2026 state T&F meet (AL) `[C 12]` |
| Implementation recommendation | **PRIMARY** — Bound Iowa tenants for school/team/athlete/grade + results, seeded by the association member/classification lists. Secondary: RESULT_SOURCE = AthleticLIVE/Wayzata; COACH_SOURCE = school-district sites (association exposes none) |

**Measured pipeline counts (repo run 2026-09-21, `var/midwest-census/out/census-by-state.csv`):** IA —
schools 913, athletes 62,201, Co2027 14,076 (7,906 boys / 6,108 girls), with-coach 105, with-coach-email 21,
coaches 53 (4 emails). Grade evidence by source: `milesplit_ia` 8,738 athletes (`report.json.providers`).

**Open items.** 24 IHSAA members have no obvious Bound boys-T&F team `[C 11]`; XC rosters are result-driven
only; no athlete ids anywhere; AthleticLIVE→Athletic.net meet mapping unverified on this machine `[C 11]`.

## 4. Kansas (KS)

**Association / sources.** KSHSAA `https://www.kshsaa.org/` + public JSON API `https://kshsaa-api.kshsaa.org/`
· championship results `https://www.kshsaachamps.org/` · timers: Midwest Timing (Supabase + PDFs), DirectAthletics.

**Adapter status: implemented — `crates/midwest-census/src/sources/ks/`** (`collect.rs` entry `collect()`,
`wire.rs`, `parse.rs`, `tests.rs`). One request to
`https://kshsaa-api.kshsaa.org/directory/search/name/a/` yields ~526 records; each becomes a
`CanonicalSchool` (city, `classification`=Class, `enrollment`, website, association label `kshsaa`,
`SourceIdentity::AssociationSchool{association:"kshsaa"}` keyed by `Identifier`) **and** a `CanonicalCoach`
(AD role) with honorific stripping. **What it does NOT collect:** the `PublicClassifications` endpoint
(class/enrollment already arrive in the directory record), track/XC team or participation lists, athletes,
rosters, any result surface (`kshsaachamps`, Midwest Timing, DAT), assistant ADs, sport coaches (AD only),
and the kshsaachamps id space. Address/phone/principal fields are deliberately **not deserialized** `[V]`
(`ks/wire.rs` doc comment).

| §13 field | Value |
|---|---|
| Source name | KSHSAA directory + classification API; KSHSAA championship site `kshsaachamps.org`; Midwest Timing; DirectAthletics (KS slice) |
| Geographic coverage | Kansas only |
| Sports | B/G XC, B/G outdoor T&F; the championship site also carries indoor activity ids 67/68 `[C 23]` |
| Historical depth | `kshsaachamps` TF boys 2000–2026 in one request (20,887 rows), XC boys 2000–2025; champion history 1911 (TF) / 1956 (XC); DAT results index 42 pages × 100 rows; Midwest Timing meets back to 2007 `[C 23]` |
| Discovery mechanism | `PublicClassifications` (348 members, 6 classes) → `directory/search/name/a/` (full record incl. AD) → `playoff-sites` (state sites); `kshsaachamps.org/Shared/GetSchools|GetActivities` |
| Stable identifiers | `Identifier` (`KSS0001`) + numeric `Id` (458) — **same space as `schoolId` in PublicClassifications**; `USD`, `League`/`LeagueName`; `kshsaachamps` has a **separate** space (`SchoolID` 8/646, `ActivityYearClassID` 8283, `ActivityYearClassIndividualResultID` 187267); DAT team/athlete/meet ids `[C 23]` |
| Pagination | none for `directory/search/name/a/` (one response); DAT index paged 42×100 |
| Athlete fields | `kshsaachamps` rows: `Name` string only (no athlete id); DAT `/athletes/{sport}/{id}`; roster rows exist only for champion teams |
| Meet fields | KSHSAA state-site ids (TF 4135, XC 6A 4179); `ActivityYearID` 2309/2346; DAT meet ids; Midwest Timing `events.id` UUID |
| Result fields | `Statistic1` (mark), `PlaceFinish`, `Category` (round), `SchoolID`; grade present in XC rows (89 G11 of 143 graded rows, 2025 boys XC); **relay rows have an empty `Student`** `[C 23]` |
| Grade/class evidence | `classId`/`Class` (6A…1A) in the classification API; `kshsaachamps` XC grade column; no grade in the 2000–2026 TF history `[C 23]` |
| Coach/contact fields | AD name + email for ~526 schools in **1 request** (0 coach rows; `DateModified` ranges 2018-08-07 → 2026-07-08, useful staleness flag) `[C 23][C 29]` |
| Public API availability | **yes** — unauthenticated JSON API (directory, classifications, playoff sites); desktop User-Agent needed `[C 23]` |
| Static file availability | `kshsaachamps` HTML/JSON; Midwest Timing result PDFs (279 of 336 2026 events) `[C 23]` |
| Browser requirement | none |
| Request cost | 3 requests for the whole school layer (classifications + directory + state sites) `[C 23]`; 1 request for 2000–2026 TF boys results |
| Published rate limits | `www.kshsaa.org` `Crawl-delay: 30` `[V]`; `kshsaa-api` robots **404** (none published) `[V]` |
| Known blocks | `kansascoaches.com` unreachable `[C 23]` |
| Cross-source join keys | `Id` = `schoolId` (KSHSAA-internal); `kshsaachamps SchoolID` is **not** the same space; name+city to Athletic.net |
| Estimated marginal coverage | complete 348-school classified universe with class/enrollment/league/AD in 3 requests; grade evidence narrow (89 G11 XC rows 2025 + champion rosters) `[C 23]` |
| Implementation recommendation | **COACH_SOURCE** (AD-only, cheapest registry in the whole study; already implemented) — secondary VALIDATION_SOURCE (grade oracle at state) and DISCOVERY_SOURCE for the school universe |

**Measured pipeline counts:** KS — schools 836, athletes 33,628, Co2027 10,077, coaches 526 (523 emails),
Co2027-with-coach = 0 (the AD rows are not yet attached to athletes). Grade evidence: `milesplit_ks` 9,252.
Fresh classification capture: 348 schools, 36/36/36/64/64/112 by `classId` 6→1 `[V]`.

## 5. Minnesota (MN)

**Association / sources.** MSHSL `https://www.mshsl.org/` (Drupal 11: JSON:API + custom JSON endpoints)
· timers: Wayzata, Hero's, Fast Finish, GSE (partially absorbed) — four of six resolve to `*.anet.live`
· archive: `raceberryjam.com` (XC 1991+).

**Adapter status: implemented — `crates/midwest-census/src/sources/mshsl/`** (`collect.rs` + `collect/run.rs`,
`teams.rs`, `map.rs`, `parse.rs`, `text.rs`). Per school: `/schools` listing row → `/schools/<slug>` (facts +
AD/assistant-AD with Cloudflare-obfuscated email decoded locally) → JSON:API team view
`/jsonapi/views/teams/list_school` (filtered to track/XC activities) → `/api/coaches/<nid>` per team (levels
filtered; email kept only on the school's own domain). **What it does NOT collect:** the roster endpoint
`/api/team-data/roster/<schoolId>/<level>/<activityId>` (grade evidence), `/api/team-data/schedule|scores`,
any athlete entity, regular-season results, state-tournament archive PDFs, and the school `export/schools.csv`
(robots-disallowed extension) `[V]`.

| §13 field | Value |
|---|---|
| Source name | MSHSL school/team/coach surfaces; MN timers via AthleticLIVE; Raceberryjam archive |
| Geographic coverage | Minnesota only |
| Sports | B/G XC and B/G T&F (adapter filters team nodes to those activity ids; 124\|125\|150\|151 `[C 09]`) |
| Historical depth | in-season rosters per year; state archive PDFs 2017–2025 (T&F text-extractable `Yr` column); AthleticLIVE blobs (2025 state XC); Raceberryjam XC 1991+ `[C 09][C 10]` |
| Discovery mechanism | `/schools` Drupal view 50/page + A–Z filter → `/schools/<slug>` → team view → coach endpoint; sitemap gives 664 school URLs in one request `[V]`; AL meet index |
| Stable identifiers | `hostSchoolId` (611 Wayzata), team node `nid` (589034), `activityId` (124\|125\|150\|151), `yearId`, group id `/group/<id>/`; Athletic.net MeetID 666673 (2026 state T&F) from the schedule JSON; AL meet ids 74652 (2026 T&F) / 58505 (2025 XC) `[C 09][C 10]` |
| Pagination | `/schools?page=N`, 50 rows/page `[C 09]`; team view and coach endpoint unpaginated |
| Athlete fields | roster endpoint: `student_id` (32-hex), `student_name`, `student_grade` (`07\|08\|FR\|SO\|JR\|SR`) `[C 09]` — **not retained in any capture** (see `schema.json` note); AL rows carry name, school, grade, PR, `ani` |
| Meet fields | `/api/team-data/schedule` (meet name, date, Athletic.net id); AL meet doc; state archive page (meet, class, date) |
| Result fields | AL/RTDB rows (place, mark, wind, PR, team, grade, `ani`); state PDFs (`Name`, `Grade`, `Mark` for champions); MSHSL publishes **no** regular-season results `[C 09][C 10]` |
| Grade/class evidence | roster grade codes; state archive `Grade`/`Yr` columns; AL `Yr` — 242/242 Juniors at the 2025 state XC `[C 10]` |
| Coach/contact fields | AD + assistant AD (professional email decoded from Cloudflare obfuscation, school-domain only); per-team coach records with published level and email; work phone deliberately dropped `[V]` |
| Public API availability | **yes** — unauthenticated JSON:API + custom `/api/...` JSON; Cloudflare stands only in front of email obfuscation `[C 09]` |
| Static file availability | state PDFs (robots-disallowed extensions), sitemap XML `[V]`; AL blob JSON |
| Browser requirement | none for MSHSL; AL blobs none; Athletic.net core yes |
| Request cost | 14 listing pages (664 schools @50) + 1 school page + 1 team view + 1 coach call per team; store run measured 665 schools → 6,596 coach rows `[V]` |
| Published rate limits | none published `[V]` |
| Known blocks | robots disallows `/core/`, `/profiles/`, `/search*`, `/admin/`, `/user/*` and `*.pdf\|doc\|docx\|xls\|xlsx\|csv` `[V]`; MTEC API authenticated → rejected `[C 10]` |
| Cross-source join keys | `hostSchoolId`; school email domain; name+city → Athletic.net; Athletic.net MeetID from schedule JSON |
| Estimated marginal coverage | 664 schools `[V]`; coaches 6,596 (2,333 with email) `[V store]`; grade-bearing state XC results via 6 AL requests → 959 rows / 242 Juniors `[C 10]` |
| Implementation recommendation | **PRIMARY** (school + team + AD/coach layer; already implemented) — secondary RESULT_SOURCE via AthleticLIVE blobs, and Raceberryjam for pre-2017 history |

**Measured pipeline counts:** MN — schools 1,016, athletes 64,275, Co2027 14,420, with-coach 10,726,
with-coach-email 9,905, coaches 6,596 (2,333 emails). Grade evidence: `milesplit_mn` 11,261.

## 6. Missouri (MO)

**Association / sources.** MSHSAA `https://www.mshsaa.org/` · timer: PrimeTime Timing
(`data.pttiming.com` public bucket; `live.pttiming.com` ToU **prohibits** automated access `[C 22]`)
· official T&F results are delegated to Athletic.net (CF-403 here) `[C 21]`.

**Adapter status: none** (`grep -rn "mshsaa" crates/ src/` → no matches `[V]`). In the store run MO school
rows come only from `athleticlive_athletes` (850) and `milesplit_mo` (699); coaches 0.

| §13 field | Value |
|---|---|
| Source name | MSHSAA school listing / classifications / postseason results; PrimeTime Timing; Athletic.net (delegated) |
| Geographic coverage | Missouri only |
| Sports | B/G XC (`activity=5`), B/G outdoor T&F (`activity=19`); HS + JH levels; interior `alg` keys 11–14 (XC) and 52–55 (T&F) `[C 21]` |
| Historical depth | postseason pages 2001-02 → 2026-27 (2019-20 absent); schedules 2005-06 → 2026-27; individual champions 1926 → 2026 (6,163 rows); enrollment history to 2023-24 `[C 21]` |
| Discovery mechanism | `SchoolListing.aspx` (single page, 1,006 rows) → `MySchool/?s=<id>` / `Schools/Navigation.aspx?s=<id>`; `PostseasonResult.aspx?id=<groupId>`; `ClassAndDistrictAssignments.aspx?alg=` |
| Stable identifiers | integer school id (`s=`, 2…1,913, 47.4 % sparse; verified example `s=1567` = Academie Lafayette Charter HS `[V]`); `alg` sport keys; **no** athlete/meet/event/result ids; PDF names carry .NET ticks `[C 21]` |
| Pagination | none (single listing page) `[V]` |
| Athlete fields | state-championship PDF rows only: name, school, `Year` (9–12), mark, wind, heat, round, place; relay rows are team-level (no legs) `[C 21]` |
| Meet fields | PDF title + date range only; no MeetID |
| Result fields | mark + raw/converted time (`10.956 (10.956)`), `Wind` column, `Q`/`q` qualifier flags; champions-history rows carry Year/Class/Student/School/Mark only `[C 21]` |
| Grade/class evidence | PDF `Year` column (grade oracle); class/district pages; champions history has **no** grade `[C 21]` |
| Coach/contact fields | **none** — verified zero coach/AD fields on MSHSAA surfaces; MO is a contact-graph gap to be solved via district staff directories `[C 21][C 29]` |
| Public API availability | none (ASP.NET pages) |
| Static file availability | championship + enrollment + equipment-spec PDFs; PrimeTime `data.pttiming.com` bucket |
| Browser requirement | none for MSHSAA pages; yes for the delegated Athletic.net state page |
| Request cost | school universe = 1 page; state T&F/XC PDFs ≈20 requests per sport `[C 21]`; PrimeTime: 10 HTML files → 3,393 athlete-event rows `[C 22]` |
| Published rate limits | none published; **robots host-wide disallow** (§0.3) `[V]` |
| Known blocks | `MySchool/Matchup.aspx*` explicitly disallowed `[V]`; `live.pttiming.com` ToU; Athletic.net CF-403 `[C 21][C 22]` |
| Cross-source join keys | integer school id; name+city → Athletic.net; MSHSAA `School Website` field as the hop to district staff directories `[C 21]` |
| Estimated marginal coverage | 646 HS-level orgs (592 full members) with per-season sport sponsorship + class `[V]`; grade-bearing state PDFs ≈300–350 Co2027 per sport `[I, report 21]`; PrimeTime files for ~14 meets 2025 / ~9 2026 `[C 22]` |
| Implementation recommendation | **CONDITIONAL** — high-value school universe + grade oracle, but `mshsaa.org` is robots-disallowed for generic crawlers, so: manual/on-demand retrieval or a written permission request; otherwise substitute Athletic.net/MileSplit/PrimeTime. Do **not** build a scheduled association crawler. No adapter exists |

**Measured pipeline counts:** MO — schools 1,037, athletes 50,830, Co2027 15,168, coaches 0,
grade evidence `milesplit_mo` 13,218. Fresh listing capture: 1,006 rows (316 HS / 360 JH / 330 combined;
719 member / 257 affiliate / 30 hsa) `[V]`.

## 7. North Dakota (ND)

**Association / sources.** NDHSAA `https://ndhsaa.com/` (+ `https://ndhsaanow.com/`) · timer: Hero's Timing on
AthleticLIVE (`live.herostiming.com`, index `search.athletic.live/heros_meet_list`) · state results also on
`live.athletic.net/meets/<id>`.

**Adapter status: implemented — `crates/midwest-census/src/sources/plain_names/` (ND half:**
`nd.rs`, `nd_walk.rs`, `nd_coaches.rs`; shared `mod.rs`). One request for the 169-school index, then one
request per school for the staff block + `Sport/Activity Offering | Coaches` table. Sport-scoped rows carry
`CoachRole::Unknown` (NDHSAA never says which name is head coach); office roles (secretary, principal,
superintendent, trainer…) are dropped by an explicit token list; emails are **never** published → every row
has `professional_email == None`. **What it does NOT collect:** athletes, rosters, results, the co-op
Google Sheet, AthleticLIVE meet enumeration (ES) or result payloads (RTDB), NDHSAA tournament/champion
surfaces, and the pre-2025 state PDFs.

| §13 field | Value |
|---|---|
| Source name | NDHSAA school pages; AthleticLIVE (Hero's Timing) state series |
| Geographic coverage | North Dakota only (border co-ops carried as co-op labels) |
| Sports | B/G XC, B/G T&F; indoor state meets exist (Class A/B/JV, March) `[C 25]` |
| Historical depth | AL state series 2024 → 2026 (state XC 55421/261403; indoor 2026 61484/622458…); state outdoor track 2024 and earlier = CloudFront PDFs `[C 25]` |
| Discovery mechanism | `/schools` (169 anchors, one page) → `/schools/<id>/<slug>`; ES meet index by `ls` + `md` range `[V]`; RTDB per-event GETs |
| Stable identifiers | `/schools/<id>/<slug>` (ids 3…1378; 169 live); `ndhsaanow` team ids per sport+gender (Bismarck Century XC boys 469); AL meet id + event id + `rui` round key; **Athletic.net meet id (`ani`) pre-attached** in ES/RTDB docs; per-athlete `ani` (8-digit) / `anli` (7-digit) in RTDB (semantics `[I]` for athlete-level) `[C 25]` |
| Pagination | none for the school index |
| Athlete fields | RTDB standings rows: name, school, `y` = year in school (SR/JR/SO/FR), place, time, splits, `ani` `[C 25]` |
| Meet fields | ES doc (`i`, `ani`, `n`, `md`, `ls`, `o`); RTDB `meet_<id>` node; NDHSAA page links |
| Result fields | RTDB: place, mark, grade, team, splits; whole-meet node (meet 42224 = 2.9 MB) `[C 25]` |
| Grade/class evidence | per-athlete `y` in RTDB standings — the best ND Co2027 verifier; NDHSAA publishes no grade column `[C 25]` |
| Coach/contact fields | staff block (Superintendent, Principal, Athletic/Activities Director) + per-sport coach names; **0 emails**; 22/22 sampled schools had coach tables, ≥1 of the four XC/TF sports named in 20/22 `[C 25][C 29]` |
| Public API availability | **yes** — ES POST + RTDB GET + static JSON, no auth, no browser `[V]` |
| Static file availability | CloudFront PDFs (2024 and earlier state track) |
| Browser requirement | none for the state series (fully curl-able) |
| Request cost | 1 + 169 for the school/coach layer; 1 ES POST for meet enumeration; 1 RTDB GET per event `[C 25]` |
| Published rate limits | none (`robots.txt` = `Disallow:` only) `[V]` |
| Known blocks | the 2026 state **outdoor** T&F meet is not in the Hero's ES index (tenant unproven) `[C 25]`; 2026 state XC not yet indexed on 2026-09-19 `[C 25]` |
| Cross-source join keys | Athletic.net meet ids from the ES index (removes Athletic.net search entirely); school name; co-op labels |
| Estimated marginal coverage | 169 schools `[V]`; 6 ND 2026-season XC meets with AL+AN ids `[V]`; 2025 state XC = 4 races, Class A boys 185 entries `[C 25]` |
| Implementation recommendation | **COACH_SOURCE** (names only; already implemented) — secondary RESULT_SOURCE via AthleticLIVE RTDB for the state series |

**Measured pipeline counts:** ND — schools 339, athletes 15,138, Co2027 3,004, with-coach 1,317,
with-coach-email 0, coaches 922 (0 emails). Grade evidence: `milesplit_nd` 2,043.

## 8. Nebraska (NE)

**Association / sources.** NSAA `https://nsaahome.org/` (apex 403s curl; use `secure.nsaahome.org` forms and
the S3 mirror `https://nsaa-static.s3.amazonaws.com/textfile/**`) · timers: Black Squirrel Timing
(AthleticLIVE instance), Precision Race Results (`onlineraceresults.com`), Hy-Tek PDFs for state T&F.

**Adapter status: implemented — `crates/midwest-census/src/sources/plain_names/` (NE half:**
`nsaa.rs`, `nsaa_walk.rs`, `nsaa_coaches.rs`). One form GET for the 312-school `<option>` list, then one GET
per school (`?session=&school=<name>`) for the full record (superintendent, principal, AD(s), one row per
sport). Sport rows carry `CoachRole::HeadCoach` (NE publishes exactly one coach per sport); office roles are
dropped; **emails are absent by design**. The bulk "View all schools" form renders all 312 schools in one
1,085,584-byte body but took **49.4 s**, past the 45 s client timeout, so the per-school walk is used `[C 24]`.
**What it does NOT collect:** athletes, results (state XC `onlineraceresults` plain text, Hy-Tek state T&F
PDFs, district XC HTML with grade), the 28 district Athletic.net MeetIDs from the `/track-field/` page, XC/TF
classifications, and AthleticLIVE state-T&F payloads.

| §13 field | Value |
|---|---|
| Source name | NSAA directory + S3 `/textfile/**` mirror; Black Squirrel / Precision Race Results / onlineraceresults |
| Geographic coverage | Nebraska only |
| Sports | B/G XC, B/G T&F (+ middle level, no results surface) `[C 24]` |
| Historical depth | state XC onlineraceresults 2008–2025; state T&F Hy-Tek PDFs (complete files from 2012 onward as HTML/PDF, links back to the 1900s); XC classifications 1960→2025 boys / 1980→2025 girls `[C 24]` |
| Discovery mechanism | directory form `<option>` list (312) → per-school GET; `/track-field/` page carries 28 Athletic.net district meet links; S3 object listing by path pattern `[C 24]` |
| Stable identifiers | **no numeric school id** — the published school name *is* the key space; S3 object keys; Athletic.net district MeetIDs (28, e.g. A1 645702); AthleticLIVE state T&F meet ids (72610…); onlineraceresults `event_id`/`race_id` (2025 state XC `event_id=25737`) `[C 24]` |
| Pagination | none (option list single page) |
| Athlete fields | district XC HTML top-15 rows `Name (grade)` + mark; state XC plain text with 1m/2m splits, time, pace, grade; Hy-Tek state T&F PDF rows with attempt series + per-attempt wind `[C 24]` |
| Meet fields | district/state links, Athletic.net MeetIDs, S3 file names and `Last-Modified`/`ETag` |
| Result fields | grade printed next to **every** athlete in postseason results (NE's signature); place, mark, splits, wind, relay membership in T&F PDFs `[C 24]` |
| Grade/class evidence | `(11)`-style grade inline (`J'Shawn Afuh (11), 15:50.75`) `[C 24]`; classes A–D districts |
| Coach/contact fields | 1,217 coach name rows + 311 AD rows, **0 emails** `[C 29]`; sport rows name a single head coach |
| Public API availability | none (ASP.NET form); the S3 bucket is public and unauthenticated `[C 24]` |
| Static file availability | **yes** — full `/textfile/**` mirror with conditional-GET support `[C 24]` |
| Browser requirement | none for directory/S3; yes for `results.blacksquirreltiming.com` (AL SPA shell) `[C 24]` |
| Request cost | 1 form GET (plus 1 per school for details); 9 requests for full state XC (1 event page + 8 races) `[C 24]` |
| Published rate limits | `nsaahome.org` `Crawl-delay: 5` `[V]`; `secure.nsaahome.org/robots.txt` → 302 to apex `[V]` |
| Known blocks | apex `nsaahome.org` 403s plain curl `[C 24]` |
| Cross-source join keys | school name (only association key); Athletic.net district MeetIDs; name+date for other meets |
| Estimated marginal coverage | 312 schools with complete name-level staff rosters; grade-bearing postseason results for all qualifiers `[C 24]` |
| Implementation recommendation | **COACH_SOURCE** (names only; already implemented) — secondary VALIDATION_SOURCE (grade-bearing postseason files) and ATHLETIC.NET-SEED (28 district MeetIDs from one page) |

**Measured pipeline counts:** NE — schools 678, athletes 44,983, Co2027 9,200, with-coach 6,059,
with-coach-email 0, coaches 1,772 (0 emails). Grade evidence: `milesplit_ne` 5,921.

## 9. South Dakota (SD)

**Association / sources.** SDHSAA `https://sdhsaa.com/` · Bound association surface
`https://gobound.com/sd/associations/sdhsaa/{schools,memberships}` · state-meet yearbook PDFs · Athletic.net
(delegated official results; SDHSAA's XC "Results" link points at
`https://www.athletic.net/events/usa/south-dakota;sport=2`) `[C 26]`.

**Adapter status: none** (`grep -rn "sdhsaa" crates/ src/` → no matches `[V]`). In the store run SD school
rows come from `athleticlive_athletes` (365), `milesplit_sd` (195) and `coach_contacts_csv` (5); coaches 25.

| §13 field | Value |
|---|---|
| Source name | SDHSAA activity pages; Bound SD member directory + school staff pages; yearbook PDFs; Athletic.net divisions |
| Geographic coverage | South Dakota only (ND border programs compete as "unclassed": Fargo South, Grand Forks Central, Grand Forks Red River, Solen `[C 26]`) |
| Sports | B/G XC (fall), B/G outdoor T&F (spring); no indoor association program `[C 26]` |
| Historical depth | Bound seasons 2026-27 → 2021-22 (7); Athletic.net divisions TF 2009→2026, XC 2009→2026 (sparse); yearbooks cover the last completed season only (2025 XC = 79th boys; 2026 T&F = 121st boys) `[C 26]` |
| Discovery mechanism | SDHSAA activity page → Bound schools index (176) → alignment maps (team cards with Bound ids); region/state pages carry Athletic.net meet links — 10 XC region ids in one request `[V]` |
| Stable identifiers | Bound association `h20201103092322012199041ce958c44`, state `h2019…`, school `h2020…` + slug, team `/direct/teams/h…`; Athletic.net `SchoolID` == team id (`/team/6112/cross-country/2026`); 10 region MeetIDs (277461, 278813, 278815–278817, 278821, 278826, 278829, 278838, 278840 `[V]`), state XC 278807, state TF 646856/646859/646861; Bound calendar id 4931063 ↔ MeetID 278813 in the same row `[C 26]` |
| Pagination | none |
| Athlete fields | **none** on SDHSAA/Bound (rosters, stats and leaderboards empty; `/scores` `Result` column empty for the state T&F date `[C 26]`); yearbook PDFs give place, name, grade, school, mark for state-meet athletes |
| Meet fields | Bound calendar rows (+ Athletic.net MeetID), yearbook season/date |
| Result fields | yearbook rows (`Holdeman, Silas / 11 Mitchell Christian / 16:16.94 / 1`) `[C 26]`; regular-season marks exist only on Athletic.net |
| Grade/class evidence | yearbook grade column (boys XC 400 grade rows, 106 = grade 11) `[C 26]`; classes AA/A/B |
| Coach/contact fields | Bound `/sd/schools/<slug>/directory/new`: ~6 role rows/school (AD + 4 sport head coaches) with names; email **per-school opt-in** (Rapid City Stevens published 4; 4 of 5 sampled schools published 0) `[C 26][C 29]` |
| Public API availability | none; Athletic.net is the delegated results surface (CF-403 to curl) |
| Static file availability | yearbook + sanctioned-meet PDFs; the sanctioned-meet Google Sheet CSV is header-only `[C 26]` |
| Browser requirement | yes for Athletic.net; no for Bound/SDHSAA |
| Request cost | 2 requests for the universe (association index + 1/school directory ≈176); 1 per activity page for the Athletic.net meet links |
| Published rate limits | `sdhsaa.com` `Crawl-delay: 3`, disallow `/*?` `[V]`; Bound `Crawl-Delay: 10` `[C 11]` |
| Known blocks | Bound `/leaderboard` HTTP 202 empty body `[C 26]`; Athletic.net CF-403 |
| Cross-source join keys | Athletic.net MeetIDs from association pages; Bound school id ↔ Athletic.net SchoolID (153/176 = 86.9 % name match `[C 26]`) |
| Estimated marginal coverage | 176 schools, 155/155/158/157 sport teams for 2026-27, 207 Athletic.net school records, 10 region + 3 state MeetIDs; 937 grade-11 boys in the retained Athletic.net corpus `[C 26]` |
| Implementation recommendation | **CONDITIONAL** — build only the *seed + coach-name* slice (≈177 requests: 176 schools + 1 index) because it buys the Athletic.net meet/team ids and coach names cheaply; skip it as a result or athlete source (association surfaces carry no athlete-level data), and treat the email layer as opt-in |

**Measured pipeline counts:** SD — schools 427, athletes 22,812, Co2027 4,868, with-coach 283,
with-coach-email 79, coaches 25 (5 emails). Grade evidence: `milesplit_sd` 2,669.

## 10. Adapter inventory — plains slice

| ST | Module | Entry | Writes | Does NOT collect |
|---|---|---|---|---|
| KS | `crates/midwest-census/src/sources/ks/` | `collect()` (`ks/collect.rs`) → `kshsaa-api.kshsaa.org/directory/search/name/a/` | CanonicalSchool (city, class, enrollment, website, `kshsaa` identity) + CanonicalCoach (AD) | classifications endpoint, teams, participation, athletes, rosters, all results, assistant ADs, sport coaches |
| MN | `…/sources/mshsl/` | `collect()` (`mshsl/collect.rs`) → school page → team view → `/api/coaches/<nid>` | CanonicalSchool + AD/assistant-AD coaches + per-team coaches (level-filtered, domain-checked email) | rosters/grades, schedules, scores, athletes, results, archive PDFs, `export/schools.csv` |
| ND | `…/sources/plain_names/` (nd) | `collect()` (`plain_names/mod.rs`) → `ndhsaa.com/schools` + 1/school | CanonicalSchool + coach/AD rows (names only, role `Unknown` for sport rows) | athletes, results, ES meet enumeration, RTDB payloads, co-op sheet, state PDFs |
| NE | `…/sources/plain_names/` (nsaa) | `collect()` → `secure.nsaahome.org/nsaaforms/direxportscreen.php` + 1/school | CanonicalSchool + AD/head-coach rows (names only) | athletes, results, district Athletic.net ids, S3 mirror, classifications |
| IA | — | — | (store data from `athleticlive_athletes`, `milesplit_ia`, `coach_contacts_csv`) | everything association-specific |
| MO | — | — | (store data from `athleticlive_athletes`, `milesplit_mo`) | everything association-specific |
| SD | — | — | (store data from `athleticlive_athletes`, `milesplit_sd`, `coach_contacts_csv`) | everything association-specific |

Cross-check (store `schools.jsonl`, evidence-source ids per state): KS `ks` 526; MN `mshsl` 665;
ND `ndhsaa` 169; NE `nsaa` 312; IA/MO/SD have **no** association-typed evidence rows `[V]`.

Adjacent adapters that exist in the crate but cover **none** of these seven jurisdictions (so they are
not an implementation path for this slice): `…/sources/wiaa/` (WI - `schools.wiaawi.org`,
`www.wiaawi.org`), `…/sources/wiaa_results/`, `…/sources/ihsa/` (IL - `api.ihsa.org`),
`…/sources/ohsaa/` (OH - `officials.myohsaa.org`), `…/sources/wayzata/`, `…/sources/raceday/`
(`www.wiaawi.org`), plus the national adapters `…/sources/{athleticnet,athleticlive,athleticlive_athletes,milesplit,coach_contacts}/`.
Hosts listed here were read out of the adapter sources by this lane (`grep -rho "https\\?://[a-z0-9.-]*"`).

## 11. Cross-source join keys — plains slice

| Key | Producer | Consumers |
|---|---|---|
| Athletic.net MeetID (`ani`) | AthleticLIVE docs (ND `[V]`, IA `[C 12]`, MN `[C 10]`, NE state TF `[C 24]`), Bound calendar rows (SD), MSHSL schedule JSON (MN) | Athletic.net lane; meet reconciliation |
| Athletic.net TeamID/AthleteID (`t.ani`/`a.ani`) | AthleticLIVE result rows | profile/result resolution |
| Association school ids | KSHSAA `Id`/`Identifier`, MSHSL `hostSchoolId`, NDHSAA `/schools/<id>/`, MSHSAA `s=` | school reconciliation; AN seeding |
| Bound ids | school GUID/slug, season-scoped team `h…`, calendar/comp `h…` | IA/SD team and meet identity |
| School name + city | every association directory (NE has *only* this) | to Athletic.net/MileSplit when no id exists |

## 12. Open questions / unverified

1. **MO robots intent** — whether MSHSAA intends the trailing host-wide disallow or it is a paste error.
   Until answered, no automated MO association fetching (my one probe is disclosed in `samples/CAPTURES.md`).
2. **ND state outdoor T&F tenant** — `live.athletic.net/meets/73476` is confirmed, but the ES tenant/index
   serving its docs is not `heros` `[C 25]`; also unproven whether 2026 state XC (Oct 23-24) lands there.
3. **Athlete-level `ani` semantics** — treated as the Athletic.net athlete id in ND RTDB rows; unconfirmed
   against a known Athletic.net profile because Athletic.net 403s here `[C 25]`.
4. **MSHSL roster endpoint** — keys are documented (`student_id`, `student_name`, `student_grade`) but no
   captured roster body exists in this repo, so `schema.json` carries a null example for it.
5. **SD email opt-in rate** — two samples disagree (4 of 5 schools with zero emails vs Rapid City Stevens
   publishing 4); the true statewide rate needs a wider sample `[C 26]`.
6. **IA Bound slug mapping** — 1:1 but text-differing (`adm-adel`→`adm`); the full 379-school map has not
   been executed (60-slug sample only) `[C 29]`.
7. **NE apex 403** — the exact trigger (UA/rate/geo) was not isolated; the S3 mirror plus `secure.` host
   routes around it `[C 24]`.
8. **MO district/sectional results** — no association artifact exists, so the majority of MO participants
   have no MSHSAA result row; PrimeTime/DAT/Athletic.net coverage of those rounds is unquantified `[C 21]`.
