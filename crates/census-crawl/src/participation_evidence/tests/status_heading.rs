use super::*;

const INDEX: &str = "<h4><span class=\"text--secondary\">Previous 2025-26 honorees</span></h4>";

fn status_only(body: &[u8]) -> TestResult<status::Status> {
    let completed = DateTime::parse_from_rfc3339("2026-10-16T00:05:15Z")?.with_timezone(&Utc);
    let capture = RetainedHtmlCaptureV1::new(
        STATUS_URL,
        STATUS_URL,
        200,
        completed,
        body,
        provenance(
            "2026-10-16T00:05:12Z",
            include_bytes!("fixtures/mhsaa-status.headers"),
        )?,
    )?;
    Ok(status::read(&capture)?)
}

#[test]
fn original_subject_and_exact_later_index_preserve_original_interpretation() -> TestResult {
    let body = std::str::from_utf8(STATUS)?;
    check!(eq; body.get(118714..118785), Some(INDEX));
    let value = read(&packet(STATUS, SCHEDULE)?)?;
    check!(eq; value.subject().name(), "Zacchaeus Brocks");
    check!(eq; value.subject().locator(), "html_bytes:113581:113939");
    check!(eq;
        value.status_proof().interpretation(),
        "37f808a142850ee27ba10e506fb169b5270708c855dbc0f939995f80ce4328de"
    );
    let without_index = body.replacen(INDEX, "", 1);
    check!(eq;
        status_only(without_index.as_bytes())?.subject.name(),
        "Zacchaeus Brocks"
    );
    Ok(())
}

#[test]
fn second_supported_or_contrary_subject_cannot_hide_as_an_index_heading() -> TestResult {
    let body = std::str::from_utf8(STATUS)?;
    let original = body.get(113581..113939).ok_or("original subject heading")?;
    for heading in [
        original.to_string(),
        original.replace("Zacchaeus Brocks", "Second Named Participant"),
        original.replace("Senior ♦ Track", "Junior ♦ Track"),
        original.replace("Detroit Catholic Central", "Another School"),
        original.replace("Senior ♦ Track", "Senior ♦ Swimming"),
        "<h4><span class=\"text--secondary\">Unsupported status declaration</span></h4>".into(),
    ] {
        let changed = body.replacen(INDEX, &heading, 1);
        check!(status_only(changed.as_bytes()).is_err());
    }
    Ok(())
}

#[test]
fn index_must_be_unique_later_and_exactly_the_canonical_source_school_year() -> TestResult {
    let body = std::str::from_utf8(STATUS)?;
    let original = body.get(113581..113939).ok_or("original subject heading")?;
    let without = body.replacen(INDEX, "", 1);
    let before = without.replacen(original, &format!("{INDEX}{original}"), 1);
    let duplicate = body.replacen(INDEX, &format!("{INDEX}{INDEX}"), 1);
    let wrong_year = body.replacen(INDEX, &INDEX.replace("2025-26", "2024-25"), 1);
    for changed in [before, duplicate, wrong_year] {
        check!(status_only(changed.as_bytes()).is_err());
    }
    Ok(())
}

#[test]
fn index_edition_uses_existing_july_august_school_year_boundary() -> TestResult {
    let body = std::str::from_utf8(STATUS)?;
    let july = body.replacen("May 15, 2026", "July 31, 2026", 1);
    check!(eq; status_only(july.as_bytes())?.day.as_str(), "2026-07-31");
    let august = body.replacen("May 15, 2026", "August 1, 2026", 1);
    check!(status_only(august.as_bytes()).is_err());
    let august = august.replacen(INDEX, &INDEX.replace("2025-26", "2026-27"), 1);
    check!(eq; status_only(august.as_bytes())?.day.as_str(), "2026-08-01");
    check!(status_only(
        july.replacen(INDEX, &INDEX.replace("2025-26", "2026-27"), 1)
            .as_bytes()
    )
    .is_err());
    Ok(())
}

#[test]
fn secondary_claim_requires_its_exact_span_and_cannot_ignore_outside_text() -> TestResult {
    let body = std::str::from_utf8(STATUS)?;
    for changed in [
        body.replacen(INDEX, "<h4 data-label=\"text--secondary\">Previous 2025-26 honorees</h4>", 1),
        body.replacen(INDEX, "<h4>Unowned claim<span class=\"text--secondary\">Previous 2025-26 honorees</span></h4>", 1),
        body.replacen(INDEX, "<h4><span class=\"text--secondary\">Previous 2025-26 honorees</span><span class=\"text--secondary\">Contrary</span></h4>", 1),
    ] {
        check!(status_only(changed.as_bytes()).is_err());
    }
    Ok(())
}

#[test]
fn no_index_preserves_existing_status_without_adding_a_school_year_requirement() -> TestResult {
    let body = std::str::from_utf8(STATUS)?
        .replacen(INDEX, "", 1)
        .replacen("May 15, 2026", "January 1, 1900", 1);
    let value = status_only(body.as_bytes())?;
    check!(eq; value.day.as_str(), "1900-01-01");
    check!(eq; value.subject.name(), "Zacchaeus Brocks");
    check!(census_domain::model::SchoolYear::from_date(value.day.as_str()).is_none());
    Ok(())
}
