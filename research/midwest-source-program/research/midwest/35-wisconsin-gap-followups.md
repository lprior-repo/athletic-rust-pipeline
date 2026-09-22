# 35. Wisconsin WIAA — gap follow-ups (coach change-feed, 2026 track-PDF extraction, timing-provider link inventory)

Status: complete — all three items of the assignment answered with live measurements. One probe gap is
stated explicitly in §2 (the girls-archive `/Portals/0/…stratfordregionalb.pdf` link is broken; its
boys-archive counterpart resolves and was substituted into the sample).
Observed on: 2026-09-20 (all probes 09:05–09:13 America/Chicago)

**Tooling note.** Local toolchain measured, not assumed: `pdftotext` 26.08.0 (poppler), `pdftoppm`,
`pdfinfo`, `pdffonts`, `pdfimages` present; **`tesseract` 5.5.3 with `eng`+`osd` traineddata present**;
`ocrmypdf` **not installed**; `qpdf` present (used for offline PDF object-stream decompression).
**HARD CONSTRAINT honored: zero HTTP requests were sent to any `*.athletic.net` host.** Every
Athletic.net URL reported below was read out of an already-downloaded PDF (`/URI` annotation or text
layer) — extraction only, never fetched.
**Politeness:** 73 requests to `www.wiaawi.org` (5 of them 404s), 9 to `schools.wiaawi.org`, 1 redirect
chain (2 hops) to `wisconsinrunner.com` and 1 DNS-only attempt to `wistca.ihigh.com`. Sequential,
≥1.2 s apart per host.
**No 429 and no `Retry-After` header was observed in this session either.**

---

### Source

| Surface | URL | What changed vs report [06] |
|---|---|---|
| Association site | `https://www.wiaawi.org/` | Drupal 10 + Cloudflare (unchanged). **New:** `sitemap.xml` carries a per-URL `<lastmod>` (11,332 URL+lastmod pairs over 6 pages) — the only working content-change feed found anywhere in the WI stack. |
| Sport section pages | `/sports/{boys,girls}-{track-field,cross-country}/{…}-{tournament,state-archive}` | Pages re-fetched 2026-09-20; the 2026 T&F file set is now **134 unique links (131 PDF + 3 HTML)** across the two archives. |
| Sport sub-pages | `…/{…}-schedules`, `…-scores`, `…-standings`, `…-tournament/<sport>-assignments` | Newly inventoried for provider links (item 3). |
| School / coach DB | `https://schools.wiaawi.org/` | IIS + ASP.NET MVC 5.2 behind Cloudflare (unchanged); **new:** no `ETag`, no `Last-Modified`, `cache-control: no-cache, no-store` on every endpoint tested. |
| Coaches portal | `https://www.wiaawi.org/my-wiaa/coaches` | Newly found. Its two track/XC association links are **dead** (see §7). |
| Instructions PDFs | `/sites/default/files/pdfs/PDF/Sports/Track/2026/athetic.net-track-instructions.pdf`, `/sites/default/files/pdfs/PDF/Sports/Cross_Country/2026/instructions-for-entries.pdf` | The **only** WIAA-published artifacts carrying direct `www.athletic.net` URLs (item 3). |

### Coverage

- **State:** Wisconsin only. **Sports:** boys/girls T&F (outdoor tournament series) and boys/girls XC.
- **2026 T&F tournament file set (measured 2026-09-20):** boys archive = 103 unique 2026 links, girls
  archive = 97, **union = 134 (131 `.pdf`, 3 `.htm`)**; 8 of them are state-meet files, the rest
  regional/sectional, plus 2026-07/09 championship-summary PDFs (`trackbteamchamps.pdf`,
  `trackgrecords_1.pdf`, …).
- **XC:** the tournament pages still list the 2025 sectional/state files (2026 XC sectionals are
  2026-10-23/24 per the XC instructions PDF, i.e. not yet posted); state archive pages give 1952-2025.
- **Coaches:** 516 member schools / 1,796 T&F+XC team-seasons (report [06]); the coach rows live only on
  the per-school page. **No indoor season, no regular-season meets** — unchanged from [06].

### Enumeration

| Need | Exact route (all verified 2026-09-20) | Cost |
|---|---|---|
| 2026 T&F result files | GET the two archive pages, extract absolute file hrefs, normalise `/Portals/0/…` → `/sites/default/files/…`, strip `?ver=` | 2 requests (1.03 MB) → 134 unique links |
| Page change detection ("updated-since") | GET `sitemap.xml` + `sitemap.xml?page=1..6`; parse `<url><loc><lastmod>` | 6 requests (7 with the index), 1.93 MB, 11,332 pairs |
| File-level change detection | `HEAD` each known file: returns `Last-Modified` + `ETag` + `Content-Length`, 0 bytes of body | 1 HEAD/file (measured 0.087 s) |
| Coach rows | GET `/Directory/School/GetDirectorySchool?orgID=<OrganizationID>` (server-rendered `tblCoachList`) | 1 request **per school**, 181–219 KB; no bulk mode exists (§7) |
| Which file is which meet | `pdfinfo <file>` → `/Title` = `RUNMEET: <meet name>` or the printed header (`D3 Regional 1D - Stratford`); no IDs | offline |

### Stable identifiers

Unchanged from [06] (`OrganizationID`, `TeamID`, `SportSeasonID`, `ContactID`; a result *file* is
identified only by its URL). Two additions measured here:

- **PDF `/Title` = the RunMeet meet name.** 13 of the 26 sampled 2026 files carry
  `RUNMEET[:] <Division> <Regional|Sectional> <n> - <Site>` (e.g. `RUNMEET: D1 Regional 2B - Madison
  Memorial`, `RUNMEET_ D3 Sectional 1 - Marathon`). Report [08] showed this string matches the
  Athletic.net `MeetName` in the retained HAR payload (Appleton case) ⇒ a deterministic WIAA-file → AN
  `MeetID` join by *name* for those meets.
- **A printed Athletic.net manage-meet URL** appears in the page header of `tr2026madisonregional.pdf`
  (9×): `https://www.athletic.net/edit/track/managemeet/manage/7504103/runme…` (truncated by the print
  layout). `7504103` is an Athletic.net **internal meet-management id**; it is *not* in the public
  `MeetID` space seen in the HARs (667433–667524). `[INFERENCE]` that it is the RunMeet meet id, not
  the public MeetID — the mapping direction is name → public MeetID, not this number.

### Athletic.net leverage

**Item 3 answer — inventory of WIAA tournament/result pages for provider links.**

28 WIAA pages were fetched and link-extracted (8 sport-section pages, 12 schedules/scores/standings
sub-pages, 4 `-tournament/<sport>-assignments` pages, and 4 wrong-path `*-assignments` URLs that 404).
Result of the full anchor scan:

| Finding | Count | Detail |
|---|---:|---|
| Direct `athletic.net` **anchor** (24 live pages; 28 URLs fetched incl. 4 × 404) | **0 / 28** | The single `athletic.net` string on the T&F tournament pages is the *filename* of a local PDF: `…/athetic.net-track-instructions.pdf` (WIAA's own typo), served from `www.wiaawi.org`. |
| Links into a timing provider | **2 / 24 live pages** | `https://live.pttiming.com/xc-ptt.html?mid=5127` on both XC tournament pages (boys and girls are the same content, 1 link each). No other timer host (`performancetiming`, `k2timing`, `onyourmarkstiming`, `accuracetiming`, `tracksidetiming`, `racetec`) appears on any page. |
| Prose-only provider mentions (not links) | 4 pages | T&F schedules pages: `Regional Entry Form Deadline (PTTiming): May 22 at 8 a.m.`; XC tournament pages: `Sectional Entry Form Deadline (PTTiming/Athletic.net) - Tuesday, October 20th, 11:59 pm`. |
| Pages linking an *Athletic.net-bearing* PDF | 4 pages | The T&F tournament pages link the track instructions PDF; the XC tournament pages link the XC instructions PDF. |
| Athletic.net URLs inside those instructions PDFs | **5 distinct** | T&F PDF = 6 `/URI` annotations (4 × `support.athletic.net` + `https://www.athletic.net/account/login/signup` + `https://www.athletic.net/events/usa/wisconsin/2026-5-26`). XC PDF = 7 (4 × support + signup + `…/events/usa/wisconsin/2026-10-23` + `…/2026-10-24`). |
| Sampled **result** PDFs carrying an Athletic.net URL | **2 of 26** | `tr2026madisonregional.pdf` prints the manage-meet URL (9×); `tr2026hilbertsectionalindiv.pdf` prints `2026 Athletic.net - All rights reserved`. |
| Sampled result PDFs whose `/Title` is a RunMeet export | 13 of 26 | AN-produced files, but with no AN URL printed. |

**What this buys:** the XC instructions PDF hands over **two extra Athletic.net date indexes**
(`/events/usa/wisconsin/2026-10-23`, `…/2026-10-24`) on top of the already-known regional index
(`…/2026-5-26`), so the *tournament* meet set is seedable from WIAA documents without any search. The
school/team universe (4 `RunSportListReport` calls) and the grade-bearing tournament results remain the
two non-AN-replaceable assets. No new per-athlete or per-meet AN request savings were found beyond
[06]'s ≈500–1,800 team requests + ≈50 meet requests.

### Athlete evidence

Extraction of the 26-file sample (below) recovers **2,786 grade-11 rows** with a tolerant
`Name … 11 School` regex across the 24 extractable files; e.g. `tr2026homesteadsectional.pdf` 69,
`tr2026kielregionalindiv.pdf` 182, `trb2026stratfordregional.pdf` 133, `tr2026pittsvilleregional.pdf` 218.
Row shapes seen (`-layout`, verbatim):

```
 1 Quintero, Gianni        12 Marquette University   4:09.59    4:07.34      (Hy-Tek/Crystal print)
 1 Westrich, Peyton       10 Bonduel            2     59.41   10             (RUNMEET/AN export)
 1    Mateo Aizpurua     9     AMERY        10.90              10           (AN export, 2-col layout)
```

Field availability is as in [06] (name, `Yr`, school, mark, heat, round, relay legs with per-leg grade;
no AthleteID, no profile URL). New nuance: **two layout families** in the 2026 set — single-column
Hy-Tek/RUNMEET prints (`Name Yr School H# Result`) and a two-column export layout whose columns
*interleave* in `pdftotext -layout` output (see `tr2026suringregional.txt`, `tr2026chiltonsectionalindiv.txt`),
so the parser must prefer column-band splitting over naive line parsing for that family.

### Recruiting information

**Item 1 answer — is there an updated-since / last-modified signal for the coach directory?**

| Probe | Result |
|---|---|
| `GET /Directory/School/GetDirectorySchool?orgID=1` | 200, **181,100 B, 0.520 s**; headers: `cache-control: no-cache, no-store`, `expires: -1`, `cf-cache-status: DYNAMIC`, `x-aspnetmvc-version: 5.2`, **no `ETag`, no `Last-Modified`** |
| Same URL + `If-None-Match`/`If-Modified-Since` | **200 with the full 181,100 B body** — no 304 is possible because no validators exist (not even a synthetic one) |
| `…?orgID=222` | 200, **218,703 B, 0.693 s**, identical header set |
| `POST /Reports/RunSportListReport/` | 200, 406,192 B, 6.39 s, `no-cache, no-store` |
| Timestamps *inside* the payload | School page HTML: **zero** date-like strings (`\b20\d\d-\d\d-\d\d\b` or `d/d/20\d\d`) outside JS boilerplate. `GET /Profile/Account/SearchContactNames?query=Knapmiller` → JSON fields are exactly `ContactID, FirstName, LastName, FullName, Email, City, SearchType, OrgID, SchoolYear` — **no modified/updated field**. `POST /Profile/Account/ContactInfoResults` (ContactID=3390) → 3,769 B, only the scope label `Contact Information For the 2026-2027 school year`. |
| Lighter-shaped route for coaches? | No: the coach table is server-rendered inline (`<h4>Head Coaches</h4>… <table id="tblCoachList">… "Found 14 coaches"`) inside the same 181 KB page; there is no coach-only AJAX endpoint in the page's JS. |
| Bulk export? | No: `GET /Reports/` → **403**; `/Reports/SportList/` exposes only `RunSportListReport` (+ links to `/Reports/CoopList`, `/Reports/PlayoffField`); the `RunSportListReport` HTML contains **0** occurrences of the string `Coach`; `/Profile/Account/SearchContacts` is a free-text name/email autocomplete with `minLength: 3` and **no school-scoped listing**. |
| Coach-association alternative (WIAA's own `/my-wiaa/coaches` links) | `Wisconsin Track Coaches Association` → `http://wistca.ihigh.com/` → **DNS does not resolve** (curl exit 6, no HTTP). `Wisconsin Cross Country Coaches Association` → `http://www.wisconsinrunner.com/wccca/` → 301 → 302 → `https://wisconsinrunner.com/404.html` (**404**). No live WI track/XC coach feed exists there. |

**Quantified cost of poll-per-school (measured byte/time, one school page = one poll):**

| Scenario | Requests | Bytes | Single-thread wall time |
|---|---:|---:|---:|
| 1 school (coach refresh) | 1 | 181,100–218,703 B (mean 199,902 B ≈ 195 KiB) | 0.52–0.69 s |
| Full 516-school sweep | 516 | **≈ 93–113 MB** (mean ≈ 103 MB) | **≈ 5.3 min** (516 × 0.61 s) |
| Weekly rolling refresh (516 ÷ 52) | 10 | ≈ 1.95 MB | ≈ 6 s |
| For comparison: the whole team universe (`RunSportListReport` ×4) | 4 | 1.58 MB | ≈ 15–25 s (6.4 s measured for one call) |

Because there is **no change feed of any kind** (no validator, no timestamp, no bulk endpoint, no
association feed), poll-per-school is the only route; the rolling 10-schools/week schedule costs
≈ 26 MB/quarter and is the cheapest compliant design.

### Result evidence

**Item 2 answer — 2026 RUNMEET-style track PDFs.** 26 newly downloaded 2026 files, 23.3 MB, 351 pages
(mean 13.5 pages / 917 KB); all fetched from the tournament-page-linked `2026-08` upload directory.
Classified with `pdfinfo`/`pdffonts`/`pdftotext -layout` + a grade-11 row probe:

| File | KB | pg | Producer | Verdict |
|---|---:|---:|---|---|
| tr2026beaverdamsectionalindiv.pdf | 2,584 | 7 | Microsoft: Print To PDF | **RASTER — 0 embedded fonts**, `pdftotext` returns 7 chars (form feeds only) |
| tr2026marathonsectional.pdf | 1,529 | 22 | Acrobat Distiller 26.0 | **MOJIBAKE — Type 3 fonts, Custom encoding, not embedded, no ToUnicode** (`pdftotext` yields pseudo-random Latin-1, 0 recoverable grade-11 rows) |
| tr2026chiltonsectionalindiv.pdf | 1,658 | 19 | Skia/PDF m148 | extractable (154 g11 rows; 99 `…` + 38 PUA icon glyphs are the only non-ASCII) |
| tr2026horiconsectionalindiv.pdf | 2,008 | 20 | Skia/PDF m148 | extractable (206) |
| tr2026hilbertregional.pdf | 1,894 | 32 | Skia/PDF m148 | extractable (248) |
| tr2026greendaleregional.pdf | 1,284 | 27 | Skia/PDF m148 | extractable (150) |
| tr2026madisonregional.pdf | 512 | 9 | Microsoft: Print To PDF | extractable (144) — carries the AN manage URL |
| tr2026pittsvilleregional.pdf | 1,535 | 13 | iLovePDF | extractable (218) |
| tr2026kielregionalindiv.pdf | 1,420 | 13 | Skia/PDF m148 | extractable (182) |
| tr2026ricelakesectional.pdf | 1,365 | 19 | Nitro PDF Pro 14 | extractable (137) |
| tr2026mondoviregional.pdf | 1,358 | 13 | Skia/PDF m148 | extractable (5) |
| tr2026veronasectional.pdf | 749 | 7 | Skia/PDF m148 | extractable (87) |
| tr2026westdeperesectionalindiv.pdf | 797 | 7 | Skia/PDF m148 | extractable (117) |
| tr2026suringregional.pdf | 611 | 8 | Microsoft: Print To PDF | extractable (115) — `-layout` interleaves two columns |
| tr2026shorewoodregional.pdf | 578 | 5 | Microsoft: Print To PDF | extractable (51) |
| tr2026waupacaregional.pdf | 809 | 8 | Powered By Crystal | extractable (98) |
| tr2026franklinregional.pdf | 317 | 13 | Microsoft: Print To PDF | extractable (105) |
| tr2026graftonregional.pdf | 126 | 15 | Powered By Crystal | extractable (121) |
| tr2026homesteadsectional.pdf | 108 | 12 | Powered By Crystal | extractable (69) |
| tr2026mukwonagosectional.pdf | 106 | 12 | Powered By Crystal | extractable (65) |
| tr2026neenahregional-.pdf | 121 | 15 | Powered By Crystal | extractable (116) |
| tr2026southmilwsectional.pdf | 107 | 11 | Powered By Crystal | extractable (59) |
| tr2026colfaxregional.pdf | 98 | 10 | Powered By Crystal | extractable (166) |
| trb2026altoonaregional.pdf | 134 | 13 | Powered By Crystal | extractable (39) |
| trb2026stratfordregional.pdf | 989 | 9 | Pdftools SDK | extractable (133) |
| instr-track/…, instr-xc/… (instructions) | 233 / 80 | 1 | Skia/PDF (Google Docs) | extractable |

**Measured result: 24 of 26 extractable (92.3%), 2 non-extractable (7.7%)** — both failure modes are
producer-side, not site-side: one file has *no text layer at all* (zero fonts, raster), the other has a
Type 3 text layer with no ToUnicode mapping. `pdftotext` cost: **1.0 s total across the 25 files where
it was timed (0.04 s/file)**.

**OCR fallback (tesseract 5.5.3, `-l eng --psm 6`), measured on this box (Ryzen 9 9950X3D):**

| Stage | 150 dpi | 200 dpi | 300 dpi | 400 dpi |
|---|---:|---:|---:|---:|
| `pdftoppm -png` render, s/page | 0.22 | 0.31 | 0.53 | 0.77 |
| `tesseract`, s/page | 0.66 | 0.75 | 1.09 | 1.18 |
| text chars / grade-11 rows on the test page | 2,807 / 6 | 2,817 / 6 | 2,833 / 6 | 2,837 / 6 |

- **200 dpi is the sweet spot**: 1.06 s/page with identical row yield; 300 dpi (1.61 s/page) adds
  characters but no rows. Full-file end-to-end: `tr2026beaverdamsectionalindiv.pdf` (7 pages, raster)
  OCR'd in **11.4 s**; `tr2026marathonsectional.pdf` (22 pages, Type 3) projects to **≈ 35 s**
  (3 pages measured at 1.60 s/page).
- **OCR output is usable**: page 1 of the raster Beaver Dam sectional yields
  `Athlete  Yr  Team  Finals  Pts` and rows such as `1 Post, Taylor 12 BEAVER DAM 6:20.43 10`,
  relay legs `1) Hackman, Carter 11 2) Hackman, Ethan 12`; the Type-3 Marathon file yields
  `1 CHEQUAMEGON 'A 11:21.47 / 1) Ernst, Mady 11 2) McKuen, Kaydence 11`. Noise is present in digits
  and long surnames (`Engeman, Ellie 17`, `Risik, Lydia So`) ⇒ treat OCR marks as *reported*, and keep
  grade/name as the primary claims.
- **Season projection:** 131 PDFs in the 2026 T&F set × 7.7% ⇒ **≈ 10 OCR files / ≈ 136 pages ⇒ ≈ 220 s
  single-thread at 300 dpi (≈ 145 s at 200 dpi; ≈ 30 s at 8-way parallelism)**. If the naive first-pass
  classifier's worst case (23%) were used it would still be ≈ 400 pages / ≈ 11 min. **OCR is not a
  blocker for the WI tournament corpus.**

**Change detection for the result tier (measured, and a correction to a common assumption):**

| Probe | Observed |
|---|---|
| `HEAD /sites/default/files/2026-08/tr2026shorewoodregional.pdf` | 200, **0 bytes**, 0.087 s, returns `ETag: "6a95b2f9-9094b"`, `Last-Modified: Mon, 31 Aug 2026 16:59:37 GMT`, `Content-Length: 592203` → **cheap, real (origin mtime) change probe** |
| `GET` same file with matching `If-None-Match` + `If-Modified-Since` | **200 + full 592,203-byte body** — the validators are advertised but **not honored**; do not build the plan on 304s for files |
| `GET` archive page `/sports/boys-track-field/boys-track-field-state-archive` | 200, 519,413 B; `cache-control: max-age=600, public`, `last-modified: 2026-09-20 14:05:21 GMT` (= page-render time), `x-drupal-cache: MISS`, `age: 363` |
| `GET` + `If-Modified-Since` (that value) | **304, 0 bytes** — page-level revalidation *does* work, but only inside the 600 s cache window; the timestamp is regenerated with the cache entry, so it cannot express "content unchanged since last week" |
| Sitemap `<lastmod>` for the same page | `2026-08-31T17:09:33-04:00` — the **content** change date, i.e. the sitemap is the real updated-since signal, the HTTP header is not |

### Incremental use

Weekly plan, all measured costs:

1. **Page-change detection (7 requests incl. the index, 1.93 MB, ≈2 s):** GET `sitemap.xml` + `?page=1..6`, diff the
   `<lastmod>` map. A changed archive/tournament page (e.g. boys T&F archive = 2026-08-31, girls T&F
   archive = 2026-09-14, XC tournament pages = 2026-09-17) is the trigger. This **replaces** blind
   polling of 4 sport pages and gives a true updated-since signal.
2. **File-diff (2 requests):** fetch only the changed archive page(s), diff the href set (134 unique
   2026 links today), normalise `/Portals/0/…` variants, tolerate per-link 404s (the girls archive still
   has a broken `stratfordregionalb.pdf` link → 301 → 404).
3. **New file triage (1 HEAD/file, 0.09 s, 0 bytes):** compare `Last-Modified` against the stored value;
   download only changed/new files (923 KB mean).
4. **Extraction:** `pdftotext -layout` for step 3 survivors (0.04 s/file), then `pdfinfo /Title` to tag
   `RUNMEET: <meet>` names for the AN-name join. Fallback OCR (`pdftoppm -r 200` + `tesseract --psm 6`,
   1.06 s/page) only for files where the grade-11 probe returns 0 rows — ≈10 files/season.
5. **Coach tier (10 requests/week, 1.9 MB, ≈6 s):** rolling 10-schools-per-week sweep of
   `/Directory/School/GetDirectorySchool?orgID=…`, diffing the `tblCoachList` rows per `OrganizationID`
   (keep name/role/sport/email; drop City/Work Phone per [06]'s privacy rule). No conditional GET is
   possible; every poll is a full 181–219 KB transfer.

### Access characteristics

- **Classes:** normal HTML (Drupal) + HTML-over-POST (AJAX fragments) + small JSON + **static PDF/HTM**.
  No documented API, no JSON:API on the association site (404, [06]), no bulk school/coach export.
- **Authentication:** none required for any endpoint used here (school DB, sitemap, result files).
- **Rate limits:** none published (`www` robots.txt = stock Drupal, no `Crawl-delay`, does not disallow
  `/sites/default/files/**`; `schools.wiaawi.org/robots.txt` = 404). **Zero 429s, zero `Retry-After`**
  across this session's ≈85 requests. Cloudflare front ends both hosts; no challenge was ever triggered.
- **Caching / validators:** static files advertise `ETag` + `Last-Modified` but **ignore conditional
  requests** (`200` + full body); Drupal HTML pages honor `If-Modified-Since` with **304** inside their
  600 s window (their `Last-Modified` = render time, not content time); the school DB sets
  `no-cache, no-store` and exposes **no validators at all**. A sitemap `<lastmod>` map is the only
  content-accurate freshness signal.
- **HEAD is supported** on static files (fast change probe).
- **Access quirks to keep:** `GET /Reports/` → **403**; `*-assignments` 404s unless under the
  `-tournament/` sub-path; `tr2026stratfordregionalb.pdf` (girls archive, `/Portals/0/…`) → 301 → 404
  while the boys-archive `/sites/default/files/2026-08/trb2026stratfordregional.pdf` → 200; 3 of the
  134 2026 links are `.htm` (not PDF); file sizes 98 KB–2.6 MB.

### Recommendation

**Keep WIAA at [06]'s level (PRIMARY school/team universe + COACH-DIRECTORY + VALIDATION for grades) and
adopt the following three specific changes, all measured today:**

1. **Coach refresh — accept poll-per-school, and stop looking for a feed.** No change signal exists at
   the HTTP level (no validators, verified `no-cache, no-store` + 200 on a conditional GET), at the
   payload level (no timestamp field in any of the three contact endpoints; no date strings in the
   school page), or at the bulk level (`/Reports/` 403, only `RunSportListReport`/`CoopList`/
   `PlayoffField` exist, contact search is free-text-only, and `RunSportListReport` has no coach column).
   Neither WIAA-published coach association is reachable (DNS failure / 404). Cost: 1 request
   (≈195 KiB, 0.6 s) per school; 516-school sweep ≈ 103 MB / ≈5 min; recommended cadence = 10 schools/week
   (1.9 MB, 6 s) on a rolling roster, with a full re-sweep only once per season.
2. **2026 track PDFs — treat extraction as per-file, not per-site.** 92% (24/26) of 2026 files extract
   with `pdftotext` at 0.04 s/file; the 8% that fail do so for two producer-side reasons (zero-font
   raster; Type-3 fonts without ToUnicode) and **tesseract 5.5.3 is installed and recovers rows**
   (name/`Yr`/school/mark) at **1.06 s/page @200 dpi**. Season-wide fallback ≈ 10 files / ≈136 pages
   / ≈2.5 min single-threaded. Correct [08]'s blanket "2026 track PDFs are not text-extractable" to
   "**file-specific**; both failure classes are OCR-recoverable".
3. **Provider links — WIAA's pages are not the provider index; its PDFs are.** 0 of 24 live pages link to
   Athletic.net, 2 of 24 link a timing provider (`live.pttiming.com/xc-ptt.html?mid=5127`). The
   Athletic.net entry points live inside two instructions PDFs (5 distinct `www.athletic.net` URLs,
   including two previously-unknown XC sectional date indexes `…/2026-10-23` and `…/2026-10-24`) and
   inside RunMeet-printed result files (13/26 sampled files carry a `RUNMEET: <meet>` title; one prints
   the manage-meet URL). **Recommendation: DISCOVERY-ONLY for the provider link graph; PRIMARY for the
   file corpus.**

### Evidence appendix

All times 2026-09-20 America/Chicago (CDT), `www.wiaawi.org` and `schools.wiaawi.org`, browser UA
`Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 … Chrome/140.0.0.0 Safari/537.36`. `read`/`curl`
GET unless stated. **No request was sent to any `*.athletic.net` host** (constraint), and every
Athletic.net URL below was read from a downloaded file.

| URL | Method | HTTP status | What it proved | Timestamp |
|---|---|---|---|---|
| `/sports/boys-track-field/boys-track-field-tournament` | GET | 200 (224,999 B) | 0 `athletic.net` anchors; links local `athetic.net-track-instructions.pdf`; 2026 state result links still under `/Portals/0/…` | 09:05:15 |
| `/sports/girls-track-field/girls-track-field-tournament` | GET | 200 (235,629 B) | Same by gender (75 "2026ish" doc links) | 09:05:20 |
| `/sports/boys-track-field/boys-track-field-state-archive` | GET | 200 (519,413 B) | 103 unique 2026 file links; 1,741 doc links total | 09:05:27 |
| `/sports/girls-track-field/girls-track-field-state-archive` | GET | 200 (514,445 B) | 97 unique 2026 links; `/Portals/0/PDF/Results/Track/2026/…` naming variant | 09:05:32 |
| `/sports/boys-cross-country/boys-cross-country-tournament` | GET | 200 (231,064 B) | **only timer link found**: `https://live.pttiming.com/xc-ptt.html?mid=5127`; prose `(PTTiming/Athletic.net)`; links XC instructions PDF | 09:05:36 |
| `/sports/girls-cross-country/girls-cross-country-tournament` | GET | 200 (223,620 B) | Same content | 09:05:38 |
| `/sports/boys-cross-country/boys-cross-country-state-archive` | GET | 200 (528,717 B) | 1,785 doc links, 0 provider links | 09:05:40 |
| `/sports/girls-cross-country/girls-cross-country-state-archive` | GET | 200 (520,101 B) | 1,700 doc links, 0 provider links | 09:05:41 |
| `/sports/{4 sports}/{sport}-schedules` | GET ×4 | 200 (182,729–182,970 B) | T&F schedules carry `Regional Entry Form Deadline (PTTiming): May 22` (prose, no link); XC schedules = calendar only | 09:06:16–09:06:2x |
| `/sports/{4 sports}/{sport}-scores`, `…-standings` | GET ×8 | 200 (181,792–181,912 B) | Navigation shells; **no scores/standings content, no provider links** | 09:06:2x |
| `/sports/{4 sports}/{sport}-assignments` | GET ×4 | **404** (175,7xx B each) | Assignment content lives under the tournament sub-path | 09:06:3x |
| `/sports/{4 sports}/{sport}-tournament/{sport}-assignments` | GET ×4 | 200 (179,680–180,259 B) | PDF-only (D1/D2/D3 assignment PDFs); 0 provider links, 0 AN links | 09:06:54 |
| `/sites/default/files/2026-08/<14 files>` | GET ×14 | 200 | Extraction sample #1 (see §Result evidence); all 200 with `Last-Modified` 2026-08-27…08-31 | 09:07:16–09:08:31 |
| `/sites/default/files/pdfs/PDF/Sports/Track/2026/athetic.net-track-instructions.pdf` | GET | 200 (239,369 B) | 6 `/URI` annotations: 4 `support.athletic.net` + `www.athletic.net/account/login/signup` + `www.athletic.net/events/usa/wisconsin/2026-5-26`; text "WIAA is transitioning to Athletic.net for the Tournament registration" | 09:07:39 |
| `/sites/default/files/pdfs/PDF/Sports/Cross_Country/2026/instructions-for-entries.pdf` | GET | 200 (81,950 B) | 7 `/URI` annotations incl. **two new** AN date indexes `…/events/usa/wisconsin/2026-10-23` and `…/2026-10-24` | 09:07:44 |
| `/sites/default/files/2026-08/<11 more files>`, `tr2026stratfordregionalb.pdf` | GET ×12 | 11 × 200, 1 × **404** | Sample #2; the guessed `tr2026stratfordregionalb` name does not exist in the 2026-08 dir | 09:08:0x–09:08:31 |
| `/sports/boys-track-field/boys-track-field-state-archive` | GET + `If-Modified-Since` | 200 (519,413 B) → **304** (0 B) | Drupal page honors If-Modified-Since inside its 600 s window; `Last-Modified` = render time | 09:11:30 / 09:11:45 |
| same page, re-fetch ~7 min later + re-validate | GET ×2 | 200 + 304 | Same `Last-Modified` (14:05:21Z) with `age` 397→398 ⇒ header tracks the cache entry, sitemap `lastmod` (2026-08-31) is the content date | 09:12:04 / 09:12:05 |
| `/sites/default/files/2026-08/tr2026shorewoodregional.pdf` | GET, then GET with `If-None-Match`+`If-Modified-Since` | 200 (592,203 B) → **200 (592,203 B)** | File validators are advertised but **not honored** (no 304, full body) | 09:11:28 |
| same file | **HEAD** | 200 (0 B, 0.087 s) | HEAD returns `Last-Modified`+`ETag`+`Content-Length` ⇒ cheap per-file change probe | 09:11:43 |
| `/sitemap.xml` | GET | 200 (1,048 B) | 6-page index, no lastmod at index level | 09:10:47 |
| `/sitemap.xml?page=1..6` | GET ×6 | 200 (237,765–363,293 B) | **11,332 URL+`<lastmod>` pairs**; the 8 tracked sport pages (plus their 4 assignment sub-pages) carry content lastmods (e.g. boys T&F archive 2026-08-31, XC tournament 2026-09-17); all 11,332 URLs are `www.wiaawi.org` — the coach DB host is absent | 09:10:54–09:11:18 |
| `/my-wiaa/coaches` | GET | 200 (185,636 B) | Publishes the state coach-association list, incl. `Wisconsin Track Coaches Association → http://wistca.ihigh.com/` and `Wisconsin Cross Country Coaches Association → http://www.wisconsinrunner.com/wccca/` | 09:11:10 |
| `http://wistca.ihigh.com/` | GET | **000 (DNS not resolved**, curl exit 6) | WTCA link is dead | 09:12:4x |
| `http://www.wisconsinrunner.com/wccca/` | GET (followed) | 301 → 302 → **404** (`/404.html`, 1,750 B) | WCCCA link is dead ⇒ no WI coach-association feed | 09:12:4x |
| `https://www.wiaawi.org/Portals/0/PDF/Results/Track/2026/stratfordregionalb.pdf` | GET (followed) | 301 → **404** | Girls-archive 2026 link is genuinely broken; the boys-archive counterpart `/sites/default/files/2026-08/trb2026stratfordregional.pdf` → **200** (1,012,583 B, 9 pages, 133 grade-11 rows, `D3 Regional 1D - Stratford`) | 09:12:4x |
| `schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=1` | GET | 200 (181,100 B, 0.520 s) | `no-cache, no-store`, `expires: -1`, **no ETag/Last-Modified**; coach table (`tblCoachList`, "Found 14 coaches") server-rendered inline | 09:10:03 |
| same, + `If-None-Match`/`If-Modified-Since` | GET | 200 (181,100 B) | No validators ⇒ no 304 possible; full body re-transferred | 09:10:03 |
| `…?orgID=222` | GET | 200 (218,703 B, 0.693 s) | Second school, identical header set | 09:10:06 |
| `schools.wiaawi.org/Reports/RunSportListReport/` (POST SportSeasonID=1544) | POST | 200 (406,192 B, 6.39 s) | Report HTML contains **0** `Coach` occurrences; `no-cache, no-store` | 09:10:13 |
| `schools.wiaawi.org/Profile/Account/SearchContactNames?query=Knapmiller` | GET | 200 (396 B JSON) | Fields = `ContactID, FirstName, LastName, FullName, Email, City, SearchType, OrgID, SchoolYear` — **no updated/modified timestamp** | 09:10:1x |
| `schools.wiaawi.org/Profile/Account/ContactInfoResults` (ContactID=3390) | POST | 200 (3,769 B) | Only the scope label `For the 2026-2027 school year`; **no dates** | 09:10:23 |
| `schools.wiaawi.org/Reports/` | GET | **403** (1,233 B) | No report index / no bulk export page | 09:10:31 |
| `schools.wiaawi.org/Profile/Account/SearchContacts` | GET | 200 (119,123 B) | Autocomplete UI, `minLength: 3`, free-text name/email only ⇒ no school-scoped or bulk listing | 09:10:3x |
| `schools.wiaawi.org/Reports/SportList/` | GET | 200 (121,940 B) | Exposes only `RunSportListReport` (+ links to `CoopList`, `PlayoffField`) | 09:10:46 |

**Local (no-network) measurements backing §Result evidence:** `pdftotext`/`pdfinfo`/`pdffonts`/`pdftoppm`/
`tesseract` timings above; per-file classifications and the 26-row sample table in
`research/midwest/evidence/gaps/35/` (`pdf-extract/summary-v2.json`, `sample-final.json`, per-file
`.txt` extracts; OCR text under `ocr/`; curated failing PDFs under `pdfs/`).
