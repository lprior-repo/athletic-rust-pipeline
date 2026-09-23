#!/usr/bin/env bash
# Quality gate for the athletic-rust-pipeline workspace.
#
# Full-strength lints, no allow-lists, no weakened checks. Existing debt is tracked in
# tools/quality-baseline.json and may only shrink: every lane that measures debt fails when a
# number grows, and prints the remaining count so burndown progress is visible on every run.
#
# What ratchets is the debt: the forbidden constructs, the one-page function budget, and the
# oversized-file ledger. Line counts and the 25-logical-line target describe the tree rather than the
# debt — they rise with every new feature — so the ratchet prints them with `(context)` and
# `(target, not a budget)` instead of failing, and a new metric name fails until the baseline records
# what it is. See `xtask/src/baseline.rs`.
#
#   tools/gate.sh                  run every lane, compare debt against the baseline
#   tools/gate.sh --full           add the slow lane (mutation testing) that a pre-release pass
#                                  needs but a per-commit one cannot afford
#   tools/gate.sh --release        the pre-release pass: every lane runs, `--full`'s heavy lanes
#                                  included, and a missing tool is a FAILURE instead of a SKIP
#   tools/gate.sh --update-baseline   rewrite the baseline from current measurements
#                                     (refuses to raise a number unless --allow-increase)
#
# Two modes, one lane list. The default run is the per-edit pass: a lane whose cargo subcommand is
# not installed prints SKIP and returns, so the gate stays usable on a machine that has not
# `cargo install`ed every tool, and the lanes that need no tool always run. `--release` is the pass
# a release is signed off on: a SKIP there would be a claim that a lane's coverage was not needed,
# and a release cannot make that claim, so an absent tool is a FAILURE and the heavy lanes run
# without `--full` being spelled out as well.
#
# Lanes: fmt, check, doc, tests, strict clippy (source targets), production scan + size budgets,
#        domain integrity, debt ratchet, deny, audit, vet, machete, geiger, feature powerset, bench
#        presence; --full adds mutants.
#
# The toolchain is the pinned nightly from rust-toolchain.toml, and the check and clippy lanes pass
# `-Zallow-features=portable_simd,try_blocks`: the nightly feature allowlist is part of the source
# policy, so a feature gate outside it fails here instead of being discovered at review time. On a
# stable toolchain those two lanes fail on the `-Z` flag rather than quietly dropping the check.
#
# Every lane that measures code is a `xtask` subcommand — `scan`, `integrity`, `domain-purity`,
# `quality-baseline`, `ratchet` — so the measurements live next to the rest of the developer
# commands and nothing here carries analysis code of its own.
set -uo pipefail

cd "$(dirname "$(readlink -f "$0")")/.." || exit 2

BASELINE=tools/quality-baseline.json
UPDATE=0
ALLOW_INCREASE=0
FULL=0
RELEASE=0
for arg in "$@"; do
  case "$arg" in
    --update-baseline) UPDATE=1 ;;
    --allow-increase) ALLOW_INCREASE=1 ;;
    --full) FULL=1 ;;
    --release) RELEASE=1; FULL=1 ;;
    *) printf 'unknown argument: %s\n' "$arg" >&2; exit 2 ;;
  esac
done

# The nightly feature allowlist (Holzman pinned-nightly policy). `-Zallow-features` is
# crate-graph-wide rather than per-crate, so it carries two kinds of name. Ours are the two features
# this workspace's own source may gate on (`portable_simd`, `try_blocks`); the scan enforces exactly
# that list per crate through its `unstable_features` metric, so a gate on anything else fails there
# whether or not the compiler flag is present. The rest are the features the dependency graph probes
# for itself on a nightly compiler: `proc-macro2` probes `proc_macro_span` and `anyhow` probes
# `error_generic_member_access`, and without the name here those crates fail to compile under the
# flag. Our own crates gate on none of them.
FEATURE_ALLOWLIST=portable_simd,try_blocks,proc_macro_span,error_generic_member_access

LINT_SET=(
  -D warnings
  -D unsafe_code
  -D clippy::unwrap_used
  -D clippy::expect_used
  -D clippy::panic
  -D clippy::panic_in_result_fn
  -D clippy::todo
  -D clippy::unimplemented
  -D clippy::dbg_macro
  -D clippy::indexing_slicing
  -D clippy::string_slice
  -D clippy::get_unwrap
  -D clippy::arithmetic_side_effects
  -D clippy::as_conversions
  -D clippy::let_underscore_must_use
  -D clippy::await_holding_lock
)

FAILURES=()
run_lane() {
  local name="$1"; shift
  printf '\n=== %s ===\n' "$name"
  if "$@"; then
    printf -- '--- %s: PASS\n' "$name"
  else
    printf -- '--- %s: FAIL\n' "$name"
    FAILURES+=("$name")
  fi
}

# Lanes that need a cargo subcommand: absent locally prints SKIP so the gate stays usable on any
# machine; install the tools listed in the SKIP lines to make those lanes real. Under `--release` a
# missing tool is a FAILURE: the release pass exists to prove every lane ran, and an uninstalled tool
# proves the opposite. `lane_mutants` is reached through here too, so `--release` without `--full`
# still fails on a missing cargo-mutants.
run_tool_lane() {
  local tool="$1" name="$2"; shift 2
  if ! command -v "$tool" > /dev/null 2>&1; then
    if [ "$RELEASE" = 1 ]; then
      printf '\n=== %s ===\nFAIL: %s is not installed (cargo install %s), and a release pass runs every lane\n' \
        "$name" "$tool" "$tool"
      FAILURES+=("$name")
      return 1
    fi
    printf '\n=== %s ===\nSKIP: %s is not installed (cargo install %s)\n' "$name" "$tool" "$tool"
    return 0
  fi
  run_lane "$name" "$@"
}

lane_fmt() { cargo fmt --all -- --check; }
lane_check() {
  cargo -Zallow-features="$FEATURE_ALLOWLIST" check --workspace --all-targets --all-features
}
lane_doc() { cargo doc --workspace --all-features --no-deps; }
lane_deny() { cargo deny check; }
lane_audit() { cargo audit --quiet; }
lane_vet() { cargo vet --locked; }
# `cargo machete` hands the tool the subcommand token as `argv[1]`, and cargo-machete 0.9.2 strips
# that token only when `argv[0]` is a path: resolved through the mise shim (`argv[0]` =
# `cargo-machete`) the token is read as a directory and the lane dies with
# `IO error for operation on machete`. Calling the resolved binary by path with the directory it
# should analyze removes the argv[0] dependence — the lane then measures dependencies, not PATH.
lane_machete() { "$(command -v cargo-machete)" .; }
lane_geiger() { cargo geiger --workspace --all-features --output-format Json > /dev/null; }
# Every feature combination compiles: `loom` in midwest-census gates the concurrency models, and the
# root crate's telemetry sinks are optional, so a combination that only breaks under one of them
# would otherwise reach review.
lane_hack() { cargo hack check --workspace --feature-powerset; }
# Mutation testing: the slowest lane by far and the only one that measures whether the tests can
# fail. It is opt-in (`--full`) and belongs to the pre-release pass, not to every edit.
lane_mutants() { cargo mutants --workspace --in-place; }
lane_tests() {
  if command -v cargo-nextest > /dev/null 2>&1; then
    cargo nextest run --workspace --all-features
  else
    printf 'cargo-nextest absent: falling back to cargo test\n'
    cargo test --workspace --all-features --quiet
  fi
}
lane_bench_presence() {
  if [ -d benches ] || [ -d crates/midwest-census/benches ]; then
    cargo bench --workspace --no-run
  else
    printf 'no benchmark target exists yet: performance claims stay blocked until Phase 6 adds one\n'
    return 0
  fi
}

# Debt measurement: strict clippy on source targets, counted per crate and lint code.
#
# The raw JSON lands in a file first. An empty tally is a legitimate result on a clean tree, so
# "clippy counted no diagnostics" and "clippy never ran" would otherwise look identical to the
# ratchet — and the second one must never pass. The `build-finished` record is the proof that the
# compiler actually ran; `pipefail` (set at the top) is what makes a `jq` that cannot parse its
# input fail this function instead of quietly emitting nothing.
measure_clippy() {
  local raw="$1"
  cargo -Zallow-features="$FEATURE_ALLOWLIST" clippy --workspace --lib --bins --examples --all-features \
    --message-format=json -- "${LINT_SET[@]}" 2>/dev/null > "$raw"
  if ! grep -q '"reason":"build-finished"' "$raw"; then
    printf 'clippy produced no build-finished record: the measurement did not run, so its count is not evidence\n' >&2
    return 1
  fi
  jq -r 'select(.reason=="compiler-message") | select(.message.level=="error")
         | "\(.target.name)\t\(.message.code.code // "no-code")"' "$raw" |
    sort | uniq -c | awk '{printf "%s\t%s\t%s\n", $2, $3, $1}'
}

summary() {
  printf '\n================ summary ================\n'
  if [ ${#FAILURES[@]} -eq 0 ]; then
    printf 'gate: PASS (debt ratchet holds; counts above)\n'
    exit 0
  fi
  printf 'gate: FAIL -> %s\n' "${FAILURES[*]}"
  exit 1
}

main() {
  # Fail fast on the measurement prerequisites. `measure_clippy` and the scan step turn their JSON
  # into the ratchet's tallies through `jq`; without it both come back empty, an empty tally has
  # nothing to compare, and the gate would print PASS on no evidence. A missing cargo subcommand is a
  # SKIP lane because it only removes one lane's coverage (see `run_tool_lane`); a missing `jq`
  # removes the evidence under a lane that still reports PASS, so it is a hard failure here.
  if ! command -v jq > /dev/null 2>&1; then
    printf 'jq is required: the clippy and scan lanes feed the debt ratchet through it\n' >&2
    FAILURES+=("jq")
    summary
  fi

  local tmp
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT

  run_lane fmt lane_fmt
  run_lane check lane_check
  run_lane doc lane_doc
  run_lane tests lane_tests

  printf '\n=== strict clippy (source targets) ===\n'
  # A measurement that did not run leaves `$tmp/clippy.tsv` empty, which the ratchet reads as "no
  # clippy debt" — so the lane itself takes the failure here; the gate cannot print PASS with
  # `clippy` in FAILURES, whatever the ratchet concludes from the empty tally.
  if measure_clippy "$tmp/clippy.json" > "$tmp/clippy.tsv"; then
    sed 's/^/  /' "$tmp/clippy.tsv"
    printf '  total diagnostics: %s\n' "$(awk '{s+=$3} END {print s+0}' "$tmp/clippy.tsv")"
  else
    printf -- '--- strict clippy: FAIL (the measurement did not run)\n'
    FAILURES+=("clippy")
  fi

  printf '\n=== production scan (forbidden constructs + size budgets) ===\n'
  if cargo run -q -p xtask -- scan > "$tmp/scan.json"; then
    jq -r '.crates | to_entries[] | "  \(.key): expect=\(.value.expect) unwrap=\(.value.unwrap) unsafe=\(.value.unsafe) assert=\(.value.assert_family) panic=\(.value.panic) indexing=\(.value.indexing) as=\(.value.as_cast) prod_lines=\(.value.production_lines)"' "$tmp/scan.json"
    jq -r '"  structure: files>300=\(.structure.files_over_300_lines | length) fns>60=\(.structure.functions_over_60_lines) fns>25logical=\(.structure.functions_over_25_logical_lines)"' "$tmp/scan.json"
    jq -r '.structure.functions_over_60_sites[]? | "  over the one-page budget: \(.)"' "$tmp/scan.json"
    jq -r '.structure.unstable_feature_sites[]? | "  feature gate outside the allowlist: \(.)"' "$tmp/scan.json"
  else
    printf -- '--- production scan: FAIL\n'
    FAILURES+=("production scan")
  fi

  printf '\n=== domain type integrity (review candidates, ratcheted in the DDD phase) ===\n'
  # The lane used to discard the command's status, so a scan that failed read as a scan of zero
  # candidates — and this report is the ratchet's input, so a missing report must never look like a
  # clean one. The status is checked, and the report has to carry `ok: true`: `xtask integrity` sets
  # it false when a domain root it declares yields no production file at all.
  local integrity_rows='to_entries[] | select(.key != "ok") | "  \(.key): bool_sigs=\(.value.bool_in_signature | length) primitive_ids=\(.value.primitive_id_param | length) many_option_structs=\(.value.struct_with_many_options | length)"'
  if cargo run -q -p xtask -- integrity > "$tmp/integrity.json" \
    && jq -e '.ok == true' "$tmp/integrity.json" > /dev/null 2>&1; then
    jq -r "$integrity_rows" "$tmp/integrity.json"
    printf -- '--- domain type integrity: PASS\n'
  else
    jq -r "$integrity_rows" "$tmp/integrity.json" 2> /dev/null
    printf -- '--- domain type integrity: FAIL\n'
    FAILURES+=("domain integrity")
  fi

  printf '\n=== domain purity (census-domain normal tree) ===\n'
  if cargo run -q -p xtask -- domain-purity; then
    printf -- '--- domain purity: PASS\n'
  else
    printf -- '--- domain purity: FAIL\n'
    FAILURES+=("domain purity")
  fi

  printf '\n=== module seams (census top-level module edges) ===\n'
  if cargo run -q -p xtask -- seams > "$tmp/seams.json"; then
    jq -r '"  modules: \(.modules | length)  allowed edges observed: \(.edges | length)"' "$tmp/seams.json"
    printf -- '--- module seams: PASS\n'
  else
    jq -r '.violations[]? | "  \(.from) -> \(.to) at \(.file):\(.line)"' "$tmp/seams.json" 2> /dev/null
    printf -- '--- module seams: FAIL\n'
    FAILURES+=("module seams")
  fi

  if [ "$UPDATE" = 1 ]; then
    printf '\n=== baseline update ===\n'
    local extra=()
    [ "$ALLOW_INCREASE" = 1 ] && extra+=(--allow-increase)
    if cargo run -q -p xtask -- quality-baseline "$BASELINE" "$tmp/clippy.tsv" "$tmp/scan.json" "${extra[@]}"; then
      printf -- '--- baseline update: PASS\n'
    else
      printf -- '--- baseline update: FAIL\n'
      FAILURES+=("baseline update")
    fi
    summary
  fi

  printf '\n=== debt ratchet ===\n'
  if [ ! -f "$BASELINE" ]; then
    printf 'no baseline at %s: run tools/gate.sh --update-baseline once\n' "$BASELINE"
    FAILURES+=("ratchet")
  elif cargo run -q -p xtask -- ratchet "$BASELINE" "$tmp/clippy.tsv" "$tmp/scan.json"; then
    printf -- '--- ratchet: PASS\n'
  else
    printf -- '--- ratchet: FAIL\n'
    FAILURES+=("ratchet")
  fi

  run_lane deny lane_deny
  run_tool_lane cargo-audit audit lane_audit
  run_tool_lane cargo-vet vet lane_vet
  run_tool_lane cargo-machete machete lane_machete
  run_tool_lane cargo-geiger geiger lane_geiger
  run_tool_lane cargo-hack "feature powerset" lane_hack
  run_lane "bench presence" lane_bench_presence
  if [ "$FULL" = 1 ]; then
    run_tool_lane cargo-mutants mutants lane_mutants
  fi
  summary
}

main
