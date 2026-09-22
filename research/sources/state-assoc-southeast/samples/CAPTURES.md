# CAPTURES — state-assoc-southeast (FL, GA, NC, SC, VA, WV)

All captures: anonymous `curl` (no cookies, no auth, no proxy). UA used:
`athletic-rust-pipeline-research/0.1 (anonymous research crawl; contact lewis@localhost)`.
Per-host request spacing respected robots `Crawl-delay` where published (fhsaa.com 5s, ghsa.net 10s)
and otherwise kept to >=1 request/second. Each row: local file - URL - HTTP status - bytes - UTC timestamp.

## Lane-owned captures (this directory)

- `samples/tfrrs-fl-list-5587-4a-region-1.html` — `https://florida.tfrrs.org/lists/5587/FHSAA_4A_Region_1_Sanctioned_SUBJECT_TO_CHANGE` — HTTP 200 — 2,522,360 bytes on disk (curl body 2522360 bytes) — 2026-09-22T04:07:07Z — TFRRS FL 4A Region 1 sanctioned performance list (36 event sections, athlete `Year` = FR/SO/JR/SR)
- `samples/tfrrs-fl-meet-index.html` — `https://florida.tfrrs.org/tf_meets.html?per_page=25` — HTTP 404 (498-byte body `Not Found`) — 2026-09-22T04:07:09Z — guessed meet-index path is not a real route; recorded as a negative result

- `samples/robots-fhsaa.txt` — `https://www.fhsaa.com/robots.txt` — HTTP 200 — 5489 bytes on disk (curl body 5489 bytes) — 2026-09-22T03:54:10Z
- `samples/home-fhsaa.html` — `https://www.fhsaa.com/` — HTTP 200 — 152120 bytes on disk (curl body 152120 bytes) — 2026-09-22T03:54:30Z
- `samples/member-directory-fhsaa.html` — `https://fhsaa.com/sports/2020/1/28/member_directory.aspx` — HTTP 200 — 96887 bytes on disk (curl body 96887 bytes) — 2026-09-22T03:55:07Z
- `samples/sport-cross-fhsaa.html` — `https://fhsaa.com/index.aspx?path=cross` — HTTP 200 — 145799 bytes on disk (curl body 145799 bytes) — 2026-09-22T03:55:13Z
- `samples/sport-track-fhsaa.html` — `https://fhsaa.com/index.aspx?path=track` — HTTP 200 — 123361 bytes on disk (curl body 123361 bytes) — 2026-09-22T03:55:20Z
- `samples/xc-results-fhsaa.html` — `https://fhsaa.com/sports/2020/1/28/XC_results.aspx` — HTTP 200 — 108192 bytes on disk (curl body 108192 bytes) — 2026-09-22T03:56:39Z
- `samples/tk-regions-fhsaa.html` — `https://fhsaa.com/sports/2020/5/18/TK_Regions.aspx` — HTTP 200 — 97034 bytes on disk (curl body 97034 bytes) — 2026-09-22T03:56:45Z
- `samples/membership-fhsaa.html` — `https://fhsaa.com/sports/2020/1/30/Membership.aspx` — HTTP 200 — 113559 bytes on disk (curl body 113559 bytes) — 2026-09-22T03:58:40Z
- `samples/robots-fhsaa-homecampus.txt` — `https://fhsaa.homecampus.com/robots.txt` — HTTP 200 — 24 bytes on disk (curl body 24 bytes) — 2026-09-22T03:55:38Z
- `samples/homecampus-school-directory.html` — `https://fhsaa.homecampus.com/widget/school/directory` — HTTP 200 — 230370 bytes on disk (curl body 230370 bytes) — 2026-09-22T03:55:39Z
- `samples/homecampus-schools-get.json` — `https://fhsaa.homecampus.com/widget/schools/get?school=abel&section_id=10&status=active&hide_from_directory=0` — HTTP 200 — 109 bytes on disk (curl body 109 bytes) — 2026-09-22T03:56:08Z
- `samples/homecampus-school-details-2397.json` — `https://fhsaa.homecampus.com/widget/get-school-details/2397/details` — **two attempts**: 03:56:01Z → HTTP 403 (26,609-byte HTML error body, request sent without the `X-Requested-With` header); 03:56:09Z → HTTP 200 `application/json` (2,301 bytes; `.meta.txt` shows `HTTP/2 200`, `via: 1.1 …cloudfront.net (CloudFront)`, `x-cache: Miss from cloudfront`) — **the file on disk is the 200 JSON body** — 2026-09-22T03:56:09Z. The 403 retry behaviour is itself a finding: the endpoint requires `X-Requested-With: XMLHttpRequest`.
- `samples/robots-florida-tfrrs.txt` — `https://florida.tfrrs.org/robots.txt` — HTTP 200 — 99 bytes on disk (curl body 99 bytes) — 2026-09-22T03:56:51Z
- `samples/tfrrs-fl-state-series.html` — `https://florida.tfrrs.org/state_series.html` — HTTP 200 — 118444 bytes on disk (curl body 118444 bytes) — 2026-09-22T03:56:55Z
- `samples/tfrrs-fl-class-4a.html` — `https://florida.tfrrs.org/lists.html?class=4a` — HTTP 200 — 60948 bytes on disk (curl body 60948 bytes) — 2026-09-22T03:57:32Z
- `samples/tfrrs-fl-archive-2026.html` — `https://florida.tfrrs.org/archives.html?year=2026` — HTTP 200 — 60959 bytes on disk (curl body 60959 bytes) — 2026-09-22T03:57:34Z
- `samples/robots-ghsa.txt` — `https://www.ghsa.net/robots.txt` — HTTP 200 — 2189 bytes on disk (curl body 2189 bytes) — 2026-09-22T03:54:11Z
- `samples/home-ghsa.html` — `https://www.ghsa.net/` — HTTP 200 — 79339 bytes on disk (curl body 79339 bytes) — 2026-09-22T03:54:31Z
- `samples/school-directory-ghsa.html` — `https://www.ghsa.net/school-directory` — HTTP 200 — 81420 bytes on disk (curl body 81420 bytes) — 2026-09-22T03:55:07Z
- `samples/track-field-ghsa.html` — `https://www.ghsa.net/track-and-field` — HTTP 200 — 82143 bytes on disk (curl body 82143 bytes) — 2026-09-22T03:55:18Z
- `samples/cross-country-ghsa.html` — `https://www.ghsa.net/cross-country` — HTTP 200 — 83746 bytes on disk (curl body 83746 bytes) — 2026-09-22T03:55:29Z
- `samples/state-championships-ghsa.html` — `https://www.ghsa.net/state-championships` — HTTP 200 — 71836 bytes on disk (curl body 71836 bytes) — 2026-09-22T03:55:41Z
- `samples/prior-years-results-ghsa.html` — `https://www.ghsa.net/ghsa-constitutions-and-results-prior-years` — HTTP 200 — 63514 bytes on disk (curl body 63514 bytes) — 2026-09-22T03:57:32Z
- `samples/maxpreps-stats-ghsa.html` — `https://www.ghsa.net/maxpreps-stats` — HTTP 200 — 70162 bytes on disk (curl body 70162 bytes) — 2026-09-22T03:57:44Z
- `samples/coaching-ghsa.html` — `https://www.ghsa.net/coaching` — HTTP 200 — 72103 bytes on disk (curl body 72103 bytes) — 2026-09-22T03:57:55Z
- `samples/ghsa-directory-feed.pdf` — `https://www.ghsa.net/ghsa-directory-feed/pdf` — HTTP 200 — 644097 bytes on disk (curl body 644097 bytes) — 2026-09-22T03:58:35Z
- `samples/ghsa-results-records.html` — `https://www.ghsa.net/ghsa-results-and-records-state-playoff-events` — HTTP 200 — 62997 bytes on disk (curl body 62997 bytes) — 2026-09-22T04:00:14Z
- `samples/ghsa-state-track-meet-results.html` — `https://www.ghsa.net/ghsa-state-track-and-field-meet-results` — HTTP 200 — 55847 bytes on disk (curl body 55847 bytes) — 2026-09-22T04:00:42Z
- `samples/hytek-codes-ghsa.html` — `https://www.ghsa.net/hytek-codes-ghsa-schools` — HTTP 200 — 50496 bytes on disk (curl body 50496 bytes) — 2026-09-22T04:00:53Z
- `samples/ghsa-2026-girls-6a-track-results.pdf` — `https://www.ghsa.net/sites/default/files/documents/track/2026_GHSA_Girls_6A_Complete_T-F_State_Meet_Results.pdf` — HTTP 200 — 96706 bytes on disk (curl body 96706 bytes) — 2026-09-22T04:01:05Z
- `samples/ghsa-xc-state-meet-results.html` — `https://www.ghsa.net/ghsa-cross-country-state-meet-results` — HTTP 200 — 95559 bytes on disk (curl body 95559 bytes) — 2026-09-22T04:01:13Z
- `samples/robots-nchsaa.txt` — `https://www.nchsaa.org/robots.txt` — HTTP 200 — 172 bytes on disk (curl body 172 bytes) — 2026-09-22T03:54:13Z
- `samples/home-nchsaa.html` — `https://www.nchsaa.org/` — HTTP 200 — 148414 bytes on disk (curl body 148414 bytes) — 2026-09-22T03:54:33Z
- `samples/sitemap-nchsaa.xml` — `https://www.nchsaa.org/wp-sitemap.xml` — HTTP 200 — 3401 bytes on disk (curl body 3401 bytes) — 2026-09-22T03:55:09Z
- `samples/schools-nchsaa.html` — `https://www.nchsaa.org/schools/` — HTTP 200 — 346464 bytes on disk (curl body 346464 bytes) — 2026-09-22T03:55:11Z
- `samples/ad-directory-nchsaa.html` — `https://www.nchsaa.org/schools/athletic-directors/` — HTTP 200 — 119027 bytes on disk (curl body 119027 bytes) — 2026-09-22T03:55:13Z
- `samples/sport-track-nchsaa.html` — `https://www.nchsaa.org/sports/track-and-field/` — HTTP 200 — 123338 bytes on disk (curl body 123338 bytes) — 2026-09-22T03:57:31Z
- `samples/sport-cross-nchsaa.html` — `https://www.nchsaa.org/sports/cross-country/` — HTTP 200 — 126562 bytes on disk (curl body 126562 bytes) — 2026-09-22T03:57:33Z
- `samples/champ-track-nchsaa.html` — `https://www.nchsaa.org/championships/track-field-state-championships/` — HTTP 200 — 156678 bytes on disk (curl body 156678 bytes) — 2026-09-22T03:57:35Z
- `samples/champ-cross-nchsaa.html` — `https://www.nchsaa.org/championships/cross-country/` — HTTP 200 — 131562 bytes on disk (curl body 131562 bytes) — 2026-09-22T03:57:38Z
- `samples/coaches-nchsaa.html` — `https://www.nchsaa.org/coaches/` — HTTP 200 — 128081 bytes on disk (curl body 128081 bytes) — 2026-09-22T03:57:39Z
- `samples/wpjson-types-nchsaa.json` — `https://www.nchsaa.org/wp-json/wp/v2/types` — HTTP 200 — 48172 bytes on disk (curl body 48172 bytes) — 2026-09-22T03:57:42Z
- `samples/xc-champs-2025-nchsaa.html` — `https://www.nchsaa.org/2025-cross-country-championships/` — HTTP 200 — 179545 bytes on disk (curl body 179545 bytes) — 2026-09-22T03:58:31Z
- `samples/sports-champs-nchsaa.html` — `https://www.nchsaa.org/sports-championships/` — HTTP 200 — 256646 bytes on disk (curl body 256646 bytes) — 2026-09-22T03:58:33Z
- `samples/about-nchsaa.html` — `https://www.nchsaa.org/about/` — HTTP 200 — 110782 bytes on disk (curl body 110782 bytes) — 2026-09-22T03:58:35Z
- `samples/wpjson-search-nchsaa.json` — `https://www.nchsaa.org/wp-json/wp/v2/search?search=track%20and%20field%20state%20championship&per_page=5` — HTTP 200 — 2352 bytes on disk (curl body 2352 bytes) — 2026-09-22T03:58:52Z
- `samples/xc-2025-5a-girls-nchsaa.pdf` — `https://www.nchsaa.org/wp-content/uploads/2025/11/5A-Girls-Results.pdf` — HTTP 200 — 314488 bytes on disk (curl body 314488 bytes) — 2026-09-22T04:00:12Z
- `samples/record-books-nchsaa.html` — `https://www.nchsaa.org/record-books/` — HTTP 200 — 148440 bytes on disk (curl body 148440 bytes) — 2026-09-22T04:00:13Z
- `samples/track-champs-nchsaa.html` — `https://www.nchsaa.org/track-and-field-state-championships/` — HTTP 404 — 347648 bytes on disk (curl body 347648 bytes) — 2026-09-22T04:00:16Z
- `samples/robots-schsl.txt` — `https://schsl.org/robots.txt` — HTTP 200 — 110 bytes on disk (curl body 110 bytes) — 2026-09-22T03:54:15Z
- `samples/home-schsl.html` — `https://schsl.org/` — HTTP 200 — 150954 bytes on disk (curl body 150954 bytes) — 2026-09-22T03:54:35Z
- `samples/directory-schsl.html` — `https://schsl.org/schsl-directory` — HTTP 200 — 82477 bytes on disk (curl body 82477 bytes) — 2026-09-22T03:55:15Z
- `samples/track-field-schsl.html` — `https://schsl.org/track-field` — HTTP 200 — 92386 bytes on disk (curl body 92386 bytes) — 2026-09-22T03:55:17Z
- `samples/cross-country-schsl.html` — `https://schsl.org/cross-country` — HTTP 200 — 146630 bytes on disk (curl body 146630 bytes) — 2026-09-22T03:55:20Z
- `samples/sitemap-schsl.xml` — `https://schsl.org/wp-sitemap.xml` — HTTP 200 — 829 bytes on disk (curl body 829 bytes) — 2026-09-22T03:55:22Z
- `samples/class-aaaaa-schsl.html` — `https://schsl.org/aaaaa` — HTTP 200 — 127465 bytes on disk (curl body 127465 bytes) — 2026-09-22T03:57:31Z
- `samples/class-a-schsl.html` — `https://schsl.org/a` — HTTP 200 — 96342 bytes on disk (curl body 96342 bytes) — 2026-09-22T03:57:33Z
- `samples/xc-state-champions-2026-schsl.html` — `https://schsl.org/archives/19510` — HTTP 200 — 132034 bytes on disk (curl body 132034 bytes) — 2026-09-22T03:57:36Z
- `samples/xc-state-championships-2025-schsl.html` — `https://schsl.org/archives/16948` — HTTP 200 — 149578 bytes on disk (curl body 149578 bytes) — 2026-09-22T03:57:39Z
- `samples/reclassification-2026-2028-schsl.html` — `https://schsl.org/archives/17912` — HTTP 200 — 146261 bytes on disk (curl body 146261 bytes) — 2026-09-22T03:58:34Z
- `samples/champ-info-category-schsl.html` — `https://schsl.org/archives/category/championship-information` — HTTP 200 — 158284 bytes on disk (curl body 158284 bytes) — 2026-09-22T03:58:37Z
- `samples/rewind-schsl.html` — `https://schsl.org/rewind` — HTTP 200 — 182279 bytes on disk (curl body 182279 bytes) — 2026-09-22T03:58:39Z
- `samples/search-xc-results-schsl.html` — `https://schsl.org/?s=cross+country+results` — HTTP 200 — 132511 bytes on disk (curl body 132511 bytes) — 2026-09-22T04:00:13Z
- `samples/brackets-schsl.html` — `https://schsl.org/brackets` — HTTP 200 — 125130 bytes on disk (curl body 125130 bytes) — 2026-09-22T04:00:16Z
- `samples/xc-championships-schsl.html` — `https://schsl.org/archives/4541` — HTTP 200 — 153035 bytes on disk (curl body 153035 bytes) — 2026-09-22T04:00:59Z
- `samples/robots-wvssac.txt` — `https://www.wvssac.org/robots.txt` — HTTP 404 — 13 bytes on disk (curl body 13 bytes) — 2026-09-22T03:54:18Z
- `samples/home-wvssac.html` — `https://www.wvssac.org/` — HTTP 200 — 696835 bytes on disk (curl body 696835 bytes) — 2026-09-22T03:54:37Z
- `samples/school-directory-wvssac.html` — `https://www.wvssac.org/school-resources/school-directory/` — HTTP 200 — 272670 bytes on disk (curl body 272670 bytes) — 2026-09-22T03:55:24Z
- `samples/track-wvssac.html` — `https://www.wvssac.org/sports/track/` — HTTP 200 — 428535 bytes on disk (curl body 428535 bytes) — 2026-09-22T03:55:26Z
- `samples/cross-country-wvssac.html` — `https://www.wvssac.org/sports/cross-country/` — HTTP 200 — 427192 bytes on disk (curl body 427192 bytes) — 2026-09-22T03:55:28Z
- `samples/classifications-wvssac.html` — `https://www.wvssac.org/school-resources/classifications-regional-alignment/` — HTTP 200 — 274310 bytes on disk (curl body 274310 bytes) — 2026-09-22T03:55:30Z
- `samples/wpjson-page-8039-wvssac.json` — `https://www.wvssac.org/wp-json/wp/v2/pages/8039` — HTTP 200 — 4347 bytes on disk (curl body 4347 bytes) — 2026-09-22T03:58:33Z
- `samples/classifications-2025-27-wvssac.pdf` — `https://assets-rst7.rschooltoday.com/rst7files/uploads/sites/588/2026/07/03113435/Classifications-25-27.pdf` — HTTP 200 — 126126 bytes on disk (curl body 126126 bytes) — 2026-09-22T03:58:35Z
- `samples/wvmetronews-track-roundup-wvssac.html` — `https://www.wvssac.org/wvmetronews-track-championship-roundup/` — HTTP 200 — 98022 bytes on disk (curl body 98022 bytes) — 2026-09-22T03:58:38Z
- `samples/wpjson-search-track-wvssac.json` — `https://www.wvssac.org/wp-json/wp/v2/search?search=state%20track%20meet%20results&per_page=10` — HTTP 200 — 2 bytes on disk (curl body 2 bytes) — 2026-09-22T04:01:35Z
- `samples/programs-wvssac.html` — `https://www.wvssac.org/school-resources/programs/` — HTTP 200 — 287066 bytes on disk (curl body 287066 bytes) — 2026-09-22T04:01:37Z
- `samples/milestat-redirect.html` — `http://milestat.com/` — HTTP 200 — 103526 bytes on disk (curl body 103526 bytes) — 2026-09-22T03:59:38Z
- `samples/home-va-milesplit.html` — `https://va.milesplit.com/` — HTTP 200 — 103527 bytes on disk (curl body 103527 bytes) — 2026-09-22T03:59:40Z
- `samples/meets-va-milesplit.html` — `https://va.milesplit.com/meets` — HTTP 404 — 25266 bytes on disk (curl body 25266 bytes) — 2026-09-22T03:59:42Z
- `samples/vhsl-class5-state-meet-va-milesplit.html` — `https://va.milesplit.com/meets/742518` — HTTP 200 — 61148 bytes on disk (curl body 61148 bytes) — 2026-09-22T04:00:14Z
- `samples/teams-va-milesplit.html` — `https://va.milesplit.com/teams` — HTTP 200 — 208364 bytes on disk (curl body 208364 bytes) — 2026-09-22T04:00:16Z

## Captured by the sibling lane `state-assoc-midatlantic` (cited, not re-fetched)

- `../state-assoc-midatlantic/samples/robots-vhsl.txt` — `https://www.vhsl.org/robots.txt` — captured 2026-09-21 by lane `state-assoc-midatlantic` — verbatim body: `User-agent: *` / `Disallow: /` (whole-site disallow for all agents except `RavenCrawler` and `Googlebot`).
- `../state-assoc-midatlantic/samples/robots-va-milesplit.txt` — `https://va.milesplit.com/robots.txt` — captured 2026-09-21 by lane `state-assoc-midatlantic` — verbatim body disallows `/rankings`, `/virtual-meets`, `/api/`, `/contact` for `*`.

