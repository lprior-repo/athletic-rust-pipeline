# In-memory positive roster locator preservation

## TASK

Preserve each syntactically parsed positive MileSplit roster row's existing source locator through the actual parser, normalization and roster service Window/Records route. Preserve canonical projection, observation multiplicity, counts, effect identities and serialized historical Journal behavior. This is useful parser/consumer groundwork, not durable source interpretation or accepted affiliation.

## OWNERSHIP / DO NOT MODIFY

Root owns shared API adoption, integration and checks. One implementation owner should retain the complete narrow caller migration. No Store/domain/report/Workbook/acquisition/lifecycle wire changes, no new source research or requests, no source-authorship adjudication, no new accepted types, no comments or new dependencies. No change to matching/ownership policy, graduation/cohort semantics, row capacity, existing partial/quarantine outcomes, canonical IDs, old historical semantics or test expectations unrelated to the wrapper.

## INPUT CONTRACT

Keep `RosterAthlete`'s current payload fields and methods unchanged. Keep the existing `SourceRowLocator` and its meanings unchanged:

- `ordinal` is the zero-based ordinal of the matched roster row in the original complete input, including rejected rows; it is not an index into the compact positive vector or into a service window.
- `byte_offset` and `byte_length` refer to regex capture group 1 in the original UTF-8 input. This is the inner `<li>` content, not the outer tags, decoded name, character count, normalized bytes or a substring searched again by provider ID.
- A row after multibyte text or after a rejected row keeps its original byte offset and ordinal. Duplicated payloads/IDs remain distinct source occurrences.
- The overflow row at ordinal 20,000 remains the existing unfinished locator; it does not enter the positive vector.
- The locator is a raw source-coordinate DTO. It is not a checked official claim, publication authority, accepted affiliation or HS-status witness.

## OUTPUT CONTRACT / exact API choices

1. In existing `census-crawl/src/milesplit/wire.rs`, introduce `LocatedRosterAthlete` with exactly one unchanged owned `RosterAthlete` and one existing `SourceRowLocator`. Use private fields; expose `athlete(&self) -> &RosterAthlete` and `locator(&self) -> SourceRowLocator`. An internal constructor may take those two values. Derive only the needed existing value traits (`Debug`, `Clone`, `PartialEq`, `Eq`); no Serialize/Deserialize, Deref, accepted constructor, duplicate payload fields or parallel locator collection. Re-export the wrapper from existing `milesplit.rs` alongside Roster/RosterAthlete.
2. Change `Roster.athletes` to `Vec<LocatedRosterAthlete>`. Keep the field name so length/chunk/emptiness callers need no artificial migration. Keep `RosterAthlete` available as the payload type; do not maintain a second old Roster execution path.
3. In `parse/roster.rs`, `Parsed.athletes` becomes the located vector. `Fields::athlete` still parses an unchanged payload. In `Parsed::admit`, pair its successful value with the already computed locator immediately before the existing fallible reserve/push. Rejections and unfinished handling remain byte-for-byte unchanged in meaning. Do not re-run the matcher or infer location from a payload.
4. Change the one existing `normalize::roster_entities` parameter from `&[RosterAthlete]` to `&[LocatedRosterAthlete]`. Keep the remaining parameters and return tuple unchanged. Its admission and projection loops borrow `entry.athlete()`. `sport_iter`, `RosterAthlete::sports` and `roster_athlete_entity` can continue to take the unchanged payload. Do not collect, clone or allocate a payload vector; do not add a generic adapter/parallel API for absent callers. Empty `&[]` continues to infer the located slice for the existing school-only example.
5. `Window<'a>` in service `metadata.rs` holds `&'a [LocatedRosterAthlete]`. `athletes()` returns that exact borrowed slice; `next` slices the original vector as before. The byte-budget calculation borrows each payload and preserves the current canonical-projection accounting and 64-row/window boundary. No rebasing or rewriting row ordinals/offsets.
6. Make `Records<'a>` retain `members: &'a [LocatedRosterAthlete]` obtained directly from `Window::athletes`, alongside its existing owned canonical outputs/observations. Add `members(&self) -> &'a [LocatedRosterAthlete]`. `Records::of(window: &Window<'a>, ...)` must preserve that borrow. This is one borrowed input slice, not a duplicated per-output provenance ledger. Teams/schools can aggregate many input members; do not invent a 1:1 provenance claim for those outputs.
7. Migrate `Counts::include` to the located slice and inspect payload year/gender through `athlete()`. In actual `apply_roster`, use `records.members()` for counting and checked cursor advancement. This makes the retained Records input an actual production consumer, not an unused/test-only field. Keep the existing terminal comparison against original roster length and all commit/await ordering.
8. Update internal `Records` type annotations in `journal.rs` and `commit.rs` with the needed lifetimes. `Records::stage` stages exactly the same five families and canonical values in the same order; it does not serialize members. `roster_journal`, `Journal`, `roster_digest`, `roster_operation`, `Commit::window` effect/receipt behavior and fetch/body persistence remain unchanged except unavoidable lifetime spelling.

## Exact implementation file scope

Production changes expected:

- `crates/census-crawl/src/milesplit/wire.rs`: wrapper, located Roster vector.
- `crates/census-crawl/src/milesplit.rs`: wrapper re-export.
- `crates/census-crawl/src/milesplit/parse/roster.rs`: retain computed positive locator.
- `crates/census-crawl/src/milesplit/normalize.rs`: located slice admission/projection.
- `crates/census-service/src/census/sweep/roster/metadata.rs`: located Window and payload byte accounting.
- `crates/census-service/src/census/sweep/roster/counts.rs`: count located members through payload.
- `crates/census-service/src/census/sweep/roster/journal.rs`: borrowed members on Records only; old Journal unchanged.
- `crates/census-service/src/census/sweep/roster.rs`: consume Records members in actual loop.
- `crates/census-service/src/census/sweep/roster/commit.rs`: Records lifetime annotations only if required.

No change expected to `milesplit/roster.rs`, `fetch.rs` or `parse/mod.rs`; inspect and compile those affected types/callers. New focused test module files may be added under the corresponding existing test module trees. Keep production files within 300 lines and callables within the existing hard 60 trusted logical lines; do not mechanically invent context objects.

Existing direct payload-access test migrations:

- `crates/census-crawl/src/milesplit/tests/mod.rs`: first-row fields, season accessor, name lookup, cohort filters and rejected-ID exclusion.
- `crates/census-crawl/src/milesplit/tests/roster_entities.rs`: parsed positive fields; canonical output assertions remain unchanged.
- `crates/census-crawl/src/milesplit/parse/roster/owner/tests.rs`: two first-athlete ID assertions.
- `crates/census-crawl/src/milesplit/parse/roster/owner/tests/contextual.rs`: first-athlete ID assertion.
- `crates/census-service/tests/milesplit_roster_observations.rs`: source-row payload map.
- `crates/census-service/tests/recovery_mod/jurisdiction_replay.rs`: cohort count from source rows.
- `crates/census-service/src/census/sweep/roster/tests/windows.rs`: exercise Records member accessor where production counting is mirrored; add or share focused Window/Records regressions.

Affected callers that should stay source-compatible with the exact located-slice choice (verify; avoid cosmetic edits):

- `crates/census-service/tests/parity_pipeline_mod/fixture_builders.rs`: parsed slice to roster_entities.
- `crates/census-crawl/examples/qualification_bound_projection/binding.rs`: empty slice and 64-member chunks to roster_entities.
- `crates/census-service/examples/qualification_native_vm/captures.rs`: 64-member chunks to roster_entities.
- `crates/census-service/examples/qualification_native_vm/record.rs`: parser verdict/count use.
- `crates/census-crawl/tests/boundary_points.rs`, `crates/census-service/benches/core/fixtures.rs`: length/count uses.
- `crates/census-service/tests/milesplit_roster_observations/{provenance,outcomes}.rs`: parse/verdict/capture boundary uses.
- `crates/census-crawl/src/milesplit/parse/roster/ownership_tests.rs`: verdict/emptiness uses.

The separate TFRRS `RosterAthlete` is unrelated and must not be migrated. Root should re-run a scoped symbol search after integration for newly introduced callers; this list is pinned to the reviewed HEAD.

## Acceptance mapping (proposed, NOT_RUN)

Use independent literal/source-string expectations rather than recomputing expected locators with the production row regex.

- **LP01, exact positive bytes:** a tiny synthetic roster with non-ASCII prefix and HTML entities yields the exact expected inner-row byte span and zero-based ordinal. Payload retains existing decoded name, provider ID, URL, grade-year and season results. Check checked `offset + length` and safe body slice against literal expected bytes.
- **LP02, partial order:** valid/rejected/valid rows produce positive ordinals 0 and 2 and rejected ordinal 1. Confirm no compaction, sorting, ID lookup or window-relative numbering.
- **LP03, duplicate payload:** two identical member payloads at different positions remain two located members with distinct offsets/ordinals. Existing canonical dedup/projection policy is unchanged.
- **LP04, bounded failure:** existing overlong-row/invalid-year/quarantined/capacity behavior and unfinished locator remain unchanged. Exercise the 20,000/20,001 boundary under the original body cap; no extra member is accepted and no unchecked arithmetic is introduced.
- **LP05, actual service Window→Records:** create more than one 64-member window, with a rejected row before the boundary and a multibyte prefix. Verify each Window's source members, Records' source members and original Roster slice contain the same exact pairs; later windows retain global ordinals. Use checked lengths and accessor-based payload assertions.
- **LP06, output preservation:** use existing known roster fixtures and independently pinned canonical serialized outputs/digests or literal expected fields to establish the same school/athlete/team/source-observation output and ordering. Preserve the existing published-grad-year/no-invented-grade test with different caller years. No locator enters canonical serialized records or identity derivation.
- **LP07, journal/receipt preservation:** assert exact existing serialized Journal field set/content, rejected/unfinished values, and expected fact/effect digest for a fixed deterministic input. The new Records members is not serialized. Compare against a pre-change or independently authored baseline, not two paths sharing a new serializer.
- **LP08, legacy replay regression:** retain `partial_roster_survives_reopen_and_corrected_body_adds_only_missing_facts`, `thousands_of_rows_resume_after_one_atomic_window_without_duplicate_facts`, resource/refusal and completion tests. Reopen must preserve existing canonical facts/counts/owed work. Do not assert positive locator readback from old Journal. If the same bytes are explicitly reparsed, label that operation truthfully.
- **LP09, ownership/caller compile:** existing published/final-owner and ownerless-fragment tests remain unchanged in outcome; all library, test, example and benchmark targets compile. No host/owner validation weakening is allowed to make wrapper tests pass.

No new positive authority checker exists in this gate, so proposal AB02's hypothetical authority-checker execution is not a valid test requirement here. Absence of acceptance follows from no domain/Store/report/API additions and unchanged old behavior; it must not be reported as a tested end-to-end refusal by a nonexistent checker.

Root runs appropriate checks only after integration through `tools/moon-local`; exact target filters should be chosen from the actual suite names. Candidate commands: `tools/moon-local run pipeline:fmt`, `tools/moon-local run pipeline:check`, `tools/moon-local run pipeline:lint-src`, and focused `tools/moon-local run pipeline:tests -- -E 'test(roster)'`, followed by the affected named integration suites and required broader gates. These are proposed, never executed by this reviewer. Passing a name filter alone does not establish that all integration modules ran.

## Resource and async invariants

The wrapper adds one fixed-size locator per already bounded positive row, at most 20,000 rows. It adds no heap-owned body/string, parallel vector, per-window clone or new dynamic collection. Existing fallible reserve remains. Window and Records borrow the same vector; the owned RosterOutcome remains alive for the service operation. Do not spawn or detach a future holding that borrow. The shell's effects/await order, cleanup and existing bounded streaming remain unchanged. No speed, allocation-free, ABI layout or end-to-end performance claim is made; later measurement and exact changed-source checks remain required.

## Stopping condition / durable boundary

Done when all actual in-memory callers preserve the payload-locator pair and the focused/existing compatibility checks pass on Root's exact integrated candidate. The result may be described as “positive roster locators preserved through parsing and the service projection boundary.” It may not be described as “durably archived positive members,” “reopenable exact interpretation,” “accepted dated affiliation,” “HS participation,” “selected Workbook support,” or “production lifecycle enabled.” Those remain separate blocked contracts.
