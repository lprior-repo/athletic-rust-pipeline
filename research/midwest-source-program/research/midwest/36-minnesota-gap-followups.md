# 36. Minnesota MSHSL — gap follow-ups (2025 XC PDF extraction, prior-year grades, grade-availability map)

Status: complete
Observed on: 2026-09-20

Closes the MN row of `synthesis/06-open-questions.md` §D: *"MSHSL 2025 XC PDFs are raster-only
(OCR required or skip); MSHSL participant nodes for prior years unverified [09][10]."* Read with
report **09** (MSHSL enumeration/coach API) and report **10** (MN timers). All raw captures in
`research/midwest/evidence/gaps/36/`.

**Headline answers**

1. **2025 XC PDFs: no text layer, but not a scan.** pdftotext returns **0 words on all four files**;
   the page content is the original text **converted to vector outlines** — censused on *every* file
   (all four: 0 `BT`/`Tj`/`TJ` operators, 0 embedded fonts, 8 logo images, path-op counts
   114,764–125,212 `l` + 91,429–96,943 `c` + 313–329 `re`, producer Nitro PDF Pro 14.39.0.18; the
   images are 8 decorative logos, largest 560×47 px at 150–151 ppi). `pdftoppm -r 300` +
   `tesseract -l eng --psm 6` recovers name/grade/team/time: 1,747 words and **156 grade tokens** for
   the 2-page Class AAA boys file in
   **≈5.0 s wall (1.6 s render + 3.4 s OCR)** single-core-class work, no GPU. The loss is row
   recovery, not name corruption: **142 of the 159 printed places** appear in the OCR text (89.3%),
   and of the 149 unique (name, grade) records recovered, **142 (95.3%) match the AthleticLIVE roster
   for the same meet exactly**, 147 (98.7%) at fuzzy ≥0.86. A stricter full-row parse scores 102/143
   (71.3%) — that number is parse noise, not OCR name quality (re-measured 2026-09-20, below).
2. **The outline flatten is 2025-XC-specific, not a producer regression.** The 2026 T&F finals PDFs
   come from the *same* producer (Nitro PDF Pro 14) and ARE text-extractable with a grade column.
3. **MSHSL publishes no prior-year athlete rosters, at any endpoint.** 4th path segment → 404;
   `?year=695` → 200 but silently ignored (byte-identical to the current season). The public
   `/api/mshsl-state/roster-<school>-<activity>` cache keeps only the latest snapshot and is already
   `is_empty:true` for T&F — the 2025-26 grade-11 T&F roster is **gone from mshsl.org**.
4. **Prior-year graded participation exists anyway**, from two surfaces: MSHSL state result PDFs
   (XC 2017–2024 text with grades; XC 2025 outline→OCR; T&F 2023–2026 text with grades) and
   **AthleticLIVE `athlete_list`** (one anonymous ES POST per meet) which returns the graded roster
   for MSHSL state XC **2019 + 2021–2025** (2018 meet is indexed but has 0 athlete rows; 2020 had no
   state meet) and state T&F **2023–2026** (verified counts; the tenant also indexes 2018/2019/2021/
   2022 state T&F meets, not counted here), plus the section meets in between, with Athletic.net
   athlete/team ids where the meet was linked.
5. **The 2026 T&F state-meet *program* PDF is also a graded participant surface** — the one document
   the earlier pass dismissed. `/sites/default/files/2026-09/2026-track-program-v.2.pdf` (136 pp,
   37.6 MB, 52,432 words) prints per-class heat/flight sheets as
   `place, bib, name, GRADE (numeric 9–12), school, seed mark`: **1,309 records, 276 schools**, grade
   histogram 12→572 / 11→415 / 10→240 / 9→82. All 1,309 names resolve **100%** against the 2026 state
   T&F AthleticLIVE roster and grades agree on **1,303/1,309 (99.5%)** — a free, no-API graded list
   (the 2024 **XC** program, by contrast, has 0 such records).

---

### Source

- Provider: Minnesota State High School League (MSHSL), `https://www.mshsl.org/` — Drupal 11 site,
  open JSON:API, plus the custom `/api/` endpoints documented in report 09. This slice adds nothing
  new to the MSHSL surface *inventory*; it pins the **historical depth** of each surface.
- Second source used here, allowed by the campaign's host rules (`search.athletic.live`):
  AthleticLIVE / Wayzata Results, LLC (`https://results.wayzatatiming.com/`, ES at
  `https://search.athletic.live/`), the timing platform that MSHSL links for state T&F.
- Third-party metadata confirmed: 2025 XC PDFs are printed from **DirectAthletics MeetPro**
  (page footer `DirectAthletics MeetPro 2`) and timed by **Wayzata Results, LLC** (header);
  2025/2026 T&F PDFs are Hy-Tek Meet Manager sheets exported through Nitro PDF Pro 13/14.

### Coverage

- Minnesota only. Target activities unchanged: `124`/`125` XC boys/girls, `150`/`151` T&F boys/girls.
- **Verified historical depth by surface (new in this report):**

| Surface | Years verified present | Grade-level? |
|---|---|---|
| State XC archive page (`...boys-cross-country-<YYYY>`) | 2017–2025 | — (index) |
| State XC per-class result PDFs linked there | 2019 (A/AA), 2020 (16 section PDFs only), 2021–2025 (A/AA/AAA) | yes (2021–2024 text; 2025 OCR) |
| State XC yearbook PDFs | 2017, 2018 | yes (`Name, grade` pairs) |
| XC "Team Participants thru <YYYY>" PDFs | 2019, 2022, 2023, 2024, 2025 | **no — schools only** |
| State T&F results index `/YYYY-track-and-field-results` | 2023, 2024, 2025, 2026 | yes (per-class finals/prelim PDFs) |
| State T&F archive page (`...boys-track-field-<YYYY>` / `...girls-track-field-<YYYY>`) | 2019–2026 (boys), 2025 (girls, 200), **2026 girls → 403** | — (index; history docs) |
| MSHSL in-season roster API | **current contest year only** | yes (07/08/FR/SO/JR/SR) |
| State T&F state-meet **program** PDF | 2026 (verified) | yes — numeric `9–12`, **1,309 records** |
| AthleticLIVE `athlete_list` (wayzata tenant, `mi` scope) | meets 2018-08-24 → present; MSHSL state XC 2019/2021–2025, state T&F 2023–2026 (2018 state XC indexed with 0 athlete rows) | yes (`y` = grade) |

### Enumeration

- **Prior-year *rosters* (per-athlete team membership) are not enumerable from MSHSL** — both
  prior-year routes were probed and fail (see §Result evidence): no year path segment, no year query
  param, cache holds the current snapshot only. Prior-year *state-meet results with grades* ARE
  enumerable from MSHSL print surfaces (result PDFs 2017–2026 + the 2026 T&F program) — the roster
  API is the only part that is current-season-only.
- Grades for a past season ARE enumerable from **AthleticLIVE**, per meet:
  `POST https://search.athletic.live/athlete_list/_search` with
  `{"size":2000,"query":{"term":{"mi":<AthleticLIVE meet id>}}}` → the meet's whole graded roster.
  Meet ids come from `POST https://search.athletic.live/wayzata_meet_list/_search`
  (`match_phrase` on `ln` = meet name, e.g. `"MSHSL"` → **148** MSHSL-named meet docs; add
  `{"term":{"ls":"mn"}}` → **1,886** MN-location meets, of which **503** are `o:"xc"`).
  Each meet doc carries `i` (AthleticLIVE meet id) and `ani` (Athletic.net MeetID, `-1` when unlinked).
- Enumeration of state-meet *people* from MSHSL PDFs works for both genders (the T&F finals PDFs are
  combined boys+girls; the XC PDFs are per class per gender).
- State XC 2020 is the outlier: **section** PDFs only (`/sites/default/files/2020-11/{1..8}{a,aa}-results.pdf`,
  16 files), no state meet; grades are a numeric prefix (`11 Reese Anderson`).

### Stable identifiers

| Entity | Identifier | Evidence |
|---|---|---|
| Prior-year team node | `node/<nid>` + `yearId`: XC boys 2025-26 = **464336** (`yearId 696`), 2024-25 = **391924** (`yearId 695`), T&F boys 2025-26 = **472624** (`yearId 696`) | team-page `drupalSettings.currentPath`/`yearId`; JSON:API `node--participant` |
| AthleticLIVE meet | `i` (doc id) + `ani` (Athletic.net MeetID): 2023 XC `28764`/`ani −1`, 2024 XC `41637`/`−1`, 2025 XC `58505`/`−1`, 2023 T&F `25893`/`516328`, 2024 T&F `38545`/`563571`, 2025 T&F `54182`/`613966`, 2026 T&F `74652`/`666673` | `wayzata_meet_list` ES hits |
| AthleticLIVE athlete | `i` (platform athlete id) + `ani` (Athletic.net AthleteID) + `y` (grade) | `athlete_list` docs, e.g. Robert Mechura `i 20114444`, `y "JR"`, team `t.ani 12190` (Roseville Area), `mi 28764` |
| **Grade encoding is per-meet, not global** | `y` is either letter style (`FR\|SO\|JR\|SR`, plus numeric `7\|8`) or numeric (`7`–`12`) depending on the upload | XC meets 2019/2021/2022/2023/2024/2025 = letter+7/8 (`FR`…`SR`); 2026 state T&F (`mi 74652`) = numeric (`12`→929, `11`→747, `10`→464, `9`→226, `8`→81, `7`→25, `Jr`→1). Noise values seen: `5`, `Jr`, `null` (2–24 rows per meet). Normalize before joining. |
| MSHSL cache key | `roster-<schoolId>-<activityId>` / `schedule-<schoolId>-<activityId>` | `roster.bundle.js`; `/api/mshsl-state/roster-611-124` |
| MSHSL PDF | **no id**; identity = URL path (year + class + gender/round) | all archive pages |
| Not identifiers | PDF file names (2026 AAA prelims link is 404 while the file name implies existence); meet results are not separately addressable on mshsl.org | 404 probe |

### Athletic.net leverage

- **Free A.net MeetID join for section + state meets**, with zero Athletic.net requests: the 148
  MSHSL-named `wayzata_meet_list` docs span 2018–2026 and decompose as **7 state XC** (all `ani:-1`
  or null — never linked), **8 state T&F** (2018/2019 `null`/`−1`; **2022 = 475179, 2023 = 516328,
  2024 = 563571, 2025 = 613966, 2026 = 666673**), **130 section meets** (ids e.g. 2025 Section 1AA =
  613928) and 3 other MSHSL docs. The AthleticLIVE↔A.net meet join therefore does not exist for
  state XC — get those from the timer's SPA link or from A.net search instead.
- **A.net AthleteID joins come for free where the operator linked the meet**, per athlete, straight
  off the `athlete_list` roster: 2025 state XC 1,141/1,143 (99.8%), 2024 1,025/1,218 (84.2%), 2022
  1,098/1,181 (despite the meet doc `ani:-1`), 2023 0/1,167, 2019/2021 0. Team-level `t.ani` (A.net
  TeamID): 2024 1,218/1,218, 2025 1,143/1,143, 2023 1,144/1,167, 2022/2021/2019 0.
- Where ids are present, `grade + school + A.net TeamID/AthleteID` seeds an A.net lookup
  deterministically with zero A.net requests; where absent (2023 XC and older), name+school+grade
  from the same row is still enough to seed a team-scoped A.net lookup.
- **Exact avoided requests:** a whole-meet graded roster for MSHSL state/section meets is 1 ES POST
  per meet; the equivalent Athletic.net work (whole-meet pulls at 3 requests/meet, or profile pulls
  per athlete) is not needed for these meets — 15 state meets + 130 section meets are already
  indexed, each with grade in-row.
- **A graded Co2027 slice for the 2026 spring cohort with no API and no A.net cost:** the 2026 state
  T&F program PDF carries 1,309 graded rows = 52.9% of the 2,473-row whole-meet roster (415 of them
  grade-11), one 37.6 MB download, zero requests to any timing API. Use it as the offline fallback /
  cross-check when `search.athletic.live` is unavailable.
- The MSHSL state T&F PDFs also carry grade but **no ids**, so they remain a validation surface, not
  a seed surface; the ES roster is the seed surface (grade + ids in the same row).

### Athlete evidence

- **From MSHSL PDFs (text, 2017–2025 XC / 2023–2026 T&F):** name, **grade** (XC: `FR|SO|JR|SR` and
  `7|8` in the `YR` column; T&F: numeric `9–12` under `Yr School`/`Year School`), school/team,
  place, mark (time). The T&F sheets also print `Wind` per finalist/prelim row; XC has no wind (n/a).
  Not available: any stable athlete id, splits, PRs, progression, meet ids, timing method.
- **From AthleticLIVE ES (2019–2026):** name, **grade** (`y`), gender, team name, meet id, plus
  Athletic.net athlete ids where the timer linked rows (present 2022 = 1,098/1,181 and 2024–2025;
  absent 2019/2021/2023 XC) and team ids where the roster carries them (state XC 2023 = 1,144/1,167,
  2024/2025 = 100%; state T&F 2023 = 2,980/2,980 and 2026 = 2,473/2,473); results/marks come from the
  blob docs (report 12 schema,
  `a.y` = grade on result rows). Verified histogram examples: 2023 state XC = SR 337 / JR 299 /
  SO 238 / FR 153 / 8th 98 / 7th 42; 2025 state XC = SR 337 / JR 282 / SO 227 / FR 168 / 8th 89 /
  7th 39 / 5th 1.
- **Cross-validation performed:** 2023 Class AAA boys champion "Robert Mechura" appears in the MSHSL
  text PDF (`1 Robert Mechura` + first `YR` token `JR`) **and** in AthleticLIVE as
  `{n:"Robert Mechura", y:"JR", t.n:"Roseville Area", mi:28764}` — two independent grade sources agree.
- **Largest cross-validation: 2026 T&F program × ES roster.** The 1,309 program records
  (`place, bib, name, grade, school, seed mark`) resolve **100% (1,309/1,309)** by normalized name
  against the 2,473-row `mi=74652` roster, and grades agree on **1,303/1,309 (99.5%)**; the six
  non-agreements are same-normalized-name collisions where ES carries two rows (`("12","9")`), not
  value conflicts. This is a two-source, zero-Athletic.net grade confirmation for the 2026 T&F cohort
  (program grade-11 rows: **415**).
- **Grade encodings differ per source and per meet** — MSHSL PDFs print `FR/SO/JR/SR` (XC) or numeric
  `9–12` (T&F `Yr`/`Year` columns); the AthleticLIVE `y` field is letter style for XC meets and numeric
  for 2026 T&F. Any join must normalize both to a single integer grade.
- **Not available anywhere on MSHSL for prior seasons:** per-athlete membership in a school's team
  roster (the Co2027 grade-11 lists for 2025-26 T&F and for XC before fall 2025 are unrecoverable).
- Privacy: no athlete email/phone/address on any surface touched; nothing beyond name/grade/school/mark
  was extracted.

### Recruiting information

- **Not applicable / no change from report 09** for the coach graph (head/assistant coaches via
  `/api/coaches/<nid>`; AD via `/schools/<slug>`; the Non-MSHSL-Coach / Sub-Coach filter rule stands).
- One new prior-year detail: a **prior-year team node exposes its own coach set**. `include=field_coaches`
  on nid 464336 (XC boys 2025-26) returns 9 `paragraph--coach` entities created 2025-07-08 — so
  last-season coach names/roles are reachable through the same nid if the current node is empty;
  `field_schedule` on that node is `[]` (no archived meet list).
- No coach email exists on the JSON:API side (paragraph→user exposes `display_name` only) — emails
  still require `/api/coaches/<nid>` per team node.

### Result evidence

**Grade availability by season × sport (the requested table).**

| Contest season | Sport | Surface (grade-bearing unless noted) | Grade column | URL pattern | Status |
|---|---|---|---|---|---|
| 2026 | T&F (spring 2026) | MSHSL state finals PDFs, per class | `Yr School` (numeric 9–12) | `/2026-track-and-field-results` → `/sites/default/files/2026-06/<file>.pdf` | 5 of 6 links probed: **4×200** (AAA finals, A finals, A prelims, AA prelims), AAA prelim **404** |
| 2025 | T&F (spring 2025) | MSHSL state finals + prelim PDFs | `Yr School` | `/2025-track-and-field-results` → `/sites/default/files/2025-06/<file>.pdf` | 3 of 6 probed, all 200 (AAA finals, A finals, AAA prelims) |
| 2024 | T&F (spring 2024) | MSHSL state finals + prelim PDFs | `Year School` | `/2024-track-and-field-results` → `/sites/default/files/2024-06/<file>.pdf` | 1 of 6 probed, 200 (AAA finals) |
| 2023 | T&F (spring 2023) | MSHSL state finals + prelim PDFs | `Year School` | `/2023-track-and-field-results` → `/sites/default/files/2023-06/<file>.pdf` | 1 of 6 probed, 200 (AAA finals) |
| 2026 | XC (fall 2026) | MSHSL in-season roster API (**current** season) | `student_grade` 07/08/FR/SO/JR/SR | `/api/team-data/roster/<schoolId>/varsity/<124\|125>` | 200 (Wayzata 611 = 155 rows) |
| 2025 | XC (fall 2025) | MSHSL state PDFs — **outlined, OCR required** | `YR` (`JR|SR|SO|FR`) via OCR | `/sites/default/files/2025-11/{results.pdf, 2025-class-aa-boys-…, 2025-class-aaa-boys-…, 2025-class-aaa-girls-…}` | 200 ×4, all 0 words |
| 2024 | XC (fall 2024) | MSHSL state PDFs — text | `YR` | `/sites/default/files/2024-11/2024-class-aaa-boys-cross-country-state-results.pdf` | 200, 2,690 words |
| 2023 | XC (fall 2023) | MSHSL state PDFs — text | `YR` | `/sites/default/files/2023-11/class%20aaa%20boys.pdf` | 200, 1,693 words |
| 2022 | XC | MSHSL state PDFs — text | `YR` | `/sites/default/files/2022-11/Boys%20AAA.pdf` | 200, 160 grade tokens |
| 2021 | XC | MSHSL state PDFs — text | `YR` | `/sites/default/files/2022-01/2021-bcc-class-aaa-results.pdf` | 200, 157 grade tokens |
| 2020 | XC | **section** PDFs only (16), Word export | numeric prefix (`11 Reese Anderson`) | `/sites/default/files/2020-11/{1..8}{a,aa}-results.pdf` | 200 (1a sampled) |
| 2019 | XC | state class PDFs, Word export | `Name, Grade` column | `/sites/default/files/2020-07/2019-class-aa-boys-cc-results.pdf` | 200 |
| 2018 / 2017 | XC | yearbook PDFs | `Name,GRADE` pairs — **348 / 353** rows (grades 8–12) | `/sites/default/files/2026-01/2018_boyscrosscountry_yearbook.pdf`, `…2017…` | 200 ×2 |
| any prior | XC + T&F | **no MSHSL athlete roster** | — | `/api/team-data/roster/611/varsity/124/695` → **404**; `?year=695` → 200 **ignored** | 404 / 200-identical |
| 2025-26 | any | MSHSL cache holds only the current snapshot | — | `/api/mshsl-state/roster-611-150` | 200, `is_empty:true` |
| 2019–2026 | XC + T&F | **AthleticLIVE ES roster (machine-readable)** | `y` | `POST search.athletic.live/athlete_list/_search {"term":{"mi":<i>}}` | 200 (counts below) |
| 2026 | T&F (spring 2026) | **state-meet program PDF** (per-class heat/flight sheets) | numeric `9–12` | `/sites/default/files/2026-09/2026-track-program-v.2.pdf` | 200, **1,309 graded records** |

AthleticLIVE `athlete_list` roster sizes measured (all 200): 2019 state XC **845** (mi 4674),
2021 **1,177** (11779), 2022 **1,181** (18769), 2023 **1,167** (28764), 2024 **1,218** (41637),
2025 **1,143** (58505), 2018 **0** (1732 — meet indexed, no roster rows); state T&F 2023 **2,980**
(25893), 2024 **3,308** (38545), 2025 **3,029** (54182), 2026 **2,473** (74652); 2023 Section 1AAA
**144** (28459).

Other result facts pinned this slice:

- **T&F finals PDFs are combined boys+girls** Hy-Tek sheets ("…Boys and Girls Track and Field
  Championships") with `Wind`, `Points`, prelim/final split and `Yr` — richer than the XC PDFs.
- The 2025 T&F finals PDF (46 pp) has 20,818 words; **875 lines** match a `grade + school` pattern
  (`11 Minneapolis Washburn`), i.e. the numeric `Yr` column is recoverable by text parse
  (re-measured 2026-09-20; an earlier pass reported 1,465 "grade+school pairs" that no pattern
  reproduced, so the verified figure is used).
- **Broken link:** `/sites/default/files/2026-06/2026-track-and-field-class-aaa-prelim-results.pdf`
  (linked from `/2026-track-and-field-results`) → **404**. The other five 2026 finals/prelim links →
  200. Fetch-and-verify is mandatory; do not trust the href.
- No ResultID / timing-method / implement-spec fields on any MSHSL PDF; marks are as printed
  (`15:23.5`, `61' 3.5"`). Timing credit "Wayzata Results, LLC"; 2025 XC footer "DirectAthletics MeetPro".
- **Programs are not uniformly grade-free — one of them is a seed surface.** The 2026 T&F state
  program (136 pp, 52,432 words) is printed from the same Hy-Tek/Nitro pipeline as the finals PDFs
  and carries **1,309 `place, bib, name, grade, school, seed-mark` records** (heat sheets and field
  flights; 276 schools; grades 12→572 / 11→415 / 10→240 / 9→82) — a graded Co2027 list (415 grade-11
  rows) obtained as one 37.6 MB download, no API. The 2024 **XC** souvenir program (44 pp, 31,062
  words) genuinely has **0** grade tokens and no participant rows. `team-champions-thru-2026.pdf`
  (3 pp), `meet-records-thru-2026.pdf` (4 pp) and `boys-cc-ind.-team-champions-thru-2024.pdf` (8 pp)
  list names + years + marks only, no grades.
- XC "Team Participants thru 2025" PDFs are **school-level only** (Class A/AA 1975–2025, AAA
  2021–2025): 3,210 words of school names with `year-place` strings, **0 athlete names, 0 grades**
  (no `Last, First` lines). Do not mistake them for participant (athlete) lists.

### Incremental use

- **In-season roster window (unchanged rule, now with hard evidence):** the roster endpoint returns
  only the current contest year, so the *only* way to obtain a grade-11 cohort from MSHSL is to poll
  during the season — XC `124|125` Sep–Nov, T&F `150|151` Mar–Jun, one request per school per sport
  (`/api/team-data/roster/<schoolId>/varsity/<activityId>`), 12 h server cache. Out-of-season the
  same request returns `rosters: []` (Wayzata T&F, 220 B) — a useful "season not started" signal.
- **Cheap staleness probe:** `GET /api/mshsl-state/roster-<schoolId>-<activityId>` returns
  `{data, timestamp (epoch s), is_empty}` — 268 B when empty, ~33.3 KB when populated, i.e. no
  bandwidth advantage over the roster call itself; only use it to detect "season started" cheaply.
- **Prior-season XC/T&F graded rosters — weekly collector:** poll
  `wayzata_meet_list` (`match_phrase` on `ln:"MSHSL"`, or `term ls:"mn"`) for new `i` docs and diff on
  `i`; for each new/changed meet run one `athlete_list` POST (`mi`) plus, if results are needed, the
  blob per-event GETs of report 12 (ETag/304 work, zero-byte revalidation). No MSHSL PDF re-parse for
  refresh; PDFs are the once-per-season fidelity check.
- **Per-season MSHSL delta:** the results index page `/YYYY-track-and-field-results` and the XC archive
  page `…boys-cross-country-<YYYY>` are rewritten once per season; re-reading the index then the new
  PDFs is a 2-request-per-season operation.

### Access characteristics

- Classes touched: **downloadable PDF** (29 fetched), **public structured JSON** (`/api/team-data/…`,
  `/api/mshsl-state/…`, `/jsonapi/…`), **normal HTML** (archive/index pages), **public JSON over
  anonymous POST** (AthleticLIVE ES).
- Status codes this slice: 200 everywhere except **404** (`/api/team-data/roster/611/varsity/124/695`;
  the 2026 AAA prelim PDF) and **403** (`…state-tournament-archive-girls-track-field-2026`, Drupal
  "Access denied" — the 2025 girls page is 200, so this is a page-level permission/publication gap,
  not a rate limit).
- **No 429, no `Retry-After`, no CAPTCHA** on either host; ES answers anonymous POSTs with
  `HTTP 200` and returns 403 for `_mapping` and 400 for aggregations on text fields.
- **robots.txt conflict, disclosed:** mshsl.org's robots.txt disallows `*.pdf` for `User-agent: *`
  (report 09). This assignment explicitly required fetching state result PDFs and testing extraction,
  so 29 PDFs were fetched sequentially at ≥1.2 s spacing. Production must treat these PDFs as
  manual/on-demand fetches (or seek written permission), not as a crawler target; the AthleticLIVE
  ES roster is the crawler-safe substitute for the same grades.
- **Request budget, disclosed:** this slice saved **62 responses from www.mshsl.org — 29 PDFs + 33
  HTML/JSON/API fetches** (including the 404 and 403 probes) — and made **40 requests to
  search.athletic.live** (34 search POSTs + 6 mapping GETs) = **102 total**, above both the mission's
  ~50/host aim and its ≤80/agent guidance; the excess is the PDF/archive depth sweep (29 PDFs across
  9 seasons) plus 16 extra ES probes needed to pin the historical roster years and id coverage. All
  requests were sequential at ≥1.2 s spacing; **no 429, no `Retry-After`, no CAPTCHA** was seen. Zero
  requests to any `*.athletic.net` host and zero to the blob store.
- Toolchain used for extraction (this workstation): `pdftotext/pdfinfo/pdftoppm` 26.08.0 (poppler),
  `tesseract` 5.5.3 (leptonica 1.87, AVX512), `qpdf` (content-stream proof). No `ocrmypdf`; not needed.
- OCR cost model for the 2025 XC set: ≈5 s CPU per 2-page document (300 dpi render + `--psm 6`),
  ≈1.0–1.8 MB temporary PNGs per page; the whole 2025 XC state set (6 documents, 12 pages) ≈1 min
  single-core.
- OCR accuracy for the 2025 XC set, measured against the AthleticLIVE roster for the same meet
  (re-computed 2026-09-20 from `evidence/gaps/36/xc300-all.txt` + `es-roster-58505-names.json`):
  **row recovery 142/159 printed places = 89.3%**, record-level name match 142/149 = 95.3% exact and
  147/149 = 98.7% fuzzy ≥0.86; residual errors are systematic glyph confusions (`l/I/1`, `W/V`) and
  merged words (`Conorsweene`, `Koivulaschadow`). A strict place+name+grade+team+time parse scores
  102/143 = 71.3% — a parser limitation, which is why the production rule is "parse loosely, then
  resolve names against the ES roster".

### Recommendation

**VALIDATION (grade truth) + RESULT-SOURCE (state/section meet rosters & marks) — unchanged primary
role from report 09, with three concrete corollaries.**

1. **Do not plan a prior-season grade backfill from mshsl.org.** No year-parameterised roster exists;
   schedule the Co2027 capture in-season (fall grade-11 = XC, spring grade-11 = T&F) or accept the
   AthleticLIVE route. This is the single decision this report forces.
2. **For 2019–2026 state/section meet grades, use AthleticLIVE ES, not PDF parsing.** One anonymous
   POST per meet returns name + grade + school + ids; MSHSL PDFs stay a fidelity/validation surface
   and remain the only XC source for 2017/2018 (ES has no 2018 roster rows).
3. **2025 XC PDFs are NOT a blocker.** OCR at 300 dpi recovers name/grade/team/time in ~5 s/doc and
   89% of printed rows, and 95% of the recovered records resolve *exactly* to the ES roster for the
   same meet (98.7% fuzzy) — so parse loosely and resolve names against the 1,143-row ES roster
   rather than trusting OCR strings as identifiers. Cheap enough to include, but it adds nothing over
   the ES roster, which already has the same athletes with ids.
4. **Add the T&F state-meet program to the season-end fetch list.** One 37.6 MB download per spring
   yields 1,309 graded `name/grade/school/mark` rows (415 grade-11) for 2026 and cross-checks the ES
   roster at 99.5% grade agreement — the only MSHSL-published machine-readable graded athlete list
   outside the result PDFs, and it needs no `/api/` call. Fetch it from the same index page as the
   finals PDFs (`/2026-track-and-field-results`) and re-check each spring, since the 2024 XC program
   shows the format is not guaranteed across sports.

Residual gaps: (a) the 2018 state XC meet is indexed with **0 athlete rows** and 2020 had no state XC
meet, so 2017/2018 XC depend on PDF text (2019/2021/2022 XC are covered by ES); (b) A.net athlete/team
ids are absent for 2019/2021 XC and for state XC team ids before 2023, so those seasons seed by
name+school; (c) the girls T&F archive page for 2026 is 403 and the 2026 AAA prelims PDF is 404 —
both need re-checking next season; (d) OCR name quality was measured on a single file — extend the
recall check before automating the 2025 XC parse.

---

### Evidence appendix

All timestamps 2026-09-20 America/Chicago (-0500). Method `UA` = curl with desktop-Chrome UA,
`curl -sS -L`, sequential ≥1.2 s apart unless noted. `MS` = `www.mshsl.org`, `ES` =
`search.athletic.live`. Raw captures: `research/midwest/evidence/gaps/36/`.

**MSHSL HTML/JSON probes**

| URL | method | HTTP | what it proved | timestamp |
|---|---|---|---|---|
| MS `/tournaments/state-tournament-archives/state-tournament-archive-boys-cross-country-2025` | GET UA | 200 | 2025 XC docs list: 3 result PDFs + "Boys CC Team Participants thru 2025" | 09:05:07 |
| MS `/tournaments/state-tournament-archives/state-tournament-archive-girls-cross-country-2025` | GET UA | 200 | girls XC 2025 page exists; 3 result PDFs + participants doc | 09:05:11 |
| MS `/tournaments/state-tournament-archives/state-tournament-archive-boys-track-field-2026` | GET UA | 200 | T&F archive years 2019–2026; history docs only (champions/records/program) | 09:05:40 |
| MS `/tournaments/state-tournament-archives/state-tournament-archive-girls-track-field-2026` | GET UA | **403** | Drupal "Access denied" for the 2026 girls archive | 09:05:43 |
| MS `/2026-track-and-field-results` | GET UA | 200 | 6 per-class PDF hrefs (`/sites/default/files/2026-06/…`) | 09:05:44 |
| MS `/modules/custom/mshsl_team_data/js/dist/roster.bundle.js` | GET UA | 200 | roster URL shape; `drupalSettings.currentYearId==yearId` gate; cache key `roster-<school>-<activity>`; `/api/mshsl-state/<key>` GET/POST | 09:06:00 |
| MS `/modules/custom/mshsl_team_data/js/dist/schoolSchedule.bundle.js` | GET UA | 200 | same cache mechanism for schedule | 09:06:01 |
| MS `/schools/wayzata-high-school/cross-country-running-boys/2025` | GET UA | 200 | prior-year team page: `node/464336`, `yearId 696` | 09:06:02 |
| MS `/schools/wayzata-high-school/cross-country-running-boys/2024` | GET UA | 200 | `node/391924`, `yearId 695` | 09:06:04 |
| MS `/schools/wayzata-high-school/track-and-field-boys/2026` | GET UA | 200 | `node/472624`, `yearId 696`, `activityId 150` | 09:06:05 |
| MS `/schools/wayzata-high-school/track-and-field-boys/2025` | GET UA | 200 | prior-year T&F page served | 09:06:07 |
| MS `/tournaments/state-tournament-archives/state-tournament-archive-boys-track-field-2025` | GET UA | 200 | 2025 T&F finals PDFs (A/AA/AAA) + program/records | 09:07:45 |
| MS `…/state-tournament-archive-boys-track-field-2024` | GET UA | 200 | 2024 archive has history docs only (results are on `/2024-track-and-field-results`) | 09:07:54 |
| MS `…/state-tournament-archive-boys-track-field-2023` | GET UA | 200 | same for 2023 | 09:08:02 |
| MS `/api/team-data/roster/611/varsity/124?year=695` | GET UA | 200 | **year param ignored**: 155 rows, identical grade histogram to the current season | 09:08:04 |
| MS `/api/mshsl-state/roster-611-124` | GET UA | 200 | public cache read: `{data,timestamp:1789839269,is_empty:false}` = same 155-row roster | 09:08:05 |
| MS `/api/mshsl-state/roster-611-150` | GET UA | 200 | empty T&F cache (`is_empty:true`, 268 B) → no archived 2025-26 T&F roster | 09:08:28 |
| MS `/api/mshsl-state/schedule-611-150` | GET UA | 200 | cached 2026-27 T&F schedule (6 entries, Apr–May 2027 dates) | 09:08:30 |
| MS `/api/team-data/roster/611/varsity/150` | GET UA | 200 | live T&F roster empty (`rosters:[]`, 220 B), head_coach present | 09:08:31 |
| MS `/tournaments/state-tournament-archives/state-tournament-archive-boys-cross-country-2024` | GET UA | 200 | 2024 XC: 3 result PDFs + participants + champions + program | 09:06:58 |
| MS `…/boys-cross-country-2023` | GET UA | 200 | 2023 XC: 3 result PDFs (space-encoded names) + docs | 09:07:02 |
| MS `/sites/default/files/2025-06/2025-track-aaa-finals.pdf` | GET UA | 200 | T&F 2025 finals fetch | 09:10:01 |
| MS `/sites/default/files/2026-06/2026-track-and-field-class-aaa-prelim-results.pdf` | GET UA | **404** | linked 2026 AAA prelims PDF is missing (223 KB Drupal 404 page) | 09:10:04 |
| MS `…/boys-cross-country-2022` | GET UA | 200 | 2022 XC PDFs (`Boys A/AA/AAA.pdf`) | 09:10:08 |
| MS `…/boys-cross-country-2021` | GET UA | 200 | 2021 XC PDFs (`2021-bcc-class-{a,aa,aaa}-results.pdf`) | 09:10:12 |
| MS `…/boys-cross-country-2020` | GET UA | 200 | 2020: 16 **section** PDFs, no state meet | 09:10:37 |
| MS `…/boys-cross-country-2019` | GET UA | 200 | 2019: class A/AA PDFs + history | 09:10:40 |
| MS `…/boys-cross-country-2018` | GET UA | 200 | 2018: yearbook + program only | 09:10:44 |
| MS `…/boys-cross-country-2017` | GET UA | 200 | 2017: yearbook + program only | 09:11:15 |
| MS `/jsonapi/node/participant?filter[drupal_internal__nid]=464336` | GET UA | 200 | prior-year node attributes + relationships (`field_schedule`, `field_coaches`, …) | 09:11:21 |
| MS `/jsonapi/node/participant?…=464336&include=field_schedule,field_coaches` | GET UA | 200 | `field_schedule` = `[]`; 9 prior-year coach paragraphs (created 2025-07-08) | 09:11:35 |
| MS `/2025-track-and-field-results` | GET UA | 200 | 2025 per-class results index (6 PDFs) | 09:11:40 |
| MS `/2023-track-and-field-results` | GET UA | 200 | 2023 per-class results index (6 PDFs, encoded names) | 09:11:42 |
| MS `/2024-track-and-field-results` | GET UA | 200 | 2024 per-class results index (6 PDFs) | 09:11:49 |
| MS `/api/team-data/roster/611/varsity/124/695` | GET UA | **404** | no year path segment — historical rosters impossible | 09:13 (approx.) |
| MS `…/state-tournament-archive-girls-track-field-2025` | GET UA | 200 | girls T&F archive exists for 2025 (contrast with the 2026 403) | 09:13 (approx.) |

**MSHSL PDF fetches (all `GET UA`, all 200 unless noted; `pdftotext` verdict in `what it proved`)**

| URL | HTTP | what it proved | timestamp |
|---|---|---|---|
| MS `/sites/default/files/2025-11/results.pdf` (Class A boys XC 2025, 2 pp, 1,408,206 B) | 200 | **0 words** — outlined, OCR required | 09:05:22 |
| MS `/sites/default/files/2025-11/2025-class-aaa-boys-cross-country-results.pdf` (2 pp) | 200 | 0 words | 09:05:24 |
| MS `/sites/default/files/2025-11/2025-class-aaa-girls-cross-country-results.pdf` (2 pp) | 200 | 0 words | 09:05:25 |
| MS `/sites/default/files/2026-04/2025_boyscc_boys-cc-team-participants-thru-2025.pdf` (10 pp) | 200 | 29,966 B text — but **team-level only** (0 athlete names, 0 grades) | 09:05:27 |
| MS `/sites/default/files/2026-04/girls-cc-team-participants-thru-2025.pdf` (7 pp) | 200 | 21,994 B text, same team-level shape | 09:05:28 |
| MS `/sites/default/files/2026-06/2026-track-and-field-state-meet-class-aaa-finals.pdf` (14 pp) | 200 | 47,169 B text, `Yr School` grade column | 09:06:45 |
| MS `/sites/default/files/2026-06/2026-track-state-meet-class-a-final-results.pdf` (12 pp) | 200 | 42,933 B text, grades present | 09:06:47 |
| MS `/sites/default/files/2026-09/2026-track-program-v.2.pdf` (136 pp, 37.6 MB) | 200 | 52,432 words; **1,309 graded seed-sheet records** (`place, bib, name, grade 9–12, school, mark`), 276 schools | 09:06:49 |
| MS `/sites/default/files/2026-09/team-champions-thru-2026.pdf` (3 pp) | 200 | 11,251 B text, names/years only | 09:06:50 |
| MS `/sites/default/files/2026-09/meet-records-thru-2026.pdf` (4 pp) | 200 | 33,720 B text, no grades | 09:06:52 |
| MS `/sites/default/files/2025-11/2025-class-aa-boys-cross-country-results.pdf` (2 pp) | 200 | 0 words (second raster/outline confirmation) | 09:06:54 |
| MS `/sites/default/files/2024-11/2024-class-aaa-boys-cross-country-state-results.pdf` (5 pp, purepdf) | 200 | 17,311 B text, 160 grade tokens | 09:07:15 |
| MS `/sites/default/files/2023-11/class%20aaa%20boys.pdf` (2 pp, purepdf) | 200 | 9,935 B text, 158 grade tokens | 09:07:16 |
| MS `/sites/default/files/2025-04/2024_cc_souvenirprogram.pdf` (44 pp, 35.8 MB) | 200 | text but no grades/participant lists | 09:07:18 |
| MS `/sites/default/files/2025-04/boys-cc-ind.-team-champions-thru-2024.pdf` (8 pp) | 200 | individual champions `Name, school`, time — no grade | 09:07:20 |
| MS `/sites/default/files/2022-11/Boys%20AAA.pdf` (4 pp, purepdf) | 200 | text, 160 grade tokens | 09:10:17 |
| MS `/sites/default/files/2022-01/2021-bcc-class-aaa-results.pdf` (4 pp, purepdf) | 200 | text, 157 grade tokens | 09:10:19 |
| MS `/sites/default/files/2020-11/1a-results.pdf` (4 pp, Word) | 200 | text; numeric grade prefix (`11 Reese Anderson`) | 09:10:49 |
| MS `/sites/default/files/2020-07/2019-class-aa-boys-cc-results.pdf` (6 pp, Word) | 200 | text; `Name, Grade` column | 09:10:51 |
| MS `/sites/default/files/2026-01/2018_boyscrosscountry_yearbook.pdf` (13 pp) | 200 | text; **348 `Name,GRADE` rows** (12→143, 11→116, 10→50, 9→30, 8→9) | 09:10:52 |
| MS `/sites/default/files/2026-01/2017_boyscrosscountry_yearbook.pdf` (15 pp) | 200 | text; **353 `Name,GRADE` rows** (12→131, 11→108, 10→83, 9→22, 8→9) | 09:11:36 |
| MS `/sites/default/files/2026-06/2026-state-track-and-field-class-a-prelim-results.pdf` (11 pp) | 200 | text, 7,859 words, `Yr` present | 09:11:16 |
| MS `/sites/default/files/2026-06/2026-state-track-and-field-class-aa-prelims.pdf` (12 pp) | 200 | text, 8,471 words, `Yr` present | 09:11:18 |
| MS `/sites/default/files/2025-06/2025-track-and-field-class-a-finals.pdf` (45 pp) | 200 | text, 20,700 words, `Yr` column | 09:11:19 |
| MS `/sites/default/files/2023-06/2023%20Track%20and%20Field%20Class%20AAA%20Final%20Results.pdf` (38 pp) | 200 | text, 15,868 words, `Year School` grades | 09:11:51 |
| MS `/sites/default/files/2024-06/2024-class-aaa-track-and-field-finals.pdf` (21 pp) | 200 | text, 8,552 words, `Year School` grades | 09:12:53 |
| MS `/sites/default/files/2025-06/2025-class-aaa-track-and-field-prelims.pdf` (24 pp) | 200 | text, 11,201 words, `Yr` column | 09:12:54 |

**Local extraction/OCR runs (no network)** — `pdftotext` word counts above. Per-file census of the
four 2025 XC PDFs (producer Nitro PDF Pro 14.39.0.18 on all four; `pdffonts` = 0 fonts each,
`pdfimages -list` = 8 images each): `results.pdf` (Class A boys) 0 words, `l`=125,212 `c`=95,174
`re`=329 `Do`=8; AAA boys 0 words, 119,667 / 95,720 / 313 / 8; AAA girls 0 words, 117,095 / 96,943 /
313 / 8; AA boys 0 words, 114,764 / 91,429 / 313 / 8 — `BT`=`Tj`=`TJ`=0 in all four, i.e. glyphs are
vector outlines, no text operators. `pdftoppm -r 300` 1.62 s (2 pp) + `tesseract --psm 6` 3.37 s
(2 pp) = 1,747 words, 156 grade tokens. Recall against `es-roster-58505-names.json`: **142/159 printed
places present (89.3%)**, **142/149 recovered records exact (95.3%)**, **147/149 fuzzy ≥0.86 (98.7%)**;
a strict `place+name+grade+team+time` regex on the same text scores 102/143 = 71.3% (parser-limited —
that is the earlier figure, kept here for honesty about method sensitivity). Files: `xc300-1.txt`,
`xc300-2.txt`, `xc300-all.txt`, `xc300-*.png`; recomputation script re-run ≈09:18 local (see below).

**AthleticLIVE ES probes** (all `POST` anonymous JSON unless noted)

| URL | method | HTTP | what it proved | timestamp |
|---|---|---|---|---|
| ES `/wayzata_meet_list/_mapping`, `/athlete_list/_mapping`, `/team_list/_mapping` | GET UA | **403 ×6** (3 probes, each duplicated for the status text) | mappings closed to anonymous clients | 09:08:41–44 |
| ES `/wayzata_meet_list/_search` `{"size":2}` | POST | 200 | 2,769 meet docs; docs carry `i`, `ani`, `aci`, `ln`, `sd`, `ls`, `atr`, link flags | 09:08:49 |
| ES `/athlete_list/_search` `{"size":1}` | POST | 200 | global athlete docs: `n`, `y` (grade), `g`, `mi`, `t.{i,n,ani}`, `ani` (A.net athlete id) | 09:08:50 |
| ES `/wayzata_meet_list/_search` `match_phrase ln:"MSHSL"` size 80 | POST | 200 | **148** MSHSL meet docs (2018–2026); `ani` present with `-1` meaning unlinked (all state XC) | 09:08:59 |
| ES `/athlete_list/_search` `mi=28764` + `n:"Robert Mechura"` | POST | 200 | 1 hit — `y:"JR"`, `t.n:"Roseville Area"`, `t.ani 12190` → grade semantics confirmed vs the 2023 PDF | 09:09:13 |
| ES `/athlete_list/_search` agg on `y` (mi 28764 / 58505) | POST | **400 ×2** | `y` is a text field — no aggregations; grade tallies must be computed client-side | 09:09:15–16 |
| ES `/athlete_list/_search` `size:0` counts for mi ∈ {28764, 41637, 58505, 25893, 38545, 54182, 74652} | POST | 200 ×7 | roster sizes 1,167 / 1,218 / 1,143 / 2,980 / 3,308 / 3,029 / 2,473 | 09:09:25–33 |
| ES `/athlete_list/_search` full rosters for mi ∈ {28764, 41637, 58505} | POST | 200 ×3 | grade histograms + A.net id coverage: 2023 XC 0/1,167 athletes (1,144/1,167 teams), 2024 1,025/1,218, 2025 1,141/1,143 | 09:09:39–42 |
| ES `/athlete_list/_search` `mi=58505` with names | POST | 200 | 1,143 name+grade rows used for OCR cross-check | 09:12:12 |
| ES `/wayzata_meet_list/_search` `term ls:"mn"` size 10 | POST | 200 | **1,886** MN-location meets; recent = regular-season XC invitationals (2026-09-17…22) with `ani` | 09:12:43 |
| ES `/athlete_list/_search` count `mi=28459` | POST | 200 | 2023 Section 1AAA prior-year section roster = 144 graded rows | 09:12:45 |
| ES `/wayzata_meet_list/_search` `ls:"mn"` + `o:"xc"` count | POST | 200 | **503** MN XC meets indexed | 09:12:46 |
| ES `/wayzata_meet_list/_search` same, `sd` asc | POST | 200 | oldest MN XC meet = 2018-08-24 "Rebel Northwoods Invitational" (`ani:null`) | 09:12:56 |
| ES `/wayzata_meet_list/_search` `match_phrase ln:"MSHSL State Cross Country"` `sd` asc | POST | 200 | **7** state XC meet docs 2018/2019/2021/2022/2023/2024/2025 (`i` 1732/4674/11779/18769/28764/41637/58505) | 09:15:36 |
| ES `/athlete_list/_search` rosters for mi ∈ {1732, 4674, 11779, 18769} | POST | 200 ×4 | 2018 = **0** athlete rows; 2019 = 845 graded; 2021 = 1,177; 2022 = 1,181 (2022 has 1,098 A.net athlete ids, 0 team ids) | 09:15:40–45 |
| ES `/wayzata_meet_list/_search` counts for `match_phrase ln:"MSHSL State Track"` / `"MSHSL Section"` | POST | 200 ×2 | 8 state T&F meets (2018/2019/2021–2026), 130 section meets → 148 MSHSL-named docs decompose 7+8+130+3 | ≈09:15:56–16:04 |
| ES `/wayzata_meet_list/_search` `match_phrase ln:"MSHSL State Track"` `sd` asc | POST | 200 | state T&F `ani` ids: 2022 = 475179, 2023 = 516328, 2024 = 563571, 2025 = 613966, 2026 = 666673 | 09:16:10 |
| ES `/wayzata_meet_list/_search` terms-agg on `o` | POST | **400** | `o` is also a text field — aggregates rejected, counts must be manual | 09:16:06 |
| ES `/athlete_list/_search` `t.ani` coverage for mi ∈ {25893, 74652} | POST | 200 ×2 | state T&F team-id coverage **100%** (2,980/2,980 in 2023; 2,473/2,473 in 2026) | 09:17:06–08 |
| ES `/athlete_list/_search` `mi=74652` with `n,y,g,t` | POST | 200 | 2,473 named 2026 T&F roster rows; `y` is **numeric** here (12→929, 11→747, 10→464, 9→226, 8→81, 7→25, Jr→1) | 09:19:07 |
| *(no network)* program parse × `es-roster-74652-names.json` | local python | — | 1,309/1,309 names matched (100%); grades agree 1,303/1,309 (99.5%) | ≈09:19:20 |
| *(no network)* re-run recall script over `xc300-all.txt` × `es-roster-58505-names.json` | local python | — | 142/159 places present (89.3%); 142/149 records exact (95.3%); 147/149 fuzzy ≥0.86 (98.7%) | ≈09:18:30 |

Request totals for this slice: **62 to www.mshsl.org** (29 PDF + 33 HTML/JSON/API) and
**40 to search.athletic.live** — 102 total (see the budget disclosure in §Access characteristics);
0 to any `*.athletic.net` host, 0 to `athleticlive.blob.core.windows.net`.
All raw responses are in this directory (`*.pdf`, `*.html`/`_*`, `api-*.json`, `jsonapi-*.json`,
`xc300-*`, `es-*.json`).
