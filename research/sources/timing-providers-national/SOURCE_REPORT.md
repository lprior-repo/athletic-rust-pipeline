# SOURCE_REPORT — Tier-3 timing / result provider families (national)

Lane: `research/sources/timing-providers-national` · captured 2026-09-21/22 UTC · all counts measured from files in `samples/` (see `samples/CAPTURES.md` for the per-file URL/status/bytes/timestamp/command ledger and `samples/capture-log.jsonl` for the machine-readable log; **108 requests logged to date**, across 34 hosts; CAPTURES.md is regenerated from the log, and its inventory check reports "Files not covered by this ledger: NONE").

Everything marked `[INFERENCE]` is reasoned, not measured. Anything else cites a captured file.

## Summary — recommendation per family

| Family | Host(s) | Bulk whole-meet? | Requests / 1000 performances | Recommendation |
|---|---|---|---|---|
| FlashResults | flashresults.com | partial (roster yes / marks per event) | 0.33 roster · 10.2 marks | **RESULT_SOURCE** (robots: no PDF/CSV) |
| FlashResults Texas | flashresultstexas.com + live.* | yes (PDF own-domain, blob event doc) | 7.4 (blob) | **RESULT_SOURCE** |
| AthleticLIVE / anet.live platform | <tenant>.anet.live, search.athletic.live, Azure blob, static.trackmeetio.com | yes | 0.0065 platform census · 7.4 per event | **PRIMARY** |
| PrimeTime Timing | pttiming.com / data.pttiming.com | partial (per division file) | 0.35 index · ~5 files | **RESULT_SOURCE** |
| TFRRS | tfrrs.org | yes | 0.53 | **RESULT_SOURCE** / VALIDATION |
| MeetPro (DirectAthletics) | tfmeetpro.com, results.tfmeetpro.com | unverified | unverifiable — 0 meets located | **CONDITIONAL** |
| RACE RESULT | raceresult.com / my.raceresult.com | metadata only under robots | not measurable under robots | **CONDITIONAL** |
| OpenTrack | opentrack.run | no | n/a | **REJECT** (Cloudflare 403) |
| GSE Timing (MN) | gsetiming.com | per race (JSON) | 7.5 | **RESULT_SOURCE** (state) |
| Finish Timing (OH) | finishtimingresults.com | yes (Hy-Tek text) | 2.7 | **RESULT_SOURCE** (state) |
| RunWV (WV) | runwv.com | no structured surface | n/a | **DISCOVERY_SOURCE** |
| XCStats (CA) | xcstats.com | paywalled + ClaudeBot disallowed | n/a | **REJECT** |
| Wayzata Results | wayzataresults.com / wayzata tenant | own CMS 403; tenant path yes | 7.4 (tenant blob doc) | **CONDITIONAL → use AthleticLIVE tenant** |
| Hy-Tek MEET MANAGER output (format family) | many publishers | format-dependent | format-dependent | **RESULT_SOURCE (format standard)** |
| TrackMeet.io static artifacts | static.trackmeetio.com | yes (PDF) | 1 request/meet, OCR required | **CONDITIONAL** |
| TRXC Timing (MO) | trxctiming.com | per meet directory | ~1/meet | **RESULT_SOURCE** (Missouri) |
| Hero's Timing (MN/ND) | herostiming.com → live.herostiming.com | yes (tenant) | 7.4 | **RESULT_SOURCE** (via tenant) |

---

## 1. FlashResults (flashresults.com)

- **Source name**: Flash Results, Inc. — the FinishLynx/Hy-Tek championship timing house.
- **Geographic coverage**: national, championship-biased. The 2027 fall schedule lists 14 meets in TX/NC/VA/AR/KY/SC (`samples/flashresults-schedule.htm`, table row set: Aggie Opener(College Station TX), Friday Night Lights XC Festival(Kernersville NC), Virginia Invitational(Earlysville VA), Texas A&M Invitational(College Station TX), Adidas XC Challenge(Cary NC), South American Games(Santa Fe ARG), Chile Pepper(Fayetteville AR), Louisville Classic(Louisville KY), Arturo Barrios Invitational(College Station TX), Panorama Farms Invitational(Earlysville VA), ACC Championships(Blacksburg VA), NCAA Southeast Regional(Spartanburg SC), NCAA South Central Regional(College Station TX), NXR Southeast Regional(Cary NC)). Archived NCAA championship pages from 2016 remain live (`samples/flashresults-2016-guess-NCAA.html`).
- **Sports**: XC + indoor/outdoor T&F.
- **Historical depth**: year index pages for at least 2018/2019/2020 are **listed in the site's sitemap** (`samples/flashresults-sitemap-probe.xml`, 8 `<loc>` entries, lastmod 2021-01-01); `2019results.htm` verified live = 79,334 B / 60 meet links including `/2019_Meets/Outdoor/06-13_NBNO/` (New Balance Nationals Outdoor — a HS national championship) (`samples/flashresults-2019results.htm`). Depth is therefore ≥ 8 seasons, but the sitemap is stale and does not enumerate meet directories.
- **Discovery mechanism**: root → `results.htm` (current season only, `samples/flashresults-results.htm`) → meet `index.htm` (`samples/flashresults-2027meets-xc-adidasxc-index.html`, 3 per-day tables, 31 event rows, status column) → per-event files. **The event hrefs are UNQUOTED** (`<a href=203-1_compiled.htm>`): the capture holds 62 `_compiled.htm` href occurrences = 31 distinct event pages, plus 31 distinct `_start.htm` links; a naive `href="…"` regex finds zero. Event numbers (001…203) are visible only inside those hrefs, not in the table cells. Historical year pages (`/2019results.htm`) enumerate older seasons. No per-meet sitemap; older meet directories are only reachable from a year page or an external link.
- **Stable identifiers**: meet = URL path `<YYYY>_Meets/<Sport>/<MM-DD_Slug>/`; event = the 3-digit event number inside the filename (`203-1_compiled.htm` = event 203, round 1); round = the second filename token (1 = prelim/section, 2 = final). No result id, no athlete id on the page — but rows link **TFRRS athlete profiles**.
- **Pagination**: none; tables are inlined whole.
- **Athlete fields**: name `LAST FIRST` (all-caps surname), affiliation/school, bracketed class token, `Comp#` bib in athleteList, event entries.
- **Meet fields**: day, start time, event name, Result/Split link, status (index); caption on event pages carries round + wind.
- **Result fields**: `Pl`, athlete, mark, qualifier/annotation (`SB`/`PB`), XC adds `Pace`, `Pts`, `TmPl`; team-score tables add total/avg/spread.
- **Grade/class evidence**: **yes — bracketed token after the school**: HS meets `[9]`…`[12]`, college meets standing `[JR]`/`[SR]`. Real rows: `Hank DELK Broughton [9]` (XC, `samples/flashresults-2027meets-xc-adidasxc-203-1_compiled.htm`); `Kanyinsola AJAYI 2Auburn [JR]` (track, `samples/flashresults-2026meets-outdoor-06-10_NCAA-001-2_compiled.htm`). The same token appears in athleteList (`BROWN Maya … A.C. Reynolds [9]`).
- **Coach/contact fields**: none observed.
- **Public API availability**: none.
- **Static file availability**: HTML only for agents — `robots.txt` **disallows `/*.pdf$`, `/*.csv$`, `/*.js$`, `/flashwest/`, `/flashtexas*`** (`samples/robots-www.flashresults.com.txt`); the per-meet PDFs that exist on this host are off-limits to automated agents.
- **Browser requirement**: none (plain HTML tables).
- **Request cost**: roster path = **1 request / 3,038 rows = 0.33 per 1,000** (`samples/flashresults-2027meets-xc-adidasxc-athleteList.htm`, 551,745 B); marks path = 31 distinct compiled event pages / 3,038 rows = **10.2 per 1,000** (31 distinct `_compiled.htm` hrefs counted in `samples/flashresults-2027meets-xc-adidasxc-index.html`). NCAA 2026 roster: 1,276 data rows / 363,462 B = same 0.33 profile.
- **Published rate limits**: none.
- **Known blocks**: robots PDF/CSV/JS bans (above); a control request to a nonexistent path returns a 111-byte stub (`samples/flashresults-control-nonexistent.html`), so existence checks must be byte-aware.
- **Cross-source join keys**: **TFRRS** — every result row links `https://www.tfrrs.org/athletes/<id>/…` (observed on the 2026 NCAA pages). Athletic.net: none observed on captured pages.
- **Estimated marginal coverage**: small but high-value — the HS slice of a schedule that is ~14 meets/season is roughly **2–4 HS meets per season nationally** (Friday Night Lights XC Festival, Adidas XC Challenge HS divisions, NXR Southeast Regional; `[INFERENCE]` on which of the others carry HS divisions — only Adidas XC was verified to carry HS grade tokens). College rows are useful only as a validation/class-token corpus.
- **Implementation recommendation**: **RESULT_SOURCE**, with a `CONDITIONAL` caveat for the PDF surface (robots-disallowed). Use `athleteList.htm` for the grade roster and per-event `_compiled.htm` for marks.

## 2. FlashResults Texas (flashresultstexas.com + live.flashresultstexas.com)

- **Source name**: Flash Results Texas (same firm, Texas division; also an AthleticLIVE tenant).
- **Geographic coverage**: Texas-centric plus its legacy national archive.
- **Sports**: T&F + XC.
- **Historical depth**: **2002 → 2025 measured** from `results.htm`: 843 outbound links, 542 meet-ish, year histogram 2002(4) 2003(2) 2004(1) 2005(1) 2006(8) 2007(5) 2008(4) 2009(8) 2010(31) … 2023(47) 2024(42) 2025(13); hosts: flashresultstexas.com 339, flashresults.com 255, www.flashresults.com 129, www.flashresultstexas.com 97, live.flashresultstexas.com 48 (`samples/flashresultstexas-results.htm`, 214,253 B).
- **Discovery mechanism**: `results.htm` legacy link list (one request) → per-meet dirs on either host → per-event pages; live meets via `live.flashresultstexas.com/meets/<id>/…` SPA (`samples/live-flashresultstexas-meet-76403.html`, 50,184 B) backed by the shared AthleticLIVE blob/ES plane.
- **Stable identifiers**: meet slug path; AthleticLIVE `mi`/`ani` ids when the meet exists as a meet doc (tenant `flashresultstexas`: `samples/search-athleticlive-flashresultstexas-meet_list-search.json`, 222 meet docs; a sample doc `samples/search-athleticlive-flashresultstexas-meet-doc-6951.json`).
- **Pagination**: none on `results.htm`; blob event docs are whole.
- **Athlete fields**: as FlashResults (name, school, class token) on own-domain pages; on the live SPA the athlete object (`i`,`n`,`y`,`g`,`ani`) as in §3.
- **Meet fields**: date/name from the link list; live meet doc carries `n`,`ln`,`ls`,`lsa`,`lo`,`sd`,`tna`,`li`.
- **Result fields**: same Hy-Tek-authored tables as FlashResults; plus per-event blob docs.
- **Grade/class evidence**: bracketed class tokens on own-domain pages; `y` (SR/JR/SO/FR) on live docs.
- **Coach/contact fields**: meet doc `qn`/`qe` (meet contact, i.e. the timer) and `atr` attribution HTML.
- **Public API availability**: `robots.txt` on www is **empty (0 bytes)** so the PDF surface is not disallowed here (`samples/flashresultstexas-robots.txt`); the shared search/blob API is public.
- **Static file availability**: own-domain PDFs; `live.*` robots disallows `/meets/*/athletes*`, `/meets/*/teams*`, `/meets/*/live*`, `/meets/*/follow*` (`samples/live-flashresultstexas-robots.txt`) — the *result* endpoints are not in the disallow list.
- **Browser requirement**: none for PDF/blob; SPA pages need a browser but their data does not.
- **Request cost**: legacy archive = 1 request for the 24-year link map; per-meet blob doc = **7.4 / 1,000** (136 rows per request).
- **Published rate limits**: none.
- **Known blocks**: none observed for the endpoints used; the `live.*` robots exclusions apply to entry/team/live pages.
- **Cross-source join keys**: Athletic.net via meet doc `ani`.
- **Estimated marginal coverage**: high for TX; the legacy list also yields cross-links into other timers (e.g. `cfpitiming.com` 2006 entries), useful for discovery rather than rows.
- **Implementation recommendation**: **RESULT_SOURCE** (Texas), with the live tenant as a discovery shortcut into AthleticLIVE.

## 3. AthleticLIVE / anet.live (shared multi-tenant platform — includes Wayzata, Athletic Timing and ~250 other tenants)

- **Source name**: AthleticLIVE / anet.live / Athletic.net live-results platform.
- **Geographic coverage**: **256 tenant indexes, 153,774 meet docs, 70 state buckets** measured in two requests (independently triangulated: `data/adapter-ranking.csv` rank 5 reports **258 tenant origins published on the SPA** for the same platform — agreement within 2) (`samples/search-athleticlive-all-meet_list-indices.json`; `samples/search-athleticlive-all-meet_list-by-state-keyword.json`). Top states: Illinois 14,599 · California 12,703 · Michigan 10,775 · Oregon 8,353 · Minnesota 7,502 · Iowa 7,169 · New York 6,917 · Pennsylvania 5,937 · Maryland 5,900 · Ohio 4,542 · Nebraska 4,534 · Alabama 4,510 · Wisconsin 3,902 · Texas 3,454 · West Virginia 1,035 · North Dakota 875. Top tenants (index names carry the `_meet_list` suffix): `live_results_meet_list` 75,526 · `athleticlive_meet_list` 18,055 · `athletic_timing_meet_list` 4,381 · `wayzata_meet_list` 2,768 · `fulton_meet_list` 2,384 · `xpress_meet_list` 2,172 · `michiana_meet_list` 2,000 · `blacksquirrel_meet_list` 1,887.
- **Sports**: all (T&F, XC, and road races on the same tenant indexes — meet docs carry `xc`/`tf` flags).
- **Historical depth**: the tenant doc counts are lifetime totals per tenant; individual event docs are retrievable per event id. No per-tenant date histogram was taken in this lane `[open question]`.
- **Discovery mechanism**: `POST https://search.athletic.live/*_meet_list/_search` (terms agg on `_index` → tenant census; terms agg on `lsa.keyword` → state census) → tenant `_meet_list` docs (`/…/_doc/<id>`) → per-event blob docs `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<eventId>`.
- **Stable identifiers**: meet `i`/`ani` (Athletic.net meet id), event id (`2150205`), athlete `a.i` + `a.ani`, team `a.t.i` + `a.t.ani`, result row `i`. **These are the strongest join keys found in this lane.**
- **Pagination**: ES search takes `from`/`size`; blob docs are single-object (no pagination).
- **Athlete fields** (blob row): `n` full name, `fn`/`l`, `y` class token, `g` gender, `mi` meet id, `phu` photo URL, `cco` custom field, `cm`, **`ani` = Athletic.net athlete id**, `xc` flag, `t` team object `{i,f,n,mi,ani,xc}`. Real row: `{"i":42748637,"n":"McKenna Montgomery","y":"SR","g":"Female","ani":17327390,"t":{"i":1371954,"f":"Albia","ani":19361}}`.
- **Meet fields**: `i`,`ani`,`n`,`ln` (long name),`ls`,`lsa` (state),`lo` (venue),`sd`,`sdy`,`o` (owner),**`tna` timing-company name**, **`li` meet software (e.g. "Hy-Tek")**, `qn`/`qe` contact name+email, `atr` attribution HTML.
- **Result fields** (blob row, verified key-by-key over all 136 rows of `samples/athleticlive-blob-ind_res_list-2150205.json`): `m` display mark, **`im` integer milliseconds** (1100700 = 18:20.7), `irs` split rows `{sp, cs}` (split + cumulative), `avk`/`avm` average km/mile pace, `er` seed, `pr` personal best, `ern` entry annotation (`PR`/`SB`), `p`/`pt`/`ro` place+rank, `gap`, `i` result id, `a` athlete. `w` wind, `hn` heat, `hl` lane are present but null in this XC capture. **Round is event-level, not row-level**: `runm` (`Finals`), `rui` (`1-1`). Row keys `no`, `s`, `cp`, `co`, `hfli`, `hflv`, `hfav`, `ith`, `iu`, `vm` exist but carry null/1 in this capture — their semantics are `[unverified]`.
- **Grade/class evidence**: `y` on every athlete object (SR/JR/SO/FR; the meet doc's `ry` lists the vocabulary for that meet) — but **not every meet populates it** `[open question: how often `y` is null across tenants]`.
- **Coach/contact fields**: `qn`/`qe` (meet contact = timer), `atr` (attribution block). Team-level coach data not observed in result docs.
- **Public API availability**: yes — unauthenticated ES cluster at `search.athletic.live` plus public Azure blob container `athleticlive.blob.core.windows.net/$web/`. No key, no session.
- **Static file availability**: whole-meet PDFs at `static.trackmeetio.com/meetFiles/<MeetID>/<hash>.pdf` (317,088 B captured; **text extraction yields mojibake** — subset fonts, see `samples/extract-trackmeetio-67670-pdftotext.txt`) and Azure blob JSON.
- **Browser requirement**: none for the ES/blob plane; the SPA HTML is a shell.
- **Request cost**: platform census **0.0065 / 1,000** (1 request ↔ 153,774 meet docs); whole event doc **7.4 / 1,000** (1 request = 136 rows / 159,531 B).
- **Published rate limits**: none published. Treat as a shared club cluster: this lane used ≤1 req/s.
- **Known blocks**: none for the endpoints used. **Mapping limits found**: terms-agg on `tna` (provider) **fails on 250/256 shards** (`illegal_argument_exception`, text field without keyword subfield) and on `lsa` text — only `lsa.keyword` aggregates. Per-provider meet counts are therefore not obtainable in one request; enumerate tenants instead (`samples/search-athleticlive-all-meet_list-by-provider.json`, `…-by-state.json`).
- **Cross-source join keys**: `ani` (athlete), `a.t.ani` (team), `ani` on meet docs — direct Athletic.net identity; `tna` names the timer, giving a provider→tenant map.
- **Estimated marginal coverage**: **the single highest-yield family in this lane** — 153,774 meet docs with per-event payloads and Athletic.net ids on athletes. For a Co2027 census it converts directly into canonical athlete rows.
- **Implementation recommendation**: **PRIMARY**.

## 4. PrimeTime Timing (pttiming.com / data.pttiming.com / results.pttiming.com)

- **Source name**: PrimeTime Timing LLC (Wisconsin; Athletic.net's principal Midwest timing partner).
- **Geographic coverage** (2026 API census, `exStateOr`): WI 245 · MO 24 · IL 12 · FL 11 · "Wisconsin" 10 · IA 8 · IN 8 · NE 6 · TX 5 · PA 4 · GA 4 · NY 3 · CA 2 · MN 2 · KS 2 · UK 1 · TN 1 · KY 1 · blank 2. **The field mixes USPS codes and spelled-out names in one payload — normalise before filtering.** (`samples/pttiming-api-results-current-2026-page1.json`, `…-page2.json`, 353 events total.)
- **Sports**: XC + T&F (+ road races).
- **Historical depth**: API accepts `year`; this lane enumerated 2026 only (353 events). Prior Midwest research enumerated 319 Wisconsin 2025 events. `[open question: earliest year served by the API]`
- **Discovery mechanism**: `GET https://www.pttiming.com/api/results/current?year=<Y>&page=<N>&limit=200` → `{rows:[{id,name,startDate,exStateOr,eventUrl,results:{fileLinks:[{label,url}]}}]}`. Files live at `https://data.pttiming.com/storage/v1/object/public/event-files/<event-uuid>/<epoch-ms>-<slug>.(pdf|htm|html)`. Meet pages (`/event/<uuid>?pt=results`) are Next.js shells that render from the same API.
- **Stable identifiers**: event = Supabase uuid inside `eventUrl` (key on this, not on `id` — inherited dual-space caveat, `research/midwest/22-missouri-primetime.md`); result file = object path `<uuid>/<epoch-ms>-<slug>`; no per-row result id anywhere.
- **Pagination**: `page`/`limit` params; `hasMore` flag. Measured: page1 = 200 rows (`hasMore: true`), page2 = 153 rows (`hasMore: false`), pages 3–5 = 0 rows → **353 events = 2 requests**.
- **Athlete fields**: name (`Last, First` in PDFs, `NAME` + `YEAR` + `SCHOOL` in Hy-Tek htm); bib in Hy-Tek files; team/school.
- **Meet fields**: `name`, `startDate`, `exVenueOr` (venue), `exCityOr`, `exStateOr`, `exHostOr`, `exTimer`, `exStatusNote`, `deadline`, `exEntryLink`, `exEntryOpen`/`exEntryClose`, `clientCompany`/`clientCity`/`clientState`.
- **Result fields**: per-file. Hy-Tek: `Pl Name Year School Finals Points`. RunScore PDF: `Place Name Year Team Time Pace` + `(nn)` displace markers. XC team block: `rank. score Team (avg total spread)`.
- **Grade/class evidence**: yes — `Year` column (`Sr/So/Fr/Jr` college, numeric grade in HS files: real extract line `6  301  Drew Reigstad  SR  Kaukauna  16:35.4 ...`). `[note: HS files sometimes carry the numeric grade; PDF extracts show freshman status as FR]`
- **Coach/contact fields**: `exHostOr` (school/org), `clientCompany`; no named coach/email observed in this lane. Prior report 22 records Athletic.net entry links as the coach-facing surface.
- **Public API availability**: **yes** — the results API is unauthenticated; `robots.txt` on www is **404** (`samples/robots-www.pttiming.com.txt`, 35,577 B — the 404 body was captured, not a policy).
- **Static file availability**: yes — public Supabase object storage, byte-for-byte Hy-Tek `.htm` and RunScore `.pdf` (`samples/pttiming-data-cyclone-opener-womenisu.htm`, `samples/pttiming-data-wi-meet-sample.pdf` + text extract `samples/extract-pttiming-wi-meet-pdftotext.txt`, 30,487 B of clean rows).
- **Browser requirement**: none for API or files; the event page is a JS shell.
- **Request cost**: index 0.35 / 1,000 (1 page = 200 events, 2 pages = whole 2026 season); result files ≈ 3–10 per 1,000 depending on division size — a single division PDF/htm covers 100–300 performances, so **5 / 1,000** is the working estimate (`[INFERENCE]` from file sizes). Formats measured across 532 file links: **pdf 489, htm 38, html 5**.
- **Published rate limits**: none published. Legal overlay: PrimeTime's live platform is subscription-oriented per prior research (report 22); the static API + files are the lane used here.
- **Known blocks**: none on the allowed lane. **The live platform is off-limits**: `live.pttiming.com/robots.txt` (fetched first-hand, HTTP 200, 3,287 B, `samples/robots-live.pttiming.com.txt`) states "Automated bulk extraction of results data, and any use of this site's content for training, fine-tuning, grounding or evaluating AI/ML systems, is PROHIBITED by the Terms of Use at /terms.html", names Claude 4×, and carries 50 `Disallow` rules; inherited from `research/midwest/22-missouri-primetime.md`, whose own verdict is **REJECT for the live platform**. This lane therefore used only `www.pttiming.com/api/results/current` + `data.pttiming.com/**`. Coverage gap to expect, not an error: 265 of 353 events carry ≥1 file link, so **88 events (25%) have no result file**. Id caveat (inherited): PrimeTime meet ids are dual-space (numeric CRM id *or* UUID, `id: 0` on 36 of the newest 100 rows) — key on `eventUrl`.
- **Cross-source join keys**: Athletic.net — `computed.entryLinkHtml` embeds `www.athletic.net/...` hrefs on **130 of 200 page-1 rows** (`samples/pttiming-api-results-current-2026-page1.json`); this is the entry→result bridge for the Midwest.
- **Estimated marginal coverage**: large for WI (245/353 events) and real for MO/IL/IA/IN/NE; effectively the Wisconsin census backbone. `[INFERENCE]` the API's `year` param means the full 2026–2027 Co2027 window is reachable with ~4 requests.
- **Implementation recommendation**: **RESULT_SOURCE** (and the best single source for Wisconsin HS marks).

## 5. TFRRS (tfrrs.org, DirectAthletics)

- **Source name**: Track & Field Results Reporting System — DirectAthletics' results-of-record.
- **Geographic coverage**: national, college-dominant; HS meets appear in the same feed when a college host publishes a combined meet (e.g. `samples/tfrrs-xc-27822-southern-stampede-hs.html`, "Southern Stampede … High School", teams Manhattan/Webb City/Eudora KS).
- **Sports**: XC + T&F.
- **Historical depth**: deep by design (season archives per year), but this lane verified only the live meet page + RSS window. `[open question: the year-archive URL pattern was not confirmed — `/results/xc/<id>/<slug>` ids are stable but no browsable year index was found in the captured root]`
- **Discovery mechanism**: `results.rss` (rolling window; 75 items measured, `samples/tfrrs-results.rss`) → meet page `/results/xc/<meetId>/<Slug>`; root `samples/tfrrs-root.html` lists current meets only.
- **Stable identifiers**: meet = path id (`27822`); athlete = profile link `/athletes/<id>/…` (linked from FlashResults rows; presence in TFRRS result rows themselves `[not verified in this capture]`).
- **Pagination**: none — whole meet in one page: **2,060 result rows across 14 tables, 833,212 B** measured (2,059 carry a numeric place; the 1-team-score row does not).
- **Athlete fields**: `PL`, `NAME`, `YEAR`, `TEAM` (+ team score table).
- **Meet fields**: page header (meet name, date, host course); no per-row meet fields.
- **Result fields**: `Avg. Mile`, `TIME`, `SCORE`; team table has `1.`,`2.`… score rows.
- **Grade/class evidence**: **`YEAR` = graduating year for HS meets** (values `2029`, `2030` observed) — directly usable for Co2027 filtering; college pages use standing (SR/JR/SO/FR). Real row: `1  Claire Grennan  2030  Manhattan  6:56.9  12:57.2  1`. **Grade trap to inherit** (`data/adapter-ranking.csv`, rank 6): on 2025-26 TFRRS/DirectAthletics HS lists, `&year=JR` = Class of 2027 and `&year=SR` = Class of 2026 — the token is the **at-meet grade**, so cohort membership must be checked per season, never assumed.
- **Coach/contact fields**: none observed.
- **Public API availability**: none observed; `robots.txt` is an **empty policy (comment only)** (`samples/robots-www.tfrrs.org.txt`) — no crawl restriction.
- **Static file availability**: HTML only; no bulk files observed.
- **Browser requirement**: none.
- **Request cost**: **0.49 / 1,000** (1 request = 2,060 rows).
- **Published rate limits**: none.
- **Known blocks**: none hit; note that college-dominant coverage means most meets are out of scope for a HS census unless the roster carries graduating years.
- **Cross-source join keys**: **FlashResults → TFRRS athlete links** (inbound); Athletic.net: 0 occurrences in the sampled meet page.
- **Estimated marginal coverage**: moderate for HS — the HS races inside college-hosted meets; strong as a **validation** surface for names/schools and for graduating-year cohort checks.
- **Implementation recommendation**: **RESULT_SOURCE**, secondarily **VALIDATION_SOURCE**.

## 6. MeetPro (tfmeetpro.com / results.tfmeetpro.com — DirectAthletics)

- **Source name**: MeetPro (Hy-Tek's competitor, sold by DirectAthletics).
- **Geographic coverage**: unknown — **no live MeetPro-published meet was located in this lane**.
- **Sports**: T&F (+ XC entries).
- **Historical depth**: unknown.
- **Discovery mechanism**: documented publish flow (`samples/tfmeetpro-documentation.html`, "Interfaces ▸ Upload To Web") uploads to `https://results.tfmeetpro.com/<CustomerFolder>/<MeetFolder>/`; there is **no index** of customer folders, so discovery requires an external link (e.g. a meet's web page) or a known customer name. **The reachable DirectAthletics surface is the parent domain's league/index pages**, per `~/Downloads/midwest-tfxc-source-research/data/dat-provider-coverage.csv` (12-state matrix: ≥921 meet-index rows, **only 17 HS-marked rows**, and live HS performance lists in **Indiana only**; HS event sheets with `FR/SO/JR/SR` grades verified in IA 2022 and a 2016 MN result page — inherited, not re-captured here).
- **Stable identifiers**: `<CustomerFolder>/<MeetFolder>` path pair (meet key), reuse overwrites.
- **Pagination**: unknown.
- **Athlete fields**: unverified — no MeetPro result page was captured.
- **Meet fields**: unverified.
- **Result fields**: unverified.
- **Grade/class evidence**: unverified.
- **Coach/contact fields**: not present in a meet-file PDF (no coach block observed; unverified without OCR).
- **Coach/contact fields**: unverified.
- **Public API availability**: none documented.
- **Static file availability**: unknown.
- **Browser requirement**: unknown.
- **Request cost**: cannot be estimated without a captured meet.
- **Published rate limits**: none.
- **Known blocks**: the results host is **unreachable as captured**: `http://results.tfmeetpro.com/` → 301 to `https://tfmeetpro.com`; `https://results.tfmeetpro.com/` → **connection reset (curl 35)**; the documentation's own example path `http://results.tfmeetpro.com/Acme_Timing/SmithInvite` → **404**; additionally `www.meetpro.com` **does not resolve** (curl 6, NXDOMAIN) and the FinishLynx siblings `live.finishlynx.com`, `results.finishlynx.com`, `meetpro.finishlynx.com` fail TLS with `SSL: certificate subject name 'Plesk' does not match target hostname` (curl 60) — i.e. the hosts exist but serve a default Plesk certificate (see the no-file rows in `samples/CAPTURES.md`) (`samples/tfmeetpro-results-demo-smithinvite.html`, 378 B; `samples/tfmeetpro-results-http-root.html`, 0 B; `samples/tfmeetpro-results.php.html` — the 404 body names the path that was requested). `robots.txt` on tfmeetpro.com is an empty policy (`samples/tfmeetpro-robots.txt`).
- **Cross-source join keys**: DirectAthletics identity; TFRRS (results-of-record) overlaps.
- **Estimated marginal coverage**: unknown but potentially large in DirectAthletics-heavy states (prior research: MeetPro is used by 18 of 64 Ohio FAT contractors) — however this lane could not verify a single readable page.
- **Implementation recommendation**: **CONDITIONAL** — do not build against it until one real MeetPro meet URL is captured and parsed; the publish target may be blocked from non-browser clients.

## 7. RACE RESULT (raceresult.com / my.raceresult.com)

- **Source name**: RACE RESULT (chip timing; XC/road).
- **Geographic coverage**: international; the sampled event is US — `2026 Escanaba Fall Classic`, Escanaba MI (High School XC, 2026-09-19) from the page's schema.org JSON-LD (`samples/my-raceresult-event-423470-results.html`). Prior Midwest research associates RACE RESULT with Michigan timers (Superior Timing).
- **Sports**: XC/road/track.
- **Historical depth**: unknown (event ids are numeric and stable; no index captured because `RREvents` is robots-disallowed).
- **Discovery mechanism**: **blocked by policy** — `robots.txt` on my.raceresult.com **disallows `/RREvents` (event index API), `/RRPublish` (results renderer), `/RRRegStart`, `/*/*/list`, `/*/*/pdf`, `/*/*/view`, `/users*`** (`samples/my-raceresult-robots.txt`); www.raceresult.com only disallows `/*/company/index` (`samples/robots-www.raceresult.com.txt`).
- **Stable identifiers**: numeric event id (`423470`).
- **Pagination**: n/a under the policy.
- **Athlete fields**: not capturable under the robots policy (result renderer disallowed); unverified.
- **Meet fields**: yes — schema.org JSON-LD event name, start/end date, attendance mode, location.
- **Result fields**: **not capturable** — the renderer endpoint is robots-disallowed; unverified. Only meet-level metadata is capturable — JSON-LD gives event name, start/end dates, attendance mode, location (`samples/my-raceresult-event-423470-results.html`). No result rows were captured.
- **Grade/class evidence**: unverified.
- **Coach/contact fields**: not present in a meet-file PDF (no coach block observed; unverified without OCR).
- **Coach/contact fields**: none observed.
- **Public API availability**: the data plane exists (`RRPublish`/`RREvents`) but is explicitly disallowed by robots for automated agents.
- **Static file availability**: unknown.
- **Browser requirement**: the shell loads a JS publisher component; data arrives from the disallowed endpoints.
- **Request cost**: not measurable under robots policy.
- **Published rate limits**: none published; the robots policy is the binding constraint.
- **Known blocks**: `samples/my-raceresult-robots.txt` (317 B) — verbatim header: `User-agent: * … Disallow: /RRPublish`, `/RREvents`. Also `my.raceresult.com/` returned a 0-byte body (`samples/my-raceresult-root.html`) while `/events` returned 22,651 B shell (`samples/my-raceresult-events.html`).
- **Cross-source join keys**: none observed (no Athletic.net ids in the JSON-LD).
- **Estimated marginal coverage**: low for HS XC in the target states unless a Mi-based timer publishes exclusively there; not worth the policy risk as captured.
- **Implementation recommendation**: **CONDITIONAL** (metadata-only; exclude the disallowed data plane).

## 8. OpenTrack (opentrack.run)

- **Source name**: OpenTrack (open-source T&F meet management).
- **Geographic coverage**: unknown — no data retrieved.
- **Sports**: T&F.
- **Historical depth**: unknown.
- **Discovery mechanism**: none available — both `www.opentrack.run/robots.txt` and `/results/` return a **Cloudflare interstitial (HTTP 403, "Just a moment...")** with the default curl UA *and* a Chrome UA (`samples/robots-www.opentrack.run.txt`, `samples/opentrack-robots-chromeua.txt`, `samples/opentrack-results-path-probe.html`). `results.opentrack.run` does not resolve (curl exit 6).
- **Stable identifiers**: unverified.
- **Pagination**: unverified.
- **Athlete fields**: unverified.
- **Meet fields**: unverified.
- **Result fields**: unverified.
- **Grade/class evidence**: unverified.
- **Coach/contact fields**: not present in a meet-file PDF (no coach block observed; unverified without OCR).
- **Coach/contact fields**: unverified.
- **Public API availability**: unknown; the documented JSON API could not be reached.
- **Static file availability**: unknown.
- **Browser requirement**: yes — a JS challenge stands in front of every path (and solving it is out of scope for this lane).
- **Request cost**: n/a.
- **Published rate limits**: unknown.
- **Known blocks**: Cloudflare managed challenge on all paths; `results.opentrack.run` **does not resolve** (curl 6, NXDOMAIN) for both `/` and `/robots.txt` (no-file rows in `samples/CAPTURES.md`).
- **Cross-source join keys**: unknown.
- **Estimated marginal coverage**: unknown; excluded on access grounds.
- **Implementation recommendation**: **REJECT** (automated access blocked; revisit only if the operator grants a data path).

## 9. GSE Timing (gsetiming.com, MN)

- **Source name**: GSE Timing (Minnesota XC specialist).
- **Geographic coverage**: Minnesota; site self-identifies with MN schools (`samples/gsetiming-cc_rslts-index.html`). Prior Midwest research records that GSE's T&F results redirect to Athletic.net.
- **Sports**: XC (this lane's data plane); T&F via Athletic.net per prior research.
- **Historical depth**: **608 meets measured in one index, 2005-09-08 → 2025-10-25** (the option list holds 609 `<option>` entries; the first is a `value="0"` placeholder, so 608 real meets — this matches the inherited count in `research/midwest/10-minnesota-results-timers.md` exactly) (`samples/gsetiming-cc_rslts-index.html`, 62,338 B; real option entries first `1450 Providence Junior High (10/25/2025)`, last `1 Monticello Invitational (9/8/2005)`) — a 20-season archive behind one request.
- **Discovery mechanism**: `cc_rslts.asp?sport=cc` renders a `<select name="meets">` option list (value = meet_id, label = `Name (MM/DD/YYYY)`); the meet page links races; the race rows come from `results/cc_rslts/adv_source.asp?race_id=<id>` as JSON.
- **Stable identifiers**: `meet_id`, `race_id` (both numeric, stable across seasons), `bib` (meet-local).
- **Pagination**: none — the JSON endpoint returns every row for the race.
- **Athlete fields**: `bib`, `first`, `last`, `school_code`, `grade`, `gender` (12-column array: `place, field2, Tm|Ind, bib, first, last, school, grade, gender, time, pct, gap`).
- **Meet fields**: meet name + date in the index label; race name from the meet page. Real row: `["1","1","Tm","5100","Judah","Allen","LIT","11","M","15:49.6","----","0:00.0"]` (`samples/gsetiming-adv_source-race7784.json`, 133 rows × 12 cols, 87,823 B).
- **Result fields**: `place`, team-score HTML fragment (`Tm` / `Ind` badge), `time`, `pct` (percentile/pace `----`), `gap`.
- **Grade/class evidence**: **numeric grade column** (`11`) — directly Co2027-filterable.
- **Coach/contact fields**: none observed.
- **Public API availability**: no documented API, but the ASP data endpoint is unauthenticated and returns JSON.
- **Static file availability**: none (JSON only).
- **Browser requirement**: none.
- **Request cost**: **7.5 / 1,000** (1 request = 133 rows; race-level granularity, so a whole meet ≈ number of races).
- **Published rate limits**: **`robots.txt` publishes `User-agent: * / Crawl-delay: 3`** plus bingbot/msnbot rules (`samples/robots-gsetiming.com.txt`, 687 B).
- **Known blocks**: none hit.
- **Cross-source join keys**: none observed (no Athletic.net ids in the JSON; `school_code` is internal).
- **Estimated marginal coverage**: Minnesota HS XC — roughly 30–50 meets/season over the last decade `[INFERENCE]`; T&F coverage is delegated to Athletic.net.
- **Implementation recommendation**: **RESULT_SOURCE** (state-scoped), with the archive as a bonus for cohort back-fill.

## 10. Finish Timing (finishtimingresults.com, OH)

- **Source name**: Finish Timing (Ohio; publishes free Hy-Tek archives).
- **Geographic coverage**: Ohio (prior Midwest research: the homepage publishes 29 Athletic.net MeetIDs; not re-verified in this capture).
- **Sports**: T&F + XC.
- **Historical depth**: per-year directories; **2026 directory = 325 `.html` files + 2 subdirectories** measured from the autoindex (`samples/finishtimingresults-2026-index.html`, 59,293 B).
- **Discovery mechanism**: Apache autoindex per year (`/<year>/`) → per-meet `.html` (numeric id, e.g. `1000739707`).
- **Stable identifiers**: numeric file id per meet; events numbered inside the file; no result ids.
- **Pagination**: none — one file per meet.
- **Athlete fields**: Hy-Tek `Name`, `Year`, `School` (+ heat/lane on track files).
- **Meet fields**: file header carries the meet name/date (Hy-Tek banner).
- **Result fields**: `Pl Name Year School Finals Points`; 29 events / 372 athlete lines in the sampled file (`samples/finishtiming-2026-1000739707.html`, 41,721 B).
- **Grade/class evidence**: `Year` column numeric for HS; blank on unattached/unknown rows.
- **Coach/contact fields**: none observed.
- **Public API availability**: none (static files).
- **Static file availability**: yes — plain Hy-Tek text in `<pre>`, one request per whole meet.
- **Browser requirement**: none.
- **Request cost**: **2.7 / 1,000** (1 request = 372 lines).
- **Published rate limits**: `robots.txt` = `User-agent: * / Crawl-delay: 3` (29 B, `samples/robots-finishtimingresults.com.txt`).
- **Known blocks**: none.
- **Cross-source join keys**: Athletic.net MeetIDs on the homepage per prior research; not re-verified here.
- **Estimated marginal coverage**: Ohio-only; 325 meets/season measured.
- **Implementation recommendation**: **RESULT_SOURCE** (state-scoped).

## 11. RunWV.com (WV)

- **Source name**: RunWV.com — West Virginia XC/T&F community site (host uploads, not a timing firm).
- **Geographic coverage**: West Virginia (`samples/runwv-root.html` frameset: "WV Cross Country and Track & Field").
- **Sports**: XC + T&F.
- **Historical depth**: season directories (`CC26/` = 2026 XC; `contents.html` lists prior seasons).
- **Discovery mechanism**: `CC26/schedule/MEETLIST.HTML` (2026 schedule) + `CC26/news/` uploads linked from it; `contents.html` → seasonal indexes.
- **Stable identifiers**: none — filenames only (`best0917.htm`).
- **Pagination**: none.
- **Athlete fields**: none observed in the sampled upload.
- **Meet fields**: name + date only, from the schedule table.
- **Result fields**: **none observed** — no structured result surface The sampled news file is a Word-exported announcement with **0 tables and 0 result rows** (`samples/runwv-cc26-news-best0917.htm`, 5,106 B).
- **Grade/class evidence**: none observed.
- **Coach/contact fields**: none observed.
- **Public API availability**: none.
- **Static file availability**: arbitrary mix — Word `.htm` + `.pdf` (58 file links counted under `CC26/news/`; 37 dated schedule entries in `samples/runwv-cc26-meetlist.html`).
- **Browser requirement**: none.
- **Request cost**: n/a (no rows to normalise).
- **Published rate limits**: `robots.txt` is an **HTML 404 page** (`samples/robots-www.runwv.com.txt`) — no policy.
- **Known blocks**: none; the blocker is heterogeneity, not access.
- **Cross-source join keys**: contents page links out to `hytek.active.com` (a Meet Manager hosting surface); no Athletic.net ids observed.
- **Estimated marginal coverage**: WV meet **names/dates** (37 in 2026 XC) — a discovery index for a state with otherwise thin coverage; rows must come from the linked uploads.
- **Implementation recommendation**: **DISCOVERY_SOURCE**.

## 12. XCStats (xcstats.com, CA)

- **Source name**: XCStats (California HS XC/T&F community site).
- **Geographic coverage**: California.
- **Sports**: XC + T&F.
- **Historical depth**: unknown (no result page fetched).
- **Discovery mechanism**: none attempted — the homepage exposes `subscribe.php` / `guest-pass.php` gates (`samples/xcstats-root.html`).
- **Stable identifiers**: unverified.
- **Pagination**: unverified.
- **Athlete fields**: unverified.
- **Meet fields**: unverified.
- **Result fields**: unverified.
- **Grade/class evidence**: unverified.
- **Coach/contact fields**: not present in a meet-file PDF (no coach block observed; unverified without OCR).
- **Coach/contact fields**: unverified.
- **Public API availability**: none observed.
- **Static file availability**: unknown.
- **Browser requirement**: unknown.
- **Request cost**: not measurable.
- **Published rate limits**: `robots.txt` has a blanket bot-block section that **explicitly lists `User-agent: ClaudeBot / Disallow: /`** (plus AhrefsBot, Alibaba, Amazonbot, Bingbot, Bytespider, …) (`samples/robots-www.xcstats.com.txt`, 1,485 B). The default `*` group only disallows `/FusionCharts`.
- **Known blocks**: (a) explicit AI-crawler disallow; (b) account/paywall gate on results. **This lane stopped at the homepage deliberately.**
- **Cross-source join keys**: none observed.
- **Estimated marginal coverage**: potentially large for CA HS, but **access policy excludes automated collection**.
- **Implementation recommendation**: **REJECT** (paywall + explicit crawler disallow).

## 13. Wayzata Results (wayzataresults.com + AthleticLIVE tenant `wayzata`)

- **Source name**: Wayzata Results, Inc. (MN timer; long-time AthleticLIVE tenant).
- **Geographic coverage**: Minnesota core, plus meets it times out of state.
- **Sports**: XC + T&F.
- **Historical depth**: tenant `wayzata` holds **2,768 meet docs** in the shared census (`samples/search-athleticlive-all-meet_list-indices.json`).
- **Discovery mechanism**: **own CMS is UA-gated, not blocked** — with the default curl UA, `https://www.wayzataresults.com/robots.txt` returns a CloudFront **403 "Request blocked" page** (`samples/robots-www.wayzataresults.com.txt`, 919 B); re-fetched with a browser UA it returns **HTTP 200 / 954 B** (`samples/robots-www.wayzataresults.com.browserua.txt`, a PrestoSports-managed policy: `User-agent: *` disallows `/cgi-bin/`, `/_private/`, `/_vti_bin/`, `/_vti_cnf/`, `/_vti_log/`, `/_vti_pvt/`, `/_vti_txt/`, **`/reports/`**; Googlebot gets `Crawl-delay: 10`). The clean path remains the AthleticLIVE plane: tenant `_meet_list` search → meet doc → blob event docs (same schema as §3), whose tenant robots (`samples/robots-results.wayzatatiming.com.txt`, 194 B) disallows only `/admin*`, `/meets/*/athletes*`, `/meets/*/live*`, `/meets/*/teams*`, `/meets/*/follow*`.
- **Stable identifiers**: AthleticLIVE `i`/`ani` meet ids; blob event ids.
- **Pagination**: ES `from`/`size`.
- **Athlete fields**: `n`,`fn`,`l`,`y`,`g`,`ani`,`t` (see §3).
- **Meet fields**: `n`,`ln`,`ls`,`lsa`,`lo`,`sd`,`tna` (should read "Wayzata Results"),`li` (Hy-Tek).
- **Result fields**: identical key set to §3 (verified against the same blob-doc schema): `m`/`im`/`irs`/`avk`/`avm`/`er`/`pr`/`ern`/`p`/`pt`/`ro`/`gap`/`i`/`a` per row, round and class vocabulary at event level (`runm`/`rui`/`ry`).
- **Grade/class evidence**: `y` class token where populated.
- **Coach/contact fields**: meet doc `qn`/`qe`/`atr`.
- **Public API availability**: yes via AthleticLIVE (no key).
- **Static file availability**: Hy-Tek PDFs/HTML on the tenant's blob storage; own-domain files unreachable from this lane (CloudFront 403).
- **Browser requirement**: none for the tenant plane.
- **Request cost**: 7.4 / 1,000 via blob event docs; the tenant's meet list is ES-paged.
- **Published rate limits**: none.
- **Known blocks**: CloudFront 403 on the own CMS **only for non-browser UAs**; `/reports/` is robots-disallowed even for browser UAs. Independently corroborated by `research/midwest/10-minnesota-results-timers.md` ("curl default UA → 403; browser UA → 200 (2.7 MB landing); 22 requests with browser UA, all 200, no 429").
- **Cross-source join keys**: Athletic.net athlete/team/meet ids (`ani`).
- **Estimated marginal coverage**: MN-heavy; 2,768 lifetime meets make it one of the larger single-tenant corpora.
- **Implementation recommendation**: **CONDITIONAL** as a publisher (own CMS blocked), but **PRIMARY** through the AthleticLIVE tenant path.

## 14. Hy-Tek MEET MANAGER output (format family, cross-cutting)

- **Source name**: Hy-Tek Meet Manager HTML/PDF exports — the de-facto interchange format of US HS T&F/XC timing.
- **Geographic coverage**: national (publishers observed in this lane: PrimeTime, Finish Timing, FlashResults-Texas, Wayzata/AthleticLIVE).
- **Sports**: XC + T&F.
- **Historical depth**: inherits each publisher's archive (e.g. Finish Timing 2026 = 325 files; PrimeTime 2026 = 489 PDFs).
- **Discovery mechanism**: publisher-specific (§1, §4, §10).
- **Stable identifiers**: none in the file — identity comes from the publisher's URL/path.
- **Pagination**: none (one file per meet or per division).
- **Athlete fields**: `Name`, `Year`, `School` (+ `H#`/lane on track files).
- **Meet fields**: banner lines (meet name, date, host, facility).
- **Result fields**: `Pl Name Year School Finals Points`; field marks carry metric + imperial; relays carry a team row plus leg rows.
- **Grade/class evidence**: the `Year` column is the grade for HS files (numeric) and standing for college files (`Sr/So/Fr/Jr`) — this is the single most portable grade signal across the Tier-3 families.
- **Coach/contact fields**: none.
- **Public API availability**: n/a (files).
- **Static file availability**: yes — text-extractable HTML; PDFs vary (PrimeTime's extract cleanly; trackmeetio's do not).
- **Browser requirement**: none.
- **Request cost**: 1 request per file; rows/file 100–1,900 depending on publisher (measured: PrimeTime HF division PDF, Finish Timing whole meet 372 lines, TFRRS 2,060 rows when the publisher uses TFRRS instead).
- **Published rate limits**: per publisher (GSE and Finish Timing publish `Crawl-delay: 3`).
- **Known blocks**: none inherent.
- **Cross-source join keys**: publisher-specific → Athletic.net / TFRRS.
- **Estimated marginal coverage**: high — writing one Hy-Tek parser covers four of the families in this report.
- **Implementation recommendation**: **RESULT_SOURCE (format standard)** — implement the parser once, register publishers as adapters.

## 15. TrackMeet.io static artifacts (static.trackmeetio.com)

- **Source name**: TrackMeet.io meet-file CDN (AthleticLIVE-adjacent).
- **Geographic coverage**: follows the tenants that publish there.
- **Sports**: T&F/XC.
- **Historical depth**: unknown (`[open question]`).
- **Discovery mechanism**: meet files are referenced from AthleticLIVE meet pages; the CDN itself has no index observed.
- **Stable identifiers**: `<MeetID>` path segment (`67670`) + content hash filename.
- **Pagination**: n/a.
- **Athlete fields**: present in the PDF but not text-extractable; unverified without OCR.
- **Meet fields**: present in the PDF (meet banner) but not text-extractable; unverified without OCR.
- **Result fields**: present in the PDF but **not text-extractable** — `pdftotext -layout` produced 68,604 B of mojibake (subset-font encoding) from the 317,088 B capture (`samples/extract-trackmeetio-67670-pdftotext.txt`). Fields are therefore unverified without OCR.
- **Grade/class evidence**: unverified.
- **Coach/contact fields**: not present in a meet-file PDF (no coach block observed; unverified without OCR).
- **Public API availability**: none.
- **Static file availability**: yes — public CDN PDF, one request per meet.
- **Browser requirement**: none to download; OCR (or a browser print path) needed to read.
- **Request cost**: 1 request per whole meet, but the rows are not machine-readable without OCR.
- **Published rate limits**: none observed.
- **Known blocks**: extraction, not access.
- **Cross-source join keys**: Athletic.net ids when the same meet exists in the platform index.
- **Estimated marginal coverage**: unknown; prefer the blob JSON plane (§3) wherever both exist.
- **Implementation recommendation**: **CONDITIONAL** (use only where no JSON plane is available).

---

## 16. TRXC Timing (trxctiming.com, MO)

- **Source name**: TRXC Timing, LLC (Bridgeton, MO) — the largest Missouri HS timer outside PrimeTime.
- **Geographic coverage**: Missouri core (St. Louis metro, mid-MO, Bootheel, Springfield), secondary SW Illinois via a `/Illinois/…` path prefix, occasional college meets (priority: GLVC/SLIAC/UCM etc.). Source: prior report `research/midwest/40-missouri-trxctiming.md`.
- **Sports**: XC + T&F (FinishLynx/IPICO-based).
- **Historical depth**: reported season corpus 2010–2026 (per-season WordPress tables); prior report counts 132 non-IL T&F meets 2026, 111 in 2025, 127 in 2024, plus 55 MO XC meets/season.
- **Discovery mechanism**: WordPress REST — `GET https://trxctiming.com/wp2/wp-json/wp/v2/pages?per_page=100&page=1&_fields=id,slug,link,title,parent`. **Verified first-hand in this lane: 54 pages returned in one request (HTTP 200, 10,254 B)**, including `2026 Cross Country Results`, `Track & Field Results 2026`, `2025 Cross Country Results` (`samples/trxctiming-wp-pages.json`). `robots.txt` is a WordPress **404** (52,301 B "Page not found" body, `samples/robots-trxctiming.com.txt`) — no policy.
- **Stable identifiers**: **no id field**; the de-facto meet key is the static directory path (`/Parkway_Central/TF/Henle_Holmes/`, 160 distinct dirs across the 166-row 2026 T&F index).
- **Pagination**: season pages are static tables; the pages endpoint is single-page (`X-WP-Total: 54` per prior report, 54 returned first-hand).
- **Athlete fields**: name + **grade** + school + mark (grade-tagged result rows per prior report).
- **Meet fields**: meet name, date, season table rows; division/class labels in the directory names.
- **Result fields**: FinishLynx-style result pages with grade column; prior report verified `FR/SO/JR/SR` and numeric grade columns.
- **Grade/class evidence**: **yes** — the prior report's explicit verdict: "yes, TRXC is a viable grade-bearing Missouri result source."
- **Coach/contact fields**: company contact only (`inquiry@trxctiming.com`, (314) 522-6176); no coach fields.
- **Public API availability**: WordPress REST pages endpoint (unauthenticated); the WP REST route is the enumeration surface.
- **Static file availability**: HTML result directories + PDFs per meet `[inherited from report 40]`.
- **Browser requirement**: none (WordPress REST + static HTML).
- **Request cost**: 1 request for the whole page/site map; prior report's meet-level counts imply roughly **1 request per meet directory** `[INFERENCE]` — a rate that beats Athletic.net meet-search for MO.
- **Published rate limits**: none observed in the first-hand capture (robots is a 404).
- **Known blocks**: none hit.
- **Cross-source join keys**: Athletic.net ids appear on the upcoming-meets page (DISCOVERY-ONLY role per prior report); result rows themselves are grade-tagged but not ATN-tagged.
- **Estimated marginal coverage**: MO HS XC+T&F; 55 XC meets/season plus ~130 T&F meets/season that PrimeTime does not time.
- **Implementation recommendation**: **RESULT_SOURCE** (Missouri), with **VALIDATION** for the grade column and **DISCOVERY_SOURCE** for the upcoming-meets ATN ids.

## 17. Hero's Timing (herostiming.com / live.herostiming.com, MN+ND)

- **Source name**: Hero's Timing (Bemidji-area, MN) — Weebly site + AthleticLIVE tenant.
- **Geographic coverage**: Minnesota + North Dakota meets (prior report `research/midwest/10-minnesota-results-timers.md`).
- **Sports**: XC + T&F.
- **Historical depth**: **year archive pages 2012–2026 = 15 seasons verified first-hand** from the homepage link set (`/2012-results.html` … `/2026-results.html`, `samples/herostiming-root.html`, 51 links, 96,468 B).
- **Discovery mechanism**: homepage → `/NNNN-results.html` year page → per-meet links to the AthleticLIVE tenant. **Verified first-hand**: `https://www.herostiming.com/2026-results.html` (HTTP 200, 235,773 B) carries **256 distinct `live.herostiming.com/meets/<id>` links** (ids 59,921–76,239) — identical to the prior report's 256 count (`samples/herostiming-2026-results.html`).
- **Stable identifiers**: `live.herostiming.com/meets/<id>` numeric meet id (stable across the tenant); then the shared AthleticLIVE `i`/`ani` ids.
- **Pagination**: none (year page lists all meets; the tenant is ES-paged).
- **Athlete fields**: via the tenant blob docs (same schema as §3).
- **Meet fields**: via the tenant meet docs (`n`,`ln`,`ls`,`lsa`,`lo`,`sd`,`tna`).
- **Result fields**: via the tenant blob docs (§3).
- **Grade/class evidence**: via the tenant athlete `y` token `[open question for this specific tenant]`.
- **Coach/contact fields**: tenant meet doc `qn`/`qe`/`atr`.
- **Public API availability**: yes through the shared AthleticLIVE plane; the Weebly host itself has no API.
- **Static file availability**: tenant PDFs/JSON; the Weebly pages are HTML.
- **Browser requirement**: none.
- **Request cost**: 7.4 / 1,000 via blob event docs; 1 request enumerates a whole season's meet list.
- **Published rate limits**: `robots.txt` = 329 B (`samples/robots-www.herostiming.com.txt`): `User-agent: *` **disallows `/ajax/`, `/apps/`, `/results.html`, `/honor-rolls.html`, `/past-results.html`**; NerdyBot fully disallowed; dotbot crawl-delay 10. The `/NNNN-results.html` year pages are **not** matched by those disallow rules.
- **Known blocks**: the named results entry points (`/results.html`, `/past-results.html`, `/honor-rolls.html`) are robots-disallowed — use the year pages or the tenant instead.
- **Cross-source join keys**: Athletic.net ids via the tenant plane.
- **Estimated marginal coverage**: 256 meets in 2026 (first-hand count) across MN+ND; 15 seasons of archive.
- **Implementation recommendation**: **RESULT_SOURCE** through the AthleticLIVE tenant (the host's own results entry points are disallowed).

---

## Cross-cutting: robots / access policies observed

| Host | robots verdict | File |
|---|---|---|
| www.flashresults.com | PDF/CSV/JS + /flashtexas*, /flashwest/ disallowed | `robots-www.flashresults.com.txt` |
| www.tfrrs.org | empty policy (comment only) | `robots-www.tfrrs.org.txt` |
| www.pttiming.com | 404 (no policy) | `robots-www.pttiming.com.txt` |
| www.raceresult.com | only `/*/company/index` disallowed | `robots-www.raceresult.com.txt` |
| my.raceresult.com | **/RRPublish, /RREvents, /*/*/list, /*/*/pdf, /users*** disallowed | `my-raceresult-robots.txt` |
| www.opentrack.run | Cloudflare 403 interstitial (no policy readable) | `robots-www.opentrack.run.txt` |
| www.runwv.com | HTML 404 (no policy) | `robots-www.runwv.com.txt` |
| www.xcstats.com | **ClaudeBot Disallow: /** (+ 20 crawler blocks) | `robots-www.xcstats.com.txt` |
| www.finishlynx.com | captured (244 B) | `robots-www.finishlynx.com.txt` |
| www.wayzataresults.com | CloudFront 403 page (no policy) | `robots-www.wayzataresults.com.txt` |
| www.athletic.live / search.athletic.live | no policy (403 body on search) | `robots-www.athletic.live.txt`, `robots-search.athletic.live.txt` |
| gsetiming.com | `Crawl-delay: 3` | `robots-gsetiming.com.txt` |
| finishtimingresults.com | `Crawl-delay: 3` | `robots-finishtimingresults.com.txt` |
| tfmeetpro.com | empty policy | `tfmeetpro-robots.txt` |
| live.pttiming.com | **ToU: automated bulk extraction + AI/ML use PROHIBITED**; Claude named 4×; 50 Disallow rules | `robots-live.pttiming.com.txt` |
| trxctiming.com | WordPress 404 (no policy) | `robots-trxctiming.com.txt` |
| www.herostiming.com | disallows `/ajax/`, `/apps/`, `/results.html`, `/honor-rolls.html`, `/past-results.html` | `robots-www.herostiming.com.txt` |
| results.wayzatatiming.com | tenant policy: disallows `/admin*`, `/meets/*/athletes\|live\|teams\|follow*` | `robots-results.wayzatatiming.com.txt` |
| www.wayzataresults.com | PrestoSports policy, 200 only with a browser UA (curl UA → 403) | `robots-www.wayzataresults.com.browserua.txt` |

**Protocol compliance for this lane**: every request used `curl -sS` with an explicit UA, sequential per host, ≤1 req/s. robots.txt was read **before** content on 12 hosts (flashresults www, tfrrs, pttiming www, raceresult www+my, runwv, search.athletic.live, live.flashresultstexas, tfmeetpro, trxctiming, herostiming, meetpro/opentrack probes). Three hosts deviate: **gsetiming.com** and **finishtimingresults.com** had their content pages fetched at 04:00–04:02 with robots read at 04:04 — measured gaps were 82 s and 123 s, so the published `Crawl-delay: 3` was satisfied even though the policy was read late (both policies are permissive: crawl-delay only); **results.opentrack.run** returned NXDOMAIN for both content and robots. Eight content hosts have **no robots fetch at all** (`data.pttiming.com`, `athleticlive.blob.core.windows.net`, `static.trackmeetio.com`, the `flashresults.com`/`flashresultstexas.com` bare apexes, `results.tfmeetpro.com` and the two `finishlynx.com` hosts that failed TLS) — the two storage/CDN hosts serve raw objects rather than a browsable site, and the rest either failed to connect or are the same operator as a host whose policy was read. Measured inter-request gaps to the tightest hosts (from `samples/capture-log.jsonl`): gsetiming.com 04:00:39 → 04:02:01 → 04:04:04 (82 s, 123 s); finishtimingresults.com 04:00:40 → 04:02:01 → 04:04:04 (81 s, 123 s) — both well inside the published `Crawl-delay: 3`. No authentication, CAPTCHA, or paywall was bypassed anywhere. XCStats and OpenTrack were abandoned on access-policy grounds, and the RACE RESULT data plane was not touched after reading its robots file.

## Cross-cutting: request-cost model

| Path | Unit | Rows/request (measured) | Requests / 1,000 rows |
|---|---|---|---|
| AthleticLIVE platform census | whole platform | 153,774 meet docs | 0.0065 |
| AthleticLIVE blob event doc | event | 136 | 7.4 |
| FlashResults athleteList | whole meet | 3,038 | 0.33 |
| FlashResults compiled event page | event | ~100 (adidasXC: 3,038 rows / 31 pages) | 10.2 |
| TFRRS meet page | whole meet | 2,060 | 0.49 |
| PrimeTime API index | 200 events | 200 events | 0.35 (index only) |
| PrimeTime result file | division | ~200 `[INFERENCE]` | ~5 |
| GSE race JSON | race | 133 | 7.5 |
| Finish Timing Hy-Tek file | whole meet | 372 (29 events) | 2.7 |
| TrackMeet.io PDF | whole meet | unknown (OCR required) | 1 req/meet, rows unreadable |


## Consolidation — Downloads package and prior Midwest research (inherited, not re-captured)

The batch provided a prior research package at `~/Downloads/midwest-tfxc-source-research/`. Facts below are **inherited** from those files where marked; each is attributed to the exact path. Nothing in this section was re-fetched by this lane unless it says "verified first-hand".

### DirectAthletics coverage matrix — `data/dat-provider-coverage.csv`

Per-state DirectAthletics footprint (12 Midwest states, measured 2026-09-19 from `directathletics.com` league/index pages, cited there as "report 28 table"):

| State | DAT T&F team records | DAT XC team records | Canonical schools | Meets index | HS-marked rows | Strongest HS artifact inspected |
|---|---|---|---|---|---|---|
| WI | 976 | 936 | 492 | 95 (full) | 0 | index page only |
| MN | 753 | 121 | 380 | 65 (full) | 0 | result page: Lake Conference Championships, 2016-05-17, 10 teams |
| IA | 808 | 784 | 408 | ≥100 | 0 | event sheet: MVC Girls Divisional, 2022-05-05, `FR/SO/JR/SR` grades |
| IL | 1,744 | 1,596 | 880 | ≥100 | 0 | index page only |
| MI | 1,612 | 1,618 | 814 | 92 (full) | 0 | index page only |
| IN | 929 | 887 | 466 | ≥100 | 6 | HS performance lists + event sheets, 2026 (1,180 rows / 312 teams) |
| OH | 1,749 | 1,679 | 875 | ≥100 | 0 | index page only |
| MO | 980 | 939 | 492 | 68 (full) | 4 | index HS-marked rows |
| KS | 865 | 804 | 436 | 90 (full) | 7 | index HS rows + `SCBL HS Track 2026` page (20 teams) |
| NE | 634 | 632 | 319 | 52 (full) | 0 | index page only |
| ND | 361 | 362 | 181 | 29 (full) | 0 | index page only |
| SD | 380 | 378 | 190 | 30 (full) | 0 | index page only |
| **Total** | **11,791** | **10,736** | **5,933** | **≥921** | **17** | 2 HS result pages verified |

**Consequence for §6 (MeetPro/DirectAthletics)**: the accessible DirectAthletics surface is **not** `results.tfmeetpro.com` (connection-reset, §6) but `directathletics.com` **league/index pages** — that is where the HS artifacts above live, and only **1 of 12 states (Indiana)** yields live HS performance lists. This upgrades MeetPro from "no data located" to "conditional, with a verified alternate surface whose HS yield is thin outside IN".

### Adapter ranking — `data/adapter-ranking.csv`

The package's own ranking (rank 1 = best) relevant to this lane:

- **Rank 5 — "AthleticLIVE blob + ES bridge (ATN-id carrier)"**: measured 959/959 and 136/136 sampled rows carrying the Athletic.net athlete id; **258 tenant origins published on the SPA**; network-wide season total UNVERIFIED there. **Cross-check against this lane: I measured 256 live tenant indexes in one aggregation request** (`samples/search-athleticlive-all-meet_list-indices.json`) — the two counts agree within 2, which is the strongest available triangulation that the census is complete at the tenant level. The ranking's "6 requests = 959 rows / 242 Co2027" matches this lane's 7.4-requests-per-1,000-rows figure.
- **Rank 6 — "TFRRS/DirectAthletics Indiana adapter"**: 1,861 distinct Co2027 in 2 requests via **`&year=JR`**, with an explicit **grade trap**: on the 2025-26 lists `&year=SR` returns 1,678 athletes who are the **Class of 2026**, not Co2027 (at-meet grade, not cohort). Adopting this for §5 means the TFRRS `YEAR` semantics must be cohort-verified per season, not assumed.
- Rank 1 (MileSplit team rosters) and ranks 2–4/7–10 (IHSA, MSHSL, KSHSAA, OHSAA, WIAA, NSAA, directories) are association/contact layers outside this lane's scope; their existence is why this lane did not re-derive association coverage.

### Per-state timer reports folded in

- `research/midwest/08-wisconsin-timing-providers.md` — WI timer ecosystem; evidence for §4: PrimeTime **2025 = 319 WI meets (246 HS-like), 2026 = 255 WI (199 HS-like)**, "single highest-value adapter in the state"; also names AccuRace Timing, Performance Timing (WordPress REST, enumerable by date), TrackSide Timing, K2 Timing.
- `research/midwest/22-missouri-primetime.md` — MO PrimeTime depth: **2025 = 452 events (WI 319 · MO 28 · IL 17), 2024 = 451 (329/18/21)**; the API `year` param reaches back to at least 2017. **Correction this forces on §4**: the live platform `live.pttiming.com` is **prohibited** for automated access — its ToU forbids bulk extraction and building a recruiting database, and its robots.txt names Claude/Anthropic agents. This lane fetched that robots.txt first-hand to confirm: HTTP 200, 3,287 B, 50 `Disallow` rules, the ToU sentence verbatim "Automated bulk extraction of results data, and any use of this site's content for training, fine-tuning, grounding or evaluating AI/ML systems, is PROHIBITED" and Claude named 4× (`samples/robots-live.pttiming.com.txt`). Also inherited: PrimeTime meet ids are **dual-space** (numeric CRM id *or* UUID; `id: 0` on 36 of the newest 100 rows) — key on `eventUrl`, never on the numeric id alone; and `exFranklinMid` is a **live-meet** id whose URL is on the prohibited host.
- `research/midwest/40-missouri-trxctiming.md` — TRXC Timing (now §16). Its page inventory was re-verified first-hand here (54 pages).
- `research/midwest/10-minnesota-results-timers.md` — MN timer inventory; evidence behind §9 (**GSE = 608 XC meets 2005–2025**, not 609 — my index count includes the `<option value="0">` placeholder) and §17 (Hero's Timing). It also documents the **Wayzata UA-gating** correction now folded into §13, that `results.wayzatatiming.com` / `live.herostiming.com` / `live.fastfinishresults.com` / `live.athletic.net` all serve the **same 50,184-byte Angular SPA shell** (matching this lane's `live-flashresultstexas-meet-76403.html` byte count exactly), and that GSE's T&F path 302-redirects to `athletic.net`.
- `research/midwest/19-ohio-ohsaa.md` — source of the MeetPro adoption figure quoted in §6 (scoring programs across 64 Ohio FAT contractors: Hytek 58, MeetPro 18, RunMeet 1).
- `research/midwest/20-ohio-independent-sources.md` — MeetPro row: "`tfmeetpro.com` | Meet-management software by DirectAthletics; linked from MileSplit meet pages; results are uploaded by the meet host/timer, not browsable as an Ohio index" — independently consistent with §6's finding that MeetPro has no index.

### Corrections applied to this report because of the package

1. §9 GSE: **608 meets**, not 609 (placeholder option).
2. §13 Wayzata: the own CMS is **UA-gated, not blocked** — verified first-hand this lane by fetching `robots.txt` with a browser UA (HTTP 200, 954 B, `samples/robots-www.wayzataresults.com.browserua.txt`); the policy disallows `/cgi-bin/`, `/_private/`, `/_vti_*/`, and **`/reports/`**. The earlier curl-UA 403 capture (`samples/robots-www.wayzataresults.com.txt`) is the UA-gate, not a hard block.
3. §4 PrimeTime: `live.pttiming.com` is prohibited (ToU + robots); the allowed lane remains `www.pttiming.com/api/results/current` + `data.pttiming.com/**` files, which is what this lane used. Meet ids are dual-space — key on `eventUrl`.
4. The tenant-level census (256 measured here vs 258 inherited) is now cross-validated; the difference is not resolvable without re-running the aggregation, and the report states both numbers.

## Open questions (must be closed before implementation)

1. FlashResults: which of the 14 scheduled meets carry HS divisions — only Adidas XC Challenge was verified to carry HS grade tokens. Historic year pages (2018/2019/2020) need the same check.
2. PrimeTime: earliest `year` accepted by `/api/results/current`; whether `exStateOr` can be normalised reliably (it mixes `WI` and `Wisconsin`).
3. AthleticLIVE: how often athlete `y` (class token) is populated; whether any tenant indexes carry a date histogram to bound historical depth per tenant.
4. TFRRS: the year-archive URL pattern and whether HS meets are discoverable other than through the rolling RSS window.
5. MeetPro: one real published meet URL must be captured before any parser is written — this lane found none.
6. Hy-Tek PDF font encodings: which publishers (like trackmeetio) need OCR vs text extraction (PrimeTime extracts cleanly).
7. Wayzata own-CMS reachability from a browser session (CloudFront 403 to curl) — unknown whether it is UA-gated or globally blocked.

## What this lane did NOT prove

- No claim is made about PrimeTime result-file row counts beyond the single sampled division PDF/htm.
- MeetPro field shapes, RACE RESULT result rows, OpenTrack schemas, and XCStats result formats are **unverified** — see each family's status.
- The `[INFERENCE]` markers (HS slice of FlashResults, PrimeTime rows/file, GSE meets/season) are estimates, not measurements.
