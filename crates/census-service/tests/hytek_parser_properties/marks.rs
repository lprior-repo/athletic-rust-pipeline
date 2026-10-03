use super::{metres_of, parse_field_mark};
use census_domain::model::{CentiMetres, Mark};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const INCH_METRES: f64 = 0.0254;

#[test]
fn five_foot_six_is_five_foot_six_however_it_is_written() -> TestResult {
    let expected = 66.0 * INCH_METRES;
    for token in [
        "5-6", "5-06", "5-6.0", "5' 6\"", "5'6\"", "J 5-6", "J5-6", " 5-6 ",
    ] {
        let mark = parse_field_mark(token).ok_or_else(|| format!("{token} was refused"))?;
        check!(
            (metres_of(&mark)? - expected).abs() < 0.015,
            "{token} read as {} m, not {expected} m",
            metres_of(&mark)?
        );
    }
    Ok(())
}

#[test]
fn the_published_spelling_is_kept_beside_the_value() -> TestResult {
    let mark = parse_field_mark("J 61-03.50").ok_or("imperial jump did not parse")?;
    match mark {
        Mark::FieldImperial { feet_mark, metres } => {
            check!(eq; feet_mark, "61-03.50");
            check!((metres.as_metres_f64() - (61.0 * 12.0 + 3.5) * INCH_METRES).abs() < 0.015);
        }
        other => return Err(format!("a jump is an imperial mark, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn a_bare_metre_figure_is_a_distance_mark() -> TestResult {
    let mark = parse_field_mark("14.25").ok_or("metric mark did not parse")?;
    check!(eq; mark,
    Mark::DistanceMetres(CentiMetres::try_from_metres_f64(14.25).ok_or("invalid expected distance")?));
    check!((metres_of(&mark)? - 14.25).abs() < 0.015);
    Ok(())
}

#[test]
fn a_feet_only_mark_is_whole_feet() -> TestResult {
    let mark = parse_field_mark("16'").ok_or("feet-only mark did not parse")?;
    check!((metres_of(&mark)? - 16.0 * 12.0 * INCH_METRES).abs() < 0.015);
    Ok(())
}
