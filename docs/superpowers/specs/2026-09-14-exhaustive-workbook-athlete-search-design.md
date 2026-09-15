# Exhaustive Workbook Track and Cross-Country Search

## Problem

The existing `run` command streams the full workbook but selects prospects only when `Sports Sport` exactly matches configured Track & Field labels. That selection reduces the supplied workbook to 241 prospects before Athletic.net discovery begins. It therefore answers which already-tagged track athletes have profiles, not which people in the workbook participate in Track & Field or Cross Country.

The required behavior is the latter: search every real source record against both Athletic.net Track & Field and Cross Country results, run the full local-AI extraction and identity-review path on all discovered candidates, preserve all credible profiles and evidence, and return a reviewable outcome for every source row.

## Decision

Add an explicit exhaustive workbook mode and use it for the rerun. Exhaustive mode removes the source-sport eligibility gate, queries both Athletic.net sport filters for every person, escalates ambiguous searches through additional name and context variants, runs AI extraction for every discovered candidate and AI identity review for every non-empty candidate set, and checkpoints both discovery and final outcomes.

The supplied workbook contains 111,939 real rows in `Export` and 8,777 real rows in `Sheet1`, for 120,716 real source rows. Exhaustive mode processes both sheets. It does not collapse source rows: each row receives its own result keyed by sheet and Excel row. Shared request responses and Athletic.net profile candidates may be deduplicated internally.

## Goals

- Process every real source row on every source worksheet, including explicit input-error outcomes for rows without a searchable name.
- Search both Track & Field and Cross Country for every source person, regardless of `Sports Sport`.
- Maximize recall through adaptive name, school, and location query variants.
- Preserve every distinct Athletic.net athlete candidate discovered for a row.
- Run local-AI evidence extraction for every candidate and local-AI identity review for every row with candidates.
- Produce a terminal outcome and populated `AI Logic` field for every source row, including no-hit and error outcomes.
- Resume a multi-day run without repeating completed searches, AI work, or completed row resolution.
- Keep email, street address, and postal code local; send only name, school, city, state, expected graduation year, and sport scope.
- Carry every original source column into the row-level JSONL, CSV, unresolved, and `Athletic Matches` outputs without altering its value.
- Write complete CSV, JSONL, checkpoint, coverage, unresolved, and workbook outputs.

## Non-goals

- No login automation, CAPTCHA bypass, proxy rotation, access-control bypass, or stealth behavior.
- No inference of contact details or outreach automation.
- No claim that the workbook independently proves graduation year; the run uses the configured Class-of-2027 expectation.
- No silent promotion of weak same-name results into confirmed matches.
- No dependence on the source workbook's sport labels in exhaustive mode.
- No omission of AI processing merely because deterministic evidence looks sufficient.

## Population and identity

A source row is included when the streaming workbook parser recognizes it as a real, non-header row. A row is searchable when its first or last name is non-empty; the supplied workbook has seven real `Export` rows without either name, and those rows receive explicit input-error outcomes without network requests. The row identity remains `sheet:excel_row` so output can be written back without ambiguity.

The `Export` and `Sheet1` worksheets are separate source populations. They contain 120,716 real rows in total. Generated result/review sheets such as `Athletic Matches`, `Corrections`, and `Summary` are not source populations and must be excluded from later exhaustive scans. Exhaustive mode retains every real row from the two source sheets even when names, schools, or locations coincide. Network work may be shared when the complete discovery request key is identical, but results are scored and AI-reviewed independently against each source row.

The prospect's expected graduation year is `2027` from configuration. Because the workbook has no graduation-year column, this is run context rather than row-level evidence and must remain distinguishable from Athletic.net evidence.

## Source-data carry-through

Each result carries the complete original source row alongside the match result. For the supplied workbook, that means:

- `Person First`;
- `Person Last`;
- `Person Email`;
- `Address Mailing / Permanent Street Combined`;
- `Address Mailing / Permanent City`;
- `Address Mailing / Permanent Region`;
- `Address Mailing / Permanent Postal`;
- `Sports Created Date`;
- `Sports Sport`;
- `Sports Rating`;
- `Origin Source Date`;
- `Origin Source`;
- `Schools Name`.

These fields are copied locally and remain associated through `sheet:excel_row`; contact fields are never submitted to Athletic.net or the local model. The supplied workbook has no telephone or cellphone column—its worksheet range ends at column `M`, and the 13 headers above are the complete schema. The output must not invent a phone number. If a later input includes a configured phone/cellphone header, that field is carried through locally under its original header and is never used for discovery or AI.

## Discovery strategy

Each searchable source row searches both endpoint filters:

- `a:tf` for Track & Field;
- `a:xc` for Cross Country.

Candidate hits from both filters and every query stage are unioned by canonical Athletic.net athlete/profile identity. The final result records which filter and query produced each hit.

### Stage 1: full-name search

Issue the normalized full name under both sport filters. This creates a minimum of two logical searches per row.

### Stage 2: contextual search

Escalate when Stage 1 returns no candidate or does not produce one unambiguous, corroborated candidate. Search:

- full name plus school, when school is present;
- full name plus city and state, when either location field is present.

School and location searches are independent. A populated school must not suppress the location query.

### Stage 3: name-variant search

If the candidate set remains absent or ambiguous, search available distinct variants under both sport filters:

- punctuation- and diacritic-normalized name;
- family-name-first ordering;
- first initial plus family name.

Duplicate logical searches are removed before network execution. All hits survive until candidate union and scoring; an early weak hit must not prevent later variants from supplying the correct profile.

### Escalation rule

A row stops escalating only when one candidate is uniquely corroborated strongly enough to satisfy the deterministic `MATCH` floor and no competing candidate is within the configured ambiguity margin. `CLOSE_MATCH`, `REVIEW`, ties, and name-only hits continue through all applicable stages.

## Request control

The existing delay applies only between variants inside one prospect, allowing the first request for adjacent prospects to run without a delay. Exhaustive mode instead uses one global request gate shared by every Athletic.net request.

- Default concurrency: one request.
- Minimum interval: configured `search_delay_ms`, initially 750 ms, measured between request starts.
- Honor `Retry-After` when returned.
- Retry timeouts, connection failures, HTTP 429, and HTTP 5xx with bounded exponential backoff and jitter.
- Do not retry other HTTP 4xx responses.
- Stop issuing new requests after a configured consecutive 403/429 circuit-breaker threshold; preserve pending work for resume.
- Cache successful search responses by endpoint, filter, and canonical query.
- Persist cache records append-only so restarts do not repeat successful requests.

The 120,709 named rows require 241,418 Stage 1 logical searches before cache reuse; the seven blank-name rows receive local input-error outcomes. A 750 ms global interval imposes a theoretical minimum near 50.3 hours, excluding network latency, adaptive searches, candidate extraction, and identity-review time.

## Full AI processing

The exhaustive run retains the existing model-assisted behavior instead of replacing it with a deterministic-only fast pass.

### Candidate extraction

For every distinct candidate associated with a source row, send the complete bounded search evidence—and authorized exact-page evidence when available—to the configured local model. Require the model to return the structured candidate schema containing athlete name, school, location, explicit graduation year, sports, and marks. Rust then enriches only from independently present evidence, normalizes marks, and computes deterministic scores.

AI extraction results are checkpointed by the source-row and candidate-evidence fingerprint. Resume must not repeat a successful extraction. Deduplicating Athletic.net search requests must not incorrectly reuse an extraction across different source prospects because the prospect context and identity question differ.

### Identity review

For every row with one or more candidates, send the prospect plus all scored candidate summaries to the local model for the final identity review. The model may explain or lower confidence; it cannot override conflicting hard evidence or promote a candidate below the deterministic floor.

A successful search with no candidates does not require an empty model call. It receives `NO_MATCH`, `model_status=not_needed`, and explicit `AI Logic` stating that both sport filters and all applicable adaptive queries returned no Athletic.net candidate. Input and network errors likewise receive explicit `AI Logic` without fabricating a model decision.

### Profile evidence

Profile-page retrieval remains controlled by the existing explicit authorization setting and command-line acknowledgment. Exhaustive search itself must not silently enable page crawling. When authorized retrieval is active, the strongest configured candidates are fetched and passed through AI extraction again with the additional page evidence. Search-derived candidates and profile URLs remain useful when page retrieval is disabled.

## Checkpoints and resume

Use three append-only ledgers:

1. Search cache keyed by canonical endpoint/filter/query, containing response outcome, attempt metadata, and discovered hits.
2. AI cache keyed by source-row identity, candidate identity, evidence fingerprint, model, and prompt-schema version.
3. Row checkpoint keyed by `sheet:excel_row`, containing the final match record or explicit terminal error state.

A completed row is skipped on resume. A successful cached search or AI result is reused. Retryable search failures remain pending and are retried after restart. Corrupt trailing checkpoint lines fail with a precise location rather than silently discarding earlier state.

Configuration that affects population, query construction, scoring, model behavior, or endpoint filters contributes to a run fingerprint. Resume refuses a mismatched fingerprint unless the operator selects a new output directory.

## Outputs and completeness

The run writes:

- `matches.jsonl`: one complete record per finalized source row, including every original source field;
- `matches.csv`: flat review output containing every original source column plus selected profile, evidence, and full `AI Logic`;
- `unresolved.csv`: the same source-data columns for `CLOSE_MATCH`, `REVIEW`, `NO_MATCH`, and error outcomes;
- `checkpoint.jsonl`: append-only finalized row ledger;
- `search-cache.jsonl`: append-only logical search results;
- `ai-cache.jsonl`: append-only candidate extraction and identity-review results;
- `coverage.json`: expected rows, completed rows, successful searches, cached searches, AI extraction/review counts, pending retries, terminal errors, and status counts.

`writeback` copies the original workbook and replaces an existing generated `Athletic Matches` worksheet or creates it when absent. The sheet contains one row per source-row outcome, every original source column, and the match/AI columns; source sheet plus Excel row remain the stable keys. Generated review sheets are never treated as source input on a later run. A run is complete only when `coverage.json` reports 120,716 expected rows, 120,716 finalized rows, and zero pending retryable failures.

Errors are data, not omissions. A failed row remains in outputs with its failure class and message so the apparent match rate cannot be inflated by missing records.

## CLI behavior

Add an explicit exhaustive option to `run`, for example:

```text
athletic-rust-pipeline run --all-workbook-rows --both-track-and-xc ...
```

The existing sport-filtered mode remains available for bounded diagnostics. Exhaustive mode rejects conflicting population flags. Its startup summary prints real rows, eligible rows, target sport filters, resume counts, model configuration, and the estimated minimum request count before network activity begins.

The production rerun uses exhaustive mode, both sport filters, the configured local AI model, a fresh output directory, and no `--max` limit.

## Failure handling

- Invalid or missing names: finalize as an input error; do not issue an empty query.
- Search response schema failure: record retryable failure with bounded attempts.
- HTTP 403/429 burst: open the circuit breaker and stop cleanly for later resume.
- Local model unavailable: preserve the candidate evidence as an explicit retryable AI failure rather than silently treating the full-AI run as complete.
- Invalid model JSON: retry within a bound, then preserve an explicit AI error for resume/review.
- Output write failure: stop before marking the affected row complete.
- Workbook writeback failure: retain JSONL/CSV evidence and report the exact output stage.

## Verification

Behavioral tests must prove:

- exhaustive scanning includes rows with blank and unrelated `Sports Sport` values;
- both source worksheets and every real named row are included;
- each prospect searches both `a:tf` and `a:xc`;
- contextual and normalized variants run only under the escalation rule;
- school presence does not suppress city/state search;
- identical request keys use the persisted search cache;
- the global request gate separates adjacent prospects as well as variants;
- transient failures retry and circuit-breaking preserves resumability;
- no-hit is emitted only after all applicable stages complete successfully;
- AI extraction runs for every candidate;
- AI identity review runs for every non-empty candidate set, including deterministic matches;
- successful AI results are reused only when the row, evidence, model, and prompt schema match;
- AI failure prevents a candidate-bearing row from being falsely reported as a completed full-AI result;
- `CLOSE_MATCH` is present in unresolved output;
- resume does not repeat completed rows, successful searches, or successful AI calls;
- coverage cannot report complete while any row, search, or AI retry is pending;
- writeback contains one outcome and `AI Logic` value for every expected source row.
- output rows preserve every original source field exactly and never send email, street, postal, or future phone fields to Athletic.net or the model;

A bounded live smoke run uses a fresh output directory and a handful of known rows to verify the actual search endpoint, local model, checkpoint resume, both sport filters, and output generation. The full run starts only after that smoke path passes.

## Acceptance criteria

- No source-sport filter excludes a named row in exhaustive mode.
- All 120,716 real source rows receive final outputs.
- Every searchable row is queried for both Track & Field and Cross Country.
- Adaptive retries preserve all distinct candidate profiles.
- Every candidate receives AI extraction and every candidate-bearing row receives AI identity review.
- Every output row contains explicit `AI Logic`, including successful no-hit and error paths.
- Every output row carries all original source columns, including names, email, and full mailing address; no cellphone value is fabricated when the input has no cellphone column.
- The run survives interruption without repeating successful network or AI work.
- Final coverage accounts for every row and every non-success outcome.
- The enriched workbook and flat outputs are generated from the completed run.