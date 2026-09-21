# SOURCE_ADAPTER_GUIDE.md — how to add a source adapter to the census

An adapter turns one external source (state association, timing provider, aggregator, directory)
into `Evidence` and canonical observations in the Fjall store. This document is the contract an
adapter author must satisfy; `DOMAIN.md` covers the types, `ARCHITECTURE.md` the pipeline shape,
`FJALL_SCHEMA.md` the storage keys.

## Where an adapter lives

```text
crates/midwest-census/src/sources/<name>.rs        the adapter (collect + parse + tests)
crates/midwest-census/src/sources/mod.rs           `pub mod <name>;`
crates/midwest-census/src/main.rs                  dispatch arm under `Command::Provider`
```

Registration is three edits, no more:

1. `pub mod <name>;` in `sources/mod.rs` (keep the alphabetical block).
2. A `"<name>" => providers::<name>::collect(&context, &providers::<name>::Options { .. }).await`
   arm in `main.rs`, mapping the shared `ProviderArgs` fields.
3. Add `<name>` to the adapter-name doc comment on `ProviderArgs` (`main.rs`), which is also the
   list the `unknown adapter` error prints.

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

## Evidence rules

* Every canonical fact must carry `Evidence` with a `SourceRef::new("<namespace>", Some(key))`,
  where the namespace is the adapter's stable id and the key is the source's own stable object id
  (school id, meet id, athlete id) — never a name, never a position.
* `SourceNamespace` variants are declared in `crates/midwest-census/src/model.rs`; add a variant
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

* **One attempt per operation.** No retries inside the adapter, no retry loops around `fetcher`
  calls — Restate/the workflow owns retries (ADR-002). Layered retries multiply traffic against
  origins that do not care which layer caused it.
* All requests go through `ctx.fetcher`; never construct a client locally, never bypass robots
  checks, never spoof a browser user agent to evade an access control.
* Bounded concurrency only: stay within the crate's `CONCURRENCY_BOUND` unless the adapter
  declares and justifies something narrower. Per-host pacing belongs to `net.rs`.
* Respect access outcomes: a challenge/`403`/`429` is a terminal condition for that operation,
  recorded and surfaced, not something to grind through (see `docs/OPERATIONS.md`).
* Operator-authorized hosts are declared globally (`--authorized-host`, recorded as
  `robots_authorized` with the `MIN_AUTHORIZED_DELAY` floor). An adapter does not get its own
  authorization story.

## Idempotency

Re-running an adapter must be safe and must not duplicate observations:

* observations are keyed by source identity + evidence digest, so a re-run overwrites rather than
  appends a second copy;
* `refresh` only controls whether the network/parse step is repeated;
* a crash mid-collection must leave the store consistent (partial evidence is fine; duplicates are
  not).

## Reporting

Return an `AdapterReport` with real counters (schools/meets/athletes/requests/errors as the
adapter's shape implies) plus notes. `main.rs` prints notes for the operator; notes are the right
place for "artifact did not parse" style per-item failures so a parse failure is diagnosable
rather than silently counted.

## Tests and fixtures

* Unit tests live in the adapter file under `#[cfg(test)]`, driven by embedded payloads or
  `crates/midwest-census/tests/fixtures/**`; **no test may touch the network**.
* Test the parse layer against real captured snippets (trimmed), including the shapes that break
  naive parsing: missing columns, `no mark` rows, wind/attempt columns, Unicode names, empty
  divisions.
* Test the registry/input parser for rejection cases as well as acceptance (a malformed registry
  line must fail loudly, not be guessed — see `athleticnet.rs`'s registry tests).
* If your adapter is non-core, add a test asserting `is_core_source("<namespace>")` is false, so
  the core-scope guarantee cannot silently regress.

## Checklist for a new adapter

1. `sources/<name>.rs` with `Options`, `Target`/subject type, parse layer, `collect`, tests.
2. Registry wiring (mod.rs, main.rs arm, `ProviderArgs` doc list).
3. Namespace in `model.rs`; `NON_CORE_SOURCE_IDS` entry if the source is not core.
4. Evidence on every fact; source-native stable ids in keys.
5. One attempt per operation; `CONCURRENCY_BOUND` respected; no local HTTP clients.
6. Idempotent observation writes; `refresh` handled.
7. `AdapterReport` counters + notes.
8. Fixtures under `tests/fixtures/` if payloads are too large to embed; tests network-free.
9. `cargo fmt`, strict clippy (zero new diagnostics), `cargo nextest run -p midwest-census`.
10. Report back: files touched, commands run, results, remaining uncertainty.
