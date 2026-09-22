# SOURCE_REPORT — state-assoc-southeast

Lane: `research/sources/state-assoc-southeast/` · Jurisdictions: **FL, GA, NC, SC, VA, WV**
Captured: 2026-09-22 (UTC), anonymous `curl` only. All raw bodies in `samples/`, indexed in `samples/CAPTURES.md`.
Field schema actually observed: `schema.json`. Machine-readable coverage: `coverage.json`.

Robots posture per host (verbatim bodies captured before fetching):

| Host | robots.txt | Verbatim salient lines |
|---|---|---|
| `www.fhsaa.com` | 200 (5,489 B) | `User-agent: *` → `Disallow: /images/` `/documents/` `/admin/` `/services/` `/site/` `/*.js$` `/*.css$` `/*.jpg$` `/*.gif$` `/*.axd` `/*print=true*`; `Allow: /`; **`Crawl-delay: 5`**; `Visit-time: 0100-0645` |
| `www.ghsa.net` | 200 (2,189 B) | Drupal stock: `User-agent: *` `Crawl-delay: 10`; disallows `/includes/ /misc/ /modules/ /profiles/ /scripts/ /themes/ /search/ /admin/ /node/add/ /user/login/` … |
| `www.nchsaa.org` | 200 (172 B) | `# START YOAST BLOCK` / `User-agent: *` / `Disallow:` (nothing disallowed) / `Sitemap: https://www.nchsaa.org/sitemap_index.xml` |
| `schsl.org` | 200 (110 B) | `User-agent: *` / `Disallow: /wp-admin/` / `Allow: /wp-admin/admin-ajax.php` / `Sitemap: https://schsl.org/wp-sitemap.xml` |
| `www.wvssac.org` | **404** (13 B body `404 Not Found`) | no robots.txt published |
| `fhsaa.homecampus.com` | 200 (24 B) | `User-agent: *` / `Disallow:` (nothing disallowed) |
| `florida.tfrrs.org` | 200 (99 B) | comment-only file, no directives |
| `www.vhsl.org` | (sibling lane capture) | `User-agent: *` / **`Disallow: /`** — whole-site disallow |
| `va.milesplit.com` | (sibling lane capture) | `User-agent: *` → `Disallow: /rankings` `/virtual-meets` `/api/` `/contact` |

Request cost actually paid in this lane: **89 HTTP requests** (79× 200, 6× 404, 2× 403, 2× curl TLS failure against `www.milestat.com`) across 12 hosts, per-host counts `nchsaa 18 / schsl 16 / ghsa 15 / wvssac 10 / tfrrs 8 / homecampus 6 / fhsaa.com 6 / va.milesplit 4 / rest ≤2`; spacing ≥1 s/host, 5 s for `fhsaa.com`, 11 s for `ghsa.net`. No 429 and no `Retry-After` header observed on any host. Request log: `/tmp/resfetch.log` (91 lines = 89 requests + 2 curl continuation lines).

---

## FL — Florida High School Athletic Association (FHSAA)

Source name: **Florida High School Athletic Association**, `https://fhsaa.com/` (`www.fhsaa.com` → 301 → apex `fhsaa.com`).
Platform: SIDEARM on Microsoft-IIS/10.0 + ASP.NET (`x-aspnet-version: 4.0.30319`); page bodies embed their content as a `var component = {…}` JSON blob plus a school universe served from a **separate host**, `fhsaa.homecampus.com` (Home Campus). Evidence: `samples/home-fhsaa.html`, `samples/member-directory-fhsaa.html`, `samples/robots-fhsaa.txt`.

| Field | Finding | Evidence |
|---|---|---|
| Geographic coverage | Florida only (single-state association) | `samples/home-fhsaa.html` (`window.client_hostname = "fhsaa.com"`, `"site":"fhsaa"`) |
| Sports | T&F (`/index.aspx?path=track` → `https://fhsaa.com/sports/track`) and XC (`?path=cross` → `https://fhsaa.com/sports/cross`); nav JSON also lists Baseball, Basketball (B/G), Beach Volleyball, Bowling, Competitive Cheerleading, Flag Football, Football, Golf, Lacrosse, Soccer, Softball, Swimming & Diving, Tennis, Volleyball (B/G), Water Polo, Weightlifting (B/G), Wrestling | `samples/home-fhsaa.html` (nav items `Track & Field -> /index.aspx?path=track`, `Cross Country -> /index.aspx?path=cross`) |
| Historical depth | T&F via TFRRS: `archives.html?year=2019 … ?year=2025` = **7 seasons** on one page, and the state-series page exposes **19 `/lists/<id>/` links** — 16 region sanctioned lists (4 classes × 4 regions, e.g. `/lists/5512/FHSAA_1A_Region_1_Sanctioned_SUBJECT_TO_CHANGE` … `/lists/5593/FHSAA_4A_Region_4_…`), plus `/lists/4832` wheelchair, `/lists/4833` ambulatory and `/lists/5134/2025_FHSAA_All_Florida_Official_Rankings`; class/year entry points `lists.html?class=1a…4a|conferences|independents` and district pages `FHSAA 4A District 1…15` are linked from the same page. XC: the FHSAA XC results page carries the 2026 season plus a "Previous Results" pointer | `samples/tfrrs-fl-state-series.html`, `samples/tfrrs-fl-archive-2026.html`, `samples/tfrrs-fl-class-4a.html`, `samples/xc-results-fhsaa.html` |
| Discovery mechanism | Association nav JSON → sport page → results sub-page; member-directory page embeds an `<iframe src="https://fhsaa.homecampus.com/widget/school/directory">` | `samples/member-directory-fhsaa.html` (verbatim: `<iframe frameborder="0" height="1020" scrolling="yes" src="https://fhsaa.homecampus.com/widget/school/directory" width="100%">`) |
| Stable identifiers | Home Campus `school.id` (e.g. `2397`), **`fhsaa_school_id`** (e.g. `714`), **`hytek_code`** (null for the sampled school), `section_id` `10` = FHSAA, `geoGroups` ids (County `1360` = Broward, Section `1554` = Section 4). TFRRS carries numeric list ids (e.g. `5512`, `5587`) | `samples/homecampus-school-details-2397.json`; `samples/tfrrs-fl-state-series.html` (`/lists/5512/FHSAA_1A_Region_1_Sanctioned_SUBJECT_TO_CHANGE`) |
| Pagination | Home Campus widget renders the **whole** FHSAA school list in one document (880 `<button … id="school-button-N">` rows, no pager markup). TFRRS relies on server-rendered pages keyed by `?class=`/`?year=` | `samples/homecampus-school-directory.html` |
| Athlete fields | **Yes, from TFRRS (FHSAA's official T&F result provider).** A performance-list page renders a div-grid (`div.performance-list-row` with `data-label` columns) in per-event sections: `Place \| Athlete \| Year \| Team \| Time/Mark \| Meet \| Meet Date \| Wind`. `Year` is the *class label* `FR/SO/JR/SR`, not a numeric grade. Measured on the 4A Region 1 sanctioned list: 36 event sections (18 Men + 18 Women, 19 distinct events), 1,469 athlete rows, 1,633 distinct athlete-profile links, class labels SR 529 / JR 487 / SO 291 / FR 167. Verbatim rows: `1 Josh Howell SR Creekside 10.53 Creekside Friday Knight Invite Mar 13, 2026 2.4` and `2 Jonathan Slack SO Winter Park 10.54 Metro East Championships Apr 10, 2026 1.8`. Athlete identity is a stable URL: `https://florida.tfrrs.org/athletes/8405585/Creekside/_Josh_Howell` | `samples/tfrrs-fl-list-5587-4a-region-1.html` |
| Meet fields | XC: state-meet page states venue/date (`Apalachee Regional Park (Tallahassee) November 20, 2026`), per-class/per-district/per-region qualifier lists, live results via PrimeTime Timing meet id **`mid=8269`**; T&F: state championships May 6–9, 2026 at UNF Hodges Stadium, regionals May 1–2, 2026, `listing of sites` → `/sports/2020/5/18/TK_Regions.aspx` | `samples/xc-results-fhsaa.html`, `samples/sport-track-fhsaa.html`, `samples/tk-regions-fhsaa.html` |
| Grade/class evidence | **Yes, but only as a class label — and not from FHSAA's own host.** TFRRS performance lists carry `Year ∈ {FR, SO, JR, SR}` per athlete row (see Athlete fields); a `JR` in the 2026 season is the Class of 2027. Numeric grades are not published anywhere sampled. The FHSAA-hosted Hy-Tek artifact that would carry numeric grade is robots-blocked (see Known blocks) | `samples/tfrrs-fl-list-5587-4a-region-1.html` (class labels SR 529 / JR 487 / SO 291 / FR 167); blocked artifact: `samples/xc-results-fhsaa.html` (anchor `FHSAA school codes` → `/documents/2026/9/3//26_27_Hy_Tek_Short.pdf?id=8028`) |
| Coach/contact fields | Real, structured coach/AD directory per school behind the widget API: `athleticFaculties[]` = `{id, firstname, lastname, aft_name, work_extension, work_phone, email}` and `coaches[]` = `{firstname, lastname, sport, level_id, sport_id, id, user_id, aft_name, email, level_name, na_coach}`. Sample school `2397`: 1 administrator (`Antonio Donascimento`, `aft_name":"Athletic Director"`, `adonascimento@alcaeagles.com`) and 5 coach rows (`Basketball, Boys` … `Varsity`), all with `firstname:null`/`na_coach:1` (i.e. coaching vacancy, not a name) | `samples/homecampus-school-details-2397.json` |
| Public API availability | **Yes, undocumented JSON**: `GET https://fhsaa.homecampus.com/widget/schools/get?school=<q>&section_id=10&status=active&hide_from_directory=0` → `[{"id":2037,"name":"LaBelle"},…]`; `GET https://fhsaa.homecampus.com/widget/get-school-details/<id>/details` → school/coach JSON. Both return **403** without `X-Requested-With: XMLHttpRequest` (measured) | `samples/homecampus-schools-get.json`, `samples/homecampus-school-details-2397.json` |
| Static file availability | PDFs/document artifacts live under `/documents/**` — e.g. `26_27_Hy_Tek_Short.pdf` — which the site's own robots.txt disallows. FHSAA school enrollment/participation numbers are also published as PDFs (membership page link list) | `samples/robots-fhsaa.txt`, `samples/xc-results-fhsaa.html`, `samples/membership-fhsaa.html` (nav `SCHOOL ENROLLMENT & PARTICIPATION NUMBERS`) |
| Browser requirement | Not required for the member directory (the widget HTML is server-rendered; the detail API is plain JSON with one XHR header). FHSAA's own sport pages are HTML with an embedded JSON component, parseable without JS | `samples/homecampus-school-directory.html` (880 buttons server-rendered) |
| Request cost | Member universe = **1 request** (widget page) or 1 request per search prefix via the API; per-school detail = 1 request per school (880 for the full FL universe). T&F results = 1 request per TFRRS list — **16 requests per season** cover all four classes' four regions (2.5 MB each), plus 1 per class/year page. XC = 1 page + PrimeTime | counts derived from `samples/homecampus-school-directory.html`, `samples/tfrrs-fl-state-series.html`, `samples/tfrrs-fl-list-5587-4a-region-1.html` (2,522,360 bytes) |
| Published rate limits | **`Crawl-delay: 5`** and `Visit-time: 0100-0645` for `User-agent: *`; `bingbot`/`msnbot` `Crawl-delay: 30`; Home Campus host has *no* delay directive | `samples/robots-fhsaa.txt`, `samples/robots-fhsaa-homecampus.txt` |
| Known blocks | `/documents/` (all result/document PDFs), `/images/`, `/*.js$`, `/*.css$`, `/*print=true*` disallowed by robots for `*`. Widget API 403 without XHR header. `window.block_dfp`, `benchbadbehavior`-class ad/consent stacks add noise but no access barrier | `samples/robots-fhsaa.txt`, `samples/CAPTURES.md` (403 rows at 03:56:01Z/03:56:02Z) |
| Cross-source join keys | `fhsaa_school_id` and `hytek_code` (Hy-Tek meet files), school display name (`"Abundant Life Christian (Margate)"` vs `full_name`), Home Campus `id` (widget ↔ detail API), TFRRS numeric list ids (e.g. `5512`, `5587`) and athlete ids (e.g. `8405585`). Note (measured): the sampled TFRRS list page contains **0** team anchors and **0** meet anchors — team and meet arrive as plain text, so joining TFRRS to Home Campus schools must go through the team-name string `[INFERENCE]` | `samples/homecampus-school-details-2397.json`, `samples/tfrrs-fl-list-5587-4a-region-1.html`, `samples/tfrrs-fl-state-series.html` |
| Estimated marginal coverage | School universe: **880 schools** enumerated from the widget (section 10), against FHSAA's own published "**over 850** member combination/senior high schools and middles schools" — i.e. the enumeration covers ≥ the association's own count (the widget includes middle schools, e.g. `LaBelle Middle`). Coach/AD rows for all 880 schools are reachable at 1 request/school. Results coverage: state series only (XC + T&F); **regular-season meets are not enumerated by FHSAA at all** `[INFERENCE]` from the two sport pages + results page containing only tournament artifacts | count: `samples/homecampus-school-directory.html` (880 `id="school-button-…"`, ids 1828–27368); published figure: `samples/membership-fhsaa.html` (`encompasses over 850 member combination/senior high schools and middles schools`) |
| Implementation recommendation | **PRIMARY** (school universe + coach/AD contacts via Home Campus widget API), **RESULT_SOURCE** for state-series results (TFRRS official — robots is comment-only, athlete rows carry the `FR/SO/JR/SR` class label and a stable athlete id; Half Mile Timing live; PrimeTime Timing for XC). FHSAA-hosted `/documents/` PDFs must be dropped from any plan: the site's own robots.txt disallows that path | — |

### FL — Athletic.net linkage

**No Athletic.net reference was found on any FHSAA surface captured in this lane** (0 case-insensitive matches for `athletic.net` across `home-fhsaa.html`, `member-directory-fhsaa.html`, `sport-cross-fhsaa.html`, `sport-track-fhsaa.html`, `xc-results-fhsaa.html`, `membership-fhsaa.html`, `homecampus-school-directory.html`, `homecampus-school-details-2397.json`). The only third-party consumer-facing linkage FHSAA publishes is MileSplit, and it is a **webcast credit**, verbatim:

```
<strong>Webcast:</strong> <a href="https://www.nfhsnetwork.com/associations/fhsaa" target="_blank">NFHS Network</a> via MileSplit<br />
```

FHSAA's substitution for Athletic.net is its own provider chain: Home Campus for entries/directory, **TFRRS** for T&F results and leaderboards (`florida.tfrrs.org`, whose `robots.txt` is comment-only, i.e. no directives), **Half Mile Timing** (`http://live.halfmiletiming.com/`) for T&F live results, **PrimeTime Timing** (`https://live.pttiming.com/xc-ptt.html?mid=8269`) for XC live results. Because TFRRS athlete rows carry the `FR/SO/JR/SR` class label plus a stable `/athletes/<id>/` URL, TFRRS is the only Class-of-2027-bearing surface found for FL in this lane.

---

## GA — Georgia High School Association (GHSA)

Source name: **Georgia High School Association**, `https://www.ghsa.net/`. Platform: Drupal 7 (`Generator" content="Drupal 7 (http://drupal.org)"`), plus a same-org subdomain `http://app.ghsa.net` (linked from the results page, **not probed**) and `https://learn.ghsa.net` (coaches education).

| Field | Finding | Evidence |
|---|---|---|
| Geographic coverage | Georgia only. Page title is year-stamped: `2026-2027 GHSA Member School Directory \| GHSA.net` | `samples/school-directory-ghsa.html` |
| Sports | T&F `/track-and-field`, XC `/cross-country` (nav lists all sports incl. `Adapted Track and Field`) | `samples/home-ghsa.html` |
| Historical depth | XC state-meet results: **320 unique PDFs across 25 seasons** (2000-2001 … 2024-2025); per season 10 files (2000-2012), 14 (2012-2016), 16 (2016-2025). T&F state-meet results page: 2026 + 2025 complete-meet PDFs (8 classes × 2 genders each) plus historic `ga.milesplit.com` meet links (2022: `meets/479387`, `479389`, `479447`, `479380`; earlier `346234/346233/346235`) and `ptgrouponline.com` live archives (2015-2019) | `samples/ghsa-xc-state-meet-results.html`, `samples/ghsa-state-track-meet-results.html` |
| Discovery mechanism | `/school-directory` (dropdown of every member school + PDF feed link) → `/ghsa-directory-feed/pdf`; sport page → `State Results` / `State Meet Results` quick-source links | `samples/school-directory-ghsa.html`, `samples/track-field-ghsa.html`, `samples/cross-country-ghsa.html` |
| Stable identifiers | **GHSA school id** in the directory dropdown `<option value="680">Academy for Classical Education</option>` (ids 6…713); **region-class label** `(2-AA)`; **Hy-Tek school codes** page `/hytek-codes-ghsa-schools` | `samples/school-directory-ghsa.html`, `samples/ghsa-directory-feed.pdf`, `samples/hytek-codes-ghsa.html` |
| Pagination | None needed: the whole member universe is one `<select id="edit-dropdown">` (458 options = 457 schools + placeholder). The PDF feed is a single 644,097-byte download | `samples/school-directory-ghsa.html` |
| Athlete fields | T&F meet result columns: `Name \| Year \| School \| Seed \| Finals \| Wind \| H# \| Po` (timed events) and `Name \| Year \| School \| Seed \| Finals \| H# \| Points` (throws); XC PDFs are per-class files | `samples/ghsa-2026-girls-6a-track-results.pdf` (extracted text: `Licensed to Perfect Timing Group ‐ Contractor License` / `HY‐TEK's Meet Manager 5/18/2026 05:54 PM` / `GHSA Track and Field State Championship ‐ 5/11/2026 to 5/14/2026` / `Spec Towns Track`) |
| Meet fields | State championships page: dates + venue + course map + ticket links per sport (e.g. `Cross Country November 6-7, 2026 Carrollton High School`, `Cheerleading November 13-14, 2026 Macon Centreplex`) | `samples/state-championships-ghsa.html` |
| Grade/class evidence | The T&F results PDF **has a `Year` column header** (verbatim header: `    Name                    Year School                  Seed     Finals Wind H# Po`) but the sampled 2026 girls 6A file populates **0 of 211** parsed result rows with a grade value (measured over 12 pages / 18 event sections) → grade is *not* recoverable from the GHSA-authoritative artifact for this file. Coach-level class/region is available via the `(region-class)` label | `samples/ghsa-2026-girls-6a-track-results.pdf` (211 parsed result rows, 0 grade tokens) |
| Coach/contact fields | **Best-in-class for this lane.** PDF feed `/ghsa-directory-feed/pdf` (338 pages, 20,963 extracted lines); per-school block: `NAME (region-class)`, street address, `City, GA ZIP`, `Phone:`, `AD:` (AD phone), `BD:` (band), `Fax:`, website, email, `Colors:`, `Mascot:`, then staff rows `Name CODE[,CODE][*]`. Key decoded verbatim in the PDF: `P Principal`, `AP Assistant Principal`, `AD Athletic Director`, `AAD Assistant Athletic Director`, `BD Band Director`, `LC Literary Coordinator`, `OAP One-Act Play`, `ES eSports Coach`, `1 Football Coach`, … **`5 Track Coach`**, … **`14 Cross Country Coach`**, `18 Athletic Trainer`, `* Head Coach`, `B Boys`, `G Girls`. Measured over the school section (PDF pages 27-321 of 338 = printed pp. 25-317): **23,435** staff rows, **22,789** distinct person names, **1,761** rows carrying a Track code (`5`/`5B`/`5G`), **946** carrying a Cross Country code (`14`/`14B`/`14G`), **672** `AD`, **443** `P`, **1,721** `AP` | `samples/ghsa-directory-feed.pdf` (text via `pdftotext -layout`) |
| Public API availability | None found (Drupal 7 HTML; `/ghsa-directory-feed/pdf` is a generated download, not JSON) | `samples/school-directory-ghsa.html` |
| Static file availability | Yes, and rich: `/sites/default/files/documents/track/*.pdf` (2025/2026 complete state-meet results), `/sites/default/files/documents/xc/*.pdf` (25 seasons), plus constitutions archive `/sites/default/files/documents/Constitution/GHSA_RR_*.pdf` | `samples/ghsa-state-track-meet-results.html`, `samples/ghsa-xc-state-meet-results.html`, `samples/ghsa-results-records.html` |
| Browser requirement | No. Dropdown + PDF feed are in the HTTP response; results pages list plain PDF links | `samples/school-directory-ghsa.html` |
| Request cost | Member universe: **1 request** (dropdown page) or **1 request** for the entire printable directory (644 KB PDF). Coach directory: 1 request total (PDF), i.e. ~457× cheaper than per-school HTML. Results: 1 page per sport + 1 PDF per class/gender/season | derived from `samples/ghsa-directory-feed.pdf` (644,097 B) |
| Published rate limits | **`Crawl-delay: 10`** for `User-agent: *` (highest in this lane) | `samples/robots-ghsa.txt` |
| Known blocks | `/search/`, `/admin/`, `/node/add/`, `/user/login/` disallowed; no paywall/CAPTCHA observed. State results pages numbered pre-2022 link out to third parties (MileSplit / ptgrouponline.com) rather than hosting files | `samples/robots-ghsa.txt`, `samples/ghsa-state-track-meet-results.html` |
| Cross-source join keys | GHSA dropdown `id` ↔ PDF school name (PDF uses short forms, e.g. `ACE CHARTER` = `Academy for Classical Education`); `(region-class)` label ↔ region alignments pages (`/2026-2027-region-alignments`); Hy-Tek school code page for meet-file joins | `samples/school-directory-ghsa.html`, `samples/ghsa-directory-feed.pdf`, `samples/hytek-codes-ghsa.html` |
| Estimated marginal coverage | **457 schools** in the sanctioned universe (dropdown; ids 6–713) and **456** `NAME (region-class)` blocks in the printable directory — two independent enumerations agreeing within one entry. Class split from the PDF parse: A 72, AA 85, AAA 62, AAAA 60, AAAAA 61, AAAAAA 64, AAAAAAA 52. Coach rows give sport-coded Track (1,761) and XC (946) coach assignments across the whole state, which Athletic.net/MileSplit do **not** expose at all | `samples/school-directory-ghsa.html` (458 `<option>` = 1 placeholder + 457 schools), `samples/ghsa-directory-feed.pdf` (456 distinct `NAME (region-class)` headers) |
| Implementation recommendation | **PRIMARY** (school universe + coach/AD directory + region-class), **RESULT_SOURCE** (state-meet PDFs 2000-2026 XC / 2025-2026 T&F) — the single strongest association source in this lane. Grade must come from elsewhere for GA | — |

### GA — Athletic.net / MileSplit linkage (verbatim markup)

On `https://www.ghsa.net/track-and-field`:

```html
<a href="http://ga.milesplit.com/" title="">MileSplitGA</a>
```

On `https://www.ghsa.net/cross-country` (two links):

```html
<a href="http://ga.milesplit.com/" title="">MileSplitGA</a>
<a href="http://ga.milesplit.com/stats" title="">State Rankings</a>
```

GHSA additionally runs a workflow page titled **"Report Region/Area Track Winners to MileSplit"** at `/report-regionarea-track-winners-milesplit-2027-04-24-000000`, i.e. MileSplit is the association's *designated* reporting sink for post-season T&F results — while the association itself republishes the classes' complete state-meet results as PDFs. **No `athletic.net` reference appears in any GHSA capture** (0 matches in `home-ghsa.html`, `school-directory-ghsa.html`, `track-field-ghsa.html`, `cross-country-ghsa.html`, `state-championships-ghsa.html`, `ghsa-state-track-meet-results.html`, `ghsa-xc-state-meet-results.html`, `ghsa-directory-feed.pdf`).

---

## NC — North Carolina High School Athletic Association (NCHSAA)

Source name: **NCHSAA**, `https://www.nchsaa.org/`. Platform: **WordPress 7.1 + SportsPress 2.7.31**, Yoast sitemap, **open WP REST API**.

| Field | Finding | Evidence |
|---|---|---|
| Geographic coverage | North Carolina only | `samples/home-nchsaa.html` |
| Sports | T&F (`/sports/track-and-field/`, championship at `/championships/track-field-state-championships/`), XC (`/sports/cross-country/`, `/championships/cross-country/`), plus indoor track (`/championships/indoor-track-state-championships/`) | `samples/home-nchsaa.html`, `samples/champ-track-nchsaa.html` |
| Historical depth | XC championship recap archive paginated (`/championships/cross-country/` → `page/2/`, `page/3/`), with back-catalogue to 2020-21 in the first page alone and per-season result PDFs at stable `/wp-content/uploads/<yyyy>/<mm>/…` paths | `samples/champ-cross-nchsaa.html`, `samples/xc-champs-2025-nchsaa.html` |
| Discovery mechanism | `/schools/` (member table) → `/championships/<sport>/` (season posts) → per-class result PDFs; `/record-books/`; `/sports-championships/` (brackets) | `samples/schools-nchsaa.html`, `samples/record-books-nchsaa.html`, `samples/sports-champs-nchsaa.html` |
| Stable identifiers | None per school in the table (name is the key) — stabilising keys are the **conference name**, **region** (`1`-`8`) and **classification** (`1A`…`8A`); WordPress REST gives `id`/`slug` per page/post; result-file identity is the `/wp-content/uploads/…` path | `samples/schools-nchsaa.html` |
| Pagination | Member-school table: none (single page, 451 rows). Championship recap lists: `/championships/cross-country/page/2/`, `page/3/` | `samples/champ-cross-nchsaa.html` |
| Athlete fields | From the Hy-Tek result PDF: `Name`, **`Year`** (grade 9-12), `School`, `Finals` (time), `Points` (team scoring) — header verbatim: `    Name                    Year School                  Finals Points` | `samples/xc-2025-5a-girls-nchsaa.pdf` |
| Meet fields | Per-class event header verbatim: `NCHSAA State Cross Country Championships - 10/31/2025 to 11/1/2025` / `Ivey Redmon Sports Complex` / `Event 11       Women 5k Run CC 5A` | `samples/xc-2025-5a-girls-nchsaa.pdf` |
| Grade/class evidence | **Yes, directly.** Measured on the 2025 5A girls file (printed page 1 of the class file): **146** result rows carrying a grade, distribution `9:29, 10:42, 11:44, 12:31` → **44** grade-11 athletes = Class of 2027 for the 2026 corpus season | `samples/xc-2025-5a-girls-nchsaa.pdf` |
| Coach/contact fields | No per-school coach table was found. The member page's second table is **conference-level**: `Conference \| Administrator \| Email` (77 rows, e.g. `Albemarle (AAC) 2A \| Sonya Rinehart \| srinehart@ecps.k12.nc.us`). `/schools/athletic-directors/` is a **links page, not a directory** (0 `<table>` elements) | `samples/schools-nchsaa.html`, `samples/ad-directory-nchsaa.html` |
| Public API availability | **Yes — WordPress REST.** `GET /wp-json/wp/v2/types` returns custom types incl. `nchsaa_championship`, `nchsaa_record_book`, `nchsaa_bracket`, `nchsaa_sport`, `nchsaa_staff` and SportsPress `sp_event`, `sp_team`, `sp_player`, `sp_staff`; `GET /wp-json/wp/v2/search?search=…` works (200) | `samples/wpjson-types-nchsaa.json`, `samples/wpjson-search-nchsaa.json` |
| Static file availability | Yes: result PDFs under `/wp-content/uploads/` (robots-allowed). 2025 XC = **16 result PDFs** (8 classes × 2 genders, `1A-Boys.pdf` … `8A-Girls-Results.pdf`) plus one unrelated HR notice (`Notice-of-Availability-Form-1095-B39.pdf`), all 17 linked from one page | `samples/xc-champs-2025-nchsaa.html` |
| Browser requirement | No | — |
| Request cost | School universe + conference contacts: **1 request** (`/schools/`). A full XC season of grade-bearing results: **1 page + 16 PDFs** | `samples/schools-nchsaa.html`, `samples/xc-champs-2025-nchsaa.html` |
| Published rate limits | None (robots allows everything; no `Crawl-delay`) | `samples/robots-nchsaa.txt` |
| Known blocks | None observed. One URL guess 404'd (`/track-and-field-state-championships/`, 347,648-byte 404 page) — NCHSAA's 404s are themed pages, not errors, so status codes must be checked | `samples/CAPTURES.md` (404 row at 04:00:16Z) |
| Cross-source join keys | School name (NCHSAA's only school key) ↔ MileSplit/Athletic.net team names; conference name ↔ conference administrator row; `Year` column ↔ class-of-year projections | `samples/schools-nchsaa.html`, `samples/xc-2025-5a-girls-nchsaa.pdf` |
| Estimated marginal coverage | **451 member schools** with region + classification + conference (2026-2027 table); **77 conference administrators** with emails; one class/gender of XC state results yields 142 athletes of which 43 are current juniors. Timing provider for the 2025 state XC meet: `Foothills Timing and Meet Management - Contractor License` | counts: `samples/schools-nchsaa.html`, `samples/xc-2025-5a-girls-nchsaa.pdf` |
| Implementation recommendation | **PRIMARY** (member universe + region/classification + conference contacts) and **RESULT_SOURCE** (grade-bearing Hy-Tek PDFs, open robots, stable `/wp-content/uploads/` paths) | — |

### NC — Athletic.net / MileSplit linkage (verbatim markup)

Every NCHSAA page template carries a footer link to MileSplit NC (8 occurrences across the seven pages captured):

```html
<a href="http://nc.milesplit.com/rankings">…</a>
```

On the XC championship archive the same target is rendered as plain text `http://nc.milesplit.com/rankings`. **No `athletic.net` reference was found in any NCHSAA capture.**

---

## SC — South Carolina High School League (SCHSL)

Source name: **South Carolina High School League**, `https://schsl.org/`. Platform: WordPress 7.1.1 (news-style theme, Site Kit by Google).

| Field | Finding | Evidence |
|---|---|---|
| Geographic coverage | South Carolina only | `samples/home-schsl.html` |
| Sports | XC `/cross-country`, T&F `/track-field` (nav lists Baseball, Basketball, Competitive Cheer, Cross Country, Football, Golf, Lacrosse, Soccer, Softball, Swimming, Tennis, Track & Field, Volleyball, Wrestling) | `samples/directory-schsl.html` |
| Historical depth | Championship posts exist per season (e.g. `2026 SCHSL Cross Country State Championships`, `2025 State Cross Country Championships`, `2024 Cross Country Championships`, `2021/2020 Class A-AAAAA XC` posts) but they are **announcements**, not result data | `samples/cross-country-schsl.html`, `samples/xc-state-champions-2026-schsl.html`, `samples/xc-state-championships-2025-schsl.html` |
| Discovery mechanism | Sport page → related news posts; `/archives/category/championship-information` index; `/archives/category/schools`; search `?s=` (WP) | `samples/champ-info-category-schsl.html`, `samples/search-xc-results-schsl.html` |
| Stable identifiers | WordPress `archives/<id>` post ids (e.g. reclassification post `17912`, XC championships `19510`, `4541`); **classification label** (`Class 5A`…`Class 1A`) and **region** (`Region 1`…`Region 10`) inside the reclassification post. No numeric school id observed | `samples/reclassification-2026-2028-schsl.html` |
| Pagination | Category/search listings paginate by WP standard (`/page/N/`); the reclassification post is a single document | `samples/champ-info-category-schsl.html` |
| Athlete fields | **None observed.** No athlete-level artifact was found on `schsl.org` in this lane (search for `cross country results` returns 2 posts, only one of which is an XC hub) | `samples/search-xc-results-schsl.html` |
| Meet fields | Championship posts give schedule/venue granularity per class and day, e.g. verbatim from the 2021 hub: `THURSDAY NOVEMBER 11, 2021 9:00 am AAAA Girls … at Clemson Sandhill Research Educational Center in Columbia, South Carolina`; 2026 qualifier announcement: `Qualifiers will be held November 6-7, 2026 in Newberry, SC` | `samples/xc-championships-schsl.html`, `samples/xc-state-champions-2026-schsl.html` |
| Grade/class evidence | **None on this host** (no grade-bearing artifact found) | — |
| Coach/contact fields | **None found.** `/schsl-directory` is a navigation hub; the site exposes `/schsl-staff`, `/ad-notebook`, `/jobs`, `/open-dates`; a `/jobs` feed contains coach-vacancy posts (e.g. `Girls JV Lacrosse Coach – Waccamaw HS`, `Girls Soccer Coach – Mid Carolina`) — useful only as a weak employment signal | `samples/directory-schsl.html`, `samples/cross-country-schsl.html` |
| Public API availability | WP REST is reachable in principle but **not verified in this lane** (no `/wp-json` probe was issued for SC before the request budget was spent) | `samples/robots-schsl.txt` (sitemap present: `https://schsl.org/wp-sitemap.xml`) |
| Static file availability | Yes but sparse for our sports: `/wp-content/uploads/…` holds forms (`BroadcastForm-Application-for-Schools-6.3.25.pdf`), dated qualifier score PDFs (`2023/11/cc5Qscores.pdf`, `CC4QScores.pdf`, `cc3Qscores.pdf`), and a 2022 program PDF | `samples/cross-country-schsl.html` |
| Browser requirement | Class pages (`/aaaaa`, `/aaaa`, `/aaa`, `/aa`, `/a`) render **navigation only** in raw HTML — the school lists they imply are not in the response. Treat as JS/browser-dependent unless another surface is found | `samples/class-aaaaa-schsl.html`, `samples/class-a-schsl.html` (0 `<table>` elements each) |
| Request cost | Membership = 1 request (reclassification post). Results = third-party (see linkage) | `samples/reclassification-2026-2028-schsl.html` |
| Published rate limits | None (robots allows all but `/wp-admin/`) | `samples/robots-schsl.txt` |
| Known blocks | No result data for T&F/XC on the association host; `schsl-directory` is not a directory; class pages are empty shells | as cited above |
| Cross-source join keys | School name ↔ classification/region (reclassification post) ↔ MileSplit SC team pages; school name ↔ `/brackets` playoff pages | `samples/reclassification-2026-2028-schsl.html`, `samples/brackets-schsl.html` |
| Estimated marginal coverage | **229 schools** enumerated from the 2026-2028 reclassification post — 5A 35, 4A 45, 3A 35, 2A 44, 1A 70 — parsed from the post's `Class … / Region …` blocks (10-11 regions per class), of which **21** carry the post's `**` marker (the post's own note: `**Denotes Schools Not Playing Football`). Source data-quality note: the Class 4A Region 1 roster contains the verbatim string `Westsid` immediately before `Region 2`, i.e. a typo in the published post rather than a parse artifact. **Caveat (measured):** this is a *classification* list published for realignment, not a certified membership roll. No coach contacts and no grade data. | `samples/reclassification-2026-2028-schsl.html` |
| Implementation recommendation | **PRIMARY** for the school universe/classification only; **DISCOVERY_SOURCE** for results (it points at MileSplit SC); **REJECT** for coach contacts and grades | — |

### SC — Athletic.net / MileSplit linkage (verbatim markup)

The 2021 XC state-championship hub post links MileSplit SC as the results hub:

```html
<a href="https://sc.milesplit.com/articles/305756/2021-south-carolina-cross-country-state-championships-hub">…</a>
```

Text on the same page: `Cross Country Championship Hub – Schedules, parking map, course map, live results`. **No `athletic.net` reference was found in any SCHSL capture** (`home-schsl.html`, `directory-schsl.html`, `track-field-schsl.html`, `cross-country-schsl.html`, `reclassification-2026-2028-schsl.html`, `xc-championships-schsl.html`).

---

## VA — Virginia High School League (VHSL)

Source name: **Virginia High School League**, `https://www.vhsl.org/` — **crawl-disallowed**.
Verbatim robots body (captured 2026-09-21 by sibling lane `state-assoc-midatlantic`, `../state-assoc-midatlantic/samples/robots-vhsl.txt`):

```
User-agent: *
Disallow: /

User-agent: RavenCrawler
Allow: /

User-agent: Googlebot
Allow: /
```

**No request to `vhsl.org` was issued by this lane.** Every VA finding below therefore comes from the MileSplit VA / MileStat surface, whose own robots body (also captured by the midatlantic lane) permits content paths while disallowing `/rankings`, `/virtual-meets`, `/api/`, `/contact`.

| Field | Finding | Evidence |
|---|---|---|
| Geographic coverage | Virginia only. `va.milesplit.com` carries the title `MileStat.com \| Virginia High School Running News and Videos \| Cross Country and Track & Field` | `samples/home-va-milesplit.html` |
| Sports | XC and T&F (indoor + outdoor: nav exposes `XC Lists`, `Indoor Lists`, `Outdoor Lists`) | `samples/vhsl-class5-state-meet-va-milesplit.html` |
| Historical depth | Not measured for VA in this lane (MileSplit archive paths were not enumerated). The site exposes an `Archive` nav entry and `Meet History` on the state meet page | `samples/vhsl-class5-state-meet-va-milesplit.html` (fields `Meet History \| Records \| Meet Venue \| Meet Records`) |
| Discovery mechanism | `milestat.com` (plain HTTP) → **301** → `va.milesplit.com`; state meet discovered by name (`VHSL Class 5 State Championships`) with numeric meet id `742518` | `samples/milestat-redirect.html` (location `http://va.milesplit.com/`), `samples/vhsl-class5-state-meet-va-milesplit.html` |
| Stable identifiers | MileSplit **meet id** (`/meets/742518-vhsl-class-5-state-championships-2026`); photo album id (`66486`). Note `https://www.milestat.com/robots.txt` and `https://www.milestat.com/` **fail TLS from curl 8.21.0** (`error:0A000126:SSL routines::unexpected eof while reading`) while plain-HTTP `milestat.com` works and redirects | `samples/CAPTURES.md` (two CURL_FAIL rows), `samples/milestat-redirect.html` |
| Pagination | **Browser-required for lists**: `https://va.milesplit.com/teams` returns 208,364 bytes but **0** `href="/teams/…"` links (team list loads via JS); `https://va.milesplit.com/meets` → **404** | `samples/teams-va-milesplit.html`, `samples/CAPTURES.md` (404 row) |
| Athlete fields | Not captured in this lane (no MileSplit results page sampled for VA). `Compare Athletes`, `Athletes` nav entries exist | `samples/vhsl-class5-state-meet-va-milesplit.html` |
| Meet fields | **Yes, and this is the VA high-value extract.** Verbatim text on meet `742518`: `VHSL Class 5 State Championships 2026` / `Jun 05, 2026` / `Jun 06, 2026` / `Todd Stadium` / `Newport News, VA` / `Hosted by VHSL` / `State` / **`Timing/Results Tidewater Timing Company`** / `View Live Results` / `Registration Closed - View Your Entries` / `Meet History` / `Records` / `Meet Venue` / `Meet Records` | `samples/vhsl-class5-state-meet-va-milesplit.html` |
| Grade/class evidence | Not measured for VA (grade is a MileSplit results-table property; not sampled here) | — |
| Coach/contact fields | None on the captured surfaces (`/contact` is robots-disallowed on the same host, and no coach tab was observed) | `../state-assoc-midatlantic/samples/robots-va-milesplit.txt` |
| Public API availability | None usable: `/api/` is explicitly disallowed by the host's robots.txt; the JS-driven team list implies an internal API that is off-limits | `../state-assoc-midatlantic/samples/robots-va-milesplit.txt` |
| Static file availability | Not observed for VA | — |
| Browser requirement | **Yes, for enumeration** (`/teams` is empty without JS); meet pages themselves are server-rendered | `samples/teams-va-milesplit.html` |
| Request cost | Milestone pages only: 1 request per meet page; enumeration of teams/meets needs a browser or an API that robots forbids | — |
| Published rate limits | No `Crawl-delay` in the host's robots.txt; VHSL itself publishes a blanket `Disallow: /` | both sibling-lane captures |
| Known blocks | **VHSL site is off-limits by its own robots policy** — this is the defining constraint for VA. Additionally `/rankings`, `/virtual-meets`, `/api/`, `/contact` disallowed on MileSplit VA; `www.milestat.com` HTTPS handshake fails; `/meets` 404 | as cited above |
| Cross-source join keys | `VHSL Class 5 State Championships` name ↔ venue identifier (`Todd Stadium`) ↔ MileSplit meet id; `Hosted by VHSL` is the association attribution field that ties a third-party meet record back to the sanctioning body | `samples/vhsl-class5-state-meet-va-milesplit.html` |
| Estimated marginal coverage | The association itself contributes **0 fetchable rows**. The third-party surface supplies the state meet (with a published timing provider) and an attributed `Hosted by VHSL` link; sanctioned timing provider observed: **Tidewater Timing Company** | `samples/vhsl-class5-state-meet-va-milesplit.html` |
| Implementation recommendation | **CONDITIONAL** — usable only through MileSplit VA/MileStat (and any Athletic.net coverage), never from `vhsl.org`. Any plan that assumes an association-side VA school universe or coach directory must be re-scoped | — |

### VA — Athletic.net linkage

`athletic.net` was **not observed** in the VA captures (`home-va-milesplit.html`, `vhsl-class5-state-meet-va-milesplit.html`: 0 matches). The VHSL→MileSplit linkage is expressed as `Hosted by VHSL` + `Timing/Results Tidewater Timing Company` on the meet page rather than as an HTML anchor, so it must be matched by meet name.

---

## WV — West Virginia Secondary School Activities Commission (WVSSAC)

Source name: **West Virginia Secondary School Activities Commission**, `https://www.wvssac.org/`. Platform: WordPress on the rschooltoday stack (static assets served from `assets-rst7.rschooltoday.com`, page JSON at `/wp-json/wp/v2/pages/8039`). **No robots.txt is published (404)** — verified: `samples/robots-wvssac.txt` contains the literal body `404 Not Found` (13 bytes).

| Field | Finding | Evidence |
|---|---|---|
| Geographic coverage | West Virginia only | `samples/home-wvssac.html` |
| Sports | T&F `/sports/track/`, XC `/sports/cross-country/` (nav lists Cheerleading, Cross Country, Football, Golf, Soccer, Volleyball, Basketball, Swimming, Wrestling, Baseball, Softball, Tennis, Track) | `samples/track-wvssac.html`, `samples/cross-country-wvssac.html` |
| Historical depth | Not measured — WVSSAC publishes administration documents per season (coaches packets/regional information), not result archives. WP search for `state track meet results` returns `[]` | `samples/wpjson-search-track-wvssac.json` (body: `[]`), `samples/track-wvssac.html` |
| Discovery mechanism | `/school-resources/classifications-regional-alignment/` (classification PDF) → `/sports/<sport>/` (season documents + linked news posts) → `/school-resources/programs/` (7 asset links; none for T&F/XC) | `samples/classifications-wvssac.html`, `samples/programs-wvssac.html` |
| Stable identifiers | None numeric per school observed. Keys are **class label** (`AAAA`, `AAA`, `AA`, `A`) + **rank within class** + **enrollment figure**; regional alignment is a separate PDF | `samples/classifications-2025-27-wvssac.pdf` |
| Pagination | Single-document PDFs (classifications, alignment, packets) — no pagination surface for schools | `samples/classifications-2025-27-wvssac.pdf` |
| Athlete fields | **None on this host.** No result artifact was found; WV result hosting is third-party (see linkage) | `samples/wpjson-search-track-wvssac.json` |
| Meet fields | Season-administration granularity only, e.g. `Track-and-Field-Regional-Information.pdf` (2026-09-18), `Track-and-Field-Coaches-Packet-2026.pdf`, `State-Track-Games-Committee-Report-2026.pdf`, `2026-Cross-Country-Coaches-Packet.pdf` | `samples/track-wvssac.html`, `samples/cross-country-wvssac.html` |
| Grade/class evidence | None (no result rows on this host) | — |
| Coach/contact fields | None found. `/school-resources/school-directory/` is an **iframe container**: the page body (WP REST `content.rendered`, 2,101 chars) is CSS for `.responsive-iframe-container` and the captured HTML contains `<iframe src="https://live.arbiter.io/org/4223" …>` — the directory itself is client-side. Coach surface is `/coaches-education/` (training, not a directory) | `samples/wpjson-page-8039-wvssac.json`, `samples/school-directory-wvssac.html` |
| Public API availability | **Yes — WordPress REST**: `/wp-json/wp/v2/pages/8039` returns the page object (200, JSON); `/wp-json/wp/v2/search?search=…` works but is nearly empty for our terms (`[]`) | `samples/wpjson-page-8039-wvssac.json`, `samples/wpjson-search-track-wvssac.json` |
| Static file availability | **Yes and this is the WV high-value extract**: `Classifications-25-27.pdf` (126,126 B) and `WVSSAC-Regional-Alignment-REVISED-8-14-26.pdf`, both robots-unrestricted (no robots.txt at all) | `samples/classifications-2025-27-wvssac.pdf`, `samples/classifications-wvssac.html` |
| Browser requirement | **Yes for the school directory** (iframe/JS); no for the classification PDFs | `samples/school-directory-wvssac.html` |
| Request cost | Member universe + enrollment: **1 request** (classification PDF). Season rules: 1 request per sport page. No per-school cost because there is no per-school surface | — |
| Published rate limits | **None published** (no robots.txt). Lane applied ≥1 s spacing anyway | `samples/robots-wvssac.txt` |
| Known blocks | No results and no coach directory on the association host; school-directory page is an iframe shell; `wvmetronews-track-championship-roundup` redirects off-site to `wvmetronews.com` (news article, 2026-05-23) | `samples/CAPTURES.md` (redirect row), `samples/school-directory-wvssac.html` |
| Cross-source join keys | School name + enrollment + class label ↔ third-party results school names; class label ↔ regional alignment PDF | `samples/classifications-2025-27-wvssac.pdf` |
| Estimated marginal coverage | **119 classified schools** parsed from `Classifications-25-27.pdf` ("WVSSAC CLASSIFICATIONS 2025-2026 to 2026-2027", 2 pages, four class columns): **AAAA 20 / AAA 28 / AA 32 / A 39**, each carrying an enrollment value in the range **7–1753** (e.g. `1 MORGANTOWN HIGH SCHOOL 1753`; the A list's lowest entry is `47 PICKENS ELEMENTARY/HIGH SCHOOL 7`). The class bands are stated in the document itself: `"AAAA" SCHOOLS (1050 or more)`, `"AAA" SCHOOLS (625 to 1049)`, `"AA" Schools (351 to 624)`, `"A" SCHOOLS (350 or Less)`. The A list's own numbering has gaps (26, 30, 38, 40, 41, 44, 45, 46 absent), i.e. the association's IDs are not contiguous | `samples/classifications-2025-27-wvssac.pdf` |
| Implementation recommendation | **PRIMARY for school universe/class/enrollment only**; **REJECT** for results, grades, and coach contacts; results must come from WV third-party hosts | — |

### WV — Athletic.net / MileSplit linkage

No `athletic.net` and no `milesplit.com` reference appears in any WVSSAC capture (`home-wvssac.html`, `school-directory-wvssac.html`, `track-wvssac.html`, `cross-country-wvssac.html`, `classifications-wvssac.html`, `programs-wvssac.html`). The only results-adjacent outbound link on the track page is a news roundup that leaves the association host:

```
https://www.wvssac.org/wvmetronews-track-championship-roundup/  →  301/200 →  https://wvmetronews.com/2026/05/23/boys-state-track-huntington-doddridge-defends-in-class-aa/
```

The WV XC result surface used by the pipeline is `runwv.com`, whose root page is already captured by sibling lane `timing-providers-national` (`../timing-providers-national/samples/runwv-root.html`) — cited here, not re-fetched.

---

## Lane-level synthesis

| Jurisdiction | Association enumeration (measured) | Coach/AD contacts | Grade in association artifacts | Association result artifacts | Best recommendation |
|---|---|---|---|---|---|
| FL | 880 schools (Home Campus widget, section 10) | **Yes** (detail API, per school) | No (Hy-Tek PDF is robots-blocked) | TFRRS (official) + Half Mile/PrimeTime (live) | PRIMARY + RESULT_SOURCE |
| GA | 457 (dropdown) / 456 (PDF) | **Yes, best in lane** (23,435 staff rows; Track 1,761, XC 946, AD 672) | Header only, values blank (0/211) | 2025-2026 T&F PDFs; 320 XC PDFs over 25 seasons | PRIMARY + RESULT_SOURCE |
| NC | 451 schools (+77 conference admins) | Conference-level only | **Yes** (146 rows, 44 grade-11) | 16 XC result PDFs/season, open robots | PRIMARY + RESULT_SOURCE |
| SC | 229 schools (classification list; 21 non-football) | None | None | None on host (points to MileSplit SC) | PRIMARY (universe) + DISCOVERY |
| VA | 0 (VHSL robots-disallows crawling) | None | Unknown | Third-party only (MileSplit VA, timed by `Tidewater Timing`) | CONDITIONAL |
| WV | 119 schools (+enrollment 7-1753) | None | None | None on host (third-party `runwv.com`) | PRIMARY (universe) / REJECT (rest) |

## Reproduction of every measured count

Each number in this report was re-derived from the sample files on disk (not from memory) with the following commands, run from `research/sources/state-assoc-southeast/samples/` on 2026-09-22:

| Count | Command |
|---|---|
| FL 880 widget schools | `grep -c 'id="school-button-' homecampus-school-directory.html` → `880` (and 880 distinct ids) |
| FL identifier example | `python3 -c "import json;d=json.load(open('homecampus-school-details-2397.json'));print(d['school']['fhsaa_school_id'],d['school']['hytek_code'],len(d['athleticFaculties']),len(d['coaches']))"` → `714 None 1 5` |
| FL TFRRS rows/classes | `python3 -` over `tfrrs-fl-list-5587-4a-region-1.html`: div-grid rows → 1,469 athlete rows, class labels `{SR:529, JR:487, SO:291, FR:167}`, 36 event sections (18 Men + 18 Women), 1,633 distinct `/athletes/<id>/` links |
| GA 457 schools in dropdown | `grep -o '<option' school-directory-ghsa.html \| wc -l` → `458` (incl. `Select a school...`); numeric-value options → 457 |
| GA 456 schools + class split | `pdftotext -layout ghsa-directory-feed.pdf - \| python3 -` (regex `([A-Z0-9][A-Z0-9 .,'&()/-]{2,60}?)\s*\((\d)-([A-Z]{1,7})\)`) → 456 distinct names, A 72 / AA 85 / AAA 62 / AAAA 60 / AAAAA 61 / AAAAAA 64 / AAAAAAA 52 |
| GA staff rows 23,435 / track 1,761 / XC 946 / AD 672 | `pdftotext -layout ghsa-directory-feed.pdf -` then per-line split on 2+ spaces, name+legend-code validation against the PDF's own key; restricted to pages 27-321 |
| GA T&F grade column empty | `pdftotext -layout ghsa-2026-girls-6a-track-results.pdf -` → 211 `Last, First` result rows, 0 with a grade token; 18 event sections, 12 pages |
| GA XC 320 PDFs / 25 seasons | `grep -o 'GHSA [0-9]\{4\}-[0-9]\{4\} State Cross Country' ghsa-xc-state-meet-results.html \| sort \| uniq -c` |
| NC 451 schools / 77 admins | `python3 -` counting `<tr>` rows on `schools-nchsaa.html` with ≥5 and =3 `<td>` cells |
| NC 146 rows / grade dist | `pdftotext -layout xc-2025-5a-girls-nchsaa.pdf - \| grep -E '^ *[0-9]+ .+ +(9\|10\|11\|12) '` |
| SC 229 schools | `python3 -` over `reclassification-2026-2028-schsl.html`: `Class (\dA)` / `Region \d+` sections → 5A 35, 4A 45, 3A 35, 2A 44, 1A 70 |
| WV 119 schools | `pdftotext -layout classifications-2025-27-wvssac.pdf -` then per-page column split at the header offsets (`"AAA"` / `"A"` column starts) → AAAA 20, AAA 28, AA 32, A 39 |

## Open questions (unverified in this lane)

1. ~~**FL grade source.**~~ **Resolved in-lane:** TFRRS (`florida.tfrrs.org`, robots comment-only) publishes athlete rows with `Year ∈ {FR,SO,JR,SR}`; a `JR` in a 2026-season list is the Class of 2027. Still open: numeric grade (9-12) for FL is only in the FHSAA-hosted Hy-Tek artifact at the robots-disallowed `/documents/` path, and whether TFRRS *XC* lists carry the same `Year` label was not checked (only T&F lists were sampled).
2. **FL membership cross-check.** 880 widget schools vs FHSAA's published "over 850 … combination/senior high schools and middles schools" — the widget mixes middle and senior high schools; the split by level was not measured (the widget exposes no level column).
3. **GA grade.** `Year` is a real column in GHSA state-meet PDFs but is empty in the 2026 girls 6A file. Whether other classes/gender files populate it, and whether `app.ghsa.net` exposes grade, is unverified.
4. **NC grade coverage.** Only one class/gender (5A girls XC 2025) was downloaded; per-class grade coverage across the other 15 files is unverified.
5. **SC `/wp-json`.** SCHSL is WordPress with a sitemap; its REST index was not probed, so whether T&F/XC results exist as custom post types is unknown.
6. **SC membership completeness.** The 232-school figure comes from the reclassification post, which explicitly annotates non-football schools; whether it equals the certified membership roll is unverified.
7. **VA enumeration.** Beyond the state-meet name pattern (`VHSL Class N State Championships`) and the single meet sampled, the VA team/meet universe is JS-loaded and was not enumerated; `athletic.net` VA coverage was not checked (out of lane).
8. **WV school directory.** The iframe at `live.arbiter.io/org/4223` was not driven; the directory's own fields (AD names, emails) are unverified.
9. **WV/VA sanctioned timing providers** beyond the observed `Tidewater Timing Company` (VA meet page) are unverified; `runwv.com` is cited from the timing lane, not re-verified here.
