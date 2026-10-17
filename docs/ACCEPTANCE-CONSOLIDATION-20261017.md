# Acceptance consolidation — October 17, 2026

Base: source `50b02cf25534acd25b5fcd79b3229ebebb66253a`, identical tree to
merged main `a2f3979ed0b6a7b7d430b296318a2c334138dfec`.

## Changes and preserved acceptance

- Reconcile supersession fixtures share construction; all 12 tests and 98 original
  assertions remain. Review cached/inflight binding retains both scenarios, all
  expected values, physical requests and diagnostic text using fallible assertions.
- Report shares one retained-roster fixture constructor and independent metadata
  encoder. Four caller-specific bodies, dates, identities and assertions remain.
  Removed resource-classification test is dominated by the retained exact-variant
  and exact-field test using the same three inputs and corruption negative.
- Store shares membership installation, bounded fixture transport and exact physical
  single-addition checking. Returned birth sequence/digest selects the new anchor;
  prior history is unchanged. Independent raw Fjall reads and approval encoders stay.
- Service shares cache seeding and independent CSV/workbook readback mechanics.
  Missing/null metadata distinctions remain; new literal cache and malformed CSV
  regressions preserve decoder independence. Fixture fetchers now explicitly refuse
  live fallback. Workbook row counts remain separate from ID-set checks.
- Coach lanes use typed Rust descriptors, one pure policy interpreter and one binding
  shell. All 141 prior tests and 843 assertion sites remain. Three new tests cover
  inclusive freshness, SD ordering and independent jurisdiction/error expectations.
  Historical state decisions, wire formats, literals and journal readers are unchanged.
- OHSAA projects one checked, school-owned table representation instead of reparsing
  to end of document. Independent foreign-table and mixed-owner cases are added;
  malformed ERROR(000) captures explicitly refuse instead of inventing a school.
  Repository callers migrate to the checked API; old current parser paths are removed.

## Intentional changes and limits

OHSAA v3 replay reprojects captures but does not retract previously stored foreign
coach facts. Existing contaminated stores require a separately authorized correction;
this change does not migrate or erase historical data. Search ambiguity and broader
active-markup admission are not fixed here. Public OHSAA mapping/parser signatures
change; no outside-repository consumer compatibility has been established.

Historical CO/IL/UT coach-lane Armed decisions are preserved, not newly certified as
fresh production authority. Consolidation does not close R1, R2 or R3. Independent
source-backed end-to-end acceptance and historical replay remain required.

## Verification checkpoint before native gates

Static diff-whitespace checks and independent changed-surface reviews performed.
Formatting, all-target compilation, focused runtime regressions and release gates
are NOT RUN at this checkpoint. Cloud has no Moon/Rust toolchain and insufficient
build space. Native verification is serialized; existing user workloads are untouched.
No performance, peak-memory, source coverage or completed-qualification claim.

Canonical gates, from repository root:

```sh
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:fmt
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:check
env -u CI MOON_RUST_BUILD_JOBS=1 tools/moon-local run pipeline:lint-src
```

Focused offline test commands and exact results will be recorded after execution.
Compilation job limits do not bound test concurrency; configure it separately.
No code comments, dependencies, provider bypasses, production mutations or new
repository evidence bundles are introduced. `bd` is unavailable in the cloud session;
no claim of current Beads reconciliation is made.
