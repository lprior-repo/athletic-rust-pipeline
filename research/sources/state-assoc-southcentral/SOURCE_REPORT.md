# South-Central state associations — SOURCE_REPORT.md

Lane: `research/sources/state-assoc-southcentral/` · Jurisdictions: **AL, AR, KY, LA, MS, OK, TN, TX**
Collected: **2026-09-22, 03:54:10–04:06:15 UTC** (123 network captures; PDF/text extraction continued after) · Agent: `ResStateSouthCentral` · Method: anonymous `curl` only.

Every factual line below cites a captured file in `samples/` or the exact command that produced it.
Anything reasoned but not observed is marked `[INFERENCE]`; anything that could not be obtained is
marked `UNVERIFIED` with the reason.

**Collection policy actually applied.** `robots.txt` was fetched for every host before content
(sidecars: `samples/robots-*.txt`, listed in `samples/CAPTURES.md`). Requests were sequential, one
in flight, ≥1 s apart per host, no cookies, no auth, no JS engine, User-Agent
`ad-law-scrape-source-research/0.1 (anonymous; respectful 1rps; contact: repo owner)`. 123 network
captures are ledgered with URL, status, byte count, UTC timestamp and exact command.

**Compliance disclosure (must read).** `www.uiltexas.org/robots.txt` (`samples/robots-www.uiltexas.org.txt`)
contains `Disallow: /files/`, and three UIL PDFs fetched for this report live under that path
(`Alpha_26-28.pdf`, `26-28_Rank.pdf`, `TF_School_Codes_2026.pdf`). They were fetched with explicit
URLs before the `/files/` rule was reconciled; no further `/files/` requests were made after that, and
the three samples are marked in `samples/CAPTURES.md`/`coverage.json`. The Texas numbers below come
from those PDFs, so the owner must either accept them or re-derive them from the allowed HTML index
pages (`/alignments`, `/alignments/category/*`), which only *link* the PDFs. Two hosts that publish
`Disallow: /` for anonymous agents (`www.ahsaa.com`, `lhsaaonline.org`) were never fetched for content,
and `misshsaa.com` was abandoned after its blanket HTTP 403.

---

## Batch summary

| # | Jurisdiction | Association | Site | Sanctioned school universe (measured) | Result surface | Timing / producer | Recommendation |
|---|---|---|---|---|---|---|---|
| 1 | AL | AHSAA | ahsaa.com | **blocked** (robots `Disallow: /`); proxy: 562 MileSplit AL team pages | Xpress Timing → `xt.anet.live` | Xpress Timing | CONDITIONAL |
| 2 | AR | Arkansas Activities Association | ahsaa.org | **502** member schools (AAA homepage stat; no per-school list) | delegated to MileSplit AR | MileSplit | DISCOVERY_SOURCE |
| 3 | KY | KHSAA | khsaa.org | **290** schools w/ ArbiterLive entity ids | KHSAA posts + `milesplit.live` | MileSplit (registration) | VALIDATION_SOURCE |
| 4 | LA | LHSAA | lhsaa.org | 306 state-meet schools (derived); membership count **UNVERIFIED** (108-page directory PDF, font-encoded) | HY-TEK PDFs 2003–2026 + Delta Timing | Delta Timing / HY-TEK | RESULT_SOURCE |
| 5 | MS | MHSAA | misshsaa.com | **blocked** (HTTP 403); proxy: 463 MileSplit MS team pages | MileSplit MS only | UNVERIFIED | CONDITIONAL |
| 6 | OK | OSSAA ("OSSAA Illustrated") | ossaaillustrated.com | **482** member schools (PDF) | per-class MeetPro PDFs | DirectAthletics MeetPro | **PRIMARY** |
| 7 | TN | TSSAA | tssaa.org / tssaasports.com | **456** schools w/ TSSAA ids | 151 championship PDFs since 1960-61 | AllTrax Timing / HY-TEK | **PRIMARY** |
| 8 | TX | UIL | uiltexas.org | **1,566** realignment schools; **1,368** T&F school codes | HTML per-conference tables | UNVERIFIED | CONDITIONAL |

---

## 1. Alabama — AHSAA (Alabama High School Athletic Association)

1. **Source name** — Alabama High School Athletic Association; `https://www.ahsaa.com/` (apex `https://ahsaa.com/`).
2. **Geographic coverage** — Alabama (all AHSAA member schools).
3. **Sports** — track & field + cross country are sanctioned (state XC championship observed for 2025-11-08) `samples/al-ahsaa-2025-xc-state-live.html`.
4. **Historical depth** — `UNVERIFIED` on the association side (site not crawlable). Xpress Timing's public archive spans 2020–2026 live meet folders `samples/al-xpresstiming-home.html`.
5. **Discovery mechanism** — **blocked**: `robots.txt` is `User-agent: * / Disallow: /` with Googlebot/Bingbot as the only allowed agents (`samples/robots-www.ahsaa.com.txt`, `samples/robots-ahsaa-apex.txt`). The only anonymous discovery path found is the timing provider's meet index.
6. **Stable identifiers** — none observed from the association. MileSplit Alabama team ids are stable (`/teams/<id>-<slug>`), 562 enumerated `samples/al-milesplit-teams-ids.txt`.
7. **Pagination** — `n/a` (no association surface reachable). The MileSplit team index is a single page (562 links).
8. **Athlete fields** — `UNVERIFIED`: the 2025 AHSAA state XC page publishes **no** athlete data itself; it links `http://xt.anet.live/3bb7ev` ("Live Results") `samples/al-ahsaa-2025-xc-state-live.html`.
9. **Meet fields** — observed on the Xpress Timing page: `AHSAA State Championship`, `Saturday, November 8, 2025`, venue `Oakville Indian Mounds`, `Oakville, AL`, timing provider `www.xpresstiming.com`, contact `paul.xpresstiming@gmail.com`.
10. **Result fields** — none captured (delegated to anet.live; that host was deliberately not fetched, per the repo's standing Athletic.net exclusion in `SOURCES_SURVEY.md` §0).
11. **Grade/class evidence** — `UNVERIFIED`.
12. **Coach/contact fields** — none public anon-reachable; a timing-provider contact email is the only address captured.
13. **Public API availability** — none observed.
14. **Static file availability** — Xpress Timing serves plain-HTML result folders (`Live/<year>/<season>/<Meet>/index.html`); 217 distinct `Live/…` paths listed on the home page `samples/al-xpresstiming-home.html`.
15. **Browser requirement** — none for the timing host (static HTML); the association site is not fetchable at all.
16. **Request cost** — 2 requests yield a state-meet page + live link; the association adds nothing.
17. **Published rate limits** — none found on either host.
18. **Known blocks** — AHSAA `Disallow: /`; and **every** Xpress Timing meet page found delegates its live result stream to `xt.anet.live` (Athletic.net infrastructure) — 1,543 `xt.anet.live` links on the single home page.
19. **Cross-source join keys** — school name only `[INFERENCE]`; no AHSAA school id observed.
20. **Estimated marginal coverage** — low for athlete data (terminates on an excluded host); moderate as *meet discovery* (Alabama meet calendar via Xpress Timing index).
21. **Implementation recommendation** — **CONDITIONAL**. Use only as a meet-calendar/venue feed, and only after the owner rules on `anet.live`.

**Linkage quote (Athletic.net Live):**
```html
<a href="http://xt.anet.live/3bb7ev">Live Results</a>      <!-- samples/al-ahsaa-2025-xc-state-live.html -->
```
Note: `alabamarunners.com` does **not** resolve (curl: `Could not resolve host`, NXDOMAIN); the
"AlabamaRunners" brand now lives at `al.milesplit.com` (page title `AlabamaRunners | Alabama High
School Running News and Videos | Cross Country and Track & Field`) `samples/al-milesplit-home.html`.

---

## 2. Arkansas — AAA (Arkansas Activities Association)

1. **Source name** — Arkansas Activities Association; `https://www.ahsaa.org/` (robots: `Allow: /`, only `/api/` disallowed).
2. **Geographic coverage** — Arkansas.
3. **Sports** — XC (`/page/cross-country/`) and track & field (`/page/track-and-field/`) pages captured.
4. **Historical depth** — AAA's state-champion/history links point at Google Drive folders (published on the track page); no AAA-hosted result archive found.
5. **Discovery mechanism** — AAA homepage stats block plus per-sport pages. Per-school data is not on the AAA site: the nav's "Public Directory" points to `https://go.dragonflyathletics.com/` (DragonFly, AAA's roster/eligibility platform) `samples/ar-dragonfly.html`.
6. **Stable identifiers** — none from AAA; MileSplit AR meets use `milesplit.live/meets/<id>` (e.g. `731034`).
7. **Pagination** — n/a (stat block); the AAA sitemap has 47 URLs `samples/ar-aaa-sitemap.xml`.
8. **Athlete fields** — none on AAA; entries/results pass to MileSplit (`AR MileSplit` docs are published from the AAA XC page).
9. **Meet fields** — AAA XC page: state-meet entries deadline, state meet, course map; AAA track page: "Meet of Champs".
10. **Result fields** — none (delegated).
11. **Grade/class evidence** — `UNVERIFIED` (AAA class bands not published in a crawlable file in this pass).
12. **Coach/contact fields** — AAA publishes an "AD Directory" that is a **Google Form** (`docs.google.com/forms/d/e/1FAIpQLScyVZj.../viewform`) and a "Staff Directory" page; no coach list.
13. **Public API availability** — none observed.
14. **Static file availability** — AAA documents are served from `cmsv2-assets.apptegy.net/uploads/34192/file/...` (Apptegy CDN) — 100s of files reachable without auth.
15. **Browser requirement** — no for AAA pages; yes for `go.dragonflyathletics.com` (SPA; robots.txt itself returns an S3 `AccessDenied` XML, i.e. no published rules) `samples/robots-dragonflyathletics.com.txt`.
16. **Request cost** — 1 request per AAA page; the published count is on the homepage.
17. **Published rate limits** — none.
18. **Known blocks** — none.
19. **Cross-source join keys** — school name `[INFERENCE]`; AAA has no published numeric school id.
20. **Estimated marginal coverage** — high as an authority/discovery signal (sanction + calendar), zero as an athlete record source.
21. **Implementation recommendation** — **DISCOVERY_SOURCE**.

**Enumeration markup (AAA homepage counter):**
```json
"btn-title":{"text":"Member Schools"},"btn-value":{"text":"502"}
"btn-title":{"text":"Students involved in Activities"},"btn-value":{"text":"150,000+"}
"btn-title":{"text":"Championships Awarded Annually"},"btn-value":{"text":"108"}
```
**Linkage quotes (MileSplit):**
```html
<a target="_blank" rel="noopener noreferrer nofollow" href="https://ar.milesplit.com/calendar/arkansas-cc-meet-calendar">Cross Country Meets</a>
<a target="_blank" rel="noopener noreferrer nofollow" class="d-block link" href="https://milesplit.live/meets/731034/events/4/assignments/F/F">Meet of Champs Entries</a>
```
plus the AAA-published partner documents "ARMileSplit - Letter of Partnership", "ARMileSplit - How to
Enter a Meet Online", "ARMileSplit - How to Set Up Your Meet for Online Registration",
"ARMileSplit - How to Upload Meet Results" (all on `samples/ar-cross-country.html`).

---

## 3. Kentucky — KHSAA

1. **Source name** — Kentucky High School Athletic Association; `https://khsaa.org/`.
2. **Geographic coverage** — Kentucky.
3. **Sports** — XC and T&F pages: `/sports-sport-activities/individual-sports/cross-country/` and `…/track/`.
4. **Historical depth** — indoor + outdoor + XC state results posts by class; XC results history index `khsaa.org/category/cross-country/cross-country-tournament-results-history/`.
5. **Discovery mechanism** — member-school directory page with **290** per-school ArbiterLive links; basic directory at `live.arbiter.io/org/2507`.
6. **Stable identifiers** — ArbiterLive `entityId` per school (131…123058), extracted to `samples/ky-schools-arbiterlive.tsv`.
7. **Pagination** — single page (two identical 290-entry `<ul>` blocks: one for ArbiterLive pages, one for the basic directory).
8. **Athlete fields** — not on KHSAA; the directory page states data "mirrors the ArbiterLive page of each school" (rosters included).
9. **Meet fields** — state-meet posts carry class (1A/2A/3A), indoor/outdoor/XC, dates.
10. **Result fields** — KHSAA posts results as news items (`/05-23-26-class-3a-outdoor-track-field-state-championships-results/`); the page body is composed client-side — live results come from MileSplit.
11. **Grade/class evidence** — classes 1A–3A (three classes) and enrollment data referenced via "School Enrollments"/"Triple Threat Award" report pages; per-school enrollment file not exposed as a crawlable link in this pass (`samples/ky-enrollment-listings.html`).
12. **Coach/contact fields** — coaching staffs and rosters are stated to be school-managed and mirrored on ArbiterLive; the KAAA (`kaaa.us`) is linked for athletic administrators.
13. **Public API availability** — none observed (ArbiterLive is an SPA; its JSON endpoint was **not** probed or guessed).
14. **Static file availability** — WordPress site; posts are HTML.
15. **Browser requirement** — KHSAA: no. ArbiterLive school pages: yes.
16. **Request cost** — 1 request gives all 290 school ids.
17. **Published rate limits** — none published; robots forbids `/wp-admin/`, `/wp*`, `school_logos`, `payments`, `Publications`, `officials`, `rpi_details`, and all of GPTBot.
18. **Known blocks** — none for anonymous non-bot agents.
19. **Cross-source join keys** — `arbiterlive entityId` ↔ school name; MileSplit `meets/<id>` for meets.
20. **Estimated marginal coverage** — high for *roster/coach verification* and membership truth; results are a MileSplit hand-off.
21. **Implementation recommendation** — **VALIDATION_SOURCE**.

**Linkage quote (MileSplit live results + registration-provider news):**
```html
<li><a title="Follow Live Results via MileSplit" href="https://milesplit.live/meets/711786"><img src="/images/2024-khsaa-xc-cropped-transparent.png" …></a></li>
```
news post titles on the same page: "Milesplit Registration for State Indoor Track & Field Meet",
"MileSplit Named Online Registration Provider for KHSAA Cross Country/Track Field" (2024-07-31 and
2024-08-19 posts are linked from `samples/ky-cross-country.html`).

---

## 4. Louisiana — LHSAA

1. **Source name** — Louisiana High School Athletic Association; `https://www.lhsaa.org/`.
2. **Geographic coverage** — Louisiana.
3. **Sports** — XC (`/cross-country`), indoor (`/indoor-track-field`) and outdoor (`/outdoor-track-and-field`) T&F pages all captured.
4. **Historical depth** — outdoor state results PDFs **2003–2026** (with gaps 2012, 2020) linked from the outdoor page; 2022 is a Delta Timing live-results link.
5. **Discovery mechanism** — sport pages + `Champ Central`; no site-wide sitemap (`/sitemap.xml` 302→`error.php`). `lhsaaonline.org` (member portal) is `Disallow: /`; `lhsaa.org`/`www.lhsaa.org` publish **no robots.txt at all** (`/robots.txt` 302 → `/error.php` 404), i.e. no published restrictions (RFC 9309 "unavailable").
6. **Stable identifiers** — none numeric observed for schools; meet/season results are per-year PDFs, and live meets use Delta Timing `meets/<id>`.
7. **Pagination** — n/a.
8. **Athlete fields** — from the 2026 state PDF: `Name`, `Year` (blank in the B/C/1A section; present in others), `School`.
9. **Meet fields** — `2026 LHSAA Track & Field Outdoor Meet`, `5/7/2026 to 5/9/2026`, `Bernie Moore Track, LSU Campus`, per-session headers (`Thursday B, C and 1A`, `Friday 2A, 3A`, `Saturday 4A, 5A`).
10. **Result fields** — `place`, `Name`, `Year`, `School`, `Finals`, `Wind`; the 2026 outdoor PDF yields **2,266** place-led result lines (2,017 carrying a numeric mark) and **1,749** lines parsing to a school token; class/composite record lines are embedded per event. Reproduced by `pdftotext -layout samples/la-2026-state-champ-results.pdf samples/la-2026-state-champ-results.txt` then the two regexes documented in `coverage.json` (`LA.enumeration.derivation`).
11. **Grade/class evidence** — class structure `B, C, 1A, 2A, 3A, 4A, 5A` (7 classes) visible in the results PDF; the `Year` column is grade evidence.
12. **Coach/contact fields** — **LHSAA Coaches Directory 2025-2026 PDF** (108 pages): per school → school name, class, phone, Athletic Director, Principal, per-sport coach names with emails. Text layer is subset-font encoded (raw token `4G9E<` = "School", `O[[6P` = "Athletic"), so fields are documented but not yet machine-extractable.
13. **Public API availability** — none observed.
14. **Static file availability** — yes: `lhsaa.org/siteuploads/editorimg/file/...` PDFs, served without auth.
15. **Browser requirement** — no (PDF/HTML only).
16. **Request cost** — 1 request per season PDF; the 2026 state results PDF alone carries 2,266 place-led result lines across 7 classes.
17. **Published rate limits** — none.
18. **Known blocks** — `lhsaaonline.org` `Disallow: /`; unknown paths soft-404 to `error.php`.
19. **Cross-source join keys** — school name; Delta Timing meet id.
20. **Estimated marginal coverage** — high for verified state-meet results with grade + wind, and for coach/AD contacts once the PDF encoding is decoded.
21. **Implementation recommendation** — **RESULT_SOURCE**.

**Linkage quotes (MileSplit + Delta Timing):**
```html
<li><a href="https://www.milesplit.com/articles/336667/meetpro-milesplit-the-future-of-meet-management">MeetPro Meet Management Software Information</a>
<li><a href="https://support.milesplit.com/s/lhsaa-coach-onboarding">MileSplit Coach Onboarding</a>
```
```html
…<a href="https://live.deltatiming.com/meets/16151/winners">2022</a>…      <!-- samples/la-outdoor-track-field.html -->
```
Producer line inside the state PDF: `Hy-Tek's MEET MANAGER 9:12 PM 5/7/2026`.

---

## 5. Mississippi — MHSAA (Mississippi High School Activities Association)

1. **Source name** — Mississippi High School Activities Association; `https://www.misshsaa.com/`.
2. **Geographic coverage** — Mississippi.
3. **Sports** — XC/track covered historically (per-class result posts) `samples/wayback-cdx-misshsaa-xc.txt`.
4. **Historical depth** — archived CDX shows per-class XC result posts (Class 1A–6A boys/girls) and "Track Regions"/"Cross Country Championships" pages; captures continue to at least 2019.
5. **Discovery mechanism** — **blocked**: `robots.txt` returns HTTP 403 and so does the home page, on both apex and `www` (`samples/robots-misshsaa.com.txt`, `samples/ms-misshsaa-home.html`, `samples/ms-misshsaa-home-apex.html`). The Wayback crawl of 2025-11-09 captured `/.well-known/sgcaptcha/?r=%2Fcategory%2Fschools%2F…` interstitials, i.e. a bot-shield is in front of the site (`samples/wayback-cdx-misshsaa-members.txt`). No bypass attempted.
6. **Stable identifiers** — MileSplit MS team ids (`/teams/1140-tupelo-high-school`), 463 enumerated `samples/ms-milesplit-teams-ids.txt`.
7. **Pagination** — n/a (blocked); MileSplit team index is one page.
8. **Athlete fields** — `UNVERIFIED` (only third-party MileSplit meets are reachable, e.g. `ms.milesplit.com/meets/761631-simpson-academy-invitational-2026/results`).
9. **Meet fields** — MileSplit MS meet ids/titles only.
10. **Result fields** — `UNVERIFIED`.
11. **Grade/class evidence** — class structure 1A–6A is visible only in archived MHSAA posts.
12. **Coach/contact fields** — none reachable.
13. **Public API availability** — none.
14. **Static file availability** — none reachable now.
15. **Browser requirement** — the live site requires passing a bot-shield even in a browser session; not attempted.
16. **Request cost** — n/a.
17. **Published rate limits** — n/a (robots itself is 403).
18. **Known blocks** — blanket HTTP 403; archive captures after ~2023 are captcha interstitials.
19. **Cross-source join keys** — MileSplit team id/slug only.
20. **Estimated marginal coverage** — the state is reachable only through MileSplit Mississippi; the sanctioned universe (school count) is unavailable, so the Association cannot currently anchor membership.
21. **Implementation recommendation** — **CONDITIONAL**.

---

## 6. Oklahoma — OSSAA ("OSSAA Illustrated") — strongest association surface in the batch

1. **Source name** — Oklahoma Secondary School Activities Association; `https://ossaa.com/` 302-redirects to `https://ossaaillustrated.com/` (robots of the new host: only `/wp-admin/` disallowed, sitemap at `/wp-sitemap.xml`).
2. **Geographic coverage** — Oklahoma.
3. **Sports** — XC (`/cross-country/`), track (`/track/`), plus 15 other sports pages.
4. **Historical depth** — per-season PDFs (2025 XC, 2026 track) and an activity-by-activity classification history.
5. **Discovery mechanism** — **"OSSAA Member Schools" PDF**, a numbered 4-column, 2-page list: **482 schools** (`samples/ok-memberschools-2025-26.pdf`, extracted `samples/ok-memberschools-2025-26.txt`).
6. **Stable identifiers** — none numeric for schools; per-athlete bib numbers appear in result PDFs; direct URLs keyed by activity+class+year.
7. **Pagination** — the member list is 2 pages; result PDFs are single files (5A-6A XC report = 568 lines of text).
8. **Athlete fields** — `Athlete` (LAST, First), `YR` (grade), bib `#`, `Team`, `Score`.
9. **Meet fields** — `OSSAA 5A-6A CROSS COUNTRY STATE CHAMPIONSHIPS`, `Edmond, OK`, `Santa Fe High School`, `Saturday, November 1, 2025`, `Race #11`, `WOMEN (6A) • 5 Kilometers (3.11 Miles)`, printed timestamp `11/2/2025 9:35 AM`.
10. **Result fields** — individual: place, athlete, YR, bib, team, score, `Time`, `Gap`, `Avg. Mile`, `Avg. kM`, `1600 Meters`, `3200 Meters`; team: place, school, `Score`, `Scoring Order`, `Total`, `Avg.`, `Spread`.
11. **Grade/class evidence** — `YR` column per athlete (SR/JR/SO/FR observed); classification bands per activity: XC `6A=32 largest, 5A=next 40, 4A=next 56, 3A=next 56, 2A=next 72, A=all remaining` (`samples/ok-classifications-2025-26.pdf`).
12. **Coach/contact fields** — **none published**: `/athletic-directors/` is handbooks/manuals only (0 tables); OSSAA member login is `http://www.ossaarankings.com/membership/MemberLogin.aspx` (login-gated).
13. **Public API availability** — none observed.
14. **Static file availability** — yes: WordPress-hosted PDFs under `/wp-content/uploads/…` (member list, classifications, ADM, state results, regional sites).
15. **Browser requirement** — no.
16. **Request cost** — 1 PDF = 482-school universe; 1 PDF = one class's full state result with splits.
17. **Published rate limits** — none; robots permits all but `/wp-admin/`.
18. **Known blocks** — none. Quirk: the 5A XC state-results file is published with a typo `CC_2025-26_5AStaeResults.pdf`.
19. **Cross-source join keys** — school name (member list) ↔ result PDF team names; MileSplit for meet-level joins.
20. **Estimated marginal coverage** — the best all-round association source here: membership count, classification bands with enrollment basis (ADM PDF), grade-bearing results with splits, and an explicit MileSplit partnership.
21. **Implementation recommendation** — **PRIMARY**.

**Linkage quote (MileSplit partnership):**
```html
<p class="wp-block-paragraph"><strong><a href="https://ossaaillustrated.com/wp-content/uploads/2024/01/TR_2023-24_MileSplitPartnership.pdf" data-type="link" data-id="https://ossaaillustrated.com/wp-content/uploads/2024/01/TR_2023-24_MileSplitPartnership.pdf">OSSAA Partners with MileSplit</a></strong></p>
```
PDF text (`samples/ok-milesplit-partnership.pdf`): *"MileSplit Named Strategic Partner of OSSAA Cross
Country and Track and Field … OKLAHOMA CITY, OK – January 4, 2024"*. Result producer line:
`DirectAthletics MeetPro 12`.

---

## 7. Tennessee — TSSAA

1. **Source name** — Tennessee Secondary School Athletic Association; `https://tssaa.org/`, championships at `https://tssaasports.com/`.
2. **Geographic coverage** — Tennessee (high schools + a separate middle-school portal `ms.tssaa.org`).
3. **Sports** — XC (boys/girls), T&F (boys/girls), unified T&F — 24 sports listed `samples/tn-sports-index.html`.
4. **Historical depth** — **151** boys-XC championship result files back to **1960-1961** `samples/tn-champs-history-results-xc-boys.html`.
5. **Discovery mechanism** — the member directory is a JS-embedded array on `portal.tssaa.org/common/directory/`: **456 schools** with TSSAA ids (`samples/tn-portal-directory.html`, extracted `samples/tn-schools-extracted.tsv`). robots for the portal: `Allow: /common` with `Disallow: /` otherwise — the directory path is explicitly allowed (`samples/robots-portal.tssaa.org.txt`).
6. **Stable identifiers** — TSSAA school id (1…875 in the captured set); championship ids (`20250803` = 2025 D-I AAA boys XC).
7. **Pagination** — single page for the directory; results index is one page per sport.
8. **Athlete fields** — state result PDFs: `Name`, `Year` (grade 9–12), `School`, `Finals`, `Points`.
9. **Meet fields** — `2025 TSSAA State Cross Country Championship`, `Shelby Farms Park - Memphis,TN`, `11/6/2025 to 11/7/2025`, event `Event 2 Boys 5k Run CC D1AAA`.
10. **Result fields** — place, name, year, school, finals time, points; producer `Hy-Tek's MEET MANAGER`, timing service `AllTrax Timing - Contractor License`.
11. **Grade/class evidence** — `Year` column is literal grade; classification is enrollment-based: *"TSSAA classifies schools by Grade 9-12 enrollment"*, current cycle 2025-2027 (`samples/tn-classification.html`); 2025 XC divisions D-I AAA/AA/A and D-II A/AA.
12. **Coach/contact fields** — not published; `/home/coaches` is a coaching-resources hub and the directory gives school + city only.
13. **Public API availability** — none; but the directory data is embedded in the HTML (no API needed).
14. **Static file availability** — yes: `/event/file.cfm?championshipid=<id>&type=results` returns PDFs (328,656 bytes / 7 pages for 2025 D-I AAA).
15. **Browser requirement** — no.
16. **Request cost** — 1 request = whole member directory; 1 request per championship PDF.
17. **Published rate limits** — none; `tssaa.org` robots disallows `/cpresources/`, `/vendor/`, `/.env`, `/cache/`.
18. **Known blocks** — none.
19. **Cross-source join keys** — TSSAA school id; championship id; sport slug (`track-boys`, `cross-country-boys`).
20. **Estimated marginal coverage** — high: a 456-school id'd universe, per-sport rosters/schedules URLs, and deep, grade-bearing championship PDFs.
21. **Implementation recommendation** — **PRIMARY**.

**Linkage quotes (MileSplit + MaxPreps, rosters/schedules):**
```html
<a class="list-group-item list-group-item-action" href="https://tssaasports.com/sports/rosters/?sport=track-boys">Team Rosters</a>
<a class="list-group-item list-group-item-action" href="https://tssaasports.com/sports/schedules/?sport=track-boys">Team Schedules</a>
<a class="list-group-item list-group-item-action" href="https://tssaa.org/maxpreps-official-sports-statistics">MaxPreps Official Sports Statistics</a>
<a class="list-group-item list-group-item-action" href="https://tssaa.org/getting-started-with-milesplit">Getting Started with MileSplit</a>
```
`getting-started-with-milesplit` 302 → `https://cms-files.tssaa.org/documents/tssaa/track/milesplit-on-boarding-2021.pdf`
(TSSAA-hosted MileSplit onboarding PDF).

---

## 8. Texas — UIL (special case: the association is not the result host)

1. **Source name** — University Interscholastic League; `https://www.uiltexas.org/`.
2. **Geographic coverage** — Texas (public schools; the UIL is the state's sole sanctioned association for these sports).
3. **Sports** — XC (`/cross-country`, `/cross-country/state`) and T&F (`/track-field`, `/track-field/state`).
4. **Historical depth** — XC state results per class from 2025 pages plus `/cross-country/historical-archive`; alignment cycles are 2-year (2026-28 current).
5. **Discovery mechanism** — **UIL realignment**: HTML index `/alignments` → per-activity category pages (`/alignments/category/align-cross-country`, `align-spring-athletics`, …) which link the per-class alignment PDFs under `/files/alignments/…`; plus a **Track & Field School Codes** PDF.
6. **Stable identifiers** — **4-letter UIL school code** (e.g. `ABBO` for `ABBOTT HS`) with conference/district/region; 1,368 unique codes.
7. **Pagination** — HTML index page + one PDF per class; the school-code list is a single PDF.
8. **Athlete fields** — state results pages carry `Name`, `School`, `Time` (individual table). No grade column observed on the UIL page.
9. **Meet fields** — state XC: `2025-2026 6A Boys Results`, date `11/01/2025`, venue Old Settlers Park, Round Rock; broadcast/spectator/program sub-pages.
10. **Result fields** — team table `School | Total Score | Notes` (e.g. `Southlake Carroll | 71 | 1st Place`); individual table `Name | School | Time | Notes`.
11. **Grade/class evidence** — conference `1A–6A` + enrollment per school in the realignment PDFs (e.g. `Allen 6798 → 6A`, `San Vicente 3 → 1A`); conference counts: 1A 225, 2A 241, 3A 285, 4A 283, 5A 269, 6A 263 (1,566 schools); T&F school-code list: 1A 210, 2A 223, 3A 229, 4A 213, 5A 250, 6A 243 (1,368 schools).
12. **Coach/contact fields** — none per school; UIL publishes T&F department contacts (Assistant ADs Virginia Flores and Joseph Garmon, phone 512-471-5883, emails behind `.(JavaScript must be enabled…)`). Coaches/ADs use `/uil-portal` (login).
13. **Public API availability** — none observed.
14. **Static file availability** — yes, but under the robots-disallowed `/files/` path (see disclosure).
15. **Browser requirement** — no.
16. **Request cost** — 1 PDF = full 1,566-school realignment with enrollment; 1 PDF = 1,368 T&F schools with codes/districts.
17. **Published rate limits** — none published; robots disallow list quoted below.
18. **Known blocks** — `Disallow: /files/` (plus `/archives/`, `/resources/`, `/views/`, `/site/`, …) at `samples/robots-www.uiltexas.org.txt`.
19. **Cross-source join keys** — UIL 4-letter school code; school name; ISD name in the rank-order PDF.
20. **Estimated marginal coverage** — the best identifier and class-alignment surface in the batch (1,368 T&F codes; 1,566-school universe), but results are shallow (two tables, no grade) and file access is robots-restricted; Texas MileSplit (`tx.milesplit.com`) exists but is **not** linked by UIL.
21. **Implementation recommendation** — **CONDITIONAL** (pending the owner's decision on `/files/`).

**Linkage quote (UIL's published stats partner is MaxPreps, not MileSplit):**
```html
<p class="center"><img width=130px src="https://www.uiltexas.org/images/site/maxPrepsLogo.png" alt="MaxPreps">
The UIL and MaxPreps.com have teamed up to make results, records, team information and stats from UIL sports available using MaxPrep's sports information system.
<a href="https://www.uiltexas.org/athletics/uil-maxpreps">Learn how coaches and fans can participate.
```
Grep over all captured UIL pages (`samples/tx-*.html`) for `athletic.net|anet.live|milesplit` returns
**no hits**; `tx.milesplit.com` was verified to exist independently (`samples/tx-milesplit-home.html`).

---

## Cross-cutting findings

1. **MileSplit is the de-facto result plane across this region** — AR (calendar + entries + "Letter of
   Partnership"), KY (official registration provider + live results), LA (coach onboarding + MeetPro),
   OK (strategic partner, 2024-01-04), TN (onboarding PDF). The two exceptions are **AL**, where the
   timing provider pipes live results to `anet.live` (Athletic.net), and **TX**, where UIL's partner is
   MaxPreps.
2. **The association member list is the cheap identity anchor** — four states publish it machine-readably
   in one request (KY 290 ids, OK 482 names, TN 456 ids, TX 1,368 codes / 1,566 schools). AL and MS
   publish nothing anonymously reachable; LA publishes it only as a font-encoded PDF.
3. **HY-TEK Meet Manager output is the interchange format** for LA (state PDF), TN (championship PDFs)
   and OK via MeetPro (same columnar layout); the repo's existing HY-TEK parser family applies.
4. **Grade evidence is available at the association layer** in three states: OK (`YR` per athlete),
   TN (`Year` 9–12), LA (`Year`, though blank in the small-class section), plus classification schemes
   by enrollment in TN/OK/TX.
5. **Blocked-by-policy hosts in this batch:** `www.ahsaa.com` (robots `Disallow: /`), `lhsaaonline.org`
   (robots `Disallow: /`), `misshsaa.com` (HTTP 403 WAF). `www.uiltexas.org` is crawlable except `/files/`.
6. **No CAPTCHA was encountered live** except Mississippi's site-shield; every other host answered
   anonymous `curl` immediately (0.1–7 s per request in these runs).

## Open questions

1. **TX `/files/` decision** — treat UIL's published PDFs as in-scope or restrict the collector to the
   HTML index pages (which only link them)?
2. **AL** — is there any AHSAA-published membership/classification file on a host whose robots allows
   anonymous access, or must the school universe come from the state department of education?
3. **LA** — decode the LHSAA subset-font encoding to extract the coaches directory and the member count.
4. **MS** — is the 403 a global site-shield or agent/IP-scoped? Which company times MHSAA state meets?
5. **KY/TN** — the ArbiterLive and tssaasports per-school roster payloads are behind SPAs; the JSON
   endpoints were deliberately not guessed. A browser pass would settle the coach/roster fields.
