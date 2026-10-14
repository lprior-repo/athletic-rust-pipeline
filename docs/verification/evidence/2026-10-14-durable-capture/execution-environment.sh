# Source this file to activate the repository-pinned recovery tools.
export ATHLETIC_TOOLCHAIN_ROOT=/workspace/shared/athletic-recovery-20261013-2203/toolchain
export CARGO_HOME="$ATHLETIC_TOOLCHAIN_ROOT/cargo"
export RUSTUP_HOME="$ATHLETIC_TOOLCHAIN_ROOT/rustup"
export RUSTUP_TOOLCHAIN=nightly-2026-04-27
export PATH="$ATHLETIC_TOOLCHAIN_ROOT/bin:$CARGO_HOME/bin:$PATH"
export XDG_DATA_HOME="$ATHLETIC_TOOLCHAIN_ROOT/xdg-data"
export XDG_CACHE_HOME="$ATHLETIC_TOOLCHAIN_ROOT/xdg-cache"
