use super::super::{
    codec, enrollment::AuthorityHead, format, RosterIssuanceInspectionV1,
    RosterSourceIssuanceStoreError as Error,
};
use super::{
    objects, review, PreparedRosterSourceReviewV1, ReviewBasis, RosterSourceReviewFieldsV1,
    RosterSourceReviewObjectsV1,
};
use crate::{Store, StoreError, StoreResult};
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use fjall::Readable;

impl Store {
    pub fn prepare_roster_source_review_v1<'s>(
        &'s self,
        mut inspection: RosterIssuanceInspectionV1<'s>,
        proposal: LocallyCheckedRosterIssuanceV1,
        fields: RosterSourceReviewFieldsV1,
        supplied: RosterSourceReviewObjectsV1,
    ) -> StoreResult<PreparedRosterSourceReviewV1<'s>> {
        let _guard = self.lock_appends();
        let result = prepare(self, &mut inspection, proposal, fields, supplied);
        match result {
            Ok((record, objects, approval)) => {
                let approval_digest = codec::digest(&approval);
                Ok(PreparedRosterSourceReviewV1 {
                    inspection,
                    record,
                    objects,
                    approval,
                    approval_digest,
                })
            }
            Err(error) => {
                inspection.grant.poison();
                Err(error)
            }
        }
    }
}

type Prepared = (ReviewBasis, objects::Objects, Vec<u8>);

fn prepare(
    store: &Store,
    inspection: &mut RosterIssuanceInspectionV1<'_>,
    proposal: LocallyCheckedRosterIssuanceV1,
    fields: RosterSourceReviewFieldsV1,
    supplied: RosterSourceReviewObjectsV1,
) -> StoreResult<Prepared> {
    require_current(store, inspection, fields.operation.as_str())?;
    require_target(inspection, &proposal)?;
    let mut objects = objects::collect(inspection, supplied, &proposal)?;
    let prior: AuthorityHead = codec::decode(&inspection.authority_head)?;
    let input_token =
        objects::insert(&mut objects, &inspection.input_token, &mut inspection.grant)?;
    let prior_head = objects::insert(
        &mut objects,
        &inspection.authority_head,
        &mut inspection.grant,
    )?;
    inspection
        .grant
        .charge_bytes(64 * 1024)
        .map_err(super::super::source_error)?;
    inspection
        .grant
        .charge_steps(1)
        .map_err(super::super::source_error)?;
    let basis_bytes = review::proposal_basis(&proposal)?;
    let proposal_review_basis = objects::insert(&mut objects, &basis_bytes, &mut inspection.grant)?;
    let (qualification, activation) = super::qualification::derive(
        &proposal,
        &proposal_review_basis,
        fields.decision,
        &mut objects,
        &mut inspection.grant,
    )?;
    let semantic = review::semantic(
        inspection,
        &proposal,
        fields,
        proposal_review_basis.clone(),
        input_token.clone(),
        prior_head.clone(),
        qualification,
        activation,
    )?;
    let raw = codec::encode_with_source_grant(&semantic, &mut inspection.grant)?;
    let review = objects::insert(&mut objects, &raw, &mut inspection.grant)?;
    inspection
        .grant
        .charge_bytes(64 * 1024)
        .map_err(super::super::source_error)?;
    inspection
        .grant
        .charge_steps(1)
        .map_err(super::super::source_error)?;
    let proposal = proposal
        .with_semantic_review_reference(
            &review.digest,
            u64::try_from(review.bytes).map_err(|_| Error::BudgetExceeded)?,
            super::limits()?,
        )
        .map_err(|_| Error::Corrupt("derived review claim reference"))?;
    objects::require_proposal(&objects, &proposal)?;
    let physical_inventory = super::ObjectRef {
        digest: proposal
            .binding()
            .physical_projection_inventory()
            .digest()
            .as_str()
            .into(),
        bytes: usize::try_from(
            proposal
                .binding()
                .physical_projection_inventory()
                .bytes()
                .get(),
        )
        .map_err(|_| Error::BudgetExceeded)?,
    };
    let record = ReviewBasis {
        schema: "roster-source-reviewed-claims-basis-v1".into(),
        store: inspection.store.clone(),
        operation: semantic.fields.operation,
        inspected_sequence: inspection.evidence.snapshot_sequence(),
        evidence_generation: inspection.revision.evidence_generation(),
        derived_generation: inspection.revision.derived_generation(),
        input_token,
        prior_head,
        next_ordinal: prior.ordinal.checked_add(1).ok_or(Error::BudgetExceeded)?,
        proposal: codec::encode_with_source_grant(&proposal, &mut inspection.grant)?,
        review,
        proposal_review_basis,
        physical_inventory,
        closure: objects::inventory(&objects),
    };
    inspection
        .grant
        .charge_bytes(64 * 1024)
        .map_err(super::super::source_error)?;
    inspection
        .grant
        .charge_steps(1)
        .map_err(super::super::source_error)?;
    let approval = review::approval(&record)?;
    inspection
        .evidence
        .require_original_grant(&mut inspection.grant)?;
    Ok((record, objects, approval))
}

pub(super) fn require_current(
    store: &Store,
    inspection: &mut RosterIssuanceInspectionV1<'_>,
    operation: &str,
) -> StoreResult<()> {
    inspection
        .evidence
        .require_original_grant(&mut inspection.grant)?;
    inspection.revision.ensure_origin(store, operation)?;
    inspection.revision.ensure_current(store, operation)?;
    let current = store.store_format()?;
    let lifecycle = format::validated_lifecycle(
        &store.db.snapshot(),
        &store.meta,
        &store.journal,
        &store.receipts,
        &current,
        &mut super::super::RosterStartupValidationBudgetV1::new(),
    )?;
    if lifecycle != Some(super::super::LIFECYCLE) {
        return Err(Error::EnrollmentRequired.into());
    }
    let view = store.snapshot_locked();
    inspection
        .grant
        .charge_bytes(2 * 512 + 64 * 1024)
        .map_err(super::super::source_error)?;
    inspection
        .grant
        .charge_steps(12)
        .map_err(super::super::source_error)?;
    let token = view.attachment_input_revision_bytes()?;
    let head = store
        .db
        .snapshot()
        .get(&store.meta, super::super::HEAD)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("review head missing"))?;
    if token != inspection.input_token
        || head.as_ref() != inspection.authority_head.as_slice()
        || view.store_identity()? != inspection.store
    {
        return Err(Error::InputMoved.into());
    }
    inspection
        .evidence
        .require_original_grant(&mut inspection.grant)
}

fn require_target(
    inspection: &RosterIssuanceInspectionV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
) -> StoreResult<()> {
    let binding = proposal.binding();
    if binding.store() != &inspection.store
        || binding.snapshot_revision().get() != inspection.evidence.snapshot_sequence()
        || binding.school() != &inspection.request.school
        || binding.program() != &inspection.request.program
    {
        return Err(Error::InputMoved.into());
    }
    Ok(())
}
