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
A season token in a team label (`2024-2025`, `2024-25`, `2024/25`, `2024`,
optionally prefixed `season` or `sy`) is read as that listing's published season
scope and ignored for label matching.

Role from title: "head coach" → HeadCoach, "assistant coach" → AssistantCoach,
otherwise Unknown. A published team assignment does not invent a head/assistant
role. Titles containing "athletic director" become AthleticDirector rows with no
sport or gender, whether or not they also say "former".

A title containing "former" states a **past** appointment, never a current one.
It keeps the role its title names — so the row resolves to the same
sport/role/gender appointment identity as the corresponding current listing —
and carries `CoachTenure::Former` evidence on that identity. A later former
listing therefore merges with, and withholds, the earlier current claim instead
of minting a replacement person: the contact projection reports a tenure
conflict and publishes no mailbox. Tenure statements are page-bound tenure
evidence: the mapper never rewrites the published title, and an explicit
published season scope is parsed from the title first and the team label second.
A stated season that is not the run season is retained as
`CoachTenure::Former { last_school_year: <stated> }`, so it cannot assert a
current appointment; a stated future season is retained the same way and the
tenure assessment ignores it, leaving the identity Unknown. Only a listing with
no stated season follows ADR-024 and takes the run school year as its current
claim. Every emitted claim is validated before it is returned.

Three passes: team `coachProfileIds` first, then unplaced staff with a `teamName`, then directors.
All published records for an ID reach admission; no earlier eligible record is overwritten before
the level/name/vendor filters. Reverse publication order gives the latest **admissible** record
precedence within each sport/role/gender context. Admitted known-ID contexts deduplicate by that
provider ID; missing IDs retain separate published members rather than inferring identity by name.
A rejected row does not reserve an admitted context or mark the member placed. `amrId` becomes
the row's `SourceIdentity` under `association_school`, composed with sport family, role and gender.
Contact extraction uses `emails[0]`, `tel[0].num`, and `amrId`.

The requested association route is not a school's published geographic state. Directory
admission still requires its parsed `stateCode` to equal the requested jurisdiction before
school, postal or summary facts can be attached. Refusals distinguish recognized foreign
state, unrecognized published state and missing/unusable state, retaining the requested and
published jurisdiction, exact short code, capture URL/date and SHA-256. They do not establish
association non-membership or authorize source-value corrections. Raw evidence remains archived.
The collection summary's `with_email` count means published email addresses, not postal-address
coverage; the separate survey `with_address` field retains its historical meaning.

## Census scope at emission

Census emission runs the prototype's row hygiene (`row_hygiene`): names and
school labels collapse every whitespace run (including NBSP), a person label
loses a trailing post and is dropped when nothing but a post, a non-coach lead or
a `Dean` lead remains, and a school name matching `\btest\s+school\b` or a coach
address ending `@dragonflyathletics.com` is dropped as a vendor fixture. The
level scope runs first, then name hygiene and the vendor drop, before admitted
row de-duplication. Rejected contexts have a separate diagnostic ledger keyed
by staff member, sport, gender, role and normalized level; repeat rejections
count once per context without hiding a later eligible row. Probe scope runs
neither hygiene nor level filtering. `collect` reports the dropped rows in its
notes: per-level, vendor, person and directory-row counts.

## Page-bound appointment evidence (ADR-024)

Production `coach_entities` requires the census run's `SchoolYear` and the retained
summary capture's SHA256 and RFC3339 retrieval time. The collector supplies
`AdapterContext.school_year` and the summary `FetchOutcome` metadata, never a year
inferred from the observation date or a digest manufactured by the mapper.

A listed head/assistant coach with a named team emits `CoachTenure::Current` for
that run season. Team-index placement keeps the source team's label; unplaced staff
keeps its own `teamName`. A listed Athletic Director uses the source school's name
and school athletics as its program. Unknown/former roles and coaching roles without
a team emit no tenure evidence. Name-only appointments may have current tenure but
have no mailbox claim.

Each evidence record includes the capture URL/digest/time and a statement containing
the source role and team/program labels verbatim. A `CoachContactClaim` binds the minted
coach ID, school ID, role, sport/gender or SchoolAthletics, and only the mailbox this
same listing emitted. Missing mailboxes remain `None`; other captures' addresses are
never borrowed. Domain validation checks the constructed evidence; malformed capture
metadata and statements exceeding 512 bytes are explicit mapping errors. Labels are
never truncated to fit.

The separately named `probe_coach_entities` has no census season and cannot emit
tenure evidence. Its diagnostic counts retain their existing scope. Focused regressions
exercise both pure mapping and the real offline collector/cache/store path, including a
run season different from the retained timestamp.

Run the focused suite with `cargo test -p census-crawl --lib coach_directories`.
Recruiter contact publication is a separate report-layer acceptance (bead `0hx`);
directory appointment emission alone does not prove that a mailbox publishes.

## Parity with the prototype

Both lanes are compared against the prototype's own parsers, executed over the
captured bodies:

* `research/sources/coach-directories-national/dragonfly-directory-parity.md` —
  3,693 directory rows, 0 field divergences, the NC/AK/WY pages pinned by
  `golden_directory_rows.json`.
* `research/sources/coach-directories-national/dragonfly-summary-parity.md` records the historical 18-summary prototype comparison. `golden_summary_rows.json` preserves that evidence, not the current admission contract. The captured WY SS28UB regression, `tests::summary_parity::captured_summary_keeps_four_varsity_contexts_and_counts_four_jv_rejections`, checks all four varsity sport/gender contexts and all four distinct JV rejections. Prototype claim-before-filter behavior is deliberately not retained.

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

**Verb (frozen name, wired 2026-10-05).** Per ADR-015 the probe lands as a
`census-service` verb rather than a second binary in this crate:
`survey --states <list> [--offline] --out out/dragonfly_probe.json`, calling
`coach_directories::{parse_state_filter, selected_associations, survey,
table_line, report_json}`. The verb lives in `census-service/src/cli/survey.rs`
and paces through the same registry-bound fetcher as every provider arm; without
`--offline` it probes live, with it a replayed run decides from retained bytes.

## Registered scope

NC/NCHSAA, AL/AHSAA, AR/ArkAA, GA/GHSA, MT/MHSA, MS/MHSAA, SC/SCHSL, ID/IdHSAA,
NM/NMAA, MD/MPSSAA, DC/DCSAA, DE/DIAA, TN/TSSAA, WY/WHSAA, ND/NDHSAA.

CIAC/CT is refused: the directory publishes 1,302 rows with addresses, but every
sampled summary carries an empty `staff` array and zero `totalCoachCount` per
team (measured 2026-10-05, `docs/VERIFICATION-EVIDENCE.md`), so the association
offers this adapter no contact material — CT coach contacts come from the
separate `ciac` adapter over `ciac.fpsports.org`.

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

## School mailbox and contact research publication

Generic school-office and athletics-office mailboxes are separate school-owned
claims, never substitute coach or athletic-director identities, and may use a
personal-provider address when the publisher explicitly designates its office
purpose. Acquisition reads the school's declared website and bounded,
same-origin contact links actually present in captured pages; it does not guess
contact routes, people or addresses.

Each admitted claim retains its exact capture URL, lowercase SHA-256, acquisition
timestamp, publisher statement, school owner and purpose. Selection checks the
capture's academic year, not merely the requested year stamped onto a crawl.
Stale captures remain retained but cannot become current by replaying an old
cache under a new requested year; a later eligible capture can replace that
stale evidence. Independently published current office addresses conflict
rather than silently choosing one.

Contact research persists for school athletics and each boys'/girls' XC,
indoor-track and outdoor-track program even when a summary contains no coaches.
Unattempted, completed-empty, completed-claims, partial, blocked, failed,
exhausted, stale, ambiguous and conflict remain distinct. Attempts retain the
actual locator, acquisition time, available capture digest, outcome and reason.
Retries supersede only the same locator: a successful source cannot hide another
blocked source. Dropped rows or incomplete parsed appointments remain partial.
Access refusals remain blocked; transport and decode failures remain failed.

Research subjects distinguish each team program from school-office and
athletics-office mailbox discovery. A blocked AD directory cannot change the
office purpose's completed state, even when both observations share a URL.
Nonterminal row assessments remain owed despite empty child attempts; newer
retries supersede only their own source. An audited exhausted directory cannot
erase a separately admitted completed-empty capture or itself certify one.

Yearless staff titles inherit the academic year of the physical capture, not
the requested collection year. Journal completion keys include the requested
year, so an older run cannot skip a newer school contact obligation.

Exhaustion is certified only by the final captured page of an unfiltered,
unlimited finite state-directory walk with complete rows and no failed,
rejected or unresolved school summary. It is attached only to schools processed
in that walk whose source-local program research is terminal. A directory
frontier is research evidence, never mailbox or appointment provenance.

Workbook publication uses separate School Contacts and Contact Research sheets
and matching `school-contacts.csv` and `contact-research.csv` sidecars. Selected
coach and preferred-athlete contact cells carry the exact independently chosen
mailbox capture; archival canonical-coach CSV source fields are explicitly named
archival fields rather than presented as selected contact provenance.

The offline CAC fixture is a verbatim contact-paragraph and owner-metadata
extract from `https://cacmustangs.org/about/contact/`, not the full captured
page. Its tests compute the digest of those fixture bytes and use a controlled
acquisition timestamp. Gmail, stale-capture and malformed-content cases, named
coach/AD records in the three-mailbox publication scenario, and the prior-year
AK cache clock are controlled variants, not live publisher observations.
The concurrent repair wave prohibited commands, so these added tests
and publication scenarios are unexecuted until Main runs the acceptance lanes;
this documentation does not certify their results.

## Recording-aware acquisition closure (2026-10-07)

The school-site canonical collector persists the seven team/program research
subjects and two independently assessed office purposes, including acquisitions
that publish no named staff. Each physical page contributes only its new claims
and research rows through `AdapterContext.write_batch`; the effect and journal
association remain recorded rather than writing the store directly when a native
recording is active. Later page or retained-state limits do not discard admitted
earlier page prefixes. Invalid capture timestamps cannot certify completed-empty
research merely because their first ten bytes look like a current date.

SIDEARM native discovery consumes at most 64 source-discovered canonical schools
per call through `collect_discovered`. Only published school/athletics website
seeds and same-origin published staff/directory links are used; the collector
does not synthesize routes or replace those owners with the fixed gomats fixture.
Unmatched provider pages and deferred endpoints remain unfinished. Exact
canonical names or canonical aliases are required before staff is attached, and
the existing located canonical school identity is preserved. Physical projection
keys omit requested school year and performance horizon; per-year research has
its own recording-aware effect. UH, PIAA and TSSAA use the same separate staff
research persistence.

The three-purpose scenario's named athletic director and mailbox are taken from
the captured NC ZCUM49 summary. Its two office paragraphs and 2026 acquisition
clock are controlled test inputs, not additional live publication evidence.
The added service and publication scenarios remain unexecuted during concurrent
edits. Main owns the integrated compiler, runtime, test and source-gate evidence;
this handoff certifies none of those lanes.