# `aia` source adapter (Arizona Interscholastic Association)

## Purpose

AIA publishes its member schools and the head coach of each school-sport on `aiaonline.org`. The
adapter collects Arizona member schools and their Cross Country / Track head coaches into
`CanonicalSchool` and `CanonicalCoach` rows, each citing the profile page it was read from. It
collects no athletes and no results. `AIA_EVIDENCE`/`AIA_REFUSAL` in
`crates/census-crawl/src/applicability/table/prose.rs` own the applicability claim.

## Public surfaces

| Surface | URL | Notes |
| --- | --- | --- |
| Robots | `https://aiaonline.org/robots.txt` | `User-agent: *` / `Disallow:` — all paths allowed |
| School search | `https://aiaonline.org/schools/search.json?q=<query>` | JSON array; at most 10 rows per query and 20 for an empty query, so there is no full inventory route |
| School profile | `https://aiaonline.org/schools/<id>` | Server-rendered HTML: name, address, and one sport card per sport with the head coach's name |

## Entry points

| Item | What it is |
| --- | --- |
| `mod.rs` `collect` | the adapter body, `(&AdapterContext, &Options) -> Result<AdapterReport>`; `Options` carries `limit`, `refresh`, `observed_on`, `states` and `school_names` |
| `mod.rs` `SOURCE_ID` | the evidence slug stamped into every `SourceRef` |
| `parse.rs` | pure parsing: profile HTML and search JSON in, parsed rows out, no I/O |
| `map.rs` | parsed rows to canonical entities, including the `aia_schools`/`aia_coaches` journal rows |
| `tests.rs` | fixture-driven tests; every capture in `tests/fixtures/aia/` must parse |

The parser anchors on the live template: the `xl:text-6xl` h2 holds the school name, and each
`md:flex px-4 py-2` card carries the sport label in its `md:w-1/3` half and `Head Coach`-roled
blocks in its `md:w-2/3` half. Labels are HTML-escaped, and track cards read `Track - Boy's`. The
captures and their hashes are in `tests/fixtures/aia/SOURCE.md`.

## Discovery and limits

- Discovery is a sample by construction: the search endpoint returns at most 10 rows per query, so
  `collect` walks a fixed city list (plus any `--school-names`) rather than a member index. A school
  a query does not match is not thereby absent from membership.
- Only head coaches are published, and coach emails sit behind a login.
- Profiles carry a street address; the adapter records the city on the canonical school but does not
  mint postal claims (that port belongs to ADR-020).
- The adapter covers Arizona only and reports a state mismatch for any other requested state.

## Commands

```bash
cargo xtask source-fixture aia   # what is captured for this source
cargo xtask source-test aia      # this adapter's tests
cargo run -p census-service -- --store <dir> provider aia --limit 3 --observed-on <date>
```

## Landing checklist

- [x] real URLs and their robots status recorded here and in `research/sources/aia/SOURCE_REPORT.md`
- [x] parsing and mapping asserted against live captures (`cargo test -p census-crawl --lib aia`)
- [x] registered in the source registry and applicability table
      (`registry/table/through_milesplit.rs`, `applicability/table/data.rs`), and exposed as the
      `provider aia` CLI subcommand
- [x] rows and journal entries appended through the page `AdapterContext::write_batch` hands out
- [x] live smoke and durable readback recorded in `docs/VERIFICATION-EVIDENCE.md`
