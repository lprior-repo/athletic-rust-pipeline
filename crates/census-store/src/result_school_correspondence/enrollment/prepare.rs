use super::{
    guarded, Error, PreparedResultSchoolEnrollmentV1, ResultSchoolEnrollmentCommandV1,
    ResultSchoolEnrollmentInputsV1,
};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::result_school_correspondence::*;

pub(super) fn prepare<'s>(
    store: &'s Store,
    inputs: super::ResultSchoolEnrollmentInputsV1<'s>,
    review: super::ResultSchoolEnrollmentReviewV1,
) -> StoreResult<PreparedResultSchoolEnrollmentV1<'s>> {
    let ResultSchoolEnrollmentInputsV1 {
        revision,
        store: store_identity,
        prior,
        token,
        head,
        mut grant,
    } = inputs;

    let command = ResultSchoolEnrollmentCommandV1 {
        version: 1,
        store: store_identity,
        prior,
        input_token: token,
        accepted_head: head,
        evidence_generation: revision.evidence_generation(),
        derived_generation: revision.derived_generation(),
        operation: review.operation,
        actor: review.actor,
        reviewed_at: review.reviewed_at,
        rationale: review.rationale,
    };

    let (approval, approval_digest) = guarded(&mut grant, |grant| {
        revision.ensure_origin(store, command.operation.as_str())?;
        require_current(store, &revision, &command, grant)?;

        let approval = super::super::wire::enrollment_approval(&command, grant)?;
        let digest = super::super::codec::digest(&approval, grant)?;
        Ok((approval, digest))
    })?;

    Ok(PreparedResultSchoolEnrollmentV1 {
        revision,
        command,
        approval,
        approval_digest,
        replay: None,
        grant,
    })
}

pub(super) fn require_current(
    store: &Store,
    revision: &StoreRevision<'_>,
    command: &ResultSchoolEnrollmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    super::super::wire::validate_enrollment_command(command)?;
    revision.ensure_current(store, command.operation.as_str())?;

    let format = store.store_format()?;
    if format.lifecycle_revision != Some(command.prior.lifecycle())
        || format.schema_version != Some(2)
        || format.key_format != Some(1)
        || format.migration_target.is_some()
    {
        return Err(Error::InputMoved.into());
    }

    let snapshot = store.db.snapshot();
    super::super::read::load(
        &snapshot,
        &store.meta,
        &store.journal,
        &store.receipts,
        &format,
        grant,
    )?;

    let qualification = super::super::read::qualification(&snapshot, &store.meta, grant)?;
    if &super::super::codec::digest(&qualification, grant)? != command.prior.qualification()
        || super::super::read::identity(&snapshot, &store.journal, grant)? != command.store
    {
        return Err(Error::InputMoved.into());
    }

    require_prior_proof(&snapshot, store, &command.prior, &command.store, grant)?;

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

fn require_prior_proof(
    snapshot: &fjall::Snapshot,
    store: &Store,
    prior: &ResultSchoolPriorCapabilityV1,
    store_identity: &census_domain::model::StoreIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let proof = super::super::read::prior(
        snapshot,
        &store.meta,
        &store.journal,
        &store.receipts,
        prior,
        grant,
    )?;
    if let super::super::wire::PriorProof::IdentityAndRosterSource {
        enrollment,
        receipt,
    } = proof
    {
        let ResultSchoolPriorCapabilityV1::IdentityAndRosterSource {
            roster_enrollment, ..
        } = prior
        else {
            return Err(Error::InputMoved.into());
        };
        if &super::super::codec::digest(&enrollment, grant)? != roster_enrollment {
            return Err(Error::InputMoved.into());
        }
        crate::roster_source_issuance::validate_roster_enrollment_proof_v1(
            &enrollment,
            &receipt,
            store_identity,
            grant,
        )?;
    }
    Ok(())
}
