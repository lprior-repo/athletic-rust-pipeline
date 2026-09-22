| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| https://www.ihsa.org/assets/StateSeries-w2fH3odu.js | GET | 200 | current IHSA site chunk (imported by main-B2rKN2jy.js): `x={"boys-cross-country":{"1A":"688","2A":"689","3A":"690"},"girls-cross-country":{"1A":"691","2A":"692","3A":"693"}}`; site's fetch call `${pe(n)}/statefinal/cc-qualifiers?tournamentId=${id}`; girls XC-side asset constants use `/data/ccg/` | 09-20T09:06 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=691 | GET | 200 | girls 1A: 357 athletes / 30 teams / 41 individual qualifiers; 97 grade-11 | 09-20T09:06 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=692 | GET | 200 | girls 2A: 397 athletes / 28 teams / 31 individuals; 96 grade-11 | 09-20T09:06 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=693 | GET | 200 | girls 3A: 423 athletes / 28 teams / 35 individuals; 125 grade-11 | 09-20T09:06 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers?tournamentId=688 | GET | 200 | boys 1A control: 398 athletes / 30 teams / 42 individual qualifiers, 104 grade-11 (reproduces report 13) | 09-20T09:06 |
| https://api.ihsa.org/v1/2026-27/statefinal/cc-qualifiers?tournamentId=691 | GET | 404 | 404 `{"error":"Archive not available for term 2026-27"}` — current-season roster not posted yet (2026-27 girls XC final = 2026-11-07) | 09-20T09:06 |
| https://api.ihsa.org/v1/2025-26/statefinal/cc-qualifiers | GET | 400 | 400 `{"error":"Missing tournamentId"}` — XC ids are NOT enumerable from the API; they exist only in the site chunk | 09-20T09:06 |
| https://api.ihsa.org/v1/sports/girls-cross-country/tournament-info | GET | 200 | current-term girls XC state-final info: `classes:"1A-3A"`, Saturday November 7 2026, Detweiller Park — no tournament ids in this payload | 09-20T09:06 |
| https://api.ihsa.org/v1/2024-25/statefinal/cc-qualifiers?tournamentId=691 | GET | 200 | 200 with empty arrays — the id exists for older terms but no roster is archived (mirrors boys 688) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools | GET | 200 | 828 member-school rows — sample frame for the fresh coach sample | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0120/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0120/staff/21953/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0219/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0219/staff/48746/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0302/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0302/staff/6889/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0337/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0337/staff/7372/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0415/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0415/staff/21368/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0523/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0523/staff/40351/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0705/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0809/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/0809/staff/137473/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1103/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1232/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1232/staff/108737/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1329/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1329/staff/11187/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1366/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1504/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1504/staff/12157/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1618/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1803/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1903/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1941/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/1941/staff/34575/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2205/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2205/staff/98284/email | GET | 200 | email reveal on a head-coach row → 200 with a non-empty address — email-present evidence | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2330/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2720/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2759/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2817/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2924/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v1/schools/2966/staff2 | GET | 200 | staff2 directory payload for a sampled school (role rows + `HasEmail`) | 09-20T09:08 |
| https://api.ihsa.org/v2/classification/classifications?season=S | GET | 200 | 3,121 spring rows — T&F sponsorship/entry join for the sampled schools | 09-20T09:09 |
| https://api.ihsa.org/v2/classification/classifications?season=F | GET | 200 | 3,960 fall rows — XC sponsorship/entry join for the sampled schools; also the girls-XC class-mapping cross-check | 09-20T09:09 |
| https://api.ihsa.org/v1/schools/0809/staff/107373/email | GET | 200 | HasEmail=false probe (0809 girls T&F head coach) → `{"email":""}` — `HasEmail` is an accurate gate | 09-20T09:09 |
| https://www.directathletics.com/rankings.html | GET | 200 | live list index: 5,584 performance-list links; exactly 30 IL Top Times links, highest ids 4524/4525/4526; max global list id 5775 (2026 lists); no TFRRS-shaped Illinois list | 09-20T09:09 |
| https://www.directathletics.com/lists/track/1467_4524.html | GET | 200 | title `Illinois Top Times (1A)`; 1,471 data rows (1,176 athlete + 295 relay, = report 14's default-view count), all 2024 (Feb 1 – Mar 9) | 09-20T09:09 |
| https://www.directathletics.com/lists/track/1468_4525.html | GET | 200 | title `Illinois Top Times (2A)`; 1,500 data rows (1,200 + 300 relay), all 2024 | 09-20T09:09 |
| https://www.directathletics.com/lists/track/1469_4526.html | GET | 200 | title `Illinois Top Times (3A)`; 1,500 data rows (1,200 + 300 relay), all 2024 | 09-20T09:09 |
| https://www.directathletics.com/leagues/track/1467.html | GET | 200 | league home — `View Performance List` → `list_hnd=4524` (the league's current list is the 2024 one) | 09-20T09:09 |
| https://www.directathletics.com/legacy_da/results?filterrific%5Bwith_states%5D=IL&filterrific%5Bsearch_query%5D=Top%20Times | GET | 200 | IL meet-index name filter `Top Times`: 20 rows, newest 2024-03-23 (2A, meet 84225) / 2024-03-22 (1A, meet 84224); nothing in 2025/2026 | 09-20T09:09 |
| https://www.directathletics.com/results/track/84225.html | GET | 200 | `Illinois Top Times Indoor Track & Field Championships (2A)` meet sheet, March 2024 | 09-20T09:09 |
| https://www.tfrrs.org/lists/4524/Illinois_Top_Times_1A | GET | 200 | TFRRS mirror `TFRRS \| Illinois Top Times (1A)`; 1,471 date-stamped rows, all 2024 (no athlete links server-side) | 09-20T09:09 |
| https://www.directathletics.com/lists/track/1355_290.html | GET | 200 | Chicago Public Schools league list: 360 rows, all May 2007 — historical | 09-20T09:10 |
| https://www.directathletics.com/legacy_da/results?filterrific%5Bwith_states%5D=IL | GET | 200 | IL results index page 1 (newest 100): 78×2026 + 22×2025 dates, all collegiate (TFRRS links) — no HS rows | 09-20T09:10 |
| https://www.directathletics.com/legacy_da/results?filterrific%5Bwith_states%5D=IL&filterrific%5Bsearch_query%5D=High%20School | GET | 200 | IL meet names containing `High School`: 83 rows, newest 03/02/2024 (Cogdal HS Invite) — no 2025/2026 row matches this name filter | 09-20T09:10 |
| https://www.directathletics.com/leagues/track/1468.html | GET | 200 | league home → `list_hnd=4525` | 09-20T09:10 |
| https://www.directathletics.com/leagues/track/1469.html | GET | 200 | league home → `list_hnd=4526` | 09-20T09:10 |
| https://www.ihsa.org/ | GET | 200 | entry HTML references `/assets/main-B2rKN2jy.js` (hash unchanged since report 13's capture) | 09-20T09:12 |
| https://www.ihsa.org/assets/main-B2rKN2jy.js | GET | 200 (×2) | entry bundle imports `./StateSeries-w2fH3odu.js` → the id-map chunk belongs to the live deployment (fetched twice: inventory + saved capture) | 09-20T09:12 |
| https://www.ihsa.org/sports/girls-cross-country/state-series/xc-state-central | browser load (headless Chromium, `request` listener) | page 200 / XHR 200 | live girls XC page requested `/v1/2025-26/statefinal/cc-qualifiers?tournamentId=691` on load and `=692`,`=693` on the 2A/3A class-tab clicks; zero `athletic.net` requests | 09-20T09:07 |
