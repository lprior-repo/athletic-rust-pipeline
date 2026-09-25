//! The mark rules: how a row's published column and the platform's integer channel become a mark.
//!
//! Part of the reader contract in [`super`], whose `# Marks` section documents the channels and the
//! measured agreement between them; the two readers that use these rules are `documents::record_row`
//! (event documents) and `standings::StandingRow::canonical_mark`.

use serde_json::Value;

use super::value_u64;
use census_domain::model::{CentiMetres, CentiSeconds};
use census_domain::model::{EventKind, Mark};

/// The canonical mark of one row from both published channels.
///
/// Field marks take the published notation first — `5-02.00` becomes the imperial mark the other
/// adapters also mint — and every other kind takes the integer channel, which is the exact value
/// the display column rounds. The two agree on every captured row (a fixture asserts that).
pub(super) fn row_mark(
    kind: &EventKind,
    published: Option<&str>,
    mark_int: Option<&Value>,
) -> Option<Mark> {
    if kind.is_field() {
        if let Some(mark) = published.and_then(crate::hytek::parse_field_mark) {
            return Some(mark);
        }
    }
    canonical_mark(kind, mark_int?)
}

/// The canonical mark of an integer channel: milliseconds for times, micrometres for field marks.
pub(super) fn canonical_mark(kind: &EventKind, mark_int: &Value) -> Option<Mark> {
    let micros = value_u64(mark_int)?;
    if micros == 0 {
        return None;
    }
    if kind.is_field() {
        // micrometres → centimetres (÷10,000); overflow on absurd inputs returns None
        let cm = (micros.checked_add(5_000)? / 10_000).try_into().ok()?;
        Some(Mark::DistanceMetres(CentiMetres::new(cm)))
    } else {
        // milliseconds → centiseconds (÷10); overflow on absurd inputs returns None
        let cs = (micros.checked_add(5)? / 10).try_into().ok()?;
        Some(Mark::TimeSeconds(CentiSeconds::new(cs)))
    }
}
