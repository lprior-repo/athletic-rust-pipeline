# DirectAthletics + TFRRS path qualification - 2026-10-12 (bead `athletic-rust-pipeline-3bo`)

Staged section for `research/sources/national-aggregators/SOURCE_REPORT.md` (owner of §1 DAT and
§2 TFRRS). Staged rather than spliced for the same reason as the TFRRS section: this lane's write
tool replaces whole files and cannot append, and re-transcribing a 46 KB report would risk
destroying evidence. Intended placement: appended as §1/§2 qualification, or merged into §18
("Known blocks") and §21 ("Recommendation") of each source.

## DAT: robots.txt is not published

`https://www.directathletics.com/robots.txt` returns the **homepage** (`Content-Type: text/html`,
`#nav`/slideshow markup, 300+ lines) - the platform has no robots.txt at all. There is therefore no
robots-based restriction on DAT paths; the constraint is the Terms of Use alone. (Prior lane had
noted this in passing; re-verified 2026-10-12 by this lane.)

## DAT/TFRRS: the Terms of Use refuse automated acquisition

`https://www.directathletics.com/terms_of_use.html` (200, text/html) is the operator's terms for
both platforms - TFRRS footers read `Copyright (c) 2026 DirectAthletics, Inc.`. Two operative
clauses (verbatim, excerpt retained at
`samples/directathletics-tfrrs/dat-terms-of-use-excerpt.txt`):

> Any commercial or promotional distribution, publishing or exploitation of the DirectAthletics meet
> results, performance lists, roster data, roster names, roster teams, roster years, DirectAthletics
> IDs, TFRRS IDs, site, or any content, code, data or materials on the DirectAthletics site, is
> strictly prohibited unless you have received the express prior written permission from authorized
> personnel of DirectAthletics, Inc. or the otherwise applicable rights holder. Other than as
> expressly allowed herein, you may not download, post, display, publish, copy, reproduce,
> distribute, transmit, modify, perform, broadcast, transfer, create derivative works from, sell or
> otherwise exploit any content, code, data or materials on or available through the DirectAthletics
> web site.

> You agree that you will not use any robot, spider, scraper, or other automated means to access the
> sites for any purpose without our express written permission.

Rendered-text line numbers on the page (this reader): heading `USE OF DIRECTATHLETICS/TFRRS ROSTER
DATA AND MATERIALS` line 19, redistribution paragraph line 21, automated-access paragraph line 25,
`DATA SUBMISSIONS` line 27. `[INFERENCE]` The census as operated is a local, non-redistributed
aggregate, but "automated means ... for any purpose without our express written permission" is
unconditional, and the census has no such permission on file. **Classification: REFUSED (terms) for
every DAT and TFRRS path.** robots.txt does not lift this, and the existing "Known blocks - none
observed (all 200)" lines in §1.18/§2.18 describe transport, not permission - they must not be read
as approval.

## DAT reachability verified 2026-10-12 (this lane)

| URL | Observed | Note |
|---|---|---|
| `https://www.directathletics.com/terms_of_use.html` | 2xx text/html | clause text above |
| `https://www.directathletics.com/robots.txt` | 2xx **text/html** | homepage, not a robots policy |
| `https://www.directathletics.com/teams/track/42070.html` | 2xx | OHSAA Division III team; league/team links (`/leagues/track/1081.html`, `/teams/track/42071.html`, `/teams/xc/25063.html`, `/teams/xc/25064.html`); `Team Roster` empty in the served HTML |
| `https://www.directathletics.com/results/track/96459.html` | 2xx | meet page; event links `/results/track/96459_5955436.html` etc. (event sheet = `/results/track/{meet_id}_{event_id}.html`) |
| `https://www.directathletics.com/athletes/track/8813093.html` | 2xx, **effective URL `https://www.tfrrs.org/athletes/8813093`** | DAT athlete alias 302s to TFRRS and **keeps the numeric id**: DAT ids and TFRRS ids are one namespace |

Representative prior-lane captures in `samples/directathletics-tfrrs/` (`dat-rankings.html`,
`dat-list-track-1428_5490-jr.html`, `dat-results-index-oh-track.html`,
`dat-hs-event-sheet-74867_4545019.html`, `dat-league-track-43-oh.html`,
`dat-fusedriver-upcoming.js`, `indiana-tfrrs-list-5489.html`) were captured 2026-09-22 and are not
re-probed here: the classification above is a permission result, so re-fetching adds nothing.

## Evidence digests (this lane's artifacts)

| File | Bytes (workspace) | sha256 |
|---|---|---|
| `samples/directathletics-tfrrs/dat-terms-of-use-excerpt.txt` | 3278 | **pending in this lane** - no local digest tool, and this lane's reader rejects a `:sha256` path suffix (`Path '...:sha256' not found`); hand-encoding 3 KB into `api.hashify.net` was rejected as error-prone. Reviewer command: `sha256sum samples/directathletics-tfrrs/dat-terms-of-use-excerpt.txt` |

`research/sources/tfrrs/captures/PROVENANCE.md` carries the TFRRS-side table, including the two
98-byte robots bodies whose digests were computed (`2f15987e4606501202b9d337085439339e53a374e3d194847192c12f85ba78f8`)
and the five prior-lane bodies that still need `sha256sum`.

## Unverified

- Whether any DAT path requires auth/registration (not probed; moot under the refusal above).
- Whether `/legacy_da/results?filterrific[...]` still serves 200 today (prior-lane capture only).
- The FloSports terms page that TFRRS footers link serves a client-rendered app shell; its text
  could not be read with this reader.
