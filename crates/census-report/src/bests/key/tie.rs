#[derive(Debug, Clone, Copy)]
pub struct MarkOrdering<'a> {
    pub date: &'a str,
    pub meet: &'a str,
    pub performance_id: &'a str,
}

impl<'a> MarkOrdering<'a> {
    pub const fn new(date: &'a str, meet: &'a str, performance_id: &'a str) -> Self {
        Self {
            date,
            meet,
            performance_id,
        }
    }
}

pub fn tie_break_later(candidate: MarkOrdering<'_>, incumbent: MarkOrdering<'_>) -> bool {
    let has_cand = !candidate.date.is_empty();
    let has_inc = !incumbent.date.is_empty();

    match (has_cand, has_inc) {
        (true, false) => return true,
        (false, true) => return false,
        _ => {}
    }

    candidate
        .date
        .cmp(incumbent.date)
        .then_with(|| candidate.meet.cmp(incumbent.meet))
        .then_with(|| candidate.performance_id.cmp(incumbent.performance_id))
        .is_ge()
}

pub(crate) fn should_replace_impl(
    candidate_value: i64,
    incumbent_value: i64,
    candidate: MarkOrdering<'_>,
    incumbent: MarkOrdering<'_>,
    is_better: impl Fn(i64, i64) -> bool,
) -> bool {
    if is_better(candidate_value, incumbent_value) {
        return true;
    }

    if candidate_value == incumbent_value {
        return tie_break_later(candidate, incumbent);
    }

    false
}
