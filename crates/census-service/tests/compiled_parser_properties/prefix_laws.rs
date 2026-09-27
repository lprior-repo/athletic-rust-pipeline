
use super::{
    lines, parse_body, parse_lines, rendered_reading, seam_config, HEAT, LAYOUTS, REGIONAL,
};
use proptest::prelude::*;

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
