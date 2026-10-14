# Exact Root command ledger

This is a transcription of Root’s executed-call record, not commands executed by this evidence worker. Historical outer HOME was inherited and was not overridden; its literal value was not retained in the provided execution record. Do not invent it. The active toolchain paths and all explicit exports are preserved in execution-environment.sh.

Working directory: /workspace/shared/athletic-rebuild-20261013
Shell: bash

Exact setup used before each command:
```sh
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
```

The sourced file contains:
```sh
# Source this file to activate the repository-pinned recovery tools.
export ATHLETIC_TOOLCHAIN_ROOT=/workspace/shared/athletic-recovery-20261013-2203/toolchain
export CARGO_HOME="$ATHLETIC_TOOLCHAIN_ROOT/cargo"
export RUSTUP_HOME="$ATHLETIC_TOOLCHAIN_ROOT/rustup"
export RUSTUP_TOOLCHAIN=nightly-2026-04-27
export PATH="$ATHLETIC_TOOLCHAIN_ROOT/bin:$CARGO_HOME/bin:$PATH"
export XDG_DATA_HOME="$ATHLETIC_TOOLCHAIN_ROOT/xdg-data"
export XDG_CACHE_HOME="$ATHLETIC_TOOLCHAIN_ROOT/xdg-cache"
```

Root used set -o pipefail and sent combined stdout/stderr through tee to the named absolute log. All build/test/check/lint/format operations enter through tools/moon-local; internal Cargo lines in raw logs are implementation detail, not an alternative entrypoint.

## core-first.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 600s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-store --lib -E 'test(model::source_acquisition::) | test(acquisition::)' 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/core-first.log
```

## core-second.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 600s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-store --lib -E 'test(model::source_acquisition::) | test(acquisition::)' 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/core-second.log
```

## transport-first.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 600s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-crawl --lib -E 'test(net::acquisition::) | test(context::capture::) | test(milesplit::results::run::owned_tests::durable_capture::)' 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/transport-first.log
```

## composed-first.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 600s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-crawl -p census-service --lib -E 'test(net::acquisition::) | test(context::capture::) | test(milesplit::results::run::owned_tests::durable_capture::) | test(capture_tests::) | test(capture_protocol::tests::) | test(capture_admission::) | test(capture_scope::) | test(work_admission::tests::) | test(spawn::)' 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/composed-first.log
```

## composed-second.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 600s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-store -p census-crawl -p census-service --lib -E 'test(model::source_acquisition::) | test(acquisition::) | test(context::capture::) | test(milesplit::results::run::owned_tests::durable_capture::) | test(capture_tests::) | test(capture_protocol::tests::) | test(capture_admission::) | test(capture_scope::) | test(work_admission::tests::) | test(spawn::) | test(research_failure::tests::)' 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/composed-second.log
```

## format-first.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
env -u CI tools/moon-local run pipeline:fmt-write 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/format-first.log
```

## format-second.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
env -u CI tools/moon-local run pipeline:fmt-write 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/format-second.log
```

## format-third.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
env -u CI tools/moon-local run pipeline:fmt-write 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/format-third.log
```

## basic-first.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 600s env -u CI tools/moon-local run pipeline:check pipeline:lint-src pipeline:fmt 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/basic-first.log
```

## basic-second.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 600s env -u CI tools/moon-local run pipeline:check pipeline:lint-src pipeline:fmt 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/basic-second.log
```

## all-target-first.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 600s env -u CI tools/moon-local run pipeline:check 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/all-target-first.log
```

## all-target-serial.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 900s env -u CI tools/moon-local run pipeline:check -- --jobs 1 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/all-target-serial.log
```

## format-fourth.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
env -u CI tools/moon-local run pipeline:fmt-write 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/format-fourth.log
```

## lint-format-final.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 600s env -u CI tools/moon-local run pipeline:lint-src pipeline:fmt 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/lint-format-final.log
```

## composed-third.log
```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 1200s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-store -p census-crawl -p census-service --lib -E 'test(model::source_acquisition::) | test(acquisition::) | test(context::capture::) | test(milesplit::results::run::owned_tests::durable_capture::) | test(capture_tests::) | test(capture_admission::) | test(capture_scope::) | test(work_admission::tests::) | test(spawn::) | test(research_failure::tests::) | test(results_arms::tests::dedicated_worker::) | test(jurisdiction::results_work::replay_tests::) | test(coach_directories::map::postal::tests::) | test(net::types::tests::) | test(jurisdiction::team_source::tests::replay::)' 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/composed-third.log
```

## Required focused selections not established by earlier --lib evidence

These are proposed Root reruns, not historical executed commands. Preserve the same full environment above. A zero selection does not qualify a lane.
```sh
env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --lib -E 'test(jurisdiction::results_work::replay_tests::) | test(results_arms::tests::dedicated_worker::)'
env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --bin census-service -E 'test(cli::capture_protocol::tests::)'
env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --test capture_protocol_cli
```

Expected source inventory: three SDK replay tests, one actual selector/DNS dedicated-worker test, two binary CLI parser/control tests, and three integration CLI tests. These counts are source inventory only until raw selected-name evidence exists. The replay test source is not a native Restate kill/restart campaign.

The final corrected composed lane must additionally include the new SelectedHistory membership test, known-response service cap test, two transport lint-regression tests, and clock-race-repaired bounds assertions. Retain earlier red/green runs separately. Full release, native fault, fuzz, dependency/security, resource and performance lanes remain governed by AGENTS.md, ARCHITECTURE §§10–12 and xtask/README.md.
