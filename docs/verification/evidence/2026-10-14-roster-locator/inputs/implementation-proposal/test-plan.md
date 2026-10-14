# Regression and compatibility plan

Candidate tests are authored but NOT_RUN by this worker. Root alone runs Moon checks after integration.

## Added crawl tests

- `positive_roster_locator_preserves_exact_utf8_bytes_and_unchanged_payload`: exact inner-li span after a multibyte prefix; independently authored whole RosterAthlete payload including decoded entity/non-ASCII name and season.
- `partial_roster_keeps_global_ordinals_and_duplicate_occurrence_positions`: positive ordinals 0/2 around rejected ordinal 1; identical payloads keep distinct source occurrences; independently calculated source byte offsets.
- `positive_roster_locator_capacity_keeps_only_the_existing_bounded_prefix`: exactly 20,000 members is complete; row 20,001 becomes only the existing unfinished locator. Original body cap is retained.
- `canonical_roster_projection_is_unchanged_by_positive_locator_offsets`: existing Wisconsin fixture, unchanged canonical bytes when a non-ASCII prefix shifts only source coordinates. The independent pre-change golden below supplies the historical compatibility oracle.

## Added service tests

- `positive_roster_windows_and_records_borrow_the_same_global_members`: 67 source rows, rejected ordinal 31, 66 positive members; windows of 64 and 2; pointer-and-length equality with original slice and Records; Records outlives the temporary Window; globally numbered later members; byte slices; exact athlete/cohort/gender/program counts and checked cursor.
- `actual_roster_apply_keeps_partial_window_counts_and_legacy_replay`: real service apply twice on that input, unchanged partial/owed state, one rejection and 66 canonical athletes, no duplication.
- `positive_roster_preserves_prechange_canonical_journal_and_effect_bytes`: exact strings for canonical school, nonempty teams, athletes, source-school observation, athlete observations and Journal; exact facts/effect digests from old-code baseline. Input guards require two athletes, two teams, two athlete observations, gender/year order, partial rejection and counts before comparison.

## Old-code baseline provenance

Root applied each test-only probe to clean baseline HEAD 3708d6ba, ran the named probe, then reversed its exact patch and confirmed a clean checkout before production integration. These probe patches and logs remain separate from changes.patch.

V1: `baseline-probe/probe.patch`, original output `baseline-probe/legacy-bytes.json`, 4001 bytes, SHA256 0114d5deea680ddcfa56baec2fa5487f8f6fd594034dc0f190722ae7ad57f9cc. Root reports 1/1 PASS, 3271 skipped, nextest 684a5b5e-e3d1-4b36-bb16-7282385a4467, outer exit 0. Its team array was empty because old test-style season markup did not match production season parsing. It remains historical evidence and is not the final regression fixture.

V2: `baseline-probe-v2/probe.patch`, original output `baseline-probe-v2/legacy-bytes.json`, 4971 bytes, SHA256 5dce866cca6bab85495c5dc46d9e9e6610d7eda53e63305374153b6e88827fed. Root reports 1/1 PASS, 3271 skipped, nextest 445729b4-0a11-472d-896d-8054ba1bb501, outer exit 0. V2 uses actual data-point/icon icon-yes markup and asserts exact nonempty outputs, gender/cohort and rejection state before exporting.

Root's command form was `POSITIVE_ROSTER_BASELINE_PATH=<matching detached probe output path> tools/moon-local run pipeline:tests-scoped -- --build-jobs 1 -p census-domain -p census-store -p census-crawl -p census-service --lib -E 'test(positive_roster_legacy_bytes_probe)'`. These successful baseline commands were executed by Root, not this worker.

The installed final fixture is the V2 JSON plus one terminal LF outside its serialized payload strings. No data values or serialized strings were altered. Final legacy_compatibility input is identical to the V2 probe; only member consumption and the writer test were replaced with production member access and fixed-golden comparison.

## Existing mandatory regression coverage

Retain parser owner/final-owner/ownerless/inert-markup tests; all existing direct-year/no-invented-grade checks; `partial_roster_survives_reopen_and_corrected_body_adds_only_missing_facts`; `thousands_of_rows_resume_after_one_atomic_window_without_duplicate_facts`; resource refusal, incomplete journal and partial/quarantine replay. Reopen assertions cover old durable facts and counts, not positive-locator retention.

Root should compile all library/test/example/benchmark callers, run format and lint, then select actual roster unit and affected integration suites through tools/moon-local. The affected integration source targets include milesplit_roster_observations, boundary_points, recovery_mod/jurisdiction_replay and parity_pipeline_mod/fixture_builders; use the repository's actual binary names/filters rather than assuming a module is a test binary. Required broader release/fault/resource obligations remain unchanged.
