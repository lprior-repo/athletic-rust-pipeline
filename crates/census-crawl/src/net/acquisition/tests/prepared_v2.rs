use super::*;
use crate::net::{EligiblePublicationOutcome, EligiblePublicationSkip, PreparedRecoveryOutcome};
use census_domain::model::*;
use census_store::{
    AcquisitionReadBudget, ExactBodySource, ExistingRecoveryRoute, PreparedRecoveryBudget,
    ResultsCaptureOwnerV1,
};

mod acceptance;
mod boundaries;
mod diagnostic;
mod publication;
mod refusal;
mod support;
use support::*;

#[tokio::test]
async fn fixed_v2_exact_recovery_moves_the_genuine_body_into_real_parser_once() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    publish(&fixture, &prepared)?;
    let pointer_expected = BODY;
    let mut budget = prepared.budget()?;
    let outcome = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    let PreparedRecoveryOutcome::Retained {
        captured,
        terminal,
        receipt,
        eligible,
    } = outcome
    else {
        return Err("retained outcome".into());
    };
    check!(matches!(eligible, EligiblePublicationOutcome::Published));
    check!(eq; captured.capture().body, pointer_expected);
    check!(eq; captured.capture().fetched_at, CAPTURED);
    check!(eq; terminal.attempt(), prepared.target.attempt());
    check!(eq; fixture.store.receipt(&receipt.operation)?, Some(receipt.clone()));
    check!(eq; receipt.digest, serialized_digest(&terminal)?);
    let pointer = captured.capture().body.as_ptr();
    let before = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    let context = crate::ResultsCaptureContext::from_prepared_recovery(
        prepared.owner,
        prepared.target,
        prepared.work,
        captured,
    )?;
    let ctx = fixture.context(&context, false)?;
    let interpreted =
        crate::milesplit::fetch_owned_meet(&ctx, &fixture.reference()?, &ctx.fetch_options())
            .await?;
    check!(eq; interpreted.capture().body.as_ptr(), pointer);
    check!(matches!(
        interpreted.verdict(),
        crate::milesplit::OwnedMeetVerdict::Parsed(_)
    ));
    let alias =
        crate::milesplit::fetch_owned_meet(&ctx, &fixture.reference()?, &ctx.fetch_options()).await;
    check!(matches!(
        alias,
        Err(crate::CrawlError::Capture(
            crate::net::CaptureTransportError::DispatchConsumed
        ))
    ));
    check!(eq; fixture.count(), 0);
    check!(eq; fixture.store.journal_keys(CaptureProtocolVersion::V2.phase())?, before);
    Ok(())
}

#[tokio::test]
async fn fixed_v2_restoration_uses_exact_metadata_and_store_final_verification() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Compact)?;
    let (body, metadata) = publish(&fixture, &prepared)?;
    std::fs::remove_file(&metadata)?;
    let mut budget = prepared.budget()?;
    let recovered = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::NoneSupplied,
    )?;
    let PreparedRecoveryOutcome::Retained {
        captured, eligible, ..
    } = recovered
    else {
        return Err("retained restoration".into());
    };
    check!(matches!(
        eligible,
        EligiblePublicationOutcome::Skipped(EligiblePublicationSkip::NonRoundtrippingMetadata)
    ));
    check!(eq; std::fs::read(metadata)?, prepared.metadata);
    check!(eq; std::fs::read(body)?, BODY);
    check!(eq; captured.capture().body, BODY);
    check!(eq; fixture.count(), 0);
    Ok(())
}

#[tokio::test]
async fn no_op_restoration_cannot_finish_and_failed_candidate_cannot_close() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    let (_, metadata) = publish(&fixture, &prepared)?;
    std::fs::remove_file(metadata)?;
    let mut budget = prepared.budget()?;
    let read = fixture
        .store
        .begin_prepared_recovery(&prepared.owner, &prepared.target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(mut session) = read else {
        return Err("unresolved restoration".into());
    };
    session.prepare_restore(ExactBodySource::NoneSupplied)?;
    {
        let (_fence, _prepared, _body, _io) = session.restoration_io()?;
    }
    check!(session.verify_restored().is_err());
    check!(session.finish().is_err());
    let fixture = Fixture::new_v2(&listener)?;
    let prepared = prepare(&fixture, BODY, Encoding::Canonical)?;
    let candidate = fixture.root.path().join("wrong.body");
    std::fs::write(&candidate, b"wrong")?;
    let mut budget = prepared.budget()?;
    let before = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?;
    let recovered = fixture.fetcher.recover_prepared_target(
        &fixture.store,
        &prepared.owner,
        &prepared.target,
        &mut budget,
        ExactBodySource::SuppliedLocalCopy(&candidate),
    );
    check!(recovered.is_err());
    check!(eq; fixture.store.journal_keys(CaptureProtocolVersion::V2.phase())?, before);
    check!(eq; fixture.count(), 0);
    Ok(())
}
