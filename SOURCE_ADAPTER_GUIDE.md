# SOURCE_ADAPTER_GUIDE.md — how to add a source adapter to the census

An adapter turns one external source (state association, timing provider, aggregator, directory)
into `Evidence` and canonical observations in the Fjall store. This document is the contract an
adapter author must satisfy; `DOMAIN.md` covers the types, `ARCHITECTURE.md` the pipeline shape,
`FJALL_SCHEMA.md` the storage keys.

## Where an adapter lives

```text
crates/census-crawl/src/<name>.rs         module root (facade) when the adapter has parts
crates/census-crawl/src/<name>/           the adapter's parts (collect + parse + map + tests)
crates/census-crawl/src/lib.rs             `pub mod <name>;` in the alphabetical module list
crates/census-service/src/cli/provider.rs           `<name>` arm in `run_provider` + `ProviderArgs` doc list
crates/census-service/src/cli/mod.rs                the `Command::Provider` variant itself
```

The worked example is `wiaa` (Wisconsin association directory), which has no flat file:
`wiaa/{mod.rs, parse.rs, map.rs, collect.rs, collect_schools.rs, primitives.rs, tests.rs}` at the
crawl crate root.
Adapters that only need one file use the flat `<name>.rs` at the crawl crate root (e.g. `ks.rs`,
`ohsaa.rs`); both
shapes are ordinary Rust module layouts, not two kinds of adapter.

Registration is three edits, no more:

1. `pub mod <name>;` in `crates/census-crawl/src/lib.rs` (keep the alphabetical block).
2. A `"<name>" => <name>_report(&context, args, observed_on).await` arm in the `match` of
   `run_provider` (`crates/census-service/src/cli/provider.rs`), plus the thin helper that maps the
   shared `ProviderArgs` fields onto the adapter's own `Options`.
3. Add `<name>` to the adapter-name doc comment on `ProviderArgs` (`cli/provider.rs`), which is also
   the list the `unknown adapter` error prints.

`cargo xtask new-source <name>` performs edits 1 and the module layout for you
(`xtask/src/scaffold.rs`, templates in `xtask/src/templates.rs`); it writes a placeholder adapter
whose `collect` bails, so edits 2 and 3 are still yours.

## The one required function

```rust
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> anyhow::Result<AdapterReport>
```

`AdapterContext` (shared, do not extend it per-adapter) carries:

| field | meaning |
|---|---|
| `fetcher` | the only way to make a request — rate limits, robots policy, authorized hosts, user agent and retry rules live inside it |
| `store` | the Fjall store; observations land here, keyed by source identity |
| `refresh` | when true, re-fetch and re-parse even if the store already holds the observation |
| `school_year` | the academic year this collection belongs to |
| `observed_on` | the ISO date recorded on every piece of evidence (`net::today_iso` by default) |

`Options` is adapter-owned but should mirror the shared CLI conventions rather than inventing new
ones: `limit`, `states`, `seasons`, `school_names`, `input`, `refresh`, `observed_on`. `input`
exists for import-style adapters whose subject list comes from a file (the Athletic.net bio
adapter reads an operator-supplied athlete registry, never a search endpoint).

A state-shaped subject list is `census_domain::UsJurisdiction`, not `String`: the `teams`/`collect`
CLI flags parse `Vec<UsJurisdiction>` (`crates/census-service/src/cli/gather.rs` accepts any USPS
code), and the `milesplit` adapter derives its per-state host from the jurisdiction code
(`crates/census-crawl/src/milesplit/wire.rs: Site::for_jurisdiction`) instead of carrying a
table of site strings. **Mid-refactor**: adapters that still declare a free-form
`states: Vec<String>` are the remaining cutover sites — new adapters must take the typed jurisdiction
and derive any host/URL from it, never a parallel string convention.

## Evidence rules

* Every canonical fact must carry `Evidence` with a `SourceRef::new("<namespace>", Some(key))`,
  where the namespace is the adapter's stable id and the key is the source's own stable object id
  (school id, meet id, athlete id) — never a name, never a position.
* `SourceNamespace` variants are declared in `crates/census-domain/src/model.rs`; add a variant
  there rather than smuggling a source tag through a `String` (ADR-003's rule about precision
  applies to source identity too).
* Evidence method matters: `EvidenceMethod::Parsed` for a document you parsed, distinct from
  derived/inferred facts. Never mark an inference as parsed.
* A source failure is **never** `NO_MATCH`. An adapter that cannot fetch a resource reports the
  failure in `AdapterReport` (and lets the workflow decide about a retry, ADR-002) instead of
  writing "not found" evidence.
* Non-core sources: the platform's own strongest source and its derivatives are excluded from the
  core census scope via `report::NON_CORE_SOURCE_IDS` / `is_core_source`. An adapter whose data
  cannot stand alone must be listed there so the core filter can ignore it.

## Traffic rules (non-negotiable)

* **No retries inside the adapter.** Adapters perform one attempt per requested resource, and never
  wrap a `fetcher` call in their own retry loop. The transport (`net/`) owns the only two retry
  paths: the loop over `FetchError::retryable` (`Transport`, `Timeout`, `RateLimited`, `Http` with
  status ≥ 500 or 429), at most `MAX_RETRIES = 3` attempts with ±25% jittered exponential backoff
  from `RETRY_BASE_DELAY_MS = 500`, and the conditional-GET path's retry of a `304` with no cached
  body (`net/execute/attempt.rs::replay_cached`). Restate/the workflow owns every other retry,
  including retrying a whole step (ADR-002). A second retry layer on top of those multiplies
  traffic against origins that do not care which layer caused it.
* All requests go through `ctx.fetcher`; never construct a client locally, never bypass robots
  checks, never spoof a browser user agent to evade an access control.
* Bounded concurrency only: stay within the crate's `CONCURRENCY_BOUND` unless the adapter
  declares and justifies something narrower. Per-host pacing belongs to `net/`.
* Respect access outcomes: a challenge/`403`/`429` is a terminal condition for that operation,
  recorded and surfaced, not something to grind through (see `docs/OPERATIONS.md`).
* Operator-authorized hosts are declared globally (`--authorized-host`, recorded as
  `robots_authorized` with the `MIN_AUTHORIZED_DELAY` floor). An adapter does not get its own
  authorization story.

## Idempotency

Re-running an adapter must be safe and must leave the store consistent:

* observations are **append-only** under `<table>\0<entity-id>\0<sequence:u64 big-endian>`
  (`crates/census-store/src/keys.rs`). A re-run appends new observations; it does not
  overwrite the old ones. Readers merge an entity's observations (`Store::scan`,
  `crates/census-store/src/read.rs`) through `Entity::merge` and then `Entity::publish`, and
  merge is idempotent and commutative — the set-valued fields (`source_identities`, `evidence`,
  `aliases`, `known_names`, `sports`, `source_urls`, `source_labels`) cannot double-count and a
  scalar a row already carries is never replaced. Two identical observations therefore cost bytes,
  not correctness;
* the journal is the real dedup: a completed unit of work is recorded (`Store::journal_done`) and a
  re-run skips it, so a resumed adapter re-requests only what it never finished;
* `refresh` only controls whether the network/parse step is repeated (the HTTP cache is the other
  gate); it does not change what a re-run appends;
* a crash mid-collection must leave the store consistent (partial evidence is fine; a half-written
  batch is not — a batch commits as one `SyncData` transaction,
  `crates/census-store/src/write.rs`).

## Reporting

Return an `AdapterReport` with real counters (schools/meets/athletes/requests/errors as the
adapter's shape implies) plus notes. The `provider` subcommand prints the whole report as pretty
JSON (`crates/census-service/src/cli/provider.rs`), and the gather stages print the notes line by
line for the operator (`crates/census-service/src/cli/cycle.rs`, `cli/gather.rs`); notes are the right
place for "artifact did not parse" style per-item failures so a parse failure is diagnosable
rather than silently counted.

## Tests and fixtures

* Unit tests live under the adapter's `#[cfg(test)] mod tests;` (a sibling `tests.rs` — `wiaa/mod.rs`
  ends with `mod tests;`), driven by embedded payloads or `crates/census-crawl/tests/fixtures/**`;
  **no test may touch the network**.
* Test the parse layer against real captured snippets (trimmed), including the shapes that break
  naive parsing: missing columns, `no mark` rows, wind/attempt columns, Unicode names, empty
  divisions.
* Test the registry/input parser for rejection cases as well as acceptance (a malformed registry
  line must fail loudly, not be guessed — see `athleticnet/tests.rs`'s registry tests).
* If your adapter is non-core, add a test asserting `is_core_source("<namespace>")` is false, so
  the core-scope guarantee cannot silently regress.

## Checklist for a new adapter

1. `<name>/` at the crawl crate root with `Options`, `Target`/subject type, parse layer, `collect`,
   tests.
2. Registry wiring (`crates/census-crawl/src/lib.rs` module list, `cli/provider.rs` arm and
   `ProviderArgs` doc list).
3. Namespace in `crates/census-domain/src/model.rs`; `NON_CORE_SOURCE_IDS` entry in
   `crates/census-domain/src/core_scope.rs` if the source is not core.
4. Evidence on every fact; source-native stable ids in keys.
5. No retries in the adapter; `CONCURRENCY_BOUND` respected; no local HTTP clients.
6. Idempotent observation writes; `refresh` handled; the journal records each completed unit.
7. `AdapterReport` counters + notes.
8. Fixtures under `crates/census-crawl/tests/fixtures/<name>/` if payloads are too large to
   embed; tests network-free.
9. `cargo fmt`, strict clippy via `tools/gate.sh` (zero new diagnostics),
   `cargo xtask source-test <name>`.
10. Report back: files touched, commands run, results, remaining uncertainty.
