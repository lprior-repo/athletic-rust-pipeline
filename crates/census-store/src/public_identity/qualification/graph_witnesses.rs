use crate::public_identity::{
    graph_witness::{HeldGraphWitness, SourceReplayCache},
    retained::{GraphRead, Read, Rows},
    witness, PublicIdentityError,
};
use crate::{StoreError, StoreResult};
use census_domain::public_identity_history::{
    genesis, prepare_next, replay_candidate_history, DecodedCandidateHistory, HistoryRecord,
};
use census_domain::public_identity_qualification::{
    PublicIdentityQualification, PublicIdentityQualificationVersion,
};
use std::collections::BTreeSet;

pub(in crate::public_identity) fn accepted_decisions(
    reader: &impl Read,
    records: &[HistoryRecord],
) -> StoreResult<usize> {
    records.iter().try_fold(0usize, |count, record| {
        let held = load(reader, record.sequence)?;
        count
            .checked_add(usize::from(held.decision_id().is_some()))
            .ok_or(StoreError::CounterOverflow)
    })
}

pub(in crate::public_identity) fn validate_graph_retained(
    reader: &impl GraphRead,
    history: &DecodedCandidateHistory,
    declaration: &PublicIdentityQualification,
) -> StoreResult<()> {
    if declaration.version() != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3 {
        return Err(PublicIdentityError::Corrupt("graph witness qualification family").into());
    }
    let identity = declaration.store_identity();
    let initial = genesis(identity).map_err(PublicIdentityError::from)?;
    let prefix = replay_candidate_history(
        identity,
        &initial,
        &[],
        crate::public_identity::REPLAY_LIMITS,
    )
    .map_err(PublicIdentityError::from)?;
    let mut sources = SourceReplayCache::default();
    let (prefix, _) = history.records().iter().try_fold(
        (prefix, BTreeSet::new()),
        |(prefix, mut superseded), record| -> StoreResult<_> {
            let prepared = prepare_next(
                identity,
                prefix.head(),
                record.request.clone(),
                crate::public_identity::REPLAY_LIMITS,
            )
            .map_err(PublicIdentityError::from)?;
            if &prepared.uncommitted_record != record {
                return Err(
                    PublicIdentityError::Corrupt("graph historical operation position").into(),
                );
            }
            let held = load(reader, record.sequence)?;
            held.require_available(reader, identity, &superseded)?;
            held.verify(reader, &prefix, &prepared, history.records(), &mut sources)?;
            held.retain_superseded_material(reader, &mut superseded)?;
            let next = prefix
                .extend_candidate(record)
                .map_err(PublicIdentityError::from)?;
            Ok((next, superseded))
        },
    )?;
    if prefix.head() != history.projection().head() {
        return Err(PublicIdentityError::Corrupt("graph historical ledger head").into());
    }
    Ok(())
}

fn load(reader: &impl Read, sequence: u64) -> StoreResult<HeldGraphWitness> {
    let key = witness::key(sequence);
    let bytes = reader
        .row(Rows::Witness, &key)?
        .ok_or(PublicIdentityError::Corrupt("missing graph commit witness"))?;
    HeldGraphWitness::decode(&key, &bytes)
}
