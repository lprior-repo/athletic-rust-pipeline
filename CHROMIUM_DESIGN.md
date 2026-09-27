# Persistent headed browser lane

Browser transport contract and current code map. Source planning/parsing belongs in
[SOURCE_ADAPTER_GUIDE.md](SOURCE_ADAPTER_GUIDE.md); durable handler identity/retry policies belong in
[RESTATE_WORKFLOWS.md](RESTATE_WORKFLOWS.md). Dated qualification results belong in
[verification evidence](docs/VERIFICATION-EVIDENCE.md), not this design reference.

## Boundary

`athleticnet-browser` owns the persistent Chromium profile, bounded tabs, CDP request/response
capture and `BrowserError` classification. `census-crawl` bridges it into the shared fetcher;
`census-service` hosts the optional `BrowserSession` object. The endpoint owns process/page handles;
those handles are not serialized into Restate state.

Athletic.net uses the ordinary installed **headed** browser and a dedicated persistent profile.
Chrome owns cookies, storage and cache. No spoofing, webdriver patch, proxy rotation, CAPTCHA service,
cookie extraction/replay, authentication/paywall bypass or direct-HTTP challenge fallback. Other
source families may use their registry-approved direct HTTP transport; that is not an Athletic.net
fallback. Robots and admission still apply inside a browser.

## Capture contract

- Bind physical request method/URL/body, status/headers, decoded response and source provenance.
  A navigation URL or HTTP 200 challenge is not successful source evidence.
- Preserve the distinction between the physical API URL and semantic citation URL. `RequestSpec`
  carries both in `crates/athleticnet-browser/src/request/build.rs`.
- `RankingPageObservation` in `protocol.rs` carries capture kind, request method/URL/body and next
  page; the census bridge wire representation is in `crates/census-crawl/src/net/bridge/wire.rs`.
  These types alone do not prove complete capture-to-export lineage or independent readback.
- Navigation capture and result capture are different operations. The rankings transport sends
  `GetRankings` results through the bootstrapped page; `GetNavInfo` uses navigation/interception.
  `transport/rankings/capture.rs::validate_route` checks the expected method/path on the configured
  source origin. Domain parsing, grade attribution, relay-member joins and pagination completion
  remain adapter responsibilities, not transport guarantees.
- The fetcher's current cache reuses HTTP-200 metadata only when the stored body's digest matches
  (`census-crawl/src/net/cache.rs`). Cache eligibility does not replace challenge/schema validation.
  Keep failures and actual capture timestamps; never replay a failure as successful absence.

The deleted workbook-search package had `DocumentReceipt`, `RankingPageIndex`, `parse_page_response`
and `result_verify` types. They are historical, not current qualification claims. Its captured
rankings/search findings remain research evidence; do not restore its competing workflow to recover
those names.

## Bounds, challenges and retries

`ProfileConfig::validate` allows 1–8 tabs; browser decoded bodies are capped at 32 MiB by
`MAX_SOURCE_RESPONSE_BYTES`. Direct fetch and parser bounds are separate. Neither bound establishes
a whole-process memory cap. Keep queue ownership and outstanding work bounded, share physical-origin
admission, and measure successful throughput before increasing concurrency.

A detected challenge closes affected admission. Already-dispatched work may finish and must be
accounted for. Readiness attempts are bounded; unresolved access requires `HumanRequired` and an
operator handoff in the same headed profile. Passive overlays are not sufficient challenge evidence,
and a fetched challenge string is never executed as page JavaScript.

Restate is the sole retry owner; browser, fetcher and model transports each make one attempt. There
are no independent nested retry budgets. A refusal before physical dispatch consumes no HTTP attempt;
after dispatch, cancellation cannot undo the request. Lost acknowledgements may repeat physical
HTTP, so required request-counter scenarios—not an invocation setting—prove the global ceiling.

## Shutdown and qualification

Stop intake, drain/cancel owned work, close pages/browser and join process ownership while preserving
the private profile. Record every completed/cancelled/timed-out/aborted/panicked outcome. Deployment
compatibility must preserve run/store/journal semantics; replacing a worker does not inherently
require a new census or empty data root. See [lifecycle](docs/deployment-lifecycle.md).

Qualification must use the actual headed path, challenge/429/5xx responses, bounded bodies, cancellation
and native retry counters. The [fault catalog](docs/NATIONAL-CENSUS-FAULTS.md) owns acceptance; a
transport unit test or historical successful capture is not proof of today's whole source workflow.
