# architecture.md — the census as it exists today

Current-state reference: what each crate is, which phase each subcommand advances, what the store
holds and how it survives a crash, what a seal certifies and refuses, and which module edges are
allowed. Every claim below names the file it comes from.

For the narrative view — execution model, the browser session supervisor, the census data model and
the planned workspace split — read `ARCHITECTURE.md` at the repository root. For planned work read
`docs/HARDENING-PROGRAM.md`; for the adapter contract read `SOURCE_ADAPTER_GUIDE.md`.

## 1. Crate map

Workspace members come from the root `Cargo.toml`: `crates/athleticnet-browser`, `crates/census-domain`, `crates/census-crawl`, `crates/census-reconcile`, `crates/census-report`, `crates/census-review`, `crates/census-store`, `crates/census-service`, and `xtask`.

| Crate / package | Path | What it is | Bins |
|---|---|---|---|
| `census-domain` | `crates/census-domain/` | pure domain model: canonical entities, deterministic ids, cohort identity, the school-name index and the core-scope predicate; normal dependency tree carries no async runtime, store engine, HTTP client, service framework or browser engine | — |
| `census-store` | `crates/census-store/` | the Fjall system of record: keyspaces, append-only observations merged through the `Entity` rules, snapshots, backup/restore/integrity, the legacy import, and the clock capability | — |
| `census-crawl` | `crates/census-crawl/` | the acquisition plane: robots-enforcing cache-first fetcher, the Restate-backed browser bridge, one module per provider, the provider registry | — |
| `census-review` | `crates/census-review/` | the local-model identity-review lane: retained families, model packets, verdict records | — |
| `census-service` | `crates/census-service/` | the composition root: the sweep and meet walk, orchestration (`src/census/`), reductions (`crates/census-report/src/{report,bests,workbook}/`, `crates/census-reconcile/src/index.rs`), durable services (`src/restate_services/`), supervisor (`src/bootstrap.rs`), CLI | `census-service`, `census-serve` |
| `xtask` | `xtask/` | developer commands: the gate wrapper, the gate's measurement layer, source fixtures/tests, census reports, adapter scaffolding | `xtask` |

Not cargo members of this workspace:

| Path | What it is |
|---|---|
| `fuzz/` | standalone cargo-fuzz workspace (`fuzz/Cargo.toml`), targets under `fuzz/fuzz_targets/` |
| `tools/` | `tools/gate.sh` (the one quality gate) and `tools/quality-baseline.json` (the debt ratchet) |
| `vendor/chromiumoxide_cdp/` | the one locally patched crate, wired through `[patch.crates-io]` in the root `Cargo.toml` |
| `deploy/systemd/` | the census service units: `census-serve@.service`, `restate-server.service`, `census-service-collect.{service,timer}` |
| `research/` | source reconnaissance lanes, append-only captures |

Census measurement targets are `crates/census-service/benches/core.rs`; the standalone harness
binaries live in `crates/census-service/examples/bench_store.rs` and `bench_census.rs`.

## 2. Acquisition phases

The phase ladder is `Phase` in `crates/census-service/src/census/state.rs` — `Discovering`,
`Acquiring`, `Reconciling`, `Reviewing`, `ResolvingGaps`, `Exporting`, `Complete` — entered one step
at a time (`CensusState::advance` refuses anything but the immediate next phase), with `Complete`
reachable only through `CensusState::seal`.

`crates/census-service/src/cli/seal.rs` (`reached_phase`) reads the phase back out of the store's own
artifacts, so the ladder is recorded progress rather than a caller's claim. The conditions are
monotone: a later artifact cannot exist without the earlier ones.

| Phase | Advanced by | Subcommands that produce it |
|---|---|---|
| Discovering | (an empty store; every census starts here) | `teams`, `meets`, `provider <name>`, `collect`, `run --input/--meets` |
| Acquiring | `StoreStats::observations > 0` | the same adapter commands, appending rows through `AdapterContext::write_batch` and filing observations through its `observe_*` funnels |
| Reconciling | `snapshots` holds ≥ 1 row | `index` writes that row; `consolidate` merges observations into `out/*.jsonl` |
| Reviewing | `review_cases` **or** `identity_verdicts` holds ≥ 1 row | `index` (derives the cases), `review` (records verdicts and closes the cases it decided) |
| ResolvingGaps | `coverage` holds ≥ 1 row | `index` (writes the per-jurisdiction coverage rows and their §47 gap classes) |
| Exporting | a `*.xlsx` exists in `<store>/out/` | `workbook`, `export`, `run` |
| Complete | `seal` returns `Ok` | `seal`, `seal --write` |

The commands themselves are the clap surface in `crates/census-service/src/cli/mod.rs`; `run`
parses, opens the store and dispatches. Four commands (`national`, `jurisdiction`, `national-report`,
`open-work`) are dispatched *before* the store opens, because they drive Restate and never read
it — and because opening the store takes the exclusive Fjall lock a live `census-serve` holds.

`index` is what advances three phases in one pass: it writes `source_identities`, `conflicts`,
`review_cases`, `coverage` (all through `Store::replace_many`) and the `snapshots` row
(`crates/census-reconcile/src/index.rs`). The gap classes the ResolvingGaps phase publishes are `GapClass` in
`crates/census-service/src/report/coverage/gaps.rs`, one row per class per jurisdiction, derived
from the counts the coverage row measured.

## 3. Store

Root default `var/census-service` (`--store`, global, in `cli/mod.rs`). `Store::open` creates `http/`
and `out/` if absent, opens the Fjall database under `fjall/`, and sweeps temporary files a dead
writer left behind (`read::sweep_stale_temporaries`).

```text
<store>/
  fjall/        the Fjall database (LSM tree): keyspaces entities, journal, meta
  http/         the fetch cache: <key>.body + <key>.meta.json, keyed by sha256(method, url, extra)
  out/          read-model outputs: report JSON/CSV, workbook .xlsx, seal.json
  entities/     pre-Fjall JSONL journals — one-time import source, never rewritten
  journal/      pre-Fjall resume ledger — one-time import source
```

Keys (`crates/census-service/src/store/keys.rs`):

```text
entities: <table>\0<entity-id>\0<sequence:u64 big-endian>   -> observation JSON
journal:  <phase>\0<key>                                    -> {key, at, payload}
meta:     <name>                                            -> small JSON/scalar
```

The sequence is a fixed-width big-endian tail, so byte order is numerical order and a prefix scan
reads one entity's history in write order. `observation_id` rejects an empty id and one over
`MAX_ID_BYTES` (512) before it can reach the key; `MAX_ROWS_PER_TABLE` is 20,000,000 observations
per table.

### 3.1 Tables

`Table` (`crates/census-store/src/table.rs`) is the store's naming for the fifteen collections; the names ride in
keys and sidecar file names.

| Table | Meaning | Written by |
|---|---|---|
| `schools`, `teams`, `coaches`, `athletes`, `meets`, `events`, `performances` | the canonical entities | adapters, appended (`append`/`append_many`) |
| `source_identities` | the §31 join from a provider's own id to a canonical row | `index`, replaced |
| `conflicts` | conflicts the merge retained: two rows one stored key says are the same subject | `index`, replaced |
| `review_cases` | findings the review lane owns, keyed so a repeated finding reuses its case | `index`, replaced; `review` closes the decided ones |
| `coverage` | coverage measurements per jurisdiction and per source namespace | `index`, replaced |
| `snapshots` | one row per finished pass over the store | `index`, replaced |
| `source_access` | access conditions a source imposed: one row per blocked `(kind, host)` | the sweep's access pass, replaced |
| `identity_verdicts` | adjudications the review lane recorded, one row per case | `review`, replaced |
| `source_meets` | meets a source enumerated, before their results were read | the `meets` command, appended |

Two write disciplines, not one:

* **Canonical entities are append-only.** An observation is never overwritten — appending the same
  entity twice writes two rows, and readers merge through `Entity::merge` (`Store::consolidate`,
  `crates/census-store/src/read/`). A transfer does not rewrite a prior school attribution.
* **The derived tables are not evidence.** Their rows are a function of the store as it stands, so
  they are written through `replace_many`/`replace`: one row per key, overwritten in place, so a
  re-derivation cannot grow them.

`Entity::publish` is the collection contract applied to the merged value, so the rule holds for the
report, the workbook, the snapshot and the Restate handlers at once; `withheld_mailboxes` counts what
that contract withheld, summed in the same pass as the snapshot.

### 3.2 Durability

Fjall is an embedded LSM-tree store: writes land in a write-ahead journal and a memtable and are
compacted into immutable sorted tables, so an interrupted run costs at most the observations never
flushed — never a rewritten snapshot.

| Property | Value | Source |
|---|---|---|
| commit mode | `PersistMode::SyncData` (`fdatasync`) per batch | `crates/census-store/src/lib.rs` |
| upgrade | `Store::flush()` → `PersistMode::SyncAll`, called at consolidation and shutdown | `crates/census-store/src/lib.rs` |
| block cache | 256 MiB (`CACHE_BYTES`) | `crates/census-store/src/lib.rs` |
| sequence seeding | from the last key present at open, so a reopened database never reuses a sequence or overwrites an observation | `crates/census-store/src/lib.rs`, `crates/census-store/src/sequences.rs` |
| resume journal | durable per completed unit of work; the journal keyspace holds `<phase>\0<key>` | `crates/census-store/src/keys.rs` |
| legacy import | `Store::open` imports the pre-Fjall `entities/` and `journal/` JSONL exactly once, recorded under `meta` | `crates/census-store/src/legacy.rs` |

The store API also owns `Store::backup`, `Store::restore` and `Store::integrity`
(`crates/census-store/src/backup/`): `backup` copies the durable material and writes a `backup.json` manifest of
file digests, byte lengths and per-table counts taken from the same sequence counters `Store::stats`
reads; `restore` validates that manifest and re-opens through the normal `Store::open` path;
`integrity` checks each table's row count against its sequence counter plus journal/entity-log
pairings. `Store::stats` (`crates/census-store/src/lib.rs`) is what a status command prints: per-table counts from
those counters, LSM bytes on disk, and the recursive store size.

## 4. Seal

`census-service seal` is the one place the pipeline is allowed to call a census finished
(`crates/census-service/src/cli/seal.rs`). It assembles its evidence from what the store, the
classifier and the exported workbook already hold — nothing is passed in as a claim.

Scope is the core scope unless `--all-sources` is given; the cohort defaults to `--grad-year 2027`.

### 4.1 What it certifies

`SealCounts` (`census/state/evidence.rs`): `jurisdiction_buckets` (the rows the census's state rollup
publishes: one per covered jurisdiction plus the unplaced row), `schools`, `meets`, `athletes`,
`class_of_2027`, `performances`, `coaches`. With `--write` the sealed state is persisted to
`<store>/out/seal.json`; a seal recorded by an earlier run is printed and then *checked against this
run's evidence*, never trusted — a difference is reported, not resolved in either direction.

### 4.2 What it retains

Findings are not blockers. `RetainedFindings` carries `gaps` (one `GapTally` of `class`, `unit`,
`count` per §47 class), `conflicts`, `access_conditions` (the `source_access` rows: the store's own
count of hosts that blocked or throttled a lane, which is **not** §70's *retry-exhausted operation* —
that is an invocation state on the durable side and no store row records it), `source_failures`,
`observations` and `calculations`. A non-zero gap tally seals fine and stays visible inside the
seal. The field carried the name `retry_exhausted` until the two were told apart; the digest prefix
moved to `census-seal-v3` with the rename so a `v2` digest cannot be read as this one. The state
rollup's row count carried the name `jurisdictions` until the unplaced row it also counts was told
apart; the prefix moved to `census-seal-v4` with that rename, so a `v3` digest cannot be read as this
one.

### 4.3 What it refuses

`SealEvidence::open_items` returns the §70 acceptance items that are unmet, and `CensusState::seal`
refuses with `SealError::ItemUnmet` naming the first one plus the number behind it. A refusal prints
the item and its detail, then exits non-zero.

| Refusing item | Unmet when |
|---|---|
| `JurisdictionSweepsTerminal` | `open.jurisdiction_sweeps > 0` |
| `SourceObjectsTerminal` | `open.source_objects > 0` |
| `CohortDecisionsTerminal` | `open.cohort_decisions > 0` |
| `IdentityCandidatesTerminal` | `open.identity_candidates > 0` (retained cases with no verdict, counted from the rows) |
| `EvidenceDurable` | `observations == 0` while `athletes > 0` |
| `CalculationsReproducible` | `calculations == 0` while `performances > 0` |
| `WorkbookMapped` | `mapped_athletes < class_of_2027` |
| `WorkbookCountsReconcile` | the workbook's counts do not reconcile with the store |
| `CoverageReportReconciles` | the coverage sheet does not carry every jurisdiction the classifier produced |
| `RunMetricsReconcile` | the run-metrics sheet does not name the cohort the store counted |
| `ExportVerified` | the export check failed, or any discrepancy was recorded |

`AcceptanceItem::ConflictsRetained` and `AcceptanceItem::RetriesRepresented` are in the §70 list but
never appear in `open_items`: they demand that a finding be *kept*, and a seal refused over a kept
finding would push an operator to hide one. They are satisfied by the seal recording the count.

The workbook check (`cli/seal/workbook.rs`) requires the sheets `Athletes`, `Coverage` and
`Run Metrics`; it hashes the file's bytes, and materialises only the two meta sheets — a census
workbook holds a million rows per sheet, so cell-level counting is deliberately not attempted.
Renaming a required sheet, a jurisdiction missing from `Coverage`, or a cohort number the
`Run Metrics` row does not match all come back as a named discrepancy.

One honest limit: the CLI seal carries what the store holds. `jurisdiction_sweeps` and
`source_objects` are `None` — unmeasured, deliberately not `0` — because owed sweeps and un-terminal
source objects live in the durable run's own objects, which a command holding the store's single
writer cannot ask; `census-service open-work` is the read that does. `cohort_decisions` is measured
here, from the store's retained cases: a case in a cohort family (`COHORT_UNVERIFIED_FAMILY`,
`COHORT_IDENTITY_CONFIDENCE_FAMILY`) with no verdict is an open cohort decision, and `Retained` is a
terminal one. Both families are decided by the rule the row itself is read through — the row is
published at the confidence its observations support — so `ReviewCase::minted` starts their cases
`Retained` (`ReviewCase::decided_by_its_own_rules`) and the item only ever counts a case a lane left
genuinely open. Evidence that arrives later closes the finding by deriving it away, which the next
pass records as `Superseded`. `identity_candidates` is the same scan without the family filter: every
retained case with no verdict, because the item is about the decision and only a verdict is one.

## 5. Module seams

The census stays a single crate with module seams (`docs/HARDENING-PROGRAM.md` §9), so the compiler
seals items but cannot forbid an edge between top-level modules. `cargo xtask seams`
(`xtask/src/seams.rs`) reads every production `.rs` file under `crates/census-service/src`, resolves
each `crate::…` reference to its top-level module, and compares the `(from, to)` pair against the
table in that file. The gate runs it as the "module seams" lane.

* **Direction rule.** Adapters and workflows depend on domain types and on the store, never the
  reverse. `net` and `school_index` are leaves; `store` and `report` may not reach into `net` (the
  clock lives in `clock`).
* **The table is the ratchet.** Adding an edge is a deliberate edit to `ALLOWED` in
  `xtask/src/seams.rs`; deleting a row makes that edge a violation again, because the check fails
  closed. A violation names the file and line and exits non-zero.
* **Known exception, listed on purpose.** `(sources, store)` is allowed although `ARCHITECTURE.md`
  calls it a direction violation: `AdapterContext` carries `&Store` today. When adapters return
  entity batches instead, delete the row and the walker enforces the narrower graph.
* **Test code is out of scope.** Files named `tests.rs`, files under a `tests/` directory, and the
  region after the `#[cfg(test)]` that opens a module cannot reach production callers.

Domain purity is the matching check inside the domain crate: `cargo xtask domain-purity`
(`xtask/src/purity.rs`) resolves `cargo tree -p census-domain --edges normal` and fails if any of the
banned packages appear — `tokio`, `fjall`, `reqwest`, `serde_json`, `restate-sdk`, `chromiumoxide`,
`hyper`, `axum`, `tower`, `mio`, `h2`, `rustls`, `openssl`, `socket2`, `tungstenite`, `tokio-util`
and their relatives. Normal edges only, so a dev-dependency cannot taint the verdict.

## 6. Where the numbers come from

| Question | Command | Reads |
|---|---|---|
| what does the store hold? | `census-service fjall-stats` | `Store::stats` |
| what is the census? | `census-service report [--core] [--print]` | `report::build_census` |
| what is missing, per jurisdiction? | coverage rows + §47 gaps written by `index` | `report::coverage_report` |
| is the export consistent with the store? | `census-service seal` | `cli/seal/workbook.rs` |
| is the tree inside its budgets? | `cargo xtask scan`, `cargo xtask ratchet` | `xtask/src/scan.rs`, `xtask/src/baseline.rs` |
| are the module edges allowed? | `cargo xtask seams` | `xtask/src/seams.rs` |

A performance claim may cite only a committed benchmark target; see `PERFORMANCE.md`.
