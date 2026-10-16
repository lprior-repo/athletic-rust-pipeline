use crate::milesplit::raw::{parse_raw, RawPage};
use crate::result_file::ParsedRow;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod guards;
mod located;
mod placing;
mod support;

use support::{base_header, base_row, document, one_row, page, refused};

#[test]
fn declared_wind_is_not_promoted_to_a_heat() -> TestResult {
    let header = format!("{}{:>8}", base_header(), "Wind");
    let row = format!("{}{:>8}", base_row("Final Runner", "13.26"), "+0.2");
    let parsed = page(&header, &row)?;
    let row = one_row(&parsed)?;
    check!(eq; row.wind_mps, Some(0.2));
    check!(eq; row.heat, None);
    check!(eq; row.name, "Final Runner");
    check!(eq; row.school, "Synthetic School");
    check!(eq; row.grade.map(|grade| grade.get()), Some(12));
    check!(eq;
        row.mark,
        census_domain::model::Mark::TimeSeconds(
            crate::hytek::parse_time("13.26").ok_or("exact time")?
        )
    );
    Ok(())
}

#[test]
fn declared_heat_and_wind_follow_either_header_order_and_case() -> TestResult {
    for label in ["H#", "Heat", "hEaT"] {
        let heat_first = format!("{}{:>6}{:>8}", base_header(), label, "wInD");
        let first_row = format!("{}{:>6}{:>8}", base_row("Runner", "13.26"), "2", "+0.2");
        let wind_first = format!("{}{:>8}{:>6}", base_header(), "Wind", label);
        let second_row = format!("{}{:>8}{:>6}", base_row("Runner", "13.26"), "+0.2", "2");
        let first = page(&heat_first, &first_row)?;
        let second = page(&wind_first, &second_row)?;
        check!(eq; one_row(&first)?, one_row(&second)?);
        check!(eq; one_row(&first)?.wind_mps, Some(0.2));
        check!(eq; one_row(&first)?.heat.as_deref(), Some("2"));
    }
    Ok(())
}

#[test]
fn declared_signed_decimal_wind_keeps_its_value() -> TestResult {
    let header = format!("{}{:>12}", base_header(), "Wind");
    for (token, value) in [
        ("+0.2", 0.2),
        ("-1.4", -1.4),
        ("0", 0.0),
        ("2", 2.0),
        ("+2.000", 2.0),
    ] {
        let row = format!("{}{:>12}", base_row("Runner", "13.26"), token);
        let parsed = page(&header, &row)?;
        check!(eq; one_row(&parsed)?.wind_mps, Some(value));
        check!(eq; one_row(&parsed)?.heat, None);
    }
    Ok(())
}

#[test]
fn declared_empty_cells_remain_independently_absent() -> TestResult {
    let header = format!("{}{:>8}{:>6}", base_header(), "Wind", "H#");
    for (wind, heat, expected_wind, expected_heat) in [
        ("", "", None, None),
        ("", "2", None, Some("2")),
        ("+0.2", "", Some(0.2), None),
    ] {
        let row = format!("{}{:>8}{:>6}", base_row("Runner", "13.26"), wind, heat);
        let parsed = page(&header, &row)?;
        check!(eq; one_row(&parsed)?.wind_mps, expected_wind);
        check!(eq; one_row(&parsed)?.heat.as_deref(), expected_heat);
    }
    let truncated = page(&header, &base_row("Runner", "13.26"))?;
    check!(eq; one_row(&truncated)?.wind_mps, None);
    check!(eq; one_row(&truncated)?.heat, None);
    Ok(())
}

#[test]
fn unlabeled_prelim_and_three_hundred_hurdle_tails_keep_legacy_heat() -> TestResult {
    for (event, mark) in [
        ("Boys 110 Hurdles Prelims", "13.26"),
        ("Boys 300 Hurdles Finals", "36.75"),
    ] {
        let row = format!("{} 8 (1)", base_row("Runner", mark));
        let block = format!("{event}\n{}\n{row}\n", base_header());
        let parsed = parse_raw(&document(&block), support::URL)?;
        check!(eq; one_row(&parsed)?.wind_mps, None);
        check!(eq; one_row(&parsed)?.heat.as_deref(), Some("8"));
    }
    Ok(())
}

#[test]
fn heat_only_header_does_not_invent_wind() -> TestResult {
    let header = format!("{}{:>6}", base_header(), "Heat");
    let row = format!("{}{:>6}", base_row("Runner", "36.75"), "7");
    let parsed = page(&header, &row)?;
    check!(eq; one_row(&parsed)?.wind_mps, None);
    check!(eq; one_row(&parsed)?.heat.as_deref(), Some("7"));
    Ok(())
}

#[test]
fn unqualified_unknown_tail_label_retains_the_existing_legacy_grammar() -> TestResult {
    let header = format!("{}{:>8}", base_header(), "Score");
    let row = format!("{}{:>8} (1)", base_row("Runner", "13.26"), "5");
    let parsed = page(&header, &row)?;
    check!(eq; one_row(&parsed)?.wind_mps, None);
    check!(eq; one_row(&parsed)?.heat.as_deref(), Some("5"));
    Ok(())
}
