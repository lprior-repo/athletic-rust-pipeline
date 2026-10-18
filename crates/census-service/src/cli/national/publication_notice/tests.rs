use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn contact(jurisdiction: UsJurisdiction, status: ContactStatus) -> ContactSummary {
    ContactSummary {
        jurisdiction,
        identity: "retained-contact-workflow".into(),
        status,
        source_rows: Some(0),
        errors: None,
        unfinished: None,
        sources: Vec::new(),
    }
}

#[test]
fn noncomplete_contact_statuses_never_claim_publication() -> TestResult {
    for status in [
        ContactStatus::Partial,
        ContactStatus::Terminal,
        ContactStatus::Exhausted,
        ContactStatus::Interrupted,
        ContactStatus::HandlerFailed,
    ] {
        let notice = describe(
            &[contact(UsJurisdiction::Wisconsin, status)],
            &[UsJurisdiction::Wisconsin],
        );
        check!(notice.starts_with("publication incomplete:"));
        check!(notice.contains(&format!("WI contacts {status:?}")));
        check!(notice.contains("does not certify a workbook or seal"));
    }
    Ok(())
}

#[test]
fn complete_contact_measurement_is_not_a_publication_certificate() -> TestResult {
    check!(eq; describe(
        &[contact(UsJurisdiction::Wisconsin, ContactStatus::Complete)],
        &[UsJurisdiction::Wisconsin],
    ), UNVERIFIED);
    Ok(())
}

#[test]
fn missing_duplicate_foreign_and_oversized_contact_measurements_are_unconfirmed() -> TestResult {
    for contacts in [
        Vec::new(),
        vec![contact(UsJurisdiction::Michigan, ContactStatus::Complete)],
        vec![contact(UsJurisdiction::Wisconsin, ContactStatus::Complete); 2],
        vec![contact(UsJurisdiction::Wisconsin, ContactStatus::Complete); 50],
    ] {
        check!(describe(&contacts, &[UsJurisdiction::Wisconsin])
            .starts_with("publication unconfirmed:"));
    }
    let duplicate = vec![contact(UsJurisdiction::Wisconsin, ContactStatus::Complete); 2];
    check!(describe(
        &duplicate,
        &[UsJurisdiction::Wisconsin, UsJurisdiction::Michigan]
    )
    .starts_with("publication unconfirmed:"));
    Ok(())
}
