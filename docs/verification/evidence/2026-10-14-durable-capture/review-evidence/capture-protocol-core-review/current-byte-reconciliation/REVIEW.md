# Current-byte capture-core coverage reconciliation

Date: 2026-10-14 07:00 UTC. Scope: exactly the29 paths in capture-evidence-proposal/core-review-gaps.txt, compared only with their latest retained reviewer baseline.

## Result

All29 current hashes match the supplied evidence inventory and are now independently covered by exact retained source pins. No new static blocker was found. Twenty-eight files have formatting-only changes; tests/bounds.rs has the approved test-only clock-race correction plus formatting. There is no production semantic change in this reconciliation.

The reviewer did not edit live source, run a build/test/formatter/gate, restart the broader audit, or interact with production. Root's serial all-target check57308 was in progress when this task was assigned; this report makes no claim about its eventual outcome.

## Exact coverage

- current-source.sha256 pins all29 reviewed current files under source/.
- coverage.json records the exact current hash, latest previously reviewed baseline path/hash/report, and semantic disposition for each path.
- input-manifest.json preserves the baseline/current pairing used for the delta comparison.
- literal-preserving-token-delta.json records the non-whitespace token differences inspected. This is an aid to source review, not a compiler or executable gate.

The baseline is the latest available exact reviewer copy: focused-live-20261014-0644, then root-compile-delta-20261014-0618, then frozen-core-20261014-0615. Those baselines retain the previously approved compile repairs and focused splits where applicable. Current source is separately copied; no historical review snapshot was overwritten.

## Formatting disposition

Across the28 formatting-only paths, differences are import/module ordering, layout, optional trailing commas, and rustfmt-style expression block wrapping. The block wrappers in protocol.rs, request.rs, init.rs, read/closure.rs and lineage.rs preserve the original final expression and error behavior. Import bindings, aliases and visibility are unchanged, independently checked as normalized import sets; acquisition module/export visibility was also inspected directly.

No literal, enum variant, numeric bound, serialized field ordering, digest input, branch condition, terminal classification, budget debit/reset, snapshot creation, Store-origin check, consumed token, or appends guard changed. The approved history grouping and responsibility splits remain covered by formatted-splits-review.md; this task does not expand their scope.

## Clock-race correction

The only semantic delta is test-only, in crates/census-store/src/acquisition/tests/bounds.rs. It corresponds to capture-bounds-clock-race-v1, patch SHA2568e3531f3391d4a8c0230339bfabc88421866cdfd3b4be146805cd471190fa6be. Its manifest's base/source hashes were verified; the current file differs from the proposal only by formatting.

Previously the test compared an envelope built from one SystemClock sample against codec::stage's independently sampled timestamp, so a second boundary could fail an otherwise valid assertion. Store's actual SystemClock uses UTC RFC3339 with SecondsFormat::Secs and Z. The revised size calculation uses a fixed20-byte RFC3339-seconds specimen, without changing production time handling or pretending a clock was injected.

The admitted row still must have exactly65,536 bytes; codec::decode must return the exact payload for the expected key; the actual retained timestamp is read from the actual row; and canonical reconstruction with that timestamp must equal the exact row bytes. The encoded budget still includes the physical key, and the65,537-byte envelope still must refuse before journal mutation. The legal83/default84 joined-closure test is unchanged. There is no sleep, retry loop, weakened threshold or relaxed integrity assertion.

## Boundaries of this result

This completes only current-byte review coverage for the29 requested paths. It does not supersede the original core inventory-capacity and missing-Prepared-backing recovery exclusions. It does not assert execution of the clock correction, post-split tests, or all-target checks. Any later source hash change requires its own delta coverage.
