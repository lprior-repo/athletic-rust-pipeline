use super::*;
use crate::net::acquisition::CaptureProvenance;

#[test]
fn foreign_store_original_with_identical_source_bytes_cannot_issue_child_authority() -> TestResult {
    let first = fixture(DIRECTORY, 200)?;
    let second = fixture(DIRECTORY, 200)?;
    let evidence = first.evidence()?;
    let before_first = StoreWitness::read(&first.store)?;
    let before_second = StoreWitness::read(&second.store)?;
    check!(ne; first.reference.attempt().store_identity(),
        second.reference.attempt().store_identity());
    check!(assess_directory_roster_child(&evidence, &second.captured, TEAM).is_err());
    check!(eq; before_first, StoreWitness::read(&first.store)?);
    check!(eq; before_second, StoreWitness::read(&second.store)?);
    Ok(())
}

#[test]
fn repaired_outer_body_fields_cannot_replace_the_retained_original() -> TestResult {
    let mut f = fixture(DIRECTORY, 200)?;
    let evidence = f.evidence()?;
    let before = StoreWitness::read(&f.store)?;
    let byte = f.captured.capture.body.first_mut().ok_or("retained byte")?;
    *byte ^= 1;
    f.captured.capture.content_digest = crate::net::cache::content_digest(&f.captured.capture.body);
    check!(assess_directory_roster_child(&evidence, &f.captured, TEAM).is_err());
    check!(eq; before, StoreWitness::read(&f.store)?);
    Ok(())
}

#[test]
fn equivalent_metadata_spelling_and_foreign_occurrence_do_not_rebind_evidence() -> TestResult {
    let mut f = fixture(DIRECTORY, 200)?;
    let evidence = f.evidence()?;
    let before = StoreWitness::read(&f.store)?;
    let mut metadata = f.captured.original_metadata.take().ok_or("metadata")?;
    metadata.push(b' ');
    f.captured.original_metadata = Some(metadata);
    check!(matches!(
        assess_directory_roster_child(&evidence, &f.captured, TEAM),
        Err(CrawlError::Store(census_store::StoreError::Refused { detail }))
            if detail == "owned directory retained binding differs"
    ));
    let foreign = AcquisitionOccurrenceRefV1::new(
        AcquisitionAttemptId::new(
            f.reference.attempt().store_identity().clone(),
            f.reference.attempt().work_id().clone(),
            AttemptOrdinal::new(2)?,
        ),
        AcquisitionDigest::parse(&"d".repeat(64))?,
    );
    f.captured.original_metadata = Some(evidence.retained().prepared().metadata_bytes().to_vec());
    f.captured.provenance = CaptureProvenance::retained(foreign, f.reference.clone());
    check!(assess_directory_roster_child(&evidence, &f.captured, TEAM).is_err());
    check!(eq; before, StoreWitness::read(&f.store)?);
    Ok(())
}

#[test]
fn wrong_method_final_url_status_and_missing_provenance_refuse() -> TestResult {
    let mut f = fixture(DIRECTORY, 200)?;
    let evidence = f.evidence()?;
    let before = StoreWitness::read(&f.store)?;
    f.captured.capture.method = "POST".into();
    check!(assess_directory_roster_child(&evidence, &f.captured, TEAM).is_err());
    f.captured.capture.method = "GET".into();
    f.captured.capture.response_url = Some("https://co.milesplit.com/teams".into());
    check!(assess_directory_roster_child(&evidence, &f.captured, TEAM).is_err());
    f.captured.capture.response_url = Some(URL.into());
    f.captured.capture.status = 403;
    check!(assess_directory_roster_child(&evidence, &f.captured, TEAM).is_err());
    f.captured.capture.status = 200;
    f.captured.original_metadata = None;
    check!(assess_directory_roster_child(&evidence, &f.captured, TEAM).is_err());
    check!(eq; before, StoreWitness::read(&f.store)?);
    Ok(())
}

#[test]
fn valid_retention_of_hidden_duplicated_and_foreign_literal_rows_does_not_issue_authority(
) -> TestResult {
    let body = std::str::from_utf8(DIRECTORY)?;
    let row = body.get(91718..91989).ok_or("genuine row")?;
    let variants = [
        body.replacen(row, &format!("<div hidden>{row}</div>"), 1),
        body.replacen(row, &format!("{row}{row}"), 1),
        body.replacen(
            "teams/12439-detroit-catholic-central",
            "teams/999999-detroit-catholic-central",
            1,
        ),
        body.replacen(
            "https://mi.milesplit.com/teams/12439-detroit-catholic-central",
            "https://co.milesplit.com/teams/12439-detroit-catholic-central",
            1,
        ),
    ];
    for changed in variants {
        check!(ne; changed.as_bytes(), DIRECTORY);
        let f = fixture(changed.as_bytes(), 200)?;
        let evidence = f.evidence()?;
        let before = StoreWitness::read(&f.store)?;
        check!(assess_directory_roster_child(&evidence, &f.captured, TEAM).is_err());
        check!(eq; before, StoreWitness::read(&f.store)?);
    }
    Ok(())
}

#[test]
fn retained_non_utf8_empty_and_non_success_sources_do_not_issue_authority() -> TestResult {
    for body in [b"<html>\xff</html>".as_slice(), b"".as_slice()] {
        let f = fixture(body, 200)?;
        let evidence = f.evidence()?;
        let before = StoreWitness::read(&f.store)?;
        check!(assess_directory_roster_child(&evidence, &f.captured, TEAM).is_err());
        check!(eq; before, StoreWitness::read(&f.store)?);
    }
    let f = fixture(DIRECTORY, 403)?;
    let before = StoreWitness::read(&f.store)?;
    check!(f.evidence().is_err());
    check!(eq; before, StoreWitness::read(&f.store)?);
    Ok(())
}

#[test]
fn literal_observed_mime_cannot_be_replaced_by_equivalent_or_missing_metadata_type() -> TestResult {
    let mut f = fixture(DIRECTORY, 200)?;
    let evidence = f.evidence()?;
    let before = StoreWitness::read(&f.store)?;
    for value in [None, Some("Text/HTML"), Some("application/json")] {
        f.captured.capture.content_type = value.map(str::to_owned);
        check!(matches!(
            assess_directory_roster_child(&evidence, &f.captured, TEAM),
            Err(CrawlError::Store(census_store::StoreError::Refused { detail }))
                if detail == "owned directory observed MIME differs"
        ));
    }
    check!(eq; before, StoreWitness::read(&f.store)?);
    Ok(())
}

#[test]
fn normalized_duplicate_or_non_utf8_content_type_is_not_a_singleton_html_observation() -> TestResult
{
    let f = fixture(DIRECTORY, 200)?;
    let observed = f
        .captured
        .observed_http()?
        .ok_or("controlled observed HTTP")?;
    for (duplicate, value) in [(true, b"text/html".to_vec()), (false, vec![255])] {
        let mut document = serde_json::to_value(&observed)?;
        let headers = document
            .pointer_mut("/hops/0/response/headers")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or("controlled response headers")?;
        if !duplicate {
            headers.retain(|header| {
                header.get("name").and_then(serde_json::Value::as_str) != Some("content-type")
            });
        }
        headers.push(serde_json::to_value(ObservedHttpHeaderV1::new(
            "content-type",
            value,
        )?)?);
        let changed: NormalizedObservedHttpV1 = serde_json::from_value(document)?;
        check!(require_observed_type(f.captured.capture(), &changed).is_err());
    }
    Ok(())
}
