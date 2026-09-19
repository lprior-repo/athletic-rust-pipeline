# Persistent Chromium source transport — current design

**Qualification status:** implemented interfaces are under current-tree qualification. This document describes the design, not a shipped live-readiness result. Full collection, matching, immutable export, and exact replay remain pending end-to-end execution.

## Boundary and invariants

Chromium is the source effect shell. Rust deterministic code consumes bounded, provenance-bearing receipts; it does not treat rendered DOM text or a model answer as source truth.

1. Use the ordinary installed headed Chromium with one private persistent profile. Chrome owns cookies, localStorage, IndexedDB, and cache.
2. Capture exact request method, URL, body, response status, headers, decoded bytes, and challenge indication through CDP/Fetch. Do not treat a navigation URL or an HTTP 200 challenge page as source success.
3. A challenge closes new source admission across the profile. Already-issued requests may finish. The transport must drain owned work before reuse or shutdown.
4. Automatic readiness is bounded. An unresolved challenge becomes durable `human_required`; the operator completes a real challenge manually in the same headed window/profile. Resume is allowed only after successful non-challenged document evidence.
5. No UA/fingerprint spoofing, webdriver patch, canvas/WebGL trick, proxy rotation, CAPTCHA service, cookie extraction/replay, or direct source-HTTP fallback. `reqwest` is for local model/control traffic.
6. Keep browser concurrency bounded (two tabs initially, validated upper bound eight) and preserve configured source admission. More tabs are not a substitute for authorization or rate policy.
7. Source body and parser-memory limits remain distinct: source transport caps decoded bodies at 32 MiB; ranking parsing accounts at most 8 MiB of capture state. Neither is a process-RSS claim.
8. External HTTP is not exactly-once. Lost acknowledgement is represented as uncertain effect; a replay may repeat a physical request.

## Durable ownership

Restate owns workflow state, retries, admission, timers, pause/resume, and checkpoints. A BrowserSession state machine records readiness, challenge, cooldown, human-required, recovery, and stopped phases. Browser process/page handles are effect-local and never serialized into durable state.

The source path is:

```text
SourceResource
  -> SourceCache (success cache; failed rankings are not cached)
  -> SourceGateway + scoped admission
  -> BrowserSession readiness
  -> one bounded Chromium tab / CDP capture
  -> immutable receipt/body/attempt evidence
  -> deterministic parser and decision core
```

Every attempt has a stable operation identity and a fresh attempt identity. Source retries, SDK retries, and model transport retries are separate budgets. A gate-closing race before physical dispatch is no attempt and consumes no HTTP retry budget. A gate-closing event after dispatch cannot revoke the already-issued request.

The rankings path is an additional `SourceResource::Rankings` action. Navigation capture builds a catalog from 46 requested event families, excludes walk, expands observed variants (current target 95), and records absences. Results capture validates exact scope, page advancement, and lower-bound coverage before publishing a page checkpoint. Relay results join through roster IDs; Grade 11 is a discovery projection, not workbook eligibility.

## Parser and evidence contract

`DocumentReceipt` binds digest, source URL, status, media type, byte count, timing, and ranking capture metadata. Ranking `PageObservation` preserves source row numbers, result IDs, candidate kind, roster presence, and unresolved roster counts. `RankingPageIndex` and immutable checkpoints bind each index to its raw receipt and collection/event/page identity.

The parser is split through the existing module graph: `src/runtime/rankings/page/parse.rs` declares `main` and `relay` and re-exports `parse_page_response` and `PageParseError` from `main`. `pub(crate)` keeps internal helpers separate from explicit public serialized IDs and provenance. This split is a source organization choice, not a second parser or alternate collector.

A failed ranking fetch is not cached as a successful absence. `SourceCache` only stores a failed outcome for non-ranking resources; failed rankings return through the gateway for fresh audit/retry. A final collection snapshot is immutable and can only be sealed when every event head is terminal and checkpointed.

## Challenge and shutdown behavior

Normal challenge execution may occur only in the bounded readiness window. Passive overlays are not proof of a challenge. HTTP challenge markers and response evidence close admission; a fetched challenge string is never executed as page JavaScript. The operator must perform any real Cloudflare action manually. No live challenge was automated for this documentation refresh.

Shutdown stops intake, drains/cancels owned work, closes pages/browser, joins process ownership, and leaves the private profile intact. Worker replacement requires a fresh compatible deployment/data identity; an in-flight journal must not be replayed against changed browser/parser semantics. Existing profiles, stores, journals, and frozen binaries are retained.

## Qualification gates

The current record includes a fresh production/test-library `cargo check`; strict Clippy still has one known trivial conversion for Main to repair. The 26-case private storage qualification exposed bugs under repair, so it has no passing claim. The private fixture addresses are Restate admin 21041, ingress 21042, fixture 21043, and worker 21140, with the worker not deployed.

Main must still execute the current frozen worker against native scenarios covering normal capture, exact request identity, compressed/decoded bytes, bounds, cancellation and tab reuse, challenge/human-required pause and explicit resume, profile-wide drain, denial/rate-limit/redirect classification, browser loss, pause/resume controls, immutable owner export, stopped-writer verification, and exact replay. These are gates, not completed results. No live-collection readiness claim is permitted.
