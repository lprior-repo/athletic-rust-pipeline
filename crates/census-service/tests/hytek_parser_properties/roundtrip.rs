//! The round trip between a published time and the seconds it means.
//!
//! Every mark the corpus keeps is a `f64` of seconds, so the only thing standing between a
//! published page and a wrong personal best is this conversion. The law is stated in both
//! directions: render a duration the way the vendor writes one, and it must read back unchanged —
//! at every scale the notation distinguishes (tenths-and-up under a minute, minutes, hours).

use super::{parse_time, seam_config};
use proptest::prelude::*;

/// Centiseconds published in the vendor's own notation, plus the seconds they mean.
///
/// `10.56` under a minute, `1:54.32` under an hour, `1:05:12.34` past it — the three shapes the
/// vendor's captures show. The renderer is deliberately the *inverse* of the parser and shares no
/// code with it, so agreement is evidence rather than a restatement.
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

    /// Ten hours of published times, to the centisecond, at every shape of the notation.
    #[test]
    fn a_rendered_time_reads_back_as_the_same_duration(centis in 0u64..36_000_000) {
        let (text, expected) = render_centis(centis);
        let parsed = parse_time(&text)
            .unwrap_or_else(|| panic!("{text} (from {centis}) was refused"));
        prop_assert!(
            (parsed - expected).abs() < 1e-9,
            "{text} parsed to {} not {}",
            parsed,
            expected
        );
    }
}

#[test]
fn the_vendors_own_notation_reads_exactly() {
    // The notation table from the parser's own documentation, plus the one mark this repository has
    // in hand: the `12:40.6` of the MileSplit `/raw` capture.
    let cases = [
        ("10.56", 10.56),
        ("1:54.32", 114.32),
        ("15:32.1", 932.1),
        ("1:05:12.34", 3912.34),
        ("12:40.6", 760.6),
        ("  4:32  ", 272.0),
    ];
    for (text, expected) in cases {
        let parsed = parse_time(text).unwrap_or_else(|| panic!("{text} was refused"));
        assert!(
            (parsed - expected).abs() < 1e-9,
            "{text} parsed to {parsed}, not {expected}"
        );
    }
}

#[test]
fn a_seconds_field_of_sixty_or_more_is_not_a_time() {
    // `1:75` is not a minute and a quarter: the seconds field of a two- or three-part time is a
    // field of a sexagesimal, and a parser that adds it up anyway would silently invent a mark.
    for text in ["1:75", "1:60", "1:60.0", "1:59:60", "1:60:00", "2:-1"] {
        assert!(
            parse_time(text).is_none(),
            "{text} was read as a time: {:?}",
            parse_time(text)
        );
    }
}
