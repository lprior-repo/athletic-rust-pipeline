# Live source challenge investigation and continuation

Updated 2026-09-18 UTC. This is a handoff, not a completed 10,000-row qualification.

## Start here

- Repository: `/home/lewis/src/ad-law-scrape/athletic-rust-pipeline`, branch `main`.
- Completed code fixes are published through `e4e8153`. This handoff adds operational findings; it does not change the deployed worker.
- Native Restate UI: <http://127.0.0.1:20331/ui/>. Verified in Chromium, including loaded service and invocation views.
- Current coordinator: <http://127.0.0.1:20331/ui/invocations/inv_1fsg95kbGEBB6nOEElxygnCRaWVReZHGuX>.
- State: <http://127.0.0.1:20331/ui/state>. Scope counters: <http://127.0.0.1:20331/ui/flow-control/counters>.
- Admin API is port 20331; ingress 20332; worker 20333; internal Restate 20330. Do not expose admin publicly. From another machine, use an SSH tunnel to this workstation: `ssh -L 20331:127.0.0.1:20331 <workstation>`.
- UI completed-invocation counts include internal calls. They are NOT completed workbook rows. Use coordinator coverage below.

## User intent and hard constraints

The user authorizes source access and wants the Cloudflare challenge identified, local debugging access, JavaScript/browser acquisition investigated (including Spider RS), and all code/context published on main for another AI to continue. The latest priority is diagnosis and handoff. **Browser acquisition is not implemented yet.** The prior no-Spider decision has been reconsidered by the user; do not repeat it as a prohibition.

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
