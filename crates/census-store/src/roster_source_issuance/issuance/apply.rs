use super::super::{codec, enrollment::AuthorityHead, RosterSourceIssuanceStoreError as Error};
use super::{PreparedRosterReviewCore, PreparedRosterSourceReviewV1, ReviewRecord};
use crate::{Application, Receipt, Store, StoreError, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use chrono::{DateTime, Utc};
use fjall::{PersistMode, Readable};

pub(super) struct SealedRosterReview {
    bytes: Vec<u8>,
    head: Vec<u8>,
    pub(super) receipt: Receipt,
    objects: super::objects::Objects,
    record_key: String,
}

impl Store {
    pub fn apply_roster_source_review_v1(
        &self,
        prepared: PreparedRosterSourceReviewV1<'_>,
        approved: &[u8],
        approval_digest: &str,
    ) -> StoreResult<Application> {
        let _guard = self.lock_appends();
        let PreparedRosterSourceReviewV1 {
            mut inspection,
            core,
        } = prepared;
        let result = apply(self, &mut inspection, core, approved, approval_digest);
        if result.is_err() {
            inspection.grant.poison();
        }
        result
    }
}

fn apply(
    store: &Store,
    inspection: &mut super::RosterIssuanceInspectionV1<'_>,
    core: PreparedRosterReviewCore,
    approved: &[u8],
    digest: &str,
) -> StoreResult<Application> {
    inspection
        .revision
        .ensure_origin(store, core.record.operation.as_str())?;
    inspection
        .evidence
        .require_original_grant(&mut inspection.grant)?;
    if approved != core.approval || digest != core.approval_digest {
        return Err(Error::ApprovalMismatch.into());
    }
    let operation = format!(
        "{}{}",
        super::super::OPERATION_PREFIX,
        core.record.operation.as_str()
    );
    if replay_if_settled(store, &operation)? {
        return super::replay::replay(store, core.record.operation.as_str(), digest);
    }
    super::prepare::require_current(store, inspection, core.record.operation.as_str())?;
    let sealed = seal_core(core, Utc::now(), &mut inspection.grant)?;
    inspection
        .evidence
        .require_original_grant(&mut inspection.grant)?;
    commit_review(store, &sealed)?;
    Ok(Application::Written(sealed.receipt))
}

pub(super) fn replay_if_settled(store: &Store, operation: &str) -> StoreResult<bool> {
    store
        .db
        .snapshot()
        .contains_key(&store.receipts, operation)
        .map_err(|source| StoreError::Read { source })
}

pub(super) fn seal_core(
    core: PreparedRosterReviewCore,
    committed_at: DateTime<Utc>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<SealedRosterReview> {
    let review: super::SemanticReview = codec::decode(
        core.objects
            .get(&core.record.review.digest)
            .ok_or(Error::Corrupt("prepared review absent"))?,
    )?;
    if committed_at < review.fields.reviewed_at {
        return Err(Error::Corrupt("commit precedes review").into());
    }
    let record_key = format!(
        "{}{}",
        super::RECORD_PREFIX,
        codec::digest(core.record.operation.as_str().as_bytes())
    );
    let operation = format!(
        "{}{}",
        super::super::OPERATION_PREFIX,
        core.record.operation.as_str()
    );
    let ordinal = core.record.next_ordinal;
    let record = ReviewRecord {
        basis: core.record,
        approval_digest: core.approval_digest,
        approval: core.approval,
        committed_at,
    };
    let bytes = codec::encode_review_composite_with_source_grant(&record, grant)?;
    let digest = codec::digest(&bytes);
    let head = codec::encode(&AuthorityHead {
        version: 1,
        ordinal,
        digest: digest.clone(),
    })?;
    Ok(SealedRosterReview {
        bytes,
        head,
        receipt: Receipt {
            operation,
            digest,
            at: committed_at.to_rfc3339(),
            appended: 0,
            credited: false,
        },
        objects: core.objects,
        record_key,
    })
}

pub(super) fn commit_review(store: &Store, sealed: &SealedRosterReview) -> StoreResult<()> {
    commit_review_with_admission(store, sealed, |estimate| {
        store.require_write_space(estimate).map(|_| ())
    })
}

fn commit_review_with_admission(
    store: &Store,
    sealed: &SealedRosterReview,
    admit: impl FnOnce(&crate::WriteSpaceEstimateV1) -> StoreResult<()>,
) -> StoreResult<()> {
    let receipt_bytes = codec::encode(&sealed.receipt)?;
    super::preflight::postcommit(
        store,
        &sealed.objects,
        sealed.record_key.as_bytes(),
        &sealed.bytes,
        &sealed.head,
        sealed.receipt.operation.as_bytes(),
        &receipt_bytes,
    )?;
    let mut batch = store.db.batch();
    let missing_bytes = stage_objects(store, &mut batch, &sealed.objects)?;
    let estimate = write_space_estimate(missing_bytes, sealed, &receipt_bytes)?;
    admit(&estimate)?;
    batch.insert(&store.journal, &sealed.record_key, &sealed.bytes);
    batch.insert(&store.meta, super::super::HEAD, &sealed.head);
    batch.insert(
        &store.receipts,
        sealed.receipt.operation.as_bytes(),
        &receipt_bytes,
    );
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

fn stage_objects(
    store: &Store,
    batch: &mut fjall::OwnedWriteBatch,
    objects: &super::objects::Objects,
) -> StoreResult<u64> {
    let snapshot = store.db.snapshot();
    let mut bytes = 0_u64;
    for (digest, body) in objects {
        let key = format!("{}{digest}", super::OBJECT_PREFIX);
        match snapshot
            .get(&store.journal, &key)
            .map_err(|source| StoreError::Read { source })?
        {
            Some(existing) if existing.as_ref() == body.as_slice() => {}
            Some(_) => return Err(Error::Corrupt("object digest collision").into()),
            None => {
                bytes = add_bytes(bytes, key.len())?;
                bytes = add_bytes(bytes, body.len())?;
                batch.insert(&store.journal, key, body);
            }
        }
    }
    Ok(bytes)
}

fn write_space_estimate(
    missing_bytes: u64,
    sealed: &SealedRosterReview,
    receipt: &[u8],
) -> StoreResult<crate::WriteSpaceEstimateV1> {
    let writes = [
        sealed.record_key.len(),
        sealed.bytes.len(),
        super::super::HEAD.len(),
        sealed.head.len(),
        sealed.receipt.operation.len(),
        receipt.len(),
    ]
    .into_iter()
    .try_fold(missing_bytes, add_bytes)?;
    crate::WriteSpaceEstimateV1::new(writes, writes, writes)
}

fn add_bytes(total: u64, bytes: usize) -> StoreResult<u64> {
    total
        .checked_add(u64::try_from(bytes).map_err(|_| StoreError::CounterExhausted)?)
        .ok_or(StoreError::CounterExhausted)
}

#[cfg(test)]
mod tests;
