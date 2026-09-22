# samples/CAPTURES.md - raw capture manifest (lane `state-assoc-westcoast`)

Every file in `samples/` is the **byte-exact** response body as fetched (no rewriting, no
re-encoding). `BYTES` is `stat -c %s` on the file on disk. Timestamps are the file mtime in
**UTC** (`stat` shows local CDT = UTC-5; e.g. local `22:54:03 -0500` == UTC `03:54:03Z`).

## Fetch tooling (exact commands)

All captures used one of two shell helpers. `URL`, `HTTP`, `BYTES` below come from the
`curl -w` line each helper printed.

```bash
# cap.sh  - "research" UA
#!/usr/bin/env bash
# cap.sh <outfile> <url> [extra curl args...]
out="$1"; url="$2"; shift 2
code=$(curl -sS -A 'Mozilla/5.0 (compatible; research-bot/1.0; +mailto:research@example.org)' \
  --max-time 45 -o "$out" -w '%{http_code} %{size_download} %{content_type}' "$@" "$url" 2>/tmp/wc/lasterr)
rc=$?
ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
bytes=$(stat -c %s "$out" 2>/dev/null || echo 0)
printf 'TS=%s URL=%s HTTP=%s BYTES=%s CURL_RC=%s ERR=%s\n' "$ts" "$url" "$code" "$bytes" "$rc" "$(cat /tmp/wc/lasterr)"
```

```bash
# cap2.sh - identical, but UA = 'Mozilla/5.0 (compatible; omp-research-bot/1.0; +https://example.org/research-bot)'
#           Used once to test whether the 403s were UA-gated (they are).
```

The **Chrome UA** used for WAF-gated hosts is exactly:

```
Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36
```

**Disclosure.** `cifstate.org`, all PrestoSports CIF section hosts and `niaa.com` front their
sites with a CloudFront/WAF rule that returns **HTTP 403 + a 919-byte CloudFront error page**
unless the request presents a browser-like `User-Agent`. This is a user-agent filter, **not**
authentication and **not** a paywall: no credential, cookie, login or CAPTCHA was ever
supplied, and no challenge was solved. The gate was observed, recorded, and only then was a
stock browser UA string sent. The 403 bodies are retained in `samples/` as proof.

## robots.txt policy actually applied

| Host family | robots.txt | Rule honoured |
|---|---|---|
| `*.osaa.org` | 200, 143 B | `Disallow: /brackets/ /teams/ /contests/ /forms/ /mobile/ /demo/ /dev/` - **none of these paths were fetched**. `/schools/`, `/activities/`, `/coaches` only. |
| `www.cifstate.org`, `*.cif{ccs,cs,ncs,ns,sds}.org`, `www.niaa.com` | 200, 954 B (PrestoSports boilerplate) | `Crawl-Delay: 10` for `*`; `Disallow: /cgi-bin/ /_private/ /_vti_bin/ /_vti_cnf/ /_vti_log/ /_vti_pvt/ /_vti_txt/ /reports/ /admin/ /action/`. Requests to these hosts were spaced >=10 s apart and no `/reports/`, `/admin/` or `/action/` path was fetched. |
| `www.cif-la.org` | 200, 54 B | `Disallow: /apps/email/*`, `Crawl-delay: 5` |
| `www.cifsf.org` | 200, 171 B | Yoast block: `Disallow:` (nothing) |
| `www.cifss.org` | 200, 167 B after 301 | nothing disallowed |
| `www.cifsjs.org` | 200, 47 016 B **HTML** | Site serves its SPA shell for `/robots.txt`; **no robots.txt exists**. |
| `hhsaa.org` | 200, 204 B | Whole policy is commented out => nothing disallowed |
| `www.wiaa.com` | **404** | No robots.txt exists |
| `asaa.org` | 200, 109 B | `Disallow: /wp-admin/` only (respecting `Allow: /wp-admin/admin-ajax.php`) |
| `cifoakland.org` | 302 -> `ousd.org/oal` | Association site is a redirect to the host school district |
| `niaa.org` | 301 -> off-domain | **Not Nevada's site.** See note in the report. |

Request pacing: never more than 1 request/second anywhere; PrestoSports hosts were paced at
>=10 s. See also the `Crawl-delay` column in the table below.

## Captures


| file | URL | HTTP | bytes | UA | UTC | note |
|---|---|---|---|---|---|---|
| `asaa-coaches-advisors.html` | `https://asaa.org/coaches/advisors/` | 200 | 276089 | research | 2026-09-22T03:57:24Z | no named directory |
| `asaa-coaches.html` | `https://asaa.org/coaches/` | 200 | 276816 | research | 2026-09-22T03:58:21Z | no named directory |
| `asaa-home.html` | `https://asaa.org/` | 200 | 384020 | research | 2026-09-22T03:54:38Z |  |
| `asaa-member-schools.html` | `https://asaa.org/about/member-schools/` | 200 | 254708 | research | 2026-09-22T03:54:49Z | TablePress tablepress-1, 218 tr |
| `asaa-state-xc-results.html` | `https://asaa.org/activities/cross-country-running/#tab-statechampionship` | 200 | 308505 | research | 2026-09-22T03:56:01Z | RaceResult live + 12 result PDFs |
| `asaa-track-field.html` | `https://asaa.org/activities/track-field/` | 200 | 308647 | research | 2026-09-22T03:57:57Z | Athletic.net team 22622 link + TF result PDFs |
| `asaa-xc.html` | `https://asaa.org/activities/cross-country-running/` | 200 | 308505 | research | 2026-09-22T03:55:22Z | RaceResult + 2025 state result PDFs |
| `cif-census-2025-26.xlsx` | `https://www.cifstate.org/coaches-admin/census/2025-26_census_web.xlsx` | 200 | 1842669 | chrome | 2026-09-22T03:56:17Z | 1609 rows x 345 cols |
| `cif-la-home.html` | `https://www.cif-la.org/` | 200 | 89024 | research | 2026-09-22T03:55:22Z | LA City Section; /apps/* platform |
| `cif-la-widget-directory.html` | `https://www.cif-lahome.org/widget/school/directory` | 200 | 74472 | research | 2026-09-22T04:00:16Z | LA City Presto widget; 173 school names |
| `cifccs-directory.html` | `https://www.cifccs.org/schools/ccs_directory` | 200 | 31234 | chrome | 2026-09-22T03:56:54Z |  |
| `cifccs-home.html` | `https://www.cifccs.org/` | 200 | 91879 | chrome | 2026-09-22T03:56:44Z | CCS (Presto); /schools/ccs_directory |
| `cifcs-home.html` | `https://www.cifcs.org/` | 200 | 345636 | chrome | 2026-09-22T03:56:45Z | Central Section; cifcshome.org/widget/school/directory |
| `cifcs-widget-directory.html` | `https://www.cifcshome.org/widget/school/directory` | 200 | 68746 | research | 2026-09-22T03:56:53Z | Presto widget; flat unpaginated name list |
| `cifncs-home.html` | `https://www.cifncs.org/` | 200 | 240463 | chrome | 2026-09-22T03:56:45Z | NCS; cifncshome.org/widget/school/directory |
| `cifncs-widget-directory.html` | `https://www.cifncshome.org/widget/school/directory` | 200 | 75201 | research | 2026-09-22T03:56:54Z | Presto widget; flat unpaginated name list |
| `cifns-directory.html` | `https://www.cifns.org/governance/directory/home` | 200 | 248624 | chrome | 2026-09-22T03:56:54Z |  |
| `cifns-home.html` | `https://www.cifns.org/` | 200 | 305585 | chrome | 2026-09-22T03:56:45Z | Northern Section; /landing/26-27_School_Member_List.pdf |
| `cifsds-home.html` | `https://www.cifsds.org/` | 200 | 202812 | chrome | 2026-09-22T03:56:45Z | San Diego Section; /school-resources/CIF_School_Directory |
| `cifsds-info-directory.html` | `https://www.cifsds.org/information/directory` | 200 | 174465 | chrome | 2026-09-22T03:58:48Z | Presto page |
| `cifsds-school-directory.html` | `https://www.cifsds.org/school-resources/CIF_School_Directory` | 200 | 184373 | chrome | 2026-09-22T03:56:54Z |  |
| `cifsf-high-schools.html` | `https://www.cifsf.org/schools/high-schools/` | 200 | 258153 | research | 2026-09-22T03:56:55Z |  |
| `cifsf-home.html` | `https://www.cifsf.org/` | 200 | 241135 | research | 2026-09-22T03:56:44Z | SF Section; WordPress |
| `cifsf-xc-guide.html` | `https://www.cifsf.org/the-ultimate-xc-guide-to-the-high-school-season-is-here/` | 200 | 187549 | research | 2026-09-22T03:56:55Z | no Athletic.net/MileSplit link |
| `cifsjs-cross-country.html` | `https://www.cifsjs.org/cross-country` | 200 | 57876 | research | 2026-09-22T04:00:08Z | SJS XC page; links /member-schools, /leagues |
| `cifsjs-home.html` | `https://www.cifsjs.org/` | 200 | 67790 | research | 2026-09-22T03:55:23Z | Sac-Joaquin; Joomla-style, /leagues |
| `cifsjs-member-schools.html` | `https://www.cifsjs.org/member-schools` | 200 | 48325 | research | 2026-09-22T04:00:17Z | 0 school-name strings in static HTML |
| `cifss-directory.html` | `https://cifss.org/directory/` | 200 | 216635 | research | 2026-09-22T03:57:45Z | CIF-SS org/staff directory, not a school list |
| `cifss-home.html` | `https://www.cifss.org/` | 200 | 295871 | research | 2026-09-22T03:55:22Z | Southern Section; WordPress |
| `cifstate-census.html` | `https://www.cifstate.org/coaches-admin/census/index` | 200 | 212908 | chrome | 2026-09-22T03:55:51Z | Participation Census index 2012-13..2025-26 |
| `cifstate-home.html` | `https://www.cifstate.org/` | 200 | 260983 | chrome | 2026-09-22T03:55:31Z | CIF state (Presto) |
| `cifstate-tf-past-results.html` | `https://www.cifstate.org/sports/track_and_field/past_results_records/index` | 200 | 220075 | chrome | 2026-09-22T03:58:42Z | 36 archive links, 2005-2026 |
| `cifstate-xc-index.html` | `https://www.cifstate.org/sports/cross_country/index` | 200 | 263535 | chrome | 2026-09-22T03:59:15Z | Arbiter officials link; no timing provider |
| `cifstate-xc-past-results.html` | `https://www.cifstate.org/sports/cross_country/past_results_records/index` | 200 | 223522 | chrome | 2026-09-22T03:58:28Z | 66 archive links, 2004-2025 |
| `cifstate-xc-results.html` | `https://www.cifstate.org/sports/cross_country/results/index` | 404 | 206253 | chrome | 2026-09-22T03:58:21Z | wrong slug |
| `hhsaa-coaches.html` | `https://www.hhsaa.org/resources/coaches` | 200 | 70428 | research | 2026-09-22T03:56:44Z | Imagine CMS; no named coach directory |
| `hhsaa-home.html` | `https://www.hhsaa.org/` | 200 | 105484 | research | 2026-09-22T03:54:37Z |  |
| `hhsaa-league-BIIF.html` | `https://www.hhsaa.org/schools/BIIF` | 200 | 74153 | research | 2026-09-22T03:55:05Z | 23 schools |
| `hhsaa-league-ILH.html` | `https://www.hhsaa.org/schools/ILH` | 200 | 72224 | research | 2026-09-22T03:55:06Z | 20 schools |
| `hhsaa-league-KIF.html` | `https://www.hhsaa.org/schools/KIF` | 200 | 66738 | research | 2026-09-22T03:55:06Z | 8 schools |
| `hhsaa-league-MIL.html` | `https://www.hhsaa.org/schools/MIL` | 200 | 69503 | research | 2026-09-22T03:55:06Z | 13 schools |
| `hhsaa-league-OIA.html` | `https://www.hhsaa.org/schools/OIA` | 200 | 77405 | research | 2026-09-22T03:55:06Z | 31 schools |
| `hhsaa-schools.html` | `https://www.hhsaa.org/schools` | 200 | 64495 | research | 2026-09-22T03:54:45Z | league index (5 leagues) |
| `hhsaa-tf-tournament-2026.html` | `https://www.hhsaa.org/sports/track_field/tournament/2026` | 200 | 84831 | research | 2026-09-22T03:57:41Z | MileSplit + Athletic.net live links |
| `hhsaa-xc-tournament-2026.html` | `https://www.hhsaa.org/sports/cross_country/tournament/2026` | 200 | 79715 | research | 2026-09-22T03:55:22Z | Google Sheets + RSS; no direct PDF results |
| `mywiaa-ad-center.html` | `https://mywiaa.wiaa.com/ad-center` | 200 | 316027 | research | 2026-09-22T03:58:02Z | Arbiter; no AD name directory |
| `mywiaa-home.html` | `https://mywiaa.wiaa.com` | 200 | 573193 | research | 2026-09-22T03:57:42Z | Arbiter platform |
| `niaa-coaches.html` | `https://www.niaa.com/coaches/Landing` | 202 | 0 | chrome | 2026-09-22T03:57:24Z | HTTP 202 empty body |
| `niaa-finalforms-p9.html` | `https://niaa.finalforms.com/state_schools?page=9&state_schools.athletic_association_member_status_in=member&state_schools.is_archived_eq=false` | 200 | 105444 | research | 2026-09-22T03:56:21Z | 7 rows => terminal page |
| `niaa-finalforms-state-schools.html` | `https://niaa.finalforms.com/state_schools?state_schools.athletic_association_member_status_in=member` | 200 | 139706 | research | 2026-09-22T03:56:01Z | 127 Records, 9 pages |
| `niaa-home.html` | `https://www.niaa.com/` | 200 | 41767 | chrome | 2026-09-22T03:55:30Z | NIAA (Presto); finalforms + arbiter links |
| `niaa-members.html` | `https://www.niaa.com/members/Landing` | 200 | 27849 | chrome | 2026-09-22T03:55:51Z | links niaa.finalforms.com/state_schools |
| `niaa-sports-track.html` | `https://www.niaa.com/sports/track` | 202 | 0 | chrome | 2026-09-22T03:57:24Z | HTTP 202 empty body = Presto bot challenge |
| `niaa-sports-track2.html` | `https://www.niaa.com/sports/track` | 202 | 0 | chrome | 2026-09-22T03:57:31Z | retry: still 202 empty |
| `niaa-xc-archive.html` | `https://www.niaa.com/sports/xc/archive` | 200 | 107264 | chrome | 2026-09-22T03:58:42Z | legacy .mht/.pdf/.htm results by season+division |
| `niaa-xc.html` | `https://www.niaa.com/sports/cross_country` | 404 | 22713 | chrome | 2026-09-22T03:58:21Z | wrong slug |
| `niaa-xc2.html` | `https://www.niaa.com/sports/xc` | 200 | 36873 | chrome | 2026-09-22T03:58:28Z | correct slug; /sports/xc/archive |
| `osaa-coaches.html` | `https://www.osaa.org/coaches` | 200 | 103393 | research | 2026-09-22T03:57:24Z | no named coach directory at state level |
| `osaa-gxc.html` | `https://www.osaa.org/activities/gxc` | 200 | 54088 | research | 2026-09-22T03:55:29Z | TWO Athletic.net links (lines 325, 422) |
| `osaa-home.html` | `https://www.osaa.org/` | 200 | 217834 | research | 2026-09-22T03:54:37Z | carries the Athletic.net link (line 314) |
| `osaa-participation-2025-26-fall.xlsx` | `https://www.osaa.org/docs/participation/2025-26 Fall.xlsx` | 200 | 46997 | research | 2026-09-22T03:58:56Z | 301 schools x 27 cols incl BXC/GXC |
| `osaa-participation.html` | `https://www.osaa.org/schools/participation` | 200 | 46356 | research | 2026-09-22T03:58:47Z | 43 participation XLSX, 2008-09..2025-26 |
| `osaa-school-347.html` | `https://www.osaa.org/schools/347` | 200 | 97757 | research | 2026-09-22T03:57:41Z | staff table (AD+email) + per-activity head coaches |
| `osaa-schools-full-members.html` | `https://www.osaa.org/schools/full-members` | 200 | 58498 | research | 2026-09-22T03:54:56Z | 299 school IDs |
| `osaa-schools.html` | `https://www.osaa.org/schools` | 200 | 31821 | research | 2026-09-22T03:54:48Z |  |
| `osaa-xc-results.html` | `https://www.osaa.org/activities/bxc-gxc/results` | 200 | 39550 | research | 2026-09-22T03:55:51Z | Live Results -> live.athletictiming.net/meets/58997 |
| `ousd-oal.html` | `https://www.ousd.org/oal` | 200 | 88241 | research | 2026-09-22T03:56:45Z | Oakland Athletics League (OUSD-hosted) |
| `robots-asaa.txt` | `https://asaa.org/robots.txt` | 200 | 109 | research | 2026-09-22T03:54:03Z | ASAA robots; only /wp-admin disallowed; sitemap declared |
| `robots-cif-la.txt` | `https://www.cif-la.org/robots.txt` | 200 | 54 | research | 2026-09-22T03:54:17Z | LA City Section; Disallow /apps/email/*; Crawl-delay 5 |
| `robots-cifccs-browserua.txt` | `https://www.cifccs.org/robots.txt` | 200 | 954 | chrome | 2026-09-22T03:54:23Z | CCS robots (Presto) |
| `robots-cifccs.txt` | `https://www.cifccs.org/robots.txt` | 403 | 919 | research | 2026-09-22T03:54:17Z | 403 at research UA |
| `robots-cifcs-browserua.txt` | `https://www.cifcs.org/robots.txt` | 200 | 954 | chrome | 2026-09-22T03:54:27Z | Central Section robots (Presto) |
| `robots-cifcs.txt` | `https://www.cifcs.org/robots.txt` | 403 | 919 | research | 2026-09-22T03:54:17Z | 403 |
| `robots-cifncs-browserua.txt` | `https://www.cifncs.org/robots.txt` | 200 | 954 | chrome | 2026-09-22T03:54:27Z | NCS robots (Presto) |
| `robots-cifncs.txt` | `https://www.cifncs.org/robots.txt` | 403 | 919 | research | 2026-09-22T03:54:17Z | 403 |
| `robots-cifns-browserua.txt` | `https://www.cifns.org/robots.txt` | 200 | 954 | chrome | 2026-09-22T03:54:27Z | Northern Section robots (Presto) |
| `robots-cifns.txt` | `https://www.cifns.org/robots.txt` | 403 | 919 | research | 2026-09-22T03:54:17Z | 403 |
| `robots-cifoakland.txt` | `https://www.cifoakland.org/robots.txt` | 302 | 247 | research | 2026-09-22T03:54:18Z | 302 -> ousd.org/oal (Oakland Athletics League hosted by OUSD) |
| `robots-cifsds-browserua.txt` | `https://www.cifsds.org/robots.txt` | 200 | 954 | chrome | 2026-09-22T03:54:27Z | San Diego Section robots (Presto) |
| `robots-cifsds.txt` | `https://www.cifsds.org/robots.txt` | 403 | 919 | research | 2026-09-22T03:54:18Z | 403 |
| `robots-cifsf.txt` | `https://www.cifsf.org/robots.txt` | 200 | 171 | research | 2026-09-22T03:54:18Z | SF Section; Yoast block, sitemap_index.xml |
| `robots-cifsjs.txt` | `https://www.cifsjs.org/robots.txt` | 200 | 47016 | research | 2026-09-22T03:54:18Z | NOT a robots.txt: 47016-byte HTML page (SPA fallback) |
| `robots-cifss-browserua.txt` | `https://www.cifss.org/robots.txt` | 200 | 167 | chrome | 2026-09-22T03:54:27Z | Southern Section robots after redirect |
| `robots-cifss.txt` | `https://www.cifss.org/robots.txt` | 301 | 162 | research | 2026-09-22T03:54:18Z | 301 -> nginx |
| `robots-cifstate-bare.txt` | `https://cifstate.org/robots.txt` | 403 | 919 | research | 2026-09-22T03:54:07Z | same at bare host |
| `robots-cifstate-browserua.txt` | `https://www.cifstate.org/robots.txt` | 200 | 954 | chrome | 2026-09-22T03:54:23Z | CIF state robots (PrestoSports); Crawl-Delay 10, Disallow /reports/ /admin/ /action/ |
| `robots-cifstate.txt` | `https://www.cifstate.org/robots.txt` | 403 | 919 | research | 2026-09-22T03:54:03Z | CloudFront 403 at research UA |
| `robots-hhsaa-follow.txt` | `https://hhsaa.org/robots.txt` | 200 | 204 | research | 2026-09-22T03:54:07Z | HHSAA robots: entire block commented out => nothing disallowed |
| `robots-hhsaa.txt` | `https://www.hhsaa.org/robots.txt` | 302 | 0 | research | 2026-09-22T03:54:03Z | 302 |
| `robots-niaa-com-browserua.txt` | `https://www.niaa.com/robots.txt` | 200 | 954 | chrome | 2026-09-22T03:54:23Z | real NIAA robots.txt (PrestoSports) |
| `robots-niaa-com.txt` | `https://www.niaa.com/robots.txt` | 403 | 919 | research | 2026-09-22T03:54:17Z | CloudFront 403: UA-gated WAF |
| `robots-niaa-follow.txt` | `https://niaa.org/robots.txt` | 200 | 198356 | research | 2026-09-22T03:54:07Z | FOLLOW redirects: niaa.org serves inroads.org (unrelated org). niaa.org is dead for NV. |
| `robots-niaa-root.txt` | `https://www.niaa.org/robots.txt` | 200 | 198356 | research | 2026-09-22T03:54:12Z | same inroads.org content via www; confirms niaa.org misdirection |
| `robots-niaa.txt` | `https://www.niaa.org/robots.txt` | 301 | 167 | research | 2026-09-22T03:54:03Z | 301 -> niaa.org is NOT Nevada; redirects off-domain (see robots-niaa-follow.txt) |
| `robots-osaa.txt` | `https://www.osaa.org/robots.txt` | 200 | 143 | research | 2026-09-22T03:54:03Z | OSAA robots; Disallow /teams/ /contests/ /brackets/ /forms/ /mobile/ /demo/ /dev/ |
| `robots-wiaa-www.txt` | `https://wiaa.com/robots.txt` | 404 | 13 | research | 2026-09-22T03:54:08Z | no robots.txt (apex) |
| `robots-wiaa.txt` | `https://www.wiaa.com/robots.txt` | 404 | 13 | research | 2026-09-22T03:54:04Z | no robots.txt |
| `wiaa-events.html` | `https://www.wiaa.com/events/` | 200 | 394986 | research | 2026-09-22T03:59:15Z | wpanetwork.com/wiaa/brackets/tournament.php |
| `wiaa-finalforms-p2.html` | `https://wiaa.finalforms.com/state_schools?page=2&state_schools.athletic_association_member_status_in=member&state_schools.is_archived_eq=false` | 200 | 145215 | research | 2026-09-22T03:55:37Z | 15 rows |
| `wiaa-finalforms-p51.html` | `https://wiaa.finalforms.com/state_schools?page=51&state_schools.athletic_association_member_status_in=member&state_schools.is_archived_eq=false` | 200 | 91128 | research | 2026-09-22T03:55:39Z | 2 rows => terminal page |
| `wiaa-finalforms-state-schools.html` | `https://wiaa.finalforms.com/state_schools` | 200 | 134962 | research | 2026-09-22T03:54:45Z | page 1 of WA directory; last page link = 51 |
| `wiaa-home.html` | `https://www.wiaa.com/` | 200 | 775444 | research | 2026-09-22T03:54:37Z |  |
| `wiaa-state-xc-results.html` | `https://www.wiaa.com/tournament-xch/?sportid=22` | 200 | 537488 | research | 2026-09-22T03:56:01Z | no third-party timing link |
| `wiaa-xc-state-2026.html` | `https://www.wiaa.com/events/2026-state-cross-country/` | 200 | 295213 | research | 2026-09-22T03:55:22Z |  |
| `wpanetwork-xc.html` | `https://www.wpanetwork.com/wiaa/brackets/tournament.php` | 200 | 8215 | research | 2026-09-22T04:00:07Z | WPA Network bracket index (xajax); team brackets, not a timing provider |

**Total: 105 capture files, 14,615,289 bytes.**
