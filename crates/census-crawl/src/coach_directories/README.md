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
prefix matching: `Boys'`/`Boy's` → Boys, `Girls'`/`Girl's` → Girls,
`Unified ` and `Mixed ` stripped (gender stays unset). Role from title: "head
coach" → HeadCoach, "assistant coach" → AssistantCoach, otherwise Coach.
Directors (title contains "athletic director") become AthleticDirector rows with
no sport or gender.

Three passes over the staff index: team `coachProfileIds` first, then unplaced
staff with a `teamName`, then directors. Dedup key is
`(person, sport family, role, gender)`, with the person taken from `amrId` when
present and from the name otherwise, and the sport family collapsing indoor and
outdoor Track into `Track` while Cross Country stays its own family: a person
listed for both an indoor and an outdoor team of one role and gender emits a
single row, and the first occurrence's concrete sport is the one recorded.
First occurrence wins. Contact extraction uses `emails[0]`, `tel[0].num`, and
`amrId`.

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
by state and serializes them; a fetch or decode failure inside `probe_one`
becomes a record whose `status` names the failure instead of an error, the way
the prototype records it.

The probe is a qualification tool, never a census path: it writes no store rows.
Its fixtures under `tests/fixtures/coach_directories/probe/` hold the captured
directory and summary bodies plus the prototype's own record for AK, AL, WY and
GA, and `survey_tests.rs` asserts the Rust record equals that prototype record
field-for-field through an offline fetcher.

## Registered scope

NC/NCHSAA, AL/AHSAA, AR/ArkAA, GA/GHSA, MT/MHSA, MS/MHSAA, SC/SCHSL, ID/IdHSAA,
NM/NMAA, MD/MPSSAA, DC/DCSAA, DE/DIAA, TN/TSSAA, WY/WHSAA, ND/NDHSAA.

Robots policy is not published (403 with API Gateway body). Pacing follows the
registry's 1 request/s with one in-flight request.