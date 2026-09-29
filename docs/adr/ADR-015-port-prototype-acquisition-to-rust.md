# ADR-015 — Port prototype acquisition to Rust; captured parity fixtures are the evidence

Status: accepted.

## Context and decision

The national source inventory and the coach/school acquisitions were produced by the
`census-prototype` research monorepo: `sources.py` names ~40 extractor kinds across 51
jurisdictions, `run.py`/`deepen.py` drive them, and `raw/` caches every response body they read.
ADR-008 already forbids a second language in the pipeline, so the prototype is not a delivery path —
but its capabilities are not all implemented in Rust yet, and a fresh national re-run must not need
Python at all.

Decide:

1. Every prototype capability that produces or validates census school/coach rows is ported to
   `census-crawl` adapters behind the existing registry, then removed from the run path.
2. The prototype stays in the repository as read-only research evidence and as the one-time parity
   oracle. It is never invoked by a census run, a gate, or a deployment step.
3. Each ported source ships the captured response body under
   `crates/census-crawl/tests/fixtures/<source>/` and the prototype extractor's rows for that body as
   a committed golden file. The Rust test asserts parity on the shared fields; the golden is dated
   evidence, not a second implementation.
4. Source admission (robots verdict, rate, transport) comes from
   `research/sources/applicability-matrix.md`. A source whose host refuses robots, or that publishes
   no coach-bearing content, is recorded as out-of-scope with its citation instead of being ported
   speculatively.
5. The capture cache under `census-prototype/raw/` is the origin of those fixtures; captures are
   copied, never re-fetched to build a fixture.

## Consequences

- Golden files are generated once, by the prototype extractor, at port time; the generating command
  is recorded in [VERIFICATION-EVIDENCE.md](../VERIFICATION-EVIDENCE.md).
- Tests run without Python, and the fixture bodies make each adapter's behavior reproducible offline.
- A national re-run reaches parity only when the ported set covers every in-scope jurisdiction; the
  remaining gap stays visible in `research/sources/applicability-matrix.md` until closed.
- Prototype code may be deleted only after its capability is ported and its evidence committed;
  [ADR-010](ADR-010-preserve-existing-corpus.md) and [ADR-013](ADR-013-fresh-national-source-census.md)
  preserve captures, not executable authority.

## Port slice contract (frozen 2026-09-29)

One port slice is one source family, owned end to end by one worker, which keeps every file below
until the slice is integrated. This is the contract that lets slices run in parallel without
touching one another's decisions.

- **Ownership.** `crates/census-crawl/src/<slug>/**` (module, parse, map, collect, tests, README),
  `crates/census-crawl/tests/fixtures/<slug>/**`, one `SourceDescriptor` appended to
  `crates/census-crawl/src/registry/table/*.rs`, one `Applicability` appended to
  `crates/census-crawl/src/applicability/table.rs`, the `pub mod <slug>;` line in
  `crates/census-crawl/src/lib.rs`, and the slice's dated section in
  [VERIFICATION-EVIDENCE.md](../VERIFICATION-EVIDENCE.md). Nothing else — in particular not
  `net/**`, not another adapter, not `census-service/**`.
- **Admission.** The descriptor and the applicability entry are appended as a pair carrying the same
  slug (`applicability/tests.rs` asserts the two slug sets are equal). `transport`, `capabilities`
  and `access_class` describe what the source actually does — an artifact reader declares
  `local-artifact` and no fetchable host — one request stays in flight, `robots_crawl_delay_respected`
  stays true, and the rate is the lane's measured rate (1 request/second unless the host publishes a
  crawl-delay).
- **Fixtures.** Copied from `census-prototype/raw/`, matched by the `url` field of the sibling
  `*.meta.json`, byte-identical (`cmp` plus sha256 re-verified by a second pass), each fixture set
  carrying `PROVENANCE.json` with `url`, `prototype_file`, `sha256` and `bytes`. Never re-fetched to
  build a fixture.
- **Goldens.** One JSON file per fixture page holding the prototype extractor's rows for that body,
  generated once at port time by running the extractor over the fixture file; the exact command and
  its row counts are recorded in the slice's evidence section. A golden is dated evidence, not a
  second implementation: where a golden and the raw fixture disagree, the fixture wins and the
  golden is regenerated.
- **Parity tests.** The Rust test asserts the fields both shapes share, states taxonomy differences
  explicitly (for example the prototype's conflated `Track` against Rust `IndoorTrack` /
  `OutdoorTrack`) and covers malformed or truncated input, which must surface typed errors rather
  than panics.
- **Gates.** `cargo test -p census-crawl`, `cargo clippy -p census-crawl --all-targets -- -D warnings`
  and `cargo fmt -p census-crawl -- --check`, with the observed output in the slice's evidence
  section.

## CLI host (frozen 2026-09-29)

Source drives stay where they already are: adapters are libraries in `census-crawl`, and everything
that fetches on an operator's or developer's behalf is a verb in `census-service/src/cli/**` —
provider arms under `cli/provider/arms/`, diagnostics beside them — while `xtask` keeps measured
gates, scaffolds and replay harnesses. The DragonFly probe therefore lands as a `census-service`
verb calling `census_crawl::coach_directories::survey`, not as a second binary in the crawl crate.
At this base `census-service` cannot compile (`census-report` is red), so port work proceeds in
`census-crawl` and the verbs are wired with the first green `census-service` build.
