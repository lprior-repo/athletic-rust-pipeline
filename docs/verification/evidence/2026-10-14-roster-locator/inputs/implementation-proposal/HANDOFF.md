# Positive roster locator preservation proposal

## TASK

Preserve the existing positive MileSplit row locator with its unchanged parsed payload through the actual parser, normalization and service Window/Records route. This is the smallest in-memory slice adopted by Root and the independent contract review.

## OWNERSHIP

This worker wrote only this detached proposal directory. Root remains the sole live repository writer and build runner. `base/` contains exact pre-change copies of every modified existing file; `proposal/` contains the candidate. `changes.patch`, `changed-files.json`, `source-pins.sha256` and `proposal-pins.sha256` identify exact bytes. Baseline HEAD is `3708d6bab0b08c390887c562a89e8c6714255a5d`.

## DO NOT MODIFY

No lifecycle/schema/Store family, acquisition wire, accepted-affiliation API, identity policy, canonical payload shape, parser matching/ownership policy, Journal wire, effect/digest recipe or replay ordering changes. No browser, census, production service, publication, network call or repository commit was performed. The separate TFRRS roster type is untouched.

## INPUT CONTRACT

`LocatedRosterAthlete` privately owns one unchanged `RosterAthlete` and the already computed `SourceRowLocator`. Only the existing parser constructs pairs. Public `athlete()` borrows the payload and `locator()` copies its coordinates; there is no Deref, serde or mutable accessor. Locators describe original regex group-1 inner-li UTF-8 bytes and original matched-row ordinals, including rejected rows.

## OUTPUT CONTRACT

`Roster.athletes` becomes the located vector. `roster_entities`, Window and Counts take located slices, explicitly borrowing payloads. `Records<'a>` retains the exact Window slice, and the real apply loop consumes `records.members()` for counts and checked cursor advancement. No temporary payload vector or parallel locator vector is created. Canonical entity/observation output and the five-family stage order remain unchanged.

Source locators survive while the parsed roster lives. They are not persisted by this slice. Old FetchOutcome serde skips body and old Journal has no positive-member inventory; reopening does not recover positive locators. Existing durable facts/counts/receipts/replay behavior retain their meaning.

## ACCEPTANCE

- Seven new focused regressions cover positive UTF-8 bytes and complete payload; partial gaps and duplicate occurrences; 20,000/20,001 row capacity; canonical output invariance under source-offset changes; multi-window borrow identity/count/cursor alignment; the real apply/replay route; and exact pre-change canonical/Journal/effect bytes.
- Existing owner-context fixtures explicitly compare unchanged payloads and prefix-shifted byte offsets. Full-verdict equality across differently prefixed bodies would now contradict truthful locations.
- Existing direct payload access in crawl, roster observation and recovery fixtures migrates through `athlete()`. Existing chunk/empty-slice normalizer examples, parity fixtures and length-only benchmarks remain source-compatible without cosmetic edits.
- See `test-plan.md` for exact names, pre-change artifact provenance and remaining Root-run gates.

## HANDOFF

`changes.patch` is contextual against copied baseline and passed `git apply --check` there. Static checks confirm unchanged RosterAthlete fields, complete Journal serialized shape/impl, roster_digest/roster_operation bodies and Records::stage body. Current modified production files are at most 288 physical lines, not a trusted callable-size certification.

All implementation compile/test/lint/format/benchmark/fuzz/fault gates are NOT_RUN by this worker under the no-build assignment. Root separately ran the test-only old-code baseline probes successfully before implementation; this is the origin of the frozen golden, not candidate validation. Format and all-target checks remain necessary, then the focused and affected integration suites. No performance or release-readiness claim is made.

The independent reviewer owns a separate static review under `../positive-roster-locator-review/`; its findings and Root's exact integrated checks determine acceptance. Do not regenerate the legacy golden from changed production code to fix a regression.
