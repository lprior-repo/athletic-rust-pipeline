use super::super::super::{
    codec, enrollment::AuthorityHead, source_error, RosterSourceIssuanceStoreError as Error,
};
use super::super::{
    objects, ObjectRef, PreparedRosterReviewCore, ReviewBasis, RosterSourceReviewDecisionV1,
    RosterSourceReviewFieldsV1, SemanticReview,
};
use super::{
    source, PreparedRosterMemberWithdrawalV1, RosterMemberWithdrawalDecisionV1,
    RosterMemberWithdrawalReviewV1, WithdrawalInput,
};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::StoreIdentity;
use census_domain::roster_affiliation_authority::{
    check_roster_member_witness_v1, RosterAuthorityDigestV1, RosterAuthorityTextV1,
};

struct Inspected<'s> {
    revision: StoreRevision<'s>,
    store: StoreIdentity,
    sequence: u64,
    prior: AuthorityHead,
    prior_head: ObjectRef,
    source_operation: RosterAuthorityTextV1,
    source: source::OriginalSource,
    objects: objects::Objects,
}

impl Store {
    pub fn prepare_roster_member_withdrawal_v1<'s>(
        &'s self,
        source_operation: RosterAuthorityTextV1,
        witness: Vec<u8>,
        fields: RosterMemberWithdrawalReviewV1,
        mut grant: AttachmentReplayGrantV1,
    ) -> StoreResult<PreparedRosterMemberWithdrawalV1<'s>> {
        let _guard = self.lock_appends();
        let result = prepare(self, source_operation, witness, fields, &mut grant);
        match result {
            Ok((revision, core)) => Ok(PreparedRosterMemberWithdrawalV1 {
                revision,
                grant,
                core,
            }),
            Err(error) => {
                grant.poison();
                Err(error)
            }
        }
    }
}

fn prepare<'s>(
    store: &'s Store,
    source_operation: RosterAuthorityTextV1,
    witness: Vec<u8>,
    fields: RosterMemberWithdrawalReviewV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(StoreRevision<'s>, PreparedRosterReviewCore)> {
    grant.ensure_available().map_err(source_error)?;
    if witness.is_empty() || witness.len() > codec::MAX_BYTES {
        return Err(Error::BudgetExceeded.into());
    }
    if fields.operation.as_str().len() > 256 || source_operation.as_str().len() > 256 {
        return Err(Error::OperationConflict.into());
    }
    let mut input = inspect(store, source_operation, grant)?;
    super::check_scope(&input.source.proposal, &fields.scope)?;
    grant.charge_bytes(witness.len()).map_err(source_error)?;
    grant.charge_steps(1).map_err(source_error)?;
    let checked = check_roster_member_witness_v1(
        &input.source.proposal,
        &input.source.interpretation,
        &witness,
        super::super::limits()?,
    )
    .map_err(super::domain)?;
    let target = RosterMemberWithdrawalDecisionV1 {
        source_operation: input.source_operation.clone(),
        source_record: input.source.reference.clone(),
        witness: objects::insert_owned(&mut input.objects, witness, grant)?,
        member_claim: RosterAuthorityDigestV1::parse(checked.claim_id()).map_err(super::domain)?,
        person: checked.source_person().clone(),
        scope: fields.scope,
    };
    let review_fields = RosterSourceReviewFieldsV1 {
        operation: fields.operation,
        actor: fields.actor,
        decision: RosterSourceReviewDecisionV1::WithdrawExactSourceMember(Box::new(target)),
        reviewed_at: fields.reviewed_at,
        rationale: fields.rationale,
    };
    finish(input, review_fields, grant)
}

fn inspect<'s>(
    store: &'s Store,
    source_operation: RosterAuthorityTextV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Inspected<'s>> {
    super::require_owner(store)?;
    let view = store.snapshot_locked();
    let snapshot = store.db.snapshot();
    let head = super::io::read(&snapshot, &store.meta, super::super::super::HEAD, grant)?;
    let prior: AuthorityHead = codec::decode(&head)?;
    let source = source::load(store, &snapshot, &source_operation, grant)?;
    let identity = view.store_identity()?;
    if source.record.basis.store != identity {
        return Err(Error::OperationConflict.into());
    }
    let mut objects = objects::Objects::new();
    let prior_head = objects::insert_owned(&mut objects, head, grant)?;
    Ok(Inspected {
        revision: view.revision(),
        store: identity,
        sequence: view.sequence(),
        prior,
        prior_head,
        source_operation,
        source,
        objects,
    })
}

fn input_token(
    input: &mut Inspected<'_>,
    target: &RosterMemberWithdrawalDecisionV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ObjectRef> {
    let token = WithdrawalInput {
        schema: super::INPUT_SCHEMA.into(),
        store: input.store.clone(),
        sequence: input.sequence,
        evidence_generation: input.revision.evidence_generation(),
        derived_generation: input.revision.derived_generation(),
        source_operation: target.source_operation.clone(),
        source_record: target.source_record.clone(),
        witness: target.witness.clone(),
        scope: target.scope.clone(),
        prior_head: input.prior_head.clone(),
    };
    let bytes = codec::encode_with_source_grant(&token, grant)?;
    objects::insert_owned(&mut input.objects, bytes, grant)
}

fn finish<'s>(
    mut input: Inspected<'s>,
    fields: RosterSourceReviewFieldsV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(StoreRevision<'s>, PreparedRosterReviewCore)> {
    let RosterSourceReviewDecisionV1::WithdrawExactSourceMember(target) = &fields.decision else {
        return Err(Error::OperationConflict.into());
    };
    let token = input_token(&mut input, target, grant)?;
    let source = input.source;
    let semantic = SemanticReview {
        schema: super::REVIEW_SCHEMA.into(),
        provenance: "explicit-unauthenticated-local-operator".into(),
        fields,
        decision: "withdraw-exact-source-member-claim".into(),
        qualification: source.review.qualification,
        activation: source.review.activation,
        store: input.store.clone(),
        input_token: token.clone(),
        prior_head: input.prior_head.clone(),
        proposal_basis: source.record.basis.proposal_review_basis.clone(),
        scope_policy: source.review.scope_policy,
        year_convention: source.review.year_convention,
        projection_merge_contract: source.review.projection_merge_contract,
    };
    let raw = codec::encode_with_source_grant(&semantic, grant)?;
    let review = objects::insert_owned(&mut input.objects, raw, grant)?;
    let closure = closure(source.record.basis.closure, &input.objects, grant)?;
    let record = ReviewBasis {
        schema: super::BASIS_SCHEMA.into(),
        store: input.store,
        operation: semantic.fields.operation,
        inspected_sequence: input.sequence,
        evidence_generation: input.revision.evidence_generation(),
        derived_generation: input.revision.derived_generation(),
        input_token: token,
        prior_head: input.prior_head,
        next_ordinal: input
            .prior
            .ordinal
            .checked_add(1)
            .ok_or(Error::BudgetExceeded)?,
        proposal: source.record.basis.proposal,
        review,
        proposal_review_basis: source.record.basis.proposal_review_basis,
        physical_inventory: source.record.basis.physical_inventory,
        closure,
    };
    let approval = super::super::review::approval(&record)?;
    let approval_digest = codec::digest(&approval);
    Ok((
        input.revision,
        PreparedRosterReviewCore {
            record,
            objects: input.objects,
            approval,
            approval_digest,
        },
    ))
}

fn closure(
    mut prior: Vec<ObjectRef>,
    added: &objects::Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<ObjectRef>> {
    let count = prior
        .len()
        .checked_add(added.len())
        .ok_or(Error::BudgetExceeded)?;
    if count > 4096 {
        return Err(Error::BudgetExceeded.into());
    }
    let bytes = added
        .len()
        .checked_mul(std::mem::size_of::<ObjectRef>() + 64)
        .ok_or(Error::BudgetExceeded)?;
    grant.charge_bytes(bytes).map_err(source_error)?;
    grant.charge_steps(count).map_err(source_error)?;
    prior
        .try_reserve(added.len())
        .map_err(|_| Error::BudgetExceeded)?;
    for (digest, raw) in added {
        prior.push(ObjectRef {
            digest: digest.clone(),
            bytes: raw.len(),
        });
    }
    prior.sort_by(|a, b| a.digest.cmp(&b.digest));
    prior.dedup_by(|a, b| a.digest == b.digest && a.bytes == b.bytes);
    let bytes = prior.iter().try_fold(0_usize, |sum, reference| {
        sum.checked_add(reference.bytes)
            .ok_or(Error::BudgetExceeded)
    })?;
    if bytes > 8 * 1024 * 1024 {
        return Err(Error::BudgetExceeded.into());
    }
    Ok(prior)
}
