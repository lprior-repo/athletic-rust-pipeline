
use serde_json::Value;

use super::value_u64;
use census_domain::model::{CentiMetres, CentiSeconds};
use census_domain::model::{EventKind, Mark};

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

pub(super) fn canonical_mark(kind: &EventKind, mark_int: &Value) -> Option<Mark> {
    let micros = value_u64(mark_int)?;
    if micros == 0 {
        return None;
    }
    if kind.is_field() {
        let cm = (micros.checked_add(5_000)? / 10_000).try_into().ok()?;
        Some(Mark::DistanceMetres(CentiMetres::new(cm)))
    } else {
        let cs = (micros.checked_add(5)? / 10).try_into().ok()?;
        Some(Mark::TimeSeconds(CentiSeconds::new(cs)))
    }
}
