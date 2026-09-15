# Exhaustive Workbook Athlete Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Search every real workbook row against Athletic.net Track & Field and Cross Country, run the full local-AI candidate pipeline, preserve every source field, and launch a resumable exhaustive run.

**Architecture:** Add an explicit exhaustive scan mode that retains all source rows and fields. Move the long-running workflow into a focused `exhaustive` module backed by persistent search and AI caches, a globally rate-limited adaptive discovery client, explicit coverage accounting, and existing deterministic scoring. Keep the original sport-filtered run available for diagnostics.

**Tech Stack:** Rust 2021, Tokio, Reqwest, Serde/JSONL, Quick XML/ZIP OOXML streaming, CSV, SHA-256 fingerprints, Clap, Mockito, local llama.cpp OpenAI-compatible API.

---

## File structure

- Modify `Cargo.toml`: add SHA-256 support used for stable run and AI-cache fingerprints.
- Modify `src/commands.rs`: add exhaustive CLI flags.
- Modify `src/config.rs`: add bounded retry, circuit-breaker, and ambiguity-margin settings with defaults.
- Modify `src/model.rs`: retain original source fields and search provenance; add explicit error details.
- Modify `src/xlsx.rs`: implement `ScanMode`, include every real row in exhaustive mode, and emit source fields in writeback.
- Modify `src/discovery.rs`: expose staged query planning, both sport filters, a global request gate, retries, and canonical candidate union.
- Create `src/search_cache.rs`: append-only search-response cache with retryable failure semantics.
- Create `src/ai_cache.rs`: append-only candidate-extraction and identity-review cache with stable fingerprints.
- Create `src/coverage.rs`: exhaustive-run accounting and atomic `coverage.json` writes.
- Create `src/exhaustive.rs`: orchestrate scan, adaptive searches, AI extraction/review, checkpoints, outputs, and resume.
- Modify `src/extract.rs`: expose cacheable AI extraction results without weakening the full-AI path.
- Modify `src/checkpoint.rs`: classify finalized versus retryable records and bind checkpoints to a run fingerprint.
- Modify `src/output.rs`: carry all source columns and include `CLOSE_MATCH` plus errors in unresolved output.
- Modify `src/main.rs`: delegate exhaustive execution without growing the existing orchestration body.
- Modify `config.toml` and `config.example.toml`: document exhaustive request controls.
- Modify `README.md`: document exhaustive invocation, outputs, duration floor, AI behavior, and source contact-field handling.

### Task 1: Preserve source rows and fields in the domain model

**Files:**
- Modify: `src/model.rs:29-130`
- Modify: `src/xlsx.rs:17-94`
- Test: `src/xlsx.rs` test module

- [ ] **Step 1: Write failing scanner tests**

Add fixtures covering a blank sport, an unrelated sport, and a blank-name real row. Assert filtered mode retains existing behavior while exhaustive mode returns all three records and their complete field maps:

```rust
#[test]
fn exhaustive_scan_keeps_every_real_row_and_source_field() {
    let path = workbook_fixture(&[
        &["Ada", "Lovelace", "ada@example.test", "1 Main", "London", "CA", "90001", "", "", "", "", "", "Academy"],
        &["Grace", "Hopper", "grace@example.test", "2 Main", "Arlington", "VA", "22201", "", "Swimming and Diving: Womens", "", "", "", "Central"],
        &["", "", "unknown@example.test", "3 Main", "Boston", "MA", "02108", "", "", "", "", "", ""],
    ]);
    let result = scan(&path, ScanMode::Exhaustive, Some(2027)).unwrap();
    assert_eq!(result.prospects.len(), 3);
    assert_eq!(result.prospects[0].source_fields["Person Email"], "ada@example.test");
    assert_eq!(result.prospects[1].source_fields["Sports Sport"], "Swimming and Diving: Womens");
    assert!(result.prospects[2].full_name().is_empty());
}

#[test]
fn filtered_scan_still_selects_only_configured_sports() {
    let path = workbook_fixture_with_sports(&["", "Swimming", "Track and Field: Womens"]);
    let result = scan(
        &path,
        ScanMode::Sports(vec!["Track and Field: Womens".to_owned()]),
        Some(2027),
    )
    .unwrap();
    assert_eq!(result.prospects.len(), 1);
}

#[test]
fn exhaustive_scan_ignores_generated_result_sheets() {
    let path = workbook_fixture_with_sheets(&[
        ("Export", source_headers(), vec![source_row("Ada", "Lovelace")]),
        ("Sheet1", source_headers(), vec![source_row("Grace", "Hopper")]),
        ("Athletic Matches", &["Source Key", "Status"], vec![vec!["Export:2", "MATCH"]]),
    ]);
    let result = scan(&path, ScanMode::Exhaustive, Some(2027)).unwrap();
    assert_eq!(result.stats.actual_data_rows, 2);
    assert_eq!(result.prospects.len(), 2);
    assert!(result.prospects.iter().all(|value| value.sheet != "Athletic Matches"));
}
```

- [ ] **Step 2: Run scanner tests and confirm failure**

Run: `cargo test xlsx::tests::exhaustive_scan_keeps_every_real_row_and_source_field xlsx::tests::filtered_scan_still_selects_only_configured_sports xlsx::tests::exhaustive_scan_ignores_generated_result_sheets -- --nocapture`

Expected: compilation fails because `ScanMode` and `Prospect::source_fields` do not exist.

- [ ] **Step 3: Add scan mode and source fields**

Add to `src/xlsx.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanMode {
    Sports(Vec<String>),
    Exhaustive,
}
```

Change `scan` to accept `ScanMode`. Before parsing row data, classify sheets whose names are `Athletic Matches`, `Corrections`, or `Summary` as generated output and exclude them from both exhaustive population counts and prospects. In `Sports`, apply the current exact normalized-sport predicate and skip blank names. In `Exhaustive`, push every parsed source `SourceRecord`, including blank-name rows. Add this field to `Prospect`:

```rust
#[serde(default)]
pub source_fields: BTreeMap<String, String>,
```

Construct it with `source_fields: record.fields`. Update `summary::inspect` to call `ScanMode::Sports(Vec::new())`, and update `alpha_match` plus the legacy `run` path to call `ScanMode::Sports(config.workbook.sports.clone())`. The exhaustive orchestrator added in Task 6 calls `ScanMode::Exhaustive`.

- [ ] **Step 4: Run scanner and existing XLSX tests**

Run: `cargo test xlsx::tests -- --nocapture`

Expected: all XLSX tests pass; the exhaustive row fixture contains three prospects, the generated-sheet fixture contains only the two source prospects, and the filtered fixture contains one.

- [ ] **Step 5: Commit source preservation**

```bash
git add src/model.rs src/xlsx.rs src/summary.rs src/alpha_match.rs src/main.rs
git commit -m "feat: scan every workbook row"
```

### Task 2: Carry original columns through all outputs

**Files:**
- Modify: `src/output.rs:29-141`
- Modify: `src/xlsx.rs:545-644`
- Test: `src/output.rs`
- Test: `src/xlsx.rs`

- [ ] **Step 1: Write failing output tests**

Create one `MatchRecord` whose `prospect.source_fields` contains all 13 supplied headers. Write flat output and workbook XML, then assert exact headers and values:

```rust
#[test]
fn csv_carries_original_source_columns_before_match_columns() {
    let record = record_with_source_fields();
    let dir = tempfile::tempdir().unwrap();
    write_all(dir.path(), &[record]).unwrap();
    let text = std::fs::read_to_string(dir.path().join("matches.csv")).unwrap();
    assert!(text.lines().next().unwrap().starts_with(
        "Person First,Person Last,Person Email,Address Mailing / Permanent Street Combined"
    ));
    assert!(text.contains("ada@example.test,1 Main"));
}

#[test]
fn unresolved_contains_close_match_and_source_address() {
    let mut record = record_with_source_fields();
    record.status = "CLOSE_MATCH".to_owned();
    let dir = tempfile::tempdir().unwrap();
    write_all(dir.path(), &[record]).unwrap();
    let text = std::fs::read_to_string(dir.path().join("unresolved.csv")).unwrap();
    assert!(text.contains("CLOSE_MATCH"));
    assert!(text.contains("1 Main"));
}
```

Add XLSX tests asserting the generated `Athletic Matches` XML contains `Person Email`, the street value, and `AI Logic`, and asserting writeback replaces one existing generated `Athletic Matches` sheet instead of rejecting it or creating a duplicate:

```rust
#[test]
fn writeback_replaces_existing_athletic_matches_sheet() {
    let input = workbook_fixture_with_existing_matches_sheet();
    let output = tempfile::tempdir().unwrap().path().join("replaced.xlsx");
    append_matches_sheet(&input, &output, &[record_with_source_fields()]).unwrap();
    let names = workbook_sheet_names(&output);
    assert_eq!(names.iter().filter(|name| name.as_str() == "Athletic Matches").count(), 1);
    assert!(worksheet_text(&output, "Athletic Matches").contains("ada@example.test"));
}
```

- [ ] **Step 2: Run output tests and confirm failure**

Run: `cargo test output::tests xlsx::tests::matches_sheet_carries_source_fields xlsx::tests::writeback_replaces_existing_athletic_matches_sheet -- --nocapture`

Expected: source headers are missing, `CLOSE_MATCH` is absent from unresolved output, and writeback rejects the existing generated sheet.

- [ ] **Step 3: Implement stable source-column projection**

Define the supplied source header order once in `model.rs`:

```rust
pub const SOURCE_HEADERS: &[&str] = &[
    "Person First",
    "Person Last",
    "Person Email",
    "Address Mailing / Permanent Street Combined",
    "Address Mailing / Permanent City",
    "Address Mailing / Permanent Region",
    "Address Mailing / Permanent Postal",
    "Sports Created Date",
    "Sports Sport",
    "Sports Rating",
    "Origin Source Date",
    "Origin Source",
    "Schools Name",
];
```

Have CSV and XLSX writers prepend those headers and read values using `source_fields.get(header).map_or("", String::as_str)`. Include extra source headers after the known 13 in sorted order so later phone/cellphone fields carry through unchanged. Change unresolved filtering to:

```rust
matches!(
    record.status.as_str(),
    "CLOSE_MATCH" | "REVIEW" | "NO_MATCH" | "INPUT_ERROR" | "SEARCH_ERROR" | "AI_ERROR"
)
```

Never add or infer a cellphone value when no source header exists.

Change `append_matches_sheet` to replace an existing `Athletic Matches` worksheet in place: reuse its sheet ID, relationship, and ZIP member path; copy every unrelated workbook member unchanged; write one replacement XML member; and never emit a duplicate ZIP member or workbook `<sheet>` entry. Retain the current create-new-sheet path when no generated match sheet exists.

- [ ] **Step 4: Run output and XLSX tests**

Run: `cargo test output::tests xlsx::tests -- --nocapture`

Expected: source fields appear exactly in CSV and worksheet XML; `CLOSE_MATCH` is unresolved.

- [ ] **Step 5: Commit source-field outputs**

```bash
git add src/model.rs src/output.rs src/xlsx.rs
git commit -m "feat: carry source fields into match outputs"
```

### Task 3: Build staged dual-sport query plans

**Files:**
- Modify: `src/model.rs:51-57`
- Modify: `src/discovery.rs:88-165`
- Test: `src/discovery.rs`

- [ ] **Step 1: Write failing query-plan tests**

```rust
#[test]
fn first_stage_queries_full_name_for_both_sports() {
    let plan = QueryPlan::for_prospect(&prospect("José Smith", "Central", "Austin", "TX"));
    assert_eq!(plan.stage(0), &[
        SearchRequest::new("José Smith", "a:tf"),
        SearchRequest::new("José Smith", "a:xc"),
    ]);
}

#[test]
fn context_stage_keeps_school_and_location_queries() {
    let plan = QueryPlan::for_prospect(&prospect("Ada Runner", "Central", "Austin", "TX"));
    let stage = plan.stage(1);
    assert!(stage.contains(&SearchRequest::new("Ada Runner Central", "a:tf")));
    assert!(stage.contains(&SearchRequest::new("Ada Runner Austin TX", "a:tf")));
    assert!(stage.contains(&SearchRequest::new("Ada Runner Central", "a:xc")));
    assert!(stage.contains(&SearchRequest::new("Ada Runner Austin TX", "a:xc")));
}

#[test]
fn query_plan_deduplicates_identical_normalized_variants() {
    let plan = QueryPlan::for_prospect(&prospect("Ada Runner", "", "", ""));
    let keys = plan.all().map(SearchRequest::cache_key).collect::<BTreeSet<_>>();
    assert_eq!(keys.len(), plan.all().count());
}
```

- [ ] **Step 2: Run query tests and confirm failure**

Run: `cargo test discovery::tests::first_stage_queries_full_name_for_both_sports discovery::tests::context_stage_keeps_school_and_location_queries discovery::tests::query_plan_deduplicates_identical_normalized_variants -- --nocapture`

Expected: compilation fails because `QueryPlan` and `SearchRequest` do not exist.

- [ ] **Step 3: Implement query plan and provenance**

Add serializable types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SearchRequest {
    pub query: String,
    pub filter: String,
    pub stage: u8,
}

#[derive(Debug, Clone)]
pub struct QueryPlan {
    stages: Vec<Vec<SearchRequest>>,
}
```

Generate Stage 0 full-name requests, Stage 1 independent school and location requests, and Stage 2 normalized/reversed/initial variants. Pair every distinct query with both `a:tf` and `a:xc`. Add `filter: String` to `SearchHit` so output records whether Track or XC found it. Canonicalize each cache key as `filter + "\n" + collapsed-lowercase-query`.

- [ ] **Step 4: Run discovery unit tests**

Run: `cargo test discovery::tests -- --nocapture`

Expected: existing URL allow-list tests and all staged-query tests pass.

- [ ] **Step 5: Commit dual-sport query planning**

```bash
git add src/model.rs src/discovery.rs
git commit -m "feat: plan adaptive track and xc searches"
```

### Task 4: Add persistent globally rate-limited search execution

**Files:**
- Create: `src/search_cache.rs`
- Modify: `src/discovery.rs:15-86`
- Modify: `src/config.rs:23-29,73-92`
- Modify: `src/main.rs:1-65`
- Test: `src/search_cache.rs`
- Test: `src/discovery.rs`

- [ ] **Step 1: Write failing cache and request-gate tests**

Test append/load replacement, cache hits, adjacent-prospect spacing, retry classification, `Retry-After`, and circuit breaking with Mockito. Use Tokio paused time only after adding `test-util` to Tokio features; otherwise use a 500 ms configured interval and assert elapsed time is at least 450 ms.

```rust
#[tokio::test]
async fn global_gate_spaces_requests_from_different_prospects() {
    let server = mockito::Server::new_async().await;
    let mock = server.mock("POST", "/search").with_status(200)
        .with_body(r#"{"d":{"results":""}}"#).expect(2).create_async().await;
    let client = test_client(server.url(), Duration::from_millis(500));
    let start = Instant::now();
    client.execute(&SearchRequest::new("Ada One", "a:tf")).await.unwrap();
    client.execute(&SearchRequest::new("Grace Two", "a:xc")).await.unwrap();
    assert!(start.elapsed() >= Duration::from_millis(450));
    mock.assert_async().await;
}

#[test]
fn cache_latest_record_wins_and_retryable_failure_is_not_success() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("search-cache.jsonl");
    append_search(&path, &SearchCacheRecord::retryable("a:tf\nada", "timeout")).unwrap();
    append_search(&path, &SearchCacheRecord::success("a:tf\nada", vec![])).unwrap();
    let cache = load_searches(&path).unwrap();
    assert!(cache["a:tf\nada"].is_success());
}
```

- [ ] **Step 2: Run tests and confirm failure**

Run: `cargo test search_cache discovery::tests::global_gate_spaces_requests_from_different_prospects -- --nocapture`

Expected: compilation fails because the cache and global gate do not exist.

- [ ] **Step 3: Implement append-only search cache**

Create `SearchCacheRecord` with `key`, `query`, `filter`, `attempts`, `status`, `hits`, `error`, `retryable`, and `completed_at_unix`. `load_searches` reads every non-empty JSONL line with line-number context and retains the latest record per key. `append_search` flushes one complete line before returning.

- [ ] **Step 4: Implement one global request gate and bounded retries**

Enable Tokio's `sync` feature in `Cargo.toml`, then store `next_request_at: tokio::sync::Mutex<Instant>` on `AthleticNetClient`. Before every request, reserve the next slot and sleep until it. Add config defaults:

```rust
fn default_max_attempts() -> u32 { 4 }
fn default_circuit_breaker_threshold() -> u32 { 8 }
fn default_ambiguity_margin() -> f64 { 0.03 }
```

Add `max_attempts`, `circuit_breaker_threshold`, and `ambiguity_margin` fields with Serde defaults. Retry connection/timeouts, 429, and 5xx with bounded exponential backoff capped at 60 seconds; honor a larger valid `Retry-After` duration. Return typed `SearchFailure { message, retryable, status }`. Open the circuit after the configured consecutive 403/429 count and return a retryable stop condition without issuing more requests.

- [ ] **Step 5: Run cache and discovery tests**

Run: `cargo test search_cache discovery::tests -- --nocapture`

Expected: cache replay, request spacing, retry, and circuit-breaker tests pass.

- [ ] **Step 6: Commit reliable search execution**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/config.rs src/discovery.rs src/search_cache.rs
git commit -m "feat: persist and rate-limit athlete searches"
```

### Task 5: Cache the full AI extraction and review path

**Files:**
- Modify: `Cargo.toml`
- Create: `src/ai_cache.rs`
- Modify: `src/extract.rs:49-62,181-335,337-382`
- Modify: `src/main.rs:1-65`
- Test: `src/ai_cache.rs`
- Test: `src/extract.rs`

- [ ] **Step 1: Write failing fingerprint/cache tests**

```rust
#[test]
fn extraction_key_changes_with_row_evidence_model_or_schema() {
    let base = extraction_key("Export:2", "https://www.athletic.net/athlete/7", "evidence", "model-a", 1);
    assert_ne!(base, extraction_key("Export:3", "https://www.athletic.net/athlete/7", "evidence", "model-a", 1));
    assert_ne!(base, extraction_key("Export:2", "https://www.athletic.net/athlete/7", "changed", "model-a", 1));
    assert_ne!(base, extraction_key("Export:2", "https://www.athletic.net/athlete/7", "evidence", "model-b", 1));
    assert_ne!(base, extraction_key("Export:2", "https://www.athletic.net/athlete/7", "evidence", "model-a", 2));
}

#[test]
fn ai_cache_round_trips_candidate_and_decision() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ai-cache.jsonl");
    append_ai(&path, &AiCacheRecord::candidate("key", Candidate::default())).unwrap();
    append_ai(&path, &AiCacheRecord::decision("decision-key", ModelDecision::default())).unwrap();
    let cache = load_ai(&path).unwrap();
    assert!(matches!(cache["key"].value, AiCacheValue::Candidate(_)));
    assert!(matches!(cache["decision-key"].value, AiCacheValue::Decision(_)));
}
```

- [ ] **Step 2: Run tests and confirm failure**

Run: `cargo test ai_cache extract::tests -- --nocapture`

Expected: compilation fails because AI cache types and public cacheable extraction methods do not exist.

- [ ] **Step 3: Implement stable SHA-256 keys and append-only AI cache**

Add `sha2 = "0.10"`. Hash length-prefixed components to avoid concatenation ambiguity. Define:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum AiCacheValue {
    Candidate(Candidate),
    Decision(ModelDecision),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCacheRecord {
    pub key: String,
    pub value: AiCacheValue,
    pub completed_at_unix: u64,
}
```

Expose the bounded evidence builder and a method that performs exactly one full candidate extraction. Keep fallback extraction only for explicitly recorded model failures; exhaustive orchestration must treat failed required AI work as `AI_ERROR`, not as a successful full-AI record.

- [ ] **Step 4: Add a model-server test proving full AI calls occur**

Mock two candidate-extraction responses and one identity-review response. Assert three requests occur even when the first candidate's deterministic score already exceeds `match_threshold`. Assert a second run reuses all three AI-cache records and performs zero model requests.

- [ ] **Step 5: Run AI tests**

Run: `cargo test ai_cache extract::tests -- --nocapture`

Expected: fingerprints vary on every required dimension, records round-trip, and full-AI calls are cached.

- [ ] **Step 6: Commit AI caching**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/ai_cache.rs src/extract.rs
git commit -m "feat: cache full AI athlete analysis"
```

### Task 6: Orchestrate exhaustive adaptive processing

**Files:**
- Create: `src/exhaustive.rs`
- Modify: `src/commands.rs:18-33`
- Modify: `src/main.rs:78-143`
- Modify: `src/checkpoint.rs`
- Modify: `src/scoring.rs`
- Test: `src/exhaustive.rs`

- [ ] **Step 1: Write failing end-to-end orchestration tests**

Use a two-row XLSX fixture, Mockito search endpoint, and mock model endpoint. Cover: both filters, adaptive escalation after a no-hit, no escalation after a unique corroborated match, blank-name `INPUT_ERROR`, and resume with zero repeated requests.

```rust
#[tokio::test]
async fn exhaustive_run_finalizes_every_row_and_resumes_without_repeating_work() {
    let fixture = exhaustive_fixture_with_named_and_blank_rows();
    let services = MockServices::track_hit_and_xc_no_hit().await;
    let out = tempfile::tempdir().unwrap();
    run_exhaustive(&fixture, &services.config, out.path(), Some(2), false).await.unwrap();
    let first = output::read_jsonl(&out.path().join("matches.jsonl")).unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first[1].status, "INPUT_ERROR");
    services.assert_search_and_ai_counts(2, 2);
    run_exhaustive(&fixture, &services.config, out.path(), Some(2), false).await.unwrap();
    services.assert_search_and_ai_counts(2, 2);
}
```

- [ ] **Step 2: Run orchestration test and confirm failure**

Run: `cargo test exhaustive::tests::exhaustive_run_finalizes_every_row_and_resumes_without_repeating_work -- --nocapture`

Expected: compilation fails because `run_exhaustive` does not exist.

- [ ] **Step 3: Add CLI flags and conflict validation**

Extend `Run`:

```rust
#[arg(long)]
all_workbook_rows: bool,
#[arg(long)]
both_track_and_xc: bool,
```

Require both flags together for exhaustive behavior. Reject `--include-xc` combined with exhaustive mode. Preserve `--max` for bounded smoke runs.

- [ ] **Step 4: Implement exhaustive row state machine**

For each prospect:

1. Emit `INPUT_ERROR` immediately for an empty name.
2. Execute or reuse Stage 0 searches.
3. Parse all hits deterministically enough to decide escalation; if no unique corroborated deterministic match, execute Stage 1 and then Stage 2.
4. Union candidates by canonical athlete URL/ID while retaining every query and filter provenance.
5. Run or reuse full AI candidate extraction for every candidate.
6. Score every candidate.
7. Run or reuse AI identity review for every non-empty candidate set.
8. Finalize and append the row only after required search and AI work succeeds.
9. Preserve retryable failures without marking the row complete.

Represent fatal per-row outcomes as `SEARCH_ERROR` or `AI_ERROR` only after configured attempts are exhausted. A clean endpoint no-hit after all stages is `NO_MATCH` with explicit `model_status=not_needed` AI logic.

- [ ] **Step 5: Correct deterministic evidence defects encountered by the approved workflow**

Add tests before each correction:

- state abbreviations match normalized tokens, not arbitrary substrings;
- only explicit `class of 2027`, `graduation year 2027`, or equivalent profile fields count as year evidence;
- date/season ranges such as `2024-2026` do not become graduation years;
- location queries remain independent from school queries.

Run: `cargo test scoring::tests extract::tests discovery::tests -- --nocapture`

Expected: all evidence-boundary tests pass.

- [ ] **Step 6: Run exhaustive integration tests**

Run: `cargo test exhaustive::tests -- --nocapture`

Expected: every fixture row finalizes, required AI calls happen, and resume repeats no successful work.

- [ ] **Step 7: Commit exhaustive orchestration**

```bash
git add src/commands.rs src/main.rs src/exhaustive.rs src/checkpoint.rs src/scoring.rs src/discovery.rs src/extract.rs
git commit -m "feat: run exhaustive adaptive athlete matching"
```

### Task 7: Add coverage accounting and run fingerprints

**Files:**
- Create: `src/coverage.rs`
- Modify: `src/exhaustive.rs`
- Modify: `src/checkpoint.rs`
- Modify: `src/main.rs`
- Test: `src/coverage.rs`

- [ ] **Step 1: Write failing completeness tests**

```rust
#[test]
fn coverage_is_complete_only_when_every_row_is_final_and_no_retry_is_pending() {
    let mut report = CoverageReport::new(120_716, "fingerprint".to_owned());
    report.finalized_rows = 120_716;
    report.pending_retryable_searches = 1;
    assert!(!report.is_complete());
    report.pending_retryable_searches = 0;
    report.pending_retryable_ai = 1;
    assert!(!report.is_complete());
    report.pending_retryable_ai = 0;
    assert!(report.is_complete());
}

#[test]
fn mismatched_run_fingerprint_is_rejected() {
    let existing = RunMetadata { fingerprint: "old".to_owned(), expected_rows: 2 };
    let error = validate_resume(&existing, "new", 2).unwrap_err();
    assert!(error.to_string().contains("run fingerprint mismatch"));
}
```

- [ ] **Step 2: Run coverage tests and confirm failure**

Run: `cargo test coverage -- --nocapture`

Expected: compilation fails because coverage types do not exist.

- [ ] **Step 3: Implement coverage and atomic writes**

Track expected, searchable, finalized, status counts, successful/cached searches, AI extraction/review counts, pending search/AI retries, terminal errors, run fingerprint, and `complete`. Serialize to a same-directory temporary file, flush, then rename to `coverage.json` after each finalized row and before a clean stop.

Fingerprint the input workbook path plus size/mtime, expected year, exhaustive mode, both endpoint filters, query schema version, scorer thresholds, model/API/model name, and prompt schema version using SHA-256.

- [ ] **Step 4: Run coverage and resume tests**

Run: `cargo test coverage exhaustive::tests -- --nocapture`

Expected: completeness rejects pending work and resume rejects mismatched fingerprints.

- [ ] **Step 5: Commit coverage accounting**

```bash
git add src/main.rs src/coverage.rs src/checkpoint.rs src/exhaustive.rs
git commit -m "feat: account for exhaustive run coverage"
```

### Task 8: Document and configure exhaustive operation

**Files:**
- Modify: `config.toml`
- Modify: `config.example.toml`
- Modify: `README.md:72-158`

- [ ] **Step 1: Add explicit request controls to both configs**

Under `[discovery]`, add:

```toml
max_attempts = 4
circuit_breaker_threshold = 8
ambiguity_margin = 0.03
```

Retain `search_delay_ms = 750`. For the production public-search run, use a local uncommitted runtime config with `authorized_direct_fetch = false`; do not assert `--i-have-written-authorization` without documented permission.

- [ ] **Step 2: Document exact invocation and data handling**

Document:

```bash
cargo run --release -- run \
  --input "../../out-full-variants-final/2027 New Slate Members - Athletic Matches.xlsx" \
  --config exhaustive-runtime.toml \
  --out-dir out-exhaustive-2027 \
  --all-workbook-rows \
  --both-track-and-xc
```

State that 120,716 rows are finalized, 120,709 are searchable because seven lack names, Stage 0 has 241,418 logical searches, full AI runs for every candidate, and all 13 input columns carry into outputs. State explicitly that the input contains no cellphone field.

- [ ] **Step 3: Check documentation and configuration parsing**

Run: `cargo test config -- --nocapture`

Expected: config defaults and explicit values parse and validate.

- [ ] **Step 4: Commit operational documentation**

```bash
git add config.toml config.example.toml README.md
git commit -m "docs: explain exhaustive athlete run"
```

### Task 9: Verify behavior and launch the resumable full run

**Files:**
- Runtime output: `out-exhaustive-smoke/`
- Runtime output: `out-exhaustive-2027/`
- Runtime config: `exhaustive-runtime.toml` (uncommitted; contains no secret)

- [ ] **Step 1: Run focused and project-wide verification**

Run:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

Expected: every command exits zero with no warnings.

- [ ] **Step 2: Verify the actual local model endpoint**

Run a direct OpenAI-compatible model-list request against `http://127.0.0.1:11000/v1/models` and require a non-empty model list containing the configured model. If unavailable, start the existing project-approved llama.cpp server through the process supervisor, then repeat the request.

Expected: HTTP 200 and configured model present.

- [ ] **Step 3: Run a fresh bounded live smoke**

Run:

```bash
rm -rf out-exhaustive-smoke
./target/release/athletic-rust-pipeline run \
  --input "../../out-full-variants-final/2027 New Slate Members - Athletic Matches.xlsx" \
  --config exhaustive-runtime.toml \
  --out-dir out-exhaustive-smoke \
  --all-workbook-rows \
  --both-track-and-xc \
  --max 5
```

Expected: five finalized records, Track and XC search-cache entries, AI-cache entries for every discovered candidate and candidate-bearing row, all source fields in CSV/JSONL, and `coverage.json` reflecting a bounded incomplete smoke rather than falsely claiming full completion.

- [ ] **Step 4: Verify smoke resume behavior**

Run the identical command again.

Expected: five rows are skipped from the row checkpoint; successful search and AI request counts do not increase.

- [ ] **Step 5: Inspect smoke outputs behaviorally**

Check that:

- `matches.jsonl` has five records;
- `matches.csv` includes all 13 source headers and `AI Logic`;
- `unresolved.csv` includes every non-match status among the five;
- `search-cache.jsonl` contains both `a:tf` and `a:xc`;
- `ai-cache.jsonl` has extraction and review entries when candidates exist;
- `coverage.json` counts five finalized smoke rows and no dropped failures.

- [ ] **Step 6: Launch the full resumable run under the process supervisor**

Start the release binary with:

```text
run --input ../../out-full-variants-final/2027 New Slate Members - Athletic Matches.xlsx --config exhaustive-runtime.toml --out-dir out-exhaustive-2027 --all-workbook-rows --both-track-and-xc
```

Use a stable process name, `athletic-exhaustive-2027`, with restart policy `on-failure` and persistent lifecycle. Do not add `--max`. Do not enable exact-profile retrieval without the existing explicit authorization prerequisites.

Expected startup evidence: `parsed 120716 real rows; selected 120716 prospects`, `120709 searchable`, both filters listed, model enabled, and minimum Stage-0 logical searches `241418`.

- [ ] **Step 7: Verify live progress and checkpoint durability**

After the first finalized row, inspect process logs and parse `coverage.json`, `checkpoint.jsonl`, `search-cache.jsonl`, and `ai-cache.jsonl`.

Expected: counters increase consistently, source contact fields remain only in local outputs, and the process continues under its persistent supervisor name.

- [ ] **Step 8: Produce final workbook after complete coverage**

When `coverage.json.complete` is true, run:

```bash
./target/release/athletic-rust-pipeline writeback \
  --input "../../out-full-variants-final/2027 New Slate Members - Athletic Matches.xlsx" \
  --matches out-exhaustive-2027/matches.jsonl \
  --output "2027 New Slate Members - Exhaustive Athletic Matches.xlsx"
```

Expected: the output workbook preserves original sheets and contains 120,716 `Athletic Matches` data rows with all source fields, candidate links, marks, statuses, and AI Logic.

- [ ] **Step 9: Commit final implementation state**

```bash
git add Cargo.toml Cargo.lock src config.toml config.example.toml README.md
git commit -m "feat: deliver exhaustive athlete matching"
```
