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
        const init = {method: spec.method, redirect: 'error', signal: controller.signal, headers: {}};
        for (const pair of spec.headers) {
            init.headers[pair[0]] = pair[1];
        }
        if (spec.body !== null) {
            init.headers['content-type'] = 'application/json';
            init.body = spec.body;
        }
        const response = await fetch(spec.url, init);
        let body = response.body;
        if (body) {
            const draining = response.clone();
            await response.body.cancel();
            body = draining.body;
        }
        const reader = body && body.getReader();
        if (reader) {
            let size = 0;
            try {
                for (;;) {
                    const part = await reader.read();
                    if (part.done) break;
                    const nextSize = size + part.value.byteLength;
                    if (!Number.isSafeInteger(nextSize) || nextSize > spec.max_body) {
                        controller.abort();
                        return {ok: false, error: 'payload_limit'};
                    }
                    size = nextSize;
                }
            } finally {
                reader.releaseLock();
            }
        }
        return {ok: true, error: null, status: response.status};
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
