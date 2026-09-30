# Captures — usatf-aau

All fetches anonymous (no login, no cookies, no auth bypass). Same-host calls spaced by `sleep 1.2`–`1.3` (<=1 req/s).
HTTP status / byte counts are as recorded by `curl -w` in `CAPTURES.log` (raw log retained next to this file); `bytes` is the on-disk size of the retained sample.
`000` = connection/DNS failure (nothing downloaded); `403`/`400` rows are retained block/error pages; `n/a` = not a fetched response (see the command column).

| file | URL | HTTP | bytes | UTC timestamp | command |
|---|---|---|---|---|---|
| `aau-sports-track.html` | https://www.aausports.org/sports/track-field | 403 | 5787 | 2026-09-22T03:58:03Z | `curl -sS --max-time 25 -o aau-sports-track.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.aausports.org/sports/track-field'` |
| `aau-track-hub-followed.html` | — | n/r | 5782 | n/r | `not recorded` |
| `aau-track-hub.html` | https://aautrackandfield.org/ | 301 | 160 | 2026-09-22T03:58:04Z | `curl -sS --max-time 25 -o aau-track-hub.html -w '%{http_code} %{size_download} %{content_type}' 'https://aautrackandfield.org/'` |
| `other-www.aausports.org_robots.txt` | https://www.aausports.org/robots.txt | 404 | 0 | 2026-09-22T04:03:04Z | `curl -sS --max-time 25 -o other-www.aausports.org_robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://www.aausports.org/robots.txt'` |
| `other-www.usatf.org_robots.txt` | https://www.usatf.org/robots.txt | 200 | 96 | 2026-09-22T04:09:29Z | `curl -sS --max-time 25 -o other-www.usatf.org_robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://www.usatf.org/robots.txt'` |
| `usatf-home.html` | https://www.usatf.org/ | 200 | 100828 | 2026-09-22T03:56:10Z | `curl -sS --max-time 25 -o usatf-home.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.usatf.org/'` |
| `usatf-jo-live-finishedresults.html` | https://finishedresults.trackscoreboard.com/meets/14258/events | 200 | 74310 | 2026-09-22T04:01:41Z | `curl -sS --max-time 25 -o usatf-jo-live-finishedresults.html -w '%{http_code} %{size_download} %{content_type}' 'https://finishedresults.trackscoreboard.com/meets/14258/events'` |
| `usatf-jo-tf.html` | https://www.usatf.org/events/2026/2026-usatf-national-junior-olympic-track-field-cha | 200 | 44435 | 2026-09-22T04:01:25Z | `curl -sS --max-time 25 -o usatf-jo-tf.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.usatf.org/events/2026/2026-usatf-national-junior-olympic-track-field-cha'` |
| `usatf-junior-olympic-xc.html` | https://www.usatf.org/events/2026/2026-usatf-national-junior-olympic-cross-country-c | 200 | 43018 | 2026-09-22T03:57:59Z | `curl -sS --max-time 25 -o usatf-junior-olympic-xc.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.usatf.org/events/2026/2026-usatf-national-junior-olympic-cross-country-c'` |
| `usatf-sitemap-www.xml` | https://www.usatf.org/sitemap.xml | 200 | 744058 | 2026-09-22T03:56:25Z | `curl -sS --max-time 25 -o usatf-sitemap-www.xml -w '%{http_code} %{size_download} %{content_type}' 'https://www.usatf.org/sitemap.xml'` |
| `usatf-sitemap.xml` | https://usatf.org/sitemap.xml | 301 | 156 | 2026-09-22T03:56:12Z | `curl -sS --max-time 25 -o usatf-sitemap.xml -w '%{http_code} %{size_download} %{content_type}' 'https://usatf.org/sitemap.xml'` |
