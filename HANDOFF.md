# Live source challenge investigation and continuation

Updated 2026-09-18 UTC. This is a handoff, not a completed 10,000-row qualification.

## Current Chromium work and next-session ownership

**Not shipped; live qualification awaits the operator.** The GPU-written candidate passed fmt, strict all-target Clippy and build. Twenty-two synthetic exports across retained candidates passed independent stopped-writer verification. The fresh live control reached a real Cloudflare checkbox and is safely paused in `human_required` with zero active source requests. Do not automate the real challenge or claim live acquisition success.

Main rejected the first GPU gate submission: separate atomics allowed stale reopening, the wait loop busy-polled, and overflow/poison handling did not satisfy the contract. Repairs were returned to the 5090. Further review returned lifecycle/cooldown/recovery defects to the 5090 and empty-body/deadline/abort/navigation-completion defects to the 3090. **The pre-GPU build success does not apply to their current edits.** Re-read the latest deliveries and actual code before validation or publication.

Both candidate builds passed `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --bin athletic-rust-pipeline`. Initial records are `gpu-final-*-01.json`; latest repair records are `gpu-cooldown-repair-{fmt,clippy,build}.json`. No unit suite or fuzz campaign ran.

Subsequent source review also rejected behavioral regressions: transport compared response events with a never-assigned document ID, aborted on challenge despite the drain contract, and confused fetch success with abort settlement; navigation rejected the intentionally closed startup/recovery gate and could lose events across separate streams. Lifecycle review found missing challenge-latch resets, repeated cached cooldowns, discarded combined challenge/429 feedback, and unconfirmed physical drain after job failure. Both GPUs received concrete counterexamples and exclusive-owner repair assignments. Completion messages remain unverified claims until Main reviews the actual source and runs the fresh native scenarios.

### Current live control and authorized legacy cutover

- The user explicitly selected **Checkpoint and stop legacy** before the bounded browser probe. Legacy monitor, worker and Restate are now stopped; original binaries, data and configurations remain preserved.
- Coordinator cancellation returned HTTP 202 at 2026-09-18T13:34:13.489Z. Subsequent native SQL reported zero noncompleted invocations. Both pre-cancel and final owner-online exports succeeded; final stopped-writer verification preserved 120,716 rows, two sheets and all 1,569,308 original fields.
- Final legacy checkpoint: zero accepted, three no-match, 73 review, 120,640 pending across the full workbook. This is partial/cancelled, not completed 10,000-row qualification. Evidence and exports are under the private proof root's `legacy-cutover/`; final XLSX SHA-256 `a034f04e2b5c71202893683fc420b565b3e44db6f9829118a1e5939cb06ffc49`, JSONL SHA-256 `fcf3715c39d7f30863fe2009307dc597f6ea17a2ad41ee8ed6913038df64a25f`.
- Latest frozen candidate `worker-chrome-gpu-v2`: SHA-256 `57891815f6a38f4f5d67fe46ed3ac75f306609c08bdcafe7a64655acd446e4b8`. Never replace this file or use changed code under its active journals.
- Live services: `athletic-chrome-live-control-restate` (internal 20930, admin 20931, ingress 20932) and `athletic-chrome-live-control-worker` (20933). Fresh deployment `dp_172gHbtH3GezUtFEbpRg7mx`; config/data/store are under private `live-control/`.
- Live profile remains `/home/lewis/.local/share/athletic-rust-pipeline/chrome-profile-0`, headed, two source tabs, one-second configured spacing. This is not an assertion of an operator-approved rate. No identity spoofing or cookie replay.
- Input is one manually prepared public positive-control row, not Golden data. Run `f9c24ec7d31033f56dbbed06bb3550d1b87eb82e004c32e89e5be291c4877769`; invocation `inv_1fsg95kbGEBB1uyEEwzPEWHFrn9PiQ4G7g`. Submission is not completion.
- Current browser DevTools port 41149. Headed page shows **Verify you are human**, title **Just a moment...**. Operator must complete it in that same window/profile; Restate then observes clearance and resumes. Main did not click or automate the real challenge. Private screenshot and metadata: `live-control/human-required.webp`, `human-required-state.json`.
- All completed synthetic verification services are stopped. Only the fresh live-control pair should continue for operator verification. Services remain hub-managed and are not guaranteed to survive harness teardown.

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

Latest integration contract: `NavigationOutcome::Pending` means an incomplete/changing document, not a one-second rate limit. Failed inspection/recovery must close admission. The final owner-frozen source passed fmt, strict Clippy and binary build; Main is now exercising the actual frozen worker. Any further repair returns to its GPU owner and requires a new frozen binary and fresh deployment/data identity rather than changing this deployed worker.

### Evidence already obtained

- Ordinary headed `/usr/bin/chromium`, using `/home/lewis/.local/share/athletic-rust-pipeline/chrome-profile-0`, loaded the public source homepage with HTTP 200.
- The exact public positive-control search POST also returned HTTP 200, JSON, 19,831 bytes, no `cf-mitigated` challenge header. `navigator.webdriver` was naturally false in this plain-browser control; no override was injected. A screenshot confirmed the normal homepage.
- This is a **plain-browser diagnostic**, not proof of the Rust/Restate acquisition path or sustained source access. The plain browser was stopped; its profile was retained.
- A fixed local encoding probe proved CDP `getResponseBody` changes Latin-1 `3c703ee93c2f703e` to UTF-8 `3c703ec3a93c2f703e`. Source evidence therefore uses the browser Fetch byte stream, base64 transport, and Rust decoding; CDP text is restricted to navigation classification.
- A managed headless Chromium redirect probe against the synthetic source produced `requestWillBeSent`, `responseReceivedExtraInfo` with status 302, then `loadingFailed(net::ERR_FAILED)` and a Fetch `TypeError`; it produced no `responseReceived` or `redirectResponse`. The raw event record is `redirect-cdp-proof.json` in the private root. Transport must correlate extra-info redirect status before treating loading failure as generic transport error. The proxy was restored to normal. This is a browser-platform probe, not a Rust-path pass.
- Four native probes executed the actual extracted `FETCH_FUNCTION` and `ABORT_FUNCTION`: empty HTTP 204, exact Latin-1 bytes, payload-limit rejection, and abort settlement all passed. `settlement-script-proof.json` binds the results to both script hashes. This exercised managed headless Chromium against synthetic localhost endpoints; it does **not** verify Rust marshalling, the pool, headed-profile behavior, or the Restate workflow.
- Before GPU coding began, `cargo fmt --all && cargo build --bin athletic-rust-pipeline --example native_fixture --example source_smoke` succeeded. No unit suite or fuzz campaign ran.
- Frozen pre-GPU baseline `worker-chrome-v1`: SHA-256 `fe6a5193dc1aa72cfb1dc6975067f9eb030503885642499210a40dfe74ee2944`.
- Frozen synthetic fixture `fixture-v1`: SHA-256 `03397091a113a50f0250e61a06f7bc6e477f2a231fd57fc315b562f722931db6`.
- Frozen GPU-built worker `worker-chrome-gpu-v1`: SHA-256 `ccd8fc7c07f2d73233294387baf10e6892eb8da4eaa774c4f577a8031710074f`. Fresh deployment `dp_15ELS2UUekYhRw8x09xyait` registered 12 services using Restate Rust SDK 0.12.0.
- Actual CLI `browser-start` completed through Restate; status became Ready with two tabs, zero active requests and zero cooldown. The headed worker browser uses the private synthetic profile. CDP inspection observed Chrome 151.0.7922.173 and naturally true `navigator.webdriver`, without an override. A screenshot showed the normal synthetic homepage.
- Native run `2c6ae05c0f16bc49923215b8cb80db0de6c199135b74d3bf8775d70367e25d91` completed 2/2 synthetic rows, both deterministic. Its 15 new source requests had zero missing bootstrap cookies and zero cross-origin requests. Owner-online export and later stopped-writer independent verification preserved both rows and all 30 original fields.
- Two separately submitted synthetic runs both completed 2/2 but executed serially. The distinct-identity `match,split-location` run `7012b2d796659e41ae5f522b9810ea5f5d359d51baa9ec08a19859bfacf3e59b` then completed 3/3 (two deterministic, one review-required), issued 30 requests, and reached actual source concurrency 2.
- Automatic synthetic challenge run `e966042276e228c54ce3d3163cf26ebc322a0748438e2fe7b09c24fec60b3df5` completed 3/3 with the same outcome distribution in approximately 11 seconds. Proxy evidence recorded 31 new source requests, two challenged responses including navigation, and two new homepage requests. Browser status returned Ready with zero active requests/cooldown. No human intervention or manual durable-state reset occurred.
- Human synthetic challenge run `b5d002945ced06ef681aaf6f185bef56da4121dd066cdce420d94a10154b7724` reached `human_required`, drained to zero active requests, and issued no additional source requests across two paused observations. Clicking the observed local synthetic verification button allowed SDK polling to resume automatically; final state Ready and coverage 3/3 (two deterministic, one review-required). Raw interaction record and screenshot are retained. An initial CSS-selector tool click timed out; the observed-element click succeeded. No real CAPTCHA was automated.
- Headerless HTML challenge run `66477b2be618ad32d4f9169e362c0c583a115cb1ca8295c74131539886c81968` reached Challenged, then HumanRequired with zero active requests. Completing the local synthetic button allowed the workflow to finish 3/3 with the same outcome distribution.
- Inspection of the multi-identity export explains its review-required result: exact name and matching school, but no corroborated school location; assessment `EvidenceReview`, with no valid 2..=64 hard-eligible candidate set for local review. This is a recorded matching-policy outcome, not a claimed acceptance or transport error.
- Gzip scenario `859fabace61a1961aea459f60897e59d9ccd442f55c65d4a93dc4f3d9a80cb9a` completed 3/3. All 27 exported response receipts (nine unique bodies) matched the independent proxy's decompressed byte lengths, SHA-256 hashes and statuses. This verifies receipt parity for those exported bodies, not a claim that compressed wire bytes are retained.
- All six exports (`normal`, `multi`, `auto`, `human`, `html`, `gzip`) passed independent CLI verification after Restate reported zero noncompleted invocations and the sole worker/store owner was stopped. Normal preserved 30 fields; each three-row export preserved all 45 fields. Exact records are `gpu-native-*-offline-verify.json`.
- The unchanged frozen worker was restarted against its existing deployment and data. SDK `browser-start` returned it to Ready. The same mode-0700 profile retained a synthetic localStorage marker across browser restart; natural `navigator.webdriver` remained true. New DevTools port is 36899. Evidence: `gpu-native-profile-persistence.json`.
- Redirect scenario `032f38bc770fde8f371ad972a729e274a7543e66b11ed9794687a48b0be61af0` completed all three rows as review-required: 20 source responses were rejected, and the redirect target received zero requests. Its export passed stopped-writer verification.
- Timeout scenario `57ea3b5d8bd09b6b1f36f3791625ac6be9a2c194db91cac611e8a144a8701b2d` used a 35-second backend delay against a 30-second request bound, then recovered through SDK retry. Retained attempt records classify `transport`, message `browser request timed out`; the matching physical retry began 31,217 ms after the first request. Final 3/3 outcomes and all 45 fields passed offline verification.
- Closing only the owned synthetic browser through CDP left status Stopped, zero active requests, and zero additional source HTTP. Fresh run `ffb708fc91e78ade1cf8bdbde2740e0b909d3f5ff2edbdbdb36bdfa00d05a4b5` completed three explicit review outcomes, not false no-matches. Its export passed stopped-writer verification. This is fail-closed browser-loss proof, not automatic browser respawn proof.
- Existing frozen `verify` reconstructs full evidence graphs; the existing narrow `extract` binary supports selected raw audits. A proposed redundant collector was rejected, never executed, and removed. One auto-challenge query retained two attempts, its previous HTTP 403 body receipt (386 bytes), and the subsequent HTTP 200 response.
- Raw native command records use the `gpu-native-*` prefix. These are synthetic deterministic matching checks, not real GPU-matching qualification or sustained public-source access.
- V1's extended matrix passed valid 429 recovery (next physical request after 5,208 ms for Retry-After 2), one-shot 503 recovery, ordinary 404, and 33 MiB body rejection. Simultaneous challenge-drain proof: second request returned 403 while first remained issued; the first completed normally 1,405 ms later, was dispatched only once, and no new source requests occurred while human-required.
- Native malformed Retry-After exposed an actor defect: parse failure incorrectly became a 60-second cooldown. The 5090 repaired `ops.rs` and draining guards in `lifecycle_ops.rs`; the first proposed repair was rejected for conflating missing delay with invalid metadata. Final error handling preserves the completed response, closes admission, drains owned work and shuts the browser down.
- Fresh `gpu-v2/` deployment verified the repair: malformed metadata terminated in 2,333 ms after one request, browser Stopped/zero cooldown, retained HTTP 429 body of 17 bytes with SHA-256 `3850dfdbf4489250268b5f0740240a9f4445e7c5c29e1d03aa0c5446808d7507`, matching the independent proxy. Normal, valid 429 and automatic challenge regressions also passed; all four exports passed offline verification.
- Isolated `denial/` proved ordinary 403 stops after one request without entering challenge recovery; its export passed offline verification. `profile-lock/` proved a second owner fails while the first remains Ready, with zero extra source/home requests. Supported eight-tab bootstrap reached Ready; configurations with 0/9 tabs, a 14-second window, or headless live mode were rejected before service readiness.
- Same-build worker and Restate restart/replay under `profile-lock/` restored the exact completed run with zero additional source or homepage requests. Its export passed offline verification. The combined count is 22 independently verified synthetic exports; none establishes real GPU-matching or sustained live-source qualification.

### Isolated native verification environment

Private root: `/home/lewis/.local/share/athletic-rust-pipeline/evidence-repairs-v8/chromium-TFpmUt`.

- The original synthetic Restate/worker identity used ports 20530–20533; the repaired candidate used 20630–20633. Both are stopped with data preserved.
- GPU-written `browser-source-proxy-v2.mjs` and frozen multi-identity fixture used ports 20535 and 20534; both are stopped after their completed scenarios. Counter records remain private.
- Synthetic profile `synthetic-chrome-profile` remains separate from the one permanent live profile solely for test isolation, not challenge avoidance.
- Isolated `denial/` used ports 20730–20733; `profile-lock/` used 20830–20833. All associated workers/Restate instances are stopped after exports, verification and replay.
- Restore only a retained frozen binary against its compatible data. Do not clear latched blocks to make another scenario pass; separate test identities preserved prior blocked state.
- The root also contains the plain public search response and metadata. Do not display source bodies or private profiles to remote models.
- Verify service readiness on resume. Services are hub-managed, not detached or guaranteed to survive session teardown.

### Review and verification gates still open

1. Operator completes the real checkbox in the existing headed live profile; inspect live `browser-status` and single-row run completion through ingress 20932.
2. If the bounded live probe completes, owner-export its actual results, drain/stop and independently verify. If it remains challenged or denied, retain the explicit blocker and do not expand workload or rotate identity.
3. Current proof includes selected byte/attempt audits and all synthetic export graphs, not schedule-exhaustive concurrency proof. Empty/Latin-1 scripts were browser-platform probes; keep that distinction.
4. Browser loss is proven fail-closed, not automatic respawn. No claim of exact-once external HTTP across uncertain crashes.
5. README/design operating cutover is updated locally; production Rust remains uncommitted pending final live qualification/review. Historical workflow packet remains baseline-scoped. Do not replace historical deployed binaries or replay changed code under existing journals.

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

## Historical live v7 run — now cancelled and checkpointed

Run root: `$E/ten-thousand-fair-EumuTg`.

The observations below are historical. The user-authorized cutover above supersedes them: monitor/worker/Restate stopped, root cancelled and drained, final export independently verified. Do not resume this attempt or restart source traffic without an explicit new decision.

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

### Historical inspection commands for unchanged restored services

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

## Earlier acceptance plan — superseded by current gates above

1. The v7 run was checkpointed and cancelled with user approval; it did not complete the 10,000-row goal. Preserve its final partial export and stopped data. Do not launch full-workbook processing.
2. Use the captured Managed Challenge and Ray ID for operator/browser diagnosis. Capture fresh response headers and real browser API behavior privately if needed. Current receipts retain body/status/media type and parsed delay, not the complete raw response-header block; do not fabricate missing headers.
3. Chromium acquisition is now implemented by the local GPUs and synthetically qualified as recorded above. The current live positive control awaits real human verification; there is no sustained-access claim.
4. Transport changes require fresh frozen workers and compatible journal/cache identities. The browser candidates used fresh data, and the historical v20 worker was never hot-swapped.
5. Complete the real 10,000 selected rows; prove real matching GPU requests and results, then same-build recovery/replay and owner-online export → drain → stop → offline original-field verification. Static reviews and synthetic model calls do not satisfy this.
6. Package the final evidence only after those obligations are met. Current state is **UNVERIFIED for real 10,000-row end-to-end acceptance**. Full-workbook rollout remains gated.

## Continuation environment

Build environment previously used: `CARGO_HOME=/cache/cargo-shared`, `CARGO_TARGET_DIR=/home/lewis/src/ad-law-scrape/athletic-rust-pipeline/.worktrees/authorized-alpha-continued/target`, `CARGO_BUILD_JOBS=16`; pinned nightly 2026-04-27. Do not overwrite frozen evidence binaries.

Services are currently hub-managed with `persist=false`, `detached=false`; they are not guaranteed to survive harness teardown. Check actual process state on a new session. If absent, recover the **same** frozen binaries using retained `worker.toml`, `restate.toml`, data directories and service command records; check SQL readiness, not TCP alone. Preserve evidence; never start two writers on one store. Existing worktrees/build artifacts were not pruned as part of this handoff.
