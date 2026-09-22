# 13. Illinois IHSA

Status: complete
Observed on: 2026-09-19

### Source

Illinois High School Association (IHSA), the sole state association for Illinois high-school
athletics. Illinois is a single-association state (no separate boys/girls associations as in Iowa).

| Surface | URL | What it is |
|---|---|---|
| Current site (SPA) | `https://www.ihsa.org/` | React/Vite single-page app; all data comes from the API. Route table recovered from `/assets/main-B2rKN2jy.js`. |
| Public JSON API | `https://api.ihsa.org/v1/...`, `https://api.ihsa.org/v2/...` | Undocumented but unauthenticated JSON used by the site itself. No OpenAPI/doc route (`/docs`, `/openapi.json` both 404). |
| Legacy static tree | `https://www.ihsa.org/data/{sport}/...` | Plain HTML state-series pages (qualifiers etc.), still generated every season. |
| Asset/document CDN | `https://ihsa-assets-prod.nyc3.cdn.digitaloceanspaces.com/...` | Heat-sheet/winner-report PDFs, programs, school logos. |
| Legacy scoreboard host | `https://scorezone.ihsa.org/img/logos/thumbs/{School}.jpg` | Still referenced for school logos in API payloads. |
| State-final scoring partner | `https://live.athletic.net/meets/74003` (boys), `74002` (girls) | IHSA links Athletic.net Live as the official live scoring/timing surface for the T&F state finals. |

Note: this is **not** a scraping-hostile site — `robots.txt` is `User-agent: * / Allow: /`
and the API answers plain `curl` with no cookie, no auth, no CAPTCHA, no Cloudflare challenge.

### Coverage

* **State:** Illinois only. `Illinois` is also explicit in the API (`State: "IL"` on every school).
* **Schools:** 828 member-school rows in `/v1/schools` — 801 `full member`, 26 `approved school`,
  1 `associate member`; 604 boundary / 224 non-boundary; 677 public / 151 private;
  126 Chicago Public Schools flagged (`isCPS: "true"`). `SchoolID` range 0101–7097;
  18 records have no latitude/longitude.
* **Sports relevant here (verified in `/v1/sports` and `/v2/classification`):**
  `TRB` Boys Track & Field (spring), `TRG` Girls Track & Field (spring),
  `CCB` Boys Cross Country (fall), `CCG` Girls Cross Country (fall). Each has classes `1A/2A/3A`.
  IHSA does **not** run an indoor track state series (no indoor activity in the 45-entry `/v1/sports`
  list); indoor is a non-IHSA/independent season in Illinois. A separate `Athletes with Disabilities`
  activity exists (`AWD`), and wheelchair T&F is scored inside the main meet as class `WD`
  (4 wheelchair events in the 2026 boys state finals).
* **School-level coverage (2026-27 classification, `/v2/classification/classifications?season=…`):**
  * spring `S`: TRB **632** schools (267× 1A, 168× 2A, 197× 3A; 562 with `EntryStatus: Y`),
    TRG **624** schools (261/168/195; 558 entered).
  * fall `F`: CCB **539** schools (245/140/154; 531 entered), CCG **507** schools (215/141/151; 499 entered, 2 `W`).
  * Rows also carry `ActualEnroll`, `EffectiveEnroll`, `MultipliedEnroll`, `TourneyEnroll`,
    `MultiplierNB`, `SuccessFactor`, `Coop`/`CoopWith`/`CoopName`, `TourneyWaiver`.
* **Season / historical depth (the hard limit):**
  * `/v1/terms` → `{"currentTerm":"2026-27","terms":["2025-26","2024-25","2023-24"]}` (all `routes:"*"`).
  * Track & field **results** API contains exactly **2 meets** (`/v1/track-field/meets` → `count: 2`):
    the 2026 boys and girls state finals. `/v1/track-field/meets/2025?gender=Boys` → 404 `Meet not found`.
    There is no historical T&F result depth in this API.
  * Cross-country **qualifier rosters** exist for term `2025-26` only:
    `/v1/{2024-25}/statefinal/cc-qualifiers` returns the right envelope with **empty** arrays;
    `/v1/2026-27/...` → 404 `Archive not available for term 2026-27`.
  * Static qualifier pages (`/data/trb/1qual.htm` …) are **overwritten each season** — the current
    copies are the 2025-26 (May 2026) state finals; no archive path found
    (`/data/trb/`, `/data/trb/index.htm`, `/data/trb/1index.htm` all 404).
  * Document index `/v1/2025-26/sports/boys-track-field/tournament-results` → 4 heat-sheet PDFs;
    the same call for `2024-25` → 0 documents.
* **Typical Illinois T&F volume per season** (parsed from the six 2026 qualifier pages):
  **3,342 athlete-entry lines**, grade split `Sr 1,295 / Jr 1,079 / So 636 / Fr 332`;
  deduped to **2,555 unique (name, grade, school) triples, 838 of them Jr.**

### Enumeration

All recipes below are exact request patterns; each was executed once during this report.

1. **Schools (complete universe, 1 request):**
   `GET https://api.ihsa.org/v1/schools` → `{"data":[…828 rows…]}` (451,188 bytes).
   Row keys: `SchoolID, nameFormal, NameIHSA, Address, POBox, State, Zip, city, DirectorySort,
   membershipType, type, schooltype, enrollmentType, hasBoundary, isCPS, Latitude, Longitude,
   schoolLogo, Color1/Color2, TextOnColor1/2, HasColors, ColorSource`.
   The SPA pages `/schools/index`, `/schools/school-directory`, `/schools/school-directory/all`
   and `/schools/school-directory/:letter` all read this same payload (verified in
   `SchoolIndexPage-C45gepzG.js`, `SchoolsPage-DTFtknRu.js`), so no per-letter requests are needed.
2. **School detail (1 request/school):** `GET /v1/schools/{SchoolID}` → phone, fax, county,
   district, IASA super-region, division, nicknames, enrollment (`Enrollment`, `EnrollmentString`),
   `URL` (school/district website), `AthleticsURL` (null in all 11 sampled records),
   `NFHSNetwork_School_Landing`, `GoFan_School_Landing`, `Conferences[]` (conference name per school),
   `Latitude/Longitude`. Example: `/v1/schools/0101`.
3. **School × sport × class matrix (1 request per season = the best enumeration primitive):**
   `GET https://api.ihsa.org/v2/classification/classifications?season=S` (spring, 1,789,503 bytes),
   `?season=F` (fall, 2,271,933 bytes), `?season=W` (winter, 1,694,748 bytes).
   Rows are `(SchoolID, ActivityID, ActivityName, Class, TournamentClass, EntryStatus, …)`.
   Filter `ActivityID in {TRB,TRG}` for track, `{CCB,CCG}` for XC → the Illinois school universe
   with class and entry status in one call. (`?season=2026-27` returns `{"data":[]}`; the parameter
   is the single-letter season code, not the term.)
4. **Per-school state-series entries:** `GET /v1/schools/{SchoolID}/entries` → for each activity the
   school entered: `TournamentID, SchoolTerm, ActivityID, ActivityName, Season, Class, EntryStatus,
   EntryType, TotalEnrollment, Coop, Waiver, MaxPrepsID/MaxPrepsGenderSport` (MaxPreps IDs were
   `null` here), plus a `teamrecord` stub. Only the **current term** is returned (all 19 rows for
   0101 were `2026-27`), so season-over-season comparison uses the classification endpoint instead.
5. **Meets:** `GET /v1/track-field/meets` → 2 records
   (`MeetId 74003`, `Year "2026"`, `Title "2026 IHSA Boys State Track & Field"`, `DataSource "stored"`,
   `IsActive`, `CreatedAt/UpdatedAt/LastRefreshedAt`, `LastRefreshSummary{eventsDiscovered,
   eventsUpserted, resultsDocsChanged, scoreGroupsDiscovered, teamsDocChanged, errors[]}`).
   Girls counterpart is `74002` (constant in `StateSeries-w2fH3odu.js`: `pt={boys:"…/meets/74003",girls:"…/meets/74002"}`).
   `GET /v1/track-field/meets/2026?gender=Boys|Girls`,
   `…/score-groups?gender=…` (4 class buckets: 1A/2A/3A/WD),
   `…/events?gender=…` (97 events for boys: 39 prelims, 39 finals, 15 no-round, 4 wheelchair).
   Cross-country has **no meet-list endpoint**; it is enumerated by state-series tournament id
   `{1A:688, 2A:689, 3A:690}` (boys) and `{691,692,693}` (girls), taken from
   `StateSeries-w2fH3odu.js` (`x={"boys-cross-country":{"1A":"688",…}}`).
   `GET /v1/{term}/statefinal/cc-qualifiers?tournamentId=688` returns the qualifier roster.
   Regular-season meets are **not** enumerable from IHSA (no calendar/site/schedule data:
   `/v1/sports/boys-track-field/calendar` → 404; `/v1/{term}/sports/boys-track-field/sites|schedules`
   → 400 `Missing required parameter: class` with `availableClasses: []`).
6. **Athletes / Class of 2027:** IHSA enumerates only *postseason* athletes.
   * T&F state-final **qualifiers**, static HTML, 6 files, no auth:
     `https://www.ihsa.org/data/trb/1qual.htm`, `…/trb/2qual.htm`, `…/trb/3qual.htm`,
     `…/trg/1qual.htm`, `…/trg/2qual.htm`, `…/trg/3qual.htm`
     (56 KB–62 KB each; layout `mark  First Last (Gr.), School`). These are the exact pages the SPA
     loads through `https://api.ihsa.org/v1/proxy/data?url=https%3A%2F%2Fwww.ihsa.org%2Fdata%2Ftrb%2F1qual.htm`.
   * XC state-final qualifiers via API (`boxAssignments`, `teamQualifiers`, `individualQualifiers`),
     with `yearInSchool`, box, team place and **coach names** per school.
     Measured for boys: 1A 398 athletes / 30 team qualifiers / 42 individual qualifiers,
     2A 396 / 28 / 30, 3A 420 / 28 / 32 → **1,214 boy qualifiers, 358 of them grade 11**.
     Girls use ids 691/692/693 (not fetched; [UNVERIFIED]).
7. **Results:** state finals only — `GET /v1/track-field/events/{eventId}/summary`
   (eventId list from `…/meets/2026/events?gender=…`). Per-event results with athlete objects,
   places, marks, relays, PB flags. No sectional/regional result data exists in the API
   (see Result evidence).

### Stable identifiers

| Identifier | Where | Shape / example | Stability |
|---|---|---|---|
| `SchoolID` | `/v1/schools`, all entry/result payloads | 4-char string `"0101"` … `"7097"` | Stable association key; use this, never the name (`NameIHSA` `"Abingdon (A.-Avon)"` vs `nameFormal` `"Abingdon-Avon High School"` vs `NameShort` all differ). |
| `PersonID` | `/v1/schools/{id}/staff2` | int `96256` | Stable across schools/roles (same PersonID appears for both boys and girls T&F coach at 0101). |
| `RoleID` | same | `HCB-TRB`, `HCG-TRG`, `HCB-CCB`, `HCG-CCG`, `C1-BoysAD`, `D1-GirlsAD`, `E1-ActivDir`, `A1-Super`, `B1-Prin`, … | Enumerated role vocabulary; safe for role matching. |
| `ActivityID` | sports, entries, classifications | `TRB`, `TRG`, `CCB`, `CCG` (+`FB`, `VBG`, …) | Stable per sport. |
| `TournamentID` | entries | `TRB.2026-27.1A`, `CCB.2026-27.1A`, `BWL.2026-27` | Deterministic composition `{ActivityID}.{SchoolTerm}.{Class}`; class-less tournaments omit the suffix. |
| `MeetId` | `/v1/track-field/meets` | `74003` (boys), `74002` (girls) | Equals the **Athletic.net Live meet id** (`live.athletic.net/meets/74003`). |
| `eventId` | events/summary | `"2790204"`, `"500937"` | Stable per event instance (class+round+event). |
| `ScoreId` | score-groups | `"genDivMSR\|74003\|114433\|3"` | Composite; embeds meet + group id + round. |
| Athlete id | event summary `finishers[].athlete` | `athleticNetId: 27740691`, `athleticLiveId: 49752378` | **Athletic.net AthleteID is delivered by IHSA** for T&F state-final finishers (incl. relay members). |
| Team id | `finishers[].team` | `athleticNetId: 16352`, `athleticLiveId: 1679604` | Athletic.net TeamID for the school's T&F team. |
| Tournament id (XC) | `cc-qualifiers` | `688`…`693` | Hard-coded constants in the site bundle; not discoverable from an API index. |

No IHSA athlete ID exists outside the state-finals result feed — qualifier pages and XC qualifier
lists are **name+school+grade only**, so name/school matching there is not identifier-grade.
Also note `placesScored: 0` on every event I inspected, and `header.isFinalResultsRecorded: false`
on the HJ final — do not treat IHSA "final" flags as complete scoring verification.

### Athletic.net leverage

IHSA is the strongest Athletic.net-leverage source found in Illinois: it hands over Athletic.net
IDs and Athletic.net URLs without any Athletic.net request.

* **T&F state-finals meet IDs by class/gender (constants in `StateSeries-w2fH3odu.js`):**
  `boys: {1A:"663321", 2A:"663320", 3A:"663319"}`, `girls: {1A:"663324", 2A:"663323", 3A:"663322"}`,
  used to build `https://www.athletic.net/TrackAndField/meet/{id}/{qualifiers|heat-sheets|state-results}`
  (observed rendered link: `Athletic.net Entries => https://www.athletic.net/TrackAndField/meet/663321/entries`).
* **XC state-finals meet IDs:** `268054` (1A), `268064` (2A), `268066` (3A) — same ids for boys and
  girls (one meet per class, both genders); rendered links
  `…/CrossCountry/meet/268054/entries` (Qualifiers button) and `…/CrossCountry/meet/268054/results` (Results button).
* **Athletic.net Live:** `https://live.athletic.net/meets/74003` (boys) / `74002` (girls) — linked from
  the state-central page as "AthleticLive Results"; these are the ids IHSA stores as `MeetId`.
* **Per-athlete/per-team Athletic.net IDs:** every T&F state-final finisher row carries
  `athlete.athleticNetId` + `team.athleticNetId` (+ `athleticLiveId`), including all four members of
  a relay. Sampling of the 4x800 m 1A final shows grade (`year`) for each relay member.
* **Ranking-list ids surfaced by IHSA:** each event summary embeds
  `rankings: [{label:"2026 High School Outdoor High School Rankings", link:"https://www.athletic.net/TrackAndField/rankings/list/168416/m/hj"}, {label:"2026 Illinois Outdoor High School Rankings", link:"…/rankings/list/169070/m/hj"}]`
  — note **169070** as the Illinois-scoped 2026 outdoor list (168416 is the national-one list id the
  mission brief already tracks).
* **No direct Athletic.net link** exists for XC individual results beyond the class-level meet links,
  and none for qualifier pages.

**Estimated Athletic.net source-request avoidance [INFERENCE, quantified where measured]:**
* T&F state-final cohort (~2,555 unique qualifiers, 838 Jr; 3,342 entries) — profile identification
  is free (IDs supplied). If the alternative were ranking enumeration + profile lookup, this removes
  ~1 request per finisher row plus the ranking pages; conservatively **≥1,000–2,000 Athletic.net
  requests per season** for the state-final cohort alone (counted from 97 boys events × ~20 finishers
  and the measured 2 events: 20 finishers HJ final, 12 teams 4x800 final).
* XC: 1,214 male qualifiers measured (358 grade 11) with name+school+grade but **no Athletic.net ID**;
  IHSA still saves the discovery step for these (deterministic Athletic.net seed: class meet +
  school + name + grade), at the cost of one targeted lookup per athlete rather than broad searching.
* 6 static pages + 2 API calls per season replace an unknown share of Athletic.net meet/results reads
  for the championship weekend.

### Athlete evidence

| Field | Available? | Evidence |
|---|---|---|
| Name | Yes | `athleteName` / `athlete.firstName`+`lastName` (results); `First Last (Gr.), School` (qualifier pages). |
| Graduating class / grade | **Yes** | `finishers[].year` = `"11"` (measured distribution in one 1A HJ final: 11→10, 12→4, 10→5, 9→1); XC `athletes[].yearInSchool`; qualifier pages `(Jr.)`. |
| School | Yes | `ihsaSchoolId`, `ihsaName`, `ihsaShortName`, `teamName`; XC `schoolName`+`ihsaSchoolId`. |
| City/state | Indirect | Not on athlete rows; join `SchoolID` → `/v1/schools/{id}` (`city`, `State:"IL"`). |
| Gender/category | Yes | `gender:"M"/"F"`, `genderLabel`, separate meets per gender. |
| TF vs XC distinction | Yes | Separate activities/endpoints. |
| Indoor/outdoor | Outdoor only | IHSA sanctions outdoor T&F only; no indoor state series. |
| Performances | Yes (state finals) | `mark` (`"2.02m"`, `"7:51.37"`), `imperialMark` micro-units (`2020000`), `seedMark`, `previousBest`, `newPersonalBest`, per-round splits. Qualifier pages: seed marks only. |
| PRs | Partial | `previousBest` / `newPersonalBest` for that event as tracked by the scoring feed (field-event values like `"6-08.00"`); not a career PR history. |
| Progression | No | No cross-season history in IHSA data (2026-only T&F results, 2025-26-only XC). |
| Meets | Yes for state finals | `meetId`, `scheduledTs`, `roundLabel`, `relatedRounds[]` (prelim ↔ final links). |
| Athlete profile URL | No | IHSA exposes `athleticNetId` but **no profile URL string**; URL construction is an Athletic.net-side question (assignment 5). |

### Recruiting information

IHSA maintains a real coach/AD directory in JSON, keyed by school + role.

* **Endpoint:** `GET https://api.ihsa.org/v1/schools/{SchoolID}/staff2` → grouped staff:
  `Administration`, `Boys Athletics - Head Coaches`, `Girls Athletics - Head Coaches`,
  `Athletic Medical Staff`, `Activities - …`, `Non-Competitive Activities - …`.
  Each row: `PersonID, Name, DefaultTitle, HasEmail, LastName, RoleID, Phone, Fax`.
* **Role vocabulary for this mission (observed):**
  `HCB-TRB` Boys Track & Field Head Coach, `HCG-TRG` Girls Track & Field Head Coach,
  `HCB-CCB` Boys Cross Country Head Coach, `HCG-CCG` Girls Cross Country Head Coach,
  `C1-BoysAD` / `D1-GirlsAD` Athletic Director, `E1-ActivDir` Activities Director.
* **Sample measurement (14 schools, every 60th `SchoolID` ascending; plus Abingdon 0101 = 15 schools):**
  Boys T&F head coach present **11/14**, Girls T&F **12/14**, Boys XC **9/14**, Girls XC **10/14**,
  Boys AD **14/14**, Girls AD **14/14**, Activities Director 9/14. 482 staff records examined;
  477 had `HasEmail: true`; **all 49 track/XC head-coach rows had `HasEmail: true`** (the 5
  `HasEmail:false` rows were superintendents/network CEOs and one cheer coach).
  Extrapolated to 632 TRB schools: ~500 boys T&F head-coach records [INFERENCE; 14-school sample,
  95% Wilson CI for 11/14 ≈ 55–91%].
* **Known gaps (measured, not inferred):** the listing is school-reported. `0242`… no — concretely
  `1432` Northfield (Christian Heritage Academy) and `2942` Chicago (Noble/Butler) both have
  `TRB`/`TRG` rows in `/v1/schools/{id}/entries` **and no T&F coach in `staff2`**;
  `2715` Chicago (DuSable) lists only `DWR` and no athletics coaches at all.
  So head-coach coverage ≈ 11/13 (85%) **among schools that actually enter T&F** [INFERENCE from the
  3 entries checks].
* **Public professional email:** `GET /v1/schools/{SchoolID}/staff/{PersonID}/email` → `{"email":"…"}`
  (200, no auth, no cookie). This is the same request the page's "Show email" button makes; the UI
  hides addresses by default with the note "to protect against spam, email addresses are hidden by
  default. Choose 'Show email' to reveal one." One address observed for evidence:
  `/v1/schools/0101/staff/96256/email` → `{"email":"jrakestraw@atown276.net"}` (Justin Rakestraw,
  Boys/Girls Track & Field Head Coach, Abingdon-Avon HS). **Retention rule for the pipeline: keep it
  to head T&F/XC coach + AD only, exactly as the mission permits, and do not expand into
  superintendent/principal or non-sport staff.** The reveal gate is a deliberate anti-spam control:
  use it at low volume, per school, and never in a loop that harvests every staff member.
* **Team/school web context:** `/v1/schools/{id}` provides `URL` (school or district website; in 11
  sampled records 10 were district/school sites, 1 — Elgin H.S. `0516` — was the athletics platform
  `https://il.8to18.com/elgin/dailycalendar`), `AthleticsURL` (**null in all 11 sampled**),
  `NFHSNetwork_School_Landing`, `GoFan_School_Landing`. Conferences come from `/v1/schools/{id}.Conferences[]`.
* **XC bonus:** `cc-qualifiers` responses embed the **coach names per qualifying school**
  (`coaches:[{coachType:"Head"|"Assistant", coachName}]`) alongside `ihsaSchoolId` — a second,
  season-frozen path to confirm who coaches XC (no emails there).

No athlete contact information of any kind is exposed by IHSA (no athlete email/phone/address fields
exist in any payload inspected).

### Result evidence

`GET /v1/track-field/events/{eventId}/summary` (110 KB for a 20-finisher field event):

| Field wanted | Present? | Notes |
|---|---|---|
| ResultID | **No** | No result primary key; identify a row by `(eventId, heat, lane/place)` or `(eventId, athlete.athleticNetId)`. |
| AthleteID | Yes | `finishers[].athlete.athleticNetId` (+ `athleticLiveId`) for finals; relay members carry the same object. |
| MeetID | Yes | `meetId: 74003` (= `live.athletic.net/meets/74003`). |
| EventID | Yes | `eventId` + full event header (`name`, `shortName`, `abbreviation`, `eventCategory`, `eventType` Individual/Relay, `classDivision`, `round`, `roundCode`). |
| Mark | Yes | `mark` metric string (`"2.02m"`, `"10.94"`), plus `imperialMark` integer (2,020,000 for 2.02 m → 1e-6 m units in the field event inspected; the field name is misleading, so verify the scale per event family). `previousBest`/`newPersonalBest` use feet-inches strings (`"6-08.00"`). |
| Normalized mark inputs | Yes | Both metric and imperial forms; `seedMark`, `previousBest`, `newPersonalBest`, `recordNotation`, `isValid` flag. |
| Timing method | **No** | No FAT/hand-time field anywhere. |
| Wind | Field exists | `wind` present on every finisher row; `null` in the field event inspected. Per-sprint legality unknown from this sample [UNVERIFIED]. |
| Implement/hurdle spec | Partial | Event `metadata._source` carries the scoring feed's raw config (e.g. `hno:"Opening Height: 1.80 for finals"`, `ano:"Top 20 Advance by Mark"`, `peg:"Field"`, `mms:"HY-DB"`); shot/discus weight not observed. |
| Heat/round | Yes | `heat`, `lane`, `roundLabel`, `heats[]` with per-heat entries, `relatedRounds[]` linking prelims↔finals. |
| Place | Yes | `place`, `points` (team scoring), `placesScored` (0 in every event inspected). |
| Date | Yes | `scheduledTs`/`scheduledDate` per event, `finalizedAt`, `metadataFetchedAt`, `resultsFetchedAt`. |
| School represented | Yes | `ihsaSchoolId`, `ihsaName`, `ihsaShortName`, `ihsaLogoUrl` (scorezone.ihsa.org logo). |
| Relay membership | **Yes** | `eventType:"Relay"` rows contain `members:[{order, athlete:{…year, athleticNetId}}]` (4 members for the 4x800 m final inspected) + `relayDivision:"A"`. |

Cross-country has **no performance results** in the IHSA API — only qualifier rosters (box, school,
name, grade, coach) and Athletic.net links for results. Sectional/regional T&F results are not
published by IHSA at all (no class-parameterized sites/schedules data; only 2 meets in the API).

### Incremental use

Weekly collector design that never re-fetches the historical corpus:

1. **Meet-level change detection (2 requests/week):**
   `GET /v1/track-field/meets` → compare `UpdatedAt` / `LastRefreshedAt` / `LastRefreshSummary` per
   `MeetId`. `GET /v1/track-field/meets/{year}/events?gender=…` → 97-event index with `status`
   (`final`), `hasResults`, `metadataFetchedAt`; only events whose `(eventId,status,metadataFetchedAt)`
   tuple changed need a summary fetch (typically 0 requests outside the May championship window).
2. **Result fetch (only changed events):** `GET /v1/track-field/events/{eventId}/summary`; store
   `updatedAt`/`finalizedAt`/`resultsFetchedAt` as the row watermark. In season, the whole T&F
   championship adds ~194 event summaries over 3 days — everything else is cached forever.
3. **Seasonal qualification snapshot (6 requests, once per season):** the six
   `/data/{trb,trg}/{1,2,3}qual.htm` pages carry a `Last updated at …` banner; fetch monthly
   (May–June for T&F, Oct–Nov for XC) and diff by content hash, then extract new `(name, grade, school)`
   rows. This is the only way to see qualifiers before finals results exist.
4. **XC qualifiers (6 requests, once per season):** `GET /v1/{term}/statefinal/cc-qualifiers?tournamentId={688..693}`
   per class/gender; a 404 `Archive not available for term …` means the season's roster is not posted
   yet, and an empty `boxAssignments` means "not published" (observed for 2024-25). No watermark
   fields, so hash the payload.
5. **School/roster drift (3 requests/week or 3/season):** `/v1/schools` (hash; 828 rows) and
   `/v2/classification/classifications?season={S,F}` (hash; catches schools that start/stop
   sponsoring T&F/XC and class moves). `staff2` refreshes should be per-school, spread (e.g. 25
   schools/day over a month) rather than a nightly sweep — see the anti-spam note below.
6. **Documents:** `/v1/{term}/sports/{slug}/tournament-results` returns `CreatedDate` + CDN `finalUrl`
   per heat-sheet/winner-report PDF — diff on `(name, CreatedDate)` to pick up new PDFs without
   re-downloading.

### Access characteristics

* **Classification:** public structured JSON (undocumented REST, `/v1` + `/v2`), plus normal HTML for
  the legacy `/data/` tree and static PDFs on the CDN.
* **Auth/anti-bot:** none observed. Cookie-free `curl` with a plain UA string returns 200 for every
  documented path (`api.ihsa.org`, `www.ihsa.org`, `/data/*.htm`). No CAPTCHA, no paywall, no
  Cloudflare challenge. `robots.txt`: `User-agent: * / Allow: /`.
* **Headers observed on `GET /v1/schools/0101`:** `HTTP/2 200`, `server: nginx/1.20.1`,
  `content-type: application/json; charset=utf-8`, `cache-control: public, max-age=180`, `expires: …`,
  `vary: Origin, Accept-Encoding`, plus a strict CSP/COOP/CORP/HSTS header set.
  **No** `RateLimit-*`, `Retry-After`, or `X-RateLimit-*` headers, and **no 429 or 403 observed** in
  the ~79 API requests issued (63 curl + ~16 XHR from the 6 page loads). Payload sizes to plan around: `/v1/schools` 451 KB,
  `/v2/classification?season=S` 1.79 MB, `?season=F` 2.27 MB, one event summary ~110–150 KB.
* **Published limits:** none found (no docs, no robots directives, no rate-limit headers). Treat as
  unstated-but-polite: sequential requests, ≥0.3 s apart, as done here.
* **One soft control:** staff email addresses are behind the `…/staff/{PersonID}/email` reveal
  request (UI hides them by default "to protect against spam"). Public and unauthenticated, but this
  is a deliberate anti-harvest measure — respect it with low-volume, role-scoped use.
* **Not available / not observable:** OpenAPI schema; a bulk staff/coach directory (only per-school);
  any historical T&F results endpoint; any regular-season meet/results endpoint; sectional results.

### Recommendation

**PRIMARY (school/sport universe + season structure) + COACH-DIRECTORY (with the caveats below) +
RESULT-SOURCE for Illinois state finals only.**

* Use `GET /v1/schools` (1 request) + `/v2/classification/classifications?season={F,S}` (2 requests)
  as the canonical Illinois `CanonicalSchool` seed: 828 members, 632 boys / 624 girls T&F schools,
  539 boys / 507 girls XC schools, with class, enrollment, coop and current entry status.
* Use the T&F state-finals feed as the highest-value Athletic.net-leverage artifact in the whole
  Midwest sweep: it supplies `athleticNetId` for every finisher **and** every relay member, plus
  grade, school and marks — eliminating both Athletic.net discovery and profile parsing for that
  cohort. Harvest it once per season (194 event summaries) and cache permanently.
* Use `staff2` + the email reveal for `HCB-TRB`/`HCG-TRG`/`HCB-CCB`/`HCG-CCG` (+ `C1/D1` AD)
  only: ~two thirds to ~90% of schools by sport carry a head-coach record in the sample, and where
  the record exists it is nearly always accompanied by a published email. Backfill the missing
  schools from district/school sites (or the shared coach-contact graph, assignment 29).
* **Expected marginal coverage:** the complete Illinois school universe with verified T&F/XC
  sponsorship; ~2,555 T&F state-final qualifiers (838 Jr) and ~1,214 measured male XC qualifiers
  (358 grade 11) per season with authoritative grade; the head coach/AD slice of `staff2`
  (≈4,000–5,000 records statewide at the sample's ~3 sport-coach rows + 2 AD rows per school
  [INFERENCE]); zero Athletic.net requests for the state-final cohort's identity resolution.
* **Do not** use IHSA for regular-season meets/results, sectional results, career PR progression,
  indoor track, or athlete profile URLs — those remain Athletic.net/MileSplit/DirectAthletics work.
* **Cost envelope:** ~10 requests per season for universe + structure, ~200 requests per T&F
  championship week, ~6 requests per XC season, plus a spread-out `staff2` refresh budget.
  This is negligible next to one ranking page per event on Athletic.net.

### Evidence appendix

Timestamps are America/Chicago on 2026-09-19 (grouped by the order the calls were issued; file mtimes
were used for the captures that were written to disk). `curl` = `curl -sS -L -A "Mozilla/5.0 … Chrome/128.0"`,
sequential with 0.25–0.5 s spacing. Browser rows are a real headless-Chromium page load (no other
agent's tooling, no athletic.net traffic) with network capture.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| https://www.ihsa.org/robots.txt | GET curl | 200 | `User-agent: * / Allow: /`; sitemap declared; no crawl restrictions | 23:11 |
| https://www.ihsa.org/sitemap.xml | GET curl | 200 (text/html) | Sitemap is an SPA fallback, not XML — no route enumeration from it | 23:11 |
| https://www.ihsa.org/ | GET curl | 200 | Site is a Vite/React SPA; entry bundle `/assets/main-B2rKN2jy.js`; `api.ihsa.org` preconnect | 23:11 |
| https://www.ihsa.org/assets/main-B2rKN2jy.js | GET curl | 200 (116 KB) | Full route table (`/schools/school-directory/:letter`, `/schools/details/:schoolId`, `/sports/:sportId/state-series/tf-results/:year/:classId/events/:eventId`, `/sports/:sportId/regular-season/*`) | 23:12 |
| https://www.ihsa.org/assets/SchoolDirectoryLayout-Cm24HvEF.js | GET curl | 200 | School detail page meta: "school details, sports, and staff directory" | 23:12 |
| https://www.ihsa.org/assets/SchoolIndexPage-C45gepzG.js | GET curl | 200 | `fetch("https://api.ihsa.org/v1/schools")` powers the whole directory ("828 member schools") | 23:12 |
| https://www.ihsa.org/assets/SchoolsPage-DTFtknRu.js | GET curl | 200 | Same `/v1/schools` payload for `/schools/school-directory/all` and `/:letter` | 23:12 |
| https://www.ihsa.org/assets/TeamResults-CSmw7_As.js | GET curl | 200 | Team result views use `/v1/proxy/${d}/teams/cumulative` | 23:12 |
| https://www.ihsa.org/assets/SportLandingPage-DjWJbm0p.js | GET curl | 200 | Sport pages use `/v1/sports/${s}/tournament-info`, `/tournament-scoreboard`, `/scoreboard` | 23:12 |
| https://www.ihsa.org/assets/SchoolDetailsPage-mRTjXW7Y.js | GET curl | 200 | Staff directory UI: `Head Coach:`, `administratorEmail`, "Show email" reveal, anti-spam privacy note | 23:13 |
| https://www.ihsa.org/assets/SchoolProfileLayout-PwFdow3U.js | GET curl | 200 | School profile sub-routes (records, participation, champions) | 23:13 |
| https://www.ihsa.org/assets/SchoolLookupPage-CZ5vQeYW.js, SchoolsPage-DTFtknRu.js | GET curl | 200 | School lookup/letter index built from the same single payload | 23:13 |
| https://www.ihsa.org/assets/MultiSportStateFinalsPage-DHYMHjc3.js | GET curl | 200 | `/v1/officials/multi-sport-state-finals?minSports=` (officials, not results) | 23:13 |
| https://www.ihsa.org/assets/SchoolDirectoryHeader-CvcT-W90.js | GET curl | 200 | **Endpoint inventory**: `/v1/schools/{id}`, `/v1/schools/{id}/staff2`, `/v1/schools/{id}/staff/{staffId}/email`, `/v1/schools/{id}/entries`, API base `https://api.ihsa.org` | 23:13 |
| https://www.ihsa.org/assets/StateSeries-w2fH3odu.js | GET curl | 200 (792 KB) | Athletic.net meet-id constants, XC tournament ids 688–693, cc-qualifiers/track summary endpoints, `/v1/proxy/data?url=`, slug→ActivityID map | 23:14 |
| https://www.ihsa.org/assets/EntriesPage-GcCwMIpC.js | GET curl | 200 | Entries page uses `/v1/schools/{id}/entries` | 23:14 |
| https://www.ihsa.org/assets/ClassificationBySportNextPage-CxdwspKS.js | GET curl | 200 | **`https://api.ihsa.org/v2/classification/classifications?season=${a}`** (bulk school×sport×class) | 23:19 |
| https://api.ihsa.org/v1 | GET curl | 404 | No API index (`{"error":"Route not found"}`) | 23:13 |
| https://api.ihsa.org/openapi.json, /docs | GET curl | 404 | No public API documentation | 23:13 |
| https://api.ihsa.org/v1/schools | GET curl | 200 (451,188 B) | **828 member-school rows**; membershipType 801/26/1; SchoolID `"0101"`–`"7097"`; 18 records missing lat/long | 23:13 |
| https://api.ihsa.org/v1/schools/0101 | GET curl | 200 | School detail schema (phone/fax/county/enrollment/`URL`/`AthleticsURL`/GoFan/NFHS/conferences) | 23:13 |
| https://api.ihsa.org/v1/schools/0101/staff2 | GET curl | 200 | Staff tree with `HCB-TRB`, `HCG-TRG`, `HCB-CCB`, `HCG-CCG`, `C1-BoysAD`, `D1-GirlsAD`, `E1-ActivDir` + `HasEmail` | 23:14 |
| https://api.ihsa.org/v1/schools/0101/staff/96256/email | GET curl | 200 | Email reveal returns `{"email":"jrakestraw@atown276.net"}` (role: Boys/Girls T&F head coach) | 23:14 |
| https://api.ihsa.org/v1/schools/0101/entries | GET curl | 200 (9,939 B) | 19 entries, all `2026-27`, incl. `TRB.2026-27.1A`, `CCB.2026-27.1A` — tournament-id pattern and per-school sponsorship | 23:14 |
| https://api.ihsa.org/v1/schools/{1432,2715,2942}/entries | GET curl | 200 | 1432 has TRB/TRG entries but no T&F coach; 2715 lists only `DWR`; 2942 has TRB/TRG entries but no T&F coach → coach-listing gaps | 23:20 |
| https://api.ihsa.org/v1/sports | GET curl | 200 | 45 activities; `boys-track-field`→TRB, `girls-track-field`→TRG, `boys-cross-country`→CCB, `girls-cross-country`→CCG; classes `1A/2A/3A`; **no indoor track activity** | 23:14 |
| https://api.ihsa.org/v1/sports/boys-track-field | GET curl | 200 | Sport config: nav items, classes, state-series terms & conditions URL | 23:16 |
| https://api.ihsa.org/v1/track-field/meets | GET curl | 200 | `count: 2` — only the 2026 boys/girls state finals exist in the API | 23:15 |
| https://api.ihsa.org/v1/track-field/meets/2026?gender=Boys | GET curl | 200 | `MeetId 74003`, `DataSource "stored"`, `LastRefreshedAt 2026-05-31`, `LastRefreshSummary` (97 events, 4 score groups) | 23:15 |
| https://api.ihsa.org/v1/track-field/meets/2026/score-groups?gender=Boys | GET curl | 200 | 4 groups 1A/2A/3A/WD with `ScoreId` composite keys | 23:15 |
| https://api.ihsa.org/v1/track-field/meets/2026/events?gender=Boys | GET curl | 200 (200 KB) | 97 events with `eventId`, round, class, status, `hasResults`, raw scoring metadata | 23:15 |
| https://api.ihsa.org/v1/track-field/meets/2025?gender=Boys | GET curl | 404 | `{"error":"Meet not found"}` — no T&F history | 23:15 |
| https://api.ihsa.org/v1/cross-country/meets/2025?gender=Boys (+/events) | GET curl | 404 | No `cross-country` meet namespace; XC is served by `/{term}/statefinal/cc-qualifiers` | 23:15 |
| https://api.ihsa.org/v1/terms | GET curl | 200 | `currentTerm 2026-27`; archives `2025-26`,`2024-25`,`2023-24` (`routes:"*"`) | 23:16 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=688 | GET curl | 200 (87,567 B) | XC 1A boys: 398 box assignments, 30 team qualifiers, 42 individual qualifiers with `yearInSchool` + coach names | 23:16 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=689 | GET curl | 200 | 2A: 396 athletes, grade-11 = 119 | 23:21 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=690 | GET curl | 200 | 3A: 420 athletes, grade-11 = 135; boys total 1,214 with 358 grade-11 | 23:21 |
| https://api.ihsa.org/v1/2024-25/statefinal/cc-qualifiers?tournamentId=688 | GET curl | 200 (empty) | Archive route exists but carries no XC qualifier rows for 2024-25 | 23:17 |
| https://api.ihsa.org/v1/2026-27/statefinal/cc-qualifiers?tournamentId=688 | GET curl | 404 | `{"error":"Archive not available for term 2026-27"}` — current-season XC roster not posted yet | 23:21 |
| https://api.ihsa.org/v1/2025-26/sports/boys-track-field/tournament-results | GET curl | 200 | 4 heat-sheet PDFs with `CreatedDate` + CDN `finalUrl` (2025-26 only) | 23:17 |
| https://api.ihsa.org/v1/2024-25/sports/boys-track-field/tournament-results | GET curl | 200 | `data: []` — no document archive for prior terms | 23:17 |
| https://api.ihsa.org/v1/track-field/events/2790204/summary | GET curl | 200 (110 KB) | HJ 1A final: 20 finishers with `place/mark/imperialMark/seedMark/previousBest/newPersonalBest/wind/lane/heat`, `athlete.athleticNetId 27740691`, `team.athleticNetId 16352`, `year "11"`, `ihsaSchoolId "0611"`; `rankings` links to Athletic.net list ids 168416 + **169070 (Illinois)** | 23:15 |
| https://api.ihsa.org/v1/track-field/events/500937/summary | GET curl | 200 (148 KB) | 4x800 m relay final: `eventType Relay`, `members[4]` each with `athlete.athleticNetId` + grade; `relayDivision "A"` | 23:17 |
| https://api.ihsa.org/v2/classification/classifications?season=S | GET curl | 200 (1,789,503 B) | Spring matrix: TRB 632 schools (267/168/197 by class), TRG 624; `EntryStatus`, enrollments, coop, success factor | 23:19 |
| https://api.ihsa.org/v2/classification/classifications?season=F | GET curl | 200 (2,271,933 B) | Fall matrix: CCB 539 (531 entered), CCG 507 (499 entered) | 23:19 |
| https://api.ihsa.org/v2/classification/classifications?season=W | GET curl | 200 (1,694,748 B) | Winter matrix present (proves the `season` code semantics) | 23:19 |
| https://api.ihsa.org/v2/classification/classifications?season=2026-27 | GET curl | 200 | `{"data":[]}` — term string is not a valid season parameter | 23:19 |
| https://api.ihsa.org/v1/sports/boys-track-field/calendar | GET curl | 404 | No calendar endpoint → no regular-season meet enumeration | 23:21 |
| https://api.ihsa.org/v1/2025-26/sports/boys-track-field/sites (+/schedules) | GET curl | 400 | `Missing required parameter: class` with `availableClasses: []` → no sectional sites/schedules data for T&F | 23:21 |
| https://api.ihsa.org/v1/schools/{0101,0229,0338,0516,0725,1205,1332,1432,1640,1916,2210,2715,2797,2942}/staff2 | GET curl ×14 | 200 each | Coach-coverage sample (482 staff rows total incl. 0101): TRB 11/14, TRG 12/14, CCB 9/14, CCG 10/14, AD 14/14; 49/49 track-XC coach rows `HasEmail:true` | 23:18 |
| https://api.ihsa.org/v1/schools/{1205,2942,0516,0229,0338,0725,1332,1640,2210,0101,2715}/ | GET curl ×11 | 200 each | School website fields: of 11 sampled `URL` values, 10 are school/district sites; Elgin 0516 points to `il.8to18.com`; `AthleticsURL` null in all 11 | 23:21 |
| https://www.ihsa.org/data/trb/1qual.htm | GET curl | 200 (56,433 B) | Static 1A boys state-final qualifiers with seed mark, name, **(grade)**, school; "Last updated … May 26, 2026" | 23:18 |
| https://www.ihsa.org/data/{trb/2qual,trb/3qual,trg/1qual,trg/2qual,trg/3qual}.htm | GET curl ×5 | 200 (51–62 KB) | All six class/gender qualifier pages; parsed totals 3,342 entries (Sr 1,295 / Jr 1,079 / So 636 / Fr 332) | 23:18 |
| https://www.ihsa.org/data/{trb,trg,ccb,ccg}/… probes (`/`, `index.htm`, `1index.htm`, `1res.htm`, `1result.htm`, `1fin.htm`, `1ent.htm`, `qual.htm`, `ccg/1qual.htm`, `ccb/1qual.htm`) | HEAD curl ×12 | 404 | No directory listing, no result/entry files; XC has no static qualifier page | 23:19 |
| https://www.ihsa.org/sports/boys-track-field/state-series/tf-results/2026 | browser load | 200 | Page calls `/v1/track-field/meets/2026?gender=Boys`, `…/score-groups`, `…/events`; renders "Live timing & scoring by", 4 class cards | 23:15 |
| https://www.ihsa.org/sports/boys-track-field/state-series/tf-results/2026/1A/events/2790204 | browser load | 200 | Calls `/v1/track-field/events/2790204/summary`; renders "Kehlin Crawford(Jr.) Flora 2.02m PB" | 23:15 |
| https://www.ihsa.org/sports/boys-cross-country/state-series/xc-state-central | browser load | 200 | Calls `/v1/2025-26/statefinal/cc-qualifiers?tournamentId=688`; page links: `Qualifiers => https://www.athletic.net/CrossCountry/meet/268054/entries`, `Results => …/268054/results` | 23:16 |
| https://www.ihsa.org/sports/boys-track-field/state-series/tf-state-central | browser load | 200 | Links `AthleticLive Results => https://live.athletic.net/meets/74003`, `Athletic.net Entries => https://www.athletic.net/TrackAndField/meet/663321/entries`; calls `/v1/proxy/data?url=…%2Fdata%2Ftrb%2F1qual.htm` and `/v1/2025-26/sports/boys-track-field/tournament-results` | 23:16 |
| https://www.ihsa.org/sports/boys-track-field/state-series/state-results, /state-series (×2) | browser load | 200 | 1A/2A/3A result tabs; page text confirms state-finals-only scope; no Athletic.net links on the results page | 23:16 |
| https://api.ihsa.org/v1/schools/0101 (header capture) | GET curl -D | 200 | `nginx/1.20.1`, `cache-control: public, max-age=180`, `vary: Origin`; **no rate-limit/Retry-After headers** | 23:21 |

**Requests issued (politeness ledger):** 41 `www.ihsa.org` requests (20 site/JS-bundle fetches,
6 qualifier-page GETs, 15 HEAD existence probes), 63 `api.ihsa.org` curl requests (dominated by the
14-school coach sample and the 11 school-detail samples), plus ~16 XHR issued by 6 headless page
loads of `www.ihsa.org` (asset bundles cached after the first load).
Sequential throughout, ≥0.25 s spacing, no retries abused, no 403/429/CAPTCHA/paywall encountered,
`robots.txt` honored, `Retry-After` never returned. No cookies, tokens or auth of any kind used.
