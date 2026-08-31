# Athletic Rust Pipeline

A fully Rust pipeline for:

1. streaming every real row from both workbook worksheets;
2. selecting Track & Field and optionally Cross Country prospects;
3. discovering public Athletic.net candidate links through its scoped search endpoint;
4. optionally retrieving an exact candidate page with `spider-rs/spider` when you are authorized;
5. extracting identity evidence and marks with deterministic Rust plus a local model;
6. conservatively scoring identity matches;
7. checkpointing every processed row;
8. writing CSV/JSONL and an enriched copy of the original XLSX with a new `Athletic Matches` worksheet.

The application is a directly invokable Rust binary. It uses the local
llama.cpp-compatible model server configured in `[ollama]`; it does not call a
hosted AI API.

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

The workbook's exact Track labels are `Track and Field: Mens` (146 rows) and `Track and Field: Womens` (95 rows); those values are already in `config.example.toml`. To include XC, add `Cross Country: Mens` and `Cross Country: Womens`.
Discovery tries multiple name forms before choosing the final three candidates: original spelling, diacritic/punctuation-normalized spelling, family-name-first ordering, and first-initial plus family name, with school or city/state context where available. Candidates are ranked by independent name, school, and location evidence before the three-profile output cap is applied.

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

Dry-run discovery and local validation on five prospects, without direct page fetching:

```bash
./athletic_matcher run \
  --input "/path/to/input.xlsx" \
  --config config.toml \
  --out-dir out \
  --max 5
```

To extract marks without automated page retrieval, manually save a candidate athlete page into `saved_pages/<athlete-id>.html`. For example, a profile URL containing `/athlete/12345678/` maps to `saved_pages/12345678.html`. A resumed row is not reprocessed, so remove only that row's old checkpoint line or start a fresh output directory after adding saved evidence.

Authorized exact-page retrieval with Spider:

```bash
./athletic_matcher run \
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

Resume is automatic: `out/checkpoint.jsonl` is append-only and keyed by `sheet:excel_row`. Delete or move the checkpoint only if you intentionally want to redo discovery.

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
- `unresolved.csv`: only `REVIEW`, `NO_MATCH`, and errors.
The flat outputs also include `Hint Count` (all retained Athletic.net candidate profiles, including weak candidates) and `AI Logic` (the model decision, reason, and Rust score inputs used for the final status). The workbook writes both values as cells in `Athletic Matches`.

`writeback` creates a copy of the source workbook and adds an `Athletic Matches` worksheet keyed by source sheet and Excel row. It uses inline strings and `HYPERLINK()` formulas, so it does not rewrite or reorder the source worksheets.

## Match policy

The model is not allowed to override hard identity evidence. Rust computes name, school, location, class-year, and sport scores. An exact name with no corroborating school/location/year evidence is capped at `REVIEW`. Model output can lower confidence or explain ambiguity; it cannot promote a candidate that misses the deterministic floor.

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
