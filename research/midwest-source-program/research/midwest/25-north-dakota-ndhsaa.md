# 25. North Dakota NDHSAA (North Dakota High School Activities Association)

Status: complete
Observed on: 2026-09-19
Session window: 2026-09-19 23:09-23:33 CDT. All fetches sequential with 0.7-1 s sleeps. Approximate request
counts (from this session's command log; earlier ND probes in the same run are included where the saved artifact
identifies them): `ndhsaa.com` ~45-50 (at the ~50/host politeness ceiling - no further NDHSAA fetches were made
after the 22-school sample), `ndhsaanow.com` ~15, `live.herostiming.com` ~10, `search.athletic.live` 12 (POST),
`s-gke-usc1-nssi3-33.firebaseio.com` ~12 (GET), `livestatic/alivestatic.athletic.net` ~14, `docs.google.com` ~6,
managed-Chromium page loads ~8. No 429, no `Retry-After`, no CAPTCHA, no auth prompt, no WAF block was observed
on any ND host. No tool returned a usage/quota/rate error during this session.

**Headline:** North Dakota's entire state-series result surface is fetchable with **plain HTTP JSON, no browser and
zero Athletic.net requests**. NDHSAA exposes a 169-school member directory whose per-school pages carry **per-sport
coach names for cross country and track & field** plus AD/Activities-Director names; NDHSAA/NDHSAANow tournament
pages publish the state meet as an **AthleticLIVE** link; and AthleticLIVE serves both the meet index
(`search.athletic.live/<tenant>_meet_list/_search`) and full athlete-level results (`<tenant> Firebase RTDB`
`liveRunStandings/<rui>`), with **Athletic.net meet ids and per-athlete `ani` ids embedded in the payloads**.

---
### Source
| Surface | Host / path | Role |
|---|---|---|
| Association site | `https://ndhsaa.com/` (apex 200; `www.` 301 -> apex) | Member schools, sport pages, regulations, board/coop pages, published sheets |
| Association "Now" site | `https://ndhsaanow.com/` | Team/schedule/tournament/champion surfaces; state-meet result links |
| Sport pages | `/athletics/track-boys`, `/athletics/track-girls`, `/athletics/crosscountry-boys`, `/athletics/crosscountry-girls` | Regulations, qualifying standards, tournament links |
| School directory | `/schools`, `/schools/<id>/<slug>` | 169 schools; AD/Activities Director; **per-sport coach table** |
| Published data (Google Sheets, official) | linked from `/board/coops`, `/calendar/approved`, `ndhsaanow.com/champions/...`, `standards_*` | Coop sponsorships, multi-year approved calendar, champions, Class A/B qualifying standards |
| Timing provider | `https://herostiming.com/` -> `https://live.herostiming.com/` (AthleticLIVE tenant `heros`) | Meet results (live + final), meet index |
| Platform APIs discovered | `POST https://search.athletic.live/heros_meet_list/_search`; `GET https://s-gke-usc1-nssi3-33.firebaseio.com/<path>.json?ns=trackmeet-io` | Meet enumeration + athlete-level results |
| Athletic.net | `www.athletic.net` (403 to non-browser from this machine 2026-09-19) | Only referenced via ids/URLs embedded in AthleticLIVE payloads - **not fetched** |

Platform notes observed while resolving the result path (relevant to every state that uses AthleticLIVE):
- The AthleticLIVE SPA shell is 50,184 B of static HTML for every meet (verified for meet 55421 with plain and
  browser-like headers); meet data is **never** in the HTML - it arrives over a **Firebase Realtime Database
  WebSocket** (`wss://s-gke-usc1-nssi3-33.firebaseio.com/.ws?v=5&...&ns=trackmeet-io`, target `trackmeet-io`)
  and, for the meet list, over an HTTP Elasticsearch service.
- The shell embeds the tenant list (hundreds of `livestatic.athletic.net/assets/sites/<tenant>/` logos and
  `*.anet.live` domains), i.e. the platform is shared by many timers; the footer reads
  "© 2026 RunnerSpace.com" and "Version 3.5.6" (AthleticLIVE is a RunnerSpace/Athletic product).
- Site skin `alivestatic.athletic.net/site-skins/112/config.json` names **"NDHSAA"** with NDHSAA brand colours
  (`#312f5c`/`#b98e3f`) - the NDHSAA-branded AthleticLIVE skin.
- `livestatic.athletic.net/assets/sites/heros/config.json` exposes `elasticSearchIndex: "heros"`,
  `esReportIndex: "heros_reports"`, `siteUrl: https://live.herostiming.com`; the shared base-site config exposes
  `primaryElasticSearchIndex: live_results`, `primaryBaseUrl: https://live.athletic.net`.

### Coverage
Counts below are exact, from the fetches in the appendix (not estimates).

**Schools - 169 member high schools.** `https://ndhsaa.com/schools` contains 169 unique
`/schools/<id>/<slug>` anchors; `https://ndhsaanow.com/schools` contains the same 169 (cross-checked). ID range
3-1378 (ids 1, 2, 25, 71, 74, 91, 93, 104, 119, 126, 127, 145, 155, 171, 173-660 etc. are absent/retired).
Full id list (comma-separated, 169):
```
3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,
45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63,64,65,66,67,68,69,70,72,73,75,76,77,78,79,80,81,82,83,84,85,
86,87,88,89,90,92,94,95,96,97,98,99,100,101,102,103,105,106,107,108,109,110,111,112,113,114,115,116,117,118,120,121,122,
123,124,125,128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,143,144,146,147,148,149,150,151,152,153,154,156,
157,158,159,160,161,162,163,164,165,166,167,168,169,170,172,661,1045,1129,1130,1131,1147,1148,1281,1307,1325,1378
```
Sample slugs: `/schools/7/bismarck`, `/schools/100/` (Maple Valley), `/schools/1045/west-fargo-sheyenne`,
`/schools/1378/mandan-classical-academy`.

**Track/XC participation (ndhsaanow team pages, 4 fetches):**
| Sport page | Teams |
|---|---|
| `/teams/crosscountry-boys` | 100 |
| `/teams/crosscountry-girls` | 96 |
| `/teams/track-boys` | 113 |
| `/teams/track-girls` | 114 |
Team ids are per-sport and per-gender (see Stable identifiers). Fewer XC teams than member schools because of
co-ops and non-sponsoring schools.

**Meets in the state-series / provider chain.** The Hero's Timing AthleticLIVE tenant index contains
**1,117 meet documents** (`POST /heros_meet_list/_search`, `match_all`, `total.value=1117`). ND-filtered counts
observed (each is one query): 175 docs match `ls:ND`; ND 2026 XC season (Aug 15-Nov 30 2026) = **6** meets indexed;
ND spring 2026 (Mar 15-Jun 15) = **29**; ND May 1-Jun 15 2025 outdoor = **9**; ND state-meet-named hits
(`match n:"North Dakota State"`) = 65 docs (mostly indoor/XC across seasons).

### Enumeration
**A. School universe (169).** One GET: `https://ndhsaa.com/schools`; parse anchors
`href="/schools/(\d+)/([a-z0-9-]+)"` (dedupe by id) - 169 results. Identical set confirmed on
`https://ndhsaanow.com/schools`. No pagination, no auth, no JS needed (server-rendered HTML).

**B. Per-school record (AD + coaches + school metadata).** One GET per school:
`https://ndhsaa.com/schools/<id>/<slug>`. Server-rendered; contains address/mailing address, phone, fax, website,
grades, 2025 enrollment, superintendent, principal(s), **Athletic Director**, **Activities Director**,
assistant ADs, business manager, staff, school song/colors/mascot, and a **"Sport/Activity Offering | Coaches"
table** with one row per sport, coop annotation inline in the sport cell
(e.g. `"Boys' Cross Country (Coop: Williams County)" -> "Kasandra Feiring"`).

**C. Team universe (ndhsaanow).** Four GETs (`/teams/crosscountry-boys`, `/teams/crosscountry-girls`,
`/teams/track-boys`, `/teams/track-girls`). Team links carry the id in the href and the school name in the anchor
`title`: `href="/teams/crosscountry-boys/469" ... title="Bismarck Century"`. Example pairs:
XC boys 469 Bismarck Century / 441 Bismarck High / 7377 Bismarck Legacy / 4820 Fargo Davies;
XC girls 473 Bismarck Century / 445 Bismarck High / 7375 Bismarck Legacy / 4821 Fargo Davies;
TF boys 4833 Fargo Davies / 2557 Fargo North / 272 Grand Forks Central;
TF girls 4834 Fargo Davies / 2563 Fargo North / 278 Grand Forks Central.

**D. Meet universe (AthleticLIVE Elasticsearch).** One POST per query:
`POST https://search.athletic.live/heros_meet_list/_search` with an ES DSL body. Verified working from curl
(no browser, no auth, no cookie), e.g.
```json
{"size":8,"track_total_hits":true,
 "query":{"bool":{"filter":[{"match":{"ls":"ND"}},
                            {"range":{"md":{"gte":"2026-08-15","lte":"2026-11-30"}}}]}},
 "sort":[{"md":"asc"}],"_source":["i","ani","n","md","ls","o"]}
```
returns the ND 2026 XC meets, each with AL meet id `i` **and Athletic.net meet id `ani`**:
`76088/277450 Orriginals Invite (2026-08-22)`, `76109/277021 Breckenridge-Wahpeton Memorial (09-17)`,
`76112/276762 Blue and White XC Invitational (09-19)`, `76117/272168 Border Battle (09-26)`,
`76139/278116 EDC XC Championship (10-10)`, `76140/278115 ND Class B East XC Championships (10-10)`.
The three query bodies the SPA itself sends (captured via CDP on the tenant home page) are "today"
(`term md == today`, size 500), "upcoming" (`range md`, sort asc, size 5) and "past" (`range md`, sort desc,
size 5) - all against `heros_meet_list`, i.e. the same index is directly queryable by date range.
Document fields include `i`, `ani`, `anis[{u,ani,n}]`, `n`/`ln` (name), `lo`/`ls`/`lsa` (venue/city-state/state),
`sd`/`ed`/`md` (dates), `o` (`xc|indoor|outdoor`), `e` (tenant), `pb` (public), `ua` (updated), `abs` (event list),
`dvs` (divisions), `gls` (genders) - 92 fields on a sample doc. Note: `heros_reports` and `live_results` both
return **404** at `search.athletic.live` (only `<tenant>_meet_list` is served).

**E. Result universe (Firebase RTDB).** One GET per (meet, event, round):
`https://s-gke-usc1-nssi3-33.firebaseio.com/<path>.json?ns=trackmeet-io`. `?ns=trackmeet-io` is **required** -
without it the same URL returns `401 {"error":"Permission denied"}` (verified). Paths observed (all 200, curl):
| Path | Content |
|---|---|
| `meet_list/<AL meet id>` | Meet metadata incl. `ani` (Athletic.net meet id) + `anis[].u` (Athletic.net URL), dates, attribution |
| `meet_<id>` (shallow) | Key list: `liveRunStandings`, `event_summary`, `event_detail`, `liveRunEventList`, `liveTeam`, `liveBySplit`, `runTime`, `rawLiveRunEvent`, `latest_update_list`, `liveRunTeamStandings`, `meet_score_report`, `splitGrids`, `byFlightStandings`, `byHeatResults`, `byRoundResults`, `liveFieldResults`, `session_detail`, ... |
| `meet_<id>/event_summary` | Per-event summary keyed `Individual-<eventId>` (`cgn` = class/event name, `d` = distance `5000`, `rui` = round-unit index `4-1`, `resd` = result time) |
| `meet_<id>/event_detail/Individual-<eventId>` | Event detail (`frvi`/`frvs`/`frvt` = finish-result versions) |
| `meet_<id>/liveRunStandings/<rui>` | **Full athlete-level results** (234,753 B for the ND state XC 2025 Class A Boys race) |
| `meet_<id>/liveRunEventList/<rui>` | Entry list with `h["<rui>-<heat>"].ani` = comma-separated athlete ids ("4-1-1" for Class A Boys) |
| `meet_<id>/runTime/<eventId>`, `meet_<id>/rawLiveRunEvent/*` | Live-timing scratch (small/empty after the meet) |

### Stable identifiers
| Identifier | Example | Source | Notes |
|---|---|---|---|
| NDHSAA school id | `7` (`/schools/7/bismarck`) | ndhsaa.com | Stable across seasons; page slug is cosmetic |
| Athlete.net school cross-ref | - | - | NDHSAA does not publish an Athletic.net team id |
| ndhsaanow team id | XC boys `469`, XC girls `473` (Bismarck Century) | ndhsaanow.com | **Per sport and per gender**; no school-level mapping published |
| AL meet id | `55421` (state XC 2025), `42224` (state indoor Class A 2025), `53090` (NDHSAA East Region 2025), `73476` (state track 2026 per NDHSAA page) | ES/RTDB/NDHSAA links | `i` field; primary join key for results |
| AL event id | `2144806` (Class A Boys 5k 2025), `2144807` (B), `2144808` (A girls), `2144809` (B girls) | RTDB `event_summary` | Numeric, stable within meet; RTDB key is `Individual-<id>` |
| AL round/unit key | `rui` = `4-1` (Class A Boys finals) | RTDB | Selects the standings node |
| **Athletic.net meet id** | `261403` (XC 2025), `571583` (TF indoor 2025 Class A), `593135` (East Region 2025), `622458/622459/622457` (indoor 2026), `277450` … (XC 2026) | `ani` in ES/RTDB | Comes with the ready-made URL, e.g. `https://www.athletic.net/CrossCountry/meet/261403`, `https://www.athletic.net/TrackAndField/meet/571583` |
| Athlete id in results | `ani` 8-digit (`20010099`), `anli` 7-digit (`42660317`) | RTDB `liveRunStandings` | `ani` semantics at athlete level are **[INFERENCE]**: AL uses `ani` for Athletic.net entity ids at meet level (verified by URL) and the value range is consistent, but no Athletic.net-side confirmation was possible (anet is 403 here) |
| Competitor number | `cm`/`i` = `1543`, displayed as `#1543` | RTDB / DOM | Meet-local, not stable across meets |
| Co-op identity | `"Boys' Cross Country (Coop: Williams County)"` | NDHSAA school page + published coop sheet | Coops are the real team identity for many small schools |

### Athletic.net leverage
1. **Meet-level mapping is free and complete for every indexed meet.** `ani`/`anis[].u` gives the Athletic.net meet
   URL without touching Athletic.net: state XC 2025 -> `https://www.athletic.net/CrossCountry/meet/261403`; state
   indoor Class A 2025 -> `https://www.athletic.net/TrackAndField/meet/571583`; state XC 2024 -> `250230`;
   indoor 2026 Class A/B/JV -> `622458`/`622459`/`622457`.
2. **NDHSAA itself publishes the AthleticLIVE state-meet link**, so the official-chain path is
   `ndhsaanow.com/tournaments/track-{boys,girls}` -> `live.athletic.net/meets/<id>`: 2026 -> `meets/73476/events`,
   2025 -> `meets/53808`; 2024 and older -> official PDFs on
   `d2q0tptsfejku7.cloudfront.net/uploads/files/...` (`Complete Meet Results PDF`, `A/B Team Results`,
   `A/B Individual Results`). NDHSAA pages never link `www.athletic.net` directly.
3. **Athlete-level Athletic.net linkage exists in the result payloads** (`ani` per standings entry and the
   comma-separated `ani` list per heat in `liveRunEventList`) - this is the cheapest known way to attach MIDWEST
   Class-of-2027 candidates to Athletic.net identities without an Athletic.net query. Verify the athlete-level
   `ani` interpretation once against a known Athletic.net profile before production use.
4. **What is not available:** no Athletic.net team/roster ids, no NDHSAA-published athlete→profile URLs, and no
   XC/track roster pages on ndhsaanow.com (team pages render without rosters).

### Athlete evidence
ND state XC 2025 (AL meet 55421, Oct 24-25 2025, Jamestown - Parkhurst Campground), `liveRunStandings/4-1`
(Class A Boys) fields observed per athlete: `fn`/`l`/`n` (first/last/full name), `tn` (team name),
`y` (**year in school**: `SR`/`JR`/`SO`/`FR`), `g` (gender), `p` (place), `rtm` (result time, `18:23.114`),
`m`/`sc`/`sk` (mark/score), `sp` (per-split objects with `cs`/`sp` times and split place), `ani`, `anli`,
`cm` (competitor number), `lg` (a Google-hosted image URL used by the UI). Verbatim example entry:
`{"ani":20010099,"anli":42660317,"fn":"Jaydyn","l":"Velek","n":"Jaydyn Velek","tn":"Jamestown","g":"M","y":"SR","p":98,"rtm":"18:23.114","m":"18:23.2"}`.
Rendered page (same meet/event) shows the same data with places and grade:
`1 Owen Hintz Bismarck 15:27.8 +1pts • #1133 • Yr: SR`, `2 Luka Rout Davies 15:52.2 • Yr: SO`,
`3 Hudsen Bullinger Century 16:05.6 • Yr: SO`, `4 Ethan Erickson Shanley 16:11.8 • Yr: JR`.
Evidence that this is a **complete** field: the Class A Boys event summary reports `nr: 185` entries
(the computed standings node weighs 234,753 B).
Grade semantics for the Class-of-2027 target: in the 2025-26 season a Class-of-2027 athlete is grade 11 (`JR`);
`y` therefore gives an independent grade source for the current season, but for 2025 XC it labels the previous
class year. Cross-season identity relies on `ani`/`anli` (stable across meets) rather than name+school alone.
No athlete email/phone/home address appears anywhere in the observed payloads (only school/team names and the
AthleticLIVE-hosted photo URL).

### Recruiting information
**Verdict: NDHSAA school pages are a usable, official coach-identity source; they are not a contact-database
source (no emails anywhere).**

*Structure* (verified by parsing the "Sport/Activity Offering | Coaches" table on 22 sampled school pages, 1 in 8
of the 169): 22/22 pages carry the table; the sport cell carries the coop annotation, the second cell carries
one or more coach names.
| Row | Row present | Non-empty coach name |
|---|---|---|
| Boys' Cross Country | 21/22 | **17/22 (77%)** |
| Girls' Cross Country | 21/22 | **16/22 (73%)** |
| Boys' Track and Field | 21/22 | **19/22 (86%)** |
| Girls' Track and Field | 21/22 | **19/22 (86%)** |
| at least one of the four | 22/22 | **20/22 (91%)** |
Examples parsed verbatim: Kindred - Josh Roberts (B XC) / Joshua Allmaras (B TF); Dickinson Trinity - Tim Baustian
(B XC) / Jake Daniel, Tim Baustian (B TF); Beach - Mike Zier (B TF), XC cells empty; Ray - Kasandra Feiring
(XC, coop Williams County) / Caitlin Shavi (Gunderson) (TF, coop); Midway - Molly Pengilly (TF, coop
Midway/Minto), XC cells empty. Blank cells are real (small schools, unfilled tables), so per-coach coverage must
be computed per sport, not assumed.

*AD / Activities Director:* always present on the school page, e.g. Bismarck `Dave Zittleman` (AD & Activities),
`Matthew Crane` (AD); West Fargo Sheyenne `Logan Midthun`, `James Moe`, assistant `Corissa Kolesar`;
Beach `Taryn Sveet`; Maple Valley `Nathan Hoots`. Contact fields are the school's general phone/fax plus the
district website - **no coach/AD email or mobile in any of the 22 sampled pages** (0 pages with `mailto:` or any
email-like string). Those are exactly the fields the collection contract allows (public professional role
contacts), and they are safe to collect; private athlete contact data does not appear in this source at all.

*Additional official coach lists:* the sport advisory-committee pages (e.g.
`https://ndhsaa.com/advisory/crosscountry-boys`) list four named coach representatives per gender with terms -
2026-09-19 page: Josh Roberts (Kindred HS, B-East 2023-2027), Richard Dafoe (Grand Forks Red River HS, A-East
2024-2028), Tim Baustian (Dickinson Trinity HS, B-West 2025-2029), Chase Gregory (Williston HS, A-West
2026-2030). These are officers, not a directory, but they are high-confidence XC coach names.

*Co-op resolution is mandatory for XC/TF recruiting.* The official co-op sheet (published Google Sheet linked from
`https://ndhsaa.com/board/coops`) has 885 rows; activity codes include `XC` (91 rows) and `TR` (107 rows);
status column: 873 `Current`, 9 `26-27`, 3 `27-28` (i.e. co-op changes are pre-announced up to two seasons out).
The sheet's school-name column is noisy in historical rows (`"Alexander Trenton '24"`,
`"Barnes Co. North '24 Victory Christian '25"`): 480 distinct name tokens across current rows - it needs the
shared alias map before joining to the 169-school directory.

### Result evidence
All rows below were fetched and parsed in this session (status 200). `i` = AthleticLIVE meet id, `ani` =
Athletic.net meet id.
| Meet | Date | i | ani | Result payload |
|---|---|---|---|---|
| ND State XC Meet (A/B, boys+girls, 4 races) | 2025-10-24/25, Jamestown ND | **55421** | **261403** | `meet_55421/liveRunStandings/{1-1,2-1,3-1,4-1}`; Class A Boys = 4-1 (234,753 B, 185 entries); event ids 2144806 (A B), 2144807 (B B), 2144808 (B G), 2144809 (A G) |
| ND State XC Meet | 2024-10-25/26, Jamestown ND | 40863 | 250230 | indexed (ES); payload not fetched |
| ND State Class A indoor | 2026-03-27, Fargo ND | 61484 | 622458 | indexed; per-event RTDB available |
| ND State Class B indoor | 2026-03-28, Fargo ND | 61486 | 622459 | indexed |
| ND State JV indoor | 2026-03-26, Fargo ND | 61482 | 622457 | indexed |
| ND State Class A / Class B / JV indoor | 2025-03-28/29, Fargo ND | 42224 / 42225 / 42226 | 571583 / 571586 / 571584 | `meet_list/42224` fetched: full `meet_42224` node = 2,935,116 B (whole meet, all sports events) |
| NDHSAA EDC Championship | 2026-05-15, Fargo ND | 73149 | 653946 | indexed |
| NDHSAA SE Region | 2026-05-16, Kindred ND | 73150 | 641408 | indexed |
| NDHSAA East Region Championship | 2025-05-16, Fargo ND | **53090** | 593135 | indexed; event payload for Boys 100m (event 1975637, `ua 2025-05-16T20:04:44Z`) fetched from the AL data service |
| NDHSAA NE Region Meet | 2025-05-14, Grafton ND | 47044 | 593126 | indexed |
| ND state outdoor track | 2026-05, Bismarck ND | **73476** | not in Hero's index | NDHSAA page links `http://live.athletic.net/meets/73476/events`; 2025 -> `https://live.athletic.net/meets/53808`; 2024 and earlier -> official PDFs on the NDHSAA CloudFront CDN |
| Regular-season ND (2026 XC) | Aug 22 - Oct 10 2026 | 76088, 76109, 76112, 76117, 76139, 76140 | 277450, 277021, 276762, 272168, 278116, 278115 | indexed; 1 RTDB GET per event once raced |
| Regular-season ND (spring 2026) | Mar 19 - Apr 13 2026 (first 10 of 29) | 61471, 61472, 61475, 61482, 61483, 61484, 61486, 63521, 61494, 61517 | 622460, 620042, 623989, 622457, 620044, 622458, 622459, 639509, 622461, 622462 | indexed |

**Gaps and caveats.**
* The **state outdoor track meet is not in the Hero's tenant Elasticsearch index** (0 docs for
  `filter: match ls:ND + term o:outdoor + match n:"State"`; the six `match n:NDHSAA` docs contain only regions and
  one 2022 XC region). Either it is timed under a different AthleticLIVE tenant or it is not an AL meet at all -
  the `live.athletic.net/meets/73476` link from NDHSAA proves the AL platform hosts it, but which tenant/index
  serves its docs was not established. **Unproven: the state outdoor track result payload path for 2026.**
* Historical state-track results 2024 and earlier are PDFs (`d2q0tptsfejku7.cloudfront.net/uploads/files/...`);
  they are official but not machine-readable without PDF parsing.
* The 2026 state XC meet (Oct 23-24, 2026) appears on the Hero's year page ("NDHSAA State Championship
  Oct 23-24 - Jamestown ND") but is **not yet in the ES index** (0 docs for `md` between 2026-10-15 and
  2026-11-05), i.e. the index lags publication of future meets.
* Regular-season ND meet coverage in AL is partial: 6 XC meets indexed for the whole 2026 season vs 91 coop XC
  entries, so many small-school duals/invites are not on the platform (or are hosted by another timer).

### Incremental use
**Minimal production loop (no browser, no Athletic.net):**
1. `POST https://search.athletic.live/heros_meet_list/_search` with
   `{"size":500,"query":{"bool":{"filter":[{"match":{"ls":"ND"}},{"range":{"md":{"gte":"<last run>","lte":"<today>"}}}]}},"sort":[{"md":"asc"}],"track_total_hits":true}`
   -> new/changed ND meets with `i`, `ani`, `n`, `md`, `ls`, `o`. (Paging via `from`/`size`, or by date window.)
2. For each meet: `GET meet_list/<i>.json?ns=trackmeet-io` -> Athletic.net meet id + URL, attribution, dates.
3. For each event/round: `GET meet_<i>/liveRunStandings/<rui>.json?ns=trackmeet-io` (enumerate `rui` from
   `event_summary`, keyed `Individual-<eventId>`) -> athlete rows with name, team, year in school, place, time,
   splits and `ani`/`anli`.
4. Per school: 1 GET of the NDHSAA school page for AD + track/XC coach names (refresh once per season; the page
   carries a "2025" enrollment stamp so year-over-year drift is detectable).
**Cost profile:** ND state XC = 4 events (4 GETs) + 1 meet GET + 1 ES POST; a 29-meet spring season ≈ 30-90 GETs
depending on how many events are stored per meet; each response is deterministic JSON, 2 KB - 3 MB, with no
per-request auth. Compare: the same coverage via Athletic.net would require per-athlete profile and per-meet
requests against a Cloudflare-protected host (403 for non-browser clients here on 2026-09-19).
**Reuse beyond ND:** every Hero's-Timing-hosted meet in any state is reachable the same way (the tenant index is
state-filtered by `ls`/`lsa`), and other AthleticLIVE tenants expose `<tenant>_meet_list/_search` on the same
host. The RTDB host + `ns=trackmeet-io` parameter is platform-wide (it was observed for the `heros` tenant),
which is the cross-state lead for agents 3/22/28 - verify per tenant before depending on it.
**Data hygiene:** join NDHSAA school names to co-op display names via the published co-op sheet (XC 91 rows,
TR 107 rows, plus 12 pre-announced future changes); treat `Coop:` labels as the team identity for XC/TF; keep
`ani`/`anli` as the athlete join key rather than name+school.

### Access characteristics
* **Class:** public, unauthenticated, static JSON/HTML; **no browser automation required** for the ND state series
  (browser only needed once to discover the Firebase/ES transport).
* **Observed behaviour:** every ND source answered `200` (ndhsaa.com apex `200`; `www.` `301` -> apex;
  ndhsaanow.com `200`; live.herostiming.com `200`; search.athletic.live `200` for `<tenant>_meet_list/_search`,
  `404` for other index names; firebaseio `200` with `?ns=trackmeet-io`, `401 "Permission denied"` without it).
  No CAPTCHA, no login wall, no JS challenge, no 429, no `Retry-After`, no bot-block page.
* **Limits honoured:** sequential fetches only, 0.7-1 s sleeps, no parallelism, no cookie replay, no UA spoofing
  beyond a stock desktop Chrome UA string, and **no further `ndhsaa.com` fetches** once the ~50-request host cap was
  reached (the coach-coverage numbers therefore rest on a 22-school systematic sample, not all 169).
* **Sensitivity:** the Firebase endpoint is a public-read database (`?ns=trackmeet-io` namespace, no token);
  treat it as a courtesy endpoint - cache results, poll once per meet day, and do not enumerate the whole
  namespace. The ES endpoint accepts arbitrary DSL, which makes broad queries tempting; keep them scoped to
  `ls`/`md` filters and modest `size` values. Athletic.net itself remains 403 to this machine and was not queried.
* **Retention/privacy:** nothing in the ND sources exposes athlete email/phone/home address; the RTDB rows carry
  an AthleticLIVE-hosted photo URL and competitor numbers - neither is in the collection contract, and the photo
  URL should not be stored.

### Recommendation
**Adopt North Dakota as three low-cost adapters, in this order:**
1. **NDHSAA school/coach adapter** (1 request/school, seasonally refreshed). Yields the full 169-school universe,
   AD/Activities Director per school, and track/XC coach names for ~75-86% of sampled rows, with co-op labels for
   team reconciliation. This is the ND answer to "authoritative coach contacts" - names + school switchboard, not
   personal emails. Priority: **high** (cheap, official, unique data).
2. **AthleticLIVE meet-index adapter** (ES POST). Enumerates ND meets for any window with **Athletic.net meet ids
   pre-attached**, which removes the need to search Athletic.net for ND meet identity. Priority: **high**.
3. **AthleticLIVE result-pull adapter** (RTDB GET per event). Full athlete-level results (name, school, year in
   school, place, time, splits, Athletic.net athlete ids) for state XC and indoor state meets and any
   Hero's-timed ND invite. Priority: **high for state series**, medium for regular season (partial platform
   coverage), and it is the single best ND Class-of-2027 verifier because `y` states the grade directly.
**Do not** build an Athletic.net-scraping path for ND: both the association link and the platform payload already
carry the Athletic.net ids needed for join-up.
**Open items for the synthesis:** (a) locate the tenant/index for the ND state outdoor track meet
(`live.athletic.net/meets/73476` is confirmed but its data host was not); (b) confirm the athlete-level `ani`
semantics once against a known Athletic.net profile; (c) decide whether the NDHSAA 2024-and-earlier state PDFs
deserve a parser (historical depth only, likely low value for Class-of-2027).

### Evidence appendix
Timestamps are America/Chicago (CDT) on 2026-09-19, taken from the session log / artifact mtimes.
| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://ndhsaa.com/` (via `www.` 301) | curl (Chrome UA) | 301 -> 200 | Association site reachable; apex serves content | 23:09 |
| `https://ndhsaa.com/schools` | curl | 200 | 169 unique member-school ids/slugs; source of `school_ids.txt` | 23:23 |
| `https://ndhsaa.com/schools/7/bismarck` | curl | 200 | School page carries AD/Activities Director, staff, enrollment + per-sport coach table (`Boys' Cross Country -> Scott Reichenberger`, `(Coop: Bismarck High)`) | 23:24 |
| `https://ndhsaa.com/schools/1045/west-fargo-sheyenne` | curl | 200 | AD/Activities/Assistant AD names on a large school | 23:24 |
| `https://ndhsaa.com/schools/100/`, `/105/ray`, `/107/south-heart` | curl | 200 | Same page schema on small schools (AD present, no emails) | 23:24 |
| 22 sampled school pages (`/schools/{3,11,19,28,36,44,52,60,68,78,86,96,105,113,122,132,140,149,158,166,1129,1378}/...`) | curl, 0.7 s apart | 200 x22 | Coach-table coverage: table 22/22; B-XC named 17/22, G-XC 16/22, B-TF 19/22, G-TF 19/22; AD present 22/22; **0 emails** | 23:30-23:31 |
| `https://ndhsaa.com/advisory/crosscountry-boys` | curl | 200 | Four named XC coach representatives with terms | 23:20 |
| `https://ndhsaa.com/board/coops` | curl | 200 | Links the official co-op Google Sheets (plus coop policy PDFs) | 23:21 |
| `https://docs.google.com/spreadsheets/d/e/2PACX-1vTxLcb44C.../pubhtml` + `.../pub?output=csv` | curl | 200 (73,131 B csv, 885 rows) | Official co-op sponsorships: `XC` 91 rows, `TR` 107 rows, status 873 Current / 9 26-27 / 3 27-28 | 23:31 |
| `https://docs.google.com/spreadsheets/d/e/2PACX-1vQImWrmBn.../pub?output=csv` | curl | 200 (45,050 B, 179 rows) | Multi-year approved calendar incl. XC `State A/B` dates and Track `A/B Regionals` + `State` dates per season | 23:31 |
| `https://docs.google.com/spreadsheets/d/e/2PACX-1vSWbnQz_P.../pub?output=csv` | curl | 200 (2,369 B, 69 rows) | XC team champions list (from `ndhsaanow.com/champions/crosscountry-boys`) | 23:31 |
| `https://docs.google.com/spreadsheets/d/e/2PACX-1vTGdzZ5aVH9-.../pub?output=csv` | curl | 200 (1,257 B, 45 rows) | Boys/Girls **Class A & B qualifying standards**, "updated: 9/17/26" | 23:31 |
| `https://ndhsaanow.com/schools` | curl | 200 | Same 169 schools as ndhsaa.com (cross-check) | 23:20 |
| `https://ndhsaanow.com/teams/crosscountry-boys` | curl | 200 | 100 XC boys teams with ids (e.g. 469 Bismarck Century) | 23:24 |
| `https://ndhsaanow.com/teams/crosscountry-girls` | curl | 200 | 96 XC girls teams | 23:24 |
| `https://ndhsaanow.com/teams/track-boys` | curl | 200 | 113 boys TF teams | 23:24 |
| `https://ndhsaanow.com/teams/track-girls` | curl | 200 | 114 girls TF teams | 23:24 |
| `https://ndhsaanow.com/tournaments/track-boys` / `track-girls` | curl | 200 | State track results chain: 2026 -> `live.athletic.net/meets/73476/events`, 2025 -> `live.athletic.net/meets/53808`, 2024- -> NDHSAA CDN PDFs; page also states "State Track Coach Information" links | 23:22 |
| `https://ndhsaanow.com/tournaments/crosscountry-{boys,girls}` | curl | 200 | XC state-series pages (no AL link rows found in the saved copy) | 23:22 |
| `https://herostiming.com/` year pages (`2026-results`, `2025-results`) | curl | 200 | ND meet chronology incl. `NDHSAA State Championship Oct 23-24 - Jamestown ND`, regionals, indoor series, and `live.herostiming.com/meets/<id>` result links | 23:18-23:19 |
| `https://live.herostiming.com/meets/55421` | curl (plain UA and browser headers) | 200 (50,184 B both) | SPA shell only - meet data is not in the HTML (two header variants byte-identical) | 23:26 |
| `https://live.herostiming.com/meets/55421/events/xc/2144806` | managed Chromium + CDP | 200 | Rendered athlete results with `Yr: SR/SO/JR` and competitor numbers; only Athletic.net link is the meet-level `View on AthleticNET` -> `https://www.athletic.net/CrossCountry/meet/261403` | 23:26, 23:29 |
| `wss://s-gke-usc1-nssi3-33.firebaseio.com/.ws?...&ns=trackmeet-io` (captured via CDP) | CDP WebSocket frames | n/a | Results transport is Firebase RTDB; paths `/meet_list/55421`, `/meet_55421/event_summary`, `/meet_55421/event_detail/Individual-2144806`, `/meet_55421/liveRunEventList/4-1` | 23:28 |
| `https://search.athletic.live/heros_meet_list/_search` (3 SPA bodies captured via CDP) | CDP + curl POST | 200 (2,622 B) | ES query shape for today/upcoming/past meet lists; curl works without a browser or auth | 23:27 |
| `https://search.athletic.live/heros_reports/_search`, `/live_results/_search` | curl POST | 404 / 404 | Only `<tenant>_meet_list` index is served by this search host | 23:27 |
| `https://search.athletic.live/heros_meet_list/_search` (`match_all`, `match ls:ND`, `match n:"North Dakota State"`, `match n:NDHSAA`, ND date-range windows) | curl POST | 200 x8 | 1,117 meet docs total; 175 ND docs; 6 ND XC meets in the 2026 season; 29 ND meets spring 2026; 9 ND outdoor meets May-Jun 2025; all with `ani` Athletic.net meet ids | 23:28-23:32 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_list/55421.json?ns=trackmeet-io` | curl GET | 200 (3,285 B) | Meet metadata incl. `ani: 261403` + `anis[0].u = https://www.athletic.net/CrossCountry/meet/261403` | 23:28 |
| same URL **without** `?ns=trackmeet-io` | curl GET | 401 `{"error":"Permission denied"}` | The namespace parameter is required (not an auth bypass; no token used) | 23:28 |
| `.../meet_55421/{event_summary,event_detail/Individual-2144806,liveRunEventList/4-1}.json?ns=trackmeet-io` | curl GET | 200 (2,395 / 1,766 / 2,313 B) | Event/round structure; entry list with Athletic.net-linked athlete id list (`h.4-1-1.ani`) | 23:28 |
| `.../meet_55421.json?shallow=true&ns=trackmeet-io` | curl GET | 200 (217 B) | Meet node key schema (`liveRunStandings`, `event_summary`, `liveRunEventList`, ...) | 23:28 |
| `.../meet_55421/liveRunStandings/4-1.json?ns=trackmeet-io` | curl GET | 200 (**234,753 B**) | Full athlete results for ND state XC 2025 Class A Boys: name, team, `y: SR/JR/SO/FR`, place, `rtm` time, splits, `ani`/`anli` | 23:28 |
| `.../meet_list/42224.json?ns=trackmeet-io`, `.../meet_42224.json?shallow=true&ns=trackmeet-io` | curl GET | 200 (3,729 B / 383 B) | Same schema for indoor track; `ani: 571583` -> `https://www.athletic.net/TrackAndField/meet/571583`; full meet node = 2,935,116 B | 23:30 |
| `https://alivestatic.athletic.net/site-skins/112/config.json` | browser fetch (observed) | 200 (1,075 B) | Skin 112 is named "NDHSAA" with NDHSAA colours | 23:25 |
| `https://livestatic.athletic.net/assets/sites/heros/config.json` | browser fetch (observed) | 200 (2,645 B) | `elasticSearchIndex: heros`, `esReportIndex: heros_reports`, `siteUrl` = tenant | 23:25 |
| `https://livestatic.athletic.net/assets/sites/base-site/config.json?...` | curl | 200 (820 B) | `primaryElasticSearchIndex: live_results`, `primaryBaseUrl: https://live.athletic.net` | 23:26 |
| `https://livestatic.athletic.net/main-5ADGVJIV.js`, `chunk-*.js` (10) | curl | 200 | Bundle confirms `api.athletic.live/api/...` and `${elasticEndpoint}` usage; error string `Http failure response for https://api.athletic.live/api/notices/my` | 23:26 |
| `https://live.herostiming.com/meets/55421/athletes` | managed Chromium + CDP | 200 | Athletes tab renders an empty state ("...for an athlete"); footer shows "© 2026 RunnerSpace.com", "Version 3.5.6" | 23:29 |
| `https://live.herostiming.com/meets/55421/results` | managed Chromium + CDP | 200 | Event links `/meets/55421/events/xc/<eventId>`; no results in HTML | 23:27 |
