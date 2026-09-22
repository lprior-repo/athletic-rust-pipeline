# Captures — `research/sources/milesplit-national/samples/`

Byte-exact as-fetched captures plus the machine logs that record how each file was produced. Two waves:
**wave 1** = the capture set already on disk when this lane started (by the prior phase), **wave 2** = this lane (2026-09-22T03:54Z–04:00Z).

## Fetch conventions (every wave-2 row)

```sh
UA='Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36'
# sequential requests only; >=1.3 s between successive requests; <=1 request/second/host;
# -A "$UA" --compressed --max-time 30|40; cookies only where the command says -b/-c
```

- **`transfer`** = curl `%{size_download}` with `--compressed`: the on-wire (gzip) size. **`on disk`** = the decoded artifact size, which is the number to compare against other captures of the same URL.
- Wave-1 rows carry no command (that phase did not record one) and their byte column is the size that phase printed; where a URL column says *canonical*, the URL was read from the `<link rel="canonical">` inside the captured bytes, and `[INFERENCE]` marks a URL rebuilt from other evidence.
- `robots.txt` was read before the first request to each host, and the captures are in this directory. No request was made to `/rankings`, `/api/`, `/virtual-meets` or `/contact` in wave 2.
- The six `teams-<code>.html` rows below are the kept bodies of the 51-jurisdiction `/teams` sweep; the other 45 bodies were counted and discarded, counts + per-host command in `samples/teams-count-sweep.tsv`.

## Wave 2 — this lane (40 files)

| file | URL | HTTP | transfer | on disk | UTC | exact command |
|---|---|---|---|---|---|---|
| `robots-oh-milesplit.txt` | https://oh.milesplit.com/robots.txt | 200 | 173 | 173 | 2026-09-22T03:54:43Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/robots-oh-milesplit.txt -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/robots.txt` |
| `robots-www-milesplit.txt` | https://www.milesplit.com/robots.txt | 200 | 173 | 173 | 2026-09-22T03:54:45Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/robots-www-milesplit.txt -w '%{http_code} %{size_download} %{content_type}' https://www.milesplit.com/robots.txt` |
| `robots-js-sp-milesplit.txt` | https://js.sp.milesplit.com/robots.txt | 404 | 548 | 548 | 2026-09-22T03:54:46Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/robots-js-sp-milesplit.txt -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/robots.txt` |
| `robots-ca-milesplit.txt` | https://ca.milesplit.com/robots.txt | 200 | 173 | 173 | 2026-09-22T03:54:48Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/robots-ca-milesplit.txt -w '%{http_code} %{size_download} %{content_type}' https://ca.milesplit.com/robots.txt` |
| `robots-tx-milesplit.txt` | https://tx.milesplit.com/robots.txt | 200 | 173 | 173 | 2026-09-22T03:54:49Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/robots-tx-milesplit.txt -w '%{http_code} %{size_download} %{content_type}' https://tx.milesplit.com/robots.txt` |
| `js-rankings.js` | https://js.sp.milesplit.com/drivefaze/rankings.js?build=20260921153357 | 200 | 7915 | 7915 | 2026-09-22T03:54:51Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-rankings.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/drivefaze/rankings.js?build=20260921153357` |
| `js-paywall.js` | https://js.sp.milesplit.com/drivefaze/pro/paywall.js?build=20260921153357 | 200 | 583 | 583 | 2026-09-22T03:54:52Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-paywall.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/drivefaze/pro/paywall.js?build=20260921153357` |
| `js-ms18-teams-index.js` | https://js.sp.milesplit.com/ms18/teams/index.js?build=20260921153357 | 200 | 4846 | 4846 | 2026-09-22T03:54:54Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-ms18-teams-index.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/ms18/teams/index.js?build=20260921153357` |
| `js-ms18-teams-roster-index.js` | https://js.sp.milesplit.com/ms18/teams/roster/index.js?build=20260921153357 | 200 | 1371 | 1371 | 2026-09-22T03:54:55Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-ms18-teams-roster-index.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/ms18/teams/roster/index.js?build=20260921153357` |
| `js-results-index.js` | https://js.sp.milesplit.com/drivefaze/results/index.js?build=20260921153357 | 200 | 21455 | 21455 | 2026-09-22T03:54:56Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-results-index.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/drivefaze/results/index.js?build=20260921153357` |
| `js-calendar-index.js` | https://js.sp.milesplit.com/drivefaze/calendar/index.js?build=20260921153357 | 200 | 12198 | 12198 | 2026-09-22T03:54:58Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-calendar-index.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/drivefaze/calendar/index.js?build=20260921153357` |
| `js-meets.js` | https://js.sp.milesplit.com/drivefaze/meets/meets.js?build=20260921153357 | 200 | 2377 | 2377 | 2026-09-22T03:54:59Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-meets.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/drivefaze/meets/meets.js?build=20260921153357` |
| `js-search-athletes.js` | https://js.sp.milesplit.com/ms18/search/athletes.js?build=20260921153357 | 200 | 4808 | 4808 | 2026-09-22T03:55:01Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-search-athletes.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/ms18/search/athletes.js?build=20260921153357` |
| `meet-oh-770621-results.html` | https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results | 200 | 45308 | 45308 | 2026-09-22T03:55:44Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/meet-oh-770621-results.html -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results` |
| `js-loadresultsnew.js` | https://js.sp.milesplit.com/drivefaze/meets/loadResultsNew.js?build=20260921153357 | 200 | 49072 | 49072 | 2026-09-22T03:55:55Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-loadresultsnew.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/drivefaze/meets/loadResultsNew.js?build=20260921153357` |
| `js-meets-results.js` | https://js.sp.milesplit.com/drivefaze/meets/results.js?build=20260921153357 | 200 | 18089 | 18089 | 2026-09-22T03:55:56Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-meets-results.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/drivefaze/meets/results.js?build=20260921153357` |
| `js-hashtable.js` | https://js.sp.milesplit.com/drivefaze/hashtable.js?build=20260921153357 | 200 | 6774 | 6774 | 2026-09-22T03:55:58Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-hashtable.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/drivefaze/hashtable.js?build=20260921153357` |
| `raw-oh-770621-rs1321880.txt` | https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw | 200 | 47564 | 47564 | 2026-09-22T03:55:59Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/raw-oh-770621-rs1321880.txt -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results/1321880/raw` |
| `teams-ca.html` | https://ca.milesplit.com/teams | 200 | 66567 | 621163 | 2026-09-22T03:56:34Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/teams-ca.html -w '%{http_code} %{size_download} %{content_type}' https://ca.milesplit.com/teams` |
| `teams-fl.html` | https://fl.milesplit.com/teams | 200 | 37442 | 332809 | 2026-09-22T03:56:43Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/teams-fl.html -w '%{http_code} %{size_download} %{content_type}' https://fl.milesplit.com/teams` |
| `teams-il.html` | https://il.milesplit.com/teams | 200 | 31763 | 277314 | 2026-09-22T03:56:50Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/teams-il.html -w '%{http_code} %{size_download} %{content_type}' https://il.milesplit.com/teams` |
| `teams-ny.html` | https://ny.milesplit.com/teams | 200 | 46965 | 419778 | 2026-09-22T03:57:23Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/teams-ny.html -w '%{http_code} %{size_download} %{content_type}' https://ny.milesplit.com/teams` |
| `teams-pa.html` | https://pa.milesplit.com/teams | 200 | 35026 | 297673 | 2026-09-22T03:57:34Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/teams-pa.html -w '%{http_code} %{size_download} %{content_type}' https://pa.milesplit.com/teams` |
| `teams-tx.html` | https://tx.milesplit.com/teams | 200 | 75461 | 724381 | 2026-09-22T03:57:42Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/teams-tx.html -w '%{http_code} %{size_download} %{content_type}' https://tx.milesplit.com/teams` |
| `search-v2-athletes-oh.json` | https://oh.milesplit.com/search/v2/athletes (POST body: searchToken=<stale, from wave-1 page>, q=Fuller, perPage=25, filters[subdomain]=oh) | 400 | 23798 | 23798 | 2026-09-22T03:58:32Z | `curl -sS -A "$UA" --compressed --max-time 30 -X POST --data-urlencode searchToken=14273bc9b89a24afd28d6ce210190508 --data-urlencode q=Fuller --data-urlencode perPage=25 --data-urlencode 'filters[subdomain]=oh' -o samples/search-v2-athletes-oh.json -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/search/v2/athletes   # POST body: searchToken=<stale, from wave-1 page>, q=Fuller, perPage=25, filters[subdomain]=oh` |
| `search-v2-athletes-oh.headers` | https://oh.milesplit.com/search/v2/athletes (-D header dump) | 400 | n/r | 307 | 2026-09-22T03:58:32Z | `curl -sS -A "$UA" --compressed -D samples/search-v2-athletes-oh.headers ... https://oh.milesplit.com/search/v2/athletes   # response headers dumped by the paired request above` |
| `athletes-oh-searchform.html` | https://oh.milesplit.com/athletes | 200 | 11499 | 55426 | 2026-09-22T03:58:43Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/athletes-oh-searchform.html -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/athletes` |
| `search-v2-athletes-oh-2.headers` | https://oh.milesplit.com/search/v2/athletes (-D header dump) | 200 | n/r | 152 | 2026-09-22T03:58:45Z | `curl -sS -A "$UA" --compressed -D samples/search-v2-athletes-oh-2.headers ... https://oh.milesplit.com/search/v2/athletes   # response headers dumped by the paired request above` |
| `search-v2-athletes-oh-2.json` | https://oh.milesplit.com/search/v2/athletes (POST body: fresh token + jar /tmp/ms-cookies.txt + X-Requested-With) | 200 | 1273 | 5704 | 2026-09-22T03:58:45Z | `curl -sS -A "$UA" --compressed --max-time 30 -b /tmp/ms-cookies.txt -H 'X-Requested-With: XMLHttpRequest' -X POST --data-urlencode searchToken=6c89da652151bb31cd3e7fe634afb717 --data-urlencode q=Fuller --data-urlencode perPage=25 --data-urlencode 'filters[subdomain]=oh' -o samples/search-v2-athletes-oh-2.json -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/search/v2/athletes   # POST body: fresh token + jar /tmp/ms-cookies.txt + X-Requested-With` |
| `search-v2-athletes-oh-fuller-p2.json` | https://oh.milesplit.com/search/v2/athletes (same session, page=2) | 200 | 1247 | 5641 | 2026-09-22T03:58:58Z | `curl -sS -A "$UA" --compressed --max-time 30 -b /tmp/ms-cookies.txt -H 'X-Requested-With: XMLHttpRequest' -X POST --data-urlencode searchToken=6c89da652151bb31cd3e7fe634afb717 --data-urlencode q=Fuller --data-urlencode perPage=25 --data-urlencode page=2 --data-urlencode 'filters[subdomain]=oh' -o samples/search-v2-athletes-oh-fuller-p2.json -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/search/v2/athletes   # same session, page=2` |
| `search-v2-athletes-oh-smith-p1.json` | https://oh.milesplit.com/search/v2/athletes (same session, q=Smith) | 200 | 1193 | 5453 | 2026-09-22T03:58:59Z | `curl -sS -A "$UA" --compressed --max-time 30 -b /tmp/ms-cookies.txt -H 'X-Requested-With: XMLHttpRequest' -X POST --data-urlencode searchToken=6c89da652151bb31cd3e7fe634afb717 --data-urlencode q=Smith --data-urlencode perPage=25 --data-urlencode 'filters[subdomain]=oh' -o samples/search-v2-athletes-oh-smith-p1.json -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/search/v2/athletes   # same session, q=Smith` |
| `search-v2-athletes-oh-perpage100.json` | https://oh.milesplit.com/search/v2/athletes (same session, perPage=100) | 200 | 4361 | 22623 | 2026-09-22T03:59:08Z | `curl -sS -A "$UA" --compressed --max-time 30 -b /tmp/ms-cookies.txt -H 'X-Requested-With: XMLHttpRequest' -X POST --data-urlencode searchToken=6c89da652151bb31cd3e7fe634afb717 --data-urlencode q=Fuller --data-urlencode perPage=100 --data-urlencode 'filters[subdomain]=oh' -o samples/search-v2-athletes-oh-perpage100.json -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/search/v2/athletes   # same session, perPage=100` |
| `athletes-www-searchform.html` | https://www.milesplit.com/athletes | 200 | 11554 | 55517 | 2026-09-22T03:59:09Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/athletes-www-searchform.html -w '%{http_code} %{size_download} %{content_type}' https://www.milesplit.com/athletes` |
| `search-v2-athletes-www.json` | https://www.milesplit.com/search/v2/athletes (POST body: www token, no subdomain filter (national)) | 200 | 1349 | 5608 | 2026-09-22T03:59:11Z | `curl -sS -A "$UA" --compressed --max-time 30 -b /tmp/ms-cookies-www.txt -H 'X-Requested-With: XMLHttpRequest' -X POST --data-urlencode searchToken=1431c0656c355f313fc79ea47c236075 --data-urlencode q=Fuller --data-urlencode perPage=25 -o samples/search-v2-athletes-www.json -w '%{http_code} %{size_download} %{content_type}' https://www.milesplit.com/search/v2/athletes   # POST body: www token, no subdomain filter (national)` |
| `search-v2-athletes-oh-notoken.json` | https://oh.milesplit.com/search/v2/athletes (POST body: empty token, no cookie (negative control)) | 400 | 23799 | 23799 | 2026-09-22T03:59:19Z | `curl -sS -A "$UA" --compressed --max-time 30 -H 'X-Requested-With: XMLHttpRequest' -X POST --data-urlencode searchToken= --data-urlencode q=Fuller --data-urlencode perPage=25 -o samples/search-v2-athletes-oh-notoken.json -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/search/v2/athletes   # POST body: empty token, no cookie (negative control)` |
| `robots-mi-milesplit.txt` | https://mi.milesplit.com/robots.txt | 200 | 173 | 173 | 2026-09-22T03:59:21Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/robots-mi-milesplit.txt -w '%{http_code} %{size_download} %{content_type}' https://mi.milesplit.com/robots.txt` |
| `robots-wy-milesplit.txt` | https://wy.milesplit.com/robots.txt | 200 | 173 | 173 | 2026-09-22T03:59:23Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/robots-wy-milesplit.txt -w '%{http_code} %{size_download} %{content_type}' https://wy.milesplit.com/robots.txt` |
| `team-oh-10002-rankings.html` | https://oh.milesplit.com/teams/10002-mason/rankings | 200 | 9440 | 40215 | 2026-09-22T03:59:51Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/team-oh-10002-rankings.html -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/teams/10002-mason/rankings` |
| `meet-oh-770621-entries.html` | https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/entries | 200 | 10836 | 37101 | 2026-09-22T03:59:53Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/meet-oh-770621-entries.html -w '%{http_code} %{size_download} %{content_type}' https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/entries` |
| `js-ms18-teams-rankings.js` | https://js.sp.milesplit.com/ms18/teams/rankings.js?build=20260921153357 | 200 | 1503 | 8016 | 2026-09-22T04:00:03Z | `curl -sS -A "$UA" --compressed --max-time 30 -o samples/js-ms18-teams-rankings.js -w '%{http_code} %{size_download} %{content_type}' https://js.sp.milesplit.com/ms18/teams/rankings.js?build=20260921153357` |

## Wave 1 — prior phase (pre-existing when this lane started)

| file | URL | bytes | mtime (UTC) | URL provenance |
|---|---|---|---|---|
| `teams-dc.html` | https://dc.milesplit.com/teams | 52995 | 2026-09-22T03:31:14Z | <link rel=canonical> in the bytes |
| `teams-wy.html` | https://wy.milesplit.com/teams | 51750 | 2026-09-22T03:31:15Z | <link rel=canonical> in the bytes |
| `teams-oh.html` | https://oh.milesplit.com/teams | 303979 | 2026-09-22T03:31:14Z | <link rel=canonical> in the bytes |
| `roster-oh-mason.html` | https://oh.milesplit.com/teams/10002-mason/roster | 688106 | 2026-09-22T03:32:14Z | <link rel=canonical> in the bytes |
| `team-oh-10002-mason.html` | https://oh.milesplit.com/teams/10002-mason | 50032 | 2026-09-22T03:32:46Z | <link rel=canonical> in the bytes |
| `meet-oh-771577.html` | https://oh.milesplit.com/meets/771577-wooster-xc-invitational-hs-2026/info | 48219 | 2026-09-22T03:32:46Z | <link rel=canonical> in the bytes |
| `results-oh.html` | https://oh.milesplit.com/results/ohio-meet-results?&page=1 | 187357 | 2026-09-22T03:32:15Z | <link rel=canonical> in the bytes |
| `calendar-oh.html` | https://oh.milesplit.com/calendar/ohio-cc-meet-calendar | 419186 | 2026-09-22T03:32:15Z | <link rel=canonical> in the bytes |
| `rankings-oh-boys-xc.html` | https://oh.milesplit.com/rankings/leaders/high-school-boys/cross-country?year=2026&accuracy=fat&grade=all&conversion=n&page=1 | 150022 | 2026-09-22T03:32:14Z | <link rel=canonical> in the bytes |
| `rank-event-oh-xc-5000m.html` | https://oh.milesplit.com/rankings/events/high-school-boys/cross-country/5000m?year=2026&accuracy=fat&grade=all&conversion=n&page=1 | 212528 | 2026-09-22T03:32:32Z | <link rel=canonical> in the bytes |
| `rank-event-oh-out-1600m.html` | (empty file — see Failures) | 0 | 2026-09-22T03:32:29Z | [INFERENCE] sibling pattern; nothing to read in the file |
| `athlete-oh-15005189.html` | https://oh.milesplit.com/athletes/15005189-brice-fuller | 262649 | 2026-09-22T03:32:44Z | <link rel=canonical> in the bytes |
| `athlete-www-13966799.html` | https://ny.milesplit.com/athletes/13966799-abigail-burt (served on www) | 181876 | 2026-09-22T03:32:47Z | <link rel=canonical> in the bytes |
| `athletes-oh.html` | https://oh.milesplit.com/athletes | 55426 | 2026-09-22T03:32:15Z | <link rel=canonical> in the bytes |
| `homepage.html` | https://www.milesplit.com/ | 113052 | 2026-09-22T03:31:13Z | [INFERENCE] title 'MileSplit United States' + page-call site_code=www (no canonical tag in bytes) |
| `js-api.js` | https://js.sp.milesplit.com/drivefaze/api.js?build=20260921153357 | 6877 | 2026-09-22T03:31:54Z | [INFERENCE] <script src> of the wave-1 pages; build stamp 20260921153357 |
| `js-core.js` | https://js.sp.milesplit.com/drivefaze/core.js?build=20260921153357 | 26334 | 2026-09-22T03:31:55Z | [INFERENCE] <script src> of the wave-1 pages; build stamp 20260921153357 |
| `js-common-ms05.js` | https://js.sp.milesplit.com/drivefaze/common-ms05.js?build=20260921153357 | 6598 | 2026-09-22T03:31:55Z | [INFERENCE] <script src> of the wave-1 pages; build stamp 20260921153357 |
| `js-prereq.js` | https://js.sp.milesplit.com/drivefaze/prereq.js?build=20260921153357 | 2010 | 2026-09-22T03:31:55Z | [INFERENCE] <script src> of the wave-1 pages; build stamp 20260921153357 |
| `state-subdomain-probe.txt` | (derived text file: one GET per host, see its own header) | 3575 | 2026-09-22T03:31:41Z | self-describing; the probe header names the command and the UA |

Wave-1 HTTP statuses were not recorded by that phase. Every non-empty HTML file in that set carries its own `<title>`, and all but `homepage.html` carry a `<link rel=canonical>`, so each parses as the page it claims to be.

## Failures, anomalies, and what was deliberately not fetched

- **`rank-event-oh-out-1600m.html` — 0 bytes: capture failed.** Its URL cannot be recovered from the empty file; the sibling `rank-event-oh-xc-5000m.html` shows the pattern `https://oh.milesplit.com/rankings/events/high-school-boys/<season>/<event>?…` `[INFERENCE]`. It was **not re-captured**: `/rankings` is `Disallow`ed by `robots.txt` on every host sampled, so the re-fetch would itself be a robots violation. Treat the file as unusable; both rankings captures are analysis-only and out of scope for the pipeline for the same reason.
- **`robots-js-sp-milesplit.txt` — HTTP 404** (548 B HTML error page). That asset host publishes no robots.txt. The bundles fetched from it are exactly the URLs the captured pages already request in `<script src>`.
- **`search-v2-athletes-oh.json` — HTTP 400.** The first POST reused a `searchToken` from a page fetched ~4 minutes earlier and sent no cookie: the site answers with a 400 HTML page rather than JSON. Minting token + `unique_id` cookie in one session (`athletes-oh-searchform.html`) makes the same call return 200 JSON. `search-v2-athletes-oh-notoken.json` reproduces the 400 on demand, which is what makes the session precondition a measured fact rather than a guess.
- **`/api/**` was never requested** in either wave (robots `Disallow: /api/`). Everything known about it here is read from the site's own bundles: endpoint templates, parameter names and the field list are quoted in `SOURCE_REPORT.md` from `js-api.js`, `js-loadresultsnew.js`, `js-ms18-teams-index.js` and `js-ms18-teams-rankings.js`. Reachability unauthenticated is therefore **unverified by design**.
- **`meet-oh-770621-results.html`** — `/meets/770621-beaver-eastern-invite-2026/results` lands on that meet's first result set; the capture's canonical is `…/results/1321880/formatted`, which contains 0 result rows (they come from `/api/`). The usable artifact is `raw-oh-770621-rs1321880.txt`.
- **`meet-oh-770621-entries.html` and `team-oh-10002-rankings.html`** — JS shells: 0 data rows, visible text `Loading`. Their widgets read `/api/`, so they carry no free payload (the team page's `/rankings` path is robots-**allowed**, which is why it was captured; the data behind it is not).
- **No login, no PRO entitlement, no paywall bypass, no CAPTCHA work, no Athletics.net host contact** in either wave.

## robots.txt (verbatim captures)

| host | file | HTTP | Disallow set |
|---|---|---|---|
| oh.milesplit.com | `robots-oh-milesplit.txt` | 200 (173 B) | `/rankings`, `/virtual-meets`, `/api/`, `/contact` |
| www.milesplit.com | `robots-www-milesplit.txt` | 200 (173 B) | `/rankings`, `/virtual-meets`, `/api/`, `/contact` |
| ca.milesplit.com | `robots-ca-milesplit.txt` | 200 (173 B) | `/rankings`, `/virtual-meets`, `/api/`, `/contact` |
| tx.milesplit.com | `robots-tx-milesplit.txt` | 200 (173 B) | `/rankings`, `/virtual-meets`, `/api/`, `/contact` |
| mi.milesplit.com | `robots-mi-milesplit.txt` | 200 (173 B) | `/rankings`, `/virtual-meets`, `/api/`, `/contact` |
| wy.milesplit.com | `robots-wy-milesplit.txt` | 200 (173 B) | `/rankings`, `/virtual-meets`, `/api/`, `/contact` |
| js.sp.milesplit.com | `robots-js-sp-milesplit.txt` | 404 (548 B) | none published — the 404 error page is what was captured |
| va.milesplit.com | `research/sources/state-assoc-midatlantic/samples/robots-va-milesplit.txt` (another lane, cited not copied) | 200 (173 B) | `/rankings`, `/virtual-meets`, `/api/`, `/contact` |

All seven hosts that answered publish byte-identical policy: `User-agent: Mediapartners-Google` + `Disallow:`, then `User-agent: *` with those four disallows, then `User-agent: AmazonAdBot` + `Allow: /`.
That makes the policy a network-wide constant, not a per-state variable: **`/teams`, `/teams/<id>/roster`, `/teams/<id>/rankings`, `/meets/**`, `/results`, `/calendar`, `/athletes`, `/search/v2/**`, `/sitemap.xml` are allowed; `/rankings`, `/virtual-meets`, `/api/`, `/contact` are not.**

## Derived (non-capture) files

| file | contents | provenance |
|---|---|---|
| `fetch-log-wave2.tsv` | 40 rows: utc, http, transfer_bytes, on_disk_bytes, file, url | written by the same script that ran the curls |
| `teams-count-sweep.tsv` | 51 rows: code, http, bytes, team_links, utc, url, command | `team_links` counted from each fetched body; the six largest bodies kept as captures, the other 45 discarded after counting; `bytes` there is the compressed transfer size |
| `state-subdomain-probe.txt` | 51 codes + 3 failures with status/bytes | wave 1 |

## Reproduce the headline counts from the captures

```sh
grep -o 'href="https://oh\.milesplit.com/teams/[0-9]*-' samples/teams-oh.html | wc -l   # 977
grep -o 'href="https://[a-z.]*milesplit\.com/teams/[0-9]*-' samples/teams-dc.html | wc -l  # 80
grep -o 'href="https://wy\.milesplit\.com/teams/[0-9]*-' samples/teams-wy.html | wc -l  # 81
grep -o 'data-meet-id="[0-9]*"' samples/calendar-oh.html | sort -u | wc -l                 # 294
grep -o 'data-meet-id="[0-9]*"' samples/results-oh.html | wc -l                           # 50 (one page)
grep -c 'class="rankings-row' samples/rank-event-oh-xc-5000m.html                        # 50
grep -c 'rankings-row lk' samples/rank-event-oh-xc-5000m.html                            # 49 locked
grep -o 'class="rankings-row"' samples/rankings-oh-boys-xc.html | wc -l                   # 8 leaderboard rows
grep -c 'column-grad-year' samples/roster-oh-mason.html                                  # 319
grep -o 'class="mask' samples/athlete-oh-15005189.html | wc -l                            # 480 mask spans
grep -o 'class="record row "' samples/athlete-oh-15005189.html | wc -l                    # 96 profile result rows
grep -o 'class="season"' samples/athlete-oh-15005189.html | wc -l                         # 12 season blocks
grep -o '<h4>[0-9]\{4\} - [A-Za-z]*</h4>' samples/athlete-oh-15005189.html | wc -l        # 12 season headings
grep -o '<tr class="rankings-row lk"' samples/rank-event-oh-xc-5000m.html | wc -l         # 49 locked rows
grep -o 'class="redact\|class="mask\|team-logo--mask' samples/rank-event-oh-xc-5000m.html | wc -l  # 343 placeholders (7 per locked row x 49)
grep -o 'aria-label="Locked' samples/rank-event-oh-xc-5000m.html | wc -l                  # 0
python3 -c "import re; h=open('samples/results-oh.html').read(); print(len(re.findall(r'<option[^>]*value=\"([^\"]*)\"', re.search(r'id=\"ddYear\"(.*?)</select>', h, re.S).group(1))))"  # 22 options ('' + 2006..2026)
sed -n '/<pre>/,/<\/pre>/p' samples/raw-oh-770621-rs1321880.txt | sed 's/<[^>]*>//g' | grep -cE '^\s+[0-9]+ '  # 80 result rows
python3 -c "import json;print(len(json.load(open('samples/search-v2-athletes-oh-perpage100.json'))['hits']))"  # 100
```

Every number above was re-run against the on-disk bytes at the end of this lane; the `#` value is the observed output.

## Verification protocol (end of lane)

All four deliverables were re-audited against the captured bytes after drafting, by scripting two passes over the capture set:

1. **Counting pass** — every number stated in `SOURCE_REPORT.md`, `schema.json` and `coverage.json` was re-derived with the commands in the block above (or an equivalent extraction) and compared with the claim.
2. **Example pass** — every `example` value in `schema.json` was re-searched in the file it cites: verbatim first, then whitespace-normalized for the multi-line markup excerpts.

**Corrections this pass produced** (claims that were wrong in the first draft; all are fixed in the current files):

| Wrong claim (first draft) | Measured truth | Where it lived |
|---|---|---|
| `ddYear` has 23 options incl. anomalous `2092`/`2094` | `ddYear` has 22 options (empty + 2006–2026); `2092`/`2094` are **`ddLeague`** option values (league IDs) | `SOURCE_REPORT.md` §4, `schema.json` `results_index_row` |
| `ddSeason ∈ {cc, indoor, outdoor}` | also `road` | `SOURCE_REPORT.md` §3 |
| `ddLevel ∈ {ms, hs, college}` | `{youth, ms, hs, college, open, pro}` | `SOURCE_REPORT.md` §3 |
| 5 calendar `data-level` values | 14 distinct values (counts in §3) | `SOURCE_REPORT.md` §3 |
| team types are a per-row attribute `data-team-type` | no such attribute; they are the `#teamType` filter options 1–11, High School selected by default | `SOURCE_REPORT.md` §3 |
| locked rankings rows hold "245 mask spans" | 7 placeholder elements per locked row × 49 = **343** (245 `mask` + 49 `redact` + 49 `team-logo--mask`) | `SOURCE_REPORT.md` §18 |
| meet rows use attributes `meet-row__day="…"`, `meet-row__venue="…"` | they are **elements** (`<span class="meet-row__day">`), inside `<li class="meet-row" data-meet-id=…>` | `SOURCE_REPORT.md` §9 |
| PRO CTA is one searchable sentence | it is split across elements (`To see these rankings, ` + `<span>subscribe to</span>` + `<strong>MileSplit PRO</strong>`), with `/join` and `/login` links around it | `SOURCE_REPORT.md` §18, `schema.json` `rankings_event_row` |
| `www` athlete search first hits are `ca` | they are `ga`(4) `tx`(2) `al`(2) `ma`(2) … | `SOURCE_REPORT.md` §2 |
| "23 year options" for `ddYear` in `schema.json` | 22 options; `ddYear_values_all` now lists them | `schema.json` `results_index_row.count_in_capture` |

Two values are deliberately **not** byte-exact and are labelled as such in place: `pager.href_decoded` (`/results?&page=2`; the bytes carry the HTML-escaped `&amp;`) and the `…`-elided `fragments` in `rankings_event_row.cta_text` / `locked_row.example.placeholders` (schematic, each part independently verified).

Nothing in the deliverables was asserted from memory: claims that could not be measured from these captures are marked `[INFERENCE]` (one: the 10,000-hit search ceiling) or `[inherited]` (prior-phase reports 27/32/33/34, cited by filename).
