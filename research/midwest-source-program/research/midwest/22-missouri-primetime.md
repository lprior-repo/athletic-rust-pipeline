# 22. Missouri PrimeTime Timing

Status: complete
Observed on: 2026-09-19

### Source

PrimeTime Timing — operating entity **PrimeTime Event & Race Management** (Brookfield, WI; `info@pttiming.com`, phone (229) 796-1234, observed at `https://www.pttiming.com/company/contact`). Missouri relevance: it is the timer for **MSHSAA state track championships (2025, 2026)**, **MSHSAA Cross Country Championships (2024, 2025)**, the **Gans Creek Classic** (Missouri's largest HS XC meet), the **Mizzou HS indoor series** (Show-Me Showdown, Bob Teel, Dr. Rick McGuire, Missouri Invitational, Mizzou HS Elite/Distance Elite, Alexis Jarrett), and a mid-Missouri spring invite/conference circuit (Osage, Blair Oaks, California, New Bloomfield, Crystal Branch, Tri County Conference, Hallsville, CS8 JH).

Host map (all observed 2026-09-19, curl, no browser headers):

| URL | Behaviour |
|---|---|
| `https://primetimetiming.com/` | **301 → `https://pttiming.com`** (server header leaked `ip-10-123-124-38.ec2.internal`) |
| `http://primetimetiming.com/`, `https://www.primetimetiming.com/` | same 301 → `https://pttiming.com` |
| `https://pttiming.com/` | **307 → `https://www.pttiming.com/`** (Vercel) |
| `https://www.pttiming.com/` | 200, Next.js (Turbopack) app; 184,526 bytes |
| `https://www.pttiming.com/results` | 200 — meet list (server-rendered + JSON API) |
| `https://www.pttiming.com/event/<id>` | 200 — per-meet page with result-file links |
| `https://data.pttiming.com/...` | static result files (Supabase public object bucket, fronted by Cloudflare) |
| `https://live.pttiming.com/` | **legacy/real-time results platform operated by Karmarush LLC** — see Access characteristics; ToU prohibits automated use |

The 301 target named in the brief is resolved: the real host is `www.pttiming.com` (Next.js), with static results on `data.pttiming.com` and the live feed on `live.pttiming.com`.

### Coverage

Multi-state timer with Wisconsin as its home market. Event counts come from `GET https://www.pttiming.com/api/results/current?year=<Y>&page=N&limit=200` (`hasMore` followed to exhaustion):

| Year | Events | WI | MO | IL | Other notable |
|---|---|---|---|---|---|
| 2026 (through 2026-09-19) | 352 | 255 (245 `WI` + 10 `Wisconsin`; state field is dirty) | **24** | 13 | FL 11, IA 8, IN 8, NE 6, KS 2, MN 2, TX 5, PA 4 |
| 2025 | 452 | 319 | **28** | 17 | FL 14, TX 12, IA 9, PA 9, IN 8 |
| 2024 | 451 | 329 | **18** | 21 | FL 11, TX 9, IA 8, PA 8 |
| 2012 / 2009 | ≥200 each (page 1 = 200, `hasMore=true`) | — | — | — | year filter list on `/results` offers **2009–2026** (18 seasons) |

Sports (meet-level `eventTypes` exposed on `/results`): Cross Country, Indoor Track & Field, Outdoor Track & Field, Track & Field, Road Race, Triathlon, Golf, Video Board Event. School levels: HS **and** middle/Junior-High meets are common; college/international meets also appear (NCAA DI XC Championships 2025, NAIA XC, Pre-Nationals, Gans Creek College, Mizzou college meets, USATF championships, Athlos London) — college and road/trail rows must be filtered out for HS recruiting.

**Missouri 2026 (24 events; HS/JH = 20, college = 1, trail = 3):** MSHSAA Class 4 & 5 Champs (2026-05-29), MSHSAA Class 1-3 Champs (2026-05-22), MSHSAA Class 4 District 4 (05-16), MSHSAA Class 2 District 4 (05-09), Tri County Conf MS (05-04), CS8 Conf JH (05-01), Guy Rush Memorial (04-30), Tri County Conference (04-28), California MS Invite + New Bloomfield JH Relays (04-21), Osage HS Warpath (04-13), Crystal Branch Invite (04-03), California HS Open (03-31), New Bloomfield HS Relays + Blair Oaks MS + Blair Oaks HS (03-26/27), Mizzou HS Elite (02-06), Mizzou Alexis Jarrett (01-10), CPS Cosmo-Bethel (09-16), CPS Gans Creek Opener (09-09). College: Larry Young Invitational (09-11). Trail: ROC/Stonegrinder/GRiT 7K.

**Missouri 2025 (28 events; HS/JH = 22, college = 4, trail = 2):** MSHSAA XC Champs (2025-11-07), CMAC MS Conf (10-01), SBC Eagle Run (09-30), **Gans Creek Classic HS (09-26, 28 division files)** + College, Mizzou XC Opener (08-29), MSHSAA Class 4 & 5 Champs (05-30), MSHSAA Class 1-3 Champs (05-23), Class 2 District 4 (05-10), Tri County Conf (04-29), Osage Varsity (04-22), New Bloomfield JH (04-22), Osage MS/HS Warpath (04-21/14), Hallsville Purple MS (04-09), Crystal Branch (04-05), New Bloomfield Invite (04-03), New Bloomfield HS Relays (03-28), Missouri Invitational (02-07), Mizzou HS Distance Elite (02-07), Dr. Rick McGuire (01-31), Bob Teel (01-25), Show-Me Showdown (01-17), Alexis Jarrett (01-11). College: NCAA DI XC, Pre-National, Gans Creek College, Mizzou XC Opener. Trail: ROC/Stonegrinder 7K.

Note the seasonal shape: PTT's Missouri footprint is **weighted to the Columbia/Jefferson City (mid-MO) area plus MSHSAA championships**; St. Louis metro and Kansas City metro meets are largely timed by other companies (see Incremental use / other timers below).

### Enumeration

**Meets (the only first-class enumerable entity).** Two equivalent surfaces:

1. HTML: `GET https://www.pttiming.com/results?year=<2009..2026>&type=<eventType>&name=<substring>` → server-rendered Next.js page whose RSC payload contains `initialResponse.{page,limit,hasMore,rows[]}`; default page size 100. The `?page=` query is ignored server-side (observed: `?page=2` returned identical page 1), so use the JSON endpoint for paging.
2. JSON (used by the page's own client): `GET https://www.pttiming.com/api/results/current?year=<Y>&type=<t>&name=<s>&filter=<All|Featured>&page=<N>&limit=<N>` with `Accept: application/json` → `{page, limit, hasMore, rows[]}`. `limit` is silently clamped to **200** (requested 500 → returned 200). Rows are sorted `startDate` descending, stable across pages (page 2 begins exactly where page 1 ended). 2026 needed 2 pages, 2025 3 pages, 2024 3 pages at limit=200.

Upcoming meets: `GET https://www.pttiming.com/schedule` + `GET https://www.pttiming.com/api/schedule?page=1&limit=200` (observed 77 rows, 2026-09-18→2026-10-31, `hasMore=false`; forward-looking only — `year=2025` on this endpoint is ignored and still returns the same 77 future rows).

Per-meet page: `GET https://www.pttiming.com/event/<id>?pt=results` where `<id>` is a numeric CRM id (`/event/2874`) **or** a UUID (`/event/3b754ccf-4c63-426e-8d2a-e0f10b5df296`). The row's `eventUrl` field gives the canonical form. The page lists: Live Results link (when the meet ran live), and result files as `{label, url}`.

Row schema (exact keys, all rows): `id, clientId, name, status, startDate, deadline, exTimezone, exEventStart, exStatusNote, exFranklinMid, exFranklinPlat, exTimer, exEntryLink, exEntryOpen, exEntryClose, exGps, exVenueOr, exHostOr, exCityOr, exStateOr, clientCompany, clientCity, clientState, eventUrl, computed{dateRangeText, locationTextLines, meetStatusText, meetStatusVariant, showLiveCount, entryBadge, entryLinkHtml, timerIsPtt}, results{liveResultsHtml, fileLinks[{label,url}]}`. `exEntryLink` is raw HTML (`<a href="…">Click Here to Register</a>`) and frequently points at Athletic.net (see Athletic.net leverage).

Observed `status` codes: **2 = Meet in progress, 4 = Meet is Complete, 5 = Meet is Cancelled** (derived from `computed.meetStatusText` over all 1,655 collected 2025–2026 rows: 2→"Meet in progress" ×1, 4→"Meet is Complete" ×775, 5→"Meet is Cancelled" ×28).

**Schools/teams:** not enumerable — no school, team, or district index exists; school identity appears only as the Hy-Tek school label inside result files.

**Athletes:** not enumerable — no athlete index or profile pages (route scan of all 11 page chunks found only `/event/`, `/results`, `/schedule`, `/api/schedule`, `/api/results/current`).

**Class of 2027:** enumerable *inside* result files only, via the Hy-Tek `Year` column = 11 (see Athlete evidence).

**Results:** enumerable per meet; each file is static (no pagination).

### Stable identifiers

| Entity | Identifier | Quality |
|---|---|---|
| Meet | PrimeTime event id — numeric CRM id (`2026: 2550–2911`, `2025: 938–2548`, `2024: 1802–2075`) **or** UUID for Franklin-platform events; when the canonical page is a UUID, the list row's `id` is `0` (36 of the newest 100 rows) | Stable but dual-space; must key on `eventUrl` |
| Meet (live) | `exFranklinMid` — 4-digit Franklin meet id (`9011` = 2026 MSHSAA C4&5; observed 2026 range 8307–9132, 2025 range 7596–8261) | Stable, used to build the live URL `https://live.pttiming.com/?mid=<mid>` (track) / `…/xc-ptt.html?mid=<mid>` (XC) |
| Result file | `https://data.pttiming.com/storage/v1/object/public/event-files/{crm/<eventId> \| <uuid>}/<hash>.<ext>` — track files use an MD5-style hash; XC (2026) files use an epoch-ms slug (`1789175231149-larry-young-mens-results.pdf`) | File-path is the de-facto file ID; **not derivable** — must be read from the event page/API row. Bucket listing is disabled (`GET …/event-files/` → 400 `InvalidKey`) |
| Athletic.net meet | `https://www.athletic.net/<Sport>/meet/<MeetID>/register` inside `exEntryLink` | Explicit when present (see below) |
| Event (within results) | Hy-Tek `Event NN` number, e.g. `Event 12 Boys 5k Run CC Elite` | Per-meet only, not global |
| Season | year filter (2009–2026) on `/results`; no season ID field. The restricted platform additionally carries `"season":"2025-26"` in its `Meta` node | No public season ID |
| Athlete / School / Result / Team | **none** | No athlete IDs, no school IDs, no result IDs anywhere on the public surface |

Grades, school labels and marks are free text/structured columns inside files, not identifiers; Hy-Tek school labels ("Rock Bridge", "Warrensburg", "Parkway Central") are Athletic.net-shaped names but need alias resolution.

### Athletic.net leverage

**Direct Athletic.net links — yes, but only as entry/registration URLs.** `exEntryLink` (and its duplicate `computed.entryLinkHtml`) contains an Athletic.net meet registration anchor for a large share of recent meets:

- 2026 season: **209 of 352 events (59 %)** carry an Athletic.net link; 2025: 93 of 452 (20 %); 2024: 2 of 451 (0.4 %) → the Athletic.net registration integration is recent.
- Unique Athletic.net MeetIDs in the 2025+2026 lists: **302** — WI 271 (91 CrossCountry + 166 TrackAndField + 14 under the dirty state label `Wisconsin`), IL 23, **MO 6 (TrackAndField)**, 2 rows with an empty state field. Sport path split: `/TrackAndField/meet/<id>/register` 198, `/CrossCountry/meet/<id>/register` 104.
- Missouri MeetIDs observed (all Mizzou indoor meets, `TrackAndField`): 619619 (2026 Alexis Jarrett), 575635 (Missouri Invitational), 575627 (Dr. Rick McGuire), 575625 (Bob Teel), 575610 (Show-Me Showdown), 570413 (2025 Alexis Jarrett). Example row: `https://www.athletic.net/TrackAndField/meet/619619/register`.
- No Athletic.net **team**, **athlete** or **result** links exist anywhere on the site (no such routes; anchor scan of the event page and list pages shows only `data.pttiming.com`, `live.pttiming.com` and social links).

**Deterministic seeding for the rest.** For the ~80 % of meets (and essentially all Missouri XC/outdoor meets, including MSHSAA championships and districts) there is **no** Athletic.net link. The available join keys are (meet name, start date, host school `exHostOr`, city/state) plus the Hy-Tek school labels inside the files:

- These keys are sufficient *in principle* to seed a single Athletic.net meet lookup or team lookup (`name + date + state`), and the grade column makes Class-of-2027 confirmation independent of Athletic.net.
- Not verifiable today: `https://www.athletic.net/` is Cloudflare-403 from this machine (mission-brief finding), and neither HAR capture contains any Missouri meet page (grep of both HARs: `MSHSAA` 0 hits, `Gans Creek` 0 hits; only generic `Missouri` division entries — see below). So every non-linked join is **[INFERENCE]**, not evidence.
- Captured Athletic.net-side context available for MO seeding (HAR #1, `/api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m&…`): Missouri appears as a state division `{"id":169578,"name":"Missouri","state":"MO","divType":"State","subDivType":"Associations","depth":1}`. That division id is a captured artifact, not a live observation.

**Estimated Athletic.net requests avoided** (all [INFERENCE], contingent on the access question below):

- 302 unique MeetIDs already resolve meet discovery for the linked 2025–2026 meets with zero Athletic.net meet-search requests (MO share: 6).
- Result files substitute per-athlete Athletic.net profile/history fetches with 1–28 file GETs per meet. Measured example: MSHSAA 2026 state track = **10 HTML files covering 3,393 athlete-event rows (3,312 placements + 81 `DNF`/`DNS` rows)** — Class 4/5 files hold 19 events and 288–300 placements each, Class 1–3 files 27 events and 352–362 placements each ≈ ~2,600–2,900 unique athletes [INFERENCE] → on the order of **0.004 file-requests per athlete** versus ≈1 Athletic.net profile request per athlete.
- The full Missouri HS corpus with published files is ~14 meets (2025) / ~9 meets (2026 YTD), ≈84 division files in 2025 → a few hundred file GETs per season would cover every grade-tagged Missouri result PTT publishes. That is negotiation leverage, not a build plan: see Recommendation.

### Athlete evidence

Available (inside result files, per division/class):

- **Name** — `Cameron Miller` / `Miller, Cameron` depending on generator format (Hy-Tek `Meet Manager` prints "First Last"; the compiled export prints "Last, First").
- **Grade / class** — Hy-Tek `Year` column: `9/10/11/12` for HS (college meets print `FR/SO/JR/SR`). Verified across MSHSAA 2026 track HTML (`Cameron Miller 10 West Plains`, `David Odekunle 11 Parkway Central`), MSHSAA XC 2025 HTML (`Sinry Mendoza 12 Hollister`, `Kyle Hathcock 11 Springfield`), XC PDF and compiled PDF. **Grade 11 rows are Class-of-2027 rows** (2025-26 school year).
- **School** — Hy-Tek short label (`Rock Bridge`, `Poplar Bluff`, `Warrensburg`); separate boys/girls files usually encode gender at file level.
- **Gender/category** — event title (`Boys …`/`Girls …`, `Men`/`Women` section headers) and class/division label.
- **TF/XC distinction, indoor/outdoor** — meet level only (name/sport), never per athlete.
- **Performances** — marks with precision variants and normalization hints: `10.51` + `(10.510)`, `1:27.50` + `(1:27.493)`, XC `14:44.7`; field series (`1.65 1.75 1.80 … XXX`) and best mark (`1.95m  6-04.75`).
- **Meets** — one row per meet; the event page is the meet artifact.

Not available: athlete city/state, athlete profile URL, athlete IDs, PRs, cross-meet progression, career history, recruiting/profile pages. City/state exists only for the meet (`exCityOr`/`exStateOr`).

### Recruiting information

**None.** The source exposes no coach, assistant coach, AD, school athletics website or team website fields, and no directory of any kind. (Checked: all site routes, event page anchors, and result-file headers; only the timing company's own contact appears.)

### Result evidence

| Field | Availability |
|---|---|
| ResultID | **No** |
| AthleteID | **No** |
| MeetID | PrimeTime event id + `exFranklinMid`; plus Athletic.net MeetID when `exEntryLink` is an Athletic.net URL |
| EventID | Hy-Tek `Event NN` per meet only (e.g. `Event 12`); no global event taxonomy |
| Mark | Yes (Finals; plus splits `(10.510)` and field series) |
| Normalized mark inputs | Partial: raw + parenthesized thousandths (`10.82 … 10.813`), conversions only for field marks (`1.95m  6-04.75`); no wind-adjusted/altitude-normalized values |
| Timing method | Implicit only: Hy-Tek output header (`HY-TEK's Meet Manager`, `Licensed to PrimeTime Timing - Contractor License`); no FAT/hand field |
| Wind | Yes for sprints/jumps (`10.51   1.5  2`, `Wind H#` columns); XC none |
| Implement/hurdle specification | **Not observed** in compiled output (no implement weights, no hurdle heights) |
| Heat/round | Yes — `H#` column, `Finals`/`Section N` headers |
| Place | Yes, including `--` with `DNF`/`DNS`/`DQ` states |
| Date | Yes (meet header and event `startDate`) |
| School represented | Yes (Hy-Tek label) |
| Relay membership | Yes — 4 legs with individual grade + team mark, e.g. `Poplar Bluff 1:27.50` → `1) Lee Hughes 9  2) Devin Ferguson 11  3) Xzavir Jones 11  4) Kinyon Johnson 12` |

File formats in the 2025–2026 corpus: **1,349 PDF, 333 `.htm`, 54 `.html`** file links. Machine-readability is uneven and worsening:

- 2025: 244 of 452 events (53 %) publish at least one HTML file; those `.htm` files are plain Hy-Tek ASCII (perfectly parseable) but are **single-event snapshots** — e.g. `Gans Creek Classic – Elite Boys Varsity` = 304 finisher rows + team scores for one race.
- 2026: only 28 of 352 events (7 %) publish HTML; most 2026 track meets publish a single `Compiled Results (PDF)`.
- Those 2026 PDFs split three ways (verified by extracting): text-based Hy-Tek output (`MSHSAA C5 boys.pdf` → 60 KB of text, 17 pp.), text-based results-export layout (`Osage HS Warpath …pdf` → 175 KB text, 73 pp., columns `Athlete / Yr / Team / Finals`), and **image-only** scans (`Microsoft: Print To PDF`, `DCTDecode`, no `/Font` — `Blair Oaks HS Invitational 2026`, `Crystal Branch 2026` → `pdftotext` yields 18 characters). Image-only files would require OCR, which this project must not add.

### Incremental use

- **New/changed meets:** one `GET /api/results/current?year=<current>&page=1&limit=200` per week (rows are `startDate` descending and stable), diff on `(eventUrl, startDate, status, results.fileLinks[].url)`. Page 1 covers roughly 3–5 months of events, so a weekly poll costs **1 request** and never re-fetches history. `status` transitions 5/2 → 4 and the first appearance of `fileLinks` are the publication signal.
- **Upcoming/next-week:** `GET /api/schedule?page=1&limit=200` (77 upcoming rows observed) gives date, host, city/state and the Athletic.net registration link before results exist — useful as a *discovery* feed (pre-announcement) rather than a results feed.
- **New results:** the per-event file list grows; file URLs are immutable per publication (hash names for track, epoch-ms slugs for XC), so new files appear as new URLs. No `ETag`/`Last-Modified`/`updatedAt` field was observed on the API rows, so treat each row's `fileLinks` array as the change token.
- **Affected athletes:** not identifiable without re-parsing a changed file (no athlete entity/id on the site). Per-event re-parse is the only mechanism; there is no athlete-level delta feed.
- **Not available for incremental use:** the live feed (`live.pttiming.com` + its realtime DB) — see below; and no per-meet `updated` timestamp on the public page.

### Access characteristics

Classify as **normal HTML + static PDF/HTML files + undocumented public JSON** on the PrimeTime-owned surface, and **browser application / subscription-restricted (real-time results platform)** on the live surface. There is also a legal overlay on the live platform that is decisive:

- `https://www.pttiming.com/robots.txt` → **404**; `https://www.pttiming.com/terms` → **404**; no `X-Robots-Tag`/`tdm-reservation` header observed on any response from `www.pttiming.com`, `data.pttiming.com` or `pttiming.com`. No ToU was found for the PrimeTime-owned pages.
- `https://data.pttiming.com/storage/v1/object/public/event-files/` → **400 `{"statusCode":"400","error":"InvalidKey"}`** (public bucket, no listing). Individual file GETs return 200 with `cache-control: public, max-age=3600`, `server: cloudflare`.
- `https://live.pttiming.com/` (200, IIS SPA, `PWA`, reads a realtime Firebase DB) is governed by **Karmarush LLC's "Live Results Platform Terms of Use", v1.0, effective 2026-09-03** (`https://live.pttiming.com/terms.html`). Verbatim, the operative parts for this project: automated access/extraction "by Automated Means" is prohibited (§4.1); reading the "Underlying Systems" directly is prohibited and endpoint discovery/documentation is prohibited (§4.2); using the data to "train, pre-train, fine-tune, distill, align, evaluate … or ground, augment, or supply any model at inference time" is prohibited (§4.3); and — decisive for this mission — §4.5 prohibits using the data "to build, populate, or supply any **recruiting database**, service, or platform". `https://live.pttiming.com/robots.txt` (200) additionally disallows `ClaudeBot`, `Claude-Web`, `Claude-User`, `Claude-SearchBot`, `anthropic-ai`, `GPTBot`, `Scrapy` and other automated agents across the whole host, and declares an express TDM reservation; `https://live.pttiming.com/.well-known/tdmrep.json` → 404 (the reservation is in robots.txt, meta and headers instead). Licensing contact: `legal@karmarush.com`.
- Limits observed: **no 429 and no `Retry-After` on any host** during ≈35 sequential requests to `www.pttiming.com`, ≈18 to `data.pttiming.com`, 4 to `live.pttiming.com`, 6 to the legacy realtime DB, all 2026-09-19 23:12–23:20 CDT. No published rate limit found on the PrimeTime-owned surface.
- Disclosure: six GETs were made to the legacy realtime database host before its ToU was discovered (its `Meta` node is what surfaced the ToU text). They returned only key/metadata structure (top-level meet-id keys, per-meet node names, division labels) and a meet title — no athlete names, marks or rosters were retrieved, and no further requests were made once the ToU was read. That host must not be used again without a licence.

### Recommendation

**REJECT** for automated ingestion — with one narrow, explicitly-flagged exception path.

- The live results platform (`live.pttiming.com` and its realtime database) is **off-limits** for this pipeline: its ToU prohibits automated access, bulk extraction and — specifically — building or supplying a recruiting database, and its robots.txt names Claude/Anthropic agents. Technically open ≠ permitted. Licensing: `legal@karmarush.com`.
- The PrimeTime-owned surface (`www.pttiming.com` meet list/event pages, `data.pttiming.com` compiled files) has no published ToU or robots.txt, and its two most valuable artifacts for this mission — explicit Athletic.net MeetIDs (302 unique, 6 in Missouri) and grade-tagged compiled results — are exactly what the pipeline wants. But the files and links are the same operator's results data (PrimeTime licenses the Franklin platform from Karmarush, and the `Meta` metadata for the MSHSAA meet is served by that stack), so **the conservative reading is that a licence is required for systematic use**. PrimeTime contact: `info@pttiming.com` (Brookfield, WI).
- If Main wants a stop-gap, the honest classification is **DISCOVERY-ONLY**, manual and low-volume: a human opens `/results` for the current season, reads the Athletic.net registration links off the meet rows, and uses those MeetIDs to seed targeted Athletic.net work. That is "a coach … who reads a result and acts on it individually" and stays inside the tolerated band; it must not become a scheduled collector.
- Marginal coverage if licensed (Missouri): ~20–22 HS/JH meets per year including **both MSHSAA state track championships and the MSHSAA XC championships** plus **Gans Creek Classic** (28 division files in 2025 — the state's biggest XC meet), all with grade columns → direct Class-of-2027 evidence for roughly the same athlete pool Athletic.net would have to be scraped profile-by-profile. However, the two state championship meets are the likeliest to be duplicated on MSHSAA.org and/or Athletic.net [UNVERIFIED — Athletic.net is 403 from this machine and peer report 21 covers MSHSAA], which lowers the marginal value of the blocked files to the mid-MO regular-season circuit (Osage, Blair Oaks, California, New Bloomfield, Crystal Branch, Tri County, Hallsville, Mizzou indoor series).
- Comparison with the other Missouri timers (for Main's provider map): **TRXC Timing** (Bridgeton, MO — `trxctiming.com`, results index `/wp2/track-and-field/track-and-field-results/2026-track-and-field-results/` and `/wp2/cross-country/past-results/2026-cross-country-results/`) is the larger Missouri provider — 155 T&F result files (124 non-Illinois) and 17 XC files listed for 2026, **including MSHSAA district/sectional meets** (e.g. `MSHSAA/2026_TF/Sectional/C4S2_C5S2/`, `…/District/C4D5/`, `…/District/C2D2_C3D2/`), St. Louis metro conferences (Suburban, GAC, Metro League, CMAC) and KC-adjacent schools (Kearney/Kennett, North Callaway, Boonville). Its files are the same Hy-Tek HTML format with `Year` grades (verified: `Parkway Central Henle Holmes Invite 2026`, 586 finisher rows, columns `Name / Year / School / Finals / Points`) and carry no Athletic.net links. Also present but **not** HS-relevant: KC Running Company (road races only), `midwesttiming.com` (≈3 KB landing page, no meet index), AthleticLIVE (`athletic.live` → `live.athletic.net`, covered by assignment 3), MileSplit Missouri (`mo.milesplit.com`, 200; assignment 27).

### Evidence appendix

All requests issued with plain `curl` (default UA, no cookies, no auth, sequential), 2026-09-19 23:12–23:20 CDT unless noted. `www.pttiming.com` ≈35 requests, `data.pttiming.com` ≈18, `live.pttiming.com` 4, legacy realtime DB 6, other hosts ≤2 each.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://primetimetiming.com/` | GET | 301 → `https://pttiming.com` | resolves the brief's 301 question; target host is `pttiming.com` | 2026-09-19 23:12 CDT |
| `http://primetimetiming.com/`, `https://www.primetimetiming.com/` | GET | 301 → `https://pttiming.com` | same redirect on http and www | 23:12 |
| `https://pttiming.com/` | GET | 307 → `https://www.pttiming.com/` | final real host (Vercel) | 23:12 |
| `https://www.pttiming.com/` | GET | 200 (184,526 B) | Next.js site; CSP allowlists `*.firebaseio.com`; legacy asset paths | 23:12 |
| `https://www.pttiming.com/results` | GET | 200 (309,612 B) | meet list + embedded `initialResponse` JSON (100 rows: id, name, date, state, Franklin mid, Athletic.net entry link, file links) | 23:12 |
| `https://www.pttiming.com/results?year=2025` | GET | 200 (355,522 B) | server-side year filter works; 100 rows of 2025 | 23:14 |
| `https://www.pttiming.com/results?page=2` | GET | 200 (identical page 1) | `page` param ignored server-side → must use JSON endpoint to page | 23:14 |
| `https://www.pttiming.com/api/results/current?page=1&limit=200` | GET (Accept: application/json) | 200 JSON | 200 rows, `hasMore=true`; row schema captured | 23:14 |
| `…/api/results/current?page=2&limit=100` | GET | 200 | page 2 continues exactly at previous last row → stable sort | 23:15 |
| `…/api/results/current?year={2026,2025,2024,2012,2009}&page=1..3&limit=200` | GET ×9 | 200 | 2026=352, 2025=452, 2024=451 events; 2012/2009 page 1 = 200 rows with `hasMore=true` → ≥200 events/yr back to 2009 | 23:15–23:17 |
| `…/api/results/current?year=2025&type=Cross Country&page=1&limit=50` | GET | 200 | `type` filter works for XC (50/50 XC events); `type=Track & Field` returned a mixed set including an XC meet → do not trust `type` for sport classification | 23:19 |
| `https://www.pttiming.com/api/schedule?page=1&limit=5` / `…&limit=500` / `…&year=2025&limit=200` | GET ×4 | 200 | upcoming feed = 77 rows (2026-09-18→10-31), `limit` clamped to 200, `year` ignored (forward-looking) | 23:14–23:16 |
| `https://www.pttiming.com/event/2874?pt=results` | GET | 200 | MSHSAA C4&5 event page: 8 result files (HTM+PDF) + live link; also links `/event/2415` (2025) and `/event/1939` (2024) | 23:15 |
| `https://www.pttiming.com/_next/static/chunks/*.js` (11 chunks) | GET ×11 | 200 | `/api/schedule` + `/api/results/current` contracts, `page/limit/filter/type/name/year` params, Firebase DB `ptt-franklin`; no athlete/team routes | 23:13–23:16 |
| `https://www.pttiming.com/robots.txt`, `/terms` | GET | 404, 404 | no robots and no ToU on the PrimeTime-owned site | 23:17 |
| `https://www.pttiming.com/company/contact` | GET | 200 | operator identity: PrimeTime Event & Race Management, Brookfield WI, info@pttiming.com | 23:18 |
| `https://data.pttiming.com/storage/v1/object/public/event-files/` | GET | 400 `InvalidKey` | public bucket but listing disabled → files only obtainable via event page paths | 23:19 |
| `https://data.pttiming.com/robots.txt` | GET | 404 | no robots on the file host | 23:19 |
| `…/crm/2874/f2203803….htm` (MSHSAA 2026 C4 boys) | GET | 200 | Hy-Tek track HTML: `Name / Year(grade) / School / Finals / Wind / H# / Points`, splits, relay legs with grades, 19 events, 293 placements (+9 `DNF`/`DNS`) | 23:15 |
| `…/crm/2873/{6 files}.htm` + `…/crm/2874/{4 files}.htm` (MSHSAA 2026 all classes) | GET ×10 | 200 | 3,393 athlete-event rows total (3,312 placements + 81 `DNF`/`DNS`); C4/C5 = 19 events with 288–300 placements each, C1–C3 = 27 events with 352–362 each | 23:19 |
| `…/crm/2539/de6117ae….html` (MSHSAA XC 2025 C3 boys) | GET | 200 | XC Hy-Tek HTML: `Name / Year / School / Finals / Points`; 181 finisher rows in that class file | 23:18 |
| `…/crm/2526/9af45beb….htm` (Gans Creek Classic HS 2025, Elite Boys) | GET | 200 | single-race snapshot (304 finishers + DNFs + team scores) — HTML files are per-race, not per-division | 23:17 |
| `…/3b754ccf…/1789175231149-larry-young-mens-results.pdf` | GET | 200 (299 KB) | 2026 XC PDF is text-extractable (`pdftotext`); college `Year` values JR/SR/SO/FR | 23:17 |
| `…/crm/2874/3d1f0108….pdf` (MSHSAA 2026 C5 boys) | GET | 200 (361 KB) | 2026 Hy-Tek PDF is text-extractable: 288 numbered placements, wind/heat columns present | 23:17 |
| `…/crm/2550/7097418528….pdf` (Osage HS Warpath 2026) | GET | 200 (2.5 MB) | 2026 `Compiled Results` layout `Athlete / Yr / Team / Finals`; 175 KB extractable text, 73 pp. | 23:18 |
| `…/crm/2747/2ef383….pdf` (Blair Oaks HS 2026), `…/crm/2836/f7e280….pdf` (Crystal Branch 2026) | GET ×2 | 200 | **image-only** PDFs (`Microsoft: Print To PDF`, `/DCTDecode`, no `/Font`) — no text layer | 23:18–23:19 |
| `https://live.pttiming.com/?mid=9011` | GET | 200 (3,427 B) | legacy SPA shell; references `ptt-franklin.firebaseio.com` + `/lib/browser-bundle.js` | 23:13 |
| `https://live.pttiming.com/terms.html` | GET | 200 | Karmarush LLC ToU v1.0 (2026-09-03): §4.1/4.2 automated access & endpoint discovery banned; §4.3 AI/ML + TDM banned; §4.5 **recruiting databases banned**; licensing via legal@karmarush.com | 23:16 |
| `https://live.pttiming.com/robots.txt` | GET | 200 | blocks ClaudeBot/Claude-Web/Claude-User/Claude-SearchBot/anthropic-ai/GPTBot/Scrapy + SEO bots; TDM reservation; `Allow: /` for everyone else | 23:19 |
| `https://live.pttiming.com/.well-known/tdmrep.json` | GET | 404 | no TDM protocol file (reservation lives in robots/headers instead) | 23:19 |
| `https://ptt-franklin.firebaseio.com/.json?shallow=true` | GET | 200 | top-level meet-id keys only (no athlete data) — the live DB answers unauthenticated; discovery of the ToU stopped further use | 23:14 |
| `…/9011.json?shallow=true`, `/9011/Schedule.json`, `/9011/MeetEvents.json`, `/9011/Scores.json` | GET ×4 | 200 | node structure (`Scoreboard, Records, Meta, Scores, MeetEvents, Schedule`; event-round keys) — structure only, no results extracted | 23:14 |
| `…/9011/Meta.json` | GET | 200 | meet metadata (`sport: Outdoor Track`, `season: 2025-26`, timer `PTT`, `mms: Hytek`) **and the ToU notice** that made this host off-limits | 23:15 |
| `https://www.trxctiming.com/`, `https://trxctiming.com/` | GET | 301 → `trxctiming.com/wp2/` | Missouri competitor #2 identity (TRXC Timing LLC, Bridgeton MO) | 23:19 |
| `…/wp2/track-and-field/track-and-field-results/2026-track-and-field-results/` | GET | 200 (178 KB) | 155 TRXC 2026 T&F result files (124 non-IL), incl. MSHSAA district/sectional meets; no Athletic.net links in file | 23:19 |
| `…/wp2/cross-country/past-results/2026-cross-country-results/` | GET | 200 (65 KB) | 17 TRXC 2026 XC result files (15 non-IL) | 23:19 |
| `https://trxctiming.com/Parkway_Central/TF/Henle_Holmes/Results/2026.htm` | GET | 200 (110 KB) | TRXC files are the same Hy-Tek HTML with `Year` grade (586 rows) | 23:19 |
| `https://mo.milesplit.com/` | GET | 200 | MileSplit MO reachable (aggregator; assignment 27 scope) | 23:19 |
| `https://www.mshsaa.org/` | GET | 200 (76 KB) | MSHSAA home page carries no timing-provider links (assignment 21 scope) | 23:19 |
| `https://www.kcrunningcompany.com/` | GET | 200 (624 KB) | road-race company; no HS track/XC meet index | 23:19 |
| `https://midwesttiming.com/` | GET | 302 → 200 (3 KB) | placeholder landing page; no meet index | 23:19 |
| `https://athletic.live/` | GET | 302 → `https://live.athletic.net/` | AthleticLIVE redirect (assignment 3 scope) | 23:19 |
| `https://ultramaxtiming.com/`, `https://www.ultramaxtiming.net/` | GET | DNS failure (`Could not resolve host`) | these domains do not exist | 23:19 |
| `/home/lewis/Downloads/www.athletic.net{,2}.har` (local grep) | grep | n/a | `MSHSAA`: 0 hits, `Gans Creek`: 0, `PrimeTime`: 0; only generic `Missouri` division entries → no Athletic.net-side MO meet evidence in the captures | 23:19 |
| `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/**` (local grep) | grep | n/a | no `MSHSAA` / `PrimeTime` references in the pipeline repo (only `"MO" => "Missouri"` state maps) | 23:18 |
| `web_search` tool, query "Missouri high school track and field meet results timing company 2026 …" and "TRXC Timing Missouri …" | tool call ×2 | provider error | verbatim: **"Sign up and repeat your request."** — web search unavailable; no further web_search calls made, timer discovery done by direct HTTP instead | 23:18 |
