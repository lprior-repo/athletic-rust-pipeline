use crate::model::{serialized_digest, StoreIdentity};
use serde::Serialize;

use super::{
    budget, request_order, EstablishmentOrdinal, HistoryError, HistoryRequest, PublicAthleteId,
    ReplayLimits,
};

pub const HISTORY_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegistryHead {
    pub sequence: u64,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistoryRecord {
    pub version: u32,
    pub installation: StoreIdentity,
    pub sequence: u64,
    pub previous: RegistryHead,
    pub request: HistoryRequest,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedHistoryRecord {
    pub uncommitted_record: HistoryRecord,
    pub next_head: RegistryHead,
    pub established_ids: Vec<PublicAthleteId>,
}

pub fn genesis(installation: &StoreIdentity) -> Result<RegistryHead, HistoryError> {
    Ok(RegistryHead {
        sequence: 0,
        digest: digest(&("public-identity-history-genesis-v1", installation))?,
    })
}

pub fn prepare_next(
    installation: &StoreIdentity,
    current_head: &RegistryHead,
    request: HistoryRequest,
    limits: ReplayLimits,
) -> Result<PreparedHistoryRecord, HistoryError> {
    budget::request(&request, limits)?;
    if !is_digest(&current_head.digest) {
        return Err(HistoryError::Invalid("head digest"));
    }
    let sequence = current_head
        .sequence
        .checked_add(1)
        .ok_or(HistoryError::CounterOverflow)?;
    let request = request_order::normalize(request)?;
    let record = HistoryRecord {
        version: HISTORY_VERSION,
        installation: installation.clone(),
        sequence,
        previous: current_head.clone(),
        request,
        digest: String::new(),
    };
    finish_preparation(record)
}

fn finish_preparation(mut record: HistoryRecord) -> Result<PreparedHistoryRecord, HistoryError> {
    record.digest = payload_digest(&record)?;
    let established_ids = (0..births(&record.request))
        .map(|position| public_id(&record.installation, record.sequence, position))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PreparedHistoryRecord {
        next_head: RegistryHead {
            sequence: record.sequence,
            digest: record.digest.clone(),
        },
        uncommitted_record: record,
        established_ids,
    })
}

pub(super) fn payload_digest(record: &HistoryRecord) -> Result<String, HistoryError> {
    digest(&(
        "public-identity-history-record-v1",
        record.version,
        &record.installation,
        record.sequence,
        &record.previous,
        &record.request,
    ))
}

pub(super) fn public_id(
    installation: &StoreIdentity,
    sequence: u64,
    position: usize,
) -> Result<PublicAthleteId, HistoryError> {
    let ordinal = ordinal(sequence, position)?;
    let value = digest(&("public-athlete-id-v1", installation, ordinal))?;
    Ok(PublicAthleteId(format!("ath_public_v1_{value}")))
}

pub(super) fn ordinal(
    sequence: u64,
    position: usize,
) -> Result<EstablishmentOrdinal, HistoryError> {
    if sequence == 0 {
        return Err(HistoryError::Sequence);
    }
    Ok(EstablishmentOrdinal {
        sequence,
        position: u32::try_from(position).map_err(|_| HistoryError::CounterOverflow)?,
    })
}

pub(super) fn births(request: &HistoryRequest) -> usize {
    match request {
        HistoryRequest::Establish { .. } | HistoryRequest::EstablishMatchedGroup { .. } => 1,
        HistoryRequest::Split(split) => split.partitions.len(),
        HistoryRequest::Bind { .. }
        | HistoryRequest::BindCandidateMembership { .. }
        | HistoryRequest::Merge { .. }
        | HistoryRequest::Revoke(_) => 0,
    }
}

pub(super) fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn digest(value: &impl Serialize) -> Result<String, HistoryError> {
    serialized_digest(value).map_err(|error| HistoryError::Encoding(error.to_string()))
}
