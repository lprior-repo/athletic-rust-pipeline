# Live source challenge investigation and continuation

Updated 2026-09-18 UTC. This is a handoff, not a completed 10,000-row qualification.

## Current Chromium work and next-session ownership

**Not shipped or end-to-end qualified.** The working tree contains browser implementation changes. The last integrated baseline builds; two local GPU coding workers are now repairing it. Do not treat their eventual completion messages as verification.

Main rejected the first GPU gate submission: separate atomics allowed stale reopening, the wait loop busy-polled, and overflow/poison handling did not satisfy the contract. Repairs were returned to the 5090. Further review returned lifecycle/cooldown/recovery defects to the 5090 and empty-body/deadline/abort/navigation-completion defects to the 3090. **The pre-GPU build success does not apply to their current edits.** Re-read the latest deliveries and actual code before validation or publication.

Two integrated post-GPU commands, `cargo fmt --all && cargo check --all-targets`, failed. The first reported 24 errors and 3 warnings; the second reported 40 errors and 2 warnings after the restored transport module exposed additional cleanup-helper failures. Formatting succeeded; no test suite executed. Raw logs are retained as `gpu-check-01.log` and `gpu-check-02.log` under the private root below. Repairs were returned to both GPUs. **No post-GPU compilation success has been established.**

Subsequent source review also rejected behavioral regressions: transport compared response events with a never-assigned document ID, aborted on challenge despite the drain contract, and confused fetch success with abort settlement; navigation rejected the intentionally closed startup/recovery gate and could lose events across separate streams. Lifecycle review found missing challenge-latch resets, repeated cached cooldowns, discarded combined challenge/429 feedback, and unconfirmed physical drain after job failure. Both GPUs received concrete counterexamples and exclusive-owner repair assignments. Completion messages remain unverified claims until Main reviews the actual source and runs the fresh native scenarios.

### Latest user direction

- The 5090 and 3090 do the actual coding. Main owns planning, review, native verification, and handoff.
- All production workflow actions, admission, waits, recovery and retries remain owned by Restate SDK.
- Start with one permanent private headed Chromium profile and two tabs, configurable up to eight. Do not rotate profiles to work around challenges or rate limits.
- A challenge closes profile-wide admission; already-issued requests may drain. Resume only through the controlled browser/Restate recovery path.
- No User-Agent overrides, webdriver patches, CAPTCHA services, proxy rotation, or browser-cookie extraction/replay through reqwest.

### Published versus uncommitted state

- `95331dd`: published [WORKFLOW_REVIEW.md](WORKFLOW_REVIEW.md), a historical function-level map against `fee6c82`.
- `e519683`: published [CHROMIUM_DESIGN.md](CHROMIUM_DESIGN.md). The requested immediate design push to `main` is complete.
- Chromium implementation is uncommitted. Its acquisition revision is `captured-chromium-source-request-response-v8`; parser revision remains v13.
- Current code adds the private profile, browser pool, CDP observation, Restate `BrowserSession`, CLI `browser-start`/`browser-status`, and browser-backed source capture. The old cookie/session-file transport has been removed from the working tree.
- Do not deploy these changes over the historical worker or replay its active journals using this implementation.

### Local GPU coding workers

| Worker | Actual model binding | Exclusive ownership |
|---|---|---|
| `GPU5090ChromeCoding` | `qwen36-5090/Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf:high` | `browser.rs`, `browser/gate.rs`, lifecycle/actor/ops/shutdown/pool, `browser_session.rs`, `browser_readiness.rs` |
| `GPU3090ChromeCoding` | `qwen36-3090/Qwen3.6-35B-A3B-UD-Q4_K_XL.gguf:high` | transport/navigation and their descendant modules |

Agent configurations are in the parent workspace, **outside this Git repository**: `/home/lewis/src/ad-law-scrape/.omp/agents/`. The 5090 coder was widened from synthetic-only ownership; a separate 3090 coding profile was added. Neither model server was restarted or reconfigured. Existing reviewer profiles remain separate.

Shared coding contract: the 5090 implements `browser::gate::ProfileGate` with `new`, `snapshot` (`generation`, `ready`), `revoke`, `try_open(observed_generation)`, and cancellation-safe async `closed`. The 3090 consumes it through `Arc<ProfileGate>` in `transport::fetch` and `navigation::start_observer`. An async inspection may reopen admission only if its observed generation is unchanged. Public browser manager/status/settings/error interfaces remain stable. No overlapping file edits; no worker-run builds, formatters, linters, tests, or services while both edit.

Resume coordination via exact worker IDs in `hub`, or read `agent://GPU5090ChromeCoding` and `agent://GPU3090ChromeCoding` after they settle. Check status before assuming either is still running. Review actual changed artifacts and then run integration checks once; do not accept model assertions as evidence.

### Evidence already obtained

- Ordinary headed `/usr/bin/chromium`, using `/home/lewis/.local/share/athletic-rust-pipeline/chrome-profile-0`, loaded the public source homepage with HTTP 200.
- The exact public positive-control search POST also returned HTTP 200, JSON, 19,831 bytes, no `cf-mitigated` challenge header. `navigator.webdriver` was naturally false in this plain-browser control; no override was injected. A screenshot confirmed the normal homepage.
- This is a **plain-browser diagnostic**, not proof of the Rust/Restate acquisition path or sustained source access. The plain browser was stopped; its profile was retained.
- A fixed local encoding probe proved CDP `getResponseBody` changes Latin-1 `3c703ee93c2f703e` to UTF-8 `3c703ec3a93c2f703e`. Source evidence therefore uses the browser Fetch byte stream, base64 transport, and Rust decoding; CDP text is restricted to navigation classification.
- Before GPU coding began, `cargo fmt --all && cargo build --bin athletic-rust-pipeline --example native_fixture --example source_smoke` succeeded. No unit suite or fuzz campaign ran.
- Frozen pre-GPU baseline `worker-chrome-v1`: SHA-256 `fe6a5193dc1aa72cfb1dc6975067f9eb030503885642499210a40dfe74ee2944`.
- Frozen synthetic fixture `fixture-v1`: SHA-256 `03397091a113a50f0250e61a06f7bc6e477f2a231fd57fc315b562f722931db6`.

### Isolated native verification environment

Private root: `/home/lewis/.local/share/athletic-rust-pipeline/evidence-repairs-v8/chromium-TFpmUt`.

- `athletic-chromium-proof-restate`: native Restate 1.7.10, internal 20530, admin 20531, ingress 20532. Configuration and data are in that private root.
- `athletic-chromium-proof-source`: controlled Bun source proxy on 20535, forwarding synthetic source traffic to fixture port 20534.
- `athletic-chromium-proof-fixture`: the frozen `fixture-v1` backend is now launched and ready on 20534 with the synthetic `match` scenario, output directory `fixture-match`. Worker configuration `worker-browser-synthetic.toml` points source traffic through 20535, model-fixture traffic directly to 20534, and uses a separate private headed two-tab profile. **No browser worker has been launched or deployed.**
- Reserved worker port: 20533.
- Proxy control modes currently include normal, automatic synthetic challenge, human synthetic challenge, HTML-only challenge, gzip, and redirect. `/__test/encoding` is the fixed Latin-1 boundary. Private records retain synthetic request/response hashes and request/cookie/concurrency counts.
- The root also contains the plain public search response and metadata. Do not display source bodies or private profiles to remote models.
- Verify service readiness on resume. Services are hub-managed, not detached or guaranteed to survive session teardown.

### Review and verification gates still open

1. Review GPU repairs for normal concurrent admission: active jobs must not make `inspect` abort every second request.
2. Verify stale homepage evidence cannot clear a challenged API gate; recovery must be issued only when it can navigate, followed by the bounded SDK wait and human resume.
3. Verify profile-wide revocation cannot lose a race against an async Ready transition, and failed jobs cannot leave apparently Ready ghost slots.
4. Verify continuous navigation evidence ordering, no stale bootstrap overwrite, and body-fallback classification before Ready.
5. Verify exact body bytes, empty bodies, POST identity, compressed responses, bounded timeout/abort cleanup, and no redirect following.
6. Run fresh native Restate/worker/Chromium scenarios against synthetic data, then owner-online export, drain/stop, and independent offline verification. Never open an active Fjall store twice.
7. Run the actual Rust/Restate public-source path conservatively. The successful plain-browser control alone does not satisfy this gate.
8. Update current README/design/operating instructions and publish only reviewed, verified implementation. Historical workflow documentation must remain explicitly baseline-scoped.

The actual authorization email, authorized scope, and operator-approved rate have not been supplied as an artifact. Do not fabricate them or claim the configured one-second interval is an approved rate. Retain the real document privately when supplied.


## Earlier live-run investigation: historical context

- Repository: `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline`, branch `main`.
- Completed code fixes are published through `e4e8153`. This handoff adds operational findings; it does not change the deployed worker.
- Native Restate UI: <http://127.0.0.1:20331/ui/>. Verified in Chromium, including loaded service and invocation views.
- Current coordinator: <http://127.0.0.1:20331/ui/invocations/inv_1fsg95kbGEBB6nOEElxygnCRaWVReZHGuX>.
- State: <http://127.0.0.1:20331/ui/state>. Scope counters: <http://127.0.0.1:20331/ui/flow-control/counters>.
- Admin API is port 20331; ingress 20332; worker 20333; internal Restate 20330. Do not expose admin publicly. From another machine, use an SSH tunnel to this workstation: `ssh -L 20331:127.0.0.1:20331 <workstation>`.
- UI completed-invocation counts include internal calls. They are NOT completed workbook rows. Use coordinator coverage below.
- Full implementation map for the next reviewer: [WORKFLOW_REVIEW.md](WORKFLOW_REVIEW.md), prepared against source baseline `fee6c82` with static reviews on the 5090 and 3090. It includes the call graph, state/cache ownership, verification limits, and a source-derived local-review selection-reachability finding. This documentation does not change the deployed binary or establish real GPU-matching qualification.

## User intent and hard constraints

The user authorizes source access and requested the challenge investigation recorded below. The current direction is Chromium acquisition wrapped in Restate, with coding performed by the two local GPUs; the section above supersedes the earlier implementation status. Historical deployed-worker observations below remain distinct from the uncommitted browser implementation.

- Native Rust and native Restate; no Docker, cloud browser, external matching model, or GitHub Actions.
- Golden workbook: 120,716 rows, two sheets, 1,569,308 original fields. Preserve all originals. Membership establishes eligibility: no class, grade, or graduation-year filter.
- Never display Golden identities or raw rows to remote AI. Run local processing and emit aggregates only. Raw request URLs, source bodies, exports, and credentials remain private.
- Existing local GPU model servers are ports 11000 and 11001. Do not restart/reconfigure them. Static GPU code review is NOT proof of real matching GPU execution.
- No unit-suite executions or fuzz campaigns. Use native CLI scenarios, fixed-corpus smoke, fmt, all-target compilation/Clippy as appropriate.
- Never open or copy an active Fjall database from a second process. Owner-online export, drain, stop writer, then offline verification.
- Never replace a deployed binary or replay active journals against changed worker code. Freeze uniquely named binaries and fresh acquisition identities for changed transport semantics.
- Do not clear durable blocks/cooldowns to force progress. Restate owns source admission and bounded retries.

## What the challenge actually is

An exact retained response from the stopped v5 live run was extracted on 2026-09-18. The historical worker owner `athletic-session-10k-worker` was confirmed exited before opening its artifact store. The current live store was not opened.

| Field | Observed value |
|---|---|
| Response time | 2026-09-17T23:41:12.432Z |
| Endpoint | `https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData` (private athlete parameters omitted) |
| HTTP status | 429 |
| Content type | `text/html; charset=UTF-8` |
| Body length | 5,939 bytes |
| HTML title | `Just a moment...` |
| Cloudflare configuration | `cType: 'managed'` |
| Cloudflare Ray ID | `a3cbf5d8ab5d0f96` |
| Script path | `/cdn-cgi/challenge-platform/h/g/orchestrate/chl_page/v1` |
| No-JavaScript instruction | `Enable JavaScript and cookies to continue` |
| Retained body digest | `9e3fc5f32e1c1ddb227495e6f6f9fb8277357ec065d93d0f679a9c7517785d10` |

This is a **Cloudflare Managed Challenge response**, not evidence of a permanently visible popup or demonstrated interactive CAPTCHA. HTTP 429 is observed; the exact triggering Cloudflare rule is unknown. The site operator can correlate the Ray ID and UTC time in Cloudflare Security Events. Ask for the matched rule, action, and rate-limit scope; do not infer them from the HTML alone.

The native source worker uses reqwest. Replaying a browser's cookies/User-Agent is not browser JavaScript execution. At 2026-09-18T02:43Z, a fresh local Chromium navigation to the public homepage loaded the normal Athletic.net page. This does not prove the challenged API or a sustained workload is exempt.

Spider's [`chrome` feature](https://docs.rs/spider/latest/spider/) supports JavaScript rendering using Chrome. A browser still needs correct cookies, origin, HTTP method/body, and durable pacing. JavaScript does not exempt requests from rate limits. **Fetching challenge HTML through `fetch()` does not execute that HTML's scripts as a page navigation would.** The source includes POST search requests as well as GET API/profile requests; a generic rendered-GET crawler is not a drop-in transport replacement. Do not enable nested Spider retries, recursive discovery, proxies, or cloud/AI features without preserving the current contracts.

Cloudflare references: [clearance](https://developers.cloudflare.com/cloudflare-challenges/concepts/clearance/), [operator skip rules](https://developers.cloudflare.com/waf/custom-rules/skip/). Clearance can be invalidated or rate-limited. A narrowly scoped operator-approved exception is distinct from solving a challenge.

## Local debug files — do not commit their contents

Base: `/home/lewis/.local/share/athletic-rust-pipeline/evidence-repairs-v8` (abbreviated `$E` below).

`$E/challenge-debug-2BUBkB/` is mode 0700; captures are mode 0600:

- `challenge-original-private.html`: exact archived response, including challenge tokens. Inspect as text; opening it locally does not solve a live origin-bound challenge.
- `attempt-private.json`: exact retained attempt, including request URL/body and receipt. May contain identifying parameters; never print into AI context.
- `challenge-offline-preview.html`: scripts removed, network blocked with CSP, noscript instruction made visible. Open this to see the captured instruction safely. **This is an offline diagnostic preview, not a fresh live challenge.**
- `challenge-facts.json`: safe extracted identifiers/markers.
- `extract-build.json`, `extract-challenge.json`, `extract-attempt.json`: actual extraction commands and exit codes, all zero.
- `ui-browser-check.json`, `live-summary.json`: time-stamped checks.
- `extract.rs` / `extract`: local diagnostic helper; only for confirmed stopped stores.

Actual source session: `$E/access-e2e-Rtp0RG/source-session.json`, mode 0600. Do not display, commit, upload, or copy into handoffs. Origin/expiry constrained, loaded once by the worker; nominal expiry does not guarantee continuing clearance.

For human debugging: open the offline preview; inspect `attempt-private.json` locally for the exact URL; in your browser's DevTools enable Network → Preserve log and inspect Status, Content-Type, `cf-ray`, `cf-mitigated`, Retry-After, and Response. Keep exported HARs private: they can contain identities and cookies. Coordinate live probes with the running workload rather than generating a parallel scrape. A successful homepage navigation is not an API or load test.

## Current live v7 run — still incomplete

Run root: `$E/ten-thousand-fair-EumuTg`.

- Run digest: `016810253316119ba76f60fe2086d34aed5f9c8c519526dec9d191defd21fac7`.
- Coordinator invocation: `inv_1fsg95kbGEBB6nOEElxygnCRaWVReZHGuX`.
- Started approximately 2026-09-18T02:04:40Z.
- Services: `athletic-fair-live-restate`, `athletic-fair-live-worker`, `athletic-fair-live-monitor` (Oh My Pi hub-managed). Inspect these exact names before starting anything else.
- Selection 5,000 per sheet / 10,000 total; row concurrency 64; source concurrency ceiling 16; interval 400 ms; acquisition v7 / parser v13.
- Snapshot `live-v7-fair-10k-EumuTg`; execution `live-fair-v20`.
- At **02:43:47.609Z**: selected 10,000, completed **1**, no-match 1, local review 0, source cache results 2,455, no persistent source block, no paused invocations. Repeated cooldowns are ongoing. These cache results are NOT completed rows.
- A prior 02:27 SQL observation counted ten successful source operations that recovered after HTTP 429. Recovery does not imply adequate sustained throughput or completed qualification.
- `monitor-samples.jsonl` contains safe aggregate status. `monitor-latest-private.json` includes private state; locally filter before displaying.
- Monitor checks every 15 s, prints every fourth poll. It cancels on persistent source block or artificial no-attempt admission failure. Paused invocation detection stops the monitor without cancellation. Check `monitor-result.json` if it exits.
- Monitor has a bounded ~72-hour polling window and requires renewal if still healthy; it does not cancel solely at that boundary. Do not claim autonomous indefinite monitoring.

Frozen live worker: `$E/fair-admission-hBj6aA/worker-fair-v20`.
SHA-256: `0d3a9ba4ba841c00edd3e5e812ff2dddc608ff94de98d3f16f99ae58df788bce`.
This is the release worker with `edaf408` behavior. **Do not replace it with a newly compiled main binary under these journals.**

CLI-only export repair: `$E/fair-admission-hBj6aA/export-cli/cli-export-v21`.
SHA-256: `faa9272dc41979d6fa69db34db1b37c32872cf5a66672ca52909b3bed266d57e`.
It can inspect/export the unchanged v20 worker; it must not be substituted as a changed worker.

### Safe inspection commands

```sh
E="$HOME/.local/share/athletic-rust-pipeline/evidence-repairs-v8"
CLI="$E/fair-admission-hBj6aA/export-cli/cli-export-v21"
RUN=016810253316119ba76f60fe2086d34aed5f9c8c519526dec9d191defd21fac7
"$CLI" status --ingress http://127.0.0.1:20332/ --run "$RUN" \
  | jq 'if . == null then {initializing:true} else {coverage,complete} end'
curl --fail-with-body --silent --show-error http://127.0.0.1:20331/query \
  -H 'Content-Type: application/json' -H 'Accept: application/json' \
  --data '{"query":"SELECT target_service_name,status,COUNT(*) AS count FROM sys_invocation GROUP BY target_service_name,status"}'
```

To export, choose a NEW output path and run `"$CLI" export --ingress http://127.0.0.1:20332/ --run "$RUN" --output /absolute/new-result.xlsx`. This is owner-online; do not independently open the active store. It can take time. CLI interruption does not cancel the durable export. Repeat the identical request to retrieve the same publication.

For a deliberate cutover, cancel the coordinator using `PATCH /invocations/<invocation>/cancel` on admin port 20331, wait for all affected invocations to drain, export retained partial results through the owner, then stop the worker and verify offline. Preserve Restate data/config/frozen binaries. Never describe a cancelled partial export as a completed run. Consult README recovery/cancellation sections before execution.

## What changed and what was actually verified

1. **`2188715` — source-only sessions and challenged-429 recovery.** Strict private file, origin/expiry checks, sensitive header redaction and isolation. Complete 429 challenge bodies use bounded durable retry, not a parser success. No-Retry-After fallback 60/120/240 s; four attempts maximum in the normal retained sequence. 200/403 challenges stop immediately; malformed/excessive Retry-After fails closed. No exactly-once HTTP claim across uncertain crashes.
2. **`edaf408` — fair durable admission.** Old concurrent 64-recheck pacing generated synthetic `RateLimited`/`NotAttempted` cache failures: 120 observed on the v6 live run. New exclusive `athletic-source-admission` queue holds the durable sleeping head; separate quick `athletic-source-control` accepts feedback. Scope `athletic-source` bounds HTTP concurrency at 16. Guard failure is infrastructure retry, not false domain evidence. Acquisition bumped to v7.
3. **`e4e8153` — durable export client observation.** Old CLI timed out after 300 s while Restate continued exporting. CLI now submits one idempotent export, polls the maintained SDK output handle, and has a bounded 24-hour observation window. Terminal errors propagate. Worker behavior unchanged.

Evidence root `$E/fair-admission-hBj6aA`:

- `matrix-concurrent`: 21 scenarios / 27 rows, final 8 accepted, 3 no-match, 16 review; 404 source HTTP, one synthetic model HTTP, observed source concurrency 16, one running admission head, zero artificial failures; exact replay zero extra effects; offline 405 original fields verified.
- `queued-429-recovery`: 60-second cooldown persisted through same-build worker + Restate restart with a queued admission head. First subsequent HTTP after 60,301 ms, no manual resume; final 27 rows / 8-3-16; exact replay no extra effects. Cancellation subcase drained to zero active invocations without clearing cooldown or extra HTTP. Proxy uses an 8 MiB bound and converts oversized upstream responses to 502; direct matrix covers native body-limit behavior.
- Five denial cases: persistent challenged 429 stops at four attempts; 200 and 403 challenge stop at one; invalid and >24-hour Retry-After stop at one. Fresh snapshots produce no extra HTTP after persistent block. All owner-exported, stopped and offline-verified.
- Eight regression exports total, 405 original fields each. Format, all-target Clippy, release build and fuzz-target compilation/Clippy succeeded. No unit runs or fuzz campaigns.
- `export-cli/behavior-proof.json`: actual CLI remained observing after 389,084 ms of worker unavailability; restored unchanged v20 worker and **explicitly resumed one paused export**; completion exit 0, 405 fields. Identical reattach no new durable export; missing run terminal exit 1 in 1,009 ms; final offline verification passed. Do not call the explicit resume automatic recovery.
- Static local GPU reviews are archived with dispositions. Several claims were rejected against SDK/runtime evidence; notably Restate 409 is cancellation, not concurrency rejection. Their summaries do not supersede raw proof. Disposition files may still describe subsequently executed obligations as pending; reconcile against raw evidence before claiming a final assurance bundle.

### Historical runs, retained rather than erased

- v5 `$E/ten-thousand-session-IfXEy8`: stopped on challenged 429 under old policy; one completed row. Source of exact challenge above. Fully preserved partial output, 1,569,308 original fields verified.
- v6 `$E/ten-thousand-durable-rate-NNyMI0`: cancelled for admission-starvation defect, not persistent Cloudflare denial. Three completed rows (one no-match, two review), 9,997 selected unfinished. Exact real cooldown deadline survived a same-build restart. Final owner export continued after CLI timeout; later retrieved, drained/stopped, offline verified all 120,716 rows / 1,569,308 fields. Files `admission-defect-final-outcome.json`, `admission-defect-offline-verification.json`, `real-restart-*.json` preserve the distinctions.

## Remaining acceptance and next actions

1. Inspect current services/monitor before acting; continue or deliberately checkpoint/cancel the v7 run. It is nowhere near a completed 10,000-row proof. Do not launch full-workbook processing.
2. Use the captured Managed Challenge and Ray ID for operator/browser diagnosis. Capture fresh response headers and real browser API behavior privately if needed. Current receipts retain body/status/media type and parsed delay, not the complete raw response-header block; do not fabricate missing headers.
3. Design and exercise local Chrome/Spider acquisition for the actual GET and POST source operations. Preserve request/body/response evidence, bounds, cancellation, origin-bound credentials, session lifecycle, single Restate retry ownership, and no recursive crawling. Validate JavaScript challenge handling separately from rate-limit behavior. No claim yet that this resolves sustained access.
4. Any transport change needs an acquisition revision, safe old-run drain/export/stop/verify, fresh frozen worker and fresh compatible journals/cache identities. Never hot-swap the active v20 deployment.
5. Complete the real 10,000 selected rows; prove real matching GPU requests and results, then same-build recovery/replay and owner-online export → drain → stop → offline original-field verification. Static reviews and synthetic model calls do not satisfy this.
6. Package the final evidence only after those obligations are met. Current state is **UNVERIFIED for real 10,000-row end-to-end acceptance**. Full-workbook rollout remains gated.

## Continuation environment

Build environment previously used: `CARGO_HOME=/cache/cargo-shared`, `CARGO_TARGET_DIR=/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/.worktrees/authorized-alpha-continued/target`, `CARGO_BUILD_JOBS=16`; pinned nightly 2026-04-27. Do not overwrite frozen evidence binaries.

Services are currently hub-managed with `persist=false`, `detached=false`; they are not guaranteed to survive harness teardown. Check actual process state on a new session. If absent, recover the **same** frozen binaries using retained `worker.toml`, `restate.toml`, data directories and service command records; check SQL readiness, not TCP alone. Preserve evidence; never start two writers on one store. Existing worktrees/build artifacts were not pruned as part of this handoff.
