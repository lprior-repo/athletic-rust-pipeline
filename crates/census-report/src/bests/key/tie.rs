pub fn tie_break_later(
    cand_date: &str,
    cand_meet: &str,
    cand_perf_id: &str,
    inc_date: &str,
    inc_meet: &str,
    inc_perf_id: &str,
) -> bool {
    let has_cand = !cand_date.is_empty();
    let has_inc = !inc_date.is_empty();

    match (has_cand, has_inc) {
        (true, false) => return true,
        (false, true) => return false,
        _ => {}
    }

    cand_date
        .cmp(inc_date)
        .then_with(|| cand_meet.cmp(inc_meet))
        .then_with(|| cand_perf_id.cmp(inc_perf_id))
        .is_ge()
}

pub(crate) fn should_replace_impl(
    candidate_value: i64,
    incumbent_value: i64,
    cand_date: &str,
    cand_meet: &str,
    cand_perf_id: &str,
    inc_date: &str,
    inc_meet: &str,
    inc_perf_id: &str,
    is_better: impl Fn(i64, i64) -> bool,
) -> bool {
    if is_better(candidate_value, incumbent_value) {
        return true;
    }

    if candidate_value == incumbent_value {
        return tie_break_later(
            cand_date,
            cand_meet,
            cand_perf_id,
            inc_date,
            inc_meet,
            inc_perf_id,
        );
    }

    false
}
