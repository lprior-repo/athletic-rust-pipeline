use super::*;
use crate::evidence_subject_attachment::{material::Material, plan::AttachmentJournalPlanV1};
use crate::StoreSnapshot;
use census_domain::evidence_subject_attachment::ReviewedReferenceRecordV1;
use chrono::{DateTime, Utc};

pub(super) struct FinalAppend<'a> {
    pub(super) draft: &'a mut ReviewedReferenceRecordV1,
    pub(super) introduced: Vec<Material>,
    pub(super) inputs: super::super::ledger::wire::EntryInputs,
}

pub(super) fn commit<'s>(
    guard: AttachmentCommitGuard<'s>,
    view: &StoreSnapshot<'s>,
    mut ledger: Ledger<'s>,
    append: FinalAppend<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Application> {
    commit_at(guard, view, &mut ledger, append, grant, Utc::now())
}

fn commit_at<'s>(
    guard: AttachmentCommitGuard<'s>,
    view: &StoreSnapshot<'s>,
    ledger: &mut Ledger<'s>,
    append: FinalAppend<'_>,
    grant: &mut AttachmentReplayGrantV1,
    committed_at: DateTime<Utc>,
) -> StoreResult<Application> {
    let FinalAppend {
        draft,
        introduced,
        inputs,
    } = append;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if committed_at < draft.command.reviewed_at {
        return Err(Error::Corrupt("commit precedes explicit review").into());
    }
    draft.committed_at = committed_at;
    draft.record_digest = draft
        .calculated_record_digest_with_grant(grant)
        .map_err(Error::Domain)?;
    let encoded = census_domain::evidence_subject_attachment::encode_reviewed_reference_with_grant(
        draft,
        4 * 1024 * 1024,
        grant,
    )
    .map_err(Error::Domain)?;
    let record = census_domain::evidence_subject_attachment::decode_reviewed_reference_with_grant(
        &encoded,
        4 * 1024 * 1024,
        grant,
    )
    .map_err(Error::Domain)?;
    ledger.append_proposed(view, record, introduced, inputs, grant)?;
    ledger.validate_projection(&draft.command.store, grant)?;
    let record = ledger
        .records
        .last()
        .ok_or(Error::Corrupt("final record missing"))?;
    super::super::target::verify_historical(ledger, record, grant)?;
    let introduced = ledger
        .introductions
        .last()
        .ok_or(Error::Corrupt("final context missing"))?;
    let ordinal =
        u64::try_from(ledger.records.len()).map_err(|_| crate::StoreError::CounterOverflow)?;
    let inputs = ledger
        .input_refs
        .last()
        .ok_or(Error::Corrupt("final input refs missing"))?;
    let rows =
        super::super::planning::rows(ordinal, record, introduced, inputs, committed_at, grant)?;
    super::super::planning::require_complete_fit(ledger, &rows, grant)?;
    grant.charge_bytes(2048).map_err(Error::Domain)?;
    grant.charge_steps(2).map_err(Error::Domain)?;
    let operation = super::super::keys::receipt(&record.command.operation)?;
    let digest = record.record_digest.as_str().to_owned();
    let plan = AttachmentJournalPlanV1::new(view.revision(), rows, operation, digest, committed_at);
    crate::write_batch::commit_evidence_subject_attachment(guard, plan)
}
