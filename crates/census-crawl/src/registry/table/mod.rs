//! The registry table: one entry per adapter that fetches or parses external source material.
//!
//! A module is absent on purpose when it is machinery rather than a provider. `compiled`, `hytek`,
//! `raceday` and `xc` parse result-file bytes that `wiaa_results` fetched and dispatched to them
//! (`wiaa_results::parse`, `run_artifacts`), and `result_file` holds the shared shapes they return;
//! `mod.rs` holds the adapter context. None of them issues a request or names an origin, so an entry
//! here would declare a provider that does not exist.
//!
//! Evidence discipline: the comment above each entry names the symbol its `true` capabilities rest
//! on, in the adapter's own file, so a reviewer can check the claim without reading the adapter end
//! to end. A capability that cannot be pointed at is not claimed.

use super::SourceDescriptor;

mod from_mshsl;
mod through_milesplit;

use from_mshsl::FROM_MSHSL;
use through_milesplit::THROUGH_MILESPLIT;

/// Every adapter that fetches or parses external source material, in slug order.
///
/// Multiple adapters behind one origin each get their own entry, and each repeats the origin's
/// policy: that repetition is what makes a budget violation visible, because two entries for one host
/// can be compared against the one rate the host permits.
///
/// The list is sharded across `through_milesplit` and `from_mshsl`; this is the one place the halves
/// are spliced, so a caller cannot see one half without the other.
pub fn descriptors() -> impl Iterator<Item = &'static SourceDescriptor> {
    THROUGH_MILESPLIT.iter().chain(FROM_MSHSL.iter())
}
