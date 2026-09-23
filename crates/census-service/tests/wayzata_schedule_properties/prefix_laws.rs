//! Append-only reading: what a body cut short is allowed to yield.
//!
//! Schedule pages reach the census through a resuming walk and half-written downloads, and the walk
//! journals the rows it read. Rows are read in document order and each row is read once, so a body cut
//! short can only yield rows the whole body starts with — never a competition day the missing bytes
//! would have contradicted.

use super::{rendered_rows, rows, seam_config, PAGES, SEASON};
use proptest::prelude::*;

/// The law: a body cut after `cut` characters parses to rows the whole body starts with.
fn prefix_holds(name: &str, body: &str, cut: usize) -> Result<(), TestCaseError> {
    let full =
        rows(body, SEASON).map_err(|error| TestCaseError::fail(format!("{name}: {error:?}")))?;
    let full_rows = rendered_rows(&full);

    let cut = cut % (body.chars().count() + 1);
    let truncated: String = body.chars().take(cut).collect();

    let Ok(prefix) = rows(&truncated, SEASON) else {
        return Ok(());
    };
    let prefix_rows = rendered_rows(&prefix);

    prop_assert!(
        prefix_rows.len() <= full_rows.len(),
        "{}: a truncated page cannot publish more rows than the whole one",
        name
    );
    prop_assert!(
        prefix_rows
            .iter()
            .zip(full_rows.iter())
            .all(|(a, b)| a == b),
        "{}: cut={} yields rows the whole page starts with\n  cut:  {:?}\n  full: {:?}",
        name,
        cut,
        prefix_rows,
        full_rows
    );
    Ok(())
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn the_track_schedule_stays_a_prefix_when_cut_short(cut in 0usize..16_384) {
        prefix_holds(PAGES[0].0, PAGES[0].1, cut)?;
    }

    #[test]
    fn the_cross_country_schedule_stays_a_prefix_when_cut_short(cut in 0usize..16_384) {
        prefix_holds(PAGES[1].0, PAGES[1].1, cut)?;
    }
}
