# Native athlete evidence pipeline

Rust workbook ingestion, Athletic.net candidate discovery and evidence parsing, with native Restate orchestration and local-only model review. The executable exposes `worker`, `deploy`, `start`, `status`, `export`, and `verify`. Earlier CLI commands are no longer the production interface.

## Identity and eligibility

The source workbook is the golden input for membership and source identity context. Every source row remains in output accounting, including duplicate rows and rows missing identity fields. Workbook membership establishes eligibility: there is no junior, grade, or graduation-year requirement. Observed graduation information is descriptive, not a selection or ranking criterion.

Discovery uses names and available school/city/state context and searches both track-and-field and cross-country. A missing school does not prevent discovery. Acceptance remains conservative: missing corroboration or conflicting evidence produces review, not a guessed match. A mailing address is not proof of school geography or candidate residence. Source sport is context, not proof of Athletic.net participation; unknown or blank sport does not exclude a row.

All original fields are retained. The assigned local reviewer receives the complete source record and structured candidate evidence. Email, street and postal information must not be sent to external search services or remote development models. Such fields are context only unless independently corroborated; email domains, ratings and source metadata do not themselves prove a candidate identity. Source cells and retrieved text are untrusted data, never model instructions.

Rust performs routine parsing, arithmetic, mark comparison, and deterministic matching. Only genuine ambiguity reaches one assigned local model. Different cases are assigned across the existing Q5/5090 and Q4/3090 servers; no two-model consensus is required. A model cannot override deterministic contradictions or manufacture evidence.

## Build and checks

```sh
cargo build --release
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

`fuzz/` contains parser fuzz targets. Passing the ordinary test suite does not establish fuzz coverage, recovery correctness, real-world identity accuracy, or full-population readiness.

## Native services

Use native Restate, not Docker. `tools/restate-native.sh start` launches the already-installed pinned Restate 1.7.10 binary and configuration under `${XDG_DATA_HOME:-$HOME/.local/share}/athletic-rust-pipeline/restate`. It requires an existing installation and persistent data directory; it is not an installer.

`config.native.toml` contains this workstation's live configuration: private artifact storage, Athletic.net origin and admission interval, bounded CPU/row work, and the two existing local model endpoints. Adjust paths and model identifiers for another workstation. Do not commit private inputs, credentials, model prompts, or generated athlete artifacts.

```sh
cargo run --release -- worker --config config.native.toml --bind 127.0.0.1:19181
cargo run --release -- deploy --admin http://127.0.0.1:19070/ --endpoint http://127.0.0.1:19181/
```

Keep worker, Restate, and model services local. SDK journals and artifact storage contain sensitive source context and require private storage. Retain Restate's persistent data directory across restarts. Do not restart or reconfigure user-owned model servers as part of pipeline deployment.

## Submit, inspect and export

Record the original SHA-256 before starting. Keep the original unchanged and choose new output paths.

```sh
cargo run --release -- start \
  --input /absolute/path/to/original.xlsx \
  --sha256 ORIGINAL_SHA256 \
  --per-sheet 50 --concurrency 4 \
  --snapshot pilot-source-v1 --execution pilot-v1

cargo run --release -- status --run RUN_DIGEST
cargo run --release -- export --run RUN_DIGEST --output /absolute/new/results.xlsx
cargo run --release -- verify \
  --input /absolute/path/to/original.xlsx \
  --output /absolute/new/results.xlsx \
  --sha256 ORIGINAL_SHA256
```

`--per-sheet` selects a bounded number from each source sheet; `--all` requests all source rows. Submission is not completion. Inspect coverage and verify outputs before expanding a real pilot. Reuse a source snapshot only when its evidence contract is unchanged; cached observations are snapshot-scoped. Changed source fixtures require a new snapshot.

Export publishes an XLSX, a detailed JSONL sidecar, and a commit receipt. Original source fields and row positions are preserved; annotation columns are appended. Partial exports retain explicit pending rows. Destinations are non-clobbering and bound to one run. The commit receipt is published last: the files are not an atomic multi-file transaction. Treat a missing receipt as an incomplete publication.

The independent `verify` command checks source hashes, original fields, source-sheet order and row accounting in the actual XLSX. **It does not independently prove positive identity decisions or PR provenance.** Do not describe this field verifier as a complete result-accuracy audit.

Restate owns durable calls, cached workflow results, operation retry policies and orchestration. HTTP attempt evidence is retained, but an external response not acknowledged before a crash can be repeated. There is no exactly-once HTTP guarantee.

## Synthetic native exercise

```sh
cargo run --example native_fixture -- \
  --bind 127.0.0.1:18081 \
  --output-dir /absolute/new/synthetic-fixture \
  --scenario match,duplicate,ambiguous,missing-cohort,conflict,empty-search
```

The fixture writes a synthetic workbook and worker configuration, and serves controlled source/model responses and request counters. Run its worker against a dedicated native Restate instance. Fixture model endpoints are synthetic; they do not exercise the real GPUs. Keep generated fixture inputs stable during a run: fixture startup currently regenerates its workbook, changing its digest.

Observed native happy-path exercise: 8 source rows across two sheets, 5 accepted, 1 no-match, 2 review; exported fields verified 120/120 with source SHA-256 unchanged during export. This is synthetic evidence, not the real 100-row pilot or completed full-workbook delivery. Live rollout, independent positive-result verification, recovery/failure campaigns, performance and security acceptance remain separate requirements.
