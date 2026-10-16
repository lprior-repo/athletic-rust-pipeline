mod capture;
use super::{
    admission, affected, claims, LocalAdjudicationInventoryV1, PreparedLocalAdjudicationV1,
};
use crate::public_identity::{codec, context, PublicIdentityError, PublicIdentityWriterV1};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::local_adjudication::{
    LocalIdentityAdjudicationRecordV2, LocalIdentitySplitV1,
};
use census_domain::public_identity_history::{
    prepare_next, HistoryProjection, HistoryRequest, PublicAthleteId, PublicResolution,
    RegistryHead, ReplayLimits,
};

impl PublicIdentityWriterV1 {
    pub fn inspect_local_adjudication(
        &self,
        store: &Store,
        canonical: &PublicAthleteId,
    ) -> StoreResult<LocalAdjudicationInventoryV1> {
        let guard = store.lock_appends();
        let ledger = self.verified(store)?;
        self.require_current_qualification()?;
        let snapshot = store.snapshot_locked();
        let inputs = context::Inputs::capture(&snapshot)?;
        let members = members(&inputs, ledger.history.projection(), canonical)?;
        let claims = claims::membership_inventory(
            &inputs,
            &members,
            ledger.history.projection(),
            ledger.history.records(),
        )?;
        let authority_context = inputs.measure_context(&self.identity)?;
        let affected = affected::Inputs::capture(&snapshot, &self.identity, &authority_context)?;
        let affected_record_keys = affected.record_keys(&inputs, &members)?;
        drop(guard);
        Ok(LocalAdjudicationInventoryV1 {
            authority_context,
            affected_context: affected.measure_context()?,
            affected_record_keys,
            claims,
        })
    }

    pub fn prepare_local_adjudication<'s>(
        &self,
        store: &'s Store,
        operation: &str,
        request: LocalIdentitySplitV1,
        expected: StoreRevision<'s>,
        head: &RegistryHead,
    ) -> StoreResult<PreparedLocalAdjudicationV1<'s>> {
        let staged = self.reserve_local_adjudication(store, operation, request, expected, head)?;
        let [authority, affected] = &staged.permits;
        staged
            .inputs
            .stage_admitted(store, &self.identity, authority)?;
        staged.affected.stage_admitted(store, affected)?;
        validate_current(self, store, expected, &staged.prepared.record)?;
        Ok(staged.prepared)
    }

    pub(in crate::public_identity) fn reserve_local_adjudication<'s>(
        &self,
        store: &'s Store,
        operation: &str,
        request: LocalIdentitySplitV1,
        expected: StoreRevision<'s>,
        head: &RegistryHead,
    ) -> StoreResult<ReservedPreparation<'s>> {
        validate_request(&request)?;
        let guard = store.lock_appends();
        expected.ensure_current(store, operation)?;
        let ledger = self.verified(store)?;
        self.require_current_qualification()?;
        if head != ledger.history.projection().head() {
            return Err(PublicIdentityError::HeadMoved.into());
        }
        validate_prior(&request, ledger.history.records())?;
        let reserved = admission::load(store, &self.identity, operation)?;
        if let Some(reserved) = &reserved {
            reserved.require_request(operation, &request, expected, head)?;
        }
        let snapshot = store.snapshot_locked();
        let captured = capture::capture(
            &snapshot,
            &self.identity,
            &request,
            ledger.history.projection(),
            ledger.history.records(),
            reserved.as_ref(),
        )?;
        let charges = [
            captured.inputs.admission_charge(store, &self.identity)?,
            captured.affected.admission_charge(store)?,
        ];
        let record = match &reserved {
            Some(reserved) => {
                reserved.require_allowance(&charges)?;
                reserved.split_record()?.clone()
            }
            None => capture::fresh_record(
                self, operation, request, head, &snapshot, &captured, &charges,
            ),
        };
        let history = prepare_history(&record, &captured.claims, ledger.history.projection())?;
        preview_witness(&record, &history)?;
        let reservation = match reserved {
            Some(reservation) => reservation,
            None => admission::reserve(store, record.clone(), charges)?,
        };
        let permits = reservation.permits(store);
        drop(guard);
        Ok(ReservedPreparation::new(
            captured, record, expected, history, permits,
        ))
    }
}

pub(in crate::public_identity) struct ReservedPreparation<'s> {
    pub(in crate::public_identity) permits: [admission::Permit<'s>; 2],
    pub(in crate::public_identity) inputs: context::Inputs,
    pub(in crate::public_identity) affected: affected::Inputs,
    pub(in crate::public_identity) prepared: PreparedLocalAdjudicationV1<'s>,
}

impl<'s> ReservedPreparation<'s> {
    fn new(
        captured: capture::Captured,
        record: LocalIdentityAdjudicationRecordV2,
        expected: StoreRevision<'s>,
        history: census_domain::public_identity_history::PreparedHistoryRecord,
        permits: [admission::Permit<'s>; 2],
    ) -> Self {
        Self {
            permits,
            inputs: captured.inputs,
            affected: captured.affected,
            prepared: PreparedLocalAdjudicationV1 {
                record,
                expected,
                history,
            },
        }
    }
}

pub(in crate::public_identity) fn members(
    inputs: &context::Inputs,
    projection: &HistoryProjection,
    canonical: &PublicAthleteId,
) -> StoreResult<Vec<AthleteCandidateId>> {
    let mut members = Vec::new();
    for athlete in &inputs.athletes {
        let candidate = athlete.id.cast();
        let resolution = projection
            .resolve_candidate(&candidate)
            .map_err(PublicIdentityError::from)?;
        let current = match resolution {
            PublicResolution::Canonical(row)
            | PublicResolution::Redirect { successor: row, .. }
            | PublicResolution::RevokedRedirect { successor: row, .. } => row.public_id,
            _ => continue,
        };
        if &current == canonical {
            if members.len() >= ReplayLimits::SMALL.members_per_operation {
                return Err(PublicIdentityError::Corrupt("manual member bound").into());
            }
            members
                .try_reserve(1)
                .map_err(|error| crate::StoreError::Refused {
                    detail: format!("manual member allocation: {error}"),
                })?;
            members.push(candidate);
        }
    }
    members.sort();
    members.dedup();
    Ok(members)
}

pub(super) fn validate_prior(
    request: &LocalIdentitySplitV1,
    records: &[census_domain::public_identity_history::HistoryRecord],
) -> StoreResult<()> {
    for prior in &request.supersedes {
        let record = records
            .iter()
            .find(|row| row.sequence == prior.sequence)
            .ok_or(PublicIdentityError::Corrupt(
                "manual superseded merge missing",
            ))?;
        if record.digest != prior.digest
            || !matches!(
                record.request,
                HistoryRequest::Merge { .. } | HistoryRequest::BindCandidateMembership { .. }
            )
        {
            return Err(PublicIdentityError::Corrupt("manual superseded merge mismatch").into());
        }
    }
    Ok(())
}

pub(super) fn validate_case(
    inputs: &context::Inputs,
    request: &LocalIdentitySplitV1,
    members: &[AthleteCandidateId],
) -> StoreResult<()> {
    let case = inputs
        .cases
        .iter()
        .find(|case| case.id == request.case_id)
        .ok_or(PublicIdentityError::Corrupt("manual review case missing"))?;
    let mut case_members = case.member_ids.clone();
    case_members.sort();
    if case_members != members {
        return Err(PublicIdentityError::Corrupt("manual review case members").into());
    }
    Ok(())
}

fn validate_request(request: &LocalIdentitySplitV1) -> StoreResult<()> {
    if request.supersedes.len() > 128
        || request.partitions.len() > 128
        || request.dispositions.len() > 4096
        || request.unresolved_bindings.len() > 128
        || request.affected_record_keys.len() > 1024
    {
        return Err(PublicIdentityError::Corrupt("manual request collection bound").into());
    }
    codec::encode(request, 1024 * 1024)?;
    Ok(())
}

fn prepare_history(
    record: &LocalIdentityAdjudicationRecordV2,
    claims: &[census_domain::public_identity_history::local_adjudication::RetainedClaimV1],
    projection: &HistoryProjection,
) -> StoreResult<census_domain::public_identity_history::PreparedHistoryRecord> {
    codec::encode(record, 1024 * 1024)?;
    let request = record
        .history_request(claims)
        .map_err(PublicIdentityError::from)?;
    let history = prepare_next(
        &record.store,
        &record.expected_head,
        request,
        ReplayLimits::SMALL,
    )
    .map_err(PublicIdentityError::from)?;
    projection
        .clone()
        .extend_candidate(&history.uncommitted_record)
        .map_err(PublicIdentityError::from)?;
    Ok(history)
}

fn preview_witness(
    record: &LocalIdentityAdjudicationRecordV2,
    history: &census_domain::public_identity_history::PreparedHistoryRecord,
) -> StoreResult<()> {
    let approved_bytes = super::consume::approval_bytes(record)?;
    let witness = super::LocalAdjudicationWitnessV1::new(super::ApprovedLocalAdjudicationV1 {
        record: record.clone(),
        history: history.clone(),
        approval_digest: super::consume::approval_digest(&approved_bytes),
    });
    codec::encode(&witness, super::MAX_WITNESS_BYTES)?;
    Ok(())
}

fn validate_current(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    expected: StoreRevision<'_>,
    record: &LocalIdentityAdjudicationRecordV2,
) -> StoreResult<()> {
    let _guard = store.lock_appends();
    expected.ensure_current(store, &record.operation)?;
    if &record.expected_head != writer.verified(store)?.history.projection().head() {
        return Err(PublicIdentityError::HeadMoved.into());
    }
    super::fence::validate_live(store, record)
}
