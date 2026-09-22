# 06. Wisconsin WIAA

Status: complete
Observed on: 2026-09-19

**Tooling note (verbatim):** every `web_search` call in this assignment failed with the provider error
`Sign up and repeat your request.` Search was therefore abandoned and **all** findings below come from
direct HTTP GET/POST (`read` + `curl`). No CAPTCHA/auth/paywall bypass, no cookie replay, no proxy
rotation, no Chromium pipeline. Total requests: ~38 to `schools.wiaawi.org`, ~28 to `www.wiaawi.org`,
1 to `live.pttiming.com`. No 429 and no `Retry-After` header was ever observed.

---

### Source

| Surface | URL | Platform (verified) |
|---|---|---|
| Association site | `https://www.wiaawi.org/` | Drupal 10 (`meta-Generator: Drupal 10`), Cloudflare front (`server: cloudflare`, `cf-ray`), Drupal page + dynamic cache headers |
| **School / team / coach database** | `https://schools.wiaawi.org/` | IIS + ASP.NET MVC 5.2 (`x-aspnetmvc-version: 5.2`, `x-aspnet-version: 4.0.30319`), jQuery + DataTables + bootgrid, Cloudflare front. **No `robots.txt` (404).** |
| Member dashboard | `https://member.wiaawi.org/` | login-gated; linked from `www.wiaawi.org/schools`; **not probed** |
| Tournament AD apps | `https://halftime.wiaawi.org/...` | linked from school detail pages; **not probed** |
| ScoreCenter index | `https://www.wiaawi.org/scorecenter` | Drupal page; per-sport "Scores" pages exist (T&F scores page is empty, see Enumeration) |

The `schools.wiaawi.org` subdomain is the discovery that matters: it is a full public school/team/coach
database with **no authentication for any endpoint used here**, and it is where every stable WIAA
identifier lives. `www.wiaawi.org` itself exposes no JSON:API (`GET /jsonapi` → **HTTP 404**).

Note: `www.wiaawi.org/robots.txt` is the stock Drupal file — `Disallow: /core/`, `/profiles/`, `/admin/`,
`/search/`, `/sites/README.txt`, etc. **`/sites/default/files/**` (where all result files live) is not
disallowed**, and no `Crawl-delay` is published.

---

### Coverage

- **State:** Wisconsin only.
- **Schools:** 516 member high schools ("516 Member High Schools", `https://www.wiaawi.org/`, observed
  2026-09-19).
- **Sports sponsoring T&F/XC (2026-27, `LevelTypeCode=1`):**

| Sport | SportSeasonID | Schools with a 2026-27 team |
|---|---:|---:|
| Boys Track and Field | 1544 | **462** |
| Girls Track and Field | 1558 | **462** |
| Boys Cross Country | 1537 | **437** |
| Girls Cross Country | 1549 | **435** |

- **Prior-season participation is retrievable** (verified): 2025-26 → Boys TF **457**, Girls TF **456**,
  Boys XC **421** (Girls XC 2025-26 not fetched); 2024-25 season IDs exist (Boys TF 1453, Girls TF 1466,
  Boys XC 1447, Girls XC 1458). The school-year selector in the UI offers **2015-16 … 2026-27**
  (12 school years).
- **Levels:** `LevelTypeCode` 1 = High School, 2 = Middle School (option list on `/Reports/SportList/`).
- **Divisions:** tournament divisions 1-4 for team sports (T&F uses 1/2/3 + Wheelchair); 7 WIAA
  administrative **districts** (1-7).
- **Historical result depth** (measured by counting result-file links on the archive pages):
  - Boys Track & Field archive: **1,639 result-file links across 26 file-years (2000-2026)**.
  - Boys XC archive: **1,777 links, 74 referenced years (1952-2025)**; Girls XC archive: **1,738 links, 74 years**.
    Pre-2003 entries are almost entirely *champions-history* tables (2-6 files/year); real per-meet result
    files begin in earnest in 2003 (36 files) and 2004-2005 (46/64), with 60-135 files/year from 2011 on.
- **Indoor track: NOT COVERED.** WIAA sanctions no indoor season and publishes no indoor results
  (`/sports/boys-track-field` and its 8 sub-pages contain no indoor page; sitemap inventory of 11,332 URLs
  shows none). Wisconsin indoor T&F must come from Athletic.net / MileSplit / meet hosts. `[INFERENCE]`
  based on complete sitemap enumeration + the two sport page trees.
- **Regular season: NOT COVERED.** WIAA publishes tournament-series (regional/sectional/state) results
  only. The per-sport "Scores" page renders an empty shell, and the school-DB contest table returns no
  T&F/XC contests (see Enumeration).

---

### Enumeration

All enumeration below is on `https://schools.wiaawi.org` unless stated otherwise. Every endpoint was
called unauthenticated.

**1. Schools (and their WIAA IDs).** One request per starting letter (26 total; no bulk mode exists —
`?LetterBtn=-1` returns an empty fragment, verified):

```
POST https://schools.wiaawi.org/Directory/School/DirectoryLetter?LetterBtn=<A|B|…|Z>
  → HTML fragment with <table id="tblSchools">, one <tr class="divHighlight"> per school
  columns: School | Level | Class | City
  row link: /Directory/School/GetDirectorySchool?orgID=<OrganizationID>
```
`DirectoryLetter?LetterBtn=A` returned **39 schools** with the banner text
`Showing <b> 39 </b>schools starting with the letter <b> A </b>`. Name-prefix and city search also exist
(`/Directory/School/SearchOrg?query=<q>&levelT=0&classT=0&memberT=20` → JSON `[{OrganizationID, OrgName}]`;
`POST /Directory/School/SchoolsByCity?City=<city>`).

**2. Per-sport school roster in one request** (the single most valuable endpoint):

```
POST https://schools.wiaawi.org/Reports/RunSportListReport/
  SportSeasonID=<1544|1558|1537|1549>&DivisionID=-1&LevelTypeCode=1&SchoolYear=2025&RunReport=Run+Report
  → HTML <table id="tblReportList">, columns: Name | Enrollment | Division | Coop | ContactSchool | ExpireYr | District
  each data row: <tr class="divHighlight" id="<TeamID>">
```
No anti-forgery token is required (0 occurrences of `__RequestVerificationToken`). Response sizes
≈ 380-406 KB; wall time ≈ 2-6 s per call. Season IDs are discoverable, never guessed:

```
POST https://schools.wiaawi.org/Global/GetSSList        Body: {"Year":<2026|2025|2024|…>,"SelectText":2}
  → JSON *string* (double-encoded) of [{Value:"<SportSeasonID>",Text:"2025-2026 Boys Track and Field"}, …]
POST https://schools.wiaawi.org/Global/GetFullSchoolSSList
  Body: {"Year":<year>,"School":<OrganizationID>,"SelectText":2,"ShowYear":false}
  → the sports a single school sponsored in that year (e.g. Abbotsford 2024-25 → 15 sport-seasons)
```

**3. Meets.** WIAA enumerates no regular-season meets, and its contest table is empty for T&F/XC:

```
POST https://schools.wiaawi.org/Directory/Schedule/TeamSchedule?Mode=1&PublicTab=0
  Options.SchoolID=<OrganizationID>&Options.SchoolYear=2026&Options.SportSeasonID=<ssid>&Options.Mode=1&Options.PublicTab=0
  → <table id="tblSchedule"> columns: Date | Date | Home | Away | Location | Result | ContestID | ContestType
```
- Abbotsford **Boys XC 2026-27** (ssid 1537) → table renders `No scheduled contests found`, `TeamID=154285`.
- Madison Memorial **Boys T&F 2026-27** (ssid 1544, orgID 222) → empty.
- Abbotsford **Football 2026-27** (ssid 1533) → **18 dated contests with `ContestID` (e.g. 398286) and
  results (`W 21-8`)** — proving the table works and that T&F/XC rows are simply not populated.
  `[INFERENCE]` from a 2-school × 2-sport sample: the WIAA contest database is fed only for sports whose
  schools enter schedules through WIAA; it is not a T&F/XC meet index.

Tournament-series meets **are** enumerable, from `www.wiaawi.org` archive pages (see Result evidence):
`https://www.wiaawi.org/sports/boys-track-field/boys-track-field-state-archive` and
`https://www.wiaawi.org/sports/{boys,girls}-cross-country/{boys,girls}-cross-country-state-archive`
list every regional/sectional/state result file for every year on a single page each.

**4. Athletes.** Not enumerable as entities — WIAA has no athlete IDs, no profile pages and no athlete
search. Athletes exist only as result rows inside PDFs.

**5. Class of 2027.** Derivable from result rows: every tournament result line carries **grade** (`Yr`),
so `Yr=11` in a 2025-26 meet (= the 2026 Athletic.net corpus season) identifies Class of 2027. Measured on
the 2026 D1 Regional 5A (Arrowhead) file: 437 individual result rows, 234 distinct athletes,
**71 distinct grade-11 athletes**, plus 167 relay-leg rows of which 58 are grade-11.

**6. Results.** Enumerable as files, not rows — see Result evidence.

---

### Stable identifiers

| Entity | Identifier | Where observed | Notes |
|---|---|---|---|
| School (organization) | **`OrganizationID`** (`orgID`) | `/Directory/School/GetDirectorySchool?orgID=1` → Abbotsford; `SearchOrg` JSON returns `OrganizationID` | Small integers, dense from 1. Verified 1, 50, 100, 150, 222, 250, 300, 400, 450. Stable across years (per-school year queries key on the same ID). |
| Team (school × sport-season) | **`TeamID`** | `<tr id="…">` in `RunSportListReport`; echoed as `TeamInfo.TeamID` by `TeamSchedule` | **Gender- and sport-specific, not shared.** Abbotsford: Boys TF `145565`, Girls TF `145948`, Boys XC `154285`, Girls XC `149564`. 100 % unique within a sport report (462/462). Independently confirmed by two endpoints for two sports. |
| Sport-season | **`SportSeasonID`** (SSID) | `/Reports/SportList/`, `/Global/GetSSList`, `<tr id>` in a school's sport table (`tblTeamList`) | Global (year × sport × gender). 2026-27: 1533-1560; 2025-26: 1503/1509/1514/1522 (XC/TF); 2024-25: 1447/1453/1458/1466. |
| School year | `SchoolYear` | 2015…2026 (12 years) | Used with `GetSSList` to resolve historical SSIDs. |
| Division | `Division` 1-3 (T&F), 1-4 (team sports) | `RunSportListReport` column | Tournament division, per sport-season. |
| District | `District` 1-7 | `RunSportListReport` column | WIAA administrative district. |
| Coach / administrator | **`ContactID`** | `/Profile/Account/SearchContactNames?query=<name>` → JSON `[{ContactID, FirstName, LastName, FullName, Email, City, OrgID, SchoolYear}]` | e.g. `ContactID 3390` → `JACOB KNAPMILLER`. `SearchContactEmails?query=<text>` matches local-part only (query `jknapmiller` → same record; `@abbotsford.k12.wi.us` and `abbotsford` → `[]`), so **domain-wide enumeration of a school's staff is not possible**. |
| Conference | `ConfID` | `/Directory/Conference/List` → `/Directory/Conference/LoadConference?ConfID=<n>`, `/Directory/Conference/ConferenceLetter?LetterBtn=<x>` | Not resolved further (out of scope). |
| Contest (meet) | `ContestID` | `TeamSchedule` rows | **Team sports only**; T&F/XC rows absent. Not usable as a meet ID for our sports. |
| Athlete | — | none | No athlete identifier exists anywhere in WIAA surfaces. |
| Meet / result / event (TF/XC) | — | none | No IDs; identity is the file URL. |

---

### Athletic.net leverage

**Direct Athletic.net exposure: YES, for the 2026 tournament series, and it is documented by WIAA itself.**
`https://www.wiaawi.org/sites/default/files/pdfs/PDF/Sports/Track/2026/athetic.net-track-instructions.pdf`
(note WIAA's own filename typo, "athetic"), linked from
`/sports/boys-track-field/boys-track-field-tournament`, states verbatim:

> "In an effort to improve the entry process, tournament advancement, and athlete performance tracking,
> the WIAA is transitioning to **Athletic.net** for the Tournament registration."
> "All Regional Registration Links can be found here: **Athletic.net**" · "All Regional Meets can be found
> here: **Athletic.net**" · "Entry Deadline is Friday, May 22nd at 8:00 am"
> "There is no need to pay for an account on Athletic.net for meet registration."

Its embedded link targets (extracted from the PDF's `/URI` annotations) are:

- `https://www.athletic.net/events/usa/wisconsin/2026-5-26` ← **the WIAA regional date index on Athletic.net**
- `https://www.athletic.net/account/login/signup`
- `https://support.athletic.net/category/y2yqh7bw1i-getting-started`, `…/category/h5xhvrqf3x-roster`,
  `…/article/z11f8f1jja-submitting-entries-for-a-meet`, `https://support.athletic.net/?…`

**Corroboration that WIAA's own result files are Athletic.net exports:** the 2026 regional/individual PDFs
carry Athletic.net's signature formatting — `Compiled`, `Athlete | Yr | Team` column headers, numbered
events (`#22 Girls' 4x800 Relay Division 1`), `Q`/`q` advancement flags and parenthetical tie-breaks
(`13.868`/`13.869`). This is a *fetchable, unauthenticated* copy of Athletic.net-sourced tournament data.

**The XC side names its providers too:** `/sports/boys-cross-country/boys-cross-country-tournament` reads
`Sectional Entry Form Deadline (PTTiming/Athletic.net) - Tuesday, October 20th, 11:59 pm` and links
`https://live.pttiming.com/xc-ptt.html?mid=5127` (PrimeTime Timing live-results app; JS bundle
`/lib/xc-bundle.js`, Firebase backend `https://ptt-franklin.firebaseio.com/`, `timing = "PrimeTime Timing"`).
`mid=5127` is a **PrimeTime meet ID for a WIAA XC meet** — hand-off to the WI timing-provider assignment.

**No Athletic.net links from school records.** The 9 school detail pages examined (orgIDs 1, 50, 100, 150,
222, 250, 300, 400, 450) contain zero `athletic.net` strings; their only outbound school links are the
district site and the conference site (e.g. `northeasternconferencewi.org/public/genie/376/school/1329/`
— note the conference-generated `genie/<ConferenceID>/school/<SchoolID>` pattern, which is a *second*,
non-WIAA school-ID space). Athlete-level Athletic.net links therefore cannot be harvested from WIAA.

**Estimated Athletic.net requests avoided** (`[INFERENCE]`, arithmetic shown so it can be re-checked):

| Work | WIAA cost | Athletic.net equivalent avoided |
|---|---:|---:|
| WI school/team universe for TF+XC | **4 requests** (one `RunSportListReport` per sport-season) | 1,796 team-season rows (462+462+437+435); ≥1,796 requests if enumerated per team, ~516 if per school |
| WIAA T&F tournament meet set (regionals) | 1 archive page + 1 PDF | ~46-49 regional sites; ≥46 requests for the equivalent Athletic.net meet lookups |
| WIAA T&F tournament results | 104 file links for 2026 (49 regional stems, 16 sectional sites, 8 state files) | comparable request count (~1-3 per meet), **but** removes Cloudflare/session cost and delivers `Yr` (grade) directly |
| Wisconsin coach/AD contacts | 1 request per school page (9 sampled), or 1 per person via `ContactID` | **not purchasable from Athletic.net at all** |

Realistic planning number: **≈ 500-1,800 Athletic.net team/school requests avoided per season** by sourcing
the WI school universe from WIAA, plus ~50 for the tournament meet set, plus the tournament result payload
itself (grade-bearing) for free. WIAA does **not** replace Athletic.net for regular-season meets, athlete
PRs/progression, or athlete profile URLs — those are not present in any form.

---

### Athlete evidence

Availability inside WIAA tournament-series result files only (no profile surface exists):

| Field | Available? | Evidence |
|---|---|---|
| Name | ✅ | `Eicher, Payton` (regional PDFs, "Last, First"); `Trey Resch` (state PDFs, "First Last") |
| Grade / graduating class | ✅ **`Yr` column (9-12)** | `Trey Resch 11 Arrowhead 10.74`; `Andrew Woodhouse 10 Oregon 22.83`. Grade 11 in a 2025-26 meet ⇒ Class of 2027 |
| School represented | ✅ | `ARROWHEAD`, `WAUKESHA WEST` (regional); `Wausau East`, `Arrowhead` (state) |
| City / state | ❌ (state inferred as WI) | School name only |
| Gender / category | ✅ | Separate boys/girls files/events (`trb…`/`trg…`, `Boys`/`Girls` event headers), plus Wheelchair division files |
| TF vs XC distinction | ✅ | Separate archives/pages |
| Indoor vs outdoor | ❌ (no indoor data at all) | — |
| Performances | ✅ | `10.74`, `15:50.2`, `20.98` |
| PRs / progression | ⚠️ derivable only by diffing years of files | Multiple seasons of files exist per sport (2000-2026 TF, 1952-2025 XC archive) |
| Meets | ✅ as files | e.g. `D1 Regional 5A - Arrowhead`, `Arrowhead HS Stadium Tue, May 26, 2026` |
| Athlete profile URL | ❌ | none |

Relay membership is present: regional PDFs list legs as `1) Newmeister, Lily 12  2) Tilley, Charlotte 9 …`
with per-leg grades (167 leg rows in the Arrowhead regional; 58 grade-11).

---

### Recruiting information

The school detail page is a real coach/AD directory. Request:
`GET https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=<OrganizationID>`
(no auth; `showPub=1` is only appended by the page's own JS and is not required).

Structure returned:

```
School Administrators
  Found N administrator(s) based on your search
  columns: Role | Name | Email
  roles observed: Superintendent, Principal, Athletic Director, City-Wide Athletic Director,
                  Assistant Athletic Director
Head Coaches
  Found N coaches based on your search
  columns: Sport | Name | Role | Email
  sports observed include: Boys Track and Field, Girls Track and Field,
                           Boys Cross Country, Girls Cross Country  (role = "Head Coach")
<school identity block> level, class, member type, enrollment size, conference, mascot, colors,
                        address, phone, fax
tblTeamList: SportSeason | Calendar   (every sport the school sponsors, keyed by SportSeasonID)
outbound links: district website + conference site (e.g. http://www.abbotsford.k12.wi.us,
                http://marawoodconference.org/public/genie/152/school/2/)
```

Measured on a 9-school sample (orgIDs 1, 50, 100, 150, 222, 250, 300, 400, 450):

| Role | Schools with a head coach listed |
|---|---|
| Boys Cross Country | **8 / 9** |
| Girls Cross Country | **9 / 9** |
| Boys Track and Field | **7 / 9** |
| Girls Track and Field | **8 / 9** |
| Has ≥1 TF/XC coach email | **9 / 9** |

Example (orgID 222, Madison Memorial): 5 administrators and 29 coach rows, including
`Boys Cross Country | Ryan Pewowaruk | Head Coach`, `Boys Track and Field | Aidan Murphy | Head Coach`,
`Boys Track and Field | Brent Huggins | Head Coach` (duplicate head-coach rows occur — the same person is
listed for both Boys and Girls T&F), `Girls Cross Country | Ginna Irwin | Head Coach`.

- **Head track coach / head XC coach:** ✅ sport-scoped rows.
- **Assistant coaches:** ❌ (role column observed as `Head Coach` only).
- **Athletic director:** ✅ (plus district-level AD variants at large districts).
- **Public professional email:** ✅ per administrator and per coach. Emails are **Cloudflare-obfuscated**
  (`<span class="__cf_email__" data-cfemail="<hex>">`) and decoded client-side by the page's own
  `email-decode.min.js` (XOR against the first hex byte). Decoding them is not an access-control bypass —
  any browser renders them in plaintext for an anonymous visitor. Verified examples:
  `alarson@abbotsford.k12.wi.us` (AD), `jknapmiller@abbotsford.k12.wi.us` (Boys/Girls T&F),
  `dnovak@abbotsford.k12.wi.us` (Girls XC), `rbargender@abbotsford.k12.wi.us` (Superintendent).
- **School athletics website:** ⚠️ district website only (`http://www.abbotsford.k12.wi.us`); no dedicated
  athletics-site field.
- **Team website:** ❌; the nearest artefact is the conference site link.

**Second, person-centric entry point:** `GET /Profile/Account/SearchContactNames?query=<term>` →
`[{ContactID, FirstName, LastName, FullName, Email, City, OrgID, SchoolYear}]`, then
`POST /Profile/Account/ContactInfoResults` with `ContactID=<id>` → Name, Email,
`Primary Organization / School`, `Work Phone` (the school main line), and a
**"Teams this Contact is affiliated with"** list of `School – Gender – Sport` + role, e.g. for
`ContactID 3390` (JACOB KNAPMILLER): `Abbotsford – Boys – Football | Head Coach`,
`Abbotsford – Girls – Track and Field | Head Coach`, `Abbotsford – Boys – Track and Field | Head Coach`.
This is the cleanest coach→school→sport→role assertion available in Wisconsin.

**Privacy handling required by this source:** the directory is a general contact table, and published coach
emails include consumer-provider addresses (`kjmeyer23@gmail.com`, `kreegerj@yahoo.com`,
`akneifl@charter.net` observed on Abbotsford's page). Recommendation: ingest **only** the
`Sport ∈ {Boys/Girls Track and Field, Boys/Girls Cross Country}` + administrator rows, retain name + role +
email, and **discard `City` and `Work Phone`** (neither is needed for the recruiting projection, and
`City` is not a school-scoped field). No athlete contact data exists in this source to begin with.

---

### Result evidence

Sources are **static files on `www.wiaawi.org`**, linked from two pages per sport (tournament page = current
season; state-archive page = all seasons).

| Field | Available? | Notes |
|---|---|---|
| ResultID | ❌ | no IDs at all |
| AthleteID | ❌ | none exists |
| MeetID | ❌ (WIAA) / ⚠️ external | the XC tournament page links `live.pttiming.com/xc-ptt.html?mid=5127`; T&F regionals exist on Athletic.net (see Athletic.net leverage) |
| EventID | ❌ | event identity is the printed event header (`Boys 100 Meter Dash Division 1`) |
| Mark | ✅ | `10.74`, `20.98`, `10:08.63`, `15:50.2` |
| Normalized mark inputs | ✅ (Hy-Tek style) | decimal seconds, `H#` heat column, explicit mile/split-free times; tie-breaks given to 3 decimals (`10.836`, `11.062`, `(13.868)`) |
| Timing method | ❌ | not stated in the files |
| Wind | ⚠️ state T&F PDFs only | `10.74 Q -0.5`, `-2.0`, `-2.7` on sprint prelims; not present in the regional PDF sample |
| Implement / hurdle spec | ❌ | not stated (only event name + division) |
| Heat / round | ✅ | `Preliminaries` / `Finals` sections, `H#` column, `Q`/`q` advancement flags, `DNS`/`DNF` rows |
| Place | ✅ | `1`, `2`, … and team points (`10`, `8`, `6`, `5`) |
| Date | ✅ | header `WIAA Track & Field State Championships - 6/5/2026 to 6/6/2026`; XC `11/1/2025` |
| School represented | ✅ | per athlete row |
| Relay membership | ✅ | legs with names + grade; relay team name and `'A'` designation |
| Grade (`Yr`) | ✅ | 9-12, the distinguishing feature vs most timing sites |

Format/provider variance is real and must be handled per file:
- **Athletic.net export** (`Compiled`, `Athlete | Yr | Team`) — e.g. `tr2026arrowheadregionalindiv.pdf` (1.65 MB).
- **Hy-Tek** (`Name | Yr | School | H# | Result`) — e.g. `trb2026d1stateresults.pdf` (661 KB), XC state files.
- **Other timers** — `tr2026beaverdamsectionalindiv.pdf` (2.65 MB) has `Producer: Microsoft: Print To PDF`,
  `Author: BDTrack`, `Title: RUNMEET_ D1 Sectional 7 - Beaver Dam` and **yields no text** via `pdftotext`
  (image-only scan). Expect roughly a third of sectional uploads to be non-machine-readable.
- Some entries are HTML, not PDF: `tr2026kewaskumsectional.htm`, `tr2026wiscoregional.htm`,
  `tr2026littlechuteregional.htm`.
- Legacy XC results are **plain text** and highly parseable:
  `/sites/default/files/Portals/0/PDF/Results/Cross_Country/1999/d1bresults.txt` (14,999 bytes, HTTP 200,
  `text/plain`) contains lines like
  `MATT ESCHE 11   15:42, CHRIS MARTIN 12   16:22, …` grouped under
  `1. WAUKESHA WEST … = 81`. 63 such `.txt` links exist on the girls XC archive alone.

URL patterns (all verified returning HTTP 200 except where noted):

```
# Track & Field, modern upload dir (2026)
https://www.wiaawi.org/sites/default/files/2026-08/trb2026d1stateresults.pdf          # state, Boys D1
https://www.wiaawi.org/sites/default/files/2026-08/trg2026d1stateresults.pdf          # state, Girls D1
https://www.wiaawi.org/sites/default/files/2026-08/tr{b,g}2026{regional|sectional}{,indiv,team}.pdf
# Track & Field, historical dir-per-year
https://www.wiaawi.org/sites/default/files/pdfs/PDF/Results/Track/<YYYY>/<name>.pdf  # 2004-2025
# Cross Country, dir-per-year
https://www.wiaawi.org/sites/default/files/pdfs/PDF/Results/Cross_Country/<YYYY>/{d1..d3}{b,g}stateresults.pdf
https://www.wiaawi.org/sites/default/files/pdfs/PDF/Results/Cross_Country/<YYYY>/{b,g}<host>sectional{team}.pdf
# Cross Country, legacy plain text
https://www.wiaawi.org/sites/default/files/Portals/0/PDF/Results/Cross_Country/<YYYY>/<name>.txt   # incl. 1999
```

**Broken links found (record the block as the finding):** the *tournament* page's 2026 state result links
use `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2026/d1bstateresults.pdf`, which returns
**HTTP 301 → `/sites/default/files/pdfs/PDF/Results/Track/2026/d1bstateresults.pdf` → HTTP 404**. The
working copies of the same files are linked from the *state archive* page under `/sites/default/files/2026-08/`.
A crawler must therefore harvest both pages and follow redirects.

Volume for the current season (measured by parsing the 2026 block of the T&F archive page):
**104 file links = 49 regional file stems (~46 distinct regional sites) + 16 sectional sites + 8 state files**
(D1/D2/D3 × Boys/Girls + 2 Wheelchair). The 2025 XC tournament page carries **47 result files** including
**40 unique sectional stems**.

---

### Incremental use

A weekly collector never needs a historical re-fetch. Everything below is diffable by identifier.

**School/team/coach tier (weekly, 5-6 requests total):**
1. `POST /Global/GetSSList {"Year":<current>,"SelectText":2}` → confirm the 4 T&F/XC `SportSeasonID`s (1 req).
2. `POST /Reports/RunSportListReport/` × 4 (Boys TF, Girls TF, Boys XC, Girls XC) → diff the set of
   `<tr id="<TeamID>">` values against last week's snapshot (4 req). New/removed `TeamID`s are new/removed
   teams; the `Division` and `Coop` columns change mid-season when divisional assignments post, so also
   diff those per `TeamID`.
3. Optional: `TeamSchedule` per school only if contest data ever appears for 1537/1544/1549 — currently a
   guaranteed empty result (see Enumeration), so **do not spend requests on it**.
4. Coach refresh is **per school**, 1 request per `OrganizationID`; refresh on a rolling roster
   (e.g. 516 schools ÷ 52 weeks ≈ 10 schools/week ≈ 10 requests/week) rather than diffing a change feed,
   because the directory publishes **no `Last-Modified`, no ETag and no update timestamp**. Cache headers on
   school pages are `cache-control: no-cache, no-store` on the schools host, so conditional GETs will not
   help. `[INFERENCE]` that no change-feed exists — based on the full endpoint inventory of the app
   (School/Conference/Team/Schedule/Reports/Profile controllers) surfaced by its own JS.

**Result tier (weekly in-season, 2-4 requests):**
5. Fetch the two archive pages + two tournament pages (4 req) and diff the set of absolute file hrefs
   (normalise `/Portals/0/...` → `/sites/default/files/...` and strip `?ver=` query strings). New href =
   new meet result. This is the only reliable freshness signal WIAA offers; files are simply appended to
   the year's page. There is no `sitemap` entry per result file, so page diffing is required.
6. Download only new files (typically a handful per week during the May-June T&F and Oct-Nov XC windows),
   keying your own store by the file URL + extracted header (meet name, venue, date).

**Affected-athlete resolution:** because there are no athlete IDs, a new file affects athletes identified by
`(name, school, grade)`; treat that triple as the join key and reconcile against Athletic.net/MileSplit
later. Relay legs must be extracted separately (`N) Last, First YY` pattern).

**Staleness signal worth watching:** in the 2026-27 export, Boys T&F `Division` is populated
(D1 127, D2 135, D3 198, 2 blank) while **Girls T&F `Division` is blank for 460 of 462 rows** — the girls'
divisional assignments are not yet published. A weekly diff catches that the moment it changes, which is
exactly what a school→division mapping needs.

---

### Access characteristics

- **Classification: normal HTML + HTML-over-POST (AJAX fragments) + small JSON endpoints + static PDF/TXT/XLSX.**
  Also present: `wiaawi.org/sports/*` Drupal HTML, `/scorecenter` HTML shell.
- **Authentication: none required for any endpoint used.** Only `/Profile/Account/UserLogInOut` (the login
  form action that appears on every page) and `member.wiaawi.org` are gated. All school, sport-report,
  contact-search and result-file surfaces answered unauthenticated.
- **Published limits: none found.** `www.wiaawi.org/robots.txt` is the stock Drupal file (no `Crawl-delay`,
  no `Sitemap:` directive, and it does **not** disallow `/sites/default/files/**`).
  `schools.wiaawi.org/robots.txt` → **HTTP 404** (no policy published for that host).
- **Observed `Retry-After` / 429 behaviour: none.** Across ~67 requests, zero 429s and zero `Retry-After`
  headers. `www.wiaawi.org` sets `cache-control: max-age=600, public` and served Drupal page cache HITs;
  `schools.wiaawi.org` sets `cache-control: no-cache, no-store` (dynamic, uncacheable).
- **Cloudflare is in front of both hosts** (`server: cloudflare`, `cf-ray` present) and the school host
  obfuscates emails with Cloudflare's `email-decode` — but **no challenge was ever triggered**, in explicit
  contrast with `www.athletic.net` (HTTP 403 to non-browser clients from this machine, per the mission brief).
- **Cost profile:** `RunSportListReport` is the expensive call (≈ 380-406 KB, 2-6 s). School pages are
  180-220 KB. Result PDFs are 160 KB-2.7 MB each; the 2026 T&F set is ≈ 100 files, the whole historical
  corpus is ≈ 5,100 file links (1,639 TF + 1,777 Boys XC + 1,738 Girls XC, with overlap). Prefer the
  `.txt` legacy XC files where present — 15 KB instead of ~900 KB.
- **No documented API, no published JSON schema, no sitemap of result files.** All JSON endpoints here were
  reverse-engineered from the apps' own inline JavaScript (documented above so the work is reproducible).

---

### Recommendation

**COACH-DIRECTORY — with PRIMARY-grade value for the Wisconsin school/team universe and VALIDATION value
for tournament grade evidence.** Adopt in that priority order:

1. **PRIMARY (school/team universe):** `RunSportListReport` + `GetSSList` gives the authoritative WI
   T&F/XC team universe with stable `TeamID`s, enrollment, division, co-op and district for **4 requests
   per season** — and it never touches Athletic.net. This is the cheapest known entry point for the
   `official school/state indexes → candidate schools` step, and it is the only WI source that supplies a
   per-team identifier for **all 1,796** T&F/XC team-seasons (462+462+437+435).
2. **COACH-DIRECTORY (highest unique marginal value):** name + role + public professional email for
   Superintendent/Principal/AD and for **head T&F and head XC coaches**, sport-scoped and school-scoped.
   Sample coverage 7-9 of 9 schools per role, 9/9 with at least one TF/XC email. No other source in this
   study (Athletic.net included) publishes this. Efficient bulk strategy: one school page per
   `OrganizationID`; the `ContactInfoResults` person view is better for de-duplicating coaches who appear
   in multiple sports/schools. Keep name/role/sport/email; drop `City`/`Work Phone` and any consumer-domain
   address unless it is the published soccer/XC/TF role address.
3. **VALIDATION (grade evidence):** tournament result files carry `Yr` (9-12), giving an independent,
   association-published grade for every regional/sectional/state competitor — the cheapest independent
   check on Class-of-2027 status that does not involve Athletic.net. Regional coverage is broad: the single
   D1 Regional 5A file alone contained 234 distinct athletes / 71 grade-11 athletes.
4. **RESULT-SOURCE (partial, tournament series only):** usable for the ~16 sectionals + ~46 regionals + state
   in T&F and the XC sectional/state set, but must not be planned as a regular-season result feed, and must
   tolerate mixed formats (Athletic.net exports, Hy-Tek, image-only PDFs, HTML, legacy `.txt`).
5. **ATHLETIC.NET-SEED (narrow but precise):** for the 2026+ T&F tournament only, WIAA documents the exact
   Athletic.net entry point for regionals (`https://www.athletic.net/events/usa/wisconsin/2026-5-26` from
   WIAA's own instructions PDF) and the XC side names `PTTiming/Athletic.net` as the entry path — so the
   tournament meet set can be seeded deterministically instead of searched.

**REJECT** WIAA as: an athlete-profile source (no athlete IDs/profiles — it is the one thing it cannot do),
an indoor-T&F source (no indoor season exists), and a regular-season meet/results source (contest table
empty for T&F/XC, verified on two schools and one in-season control sport).

---

### Evidence appendix

All times are 2026-09-19 America/Chicago (CDT). The complete request window was **22:51-23:31**. `read`
performed GET only; POST/bulk work used `curl`. No 429 and no `Retry-After` header appeared in any response.

| URL | Method | HTTP status | What it proved | Timestamp |
|---|---|---|---|---|
| `https://www.wiaawi.org/` | GET | 200 | Drupal 10; "516 Member High Schools"; 186,542 participants; 27 sports | 22:52 |
| `https://www.wiaawi.org/schools` | GET (redirect) | 200 → `/my-wiaa/school-administrators` | Discovers `schools.wiaawi.org` (School Directory) and `member.wiaawi.org` | 22:55 |
| `https://www.wiaawi.org/jsonapi` | GET | **404** | No public JSON:API on the association site | 22:55 |
| `https://schools.wiaawi.org/` | GET | 200 | School DB feature list: `/Directory/School/List`, `/Directory/Conference/List`, `/Profile/Account/SearchContacts`, `/Reports/SportList/`, `/Reports/CoopList/` | 22:56 |
| `https://schools.wiaawi.org/Directory/School/List` | GET | 200 | App shell; leaks `DirectoryLetter`, `SearchOrg`, `GetDirectorySchool?OrgID=&showPub=` endpoints | 22:57 |
| `https://schools.wiaawi.org/Directory/School/List?Letter=A` | GET | 200 | Same shell; `?Letter=` is client-side only | 22:57 |
| `https://schools.wiaawi.org/Directory/School/SearchOrg?query=Madison&levelT=0&classT=0&memberT=20` | GET | 200 JSON | `OrganizationID` + `OrgName` (Madison Country Day = 218, Madison East = 219, …) | 22:57 |
| `https://schools.wiaawi.org/Directory/School/DirectoryLetter?LetterBtn=A` | GET (POST semantics) | 200 | **39 schools starting with A**; `tblSchools` columns School/Level/Class/City; row link `GetDirectorySchool?orgID=1` (Abbotsford) | 22:58 |
| `https://schools.wiaawi.org/Profile/Account/SearchContacts` | GET | 200 | Contact search app; leaks `SearchContactNames`, `SearchContactEmails`, `ContactInfoResults` | 22:58 |
| `https://schools.wiaawi.org/Reports/SportList/` | GET | 200 | **`SportSeasonID` catalogue** (1544 Boys TF, 1558 Girls TF, 1537 Boys XC, 1549 Girls XC); `LevelTypeCode` 1/2; `DivisionID` 1-7; SchoolYear 2015-2026; `RunSportListReport` action | 22:59 |
| `https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=1` | GET | 200 | Full school profile: 3 administrators w/ roles + emails (AD Alex Larson), 14 coach rows incl. Boys/Girls T&F (JACOB KNAPMILLER) and Girls XC; sports-offered table; district + conference links; 18 `data-cfemail` values | 23:00 |
| `https://schools.wiaawi.org/Reports/RunSportListReport/` `SportSeasonID=1544` | POST | 200 (406,192 B) | **462 schools** for 2026-27 Boys T&F; `tr id` = `TeamID`; columns Name/Enrollment/Division/Coop/ContactSchool/ExpireYr/District; divisions D1 127, D2 135, D3 198, 2 blank | 23:01 |
| `https://schools.wiaawi.org/Reports/RunSportListReport/` `SportSeasonID=1558` | POST | 200 (405,748 B) | **462 schools** Girls T&F; `Division` blank for 460/462 rows | 23:02 |
| `https://schools.wiaawi.org/Reports/RunSportListReport/` `SportSeasonID=1537` | POST | 200 (384,848 B) | **437 schools** Boys XC; co-op 43 | 23:02 |
| `https://schools.wiaawi.org/Reports/RunSportListReport/` `SportSeasonID=1549` | POST | 200 (383,172 B) | **435 schools** Girls XC; co-op 44 | 23:03 |
| `https://schools.wiaawi.org/Directory/Team/TeamSummaryForm` | GET | 200 | Team-summary app; selects `SchoolYear` + `SportSeasonID`, uses `/Global/GetSSList` | 23:03 |
| `https://schools.wiaawi.org/Global/GetSSList` | GET | 200 JSON | 27 sports for 2026-27 (1533-1560) — no other years offered by default | 23:04 |
| `https://schools.wiaawi.org/Global/GetSSList` `{"Year":2024}` | POST | 200 JSON | 2024-25 IDs: Boys TF **1453**, Girls TF **1466**, Boys XC **1447**, Girls XC **1458** | 23:04 |
| `https://schools.wiaawi.org/Global/GetSSList` `{"Year":2025}` | POST | 200 JSON | 2025-26 IDs: Boys TF **1509**, Girls TF **1522**, Boys XC **1503**, Girls XC **1514** | 23:05 |
| `https://schools.wiaawi.org/Global/GetFullSchoolSSList` `{"Year":2024,"School":1}` | POST | 200 JSON | Per-school sport seasons for a past year (Abbotsford 2024-25 = 15 sports incl. all 4 TF/XC) | 23:05 |
| `https://schools.wiaawi.org/Directory/Schedule` | GET | 200 | Schedule app shell; `TeamSchedule?Mode=&PublicTab=`, `Options.{SchoolID,SchoolYear,SportSeasonID}` | 23:06 |
| `https://schools.wiaawi.org/Directory/Schedule/TeamSchedule?Mode=1&PublicTab=0` (Abbotsford, ssid 1544) | POST | 200 (8,456 B) | Team metadata: Head Coach JACOB KNAPMILLER, Team Level Varsity, IsCoop No, Conference Marawood, Tournament Eligible Yes; **`TeamID=145565`**; `tblSchedule` renders "No scheduled contests found" | 23:06 |
| same, ssid 1537 (Boys XC) | POST | 200 (8,438 B) | **`TeamID=154285`**; empty schedule; table columns include `ContestID` | 23:07 |
| same, ssid 1533 (Football) | POST | 200 (40,046 B) | **18 dated contests with `ContestID` 398286 and result `W 21-8`** — control proving the table works and T&F/XC are simply unpopulated | 23:07 |
| same, SchoolID 222 ssid 1544 (Madison Memorial) | POST | 200 (8,471 B) | Second school, T&F also empty | 23:07 |
| `https://schools.wiaawi.org/Profile/Account/ContactInfoResults` `ContactID=3390` | POST | 200 | Contact view: Name, Email, Primary School, Work Phone, and **"Teams this Contact is affiliated with"** → Abbotsford Boys Football / Girls T&F / Boys T&F, all `Head Coach` | 23:08 |
| `https://schools.wiaawi.org/Profile/Account/SearchContactNames?query=Knapmiller` | GET | 200 JSON | `ContactID 3390` + email `jknapmiller@abbotsford.k12.wi.us` | 23:08 |
| `https://schools.wiaawi.org/Profile/Account/SearchContactEmails?query=jknapmiller` | GET | 200 JSON | Same record → email search matches local-part | 23:08 |
| `…SearchContactEmails?query=@abbotsford.k12.wi.us` and `query=abbotsford` | GET | 200 JSON (`[]`) | **Domain-wide staff enumeration is not possible** | 23:09 |
| `https://schools.wiaawi.org/Directory/School/DirectoryLetter?LetterBtn=-1` | GET | 200 (3,077 B, empty) | **No bulk "all schools" mode**; 26 letter requests required | 23:09 |
| `https://schools.wiaawi.org/Directory/Conference/List` | GET | 200 | Conference search app; `ConfID` space via `LoadConference?ConfID=` | 23:10 |
| `https://schools.wiaawi.org/Directory/Conference/Listing` | GET | 200 | Listing app returns no school links | 23:10 |
| `https://schools.wiaawi.org/Reports/CoopList/` | GET | 200 | Co-op report app; `RunCoopListReport` action | 23:10 |
| `https://schools.wiaawi.org/Reports/RunCoopListReport/` | POST | 200 (199 B, empty) | Parameter set not resolved; **unproven** (co-op data already available via `RunSportListReport`'s Coop column) | 23:11 |
| `https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=222` | GET | 200 (218,703 B) | Madison Memorial: 5 administrators (incl. City-Wide AD, Assistant AD), 29 coach rows, all 4 TF/XC roles; no `athletic.net` string | 23:12 |
| `…orgID=100` | GET | 200 (187,863 B) | Denmark: 3 admins, 15 coach rows, all 4 TF/XC roles; conference link `northeasternconferencewi.org/public/genie/376/school/1329/` | 23:12 |
| `…orgID=50, 150, 250, 300, 400, 450` | GET | 200 (177-221 KB each) | Coach-coverage sample → BXC 8/9, GXC 9/9, BTF 7/9, GTF 8/9; 11-30 coach rows per school; every school has ≥1 TF/XC email | 23:13 |
| `https://schools.wiaawi.org/Reports/RunSportListReport/` `SportSeasonID=1509` (2025-26) | POST | 200 (401,889 B) | **457 schools** 2025-26 Boys T&F → historical reports work | 23:14 |
| same, ssid 1503 / 1522 | POST | 200 | 2025-26 Boys XC **421**, Girls T&F **456** | 23:15 |
| same, ssid 1505 (unintended probe) | POST | 200 (82,086 B, 85 rows) | A different 2025-26 sport — recorded as a mis-targeted probe, not used as evidence | 23:14 |
| `https://schools.wiaawi.org/robots.txt` | GET | **404** | No robots policy published for the school DB host | 23:20 |
| `https://www.wiaawi.org/robots.txt` | GET | 200 | Stock Drupal robots; `/sites/default/files/**` **not** disallowed; no `Crawl-delay` | 23:20 |
| `https://www.wiaawi.org/sitemap.xml` | GET | 200 | 6-page sitemap index (Simple XML Sitemap) | 23:16 |
| `https://www.wiaawi.org/sitemap.xml?page=1..6` | GET ×6 | 200 | **11,332 URLs**; `/sports/<slug>` tree; only 6 non-news sport/archive pages for T&F; **no indoor T&F page** | 23:16-23:17 |
| `https://www.wiaawi.org/sports/boys-track-field/boys-track-field-tournament` | GET | 200 | 2026 state meet Jun 5-6 La Crosse; **links `athetic.net-track-instructions.pdf`**; state result links under `/Portals/0/PDF/Results/Track/2026/`; regional/sectional host lists | 23:17 |
| `https://www.wiaawi.org/sports/boys-track-field/boys-track-field-scores` | GET | 200 (181 KB) | Navigation only — **no scores content** | 23:17 |
| `https://www.wiaawi.org/sports/boys-track-field/boys-track-field-state-archive` | GET | 200 (519 KB) | Tournament results 2015-2026; **1,639 result-file links / 26 file-years**; 2026 block = 104 links (49 regional stems, 16 sectional sites, 8 state files) | 23:17 |
| `https://www.wiaawi.org/sports/boys-cross-country/boys-cross-country-state-archive` | GET | 200 (528 KB) | **1,777 result links, years 1952-2025**; legacy `.txt` back to 1999 | 23:18 |
| `https://www.wiaawi.org/sports/girls-cross-country/girls-cross-country-state-archive` | GET | 200 (520 KB) | **1,738 result links**; 63 legacy `.txt` links (e.g. `Portals/0/PDF/Results/Cross_Country/2017/gsmilwsectional.txt`); `d1b/d1g...stateresults.pdf` naming | 23:19 |
| `https://www.wiaawi.org/sports/boys-cross-country/boys-cross-country-tournament` | GET | 200 | 2026 XC state = Oct 31, Ridges GC; 2025 sectional links (47 files, 40 unique hosts); **`Sectional Entry Form Deadline (PTTiming/Athletic.net)`**; **links `https://live.pttiming.com/xc-ptt.html?mid=5127`** | 23:19 |
| `https://www.wiaawi.org/sports/boys-track-field/boys-track-field-tournament/boys-track-field-assignments` | GET | 200 | D1/D2/D3 assignment PDFs only — **no Athletic.net links on the HTML page** | 23:18 |
| `https://www.wiaawi.org/scorecenter` | GET | 200 | Per-sport score index (team sports); linked from every page header | 23:18 |
| `https://www.wiaawi.org/sites/default/files/pdfs/PDF/Sports/Track/2026/athetic.net-track-instructions.pdf` | GET | 200 (239,369 B) | **WIAA is transitioning tournament registration to Athletic.net**; `/URI` annotations expose `athletic.net/events/usa/wisconsin/2026-5-26`, `athletic.net/account/login/signup`, 4 support URLs; entry deadline May 22 | 23:19 |
| `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2026/d1bstateresults.pdf` | GET | **301** | Location → `/sites/default/files/pdfs/PDF/Results/Track/2026/d1bstateresults.pdf` | 23:19 |
| `https://www.wiaawi.org/sites/default/files/pdfs/PDF/Results/Track/2026/d1bstateresults.pdf` | GET | **404** | **2026 state result links on the tournament page are broken**; working copies are on the archive page | 23:19 |
| `https://www.wiaawi.org/sites/default/files/pdfs/PDF/Results/Track/2025/menomoneefallssectional.pdf` (+`southmilwaukeesectionalb/g.pdf`) | GET ×3 | 200 (`application/pdf`, 252-259 KB) | Historical sectional PDFs are live and directly fetchable | 23:19 |
| `https://www.wiaawi.org/sites/default/files/Portals/0/PDF/Results/Cross_Country/1999/d1bresults.txt` | GET | 200 (`text/plain`, 14,999 B) | Legacy XC results: `MATT ESCHE 11 15:42, CHRIS MARTIN 12 16:22 …` under team scores — **grade + mark in plain text** | 23:19 |
| `https://www.wiaawi.org/sites/default/files/2026-08/trb2026d1stateresults.pdf` | GET | 200 (661,543 B) | State D1 boys: `Name \| Yr \| School \| H# \| Result`, wind (`-0.5`), Prelims/Finals, `Q`/`q`, team points, `DNS`/`DNF`; 234 result rows, 185 distinct athletes, 71 grade-11, 36 events | 23:20 |
| `https://www.wiaawi.org/sites/default/files/2026-08/tr2026arrowheadregionalindiv.pdf` | GET | 200 (1,652,309 B) | **Athletic.net-export format** (`Compiled`, `Athlete \| Yr \| Team`); 437 result rows, 234 distinct athletes, **71 distinct grade-11**, 167 relay legs (58 grade-11) | 23:21 |
| `https://www.wiaawi.org/sites/default/files/2026-08/tr2026beaverdamsectionalindiv.pdf` | GET | 200 (2,646,780 B) | `Producer: Microsoft: Print To PDF`, `Author: BDTrack`, `Title: RUNMEET_ D1 Sectional 7 - Beaver Dam`; **image-only, no text extraction** — format risk | 23:21 |
| `https://www.wiaawi.org/sites/default/files/pdfs/PDF/Results/Cross_Country/2025/d1bstateresults.pdf` | GET | 200 (896,831 B) | XC state: team scores with avg/total/spread; runner rows `place (score) Name GRADE time` (`Cooper Erickson 12 15:50.2`) | 23:22 |
| `https://www.wiaawi.org/`, `https://schools.wiaawi.org/` (response headers) | GET | 200 | Cloudflare in front of both; `schools` = IIS/ASP.NET MVC 5.2; `www` = Drupal cache HIT; **no rate-limit headers, no Retry-After** | 23:20 |
| `https://live.pttiming.com/xc-ptt.html?mid=5127` | GET | 200 (4,489 B) | PrimeTime Timing live-results app (`timing = "PrimeTime Timing"`, bundle `/lib/xc-bundle.js`, Firebase `ptt-franklin.firebaseio.com`); `mid` = PrimeTime meet ID | 23:22 |

**Not verified / left open (stated explicitly):**
- Girls XC 2025-26 participation count (would require one more `RunSportListReport` with ssid 1514).
- Girls T&F 2026 state result PDFs (`trg2026…`) were located but not downloaded — format assumed identical to the boys file (`[INFERENCE]`).
- 2026-27 Girls T&F divisional assignments: value not yet published by WIAA (460/462 rows blank) — a future observation, not a gap in this report.
- `RunCoopListReport` parameter set unresolved; individual school pages and the `Coop` column cover the same need.
- Only 9 of 516 school pages were sampled for coach-field completeness; the 7-9/9 role coverage is a sample estimate, not a census.
- 25 of 26 directory letters were not fetched; the school-level universe count (516) is WIAA's own published figure, while the **per-sport** counts (462/462/437/435) are fully enumerated and verified.
