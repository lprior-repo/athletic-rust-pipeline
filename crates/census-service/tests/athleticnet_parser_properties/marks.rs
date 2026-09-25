//! What a published Athletic.net mark column means.
//!
//! The column is the corpus's most valuable two bytes per athlete, so the laws here are the ones
//! that decide whether a stored mark is the mark the page printed.

use super::{number_of, parse_mark, seam_config};
use census_domain::model::{CentiSeconds, EventKind, Mark};
use proptest::prelude::*;

/// Centiseconds published in the notation both parsers read, plus the seconds they mean.
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

/// A running-event kind: not a field event, not a multi-event.
const RUN: EventKind = EventKind::CrossCountry;

proptest! {
    #![proptest_config(seam_config())]

    /// Every published time, at every shape of the notation, is the duration it prints.
    #[test]
    fn a_published_time_reads_as_the_duration_it_prints(centis in 0u64..36_000_000) {
        let (text, expected) = render_centis(centis);
        let (mark, auto) = parse_mark(&RUN, &text)
            .unwrap_or_else(|| panic!("{text} (from {centis}) was refused"));
        prop_assert!(!auto, "a bare mark is not marked automatic: {text}");
        prop_assert_eq!(mark, Mark::TimeSeconds(CentiSeconds::try_from_seconds_f64(expected).expect("fixture is in range")));
    }

    /// The same token, published with the automatic-timing suffix, is the same mark.
    #[test]
    fn an_automatic_suffix_marks_the_flag_not_the_mark(centis in 0u64..36_000_000) {
        let (text, expected) = render_centis(centis);
        let (mark, auto) = parse_mark(&RUN, &format!("{text}a"))
            .unwrap_or_else(|| panic!("{text}a was refused"));
        prop_assert!(auto, "the `a` suffix is the automatic flag: {text}a");
        prop_assert_eq!(mark, Mark::TimeSeconds(CentiSeconds::try_from_seconds_f64(expected).expect("fixture is in range")));
    }
}

#[test]
fn the_notation_table_holds_and_defers_to_one_time_parser() {
    // The two routes to a published time — this source's mark reader and the shared vendor parser
    // that reads the same notation — must not diverge: a divergence stores two values for one mark.
    for (text, expected) in [
        ("10.56", 10.56),
        ("1:54.32", 114.32),
        ("15:32.1", 932.1),
        ("1:05:12.34", 3912.34),
    ] {
        let (mark, _) = parse_mark(&RUN, text).unwrap_or_else(|| panic!("{text} was refused"));
        assert_eq!(
            mark,
            Mark::TimeSeconds(
                CentiSeconds::try_from_seconds_f64(expected).expect("fixture is in range")
            ),
            "{text}"
        );
    }
}

#[test]
fn a_qualifier_is_stripped_rather_than_read_as_part_of_the_mark() {
    for (text, auto) in [
        ("12.34", false),
        ("12.34q", false),
        ("12.34Q", false),
        ("12.34p", false),
        ("12.34P", false),
        ("12.34a", true),
        ("12.34A", true),
    ] {
        let (mark, got_auto) = parse_mark(&RUN, text).unwrap_or_else(|| panic!("{text} refused"));
        assert_eq!(
            mark,
            Mark::TimeSeconds(
                CentiSeconds::try_from_seconds_f64(12.34).expect("fixture is in range")
            ),
            "{text}"
        );
        assert_eq!(got_auto, auto, "{text}");
    }
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
fn a_metric_field_mark_is_a_distance_and_never_a_time() {
    for kind in [
        EventKind::ShotPut,
        EventKind::Discus,
        EventKind::Javelin,
        EventKind::LongJump,
        EventKind::TripleJump,
        EventKind::HighJump,
        EventKind::PoleVault,
    ] {
        assert!(kind.is_field(), "{kind:?} should be a field event");
        let mark = parse_mark(&kind, "12.34m")
            .unwrap_or_else(|| panic!("{kind:?} refused a metric mark"))
            .0;
        match mark {
            Mark::DistanceMetres(metres) => {
                assert!(
                    (metres.as_metres_f64() - 12.34).abs() < 1e-9,
                    "{kind:?} read {metres}"
                );
            }
            other => panic!("{kind:?} read a metric field mark as {other:?}"),
        }
    }
}

#[test]
fn a_multi_event_total_is_whole_points() {
    for (text, expected) in [("3456", 3456.0), ("3,456", 3456.0), ("1,234.5", 1234.5)] {
        let mark = parse_mark(&EventKind::Decathlon, text)
            .unwrap_or_else(|| panic!("{text} was refused"))
            .0;
        let points = number_of(&mark).expect("points carry a number");
        assert!((points - expected).abs() < 1e-9, "{text} read as {points}");
    }
}
