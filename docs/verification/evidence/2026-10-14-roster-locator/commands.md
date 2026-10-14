# Actual Root commands

These commands were executed by Root. This worker did not run builds, tests, checks, formatting, source lint, browser, production, census or publication operations.

Numeric outer exits are Root attestations of observed exec/write_stdin results. They are not raw tool exports and are not inferred from a PASS string. Raw logs are copied unchanged and independently hashed.

Working directory: `/workspace/shared/athletic-rebuild-20261013`

Setup: `source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh`

Shell convention: `bash; set -o pipefail; command 2>&1 | tee named_log`

## baseline-probe

Source: `baseline-old-production-test-probe`; outer exit: `0`; evidence: `PASS`; executor session: `25657`.

```sh
timeout --signal=INT --kill-after=10s 1200s env -u CI CARGO_INCREMENTAL=0 POSITIVE_ROSTER_BASELINE_PATH=/workspace/shared/athletic-recovery-20261013-2203/positive-roster-locator-proposal/baseline-probe/legacy-bytes.json tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-store -p census-crawl -p census-service --lib -E 'test(positive_roster_legacy_bytes_probe)'
```

[Raw log](raw-logs/baseline-probe.log); SHA-256 `2fd6c51f892942d3a500cae3ffab06ef4a770d7dbbf1b025220e07ca3fa65b26`; 1006 bytes.

Observed terminal summary: 1 test run: 1 passed, 3271 skipped. Nextest ID: `684a5b5e-e3d1-4b36-bb16-7282385a4467`.

## baseline-probe-v2

Source: `baseline-old-production-test-probe`; outer exit: `0`; evidence: `PASS`; executor session: `13580`.

```sh
timeout --signal=INT --kill-after=10s 1200s env -u CI CARGO_INCREMENTAL=0 POSITIVE_ROSTER_BASELINE_PATH=/workspace/shared/athletic-recovery-20261013-2203/positive-roster-locator-proposal/baseline-probe-v2/legacy-bytes.json tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-store -p census-crawl -p census-service --lib -E 'test(positive_roster_legacy_bytes_probe)'
```

[Raw log](raw-logs/baseline-probe-v2.log); SHA-256 `7ae8d41ded6e240f85cf35117cfdcd88ba9d40885e8fea114355e1b3a61b4c89`; 1006 bytes.

Observed terminal summary: 1 test run: 1 passed, 3271 skipped. Nextest ID: `445729b4-0a11-472d-896d-8054ba1bb501`.

## format-first

Source: `candidate-first`; outer exit: `0`; evidence: `PASS`; executor session: `95995`.

```sh
env -u CI tools/moon-local run pipeline:fmt-write
```

[Raw log](raw-logs/format-first.log); SHA-256 `d22409b8d2c305f31f9e319f8b306e07047ea86006a3040481cc838b068cbf9a`; 253 bytes.

## tests-first

Source: `candidate-first`; outer exit: `0`; evidence: `PASS`; executor session: `92342`.

```sh
timeout --signal=INT --kill-after=10s 1200s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-store -p census-crawl -p census-service --lib -E 'test(milesplit::parse::roster::) | test(milesplit::tests::) | test(census::sweep::roster::)'
```

[Raw log](raw-logs/tests-first.log); SHA-256 `c420f90b1e83d7f7d6e9b2f301fb43985d1b19280dec884c6b703b753c027c36`; 12091 bytes.

Observed terminal summary: 73 tests run: 73 passed, 3205 skipped. Nextest ID: `24be34bc-9f1d-4487-a797-7376c0c8e59a`.

## format-second

Source: `candidate-final`; outer exit: `0`; evidence: `PASS`; executor session: `62188`.

```sh
env -u CI tools/moon-local run pipeline:fmt-write
```

[Raw log](raw-logs/format-second.log); SHA-256 `93e14935491e4a40d240c7ecec8cab3df3fee7d36d590eaed17dfe2255b9d015`; 253 bytes.

## tests-second

Source: `candidate-final`; outer exit: `0`; evidence: `PASS`; executor session: `65304`.

```sh
timeout --signal=INT --kill-after=10s 1200s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-store -p census-crawl -p census-service --lib -E 'test(milesplit::parse::roster::) | test(milesplit::tests::) | test(census::sweep::roster::)'
```

[Raw log](raw-logs/tests-second.log); SHA-256 `7bee9e00ef1615ce6b93ecee625ea3e7d29e0d01dc264fcb13a7ef613e553094`; 11707 bytes.

Observed terminal summary: 75 tests run: 75 passed, 3205 skipped. Nextest ID: `28275d6e-95c9-49a2-9a9d-835204327fd8`.

## integration-first

Source: `candidate-final`; outer exit: `0`; evidence: `PASS`; executor session: `82061`.

```sh
timeout --signal=INT --kill-after=10s 1200s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --test milesplit_roster_observations --test recovery -E 'binary(milesplit_roster_observations) | test(jurisdiction)'
```

[Raw log](raw-logs/integration-first.log); SHA-256 `b4dd836468fac67198f105b6e8d27d14a357e9cfb0b4a685af96f39aeb71fc96`; 2090 bytes.

Observed terminal summary: 5 tests run: 5 passed, 8 skipped. Nextest ID: `4c84f81f-55ec-4836-9dfe-289eebad05ea`.

## boundary-parity-first

Source: `candidate-final`; outer exit: `0`; evidence: `PASS`; executor session: `97756`.

```sh
timeout --signal=INT --kill-after=10s 1200s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-crawl -p census-service --test boundary_points --test parity_pipeline
```

[Raw log](raw-logs/boundary-parity-first.log); SHA-256 `b356285939083db8ced1c48d725941fac81dcb179bf2554f2bc7d98d97e81d8e`; 1177 bytes.

Observed terminal summary: 3 tests run: 3 passed, 0 skipped. Nextest ID: `2d3a71dd-ca46-4697-9aba-c86da82afa52`.

## lint-first

Source: `candidate-final`; outer exit: `0`; evidence: `PASS`; executor session: `54352`.

```sh
timeout --signal=INT --kill-after=10s 900s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:lint-src
```

[Raw log](raw-logs/lint-first.log); SHA-256 `4134f7040593213b99200c723419596e0fabdff53651a4f134c41bd1dee84dad`; 862 bytes.

## check-first

Source: `candidate-final`; outer exit: `0`; evidence: `PASS`; executor session: `54352`.

```sh
timeout --signal=INT --kill-after=10s 900s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:check -- --jobs 1
```

[Raw log](raw-logs/check-first.log); SHA-256 `196993196edf660a1db80aa43bd71c4f9766077eb38f5a25d730a6ae18aac76d`; 1140 bytes.

## fmt-check-first

Source: `candidate-final`; outer exit: `0`; evidence: `PASS`; executor session: `54352`.

```sh
env -u CI tools/moon-local run pipeline:fmt
```

[Raw log](raw-logs/fmt-check-first.log); SHA-256 `6b9bccb9d9f667d02f658c6e0fa7bad8bbf220c739e29786cb65fd08b2c6d956`; 241 bytes.
