| URL | method | HTTP status | what it proved | timestamp |
|---|---|---|---|---|
| `https://www.iahsaa.org/ (XC classification table, 2026 season)` | GET | 200¹ | IHSAA XC class list: 337 rows incl. the Sharing Agreements column used for the co-op resolver | 2026-09-20 09:07:57 CDT |
| `https://www.iahsaa.org/schools/ottumwa-christian/` | GET | 200¹ | Ottumwa Christian school page: profile present, no T&F team | 2026-09-20 09:08:01 CDT |
| `https://www.iahsaa.org/schools/great-river-christian/` | GET | 404¹ | Great River Christian has no IHSAA school page (contrast with Ottumwa Christian) | 2026-09-20 09:08:05 CDT |
| `https://www.gobound.com/ia/schools/ottumwachristian` | GET | 200¹ | Bound school page shows a COOP T&F program with host Ottumwa -> 18th co-op guest | 2026-09-20 09:08:34 CDT |
| `https://www.gobound.com/ia/schools/srcs` | GET | 200¹ | Strong Roots Christian: girls Softball only -> no T&F/XC program | 2026-09-20 09:08:45 CDT |
| `https://www.gobound.com/ia/schools/tristatechristian` | GET | 200¹ | Tri-State Christian: no T&F/XC activity listed | 2026-09-20 09:08:56 CDT |
| `https://www.gobound.com/ia/schools/grcschool` | GET | 200¹ | Great River Christian: no T&F/XC activity listed | 2026-09-20 09:09:08 CDT |
| `https://www.gobound.com/ia/schools/schristian` | GET | 200¹ | Southeast Christian Academy: no T&F/XC activity listed | 2026-09-20 09:09:19 CDT |
| `https://www.gobound.com/ia/schools/tlhschools` | GET | 200¹ | The Lighthouse: boys Basketball only -> no T&F/XC program | 2026-09-20 09:09:31 CDT |
| `https://www.iahsaa.org/coaches-administrators/` | GET | 200¹ | IHSAA resource centre: no coach/AD directory (resource links only) | 2026-09-20 09:10:15 CDT |
| `https://www.iahsaa.org/contact/` | GET | 200¹ | IHSAA staff table + literal instruction "Find School Contacts at Bound" -> association publishes no school coach contacts | 2026-09-20 09:10:18 CDT |
| `https://ighsau.org/sitemap_index.xml` | GET | 200¹ | IGHSAU sitemap index: no coach/directory pages | 2026-09-20 09:10:20 CDT |
| `https://ighsau.org/sitemap.xml` | GET | 200¹ | IGHSAU page sitemap: sports/state-meet pages only | 2026-09-20 09:10:44 CDT |
| `https://ighsau.org/wp-sitemap.xml` | GET | 200¹ | alternate WordPress sitemap: same conclusion | 2026-09-20 09:10:49 CDT |
| `https://ighsau.org/robots.txt` | GET | 200¹ | no crawl restrictions relevant to docs; sitemap pointers | 2026-09-20 09:10:37 CDT |
| `https://ighsau.org/` | GET | 200 (80,513 B, re-verified this pass) | home nav: sport pages + "Bound Login"; only one mailbox mail@ighsau.org | 2026-09-20 09:15:20 CDT |
| `https://ighsau.org/staff` | GET | 200 (89,141 B, re-verified this pass) | IGHSAU staff page = executive staff bios; no school coach directory | 2026-09-20 09:15:50 CDT |
| `https://ighsau.org/contact-us` | GET | 200 (60,490 B, re-verified this pass) | IGHSAU contact page = office address/phone + single mailbox mail@ighsau.org | 2026-09-20 09:15:51 CDT |
| `https://www.iatrackcoaches.org/` | GET | 200¹ | IATC site map: membership list, advisory boards, weekly results index | 2026-09-20 09:10:23 CDT |
| `https://www.iatrackcoaches.org/sitemap.xml (+ wp-sitemap)` | GET | 200¹ | IATC sitemaps: no coach-contact pages | 2026-09-20 09:10:35 CDT |
| `https://www.iatrackcoaches.org/membership-list/` | GET | 200¹ | 163 member schools by class (1A 69 / 2A 37 / 3A 33 / 4A 24); zero coach names or sport emails; one membership-payment address on a personal domain (omitted) | 2026-09-20 09:10:57 CDT |
| `https://www.iatrackcoaches.org/track-field-advisory-board/` | GET | 200¹ | IATC T&F advisory board: 9 named ADs/coaches with schools (no emails) | 2026-09-20 09:15:02 CDT |
| `https://www.ames.k12.ia.us/` | GET | 200¹ | district home: staff directory entry point | 2026-09-20 09:11:11 CDT |
| `https://www.ankenyschools.org/` | GET | 200¹ | district home reachable | 2026-09-20 09:11:13 CDT |
| `https://www.waukeeschools.org/` | GET | 200¹ | district home reachable | 2026-09-20 09:11:14 CDT |
| `https://www.wdmcs.org/` | GET | 200¹ | district home reachable | 2026-09-20 09:11:15 CDT |
| `https://www.dowlingcatholic.org/` | GET | 200¹ | district home reachable | 2026-09-20 09:11:16 CDT |
| `https://www.dbqschools.org/` | GET | 200¹ | district home reachable | 2026-09-20 09:11:21 CDT |
| `https://www.linnmar.k12.ia.us/ (+ linnmar.org alias)` | GET | 200¹ | district home + alias both reachable | 2026-09-20 09:11:23 CDT |
| `https://www.{ankenyschools,waukeeschools,wdmcs,dowlingcatholic,dbqschools,linnmar.k12.ia.us}.org/` | GET | 200¹ | duplicate-host confirmation, no extra coach data | - |
| `https://www.iahsaa.org/schools/johnston/` | GET | 200¹ | IHSAA school page template carries no coach contact | 2026-09-20 09:11:44 CDT |
| `https://www.iahsaa.org/schools/{cedar-falls,bettendorf,southeast-polk}/` | GET | 200¹ | same template; no coach contacts -> Bound is the contact carrier | 2026-09-20 09:11:48 CDT |
| `https://ameshighathletics.org/` | GET | 200¹ | Ames athletics home: team pages exist per sport (used for the XC coach page) | 2026-09-20 09:12:06 CDT |
| `https://www.dbqschools.org/coaches/` | GET | 200¹ | Dubuque "coaches" page = 1 district contact (employee-access questions); no per-sport coach emails | 2026-09-20 09:12:07 CDT |
| `https://www.dowlingcatholic.org/athletics` | GET | 200¹ | Dowling athletics: AD names + phone numbers, no published emails | 2026-09-20 09:12:08 CDT |
| `https://www.linnmar.k12.ia.us/athletics/` | GET | 200¹ | Linn-Mar athletics: points at the district staff directory | 2026-09-20 09:12:11 CDT |
| `https://www.waukeeschools.org/athletics/` | GET | 200¹ | Waukee athletics: pointers only | 2026-09-20 09:12:12 CDT |
| `https://www.wdmcs.org/staff/` | GET | 200¹ | WDM staff landing: 1 general mailbox (scroffice@wdmcs.org) | 2026-09-20 09:12:15 CDT |
| `https://www.cfschools.org/` | GET | 200¹ | Cedar Falls district home: staff directory entry point | 2026-09-20 09:12:57 CDT |
| `https://www.bettendorf.k12.ia.us/` | GET | Challenge page (3,036 B JS interstitial) | Bettendorf district site returns a JS client-challenge instead of content -> no coach data; classified browser-application/challenge | 2026-09-20 09:12:59 CDT |
| `https://www.southeastpolk.org/` | GET | 200¹ (278,374 B) | SE Polk district home; its directory page exposes a share link only, no emails | 2026-09-20 09:13:00 CDT |
| `https://ameshighathletics.org/activities/athletics/boys-cross-country/` | GET | 200¹ | Ames boys XC page: "Contact the Coach ... Email Ravi Bhave" = 1 real head-coach mailto | 2026-09-20 09:13:21 CDT |
| `https://ameshighathletics.org/activities/athletics/track-field/` | GET | 404 | Ames has no /track-field/ page at that slug (57114 B "Page not found" body) | 2026-09-20 09:13:36 CDT |
| `https://ameshighathletics.org/activities/athletics/track-field/` | GET | 404 (re-verified this pass) | duplicate confirmation of the 404; the T&F slug differs | 2026-09-20 09:15:53 CDT |
| `https://www.wdmcs.org/staff-directory/` | GET | 200¹ | WDM directory page: 0 emails in body | 2026-09-20 09:13:25 CDT |
| `https://www.cfschools.org/staff-directory/` | GET | 200¹ | Cedar Falls directory slice: 878 published staff emails incl. Athletics & Activities Director (justin.urbanek@), Assoc. Principal/Asst. Athletic & Activities Director, Assistant Principal/Activities Director; 29 role labels containing "Coach" (7 "Head Coach", no sport suffix in the slice) | 2026-09-20 09:13:27 CDT |
| `https://www.southeastpolk.org/directory/` | GET | 200¹ | SE Polk directory: no addresses exposed | 2026-09-20 09:13:29 CDT |
| `https://www.waukeeschools.org/directory/` | GET | 200¹ | Waukee directory slice: 176 published addresses incl. 3 Activities roles (Northwest HS Activities Director, Waukee HS Activities Director, Assistant Activities Director) + Activities Secretary | 2026-09-20 09:13:30 CDT |
| `https://www.linnmar.k12.ia.us/staff-directory/` | GET | 200¹ | Linn-Mar directory slice: 10 published addresses (paged directory, none coach/AD-labelled in the returned slice) | 2026-09-20 09:13:32 CDT |
| `https://www.dbqschools.org/staff-directory/` | GET | 200¹ | Dubuque directory page: 0 emails in body | 2026-09-20 09:13:33 CDT |
| `https://www.ankenyschools.org/staff-directory/` | GET | 200¹ | Ankeny staff directory: raw HTML contains no "mailto" and no email literal -> 0 coach contacts | 2026-09-20 09:14:53 CDT |
| `POST https://search.athletic.live/live_results_meet_list/_search  {"query":{"term":{"lsa":"Iowa"}},"size":1000}` | POST | 200 | 772 Iowa meet docs returned (total.value=772, relation eq); 765 carry ani (99%), 0 carry aci | 2026-09-20 09:14:17 CDT |
| `POST .../live_results_meet_list/_search (count form)` | POST | 200 | count-only confirmation of the 772 figure | 2026-09-20 09:14:19 CDT |
| `POST https://search.athletic.live/*_meet_list/_search {"size":1,...}` | POST | 200 (408 B, zero hits) | union-pattern query returned no hits: term filter on lsa is not usable across the 256-index union (mapping conflict); concrete-index queries are the working route | 2026-09-20 09:17:43 CDT |
| `POST https://search.athletic.live/*_meet_list/_search (aggs by_machine/by_tna/date_histogram)` | POST | 400 | union-pattern aggregation rejected (mapping conflict across tenant indices) — recorded failure, worked around with the concrete index | 2026-09-20 09:17:57 CDT |
| `POST https://search.athletic.live/*_meet_list/_search {"query":{"bool":{"filter":[{"term":{"lsa":"Iowa"}},{"range":{"sd":{"gte":"2026-01-01"}}}]}},"aggs":{"by_machine":...}}` | POST | 200 (0 hits) + 2nd attempt 400 (sort on d rejected) | sd range/aggregations on the union pattern return nothing usable; concrete-index route used instead | 2026-09-20 09:18:04 CDT |
| `local derivation from the retained 772-doc response + tools/iowa-11 CSVs` | - | - | provider census (by tna) for the 2026 calendar + the 379-member resolution counts | 2026-09-20 09:18:19 CDT |
| `https://shannoneventtimings.anet.live/` | GET | 200 (50,184 B, re-verified this pass) | Shannon Event Timings = AthleticLIVE white-label (identical SPA shell) -> confirm the machine is a real Iowa operator | 2026-09-20 09:14:56 CDT |
| `https://live.kauderresults.com/` | GET | 200 (50,184 B, re-verified this pass) | Kauder Racing = AthleticLIVE white-label | 2026-09-20 09:14:54 CDT |
| `https://live.rapidresultstiming.com/` | GET | 200 (50,184 B, re-verified this pass) | Rapid Results Timing = AthleticLIVE white-label | 2026-09-20 09:14:58 CDT |
| `https://aatiming.com/` | GET | 200 (50,184 B, re-verified this pass) | AA Timing (tna "All-American Timing") = AthleticLIVE white-label | 2026-09-20 09:14:59 CDT |
