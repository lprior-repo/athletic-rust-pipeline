# Persistent Chromium source transport — implementation design

Status: approved direction from the user; implementation in progress. This document is not a claim that browser acquisition is deployed or verified. Baseline workflow packet was published on main in `95331dd`. Existing deployed reqwest-era workers and their journals remain frozen and untouched.

## Required behavior

1. Use chromiumoxide 0.9.1 and installed `/usr/bin/chromium`; one ordinary headed Chrome process with a permanent private profile. Never delete the profile between runs.
2. Bootstrap through normal source-page navigation. JavaScript, cookies, localStorage, IndexedDB and cache remain Chrome-owned.
3. Capture actual CDP Network request/response identity, status and headers. Detect `cf-mitigated: challenge` and retain the existing bounded HTML challenge fallback. Unknown/denied HTTP statuses are not success.
4. On challenge, immediately stop new source commands across all tabs in that profile. Already-issued physical requests can finish; do not promise instantaneous revocation.
5. Allow normal browser challenge execution for a bounded automatic-resolution window, initially 30 seconds. If unresolved, expose durable `HumanRequired`; the user completes any interactive challenge in headed Chrome.
6. Resume only after actual successful, non-challenged source document evidence. URL equality alone is not clearance. A fetched HTML string does not execute its scripts.
7. Start with one browser and two bounded tabs; support explicit bounds up to eight. Keep the initial source rate conservative. Adding tabs/processes is not a substitute for agreed source-rate expectations.
8. Keep normal Chrome identity and the same machine/profile. No UA overrides, fingerprint spoofing, webdriver patches, fake canvas/WebGL, proxies, CAPTCHA services, or cookie extraction/replay through reqwest.
9. Chrome makes all Athletic.net acquisition requests, including exact POST search and GET Bio/Profile/Team requests. reqwest remains only for local model/control traffic.
10. Preserve full bounded source-response evidence, deterministic Rust parsing/matching, local-only ambiguity review, all original workbook fields and independent export verification.
11. Keep authorization email and agreed scope/rate evidence private alongside operational configuration. The actual email is not supplied in this implementation request; do not fabricate or publish it. A conservative configured rate is not an assertion of a site-approved rate.

## Restate is the workflow owner

```text
CLI / RunCoordinator / RowWorker
  -> QueryWorker / ProfileWorker
  -> SourceCache
  -> SourceGateway (existing bounded source scope)
  -> serialized source admission
  -> BrowserSession.await_ready (durable exclusive object)
       -> journaled launch / bootstrap / inspect / recovery effects
       -> durable phase state and durable sleeps
       -> Ready / Challenged / CoolingDown / HumanRequired / Restarting / Stopped
  -> journaled single source HTTP attempt
       -> local BrowserManager
       -> one bounded Chrome tab
       -> CDP-attested request/response capture
  -> immutable attempt/body evidence
  -> existing Rust parsers and decision workflow
```

### Durable ownership

- A Restate `BrowserSession` object owns workflow phase, automatic-resolution window, human-required state, resume observations and recovery ordering.
- Browser startup/bootstrap, inspection, and explicitly authorized recovery navigation occur inside SDK `ctx.run` effects. Live process handles are never serialized into durable state.
- `ctx.sleep` owns challenge/human/cooldown waiting. The browser manager must not implement an independent application retry scheduler or autonomous human-polling policy.
- Shared status is a Restate observation API. CLI startup/status commands use generated SDK clients; no separate web server or filesystem queue.
- SourceGateway keeps its existing bounded source retry policy and durable cooldown. Browser transport never silently retries the original request.
- A challenge response is retained as challenge evidence, not success and not automatically an unrecoverable ordinary 403. Subsequent original-request attempts re-enter durable admission after browser readiness.
- A race in which the profile gate closes after durable admission but before physical dispatch must be treated as no physical request, not a fabricated HTTP attempt or a consumed HTTP retry budget.
- Plain 401/403 denial, invalid rate-limit metadata, retry exhaustion and evidence-integrity failures retain conservative failure semantics.
- Browser readiness waits are explicitly bounded; inactivity/cancellation/recovery policies must be verified together rather than allowing a shorter handler inactivity timeout to silently invalidate a longer human wait.

### Physical ownership

- Runtime owns one lazily initialized BrowserManager; initialization is invoked from the SDK effect boundary, not an eager source navigation before worker registration.
- BrowserManager owns the Chromium child, CDP handler, bounded tab resources and immediate fail-closed physical gate.
- CDP challenge events may revoke physical admission immediately. They do not independently authorize durable workflow resumption.
- Every spawned driver/request task has an explicit owner and shutdown path. Cancellation closes/aborts pending page work before a tab is reused.
- Shutdown stops intake, drains or cancels bounded in-flight work, closes Chrome, joins handler/process ownership and reports errors. Profile contents survive shutdown.
- Chromium singleton/profile ownership must prevent two processes claiming the same profile. No attachment to or takeover of the user's unrelated browser.

## Evidence and transport contract

- Preserve exact requested URL, HTTP method and serialized JSON search body. Never interpolate source values as executable JavaScript.
- Register CDP observations before issuing the action; correlate target request/response identifiers. Reject redirects rather than certifying a response from another route.
- Capture raw decoded response bytes, not rendered DOM serialization. Keep the 32 MiB source-body limit and bounded request duration.
- CDP/browser decompression means compressed wire Content-Length is not the decoded-body length. Enforce the decoded bound without a false equality check.
- Preserve status, Content-Type, Retry-After and challenge indication. Browser cookies/authentication state must not appear in captured request records or logs.
- Complete bodies and attempt evidence remain in the existing worker-owned Fjall store. No second process opens the live database.
- Bootstrap/navigation subresources are ordinary Chrome-managed page activity. Restate controls pipeline commands; it does not transactionally journal every browser-generated image/script request.
- External HTTP is not exactly once. Lost acknowledgement remains an explicit uncertain-effect boundary.

## Clean cutover and deployment

- Replace source-session-header replay in the new production source path. Do not retain a live-source reqwest fallback or silently transplant browser cookies.
- Add a new acquisition revision for browser semantics. Update verifier/request contracts where representation changes; retain historical documentation as explicitly baseline-specific.
- Do not replace the currently deployed binary or run changed code under its active journals. Build a uniquely named browser-enabled binary and use a fresh compatible native Restate deployment/data identity for verification.
- No Docker, cloud browser, external matching model, GitHub Actions, GPU-server restart or configuration changes.
- Existing local model servers stay on ports 11000 and 11001. Browser source work does not send Golden/private context to development reviewers.

## Implementation ownership

- Main: planning, adversarial source review, native verification, documentation and publication. Per the latest user direction, Main does not perform further production coding.
- GPU5090ChromeCoding: local 5090 implementation of process/profile/tab ownership, generation-gated admission, lifecycle/actor/pool/shutdown, and SDK BrowserSession/readiness integration.
- GPU3090ChromeCoding: local 3090 implementation of CDP request/response capture, raw byte acquisition, cancellation cleanup, and continuous navigation observation.
- The two coders have disjoint file ownership. Main integrates only after both freeze; neither coder runs concurrent builds, formatters, linters, tests, or services.

The GPUs now perform coding, superseding the earlier review-only assignment. Their completion reports are not verification, and development-model execution is not evidence that the production matching workflow has executed on both matching lanes. Current repair status and private native evidence locations are retained in [HANDOFF.md](HANDOFF.md).

## Required verification before claiming implementation complete

Use native executable scenarios, not a unit-suite or fuzz campaign:

- Normal JS bootstrap; persistent browser state survives process restart.
- Exact GET and POST acquisition, response bytes/status/headers and retained evidence.
- Compressed response, body limit, timeout and cancellation without tab reuse races.
- Automatic challenge resolution and unresolved `HumanRequired`; manual completion resumes through durable readiness.
- Two-tab/profile-wide stop on one challenge; queued commands cannot leak through the gate.
- Plain 404, denial, 429/Retry-After, server error and redirect classification.
- Browser death/profile lock/shutdown and native Restate replay/recovery behavior.
- Raw evidence and exported bundle verification using the correct store owner or a confirmed stopped writer.
- A bounded authorized live browser check; no claim of whole-workbook/10,000-row scale success from that check.
- Compilation, formatting, source Clippy and dependency checks appropriate to the pinned crate. No unit suites or fuzz campaigns.

Unexecuted scenarios remain unverified. An interactive challenge requiring the user's action or missing authorization-email artifact is an explicit external prerequisite, not permission to synthesize success.
