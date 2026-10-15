use super::{MembershipWithholdingInventoryV1, PreparedMembershipWithholdingV1};
mod capture;
use crate::public_identity::{
    codec, context,
    local_adjudication::{admission, affected, claims, prepare::members},
    PublicIdentityError, PublicIdentityWriterV1,
};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, RetainedClaimDispositionV1, RetainedClaimV1, RetainedIdentitySectionV1,
};
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, LocalMembershipWithholdingRecordV1,
};
use census_domain::public_identity_history::{
    prepare_next, HistoryProjection, PreparedHistoryRecord, PublicAthleteId, RegistryHead,
    ReplayLimits,
};

impl PublicIdentityWriterV1 {
    pub fn inspect_membership_withholding(
        &self,
        store: &Store,
        canonical: &PublicAthleteId,
    ) -> StoreResult<MembershipWithholdingInventoryV1> {
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
        Ok(MembershipWithholdingInventoryV1 {
            authority_context,
            affected_context: affected.measure_context()?,
            affected_record_keys,
            claims,
        })
    }

    pub fn prepare_membership_withholding<'s>(
        &self,
        store: &'s Store,
        operation: &str,
        request: LocalMembershipWithholdV1,
        expected: StoreRevision<'s>,
        head: &RegistryHead,
    ) -> StoreResult<PreparedMembershipWithholdingV1<'s>> {
        let reserved =
            self.reserve_membership_withholding(store, operation, request, expected, head)?;
        let [authority, affected] = &reserved.permits;
        reserved
            .inputs
            .stage_admitted(store, &self.identity, authority)?;
        reserved.affected.stage_admitted(store, affected)?;
        let _guard = store.lock_appends();
        expected.ensure_current(store, operation)?;
        if &reserved.prepared.record.expected_head
            != self.verified(store)?.history.projection().head()
        {
            return Err(PublicIdentityError::HeadMoved.into());
        }
        Ok(reserved.prepared)
    }

    fn reserve_membership_withholding<'s>(
        &self,
        store: &'s Store,
        operation: &str,
        request: LocalMembershipWithholdV1,
        expected: StoreRevision<'s>,
        head: &RegistryHead,
    ) -> StoreResult<Reserved<'s>> {
        codec::encode(&request, 1024 * 1024)?;
        let guard = store.lock_appends();
        expected.ensure_current(store, operation)?;
        let ledger = self.verified(store)?;
        self.require_current_qualification()?;
        if head != ledger.history.projection().head() {
            return Err(PublicIdentityError::HeadMoved.into());
        }
        let reserved = admission::load(store, &self.identity, operation)?;
        if let Some(reserved) = &reserved {
            reserved.require_withholding_request(operation, &request, expected, head)?;
        }
        let snapshot = store.snapshot_locked();
        let captured = capture::capture(
            store,
            &snapshot,
            &self.identity,
            &request,
            &ledger,
            reserved.as_ref(),
        )?;
        let record = match &reserved {
            Some(reserved) => {
                reserved.require_allowance(&captured.charges)?;
                reserved.withholding_record()?.clone()
            }
            None => captured.fresh_record(&self.identity, operation, request, head, &snapshot)?,
        };
        super::fence::validate_live(store, &record, ledger.history.records())?;
        let history = prepare_history(&record, &captured.inventory, ledger.history.projection())?;
        preview(&record, &history)?;
        let capture::Captured {
            inputs,
            affected,
            charges,
            ..
        } = captured;
        let reservation = match reserved {
            Some(reserved) => reserved,
            None => admission::reserve_withholding(store, record.clone(), charges)?,
        };
        let permits = reservation.permits(store);
        drop(guard);
        Ok(Reserved::new(
            inputs, affected, record, expected, history, permits,
        ))
    }
}

struct Reserved<'s> {
    inputs: context::Inputs,
    affected: affected::Inputs,
    permits: [admission::Permit<'s>; 2],
    prepared: PreparedMembershipWithholdingV1<'s>,
}

impl<'s> Reserved<'s> {
    fn new(
        inputs: context::Inputs,
        affected: affected::Inputs,
        record: LocalMembershipWithholdingRecordV1,
        expected: StoreRevision<'s>,
        history: PreparedHistoryRecord,
        permits: [admission::Permit<'s>; 2],
    ) -> Self {
        Self {
            inputs,
            affected,
            permits,
            prepared: PreparedMembershipWithholdingV1 {
                record,
                expected,
                history,
            },
        }
    }
}

fn preview(
    record: &LocalMembershipWithholdingRecordV1,
    history: &PreparedHistoryRecord,
) -> StoreResult<()> {
    let approval = super::consume::approval_bytes(record)?;
    let witness = super::MembershipWithholdingWitnessV1::new(
        record.clone(),
        history,
        super::consume::approval_digest(&approval),
    );
    codec::encode(&witness, 1024 * 1024)?;
    Ok(())
}

fn dispositions(
    request: &LocalMembershipWithholdV1,
    claims: &[RetainedClaimV1],
) -> StoreResult<Vec<RetainedClaimDispositionV1>> {
    let mut result = Vec::new();
    result
        .try_reserve(claims.len())
        .map_err(|error| crate::StoreError::Refused {
            detail: format!("withholding disposition allocation: {error}"),
        })?;
    let mut remaining: usize = 1024 * 1024;
    for claim in claims {
        let target = claim.section == RetainedIdentitySectionV1::CandidateMembership
            && claim.ordinal == request.attachment.sequence
            && claim.row_digest == request.attachment.digest
            && claim.claim_path == request.attachment.membership_id;
        let disposition = RetainedClaimDispositionV1 {
            claim: claim.clone(),
            treatment: if target {
                ClaimTreatmentV1::RejectIdentityCorroboration
            } else {
                ClaimTreatmentV1::RetainAsContext
            },
            rationale: request.rationale.clone(),
        };
        remaining = remaining
            .checked_sub(codec::encode(&disposition, remaining)?.len())
            .ok_or(crate::StoreError::CounterOverflow)?;
        result.push(disposition);
    }
    Ok(result)
}

pub(super) fn prepare_history(
    record: &LocalMembershipWithholdingRecordV1,
    claims: &[RetainedClaimV1],
    projection: &HistoryProjection,
) -> StoreResult<PreparedHistoryRecord> {
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
    let after = projection
        .clone()
        .extend_candidate(&history.uncommitted_record)
        .map_err(PublicIdentityError::from)?;
    if !history.established_ids.is_empty()
        || after.withheld_memberships(history.next_head.sequence)
            != Some(record.request.expected_withheld_memberships.as_slice())
    {
        return Err(PublicIdentityError::Corrupt("withholding computed member set").into());
    }
    Ok(history)
}
