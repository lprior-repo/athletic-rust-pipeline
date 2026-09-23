# Athletic.net lane - sample captures

All live requests in this directory were made from this workstation on **2026-09-21/22 (UTC)** at
**>=1.2 s spacing (<=1 req/s/host)**, with a desktop-Chrome user agent and no cookie reuse across
samples. Tokens and cookies are redacted in place; never persisted.

The `python3 <script>.py` commands in the tables are **provenance, not a reproduction recipe**: this
lane's Python probes (`build-schema.py`, `checks-enum-crosscheck.py`, `extract-har-samples.py`,
`gen-captures.py`, `probe-anon-meet.py`, `redact-samples.py`) were deleted from the tree by the
contract commit `ea81c56` ("Land the workspace contract and delete Python from the repository"). The
byte-exact captures and the derived `.tsv`/`.txt` files they produced remain and are the evidence.

Two provenance classes: **live** (this lane's own requests) and **copied** (retained corpus captures,
re-used instead of re-fetched; the original path is given per file).

The **Bytes** column is the on-disk size (tokens already redacted); the un-redacted HTTP body is
larger by the redaction delta, which the (since removed) `probe-anon-meet.py` printed as `bytes` in
`anon-meet-probe-report.json` (e.g. GetMeetData: 16,644 B over the wire vs 16,403 B on disk, delta =
the 255-char `jwtMeet` replaced by a fixed placeholder). Where a byte count is quoted in
`../SOURCE_REPORT.md`, the wire value is used and the sample it came from is named.

## Live captures

| File | URL | HTTP | Bytes | Captured (UTC) | sha256[:16] | Command |
|---|---|---|---|---|---|---|
| `anon-allresults-634313.headers` | `https://www.athletic.net/api/v1/Meet/GetAllResultsData?meetId=634313&sport=tf&rawResults=false&showTips=false` | 200 | 349 | 2026-09-22 04:12:01Z | `3575aa0bd4cc2359` | `python3 probe-anon-meet.py` |
| `anon-allresults-634313.json` | `https://www.athletic.net/api/v1/Meet/GetAllResultsData?meetId=634313&sport=tf&rawResults=false&showTips=false` | 200 | 506882 | 2026-09-22 04:12:01Z | `5ef415aa1b6c9f6e` | `python3 probe-anon-meet.py` |
| `anon-eventdiv-634313.headers` | `https://www.athletic.net/api/v1/Meet/GetEventDivisionData?meetId=634313&sport=tf` | 200 | 326 | 2026-09-22 04:12:00Z | `5dd00b4632da5924` | `python3 probe-anon-meet.py` |
| `anon-eventdiv-634313.json` | `https://www.athletic.net/api/v1/Meet/GetEventDivisionData?meetId=634313&sport=tf` | 200 | 8341 | 2026-09-22 04:12:00Z | `fdf607f62a444fce` | `python3 probe-anon-meet.py   (anettokens = jwtMeet from GetMeetData)` |
| `anon-meet-probe-report.json` | `(derived)` | - | 1611 | 2026-09-22 04:12:01Z | `ae7264ed7ec53f8f` | `python3 probe-anon-meet.py` |
| `anon-meetdata-634313.headers` | `https://www.athletic.net/api/v1/Meet/GetMeetData?meetId=634313&sport=tf` | 200 | 326 | 2026-09-22 04:11:58Z | `ba30f14d86ddc0d4` | `python3 probe-anon-meet.py` |
| `anon-meetdata-634313.json` | `https://www.athletic.net/api/v1/Meet/GetMeetData?meetId=634313&sport=tf` | 200 | 16403 | 2026-09-22 04:11:58Z | `838d8eb04bca785b` | `python3 probe-anon-meet.py` |
| `atn-probe-divchildren-168416.json` | `https://www.athletic.net/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=168416` | 200 | 11875 | 2026-09-22 04:01:59Z | `3885f6990d7e232d` | `same shape as above, divId=168416` |
| `atn-probe-divchildren-168546.json` | `https://www.athletic.net/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=168546` | 200 | 4567 | 2026-09-22 04:02:30Z | `0d5b9b6efb92842c` | `same shape as above, divId=168546` |
| `atn-probe-divchildren-170305.json` | `https://www.athletic.net/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170305` | 200 | 51554 | 2026-09-22 04:02:32Z | `0e4f88729f2820d7` | `same shape as above, divId=170305` |
| `atn-probe-divchildren-170770-page2.json` | `https://www.athletic.net/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170770&page=2` | 200 | 22361 | 2026-09-22 04:02:34Z | `d44148702207693b` | `same shape as above, divId=170770&page=2` |
| `atn-probe-divchildren-170770.json` | `https://www.athletic.net/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170770` | 200 | 22361 | 2026-09-22 04:01:57Z | `d44148702207693b` | `curl -sS -A "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36" -H "Accept: application/json, text/plain, */*" -H "Referer: {ref}/TrackAndField/rankings/list/170770" -o <f> "<url>"` |
| `atn-probe-divchildren-170771.json` | `https://www.athletic.net/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170771` | 200 | 1962 | 2026-09-22 04:01:51Z | `dd0d438e34c97ffe` | `same shape as above, divId=170771` |
| `atn-probe-divchildren-170772.json` | `https://www.athletic.net/api/v1/SiteHeader/GetDivChildren?sport=tf&divId=170772` | 200 | 503 | 2026-09-22 04:02:01Z | `a0438335371222c5` | `same shape as above, divId=170772` |
| `atn-probe-events-wisconsin-2026-5-26.headers` | `https://www.athletic.net/events/usa/wisconsin/2026-5-26` | 200 | 290 | 2026-09-22 04:07:10Z | `2d14dbf7d1d4dde7` | `same request` |
| `atn-probe-events-wisconsin-2026-5-26.html` | `https://www.athletic.net/events/usa/wisconsin/2026-5-26` | 200 | 7430 | 2026-09-22 04:02:43Z | `15620fe2a2e8754f` | `curl -sS -A "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36" -H "Accept: text/html,application/xhtml+xml"  -o <f> "<url>"` |
| `atn-probe-getrankings-anon-g11-p2.headers` | `https://www.athletic.net/api/v1/tfRankings/GetRankings` | 200 | 328 | 2026-09-22 04:07:10Z | `1f60d0933900a2e3` | `same request, qParams.grades=[11], page=2` |
| `atn-probe-getrankings-anon-g11-p2.json` | `https://www.athletic.net/api/v1/tfRankings/GetRankings` | 200 | 64614 | 2026-09-22 04:07:10Z | `e7b332d0add8c161` | `same request, qParams.grades=[11], page=2` |
| `atn-probe-getrankings-anon-nopage.json` | `https://www.athletic.net/api/v1/tfRankings/GetRankings` | 200 | 64793 | 2026-09-22 04:07:10Z | `a64e9f37acd42a2e` | `curl -sS -A "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36" -H "Accept: application/json, text/plain, */*"  -H "Content-Type: application/json" --data @<body> -o <f> "<url>"  (body with NO qParams.page key)` |
| `atn-probe-getrankings-anon.headers` | `https://www.athletic.net/api/v1/tfRankings/GetRankings` | 200 | 328 | 2026-09-22 04:07:10Z | `6bcee4a267890592` | `same request as atn-probe-getrankings-anon.json` |
| `atn-probe-getrankings-anon.json` | `https://www.athletic.net/api/v1/tfRankings/GetRankings` | 200 | 64793 | 2026-09-22 04:07:10Z | `a64e9f37acd42a2e` | `curl -sS -A "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36" -H "Accept: application/json, text/plain, */*"  -H "Content-Type: application/json" --data @<body> -o <f> "<url>"  (body: qParams.grades=[], page=1, eventShort=100m)` |
| `atn-probe-statescountries-anon.headers` | `https://www.athletic.net/api/v1/public/GetStatesCountries2` | 200 | 305 | 2026-09-22 04:06:09Z | `9c7a35423948be6e` | `same request` |
| `atn-probe-statescountries-anon.json` | `https://www.athletic.net/api/v1/public/GetStatesCountries2` | 200 | 34159 | 2026-09-22 04:06:09Z | `6b97944e7ff4919e` | `curl -sS -A "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36" -H "Accept: application/json, text/plain, */*" -H "Referer: {ref}/TrackAndField/rankings" -o <f> "<url>"` |
| `atn-robots.headers` | `https://www.athletic.net/robots.txt` | 200 | 317 | 2026-09-22 04:01:49Z | `554582edab48dca5` | `same request as atn-robots.txt` |
| `atn-robots.txt` | `https://www.athletic.net/robots.txt` | 200 | 1080 | 2026-09-22 04:01:49Z | `94733f07d7df6f25` | `bash refetch.sh   (curl -sS -A "<Chrome UA>" -D <f>.headers -o <f> <url>)` |

## HAR extracts (from the two retained sessions, not re-fetched)

| File | URL | HTTP | Bytes | Extracted (UTC) | sha256[:16] | Command |
|---|---|---|---|---|---|---|
| `har1-getagesgrades-wi-2026.json` | `https://www.athletic.net/api/v1/tfRankings/GetAgesGrades` | 200 | 1080 | 2026-09-22 04:01:37Z | `646b20a2ec4724f9` | `python3 extract-har-samples.py` |
| `har1-getbreadcrumbs-wi-170770.json` | `https://www.athletic.net/api/v1/SiteHeader/GetBreadcrumbs?sport=tf&athleteId=0&meetId=0&calendarId=0&divId=170770&teamId=0&seasonId=0` | 200 | 1456 | 2026-09-22 04:01:37Z | `19defb03cfb48b6c` | `python3 extract-har-samples.py` |
| `har1-getconvertevents.json` | `https://www.athletic.net/api/v1/tfRankings/GetConvertEvents` | 200 | 23613 | 2026-09-22 04:01:37Z | `19efd933d47b0622` | `python3 extract-har-samples.py` |
| `har1-getnavinfo-wi-2026-hs-boys.json` | `https://www.athletic.net/api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m&recordSetId=0&locationId=0&teamId=0&indoor=false` | 200 | 78468 | 2026-09-22 04:01:37Z | `7541a611ba8a596f` | `python3 extract-har-samples.py` |
| `har1-getrankings-request-bodies.json` | `(derived) the 8 GetRankings POST bodies captured in HAR #1` | - | 5122 | 2026-09-22 04:01:37Z | `f7b7aa0e7f3381a6` | `python3 extract-har-samples.py` |
| `har1-getrankings-wi-170770-m-100m-g11-p17-n17.json` | `https://www.athletic.net/api/v1/tfRankings/GetRankings` | 200 | 16414 | 2026-09-22 04:07:10Z | `05ddd536bd366578` | `python3 extract-har-samples.py` |
| `har1-getrankings-wi-170770-m-100m-p2-n102.json` | `https://www.athletic.net/api/v1/tfRankings/GetRankings` | 200 | 88818 | 2026-09-22 04:07:10Z | `598af125e8421b7a` | `python3 extract-har-samples.py` |
| `har1-getrankings-wi-170770-m-multievent-n180.json` | `https://www.athletic.net/api/v1/tfRankings/GetRankings` | 200 | 190708 | 2026-09-22 04:07:10Z | `a5b85d747cb5ba86` | `python3 extract-har-samples.py` |
| `har1-getstandards-empty.json` | `https://www.athletic.net/api/v1/tfRankings/GetStandards?recordsetId=null&eventId=null&eventTypeId=null` | 200 | 2 | 2026-09-22 04:01:37Z | `4f53cda18c2baa0c` | `python3 extract-har-samples.py` |
| `har1-getstatescountries2.json` | `https://www.athletic.net/api/v1/public/GetStatesCountries2` | 200 | 34159 | 2026-09-22 04:01:37Z | `6b97944e7ff4919e` | `python3 extract-har-samples.py` |
| `har2-general-getrankings-28872883.json` | `https://www.athletic.net/api/v1/General/GetRankings?athleteId=28872883&sport=tf&seasonId=2026&truncate=true` | 200 | 6122 | 2026-09-22 04:01:37Z | `96b6296b72d7948f` | `python3 extract-har-samples.py` |
| `har2-getathletebiodata-28872883.json` | `https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=28872883&sport=tf&level=0` | 200 | 30344 | 2026-09-22 04:01:37Z | `a9e1c65d77f1e894` | `python3 extract-har-samples.py` |
| `har2-teamnav-3204.json` | `https://www.athletic.net/api/v1/TeamNav/Team?team=3204&sport=tf&season=2026` | 200 | 1222 | 2026-09-22 04:01:37Z | `f5eb3013b2b84c38` | `python3 extract-har-samples.py` |

## Copied from the retained corpus evidence tree

| File | Original path | Bytes | sha256[:16] |
|---|---|---|---|
| `live-allresults-634313.redacted.json` | `~/Downloads/midwest-tfxc-source-research/research/midwest/evidence/gap-closure/atn/allresults-634313.redacted.json` | 565387 | `a209a305df376752` |
| `live-eventdiv-634313.json` | `~/Downloads/midwest-tfxc-source-research/research/midwest/evidence/gap-closure/atn/eventdiv-634313.json` | 8341 | `fdf607f62a444fce` |
| `live-getathletebiodata-28872883-2026-09-20.json` | `~/Downloads/midwest-tfxc-source-research/research/midwest/evidence/gap-closure/atn/athletebio-28872883.json` | 30285 | `670993c5a50a9602` |
| `live-meetdata-634313.redacted.json` | `~/Downloads/midwest-tfxc-source-research/research/midwest/evidence/gap-closure/atn/meetdata-634313.redacted.json` | 18790 | `88a5261ef078b7e4` |
| `live-resultsdata3-100m-wind-615580.json` | `~/Downloads/midwest-tfxc-source-research/research/midwest/evidence/gap-closure/atn/resultsdata3-100m-wind-615580.json` | 49826 | `8be6ec826cabf58d` |
| `live-resultsdata3-100m.json` | `~/Downloads/midwest-tfxc-source-research/research/midwest/evidence/gap-closure/atn/resultsdata3-100m.json` | 22186 | `4c0f4f558513ec53` |
| `live-resultsdata3-4x100m.json` | `~/Downloads/midwest-tfxc-source-research/research/midwest/evidence/gap-closure/atn/resultsdata3-4x100m.json` | 37415 | `6e8d281a4edef2fd` |
| `live-resultsdata3-shot-615580.json` | `~/Downloads/midwest-tfxc-source-research/research/midwest/evidence/gap-closure/atn/resultsdata3-shot-615580.json` | 40953 | `007925c807d44eeb` |

## Scripts and derived files

Rows marked **removed** are scripts the contract commit `ea81c56` deleted from the tree: the row says
what the script did, not what the directory holds. The derived `.tsv`/`.txt` output beside them stays.

| File | Purpose |
|---|---|
| `build-schema.py` **removed** | generated ../schema.json from the sample bytes |
| `checks-enum-crosscheck.py` **removed** | asserted the 51 nav state nodes map to census-domain's UsJurisdiction enum; wrote checks-state-divs.tsv |
| `checks-enum-crosscheck.txt` | console output of checks-enum-crosscheck.py (51/51 enum match, Overseas out of scope) |
| `checks-state-divs.tsv` | output of checks-enum-crosscheck.py (52 rows) |
| `extract-har-samples.py` **removed** | extracted the HTTP Archive entries into har1-*/har2-* files; wrote har-atn-request-log.tsv |
| `gen-captures.py` **removed** | generated this file (CAPTURES.md) from the bytes actually present |
| `har-atn-request-log.tsv` | URL + status + timestamp for every request in the two retained HARs |
| `probe-anon-meet.py` **removed** | the 3-request anonymous whole-meet probe |
| `redact-samples.py` **removed** | redacted jwtMeet/jwtTFTopReport/cookie values from every sample in place |
| `refetch.sh` | the exact curl command for every live sample (verified byte-identical on re-run, see below) |

## Reproduction check (run 2026-09-21 23:11-23:12 UTC)

`bash refetch.sh /tmp/atn-refetch` re-issued every live request; bodies were compared by sha256 against
the stored samples:

* **byte-identical** - `atn-robots.txt`, `atn-probe-statescountries-anon.json`, all six
  `atn-probe-divchildren-*.json`, and the 3 whole-meet bodies (same 16,644 / 8,341 / 506,882 B, same
  758/288/12/45 counts on the third run);
* **differ, explained** - `atn-probe-events-wisconsin-2026-5-26.html` (+361 B: Cloudflare injects a
  `beacon.min.js` tag and a per-response nonce), `atn-probe-getrankings-anon*.json` (+296 B each: the
  server-issued `jwtTFTopReport` is present in the fresh body and was replaced by the redaction
  placeholder in the stored one - the only differing JSON path in either file).

The scratch directory holding the un-redacted re-fetch was deleted immediately (`rm -rf /tmp/atn-refetch`).

## Request ledger

The reproduction check issued exactly **14** live requests (11 from `refetch.sh` + 3 from
`probe-anon-meet.py`). This directory held 58 files when this ledger was written — 52 today, since the
6 `samples/*.py` probes were deleted from the tree (`ea81c56`; see the note at the top). **17 live response bodies + 9 header files**
(`anon-*`, `atn-probe-*`, `atn-robots.*`), **13 HAR extracts + 1 request log**, **8 copies** of the
retained corpus captures, **1 script** (`refetch.sh`), and 3 derived files (`CAPTURES.md`, `checks-state-divs.tsv`,
`checks-enum-crosscheck.txt`). No 429, no `Retry-After`, no CAPTCHA, no login. `robots.txt` was read
before the first request in every session.

## Provenance notes

* The two HARs live outside the repo at `/home/lewis/Downloads/www.athletic.net{,2}.har`
  (722 + 167 entries, captured 2026-09-18 by the main agent); every `har*` file here is an extract.
* `atn-probe-divchildren-170770-page2.json` is byte-identical to `atn-probe-divchildren-170770.json`:
  the `page` parameter is ignored by `GetDivChildren`.
* The `*.headers` files carry the response headers verbatim with `set-cookie` redacted; they are the
  evidence for the rate-limit and Cloudflare claims in `../SOURCE_REPORT.md` §17.
* Nothing in this directory contains a token, cookie, email or personal contact value.
