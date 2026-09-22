# 15. Michigan MHSAA

Status: complete
Observed on: 2026-09-19

### Source

Michigan High School Athletic Association (MHSAA), `https://www.mhsaa.com` (site is Drupal; member portal `https://my.mhsaa.com` is DNN).

Surfaces that matter for acquisition:

| Surface | URL pattern | Content |
|---|---|---|
| Sport hub | `/sports/{boys\|girls}-{track-field\|cross-country}` | "Tracking the Tournament" (finals AN links), "The Finals" block, links to per-year regional page |
| Per-year TF regional page | `/sports/boys-track-field/{YYYY}-mhsaa-track-field-regional-results` (2026, 2024) · `/sports/girls-track-field/2025-mhsaa-track-field-regional-results` · `/sports/boys-track-field/2023-mhsaa-track-field-regional-info-and-results` | one line per regional: host + "Meet Info" PDF + Athletic.net Results link |
| Per-year XC regional page | `/sports/boys-cross-country/{YYYY}-mhsaa-cross-country-regional-info-and-results` (2023, 2024, 2025) · `.../2026-mhsaa-lp-cross-country-regional-info-and-results` | one line per region: host + Meet Info PDF + AN RESULTS link (once contested) |
| Results archive | `/sports/{boys\|girls}-{track-field\|cross-country}/results-archive` | every season's finals block (AN links for recent years, PDFs for UP + older years) |
| Meet-Info / result PDFs | `/sites/default/files/{Track\|Cross}%20{Field-Boys\|Country-Boys}/{year}/...` | participating schools, host AD/coach contacts, entry rules; UP finals + XC finals full results with grade |
| School universe | `/i-am/administrators/enrollment-classification-co-ops` -> PDFs under `/sites/default/files/Enrollment%20and%20Classification/` | 755 member schools with MHSAA ID, enrollment, class; per-sport division sizes; co-op programs |
| Division lists (per school/sport) | `https://my.mhsaa.com/Sports/{Sport}/School-Division-List-{year}-{LP\|UP}` | Knockout SPA - school -> division; **no server-rendered data** |
| Sitemap | `/sitemap.xml?page=1..7` | 12,046 URLs (mostly stories; no school pages, no result data) |

### Coverage

* State: Michigan only (LP + UP). Member HS: **755** (2026-27 enrollment list, unique MHSAA IDs).
* Sports: boys/girls outdoor track & field, boys/girls cross country. Indoor TF is not an MHSAA-run tournament series (MHSAA publishes only news coverage of indoor meets - e.g. `/sports/girls-track-field/stories/keweenaw-classic-provides-indoor-competition-spring-begins`).
* Levels: varsity HS tournaments (regionals + finals) and a separate JH/MS regional series (zones, no AN meet links except one 2026 TF zone and the XC MS directory).
* Seasons with Athletic.net meet links observed on MHSAA pages:
  * **TF regionals**: 2026, 2025, 2024 (48 regionals each, 100% linked).
  * **TF finals**: 2026, 2025, 2024, 2023 (+ legacy `Print/EntryMeet.aspx?Meet=` qualifier links 2016-2019).
  * **XC regionals**: 2025, 2024, 2023 (36 LP regions each, 100% linked); 2026 page is live but not yet populated with results.
  * **XC finals**: PDF only (no AN links in any year observed).
* **XC finals artifact inventory (verified on the results-archive page, 2026-09-19):** every season since ~2022 publishes **14 finals PDFs = 8 LP (boys D1-4 + girls D1-4) + 6 UP (boys D1-3 + girls D1-3)**; 2025 = `.../Cross Country-Boys/2025/Finals/2025lpbd{1..4}final.pdf`, `2025lpgd{1..4}final.pdf`, `2025upbd{1..3}final.pdf`, `2025upgd{1..3}final.pdf` (all sexes in the **Boys** directory, `?time=1764337348786` = 2025-11-28 upload). 2023/2022 instead split girls PDFs into a real `Cross Country-Girls/...` directory (`LP%20Girls%20D1.pdf`, `UP%20Girls%20D1.pdf`). Older seasons degrade: 2012-2013 = `my.mhsaa.com/Portals/0/documents/BXC/...txt`, 2016/2018 = legacy AN links, 2021 = `athletic.net/events/usa/michigan/2021-10-29`, 2022 JH/MS = 8 AN zone meet ids (207985-207992, 208580). Archive depth reaches at least 2008.
* Historical depth: archive pages carry finals back to the 1920s for narratives/past champions, but machine-readable links/PDFs start ~2016 (legacy AN ids) and are complete 2023-2026.
* Regular season, invitationals and conference meets are **not** covered by MHSAA - postseason only.

### Enumeration

* **Schools** - yes, cheapest possible: one PDF per year.
  `https://www.mhsaa.com/sites/default/files/Enrollment%20and%20Classification/26-27-Enrollment-List.pdf`
  = 755 rows `(MHSAA ID, school name, enrollment total, classification enrollment)`, e.g. `5792 East Kentwood 3042 3042`,
  `4183 Rockford 2319`, `1833 Novi 2112`, `4638 Detroit Catholic Central 1046 2092` (single-gender schools carry a doubled classification enrollment).
  Classes: A >=783 (188 schools), B 358-782 (189), C 167-357 (189), D <=166 (189) - totals printed in
  `classification-by-sport-2026-27.pdf`.
* **Teams** - not enumerated as teams. Two partial substitutes:
  1. per-meet **SCHOOLS ASSIGNED** lists inside Meet-Info PDFs (Region 24 = 16 schools; Region 2 = 14 schools);
  2. per-school division lists at `https://my.mhsaa.com/Sports/{Sport}/School-Division-List-{year}-{LP|UP}` (JS app, see Access).
* **Meets** - yes, by walking three pages per sport per season:
  `hub -> per-year regional page -> results-archive`. 2026 TF: 48 regionals + 5 finals meet IDs; 2025 TF: 48 + 5; 2026 XC: 36 regionals (links pending) + LP/UP finals (PDF).
* **Athletes** - **not enumerable from MHSAA** except inside finals PDFs (name + grade + school + mark).
* **Class of 2027** - only from finals PDFs: XC finals have a `Year` column, UP TF finals have a `Yr` column, and 2023/2024 LP TF finals PDFs also have `Yr` (+ `Wind`).
* **Results** - regionals: MHSAA points to Athletic.net (no results of its own). Finals: LP = AN links (2026), PDFs (<=2024); UP = PDFs every year; XC = PDFs every year.
* Sitemap `https://www.mhsaa.com/sitemap.xml?page=1..7` = 12,046 URLs; contains no school pages and no result data - not an enumeration path.

### Stable identifiers

| Identifier | Available? | Where / example |
|---|---|---|
| MHSAA school ID | **yes** | enrollment-list PDF column `ID`; 755 unique (e.g. `5792` East Kentwood, `4183` Rockford). Numeric, stable across years (same number appears in 2024-25 / 2025-26 lists). |
| Co-op program ID | **yes** | `hscoop.pdf`, leading `(NNNN)` per program, e.g. `(1804) Ada Forest Hills Eastern(735)`; distinct ID space from school ID (that school's MHSAA ID is `4506`). |
| Athletic.net MeetID | **yes, directly** | every Results link, e.g. `https://www.athletic.net/TrackAndField/meet/622922/results` -> MeetID `622922`. |
| Athletic.net team ID / athlete ID / result ID | **no** | zero AN team/athlete/result links on any MHSAA page sampled (see Athletic.net leverage). |
| MHSAA meet ID | **no** | MHSAA has no meet identifier of its own; meets are identified by season + sport + division + region number + host. |
| Event ID | **no** | event names only (`Boys' High Jump D1`); ranking-order numbering (1)-(5) is printed next to event names in the finals PDFs but is not a global id. |
| Season ID | **no** | season expressed as school year (`2026-27`) and calendar year page slug (`2026-...`). |
| Region / division numbers | MHSAA-local, not stable | region membership and hosts change yearly (`Regional 24` = Bangor HS in 2024/2025/2026 but a different set of schools is not guaranteed). |
| Bib number | meet-scoped only | finals PDFs print `Bib`, e.g. `2025 LP XC Girls D1: 12 Sienna KLEMMER`; not an athlete id. |
| Legacy AN ids | historical | `TrackAndField/Print/EntryMeet.aspx?Meet=319819&t=ia74b` (2016-2019 qualifier lists), `CrossCountry/State/Archive.aspx?State=46698` (legacy state id, 2018). |

### Athletic.net leverage

**Exact link patterns observed** (only meet links; never team/athlete links):

```
https://www.athletic.net/TrackAndField/meet/{MeetID}/results
https://www.athletic.net/TrackAndField/meet/{MeetID}/results/all
https://www.athletic.net/TrackAndField/meet/{MeetID}/teamscores
https://www.athletic.net/TrackAndField/meet/{MeetID}/info
https://www.athletic.net/CrossCountry/meet/{MeetID}/results
https://www.athletic.net/CrossCountry/meet/{MeetID}/results/all
https://www.athletic.net/CrossCountry/meet/{MeetID}/info
legacy: https://www.athletic.net/TrackAndField/Print/EntryMeet.aspx?Meet={MeetID}&t={token}
non-meet: https://www.athletic.net/TrackAndField/Michigan/ , https://www.athletic.net/CrossCountry/Michigan/ ,
          https://www.athletic.net/cross-country/usa/middle-school/michigan
```

**Coverage counts of MHSAA -> AN MeetID mappings** (all extracted from server-rendered hrefs, not inferred):

| Season | TF regionals | TF finals | XC regionals | XC finals | Total AN meet IDs |
|---|---|---|---|---|---|
| 2026 | 48 | 5 | 0 (page live, not yet contested) | 0 (PDF) | 53 so far |
| 2025 | 48 | 5 | 36 | 0 (PDF) | 89 |
| 2024 | 48 | 4 | 36 | 0 (PDF) | 88 |
| 2023 | 48 | 4 | 36 | 0 (PDF) | 88 |

The 48 TF regionals = **40 LP** (D1-D4 x 10 regions, R1-R40) + **8 UP** (D1: R41-R42, D2: R43-R44, D3: R45-R48). MHSAA prints one Results link per regional, and the girls hub points at the same regional page/ids as the boys hub; that each AN regional meet page therefore spans both genders [INFERENCE from the single link + combined-site format - not opened on Athletic.net, which is 403 to this machine].

**2026 MHSAA TF regional -> Athletic.net MeetID (full mapping, 48 regionals + 1 JH/MS zone)**

| Level | Division | Region | Site | Athletic.net MeetID |
|---|---|---|---|---|
| LP | Division 1 | R1 | Mt Pleasant HS | `622922` |
| LP | Division 1 | R2 | Zeeland East HS | `630085` |
| LP | Division 1 | R3 | Portage Central HS | `630086` |
| LP | Division 1 | R4 | Grand Ledge HS | `630130` |
| LP | Division 1 | R5 | Saline HS | `630132` |
| LP | Division 1 | R6 | Novi HS | `630138` |
| LP | Division 1 | R7 | Grosse Pointe South | `630281` |
| LP | Division 1 | R8 | Rochester Adams | `630282` |
| LP | Division 1 | R9 | Milford HS | `630284` |
| LP | Division 1 | R10 | Fraser HS | `630285` |
| LP | Division 2 | R11 | Cadillac HS | `630287` |
| LP | Division 2 | R12 | Allendale HS | `630668` |
| LP | Division 2 | R13 | Sparta HS | `630673` |
| LP | Division 2 | R14 | Sturgis HS | `630674` |
| LP | Division 2 | R15 | Williamston HS | `630676` |
| LP | Division 2 | R16 | Chelsea HS | `630677` |
| LP | Division 2 | R17 | Divine Child HS | `630679` |
| LP | Division 2 | R18 | Marian HS | `630691` |
| LP | Division 2 | R19 | North Branch HS | `630698` |
| LP | Division 2 | R20 | Frankenmuth HS | `630702` |
| LP | Division 3 | R21 | Charlevoix HS | `630705` |
| LP | Division 3 | R22 | Manton HS | `630752` |
| LP | Division 3 | R23 | Montague HS | `630755` |
| LP | Division 3 | R24 | Bangor HS | `630757` |
| LP | Division 3 | R25 | Hillsdale HS | `630758` |
| LP | Division 3 | R26 | Madison HS | `630763` |
| LP | Division 3 | R27 | Mt Clemens HS | `630765` |
| LP | Division 3 | R28 | Stockbridge HS | `630767` |
| LP | Division 3 | R29 | Bad Axe HS | `630768` |
| LP | Division 3 | R30 | Chesaning HS | `630769` |
| LP | Division 4 | R31 | Inland Lakes HS | `630770` |
| LP | Division 4 | R32 | Frankfort HS | `630913` |
| LP | Division 4 | R33 | Brethren HS | `630914` |
| LP | Division 4 | R34 | Gobles HS | `630915` |
| LP | Division 4 | R35 | Hillsdale Academy | `630916` |
| LP | Division 4 | R36 | Concord HS | `630917` |
| LP | Division 4 | R37 | Fowler HS | `630918` |
| LP | Division 4 | R38 | Vassar HS | `630919` |
| LP | Division 4 | R39 | Dryden HS | `630920` |
| LP | Division 4 | R40 | Whitmore Lake HS | `630921` |
| UP | Division 1 | R41 | Kingsford HS | `630923` |
| UP | Division 1 | R42 | Manistique HS | `630922` |
| UP | Division 2 | R43 | Gwinn HS | `630924` |
| UP | Division 2 | R44 | Bark River | `630925` |
| UP | Division 3 | R45 | St Ignace HS | `630926` |
| UP | Division 3 | R46 | Rapid River HS | `630927` |
| UP | Division 3 | R47 | Ishpeming HS | `630929` |
| UP | Division 3 | R48 | Lake Linden | `630928` |
| JH/MS | Zone 1 | JH-1 | Paw Paw HS | `648458` |

**2025 MHSAA XC regional -> Athletic.net MeetID (full mapping, 36 LP regions)**

| Division | Region | Site | Athletic.net MeetID |
|---|---|---|---|
| LP D 1 | 1 | Allendale | `256167` |
| LP D 1 | 2 | Mount Pleasant | `256169` |
| LP D 1 | 3 | Portage Central | `256171` |
| LP D 1 | 4 | DeWitt | `256172` |
| LP D 1 | 5 | Wyandotte Roosevelt | `256173` |
| LP D 1 | 6 | Ann Arbor Huron | `256174` |
| LP D 1 | 7 | Royal Oak | `256175` |
| LP D 1 | 8 | Clarkston | `256176` |
| LP D 1 | 9 | Romeo | `256177` |
| LP D 2 | 10 | Benzie Central | `256178` |
| LP D 2 | 11 | Allendale | `256179` |
| LP D 2 | 12 | Grand Rapids South Christian | `256180` |
| LP D 2 | 13 | Portage Central | `256181` |
| LP D 2 | 14 | DeWitt | `256182` |
| LP D 2 | 15 | Shepherd | `256184` |
| LP D 2 | 16 | Linden | `256186` |
| LP D 2 | 17 | Algonac | `256187` |
| LP D 2 | 18 | Wyandotte Roosevelt | `256199` |
| LP D 3 | 19 | Mancelona | `256188` |
| LP D 3 | 20 | Benzie Central | `256203` |
| LP D 3 | 21 | Allendale | `256194` |
| LP D 3 | 22 | Portage Central | `256195` |
| LP D 3 | 23 | Napoleon | `256196` |
| LP D 3 | 24 | Bath | `256198` |
| LP D 3 | 25 | Deckerville | `256204` |
| LP D 3 | 26 | Algonac | `256205` |
| LP D 3 | 27 | Ann Arbor Huron | `256206` |
| LP D 4 | 28 | Mancelona | `256207` |
| LP D 4 | 29 | Benzie Central | `256208` |
| LP D 4 | 30 | Shepherd | `256209` |
| LP D 4 | 31 | Allendale | `256216` |
| LP D 4 | 32 | Three Oaks River Valley | `256210` |
| LP D 4 | 33 | Webberville | `256211` |
| LP D 4 | 34 | Napoleon | `256212` |
| LP D 4 | 35 | Deckerville | `256213` |
| LP D 4 | 36 | Royal Oak | `256215` |

2024 XC: 36 regionals, Region 1 Allendale `238239` ... Region 36 Royal Oak `238292`, plus a JH/MS "Zone 6 - Holland Christian" row `238694` (37 AN ids on the page). 2023 XC: 36 regionals, Region 1 Allendale `223261` ... Region 36 Royal Oak Shrine Catholic `223335`.
Per-page id inventories measured from the hrefs: 2026 TF = 49 (48 regionals + JH/MS Zone 1 `648458`), 2025 TF = 49 (48 + 1 JH/MS), 2024 TF = 48, 2025 XC = 36, 2024 XC = 37 (36 + 1 zone), 2023 XC = 36, 2026 XC = 0 so far (only the middle-school directory link).

**2026 TF/LP finals MeetIDs** (from the hubs' "Tracking the Tournament"/"The Finals" blocks; D1-D4 + UP):

| Finals | 2026 | 2025 | 2024 | 2023 |
|---|---|---|---|---|
| LP D1 | `622966` | `571113` | `529571` | `481008` |
| LP D2 | `622969` | `571114` | `529788` | `481009` |
| LP D3 | `622972` | `571115` | `529793` | `481010` |
| LP D4 | `622973` | `571116` | `529795` | `481011` |
| UP | `622974` | `571117` | (PDF only) | (PDF only) |

**What this buys the pipeline**

* **Meet-ID discovery removed for every MHSAA postseason meet.** One MHSAA page request yields 36-48 ids at once, versus searching/enumerating meets on Athletic.net (which is Cloudflare-403 to non-browser clients from this machine, verified 2026-09-19).
* **Deterministic MeetID + division + host + participating schools** for a whole season, so the Athletic.net calls can go straight to meet-level endpoints (`MeetResults`/`GetMeetData` style) with no per-meet lookup.
* **Class-of-2027 evidence that never touches Athletic.net**: finals PDFs carry grade. Sampled: `2025lpgd1final.pdf` = 187 graded finishers, **47 with `Year = 11`**; `2026-UP-Boys-D1-Finals.pdf` = 103 graded rows, **30 with `Yr = 11`**. That gives an independent AN-grade cross-check for the top XC runners of each division and for UP TF finalists.
* **School/team seeding**: 755 MHSAA IDs + names + enrollments let the pipeline resolve Michigan schools without AN team search; the AN TeamID itself is **not** exposed by MHSAA (must be resolved once elsewhere, then cached).
* **Request-avoidance estimate (conservative)**: 89 AN meet ids/season (48+5 TF, 36 XC) - if the current pipeline spends even 1-2 AN requests per meet to discover/confirm a meet, MHSAA removes ~89-178 AN requests per season for Michigan, plus any per-athlete profile requests that become unnecessary for the graded finishers MHSAA already names. Graded-row volume [INFERENCE - extrapolated from two sampled PDFs at ~100-190 rows each]: 14 XC finals PDFs + 6 UP TF finals PDFs per season ≈ 2,000 athlete-rows/year carrying name, school, grade and mark.

### Athlete evidence

Field-by-field, from MHSAA alone:

| Attribute | Available | Where | Note |
|---|---|---|---|
| Name | yes (finals only) | finals PDFs (`2026-UP-Boys-D1-Finals.pdf`, `2025lpgd1final.pdf`) | regionals have no MHSAA-hosted results at all |
| Grade | yes (finals only) | `Yr` (TF) / `Year` (XC) column | grade 11 = Class of 2027 |
| School | yes | `Team` column / SCHOOLS ASSIGNED lists | school name, not MHSAA id, inside result PDFs |
| Gender + sport + category | yes | event headers (`Girls 1st Starting Ht.` / PDF section headers) | boys/girls x TF/XC |
| City / state | state only (MI implied) | - | no city column anywhere in MHSAA data |
| Performances | finals only | PDF mark columns | regional marks live at Athletic.net |
| PR / progression | no | - | per-year archives allow manual reconstruction only |
| Meets attended | partial | participating-school lists (regionals), finals PDFs | no athlete-meet cross index |
| Athlete profile URL | no | - | MHSAA never links athletes |
| Indoor results | no | - | MHSAA sanctions no indoor series |

### Recruiting information

| Field | Available? | Evidence |
|---|---|---|
| Head track coach | **partial** - host school only | Region 24 (Bangor HS) Meet-Info: `Marc Hunter, Head Boys Coach - mhunter@bangorvikings.org`, `Ben Munoz, Head Coach Girls - bmunoz@bangorvikings.org` |
| Head XC coach | **no** (only meet administrators) | 2025 XC Region 1 (Allendale) Meet-Info: `Jason Dykstra, Meet Director jason.dykstra@apsfalcons.net`; no visiting-school coaches |
| Assistant coaches | no | not published anywhere on mhsaa.com |
| Athletic director | **partial** - host school only | Region 24 Meet-Info: `Fred Smith, Athletic Director - fsmith@bangorvikings.org (269)427-6800` |
| Public professional email | **partial** - host school only | Region 2 Meet-Info: Zeeland East athletic secretary `gbroekhu@zps.org`; Allendale: `don@michianatiming.com` (timer) |
| School athletics website | no | MHSAA data carries school names only |
| Team website / social | no | - |
| Statewide coach directory | **no such directory exists** | `/schools` landing page (professional-development page, no school list); `/schools/5792` -> **404**; `/sports/boys-cross-country/adcoach` returns an empty shell (JS-rendered, no coach data in HTML); `my.mhsaa.com` AD/Coach services are login-gated and were **not** accessed |

Contact harvest value: every regional Meet-Info PDF is produced by the host school, so at most one school's AD/coaches per meet:
48 TF regionals + 36 XC regionals ≈ **84 host-school contact opportunities per year** (fewer once blank/no-PDF rows are excluded: 2026 TF had 35/48 PDFs, 2026 XC 18/36 to date). Every contact seen is a role-based professional address published for the school's athletic program - exactly the allowed class; sampled 3 of 3 PDFs that carry text all contained at least one such email.

### Result evidence

| Field | Available? | Evidence / note |
|---|---|---|
| ResultID | no | PDFs only; no ids |
| AthleteID | no | - |
| MeetID | **AN MeetID yes** | from the link; MHSAA itself has no meet id |
| EventID | no | event names (`Boys' 110m Hurdles D1`); finals PDFs print a per-meet event ordering number |
| Mark | **yes** (finals only) | `2025lpgd1final.pdf` XC: `Pl | Bib | Pts | Name | Year | Time | Pace`; TF: `Athlete | Yr | Team | Finals | Pts` |
| Timing/normalization | partial | Meet-Info states entries must be FAT to .00 or handheld +.24 - the exact normalization rule the pipeline applies |
| Wind | **yes** for LP TF finals | `Wind` column in 2024/2023 LP finals PDFs; not present in UP finals list-style PDFs |
| Implement/hurdle spec | partial | Meet-Info PDFs state hurdle heights/starting heights (e.g. r24: `Girls 1st Starting Ht. 7'`) |
| Heat / round / section | yes | PDFs show `Finals` sections, prelims, flights |
| Place | yes | `Pl` column / ranking prefix |
| Date | yes | PDF header `Sat, May 30, 2026`; page slug year |
| School represented | yes | `Team` column (school name, no MHSAA id) |
| Relay membership | **yes** | `MARQUETTE (Joe Graci, Jamison Cihak, Conrad Esslin, Evan Milton)` - order preserved |
| Parse risk | **real** | Meet-Info PDFs vary: text-layer (r2.pdf, r24.pdf - Calibri, `pdftotext` clean) vs scanned images (`benzie-central.pdf` = RICOH IM C2510 scan, 4 pages, **no text layer** -> OCR required) |

Finals PDFs are the only MHSAA-native result evidence; regional results and all regular-season marks exist only at Athletic.net.

### Incremental use

* **New meets appear** once per year per sport, in bursts: MHSAA adds the season's regional page in May (TF) / August-October (XC) and appends one line per regional as hosts submit Meet-Info files (2026 XC page note: "information will be posted periodically as submitted by sites").
* **Weekly poll plan (cheap, 1 request each):**
  1. TF: `/sports/boys-track-field` (finals AN links appear late May) and `/{year}-mhsaa-track-field-regional-results`.
  2. XC: `/sports/boys-cross-country` and `/{year}-mhsaa-lp-cross-country-regional-info-and-results` (AN RESULTS links appear the weekend of the regionals, late October in 2026).
  Both pages are ~200 KB HTML; diff the set of `athletic.net/{TrackAndField|CrossCountry}/meet/(\d+)` hrefs -> every new id is a new meet to enqueue.
* **Change detection without downloading PDFs**: MHSAA appends a cache-buster to file links, e.g. `...2026-UP-Boys-D1-Finals.pdf?time=1780175792370`; the value is a millisecond epoch (decoded: 1780175792370 = 2026-05-30T21:16:32Z) that changes when the file is re-uploaded. Diffing `?time=` gives free "this PDF changed" signals; re-download only when it moves.
* **Affected-athlete refresh**: a changed/new finals PDF names both the athletes and their schools; enqueue only those schools' roster refreshes instead of a state sweep.
* **Affected-school lists**: `SCHOOLS ASSIGNED` inside Meet-Info PDFs (16 schools in Region 24, 14 in Region 2) plus the region-host mapping let the pipeline pre-seed which schools will appear in each regional before results exist.
* **Season rollover**: after finals, the per-year regional page stays at a stable URL forever, and the hub's archive link becomes the historical entry point; nothing needs to be recomputed, so re-crawls can be limited to the current year.
* **Cost per season (measured shape, not measured wall time)**: 4 hub pages + 2 regional pages + 1 archive page x 2 sports ≈ **12 HTML requests** to learn every postseason MeetID; PDFs only for grade/contact extraction.

### Access characteristics

* **Class: normal HTML + static PDF** (`www.mhsaa.com`, Drupal 10 on Pantheon - verified headers: `x-generator: Drupal 10`, `x-pantheon-styx-hostname`, `server: cloudflare` with `cf-cache-status: DYNAMIC`, i.e. **Cloudflare fronts the site without challenging** - the opposite of Athletic.net's 403). No auth, no CAPTCHA, no JS required for the result/regional/archive pages.
* **Polling is cheap and conditional-friendly**: responses carry `cache-control: max-age=3600, public`, `last-modified` (e.g. `2026-09-20 02:34:24 GMT` for the TF hub) and `x-drupal-cache: HIT`, so a collector can poll with `If-Modified-Since` and skip bodies; the PDF `?time=<epoch-ms>` values give the same signal per file.
* **JS-only surfaces** (must render or must not be used): `my.mhsaa.com/Sports/{Sport}/School-Division-List-{year}-{LP|UP}` (Knockout SPA - the division list is fetched client-side; the HTML shell was static, data was not); `/Sports/.../assignments/{regional|final}-{lp|up}` (DNN module `MHSAA-ATFTournamentManager`, data via its own API base in `DesktopModules/.../Configuration/config.js`); `my.mhsaa.com` AD/Coach services behind login.
* **robots.txt**: `Disallow: /core/`, `/profiles/`, `/admin/`, `/search/`, `/user/*`, `/README.md`, `composer/*`; content paths used here are allowed. No `Crawl-delay` directive.
* **No API, no documented rate limit, no 429, no `Retry-After` observed** in this session.
* **Request volume actually observed (honest count)**: **63 requests to `www.mhsaa.com`** (38 helper GETs + 22 curl + 2 urllib probes + 1 HEAD) and **4 to `my.mhsaa.com`**, all sequential, between 23:12 and 23:22 local, ~1 request/5 s. This is **above the brief's ~50-per-host courtesy guideline (≈1.26x)** - the overshoot came from re-fetching the same pages across analysis passes (the fetch helper cached within a pass). A production collector needs each page exactly once: a full-season refresh is ~30 requests including PDFs, and `If-Modified-Since` polling makes the steady state a couple of conditional GETs per week.
* **Blocking observed**: none from MHSAA (no 403/429/Retry-After/CAPTCHA anywhere). Two 404s came from my own guessed URLs and are reported as findings: `/schools/5792` -> there is no per-school page, and `Track Field-Girls/...` -> TF PDFs for both sexes live under `Track Field-Boys` (XC is different: it keeps a real `Cross Country-Girls` directory for 2022/2023 girls finals PDFs).
* **Sitemap** `https://www.mhsaa.com/sitemap.xml?page=1..7` = 12,046 URLs, `lastmod` 2026-09-19T11:36 - useful for discovering story pages only, not data.

### Recommendation

**ATHLETIC.NET-SEED** (primary, for Michigan postseason meets) with a secondary **VALIDATION** role for Class-of-2027 grade evidence and a marginal **COACH-DIRECTORY** role.

Why:

* **Expected marginal coverage**: MHSAA deterministically yields **53 TF + 36 XC = 89 AN MeetIDs per season** (48 TF regionals + 5 TF finals; 36 XC regionals; XC finals stay PDF-only), plus the host/division and participating-school context for each. That removes AN meet discovery for every Michigan postseason meet (~89-178 AN requests/season avoided under a 1-2-request-per-meet assumption) and is oracle-grade: ids come from MHSAA's own hrefs, no AN request needed to obtain or trust them.
* **Independent grade verification** (the thing AN scraping cannot be trusted for): XC finals PDFs and UP TF finals PDFs carry grade per finisher - 47/187 grade-11 in `2025lpgd1final.pdf`, 30/103 in `2026-UP-Boys-D1-Finals.pdf`. Also note the unlinked-but-predictable girls UP PDFs (`.../Finals/{year}-UP-Girls-D{1,2,3}-Finals.pdf` in the **Boys** directory; D1 verified 200) which the girls hub never links.
* **Team/school universe for free**: 755 MHSAA schools with stable numeric ids, enrollment, class, per-sport division sizes and 33+33 co-op program entries - no AN requests for school identity or division context.
* **Marginal coach value (low but real)**: ~84 host-school AD/coach role emails per year from regional Meet-Info PDFs; not a directory, and OCR is needed for scanned ones.
* **Why not PRIMARY**: no regular-season meets, no athlete-level enumeration, no team/athlete AN ids, postseason-only, and grade data is limited to finalists.
* **Cost**: ~12 HTML requests per season + optional PDFs; lowest-friction source seen in this assignment (no Cloudflare, no auth, no rate limiting).
* **Trap to avoid**: MHSAA's `boys-` URLs are the superset (the girls hubs link boys-hosted regional pages and boys-named UP PDFs); scrape the boys paths first, then construct the `-Girls-` variants.

### Evidence appendix

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.mhsaa.com/robots.txt` | GET (curl) | 200 | Crawl rules: /core/, /profiles/, /admin/, /search/, /user/* disallowed; result/regional/archive paths allowed; no Crawl-delay | 2026-09-19T23:12:42-05:00 |
| `https://www.mhsaa.com/sports/boys-track-field` | HEAD (curl) | 200 | Response headers: `x-generator: Drupal 10`, `server: cloudflare` + `cf-cache-status: DYNAMIC` (no challenge), `cache-control: max-age=3600, public`, `last-modified: 2026-09-20 02:34:24 GMT`, `x-drupal-cache: HIT`, `x-pantheon-styx-hostname` | 2026-09-19T23:24:46-05:00 (date header 04:24:46 GMT 2026-09-20) |
| `https://www.mhsaa.com/sitemap.xml` | GET (curl) | 200 | Sitemap index = 7 pages, lastmod 2026-09-19T11:36 | 2026-09-19T23:12:44-05:00 |
| `https://www.mhsaa.com/sitemap.xml?page=1..7` | GET (curl) x7 | 200 | 12,046 URLs total; stories only - no school pages, no result data | 2026-09-19T23:12:46-05:00 |
| `https://www.mhsaa.com/sports/boys-track-field` | GET | 200 | TF hub: "Tracking the Tournament" = finals AN ids 622966/622969/622972/622973/622974 + link to 2026 regional page | 2026-09-19T23:13:07-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2026-mhsaa-track-field-regional-results` | GET | 200 | 48 regional rows, each 1 AN Results link + 1 JH/MS zone row = 49 MeetIDs -> full 2026 TF regional map | 2026-09-19T23:13:11-0500 |
| `https://www.mhsaa.com/sports/girls-track-field` | GET (in-session cache) | 200 | Girls hub links the BOYS regional page (girls path has none) and only 4 LP finals ids (622974 missing) -> boys path is the superset | 2026-09-19T23:13:29-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/2025-mhsaa-track-field-regional-results` | GET | 200 | 2025 TF: 48 regionals, AN ids 575276..577273 (40 LP + 8 UP) | 2026-09-19T23:14:00-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2024-mhsaa-track-field-regional-results` | GET | 200 | 2024 TF: 48 regionals, AN ids 522051..533716 | 2026-09-19T23:14:02-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2023-mhsaa-track-field-regional-info-and-results` | GET | 200 | 2023 TF: 48 AN regional links (481015 Midland R1 ...) in "Host Region N - [Meet Info|PDF] | [Results|AN]" rows | 2026-09-19T23:21:32-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/results-archive` | GET | 200 | Finals history 2016-2026: AN ids + legacy Print/EntryMeet.aspx?Meet=... qualifier links (2016-2019) | 2026-09-19T23:13:52-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country` | GET | 200 | XC hub: LP+UP finals block = PDFs only; zero AN meet links on the hub | 2026-09-19T23:13:30-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country` | GET | 200 | Girls XC hub links the BOYS 2025 XC regional page; zero AN meet links | 2026-09-19T23:13:31-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2026-mhsaa-lp-cross-country-regional-info-and-results` | GET | 200 | 2026 XC (live): 36 LP regions + 17 JH/MS zones; 18 and 9 have Meet-Info PDFs; 0 AN result links yet; "posted periodically as submitted by sites" | 2026-09-19T23:13:35-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2025-mhsaa-cross-country-regional-info-and-results` | GET | 200 | 2025 XC: 36/36 regions with AN RESULTS ids 256167..256215 (full map in this report) | 2026-09-19T23:13:37-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2024-mhsaa-cross-country-regional-info-and-results` | GET | 200 | 2024 XC: 36 regional ids 238239 (Allendale R1) .. 238292 (Royal Oak R36) + JH/MS Zone 6 id 238694 = 37 AN ids | 2026-09-19T23:14:11-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2023-mhsaa-cross-country-regional-info-and-results` | GET | 200 | 2023 XC: 36 ids 223261 .. 223335 (Royal Oak Shrine Catholic R36) | 2026-09-19T23:14:13-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/results-archive` | GET | 200 | XC archive by year: 2025/2024 = 8 LP + 6 UP finals PDFs; 2023/2022 = 14 PDFs split across Cross Country-Boys/-Girls dirs; 2016/2018 legacy AN links; 2021 = athletic.net/events/usa/michigan/2021-10-29; 2022 JH/MS = 8 AN zone ids (207985-207992, 208580); 2012-2013 = my.mhsaa.com .txt files -> finals are PDF/legacy, never AN meet ids | 2026-09-19T23:14:04-0500 |
| `https://www.mhsaa.com/i-am/administrators/enrollment-classification-co-ops` | GET | 200 | One page links the 3 PDFs that define the whole state school universe | 2026-09-19T23:14:40-0500 |
| `https://www.mhsaa.com/sites/default/files/Enrollment%20and%20Classification/26-27-Enrollment-List.pdf` | GET (curl) | 200 | 755 member HS: MHSAA id, name, enrollment total, classification enrollment (e.g. 5792 East Kentwood 3042; 4638 Detroit Catholic Central 1046/2092) | 2026-09-19T23:14:48-05:00 |
| `https://www.mhsaa.com/sites/default/files/Enrollment%20and%20Classification/classification-by-sport-2026-27.pdf` | GET (curl) | 200 | Per-sport division sizes: TF boys 154x4, girls 150/157/156/152; XC boys 142/142/143/143, girls 136/144/145/141; classes A188 B189 C189 D189 | 2026-09-19T23:14:49-05:00 |
| `https://www.mhsaa.com/sites/default/files/Enrollment%20and%20Classification/hscoop.pdf` | GET (curl) | 200 | Approved co-op programs with separate program ids: (1804)Ada Forest Hills Eastern(735) vs that school MHSAA id 4506; 33 XC + 33 TF varsity entries | 2026-09-19T23:14:50-05:00 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/r2.pdf` | GET (curl) | 200 | Region 2 Meet-Info (Zeeland East): 14 participating schools, host contact email, entries "must be FAT times ... handheld +.24", live scoring at FATResults.com | 2026-09-19T23:18:05-05:00 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Regionals/r24.pdf` | GET (curl) | 200 | Region 24 Meet-Info (Bangor): AD + head boys/girls coach professional emails; 16 SCHOOLS ASSIGNED; "All entries need to be made on athletic.net" | 2026-09-19T23:19:13-05:00 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Boys-D1-Finals.pdf?time=1780175792370` | GET (curl) | 200 | UP finals export: Team Scores + 17 boys events, Athlete|Yr|Team|mark|Pts; 103 graded rows, 30 grade-11; ?time= decodes to 2026-05-30T21:16:32Z (upload timestamp) | 2026-09-19T23:17:54-05:00 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Girls-D1-Finals.pdf` | GET (urllib) | 200 | GIRLS UP finals PDF exists inside the "-Boys" directory and is NOT linked from the girls hub -> constructible by pattern (D2/D3 not probed; the girls hub links the boys PDF instead) | 2026-09-19T23:21:50-05:00 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Girls/2026/Finals/2026-UP-Girls-D1-Finals.pdf` | GET (urllib) | 404 | No "Track Field-Girls" directory: TF PDFs (both sexes) live under Track Field-Boys. By contrast XC keeps a real "Cross Country-Girls" directory for 2022/2023 girls finals PDFs | 2026-09-19 23:12-23:22 CDT (session window) |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Regionals/r2.pdf` | GET (curl) | 404 | A guessed path is wrong: the actual r2 link is /2026/r2.pdf (no /Regionals/) -> Meet-Info paths are inconsistent | 2026-09-19 23:12-23:22 CDT (session window) |
| `https://www.mhsaa.com/sites/default/files/Cross%20Country-Boys/2025/Regionals/allendale.pdf` | GET (curl) | 200 | XC regional Meet-Info: divisions hosted, meet director + timer emails, "Registration ... on athletic.net", MS zones for JH | 2026-09-19T23:18:07-05:00 |
| `https://www.mhsaa.com/sites/default/files/Cross%20Country-Boys/2025/Regionals/benzie-central.pdf` | GET (curl) | 200 | RICOH scan, 4 pages, no text layer (pdffonts empty) -> OCR required for some Meet-Info PDFs | 2026-09-19T23:18:32-05:00 |
| `https://www.mhsaa.com/sites/default/files/Cross%20Country-Boys/2025/Finals/2025lpgd1final.pdf` | GET (curl) | 200 | XC finals results with grade: Pl|Bib|Pts|Name|Year|Time|Pace; 187 finishers, 47 grade-11 (Class of 2027) | 2026-09-19T23:17:55-05:00 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2024/Finals/LP-D1-Individuals.pdf` | GET (curl) | 200 | 2024 LP TF finals PDF carries Yr (grade) AND Wind columns -> grade + wind without Athletic.net | 2026-09-19T23:18:31-05:00 |
| `https://www.mhsaa.com/schools` | GET | 200 | No school list on this page (professional-development landing) - MHSAA publishes no browsable school directory | 2026-09-19T23:14:16-0500 |
| `https://www.mhsaa.com/schools/5792` | GET | 404 | No per-school page pattern exists (MHSAA id 5792 = East Kentwood) | 2026-09-19T23:15:10-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/adcoach` | GET | 200 | AD/Coach page is an empty JS shell: no coach directory data in HTML | 2026-09-19T23:16:04-0500 |
| `https://my.mhsaa.com/` | GET | 200 | Member portal landing; AD/Coach services require login (not attempted) | 2026-09-19T23:15:08-0500 |
| `https://my.mhsaa.com/Sports/Boys-Track-Field/School-Division-List-2026-27-LP` | GET | 200 | Division list is a Knockout SPA - data arrives client-side, no server-rendered school/division rows | 2026-09-19T23:15:16-0500 |
| `https://my.mhsaa.com/DesktopModules/MHSAA-ATFTournamentManager/js/Configuration/config.js` | GET | 200 | Tournament manager module config (API base) - the only machine-readable route into assignments/divisions | 2026-09-19T23:15:49-0500 |
| `https://www.mhsaa.com/Sports/Boys-Track-Field/assignments/regional-lp?uplpcode=lp&year=2025-26` | GET | 200 | Assignments page is a JS shell - per-regional school lists are not in HTML (unlike Meet-Info PDFs) | 2026-09-19T23:15:18-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/rankings` | GET | 200 | MHSAA "rankings" = narrative page, no structured rankings data | 2026-09-19T23:15:06-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/todays-schedule` | GET | 200 | Schedule page empty shell - MHSAA publishes no regular-season schedule | 2026-09-19T23:16:03-0500 |
| `https://www.mhsaa.com/about/general-resources/sports-participation-listings` | GET | 200 | Participation listings page (schools per sport) - links out, no per-school data | 2026-09-19T23:18:46-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/results-archive (2026-09-19 re-read)` | GET | 200 | Year-by-year XC artifact map - the source of the XC finals PDF inventory bullet | 2026-09-19T23:14:04-0500 |
