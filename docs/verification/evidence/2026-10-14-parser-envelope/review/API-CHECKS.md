# Compiler/API probes

Status: **prepared only; none executed**. Eight complete snippets are in `compile-probes/` with hashes in `compile-probe-hashes.json`. They are review artifacts, not production examples or new permanent test infrastructure. Root alone may stage them into owned live source or an authorized fully integrated isolated source tree and invoke Moon. Do not invoke rustc/Cargo directly, add doc comments, dependencies, production test features or authority escape hatches.

## Execution discipline

First obtain a clean integrated `tools/moon-local run pipeline:check` result. For each external probe (01–06), temporarily stage exactly one as `crates/census-crawl/examples/parser_envelope_api_probe.rs`, then run:

`timeout 1200 tools/moon-local run pipeline:check`

Record compiler output against the exact staged probe hash. 01 must compile. 02–06 must fail for the indicated named operation, not an unrelated baseline/tool/network failure. Cases contain multiple named functions; require a diagnostic for each expected forbidden expression, or split the expressions into separate temporary probes when compiler recovery omits any. Exact numeric error codes below are predictions, not observed output.

Remove only the temporary file Root created, preserving any pre-existing file and concurrent edits. Re-run the clean check after removal. Do not run source lint against deliberate negative probes or report their intended failures as production regressions.

For 07, stage as a temporary **sibling of the owner modules** at census-crawl crate root and add an ordinary temporary module declaration at the root under Root's exclusive ownership. Do not place it beneath net::acquisition or milesplit::owned::outcome; that would test owner privileges rather than sibling privacy. Run the same Moon check and require field/mint privacy errors, then remove only those temporary edits.

For 08, stage under `milesplit` and add an ordinary non-cfg(test) temporary module declaration in `milesplit.rs`. Run the same Moon check; the normal library compilation must reject the missing historical_fixture method (expected E0599). The test-library target enables cfg(test), so its existence there is expected. A visibility error from an external module alone does not prove that the fixture factory is absent from the production library. Capture the normal-library diagnostic specifically. Then remove temporary edits and recheck.

## Oracles

1. `01_external_allowed.rs`: ordinary external consumer can borrow capture/provenance/verdict, consume to raw FetchOutcome, and call the raw pure parser. Must compile. It creates no authority.
2. `02_external_recombination_rejected.rs`: both CapturedFetch destructure/reassembly with unrelated capture and OwnedMeetOutcome destructure/reassembly with unrelated verdict must fail private-field checks (normally E0451). This is the exact former escape, not merely an invented API name.
3. `03_external_mutation_rejected.rs`: capture().body mutation, interpreted capture().body mutation and parsed verdict row mutation must fail shared-reference mutability (normally E0596). Calling release_body outside MileSplit must fail method privacy (normally E0624).
4. `04_external_constructor_escape_rejected.rs`: into_parser_parts and historical raw wrapping are crate-private (normally E0624); historical_fixture is absent from a normal external dependency build (normally E0599). Nothing public can reconstruct retained authority after into_capture.
5. `05_external_trusted_traits_rejected.rs`: serde Deserialize, Default and From<FetchOutcome> are unavailable for each trusted envelope (normally E0277). FetchOutcome and raw parser types remain freely serializable/deserializable; that is intentionally not a trusted path.
6. `06_external_retained_state_rejected.rs`: CaptureProvenance itself is crate-private, so importing it externally must fail (normally E0603). The public CaptureProvenanceView is descriptive and intentionally remains constructible.
7. `07_internal_sibling_recombination_rejected.rs`: another census-crawl module cannot assign private CapturedFetch.capture or OwnedMeetOutcome.verdict (normally E0616) and cannot call CaptureProvenance::retained (normally E0624). This distinguishes module-private sealing from merely pub(crate) fields.
8. `08_milesplit_production_fixture_rejected.rs`: even a production MileSplit module cannot invoke the synthetic factory because cfg(test) removes it. Normal-library E0599 is required.

## Static checks that complement compiler probes

- Trusted type definitions have no public/crate-public fields, mutable borrow APIs, mutable dereference, deserialization, default or retained from-parts constructor.
- Only the two owner-local calls use into_parser_parts; no accepted builder takes CaptureProvenanceView or opaque provenance plus arbitrary raw capture.
- Actual retained construction is limited to verified read/session producers. Compiler privacy alone does not prove byte verification or same-response provenance.
- The factory is exactly cfg(test), without feature escape. Historical test verdicts do not claim retained evidence.
- A released interpretation never returns CapturedFetch. Erased FetchOutcome is untrusted even if its digest/date remain intact.

The variant-only context guard intentionally does not reject retained A plus arbitrary internal DurableV1 B. Root adopted that limitation; exact owner/work binding was established only by the current production acquisition caller trace in REVIEW.md. Do not add an assertion or claim that the context guard performs exact owner equality.
