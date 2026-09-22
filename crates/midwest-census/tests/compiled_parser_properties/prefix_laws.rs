//! Append-only reading: what a page cut short is allowed to yield.
//!
//! Compiled exports reach the census through archives, resumes and half-written downloads, and the
//! walk journals what it read. Reading is one pass over the lines, so a page cut short can only yield
//! what the lines above the cut said: the same meet, the same events, and — per event — a prefix of
//! the readings the whole page publishes.
//!
//! A relay row is the one reading a later line enriches rather than replaces: the leg lines sit
//! beneath the team row, so a page cut before them publishes the team and no legs. The comparison
//! below therefore leaves legs out and asserts the enrichment separately, in `accounting`.

use super::{
    lines, parse_body, parse_lines, rendered_reading, seam_config, HEAT, LAYOUTS, REGIONAL,
};
use proptest::prelude::*;

/// The law: a body cut after `cut` lines parses to the meet the whole body names, and every event it
/// read holds a prefix of that event's readings in the whole body.
fn prefix_holds(name: &str, body: &str, cut: usize) -> Result<(), TestCaseError> {
    let full = parse_body(body).expect("the whole body is a meet");

    let all = lines(body);
    let cut = cut % (all.len() + 1);
    let truncated: Vec<String> = all.into_iter().take(cut).collect();

    let Some(prefix) = parse_lines(&truncated) else {
        return Ok(());
    };

    prop_assert_eq!(
        prefix.name.as_str(),
        full.name.as_str(),
        "{}: cut={} reads the meet the whole body names",
        name,
        cut
    );

    for event in &prefix.events {
        let Some(full_event) = full
            .events
            .iter()
            .find(|candidate| candidate.label == event.label && candidate.gender == event.gender)
        else {
            return Err(TestCaseError::fail(format!(
                "{name}: cut={cut} read the event {:?} the whole page does not publish",
                event.label
            )));
        };
        prop_assert!(
            event.rows.len() <= full_event.rows.len(),
            "{}: cut={} cannot publish more rows of {:?} than the whole page",
            name,
            cut,
            event.label
        );
        let prefix_readings: Vec<String> = event.rows.iter().map(rendered_reading).collect();
        let full_readings: Vec<String> = full_event.rows.iter().map(rendered_reading).collect();
        prop_assert!(
            prefix_readings
                .iter()
                .zip(full_readings.iter())
                .all(|(a, b)| a == b),
            "{}: cut={} publishes rows the whole page starts {:?} with\n  cut:  {:?}\n  full: {:?}",
            name,
            cut,
            event.label,
            prefix_readings,
            full_readings
        );
    }
    Ok(())
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn the_two_block_page_stays_a_prefix_when_cut_short(cut in 0usize..40) {
        prefix_holds(LAYOUTS[0].0, REGIONAL, cut)?;
    }

    #[test]
    fn the_single_heat_page_stays_a_prefix_when_cut_short(cut in 0usize..40) {
        prefix_holds(LAYOUTS[1].0, HEAT, cut)?;
    }
}
