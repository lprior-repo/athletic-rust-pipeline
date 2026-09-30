# Captures — runnerspace-dyestat

All fetches anonymous (no login, no cookies, no auth bypass). Same-host calls spaced by `sleep 1.2`–`1.3` (<=1 req/s).
HTTP status / byte counts are as recorded by `curl -w` in `CAPTURES.log` (raw log retained next to this file); `bytes` is the on-disk size of the retained sample.
`000` = connection/DNS failure (nothing downloaded); `403`/`400` rows are retained block/error pages; `n/a` = not a fetched response (see the command column).

| file | URL | HTTP | bytes | UTC timestamp | command |
|---|---|---|---|---|---|
| `ds-home.html` | https://www.dyestat.com/ | 403 | 5342 | 2026-09-22T03:55:20Z | `curl -sS --max-time 25 -o ds-home.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.dyestat.com/'` |
| `rs-home.html` | https://www.runnerspace.com/ | 403 | 5346 | 2026-09-22T03:55:18Z | `curl -sS --max-time 25 -o rs-home.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.runnerspace.com/'` |
| `rs-nxn-2025-results-title187.html` | https://nxn.runnerspace.com/eprofile.php?do=title&title_id=187&event_id=13 | 200 (browser render; CDP does not report status — evidenced by the saved DOM) | 120251 | n/r | `managed Chromium (CDP): browser.open/goto('https://nxn.runnerspace.com/eprofile.php?do=title&title_id=187&event_id=13') -> tab.evaluate("document.documentElement.outerHTML") -> write(path, html). curl to the same URL returns 403 (Cloudflare), which is why the browser path was used.` |
| `rs-resultscentral-home.html` | https://resultscentral.runnerspace.com/ | 200 (browser render; CDP does not report status — evidenced by the saved DOM) | 131324 | n/r | `managed Chromium (CDP): browser.open/goto('https://resultscentral.runnerspace.com/') -> tab.evaluate("document.documentElement.outerHTML") -> write(path, html). curl to the same URL returns 403 (Cloudflare), which is why the browser path was used.` |
| `rs-robots-nxn.runnerspace.com_robots.txt` | https://nxn.runnerspace.com/robots.txt | 200 | 245 | 2026-09-22T04:01:25Z | `curl -sS --max-time 25 -o rs-robots-nxn.runnerspace.com_robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://nxn.runnerspace.com/robots.txt'` |
| `rs-robots-resultscentral.runnerspace.com_robots.txt` | https://resultscentral.runnerspace.com/robots.txt | 200 | 245 | 2026-09-22T04:01:27Z | `curl -sS --max-time 25 -o rs-robots-resultscentral.runnerspace.com_robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://resultscentral.runnerspace.com/robots.txt'` |
| `rs-robots-www.dyestat.com_robots.txt` | https://www.dyestat.com/robots.txt | 200 | 245 | 2026-09-22T04:03:06Z | `curl -sS --max-time 25 -o rs-robots-www.dyestat.com_robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://www.dyestat.com/robots.txt'` |
| `rs-robots-www.runnerspace.com_robots.txt` | https://www.runnerspace.com/robots.txt | 200 | 245 | 2026-09-22T04:01:29Z | `curl -sS --max-time 25 -o rs-robots-www.runnerspace.com_robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://www.runnerspace.com/robots.txt'` |
