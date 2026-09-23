//! The census counters and the domain conversions their comparisons run in.
//!
//! Every counter in this report is a count of things that exist in memory at that moment, so the
//! saturating helpers here replace the wrap-around casts the port inherited without changing any
//! value a real corpus can produce.

use indexmap::IndexMap;

/// Add `by` to a counter slot, saturating instead of wrapping.
///
/// Every counter here is a census of things that exist in memory at that moment (pages, rows, links
/// of the retained fixtures), so saturation is unreachable for a real corpus; the wrap-around it
/// replaces is not a value any caller of this report can use.
pub(crate) fn bump(slot: &mut usize, by: usize) {
    *slot = slot.saturating_add(by);
}

/// Add `by` to the named tally, saturating instead of wrapping.
pub(crate) fn tally(map: &mut IndexMap<String, usize>, key: &str, by: usize) {
    bump(map.entry(key.to_string()).or_insert(0), by);
}

/// The envelope's `count` — an `i64` as the site reports it — read in the `usize` domain the page's
/// rows are measured in.
///
/// A negative `i64` is not a row count; it saturates to `usize::MAX` so it still compares as larger
/// than any page, which is the branch outcome the pre-repair cast produced for every negative value.
pub(crate) fn count_len(count: i64) -> usize {
    usize::try_from(count).unwrap_or(usize::MAX)
}

/// A `usize` length — a row or candidate count — read in the envelope's `i64` domain.
///
/// A length above `i64::MAX` would need that many live elements; saturating to `i64::MAX` keeps the
/// comparison against an envelope count total rather than wrapping into a negative.
pub(crate) fn len_count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}
