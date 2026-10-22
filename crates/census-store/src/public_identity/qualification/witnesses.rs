use crate::public_identity::{
    operation::HeldWitness,
    retained::{HistoricalRead as Read, Live, Revision, Rows},
    witness, PublicIdentityError,
};
use crate::StoreResult;
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::{
    genesis, prepare_next, replay_candidate_history, DecodedCandidateHistory, HistoryProjection,
    HistoryRecord, HistoryRequest, PreparedHistoryRecord, PublicResolution,
};
use census_domain::public_identity_qualification::{
    PublicIdentityQualification, PublicIdentityQualificationVersion,
};
use fjall::{Keyspace, Snapshot};

pub(super) fn validate(
    snapshot: &Snapshot,
    entities: &Keyspace,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    history: &DecodedCandidateHistory,
    declaration: &PublicIdentityQualification,
) -> StoreResult<()> {
    witness::verify_count(snapshot, journal, history.records().len())?;
    witness::verify_count(snapshot, receipts, history.records().len())?;
    let reader = Live::new(snapshot, entities, meta, journal, receipts)?;
    validate_retained(&reader, history, declaration)
}

pub(in crate::public_identity) fn validate_retained(
    reader: &impl Read,
    history: &DecodedCandidateHistory,
    declaration: &PublicIdentityQualification,
) -> StoreResult<()> {
    let identity = declaration.store_identity();
    let initial = genesis(identity).map_err(PublicIdentityError::from)?;
    let mut prefix = replay_candidate_history(
        identity,
        &initial,
        &[],
        crate::public_identity::REPLAY_LIMITS,
    )
    .map_err(PublicIdentityError::from)?;
    let mut superseded_material = std::collections::BTreeSet::new();
    for record in history.records() {
        let prepared = prepare_next(
            identity,
            prefix.head(),
            record.request.clone(),
            crate::public_identity::REPLAY_LIMITS,
        )
        .map_err(PublicIdentityError::from)?;
        if &prepared.uncommitted_record != record {
            return Err(PublicIdentityError::Corrupt("historical operation position").into());
        }
        let previous = prefix.clone();
        prefix = prefix
            .extend_candidate(record)
            .map_err(PublicIdentityError::from)?;
        let key = witness::key(record.sequence);
        let raw = reader
            .row(Rows::Witness, &key)?
            .ok_or(PublicIdentityError::Corrupt("missing commit witness"))?;
        let held = HeldWitness::decode(&key, &raw)?;
        held.require_available(reader, identity, &superseded_material)?;
        held.verify(
            reader,
            declaration.version(),
            &prefix,
            &prepared,
            &previous,
            history.records(),
        )?;
        held.retain_superseded_material(reader, &mut superseded_material)?;
    }
    if prefix.head() != history.projection().head() {
        return Err(PublicIdentityError::Corrupt("historical ledger head").into());
    }
    Ok(())
}

impl HeldWitness {
    fn retain_superseded_material(
        &self,
        reader: &impl Read,
        superseded_material: &mut std::collections::BTreeSet<String>,
    ) -> StoreResult<()> {
        if let Self::MembershipWithheld(withheld) = self {
            superseded_material.insert(
                crate::public_identity::membership_withholding::fence::material(
                    reader,
                    &withheld.adjudication,
                )?,
            );
        }
        if let Self::Manual(manual) = self {
            superseded_material.insert(
                crate::public_identity::local_adjudication::fence::manual_material(
                    reader,
                    &manual.adjudication,
                )?,
            );
        }
        Ok(())
    }

    fn require_available(
        &self,
        reader: &impl Read,
        identity: &StoreIdentity,
        superseded_material: &std::collections::BTreeSet<String>,
    ) -> StoreResult<()> {
        if let Self::Membership(held) = self {
            let inputs = reader.context(&held.authority_context, identity)?;
            let case = inputs
                .cases
                .iter()
                .find(|case| case.id == held.request.case_id)
                .ok_or(PublicIdentityError::NotReviewed)?;
            let material =
                crate::public_identity::authority::identity_material(&inputs, &case.member_ids)?;
            if superseded_material.contains(&material) {
                return Err(PublicIdentityError::Corrupt(
                    "membership authority superseded by local adjudication",
                )
                .into());
            }
        }
        if let Self::Reviewed(reviewed) = self {
            let material = crate::public_identity::local_adjudication::fence::reviewed_material(
                reader, identity, reviewed,
            )?;
            if superseded_material.contains(&material) {
                return Err(PublicIdentityError::Corrupt(
                    "reviewed authority superseded by local adjudication",
                )
                .into());
            }
        }
        Ok(())
    }

    fn verify(
        &self,
        reader: &impl Read,
        version: PublicIdentityQualificationVersion,
        prefix: &HistoryProjection,
        prepared: &PreparedHistoryRecord,
        previous: &HistoryProjection,
        records: &[HistoryRecord],
    ) -> StoreResult<()> {
        let record = &prepared.uncommitted_record;
        match self {
            Self::Source(held) => {
                source(reader, &record.installation, version, held, record, prefix)
            }
            Self::MembershipWithheld(held) => {
                held.verify(reader, version, previous, prepared, records)
            }
            Self::Membership(held) => held.verify(reader, version, previous, prepared),
            Self::Manual(held) => held.verify(reader, version, previous, prepared, records),
            Self::Reviewed(held) => {
                held.validate(version, prepared)?;
                reader.revision()?.includes(Revision {
                    evidence_generation: held.evidence_generation,
                    derived_generation: held.derived_generation,
                    snapshot_sequence: held.snapshot_sequence,
                })?;
                held.verify_rows(reader, &record.installation, record)
            }
        }
    }
}

fn source(
    reader: &impl Read,
    identity: &StoreIdentity,
    version: PublicIdentityQualificationVersion,
    held: &witness::Witness,
    record: &HistoryRecord,
    prefix: &HistoryProjection,
) -> StoreResult<()> {
    let HistoryRequest::Establish { binding, evidence } = &record.request else {
        return Err(PublicIdentityError::Corrupt("unsupported acceptance operation").into());
    };
    let PublicResolution::Canonical(resolved) = prefix
        .resolve_candidate(&binding.candidate)
        .map_err(PublicIdentityError::from)?
    else {
        return Err(PublicIdentityError::Corrupt("unsupported public identity resolution").into());
    };
    held.require_qualification_version(version)?;
    held.validate(record, &resolved.public_id)?;
    reader.revision()?.includes(Revision {
        evidence_generation: held.evidence_generation,
        derived_generation: held.derived_generation,
        snapshot_sequence: held.snapshot_sequence,
    })?;
    witness::verify_rows(reader, identity, held, evidence, binding)
}
