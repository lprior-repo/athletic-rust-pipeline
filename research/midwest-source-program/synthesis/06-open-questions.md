# Open questions — the exact captures/fetches that close them

> **Status 2026-09-20 (gap phase).** Live re-verification moved several rows. Full evidence and response
> inventories: `synthesis/08-gap-closure-2026-09-20.md` + `research/midwest/evidence/gap-closure/`.
> - **CLOSED live:** A1 (meet response shapes: whole-meet + per-event + relay legs + implement spec +
>   Wind/Heat), A4 (`DownloadRankings` = role-gated 403 anonymous, not needed), A5 (team object; team-home
>   route is session-bound), A8 partial (relay rows + `relayLegs[]` legs with grade), **B1–B5** (AthleticLIVE:
>   tenant universe 256 indices w/ counts in 1 request; `i`↔`ani` meet mapping; blob ETag/304; container
>   listing disabled; data path = ES+blob+Firebase), **A10 partial** (eventTypes descriptions give implement specs).
> - **PARTIAL:** C5 (paywall markers measured: 44 locked rows, `paywall_present`, `Class of 2027` free),
>   C6 (sitemap = 7,500 rolling `/athletes/` URLs, ~2 h lastmod window; 7-day recurrence check pending).
> - **STILL OPEN:** A2 (meet-calendar endpoint — needs one session capture), A3/A6/A7/A9 (XC/indoor/state
>   division maps — `GetNavInfo` is session-bound; plain-client `GetRankings` works), C2/C3 (join audit +
>   full roster sweep — audit in flight), D-state follow-ups (agents 31–46 in flight).
> - Network note: **plain-HTTPS anonymous access works for the meet/bio/rankings/public endpoints with a
>   browser-grade UA**; `GetNavInfo` requires the SPA-bootstrapped `anettokens`/`anet-site-roles-token`/
>   `anet-appinfo` headers. Default-UA curl still gets the Cloudflare interstitial — that was the source of
>   the earlier "headed session required" conclusion.

Register of the highest-value unresolved items across reports 01–29, ordered by how much a single
session closes. Each item names the run that resolves it. Athletic.net items generally require the
already-cleared headed Chromium profile (live non-browser access is Cloudflare-403).

## A. Athletic.net (blocked live; captured-session required)

| # | Question | Cost to close | Ref |
|---|---|---|---|
| A1 | **Meet response shapes** (`GetMeetData`, `GetEventDivisionData`, `GetAllResultsData`): row fields, wind, implement/hurdle spec, heat, relay-leg membership | 1 headed browser session, 4 named captures | [03] |
| A2 | **Meet-calendar/index endpoint** — no `/api/v1/Meet/*` request exists in any capture; is there a weekly new-meet discovery route without rankings sweeps? Capture `/events` + a division home | 1 session, ~4 page loads | [03] |
| A3 | **XC rankings mechanics** — list ids, distances, per-state counts, grade filter behaviour on `/CrossCountry/rankings/list/...` | 1 session, 2 states × 2 distances | [04] |
| A4 | `DownloadRankings` (role-gated bulk blob), `GetAthletes` semantics, anonymous vs signed-in entitlement on meet calls | 1 session + role check | [05][03] |
| A5 | Team-home + public team-roster request shapes (bundles `team-home.routes*`, `division-home.routes*` never fetched) | 1 session, 2 requests | [01] |
| A6 | Per-state division-tree node counts for the other 11 states (only WI measured: 68 nodes ⇒ 70 requests) | 2 requests/state in session | [01] |
| A7 | XC/indoor `divListId`s per state (only TF-outdoor 2026 captured; indoor = year+10000, list ids rotate each season) | 1 `GetNavInfo`/state/season | [01][05] |
| A8 | Relay query cost with grade filter (relay pages fetched unfiltered today) | 1 session | [04] |
| A9 | Girls + indoor divisions on the rankings path (all retained artifacts are boys outdoor) | 1 session per division | [04] |
| A10 | 7 unexplained numeric event tokens (~11.3k rows in retained delivery) — family mapping | offline only (bundle read) | [04] |

## B. AthleticLIVE (Athletic.net white-label; no Cloudflare)

| # | Question | Cost | Ref |
|---|---|---|---|
| B1 | `api.athletic.live` data API shape (host reachable, `Cannot GET /api/meets`) | 1 browser capture of a meet page + 1 network trace | [03] |
| B2 | Blob container listing — event ids must come from a rendered meet page; is there an index? | 1 probe set | [10] |
| B3 | Weekly-change detection on the blob store (ETag/Last-Modified available?) | 1 meet re-fetch | [10] |
| B4 | `live.athletic.net/meets/<id>` ↔ `athletic.net` core MeetID mapping (301s to tenant; `/meet/75990` 404s) — does the SPA resolve a canonical id? | 1 capture | [11] |
| B5 | Tenant-universe enumeration (258 machines listed in SPA HTML; per-tenant meet counts) | scripted, ~258 requests | [12] |

## C. MileSplit

| # | Question | Cost | Ref |
|---|---|---|---|
| C1 | `/api/v1/meets/{id}/performances` is robots-disallowed — decide policy: license, or accept HTML-only (`/meets/<id>/results/<RSID>/raw`) | decision + possibly licensing | [27][07] |
| C2 | Athletic.net join quality MileSplit→AN (structural today, not measured) | join script over rosters once A-items known | [07] |
| C3 | Per-state Class-of-2027 census (3-team samples today) | full roster sweep ≈6,633 requests across 12 states | [27] |
| C4 | `/entries` and `/timing` coverage beyond the 4 tested states; meet-has-data vs meet-has-link | ~1 req/meet sample | [27] |
| C5 | Progression-tab payload (`paywall_present:1`) | 1 browser session | [07] |
| C6 | Sitemap 7,500-cap semantics (rolling recency vs hard cap) | 2 pulls over a week | [16][27] |

## D. State association / timer follow-ups (small)

- **ND (25)**: report landed — see it for the ND open list (state page → qualifier spreadsheet etc.).
- **MN**: MSHSL 2025 XC PDFs are raster-only (OCR required or skip); MSHSL participant nodes for prior years unverified [09][10].
- **IA**: Bound `/directory` (robots-disallowed) is not needed — but the AthleticLIVE↔athletic.net meet-ID join is [11]; 24 IHSAA members lack a Bound T&F team [11].
- **IL**: girls XC tournament ids 691–693 inferred, only 688–690 fetched [13]; DA IL has no live HS data (2024 Top Times only) [14].
- **MI**: UP girls finals D2/D3 naming inferred [15]; MITS indoor archive not located [16].
- **OH**: OATCCC indoor (no directory); per-school AN TeamID resolution still needs a headed session [19][20].
- **MO**: MSHSAA→AN MeetID mapping unverified; PrimeTime ToU scope ruling [21][22].
- **KS**: grade-bearing rosters champion-team-only; TF `Grade` null in kshsaachamps [23].
- **NE**: NSAA coach emails absent; regular-season results absent [24].
- **SD**: no athlete-level data anywhere; co-op naming for 53 AN records [26].
- **WI**: coach refresh has no change feed (poll per school) [06]; 2026 RUNMEET-style track PDFs not text-extractable [08].

## E. Program-level

- **P1**: Decision on lowest-cost compliant athletic.net profile path: keep `GetAthleteBioData` for
  progression only, move results to meets (needs A1) [03].
- **P2**: Retain `MeetID` in the rankings parser output (zero extra requests) so meet seeding is free [03][05].
- **P3**: Coach-graph consolidation (report 29) → pick the per-state authoritative source; several states
  have no public coach emails (MO, IA(emails), SD, KS, IN, NE) — those states need district-site work [29].
- **P4**: Weekly incremental design: rankings diff (page-until-empty, dedupe on `IDResult`) + timer index
  deltas + MileSplit sitemap tail; no `ETag`/`Last-Modified` on Athletic.net (all `no-store`) [03][04][07].
