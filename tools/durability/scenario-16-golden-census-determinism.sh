#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
SCRATCH_STORE="${SCRATCH_STORE:-${TMPDIR:-/tmp}}"
mkdir -p -- "$SCRATCH_STORE"
SCRATCH_STORE="$(cd "$SCRATCH_STORE" && pwd)"
EVIDENCE_DIR="$(mktemp -d "$SCRATCH_STORE/scenario-16-XXXXXX")"
printf 'EVIDENCE: %s\n' "$EVIDENCE_DIR"
export TMPDIR="$EVIDENCE_DIR/tmp"
mkdir -p -- "$TMPDIR"
export LC_ALL=C

for tool in timeout grep sed sort find sha256sum cmp mktemp wc tail; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'SKIPPED: %s not available\n' "$tool"
        exit 0
    fi
done

cd "$REPO_ROOT"

if [[ ! -x "$REPO_ROOT/tools/moon-cargo" ]]; then
    printf 'SKIPPED: %s not available\n' "$REPO_ROOT/tools/moon-cargo"
    exit 0
fi

if [[ -z "${MOON_RUST_CARGO:-}" ]]; then
    printf 'SKIPPED: MOON_RUST_CARGO unset; invoke through env -u CI tools/moon-local run pipeline:durability -- scenario-16-golden-census-determinism\n'
    exit 0
fi

FIXTURES="crates/census-crawl/tests/fixtures"
EMBEDDED="crates/census-crawl/src/milesplit/owned/fixtures"

for tree in "$FIXTURES/pa_piaa" "$FIXTURES/milesplit" "$EMBEDDED"; do
    if [[ ! -d "$tree" ]]; then
        printf 'FAIL: frozen capture tree missing at %s\n' "$tree"
        exit 1
    fi
done

frozen_manifest() {
    find "$FIXTURES" "$EMBEDDED" -type f -exec sha256sum {} + | sort -k2
}

frozen_manifest > "$EVIDENCE_DIR/frozen-captures-before.sha256"
printf 'PROOF: frozen capture manifest before replay covers %s file(s) under %s and %s\n' \
    "$(wc -l < "$EVIDENCE_DIR/frozen-captures-before.sha256")" "$FIXTURES" "$EMBEDDED"

run_suite() {
    local name="$1"
    local required="$2"
    shift 2
    local log="$EVIDENCE_DIR/$name.log"
    printf 'COMMAND: tools/moon-cargo test %s\n' "$*"
    printf 'COMMAND: tools/moon-cargo test %s\n' "$*" > "$log"
    local rc=0
    set +e
    timeout --signal=TERM 2700s tools/moon-cargo test "$@" >> "$log" 2>&1
    rc=$?
    set -e
    if [[ "$rc" -ne 0 ]]; then
        printf 'FAIL: %s exited %s; log retained %s\n' "$name" "$rc" "$log"
        tail -n 30 -- "$log"
        exit 1
    fi
    if grep -q '^SKIPPED:' "$log"; then
        printf 'FAIL: %s reported SKIPPED output; log retained %s\n' "$name" "$log"
        grep '^SKIPPED:' "$log"
        exit 1
    fi
    if ! grep -q '^test result: ' "$log"; then
        printf 'FAIL: %s reported no test result line; log retained %s\n' "$name" "$log"
        tail -n 30 -- "$log"
        exit 1
    fi
    local line
    while IFS= read -r line; do
        case "$line" in
            'test result: ok.'*'; 0 failed; 0 ignored;'*) ;;
            *)
                printf 'FAIL: %s reported a non-green or skip-affected test result; log retained %s\n' "$name" "$log"
                printf '%s\n' "$line"
                exit 1
                ;;
        esac
    done < <(grep '^test result: ' "$log")
    local last passed
    last="$(grep '^test result: ' "$log" | tail -n 1)"
    passed="$(sed -n 's/^test result: ok\. \([0-9][0-9]*\) passed;.*/\1/p' <<< "$last")"
    if [[ -z "$passed" || "$passed" -lt "$required" ]]; then
        printf 'FAIL: %s passed %s test(s), expected at least %s; log retained %s\n' "$name" "${passed:-unknown}" "$required" "$log"
        exit 1
    fi
    printf 'PROOF: %s green across %s test-result line(s); last: %s\n' "$name" "$(grep -c '^test result: ' "$log" || true)" "$last"
}

require_test() {
    local name="$1"
    local test_name="$2"
    local log="$EVIDENCE_DIR/$name.log"
    if ! grep -q -F "test $test_name ... ok" "$log"; then
        printf 'FAIL: %s did not report %s as ok; log retained %s\n' "$name" "$test_name" "$log"
        grep -F "test $test_name" "$log" || true
        exit 1
    fi
    printf 'PROOF: %s reached %s\n' "$name" "$test_name"
}

require_line() {
    local name="$1"
    local log="$2"
    local pattern="$3"
    local matched=""
    matched="$(grep -m1 -E -- "$pattern" "$log" || true)"
    if [[ -z "$matched" ]]; then
        printf 'FAIL: %s did not print the required artifact /%s/; log retained %s\n' "$name" "$pattern" "$log"
        sed -n '1,40p' -- "$log"
        exit 1
    fi
    printf 'PROOF: %s\n' "$matched"
}

run_replay() {
    local name="$1"
    local source="$2"
    local log="$EVIDENCE_DIR/$name.log"
    printf 'COMMAND: tools/moon-cargo run -p xtask --bin xtask --locked --offline -- replay %s\n' "$source"
    printf 'COMMAND: tools/moon-cargo run -p xtask --bin xtask --locked --offline -- replay %s\n' "$source" > "$log"
    local rc=0
    set +e
    timeout --signal=TERM 2700s tools/moon-cargo run -p xtask --bin xtask --locked --offline -- replay "$source" >> "$log" 2>&1
    rc=$?
    set -e
    if [[ "$rc" -ne 0 ]]; then
        printf 'FAIL: replay %s exited %s; log retained %s\n' "$source" "$rc" "$log"
        tail -n 30 -- "$log"
        exit 1
    fi
    printf 'PROOF: replay %s exited 0; log retained %s\n' "$source" "$log"
}

printf 'OBLIGATION: catalog row 16: replay frozen real captures and retained advice with fixed run semantics; exact IDs and semantic artifacts match, nondeterministic operational measurements stay separate\n'

printf 'SECTION: frozen real captures replay offline through the current parsers, never a re-crawl\n'
run_replay replay-pa-piaa pa_piaa
REPLAY_PA_PIAA="$EVIDENCE_DIR/replay-pa-piaa.log"
require_line replay-pa-piaa "$REPLAY_PA_PIAA" '^source pa_piaa: 10 capture[(]s[)] under crates/census-crawl/tests/fixtures/pa_piaa, offline, no store, no clock$'
golden_a="$(grep -c '"association_id":' "$FIXTURES/pa_piaa/golden_directory_alpha_a.json" || true)"
golden_b="$(grep -c '"association_id":' "$FIXTURES/pa_piaa/golden_directory_alpha_b.json" || true)"
golden_z="$(grep -c '"association_id":' "$FIXTURES/pa_piaa/golden_directory_alpha_z.json" || true)"
if [[ ! "$golden_a" =~ ^[1-9][0-9]*$ || ! "$golden_b" =~ ^[1-9][0-9]*$ || ! "$golden_z" =~ ^[1-9][0-9]*$ ]]; then
    printf 'FAIL: the fixture goldens carry no school ids (a=%s b=%s z=%s)\n' "$golden_a" "$golden_b" "$golden_z"
    exit 1
fi
printf 'PROOF: the fixture goldens pin the expected school-id counts: a=%s b=%s z=%s\n' "$golden_a" "$golden_b" "$golden_z"
require_line replay-pa-piaa "$REPLAY_PA_PIAA" "^directory_alpha_a[.]html +directory_letter letter=a schools=${golden_a} first=\"A J McMullen School\" id=12048$"
require_line replay-pa-piaa "$REPLAY_PA_PIAA" "^directory_alpha_b[.]html +directory_letter letter=b schools=${golden_b} first=\"[^\"]+\" id=[0-9]+$"
require_line replay-pa-piaa "$REPLAY_PA_PIAA" "^directory_alpha_z[.]html +directory_letter letter=z schools=${golden_z} first=\"[^\"]+\" id=[0-9]+$"
require_line replay-pa-piaa "$REPLAY_PA_PIAA" "^golden_directory_alpha_a[.]json +golden_directory_alpha_a ids match directory_alpha_a[.]html [(]${golden_a} school[(]s[)][)]$"
require_line replay-pa-piaa "$REPLAY_PA_PIAA" "^golden_directory_alpha_b[.]json +golden_directory_alpha_b ids match directory_alpha_b[.]html [(]${golden_b} school[(]s[)][)]$"
require_line replay-pa-piaa "$REPLAY_PA_PIAA" "^golden_directory_alpha_z[.]json +golden_directory_alpha_z ids match directory_alpha_z[.]html [(]${golden_z} school[(]s[)][)]$"
require_line replay-pa-piaa "$REPLAY_PA_PIAA" '^details_12048[.]html +details id=12048 school="A J McMullen School" contacts=1$'
require_line replay-pa-piaa "$REPLAY_PA_PIAA" '^robots[.]txt +robots `[*]` group holds [0-9]+ disallow[(]s[)], including /officials/directory/ and permitting /schools/$'
require_line replay-pa-piaa "$REPLAY_PA_PIAA" '^PROVENANCE[.]json +provenance 5 capture[(]s[)] match their recorded bytes and digest$'
require_line replay-pa-piaa "$REPLAY_PA_PIAA" '^golden_details_12048[.]json +golden_details_12048 is a prototype record; the adapter.s golden test compares it$'
require_line replay-pa-piaa "$REPLAY_PA_PIAA" '^10 capture[(]s[)] replayed for source pa_piaa$'
capture_lines="$(grep -c -E '^(directory_alpha_|details_12048|robots[.]txt|PROVENANCE[.]json|golden_)' "$REPLAY_PA_PIAA" || true)"
if [[ "$capture_lines" -ne 10 ]]; then
    printf 'FAIL: replay pa_piaa printed %s per-capture summaries instead of 10; log retained %s\n' "$capture_lines" "$REPLAY_PA_PIAA"
    exit 1
fi
printf 'PROOF: replay pa_piaa printed all 10 per-capture summaries, offline with no store and no clock\n'

printf 'SECTION: the compile-time embedded owned capture and two raw captures replay through the current collector in an isolated offline store, and a rebuilt fixture store keeps semantics while publication identity is not inherited\n'
run_suite parity-pipeline 1 -p census-service --test parity_pipeline --locked --offline -- --nocapture
require_test parity-pipeline rebuilding_the_fixture_store_reproduces_semantics_not_publication_identity
printf 'NOTE: parity_pipeline seeds a fresh tempfile store from the embedded bytes of %s/troy_725218.json and from the raw captures under %s/milesplit, replays them with offline fetches, and asserts zero physical requests, three cache answers, exact athlete and result ids, retained archive bytes against the capture digest, and unchanged journal payloads on repeat\n' "$EMBEDDED" "$FIXTURES"

printf 'SECTION: retained advice replays with fixed run semantics without re-asking or writing another checkpoint\n'
run_suite review-advice-replay 7 -p census-review --lib --locked --offline -- consensus::tests::replay --nocapture
require_test review-advice-replay consensus::tests::replay::exact_dual_advice_replays_without_reasking_or_writing_another_checkpoint
require_test review-advice-replay consensus::tests::replay::historical_single_model_acceptance_is_reasked_and_cannot_survive_disagreement
require_test review-advice-replay consensus::tests::replay::a_case_superseded_during_advice_is_not_resurrected_by_the_checkpoint
require_test review-advice-replay consensus::tests::replay::missing_subjects_are_durable_unresolved_review_not_silent_success

printf 'SECTION: catalog subcase: quarantine one malformed source object while unrelated objects continue; never a successful empty source; retain its bytes and rejection locator\n'
run_suite crawl-quarantine 7 -p census-crawl --lib --locked --offline -- quarantine --nocapture
require_test crawl-quarantine net::cache::archive_tests::quarantine_crashes::every_durable_quarantine_boundary_allows_fresh_publication_without_evidence_loss
require_test crawl-quarantine net::cache::archive_tests::quarantine_crashes::crash_before_reason_publication_does_not_block_recovery_or_remove_orphan_evidence
require_test crawl-quarantine net::cache::archive_tests::quarantine_crashes::occupied_quarantine_destination_is_an_error_and_never_replaced
require_test crawl-quarantine net::cache::archive_tests::quarantine_crashes::interrupted_mutable_metadata_rename_does_not_hide_immutable_archive_corruption
require_test crawl-quarantine net::cache::archive_tests::recovery::repeated_damage_creates_distinct_evidence_without_replacing_prior_quarantine
require_test crawl-quarantine net::cache::archive_tests::recovery::quarantine_io_failure_preserves_damaged_mutable_evidence_and_refuses_publication
require_test crawl-quarantine milesplit::results::pages::tests::an_unreadable_page_is_quarantined_and_the_walk_goes_on
run_suite crawl-roster-quarantine 5 -p census-crawl --lib --locked --offline -- milesplit::tests --nocapture
require_test crawl-roster-quarantine milesplit::tests::an_empty_known_roster_is_a_named_gap_not_negative_identity_evidence
require_test crawl-roster-quarantine milesplit::tests::a_missing_name_does_not_erase_a_readable_provider_identity
require_test crawl-roster-quarantine milesplit::tests::rejected_rows_keep_their_provider_identity_and_exact_utf8_span
require_test crawl-roster-quarantine milesplit::tests::malformed_html_fails_loudly
require_test crawl-roster-quarantine milesplit::tests::roster_entities::lookalike_hosts_cannot_supply_roster_identity
printf 'PROOF: quarantine lanes retain damaged bytes and rejection locators, keep unrelated captures and pages serving, and treat an empty known roster as a named gap rather than negative identity evidence\n'

printf 'SECTION: catalog subcase shared with scenario 14: interrupt export before promotion; the previous accepted generation stays published and a newer partial file is never selected by filename or timestamp\n'
run_suite service-generation-crash 1 -p census-service --lib --locked --offline -- --exact school_address::generation::tests::crash_boundaries_never_publish_a_mixed_generation --nocapture
require_test service-generation-crash school_address::generation::tests::crash_boundaries_never_publish_a_mixed_generation
run_suite exporter-kill-restart 2 -p census-service --test exporter_kill_restart --locked --offline -- --nocapture
require_test exporter-kill-restart a_workbook_export_interrupted_by_sigkill_rebuilds_completely_on_restart
require_test exporter-kill-restart b_workbook_without_interrupt_exits_cleanly
run_suite school-address-publication 3 -p census-service --test school_address_publication --locked --offline -- --nocapture
require_test school-address-publication blocking_destination_preserves_previous_publication
require_test school-address-publication unmanifested_external_baseline_is_not_consumed_or_republished
require_test school-address-publication missing_manifest_artifact_refuses_readback_and_preserves_current
printf 'PROOF: a child killed at each school-address commit point leaves the previous accepted generation byte-identical, an interrupted workbook export never names a partial staging file, and a blocking or unmanifested destination preserves the current publication\n'

printf 'SECTION: derived-generation staging stays invisible until one atomic publication and the next pass reclaims the superseded generation\n'
run_suite store-derived-generations 9 -p census-store --test derived_generations --locked --offline -- --nocapture
require_test store-derived-generations one_stage_holds_one_row_per_id_and_the_last_write_wins
require_test store-derived-generations a_published_generation_is_invisible_until_publish_and_durable_across_reopen
require_test store-derived-generations republishing_one_operation_and_digest_is_a_repeat_and_a_different_digest_is_refused
require_test store-derived-generations a_generation_staged_against_moved_evidence_is_refused
require_test store-derived-generations publishing_reclaims_the_generation_it_supersedes
require_test store-derived-generations reclaim_honours_its_budget_and_finishes_on_the_next_call
require_test store-derived-generations carry_forward_moves_the_current_generation_into_the_next
require_test store-derived-generations a_generation_larger_than_one_staging_batch_publishes_whole
require_test store-derived-generations staging_refuses_tables_that_are_not_generation_partitioned
run_suite reconcile-stage-gate 3 -p census-reconcile --lib --locked --offline -- index::stage_gate_tests --nocapture
require_test reconcile-stage-gate index::stage_gate_tests::a_changed_input_publishes_a_new_generation_holding_the_whole_row_set
require_test reconcile-stage-gate index::stage_gate_tests::a_repeated_pass_publishes_nothing_and_leaves_reclaim_nothing
require_test reconcile-stage-gate index::stage_gate_tests::index_preserves_located_unsupported_cohort_review_without_a_canonical_subject
printf 'PROOF: an uncommitted derived generation is invisible and unreachable by name, becomes readable only after one atomic publication that survives reopen, refuses a repeat with a different digest, is reclaimed by the next pass, and the index gate republishes the whole row set or reclaims its abandoned stage\n'

printf 'SECTION: a restarted workbook exporter republishes identical snapshots and totals for the same frozen store\n'
run_suite service-worker-export 1 -p census-service --test recovery --locked --offline -- --exact worker_export::exporter_restart_republishes_identical_snapshots_and_totals --nocapture
require_test service-worker-export worker_export::exporter_restart_republishes_identical_snapshots_and_totals

frozen_manifest > "$EVIDENCE_DIR/frozen-captures-after.sha256"
if ! cmp -s "$EVIDENCE_DIR/frozen-captures-before.sha256" "$EVIDENCE_DIR/frozen-captures-after.sha256"; then
    printf 'FAIL: frozen capture bytes or paths changed across the replay lanes; diff retained in %s\n' "$EVIDENCE_DIR"
    diff -u "$EVIDENCE_DIR/frozen-captures-before.sha256" "$EVIDENCE_DIR/frozen-captures-after.sha256" || true
    exit 1
fi
printf 'PROOF: the frozen capture manifest is unchanged after every lane (%s file(s), sha256 per file)\n' "$(wc -l < "$EVIDENCE_DIR/frozen-captures-after.sha256")"

printf 'NOTE: nondeterministic operational measurements stay outside the assertions: elapsed times, run durations, cache byte counters and rebuilt publication digests are only recorded in the logs; the determinism lane asserts semantic counts and mapped athletes match across a rebuilt store while its publication digest differs\n'
printf 'LIMIT: the replay lanes are the committed byte-exact fixture captures plus one compile-time embedded owned capture; no live national store, production snapshot or re-crawl is replayed here\n'
printf 'LIMIT: retained-advice replay runs through the in-process loopback lanes of census-review; no local model server or GPU lane was contacted\n'
printf 'LIMIT: the derived-generation lanes exercise interruption as an abandoned unpublished stage and as process-level export interruption; an OS signal delivered inside index materialization is scenario 17s subcase and is not claimed here\n'

printf 'PASS: scenario 16: frozen captures and retained advice replay with exact golden ids and unchanged capture bytes, quarantine retains malformed-source evidence while unrelated work continues, interrupted export never publishes a partial generation, and derived generations stay invisible until one atomic publication\n'
