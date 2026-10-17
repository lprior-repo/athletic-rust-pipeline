use super::{PreparedRosterSourceReviewV1, ReviewRecord};
use super::super::{codec, enrollment::AuthorityHead, RosterSourceIssuanceStoreError as Error};
use crate::{Application, Receipt, Store, StoreError, StoreResult};
use chrono::Utc;
use fjall::{PersistMode, Readable};

impl Store {
    pub fn apply_roster_source_review_v1(
        &self,
        mut prepared: PreparedRosterSourceReviewV1<'_>,
        approved: &[u8],
        approval_digest: &str,
    ) -> StoreResult<Application> {
        let _guard = self.lock_appends();
        let result = apply(self, &mut prepared, approved, approval_digest);
        if result.is_err() { prepared.inspection.grant.poison(); }
        result
    }
}

fn apply(store: &Store, prepared: &mut PreparedRosterSourceReviewV1<'_>, approved: &[u8], digest: &str) -> StoreResult<Application> {
    prepared.inspection.revision.ensure_origin(store, prepared.record.operation.as_str())?;
    prepared.inspection.evidence.require_original_grant(&mut prepared.inspection.grant)?;
    if approved != prepared.approval || digest != prepared.approval_digest { return Err(Error::ApprovalMismatch.into()); }
    let operation = format!("{}{}", super::super::OPERATION_PREFIX, prepared.record.operation.as_str());
    if store.db.snapshot().contains_key(&store.receipts, &operation).map_err(|source| StoreError::Read { source })? {
        return super::replay::replay(store, prepared.record.operation.as_str(), digest);
    }
    super::prepare::require_current(store, &mut prepared.inspection, prepared.record.operation.as_str())?;
    let committed_at = Utc::now();
    let review: super::SemanticReview = codec::decode(prepared.objects.get(&prepared.record.review.digest)
        .ok_or(Error::Corrupt("prepared review absent"))?)?;
    if committed_at < review.fields.reviewed_at { return Err(Error::Corrupt("commit precedes review").into()); }
    let encoded_basis = codec::encode_with_source_grant(&prepared.record, &mut prepared.inspection.grant)?;
    prepared.inspection.grant.charge_bytes(encoded_basis.len()).map_err(super::super::source_error)?;
    prepared.inspection.grant.charge_steps(1).map_err(super::super::source_error)?;
    let basis = codec::decode(&encoded_basis)?;
    let record = ReviewRecord { basis, approval_digest: digest.into(), approval: approved.to_vec(), committed_at };
    let bytes = codec::encode_with_source_grant(&record, &mut prepared.inspection.grant)?;
    let record_digest = codec::digest(&bytes);
    let head = codec::encode(&AuthorityHead { version: 1, ordinal: prepared.record.next_ordinal, digest: record_digest.clone() })?;
    let receipt = Receipt { operation, digest: record_digest, at: committed_at.to_rfc3339(), appended: 0, credited: false };
    prepared.inspection.evidence.require_original_grant(&mut prepared.inspection.grant)?;
    commit(store, prepared, &bytes, &head, &receipt)?;
    Ok(Application::Written(receipt))
}

fn commit(store: &Store, prepared: &PreparedRosterSourceReviewV1<'_>, bytes: &[u8], head: &[u8], receipt: &Receipt) -> StoreResult<()> {
    let record_key = format!("{}{}", super::RECORD_PREFIX, codec::digest(prepared.record.operation.as_str().as_bytes()));
    let receipt_bytes = codec::encode(receipt)?;
    super::preflight::postcommit(store, prepared, record_key.as_bytes(), bytes, head, receipt.operation.as_bytes(), &receipt_bytes)?;
    let mut batch = store.db.batch();
    for (digest, object) in &prepared.objects {
        batch.insert(&store.journal, format!("{}{digest}", super::OBJECT_PREFIX), object);
    }
    batch.insert(&store.journal, record_key, bytes);
    batch.insert(&store.meta, super::super::HEAD, head);
    batch.insert(&store.receipts, receipt.operation.as_bytes(), receipt_bytes);
    batch.durability(Some(PersistMode::SyncData)).commit().map_err(|source| StoreError::Write { source })
}
