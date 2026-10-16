use super::{
    guard::AttachmentCommitGuard,
    ledger::Ledger,
    prepare::{PreparedEvidenceSubjectAttachmentV1, PreparedState},
    EvidenceSubjectAttachmentStoreError as Error,
};
use crate::{Application, Store, StoreResult};
use census_domain::evidence_subject_attachment::{
    AttachmentActionV1, AttachmentDigestV1, AttachmentReplayGrantV1,
};

mod commit;

impl Store {
    pub fn apply_evidence_subject_attachment_v1(
        &self,
        mut prepared: PreparedEvidenceSubjectAttachmentV1<'_>,
        approval: &[u8],
        digest: &AttachmentDigestV1,
    ) -> StoreResult<Application> {
        prepared.grant.ensure_available().map_err(Error::Domain)?;
        let result = apply(self, &mut prepared, approval, digest);
        if result.is_err() {
            prepared.grant.poison();
        }
        result
    }
}

fn apply(
    store: &Store,
    prepared: &mut PreparedEvidenceSubjectAttachmentV1<'_>,
    approval: &[u8],
    digest: &AttachmentDigestV1,
) -> StoreResult<Application> {
    prepared
        .revision
        .ensure_origin(store, prepared.record.command.operation.as_str())?;
    prepared.grant.charge_steps(1).map_err(Error::Domain)?;
    if approval != prepared.approval || digest != &prepared.record.approval_digest {
        return Err(Error::Corrupt("explicit attachment approval differs").into());
    }
    let guard = AttachmentCommitGuard::acquire(store);
    let view = guard.snapshot();
    super::inputs::require_format(store)?;
    prepared.grant.charge_steps(1).map_err(Error::Domain)?;
    let identity = view.store_identity()?;
    if identity != prepared.record.command.store {
        return Err(Error::InputMoved.into());
    }
    match std::mem::replace(&mut prepared.state, PreparedState::Replay) {
        PreparedState::New {
            inspection,
            introduced,
        } => apply_new(guard, &view, &identity, prepared, inspection, introduced),
        PreparedState::Replay => replay_current(&view, &identity, prepared, false),
    }
}

fn apply_new<'s>(
    guard: AttachmentCommitGuard<'s>,
    view: &crate::StoreSnapshot<'s>,
    identity: &census_domain::model::StoreIdentity,
    prepared: &mut PreparedEvidenceSubjectAttachmentV1<'_>,
    mut inspection: super::inspection::EvidenceSubjectAttachmentInspectionV1<'s>,
    introduced: Vec<super::material::Material>,
) -> StoreResult<Application> {
    if !inspection.state.ledger.refresh(
        guard.store(),
        view,
        &prepared.record.command.store,
        &mut prepared.grant,
    )? {
        drop(inspection);
        drop(introduced);
        return replay_current(view, identity, prepared, true);
    }
    if let Some(application) = repeated(&inspection.state.ledger, prepared, false)? {
        return Ok(application);
    }
    require_new(
        guard.store(),
        view,
        &inspection,
        &prepared.record.command,
        &mut prepared.grant,
    )?;
    let inputs = super::ledger::wire::EntryInputs {
        capture: inspection.state.inputs.capture.reference().clone(),
        token: inspection.state.inputs.token.reference().clone(),
    };
    let append = commit::FinalAppend {
        draft: &mut prepared.record,
        introduced,
        inputs,
    };
    commit::commit(
        guard,
        view,
        inspection.state.ledger,
        append,
        &mut prepared.grant,
    )
}

fn replay_current(
    view: &crate::StoreSnapshot<'_>,
    identity: &census_domain::model::StoreIdentity,
    prepared: &mut PreparedEvidenceSubjectAttachmentV1<'_>,
    changed: bool,
) -> StoreResult<Application> {
    let ledger = Ledger::load(view, identity, &mut prepared.grant)?;
    if let Some(application) = repeated(&ledger, prepared, !changed)? {
        return Ok(application);
    }
    if changed {
        return Err(Error::InventoryMoved.into());
    }
    Err(Error::Corrupt("replay-only prepared record disappeared").into())
}

fn repeated(
    ledger: &Ledger<'_>,
    prepared: &mut PreparedEvidenceSubjectAttachmentV1<'_>,
    exact_record: bool,
) -> StoreResult<Option<Application>> {
    let Some(index) = ledger.find_operation(
        prepared.record.command.operation.as_str(),
        &mut prepared.grant,
    )?
    else {
        return Ok(None);
    };
    prepared.grant.charge_steps(1).map_err(Error::Domain)?;
    let record = ledger
        .records
        .get(index)
        .ok_or(Error::Corrupt("replay record missing"))?;
    if record.command != prepared.record.command
        || record.approval_digest != prepared.record.approval_digest
        || (exact_record && record != &prepared.record)
    {
        return Err(Error::OperationConflict.into());
    }
    Ok(Some(Application::Repeated(
        ledger
            .receipts
            .get(index)
            .ok_or(Error::Corrupt("replay receipt missing"))?
            .clone(),
    )))
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
