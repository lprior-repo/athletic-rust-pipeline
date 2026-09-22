# 44. Iowa gap follow-ups (IHSAA 24-school resolution, coach coverage beyond Bound, 2026 timer delta)

Status: complete
Observed on: 2026-09-20 (America/Chicago)

This file closes the three open items left by report 11 (`11-iowa-ihsaa-ighsau.md`) and report 12
(`12-iowa-wayzata-results.md`):

1. **The "24 IHSAA members with no Bound T&F team" resolves to an exact rule: 33 members lack a
   name-identical Bound boys-T&F team, and they split 10 / 18 / 5** — 10 are Bound display-name
   variants (team exists), 18 are T&F co-op guests whose athletes compete under a host school's
   Bound team, 5 field no T&F or XC program at all. Report 11's 8 named examples decompose
   4 variants + 3 co-op + 1 no-program, which is why the earlier figure was approximate.
2. **Iowa coach coverage beyond Bound: the two associations publish no school-level coach contacts at
   all** (IHSAA's own contact page says "Find School Contacts at Bound"), the coaches' association
   publishes 163 member *schools* and an advisory board with 9 named people, and across 11 district
   site probes only **1 head-coach email + 7 role-labelled AD/activities emails** were reachable.
3. **Timer delta: 11 AthleticLIVE timing companies that operate Iowa 2026 meets are absent from report
   12's Iowa timer table** (Shannon Event Timings 66 meet docs, Kauder Racing 46, Rapid Timing 16,
   Race the Clock 12, Blue Machine 7, Hero's 7, Meade 3, WestWood 3, TC Solutions 2, Buffalo Ridge 1,
   Flash Results 1 = 164 of the 768 2026 Iowa meet docs), plus 17 school-run timer identities (63 docs).
   Report 12's non-AthleticLIVE entries
   (True Time Racing, Results CM, Cross Country Ratings) do not appear in the AthleticLIVE index at
   all — they publish outside it, so they are not a delta, they are a separate lane.

---

### Source

| Source | URL(s) | Role in this report |
|---|---|---|
| IHSAA classifications | `https://www.iahsaa.org/classifications/cross-country/` (canonical of the retained page) | XC class list + **Sharing Agreements** column = the co-op resolver |
| IHSAA track classification CSV | `tools/iowa-11/ihsaa-track-class-2026.csv` (351 rows, cols `School, Class, TotalBedsCount, Coops`) | T&F co-op / postseason-sharing column |
| IHSAA member list | `tools/iowa-11/ihsaa-members.csv` (379 rows) | The universe being resolved |
| Bound (gobound.com) | `https://www.gobound.com/ia/schools/<slug>`, plus `tools/iowa-11/bound-ihsaa-boystrack-teams-2025-26.csv` (365), `b-teams-boysxc-2026.csv` (365), `bound-ighsau-girlstrack-teams-2025-26.csv` (365), `b-teams-girlsxc-2026.csv` (369), `bound-ia-schools.csv` (444) | team/program presence; the *absence* signal for the 5 program-less schools |
| IHSAA contact/coaches pages | `https://www.iahsaa.org/contact/`, `https://www.iahsaa.org/coaches-administrators/` | official coach-directory check |
| IGHSAU | `https://ighsau.org/`, `/staff`, `/contact-us`, `/robots.txt` | official girls-side directory check |
| IATC | `https://www.iatrackcoaches.org/membership-list/`, `/track-and-field-advisory-board/`, `/results/` | school-membership list, named board, weekly result index |
| 11 district/school sites | Ames (`ameshighathletics.org`, `amescsd.org`), Ankeny, Waukee, West Des Moines, Dowling, Dubuque, Linn-Mar, Cedar Falls, Southeast Polk, Bettendorf, Johnston | public coach/AD email probes |
| AthleticLIVE search | `POST https://search.athletic.live/live_results_meet_list/_search` | 2026 Iowa meet + timer census |
| AthleticLIVE white-labels | `shannoneventtimings.anet.live`, `live.kauderresults.com`, `live.rapidresultstiming.com`, `aatiming.com` (+ company sites `kauderresults.com`, `rapidtiming.net`) | operator confirmation |

### Coverage

* **Iowa only**, all four target sports (boys/girls T&F, boys/girls XC), member schools only (379 IHSAA
  members; IGHSAU BEDs = 379 per report 11).
* Seasons covered by the census: **calendar 2026** — 768 Iowa meet docs with `sdy` in 2026, of which
  630 are Jan–Jul (spring T&F) and **138 are the current XC season** (`sdy` in 2026-08-01 → 2026-10-22;
  the late dates are future-dated state-qualifying/postseason entries, not results). A `sdy >=
  2026-08-01` filter returns 142 docs because it also picks up four advanced/placeholder entries dated
  2027-03-30, 2027-04-15, 2027-04-20 and 2038-04-02.
* Historical depth for the co-op rule: the 2026-27 school-year classification pages and the 2025-26
  Bound team indexes; the sharing-agreement pattern is stable and re-published each year, so the rule
  transfers across seasons but the guest list must be re-read annually.
* Historical depth for the timer census: `live_results_meet_list` holds all AthleticLIVE tenants'
  meets; the 772-doc Iowa slice returned 768 docs dated 2026, 3 dated 2027 and 1 dated 2038
  (placeholder/long-range entries) — i.e. this index is a *rolling current-season* view plus advanced
  scheduling, not an archive.

### Enumeration

**A. Resolving the 379 members against Bound boys T&F (exact rule).**

Artifact: `tools/44-iowa/compute.py` (re-runnable), output `tools/44-iowa/out-resolution.csv` +
`out-resolution.json` (also copied to `research/midwest/evidence/gaps/44/`). The CSV carries, per
member: `how`, `bound_team_2025_26`, `host_or_note`, `xc_own_class_row`, `xc_coop_named_at`,
`boysXC_teams`, `girlsXC_teams`, `girlsTF_teams`. Three matchers are used because the three name
spaces drift differently (own class rows: raw tokens with the member contained in the row name, plus a
stop-word-stripped equality for denomination drift; sharing cells: contiguous-substring segments, never
the host's own name; Bound team names: raw-token containment).

Request pattern (all local, no network): read `ihsaa-members.csv` (379) and
`bound-ihsaa-boystrack-teams-2025-26.csv` (365); match on city-suffix-aware token sets with a
mascot bonus, greedy by score; then apply three explicit override sets (display-name variants, co-op
hosts, no-program) documented in the script.

| Channel | Count | Meaning |
|---|---|---|
| name-identical / mascot-resolved Bound team | **346** | school fields its own T&F team and Bound names it recognisably |
| display-name variant | **10** | team exists; Bound uses the merged/colloquial program name (Table A2) |
| co-op guest (competes under a host) | **18** | no standalone Bound team; athletes appear on the host's team (Table A1) |
| no T&F program (and no XC program) | **5** | no T&F/XC activity anywhere on the Bound school page |
| **total** | **379** | matches the member list exactly |

So the correct statement is **33 members lack a name-identical Bound T&F team = 10 + 18 + 5**, and
**23 members lack any standalone Bound T&F program = 18 + 5**. Report 11's "24" is an approximation of
the latter; its eight named examples resolve as four variants (H-L-V Victor → `HLV-TC Warriors`,
Kingsley-Pierson → `KP-WC-RV Panthers`, Ruthven-Ayrshire → `GTRA Titans`, Regina Iowa City → `Regina
Catholic Regals`), three co-op guests (Coram Deo Academy → Bettendorf, Des Moines Prep → Des Moines
Roosevelt, Scattergood Friends → West Branch) and one no-program school (Great River Christian).

**Cross-check (independent of Bound).** IHSAA's own 2026 T&F classification `Coops` column names 23
guest entries. 17 of my 18 co-op guests appear there verbatim; **Ottumwa Christian is the exception**
(its classification row is absent from the 351-row class list; its co-op status is proven instead by the
Bound school page, which shows `Varsity Track & Field | COOP | Host - Ottumwa`). The remaining six
`Coops` entries (Rock Valley, Ruthven-Ayrshire, Tri-County-Thornburg, Lone Tree, Woodbury Central
Moville, River Valley Correctionsville) are schools that **do** field teams and merely share for
postseason — so the `Coops` column is a *postseason-sharing* signal, **not** a proxy for "no team".
The authoritative no-team signal is the absence of a Bound team entry.

**B. The XC resolver.** The 2026 XC classification page carries a `Sharing Agreements` cell on 24 data
rows (19 host rows carry the T&F equivalent in the class list's `Coops` column); those cells name the
guest programs outright:

| Host (IHSAA XC class row) | Sharing Agreements cell |
|---|---|
| Des Moines, East (4A) | JW Reed Christian Academy |
| Ottumwa (4A) | Ottumwa Christian |
| Bettendorf (4A) | Morning Star Academy |
| Iowa City, City High (4A) | West Branch |
| Waterloo United (4A) | Waterloo West, Waterloo East |
| Mason City (3A) | North Iowa Christian |
| Storm Lake (3A) | St. Mary's, Storm Lake |
| Des Moines, Hoover (3A) | Empigo Academy |
| Marion (3A) | **Issac** Newton Christian Academy *(source misspelling)* |
| Mount Vernon-Lisbon (3A) | Lisbon |
| Greene County (2A) | Paton-Churdan |
| Mid-Prairie, Wellman (2A) | Pathway Christian |
| NDWB (2A) | Notre Dame-Burlington, West Burlington |
| South Central Calhoun (2A) | Glidden-Ralston |
| Boyden-Hull/Rock Valley (2A) | Boyden-Hull, Rock Valley |
| George-Little Rock/Central Lyon (2A) | George-Little Rock Central Lyon-Rock Rapids |
| Missouri Valley (1A) | West Harrison, Mondamin |
| WC-KP-RV (1A) | Woodbury Central-Moville, Kingsley-Pierson, River Valley-Correctionville |
| Starmont-West Central (1A) | Starmont, West Central, Maynard |
| West Monona-Whiting (1A) | Whiting |
| Springville (1A) | Cedar Valley Christian |
| GTRA (1A) | Graettinger-Terril, Ruthven-Ayrshire |
| HLV-TC (1A) | HLV-Victor, Tri-County-Thornburg |
| Ar-We-Va, Westside (1A) | Unity Ridge Lutheran School |

This single column is the cheapest deterministic T&F/XC co-op resolver in the state: it names guest →
host for XC, and the T&F `Coops` column does the same for track. Applied to all 379 members it yields
**31 XC co-op edges** (a superset of the 18 T&F guests — several schools field their own T&F team but
co-op for XC, e.g. Boyden-Hull/Rock Valley, George-Little Rock/Central Lyon, Starmont-West Central,
Waterloo United, HLV-TC, GTRA, WC-KP-RV). Caveat: spelling is not consistent
("Issac Newton", "Tri-County-Thornburg" vs member "Tri-County, Thornburg"), so joins must be
token-based, never string-equal.

**C. Timer census.** One request to the concrete index returns the whole Iowa universe:

```
POST https://search.athletic.live/live_results_meet_list/_search
{"query":{"term":{"lsa":"Iowa"}},"size":1000}
-> 200, hits.total.value = 772 (relation "eq"), 772 docs returned
```

All 772 docs carry `lsa="Iowa"`, `n` (name), `sdy` (start date), `tna` (timing company), `tu`
(timing URL), `i` (AthleticLIVE meet id), `ani` (Athletic.net MeetID — present on 765/772 = 99%),
`aci` (present on 0), `lo`/`ls` (location), `qe` (timer email).

**Routes that failed (recorded, not hidden):**

* `POST …/*_meet_list/_search` (union over all 256 tenant indices) with `{"term":{"lsa":"Iowa"}}`
  → **200 with zero hits**; with aggregations or a sort on `d`/`sd` → **400** and, on one attempt, a
  200 with empty aggregations. The union pattern is not usable: tenant indices disagree on mappings.
  Concrete index names (`live_results_meet_list`, `<machine>_meet_list`) are the working route.
* The `e` (tenant machine) field is **null on every doc in `live_results_meet_list`** — the generic
  index does not carry the white-label machine. Provider identity there comes from `tna`, not `e`.

**D. The 23 members, their host, and the alternate source for rosters/results.**

| # | IHSAA member (no standalone T&F team) | Why | T&F host (athlete attribution) | XC status | Alternate source for rosters/results |
|---|---|---|---|---|---|
| 1 | Cedar Valley Christian | co-op guest | Springville | **own** Bound XC teams (B+G) + named at Springville | Bound `Cedar Valley Christian Huskies` XC team pages; T&F rows under Springville in AthleticLIVE/Bound |
| 2 | Coram Deo Academy | co-op guest | Bettendorf | **own** XC class row (1A) + own boys XC team | Bound class row + own XC team page |
| 3 | Des Moines Prep | co-op guest | Des Moines, Roosevelt | **own** XC class row (1A) + own B+G XC teams | own Bound XC team pages |
| 4 | Holy Trinity Catholic, Fort Madison | co-op guest | Fort Madison | **own** XC class row (1A) + own B+G teams | own Bound XC team pages |
| 5 | Isaac Newton Christian Academy | co-op guest | Marion | co-op (named at Marion, source typo) + own B+G XC teams | Marion's XC team page; own XC team pages |
| 6 | JW Reed Christian Academy | co-op guest | Des Moines, East | co-op (named at Des Moines East) | DM East team pages |
| 7 | Maharishi, Fairfield | co-op guest | Fairfield | none found (no team, no class row, no sharing cell) | Fairfield T&F pages only |
| 8 | Morning Star Academy, Bettendorf | co-op guest | Bettendorf | co-op (named at Bettendorf) | Bettendorf team pages |
| 9 | Mount Pleasant Christian School | co-op guest | Mount Pleasant | own B+G XC teams (Bound) but no class row | own Bound XC team pages |
| 10 | New City Classical | co-op guest | Bettendorf | none found | Bettendorf team pages |
| 11 | North Iowa Christian | co-op guest | Mason City | co-op (named at Mason City) | Mason City team pages |
| 12 | Notre Dame, Burlington | co-op guest | West Burlington (NDWB) | co-op (named at `NDWB`) + own B+G `Burlington Notre Dame Nikes` XC teams | own Bound XC team pages; team results under NDWB |
| 13 | Ottumwa Christian | co-op guest (Bound page: COOP, host Ottumwa) | Ottumwa | co-op (named at Ottumwa) | Ottumwa team pages |
| 14 | Pathway Christian | co-op guest | Mid-Prairie, Wellman | co-op (named at Mid-Prairie) | Mid-Prairie team pages |
| 15 | Rivermont Collegiate, Bettendorf | co-op guest | Bettendorf | **own** XC class row (1A), no Bound team | IHSAA XC class row + meet entries |
| 16 | Scattergood Friends | co-op guest | West Branch | own `Scattergood Friends Scattergood Crew` team (Bound) | own Bound XC team page |
| 17 | Unity Ridge Lutheran | co-op guest | Ar-We-Va, Westside | co-op (named at Ar-We-Va) | Ar-We-Va team pages |
| 18 | West Harrison, Mondamin | co-op guest | Missouri Valley | co-op (named at Missouri Valley) | Missouri Valley team pages |
| 19 | Great River Christian | **no program** | — | none | none (Bound school page shows no T&F/XC; no IHSAA school page) |
| 20 | Southeast Christian Academy | **no program** | — | none | none (Bound school page lists no activities) |
| 21 | Strong Roots Christian | **no program** | — | none | none (Bound school page: girls Softball only) |
| 22 | The Lighthouse | **no program** | — | none | none (Bound school page: boys Basketball only) |
| 23 | Tri-State Christian School | **no program** | — | none | none (Bound school page lists no activities) |

Source URLs for rows 1-18: `https://www.gobound.com/ia/ihsaa/boystrack/2025-26/teams/<team_id>` for the
host (team ids in `tools/iowa-11/bound-ihsaa-boystrack-teams-2025-26.csv`) and
`https://www.gobound.com/ia/ihsaa/boyscrosscountry/2026-27/teams/<team_id>` for own XC programs
(`b-teams-boysxc-2026.csv`); meet results for every one of them are published by the AthleticLIVE tenant
whose `tna` timed the meet, keyed by the host school name. **Practical consequence:** no member school
needs an Athletic.net lookup to establish its T&F/XC identity — the two classification columns plus the
Bound team index already resolve all 379 to a host or a "does not field a team" verdict.

**The 10 display-name variants (these were never gaps; recorded so future joins do not re-open them).**

| IHSAA member | Bound team display name |
|---|---|
| B-G-M, Brooklyn | BGM Bears |
| Boyden-Hull | Boyden-Hull/Rock Valley Nighthawks |
| Graettinger-Terril | GTRA Titans |
| H-L-V, Victor | HLV-TC Warriors |
| Kingsley-Pierson | KP-WC-RV Panthers |
| LeMars | Le Mars Bulldogs |
| Manson Northwest Webster | Manson-NW Webster Cougars |
| Rock Valley | Boyden-Hull/Rock Valley Nighthawks |
| Ruthven-Ayrshire | GTRA Titans |
| Tri-County, Thornburg | HLV-TC Warriors |

### Stable identifiers

| Identifier | Where | Notes |
|---|---|---|
| IHSAA member school list (379 rows, last column = iahsaa.org slug) | `tools/iowa-11/ihsaa-members.csv` | no numeric id; nickname + colors + conference are the join aids |
| IHSAA class row (351 T&F / 337 XC) | classification pages | carries `TotalBedsCount` (BEDS) — a real enrolment-ish number per school, usable for size-based cohorts |
| Bound team id (`team_id`, opaque `h…` string) + `school_guid` | Bound team CSVs | stable per team-season; the only numeric-ish team key available outside Athletic.net |
| AthleticLIVE meet id `i` | meet docs | e.g. 6727, 73968, 76401 |
| **Athletic.net MeetID `ani`** | meet docs, 765/772 (99%) | 758 unique Athletic.net meet ids recoverable from one request |
| Athletic.net comp id `aci` | absent (0/772) | not published in this index |
| Co-op edge (guest → host) | classification `Coops` and `Sharing Agreements` columns | text, not ids; must be token-normalised (spelling drift) |

No athlete-level identifiers are added by this slice; report 12's ResultID/AthleteID/`ani` analysis
stands (AthleticLIVE result rows carry `r[].i`, `a.i`, `a.ani`, `t.i`, `t.ani`).

### Athletic.net leverage

* **758 unique Athletic.net MeetIDs from one anonymous request** (99% of the 2026 Iowa meet docs carry
  `ani`). The 2026 Iowa calendar = 768 docs; 630 spring-T&F + 138 XC. For pipeline seeding this is the
  cheapest Athletic.net meet-id recovery route found in the whole campaign.
* **The 18 co-op guests never need an Athletic.net *team* lookup of their own:** their athletes are
  attributed to the host team in every result feed, and the guest→host edge is available offline from
  the two classification columns. Only the *guest label* needs to be re-attached (see Athlete evidence).
* **The 5 program-less schools must be excluded from team enumeration** — otherwise any
  school-list-driven crawler spends Athletic.net requests on schools that have no T&F/XC team in any
  season. Estimated saving: 5 schools × (T&F + XC, boys + girls) lookups per season, plus the 10
  variant names that would otherwise cause duplicate-team lookups (15 schools ≈ 60 avoided
  team-resolution requests per season, and 15 fewer phantom teams in the roster graph).
* Timers: the 11 missing providers publish on AthleticLIVE, so their meet docs carry `ani` and their
  result docs carry `a.ani` — they are Athletic.net-seedable without hitting Athletic.net.

### Athlete evidence

* Co-op guests: `[INFERENCE]` the result feeds attribute the athlete to the **host** team (`t.i`/`t.ani`
  = host), because the classification model gives the guest no separate team row. Nothing in the
  AthleticLIVE result schema observed by report 12 carries a secondary "guest school" field, so
  re-attaching the guest school must be done from the classification edge (Table A1), keyed by
  (season, host team, guest name). Treat this as a known attribution gap to be tested on one real
  co-op meet before pipeline use.
* The 5 program-less schools contribute **no athlete rows** in T&F/XC for any season; their students
  cannot appear in this dataset as team members (unattached entries, if any, would not carry the
  school name).
* Names/grade/class evidence is unchanged from reports 11/12 (Bound rosters for team membership;
  AthleticLIVE result rows for performance; MileSplit profile pages for `Class of 2027`).
* No athlete personal contact data was collected.

### Recruiting information

**Official directories first (the assignment's ask):**

| Official source | What it publishes | Coach/AD email yield |
|---|---|---|
| IHSAA `/contact/` | staff table (16 IHSAA staff; emails published as `@iahsaa.org` local parts, e.g. `cburt`, `jchizek`), office hours, and the literal navigation item **"Find School Contacts at Bound"** + "High School Directory" / "Member School & Conference List" (both link out) | **0 school coach emails** — the association delegates school contacts to Bound |
| IHSAA `/coaches-administrators/` | resource centre (forms, rules, clinics) | 0 |
| IHSAA member-school pages (`/schools/<slug>/`) | school identity template (name, class, colours, conference) | 0 (verified on Johnston, Cedar Falls, Bettendorf, Southeast Polk) |
| IGHSAU `/staff` (200, 89,141 B) | executive staff bios (Executive Director etc.) | 0 school contacts |
| IGHSAU `/contact-us` (200, 60,490 B) | office address/phone + single mailbox `mail@ighsau.org` | 0 school contacts |
| IGHSAU sitemaps (`sitemap_index.xml`, `sitemap.xml`, `wp-sitemap.xml`) | **404** (6,603 B WordPress "Not Found" body) | n/a — no sitemap exists; `robots.txt` is `User-agent: * / Disallow:` (24 B, allow-all) |
| IATC `/membership-list/` (200) | **163 member schools** by class (1A 69, 2A 37, 3A 33, 4A 24) — schools only, no coach names | 0 coach emails (one membership-payment address on a personal domain; omitted per privacy contract) |
| IATC `/track-and-field-advisory-board/` (200) | "This list represents the track and field advisory board to the IHSAA and the IGHSAU": Cody Eichmeier (Chairperson, AD, Dike-New Hartford), Brad Travis (Spirit Lake), Kenny Wheeler (Pleasant Valley), Erica Douglas (Head coach, Indianola), Kyle Sandstrom (MFL MarMac), Rachel Larsen (Griswold), Jared Fletcher (Carlisle), Darin Arkema (Lynnville-Sully), Jim Nichols (Official, Storm Lake) | **9 named people + schools, 0 emails** — a name-level seed for 9 schools; a parallel XC advisory board page exists (report 11) |
| IATC sitemap (`wp-sitemap-posts-page-1.xml`, 65 URLs) | includes an unexplored `https://www.iatrackcoaches.org/members/` page (IATO officials members) | not fetched — flagged as a possible additional name source |

**School-site probes (11 districts, results as observed):**

| District site | Directory/emails found | Coach/AD-specific |
|---|---|---|
| Cedar Falls (`cfschools.org` staff directory, 1,028,567 B) | **878 published staff addresses** | "Athletics & Activities Director" (`justin.urbanek@cfschools.org`), "Associate Principal/Asst. Athletic & Activities Director" (`Megan.Youngkent@`), "Assistant Principal/Activities Director" (`whitney.fischer@`); 29 role labels containing "coach" (7 "Head Coach", 1 "Assistant Coach", 1 "Varsity Head Coach") — **no sport suffix in the returned slice** |
| Waukee (`waukeeschools.org/district/admin-directory/`) | **176 published addresses** | "Activities Director" (Northwest HS, `johl@`), "Activities Director" (Waukee HS, `eboyle@`), "Assistant Activities Director" (`ahauptmann@`), "Activities Secretary" (`wrichtsmeier@`) |
| Linn-Mar (`linnmar.k12.ia.us/district/directory/`) | 10 addresses (paged directory) | none coach/AD-labelled in the returned slice |
| Dubuque (`dbqschools.org/directory/`) | 0 addresses | 1 contact on `/staff/coaches/`: employee-access questions (AD-related, `jmaloney@dbqschools.org`) |
| Ames (`ameshighathletics.org/activities/athletics/boys-cross-country/`) | 1 address | **head boys XC coach mailto** ("Contact the Coach … Email Ravi Bhave") — the only true head-coach email found in this probe set |
| West Des Moines (`wdmcs.org/our-district/about/employee-directory`) | 0 emails in body | 1 general mailbox on the staff dashboard (`scroffice@wdmcs.org`) |
| Ankeny (`ankenyschools.org/district/directory/`) | **0** — raw HTML contains no `mailto` and no address literal | 0 |
| Southeast Polk (`southeastpolk.org/employee/`) | 0 emails (share link only) | 0 |
| Dowling Catholic (`dowlingcatholic.org/athletics`) | 0 emails | AD names + phone numbers published instead |
| Bettendorf | **JS client-challenge page (3,036 B "Client Challenge")** — no content | 0 (host classified browser-application/challenge) |
| Johnston | district site not retrieved in this pass (no artifact); the IHSAA school page for Johnston was retrieved (200) | unknown |

**Coverage counts (acceptance figure):** official association + coaches'-association sources produce
**0 school coach emails** and **9 named coach/AD people**; the 11 district probes produce **1 head-coach
email + 7 role-labelled AD/activities addresses** (Cedar Falls 2, Waukee 4, Dubuque 1) from six
directories that publish email at all. Sport-attributed head-coach emails are effectively not published
by Iowa districts in directory form; Bound (report 11) remains the only per-school staff carrier, and
team pages (Ames pattern) are the only coach-email surface. **Recruiting-side verdict: the official
directories do not close the coach gap — coach coverage beyond Bound is limited to AD/activities
contacts plus per-sport team pages, school by school.**

### Result evidence

AthleticLIVE field-level result evidence is unchanged from report 12 (ResultID `r[].i`, AthleteID
`a.i`, Athletic.net AthleteID `a.ani`, MeetID `mi` + `ani`, EventID doc id, mark `m`, normalised `im`
(ms for time events, µm for distance/throws), wind `r[].w`, `vm` validity flag; **no timing-method
field**).

What this slice adds:

* **Meet-level**: `sdy`/`sd`/`ed` dates, `n`/`ln`/`son` names, `lo`/`ls`/`lsa` location, `tna`/`tu`
  timing company, `qe` timer email, `o` (outdoor/indoor), `tz` timezone, and `ani` (Athletic.net meet
  id) on 99% of Iowa 2026 docs.
* **School-represented attribution for co-op guests is the host team** (see Athlete evidence) — the
  result row itself does not name the guest school.
* Timing method stays unavailable; where a meet was timed by a school programme rather than a company
  the `tna` value *is* the school (see the census below), which is itself a useful provenance signal.

**Timer census for calendar 2026 (Iowa, `tna` values, 768 docs):**

| Timer (`tna`) | Docs | In report 12's list? |
|---|---|---|
| All-American Timing (= "AA Timing", `aatiming` machine) | 222 | yes |
| *(empty / `None`)* | 150 | n/a — self-timed or unset |
| Wayzata Results, LLC (+ "Inc." 1) | 87 | yes |
| **Shannon Event Timings** (+ "Timing" 5, "TImings" 2, "Matthew Shannon" 1) | **66** | **no** |
| **Kauder Racing, LLC** (+ "Andrew Kauder" 2, "Scheckel - Kauder Racing, LLC" 2; a separate "Scheckel" label carries 1) | **46** | **no** |
| B & W Racing Services | 32 | yes (`bwracing`) |
| Black Squirrel Timing | 19 | yes |
| **Rapid Timing** (+ "Rapid Timing, LLC" 14) | **16** | **no** |
| **Race the Clock Timing** | **12** | **no** |
| Algona High School (school programme) | 10 | no |
| Dakota Timing | 8 | yes |
| Glenwood High School (+ "High school" 3) | 11 | no |
| Cedar Falls High School | 8 | no |
| **Hero's Timing** | **7** | **no** |
| Brett Carney (individual) | 7 | no |
| **Blue Machine Timing** (+ "Company" 5) | **7** | **no** |
| Tipton High School | 6 | no |
| **Midwest Timing LLC** | **5** | machine named in report 12, absent from its Iowa table |
| Audubon Community Schools | 5 | no |
| Lewis Central / Decorah High School | 4 / 4 | no |
| Lisbon High School / Jared Pickett / **Meade Timing, LLC** / South O'Brien / **WestWood Consulting** | 3 each | Meade + WestWood: **no** |
| **TC Solutions** / Geoff Kruse / T. Coleman / Spencer CSD / Shenandoah HS | 2 each | TC Solutions: **no** |
| **Buffalo Ridge Timing**, **Flash Results**, Iowa City High, Nevada CSD, Hy-Tek Active Network LLC, Underwood High School | 1 each | Buffalo Ridge + Flash Results: **no** |

**Delta answer:** report 12's Iowa timer table is missing **Shannon Event Timings (66 meet docs)**,
**Kauder Racing (46)**, **Rapid Timing (16)**, **Race the Clock (12)**, **Hero's Timing (7)**, **Blue
Machine (7)**, **Meade (3)**, **WestWood (3)**, **TC Solutions (2)**, **Buffalo Ridge (1)** and **Flash
Results (1)** — i.e. 11 companies, **164 of the 768 2026 Iowa meet docs (21%)**, plus 17 school-run
timer identities (63 docs; e.g. Algona HS 10, Glenwood HS 11, Cedar Falls HS 8, Tipton HS 6, Audubon 5 (+1 typo variant),
Lewis Central 4, Decorah HS 4). Report 12 discovered several of these machine names in the 258-machine roster but
did not connect them to Iowa meets; the connection is `tna` on `live_results_meet_list`.

Non-AthleticLIVE providers from report 12 (True Time Racing, Results CM, Cross Country Ratings) **do not
appear in this index at all** (0 docs) — they publish on their own stacks, so their absence is expected,
and the IATC weekly index stays their only discovery route.

**Weekly-index completeness check (`iatrackcoaches.org/results/`, captured 2026-09-19):** the page links
9 result destinations (4 `resultscm.com`, 4 `dakotatiming`, 3 `aatiming`, 1 `bwracingservices`, 1
`blacksquirreltiming`, 9 milesplit meet pages, 2 `crosscountryratings`) and hosts coach-submitted PDFs
for recent meets (e.g. Starmont 9-15-2026, Emmetsburg 9-15-2026, Oskaloosa 9-15-2026, Cedar Falls
Rich Engle 9-15-2026, and `Cascade 9-17-2026 – CANCELLED` as a MileSplit link). The same week's Iowa
meets timed by Shannon/Kauder/Hero's appear in the AthleticLIVE index but **not on the IATC page with a
provider attribution** — so the IATC index is not sufficient for timer discovery; the ES index is.

**Host confirmation (re-verified this pass, 2026-09-20 09:1x):**

| Host | Result |
|---|---|
| `https://shannoneventtimings.anet.live/` | 200, 50,184 B (AthleticLIVE SPA shell) |
| `https://live.kauderresults.com/` | 200, 50,184 B (same shell) |
| `https://live.rapidresultstiming.com/` | 200, 50,184 B (same shell — machine `rapidresultstiming`, **no Iowa meet doc carries this name**; distinct from `rapidtiming`) |
| `https://aatiming.com/` | 200, 50,184 B (same shell) |
| Company sites: `https://www.kauderresults.com` (676,024 B), `https://www.rapidtiming.net` (453,016 B), Shannon company site (30,905 B, title "Shannon Event Timings - Home") | 200 — real timing businesses, not empty white-labels |

### Incremental use

* **Weekly timer/meet discovery:** one request (`term lsa:Iowa` on `live_results_meet_list`) yields the
  full Iowa meet list with `sdy`, `tna`, `ani`; diff on (i, `sdy`, `tna`, `n`) for new/changed meets.
  No historical re-fetch is needed because the index is current-season rolling; keep a local
  `(i, ani, sdy, tna)` table and add only new ids. Report 12's change-detection design (blob
  `ETag`/`Last-Modified` + HTTP 304 on `$web/ind_res_list/_doc/<EventID>`) remains the per-event
  incremental route.
* **New results / affected athletes:** per meet, the site's own XHR pattern
  (`{"size":500,…,"term":{"md":<date>}}`-style queries observed in report 12) or the blob document for
  the event; athlete-level deltas are new `r[].i` values per event doc.
* **Co-op edge refresh:** annually, re-read the two classification columns (T&F `Coops`, XC `Sharing
  Agreements`) — the guest list changes with mergers/co-ops (e.g. Rock Valley appeared only as a
  sharing entry in 2026). Store guest→host as a season-scoped edge table; do not hard-code.
* **Program-less schools:** keep the 5-school exclusion list season-scoped; verify by the absence of a
  Bound team entry rather than by a stored name, and re-check each August.
* **Coaches:** no incremental route beyond Bound team pages (per report 11) and per-site staff
  directories; district directory pages are paginated, so a weekly diff would require repeated paged
  fetches — not recommended. The Ames-style team page pattern ("Contact the Coach" mailto) is the only
  cheap coach-email signal, and it is per-sport, not statewide.

### Access characteristics

* All sources used here are **normal HTML / public JSON**: IHSAA and IGHSAU are WordPress (public
  pages), Bound is server-rendered public HTML, IATC is WordPress, district directories are public
  HTML (some paginated), and AthleticLIVE search answers **anonymous POST** queries with JSON.
* Classification: AthleticLIVE ES = *public structured JSON*; IHSAA/IGHSAU/IATC = *normal HTML*;
  Bound = *normal HTML*; district directories = *normal HTML*, one host (Bettendorf) = *browser
  application/challenge* (JS interstitial, unusable without a browser).
* Observed failures: IHSAA school page for Great River Christian = **404**; Ames `track-field` slug =
  **404** (57,114 B "Page not found"); IGHSAU sitemap paths = **404** (6,603 B WordPress 404 body);
  `iatc-sitemap.xml` = **0 B** (empty); the ES union pattern = 0 hits / 400 responses (mapping
  conflict); Bettendorf = JS challenge.
* **No 403/429/CAPTCHA was encountered on any host in this slice, and no `Retry-After` was observed.**
  Politeness: 1-3 s between requests to the same host, sequential, ≤ ~80 requests total for this
  follow-up. No auth, cookies, or paywall involved; no request was sent to any `*.athletic.net` host.

### Recommendation

**PRIMARY (validation-grade) for the co-op/no-team rule; RESULT-SOURCE for the timer census;
COACH-DIRECTORY: REJECT for official directories, DISCOVERY-ONLY for district sites.**

* Adopt the classification-derived resolver (T&F `Coops` + XC `Sharing Agreements` + Bound team index)
  as the deterministic Iowa school→team map. It resolves all 379 members, explains the 33/23 count
  exactly, and prevents phantom-team Athletic.net lookups.
* Adopt `live_results_meet_list` (`term lsa:Iowa`) as the weekly Iowa meet/provider index: 1 request,
  99% `ani` coverage, complete `tna` provenance including the 11 timers report 12 missed. Keep the IATC
  page only as a human cross-check for non-AthleticLIVE providers (Results CM, CC Ratings, True Time).
* Do **not** invest in the association/coaches'-association directories as coach sources: they publish
  no school-level coach contacts (IHSAA explicitly redirects to Bound). District staff directories are
  worth a one-off sweep for AD/activities emails (Cedar Falls and Waukee publish them outright), and the
  Ames-style per-sport team page is the only repeatable head-coach email pattern found.

### Evidence appendix

Statuses marked `¹` were printed by the fetch wrapper at capture time (the same run that wrote the
retained body in `tools/44-iowa/`); each such body was re-opened during analysis and matches the
expected document (title/content check), so those captures are content-verified. Statuses without `¹`
were observed directly during the final verification pass. Timestamps are the retained-body mtimes
(America/Chicago).

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.iahsaa.org/classifications/cross-country/` | GET | 200¹ | IHSAA XC class list, 337 rows, **Sharing Agreements** column = the state's co-op resolver | 2026-09-20 09:07:57 |
| `POST https://search.athletic.live/live_results_meet_list/_search` `{"query":{"term":{"lsa":"Iowa"}},"size":1000}` | POST | 200 | 772 Iowa meet docs (total.value 772, relation eq); 765 carry `ani` (99%), 0 carry `aci` | 2026-09-20 09:14:29 |
| `POST https://search.athletic.live/live_results_meet_list/_search` (count form) | POST | 200 | count-only confirmation of the same 772 figure | 2026-09-20 09:14:19 |
| `POST https://search.athletic.live/*_meet_list/_search` `{"term":{"lsa":"Iowa"}}` (union pattern) | POST | **200 with 0 hits** | union across the 256 tenant indices is unusable for `lsa` filters (mapping conflict) | 2026-09-20 09:16 |
| `POST https://search.athletic.live/*_meet_list/_search` (aggs by machine/tna/date_histogram) | POST | **400** | union-pattern aggregation rejected; concrete index required | 2026-09-20 09:16 |
| `POST https://search.athletic.live/wayzata_meet_list/_search` (`{"size":1}` and state agg) | POST | 200 / 400 | meet-doc schema (`i,ani,aci,tna,tu,qe,lo,ls,lsa,sdy,sd,ed,o,tz`); sort on `d` rejected | 2026-09-20 09:16-09:17 |
| `https://www.gobound.com/ia/schools/ottumwachristian` | GET | 200¹ (220,979 B) | activity list shows `Varsity Track & Field` (COOP, "Host - Ottumwa") → Ottumwa Christian = 18th co-op guest | 2026-09-20 09:08:34 |
| `https://www.gobound.com/ia/schools/srcs` | GET | 200¹ (57,939 B) | Strong Roots Christian: girls Softball only → no T&F/XC program | 2026-09-20 09:08:45 |
| `https://www.gobound.com/ia/schools/tristatechristian` | GET | 200¹ (59,721 B) | Tri-State Christian: no activities listed → no T&F/XC program | 2026-09-20 09:08:56 |
| `https://www.gobound.com/ia/schools/grcschool` | GET | 200¹ (53,736 B) | Great River Christian: no activities listed | 2026-09-20 09:09:08 |
| `https://www.gobound.com/ia/schools/schristian` | GET | 200¹ (54,488 B) | Southeastern Christian Stallions: no activities listed | 2026-09-20 09:09:19 |
| `https://www.gobound.com/ia/schools/tlhschools` | GET | 200¹ (57,225 B) | The Lighthouse Schools: boys Basketball only | 2026-09-20 09:09:31 |
| `https://www.iahsaa.org/schools/ottumwa-christian/` (page title "Ottumwa Christian") | GET | 200¹ (196,988 B) | member school page exists, carries no T&F team data | 2026-09-20 09:08:01 |
| `https://www.iahsaa.org/schools/great-river-christian/` | GET | **404¹** (171,156 B "404 - Page not found") | no IHSAA school page for Great River Christian | 2026-09-20 09:08:05 |
| `https://www.iahsaa.org/contact/` | GET | 200¹ (185,622 B) | IHSAA staff table + navigation item **"Find School Contacts at Bound"** → no school coach contacts | 2026-09-20 09:10:18 |
| `https://www.iahsaa.org/coaches-administrators/` | GET | 200¹ (205,469 B) | resource centre only; no coach directory | 2026-09-20 09:10:15 |
| `https://www.iahsaa.org/schools/{johnston,cedar-falls,bettendorf,southeast-polk}/` | GET | 200¹ (205-207 KB each) | member-school template: identity only, zero contacts | 2026-09-20 09:11:44-09:11:55 |
| `https://ighsau.org/` | GET | 200 (80,513 B, re-verified) | home nav → sport pages + "Bound Login"; single mailbox `mail@ighsau.org` | 2026-09-20 09:15:20 |
| `https://ighsau.org/staff` | GET | 200 (89,141 B, re-verified) | executive staff bios only; no school coach directory | 2026-09-20 09:15:50 |
| `https://ighsau.org/contact-us` | GET | 200 (60,490 B, re-verified) | office address/phone + single mailbox | 2026-09-20 09:15:51 |
| `https://ighsau.org/sitemap_index.xml`, `/sitemap.xml`, `/wp-sitemap.xml` | GET | **404** (6,603 B WordPress "Not Found") | IGHSAU publishes no sitemap | 2026-09-20 09:10:20-09:10:49 |
| `https://ighsau.org/robots.txt` | GET | 200 (24 B: `User-agent: *` / `Disallow:`) | allow-all crawl policy | 2026-09-20 09:10:37 |
| `https://ighsau.org/contact/` | GET | **404** (6,603 B "Not Found") | the `/contact/` path does not exist (only `/contact-us`) | 2026-09-20 09:15:00 |
| `https://www.iatrackcoaches.org/` | GET | 200¹ (154,509 B) | IATC site structure (membership list, advisory boards, weekly results) | 2026-09-20 09:10:23 |
| `https://www.iatrackcoaches.org/membership-list/` | GET | 200¹ (154,760 B) | **163 member schools** by class (69/37/33/24); zero coach names or sport emails | 2026-09-20 09:10:57 |
| `https://www.iatrackcoaches.org/track-and-field-advisory-board/` | GET | 200¹ (129,649 B) | 9 named ADs/coaches with schools, explicitly "advisory board to the IHSAA and the IGHSAU"; no emails | 2026-09-20 09:15:02 |
| `https://www.iatrackcoaches.org/wp-sitemap-posts-page-1.xml` | GET | 200¹ (63,310 B, 65 URLs) | lists `/members/` (officials) — possible additional name source, not fetched | 2026-09-20 09:10:47 |
| `https://www.iatrackcoaches.org/sitemap.xml` | GET | **0 B response**¹ | failed fetch recorded | 2026-09-20 09:10:35 |
| `https://www.iatrackcoaches.org/sitemap_index.xml` (followed) | GET | 200¹ (1,175 B XML sitemap index) | the working sitemap for the site | 2026-09-20 09:10:41 |
| `https://www.cfschools.org/staff-directory/` (canonical; title "Cedar Falls Community School District") | GET | 200¹ (1,028,567 B) | **878 published staff emails**, incl. 3 role-labelled Athletics/Activities Director entries; 29 "coach" role labels, no sport suffix in slice | 2026-09-20 09:13:27 |
| `https://www.waukeeschools.org/district/admin-directory/` | GET | 200¹ (128,485 B) | **176 published addresses**, incl. 2 Activities Directors + Assistant Activities Director + Activities Secretary | 2026-09-20 09:13:30 |
| `https://www.linnmar.k12.ia.us/district/directory/` | GET | 200¹ (109,349 B) | 10 published addresses in the returned slice; none coach/AD-labelled | 2026-09-20 09:13:32 |
| `https://www.dbqschools.org/directory/` | GET | 200¹ (74,956 B) | 0 email addresses in body | 2026-09-20 09:13:33 |
| `https://www.dbqschools.org/staff/coaches/` | GET | 200¹ (71,219 B) | single district contact for employee-access/coaches questions (`jmaloney@dbqschools.org`) | 2026-09-20 09:12:07 |
| `https://www.ankenyschools.org/district/directory/` | GET | 200¹ (71,928 B) | raw HTML contains **no** `mailto` and no address literal → 0 coach contacts | 2026-09-20 09:14:53 |
| `https://www.southeastpolk.org/employee/` | GET | 200¹ (135,492 B) | directory page exposes no addresses (share link only) | 2026-09-20 09:13:29 |
| `https://www.wdmcs.org/our-district/about/employee-directory` | GET | 200¹ (84,634 B) | 0 emails in body | 2026-09-20 09:13:25 |
| `https://www.wdmcs.org/faculty-staff` (Employee Dashboard) | GET | 200¹ (85,467 B) | 1 general mailbox (`scroffice@wdmcs.org`) | 2026-09-20 09:12:15 |
| `https://ameshighathletics.org/activities/athletics/boys-cross-country/` | GET | 200¹ (65,319 B) | **head boys XC coach mailto** ("Contact the Coach … Email Ravi Bhave") | 2026-09-20 09:13:21 |
| `https://ameshighathletics.org/activities/athletics/track-field/` | GET | **404** (57,121 B, re-verified) | Ames has no T&F page at that slug | 2026-09-20 09:15:53 |
| `https://ameshighathletics.org/` | GET | 200¹ (85,662 B) | Ames athletics home (per-sport team pages) | 2026-09-20 09:12:06 |
| `https://amescsd.org/` (district home) | GET | 200¹ (215,149 B) | district landing page | 2026-09-20 09:11:11 |
| `https://www.dowlingcatholic.org/athletics` | GET | 200¹ (79,623 B) | AD names + phone numbers, no emails | 2026-09-20 09:12:08 |
| `https://www.linnmar.k12.ia.us/school/linn-mar/athletics/` | GET | 200¹ (106,632 B) | athletics page points at the district directory | 2026-09-20 09:12:11 |
| `https://www.waukeeschools.org/experiences/athletics/` | GET | 200¹ (70,904 B) | athletics page: pointers only | 2026-09-20 09:12:12 |
| `https://www.bettendorf.k12.ia.us/` | GET | **JS client-challenge** (3,036 B "Client Challenge") | no content; classified browser-application/challenge | 2026-09-20 09:12:59 |
| `https://www.southeastpolk.org/` | GET | 200¹ (278,476 B) | district home reachable | 2026-09-20 09:13:00 |
| `https://www.cfschools.org/` | GET | 200¹ (32,233 B) | district home (directory entry point) | 2026-09-20 09:12:57 |
| district home probes: `ankenyschools.org`, `waukeeschools.org`, `wdmcs.org`, `dowlingcatholic.org`, `dbqschools.org`, `linnmar.k12.ia.us` | GET | 200¹ (81-124 KB) | all reachable; `dowlingcatholic.org`, `dbqschools.org`, `waukeeschools.org` 301-redirect from their alternate hosts (162-278 B redirect stubs recorded); `ankeny/k12.ia.us/wdmcs` alternate hostname probes returned 0-byte bodies | 2026-09-20 09:11:30-09:11:39 |
| `https://shannoneventtimings.anet.live/` | GET | 200 (50,184 B, re-verified) | Shannon Event Timings = AthleticLIVE white-label | 2026-09-20 09:16 (re-verify) |
| `https://live.kauderresults.com/` | GET | 200 (50,184 B, re-verified) | Kauder Racing = AthleticLIVE white-label | 2026-09-20 09:16 (re-verify) |
| `https://live.rapidresultstiming.com/` | GET | 200 (50,184 B, re-verified) | machine `rapidresultstiming` is a live instance but **no Iowa meet doc carries this name** (distinct from `rapidtiming`) | 2026-09-20 09:16 (re-verify) |
| `https://aatiming.com/` | GET | 200 (50,184 B, re-verified) | "All-American Timing" (dominant Iowa timer, 222 docs) = AthleticLIVE white-label | 2026-09-20 09:16 (re-verify) |
| `https://www.kauderresults.com` (company site) | GET | 200¹ (676,024 B) | real timing business (canonical of the retained Kauder page) | 2026-09-20 09:14:54 |
| `https://www.rapidtiming.net` (company site) | GET | 200¹ (453,016 B) | real timing business ("Rapid Timing, LLC"/"Rapid Timing" = 16 Iowa docs) | 2026-09-20 09:14:58 |
| Shannon company site (title "Shannon Event Timings - Home") | GET | 200¹ (30,905 B) | real timing business (not a bare white-label); exact host not re-recorded, body retained as `x-shannon.html` | 2026-09-20 09:14:56 |
| local derivations: `tools/44-iowa/compute.py` → `out-resolution.csv/.json`, `out-providers.json` | local | n/a | 379-member resolution (346/10/18/5) and the 2026 provider census | 2026-09-20 09:16 |
