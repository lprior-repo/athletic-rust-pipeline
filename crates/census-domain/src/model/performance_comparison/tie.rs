use chrono::NaiveDate;

#[derive(Debug, Clone, Copy)]
pub struct MarkOrdering<'a> {
    pub date: &'a str,
    pub performance_id: &'a str,
}

impl<'a> MarkOrdering<'a> {
    pub const fn new(date: &'a str, performance_id: &'a str) -> Self {
        Self {
            date,
            performance_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PerformanceDate {
    Known(NaiveDate),
    Unknown,
}

impl PerformanceDate {
    fn of(date: &str) -> Self {
        let exact_shape = date.len() == 10
            && date.bytes().enumerate().all(|(index, byte)| {
                if matches!(index, 4 | 7) {
                    byte == b'-'
                } else {
                    byte.is_ascii_digit()
                }
            });
        if !exact_shape {
            return Self::Unknown;
        }
        match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(date) => Self::Known(date),
            Err(_) => Self::Unknown,
        }
    }
}

pub fn tie_break_preferred(candidate: MarkOrdering<'_>, incumbent: MarkOrdering<'_>) -> bool {
    PerformanceDate::of(candidate.date)
        .cmp(&PerformanceDate::of(incumbent.date))
        .then_with(|| candidate.performance_id.cmp(incumbent.performance_id))
        .is_lt()
}

pub fn should_replace(
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
        return tie_break_preferred(candidate, incumbent);
    }
    false
}
