# G1 — the live Athletic.net search response contract

Settles the endgame question in `research/ENDGAME-GAPS.md` (G1): the pipeline's own envelope
rejects every live search page, so **no live row has ever been accepted**. This document records
(a) one fresh authorized capture attempt against the live endpoint, which came back as a Cloudflare
managed challenge, and (b) the counted inventory of the live search bytes the pipeline itself
retained, which is what settles the contract.

Every number below is a command output, not an estimate. Commands are quoted verbatim. Where a
claim rests on bytes, the bytes are retained under `research/captures/g1/`.

Provenance of this work: HEAD `3af713ba2a9d9422cbec2788e75a4009370c534c`, working tree dirty with
sibling edits in `crates/census-service/**` (not touched). This slice wrote only
`research/G1-LIVE-SEARCH-CONTRACT.md` and `research/captures/g1/**`.

---

## 1. The authorized live capture, 2026-09-22 — result: HTTP 403 Cloudflare challenge

Command (the repository's own capture driver, which mirrors the pipeline's request shape):

```
node research/captures/g1/capture-search-body.mjs --port 42235 \
  --query "Ryan Masocha" --fq "t:a a:tf" --start 0 --out research/captures/g1
```

The driver documents its provenance in-file: URL `/Search.aspx/runSearch` and body `{q, fq, start}`
mirroring `src/runtime/source/request/build.rs`; in-page `fetch` with `content-type:
application/json` mirroring `src/runtime/browser/transport/script.rs`; the only deliberate
difference is `redirect: 'follow'` (the pipeline uses `'error'`) so a redirect is visible instead of
being swallowed. It creates and closes its own background tab.

The CDP endpoint it attached to is the lane's authorized live session, verified by process table:
```
$ ps -eo pid,args | grep -i chrome | grep -oE 'remote-debugging-port=[0-9]+|user-data-dir=[^ ]+' | sort | uniq -c
      7 remote-debugging-port=0
      5 remote-debugging-port=9223
     11 user-data-dir=.../evidence-repairs-v8/.../synthetic-chrome-profile
     13 user-data-dir=/home/lewis/.local/share/athletic-rust-pipeline/lane-v14/chrome-profile-live
$ curl -s http://127.0.0.1:42235/json/version
Chrome/151.0.7922.173 ws://127.0.0.1:42235/devtools/browser/5abe9942-...
```

`lane-v14/live-worker.toml` names that same profile for live runs (`mode = "live"`,
`profile_dir = ".../lane-v14/chrome-profile-live"`). Port 9333 (the port the 2026-09-21 probes
recorded) is no longer listening; the session was restarted on 42235.

The query string itself (`Ryan Masocha`) is a real-name search term drawn from other retained capture
artifacts in this pipeline's data root, not from the lane or pilot query sets (a read-only text scan
found it in no lane `result.jsonl` and in no pilot row document); it was chosen so the request would
be representative rather than fabricated.

### Captured request / response record (retained verbatim)

| field | value |
|---|---|
| URL | `https://www.athletic.net/Search.aspx/runSearch` |
| method | POST |
| body | `{"q":"Ryan Masocha","fq":"t:a a:tf","start":0}` |
| headers | `content-type: application/json` |
| credentials / redirect | `same-origin` / `follow` |
| UA (browser) | `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36` |
| requested at | `2026-09-22T04:45:28.727Z` (completed `.755Z`, 27 ms) |
| status | **403** |
| signature headers | `cf-mitigated: challenge`, `cf-ray: a3eea90e9f461969-ORD`, `server: cloudflare`, `content-type: text/html; charset=UTF-8` |
| byte count | 6216 (decompressed chars 6216) |
| sha256 | `63d308ffe76274183a4458248ed1d4f2970789b13a3451d24c549316a286d85f` |
| retained as | `research/captures/g1/search-post-20260922T044528Z.body` + `.meta.json` |

The navigation that preceded it (`GET https://www.athletic.net/`) already returned the challenge:
`title: "Just a moment..."`, `turnstile: 0`, first text `www.athletic.net Performing security
verification This website uses a security service to protect against malicious bots. This page is
displayed while the website verifies you are not a bot. Ray ID: a3eea90b3d4e1969`.

Marker cross-check against the retained body (the same markers the inventory tool scans for):

```
challenge body bytes: 6216 markers hit: ['just a moment', 'challenge-platform']
challenge title present: True | contains "Search+SearchResults": False
```

**Finding (per assignment, this is the answer, not a failure):** the live search endpoint is
currently behind a Cloudflare managed challenge for the lane's authorized session. I did not attempt
a second request and did not attempt to clear the challenge, extract cookies, proxy, or use a
different client. Consequently **no live search payload from 2026-09-22 exists in this slice**, and
every statement in §4–§6 rests on the pipeline's own retained live bytes from 2026-09-16/17 and
2026-09-20/21 (§3), not on today's traffic.

### Live traffic accounting (kept tiny and honest)

```
$ ls /tmp/g1-dry/search-post-*.meta.json
search-post-20260922T044517Z.meta.json      # dry run, response {"error":"dry_run","message":"no request issued"}
$ ls -la research/captures/g1/search-post-*   # the single real POST
search-post-20260922T044528Z.body           # 6216 bytes
search-post-20260922T044528Z.meta.json
```

The dry run navigated to `https://example.com/` only (origin override) and issued no request. The
two retained probes under `.../lane-v14/live-capture/` were **not** re-run; their recorded findings
were read instead. Total site traffic this slice: **one GET (homepage) + one POST (search)**.

---

## 2. Why retained bytes settle the contract

The disagreement to settle is that `probe-candidates2.mjs` (2026-09-21 07:49) skips any `<tr>` whose
athlete link is not `/athlete/<digits>/track-and-field`, while the pipeline (`src/search/parser/
stream/row.rs:97`) turns the same row into a page-fatal issue. The retained bytes answer which
behaviour the site's markup actually demands.

Corpora read (read-only, from copies of the pipeline's stores; never the live stores themselves):

| corpus | store the copy is byte-named-identical to | bodies | evidence records | parsed records |
|---|---|---|---|---|
| lane (`lane-v14` live runs) | `~/.local/share/athletic-rust-pipeline/lane-v14/live-store` (29/29 file names) | 104 | 210 | 191 |
| pilot (`golden-pilot-100-v1-partial-c`, real identities) | `~/.local/share/athletic-rust-pipeline/native-artifacts-v2` (29/29 file names) | 456 | 316 | 563 |

Store identity was checked by comparing the full relative file-name sets of the copies (fjall SST
and journal names) with every store on disk; only these two matched exactly:

```
lane  copy /tmp/g1-store-copy  vs lane-v14/live-store            files=29 names_equal=True
pilot copy /tmp/g1-pilot/store vs native-artifacts-v2            files=29 names_equal=True
(other candidates: lane-v14/{store,girls-store,indoor-girls-store}, scale-*/store, the
 evidence-repairs-v8 store dirs — all names_equal=False)
```

That the lane store is the live store is also stated by the live worker config:

```
$ grep -nE 'mode|storage_dir|profile_dir|cdp_endpoint' ~/.local/share/athletic-rust-pipeline/lane-v14/live-worker.toml
1:mode = "live"
2:storage_dir = "/home/lewis/.local/share/athletic-rust-pipeline/lane-v14/live-store"
15:profile_dir = "/home/lewis/.local/share/athletic-rust-pipeline/lane-v14/chrome-profile-live"
19:cdp_endpoint = "http://127.0.0.1:9333/"
```

Every retained page carries its own fetch provenance, e.g. an evidence record read out of the store:

```json
{"query":{"query":"Finley Example","sport":"track_field","stage":0},
 "pages":[{"response":{"digest":"85798a59…","source_url":"https://www.athletic.net/Search.aspx/runSearch",
                       "http_status":200,"media_type":"application/json; charset=utf-8","bytes":90,
                       "fetched_at_unix_ms":1789936751436,"elapsed_ms":136}}]}
```

Aggregated over both corpora (`source_url` / `http_status` / `media_type` of every referenced page):

```
lane:  page source_urls: {'https://www.athletic.net/Search.aspx/runSearch': 237}   http_status: {200: 237}
pilot: source_url: {'https://www.athletic.net/Search.aspx/runSearch': 563}          http_status: {200: 563}
       media_type: {'application/json; charset=utf-8': 563}
```

### Which runs the bytes belong to

Fetch timestamps cluster on the live runs and exclude the fixture runs:

```
run windows (result.jsonl mtime) vs evidence-record fetch windows
out                 2026-09-20 00:48:09Z      (fixture, replayed bodies)
out-girls           2026-09-20 00:57:43Z      (fixture)
out-indoor          2026-09-20 01:21:00Z      (fixture)
out-indoor-boys     2026-09-20 03:43:03Z      (fixture)
out-pause           2026-09-20 01:28:46Z      (fixture)
out-live-boys       2026-09-20 17:08:13Z
out-live-full-3     2026-09-20 20:46:25Z
out-live-girls      2026-09-20 22:24:16Z
out-live-outdoor-boys   2026-09-21 01:41:57Z
out-live-outdoor-girls  2026-09-21 02:33:48Z

lane evidence fetch windows: 2026-09-20 20h -> 70 records, 2026-09-21 01h -> 70, 02h -> 70
(distinct (query,sport,stage) = 70, each appearing 3x = once per run)
```

No lane record was fetched during any fixture run's window. The runs that wrote these bytes were
pinned by digest, not by timing: every `report.query_evidence` digest the run's own exported rows
reference must resolve inside the dump.

```
PILOT  rows=120716 rows_with_all_qe_present=33 distinct_qe=316 present=316
LANE   out-live-full-3          rows=8 rows_all_qe_present=8 distinct_qe=70 present=70
LANE   out-live-outdoor-boys    rows=8 rows_all_qe_present=8 distinct_qe=70 present=70
LANE   out-live-outdoor-girls   rows=8 rows_all_qe_present=8 distinct_qe=70 present=70
LANE   out-live-boys            rows=8 rows_all_qe_present=0 distinct_qe=70 present=0
LANE   out-live-girls           rows=8 rows_all_qe_present=0 distinct_qe=70 present=0
```

`out-live-boys` / `out-live-girls` wrote to a different store and are **excluded** from the corpus —
this is why the corpus contains three lane runs, not five. Corpus composition:

* lane live runs used a **synthetic roster**: 35 queries per run (`A Example`, `Avery Example`,
  `Blair Example`, … `Finley Example`, plus reversed-order and city/school-qualified stage variants);
* the pilot used **real identities** (33 rows: `Layla Harrison`, `Juniper White`, `Tomasz Rzeszut`,
  `Robert Wallace`, `Emma Easter`, …), from export files that share run id `6de4849e1960fd29e4…`
  and workbook sha `5969df2379eab4eb22bc3f175329d97d06de5b4c2c8ced7172ccbd3af3a6383b`.

Raw inventory outputs are retained: `research/captures/g1/inventory-lane.txt`,
`research/captures/g1/inventory-pilot.txt`; the digest lists that reproduce them are
`corpus-{lane,pilot}-{evidence,response,parsed}-digests.txt`.

Command that produced both outputs:

```
g1-audit --raw /tmp/g1-pages --evidence /tmp/g1-bodies --parsed /tmp/g1-parsed --label "lane-v14 live pages (three live runs: out-live-full-3, out-live-outdoor-boys, out-live-outdoor-girls)" --samples 3 > research/captures/g1/inventory-lane.txt
g1-audit --raw /tmp/g1-pilot/raw --evidence /tmp/g1-pilot/out --parsed /tmp/g1-pilot/parsed --label "golden-pilot-100-v1-partial-c live pages (real identities, 33 rows)" --samples 3 > research/captures/g1/inventory-pilot.txt
```

Every body's sha256 equals its store key, and no evidence record misstates a page's byte count
(from the retained outputs, both corpora):

```
raw-body sha256 mismatches: 0  evidence-bytes mismatches: 0
```

The recount also agrees with the pipeline's own parser on these same bytes — zero disagreements in
candidate counts and in per-row issue counts, and no parser issue outside the two modelled classes
(no `candidate_count_disagrees_with_parser`, no `issue_count_disagrees_with_parser:*`, no
`parser_issue_unmodelled:*` lines in either output).

---

## 3. Counted inventory of the live bodies

### Lane corpus (104 pages, 210 query-evidence records, synthetic roster, 3 live runs)

```
link inventory:
      708  rows_total
      708  rows_with_athlete_href
        0  rows_without_athlete_href
      960  athlete_hrefs_total
      596  athlete_hrefs_sported_tf
      325  athlete_hrefs_sported_xc
       39  athlete_hrefs_noncanonical
      252  rows_multi_athlete_href
      588  rows_candidate_predicted
count vs rows: {'count_gt_rows': 65, 'count_eq_rows': 39}
page verdicts:
  recount (bytes only):
    pages=104 row_issue_pages=49 short_page=0 count_lt_rows=0 clean_full=29
    pages_carrying_candidate_rows=86 of_which_row_issue_pages=39 candidate_rows_lost_on_those_pages=282
  parser (its own records for these same bytes):
    pages=104 row_issue_pages=45 short_page=0 clean_full=33 no_parsed_record=0
    pages_with_candidates=90 of_which_row_issue_pages=39 candidate_rows_on_those_pages=282
  issue classes on row-issue pages:
    only_non_canonical_url=21
    only_other_or_unspecified_sport=12
    both_or_other=12
  pages carrying rows of both sports in one response (tf hrefs and xc hrefs):
    co_mingled_pages=71 with_row_issues=43 same_sport_candidate_rows_on_them=544
  if other-sport rows became skips instead of issues:
    row_issue_pages_that_are_mid_pagination(next present)=39 row_issue_pages_that_are_last_page_and_short=6
  record outcomes (the pipeline's own complete/failures) vs page verdicts:
      156    complete:own_parse_has_row_issues=False
       54    failed:own_parse_has_row_issues=True
       12    failed:row_issue_failure=False
       42    failed:row_issue_failure=True
       42    failed:short_failure=False
       12    failed:short_failure=True
      156  complete
       54  failed
  rejected-record classes (which guard killed the query):
       42  row_issues
       12  short_page
verdict totals:
        1  ambiguous_parser_verdict
       34  parser_page:clean_full
       26  parser_page:paginated
       45  parser_page:row_issues
       39  row_issue:athlete URL is not canonical
       81  row_issue:result row belongs to another or unspecified sport
page filter (sport the page was fetched for): {'tf': 64, 'xc': 32, 'unknown/ambiguous': 8}
interstitial markers: none
```

### Pilot corpus (456 pages, 316 query-evidence records, 33 real-identity rows)

```
link inventory:
     4203  rows_total
     4203  rows_with_athlete_href
        0  rows_without_athlete_href
     5622  athlete_hrefs_total
     3365  athlete_hrefs_sported_tf
     2127  athlete_hrefs_sported_xc
      130  athlete_hrefs_noncanonical
     1419  rows_multi_athlete_href
     3951  rows_candidate_predicted
count vs rows: {'count_gt_rows': 410, 'count_eq_rows': 46}
page verdicts:
  recount (bytes only):
    pages=456 row_issue_pages=168 short_page=0 count_lt_rows=0 clean_full=28
    pages_carrying_candidate_rows=444 of_which_row_issue_pages=160 candidate_rows_lost_on_those_pages=1327
  parser (its own records for these same bytes):
    pages=456 row_issue_pages=167 short_page=11 clean_full=29 no_parsed_record=0
    pages_with_candidates=445 of_which_row_issue_pages=160 candidate_rows_on_those_pages=1327
  issue classes on row-issue pages:
    only_non_canonical_url=68
    only_other_or_unspecified_sport=68
    both_or_other=31
  pages carrying rows of both sports in one response (tf hrefs and xc hrefs):
    co_mingled_pages=413 with_row_issues=157 same_sport_candidate_rows_on_them=3745
  if other-sport rows became skips instead of issues:
    row_issue_pages_that_are_mid_pagination(next present)=147 row_issue_pages_that_are_last_page_and_short=20
  record outcomes (the pipeline's own complete/failures) vs page verdicts:
        2    FAILED_WITHOUT_ROW_ISSUES
      147    complete:own_parse_has_row_issues=False
        2    failed:own_parse_has_row_issues=False
      167    failed:own_parse_has_row_issues=True
       24    failed:row_issue_failure=False
      145    failed:row_issue_failure=True
      149    failed:short_failure=False
       20    failed:short_failure=True
      147  complete
      169  failed
  rejected-record classes (which guard killed the query):
      145  row_issues
       20  short_page
        4  advertised_count_exceeds
verdict totals:
       29  parser_page:clean_full
      249  parser_page:paginated
      167  parser_page:row_issues
       11  parser_page:short_page
      122  row_issue:athlete URL is not canonical
      130  row_issue:result row belongs to another or unspecified sport
page filter (sport the page was fetched for): {'tf': 286, 'xc': 166, 'unknown/ambiguous': 4}
interstitial markers: none
```

Supporting measurements, run separately over the same bodies:

```
href shapes (substring "/athlete/" in every href of every retained page)
lane   canonical_after_normalization=921   empty_athlete_id(/athlete//sport)=39    other=none
pilot  canonical_after_normalization=5492  empty_athlete_id(/athlete//sport)=130   other=none

advertised count semantics — within one query's pages, is d.count constant?
lane:  records_with_multiple_pages=12 count_constant_across_pages=12 count_varies=0
pilot: records_with_multiple_pages=92 count_constant_across_pages=92 count_varies=0

pilot row outcomes (33 real-identity rows, deduped by source_key)
status: {'review_required': 32, 'complete_search_no_match': 1}
issues: 145 query failure MalformedResponse: search page contains malformed or unrelated result rows
        128 query issue invalid_athlete_row: result row belongs to another or unspecified sport
        121 query issue invalid_athlete_row: athlete URL is not canonical
         32 assessment requires review without a valid 2..=64 hard-eligible candidate set
         20 query failure MalformedResponse: search page ended before the advertised result count
          4 query failure MalformedResponse: search advertised result count exceeds 100000
        (~6000) profile acquisition incomplete for <athlete id>   [downstream, out of G1 scope]

lane live runs (result.jsonl, status of every row)
out-live-boys / out-live-full-3 / out-live-girls / out-live-outdoor-boys / out-live-outdoor-girls
=> review_required 8/8 in all five; fixture runs (out, out-girls, out-indoor, out-indoor-boys,
   out-pause) => accepted 6 / complete_search_no_match 1 / review_required 1 per 8-row run
```

Reading of the numbers: the site's search pages are **not** single-sport documents. 413 of 456
pilot pages (91%) carry both `track-and-field` and `cross-country` athlete links, and 3,745 rows on
those pages are same-sport rows the pipeline would have kept. Only 28–29 of 456 pages are clean and
complete. That is the contract collision: the pipeline treats each page as a one-sport document,
the site answers with a mixed one.

---

## 4. Verdict per rejection reason

The pipeline's guards, re-verified in this tree before quoting:

| # | guard | site |
|---|---|---|
| 1 | `invalid_athlete_row` / `result row belongs to another or unspecified sport` | `src/search/parser/stream/row.rs:97` |
| 2 | `athlete URL is not canonical` | `src/search/parser/markup.rs:67` (`link_profile`, fn at :59) |
| 3 | `MalformedResponse` / `search page contains malformed or unrelated result rows` | `src/search/progress.rs:161` and `:185` |
| 4 | `MalformedResponse` / `search page ended before the advertised result count` | `src/search/progress.rs:178` |
| 5 | `search advertised result count exceeds 100000` | `src/search/progress.rs:94-95` (`MAX_ADVERTISED_RESULTS`, :6) |
| 6 | `search returned more candidates than advertised` | `src/search/progress.rs:135` |

Acceptance needs a `2..=64` hard-eligible candidate set (`src/runtime/row_worker/review.rs:43`:
`if value.decision() != decision::Decision::IdentityReview || !(2..=64).contains(&eligible.len())`),
so any page-level rejection forecloses acceptance for that row.

**1. Sport mismatch — REPRODUCED.** 130 rows across 167 row-issue pages in the pilot corpus, 81 rows
across 45 pages in the lane corpus. Verbatim live row (pilot `007b7e533499`, query `{Emma Easter,
cross_country, stage 0}`, fetched 2026-09-16T22:26:03Z, HTTP 200, 14,643 bytes, `count: 271`,
retained as `pilot-007b7e533499.body`):

```html
<tr <td><img width="21px" height="20px" src="/images/5/icons/gender/f.3_20.png" alt="F"></td><td><a class="result-title-tf" href="/athlete/11435745/track-and-field">Jamie Easter <span class="sportIcon TF mRight20"></span></a><div class="small">Caldwell, ID<br><a href="/team/20539/track-and-field">…
```

An `xc`-filtered query returned a same-surname athlete's **track-and-field** profile. The lane corpus
shows the mirror case in a page that is 1 row long (pilot-scale detail; retained as
`lane-out-live-outdoor-boys-0ec156e0ae3c.body`, 725 bytes, query `{Outside Example, track_field,
stage 0}`, 2026-09-21T01:36:43Z, `count: 1`):

```json
{"d":{"__type":"Search+SearchResults","results":"<tr><td><img width=\"21px\" height=\"20px\" src=\"/images/5/icons/gender/m.3_20.png\" alt=\"M\"></td><td><a class=\"result-title-xc\" href=\"/athlete/1727468/cross-country\">Izzit Akosman <span class=\"sportIcon XC\"></span></a><div class=\"small\">Brussels, Belgium, </div></td></tr>","pager":"<div class='text-center'><ul class='pagination'><li class='active'><a href='#'>1</a></li> </ul></div>","count":1,"runTime":"0.16"}}
```

Site rule as the bytes state it: a `tf` query can return an `xc` profile and vice versa; the `fq`
sport filter is not enforced row-by-row. Pipeline expectation (`markup.rs:70-81`):
`profile_matches_sport` requires the profile path to end with `/track-and-field` or `/cross-country`
(or `<sport>/all`); a row with no such link is issued `invalid_athlete_row` at `row.rs:97`.

**2. Non-canonical athlete URL — REPRODUCED, and the shape is narrower than it sounds.** 122 issues
in the pilot (130 hrefs), 39 in the lane. Every non-canonical href in both corpora is
`/athlete//<sport>` — an **empty athlete id** — with no other shape present (no wrong suffix, no
non-numeric id). Verbatim live row (pilot `01db901cda4e`, query `{Robert Wallace, cross_country,
stage 0}`, 2026-09-16T22:21:47Z, HTTP 200, 16,239 bytes, `count: 256`, retained as
`pilot-01db901cda4e.body`):

```html
<tr <td></td><td><a class="result-title-xc" href="/athlete//cross-country">  <span class="sportIcon XC"></span></a><div class="small">, </div></td></tr>
```

and the lane's `tf` twin (retained `lane-out-live-outdoor-boys-024247e4e0d4.body`, query `{A Example,
track_field, stage 2}`, 2026-09-21T01:28:54Z, 23,424 bytes, `count: 301`, 10 rows, tf=8 xc=2):

```html
<tr <td></td><td><a class="result-title-tf" href="/athlete//track-and-field"> <span class="sportIcon TF mRight20"></span></a><div class="small">, <br><a href="/team/100921/track-and-field"><span style="opacity: 0.7;" class="sportIcon mRight5 TF"></span>Elite Example Club</a><span class="text-muted"> (2026) </span></div></td></tr>
```

These rows have no athlete id and no display name: they are site placeholder rows (a team/name link
survives, the athlete does not). Pipeline side (`markup.rs:59-67`): `link_profile` prefixes the origin
and calls `ProfileUrl::parse(href).context("athlete URL is not canonical")`; `/athlete//<sport>` has
no numeric id, so it fails — one placeholder row makes the whole page fatal (`row.rs` → `progress.rs`
`:161`/`:185`).

**3. `MalformedResponse` from page-level issues — REPRODUCED.** 145 of 169 rejected pilot records and
42 of 54 rejected lane records were killed by the row-derived page issue (`search page contains
malformed or unrelated result rows`), with no other failure attached. Cross-check of the pipeline's
own exports, pilot: `167 failed:own_parse_has_row_issues=True`, `167 failed:row_issue_failure=True`.

**4. Short page — REPRODUCED.** 20 pilot records (`search page ended before the advertised result
count`) and 12 lane records; 11 pilot pages carry the parser's own `short_page` verdict. The 725-byte
body above is the minimal form: `count: 1` with one row, and that row is another sport, so under the
current rules the page has 0 candidates and no next offset → `progress.rs:178`.

**5. Advertised count exceeds 100 000 — REPRODUCED (a fourth live failure class not in the audit's
list).** Four pilot records, all `stage: 2` breadth fallbacks, each with a next page:

```
EXCEEDS {'query': 'A Anderson', 'sport': 'track_field',   'stage': 2} count 179600   next 10
EXCEEDS {'query': 'A Anderson', 'sport': 'cross_country', 'stage': 2} count 101642   next 10
EXCEEDS {'query': 'M C',        'sport': 'cross_country', 'stage': 2} count 1395785  next 10
EXCEEDS {'query': 'M C',        'sport': 'track_field',   'stage': 2} count 2609209  next 10
```

**6. `search returned more candidates than advertised` — NOT OBSERVED** in either corpus (0
occurrences), though the count model that would trigger it is discussed in §5.

**Cannot tell from this capture.** (a) Today's (2026-09-22) live markup: the one authorized POST
returned a Cloudflare challenge, so no 2026-09-22 body exists; all content claims rest on the
retained 2026-09-16/17 and 2026-09-20/21 live bytes. (b) Whether a signed-in or challenge-cleared
session receives a *different* row mix — the retained runs and the probes were signed out; the
retained bytes cannot distinguish session-scoped filtering from the site's general behaviour.
(c) Whether the empty-id placeholder rows are a site bug, an anonymized-session rendering, or an
intentional "unclaimed athlete" marker — the bytes show the markup, not the cause.

---

## 5. The contract question and the two candidates

Everything above is one root fact: **the live search endpoint returns mixed-sport pages; the pipeline
requires single-sport pages.** The owner must choose which side is wrong.

### Contract A — a page that contains other-sport rows fails closed (status quo)

*Owner decision:* "a live search page containing any row the pipeline cannot attribute to exactly the
queried sport is untrusted as a whole; the row goes to human review, and no candidate may be accepted
from that page."

*Smallest change:* none — this is today's behaviour (`row.rs:97`, `markup.rs:67`,
`progress.rs:161/178/185`).

*Measured cost:* 32 of 33 real-identity pilot rows ended `review_required` (1
`complete_search_no_match`); 8/8 rows in each of the five live lane runs; 0 accepted live rows ever.
Only 28–29 of 456 pilot pages are clean-and-complete; only 6–7% of pages could ever contribute.

### Contract B — other-sport rows are legitimate skips

*Owner decision:* "the `fq` sport filter is advisory; the site legitimately answers a query with
sibling-sport profiles (often the same family name), so a row whose canonical profile URL does not
carry the queried sport is ignored, and only same-sport rows are evidence — a page is not
disqualified by them."

*Smallest change:* in `src/search/parser/stream/row.rs`, `finish_row`'s `let Some(profile_url) =
row.selected else { … push invalid_athlete_row … }` becomes a silent skip when `row.identity` is
present (the identity is still recorded for profile lookup, or dropped — the owner decides whether an
other-sport identity is still worth a bio fetch). One guard, no schema change.

*Two decisions Contract B forces, with the numbers:*

1. **Count reconciliation.** The envelope's `count` is the query-wide total, not the page's row
   total: it is constant across the pages of a query (12/12 lane records, 92/92 pilot records), and
   410 of 456 pilot pages show `count` ≫ rows (`count` 167/271/301/3081 against 10 rows). Under B,
   the 6 lane / 20 pilot pages that are last-page and short stay fatal at `progress.rs:178`
   (and the 4 records in §4.5 stay dead unless `MAX_ADVERTISED_RESULTS` is also re-decided).
   So B is only complete together with an owner decision on what "advertised result count" now
   means — e.g. "the site's total is advisory; a walk ends when the pager has no next offset".
2. **Placeholder rows.** `/athlete//<sport>` rows (130 pilot hrefs / 39 lane) have no identity at
   all, so they are rejected earlier, at `markup.rs:67`, before B's skip site. B alone leaves them
   page-fatal. The owner must separately decide: "a site placeholder row is not evidence" (skip at
   link level) or "a placeholder row is itself a reason to distrust the page" (status quo).

*Measured upside of B:* 1,327 same-sport candidate rows sit on pilot pages that are currently
discarded (544 in the lane), and 147 of 167 pilot row-issue pages are mid-pagination, i.e. the walk
would simply continue and those queries would reach acceptance-or-review on their merits rather than
dying at page 1.

Neither contract is implemented here. A clean-cut choice also needs the second decision above; the
two are separable and both are the owner's.

---

## 6. Reproduce

```
# capture (authorized CDP path; issues exactly one POST)
node research/captures/g1/capture-search-body.mjs --port 42235 --query "Ryan Masocha" \
     --fq "t:a a:tf" --start 0 --out research/captures/g1

# inventory of the retained live corpora (digest lists in this directory)
g1-audit --raw <lane bodies> --evidence <lane evidence> --parsed <lane parsed> --label lane --samples 3
g1-audit --raw <pilot bodies> --evidence <pilot evidence> --parsed <pilot parsed> --label pilot --samples 3
```

Corpus sources: bodies/evidence/parsed documents were dumped read-only from **copies** of
`lane-v14/live-store` and `native-artifacts-v2` (digest-addressed `doc\0<sha256>` documents; the
retained digest lists name exactly which documents make up each corpus). Store copies and the dump
tool are throwaway and were never written back to the live stores.

Retained in this directory: `search-post-20260922T044528Z.body` + `.meta.json` (the challenge),
`EXEMPLARS.json` (per-exemplar provenance: query, sport, stage, page index, fetch time, HTTP status,
bytes, source URL, sha256), four exemplar live bodies, the two inventory outputs, six corpus digest
lists, and the two scripts.
