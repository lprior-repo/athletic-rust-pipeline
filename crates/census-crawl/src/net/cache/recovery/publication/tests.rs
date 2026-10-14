use super::publish_recovered_eligible_held;
use crate::net::acquisition::tests::fixture::{injected, serve, Fixture, Reply, TestResult, BODY};
use crate::net::cache::archive_lock_test;
use crate::net::cache::{CacheMeta, Capture, EligibleCacheV2};
use census_domain::model::*;
use census_store::{
    AcquisitionReadBudget, ExistingRecoveryRoute, PreparedRecoveryBudget, ResultsCaptureOwnerV1,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::test(flavor = "current_thread")]
async fn raw_codec_envelope_over_limit_skips_before_busy_lock_and_preserves_committed_capture(
) -> TestResult {
    let fixture = archived_fixture().await?;
    let (owner, target, mut budget) = recovery_target(&fixture)?;
    let old = old_pair(&fixture)?;
    let lock = busy_lock(&old.metadata)?;
    let read = fixture
        .store
        .begin_prepared_recovery(&owner, &target, &mut budget)?
        .read_backing()?;
    let census_store::PreparedRecoveryRead::Unresolved(session) = read else {
        return Err("genuine unresolved Prepared session".into());
    };
    let mut retained = session.finish()?;
    let committed = Committed::read(&fixture)?;
    let archived = skipped_raw_envelope(&fixture, &mut retained, &target)?;
    archive_lock_test::require_held(&old.metadata.with_extension("json.lock"))?;
    old.unchanged()?;
    archived.unchanged()?;
    committed.unchanged(&fixture, retained.into_parts(), &target, &archived)?;
    drop(lock);
    check!(eq; fixture.count(), 1);
    Ok(())
}

fn skipped_raw_envelope(
    fixture: &Fixture,
    retained: &mut census_store::CommittedRetainedBody<'_>,
    target: &PreparedRecoveryTarget,
) -> TestResult<Pair> {
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&events);
    let _observer = archive_lock_test::observe(move |_, event| {
        observed.borrow_mut().push(event);
        Ok(())
    })?;
    let (fence, genuine_input, occurrence, io) = retained.publication_io()?;
    check!(eq; occurrence.attempt(), target.attempt());
    let archived = archived_pair(fixture, genuine_input)?;
    let raw_codec_input = raw_codec_input(fixture, genuine_input, occurrence)?;
    check!(raw_codec_input.metadata_digest() != genuine_input.metadata_digest());
    let outcome =
        publish_recovered_eligible_held(fence, &raw_codec_input, occurrence, u64::MAX, io);
    check!(matches!(
        outcome,
        super::super::EligiblePublicationOutcome::Skipped(
            super::super::EligiblePublicationSkip::EnvelopeTooLarge
        )
    ));
    check!(events.borrow().is_empty());
    Ok(archived)
}

async fn archived_fixture() -> TestResult<Fixture> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new_v2(&listener)?;
    fixture.stop_after_archive();
    let fetch = async {
        let outcome = fixture.acquire(2026, false).await?;
        check!(matches!(&outcome, Err(error) if injected(error)));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    fixture.resume_capture();
    Ok(fixture)
}

fn recovery_target(
    fixture: &Fixture,
) -> TestResult<(
    ResultsCaptureOwnerV1,
    PreparedRecoveryTarget,
    PreparedRecoveryBudget,
)> {
    let owner = fixture.owner(2026, CaptureCacheIntent::ReuseEligible)?;
    let reference = fixture.reference()?;
    let request = crate::milesplit::owned_meet_request(
        &fixture.fetcher,
        &reference,
        &crate::net::FetchOptions::default(),
    )?;
    let work = owner.for_owned_meet(reference.site.jurisdiction(), &reference.meet_id, request)?;
    let mut selection = AcquisitionReadBudget::for_selection();
    let ExistingRecoveryRoute::Prepared(target) =
        fixture
            .store
            .read_existing_recovery_target(&owner, &work, &mut selection)?
    else {
        return Err("genuine archived Prepared target".into());
    };
    let budget = PreparedRecoveryBudget::admit(selection.selection_usage(), 1)?;
    Ok((owner, target, budget))
}

fn raw_codec_input(
    fixture: &Fixture,
    genuine: &PreparedCaptureInputV1,
    occurrence: &AcquisitionOccurrenceRefV1,
) -> TestResult<PreparedCaptureInputV1> {
    let mut metadata: CacheMeta = serde_json::from_slice(genuine.metadata_bytes())?;
    metadata.content_type = Some(String::new());
    let baseline = Capture::from_meta(&metadata, fixture.fetcher.cache_dir())?
        .encoded
        .len();
    let fill = 65_536_usize
        .checked_sub(64)
        .and_then(|length| length.checked_sub(baseline))
        .ok_or("raw codec metadata edge length")?;
    metadata.content_type = Some("x".repeat(fill));
    let raw_metadata = Capture::from_meta(&metadata, fixture.fetcher.cache_dir())?.encoded;
    check!(eq; raw_metadata.len(), 65_472);
    check!(raw_metadata.len() <= 65_536);
    let envelope = EligibleCacheV2::new(metadata, occurrence.clone());
    let actual_envelope = serde_json::to_vec_pretty(&envelope)?;
    check!(actual_envelope.len() > 65_536);
    Ok(PreparedCaptureInputV1::new(
        genuine.content_digest().as_str(),
        genuine.content_bytes(),
        raw_metadata,
    )?)
}

struct Pair {
    body: PathBuf,
    metadata: PathBuf,
    body_bytes: Vec<u8>,
    metadata_bytes: Vec<u8>,
}

impl Pair {
    fn read((body, metadata): (PathBuf, PathBuf)) -> TestResult<Self> {
        Ok(Self {
            body_bytes: std::fs::read(&body)?,
            metadata_bytes: std::fs::read(&metadata)?,
            body,
            metadata,
        })
    }

    fn unchanged(&self) -> TestResult {
        check!(eq; std::fs::read(&self.body)?, self.body_bytes);
        check!(eq; std::fs::read(&self.metadata)?, self.metadata_bytes);
        Ok(())
    }
}

fn archived_pair(fixture: &Fixture, genuine: &PreparedCaptureInputV1) -> TestResult<Pair> {
    let root = fixture.store.http_cache_dir().join("archive");
    let digest = genuine.content_digest().as_str();
    let body = root.join("bodies").join(format!("{digest}.body"));
    let metadata = root
        .join("captures")
        .join(digest)
        .join(format!("{}.meta.json", genuine.metadata_digest().as_str()));
    let pair = Pair::read((body, metadata))?;
    check!(eq; pair.body_bytes, BODY);
    check!(eq; pair.metadata_bytes, genuine.metadata_bytes());
    Ok(pair)
}

fn old_pair(fixture: &Fixture) -> TestResult<Pair> {
    let (body, metadata) = fixture.cache_paths()?;
    std::fs::write(&body, b"old body")?;
    std::fs::write(&metadata, b"old metadata")?;
    Pair::read((body, metadata))
}

fn busy_lock(metadata: &std::path::Path) -> TestResult<std::fs::File> {
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(metadata.with_extension("json.lock"))?;
    lock.try_lock()?;
    Ok(lock)
}

struct Committed {
    journal: Vec<serde_json::Value>,
    receipt: census_store::Receipt,
}

impl Committed {
    fn read(fixture: &Fixture) -> TestResult<Self> {
        let phase = CaptureProtocolVersion::V2.phase();
        let keys = fixture.store.journal_keys(phase)?;
        let mut terminals = keys.iter().filter(|key| key.ends_with("/terminal"));
        let terminal = terminals.next().ok_or("committed terminal journal key")?;
        check!(eq; terminals.next(), None);
        let receipt = fixture
            .store
            .receipt(&format!("{phase}/{terminal}"))?
            .ok_or("committed receipt before publication")?;
        Ok(Self {
            journal: fixture.store.journal_payloads(phase)?,
            receipt,
        })
    }

    fn unchanged(
        &self,
        fixture: &Fixture,
        parts: census_store::RetainedCaptureParts,
        target: &PreparedRecoveryTarget,
        archived: &Pair,
    ) -> TestResult {
        check!(eq; parts.body, archived.body_bytes);
        check!(eq; parts.prepared.metadata_bytes(), archived.metadata_bytes);
        check!(eq; parts.terminal.attempt(), target.attempt());
        check!(eq; parts.terminal.terminal(), &census_store::AcquisitionTerminal::ResponseRetained {
            prepared_digest: target.prepared_digest().clone(),
            disposition: CaptureDispositionV1::ReusableResponse,
        });
        check!(eq; parts.occurrence, parts.terminal.occurrence()?);
        check!(eq; parts.receipt, self.receipt);
        check!(eq; parts.receipt.digest, serialized_digest(&parts.terminal)?);
        check!(eq; fixture.store.receipt(&self.receipt.operation)?, Some(self.receipt.clone()));
        check!(eq; fixture.store.journal_payloads(CaptureProtocolVersion::V2.phase())?, self.journal);
        Ok(())
    }
}
