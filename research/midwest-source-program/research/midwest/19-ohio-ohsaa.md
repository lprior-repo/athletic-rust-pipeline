# 19. Ohio OHSAA

Status: complete
Observed on: 2026-09-19

Ohio is the only one of the twelve target states where the state association **makes** Athletic.net
rather than merely linking to it. Effective with the 2026 Track & Field and Cross Country seasons the
OHSAA moved its entire tournament entry/results surface onto Athletic.net, pre-created one team page
per member school, and locked high-school team pages to grades 9–12 (exact policy wording in the
Athletic.net leverage section). That makes OHSAA a first-class
**ATHLETIC.NET-SEED** source (school → team, tournament → MeetID) and simultaneously the best
**COACH-DIRECTORY** in the Midwest (names *and* public professional emails via myOHSAA).

---

### Source

| Surface | Provider | URL | Role |
|---|---|---|---|
| OHSAA main site | Ohio High School Athletic Association | `https://www.ohsaa.org/` | Policy, tournament pages, divisional alignments, enrollment |
| Athletic.net policy page | OHSAA | `https://www.ohsaa.org/Sports-Tournaments/Track-Field/Athleticnet-How-To-Usage-Policy` | The OHSAA mandate + link inventory |
| Policy & How-To Guide | OHSAA (Google Doc) | `https://docs.google.com/document/d/1R1BDB3GUFNVgZXmkITGZOiESmvIkKDb0Tbv3axblGdQ/edit` | Full operating rules (public, 17,193 bytes as txt export) |
| Member school directory | OHSAA (myOHSAA, public) | `https://officials.myohsaa.org/Outside/SearchSchool` | 815 schools, AD, head coaches |
| Document CDN | OHSAA (Azure blob) | `https://ohsaaweb.blob.core.windows.net/files/...` | Divisional alignments, results, manuals (stable, unauthenticated) |
| Coaches association | OATCCC | `https://www.oatccc.com/` | Polls, clinics, indoor state meet — **no member directory** |
| Ohio timing industry list | OHSAA (public Google Sheet) | `https://docs.google.com/spreadsheets/d/1Lj20AZpksZ6ohUjUv37L_r8-OwRqoXNeHLYEGMDwa-s/edit` | 64 FAT contractors (business contacts) |

The binding OHSAA tournament regulations (`TFTournamentRegulations.pdf`, `CrossCountryTournamentRegs.pdf`)
refer only to "the approved OHSAA registration system" and never name Athletic.net; the **policy page
and usage guide** carry the explicit Athletic.net mandate. Both were retrieved and searched
(`grep -i "Athletic\.net"` on both PDFs → 0 matches). Do not cite the regulations as the mandate.

---

### Coverage

- **State:** Ohio only (OHSAA jurisdiction). Boys and girls **outdoor track & field** and **cross
  country**. Indoor track is **not** OHSAA-sanctioned (the indoor state meet is run by OATCCC), so
  Ohio indoor coverage is not obtainable here.
- **School levels:** High school (grades 9–12) and, for the state championships only, middle school
  (grades 7–8).
- **Seasons observed:**
  - 2026 outdoor T&F — completed (state meet June 4–7, 2026, Jesse Owens Memorial Stadium); full
    results available. Divisional alignment published as **2025-26** alignments.
  - 2026 XC — in progress at observation; divisional alignment published as **2026-27 & 2027-28**
    (two-year cycle) with 556 boys and 468 girls teams. State-meet page still shows placeholders
    (`Athletic.net Meet Page: TBD`, `Athletic.net Live Results: TBD`) for all four divisions.
  - 2026-27 T&F divisional table on the Divisional-Breakdowns page is **blank** for boys and girls
    T&F (published in HTML for XC, football, volleyball, etc.). T&F alignment for the 2027 spring
    tournament is therefore **not yet published** as of 2026-09-19.
- **How divisions are set (quoted from the Divisional-Breakdowns page):** "OHSAA tournament divisions
  were approved by the OHSAA Board of Directors and announced on April 23, 2026. These tournament
  divisions were determined by **the number of schools that participated in the 2025 tournament** and
  will be used for the 2026 and 2027 tournaments." So division membership is a function of *prior-year
  tournament participation*, not of a fixed enrollment ladder — a school that skips a season can move
  divisions. The page also links a news item for "Full Divisional Assignments"; that news URL was not
  fetched (existence unverified).
- **Historical depth:** T&F divisional PDFs carry 2023 enrollment + 2025-26 and 2024-25 divisions.
  The Central DAB page alone links 36 historical district-result PDFs (2022–2025) plus 9 for 2026.
  Southwest DAB links 4 team-list PDFs for 2026. `/sports/track/history` and `/sports/cc/history`
  exist but were **not fetched** (not needed; existence unverified).
- **Population scale (all parsed by me from the artifacts, not quoted from the site):**
  815 member high schools; 742 competed in the T&F or XC tournament; 721 T&F; 602 XC; 581 both.

---

### Enumeration

#### Schools — 1 request, complete

`GET https://www.ohsaa.org/school-resources/school-enrollment` (HTTP 200, 228,567 bytes) embeds a
single HTML `<table>` with 816 rows (1 header + **815 data rows**) and columns:

```
OhsaaSchoolId | SchoolName | City | AthleticDistrict | Male Enrollment | Male Class | Female Enrollment | Female Class
```

Parsed facts (all 815 rows unique, all IDs numeric, min 100 / max 1000027):

- AthleticDistrict: Northeast 233, Southwest 179, Northwest 162, Central 114, Southeast 73, East 54 = 815.
- Governance class (A/AA/AAA): 267/267/267 male, 14 blank; 268/268/268 female, 11 blank.
- Example row: `972 | MASON | Mason | Southwest | 1310 | AAA | 1262 | AAA`.

This is the authoritative enumerate-everything entry point. **One request yields the entire Ohio
school universe with a stable numeric ID.**

#### Teams — 18 PDF requests, complete

Per-division PDFs linked from `/Sports-Tournaments/Track-Field/2026-Track-Field` (10 files) and
`/Sports-Tournaments/Cross-Country/2026-Cross-Country` (8 files). Each PDF is a table
(`OHSAA School ID | School Name | City | Athletic District | Enrollment | Division | Prior Division`).
Parsed row counts **exactly match the totals OHSAA prints on its own HTML summary tables** where both
exist, which validates the parse:

| Sport / gender | D1 | D2 | D3 | D4 | D5 | Total |
|---|---|---|---|---|---|---|
| Boys XC (2026-27 & 2027-28) | 78 | 160 | 160 | 158 | — | **556** (= site's "Total Schools 556") |
| Girls XC (2026-27 & 2027-28) | 67 | 134 | 133 | 134 | — | **468** (= site's "Total Schools 468") |
| Boys T&F (2025-26) | 82 | 151 | 152 | 149 | 151 | **685** |
| Girls T&F (2025-26) | 78 | 138 | 142 | 144 | 142 | **644** |

Union: 721 distinct school IDs in T&F (all 721 present in the 815-school table), 602 in XC, 742 in
either, 581 in both, 73 member schools in neither.

Verification of the parse (re-run 2026-09-19, `pdftotext -layout` + row regex): each of the 18 PDFs
contains **exactly one** division (no cross-contamination), and boys XC reproduces the site's printed
`II 232-522 → 160`, `III 132-231 → 160`, `IV 131 and less → 158`, `Total Schools 556` exactly, while
girls XC prints `I 516 and more → 66`, `II 236-515 → 135`, `III 137-235 → 133`, `III 136 and less → 134`
(note the duplicate "III" label and the missing "IV" in the source table), `Total Schools 468`. The
girls D2 row differs by one school (site 135 vs PDF 134) — a source-data caveat, not a parse error;
D3 and D4 match exactly and both totals agree at 468.

#### Meets

OHSAA does **not** publish a machine-readable meet index. Postseason meets are enumerated only through:

1. `https://www.athletic.net/team/22671/{track-and-field-outdoor|cross-country}/2026` — OHSAA's own
   Athletic.net "team" calendar, labelled *"Participants and Results"* (T&F) and *"District & Regional
   Athletic.net Calendar, Meet Pages, & Live Results"* (XC). Data requires the Athletic.net app (see
   Access characteristics), so the *list* is Athletic.net-side, but the *entry point URL* is free.
2. Static PDFs: `2026Track&FieldTournamentCalendar.pdf`, `2026StateMeetSchedule.pdf`,
   `2026SitesRepresentationSchedule.pdf` (XC).
3. Regular-season invitationals are **not** enumerated by OHSAA at all.

#### Athletes / Class of 2027 / results

No athlete list, no athlete IDs, no roster export. Athletes appear only as rows inside results PDFs,
each carrying an explicit **year-of-school token**:

```
  1    Anna Wile                12 Hil. Davidso        13.54Q
  1) Noah Dostal 10                  2) Connor McCann 11
```

#### Coach / AD directory — 3 requests per school

`https://officials.myohsaa.org/Outside/Schedule?ohsaaId=<id>` (school info),
`.../AthleticDirector?ohsaaId=<id>`, `.../SportsInformation?ohsaaId=<id>`. Direct ID addressing works
without a search. Search form: `.../SearchSchool?Name=<prefix>&page=<n>` (Name is prefix/word-based;
empty `OhsaaId` alone returns the bare form).

---

### Stable identifiers

| Identifier | Owner | Example | Evidence |
|---|---|---|---|
| **OhsaaSchoolId** | OHSAA | `972` = MASON | Same ID in enrollment table, all 18 divisional PDFs, and myOHSAA URLs; 815 unique |
| **HyTek Code** | OHSAA | `MAS` | `/Outside/Schedule?ohsaaId=972` → "HyTek Code: MAS". Caveat: the code does **not** appear in any checked result artifact — the 2026 state final results PDF, the Central D2A district PDF, the Southwest D2 team list, the enrollment table and all divisional PDFs contain 0 occurrences of `HyTek` and no code column. So the code joins OhsaaSchoolId to *timing-contractor / Hy-Tek meet-manager* data only [INFERENCE], not to published OHSAA output |
| **Athletic.net TeamID** | Athletic.net | `22671` | `athletic.net/team/22671/track-and-field-outdoor/2026` returns HTTP 200 with `<title>Ohio High School Athletic Association - Track and Field Outdoor 2026</title>`. Not in either HAR as an ID — the `22671` strings in both HARs are `"columnNumber": 22671` values inside JS stack traces (verified 2026-09-19), i.e. false positives |
| **Athletic.net MeetID** | Athletic.net | `656920` (2026 HS state T&F), `656686` (2026 7-8 grade state T&F) | URL patterns published on OHSAA state pages |
| **Athletic.net meet sub-id** | Athletic.net | `251905` in `/meet/656686/info/251905` | OHSAA 7-8 grade page |
| **AthleticLive meet id** | AthleticLIVE | `74520` | `live.athletic.net/meets/74520` → 301 → `https://live.seotiming.com/meets/74520` |
| **Athletic.net division/qualifier node IDs** | Athletic.net | `178543`, `178547`, `178980` | OHSAA 7-8 grade page "Auto Updated Top Performers List" links, used as `rankings/{list,qualifying}/<nodeId>/m`. These are **the same node-id space** as `regionDivId`/`divId` in GetNavInfo — evidenced by HAR #1, whose captured page is `TrackAndField/rankings/list/170770/m` while the matching nav response is `"regionDivId":170770` with `tree[0].id = 170770`. So they are resolvable from the division tree, not opaque list IDs |
| **Athletic.net custom list** | Athletic.net | `50249` on team 22671 | `/team/22671/track-and-field-outdoor/custom-list/meets/50249/m?showItems=true` — "Results From OHSAA Qualifying Meets" |
| **Athletic.net Ohio division node** | Athletic.net | `170050` | `GetNavInfo` response in `www.athletic.net.har`, inside a flat state list: `{"id":170050,"name":"Ohio","state":"OH","divType":"State","subDivType":"Divisions","depth":1,"customDivision":null}` (neighbours `170039` North Dakota, `170117` Oklahoma). Note `subDivType:"Divisions"` — Athletic.net's own model of Ohio is division-based, which is why a rankings sweep must be division-parameterized |
| Enrollment cutoffs | OHSAA | XC **boys**: D1 ≥523, D2 232–522, D3 132–231, D4 ≤131. XC **girls**: D1 ≥516, D2 236–515, D3 137–235, D4 ≤136 | Divisional-Breakdowns page (printed tables, verified 2026-09-19) |

**Absent:** athlete IDs, result IDs, event IDs (only Hy-Tek *event numbers* inside PDFs, e.g. `Event 103`,
which are meet-local), season IDs, and any OHSAA-side meet ID.

**Not identifiers:** school name. Names vary between surfaces (`MASON` in the enrollment table,
`Bloom-Carroll High School` in a divisional PDF, abbreviated `Hil. Davidso` in results). The numeric
OhsaaSchoolId is the only safe join; HyTek Code is the bridge to Hy-Tek result text.

---

### Athletic.net leverage

#### OHSAA pages that link to Athletic.net — exhaustive inventory (24 distinct OHSAA pages fetched)

| OHSAA page | Athletic.net links | What they expose |
|---|---|---|
| `/sports/track` | 0 direct (+1 relative link to the OHSAA policy page) | — |
| `/sports/cc` | 0 direct (+1 relative link to the OHSAA policy page) | — |
| `/Sports-Tournaments/Track-Field/Athleticnet-How-To-Usage-Policy` | 6 occurrences / **4 distinct** | `.../track-and-field-outdoor/usa/high-school/ohio`, `.../usa/middle-school/ohio`, 3× `https://www.athletic.net/` (account setup), `live.athletic.net/about` |
| `/Sports-Tournaments/Track-Field/2026-Track-and-Field/2026-Track-Field-State-Coverage` | 3 | `meet/656920/info` (meet page), `live.athletic.net/meets/74520` (live), `meet/656920/entries` (**qualifiers**) |
| `/Sports-Tournaments/Track-Field/2026-Track-Field/ohsaa-jesse-owens-track-and-field-state-championships-qualifiers` | 1 (SafeLinks-wrapped) | `meet/656920/entries`, reached via `nam12.safelinks.protection.outlook.com/?url=…` — a press-release page independently corroborating the qualifiers URL (and leaking an `@ohsaa.org` staff address in the SafeLinks payload) |
| `/sports/track/tournament-info` | 4 occurrences / **1 distinct** | `team/22671/track-and-field-outdoor/2026` — 2 anchor labels ("Participants and Results", "Athletic.net Meet Page") × 2 duplicate markup blocks |
| `/sports/cc/tournament-info` | 1 | `team/22671/cross-country/2026` ("District & Regional Athletic.net Calendar, Meet Pages, & Live Results") |
| `/Sports-Tournaments/Track-Field/2026-Track-Field/2026-7th-8th-Grade-State-Track-Meet` | 6 | `meet/656686/info/251905`, `meet/656686/results`, `rankings/qualifying/178543/m?depth=50`, `rankings/qualifying/178547/m?depth=50`, `rankings/list/178980/m`, `team/22671/.../custom-list/meets/50249/m?showItems=true` |
| **All 6 district T&F pages** (Central, East, Northeast, Northwest, Southeast, Southwest) | **0 each** | — |
| XC state championships, XC preseason invitational, XC-2026 divisional page, Coaches Corner, SW Daily Tournament Results, East D2 site page | **0 each** | — |

**21 href occurrences / 15 distinct destinations** across all 24 OHSAA pages scanned (11 of the 15 are
meet/team/ranking/live result surfaces; the other 4 are on the policy page: `athletic.net/` ×1 distinct,
HS-OH, MS-OH, `live.athletic.net/about`). Linkage is confined to state-level surfaces;
**not one district-level page links to Athletic.net.** Instead the districts link to MileSplit
(Southeast: 5 `oh.milesplit.com/meets/...-ohsaa-...-2025` meet pages; Southwest: `oh.milesplit.com/results`)
and to their own Hy-Tek result PDFs on `ohsaaweb.blob.core.windows.net`.

#### Is school/team mapping deterministic? Yes — on OHSAA's own assertion

Quote from the OHSAA Policy & How To Guide (observed 2026-09-19):

> "All team pages have been created for each member's high school and middle school. New member
> schools will have pages created for them. **No OHSAA member school or coach should set up a new team
> page.** All high schools must use the page created for their high school…"
> "All team names have been set by the OHSAA based on the official school name and official tournament name."

Roster lock, verbatim under "Roster Restrictions":

> "All team pages will be locked so that only 9th, 10th, 11th, & 12th grade athletes may be on a high
> school team page, and only middle school athletes may be on a middle school page. In line with OHSAA
> regulations OHSAA member middle schools may only have 7th and 8th grade athletes on their rosters."

Two consequences the acquisition flow should bank:

1. **A high-school team page cannot contain a non-9–12 athlete**, so any Athletic.net school-team roster is
   grade-safe by construction, and a *grade-11* label seen on such a page is authoritative for eligibility
   screens (still only discovery evidence per the mission's rules).
2. **Athletic.net is the canonical roster even for meets run elsewhere.** The guide requires that when a
   coach registers "on other platforms (MileSplit, **Baumspage**, etc.)… the spelling of all athletes'
   names, grades, & DOBs matches the one in Athletic.net." So Ohio MileSplit/Baumspage meet data is
   roster-synchronised *from* Athletic.net — they corroborate, they do not independently originate identity.

Free-tier cap worth knowing for enumeration cost: "Anyone with an Athletic.net account … will be able to
view the top 25 at any time", and OHSAA "will post a static top 50 list in the last week of the season"
per gender and division. Beyond top 25, listing depth requires a paid upgrade.

Combined with "All OHSAA tournament events will be hosted on Athletic.net" and "Athletic Live is
mandatory for all OHSAA tournament track and cross country contests", the mapping
`OhsaaSchoolId → Athletic.net school team page` is 1:1 and authoritative, and
`OHSAA tournament round → Athletic.net MeetID` is guaranteed to exist.

Only the *resolution* is blocked: `athletic.net/track-and-field-outdoor/usa/high-school/ohio`
(the OHSAA-approved page list) returns HTTP 200 but a 7,284-byte Angular shell with **zero team links
in the HTML**, and the backing API `GET /api/v1/tfRankings/GetNavInfo?...` returns **HTTP 403, 0 bytes**.
TeamID resolution therefore needs the browser session (harness capability) or the name-search route.

**The blocked call is fully specified already — it does not need discovery.** HAR #1 captured both halves
for another state:

- Request URL (verbatim, from HAR #1): `https://www.athletic.net/api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m&recordSetId=0&locationId=0&teamId=0&indoor=false`
- Response shape (verbatim, HAR #1): a flat state list plus `"regionDivId":170770,"tree":[…]`. The tree for
  Wisconsin is **68 nodes over 4 depths**: `depth 0` = state (`170770` Wisconsin, `subDivType:"Divisions"`),
  `depth 1` = Divisions 1/2/3 (`subDivType:"Sectionals"`), `depth 2` = Sectionals (`subDivType:"Schools"`),
  `depth 3` = Regionals (`Regional 1A - Menomonie` … `Regional 8B - Green Bay Preble`). Node ids are
  contiguous within the state.

Ohio's equivalent tree (`regionDivId:170050`) is the missing artifact: it would give `OhsaaSchoolId`-adjacent
Athletic.net division/regional node ids for Divisions I–V plus the district/regional rounds, in **one**
API call per gender/season — matching the node-id form OHSAA already publishes (`rankings/qualifying/178543/m`).
Because that call is 403 to curl, this is a **browser-session capability**, not a plain-HTTP one; budget it
as one app call plus the harvest, not as an unknown. Note the HARs do **not** contain Ohio's tree, and do not
contain `656686`, `656920`, `74520`, `178543`, `178980` or `50249` anywhere.

#### Estimated Athletic.net request avoidance

| Work item | With OHSAA | Athletic.net-only alternative | Avoided |
|---|---|---|---|
| Ohio school universe + classification (815 schools, 5 districts, class + enrollment) | 1 HTML request | 1–N app/API navigation calls, then per-school team pages | **≥815 team-page requests** [INFERENCE, 1 page/school] |
| School → division (T&F 5 div, XC 4 div, per gender) | 18 PDF requests | Division-scoped rankings navigation per event | **2,353 school-gender-division rows** (T&F 685+644, XC 556+468) resolved without Athletic.net |
| Ohio tournament MeetID set (state HS, state MS, district/regional via team 22671) | 4 page requests (URLs already published) | Meet search/navigation | ≥2 MeetIDs + the full district/regional calendar |
| Coach + AD contacts | 3 requests × 742 competing schools = **2,226** | Team-page claiming / "download team contact information" (free team feature, requires an account) | **2,226 contact-page requests**, and no Athletic.net account needed |
| Class-of-2027 grade evidence | 2 PDF requests (state results + a district result) | Profile requests per athlete | Not comparable — OHSAA is *independent* evidence, not cached Athletic.net data |
| Ranking surface | 0 | Repo's own measured cost: 95 queries / division sweep, 4,256 receipts, 142,705 athletes across 4 divisions | Not avoided — OHSAA cannot replace rankings enumeration |

The genuinely large number is the ~2,226 coach/AD request avoidance plus ≥815 team-page resolutions
that become offline lookups. OHSAA does **not** reduce the rankings sweep cost.

---

### Athlete evidence

| Field | Available? | Where / caveat |
|---|---|---|
| Name | Yes | Results PDFs only (no roster export). State PDF uses `First Last`, Central district PDF uses `Last, First` |
| Graduating class / grade | **Yes — explicit year token** | Every result row and relay leg carries `9|10|11|12`. 2026 state results, re-parsed 2026-09-19: **6,667 year tokens** (2,719 individual-result rows + 3,948 relay legs) = 627 grade-9 / 1,269 grade-10 / **2,069 grade-11** / 2,702 grade-12 |
| School | Yes | Result rows (abbreviated) + OhsaaSchoolId join |
| City / state | Yes | School directory (`Mason, OH 45040`) |
| Gender/category | Yes | Event/table context (boys/girls) |
| TF vs XC | Yes | Separate sport pages and PDFs |
| Indoor / outdoor | **No indoor** | OHSAA sanctions outdoor T&F + XC only |
| Performances | Yes | Marks in results PDFs |
| PRs | No | Not published |
| Progression | Partial | Multi-season divisional/result PDFs exist but no per-athlete series |
| Meets | Yes (postseason) | State/district/regional only |
| Athlete profile URL | **No** | OHSAA publishes no athlete pages |

Class-of-2027 yield measurable from a single artifact: the 2026 state final results PDF yields
**561 distinct grade-11 names in individual events** with their school (verified 1:1 — 0 of the 561 names
map to more than one school; 183 appear in more than one row because they contest multiple events/rounds,
max 6 rows for one athlete), rising to **1,067 distinct grade-11 names** when
relay legs are included (that figure is an upper bound: a relay athlete may also have an individual
event, so the true distinct-person count lies between 561 and 1,067). Method: `pdftotext -layout`, then
a place + name + `9|10|11|12` + school + mark row pattern; validated on 26 randomly sampled rows with
zero false positives, and a recall check confirmed the only unmatched year tokens are seconds inside
times (`9:24.49`) and place numbers. No-mark rows (`---` place, `NH`/`FOUL`/`DNF`) are included (83 such
rows). This is the state-meet tip of the pyramid only — district PDFs (Central, 9 files for 2026, plus
36 historical) extend it downward.
No athlete email, phone, or address exists in any of these artifacts.

---

### Recruiting information

The myOHSAA member directory is the strongest coach/AD surface found in this project so far.

| Field | Available | Evidence |
|---|---|---|
| Athletic director | **Yes — name** | `.../AthleticDirector?ohsaaId=972` → "Athletic Director: SCOTT STEMPLE" |
| Public professional email (AD) | **Yes** | same page → `stemples@masonohioschools.com`; also assistant AD and athletic secretaries with emails |
| Head XC coach | **Yes — name + email, boys and girls separately** | `.../SportsInformation?ohsaaId=972` → Cross Country: `Tim Pitcher (Div-I)` → `mailto:pitchert@masonohioschools.com`, `Chip Dobson (Div-I)` → `mailto:dobsonc@masonohioschools.com` |
| Head T&F coach | **Yes — name + email** | same page → Track & Field: `Tony Affatato (Div-I)` → `mailto:tonyaffatato@gmail.com` |
| Assistant coaches | **No** | Only head boys / head girls per sport |
| School athletics website | **Yes** | `Website: http://mhs.masonohioschools.com/` |
| Team website | Partial | Only the school athletics site; no per-team URL |
| Principal (name + email) | Yes, bonus | `brownb@masonohioschools.com` |
| Per-sport tournament division | Yes | Coach cell text carries `(Div-I)`…`(Div-V)`; page is tagged `Sports (2026-27)` |
| **OHSAA Tournament Name** | **Yes — deterministic join key** | School page field `OHSAA Tournament Name: MASON`. This is the exact string OHSAA sets Athletic.net team names from ("All team names have been set by the OHSAA based on the official school name and **official tournament name**"), so it links OhsaaSchoolId → Athletic.net team name without a search |
| Conference, colors, mascot, county, school type | Yes | `Primary Athletic Conference: Greater Miami Conference`, `School Colors: Green and White`, `Boys/Girls Mascot: Comets`, `County: Warren`, `School Type: PUBLIC`, `Phone/Fax` |
| Athletic director phone/fax | Yes (school-level) | `Phone: (513) 398-5025`, `Fax: (513) 398-4183` |

**Measured coverage** (14 schools × 2 sports × 2 genders = 56 coach cells). Selection, verified against
the enrollment table's own ranking: the largest school in five districts (Gahanna Lincoln 1/114 Central,
New Philadelphia 1/54 East, Whitmer 1/162 Northwest, Logan 1/73 Southeast, Mason 1/179 Southwest), the
smallest in two (Bishop Rosecrans 54/54 East, St Joseph Central 73/73 Southeast), five single-sex schools
where one enrollment column is 0 (Columbus School for Girls, St Ignatius, Our Lady of the Elms, Notre
Dame Academy, Mount Notre Dame), and mid-size coed schools (Teays Valley 32/114, Lakewood 28/233). This is
a deliberate stress sample, not a random sample — treat the percentages as directional.

- **33 cells named, and all 33 carry a public email (100% of named cells)** — re-verified 2026-09-19 by
  re-parsing the cached SportsInformation HTML. Per sport: Cross Country **18/28** cells named (10 `N/A`,
  0 `TBA`); Track & Field **15/28** named, 7 `TBA`, 6 `N/A`.
- 7 cells `TBA` (position exists, not yet published — all 7 in Track & Field), 16 cells `N/A` (sport not
  offered). The page states the semantics itself: *"Note: N/A indicates sport not offered. The OHSAA
  tournament division is displayed after each coaches name. Ex: Div-II"*. All-girls schools still carry a
  full girls XC/T&F coach entry (e.g. Columbus School for Girls, Mount Notre Dame, Notre Dame Academy).
- AD page: 5/5 sampled schools had AD name + email (email counts per page 1–5); 4/5 added assistant AD
  or athletic secretary names + emails.
- Coach names present for XC in **11/14** schools; T&F has a named-or-`TBA` entry in **14/14** (T&F cells
  are never wholly empty for the sampled schools — they are named, `TBA`, or `N/A` where the sport is
  genuinely not offered).
- The same 4 coach entries repeat across XC and T&F for small schools (e.g. oid 1076, 1182: one person
  coaches both sports), so a dedup on (name, email) is required before counting distinct coaches.

Privacy: only role-published professional contacts were read. The directory also exposes phone/fax and
principal details; nothing outside school/sport role scope is retained, and no athlete contact data
exists on any OHSAA surface. Note the directory is the district's own self-published data — some coach
emails are personal-provider addresses (e.g. `@gmail.com`), which is a data-quality signal, not a
collection expansion.

---

### Result evidence

| Field | Available? | Detail |
|---|---|---|
| ResultID | **No** | PDF rows have no IDs |
| AthleteID | **No** | Names only; no stable athlete key |
| MeetID | Athletic.net side only | `656920`, `656686` published by OHSAA |
| EventID | **No** | Hy-Tek event numbers only (`Event 103`), meet-local |
| Mark | Yes | `13.54Q`, `8:53.49`, `160` (team scores) |
| Normalized mark inputs | **No** | Rendered strings only; feet/inches and heat labels not structured |
| Timing method | Implied only | Output is Hy-Tek `MEET MANAGER`; OHSAA mandates FAT contractors and mandatory Athletic Live for tournaments |
| Wind | **Yes (state PDF)** | `Heat 1 Preliminaries Wind: -0.3` |
| Implement / hurdle spec | **Yes** | Embedded in event name: `Girls 100 Meter Hurdles 33"`, `Boys 110 Meter Hurdles 39"` |
| Heat / round | **Yes** | `Heat 1 Preliminaries`, `Section 1`, `Finals` |
| Place | Yes | Leading integer; `--` for non-finishers; `Q`/`q` qualification flags |
| Date | Yes | Meet header (`6/4/2026 to 6/7/2026`) and per-page timestamps (`05/26/2026, 03:21 PM`) |
| School represented | Yes | Team column, abbreviated (`Hil. Davidso`, `Wor. Kilbour`) — needs mapping to OhsaaSchoolId |
| Relay membership | **Yes** | `1) Sophia Ryan 12  2) Kaylee May 10  3) Josie Zielinski 9  4) Sydney Lader 11` |
| Records / standards | Yes | `OHSAA Div. 1: 8:53.49 % 6/1/2018 Gahanna Lincoln` |

Artefacts: `OHSAA_State_2026_Final_Results.pdf` (816,147 bytes; 83 pages; 5,638 extracted text lines;
178 unique event-division combinations = 34 events × Divisions I–V plus 8 wheelchair-seated) is Hy-Tek
`MEET MANAGER` output and contains **no Athletic.net URLs or IDs**;
`Athletic.net` appears 0 times in it. The American Athletic.net *result* surface for the same meet is
the URL OHSAA publishes (`meet/656920/...`), whose HTML is a shell (see below).

---

### Incremental use

A weekly Ohio collector needs **no historical re-fetch**:

1. **Meet/results watcher (≈6 requests/week).** The XC state page already contains four literal
   `Athletic.net Meet Page: TBD / Athletic.net Live Results: TBD` placeholders — OHSAA fills these in
   as each division's tournament begins. Poll
   `/Sports-Tournaments/Cross-Country/2026-Cross-Country/2026-Cross-Country-State-Championships`,
   the corresponding T&F state-coverage page, and the four district XC pages. A change in the
   placeholder text is the new-event signal.
2. **Result harvesting by dated filename.** Central DAB result PDFs are named
   `2026-<BTF|GTF|TF>-D<n>[A|B]-Results.pdf`, i.e. the URL **is** the version key — fetch only unseen
   filenames. State results arrive as `OHSAA_State_<year>_Final_Results.pdf`.
3. **Divisional alignments: annual/bi-annual.** XC alignments are two-year (`2026-27 & 2027-28`); T&F
   is per-season. Re-fetch 18 PDFs once per cycle, keyed by the PDF's own title line.
4. **Coach directory: annual.** The SportsInformation page carries a `Sports (2026-27)` year tag —
   re-crawl the 3×N pages once per school year, or spot-refresh schools whose TBA cells were non-null.
5. **Cheap change detection.** Every OHSAA artifact has a `Content-Length`; the enrollment table and
   divisional PDFs are byte-stable between publications in this observation. Compare size/hash before
   re-parsing.
6. **Affected-athlete derivation is offline.** Because results carry name + grade + school, the
   Grade-11 delta for a new result PDF can be computed without any Athletic.net call.

Avoid: polling `athletic.net/team/22671/...` for schedules — the HTML is a shell with no data, so that
would burn browser-session requests for nothing.

---

### Access characteristics

| Host | Class | Observed |
|---|---|---|
| `www.ohsaa.org` | normal HTML (DNN), server-rendered, no auth | 25 requests, all 200; no 429, no `Retry-After` |
| `ohsaaweb.blob.core.windows.net` | static PDF (Azure blob, public, unauthenticated) | 24 requests, all 200; ~0.2 s each; some links carry a SAS `?sv=...&sig=...` query but the same files also resolve **without** it |
| `officials.myohsaa.org` | normal HTML (ASP.NET MVC), public, no auth | 24 requests, all 200 (robots.txt 404 → no published policy) |
| `docs.google.com` | public Google Doc/Sheet, `export?format=txt|csv` works | 2 requests, 200 |
| `www.oatccc.com` | normal HTML | 1 request, 200; no member directory |
| `www.athletic.net` | **browser application** | `/` → 200 (28 KB shell); `/team/22671/...` → 200 (9,134 B shell, title = OHSAA); `/TrackAndField/meet/656920/info` → 200 shell; **`/api/v1/tfRankings/GetNavInfo` → HTTP 403, 0 bytes** |
| `live.athletic.net` | **browser application** | `/meets/74520` → **301** → `live.seotiming.com/meets/74520` → 200 (50,184 B) but title-page shell only; **0** occurrences of "OHSAA"/"Jesse Owens"/JSON in HTML; Angular assets from `livestatic.athletic.net` |
| `oh.milesplit.com` | normal HTML | Only reachable *through* OHSAA's Southeast and Southwest district pages (5 meet links + 1 results index); not crawled here |

Published limits: none. `www.ohsaa.org/robots.txt` (4,173 B) disallows only DNN framework paths
(`/admin/`, `/bin/`, `/Portals/`, `/DesktopModules/`, `/Resources/*`, `/images/`, `/js/`, …) plus
`/Activity-Feed/userId/`; all content paths used here are allowed and **no `Crawl-delay` is set**.
`/sitemap.xml` → 404. Politeness honoured: sequential, ≥1–2 s spacing, ≤26 requests/host.

Reconciliation with the mission brief: the brief records `athletic.net` returning 403 to non-browser
clients "verified 2026-09-19". At 2026-09-19 23:19 CDT I measured a **200** on `/` and on team/meet
HTML shells, but a **403 on the `/api/v1/` path**. So the block is path-dependent (API is protected;
marketing/HTML shells are not), and it is not a blanket site block. Both observations stand; the
error string for the API block is a bare `HTTP 403` with 0-byte body.

---

### Recommendation

**ATHLETIC.NET-SEED** — primary role: seed Ohio Athletic.net team/meet identities and the complete
school universe without paying Athletic.net for discovery. With two secondary roles:

- **COACH-DIRECTORY** (equals the best contact source in this study): 3 requests/school returns AD name
  + public professional email, head boys/girls XC coach names + emails, head T&F coach names + emails,
  school athletics website, and the per-sport tournament division. At 742 competing schools that is
  2,226 requests for the whole state, once per school year.
- **VALIDATION**: Hy-Tek result PDFs carry independent, OHSAA-published **grade** for every athlete —
  the cleanest Class-of-2027 corroboration available in the Midwest (561 distinct grade-11 names in
  individual events, 1,067 including relay legs, from the 2026 state meet alone; expandable via the
  9 current + 36 historical Central district PDFs and the other districts' equivalents).

Expected marginal coverage: 815/815 Ohio member schools identified and classified; 742 with a
deterministic path to an Athletic.net team page; 556 boys + 468 girls XC team-division rows and
685 + 644 T&F rows; the entire Ohio postseason MeetID set published as literal URLs; coach contacts for
33/56 stress-sampled coach cells (59%) with a public email on **every** named cell, plus an AD name +
email for every school sampled. OHSAA contributes **no**
athlete-level enumeration and **no** rankings replacement, so it does not reduce the rankings sweep —
its value is seeding, contacts, and independent grade verification.

Implementation order: (1) enrollment table → `CanonicalSchool` with `OhsaaSchoolId` as the Ohio key;
(2) myOHSAA 3-page fan-out → `CanonicalCoach`; (3) divisional PDFs → division/class attributes;
(4) state + district result PDFs → graded performance rows; (5) publish the 11 Athletic.net URLs as the
MeetID seed set. Do not attempt the OHSAA-approved team-page listing without a browser session; it is
an Angular shell backed by a 403 API.

---

### Evidence appendix

Timestamps are America/Chicago (UTC−05:00) on 2026-09-19; the whole session ran 23:12–23:22 and each
row's time is the scratch-file mtime of that response (`stat -c '%y'`) unless the row is a
non-persisted probe, marked *(probe)*. `curl`/`urllib` UA = Chrome 128 on Linux x86_64, sequential,
2 s spacing via `tools/19-ohsaa-fetch.py`.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.ohsaa.org/` | GET | 200 | Site reachable; nav exposes `/sports/cc`, `/sports/track` | 23:12 |
| `https://www.ohsaa.org/sports/track` | GET | 200 | T&F hub; links Athletic.net policy page | 23:12 |
| `https://www.ohsaa.org/sports/cc` | GET | 200 | XC hub; 44-page news pagination `PgrID/1003` | 23:12 |
| `https://www.ohsaa.org/Sports-Tournaments/Track-Field/Athleticnet-How-To-Usage-Policy` | GET | 200 | **The mandate**: "transition to Athletic.net… effective with the 2026 Track & Field & Cross Country seasons"; links to `/track-and-field-outdoor/usa/high-school/ohio` and `/usa/middle-school/ohio`; AthleticLive + FAT support links | 23:12 |
| `https://www.ohsaa.org/Sports-Tournaments/Track-Field/2026-Track-Field` | GET | 200 | Divisional PDF links D1–D5 × girls/boys; sport administrator "BJ Duckworth" (email Cloudflare-obfuscated) | 23:12 |
| `https://www.ohsaa.org/Sports-Tournaments/Track-Field/2026-Track-and-Field/2026-Track-Field-State-Coverage` | GET | 200 | `meet/656920/info`, `live.athletic.net/meets/74520`, `meet/656920/entries` (Qualifiers List), heat-sheet PDFs, `OHSAA_State_2026_Final_Results.pdf` | 23:13:17 |
| `https://www.ohsaa.org/Sports-Tournaments/Cross-Country/2026-Cross-Country` | GET | 200 | 8 XC divisional PDFs (BXC/GXC D1–D4) | 23:13:20 |
| `https://www.ohsaa.org/sports/track/tournament-info` | GET | 200 | `athletic.net/team/22671/track-and-field-outdoor/2026` ×2 labels; 6 district page links; Google Doc regional info | 23:13:23 |
| `https://www.ohsaa.org/sports/cc/tournament-info` | GET | 200 | `athletic.net/team/22671/cross-country/2026` "District & Regional Athletic.net Calendar, Meet Pages, & Live Results" | 23:13:26 |
| `https://www.ohsaa.org/school-resources/school-enrollment` | GET | 200 (228,567 B) | **815 member schools** with OhsaaSchoolId, city, district, enrollments, A/AA/AAA class | 23:13:29 |
| `https://www.ohsaa.org/School-Resources/Divisional-Breakdowns-2026-27-School-Year` | GET | 200 (127,770 B) | XC 2026-27/2027-28 totals (556 boys / 468 girls, cutoffs); T&F tables blank; "9" = minimum individuals for a T&F team designation | 23:13:31 |
| `https://www.ohsaa.org/Central-Sports-Tournaments/Track-Field` | GET | 200 (117,422 B) | 9× 2026 district result PDFs + 36 historical; "2026 Tournament Roster Entry - Athletic.net" module; coaches seeding login `ohsaaforms.org` | 23:14:26 |
| `https://www.ohsaa.org/East-Sports-Tournaments/Track-Field` | GET | 200 | 0 Athletic.net links; per-division 2026 site subpages | 23:14:29 |
| `https://www.ohsaa.org/Northeast-Sports-Tournaments/Track-Field` | GET | 200 | 0 Athletic.net links; 5 Google Sheets D1–D5 team lists | 23:14:32 |
| `https://www.ohsaa.org/Northwest-Sports-Tournaments/Track-Field` | GET | 200 | 0 Athletic.net links; locations/managers Google Doc | 23:14:35 |
| `https://www.ohsaa.org/Southeast-Sports-Tournaments/Track-Field` | GET | 200 | 0 Athletic.net; **5 `oh.milesplit.com/meets/...-ohsaa-...-2025` links** | 23:14:38 |
| `https://www.ohsaa.org/Southwest-Sports-Tournaments/Track-Field` | GET | 200 | 0 Athletic.net; 4× "2026 D<n> Track Teams.pdf"; `oh.milesplit.com/results` | 23:14:40 |
| `https://www.ohsaa.org/Sports-Tournaments/Cross-Country/2026-Cross-Country/2026-Cross-Country-State-Championships` | GET | 200 | Text placeholders `Athletic.net Meet Page: TBD` / `Athletic.net Live Results: TBD` per division → confirms the fill-in pattern | 23:14:56 |
| `https://www.ohsaa.org/Coaches-Corner` | GET | 200 | No coach directory; links OATCCC and coaches associations | 23:14:59 |
| `https://www.ohsaa.org/East-Sports-Tournaments/Track-Field/Track-Field-D2-2026-Marietta-College` | GET | 200 | District site page: no Athletic.net; gmail mailto for meet manager | 23:15:02 |
| `https://www.ohsaa.org/Southwest-Sports-Tournaments/Daily-Tournament-Results` | GET | 200 | No links/table (empty at observation) | 23:15:05 |
| `https://docs.google.com/document/d/1R1BDB3GUFNVgZXmkITGZOiESmvIkKDb0Tbv3axblGdQ/export?format=txt` | GET | 200 (17,193 B) | **OHSAA policy text** ("Updated 2-1-2026 \| Subject to change"): team pages pre-created; names set by OHSAA; **"Roster Restrictions": "All team pages will be locked so that only 9th, 10th, 11th, & 12th grade athletes may be on a high school team page"**; rosters must match Athletic.net even on "other platforms (MileSplit, Baumspage, etc.)"; "All OHSAA tournament events will be hosted on Athletic.net"; "Athletic Live is mandatory"; "All qualifying meets MUST use Athletic.net… no other platform may be used"; Meet URL + **six-digit Meet ID** required; free tier sees only top 25, OHSAA posts a static top-50 late season; free team feature "download team contact information" | 23:15:16 |
| `https://ohsaaweb.blob.core.windows.net/files/Sports/Track-Field/TFTournamentRegulations.pdf` | GET | 200 (485,759 B) | Entry "via the approved OHSAA registration system"; "Only verified/official performances will be allowed for entry"; **0 mentions of "Athletic.net"** | 23:15:18 |
| `https://ohsaaweb.blob.core.windows.net/files/Sports/Cross%20Country/CrossCountryTournamentRegs.pdf` | GET | 200 (349,927 B) | Same — 0 mentions of "Athletic.net" | 23:15:20 |
| `https://www.ohsaa.org/Sports-Tournaments/Track-Field/2026-Track-Field/2026-7th-8th-Grade-State-Track-Meet` | GET | 200 | 6 Athletic.net links incl. `meet/656686/info/251905`, `rankings/qualifying/178543|178547/m?depth=50`, `rankings/list/178980/m`, custom list `50249`; "Meet URL & Meet ID Numbers are required" | 23:15:50 |
| `https://ohsaaweb.blob.core.windows.net/files/Sports/Track-Field/2026/GirlsTF2026D1.pdf` | GET | 200 (58,539 B) | Title "2025-26 OHSAA Girls Track & Field Divisional Alignments"; columns ID/Name/City/District/2023 Enrollment/2025-26 Div/2024-25 Div | 23:15:52 |
| `https://ohsaaweb.blob.core.windows.net/files/Sports/Track-Field/2026/OHSAA_State_2026_Final_Results.pdf` | GET | 200 (816,147 B) | Hy-Tek MEET MANAGER output; 82 pages; name + year token + school + mark + wind + heat; relay legs with grades; **0 Athletic.net references** | 23:15:54 |
| `https://docs.google.com/spreadsheets/d/1Lj20AZpksZ6ohUjUv37L_r8-OwRqoXNeHLYEGMDwa-s/export?format=csv&gid=2090882136` | GET | 200 (14,961 B) | 69 rows / **64 distinct Ohio FAT contractors**, districts served, scoring programs (Hytek 58, MeetPro 18, RunMeet 1) | 23:15:57 |
| `.../2026/GirlsTF2026D2.pdf` … `D5.pdf` | GET ×4 | 200 | Girls T&F D2 138 / D3 142 / D4 144 / D5 142 rows | 23:16:11–23:16:18 |
| `.../2026/BoysTF2026D1.pdf` … `D5.pdf` | GET ×5 | 200 | Boys T&F D1 82 / D2 151 / D3 152 / D4 149 / D5 151 rows | 23:16:20–23:16:29 |
| `https://www.ohsaa.org/Sports-Tournaments/Cross-Country/2026-Cross-Country/2026-Preseason-Invitational` | GET | 200 | No Athletic.net links (PDF-only page) | 23:17:11 |
| `https://officials.myohsaa.org/Outside/SearchSchool` | GET | 200 (8,129 B) | Public directory form: GET params `Name`, `OhsaaId` | 23:17:13 |
| `http://www.oatccc.com/` | GET | 200 (38,375 B) | Coaches association; no member/coach directory; links MileSplit OH, FinishTiming | 23:17:16 |
| `https://officials.myohsaa.org/Outside/SearchSchool?OhsaaId=972` | GET | 200 | `OhsaaId` alone does **not** search (bare form returned) | 23:17:23 |
| `https://officials.myohsaa.org/Outside/SearchSchool?Name=Mason` | GET | 200 | Name search works → `MASON` = ohsaaId **972**, `MASON MIDDLE SCHOOL` = 6702; `&page=2` pagination | 23:17:25 |
| `https://officials.myohsaa.org/Outside/Schedule?ohsaaId=972` | GET | 200 | School page: OHSAA ID 972, county, type PUBLIC, district, **HyTek Code MAS**, conference, enrollment + class, athletics website, principal name + email | 23:17:31 |
| `.../Outside/Schedule/AthleticDirector?ohsaaId=972` | GET | 200 | AD name + professional email; assistant AD + 3 secretaries with emails | 23:17:36 |
| `.../Outside/Schedule/SportsInformation?ohsaaId=972` | GET | 200 | "Sports (2026-27)" table: head boys/girls coach per sport with `mailto:` emails and `(Div-n)`; Cross Country = Tim Pitcher / Chip Dobson | 23:17:40 |
| `.../SportsInformation?ohsaaId=<876,414,1106,222,1354,1222,1704,1182,888,1368,1526,832,1076>` | GET ×13 | 200 | Stress sample re-parsed 2026-09-19 via `<tr>`/`<td>` table extraction: 56 cells → **33 named, all 33 with `mailto:`** (XC 18/28, T&F 15/28), 7 `TBA` (all T&F), 16 `N/A`; page note confirms "N/A indicates sport not offered" and that `(Div-n)` is the tournament division | 23:18:00–23:18:29 |
| `.../AthleticDirector?ohsaaId=<1106,1368,1704,414>` | GET ×4 | 200 | AD name + email on 4/4; the full 5-page set (adding 972) shows 1–5 emails per page, with assistant AD / athletic-secretary rows on 4 of 5; note `@mac.com` and `@gmail.com` personal-provider addresses on some coach/AD rows | 23:18:5x |
| `https://officials.myohsaa.org/Outside/SearchSchool?Name=a` | GET | 200 | Prefix search; ~3 schools/page → full crawl impractical; use the enrollment table for the ID list | 23:19 |
| `https://ohsaaweb.blob.core.windows.net/files/Sports/Cross%20Country/2026/2026BXC-D1.pdf` | GET | 200 (54,285 B) | Title "2026-27 & 2027-28 OHSAA Boys Cross Country Divisional Alignments"; OHSAA School ID column present | 23:19 |
| `.../2026/2026BXC-D{2,3,4}.pdf`, `.../2026GXC-D{1,2,3,4}.pdf` | GET ×7 | 200 | Boys 160/160/158; Girls 67/134/133/134 rows; totals match OHSAA's printed 556 / 468 exactly | 23:19–23:20 |
| `https://www.ohsaa.org/Sports/News/ohsaa-jesse-owens-track-and-field-state-championships-qualifiers` | GET | 200 (75,044 B) | Qualifiers news page — **does** carry 1 Athletic.net link, wrapped in an Outlook SafeLinks redirect (`nam12.safelinks.protection.outlook.com/?url=…meet%2F656920%2Fentries…`) that exposes an `@ohsaa.org` staff address in the payload; independently corroborates the qualifiers URL | 23:19 |
| `https://ohsaaweb.blob.core.windows.net/files/Sports/Cross%20Country/2026/2026SitesRepresentationSchedule.pdf` | GET | 200 (296,075 B) | 4 regional sites (Northeast/Boardman, Northwest/Tiffin, Central/Pickerington, Southwest/Troy) covering regions 1–15; regional schedule; district→regional representation | 23:20 |
| `https://ohsaaweb.blob.core.windows.net/files/DAB/Central/Track_Field/2026/2026-BTF-D2A-Results.pdf` | GET | 200 (275,125 B) | District results format `Last, First NN SCHOOL MARK`: re-parsed 2026-09-19 → **428** graded result rows (56 grade-9 / 120 grade-10 / **131 grade-11** / 121 grade-12), **49 unique grade-11 names** in this single file | 23:20 |
| `https://ohsaaweb.blob.core.windows.net/files/DAB/SouthWest/Track%20and%20Field/2026%20D2%20Track%20Teams.pdf` | GET | 200 (50,482 B) | Southwest D2 site assignment: 38 girls / 36 boys schools across ROSS/BELLBROOK/TROTWOOD MADISON sites with dates | 23:20 |
| `https://www.oatccc.com/Coaches/Membership/` | GET | 200 (32,148 B) | No member table/directory (join form only) | 23:21 |
| `https://www.ohsaa.org/robots.txt` | GET | 200 (4,173 B) | DNN defaults only; content paths allowed; no `Crawl-delay`; no sitemap directive active | 23:21 |
| `https://officials.myohsaa.org/robots.txt` | GET | **404** | No published crawl policy on the directory host | 23:21 |
| `https://www.ohsaa.org/sitemap.xml` | GET | **404** | No sitemap | 23:21 |
| `https://www.athletic.net/team/22671/track-and-field-outdoor/2026` | GET | **200** (9,134 B) | `<title>Ohio High School Athletic Association - Track and Field Outdoor 2026</title>` ⇒ **TeamID 22671 = OHSAA**; body is an Angular shell (`anetSiteAppParams.teamHeader=22671`), 0 meet mentions | 23:19 |
| `https://www.athletic.net/TrackAndField/meet/656920/info` | GET | **200** (9,241 B) | Meet page shell; generic title; data not in HTML | 23:19 |
| `https://www.athletic.net/track-and-field-outdoor/usa/high-school/ohio` | GET | **200** (7,284 B) | OHSAA-approved team-page listing is an Angular shell: **0 `/team/<id>` links**, 0 "OHSAA" occurrences | 23:19 |
| `https://www.athletic.net/` | GET | **200** (28,026 B) | Root not blocked at observation time (reconciles the brief's earlier 403) | 23:19 |
| `https://www.athletic.net/api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m&recordSetId=0&locationId=0&teamId=0&indoor=false` | GET | **403** (0 B) | The Athletic.net **API is Cloudflare-blocked** to this client; HTML shells are not | 23:19 |
| `https://live.athletic.net/meets/74520` | GET | **301** → `https://live.seotiming.com/meets/74520` | OHSAA state-meet live-results link resolves to a white-labelled AthleticLIVE host | 23:19 |
| `https://live.seotiming.com/meets/74520` (via -L) | GET | 200 (50,184 B) | AthleticLIVE shell (title "AthleticLIVE"); 0 "OHSAA"/"Jesse Owens" occurrences; Angular assets at `livestatic.athletic.net` | 23:19 |
| Local: `www.athletic.net.har` (+`www.athletic.net2.har`) — `GetNavInfo` responses, ID sweeps | parse | n/a | HAR #1 holds a full **68-node Wisconsin nav tree** (`"regionDivId":170770,"tree":[…]`, depths 0–3: state → Division 1/2/3 → Sectionals → Regionals) plus a flat state list containing `170050 Ohio / subDivType "Divisions"` (neighbours `170039` ND, `170117` OK); the same HAR's captured page is `TrackAndField/rankings/list/170770/m`, proving `rankings/list/<nodeId>` consumes division-node ids. The nav request URL is captured verbatim. **ID absence check:** `656686`, `656920`, `74520`, `178543`, `178980`, `50249` → 0 occurrences in *either* HAR; `22671` → the 61/31 hits are `"columnNumber": 22671` JS stack-trace values, not the team id (verified 2026-09-19) | 23:13, re-verified 23:29 |
| Local: `www.athletic.net2.har` | parse | n/a | 0 `GetNavInfo`, 3 `GetRankings`, 0 `Sectionals` — the second capture is rankings-heavy and contains no nav tree; useful only for cross-checking | 23:13 |
| Local: 18 divisional PDFs, re-parsed with `pdftotext -layout` + a row regex (2023 enrollment / 2025-26 & 2024-25 division columns) | parse | n/a | **All 18 files contain exactly one division** and reproduce the printed totals exactly: Boys T&F 82+151+152+149+151=**685**; Girls T&F 78+138+142+144+142=**644**; Boys XC 78+160+160+158=**556**; Girls XC 67+134+133+134=**468**; 1,024 XC rows. Unique OhsaaSchoolIds: T&F **721**, XC **602**, union **742**, both **581** (TF-only 140, XC-only 21) | 23:24 |
| Local: enrollment table HTML re-parsed | parse | n/a | 816 `<tr>` = 1 header + **815 data rows**, 815 unique numeric IDs (min 100, max 1000027); districts NE 233 / SW 179 / NW 162 / C 114 / SE 73 / E 54; class A/AA/AAA = 267/267/267 male (14 blank) and 268/268/268 female (11 blank); **all 721 T&F and 602 XC IDs are present in the 815**, and 815−742=**73** member schools competed in neither | 23:24 |
| Local: `OHSAA_State_2026_Final_Results.pdf` re-parsed | parse | n/a | 2,719 individual rows + 3,948 relay legs = **6,667 year tokens** (627/1,269/**2,069**/2,702 for grades 9/10/11/12); **561** distinct grade-11 names (0 map to >1 school; 183 appear in >1 row, max 6); 1,067 distinct grade-11 names including relay legs; **83 pages**, 5,638 text lines, **178** event-division combos (34×5 + 8 wheelchair); precision validated on 26 random rows with 0 false positives | 23:25 |
| Local: `2026-BTF-D2A-Results.pdf` (Central D2 district) re-parsed | parse | n/a | 428 graded rows; 131 grade-11 tokens; 49 unique grade-11 names from one district file | 23:25 |
| Local: all 24 OHSAA pages re-scanned for `athletic.net`/`live.athletic.net` | parse | n/a | **21 href occurrences / 15 distinct destinations**; 5 pages carry links (State Coverage 3, 7&8 Grade 6, T&F Tournament Info 4/1 distinct, XC Tournament Info 1, Qualifiers news 1 SafeLinks) | 23:25 |
| Local: Central DAB page re-scanned | parse | n/a | 72 `.pdf` links; **45 distinct result PDFs** = 9 for 2026 + **36 historical** (2022:10, 2023:13, 2024:6, 2025:7) | 23:25 |
