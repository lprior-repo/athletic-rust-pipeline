# 29. Official coach-contact graph (12-state AD + track/XC coach directory map)

Status: partial — MO and IN have no verified official association-level or school-level contact surface I could collect within the access rules (both documented as blocked/no-data, not skipped); coach-level (sport-specific) contacts are proven only for WI, IL, OH plus opt-in portions of IA/SD; MN/MI/KS/MO/IN expose AD only (or nothing), ND/NE expose names without email; MO/IN tier-3 (district staff directory) sampling not completed.
Observed on: 2026-09-19

Output data: `data/coach-contacts.csv` — 2,298 rows, 10 states, 874 distinct schools (1,347 coach rows of which 51 carry email; 951 AD rows of which 573 carry email), provenance columns on every row (built by `tools/a29-coach/build_csv.py` from the raw capture files in `tools/a29-coach/`). MO and IN contribute zero rows because no official source published collectible contacts.

## Source

Per-state winner selected by the assignment hierarchy (state association → official school athletics directory → district staff directory → other official school source). Every winner below was fetched and parsed; "no data" means the page was fetched and the contact fields were absent, not that the page was unreachable.

| # | State | Winner (tier) | Fill pattern | AD | AD email | Sport coach | Coach email |
|---|---|---|---|---|---|---|---|
| 1 | WI | WIAA school detail (1) | `GET schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=<n>` | yes | yes | **yes** (table: Sport/Name/Role/Email) | **yes** |
| 2 | MN | MSHSL school page (1) | `GET mshsl.org/schools/<slug>` | yes ("Activities Director") | yes (Cloudflare-obfuscated hex, decodable) | no | — |
| 3 | IL | IHSA API (1) | `GET api.ihsa.org/v1/schools/<id>/staff2` + `/staff/<personId>/email` | yes (Boys/Girls AD) | yes | **yes** (Boys/Girls XC + TF Head Coach) | **yes** |
| 4 | IA | Bound school directory (2, school-published) | `GET gobound.com/ia/schools/<slug>/directory/new` | yes | opt-in per school | **yes** ("Varsity Head Coach - Boys/Girls Cross Country \| Track & Field") | opt-in per school |
| 5 | MI | MHSAA admin API (1) | `GET my.mhsaa.com/…/API/School/AdministrationDirectory?SchoolId=<n>` | yes | yes | no | — |
| 6 | OH | OHSAA officials portal (1) | `GET officials.myohsaa.org/Outside/Schedule/SportsInformation?ohsaaId=<n>` | yes | yes | **yes** (XC/TF head coach, boys+girls) | **yes** |
| 7 | MO | **none** — MSHSAA exposes no contacts (1 = dead end) | — | no | no | no | — |
| 8 | KS | KSHSAA directory API (1) | `GET kshsaa-api.kshsaa.org/directory/search/name/<q>/` | yes (100% of 526) | yes (100%) | no | — |
| 9 | NE | NSAA school directory (1) | `POST secure.nsaahome.org/nsaaforms/direxportscreen.php` (school select) | yes | no | **names yes**, no email | — |
| 10 | ND | NDHSAA school page (1) | `GET ndhsaa.com/schools/<id>/<slug>` | yes | no | no | — |
| 11 | IN | **none** — IHSAA directory has no contacts; myIHSAA is login-only | — | no | no | no | — |
| 12 | SD | Bound school directory (2, school-published) | `GET gobound.com/sd/schools/<slug>/directory/new` | yes | opt-in | **yes** (Varsity Head Coach per sport) | opt-in |

Rejected/excluded candidates (fetched, then ruled out): `clellwade coachesdirectory.com` — subscriber login (`Subscriber Login` on landing page); `iiaaa.org?forward_path=/members` — association members area behind a FinalForms login; `myihsaa.net/schools` — HTTP 404; `mshsaa.org/MySchool/Info.aspx` and `/Review.aspx` — HTTP 404; `www.mhsaa.com/schools/<slug>` raw HTML — zero emails/mailto because staff block is JS-injected (the API is the reliable path); `not shown: any CAPTCHA/paywall/proxy workaround was never attempted` (policy).

Field availability per source (what the contact row actually contains):

| Source | Fields observed | Personal-data fields present but NOT collected |
|---|---|---|
| WIAA | Sport, Name, Role, Email; Admin Role/Name/Email; phone absent | none seen |
| MSHSL | Role, Name, tel: phone, obfuscated email | phone (not written) |
| IHSA | Name, DefaultTitle, per-person email endpoint | none seen |
| MHSAA | Prefix/First/Mi/Last/Suffix, RoleType.RoleName, MailAddress.Address, PhoneNumber (null in samples) | phone (not written) |
| OHSAA | role, name, email per division | none seen |
| Bound (IA/SD) | Role, Name, School Phone, **Home Phone, Address** columns exist | Home Phone + Address never read into the CSV |
| KSHSAA | ADName/ADEmail, PrincipalName/PresName, **ADCell/PresCell/PrincipalCell** | all Cell fields never read |
| NSAA | role→name pairs (Superintendent…AD…each sport) | none seen |
| NDHSAA | role→name pairs | none seen |

## Coverage

Enumerated school universes (each is a verified count, not an estimate):

- WI: 629 schools from 26 letter queries (600 flagged High School; 515 public HS) — `schools.wiaawi.org` letter directory.
- MN: `/schools` is a Drupal view, 50 slugs per page with an A–Z filter (`custom_az_filter`); full slug list requires paging the A–Z view (not exhaustively enumerated here).
- IL: 828 rows in `api.ihsa.org/v1/schools` (`data[]`, key `SchoolID` zero-padded 4-digit).
- MI: search API returns exact rows for a name query; no bulk endpoint observed (sampled 4 names → 3 exact schools + related).
- IA: 362 IAHSA school slugs enumerated; 60 of those mapped to a Bound slug inside the sampled slice (mapping is 1:1 but slug text differs, e.g. `adm-adel → adm`, so the full map needs one pass over 362 IAHSA pages).
- KS: **526 unique schools in one API response** (`search/name/a/` matches almost everything) — the cheapest full-state AD directory found in this study.
- ND: 169 school links from `ndhsaa.com/schools`.
- NE: 311 schools with published staff rosters.
- SD: 176 schools at `gobound.com/sd/associations/sdhsaa/schools`.
- OH: `officials.myohsaa.org/Outside/SearchSchool?Name=` resolves a school name to `ohsaaId`; no bulk list observed.
- MO: MSHSAA member-school search resolves a name to `/MySchool/?s=<id>` (Rock Bridge = 578); the page body carries schedule/news only.
- IN: the IHSAA directory page self-reports full members 408, provisional 5, total 413 (356 public / 57 non-public) with no per-school contact fields.

Contact-yield measurements from the collected sample (rows written, schools, and how many rows actually carry an email):

| ST | schools | rows | coach rows | coach rows w/ email | AD rows | AD rows w/ email |
|---|---|---|---|---|---|---|
| WI | 5 | 22 | 17 | 17 | 5 | 5 |
| MN | 5 | 9 | 0 | 0 | 9 | 7 |
| IL | 4 | 16 | 12 | 12 | 4 | 4 |
| IA | 5 | 83 | 63 | 3 | 20 | 2 |
| MI | 2 | 14 | 0 | 0 | 14 | 14 |
| OH | 5 | 31 | 18 | 15 | 13 | 13 |
| KS | 526 | 526 | 0 | 0 | 526 | 526 |
| NE | 311 | 1,528 | 1,217 | 0 | 311 | 0 |
| ND | 6 | 39 | 0 | 0 | 39 | 0 |
| SD | 5 | 30 | 20 | 4 | 10 | 2 |

Two distinct yield profiles exist and should drive adapter design: (a) **association-keyed AD registry** — KS (526/526 with email), MI (AD+email for every school sampled), MN (7/9), OH (13/13), WI (5/5), IL (4/4): one request per school, deterministic; (b) **school-published roster with per-school email opt-in** — Bound IA/SD where every school publishes role+name but email appears per school (Rapid City Stevens SD 36/43 contact rows had email; Albia IA 32/121; the other 8 sampled schools 0) and NSAA where names are complete but emails are absent by design.

## Enumeration

Request-cost model per state (all counts below are from executed calls, not projections):

- **KS — best ratio found.** `GET https://kshsaa-api.kshsaa.org/directory/search/name/a/` = 1 request → 526 schools with ADName+ADEmail+Class+Enrollment+League+USD+WebSite (HTTP 200, 489,381 bytes). Letter-level queries (`name/<letter>/`) page the same directory if a narrower slice is wanted; `/directory/all/`, `/directory/list/`, `/directory/search/` and `/swagger/index.html` all returned 404, so the letter query is the enumerator.
- **SD — 2 requests for the universe**: association index `gobound.com/sd/associations/sdhsaa/schools` (200; 528 school-related links = 176 schools × 3 URL forms), then 1 request per school `/directory/new` (~184 KB HTML each; 7 tables: address block + Administration + Training staff + per-level coach tables).
- **IA — 2 requests per school** (IAHSA page to discover the Bound slug, then the Bound directory) for the 60-slug sample; Bound slugs differ from IAHSA slugs, so a full build is 362 + 362 ≈ 724 requests, sequential.
- **WI — 26 requests to enumerate 629 schools** (letter POSTs, ~150–215 KB each) + 1 detail GET per school (`GetDirectorySchool?orgID=<n>`); the sample fetched 5 details, a full build is ~655 requests.
- **NE — 1 request per school** via the POST school-select (311 schools in the collected set).
- **ND — 1 request per school** of 169 links for names only (no email payoff, so refresh is optional).
- **IL — 3 requests per school** (list once + `staff2` + one `/email` per person of interest; the sample used ~5 email lookups per school).

## Stable identifiers

- **KS**: `Id` (internal) + `Identifier` (e.g. `KSS0307`) + `USD` (district) + `Class` + `LeagueName`; `DateModified` present (observed values range 2018-08-07 → 2026-07-08, which flags stale AD records).
- **MI**: `SchoolID` (e.g. East Kentwood 4351, Detroit Catholic Central 4239) + `VanityURL` (`eastkentwood`) + `OldSchoolId`; sports rows carry `SportTeamID` (e.g. Track & Field 3121100) and `SportTypeCode` (TR).
- **IL**: `SchoolID` (4-digit, e.g. `0101`) — the same key the IHSA result surfaces use.
- **OH**: `ohsaaId` (e.g. 474 Dublin Coffman) + school `hytek`/meet code (e.g. `DCO`) + `county`.
- **ND**: `/schools/<numericId>/<slug>`; **WI**: `orgID`; **Bound IA/SD**: school slug.
- **NSAA**: school-name keyed only (no numeric id observed in the directory form).

These are directly reusable as join keys for team/school reconciliation; none of them is an Athletic.net id, so a name+state+city bridge is still required.

## Athletic.net leverage

This assignment supplies the *contact* leg of the acquisition flow, not athlete discovery, but three concrete leverage points exist:

1. **Sport-scoped coach identity without Athletic.net team pages.** OH, IL, WI, IA, SD, NE answer "who is the boys/girls XC or TF head coach at school X" — the same question the pipeline would otherwise answer by scraping an Athletic.net team page or coach list. NE alone names XC/TF coaches for 311 schools with one request each.
2. **Verification of school identity for Athletic.net reconciliation.** KS (`USD`, `Class`, `Enrollment`), MI (`SportTeamID`), OH (`ohsaaId`, `hytek`) and IL (`SchoolID`) give authoritative school keys/enrollment to disambiguate same-named Athletic.net teams (e.g. "Saline" vs "Saline Washtenaw Christian" both returned by MHSAA search) without hitting Athletic.net.
3. **Outreach for missing-athlete discovery.** A coach/AD contact per school is the only mechanism that reaches athletes who are absent from Athletic.net ranking enumeration — email the coach "who else ran XC at your school this fall" instead of paying per-page scraping. Coverage for that is 6 states with sport-level coach email + 2 states with sport-level coach *name* only.

Cost delta is measurable: KS AD+email for 526 schools = 1 request; the equivalent Athletic.net team-page crawl is ~526 requests that the mission brief says are Cloudflare-blocked anyway.

## Athlete evidence

None. This source class contains no athlete records; no athlete names, times, or Class-of-2027 evidence appear in any directory row. Nothing in the CSV derives from or requires athletic participation data. The coach↔school↔sport triple is the only athlete-adjacent signal, and it is used solely to route outreach.

## Recruiting information

Public professional contact information only, scoped to the coaching/administrative role:

- Collected: coach name + head-coach role + sport + gender side where published; AD/Activities Director name; emails that the school or association published *for that role* (school-domain emails dominate: `jknapmiller@abbotsford.k12.wi.us`, `alarson@abbotsford.k12.wi.us`, `drose@csd99.org`, `sheldon_duane@dublinschools.net`, `rbaldwin@usd260.com`).
- Deliberately **not** collected, even when the source exposes it: KSHSAA `ADCell`/`PresCell`/`PrincipalCell`; Bound `Home Phone` and `Address` columns; MSHSL `tel:` phone numbers. None of these fields is read by `build_csv.py`, so they cannot leak into the deliverable.
- Judgment call recorded for the consumer: OHSAA's official coach fields sometimes hold a non-school address that the school itself published in the coach slot (e.g. `j.legins106@gmail.com`, `steeple844@aol.com`, `dennison@secondsoleohio.com`). These are role-published, not harvested, but they are personal-domain mailboxes; if the production policy is school-domain-only, filter `public_professional_email` on the domain.
- No athlete email/phone/address, no parent contacts, no student rosters were touched by any query in this assignment.

## Result evidence

None directly — no meet results, times or marks live in these directories. Two adjacency notes worth carrying to the result adapters: OH's `outdoors.official`/portal school records carry a `hytek` code that ties a school to Hy-Tek meet result files, and MI's `SportsDirectory` returns `SportTeamID` which is the same id space MI meet/scoreboard endpoints use, so school-key ↔ result-key joins exist without Athletic.net.

## Incremental use

- **Week-1 adapter (best coverage/engineering): KSHSAA directory API.** One GET, 526 schools, 100% AD name+email, plus enrollment/class/USD for school-identity bridging. Failure mode is trivial to detect (JSON shape change).
- **Week-1 adapter (highest value for outreach): Bound school directories for IA + SD.** One GET per school yields role-scoped head coaches for XC/TF by side, with email where the school published it; the association index enumerates the school universe (176 SD schools in one request).
- **Week-1 adapter: WIAA detail pages (WI, 629 schools).** Full coach table *including email* — the densest coach-level source observed; 27 requests for the whole state.
- **Week-2 adapter: IHSA API (IL).** Cleanest schema and a per-person email endpoint; 828 schools; costs ~3 requests per school, so run it on-demand/refresh rather than bulk.
- **Week-2 adapter: OHSAA portal (OH).** Sport-specific head-coach rows with emails for a school id you already resolve by name; ideal for per-school enrichment triggered by pipeline events.
- **Week-2 adapter (AD only): MHSAA API (MI), MSHSL pages (MN), KSHSAA-shape for ND** — names/emails for the athletic office, good for cold outreach, no sport scoping.
- **Refresh cadence:** KSHSAA ships `DateModified` (use it to skip unchanged schools); everything else needs a full re-read; NE/ND are name-only so refresh cost is 1 request/school with no email payoff.
- **Do not build:** MO and IN association adapters — both were fetched and contain no contact data. For those two states route through tier 3 (district staff directories / the school's own athletics site) or leave the contact column empty.
- **Cost avoided:** the 2,298 rows in `data/coach-contacts.csv` were produced by roughly 500 HTTP requests: 1,528 rows came from 311 NSAA requests, 526 rows from a single KSHSAA request, and 95 rows from the five Bound IA/SD school directories plus one association index. That is a per-school cost two orders of magnitude below Athletic.net team-page scraping, and for KS the entire state AD registry is one request.

## Access characteristics

- **Class: public, unauthenticated, no CAPTCHA/paywall encountered** on every winner listed above. No cookie replay, no fingerprint spoofing, no proxy rotation, no auth bypass was used anywhere in this assignment; all fetches used `User-Agent: omp-research/1.0 (contact-directory research)` sequentially with 0.4–0.8 s spacing.
- **curl-friendly (no browser needed):** `kshsaa-api.kshsaa.org`, `api.ihsa.org` (fetched from a browser context; its JSON is unauthenticated), `my.mhsaa.com` MHSAA-Endpoint APIs (`POST` JSON + `GET`), `ndhsaa.com/schools/...`, `schools.wiaawi.org`, `mshsl.org/schools/<slug>`, `sdhsaa.com`, `ihsaa.org`, `mshsaa.org`.
- **Browser required / JS-rendered:** `nsaahome.org` (curl 403 per the shared brief; rendered fine in the browser — DNS for the hyphenated `nsaa-home.org` did **not** resolve), `gobound.com` IA/SD directory pages (403 to curl; rendered + same-origin `fetch()` worked in the browser), `officials.myohsaa.org` (fetched from the browser session), `www.mhsaa.com/schools/<slug>` staff block (JS-injected; the JSON API is the stable path).
- **Blocked / not collectible:** `Clell Wade Coaches Directory` (`coachesdirectory.com`) — subscription login; `IIAAA` members area (`iiaaa.finalforms-amp.com/?forward_path=%2Fmembers`) — login; `myIHSAA.net` — 404 on `/schools`.
- **Notable dead ends verified as "no data" rather than "no access":** `mshsaa.org/MySchool/?s=578` (200, 321 KB, 0 occurrences of "Athletic Director"/"Principal", no emails beyond `email@mshsaa.org`), `mshsaa.org/MySchool/Info.aspx?s=578` (404), `ihsaa.org/schools/ihsaa-school-directory` (200, renders membership stats and a "Visit myIHSAA.net" hand-off, no school contact rows), `www.mhsaa.com/schools/{detroitcatholiccentral,saline,eastkentwood,novi}` (200, 0 `mailto:` in raw HTML).
- **Host request budget respected:** per-host counts stayed inside the ~50 ceiling for all association/timer hosts (largest: gobound.com ~13, mshsaa.org 4, kshsaa-api 7, mhsaa 8); no Retry-After was ever returned.
- **Refresh risk to plan for:** MHSAA/IHSA/Bound pages are CMS-rendered and NSAA/NDHSAA/MSHSAA school pages are template-driven — a template change silently empties a field. Each adapter needs a non-empty assert on the email field (not just HTTP 200).

## Recommendation

Adopt a **two-layer contact graph**: (1) an association-keyed AD/identity layer built from KSHSAA (KS), MHSAA (MI), MSHSL (MN), IHSA (IL), WIAA (WI), OHSAA (OH) — one request per school, school identity keys included, refreshable; and (2) a sport-scoped coach layer from WIAA (WI), IHSA (IL), OHSAA (OH) and Bound (IA, SD) for the states that actually publish XC/TF head coaches with email. Treat NE and ND as name-only enrichment (outreach research), and treat **MO and IN as contact-graph gaps** — do not build association adapters for them; either accept an empty coach column or solve them via district staff directories in a later pass.

Reusable per state (2,298-row dataset already written): `data/coach-contacts.csv` with `source_url` + `last_observed` on every row; rebuild any time with `python3 tools/a29-coach/build_csv.py` from the raw captures in `tools/a29-coach/`.

### Evidence appendix

| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| https://kshsaa-api.kshsaa.org/directory/search/name/a/ | GET curl | 200 (489,381 B) | 526 unique KS schools, 100% with ADName+ADEmail; fields incl. Class/Enrollment/League/USD/DateModified; no coach field; ADCell present (excluded) | 2026-09-19 |
| https://kshsaa-api.kshsaa.org/directory/all/ , /directory/list/ , /directory/search/ | GET curl | 404 | Letter query is the only enumerator found; no bulk "all" endpoint | 2026-09-19 |
| https://kshsaa-api.kshsaa.org/ | GET curl | 200 (`OK`) | API host live | 2026-09-19 |
| https://www.gobound.com/sd/associations/sdhsaa/schools | GET browser | 200 | SD school universe = 176 schools (528 school links = 176×3 URL forms) | 2026-09-19 |
| https://www.gobound.com/sd/schools/aberdeencentral/directory/new | GET browser (same-origin fetch) | 200 (184,148 B) | 7 tables incl. Administration (AD "Bo Beck") + Varsity Head Coach rows for Boys/Girls XC + TF; Email column exists but empty for this school | 2026-09-19 |
| https://www.gobound.com/sd/schools/rapidcitystevens/directory/new | GET browser | 200 | 36/43 contact rows carry an email → email is per-school opt-in, not per-platform | 2026-09-19 |
| https://www.gobound.com/ia/schools/albia/directory/new | GET browser | 200 | 32/121 rows with email; Boys/Girls XC + TF head coaches present | 2026-09-19 |
| (IA ×5, SD ×5) /{ia,sd}/schools/<slug>/directory/new | GET browser | 200 all | Bound IA/SD provide role-scoped coach names for every sampled school; emails only for 2 of 10 schools | 2026-09-19 |
| https://www.iahsaa.org/schools/<slug>/ | GET curl | 200 ×60 | IAHSA member-school pages carry no contacts; they link out to `gobound.com/ia/schools/<boundSlug>` (slug text differs from IAHSA slug) | 2026-09-19 |
| https://sdhsaa.com/ | GET curl | 200 (1.1 MB) → https://sdhsaa.com/ | Homepage links `gobound.com/sd/associations/sdhsaa/schools`; also links Athletic.net for cross-country (confirms dual usage) | 2026-09-19 |
| https://api.ihsa.org/v1/schools | GET browser | 200 | 828 IL member schools (`data[]`, SchoolID, nameFormal, city, enrollmentType) | 2026-09-19 |
| https://api.ihsa.org/v1/schools/0101/staff2 | GET browser | 200 | Staff by committee incl. Boys/Girls XC + TF **Head Coach** with PersonID + HasEmail | 2026-09-19 |
| https://api.ihsa.org/v1/schools/0101/staff/<personId>/email | GET browser | 200 ×20 | Per-person email endpoint returns the professional address (e.g. `bmink@atown276.net`); 12/12 coach rows sampled had email | 2026-09-19 |
| https://my.mhsaa.com//DesktopModules/MHSAA-Endpoint/API/School/Search | POST JSON curl | 200 | Name → SchoolID/VanityURL (`East Kentwood`→4351); no bulk list | 2026-09-19 |
| https://my.mhsaa.com//DesktopModules/MHSAA-Endpoint/API/School/AdministrationDirectory?SchoolId=4351 | GET curl | 200 (3,161 B) | AD + Secretary + Trainer + Principal + Superintendent with role names and `MailAddress.Address` | 2026-09-19 |
| …/API/School/AdministrationDirectory?SchoolId=4239 | GET curl | 200 | Detroit Catholic Central: 3 Athletic Directors + city-wide AD, each with school email | 2026-09-19 |
| https://my.mhsaa.com//DesktopModules/MHSAA-Endpoint/handlers/SchoolInfo.ashx?Method=SportsDirectory&SchoolId=4351 | GET curl | 200 (20,640 B) | Sport list with SportTeamID/SportTypeCode (stable ids) but **no coach names** | 2026-09-19 |
| https://www.mhsaa.com/schools/{detroitcatholiccentral,saline,eastkentwood,novi} | GET curl ×4 | 200 (≈124 KB each) | 0 emails / 0 `mailto:` in raw HTML → staff block is JS-injected; browser render of DCC showed 6 emails/3 ADs | 2026-09-19 |
| https://www.mshsl.org/schools | GET curl | 200 (221,975 B) | Drupal view, 50 slugs/page, A–Z filter; MN school enumeration surface | 2026-09-19 |
| https://www.mshsl.org/schools/{ada-borup-west,aitkin,alexandria-area,albany,academic-arts}-high-school | GET curl ×5 | 200 all | Activities Director name + email (Cloudflare `email-protection#<hex>` decodable), phone; **no sport coaches** | 2026-09-19 |
| https://www.mshsl.org/schools/wayzata-high-school | GET curl | 200 | AD Meghan Potter + 2 AD assistants with emails | 2026-09-19 |
| https://ndhsaa.com/schools (index) + /schools/{1045,1131,1281,1307,24,31}/<slug> | GET curl ×7 | 200 all | 169 school links; Superintendent/Principal/Athletic Director/Activities Director **names only**, 0 emails on every page | 2026-09-19 |
| https://schools.wiaawi.org/Directory/School/DirectoryLetter?LetterBtn=<A–Z> | POST curl ×26 | 200 all | 629 WI schools (600 HS, 515 public HS); universe for the WIAA adapter | 2026-09-19 |
| https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=<n> | GET curl ×5 | 200 all | **Coach table** (Sport/Name/Role/Email, e.g. Boys Track & Field → `jknapmiller@abbotsford.k12.wi.us`) + Administration table with AD email | 2026-09-19 |
| https://officials.myohsaa.org/Outside/SearchSchool?Name=<school> | GET curl ×5 | 200 all | Name → `ohsaaId` (Dublin Coffman 474, St Ignatius 1354, Mentor 1016, Centerville 336, Hilliard Davidson 716) | 2026-09-19 |
| https://officials.myohsaa.org/Outside/Schedule/AthleticDirector?ohsaaId=474 | GET curl | 200 | AD role→name→email triples (`sheldon_duane@dublinschools.net`) | 2026-09-19 |
| https://officials.myohsaa.org/Outside/Schedule/SportsInformation?ohsaaId=<n> | GET curl ×5 | 200 all | Boys+Girls XC and TF **head coaches with emails** for the sampled schools (15/18 coach rows had email; 3 were `TBA`) | 2026-09-19 |
| https://secure.nsaahome.org/nsaaforms/direxportscreen.php | GET browser + POST school select | 200 | NSAA member directory: 311 schools with staff-by-role including each sport's coach by name (e.g. Adams Central `Cross-Country (Boys)` → Toni Fowler); **no email fields** | 2026-09-19 |
| https://nsaahome.org/ (and https://nsaa-home.org/) | GET browser | 200 / DNS ERR_NAME_NOT_RESOLVED | NSAA lives at `nsaahome.org`; hyphenated host does not resolve | 2026-09-19 |
| https://www.mshsaa.org/Schools/Default.aspx | GET curl + browser POST | 200 (21,474 B response) | Server postback always falls through to a `MySchool` page; search UI is JS-driven | 2026-09-19 |
| https://www.mshsaa.org/MySchool/?s=578 | GET curl | 200 (321,572 B) | Rock Bridge page: 0 occurrences of "Athletic Director"/"Principal", no contact emails → MSHSAA publishes no contacts | 2026-09-19 |
| https://www.mshsaa.org/MySchool/Info.aspx?s=578 , /MySchool/Review.aspx?s=578 | GET curl | 404, 404 | No Info/contacts subpage exists | 2026-09-19 |
| http://www.miaaamo.org | GET curl | 200 → https://www.miaaamo.org/ | MO AD association links coachesdirectory.com; no own member directory | 2026-09-19 |
| https://www.coachesdirectory.com/ | GET curl | 200 (8,317 B) | "Subscriber Login" → paid directory, excluded | 2026-09-19 |
| https://www.iiaaa.org | GET curl | 200 → https://www.iiaaa.org/ | IN AD association; members area is `iiaaa.finalforms-amp.com/?forward_path=/members` (login) → excluded | 2026-09-19 |
| https://www.ihsaa.org/schools/ihsaa-school-directory | GET browser + curl | 200 (267,694 B) | Membership stats only (408 full / 413 total, 356 public-57 non-public); no school contact rows; hand-off link to myIHSAA.net | 2026-09-19 |
| https://www.myihsaa.net/schools | GET curl | 404 | Login app; no public school directory (IN gap confirmed) | 2026-09-19 |

Unverified/residual: the MN full school-slug enumeration (A–Z paging) was not completed; the IA IAHSA→Bound slug map is 60/362; MO and IN tier-3 (district staff directory) collection was not performed; the per-state crawl costs in "Enumeration" are request arithmetic from the executed samples, not executed full runs [INFERENCE for the totals]; email-format similarity across a district is an observation from the samples, not a confirmed rule.
