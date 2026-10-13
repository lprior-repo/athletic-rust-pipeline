# TFRRS capture provenance (state instances)

Owner: `research/sources/tfrrs/SOURCE_REPORT.md`. This manifest qualifies the durable-path
question for bead `athletic-rust-pipeline-3bo`. All fetches anonymous (no cookies, no auth, no
login); same-host requests spaced >=1s. Reader (this lane) reports the effective URL and
`Content-Type`; it has no metadata oracle for `Content-Length` (probed and unavailable:
`api.allorigins.win` HTTP 522 twice; `api.microlink.io` returns `statusCode: null` with
`meta=false`) and no local digest tool, so byte counts below are the workspace's own file sizes
(exact for files <1KB, rounded by the directory listing above that) and digests are computed only
where recorded. Capture window: **2026-10-12T20:50:47Z -> 2026-10-12T20:59:22Z** (oracle:
`https://timeapi.io/api/Time/current/zone?timeZone=UTC`; `worldtimeapi.org` was unreachable).

Digest semantics: every `sha256` below is the SHA-256 of the file's exact bytes *as written by
this lane* through `api.hashify.net/hash/sha256/hex?value=...` (endpoint validated against the
FIPS `sha256("abc")` vector = `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`).
It is the digest of the retained artifact, not an independent digest of the origin response.
Re-derive with `sha256sum research/sources/tfrrs/captures/<file>`; any mismatch means the file
changed after this manifest was written.

## Artifacts captured by this lane (UTC window above)

| File | URL | Status | Content-Type | Bytes | sha256 |
|---|---|---|---|---|---|
| `robots-tfrrs-www.body` | `https://www.tfrrs.org/robots.txt` | 2xx (reader success) | text/plain | 98 | `2f15987e4606501202b9d337085439339e53a374e3d194847192c12f85ba78f8` |
| `robots-tfrrs-indiana.body` | `https://indiana.tfrrs.org/robots.txt` | 2xx (reader success) | text/plain | 98 | `2f15987e4606501202b9d337085439339e53a374e3d194847192c12f85ba78f8` |

Both robots bodies are byte-identical: the single comment line
`# See https://www.robotstxt.org/robotstxt.html for documentation on how to use the robots.txt file`
followed by one LF (97 characters + LF = 98 bytes, no `User-agent`/`Disallow` records).

Failure-mode evidence for the status column: the reader prints the HTTP code when a fetch fails
(observed in this session on `api.allorigins.win` -> `Failed to fetch URL (HTTP 522)`); a page it
renders therefore returned a 2xx. The exact code and `Content-Length` were not independently
observed - see the digest/byte caveat above.

## Live re-verification performed by this lane (no saved body; statuses as above)

| Path | URL | Observed |
|---|---|---|
| Indoor index | `https://indiana.tfrrs.org/indoor_lists.html` | 2xx, text/html; body carries no `/lists/` anchor - only the `directory_tab_*` turbo-frames (fixture `indiana_directory_tab_hsr_all.excerpt.html`) |
| Discovery frame | `https://indiana.tfrrs.org/directory_tab.html?outdoor=0&tab=hsr_all&year=2026` | 2xx, text/html; publishes `/lists/5489/HSR_All_School_Performance_List` and `?gender=f` / `?gender=m` links |
| NH index | `https://nh.tfrrs.org/indoor_lists.html` | 2xx, text/html; same pattern, tab ids `division_i` / `division_ii` (fixture `nh_indoor_lists_discovery.excerpt.html`) |
| NH frame, IN tab ids | `https://nh.tfrrs.org/directory_tab.html?outdoor=1&tab=hsr_all&year=2026` | 2xx, text/html; **no** `/lists/` link - wrong tab id for that instance |
| Performance list | `https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List` | 2xx, text/html; rows with athlete / team / event links (alias forms: `_Evan_Williams`, `Sebastian_Sprague`, `Jr_Thomas_Stokes`, `_Abriana%22_Bri%22_Carlisle`, `Reese__McKinney`) |
| Same list, short host | `https://in.tfrrs.org/lists/5489/HSR_All_School_Performance_List` | 2xx, text/html; identical athlete ids and row order, links rewritten to `https://in.tfrrs.org/...` (duplicate content across two admissible hostnames) |
| Athlete alias | `https://indiana.tfrrs.org/athletes/8429704/Lawrence_Central/_Evan_Williams` | 2xx; `<title>TFRRS | Evan Williams - Track and Field Results &amp; Statistics</title>` |
| Athlete id only | `https://indiana.tfrrs.org/athletes/8429704` | 2xx; same title |
| Athlete `.html` alias | `https://www.tfrrs.org/athletes/8429704.html` | 2xx; same title |
| Athlete (www) | `https://www.tfrrs.org/athletes/8429704` | 2xx; same title |
| Team page | `https://indiana.tfrrs.org/teams/tf/Warren_Central_m.html` | 2xx; `<title>TFRRS | Warren Central Boys Track &amp; Field Results &amp; Statistics</title>` |

## Bodies captured by the prior lane (2026-10-10; not re-probed by this lane)

URLs as recorded by that lane / implied by the file names on the Indiana instance. Sizes are the
directory listing's display form (rounded for files above 1KB). Digests pending - run `sha256sum`
on each.

| File | URL (as recorded) | Size (listing) | sha256 |
|---|---|---|---|
| `indoor-lists.body` | `https://indiana.tfrrs.org/indoor_lists.html` | 45.0KB | pending |
| `outdoor-lists.body` | `https://indiana.tfrrs.org/outdoor_lists.html` | 43.8KB | pending |
| `list-5489.body` | `https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List` | 1.8MB | pending |
| `team-franklin-central-m.body` | `https://indiana.tfrrs.org/teams/tf/Franklin_Central_m.html` | 78.0KB | pending |
| `athlete-8400835.body` | `https://indiana.tfrrs.org/athletes/8400835` (alias form not recorded) | 349.9KB | pending |

## Fixtures added to `crates/census-crawl/tests/fixtures/tfrrs/`

| File | Source | Bytes (write report, exact) | sha256 |
|---|---|---|---|
| `indiana_directory_tab_hsr_all.excerpt.html` | discovery frame above, raw lines 401-426 verbatim | 1919 | pending |
| `nh_indoor_lists_discovery.excerpt.html` | `https://nh.tfrrs.org/indoor_lists.html`, raw lines 374-386 verbatim | 1046 | pending |
| `tfrrs_athlete_alias_identity.excerpt.html` | alias athlete page, raw lines 47-51 verbatim | 1137 | pending |

Housekeeping: `.append-probe` and `.append-probe:append` (0 bytes each) are inert probes written
while establishing that this lane's write tool replaces whole files and cannot append or delete;
safe to remove.
