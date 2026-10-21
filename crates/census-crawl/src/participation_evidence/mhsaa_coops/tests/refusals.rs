use super::*;

#[test]
fn changed_source_and_oversize_original_refuse_before_conversion() -> TestResult {
    for source in [
        "https://www.mhsaa.com/sites/default/files/Enrollment%20and%20Classification/hscoop.pdf?time=1",
        "http://www.mhsaa.com/sites/default/files/Enrollment%20and%20Classification/hscoop.pdf",
        "https://example.invalid/hscoop.pdf",
    ] {
        check!(matches!(
            decode_mhsaa_coop_candidate_v1(source, PDF),
            Err(CrawlError::Schema { .. })
        ));
    }
    let oversized = vec![
        0;
        MAX_PDF_BYTES
            .checked_add(1)
            .ok_or("fixture length overflow")?
    ];
    check!(matches!(
        decode_mhsaa_coop_candidate_v1(MHSAA_COOPS_SOURCE_URL, &oversized),
        Err(CrawlError::Resource {
            resource: "MHSAA co-op PDF bytes",
            ..
        })
    ));
    check!(decode_mhsaa_coop_candidate_v1(MHSAA_COOPS_SOURCE_URL, &[]).is_err());
    Ok(())
}

#[test]
fn malformed_original_is_not_replaced_with_hand_authored_layout() -> TestResult {
    check!(decode_mhsaa_coop_candidate_v1(MHSAA_COOPS_SOURCE_URL, b"not a PDF").is_err());
    Ok(())
}

#[test]
fn changed_source_year_member_sport_category_peninsula_or_division_refuses() -> TestResult {
    let original = decoded()?;
    let parsed = document::read(original.layout_text())?;
    for (span, old, new) in [
        (&parsed.heading, "2026-2027", "2025-2026"),
        (&parsed.group.header, "Pickford", "Pickfard"),
        (
            &parsed.group.header,
            "Kinross Maplewood Baptist",
            "Kinross Other School",
        ),
        (&parsed.group.row, "Cross Country", "Track and Field"),
        (&parsed.group.row, "(Boys)", "(Girls)"),
        (&parsed.group.row, " - UP", " - LP"),
        (&parsed.group.row, "(3)", "(2)"),
        (&parsed.group.row, "8/21/2024", "8/22/2024"),
    ] {
        let changed = changed_span(original.layout_text(), span, old, new)?;
        check!(document::read(&changed).is_err());
    }
    Ok(())
}

#[test]
fn enrollment_is_checked_as_enrollment_and_never_used_as_an_identity() -> TestResult {
    let original = decoded()?;
    let parsed = document::read(original.layout_text())?;
    for (old, new) in [
        ("(162)", "(163)"),
        ("(139)", "(0)"),
        ("(23)", "(023)"),
        ("(23)", "(65536)"),
        (
            "Kinross Maplewood Baptist(23)",
            "Kinross Maplewood Baptist(23), Other School(5)",
        ),
    ] {
        let changed = changed_span(original.layout_text(), &parsed.group.header, old, new)?;
        check!(document::read(&changed).is_err());
    }
    Ok(())
}

#[test]
fn duplicated_program_primary_or_ambiguous_partner_continuation_refuses() -> TestResult {
    let original = decoded()?;
    let parsed = document::read(original.layout_text())?;
    for span in [&parsed.group.header, &parsed.group.row] {
        let literal = original.literal(span).ok_or("missing duplicated literal")?;
        let changed = changed_span(
            original.layout_text(),
            span,
            literal,
            &format!("{literal}\n{literal}"),
        )?;
        check!(document::read(&changed).is_err());
    }
    let header = original
        .literal(&parsed.group.header)
        .ok_or("missing primary header")?;
    let changed = changed_span(
        original.layout_text(),
        &parsed.group.header,
        header,
        &format!("{header}\n                                          Other School(5)"),
    )?;
    check!(document::read(&changed).is_err());
    let changed = changed_span(
        original.layout_text(),
        &parsed.group.row,
        "8/21/2024",
        "8/21/2024  2026-27",
    )?;
    check!(document::read(&changed).is_err());
    Ok(())
}

#[test]
fn layout_controls_wrong_page_accounting_and_resource_overflow_refuse() -> TestResult {
    let original = decoded()?;
    let parsed = document::read(original.layout_text())?;
    let changed = changed_span(
        original.layout_text(),
        &parsed.revision_label,
        "23-Jun-26",
        "23-Jun-25",
    )?;
    check!(document::read(&changed).is_err());
    let changed = original
        .layout_text()
        .replace("Page 24 of 33", "Page 25 of 33");
    check!(document::read(&changed).is_err());
    let changed = original.layout_text().replace("Pickford", "Pick\0ford");
    check!(document::read(&changed).is_err());
    let oversized = " ".repeat(MAX_TEXT_BYTES.checked_add(1).ok_or("fixture overflow")?);
    check!(matches!(
        document::read(&oversized),
        Err(CrawlError::Resource {
            resource: "MHSAA co-op layout bytes",
            ..
        })
    ));
    let long_line = " ".repeat(MAX_LINE_BYTES.checked_add(1).ok_or("fixture overflow")?);
    let changed = format!("{long_line}\n{}", original.layout_text());
    check!(matches!(
        document::read(&changed),
        Err(CrawlError::Resource {
            resource: "MHSAA co-op layout line bytes",
            ..
        })
    ));
    Ok(())
}
