# census-service

An independent census of Midwest high-school track & field / cross-country athletes, schools, teams,
coaches, meets, events and performances.

**Athletic.net and its AthleticLIVE mirror are outside the core contract.** Every number in the
`core` rows below was produced with both switched off: the census reads state associations,
MileSplit-style rosters, official result artifacts and timing-provider schedules that the pipeline
fetches itself. The `all_sources` scope adds the two AthleticLIVE modules (a results mirror and its
athlete index) purely as comparison, and the difference between the two rows is the measured
marginal coverage of the source this pipeline refuses to depend on.

## Run

```text
cargo run --release -p census-service -- <command>

  fetch           Fetch a single URL through the polite fetcher (robots-enforced, cached)
  sites           List the MileSplit state sites — one per jurisdiction, host derived from its code
  teams           Fetch (and cache) team indexes for the given states
  meets           Enumerate a state's published meets into `source_meets` [--year YYYY] [--states …]
  collect         Walk rosters and emit canonical entities for the given states
  import-coaches  Import the researched official coach-contact CSV into canonical entities
  provider        Run one association contact adapter by name
  consolidate     Merge append observations into `out/*.jsonl` snapshots
  index           Derive the store's indexes: source-object identities, conflicts, review cases,
                  coverage and a snapshot of the pass
  report          Compute the measured census from the store
  bests           Reduce to one best mark per athlete and event   [--grad-year 2027] [--limit N] [--all]
  workbook        Build the census workbook (.xlsx) and sidecars [--out PATH] [--grad-year 2027]
                  [--limit N] [--core]
  seal            Certify the census, or refuse and name the §70 item that blocked it [--core]
  run             Gather a registry (optional), consolidate, derive the indexes, publish both
                  census scopes, reduce bests and write the workbook in one command
                  [--input PATH] [--states WI,MN] [--limit N] [--grad-year 2027] [--core]
                  [--refresh] [--observed-on YYYY-MM-DD] [--out PATH]
  fjall-stats     Print per-table observation counts and the database footprint
  import-legacy   Run the one-time pre-Fjall JSONL import, then print the store stats
  serve           Print the command that runs the `census-serve` Restate endpoint

Global: --store <dir> (default var/census-service), --delay-ms <n>, --user-agent <ua>,
        --authorized-host <host> (repeatable; records the host's robots rules as authorized and
        applies the 2 rps ceiling instead of blocking)
```

A full cycle is `collect` → `provider <name>` per adapter → `consolidate` → `report` / `bests` /
`workbook`. Every source is the default; `--core` restricts a command to the Athletic.net-free
scope, which is the independence diagnostic rather than the recruiter product. Every adapter is resumable: a unit of
work is journaled with its parser version, and a re-run skips what an unchanged parser already
produced. Each provider takes `--seasons`, `--limit` and `--refresh`.

`bests` and `workbook` default to the class of 2027 and take `--limit`; `bests --all` reduces every
athlete in the core scope instead, and is rejected in combination with `--grad-year` rather than
silently ignoring it. The cohort selector is a `u16` on the command line, so a negative year never
reaches the store and a year above `i16::MAX` is rejected instead of truncated. `fjall-stats` reports
what the database holds, table by table, and `import-legacy` runs the one-time import and then prints
the same statistics. `serve` prints the service command; it never starts a server itself.

The CLI holds the store for the life of the command because the Fjall database takes an exclusive
lock, which is what keeps two runs from interleaving writes. The same lock means `census-serve` and
a batch command cannot work against one `--store` at the same time — stop the service before running
`collect`, or point them at different stores.

## Storage: Fjall

The store is a [Fjall](https://fjall-rs.github.io) database (`fjall =3.1.10`) — an embedded LSM-tree
key-value store written in safe Rust — under `<store>/fjall`. One process owns a database at a time:
`Store::open` takes an exclusive lock on it, so a run either holds the store for its whole life or
fails with that reason instead of interleaving writes with another process.

| keyspace | key | value |
|---|---|---|
| `entities` | `<table>\0<entity-id>\0<sequence:u64 big-endian>` | one observation, as JSON |
| `journal` | `<phase>\0<key>` | `{key, at, payload}` — the resume ledger |
| `meta` | `<name>` | small JSON and scalar values, including the legacy-import markers |

Observations are append-only: appending the same entity twice stores two rows, and
`Store::scan::<T>(Table::X)` merges them through `Entity::merge` and then applies `Entity::publish` —
the collection contract, applied once per merged entity so the report, the workbook, the snapshot and
the Restate handlers all see the same projection. Every published address is kept in the field for
its domain kind (`professional_email` for organisation domains, `personal_email` for consumer
mailboxes); all published addresses remain available.
That is the guarantee the old JSONL entity logs provided, now applied at read time. Sequence numbers are seeded from the last key
present at open, and the sequence component is big-endian so byte order is numerical order, so a
reopened database never reuses a sequence number and never overwrites an observation.

Each batch of observations commits with `SyncData` (`fdatasync`) — the cheapest mode that survives a
machine crash — and `flush()` upgrades to `SyncAll` at consolidation and at shutdown. A lost tail
costs re-running an adapter, and the resume journal is durable per completed unit of work, so a
resumed run does not repeat finished work. A scan aborts with a typed error if a table holds more
than 20,000,000 observations (`store::MAX_ROWS_PER_TABLE`) rather than exhausting memory; the cap
counts observations read, not merged rows. The LSM cache is bounded at 256 MiB.

`out/*.jsonl` is the materialized read model: `consolidate` writes one snapshot per table through
`Store::consolidate::<T>`. Every command reads the merged tree through `Store::scan::<T>` — `report`,
`bests` and the workbook included — so they see every committed observation without waiting for a
consolidation pass, and the snapshot is what goes out as the tabular record beside them. `bests`
writing `out/best-results-*.jsonl` therefore needs no prior `consolidate`.

Databases created before this substrate keep their rows in `<store>/entities/*.jsonl` and their
resume ledger in `<store>/journal/*.jsonl`. `Store::open` imports both exactly once: a table is
marked imported only after all of its observations are committed, so an interrupted import resumes
instead of restarting, and later opens skip it. The JSONL files stay in place as the record of what
the database was built from, and `import-legacy` runs that path and prints the import source next to
the resulting store statistics.

## Merge laws

`tests/merge_properties.rs` pins the read model's contract: every read merges an entity's
observations through `Entity::merge` and then applies `Entity::publish`.

- **Idempotency** — merging an identical observation changes nothing.
- **Union commutativity** — `source_identities`, `evidence`, `aliases`, `known_names`, `sports`, `source_urls` and `source_labels` are sets: merge order cannot reach a row.
- **First-writer-wins** — a scalar a row already carries is never replaced (`name`, `city`,
  `enrollment`, `level`, `professional_email`, `personal_email`, `wind_mps`, …); a hole is filled
  from the other side (`None`, or the meet's `Unknown` level), which is why merge is idempotent on
  a row it produced.
- **Identity preservation** — merge never rewrites the name a record was minted from
  (`school.name`, `school.normalized_name`, `athlete.canonical_name`); other spellings land in
  `aliases` / `known_names`.
- **Cohort rule** — an `ObservedGrade` that disagrees with `grad_year` lowers `identity_confidence`
  to `LOW`, agreement raises it to `HIGH`, and silence leaves it alone.
- **Contact policy** — `publish` keeps every valid address, placing organisation domains in
  `professional_email` and consumer domains in `personal_email`; every valid address remains available.

Both suites are deterministic: every `proptest!` block pins 64 cases on ChaCha with the fixed seed
`0x4D45_5247_5F_4944`, so a failure reproduces from the seed alone.

## Durable execution: Restate

`census-serve` (same crate, `restate-sdk =0.12.0`) exposes the same store and the same reports as
the batch CLI, with Restate owning the journal: every handler effect runs inside `ctx.run`, so a
retry or a restart replays the journaled result instead of repeating a completed step. The service
does not fetch: a producer hands it a table and its observation rows, and `Ingest.record` appends
them to the same Fjall database the CLI would.

```text
census-serve --listen 127.0.0.1:9080 --data-dir var/census-service --max-concurrent 8 --drain-timeout 30
run it with: cargo run --release -p census-service --bin census-serve -- --listen 127.0.0.1:9080 --data-dir var/census-service --max-concurrent 8 --drain-timeout 30
```

The endpoint speaks HTTP/2 without an upgrade dance (hyper's `http2::Builder`), so a plain HTTP/1.1
client sees an `UnexpectedMessage`; `curl --http2-prior-knowledge` and `reqwest` with
`http2_prior_knowledge()` are the working clients. `--listen` refuses any non-loopback address: the
endpoint carries no request-identity key, so it must not leave the machine. Reports land under
`<data-dir>/out`, exactly where the batch CLI writes them.

`census-service serve` prints those two lines: the service command and how to run the same binary
under cargo. The current `--store` fills the `--data-dir` value, the rest are the service defaults,
and it never starts a server itself.

| kind | wire name | shape |
|---|---|---|
| service | `Census` | `status`, `consolidate`, `report`, `bests`, `workbook`; each heavy job runs on `spawn_blocking` behind a semaphore sized by `--max-concurrent`, inside `ctx.run`, so a restart replays the journal value rather than repeating a completed pass |
| virtual object | `Ingest` | one object per source endpoint, which is what makes the per-endpoint cursor and window bookkeeping safe against concurrent writers: `state`, `record`, `complete_window` |
| workflow | `Sweep` | observes the ingest objects over `windows` windows (`window_seconds` apart, durable sleeps), exits early when `interrupt` is resolved, and reports per-endpoint observation counts, endpoints that never accepted an observation, and where the pass wrote its report |
| virtual object | `JurisdictionCensus` | one object per jurisdiction identity (`jurisdiction:<state>:<season>:<revision>`, `census::WorkflowIdentity`), running that state's team index, roster walk, meet census and consolidate stages and recording each in durable state so a re-invocation resumes at the stage it still owes: `state` (shared), `run` |
| workflow | `NationalCensus` | the root run per season and revision (`national:<season>:<revision>`): fans out one `JurisdictionCensus` call per jurisdiction and folds the per-state reports into one national report, listing failed states as rows instead of failing the run: `run`, `report` (shared) |

The last two rows are the newest definitions (`restate_services/{jurisdiction,national}.rs`, bound in
`build_endpoint`); they are the national-scope surface and have no CLI command in this crate yet.

Shutdown is a protocol rather than a flag: intake stops, in-flight invocations get `--drain-timeout`
seconds to finish, whatever outlives the deadline is aborted and counted, and the store is flushed
(`SyncAll`) before the database is dropped. The last line the service prints is that drain
certificate — `accepted`, `completed`, `cancelled`, `timed_out`, `aborted`, `panicked`.

## Reports and the workbook

Three commands turn the store into evidence:

* `report` — the census itself: `out/report.json` (and `out/report-core.json` for the core scope)
  plus `out/census-by-state*.csv`. It reduces the merged store tables directly.
* `bests` — one best mark per `(athlete, event)`, compared only within a mark's own measure with
  relays excluded, written as `out/best-results-<cohort>.jsonl` and `.csv`, where the cohort is
  `co2027` or `all`. It reads the merged store, so it never depends on a prior `consolidate`.
* `workbook` — the census as one `.xlsx` with nine sheets (goal & method, summary, by state for each
  scope, Athletic.net marginal, best results, meets, evidence mix, method notes) and the best-mark
  sidecars beside it. Every cell is copied from the typed census or the best-mark reduction, nothing
  is recomputed, and the file is written by the same `rust_xlsxwriter` dependency the rest of the
  crate uses. `--out` chooses the path; the default is
  `<store>/out/census-service-<generated-on>.xlsx`.

These commands replace `reports/build-census-workbook.py`, which transformed the two scope reports
into a workbook by emitting flat ODS XML and converting it with headless LibreOffice. That script is
deleted: the crate writes the workbook in process and nothing in the repository runs Python. The
2026-09-20 workbook and its CSVs under `reports/` are the output of that earlier run and are kept as
the record of it, including the sheet that names the command they were built with.

## Measurement harness

Two example targets measure the substrate and the reduction. Both build their own bounded synthetic
corpus in a temporary store, so neither touches the store you collected into, and both print
`metric=<name> ...` lines plus one `json={...}` summary line:

```text
cargo run --release -p census-service --example bench_store  -- --rows 200000 --batch 1000 --scan
cargo run --release -p census-service --example bench_census -- --schools 500
```

`bench_store` measures the store itself: single appends, batched appends, and — with `--scan` — the
merge-scan and consolidation. `bench_census` drives a deterministic corpus through the whole
pipeline, report and workbook steps included. Every phase asserts the counts it produced before
reporting a rate, so a run cannot silently measure a store that lost rows. Run them from the crate
directory as `cargo run --release --example bench_store`; from the repository root they need
`-p census-service`. No timing is quoted in this README — measure the machine in hand.

## Measured, 2026-09-20

Both tables below are a snapshot of that day's ingestion, taken before the Fjall substrate and the
Restate layer landed; they are a record of a run, not a claim about any store held now. Re-run
`consolidate`, `report` and `workbook` to measure the store in hand.

| scope | athletes | Class of 2027 | boys | girls | grade-evidenced | profile URL | coach | coach email |
|---|---|---|---|---|---|---|---|---|
| **core** (Athletic.net off) | 651,736 | **146,858** | 80,896 | 65,751 | 100% | 142,916 (97.3%) | 37,179 (25.3%) | 28,929 (19.7%) |
| all sources | 787,584 | 190,087 | 105,755 | 84,034 | 100% | 176,885 (93.1%) | 43,201 (22.7%) | 33,340 (17.5%) |

Core independently reaches **77.3%** of the all-source Class-of-2027 population. 27,580 canonical
coaches are held in total.

| state | schools | athletes | Class of 2027 | boys | girls | with coach | with coach email |
|---|---|---|---|---|---|---|---|
| OH | 1,504 | 87,765 | 26,998 | 14,414 | 12,507 | 313 | 313 |
| IL | 1,889 | 69,493 | 19,392 | 10,856 | 8,521 | 6,432 | 6,244 |
| WI | 1,027 | 118,031 | 19,281 | 10,323 | 8,929 | 14,699 | 13,646 |
| MI | 1,493 | 94,771 | 16,176 | 9,285 | 6,891 | 0 | 0 |
| MO | 1,037 | 42,226 | 13,218 | 7,377 | 5,839 | 0 | 0 |
| IN | 1,108 | 40,846 | 11,909 | 6,430 | 5,402 | 0 | 0 |
| MN | 1,016 | 54,084 | 11,261 | 6,147 | 5,114 | 9,338 | 8,629 |
| KS | 836 | 29,950 | 9,252 | 5,258 | 3,993 | 0 | 0 |
| IA | 913 | 49,024 | 8,738 | 4,924 | 3,804 | 76 | 19 |
| NE | 678 | 35,800 | 5,921 | 3,248 | 2,673 | 4,971 | 0 |
| SD | 427 | 17,134 | 2,669 | 1,518 | 1,151 | 239 | 78 |
| ND | 339 | 12,612 | 2,043 | 1,116 | 927 | 1,111 | 0 |

Meets: **1,532 core** (WI 796, MN 209, IA 85, otherwise unresolved) against 11,007 across all
sources, of which 9,593 carry an Athletic.net meet id - that id is retained as an enrichment key and
is never dereferenced by a core run.

## Source tiers

| tier | source | adapter | what it yields |
|---|---|---|---|
| A | WIAA school/team/coach database (WI) | `wiaa` | schools, teams, coach names and email |
| A | WIAA result archive (WI) | `wiaa_results` | grade-bearing result rows, schools, class-of-2027 evidence |
| A | MSHSL (MN) | `mshsl` | schools, activities director, published staff email |
| A | IHSA (IL) | `ihsa` | member schools, per-school head-coach roles and email |
| A | OHSAA (OH) | `ohsaa` | schools, head coach names by sport |
| A | KSHSAA (KS) | `ks` | member schools, athletic-director name and email |
| A | NDHSAA + NSAA (ND, NE) | `plain_names` | school universes, coach names (no email published) |
| A | researched official contact graph | `coach_contacts` (`import-coaches`) | artifact import: school / sport / role + published email |
| B | MileSplit-style state sites | `milesplit` (driven by `teams` and `collect`), `milesplit_results` (driven by `meets`) | rosters, graded athletes, profile URLs, the meet census and whole-meet result sets |
| D | vendor result artifacts | `result_file` dispatching `hytek`, `compiled`, `xc`, `raceday` | performances with grade evidence |
| E | Wayzata Results (MN / IA / WI timer) | `wayzata` (provider name `wayzata_schedule`) | meet inventory from published schedules |
| - | AthleticLIVE mirror | `athleticlive`, `athleticlive_athletes`, `athleticlive_results` (manifest-driven import of captures, no request) | **non-core**: comparison only |

### Result-artifact parsing

`result_file` dispatches on the detected vendor layout and shares one header-anchor rule across
`hytek`, `compiled` and `xc`. On the Wisconsin archive that rule takes 96 parsed artifacts to 1,740
(compiled 763, cross-country 380, Hy-Tek 597) and produces 834,254 result rows, 264,167 of which
carry a grade - enough to add 4,323 class-of-2027 athletes to core without a single Athletic.net
request.

The public seam is `sources::wiaa_results::parse_result_body(&[u8], ArtifactFormat, SourceRef, i16)
-> Option<ParsedMeet>`; `artifact_format(extension, body)` picks the `ArtifactFormat` for a release.
It is pure — no network, file, clock or randomness — except the PDF arm, which shells out to
`pdftotext -layout`:

| `ArtifactFormat` | read as |
|---|---|
| `HytekHtml` | `hytek::lines_from_html` then `hytek::parse`; a body carrying a `<pre>` block is read from that block alone, otherwise from its paragraphs |
| `HytekText` | `hytek::lines_from_text` then `hytek::parse` — the HTML and text releases of one report parse to the same `ParsedMeet` |
| `RaceDay` | `raceday::parse`; the format publishes no date, so the archive `year` becomes the meet date |
| `Pdf` | `pdftotext` text, then `hytek`, `compiled`, `xc` in that order |
| `Unparsed` | `None`, whatever the bytes are |

`None` means "not a meet that can be minted" — a missing header, an unreadable PDF, an unknown
layout — never a half-parsed record. `tests/parser_roundtrip_properties.rs` pins the dispatch,
front-end agreement, prefix stability under trailing garbage and truncation, and that arbitrary
bytes give `None` or a meet but never a panic.

### Wayzata schedules

The provider publishes one server-rendered schedule table per sport and season. Its `/links/<slug>`
pages render AthleticLIVE, so the adapter keeps the slug as a `TimerMeet` key and the URL as a
source URL while never fetching either: the schedule row is the evidence. Meet identity is the
platform's own (`state + date + normalized name`), so a meet listed here reconciles with the same
meet arriving from an association artifact. Venues are resolved to a state by a recurring-site table
or, failing that, by the consolidated school snapshot restricted to the provider's region - a
nationwide search turns "Austin HS" into a three-way tie, while the provider's Austin is the
Minnesota one. Unresolved venues are filed under `??` rather than guessed: on the 2026 schedules,
304 of 537 rows resolve (95 sites, 209 schools).

## Isolation contract

- `report::NON_CORE_SOURCE_IDS` names the AthleticLIVE mirror modules (meet index, athlete rows,
  result plane) and Athletic.net itself; `core` scope drops any evidence they produced. Test:
  `report::tests::core_scope_keeps_only_non_athletic_net_evidence`.
- Core adapters mention Athletic.net only in prose, never as a request target or a parser input.
- Cross-source agreement (`multisource`) is an all-source metric; it is 0 in core scope by
  construction, because core scope holds one independent view of each athlete.

## Known limits

- Coach coverage is uneven by design: the states whose association publishes a coach directory (WI,
  MN, IL, OH, NE, ND) have it, IA and SD have partial coverage from the researched contact graph, and
  MI, MO, IN and KS have none yet.
- 442 core meets have no resolved state, and core meet inventory outside Wisconsin, Minnesota and
  Iowa depends on which association artifacts have been ingested.
- Grade evidence is "grade observed in a source", not a verified graduation year; the platform keeps
  `GradYear` and `ObservedGrade` as separate fields for exactly that reason.
- Test and lint counts are not restated here because every slice of work moves them: the
  2026-09-20 record was 183 passing tests and 17 clippy warnings; both are now cleared, and the
  crate is clippy-clean. The workspace gate is the current answer: run `tools/gate.sh` from the
  repository root — or `cargo xtask source-test <name>` for one adapter's tests,
  `cargo xtask census-status --store <dir>` / `cargo xtask coverage --store <dir>` for the measured
  census — and never quote a count that a gate run did not print.
