use super::*;
use crate::net::capture_fixture::{serve, Fixture, Reply, TestResult, BODY, CAPTURED};
use crate::net::CaptureTransportError;
use crate::{CrawlError, ResultsCaptureContext};
use census_domain::model::SOURCE_ACQUISITION_PHASE;
use std::sync::Arc;
use tokio::net::TcpListener;

async fn captured(body: &[u8]) -> TestResult<(Fixture, ResultsCaptureContext, CapturedFetch)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let owner = fixture.context_owner(2026, false)?;
    let fetch = async {
        let ctx = fixture.context(&owner, false)?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
            crate::milesplit::fetch::fetch_owned_capture(
                &ctx,
                &fixture.reference()?,
                &ctx.fetch_options(),
            )
            .await?,
        )
    };
    let ((), capture) = tokio::try_join!(
        serve(listener, [Reply::ok(body)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    Ok((fixture, owner, capture))
}

fn resource_body() -> TestResult<Vec<u8>> {
    Ok(serde_json::to_vec(
        &serde_json::json!({"data": [], "providerExtension": vec![0; 20_001]}),
    )?)
}

#[tokio::test]
async fn sealed_interpretation_preserves_checked_bytes_and_metadata_through_body_release() -> TestResult {
    let (fixture, owner, capture) = captured(BODY).await?;
    let metadata = serde_json::to_vec(capture.capture())?;
    let body_pointer = capture.capture().body.as_ptr();
    let (occurrence, original) = match capture.provenance() {
        CaptureProvenanceView::Retained { occurrence, original } => {
            (occurrence.clone(), original.clone())
        }
        provenance => return Err(format!("expected retained capture: {provenance:?}").into()),
    };
    let result = OwnedMeetOutcome::interpret(capture, 725218, CaptureContext::DurableV1(&owner))?;
    let OwnedInterpretationOutcome::Interpreted(mut outcome) = result else {
        return Err(format!("expected parsed fixture: {result:?}").into());
    };
    check!(eq; outcome.capture().body.as_ptr(), body_pointer);
    check!(eq; outcome.capture().body, BODY);
    check!(eq; outcome.capture().fetched_at, CAPTURED);
    check!(eq; serde_json::to_vec(outcome.capture())?, metadata);
    check!(eq; outcome.verdict(), &OwnedMeetVerdict::Parsed(parse_owned_meet(BODY, 725218)?));
    let verdict = serde_json::to_vec(outcome.verdict())?;
    outcome.release_body();
    check!(eq; outcome.capture().body.len(), 0);
    check!(eq; outcome.capture().body.capacity(), 0);
    check!(eq; serde_json::to_vec(outcome.capture())?, metadata);
    check!(eq; serde_json::to_vec(outcome.verdict())?, verdict);
    check!(eq; outcome.provenance(), CaptureProvenanceView::Retained {
        occurrence: &occurrence, original: &original,
    });
    let raw = outcome.into_capture();
    check!(eq; raw.body.len(), 0);
    check!(eq; serde_json::to_vec(&raw)?, metadata);
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn sealed_interpretation_resource_returns_the_original_retained_envelope() -> TestResult {
    let body = resource_body()?;
    let (fixture, owner, capture) = captured(&body).await?;
    let metadata = serde_json::to_vec(capture.capture())?;
    let body_pointer = capture.capture().body.as_ptr();
    let before = fixture.store.journal_payloads(SOURCE_ACQUISITION_PHASE)?;
    let result = OwnedMeetOutcome::interpret(capture, 725218, CaptureContext::DurableV1(&owner))?;
    let OwnedInterpretationOutcome::Resource { capture, failure } = result else {
        return Err(format!("expected retained resource failure: {result:?}").into());
    };
    match failure {
        OwnedParseResourceFailure::Limit {
            resource: crate::milesplit::OwnedParseResource::ArrayValues,
            locator: crate::milesplit::OwnedParseLocator::Byte { offset },
            requested,
            limit,
            retention: crate::milesplit::OwnedCaptureRetention::Unrecorded,
        } => {
            check!(eq; (requested, limit), (20_001, 20_000));
            check!(eq; body.get(offset), Some(&b'0'));
        }
        failure => return Err(format!("unexpected resource origin: {failure:?}").into()),
    }
    check!(eq; capture.capture().body.as_ptr(), body_pointer);
    check!(eq; capture.capture().body, body);
    check!(eq; serde_json::to_vec(capture.capture())?, metadata);
    check!(matches!(capture.provenance(), CaptureProvenanceView::Retained { .. }));
    let raw = capture.into_capture();
    check!(eq; raw.body.as_ptr(), body_pointer);
    check!(eq; raw.body, body);
    check!(eq; serde_json::to_vec(&raw)?, metadata);
    check!(eq; fixture.store.journal_payloads(SOURCE_ACQUISITION_PHASE)?, before);
    check!(eq; fixture.store.journal_keys(super::super::OWNED_MEET_PHASE)?, Vec::<String>::new());
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn sealed_interpretation_refuses_historical_authority_before_resource_parsing() -> TestResult {
    let body = resource_body()?;
    let (fixture, owner, retained) = captured(&body).await?;
    let historical = CapturedFetch::historical(retained.into_capture());
    let before = fixture.store.journal_payloads(SOURCE_ACQUISITION_PHASE)?;
    let result = OwnedMeetOutcome::interpret(historical, 725218, CaptureContext::DurableV1(&owner));
    check!(matches!(result, Err(CrawlError::Capture(CaptureTransportError::CacheOccurrenceMismatch))));
    check!(eq; fixture.store.journal_payloads(SOURCE_ACQUISITION_PHASE)?, before);
    check!(eq; fixture.store.journal_keys(super::super::OWNED_MEET_PHASE)?, Vec::<String>::new());
    check!(eq; fixture.store.journal_keys(super::super::OWNED_CAPTURE_PHASE)?, Vec::<String>::new());
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn sealed_interpretation_refuses_retained_authority_in_historical_context() -> TestResult {
    let body = resource_body()?;
    let (fixture, _owner, capture) = captured(&body).await?;
    let before = fixture.store.journal_payloads(SOURCE_ACQUISITION_PHASE)?;
    let result = OwnedMeetOutcome::interpret(capture, 725218, CaptureContext::LegacyMetadata);
    check!(matches!(result, Err(CrawlError::Capture(CaptureTransportError::CacheOccurrenceMismatch))));
    check!(eq; fixture.store.journal_payloads(SOURCE_ACQUISITION_PHASE)?, before);
    check!(eq; fixture.store.journal_keys(super::super::OWNED_MEET_PHASE)?, Vec::<String>::new());
    check!(eq; fixture.store.journal_keys(super::super::OWNED_CAPTURE_PHASE)?, Vec::<String>::new());
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn erased_capture_can_only_reenter_as_historical_unknown() -> TestResult {
    let (fixture, _owner, capture) = captured(BODY).await?;
    let raw = capture.into_capture();
    let metadata = serde_json::to_vec(&raw)?;
    let result = OwnedMeetOutcome::interpret(
        CapturedFetch::historical(raw),
        725218,
        CaptureContext::LegacyMetadata,
    )?;
    let OwnedInterpretationOutcome::Interpreted(outcome) = result else {
        return Err(format!("expected historical interpretation: {result:?}").into());
    };
    check!(eq; outcome.provenance(), CaptureProvenanceView::HistoricalAttemptUnknown);
    check!(eq; outcome.capture().body, BODY);
    check!(eq; serde_json::to_vec(outcome.capture())?, metadata);
    check!(eq; outcome.verdict(), &OwnedMeetVerdict::Parsed(parse_owned_meet(BODY, 725218)?));
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn synthetic_historical_verdict_never_grants_durable_capture_authority() -> TestResult {
    let (fixture, owner, capture) = captured(BODY).await?;
    let metadata = serde_json::to_vec(capture.capture())?;
    let before = fixture.store.journal_payloads(SOURCE_ACQUISITION_PHASE)?;
    let outcome = OwnedMeetOutcome::historical_fixture(
        capture.into_capture(),
        OwnedMeetVerdict::Refused { status: 418 },
    );
    check!(eq; outcome.provenance(), CaptureProvenanceView::HistoricalAttemptUnknown);
    check!(eq; outcome.verdict(), &OwnedMeetVerdict::Refused { status: 418 });
    check!(eq; outcome.capture().status, 200);
    check!(eq; outcome.capture().body, BODY);
    check!(eq; serde_json::to_vec(outcome.capture())?, metadata);
    let result = outcome.require_capture_context(CaptureContext::DurableV1(&owner));
    check!(matches!(result, Err(CrawlError::Capture(CaptureTransportError::CacheOccurrenceMismatch))));
    check!(eq; fixture.store.journal_payloads(SOURCE_ACQUISITION_PHASE)?, before);
    check!(eq; fixture.store.journal_keys(super::super::OWNED_MEET_PHASE)?, Vec::<String>::new());
    check!(eq; fixture.store.journal_keys(super::super::OWNED_CAPTURE_PHASE)?, Vec::<String>::new());
    check!(eq; fixture.count(), 1);
    Ok(())
}
