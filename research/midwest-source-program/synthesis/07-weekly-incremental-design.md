# Weekly incremental design — how to refresh without re-fetching the corpus

Design synthesized from reports 01–29. Goal: a weekly collector that discovers new/changed meets,
results and athletes in the 12-state scope without re-pulling the historical corpus and without broad
Athletic.net searching. Every mechanism below is evidenced in a report; costs are per run unless noted.

## 0. Keys and gotchas

- **Cohort key**: absolute graduating class (`gradYear`, `Class of 2027`), never grade level. In 2026-27
  Class of 2027 = Grade 12; Grade-11 corpora are 2025-26 data [07][27].
- **Dedupe key**: `IDResult` (Athletic.net result id) where available; else (meet, event, athlete, mark)
  tuple. Rankings pages overlap by ~2 ranks at boundaries — dedupe is mandatory [05].
- **Athletic.net sends `cache-control: no-store` on all 41 captured calls** — no conditional GET; deltas
  must be client-side content-digest diffing [03].
- Sources that DO support conditional GET: AthleticLIVE Azure blob (304s observed) [12]; MHSAA
  (`max-age`+`last-modified`+`x-drupal-cache`) [15].
- `MeetID` exists in every rankings row but was dropped by the retained workbook — retain it; that makes
  meet seeding free [03][05].

## 1. Tiered weekly job set

### Tier 0 — cheap deltas (minutes, no browser)
| Job | Source | Cost | Yields |
|---|---|---|---|
| State timer indexes | PrimeTime `/api/results/current?year=YYYY&page=N` (200/page) [22][08]; AthleticLIVE ES `search.athletic.live/<machine>_meet_list/_search` [12][25]; Bound scoreboard `?date=` [11]; IATC weekly XC page [11]; MITCA [16] | 1–3 requests per state | New meets, new result files, often the AN MeetID directly |
| MileSplit tails | `sitemap.xml` per state (7,500 URL+lastmod, ~3–4 week window) [16][27]; `/results?...&page=N` 50/page date-desc [07] | 1 + k per state | New meets/result sets; athlete-URL change tail |
| Association surfaces | WIAA tournament pages [06]; IHSA API [13]; KSHSAA champs API [23]; NDHSAA advisories [25]; MSHSL `/2026-track-and-field-results` [09] | 1–5 per state | Postseason updates, qualifier lists, coach changes |

### Tier 1 — result pulls for touched meets (browser or blob)
| Channel | Cost | Notes |
|---|---|---|
| Athletic.net whole-meet | 3 XHR/meet (`GetMeetData`+`GetEventDivisionData`+`GetAllResultsData`) — returns every division/event/athlete incl. unknown participants; ~50–250× cheaper than per-athlete profiles for a 200–1000-participant meet [03] | needs headed session; response shapes still un-verified (capture item A1) |
| AthleticLIVE blob | 1 request/event (`ind_res_list/_doc/<eventId>`) with `a.ani` athlete ids + grade [10] | no Cloudflare; event ids need one rendered page per meet |
| Timer files | 1 file/meet (PrimeTime PDF/HTML, Hy-Tek, Crystal, RUNMEET) [08][22][24] | text-layer quality varies; some 2026 files raster-only |

### Tier 2 — athlete/roster layer (seasonal or on-change, not weekly)
| Job | Cost | Frequency |
|---|---|---|
| MileSplit rosters (grad year per athlete, robots-allowed) | 1 req/team; 6,633 for all 12 states [27] | 2×/season (before XC, before outdoor) |
| TFRRS Indiana rosters | ~901 req for the whole state, ~0.09 req/athlete [18] | per season |
| MSHSL rosters w/ grade | 1 req/team/activity [09] | per season (no backfill — current-season only) |
| Bound IA/SD rosters | 1 req/team [11][26] | per season; XC rosters populate late |
| Rankings sweep (AN, state-scoped) | 2,728 requests for 12-state boys outdoor single-event scan; 4,233 national [04] | weekly only if AN discovery is the goal; otherwise 1×/season + delta via timer indexes |

### Tier 3 — coach graph (rolling)
- No source has a change feed (WIAA serves `no-cache,no-store`) [06]. Poll per school on a rolling
  schedule. Full-state costs: OH ~2,226 requests (742 schools × 3) [19]; KS 1 request for 526 ADs [23];
  IL 828 × 2 [13]; WI 629 school pages [29]. Refresh monthly; coach churn is seasonal.

## 2. Where the AN requests actually go (budget discipline)

1. **Never enumerate athletes through AN when another source already names them.** Rankings rows cost
   ~0.03 req/athlete vs 1 `GetAthleteBioData`/athlete [04][03].
2. **Spend AN only on**: (a) the tracked-athlete core needing progression/PR history
   (`GetAthleteBioData`, still the only captured source of `AllSeasons`) [03]; (b) meets whose results no
   timer publishes; (c) team/home-page resolution for school seeds not covered by association exports.
3. **Convert external identities to AN ids locally**: MileSplit/DAT/Bound give (name, school, grad year);
   association sources give AN ids outright in 5 states (see playbook tier A/B). Keep the mapping table.
4. **Meet discovery without AN**: the timer/association indexes above supply most MeetIDs; AN's own
   meet-calendar endpoint is still unidentified (open item A2) [03].

## 3. Suggested weekly pipeline (concrete)

```
sat 02:00  Tier-0 sweep (timer indexes, MileSplit tails, association pages)   ~60-120 requests, no browser
sat 02:30  diff vs stored state -> new_meets, changed_meets, new_files
sat 03:00  for each touched meet with no timer file: AN whole-meet pull (3 XHR) or AthleticLIVE blob (1 req)
sat 04:00  parse files -> result rows (grade where present) -> upsert on IDResult/tuple
sun        rolling coach refresh slice (1/4 of schools), sitemap re-pull for change tail
season     roster sweeps (MileSplit / TFRRS / MSHSL / Bound) on schedule boundaries
```

State the pipeline may not exceed per host: ~50 requests/session, ≥1 s spacing, no CAPTCHA/auth bypass;
Bound needs ≥10 s crawl-delay [11].

## 4. What each state still needs before it can run weekly unattended

- **TX-free states**: MO regular season has no machine index (state finals only; PrimeTime blocked by ToU
  absent a license) [21][22]; KS TF grades are null [23]; SD has no athlete-level data at all [26];
  IN provider switched TFRRS→MileSplit in 2025-26 — watch the 2026-27 XC provider [17].
- **Browser-required**: any AN interaction (Cloudflare), AthleticLIVE event-id discovery, myOHSAA/officials
  portals, gobound (403 to curl) [19][29].
- **Open captures** (see `06-open-questions.md`): A1 meet responses, A2 meet calendar, B1 AthleticLIVE API,
  C1 MileSplit API policy decision.
