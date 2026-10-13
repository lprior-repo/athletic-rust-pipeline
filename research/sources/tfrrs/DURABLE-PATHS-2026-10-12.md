# TFRRS durable-path qualification - 2026-10-12 (bead `athletic-rust-pipeline-3bo`)

Staged section for `research/sources/tfrrs/SOURCE_REPORT.md`. This lane's write tool replaces whole
files and cannot append (append selector probes created new files instead of appending), and
re-transcribing a 30 KB report to splice in a section risks destroying evidence - so the section is
staged here for the report owner/main agent to merge verbatim.

## Equipment used

Anonymous fetches only (no cookies, no auth). Same-host requests spaced >=1s. Evidence and digests:
`captures/PROVENANCE.md`. Fixtures: `crates/census-crawl/tests/fixtures/tfrrs/`.

## Headline

- **robots.txt permits.** `https://www.tfrrs.org/robots.txt` and
  `https://indiana.tfrrs.org/robots.txt` both return the stock single comment line with no
  `User-agent`/`Disallow` records (98 bytes each, one digest). Nothing is disallowed by robots.
- **The terms refuse automated acquisition.** TFRRS is a DirectAthletics product (its page footers
  read `Copyright (c) 2026 DirectAthletics, Inc.`), and the platform's Terms of Use state:
  *"You agree that you will not use any robot, spider, scraper, or other automated means to access
  the sites for any purpose without our express written permission."* and prohibit distribution or
  exploitation of *"performance lists, roster data, ... DirectAthletics IDs, TFRRS IDs"*. Text:
  `samples/directathletics-tfrrs/dat-terms-of-use-excerpt.txt`. Consequence: **every path below is
  technically reachable and parseable but is REFUSED for automated census acquisition** until
  written permission exists. robots.txt does not override this; the existing "Known blocks - none
  observed" line must not be read as permission.
- **Discovery gained a hop that the report does not describe.** Index pages no longer publish
  `/lists/{id}/{slug}` anchors. The body of `indoor_lists.html` / `outdoor_lists.html` contains only
  lazy turbo-frames (`<turbo-frame id="directory_tab_*" src="/directory_tab.html?outdoor=0&tab=...&year=2026">`);
  the `/lists/...` links are emitted by `/directory_tab.html` itself. A crawler that parses index
  anchors today discovers **zero** lists. Fixtures: `indiana_directory_tab_hsr_all.excerpt.html`,
  `nh_indoor_lists_discovery.excerpt.html`.
- **Tab ids are instance-specific.** Indiana: `hsr_all` / `hsr_large` / `hsr_small`. NH:
  `division_i` / `division_ii` (live: `https://nh.tfrrs.org/indoor_lists.html`, raw lines 374-386).
  Requesting `/directory_tab.html?tab=hsr_all` on NH returns a 2xx page with **no** list links, so
  tab names must be read from the index page's `src` attributes, never hardcoded.

## Path qualification

| Path (state hosts) | Live status 2026-10-12 | Parse evidence | Classification |
|---|---|---|---|
| `/indoor_lists.html`, `/outdoor_lists.html` | 2xx text/html, both Indiana and NH | frame chain only; no list anchors | reachable, **terms-refused** |
| `/directory_tab.html?outdoor={0,1}&tab={instance tab}&year={YYYY}` | 2xx text/html | publishes `/lists/5489/HSR_All_School_Performance_List` (+ `?gender=f`, `?gender=m`) | reachable, **terms-refused**; also NOT a runner-admissible URL - `crates/census-crawl/src/tfrrs/run.rs:62` fails anything that is "not a TFRRS performance-list or team page", so it must be *parsed for links*, never *configured as a page* |
| `/lists/{id}/{slug}` and `?year=`, `?gender=f|m` | 2xx text/html | rows carry athlete/team/event links (see alias shapes below) | **terms-refused**; route parsing already covered by `parse_list_path` tests |
| `/lists/{id}/{slug}/{year}/{season}` (fixture form) | not re-probed today | parses in tests; the live frames now emit the no-suffix form | unverified live, **terms-refused** |
| `/teams/{route}/{Slug}_{m|f}.html` | 2xx (`/teams/tf/Warren_Central_m.html` -> `TFRRS | Warren Central Boys Track & Field Results & Statistics`) | title present; roster links observed by prior lane (`nh_pembroke_xc_team.html`) | **terms-refused** |
| `/athletes/{id}` (+ `/Team/_Name` slug, `+ .html`, www or state host) | 2xx, all four forms | identical `<title>TFRRS | Evan Williams - Track and Field Results & Statistics</title>` | **terms-refused** |
| `/results/{meet_id}/{slug}` and `/results/{meet_id}/{event_id}/{slug}/{Event-Name}` | observed as live row links (e.g. `/results/94244/5842971/Warren_Central_HSR_Qualifier_1/60-Meters`) | link shape verified; pages not fetched today | **terms-refused** |
| `www.tfrrs.org/**` | 2xx | refused by the reader's own host rule (`jurisdiction_of_url` -> `None`, `parse/route.rs:147`; test `tfrrs/parse/tests.rs:221`) | out of scope by design |
| `in.tfrrs.org/**` | 2xx | **same list content as `indiana.tfrrs.org`**, identical athlete ids/row order, links rewritten to the `in.` host; `in` parses as Indiana, so both hostnames are admissible | duplicate content across admissible hosts - pin one canonical host per state or the same 1.9 MB list is fetched twice |

## Identity keys (athlete aliases)

The numeric id is the identity key; the slug is decorative and varies:

- `.../athletes/8429704/Lawrence_Central/_Evan_Williams` (leading `_` = no middle initial)
- `.../athletes/8447751/Mt_Vernon_Fortville/Sebastian_Sprague` (no `_` when the last name is printed first)
- `.../athletes/8853275/Warren_Central/Jr_Thomas_Stokes` (suffix pulled into the slug)
- `.../athletes/9328537/Avon/_Abriana%22_Bri%22_Carlisle` (percent-encoded quoting)
- `.../athletes/8444410/Indianapolis_Bishop_Chatard/Reese__McKinney` (collapsed double space)

Code: `crates/census-crawl/src/tfrrs/parse/route.rs:66` `athlete_id()` parses the token after
`athletes` as `u64` and drops the slug; `map/entity.rs:98` stores `SourceNamespace::TfrrsAthlete`
with that id (`Other("tfrrs_document_row")` when absent). Team keys use `TfrrsTeam` +
`map/entity.rs:47`, sport from the route (`/teams/tf/... vs /teams/xc/...`).

## Unverified

- `florida.tfrrs.org` instance: not probed this session (still "assumed to match Indiana").
- Whether `/directory_tab.html` honours arbitrary/unknown tab ids beyond returning an empty frame.
- Meet/result HTML pages (only their link shape was observed today).
- The FloSports terms page that TFRRS footers link (`https://www.flosports.tv/terms-of-service/`):
  served HTML is a client-rendered app shell (229 lines, no clause text, no `robot|scraper|crawl`
  match in the served markup) - its governing text could not be read with this reader.
