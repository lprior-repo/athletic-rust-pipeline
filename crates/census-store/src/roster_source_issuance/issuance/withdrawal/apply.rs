use super::super::super::{source_error, RosterSourceIssuanceStoreError as Error};
use super::super::{apply as common, PreparedRosterReviewCore};
use super::PreparedRosterMemberWithdrawalV1;
use crate::{Application, Store, StoreResult, StoreRevision};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use chrono::Utc;

impl Store {
    pub fn apply_roster_member_withdrawal_v1(
        &self,
        prepared: PreparedRosterMemberWithdrawalV1<'_>,
        approved: &[u8],
        approval_digest: &str,
    ) -> StoreResult<Application> {
        let _guard = self.lock_appends();
        let PreparedRosterMemberWithdrawalV1 {
            revision,
            mut grant,
            core,
        } = prepared;
        let result = apply(self, &revision, &mut grant, core, approved, approval_digest);
        if result.is_err() {
            grant.poison();
        }
        result
    }
}

fn apply(
    store: &Store,
    revision: &StoreRevision<'_>,
    grant: &mut AttachmentReplayGrantV1,
    core: PreparedRosterReviewCore,
    approved: &[u8],
    approval_digest: &str,
) -> StoreResult<Application> {
    grant.ensure_available().map_err(source_error)?;
    revision.ensure_origin(store, core.record.operation.as_str())?;
    if approved != core.approval || approval_digest != core.approval_digest {
        return Err(Error::ApprovalMismatch.into());
    }
    let operation = format!(
        "{}{}",
        super::super::super::OPERATION_PREFIX,
        core.record.operation.as_str()
    );
    if common::replay_if_settled(store, &operation)? {
        return super::super::replay::replay(
            store,
            core.record.operation.as_str(),
            approval_digest,
        );
    }
    revision.ensure_current(store, core.record.operation.as_str())?;
    super::require_owner(store)?;
    let current = super::io::read(
        &store.db.snapshot(),
        &store.meta,
        super::super::super::HEAD,
        grant,
    )?;
    let expected = core
        .objects
        .get(&core.record.prior_head.digest)
        .ok_or(Error::Corrupt("withdrawal parent head absent"))?;
    if current != *expected {
        return Err(Error::InputMoved.into());
    }
    let sealed = common::seal_core(core, Utc::now(), grant)?;
    grant.ensure_available().map_err(source_error)?;
    common::commit_review(store, &sealed)?;
    Ok(Application::Written(sealed.receipt))
}
