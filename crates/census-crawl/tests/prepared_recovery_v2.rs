#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;
#[path = "prepared_recovery_v2/support.rs"]
mod support;

use census_crawl::net::{
    CaptureProvenanceView, CaptureTransportError, EligiblePublicationOutcome, FetchError,
    PreparedRecoveryOutcome,
};
use census_crawl::{milesplit, CrawlError, ResultsCaptureContext};
use census_domain::model::*;
use census_store::AcquisitionTerminal;
use support::{reference, Fixture, TestResult, BODY, CAPTURED};

#[tokio::test]
async fn prepared_commit_retains_exact_receipt_and_moves_body_into_adapter_once() -> TestResult {
    let fixture = Fixture::new()?;
    let prepared = fixture.prepare()?;
    fixture.publish(&prepared)?;
    let PreparedRecoveryOutcome::Retained {
        captured,
        terminal,
        receipt,
        eligible,
    } = fixture.recover(&prepared)?
    else {
        return Err("expected retained Prepared capture".into());
    };
    check!(matches!(eligible, EligiblePublicationOutcome::Published));
    check!(eq; terminal.protocol(), CaptureProtocolVersion::V2);
    check!(eq; terminal.attempt(), prepared.target.attempt());
    check!(eq; terminal.terminal(), &AcquisitionTerminal::ResponseRetained {
        prepared_digest: prepared.target.prepared_digest().clone(),
        disposition: CaptureDispositionV1::ReusableResponse,
    });
    check!(eq; receipt.digest, serialized_digest(&terminal)?);
    check!(eq; fixture.store.receipt(&receipt.operation)?, Some(receipt.clone()));
    check!(eq; captured.capture().body, BODY);
    check!(eq; captured.capture().fetched_at, CAPTURED);
    let occurrence = terminal.occurrence()?;
    check!(eq; captured.provenance(), CaptureProvenanceView::Retained {
        occurrence: &occurrence, original: &occurrence,
    });
    let before = fixture
        .store
        .journal_payloads(CaptureProtocolVersion::V2.phase())?;
    let PreparedRecoveryOutcome::Retained {
        terminal: replayed,
        receipt: replay_receipt,
        ..
    } = fixture.recover(&prepared)?
    else {
        return Err("expected retained replay".into());
    };
    check!(eq; replayed, terminal);
    check!(eq; replay_receipt, receipt);
    check!(eq; fixture.store.journal_payloads(CaptureProtocolVersion::V2.phase())?, before);
    let pointer = captured.capture().body.as_ptr();
    let capture = ResultsCaptureContext::from_prepared_recovery(
        prepared.owner,
        prepared.target,
        prepared.work,
        captured,
    )?;
    let context = fixture.context(&capture)?;
    let reference = reference()?;
    let parsed =
        milesplit::fetch_owned_meet(&context, &reference, &context.fetch_options()).await?;
    check!(matches!(
        parsed.verdict(),
        milesplit::OwnedMeetVerdict::Parsed(_)
    ));
    check!(eq; parsed.capture().body, BODY);
    check!(eq; parsed.capture().body.as_ptr(), pointer);
    check!(eq; parsed.provenance(), CaptureProvenanceView::Retained {
        occurrence: &occurrence, original: &occurrence,
    });
    check!(matches!(
        milesplit::fetch_owned_meet(&context, &reference, &context.fetch_options()).await,
        Err(CrawlError::Capture(CaptureTransportError::DispatchConsumed))
    ));
    check!(eq; fixture.store.journal_payloads(CaptureProtocolVersion::V2.phase())?, before);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn missing_prepared_body_records_exact_loss_and_replay_cannot_resurrect_it() -> TestResult {
    let fixture = Fixture::new()?;
    let prepared = fixture.prepare()?;
    let body = fixture.publish(&prepared)?;
    std::fs::remove_file(body)?;
    let PreparedRecoveryOutcome::RetentionFailed { terminal, receipt } =
        fixture.recover(&prepared)?
    else {
        return Err("expected missing Prepared retention failure".into());
    };
    let decision = MissingPreparedDecisionV1::new(
        prepared.target.clone(),
        MissingPreparedAssessmentV1::new(MissingLocation::BodyLeaf, MissingMetadataV1::Exact)?,
    );
    check!(eq; terminal.protocol(), CaptureProtocolVersion::V2);
    check!(eq; terminal.attempt(), prepared.target.attempt());
    check!(eq; terminal.terminal(), &AcquisitionTerminal::PreparedRetentionFailed {
        prepared_digest: prepared.target.prepared_digest().clone(), decision,
    });
    check!(eq; receipt.digest, serialized_digest(&terminal)?);
    check!(eq; fixture.store.receipt(&receipt.operation)?, Some(receipt.clone()));
    let before = fixture
        .store
        .journal_payloads(CaptureProtocolVersion::V2.phase())?;
    fixture.publish(&prepared)?;
    let PreparedRecoveryOutcome::RetentionFailed {
        terminal: replayed,
        receipt: replay_receipt,
    } = fixture.recover(&prepared)?
    else {
        return Err("original loss must stay terminal after body reappears".into());
    };
    check!(eq; replayed, terminal);
    check!(eq; replay_receipt, receipt);
    check!(eq; fixture.store.journal_payloads(CaptureProtocolVersion::V2.phase())?, before);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn refused_eligible_publication_keeps_committed_body_terminal_and_receipt() -> TestResult {
    let fixture = Fixture::new()?;
    let prepared = fixture.prepare()?;
    fixture.publish(&prepared)?;
    let reservations = fixture.store.http_cache_dir().join("archive/reservations");
    std::fs::create_dir_all(&reservations)?;
    let invalid = reservations.join(format!("{}-{}", "c".repeat(64), "d".repeat(64)));
    std::fs::write(&invalid, b"not\ncounts")?;
    let PreparedRecoveryOutcome::Retained {
        captured,
        terminal,
        receipt,
        eligible,
    } = fixture.recover(&prepared)?
    else {
        return Err("eligible failure must preserve retained result".into());
    };
    check!(matches!(eligible,
        EligiblePublicationOutcome::RefusedBeforeReplace(FetchError::Cache { source, .. })
            if source.kind() == std::io::ErrorKind::InvalidData
    ));
    check!(eq; captured.capture().body, BODY);
    check!(eq; terminal.attempt(), prepared.target.attempt());
    check!(eq; terminal.terminal(), &AcquisitionTerminal::ResponseRetained {
        prepared_digest: prepared.target.prepared_digest().clone(),
        disposition: CaptureDispositionV1::ReusableResponse,
    });
    check!(eq; receipt.digest, serialized_digest(&terminal)?);
    check!(eq; fixture.store.receipt(&receipt.operation)?, Some(receipt.clone()));
    check!(eq; std::fs::read(&invalid)?, b"not\ncounts");
    let before = fixture
        .store
        .journal_payloads(CaptureProtocolVersion::V2.phase())?;
    let PreparedRecoveryOutcome::Retained {
        terminal: replayed,
        receipt: replay_receipt,
        ..
    } = fixture.recover(&prepared)?
    else {
        return Err("committed truth must replay after publication refusal".into());
    };
    check!(eq; replayed, terminal);
    check!(eq; replay_receipt, receipt);
    check!(eq; fixture.store.journal_payloads(CaptureProtocolVersion::V2.phase())?, before);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}
