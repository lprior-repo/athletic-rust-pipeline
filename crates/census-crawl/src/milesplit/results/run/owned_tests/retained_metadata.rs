use super::*;
use crate::milesplit::{
    read_retained_owned_result, read_retained_result_metadata, CaptureReadBudget,
    OwnedAcquisitionRef, RawMetadataArchiveRef, RetainedMetadataError, RetainedPerformanceDate,
    RetainedResultDivision, VerifiedRetainedResultMetadata, OWNED_CAPTURE_PHASE,
};
use crate::net::FetchOutcome;
use census_domain::model::ProfileDate;
use serde_json::Value;

mod attacks;
mod fixture;
mod session;
use fixture::*;

#[tokio::test]
async fn authentic_troy_retains_owned_date_without_inventing_high_school_status() -> TestResult {
    let fixture = Fixture::prepare(TROY).await?;
    let before = fixture.store.journal_payloads(OWNED_CAPTURE_PHASE)?;
    let source = fixture.read()?;
    check!(eq; source.result().row().result_id, 201782263);
    check!(eq; source.result().row().source_athlete.id, "14222592");
    check!(
        eq;
        source.date(),
        &RetainedPerformanceDate::Exact(ProfileDate::parse("2026-03-27")?)
    );
    check!(eq; source.division(), RetainedResultDivision::Unknown);
    check!(eq; source.capture().body, FEMALE_RAW);
    check!(
        eq;
        source.store_identity(),
        &fixture.store.snapshot().store_identity()?
    );
    check!(eq; fixture.store.journal_payloads(OWNED_CAPTURE_PHASE)?, before);
    Ok(())
}

#[tokio::test]
async fn synthetic_explicit_division_is_bound_to_original_owned_row_and_real_date() -> TestResult {
    let mut synthetic: Value = serde_json::from_slice(TROY)?;
    let row = synthetic
        .get_mut("data")
        .and_then(Value::as_array_mut)
        .and_then(|rows| rows.first_mut())
        .ok_or("synthetic row")?;
    row.as_object_mut()
        .ok_or("synthetic object")?
        .insert("divisionName".into(), json!("High School"));
    let bytes = serde_json::to_vec(&synthetic)?;
    let fixture = Fixture::prepare(&bytes).await?;
    let source = fixture.read()?;
    check!(eq; source.division(), RetainedResultDivision::HighSchool);
    check!(eq; source.result().acquisition().body(), bytes);
    check!(
        eq;
        source.date(),
        &RetainedPerformanceDate::Exact(ProfileDate::parse("2026-03-27")?)
    );
    Ok(())
}

#[tokio::test]
async fn multi_day_and_malformed_end_dates_do_not_become_the_first_meet_day() -> TestResult {
    let fixture = Fixture::prepare(TROY).await?;
    for end in ["2026-03-28", "2026-03-26", "2026-03-27invalid", "2026"] {
        let raw = std::str::from_utf8(FEMALE_RAW)?.replace(
            "\"endDate\": \"2026-03-27\"",
            &format!("\"endDate\": \"{end}\""),
        );
        check!(ne; raw.as_bytes(), FEMALE_RAW);
        let archive = fixture.archive(raw.as_bytes(), None)?;
        let source = fixture.read_archive(&archive)?;
        check!(eq; source.date(), &RetainedPerformanceDate::Unresolved);
    }
    Ok(())
}

#[tokio::test]
async fn raw_and_owned_objects_must_exist_together_in_the_same_snapshot() -> TestResult {
    let fixture = Fixture::prepare(TROY).await?;
    let view = fixture.store.snapshot();
    let owned = read_retained_owned_result(&view, &fixture.owned, "data[0]", budget()?)?;
    let (_dir, foreign, fetcher, reference) = setup()?;
    foreign.store_identity()?;
    let capture = raw_capture(FEMALE_RAW, &reference, None);
    super::super::capture::archive_metadata(&context(&foreign, &fetcher)?, &reference, &capture)?;
    check!(matches!(
        read_retained_result_metadata(
            &foreign.snapshot(),
            &reference,
            &fixture.raw,
            &owned,
            budget()?
        ),
        Err(RetainedMetadataError::OwnedResult(_))
    ));
    Ok(())
}
