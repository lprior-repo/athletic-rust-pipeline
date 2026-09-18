# Native athlete evidence pipeline

Rust workbook ingestion, Athletic.net candidate discovery and evidence parsing, with native Restate orchestration and local-only model review. The executable exposes `worker`, `deploy`, `start`, `status`, `export`, and `verify`. Earlier CLI commands are no longer the production interface.

Current workstation run, Cloudflare challenge evidence, Restate UI links, verified repairs, and safe continuation steps: [HANDOFF.md](HANDOFF.md). The real 10,000-row qualification remains incomplete; browser/Spider acquisition has not yet been implemented.

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

Authorized source sessions can optionally set `source_session_file = "/absolute/private/source-session.json"` in the worker TOML. The strict JSON object contains `origin`, `user_agent`, `cookie_header`, and `expires_at_unix_ms` (Unix milliseconds). Use only the source cookies required by the authorized browser session, its User-Agent, and its actual clearance expiry; do not copy unrelated account or tracking cookies. The origin must match the configured source scheme, host, and effective port, with no path, query, fragment, or credentials.

The session file must be an absolute regular non-symlink file, at most 16 KiB; Unix group/other permissions must be unset (`chmod 600`). Each nonempty header is bounded to 8 KiB and rejects control characters. Invalid or expired sessions fail startup before the artifact store opens. The worker loads the file once: drain and stop it before replacing credentials, then restart the unchanged frozen binary. A session that expires during uptime fails as `AccessDenied` before sending HTTP; this does not clear a previously latched source block.

Session headers are attached only to source requests, never to the shared HTTP client's defaults, model requests, or captured request records. Debug output redacts both headers. Keep the credential file outside the repository and evidence bundles. This feature does not launch a browser, renew cookies, solve CAPTCHAs, or remove admission blocks. [Cloudflare clearance](https://developers.cloudflare.com/cloudflare-challenges/concepts/clearance/) is visitor/device-bound and can be invalidated before cookie expiry; a successful probe or unexpired file is not proof of sustained access.

`source_interval_ms = 0` disables fixed admission spacing; the default workstation configuration uses zero, while individual runs may choose a positive interval. Restate's `athletic-source` scope has a deployment ceiling of 16 concurrent shared HTTP operations, independently of row concurrency; stricter operator-set limits are preserved. Total request volume is not a concurrency setting. Already-admitted operations may remain in flight after a denial; every subsequent retry re-enters shared admission. This is not instantaneous revocation of physical requests.

Native flow control is an opt-in Restate feature. The pinned 1.7.10 server must have all three settings enabled in its private `restate.toml`:

```toml
experimental-enable-protocol-v7 = true
experimental-enable-vqueues = true
experimental-enable-scoped-virtual-objects = true
```

`deploy` checks the server version and advertised capabilities before registration. It conditionally creates the exact source rule at `min(16, enabled wildcard limit)`, rejects disabled, unlimited or excessive exact limits, and checks the confirmed rule against a stricter wildcard. To tighten a running deployment, change the exact `athletic-source` rule: Restate's exact rules override `*`. See [Restate flow control](https://docs.restate.dev/services/flow-control).

The `athletic-source-admission` scope serializes durable admission waits. The separate `athletic-source-control` scope remains available for quick pacing, denial, and cooldown updates while that queue sleeps. Competing callers cannot repeatedly advance the pacing deadline ahead of the queue head; only already-admitted operations can extend its cooldown. A bounded observation guard reports an orchestration invariant failure rather than caching an artificial rate-limit result. Cancelling a caller cancels its queued or sleeping admission invocation. Denial/cooldown feedback is sent as a detached native invocation and then attached for the handler result, so cancelling the caller does not discard an acknowledged policy update.

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

The CLI submits an idempotent export and observes its durable result with short output queries, rather than keeping one HTTP request open for the entire export. It polls once per second, with a 24-hour observation deadline and at most 86,400 polls; individual HTTP requests remain bounded to five minutes. An observation timeout or client interruption does not cancel the export. Rerun the same ingress/run/output arguments to recover its durable result. Terminal server errors are reported rather than treated as pending work.

Export verifies through the worker that owns its artifact store. Standalone `verify --store` requires the existing stopped database, not an empty directory; never open or copy a live Fjall directory from another process. An artifact-directory copy alone is not a coordinated Restate rollback image.

The independent `verify` command preflights both workbooks before sparse-cell readback, checks source hashes, original fields, source-sheet order and row accounting in the actual XLSX, then checks retained JSONL result evidence and binds every workbook annotation, including performance summaries, to its sidecar row. Sidecar source fields must have exactly the original column keys and values; annotation columns cannot substitute for original fields. Typed report, assessment and profile digests and retained-performance projections must agree. Positive checks cover selected identity, retained profile/document references, attributed participation, complete discovery and deterministic uniqueness. Review rows may retain conflicts without becoming positive results. Source, XLSX and sidecar hashes are checked for changes during verification. **This establishes retained-evidence consistency, not source authenticity or unknowable real-world identity accuracy; PR arithmetic and raw-source authenticity are not independently proved by this command.**

Embedded report, assessment, discovery, probe and full-acquisition metadata must match their hash-verified bytes in the artifact store. Discovery is reconstructed from the deterministic query plan and raw search pages, including pagination, candidate union, issues and completeness. Every page is bound to its indexed captured request, including POST method, exact query/filter/offset and frozen origin. Cache-equivalent case/stage variants use the actually executed retained query, never a substituted request. Complete profiles are reconstructed from raw TF/XC Bio, HTML and the exact ordered, bounded TeamNav requests authorized by those initial documents; their successful probe and receipt/operation prefixes must agree. Name exclusions independently reparse their raw initial documents. Missing bodies, altered bytes, omitted authorized operations and rehashed metadata that contradicts raw evidence fail verification. Parser-valid successful 2xx responses remain valid regardless of MIME.

Terminal query failures are bounded to one and cannot follow completed pagination. A `MalformedResponse` must bind the exact captured successful search operation and reproduce a raw parsing failure or the final page's pagination-reconciliation failure. Worker drain, CPU admission and task-join failures are classified separately as `ArtifactFailure`, since their cause need not recur during offline parsing. Global admission failures may legitimately retain the original denial receipt from a different query.

Every retained assessment is recomputed through the production decision rules over all candidate coverage, including the cumulative metadata budget; this applies to accepted, no-match and review outcomes. Local-review acceptance additionally applies the production selection-authorization rule. Rehashed scores or candidate flags cannot bypass those rules. A verified relay-member result identifies the relay separately from its member; the relay ID is not required to equal the selected athlete ID. Incomplete acquisitions may remain review evidence without claiming full raw reconstruction. Local capture records are trusted acquisition evidence, not cryptographic authentication of the upstream server or proof that its career corpus is exhaustive.

The `captured-source-request-response-v7` acquisition contract and `streaming-source-parsers-v13` parser revision invalidate older cache identities. Acquisition version 7 isolates serialized admission waits from policy updates, replacing version 6's contended pacing loop that could cache a failure after 64 ordinary rechecks. It retains version 6's bounded recovery for complete HTTP 429 responses, including challenge pages, version 5's source-only session isolation, and version 4's workflow-owned retry evidence. Parser version 13 retains malformed-row diagnostics even when an athlete link has no valid ID; such a row cannot silently become a complete empty search. Old observations cannot acquire missing request provenance retroactively: retain the compatible frozen worker for historical exports, and use a fresh snapshot/deployment for a changed acquisition contract. Never replay in-flight journals against a changed worker.

CLI preparation identities include the acquisition revision, so identical CLI arguments cannot reuse a preparation cached under the previous acquisition contract.

Source acquisition automatically schedules an initial attempt plus at most three retries, entirely before deterministic assessment and the separate local-model review queue. Restate owns durable calls, timers, journals and orchestration; the source workflow owns the four-step budget, with SDK retries disabled inside each HTTP step to avoid nested amplification. Every step re-enters shared source admission, capped at 16 concurrent source invocations. A valid positive `Retry-After` of at most 24 hours publishes a durable shared cooldown before retrying. Otherwise, HTTP 429 backs off for 60, 120 and 240 seconds; transient transport/5xx failures use 1, 2 and 4 seconds, respecting any larger configured source interval. Exhausted 429 retries latch the shared source block; malformed or excessive retry delays, definitive access challenges and artifact failures stop rather than loop indefinitely. A successful recovery does not republish an earlier cooldown. Local-model requests retain their independent SDK retry policy.

HTML/JSON acquisition does not interact with ordinary cookie, signup or subscription overlays. A passive Cloudflare script alone is not classified as a challenge. Definitive challenge headers or conjunctive challenge-page markers on non-429 responses retain the response as `AccessDenied`, even on HTTP 200, and stop source admission whether or not a source session is configured. A complete HTTP 429 challenge remains failed rate-limit evidence and follows the bounded cooldown policy instead; it is never selected as an athlete page or sent as athlete evidence to a model. Invalid retry delays, exhausted rate limits, and body/artifact safety failures still fail closed.

`observed_attempts` counts retained completed-attempt records, not remote request starts: records are published after response handling. An external response not acknowledged before a crash can be repeated. There is no exactly-once HTTP guarantee or four-physical-request ceiling across uncertain crash recovery. Historical recovery exercises below describe their frozen worker versions, including the earlier stop-on-first-429 policy.

The v6 native challenged-rate exercise recovered the initial 429 request after 60,338 ms with two deterministic matches and no model calls. Stopping and restarting both the unchanged worker and native Restate during that cooldown preserved its exact deadline; no source request preceded it and no invocation needed an operator resume. Persistent challenged 429 stopped after four attempts. Challenged HTTP 200/403 and invalid or over-24-hour retry delays stopped after one. Every blocked case rejected fresh-snapshot HTTP, and all six exports passed stopped-store verification. These are synthetic behavioral proofs, not completion of the live workbook.

The v7 admission exercise retained the same 27-row 8/3/16 result and 404 source requests under positive pacing, with 16 source slots occupied, at most one running admission waiter, and no artificial admission failures. A queued challenged-429 exercise restored both native services and made no subsequent HTTP request before the retained deadline; the first arrived after 60,301 ms. Cancelling a sleeping admission head and eight queued callers drained every invocation, preserved the cooldown, and emitted no extra source/model requests. The five persistent-denial cases retained their one/four-attempt bounds and prevented fresh snapshots from bypassing the block. Eight synthetic exports independently preserved all 405 original fields each. These are code-regression exercises, not a completed real 10,000-row qualification.

The policy follows established crawler separation of request retries, source pacing and downstream processing. [Scrapy RetryMiddleware](https://docs.scrapy.org/en/latest/topics/downloader-middleware.html#module-scrapy.downloadermiddlewares.retry) bounds transient-error retries, including 429; [AutoThrottle](https://docs.scrapy.org/en/latest/topics/autothrottle.html) explicitly prevents fast error responses from reducing delays. [Crawlee's documented throttling manager](https://crawlee.dev/js/docs/next/guides/request-loaders#per-domain-throttling) pauses the affected domain using `Retry-After` or exponential backoff and bounds persistent stalls. Its throttled retries do not consume its ordinary request-retry budget; this pipeline deliberately uses a single bounded source budget instead. No additional crawling framework, session rotation or proxy rotation is introduced.

The v4 native resilience exercise used only synthetic, credential-free loopback sources. A two-second `Retry-After` produced a 2,114 ms retry gap; an HTTP-date retry occurred 115 ms after the absolute deadline. Missing-header 429 recovery waited 60,106 ms, and three 503 retries waited 1,128 / 2,110 / 4,106 ms. Persistent 429 stopped after four attempts; malformed and over-24-hour headers stopped after one. Both HTTP-200 challenge forms stopped after one request, retained every row for review, and blocked fresh source work. Ordinary overlays retained both deterministic acceptances with zero model requests.

Restarting the unchanged worker and Restate during a 45-second cooldown preserved its deadline and completed automatically without resuming an invocation. Separate queue exercises kept models idle during acquisition retries; the ambiguous case invoked one model afterward. A 32-row-concurrency exercise observed 15 concurrent source requests, no new requests during the acknowledged cooldown hold, and the expected 27-row result: 8 accepted, 3 no-match, 16 review. Independent verification of the full regression and concurrency exports preserved all 405 original fields. Unit suites and fuzz campaigns were not executed for this exercise; fixed-input corpus checks, native scenarios, and compile/lint gates are separate evidence.

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

The higher-concurrency live 1,000-row stage selected 500 rows from each sheet with row concurrency 64 and source concurrency 16. Source admission stopped on a retained HTTP 429 HTML challenge; the body contained the challenge title, platform script, configuration, and JavaScript/cookie instruction. The run was cancelled and drained rather than expanded to 10,000 or the full workbook. Its verified partial export completed only two selected rows (one no-match, one review) while preserving all 120,716 source rows and 1,569,308 original fields. This is a blocked rollout, not an error-free 1,000-row result.
