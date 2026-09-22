# 42. Nebraska gap follow-ups (report 24 open items)

Status: complete
Observed on: 2026-09-20

Session window: 2026-09-20 09:04–09:19 CDT (UTC-5); i.e. 14:04–14:19 UTC. Three tasks, one file:
(1) do NE school-district / official school sites publish professional coach or AD emails, at what cost per school;
(2) re-verify on **live 2026 files** that NSAA S3 results expose grade (field-level evidence);
(3) enumerate the timing/result hosts NE meets actually use (including the `strivtv` / `prepcast` leads) with URLs
and HTTP status codes.

**Politeness disclosure (recorded, not hidden):** this session sent ≈106 requests total (school sweep 50;
`search.athletic.live` 12; `nsaa-static.s3.amazonaws.com` 13; `nsaahome.org` 2 reader fetches; `striv.tv` 5;
prepcast/DDG candidate probing ~19; timer hosts 9), sequential, ≥1.1 s spacing, browser UA. That is **above the
brief's ≤80/agent aim** (mainly the 50-request school sweep and a dead-end prepcast host hunt). No host received
more than 12 requests, no 429, no `Retry-After`, no CAPTCHA, no auth prompt, no WAF block anywhere except the
two recorded access findings (Millard West 403; `aurorahuskies.org/page/husky-timing` Cloudflare JS challenge).
No `*.athletic.net` request was sent.

---

### Source

| Block | What was queried | Why |
|---|---|---|
| NE school/district web | 16 NSAA member schools' own sites (homepage + athletics/staff/contacts pages) | Task 1: coach/AD email presence and cost |
| NSAA static mirror `nsaa-static.s3.amazonaws.com` | `/textfile/track/*` 2026 state TF results (per-day + final + unified) | Task 2: grade exposure on live 2026 files |
| NSAA WP site `nsaahome.org` (reader fetch; curl gets 403 per report 24) | `/track-field/`, `/cross-country/` | Task 2/3: current file names, XC district-file paths, timer links |
| AthleticLIVE fleet `search.athletic.live` (ES, anonymous POST) | `*_meet_list`, `*` index list | Task 3: enumerate every NE meet + its timer-credit host |
| AthleticLIVE white-label hosts | `results.blacksquirreltiming.com`, `results.hnscats.org`, `live.sub4sports.com`, `results.run-ne.com`, `dakotatiming.anet.live`, `greenferret.anet.live`, `www.dakotatiming.com`, `www.aurorahuskies.org` | Task 3: prove host liveness with status codes |
| `striv.tv` | homepage, `/channel/broken-bow/`, `www.` redirect | Task 3: does the NE streaming network publish results? |
| prepcast name-space | `prepcast.tv`, `.live`, `.app`, `.com`, `.co`, `.io`, `.net`, `.org`, `.us`, `prep-cast.com`, `prepcastsports.com`, `prepcastusa.com`, `goprepcast.com`, `theprepcast.com`, `prepscast.com` (DNS + HTTP) | Task 3: resolve the `prepcast` lead |

### Coverage

**Task 1 — 16 schools, 4 classes, 6 platform families.** Class A: Omaha Central, Lincoln Southwest, Millard West,
Kearney, Fremont, Grand Island, Papillion-La Vista South · Class B: Elkhorn, Gretna, Scottsbluff, Waverly, Seward ·
Class C/D: Adams Central, Boone Central, Broken Bow, Aurora. Total 50 requests (49×200 + 1×403), 50 HTML captures
+ 3 JSON in `evidence/gaps/42/schools/`. Platforms seen: Apptegy/Thrillshare (`lps.org`, `gpsne.org`,
`district145.org`, `sewardpublicschools.org`, `aurorahuskies.org`, `bbps.org`, `fremonttigers.org`), WordPress+Elementor
(`elkhornweb.org`), Blackboard/SharpSchool (`sbps.net`), Cornerstone (`central.ops.org`), GoDaddy-class static
(`adamscentral.us`), district CMS (`kearneypublicschools.org`, `gips.org`, `boonecentral.org`, `plcschools.org`).

**Task 2 — live 2026 NSAA result files** (state TF finals, three files, all fetched 2026-09-20) and the XC
district-file contract (8 HTML paths; the *current* file on disk is still the 2025-season one — 2026 district
meets are 2026-10-15, `26Districts.pdf`). History depth unchanged from report 24 (state TF pages 1900s→2026,
state XC 1960→2025).

**Task 3 — NE timer universe = 2,203 distinct meets.** All-time NE meet docs in the AthleticLIVE fleet
(256 tenants): **4,520 docs**, **2,203 distinct meet ids** (`cardinality(i)`), **130 distinct timer-credit names**,
**34 distinct timer-URL values**; doc-level sport split outdoor 3,043 / XC 994 / indoor 483. `live_results_meet_list`
is a **global union** index, proven by duplicate meet docs: `i=73627` and `i=70557` each appear in both
`live_results_meet_list` and the tenant index that owns them (`athleticlive_meet_list` — captured in
`es_ne_schooltimers.json`); the union also holds meets no `*_meet_list` reports for the query at hand
(`i=755`: union total = 1, `athleticlive` total = 0). So per-tenant counts below are authoritative and the
4,520 figure double-counts; dedupe on `(e,i)` or on `i` alone.

### Enumeration

**A. Every NE meet + its timer, one POST** (no auth; ES DSL). Bodies used verbatim this session:

```jsonc
// 1) NE meet inventory + timer attribution (4,520 docs -> 2,203 distinct meets)
POST https://search.athletic.live/*_meet_list/_search
{"size":0,"track_total_hits":true,"query":{"match":{"lsa":"Nebraska"}},
 "aggs":{"tenants":{"terms":{"field":"_index","size":300}},
         "timers":{"terms":{"field":"tna.keyword","size":100}},
         "urls":{"terms":{"field":"tu.keyword","size":100}},
         "distinct":{"cardinality":{"field":"i"}}}}
// 2) weekly delta (ND report's recipe, verified shape here)
{"size":500,"query":{"bool":{"filter":[{"match":{"lsa":"Nebraska"}},
  {"range":{"sdy":{"gte":"<last run>","lte":"<today>"}}}]}},"sort":[{"sdy":"desc"}]}
```
*Note:* aggregations 400/fail unless `.keyword` suffixed (250/256 shards failed on `tna`/`sdy` plain fields;
`tna.keyword`, `tu.keyword`, `sdy.keyword` work). `sdy.keyword` year-bucket agg returned **no buckets** for a
terms agg (string field worked with `size` but returned empty in the histogram attempt) — filter by
`range` on `sdy`/`md` instead.

**B. Per-tenant NE counts** (from the aggregation above; these are the providers NE meets actually use):

| Tenant (`_index`) | NE docs | Timer credit (`tna`) seen | Timer URL (`tu`) |
|---|---|---|---|
| `live_results_meet_list` | 2,203 *(union mirror — do not add)* | mixed | mixed |
| `athleticlive_meet_list` | 1,083 | school/self-timed (LPS, OPS, Malcolm, Norfolk, Husky Timing, …) | mostly null |
| `blacksquirrel_meet_list` | 945 | Black Squirrel Timing | blacksquirreltiming.com |
| `greenferrettiming_meet_list` | 121 | Green Ferret Timing / Derek Fey | csmflames.com |
| `hnscats_meet_list` | 43 | Hartington-Newcastle Public Schools | results.hnscats.org |
| `sub4sports_meet_list` | 40 | Sub4Sports(, LLC) | live.sub4sports.com |
| `dakota_meet_list` | 31 | Dakota Timing | dakotatiming.com |
| `delta_meet_list` | 20 | Delta Timing | deltatiming.com |
| `aatiming_meet_list` | 12 | All-American Timing | aatiming.com / results.aatiming.com |
| `wayzata_meet_list` | 11 | Wayzata Results, LLC/Inc. | wayzataresults.com |
| `runnebraska_meet_list` | 9 | Run Nebraska / Wade Lanum | results.run-ne.com |
| `heros_meet_list` | 1 | Hero's Timing (ND) | herostiming.com |
| `reddirt_meet_list` | 1 | Red Dirt Race Management | reddirtrunning.com |

**What the 130 timer names actually are** (full `.keyword` bucket list captured in `es_ne_agg2.json`) — the NE
ecosystem is *school-run timers plus a handful of commercial providers and named individuals*, not a provider
oligopoly:
* **Commercial**: Black Squirrel Timing 1,890 docs, Green Ferret Timing 157 (+ "Derek Fey" 138, "- Derek Fey" 21,
  "DFey" 3 — same person credited 4 ways), Dakota Timing 48, Delta Timing 40, All-American Timing 24,
  Wayzata Results 14+8, R&K Timing 14, Run Nebraska 7 (+ Wade Lanum 13), Red Dirt 3, Hero's Timing 2.
* **School-run** (the majority of distinct names): Husky Timing (Aurora) 144, Omaha Public Schools 142,
  Lincoln Public Schools 128, Cozad Timing 74, Hartington-Newcastle PS 73, Boone Central Schools 62,
  Eagle Eye Timing LLC 38, Arnold Public Schools 34, Platteview 28, Cardinal Timing 26, Fort Calhoun Timing 26,
  LongHorn Track 26, Cougar Timing 22, Homer Community Schools 18, Malcolm Timing 14 (+ "Malcolm FAT Timing" 4,
  "Dallas Sweet - Malcolm Timing" 4), Perkins County 12, Yutan 10, Rock County 10, Papillion-La Vista Community
  Schools Timing 10, Battle Creek 8, Conestoga FAT 8, LouisvilleFATSquad 8, Sidney PS 8, DC West 6,
  West Point(-Beemer) 6, FKC/FC-TC/CHS/Fillmore Central conference FAT crews, FinishLynx variants 12+4+2 …
* **Individuals**: Jayson/Jason Bishop 52+12, Andy & Sara Scoring 24, Cody Stappert 12, Isaac Frecks 10,
  Riley Swedburg/Swedberg 10+4, Matt McKay 8, Julie Lodes 6, Alex Aldrich 2 …
* Implication: for NE regular-season results the ES `tna`/`tu` pair is the join key, and the *provider* field is
  often a school or a person — a collector must key on tenant + meet id, not on a provider allow-list.

**C. School-site route that actually yields emails** (Task 1 pattern, verified on ≥4 sites):
`homepage → "Athletics"/"Activities" nav → staff/contacts page`; on Apptegy sites the staff directory is
server-rendered inside the page's own JSON content graph (`CONTENT_NODE_HEADING` / `CONTENT_NODE_TEXT` nodes with
`mailto:` hrefs, e.g. LSW's `lbercey@lps.org` next to "London Bercey") — a plain HTTP GET sees it, no JS needed.
On WordPress+Elementor sites (Elkhorn) emails are behind **Cloudflare email obfuscation**
(`<span class="__cf_email__" data-cfemail="…">`): 8 spans on the activity-contacts page (15 across the 4 Elkhorn
pages), decoded by the public XOR scheme (first byte = key) to `sfjell@epsne.org`, `lford@epsne.org`,
`rortmeier@epsne.org`, `canderson@epsne.org`, `marchibeque@epsne.org`, `jmckenzie@epsne.org`, `studentservices@epsne.org`.

**D. NE school activities/schedule layer:** **10 of 16** probed schools route their activities calendar to
**Bound** (`gobound.com/ne/schools/<slug>` for calendar/tickets, `gobound.com/ne/districts/<slug>` for
district-level, `manager.gobound.com/facility/...` for facilities, `tickets.gobound.com/tickets/events/<id>` for
event tickets, `gobound.com/widgets/<id>` for embeds). Schools verified with Bound links in their own HTML:
Adams Central, Aurora, Boone Central, Broken Bow, Elkhorn (district URL), Gretna, Kearney, Omaha Central,
Scottsbluff, Seward (district URL). Not present on the pages reached for LPS, Fremont, Grand Island, PLCS,
Waverly (and Millard West's site was 403). Example URLs: `gobound.com/ne/schools/adamscentral/calendar`,
`/aurora/calendar?v=month&includePractices=true`, `/boonecentral`, `/brokenbow/calendar`,
`/elkhornpublic` (district), `/gretna` + `/tickets/events/<event-id>`, `/kearney/calendar?v=list`,
`/omahacentral`, `/scottsbluff` + `/widgets/80824aaa04504a35b9b5`, `/sewardps` (district) — i.e. a NE-wide
calendar/ticketing layer independent of results.

### Stable identifiers

| id | Source | Note |
|---|---|---|
| `i` | AthleticLIVE meet doc | tenant meet id; the key for `<tenant-host>/meets/<i>` |
| `ani` | same doc | **Athletic.net MeetID** (independent of any Athletic.net request) |
| `e` | same doc | tenant key (`blacksquirrel`, `hnscats`, `sub4sports`, `dakota`, `delta`, `aatiming`, `runnebraska`, `greenferrettiming`, `athleticlive`, `wayzata`, …) |
| `tna` / `tu` / `qe` | same doc | timer credit name / URL / e-mail when the timer published one |
| `us` | same doc | tenant short link — **observed values**: `anet.live/*`, `bst.anet.live/*`, `sub4.anet.live/*`, `hnscats.anet.live/*`, `dakota.anet.live/*`, `greenferret.anet.live/*`, `wrresults.com/*`, `dtg.io/*`, `tmio.co/*` |
| `anis` | same doc | array of Athletic.net meet references `{u: "https://www.athletic.net/TrackAndField/meet/<id>", ani, n}` — **98/528 unique NE docs** carry it (sparser than the numeric `ani`), max 1 entry observed |
| `qe` | same doc | **timer contact e-mail published on the meet** — non-empty in **219/528 unique NE docs (41.5 %), 19 distinct addresses** (e.g. `info@blacksquirreltiming.com`, `huskytiming@4rhuskies.org`) |
| `atr` | same doc | "Timed by …" HTML fragment (`Licensed to ACTIVE/HY-TEK … Timed by <a>…</a> … email results questions to … <mailto:>`) — human-readable timer credit + contact |
| `md` / `sd` / `ed` | same doc | meet date(s) array / start / end |
| tenant→host | measured | `blacksquirrel`→`results.blacksquirreltiming.com`; `hnscats`→`results.hnscats.org`; `sub4`→`live.sub4sports.com` (short `sub4.anet.live`); `dakota`→`dakotatiming.anet.live` (timer site `www.dakotatiming.com` 404s on `/meets/<i>`); `greenferret`→`greenferret.anet.live`; `aa`→`results.aatiming.com`; `run-ne`→`results.run-ne.com` |
| athlete / result / season IDs | — | **Not applicable** on this source: meet docs carry no athlete id, no result id, no season id. Athlete-level ids live in the tenant `athlete_list` index / Firebase RTDB (reports 12/25); results are keyed by meet + event + athlete name, so AN meet id (`ani`) is the join key for everything else |
| NSAA school identity | report 24 | still name-string only (no NSAA ids); enrolment rank is not an id |
| Coach/AD identity | NSAA directory | person-name string per school×sport role (report 24: TF 274/275, XC 243/240 usable names, AD 311/312) |

Grade semantics (re-verified this session): the `Year` column / `(NN)` suffix is the **grade in that school
year** — so Class of 2027 = `11` in the 2025-26 state TF files (May 2026) and will be `12` in the 2026 XC
district/state files (Oct–Nov 2026).

### Athletic.net leverage

* **Recent NE meets carry the AN MeetID directly.** In the 60 most recent NE meet docs (window
  2026-06-17 → 2026-10-15: 57 XC + 3 outdoor), **60/60 (100 %)** carry a numeric `ani`; each doc also carries
  `us` short links and (where the timer set it) `atr` "Timed by" HTML. [Sampled window, not a whole-corpus claim.]
* **Full Athletic.net meet URLs are exposed too**, via the `anis` array (`u: https://www.athletic.net/
  TrackAndField/meet/367852` style) on 98/528 unique NE docs in hand — that is a *link field*, not a request:
  the source hands over the AN URL, no Athletic.net call needed to obtain it.
* Whole-corpus `ani` coverage is lower in older data (of 575 unique NE `(tenant,i)` docs in hand across all
  years, 419 = 72.9 % have `ani > 0`; per tenant: hnscats 43/43, sub4sports 40/40, dakota 31/31, aatiming 12/12,
  runnebraska 9/9, greenferrettiming 113/121, athleticlive 89/106, blacksquirrel 56/180, wayzata 8/11,
  delta 17/20).
* **Meet-id resolution avoided:** 2,203 NE meets × (meet-search + resolve ≈ 1–3 AN requests) ≈ **2,200–6,600
  Athletic.net requests avoided**, one ES POST each direction. The AN meet ids are also the join key to the
  production pipeline's retained meet entities.
* Team/athlete links: not exposed — no `athletic.net/team|athlete` anchors in the ES fields or the white-label
  shells; athlete-level ids live in the Firebase RTDB / `athlete_list` path (verified in reports 12/25 by other
  agents; not re-verified here).
* **Timer professional contacts come free**: `qe` (and the `atr` mailto) yield 19 distinct timer addresses from
  528 NE meet docs — a byproduct the contact layer can use (see Recruiting information).

### Athlete evidence

| Source | name | grade | school | performances | notes |
|---|---|---|---|---|---|
| AthleticLIVE white-label meet page | yes (from Firebase/ES athlete docs — reports 12/25) | `y` field 9-12/SR-JR | yes | per-event rows | browser app to a plain client (50,184 B shell — verified 4×, see Access) |
| NSAA 2026 state TF finals PDF | `Name` | **`Year` column, verbatim `Leo Cauble 11 Millard West`** | yes | mark + wind + heat + attempt series | grade distribution in finals files (regex-counted): A&B 9:95 10:272 **11:457** 12:571; C&D 9:141 10:287 **11:419** 12:559 |
| NSAA 2025-season XC district HTML (current file until 2026-10-15) | `Name` | **`(NN)` suffix, verbatim `J'Shawn Afuh (11)`** | yes | time, top-15 | 8 files, one per class×gender |
| State XC (onlineraceresults, 2008–2025) | yes | **`Year` column** (report 24) | yes | full field + 1m/2m splits | host not re-probed this session (report-24 evidence) |
| MaxPreps NSAA feeds | top-5 names | **no grade** | yes | seeding times | report 24 |

### Recruiting information

**Direct answer to the open item: NSAA itself still publishes zero coach/AD emails** (re-verified locally over the
captured 312-school directory: 1,085,584 B contains exactly **one** `@` string — `Bridget-doyle@cdolinc.net` in a
*Principal* cell). The email field must come from school/district sites. Measured on 16 schools:

| School (class) | Platform | Requests | Pages that answered | Emails on pages (any) | Role-matched coach/AD email (verbatim) |
|---|---|---|---|---|---|
| Omaha Central (A) | Cornerstone | 4 | 200×4 | 0 | — |
| Lincoln Southwest (A) | Apptegy / LPS | 4 | 200×4 | 12 (staff page) | — (no coach/AD match in LPS staff render) |
| Millard West (A) | mpsomaha.org | 1 | **403** | 0 | — (access finding) |
| Kearney (A) | district CMS | 4 | 200×4 | 0 | — (routes to Bound + staff search) |
| Fremont (A) | Apptegy | 1 | 200 | 0 | — |
| Grand Island (A) | district CMS | 2 | 200×2 | 3 (`communications@gips.org`, `mfisher@gips.org`, + the federal `ocr.kansascity@ed.gov` boilerplate) | — (district office only) |
| **Papillion-La Vista South (A)** | PLCS (embedded sheet table) | 4 | 200×4 | 139 | **`kyle.mcmahon@plcschools.org`** (table label *BOYS TRACK / Head Varsity* — NSAA `tf_boys` = Kyle McMahon) and **`shannon.stenger@plcschools.org`** (*BOYS XC / Head Varsity* — NSAA `xc_boys` = Shannon Stenger) |
| **Elkhorn (B)** | WordPress+Elementor (CF-obfuscated) | 4 | 200×4 | 15 `data-cfemail` spans → 8 unique; 7 real `@epsne.org` addresses | **`sfjell@epsne.org`** ← *"Sara Fjell / Assistant Principal / Activities Director"* = NSAA AD "Sara Fjell"; also `lford@` (Luke Ford, AP/AD = NSAA AD of Elkhorn North), `rortmeier@` (Roger Ortmeier, AP/AD = NSAA AD of Elkhorn South), `canderson@`, `marchibeque@`, `jmckenzie@` (activities admin assistants) |
| **Gretna (B)** | Apptegy | 3 | 200×3 | 4 | **`joshua.gibbs@gpsne.org`** ← *"Activities Director/Assistant Principal — Josh Gibbs"* (NSAA AD = Josh Gibbs); also `srangel@gpsne.org` (Sarah Rangel, AD Secretary) |
| Scottsbluff (B) | SharpSchool | 4 | 200×4 | 0 | — (delegates to Bound) |
| Waverly (B) | Apptegy | 2 | 200×2 | 15 unique emails among 20 staff-directory entries — all district-office roles (Superintendent, Directors, Technology, Registrar, Admin Asst.) | — (no coach in the directory render) |
| **Seward (B)** | Apptegy | 4 | 200×4 | 37 | **`kurt.holliday@sewardschools.org`** ← activities-page table cell "Kurt Holliday" with `mailto:` (NSAA `xc_boys`/`xc_girls` = Kurt Holliday); also `aaron.blersch@` (Aaron Blersch), `krystin.cast@` (Krystin Cast) |
| Adams Central (C) | static site / GoDaddy | 4 | 200×4 | 1 (`neile.anderson@adams-central.org`) | — (appears in a **news item** — FBLA blood drive contact — not a staff directory) |
| Boone Central (C) | district CMS (GoDaddy) | 4 | 200×4 | 0 | — (staff directory lists coach *titles*, no emails) |
| Broken Bow (C) | Apptegy | 1 | 200 | 0 | — (links Striv + Bound) |
| Aurora (C) | Apptegy | 4 | 200×4 | 4 unique — all Apptegy footer role aliases (`Office-District@`, `Office-High@`, `Office-Middle@`, `Office-Elementary@4rhuskies.org`) | — (activity-filtered staff page renders no coach rows) |

Two artifact caveats the collector must apply: (a) `ocr.kansascity@ed.gov` (US Dept. of Education OCR) is
boilerplate on multiple district sites and inflates naive "any email" counts; (b) Apptegy pages repeat the same
footer aliases on every page, so unique-address counts are smaller than occurrence counts.

**Second, cheaper contact layer (from the ES docs, 0 extra hosts):** each NE meet doc carries the timer's
published contact — `qe` (non-empty in **219/528** unique docs, **19 distinct addresses**) and the same address
inside the `atr` "Timed by … email results questions to …" HTML. Distinct `qe` values observed: `info@` and
`ericwellman@`/`ewellman@blacksquirreltiming.com`, `huskytiming@4rhuskies.org`, `dfey@csm.edu`,
`fey.derek@westside66.net`, `dennis@heartlandtiming.com`, `clatta@dcstigers.org`, `dmaline@dcwest.org`,
`jbrockhaus@dcwest.org`, `jstewart@lpslions.org`, `bensigle@manhattanrunningco.com`, `jayson_bishop@hotmail.com`, …
These are **professionals published by the meet itself for results questions** (timer staff / athletics staff),
not athlete data — usable as a timing-provider contact field; the rate-limited `qe`/`atr` pair costs one ES POST
for the whole state rather than one crawl per provider.

Findings:
1. **Yield 4/16 = 25 %** of schools produced a *role-matched* professional coach/AD email (8 addresses total:
   2 PLCS head coaches, 3 Metro/EMC ADs, 1 Seward XC head, plus an AD secretary), each verified against the NSAA
   directory name; **8/16 = 50 %** published at least one staff email in plain page text, **9/16 = 56 %**
   including Elkhorn's Cloudflare-obfuscated contacts page.
2. **Cost per school: 2–4 requests** where the school has a directory (mean 3.1 in this sweep: 50/16); the yield
   page is almost always the *activities/athletics contacts* page (Elkhorn #2, Seward #2, Gretna #2) or the school
   athletics page (PLVS #4). Flat cost, no auth, no JS.
3. **Two decoders matter**: Apptegy embeds staff JSON in the served HTML (1 GET, no JS), and Cloudflare
   `data-cfemail` decode recovers Elkhorn-class obfuscated addresses (8 spans on the contacts page, 15 across the
   4 Elkhorn pages). Neither is an access-control bypass — both are page source the browser itself receives.
4. **Blocked/delegated cases:** Millard West origin 403s our client (0 emails obtainable); Scottsbluff/Kearney/
   Boone/Broken Bow/Aurora hand athletics off to Bound/Striv; OPS (`central.ops.org`) has **no** staff emails on
   the four pages reachable from the school site (district directory is a separate, larger job).
5. **Extrapolation for NE (312 directory entries, ~283 TF + ~253 XC participants) [INFERENCE]:** a full sweep at
   the measured rate ≈ **900–1,000 requests** (≈3.1/school) yielding ≈**70–80 role-matched coach/AD emails**
   (25 % rate) plus ≈**140–160 staff-directory addresses** (50 % plain-text rate); ~1/2 of schools will need a
   named-route follow-up (coach pages, paginated staff directories, PDFs) before any staff email is found. If the
   pipeline only needs *AD or athletics-office* mail, the same sweep is the cheapest route (an AD/AP-Activities
   address appeared on 3/16 in ≤4 requests — the highest-value field per request).
6. Assistant coaches remain unavailable from NSAA (one head per sport) and were not found on the probed school
   pages either (PLCS lists assistant/head per sport on its athletics page — that is the exception, not the rule).

### Result evidence

Field-level proof on **live 2026 files** (all fetched and parsed 2026-09-20):

* **`/textfile/track/abresults.pdf`** — 2026 Class A&B **finals**, 255,593 B, `Last-Modified: Fri, 22 May 2026
  15:17:50 GMT`, header `Black Squirrel Timing - Contractor License … Hy-Tek's MEET MANAGER 7:15 PM 5/21/2026`.
  Column header: `Name  Year School Seed Finals Wind H# Points`.
  Verbatim: `  1 Ethan Laux                12 Creighton Prep         14.28      14.05   1.4 10` (grade 12, wind 1.4);
  field final `  1 Alexa Jacobsen            12 Kearney             40-11.50   43-01.00   3 10` + attempt series
  `43-01 42-04.50 41-02 FOUL 41-04.75 40-09.75`; relay legs with per-leg grades
  `1) Marianne Forney 10   2) Paige Dillon 12   3) Lucy Peterson 11   4) Mabel Henningsen 10`.
* **`/textfile/track/cdresults.pdf`** — 2026 Class C&D finals, 260,492 B, `Last-Modified: Mon, 22 Jun 2026
  16:19:29 GMT`. Verbatim: `  1 Maizie Stoklasa            11 Clarkson-Leigh       37-07.00   38-11.50  NWI 3 10`
  (grade 11; `NWI` = no wind information, i.e. the wind field is explicit even when absent); relay
  `Loomis 52.04 51.47 2 3` + `1) Adrianna Thorell 10  2) Kennedy Wood 10 …`.
* **`/textfile/track/abres26.pdf`** — 2026 A&B per-day file, 142,365 B, `Last-Modified: 2026-05-28`, ETag
  `"6d61893009c2e2f1acc5a6529bb99e08"`, `x-amz-meta-sha256: adbc7cca87f415e065b2f8ee568cef61e584c42aaae0789e533fdc5f9bcbafeb`
  (object-level sha256 published by S3 — free integrity check for the collector). Row:
  `  3 Leo Cauble                 11 Millard West          22-11.00   0.5 3 6` + `22-11(0.5) 21-04.25(+0.0) …`.
* **Grades present in every one of the three 2026 files.** Re-counted ≈09:17 with the reproducible rule
  `^\s*\d+\s+<name>\s+(9|10|11|12)\s+[A-Z]` (place, name, grade, school-initial): **abresults 1,395 rows**
  (9:95, 10:272, **11:457**, 12:571), **cdresults 1,406 rows** (9:141, 10:287, **11:419**, 12:559),
  **abres26 395 rows** (9:16, 10:74, **11:117**, 12:188); girls per-day `agres26` 386 rows (11th: 135). These are
  lower bounds on true row counts (names >30 chars and non-ASCII spellings fall outside the rule). No separate
  grade file is needed; grade rides on each result row.
* **`/textfile/track/unifiedresults.pdf`** (178,012 B) is the exception: unified pairs print as
  `Kynlee, Pollock / Tatiana, Dillard  Norfolk Senior High 29.06 1/2` — **no grade, no points**.
* **XC district HTML** (`/textfile/cc/ccbClassAResults.html`, still the 2025-season object on 2026-09-20:
  `Last-Modified: Thu, 16 Oct 2025 00:10:06 GMT`, ETag `"d11cbaa485cefc0cd42efde78e44dc52"`) — cells are
  `J'Shawn Afuh (11) \| Lincoln North Star \| 15:50.75`; the same 8 paths are the 2026 contract
  (`cc{b,g}Class{A,B,C,D}Results.html`), rewritten the day of each 2026-10-15 district meet. Until then the
  2026-season XC result surface is **regular-season only** (AthleticLIVE tenants + MaxPreps).

### Incremental use

1. **Weekly NE meet delta — 1 POST/wk:** `{"size":500,"query":{"bool":{"filter":[{"match":{"lsa":"Nebraska"}},
   {"range":{"sdy":{"gte":"<last_run>","lte":"<today>"}}}]}},"sort":[{"sdy":"desc"}],"track_total_hits":true}`
   → new/changed meets with `i`, `ani`, tenant, timer; dedupe on `(e,i)` (the union index double-counts).
2. **NSAA objects — conditional GET only** (verified headers this session): `abresults.pdf`, `cdresults.pdf`,
   `abres26.pdf`, the 8 XC district HTML files, classification PDFs. A 304 costs nothing; XC files turn over in a
   single-day window (2025 file `Last-Modified: 2025-10-16`), state TF finals in May.
3. **School coach/AD mail — quarterly, per-school diff:** re-GET the activities-contacts/staff page per school
   (~3 requests/school), hash the page, diff the decoded email set (Elkhorn-class pages) and the embedded JSON
   (Apptegy-class pages). No change feed exists for any NE school site (nor for NSAA, which the director re-types).
4. **Postseason windows:** 2026 XC — poll the 8 S3 HTML files + `onlineraceresults` group 33 for the new
   `event_id` (state meet 2026-10-23) + the MaxPreps feeds; 2027 TF — `/track-field/` rotates its 28 Athletic.net
   district links and adds new `abresults/cdresults` objects.

### Access characteristics

| Host | Class | Observed this session |
|---|---|---|
| `search.athletic.live` (ES) | public structured JSON, undocumented | 200 to anonymous POST; `.keyword` needed for aggs (verbatim ES error captured); 12 requests, no throttle |
| `nsaa-static.s3.amazonaws.com` | static PDF/HTML, public bucket | 200/304-capable; `ETag` + `Last-Modified` on every object, `x-amz-meta-sha256` on **3 of the 4** 2026 TF files (`cdresults.pdf` lacks it, as do the XC HTML files) |
| `nsaahome.org` | normal HTML via reader; curl 403 (report 24) | 200 reader; the two sport pages carry all file links |
| `results.blacksquirreltiming.com`, `results.hnscats.org`, `live.sub4sports.com`, `results.run-ne.com` | AthleticLIVE white-label SPA | 200, **identical 50,184-byte Angular shell** — data comes from ES/blob/Firebase, so this status proves the tenant is live, not that results are parseable from HTML |
| `dakotatiming.anet.live`, `greenferret.anet.live` | same white-label family (`*.anet.live`) | 200 (short-link redirects land here) |
| `www.dakotatiming.com` | timer's own site | **404** at `/meets/<i>` — do not use the timer domain for meet pages |
| `www.aurorahuskies.org/page/husky-timing` (Husky Timing) | Cloudflare JS challenge | 200 but body = "Client Challenge … enable JavaScript" (3,038 B) — the school homepage served fine (1.4 MB), only this page is challenged |
| `ahs.aurorahuskies.org/athletics/#1522160153899-7429b136-6b41` (exact ES `tu` value, 104 docs) | normal HTML | referenced by `tu`; not re-fetched (challenge risk). Other school-run timers in `tu`: `sandycreek.us` (22 docs) |
| `striv.tv` | normal HTML (WordPress), video only | 200; `www.` → 301; channel pages carry Events/Scores sections but for Broken Bow **no scores in the past six weeks** and no track results |
| `mwhs.mpsomaha.org` | blocked to our client | **403** (5,621 B) |
| `onlineraceresults.com` / `precisionraceresults.com` | normal HTML | report-24 evidence (state XC 2008–2025; timing credit "Precision Race Results") — not re-probed |
| prepcast name-space | **does not exist as a NE sports property** | see below |

**`strivtv` / `prepcast` resolution:**
* **Striv (`striv.tv`) = NE-specific live-streaming platform, not a results host.** Homepage re-fetched 2026-09-20:
  200, 66,523 B, containing exactly **25 channel links** (24 school/program channels — Alma, Arlington, Bergan,
  Broken Bow, Columbus, Crete, Elmwood-Murdock, EMF, Freeman, Gibbon, Heartland, JCC, Kearney, Lakeview,
  Lincoln Christian, McCool Junction, Milford, Northwest, Omaha Central, Omaha North, Osceola, Silver Lake, Sutton,
  Waverly — plus the platform's own `striv-sports`) + app/Roku distribution; the upcoming-events strip on
  2026-09-20 listed volleyball/softball/football only; the channel page's Scores block is empty. Use it as a
  **stream link field** for school records, never as a results source.
* **No `prepcast` sports property exists.** DNS NXDOMAIN for `prepcast.tv`, `prepcast.live`, `prepcast.co`,
  `.io`, `.net`, `.org`, `.us`, `prep-cast.com`, `goprepcast.com`, `prepcastusa.com`, `prepcastsports.com`,
  `nepreps.com`. The resolvable names are unrelated entities: `prepcast.app` = restaurant-prep forecasting SaaS;
  `theprepcast.com` = a food podcast; `prepcast.com` = a 200-returning but title-less 27 KB page (and a timeout on
  the second attempt); `nebraskapreps.com`/`nebpreps.com` → Hurrdat Sports' "NebPreps" media site (articles, not
  results); `strivsports.com` = an unrelated Korean streaming aggregator. Record: **the `prepcast` lead is a
  non-existent provider; do not allocate a collector slot for it.**

### Recommendation

**Split verdict — the open items close as follows:**

1. **Coach/AD email (NE): `COACH-DIRECTORY` via school sites, 25 % direct yield at ≈3 requests/school — no
   association-level shortcut.** NSAA stays name-only (0 emails, re-verified). Build the mail field as a
   per-school crawl of the *activities contacts* page (Apptegy JSON + Cloudflare `data-cfemail` decode), keyed by
   the NSAA school name; budget ≈900–1,000 requests for all 312 schools, or ≈150-ish requests for the ~50 largest
   (classes A/B) where coach email density is highest. Treat Bound (`gobound.com/ne/schools/<slug>`) as the
   schedule layer, not a mail source. Add the free ES `qe`/`atr` timer-contact layer (19 distinct professional
   addresses, one POST) to the same contact table.
2. **Grade exposure: confirmed on live 2026 NSAA files.** State TF finals (`abresults.pdf`, `cdresults.pdf`)
   carry `Year` (grade) on every row plus wind, heat and relay-leg grades; XC district HTML carries `(grade)` on
   the top-15; state XC plain text carries `Year` on the full field. Unified results are the one grade-less file.
   Keep the NSAA S3 mirror as the `RESULT-SOURCE` for NE postseason; the 2026 XC files are the next turnover
   (2026-10-15).
3. **Timer ecosystem: `RESULT-SOURCE` via AthleticLIVE ES is now the NE regular-season index.** One POST returns
   every NE meet and its timer (2,203 meets, 130 timer names, 34 hosts); the recent window is **100 %
   `ani`-linked**, so NE meet identity → Athletic.net MeetID costs zero Athletic.net requests. Primary providers
   to wire first: Black Squirrel Timing (945), Sub4Sports (40 + PAL series), Dakota Timing (31, NE/IA/SD corner),
   HNS Cats (43), Green Ferret (121), Delta Timing (indoor 20), AA Timing (12), Run Nebraska (9), plus the
   long tail of school-run timers (LPS/OPS/Husky/Malcolm/Cozad/Cardinal/Fort Calhoun/LongHorn/Eagle Eye) that ride
   the generic `athleticlive` tenant and publish only via that SPA.
4. **Do not build: `striv.tv` (stream links only) and `prepcast` (does not exist).** MaxPreps feeds stay the
   `DISCOVERY-ONLY` XC seeding/early-season surface from report 24.
5. **Blocked routes to note in the risk register:** Millard West (`mpsomaha.org`) 403s non-browser clients;
   Aurora's Husky Timing page sits behind a Cloudflare JS challenge; `www.dakotatiming.com` 404s on `/meets/<i>`
   (use `dakotatiming.anet.live`).

### Evidence appendix

All times CDT (UTC-5), 2026-09-20. Method: `curl` = `/usr/bin/curl` with browser UA
`Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36`;
`reader` = harness reader fetch; `py` = local parse over saved bytes. Timestamps marked `≈` are reconstructed to
the nearest minute from capture mtimes/job logs (the school-sweep ledger `schools/probe_results.json` is mtime
09:10:40, so the 16-school sweep spans ≈09:08–09:11 local); unmarked ones are from `mtime`/response headers.
Re-verification rows (09:15–09:19) were re-run while drafting this report and their saved outputs are the
`*_2026-09-20`/`es_ne_card*`/`striv_home.html` artifacts. The 16-school sweep is grouped by school below —
the **complete per-request ledger (50 rows: tag, URL, final URL, status, bytes, seconds)** is
`research/midwest/evidence/gaps/42/schools/probe_results.json`.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| **Block A — NSAA S3 live 2026 result files (Task 2)** | | | | |
| `https://nsaa-static.s3.amazonaws.com/textfile/track/abres26.pdf` | curl GET | 200 (142,365 B) | 2026 A&B per-day results; **re-verified headers 09:15** → `Last-Modified: Thu, 28 May 2026 19:27:37 GMT`, ETag `"6d61893009c2e2f1acc5a6529bb99e08"`, `x-amz-meta-sha256: adbc7cca…bafeb`, `Accept-Ranges: bytes` | 09:05:29 (GET) / 09:15 (HEAD) |
| `https://nsaa-static.s3.amazonaws.com/textfile/track/agres26.pdf` | curl GET | 200 (141,343 B) | 2026 girls A&B per-day results exist and parse | 09:05:29 |
| `https://nsaa-static.s3.amazonaws.com/textfile/track/abres26.pdf` | curl HEAD | 200 | Object headers: ETag, Last-Modified, `x-amz-version-id`, AES256 | 09:05:3x |
| `https://nsaa-static.s3.amazonaws.com/textfile/track/abresults.pdf` | curl GET + HEAD | 200 (255,593 B) | 2026 **final** A&B results; `Last-Modified: Fri, 22 May 2026 15:17:50 GMT`, ETag `"f099c19cb93eaad02fa364e17888b220"`, sha256 meta present | 09:05:42 (GET) / 09:15 (HEAD) |
| `https://nsaa-static.s3.amazonaws.com/textfile/track/cdresults.pdf` | curl GET + HEAD | 200 (260,492 B) | 2026 **final** C&D results; `Last-Modified: Mon, 22 Jun 2026 16:19:29 GMT`, ETag `"76211856fab9b67228df33e00b740b87"`, **no `x-amz-meta-sha256`** | 09:05:44 (GET) / 09:15 (HEAD) |
| `https://nsaa-static.s3.amazonaws.com/textfile/track/unifiedresults.pdf` | curl GET + HEAD | 200 (178,012 B) | Unified pairs — **no grade column**; `Last-Modified` identical to `abresults.pdf` (same publish batch), ETag `"55d017a63617605cd2a710e464c6d621"` | 09:05:45 (GET) / 09:15 (HEAD) |
| `https://nsaa-static.s3.amazonaws.com/textfile/cc/ccbClassAResults.html` | curl HEAD | 200 (16,301 B) | XC district file **still the 2025-season object** (`Last-Modified: Thu, 16 Oct 2025 00:10:06 GMT`) → 2026 XC results not yet posted | 09:12:20 |
| local: `pdftotext -layout` over the 4 PDFs | py | n/a | Grade (`Year`), wind, `H`, relay-leg grades; re-counted rows (abresults 1,395 / cdresults 1,406 / abres26 395 / agres26 386) | 09:05–09:17 |
| `tools/ne/ccbClassAResults.html` (2025-season capture, local) | py | n/a | Verbatim XC district row `J'Shawn Afuh (11) \| Lincoln North Star \| 15:50.75` | 09:10 |
| `tools/ne/nsaa_directory_all_2026-09-19.html` (local) | py | n/a | NSAA directory contains **exactly 1** email string in 1,085,584 B (`Bridget-doyle@cdolinc.net`, Principal) | 09:10 / 09:17 |
| **Block B — NSAA sport pages (file inventory, Task 2/3)** | | | | |
| `https://nsaahome.org/track-field/` | reader | 200 | 2026 finals = `abresults.pdf` / `cdresults.pdf`; 28 Athletic.net district links; state TF live `bst.anet.live/oolpf1` + `/aetvft`; gold-medal PDF on `blacksquirrelresults.com` | ≈09:04 |
| `https://nsaahome.org/cross-country/` | reader | 200 | 8 XC district HTML paths (`cc{b,g}Class{A..D}Results.html`); 2025 state races `race_id=79870–79877`; 4 MaxPreps feeds; state history 1960/1980→2025 | ≈09:14 |
| **Block C — AthleticLIVE ES (Task 3)** | | | | |
| `POST https://search.athletic.live/*_meet_list/_search` (lsa:Nebraska count) | curl POST | 200 | **4,520** NE docs across 256 tenant indices | 09:06:02 |
| same, 500 docs asc | curl POST | 200 (350,881 B) | Meet docs carry `i/ani/e/tna/tu/us/ls/lsa/sdy/o` | 09:06:03 |
| same, terms aggs without `.keyword` | curl POST | 200 (250/256 shards failed) | ES refuses aggs on text fields → must use `.keyword` | 09:06:09 |
| same, aggs with `.keyword` | curl POST | 200 | Tenant/timer-name/timer-URL buckets | 09:06:10 |
| same, non-major tenants (289 docs) | curl POST | 200 | Per-provider NE counts + examples (hnscats 43, sub4sports 40, dakota 31, delta 20, aatiming 12, runnebraska 9, wayzata 11, greenferrettiming 121, reddirt 1, heros 1) | 09:06:23 |
| `POST https://search.athletic.live/*/_search` (index agg) | curl POST | 200 (296 indices) | Tenant universe; no `striv`/`prep` tenant exists | ≈09:07 |
| `POST …/live_results_meet_list/_search` `{"term":{"i":755}}` | curl POST | 200 (total=1) | `live_results` is a **union mirror** of tenant indices (dedupe by `i`) | 09:09:16 |
| `POST …/athleticlive_meet_list/_search` `{"term":{"i":755}}` | curl POST | 200 (total=0) | `athleticlive` is a distinct tenant, not a mirror | 09:09:17 |
| `POST …/*_meet_list/_search` recent desc (120 docs) | curl POST | 200 | 2026-06-17→2026-10-15 window: 60 unique meets, **60/60 with `ani`**, 57 XC / 3 outdoor | 09:09:15 |
| same, cardinality + sport aggs (kwargs) | curl POST | 200 | **2,203 distinct meets**, **130 timer names**, **34 timer URLs**, outdoor 3,043 / xc 994 / indoor 483 docs; `years` terms-agg returned **empty** on `sdy.keyword` → use a `range` filter on `sdy`/`md` instead | 09:12:34 |
| same, **named-cardinality re-verify** (`meets`=`i`, `timer_names`=`tna.keyword`, `timer_urls`=`tu.keyword`), request body saved as `es_ne_card_request.json` | curl POST | 200 | Confirms 2,203 / 130 / 34 on explicitly named fields, 256 shards 0 failed → the headline numbers are field-specific, not `_index` artefacts | 09:15 |
| same, school-timer phrase query | curl POST | 200 | Examples for Cozad (i=70833), Cardinal (i=70557), Fort Calhoun (i=65885), LongHorn (i=72746), Eagle Eye (i=72408) | 09:13:14 |
| local: unique-doc field inventory over `es_ne_docs.json` + `es_ne_docs_other_tenants.json` (528 unique `(e,i)`) | py | n/a | Field set is exactly `tu tna qe ls lsa sdy md i n o e us ani anis atr`; `qe` non-empty 219/528 (19 distinct), `anis` present 98/528 (max 1 entry) → timer contact emails + AN meet URLs come free with the meet index | ≈09:18 |
| **Block D — timer white-label hosts (Task 3)** | | | | |
| `https://results.blacksquirreltiming.com/meets/77526` | curl GET | 200 (50,184 B) | BST tenant live (2026 Bennington Invite; AN id 279185); SPA shell only | 09:12:07 |
| `http://results.hnscats.org/meets/76939` | curl GET | 200 → https (50,184 B) | HNS Cats tenant live (Niobrara-Verdigre Invite; AN id 282694) | 09:12:09 |
| `https://live.sub4sports.com/meets/77691` | curl GET | 200 (50,184 B) | Sub4Sports tenant live (PAL XC Memorial Park; AN id 280156) | 09:12:10 |
| `https://www.dakotatiming.com/meets/76043` | curl GET | **404** (3,311 B) | Timer's own domain does **not** serve meet pages | 09:12:12 |
| `http://dakota.anet.live/2kgv3c` | curl GET -L | 200 → `https://dakotatiming.anet.live/meets/31873` | Correct Dakota Timing meet host = `dakotatiming.anet.live` | 09:12:5x |
| `http://anet.live/o8puob` | curl GET -L | 200 → `https://greenferret.anet.live/meets/32280` | Correct Green Ferret meet host = `greenferret.anet.live` | 09:12:5x |
| `https://results.run-ne.com/meets/71180` | curl GET | 200 (50,184 B) | Run Nebraska tenant live (Central City Invite; AN id 636173) | 09:12:13 |
| `https://www.aurorahuskies.org/page/husky-timing` | curl GET | 200 (3,038 B, **Cloudflare JS challenge body**) | Husky Timing page is bot-challenged; school homepage unaffected | 09:12:03 |
| (examples per provider, ES-derived — hosts listed above) | ES | n/a | BST: i=77526/77527/77522/77520 · Green Ferret: i=77503/74895/74947/9770 · HNS: i=76939/70633/70632/33184 · Sub4: i=77691/77687/76546/76086 · Dakota: i=76043/76026/76000/31873 · Delta: i=43958/43597/43271/12495 · AA: i=72830/62539/62631/24392 · RunNE: i=71180/74686/70627/57943 · Wayzata: i=51381/37340/35801/1636 · Husky: i=77353/76659/13118/13117 · LPS: i=76505/76504/76503 · OPS: i=77239/76695/5617 | 09:06–09:13 |
| **Block E — striv.tv / prepcast (Task 3 leads)** | | | | |
| `https://striv.tv/` | reader + curl | 200 (66,502 B) | NE live-streaming platform; events strip = volleyball/softball/football | ≈09:04 / 09:07 |
| `https://striv.tv/` (re-fetched, saved as `striv_home.html`) | curl GET | 200 (66,523 B captured) | **exactly 25 `striv.tv/channel/<slug>/` links** — the 24 school channels + `striv-sports`; no track/XC results surface | 09:16 |
| `https://www.striv.tv/` | curl GET | 301 → `https://striv.tv/` | Canonical host | 09:07 |
| `https://striv.tv/channel/broken-bow/` | curl GET | 200 (55,846 B) | Channel page: "no events in the next six weeks", "no scores from the past six weeks" → **no track/XC results** | 09:12:06 |
| `prepcast.tv`, `prepcast.live`, `prepcastsports.com` | curl GET | 000 (exit 6, NXDOMAIN) | No such hosts | 09:07 |
| `https://prepcast.app/` | curl GET | 200 (16,866 B) | "Restaurant forecasting and daily prep planning" — unrelated | 09:07:39 |
| `https://theprepcast.com/` | curl GET | 200 (484 B) | "A clean, simple food podcast" — unrelated | 09:07:40 |
| `https://prepscast.com/` | curl GET | 000 | Unreachable | 09:07 |
| `https://strivsports.com/` | curl GET | 200 (141,551 B) | Korean streaming aggregator — unrelated | 09:07 |
| `https://prepcast.com/` | curl GET ×2 | 200 (27,580 B, title-less) then 000 (timeout) | Not a NE sports property | 09:07 / 09:08 |
| `https://nebpreps.com/` | curl GET -L | 200 → `hurrdatsports.com/nebpreps/` | NebPreps = Hurrdat Sports media, not a timer | 09:07:57 |
| DNS sweep (`prepcast.co/.io/.net/.org/.us`, `prep-cast.com`, `goprepcast.com`, `prepcastusa.com`, `nepreps.com`) | getent | n/a | NXDOMAIN — no prepcast property exists | 09:07 |
| `html.duckduckgo.com`, `mojeek.com`, `lite.duckduckgo.com` (search fallbacks) | curl GET | 202 / 200 / 202 | Search engines challenge/blank; no external search used | 09:07 |
| **Block F — school sites (Task 1; 50 requests; per-school detail in `schools/probe_results.json`)** | | | | |
| `central.ops.org` + `/activities-athletics/…` ×3 | curl GET | 200×4 (92,786 / 55,578 / 56,081 / 58,274 B) | OPS school pages: **0 emails**; district directory not on this host | 09:08 |
| `lsw.lps.org` + `/o/lsw/page/{academicsactivities,clubs-and-activities-calendar,staff}` | curl GET | 200×4 (1.51–1.58 MB) | LPS staff render exposes 12 emails; none match NSAA coaches/AD | 09:08–09:09 |
| `mwhs.mpsomaha.org` | curl GET | **403** (5,621 B) | Millard Public Schools blocks our client | 09:09 |
| `kearneypublicschools.org` + 3 subpages | curl GET | 200×4 | 0 emails; routes to Bound + a staff search page | ≈09:09 |
| `fremonttigers.org` | curl GET | 200 (1.24 MB) | Apptegy shell; 0 emails on page 1 | ≈09:09 |
| `gips.org` + `/about-us/contact-us` | curl GET | 200×2 | 3 email strings: district office (`communications@gips.org`, `mfisher@gips.org`) **plus the federal `ocr.kansascity@ed.gov` OCR boilerplate** — filter it | ≈09:09 |
| `plshs.plcschools.org` + `/student-life/{athletics,activities-clubs,athletics/live-stream}` | curl GET | 200×4 (70,673 / 61,873 / 56,677 / 39,761 B) | **139 unique emails**; the embedded sheet table labels each entry by sport+role → `kyle.mcmahon@plcschools.org` = "BOYS TRACK / Head Varsity" and `shannon.stenger@plcschools.org` = "BOYS XC / Head Varsity", both matching the NSAA directory | 09:09 |
| `elkhornweb.org/ehs/` (home) + `/parents/activities/{activity-contacts,activity-pass,athletics-registration}/` | curl GET | 200×4 (257,922 / 169,447 / … B) | Contacts page holds **8 `data-cfemail` spans (15 across the 4 pages)**; decoded → `sfjell@` (Sara Fjell, Assistant Principal/Activities Director = NSAA AD), `lford@` (Luke Ford AP/AD), `rortmeier@` (Roger Ortmeier AP/AD), `canderson@`, `marchibeque@`, `jmckenzie@`, `studentservices@epsne.org` (+ the OCR boilerplate) — **AD email published** | 09:09 |
| `ghs.gpsne.org` + `/o/ghs/page/athletics` + `/o/ghs/staff` | curl GET | 200×3 (1.37–1.43 MB) | **AD email `joshua.gibbs@gpsne.org`** appears in the **athletics page** body next to "Activities Director/Assistant Principal — Josh Gibbs" (not in the staff render); plus `srangel@gpsne.org` (AD Secretary) | 09:09 |
| `shs.sbps.net` + 3 paths (≤`/activities`) | curl GET | 200×4 | 0 emails; `/activities` redirects to `gobound.com/ne/schools/scottsbluff` | ≈09:09 |
| `district145.org` + `/staff` | curl GET | 200×2 (1.99 / 2.03 MB) | 15 unique staff emails, all district-office roles (Superintendent Cory Worrell, Directors, Registrar, Technology) — **no coach** on the page reached | 09:09 |
| `sewardpublicschools.org` + `/page/{activities,athletics,staff}` | curl GET | 200×4 | Activities page table has `mailto:` per coach → `kurt.holliday@sewardschools.org` ("Kurt Holliday", NSAA `xc_boys`/`xc_girls`), `aaron.blersch@`, `krystin.cast@` | 09:09 |
| `adamscentral.us` + `/staff`, `/staff-directory.html`, `/contact/` | curl GET | 200×4 | 1 email (`neile.anderson@adams-central.org`) — a **news-item contact** (FBLA blood drive), not a staff directory; site links Bound | ≈09:09 |
| `boonecentral.org` + 3 paths | curl GET | 200×4 | Staff directory lists coach *titles*; **0 emails**; links Bound + `manager.gobound.com` facilities | ≈09:09 |
| `bbps.org` | curl GET | 200 (1.36 MB) | 0 emails; links `striv.tv/channel/broken-bow` + `gobound.com/ne/schools/brokenbow/calendar` | ≈09:09 |
| `aurorahuskies.org` + `/page/{student-activities,athletics-info}` + `/staff?filter_ids=125870` | curl GET | 200×4 | 4 unique emails — Apptegy footer role aliases (`Office-{District,High,Middle,Elementary}@4rhuskies.org`); no coach rows; links `gobound.com/ne/schools/aurora/{calendar,tickets}` | ≈09:09 |
| local decoders (`schools/*.html`) | py | n/a | Elkhorn cfemail decode → 7 `@epsne.org` addresses with name+role context; email/role matching vs the NSAA directory (`tools/ne/ne_schools_2026.csv`) → 4 schools with role-matched coach/AD mail; per-school tables in `schools/coach_email_matches*.json`, request ledger in `schools/probe_results.json` (50 requests, 49×200 + 1×403) | ≈09:11 / 09:17 |
| local: Bound-link scan over all 50 school captures | py | n/a | **10/16** schools embed `gobound.com` links (school or district URL) — Adams Central, Aurora, Boone Central, Broken Bow, Elkhorn, Gretna, Kearney, Omaha Central, Scottsbluff, Seward | ≈09:17 |
| **Block G — politeness/aggregate** | | | | |
| — | — | — | ≈106 requests total this session; max per host 12 (`search.athletic.live`); no 429 / `Retry-After` / CAPTCHA / auth prompt anywhere; the only 403 is Millard West's origin | 09:04–09:19 |

Session artifacts (read-only inputs for Main, all inside the campaign folder):
`research/midwest/evidence/gaps/42/` — `live_abres26.pdf/.txt`, `live_agres26.pdf/.txt`, `live_abresults.pdf/.txt`,
`live_cdresults.pdf/.txt`, `live_unifiedresults.pdf/.txt`, `es_ne_count.json`, `es_ne_docs.json`,
`es_ne_docs_other_tenants.json`, `es_ne_agg1.json` (verbatim `.keyword` failure), `es_ne_agg2.json`,
`es_ne_final_aggs.json`, `es_ne_card_request.json` + `es_ne_card.json` (named-cardinality re-verify),
`es_ne_recent.json`, `es_ne_schooltimers.json`, `es_dedup_lr.json`, `es_dedup_al.json`,
`striv_home.html`, `striv_broken_bow.html`, `s3_headers_2026-09-20.txt`, `aurora_husky_timing.html`,
`bst_meet_77526.html`, `hnscats_76939.html`, `sub4_77691.html`, `dakota_76043.html`, `runne_71180.html`,
`home_prepcast.app.html`, `home_theprepcast.com.html`, `home_strivsports.html`, `mojeek_prepcast.html`,
`ddg_prepcast.html`, `ddglite.html`, `tmp_a79552.html`, `tmp_af1f76.html` (NebPreps),
and `schools/` (50 HTML captures + `probe_results.json`, `coach_email_matches.json`, `coach_email_matches2.json`).
Script: `tools/ne42_probe_schools.py`.
