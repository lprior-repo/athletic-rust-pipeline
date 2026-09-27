use super::{parse_body, rendered_rows, seam_config, CAPTURE, DECLINED_ROW};
use proptest::prelude::*;

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
