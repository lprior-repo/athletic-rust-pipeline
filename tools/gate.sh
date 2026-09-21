#!/usr/bin/env bash
# Quality gate for the athletic-rust-pipeline workspace.
#
# Full-strength lints, no allow-lists, no weakened checks. Existing debt is tracked in
# tools/quality-baseline.json and may only shrink: every lane that measures debt fails when a
# number grows, and prints the remaining count so burndown progress is visible on every run.
#
#   tools/gate.sh                  run every lane, compare debt against the baseline
#   tools/gate.sh --update-baseline   rewrite the baseline from current measurements
#                                     (refuses to raise a number unless --allow-increase)
#
# Lanes: fmt, check, doc, tests, strict clippy (source targets), production scan + size budgets,
#        debt ratchet, deny, audit, machete, geiger, bench presence.
set -uo pipefail

cd "$(dirname "$(readlink -f "$0")")/.." || exit 2

BASELINE=tools/quality-baseline.json
UPDATE=0
ALLOW_INCREASE=0
for arg in "$@"; do
  case "$arg" in
    --update-baseline) UPDATE=1 ;;
    --allow-increase) ALLOW_INCREASE=1 ;;
    *) printf 'unknown argument: %s\n' "$arg" >&2; exit 2 ;;
  esac
done

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
# machine; install the tools listed in the SKIP lines to make those lanes real.
run_tool_lane() {
  local tool="$1" name="$2"; shift 2
  if ! command -v "$tool" > /dev/null 2>&1; then
    printf '\n=== %s ===\nSKIP: %s is not installed (cargo install %s)\n' "$name" "$tool" "$tool"
    return 0
  fi
  run_lane "$name" "$@"
}

lane_fmt() { cargo fmt --all -- --check; }
lane_check() { cargo check --workspace --all-targets --all-features; }
lane_doc() { cargo doc --workspace --all-features --no-deps; }
lane_deny() { cargo deny check; }
lane_audit() { cargo audit --quiet; }
lane_machete() { cargo machete; }
lane_geiger() { cargo geiger --workspace --all-features --output-format Json > /dev/null; }
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
measure_clippy() {
  cargo clippy --workspace --lib --bins --examples --all-features \
    --message-format=json -- "${LINT_SET[@]}" 2>/dev/null |
    jq -r 'select(.reason=="compiler-message") | select(.message.level=="error")
           | "\(.target.name)\t\(.message.code.code // "no-code")"' |
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
  local tmp
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT

  run_lane fmt lane_fmt
  run_lane check lane_check
  run_lane doc lane_doc
  run_lane tests lane_tests

  printf '\n=== strict clippy (source targets) ===\n'
  measure_clippy > "$tmp/clippy.tsv"
  sed 's/^/  /' "$tmp/clippy.tsv"
  printf '  total diagnostics: %s\n' "$(awk '{s+=$3} END {print s+0}' "$tmp/clippy.tsv")"

  printf '\n=== production scan (forbidden constructs + size budgets) ===\n'
  python3 tools/production_scan.py > "$tmp/scan.json"
  jq -r '.crates | to_entries[] | "  \(.key): expect=\(.value.expect) unwrap=\(.value.unwrap) unsafe=\(.value.unsafe) assert=\(.value.assert_family) panic=\(.value.panic) indexing=\(.value.indexing) as=\(.value.as_cast) prod_lines=\(.value.production_lines)"' "$tmp/scan.json"
  jq -r '"  structure: files>300=\(.structure.files_over_300_lines | length) fns>60=\(.structure.functions_over_60_lines) fns>25logical=\(.structure.functions_over_25_logical_lines)"' "$tmp/scan.json"

  printf '\n=== domain type integrity (review candidates, ratcheted in the DDD phase) ===\n'
  python3 tools/type_integrity_scan.py | jq -r 'to_entries[] | "  \(.key): bool_sigs=\(.value.bool_in_signature | length) primitive_ids=\(.value.primitive_id_param | length) many_option_structs=\(.value.struct_with_many_options | length)"'

  printf '\n=== domain purity (census-domain normal tree) ===\n'
  if python3 tools/check_domain_purity.py; then
    printf -- '--- domain purity: PASS\n'
  else
    printf -- '--- domain purity: FAIL\n'
    FAILURES+=("domain purity")
  fi

  if [ "$UPDATE" = 1 ]; then
    printf '\n=== baseline update ===\n'
    local extra=()
    [ "$ALLOW_INCREASE" = 1 ] && extra+=(--allow-increase)
    if python3 tools/update_baseline.py "$BASELINE" "$tmp/clippy.tsv" "$tmp/scan.json" "${extra[@]}"; then
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
  elif python3 tools/ratchet.py "$BASELINE" "$tmp/clippy.tsv" "$tmp/scan.json"; then
    printf -- '--- ratchet: PASS\n'
  else
    printf -- '--- ratchet: FAIL\n'
    FAILURES+=("ratchet")
  fi

  run_lane deny lane_deny
  run_tool_lane cargo-audit audit lane_audit
  run_tool_lane cargo-machete machete lane_machete
  run_tool_lane cargo-geiger geiger lane_geiger
  run_lane "bench presence" lane_bench_presence
  summary
}

main
