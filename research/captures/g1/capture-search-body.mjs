// One live POST /Search.aspx/runSearch through the lane's authorized browser session.
//
// Request shape mirrors the pipeline's own client:
//   * src/runtime/source/request/build.rs  -> url /Search.aspx/runSearch, body {q, fq, start}
//   * src/runtime/browser/transport/script.rs -> in-page fetch, POST, content-type: application/json
// The only deliberate difference is `redirect: 'follow'` (the pipeline uses 'error'); the final
// URL is recorded so a redirect is visible instead of being swallowed as a network error.
//
// It never touches an existing page target: it creates its own background tab and closes it.
//
// usage: node capture-search-body.mjs --port <cdpPort> --query "<First Last>" --out <dir> [--fq "t:a a:tf"] [--start 0]
import { writeFileSync, mkdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { join } from "node:path";

function arg(name, fallback) {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 && i + 1 < process.argv.length ? process.argv[i + 1] : fallback;
}

const port = Number(arg("port", "9333"));
const query = arg("query", null);
const fq = arg("fq", "t:a a:tf");
const start = Number(arg("start", "0"));
const outDir = arg("out", ".");
const origin = arg("origin", "https://www.athletic.net");
const timeoutMs = Number(arg("timeout-ms", "30000"));
const navWaitMs = Number(arg("nav-wait-ms", "30000"));
const searchPath = arg("path", "/Search.aspx/runSearch");
const dryRun = process.argv.includes("--dry-run");
if (!query) {
  console.error("--query is required");
  process.exit(2);
}
mkdirSync(outDir, { recursive: true });

const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
const ws = new WebSocket(version.webSocketDebuggerUrl);
let nextId = 0;
const pending = new Map();
const events = [];
const send = (method, params = {}, sessionId) =>
  new Promise((resolve, reject) => {
    const id = ++nextId;
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify(sessionId ? { id, method, params, sessionId } : { id, method, params }));
  });
ws.addEventListener("message", (ev) => {
  const m = JSON.parse(ev.data);
  if (m.id && pending.has(m.id)) {
    const { resolve, reject } = pending.get(m.id);
    pending.delete(m.id);
    if (m.error) reject(new Error(`${m.error.message} (${m.error.code})`));
    else resolve(m.result);
  } else if (m.method) {
    events.push({ method: m.method, params: m.params, sessionId: m.sessionId });
  }
});
await new Promise((resolve) => ws.addEventListener("open", resolve));

const evalIn = async (sessionId, expression, awaitPromise = false) => {
  const r = await send("Runtime.evaluate", { expression, awaitPromise, returnByValue: true }, sessionId);
  if (r.exceptionDetails) throw new Error(`evaluate failed: ${r.exceptionDetails.text}`);
  return r.result?.value;
};

const created = await send("Target.createTarget", { url: "about:blank", background: true });
const attached = await send("Target.attachToTarget", { targetId: created.targetId, flatten: true });
const sid = attached.sessionId;
await send("Page.enable", {}, sid);
await send("Runtime.enable", {}, sid);
const navigatedAt = new Date().toISOString();
await send("Page.navigate", { url: `${origin}/` }, sid);

const docDeadline = Date.now() + navWaitMs;
let doc = null;
for (;;) {
  doc = await evalIn(
    sid,
    `({href: location.href, title: document.title, ready: document.readyState,` +
      ` turnstile: document.querySelectorAll('iframe[src*="challenges.cloudflare.com"]').length,` +
      ` ua: navigator.userAgent,` +
      ` first_text: (document.body ? document.body.innerText.replace(/\\s+/g, " ").slice(0, 240) : "")})`,
  );
  if (doc && doc.ready === "complete") break;
  if (Date.now() > docDeadline) break;
  await new Promise((r) => setTimeout(r, 500));
}

const requestBody = JSON.stringify({ q: query, fq, start });
const issuedAt = new Date().toISOString();
const outcome = dryRun
  ? { ok: false, error: "dry_run", message: "no request issued" }
  : await evalIn(
  sid,
  `(async () => {
     const t0 = Date.now();
     try {
       const r = await fetch(${JSON.stringify(searchPath)}, {
         method: 'POST',
         headers: {'content-type': 'application/json'},
         body: ${JSON.stringify(requestBody)},
         redirect: 'follow',
         credentials: 'same-origin',
         signal: AbortSignal.timeout(${timeoutMs})
       });
       const buf = new Uint8Array(await r.arrayBuffer());
       const headers = {};
       for (const [k, v] of r.headers.entries()) headers[k] = v;
       return { ok: true, status: r.status, status_text: r.statusText, final_url: r.url,
                redirected: r.redirected, headers, byte_count: buf.length,
                body_base64: buf.toBase64(), elapsed_ms: Date.now() - t0 };
     } catch (e) {
       return { ok: false, error: String((e && e.name) || e), message: String((e && e.message) || e),
                elapsed_ms: Date.now() - t0 };
     }
   })()`,
  true,
);
const finishedAt = new Date().toISOString();

const stamp = issuedAt.replace(/[-:]/g, "").replace(/\.\d+Z$/, "Z");
const base = `search-post-${stamp}`;
let bodyFile = null;
let byteCount = null;
let sha256 = null;
let decodedChars = null;
if (outcome && outcome.ok) {
  const bytes = Buffer.from(outcome.body_base64, "base64");
  byteCount = bytes.length;
  decodedChars = bytes.toString("utf8").length;
  sha256 = createHash("sha256").update(bytes).digest("hex");
  bodyFile = `${base}.body`;
  writeFileSync(join(outDir, bodyFile), bytes);
}

const meta = {
  captured_utc: issuedAt,
  capture_driver: "research/captures/g1/capture-search-body.mjs",
  cdp: {
    endpoint: `http://127.0.0.1:${port}`,
    browser: version.Browser,
    user_agent: version["User-Agent"],
    target_id: created.targetId,
  },
  navigation: { requested_url: `${origin}/`, navigated_utc: navigatedAt, ...doc },
  request: {
    url: `${origin}${searchPath}`,
    method: "POST",
    headers: { "content-type": "application/json" },
    body: requestBody,
    redirect_mode: "follow",
    credentials: "same-origin",
    timeout_ms: timeoutMs,
    completed_utc: finishedAt,
  },
  response: outcome
    ? outcome.ok
      ? {
          status: outcome.status,
          status_text: outcome.status_text,
          final_url: outcome.final_url,
          redirected: outcome.redirected,
          headers: outcome.headers,
          byte_count: byteCount,
          decoded_chars: decodedChars,
          sha256,
          body_file: bodyFile,
          elapsed_ms: outcome.elapsed_ms,
        }
      : { error: outcome.error, message: outcome.message, elapsed_ms: outcome.elapsed_ms }
    : { error: "no_result" },
  page_events: events.filter((e) => e.method.startsWith("Page.")).map((e) => e.method),
};

writeFileSync(join(outDir, `${base}.meta.json`), `${JSON.stringify(meta, null, 2)}\n`);
await send("Target.closeTarget", { targetId: created.targetId }).catch(() => {});
ws.close();
console.log(JSON.stringify({ base, body_file: bodyFile, sha256, byte_count: byteCount, status: meta.response.status ?? null, error: meta.response.error ?? null }));
