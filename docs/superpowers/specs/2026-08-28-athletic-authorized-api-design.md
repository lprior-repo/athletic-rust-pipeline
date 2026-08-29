# Authorized Athletic.net Alpha Collection — Class of 2027

## Problem

The current pipeline starts with a supplied workbook and searches Athletic.net for a small set of candidate athlete profiles. It does not produce a complete, state-wide source list. The approved alpha needs to enumerate high-school performance records for the Class of 2027 across all 50 states, retain source URLs, deduplicate athletes, and feed the normalized records into the existing matching/output workflow.

The collection concerns public performance data for high-school athletes. It must not become a contact-list builder or collect unrelated personal information.

## Decision

Use the developer-authorized alpha rankings API as the primary source. Use Playwright only to validate the rendered site’s request/response contract and to exercise a bounded smoke path. Do not use DOM crawling as the normal extraction mechanism.

The current rendered rankings page issues a request to:

```text
POST /api/v1/tfRankings/GetRankings
```

The observed request envelope contains `reportType`, `mode`, `divListId`, `indoor`, `eventShort`, `gender`, `qParams`, `qualifyingListKey`, `version`, and `debug`. The adapter will send only parameters documented or explicitly confirmed by the Athletic.net alpha developers. It will not guess pagination, cohort, or page-size parameters.

The authorization basis is recorded in a local manifest. The program refuses collection without an enabled manifest and an explicit command-line acknowledgment. The manifest records the permission reference, allowed sports/routes, fields, request rate, page-size/continuation rule, and retention limits supplied by the developers.

## Goals

- Enumerate all 50 U.S. states without duplicating state subregions or conference views.
- Collect Class-of-2027 high-school Track & Field records from the authorized alpha surface.
- Preserve athlete/profile URLs and individual result URLs when returned by the API.
- Capture athlete identity, school, state, graduation/cohort evidence, gender, sport, event, mark, season, date, meet, and source metadata.
- Detect and report incomplete pages, API caps, malformed records, authorization failures, and rate-limit responses.
- Resume from checkpoints at state/gender/season/event/page granularity.
- Deduplicate by stable Athletic.net athlete ID while preserving all associated results and source URLs.
- Produce CSV/JSONL suitable for the existing matcher and workbook write-back flow.
- Keep the existing privacy boundary: no email, phone, street address, postal code, or direct outreach automation.

## Non-goals

- No unauthorised scraping or broad crawling.
- No stealth browser behavior, proxy rotation, CAPTCHA bypass, login automation, or access-control bypass.
- No reverse engineering beyond the alpha contract confirmed by the developers.
- No collection of athlete contact details or inferred contact details.
- No automatic messaging, recruiting outreach, or solicitation.
- No Cross Country collection in the first implementation unless separately enabled in the authorization manifest.
- No replacement of the existing deterministic identity matcher.

## Cohort definition

The target cohort is graduation year 2027.

Cohort evidence is applied in this order:

1. Explicit API graduation year equal to 2027.
2. An explicit profile/result class-year field equal to 2027, if the authorized response provides one.
3. Season/grade mapping only when the season and grade are both present:
   - grade 11 in the 2025–26 high-school season;
   - grade 12 in the 2026–27 high-school season.
4. Missing or conflicting cohort evidence is retained in an exception output and excluded from the definitive Class-of-2027 list.

The implementation will not treat an athlete’s age, name, school year inferred from a current date, or ranking position as cohort evidence.

## Coverage model

The source catalog contains exactly the 50 U.S. states. District of Columbia, overseas regions, country-wide views, California subregions, Texas subregions, leagues, conferences, and divisions are excluded from the state sweep unless an authorization manifest explicitly adds them.

The run matrix is:

```text
state × outdoor/indoor season allowed by manifest × gender × event × API continuation page
```

The catalog stores the state identifier required by the alpha API, canonical state code, display name, and source route. State identifiers are not inferred from display names at runtime.

All event identifiers and their directionality are supplied by the authorized alpha contract or a checked-in catalog generated from that contract. Timed events rank lower marks as better; distance/height events rank higher marks as better. Relay and combined-event handling follows the existing marks normalizer and is not silently discarded.

## Components

### Authorization manifest

A versioned local configuration declares:

- alpha permission reference and effective scope;
- API base URL and permitted route;
- permitted sports, seasons, states, genders, and fields;
- request rate and maximum concurrent requests;
- page-size and continuation semantics;
- whether profile/result enrichment is allowed;
- retention and output restrictions.

Secrets and session tokens are not committed, logged, or written into output records. The CLI acknowledgment is separate from the manifest so a copied configuration cannot silently enable collection.

### State and event catalog

A checked-in catalog maps each authorized state and event to stable API identifiers. Catalog validation rejects duplicate canonical states, missing required identifiers, and entries outside the manifest.

### Alpha API client

The client constructs one request from one run-matrix item, applies the documented continuation rule, enforces timeout/rate/concurrency limits, validates HTTP status and JSON shape, and returns typed ranking records plus completeness metadata. It must not retry authorization failures or rate-limit responses aggressively.

### Cohort filter

The filter records the exact evidence used for inclusion. A record without acceptable Class-of-2027 evidence is written to `cohort-exceptions.jsonl`, not silently promoted into the main list.

### Normalizer and deduplicator

The normalizer canonicalizes event names, marks, school names, state codes, dates, and URLs. The deduplicator keys athletes by stable athlete ID. If the API provides no stable ID, the record is exception-only; name-based identity is not an acceptable primary key for the source list.

Every retained athlete contains all distinct profile URLs and every retained result contains its source result URL when available. Duplicate result records are collapsed only when athlete ID, event, mark, date, meet, and source identity all agree.

### Checkpoint and completeness ledger

Each completed unit records state, gender, season, event, continuation token/page, response count, and completeness status. A run is successful only when every planned unit is complete or explicitly recorded as an authorized empty result. A response that appears capped, truncated, or missing continuation metadata fails closed and remains resumable.

### Outputs

The authorized source run writes a separate directory, by default `out-authorized-2027/`:

- `athletes.csv`: one normalized athlete-level row with cohort evidence and aggregated URLs;
- `athletes.jsonl`: full audit records and all source evidence;
- `results.jsonl`: normalized event/result records;
- `cohort-exceptions.jsonl`: missing or conflicting cohort evidence;
- `unresolved.csv`: malformed, incomplete, unauthorized, or otherwise review-required records;
- `checkpoint.jsonl`: append-only resumable state;
- `coverage.json`: planned/completed state-event matrix and completeness summary.

The existing `matches.csv`, `matches.jsonl`, and `Athletic Matches` worksheet remain the final workbook-matching outputs. The new source list is an input to matching, not a replacement for match decisions.

## Data flow

```text
authorized manifest + state/event catalog
  -> run-matrix planner
  -> alpha API client
  -> typed response validation
  -> cohort evidence filter
  -> event/mark normalization
  -> athlete-ID deduplication
  -> coverage ledger + checkpoints
  -> athletes.csv / athletes.jsonl / results.jsonl
  -> existing matcher
  -> matches.csv + matches.jsonl + Athletic Matches workbook
```

Playwright is used before a production-sized run for one authorized state, one gender, and one event. The smoke path compares the rendered request shape and response completeness with the adapter. It does not iterate all states or capture session credentials.

## Failure behavior

- Missing or disabled manifest: refuse to start.
- Route or field outside manifest: refuse the request and record configuration error.
- HTTP 401/403 or authorization response: stop the affected unit; no credential guessing or retries.
- HTTP 429: honor the documented retry-after/rate rule; if absent, stop the affected unit.
- Timeout or transient 5xx: bounded retry with checkpoint-safe backoff; unresolved after the limit remains resumable.
- Invalid JSON or missing required response fields: exception output; never fabricate records.
- Apparent top-N cap or absent continuation metadata: mark the unit incomplete and fail the run summary.
- Missing/conflicting Class-of-2027 evidence: exception output, excluded from definitive list.
- Duplicate athlete IDs: merge evidence; never overwrite a stronger source with a weaker one.
- Interrupted process: resume from the last completed unit without repeating completed units.
- Matcher ambiguity: preserve `REVIEW`, `NO_MATCH`, or existing deterministic status rules.

## Verification

The implementation is accepted only when these observable checks pass:

1. A one-state API fixture parses the confirmed alpha response into typed athlete/result records without exposing tokens.
2. The Playwright smoke path confirms the adapter’s request fields and detects a deliberately truncated response as incomplete.
3. The state catalog contains exactly 50 canonical state entries and no excluded region duplicates.
4. A fixture covering explicit graduation year, grade/season fallback, missing year, and conflicting year produces the specified cohort decisions.
5. Duplicate athlete/result fixtures merge by stable ID while preserving distinct profile and result URLs.
6. Checkpoint/resume tests skip completed matrix units and retry only incomplete units.
7. Rate-limit, authorization, malformed JSON, and missing-continuation cases fail closed with actionable exception output.
8. A bounded live smoke run completes one authorized state/gender/event and writes all required output files.
9. The existing matcher consumes the normalized source list and retains its exact-name/no-corroboration `REVIEW` rule.
10. A source scan confirms no email, phone, street, postal, token, or session-cookie value appears in logs or generated outputs.
11. Final output coverage reports all planned state/event units as complete or explicitly excepted; no “complete” claim is emitted for a capped response.

## Operational boundary

The alpha permission governs only the routes, fields, and limits stated by the developers. If the permission changes, the manifest must change before the next run. The pipeline remains intentionally conservative: it prefers an incomplete, reviewable run over an unverified nationwide list.
