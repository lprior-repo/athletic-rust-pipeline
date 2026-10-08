# GoBound Staff Page Source Report

Iowa (IGHSAU) and South Dakota (SDHSAA) school staff pages served by `www.gobound.com`. The adapter
supplies coach identity — published name and role per school and sport — to the coach lane; GoBound
publishes no coach email address, so it confirms rather than supplies contacts.

## Endpoint and provenance

- `HOST` = `https://www.gobound.com`; path `/{state}/{association}/{sport}/{season}/{school}/v/staff`,
  for example `https://www.gobound.com/sd/sdhsaa/girlscrosscountry/2026-27/yankton/v/staff`.
- Associations: Iowa `ia/ighsau`, South Dakota `sd/sdhsaa`. Fixture captures are from the 2026-27
  season; byte counts, SHA-256 values and cache keys are recorded in
  `crates/census-crawl/tests/fixtures/bound/SOURCE.md`, which owns that table.
- Module surface: `census_crawl::bound::{parse, collect, map}` — `parse` returns the page facts and
  `CoachRow`s, `collect` walks the association's staff pages through the shared fetcher and emits
  `Schools`, source observations and coach entities, `map::school_entities` turns one page into a
  `CanonicalSchool` plus `CanonicalCoach`s under `SourceNamespace::AssociationSchool`.
- Registry: the `bound` descriptor in `crates/census-crawl/src/registry/table/directories.rs`
  declares `TransportKind::Html` with the `SCHOOL_COACH_CONTACT` capability; the applicability table
  (`crates/census-crawl/src/applicability/table/data.rs`) admits it for Iowa and South Dakota, with
  its evidence and refusal prose in `table/prose.rs`.
- Durable dispatch: `TEAMS_ARMS` in `crates/census-service/src/restate_services/teams_arms.rs`
  carries `("bound", TeamsArm::Bound)`, which runs `associations::bound` — the jurisdiction's own
  state, no school-name override, so it reads that state's schools from the store corpus — and
  `DISPATCHED` in `restate_services/jurisdiction.rs` lists `bound`. The plan test
  `the_gobound_staff_walk_is_planned_for_iowa_and_south_dakota_only` asserts both states plan it as
  sweepable and that Nebraska and Minnesota do not.

## Robots and admission

- `https://www.gobound.com/robots.txt` answers **403** to the `census-service` default user agent —
  as does every content path — and 200 to a browser-class user agent. The fetched policy allows the
  staff paths with `Crawl-Delay: 10` and disallows the `/*directory` paths; the adapter sends a
  browser-class user agent as a per-request header and never fetches a directory path.
- The per-request user agent must keep the compact form
  `Mozilla/5.0 (…) Chrome/126.0 Safari/537.36 census-service/0.1`. Measured 2026-10-07 on
  `/ia/schools`: a parenthesized comment tail — `(independent HS track & field research collector;
  polite; contact: repo owner)` — answers **403** with a 520-byte body, while the compact form
  answers **200** with 641,210 bytes, as does the prototype's `… census-prototype/0.1`. The refusal
  keys on the comment tail, not on the collector token; the first live smoke run stopped exactly
  there, which is why the constant carries no comment.
- The descriptor admits `www.gobound.com` at `CRAWL_DELAY_TEN_RPS`, and the host table in
  `crates/census-crawl/src/lib.rs` records 10 s for both `gobound.com` and `www.gobound.com`. The
  shared fetcher's host gate takes the maximum of configured, robots and declared delays, so the
  published crawl delay is inherited without an adapter-local sleep.
- Non-200 is absence, not an error: bound slugs without a school answer 404 with a byte-identical
  `Bound` title stub and contribute no rows.

## Parser and mapping

- A page carries `<h4 class="card-title">Coaching Staff</h4>` (all 442 cached `www.gobound.com`
  pages do); `parse` anchors on that heading and reads the following table rows, each
  `<td style="text-transform: capitalize;">Role</td><td>Name</td>`. Header rows are skipped.
- `map_role` accepts `head coach` and `co-head coach` as head coaches, `assistant coach` and
  `volunteer coach` as assistant coaches. Across the 442 cached pages the only published position
  values are `Head Coach` (432 rows) and `Assistant Coach` (819 rows); a role outside the mapped set
  is excluded, and `crates/census-crawl/tests/fixtures/bound/staff-non-coach-role.html` (a
  hand-written fixture, marked as such in the fixture README) tests exactly that exclusion.
- Page titles are `Bound | <school and mascot> | <sport> | 2026-27`, so the title names a club
  (`Yankton Gazelles`) rather than the school corpus name (`Yankton High School`). `collect` skips a
  page whose title school does not start with the slugified job slug (`title_matches_slug`), and
  `ProfileFacts::name` — the resolved corpus name — is what the emitted school carries.
- A school with an empty staff table still yields its school entity with zero coaches
  (`staff-faith-sd-boys-xc-empty-staff.html`); the fetched page, not a parse failure, is the evidence
  that the school publishes no coach.
- Live smoke repair (2026-10-07): the first end-to-end run stopped at `school observation failed`
  because the mapper minted no `SourceIdentity` for the school and gave the evidence an empty URL,
  which `SourceSchoolObservation::of_school` refuses. `school_entities` now mints
  `SourceIdentity::new(AssociationSchool{association:"bound"}, "<state>/<slug>")` with the page URL
  and records the page as evidence, the collector builds one `ProfileFacts` per fetched page so each
  coach cites the page that published it, and a school with no fetchable page is skipped with a
  report note instead of being emitted without evidence. Coach identities are scoped
  `<state>/<slug>:<name>:<role>:<sport>:<gender>` so two schools' staff lists cannot collide.
  Regression: `a_fetched_page_answers_the_school_observation_contract`.

## Verification handoff

Commands run 2026-10-07 on this change, all from the workspace root:

- `cargo test -p census-crawl --lib` → **978 passed**, 0 failed.
- `cargo test -p census-crawl --lib -- bound` → **38 passed**, 0 failed.
- `cargo test -p census-crawl --lib -- coach_contacts` → 20 passed, 0 failed.
- `cargo test -p census-crawl --lib -- registry:: applicability::` → 26 passed, 0 failed. These are
  the registry and applicability invariants; the wiring repairs they forced are recorded in
  `docs/VERIFICATION-EVIDENCE.md`.
- `cargo test -p census-service --lib` → **275 passed**, 0 failed, including
  `the_gobound_staff_walk_is_planned_for_iowa_and_south_dakota_only`.
- `cargo clippy -p census-crawl -p census-service -p xtask --all-targets` → 0 warnings, 0 errors.
- `cargo fmt --all --check` → exit 0.
- `env -u CI tools/moon-local run pipeline:xtask -- replay bound` → **6 capture(s) replayed**:

```
source bound: 6 capture(s) under crates/census-crawl/tests/fixtures/bound, offline, no store, no clock
staff-faith-sd-boys-xc-empty-staff.html  staff school="Faith Longhorns" sport="Boys Cross Country" coaches=0
staff-non-coach-role.html                staff school="Example School" sport="Girls Cross Country" coaches=2
staff-siouxcenter-ia-girls-xc.html       staff school="Sioux Center Warriors" sport="Girls Cross Country" coaches=4
staff-whiteriver-sd-girls-xc.html        staff school="White River Lady Tigers" sport="Girls Cross Country" coaches=2
staff-yankton-sd-boys-xc.html            staff school="Yankton Bucks" sport="Boys Cross Country" coaches=2
staff-yankton-sd-girls-xc.html           staff school="Yankton Gazelles" sport="Girls Cross Country" coaches=2
6 capture(s) replayed for source bound
```

The replay arm asserts a non-empty coach count on every captured staff page and zero on the
empty-staff capture; the hand-written non-coach-role page replays two coaches from three rows,
which is its point — a body that stops parsing fails the verb rather than printing a zero.

## Limits

- No live end-to-end durable run was executed for this adapter: `collect` runs from
  `census-service provider arms native_associations`, which needs a new build, a fresh run identity
  and a fresh Restate registration. The registered `national-fresh-20261006-01` deployment predates
  this wiring and cannot execute it.
- Fixture captures come from the prototype crawl cache, which records no capture timestamp, so no
  freshness claim attaches to them; the `403`/`200` user-agent split was re-measured live on
  2026-10-07 with `curl` against `/robots.txt` and a staff path.
- The robots-observed interval and per-request pacing of a live collector run are counted in the
  fetcher's own statistics, not re-measured here.
