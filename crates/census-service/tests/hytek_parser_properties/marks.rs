//! One field mark, written every way the vendor writes it, reading as one mark.
//!
//! A mark's *value* is what the corpus compares; its `feet_mark` is kept verbatim because it is what
//! the source published. The law is that the several spellings of five foot six — and of a jump
//! tie-break's `J` prefix, which is not part of the mark — agree on both the value and the metric
//! conversion, and that a plain metre figure is a distance mark of its own.

use super::{metres_of, parse_field_mark};
use census_domain::model::{CentiMetres, Mark};

/// Inches in a foot, per the conversion the parser applies.
const INCH_METRES: f64 = 0.0254;

#[test]
fn five_foot_six_is_five_foot_six_however_it_is_written() {
    let expected = 66.0 * INCH_METRES;
    for token in [
        "5-6", "5-06", "5-6.0", "5' 6\"", "5'6\"", "J 5-6", "J5-6", " 5-6 ",
    ] {
        let mark = parse_field_mark(token).unwrap_or_else(|| panic!("{token} was refused"));
        assert!(
            (metres_of(&mark) - expected).abs() < 0.015,
            "{token} read as {} m, not {expected} m",
            metres_of(&mark)
        );
    }
}

#[test]
fn the_published_spelling_is_kept_beside_the_value() {
    let mark = parse_field_mark("J 61-03.50").expect("a six-foot-plus jump parses");
    match mark {
        Mark::FieldImperial { feet_mark, metres } => {
            assert_eq!(feet_mark, "61-03.50");
            assert!((metres.as_metres_f64() - (61.0 * 12.0 + 3.5) * INCH_METRES).abs() < 0.015);
        }
        other => panic!("a jump is an imperial mark, got {other:?}"),
    }
}

#[test]
fn a_bare_metre_figure_is_a_distance_mark() {
    let mark = parse_field_mark("14.25").expect("a metric mark parses");
    assert_eq!(
        mark,
        Mark::DistanceMetres(CentiMetres::try_from_metres_f64(14.25).expect("fixture is in range"))
    );
    assert!((metres_of(&mark) - 14.25).abs() < 0.015);
}

#[test]
fn a_feet_only_mark_is_whole_feet() {
    let mark = parse_field_mark("16'").expect("a feet-only mark parses");
    assert!((metres_of(&mark) - 16.0 * 12.0 * INCH_METRES).abs() < 0.015);
}
