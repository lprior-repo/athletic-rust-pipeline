use super::super::super::{
    codec, enrollment::AuthorityHead, RosterSourceIssuanceStoreError as Error,
};
use super::super::{
    ledger, objects, proposal, ObjectRef, ReviewBasis, ReviewRecord, RosterSourceReviewDecisionV1,
    SemanticReview,
};
use super::{RosterMemberWithdrawalDecisionV1, BASIS_SCHEMA, INPUT_SCHEMA, REVIEW_SCHEMA};
use crate::{StoreError, StoreResult};
use census_domain::model::StoreIdentity;
use census_domain::roster_affiliation_authority::{
    check_roster_member_witness_v1, LocallyCheckedRosterIssuanceV1, RosterAuthorityError,
};

pub(in super::super) fn record(
    record: &ReviewRecord,
    objects: &objects::Objects,
    identity: &StoreIdentity,
    prior: &AuthorityHead,
    source_records: &[(Vec<u8>, Vec<u8>)],
) -> StoreResult<()> {
    if record.basis.schema != BASIS_SCHEMA
        || &record.basis.store != identity
        || prior.version != 1
        || prior.ordinal.checked_add(1) != Some(record.basis.next_ordinal)
    {
        return Err(Error::Corrupt("withdrawal basis lineage").into());
    }
    let review: SemanticReview = codec::decode(ledger::object(objects, &record.basis.review)?)?;
    verify_review(record, &review)?;
    let target = decode_target(&review.fields.decision)?;
    let original = find_original(target, source_records)?;
    verify_original(record, &original, target, objects)?;
    let proposal = proposal(&original.basis.proposal)?;
    super::check_scope(&proposal, target.scope())?;
    let original_review = codec::decode(ledger::object(objects, &original.basis.review)?)?;
    verify_contracts(&review, &original_review, &proposal)?;
    verify_witness(&proposal, target, objects)?;
    verify_input(&record.basis, target, objects)?;
    verify_closure(&record.basis, &original.basis, target, objects)
}

fn decode_target(
    decision: &RosterSourceReviewDecisionV1,
) -> StoreResult<&RosterMemberWithdrawalDecisionV1> {
    match decision {
        RosterSourceReviewDecisionV1::WithdrawExactSourceMember(target) => Ok(target),
        RosterSourceReviewDecisionV1::QualifyAndActivateExactCaptureRepresentation => {
            Err(Error::Corrupt("withdrawal decision variant").into())
        }
    }
}

fn find_original(
    target: &RosterMemberWithdrawalDecisionV1,
    source_records: &[(Vec<u8>, Vec<u8>)],
) -> StoreResult<ReviewRecord> {
    let key = format!(
        "{}{}",
        super::super::RECORD_PREFIX,
        codec::digest(target.source_operation().as_bytes())
    );
    let mut matches = source_records
        .iter()
        .filter(|(key_bytes, _)| key_bytes == key.as_bytes());
    let (_, raw) = matches
        .next()
        .ok_or(Error::Corrupt("original source record absent"))?;
    if matches.next().is_some() {
        return Err(Error::Corrupt("duplicate original source record").into());
    }
    if target.source_record.digest != codec::digest(raw) || target.source_record.bytes != raw.len()
    {
        return Err(Error::Corrupt("source record digest or length").into());
    }
    codec::decode_review_composite(raw)
}

fn verify_original(
    record: &ReviewRecord,
    original: &ReviewRecord,
    target: &RosterMemberWithdrawalDecisionV1,
    objects: &objects::Objects,
) -> StoreResult<()> {
    if original.basis.schema != "roster-source-reviewed-claims-basis-v1"
        || original.basis.operation.as_str() != target.source_operation()
        || original.basis.store != record.basis.store
        || original.basis.next_ordinal >= record.basis.next_ordinal
        || original.committed_at > record.committed_at
        || original.basis.proposal != record.basis.proposal
        || original.basis.physical_inventory != record.basis.physical_inventory
        || original.basis.proposal_review_basis != record.basis.proposal_review_basis
    {
        return Err(Error::Corrupt("original review lineage").into());
    }
    let prior: AuthorityHead = codec::decode(ledger::object(objects, &original.basis.prior_head)?)?;
    if prior.version != 1 || prior.ordinal.checked_add(1) != Some(original.basis.next_ordinal) {
        return Err(Error::Corrupt("original review ordinal").into());
    }
    ledger::validate_issued_record(original, objects, &record.basis.store, &prior)
}

fn verify_review(record: &ReviewRecord, review: &SemanticReview) -> StoreResult<()> {
    let basis = &record.basis;
    if review.schema != REVIEW_SCHEMA
        || review.provenance != "explicit-unauthenticated-local-operator"
        || review.decision != "withdraw-exact-source-member-claim"
        || review.fields.operation != basis.operation
        || review.fields.reviewed_at > record.committed_at
        || review.store != basis.store
        || review.input_token != basis.input_token
        || review.prior_head != basis.prior_head
        || review.proposal_basis != basis.proposal_review_basis
    {
        return Err(Error::Corrupt("withdrawal semantic review").into());
    }
    Ok(())
}

fn verify_contracts(
    review: &SemanticReview,
    original: &SemanticReview,
    proposal: &LocallyCheckedRosterIssuanceV1,
) -> StoreResult<()> {
    if review.qualification != original.qualification
        || review.activation != original.activation
        || review.scope_policy != original.scope_policy
        || review.year_convention != original.year_convention
        || review.projection_merge_contract != original.projection_merge_contract
        || &review.scope_policy != proposal.scope().policy()
        || &review.year_convention != proposal.binding().year_convention()
        || &review.projection_merge_contract != proposal.binding().projection_merge_contract()
    {
        return Err(Error::Corrupt("withdrawal original contracts").into());
    }
    Ok(())
}

fn verify_witness(
    proposal: &LocallyCheckedRosterIssuanceV1,
    target: &RosterMemberWithdrawalDecisionV1,
    objects: &objects::Objects,
) -> StoreResult<()> {
    let interpretation = objects
        .get(proposal.capture().interpretation().digest().as_str())
        .ok_or(Error::Corrupt("interpretation object absent"))?;
    let witness = ledger::object(objects, &target.witness)?;
    let checked =
        check_roster_member_witness_v1(proposal, interpretation, witness, super::super::limits()?)
            .map_err(witness_error)?;
    if checked.claim_id() != target.member_claim() || checked.source_person() != target.person() {
        return Err(Error::Corrupt("withdrawal member witness target").into());
    }
    Ok(())
}

fn witness_error(error: RosterAuthorityError) -> StoreError {
    match error {
        RosterAuthorityError::Capacity {
            resource,
            needed,
            available,
        } => StoreError::BudgetExceeded {
            what: resource,
            needed,
            available,
        },
        RosterAuthorityError::Overflow | RosterAuthorityError::Allocation => {
            Error::BudgetExceeded.into()
        }
        RosterAuthorityError::Invalid(reason) => Error::Corrupt(reason).into(),
        _ => Error::Corrupt("withdrawal witness verification").into(),
    }
}

fn verify_input(
    basis: &ReviewBasis,
    target: &RosterMemberWithdrawalDecisionV1,
    objects: &objects::Objects,
) -> StoreResult<()> {
    let input: super::WithdrawalInput =
        codec::decode(ledger::object(objects, &basis.input_token)?)?;
    if input.schema != INPUT_SCHEMA
        || input.store != basis.store
        || input.sequence != basis.inspected_sequence
        || input.evidence_generation != basis.evidence_generation
        || input.derived_generation != basis.derived_generation
        || input.source_operation != target.source_operation
        || input.source_record != target.source_record
        || input.witness != target.witness
        || input.scope != target.scope
        || input.prior_head != basis.prior_head
    {
        return Err(Error::Corrupt("withdrawal input").into());
    }
    Ok(())
}

fn verify_closure(
    basis: &ReviewBasis,
    original: &ReviewBasis,
    target: &RosterMemberWithdrawalDecisionV1,
    objects: &objects::Objects,
) -> StoreResult<()> {
    original
        .closure
        .iter()
        .chain([
            &target.witness,
            &basis.review,
            &basis.input_token,
            &basis.prior_head,
            &basis.proposal_review_basis,
            &basis.physical_inventory,
        ])
        .try_for_each(|reference| require_reference(basis, reference, objects))
}

fn require_reference(
    basis: &ReviewBasis,
    reference: &ObjectRef,
    objects: &objects::Objects,
) -> StoreResult<()> {
    if !basis.closure.contains(reference) {
        return Err(Error::Corrupt("withdrawal reference outside own closure").into());
    }
    if codec::digest(ledger::object(objects, reference)?) != reference.digest {
        return Err(Error::Corrupt("withdrawal closure object digest").into());
    }
    Ok(())
}
