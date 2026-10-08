use super::{number_of, parse_mark, seam_config};
use census_domain::model::{EventKind, ExactSeconds, Mark};
use proptest::prelude::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn render_centis(centis: u64) -> (String, f64) {
    let total = centis as f64 / 100.0;
    let hours = centis / 360_000;
    let minutes = (centis / 6_000) % 60;
    let seconds = (centis % 6_000) as f64 / 100.0;
    let text = if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:05.2}")
    } else if minutes > 0 {
        format!("{minutes}:{seconds:05.2}")
    } else {
        format!("{seconds:.2}")
    };
    (text, total)
}

const RUN: EventKind = EventKind::CrossCountry;

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn a_published_time_reads_as_the_duration_it_prints(centis in 1u64..36_000_000) {
        let (text, expected) = render_centis(centis);
        let (mark, auto) = parse_mark(&RUN, &text)
            .ok_or_else(|| TestCaseError::fail(format!("{text} (from {centis}) was refused")))?;
        prop_assert!(!auto, "a bare mark is not marked automatic: {text}");
        prop_assert_eq!(mark, Mark::TimeSeconds(ExactSeconds::parse(&expected.to_string()).map_err(|error| TestCaseError::fail(error.to_string()))?));
    }

    #[test]
    fn an_automatic_suffix_marks_the_flag_not_the_mark(centis in 1u64..36_000_000) {
        let (text, expected) = render_centis(centis);
        let (mark, auto) = parse_mark(&RUN, &format!("{text}a"))
            .ok_or_else(|| TestCaseError::fail(format!("{text}a was refused")))?;
        prop_assert!(auto, "the `a` suffix is the automatic flag: {text}a");
        prop_assert_eq!(mark, Mark::TimeSeconds(ExactSeconds::parse(&expected.to_string()).map_err(|error| TestCaseError::fail(error.to_string()))?));
    }
}

#[test]
fn the_notation_table_holds_and_defers_to_one_time_parser() -> TestResult {
    for (text, expected) in [
        ("10.56", 10.56),
        ("1:54.32", 114.32),
        ("15:32.1", 932.1),
        ("1:05:12.34", 3912.34),
    ] {
        let (mark, _) = parse_mark(&RUN, text).ok_or_else(|| format!("{text} was refused"))?;
        check!(eq;
            mark,
            Mark::TimeSeconds(
                ExactSeconds::parse(&expected.to_string())?
            ),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn a_qualifier_is_stripped_rather_than_read_as_part_of_the_mark() -> TestResult {
    for (text, auto) in [
        ("12.34", false),
        ("12.34q", false),
        ("12.34Q", false),
        ("12.34p", false),
        ("12.34P", false),
        ("12.34a", true),
        ("12.34A", true),
    ] {
        let (mark, got_auto) = parse_mark(&RUN, text).ok_or_else(|| format!("{text} refused"))?;
        check!(eq;
            mark,
            Mark::TimeSeconds(
                ExactSeconds::parse("12.34")?
            ),
            "{text}"
        );
        check!(eq; got_auto, auto, "{text}");
    }
    Ok(())
}

#[test]
fn a_no_mark_word_is_not_a_mark() {
    for text in ["DNS", "ND", "FOUL", "dns", "foul", "", "   ", "q"] {
        assert!(
            parse_mark(&RUN, text).is_none(),
            "{text:?} was read as a mark"
        );
    }
}

#[test]
fn a_metric_field_mark_is_a_distance_and_never_a_time() -> TestResult {
    for kind in [
        EventKind::ShotPut,
        EventKind::Discus,
        EventKind::Javelin,
        EventKind::LongJump,
        EventKind::TripleJump,
        EventKind::HighJump,
        EventKind::PoleVault,
    ] {
        check!(kind.is_field(), "{kind:?} should be a field event");
        let mark = parse_mark(&kind, "12.34m")
            .ok_or_else(|| format!("{kind:?} refused a metric mark"))?
            .0;
        match mark {
            Mark::DistanceMetres(metres) => {
                check!(
                    (metres.as_metres_f64() - 12.34).abs() < 1e-9,
                    "{kind:?} read {metres}"
                );
            }
            other => return Err(format!("{kind:?} read a metric field mark as {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn a_multi_event_total_is_whole_points() -> TestResult {
    for (text, expected) in [("3456", 3456.0), ("3,456", 3456.0), ("1,234.5", 1234.5)] {
        let mark = parse_mark(&EventKind::Decathlon, text)
            .ok_or_else(|| format!("{text} was refused"))?
            .0;
        let points = number_of(&mark).ok_or("points carry no number")?;
        check!((points - expected).abs() < 1e-9, "{text} read as {points}");
    }
    Ok(())
}
