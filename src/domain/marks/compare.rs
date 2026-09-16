use super::{Comparison, MarkValue};
use std::cmp::Ordering;

pub(super) fn compare_values(left: MarkValue, right: MarkValue, lower: bool) -> Comparison {
    let ordering = match (left, right) {
        (MarkValue::Time(a), MarkValue::Time(b)) => a.cmp(&b),
        (MarkValue::Distance(a), MarkValue::Distance(b)) => a.cmp(&b),
        (MarkValue::Points(a), MarkValue::Points(b)) => a.cmp(&b),
        (MarkValue::Count(a), MarkValue::Count(b)) => a.cmp(&b),
        (MarkValue::Time(_), MarkValue::Distance(_))
        | (MarkValue::Time(_), MarkValue::Points(_))
        | (MarkValue::Time(_), MarkValue::Count(_))
        | (MarkValue::Distance(_), MarkValue::Time(_))
        | (MarkValue::Distance(_), MarkValue::Points(_))
        | (MarkValue::Distance(_), MarkValue::Count(_))
        | (MarkValue::Points(_), MarkValue::Time(_))
        | (MarkValue::Points(_), MarkValue::Distance(_))
        | (MarkValue::Points(_), MarkValue::Count(_))
        | (MarkValue::Count(_), MarkValue::Time(_))
        | (MarkValue::Count(_), MarkValue::Distance(_))
        | (MarkValue::Count(_), MarkValue::Points(_)) => return Comparison::IncompatibleEvent,
    };
    match (lower, ordering) {
        (true, Ordering::Less) | (false, Ordering::Greater) => Comparison::Better,
        (_, Ordering::Equal) => Comparison::Equal,
        _ => Comparison::Worse,
    }
}
