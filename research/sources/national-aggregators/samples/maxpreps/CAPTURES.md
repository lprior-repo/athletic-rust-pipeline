# Captures — maxpreps

All fetches anonymous (no login, no cookies, no auth bypass). Same-host calls spaced by `sleep 1.2`–`1.3` (<=1 req/s).
HTTP status / byte counts are as recorded by `curl -w` in `CAPTURES.log` (raw log retained next to this file); `bytes` is the on-disk size of the retained sample.
`000` = connection/DNS failure (nothing downloaded); `403`/`400` rows are retained block/error pages; `n/a` = not a fetched response (see the command column).

| file | URL | HTTP | bytes | UTC timestamp | command |
|---|---|---|---|---|---|
| `mp-aledo-roster.html` | https://www.maxpreps.com/tx/aledo/aledo-bearcats/track-field/roster/ | 200 | 172139 | 2026-09-22T03:59:50Z | `curl -sS --max-time 25 -o mp-aledo-roster.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/tx/aledo/aledo-bearcats/track-field/roster/'` |
| `mp-aledo-staff.html` | https://www.maxpreps.com/tx/aledo/aledo-bearcats/track-field/staff/ | 200 | 168932 | 2026-09-22T04:01:01Z | `curl -sS --max-time 25 -o mp-aledo-staff.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/tx/aledo/aledo-bearcats/track-field/staff/'` |
| `mp-athlete-gage-baker.html` | https://www.maxpreps.com/az/surprise/paradise-honors-panthers/athletes/gage-baker/?careerid=9qdc8bm62h599 | 200 | 565183 | 2026-09-22T04:00:07Z | `curl -sS --max-time 25 -o mp-athlete-gage-baker.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/az/surprise/paradise-honors-panthers/athletes/gage-baker/?careerid=9qdc8bm62h599'` |
| `mp-athletes-index-p2.html` | https://www.maxpreps.com/track-field/athletes/?page=2 | 200 | 464862 | 2026-09-22T04:00:45Z | `curl -sS --max-time 25 -o mp-athletes-index-p2.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/track-field/athletes/?page=2'` |
| `mp-athletes-index.html` | https://www.maxpreps.com/track-field/athletes/ | 200 | 464825 | 2026-09-22T04:00:05Z | `curl -sS --max-time 25 -o mp-athletes-index.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/track-field/athletes/'` |
| `mp-home.html` | https://www.maxpreps.com/ | 200 | 254583 | 2026-09-22T03:55:18Z | `curl -sS --max-time 25 -o mp-home.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/'` |
| `mp-mobile-team-aliedo.html` | (redirect | 200 | 189887 | 2026-09-22T03:59:36Z | `curl -sS --max-time 25 -o mp-mobile-team-aliedo.html -w '%{http_code} %{size_download} %{content_type}' '(redirect'` |
| `mp-oh-track-athletes.html` | https://www.maxpreps.com/oh/track-field/athletes/ | 200 | 273489 | 2026-09-22T04:00:59Z | `curl -sS --max-time 25 -o mp-oh-track-athletes.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/oh/track-field/athletes/'` |
| `mp-robots.txt` | https://www.maxpreps.com/robots.txt | 200 | 5094 | 2026-09-22T04:09:25Z | `curl -sS --max-time 25 -o mp-robots.txt -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/robots.txt'` |
| `mp-state-tx-track.html` | https://www.maxpreps.com/tx/track-field/ | 200 | 218018 | 2026-09-22T03:56:25Z | `curl -sS --max-time 25 -o mp-state-tx-track.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/tx/track-field/'` |
| `mp-track-field.html` | https://www.maxpreps.com/track-field/ | 200 | 219624 | 2026-09-22T03:56:10Z | `curl -sS --max-time 25 -o mp-track-field.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/track-field/'` |
| `mp-tx-track-schools.html` | https://www.maxpreps.com/tx/track-field/schools/ | 200 | 359208 | 2026-09-22T03:57:59Z | `curl -sS --max-time 25 -o mp-tx-track-schools.html -w '%{http_code} %{size_download} %{content_type}' 'https://www.maxpreps.com/tx/track-field/schools/'` |
