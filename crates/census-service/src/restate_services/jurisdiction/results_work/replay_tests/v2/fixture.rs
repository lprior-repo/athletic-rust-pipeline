use super::*;
use crate::restate_services::jurisdiction::capture_admission::ResultsCaptureExecution;
use census_domain::model::{
    CaptureDispositionV1, ConditionalRequestV1, DispatchRequestV1, PreparedCaptureInputV1,
    ResponseObservationV1, SourceMeetRef,
};
use sha2::{Digest, Sha256};

pub(super) struct Prepared {
    pub(super) attempt: census_domain::model::AcquisitionAttemptId,
    pub(super) input: PreparedCaptureInputV1,
    pub(super) body: Vec<u8>,
}

pub(super) fn seed_prepared(fixture: &Fixture) -> TestResult<Prepared> {
    let ResultsCaptureExecution::FreshQualification { writer, owner, .. } = fixture.capture()?
    else {
        return Err("v2 fixture owner absent".into());
    };
    let reference =
        census_crawl::milesplit::ResultSetRef::parse(super::super::fixture::RESULTS_URL)
            .ok_or("fixture reference")?;
    let request = census_crawl::milesplit::owned_meet_request(
        &fixture.fetcher,
        &reference,
        &Default::default(),
    )?;
    let work = owner.for_owned_meet(census_domain::UsJurisdiction::Michigan, "725218", request)?;
    let grant = writer.continuation_for_owned_meet(
        &fixture.store,
        &owner,
        census_domain::UsJurisdiction::Michigan,
        "725218",
    )?;
    let owned = fixture
        .store
        .register_acquisition_work(&owner, work, grant)?;
    let reserved = fixture.store.reserve_acquisition_attempt(owned)?;
    let url = super::super::fixture::owned_url();
    let dispatched = fixture
        .store
        .arm_acquisition_dispatch(
            reserved,
            DispatchRequestV1::new(&url, ConditionalRequestV1::Unconditional)?,
        )?
        .into_observation();
    let response = ResponseObservationV1::new(200, &url, "2026-10-14T05:00:00Z", Vec::new())?;
    let observed = fixture
        .store
        .record_acquisition_response(dispatched, response)?;
    let body = br#"{"data":[]}"#.to_vec();
    let digest = format!("{:x}", Sha256::digest(&body));
    let metadata = serde_json::to_vec_pretty(&serde_json::json!({
        "url":url,"response_url":url,"method":"GET","status":200,"content_digest":digest,
        "bytes":body.len(),"fetched_at":"2026-10-14T05:00:00Z",
    }))?;
    let input = PreparedCaptureInputV1::new(&digest, u64::try_from(body.len())?, metadata)?;
    let prepared = fixture.store.prepare_acquisition_capture(
        observed,
        input.clone(),
        CaptureDispositionV1::for_http_status(200)?,
    )?;
    Ok(Prepared {
        attempt: prepared.attempt_id().clone(),
        input,
        body,
    })
}

pub(super) fn publish_then_corrupt(fixture: &Fixture, prepared: &Prepared) -> TestResult {
    let body_dir = fixture.store.http_cache_dir().join("archive/bodies");
    let meta_dir = fixture
        .store
        .http_cache_dir()
        .join("archive/captures")
        .join(prepared.input.content_digest().as_str());
    std::fs::create_dir_all(&body_dir)?;
    std::fs::create_dir_all(&meta_dir)?;
    let body = body_dir.join(format!("{}.body", prepared.input.content_digest()));
    std::fs::write(&body, &prepared.body)?;
    std::fs::write(
        meta_dir.join(format!("{}.meta.json", prepared.input.metadata_digest())),
        prepared.input.metadata_bytes(),
    )?;
    fixture.seed_failed_attempt()?;
    std::fs::write(body, b"corrupted after later terminal")?;
    Ok(())
}

pub(super) fn change_candidates_and_discovery(fixture: &Fixture) -> TestResult {
    let mut meet = super::super::fixture::meet();
    meet.id = SourceMeetRef::row_id("milesplit", "999");
    meet.source_meet_id = "999".to_owned();
    meet.results_url = "https://mi.milesplit.com/meets/999/results".to_owned();
    fixture
        .store
        .append(census_store::Table::SourceMeets, &meet)?;
    crate::capture_cache::seed(
        fixture.fetcher.cache_dir(),
        super::super::fixture::RESULTS_URL,
        br#"<script>meetResultFiles = [{"id":999,"name":"changed","isMeetPro":0}];</script>"#,
        "2026-10-14T06:00:00Z",
        &[],
    )?;
    Ok(())
}

pub(super) fn pending_overflow(fixture: &Fixture) -> TestResult {
    for index in 0..2049 {
        let mut meet = super::super::fixture::meet();
        meet.id = format!("unknown-date-{index:04}");
        meet.date = None;
        meet.results_url = format!("{index:04}{}", "x".repeat(4092));
        fixture
            .store
            .append(census_store::Table::SourceMeets, &meet)?;
    }
    Ok(())
}
