use super::*;

#[test]
fn heat_local_placing_does_not_replace_overall_place_or_heat() -> TestResult {
    for heat_label in ["H#", "Heat"] {
        let header = format!("{}{:>6}{:>8}", base_header(), heat_label, "(Pl)");
        let row = format!("{}{:>6}{:>8}", base_row("Runner", "8:44.73"), "8", "(3)");
        let parsed = page(&header, &row)?;
        let row = one_row(&parsed)?;
        check!(eq; row.place, Some(1));
        check!(eq; row.heat.as_deref(), Some("8"));
        check!(eq; row.wind_mps, None);
        check!(eq;
            row.mark,
            census_domain::model::Mark::TimeSeconds(
                crate::hytek::parse_time("8:44.73").ok_or("published time")?
            )
        );
    }
    Ok(())
}

#[test]
fn wind_heat_and_local_placing_use_all_three_declared_endpoints() -> TestResult {
    for (labels, values) in [
        (["Wind", "H#", "(Pl)"], ["+0.2", "8", "(3)"]),
        (["Heat", "Wind", "(pL)"], ["8", "+0.2", "(3)"]),
        (["H#", "(Pl)", "Wind"], ["8", "(3)", "+0.2"]),
    ] {
        let [first_label, second_label, third_label] = labels;
        let [first_value, second_value, third_value] = values;
        let header = format!(
            "{}{:>8}{:>8}{:>8}",
            base_header(),
            first_label,
            second_label,
            third_label
        );
        let row = format!(
            "{}{:>8}{:>8}{:>8}",
            base_row("Runner", "13.26"),
            first_value,
            second_value,
            third_value
        );
        let parsed = page(&header, &row)?;
        let row = one_row(&parsed)?;
        check!(eq; row.place, Some(1));
        check!(eq; row.heat.as_deref(), Some("8"));
        check!(eq; row.wind_mps, Some(0.2));
    }
    Ok(())
}

#[test]
fn no_mark_rows_allow_an_empty_declared_heat_local_place() -> TestResult {
    let header = format!("{}{:>6}{:>8}", base_header(), "H#", "(Pl)");
    for mark in ["DNF", "NT"] {
        let row = format!("{}{:>6}{:>8}", base_row("Runner", mark), "8", "");
        let parsed = page(&header, &row)?;
        let row = one_row(&parsed)?;
        check!(eq; row.mark, census_domain::model::Mark::Raw(mark.to_string()));
        check!(eq; row.place, Some(1));
        check!(eq; row.heat.as_deref(), Some("8"));
        check!(eq; row.wind_mps, None);
    }
    Ok(())
}

#[test]
fn malformed_heat_local_place_is_a_located_row_refusal() -> TestResult {
    let header = format!("{}{:>6}{:>20}", base_header(), "H#", "(Pl)");
    for token in [
        "1", "(0)", "(000)", "(-1)", "(+1)", "()", "(1.0)", "(1) (2)", "(١)", "(1", "1)",
    ] {
        let row = format!("{}{:>6}{:>20}", base_row("Runner", "13.26"), "8", token);
        refused(&header, &row, "invalid declared heat place field")?;
    }
    Ok(())
}

#[test]
fn heat_local_place_admits_sixteen_bytes_and_refuses_seventeen() -> TestResult {
    let header = format!("{}{:>6}{:>20}", base_header(), "H#", "(Pl)");
    let row = format!(
        "{}{:>6}{:>20}",
        base_row("Runner", "13.26"),
        "8",
        "(12345678901234)"
    );
    let parsed = page(&header, &row)?;
    check!(eq; one_row(&parsed)?.place, Some(1));
    check!(eq; one_row(&parsed)?.heat.as_deref(), Some("8"));
    let over = format!(
        "{}{:>6}{:>20}",
        base_row("Runner", "13.26"),
        "8",
        "(123456789012345)"
    );
    refused(&header, &over, "invalid declared heat place field")
}

#[test]
fn local_place_requires_one_well_formed_header_after_a_declared_heat() -> TestResult {
    let row = format!(
        "{}{:>8}{:>8}{:>8}",
        base_row("Runner", "13.26"),
        "8",
        "(1)",
        ""
    );
    for labels in [
        ["(Pl)", "", ""],
        ["Wind", "(Pl)", ""],
        ["(Pl)", "H#", ""],
        ["H#", "(Pl)", "(Pl)"],
        ["H#", "(Pl?)", ""],
        ["H#", "(Place)", ""],
    ] {
        let [first, second, third] = labels;
        let header = format!("{}{:>8}{:>8}{:>8}", base_header(), first, second, third);
        refused(&header, &row, "invalid declared tail columns")?;
    }
    Ok(())
}

#[test]
fn local_place_does_not_relax_unknown_columns_or_trailing_content() -> TestResult {
    let row = format!(
        "{}{:>8}{:>8}{:>8}",
        base_row("Runner", "13.26"),
        "8",
        "(1)",
        ""
    );
    for labels in [["H#", "Score", "(Pl)"], ["H#", "(Pl)", "Score"]] {
        let [first, second, third] = labels;
        let header = format!("{}{:>8}{:>8}{:>8}", base_header(), first, second, third);
        refused(&header, &row, "invalid declared tail columns")?;
    }
    let header = format!("{}{:>8}{:>8}", base_header(), "H#", "(Pl)");
    let row = format!(
        "{}{:>8}{:>8} extra",
        base_row("Runner", "13.26"),
        "8",
        "(1)"
    );
    refused(&header, &row, "undeclared result tail content")
}

#[test]
fn invalid_local_place_headers_replace_the_prior_map_until_a_valid_header_returns() -> TestResult {
    let valid = format!("{}{:>8}{:>8}", base_header(), "H#", "(Pl)");
    let first = format!(
        "{}{:>8}{:>8}",
        base_row("First Runner", "13.26"),
        "8",
        "(1)"
    );
    let later = format!(
        "{}{:>8}{:>8}",
        base_row("Later Runner", "13.27"),
        "8",
        "(2)"
    );
    for invalid in [
        format!("{valid}{:>8}", "(Pl)"),
        format!("{}{:>8}{:>8}", base_header(), "H#", "(Pl?)"),
        format!("{}{:>8}{:>8}", base_header(), "(Pl)", "H#"),
    ] {
        let block = format!(
            "Boys 110 Hurdles Finals\n{valid}\n{first}\n{invalid}\n{later}\n{valid}\n{later}\n"
        );
        let html = document(&block);
        let parsed = parse_raw(&html, support::URL)?;
        check!(eq; parsed.meet.rows_parsed, 2);
        check!(eq; parsed.meet.rows_skipped, 1);
        check!(eq; parsed.skipped.len(), 1);
        let offset = html.find(&later).ok_or("original row")?;
        check!(eq;
            parsed.skipped.first().ok_or("skip")?,
            &format!("pre line=5 byte_offset={offset} byte_length={}: invalid declared tail columns; raw={later:?}", later.len())
        );
        let rows = &parsed.meet.events.first().ok_or("event")?.rows;
        check!(eq; rows.first().ok_or("first")?.place, Some(1));
        check!(eq; rows.get(1).ok_or("restored")?.heat.as_deref(), Some("8"));
    }
    Ok(())
}
