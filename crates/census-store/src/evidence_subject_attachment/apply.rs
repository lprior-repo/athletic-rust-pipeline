use super::{
    guard::AttachmentCommitGuard,
    ledger::Ledger,
    prepare::{NewPrepared, PreparedEvidenceSubjectAttachmentV1, PreparedState},
    EvidenceSubjectAttachmentStoreError as Error,
};
use crate::{Application, Store, StoreResult, StoreRevision};
use census_domain::evidence_subject_attachment::{
    AttachmentActionV1, AttachmentDigestV1, AttachmentReplayGrantV1,
};

mod commit;
mod replay;

impl Store {
    pub fn apply_evidence_subject_attachment_v1(
        &self,
        prepared: PreparedEvidenceSubjectAttachmentV1<'_>,
        approval: &[u8],
        digest: &AttachmentDigestV1,
    ) -> StoreResult<Application> {
        let PreparedEvidenceSubjectAttachmentV1 {
            revision,
            approval: expected,
            state,
            mut grant,
        } = prepared;
        grant.ensure_available().map_err(Error::Domain)?;
        let result = apply(
            self, revision, &expected, state, approval, digest, &mut grant,
        );
        if result.is_err() {
            grant.poison();
        }
        result
    }
}

fn apply(
    store: &Store,
    revision: StoreRevision<'_>,
    expected: &[u8],
    state: PreparedState<'_>,
    approval: &[u8],
    digest: &AttachmentDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Application> {
    revision.ensure_origin(store, state.command().operation.as_str())?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if approval != expected || digest != state.approval_digest() {
        return Err(Error::Corrupt("explicit attachment approval differs").into());
    }
    let guard = AttachmentCommitGuard::acquire(store);
    let view = guard.snapshot();
    super::inputs::require_format(store)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let identity = view.store_identity()?;
    if identity != state.command().store {
        return Err(Error::InputMoved.into());
    }
    match state {
        PreparedState::New(input) => apply_new(guard, &view, &identity, *input, grant),
        PreparedState::Replay(input) => replay::apply(guard, &view, &identity, *input, grant),
    }
}

fn apply_new<'s>(
    guard: AttachmentCommitGuard<'s>,
    view: &crate::StoreSnapshot<'s>,
    identity: &census_domain::model::StoreIdentity,
    input: NewPrepared<'s>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Application> {
    let NewPrepared {
        mut record,
        mut inspection,
        introduced,
    } = input;
    if !inspection
        .state
        .ledger
        .refresh(guard.store(), view, &record.command.store, grant)?
    {
        drop(inspection);
        drop(introduced);
        return replay::fresh(view, identity, record, true, grant);
    }
    if let Some(application) = replay::repeated(&inspection.state.ledger, &record, false, grant)? {
        return Ok(application);
    }
    require_new(guard.store(), view, &inspection, &record.command, grant)?;
    let inputs = super::ledger::wire::EntryInputs {
        capture: inspection.state.inputs.capture.reference().clone(),
        token: inspection.state.inputs.token.reference().clone(),
    };
    let append = commit::FinalAppend {
        draft: &mut record,
        introduced,
        inputs,
    };
    commit::commit(guard, view, inspection.state.ledger, append, grant)
}

fn require_new(
    store: &Store,
    view: &crate::StoreSnapshot<'_>,
    inspection: &super::inspection::EvidenceSubjectAttachmentInspectionV1<'_>,
    command: &census_domain::evidence_subject_attachment::RetainedEvidenceSubjectAttachmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    inspection
        .state
        .revision
        .ensure_current(store, command.operation.as_str())?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if inspection.state.ledger.inventory != command.expected_attachment_inventory {
        return Err(Error::InventoryMoved.into());
    }
    super::inputs::require_head(view, &command.expected_public_head, grant)?;
    inspection.state.inputs.require_current(
        store,
        view,
        command.expected_evidence_generation,
        command.expected_derived_generation,
        grant,
    )?;
    if matches!(command.action, AttachmentActionV1::Attach { .. }) {
        grant.charge_steps(1).map_err(Error::Domain)?;
        let accepted = view
            .accepted_public_identity_snapshot()?
            .ok_or(Error::TargetMoved)?;
        super::target::current(&accepted, &command.target.public_id, grant)?
            .require_command_target(&command.target, grant)?;
    }
    Ok(())
}
