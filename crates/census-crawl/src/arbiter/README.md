# Arbiter organisation adapter

Associations that delegate their member directory to Arbiter embed `live.arbiter.io/org/<id>`.
The organisation API behind the embed carries member schools and their coach rows, which is the
largest measured yield of the port queue (2,932 coach rows across the four records). The adapter walks it end to end: mint a token, page the member list, then read one
coach page per school.

## Hosts and robots

- `live.arbiter.io/directory/assets/index-<hash>.js` — the public SPA bundle. The client
  credentials the embed mints its token with are published inside it as
  `client_id:"…",client_secret:"…"`. `robots.txt` answers the SPA shell with no directives.
- `token.arbitersports.com/connect/token` — OpenID token endpoint, `POST` form
  `client_id`/`client_secret`/`grant_type=client_credentials`/`scope=Registration`, answers
  `{access_token, expires_in, token_type}`. `robots.txt` answers `404`, i.e. allow-all.
- `services.arbitersports.com/api/v2` — the gateway. Every path answers `401` without a Bearer
  token, and `robots.txt` answers `404`, i.e. allow-all.

Association-hosted Arbiter sites such as `ossaa.arbitersports.com` refuse every user agent and are
not read: the lane uses the API hosts only.

## Endpoints

- Members: `GET /organization/public/{org}/children?&pageSize=200&pageNumber={n}`. Returns
  `{data:{total, rows[]}}`; a row carries `publicId`, internal `id`, `name`,
  `phoneNumber`, `enrollmentCount`, and an embedded `primaryContact` with `firstName`,
  `lastName`, `roleName`.
- Coaches: `GET /legacy/public/{org}/coaches?filter.EntityId={publicId}&pageSize=200&pageNumber={n}`
  for one school, or without the filter for the whole organisation. A row carries `firstName`,
  `lastName`, `coachPositionName` (`Head Coach`, `Assistant Coach`, `Associate Head Coach`, …),
  `sportName` (`Cross Country, Boys`, `Track & Field - Indoor, Girls`, …), `levelName`
  (`Boys Varsity`, `Coed Middle School`, …) and `roleName`.

Both endpoints paginate on `pageNumber`; the walk ends on the first page that returns fewer rows
than `pageSize` (an empty page included), not on the offset against `total`. A page that is full but
answers without a usable `total` therefore keeps the walk going instead of stopping early, and a
short page whose response still reports more rows than have been read — the server contradicting
itself, which the walk detects on the rows actually received rather than on the page window — is
recorded as a failure note rather than passing as the end of the list. Each member page is mapped
and journaled as it is read, so the walk's working set is one page rather than the whole member
list. A member walk, or a per-school coach walk, stopped by the 64-page bound records a failure
note too, so no truncation is silent. A state named twice in the `--states` list is walked once.

## Organisations

| Organisation | State | Member schools |
|---|---|---|
| NHIAA, org `2132` | New Hampshire | 89 |
| KHSAA, org `2507` | Kentucky | 489 |
| MHSA, org `4497` | Montana | 215 |
| WVSSAC, org `4223` | West Virginia | 269 |

Org `106940` was the queue's Oklahoma candidate and answers an empty member page (re-checked
2026-09-29), so Oklahoma stays uncovered by this lane.

## Mapping rules

Only Cross Country and Track sports become rows; a sport label carrying `ski` is dropped before
the shared label table sees it (Arbiter's `Cross Country Skiing` would otherwise read as Cross
Country). The label table is the measured one the prototype's exact-key table was replaced with,
so `Track & Field - Indoor` and `Track & Field - Outdoor` map to indoor and outdoor Track instead
of being dropped.

The side of a coach comes from the sport label when it states one and from the level label
otherwise: NHIAA writes `Cross Country, Boys` for some rows and `Boys Varsity` for the rest, so
both are read and a row that states neither becomes `Gender::Mixed`.

Role comes from the position label through the shared table, which keeps head and assistant coaches
and drops the non-coaching titles (`Volunteer Coach`, `Secretary`, …) the API also returns. The
table matches on substrings, so `Associate Head Coach` — one NHIAA row, which the prototype's
exact-key role map drops — is kept as `HeadCoach`.
Athletic directors are minted only from the member row's `primaryContact`, with no sport, because
the coach endpoint does not carry them; a contact whose role name is not a coach role is dropped
rather than minted as a director, and the member row itself is still written (see below).

A row whose person name is empty, whose position is not an identifiable coaching role, whose sport
is outside Cross Country and Track, or whose level label states a division other than varsity (`Coed
Unified`, `Coed Middle School`, `Boys JV` — matched as a substring, so `Varsity` and `- Varsity`
both count) is dropped without a coach row and without an absence claim: the endpoint is partial
across sports by design, so a missing row is not evidence of no coach.

Every member row with a name is written as a canonical school plus a source observation keyed by
the source namespace; the contact only decides whether an athletic-director coach row is written
alongside it. The journal key is the school id, and a run reads the journal before it walks: a
school the journal already names is skipped without fetching its coach page, so a re-run continues
with the schools it has not written and reports how many it skipped. The journal detail carries the
organisation, the state's two-letter code, the association id, the organisation's own school id and
the coach-row count.

A member page that fails or does not parse, and a coach page that fails, are recorded as failures
and end only that walk — the remaining organisations in `--states` still run — while a failure to
read the bundle or mint a token ends the run, because no organisation can be read without one.

## Fixtures

`crates/census-crawl/tests/fixtures/arbiter/` holds the captures, the prototype goldens and the
redacted credential fixtures; `PROVENANCE.json` names the capture hash, URL and byte size of each
and records the prototype-run checks (`NH-arbiter.jsonl` schools and contacts are byte-equal to
the goldens after JSON decode). `tests/fixtures/arbiter/golden_alvirne_coaches.json` is the
prototype's two-row Alvirne result, and `golden_nh_all_coaches_p11.json` is its empty result over
the org-wide page that carries 137 eligible track rows — the divergence the fixtures pin.
