# 34. MileSplit team-roster enumeration vs the delivered Athletic.net corpus — acceptance-question pilot (C2/C3)

Status: complete — the pilot ran end-to-end (live MileSplit sample + offline corpus join), with the
limits stated rather than papered over: (a) the join is against the **delivered corpus snapshot**, not
live Athletic.net (athletic.net is off-limits for this assignment); (b) the corpus covers **US boys
Grade 11 only**, so the female half of the sample cannot be validated by it at all; (c) the sample is a
**systematic every-23rd-team draw** from the state team index, not a random or size-weighted sample;
(d) WI/MN are the only two states measured — the other ten state subdomains are untested here.
Observed on: 2026-09-20

---

### Source

| Item | Value |
|---|---|
| Provider | MileSplit (FloSports) — Wisconsin `https://wi.milesplit.com/`, Minnesota `https://mn.milesplit.com/` |
| Surfaces used | `GET /teams?type=1` (HS team frame) and `GET /teams/<TeamID>-<slug>/roster` (Class column, gender, Indoor/Outdoor/XC membership) |
| Compliance | both paths present in the live `robots.txt` (200, 173 B) and **not** disallowed; `Disallow: /rankings`, `/virtual-meets`, `/api/`, `/contact` untouched. Browser UA, strictly sequential, ≥1.2 s per host, 54 requests total |
| Corpus joined against | `Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx` — the delivered Athletic.net boys Grade-11 workbook (10,147,760 B; sheet `All Athletes`; creator metadata `Retained rankings export`; header `AthleteID, Names, GradeID, Teams, States, Events, Result Count, Other Observed Grades, Identity Match Status`). Read-only; **no network involved in the join** |

### Coverage

- Sample frame: WI HS team index = **597** teams; MN = **592** teams (single A–Y page each, `type=1`
  pre-selected). Systematic draw of every 23rd entry → **25 teams per state, 50 total**.
- Roster pages returned **50/50 HTTP 200** and are server-rendered: **4,883 rostered athletes** parsed
  (WI 3,112 / MN 1,771) spanning **every grade**, of which **1,029 are Class of 2027** (579 M / 450 F).
- Class field completeness on the sampled rosters: grad year is present for **95.9 %** of the 4,883
  rows (`0`/unknown = 198 rows); gender is present for 4,882 (1 blank). Distribution:
  2027 **1,029** · 2028 1,067 · 2029 922 · 2030 746 · 2031 519 · 2032 273 · 2033 112 · 2034 15 ·
  2035 2 · unknown 198. So Class-of-2027 is **21.1 %** of all rostered athletes, and the long tail below
  2030 confirms rosters include middle-school entries (per-peer observation) — filter on the *athlete's*
  class, never on the team.
- Roster emptiness is real, not a parse failure: **10 of 50** sampled teams served an empty roster
  (WI 2: Pembine 40599, Rock River Charter School 14270; MN 8: Academy for Sciences & Agriculture,
  Butterfield-Odin, Covenant Life, Duluth Cathedral, Hill City, RiverTree, Taylors Falls, Vail Home
  School). These are small/private/home schools; 38/50 teams contributed ≥1 Class-of-2027 athlete.
- Cohort alignment: today is inside 2026-27, so Class of 2027 = current Grade 12; the corpus is
  **2025-26 boys Grade 11**, i.e. the same cohort (peer reports 27/30 established this mapping). The
  comparison is cohort-consistent; the "Grade 11" equivalence is *not* valid for 2026-27 meets.

### Enumeration

Exact recipe used (byte-identical on both subdomains):

```
1. GET https://{wi|mn}.milesplit.com/teams?type=1        # 1 request -> every HS team (id, label, city)
2. GET https://{wi|mn}.milesplit.com/teams/{id}-{slug}/roster   # 1 request -> every rostered athlete
```

Each roster row is `<li class="athlete-row data-row">` with an athlete link
(`/athletes/{athleteId}-{slug}`), `column-gender`, `column-grad-year`, and three season cells carrying
`data-season-id` (`0` = not on that roster, else `1` Indoor / `2` Outdoor / `3` XC, per the page's own
`#rosterFilterType` options `{"": "All Rosters", "1": "Indoor", "2": "Outdoor", "3": "XC"}`).
Column header row, verbatim: `Athlete, Gender, Class, Indoor, Outdoor, XC`. Two independent signals —
positional flag cells and `data-season-id` — **agree on all 4,883 rows (0 disagreements)**.

Sample manifest (all 50 roster URLs, statuses, per-team Class-of-2027 counts):

| # | state | teamId | MileSplit team label | city (from /teams) | HTTP | roster athletes | Co2027 M | Co2027 F | roster URL |
|---|---|---|---|---|---|---|---|---|---|
| 1 | WI | 52649 | Abbotsford | ABBOTSFORD, WI, USA | 200 | 59 | 12 | 15 | `https://wi.milesplit.com/teams/52649-abbotsford/roster` |
| 2 | WI | 14044 | Ashland | Ashland, WI, USA | 200 | 80 | 13 | 16 | `https://wi.milesplit.com/teams/14044-ashland/roster` |
| 3 | WI | 14061 | Birchwood | Birchwood, WI, USA | 200 | 15 | 4 | 0 | `https://wi.milesplit.com/teams/14061-birchwood/roster` |
| 4 | WI | 14073 | Cameron | Cameron, WI, USA | 200 | 189 | 7 | 14 | `https://wi.milesplit.com/teams/14073-cameron/roster` |
| 5 | WI | 21422 | Cochrane-Fountain City | Fountain City, WI, USA | 200 | 77 | 4 | 4 | `https://wi.milesplit.com/teams/21422-cochrane-fountain-city/roster` |
| 6 | WI | 20399 | De Pere | De Pere, WI, USA | 200 | 376 | 37 | 43 | `https://wi.milesplit.com/teams/20399-de-pere/roster` |
| 7 | WI | 14111 | Elk Mound | Elk Mound, WI, USA | 200 | 211 | 10 | 12 | `https://wi.milesplit.com/teams/14111-elk-mound/roster` |
| 8 | WI | 14120 | Gale-Ettrick-Trempealeau | Galesville, WI, USA | 200 | 191 | 37 | 21 | `https://wi.milesplit.com/teams/14120-gale-ettrick-trempealeau/roster` |
| 9 | WI | 14127 | Greenfield (WI) | Greenfield, WI, USA | 200 | 140 | 33 | 14 | `https://wi.milesplit.com/teams/14127-greenfield-wi/roster` |
| 10 | WI | 14144 | Hustisford | Hustisford, WI, USA | 200 | 8 | 2 | 2 | `https://wi.milesplit.com/teams/14144-hustisford/roster` |
| 11 | WI | 25903 | Kettle Moraine Lutheran | Jackson, WI, USA | 200 | 133 | 24 | 30 | `https://wi.milesplit.com/teams/25903-kettle-moraine-lutheran/roster` |
| 12 | WI | 22767 | Laona/Wabeno | Laona, WI, USA | 200 | 64 | 6 | 6 | `https://wi.milesplit.com/teams/22767-laonawabeno/roster` |
| 13 | WI | 14170 | Manawa | Manawa, WI, USA | 200 | 93 | 12 | 14 | `https://wi.milesplit.com/teams/14170-manawa/roster` |
| 14 | WI | 14189 | Menomonie | Menomonie, WI, USA | 200 | 351 | 29 | 18 | `https://wi.milesplit.com/teams/14189-menomonie/roster` |
| 15 | WI | 30245 | Milwaukee Pulaski | Milwaukee, WI, USA | 200 | 59 | 14 | 14 | `https://wi.milesplit.com/teams/30245-milwaukee-pulaski/roster` |
| 16 | WI | 26846 | New Auburn | New Auburn, WI, USA | 200 | 73 | 7 | 8 | `https://wi.milesplit.com/teams/26846-new-auburn/roster` |
| 17 | WI | 14228 | Oconomowoc | Oconomowoc, WI, USA | 200 | 329 | 44 | 25 | `https://wi.milesplit.com/teams/14228-oconomowoc/roster` |
| 18 | WI | 40599 | Pembine | Pembine, WI, USA | 200 | 0 | 0 | 0 | `https://wi.milesplit.com/teams/40599-pembine/roster` |
| 19 | WI | 14074 | Racine Case | Racine, WI, USA | 200 | 154 | 35 | 19 | `https://wi.milesplit.com/teams/14074-racine-case/roster` |
| 20 | WI | 14270 | Rock River Charter School | Janesville, WI, USA | 200 | 0 | 0 | 0 | `https://wi.milesplit.com/teams/14270-rock-river-charter-school/roster` |
| 21 | WI | 52909 | Shell Lake | Shell Lake, WI, USA | 200 | 95 | 13 | 15 | `https://wi.milesplit.com/teams/52909-shell-lake/roster` |
| 22 | WI | 43859 | St. Augustine Preparatory Academy | Milwaukee, WI, USA | 200 | 109 | 23 | 8 | `https://wi.milesplit.com/teams/43859-st-augustine-preparatory-academy/roster` |
| 23 | WI | 14295 | Suring | Suring, WI, USA | 200 | 72 | 14 | 7 | `https://wi.milesplit.com/teams/14295-suring/roster` |
| 24 | WI | 14307 | Valders | Valders, WI, USA | 200 | 140 | 8 | 8 | `https://wi.milesplit.com/teams/14307-valders/roster` |
| 25 | WI | 14320 | Wautoma | Wautoma, WI, USA | 200 | 94 | 15 | 12 | `https://wi.milesplit.com/teams/14320-wautoma/roster` |
| 26 | MN | 12984 | Academy for Sciences & Agriculture | Vadnais Heights, MN, USA | 200 | 0 | 0 | 0 | `https://mn.milesplit.com/teams/12984-academy-for-sciences-and-agriculture/roster` |
| 27 | MN | 12997 | Ashby High School | Ashby, MN, USA | 200 | 84 | 4 | 6 | `https://mn.milesplit.com/teams/12997-ashby-high-school/roster` |
| 28 | MN | 13019 | Bigfork High School | Bigfork, MN, USA | 200 | 35 | 4 | 5 | `https://mn.milesplit.com/teams/13019-bigfork-high-school/roster` |
| 29 | MN | 13037 | Butterfield-Odin High School | Butterfield, MN, USA | 200 | 0 | 0 | 0 | `https://mn.milesplit.com/teams/13037-butterfield-odin-high-school/roster` |
| 30 | MN | 13056 | Chisholm High School | Chisholm, MN, USA | 200 | 98 | 14 | 13 | `https://mn.milesplit.com/teams/13056-chisholm-high-school/roster` |
| 31 | MN | 13076 | Covenant Life School | Wells, MN, USA | 200 | 0 | 0 | 0 | `https://mn.milesplit.com/teams/13076-covenant-life-school/roster` |
| 32 | MN | 61841 | Duluth Cathedral High School | Duluth, MN, USA | 200 | 0 | 0 | 0 | `https://mn.milesplit.com/teams/61841-duluth-cathedral-high-school/roster` |
| 33 | MN | 13109 | Elk River High School | Elk River, MN, USA | 200 | 303 | 22 | 16 | `https://mn.milesplit.com/teams/13109-elk-river-high-school/roster` |
| 34 | MN | 13129 | Fond du Lac Ojibwe High School | Cloquet, MN, USA | 200 | 7 | 0 | 0 | `https://mn.milesplit.com/teams/13129-fond-du-lac-ojibwe-high-school/roster` |
| 35 | MN | 13145 | Grand Rapids High School | Grand Rapids, MN, USA | 200 | 207 | 31 | 16 | `https://mn.milesplit.com/teams/13145-grand-rapids-high-school/roster` |
| 36 | MN | 13163 | Hill City High School | Hill City, MN, USA | 200 | 0 | 0 | 0 | `https://mn.milesplit.com/teams/13163-hill-city-high-school/roster` |
| 37 | MN | 13184 | Jackson County Central High School | Jackson, MN, USA | 200 | 79 | 8 | 9 | `https://mn.milesplit.com/teams/13184-jackson-county-central-high-school/roster` |
| 38 | MN | 13203 | Lakeview High School | Cottonwood, MN, USA | 200 | 74 | 8 | 5 | `https://mn.milesplit.com/teams/13203-lakeview-high-school/roster` |
| 39 | MN | 41733 | Lyle Public School | Lyle, MN, USA | 200 | 31 | 0 | 0 | `https://mn.milesplit.com/teams/41733-lyle-public-school/roster` |
| 40 | MN | 13241 | McGregor High School | McGregor, MN, USA | 200 | 50 | 2 | 5 | `https://mn.milesplit.com/teams/13241-mcgregor-high-school/roster` |
| 41 | MN | 13261 | Minnesota State Academy for the Deaf | Faribault, MN, USA | 200 | 25 | 1 | 2 | `https://mn.milesplit.com/teams/13261-minnesota-state-academy-for-the-deaf/roster` |
| 42 | MN | 13282 | New Life Academy of Woodbury | Saint Paul, MN, USA | 200 | 84 | 3 | 2 | `https://mn.milesplit.com/teams/13282-new-life-academy-of-woodbury/roster` |
| 43 | MN | 21432 | Onamia High School | Onamia, MN, USA | 200 | 31 | 5 | 4 | `https://mn.milesplit.com/teams/21432-onamia-high-school/roster` |
| 44 | MN | 13321 | Pine City High School | Pine City, MN, USA | 200 | 117 | 14 | 13 | `https://mn.milesplit.com/teams/13321-pine-city-high-school/roster` |
| 45 | MN | 64886 | RiverTree School | Crystal, MN, USA | 200 | 0 | 0 | 0 | `https://mn.milesplit.com/teams/64886-rivertree-school/roster` |
| 46 | MN | 13358 | Rushford-Peterson High School | Rushford, MN, USA | 200 | 84 | 10 | 9 | `https://mn.milesplit.com/teams/13358-rushford-peterson-high-school/roster` |
| 47 | MN | 13379 | Saint Paul Central High School | St. Paul, MN, USA | 200 | 424 | 46 | 16 | `https://mn.milesplit.com/teams/13379-saint-paul-central-high-school/roster` |
| 48 | MN | 13396 | Silver Bay (William Kelley) High School | Silver Bay, MN, USA | 200 | 38 | 4 | 4 | `https://mn.milesplit.com/teams/13396-silver-bay-william-kelley-high-school/roster` |
| 49 | MN | 61848 | Taylors Falls High School | Taylors Falls, MN, USA | 200 | 0 | 0 | 0 | `https://mn.milesplit.com/teams/61848-taylors-falls-high-school/roster` |
| 50 | MN | 75555 | Vail Home School | MN, USA | 200 | 0 | 0 | 0 | `https://mn.milesplit.com/teams/75555-vail-home-school/roster` |
| — | WI+MN | — | **50 teams / 38 with ≥1 Co2027** | — | 50×200 | **4,883** | **579** | **450** | — |

### Stable identifiers

| Entity | Identifier | Evidence |
|---|---|---|
| Athlete | numeric `athleteId`, canonical `/athletes/{id}-{slug}` | e.g. `/athletes/14399169-julian-aguilera` (Abbotsford) |
| Team | numeric `teamId`, network-global; `/teams/{id}-{slug}` | 50 team IDs recorded in the manifest; ID-only forms 301 to the slug form |
| Class | `column-grad-year` = absolute graduating class (`2027`) | not a per-season grade — survives the academic-year boundary |
| Season membership | `data-season-id` ∈ {0,1,2,3} per roster row | `#rosterFilterType` options prove the 1/2/3 → Indoor/Outdoor/XC mapping |
| Corpus athlete | `AthleteID` (Athletic.net), e.g. `18454640` | the 4 sampled names that also occur in the repo's own matcher output resolve to the **same** Athletic.net athlete IDs (see confidence note) |

No name is an identifier: the corpus itself holds 133,570 distinct normalized names for 142,705 rows.

### Athletic.net leverage

- MileSplit exposes **no Athletic.net links or IDs** (peer report 07: 0 occurrences across 28 documents).
  The leverage is therefore *name + school + class + state*, resolved offline in this pilot.
- What the pilot proves about request avoidance: **1 roster request yields ≈ 97.7 rostered athletes and
  ≈ 20.6 Class-of-2027 athletes**; the 50-request sample produced **269 boys whose full name occurs
  nowhere in the 142,705-row national corpus** (≈ 5.4 beyond-corpus boys per request).
- Whole-frame projection. `[INFERENCE]` from the measured per-team means: sample mean = 579 Co2027
  boys / 50 teams = **11.58 per team** (empty rosters included), of which 46.5 % (269/579) are
  name-absent from the corpus → **5.38 net-new boys per team**. Applying that to the whole frame:
  WI 597 teams → ≈ 6,913 Co2027 boys, ≈ **3,215 beyond corpus**; MN 592 teams → ≈ 6,855, ≈ **3,187
  beyond corpus**; total ≈ **6,400 beyond-corpus boys for ≈ 1,189 roster requests**, plus the female and
  XC-only segments the corpus cannot see at all. This is a **projection from a systematic 25-team/state
  sample, not a census**: 20 % of sampled teams carry no roster, the sampling is deterministic rather
  than random, and the corpus itself may simply lag live Athletic.net (see Confidence note 5).

### Athlete evidence

| Field | Free on the roster page? | Detail |
|---|---|---|
| name | yes | `{Last}, {First}` in the athlete link text |
| graduating class | yes | `column-grad-year` (`2027`) |
| gender | yes | `column-gender` (`m`/`f`) |
| school / city / state | yes | team label + `/teams` city field (`ABBOTSFORD, WI, USA`) |
| TF/XC + Indoor/Outdoor distinction | yes | `data-season-id` flags per athlete |
| performances / PRs / meets | not on this surface | lives on the athlete profile (PR table free; full history is MileSplit PRO) — out of scope for this pilot |
| athlete profile URL | yes | `/athletes/{athleteId}-{slug}` on every row |

### Recruiting information

Not applicable to this pilot. Roster pages expose no coach/AD fields (peer report 07 measured 0 matches
on the same page family); no athlete personal contact data was collected or is present.

### Result evidence

Not measured — this pilot's unit is the roster (class membership), not the result. Peer report 07
documents the result surfaces (`/meets/.../results/<RSID>/raw`, `gradYear` on performance rows).

### Incremental use

- Rosters change **once per year** (class promotion + roster edits), so the whole-state sweep is a
  start-of-season job, not a weekly one. 1 request/team, no pagination, no change signal (no ETag /
  Last-Modified observed on roster pages; responses carried only `date`).
- The weekly delta that *matters* is not the roster but the athlete's participation flags
  (`data-season-id`) — already inside the same request, so a monthly re-poll of the frame plus the
  roster sweep at season start covers it.

### Access characteristics

**Normal HTML (server-rendered), free, unauthenticated.** Classification: normal HTML.
- 54/54 requests HTTP 200; **zero 403/429/CAPTCHA and zero `Retry-After`** across both hosts
  (sequential, ≥1.2 s spacing, browser UA). No cookies, no tokens, no login.
- Roster pages ranged 29,242 B (empty roster template) to 901,524 B (Saint Paul Central, 424 athletes).
- Page size scales with roster size → the frame's cost is ~(number of teams) requests per state per sweep.

### Recommendation

**DISCOVERY-ONLY** for the marginal role this pilot tested (plus a secondary **VALIDATION** role for the
roster half that *is* in the corpus). Not PRIMARY (no results, no Athletic.net IDs, no coach data) and
not ATHLETIC.NET-SEED (no AthleteID is ever exposed).

**Marginal coverage, measured (boys = the corpus's scope):**

- Roughly **half** of roster-registered Class-of-2027 boys are **not** in the delivered corpus:
  283/579 = **48.9 %** (95 % CI 44.8–52.9 %) by the loose join, 300/579 = 51.8 % by the alias-aware
  strict join; of the 283 unmatched, **269 (95.1 %)** have a full name that occurs **nowhere** in the
  142,705-row national corpus — so this is not a join artefact.
- The gap concentrates where the corpus structurally cannot see: **XC-only boys** (no outdoor/in/outdoor
  TF flag) are 68/76 = **89.5 %** beyond corpus (95 % CI 80.6–94.6 %); boys with **no season flag at
  all** are 41/41 = 100 %; boys flagged outdoor are the *most* covered at 65.3 % matched.
- **The female half is invisible to this corpus**: 0/450 girls matched, against 230.1 matches expected
  if the corpus were mixed-gender — i.e. the corpus is boys-only and the 450 sampled Class-of-2027 girls
  (43.7 % of the sample) are *all* marginal discovery for any girls pipeline.

**Marginal cost:** 1 request per team (measures above: 97.7 athletes and 20.6 Class-of-2027 athletes per
request); 1,189 requests for the whole WI+MN HS frame.

### Measured numbers (single source of truth)

| stratum | N | strict match | +reviewed aliases | loose match | beyond corpus (loose-miss) | of which name absent nationally |
|---|---|---|---|---|---|---|
| ALL sampled Co2027 | 1029 | 268 (26.0%, 95% CI 23.5–28.8%) | 279 (27.1%) | 296 (28.8%) | 733 (71.2%, 95% CI 68.4–73.9%) | 718 |
| Boys (corpus cohort) | 579 | 268 (46.3%, 95% CI 42.3–50.4%) | 279 (48.2%) | 296 (51.1%) | 283 (48.9%, 95% CI 44.8–52.9%) | 269 |
| Girls (corpus is boys-only) | 450 | 0 (0.0%, 95% CI 0.0–0.8%) | 0 (0.0%) | 0 (0.0%) | 450 (100.0%, 95% CI 99.2–100.0%) | 449 |
| WI boys | 403 | 178 (44.2%, 95% CI 39.4–49.0%) | 182 (45.2%) | 192 (47.6%) | 211 (52.4%, 95% CI 47.5–57.2%) | 201 |
| MN boys | 176 | 90 (51.1%, 95% CI 43.8–58.4%) | 97 (55.1%) | 104 (59.1%) | 72 (40.9%, 95% CI 33.9–48.3%) | 68 |
| Boys outdoor-flagged | 441 | 267 (60.5%, 95% CI 55.9–65.0%) | 278 (63.0%) | 288 (65.3%) | 153 (34.7%, 95% CI 30.4–39.3%) | 149 |
| Boys indoor-flagged | 365 | 216 (59.2%, 95% CI 54.1–64.1%) | 225 (61.6%) | 231 (63.3%) | 134 (36.7%, 95% CI 31.9–41.8%) | 132 |
| Boys XC-flagged | 171 | 62 (36.3%, 95% CI 29.4–43.7%) | 66 (38.6%) | 78 (45.6%) | 93 (54.4%, 95% CI 46.9–61.7%) | 89 |
| Boys XC-only (no TF flag) | 76 | 1 (1.3%, 95% CI 0.2–7.1%) | 1 (1.3%) | 8 (10.5%) | 68 (89.5%, 95% CI 80.6–94.6%) | 64 |
| Boys TF-only (no XC flag) | 367 | 206 (56.1%, 95% CI 51.0–61.1%) | 213 (58.0%) | 218 (59.4%) | 149 (40.6%, 95% CI 35.7–45.7%) | 145 |
| Boys TF+XC | 95 | 61 (64.2%, 95% CI 54.2–73.1%) | 65 (68.4%) | 70 (73.7%) | 25 (26.3%, 95% CI 18.5–36.0%) | 25 |
| Boys no season flag (stub) | 41 | 0 (0.0%, 95% CI 0.0–8.6%) | 0 (0.0%) | 0 (0.0%) | 41 (100.0%, 95% CI 91.4–100.0%) | 35 |
| Boys, school in corpus | 561 | 268 (47.8%, 95% CI 43.7–51.9%) | 279 (49.7%) | 291 (51.9%) | 270 (48.1%, 95% CI 44.0–52.3%) | 256 |
| Boys, school absent from corpus | 18 | 0 (0.0%, 95% CI 0.0–17.6%) | 0 (0.0%) | 5 (27.8%) | 13 (72.2%, 95% CI 49.1–87.5%) | 13 |

### Confidence note

1. **Join method.** Tiers, in increasing looseness: `T1` normalized `name + school + state`;
   `T2` `name + school` (any state); `T3` `name + state`; `T5` `T1/T2` through the three hand-reviewed
   school-label aliases below; "beyond corpus" = a miss at the loose tier (T1∪T2∪T3∪T4∪T5).
   Normalization is symmetric on both sides: casefold, `&`→`and`, drop parenthetical qualifiers
   (fixes `Greenfield (WI)`), punctuation→space, `st/ft/mt`→`saint/fort/mount` (fixes
   `Saint Paul Central` vs `St. Paul Central`), then school suffixes stripped. All 268 strict matches are
   `T1` (none needed the weaker `T2`), and **0 strict matches are ambiguous** (exactly one corpus
   `AthleteID` per match).
2. **Reviewed aliases (3 admitted, 5 rejected).** Admitted: `Laona/Wabeno` → corpus `Wabeno Area-Laona`;
   `St. Augustine Preparatory Academy` → `St. Augustine Prep`; `Rushford-Peterson High School` →
   `Rushford-Peterson/Houston` (co-op label, flagged). Rejected as different schools:
   `West De Pere`, `Kettle Moraine`, `Woodbury` (New Life Academy of Woodbury ≠ Woodbury HS),
   `Minnesota`, `Milwaukee`. The rejected pairs are in `school-alias-candidates.json` with the probe output.
3. **False-positive direction.** The loose tier is the *liberal* bound. 28 loose matches ignore a school
   mismatch (e.g. `Icean Hangartner` at Gale-Ettrick-Trempealeau matched a corpus row labelled
   `Melrose-Mindoro`), so some of the 296 loose matches are probably different people; only 4 are
   *internally* ambiguous. The strict tier (268, all unambiguously school-anchored) is the conservative
   bound on "already in the corpus", so the true beyond-corpus share for boys lies in **46.5 %–51.8 %**.
4. **Independent join validation (4/4).** Four sampled boys also appear in the repository's own matcher
   output (`out-full-variants-final/matches.csv`, a *different* population: 241 "2027 New Slate Members"
   prospects). The repo resolved them to Athletic.net profile URLs; my offline name+school join landed on
   the **same AthleteID** in all four cases:
   `Fynn Schlicht` (Grand Rapids HS, MN) 23310257; `Mikah Vail` (Gale-Ettrick-Trempealeau, WI) 24938493;
   `Marshel Woyak` (Wautoma, WI) 24721396; `Brennan Shafer` (Shell Lake, WI) 24865483.
5. **What is *not* proven.** (a) No live Athletic.net check was made (hard constraint) — an athlete
   "beyond the delivered corpus" may still exist on Athletic.net; the claim is strictly about the corpus.
   (b) The corpus is a rankings-derived export, so the ~90 % XC-only miss rate is *expected by
   construction* rather than surprising; the pilot quantifies the consequence, it does not explain
   Athletic.net's full population. (c) Two states, n=25 teams each; per-state and per-sport CIs are in
   the table above, and the sampling is systematic, not random. (d) School-label matching still leaves
   17 of the 50 sampled teams whose label is absent from the corpus (WI 3, MN 14) — 18 boys sit at those
   schools; exact-label presence before aliases is 30/50 (WI 20, MN 10) and 33/50 after the three
   admitted aliases.

### Repo-corpus probe (the read-only repo delivers no athlete corpus)

Path checks executed (raw output in `evidence/gaps/34/repo-corpus-probe.txt` and
`repo-artifact-check.txt`):

```
ABSENT  /home/lewis/src/ad-law-scrape/athletic-rust-pipeline/data            (directory does not exist)
ABSENT  /home/lewis/src/ad-law-scrape/athletic-rust-pipeline/data/athletes.{csv,parquet}
ABSENT  .../target/athletes.{csv,parquet}   (target/ = 31 GB of cargo build artifacts only)
ABSENT  .../docs/corpus.xlsx
FILE    .../out-{full-variants-final,full-hardened}/matches.csv   241 rows — 2027 New Slate Members prospects
FILE    .../out-*/unresolved.csv                                  84 rows  — same population, unresolved
FILE    .../out-full-*/2027 New Slate Members - Athletic Matches.xlsx  11.4 MB, sheets [Export, Sheet1, Athletic Matches]
```

Those repo artifacts are the recruiting-matcher's output over a college "2027 New Slate Members"
prospect list, **not** a Midwest HS athlete corpus: only 4 of my 579 sampled boys' names occur in them.
The delivered Athletic.net corpus therefore had to be taken from the retained delivery workbook named in
the mission brief and used by peer reports 27/30 —
`/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx` (read-only, offline, unmodified).
All measurements above come from that file, streamed locally (142,705 rows read; primary-state counts
WI 3,961 / MN 3,845 reproduce the peer reports exactly).

### Evidence appendix

All live requests issued 2026-09-20T14:05:44Z–14:07:02Z (session local time 09:05:44–09:07:02 America/Chicago; server `date:` headers are authoritative and shown verbatim). Method `GET` throughout, one browser UA, sequential.

| URL | method | HTTP status | what it proved | timestamp (server `date:`) |
|---|---|---|---|---|
| `https://wi.milesplit.com/robots.txt` | GET | 200 | robots.txt verbatim (173 B): `/teams` and `/teams/<id>/roster` are NOT disallowed; `Disallow: /rankings`, `/virtual-meets`, `/api/`, `/contact` | Sun, 20 Sep 2026 14:05:44 GMT |
| `https://wi.milesplit.com/teams?type=1` | GET | 200 | HS team frame: 597 team records (id, label, city) on a single A–Y page — the sampling frame (stride 23 → 25 picks) | Sun, 20 Sep 2026 14:05:45 GMT |
| `https://mn.milesplit.com/robots.txt` | GET | 200 | robots.txt verbatim (173 B): `/teams` and `/teams/<id>/roster` are NOT disallowed; `Disallow: /rankings`, `/virtual-meets`, `/api/`, `/contact` | Sun, 20 Sep 2026 14:05:45 GMT |
| `https://mn.milesplit.com/teams?type=1` | GET | 200 | HS team frame: 592 team records (id, label, city) on a single A–Y page — the sampling frame (stride 23 → 25 picks) | Sun, 20 Sep 2026 14:05:47 GMT |
| `https://wi.milesplit.com/teams/52649-abbotsford/roster` | GET | 200 | roster page, 151,677 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:05:52 GMT |
| `https://wi.milesplit.com/teams/14044-ashland/roster` | GET | 200 | roster page, 191,321 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:05:53 GMT |
| `https://wi.milesplit.com/teams/14061-birchwood/roster` | GET | 200 | roster page, 63,603 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:05:55 GMT |
| `https://wi.milesplit.com/teams/14073-cameron/roster` | GET | 200 | roster page, 421,113 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:05:56 GMT |
| `https://wi.milesplit.com/teams/21422-cochrane-fountain-city/roster` | GET | 200 | roster page, 187,485 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:05:58 GMT |
| `https://wi.milesplit.com/teams/20399-de-pere/roster` | GET | 200 | roster page, 796,381 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:05:59 GMT |
| `https://wi.milesplit.com/teams/14111-elk-mound/roster` | GET | 200 | roster page, 462,051 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:01 GMT |
| `https://wi.milesplit.com/teams/14120-gale-ettrick-trempealeau/roster` | GET | 200 | roster page, 423,246 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:02 GMT |
| `https://wi.milesplit.com/teams/14127-greenfield-wi/roster` | GET | 200 | roster page, 315,760 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:04 GMT |
| `https://wi.milesplit.com/teams/14144-hustisford/roster` | GET | 200 | roster page, 47,822 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:05 GMT |
| `https://wi.milesplit.com/teams/25903-kettle-moraine-lutheran/roster` | GET | 200 | roster page, 299,392 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:07 GMT |
| `https://wi.milesplit.com/teams/22767-laonawabeno/roster` | GET | 200 | roster page, 161,992 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:08 GMT |
| `https://wi.milesplit.com/teams/14170-manawa/roster` | GET | 200 | roster page, 225,007 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:09 GMT |
| `https://wi.milesplit.com/teams/14189-menomonie/roster` | GET | 200 | roster page, 748,382 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:11 GMT |
| `https://wi.milesplit.com/teams/30245-milwaukee-pulaski/roster` | GET | 200 | roster page, 149,902 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:12 GMT |
| `https://wi.milesplit.com/teams/26846-new-auburn/roster` | GET | 200 | roster page, 180,752 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:14 GMT |
| `https://wi.milesplit.com/teams/14228-oconomowoc/roster` | GET | 200 | roster page, 695,010 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:15 GMT |
| `https://wi.milesplit.com/teams/40599-pembine/roster` | GET | 200 | roster page, 29,242 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:17 GMT |
| `https://wi.milesplit.com/teams/14074-racine-case/roster` | GET | 200 | roster page, 342,315 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:18 GMT |
| `https://wi.milesplit.com/teams/14270-rock-river-charter-school/roster` | GET | 200 | roster page, 31,043 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:20 GMT |
| `https://wi.milesplit.com/teams/52909-shell-lake/roster` | GET | 200 | roster page, 227,082 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:21 GMT |
| `https://wi.milesplit.com/teams/43859-st-augustine-preparatory-academy/roster` | GET | 200 | roster page, 254,822 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:23 GMT |
| `https://wi.milesplit.com/teams/14295-suring/roster` | GET | 200 | roster page, 180,179 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:24 GMT |
| `https://wi.milesplit.com/teams/14307-valders/roster` | GET | 200 | roster page, 316,663 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:25 GMT |
| `https://wi.milesplit.com/teams/14320-wautoma/roster` | GET | 200 | roster page, 225,638 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:27 GMT |
| `https://mn.milesplit.com/teams/12984-academy-for-sciences-and-agriculture/roster` | GET | 200 | roster page, 31,225 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:27 GMT |
| `https://mn.milesplit.com/teams/12997-ashby-high-school/roster` | GET | 200 | roster page, 204,004 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:29 GMT |
| `https://mn.milesplit.com/teams/13019-bigfork-high-school/roster` | GET | 200 | roster page, 103,032 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:30 GMT |
| `https://mn.milesplit.com/teams/13037-butterfield-odin-high-school/roster` | GET | 200 | roster page, 31,105 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:32 GMT |
| `https://mn.milesplit.com/teams/13056-chisholm-high-school/roster` | GET | 200 | roster page, 228,248 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:33 GMT |
| `https://mn.milesplit.com/teams/13076-covenant-life-school/roster` | GET | 200 | roster page, 29,251 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:35 GMT |
| `https://mn.milesplit.com/teams/61841-duluth-cathedral-high-school/roster` | GET | 200 | roster page, 29,390 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:36 GMT |
| `https://mn.milesplit.com/teams/13109-elk-river-high-school/roster` | GET | 200 | roster page, 652,632 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:37 GMT |
| `https://mn.milesplit.com/teams/13129-fond-du-lac-ojibwe-high-school/roster` | GET | 200 | roster page, 45,512 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:39 GMT |
| `https://mn.milesplit.com/teams/13145-grand-rapids-high-school/roster` | GET | 200 | roster page, 449,091 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:40 GMT |
| `https://mn.milesplit.com/teams/13163-hill-city-high-school/roster` | GET | 200 | roster page, 29,704 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:42 GMT |
| `https://mn.milesplit.com/teams/13184-jackson-county-central-high-school/roster` | GET | 200 | roster page, 189,995 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:43 GMT |
| `https://mn.milesplit.com/teams/13203-lakeview-high-school/roster` | GET | 200 | roster page, 182,416 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:45 GMT |
| `https://mn.milesplit.com/teams/41733-lyle-public-school/roster` | GET | 200 | roster page, 95,702 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:46 GMT |
| `https://mn.milesplit.com/teams/13241-mcgregor-high-school/roster` | GET | 200 | roster page, 131,903 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:48 GMT |
| `https://mn.milesplit.com/teams/13261-minnesota-state-academy-for-the-deaf/roster` | GET | 200 | roster page, 83,351 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:49 GMT |
| `https://mn.milesplit.com/teams/13282-new-life-academy-of-woodbury/roster` | GET | 200 | roster page, 203,232 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:50 GMT |
| `https://mn.milesplit.com/teams/21432-onamia-high-school/roster` | GET | 200 | roster page, 94,904 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:52 GMT |
| `https://mn.milesplit.com/teams/13321-pine-city-high-school/roster` | GET | 200 | roster page, 268,736 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:53 GMT |
| `https://mn.milesplit.com/teams/64886-rivertree-school/roster` | GET | 200 | roster page, 30,817 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:55 GMT |
| `https://mn.milesplit.com/teams/13358-rushford-peterson-high-school/roster` | GET | 200 | roster page, 201,851 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:56 GMT |
| `https://mn.milesplit.com/teams/13379-saint-paul-central-high-school/roster` | GET | 200 | roster page, 901,524 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:58 GMT |
| `https://mn.milesplit.com/teams/13396-silver-bay-william-kelley-high-school/roster` | GET | 200 | roster page, 108,069 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:06:59 GMT |
| `https://mn.milesplit.com/teams/61848-taylors-falls-high-school/roster` | GET | 200 | roster page, 29,360 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:07:01 GMT |
| `https://mn.milesplit.com/teams/75555-vail-home-school/roster` | GET | 200 | roster page, 28,962 B server-rendered HTML (Athlete / Gender / Class / Indoor / Outdoor / XC) | Sun, 20 Sep 2026 14:07:02 GMT |
| **54 requests total** (2 robots.txt + 2 team indexes + 50 rosters) | GET | 200 ×54 | zero non-200, zero 403/429/CAPTCHA, zero `Retry-After`; sequential, ≥1.2 s/host, browser UA, no cookies | — |

Offline (no network) work in the same session — not HTTP:

| Artifact | Method | Status | What it proved |
|---|---|---|---|
| `research/midwest/../evidence/gaps/34/rosters/{wi,mn}/*.html` (50 files) | local parse (stdlib regex, peer-validated roster parser) | n/a | 4,883 rows; header `Athlete, Gender, Class, Indoor, Outdoor, XC`; `data-season-id` agrees with positional flags 4,883/4,883 |
| `/home/lewis/Downloads/Athletic-2026-US-Boys-Grade11-All-Athletes.xlsx` | local read (streaming `zipfile`+`ElementTree`) | n/a | 142,705 rows, 133,570 distinct names; WI 3,961 / MN 3,845 primary-state rows |
| `join-results.json`, `join-sample.csv`, `join-metrics.json`, `final-tables.md` | local compute | n/a | the match/miss numbers above (per-athlete tier flags + corpus AthleteIDs) |
| `join-examples.md`, `name-presence-check.txt` | local compute | n/a | match/miss inspection samples; 269/283 unmatched boys absent nationally by name |
| `school-alias-candidates.json`, `school-alias-probe-output.txt` | local compute | n/a | deterministic alias candidates + the false alias pairs that were rejected |
| `repo-corpus-probe.txt`, `repo-artifact-check.txt` | local read of the read-only repo | n/a | `data/` absent; repo artifacts are a different population (4/579 name intersection) |

Reproduce: `python3 tools/gap34/fetch_gex.py teams && python3 tools/gap34/fetch_gex.py rosters`
(live, 54 requests) then, offline, `parse_rosters.py`, `school_alias_probe.py`, `join_corpus.py`,
`name_presence_check.py`, `final_tables.py`, `check_repo_artifacts.py`.
