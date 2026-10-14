# Independently adoptable parser-envelope repair

Status: **PROPOSED API CONTRACT ONLY.** No live edits or compiler/test execution. This closes the named transport-to-owned-parser envelope without adopting structural open, Prepared recovery or a new persisted version.

## Recommended minimum cohesive change

Seal both `CapturedFetch` and `OwnedMeetOutcome`, not merely the provenance token. Preserve their legitimate transport/parser behavior. A public or crate-wide builder taking `FetchOutcome + Retained refs`, or `FetchOutcome + caller-supplied parsed verdict`, would recreate the gap.

### A. Owning transport envelope

Keep `CapturedFetch` owned by `net::acquisition`. Its fields become module-private, not `pub(crate)`. Retained construction occurs only inside that owner and its verified read/session children. No `Default`, `Deserialize`, public struct literal, retained `From<FetchOutcome>`, or public/crate-wide retained `from_parts`.

Expose:

- `capture(&self) -> &FetchOutcome` for immutable byte/metadata reads.
- `provenance(&self) -> CaptureProvenanceView<'_>` for a read-only description: HistoricalAttemptUnknown or Retained with borrowed occurrence/original references. The view may be copied or constructed, but no accepted constructor takes it. It is not an authority token.
- `into_capture(self) -> FetchOutcome` only as an explicitly proof-erasing consuming operation for resource/error retention and existing historical fixture extraction. Its returned raw value cannot be reattached to retained authority.
- A narrowly internal consuming decomposition if the parser owner requires it: crate-private only, not public, returning the exact owned capture and opaque private-state provenance. No corresponding builder can accept those parts outside the transport/parser owners. Prefer direct owner-to-parser consumption over a generic reusable parts API.

Replace the publicly constructible retained provenance representation with an opaque private-state representation. The public view above replaces pattern matching; do not export a retained-state constructor. Only retained owner code can mint it. An explicit crate-private historical constructor may accept `FetchOutcome`, because it grants no durable authority; the legacy adapter must still call `require_legacy_structured_capture` first.

Private fields are necessary API protection, not cryptographic proof. The constructor's authority comes from the verified producer path below. Do not claim `pub(crate)` alone makes an unverified hash or path trusted.

### B. Owning interpreted outcome

Move construction of `OwnedMeetOutcome` into its `milesplit::owned` owner. Make capture, provenance and verdict fields private. The production entry consumes the intact `CapturedFetch` plus the existing parsed meet identity and expected capture context, and derives its verdict by parsing that envelope's own immutable body. It must reject incompatible historical/durable context before any parse/interpretation journal write. Do not accept a caller-supplied verdict or separately supplied retained refs.

Both `fetch_owned_meet` and `read_owned_meet` use this one interpretation constructor so their present parsed/malformed/refused/resource behavior stays aligned. The public raw `parse_owned_meet(&[u8], meet_id)` remains a pure parser for untrusted documents; its returned `OwnedMeetPage` cannot be promoted to an admitted durable `OwnedMeetOutcome` through a from-parts API.

Resource exhaustion must return the original captured envelope in a typed resource outcome, for example a private `OwnedInterpretationOutcome::Resource { capture: CapturedFetch, failure: OwnedParseResourceFailure }`, so the caller may consume it into raw error-retention metadata. It must not clone a body or lose the only original capture on the error branch. This private return shape is not a new journal or externally serialized protocol.

Expose:

- `capture(&self) -> &FetchOutcome`, `verdict(&self) -> &OwnedMeetVerdict`, and `provenance(&self) -> CaptureProvenanceView<'_>` for current projection/receipt readers.
- `into_capture(self) -> FetchOutcome` for an interpretation-recording resource error; it discards the interpretation/provenance authority.
- A module-owned, restricted `release_body(&mut self)` operation for the current post-parse buffer-release boundary; no general mutable capture/verdict getter, no mutable deref and no returned mutable vector. The type's guarantee is “this verdict was derived from these checked bytes/metadata,” not “a currently resident body is always present.” Releasing resident bytes cannot yield a `CapturedFetch` or a byte-verification witness.

An implementation may instead express resident/released interpreted states explicitly if needed, but must not keep an empty-body value advertised as a complete verified transport envelope. The simpler existing `OwnedMeetOutcome` may retain its current metadata-only-after-release semantics as long as it is a sealed **interpretation** and has no conversion back into a verified input. Keep `bytes`, digest, date and source context unchanged when releasing body.

### C. Acceptance entry

`AcquiredMeet::new` continues to accept only the sealed `OwnedMeetOutcome`. `require_capture_context` becomes a check over its borrowed provenance view, or the owner performs the same check before interpretation and the downstream check remains defensive. It is not described as a second SHA256 verification.

The factual checks still come from owning transport constructors and exact-source parsing. No `Into<OwnedMeetOutcome>` from raw `FetchOutcome`, `OwnedMeetPage`, `OwnedCaptureRetention`, `VerifiedOwnedAcquisition` or a resource receipt is introduced. The older verified-owned family needs its own explicit future compatibility boundary if adopted for offline reparse; it is not silently upgraded to attempt provenance.

## Actual verified producer closure to preserve

1. Retained replay: `net/acquisition/read.rs::outcome` consumes Store's strict `VerifiedRetainedOccurrence`, reads bounded bytes and calls `Capture::verify_body` on the actual owned Vec, then mints retained envelope. Preserve that order.
2. Fresh response: `net/execute/cache_writer.rs::record_response` passes `&body_vec` and canonical metadata to `CaptureSession::retain`; the latter verifies that memory, writes the archive and finishes the exact Prepared terminal. The former then moves **that same body_vec and metadata** into `FetchOutcome`. `get_acquisition` combines it with `Stage::Complete` only on success. There is no intervening body replacement in this actual path. Keep the construction inside this owning boundary; do not expose an arbitrary `finish(session_token, unrelated_capture)` to callers.
3. 304: `PinnedEligibleCache::compare_body` hands the exact hash-checked Vec to `replay_not_modified`, which verifies/settles the direct original before returning it and Complete provenance. Keep the same original/date and distinct 304 occurrence.
4. Prepared: `recover_prepared` finishes the exact original, strictly rereads, calls the same actual-byte outcome constructor, then republishes eligible cache under its fence. No local refetch.
5. Legacy: `milesplit/fetch.rs` still requires legacy structured capture permission and uses only the explicit historical constructor. Historical does not become durable merely because it has a digest or was archived by an older cache path.

This source proof is narrower than proof of every `Fetcher` surface. It closes the actual producers of this type; unsupported browser lanes remain refused for adopted sessions in `net/execute/fetch.rs`.

## Exact source caller migration

Substantive ownership changes:

- `net/acquisition.rs`: private envelope/provenance state and read/consume API; existing owned acquisition success construction.
- `net/acquisition/read.rs`: retained checked-byte constructor and accessors in recovery.
- `net/acquisition/retention.rs`, `session.rs`: private completion provenance minting, no externally attachable token.
- `milesplit/owned/mod.rs` (prefer a focused private outcome module if needed for the 300-line ceiling): sealed interpreted outcome and owning interpretation/resource transition.
- `milesplit/fetch.rs`, `milesplit/owned/effect.rs`: use the one constructor, perform context admission before parse/effect recording, consume raw capture only for errors.
- `milesplit/results/run/acquired.rs`: immutable accessors, restricted release, provenance view.
- `net/mod.rs`: re-export only the intended envelope/view API; no raw retained-state constructor.

Mechanical consumers found from direct field references:

- `milesplit/owned/effect/interpretation.rs` reads capture/verdict for existing JSON; preserve serialized values and keys.
- `milesplit/results/run.rs`, `run/receipt.rs`, `run/resource.rs`, `run/projection.rs`, `run/windows.rs`, `run/windows/{admission,completion,journal}.rs` borrow the same source metadata/verdict and invoke restricted release.
- `net/acquisition/tests/fixture.rs` extracts borrowed occurrence via provenance view; `tests/cache.rs` checks the 304/original pair. They do not need a fabricated durable constructor.

No source writer, parser policy, canonical identity rule, public source family, retained reader or workbook implementation is added by this slice.

## Actual fixture callers and migration rules

The targeted named-type search found no external-crate `CapturedFetch` or `OwnedMeetOutcome` struct-literal consumer. All current synthetic `OwnedMeetOutcome` fixtures use historical provenance:

- `milesplit/map/owned/tests.rs`: builds a parsed pure projection fixture.
- `milesplit/owned/effect/tests.rs`: builds a parsed resource/recording fixture.
- `milesplit/owned/effect/acquisition_partial_tests.rs`: builds a historical partial outcome, then mutates from-cache/date/final-URL metadata to test replay and new occurrence semantics.
- `milesplit/owned/effect/tests/parser_revisions.rs`: deliberately changes the parsed historical event kind to seed an old interpretation.
- `milesplit/results/run/receipt/tests.rs`: two historical fixture outcomes for receipt/complete-count behavior.

Use a restricted `#[cfg(test)]` historical-only fixture factory where deliberately fabricated historical verdicts are required. It must be unavailable in production builds and cannot select retained provenance. For date/from-cache/final-URL variants, create distinct raw historical captures **before** factory construction rather than adding a general mutable accessor. Historical parser-revision fixtures must remain visibly synthetic; do not route them through a forged durable witness.

Actual durable tests in `net/acquisition/tests.rs`, `tests/{cache,recovery,failures}.rs`, and `milesplit/results/run/owned_tests/durable_capture.rs` already acquire through real local transport/store fixtures. Keep that route and update only inspection syntax. `run/owned_tests/bounded_buffers.rs` and durable tests must still prove body release; getter syntax cannot cause a second resident clone.

## Persistence and compatibility

These runtime types currently have no serde serialization contract. `FetchOutcome` does have one; its body is skipped, and fields including actual capture date and optional response URL must remain unchanged. Keep all archive metadata, source journals, manifests, terminal/result/receipt bytes, normalized facts and frozen policy/schema identities identical for equivalent current input.

This is an intentional Rust source API tightening: direct field access and enum matching migrate to accessors/views in the same slice. It requires normal compilation and exact focused regression evidence before adoption. It does not require a store migration or a new capture/frozen protocol and does not grant structural-open or publication authority.

## Remaining limits

- Opaque tokens alone do not protect an outcome with public capture/verdict setters; do not split this repair and claim the first half establishes verified input.
- An immutable borrow is only safe while it cannot return mutable internal state; raw cloned/deserialized `FetchOutcome` remains untrusted.
- A valid envelope does not certify provider ownership, full meet completeness, canonical identity, cohort eligibility or publication; existing parser/projection rules still apply.
- Source backing may change after parsing. The resulting interpreted outcome retains evidence of what it parsed, not a future guarantee that the backing still exists. New publication needs its own evidence-use contract from the main review.

## Integration-test boundary clarification

The five literal-fixture families above are **unit modules under `src`**, not external integration test crates. A targeted search of existing `crates/census-crawl/tests` and `crates/census-service/tests` found no `CapturedFetch`, `CaptureProvenance`, `OwnedMeetOutcome`, `fetch_owned_meet` or `read_owned_meet` references. There is no `crates/census-report/tests` directory. This is a named-interface negative finding, not a claim about all future tests.

A dependency library is built without `cfg(test)` for an external integration-test crate. Therefore the restricted historical fixture factory is not a proposed integration-test API, and no production feature may turn it on. External integration tests must use the public real offline/loopback legacy Fetcher plus legacy adapter context for historical behavior, or real qualified transport/store fixtures for retained behavior. Pure parser fixtures can call `parse_owned_meet` and remain synthetic raw parser output. Do not export a public from-parts builder or test-support retained token to make an integration test compile.

Concrete visibility rules: CapturedFetch fields and the stored retained-provenance variant are private to `net::acquisition`; the historical envelope constructor alone may be `pub(crate)`. OwnedMeetOutcome fields are private to `milesplit::owned`; its production interpreter may be `pub(in crate::milesplit)` and accepts the intact CapturedFetch, never parts or a caller verdict. Historical synthetic outcome factory is `#[cfg(test)] pub(in crate::milesplit)` and forces historical unknown. Immutable getters/views may be public; proof-erasing `into_capture` may be public because no accepted builder takes its raw result. If the interpreter needs an owned decomposition to avoid cloning, keep it crate-private and provide **no** inverse constructor outside the two owners. No `AsMut`, `DerefMut`, mutable getter, general state setter, deserializable trusted type, or public retained builder is added.
