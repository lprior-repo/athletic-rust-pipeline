use crate::BrowserError;
/// Verified browser-realm interceptor prototype.
/// Replaces __RANKINGS_CAPTURE_CONFIG__ with serde_json-serialized config.
const INTERCEPTOR_TEMPLATE: &str = r#"(() => {
  const config = __RANKINGS_CAPTURE_CONFIG__;
  if (window !== window.top) return;
  window.__RANKINGS_ORIGINAL_FETCH = window.fetch;
  const originalFetch = window.fetch;
  const responseLimit = 8 * 1024 * 1024;
  const requestLimit = 64 * 1024;
  const failure = code => Object.assign(new Error(code), { captureCode: code });
  async function readBounded(stream, limit) {
    if (!stream) return new Uint8Array(0);
    const reader = stream.getReader();
    let buffer = new Uint8Array(Math.min(limit, 65536));
    let length = 0;
    try {
      for (;;) {
        const part = await reader.read();
        if (part.done) return buffer.subarray(0, length);
        const needed = length + part.value.byteLength;
        if (needed > limit) {
          void reader.cancel().catch(() => {});
          throw failure('payload_limit');
        }
        if (needed > buffer.byteLength) {
          const expanded = new Uint8Array(Math.max(needed, Math.min(limit, buffer.byteLength * 2)));
          expanded.set(buffer.subarray(0, length));
          buffer = expanded;
        }
        buffer.set(part.value, length);
        length = needed;
      }
    } finally {
      reader.releaseLock();
    }
  }
  function base64(bytes) {
    const chunks = [];
    const stride = 24576;
    for (let offset = 0; offset < bytes.byteLength; offset += stride) {
      chunks.push(btoa(String.fromCharCode(...bytes.subarray(offset, offset + stride))));
    }
    return chunks.join('');
  }
  function describe(input, init) {
    const rawUrl = typeof input === 'string' || input instanceof URL ? String(input) : input?.url;
    const url = new URL(rawUrl, location.href);
    if (url.origin !== config.origin) return null;
    const kind = url.pathname === '/api/v1/tfRankings/GetNavInfo' ? 'navigation'
      : url.pathname === '/api/v1/tfRankings/GetRankings' ? 'results' : null;
    if (!kind) return null;
    const method = String(init?.method ?? input?.method ?? 'GET').toUpperCase();
    let bodyInput;
    let requestClone;
    if (method !== 'GET' && method !== 'HEAD') {
      if (init && Object.prototype.hasOwnProperty.call(init, 'body')) bodyInput = init.body;
      else if (input instanceof Request) requestClone = input.clone();
    }
    return { kind, url: url.toString(), method, bodyInput, requestClone };
  }
  async function requestBody(description) {
    if (description.requestClone) {
      return new TextDecoder('utf-8', { fatal: true }).decode(await readBounded(description.requestClone.body, requestLimit));
    }
    if (description.bodyInput === undefined || description.bodyInput === null) return null;
    if (typeof description.bodyInput !== 'string') throw failure('unsupported_request_body');
    if (description.bodyInput.length > requestLimit || new TextEncoder().encode(description.bodyInput).byteLength > requestLimit) throw failure('request_payload_limit');
    return description.bodyInput;
  }
  function publish(description, fields) {
    const binding = window[config.binding];
    if (typeof binding !== 'function') return;
    binding(JSON.stringify({ nonce: config.nonce, kind: description.kind, requestUrl: description.url, method: description.method, ...fields }));
  }
  window.fetch = function(input, init) {
    let description;
    try { description = describe(input, init); } catch { description = null; }
    const promise = Reflect.apply(originalFetch, this, arguments);
    if (!description) return promise;
    void promise.then(async response => {
      const headers = {};
      for (const name of ['content-type', 'cf-mitigated', 'retry-after', 'location']) {
        const value = response.headers.get(name);
        if (value !== null) headers[name] = value;
      }
      const metadata = { status: response.status, headers };
      try {
        const retained = response.clone();
        const body = await requestBody(description);
        const bytes = await readBounded(retained.body, responseLimit);
        publish(description, { ...metadata, requestBody: body, body: base64(bytes), bodyBytes: bytes.byteLength });
      } catch (error) {
        publish(description, { ...metadata, error: error?.captureCode ?? 'capture_failed' });
      }
    }, () => publish(description, { error: 'fetch_failed' })).catch(() => {});
    return promise;
  };
})();"#;

/// Interceptor config serialized as JSON for the JS prototype.
#[derive(serde::Serialize)]
struct InterceptorConfig<'a> {
    origin: &'a str,
    binding: &'static str,
    nonce: u64,
}
pub(crate) const BINDING_NAME: &str = "retainRankingResponse";

/// Build the interceptor script with serialized config.
pub(crate) fn build_interceptor_script(origin: &str, nonce: u64) -> Result<String, BrowserError> {
    let config = InterceptorConfig {
        origin,
        binding: BINDING_NAME,
        nonce,
    };
    let json = serde_json::to_string(&config).map_err(|_| BrowserError::Protocol)?;
    let script = INTERCEPTOR_TEMPLATE.replace("__RANKINGS_CAPTURE_CONFIG__", &json);
    Ok(script)
}
