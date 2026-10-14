# Actual Root execution commands

These are transcriptions of Root execution records, not commands executed by this evidence worker. Working directory: /workspace/shared/athletic-rebuild-20261013. Shell: bash. HOME was inherited and not overridden; its historical literal value is unavailable. No guessed home path or environment value is supplied.

The first two commands ran sequentially; Root then ran lint, all-target check and format serially, preserving PIPESTATUS individually. Numeric exit attestations and log-observed terminal states are separated in verification-ledger.jsonl.

## composed-third

Status at snapshot: pass_terminal_log.

```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 1200s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-store -p census-crawl -p census-service --lib -E 'test(model::source_acquisition::) | test(acquisition::) | test(context::capture::) | test(milesplit::results::run::owned_tests::durable_capture::) | test(capture_tests::) | test(capture_admission::) | test(capture_scope::) | test(work_admission::tests::) | test(spawn::) | test(research_failure::tests::) | test(results_arms::tests::dedicated_worker::) | test(jurisdiction::results_work::replay_tests::) | test(coach_directories::map::postal::tests::) | test(net::types::tests::) | test(jurisdiction::team_source::tests::replay::)' 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/composed-third.log
```

## cli-first

Status at snapshot: pass_terminal_log.

```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 1200s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --bin census-service --test capture_protocol_cli -E 'test(cli::capture_protocol::tests::) | binary(capture_protocol_cli)' 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/cli-first.log
```

## lint-third

Status at snapshot: pass_terminal_log.

```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 900s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:lint-src 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/lint-third.log
```

## all-target-final

Status at snapshot: pass_terminal_log.

```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 900s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:check -- --jobs 1 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/all-target-final.log
```

## format-final

Status at snapshot: pass_terminal_log.

```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
env -u CI tools/moon-local run pipeline:fmt 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/format-final.log
```

## parity-final

Status at snapshot: pass_terminal_log.

```sh
cd /workspace/shared/athletic-rebuild-20261013
source /workspace/shared/athletic-recovery-20261013-2203/toolchain/env.sh
set -o pipefail
timeout --signal=INT --kill-after=10s 1200s env -u CI CARGO_INCREMENTAL=0 tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-service --test parity_pipeline 2>&1 | tee /workspace/shared/athletic-recovery-20261013-2203/capture-integration-evidence/parity-final.log
```

## Explicit recovery toolchain setup

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

All explicit exports are retained here. Ambient environment variables were not retrospectively captured. `moon.yml:68–77` makes tests-scoped manual and uncached, and its command selects all features. Current logs show local compilation and fresh Nextest run IDs after the remote cache connection warning. This establishes actual test execution, not cold compilation or no compiler-state reuse.
