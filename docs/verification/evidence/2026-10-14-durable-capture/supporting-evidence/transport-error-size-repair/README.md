# Capture Store error-size repair

## Scope and contract

Root approved boxing only the new FetchError::CaptureStore payload. The variant becomes CaptureStore(Box<StoreError>) and keeps its transparent error annotation. Explicit From<StoreError> allocates one box only on that failure path; From<CaptureError> delegates through the same conversion. CrawlError and existing errors are not boxed or reclassified, and the85 downstream service consumers are unchanged.

Six exact current-live bases plus one new regression file are included. Root remains sole live writer/integrator. This worker ran no build, test, formatter, network, gate or commit. Apply changes.patch with strip level2 after checking exact base pins.

## Consumer changes

- Four exact nested terminal assertions now bind the boxed StoreError and inspect error.as_ref() with an exact CaptureError guard. The two budget-exhaustion and two retained-outcome-unknown assertions retain their previous meaning and request counts.
- Coach classification fixture constructs through From<CaptureError>, exercising the actual conversion rather than handboxing a synthetic payload. Its Failed/invariant assertions remain intact.
- All other CaptureStore match arms use wildcard payloads and need no change. A read-only workspace search found no remaining payload constructors or deep patterns outside this seven-file delta.

## Authored regressions, unrun

- capture_store_indirection_keeps_fetch_and_crawl_errors_below_128_bytes
- boxed_capture_store_conversions_preserve_the_exact_typed_refusal
- boxed_capture_store_io_preserves_path_display_and_typed_source

The tests enforce size_of::<FetchError>() and size_of::<CrawlError>() below128 bytes, direct StoreError and CaptureError conversion identity, unchanged retryability/diagnostics, retained I/O path, and the original concrete I/O source kind/detail through both transparent wrappers.

Separate source review is running against these exact pins. Root should rerun lint, these three tests, existing coach tests and durable capture tests after integration. No passing gate is claimed by this detached proposal.
