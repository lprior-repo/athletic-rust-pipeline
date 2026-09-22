# 12. Iowa Wayzata/results ecosystem

Status: complete
Observed on: 2026-09-19

**Tooling note (verbatim failure):** the `web_search` tool returned `Sign up and repeat your request.` on
its first call (2026-09-19 ~23:13 CDT); all search capability was abandoned immediately and every finding
below comes from direct HTTP fetches. Peer report 24 recorded the identical string, so this is
environment-wide, not slice-specific. Nothing in this report depends on a search engine.

**Headline:** Wayzata Results' public results platform is **not an independent data source** — it is a
white-label of **AthleticLIVE, Athletic.net's own live-results product** (`live.athletic.net` →
`results.wayzatatiming.com` for the same meet id). It is nonetheless the single highest-value Iowa surface
found here, because it hands out Athletic.net MeetIDs, TeamIDs and AthleteIDs in bulk over a host that is
**not** Cloudflare-gated, and it times the Iowa state championships.

---

### Source

| Surface | Host | Role |
|---|---|---|
| `https://www.wayzataresults.com/` | PrestoSports (`ps-source-type: LocalSite`, Cloudflare+CloudFront) | Company site: services, season schedules, `/links/<code>` shortcodes |
| `http://wrresults.com/<code>` | Azure App Service (Express) | Shortlink resolver → `results.wayzatatiming.com/meets/<id>` |
| `https://results.wayzatatiming.com/` | Azure App Service + Angular SPA | **AthleticLIVE** instance ("Wayzata Results"), the public results site |
| `https://livestatic.athletic.net/assets/sites/wayzata/config.json` | Athletic.net CDN | White-label config (branding, search index names) |
| `https://search.athletic.live/` | Elasticsearch (public, unauthenticated) | Meet / team / athlete search indices |
| `https://athleticlive.blob.core.windows.net/$web/` | Azure blob (`$web`) | Per-document JSON: meet, event, results, athletes, teams |
| `https://static.trackmeetio.com/meetFiles/<MeetID>/<hash>.pdf` | S3 | Official complete-results PDFs |
| `https://s-gke-usc1-nssi3-33.firebaseio.com` (ns `trackmeet-io`) | Firebase RTDB | Live (in-progress) meet state |

Wayzata Results, LLC is a Minnesota timing/scoring/data company (Track & Field, XC, Nordic skiing, road
races). Company page self-describes clients including University of Minnesota (since 2013), University of
Iowa (since 2015), Luther College (2006), Gustavus Adolphus (2009), Hamline (2014), UNI (2012-13, 2019-).
`https://www.wayzataresults.com/about/aboutus`, 200, 23:29 CDT.

**Proof that the results site is Athletic.net infrastructure** (`config.json`, 200, 2,500 B, 23:14 CDT):
`"primaryBaseUrl": "https://live.athletic.net"`, `"primaryMachineName": "athleticlive"` (base-site config);
`"machineName": "wayzata"`, `"providedBy": "Wayzata Results"`, `"siteName": "Wayzata Results"`,
`"siteUrl": "https://results.wayzatatiming.com"`, `"elasticSearchIndex": "wayzata"`. The app loads
`https://livestatic.athletic.net/main-5ADGVJIV.js`; footers read `© 2026 RunnerSpace.com` and
"Timed by Wayzata Results, LLC."

**Same-meet aliasing, observed:** `https://live.athletic.net/meets/59382` → 301 →
`https://results.wayzatatiming.com/meets/59382` (200, 50,184 B). The meet page carries a
`View on AthleticNET` anchor → `https://www.athletic.net/TrackAndField/meet/624928`.

**Iowa ecosystem (other providers found in-slice):**

| Provider | Host(s) | Platform | Iowa evidence |
|---|---|---|---|
| Wayzata Results | `results.wayzatatiming.com`, `wrresults.com` | AthleticLIVE (machine `wayzata`) | **IHSAA state T&F + state XC meet pages link here** |
| AA Timing | `aatiming.com` | AthleticLIVE (machine `aatiming`) | IATC result links; 1,308 Iowa meets in index |
| Dakota Timing | `dakotatiming.anet.live`, `dakota` machine | AthleticLIVE | IATC links (`/meets/76356`, `/meets/75990`) |
| BW Racing Services | `bwracingservices.anet.live`, machine `bwracing` | AthleticLIVE | IATC link `/meets/75197` |
| Black Squirrel Timing | `results.blacksquirreltiming.com`, machine `blacksquirrel` | AthleticLIVE | IATC link `/meets/77526` |
| Results CM ("Results by Cal and Mike") | `resultscm.com` | Django (LiteSpeed), CSRF cookie, `/accounts/login/` | linked 4× from IATC `/results/`; **no public meet index observed** |
| True Time Racing | `results.truetimeracing.com` | ASP.NET on `racetec.net` | IHSAA state-qualifying XC results link (Pekin/1A) |
| Cross Country Ratings | `www.crosscountryratings.com` | Cloudflare + session cookie | IATC links; IA/NE/MN/ND/SD/IL/WI/MO/KS |
| IATC (index, not a timer) | `www.iatrackcoaches.org/results/` | WordPress | **curated weekly Iowa meet→result-provider index, 2018–2026** |
| Bound | `gobound.com/ia/ihsaa/...` | — | linked from IHSAA T&F state-meet central (assignment 11 scope) |

AthleticLIVE runs **258 white-label machines** (`/assets/sites/<machine>/` + `base-site`); the roster is
embedded verbatim in any instance's HTML and was extracted from `aatiming.com/meets/76994`. Iowa-relevant
members observed: `wayzata`, `aatiming`, `dakota`, `bwracing`, `blacksquirrel`, `midwesttiming`,
`badgerstate`, plus `athleticlive` itself.

---

### Coverage

**States served by Wayzata Results** (from its own meet index, 2,698 published meets, all-time;
`POST search.athletic.live/wayzata_meet_list/_search` ⇒ `{"total":2698}`, 23:16 CDT):

| State | Meets | | State | Meets |
|---|---|---|---|---|
| Minnesota | 1,852 | | Nebraska | 9 |
| **Iowa** | **598** | | South Dakota | 6 |
| Wisconsin | 178 | | Kansas | 1 |
| Illinois | 35 | | Arizona | 1 |
| North Dakota | 17 | | (unlabeled) | 1 |

Iowa is Wayzata's **second-largest state (598 meets)** — confirmed independently by the public schedule
page (`https://www.wayzataresults.com/sports/track/2026/schedule`, 200, 1.03 MB) whose rows name Iowa
venues (Clear Lake HS, Ankeny Stadium, Waukee Northwest HS, Southeast Polk HS, Sioux Central HS,
Hampton-Dumont-Cal HS) and Iowa colleges (U. of Iowa, Wartburg, Luther, Grinnell).

**Iowa HS-flagged meets by year** (`mvs` contains `highSchool`):

| Year | Indoor | Outdoor | XC | Total |
|---|---|---|---|---|
| 2019 | – | – | – | 15 |
| 2020 | – | – | – | 13 |
| 2021 | – | – | – | 23 |
| 2022 | – | – | – | 33 |
| 2023 | – | – | – | 53 |
| 2024 | 10 | 26 | 18 | 54 |
| 2025 | 9 | 31 | 22 | 62 |
| 2026 (to 09-19) | 7 | 40 | 6 | 53 |

Historical depth: Iowa 2018→2026 (9 seasons); the instance as a whole holds 2000/2016 outliers plus
2018→2026. Sports: indoor track, outdoor track, cross-country, plus road racing/Nordic on the company site.

**Notable Iowa breadth beyond HS:** collegiate indoor is a large share of Iowa rows (Iowa 2026 indoor
samples are all college: Hawkeye Invitational, Wartburg, Luther, Grinnell, Iowa Open, NJCAA Region 11).
88 of the 139 Iowa meets with no Athletic.net id are `mvs=['collegiate']`.

**Official state-meet coverage (decisive):** IHSAA's own pages link to Wayzata for the state championships:

| IHSAA page | Link | Meet |
|---|---|---|
| `/track-field/state-meet-central` ("LIVE RESULTS") | `results.wayzatatiming.com/meets/73956` | 2026 Iowa HS T&F Championships, May 21, Des Moines → **Athletic.net meet 667958** |
| `/track-field/state-meet-central` ("2025 Meet Results") | `.../meets/53818` | 2025 state T&F |
| `/cross-country/state-meet-central` ("MEET RESULTS") | `.../meets/58504` | 2025 Iowa HS State XC, Fort Dodge → 4 class Athletic.net meets (270269/270273/270275/270277) |
| same page ("2024/2023 MEET RESULTS") | `.../meets/41636`, `.../meets/28465` | 2024 & 2023 state XC (4 Athletic.net class meets each) |
| **IGHSAU** `/2026-track-field-meet-central` ("2026 State Meet Results") | `results.wayzatatiming.com/meets/73956` | **same meet as the boys' finals** — one combined state T&F meet |
| **IGHSAU** `/2025-cross-country-tournament-central` | `results.wayzatatiming.com/meets/58504/events/xc/2150205` | deep-links the *event document* analysed in this report (Class 2A Girls 5000M) |

The girls' association independently confirms the join: its 2025 XC central points straight at Wayzata's
AthleticLIVE event 2150205, and its 2026 T&F central at meet 73956. IGHSAU also publishes, alongside Wayzata,
non-AthleticLIVE state-qualifier surfaces — `results.truetimeracing.com/results.aspx?CId=16535&RId=1611&EId=5`
(girls event 5), `m1.onlineraceresults.com/race/view_plain_text.php?race_id=79937`, and result PDFs on
`ighsau.nyc3.digitaloceanspaces.com` — so Iowa's postseason has a genuine second provider to lean on. IHSAA's
XC central likewise links one True Time Racing qualifier (`...&EId=3`, boys).

---

### Enumeration

Everything below is reachable with plain HTTP; the SPA shell itself contains no data (identical 50,184-byte
shell for `/`, `/meet-list`, `/meets/<id>`), so the data plane must be addressed directly.

**1. Meets** — Elasticsearch, one POST per machine:
`POST https://search.athletic.live/<machine>_meet_list/_search`, body
`{"size":1000,"from":0,"query":{"bool":{"must":[{"term":{"pb":true}}]}}}` (verified: 2,698 docs in 4 requests;
`pb` = published). Site-native variants captured verbatim from its own XHR:
- today: `{"size":500,"query":{"bool":{"must":[{"term":{"pb":true}},{"term":{"md":"2026-09-19"}}]}}}`
- upcoming: `{"size":5,"sort":{"md":"asc"},"query":{"bool":{"must":[{"term":{"pb":true}},{"range":{"md":{"gte":"2026-09-20","lt":"2027-09-19"}}}],"must_not":[{"range":{"md":{"lt":"2026-09-20"}}}]}}}`
- by name/venue: `function_score` with `match_phrase_prefix` on `n`/`ln` + gauss decay on `sdy`.
Per-meet metadata: `n`, `son`, `ln` (venue), `ls` ("Des Moines, IA"), `lsa` (**state name**), `sdy`/`sd`/`ed`/`md`
(dates), `o` (`indoor|outdoor|xc`), `ses` (sessions), `egs`/`abs` (event groups/events), `gls` (genders),
`mvs` (`highSchool|middleSchool|collegiate|club|unattached`), `ani`/`anis` (Athletic.net meet ids/URLs),
`us` (shortlink), `ml` (PDF links), `ua` (last updated), `tna`/`tu` (timing company).
**Aggregations on `lsa`/`sdy` are rejected** (`Text fields are not optimised for … aggregations`, 400), so
state/year tallies must be computed client-side after paging — that is how the tables above were produced.

**2. Teams (schools) in a meet** — `POST https://search.athletic.live/team_list/_search`,
`{"from":0,"size":5000,"query":{"bool":{"filter":{"term":{"mi":<MeetID>}}}}}` (site's own body).
Verified for meet 67670: **12 schools**, each `{i: <TeamID>, n: <name>, ani: <Athletic.net TeamID>}`.

**3. Athletes (roster + grade) for a meet** — `POST https://search.athletic.live/athlete_list/_search`,
`{"size":1000,"query":{"bool":{"filter":{"term":{"mi":<MeetID>}},"must":{"match":{"y":"11"}}}}}`.
This index is **platform-global** (holds AA Timing meet 77461 = 247 athletes and Wayzata meet 67670 = 520
athletes), so `mi` is the only scope key. Verified `size:0` counts: 67670→520, 67671→308, 58504→1,137,
73956→**4,151**, 77475→790.
Class-of-2027 slices verified: meet 73956 with `y:"11"` → **1,265**; meet 58504 with `y:"JR"` → **349**.

**4. Events of a meet** — no JSON index; event ids appear only in the rendered DOM
(`/meets/<MeetID>/events/{individual|relay|xc}/<EventID>`), obtained in-session from the meet page. The
per-event JSON is then a direct GET (below). `session_list/_doc/<SessionID>` and `division_list/_doc/<MeetID>`
exist but are keyed by their own ids.

**5. Per-event results** — `GET https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<EventID>`
(individual) and `.../rel_res_list/_doc/<EventID>` (relay). Verified 200 for 2470196, 2471043, 2471040,
2471049, 2471031, 2471047, 2759384, 2150205, 426316.
33 documented blob doc types exist (extracted from the app's own client,
`chunk-TNNPCCBT.js`): `athlete_detail`, `combined_entries_list`, `combined_results_list`,
`combined_winners_report/winners<id>`, `division_detail`, `division_list`, `field_app_round_list`,
`field_run_relay_report/FRR|<id>`, `horizontal_series_report/HSR|<id>`, `ind_ent_list`, `ind_heat_list`,
`ind_res_list`, `individual_winners_report/winners<id>`, `live_results_meet_list`, `meet_archive_report`,
`meet_score_report`, `meet_score_spreadsheet_report`, `record_report/records|<id>`, `rel_ent_list`,
`rel_heat_list`, `rel_res_list`, `relay_winners_report/winners<id>`, `run_split_report/IRSR|<id>[-H<n>]`,
`run_split_report/RRSR|<id>`, `search_record`, `session_detail`, `session_list`, `team_detail`,
`team_detail_by_event`, `time_standard_report/timeStandards|<id>`, `track_point_mvp_report`, `vertical_series_report/VSR|<id>`,
`xc_score_report/xcSR|<id>`, `xc_split_score_report/xcSSR|<id>`, `xc_team_summary_report/xcTSR|<id>`.
**Not every type is materialised per meet** — `combined_results_list/_doc/67670`, `team_list/_doc/67670`,
`meet_archive_report/_doc/67670`, `meet_score_report/_doc/67670`, `xc_team_summary_report/xcTSR|58504` all
returned 404 while sibling types returned 200; `combined_entries_list/_doc/67670` also 404. Treat the doc-type
map as addressable-but-conditional, and always confirm per meet.

**6. Athlete profile** — `GET .../$web/athlete_detail/_doc/<AthleteID>` (200, 4,649 B for 46505023):
identity (`i`, `n`, `fn`, `l`, `y` grade, `g`, `cco`, `ani` Athletic.net AthleteID) plus **that athlete's
results at that meet** with `pr` (previous best) and `ern` (e.g. `"PB"`), event objects, and
`ev.rks[]` — **Athletic.net rankings-list links** (observed: `.../rankings/list/168416/m/800m` and
`.../rankings/list/169178/m/800m` labelled "2026 Iowa Outdoor High School Rankings").

**7. Team roster** — `GET .../$web/team_detail/_doc/<TeamID>` (200, 85,531 B for Waukee 1558850): team
identity + `ani` (Athletic.net TeamID) + `en[]` entries (each with athlete name, grade, Athletic.net
AthleteID, event) and `r[]` (47 results), `rns[]`, `rts[]`, `ers[]`.

**8. Meet-level PDF** — `ml[]` in the meet doc → `https://static.trackmeetio.com/meetFiles/<MeetID>/<hash>.pdf`
(verified 200, 317 KB, 12 pages, for 67670; also 58504, 73956, 77475).

**9. Iowa-wide discovery index** — `https://www.iatrackcoaches.org/results/` (IATC, WordPress, 200): per-week
rows of `Meet name + date` → provider page (`milesplit.com/meets/<id>`, `dakotatiming.anet.live/meets/<id>`,
`aatiming.com/meets/<id>`, `results.blacksquirreltiming.com/meets/<id>`, `resultscm.com`,
`crosscountryratings.com/results`) or a hosted PDF (`/wp-content/uploads/<YYYY>/<MM>/<Meet>-<date>.pdf`).
Paginated `/results/page/2/` … `/results/page/52/`; year filter `All Years, 2018–2026`. Page 52 holds the
2018 entries (e.g. "08/23 Iowa City West 8-23", "10/24 2020 Class 2A Cross Country State Qualifying Meet").

**10. Non-AthleticLIVE Iowa surfaces:** True Time Racing
`results.truetimeracing.com/Results.aspx?CId=<client>&RId=<race>&EId=<event>&dt=<0|1|2>&top=<n>`, athlete rows
with bib in the name cell (`Cole Millikin (#1177) 00:16:47.850`) and per-athlete pages
`myresults.aspx?uid=<CId>-<RId>-<EId>-<ResultID>`; Cross Country Ratings `crosscountryratings.com/event/<id>`
(verified `/event/1039` = "MSA SYA XC", Sep 09 2026, Sioux City IA, 149 runners, 2 races).

---

### Stable identifiers

| ID | Field / URL | Example | Notes |
|---|---|---|---|
| **AthleticLIVE MeetID** | `i`; `/meets/<id>` | 67670, 58504, 73956 | Global across the whole AthleticLIVE network — a Black Squirrel meet (77526) and a Wayzata meet (77475) share one id space. `live.athletic.net/meets/<id>` resolves to the branded host for the same meet |
| **Athletic.net MeetID** | `ani`; `anis[].ani` | 667958 (IA state T&F 2026) | The join key to Athletic.net. `-1` is a sentinel meaning "no single meet — see `anis[]`" |
| **Athletic.net meet URL** | `anis[].u` | `https://www.athletic.net/TrackAndField/meet/667958` | Emitted for multi-class meets (state XC carries 4) |
| **AthleteID (platform)** | `a.i` in results; `athlete_detail/_doc/<id>` | 46505023 | Scoped to the platform, not to a meet |
| **Athletic.net AthleteID** | `a.ani` | 21206342 | Present on 132/136 rows of a sampled XC race, 6/6 of a track race |
| **TeamID (platform)** | `t.i`; `team_detail/_doc/<id>` | 1558850 (Waukee @ 67670) | |
| **Athletic.net TeamID** | `t.ani` | 17600 = Waukee, 17238 = Ames, 17243 = Ankeny, 17411 = Indianola, 17378 = Gilbert | Present on 136/136 XC rows |
| **EventID** | `/meets/<m>/events/{individual\|relay\|xc}/<EventID>`, doc `_doc/<EventID>` | 2471047 (Boys 110mH Varsity) | Stable per event instance; class/round encoded in the name, not the id |
| **ResultID** | `r[].i` (and `r[].ro` = row order) | 38346144 | |
| **SessionID / DivisionID** | `se.i` / `dv.i` | 117926 / 99460 | Embedded in event docs |
| **Shortcodes** | `wrresults.com/<code>`, `us` field | `/fw24l5` → meet 59382 | Opaque; **not** identifiers |
| **PDF hash** | `.../meetFiles/<MeetID>/<hash>.pdf` | `69def2daa3b95` | Hash changes when re-exported |

Not identifiers: school names, meet names (`n`/`son`/`ln`), event names, `cm` (an athlete ordering key),
`lg` (Google-hosted logo URL), `sk` (event sort key).

---

### Athletic.net leverage

**Direct Athletic.net links: yes, by design, at every level.**

- Meet level: for **Iowa HS-flagged meets, 306/306 carry an Athletic.net meet URL** (`ani`/`anis`). Breakdown:
  300 single integer `ani`, 4 `ani=-1` + 2 class links, 2 `ani=-1` + 4 class links. Across all 598 Iowa meets:
  447 integer `ani`, 6+5+1 sentinel-with-links, 139 none — and those 139 are 88 collegiate + 49 unlabeled, i.e.
  **the Athletic.net join is ~100 % complete for exactly the meets we care about**.
- The meet page renders a visible `View on AthleticNET` anchor (e.g. meet 73956 → `TrackAndField/meet/667958`).
- Team level: every result row's team object carries `ani` = Athletic.net TeamID.
- Athlete level: every result row's athlete object carries `ani` = Athletic.net AthleteID.
- Ranking lists: `ev.rks[]` on event docs gives Athletic.net rankings-list URLs (e.g. list `169178` = 2026
  Iowa Outdoor HS rankings), which is the list-id family assignment 04 works with.
- The repo's own URL contract (`athletic.net/athlete/{id}/track-and-field`) means an `a.ani` is directly
  convertible to a profile URL with no Athletic.net request. Profile URL itself was **not fetched** here
  (www.athletic.net is challenge-gated to non-browser clients; see Access).

**Name+date triangulation is therefore not needed** for the covered meets — but it remains available as a
fallback for the 139 Iowa meets lacking `ani` (mostly collegiate).

**Request avoidance (measured, not modelled).** Because each event doc is one GET and each meet is a handful,
the marginal cost per Iowa meet is ~2–5 requests (meet doc + roster + 1–3 event docs) versus Athletic.net-side
discovery that costs ~1 request per athlete profile. Measured athlete volumes per Iowa meet:
Fred Smith Hawk Relays 520 · Clear Lake Boys Inv. 308 · Clear Lake XC Inv. 790 · IA State XC 1,137 ·
IA State T&F **4,151**.
Concretely: the 2026 Iowa state T&F meet yields **1,265 Grade-11 (Class-of-2027) athletes with Athletic.net
AthleteIDs**, obtainable with ~2 `athlete_list` queries plus the meet doc — against ≥1,265 profile requests if
attempted profile-by-profile on Athletic.net. The 2025 state XC meet yields 349 JR athletes the same way.
At 53 Iowa HS meets in 2026-to-date (62 in all of 2025), the full-season roster enumeration for Iowa is a
few hundred requests total.

**Caveat that must not be lost:** this is the *same data* as Athletic.net. It avoids Athletic.net **requests
and Cloudflare friction**, not Athletic.net as the upstream. If Main's goal is "verify independently of
Athletic.net", Wayzata/AthleticLIVE is **not** independent — AA Timing, True Time Racing and Cross Country
Ratings are the Iowa surfaces that are.

---

### Athlete evidence

| Field | Availability | Evidence |
|---|---|---|
| name / first / last | Yes | `a.n`, `a.fn`, `a.l` |
| graduating class / grade | **Yes** | `a.y` — numeric `"9"…"12"` on track (`RunMeet`) events; `"FR"|"SO"|"JR"|"SR"` on XC. 136/136 populated on a sampled XC race; 6/6 on a track race |
| school | Yes | `a.t.n`/`a.t.f` + `a.t.i` |
| city / state | Partial | `a.cco` = `USA` only; the venue's city/state lives on the meet (`ls`, `lsa`). No per-school city/state field |
| gender / category | Yes | `a.g` (`Male`/`Female`) and event `gl` (`Boys`/`Girls`), `g`, plus `sg` for division-specific e.g. `Boys (Varsity)` |
| TF/XC distinction | Yes | meet `o` ∈ `indoor|outdoor|xc`; event `xc` boolean; event `ab`/`un` |
| indoor/outdoor | Yes | meet `o`; `tf: 12` appears on both, so use `o` |
| performances | Yes | `r[].m` (display), `r[].im` (integer mark), `r[].s` (seed), `ev.tst[]` (time standards) |
| PRs | Yes | `pr` = previous best on `athlete_detail` rows; `ern` = `"PB"` marker |
| progression | Partial | only within an instance/meet set; `athlete_detail` is meet-scoped, so progression = union over that athlete's meets |
| meets | Yes | every result carries `mi`; `athlete_detail` is keyed per meet |
| athlete profile URL | **No public URL** — `/meets/<id>/athletes*` is robots-disallowed. Use `athlete_detail/_doc/<AthleteID>` JSON instead |
| relay membership | Yes | `rts[].rm[]` with `to` = leg order and a full athlete object per leg |

---

### Recruiting information

**Not applicable — the platform exposes no coach or administrator data.** Explicitly checked: the full
`team_detail/_doc/1558850` document contains no key or value matching `coach` or `contact` (zero hits), and
the only contact string anywhere on the surfaces is the generic timer inbox `results@wayzataresults.com`
(site footer, and the meet doc's `qe` field). Meet docs carry the timing company name/URL (`tna`/`tu`), not
school staff. Nothing here contributes to the coach-contact graph; assignment 29's sources remain the only
path. No athlete personal contact data was collected or is exposed.

---

### Result evidence

Sampled across an Iowa HS dual-format meet (67670), a throws/live meet (67671) and the state championships
(58504, 73956):

| Field | Present? | Key / example |
|---|---|---|
| ResultID | Yes | `r[].i` = 38346144 |
| AthleteID | Yes | `a.i` = 46505023 |
| Athletic.net AthleteID | Yes | `a.ani` = 21206342 |
| MeetID | Yes | `mi` = 67670 (plus `ani` = 657368 for the Athletic.net side) |
| EventID | Yes | doc id + `i` = 2471047 |
| TeamID | Yes | `t.i` = 1558850, `t.ani` = 17600 |
| mark | Yes | `m` = `"10:34.63"`, `"56-07.00"`, `"21-08.50"`, `"15.08"` |
| normalized mark inputs | Yes | `im` integer: time events in **milliseconds** (10:34.63 → 634628; 18:20.7 → 1100700), horizontal/throws in **micrometres** (56-07.00 → 17,246,600; 21-08.50 → 6,616,700). `vm` = valid-mark flag |
| timing method | **No** — not exposed; `upts` on the meet shows ingestion route (`finishlynxRuntimeConfig`, `hytekDbConnConfig`, `meetproFtpConfig`, `runMeetConfig`, `fieldAppResultsConfig`, `finishlynxXcLifConfig`) |
| wind | **Yes, where the timer entered it** — `r[].w` (`"-0.2"`, `"1.0"`) with event flag `hwr`. Observed `hwr=false`/blank at regular-season Iowa HS meets, `hwr=true` with real readings at the state meet |
| implement / hurdle specification | **No** — event names are generic (`Shot Put`, `Discus Throw`, `110m Hurdles`); no weight/height field. `sk` (50064/50004/164) is a sort key, not a spec |
| heat / round | Yes | `hn` heat; `runm` = `Finals`/`Preliminaries`; `rui` = `"8-1"`; `nh` = heat count; qualifier marker appended to `p` (`10.65q`) |
| place | Yes | `p` + `hl` (display) + `ro` (row order) |
| points | Yes | `pt` = 10; plus `gap` (to leader) and `iv` (to previous) |
| date | Yes | meet `sdy`/`md`/`ds`; session `ds`; `ua` = last updated |
| school represented | Yes | `a.t.n` (+ `ani`) |
| relay membership | Yes | `rts[].rm[]` with `to` leg number, athlete object per leg, `rd` relay designation |
| attempts / series | Yes (per doc type) | `horizontal_series_report/HSR\|<id>`, `vertical_series_report/VSR\|<id>`, `field_run_relay_report/FRR\|<id>`, `run_split_report/IRSR\|<id>` (splits) |
| time standards | Yes | `ev.tst[]`, e.g. `{"n":"Drake Relays HS Standards","ab":"Drake Relays","m":"1:54.00"}` |

---

### Incremental use

Designed for, and verified against, the platform's own polling patterns:

1. **New/changed meets** — page the meet index once per poll (`pb:true`), sorted or filtered on `md`/`sdy`.
   The site itself asks for `md == today` (size 500) and for the next 12 months of `md`. **Future meets are
   already indexed** — on 2026-09-19 the AA Timing index held meets dated 2026-09-22/24 — so a schedule
   (not just results) can be pulled ahead of time. Cheapest delta: run the same `ate`-style query and diff
   on `(i, ua)`; `ua` is the meet's last-updated timestamp.
2. **Changed results inside a meet** — event docs carry `ua`, `frua`, `fruam` (first-result-updated, epoch ms);
   compare `frua`/`ua` to skip untouched events. Do **not** re-pull every event every week.
3. **Zero-byte revalidation (proven)** — the blob store returns `ETag` + `Last-Modified` and honours
   conditional requests: re-GET of `ind_res_list/_doc/2471043` with `If-None-Match: 0x8DE9AEBEEF8A535` →
   **304, 0 bytes**; with `If-Modified-Since` in the past → **304, 0 bytes**. A weekly refresh over a
   historical corpus is therefore nearly free in bytes.
4. **New athletes / affected athletes** — `athlete_list/_search` filtered by `mi` (and optionally `y`) gives
   the roster for a meet in ≤5 requests (size 1,000, `max_result_window` 10,000). New meets → new rosters;
   changed meets → re-query only those `mi`.
5. **Iowa-wide cross-timer sweep** — because meet ids and the `athlete_list`/`team_list` indices are
   platform-global, one pass over `live_results_meet_list` (filtered to `lsa: Iowa`) plus `athlete_list`
   per new `mi` covers every AthleticLIVE timer in Iowa, not just Wayzata. (Index confirmed to exist and to
   return 10,000+ docs; Iowa-wide count with `match: {lsa: "Iowa"}` = **3,476 meets** across the network.)
6. **Non-AthleticLIVE deltas** — poll IATC `/results/` (and `/results/page/2/`) once a week during XC
   season: the new page is the authoritative "which Iowa meets happened and who timed them" delta, and the
   52-page archive backfills 2018→2026.

Recommended polling plane: meet index (1 req/machine/day) → diff `(i, ua, frua)` → per-changed-meet
`athlete_list` + changed event docs via conditional GET.

---

### Access characteristics

| Surface | Classification | Detail |
|---|---|---|
| `www.wayzataresults.com` | **normal HTML** | curl default UA → **403**; browser UA → 302 → 200. `robots.txt` (200) is PrestoSports': `User-agent: *` disallows `/cgi-bin/`, `/_private/`, `/_vti_*/`, **`/reports/`**, `/admin/`, `/action/`, `Crawl-Delay: 10`. Schedules/sitemap: `/sitemap.xml` 404 |
| `wrresults.com` | **redirector** | `robots.txt` → **204 (empty, no policy)**; `/<code>` → 301 to the AthleticLIVE meet |
| `results.wayzatatiming.com` | **browser application** | 50,184-byte Angular shell, no HTML data. `robots.txt` (200): `Disallow: /admin*`, **`/meets/*/athletes*`**, `/meets/*/live*`, `/meets/*/teams*`, `/meets/*/follow*`; **`Sitemap: …/sitemap.xml`** → 200, only 2 URLs (`/`, `/meet-list`). These directives were honoured: no disallowed path was fetched |
| `search.athletic.live` | **public structured JSON (undocumented API)** | Elasticsearch over HTTPS. Answers **with no auth**: a `size:0` query sent by plain curl with no `Origin`/`Referer`/`Authorization` returned 200, `total: 2698`. `/robots.txt` is 403 (`security_exception`), so no robots policy is served for this host; the cluster root `/` returns 200 discovery info. Treat as unadvertised, not as an invitation — the production collector should throttle far below the website's own traffic profile |
| `athleticlive.blob.core.windows.net/$web` | **public structured JSON** | Per-document GET, `Access-Control-Allow-Origin: *`, `ETag`/`Last-Modified`, **304 supported**. No robots.txt (400 for that path). Some doc types 404 per meet |
| `static.trackmeetio.com` | **downloadable PDF** | `robots.txt` → S3 `AccessDenied` (no policy). Official results PDFs, 300 KB-ish, 12+ pages |
| Firebase `trackmeet-io` | **browser application (live stream)** | Only used in-session for in-progress meets |
| `resultscm.com` | **authenticated** (partly) | `/accounts/login/`, `/accounts/register/`, CSRF cookie; no public meet index observed on `/` |
| `results.truetimeracing.com` | **normal HTML (ASP.NET)** | Public `Results.aspx` pages, no auth |
| `crosscountryratings.com` | **normal HTML + session cookie** | Public `/results` and `/event/<id>`; Search/Goal-Setting behind 🔒 |
| **429 / `Retry-After`** | **never observed** | No rate-limit headers on ES or blob responses (checked `ratelimit*`, `retry-after`, `x-azure`, `x-cache`); no 429 in any of my requests |

**Request counts (disclosed in full, because one host exceeded the ~50 guidance).** Sequential, 1–2 s
spacing, browser UA, no parallelism:
`search.athletic.live` **≈59** (index probes 6, meet paging 4, provider matrix 14, athlete counts 7, link/PDF
metadata 4, misc 24) — above the brief's "~50" aim; I stopped there and did **not** complete an Iowa-wide
per-meet athlete sweep, which would have needed ~300 more. No 429, no `Retry-After`, no block resulted.
`livestatic.athletic.net` ≈26 · `athleticlive.blob.core.windows.net` ≈20 ·
`www.wayzataresults.com` 13 · `results.wayzatatiming.com` ≈14 (6 curl + 8 browser navigations) ·
`wrresults.com` 3 · `www.iatrackcoaches.org` 2 · `www.iahsaa.org` 3 · `ighsau.org` 4 ·
`www.athletic.net` 2 · IATC/providers ≈10 · `static.trackmeetio.com` 1.
If this becomes a production adapter, budget the ES/blob endpoints as a shared multi-tenant service
(all AthleticLIVE timers hit the same cluster) and cache aggressively — conditional GETs make it cheap.

**Cloudflare posture (relevant finding):** `www.athletic.net/TrackAndField/meet/667958` → **403** ("Just a
moment…", 5,460 B) with curl's default UA; **200 with a browser UA but only a 9,306-byte app shell** — the
page is a shell either way and the API behind it is the gated one. `live.athletic.net` and
`results.wayzatatiming.com` return 200 to the same client. So the AthleticLIVE path is a *publisher's own
public host*, not a bypass of an access control: the same data is served to any visitor, on a host this
site's robots.txt explicitly exposes with a sitemap. No cookie replay, no fingerprint spoofing, no proxy
rotation, no CAPTCHA handling, no login was used anywhere in this slice.

---

### Recommendation

**RESULT-SOURCE** for Iowa (and, by extension of the same machinery, the cheapest RESULT-SOURCE available
for every AthleticLIVE-network timer in the Midwest) — **with ATHLETIC.NET-SEED as a compound function**,
because the same documents hand over Athletic.net MeetIDs, TeamIDs and AthleteIDs outright.

Expected marginal coverage:

- **Iowa (primary):** the state championships for T&F and XC are on this platform, and 306 Iowa HS meets are
  indexed with 100 % Athletic.net meet linkage. Both associations point at it from the top of their
  state-meet pages (IHSAA → meet 73956 for T&F, 58504 for XC; IGHSAU → the *same* 73956, and the same XC
  meet via event deep-link 2150205), which makes the Athletic.net join association-confirmed rather than
  name+date triangulated. Class-of-2027 (grade 11 / JR) athletes come with name, school,
  grade, mark, wind/heat/round, ResultID and Athletic.net AthleteID — measured 1,265 grade-11 athletes at the
  2026 state T&F meet alone.
- **Cross-state bonus:** the same four endpoints (meet index → `athlete_list` → `team_list` → event docs)
  work per machine, so `aatiming` (1,308 Iowa meets), `dakota` (253), `bwracing` (115), `blacksquirrel` (120)
  and `wayzata` (615) are five Iowa-relevant machines under one design; `live_results_meet_list` gives the
  whole network (3,476 Iowa meets) in one index.
- **Marginal cost:** ~2–5 requests per meet versus ~1 Athletic.net profile request per athlete; conditional
  GETs make weekly re-validation ~0 bytes for unchanged events.
- **What it does NOT buy:** independent verification. It is Athletic.net's own product (RunnerSpace footer,
  `live.athletic.net` as `primaryBaseUrl`), so it is a **reseller/white-label of the same pipeline, not a
  competing source**. For independent corroboration in Iowa, pair it with AA Timing's data *only if* sourced
  outside AthleticLIVE, and with the genuinely independent surfaces: True Time Racing (racetec.net),
  Cross Country Ratings, IATC's hosted PDFs, and Bound (assignment 11).
- **Gaps to route elsewhere:** coach/AD contacts (none here — assignment 29), implement/hurdle specs and
  timing method (absent), athlete city/state (derive by joining school → association directory),
  regular-season meets not timed by an AthleticLIVE member (use IATC + MileSplit, assignments 11/27).

---

### Evidence appendix

All times 2026-09-19 CDT (America/Chicago). `GET` unless noted. "curl-UA" = default curl User-Agent;
"browser-UA" = a normal Chrome UA header (no other browser emulation).

| URL | Method | HTTP | What it proved | Time |
|---|---|---|---|---|
| `https://wayzataresults.com/` | GET curl-UA | 403, 919 B | Host blocks non-browser clients (Cloudflare) | 23:13 |
| `https://wayzataresults.com/` | GET browser-UA | 302 → `/index` → `/landing/index` (200, 2,729,258 B) | PrestoSports site; `ps-source-type: LocalSite`; nav exposes `/sports/{track,xc,rr}/2026/schedule`, `/records` | 23:13 |
| `https://wayzataresults.com/robots.txt` | GET | 200 | PrestoSports policy; `Disallow /reports/` for `*`; `Crawl-Delay 10` | 23:13 |
| `https://www.wayzataresults.com/sports/track/2026/schedule` | GET | 200, 1,031,235 B | 386 dated meet rows in one HTML table with `/links/<code>` result links; Iowa venues present (Ankeny, Waukee NW, Clear Lake, Southeast Polk, Sioux Central, Hampton-Dumont-Cal) | 23:13 |
| `https://www.wayzataresults.com/sports/xc/2026/schedule` | GET | 200, 373,440 B | 147 XC rows, 49 unique result links | 23:13 |
| `https://www.wayzataresults.com/sports/rr/2026/schedule` | GET | 200, 215,152 B | Road-race schedule (separate sports line, not HS) | 23:13 |
| `https://wayzataresults.com/sitemap.xml` | GET | 404 | No sitemap on the company site | 23:13 |
| `https://wayzataresults.com/links/gqumkk` | GET | 301 → `http://wrresults.com/fw24l5` | Result links are two-hop shortcodes | 23:14 |
| `http://wrresults.com/` | GET | 301 → `https://results.wayzatatiming.com/` | Second host is the results platform | 23:14 |
| `http://wrresults.com/robots.txt` | GET | **204** (empty) | No robots policy on the shortener | 23:14 |
| `http://wrresults.com/fw24l5` | GET | 301 → `https://results.wayzatatiming.com/meets/59382` | Shortcode → numeric AthleticLIVE MeetID | 23:14 |
| `https://results.wayzatatiming.com/robots.txt` | GET | 200 | `Disallow /admin*, /meets/*/athletes*, /meets/*/live*, /meets/*/teams*, /meets/*/follow*`; `Sitemap: …/sitemap.xml` | 23:14 |
| `https://results.wayzatatiming.com/sitemap.xml` | GET | 200, 840 B | Only 2 URLs (`/`, `/meet-list`); `last-modified 2026-09-19` | 23:14 |
| `https://results.wayzatatiming.com/meet-list` | GET | 200, 50,184 B | SPA shell; "Wayzata Results is part of the AthleticLIVE Enterprise Network"; title `AthleticLIVE` | 23:14 |
| `https://livestatic.athletic.net/assets/sites/base-site/config.json` | GET | 200, 820 B | `primaryBaseUrl: https://live.athletic.net`, `primaryMachineName: athleticlive` | 23:14 |
| `https://livestatic.athletic.net/assets/sites/wayzata/config.json` | GET | 200, 2,500 B | White-label identity: `machineName: wayzata`, `siteUrl: results.wayzatatiming.com`, `providedBy: Wayzata Results`, `elasticSearchIndex: wayzata` | 23:14 |
| `https://live.athletic.net/meets/59382` | GET browser-UA | 200 | Resolves to `results.wayzatatiming.com/meets/59382` — same meet id, branded host | 23:14 |
| `https://results.wayzatatiming.com/meets/59382` | browser navigate | 200 | Hawkeye Invitational, Iowa City IA; anchors: **`https://www.athletic.net/TrackAndField/meet/624928`**, `/meets/59382/{live,events,teams,athletes,reports/winners,reports/records}`; "Timed by Wayzata Results, LLC"; footer `© 2026 RunnerSpace.com` | 23:14 |
| `https://results.wayzatatiming.com/meet-list` (browser) | XHR capture | 200 | Site's own query bodies: `{"size":500,…,{"term":{"pb":true}},{"term":{"md":"2026-09-19"}}}` and the `md`-range next/previous queries on index `wayzata_meet_list` | 23:15 |
| `POST https://search.athletic.live/wayzata_meet_list/_search` | POST | 200 | `{"size":0}` ⇒ **2,698 published meets**; sample docs expose `i`, `ani`, `anis[].u`, `lsa`, `ls`, `sdy`, `o`, `mvs`, `ml`, `us`, `ua` | 23:15–23:19 |
| same, `{"size":1000,"from":0..2000}` | POST ×4 | 200 | Full 2,698-doc enumeration → state/year tables; 343 meets without an Athletic.net id | 23:16 |
| same, aggregation on `lsa`/`sdy` | POST | **400** | ES refuses per-document aggregations on text fields (design note for the collector) | 23:15 |
| `POST …/wayzata_meet_list/_search` (no Origin/Referer/auth, curl-UA) | POST | 200 | The data API is **unauthenticated** | 23:47 |
| `POST …/{wayzata,dakota,bwracing,aatiming,blacksquirrel,midwesttiming,badgerstate}_meet_list/_search` | POST ×14 | 200 (all) | Iowa meet counts: 615 / 253 / 115 / **1,308** / 120 / 8 / 0; totals 2,761 / 916 / 116 / 1,344 / 1,877 / 256 / 54 | 23:40 |
| `POST …/live_results_meet_list/_search` | POST | 200 | Platform-wide index exists, `total ≥ 10,000`; `match lsa:Iowa` ⇒ **3,476** Iowa meets network-wide | 23:29 |
| `POST …/athlete_list/_search` (`mi=67670`, prefix "Patel") | POST | 200 | 2 hits — "Rishi Patel"/"Rishabh Patel", grade 12, `ani` 21206344/21206342 → athlete search works and is meet-scoped | 23:27 |
| `POST …/athlete_list/_search` (`size:0`, various `mi`) | POST ×7 | 200 | Athlete counts: 67670→520, 67671→308, 58504→1,137, 73956→**4,151**, 77475→790; `mi=73956&y=11`→**1,265**; `mi=58504&y=JR`→**349**; AA Timing `mi=77461`→247 (index is platform-global) | 23:44–23:46 |
| `POST …/team_list/_search` (`mi=67670`, size 100) | POST | 200 | **12 schools** with Athletic.net TeamIDs (Waukee 17600, Ames 17238, Ankeny 17243, Indianola 17411, Gilbert 17378, …) | 23:36 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2470196` | GET | 200, 6,396 B | Boys 3200m Varsity; full result schema incl. `r[].i`, `im`, `w`, `hn`, `a.y`, `a.ani`, `t.ani` | 23:21 |
| `…/ind_res_list/_doc/2471043` | GET | 200, 15,710 B | Boys 100m Varsity; `hwr=false`, blank `w` at a regular-season Iowa HS meet | 23:39 |
| `…/ind_res_list/_doc/2471040`, `…/2471049`, `…/2471031`, `…/2471047` | GET ×4 | 200 | Shot Put / Discus / Long Jump / 110mH: marks in feet-inches, `im` in µm; **no implement weight or hurdle height field** | 23:36 |
| `…/ind_res_list/_doc/2759384` | GET | 200, 20,379 B | IA state T&F 2026, Boys 100m 4A Prelims: `hwr=true`, `w` = `-0.2`/`1.0`, `hn` heat, `p` = `10.65q`, ResultIDs, A.net athlete+team ids | 23:42 |
| `…/ind_res_list/_doc/2150205` | GET | 200, 159,531 B | IA state XC 2025, Class 2A Girls 5000m: 136 finishers, grade `FR/SO/JR/SR` 136/136, A.net athlete ids 132/136, A.net team ids 136/136 | 23:32 |
| `…/rel_res_list/_doc/426316` | GET | 200, 9,554 B | Relay doc: `rts[].rm[]` legs with `to` order and full athlete objects incl. grade + A.net athlete id | 23:24 |
| `…/athlete_detail/_doc/46505023` | GET | 200, 4,649 B | Athlete profile JSON: name, grade 12, `ani` 21206342, per-meet results with `pr` and `ern:"PB"`, `ev.rks[]` → **`athletic.net/TrackAndField/rankings/list/169178/m/800m` ("2026 Iowa Outdoor High School Rankings")** | 23:22 |
| `…/team_detail/_doc/1558850` | GET | 200, 85,531 B | Waukee roster: 47 entries + 47 results; **zero `coach`/`contact` keys anywhere** | 23:22 |
| `…/combined_results_list/_doc/67670`, `…/combined_entries_list/_doc/67670`, `…/team_list/_doc/67670`, `…/meet_archive_report/_doc/67670`, `…/meet_score_report/_doc/67670` | GET ×5 | 404 | Doc types are addressable but not materialised for every meet | 23:22–23:23 |
| `…/session_list/_doc/67670`, `…/division_list/_doc/67670`, `…/combined_winners_report/_doc/winners67670`, `…/ind_heat_list/_doc/2470196` | GET ×4 | 200 | Those types do exist (session doc keyed by SessionID, not MeetID) | 23:23 |
| `https://athleticlive.blob.core.windows.net/$web?restype=container&comp=list` | GET | 404 | **Container listing is not open** — no bulk blob enumeration | 23:22 |
| `…/ind_res_list/_doc/2471043` with `If-None-Match: 0x8DE9AEBEEF8A535` | GET | **304, 0 B** | Conditional revalidation works | 23:44 |
| `…/ind_res_list/_doc/2471043` with `If-Modified-Since` (past) | GET | **304, 0 B** | Date-based revalidation works | 23:44 |
| blob headers on `…/ind_res_list/_doc/2470196` | GET -D | 200 | `ETag`, `Last-Modified`, `Access-Control-Allow-Origin: *`, `x-ms-blob-type: BlockBlob`; **no rate-limit headers** | 23:33 |
| `https://search.athletic.live/robots.txt`, `/` | GET | 403, 200 | No robots policy served; cluster root readable without auth (no write attempted) | 23:33 |
| `https://static.trackmeetio.com/robots.txt` | GET | 403 (S3 AccessDenied) | No robots policy on the PDF host | 23:33 |
| `https://static.trackmeetio.com/meetFiles/67670/69def2daa3b95.pdf` | GET | 200, 317,088 B, 12 pp | Official complete-results PDF; produced by **Nitro PDF Pro 14** (a print-to-PDF, **not** a native Hy-Tek export) | 23:47 |
| `https://results.wayzatatiming.com/meets/58504`, `/73956`, `/41636`, `/28465` | curl + browser | 200 | State-meet pages; `athletic.net/CrossCountry/meet/270269,270273,270275,270277` anchors on 58504; XC event ids 2150199–2150207 | 23:31–23:41 |
| `POST …/wayzata_meet_list/_search` (`i` ∈ 73956/58504/41636/28465) | POST ×4 | 200 | "Iowa High School Track & Field Championships" 2026-05-21 `ani=667958`; state XC 2025/2024/2023 `ani=-1` with 4 `anis[]` class links each | 23:33 |
| `https://www.iahsaa.org/track-field/state-meet-central` | GET | 200, 241,562 B | **"LIVE RESULTS" → `results.wayzatatiming.com/meets/73956`**; 2025 → `/meets/53818`; `gobound.com/ia/ihsaa/boystrack/2025-26/leaders`; IATC results link | 23:26 |
| `https://www.iahsaa.org/cross-country/state-meet-central` | GET | 200, 202,850 B | **"MEET RESULTS" → `/meets/58504`, `/41636`, `/28465`**; one `results.truetimeracing.com/results.aspx?CId=16535&RId=1611&EId=3` (Pekin 1A qualifier) | 23:26 |
| `https://ighsau.org/2026-track-field-meet-central` | GET | 200, 57,989 B | Girls' association links **the same state T&F meet** `results.wayzatatiming.com/meets/73956`; `gobound.com/ia` | 23:53 |
| `https://ighsau.org/2025-cross-country-tournament-central` | GET | 200, 59,737 B | Girls' state XC links **`results.wayzatatiming.com/meets/58504/events/xc/2150205`** (the exact event doc analysed above), plus `results.truetimeracing.com/...&EId=5`, `m1.onlineraceresults.com/race/view_plain_text.php?race_id=79937` and 10 result PDFs on `ighsau.nyc3.digitaloceanspaces.com` | 23:53 |
| `https://www.iatrackcoaches.org/results/` | GET | 200, 137,880 B | Iowa weekly results index: provider links to `milesplit.com/meets/<id>`, `dakotatiming.anet.live/meets/75990,76356,76357`, `bwracingservices.anet.live/meets/75197`, `aatiming.com/meets/76921,76994,77461`, `results.blacksquirreltiming.com/meets/77526`, `resultscm.com`, `crosscountryratings.com`; hosted PDFs `/wp-content/uploads/2026/09/*.pdf`; pagination to `/results/page/52/` | 23:26 |
| `https://www.iatrackcoaches.org/results/page/52/` | GET | 200, 129,759 B | 2018 entries + year filter 2018–2026 → archive depth | 23:29 |
| `https://ighsau.org/` | GET | 200, 80,513 B | Girls association hub; `/sports/track-field`, `/sports/cross-country`, `/2026-track-field-meet-central` | 23:26 |
| `https://results.truetimeracing.com/results.aspx?CId=16535&RId=1611&EId=3&dt=1&top=10` | GET | 200, 33,386 B | "1A State Qualifying Meet - Wildwood Park, 10/23/2025"; rows `Cole Millikin (#1177) 00:16:47.850`; per-athlete `myresults.aspx?uid=16535-1611-3-<ResultID>` | 23:51 |
| `https://resultscm.com/` | GET | 200, 12,720 B | "Results by Cal and Mike"; only `/accounts/login/`, `/accounts/register/`, `runablaze.com/history` — no public meet index | 23:51 |
| `https://www.crosscountryratings.com/results` | GET | 200, 131,344 B | Iowa-anchored XC results portal (IA/NE/MN/ND/SD/IL/WI/MO/KS); Search/Goal-Setting gated; `/event/<id>` link list | 23:51 |
| `https://www.crosscountryratings.com/event/1039` | GET | 200, 102,485 B | "MSA SYA XC", 2026-09-09, Sioux City IA, 149 runners, 2 races — public structured result page | 23:51 |
| `https://www.athletic.net/TrackAndField/meet/667958` | GET curl-UA | **403**, 5,460 B ("Just a moment…") | Athletic.net remains Cloudflare-challenge-gated to non-browser clients | 23:47 |
| `https://www.athletic.net/TrackAndField/meet/667958` | GET browser-UA | 200, 9,306 B shell | Even with a browser UA only an app shell is returned — the data API stays gated | 23:47 |
| `https://livestatic.athletic.net/main-5ADGVJIV.js` + 23 `chunk-*.js` | GET ×24 | 200 | API client `chunk-TNNPCCBT.js`: the 33 blob doc-type paths, `getMeetsByDay`/`getRangeMeets`/`getMeetsByName` ES bodies, `getAthleteList` (`athlete_list/_search`, filter `mi`), `getTeamList` (`team_list/_search`, filter `mi`), `searchEndpoint` default `athleticlive.blob.core.windows.net/$web`, `elasticEndpoint` `search.athletic.live`, Firebase `trackmeet-io` paths | 23:14–23:19 |
| `https://aatiming.com/meets/76994`, `https://results.blacksquirreltiming.com/meets/77526` | GET | 200, 50,184 B each | Both are AthleticLIVE white-labels; each embeds the full **258-machine** `/assets/sites/<machine>/` roster | 23:35 |

Local tooling written for this slice (scripts only, per contract):
`/home/lewis/Downloads/midwest-tfxc-source-research/tools/12_wayzata_meet_index.py` — pages the public
`wayzata_meet_list` index (1 count + 1 request per 1,000 docs, 2 s spacing) and prints the state/year
coverage tables plus the Athletic.net-link census. Reproduces the 2,698-meet figure in 4 requests.
