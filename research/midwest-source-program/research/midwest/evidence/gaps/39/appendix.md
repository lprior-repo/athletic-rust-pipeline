| URL | method | HTTP | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.mhsaa.com/sports/boys-track-field` | GET | 200 | 2026 TF hub: Tracking-the-Tournament block links 5 finals meets (D1 622966, D2 622969, D3 622972, D4 622973, UP 622974) in 13 hrefs | 2026-09-20T09:05:35-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/results-archive` | GET | 200 | TF results archive (boys): year sections 2016-2026 (no 2020); 20 /meet/ ids + 36 legacy Print/EntryMeet ids; links every UP girls finals artifact | 2026-09-20T09:05:36-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/results-archive` | GET | 200 | TF results archive (girls): byte-identical link set (same 52 meet ids + same UP girls files) | 2026-09-20T09:05:40-0500 |
| `https://www.mhsaa.com/sports/girls-track-field` | GET | 200 | Girls TF hub: same 5 finals meet ids; its "2026 Regional Results" link points at the boys-* regional page | 2026-09-20T09:05:41-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2026-mhsaa-track-field-regional-results` | GET | 200 | 2026 TF regional page: 48 HS regionals + 1 JH/MS zone link = 49 athletic.net hrefs; one Results link per regional | 2026-09-20T09:05:59-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/2026-mhsaa-track-field-regional-results` | GET | 404 | 404 — confirms 2026 regional page is published only under boys-* | 2026-09-20T09:06:00-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2025-mhsaa-track-field-regional-results` | GET | 404 | 404 — MHSAA publishes one TF regional page per season; 2025 lives under the girls-* slug | 2026-09-20T09:06:01-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/2025-mhsaa-track-field-regional-results` | GET | 200 | 2025 TF regional page (published under girls-* slug): 49 athletic.net hrefs = 48 regionals + 1 JH/MS | 2026-09-20T09:06:03-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2024-mhsaa-track-field-regional-results` | GET | 200 | 2024 TF regional page: 48 athletic.net hrefs | 2026-09-20T09:06:04-0500 |
| `https://www.mhsaa.com/sports/boys-track-field/2023-mhsaa-track-field-regional-info-and-results` | GET | 200 | 2023 TF regional page (older slug form): 48 athletic.net hrefs | 2026-09-20T09:06:05-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country` | GET | 200 | XC hub: links only the JH/MS Middle-School index on athletic.net (no meet ids) | 2026-09-20T09:06:10-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country` | GET | 200 | XC hub (girls): same; links the boys-* regional page for 2025/2026 results | 2026-09-20T09:06:11-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2026-mhsaa-lp-cross-country-regional-info-and-results` | GET | 200 | 2026 LP XC page pre-season state: 36 region rows, only 1 athletic.net href (JH/MS index) — regional Results links appear after each regional is contested | 2026-09-20T09:06:12-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2025-mhsaa-cross-country-regional-info-and-results` | GET | 200 | 2025 XC regional page: 37 athletic.net hrefs / 36 distinct XC meet ids | 2026-09-20T09:06:14-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2024-mhsaa-cross-country-regional-info-and-results` | GET | 200 | 2024 XC regional page: 37 athletic.net hrefs | 2026-09-20T09:06:15-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/results-archive` | GET | 200 | XC archive (boys): 8 XC meet ids (2022 section) + UP girls XC finals PDF names for 2015/2016/2020/2021/2022/2023 | 2026-09-20T09:06:22-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country/results-archive` | GET | 200 | XC archive (girls): same UP girls file set (8 XC meet ids) | 2026-09-20T09:06:25-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Girls-D1-Finals.pdf` | HEAD | 200 | UP girls D1 finals 2026 artifact exists (PDF, Hy-Tek) | 2026-09-20T09:06:56-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Girls-D2-Finals.pdf` | HEAD | 200 | UP girls D2 finals 2026 (203,062 B downloaded): 92 graded rows, 25 grade-11 | 2026-09-20T09:06:57-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Girls-D3-Finals.pdf` | HEAD | 200 | UP girls D3 finals 2026 (269,086 B downloaded): 240 graded rows, 45 grade-11 | 2026-09-20T09:06:58-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2026/Finals/2026-UP-Boys-D2-Finals.pdf` | HEAD | 200 | Boys sibling naming checked for the same season | 2026-09-20T09:07:00-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2025/Finals/UP-D1-Girls.pdf` | HEAD | 200 | UP girls D1 finals 2025 exists | 2026-09-20T09:07:01-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2025/Finals/Up-D2-Girls.pdf` | HEAD | 200 | probe | 2026-09-20T09:07:02-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2025/Finals/Up-D3-Girls.pdf` | HEAD | 200 | probe | 2026-09-20T09:07:03-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2024/Finals/UP-D2-Girls.pdf` | HEAD | 200 | UP girls D2 finals 2024 (581,244 B downloaded): 96 graded rows, 40 grade-11 | 2026-09-20T09:07:05-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2024/Finals/UP-D3-Girls.pdf` | HEAD | 200 | UP girls D3 finals 2024 exists | 2026-09-20T09:07:06-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2023/UP%20D2Girls.htm` | HEAD | 200 | UP girls D2 finals 2023 (22,303 B): has a "Name Year School" header but the year column is empty — no grades | 2026-09-20T09:07:07-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2023/UP%20D3Girls.htm` | HEAD | 200 | UP girls D3 finals 2023 exists | 2026-09-20T09:07:08-0500 |
| `https://www.mhsaa.com/sports/boys-cross-country/2023-mhsaa-cross-country-regional-info-and-results` | GET | 200 | 2023 XC regional page: 36 athletic.net hrefs | 2026-09-20T09:07:35-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country/2025-mhsaa-cross-country-regional-info-and-results` | GET | 404 | 404 — XC regional page is published once per season (boys-* path) and linked from both hubs | 2026-09-20T09:07:36-0500 |
| `https://search.athletic.live/michiana_meet_list/_search` | POST | 200 | Anonymous POST 200: tenant MITS filter returns 20 docs | 2026-09-20T09:08:03-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_list/61710.json` | GET | 200 | Firebase meet metadata for MITS 4 (2026-02-14) | 2026-09-20T09:08:32-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_61710/event_summary.json` | GET | 200 | MITS 4 (2026-02-14) Firebase event index | 2026-09-20T09:08:34-0500 |
| `http://fatresults.com/` | GET | 200 | AthleticLIVE SPA shell (50,184 B) for tenant michiana; hostname→site-asset map contains "live.michianatiming.com" = sites/michiana | 2026-09-20T09:08:34-0500 |
| `https://michianatiming.com/` | GET | 200 | probe | 2026-09-20T09:08:35-0500 |
| `https://search.athletic.live/*_meet_list/_search` | POST | 200 | Anonymous POST 200: cross-tenant MITS filter returns 133 docs across 7 indices | 2026-09-20T09:08:45-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2254285` | GET | 200 | 2026-02-14 MITS 4 B 1600m: 43/43 rows graded (7 grade-11 shown), 43/43 AN ids | 2026-09-20T09:08:54-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2254286` | GET | 200 | 2026-02-14 MITS 4 G 60mH: 21/21 rows graded (4 grade-11), 21/21 AN ids | 2026-09-20T09:08:55-0500 |
| `https://michianatiming.com/results/` | GET | 200 | probe | 2026-09-20T09:08:56-0500 |
| `https://michianatiming.com/results/results-archive/` | GET | 200 | Michiana Timing WordPress results/archive index (206,603 B) — post titles only | 2026-09-20T09:08:57-0500 |
| `https://mi.milesplit.com/calendar` | GET | 200 | Indoor calendar source: 37 indoor meets Dec 2025-Feb 2026 with data-meet-id (17 MITS-named) | 2026-09-20T09:09:09-0500 |
| `https://michianatiming.com/january-22-aq-mits-meet/` | GET | 200 | AQ MITS post body = entrant/bib list; no marks, no grades, no embedded results tables | 2026-09-20T09:09:32-0500 |
| `https://michianatiming.com/results/live-results-track-and-field/` | GET | 200 | probe | 2026-09-20T09:09:33-0500 |
| `https://mi.milesplit.com/meets/722191-mitsmitca-indoor-tf-state-championships-2026/results` | GET | 200 | Results page: meet-manager shell; result rows not public | 2026-09-20T09:09:43-0500 |
| `https://mi.milesplit.com/meets/722191-mitsmitca-indoor-tf-state-championships-2026/entries` | GET | 200 | Entries page: JSON-LD SportsEvent only; entries locked (PRO) | 2026-09-20T09:09:44-0500 |
| `https://mi.milesplit.com/athletes/17984073-david-castrejon` | GET | 200 | AN athlete id 17984073 resolves to a different MileSplit athlete (Quantravious Broussard, Class of 2030, FL) — id namespaces differ | 2026-09-20T09:10:00-0500 |
| `https://mi.milesplit.com/athletes/27105091-olivia-hawkins` | GET | 404 | 404 — MileSplit ids are not AN athlete ids | 2026-09-20T09:10:01-0500 |
| `https://mi.milesplit.com/results` | GET | 200 | probe | 2026-09-20T09:10:12-0500 |
| `https://mi.milesplit.com/athletes/11141513-luka-hammond` | GET | 200 | Public profile: Class of <year> + PR table (marks/dates) free; full result history PRO | 2026-09-20T09:10:13-0500 |
| `https://fatresults.com/assets/sites/michiana/config.json` | HEAD | 200 | 200 — confirms fatresults.com is the white-label of the michiana tenant | 2026-09-20T09:10:28-0500 |
| `https://mitsindoor.com/` | HEAD | 0 | DNS failure (no A record) | 2026-09-20T09:10:28-0500 |
| `https://michiganindoortrackseries.com/` | HEAD | 0 | DNS failure (no A record) | 2026-09-20T09:10:28-0500 |
| `https://mits.run/` | HEAD | 0 | DNS failure (no A record) | 2026-09-20T09:10:28-0500 |
| `https://www.mitsmichigan.com/` | HEAD | 0 | DNS failure (no A record) | 2026-09-20T09:10:28-0500 |
| `https://live.michianatiming.com/robots.txt` | GET | 200 | Tenant robots: Disallow /admin*, /meets/*/athletes*, /meets/*/live*, /meets/*/teams*, /meets/*/follow* | 2026-09-20T09:10:34-0500 |
| `https://live.michianatiming.com/meet-list` | GET | 200 | AthleticLIVE SPA shell (50,184 B) — no server-rendered meet rows | 2026-09-20T09:10:35-0500 |
| `https://mitca.org/MITCA/` | GET | 200 | MITCA site: 0 MITS/indoor mentions; not an indoor archive | 2026-09-20T09:10:36-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_4942/event_summary.json` | GET | 200 | AQ MITS 1 (2019-12-14): 18 events — 2019 season still indexed | 2026-09-20T09:10:47-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/179592` | GET | 200 | 2019 AQ MITS G 800m: 23 rows, 0 grades, 0 AN ids | 2026-09-20T09:10:51-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_12574/event_summary.json` | GET | 200 | AQ MITS #2 (2022-01-22): 27 events | 2026-09-20T09:10:55-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_30103/event_summary.json` | GET | 200 | AQ MITS #3 (2024-02-03): 22 events | 2026-09-20T09:10:57-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/402116` | GET | 200 | 2022 AQ MITS #2 G LJ: 9 rows, 0 grades, 0 AN ids | 2026-09-20T09:11:00-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/1082624` | GET | 200 | 2024 AQ MITS #3 G SP: 7 rows, 0 grades, 0 AN ids | 2026-09-20T09:11:01-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_43298/event_summary.json` | GET | 200 | AQ MITS #2 (2025-01-25): 24 events | 2026-09-20T09:11:05-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_61819/event_summary.json` | GET | 200 | GVSU MITS #5 (2026-02-21): 37 events (31 individual, 6 relay) | 2026-09-20T09:11:07-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/1572722` | GET | 200 | 2025-01-25 AQ MITS #2 G LJ: 14 rows, 0 grades, 0 AN ids | 2026-09-20T09:11:10-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267002` | GET | 200 | 2026-02-21 GVSU MITS #5 G 200m: 6/6 graded, 6/6 AN ids; row carries result id, heat/lane, SB/PR flags, athlete+team AN ids | 2026-09-20T09:11:11-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_43498/event_summary.json` | GET | 200 | AQ MITS #3 (2025-02-01): 26 events | 2026-09-20T09:11:16-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_43408/event_summary.json` | GET | 200 | CMU MITS (2025-02-02, chttiming): 30 events | 2026-09-20T09:11:18-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/1584309` | GET | 200 | 2025-02-01 AQ MITS #3 B 60m: 77 rows, 0 grades, 0 AN ids | 2026-09-20T09:11:21-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/1577188` | GET | 200 | 2025-02-02 CMU MITS B 1600m (chttiming): 43/43 rows graded, 43/43 AN ids | 2026-09-20T09:11:22-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_59759/event_summary.json` | GET | 200 | AQ MITS #1 (2025-12-13): 24 events | 2026-09-20T09:11:29-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2183932` | GET | 200 | 2025-12-13 AQ MITS #1 B 1 Mile: doc exists but 0 rows | 2026-09-20T09:11:32-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2183950` | GET | 200 | 2025-12-13 AQ MITS #1 B 60m: 32 rows, 0 grades, 2 AN ids | 2026-09-20T09:11:40-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2183940` | GET | 200 | 2025-12-13 AQ MITS #1 G 200m: 14 rows, 0 grades, 0 AN ids | 2026-09-20T09:11:42-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/2023-mhsaa-track-field-regional-info-and-results` | GET | 404 | 404 — old slug exists only under boys-* | 2026-09-20T09:12:29-0500 |
| `https://www.mhsaa.com/sports/girls-track-field/2024-mhsaa-track-field-regional-results` | GET | 404 | 404 — confirms single per-season regional page | 2026-09-20T09:12:30-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country/2023-mhsaa-cross-country-regional-info-and-results` | GET | 404 | 404 — same single-page pattern | 2026-09-20T09:12:32-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country/2024-mhsaa-cross-country-regional-info-and-results` | GET | 404 | 404 — same single-page pattern | 2026-09-20T09:12:33-0500 |
| `https://www.mhsaa.com/sports/girls-cross-country/2026-mhsaa-lp-cross-country-regional-info-and-results` | GET | 404 | 404 — same single-page pattern | 2026-09-20T09:12:36-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2025/Finals/UP-D2-Girls.pdf` | HEAD | 404 | UP girls D2 finals 2025 exists (437,674 B downloaded): 96 graded rows, 17 grade-11 | 2026-09-20T09:13:44-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2025/Finals/UP-D3-Girls.pdf` | HEAD | 404 | UP girls D3 finals 2025 exists | 2026-09-20T09:13:45-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2024/Finals/UP-D1-Girls.pdf` | HEAD | 200 | UP girls D1 finals 2024 exists | 2026-09-20T09:13:46-0500 |
| `https://www.mhsaa.com/sites/default/files/Track%20Field-Boys/2023/UP%20D1Girls.htm` | HEAD | 200 | UP girls D1 finals 2023 (Hy-Tek HTML) | 2026-09-20T09:13:47-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/22girlstfupd1finals.txt` | HEAD | 200 | UP girls D1 finals 2022 (Hy-Tek text) | 2026-09-20T09:14:20-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/22girlstfupd2finals.txt` | HEAD | 200 | UP girls D2 finals 2022 | 2026-09-20T09:14:21-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/22girlstfupd3finals.txt` | HEAD | 200 | UP girls D3 finals 2022 (28,825 B): team rankings + event rows, no grade column | 2026-09-20T09:14:22-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/21trackupd1girlsfinal.txt` | HEAD | 200 | UP girls D1 finals 2021 | 2026-09-20T09:14:23-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/21trackupd2girlsfinal.txt` | HEAD | 200 | UP girls D2 finals 2021 | 2026-09-20T09:14:24-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/21trackupd3girlsfinal.txt` | HEAD | 200 | UP girls D3 finals 2021 | 2026-09-20T09:14:26-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267006` | GET | 200 | GVSU MITS #5 G Weight Throw: 13/13 graded | 2026-09-20T09:16:05-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267008` | GET | 200 | GVSU MITS #5 B Triple Jump: 4/4 graded | 2026-09-20T09:16:06-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267010` | GET | 200 | GVSU MITS #5 G 60m: doc exists (977 B), 0 rows | 2026-09-20T09:16:07-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267012` | GET | 200 | GVSU MITS #5 G Triple Jump: 10/10 graded | 2026-09-20T09:16:09-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2267013` | GET | 200 | GVSU MITS #5 B Weight Throw: 18/18 graded; full field-series and conversion fields present | 2026-09-20T09:16:10-0500 |
| `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_60442/event_summary.json` | GET | 200 | AQ MITS #2 (2026-01-17): 24 events | 2026-09-20T09:16:15-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2210117` | GET | 200 | 2026-01-17 AQ MITS #2 B 400m: 51 rows, 11 graded (~22%), 51/51 AN ids | 2026-09-20T09:16:18-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2210118` | GET | 200 | 2026-01-17 AQ MITS #2 G 1600m: 44 rows, 11 graded (25%), 44/44 AN ids | 2026-09-20T09:16:19-0500 |
| `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2210121` | GET | 200 | 2026-01-17 AQ MITS #2 G 60m: 39 rows, 12 graded (~31%), 39/39 AN ids | 2026-09-20T09:16:20-0500 |
| `https://www.mhsaa.com/sites/default/files/2022-07/20upxcd2girls_0.pdf` | HEAD | 200 | UP girls XC D2 finals 2020 (file name pattern "<YY>upxcd<N>girls_0.pdf") | 2026-09-20T09:17:23-0500 |
| `https://www.mhsaa.com/sites/default/files/Cross%20Country-Boys/2022/Finals/2022%20UP%20D2%20Girls.pdf` | HEAD | 200 | UP girls XC D2 finals 2022 (naming "2022 UP D<N> Girls.pdf") | 2026-09-20T09:17:24-0500 |
| `https://www.mhsaa.com/sites/default/files/Cross%20Country-Girls/2023/UP%20Girls%20D2.pdf` | HEAD | 200 | UP girls XC D2 finals 2023 (naming "UP Girls D<N>.pdf") | 2026-09-20T09:17:25-0500 |
