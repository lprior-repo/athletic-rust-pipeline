# Authorized Athletic.net Alpha Collection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an authorization-gated API-first collector for complete Class-of-2027 high-school Track & Field records across all 50 states, then feed its normalized source list into the existing matcher and workbook outputs.

**Architecture:** Keep the existing workbook/search pipeline unchanged when no alpha source is supplied. Add small alpha modules for authorization/configuration, API transport, catalog discovery, cohort filtering, normalization, checkpointing, source output, and local matching. The alpha client consumes only the developer-confirmed request/response contract; unknown pagination or completeness behavior fails closed.

**Tech Stack:** Rust 2021, `reqwest` + `serde_json` for the alpha API, existing `tokio` runtime, CSV/JSONL output, existing scoring/marks/model modules, Playwright for a bounded contract/smoke utility.

---

## File map

Create these focused files:

- `src/alpha_model.rs` — typed authorization, catalog, raw API, normalized athlete/result, run-unit, and coverage types.
- `src/alpha_config.rs` — standalone alpha TOML loader and validation; does not change the existing matcher config schema.
- `src/alpha_api.rs` — request serialization, response decoding, rate limiting, bounded retry, and completeness interpretation.
- `src/alpha_catalog.rs` — 50-state allow-list, API catalog discovery, event catalog validation, and run-matrix construction.
- `src/alpha_cohort.rs` — Class-of-2027 evidence precedence and exception classification.
- `src/alpha_normalize.rs` — safe-field normalization, URL construction/validation, athlete-ID deduplication, and mark conversion.
- `src/alpha_checkpoint.rs` — append-only alpha unit checkpoints and resume state.
- `src/alpha_output.rs` — source CSV/JSONL, exception CSV/JSONL, coverage JSON, and forbidden-field checks.
- `src/alpha_pipeline.rs` — sequential authorized collection orchestration and source-list loading for matching.
- `src/alpha_match.rs` — conversion from normalized source athletes to existing `Candidate` values and local workbook matching.
- `alpha.example.toml` — complete safe configuration example with the authorization gate disabled.
- `fixtures/alpha/get-rankings-redacted.json` — synthetic/redacted API response fixture containing the confirmed field shape, never real athlete data.
- `fixtures/alpha/get-nav-info-redacted.json` — synthetic/redacted catalog fixture.
- `tools/alpha_contract.mjs` — bounded Playwright contract capture that strips credentials and replaces identifying values before writing fixtures.
- `package.json` — pinned Playwright dev dependency for the contract/smoke utility.
- `package-lock.json` — lockfile for the Playwright utility.

Modify these existing files:

- `src/main.rs` — add `collect-authorized` and `match-authorized` subcommands.
- `src/model.rs` — add an optional stable `athlete_id` to `Candidate` so local matches retain source identity.
- `Cargo.toml` — add the Rust dependencies/features required by the new modules only.
- `.gitignore` — ignore local alpha configs, Playwright storage state, and live capture output.
- `README.md` — document the approved alpha workflow and the existing final match-list paths.
- `SCOPE.md` — add the alpha data flow and completeness/privacy boundary.

Each new Rust module must remain below 300 lines. Do not split or reformat unrelated existing modules.

---

### Task 1: Freeze and sanitize the alpha contract

**Files:**
- Create: `package.json`
- Create: `package-lock.json`
- Create: `tools/alpha_contract.mjs`
- Create: `fixtures/alpha/get-rankings-redacted.json`
- Create: `fixtures/alpha/get-nav-info-redacted.json`
- Modify: `.gitignore`

- [ ] **Step 1: Add the bounded Playwright utility and pin its dependency.**

Create `package.json` with the exact dependency:

```json
{
  "private": true,
  "type": "module",
  "devDependencies": {
    "playwright": "1.55.0"
  }
}
```

Run `npm install` to create the committed `package-lock.json`.

Use a user-supplied page URL and optional local `storageState` file. Listen only for the two confirmed alpha requests, strip all request headers/cookies/tokens, and replace all identifying values before writing JSON. The utility must stop after one rankings response and one navigation response.

```js
import { chromium } from 'playwright';
import { mkdir, writeFile } from 'node:fs/promises';

const [pageUrl, outputDir, storageState] = process.argv.slice(2);
if (!pageUrl || !outputDir) {
  throw new Error('usage: node tools/alpha_contract.mjs PAGE_URL OUTPUT_DIR [STORAGE_STATE]');
}

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext(storageState ? { storageState } : {});
const page = await context.newPage();
const captured = {};

const scrub = (value, key = '') => {
  if (Array.isArray(value)) return value.map(item => scrub(item, key));
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([name, item]) => [name, scrub(item, name)]));
  }
  if (/name|url|state|meet|school/i.test(key) && typeof value === 'string') return 'REDACTED';
  if (/athleteid|idresult|teamid|meetid/i.test(key) && typeof value === 'number') return 90000001;
  return value;
};

page.on('response', async response => {
  const url = response.url();
  if (!/api\/v1\/tfRankings\/(GetRankings|GetNavInfo)/.test(url)) return;
  const key = url.includes('GetRankings') ? 'rankings' : 'nav';
  if (captured[key]) return;
  captured[key] = scrub(await response.json());
});

await page.goto(pageUrl, { waitUntil: 'networkidle' });
if (!captured.rankings || !captured.nav) throw new Error('alpha contract responses not observed');
await mkdir(outputDir, { recursive: true });
await writeFile(`${outputDir}/get-rankings-redacted.json`, JSON.stringify(captured.rankings, null, 2));
await writeFile(`${outputDir}/get-nav-info-redacted.json`, JSON.stringify(captured.nav, null, 2));
await browser.close();
```

- [ ] **Step 2: Run the utility against one authorized page.**

Run:

```bash
npm install
node tools/alpha_contract.mjs 'https://www.athletic.net/TrackAndField/rankings/list/168493/m/100m' /tmp/alpha-contract
```

Expected: two redacted JSON files; neither contains `cookie`, `authorization`, `token`, a real athlete name, or a real school name.

- [ ] **Step 3: Replace fixture contents with synthetic values while preserving schema.**

Keep the observed keys (`groupedRankings`, `AthleteID`, `AthleteName`, `GradeID`, `TeamName`, `State`, `MeetID`, `IDResult`, `EventShort`, `Measure`, `ResultDate`, `SeasonID`, and the confirmed continuation/completeness fields). Replace all values with `90000001`, `Test Runner`, `Test High School`, `TS`, and `2099-01-01`. Add one record in each fixture that exercises an empty page and a continuation page.

- [ ] **Step 4: Add ignore rules and commit.**

Ignore `alpha.toml`, `alpha.local.toml`, `playwright/.auth/`, `storage-state.json`, and `tmp/alpha-contract/`. Commit:

```bash
git add package.json package-lock.json tools/alpha_contract.mjs fixtures/alpha .gitignore
git commit -m "build: add redacted alpha contract capture"
```

---

### Task 2: Add alpha configuration and domain types

**Files:**
- Create: `src/alpha_model.rs`
- Create: `src/alpha_config.rs`
- Create: `alpha.example.toml`
- Modify: `Cargo.toml`

- [ ] **Step 1: Write configuration validation tests.**

Define a test-only `valid_config()` with `authorization.enabled = false`, `permission_reference = "alpha-test"`, one allowed HTTPS route, all 50 state codes, one season, genders `["m", "f"]`, `max_concurrent_requests = 1`, and `min_delay_ms = 500`.

Test that disabled authorization is rejected by `collect-authorized`, an empty permission reference is rejected when enabled, the allowed state list contains exactly the 50 codes, `max_concurrent_requests` must equal `1` for the sequential first implementation, and a route outside `allowed_routes` is rejected.

```rust
#[test]
fn enabled_alpha_requires_permission_reference() {
    let mut config = valid_config();
    config.authorization.enabled = true;
    config.authorization.permission_reference.clear();
    let error = config.validate().expect_err("missing permission must fail");
    assert!(error.to_string().contains("permission_reference"));
}
```

Run: `cargo test alpha_config::tests::enabled_alpha_requires_permission_reference`
Expected: FAIL because the alpha types and validator do not exist.

- [ ] **Step 2: Define the typed contracts.**

Implement these public types in `alpha_model.rs`:

```rust
pub struct AuthorizationConfig {
    pub enabled: bool,
    pub permission_reference: String,
    pub allowed_routes: Vec<String>,
    pub allowed_sports: Vec<String>,
    pub allowed_states: Vec<String>,
    pub allowed_seasons: Vec<i32>,
    pub allowed_genders: Vec<String>,
    pub allowed_fields: Vec<String>,
    pub allow_profile_enrichment: bool,
    pub max_concurrent_requests: usize,
    pub min_delay_ms: u64,
}

pub struct AlphaApiConfig {
    pub base_url: String,
    pub rankings_path: String,
    pub nav_info_path: String,
    pub timeout_seconds: u64,
    pub max_retries: usize,
    pub pagination: PaginationConfig,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PaginationConfig {
    SingleResponse { complete_pointer: String },
    NextPage {
        has_more_pointer: String,
        next_page_pointer: String,
        request_page_key: String,
    },
}

pub struct AlphaRequest {
    pub state_id: u64,
    pub season_id: i32,
    pub gender: String,
    pub event_short: String,
    pub continuation: Option<serde_json::Value>,
}

pub struct SourceAthlete {
    pub athlete_id: u64,
    pub athlete_name: String,
    pub school: String,
    pub state: String,
    pub graduation_year: Option<i32>,
    pub cohort_evidence: String,
    pub gender: String,
    pub sport: String,
    pub profile_url: String,
    pub results: Vec<SourceResult>,
    pub source_urls: Vec<String>,
}

pub struct SourceResult {
    pub result_id: Option<u64>,
    pub event: String,
    pub mark: String,
    pub season: String,
    pub date: String,
    pub meet_name: String,
    pub wind: Option<String>,
    pub result_url: Option<String>,
    pub source_url: String,
}
```

Add `RawRankingRecord` with serde renames for the confirmed PascalCase API fields, plus `RawRankingsResponse` and `RawNavInfoResponse`. Unknown fields are ignored; required identity/result fields are not defaulted.
Create `alpha.example.toml` with this safe baseline:

```toml
[authorization]
enabled = false
permission_reference = "disabled-example-no-permission"
allowed_routes = ["/api/v1/tfRankings/GetRankings", "/api/v1/tfRankings/GetNavInfo"]
allowed_sports = ["Track and Field"]
allowed_states = ["AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV", "WI", "WY"]
allowed_seasons = [2026]
allowed_genders = ["m", "f"]
allowed_fields = ["AthleteID", "AthleteName", "GradeID", "TeamName", "State", "MeetID", "IDResult", "EventShort", "Measure", "ResultDate", "SeasonID"]
allow_profile_enrichment = false
max_concurrent_requests = 1
min_delay_ms = 750

[api]
base_url = "https://www.athletic.net"
rankings_path = "/api/v1/tfRankings/GetRankings"
nav_info_path = "/api/v1/tfRankings/GetNavInfo"
timeout_seconds = 30
max_retries = 2

[api.pagination]
mode = "single_response"
complete_pointer = "/settings/complete"
```

The example remains disabled so it cannot send this request. Enabling it requires the developer-confirmed completeness field and continuation mode to be written into the live alpha configuration first.

- [ ] **Step 3: Implement TOML loading and validation.**

`AlphaConfig::load(path: &Path) -> anyhow::Result<Self>` reads TOML, validates the allow-list, rejects non-HTTPS API URLs, rejects routes not in `allowed_routes`, and rejects `allow_profile_enrichment = true` unless the manifest explicitly includes profile routes. The example file has `authorization.enabled = false`.

- [ ] **Step 4: Run focused tests and commit.**

Add `mockito = "1"` under `[dev-dependencies]` in `Cargo.toml` so pipeline transport tests can assert request counts and bodies without contacting Athletic.net.

Run: `cargo test alpha_config::tests`
Expected: PASS.

```bash
git add Cargo.toml src/alpha_model.rs src/alpha_config.rs alpha.example.toml
git commit -m "feat: add alpha authorization contracts"
```

---

### Task 3: Implement the typed alpha API client

**Files:**
- Create: `src/alpha_api.rs`
- Test: `fixtures/alpha/get-rankings-redacted.json`

- [ ] **Step 1: Write request/response tests.**

Test that serialization produces the confirmed keys exactly:

```rust
assert_eq!(json["reportType"], "div");
assert_eq!(json["mode"], "list");
assert_eq!(json["divListId"], 90000001);
assert_eq!(json["qParams"], serde_json::json!({}));
assert_eq!(json["version"], 2);
```

Test that one synthetic ranking row maps `AthleteID`, `AthleteName`, `GradeID`, `TeamName`, `State`, `MeetID`, `IDResult`, `EventShort`, `Measure`, `ResultDate`, and `SeasonID` without reading any unknown field.

Run: `cargo test alpha_api::tests::serializes_confirmed_rankings_request`
Expected: FAIL.

- [ ] **Step 2: Implement request construction.**

Use a dedicated request struct:

```rust
#[derive(serde::Serialize)]
struct RankingsRequest<'a> {
    #[serde(rename = "reportType")]
    report_type: &'static str,
    mode: &'static str,
    #[serde(rename = "divListId")]
    div_list_id: u64,
    indoor: Option<bool>,
    #[serde(rename = "eventShort")]
    event_short: &'a str,
    gender: &'a str,
    #[serde(rename = "qParams")]
    q_params: serde_json::Map<String, serde_json::Value>,
    #[serde(rename = "qualifyingListKey")]
    qualifying_list_key: &'static str,
    version: u8,
    debug: &'static str,
}
```

Populate `qParams` only from `PaginationConfig`; never add guessed filters. Enforce the authorization route and field allow-lists before sending.

- [ ] **Step 3: Implement bounded transport and completeness.**

`AlphaApiClient::rankings(&AlphaRequest) -> Result<RankingPage, AlphaApiError>` uses the configured timeout, exactly one request at a time, `min_delay_ms`, and at most `max_retries` for 5xx/timeouts. The validator rejects a concurrency value other than `1` in the first implementation. It stops immediately on 401/403 and honors `Retry-After` for 429. It returns `complete = false` for missing continuation metadata, an explicit top-N cap, or an invalid response shape.

`AlphaApiClient::nav_info(season_id, indoor) -> Result<RawNavInfoResponse, AlphaApiError>` uses the same route allow-list and transport policy.

- [ ] **Step 4: Add fixture tests and commit.**

Run: `cargo test alpha_api::tests`
Expected: PASS, including a test that a deliberately removed continuation field returns `complete = false`.

```bash
git add src/alpha_api.rs fixtures/alpha
git commit -m "feat: add bounded alpha rankings client"
```

---

### Task 4: Build and validate the 50-state/event run matrix

**Files:**
- Create: `src/alpha_catalog.rs`
- Modify: `src/alpha_model.rs`
- Test: `fixtures/alpha/get-nav-info-redacted.json`

- [ ] **Step 1: Write catalog tests.**

Use the exact 50-code allow-list:

```rust
const STATE_CODES: [&str; 50] = [
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA",
    "HI", "ID", "IL", "IN", "IA", "KS", "KY", "LA", "ME", "MD",
    "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ",
    "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC",
    "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV", "WI", "WY",
];
```

Assert exactly 50 canonical entries, no DC/Overseas entry, no duplicate `state_id`, and no California/Texas subregion entry. Assert that event entries preserve the API’s `eventShort` and timing/distance direction.

Run: `cargo test alpha_catalog::tests::accepts_exactly_fifty_states`
Expected: FAIL.

- [ ] **Step 2: Implement nav-info parsing and catalog validation.**

Parse only U.S. state nodes and authorized event nodes from the redacted nav fixture. Match state names/codes against `STATE_CODES`, reject a missing state or duplicate ID, and reject extra region/division nodes. Construct `RunUnit { state, season_id, gender, event_short, page }` from the manifest’s seasons and genders.

- [ ] **Step 3: Implement matrix counts and bounded pilot selection.**

`RunMatrix::all()` returns every authorized state × season × gender × event unit. `RunMatrix::take(max_units)` returns the first deterministic units sorted by state code, season ID, gender, and event short. The CLI uses `--max-units 1` for the required pilot.

- [ ] **Step 4: Run tests and commit.**

Run: `cargo test alpha_catalog::tests`
Expected: PASS.

```bash
git add src/alpha_catalog.rs src/alpha_model.rs fixtures/alpha/get-nav-info-redacted.json
git commit -m "feat: add fifty-state alpha catalog"
```

---

### Task 5: Implement cohort filtering, URL safety, and deduplication

**Files:**
- Create: `src/alpha_cohort.rs`
- Create: `src/alpha_normalize.rs`
- Modify: `src/model.rs`

- [ ] **Step 1: Write cohort and dedup tests.**

Cover these cases:

```rust
assert!(matches!(
    classify_cohort(2027, Some(2027), Some("2025-26"), Some(11)),
    CohortDecision::Include { .. }
));
assert!(matches!(
    classify_cohort(2027, None, Some("2025-26"), Some(11)),
    CohortDecision::Include { .. }
));
assert!(matches!(
    classify_cohort(2027, None, Some("2026-27"), Some(12)),
    CohortDecision::Include { .. }
));
assert!(matches!(
    classify_cohort(2027, Some(2026), Some("2026-27"), Some(12)),
    CohortDecision::Exception { .. }
));
assert!(matches!(
    classify_cohort(2027, None, None, None),
    CohortDecision::Exception { .. }
));
```

Add a fixture with two rows for athlete ID 90000001 from different events and assert one athlete with two results and two distinct source URLs. Assert that a missing athlete ID is exception-only.

Run: `cargo test alpha_cohort::tests alpha_normalize::tests`
Expected: FAIL.

- [ ] **Step 2: Implement evidence precedence.**

`classify_cohort(target_year, explicit_year, season_label, grade)` returns `Include`, `Exclude`, or `Exception`. Explicit year wins only when it equals 2027; conflicting explicit year and grade is an exception. The two fallback mappings are exact string/grade pairs from the design. Age, ranking position, name, and school are never consulted.

- [ ] **Step 3: Implement safe normalization.**

Normalize state codes, event aliases, marks via `marks::normalize_mark`, dates, school whitespace, and HTTPS URLs. Construct a profile URL only from an API-confirmed athlete ID and an allowed profile route. Keep `result_id` separately when the API does not provide an approved result URL.

Add `athlete_id: Option<u64>` to `Candidate` with `#[serde(default)]`, populate it only for alpha-derived candidates, and leave existing discovery behavior unchanged.

- [ ] **Step 4: Implement athlete-ID merge.**

Use `BTreeMap<u64, SourceAthlete>`; append unseen results and URLs, retain the first non-empty identity field, and record conflicts in the athlete’s exception notes. Never use normalized name as a primary key.

- [ ] **Step 5: Run tests and commit.**

Run: `cargo test alpha_cohort::tests alpha_normalize::tests scoring::tests`
Expected: PASS, including the existing exact-name/no-corroboration rule.

```bash
git add src/alpha_cohort.rs src/alpha_normalize.rs src/model.rs
git commit -m "feat: normalize class-of-2027 source records"
```

---

### Task 6: Add resumable checkpoints, outputs, and privacy guards

**Files:**
- Create: `src/alpha_checkpoint.rs`
- Create: `src/alpha_output.rs`


- [ ] **Step 1: Write checkpoint/output tests.**

Use a temporary directory and the public APIs:

```rust
#[test]
fn incomplete_checkpoint_is_retryable() {
    let directory = tempfile::tempdir().unwrap();
    let key = AlphaUnitKey {
        state_code: "AZ".into(),
        season_id: 2026,
        gender: "m".into(),
        event_short: "100m".into(),
        continuation: "0".into(),
    };
    append(directory.path(), &AlphaCheckpoint {
        key: key.clone(),
        response_count: 100,
        complete: false,
        status: "incomplete".into(),
    }).unwrap();
    let latest = load_latest(directory.path()).unwrap();
    assert!(!latest.get(&key).unwrap().complete);
}
```

Add tests for completed-unit resume, approved CSV headers, and serialized output containing none of `email`, `phone`, `street`, `postal`, `cookie`, `authorization`, or `token`.

Run: `cargo test alpha_checkpoint::tests alpha_output::tests`
Expected: FAIL.

- [ ] **Step 2: Implement append-only checkpoint state.**

Use a stable key that supports either the numeric page or opaque continuation token confirmed by the alpha contract:

```rust
pub struct AlphaUnitKey {
    pub state_code: String,
    pub season_id: i32,
    pub gender: String,
    pub event_short: String,
    pub continuation: String,
}

pub struct AlphaCheckpoint {
    pub key: AlphaUnitKey,
    pub response_count: usize,
    pub complete: bool,
    pub status: String,
}
```

Write one JSON object per line and load the latest state by key. A `complete = false` entry is retryable; a complete entry is skipped.

- [ ] **Step 3: Implement source outputs.**

Write `athletes.csv`, `athletes.jsonl`, `results.jsonl`, `cohort-exceptions.jsonl`, `unresolved.csv`, `checkpoint.jsonl`, and `coverage.json` atomically where practical. CSV columns are exactly the approved source fields plus `cohort_evidence`, `profile_url`, `source_urls_json`, and `result_count`. `coverage.json` records planned, complete, empty, incomplete, and exception unit counts.

Reject output serialization if a forbidden key or value appears. Do not log request headers, cookies, raw tokens, or full API response bodies.

- [ ] **Step 4: Run tests and commit.**

Run: `cargo test alpha_checkpoint::tests alpha_output::tests`
Expected: PASS.

```bash
git add src/alpha_checkpoint.rs src/alpha_output.rs
git commit -m "feat: add resumable alpha outputs"
```

---

### Task 7: Wire the authorized collection command

**Files:**
- Create: `src/alpha_pipeline.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write CLI/pipeline tests.**


Use a disabled `alpha.example.toml` and assert the guard runs before client construction:

```rust
#[tokio::test]
async fn disabled_manifest_fails_before_network() {
    let error = collect_authorized(
        Path::new("alpha.example.toml"),
        Path::new("target/alpha-test-output"),
        Some(1),
        false,
    )
    .await
    .expect_err("disabled authorization must fail");
    assert!(error.to_string().contains("authorization"));
}
```

Use a `mockito::Server` for the remaining tests: one successful response must create one checkpoint, a missing completeness field must return an error, and a second invocation must issue zero mock requests for the completed key.

Run: `cargo test alpha_pipeline::tests`
Expected: FAIL.

- [ ] **Step 2: Implement the pipeline.**

`collect_authorized(config_path, out_dir, max_units, authorization_ack)` must:

1. load and validate `AlphaConfig`;
2. require `authorization.enabled` and `authorization_ack`;
3. build the 50-state/event matrix;
4. load checkpoints;
5. call `AlphaApiClient` sequentially for each unit;
6. reject incomplete/capped pages;
7. apply cohort classification;
8. normalize and merge by athlete ID;
9. append checkpoint state after each unit;
10. write outputs and coverage only after processing.

Add CLI definitions:

```rust
CollectAuthorized {
    #[arg(long)] alpha_config: PathBuf,
    #[arg(long, default_value = "out-authorized-2027")] out_dir: PathBuf,
    #[arg(long)] max_units: Option<usize>,
    #[arg(long)] i_have_alpha_authorization: bool,
},
```
Register the new modules at the top of `main.rs`:

```rust
mod alpha_api;
mod alpha_catalog;
mod alpha_checkpoint;
mod alpha_cohort;
mod alpha_config;
mod alpha_match;
mod alpha_model;
mod alpha_normalize;
mod alpha_output;
mod alpha_pipeline;
```

The command prints counts and paths only; it never prints request bodies or credentials.

- [ ] **Step 3: Run the offline command checks.**

Run:

```bash
cargo run -- collect-authorized --alpha-config alpha.example.toml --max-units 1
```

Expected: nonzero exit with an authorization-disabled error and no output directory containing athlete records.

Run:

```bash
cargo test alpha_pipeline::tests
```

Expected: PASS.

- [ ] **Step 4: Commit.**

```bash
git add src/alpha_pipeline.rs src/main.rs
git commit -m "feat: add authorized alpha collection command"
```

---

### Task 8: Feed the normalized source list into matching

**Files:**
- Create: `src/alpha_match.rs`
- Modify: `src/main.rs`
- Modify: `src/alpha_pipeline.rs`

- [ ] **Step 1: Write local matching tests.**

Test that a source athlete with matching name/school/state/year becomes a candidate with the source profile URL and marks, that a same-name different-state athlete remains `REVIEW`/`NO_MATCH` under the existing thresholds, and that the model receives only `Prospect` plus safe candidate fields.

Run: `cargo test alpha_match::tests`
Expected: FAIL.

- [ ] **Step 2: Convert source records to existing candidates.**

Implement:

```rust
pub fn candidates_for_prospect(
    prospect: &Prospect,
    source: &[SourceAthlete],
    config: &MatchingConfig,
) -> Vec<Candidate> {
    source.iter()
        .filter(|athlete| name_index_match(prospect, athlete))
        .map(|athlete| candidate_from_source(athlete, prospect, config))
        .collect()
}
```

Use a normalized full-name index first, then the existing school/location scoring to rank candidates. Do not scan every nationwide athlete for every prospect. `candidate_from_source` maps source results into existing `Mark` values, sets `page_retrieved = false`, copies only approved evidence URLs, and sets `athlete_id`.

- [ ] **Step 3: Add `match-authorized`.**

Add:

```rust
MatchAuthorized {
    #[arg(long)] input: PathBuf,
    #[arg(long)] alpha_source: PathBuf,
    #[arg(long)] config: PathBuf,
    #[arg(long, default_value = "out-authorized-matches")] out_dir: PathBuf,
    #[arg(long)] max: Option<usize>,
}
```

Reuse `xlsx::scan`, `extract::OllamaClient::validate_identity`, `scoring::score_candidate`, `scoring::finalize_match`, and `output::write_all`. This command performs no network access to Athletic.net. It writes the same `matches.csv`, `matches.jsonl`, and `unresolved.csv` formats consumed by the existing `writeback` command.

- [ ] **Step 4: Run matching tests and commit.**

Run: `cargo test alpha_match::tests scoring::tests output::tests`
Expected: PASS.

```bash
git add src/alpha_match.rs src/alpha_pipeline.rs src/main.rs
git commit -m "feat: match workbook prospects against alpha source"
```

---

### Task 9: Document operation and run the bounded live pilot

**Files:**
- Modify: `README.md`
- Modify: `SCOPE.md`
- Modify: `.gitignore`
- [ ] **Step 1: Document exact commands and boundaries.**

Add commands for:

```bash
cargo run -- collect-authorized \
  --alpha-config alpha.toml \
  --out-dir out-authorized-2027 \
  --max-units 1 \
  --i-have-alpha-authorization

cargo run -- match-authorized \
  --input '/path/to/input.xlsx' \
  --alpha-source out-authorized-2027/athletes.jsonl \
  --config config.toml \
  --out-dir out-authorized-matches

cargo run -- writeback \
  --input '/path/to/input.xlsx' \
  --matches out-authorized-matches/matches.jsonl \
  --output '2027 New Slate Members - Authorized Matches.xlsx'
```

Document that the definitive current final list remains `out-full-variants-final/matches.csv`, with the JSONL and workbook paths listed above.

- [ ] **Step 2: Run static and offline verification.**

Run:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo run -- collect-authorized --alpha-config alpha.example.toml --max-units 1
```

Expected: formatting, tests, and clippy pass; the example command fails closed before network access because authorization is disabled.

- [ ] **Step 3: Run the authorized Playwright/API pilot.**

Use the developer-provided alpha configuration and a one-unit run:

```bash
node tools/alpha_contract.mjs \
  'https://www.athletic.net/TrackAndField/rankings/list/168493/m/100m' \
  /tmp/alpha-contract \
  /path/to/local/storage-state.json

cargo run -- collect-authorized \
  --alpha-config alpha.toml \
  --out-dir out-authorized-2027-pilot \
  --max-units 1 \
  --i-have-alpha-authorization
```

Expected: one authorized state/gender/event unit, nonzero athlete/result records or an explicitly authorized empty result, `coverage.json` marked complete, and no credential or forbidden-field output.

- [ ] **Step 4: Verify resume and incomplete handling.**

Run the same pilot twice. Expected: the second run skips the completed unit. Replace the fixture/response with a missing continuation field and rerun. Expected: nonzero exit, `complete = false`, and a resumable checkpoint rather than a nationwide-complete claim.

- [ ] **Step 5: Commit documentation.**

```bash
git add README.md SCOPE.md .gitignore
git commit -m "docs: document authorized alpha workflow"
```

---

## Plan self-review

- **Spec coverage:** authorization gate (Tasks 2 and 7); API-first transport (Task 3); Playwright validation (Tasks 1 and 9); 50-state coverage (Task 4); Class of 2027 precedence (Task 5); URLs and marks (Task 5); deduplication (Task 5); checkpoints (Task 6); outputs and privacy (Task 6); existing matcher integration (Task 8); bounded verification and failure-closed behavior (Tasks 3, 7, and 9).
- **Placeholder scan:** no `TBD`, `TODO`, or unspecified fallback is used. The live alpha configuration must contain the developer-confirmed continuation fields; the client rejects a missing contract rather than guessing.
- **Type consistency:** `SourceAthlete`/`SourceResult` are introduced in `alpha_model.rs`, consumed by `alpha_normalize.rs`, `alpha_pipeline.rs`, and `alpha_match.rs`; `Candidate.athlete_id` is optional so existing discovery records remain compatible.
- **Scope:** Track & Field only in the first implementation; Cross Country requires a separate manifest enablement and event catalog.
