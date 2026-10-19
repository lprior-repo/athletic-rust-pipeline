use super::{
    guarded, Error, PreparedResultCalendarEnrollmentV1, ResultCalendarEnrollmentCommandV1, NAME,
};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;

pub(super) fn prepare<'s>(
    store: &'s Store,
    inputs: super::ResultCalendarEnrollmentInputsV1<'s>,
    review: super::ResultCalendarEnrollmentReviewV1,
) -> StoreResult<PreparedResultCalendarEnrollmentV1<'s>> {
    let super::ResultCalendarEnrollmentInputsV1 {
        revision,
        store: store_identity,
        prior_school,
        prior_digest,
        token,
        head,
        mut grant,
    } = inputs;
    let command = ResultCalendarEnrollmentCommandV1 {
        version: 1,
        store: store_identity,
        operation: review.operation,
        prior_result_school_enrollment: prior_digest,
        input_token: token,
        evidence_generation: revision.evidence_generation(),
        derived_generation: revision.derived_generation(),
        accepted_head: head,
        actor: review.actor,
        reviewed_at: review.reviewed_at,
        rationale: review.rationale,
    };
    let (prior_school, approval, approval_digest) = guarded(&mut grant, |grant| {
        revision.ensure_origin(store, command.operation.as_str())?;
        require_current(store, &revision, &command, &prior_school, grant)?;
        let prior_school = super::super::codec::raw(prior_school, grant)?;
        let approval = super::super::wire::enrollment_approval(&command, grant)?;
        let approval_digest = super::super::codec::digest(&approval, grant)?;
        Ok((prior_school, approval, approval_digest))
    })?;
    Ok(PreparedResultCalendarEnrollmentV1 {
        revision,
        command,
        prior_school,
        approval,
        approval_digest,
        replay: None,
        grant,
    })
}

pub(super) fn require_current(
    store: &Store,
    revision: &StoreRevision<'_>,
    command: &ResultCalendarEnrollmentCommandV1,
    prior_school: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    super::super::wire::validate_enrollment_command(command)?;
    revision.ensure_current(store, command.operation.as_str())?;
    let format = store.store_format()?;
    if format.lifecycle_revision != Some(9)
        || format.schema_version != Some(2)
        || format.key_format != Some(1)
        || format.migration_target.is_some()
    {
        return Err(Error::InputMoved.into());
    }
    let snapshot = store.db.snapshot();
    let identity = super::super::read::identity(&snapshot, &store.journal, grant)?;
    let current = super::super::read::school_proof(
        &snapshot,
        &store.meta,
        &store.journal,
        &store.receipts,
        grant,
    )?;
    if identity != command.store
        || current != prior_school
        || super::super::codec::digest(&current, grant)? != command.prior_result_school_enrollment
    {
        return Err(Error::InputMoved.into());
    }
    [&store.meta, &store.journal, &store.receipts]
        .into_iter()
        .try_for_each(|table| super::super::read::namespace(&snapshot, table, NAME, &[], grant))?;
    let (token, head) = super::inspect::inputs(&store.snapshot_locked(), grant)?;
    if token != command.input_token
        || head != command.accepted_head
        || revision.evidence_generation() != command.evidence_generation
        || revision.derived_generation() != command.derived_generation
    {
        return Err(Error::InputMoved.into());
    }
    Ok(())
}
