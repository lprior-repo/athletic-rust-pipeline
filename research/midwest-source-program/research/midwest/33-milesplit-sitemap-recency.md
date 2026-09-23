# 33. MileSplit sitemap recency semantics + re-pull harness (C6/C2)

Status: complete
Observed on: 2026-09-20 (America/Chicago); pulls at 09:05 CDT (t0) and 09:25 CDT (t1)

Answers the C6 register row ("Sitemap 7,500-cap semantics: rolling recency vs hard cap", `[16][27]`)
and advances C2 (Athletic.net join) by fixing what the sitemap can and cannot deliver a collector.

Method note: the register row asked for "2 pulls over a week". A literal 7-day wait is impossible in
this session, so recurrence is settled from three measurements instead — the requested 20-minute paired
pull (t0/t1), real ~9 h 40 m cross-day diffs against prior-day captures already on disk, and a
structural test of the ordering rule. A literal week-apart diff remains unrun (see *Incremental use*).

---

### Source

MileSplit (FloSports) — `https://www.milesplit.com/sitemap.xml` and one sitemap per state
subdomain, `https://<st>.milesplit.com/sitemap.xml`. Thirteen hosts were tested (www + all twelve
mission states: WI, MN, IA, IL, MI, IN, OH, MO, KS, NE, ND, SD); all thirteen answered 200 with a
flat `<urlset>` of **exactly 7,500** `<loc>`/`<lastmod>` pairs.

### Coverage

- **Content type: athlete profile URLs only.** Across the 13 hosts, **97,500 / 97,500 URLs are
  `/athletes/…`** — zero `/teams/`, zero `/meets/`, zero non-athlete paths. The sitemap is not a
  site index; it is a per-host athlete-page recency feed.
- **States: all 12 mission states have a working sitemap** (each ≈916–919 KB, 7,500 entries).
- **One host per athlete record.** Across the 13 hosts × 7,500 slots = **97,500 URLs, all 97,500
  AthleteIDs are distinct; 0 of the 78 host pairs share an ID**. `www` is therefore a *complement*
  namespace, not a superset: sampled `www` rows resolve to jurisdictions with no state subdomain
  (Jackson-Reed HS, Washington DC ×2; White Rock Christian Academy, Surrey BC, Canada). For a
  12-state Midwest collector the `www` window is mostly out-of-region; it can be skipped or pulled
  at low frequency.
- **Sports/seasons: not encoded.** The URL carries only an AthleteID (+slug); `lastmod` is the only
  temporal field and it describes *page rewrite* time, not season, event or result date. Indoor /
  outdoor / XC / grade cannot be derived from the sitemap.
- **Historical depth: none.** No sitemap index (`/sitemap_index.xml` → 404 on `wi` and `www`), no
  pagination (`?page=2` is byte-identical: same sha256 as `/sitemap.xml`, 918,197 B), no news/team/
  alternate sitemaps (`/news-sitemap.xml`, `/sitemap/`, `/team-sitemap.xml` → 404). The 7,500-entry
  window is the *entire* sitemap surface for a host.

### Enumeration

```
GET https://<host>/sitemap.xml        # host = www | wi | il | oh | mn | in | mi | ia | ks | ne | nd | sd | mo
```
- HTTP 200, `content-type: text/xml;charset=UTF-8`, `server: nginx`, ~0.5 s, 915,859–930,488 bytes.
- Response is `<urlset>` with **7,500** `<url><loc>…</loc><lastmod>…</lastmod></url>` entries,
  ordered **newest-first** (verified: the lastmod sequence is non-increasing on every host pulled).
- **URL forms:** mostly `https://<host>/athletes/<AthleteID>-<slug>`, but **bare-ID form
  `https://<host>/athletes/<AthleteID>` also occurs** — measured in this pull: `wi` 5, `il` 36,
  `oh` 2, `mn` 1, `www`/`in`/`mi` 0. A parser that requires a slug will silently drop those rows.
  The bare form resolves: `https://wi.milesplit.com/athletes/15289196` → 200, 84,748 B, canonical
  tag `https://wi.milesplit.com/athletes/15289196`.
- **Enumeration ≠ census.** The only way to widen coverage is the union of repeated pulls over time
  (see *Incremental use*); there is no backfill path.

Measured windows (first pull, 09:05–09:09 CDT), ordered by window reach:

| host | entries | oldest (`lastmod` floor) | newest (`lastmod` ceiling) | reach (h) | ceiling lag at 10:05 EDT (h) | entries stamped "today" |
|---|---|---|---|---|---|---|
| `www` | 7,500 | 2026-09-20T02:11:24-04:00 | 2026-09-20T04:12:39-04:00 | 2.0 | 5.9 | 7,500 (100 %) |
| `oh` | 7,500 | 2026-09-20T01:59:12-04:00 | 2026-09-20T03:32:57-04:00 | 1.6 | 6.5 | 7,500 (100 %) |
| `in` | 7,500 | 2026-09-20T01:53:38-04:00 | 2026-09-20T03:59:45-04:00 | 2.1 | 6.1 | 7,500 (100 %) |
| `mo` | 7,500 | 2026-09-20T01:45:06-04:00 | 2026-09-20T04:51:54-04:00 | 3.1 | 5.2 | 7,500 (100 %) |
| `wi` | 7,500 | 2026-09-12T11:55:37-04:00 | 2026-09-20T05:02:52-04:00 | 185.1 | 5.0 | 4,541 (60.5 %) |
| `il` | 7,500 | 2026-09-10T19:14:13-04:00 | 2026-09-20T03:58:08-04:00 | 224.7 | 6.1 | 4,562 (60.8 %) |
| `mi` | 7,500 | 2026-09-10T18:07:25-04:00 | 2026-09-20T03:02:28-04:00 | 224.9 | 7.0 | 3,250 (43.3 %) |
| `ks` | 7,500 | 2026-09-04T16:14:49-04:00 | 2026-09-20T02:29:46-04:00 | 370.2 | 7.6 | 4,894 (65.3 %) |
| `ia` | 7,500 | 2026-08-30T04:11:37-04:00 | 2026-09-20T02:29:44-04:00 | 502.3 | 7.6 | 3,166 (42.2 %) |
| `mn` | 7,500 | 2026-07-14T07:22:11-04:00 | 2026-09-20T02:48:56-04:00 | 1,627.4 | 7.3 | 2,177 (29.0 %) |
| `ne` | 7,500 | 2026-07-14T07:22:11-04:00 | 2026-09-20T02:29:42-04:00 | 1,627.1 | 7.6 | 395 (5.3 %) |
| `sd` | 7,500 | 2026-07-14T07:22:11-04:00 | 2026-09-20T02:29:42-04:00 | 1,627.1 | 7.6 | 196 (2.6 %) |
| `nd` | 7,500 | 2026-07-14T07:22:11-04:00 | 2026-09-20T05:07:54-04:00 | 1,629.8 | 4.9 | 403 (5.4 %) |

Two structural facts fall out of that table:

1. **Every host's ceiling is the same overnight event window** (~01:45–05:10 EDT), and nothing in any
   window is newer than 05:07:54 EDT even though the pulls happened 5–7.6 h later. The rewrite signal
   is a **nightly batch**, not a continuous stream. Corroborated by a capture from the previous day:
   `tools/scratch/wi-sitemap.xml` (captured 2026-09-19T23:25:12 CDT = 00:25 EDT, i.e. *before* that
   night's batch) had ceiling `2026-09-19T02:56:58-04:00` — a 21.5 h plateau — and the first pull here
   (after the next batch) shows `2026-09-20T05:02:52-04:00`.
2. **`mn`/`ne`/`nd`/`sd` share the identical floor `2026-07-14T07:22:11-04:00`**, and that one
   timestamp occupies 529 (mn), 3,486 (ne), 5,634 (sd) and 6,206 (nd) of their 7,500 slots. A shared
   global bulk stamp dated 2026-07-14 is the backfill that fills these low-churn windows; their
   "7,500 entries" therefore contain far fewer *changed* pages than the count suggests (nd: only
   403 entries are stamped today).

### Stable identifiers

| identifier | where | form | notes |
|---|---|---|---|
| **AthleteID** | `<loc>` path segment | numeric, e.g. `18784161` | present in both slugged and bare-ID URL forms; range on `www` 8,804,262–18,783,767 vs `wi` 40,208–18,784,161 (old and new records coexist) |
| host/site key | the URL host itself | `www`,`wi`,`il`,… | the sitemap is host-scoped: across all 13 hosts, 0 of the 78 host pairs share an AthleteID (97,500 distinct IDs in 97,500 slots) |
| **lastmod** | `<lastmod>` | ISO-8601 with `-04:00` offset | page-rewrite time; the only change signal available |

**No TeamID, MeetID, EventID or ResultID appears anywhere in the sitemap.** Cross-reference to the
report's non-sitemap surfaces (report 27) is required for those.

### Athletic.net leverage

- **Direct:** none. The sitemap contains no Athletic.net URL or ID — it is a pure URL list.
- **Indirect (the useful part):** one sitemap request hands over up to 7,500 candidate athlete pages;
  a profile fetch then yields `name + school + Class of 20XX + city/state`, which is exactly the tuple
  a deterministic Athletic.net lookup needs. Verified on two sampled sitemap URLs (both HTTP 200):
  `www…/athletes/18783767-juan-manuel-lopez-marcos` → "Jackson-Reed High School / **Class of 2027** /
  Washington, DC"; `wi…/athletes/15289196` → "Langreck / Dodgeville/Mineral Point / **Class of 2025** /
  Dodgeville, WI". `www` rows sampled (4) resolve to DC and BC schools — see *Coverage*: `www` carries
  the jurisdictions that have no state subdomain.
- **Cost shape:** the sitemap is the cheapest *targeting* request on the site (1 request → up to 7,500
  URLs), but it is not a census: it cannot enumerate Class of 2027 across a state (see *Enumeration*
  and *Incremental use*). Report 27's PRIMARY route for Class-of-2027 identity remains the per-team
  roster (`/teams/<id>/roster`, 1 request/team); the sitemap's role is to tell the collector *which*
  of the ~600–900 teams and ~10k athletes per state actually moved since the last run.

### Athlete evidence

| field | available from the sitemap? | source |
|---|---|---|
| AthleteID + profile URL | **yes** | `<loc>` |
| name | partial — slug only, and absent on bare-ID URLs | `<loc>` |
| last page-rewrite time | **yes** | `<lastmod>` |
| graduating class / grade | no (needs 1 profile fetch; verified `Class of 2027` renders free) | profile page |
| school, city/state | no (profile fetch) | profile page |
| gender / TF-vs-XC / indoor-vs-outdoor | **no** | not in sitemap or in the profile header alone |
| performances / PRs / progression / meets | **no** (and result lists are PRO-gated; report 07/32) | — |

### Recruiting information

None. No coach, AD, email, or school-site field appears in the sitemap, and none is derivable from it.

### Result evidence

None. No ResultID, AthleteID-in-result, MeetID, EventID, mark, wind, heat, place or relay data — the
sitemap carries no result payload at all.

### Incremental use

**The window rolls. Conclusively.** Three independent pieces of evidence:

1. **Intra-day the document is static — measured on the paired pull this assignment asked for.** t0
   (09:05:02 CDT) vs **t1 (09:25:20–09:25:31 CDT, +20 min)** on six hosts — `www`, `wi`, `il`, `oh`,
   `mn`, `nd`: **added 0, removed 0, kept 7,500, `lastmod` advanced 0 on every host, and all six t1
   captures are sha256-identical to their t0 counterparts**. Shorter pairs agree: `wi` 09:05:34 →
   09:11:47 (6.2 min) identical; `www` 14:03:29Z → 14:05:02Z (1.5 min) identical; the prior-day pairs
   `tools/scratch-07/sitemap.xml` = `tools/scratch/wi-sitemap.xml` (12 min apart) and
   `tools/scratch16/sitemap.xml` = `…/sitemap2-live.xml` (4.4 min apart) are sha256-identical too.
   So intra-day differences between pulls are real content change, never jitter.
2. **Turnover over ~9 h 40 m** (prior-day captures in `tools/scratch/` vs the first pull here —
   proper *baseline = older* diff, via the harness):

   | host | older capture | added | removed | kept | turnover | lastmod floor moved |
   |---|---|---|---|---|---|---|
   | `wi` | `tools/scratch/wi-sitemap.xml` (2026-09-20T04:25Z) | 4,460 | 4,460 | 3,040 | **59.5 %** | 2026-08-28 → 2026-09-12 (+15.0 d) |
   | `in` | `tools/scratch/in-sitemap.xml` (2026-09-20T04:25Z) | 7,172 | 7,172 | 328 | **95.6 %** | 2026-08-21 → 2026-09-20 (+29.2 d) |
   | `mi` | `tools/scratch16/sitemap2-live.xml` (2026-09-20T04:18Z) | 3,300 | 3,300 | 4,200 | **44.0 %** | 2026-08-28 → 2026-09-10 (+13.5 d) |

3. **The mechanism is exactly "top 7,500 by `lastmod`".** Falsifiable test: every entry of the *old*
   window whose stamp was ≥ the *new* window's floor must still be present. Result — `wi`: 2,969
   eligible, **2 missing**; `mi`: 4,196 eligible, **1 missing**; `in`: 0 eligible, 0 missing. And every
   kept URL that appeared with a stamp below the new floor had *advanced* its stamp (0 regressions
   across the three hosts diffed). The document is a truncated recency list, nothing more.

**What a weekly collector can expect** (this is the operational answer):

- **Do not read "two pulls minutes apart are identical" as "the feed rarely changes".** Report 16 drew
  that inference from a 4.5-minute pair, and report 07 saw the same on WI. It is true — re-measured
  above out to a 20-minute pair with full six-host sha256 equality — but it is *intra-day* stability
  around a once-a-night rewrite, **not** a low change rate: the same WI file differs in 4,460 of its
  7,500 entries across the following 9 h 40 m. Report 16's "stable at hour granularity" must be read
  as "no intra-day churn", not as "the window is a slow feed".
- **One weekly pull is not "a week of changes".** A week apart, the window has essentially fully
  turned over (44–96 % in under 10 h), so a weekly diff reports ~7,500 "new" URLs but tells you nothing
  about the six days in between — the changed athletes from earlier in the week are already below the
  cap and gone. There is **no backfill**: no index, no pagination, no `lastmod`-range query.
- **Pull cadence must be ≤ the window reach you need.** The reach is host- and load-dependent:
  1.6 h (OH) to 68 d (ND) in this pull. Anything that changes on a high-churn host outside the reach is
  permanently invisible. A once-a-day pull run *after* the nightly batch (the ceiling is stable from
  ~05:10 EDT onward; pulling at, say, 08:00–09:00 EDT) is the minimum viable cadence and is sufficient
  for every host whose batch fits inside 7,500 entries (this pull: MN, NE, ND, SD and mostly WI/IL/MI/KS/IA).
- **Hosts whose nightly batch exceeds the cap lose data even with a daily pull.** `www`, `oh`, `in`, `mo`
  have ~1.6–3.1 h windows ending at the batch; their 7,500 slots are filled by the newest slice of that
  night's rewrites, and the part of the batch older than the floor is dropped. For complete coverage of
  those hosts the collector must sample *inside* the batch (01:45–05:10 EDT), e.g. 3–4 pulls spread
  across that window, or accept the cap. `[INFERENCE]` on which hosts lose how much: the batch's total
  size is not observable from the sitemap itself.
- **Cheap:** each check is ~0.9 MB of XML and one request per host. There are **no conditional-GET
  validators** (no `ETag`, no `Last-Modified`, no `cache-control`, no `Age` on the sitemap response —
  unlike `robots.txt` on the same host, which does carry `ETag`/`Last-Modified`), so revalidation is
  impossible: every check is a full transfer. For a 12-state weekly collector that is 13 requests
  (~12 MB) per run — negligible against the site's own traffic, but it must be sampled more often than
  weekly to be *complete*.
- **Recommended collector loop:** store `{loc → lastmod}` per host (the harness below does exactly
  this), pull daily after the batch, treat new `loc`s and advanced `lastmod`s as the work list, and
  de-duplicate against the roster-sweep store so each athlete profile is fetched once per change.

Resulting cadence by host class (from the measured windows above):

| class | hosts (this pull) | window reach | does 1 daily pull capture the whole night's batch? | cadence |
|---|---|---|---|---|
| batch ≥ cap (window pinned to the batch) | `www`, `oh`, `in`, `mo` | 1.6–3.1 h | **no** — only the newest 7,500 rewrites | 3–4 pulls spread across ~01:30–05:30 EDT, or accept the cap |
| batch < cap (window spans days) | `wi`, `il`, `mi`, `ks`, `ia` | 7.7–21 d | yes | 1 pull/day, ≥1 h after the batch ends (≥06:00 EDT) |
| low churn (window spans weeks, backfilled by old bulk stamps) | `mn`, `ne`, `nd`, `sd` | 68 d | yes (batches are a few hundred rows) | 1 pull/week is enough; expect mostly stale rows |

**Harness:** `tools/gaps/milesplit_sitemap_recheck.py`
(`--url` repeatable; `--write-baseline`/`--merge` to snapshot, `--baseline` to diff, `--from-file` for
offline diffs, `--json` for machine output; exit 0 ran / 2 fetch failed / 3 parse failed / 4 baseline
problem; one-shot, no daemon). Baseline snapshot for all 13 hosts:
`research/midwest/evidence/gaps/33/baseline-milesplit-sitemaps-2026-09-20T0905.json` (8.7 MB, 13 hosts ×
7,500 entries). Verified working on live, stale-prior-day and offline inputs (see *Evidence appendix*).

### Access characteristics

- **Class: static XML sitemap (public structured data), robots-allowed.** `GET /sitemap.xml` from
  `www` and every state host → 200, no auth, no cookies required, no JS, no CAPTCHA.
- **robots.txt:** byte-identical 173 B on all 13 hosts — `User-agent: *` → `Disallow: /rankings`,
  `/virtual-meets`, `/api/`, `/contact`; `Mediapartners-Google` and `AmazonAdBot` exempted.
  **No `Crawl-delay`, and no `Sitemap:` directive** — so `/sitemap.xml` is discovered by convention
  only, and every sitemap path used here is outside the disallow set.
- **Rate limiting:** none observed. ~44 sequential MileSplit requests across the 13 hosts (1.2–2 s
  spacing) at HTTP 200/404, zero 429, zero `Retry-After`, zero CAPTCHA, no failed request. No
  published limit found.
- **Caching/validators:** none on `/sitemap.xml`; `set-cookie: unique_id=…` is issued but not required.

### Recommendation

**DISCOVERY-ONLY — as a change feed for athlete profiles** (not a census, and not a result source).

- Marginal coverage: 13 requests/run (one per host, ~12 MB) tells the collector exactly which athlete
  pages were rewritten since the previous run — the only per-athlete change signal on MileSplit's
  robots-allowed surface, and the natural trigger for the roster route (report 27) and for
  Athletic.net lookups (report 07's C2 join).
- It cannot answer "who is Class of 2027 in state X": grade lives on the profile, and the window is a
  recency slice. Use it to *target*; use rosters for the census.
- For a weekly refresh design, the sitemap replaces the "re-crawl everything" step entirely, but the
  collector must pull daily (and intra-batch for `www`/`oh`/`in`/`mo`) or it will silently miss changes.
- **Pipeline fit (repo read-only observation):** the existing adapter
  `crates/census-service/src/sources/milesplit.rs` exposes only `fetch_team_index` and `fetch_roster`
  over the 12 host constants, keyed by journal phases `milesplit_teams_<st>` / `milesplit_rosters_<st>` —
  there is no change feed today. A sitemap diff is the missing selector for "which rosters/profiles to
  re-pull", exactly the cadence report 16 proposes (full sweep per season, targeted re-pull on change).

### Evidence appendix

All requests used a browser UA (`Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like
Gecko) Chrome/140.0.0.0 Safari/537.36`), sequential with 1.2–2 s spacing per host. No `*.athletic.net`
host was contacted. Raw captures under `research/midwest/evidence/gaps/33/`: `www-t0.xml`,
`state/<st>.xml` (all 12 states), `pull2-<host>.xml` (t1), `robots/`, `probes*/`, `samples/`,
`baseline-milesplit-sitemaps-2026-09-20T0905.json`, `baseline-prior-2026-09-19T2325.json`,
`sitemap-window-summary-2026-09-20.json`. Harness: `tools/gaps/milesplit_sitemap_recheck.py`.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.milesplit.com/sitemap.xml` | GET | 200 (930,488 B, `text/xml`, no ETag/Last-Modified) | t0 pull: 7,500 locs, **all** `www.milesplit.com/athletes/…`, lastmod 02:11:24→04:12:39 -04:00 (2.02 h), newest-first; byte-identical (sha256 `5a05ca12…`) to the 09:03:29 capture | 2026-09-20T09:05:02-05:00 |
| `https://wi.milesplit.com/sitemap.xml` | GET | 200 (918,197 B) | 7,500 athl. URLs, window 2026-09-12 11:55→2026-09-20 05:02 (185.1 h); 5 bare-ID rows; 4,541 stamped today | 2026-09-20T09:05:34-05:00 |
| `https://il.milesplit.com/sitemap.xml` | GET | 200 (916,822 B) | 7,500; window 2026-09-10→2026-09-20 (224.7 h); 36 bare-ID rows | 2026-09-20T09:05:36-05:00 |
| `https://oh.milesplit.com/sitemap.xml` | GET | 200 (918,826 B) | 7,500; window 01:59→03:32 (1.6 h) — batch-filled, 100 % today | 2026-09-20T09:05:38-05:00 |
| `https://mn.milesplit.com/sitemap.xml` | GET | 200 (917,452 B) | 7,500; window 2026-07-14→2026-09-20; 529 rows share the 07-14 bulk stamp | 2026-09-20T09:05:40-05:00 |
| `https://in.milesplit.com/sitemap.xml` | GET | 200 (915,859 B) | 7,500; window 01:53→03:59 (2.1 h), 100 % today → batch ≥ cap | 2026-09-20T09:05:42-05:00 |
| `https://mi.milesplit.com/sitemap.xml` | GET | 200 (919,207 B) | 7,500; window 2026-09-10→2026-09-20 (224.9 h) | 2026-09-20T09:05:44-05:00 |
| `https://ia.milesplit.com/sitemap.xml` | GET | 200 (917,038 B) | 7,500; window 2026-08-30→2026-09-20 (502.3 h) | 2026-09-20T09:08:33-05:00 |
| `https://ks.milesplit.com/sitemap.xml` | GET | 200 (916,561 B) | 7,500; window 2026-09-04→2026-09-20 (370.2 h) | 2026-09-20T09:08:38-05:00 |
| `https://ne.milesplit.com/sitemap.xml` | GET | 200 (919,188 B) | 7,500; 3,486 rows = 2026-07-14 bulk stamp (46.5 % of window) | 2026-09-20T09:08:42-05:00 |
| `https://nd.milesplit.com/sitemap.xml` | GET | 200 (917,490 B) | 7,500; 6,206 rows = 2026-07-14 stamp (82.7 %) | 2026-09-20T09:08:46-05:00 |
| `https://sd.milesplit.com/sitemap.xml` | GET | 200 (918,683 B) | 7,500; 5,634 rows = 2026-07-14 stamp (75.1 %) | 2026-09-20T09:08:51-05:00 |
| `https://mo.milesplit.com/sitemap.xml` | GET | 200 (916,766 B) | 7,500; window 01:45→04:51 (3.1 h), 100 % today — earliest batch start seen | 2026-09-20T09:08:55-05:00 |
| `https://{www,wi,il,oh,mn,in,mi}.milesplit.com/robots.txt` | GET ×7 | all 200 (173 B each) | Byte-identical policy network-wide; **no `Sitemap:` directive, no `Crawl-delay`**; `/rankings`,`/virtual-meets`,`/api/`,`/contact` disallowed; `/sitemap.xml` allowed | 2026-09-20T09:05:19-05:00 |
| `https://{ia,ks,ne,nd,sd,mo}.milesplit.com/robots.txt` | GET ×6 | all 200 (173 B each) | Same 173-byte policy on the remaining mission states | 2026-09-20T09:08:30-09:08:55 |
| `https://wi.milesplit.com/sitemap.xml?page=2` | GET | 200 (918,197 B) | **Byte-identical** to `/sitemap.xml` (same sha256 `22d0c876…`) → no pagination; the sitemap is not pageable | 2026-09-20T09:06:25-05:00 |
| `https://wi.milesplit.com/sitemap_index.xml` | GET | 404 (548 B) | No sitemap index → no larger corpus behind the window | 2026-09-20T09:06:24-05:00 |
| `https://wi.milesplit.com/sitemap2.xml` | GET | 404 (25,343 B) | No second sitemap file | 2026-09-20T09:06:23-05:00 |
| `https://www.milesplit.com/sitemap_index.xml` | GET | 404 (548 B) | Same, on the `www` host | 2026-09-20T09:06:26-05:00 |
| `https://www.milesplit.com/news-sitemap.xml`, `https://www.milesplit.com/sitemap/`, `https://wi.milesplit.com/team-sitemap.xml` | GET ×3 | 404 (25,067–25,371 B) | No news/team/alternate sitemaps exist → athlete URLs are the whole sitemap surface | 2026-09-20T09:09:12–09:09:16-05:00 |
| `https://wi.milesplit.com/athletes/15289196` (bare-ID form from the sitemap) | GET | 200 (84,748 B, canonical `…/athletes/15289196`) | Bare-ID sitemap rows are live pages; profile renders "Langreck / Dodgeville/Mineral Point / **Class of 2025** / Dodgeville, WI" free | 2026-09-20T09:06:38-05:00 |
| `https://www.milesplit.com/athletes/18783767-juan-manuel-lopez-marcos` | GET | 200 (89,871 B) | `www` window is a complement namespace (DC athlete, no state host); profile renders "Jackson-Reed High School / **Class of 2027** / Washington, DC" free | 2026-09-20T09:06:39-05:00 |
| `https://www.milesplit.com/athletes/18783752-william-aranguren`, `…/18783386-natsuki-sato` | GET ×2 | 200 (90,030 / 84,879 B) | `www` rows are Canada/DC athletes ("White Rock Christian Academy, Surrey, BC"; "Jackson-Reed High School, Washington, DC") → `www` is the no-state-subdomain namespace | 2026-09-20T09:12:02-05:00 |
| prior-day captures `tools/scratch/wi-sitemap.xml`, `tools/scratch/in-sitemap.xml`, `tools/scratch16/sitemap2-live.xml` | local file (mtimes 2026-09-19T23:12–23:25 CDT) | n/a | Baseline side of the cross-day diff: 4,460/7,172/3,300 added-removed over ~9 h 40 m; WI ceiling plateaued at 2026-09-19T02:56 for 21.5 h before the next batch | captures 2026-09-19T23:12–23:25-05:00 |
| `tools/gaps/milesplit_sitemap_recheck.py` | local execution | exit 0 / 2 / 3 / 4 | Harness verified: help; wrong/missing baseline → 4; non-urlset → 3; missing file → 2; offline, stale-prior-day and live diffs → 0 with added/removed/kept counts | 2026-09-20T09:07–09:25-05:00 |
| `https://{www,wi,il,oh,mn,nd}.milesplit.com/sitemap.xml` | GET ×6 (t1 — the 20-minute paired pull) | all 200 (916,822–930,488 B) | **All six t1 captures are sha256-identical to their t0 counterparts**; harness diff on each: added 0, removed 0, kept 7,500, `lastmod` advanced 0 → the document is static intra-day | 2026-09-20T09:25:20–09:25:31-05:00 |
| `https://wi.milesplit.com/sitemap.xml` (harness live re-check) | GET | 200 (918,197 B) | Harness live-fetch path end-to-end; byte-identical to the 09:05:34 capture 6.2 min earlier | 2026-09-20T09:11:47-05:00 |
| synthetic `urlset` (bare-ID row, row with no `lastmod`, 3 entries) and `sitemapindex` | local file via harness | n/a (robustness) | Parser counts bare-ID rows correctly, tolerates a missing `lastmod`, leaves `capped=false` for short files, and rejects a sitemap index with exit 3 | 2026-09-20T09:12:19-05:00 |

