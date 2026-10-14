use census_domain::model::{InstallationConfigId, InstallationConfigV1, InstallationEnrollmentV1};
use fjall::{Keyspace, Readable, Snapshot};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{
    InstallationError, InstallationState, CONFIG_PREFIX, ENROLLMENT_KEY,
    MAX_INSTALLATION_RECORD_BYTES, PRODUCTION_LIFECYCLE,
};
use crate::{StoreError, StoreResult};

#[derive(Serialize)]
pub(super) struct ConfigEnvelope<'a> {
    pub(super) version: u32,
    pub(super) id: &'a InstallationConfigId,
    pub(super) payload: &'a InstallationConfigV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigInput {
    version: u32,
    id: InstallationConfigId,
    payload: InstallationConfigV1,
}

#[derive(Deserialize)]
struct VersionHeader {
    version: u32,
}

#[derive(Deserialize)]
struct PayloadHeader {
    payload: VersionHeader,
}

pub(crate) fn validate_format(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    lifecycle: Option<u32>,
) -> StoreResult<InstallationState> {
    let state = retained(snapshot, meta, journal)?;
    if lifecycle == Some(PRODUCTION_LIFECYCLE) && matches!(state, InstallationState::NotEnrolled) {
        return Err(InstallationError::MissingEnrollment.into());
    }
    match (&state, lifecycle) {
        (InstallationState::NotEnrolled, Some(PRODUCTION_LIFECYCLE))
        | (InstallationState::Enrolled { .. }, None) => {
            Err(InstallationError::MarkerMismatch { found: lifecycle }.into())
        }
        (InstallationState::Enrolled { .. }, Some(found)) if found != PRODUCTION_LIFECYCLE => {
            Err(InstallationError::MarkerMismatch { found: lifecycle }.into())
        }
        _ => Ok(state),
    }
}

pub(crate) fn require_legacy_execution(state: &InstallationState) -> StoreResult<()> {
    match state {
        InstallationState::NotEnrolled => Ok(()),
        InstallationState::Enrolled { enrollment, .. } => {
            Err(InstallationError::EnrollmentOnlyUnsupported {
                store_identity: enrollment.store_identity().clone(),
                config_id: enrollment.config_id().clone(),
            }
            .into())
        }
    }
}

pub(super) fn retained(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
) -> StoreResult<InstallationState> {
    let rows = installation_rows(snapshot, meta)?;
    if rows == 0 {
        return Ok(InstallationState::NotEnrolled);
    }
    if rows > 2 {
        return Err(InstallationError::MultipleRecords.into());
    }
    let enrollment: InstallationEnrollmentV1 =
        required(snapshot, meta, ENROLLMENT_KEY, "installation enrollment")?;
    let key = format!("{CONFIG_PREFIX}{}", enrollment.config_id());
    let config: ConfigInput = required(snapshot, meta, &key, "installation config envelope")?;
    if config.version != 1 {
        return Err(InstallationError::UnsupportedVersion {
            what: "installation config envelope",
            found: config.version,
        }
        .into());
    }
    if &config.id != enrollment.config_id()
        || config.payload.id().map_err(InstallationError::Domain)? != config.id
    {
        return Err(InstallationError::ConfigIdMismatch.into());
    }
    match crate::lineage::read(snapshot, journal)? {
        crate::lineage::RetainedIdentity::Present(identity)
            if &identity == enrollment.store_identity() => {}
        _ => return Err(InstallationError::ForeignStore.into()),
    }
    if crate::run::read(snapshot, meta)?.is_some() {
        return Err(InstallationError::HistoricalStatePresent {
            kind: super::HistoricalStateKind::LegacyRun,
        }
        .into());
    }
    crate::profiles::read::retained(snapshot, meta, config.payload.initial_profile_id())?
        .ok_or_else(|| crate::ProfileRegistryError::MissingProfile {
            id: config.payload.initial_profile_id().clone(),
        })?;
    Ok(InstallationState::Enrolled {
        enrollment,
        config: config.payload,
    })
}

fn installation_rows(snapshot: &Snapshot, meta: &Keyspace) -> StoreResult<usize> {
    let mut rows = 0_usize;
    for guard in snapshot.prefix(meta, b"installation:").take(4) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let supported =
            key.as_ref() == ENROLLMENT_KEY.as_bytes() || key.starts_with(CONFIG_PREFIX.as_bytes());
        if !supported {
            return Err(InstallationError::UnsupportedNamespace.into());
        }
        rows = rows.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    }
    Ok(rows)
}

fn required<T: DeserializeOwned>(
    snapshot: &Snapshot,
    meta: &Keyspace,
    key: &str,
    what: &'static str,
) -> StoreResult<T> {
    let length = snapshot
        .size_of(meta, key.as_bytes())
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| missing(key))?;
    let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
    if bytes > MAX_INSTALLATION_RECORD_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes,
            max: MAX_INSTALLATION_RECORD_BYTES,
        });
    }
    let raw = snapshot
        .get(meta, key.as_bytes())
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| missing(key))?;
    let header: VersionHeader = decode(key, &raw)?;
    if header.version != 1 {
        return Err(InstallationError::UnsupportedVersion {
            what,
            found: header.version,
        }
        .into());
    }
    if key != ENROLLMENT_KEY {
        let payload: PayloadHeader = decode(key, &raw)?;
        if payload.payload.version != 1 {
            return Err(InstallationError::UnsupportedVersion {
                what: "installation config payload",
                found: payload.payload.version,
            }
            .into());
        }
    }
    decode(key, &raw)
}

fn decode<T: DeserializeOwned>(key: &str, raw: &[u8]) -> StoreResult<T> {
    serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
        key: key.to_string(),
        source,
    })
}

fn missing(key: &str) -> InstallationError {
    if key == ENROLLMENT_KEY {
        InstallationError::MissingEnrollment
    } else {
        InstallationError::MissingConfig
    }
}
