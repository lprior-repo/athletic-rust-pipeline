use super::*;

#[test]
fn malformed_declared_wind_is_a_located_row_refusal() -> TestResult {
    let header = format!("{}{:>20}", base_header(), "Wind");
    for token in [
        "NaN", "inf", "+inf", "-inf", "1e2", "1E-2", ".2", "2.", "+", "--0.2", "0.2m/s", "0.2 0.3",
        "0,2", "٢",
    ] {
        let row = format!("{}{:>20}", base_row("Runner", "13.26"), token);
        refused(&header, &row, "invalid declared wind field")?;
    }
    Ok(())
}

#[test]
fn malformed_declared_heat_is_a_located_row_refusal() -> TestResult {
    let header = format!("{}{:>20}", base_header(), "Heat");
    for token in ["+2", "-2", "2.0", "2e1", "2 (1)", "2 3", "NaN", "٢"] {
        let row = format!("{}{:>20}", base_row("Runner", "13.26"), token);
        refused(&header, &row, "invalid declared heat field")?;
    }
    Ok(())
}

#[test]
fn optional_tokens_admit_sixteen_bytes_and_refuse_seventeen() -> TestResult {
    let exact = "1234567890123456";
    let over = "12345678901234567";
    for (label, reason) in [
        ("Wind", "invalid declared wind field"),
        ("H#", "invalid declared heat field"),
    ] {
        let header = format!("{}{:>20}", base_header(), label);
        let row = format!("{}{:>20}", base_row("Runner", "13.26"), exact);
        let parsed = page(&header, &row)?;
        if label == "Wind" {
            check!(eq; one_row(&parsed)?.wind_mps, Some(1_234_567_890_123_456.0));
        } else {
            check!(eq; one_row(&parsed)?.heat.as_deref(), Some(exact));
        }
        let row = format!("{}{:>20}", base_row("Runner", "13.26"), over);
        refused(&header, &row, reason)?;
    }
    Ok(())
}

#[test]
fn content_after_declared_columns_is_not_silently_ignored() -> TestResult {
    let header = format!("{}{:>8}{:>6}", base_header(), "Wind", "Heat");
    let row = format!("{}{:>8}{:>6}", base_row("Runner", "13.26"), "+0.2", "2");
    for suffix in [" extra", " (1)", " +0.4"] {
        refused(
            &header,
            &format!("{row}{suffix}"),
            "undeclared result tail content",
        )?;
    }
    let parsed = page(&header, &format!("{row}   \t"))?;
    check!(eq; one_row(&parsed)?.wind_mps, Some(0.2));
    check!(eq; one_row(&parsed)?.heat.as_deref(), Some("2"));
    Ok(())
}

#[test]
fn a_token_crossing_declared_field_boundaries_is_refused() -> TestResult {
    let header = format!("{}{:>8}{:>6}", base_header(), "Wind", "Heat");
    let row = format!("{}    +0.22     ", base_row("Runner", "13.26"));
    refused(&header, &row, "invalid declared tail columns")
}

#[test]
fn duplicate_or_malformed_optional_headers_disable_the_prior_column_map() -> TestResult {
    let valid = format!("{}{:>8}", base_header(), "Wind");
    let first = format!("{}{:>8}", base_row("First Runner", "13.26"), "+0.2");
    let later = format!("{}{:>8}", base_row("Later Runner", "13.27"), "+0.3");
    for invalid in [
        format!("{}{:>8}{:>8}", base_header(), "Wind", "Wind"),
        format!("{}{:>8}{:>8}", base_header(), "H#", "Heat"),
        format!("{}{:>12}", base_header(), "Wind(m/s)"),
        format!("{}{:>12}", base_header(), "Heat?"),
        format!("{}{:>8}{:>8}", base_header(), "Score", "Wind"),
        format!("{}{:>6}{:>8}{:>8}", base_header(), "H#", "Score", "Wind"),
        format!("{}{:>8}{:>8}", base_header(), "Wind", "Score"),
    ] {
        let block = format!(
            "Boys 110 Hurdles Finals\n{valid}\n{first}\n{invalid}\n{later}\n{valid}\n{later}\n"
        );
        let html = document(&block);
        let parsed = parse_raw(&html, support::URL)?;
        check!(eq; parsed.meet.rows_parsed, 2);
        check!(eq; parsed.meet.rows_skipped, 1);
        check!(eq; parsed.skipped.len(), 1);
        let offset = html.find(&later).ok_or("rejected original row")?;
        check!(eq;
            parsed.skipped.first().ok_or("skip")?,
            &format!("pre line=5 byte_offset={offset} byte_length={}: invalid declared tail columns; raw={later:?}", later.len())
        );
        let rows = &parsed.meet.events.first().ok_or("event")?.rows;
        check!(eq; rows.first().ok_or("first")?.wind_mps, Some(0.2));
        check!(eq; rows.get(1).ok_or("restored")?.wind_mps, Some(0.3));
    }
    Ok(())
}

#[test]
fn an_optional_header_before_the_mark_cannot_authorize_a_tail() -> TestResult {
    let header = format!(
        "  Pl {:<24} {:<3} {:<24}{:>9}",
        "Name", "Yr", "Team Wind", "Time"
    );
    refused(
        &header,
        &base_row("Runner", "13.26"),
        "invalid declared tail columns",
    )
}

#[test]
fn unknown_column_before_wind_cannot_be_reinterpreted_as_wind() -> TestResult {
    let header = format!("{}{:>8}{:>8}", base_header(), "Score", "Wind");
    let row = format!("{}{:>8}{:>8}", base_row("Runner", "13.26"), "5", "");
    refused(&header, &row, "invalid declared tail columns")
}

#[test]
fn unknown_column_between_heat_and_wind_cannot_supply_wind() -> TestResult {
    let header = format!("{}{:>6}{:>8}{:>8}", base_header(), "H#", "Score", "Wind");
    let row = format!(
        "{}{:>6}{:>8}{:>8}",
        base_row("Runner", "13.26"),
        "2",
        "5",
        ""
    );
    refused(&header, &row, "invalid declared tail columns")
}

#[test]
fn unknown_column_after_declared_wind_invalidates_even_a_blank_cell() -> TestResult {
    let header = format!("{}{:>8}{:>8}", base_header(), "Wind", "Score");
    let row = format!("{}{:>8}{:>8}", base_row("Runner", "13.26"), "+0.2", "");
    refused(&header, &row, "invalid declared tail columns")
}
