use super::super::super::{codec, RosterSourceIssuanceStoreError as Error};
use super::super::{ObjectRef, ReviewRecord, SemanticReview};
use crate::{Store, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::roster_affiliation_authority::{
    LocallyCheckedRosterIssuanceV1, RosterAuthorityTextV1,
};
use fjall::Snapshot;

pub(super) struct OriginalSource {
    pub(super) reference: ObjectRef,
    pub(super) record: ReviewRecord,
    pub(super) proposal: LocallyCheckedRosterIssuanceV1,
    pub(super) review: SemanticReview,
    pub(super) interpretation: Vec<u8>,
}

pub(super) fn load(
    store: &Store,
    snapshot: &Snapshot,
    operation: &RosterAuthorityTextV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<OriginalSource> {
    let key = format!(
        "{}{}",
        super::super::RECORD_PREFIX,
        codec::digest(operation.as_str().as_bytes())
    );
    let raw = super::io::read(snapshot, &store.journal, &key, grant)?;
    let record: ReviewRecord = codec::decode_review_composite(&raw)?;
    if record.basis.schema != "roster-source-reviewed-claims-basis-v1"
        || record.basis.operation != *operation
    {
        return Err(Error::OperationConflict.into());
    }
    let reference = ObjectRef {
        digest: codec::digest(&raw),
        bytes: raw.len(),
    };
    let proposal = super::super::proposal(&record.basis.proposal)?;
    let review_raw = object(store, snapshot, &record.basis.review, grant)?;
    let review: SemanticReview = codec::decode(&review_raw)?;
    let interpretation_key = format!(
        "{}{}",
        super::super::OBJECT_PREFIX,
        proposal.capture().interpretation().digest().as_str()
    );
    let interpretation = super::io::read(snapshot, &store.journal, &interpretation_key, grant)?;
    Ok(OriginalSource {
        reference,
        record,
        proposal,
        review,
        interpretation,
    })
}

pub(super) fn object(
    store: &Store,
    snapshot: &Snapshot,
    reference: &ObjectRef,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let key = format!("{}{}", super::super::OBJECT_PREFIX, reference.digest);
    let bytes = super::io::read(snapshot, &store.journal, &key, grant)?;
    if bytes.len() != reference.bytes || codec::digest(&bytes) != reference.digest {
        return Err(Error::Corrupt("withdrawal object reference differs").into());
    }
    Ok(bytes)
}
