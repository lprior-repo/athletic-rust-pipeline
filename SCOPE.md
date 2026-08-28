# Scope and architecture

## Data flow

```text
large XLSX
  -> streaming OOXML reader
  -> local-only row selection
  -> scoped Athletic.net candidate search
  -> deterministic search-evidence extraction
  -> optional authorization-gated Spider exact-page retrieval for best candidate
  -> local model structured extraction/validation
  -> Rust identity score + mark normalization
  -> append-only checkpoint
  -> CSV + JSONL
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
| `checkpoint` | Append and reload the latest row result | Delete previous evidence |
| `output` | Write audit JSONL, flat CSV, unresolved queue | Mutate source workbook rows |

## End-to-end stages

### 1. Workbook scan

The XLSX reader opens the ZIP container, streams `sharedStrings.xml`, resolves worksheet relationships, and streams each worksheet `<row>`. A row counts as real only when it contains at least one non-empty cell. This avoids the `Sheet1` inflated dimension.

Each selected prospect receives the immutable key `sheet_name:excel_row`. Every later output uses that key; names are never used as write-back keys.

### 2. Candidate discovery

For each selected athlete, queries are generated from:

- exact full name;
- school;
- city/state;
- expected class year;
- `site:athletic.net/athlete`.
Only URLs whose normalized host is `athletic.net` or `www.athletic.net` and whose path starts with `/athlete/` are retained. Completed row records checkpoint the query, URLs, snippets, extracted evidence, model decision, and scores under the immutable source key.

### 3. Retrieval

Default mode retains search-result title/snippet/URL as evidence and can ingest manually saved `<athlete-id>.html` pages. Authorized mode uses Spider with concurrency 1, a stop-after-seed callback, robots enabled, and a configured delay to retrieve each exact candidate URL. The pipeline has no anti-bot fallback.

If a site requires JavaScript, supply manually saved HTML or adapt the authorized retriever to Spider's `smart`/Chrome feature after confirming permission. The base build intentionally avoids browser automation.

### 4. Local model

The configured local model server receives only:

- prospect name, school, city/state, class year, and sport;
- candidate title/snippet/profile URL;
- compact public page text if authorized retrieval is enabled.

It never receives email, street address, postal code, or the full workbook row.

The model returns JSON. Invalid JSON, missing required fields, or an out-of-range candidate index becomes `REVIEW`, not a guessed match.

### 5. Deterministic identity policy

Name has the largest weight. School and geography provide corroboration. Class year and Track/XC participation are smaller but material checks. An exact common name cannot become `MATCH` without corroboration. Different states or conflicting class years apply penalties.

The model may add an explanation or recommend a lower status. Promotion above the deterministic result is disallowed unless the underlying structured evidence itself raises the Rust score.

### 6. Marks

Marks retain event, mark, season, date, meet, wind, and source URL. Event aliases map to stable keys. Rust parses times and distances to comparable numeric values and chooses the PR using event direction (lower for timed events, higher for distance/height events).

Records that fail mark validation remain in raw evidence but are not promoted into PR columns.

### 7. Failure and retry behavior

| Failure | Result |
|---|---|
| Athletic.net search unavailable | Row remains uncheckpointed for retry; error logged |
| Local model unavailable/invalid JSON | Deterministic result retained with `model_status=unavailable_or_invalid` |
| Model returns out-of-range candidate index | `REVIEW` with `model_status=invalid_index` |
| Page blocked/robots denied | Search evidence retained; no bypass attempted |
| No candidate | `NO_MATCH` checkpointed |
| Interrupted process | Resume skips completed `source_key` values |
| XLSX write-back fails | Source file remains untouched; output temp is not promoted |

## Privacy and operational controls

- No email or street/postal data leaves the workbook reader.
- Logs identify rows by source key and name, not email.
- Direct page retrieval requires two independent authorization controls.
- Host allow-list rejects redirects/candidates outside Athletic.net.
- There is no credential ingestion or authenticated-session support.
- Source XLSX is never overwritten.

## Acceptance criteria

- `inspect` reports 111,939 `Export` data rows and 8,777 `Sheet1` data rows for the supplied workbook.
- A five-row `run` can be stopped and resumed without repeating completed rows.
- Every selected result retains candidate URLs and evidence.
- Exact-name/no-corroboration cases are never automatic `MATCH`.
- CSV profile links and XLSX profile cells are clickable.
- A write-back workbook opens with the two original worksheets plus `Athletic Matches`.
