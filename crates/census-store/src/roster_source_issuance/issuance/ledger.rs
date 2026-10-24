use super::super::{codec, enrollment::AuthorityHead, RosterSourceIssuanceStoreError as Error};
use super::{ObjectRef, ReviewRecord, RosterSourceReviewDecisionV1};
use crate::{Receipt, StoreResult};
use census_domain::model::StoreIdentity;
use std::collections::{BTreeMap, BTreeSet};

type Rows = [(Vec<u8>, Vec<u8>)];

struct LedgerState<'a> {
    objects: &'a super::objects::Objects,
    identity: &'a StoreIdentity,
    rows: &'a Rows,
    receipts: &'a Rows,
    used_objects: BTreeSet<String>,
    used_operations: BTreeSet<String>,
}

pub(in super::super) fn validate(
    rows: &Rows,
    receipts: &Rows,
    identity: &StoreIdentity,
    enrollment_digest: &str,
    expected: &AuthorityHead,
) -> StoreResult<()> {
    let mut objects = BTreeMap::new();
    let mut records = Vec::new();
    for (key, raw) in rows {
        let key = std::str::from_utf8(key).map_err(|_| Error::Corrupt("review key encoding"))?;
        if let Some(digest) = key.strip_prefix(super::OBJECT_PREFIX) {
            if digest != codec::digest(raw) {
                return Err(Error::Corrupt("review object digest").into());
            }
            objects.insert(digest.to_owned(), raw.clone());
        } else if let Some(operation_digest) = key.strip_prefix(super::RECORD_PREFIX) {
            let record: ReviewRecord = codec::decode_review_composite(raw)?;
            if operation_digest != codec::digest(record.basis.operation.as_str().as_bytes()) {
                return Err(Error::Corrupt("review operation key").into());
            }
            records.push((record.basis.next_ordinal, raw));
        } else {
            return Err(Error::Corrupt("unknown review namespace material").into());
        }
    }
    records.sort_by_key(|(ordinal, _)| *ordinal);
    let mut head = AuthorityHead {
        version: 1,
        ordinal: 0,
        digest: enrollment_digest.into(),
    };
    let mut state = LedgerState {
        objects: &objects,
        identity,
        rows,
        receipts,
        used_objects: BTreeSet::new(),
        used_operations: BTreeSet::new(),
    };
    for (ordinal, raw) in records {
        check_chained(&mut head, ordinal, raw, &mut state)?;
    }
    if expected.version != 1
        || expected.ordinal != head.ordinal
        || expected.digest != head.digest
        || state.used_objects.len() != objects.len()
        || receipts.len() != state.used_operations.len().saturating_add(1)
    {
        return Err(Error::Corrupt("review complete inventory mismatch").into());
    }
    Ok(())
}

fn check_chained(
    head: &mut AuthorityHead,
    ordinal: u64,
    raw: &[u8],
    state: &mut LedgerState<'_>,
) -> StoreResult<()> {
    let record: ReviewRecord = codec::decode_review_composite(raw)?;
    head.ordinal = head.ordinal.checked_add(1).ok_or(Error::BudgetExceeded)?;
    if ordinal != head.ordinal || record.basis.next_ordinal != head.ordinal {
        return Err(Error::Corrupt("review ordinal chain").into());
    }
    let prior = AuthorityHead {
        version: 1,
        ordinal: head.ordinal.checked_sub(1).ok_or(Error::BudgetExceeded)?,
        digest: head.digest.clone(),
    };
    validate_record(&record, state.objects, state.identity, &prior, state.rows)?;
    for reference in &record.basis.closure {
        state.used_objects.insert(reference.digest.clone());
    }
    let operation = format!(
        "{}{}",
        super::super::OPERATION_PREFIX,
        record.basis.operation.as_str()
    );
    if !state.used_operations.insert(operation.clone()) {
        return Err(Error::OperationConflict.into());
    }
    let receipt: Receipt = codec::decode(
        state
            .receipts
            .iter()
            .find(|(key, _)| key == operation.as_bytes())
            .map(|(_, raw)| raw.as_slice())
            .ok_or(Error::Corrupt("review receipt missing"))?,
    )?;
    head.digest = codec::digest(raw);
    if receipt.operation != operation
        || receipt.digest != head.digest
        || receipt.at != record.committed_at.to_rfc3339()
        || receipt.appended != 0
        || receipt.credited
    {
        return Err(Error::Corrupt("review receipt mismatch").into());
    }
    Ok(())
}

pub(super) fn validate_record(
    record: &ReviewRecord,
    objects: &super::objects::Objects,
    identity: &StoreIdentity,
    prior: &AuthorityHead,
    source_records: &Rows,
) -> StoreResult<()> {
    match record.basis.schema.as_str() {
        "roster-source-reviewed-claims-basis-v1" => {
            validate_issued_record(record, objects, identity, prior)
        }
        super::withdrawal::BASIS_SCHEMA => {
            validate_header(record, objects, identity, prior)?;
            super::withdrawal::validate::record(record, objects, identity, prior, source_records)
        }
        _ => Err(Error::Corrupt("unsupported roster review basis").into()),
    }
}

pub(super) fn validate_issued_record(
    record: &ReviewRecord,
    objects: &super::objects::Objects,
    identity: &StoreIdentity,
    prior: &AuthorityHead,
) -> StoreResult<()> {
    let basis = &record.basis;
    if basis.schema != "roster-source-reviewed-claims-basis-v1" {
        return Err(Error::Corrupt("unsupported issuance basis").into());
    }
    validate_header(record, objects, identity, prior)?;
    require_own_closure(record)?;
    let proposal = super::proposal(&basis.proposal)?;
    super::objects::require_proposal(objects, &proposal)?;
    if proposal.binding().store() != identity
        || proposal.binding().snapshot_revision().get() != basis.inspected_sequence
        || basis.physical_inventory.digest
            != proposal
                .binding()
                .physical_projection_inventory()
                .digest()
                .as_str()
        || u64::try_from(basis.physical_inventory.bytes).map_err(|_| Error::BudgetExceeded)?
            != proposal
                .binding()
                .physical_projection_inventory()
                .bytes()
                .get()
        || object(objects, &basis.proposal_review_basis)?
            != super::review::proposal_basis_for_issuance(&proposal)?
    {
        return Err(Error::Corrupt("review proposal lineage").into());
    }
    let review: super::SemanticReview = codec::decode(object(objects, &basis.review)?)?;
    validate_semantic(record, &proposal, &review, objects)
}

fn validate_header(
    record: &ReviewRecord,
    objects: &super::objects::Objects,
    identity: &StoreIdentity,
    prior: &AuthorityHead,
) -> StoreResult<()> {
    let basis = &record.basis;
    if basis.closure.len() > census_domain::roster_affiliation_authority::MAX_ROSTER_CLOSURE_OBJECTS
    {
        return Err(Error::BudgetExceeded.into());
    }
    if &basis.store != identity
        || record.approval != super::review::approval(basis)?
        || record.approval_digest != codec::digest(&record.approval)
        || object(objects, &basis.prior_head)? != codec::encode(prior)?
    {
        return Err(Error::Corrupt("review basis or approval mismatch").into());
    }
    let mut previous: Option<&str> = None;
    let mut bytes = 0_usize;
    for reference in &basis.closure {
        if previous.is_some_and(|value| value >= reference.digest.as_str()) {
            return Err(Error::Corrupt("review closure order").into());
        }
        previous = Some(&reference.digest);
        let raw = object(objects, reference)?;
        let cap = if reference.digest == basis.physical_inventory.digest {
            census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES
        } else {
            4 * 1024 * 1024
        };
        if raw.len() > cap {
            return Err(Error::BudgetExceeded.into());
        }
        bytes = bytes.checked_add(raw.len()).ok_or(Error::BudgetExceeded)?;
        if bytes > census_domain::roster_affiliation_authority::MAX_ROSTER_CLOSURE_BYTES {
            return Err(Error::BudgetExceeded.into());
        }
    }
    Ok(())
}

fn validate_semantic(
    record: &ReviewRecord,
    proposal: &census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1,
    review: &super::SemanticReview,
    objects: &super::objects::Objects,
) -> StoreResult<()> {
    let basis = &record.basis;
    if review.schema != "roster-source-semantic-review-v1"
        || review.provenance != "explicit-unauthenticated-local-operator"
        || review.decision != "qualify-and-activate-exact-capture-representation-claims"
        || !matches!(
            &review.fields.decision,
            RosterSourceReviewDecisionV1::QualifyAndActivateExactCaptureRepresentation
        )
        || review.store != record.basis.store
        || review.fields.operation != record.basis.operation
        || review.fields.reviewed_at > record.committed_at
        || review.input_token != record.basis.input_token
        || review.prior_head != record.basis.prior_head
        || review.proposal_basis != record.basis.proposal_review_basis
        || &review.scope_policy != proposal.scope().policy()
        || &review.year_convention != proposal.binding().year_convention()
        || &review.projection_merge_contract != proposal.binding().projection_merge_contract()
        || record.basis.review.digest != proposal.semantic_review_evidence().digest().as_str()
    {
        return Err(Error::Corrupt("retained semantic review mismatch").into());
    }
    for reference in [&review.qualification, &review.activation] {
        if !record.basis.closure.iter().any(|held| held == reference) {
            return Err(Error::Corrupt("review activation outside closure").into());
        }
    }
    super::qualification::require(
        proposal,
        &review.fields.decision,
        &review.proposal_basis,
        &review.qualification,
        &review.activation,
        objects,
    )?;
    for reference in [
        &review.scope_policy,
        &review.year_convention,
        &review.projection_merge_contract,
    ] {
        if reference.revision().get() != 1 || !objects.contains_key(reference.digest()) {
            return Err(Error::Corrupt("retained semantic contract missing").into());
        }
    }
    let acquisition = chrono::DateTime::parse_from_rfc3339(proposal.capture().acquired_at())
        .map_err(|_| Error::Corrupt("review acquisition"))?;
    if review.fields.reviewed_at < acquisition {
        return Err(Error::Corrupt("review chronology").into());
    }
    object(objects, &basis.input_token)?;
    object(objects, &basis.physical_inventory)?;
    Ok(())
}

pub(super) fn object<'a>(
    objects: &'a super::objects::Objects,
    reference: &ObjectRef,
) -> StoreResult<&'a [u8]> {
    let raw = objects
        .get(&reference.digest)
        .ok_or(Error::Corrupt("review closure object absent"))?;
    if raw.len() != reference.bytes {
        return Err(Error::Corrupt("review closure object length").into());
    }
    Ok(raw)
}

fn require_own_closure(record: &ReviewRecord) -> StoreResult<()> {
    let proposal = super::proposal(&record.basis.proposal)?;
    let parser = proposal.parser();
    let capture = proposal.capture();
    let binding = proposal.binding();
    for digest in [
        parser.artifact().digest().as_str(),
        parser.schema().digest().as_str(),
        parser.configuration().digest().as_str(),
        parser.dependencies().digest().as_str(),
        capture.body().digest().as_str(),
        capture.manifest().digest().as_str(),
        capture.interpretation().digest().as_str(),
        capture.retained_source_admission().digest().as_str(),
        capture.verification_grant().digest().as_str(),
        binding.school_projection().digest().as_str(),
        binding.program_projection().digest().as_str(),
        binding.provider_packet().digest().as_str(),
        binding.physical_projection_inventory().digest().as_str(),
        proposal.semantic_review_evidence().digest().as_str(),
        &record.basis.prior_head.digest,
        &record.basis.input_token.digest,
        &record.basis.proposal_review_basis.digest,
        proposal.scope().policy().digest(),
        binding.year_convention().digest(),
        binding.projection_merge_contract().digest(),
    ] {
        if !record
            .basis
            .closure
            .iter()
            .any(|reference| reference.digest == digest)
        {
            return Err(Error::Corrupt("review reference outside own closure").into());
        }
    }
    Ok(())
}
