use super::*;

pub(super) enum AcceptedOwner<'v> {
    Owned(Box<AcceptedPublicIdentitySnapshot>),
    Borrowed(&'v AcceptedPublicIdentitySnapshot),
}

impl AcceptedOwner<'_> {
    pub(super) fn view(&self) -> &AcceptedPublicIdentitySnapshot {
        match self {
            Self::Owned(accepted) => accepted,
            Self::Borrowed(accepted) => accepted,
        }
    }
}

impl<'s> StoreSnapshot<'s> {
    pub fn read_current_evidence_subject_attachment_references_with_accepted_v1<'v>(
        &'v self,
        accepted: &'v AcceptedPublicIdentitySnapshot,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = read(self, accepted, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }
}

fn read<'v, 's>(
    view: &'v StoreSnapshot<'s>,
    accepted: &'v AcceptedPublicIdentitySnapshot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let identity = view.store_identity()?;
    let ledger = Ledger::load(view, &identity, grant)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let head = view.attachment_public_head()?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if accepted.store_identity() != &identity
        || accepted.snapshot_sequence() != view.sequence()
        || accepted.qualification_version()
            != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3
        || accepted.head() != &head
    {
        return Err(Error::TargetMoved.into());
    }
    let continuation = grant.checkpoint().map_err(Error::Domain)?;
    Ok(CurrentEvidenceSubjectAttachmentReferencesV1 {
        view,
        ledger,
        accepted: AcceptedOwner::Borrowed(accepted),
        continuation,
    })
}
