use super::{
    guard::AttachmentCommitGuard,
    inspection::EvidenceSubjectAttachmentInspectionV1,
    ledger::Ledger,
    material::{AttachmentContextObjectV1, Material},
    EvidenceSubjectAttachmentStoreError as Error,
};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentReplayGrantV1, RetainedEvidenceSubjectAttachmentCommandV1,
    ReviewedReferenceRecordV1,
};

mod context;
mod new;

pub enum EvidenceSubjectAttachmentPreparationV1<'s, 'a> {
    New {
        inspection: EvidenceSubjectAttachmentInspectionV1<'s>,
        context: &'a [AttachmentContextObjectV1<'a>],
    },
    ReplayOnly,
}

pub struct PreparedEvidenceSubjectAttachmentV1<'s> {
    pub(super) revision: StoreRevision<'s>,
    pub(super) record: ReviewedReferenceRecordV1,
    pub(super) approval: Vec<u8>,
    pub(super) state: PreparedState<'s>,
    pub(super) grant: AttachmentReplayGrantV1,
}

pub(super) enum PreparedState<'s> {
    New {
        inspection: EvidenceSubjectAttachmentInspectionV1<'s>,
        introduced: Vec<Material>,
    },
    Replay,
}

impl PreparedEvidenceSubjectAttachmentV1<'_> {
    pub fn command(&self) -> &RetainedEvidenceSubjectAttachmentCommandV1 {
        &self.record.command
    }
    pub fn approval_bytes(&self) -> &[u8] {
        &self.approval
    }
    pub fn approval_digest(&self) -> &AttachmentDigestV1 {
        &self.record.approval_digest
    }
    pub fn is_replay(&self) -> bool {
        matches!(self.state, PreparedState::Replay)
    }
}

impl Store {
    pub fn prepare_evidence_subject_attachment_v1<'s>(
        &'s self,
        command: RetainedEvidenceSubjectAttachmentCommandV1,
        input: EvidenceSubjectAttachmentPreparationV1<'s, '_>,
    ) -> StoreResult<PreparedEvidenceSubjectAttachmentV1<'s>> {
        let (mut grant, proposed) = match input {
            EvidenceSubjectAttachmentPreparationV1::New {
                mut inspection,
                context,
            } => {
                inspection
                    .revision
                    .ensure_origin(self, command.operation.as_str())?;
                let grant = inspection
                    .grant
                    .take()
                    .ok_or(Error::Corrupt("inspection grant already consumed"))?;
                (grant, Some((inspection, context)))
            }
            EvidenceSubjectAttachmentPreparationV1::ReplayOnly => {
                (AttachmentReplayGrantV1::new(), None)
            }
        };
        let result = prepare(self, command, proposed, &mut grant);
        match result {
            Ok((revision, record, approval, state)) => Ok(PreparedEvidenceSubjectAttachmentV1 {
                revision,
                record,
                approval,
                state,
                grant,
            }),
            Err(error) => {
                grant.poison();
                Err(error)
            }
        }
    }
}

type PreparedParts<'s> = (
    StoreRevision<'s>,
    ReviewedReferenceRecordV1,
    Vec<u8>,
    PreparedState<'s>,
);

fn prepare<'s>(
    store: &'s Store,
    command: RetainedEvidenceSubjectAttachmentCommandV1,
    proposed: Option<(
        EvidenceSubjectAttachmentInspectionV1<'s>,
        &[AttachmentContextObjectV1<'_>],
    )>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<PreparedParts<'s>> {
    let guard = AttachmentCommitGuard::acquire(store);
    let view = guard.snapshot();
    super::inputs::require_format(store)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let identity = view.store_identity()?;
    if command.store != identity {
        return Err(Error::InputMoved.into());
    }
    let mut ledger = Ledger::load(&view, &identity, grant)?;
    let approval = command
        .approval_bytes_with_grant(4 * 1024 * 1024, grant)
        .map_err(Error::Domain)?;
    if let Some(index) = ledger.find_operation(command.operation.as_str(), grant)? {
        grant.charge_steps(1).map_err(Error::Domain)?;
        let retained = ledger
            .records
            .get(index)
            .ok_or(Error::Corrupt("replay record index"))?;
        if retained.command != command {
            return Err(Error::OperationConflict.into());
        }
        let record = ledger.records.remove(index);
        return Ok((view.revision(), record, approval, PreparedState::Replay));
    }
    let (inspection, context) =
        proposed.ok_or(Error::Corrupt("new attachment requires an inspection"))?;
    let input = new::NewPreparation {
        command,
        inspection,
        supplied: context,
        approval,
    };
    new::prepare(store, &view, input, ledger, grant)
}
