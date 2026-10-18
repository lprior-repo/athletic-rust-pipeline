use super::{grant_error, guarded, Error};
use crate::{Store, StoreResult, StoreRevision, StoreSnapshot};
use census_domain::evidence_subject_attachment::{AttachmentPublicHeadV1, AttachmentReplayGrantV1};
use census_domain::model::StoreIdentity;
use census_domain::result_school_correspondence::*;

pub(super) struct Inspected<'s> {
    pub(super) revision: StoreRevision<'s>,
    pub(super) store: StoreIdentity,
    pub(super) prior: ResultSchoolPriorCapabilityV1,
    pub(super) token: ResultSchoolDigestV1,
    pub(super) head: AttachmentPublicHeadV1,
}

pub(super) fn inspect(
    store: &Store,
    mut grant: AttachmentReplayGrantV1,
) -> StoreResult<(Inspected<'_>, AttachmentReplayGrantV1)> {
    let result = guarded(&mut grant, |grant| {
        let format = store.store_format()?;
        if !matches!(format.lifecycle_revision, Some(7 | 8))
            || format.schema_version != Some(2)
            || format.key_format != Some(1)
            || format.migration_target.is_some()
        {
            return Err(Error::EnrollmentRequired.into());
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
        let view = store.snapshot_locked();
        let store_identity = super::super::read::identity(&snapshot, &store.journal, grant)?;
        let qualification = super::super::read::qualification(&snapshot, &store.meta, grant)?;
        let declared: census_domain::public_identity_qualification::PublicIdentityQualificationV3 =
            super::super::codec::decode(&qualification, grant)?;
        if declared.store_identity() != &store_identity {
            return Err(Error::Corrupt("qualification origin").into());
        }
        let qualification = super::super::codec::digest(&qualification, grant)?;
        let prior = if format.lifecycle_revision == Some(8) {
            let enrollment = super::super::read::value(
                &snapshot,
                &store.meta,
                crate::roster_source_issuance::ENROLLMENT.as_bytes(),
                grant,
            )?;
            ResultSchoolPriorCapabilityV1::IdentityAndRosterSource {
                qualification,
                roster_enrollment: super::super::codec::digest(&enrollment, grant)?,
            }
        } else {
            ResultSchoolPriorCapabilityV1::IdentityOnly { qualification }
        };
        validate_retained_prior(&snapshot, store, &prior, &store_identity, grant)?;
        let (token, head) = inputs(&view, grant)?;
        Ok(Inspected {
            revision: view.revision(),
            store: store_identity,
            prior,
            token,
            head,
        })
    })?;
    Ok((result, grant))
}

pub(in super::super) fn inputs(
    view: &StoreSnapshot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(ResultSchoolDigestV1, AttachmentPublicHeadV1)> {
    grant.charge_bytes(4096).map_err(grant_error)?;
    grant.charge_steps(16).map_err(grant_error)?;
    let token = view.attachment_input_revision_bytes()?;
    let token = super::super::codec::digest(&token, grant)?;
    let head = view.attachment_public_head()?;
    let head = AttachmentPublicHeadV1 {
        sequence: head.sequence,
        digest: ResultSchoolDigestV1::parse(&head.digest).map_err(grant_error)?,
    };
    Ok((token, head))
}

fn validate_retained_prior(
    snapshot: &fjall::Snapshot,
    store: &Store,
    prior: &ResultSchoolPriorCapabilityV1,
    store_identity: &StoreIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let retained = super::super::read::prior(
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
    } = retained
    {
        crate::roster_source_issuance::validate_roster_enrollment_proof_v1(
            &enrollment,
            &receipt,
            store_identity,
            grant,
        )?;
    }
    Ok(())
}
