use census_domain::public_identity_history::ReplayLimits;

pub(crate) const REPLAY_LIMITS: ReplayLimits = ReplayLimits {
    records: 2_097_152,
    identities: 1_048_576,
    bindings: 2_097_152,
    total_text_bytes: 256 * 1024 * 1024,
    traversal_steps: 2_097_152,
    graph_steps: 67_108_864,
    ..ReplayLimits::SMALL
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public_identity_tests::support::{subject, TestResult};
    use census_domain::model::{
        AcceptedAthleteIdentity, AthleteIdentityIndex, CanonicalAthlete, IdentityApplication,
        IdentityProjectionBuilder, SourceIdentity, SourceNamespace, StoreIdentity,
    };
    use census_domain::public_identity_history::{
        genesis, prepare_next, replay_candidate_history, CandidateDecisionReference, HistoryError,
        HistoryRequest, PublicResolution,
    };

    fn accepted(athlete: &CanonicalAthlete) -> TestResult<AcceptedAthleteIdentity> {
        let mut index = AthleteIdentityIndex::default();
        index.observe(athlete)?;
        let builder = IdentityProjectionBuilder::new(index, &[], &[])?;
        let application = builder
            .source_applications("2026-10-14")
            .next()
            .ok_or("fixture lacks source application")??;
        match application {
            IdentityApplication::Accepted(decision) => Ok(decision),
            IdentityApplication::Retained(issue) => {
                Err(format!("fixture retained: {issue:?}").into())
            }
        }
    }

    #[test]
    fn store_decoder_replays_one_more_identity_than_the_small_contract() -> TestResult {
        let identity = StoreIdentity::parse(&"a".repeat(64))?;
        let initial = genesis(&identity)?;
        let mut projection = replay_candidate_history(&identity, &initial, &[], REPLAY_LIMITS)?;
        let count = ReplayLimits::SMALL
            .records
            .checked_add(1)
            .ok_or("count overflow")?;
        let mut records = Vec::new();
        records.try_reserve_exact(count)?;
        let mut last_candidate = None;
        for index in 0..count {
            let label = index
                .checked_add(1)
                .ok_or("fixture index overflow")?
                .to_string();
            let athlete = subject(&label, &label)?;
            let candidate = athlete.id.cast();
            let decision = accepted(&athlete)?;
            let evidence = CandidateDecisionReference::from_accepted(&decision, REPLAY_LIMITS)?;
            let binding = crate::public_identity::commit::source_binding(
                &candidate,
                SourceIdentity::new(SourceNamespace::MilesplitAthlete, &label),
                &format!("{index:064x}"),
            );
            let prepared = prepare_next(
                &identity,
                projection.head(),
                HistoryRequest::Establish { binding, evidence },
                REPLAY_LIMITS,
            )?;
            projection = projection.extend_candidate(&prepared.uncommitted_record)?;
            records.push(prepared.uncommitted_record);
            last_candidate = Some(candidate);
        }
        let raw = serde_json::to_vec(&records)?;
        let decoded = crate::public_identity::codec::history(&raw, &identity, projection.head())?;
        check!(eq; decoded.records().len(), count);
        check!(eq; decoded.projection().head(), projection.head());
        let last_candidate = last_candidate.ok_or("last candidate missing")?;
        check!(
            matches!(decoded.projection().resolve_candidate(&last_candidate)?,
            PublicResolution::Canonical(row) if row.representative_candidate == last_candidate)
        );
        check!(eq; replay_candidate_history(&identity, projection.head(), &records, ReplayLimits::SMALL).err(), Some(HistoryError::Limit("records")));
        check!(eq; REPLAY_LIMITS.members_per_operation, ReplayLimits::SMALL.members_per_operation);
        check!(eq; REPLAY_LIMITS.text_bytes, ReplayLimits::SMALL.text_bytes);
        check!(eq; REPLAY_LIMITS.affected_record_keys, ReplayLimits::SMALL.affected_record_keys);
        Ok(())
    }
}
