use super::super::super::codec;
use super::*;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Admission {
    schema: String,
    store: StoreIdentity,
    inspected_sequence: u64,
    school: census_domain::model::SchoolId,
    program: census_domain::model::TeamId,
    pub(super) root: crate::RetainedCaptureRootV1,
    admission: String,
    original_http_authorization: String,
}

pub(super) fn objects(
    archive: &Archive,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<super::super::objects::Objects> {
    let mut objects = super::super::objects::Objects::new();
    for (key, raw) in &archive.records {
        let Some(digest) = key.strip_prefix(super::super::OBJECT_PREFIX.as_bytes()) else {
            continue;
        };
        grant
            .charge_bytes(raw.len())
            .map_err(super::super::super::source_error)?;
        grant
            .charge_steps(1)
            .map_err(super::super::super::source_error)?;
        let digest =
            std::str::from_utf8(digest).map_err(|_| Error::Corrupt("review object key"))?;
        objects.insert(digest.into(), raw.clone());
    }
    Ok(objects)
}

pub(super) fn admission(
    proposal: &LocallyCheckedRosterIssuanceV1,
    objects: &super::super::objects::Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Admission> {
    let raw = objects
        .get(
            proposal
                .capture()
                .retained_source_admission()
                .digest()
                .as_str(),
        )
        .ok_or(Error::Corrupt("review source admission absent"))?;
    grant
        .charge_bytes(raw.len())
        .map_err(super::super::super::source_error)?;
    grant
        .charge_steps(1)
        .map_err(super::super::super::source_error)?;
    let value: Admission =
        serde_json::from_slice(raw).map_err(|_| Error::Corrupt("review source admission"))?;
    if value.schema != "roster-retained-source-admission-v1"
        || value.admission != "completed-exact-provider-school-root"
        || value.original_http_authorization != "unknown"
        || &value.store != proposal.binding().store()
        || value.inspected_sequence != proposal.binding().snapshot_revision().get()
        || &value.school != proposal.binding().school()
        || &value.program != proposal.binding().program()
    {
        return Err(Error::Corrupt("review admission lineage").into());
    }
    Ok(value)
}
fn review_references(
    record: &ReviewRecord,
    objects: &super::super::objects::Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(super::super::ObjectRef, super::super::ObjectRef)> {
    record
        .basis
        .closure
        .binary_search_by(|reference| reference.digest.cmp(&record.basis.review.digest))
        .map_err(|_| Error::Corrupt("review source object absent"))?;
    let review_bytes = objects
        .get(&record.basis.review.digest)
        .ok_or(Error::Corrupt("review semantic object absent"))?;
    grant
        .charge_bytes(review_bytes.len())
        .map_err(super::super::super::source_error)?;
    grant
        .charge_steps(1)
        .map_err(super::super::super::source_error)?;
    let review: super::super::SemanticReview = codec::decode(review_bytes)?;
    Ok((review.qualification, review.activation))
}

pub(super) fn claims(
    archive: &Archive,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(
    Vec<ReviewedRosterSourceClaimV1>,
    Vec<super::super::ReviewedRosterMemberWithdrawalV1>,
)> {
    super::inherited::validate(archive, grant)?;
    validate_inventory(archive, grant)?;
    super::super::super::format::validate_enrollment(
        &archive.metadata,
        &archive.records,
        &archive.receipts,
        &archive.store,
    )?;
    let objects = Arc::new(objects(archive, grant)?);
    let source = frozen_source::FrozenRosterJournalSource::new(&archive.external, grant)?;
    let mut claims = Vec::new();
    let mut withdrawals = Vec::new();
    let mut roots = BTreeSet::new();
    for (key, raw) in &archive.records {
        if !key.starts_with(super::super::RECORD_PREFIX.as_bytes()) {
            continue;
        }
        grant
            .charge_bytes(raw.len())
            .map_err(super::super::super::source_error)?;
        grant
            .charge_steps(1)
            .map_err(super::super::super::source_error)?;
        let record: ReviewRecord = codec::decode(raw)?;
        if record.basis.inspected_sequence > archive.selected_snapshot_sequence {
            return Err(Error::Corrupt("review inspection follows publication").into());
        }
        if record.basis.schema == super::super::withdrawal::BASIS_SCHEMA {
            withdrawals
                .try_reserve(1)
                .map_err(|_| Error::BudgetExceeded)?;
            withdrawals.push(withdrawal(record, &objects, grant)?);
        } else {
            let (claim, root) = claim_for_record(archive, record, &objects, &source, grant)?;
            roots.insert(root);
            claims.try_reserve(1).map_err(|_| Error::BudgetExceeded)?;
            claims.push(claim);
        }
    }
    if roots.len() != archive.external.len() {
        return Err(Error::Corrupt("review external closure differs").into());
    }
    claims.sort_by_key(|claim| claim.record.basis.next_ordinal);
    withdrawals.sort_by_key(|withdrawal| withdrawal.ordinal());
    Ok((claims, withdrawals))
}

fn claim_for_record(
    archive: &Archive,
    record: ReviewRecord,
    objects: &Arc<super::super::objects::Objects>,
    source: &frozen_source::FrozenRosterJournalSource,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(ReviewedRosterSourceClaimV1, (&'static str, String))> {
    let proposal = super::super::proposal(&record.basis.proposal)?;
    let admitted = admission(&proposal, objects, grant)?;
    let root = (admitted.root.schema.phase(), admitted.root.key.clone());
    let held = source
        .read_root(admitted.root.schema, &admitted.root.key, grant)?
        .ok_or(Error::InspectedClosureChanged)?;
    if held.root() != &admitted.root {
        return Err(Error::InspectedClosureChanged.into());
    }
    let evidence = evidence(archive, &proposal, held, objects, grant)?;
    let (qualification, activation) = review_references(&record, objects, grant)?;
    for reference in &record.basis.closure {
        super::super::ledger::object(objects, reference)?;
        grant
            .charge_steps(1)
            .map_err(super::super::super::source_error)?;
    }
    Ok((
        ReviewedRosterSourceClaimV1 {
            proposal,
            evidence,
            record,
            objects: Arc::clone(objects),
            qualification,
            activation,
        },
        root,
    ))
}

fn evidence(
    archive: &Archive,
    proposal: &LocallyCheckedRosterIssuanceV1,
    held: crate::RetainedCaptureRootReadV1,
    objects: &super::super::objects::Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<RosterSourceEvidenceV1> {
    let continuation = grant
        .checkpoint()
        .map_err(super::super::super::source_error)?;
    let rows = RosterSourceEvidenceV1::retained_physical_projection(
        proposal.binding().physical_projection_inventory(),
        &archive.store,
        proposal.binding().snapshot_revision().get(),
        objects,
        grant,
    )?;
    let evidence = RosterSourceEvidenceV1::new(
        archive.store.clone(),
        proposal.binding().snapshot_revision().get(),
        proposal.binding().school().clone(),
        proposal.binding().program().clone(),
        held,
        rows,
        continuation,
    );
    let inventory = evidence.physical_projection_manifest(grant)?;
    grant
        .charge_bytes(inventory.len())
        .map_err(super::super::super::source_error)?;
    grant
        .charge_steps(1)
        .map_err(super::super::super::source_error)?;
    let expected = proposal.binding().physical_projection_inventory();
    if u64::try_from(inventory.len()).map_err(|_| Error::BudgetExceeded)? != expected.bytes().get()
        || codec::digest(&inventory) != expected.digest().as_str()
    {
        return Err(Error::InspectedClosureChanged.into());
    }
    Ok(evidence)
}

fn validate_inventory(archive: &Archive, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    let count = archive
        .metadata
        .len()
        .checked_add(archive.records.len())
        .and_then(|n| n.checked_add(archive.receipts.len()))
        .ok_or(Error::BudgetExceeded)?;
    if count > 4096 || archive.physical.len() > 4096 || archive.selected_snapshot_sequence == 0 {
        return Err(Error::BudgetExceeded.into());
    }
    for rows in [&archive.metadata, &archive.records, &archive.receipts] {
        let mut previous: Option<&[u8]> = None;
        for (key, _) in rows {
            grant
                .charge_steps(1)
                .map_err(super::super::super::source_error)?;
            if previous.is_some_and(|old| old >= key.as_slice()) {
                return Err(Error::Corrupt("review inventory physical order").into());
            }
            previous = Some(key);
        }
    }
    Ok(())
}

fn withdrawal(
    record: ReviewRecord,
    objects: &super::super::objects::Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<super::super::ReviewedRosterMemberWithdrawalV1> {
    let raw = super::super::ledger::object(objects, &record.basis.review)?;
    grant
        .charge_bytes(raw.len())
        .map_err(super::super::super::source_error)?;
    grant
        .charge_steps(1)
        .map_err(super::super::super::source_error)?;
    let review: super::super::SemanticReview = codec::decode(raw)?;
    let super::super::RosterSourceReviewDecisionV1::WithdrawExactSourceMember(decision) =
        review.fields.decision
    else {
        return Err(Error::Corrupt("withdrawal source decision absent").into());
    };
    Ok(super::super::ReviewedRosterMemberWithdrawalV1 {
        decision: *decision,
        operation: record.basis.operation,
        review: record.basis.review,
        committed_at: record.committed_at,
        ordinal: record.basis.next_ordinal,
    })
}
