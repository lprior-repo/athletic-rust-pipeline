# 43. South Dakota gap follow-ups (co-op naming, SD result sources, school universe)

Status: complete
Observed on: 2026-09-20

Follow-up to report 26 (`26-south-dakota-sdhsaa.md`). Three deliverables: (1) how SDHSAA officially names
co-ops + a measured mapping rule for Athletic.net team labels that do not match the member directory;
(2) what actually carries South Dakota track/XC results, given that Bound has no SD T&F results;
(3) the official SD school-universe totals. All probing was anonymous public GET/POST with a desktop
Chrome UA. **No `*.athletic.net` request was sent** (hard constraint honored); Athletic.net *links* cited
below were read out of SDHSAA pages and of Dakota Timing's own `fallback_link` JSON field, never fetched.
Raw captures: `research/midwest/evidence/gaps/43/`.

---

### Source

**A. SDHSAA co-op machinery — three official artifacts, all fetched 2026-09-20.**

| Artifact | URL | HTTP | What it gives |
|---|---|---|---|
| Co-op conditions & guidelines (rev. 7/26) | `https://sdhsaa.com/Handbook/ATH-Cooperatives.pdf` | 200 | When a co-op is permitted; student-count/participation conditions; the approval process |
| Co-op agreement application | `https://sdhsaa.com/Forms/ATHACT-CooperativeAgreements.pdf` | 200 | The form itself; requires the **"Official Name of this Cooperative"** on every agreement |
| Registered co-op list (published Google Sheet) | `https://sdhsaa.com/athletics-cooperatives/` → `docs.google.com/spreadsheets/d/e/2PACX-1vQzhLyUVMSYBWNrsyIBiD7_S9fOa7u1MtNPltklTTEMvOUKaOyJOWiHQj9H5srNsn1Lh8pCMGi7o5Vl/pub?gid=379261016&single=true&output=csv` | 200 | **70 rows = 70 approved co-op agreements** (66 distinct primary schools); the authoritative official names |

**B. SDHSAA result surfaces.**
- Yearbook state-meet PDFs (linked from the activity pages, re-verified today):
  `/Yearbook/{B,G}-CrossCountry.pdf`, `/Yearbook/{B,G}-Track.pdf` — the only SDHSAA-published per-athlete
  result files, and the only official source with a **grade** column.
- T&F top-performance lists: `https://sdhsaa.com/track-field-top-performances/` → published Google Sheet
  `2PACX-1vTgdX89fGFJgcgnBUGVoxNFBjSDVgcCGph3q0YG3zB9MdCmlNSddqMTdjmEg_mFDVLEnTfmjZ0l0G_e`,
  **40 event tabs** (20 girls + 20 boys: 100/200/400/800/1600/3200, 100H-110H, 300H, 4x100/4x200/4x400/4x800,
  sprint medley, discus, HJ, javelin, LJ, PV, shot, TJ). Header "All times FAT".
- School universe: `https://sdhsaa.com/average-daily-memberships/` → Bound `.../sdhsaa/memberships` table;
  fact sheet `https://sdhsaa.com/General/ProtectThePurposeFactSheet.pdf`; member directory
  `https://www.gobound.com/sd/associations/sdhsaa/schools`.
- `https://sdhsaa.org/` is **not** a usable mirror: 200 but a 472-byte JS-challenge page
  (`window.location.replace('https://sdhsaa.org/?ch=1&js=<JWT>…')`). All SDHSAA PDFs live on `sdhsaa.com`.

**C. SD result hosts.**
- **AthleticLIVE tenant `dakota`** = **Dakota Timing** (Phase 2 Innovation, `dakotatiming.com`):
  - anonymous Elasticsearch: `POST https://search.athletic.live/dakota_meet_list/_search` (also
    `athlete_list`, `team_list`, `live_results` indices)
  - anonymous blob JSON: `https://athleticlive.blob.core.windows.net/<tenant>/…` (`meet_list`,
    `session_list`, `division_list`, `event_list`, `ind_res_list/_doc/<EventID>`, `athlete/<id>`)
  - tenant UIs: `dakota.anet.live` (short link host), `dakotatiming.com` → `live.dakotatiming.com`,
    results viewer `results.phase2in.com/display.html?event_id=<id>`
  - timer archive API: `dakotatiming.com` `addEvent` feed → **1,688 events** (`phase2in-events.json`)
- **MileSplit SD** `https://sd.milesplit.com/…` (teams, rankings, athlete profiles).
- **Bound** `https://gobound.com/sd/sdhsaa/…` (schedules only for TF — see Result evidence).

### Coverage

| Surface | Sports | Season depth | Volume measured 2026-09-20 |
|---|---|---|---|
| AthleticLIVE `dakota` (ES docs) | XC + outdoor TF + indoor TF | rolling, 2015→ | **622 SD meet docs** (`lsa:"South Dakota"`); 374 XC / 464 outdoor / 78 indoor (overlapping); 313 docs dated 2026, 279 dated 2025, **69 docs with `sdy` ≥ 2026-09-01** |
| Dakota Timing archive (`addEvent`) | XC + TF, HS + college | 2015–2026 | 1,688 events total; **1,120 events located ", SD"**; 138 distinct SD venues; 452 SD events expose a `results.phase2in.com` viewer link |
| SDHSAA yearbook PDFs | XC + TF, **state meet only** | current file per gender/sport | B-XC 379 name/grade rows; B-TF 341 rows (narrow parse); G-TF 1,035 rows (loose parse) |
| SDHSAA top-performance sheets | TF 20 events × 3 classes × 2 genders | current season only (blank off-season) | 40 tabs; every tab `#N/A` on 2026-09-20, "Last Updated: 6/03/2026" |
| MileSplit SD | XC + TF | rolling | 220 team rows (199 team links); rankings tables carry a `Grade` column |
| Bound | XC results, TF **schedules only** | 2025-26, 2026-27 | TF state-meet date page renders event rows with an **empty Result column** (no TF results) |

### Enumeration

**Schools.** Four official/derived counts, all measured today:

| Count | Source | Note |
|---|---|---|
| **176** | `gobound.com/sd/associations/sdhsaa/memberships` (176 data rows: School, Total ADM, Male Only ADM) | operational member roster; joins to Bound school slugs |
| **176** | `gobound.com/sd/associations/sdhsaa/schools` | member directory (528 school links incl. `/directory/new`) |
| **179** | SDHSAA fact sheet PDF (`/General/ProtectThePurposeFactSheet.pdf`, md5 `31605d869491f443701306f251717aff`) | "Year Founded 1905 · Member Schools 179 · Sports + Activities 29 · Participants 32,107"; the 3-school delta vs 176 is **unexplained** — [INFERENCE] fact sheet may count non-Bound activities such as co-op-only or fine-arts members |
| **207** | Athletic.net SD division school records (`anet-sd-xc-teams-xhrs.json`) | includes legacy, closed, private/club and 2 pseudo-teams |

ADM figures (same table, recomputed): Σ Total ADM **30,716.747**, Σ Male ADM **15,672.144**,
Σ Female ADM (derived) **15,044.603**; median school 69.8; max Harrisburg 1,374.9; min Elk Mountain 2.0.

**Teams.** Class-alignment participants (`coop-{xc,tf}-{b,g}.csv` tab captures; the same Class AA/A/B
per-gender structure is what `sdhsaa.com/alignment-maps/` publishes for the 2026-27 & 2027-28 cycle):
**B-XC 20+53+80 = 153**, **G-XC 152**, **B-TF 20+53+85 = 158**, **G-TF 157**. Co-op teams marked `^` in
those lists: **20 per sport** (7 in Class A, 13 in Class B), **21 distinct names** across the four lists;
the only sport differences are `Leola/Frederick Area` (XC only) and `Scotland/Menno` (TF only) — the other
19 appear in all four. All 21 `^` names exist in the 70-row registry (**21/21** cross-check).

**Meet enumeration (no Athletic.net).** `POST search.athletic.live/dakota_meet_list/_search`
`{"query":{"match_phrase":{"lsa":"South Dakota"}},"sort":[{"sdy":"desc"}]}` returns the SD meet list with
`i` (AthleticLIVE meet id), `ani` (Athletic.net MeetID), `n`, `sdy` (start date), `ls` (city, ST), `o`
(`xc|outdoor|indoor`), `hr` (results published), `ua` (doc update timestamp), `xcp[]` (event list with
event ids). Sample (2026 SD calendar, 12 rows): state XC `i=76061` 2026-10-24 Rapid City `ani=278807`,
Region 1A–5B championship meets, ESD Conference, plus two non-HS college meets (`NSIC Conference
Championships` `ani=null`, `DII Central Region Championships` `ani=279295`) that must be filtered out.

**Athlete enumeration with grade.** `POST .../athlete_list/_search` `{"query":{"term":{"mi":<meet i>}}}`:
- SD state T&F meet `mi=63837` → **2,339 roster rows**, of which **630 rows `y:"11"` (Grade 11)**.
- Clyde Cotton Invitational (XC) `mi=75742` → **1,079 rows**, **93 rows `y:"11"`**.
Rows carry `i` (AthleticLIVE athlete id), `n/fn/l` (name), `y` (grade token), `g` (gender), `mi`, `cco`,
`cm`, and `t{…}` (team object with its own `i`, `n`, `ab`, `mi`).

**Co-op resolution — the rule.** Applied to the **207 Athletic.net SD school records**, matching against the
union of (a) class-alignment names, (b) the 70-row registry (both the primary column and the
"schools listed if different than name" column), (c) the 176 member names:

| # | Rule step | Matched | What it resolves |
|---|---|---|---|
| 1 | NFKD-fold diacritics → lowercase → non-alnum → space; exact match | **161** | every current school and co-op name, incl. `Mahpíya Lúta` |
| 2 | member name minus mascot (`Ethan Public` → `Ethan Rustlers`) | 11 | AN drops mascots |
| 3 | token-subset (`Sch For The Blind` ⊂ `School for the Blind/VI`) | (in 16 below) | abbreviations |
| 4 | legacy component: distinctive token contained in ≥1 official name → shortest official name, alignment/registry preferred over member names | **16** | `Emery`→`Bridgewater-Emery`, `Geddes Community`→`Platte-Geddes`, `Hudson`→`Alcester-Hudson`, `Hurley Ind. Dist`→`Viborg-Hurley`, `Rutland`→`Oldham-Ramona-Rutland`, `Stickney`→`Corsica-Stickney`, `Tulare Independent Sch`→`Hitchcock-Tulare`, `Volin`→`Gayville-Volin`, `Wakonda`→`Irene-Wakonda`, `South Shore Independent`→`Waverly-South Shore`, `Marion - CLOSED`→`Parker/Marion`, `Freeman Academy/Marion - CLOSED`→`Freeman/Marion/Parker/Viborg-Hurley`, `Henry Public`→`Florence/Henry`, `Montrose Public`→`McCook Central/Montrose`, `Ethan Public`→`Ethan/Parkston`, `Sch For The Blind`→`School for the Blind/VI` |
| — | **resolved total** | **188 / 207 = 90.8 %** | |
| 5 | unresolved: 9 closed/merged + 8 non-member + 2 pseudo | 19 | see below |

**Official naming pattern (evidence: 70 registry rows, 0 exceptions).**
- Official co-op name = member school names joined by **`/`**, primary school first:
  `Bridgewater-Emery/Ethan`, `Freeman/Marion/Parker/Viborg-Hurley`, `Groton Area/Langford Area`,
  `De Smet/Iroquois/Lake Preston`, `Kimball/White Lake/Platte-Geddes`, `Cheyenne-Eagle
  Butte/Dupree/Tiospaye Topa`.
- A member that is itself a merged district or "Area" school keeps **`-`** inside its own name; the registry
  lists such a member either standalone or as a slash-joined participant:
  `Britton-Hecla` (+ `/Langford Area`), `Alcester-Hudson` (in `Beresford/Alcester-Hudson`),
  `Oldham-Ramona-Rutland` (in `Madison/Oldham-Ramona-Rutland`), `Corsica-Stickney` (in
  `Mount Vernon/Plankinton/Corsica-Stickney`), `Hitchcock-Tulare` (+ `/Doland` under Redfield),
  `Irene-Wakonda`, `Platte-Geddes` (+ `/Dakota Christian`), `Deubrook Area` (in `Deuel/Deubrook Area`),
  `Highmore-Harrold` (in `Miller/Highmore-Harrold`), `Tripp-Delmont` (in `Tripp-Delmont/Armour`).
- Therefore the discriminator is punctuation: **`/` between school names ⇒ co-op agreement;
  `-` inside a school name ⇒ merged district/Area school, not a co-op.** (Registry rows confirm:
  e.g. `Britton-Hecla` primary + `Britton-Hecla/Langford Area` agreement; `Custer` + `Custer/Elk Mountain`.)
- Registry row anatomy (col A = primary school, `^`-prefixed; col B = schools if different than name =
  the agreement's official name; col C = display `primary - agreement`). 4 primary schools hold **two**
  agreements each (`Groton Area` with Langford Area and with Doland; `Kingsbury County` with two different
  partner sets; `Miller/Highmore-Harrold`; `Parker/Marion`) — hence 70 rows / 66 primary schools.
- Sport-specific co-op membership is real but small: of the 21 `^` co-ops, only Leola/Frederick Area and
  Scotland/Menno differ between XC and TF; co-op membership is otherwise identical across the four sports.
- Residual label hygiene: `Waubay - Summit` (AN) = official `Waubay/Summit`; `Scotland/Menno ` (corpus)
  carries a trailing space. Both normalize away under step 1 once ` - ` is treated as a separator.

**The 19 unresolved AN records.**

| Category | Records |
|---|---|
| Closed/merged, absent from both the 176-member directory and the 70-row registry | `Bonesteel-Fairfax 26-5`, `Bowdle`, `Conde`, `Good Shepherd Lutheran (CLOSED 2022)`, `Grant-Deuel`, `Iroquois/Doland`, `Isabel`, `Midland`, `Roslyn` |
| Non-member (private/club/institutional; not in the member directory) | `Abiding Savior`, `Abiding Savior Academy`, `Black Hills Christian Academy`, `Black Hills Lutheran`, `Premier Running Club`, `South Dakota School for the Deaf`, `St Joseph's Indian`, `Lutheran of Souix Falls` [INFERENCE: transposed misspelling of member `Sioux Falls Lutheran`] |
| Athletic.net pseudo-teams (not schools) | `SDHSAA - Class A`, `SDHSAA - Class B` |
Successor mappings for the closed/merged group are **not** evidenced by an SDHSAA document in this pass —
drop them (do not guess; [INFERENCE] only: `Iroquois/Doland` is a dissolved co-op whose members now sit in
`Iroquois/Lake Preston` and `Groton Area/Doland`, per registry rows).

**Corpus level.** Of the **148 distinct SD team labels** attached to real SD athletes in the corpus,
**147 (99.3 %)** resolve by the same rule; the single residual is a club composite
(`Kampeska Track Club; Great Plains Lutheran`). **16 of the 148 labels are co-op teams** and all 16 match a
registry name verbatim. The premise "53 AN records use co-op names" resolves as: **22 of the 207 AN SD
school records *are* co-op teams** (20 distinct co-op names), **21 more are co-op member/primary schools**
(e.g. `Aberdeen Central`↔`Aberdeen Roncalli`, `Britton-Hecla`↔Langford Area, `Hitchcock-Tulare`↔Doland),
and the remaining 164 are standalone schools — the other ~31 of the "53" are legacy/closed/non-member
labels, not co-ops.

### Stable identifiers

| Entity | Field | Carrier | Example |
|---|---|---|---|
| Meet | `i` | ES `dakota_meet_list`, blob `meet_list` | `76061` (2026 SD state XC) |
| Athletic.net meet | `ani` | same doc | `278807` — **617 / 622 SD docs** pass `exists` (only **4** SD docs lack the field; all 2023) |
| Athletic.net meet URL | `fallback_link` / `results` (timer archive), `atr` (ES, mostly empty) | phase2in `addEvent` | `https://www.athletic.net/CrossCountry/meet/279104` (10 events carry one; 6 SD) |
| Event | `xcp[].i` (per meet), `ind_res_list/_doc/<EventID>` | blob | `2919434` |
| Athlete (AthleticLIVE) | `i` | `athlete_list`, blob `athlete/<id>`, inline in result rows | `51252281` |
| Athlete (Athletic.net) | `ani` inside blob athlete docs and inside each result row's `a{}` — **32/32 rows** in `ind_res_list` doc 2915796 | blob | `26623020` |
| Team (AthleticLIVE / Athletic.net) | `t.i` / `t.ani` | `athlete_list`, blob `team_list` | `1677188` / `47531` |
| Timer event | `id` + `results` viewer URL | phase2in `addEvent` | `2257` → `results.phase2in.com/display.html?event_id=2257` |
| MileSplit | team `23396` (Kadoka), athlete `15183191`, meet `774965` | sd.milesplit.com URLs | numeric ids in URL path |
| School identity | **no numeric id anywhere official** | SDHSAA registry (name only), ADM table (name + ADM), Bound slug (`/sd/schools/deubrookarea`) | use name + Bound slug |
| Class/region | class token in alignment lists (`Class AA/A/B`, `Region 1A…5B`) | alignments, region meet names | `Region 3A XC Championships` |

### Athletic.net leverage

This is the strongest SD finding: **the entire SD join to Athletic.net is present in the AthleticLIVE data.**
- Meet level: **617/622 (99.2 %)** of SD meet docs pass an `exists` filter on `ani`; a `must_not exists`
  query returns exactly **4** docs with no `ani` at all — `i=26769` Beresford XC Invite (2023-08-25),
  `i=26955` Mack Butler XC Invite (2023-08-26), `i=27280` SDSU XC Classic (2023-09-08), `i=28602` Summit
  League XC Championships (2023-10-28). So the gap is **older HS docs and college meets**, not a systematic
  hole; the 1-doc difference between the two framings (617 vs 618) is unexplained — [INFERENCE] a doc whose
  `ani` value is present but not indexed for `exists`.
- Athlete level: every result row embeds the athlete object with `ani` (AN athlete id) plus `y` (grade) and
  `t{…}` (team, itself carrying its own ids) — verified on all 32 rows of `ind_res_list` doc `2915796`
  and on the blob athlete docs (`athlete/51252289` → `ani=18812299`, `athlete/50945889` → `ani=19130937`).
- Team level: `team_list` docs carry `ani` (24 occurrences in the `mi=75742` team response).
- Request avoidance estimate: for the 2026 SD XC season alone, a full-season
  "discover meets → events → athletes → grades → results" pass costs **≈ (1 meet-list query) + (N meet
  docs, one per meet) + (N athlete-list queries)** on `search.athletic.live` and **zero** Athletic.net
  requests, versus the ≥1 profile/bio request per athlete the pipeline pays today (2,339 roster rows at
  the state TF meet alone). A 622-meet backfill is ~1,250 anonymous ES/blob calls, all cacheable with
  `ETag`/`304` (blob) and re-checkable by `ua`.

### Athlete evidence

| Field | Available | Evidence |
|---|---|---|
| name | yes (`n`, plus `fn`/`l`) | `athlete_list` rows; result-row `a.n` |
| graduating class / grade | **yes, `y`** (`"8"`…`"12"`) | 630 Grade-11 rows at `mi=63837`; grade distribution in `ind_res_list` rows `8,9,10,11,12` |
| school | yes (`t.n`, `t.ab`, `t.i`) | `{"i":1764998,"f":"Yankton","n":"Yankton","ab":"YANK"}` |
| city/state | meet level (`ls`, `lsa`); school level via SDHSAA/Bound | `ls:"Huron, SD"` |
| gender / category | yes (`g`: `Female`/`Male`; event `gl`) |  |
| TF/XC distinction | yes (`o` per meet, `xc:1` flag on XC athlete rows, event `ab`/`d`) |  |
| indoor / outdoor | yes (`o: indoor|outdoor`) | 78 indoor docs |
| performances | yes (`m` mark, `p` place, `pt` points, `gap`, `iv`, `mps`, `hl`, `hn`, `ro`) | `{"p":"1","m":"19:38.9"}` |
| PRs / progression | **partially** — `pr:true` PR flag per row; blob athlete docs carry richer history (`prs`/`ers` per report 12 method) | `pr` field present in rows |
| meets | yes (meet `i` + `ani` per row) |  |
| athlete profile URL | partially — tenant short links (`us: http://dakota.anet.live/y2rvyg`) and the timer results viewer (`results.phase2in.com/display.html?event_id=`); no per-athlete URL was verified. MileSplit per-athlete URL **is** verified: `sd.milesplit.com/athletes/15183191` shows `Class of 2028` free | tenant UI + MileSplit |

### Recruiting information

None of the five SD surfaces in this follow-up adds a coach directory: the co-op registry, alignment lists,
yearbook PDFs, top-performance sheets and the AthleticLIVE/timer archives carry **no** coach, AD or
athletics-website fields. Report 26 owns SD coach coverage (Bound school pages, ADM/member pages); this
report adds only negative evidence. Note for the privacy file: the AthleticLIVE meet doc carries a
registration/meet-entry contact (`qe`) which is a **personal email address** (redacted here) — it is a meet
director, not a role-published coach contact, and it must **not** be collected.

### Result evidence

| Field | SD availability | Source / field |
|---|---|---|
| ResultID | not exposed as a scalar; rows are addressed by (`a.i` athlete, `i` event, `ro` round, `hn` heat) | `ind_res_list` row |
| AthleteID | yes — AthleticLIVE `i` + Athletic.net `a.ani` | row `a.i`, `a.ani` |
| MeetID | yes — meet `i` + `ani` | doc + row `a.mi` |
| EventID | yes — `xcp[].i`, doc id of `ind_res_list` |  |
| mark | yes — `m` raw string (`"19:38.9"`, field marks as feet-inches strings) |  |
| normalized mark inputs | partially — event meta carries `ab` (`5000m`, `100m`), `d` (distance), `g`/`gl`, `ec`
(`Individual`/`Relay`), `eo` (`Varsity`) so marks can be normalized locally; **no** canonical numeric mark
field | event `_source` |
| timing method | yes, strongly: every SD meet is timed by Dakota Timing; `pmgmt:0`, `sm:true` on the meet doc;
SDHSAA yearbook top sheets state "All times FAT"; SDHSAA activity pages credit the timer | meet doc + `/track-field-top-performances/` |
| wind | field present (`w`, `" "` for distance races) — populated for sprints/jumps | row `w` |
| implement/hurdle specification | no dedicated field (event name only: `Discus`, `Shot Put`, `Javelin`, `110m Hurdles`) | event `n`/`ab` |
| heat / round | yes — `hn` heat, `ro` round, `hl` heat label, `er`/`es` eligibility flags | row |
| place | yes — `p` (`"1"`, `"--"` for DNF/DNS), `pt` points, `gap` | row |
| date | yes — meet `sdy`, per-event `d` |  |
| school represented | yes — `a.t.n`/`a.t.ab` (AN team id via `a.t.ani`) | row |
| relay membership | yes — `ec:"Relay"` events with 4 rows sharing the team; legs recoverable from the event doc
(`ind_res_list` + `event_list` relay legs per report 3/12 method) |  |
| grade of the athlete **in the result row** | **yes — `a.y`** (this is the Class-of-2027 oracle that
neither Bound nor SDHSAA's own TF sheets provide) | row `a.y` |

Grade oracle corroboration from official PDFs (per assignment: ≥2 PDFs tested for grade):
- `B-CrossCountry.pdf` (2025 boys state XC, 200, fetched fresh): 379 `name/grade/school/time` rows,
  grade histogram `11:102, 10:97, 12:82, 9:49, 8:37, 7:12`. Report 26 counted 400 rows / 106 Grade-11 on the
  same PDF with a looser line pattern — the delta is method, not data (both agree the grade column exists
  and Grade 11 is the modal-or-near-modal row count).
- `B-Track.pdf` (2026 boys state TF, 200, fetched fresh): 341 rows under a single-column regex,
  grades include **99 Grade-11 tokens**; the report-26 style per-event parse yields 1,299 grade-labelled
  rows / 392 Grade-11 (upper bound across rounds).
- `G-CrossCountry.pdf` / `G-Track.pdf`: parsed from the report-26 captures; the loose regex over-counts
  (it also catches place digits), so only the two boys PDFs above are offered as clean grade evidence.

**Bound check (assignment item 2).** `https://gobound.com/sd/sdhsaa/boystrackfield/2025-26/scores?date=2026-05-28&level=varsity`
returns 200 with 3 table rows (`SDHSAA State Track Meet`, `State Track` at Howard Wood Field, 10:00 AM CT)
and an **empty Result column** — Bound carries no SD T&F results, confirming report 26.

**SDHSAA top-performance sheets.** 40 tabs enumerated with gids; fetching two (`gid=1532689771` girls 100m,
`gid=335238573` boys 1600m) returned 288-byte CSVs whose only content is
`Last Updated: 6/03/2026` + `#N/A` per class — the lists are wiped/empty outside the Apr–Jun season.
They are a **current-season validation** surface (class columns `Class AA/A/B`, "All times FAT", no grade
column), not an archive.

### Incremental use

1. **New/changed SD meets** — one ES query sorted by `sdy` desc with `match_phrase lsa:"South Dakota"`;
   detect change by `ua` (doc update timestamp) and `hr`/`xcp` becoming non-empty. Verified pattern: 12 SD
   XC meets sorted by `sdy` returned in one 3.8 KB response; all rows carried `ua` (`2026-09-09T19:19:xx`).
2. **New meetings from the timer side** — one POST to the phase2in `addEvent` feed returns all 1,688 events
   (488 KB); diff on `id`, `date`, `results`. Cheap, complete, and independent of AthleticLIVE.
3. **Results for a meet** — blob `ind_res_list/_doc/<EventID>` per event, conditional GET with
   `ETag`/`Last-Modified` (304 supported, per report 12/28 method); or `live_results` ES index during meets.
4. **Roster + grade refresh** — `athlete_list` by `mi`, re-pulled only for meets whose `ua`/`hr` changed.
5. **Do not** re-pull the yearbook PDFs or top-performance sheets weekly: they change once per state meet
   (PDFs) and once per season (sheets). Poll the SDHSAA activity pages (or their `Last-Modified`) monthly.
6. Filter traps to encode in the adapter: `i=55481` "XC Sample Meet" (dummy doc in the tenant), the
   college meets (`NSIC`, `DII Central Region`), and cancelled/test titles; and `ani:null` docs.

### Access characteristics

| Surface | Class | Notes |
|---|---|---|
| `sdhsaa.com` pages/PDFs | normal HTML + downloadable PDF | NitroPack/Elementor; no auth; 200s today; `sdhsaa.org` is a JS-challenge dead end |
| SDHSAA published Google Sheets | static CSV (`output=csv`) | 200; 288-byte content when the season lists are empty |
| `search.athletic.live` ES | public structured JSON (anonymous POST) | 200; text-field aggregations on `o` are rejected (400) — use filter aggs / `term` queries |
| `athleticlive.blob.core.windows.net` | public structured JSON | 200; `ETag`/`Last-Modified` + 304; container listing disabled |
| `dakotatiming.com` + phase2in archive | public JSON/HTML | 200; archive = one POST, 1,688 events |
| `sd.milesplit.com` | normal HTML w/ paywall markers | 200; profile "Class of 20XX" free, full result lists locked (report 32) |
| `gobound.com` | normal HTML/browser app | 200; SD TF = schedules only |
| `*.athletic.net` | **not contacted** | constraint honored throughout |

### Recommendation

- **RESULT-SOURCE (primary): AthleticLIVE tenant `dakota`.** It is the only SD surface that gives
  per-athlete results **with a grade field** and AN ids for meet/athlete/team, anonymously, for both XC and
  TF (indoor and outdoor), with `ani` on ≥99.2 % of SD meets. Recommend adopting it as the SD result feed
  and as the SD grade oracle; expected marginal coverage: **all** SDHSAA-member XC/TF meets timed by Dakota
  Timing (622 SD docs today, 1,120 archive events since 2015), i.e. effectively the whole SD high-school
  season plus region/state championships.
- **VALIDATION: SDHSAA yearbook PDFs** (grade + place for every state-meet athlete, official) and
  **SDHSAA top-performance sheets** (in-season class lists, FAT). Use to audit the AthleticLIVE rows, not
  as a feed.
- **DISCOVERY-ONLY: Dakota Timing `addEvent` archive** (meet calendar/venue inventory incl. cancelled and
  pre-registration meets) and **MileSplit SD** (supplementary athlete/progression view).
- **REJECT for results: Bound** (no SD T&F results), and **REJECT `sdhsaa.org`** (JS challenge).
- **School identity**: adopt the 176-row ADM/membership table + the 70-row co-op registry as the SD school
  and co-op authority; use the rule in *Enumeration* to canonicalize the 207 AN labels (188 resolved,
  19 to drop or map manually). Flag the fact-sheet 179 vs Bound 176 discrepancy in the data-quality log.

### Evidence appendix

| URL | method | HTTP | what it proved | timestamp (local) |
|---|---|---|---|---|
| `https://sdhsaa.com/athletics-cooperatives/` | GET curl (browser UA) | 200 (484,140 B) | Page links `ATH-Cooperatives.pdf`, `ATHACT-CooperativeAgreements.pdf`, the published co-op Google Sheet (`gid=379261016`), Bound `/schools` `/memberships` | 2026-09-20 09:05:17 |
| `https://sdhsaa.com/average-daily-memberships/` | GET curl | 200 (488,304 B) | ADM page carries no inline table; ADM lives in Bound memberships | 2026-09-20 09:05:19 |
| `POST https://search.athletic.live/dakota_meet_list/_search` (states query) | POST curl | 200 | `dakota` tenant reachable anonymously; SD docs present | 2026-09-20 09:05:22 |
| `https://docs.google.com/spreadsheets/d/e/2PACX-1vQzhLy…/pub?gid=379261016&single=true&output=csv` | GET curl | 200 | **70 co-op agreements**; cols = primary school (`^`), agreement name, display; 4 primaries with 2 agreements | 2026-09-20 09:05:37 |
| `https://sdhsaa.com/General/ProtectThePurposeFactSheet.pdf` | GET curl | 200 (590,045 B, md5 `31605d869491f443701306f251717aff`) | 179 member schools, 29 sports+activities, 32,107 participants, founded 1905 | 2026-09-20 09:05:51 (re-verified 09:2x) |
| `https://sdhsaa.com/Handbook/ATH-Cooperatives.pdf` | GET curl | 200 | Co-op conditions/guidelines (rev. 7/26): when a co-op is allowed | 2026-09-20 09:11:27 |
| `https://sdhsaa.com/Forms/ATHACT-CooperativeAgreements.pdf` | GET curl | 200 | Agreement form; "Official Name of this Cooperative" is a required field | 2026-09-20 09:11:28 |
| Alignment tab captures `coop-{xc,tf}-{b,g}.csv` (BXC/GXC/BTF/GTF) | gviz CSV export | 200 each | Class counts 153/152/158/157; 20 `^` co-ops per sport; 21 distinct co-op names, **21/21 present in the registry** (capture URL not retained — evidence cross-checked against the live registry) | 2026-09-20 09:06:07–09:06:11 |
| `POST .../dakota_meet_list/_search` (filter aggs `o`, `sdy` ranges, `lsa`) | POST curl | 200 | 622 SD meets; 374 XC / 464 outdoor / 78 indoor; 313 in 2026; 279 in 2025 | 2026-09-20 09:05:22–09:06:47 |
| `POST .../dakota_meet_list/_search` (`match_phrase lsa:"South Dakota"`, aggs `exists ani`, `exists atr`, `hr`) | POST curl | 200 | **617/622 SD docs have `ani`**; 496 `hr=true`, 126 `hr=false` | 2026-09-20 09:2x |
| `POST .../dakota_meet_list/_search` (`_source` sample, sort `sdy` desc, `o:xc`) | POST curl | 200 (3,843 B) | 12-row SD XC calendar with `i`/`ani`/`ua`/`md`/`xcp`; state XC `76061` `ani=278807`; `xcp` empty for future meets | 2026-09-20 09:2x |
| `POST .../dakota_meet_list/_search` (agg terms on `o`) | POST curl | **400** | text-field aggregation rejected (`illegal_argument_exception`) — use filter aggs | 2026-09-20 09:2x |
| `POST .../dakota_meet_list/_search` (`must_not exists ani`, `_source` 10) | POST curl | 200 (864 B) | Exactly **4** SD docs lack `ani`: Beresford XC Invite / Mack Butler XC Invite / SDSU XC Classic / Summit League XC, all 2023 | 2026-09-20 09:2x |
| `POST .../dakota_meet_list/_search` (`sdy >= 2026-09-01`, `track_total_hits`) | POST curl | 200 | **69** SD meet docs scheduled/dated on/after 2026-09-01 (current XC season in flight) | 2026-09-20 09:2x |
| `POST .../athlete_list/_search` `{"term":{"mi":63837}}` / `+ y:"11"` | POST curl | 200 | SD state TF roster **2,339** rows, **630** Grade-11 | 2026-09-20 09:10:13/09:10:14 |
| `POST .../athlete_list/_search` `{"term":{"mi":75742}}` / `+ y:"11"` | POST curl | 200 | Clyde Cotton XC roster **1,079** rows, **93** Grade-11 | 2026-09-20 09:07:26/09:07:27 |
| `POST .../dakota_meet_list/_search` (full doc, `mi=75742`) | POST curl | 200 | 104-field doc: `xcp[]` event ids, `ani=278121`, `hr`, `ua`, `us`, `pb`, plus `qe` personal email (excluded from collection) | 2026-09-20 09:07:17 |
| `athleticlive.blob.core.windows.net/.../ind_res_list/_doc/2915796` | GET curl | 200 (38,874 B) | 32 result rows; each row embeds `a{ i,n,fn,l,y,g,mi,cm,ani,t{…} }` — **32/32 rows carry `a.ani`**; event meta `ab/d/ec/eo/g/gl/n` | 2026-09-20 09:08:15 |
| `.../athlete/51252289` and `.../athlete/50945889` | GET curl | 200 | Athlete blob docs carry AN athlete ids (`ani=18812299`, `ani=19130937`) | 2026-09-20 09:07:32 / 09:08:05 |
| `.../meet_list/75742`, `.../session_list/75742`, `.../division_list/75742`, `.../event_list/75742` | GET curl | 200 | Per-meet blob JSON family; event/session/division ids for time-sliced scraping | 2026-09-20 09:07:07–09:07:10 |
| `POST .../team_list/_search` (`mi=75742`) | POST curl | 200 (11,994 B) | Team rows carry AN team ids (`ani`, first 47531) | 2026-09-20 09:07:38 |
| `https://dakotatiming.com/…` (home + archive JS + `addEvent`) | GET curl + JS parse | 200 | Timer identity (Phase 2 Innovation); archive **1,688 events**, **1,120 located ", SD"**, 138 SD venues, 452 SD `results.phase2in.com` viewer links, 10 AN `fallback_link`s | 2026-09-20 09:09:08–09:09:32 |
| `https://sd.milesplit.com/…` (results, teams, rankings, one athlete profile) | GET curl | 200 | 220 SD team rows / 199 team links; rankings carry a `Grade` column; profile shows `Class of 2028`; full result lists locked | 2026-09-20 09:08:43–09:09:08 |
| `https://sdhsaa.com/Yearbook/B-CrossCountry.pdf`, `B-Track.pdf` | GET curl | 200 each | Grade column present: B-XC 379 rows (102 G11), B-TF 341 rows (99 G11 tokens) | 2026-09-20 09:11:24/09:11:25 |
| `https://sdhsaa.com/activity/cross-country/`, `/activity/track-field/` | GET curl | 200 (509,945 / 510,483 B) | Official yearbook links + "Cross Country Alignments"→athletic.net, "Track & Field Alignments"→gobound; no live-results/timing links | 2026-09-20 09:2x |
| `https://sdhsaa.com/alignment-maps/` | GET curl | 200 (515,580 B) | Official 2026-27 & 2027-28 class maps for CC and TF (AA/A/B per gender); no embedded school data, no sheet links | 2026-09-20 09:2x |
| `https://sdhsaa.com/classifications/`, `/alignments/`, `/archived-alignments/` | GET curl | **404** | Those routes do not exist | 2026-09-20 09:2x |
| `https://sdhsaa.org/` | GET curl | 200 (472 B) | JS challenge page (`window.location.replace(...?ch=1&js=<JWT>)`) — not usable | 2026-09-20 09:2x |
| `https://sdhsaa.com/track-field-top-performances/` + 2 tab CSVs (`gid=1532689771`, `gid=335238573`) | GET curl | 200 / 200 (288 B each) | 40 event tabs enumerated; lists are `#N/A` off-season, "Last Updated: 6/03/2026", class columns, "All times FAT", no grade | 2026-09-20 09:06:30 / 09:11:57 + 09:2x |
| `https://gobound.com/sd/sdhsaa/boystrackfield/2025-26/scores?date=2026-05-28&level=varsity` | GET curl | 200 (54,149 B) | State-meet date page lists 2 events with **empty Result column** → Bound has no SD T&F results | 2026-09-20 09:2x |
| `https://www.gobound.com/sd/associations/sdhsaa/memberships` | GET curl | 200 (177,313 B) | **176 rows** (School, Total ADM, Male Only ADM); ΣTotal 30,716.747, ΣMale 15,672.144 | 2026-09-20 09:2x |
| `https://www.gobound.com/sd/associations/sdhsaa/schools` | GET curl | 200 (315,115 B) | 528 school links (`/sd/schools/<slug>`); table now JS-rendered | 2026-09-20 09:2x |
| `https://docs.google.com/spreadsheets/d/e/2PACX-1vQzhLy…/pubhtml` | GET curl | 200 | Single-tab published view (no tab menu exposed) | 2026-09-20 09:2x |
| `~/Downloads/midwest-tfxc-source-research/tools/26-scratch/anet-sd-xc-teams-xhrs.json` | local parse | n/a | **207** AN SD school records extracted; rule applied → 161 exact + 11 mascot + 16 legacy = **188 resolved** | 2026-09-20 09:1x |
| `tools/26-scratch/sd-corpus-teams.json` (148 labels) + `sd-schools.json` (176 members) | local parse | n/a | 147/148 (99.3 %) corpus labels resolve; 16 are co-op teams, all verbatim in the registry | 2026-09-20 09:1x |
| local captures `phase2in-events.json`, `blob-*`, `es-*`, `ms-sd-*` | local parse | n/a | All counts and field lists quoted above | 2026-09-20 09:05–09:2x |

Raw captures for this report: `research/midwest/evidence/gaps/43/` (108 files), including
`sd-an-team-mapping.csv` (207 rows: AN name → rule → official SDHSAA name), `sdhsaa-athletics-coops.csv`
(70 registry rows), `sdhsaa-membership-adm-2026-09-20.csv` (176 schools), `sd-an-team-mapping-final.json`,
`sd-coop-annotations.json`.
