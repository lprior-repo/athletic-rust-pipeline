use super::{parse_time, seam_config};
use proptest::prelude::*;

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

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn a_rendered_time_reads_back_as_the_same_duration(centis in 0u64..36_000_000) {
        let (text, expected) = render_centis(centis);
        let parsed = parse_time(&text)
            .ok_or_else(|| TestCaseError::fail(format!("{text} (from {centis}) was refused")))?;
        prop_assert!(
            (parsed.as_seconds_f64() - expected).abs() < 1e-9,
            "{text} parsed to {} not {}",
            parsed,
            expected
        );
    }
}

#[test]
fn the_vendors_own_notation_reads_exactly() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        ("10.56", 10.56),
        ("1:54.32", 114.32),
        ("15:32.1", 932.1),
        ("1:05:12.34", 3912.34),
        ("12:40.6", 760.6),
        ("  4:32  ", 272.0),
    ];
    for (text, expected) in cases {
        let parsed = parse_time(text).ok_or_else(|| format!("{text} was refused"))?;
        check!(
            (parsed.as_seconds_f64() - expected).abs() < 1e-9,
            "{text} parsed to {parsed}, not {expected}"
        );
    }
    Ok(())
}

#[test]
fn a_seconds_field_of_sixty_or_more_is_not_a_time() {
    for text in ["1:75", "1:60", "1:60.0", "1:59:60", "1:60:00", "2:-1"] {
        assert!(
            parse_time(text).is_none(),
            "{text} was read as a time: {:?}",
            parse_time(text)
        );
    }
}
