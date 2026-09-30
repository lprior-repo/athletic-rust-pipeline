# Captures — other-databases

All fetches anonymous (no login, no cookies, no auth bypass). Same-host calls spaced by `sleep 1.2`–`1.3` (<=1 req/s).
HTTP status / byte counts are as recorded by `curl -w` in `CAPTURES.log` (raw log retained next to this file); `bytes` is the on-disk size of the retained sample.
`000` = connection/DNS failure (nothing downloaded); `403`/`400` rows are retained block/error pages; `n/a` = not a fetched response (see the command column).

| file | URL | HTTP | bytes | UTC timestamp | command |
|---|---|---|---|---|---|
| `ccr-home.html` | https://www.crosscountryratings.com/ | 200 | 78033 | 2026-09-22T04:02:09Z | `curl -sS --max-time 25 -o ccr-home.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.crosscountryratings.com/'` |
| `ccr-individuals.html` | https://www.crosscountryratings.com/individuals | 200 | 503546 | 2026-09-22T04:02:10Z | `curl -sS --max-time 25 -o ccr-individuals.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.crosscountryratings.com/individuals'` |
| `ccr-results.html` | https://www.crosscountryratings.com/results | 200 | 137723 | 2026-09-22T04:02:46Z | `curl -sS --max-time 25 -o ccr-results.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.crosscountryratings.com/results'` |
| `ccr-runner.html` | https://www.crosscountryratings.com/runner/11669 | 200 | 94038 | 2026-09-22T04:02:35Z | `curl -sS --max-time 25 -o ccr-runner.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.crosscountryratings.com/runner/11669'` |
| `other-www.baumspage.com_robots.txt` | https://www.baumspage.com/robots.txt | 404 | 1510 | 2026-09-22T04:01:44Z | `curl -sS --max-time 25 -o other-www.baumspage.com_robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://www.baumspage.com/robots.txt'` |
| `other-www.crosscountryratings.com_robots.txt` | https://www.crosscountryratings.com/robots.txt | 200 | 1374 | 2026-09-22T04:01:46Z | `curl -sS --max-time 25 -o other-www.crosscountryratings.com_robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://www.crosscountryratings.com/robots.txt'` |
| `other-www.fieldlevel.com_robots.txt` | https://www.fieldlevel.com/robots.txt | 200 | 110 | 2026-09-22T04:01:43Z | `curl -sS --max-time 25 -o other-www.fieldlevel.com_robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://www.fieldlevel.com/robots.txt'` |
| `other-www.ncsasports.org_robots.txt` | https://www.ncsasports.org/robots.txt | 200 | 2673 | 2026-09-22T04:01:41Z | `curl -sS --max-time 25 -o other-www.ncsasports.org_robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://www.ncsasports.org/robots.txt'` |
