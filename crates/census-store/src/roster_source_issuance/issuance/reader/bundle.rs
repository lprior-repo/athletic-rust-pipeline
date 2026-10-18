use super::super::super::codec;
use super::*;
use census_domain::evidence_subject_attachment::{AttachmentDigestV1, AttachmentObjectReferenceV1};
use serde::{Deserialize, Serialize};
use std::io::Write;

const MAX_BUNDLE: usize = 8 * 1024 * 1024;

pub struct FrozenRosterReviewedClaimsBundleV1 {
    bytes: Vec<u8>,
    reference: RosterFrozenReviewedClaimsReferenceV1,
}

impl FrozenRosterReviewedClaimsBundleV1 {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn digest(&self) -> &str {
        self.reference.digest.as_str()
    }
    pub fn reference(&self) -> &RosterFrozenReviewedClaimsReferenceV1 {
        &self.reference
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterFrozenReviewedClaimsReferenceV1 {
    version: u8,
    digest: AttachmentDigestV1,
    bytes: u64,
    store: StoreIdentity,
    selected_snapshot_sequence: u64,
    authority_head: AttachmentObjectReferenceV1,
    current_input_token: AttachmentObjectReferenceV1,
}

impl RosterFrozenReviewedClaimsReferenceV1 {
    pub const MAX_ENCODED_BYTES: usize = MAX_BUNDLE;

    pub fn digest(&self) -> &str {
        self.digest.as_str()
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store
    }
    pub fn selected_snapshot_sequence(&self) -> u64 {
        self.selected_snapshot_sequence
    }
    pub fn authority_head(&self) -> &AttachmentObjectReferenceV1 {
        &self.authority_head
    }
    pub fn current_input_token(&self) -> &AttachmentObjectReferenceV1 {
        &self.current_input_token
    }
}

impl CurrentRosterReviewedClaimsV1 {
    pub fn freeze_closure(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<FrozenRosterReviewedClaimsBundleV1> {
        self.require_original_grant(grant)?;
        let result = (|| {
            let bytes = super::inherited::encode(&self.archive, grant)?;
            let reference = reference(&self.archive, &bytes, grant)?;
            self.require_original_grant(grant)?;
            Ok(FrozenRosterReviewedClaimsBundleV1 { bytes, reference })
        })();
        if result.is_err() {
            grant.poison();
        }
        result
    }
}

pub fn replay_frozen_roster_reviewed_claims_v1(
    bytes: &[u8],
    expected: &RosterFrozenReviewedClaimsReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ReplayedRosterReviewedClaimsV1> {
    let result = replay(bytes, expected, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn replay(
    bytes: &[u8],
    expected: &RosterFrozenReviewedClaimsReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ReplayedRosterReviewedClaimsV1> {
    let continuation = grant
        .checkpoint()
        .map_err(super::super::super::source_error)?;
    if bytes.len() > MAX_BUNDLE {
        return Err(Error::BudgetExceeded.into());
    }
    grant
        .charge_bytes(bytes.len().checked_mul(4).ok_or(Error::BudgetExceeded)?)
        .map_err(super::super::super::source_error)?;
    grant
        .charge_steps(512)
        .map_err(super::super::super::source_error)?;
    if expected.version != 1
        || expected.bytes != u64::try_from(bytes.len()).map_err(|_| Error::BudgetExceeded)?
        || codec::digest(bytes) != expected.digest.as_str()
    {
        return Err(Error::Corrupt("frozen reviewed closure digest").into());
    }
    let archive = super::inherited::decode(bytes, grant)?;
    if archive.store != expected.store
        || archive.selected_snapshot_sequence != expected.selected_snapshot_sequence
        || object_reference(archive.authority_head()?, grant)? != expected.authority_head
        || object_reference(&archive.current_input_token, grant)? != expected.current_input_token
    {
        return Err(Error::Corrupt("frozen reviewed publication binding differs").into());
    }
    let claims = reconstruct::claims(&archive, grant)?;
    continuation
        .require(grant)
        .map_err(super::super::super::source_error)?;
    Ok(ReplayedRosterReviewedClaimsV1(
        CurrentRosterReviewedClaimsV1 {
            archive,
            claims,
            continuation,
        },
    ))
}

pub(super) fn encode(
    value: &impl Serialize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant
        .charge_steps(1)
        .map_err(super::super::super::source_error)?;
    let mut count = Count(0);
    serde_json::to_writer(&mut count, value).map_err(|_| Error::BudgetExceeded)?;
    grant
        .charge_bytes(count.0)
        .map_err(super::super::super::source_error)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(count.0)
        .map_err(|_| Error::BudgetExceeded)?;
    serde_json::to_writer(&mut bytes, value)
        .map_err(|_| Error::Corrupt("frozen reviewed closure encoding"))?;
    if bytes.len() != count.0 {
        return Err(Error::Corrupt("frozen reviewed closure size changed").into());
    }
    Ok(bytes)
}

pub(super) fn require_canonical(
    value: &impl Serialize,
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant
        .charge_steps(1)
        .map_err(super::super::super::source_error)?;
    let mut matched = CanonicalMatch { bytes, offset: 0 };
    serde_json::to_writer(&mut matched, value)
        .map_err(|_| Error::Corrupt("noncanonical frozen reviewed closure"))?;
    if matched.offset != bytes.len() {
        return Err(Error::Corrupt("noncanonical frozen reviewed closure").into());
    }
    Ok(())
}

struct CanonicalMatch<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl Write for CanonicalMatch<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let end = self
            .offset
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("roster canonical offset overflow"))?;
        if self.bytes.get(self.offset..end) != Some(bytes) {
            return Err(std::io::Error::other("roster canonical bytes differ"));
        }
        self.offset = end;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Count(usize);
impl Write for Count {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("roster frozen size overflow"))?;
        if self.0 > MAX_BUNDLE {
            return Err(std::io::Error::other("roster frozen size cap"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn reference(
    archive: &Archive,
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<RosterFrozenReviewedClaimsReferenceV1> {
    let head = archive.authority_head()?;
    Ok(RosterFrozenReviewedClaimsReferenceV1 {
        version: 1,
        digest: object_reference(bytes, grant)?.digest,
        bytes: u64::try_from(bytes.len()).map_err(|_| Error::BudgetExceeded)?,
        store: archive.store.clone(),
        selected_snapshot_sequence: archive.selected_snapshot_sequence,
        authority_head: object_reference(head, grant)?,
        current_input_token: object_reference(&archive.current_input_token, grant)?,
    })
}

fn object_reference(
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<AttachmentObjectReferenceV1> {
    grant
        .charge_bytes(bytes.len())
        .map_err(super::super::super::source_error)?;
    grant
        .charge_steps(1)
        .map_err(super::super::super::source_error)?;
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&codec::digest(bytes))
            .map_err(super::super::super::source_error)?,
        bytes: u64::try_from(bytes.len()).map_err(|_| Error::BudgetExceeded)?,
    })
}
