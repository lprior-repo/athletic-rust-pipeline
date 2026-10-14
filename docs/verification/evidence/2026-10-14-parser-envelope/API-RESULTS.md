# Executed parser-envelope API checks

All eight reviewed probes executed through the existing Moon check entrypoint. Probe 01 compiled; probes 02–08 failed for every named intended forbidden operation. Expected negative compiler exits are preserved as outer exit 1 and internal Cargo exit 101; they are successful negative API checks, not successful normal compilation. [API-PROBE-STATUS.json](API-PROBE-STATUS.json) records exact probe hashes, commands, log hashes and line/diagnostic mappings.

| Probe | Actual outcome | Observed boundary |
|---|---|---|
| 01 external allowed | Compile success, exit 0 | Immutable reads, consuming raw erasure and pure raw parser remain available |
| 02 recombination | Four E0451 field sites plus opaque-type privacy diagnostics | CapturedFetch and OwnedMeetOutcome destructuring/reassembly denied |
| 03 mutation | Three E0596 sites and E0624 | Both capture-body mutations and parsed rows denied; release is owner-restricted |
| 04 constructors | Two E0624 and E0599 | Parser parts/historical wrapper private; production synthetic factory absent |
| 05 trusted traits | Six E0277 sites | Both envelopes reject Deserialize, Default and From<FetchOutcome> |
| 06 retained state | E0603 | Opaque retained provenance type unavailable externally |
| 07 internal sibling | Two E0616 and E0624 in normal library | Another crate module cannot assign capture/verdict or mint retained provenance |
| 08 production MileSplit | E0599 in normal library | Synthetic historical factory absent even inside production MileSplit |

The assembler reconciled all 22 named negative expression sites against compiler diagnostic locations. Extra opaque-type errors in probe 02 are retained rather than hidden. Root receipts and preserved source-hash files match the original reviewed snippets. The raw normal-library failure for probe 08 establishes cfg(test) absence, rather than merely external visibility.

Temporary examples and module registrations are removed; `lib.rs` and `milesplit.rs` match the committed bytes. An independent read found a clean working tree and no probe paths. The post-probe check and format commands passed. [Cleanup receipt](API-CLEANUP-RECEIPT.json) records exact paths and outcomes.

These are Rust source API/visibility results. They do not prove arbitrary same-variant context identity, historical-all-stores compatibility, full source correctness or release readiness. The 38-path production source snapshot excludes all temporary probes.
