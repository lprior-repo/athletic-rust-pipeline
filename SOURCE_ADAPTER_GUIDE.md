# SOURCE_ADAPTER_GUIDE.md — how to add a source adapter to the census

An adapter discovers public source objects and turns captured documents into provider-owned
observations with evidence. Rust adjudication, not an adapter, accepts canonical identity.
[ADR-013](docs/adr/ADR-013-fresh-national-source-census.md) and the
[national census master plan](docs/NATIONAL-CENSUS-PLAN.md) bind the fresh 51-jurisdiction target:
no seed workbook, admissions matching or imported recruit population. Public result spreadsheets
remain eligible captured documents. Existing collector/store coupling is migration debt, not a
second architecture; shared contract changes go through Main.

## Where an adapter lives

```text
crates/census-crawl/src/<name>.rs         module root (facade) when the adapter has parts
crates/census-crawl/src/<name>/           the adapter's parts (collect + parse + map + tests)
crates/census-crawl/src/lib.rs             `pub mod <name>;` in the alphabetical module list
crates/census-service/src/cli/provider.rs           provider dispatch and explicit CLI help
crates/census-service/src/cli/mod.rs                the `Command::Provider` variant itself
```

The worked example is `wiaa` (Wisconsin association directory), which has no flat file:
`wiaa/{mod.rs, parse.rs, map.rs, collect.rs, collect_schools.rs, primitives.rs, tests.rs}` at the
crawl crate root.
Adapters that only need one file use the flat `<name>.rs` at the crawl crate root (e.g. `ks.rs`,
`ohsaa.rs`); both
shapes are ordinary Rust module layouts, not two kinds of adapter.

Register the adapter through the shared capability registry and CLI, not a parallel source list:

1. `pub mod <name>;` in `crates/census-crawl/src/lib.rs` (keep the alphabetical block).
2. A `"<name>" => <name>_report(&context, args, observed_on).await` arm in the `match` of
   `run_provider` (`crates/census-service/src/cli/provider.rs`), plus the thin helper that maps the
   shared `ProviderArgs` fields onto the adapter's own `Options`.
3. Keep the provider's declared capabilities and explicit CLI help aligned with the registry.
   Do not introduce Rust documentation comments or a second manually maintained source catalog.

`cargo xtask new-source <name>` creates the initial module and registration material
(`xtask/src/scaffold.rs`, templates in `xtask/src/templates.rs`). Scaffolding is not delivery:
complete acquisition, provenance, fixtures and shared application integration before acceptance.

## The one required function

Migrate existing `collect` entry points in place to a typed crawl boundary; do not add a second API.
Production errors belong to `CrawlError`/`thiserror`, with `anyhow` only at application composition.

The current shared `AdapterContext` (do not extend it per-adapter) carries:

| field | meaning |
|---|---|
| `fetcher` | the HTTP acquisition boundary for robots, authorized hosts and physical-origin admission; one transport attempt |
| `store` | current transitional coupling; not authority to accept canonical identity or bypass shared atomic ingest |
| `refresh` | when true, re-fetch and re-parse even if the store already holds the observation |
| `school_year` | the academic year this collection belongs to |
| `observed_on` | observation context; actual acquisition time must come from the capture, not a later cache read or export |

The target shared ingest commits captured-evidence references, observations, progress and receipt
together in the application/store layer. Existing direct-store collectors must migrate to that
path. Keep options consistent with existing CLI conventions: bounds, jurisdictions, seasons,
refresh policy and observation context. A diagnostic file input or legacy registry adapter does
not define the national population; national source-unit IDs must come from persisted discovery.

A state-shaped subject list is `census_domain::UsJurisdiction`, not `String`: the `teams`/`collect`
CLI flags parse `Vec<UsJurisdiction>` (`crates/census-service/src/cli/gather.rs` accepts any USPS
code), and the `milesplit` adapter derives its per-state host from the jurisdiction code
(`crates/census-crawl/src/milesplit/wire.rs: Site::for_jurisdiction`) instead of carrying a
table of site strings. **Mid-refactor**: adapters that still declare a free-form
`states: Vec<String>` are the remaining cutover sites — new adapters must take the typed jurisdiction
and derive any host/URL from it, never a parallel string convention.

## Evidence rules

* Every parsed fact retains its source namespace and native object identity where one exists,
  together with the immutable capture and exact locator. Missing native IDs produce qualified
  capture-scoped subjects, never name hashes or row positions presented as canonical people.
* `SourceNamespace` variants are declared in `crates/census-domain/src/model.rs`; add a variant
  there rather than smuggling a source tag through a `String` (ADR-003's rule about precision
  applies to source identity too).
* Evidence method matters: `EvidenceMethod::Parsed` for a document you parsed, distinct from
  derived/inferred facts. Never mark an inference as parsed.
* A source failure is **never** `NO_MATCH`. An adapter that cannot fetch a resource reports the
  failure in `AdapterReport` (and lets the workflow decide about a retry, ADR-002) instead of
  writing "not found" evidence.
* Distinguish independent sources from syndicated derivatives in corroboration and coverage.
  Existing core/non-core projection filters do not authorize silently omitting a source family,
  restricting nationwide discovery or counting copies as independent identity evidence.

## Partial failure

An adapter that walks a run of source objects decides, per object, whether a failure is the
*source's* or the *run's* — and the two go opposite ways:

* **An unsupported template is a source finding.** Retain the capture/URL, parser reason and
  rejected locator, preserve any valid rows, and keep independent objects moving (§62). The
  workflow persists a structured finding; `AdapterReport::notes` is not its durability protocol.
* **An unfetched object is not an empty result.** Propagate transport, timeout, 429, 5xx and
  challenge outcomes to the workflow. Restate alone owns retries (ADR-002); the transport performs
  one attempt. Keep quarantine, human-required access and retry exhaustion distinguishable.
* **A run of mismatches is a contract change.** Scattered junk objects are a source fact; a run of
  them with few readable objects between is the template this build knows no longer being the one
  the site serves, which is §69's repeated-malformed-contract stop condition. Bound the walk on
  that ratio rather than on an absolute count, so junk among readable objects never costs a state
  its readable ones — `milesplit::MISMATCH_LIMIT` and `MeetPages::stopped` are the worked example.

## Traffic rules (non-negotiable)

* **One retry owner: Restate.** Adapters, fetchers, browser and model transports perform one
  attempt. No transport retry loop, adapter retry wrapper or nested model SDK retry. The ceiling
  is three total automatic attempts under the same logical operation, retained across restart.
  A 304 without its cached body is a typed cache/body failure, not permission for a hidden second
  request. The historical claim that `net/` owns extra retry paths is superseded.
* HTTP requests use the shared fetcher; browser acquisition uses the registered headed lane.
  Never construct a private client, bypass robots, spoof identity or use HTTP to evade a challenge.
* Bound concurrency and share admission across all physical requests to the origin, including
  browser subrequests. More adapters, workflows or endpoints must not multiply the origin budget.
* A challenge requires `HumanRequired`; an access refusal remains an explicit blocked outcome.
  Retryable 429/5xx responses follow Restate's bounded policy and Retry-After, then retain exhaustion
  as evidence. Independent sources continue.
* Operator-authorized hosts are declared globally (`--authorized-host`, recorded as
  `robots_authorized` with the `MIN_AUTHORIZED_DELAY` floor). An adapter does not get its own
  authorization story.

## Idempotency

Replaying one logical effect must not append duplicate observations or prematurely close a unit:

* Bind the stable operation ID to its source unit, capture, parser revision and content. A receipt
  is checked before replay; reuse with changed content is an error, not an overwrite.
* The shared ingest atomically commits observations, progress and receipt before acknowledging.
  Readers merging duplicate rows is not proof of exactly-once effects.
* A genuinely new capture or parser revision may produce new observations. Preserve old evidence
  and versioned decisions; do not confuse a refresh with replaying the same effect.
* Partial parses preserve accepted rows plus rejected locators and pending obligations. A journal
  entry saying the adapter ran is not a completion certificate.
* Increased execution budgets resume retained work under the same logical identity. Source/cohort/
  policy changes require an explicit semantic revision, not a hidden change to immutable input.
* A crash leaves each batch wholly visible or absent, with exact remaining work recoverable.
  `refresh` and cache hits cannot bypass receipt or durability rules.

## Reporting

Return `AdapterReport` counters derived from actual acquisition and parsing outcomes. Distinguish
physical requests, captures, parsed observations, accepted identities and retained gaps; do not
report all parsed names as accepted Class-of-2027 athletes. Persist source/program/season obligations,
discovered links, continuation cursors, access restrictions and budget stops through the shared
application. Human-readable notes supplement structured durable outcomes, not replace them.

## Tests and fixtures

* Unit tests live under the adapter's `#[cfg(test)] mod tests;` (a sibling `tests.rs` — `wiaa/mod.rs`
  ends with `mod tests;`), driven by embedded payloads or `crates/census-crawl/tests/fixtures/**`;
  **no test may touch the network**.
* Test the parse layer against real captured snippets (trimmed), including the shapes that break
  naive parsing: missing columns, `no mark` rows, wind/attempt columns, Unicode names, empty
  divisions.
* Exercise malformed captured payloads and discovered-source boundaries: invalid IDs, UTF-8,
  unsupported formats, wrong season/grade context, partial rows and continuation limits.
* Prove syndicated evidence does not count as independent corroboration, and failures never
  become absence. Projection filters must not silently change the run's national denominator.

## Checklist for a new adapter

1. `<name>/` at the crawl crate root with `Options`, `Target`/subject type, parse layer, `collect`,
   tests.
2. Shared capability registry and CLI wiring, with explicit help and no duplicate source catalog.
3. Constrained source namespace and provider-owned object identities; shared type changes through Main.
4. Durable capture and exact locator for every accepted fact; retain partial/rejected evidence.
5. One transport attempt, shared physical-origin admission and bounded work.
6. Shared atomic observation/progress/receipt ingest; retry replay cannot duplicate effects.
7. `AdapterReport` counters plus durable structured findings and continuation obligations.
8. Fixtures under `crates/census-crawl/tests/fixtures/<name>/` if payloads are too large to
   embed; tests network-free.
9. `cargo fmt`, strict clippy via `tools/gate.sh` (zero new diagnostics),
   `cargo xtask source-test <name>`.
10. Report back: files touched, commands run, results, remaining uncertainty.
