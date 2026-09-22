# SOURCE_REPORT — state-assoc-midatlantic (12 jurisdictions: DC, DE, MD, NJ, NY, PA, CT, MA, RI, NH, VT, ME)

Lane directory: `research/sources/state-assoc-midatlantic/`
Captured: 2026-09-22 (UTC). Method: anonymous `urllib`/`curl`, chrome-like UA carrying an identifying
`athletic-census-research/1.0` token, redirects followed, TLS verification never disabled, ≥1.1 s per host between
requests, `robots.txt` fetched before content on every host, no authentication/CAPTCHA/paywall bypass.
Every fact below cites a file in `samples/` or a command recorded in `samples/CAPTURES.md`. Anything reasoned but
not printed by a source is marked `[INFERENCE]`.

Recommendation vocabulary used in field 21: **PRIMARY** = best single source of athlete-level results for that state;
**RESULT_SOURCE** = publishes results but not the complete/best set; **DISCOVERY_SOURCE** = enumerates the sanctioned
universe or meets to seed crawling; **VALIDATION_SOURCE** = corroborates identities/joins but does not carry primary
results; **COACH_SOURCE** = publishes coach/AD contact data; **CONDITIONAL** = usable only under the stated condition;
**REJECT** = do not use.

---

## Global findings

1. **MileSplit regional hosts are dead; per-state hosts serve everything.** Re-verified live in this lane:
   `GET https://newengland.milesplit.com/teams` → **500**, `GET https://dcmdva.milesplit.com/teams` → **500**
   (`samples/newengland-milesplit-teams.html`, `samples/dcmdva-milesplit-teams.html`, both 0 bytes, 2026-09-22T03:59:46Z
   and 03:59:47Z). Every one of the 12 states is served by its own `<lowercase USPS code>.milesplit.com` host, each
   returning 200 with state-specific team lists (measured team-link counts in `coverage.json` →
   `cross_source.milesplit_state_hosts`; per-state files `samples/<code>-milesplit-teams.html`). The New England
   *championship* is not a MileSplit site at all — it is published as a PDF packet by each member association:
   "2026-27 CNESSPA Cross Country New England Championship Packet" (`samples/nhiaa-boys-xc.html`,
   `samples/riil-xc-boys.aspx`).
2. **Athletic.net linkage exists in exactly one of the twelve jurisdictions.** MIAA (MA) states a formal partnership
   in its own markup (quoted in §MA field 19). PA links MileSplit/PenntrackXC from its championship page, RI and ME
   publish MileSplit "Partner" links, NJ uses MileSplit for entry windows, NY carries MileSplit only as a paid ad
   campaign. MD, DC, CT, VT, NH, DE: no Athletic.net or MileSplit link found in any captured page.
3. **One shared association platform across three states.** `ciac.fpsports.org` (CT), `www.mpa.cc` (ME) and
   `www.riil.org` (RI) serve byte-identical `robots.txt` (`md5 901da3fd0f5f4be7f447ace46cdab356`) and the same path
   grammar (`/SchoolPages/School.aspx`, `/SportPages/SportPageInfo.aspx?TournamentID=`, `/MasterSchedule.aspx`,
   `/DashboardTeamSchedule.aspx`, `cdn.fpsports.org` assets). Tournament ids are shared across the three
   (1 = Cross Country Boys, 104 = Indoor Track, 204 = Outdoor Track). A second shared platform, rSchoolToday,
   backs NHIAA and DCSAA (`assets-rst7.rschooltoday.com`).
4. **Two domain traps in the DC space.** `www.dcsaa.org` is a South-Florida school-administrators' association
   ("DCSAA members have exclusive access to service from one of the top labor firms in South Florida",
   `samples/dcsaa-home.html`); `www.dcsaasports.org` is a Thai-language online-gambling page
   (`samples/dcsaa-home.html`, outbound `chaiyo88.net`/`chaiyo88.casino`). The genuine association is
   `https://www.dcsaasports.com/` (title "The District of Columbia State Athletic Association").
5. **Two hard availability blocks.** CIAC's results host `www.ciacsports.com` fails TLS verification
   (`Verify return code: 10 (certificate has expired)`, chain anchored at `DST Root CA X3`); all Delaware DIAA
   content paths sit behind a Cloudflare interstitial (`403`, `Just a moment...`). Neither was bypassed.
6. **Adjacent captures present in this directory, not researched here:** `samples/robots-vhsl.txt` (Virginia VHSL —
   `User-agent: * / Disallow: /`, i.e. the whole host is off-limits for anonymous crawling) and
   `samples/robots-va-milesplit.txt` (VA MileSplit — disallows `/rankings`, `/virtual-meets`, `/api/`, `/contact`).
   VA is outside this lane's twelve jurisdictions; recorded only so the owning lane can cite the bytes.

---

## DC — District of Columbia State Athletic Association (DCSAA)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | District of Columbia State Athletic Association (DCSAA), `https://www.dcsaasports.com/` (`samples/dcsaasports-com-home.html`, 200, 744 340 B; title "The District of Columbia State Athletic Association"). |
| 2 | **Geographic coverage** | Washington, DC only (public, charter, private and independent schools that opt into DCSAA membership). |
| 3 | **Sports** | Fall/winter/spring slate; XC pages verified at `/boys-cross-country/` and `/girls-cross-country/`, track at `/sports/indoor-track-field/` and `/sports/outdoor-track-field/` (`samples/dcsaasports-com-home.html`, `samples/dcsaasports-otf.html`). |
| 4 | **Historical depth** | Current site lists the **2025** DCSAA Cross Country State Championship Meet (`samples/dcsaasports-boys-xc.html`). No archive of earlier seasons observed. |
| 5 | **Discovery mechanism** | WordPress + Directorist school directory at `/school-directory/`, 45 distinct school slugs (`samples/dcsaasports-school-directory.html`). |
| 6 | **Stable identifiers** | School slug (`https://www.dcsaasports.com/school-directory/<slug>/`, e.g. `anacostia-high-school`); sport page paths (`/sports/outdoor-track-field/`). No numeric school id exposed. |
| 7 | **Pagination** | None observed on the school directory (single response, no `page/N` links in the markup). |
| 8 | **Athlete fields** | None. No athlete names or marks observed on DCSAA pages. |
| 9 | **Meet fields** | Meet name + season ("2025 DCSAA Cross Country State Championship Meet"), listed venue (St. John's College High School referenced as team winner) (`samples/dcsaasports-boys-xc.html`). Full meet calendar with dates not located. |
| 10 | **Result fields** | Competition level only: team winner and "MVP School" (`samples/dcsaasports-boys-xc.html`). No times/marks. |
| 11 | **Grade/class evidence** | None observed. |
| 12 | **Coach/contact fields** | None public. |
| 13 | **Public API availability** | None observed. Platform is WordPress/Directorist with `wp-json` present but not used here. |
| 14 | **Static file availability** | PDFs/forms exist ("DCSAA Forms", "DCSAA 2026 Fall Sports Bulletins" headlines in `samples/dcsaasports-com-home.html`); none fetched. |
| 15 | **Browser requirement** | No — all captured pages returned complete server-rendered HTML. |
| 16 | **Request cost** | Low: 1 request for the directory, 1 per sport page. |
| 17 | **Published rate limits** | None — `robots.txt` returns **404** (`samples/robots-dcsaasports.txt`, 13 B) and no crawl-delay is published. |
| 18 | **Known blocks** | None for `dcsaasports.com`. The look-alike hosts are the hazard (see Global finding 4). |
| 19 | **Cross-source join keys** | School name (slug ⇄ MileSplit team name, e.g. `dc.milesplit.com/teams/…-anacostia-high-school` in `samples/dc-milesplit-teams.html`); meet name for DCSAA championship events. |
| 20 | **Estimated marginal coverage** | Small but complete for DC's sanctioned universe (45 directory schools, 80 MileSplit DC team entries measured). Adds: DC membership list, DC championship winners. Does not add athlete-level results. |
| 21 | **Implementation recommendation** | **DISCOVERY_SOURCE** (DC school universe + championship validation; pair with `dc.milesplit.com` for results). |

---

## DE — Delaware Interscholastic Athletic Association (DIAA)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | Delaware Interscholastic Athletic Association. `https://www.diaa.org/` answers **302** with `location: https://education.delaware.gov/diaa` (`samples/CAPTURES.md`, command-only evidence). The "DIAA" expansion is `[INFERENCE]` from the URL path — **no DIAA page body was retrievable**. |
| 2 | **Geographic coverage** | Delaware statewide (not verifiable from a page body — inferred from the state DOE host). |
| 3 | **Sports** | Unverified (blocked). |
| 4 | **Historical depth** | Unverified (blocked). |
| 5 | **Discovery mechanism** | None available on the association host: `/diaa`, `/diaa/` and `/` on `education.delaware.gov` all return **403** with the Cloudflare `Just a moment...` interstitial (`samples/diaa-home.html`, 5 702 B; curl re-run 5 659/5 662/5 626 B). |
| 6 | **Stable identifiers** | Only via the state MileSplit host: `https://de.milesplit.com/teams/<id>-<slug>` (e.g. `alexis-i-dupont-high-school`, `appoquinimink-high-school`; `samples/de-milesplit-teams.html`). |
| 7 | **Pagination** | n/a (blocked). |
| 8 | **Athlete fields** | None captured. |
| 9 | **Meet fields** | None captured. |
| 10 | **Result fields** | None captured. |
| 11 | **Grade/class evidence** | None captured. |
| 12 | **Coach/contact fields** | None captured. |
| 13 | **Public API availability** | Not established. |
| 14 | **Static file availability** | Not established; the DOE site is WordPress (`wp-sitemap.xml` declared in `samples/robots-education-delaware.txt`) but sitemap fetches were not attempted through the interstitial. |
| 15 | **Browser requirement** | **Yes** — a Cloudflare JS challenge guards content paths (a real browser is required; I did not attempt to solve it). |
| 16 | **Request cost** | Low for the 403s; unusable for data. |
| 17 | **Published rate limits** | `education.delaware.gov/robots.txt` (`samples/robots-education-delaware.txt`): `Disallow: /wp-admin/`, `Allow: /wp-admin/admin-ajax.php`, **`Crawl-delay: 10`**. `www.diaa.org/robots.txt` → **404**, 0 B (`samples/robots-diaa.txt`). |
| 18 | **Known blocks** | Cloudflare bot challenge on all content paths (403); `www.doe.k12.de.us/robots.txt` also 403s and redirects to the same host. `regulations.delaware.gov` returns a 65 540-byte HTML shell for both `/robots.txt` and `/AdminCode/title14/` (soft-404) — no valid robots file obtained, and no "Interscholastic" mention in that shell. |
| 19 | **Cross-source join keys** | School name via `de.milesplit.com` team slugs (82 measured). |
| 20 | **Estimated marginal coverage** | Currently **zero** from the association; DE coverage must come from `de.milesplit.com` and (unverified) DIAA documents such as the state administrative code. |
| 21 | **Implementation recommendation** | **CONDITIONAL** — only if a non-challenged DIAA host or an authorised mirror is found; otherwise use `de.milesplit.com` as the DE result source. |

---

## MD — Maryland Public Secondary Schools Athletic Association (MPSSAA)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | MPSSAA, `https://www.mpssaa.org/` (WordPress + Rank Math). Site identity captured via the school directory page (200, 528 894 B, `samples/mpssaa-school-directory.html`); the homepage bytes were captured incidentally as the effective URL of `samples/mpssaa-fall-championships.html` (requested `/state-championships/fall-championships/`, effective `https://www.mpssaa.org/`, 246 138 B). |
| 2 | **Geographic coverage** | Maryland statewide (24 jurisdictions; directory includes county aggregate pages). |
| 3 | **Sports** | Fall: Cross Country, Field Hockey, Flag Football, Football, Golf, Soccer, Volleyball; Winter: Basketball, Indoor Track & Field, Swimming & Diving, Wrestling; Spring: Baseball, Lacrosse, Softball, Tennis, Track and Field (`samples/mpssaa-cross-country.html` nav). |
| 4 | **Historical depth** | 2025 season championship results are published (`/2026/cross-country/2025-championship-results/`), plus per-sport historical result posts (2025 golf, field hockey, football, soccer, volleyball in `samples/sitemap-mpssaa-page.xml`). Older seasons not enumerated. |
| 5 | **Discovery mechanism** | `/school-directory/<slug>/` (201 slugs; 9 are county aggregates → **192 school pages**), plus `sitemap_index.xml` with 7 sub-sitemaps including `school-sitemap.xml` (200 `<loc>`) and `sport-sitemap1/2.xml`. |
| 6 | **Stable identifiers** | Directory slug (`aberdeen`), result page path (`/2026/cross-country/2025-championship-results/`), region schedule paths. |
| 7 | **Pagination** | Directory is a single index; no pagination observed. |
| 8 | **Athlete fields** | Individual champion names + times appear in championship summaries — e.g. `Champion Simon McGillivray 16:28.02` (`samples/mpssaa-cross-country.html`). Full result lists live on the per-season result pages. |
| 9 | **Meet fields** | Date, venue, class, region entry/qualifying deadlines, region meet schedule, state committee & region meet directors (`samples/mpssaa-cross-country.html`, `samples/mpssaa-xc-region-schedule.html`). |
| 10 | **Result fields** | Class (1A/2A/3A/4A), team champion + points, team runner-up + points, individual champion + time: `CLASS 1A Team Champion Smithsburg 49 … CLASS 4A Team Champion Urbana 53 … Team Runner-up Brunswick 83 / Hereford 110 / Sherwood 84 / Montgomery Blair 81` (`samples/mpssaa-cross-country.html`). |
| 11 | **Grade/class evidence** | Competition class (1A–4A) is the classification axis; no per-athlete grade observed. |
| 12 | **Coach/contact fields** | Not public: `/members/` and `/members/athletic-director-email-directory/` both redirect to `https://www.mpssaa.org/login/?redirect_to=…` (`samples/mpssaa-members.html`, `samples/mpssaa-ad-directory.html`). |
| 13 | **Public API availability** | None observed. Schedule/result widgets render client-side and the captured static HTML shows `0 results found` / `No results found` (`samples/mpssaa-xc-region-schedule.html`). |
| 14 | **Static file availability** | Yes — PDFs (bulletins, handbook, region meet files) are linked from the site; `robots.txt` does not block them. |
| 15 | **Browser requirement** | No for directory/championship pages; **yes** for the live schedule/results widgets. |
| 16 | **Request cost** | Low: 1 request per directory/result page; 2 result pages cover the 2025 XC state meet. |
| 17 | **Published rate limits** | None published — `robots.txt` (`samples/robots-mpssaa.txt`, 352 B) only disallows `/wp-admin/`. |
| 18 | **Known blocks** | Login-gated `/members/` area only. |
| 19 | **Cross-source join keys** | School name (directory slug ⇄ MileSplit `md.milesplit.com/teams/<id>-<slug>`, 401 measured); class (1A–4A) maps to MileSplit division labels; meet name for state championships. |
| 20 | **Estimated marginal coverage** | High for MD: sanctioned school universe (192 schools) **plus athlete-level state championship results** fresh each season. |
| 21 | **Implementation recommendation** | **RESULT_SOURCE** (and the discovery authority for MD's school list). |

---

## NJ — New Jersey State Interscholastic Athletic Association (NJSIAA)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | NJSIAA, `https://www.njsiaa.org/` (Drupal; `samples/njsiaa-home.html`, 189 279 B). |
| 2 | **Geographic coverage** | New Jersey statewide (public and non-public school sections). |
| 3 | **Sports** | 24 sport paths on `/sports` (`samples/njsiaa-sports.html`), including `/sports/cross-country`, `/sports/track-field-indoor`, `/sports/track-field-outdoor`. |
| 4 | **Historical depth** | News/archive depth not measured; sport pages are current-season oriented. |
| 5 | **Discovery mechanism** | `/schools/member-information` — a school table with AD contact fields; `/schools/league-conference`, `/schools/classifications-tournaments`, `/schools/cooperative-sports-programs` are the classification/league entry points (`samples/njsiaa-schools.html`). |
| 6 | **Stable identifiers** | Sport path (`/sports/track-field-outdoor`); school rows carry school name + address (no numeric id in the markup). |
| 7 | **Pagination** | The member table exposes `?page=1` in the captured markup; further pages were not enumerated, so 47 rows is a **page-one count, not a total**. |
| 8 | **Athlete fields** | None observed. |
| 9 | **Meet fields** | Championship structure ("PUBLIC SECTIONAL CHAMPIONSHIPS", "STATE CHAMPIONSHIPS") plus dated entry windows (`samples/njsiaa-tf-outdoor.html`). |
| 10 | **Result fields** | None on the association host; score reporting is delegated — `<a href="https://njschoolsports.com">Report Score</a>` (`samples/njsiaa-xc.html`). |
| 11 | **Grade/class evidence** | Non-public/public split drives separate entry deadlines ("MileSplit Closes (Public Schools)" / "(Non-Public Schools)", `samples/njsiaa-tf-outdoor.html`). |
| 12 | **Coach/contact fields** | **Public**: columns School / Title / Address / Phone / **Ath. Dir.** — e.g. `Abraham Clark High School | 122 East 6th Avenue | Roselle, NJ 07203 | 908-298-2022 | Dr. Edwin Griffin` (`samples/njsiaa-member-info.html`). This is the strongest coach/AD field set in the lane. |
| 13 | **Public API availability** | None observed (Drupal JSON:API not probed). |
| 14 | **Static file availability** | Yes — forms/regulations live under `/sites/default/files/…` (e.g. sponsor logo `…/2019-12/mf-athletic-logo-16-17.jpg`, `samples/njsiaa-xc.html`). |
| 15 | **Browser requirement** | No for the member table and sport pages. |
| 16 | **Request cost** | Low–moderate: the member table is paginated, so the full universe costs (pages × 1 request). |
| 17 | **Published rate limits** | None published (Drupal default robots; `samples/robots-njsiaa.txt`, 2 183 B). |
| 18 | **Known blocks** | Only the stock Drupal disallows (`/core/`, `/profiles/`, `/admin/`, `/search/`, `/user/login`, `/media/oembed`). |
| 19 | **Cross-source join keys** | School name + town/zip (strong for joining to MileSplit `nj.milesplit.com/teams/<id>-<slug>`, 549 measured); public/non-public status joins to section structure. |
| 20 | **Estimated marginal coverage** | High for **contact/classification** data (AD names, addresses, league/section channels) and for entry-deadline calendars; none for results. |
| 21 | **Implementation recommendation** | **DISCOVERY_SOURCE** (with COACH_SOURCE-grade AD fields; NJ results come from MileSplit). |

---

## NY — NYSPHSAA (upstate) + PSAL (New York City)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | New York State Public High School Athletic Association, `https://www.nysphsaa.org/` (Sidearm Sports), and Public Schools Athletic League, `https://www.psal.org/` (ASP.NET). NYSPHSAA excludes NYC; PSAL is NYC-only — the two are disjoint authorities. |
| 2 | **Geographic coverage** | NYSPHSAA: NY minus NYC (section-based). PSAL: the five boroughs. |
| 3 | **Sports** | NYSPHSAA exposes 30 sport modules (`samples/nysphsaa-home.html` nav JSON), including `/index.aspx?path=cross`, `?path=itrack`, `?path=otrack`. PSAL exposes 36 sport codes, including `033` Cross Country, `035` Indoor Track, `030` Outdoor Track (`samples/psal-home.html`). |
| 4 | **Historical depth** | NYSPHSAA nav advertises a "7 Year Calendar" and "2026-27 Championship Schedule" (`samples/nysphsaa-schools-path.html`); no historical results archive observed. |
| 5 | **Discovery mechanism** | Sports modules per association; **no member-school directory found**: `https://www.nysphsaa.org/schools` → 404 (`samples/nysphsaa-schools.html`) and `/index.aspx?path=schools` is an administrative hub without a school list (`samples/nysphsaa-schools-path.html`). PSAL school lists surface per-sport (11 school-like entries on the XC page). |
| 6 | **Stable identifiers** | NYSPHSAA `path=<sport-slug>`; PSAL `spCode` (e.g. `?spCode=033&flag=All`). |
| 7 | **Pagination** | Not applicable for captures; PSAL sport pages list participating schools in one response. |
| 8 | **Athlete fields** | None on either association host. |
| 9 | **Meet fields** | NYSPHSAA championship calendar ("2026-27 Championship Schedule"); PSAL sport pages carry season schedules and program notes. |
| 10 | **Result fields** | None on the association hosts. PSAL publishes results as Google documents: `https://drive.google.com/file/d/15uGBx0vuDiRgOc3DsgdbzPzQBtnbVG3b/view` and a spreadsheet `https://docs.google.com/spreadsheets/d/11F7f0HVBlmwpKOsiBPWg6I7Ba6GGq6z70KN61HCuVHw/edit?gid=0#gid=0` (`samples/psal-xc.html`). |
| 11 | **Grade/class evidence** | Not observed (NY uses section/class structures; not captured). |
| 12 | **Coach/contact fields** | NYSPHSAA advertises an "ADS & Coaches" area in nav JSON (`samples/nysphsaa-schools-path.html`) but no public roster was captured. |
| 13 | **Public API availability** | None observed (Sidearm `.axd` handlers exist — `/common/controls/adhandler.aspx` — but are ad plumbing). |
| 14 | **Static file availability** | NYSPHSAA keeps PDFs under `/documents/`, which its own `robots.txt` disallows → not fetched. |
| 15 | **Browser requirement** | No for the captured pages; NYSPHSAA nav is server-rendered JSON inside the HTML. |
| 16 | **Request cost** | Low. |
| 17 | **Published rate limits** | **Yes, explicit.** `samples/robots-nysphsaa.txt` (5 489 B): for `User-agent: *` — `Crawl-delay: 5`, `Visit-time: 0100-0645`, with `/images/`, `/documents/`, `/admin/`, `/services/`, `/site/`, `*.js`, `*.css`, `*.jpg`, `*.gif`, `*.axd`, `/*print=true*` disallowed. PSAL publishes **no robots.txt** — `/robots.txt` returns the site's own 404 HTML page (40 245 B, `samples/robots-psal.txt`). |
| 18 | **Known blocks** | NYSPHSAA `/documents/` is robots-disallowed (handbooks live there); `/sitemap.aspx` → 500 → `/sorry.ashx` (`samples/CAPTURES.md`). |
| 19 | **Cross-source join keys** | Section/class (NYSPHSAA) vs PSAL borough/division; school names for joining to `ny.milesplit.com` (1 357 team links measured). |
| 20 | **Estimated marginal coverage** | Low for results; useful for **validation** (sport calendars, championship schedule, PSAL document index). |
| 21 | **Implementation recommendation** | **VALIDATION_SOURCE** for NYSPHSAA; **DISCOVERY_SOURCE** for PSAL's per-sport school lists. |

---

## PA — Pennsylvania Interscholastic Athletic Association (PIAA)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | PIAA, `https://www.piaa.org/` (ASP.NET; `samples/piaa-home.html`, 60 903 B). |
| 2 | **Geographic coverage** | Pennsylvania statewide, organised into districts and regions. |
| 3 | **Sports** | `/sitemap.aspx` lists 50 sports paths (`samples/piaa-sitemap.aspx.html`), including `/sports/crscountry/championships/default.aspx` and the Track & Field championship entry. |
| 4 | **Historical depth** | Championship pages are per-season (page title dated "2026 PIAA Cross Country Championships Information and Results"). Older seasons not enumerated. |
| 5 | **Discovery mechanism** | `/schools/directory/list.aspx?alpha=<letter>` (letters A–W and Y observed; 25 letters) + `/schools/membership/default.aspx` + `/schools/classifications/default.aspx` (`samples/piaa-sitemap.aspx.html`). |
| 6 | **Stable identifiers** | `alpha` directory letters; `sport=` championship keys (`?sport=crscountry`); district pages `/schools/directory/district.aspx`. |
| 7 | **Pagination** | Alphabetical (per-letter) rather than numeric; the alpha page renders **no school anchors** in static HTML, so per-letter rows are `[INFERENCE]` client-rendered and were not counted. |
| 8 | **Athlete fields** | None observed. |
| 9 | **Meet fields** | Championship event pages: dates/venues per sport via `/championships/default.aspx` and `/events/default.aspx?categoryCodes=…piaa_championships`. |
| 10 | **Result fields** | None in static HTML; PIAA delegates championship coverage: `<li><a href="https://pa.milesplit.com/" target="_blank">Access District and PIAA Championships coverage from PenntrackXC.com</a></li>` (`samples/piaa-championship-details-xc.html`). |
| 11 | **Grade/class evidence** | Classifications exist (`/schools/classifications/default.aspx`, `/schools/classifications/requirements.aspx`) but no class table was captured. |
| 12 | **Coach/contact fields** | Not public for officials — `/officials/directory/` is robots-disallowed (`samples/robots-piaa.txt`). |
| 13 | **Public API availability** | None observed; `.axd` handlers are robots-disallowed via `Disallow: /*.axd`. |
| 14 | **Static file availability** | Yes — PDFs under `/assets/web/…` (e.g. `2026_PIAA_Calendar.pdf`, handbook introduction) although `/assets/` is robots-disallowed for crawlers. |
| 15 | **Browser requirement** | Yes for the school directory (client-rendered list); no for championship pages. |
| 16 | **Request cost** | Low per page; 25 requests for the full alphabetical directory. |
| 17 | **Published rate limits** | None published (`samples/robots-piaa.txt`, 593 B: path disallows plus MJ12bot/SemrushBot bans). |
| 18 | **Known blocks** | `/officials/directory/`, `/assets/`, `/account/`, `/admin/`, `/shop`, `/maint/` are robots-disallowed. |
| 19 | **Cross-source join keys** | School name → `pa.milesplit.com/teams/<id>-<slug>` (918 measured); PIAA district → MileSplit district labels. |
| 20 | **Estimated marginal coverage** | Moderate: championship calendar and district/classification scaffolding; results themselves arrive through pa.milesplit.com. |
| 21 | **Implementation recommendation** | **VALIDATION_SOURCE** (PIAA's own pointer to `pa.milesplit.com` is the actionable part). |

---

## CT — Connecticut Interscholastic Athletic Conference (CIAC)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | CIAC. Live platform: `https://ciac.fpsports.org/` (200, 76 502 B, `samples/ciac-fpsports-home.html`), reached from the `casciac.org` splash image-map entry `alt="CIAC Website"` (`samples/casciac-home.html`). The legacy host `https://www.ciacsports.com/` **fails TLS verification**. |
| 2 | **Geographic coverage** | Connecticut statewide (CIAC member schools; separate boys/girls divisions plus an Open championship). |
| 3 | **Sports** | Season sport list with tournament ids, including Cross Country (Boys `1`, Girls `9`), Indoor Track (B&G `104`), Outdoor Track (B&G `204`) (`samples/ciac-school-list.aspx`). |
| 4 | **Historical depth** | Nav exposes "Historic Schedules" (`samples/ciac-xc-boys.aspx`); depth not measured. |
| 5 | **Discovery mechanism** | Vendor school list at `/SchoolPages/School.aspx`: **440 distinct `SchoolID` values** (`samples/ciac-school-list.aspx`). `casciac.org/memberschools/` exists but is robots-disallowed. |
| 6 | **Stable identifiers** | `SchoolID` (e.g. `SchoolID=6 → A.I. Prince Technical High School`), `TournamentID` per sport. |
| 7 | **Pagination** | None observed on the school list (single response). |
| 8 | **Athlete fields** | None observed on CIAC pages. |
| 9 | **Meet fields** | Championship blocks carry date + venue: "Divisional Championships, October 31 at Wickham Park; Open Championships, November 6 at Wickham Park (Weather date - Nov. 9)" (`samples/ciac-xc-boys.aspx`). |
| 10 | **Result fields** | None retrievable: results live on `www.ciacsports.com`, whose certificate expired. |
| 11 | **Grade/class evidence** | Classes/divisions implied by "Divisional Championships" vs "Open Championships" (`samples/ciac-xc-boys.aspx`). |
| 12 | **Coach/contact fields** | `/CommitteeDirectory.aspx` exists; contents not captured. |
| 13 | **Public API availability** | None observed. |
| 14 | **Static file availability** | Yes (PDFs/forms) but not enumerated in this pass. |
| 15 | **Browser requirement** | No for `ciac.fpsports.org`; **unreachable** for `ciacsports.com` regardless of browser due to the expired certificate. |
| 16 | **Request cost** | Low. |
| 17 | **Published rate limits** | None published; `samples/ciac-fpsports-robots.txt` (247 B) disallows the six dashboard paths (see field 18). `samples/robots-casciac.txt` (1 343 B) disallows `/memberschools/`, `/officials/`, `/data_files/`, `/xml/` and more. |
| 18 | **Known blocks** | (a) `www.ciacsports.com` TLS: `verify error:num=10:certificate has expired`, `Verify return code: 10`, leaf `CN=ciacsports.com` ← Let's Encrypt R3, chain anchored at `DST Root CA X3`; curl exit 60 / `ssl_verify_result 20`; plain HTTP 302. Verification was **not** disabled. (b) robots-disallowed dashboards: `/DashboardSchedule.aspx`, `/DashboardGame.aspx`, `/DashboardSchool.aspx`, `/DashboardTeamSchedule.aspx`, `/ScheduleByWeek.aspx`, `/WebBot.aspx`. |
| 19 | **Cross-source join keys** | `SchoolID` ↔ school name ↔ `ct.milesplit.com/teams/<id>-<slug>` (251 measured); `TournamentID` ↔ sport name. |
| 20 | **Estimated marginal coverage** | Moderate: CT membership (440 vendor school ids, levels mixed), championship dates/venues, shared-platform structure. No results until the results host is fixed. |
| 21 | **Implementation recommendation** | **CONDITIONAL** — usable for enumeration and championship metadata today; results blocked by the expired certificate. |

---

## MA — Massachusetts Interscholastic Athletic Association (MIAA)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | MIAA, `https://www.miaa.net/` (Drupal; `samples/miaa-home.html`, 125 645 B). |
| 2 | **Geographic coverage** | Massachusetts statewide, nine MIAA districts (district counts measured, see field 5). |
| 3 | **Sports** | Full season slate; track and XC hub at `/track-cross-country`, plus `/unified-track-field`, `/scores`, `/tournaments`, `/tournament-formats`. |
| 4 | **Historical depth** | Event archive present in `sitemap.xml` (904 URLs incl. dated committee meetings back to 2020); member list PDF is dated **March 27, 2026**. |
| 5 | **Discovery mechanism** | Member school list PDF: `https://www.miaa.net/media/824` → `…/sites/default/files/2024-05/miaa-member-school-list.pdf` (123 825 B). **381 schools** measured via `grep -c 'MIAA District:'` on `pdftotext -layout` output: School Level H 260 / MH 97 / EMH 24; Private 41; Voc 35; Charter 36; district spread 62/36/32/37/45/60/35/33/41 for districts 1–9. |
| 6 | **Stable identifiers** | School name + address in the PDF (no id); sport paths; `/media/<n>` document ids. |
| 7 | **Pagination** | n/a — one PDF covers the universe. |
| 8 | **Athlete fields** | None on the association host; rankings are published as site files derived from Athletic.net data (see field 10/19). |
| 9 | **Meet fields** | Tournament formats page + sport committee pages (`/about-miaa/events/track-and-cross-country-committee`); association calendar `/about-miaa/association-calendar`. |
| 10 | **Result fields** | None on the association host; scores/schedules delegate to ArbiterLive (`https://www.arbiterlive.com/`, `samples/miaa-scores.html`). |
| 11 | **Grade/class evidence** | `School Level` codes H / MH / EMH in the member PDF; MIAA districts 1–9 (`samples/miaa-school-list.pdf`). |
| 12 | **Coach/contact fields** | None in the PDF (school + address + classification only). |
| 13 | **Public API availability** | None observed. |
| 14 | **Static file availability** | Yes — `/sites/default/files/…` PDFs and ranking sheets (`2026-01/mstca-boys-indoor-track-rankings-…pdf`, `samples/miaa-track-xc.html`). |
| 15 | **Browser requirement** | No. |
| 16 | **Request cost** | Very low: 1 PDF + 1 sport page. |
| 17 | **Published rate limits** | None published (`samples/robots-miaa.txt`, 2 027 B, Drupal defaults). |
| 18 | **Known blocks** | None relevant. |
| 19 | **Cross-source join keys** | School name/town/zip ↔ `ma.milesplit.com/teams/<id>-<slug>` (466 measured); **Athletic.net linkage is explicit** — `samples/miaa-track-xc.html`: `<div class="text-long"><h3>Resources</h3><p><a href="https://www.athletic.net/">Athletic.Net&nbsp;</a> The MIAA has partnered with Athletic.Net for all on-line entries in Cross Country, Indoor, and Outdoor Track.</p>` and `<h3>New: MSTCA Indoor Track rankings</h3><p>These rankings were created by a committee of the Mass State Track Coaches' Association, using data supplied by athletic.net.</p>`. |
| 20 | **Estimated marginal coverage** | **High** for MA: the full sanctioned member universe (381 schools with level/private/voc/charter flags) and the authoritative statement that Athletic.net is the entry system for XC/indoor/outdoor track. |
| 21 | **Implementation recommendation** | **DISCOVERY_SOURCE** (member universe + Athletic.net routing proof); results arrive via Athletic.net/ArbiterLive. |

---

## RI — Rhode Island Interscholastic League (RIIL)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | RIIL, `https://www.riil.org/` (shared ASP.NET vendor platform; `samples/riil-home.html`, 108 860 B). |
| 2 | **Geographic coverage** | Rhode Island statewide. |
| 3 | **Sports** | Sport list with tournament ids incl. Cross Country Boys `1` / Girls `310`, Indoor Boys `104` / Girls `1024`, Outdoor Boys `204` / Girls `1021` (`samples/riil-school-list.aspx`). |
| 4 | **Historical depth** | Current season (Fall 2026) championship dates plus a "Historic Schedules" entry in nav (`samples/riil-xc-boys.aspx`). |
| 5 | **Discovery mechanism** | `/SchoolPages/School.aspx` → **128 distinct `SchoolID`** values (levels mixed — schools named include "…High School" plus middle-school entries) (`samples/riil-school-list.aspx`). |
| 6 | **Stable identifiers** | `SchoolID` (e.g. `SchoolID=1 → Achievement First (PVD) High School`), `TournamentID`. |
| 7 | **Pagination** | None observed. |
| 8 | **Athlete fields** | None observed. |
| 9 | **Meet fields** | Season structure "Dual Meet #1…#4", "Class Championships", "RI State Championships", "New England Championships" (`samples/riil-xc-boys.aspx`). |
| 10 | **Result fields** | Not on riil.org — results are published on MileSplit: resource link labelled "RIIL Cross Country on MileSplit" → `https://ri.milesplit.com/` (`samples/riil-xc-boys.aspx`). |
| 11 | **Grade/class evidence** | Class championships are the classification axis (`samples/riil-xc-boys.aspx`); no grade data. |
| 12 | **Coach/contact fields** | None public. |
| 13 | **Public API availability** | None observed. |
| 14 | **Static file availability** | Yes — e.g. the CNESSPA "New England XC Championship Packet 2026" PDF linked from the XC page. |
| 15 | **Browser requirement** | No. |
| 16 | **Request cost** | Low. |
| 17 | **Published rate limits** | None published; `samples/robots-riil.txt` (247 B) carries the six dashboard disallows. |
| 18 | **Known blocks** | robots-disallowed: `/DashboardSchedule.aspx`, `/DashboardGame.aspx`, `/DashboardSchool.aspx`, `/DashboardTeamSchedule.aspx`, `/ScheduleByWeek.aspx`, `/WebBot.aspx` — i.e. **meet schedules and scores are intentionally off-limits**; use MileSplit instead. |
| 19 | **Cross-source join keys** | `SchoolID` ↔ school name ↔ `ri.milesplit.com` team (72 measured); `TournamentID` ↔ sport. |
| 20 | **Estimated marginal coverage** | Moderate: RI school list, sport/championship scaffolding, and RIIL's own published pointer to MileSplit as the results host. |
| 21 | **Implementation recommendation** | **DISCOVERY_SOURCE** (results via `ri.milesplit.com`, as RIIL itself states). |

---

## NH — New Hampshire Interscholastic Athletic Association (NHIAA)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | NHIAA, `https://www.nhiaa.org/` (WordPress front end over rSchoolToday; `samples/nhiaa-home.html`, 674 484 B). |
| 2 | **Geographic coverage** | New Hampshire statewide (plus a Middle School League). |
| 3 | **Sports** | Sport pages per season: `/sports/fall/boys-cross-country/`, `/sports/fall/girls-cross-country/`, `/sports/winter/{boys,girls}-indoor-track/`, `/sports/spring/{boys,girls}-outdoor-track/`, plus unified outdoor track (`samples/sitemap-nhiaa.xml`; 386 sitemap URLs total). |
| 4 | **Historical depth** | Not measured; site carries "Championship Programs" and historical pages. |
| 5 | **Discovery mechanism** | `/member-directory/` — paginated member directory; pagination reaches `/member-directory/page/42/` (calendar pagination), and `/about-nhiaa/schools/` is the informational page (`samples/nhiaa-member-directory.html`, `samples/nhiaa-member-directory-p42.html`). School rows are **not** in the static HTML (`[INFERENCE]` rSchoolToday client-side data layer), so the member count is **not measured**. |
| 6 | **Stable identifiers** | Page paths per sport; rSchoolToday site id `586` visible in asset URLs (`assets-rst7.rschooltoday.com/rst7files/uploads/sites/586/…`). |
| 7 | **Pagination** | Yes, `/member-directory/page/<n>/`, n reaching 42 in the captured markup. |
| 8 | **Athlete fields** | None observed. |
| 9 | **Meet fields** | Championship programmes page plus season packets: "2026-27 CNESSPA Cross Country New England Championship Packet" (`samples/nhiaa-boys-xc.html`). |
| 10 | **Result fields** | None on the association host in static HTML; results/schedules ride the rSchoolToday + Arbiter stack. |
| 11 | **Grade/class evidence** | Divisional structures not captured. |
| 12 | **Coach/contact fields** | None captured (rSchoolToday typically carries them; not verified). |
| 13 | **Public API availability** | Not established; assets come from `rst7.rschooltoday.com`. |
| 14 | **Static file availability** | Yes — packets/programmes as PDFs. |
| 15 | **Browser requirement** | **Likely yes** for the member directory (client-rendered rows); no for sport pages. |
| 16 | **Request cost** | Moderate: 42 pages if the directory is harvested — but the rows are not in HTML, so harvesting requires the data layer. |
| 17 | **Published rate limits** | None — `robots.txt` returns **404** (13 B, `samples/robots-nhiaa.txt`). |
| 18 | **Known blocks** | None from robots; the practical block is client-side rendering. |
| 19 | **Cross-source join keys** | School name; NHIAA↔CNESSPA (New England) championship naming for cross-state meets; MileSplit `nh.milesplit.com` (108 team links measured). |
| 20 | **Estimated marginal coverage** | Moderate: sport/season scaffolding and New England packet linkage; membership and results require the rSchoolToday layer. |
| 21 | **Implementation recommendation** | **DISCOVERY_SOURCE**. |

---

## VT — Vermont Principals' Association (VPA)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | Vermont Principals' Association, `https://vpaonline.org/` (WordPress + Yoast; `samples/vpa-home.html`, 188 000 B). |
| 2 | **Geographic coverage** | Vermont statewide (VPA athletics division). |
| 3 | **Sports** | Athletics pages: `/athletics/scoreboard/`, `/athletics/tournaments/`, `/athletics/tournament-pairings/`, `/athletics/divisional-alignments/`, `/athletics/sports-rankings/`, `/athletics/vpa-hall-of-fame/`, plus policy pages (17 `/athletics/` URLs in `samples/sitemap-vpa-page.xml`). Track & field is not exposed as its own athletics page in the sitemap. |
| 4 | **Historical depth** | Not measured; the site links `vermontsportshistory.com`. |
| 5 | **Discovery mechanism** | Weakest of the twelve: `/athletics/divisional-alignments/` is a hub page with no member list in static HTML (`samples/vpa-divisional-alignments.html`); the alignment tables are `[INFERENCE]` inside linked documents. |
| 6 | **Stable identifiers** | Page paths only; no numeric school/sport ids observed. |
| 7 | **Pagination** | None observed. |
| 8 | **Athlete fields** | None on the association host. |
| 9 | **Meet fields** | Tournament pairings and scoreboard pages; the actual documents are Google Docs/Sheets. |
| 10 | **Result fields** | Published as Google documents — e.g. `https://docs.google.com/document/d/1_DMlEHHX3zIzO2jgqzaXLv2Puklx-o4y7IriQtRjQ0I/edit#heading=h.uhyazdvuarq9`, `https://docs.google.com/spreadsheets/d/17w5IYbT7P4yJx6xDjCefbRqamG2GeKDx6pl18zYYbVY/edit?usp=sharing`, `https://docs.google.com/document/d/1vFg8T7cDnMppJywj7abYGng81kBHryBSYgAviiCkMwE/edit` (`samples/vpa-scoreboard.html`). No structured result fields. |
| 11 | **Grade/class evidence** | Divisions exist as a concept (`Divisional Alignments`); no per-athlete grade. |
| 12 | **Coach/contact fields** | None captured. |
| 13 | **Public API availability** | None observed. |
| 14 | **Static file availability** | PDFs/documents exist; the high-value ones are Google-hosted, not VPA-hosted. |
| 15 | **Browser requirement** | No for captured pages. |
| 16 | **Request cost** | Very low page count, but **high wall-clock cost**: 10 s crawl-delay. |
| 17 | **Published rate limits** | **Yes.** `samples/robots-vpa.txt` (187 B): `Crawl-delay: 10` and `User-agent: * / Disallow:` (empty → nothing blocked) with `Sitemap: https://vpaonline.org/sitemap_index.xml`. |
| 18 | **Known blocks** | None; the constraint is the crawl-delay and the document-hosted data. |
| 19 | **Cross-source join keys** | School names inside Google-hosted documents (unverified); MileSplit `vt.milesplit.com` (117 team links measured); `vermontsportshistory.com` as a possible historical cross-reference. |
| 20 | **Estimated marginal coverage** | Low–moderate: championship/scoreboard pointers, NFHS Network and MaxPreps links; no structured universe or results without parsing external documents. |
| 21 | **Implementation recommendation** | **VALIDATION_SOURCE**. |

---

## ME — Maine Principals' Association (MPA)

| # | Field | Finding |
|---|---|---|
| 1 | **Source name** | MPA, `https://www.mpa.cc/` (shared ASP.NET vendor platform; `samples/mpa-home.html`, 54 994 B). Host discovery: `mpa.ccsso.org` / `www.mpa.ccsso.org` are NXDOMAIN; the live host is `mpa.cc` (homepage text carries "Maine Principals"; `robots.txt` is byte-identical to RIIL's and CIAC's). |
| 2 | **Geographic coverage** | Maine statewide, with Northern/Southern regions. |
| 3 | **Sports** | Sport list with tournament ids incl. Cross Country Boys `1` / Girls `9`, Indoor Track Boys `104` / Girls `300`, Outdoor Track Boys `204` / Girls `301` (`samples/mpa-school-list.aspx`). |
| 4 | **Historical depth** | Current season (2026-2027) plus "Historic Schedules" nav; depth not measured. |
| 5 | **Discovery mechanism** | `/SchoolPages/School.aspx` → **152 distinct `SchoolID`** values (e.g. `SchoolID=25 → Ashland District School`, `SchoolID=26 → Bangor Christian Schools`), levels mixed (`samples/mpa-school-list.aspx`). |
| 6 | **Stable identifiers** | `SchoolID`, `TournamentID`. |
| 7 | **Pagination** | None observed. |
| 8 | **Athlete fields** | None observed. |
| 9 | **Meet fields** | Season calendar keys: "Regular Season Meet", "Last Countable Meet", "Regional Championships", "State Championships", "New England Championships", plus entries deadlines (`samples/mpa-xc-boys.aspx`). |
| 10 | **Result fields** | Not on mpa.cc in static HTML; entries and results route to MileSplit and Sub5.com (field 19). |
| 11 | **Grade/class evidence** | Class/region structure implied by regional vs state championships; no grade data. |
| 12 | **Coach/contact fields** | None public. |
| 13 | **Public API availability** | None observed. |
| 14 | **Static file availability** | Yes — championship programmes/apparel links; documents under `/sites/…`/vendor paths. |
| 15 | **Browser requirement** | No. |
| 16 | **Request cost** | Low. |
| 17 | **Published rate limits** | None published; `samples/mpa-robots.txt` (247 B) carries the six dashboard disallows. |
| 18 | **Known blocks** | robots-disallowed: `/DashboardSchedule.aspx`, `/DashboardGame.aspx`, `/DashboardSchool.aspx`, `/DashboardTeamSchedule.aspx`, `/ScheduleByWeek.aspx`, `/WebBot.aspx`. |
| 19 | **Cross-source join keys** | `SchoolID` ↔ school name ↔ `me.milesplit.com` team (160 measured). **Entry-platform markup** — `samples/mpa-xc-boys.aspx`: `<span class="SportInfoTitle">Entries must be posted to MileSplit:` and `<span class="SportInfoTitle">Entries Posted to Sub5.com:</span> October 20`, plus the partner carousel `<span class='CarouselTracker' CarouselID='9' Carousel='Partner' CarouselEntryID='1025' CarouselEntry='Mile Split'><a target='_blank' href='https://me.milesplit.com/'>`. |
| 20 | **Estimated marginal coverage** | Moderate–high for Maine: school list, season calendar with entry deadlines, and two named entry platforms (MileSplit + Sub5.com) that tell the crawler where results will appear. |
| 21 | **Implementation recommendation** | **DISCOVERY_SOURCE**. |
