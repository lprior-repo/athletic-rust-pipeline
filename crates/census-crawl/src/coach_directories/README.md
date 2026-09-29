# DragonFly Coach Directories

Adapter for the 15 continental high-school associations that publish staff on
DragonFly Athletics. The other 34 continental associations on the platform
answer no staff and are out of scope.

## Endpoints

- Directory: `GET https://maxinfosite-api-live.dragonflyathletics.com/states/<ruleset>/directory/<page>`
  Returns `{currentPage, totalPages, totalResults, results[]}` with one page of
  school rows. Each row carries `orgId`, `shortCode`, name, city, state, address,
  and a free-key `competitionLevels` object (can be absent).

- Summary: `GET https://maxinfosite-api-live.dragonflyathletics.com/schools/<shortCode>/summary`
  Returns school identity plus `staff` and `teams` arrays. Keyed by `shortCode`;
  a request by `orgId` answers HTTP 200 with an S3 `AccessDenied` XML body, not
  a JSON school.

## Mapping rules

Only Cross Country and Track teams are emitted. Team labels are parsed with
prefix matching: `Boys'`/`Boy's` → Boys, `Girls'`/`Girl's` → Girls, one
`Unified ` or `Mixed ` prefix stripped (gender stays unset, and a second
genderless prefix leaves no sport — the prototype breaks after the first match).
Role from title: "head coach" → HeadCoach, "assistant coach" → AssistantCoach,
otherwise Coach.
Directors (title contains "athletic director") become AthleticDirector rows with
no sport or gender.

Three passes over the staff index: team `coachProfileIds` first, then unplaced
staff with a `teamName`, then directors. The claim key is
`(person, sport family, role, gender)` with the person joined from the trimmed
`firstName`/`lastName` parts (the prototype joins the parts untrimmed, so a padded
name would key differently; no captured summary pads one), and the sport
family collapsing indoor and outdoor Track into
`Track` while Cross Country stays its own family: a person listed for both an
indoor and an outdoor team of one role and gender emits a single row, and the
first occurrence's concrete sport is the one recorded. First occurrence wins and
the claim is taken before any drop rule, exactly as the prototype's parser
de-duplicates. `amrId` is not part of the key: it becomes the row's
`SourceIdentity` under `association_school`, composed with the sport family, role
and gender so two rows of one person stay distinguishable. Contact extraction
uses `emails[0]`, `tel[0].num`, and `amrId`.

## Census scope at emission

Census emission runs the prototype's row hygiene (`row_hygiene`): names and
school labels collapse every whitespace run (including NBSP), a person label
loses a trailing post and is dropped when nothing but a post, a non-coach lead or
a `Dean` lead remains, and a school name matching `\btest\s+school\b` or a coach
address ending `@dragonflyathletics.com` is dropped as a vendor fixture. The
level scope runs first, after the claim, then the name hygiene and the vendor
drop — the prototype's order, whose level filter runs at merge before either
hygiene rule, so a non-varsity row is always counted as a level drop. Because the
claim precedes those rules, the row that decides a key is the first one published
for it: a school that lists a junior-varsity team before the varsity team of the
same person, sport family, role and gender keeps the junior-varsity row and drops
it by level, and the later varsity row never replaces it. A later sub-varsity row
whose key is already claimed is absorbed silently and is not counted. Probe scope
runs neither hygiene nor the level scope, so a nameless row the prototype's probe
counts is kept there and dropped only on the census path. `collect` reports the
dropped rows in its notes: per-level, vendor, person and directory-row counts.

## Parity with the prototype

Both lanes are compared against the prototype's own parsers, executed over the
captured bodies:

* `research/sources/coach-directories-national/dragonfly-directory-parity.md` —
  3,693 directory rows, 0 field divergences, the NC/AK/WY pages pinned by
  `golden_directory_rows.json`.
* `research/sources/coach-directories-national/dragonfly-summary-parity.md` —
  18 summaries, 48 kept rows, 0 row/field divergences after the prototype's
  gender collapse, 15 sub-varsity drops, pinned by `golden_summary_rows.json`.
  That comparison found the claim-order defect described above; the exact row set
  for the affected school is pinned by
  `tests::a_live_coach_row_is_decided_by_the_first_team_that_publishes_the_key`.

## Survey (qualification probe)

`survey.rs` carries the association-availability probe that produced the source
report's 51-ruleset table. `ASSOCIATIONS` is the jurisdiction-to-ruleset map for
all 51 continental jurisdictions, `VERIFIED` the 15 associations whose summaries
publish staff, and `survey` walks the list while `probe_one` reads exactly one
association: directory page 1, then the four sampled schools' summaries.
Sampling follows the prototype: rows with `competitionLevels` are preferred and
stepped by `max(1, len / 4)`, taking at most four, and every fetch beyond the
directory is a summary keyed by `shortCode` (a row without one is skipped).

`ProbeRecord` is the probe's output and keeps the prototype's JSON shape:
`state`, `ruleset`, `status`, `schools`, `with_address`, `pages`,
`directory_total`, `sampled`, `staff`, `coaches`, sports counts keyed `Track` /
`CrossCountry` / `AthleticDirector`, and `staff_per_school` /
`coaches_per_school` rounded half-even to one decimal. `with_address` counts
truthy addresses, so an empty string does not count. `report_json` sorts records
by state and serializes them with the prototype's one-space indent, so its output
is byte-identical to `out/dragonfly_probe.json` for the same records; the sports
counts are insertion-ordered for that reason, not sorted. A fetch or decode
failure inside `probe_one` becomes a record whose `status` names the failure kind
instead of an error, with the message truncated to 200 characters, the way the
prototype records it.

The verb's argument handling is in the module, not in the (unwired) CLI:
`parse_state_filter` reads the `--states` list (comma separated, trimmed,
upper-cased; empty means every association) and `selected_associations` resolves
it against `ASSOCIATIONS`; `table_line` renders one record exactly as the
prototype's `main` prints it, including the Python-style sports mapping.
`--offline` is the fetcher's `with_offline(true)`, which serves the crawl cache
and fails with `FetchError::Offline` on a miss, so a replayed run decides from
retained bytes only.

The probe is a qualification tool, never a census path: it writes no store rows.
Its fixtures under `tests/fixtures/coach_directories/probe/` hold the captured
directory and summary bodies, the prototype's own record for AK, AL, WY and GA,
and the prototype's own artifacts for all 51 associations
(`dragonfly_probe_records.json` and the printed table in
`dragonfly_probe_table.txt`). `survey_tests.rs` asserts the Rust record equals
the prototype record field-for-field through an offline fetcher, that all 51
table lines and the report bytes match the prototype's, and that the state filter
resolves as the prototype's `--states` does.

**Verb (frozen name, unwired at this base).** Per ADR-015 the probe lands as a
`census-service` verb rather than a second binary in this crate:
`survey --states <list> --offline --out out/dragonfly_probe.json`, calling
`coach_directories::{parse_state_filter, selected_associations, survey,
table_line, report_json}`. `census-service` does not compile at this base, so the
verb is wired with the first green build.

## Registered scope

NC/NCHSAA, AL/AHSAA, AR/ArkAA, GA/GHSA, MT/MHSA, MS/MHSAA, SC/SCHSL, ID/IdHSAA,
NM/NMAA, MD/MPSSAA, DC/DCSAA, DE/DIAA, TN/TSSAA, WY/WHSAA, ND/NDHSAA.

Robots policy is not published (403 with API Gateway body). Pacing follows the
registry's 1 request/s with one in-flight request, and the fetcher enforces it
rather than trusting its caller:

* `net::host_gate` floors a host's pace at the rate its registry row declares
  (`registry::declared_delay_for_host`), so a shorter `--delay-ms` or a test
  default cannot exceed the published rate; the host's robots crawl-delay and the
  500 ms authorized floor still apply, whichever is slowest.
* A host inside a recorded access cooldown (a 403/429 that minted a
  `SourceAccessCondition`) refuses the request with `FetchError::Policy` instead
  of continuing to knock, so a rate-limited host is left alone for the recorded
  `Retry-After`/six-hour window.
* `robots.txt` is read once per origin under the same host gate and paced like
  any other request, so the first page request does not land in the same second
  as the policy probe.
* Pacing state (turn slots and the one-in-flight gates) belongs to the `Fetcher`;
  instances that share a host must share it with `Fetcher::with_shared_pacing`,
  which the service's per-jurisdiction fetchers must do when they are wired.
* Only a school whose summary was actually read is journalled, so a transient
  failure is retried on the next run instead of being fixed as "done without
  coaches"; a directory row carrying no short code is counted as a dropped row
  rather than disappearing from the report.
* The probe reads the cache first, as the prototype does, and its record has no
  `from_cache` field: re-running the survey over a warm cache replays retained
  bodies as if they were measured, so a fresh qualification number needs
  `--offline`-free replay over a cleared cache directory.