# 08. Wisconsin timing-provider ecosystem

Status: complete
Observed on: 2026-09-19

Scope: the Wisconsin result-host/timer ecosystem reachable from this machine on 2026-09-19, ranked by
2025–2026 meet volume, with result formats, stable IDs, URL patterns and incremental surfaces. All
`athletic.net` fetches are impossible from this host (403, see Access characteristics); Athletic.net-side
facts below come from the HAR captures and from timers' own outbound links, never from live scraping.

### Source

Wisconsin is served by a *small set of timing companies* that publish their own result files, plus three
result-hosting platforms that absorb most of their traffic:

| # | Provider / host | Own surface (verified) | Home city (source) |
|---|---|---|---|
| 1 | **PrimeTime Timing** | `pttiming.com` (+ `/api/results/current` JSON), `live.pttiming.com`, `data.pttiming.com` | Brookfield, WI (MileSplit timer 12) |
| 2 | **AccuRace Timing Services** | `accuracetiming.com` (+ `live.accuracetiming.com` = MileSplit Live) | Boscobel, WI (MileSplit 197) |
| 3 | **TrackSide Timing** | `tracksidetiming.com`, `results.tracksidetiming.com` (= AthleticLIVE tenant) | Eagle, WI (MileSplit 969) |
| 4 | **K2 Timing, LLC** | `k2timing.com`, `results.k2timing.com` (= AthleticLIVE tenant) | Milwaukee, WI (MileSplit 982) |
| 5 | **Performance Timing, LLC** | `performancetiming.com` (WordPress REST + RUNMEET PDFs), `results.performancetiming.com` (RaceTec HTML), `live.performancetiming.com` (= AthleticLIVE tenant) | Wausau, WI (MileSplit 985) |
| 6 | **OnYourMarks Timing** | `onyourmarkstiming.com`, `live.onyourmarkstiming.com` (= AthleticLIVE tenant) | Winneconne, WI (MileSplit 1133) |
| 7 | **WIAA** (association, not a timer) | `wiaawi.org` state-archive pages → Hy-Tek / RunScore / RUNMEET files | Madison, WI |
| 8 | **MileSplit WI** (aggregator) | `wi.milesplit.com/timing/**` timer directory + schedules | — |
| 9 | **RaceTec** (platform) | `racetec.net`, `results.performancetiming.com/results.aspx` | — |
| 10 | **AthleticLIVE / anet.live** (platform) | white-label tenants `results.<timer>.com` / `live.<timer>.com`; `performance.anet.live/<short>` short links redirect here | Athletic.net infra |

The directory that made this enumerable is MileSplit's own timer index:
`https://wi.milesplit.com/timing` lists **28 Wisconsin timing-company entries** with IDs and home cities
(ids: 12, 111, 190, 197, 406, 599, 695, 940, 969, 982, 985, 986, 1013, 1048, 1116, 1133, 1192, 1208, 1216,
1269, 1304, 1312, 1667, 1796, 1830, 1831, 1993, plus duplicate "Sub4 Timing" 1830/1831).

**Lead corrections (verification, not assumption):**
- "PT Timing" = **PrimeTime Timing**. `https://www.pttiming.com/` returns the PrimeTime Timing home page
  (title "PrimeTime Timing", `meta-description` "PrimeTime Timing is a Wisconsin based sports technology
  company…"), and it advertises "WIAA State Championships LaCrosse, WI" as a key event.
- "MTEC" is **Minnesota**, not Wisconsin. `https://mn.milesplit.com/timing` lists
  `mtec-results-championchip` as timer id 44 on the *Minnesota* site; no MTEC credit appears in any
  Wisconsin postseason file scanned here (see Evidence appendix). MTEC is out of this slice — assign to
  the Minnesota timers agent.
- "Hy-Tek exports" are real and central: WIAA state/sectional/regional results are Hy-Tek Meet Manager
  exports (below), and K2 Timing self-describes as an "ACTIVE/HY-TEK Certified Contractor".

### Coverage

**Provider ranking** — ordered by evidenced 2025 Wisconsin meet volume (counts **not additive**: each timer is
counted on its own surface, and HS/MS/college/road events are mixed unless stated):

| Rank | Provider | 2025 meets | 2026 meets | Count source (evidence) | Result format(s) | Grade + school for HS athletes | Enumerable by date |
|---|---|---|---|---|---|---|---|
| 1 | **PrimeTime Timing** | **319** WI (246 HS-like) | **255** WI (199 HS-like) | own API `pttiming.com/api/results/current?year=` paged to `hasMore:false`; 266/200 of those rows carry result files | Hy-Tek HTM/PDF, RunScore XC PDF, RUNMEET PDF, live JSON app | **yes** (grade column in every PDF; school in all files) | yes — `year` + newest-first paging |
| 2 | **AccuRace Timing Services** | **305** | **312** | MileSplit timer 197 schedule (`wi.milesplit.com/timing/197/accurace-timing-services?year=`) | own site: meet-info PDFs only; results on MileSplit Live (`live.accuracetiming.com/meets/<id>`) | not verifiable on own site (results not hosted there) | yes — per-year schedule |
| 3 | **TrackSide Timing** | **187** | **6** | MileSplit timer 969 schedule; own site links AthleticLIVE + Athletic.net team 72797 | AthleticLIVE live; Hy-Tek at WIAA sectionals (`neenahsectionalb.htm`, 2025 Badger regional 2026) | yes in WIAA Hy-Tek files | partially — MileSplit schedule + AthleticLIVE (browser) |
| 4 | **Tortoise and Hare Race Management** | **59** | 0 | MileSplit timer 940 schedule (Minocqua, WI) | not inspected (out of HS-first scope; youth/indoor mix) | unverified | yes — per-year schedule |
| 5 | **K2 Timing, LLC** | **36** | **0** | MileSplit timer 982 schedule; own site links AthleticLIVE + Athletic.net team 74899 | AthleticLIVE live; Hy-Tek at WIAA sectionals (`westallissectionalb.htm`, 2025) | yes in WIAA Hy-Tek files | partially — MileSplit schedule + AthleticLIVE (browser) |
| 6 | **In Focus Timing by Movin' Shoes** | **18** | **8** | MileSplit timer 695 schedule (Madison) | not inspected — `infocustiming.com` returns **403** to non-browser clients | unverified | yes — per-year schedule (site itself blocked) |
| 7 | **Performance Timing, LLC** | **10** | **0** | MileSplit timer 985 schedule; **WP archive 1,259 posts** (271+194 HS XC, 158 T&F) spanning 2019–2026 | XC PDF (grade+school) + RaceTec HTML + AthleticLIVE live | **yes in XC PDFs**; no in RaceTec HTML | yes — WP REST `after=`/`search=`; RaceTec `CId=16541` |
| 8 | **OnYourMarks Timing** | **0** | **0** | MileSplit timer 1133 schedule empty for 2025 — **contradicted** by the 2025 WIAA D1 De Pere XC sectional file credited `Timing: OnYourMarks Timing` | own `/events/` + `/results/` pages; AthleticLIVE live | yes in WIAA sectional files | only via its own site (schedule pages, no year param verified) |
| 9 | Long tail (2025 MileSplit counts) | 1269 MHS Track (Marathon) 18, 1312 Fast Lane Timing (Spencer) 10, 1116 3B Timing (Milwaukee) 9, 1048 Watertown In House Timing 9, 1192 Complete Timing (Glidden) 8, 1993 Hawkeye Timing (Osseo) 7, 1304 Norski Timing (DeForest) 3, 1667 Neillsville Timing 2, 1796 BD Timing (Beaver Dam) 2, 190 Wisconsin Runner Timing (Racine) 1, 1013 XM Timing (Manitowoc) 1, and **0** for 111 White River Sports, 406 Absolute Race Timing, 599 SMA Results, 986 Athletic Director's Edge, 1133 OnYourMarks, 1146 Track Mac, 1208 (NSL) Performance Timing, 1216 Hodag Nordic, 1830 Sub4 Timing | 0 except 695 = 8 | MileSplit timer index (28 entries incl. the 1830/1831 Sub4 duplicate) + per-timer `?year=2025` fetches | | | per-year schedule |
| — | **WIAA** (association, not a timer) | **130** postseason files | **103** postseason files (8 state) | `wiaawi.org/sports/boys-track-field/boys-track-field-state-archive` link list | Hy-Tek HTM, Crystal PDF, RunScore PDF, RUNMEET PDF, Cocoa-converted HTM | **yes** | static per-year archive page |

- **Duplicate-entity trap:** MileSplit carries two entries for Performance Timing — `985` "Performance Timing,
  LLC" (Wausau, 10 meets in 2025) and `1208` "(NSL) Performance Timing" (Wausau, 0 in 2025) — and two for
  Sub4 Timing (`1830`, `1831`, both Madison). Reconciliation by city+name, not by MileSplit id, or counts
  double-count/drop entities.
- **Geography:** Wisconsin only (this slice). Timer platforms (AthleticLIVE, RaceTec, MileSplit) are
  multi-state, so the mechanisms generalise, but every count below is Wisconsin rows only.
- **Sports:** boys/girls cross country, indoor track & field, outdoor track & field. Middle-school and
  collegiate meets are interleaved in the same feeds — see the caveat on every count.
- **Levels:** high school is the majority for PrimeTime/Performance Timing/WIAA; AccuRace's MileSplit
  schedule is college-heavy; several timers carry substantial MS volume.
- **Historical depth (verified):**
  - PrimeTime archive API: `yearOptions 2017…2026` (site config), queried 2025 and 2026 in full.
  - MileSplit timer schedules: per-timer, e.g. TrackSide's own site deep-links
    `wi.milesplit.com/timing/969/trackside-timing?year=20{20,22,23,24,25}&season=cc` and posts its 2026 XC
    schedule on `athletic.net/team/72797/cross-country/2026` — i.e. the timer's own history is split across
    both aggregators.
  - WIAA state archive: result files back to 2000 (`/Portals/0/PDF/Results/Track/2000/…`); the 2025–2026
    region of the archive alone holds **233 postseason result files — 130 for 2025 and 103 for 2026**
    (of which 8 are 2026 state results: boys/girls × D1/D2/D3 + wheelchair).
  - Performance Timing: WordPress archive holds 1,259 posts (`x-wp-total: 1259`).

### Enumeration

Every surface below was used, not inferred:

1. **PrimeTime — public JSON API (best single surface).**
   `GET https://www.pttiming.com/api/results/current?year=YYYY&page=N`
   → `{"page":N,"limit":100,"hasMore":bool,"rows":[…]}`; newest-first; `year` accepts at least 2017–2026.
   Discovered from the `/results` page's embedded config (`apiPath":"/api/results/current"`, `pageSize:100`).
   Each row carries `name, startDate, exFranklinMid, exTimer, exEntryLink, exHostOr, exCityOr, exStateOr,
   eventUrl, results.liveResultsHtml, results.fileLinks[]`.
   All Wisconsin counts reported in this file use two documented rules, because no timer marks school level or
   sport explicitly: **WI row** = `exStateOr` starts with `WI`; **HS-like** = WI row whose name does not match
   `middle school|MS|youth|grade school|elementary|college|NCAA|half-marathon|marathon|5k|10k|dash|fun run|alumni|open|community|parish|club`;
   **sport** = `XC` when `liveResultsHtml` contains `/xc-ptt.html`, `TF` when it contains `live.pttiming.com/?mid=`,
   else `other` (road races, or meets with no PrimeTime live page). Measured result: 2025 WI 319 rows / HS-like
   246 (XC 70, TF 154, other 22); 2026 WI 255 / HS-like 199 (XC 28, TF 144, other 27).
2. **Performance Timing — WordPress REST + static PDFs.**
   `GET https://www.performancetiming.com/wp-json/wp/v2/posts?per_page=&page=&categories=&after=`
   → JSON with `x-wp-total: 1259`; `?search=<term>` works (used to enumerate all `WIAA … Sectional` posts:
   20 returned, 2019→2026). `GET /wp-json/wp/v2/categories` → 12 categories with counts
   (Results 1247, School Event Results 544, Community Event Results 378, HS Cross Country 271 + 194,
   Track and Field 158). Posts use WordPress date slugs (`wiaa-d1-sectional-stevens-point-2025`). Each
   post body exposes the meet's artifacts directly:
   - RaceTec HTML: `results.performancetiming.com/results.aspx?CId=16541&RId=<n>`
   - results PDFs: `/Results/<year>CC/FinalResults_{Boys,Girls}<Meet>.pdf` (XC) and
     `/HS_Track/PDF_results/<year>/…` (track, incl. RUNMEET-printed files)
   - a live link: `http://performance.anet.live/<short>` → 301 → `https://live.performancetiming.com/meets/<LiveID>`
3. **RaceTec (behind Performance Timing).**
   `GET https://results.performancetiming.com/StartPage.aspx?CId=16541&From=1&To=20` → all races for that
   client, paginated in 20s (page links observed up to `From=801&To=820`), with a "RECENT RESULTS" block;
   each race links `results.aspx?CId=16541&RId=<n>`.
4. **MileSplit timer directory + schedules.**
   `GET https://wi.milesplit.com/timing` → 28 timers; `GET /timing/<id>/<slug>?year=YYYY&season=` → a
   server-rendered schedule table (one `<tr>` per meet, each linking `/meets/<id>-<slug>`). The numeric
   JSON claim is *false*: `/api/v1/timers/969`, `/api/v1/timers`, `/api/v1/timers/969/schedule?year=2025`
   all return HTTP 200 JSON with `"data":null` — a router stub, not a data API.
5. **WIAA state archives.** `https://www.wiaawi.org/sports/boys-track-field/boys-track-field-state-archive`
   (and `…/boys-cross-country/boys-cross-country-state-archive`) are static link lists of result files per
   year, e.g. `/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm`,
   `/sites/default/files/2026-08/trb2026d1stateresults.pdf`.
6. **AthleticLIVE tenants.** `https://results.tracksidetiming.com/meet-list`,
   `https://results.k2timing.com/meet-list`, `https://live.onyourmarkstiming.com/meet-list` all return the
   same 50,184-byte Angular shell (title "AthleticLIVE", assets from `livestatic.athletic.net`, tenant map
   containing 70 `*.anet.live` origins). No JSON endpoint was found; enumeration requires a browser.
7. **Not enumerable (blocked):** `www.athletic.net` — HTTP **403** from this machine for both curl and the
   harness's native fetcher (observed twice). Athletic.net-side enumeration must come from HAR/repo.
8. **Partial block:** `www.infocustiming.com` → 403 to a non-browser client (In Focus Timing, Madison;
   MileSplit timer 695, 18 meets in 2025).

### Stable identifiers

| Identifier | Surface | Verified example |
|---|---|---|
| Athletic.net division list id | HAR `GetRankings` request/response | `divListId 170770` = Wisconsin (`ParentID 168416`, `BaseDivID 638`, `Website http://www.wiaawi.org`) |
| Athletic.net MeetID | HAR ranking rows; PrimeTime `exEntryLink` | `667524` = "WIAA State Track & Field Championships" (HAR meet map `EndDate` `2026-06-06`); `277106` = 2026 Smiley Invitational XC |
| Athletic.net TeamID (timers) | timer sites | `72797` TrackSide Timing, `74899` K2 Timing |
| Athletic.net ranking list id | K2 site link | `175082` ("WISCONSIN HONOR ROLL", `/TrackAndField/rankings/list/175082/m`) |
| MileSplit timer id | MileSplit | `12` PrimeTime, `969` TrackSide, `982` K2, `197` AccuRace, `985` Performance, `1133` OnYourMarks, `44` MTEC (MN) |
| MileSplit meet id | timer schedules / live results | `780853`, `773350` (AccuRace live URL), `643519` |
| PrimeTime event id | `eventUrl` | UUID `0af5092e-182c-4918-bd2f-a21fe91e58c7` (Smiley 2026) *or* legacy integer `2910` (New London Bulldog 2026) |
| PrimeTime live meet id | `exFranklinMid` ↔ live URL | `9130` → `live.pttiming.com/xc-ptt.html?mid=9130` |
| PrimeTime result file URL | `results.fileLinks[].url` | `data.pttiming.com/storage/v1/object/public/event-files/<eventUuid>/<epochms>-<slug>.pdf`; track: `…/event-files/crm/<eventId>/<md5>.pdf` |
| RaceTec client/race/participant | `results.aspx` | `CId=16541`, `RId=920`, participant `uid=16541-920-3-266648` |
| WIAA result file path | wiaawi.org | year-partitioned static path, e.g. `…/Results/Track/2025/…`, `…/2026-08/trb2026d1stateresults.pdf` |
| Bib | RaceTec tables, PrimeTime live feed | RaceTec prints the bib inside the name cell (`Cooper Erickson#951`, `Pos` column = place) |
| Athlete id | PrimeTime live Firebase (`WAAthletes` node keys, e.g. `236314`) | **Not usable — see Access characteristics** |

Naming is *not* an identifier anywhere in this ecosystem: meet titles are inconsistent
("Smiley Invitational" vs "Junior Smiley Invitational", "Chuck Walek Invitational" vs `crm/2723`), and the
only cross-vendor join keys are the Athletic.net/MileSplit meet links that PrimeTime publishes in
`exEntryLink`.

### Athletic.net leverage

- **Direct Athletic.net meet links, machine-readable:** PrimeTime's `exEntryLink` is an anchor whose href is
  an Athletic.net or MileSplit meet page. In 2026 this switched almost entirely to Athletic.net:
  **2026 WI rows: athletic.net 195, milesplit 0, none 60**; **2025: milesplit 163, athletic.net 76, none 80.**
  So for the 2025–26 season PrimeTime delivers `Athletic.net MeetID → timer result file` pairs directly.
- **Timer-owned Athletic.net team pages:** TrackSide links `athletic.net/team/72797/{track-and-field-outdoor,cross-country}/<year>`
  and K2 links `athletic.net/team/74899/track-and-field-indoor/2026|2027` as their published meet hubs —
  those TeamIDs are a deterministic seed for Athletic.net meet/result lookups.
- **WIAA file name ↔ Athletic.net MeetID join is exact for at least one case and near-exact in general:**
  the 2026 WIAA file `tr2026appletonregionalindiv.pdf` carries PDF title `RUNMEET: D1 Regional 8B - Appleton North`,
  and the Wisconsin HAR ranking payload contains `MeetID 667458` with `MeetName "D1 Regional 8B - Appleton North"`.
  The HAR's Wisconsin postseason MeetIDs run `667433` ("D1 Regional 1A - Menomonie") … `667524` ("WIAA State
  Track & Field Championships"), 43 meets in the `6674xx` block; HAR2's athlete-bio meet map supplies their
  `EndDate`s (e.g. state = `2026-06-06`).
- **Timers whose *live* results already sit on Athletic.net infrastructure:** TrackSide, K2, OnYourMarks,
  Performance Timing (all four are AthleticLIVE white-label tenants; `performance.anet.live/<short>` 301s to
  `live.performancetiming.com/meets/<LiveID>`, whose shell loads `livestatic.athletic.net`). For those,
  "avoid Athletic.net" is impossible for live data — but the *static* season archive still exists on the
  timer's own domain/WIAA (PDFs, Hy-Tek). Only PrimeTime (own Karmarush app) and AccuRace (MileSplit Live)
  publish live results off Athletic.net.
- **Estimated Athletic.net request avoidance (meet-level, junior-year 2025-26 window):**
  - PrimeTime alone exposes high-school-like WI meets inside the junior-year window: **69 XC (Aug–Nov 2025
    HS-like rows) + 137 TF (Mar–Jun 2026 HS-like rows) ≈ 206**, each with grade + school + mark already parsed
    from the timer's PDF/HTML (see Athlete evidence). Indoor (Dec 2025–Feb 2026) adds 10 measured HS-like rows
    (2026-01/02 HS-like = 9, 2025-12 = 1), e.g. Titan Challenge, Caged Eagle Invite, Stout Quad. Counted with
    the rule documented under Enumeration.
  - WIAA 2026 postseason: **103 result files** for regionals/sectionals plus 8 state result files (including
    wheelchair divisions) — all carrying the same meet names as the Athletic.net MeetIDs, so the postseason
    is discoverable without Athletic.net at all.
  - Performance Timing adds ~271+194 HS XC + 158 T&F archive posts (multi-year), RaceTec adds per-race
    `results.aspx` pages.
  - Per-meet Athletic.net cost is not measured here (agent 3 owns `GetMeetData`); at the repo's observed
    scale (95 query families → 4,256 receipts for the whole corpus), avoiding ~300 Wisconsin meet fetches
    per season is a **≥300-request reduction [INFERENCE]** and, more importantly, removes the need to
    enumerate those meets on Athletic.net at all.

### Athlete evidence

| Provider | name | grade/class | school | city/state | gender | TF/XC | indoor/outdoor | marks | PR/progression | profile URL |
|---|---|---|---|---|---|---|---|---|---|---|
| PrimeTime Hy-Tek (TF) | yes | **yes** (`Yr` 9–12) | yes | meet-level | by division | TF | from meet metadata | yes + wind, round, place, points | no | no |
| PrimeTime RunScore (XC) | yes | **yes** (`FR/SO/JR/SR`) | yes | meet-level | by race | XC | — | time, team score/pack splits | no | no |
| WIAA Hy-Tek HTML (2025) | yes | **yes** (`Year`) | yes | meet-level | by division | TF | outdoor | yes + wind + heat | no | no |
| WIAA Hy-Tek Crystal PDF (2026) | yes | **yes** (`Yr`) | yes | meet-level | by division | TF | outdoor | yes + wind + points | no | no |
| Performance Timing (PDF, XC) | yes | **yes** (`Grade` 9–12) | **yes** (`School`) | meet-level | by race | XC | — | time, place, team position, points, pace | no | no |
| Performance Timing (RaceTec HTML) | yes | **no** | **no** (name column only) | meet-level | by race | both | meet-level | time, `Age Group`, `Team Points`; bib in name cell | no | no |
| AccuRace own site | — (meet-info PDFs only) | no | no | yes | no | both | — | — | — | no |
| MileSplit/AthleticLIVE live pages | yes (agent 7/2 territory) | depends on page | yes | yes | yes | both | yes | yes | yes (MileSplit athlete pages) | yes |

Concretely, the WIAA 2026 D1 boys state PDF contains lines like
`  1 Trey Resch             11   Arrowhead            1       10.74 Q              -0.5` and
`  5 Kingston Penn          11   Middleton            3       10.90 q              -1.2` — i.e. **name +
grade + school + mark + heat + wind + round (Q/q) + place** for exactly the Class-of-2027 cohort, from a
public WIAA file, with no Athletic.net request. PrimeTime XC PDFs print `Alex Wagner SO 17:40.0`
(New London Bulldog Invite boys varsity, line 9) inside team-score blocks; PrimeTime track PDFs print relay
legs with grade (`1) Preston, Annelyse 11`) alongside athletes (`1 Dahl, Jaylie 12 ARCADIA 13.16 Q`).
Performance Timing's **XC PDFs are the cleanest third-party shape found in this slice** — a 2025 WIAA D1
sectional prints `Pos No Name School Qualifier Grade Time TeamPosition Points Pace`, e.g.
`   2     991   Eli Akey                Wausau West               Ind      11    00:16:03.6       1        2      5:10 min/m`
and `   1     951   Cooper Erickson         Stevens Point             Team     12    00:15:42.8       1        1      5:04 min/m`
— bib `No`, name, school, **grade**, and a state-qualification flag (`Team`/`Ind`).

Caveats on extracting these files (all observed, not inferred):
- The 2025 state result HTML published by WIAA is a **Cocoa HTML Writer** re-export
  (`<meta name="Generator" content="Cocoa HTML Writer">`): every space run is an `Apple-converted-space`
  span, so whitespace-normalising extraction is required (grade/school data is intact:
  `4 Ryan Heiman              11 Arrowhead                10.59q  0.4  1`).
- Performance Timing's **2026 track PDFs are not text-extractable at all** — `RUNMEET_ D1 Sectional 1 -
  D.C.pdf` (656,768 B) and `Results_ D1 Regional 1B - D.C.pdf` (1,268,696 B) both decode to mojibake
  (subset fonts without ToUnicode). The WIAA-hosted copy of the same sectional
  (`tr2026dceverestsectional.pdf`, 634,674 B) is equally garbled, so this is a producer-side print pipeline
  problem, not a WIAA re-render. OCR (or the RaceTec HTML, which has times but no school/grade) is the only
  path for those meets.

### Recruiting information

**None of the timer/result sources publish coach or athletic-director information.** No coach, AD, email or
school-athletics-website field appears in any of: PrimeTime API rows, Hy-Tek/PDF result tables, RaceTec
`results.aspx` tables, WIAA result files, or the MileSplit timer pages. Timer contact pages publish only the
company's own contact address (e.g. `k2timing@gmail.com` on k2timing.com; `legal@karmarush.com` in the
PrimeTime live-data terms) — that is a vendor contact, not a school coach contact, and must not be used as
one. Coach/AD acquisition stays with the WIAA/school-directory agents (assignments 6 and 29).

### Result evidence

| Field | PrimeTime Hy-Tek (TF) | PrimeTime RunScore (XC) | WIAA files | Performance Timing PDF (XC) | RaceTec HTML | PrimeTime live (Firebase) |
|---|---|---|---|---|---|---|
| ResultID | no | no | no | no | no (only `uid` participant id) | yes (node keys) |
| AthleteID | no | no | no | no (bib `No` only) | bib `#n`, `uid` | yes (`WAAthletes`) — **terms-restricted** |
| MeetID | timer event id + Athletic.net/MileSplit meet link | same | file path only | WP post slug / PDF path | `CId`+`RId` | `mid`/`exFranklinMid` |
| EventID | event name only | race name only | event name only | race name only | race name only | event id |
| mark | yes (FAT-normalized, ties printed to 3dp) | yes (0.1s) | yes | yes (0.1 s) | yes | yes |
| normalized inputs | no | no | no | no | no | — |
| timing method | `FAT`/`F` round flags in rankings; PDFs use Hy-Tek conventions | RunScore chip/photo-finish per vendor | same | chip (vendor) | chip (vendor claim) | — |
| wind | yes (prelims/finals columns) | n/a | yes | n/a (XC) | no | — |
| implement/hurdle spec | not in the files observed | n/a | not observed | n/a | no | — |
| heat/round | yes (`H#`, Prelims/Finals, Q/q) | race categories (Varsity/JV/MS, D1/D2/D3) | yes | race categories | age group / category | yes |
| place | yes | yes | yes | yes | yes | yes |
| date | yes (meet header) | yes | yes | yes | yes | yes |
| school represented | yes | yes | yes | **yes** (`School` column) | **no** — name column only | yes |
| qualification flag | no | no | no | **yes** (`Qualifier` = `Team`/`Ind`, i.e. state entry) | no | — |
| relay membership | yes (leg names + grades) | n/a | no | n/a | n/a | — |

### Incremental use

- **PrimeTime (recommended primary feed):** poll `GET /api/results/current?year=<current>&page=1` — rows are
  newest-first (`2026-09-19` first row; page 1's last row `2026-05-19`), 100/page, `hasMore` for paging.
  Diff on `eventUrl`/`exFranklinMid`; fetch `results.fileLinks[]` only for new events. Full-year re-pull is
  4–5 requests, so even a naive weekly full re-pull is cheap; incremental diff semantics are trivial.
- **Performance Timing:** `GET /wp-json/wp/v2/posts?after=<ISO8601>&per_page=50` (or `modified_after=`),
  then read each post's `content.rendered` for the PDF/RaceTec link. No historical re-fetch needed.
- **RaceTec:** `StartPage.aspx?CId=<id>` — newest races appear first in "RECENT RESULTS"; `From/To` paging
  walks history only when needed.
- **MileSplit timer pages:** `?year=YYYY` is a full re-pull per timer-year; cheap (≈1 request per timer) but
  it is a *registration* schedule, not a results-completeness index (see caveat below).
- **WIAA:** no feed — re-read the two state-archive pages per sport (4 requests) and diff the file lists;
  the archive page is static HTML and stable.
- **AthleticLIVE tenants (TrackSide, K2, OnYourMarks, Performance Timing):** no public JSON found; weekly
  refresh needs a browser session, so prefer the timer's static archive for these four where one exists
  (TrackSide/K2/OnYourMarks publish none of their own; Performance Timing publishes PDFs + RaceTec).

### Access characteristics

| Host | Class | Observed detail |
|---|---|---|
| `www.pttiming.com`, `/api/results/current` | **public structured JSON** (undocumented) | HTTP 200 `application/json`, no auth, no rate-limit headers; no terms page linked from the site root (checked 2026-09-19) |
| `data.pttiming.com` | **static PDF downloads** | Supabase-style public object storage; PDFs fetched with no auth |
| `ptt-franklin.firebaseio.com` | **terms-restricted (do not automate)** | Publicly readable Firebase RTDB (2,727 meet nodes) but the DB's own `Meta.Terms.notice` states verbatim: *"Automated access, bulk extraction, republication, and any use of this data to train, fine-tune, evaluate, or ground AI or machine-learning systems are prohibited. That this endpoint answers without authentication is not permission to use it."* Terms URL `https://live.pttiming.com/terms.html`, version `2026-09-03`, contact `legal@karmarush.com`. Only a shallow key probe was performed before this notice was found; collection stopped there. |
| `www.performancetiming.com/json/wp-json/**` | **public structured JSON** (WordPress default REST) | 200 JSON, `x-wp-total` headers |
| `performancetiming.com/Results/**`, `/HS_Track/PDF_results/**` | **static PDF downloads** | 200 `application/pdf`, no auth; `http://` → `https://` 301 |
| `results.performancetiming.com` (RaceTec) | **normal HTML** (ASP.NET) | 200, `X-AspNet-Version` present, no auth |
| `live.performancetiming.com` (= `performance.anet.live`) | **browser application** (AthleticLIVE tenant) | `performance.anet.live/<short>` 301 → `live.performancetiming.com/meets/74631`; shell references `livestatic.athletic.net` 19× |
| `wi.milesplit.com` | **normal HTML** | 200; `/api/**` exists but returns `data:null` |
| `live.accuracetiming.com` | **normal HTML** (MileSplit Live Results) | `meets/773350` → 200; `meets/773541` → connection timeout after 30 s (host flaky) |
| `results.tracksidetiming.com`, `results.k2timing.com`, `live.onyourmarkstiming.com` | **browser application** | Angular SPA shell; no JSON API found |
| `wiaawi.org` | **normal HTML + static PDF/HTM** | 200 behind Cloudflare; archive pages are static link lists |
| `www.athletic.net` | **blocked to non-browser clients from this machine** | 403 (Cloudflare) for curl and native fetch, 2026-09-19 |
| `www.infocustiming.com` | **unavailable to non-browser clients** | 403 |
| No host | — | **no `Retry-After` or 429 was observed anywhere in this slice**; no published rate limits found |

### Recommendation

- **RESULT-SOURCE — PRIMARY for Wisconsin: PrimeTime Timing.** One JSON API covers 2017–2026, delivers WI
  meet metadata plus direct links to result PDFs (266 WI files in 2025, 200 in 2026 YTD) that carry
  **name + grade + school + mark (+wind/round/place)**, and exposes the Athletic.net/MileSplit meet id for
  most 2026 events. This is the single highest-value adapter in the state.
- **RESULT-SOURCE (secondary): Performance Timing, LLC** — WordPress REST (`?after=` / `?search=`) enumerates
  HS XC/T&F meets back years with stable `CId/RId` IDs, and its **XC PDFs are the best-shaped third-party
  files in this slice** (`No`, name, school, grade, qualifier, time, points, pace — name+grade+school for the
  whole field). Two confirmed limits: RaceTec HTML has no school/grade (bib only), and **2026 track PDFs are
  not text-extractable** (producer-side font issue, both sectional and regional variants tested), so track
  use requires OCR or WIAA's Hy-Tek/Crystal files where they exist.
- **VALIDATION: WIAA state-archive files** — official, grade-bearing Hy-Tek/Crystal PDFs and HTML for state,
  sectional and regional rounds; use to verify grades/schools and to join to Athletic.net MeetIDs by name
  without touching Athletic.net.
- **ATHLETIC.NET-SEED: PrimeTime `exEntryLink` + timer team pages.** 195 of 255 Wisconsin 2026 PrimeTime
  events already carry an Athletic.net meet URL; TrackSide (72797) and K2 (74899) publish Athletic.net
  team pages as meet hubs.
- **REJECT: `ptt-franklin.firebaseio.com` live feed** (and any other Karmarush live endpoint). It would be
  the richest structured source (athlete ids, event ids, live marks) but its own terms prohibit automated
  access, bulk extraction and ML grounding. Use `live.pttiming.com` pages only as human-facing links.
- **COACH-DIRECTORY: not applicable** — no coach/AD data in this slice.
- **Marginal coverage:** timers + WIAA cover essentially every Wisconsin high-school meet that has published
  results, including the entire postseason, with grade evidence that Athletic.net rankings only expose
  indirectly. What they do *not* give: stable athlete ids that join across meets (no provider here has one),
  PR/progression history, coach contacts, or a machine-readable index for the four AthleticLIVE tenants.

**Caveats that must survive into the synthesis**
1. Wisconsin timer counts are **not additive** — PrimeTime's own archive (319 WI rows in 2025) overlaps its
   MileSplit schedule (201), and all MileSplit timer schedules mix HS, MS, collegiate and road events
   (AccuRace's 305 rows are visibly college-heavy).
2. MileSplit timer schedules are a **registration** view: PrimeTime's MileSplit page holds 201 meets for 2025
   but only 17 for 2026 while its own archive shows 255 — the schedule shrank because registration moved to
   Athletic.net, not because results disappeared. The extreme case: OnYourMarks Timing's MileSplit page lists
   **0 meets for 2025**, yet the official 2025 WIAA D1 De Pere XC sectional file is credited
   `Timing: OnYourMarks Timing`. Never use MileSplit timer counts as a completeness measure.
3. Result files are presentation artifacts: no ResultID/AthleteID exists in any PDF or Hy-Tek HTML export,
   and the one source that *does* have athlete ids is the terms-restricted live feed.
4. **Text extractability is per-producer, not per-site.** Verified within this slice: PrimeTime PDFs (Hy-Tek
   Crystal, RunScore, RUNMEET) extract cleanly; Performance Timing PDFs (`RUNMEET_ …dc.pdf`,
   `Results_ …dc.pdf`) and the WIAA copies of the same meets decode to mojibake and need OCR; WIAA's 2025
   state HTML extracts but only after normalising `Apple-converted-space` runs. Budget an OCR fallback for a
   minority of 2026 track files, and prefer the 2025–2026 Hy-Tek/Crystal variants where both exist.

### Evidence appendix

| URL | method | HTTP | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.athletic.net/` | curl (browser UA) + native fetch | 403 / 403 | Athletic.net is unreachable to non-browser clients from this machine | 2026-09-19 23:13 CDT |
| `www.athletic.net.har` (local) `GetRankings` responses | file parse (`json` + regex) | 200 (captured) | Wisconsin division `170770` (parent `168416` "High School", `BaseDivID 638`); **701 ranking rows → 311 distinct WI meets** (`(MeetID, MeetName)`), MeetID range 620489–670169, all rows `"State":"WI"` | 2026-09-19 23:40 |
| `www.athletic.net2.har` (local) `GetAthleteBioData` | file parse | 200 (captured) | Meet map with `EndDate`: `667524` "WIAA State Track & Field Championships" → `2026-06-06`; `667509` "D1 Sectional 2 - Verona Area" → `2026-05-29`; `667436` "D1 Regional 2A - Sauk Prairie" → `2026-05-26` | 2026-09-19 23:41 |
| `https://www.pttiming.com/results` | GET | 200 | Site is PrimeTime Timing; embedded config `apiPath:/api/results/current`, `pageSize:100`, `yearOptions 2017…2026` | 2026-09-19 23:15 |
| `https://www.pttiming.com/api/results/current?year=2026&page=1..4` | GET | 200 | 352 events, newest-first, `hasMore:false` at page 4; 255 WI rows | 2026-09-19 23:15 |
| `https://www.pttiming.com/api/results/current?year=2025&page=1..5` | GET | 200 | 452 events, 319 WI rows, 266 with result files | 2026-09-19 23:15 |
| `https://www.pttiming.com/` | GET | 200 | "PT Timing" = PrimeTime Timing; declares "WIAA State Championships" a key event | 2026-09-19 23:15 |
| `https://live.pttiming.com/xc-ptt.html?mid=9130` | GET | 200 | Live page mounts `#franklinApp`; `fbURL="https://ptt-franklin.firebaseio.com/"` | 2026-09-19 23:16 |
| `https://ptt-franklin.firebaseio.com/.json?shallow=true` | GET | 200 | 2,737 keys: 2,727 numeric meet nodes + non-numeric `WAAthletes`, `Athletes`, `XC`, `Scoreboard`, `GraphicsClockRoad`, `selfieboard`, `isolynx`, `armory`, `_terms`, `undefined` | 2026-09-19 23:16 |
| `https://ptt-franklin.firebaseio.com/9130/Meta.json` | GET | 200 | Verbatim terms notice prohibiting automated access / bulk extraction / AI grounding; `version: 2026-09-03`; contact `legal@karmarush.com`; Terms-of-Use URL referenced as `https://live.pttiming.com/terms.html` (found in `live.pttiming.com` page JS) | 2026-09-19 23:19 |
| `https://ptt-franklin-default-rtdb.firebaseio.com/.json?shallow=true` | GET | 404 | New-domain Firebase form does not serve this DB | 2026-09-19 23:16 |
| `https://data.pttiming.com/…/1789835129158-boys-varsity.pdf` (New London XC 2026) | GET | 200 | RunScore XC report with `Alex Wagner SO 17:40.0` → name + **grade** + school + time | 2026-09-19 23:15 |
| `https://data.pttiming.com/…/crm/2723/…pdf` (Chuck Walek Invite 2026) | GET | 200 | Track report: `Dahl, Jaylie 12 ARCADIA 13.16 Q`; relay legs carry grades; PDF title `RUNMEET: Chuck Walek Invitational` | 2026-09-19 23:16 |
| `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm` | GET | 200 | `Licensed to PrimeTime Timing - Contractor License` + `HY-TEK's Meet Manager 6/19/2025` | 2026-09-19 23:15 |
| `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/{hudson,verona,sunprairie,westdepere,marinette}sectional.htm` | GET | 200 | PrimeTime Timing credit (5 sectionals) | 2026-09-19 23:22 |
| `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/neenahsectionalb.htm` | GET | 200 | `Licensed to TrackSide Timing - Contractor License` | 2026-09-19 23:22 |
| `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/westallissectionalb.htm` | GET | 200 | `Licensed to K2 Timing, LLC - Contractor License` | 2026-09-19 23:22 |
| `https://www.wiaawi.org/sites/default/files/2026-08/trb2026d1stateresults.pdf` | GET | 200 | PDF `/Title = 'Crystal Reports ActiveX Designer - tfmm6results2col.rpt'`; text has `Yr` grade + wind columns; no `Licensed to` line | 2026-09-19 23:14 |
| `https://www.wiaawi.org/sites/default/files/pdfs/PDF/Results/Cross_Country/2025/d1bstateresults.pdf` | GET | 200 | `Timing & Results by PrimeTime Race & Event Management, LLC`; `file:///C:/RunScore/RSTMP$$$.HTML` | 2026-09-19 23:17 |
| `…/Cross_Country/2025/{barrowhead,depere,bmanitowoclincolnsectional}.pdf` | GET | 200 | PrimeTime (Arrowhead, Manitowoc Lincoln) and **OnYourMarks Timing** (De Pere, `Timing: OnYourMarks Timing`) XC sectionals; also the evidence that MileSplit's 0-meet schedule for timer 1133 is incomplete | 2026-09-19 23:36 |
| `https://www.wiaawi.org/sports/boys-track-field/boys-track-field-state-archive` | GET | 200 | 1,661 result links overall; 233 postseason files for 2025–2026 (130 in 2025, 103 in 2026, 8 of them state results); 2026 files named `tr2026[<site>]{regional,sectional}[indiv|team].pdf` | 2026-09-19 23:22 |
| `https://www.wiaawi.org/sites/default/files/2026-08/tr2026appletonregionalindiv.pdf` | GET | 200 | PDF title `RUNMEET: D1 Regional 8B - Appleton North` — matches HAR `MeetID 667458` name exactly | 2026-09-19 23:25 |
| `https://www.wiaawi.org/sites/default/files/2026-08/tr2026dceverestsectional.pdf` | GET | 200 | Subset-font PDF: `pdftotext` returns mojibake → needs OCR | 2026-09-19 23:26 |
| `https://www.wiaawi.org/sites/default/files/2026-08/tr2026badgerregional.pdf` | GET | 200 | Producer `Powered By Crystal`; text `TrackSide Timing - Contractor License  Hy-Tek's MEET MANAGER 8:24 PM 5/26/2026` | 2026-09-19 23:22 |
| `https://www.wiaawi.org/scorecenter` | GET | 200 | Association score-reporting iframe → `schools.wiaawi.org/ScoreCenter/Results/WIAAGameResults` (not a timer) | 2026-09-19 23:19 |
| `https://wi.milesplit.com/timing` | GET | 200 | 28 Wisconsin timing companies with IDs + home cities | 2026-09-19 23:23 |
| `https://wi.milesplit.com/timing/12/primetime-timing?year=2025|2026` | GET | 200 | 201 rows (2025) vs 17 rows (2026) — registration migration, not results loss | 2026-09-19 23:18 |
| `https://wi.milesplit.com/timing/{969,197,982,940,695,1269,985,1312,1116,1048,1192,1993,1304,1796,1667,190,1013}?year=2025` | GET | 200 | 2025 meet counts: 187, 305, 36, 59, 18, 18, 10, 10, 9, 9, 8, 7, 3, 2, 2, 1, 1 | 2026-09-19 23:20 |
| `https://wi.milesplit.com/timing/{197,969,940,982,695}?year=2026` | GET | 200 | 2026 rows: 312, 6, 0, 0, 8 | 2026-09-19 23:21 |
| `https://wi.milesplit.com/api/v1/timers{,/969,/969/schedule?year=2025}` | GET | 200 | `{"data":null}` — MileSplit has no timer JSON API | 2026-09-19 23:18 |
| `https://mn.milesplit.com/timing` | GET | 200 | `mtec-results-championchip` id 44 is a **Minnesota** timer (MTEC lead corrected) | 2026-09-19 23:20 |
| `https://www.tracksidetiming.com/` | GET | 200 | Links `results.tracksidetiming.com/meet-list` ("Athletic.Live Meets"), `athletic.net/team/72797/...` (2026 XC schedule), and MileSplit archives `?year=2020,2022–2025&season=cc`; also `wi.milesplit.com/timing/969` | 2026-09-19 23:45 |
| `https://www.k2timing.com/` + `/meet-entries` | GET | 200 | "ACTIVE/HY-TEK Certified Contractor"; `results.k2timing.com/meet-list`; `athletic.net/team/74899/...`; `athletic.net/TrackAndField/rankings/list/175082/m` | 2026-09-19 23:19 |
| `https://onyourmarkstiming.com/` + `live.onyourmarkstiming.com/meet-list` | GET | 200 | Own `/events/`, `/results/`; live tenant identified as AthleticLIVE (`livestatic.athletic.net`, 70 `*.anet.live` origins) | 2026-09-19 23:23 |
| `https://www.accuracetiming.com/` | GET | 200 | 27 current-season meet-info PDFs (`files/<abbr>_26.pdf`, Google Docs renderer) + 28 `live.accuracetiming.com/meets/<id>` links | 2026-09-19 23:24 |
| `https://accuracetiming.com/files/darcc_26.pdf` | GET | 200 | It is the **invite info packet**, not results (`Title: 2026 Cross Country Invite`) — AccuRace publishes results on MileSplit Live | 2026-09-19 23:25 |
| `https://live.accuracetiming.com/meets/773350` | GET | 200 | Title `MileSplit Live Results` — AccuRace live host is MileSplit | 2026-09-19 23:23 |
| `https://www.performancetiming.com/wp-json/wp/v2/posts?per_page=1` | GET | 200 | `x-wp-total: 1259`, `x-wp-totalpages: 1259` — full WordPress REST surface | 2026-09-19 23:23 |
| `https://www.performancetiming.com/wp-json/wp/v2/categories?per_page=100` | GET | 200 | 12 categories: Results 1247, School 544, Community 378, HS XC 271+194, T&F 158, Nordic 57 | 2026-09-19 23:23 |
| `https://www.performancetiming.com/wp-json/wp/v2/posts?categories=18&per_page=3` | GET | 200 | HS posts carry `Results/YYYY/*.pdf` + RaceTec links; includes 2025 WIAA D1 Sec. Stevens Point, D2 Sec. Merrill | 2026-09-19 23:26 |
| `https://results.performancetiming.com/results.aspx?CId=16541&RId=920` | GET | 200 | RaceTec platform (`racetec.net`, `rtv3.css`); sub-pages Results/Stats/Teams; participant `uid=16541-920-3-266648`; table cols `Pos,Name,Time,Age Group,Team Points,Gender` | 2026-09-19 23:25 |
| `https://results.performancetiming.com/StartPage.aspx?CId=16541` | GET | 200 | "All Races" index, paginated `From/To` to ≥820 races, "RECENT RESULTS" block | 2026-09-19 23:26 |
| `https://racetec.net` | GET | 200 | "RaceTec - Race Timing and Results Software" (platform vendor site) | 2026-09-19 23:26 |
| `https://results.tracksidetiming.com/meet-list`, `https://results.k2timing.com/meet-list` | GET | 200 | Identical 50,184-byte AthleticLIVE Angular shell — no JSON API exposed | 2026-09-19 23:18 |
| `https://wi.milesplit.com/results` | native fetch | 200 | WI results index lists ~29 Sept-2026 XC meets with MileSplit meet ids (cross-check surface) | 2026-09-19 23:13 |
| `https://www.infocustiming.com/` | curl | 403 | In Focus Timing (MileSplit 695) refuses non-browser clients | 2026-09-19 23:22 |
| `https://www.tracksidetiming.com/`, `https://www.pttiming.com/`, `https://www.performancetiming.com/`, `https://www.accuracetiming.com/` | GET | 200 | All 2xx with a plain `Mozilla/5.0 (compatible; source-research/1.0)` UA; sequential ≤50 requests/host, no 429s seen | 2026-09-19 23:13–23:26 |
| `https://www.performancetiming.com/wp-json/wp/v2/posts?search=sectional&per_page=20` | GET | 200 | 20 `WIAA … Sectional` posts 2019→2026, incl. `wiaa-d1-sectional-stevens-point-2025` (2025-10-25) and 2026-05-28 D1 DC Everest / D3 Marathon — Performance Timing times WIAA postseason track **and** XC | 2026-09-19 23:28 |
| `…/wp-json/wp/v2/posts?search=DC%20Everest&per_page=2` | GET | 200 | Post links `RUNMEET_ D1 Sectional 1 - D.C.pdf` + `http://performance.anet.live/gy2i7c` | 2026-09-19 23:29 |
| `https://performancetiming.com/HS_Track/PDF_results/2026/RUNMEET_%20D1%20Sectional%201%20-%20D.C.pdf` | GET (followed 301) | 200 | 656,768 B; `pdftotext` → mojibake (subset fonts, no ToUnicode) — same failure as WIAA's copy of the meet | 2026-09-19 23:29 |
| `https://performancetiming.com/HS_Track/PDF_results/2026/Results_%20D1%20Regional%201B%20-%20D.C.pdf` | GET | 200 | 1,268,696 B; also mojibake → not a single-file fluke, the whole 2026 track print pipeline is non-extractable | 2026-09-19 23:30 |
| `https://performancetiming.com/Results/2025CC/FinalResults_BoysSpashSec.pdf` | GET | 200 | **Extractable** XC PDF: `Pos No Name School Qualifier Grade Time Team Position Points Pace` with `No`=bib, grade 9–12, `Team`/`Ind` state-qualifier flag | 2026-09-19 23:30 |
| `https://results.performancetiming.com/results.aspx?CId=16541&RId=872` | GET | 200 | HS sectional (Stevens Point 2025) RaceTec page: `Pos, Share, Name#bib, Time, Age Group, Team Points` — **no school, no grade**; confirms RaceTec HTML is the weaker artifact for the same meet | 2026-09-19 23:30 |
| `http://performance.anet.live/gy2i7c` | GET (followed 301) | 301 → 200 | `live.performancetiming.com/meets/74631`; shell contains `livestatic.athletic.net` 19× → Performance Timing is the 4th Wisconsin AthleticLIVE tenant | 2026-09-19 23:29 |
| `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm` | GET | 200 | `<meta name="Generator" content="Cocoa HTML Writer">` + `Apple-converted-space` spans; grade/school intact (`4 Ryan Heiman  11 Arrowhead  10.59q  0.4  1`) | 2026-09-19 23:32 |

Helper scripts written for this slice (read-only, cache to `/tmp`):
`tools/08-fetch.py` (single-URL fetch/link/anchor/grep), `tools/08-ptt-enumerate.py` (PrimeTime archive API),
`tools/08-ms-timers.py` (MileSplit timer index + per-timer season counts).
