use super::{lines, parse_body, parse_lines, rendered_rows, seam_config, ACCURACE, STATE, TABLE};
use proptest::prelude::*;

fn prefix_holds(name: &str, body: &str, cut: usize) -> Result<(), TestCaseError> {
    let full = parse_body(body).ok_or_else(|| TestCaseError::fail("whole body did not parse"))?;
    let full_rows = rendered_rows(&full);

    let all = lines(body);
    let cut = cut % (all.len() + 1);
    let truncated: Vec<String> = all.into_iter().take(cut).collect();

    let Some(prefix) = parse_lines(&truncated) else {
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
        "{}: cut={} yields rows the whole body starts with\n  cut:  {:?}\n  full: {:?}",
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
    fn the_state_team_blocks_stay_a_prefix_when_cut_short(cut in 0usize..80) {
        prefix_holds("team blocks", STATE, cut)?;
    }

    #[test]
    fn the_padded_grade_table_stays_a_prefix_when_cut_short(cut in 0usize..80) {
        prefix_holds("padded table", TABLE, cut)?;
    }

    #[test]
    fn the_rule_lined_table_stays_a_prefix_when_cut_short(cut in 0usize..80) {
        prefix_holds("rule-lined table", ACCURACE, cut)?;
    }
}
