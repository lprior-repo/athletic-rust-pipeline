//! Append-only reading: what a body cut short is allowed to yield.
//!
//! Finish lists reach the census through archives, resumes and half-written downloads, and the walk
//! journals what it read. Tables are read in document order and each row is read once, so a body cut
//! short can only yield readings the whole body starts with — never a place, a mark or an athlete
//! that the missing bytes would have contradicted.

use super::{parse_body, rendered_rows, seam_config, CAPTURE, DECLINED_ROW};
use proptest::prelude::*;

/// The law: a body cut after `cut` characters parses to the meet the whole body names and to
/// readings the whole body starts with. A body whose title the cut removed is refused, which is the
/// other legitimate answer.
fn prefix_holds(name: &str, body: &str, cut: usize) -> Result<(), TestCaseError> {
    let full = parse_body(body).expect("the whole body is a meet");
    let full_rows = rendered_rows(&full);

    let cut = cut % (body.chars().count() + 1);
    let truncated: String = body.chars().take(cut).collect();

    let Ok(prefix) = parse_body(&truncated) else {
        return Ok(());
    };
    let prefix_rows = rendered_rows(&prefix);

    prop_assert!(
        prefix_rows.len() <= full_rows.len(),
        "{}: a truncated body cannot publish more rows than the whole one",
        name
    );
    prop_assert!(
        prefix_rows
            .iter()
            .zip(full_rows.iter())
            .all(|(a, b)| a == b),
        "{}: cut={} yields readings the whole body starts with\n  cut:  {:?}\n  full: {:?}",
        name,
        cut,
        prefix_rows,
        full_rows
    );
    prop_assert_eq!(
        prefix.name.as_str(),
        full.name.as_str(),
        "{}: cut={} reads the meet the whole body names",
        name,
        cut
    );
    Ok(())
}

proptest! {
    #![proptest_config(seam_config())]

    #[test]
    fn the_committed_capture_stays_a_prefix_when_cut_short(cut in 0usize..8_192) {
        prefix_holds("committed capture", CAPTURE, cut)?;
    }

    #[test]
    fn the_small_grid_stays_a_prefix_when_cut_short(cut in 0usize..512) {
        prefix_holds("grid with a declined row", DECLINED_ROW, cut)?;
    }
}
