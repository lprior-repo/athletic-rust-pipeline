# Scope and architecture

## Data flow

```text
large XLSX
  -> streaming OOXML reader
  -> local-only row scope (strict first worksheet or legacy sport filter)
  -> scoped TF and XC search with full pagination reconciliation
  -> deterministic evidence extraction and best guess
  -> optional strict local Q5 extraction + Q4 identity review + Rust policy enforcement
  -> authorized alpha manifest + typed API client (optional separate run)
  -> 50-state/event matrix + completeness validation
  -> cohort filter + safe normalization + athlete-ID deduplication
  -> checkpoints + privacy-guarded source outputs
  -> local alpha-source matching
  -> enriched XLSX copy with a new result sheet
```
Result records retain every discovered profile hint, including weak candidates, and expose `hint_count` plus an `AI Logic` explanation in CSV and `Athletic Matches`.

## Component boundaries

| Module | Responsibility | Does not do |
|---|---|---|
| `xlsx` | Stream shared strings and worksheet XML; ignore empty styled rows; append result worksheet | Send personal data to any service |
| `discovery` | Query the scoped Athletic.net search endpoint and retain allowed athlete URLs | Fetch candidate pages |
| `fetch` | Fetch one exact, already-discovered URL via Spider after authorization gate | Crawl links, bypass blocks, log in, use stealth/proxies |
| `extract` | Convert HTML/snippets to compact evidence and call the configured local model | Decide the final identity alone |
| `scoring` | Normalize names/schools; apply thresholds and corroboration rules | Invent missing identity evidence |
| `marks` | Canonicalize events and compare validated marks | Treat model arithmetic as authoritative |
| `checkpoint` | Append and reload the latest row or alpha-unit result | Delete previous evidence |
| `output` | Write audit JSONL, flat CSV, unresolved queue | Mutate source workbook rows |
| `alpha_api_client` | Call only developer-authorized, allow-listed API routes with bounded pagination/retries | Scrape public pages or guess credentials |
| `alpha_catalog` | Validate exactly 50 states and the discovered event matrix | Add DC, territories, or unconfirmed events |
| `alpha_cohort` / `alpha_normalize` | Apply Class-of-2027 evidence precedence and safe field normalization | Promote missing or conflicting evidence |
| `alpha_match` | Match a completed local alpha source into existing candidate/scoring types | Make Athletic.net network calls |
| `exhaustive` / `exhaustive_run_rows` | Own the writer lock, cancellation, durable row commits, and final exports | Report retryable errors as complete |
| `exhaustive_search` / `search_cache` | Bound concurrent search and persist complete query outcomes | Cache failed or truncated queries as successful |
| `exhaustive_ai` / `ai_cache` | In AI mode, require both clients and cache schema-bound successful analysis | Substitute heuristic evidence after model failure |
| `exhaustive_identity` | Reconcile canonical athlete identities and conservative attribution | Attribute an unresolved row to a selected profile |
| `coverage` / `jsonl` | Bind population/configuration/schema, reconcile every source key, and repair only torn suffixes | Ignore committed corruption or orphan checkpoint keys |
| `retry_policy` / `model_transport` | Classify transient failures, honor bounded Retry-After, retry model transport | Retry invalid model content or ignore server delays |
| `reuse_search` | Validate, lock, and atomically import a compatible deterministic run's search cache | Import its decisions or checkpoints |

## Authorized alpha API path

The optional alpha run starts only from a validated local manifest with documented developer permission. It calls the confirmed nav/rankings routes, requires exactly 50 canonical states, rejects unknown completeness or cap behavior, records an append-only unit checkpoint, and writes a separate source directory. `match-authorized` reads that directory locally and does not contact Athletic.net.

### 1. Workbook scan

The XLSX reader opens the ZIP container, streams `sharedStrings.xml`, resolves worksheet relationships, and streams each worksheet `<row>`. A row counts as real only when it contains at least one non-empty cell. This avoids the `Sheet1` inflated dimension.

Each selected prospect receives the immutable key `sheet_name:excel_row`. Every later output uses that key; names are never used as write-back keys.

`--all-workbook-rows --first-worksheet-only` selects every real row in the first actual worksheet relationship, irrespective of source sport. For this workbook the population is 111,939 rows with all 13 source columns preserved. Seven rows have no name and finish as explicit `INPUT_ERROR`; the remaining 111,932 require TF and XC discovery.

### 2. Candidate discovery

For each selected athlete, queries are generated from:

- exact full name;
- school;
- city/state;
- expected class year;
- `site:athletic.net/athlete`.
Only URLs whose normalized host is `athletic.net` or `www.athletic.net` and whose path starts with `/athlete/` are retained. Completed row records checkpoint the query, URLs, snippets, extracted evidence, model decision, and scores under the immutable source key.

Strict exhaustive discovery has no candidate-count truncation. It validates athlete IDs, result counts, pagination progress, and the bounded page limit; any unknown or inconsistent completeness produces a retryable `SEARCH_ERROR`. Two concurrent query futures share one spacing gate. All deduplicated query stages complete in both sport lanes before the final decision, without an early-match shortcut.

### 3. Retrieval

Default mode retains search-result title/snippet/URL as evidence and can ingest manually saved `<athlete-id>.html` pages. Authorized mode uses Spider with concurrency 1, a stop-after-seed callback, robots enabled, and a configured delay to retrieve each exact candidate URL. The pipeline has no anti-bot fallback.

Direct retrieval additionally requires `SPIDER_MAX_SIZE_BYTES` in the inclusive range 1,048,576–4,194,304 bytes. Saved files are also bounded to 4 MiB. The owned crawl/collector futures are joined under a deadline; truncation, invalid response status, access challenges, and identity-changing redirects are errors.

If a site requires JavaScript, supply manually saved HTML or adapt the authorized retriever to Spider's `smart`/Chrome feature after confirming permission. The base build intentionally avoids browser automation.

### 4. Local model

The configured local model server receives only:

- prospect name, school, full street address, city/state, postal code, class year, and sport;
- candidate title/snippet/profile URL;
- compact public page text if authorized retrieval is enabled.

It never receives email or the full workbook row. Street/postal values are identity context, not candidate facts; missing candidate address evidence is unknown rather than a mismatch. The current implementation passes the original street/postal fields and canonicalizes observed state names; it does not implement a street-component parser or authoritative postal-address validation.

AI-enabled exhaustive mode requires Q5 extraction and Q4 identity review. Rust first records a deterministic decision, then Q5 extracts every candidate and Q4 reviews the complete candidate set once. Invalid JSON, omitted extraction schema, unavailable models, or invalid review indices become retryable `AI_ERROR`, not a guessed match or `NO_MATCH`. Explicit null evidence fields are allowed to remain absent. `--no-ai` constructs neither model client and emits deterministic decisions only. The separate legacy path retains its older tolerant fallback behavior.

### 5. Deterministic identity policy

Name has the largest weight. School and geography provide corroboration. Class year and Track/XC participation are smaller but material checks. An exact common name cannot become `MATCH` without corroboration. Explicit canonical-state or class-year conflicts remove corroboration and cap the score at the review threshold, including when configurable corroboration is disabled. State suffixes use longest matching names first, so West Virginia is not Virginia.

The model may add an explanation or recommend a lower status. Promotion above the deterministic result is disallowed unless the underlying structured evidence itself raises the Rust score.

Deterministic decisions require a unique qualifying candidate and sufficient runner-up margin for positive attribution. Nonempty but weak or ambiguous candidate sets remain `REVIEW`; only complete discovery with no candidates produces `NO_MATCH`. Scores are heuristic rankings, not calibrated probabilities. CSV exposes the original deterministic decision and the review mode separately.

### 6. Marks

Marks retain event, mark, season, date, meet, wind, and source URL. Event aliases map to stable keys. Rust parses times and distances to comparable numeric values and chooses the PR using event direction (lower for timed events, higher for distance/height events).

Records that fail mark validation remain in raw evidence but are not promoted into PR columns.

### 7. Failure and retry behavior

| Failure | Result |
|---|---|
| Athletic.net search/API unavailable | The affected row or alpha unit remains retryable; no credential guessing |
| Exhaustive local model unavailable, invalid JSON/schema, or invalid candidate index | Retryable `AI_ERROR`; row commit followed by nonzero exit |
| Legacy local model unavailable/invalid | Legacy deterministic fallback or review; not strict exhaustive proof |
| Page blocked/robots denied | Search evidence retained; no bypass attempted |
| Alpha response unauthorized, malformed, capped, or incomplete | Affected unit is checkpointed unresolved and the run fails closed |
| No candidate after every required search completes | `NO_MATCH` checkpointed |
| Neither source name present | Terminal `INPUT_ERROR`; no fabricated athlete |
| Interrupted process | Cancel uncommitted work, drain any started commit, export progress, then resume from durable outcomes |
| XLSX write-back fails | Source file remains untouched; output temp is not promoted |

Search transient failures use the configured bounded attempt count (three in the supplied configuration); model transport uses three attempts. Retry-After accepts delta seconds and HTTP dates, with a 60-second automatic-wait ceiling. Invalid or excessive delays fail explicitly. Search 403 opens a sticky circuit immediately, and accumulated 429 denials open the configured circuit without successful siblings resetting the count. Model schema/content errors do not consume automatic transport retries. The current CLI still stops after checkpointing the first retryable row; per-row fault isolation across the remaining population is not implemented.

The exhaustive runner holds an exclusive output-directory writer lock. Workbook/configuration/scope/review-mode/analysis-schema fingerprints reject incompatible resumes. JSONL persistence syncs complete newline-terminated records before exposing cache entries; only an unterminated final suffix may be repaired. Coverage is atomically replaced and reconciles checkpoint source keys against the full selected population. An AI second pass may explicitly import a compatible deterministic run's validated search cache under the donor's writer lock; it never imports deterministic decisions as AI-reviewed outcomes.

An unchanged completed run skips engine construction and all external requests. Candidate and decision caches are persistent but currently loaded into memory, as are source prospects and checkpoint records; bounded HTTP concurrency is not a claim of constant-memory operation. Cache appends and optional saved-page reads still perform synchronous file I/O in the async row path. These are explicit performance/review limitations, not passed architectural gates.

The strict compiler/Clippy gate is narrower than full architectural approval. The codebase still has functions exceeding the Farley 25-line/five-argument limits and source files exceeding 300 lines (including `coverage`, `discovery`, `extract`, and `xlsx`). Full black-hat approval is not claimed.

## Privacy and operational controls

- Email stays in local source/result records. Street/postal data stays out of search but is deliberately sent to the configured model endpoints; the supplied endpoints are local.
- Logs identify rows by source key and name, not email.
- Direct page retrieval requires two independent authorization controls.
- Alpha API collection requires a separate developer permission reference and explicit CLI acknowledgment.
- Host and route allow-lists reject redirects/candidates outside Athletic.net.
- There is no credential ingestion or authenticated-session capture.
- Source XLSX is never overwritten.

## Acceptance criteria

- `inspect` reports 111,939 `Export` data rows and 8,777 `Sheet1` data rows for the supplied workbook.
- A five-row `run` can be stopped and resumed without repeating completed rows.
- Every selected result retains candidate URLs and evidence.
- Exact-name/no-corroboration cases are never automatic `MATCH`.
- CSV profile links and XLSX profile cells are clickable.
- A write-back workbook opens with the two original worksheets plus `Athletic Matches`.
- `collect-authorized` refuses a disabled manifest before client/network construction.
- The alpha run writes separate source outputs and never replaces match decisions.
- A capped, malformed, or incomplete alpha response cannot be reported complete.
- Strict first-worksheet coverage reports 111,939 total rows and seven missing-name rows.
- A positive synthetic-search scenario with real Q5/Q4 models can match TF and XC despite a Basketball source sport, preserve all 13 source fields, and resume without external requests.
- Search/model failures cannot become `NO_MATCH`; malformed advertised search rows fail closed.
- No-AI runs make zero model requests, preserve deterministic decisions, and can supply validated cached searches to a separate AI-reviewed run.
- Same-name candidates with conflicting observed states cannot become corroborated matches; ambiguous rows retain evidence without attributed profile links.
- Transient failures recover within bounded retries; slow-search scenarios never exceed two in-flight search requests.
- Complete production delivery requires all 111,939 rows final, zero pending/retryable rows, and a verified zero-request unchanged resume. The observed live invalid-athlete-ID response currently blocks this criterion.
