# CEN-14 public identity-lineage qualification — 2026-10-08 acquisition

## Scope and result

Reviewer: **openai-codex/gpt-6.1-sol**. Ownership is this directory only. No production code, tests, shared contracts, historical fixtures, historical evidence, or other runs' state were changed. No builds, tests, formatters, lints, benchmarks, services, or Git writes were run.

**BLOCKED: no real independent school-authoritative roster / timing-results pair publishing the same admitted numeric provider-person key was acquired.** This is a qualification blocker, not a positive Class B integration result. There are zero `IndependentPublished` assertions in this deliverable. Names, grade, school, SIDEARM player IDs, provider team IDs, and different hostnames cannot substitute for the missing provider-person binding. A Class-2027 timing row and a genuine mirrored negative are available below.

Read contracts: repository `AGENTS.md`, `ARCHITECTURE.md`, ADR index and ADR-022, school-sites and timing-providers-national source reports, state-assoc-greatlakes's existing mirrored-source analysis, and `identity_attestation.rs`, `identity_index.rs`, `identity_corroboration.rs`. Startup review and Rust/async/DDD skills were read. The current index requires subject-bound attestations, compatible parsed cohort/gender, a primary admitted provider-person key, and independent document digest, source family, and upstream producer. A known source object is not automatically independent evidence.

## Capture method and acquisition facts

All live requests below used TLS-verified HTTPS, ordinary default curl identification, no authorization, cookies, browser impersonation, challenge solving, retries, or automatic redirect following. Maximum response size was 4 MiB and request timeout 60 seconds. Each returned body is complete and unmodified; associated HTTP headers are `<stem>.headers`. `.json` extensions on the failed IHSA captures do **not** imply JSON content: they contain the 153-byte HTTP 403 HTML response. DNS failures produced only empty header files and **no response body**.

For each table row, the exact acquisition command is the following recipe with that row's literal URL, body filename, corresponding header filename, and indicated delay substituted; execution directory was `/tmp/athletic-census-cen-20261007`:

```sh
# Delay prefix, when listed: sleep 30 &&, sleep 10 &&, or sleep 2 &&
date -u +%Y-%m-%dT%H:%M:%S.%NZ && curl --proto '=https' --max-time 60 --max-filesize 4194304 --dump-header research/sources/identity-lineage-20261007/captures/HEADER --output research/sources/identity-lineage-20261007/captures/BODY --write-out 'status=%{http_code} bytes=%{size_download} url=%{url_effective} redirect=%{redirect_url} ip=%{remote_ip} ssl=%{ssl_verify_result}\n' 'URL' && date -u +%Y-%m-%dT%H:%M:%S.%NZ
```

The executed command spelled `--output` as `-o`. Preserve the quoted URL substitution, especially AthleticLIVE's literal `$web` path. The first Loyola and FlashResults robots commands instead used `--write-out '%{json}\n'`; this changes only diagnostic output, not captured bytes. No redirect target was requested implicitly. Completed-response UTC instants below are local acquisition observations, not HTTP `Date`, provider publication dates, upstream fetch timestamps, or inferred fixture timestamps. Acquisitions occurred **after** the requested 2026-10-07 as-of date; underlying 2026 result dates do not turn these into as-of captures.

All 50 raw files, including headers and empty failure artifacts, are individually SHA-256 recorded in `captures/SHA256SUMS`. That manifest supplies the full raw-body digest for every table row; checksums cover bytes, not reader-rendered text. The raw files totaled **7,600,958 bytes** before adding the checksum manifest.

| Body in `captures/` | Exact URL | Observed completion UTC | HTTP / body bytes | Explicit delay prefix |
|---|---|---|---|---|
| `loyola-robots.txt` | https://athletics.loyolahs.edu/robots.txt | 2026-10-08T03:16:22.559052414Z | 200 / 6001 | none |
| `flashresults-robots.txt` | https://www.flashresults.com/robots.txt | 2026-10-08T03:16:22.612059452Z | 200 / 4640 | none |
| `loyola-xc-roster.html` | https://athletics.loyolahs.edu/sports/cross-country/roster | 2026-10-08T03:16:46.969835819Z | 200 / 1020497 | none; noncompliant delay |
| `loyola-allyn.html` | https://athletics.loyolahs.edu/sports/cross-country/roster/christopher-allyn/2540 | 2026-10-08T03:17:20.102214799Z | 200 / 261326 | none; noncompliant delay |
| `flashresults-adidas-hs.htm` | https://www.flashresults.com/2027_Meets/xc/adidasXC/203-1_compiled.htm | 2026-10-08T03:17:12.788349681Z | 200 / 101497 | none |
| `gomats-robots.txt` | https://gomats.org/robots.txt | 2026-10-08T03:18:36.336439079Z | 200 / 6001 | none |
| `tfrrs-robots.txt` | https://www.tfrrs.org/robots.txt | 2026-10-08T03:18:36.302654196Z | 200 / 99 | none |
| `ihsa-robots.txt` | https://api.ihsa.org/robots.txt | 2026-10-08T03:18:57.855151926Z | 403 / 153 | none |
| `athleticlive-robots.txt` | https://athleticlive.blob.core.windows.net/robots.txt | 2026-10-08T03:18:57.929963512Z | 400 / 226 | none |
| `tfrrs-stampede.html` | https://www.tfrrs.org/results/xc/27822/Southern_Stampede_Cross_Country_Meet_-_High_School_ | 2026-10-08T03:18:57.910012862Z | 200 / 815035 | none |
| `gomats-xc.html` | https://gomats.org/sports/cross-country | 2026-10-08T03:19:28.220289216Z | 200 / 632484 | 30 seconds |
| `ihsa-2790204.json` | https://api.ihsa.org/v1/track-field/events/2790204/summary | 2026-10-08T03:19:49.701479307Z | 403 / 153 | none |
| `athleticlive-2790204.json` | https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/2790204 | 2026-10-08T03:19:49.814249856Z | 200 / 23628 | none |
| `gomats-xc-roster.html` | https://gomats.org/sports/cross-country/roster/2026 | 2026-10-08T03:20:41.241667791Z | 200 / 1192221 | 30 seconds |
| `stx-robots.txt` | https://stxsports.net/robots.txt | 2026-10-08T03:21:28.344515607Z | 200 / 6001 | none |
| `stx-home.html` | https://stxsports.net/ | 2026-10-08T03:22:17.345308161Z | 200 / 446786 | 30 seconds |
| `stx-xc-roster.html` | https://stxsports.net/sports/cross-country/roster | 2026-10-08T03:22:59.387147108Z | 200 / 1760042 | 30 seconds |
| `stx-adams.html` | https://stxsports.net/sports/cross-country/roster/parker-adams/9734 | 2026-10-08T03:24:15.787103660Z | 200 / 318897 | 30 seconds |
| `jesuittrack-robots.txt` | https://www.jesuittrack.org/robots.txt | 2026-10-08T03:25:44.638030987Z | 200 / 30 | none |
| `jesuittrack-home.html` | https://www.jesuittrack.org/ | 2026-10-08T03:26:04.532811597Z | 200 / 66297 | 10 seconds |
| `jesuithighschool-robots.txt` | https://www.jesuithighschool.org/robots.txt | 2026-10-08T03:26:49.635965855Z | 200 / 118 | none |
| `jesuithighschool-xc.html` | https://www.jesuithighschool.org/cross-country | 2026-10-08T03:27:00.198641612Z | 301 / 154 | 2 seconds |
| `jesuithighschool-teams.html` | https://www.jesuithighschool.org/student-life/athletics/teams-seasons | 2026-10-08T03:27:09.328552259Z | 200 / 815097 | 2 seconds |

Failed-policy attempts, with no response/acquisition instant: `https://www.greatoakxc.com/robots.txt` started 2026-10-08T03:25:44.429359853Z; `https://www.newburyparkcrosscountry.net/robots.txt` started 2026-10-08T03:26:43.668688099Z; `https://www.yorkxc.com/robots.txt` started 2026-10-08T03:26:43.668685349Z. All returned curl exit 6, `Could not resolve host`, HTTP 000, zero bytes. Their empty `*-robots.headers` are retained. No homepage requests, DNS circumvention, or alternate-host identity assumptions followed.

### Admission observations and disclosed lapse

Loyola, Miramonte, and St. Xavier expose identical SIDEARM wildcard robots groups with `Allow: /` and **Crawl-delay: 30**, while disallowing assets, `/common/`, `/documents/`, `/admin/`, `/services/`, `/site/`, `/hidden/`, and certain file/query patterns. The roster/bio paths themselves are not disallowed. The first Loyola roster request started 03:16:45.343005689Z, about 23.0 seconds after the robots start; its bio request started 03:17:12.483463532Z, about 27.1 seconds after the roster start. Both violate the observed 30-second delay. They are preserved as **non-admission-certified exploratory negatives**, never certified positives. Dispatch preceded inspection of the relevant wildcard group. This was disclosed to Main. Later SIDEARM requests explicitly waited 30 seconds. No promising positive binding was found to re-acquire compliantly.

FlashResults' wildcard policy permits the requested HTML result path; it does not publish a crawl delay and disallows unrelated asset/PDF/CSV/private-result paths. TFRRS' robots body contains only a documentation comment, no restrictions or delay. AthleticLIVE blob robots returned HTTP 400 XML, not a policy; the repository's missing/non-200 robots rule was used for its known public result object. IHSA robots and event summary returned 403; no usable IHSA policy, successful fresh IHSA result acquisition, or bypass is claimed. Requests to that host stopped after the denied summary. JesuitTrack's wildcard policy declares a 10-second delay, observed before its homepage. Modern Jesuit school's robots disallow `/bin`, `/bin/`, `/secure/`; the two published athletics paths are outside those exclusions and no crawl delay is published. Its same-host 301 was followed by a separate bounded request after inspection, not automatic curl redirection.

Reader-only search/scouting attempts were not retained as raw source captures and are **not admission-certified evidence**. No uniform compliance claim covers that exploratory search history.

## Positive qualification search and exact missing facts

| Examined document / producer rationale | What was actually published | Why it cannot qualify |
|---|---|---|
| Loyola High School Los Angeles official athletics roster and Christopher Allyn bio; school-primary SIDEARM publishing, not an independently identified timing producer | 2026 XC roster; Christopher Allyn, sophomore; `data-player-id="2540"`; corresponding local biography URL | No Athletic.net, MileSplit, TFRRS, or DirectAthletics provider reference in either raw page. SIDEARM 2540 is not an admitted provider-person key. Acquisition also has the disclosed delay violation. |
| Miramonte official athletics program and 2026 roster; school-primary SIDEARM publishing | Mia Abram, sophomore; `data-player-id="5694"`; `/sports/cross-country/roster/mia-abram/5694` | Entire raw roster has no reference to those four providers. Local player ID and school/cohort/name do not establish a binding. |
| St. Xavier official school athletics homepage, 2026 XC roster and Parker Adams bio; school-primary SIDEARM publishing | Parker Adams, sophomore; `data-player-id="9734"`; matching school biography URL | Entire roster and biography have no reference to those four providers. No admissible person key to compare to timing evidence. |
| TFRRS Southern Stampede High School XC results; page names **Midwest Timing & Results** as timing producer, TFRRS/DirectAthletics as publishing platform | September 19, 2026; Payton Chapelle, **2027**, Eudora, 14:41.4; explicit `https://www.directathletics.com/athletes/track/9450666.html` | This is a real timing-side `DirectAthleticsAthlete(9450666)` candidate, not `TfrrsAthlete(9450666)`. No captured independent Eudora school roster publishes that person key. Producer independence from a missing school document cannot be certified. |
| FlashResults adidas XC Challenge HS result; independently produced timing page | Actual result date September 18–19, 2026, HS grades and marks | No admitted provider-person links on this HS result. The URL's `2027_Meets` folder is not graduation-year evidence. College links elsewhere in the existing provider report do not establish a high-school positive. |
| JesuitTrack historical team homepage; self-described school team archive | Explicitly an archive for **2000–2011**, with Athletic.net division rankings and **school** records `SchoolID=739`; links to the modern school | No provider-person link, current roster, or requested-date evidence. Team/division IDs are not athlete IDs. The historical host's current school-authoritative ownership is not independently certified. |
| Modern Jesuit High School published XC URL and explicit redirect target | XC URL redirects to general athletics teams/seasons page. Static markup inspection found no provider references or anchor matching XC/roster/provider searches | No independently published roster-person binding in the acquired response. Static inspection is not evidence about unexecuted browser rendering or unrequested CMS services. No browser/UI execution is claimed. |
| Great Oak, Newbury Park, York team-domain leads | Robots requests all failed DNS | No site authority, roster, policy, or provider key could be established. No alternative identity inferred. |
| IHSA live endpoint | HTTP 403 | No fresh event bytes. Historical copy is analyzed only as a mirrored negative below. |

Additional reader-tool queries, exact attempted URLs and observations:

- `https://www.google.com/search?q=high+school+cross+country+roster+%22athletic.net%2Fathlete%22` and `https://www.google.com/search?q=%22milesplit.com%2Fathletes%22+%22roster%22&gbv=1`: redirect/help responses, no usable school-provider binding; not circumvented.
- `https://www.bing.com/search?q=%22roster%22+%22athletic.net%2Fathlete%22+%22high%22` and `https://www.bing.com/search?q=%22cross+country%22+%22roster%22+%22milesplit.com%2Fathletes%22`: no useful result binding.
- `https://www.bing.com/search?q=high+school+cross+country+roster+milesplit+athlete&format=rss`: unrelated dictionary results for “high”.
- `https://www.bing.com/search?q=%22milesplit.com%2Fathletes%22%20%22roster%22&format=rss`: generic MileSplit roots, not an independent school roster.
- `https://www.bing.com/search?q=%22athletic.net%2Fathlete%22%20%22school%22%20-site%3Aathletic.net&format=rss`: generic provider/news results, no qualifying key.
- `https://html.duckduckgo.com/html/?q=%22roster%22+%22athletic.net%2Fathlete%22`: generic search page, no useful result binding.
- `https://search.yahoo.com/search?p=%22roster%22+%22athletic.net%2Fathlete%22`: reader HTTP 500 downstream `_bv/v.gif`; no useful result binding.
- `https://www.loyolahs.edu/athletics/`: school reader redirected to its published athletics host; `https://athletics.loyolahs.edu/index.aspx` supplied roster navigation. `https://gomats.org/` supplied the school program navigation. Raw timing/school captures in the table, not these reader renderings, are the evidence.

**Missing prerequisite:** a publicly accessible, admission-compliant, independently school-authored HS roster/person document explicitly binding an admitted numeric provider-person key to its own published athlete entry, plus a separately produced timing-results document explicitly publishing that same key. The TFRRS row provides one concrete timing-side candidate, DirectAthletics 9450666, for which the independent school-primary document is missing. A public URL publishing such a school binding would allow a new scoped acquisition; speculative matches cannot.

## Genuine mirrored negative: Athletic.net athlete 27740691

`captures/ihsa-2790204-historical.json` is a byte-identical copy, not a new fetch, of `crates/census-crawl/tests/fixtures/ihsa_tournament/event_2790204_boys_hj_1a_finals.json`. Original acquisition instant is **unknown**. Its upstream `resultsFetchedAt="2026-05-31T12:51:11.683Z"` and `metadataFetchedAt="2026-05-31T12:51:11.653Z"` are retained but **not** used as local `acquired_at`. No test-owned fixed instant is substituted. Historical raw-body SHA-256 is `ffa73baef90e494bb9b74b5473f98efd26ce083cf22c7efe12d80cf57eae0611`.

Fresh AthleticLIVE SHA-256 is `53cdaad5d04e9084bc4ba6340f3926bda27abfd69f62b3d4aebb5479c0b8fb00`, acquired 2026-10-08T03:19:49.814249856Z. Both explicitly publish **Athletic.net athlete 27740691**, not merely the same name:

| Binding / shared event detail | Historical IHSA JSON pointer | Fresh AthleticLIVE JSON pointer | Observed value |
|---|---|---|---|
| Admitted provider-person key | `/finishers/0/athlete/athleticNetId` | `/_source/r/0/a/ani` | 27740691 |
| Upstream live athlete object | `/finishers/0/athlete/athleticLiveId` | `/_source/r/0/a/i` | 49752378 |
| Name / grade | `/finishers/0/athlete/name`, `/finishers/0/athlete/year` | `/_source/r/0/a/n`, `/_source/r/0/a/y` | Kehlin Crawford / `"11"` |
| Live team / provider team | `/finishers/0/team/athleticLiveId`, `/finishers/0/team/athleticNetId` | `/_source/r/0/a/t/i`, `/_source/r/0/a/t/ani` | Flora / 1679604 / 16352 |
| Event / meet | `/eventId`, `/meetId` | `/_source/i`, `/_source/mi` | 2790204 / 74003 |
| Mark / scaled mark | `/finishers/0/mark`, `/finishers/0/imperialMark` | `/_source/r/0/m`, `/_source/r/0/im` | `2.02m` / 2020000 |

The existing state-association report identifies IHSA's AthleticLIVE-derived state-meet feed. These shared upstream object IDs and projected marks support that characterization directly. Source families may be named `ihsa-tournament` and `athleticlive`, but the upstream producer for **both** is the same AthleticLIVE state-meet result stream, meet 74003/event 2790204. A different hostname, transform, digest, or acquisition instant does not make this independent corroboration.

Qualification: **CandidateOnly on both sides**, useful for a mirrored-negative scenario, not for Class B graduation. The historical side additionally lacks local acquisition metadata required for a validated capture-backed attestation. No full historical attestation with an invented `acquired_at` is supplied. This is genuine published identity/mirror structure, not a fresh two-capture lineage certification. No identity-index integration scenario was executed in this worker.

## Executed verification and limits

Execution directory is repository root unless noted. `P` below denotes literal `research/sources/identity-lineage-20261007/captures`; expand it when replaying.

1. `sha256sum P/* && wc -c P/*` before the manifest: exit 0, complete digests and **7,600,958 total bytes**. Includes preserved HTTP failures and three empty DNS-failure headers.
2. `sha256sum --check SHA256SUMS` in `P`: exit 0, **50 files printed `OK`**, no missing/failed/improperly-formatted rows.
3. `cmp crates/census-crawl/tests/fixtures/ihsa_tournament/event_2790204_boys_hj_1a_finals.json P/ihsa-2790204-historical.json`: exit 0, no output, byte-identical historical copy; original fixture unchanged.
4. Specialized `grep` tool with regex `athletic\.net|milesplit|tfrrs|directathletics`, case-insensitive, on the six complete files `loyola-xc-roster.html`, `loyola-allyn.html`, `gomats-xc-roster.html`, `stx-xc-roster.html`, `stx-adams.html`, `jesuithighschool-teams.html` in `P`: **No matches found**. This demonstrates absence of those provider references in these exact raw responses, not a universal absence on the schools' websites.
5. `xmllint --html --recover --xpath '//a[@href="https://www.directathletics.com/athletes/track/9450666.html"]/ancestor::tr[1]' P/tfrrs-stampede.html`: exit 0, printed the Payton Chapelle / 2027 / Eudora / 14:41.4 row. Also printed original-source duplicate-ID and unmatched-anchor parser diagnostics. Recovery extraction is not strict-HTML validation.
6. `xmllint --html --recover --xpath '//a[contains(translate(string(.),"ABCDEFGHIJKLMNOPQRSTUVWXYZ","abcdefghijklmnopqrstuvwxyz"),"cross") or contains(@href,"cross") or contains(@href,"roster") or contains(@href,"athletic.net") or contains(@href,"milesplit")]/@href' P/jesuithighschool-teams.html`: exit 11, **XPath set is empty**. No browser-rendered surface claim follows from this.
7. Initial `jq -e -s` cross-file probe failed exit 5, `Error: cannot use null as iterable (array or object)`. A diagnostic `jq -s 'map({keys:keys, eventId, source_type:(._source | type)})'` on both negative files printed **two separate one-element arrays**, inconsistent with real jq's multi-file slurp. Reported to `xd://report_issue`; invoking `/usr/bin/jq` explicitly uses the real binary. First real-binary comparison used guessed legacy field names (`liveId`, `grade`, nested `athlete.team`) and printed `false`, exit 1. Direct field inspection corrected these to published `athleticLiveId`, `year`, and sibling `team`. This was a probe-schema mistake, not a demonstrated production defect. An earlier `.data.finishers` exploratory probe similarly failed because the historical JSON's root is the event object, not `.data`.
8. Final real-binary cross-file smoke below printed **`true`**, exit 0. It proves the enumerated numeric-key/upstream-object/mark equalities only, not independent lineage or identity-index execution:

```sh
/usr/bin/jq -e -s '.[0].finishers[0] as $state | .[1]._source as $feed | $feed.r[0] as $timing | [$state.athlete.athleticNetId == 27740691, $state.athlete.athleticNetId == $timing.a.ani, $state.athlete.athleticLiveId == $timing.a.i, $state.athlete.name == $timing.a.n, $state.athlete.year == $timing.a.y, $state.team.athleticLiveId == $timing.a.t.i, $state.team.athleticNetId == $timing.a.t.ani, (.[0].eventId | tonumber) == $feed.i, .[0].meetId == $feed.mi, $state.mark == $timing.m, $state.imperialMark == $timing.im] | all' research/sources/identity-lineage-20261007/captures/ihsa-2790204-historical.json research/sources/identity-lineage-20261007/captures/athleticlive-2790204.json
```

Builds, tests, gates, actual identity graduation, browser rendering, and live crawl scheduler execution are **unexecuted here by assignment**. Main owns all such validation. No optimization or completeness certification is claimed.

## Round ledger

[BLOCKED] high captures/tfrrs-stampede.html:777 — Independent school-primary document publishing the same provider-person key as the real Class-2027 timer row is missing — `xmllint --html --recover --xpath '//a[@href="https://www.directathletics.com/athletes/track/9450666.html"]/ancestor::tr[1]' P/tfrrs-stampede.html` printed Payton Chapelle / 2027 / Eudora; complete-school-response provider grep returned No matches found; zero qualified positive pairs.

[OPEN] medium captures/loyola-robots.txt:332 — Two initial Loyola acquisitions violated published 30-second crawl delay; bytes retained and excluded from admission-certified positives — acquisition recipe/table: robots start 03:16:22.340332520Z, roster start 03:16:45.343005689Z, bio start 03:17:12.483463532Z; start gaps ~23.0s / ~27.1s, not 30s. Historical noncompliance cannot be retroactively fixed; later SIDEARM requests explicitly waited 30 seconds.

[BLOCKED] medium captures/ihsa-2790204.json:1 — Fresh IHSA event and readable robots policy unavailable; historical capture lacks known local acquisition instant — table's exact curl requests printed HTTP 403 / 153 bytes for both robots and event; `cmp` exited 0 for preserved historical copy; upstream fetch timestamps are not acquisition metadata.

[FIXED] informational captures/athleticlive-2790204.json:1 — Mirrored negative is established from genuine explicit numeric person/upstream object keys rather than name matching — final `/usr/bin/jq -e -s` command above printed true, exit 0; both sides remain CandidateOnly. This records completed evidence qualification, not a production-code fix.

## Handoff

- **TASK:** CEN-14 real public positive/negative lineage evidence qualification.
- **OWNERSHIP:** this directory, raw bodies/headers, checksum manifest, this report.
- **DO NOT MODIFY:** production/domain types, test modules, other research bundles, shared docs or schemas, historical capture/evidence entries, durable run state.
- **INPUT CONTRACT:** own-entry explicit admitted provider-person key; real complete capture hash and locally observed acquisition time; independently justified source family and producer; no name-only or school-local-ID substitution.
- **OUTPUT CONTRACT:** immutable raw evidence and truthful provenance; no IndependentPublished positive; genuine same-upstream numeric-key negative plus documented missing historical acquisition instant.
- **ACCEPTANCE:** scoped raw-byte and mirror smoke checks executed above; real positive capture qualification BLOCKED; project-wide/identity-index acceptance unexecuted and owned by Main.
- **HANDOFF:** Main can use the fresh AthleticLIVE candidate and historical mirrored structure without fabricating historical acquisition time. Main must not count this as the real Class B positive acceptance. Needed input is the independent school-primary binding document described above. No code patch or permanent regression was warranted by this source investigation.

VERDICT: BLOCKED 2
