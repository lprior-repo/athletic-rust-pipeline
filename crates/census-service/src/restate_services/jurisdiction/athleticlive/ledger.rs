use crate::restate_services::{
    wire::{AthleticLiveObligation, JurisdictionRequest, StageOutcome},
    JobError,
};
use census_crawl::athleticlive::FrozenManifest;
use census_reconcile::identity::WorkflowIdentity;
use census_store::Store;
use serde::de::DeserializeOwned;

pub(super) const PHASE: &str = "athleticlive_manifest_obligation_v1";

pub(super) fn key(request: &JurisdictionRequest) -> String {
    format!(
        "{}/athleticlive_manifest",
        WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision)
            .as_str()
    )
}

pub(in crate::restate_services::jurisdiction) fn applicable(request: &JurisdictionRequest) -> bool {
    census_crawl::applicability::table()
        .iter()
        .any(|row| row.slug == "athleticlive" && row.jurisdictions.contains(&request.jurisdiction))
}

pub(in crate::restate_services::jurisdiction) fn missing_reason(
    request: &JurisdictionRequest,
) -> String {
    format!("{}: no qualified retained AthleticLIVE manifest is registered for this jurisdiction run; operator captures with physical producer metadata are required; no live discovery or invented objects", request.jurisdiction.code())
}

pub(super) fn status(
    store: &Store,
    request: &JurisdictionRequest,
) -> Result<Option<AthleticLiveObligation>, JobError> {
    read(store, &format!("{}/status", key(request)))
}

pub(super) fn bind(
    store: &Store,
    request: &JurisdictionRequest,
) -> Result<Option<AthleticLiveObligation>, JobError> {
    if !applicable(request) {
        return Ok(None);
    }
    if let Some(status) = status(store, request)? {
        return Ok(Some(status));
    }
    let status = AthleticLiveObligation::Refused {
        reason: missing_reason(request),
    };
    store.journal_done(PHASE, &format!("{}/status", key(request)), &status)?;
    Ok(Some(status))
}

pub(super) fn register(
    store: &Store,
    request: &JurisdictionRequest,
    frozen: FrozenManifest,
) -> Result<AthleticLiveObligation, JobError> {
    if !applicable(request) {
        return Err(error("AthleticLIVE is not applicable to this jurisdiction"));
    }
    if !frozen.jurisdictions.contains(&request.jurisdiction) {
        return refuse(store, request, format!("{}: retained AthleticLIVE manifest contains no published meet for this jurisdiction", request.jurisdiction.code()));
    }
    if let Some(existing) = status(store, request)? {
        if let Some(digest) = existing.digest() {
            if digest != frozen.digest {
                return Err(error(
                    "AthleticLIVE manifest digest changed within this logical obligation",
                ));
            }
            manifest(store, request)?;
            return Ok(existing);
        }
    }
    let status = AthleticLiveObligation::Armed {
        digest: frozen.digest.clone(),
    };
    let mut batch = store.write_batch();
    batch.journal_done(
        PHASE,
        &format!("{}/manifest", key(request)),
        &(request.history, frozen),
    )?;
    batch.journal_done(PHASE, &format!("{}/status", key(request)), &status)?;
    batch.commit()?;
    Ok(status)
}

pub(super) fn refuse(
    store: &Store,
    request: &JurisdictionRequest,
    reason: String,
) -> Result<AthleticLiveObligation, JobError> {
    if status(store, request)?.is_some_and(|state| state.digest().is_some()) {
        return Err(error(
            "registered AthleticLIVE manifest cannot be replaced by a refusal",
        ));
    }
    let status = AthleticLiveObligation::Refused { reason };
    store.journal_done(PHASE, &format!("{}/status", key(request)), &status)?;
    Ok(status)
}

pub(super) fn manifest(
    store: &Store,
    request: &JurisdictionRequest,
) -> Result<FrozenManifest, JobError> {
    let (window, manifest): (crate::restate_services::wire::HistoryWindow, FrozenManifest) =
        read(store, &format!("{}/manifest", key(request)))?
            .ok_or_else(|| error("armed AthleticLIVE manifest is absent"))?;
    if window != request.history {
        return Err(error(
            "AthleticLIVE manifest history scope changed within logical obligation",
        ));
    }
    Ok(manifest)
}

pub(super) fn begin(
    store: &Store,
    request: &JurisdictionRequest,
) -> Result<Option<(String, u8)>, JobError> {
    let Some(status) = bind(store, request)? else {
        return Ok(None);
    };
    if status.is_terminal() {
        return Ok(None);
    }
    let (digest, previous) = match status {
        AthleticLiveObligation::Refused { .. } => return Ok(None),
        AthleticLiveObligation::Armed { digest } => (digest, 0),
        AthleticLiveObligation::Running { digest, attempt } => return Ok(Some((digest, attempt))),
        AthleticLiveObligation::Finished {
            digest, attempt, ..
        } => (digest, attempt),
    };
    let Some(attempt) = previous.checked_add(1).filter(|attempt| *attempt <= 3) else {
        return Ok(None);
    };
    let status = AthleticLiveObligation::Running {
        digest: digest.clone(),
        attempt,
    };
    let mut batch = store.write_batch();
    batch.journal_done(
        PHASE,
        &format!("{}/attempt/{attempt}/begin", key(request)),
        &status,
    )?;
    batch.journal_done(PHASE, &format!("{}/status", key(request)), &status)?;
    batch.commit()?;
    Ok(Some((digest, attempt)))
}

pub(super) fn finish(
    store: &Store,
    request: &JurisdictionRequest,
    reserved: (String, u8),
    outcome: StageOutcome,
) -> Result<AthleticLiveObligation, JobError> {
    let status = status(store, request)?
        .ok_or_else(|| error("AthleticLIVE completion has no reservation"))?;
    if let AthleticLiveObligation::Finished {
        digest, attempt, ..
    } = &status
    {
        if digest == &reserved.0 && attempt == &reserved.1 {
            return Ok(status);
        }
    }
    if !matches!(&status, AthleticLiveObligation::Running { digest, attempt } if digest == &reserved.0 && attempt == &reserved.1)
    {
        return Err(error(
            "AthleticLIVE completion differs from reserved manifest/attempt",
        ));
    }
    let status = AthleticLiveObligation::Finished {
        digest: reserved.0,
        attempt: reserved.1,
        outcome,
    };
    let mut batch = store.write_batch();
    batch.journal_done(
        PHASE,
        &format!("{}/attempt/{}/finish", key(request), reserved.1),
        &status,
    )?;
    batch.journal_done(PHASE, &format!("{}/status", key(request)), &status)?;
    batch.commit()?;
    Ok(status)
}

fn read<T: DeserializeOwned>(store: &Store, key: &str) -> Result<Option<T>, JobError> {
    store
        .journal_payload(PHASE, key)?
        .map(|payload| {
            serde_json::from_value(payload)
                .map_err(|source| error(&format!("AthleticLIVE journal {key}: {source}")))
        })
        .transpose()
}

pub(super) fn error(message: &str) -> JobError {
    JobError::Terminal {
        message: message.into(),
    }
}
