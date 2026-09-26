use std::time::Duration;

pub(super) const FETCH_TIMEOUT: Duration = Duration::from_millis(500);

pub(super) const FETCH_FUNCTION: &str = r#"async function(spec) {
    const controller = new AbortController();
    globalThis.__adlaw_transport_controller = controller;
    let completionResolve;
    const completion = new Promise((resolve) => { completionResolve = resolve; });
    globalThis.__adlaw_transport_completion = completion;
    const timer = setTimeout(() => controller.abort(), spec.timeout_ms);
    try {
        const init = {method: spec.method, redirect: 'error', signal: controller.signal};
        if (spec.body !== null) {
            init.headers = {'content-type': 'application/json'};
            init.body = spec.body;
        }
        const response = await fetch(spec.url, init);
        const reader = response.body && response.body.getReader();
        if (typeof Uint8Array.prototype.toBase64 !== 'function') {
            controller.abort();
            clearTimeout(timer);
            return {ok: false, error: 'body_unavailable'};
        }
        if (!reader) {
            clearTimeout(timer);
            return {ok: true, error: null, status: response.status, body_base64: ''};
        }
        let bytes = new Uint8Array(Math.min(4096, spec.max_body));
        let size = 0;
        for (;;) {
            const part = await reader.read();
            if (part.done) break;
            const nextSize = size + part.value.byteLength;
            if (nextSize > spec.max_body) {
                controller.abort();
                clearTimeout(timer);
                return {ok: false, error: 'payload_limit'};
            }
            if (nextSize > bytes.byteLength) {
                const grown = new Uint8Array(Math.min(spec.max_body, Math.max(nextSize, bytes.byteLength * 2)));
                grown.set(bytes.subarray(0, size));
                bytes = grown;
            }
            bytes.set(part.value, size);
            size = nextSize;
        }
        clearTimeout(timer);
        return {ok: true, error: null, status: response.status, body_base64: bytes.subarray(0, size).toBase64()};
    } catch (error) {
        return {ok: false, error: error && error.name === 'AbortError' ? 'aborted' : 'network'};
    } finally {
        clearTimeout(timer);
        if (completionResolve) completionResolve(true);
        if (globalThis.__adlaw_transport_controller === controller) delete globalThis.__adlaw_transport_controller;
    }
}"#;

pub(super) const ABORT_FUNCTION: &str = r#"() => {
    const controller = globalThis.__adlaw_transport_controller;
    if (controller) controller.abort();
    const completion = globalThis.__adlaw_transport_completion;
    if (completion) {
        return new Promise((resolve) => {
            completion.then((confirmed) => resolve({ok: true, confirmed: confirmed === true}));
        });
    }
    return {ok: true, confirmed: false};
}"#;
