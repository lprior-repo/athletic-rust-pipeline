use super::*;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub(super) enum Encoding {
    Canonical,
    Compact,
    Unknown,
}

pub(super) struct Prepared {
    pub(super) owner: ResultsCaptureOwnerV1,
    pub(super) work: AcquisitionWorkV1,
    pub(super) target: PreparedRecoveryTarget,
    pub(super) metadata: Vec<u8>,
    pub(super) body: Vec<u8>,
    usage: census_store::AcquisitionSelectionUsage,
}

impl Prepared {
    pub(super) fn budget(&self) -> TestResult<PreparedRecoveryBudget> {
        Ok(PreparedRecoveryBudget::admit(self.usage, 1)?)
    }
}

pub(super) fn prepare(fixture: &Fixture, body: &[u8], encoding: Encoding) -> TestResult<Prepared> {
    let owner = fixture.owner(2026, CaptureCacheIntent::ReuseEligible)?;
    let reference = fixture.reference()?;
    let options = crate::net::FetchOptions::default();
    let request = crate::milesplit::owned_meet_request(&fixture.fetcher, &reference, &options)?;
    let work = owner.for_owned_meet(reference.site.jurisdiction(), &reference.meet_id, request)?;
    let grant = fixture.writer.continuation_for_owned_meet(
        &fixture.store,
        &owner,
        reference.site.jurisdiction(),
        &reference.meet_id,
    )?;
    let registered = fixture
        .store
        .register_acquisition_work(&owner, work.clone(), grant)?;
    let reserved = fixture.store.reserve_acquisition_attempt(registered)?;
    let armed = fixture.store.arm_acquisition_dispatch(
        reserved,
        DispatchRequestV1::new(
            work.request().original_url(),
            ConditionalRequestV1::Unconditional,
        )?,
    )?;
    let observed = fixture.store.record_acquisition_response(
        armed.into_observation(),
        ResponseObservationV1::new(200, work.request().original_url(), CAPTURED, vec![])?,
    )?;
    let digest = format!("{:x}", Sha256::digest(body));
    let meta: crate::net::cache::CacheMeta = serde_json::from_value(serde_json::json!({
        "url": work.request().original_url(), "response_url": work.request().original_url(),
        "method": "GET", "status": 200, "content_digest": digest,
        "bytes": body.len(), "fetched_at": CAPTURED,
    }))?;
    let metadata = match encoding {
        Encoding::Canonical => {
            crate::net::cache::Capture::from_meta(&meta, fixture.fetcher.cache_dir())?.encoded
        }
        Encoding::Compact => serde_json::to_vec(&meta)?,
        Encoding::Unknown => {
            let mut value = serde_json::to_value(&meta)?;
            value.as_object_mut().ok_or("metadata object")?.insert(
                "extension".into(),
                serde_json::json!({"nested": [null, {"z":1,"a":2}]}),
            );
            serde_json::to_vec(&value)?
        }
    };
    let input = PreparedCaptureInputV1::new(&digest, u64::try_from(body.len())?, metadata.clone())?;
    drop(fixture.store.prepare_acquisition_capture(
        observed,
        input,
        CaptureDispositionV1::ReusableResponse,
    )?);
    let mut selection = AcquisitionReadBudget::for_selection();
    let ExistingRecoveryRoute::Prepared(target) =
        fixture
            .store
            .read_existing_recovery_target(&owner, &work, &mut selection)?
    else {
        return Err("Prepared target selection".into());
    };
    Ok(Prepared {
        owner,
        work,
        target,
        metadata,
        body: body.to_vec(),
        usage: selection.selection_usage(),
    })
}

pub(super) fn publish(fixture: &Fixture, prepared: &Prepared) -> TestResult<(PathBuf, PathBuf)> {
    let digest = format!("{:x}", Sha256::digest(&prepared.body));
    let meta_digest = format!("{:x}", Sha256::digest(&prepared.metadata));
    let body = fixture
        .store
        .http_cache_dir()
        .join("archive/bodies")
        .join(format!("{digest}.body"));
    let metadata = fixture
        .store
        .http_cache_dir()
        .join("archive/captures")
        .join(&digest)
        .join(format!("{meta_digest}.meta.json"));
    std::fs::create_dir_all(body.parent().ok_or("body parent")?)?;
    std::fs::create_dir_all(metadata.parent().ok_or("metadata parent")?)?;
    std::fs::write(&body, &prepared.body)?;
    std::fs::write(&metadata, &prepared.metadata)?;
    Ok((body, metadata))
}
