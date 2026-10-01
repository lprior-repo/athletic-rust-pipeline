use crate::hytek::{parse_field_mark, parse_time};
use census_domain::model::{CentiMetres, CentiSeconds, Mark, TimingMethod};

pub fn parse_published_time(value: &str) -> Option<(CentiSeconds, Option<TimingMethod>)> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let (numeric, timing) = if let Some(numeric) = value.strip_suffix(['a', 'A']) {
        (numeric, Some(TimingMethod::Fat))
    } else if let Some(numeric) = value.strip_suffix(['h', 'H']) {
        (numeric, Some(TimingMethod::Hand))
    } else {
        (value, None)
    };
    parse_time(numeric).map(|time| (time, timing))
}

pub fn parse_published_metric_distance(value: &str) -> Option<CentiMetres> {
    let metres = value.trim().strip_suffix(['m', 'M'])?.trim();
    let (whole, fraction) = metres.split_once('.').unwrap_or((metres, ""));
    let fraction = fraction.trim_end_matches('0');
    if (whole.is_empty() && fraction.is_empty())
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    match parse_field_mark(metres)? {
        Mark::DistanceMetres(distance) => Some(distance),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_declared_metric_marks_preserve_exact_centimetres() {
        for (raw, expected) in [("3.05m", 305), ("9.47m", 947), (" 1.350M ", 135)] {
            assert_eq!(
                parse_published_metric_distance(raw),
                Some(CentiMetres::new(expected))
            );
        }
        for raw in [
            "NH", "3.057m", "3.05mm", "3.05", "-0.25m", "NaNm", "1e2m", ".m",
        ] {
            assert_eq!(parse_published_metric_distance(raw), None, "{raw}");
        }
    }

    #[test]
    fn parse_published_fat_timing() {
        let (time, method) = parse_published_time("24.95a").unwrap();
        assert_eq!(time, CentiSeconds::new(2495));
        assert_eq!(method, Some(TimingMethod::Fat));

        let (time, method) = parse_published_time("11.52a").unwrap();
        assert_eq!(time, CentiSeconds::new(1152));
        assert_eq!(method, Some(TimingMethod::Fat));

        let (time, method) = parse_published_time("3:00.64a").unwrap();
        assert_eq!(time, CentiSeconds::new(18064));
        assert_eq!(method, Some(TimingMethod::Fat));

        let (time, method) = parse_published_time("24.95A").unwrap();
        assert_eq!(time, CentiSeconds::new(2495));
        assert_eq!(method, Some(TimingMethod::Fat));
    }

    #[test]
    fn parse_published_hand_timing() {
        let (time, method) = parse_published_time("11.32h").unwrap();
        assert_eq!(time, CentiSeconds::new(1132));
        assert_eq!(method, Some(TimingMethod::Hand));

        let (time, method) = parse_published_time("11.32H").unwrap();
        assert_eq!(time, CentiSeconds::new(1132));
        assert_eq!(method, Some(TimingMethod::Hand));
    }

    #[test]
    fn parse_published_plain_numeric() {
        let (time, method) = parse_published_time("11.32").unwrap();
        assert_eq!(time, CentiSeconds::new(1132));
        assert_eq!(method, None);

        let (time, method) = parse_published_time("3:00.64").unwrap();
        assert_eq!(time, CentiSeconds::new(18064));
        assert_eq!(method, None);
    }

    #[test]
    fn parse_published_rejects_unknown_suffix() {
        assert!(parse_published_time("24.95x").is_none());
    }

    #[test]
    fn parse_published_rejects_multiple_suffixes() {
        assert!(parse_published_time("24.95aa").is_none());
        assert!(parse_published_time("24.95ah").is_none());
        assert!(parse_published_time("24.95ha").is_none());
    }

    #[test]
    fn parse_published_rejects_non_result_tokens() {
        assert!(parse_published_time("DNS").is_none());
        assert!(parse_published_time("DNF").is_none());
        assert!(parse_published_time("Scr").is_none());
        assert!(parse_published_time("NH").is_none());
        assert!(parse_published_time("NT").is_none());
        assert!(parse_published_time("Foul").is_none());
    }

    #[test]
    fn parse_published_rejects_negative_and_non_finite() {
        assert!(parse_published_time("-5.0").is_none());
        assert!(parse_published_time("inf").is_none());
        assert!(parse_published_time("nan").is_none());
    }

    #[test]
    fn parse_published_rejects_malformed_minute_field() {
        assert!(parse_published_time("1:60:00.00").is_none());
        assert!(parse_published_time("1:60.00a").is_none());
    }

    #[test]
    fn parse_published_trims_whitespace() {
        let (time, method) = parse_published_time(" 11.32 ").unwrap();
        assert_eq!(time, CentiSeconds::new(1132));
        assert_eq!(method, None);

        let (time, method) = parse_published_time(" 24.95a ").unwrap();
        assert_eq!(time, CentiSeconds::new(2495));
        assert_eq!(method, Some(TimingMethod::Fat));
    }

    #[test]
    fn parse_published_rejects_empty_string() {
        assert!(parse_published_time("").is_none());
        assert!(parse_published_time("   ").is_none());
    }

    #[test]
    fn captured_class_of_2027_result_keeps_its_numeric_mark_and_timing() {
        let html =
            include_str!("../../tests/fixtures/milesplit/dc_meet_735841_results_legacy.html");
        let page =
            crate::milesplit::parse_raw(html, "https://dc.milesplit.com/meets/735841/results")
                .expect("captured public results");
        let row = page
            .meet
            .events
            .iter()
            .filter(|event| event.kind == census_domain::model::EventKind::Track100m)
            .flat_map(|event| &event.rows)
            .find(|row| row.name == "Brett Paukstis")
            .expect("published grade-11 100m result");
        assert_eq!(
            row.mark,
            census_domain::model::Mark::TimeSeconds(CentiSeconds::new(1267))
        );
        assert_eq!(row.timing, Some(TimingMethod::Fat));
    }
}
