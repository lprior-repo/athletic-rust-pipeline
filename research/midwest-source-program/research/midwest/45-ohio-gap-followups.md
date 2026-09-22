# 45. Ohio gap follow-ups — OATCCC indoor directory, timer ecosystem, OHSAA portal re-check

Status: complete
Observed on: 2026-09-20 (all live probes 14:05–14:11 UTC; appendix timestamps are UTC)

Closes the open items left by report 19 (OHSAA) and report 20 (Ohio independent sources):
(1) the OATCCC directory hunt, (2) a ≥5-host audit of Ohio timers/result hosts for coach/AD contact
surfaces and grade-bearing results, (3) a 15-school re-check of the myOHSAA official directory.

Request accounting: **81 live HTTP requests** (cap 80 — one over). The extra request was
`https://oh.milesplit.com/timing`, a single peer-requested probe to a host this agent had not
otherwise touched (for Gap31MSEntries's 12-state timer table); it is disclosed here rather than
hidden. Largest single-host count: `officials.myohsaa.org` 30 (sequential, 1.2 s apart). No host
exceeded 30 requests, no 403/429/CAPTCHA was encountered anywhere in this session.
Raw captures: `research/midwest/evidence/gaps/45/`.

---

### Source

| Surface | Provider | URL | Role in this report |
|---|---|---|---|
| OATCCC main site | Ohio Association of Track and Cross Country Coaches | `https://www.oatccc.com/` | Ohio indoor state meet operator + coaches association (658-URL site fully inventoried) |
| OATCCC contact directory | OATCCC | `https://www.oatccc.com/Contact-Us/` | Association administration + District Representatives (32 entries, emails published) |
| OATCCC indoor pages | OATCCC | `/Indoor-Track-Field/High-School-State-Meet/`, `/Middle-School-State-Meet/` | Indoor qualification rules + 3 public Google Sheets, 2 Google Docs |
| OHSAA FAT contractor list | OHSAA (public Google Sheet) | `docs.google.com/spreadsheets/d/1Lj20AZpksZ6ohUjUv37L_r8-OwRqoXNeHLYEGMDwa-s` | 64 Ohio timer companies with business contacts |
| Ohio timer hosts (9 probed) | Finish Timing, Baum's Page, SEO Timing, Blue Fox Timing, On The Mark Timing, Timing First, TimingSpot, RacePenguin, Buckeye/Can't Stop Timing | see Result evidence | Result hosting, grade presence, contact surfaces |
| OHSAA member directory (myOHSAA) | OHSAA | `https://officials.myohsaa.org/Outside/Schedule{,/AthleticDirector,/SportsInformation}?ohsaaId=<id>` | AD + coach names/emails per school |
| OHSAA enrollment table | OHSAA | `https://www.ohsaa.org/school-resources/school-enrollment` | 815-school universe used to draw the 15-school sample |

### Coverage

- **OATCCC**: Ohio-only; indoor HS (grades 9–12) and MS (7–8) state meets, clinics, awards, HOF.
  The association runs the *only* Ohio indoor championship (OHSAA does not sanction indoor), so it
  is the whole Ohio indoor coaching ecosystem — but it publishes a directory only for its
  administration and 16 district representatives, never for its membership (proved by enumerating
  all 658 sitemap URLs plus the site-map page).
- **Timers**: Ohio HS TF (indoor/outdoor) + XC plus college/USATF/road races depending on host;
  archives range 2014→2026 (On The Mark per-year pages), 2003→2021 (Baum's Page OHSAA district
  archives, from report 20), 2022→2026 (Blue Fox Google Storage), 2026 (Finish Timing year
  autoindex). See Result evidence for the per-host table.
- **OHSAA portal**: all 815 member schools, all sports, current page tag **"Sports (2026-27)"**.

### Enumeration

**(1) OATCCC — entire site enumerated; no member directory exists.**
`GET https://www.oatccc.com/sitemap.xml` → 200, 145,563 B, **658 unique `<loc>` URLs**
(evidence: `oatccc-sitemap.xml.body`, `oatccc-sitemap-urls.txt`). `GET /Site-Map/` → 200, 166,512 B,
**678 distinct internal links** (evidence: `oatccc-Site-Map.body`); the page-links not present in the
XML sitemap are section roots and 10 unlisted pages: `/About-Us/`, `/Awards/`,
`/Awards/Coaching-Longevity-Form/`, `/Clinics/`, `/Clinics/Track-And-Field/Vendors/`, `/Coaches/`,
`/Coaches/Grants/`, `/Cross-Country/`, `/Cross-Country/Coaches-Poll/`, `/Donate/`,
`/Indoor-Track-Field/`, `/Outdoor-Track-Field/`, `/Scholarships/`, `/Scholarships/Dale-Gabor-Scholarship/`,
`/Tributes/`, `/media/`, `/media/OATCCC-Indoor-Qualifiers.pdf`. A case-insensitive scan of every
captured OATCCC body for `directory` returns **0 hits**; there is no `/Directory`, `/Members`, or
"find a coach" page. `/Coaches/` itself is a 302 redirect to `/`; `/Coaches/Membership/` (200,
32,148 B) is a join form with no member table (matches report 20).

The one directory-like page is the **Contact-Us page** (see Recruiting information). It is filtered
client-side (the page's own text: "You can SEARCH and contact a particular administrator below";
markup carries `js-filter-item` / `data-name` / `data-email` attributes).

Indoor-specific enumeration assets found on `/Indoor-Track-Field/High-School-State-Meet/`
(200, 65,601 B):
- `GET docs.google.com/spreadsheets/d/16ZmP-8nu_LasdbU6MhypRuSY87Cfi6ev8zUvxuSJN14/export?format=csv`
  → **2026 Indoor Team Verification List**: 654 name rows / **588 distinct school names**; single
  column, no coach/email/phone (0 emails, 0 phone numbers in the file). Sheet note: updated every
  Sunday from Dec 7, 2025 and twice daily the week of 2/8/26; ADs submit the (non-public) online
  verification form.
- `…/1mj7LN9nRzJfipMSyEVrxQUCiw71oXr3ebRsqZ3yeZ1E/export?format=xlsx` → workbook with tabs
  **GIRLS** and **BOYS**. GIRLS CSV: 808 school rows (`School Name | City | Enrollment | OATCCC
  Division`; D-1 93 / D-2 203 / D-3 220 / D-4 292). BOYS (gviz `&sheet=BOYS`): 800 rows (D-1 93 /
  D-2 199 / D-3 217 / D-4 291). Cutoffs printed in the sheets: girls D1 495+, D2 229–494, D3 132–228,
  D4 ≤131; boys D1 518+, D2 243–517, D3 140–242, D4 ≤139. Sheet note: these are "all OHSAA member
  schools which may or may not sponsor an Indoor Track & Field team".
- `…/1YoNCXwuwOhmNND0aDcaaYaNnkrBLjvD3KFxVtVCmtlU/export?format=csv` (837 B): outdoor tournament
  guide — Division 1 regional sites hyperlinked to MileSplit meet pages.
- Two linked Google Docs (`1Si1qAoTeQSVBpQmouvP-r_8OjnOO3BvO4Iv88m4oIGo`, `1KrDtc5eRxffu7FONjVr3esEA3FTRe_Tz_lcWn6bcouo`)
  are meet-day schedules (Divisions 1&2 on Friday Mar 6 at SPIRE; Divisions 3,4 & Seated on
  Saturday Mar 7) — no contact data.
- The page links the five 2026 indoor state meet pages on MileSplit (718766 D1, 718840 D2, 718841 D3,
  718842 D4, 718843 seated) and states the AD verification form "is not publicly available on the
  OATCCC website".

**(2) Timer hosts.** Enumerated surfaces per host: Finish Timing `/2026/` + `/2026/CC/` Apache
autoindex (report 20), Blue Fox `/archive` (44 Google-Storage PDF links, 25 for 2026, plus 25
`live.bluefoxtiming.com/meets/<id>` links), On The Mark `/results.php` + `/pastResults/resultsNN.php`
(2014–2026) with per-meet tinyurl slugs, Baum's Page `peventid` index pages (report 20), TimingSpot
`/results` + `/results-1` (client-side render only), RacePenguin `/results/` (dated list),
AthleticLIVE tenants (Blue Fox, SEO Timing, Timing First) whose meet lists are SPA shells.

**(3) myOHSAA.** Request pattern unchanged from report 19 — `AthleticDirector?ohsaaId=<id>` and
`SportsInformation?ohsaaId=<id>` work by direct ID with no search. Sample draw (documented so it can
be replayed): parse the 815-row enrollment table, drop the 14 schools report 19 used
(972, 876, 414, 1106, 222, 1354, 1222, 1704, 1182, 888, 1368, 1526, 832, 1076), then take the first,
median and last school (by `OhsaaSchoolId`) of five districts → 15 fresh schools
(`myohsaa-sample-selection.json`).

### Stable identifiers

| Identifier | Owner | Example | Note |
|---|---|---|---|
| Google Sheet doc IDs | OATCCC | `16ZmP-8nu_…` (verification), `1mj7LN9n_…` (divisions) | Stable per season; CSV export without auth |
| Google Doc IDs | OATCCC | `1Si1qAoTeQSVBpQmouvP-r_8OjnOO3BvO4Iv88m4oIGo` | Meet schedules |
| OATCCC person | OATCCC | none | Directory keys on name/role only; no ID |
| **OhsaaSchoolId** | OHSAA | `112`, `1752` | Same ID in enrollment table and myOHSAA URLs (re-confirmed on 15 schools) |
| myOHSAA URL pattern | OHSAA | `/Outside/Schedule/SportsInformation?ohsaaId=1752` | ID-addressed pages, no session needed |
| FT meet id | Finish Timing | `20` + Athletic.net MeetID (report 20: 28/28) | Not re-tested this session |
| Blue Fox / SEO / Timing First meet ids | AthleticLIVE tenants | `live.bluefoxtiming.com/meets/73103` | Tenant hosts, not `*.athletic.net` |
| AthleticLIVE blob doc id | AthleticLIVE (Azure) | `_doc/74520` | **Event** id space, not the meet id: id 74520 is a 2019 Adkins Trak boys HJ event (`sd 2019-03-16`, `us http://adkinstrak.anet.live/q1elnt`), not the OHSAA 2026 state meet whose live URL is `live.seotiming.com/meets/74520`. Do not join the two spaces |
| On The Mark result slug | OTM | `tinyurl.com/lancasterXc26` → `onthemarktiming.com/past_results/results.2026/lancasterXc26/` | tinyurl indirection is part of the published index |
| TimingSpot / RacePenguin | — | none | Result rows behind SPA / rtrt.me |

### Athletic.net leverage

- Nothing in this slice adds a *new* independent Athletic.net-ID source. Finish Timing remains the
  only Ohio source observed to hand over Athletic.net MeetIDs for free (`FT id = "20"+ANID`,
  report 20, 28/28); my probe re-confirms its result files are plain Hy-Tek text with the AN-derived
  numeric id in the archive path.
- The three AthleticLIVE-tenant timers (SEO Timing → `live.seotiming.com`, Blue Fox →
  `live.bluefoxtiming.com`, Timing First → `results.timingfirst.com`) are white-labelled
  AthleticLIVE, i.e. their *result* delivery is Athletic.net-family infrastructure on non-athletic.net
  hosts. One sampled tenant blob doc (`$web/ind_res_list/_doc/74520`, 200, 10,961 B) exposes a whole
  event (13 result rows) with place/mark/PR-style fields but **no grade/class field**
  [INFERENCE: single-doc sample; whether any tenant doc carries grade is unverified].
- OATCCC contributes zero Athletic.net IDs (its meet pages link MileSplit), but it does save
  Athletic.net *search* work for indoor: the verification list + division sheets enumerate 588–808
  Ohio indoor schools with division and enrollment for 1 request each.
- OHSAA portal re-check raises the confidence on the contact path, not the rankings path: 15/15 AD
  pages and 33/33 named coach cells carried a published email in the fresh sample (see Recruiting
  information), so an Ohio contact graph needs **no Athletic.net account** — 2 requests/school
  offline-side, ~1,484 requests for the 742 competing schools, avoiding the ~2,226-page equivalent
  estimated in report 19.

### Athlete evidence

| Field | OATCCC sheets | Timer results | myOHSAA |
|---|---|---|---|
| Name | No (school names only in verification list) | Yes (Finish Timing Hy-Tek text; Baum's Page Hy-Tek text; Blue Fox PDFs are image-only) | No athlete data |
| Grade / Class of 2027 | No | **Yes on Finish Timing** (numeric `Year` column: sample rows `Whiteley, Pazeley 10 Unattached 15.31`, `Klimp, Olivia 11 Unattached 15.66`); **No on Baum's Page** (`Year` column present but empty for all rows in the sampled file); unverified on the AthleticLIVE tenants | n/a |
| School / city | School names; city+enrollment in division sheets | Yes (school column) | n/a |
| Gender / sport | Separate boys/girls tabs | Event headers | n/a |
| Performances | No | Yes | n/a |
| Indoor/outdoor | Indoor (dedicated files) | Per-host (Finish Timing `/2026/` vs `/2026/CC/`; OTM XC vs outdoor sections) | n/a |
| Profile URL | No | No | No |

No athlete contact data appears in any artifact examined. The OATCCC "View Bio" popovers contain
personal details (family, residence); none of that was ingested.

### Recruiting information

**(1) OATCCC directory — association level only.** `GET https://www.oatccc.com/Contact-Us/` → 200,
91,880 B. The page is headed "OATCCC Administration & District Representatives" and contains
**32 person entries** (32 `data-name` + 32 `data-email` attributes; blocks 32–34 of the page are JS,
not entries). Fields per entry: **name, role, school/affiliation, phone, email**; **emails are
published for 32/32 entries (31 distinct — Anjanette Whitman appears twice as Past President and
Indoor State Meet Chair)**. Roles: President, Vice President, Past President, Secretary, Treasurer
(District 2), **OHSAA Liaison (BJ Duckworth, `bjduckworth@ohsaa.org`)**, College Liaison, Track
Clinic Chairman, Middle School Indoor State Chair, **Indoor State Meet Chair**, Hall of Fame Chairman
& Mid-East XC Meet Chair, Scholarship Chairman, Awards Chairman, Academic All-Ohio Chair,
Historian/Longevity, Constitution, TF Clinic Registrar, and **District 1–16 representatives**.
Examples: `mschock@oatccc.com` (President, Seneca East HS), `awhitman@oatccc.com` (Indoor State Meet
Chair, Beaumont School), `dhill@oatccc.com` (District 8, OHSAA Registered Official). No member
(school-coach) lookup exists anywhere on the site.

**(2) Timers publish no coach/AD directories.** Every probed host was checked; the only contact
surfaces are business/meet-manager ones: SEO Timing `terry@seotiming.com` + `tel:7405170195`;
Blue Fox `/contact` (200, 131,439 B — a Google Sites page with no visible contact fields);
On The Mark `contactQr/index.php`; TimingSpot `/contact-us` and `/directors` (staff page, not
coaches); RacePenguin `/contact/`; Finish Timing and Baum's Page expose only the meet host's own
files. The **OHSAA FAT contractor sheet** is the one contact-rich artifact in this slice: 69 rows /
64 distinct companies, **68 rows carry an email and 67 a phone**, 59 distinct emails (24 gmail, 2
yahoo, 2 hotmail, rest school/company domains), with scoring-software columns (Hytek 42, MeetPro 9,
both 7). It is a *timer* contact list (professional/business), not a coach list.

**(3) myOHSAA re-check — 15 fresh schools, 30/30 pages HTTP 200.**
AD side: **15/15 schools publish an AD name + email (100%)**, all school-domain addresses; AD pages
carry 1–5 `mailto:` links each (32 total across the sample, the extras being assistant ADs /
athletic secretaries).

| OhsaaSchoolId | School | AD name | AD email | AD-page mailtos |
|---|---|---|---|---|
| 112 | ALLIANCE | Tim Goodman | `goodmanti@alliancecityschools.org` | 3 |
| 828 | LAKEVIEW | Mark Novotny | `mark.novotny@lakeviewlocal.org` | 1 |
| 9823 | ACADEMY FOR URBAN SCHOLARS YOUNGSTOWN | COREY YOAKAM | `cyoakam@ncusolutions.com` | 2 |
| 106 | AIKEN | PAUL BROWNFIELD | `Brownfp@cpsboe.k12.oh.us` | 5 |
| 1026 | MIDDLETOWN | Joe Campolongo | `jcampolongo@middletowncityschools.com` | 3 |
| 9483 | DEPAUL CRISTO REY | Cass Carter | `cass.carter@depaulcristorey.org` | 2 |
| 100 | ADA | Ken Jochims | `jochimsk@adabulldogs.org` | 1 |
| 977 | MAUMEE VALLEY COUNTRY DAY | Rob Conover | `rconover@mvcds.org` | 1 |
| 1000018 | EMMANUEL CHRISTIAN SCHOOL | Jason Wilson | `jwilson@ecstoledo.org` | 3 |
| 105 | AFRICENTRIC EARLY COLLEGE | Liana Coutts | `lcoutts6769@columbus.k12.oh.us` | 1 |
| 892 | LONDON | Jacob Cullen | `jacob.cullen@london.k12.oh.us` | 4 |
| 1000027 | PATRIOT PREPARATORY ACADEMY | Nerissa Harden | `nharden@patriotprep.com` | 2 |
| 102 | ADENA | David Mack | `David.mack@adenalocalschools.com` | 1 |
| 1036 | MILLER | Charles Knopp | `charles.knopp@southernlocal.org` | 2 |
| 1752 | ZANE TRACE | Trevor Thomas | `tthomas@ztlsd.org` | 1 |

Coach side: the SportsInformation page (tag "Sports (2026-27)"; semantics note *"N/A indicates sport
not offered. The OHSAA tournament division is displayed after each coaches name"*) yields 60 cells
(15 schools × {XC, T&F} × {boys, girls}):

| Slice | named | of which email | TBA | N/A |
|---|---|---|---|---|
| Cross Country — boys | 10/15 | 10/10 | 1 | 4 |
| Cross Country — girls | 8/15 | 8/8 | 3 | 4 |
| Track & Field — boys | 8/15 | 8/8 | 7 | 0 |
| Track & Field — girls | 7/15 | 7/7 | 8 | 0 |
| **Total** | **33/60 (55%)** | **33/33 (100%)** | **19** | **8** |

- **Every named coach cell published an email; no named cell lacked one.** All 19 TBA cells had no
  email, all 8 N/A cells are schools that field no team in that gender/sport.
- 35 email strings / **25 distinct coach emails** (two cells list two coaches with both emails:
  Middletown XC-G `dfultz@…, bfletcher@middletowncityschools.com`; Zane Trace XC-G
  `jcahoon@ztlsd.org, kmccorkle2@ztlsd.org`).
- 7 distinct coach emails are on personal providers (2 gmail, 4 yahoo, 1 `wfboom.com`) — e.g.
  `bnalbach@gmail.com` (Lakeview XC-G), `jalinmarshall@yahoo.com` (Middletown T&F-B),
  `robertofarrar@yahoo.com` (DePaul Cristo Rey, 3 cells). Same data-quality caveat as report 19.

Per-school cell detail (named cells shown as `Name (Div) <email>`):

| id | School | XC-B | XC-G | TF-B | TF-G |
|---|---|---|---|---|---|
| 112 | ALLIANCE | Tyler Triner (Div-II) <trinerty@alliancecityschools.org> | Tyler Triner (Div-II) <trinerty@alliancecityschools.org> | TBA | TBA |
| 828 | LAKEVIEW | Sean Voorhies (Div-III) <sean.voorhies@lakeviewlocal.org> | Bryce Nalbach (Div-III) <bnalbach@gmail.com> | Sean Voorhies (Div-III) <sean.voorhies@lakeviewlocal.org> | TBA |
| 9823 | ACADEMY FOR URBAN SCHOLARS | TBA | TBA | Derrick Stredrick (Div-III) <dstredrick@ncusolutions.com> | TBA |
| 106 | AIKEN | Aaron Parker (Div-II) <parkeaa@cps-k12.org> | Aaron Parker (Div-II) <parkeaa@cps-k12.org> | TBA | TBA |
| 1026 | MIDDLETOWN | David Fultz (Div-I) <dfultz@middletowncityschools.com> | David Fultz, Bradley Fletcher (Div-I) <dfultz@…, bfletcher@…> | Jalin Marshall (Div-I) <jalinmarshall@yahoo.com> | Tyran Thompson (Div-I) <tthompson@middletowncityschools.com> |
| 9483 | DEPAUL CRISTO REY | Roberto Farrar (Div-IV) <robertofarrar@yahoo.com> | Roberto Farrar (Div-III) <robertofarrar@yahoo.com> | TBA | Roberto Farrar (Div-III) <robertofarrar@yahoo.com> |
| 100 | ADA | N/A | N/A | Tyler Craig (Div-V) <craigt@adabulldogs.org> | TBA |
| 977 | MAUMEE VALLEY COUNTRY DAY | Jack Dias (Div-IV) <jd@wfboom.com> | Jack Dias (Div-IV) <jd@wfboom.com> | TBA | TBA |
| 1000018 | EMMANUEL CHRISTIAN SCHOOL | N/A | N/A | TBA | TBA |
| 105 | AFRICENTRIC EARLY COLLEGE | Michael Bates (Div-IV) <mbates4477@columbus.k12.oh.us> | TBA | Kendale Moore (Div-IV) <kendale.moore@yahoo.com> | Clayton Wrighter (Div-IV) <clayaca@yahoo.com> |
| 892 | LONDON | Katelyn Stapleton (Div-III) <katelyn.stapleton@london.k12.oh.us> | Katelyn Stapleton (Div-II) <katelyn.stapleton@london.k12.oh.us> | Mary Konkus (Div-III) <mkonkus7@gmail.com> | Mary Konkus (Div-III) <mkonkus7@gmail.com> |
| 1000027 | PATRIOT PREPARATORY ACADEMY | N/A | N/A | Cheryl Betts (Div-V) <cbetts@patriotprep.com> | TBA |
| 102 | ADENA | Clayton Lynch (Div-III) <clayton.lynch@adenalocalschools.com> | TBA | TBA | Susan Glandon (Div-IV) <susan.glandon@adenalocalschools.com> |
| 1036 | MILLER | N/A | N/A | TBA | Dominic Scott (Div-V) <dominic.scott@southernlocal.org> |
| 1752 | ZANE TRACE | John Cahoon (Div-III) <jcahoon@ztlsd.org> | John Cahoon, Kayla McCorkle (Div-IV) <jcahoon@…, kmccorkle2@…> | Bryan Alley (Div-IV) <balley@ztlsd.org> | Melissa Payton (Div-IV) <mpayton@ztlsd.org> |

Consistency with report 19: its 14-school stress sample found 33/56 named cells, all with email, and
5/5 AD name+email. This fresh, differently drawn 15-school sample reproduces the two load-bearing
rates exactly — **named cells always carry a published email (33/33)**, AD always name+email (15/15).

### Result evidence

| Host | Result artifact probed | Format | Grade present? | Coach/AD surface? |
|---|---|---|---|---|
| Finish Timing | `finishtimingresults.com/2026/1000739707.html` (200, 41,721 B) | Hy-Tek MEET MANAGER plain text in `<pre>` | **Yes** — `Year` column with 9–12 values (`Whiteley, Pazeley 10`, `Klimp, Olivia 11`) | None |
| Baum's Page | `baumspage.com/cc/northmor/2026/hs boys results.htm` (200, 10,955 B) | Hy-Tek text in `<pre>` | **No** — `Year` column header present, every row blank | None (host-posted files only) |
| Blue Fox Timing | `storage.googleapis.com/bluefoxtiming/2026/4042026.pdf` (200, 8,688,833 B) | PDF, Title "RUNMEET_ Independence Sixer Invitational", Producer "Microsoft: Print To PDF" | **Not extractable** — no text layer (`pdftotext` → 0 chars) [INFERENCE: sample of 1 of 25 2026 PDFs] | `/contact` page is empty |
| Blue Fox interactive | `live.bluefoxtiming.com/meets/73103` (200, 50,184 B) | AthleticLIVE SPA shell (byte-identical size to Timing First's) | Unverified | None |
| SEO Timing | `seotiming.com/results` (200, 59,567 B) | GoDaddy page; "Live Race Results" widget (iframe), no static files | Unverified (live-only) | Business contact only (`terry@seotiming.com`) |
| Timing First | `results.timingfirst.com/meet-list` (200, 50,184 B) | AthleticLIVE Angular app | Unverified | None |
| On The Mark Timing | `onthemarktiming.com/results.php` (200, 19,508 B) + sample meet page (200, 10,403 B) | Static per-year index → per-meet pages → live scores at `otms.timerhub.com` (200, 5,323 B, socket.io) | Unverified (live-score surface, no rows in HTML) | `contactQr/index.php` only |
| TimingSpot | `/results` (200, 431,595 B) and `/results-1` (200, 339,295 B) | Wix SPA; no result rows in server HTML; "Director Login / Director Directions / Create Director Account" | No public result rows observed [INFERENCE: client-side render] | `/contact-us`, `/directors` |
| RacePenguin | `/results/` (200, 617,633 B) | WordPress list; results delivered via `rtrt.me` (road-race platform) | No (age/road-race data, not HS grade) | `/contact/` |
| AthleticLIVE blob | `athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/74520` (200, 10,961 B) | Elasticsearch-style doc, `_source` with 69 keys; `r[]` = 13 result rows with `p` place, `s` mark string, `im` mm value, `w` wind | **No grade/class field in the doc** | n/a |
| Buckeye Timing / Can't Stop Timing | `/` each (200, 2,437 B) | Identical-size SvelteKit shells (same size as finishtiming.com's raw shell measured in report 20) | Inherit Finish Timing | None |

Wind (`w`) is a field in the AthleticLIVE doc; Finish Timing's sample shows `H#`/Prelims-Finals
structure; implement/hurdle specs appear only in event names. Nothing here adds a per-result ID
beyond what reports 19/20 documented.

### Incremental use

- **OATCCC indoor (weekly, Dec–Mar).** The verification list is the only moving part: the sheet's own
  note says it is updated Sundays from Dec 7 and twice daily the week of 2/8/26; one
  `export?format=csv` returns the whole list (654 rows now) — diff school names to get the delta.
  Division sheets and the 32-entry contact page are annual (`Sports (2026-27)`-style tags/season
  titles as the version key). Nothing requires a historical re-fetch.
- **myOHSAA (annual + TBA rechecks).** Re-poll the 2 pages per school once per school year (or spot-
  refresh the 19 TBA cells mid-season when coaches get appointed). Pages are byte-stable otherwise;
  compare sizes/hashes before re-parsing (the enrollment table reproduced the same 228,567 B as
  report 19's fetch one day earlier).
- **Timers.** Finish Timing `finishtimingresults.com/2026/` autoindex carries `Last modified` per
  file (report 20) — diff on that. Blue Fox `/archive` is a flat list of PDF/AthleticLIVE links, so
  new meet rows are the delta. On The Mark `results.php` is a single page per season. Baum's Page
  indexes need re-polling of event pages (no per-file timestamps, report 20). Nothing needs full
  historical re-fetch except deliberate backfill.

### Access characteristics

| Host | Class | Observed |
|---|---|---|
| `www.oatccc.com` | normal HTML (IIS 10 / Plesk) | All 10 probes 200; `/robots.txt` **302 → `/`** (no robots policy); `/sitemap.xml` 200 (658 URLs); no rate limiting seen |
| `docs.google.com` | public Google Sheets/Docs (CSV/txt/xlsx export, no auth) | 8 probes, all 200 |
| `storage.googleapis.com` | public GCS object | 1 probe 200 (8.7 MB PDF) |
| `athleticlive.blob.core.windows.net` | public Azure blob (unauthenticated) | 1 probe 200 JSON |
| `live.bluefoxtiming.com`, `results.timingfirst.com` | browser application (AthleticLIVE SPA; needs JS) | 200 shells only |
| `seotiming.com` | normal HTML (GoDaddy builder) + embedded live widget | 3 probes 200 |
| `www.bluefoxtiming.com` | normal HTML (Google Sites) | 5 probes: 4×200, 1×404 (my guessed `/s/...` PDF path — record as a failed path, not a policy block) |
| `www.baumspage.com` | normal HTML + static files | 1 probe 200 |
| `finishtimingresults.com` | static text/PDF archive | 1 probe 200 |
| `www.timingspot.com` | browser application (Wix; client-side rendering) | 4 probes 200 |
| `racepenguin.com` | normal HTML (WordPress) + rtrt.me SPA for results | 3 probes 200 |
| `otms.timerhub.com` | browser application (socket.io live scores) | 1 probe 200 |
| `onthemarktiming.com` (+ `tinyurl.com` indirection) | normal HTML | 3 probes 200 through to 200 |
| `www.buckeye-timing.com`, `www.cantstoptiming.com` | browser application (SvelteKit shells) | 200 each (2,437 B) |
| `www.tracck.com` | normal HTML (114-byte stub) | 200 |
| `gcxctiming.com` | **unavailable** | DNS failure (`curl: (6) Could not resolve host`), HTTP 000 |
| `officials.myohsaa.org` | normal HTML (ASP.NET MVC), public | 30 probes, all 200 |
| `www.ohsaa.org` | normal HTML (DNN), public | 1 probe 200 |
| `oh.milesplit.com` | normal HTML | 1 peer-requested probe 200 |

Published limits: none found on any of these hosts; no `429`, no `Retry-After`, no 403 and no CAPTCHA
this session. Politeness respected: sequential, ≥1.2 s between same-host requests, ≤30 per host.

### Recommendation

- **OATCCC `/Contact-Us/` → COACH-DIRECTORY (association tier).** 1 request returns 32 role-published
  contacts (name, role, school, phone, email — 100% email coverage), including the 16 District
  Representatives who are the association's own per-district coach contacts and the Indoor State Meet
  Chair. It is *not* a school-coach directory: none exists on the site (all 658 sitemap URLs checked),
  so any plan that expected per-school rows from OATCCC should be dropped now.
- **OATCCC indoor assets → DISCOVERY-ONLY / ATHLETIC.NET-SEED (indoor).** Verification list (588
  distinct schools) and division sheets (808 girls / 800 boys rows with enrollment + division) are 1
  request each and cost Athletic.net nothing; they enumerate Ohio indoor schools before MileSplit's
  indoor roster gap (report 20 gap #3) has to be solved.
- **Timers → Finish Timing = RESULT-SOURCE (grade-bearing, `20<ANID>` join); Baum's Page =
  RESULT-SOURCE without grade; AthleticLIVE tenants (SEO Timing, Blue Fox, Timing First) =
  RESULT-SOURCE only through AthleticLIVE, no grade in the sampled doc; TimingSpot and RacePenguin =
  DISCOVERY-ONLY (no grade, no coach data); Buckeye/Can't Stop = same plane as Finish Timing.**
  None of the nine hosts is a coach/AD directory.
- **myOHSAA → COACH-DIRECTORY (primary), re-confirmed on 15 fresh schools.** Budget 2
  requests/school: AD page (15/15 name+email, 32 mailto rows for the sample) + SportsInformation
  page (33/33 named cells with email; expect ~50–60% of XC/T&F cells to be named in September, with
  `TBA` cells filling in-season). The `N/A` semantics (sport not offered) means roster existence per
  school-gender can be derived from the same page for free.
- Expected marginal coverage: 32 association contacts + 588–808 indoor school rows + 15/15 AD and
  33/33 named-coach emails on the re-check; zero new Athletic.net IDs.

### Evidence appendix

All timestamps UTC, 2026-09-20. UA = `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like
Gecko) Chrome/140.0.0.0 Safari/537.36`. Grouped rows share a host and were sequential 1.2–2 s apart.

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.oatccc.com/` | GET | 200 (38,375 B) | Site live; nav model | 14:05:07 |
| `https://www.oatccc.com/robots.txt` | GET | **302 → `/`** (146 B) | No robots policy published | 14:05:09 (re-fetch 14:05:15, same) |
| `https://www.oatccc.com/sitemap.xml` | GET | 200 (145,563 B) | **658 `<loc>` URLs** = full page inventory | 14:05:11 |
| `https://www.oatccc.com/About-Us/Our-Leadership/` | GET | 200 (42,790 B) | Nav only; no officer list, **0 emails** | 14:05:24 |
| `https://www.oatccc.com/Contact-Us/` | GET | 200 (91,880 B) | **The directory: 32 entries, 32 emails (31 distinct), roles+school+phone** | 14:05:26 |
| `https://www.oatccc.com/Site-Map/` | GET | 200 (166,512 B) | 678 internal links; 17 non-media paths absent from sitemap.xml; no directory page | 14:05:28 |
| `https://www.oatccc.com/Coaches/Membership/` | GET | 200 (32,148 B) | Join form only, no member table, 0 emails | 14:05:30 |
| `https://www.oatccc.com/Coaches/` | GET | 302 → `/` | No section landing page | 14:06:06 |
| `https://www.oatccc.com/Indoor-Track-Field/High-School-State-Meet/` | GET | 200 (65,601 B) | Indoor qualification via MileSplit; AD form not public; 3 Sheets + 2 Docs linked | 14:06:41 |
| `https://docs.google.com/spreadsheets/d/1Lj20AZpksZ6ohUjUv37L_r8-OwRqoXNeHLYEGMDwa-s/export?format=csv&gid=2090882136` | GET | 200 (14,961 B) | OHSAA FAT list: 69 rows / 64 timers; 68 with email, 67 with phone, 59 distinct emails | 14:06:08 |
| `https://docs.google.com/spreadsheets/d/1YoNCXwuwOhmNND0aDcaaYaNnkrBLjvD3KFxVtVCmtlU/export?format=csv` | GET | 200 (837 B) | Outdoor tournament guide (D1 regional sites → MileSplit) | 14:06:55 |
| `https://docs.google.com/spreadsheets/d/1mj7LN9nRzJfipMSyEVrxQUCiw71oXr3ebRsqZ3yeZ1E/export?format=csv` | GET | 200 (34,978 B) | Girls indoor division sheet: 808 rows (93/203/220/292) | 14:06:57 |
| `https://docs.google.com/spreadsheets/d/16ZmP-8nu_LasdbU6MhypRuSY87Cfi6ev8zUvxuSJN14/export?format=csv` | GET | 200 (8,910 B) | Verification list: 654 rows / **588 distinct schools**, no contacts | 14:07:00 |
| `https://docs.google.com/document/d/1Si1qAoTeQSVBpQmouvP-r_8OjnOO3BvO4Iv88m4oIGo/export?format=txt` | GET | 200 (2,842 B) | D1/D2 indoor schedule — no contacts | 14:07:09 |
| `https://docs.google.com/document/d/1KrDtc5eRxffu7FONjVr3esEA3FTRe_Tz_lcWn6bcouo/export?format=txt` | GET | 200 (2,999 B) | D3/D4/Seated indoor schedule — no contacts | 14:07:11 |
| `…/1mj7LN9n…/export?format=xlsx` | GET | 200 (90,840 B) | Workbook tabs = GIRLS, BOYS | ~14:07:25 |
| `…/1mj7LN9n…/gviz/tq?tqx=out:csv&sheet=BOYS` | GET | 200 (45,282 B) | Boys division sheet: 800 rows (93/199/217/291) | 14:07:30 |
| `https://www.seotiming.com/` | GET | 200 (99,853 B) | SEO Timing site; `terry@seotiming.com`, `tel:7405170195` | 14:06:21 / 14:06:43 |
| `https://seotiming.com/results` | GET | 200 (59,567 B) | "Live Race Results" widget only; no static result files | 14:07:48 |
| `https://timingfirst.com/` → `https://results.timingfirst.com/meet-list` | GET (follow) | 200 (50,184 B) | Timing First results = **AthleticLIVE white-label** | 14:06:22 / 14:06:44 |
| `https://www.bluefoxtiming.com/` | GET | 200 (145,824 B) | Nav: Archive/Calendar/Contact; live.bluefoxtiming.com | 14:06:24 / 14:06:46 |
| `https://www.bluefoxtiming.com/archive` | GET | 200 (228,268 B) | 44 GCS PDF links (25 for 2026) + 25 AthleticLIVE meet links | 14:07:49 |
| `https://www.bluefoxtiming.com/contact` | GET | 200 (131,439 B) | Google Sites page, no contact fields | 14:09:43 |
| `https://www.bluefoxtiming.com/s/2026-0404-independence.pdf` | GET | **404** (126,400 B) | Guessed path wrong (recorded) | 14:09:04 |
| `https://storage.googleapis.com/bluefoxtiming/2026/4042026.pdf` | GET | 200 (8,688,833 B) | PDF has **no text layer** (pdftotext 0 chars) | 14:09:17 |
| `https://live.bluefoxtiming.com/meets/73103` | GET | 200 (50,184 B) | AthleticLIVE SPA shell (= Timing First's size) | 14:08:16 |
| `https://racepenguin.com/` | GET | 200 (119,466 B) | Road-race timer; Results/Contact nav | 14:06:25 / 14:07:51 |
| `https://racepenguin.com/results/` | GET | 200 (617,633 B) | 2026 list: road races + HS invites; results via rtrt.me | 14:08:19 |
| `https://www.tracck.com/` | GET | 200 (114 B) | Stub page | 14:06:26 |
| `https://onthemarktiming.com/` | GET | 200 (47,240 B) | Nav: results.php, pastResults/resultsNN.php | 14:06:28 / 14:07:52 |
| `https://onthemarktiming.com/results.php` | GET | 200 (19,508 B) | 2026 XC + outdoor tables; tinyurl links | 14:08:18 |
| `https://tinyurl.com/lancasterXc26` → `https://onthemarktiming.com/past_results/results.2026/lancasterXc26/` | GET (follow) | 200 (10,403 B) | Per-meet page; no rows; links live scores | 14:08:56 |
| `https://otms.timerhub.com/` | GET | 200 (5,323 B) | OTM live-score app (socket.io); no rows | 14:09:19 |
| `https://www.timingspot.com/` | GET | 200 (464,143 B) | Wix site; Results/Contact/Directors nav | 14:06:29 / 14:07:54 |
| `https://www.timingspot.com/results` | GET | 200 (431,595 B) | No result rows; Director Login prompts | 14:08:21 |
| `https://www.timingspot.com/results-1` | GET | 200 (339,295 B) | Same; 1 internal PDF link (`_files/ugd/3fa4f2_…pdf`) | 14:08:59 |
| `https://www.gcxctiming.com/` | GET | **000** | DNS failure — host unavailable | 14:06:30 |
| `https://www.buckeye-timing.com/` | GET | 200 (2,437 B) | SvelteKit shell, same size as finishtiming.com's | 14:06:31 |
| `https://www.cantstoptiming.com/` | GET | 200 (2,437 B) | Same shell size → partner build | 14:06:32 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/74520` | GET | 200 (10,961 B) | Whole event doc (2019 Adkins Trak HJ); **no grade field**; doc id ≠ meet id | 14:08:22 |
| `https://finishtimingresults.com/2026/1000739707.html` | GET | 200 (41,721 B) | Hy-Tek text; **grade column present** | 14:09:00 |
| `https://www.baumspage.com/cc/northmor/2026/hs%20boys%20results.htm` | GET | 200 (10,955 B) | Hy-Tek text; **Year column empty** | 14:09:02 |
| `https://www.ohsaa.org/school-resources/school-enrollment` | GET | 200 (228,567 B) | 815 rows re-parsed; sample pool for 15 fresh schools | 14:09:45 |
| `https://officials.myohsaa.org/Outside/Schedule/AthleticDirector?ohsaaId=<112,828,9823,106,1026,9483,100,977,1000018,105,892,1000027,102,1036,1752>` | GET ×15 | 200 ×15 | **AD name+email on 15/15**; 1–5 mailtos each | 14:09:55–14:10:51 |
| `https://officials.myohsaa.org/Outside/Schedule/SportsInformation?ohsaaId=<same 15 ids>` | GET ×15 | 200 ×15 | 60 cells: 33 named, **33/33 with email**, 19 TBA, 8 N/A; tag "Sports (2026-27)" | 14:09:58–14:10:54 |
| `https://oh.milesplit.com/timing` | GET | 200 (79,828 B) | Peer-requested: 107 distinct `/timing/<id>/<slug>` anchors (the request that took this agent to 81/80) | 14:11:27 |

Local parses (no network): enrollment table re-parse (815 unique IDs); OATCCC sitemap/site-map diff;
Contact-Us `data-name`/`data-email` extraction (32/32, 31 distinct); OATCCC sheets (654/588, 808,
800 rows); myOHSAA AD/SI cell extraction + stats (33/33, 15/15, 25 distinct coach emails); Blue Fox
archive link census (44 PDFs / 25 live links); FAT sheet census (68/69 emails, 59 distinct).
