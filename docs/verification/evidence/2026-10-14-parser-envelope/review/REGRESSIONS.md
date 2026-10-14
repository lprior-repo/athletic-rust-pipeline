# Root-owned regression acceptance

All commands below are proposed, NOT RUN by reviewer. Working directory: `/workspace/shared/athletic-rebuild-20261013`. Integrate all 36 patch files plus both shared accessor deltas first, preserve archive-lock module registration, and format only under Root ownership. Keep command, exit, selected test names/count, output and final source pins. Zero tests selected is failure.

1. New envelope invariants, exactly six tests:

`timeout 600 tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & test(milesplit::owned::outcome::tests)'`

- `sealed_interpretation_preserves_checked_bytes_and_metadata_through_body_release`: same input Vec pointer/body, actual date, capture serialization, exact parsed verdict; release produces zero length/capacity and unchanged metadata/verdict/provenance; into_capture preserves metadata-only state.
- `sealed_interpretation_resource_returns_the_original_retained_envelope`: exact 20,001/20,000 ArrayValues failure and byte locator; unrecorded failure; original Vec pointer/body/metadata retained; no interpretation journal effect.
- `sealed_interpretation_refuses_historical_authority_before_resource_parsing`: mode mismatch wins over resource parsing; exact CacheOccurrenceMismatch and unchanged journals.
- `sealed_interpretation_refuses_retained_authority_in_historical_context`: reverse mode mismatch has same pre-parse/no-journal ordering.
- `erased_capture_can_only_reenter_as_historical_unknown`: raw value can be interpreted historically without regaining retained authority.
- `synthetic_historical_verdict_never_grants_durable_capture_authority`: fabricated verdict can exist only in historical cfg(test) fixture, with no durable admission.

2. Actual retained producer and acquisition-caller chain:

`timeout 1200 tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & (test(net::acquisition::tests) | test(milesplit::results::run::owned_tests::durable_capture))'`

Include every selected test, especially actual 304 original/date/pointer preservation; eligible cache/changed metadata refusal; actual replay/no new request; Prepared recovery/no hidden refetch; target-bound grant/foreign meet and foreign cache-root refusal; active meet reuses one grant for multiple result sets and new Run replay; durable request projects/reopens/replays same capture. The archive-lock tests must compile/run with migrated getters, but archive-lock correctness has its own reviewer.

3. Exact historical/error/persistence parity and all synthetic fixture families:

`timeout 1200 tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & (test(milesplit::owned::effect) | test(milesplit::map::owned::tests) | test(milesplit::results::run::receipt) | test(milesplit::results::run::owned_tests::resource_recovery))'`

Require exact malformed verdict bytes; fetch/read resource origin and NotRequested/Retained distinction; preserved original resource payloads across cache replay/reopen; partial exact no-op vs new acquisition manifest; unchanged v3/v1 history next to v4 parser interpretation; receipt final URL/ownership/key tests; recording pressure and resource relocation errors. These establish historical compatibility rather than inventing new persisted fixtures or policy revisions.

4. Existing body residency accounting:

`timeout 1800 tools/moon-local run pipeline:tests -- -E 'package(census-crawl) & test(milesplit::results::run::owned_tests::bounded_buffers)'`

Require both `a_thousand_result_sets_share_one_bounded_active_capture_and_preserve_every_fact` and `ten_hundred_thousand_distinct_captures_have_bounded_collector_retention_and_exact_replay`. Retain fact cardinality, replay and peak assertions. Source pointer equality is an ownership regression, not a throughput benchmark.

5. Compilation/format/source gates:

`timeout 600 tools/moon-local run pipeline:fmt`

`timeout 1200 tools/moon-local run pipeline:check`

`timeout 1200 tools/moon-local run pipeline:lint-src`

`env -u CI tools/moon-local run pipeline:xtask -- scan`

`env -u CI tools/moon-local run pipeline:xtask -- comments`

`env -u CI tools/moon-local run pipeline:xtask -- seams`

`env -u CI tools/moon-local run pipeline:xtask -- panic-extraction`

Normal all-target compilation catches actual caller migration; it does not prove forbidden APIs fail. `API-CHECKS.md` supplies the negative probes. This source API tightening is intentional, so public field/enum removal is an accepted breaking change, not a semver-compatible claim. No Moon semver task was found in the inspected command catalog.

Full release/proof/fault/benchmark requirements remain repository-owned and unexecuted here; this focused review does not substitute for them.
