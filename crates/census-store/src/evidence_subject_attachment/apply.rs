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
    let ledger = Ledger::load(&view, &identity, &mut prepared.grant)?;
    if let Some(index) = ledger.find_operation(
        prepared.record.command.operation.as_str(),
        &mut prepared.grant,
    )? {
        prepared.grant.charge_steps(1).map_err(Error::Domain)?;
        let record = ledger
            .records
            .get(index)
            .ok_or(Error::Corrupt("replay record missing"))?;
        if record.command != prepared.record.command
            || record.approval_digest != prepared.record.approval_digest
            || (matches!(prepared.state, PreparedState::Replay) && record != &prepared.record)
        {
            return Err(Error::OperationConflict.into());
        }
        return Ok(Application::Repeated(
            ledger
                .receipts
                .get(index)
                .ok_or(Error::Corrupt("replay receipt missing"))?
                .clone(),
        ));
    }
    let PreparedState::New {
        inspection,
        introduced,
    } = &mut prepared.state
    else {
        return Err(Error::Corrupt("replay-only prepared record disappeared").into());
    };
    require_new(
        store,
        &view,
        &ledger,
        inspection,
        &prepared.record.command,
        &mut prepared.grant,
    )?;
    let introduced = std::mem::take(introduced);
    let inputs = super::ledger::wire::EntryInputs {
        capture: inspection.inputs.capture.reference().clone(),
        token: inspection.inputs.token.reference().clone(),
    };
    let append = commit::FinalAppend {
        draft: &mut prepared.record,
        introduced,
        inputs,
    };
    commit::commit(guard, &view, ledger, append, &mut prepared.grant)
}

fn require_new(
    store: &Store,
    view: &crate::StoreSnapshot<'_>,
    ledger: &Ledger<'_>,
    inspection: &super::inspection::EvidenceSubjectAttachmentInspectionV1<'_>,
    command: &census_domain::evidence_subject_attachment::RetainedEvidenceSubjectAttachmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    inspection
        .revision
        .ensure_current(store, command.operation.as_str())?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if ledger.inventory != command.expected_attachment_inventory
        || !inspection
            .ledger
            .physical
            .fingerprint()
            .same_contents(store, ledger.physical.fingerprint())?
    {
        return Err(Error::InventoryMoved.into());
    }
    super::inputs::require_head(view, &command.expected_public_head, grant)?;
    inspection.inputs.require_current(
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
