# 21. Missouri MSHSAA

Status: complete
Observed on: 2026-09-19

Scope note: this report covers `www.mshsaa.org` only — the association side (school universe, classification,
coach/AD publication, and the association's own track/XC result surfaces). The Missouri timer ecosystem that
MSHSAA points at (PrimeTime Timing, `pttiming.com`) is assignment 22; MileSplit/DirectAthletics indexes are
assignments 27–28. Where MSHSAA *names* those providers, that is recorded here as a hand-off, not re-researched.

---

### Source

- **Provider**: Missouri State High School Activities Association (MSHSAA), 1 N Keene St, Columbia, MO 65201.
- **Primary host**: `https://www.mshsaa.org/` (canonical `www`; apex `mshsaa.org` 301s/serves the same site).
- **Stack**: Kentico CMS over ASP.NET WebForms (`.aspx`, `__VIEWSTATE`/`__EVENTTARGET` postbacks). No public JSON API.
- **Robots policy**: `https://www.mshsaa.org/robots.txt` — `<user-agent: *>` disallows `/Ajax/`, `/Data/`,
  `/Utilities/`, `/Config/`, `/Modules/`, `/Accounting/`, `/Admin/`, `/JS/`, `/Style/`, `/SchoolsAdmin/`,
  `/Officials/Admin/`, `/Officials/Ajax/`, `/VIPs/`, `/MyAccount/`, `/Resources/UploadedFiles/`,
  `/Resources/Home/Rotating/`, **`/MySchool/Matchup.aspx*`**. `Amazonbot`, `GPTBot` and `meta-externalagent`
  are disallowed entirely. Declared `sitemap: https://www.mshsaa.org/sitemap.xml`.
  **The sitemap is empty** — it returns 44 bytes containing only `<?xml version="1.0" encoding="utf-8" ?>` and
  zero `<loc>` entries, so site enumeration must go through the HTML indexes below.
  *Compliance note: `/MySchool/Matchup.aspx` is disallowed. One Matchup page was fetched before robots.txt was
  read; it is documented here for completeness and **must be excluded from any collector**. All other paths used
  in this report are robots-allowed.*
- **Association-side result partners named by MSHSAA**: PrimeTime Timing (`https://www.pttiming.com/`, "Live
  Results") and Athletic.net ("Official Results", T&F championships only).

### Coverage

- **State**: Missouri only. No other state's schools appear.
- **Levels**: High School and Junior High are both modelled (`ScheduleDownload` radio values 1 = High School,
  2 = Junior High; alg ids below are distinct per level). "Combined Schools" (orgtype 3) are 7–12 style
  buildings that field HS teams.
- **Sports in scope**: Cross Country (`activity=5`, fall) and Track & Field (`activity=19`, spring). Activity
  ids verified from the schedule-download form radios: `rblActivity` values observed `1,2,3,5,8,10,12,15,16,19,20,23`
  where `5` = Cross Country and `19` = Track and Field (labels rendered adjacent to each radio).
- **Season depth**:
  - School listing / classification / enrollment: current year (2026-2027) plus an "include archived schools"
  toggle; enrollment history columns back to **2023-2024** on one page.
  - Postseason result pages: year selector **2001-2002 → 2026-2027** (25 options; note 2019-2020 is absent from
    the list — the selector jumps 2020-2021 → 2018-2019).
  - School schedules: interactive selector 2022-2023 → 2027-2028 (6 years); the schedule-download form offers
    **2005-2006 → 2026-2027** (22 years).
  - Individual champions history: **1926 → 2026 (101 years)**, 6,163 records.
- **Indoor**: MSHSAA sanctions no indoor track season. The T&F activity page has a Spring season section only
  (`data-season='3'`). No indoor surface exists on mshsaa.org. [INFERENCE: Missouri indoor T&F is not an MSHSAA
  championship season; indoor meets would only appear as school-scheduled events, and none were seen.]

### Enumeration

#### Schools — yes, complete

`/Schools/SchoolListing.aspx` (one GET, 721 KB HTML) renders the entire member/affiliate listing with data
attributes, no pagination, no postback required.

Parsed totals (2026-2027 listing, `Observed on: 2026-09-19`):

| Organization type (`data-organizationtype`) | Rows |
|---|---|
| `1` = High Schools | 316 |
| `3` = Combined Schools | 330 |
| `2` = Junior High Schools | 360 |
| **total listed organizations** | **1,006** |

Legend proven from the filter checkbox block itself:
`cblOrganizationType_0 value="1" → "High Schools"`, `cblOrganizationType_1 value="3" → "Combined Schools"`,
`cblOrganizationType_2 value="2" → "Junior High Schools"`.

| Affiliation (`data-membertype`) | Rows |
|---|---|
| `member` (Full Member) | 719 |
| `affiliate` (Affiliate Registered) | 257 |
| `hsa` (Home School Associations) | 30 |

**High-school-level universe = orgtype 1 + 3 = 646 organizations**, of which **592 are full members**
(262 High Schools + 330 Combined) and 54 are affiliate/home-school (35 + 19).

Each row carries: school name, affiliation text, county, city, and a detail link. Link target varies:
719 rows use `../MySchool/?s=<id>` and 287 rows use `../Schools/Navigation.aspx?s=<id>`.
School-id space: **1,006 unique ids spanning 2 … 1,913, i.e. 47.4 % of the range is unused** (gaps begin at
11, 31, 32, 33, 41, 49 …) — archived/retired ids. **Enumerate from the listing page; never iterate the id space.**

Other school-level pages: `/Activities/SchoolEnrollments.aspx` (school name + total enrollment 2023-24 →
2026-27, 143 KB, no ids), `/Activities/ActivityEnrollmentBreaks.aspx` (see teams), `/About/MemberSchools.aspx`
(link labelled "View Map", not fetched).

#### Teams — yes, counted per class for both sports

`/Activities/ActivityEnrollmentBreaks.aspx?year=<YYYY>` renders, for every sport, the enrollment break per class
plus a parenthesised team count. The grey `(n)` figure is documented on the page as *"the number of teams in each
classification"*; `(n/m)` adds MSIP-Exempt schools shifted into the class by championship factor.

**2025-2026 (most recent completed season), team counts by class:**

| Sport | C1 | C2 | C3 | C4 | C5 | Total |
|---|---|---|---|---|---|---|
| Cross Country – Boys | 112 | 103+4 | 88+2 | 70+4 | 70+1 | **454** |
| Cross Country – Girls | 93 | 94+2 | 85+1 | 68+6 | 72+5 | **426** |
| Track and Field – Boys | 157 | 112+2 | 92+3 | 77+4 | 77+3 | **527** |
| Track and Field – Girls | 155 | 115+2 | 91+1 | 77+3 | 77+5 | **526** |

Enrollment breaks, 2025-2026 — T&F (B and G share breaks): C1 ≤119, C2 120–237, C3 238–491, C4 492–992, C5 993+.
XC: C1 ≤128, C2 129–253, C3 254–514, C4 515–1035, C5 1036+.

**2026-2027 (current), XC only** (T&F breaks "Release Date: TBD"): C1 115/108, C2 110/102, C3 86/84, C4 71/68,
C5 69/70 → **XC Boys 451, XC Girls 432** team slots. Breaks: C1 ≤126, C2 127–265, C3 266–525, C4 526–1000, C5 1001+.

So a Missouri collector can bound the team universe exactly without touching any other source:
≈ **527/526 T&F teams and ≥426/432 XC teams** (the XC boys figure moved 454 → 451 between the two seasons, so
treat these as season-scoped, not constants).

#### Meets — yes, per school; no association-wide meet index

There is **no MSHSAA meet calendar for XC/T&F**. Proof: `/Activities/Scoreboard.aspx` — the "Select Sport/Activity"
list contains Football, Baseball (×2), Basketball (B/G), Soccer (B/G), Softball (×2), Stunt, Tennis (B/G),
Volleyball (B/G) and **does not contain Cross Country or Track and Field**. MSHSAA's scoreboard covers only
team-score sports.

Meets are enumerable only via school schedules:

```
GET https://www.mshsaa.org/MySchool/Schedule.aspx?s=<schoolId>&alg=<alg>&year=<YYYY>
```
- `year` is the *ending* year of the school year: `2025` = 2025-2026, `2026` = 2026-2027.
- The page is fully server-rendered and the `year` query parameter works over GET (verified), even though the
  on-page dropdown requires a postback.
- Each row gives date (or date range), home/away, opponent or invitational name, level, and — where MSHSAA has
  one — a `Matchup` link (robots-disallowed; see Source).
- The page header also states the school's current **class + district** for that sport, e.g. Adrian HS
  (`s=3`) Boys XC: `Class 2 District 2` for 2026-2027 and `Class 2 District 4` for 2025-2026.

Verified example (Adrian HS, `s=3`, `alg=11`, 2026-2027): 11 dated rows — 9/5 at Bolivar, 9/12 at Knob Noster,
9/19 MSSU, 9/22 at Stockton, 9/25-26 Gans Creek Classic, 9/29 Warsaw Invitational, 10/3 Grain Valley Invitational,
10/10 Butler Invitational, 10/15 OHC Conference Meet, 10/20 Eldo Bulldog Meet, 10/31 Class 2 District 2 Tournament.

**Cost of a full meet sweep**: 1 request per school per sport per year. Whole-state XC (≈450 schools × 2 genders
is not needed — one `alg=11` call per school already lists the boys schedule; a second for girls) ⇒
≈ 646 × 2 = 1,292 requests for one sport-year if all HS-level organizations are walked. Use the class/district
assignment pages first to restrict to schools that actually sponsor the sport (below), reducing XC to ~454+426
school-sport pages and T&F to ~527+526.

A `/MySchool/ScheduleDownload.aspx?s=<id>` form exists (year 2005-2006 → 2026-2027, level HS/JH, sport). Direct
GET returns only the form; a reconstructed WebForms postback (`__EVENTTARGET=ctl00$contentMain$rblActivity$2`,
activity=5, year=2025) returned MSHSAA's own "Our Apologies / Page Error" page. **Do not build on this form** —
the per-sport `Schedule.aspx` GET is the reliable path.

#### Athletes — only at the state-championship layer

Two athlete-bearing surfaces exist, both championship-scoped:
1. **State results PDFs** (see Result evidence) — full name, school, grade, event, mark, place, round.
2. `/Activities/IndividualChampions.aspx?alg=<52|53>` — one HTML page, 6.4 MB, **6,163 champion records,
   101 years (1926–2026), 605 distinct schools, 3,834 distinct student names**. Recent seasons carry 95 records
   each (19 events × 5 classes). Columns: `Year | Class | Student | School | (wind marker) | Mark`. Relay events
   appear as a single row whose "Student" cell is the literal text `Relay` followed by the school (40 such rows in
   the boys table) — **no relay leg names**.

There is **no roster, no eligibility list, and no athlete directory on public MSHSAA pages**. The Athletic.net
setup PDF states the eligibility roster is uploaded from MSHSAA into Athletic.net, but the roster itself is
behind `/MyAccount/` (robots-disallowed, authenticated).

#### Class of 2027 — yes, at the state layer, from the grade column

State result PDFs are Hy-Tek "MEET MANAGER" output and carry a `Year` column that is the athlete's grade in that
season. Season 2025-2026 `Year = 11` ⇔ Class of 2027. Measured on the two PDFs fetched:

| PDF | rows parsed | unique athletes | grade distribution | `Year=11` (Co2027) |
|---|---|---|---|---|
| 2026 T&F Class 1 Boys state | 125 performance rows | 92 | 9:6, 10:25, **11:48**, 12:46 | **33 unique** |
| 2025 XC Class 1 Boys state | 175 | 175 | 9:49, 10:53, **11:39**, 12:34 | **39 unique** |

Extrapolating across 5 classes × 2 genders per sport gives [INFERENCE] roughly **300–350 Co2027 athletes per
sport per season** from state meets alone. That is real but is a small fraction of the ~450–530 teams per sport:
it discovers *state qualifiers*, not the population. District/sectional rounds — where the rest of the population
lives — are **not published by MSHSAA** (next section).

#### Results — state finals only

MSHSAA publishes **only the state championship round**. Evidence: `/Activities/SectionalResults.aspx` is not a
results page at all; it is a site/manager page (`Site Information`, `Manager Details`), and for 2025-2026 as well
as 2019-2020 it renders `No Site Information Entered` / site name with no result or PDF iframe. District rounds
are likewise site-info only (`DistrictSite.aspx`). The state round resolves as follows:

```
GET /Activities/PostseasonResult.aspx?alg=<52|53|11|12>&class=<1..5>&year=<YYYY>   → class view
GET /Activities/PostseasonResult.aspx?alg=<52|53|11|12>&year=<YYYY>                → 302-equivalent redirect to ?id=<groupId>
GET /Activities/PostseasonResult.aspx?alg=<52|53|11|12>&id=<groupId>               → the result page
```
The `id=` form is the canonical result page. It embeds the result as a PDF in an iframe:
`/JS/plugins/PDFJS/web/viewer.html?file=/resources//ChampionshipResults/<file>.pdf`.

Verified artifacts:

| Result | iframe PDF path | size |
|---|---|---|
| 2026 Class 1 Boys T&F Individual Results (`alg=52&id=2095`) | `/resources//ChampionshipResults/Boys-Track-and-Field-2026-Class-1-Boys-Track-and-Field-Individual-Results-639155002143527273.pdf` | 387,275 B, 23 pages |
| 2025 Class 1 Boys XC Championship Results (`alg=11&id=2030`) | `/resources//ChampionshipResults/Boys-Cross-Country-2025-Class-1-Boys-Cross-Country-Championship-Results-638981913475921342.pdf` | 320,210 B |

The trailing numeric segment is a .NET `DateTime.Ticks` stamp (`639155002143527273` ≈ 2026-06,
`638981913475921342` ≈ 2025-11). **It is not derivable — read the iframe `src` from the `id=` page, then fetch the
PDF.** Group-id sets observed on the 2025-2026 pages: T&F boys `alg=52` → `2095, 2096, 2100, 2101, 2107, 2113`
(class links plus an "Individual Championship Results" / "Championship Team Scores" pair); XC boys `alg=11` →
`2024, 2026, 2028, 2030, 2032`. Historical ids present in the same pages run `607 … 2113` (T&F) and
`439 … 2032` (XC), i.e. the archive is addressable by id.

**Complete postseason result-surface catalog.** Every track/XC postseason surface reachable from
`/Content/TrackandField/Home.aspx`, `/Content/CrossCountry/Home.aspx` and the global MSHSAA footer nav, with its
exact URL pattern and what it actually contains. `FETCHED` = retrieved and parsed for this report; `LINKED` =
URL pattern confirmed in the anchor markup of a fetched page but the page itself not opened (no new evidence
claimed).

| Surface | URL pattern | Contents | State |
|---|---|---|---|
| Postseason results hub (per sport/gender/class) | `/Activities/PostseasonResult.aspx?alg={11,12,52,53}&class={1..5}&year=<YYYY>` | class selector, year selector, iframe PDF of that class's state result | FETCHED |
| Postseason results by group id | `/Activities/PostseasonResult.aspx?alg={11,12,52,53}&id=<groupId>` | canonical result page; individual-champions and team-scores groups | FETCHED |
| State result artifact | `/resources/ChampionshipResults/<Sport>-<Year>-<Class>-…-<ticks>.pdf` | Hy-Tek MEET MANAGER PDF: prelims/finals, wind, heat, grade, marks, places | FETCHED |
| Sectional site/manager info | `/Activities/SectionalResults.aspx?alg={52,53}&class={1..5}&sectional={1..4}&year=<YYYY>` | **site + manager only — no results, ever** | FETCHED |
| District site/manager info (XC) | `/Activities/DistrictSite.aspx?alg={11,12}&class={1..5}&district={1..4}&year=<YYYY>` | host school, date, manager | FETCHED |
| Class & district assignments (school ↔ class ↔ district) | `/Activities/ClassAndDistrictAssignments.aspx?alg={11,12,52,53}&class={1..5}&year=<YYYY>` | per-district school lists, host site, host manager | FETCHED |
| Individual champions history | `/Activities/IndividualChampions.aspx?alg={52,53}` | 6,163 rows, 1926–2026, Year/Class/Student/School/wind/Mark | FETCHED |
| All-time individual champions | `/Activities/AllTimeIndividualChampionsHistory.aspx` | linked from global nav | LINKED |
| State championship history (per alg) | `/Activities/StateChampionships.aspx?alg={11,12,52,53}` | linked from both sport home pages | LINKED |
| All-time team championship history | `/Activities/AllTimeTeamChampionshipHistory.aspx` | linked from global nav | LINKED |
| Championship site record book | `/Activities/ChampionshipSiteRecordBook.aspx?activity={5,19}&gender={1,2}` | per-site state-meet records | LINKED |
| School playoff/postseason history | `/Activities/SchoolPlayoffHistory.aspx?alg={11,12,52,53}` | linked from global nav | LINKED |
| District/sectional schedules (site lists) | `/Content/TrackandField/DistrictSchedules.aspx`, `/Content/TrackandField/SectionalSchedules.aspx` | host-site schedule pages | LINKED |
| District tournaments & results | `/Activities/DistrictTournaments.aspx`, `/Activities/DistrictWinners.aspx` | global-nav index; team-score sports oriented | LINKED |
| Season records | `/Activities/SeasonRecords.aspx` | global-nav index | LINKED |
| Open dates (meets seeking opponents) | `/Activities/OpenDates.aspx?activity={5,19}` | schedule coordination only | LINKED |
| Sanctioned events | `/About/SanctionedEvents.aspx?activity={5,19}` | non-MSHSAA events | LINKED |
| Championship broadcasts | `/Activities/Broadcasts.aspx`, `https://mshsaa.tv/?S=mshsaachampionships&B=<id>` | video; ids `3014771`/`3014778` observed on the XC info page | LINKED |
| **Athletic.net official T&F results** | `https://www.athletic.net/track-and-field-outdoor/usa/high-school/missouri/mshsaa` | the delegated host for T&F official results | FETCHED → **403 Cloudflare** |

PDF content quality (2026 Class 1/2/3 T&F, `5/22/2026 to 5/23/2026`, Adkins Stadium – Jefferson City High School):
`PrimeTime Timing - Contractor License  Hy-Tek's MEET MANAGER 5:05 PM 5/27/2026`; sections `Preliminaries` and
`Finals` with `Wind` and `H#` columns, and a converted-time line under each finalist, e.g.
`1 Tanner Briddle 12 Tina-Avalon 10.96 0.6 10 / 10.956 (10.956)`.
XC equivalent (2025 state, `11/7/2025 to 11/8/2025`, Gans Creek Cross Country Course):
`Event 2  Boys 5k Run CC Class 1` with `Name / Year / School / Finals / Points`.

### Stable identifiers

There are **no athlete, result, meet or event ids** on any public MSHSAA surface. The identifiers that do exist:

| Identifier | Surface | Form | Verified example |
|---|---|---|---|
| **School ID** | `MySchool/?s=<id>`, `Schedule.aspx?s=<id>` | integer, 2…1,913, 47.4 % sparse | Page-verified: `s=3` = Adrian HS, `s=95` = Kirkwood HS. Listing-row-verified (id↔name binding read from `SchoolListing.aspx` rows, page not fetched): `s=953` = Battle HS, `s=14`/`s=15` = Blue Springs / Blue Springs South HS |
| **Org detail (alt)** | `Schools/Navigation.aspx?s=<id>` | same id space | `s=1778` = Al-Salam College Preparatory HS (listing-row binding) |
| **Activity ID** | `activity=<id>` (record books, sanctioned events, schedule form) | integer | `5` = Cross Country, `19` = Track and Field |
| **Alg ID** | `alg=<id>` — the workhorse parameter | integer | `11` = HS XC Boys, `12` = HS XC Girls, `13` = JH XC Boys, `14` = JH XC Girls, `52` = HS T&F Boys, `53` = HS T&F Girls, `54` = JH T&F Boys, `55` = JH T&F Girls |
| **Season/year** | `year=<YYYY>` | ending year of school year | `2025` = 2025-2026 season (state meet spring/fall 2026 → labeled "2026" in result titles) |
| **Result group ID** | `PostseasonResult.aspx?...&id=<id>` | integer, season-scoped | `id=2095` = 2026 Class 1 Boys T&F Individual; `id=2030` = 2025 Class 1 Boys XC |
| **Championship PDF** | `/resources/ChampionshipResults/...-<ticks>.pdf` | .NET ticks suffix | `...-639155002143527273.pdf` |
| **Class / District** | on `Schedule.aspx` header and `ClassAndDistrictAssignments.aspx` | `Class <1..5> District <1..8>` (T&F) / `District <1..4>` (XC) | Adrian Boys XC: `Class 2 District 2` (2026-27), `Class 2 District 4` (2025-26) |
| **Competition ID** | `MySchool/Matchup.aspx?...&comp=<id>` | integer | `comp=2767559` — **robots-disallowed, do not collect** |

`alg` id verification basis: the T&F home page renders `<li><a data-section="52">Boys</a><li><a data-section="53">Girls</a>`;
the XC home page links `ClassAndDistrictAssignments.aspx?alg=11…` and `alg=12…` in its Boys/Girls sections; and the
schedule page's plain-text link list contains the literal labels `HS Cross Country - Boys → alg=11`,
`HS Cross Country - Girls → alg=12`, `HS Track and Field - Boys → alg=52`, `HS Track and Field - Girls → alg=53`.
Three independent pages agree. **`alg` must be treated as the sport+gender+level key, not as a class key** — class
is a separate `class=` parameter.

### Athletic.net leverage

#### Direct links published by MSHSAA

A full-text scan of every page fetched for this report found **exactly two** `athletic.net` references:

1. `https://www.mshsaa.org/resources/Activities/CrossCountry/Athletic.net%20helpful%20links.pdf` — linked from
   **both** the T&F home page and the XC home page under an `Athletic.Net` icon (the anchor image is
   `resources/Activities/CrossCountry/athletic.net logo.png`; the adjacent `alt` text reads `MileSplit` — an
   MSHSAA authoring error, the href is the Athletic.net PDF).
2. `https://www.athletic.net/track-and-field-outdoor/usa/high-school/missouri/mshsaa` — on the **Track & Field
   Championship Information Central** page (`/Content/TrackandField/InformationCentral.aspx`), labelled
   **"Official Results - AthleticNet"**, directly beneath `Live Results - PrimeTime Timing` (→ `https://www.pttiming.com/`).

That URL is the official Athletic.net state-series page for Missouri. **It could not be loaded from this machine:
HTTP 403, Cloudflare "Just a moment…" interstitial, 5,593 bytes** — consistent with the block recorded in the
mission brief and in the repo (`HANDOFF.md`: `GetNavInfo`/`GetRankings` both return the 403 block page, 5,484 bytes).
**No `MeetID`, `TeamID`, `AthleteID` or result deep link is exposed anywhere on mshsaa.org.**

The XC Championship Information Central page does **not** use Athletic.net — its "Results" block links back to
`PostseasonResult.aspx?alg=11&class=<1..5>&year=2025` (the MSHSAA PDFs). So for cross country MSHSAA is the
official result host; for track & field it delegates the public/official result presentation to Athletic.net.

#### The roster upload (highest-leverage fact)

The Athletic.net PDF states, verbatim:

> "NOTE - Your eligibility roster from MSHSAA will automatically be uploaded to Athletic.net. Only athletes who
> were included on the roster import from MSHSAA will be designated as eligible on Athletic.net and be able to be
> registered for meets. As eligibility restarts each season, it is essential that coaches work with their Athletic
> Directors to enter accurate eligibility rosters on the MSHSAA website in order for their Athletic.net rosters to
> accurately reflect eligible athletes for the season."

The PDF's contents list is the MSHSAA→Athletic.net workflow: create account, request team page access, invite
coaches, add meet to team calendar, submit XC entries, host a meet, add timer/event manager, add non-meet events.
Support contact: `support@athletic.net`.

**Consequence for acquisition**: MSHSAA's school set *is* the authoritative seed for Missouri Athletic.net team
pages, and MSHSAA's class/district assignment pages say exactly which schools sponsor which sport in which season.
That converts "search Athletic.net for Missouri team pages" into "fetch a known list of ≤646 school names and
resolve them against Athletic.net team pages".

#### Can MSHSAA deterministically seed an Athletic.net lookup?

| Target | Seeded? | Basis |
|---|---|---|
| Athletic.net **team** pages | **Yes**, by (school name, city, sport, gender, season) | 646 HS-level orgs with name + city + county, plus per-season sponsorship from the class/district/team-count pages |
| Athletic.net **meet** IDs | **No** — only by (school, sport, date, meet name) from `Schedule.aspx` | The state-series page is published but no `MeetID` is exposed; the URL carries no numeric id |
| Athletic.net **athlete** links | **No** for the general population; **Yes** for state qualifiers via (name, school, grade, event) | State PDFs give a grade-bearing, school-scoped identity to match against an Athletic.net meet entry list |
| Athletic.net **result** links | **No** | — |

#### Request-avoidance estimate

[INFERENCE, arithmetic shown so it can be checked]

- **Team discovery**: one `SchoolListing.aspx` GET (1 request) + one `ActivityEnrollmentBreaks.aspx?year=` GET
  per season + `ClassAndDistrictAssignments.aspx` at 5 requests per sport/gender/year gives a complete, named
  Missouri team target list. Replacing Athletic.net-side team search for ~646 schools with 1–12 MSHSAA requests
  eliminates on the order of **one Athletic.net navigation/POST per candidate school**, i.e. **hundreds to ~650
  Athletic.net requests per season**.
- **State-finalist results**: 10 T&F PDFs (5 classes × 2 genders; individual + team splits) and 10 XC PDFs cover
  ~300–350 Co2027 athletes per sport with grade evidence, for **20 MSHSAA requests per season**. The equivalent
  from Athletic.net is one athlete-history acquisition per athlete — the retained consolidated pipeline figures
  (142,705 athletes across 12 states, so ~12k athletes per state) put a Missouri state-finalist set of ~600–700
  athletes at a **low-single-digit percentage of the state population** but at **zero Athletic.net requests**.
- **Honest ceiling**: MSHSAA does **not** replace Athletic.net rankings discovery for the general Missouri
  population. Its value is (a) eliminating Athletic.net *school/team* search entirely, and (b) supplying
  grade-verified state-finalist results that act as an independent control set for whichever state-wide source
  (MileSplit MO / DirectAthletics / PrimeTime) is used as the population source.

### Athlete evidence

Availability on public MSHSAA surfaces (state-result PDFs and champions history only):

| Field | Available | Evidence / limit |
|---|---|---|
| Name | **Yes** | `Tanner Briddle`, `Cooper Clair`, `Kellen Robertson` in the PDFs; `Student` column on champions page |
| Graduating class / grade | **Yes**, at state meets | PDF `Year` column, values 9–12; 6,163 champion rows have **no** grade column |
| School | **Yes** | PDF `School` column is truncated to ~12 chars (`St. Joseph C`, `Meadow Heights`, `Northeast (Cairo)`) — resolve via the school directory, do not treat the abbreviation as a key |
| City / state | **No** in results; **yes** in directory | `SchoolListing.aspx` gives city + county per school; state is always MO |
| Gender / category | **Yes** implicitly | Separate PDFs/pages per gender (`alg=52` boys vs `alg=53` girls) |
| TF/XC distinction | **Yes** | Separate activities (`activity=19` vs `5`); separate PDFs |
| Indoor / outdoor | Outdoor only | MSHSAA has no indoor season; T&F activity is Spring, XC is Fall |
| Performances | **Yes**, state meet only | Prelims + finals marks, 3-decimal converted times, field-event marks |
| PRs | **No** | Single meet per PDF; no season aggregation |
| Progression | **No** | — |
| Meets | **Yes**, names/dates as schedule rows; **no results** | `Schedule.aspx` rows |
| Athlete profile URL | **No** | No athlete page exists on mshsaa.org |

### Recruiting information

**MSHSAA publishes no coach or athletic-director information on any public page reachable without authentication.**

Negative evidence (this is a finding, not an omission):
- `/MySchool/?s=3` (Adrian HS) — a full-text search of the 190 KB page for `Coach`, `Athletic Director`,
  `Principal`, `Administrator`, `Staff`, `Phone` returns **0 hits**. The only `mailto:` is MSHSAA's own
  `email@mshsaa.org`.
- `/MySchool/?s=95` (Kirkwood HS) — same search on a 217 KB page returns **0 hits** for `Coach|Athletic Director`.
- `/Schools/` index: searches for `Athletic Director`, `Coach`, `Directory` return **0 hits**. Its "Member School
  Links" block lists only: Enrollment Data, Sport/Activity Enrollment Breaks, Championship Factor, Member School
  Listing.

What *is* published, usable as role-level contact signals:

| Field | Available | Where | Caveat |
|---|---|---|---|
| Head track coach | **No** | — | not published |
| Head XC coach | **No** | — | not published |
| Assistant coaches | **No** | — | not published |
| Athletic director | **No** on school pages | — | MSHSAA's AD-facing material (see below) confirms ADs exist as a role but publishes no directory |
| Public professional email | **No** school-level email at all | Only MSHSAA staff `email@mshsaa.org` | do not infer school email patterns |
| School website | **Yes** | `MySchool/?s=<id>` → `ctl00_SchoolHeader_aMySchoolWebsite`, e.g. Adrian HS → `https://www.adrian.k12.mo.us/` | a stable, scriptable field; feeds the coach-contact chain in assignment 29 |
| Team website | **No** | — | — |
| **District/sectional host manager** | **Partly** | `ClassAndDistrictAssignments.aspx?alg=…&class=…&year=…` and `DistrictSite.aspx` / `SectionalResults.aspx` | Name + host school + phone, published for the *host-site manager* role, e.g. `Mike Schmidli / Princeton High School / (660) 748-3211`, `Cree Beverlin / Worth County High School / (660) 564-2218`, `Julie Ward` (XC D1 host, Oak Ridge) |

⚠ **Privacy stop-sign (record, do not ingest).** The district/sectional host blocks render a *mailing address*
field, and at least one observed value is a residential-looking street address rather than a school address
(sectional 1 Boys T&F manager block: `Tony Brandt / 10135 State Rd C / Mokane, MO 65059-0038 / (573) 676-5225 x7`).
Per the mission privacy contract, only the school-attributed professional contact may be retained; **the address
field must be dropped by any parser**, and manager names should be used only as a corroborating role signal, never
as the coach of record.

Roles named by MSHSAA but not published as a directory: MIAAA (Missouri Interscholastic Athletic Administrators
Association, `http://www.miaaamo.org`, linked from every MSHSAA footer) is the AD association; MSHSAA runs
"New AD Training" and "Annual Membership Update" events. **The Missouri AD/coach directory is therefore a
MIAAA / school-district task, not an MSHSAA task** — hand off to assignment 29.

### Result evidence

For the state-championship PDFs (the only MSHSAA result artifacts):

| Field | Available | Evidence |
|---|---|---|
| ResultID | **No** | PDF rows are unkeyed |
| AthleteID | **No** | name-only (`Tanner Briddle`) |
| MeetID | **No** | addressed by `PostseasonResult.aspx?id=<groupId>` at page level |
| EventID | **No** | event as text (`Boys 100 Meter Dash Class 1`); the champions page exposes event *tabs* with numeric `data-event` values 17–53, but those are page filters, not a result key |
| Mark | **Yes** | `10.96`, `16:16.0`, `10.956 (10.956)` |
| Normalized mark inputs | **Yes** | Hy-Tek prints the raw and converted time on the line below each finalist: `10.956 (10.956)`; XC `16:16.0` |
| Timing method | **Yes (provenance)** | every PDF header is `PrimeTime Timing - Contractor License  Hy-Tek's MEET MANAGER <time> <date>`; FAT/timing system is PrimeTime's, not declared per mark |
| Wind | **Yes** | T&F prelims and finals carry a `Wind` column with signed values (`0.1`, `1.1`, `-0.1`, `0.6`) |
| Implement / hurdle spec | **Partial** | event name only (`Shot Put`, `110-Meter Hurdles`, `Discus`, `Javelin`). No weight/height specification in the result rows; MSHSAA publishes the specs separately as static PDFs on the T&F home page (`resources/activities/trackandfield/{high jump,long jump,triple jump,pole vault,shot put,discus throw,javelin throw}.pdf` and `StartingHeights.pdf`) |
| Heat / round | **Yes** | `Preliminaries` / `Finals` headers and a `H#` heat column (`10.97Q 0.1 2`) |
| Place | **Yes** | leading place column, plus `Q`/`q` qualifier flags in prelims |
| Date | **Yes** | PDF title (`5/22/2026 to 5/23/2026`) and per-row page headers |
| School represented | **Yes** | `School` column (abbreviated, ~12 chars) |
| Relay membership | **No** | relay rows are team-level only (`Relay  Knox County  44.25`); no leg names, no splits |

For the champions-history page: Year, Class, Student, School, wind marker, Mark — **no place, round, heat, date
within season or meet name**, and no grade.

**Gap**: district and sectional rounds produce no result artifact on mshsaa.org, so the bulk of Missouri
participants have no MSHSAA result row at all.

### Incremental use

Weekly/daily collection design that avoids any historical re-fetch:

1. **Season bootstrap (once per sport-season).** One GET `/Activities/SchoolListing.aspx` → the 646 HS-level
   organizations with ids, cities and counties. One GET `/Activities/ActivityEnrollmentBreaks.aspx?year=<YYYY>`
   → team counts and enrollment breaks per class. 5 GETs
   `/Activities/ClassAndDistrictAssignments.aspx?alg=<11|12|52|53>&class=<1..5>&year=<YYYY>` → the exact school
   set per class/district for that sport and gender. This is the season's target list; store it, don't refetch.
2. **Team/meet calendar (once per school-season, then diff).** `MySchool/Schedule.aspx?s=<id>&alg=<alg>&year=<YYYY>`
   is server-rendered and stable. Cache the parsed rows keyed by `(schoolId, alg, year, date, opponent)`; a weekly
   pass only needs to re-fetch schools whose schedule can still change (in-season) and diff the row set. New rows
   are new meets; the class/district line in the header is itself a diffable field (it changed Adrian Boys XC
   from `Class 2 District 4` in 2025-26 to `Class 2 District 2` in 2026-27).
   *Do not collect `MySchool/Matchup.aspx` (robots-disallowed).*
3. **Postseason results (tiny, poll-friendly).** New results appear only at the state meet.
   - Detect: GET `/Activities/PostseasonResult.aspx?alg=<alg>&class=<c>&year=<YYYY>` (10 GETs covers 5 classes ×
     2 genders for one sport) and compare the embedded `iframe[src]` PDF filename — the `.NET ticks` suffix
     changes whenever MSHSAA republishes, so a filename change is an exact "this artifact is new/replaced" signal.
   - Fetch: GET the `ChampionshipResults/...pdf`, `pdftotext -layout`, then parse the Hy-Tek columns. Idempotent
     and cheap (≤ 20 PDFs per sport-season).
   - State-meet weekend is the only high-frequency window; outside it, polling weekly is sufficient.
   - The per-sport `id=` set (e.g. `2095, 2096, 2100, 2101, 2107, 2113`) can be cached and reused across polls.
4. **Champions history (once per season).** One GET `/Activities/IndividualChampions.aspx?alg=<52|53>` (6.4 MB).
   Re-fetch only after the state meet; diff on `(Year, Class, Student, School, Mark)`.
5. **Never** iterate the `s=` id space, and never re-walk historical years — the `year=` parameter addresses
   history directly and each historical year is immutable once published.

Affected-athlete identification from incremental deltas: a new state PDF yields `(name, school, grade, event)`;
only `grade = 11` rows are Co2027. School names in PDFs are abbreviated, so join through the school directory
(646 HS-level names) rather than exact string equality.

### Access characteristics

**Classification: normal HTML (server-rendered ASP.NET WebForms) + downloadable PDF.** No documented API, no
public JSON, no CSV/XLSX endpoint, no browser application required, not authenticated, not subscription-restricted.

Observed behaviour (single-host, sequential, `curl`, 2026-09-19):

- **User-Agent gate at the edge.** Requests carrying a Chrome-like UA
  (`Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 … Chrome/128.0 Safari/537.36`) were **reset at the TCP
  level** — `curl: (92) HTTP/2 stream 1 reset by server (error 0x8 CANCEL)` then `curl: (56) Recv failure:
  Connection reset by peer` — on `/`, `http://`, apex `mshsaa.org`, and `/Activities/activities.aspx`, across
  HTTP/1.1 and HTTP/2. The **default `curl` UA (`curl/8.9.1`) returns HTTP 200** on every path tried. Sending no
  UA also returned 200. This is a bot-mitigation heuristic keyed on a browser UA that does not match the TLS/HTTP
  fingerprint; it is **not** a CAPTCHA or auth wall. Recommended: identify honestly with a plain client UA. No
  fingerprint spoofing was attempted and none is needed.
- **Rate limiting**: no `429` and **no `Retry-After` header was observed on any response**. No published limit
  found on-site. This report used **≈47 requests in total, sequential with ≥1 s spacing between batched fetches**,
  well under the mission's ~50/host budget, with no throttling.
- **robots.txt**: see Source. `/MySchool/Matchup.aspx*` is disallowed — honour it.
- **Payload sizes** (for budgeting): school listing 721 KB; `/Activities/` index 34 KB; activity home pages 26–31 KB;
  `ActivityEnrollmentBreaks` 74 KB (current) / 111 KB (2025-26); `ClassAndDistrictAssignments` class view 512 KB;
  `IndividualChampions` 6.4 MB; `PostseasonResult` pages 64–74 KB; a single school `MySchool` page 190–217 KB;
  a `Schedule.aspx` page 51–70 KB; the two state PDFs 320–387 KB.
- **Postback fragility**: WebForms postbacks that matter for data (the schedule-download form) failed when
  reconstructed (`__EVENTTARGET` replay returned MSHSAA's "Page Error" page). Stick to GET-addressable pages,
  which cover everything in this report.
- **Assertion**: every URL in the appendix answered HTTP 200 except the one deliberate 404 probe
  (`/Activities/Activities.aspx`) and the failed-UA probes.

### Recommendation

**ATHLETIC.NET-SEED** — with a secondary **VALIDATION** role for state-meet results and **DISCOVERY-ONLY** for
meet calendars.

Rationale and expected marginal coverage:

- MSHSAA is the **authoritative and complete Missouri school universe** (646 high-school-level organizations,
  592 full members, with stable integer school ids, city and county) and the only source that states, per season,
  **which schools sponsor which sport in which gender and class** (T&F 527/526 teams, XC ≈454/426 teams for
  2025-26). That is exactly the seed an Athletic.net team-page resolver needs, and it converts an unbounded
  Athletic.net team search into a bounded, named target list — the single biggest request-avoidance win available
  from this source (order of hundreds to ~650 Athletic.net requests per season [INFERENCE]).
- It supplies a **grade-bearing, independently timed control set**: the state-championship PDFs (Hy-Tek output by
  PrimeTime Timing) carry name, school, `Year` (grade), event, mark, wind, heat, round and place — ~300–350
  Co2027 athletes per sport per season for ~20 requests. Use it to validate whichever state-wide source
  (MileSplit MO, DirectAthletics MO, PrimeTime) supplies the population.
- It **cannot** be the primary athlete population source: no rosters, no regular-season results, no athlete pages,
  no PRs/progression, and no district/sectional results. Only state qualifiers are discoverable, and the
  association's own claim is that its eligibility rosters live inside Athletic.net, not on mshsaa.org.
- It is **not** a coach directory: verified zero coach/AD fields on two school pages and zero AD/coach links on the
  Schools index. Missouri coach/AD contacts must come from MIAAA (`miaaamo.org`), school-district staff directories,
  or individual school athletics sites — but MSHSAA's `MySchool` "School Website" field is the scriptable hop that
  gets a collector to those sites for all 646 schools.
- Two hard hand-off points: **PrimeTime Timing** (`pttiming.com`, linked as "Live Results" from the T&F
  championship page and the named timer on every state PDF) is assignment 22; the **Athletic.net MSHSAA state
  page** (`https://www.athletic.net/track-and-field-outdoor/usa/high-school/missouri/mshsaa`) is where MSHSAA
  delegates official T&F results and is the correct entry point for the Athletic.net side — but it 403s from this
  machine, so its `MeetID`s remain unproven here.

---

### Evidence appendix

All requests issued from this machine with `curl` (default `curl/8.9.1` UA unless noted), sequential, 2026-09-19.
Host: `www.mshsaa.org` (≈47 requests total; no 429, no `Retry-After`).

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.mshsaa.org/` | GET (Chrome UA, HTTP/2) | 000 — `(92) HTTP/2 stream 1 reset by server` | browser-like UA is reset at edge | 2026-09-19 23:11 CDT |
| `https://www.mshsaa.org/` | GET (Chrome UA, HTTP/1.1) | 000 — `(56) Recv failure: Connection reset by peer` | not an HTTP/2-only artifact | 2026-09-19 23:11 CDT |
| `https://mshsaa.org/`, `http://www.mshsaa.org/`, `http://mshsaa.org/` | GET (Chrome UA) | 000 ×3 — connection reset | gate applies to apex and plain HTTP too | 2026-09-19 23:11 CDT |
| `https://www.mshsaa.org/` | GET (no UA) | 200 | default/no UA is served | 2026-09-19 23:11 CDT |
| `https://www.mshsaa.org/` | GET (`curl/8.9.1`) | 200, 76,796 B | site root; nav = Schools / Officials / Sports & Activities / Media / About / Sports Medicine; footer contact `email@mshsaa.org`, (573) 875-4880 | 2026-09-19 23:11 CDT |
| `/Activities/SchoolEnrollments.aspx` | GET | 200, 143,752 B | school name + enrollment by year (2023-24 → 2026-27); **no school ids in this table** | 2026-09-19 23:12 CDT |
| `/Activities/Activities.aspx` | GET | 404 → `error/NotFound.aspx?aspxerrorpath=/Activities/Activities.aspx` | wrong path; find via `/Activities/` index | 2026-09-19 23:12 CDT |
| `/Activities/ActivityEnrollmentBreaks.aspx` | GET | 200, 73,893 B | per-sport class enrollment breaks + team counts; `(n)` documented as "number of teams in each classification"; XC 2026-27 counts; T&F 2026-27 = "TBD" | 2026-09-19 23:12 CDT |
| `/Activities/ActivityEnrollmentBreaks.aspx?year=2025` | GET | 200, 110,530 B | **2025-26 team counts**: XC 112/103+4/88+2/70+4/70+1 and 93/94+2/85+1/68+6/72+5; T&F 157/112+2/92+3/77+4/77+3 and 155/115+2/91+1/77+3/77+5 | 2026-09-19 23:16 CDT |
| `/Schools/` | GET | 200, 57,113 B | links: SchoolListing, SchoolEnrollments, ActivityEnrollmentBreaks, ChampionshipFactor; **0 hits for `Athletic Director`/`Coach`/`Directory`** | 2026-09-19 23:13 CDT |
| `/Activities/` | GET | 200, 34,088 B | sport index; `Content/TrackandField/Home.aspx` (`data-season='3'`), `Content/CrossCountry/Home.aspx` (`data-season='1'`) | 2026-09-19 23:13 CDT |
| `/Schools/SchoolListing.aspx` | GET | 200, 721,371 B | **1,006 orgs**; orgtype legend 1=High Schools(316) / 3=Combined(330) / 2=Junior High(360); member 719 / affiliate 257 / hsa 30; detail links `MySchool/?s=` (719) and `Schools/Navigation.aspx?s=` (287); ids 2…1,913, 47.4 % gaps | 2026-09-19 23:13 CDT |
| `/Content/TrackandField/Home.aspx` | GET | 200, 31,022 B | `data-section 52 = Boys`, `53 = Girls`; exposes `PostseasonResult.aspx?alg=52&year=2026`, `SectionalResults.aspx?alg=52&class=1&sectional=1`, `StateChampionships`, `IndividualChampions`, `ClassAndDistrictAssignments`, and the Athletic.Net icon/PDF | 2026-09-19 23:14 CDT |
| `/Content/CrossCountry/Home.aspx` | GET | 200, 26,133 B | `alg=11` / `alg=12`; `DistrictSite.aspx?alg=11&class=1&district=1&year=2026`; same Athletic.Net PDF icon | 2026-09-19 23:14 CDT |
| `/resources/Activities/CrossCountry/Athletic.net%20helpful%20links.pdf` | GET | 200, 202,900 B, `application/pdf` | MSHSAA↔Athletic.net partner workflow; **"Your eligibility roster from MSHSAA will automatically be uploaded to Athletic.net"**; support@athletic.net | 2026-09-19 23:14 CDT |
| `/Activities/PostseasonResult.aspx?alg=52&year=2026` | GET | 200, 72,021 B | class selector 1–5; year selector 2001-02 → 2026-27; T&F boys current-season view reports "There are currently no results for the selected year" | 2026-09-19 23:14 CDT |
| `/MySchool/?s=3` | GET | 200, 190,530 B | Adrian HS: sports/schedule rows, `Mailto:email@mshsaa.org` only, `School Website` → `https://www.adrian.k12.mo.us/`; **0 hits for Coach/Athletic Director/Principal/Administrator/Phone**; schedule/matchup links with `comp=` ids | 2026-09-19 23:14 CDT |
| `/Activities/PostseasonResult.aspx?alg=52&class=1&year=2025` | GET | 200, 74,089 B → final URL `?alg=52&id=2095` | `year=2025` resolves to group id `2095`; group set `2095,2096,2100,2101,2107,2113`; page is "2026 Class 1 Boys Track and Field Individual Results" | 2026-09-19 23:15 CDT |
| `/Activities/PostseasonResult.aspx?alg=52&id=2095` | GET | 200, 74,054 B | result page; **iframe** `PDFJS/web/viewer.html?file=/resources//ChampionshipResults/Boys-Track-and-Field-2026-Class-1-Boys-Track-and-Field-Individual-Results-639155002143527273.pdf`; "Individual Championship Results" / "Championship Team Scores" tabs; school-search autocomplete embeds the full member school-name list | 2026-09-19 23:15 CDT |
| `/resources/ChampionshipResults/Boys-Track-and-Field-2026-…-639155002143527273.pdf` | GET | 200, 387,275 B, PDF 1.7, 23 pages | Hy-Tek MEET MANAGER by **PrimeTime Timing - Contractor License**; `5/22/2026 to 5/23/2026`, Adkins Stadium – Jefferson City HS; `Name / Year / School / Prelims Wind H#`, `Finals Wind Points`; converted times `10.956 (10.956)`; **Year column = grade** | 2026-09-19 23:15 CDT |
| `/Activities/PostseasonResult.aspx?alg=11&year=2025` | GET | 200, 70,113 B → final URL `?alg=11&id=2030` | XC `year=2025` → `id=2030`; XC group ids `2024,2026,2028,2030,2032` + historical `439…1937`; iframe → `Boys-Cross-Country-2025-Class-1-Boys-Cross-Country-Championship-Results-638981913475921342.pdf` | 2026-09-19 23:15 CDT |
| `/resources/ChampionshipResults/Boys-Cross-Country-2025-…-638981913475921342.pdf` | GET | 200, 320,210 B | `MSHSAA State Championships - 11/7/2025 to 11/8/2025`, Gans Creek XC Course; `Event 2 Boys 5k Run CC Class 1`; 175 rows, grade distribution 9:49 / 10:53 / 11:39 / 12:34 | 2026-09-19 23:15 CDT |
| `/Activities/SectionalResults.aspx?alg=52&class=1&sectional=1` | GET | 200, 60,598 B | **not a results page** — site + manager block; `Site: South Callaway High School`, `Manager: Tony Brandt` | 2026-09-19 23:15 CDT |
| `/Activities/SectionalResults.aspx?alg=52&class=1&district=-1&year=2025` | GET | 200, 63,853 B → `&sectional=1&year=2025` | year/district query params accepted; site info only | 2026-09-19 23:15 CDT |
| `/Activities/SectionalResults.aspx?alg=52&class=1&district=-1&year=2019` | GET | 200, 63,541 B | archived year returns `No Site Information Entered` — **sectional results are never published** | 2026-09-19 23:18 CDT |
| `/Activities/DistrictSite.aspx?alg=11&class=1&district=1&year=2026` | GET | 200, 65,233 B | XC district grid = class 1–5 × district 1–4; host site `Oak Ridge`, date `October 31, 2026`, `Manager: Julie Ward` | 2026-09-19 23:15 CDT |
| `/Activities/ClassAndDistrictAssignments.aspx?alg=52&class=1` | GET | 200, 55,528 B | "Class and District assignments for 2026-2027 have not yet been released" | 2026-09-19 23:15 CDT |
| `/Activities/ClassAndDistrictAssignments.aspx?alg=52&class=1&year=2025` | GET | 200, 511,806 B | **full Class 1 Boys T&F district assignment**: District 1–8, per-district school lists, host site name/address, host manager name + school + phone (`Mike Schmidli / Princeton High School / (660) 748-3211`) | 2026-09-19 23:16 CDT |
| `/Activities/ClassAndDistrictAssignments.aspx?alg=52&class=0&year=2025` | GET | 200, 39,970 B | `class=0` returns only the sport selector — **all-classes-in-one-request is not supported**; 5 requests per sport/gender/year required | 2026-09-19 23:16 CDT |
| `/Activities/IndividualChampions.aspx?alg=52` | GET | 200, 6,478,833 B | **6,163 records, 1926–2026 (101 yrs), 605 schools, 3,834 unique names**, 95 records in recent seasons; columns `Year \| Class \| Student \| School \| wind \| Mark`; relay rows show `Relay <school>`; event tabs `data-event 17…53` | 2026-09-19 23:15 CDT |
| `/Activities/Scoreboard.aspx` | GET | 200, 32,118 B | sport list excludes Cross Country and Track and Field → **no association-wide meet/result scoreboard exists** | 2026-09-19 23:17 CDT |
| `/MySchool/Schedule.aspx?s=3` | GET | 200, 50,976 B | one page lists every sport/level schedule link with literal labels `HS Cross Country - Boys → alg=11`, `HS Cross Country - Girls → alg=12`, `HS Track and Field - Boys → alg=52`, `HS Track and Field - Girls → alg=53`, JH `13/14/54/55`; 30 alg links total | 2026-09-19 23:17 CDT |
| `/MySchool/Schedule.aspx?s=3&alg=11` | GET | 200, 69,876 B | 2026-27 Boys XC schedule: `Class 2 District 2`, 11 dated meet rows, `Schedule Download Options` | 2026-09-19 23:14 CDT |
| `/MySchool/Schedule.aspx?s=3&alg=11&year=2025` | GET | 200, 68,997 B | **`year=` works over GET**; 2025-26 Boys XC = `Class 2 District 4` (class/district is season-scoped) | 2026-09-19 23:17 CDT |
| `/MySchool/Matchup.aspx?s=3&alg=11&comp=2767559` | GET | 200, 32,965 B | `Adrian vs Knob Noster`, Sat 9/12/2026, XC boys, no box score (`TBD`) — **path is robots-disallowed; recorded, do not collect** | 2026-09-19 23:14 CDT |
| `/MySchool/ScheduleDownload.aspx?s=3` | GET | 200, 34,038 B | form with `__VIEWSTATE`; year options 2005-06 → 2026-27; HS/JH; sport radios `19`=T&F, `5`=XC | 2026-09-19 23:14 CDT |
| `/MySchool/ScheduleDownload.aspx?s=3&alg=11` | GET | 200, 35,455 B | form renders only; no file | 2026-09-19 23:14 CDT |
| `/MySchool/ScheduleDownload.aspx?s=3&alg=11` | POST (`__EVENTTARGET=ctl00$contentMain$rblActivity$2`, activity=5, year=2025) | 200 (MSHSAA "Our Apologies / Page Error" body) | reconstructed postback fails → form is not script-friendly | 2026-09-19 23:15 CDT |
| `/Content/TrackandField/InformationCentral.aspx` | GET | 200, 33,314 B | **"Official Results - AthleticNet" → `https://www.athletic.net/track-and-field-outdoor/usa/high-school/missouri/mshsaa`**; **"Live Results - PrimeTime Timing" → `https://www.pttiming.com/`**; 2027 champs May 21-22 (C1-3) / May 28-29 (C4&5), Licklider Track & Field, Jefferson City HS | 2026-09-19 23:14 CDT |
| `/Content/CrossCountry/InformationCentral.aspx` | GET | 200, 37,143 B | 2026 XC champs Nov 6-7, Gans Creek, Columbia; race schedule; Results block links **internally** to `PostseasonResult.aspx?alg=11&class=1..5&year=2025` and `alg=12…` — **no Athletic.net link for XC** | 2026-09-19 23:14 CDT |
| `/MySchool/?s=95` | GET | 200, 217,386 B | Kirkwood HS page confirms the pattern; **0 hits for `Coach` / `Athletic Director`** on a second, large school | 2026-09-19 23:18 CDT |
| `/Search/` | GET | 200, 21,899 B | search UI is JS-driven; no static coach/directory index | 2026-09-19 23:17 CDT |
| `https://www.mshsaa.org/robots.txt` | GET | 200, 670 B | `disallow: /MySchool/Matchup.aspx*`, `/MyAccount/`, `/Ajax/`, `/Data/`, …; `GPTBot`/`Amazonbot`/`meta-externalagent` disallowed; `sitemap: https://www.mshsaa.org/sitemap.xml` | 2026-09-19 23:18 CDT |
| `https://www.mshsaa.org/sitemap.xml` | GET | 200, 44 B, `text/xml` | **empty sitemap** — XML declaration only, 0 `<loc>` entries | 2026-09-19 23:18 CDT |
| `https://www.athletic.net/track-and-field-outdoor/usa/high-school/missouri/mshsaa` | GET | **403** — Cloudflare "Just a moment…", 5,593 B | published by MSHSAA as the official T&F championship result host, but **not retrievable from this machine**; no MeetID extraction possible | 2026-09-19 23:15 CDT |

Cross-references consulted (read-only, not this agent's slices): `HANDOFF.md` in
`/home/lewis/src/ad-law-scrape/athletic-rust-pipeline` (Cloudflare 403 on `GetNavInfo`/`GetRankings`, 5,484 B block
pages; division lists `168416` outdoor / `173005` indoor; season `12026`);
`/home/lewis/Downloads/www.athletic.net.har` and `www.athletic.net2.har` — grepped for `mshsaa` (0 hits in both) and
`missouri` (only a country/state code list in the second HAR) ⇒ **the HAR captures contain no Missouri state-series
evidence**, so the MSHSAA→Athletic.net MeetID mapping is UNVERIFIED.
