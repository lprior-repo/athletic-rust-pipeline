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

## Registered scope

NC/NCHSAA, AL/AHSAA, AR/ArkAA, GA/GHSA, MT/MHSA, MS/MHSAA, SC/SCHSL, ID/IdHSAA,
NM/NMAA, MD/MPSSAA, DC/DCSAA, DE/DIAA, TN/TSSAA, WY/WHSAA, ND/NDHSAA.

Robots policy is not published (403 with API Gateway body). Pacing follows the
registry's 1 request/s with one in-flight request.