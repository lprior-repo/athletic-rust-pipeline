use super::{grant_error, guarded, Error, NAME};
use crate::{Store, StoreResult, StoreRevision, StoreSnapshot};
use census_domain::evidence_subject_attachment::{AttachmentPublicHeadV1, AttachmentReplayGrantV1};
use census_domain::model::StoreIdentity;
use census_domain::result_calendar_correspondence::*;

pub(super) struct Inspected<'s> {
    pub(super) revision: StoreRevision<'s>,
    pub(super) store: StoreIdentity,
    pub(super) prior_school: Vec<u8>,
    pub(super) prior_digest: ResultCalendarDigestV1,
    pub(super) token: ResultCalendarDigestV1,
    pub(super) head: AttachmentPublicHeadV1,
}

pub(super) fn inspect(
    store: &Store,
    mut grant: AttachmentReplayGrantV1,
) -> StoreResult<(Inspected<'_>, AttachmentReplayGrantV1)> {
    let result = guarded(&mut grant, |grant| {
        let format = store.store_format()?;
        if format.lifecycle_revision != Some(9)
            || format.schema_version != Some(2)
            || format.key_format != Some(1)
            || format.migration_target.is_some()
        {
            return Err(Error::EnrollmentRequired.into());
        }
        let snapshot = store.db.snapshot();
        let identity = super::super::read::identity(&snapshot, &store.journal, grant)?;
        let prior_school = super::super::read::school_proof(
            &snapshot,
            &store.meta,
            &store.journal,
            &store.receipts,
            grant,
        )?;
        let prior_digest = super::super::codec::digest(&prior_school, grant)?;
        let view = store.snapshot_locked();
        let (token, head) = inputs(&view, grant)?;
        [&store.meta, &store.journal, &store.receipts]
            .into_iter()
            .try_for_each(|table| {
                super::super::read::namespace(&snapshot, table, NAME, &[], grant)
            })?;
        Ok(Inspected {
            revision: view.revision(),
            store: identity,
            prior_school,
            prior_digest,
            token,
            head,
        })
    })?;
    Ok((result, grant))
}

pub(in super::super) fn inputs(
    view: &StoreSnapshot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(ResultCalendarDigestV1, AttachmentPublicHeadV1)> {
    grant.charge_bytes(4096).map_err(grant_error)?;
    grant.charge_steps(16).map_err(grant_error)?;
    let token = view.attachment_input_revision_bytes()?;
    let token = super::super::codec::digest(&token, grant)?;
    let head = view.attachment_public_head()?;
    let head = AttachmentPublicHeadV1 {
        sequence: head.sequence,
        digest: ResultCalendarDigestV1::parse(&head.digest).map_err(grant_error)?,
    };
    Ok((token, head))
}
