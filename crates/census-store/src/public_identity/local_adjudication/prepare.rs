use super::{affected, claims, LocalAdjudicationInventoryV1, PreparedLocalAdjudicationV1};
use crate::clock::{Clock, SystemClock};
use crate::public_identity::{codec, context, PublicIdentityError, PublicIdentityWriterV1};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::local_adjudication::{
    LocalEvidenceBasisV1, LocalIdentityAdjudicationRecordV2, LocalIdentitySplitV1,
    LOCAL_ADJUDICATION_POLICY,
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
        let claims = claims::inventory(&inputs, &members)?;
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
        validate_request(&request)?;
        let guard = store.lock_appends();
        expected.ensure_current(store, operation)?;
        let ledger = self.verified(store)?;
        self.require_current_qualification()?;
        if head != ledger.history.projection().head() {
            return Err(PublicIdentityError::HeadMoved.into());
        }
        validate_prior(&request, ledger.history.records())?;
        let snapshot = store.snapshot_locked();
        let inputs = context::Inputs::capture(&snapshot)?;
        let members = members(&inputs, ledger.history.projection(), &request.canonical)?;
        validate_case(&inputs, &request, &members)?;
        let claims = claims::inventory(&inputs, &members)?;
        let authority_context = inputs.measure_context(&self.identity)?;
        let affected = affected::Inputs::capture(&snapshot, &self.identity, &authority_context)?;
        if affected.record_keys(&inputs, &members)? != request.affected_record_keys {
            return Err(PublicIdentityError::Corrupt("manual affected record inventory").into());
        }
        drop(guard);
        let authority_context = inputs.stage(store, &self.identity)?.object_id().to_owned();
        let affected_context = affected.stage(store)?.object_id().to_owned();
        let record = LocalIdentityAdjudicationRecordV2 {
            version: 2,
            policy: LOCAL_ADJUDICATION_POLICY.into(),
            store: self.identity.clone(),
            operation: operation.into(),
            expected_head: head.clone(),
            evidence_generation: snapshot.evidence_generation(),
            derived_generation: snapshot.derived_generation(),
            snapshot_sequence: snapshot.sequence(),
            authority_context,
            affected_context,
            members,
            request,
            observed_at: SystemClock.today_iso8601(),
            evidence_basis: LocalEvidenceBasisV1::RetainedObservationOnly,
        };
        let history = prepare_history(&record, &claims, ledger.history.projection())?;
        preview_witness(&record, &history)?;
        validate_current(self, store, expected, &record)?;
        Ok(PreparedLocalAdjudicationV1 {
            record,
            expected,
            history,
        })
    }
}

pub(super) fn members(
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
        if record.digest != prior.digest || !matches!(record.request, HistoryRequest::Merge { .. })
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
