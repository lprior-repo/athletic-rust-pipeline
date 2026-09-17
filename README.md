# Native athlete evidence pipeline

Rust workbook ingestion, Athletic.net candidate discovery and evidence parsing, with native Restate orchestration and local-only model review. The executable exposes `worker`, `deploy`, `start`, `status`, `export`, and `verify`. Earlier CLI commands are no longer the production interface.

## Identity and eligibility

The source workbook is the golden input for membership and source identity context. Every source row remains in output accounting, including duplicate rows and rows missing identity fields. Workbook membership establishes eligibility: there is no junior, grade, or graduation-year requirement. Observed graduation information is descriptive, not a selection or ranking criterion.

Discovery uses names and available school/city/state context and searches both track-and-field and cross-country. A missing school does not prevent discovery. Acceptance remains conservative: missing corroboration or conflicting evidence produces review, not a guessed match. A mailing address is not proof of school geography or candidate residence. Source sport is context, not proof of Athletic.net participation; unknown or blank sport does not exclude a row.

School comparisons normalize a trailing “High School” without fuzzy matching or changing source values; the generic value “High School” is not a distinguishing school identity. Compatible partial location observations retain their own evidence references. City and region may corroborate across observations of the same team ID, never across different teams or through an invented combined-location witness. Conflicting overlapping fields remain contradictions. Bio parsing validates event/distance metadata only for the requested sport.

All original fields are retained. The assigned local reviewer receives the complete source record and structured candidate evidence. Email, street and postal information must not be sent to external search services or remote development models. Such fields are context only unless independently corroborated; email domains, ratings and source metadata do not themselves prove a candidate identity. Source cells and retrieved text are untrusted data, never model instructions.

Rust performs routine parsing, arithmetic, mark comparison, and deterministic matching. Only genuine ambiguity reaches one assigned local model. Different cases are assigned across the existing Q5/5090 and Q4/3090 servers; no two-model consensus is required. A model cannot override deterministic contradictions or manufacture evidence.

Source acquisition uses `reqwest`; HTTP methods, admission and durable retry policy stay in the native runtime. Profile and search extraction use `lol_html` streaming handlers instead of a materialized DOM. Parser-internal accounted memory is capped at 8 MiB, with separate input and evidence-capture bounds; this is not a total-process memory limit. Source responses are bounded at 32 MiB, with incomplete bodies rejected. Parser revision changes invalidate parsed evidence while preserving compatible raw HTTP receipts.

Every discovered athlete remains in explicit `Complete`, `Incomplete`, or `NameExcluded` coverage. The initial probe fully parses both sport-specific Bio responses and profile HTML, retaining failures, receipts, retry evidence and authorized follow-up requests. Only source-bound proof of a different name can skip TeamNav expansion: raw TF/XC name components and scoped HTML hints must agree before comparison with the original source name. Search display names, missing hints, conflicting identities or failed initial acquisition cannot establish exclusion. A later full acquisition reuses the same probe; it does not repeat the initial HTTP work or reset its failure history.

Exclusion is specific to the source row's canonical name, not a global negative cache for the athlete. Normalization treats punctuation as separators rather than deleting it: `O'Neil` and `O Neil` agree, but `ONeil` differs. Only complete full acquisitions may become eligible candidates, reach model selection, or supply selected performance summaries.

Observed-best summaries retain event/timing/wind/equipment context and evidence references. Opaque numeric best flags remain unknown rather than being interpreted as verified PR claims. A summary too large for an Excel cell is explicitly relocated to the full JSONL sidecar with row/report/athlete linkage; it is not truncated.

## Build and checks

```sh
cargo build --release
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo bench --bench pipeline
cargo audit
cargo deny check
cargo deny check advisories licenses sources --deny warnings
cargo deny --manifest-path fuzz/Cargo.toml --config deny.toml check advisories licenses sources --deny warnings
```

`fuzz/` contains bounded parser and evidence-boundary fuzz targets. Use the pinned nightly toolchain with `cargo fuzz`; for example, `cargo fuzz run bio_json -- -max_total_time=60 -max_len=262144 -rss_limit_mb=2048 -timeout=10 -seed=4202`. Preserve seed corpora and record the exact target, sanitizer, seed, input bounds, duration and exit status. Passing ordinary tests or a short fuzz campaign does not establish recovery correctness, real-world identity accuracy, or full-population readiness.

The model-response and retained-results fuzz targets execute fixed positive and negative witnesses before fuzzing arbitrary bytes. A valid corpus file alone is not an acceptance oracle. Keep retained-result witnesses in the exact production JSON serialization, including typed artifact digests; regenerating them through a different JSON serializer can invalidate their representation.

Seed retained-result campaigns from `fuzz/fixtures/retained_results_jsonl`, not only an old generated corpus. Native audit captures have fresh UUIDs, so the harness binds committed synthetic digest references to its process-local captures using equal-length byte replacements. It does not parse and reserialize arbitrary inputs: malformed JSON, duplicate keys, and mutations remain intact. Both committed and hydrated positive witnesses must pass the production verifier before arbitrary inputs are processed.

The Criterion suite uses synthetic source data. Fresh publication/persistence and idempotent reuse have separate benchmarks; fresh-store setup and teardown are outside the timed operation. Successful parsing, accepted-decision fixtures and storage results are checked rather than silently benchmarking errors. These microbenchmarks are not live source throughput or a GPU benchmark.

Storage benchmarks use `tempfile`, so record the filesystem selected by `TMPDIR`. On this workstation `/tmp` is tmpfs, while the private production directory is on Btrfs. Set `TMPDIR` to a dedicated directory on the intended filesystem when measuring persistent writes; do not interpret tmpfs timings as disk persistence performance.

## Native services

Use native Restate, not Docker. `tools/restate-native.sh start` launches the already-installed pinned Restate 1.7.10 binary and configuration under `${XDG_DATA_HOME:-$HOME/.local/share}/athletic-rust-pipeline/restate`. It requires an existing installation and persistent data directory; it is not an installer.

`config.native.toml` contains this workstation's live configuration: private artifact storage, Athletic.net origin and admission interval, bounded CPU/row work, and the two existing local model endpoints. Adjust paths and model identifiers for another workstation. Do not commit private inputs, credentials, model prompts, or generated athlete artifacts.

`source_interval_ms = 0` disables fixed admission spacing; this workstation uses zero. Restate's `athletic-source` scope has a deployment ceiling of 16 concurrent shared HTTP operations, independently of row concurrency; stricter operator-set limits are preserved. Total request volume is not a concurrency setting. Already-admitted operations may finish or use their remaining SDK retries after a later denial; this is not instantaneous revocation of physical requests.

Native flow control is an opt-in Restate feature. The pinned 1.7.10 server must have all three settings enabled in its private `restate.toml`:

```toml
experimental-enable-protocol-v7 = true
experimental-enable-vqueues = true
experimental-enable-scoped-virtual-objects = true
```

`deploy` checks the server version and advertised capabilities before registration. It conditionally creates the exact source rule at `min(16, enabled wildcard limit)`, rejects disabled, unlimited or excessive exact limits, and checks the confirmed rule against a stricter wildcard. To tighten a running deployment, change the exact `athletic-source` rule: Restate's exact rules override `*`. See [Restate flow control](https://docs.restate.dev/services/flow-control).

The separate `athletic-source-control` scope serializes only admission-state transitions, never HTTP or cooldown timers. Callers perform durable waits and recheck admission; 64 changing-deadline observations end in retained review rather than an unbounded loop. Denial/cooldown feedback is sent as a detached native invocation and then attached for acknowledgement, so cancelling the caller does not cancel the policy update.

Before enabling scopes on an existing installation: cancel and drain old invocations, export through the artifact-owning worker, stop both worker and Restate, and preserve both data directories plus their configurations as one checkpoint. Enable the flags only after that checkpoint; use a fresh worker endpoint for changed journal paths. Deployment refuses any retained `SourceGateway/global` blocked state, including the old unscoped object. Do not clear a denial merely to force deployment, or restore only one half of the checkpoint.

```sh
cargo run --release -- worker --config config.native.toml --bind 127.0.0.1:19181
cargo run --release -- deploy --admin http://127.0.0.1:19070/ --endpoint http://127.0.0.1:19181/
```

Keep worker, Restate, and model services local. SDK journals and artifact storage contain sensitive source context and require private storage. Retain Restate's persistent data directory across restarts. Do not restart or reconfigure user-owned model servers as part of pipeline deployment.

## Submit, inspect and export

Record the original SHA-256 before starting. Keep the original unchanged and choose new output paths.

```sh
cargo run --release -- start \
  --input /absolute/path/to/original.xlsx \
  --sha256 ORIGINAL_SHA256 \
  --per-sheet 50 --concurrency 4 \
  --snapshot pilot-source-v1 --execution pilot-v1

cargo run --release -- status --run RUN_DIGEST
cargo run --release -- export --run RUN_DIGEST --output /absolute/new/results.xlsx
cargo run --release -- verify \
  --input /absolute/path/to/original.xlsx \
  --output /absolute/new/results.xlsx \
  --sha256 ORIGINAL_SHA256 \
  --store /absolute/path/to/stopped/artifacts
```

`--per-sheet` selects a bounded number from each source sheet; `--all` requests all source rows. Submission is not completion. Inspect coverage and verify outputs before expanding a real pilot. Reuse a source snapshot only when its evidence contract is unchanged; cached observations are snapshot-scoped. Changed source fixtures require a new snapshot.

Export publishes an XLSX, a detailed JSONL sidecar, and a commit receipt. Original source fields and row positions are preserved; annotation columns are appended. Partial exports retain explicit pending rows. Destinations are non-clobbering and bound to one run. The commit receipt is published last: the files are not an atomic multi-file transaction. Treat a missing receipt as an incomplete publication.

Export verifies through the worker that owns its artifact store. Standalone `verify --store` requires the existing stopped database, not an empty directory; never open or copy a live Fjall directory from another process. An artifact-directory copy alone is not a coordinated Restate rollback image.

The independent `verify` command preflights both workbooks before sparse-cell readback, checks source hashes, original fields, source-sheet order and row accounting in the actual XLSX, then checks retained JSONL result evidence and binds every workbook annotation, including performance summaries, to its sidecar row. Sidecar source fields must have exactly the original column keys and values; annotation columns cannot substitute for original fields. Typed report, assessment and profile digests and retained-performance projections must agree. Positive checks cover selected identity, retained profile/document references, attributed participation, complete discovery and deterministic uniqueness. Review rows may retain conflicts without becoming positive results. Source, XLSX and sidecar hashes are checked for changes during verification. **This establishes retained-evidence consistency, not source authenticity or unknowable real-world identity accuracy; PR arithmetic and raw-source authenticity are not independently proved by this command.**

Embedded report, assessment, discovery, probe and full-acquisition metadata must match their hash-verified bytes in the artifact store. Discovery is reconstructed from the deterministic query plan and raw search pages, including pagination, candidate union, issues and completeness. Every page is bound to its indexed captured request, including POST method, exact query/filter/offset and frozen origin. Cache-equivalent case/stage variants use the actually executed retained query, never a substituted request. Complete profiles are reconstructed from raw TF/XC Bio, HTML and the exact ordered, bounded TeamNav requests authorized by those initial documents; their successful probe and receipt/operation prefixes must agree. Name exclusions independently reparse their raw initial documents. Missing bodies, altered bytes, omitted authorized operations and rehashed metadata that contradicts raw evidence fail verification. Parser-valid successful 2xx responses remain valid regardless of MIME.

Terminal query failures are bounded to one and cannot follow completed pagination. A `MalformedResponse` must bind the exact captured successful search operation and reproduce a raw parsing failure or the final page's pagination-reconciliation failure. Worker drain, CPU admission and task-join failures are classified separately as `ArtifactFailure`, since their cause need not recur during offline parsing. Global admission failures may legitimately retain the original denial receipt from a different query.

Every retained assessment is recomputed through the production decision rules over all candidate coverage, including the cumulative metadata budget; this applies to accepted, no-match and review outcomes. Local-review acceptance additionally applies the production selection-authorization rule. Rehashed scores or candidate flags cannot bypass those rules. A verified relay-member result identifies the relay separately from its member; the relay ID is not required to equal the selected athlete ID. Incomplete acquisitions may remain review evidence without claiming full raw reconstruction. Local capture records are trusted acquisition evidence, not cryptographic authentication of the upstream server or proof that its career corpus is exhaustive.

The `captured-source-request-response-v3` acquisition contract and `streaming-source-parsers-v12` parser revision invalidate older cache identities. Old observations cannot acquire missing request provenance retroactively: retain the compatible frozen worker for historical exports, and use a fresh snapshot/deployment for the new contract. Never replay in-flight journals against a changed worker.

Restate owns durable calls, cached workflow results, operation retry policies and orchestration. An HTTP run allows at most an initial attempt plus three SDK retries, but any HTTP 429 immediately leaves that retry loop. A usable positive `Retry-After` publishes a durable global admission cooldown; a 429 without one latches the source block instead of amplifying throttling through automatic retries. Other retryable responses with a positive `Retry-After` also leave the retry loop and publish the cooldown. These outcomes retain failure evidence for review, without a separate application retry loop. `observed_attempts` counts retained completed-attempt records, not remote request starts: records are published after response handling. An external response not acknowledged before a crash can be repeated. There is no exactly-once HTTP guarantee, and workflow-wide request totals are not per-operation retry counts.

### Recovery and paused invocations

Restore the same worker build and artifact directory for an in-flight recovery, and retain Restate's data directory. A paused invocation is durable: restarting the worker or Restate does not automatically resume it. In the controlled worker-kill exercise, transport retries exhausted while the worker was unavailable and paused `SourceGateway/global/fetch`; queued dependents could not progress until that invocation was explicitly resumed.

Inspect invocation status and failure history through the native Restate administration API. After restoring the worker and diagnosing the cause, resume the actual paused dependency, not merely its waiting parent:

```sh
curl --fail-with-body --request PATCH \
  http://127.0.0.1:19070/invocations/PAUSED_INVOCATION_ID/resume
```

This is an operator-controlled recovery action, not an unbounded retry loop. Preserve failure history and account for uncertain in-flight HTTP effects. A separate controlled Restate-server restart recovered automatically from its existing data directory. Both recovered synthetic runs completed all eight rows and passed workbook verification; subsequent exact replays added zero source/model requests. The worker-kill result is **operator-assisted recovery**, not proof of automatic worker-only recovery.

The scoped source path was also exercised against native Restate: a 27-row synthetic run retained 8 accepted, 3 no-match and 16 review outcomes, with a fixture-observed peak of 16 active HTTP requests. Exact replay added no source or model requests. A one-second `Retry-After` delayed a fresh run, and a longer cooldown survived caller cancellation and restart of both native services. A queued detached 403 observer survived cancellation before acknowledgement; fresh runs before and after restart issued no HTTP or model requests and remained review-only. A separate in-flight worker kill recovered after an operator restart, with 31 proxy request starts over 30 request identities, including one repeated read. These are scoped fixture observations, not a physical exactly-once or universal retry-count guarantee.

A native no-`Retry-After` 429 reproduction exposed 80 physical HTTP requests across 20 failed operations in the old retry policy, with no source block. The corrected worker made one HTTP request, retained the 429 block, and made zero additional source/model requests on fresh snapshots, exact replay and a restart of both native services. All three full synthetic exports retained 27 rows and all 405 original fields. This proves stop behavior, not sustained upstream capacity. The live higher-concurrency run was stopped and coherently rolled back after throttling; its higher completed-operation rate included failures and is not a useful-throughput speedup.

Additional in-flight exercises restored the same frozen worker after interrupting source HTTP and an actual Q5 review. Both required explicit resumes of paused dependencies. The source run recovered one accepted row; its 18 requests covered 13 logical request identities, each observed at most twice. The model proxy completed two requests with identical bodies to the same assigned Q5 model, losing the first response before worker acknowledgement; the application retained one completed-attempt record. The unresolved review remained non-positive, with no second-model consensus. Independent CLI verification passed both recovered exports. These observations account for the lost-acknowledgement window; they do not establish automatic recovery or a physical-request ceiling across crashes and operator resumes.

An additional in-flight cancellation left its source row explicitly pending, with no accepted or review result. Two source operations had started before cancellation acceptance; the second completed afterward. Native invocation drainage and independent verification of the partial export both passed.

Exact replay requires the same input digest, source snapshot label, execution label and selection/concurrency arguments. Changing or omitting the snapshot label can submit different work. Upgrade CLI and worker cache protocols together; do not replay in-flight journals against incompatible parser or orchestration contracts.

### Cancellation and worker cutover

Cancel a run through its actual native invocation ID:

```sh
curl --fail-with-body --request PATCH \
  http://127.0.0.1:19070/invocations/RUN_INVOCATION_ID/cancel
```

Cancellation acceptance is not drainage. The coordinator cancels admitted row calls and drains its pending native futures; row and effect handlers propagate native cancellation instead of publishing ordinary review results or blocking source admission. In the controlled cancellation exercise, the root and eight admitted rows terminated with cancellation, no row reports were published, and exactly one already-started HTTP request occurred. A subsequent run completed and verified the synthetic workbook.

Before changing an executable or its configuration, stop admission and confirm the old deployment's invocations and external requests have drained. Do not hot-replace an incompatible worker behind the same endpoint. Preserve a verified partial export and invocation history. Keep one owning worker per artifact directory; stop it before an offline directory backup. Reuse raw source snapshots only when their acquisition contract remains valid, and advance parsed-evidence/row revisions when those contracts change.

Use a fresh endpoint URI for a new frozen worker build and inspect the registered service handlers before submission. Reposting an already registered URI can return its old deployment manifest without rediscovering changed handlers. A controlled stale-manifest submission left paused rows that required explicit operator termination after cancellation; cancellation acceptance alone did not clear those paused rows.

## Synthetic native exercise

```sh
cargo run --example native_fixture -- \
  --bind 127.0.0.1:18081 \
  --output-dir /absolute/new/synthetic-fixture \
  --scenario match,duplicate,ambiguous,missing-cohort,conflict,empty-search,malformed,retry-exhaustion,payload-limit,split-location,generic-school
```

The fixture writes a synthetic workbook and worker configuration, and serves controlled source/model responses and request counters. Run its worker against a dedicated native Restate instance. Fixture model endpoints are synthetic; they do not exercise the real GPUs. Keep generated fixture inputs stable during a run: fixture startup currently regenerates its workbook, changing its digest.

Observed native geography repair exercise: four complementary-location rows changed from review to accepted, and a previously accepted generic-school row changed to review. The completed 13-row run produced 5 accepted, 1 no-match and 7 review results. Both Bio/TeamNav and cross-sport corroboration retained separate document witnesses; mixing different team IDs remained rejected. The other scenarios cover malformed metadata for the unrequested sport, missing/non-2027 cohorts, identity contradictions, genuine ambiguity, malformed responses, retry exhaustion and oversized responses. Export verified all 195 original fields across two sheets and unchanged source SHA-256. Parser cutover added zero source requests and one synthetic model request for changed review evidence; exact run replay added neither source nor model requests.

The separate actual-GPU exercise used two distinct synthetic source records, one per reviewer. Both models returned unresolved for indistinguishable candidates, each in one HTTP attempt. Retained request/response audits verified all 15 source fields and both eligible candidates in each request, and matching actual model IDs. Observed HTTP times were 789 ms for Q5/5090 and 1,524 ms for Q4/3090; each request had 2,273 prompt tokens. The native two-row run completed in 2,470 ms and verified all 30 original fields. Exact replay created no additional reviewer invocations. These are measurements of two controlled cases, not a load or soak benchmark.

Moving those same field values to differently named worksheets in a separate workbook reused both original model responses without another reviewer invocation. The relocated export independently preserved all 30 source fields. Case identity binds complete field values and candidate evidence, not physical worksheet/row coordinates.

Completed-effect recovery was also exercised after a native Restate process restart and, separately, forced termination (`SIGKILL`) of an idle worker. A fresh coordinator execution after worker recovery reused retained source/model work and exported both rows with all 30 original fields. Neither exercise repeated a source or model call. These checks do not establish exactly-once behavior for uncertain in-flight HTTP effects.

The bounded-HTML exercise served a valid 9,437,374-byte document containing an oversized attribute: below the 32 MiB HTTP body limit, but above the streaming parser's 8 MiB accounted-memory limit. Both native rows retained a parser failure and required review, with no model outcomes; export verified all 30 original fields. A direct production-parser diagnostic accepted the equivalent small document and reported memory-limit exhaustion for the large one. The parser setting is not a whole-process RSS limit.

The identity-acquisition exercise completed 27 synthetic rows with 8 accepted, 3 no-match and 16 review results, preserving all 405 original fields. A focused 26-row cold comparison used 378 source calls instead of 394: all 16 targeted TeamNav expansions were avoided, while the failed TF probe still made four attempts rather than being retried again by full acquisition. Warm execution and exact completed replay added no source or model calls. The hardened verifier accepted the native workbook and rejected ten targeted metadata/context mutations plus seven missing/corrupt artifact controls; restoring the isolated store recovered the original 27-row result counts.

The captured-request verifier was exercised on a fresh 27-row native run and again with every successful source response rewritten to HTTP 201 and `application/octet-stream`. Both independently verified exports retained 8 accepted, 3 no-match and 16 review results and all 405 original fields. Exact completed replay issued zero additional source/model requests. Eight coherently rehashed attacks were rejected: forged assessment, removed raw-backed results, omitted TeamNav work, omitted planned query, and changed captured query, filter, offset or HTTP method. A separate worker `SIGKILL` at 26/27 completed rows required explicit native resume of five paused invocations on the same frozen deployment, then recovered the same results; observed source concurrency peaked at 16. This is operator-assisted recovery, not exactly-once HTTP. Five warm release-CLI verification measurements of the 27-row bundle had a 26.3 ms median (25.3–27.2 ms range); this is a small synthetic offline-verification measurement, not full-workbook or live-acquisition throughput.

The final parser-failure repair rejected two additional rehashed forgeries that the prior frozen verifier accepted: a claimed malformed query without response evidence, and a claimed malformed query whose captured response parses successfully. Both had previously downgraded one valid result to review, not fabricated acceptance. The repaired native build again completed all 27 rows with the same 8/3/16 outcomes; cold execution made 404 source requests and one synthetic model request, and exact replay added neither. Its five warm independent CLI checks measured a 25.6 ms median (24.7–30.1 ms range). The captured-request 429 exercise also retained exactly one physical request across a fresh blocked snapshot, exact replay and restart of both native services; all three partial exports independently preserved 27 rows and 405 fields, with two review rows and 25 pending.

Run access-denial scenarios separately: denial deliberately halts global source admission, so it can make other concurrent rows require review. Synthetic evidence is not the real 100-row pilot or completed full-workbook delivery. Live rollout, raw-source/PR verification, recovery/failure campaigns, performance and security acceptance remain separate requirements.
