#!/usr/bin/env bash
set -euo pipefail
name=${0##*/}
case "$name:$*" in
  rustup:'which cargo') printf '%s/cargo\n' "$FIXTURE_BIN" ;;
  rustup:'which rustc') printf '%s/rustc\n' "$FIXTURE_BIN" ;;
  cargo:-vV) printf 'cargo fixture\n' ;;
  cargo:'-Z unstable-options config get '* )
    if [[ -n "${FIXTURE_CONFIG_ERROR:-}" ]]; then
      printf '%s\n' "$FIXTURE_CONFIG_ERROR" >&2
      exit 1
    fi
    printf '{"%s":{}}\n' "$5"
    ;;
  rustc:-vV|selected-rustc:-vV) printf 'rustc fixture\nhost: %s\n' "$FIXTURE_HOST" ;;
  rustc:'--print sysroot'|selected-rustc:'--print sysroot') printf '%s\n' "$FIXTURE_SYSROOT" ;;
  rustc:'--print cfg -C target-cpu=native'|selected-rustc:'--print cfg -C target-cpu=native')
    printf 'target_feature="fixture"\n' ;;
  cc:--version|c++:--version|selected-cc:--version|selected-c++:--version)
    printf '%s fixture\n' "$name" ;;
  cc:-print-prog-name=ld|selected-cc:-print-prog-name=ld)
    printf '%s\n' "$FIXTURE_LINKER"
    exit "${FIXTURE_LINKER_PROBE_EXIT:-0}"
    ;;
  ar:--version)
    case "$FIXTURE_HOST" in
      *-apple-darwin) printf 'Apple ar does not accept --version\n' >&2; exit 1 ;;
      *) printf 'GNU ar fixture\n' ;;
    esac
    ;;
  custom-ar:--version) printf 'custom ar fixture\n' ;;
  selected-ar:*) printf 'Apple ar has no version option\n' >&2; exit 1 ;;
  cargo-nextest:--version) printf 'cargo-nextest fixture\n' ;;
  moon:--version) printf 'moon fixture\n' ;;
  moon:*)
    printf '%s\0' "$PWD" "$MOON_RUST_CARGO" "$RUSTC" "$RUSTC_WRAPPER" \
      "$MOON_SKIP_SYNC_WORKSPACE" "${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER-}" \
      "${CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER-}" "${CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER-}" "$@"
    exit 37
    ;;
  xcrun:'--find ar')
    printf '%s/%s\n' "$FIXTURE_BIN" "${FIXTURE_APPLE_AR:-ar}"
    exit "${FIXTURE_AR_PROBE_EXIT:-0}"
    ;;
  xcrun:'--find cc') printf '%s/selected-cc\n' "$FIXTURE_BIN" ;;
  xcrun:'--find c++') printf '%s/selected-c++\n' "$FIXTURE_BIN" ;;
  xcrun:'--find ld') printf '%s/ld\n' "$FIXTURE_BIN" ;;
  *) printf 'unexpected fixture invocation: %s %s\n' "$name" "$*" >&2; exit 91 ;;
esac
