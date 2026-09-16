# Athletic Rust Pipeline

A fully Rust pipeline for:

1. streaming real workbook rows, including a strict first-worksheet mode;
2. either selecting configured sports or processing every source sport against both Track & Field and Cross Country;
3. discovering public Athletic.net candidate links through its scoped search endpoint;
4. optionally retrieving an exact candidate page with `spider-rs/spider` when you are authorized;
5. extracting identity evidence and marks with deterministic Rust plus a local model;
6. conservatively scoring identity matches;
7. checkpointing every processed row;
8. writing CSV/JSONL and an enriched copy of the original XLSX with a new `Athletic Matches` worksheet.

The application is a directly invokable Rust binary. It uses the local
llama.cpp-compatible model servers configured in `[ollama]` and, for strict exhaustive
matching, `[identity_review]`; it does not call a hosted AI API.

## Important boundary

Public-site scraping and automated spiders are not the alpha collection mechanism. The alpha path uses only a developer-authorized unofficial API contract and remains disabled unless both the manifest and command-line authorization checks pass:

- `authorization.enabled = true` plus a non-empty `permission_reference` in `alpha.toml`;
- `--i-have-alpha-authorization` on `collect-authorized`.

The authorization reference must be real, documented permission from Athletic.net developers for the exact routes, fields, states, seasons, and limits in the manifest. The code has no credential guessing, login automation, stealth, proxy rotation, public-site crawling, or broad profile-page traversal. Missing or uncertain API pagination/completeness metadata fails closed.

## Workbook verified for this project

The supplied workbook has two worksheets with identical headers:

- `Export`: 111,939 real data rows;
- `Sheet1`: 8,777 real data rows. Its declared used range is inflated by formatted-empty rows, which the streaming parser ignores.

The source columns are:

```text
Person First
Person Last
Person Email
Address Mailing / Permanent Street Combined
Address Mailing / Permanent City
Address Mailing / Permanent Region
Address Mailing / Permanent Postal
Sports Created Date
Sports Sport
Sports Rating
Origin Source Date
Origin Source
Schools Name
```

Only name, school, city/state, expected graduation year, and sport leave the workbook process. Email and street/postal address are never sent to search or the model.

The legacy sport-filtered mode uses the Track labels `Track and Field: Mens` (146 rows) and `Track and Field: Womens` (95 rows) from `config.example.toml`. `--include-xc` additionally selects configured Cross Country rows.

Strict exhaustive mode ignores source-sport selection, searches both TF and XC, and retains every candidate returned by fully reconciled search pages. It escalates from exact-name queries to contextual and alternative-name queries when identity remains unresolved. A decisive identity can stop escalation only after both sport lanes have completed; no top-three candidate cap applies.

## Build


The matcher expects a local llama.cpp-compatible server. The example configuration targets the existing server on `127.0.0.1:11000` and uses its OpenAI-compatible `/v1/chat/completions` route. No Docker Compose service is required.

```bash
cp config.example.toml config.toml
cargo build --release
ln -sf target/release/athletic-rust-pipeline athletic_matcher
```

The binary also supports an Ollama `/api/chat` endpoint by setting
`ollama.api = "ollama"`.
The configured `Qwen3.6-35B-A3B-UD-Q5_K_XL.gguf` server is text-only. The current `/v1/models` response advertises `completion` capability and no vision projector is installed, so this pipeline does not submit pictures.

`rust-toolchain.toml` pins the compiler and development components used for verification.
`config.exhaustive.toml` configures Q5 extraction at `http://127.0.0.1:11000` and Q4 identity review at `http://127.0.0.1:11001`. Configure server origins, not URLs ending in `/v1`; the client appends the API route.

## Commands

Inspect the whole workbook without network access:

```bash
./athletic_matcher inspect \
  --input "/path/to/input.xlsx"
```

Export every real row to local JSONL:

```bash
./athletic_matcher export-records \
  --input "/path/to/input.xlsx" \
  --output all_records.jsonl
```

### Strict full first-worksheet run

Use a **new output directory** for the initial run:

```bash
cargo run --release -- run \
  --input "/path/to/input.xlsx" \
  --config config.exhaustive.toml \
  --out-dir out-first-worksheet-2027 \
  --all-workbook-rows \
  --first-worksheet-only
```

For the supplied workbook this selects all **111,939 `Export` rows**, not `Sheet1` and not just Track-labelled rows. All 13 original fields are retained in result JSONL and CSV. The seven rows with neither name are terminal `INPUT_ERROR` outcomes. Every named row is searched in both TF and XC irrespective of its original sport; source sport is provenance, not negative identity evidence.

Both models must be enabled. Extraction and identity review fail closed on unavailable, malformed, incomplete-schema, or invalid-index model responses. Search must reconcile every advertised result with valid athlete rows and complete pagination. `SEARCH_ERROR` and `AI_ERROR` are retryable, never converted to `NO_MATCH`; a retryable row is durably checkpointed and stops the command with a nonzero exit.

Resume by repeating the **same command and output directory**. Input bytes, configuration bytes, worksheet scope, and analysis schema bind the run manifest; changing them requires a fresh output directory. Do not manually edit the checkpoint. Successful searches and model results have persistent caches. An unchanged completed-run resume performs no search or model requests. An exclusive writer lock prevents concurrent writers, and Ctrl-C drains an in-flight checkpoint commit before exporting current progress.

`coverage.json` describes the whole selected population even when `--max` restricts a diagnostic run. Its completed, pending, and retryable counts are disjoint. A successful complete run requires `complete: true`, all 111,939 rows completed, and zero pending/retryable rows. Terminal missing-name input errors do not prevent completeness. `--max 0` scans and binds a fresh run without making external requests; it does **not** demonstrate completed matching.

Search concurrency is bounded to two requests with a shared request-spacing gate (750 ms in the supplied configuration). Search bodies are bounded to 16 MiB and model responses to 64 KiB. The row pipeline is sequential, so fixture timings are not a full-workbook throughput guarantee.

Observed live blocker: the full first-worksheet run encountered an Athletic.net athlete link with an invalid ID. It stopped with `SEARCH_ERROR`; coverage was 0 completed, 111,938 pending, and 1 retryable. This is not a completed data delivery. The source response must be corrected or a separately authorized, completeness-preserving source must be supplied; dropping malformed rows is not an acceptable workaround.

### Verification scenarios

The scenario driver invokes the actual binary, retains raw invocation evidence in a fresh temporary directory, and fails on violated assertions:

```bash
cargo build
python3 tools/exhaustive_cli_scenarios.py --binary target/debug/athletic-rust-pipeline
python3 tools/exhaustive_cli_scenarios.py --binary target/debug/athletic-rust-pipeline --actual-models
```

The first command exercises positive dual-sport matching, original-field preservation, unavailable/malformed models, unavailable/incomplete search, writer locking, cancellation, and zero-request completed/cache-only resumes. The second uses the real local Q5/Q4 servers through recording proxies and synthetic search evidence; it is not a live Athletic.net matching test. Synthetic email/street/postal sentinels must never reach either model.

### Legacy sport-filtered run

Dry-run discovery and local validation on five prospects, without direct page fetching:

```bash
./athletic_matcher run \
  --input "/path/to/input.xlsx" \
  --config config.toml \
  --out-dir out \
  --max 5
```

To extract marks without automated page retrieval, manually save a candidate athlete page into `saved_pages/<athlete-id>.html` and configure `retrieval.saved_pages_dir`. For example, `/athlete/12345678/` maps to `saved_pages/12345678.html`. Start a fresh output directory after changing saved evidence; completed checkpoints are not reprocessed automatically.

Authorized exact-page retrieval with Spider requires both `retrieval.authorized_direct_fetch = true` and the CLI acknowledgment. Set `SPIDER_MAX_SIZE_BYTES` to a byte limit from 1 MiB through 4 MiB; missing or invalid limits are rejected. Truncated, oversized, redirected-to-another-identity, blocked, or non-success responses fail closed.

```bash
SPIDER_MAX_SIZE_BYTES=4194304 ./athletic_matcher run \
  --input "/path/to/input.xlsx" \
  --config config.toml \
  --out-dir out \
  --i-have-written-authorization
```

Add `--include-xc` to include Cross Country rows configured alongside Track & Field.

Collect the developer-authorized alpha source through the typed API client:

```bash
./athletic_matcher collect-authorized \
  --alpha-config alpha.toml \
  --out-dir out-authorized-2027 \
  --max-units 1 \
  --i-have-alpha-authorization
```

Start from `alpha.example.toml`; keep `alpha.toml` local and fill only the exact API contract confirmed by the developers. The command requires all 50 states, validates response completeness, checkpoints each matrix unit, and refuses incomplete/capped pages.

Match an existing workbook against a completed local alpha source. This command performs no Athletic.net network access:

```bash
./athletic_matcher match-authorized \
  --input "/path/to/input.xlsx" \
  --alpha-source out-authorized-2027 \
  --config config.toml \
  --out-dir out-authorized-matches
```

Alpha outputs are separate from match decisions: `athletes.csv`, `athletes.jsonl`, `results.jsonl`, `cohort-exceptions.jsonl`, `unresolved.csv`, `unresolved.jsonl`, `checkpoint.jsonl`, and `coverage.json`. Sensitive fields and unsafe evidence are rejected before serialization.

Resume uses append-only `checkpoint.jsonl`, keyed by `sheet:excel_row`. Exhaustive runs use the latest durable outcome, skip terminal rows, and retry errors. An incomplete final JSONL suffix is repaired; corruption in a committed record is rejected. Start a fresh directory when changing input, configuration, or evidence rather than deleting checkpoint lines.

Write results back as a new worksheet while preserving the original worksheets:

```bash
./athletic_matcher writeback \
  --input "/path/to/input.xlsx" \
  --matches out/matches.jsonl \
  --output "2027 New Slate Members - Athletic Matches.xlsx"
```

## Outputs

`run` creates:

- `matches.jsonl`: complete audit records, candidates, evidence, model decision, and normalized marks;
- `matches.csv`: review-friendly flat output with the selected profile and PR columns;
- `checkpoint.jsonl`: resumable processing state;
- `unresolved.csv`: every outcome other than `MATCH`, including `CLOSE_MATCH` and input/runtime errors;
- exhaustive mode also writes `run-manifest.json`, `coverage.json`, `search-cache.jsonl`, and `ai-cache.jsonl`.
The flat outputs also include `Hint Count` (all retained Athletic.net candidate profiles, including weak candidates) and `AI Logic` (the model decision, reason, and Rust score inputs used for the final status). The workbook writes both values as cells in `Athletic Matches`.

`writeback` creates a copy of the source workbook and adds an `Athletic Matches` worksheet keyed by source sheet and Excel row. It uses inline strings and `HYPERLINK()` formulas, so it does not rewrite or reorder the source worksheets.

## Match policy

The model is not allowed to override hard identity evidence. Rust computes name, school, location, class-year, and sport scores. An exact name with no corroborating school/location/year evidence is capped at `REVIEW`. Model output can lower confidence or explain ambiguity; it cannot promote a candidate that misses the deterministic floor.

In exhaustive mode, TF/XC participation is derived from athletic evidence, not the source workbook's sport. Ambiguous different athlete identities cannot produce an automatic match. `NO_MATCH`, `REVIEW`, and error rows carry no selected-profile or sport attribution.

Default buckets:

| Score | Status |
|---:|---|
| `>= 0.93` | `MATCH` |
| `0.86–0.9299` | `CLOSE_MATCH` |
| `0.75–0.8599` | `REVIEW` |
| `< 0.75` | `NO_MATCH` |

## Mark handling

The local model extracts candidate mark records, but Rust canonicalizes events and validates/ranks marks. It understands timed marks such as `12.41` and `4:58.22`, imperial field marks such as `18-4.25`, and metric field marks such as `5.62m`. The raw evidence is retained for audit.

The flat CSV includes common events (`100m`, `200m`, `400m`, `800m`, `1600m`, `3200m`, hurdles, jumps, and throws) plus all marks as JSON.

## Design notes

See `SCOPE.md` for the component boundaries, failure modes, privacy controls, and the full end-to-end flow.
