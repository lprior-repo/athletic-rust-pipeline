use super::*;
use crate::export::qualification::physical_observations::OBSERVATION_BUDGET;

pub(super) fn live(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    profile: &QueryProfileV1,
) -> ReportResult<()> {
    selected(dataset, profile)?;
    current_policy(&profile.input().policies.participation)?;
    if snapshot.profile(&profile.id().map_err(StoreError::from)?)? != *profile
        || snapshot.store_identity()?.as_str() != dataset.lineage.store_identity
        || snapshot.sequence() != dataset.lineage.snapshot_sequence
    {
        return Err(
            invalid("participation requires exact registered profile and pinned dataset").into(),
        );
    }
    dataset.ensure_snapshot(snapshot)
}

pub(super) fn frozen(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    replay: &ParticipationReplay,
) -> ReportResult<()> {
    selected(dataset, profile)?;
    let current = profile.input().policies.participation
        == participation_policy_v4().map_err(|_| invalid("V4 participation policy unavailable"))?;
    match replay {
        ParticipationReplay::V1(_) if current => {
            return Err(invalid("V4 participation requires ReplayV2").into())
        }
        ParticipationReplay::V2(value) => {
            if !current || value.version != 2 {
                return Err(invalid("ReplayV2 requires exact V4 participation").into());
            }
            value.independent.require_binding(
                &dataset.lineage.store_identity,
                dataset.lineage.snapshot_sequence,
            )?;
        }
        ParticipationReplay::V3(value) => {
            if !current || value.version != 3 || value.retained.version != 2 {
                return Err(invalid("ReplayV3 requires exact V4 participation").into());
            }
            value.retained.independent.require_binding(
                &dataset.lineage.store_identity,
                dataset.lineage.snapshot_sequence,
            )?;
        }
        ParticipationReplay::V4(value) => {
            if !current || value.version != 4 || value.retained.version != 2 {
                return Err(invalid("ReplayV4 requires exact V4 participation").into());
            }
            value.retained.independent.require_binding(
                &dataset.lineage.store_identity,
                dataset.lineage.snapshot_sequence,
            )?;
        }
        ParticipationReplay::V1(_) => {}
    }
    let replay = replay.retained();
    if replay.version != 1
        || replay.profile != *profile
        || replay.profile_id != profile.id().map_err(StoreError::from)?
        || replay.store_identity != dataset.lineage.store_identity
        || replay.snapshot_sequence != dataset.lineage.snapshot_sequence
        || replay.input_generation != dataset.lineage.input_generation
        || replay.input_digest != dataset.lineage.input_digest
        || replay.source_digest != dataset.lineage.source_digest
    {
        return Err(
            invalid("participation replay differs from selected generation or profile").into(),
        );
    }
    Ok(())
}

fn selected(dataset: &ExportDataset, profile: &QueryProfileV1) -> ReportResult<()> {
    dataset.ensure_input_binding()?;
    replay_policy(&profile.input().policies.participation)?;
    let profile_id = profile.id().map_err(StoreError::from)?;
    let performances =
        u64::try_from(dataset.performances.len()).map_err(|_| StoreError::CounterOverflow)?;
    let athletes =
        u64::try_from(dataset.athletes.len()).map_err(|_| StoreError::CounterOverflow)?;
    if performances > OBSERVATION_BUDGET.rows
        || athletes > OBSERVATION_BUDGET.rows
        || dataset.selected_profile_id() != Some(&profile_id)
        || dataset.selected_as_of() != Some(&profile.input().as_of)
        || dataset.selected_participation_rule() != Some(&profile.input().policies.participation)
        || serialized_digest(dataset).map_err(StoreError::from)? != dataset.lineage.input_digest
    {
        return Err(invalid("participation requires the exact selected profile binding").into());
    }
    super::super::current_affiliation::aliases(dataset).map_err(Into::into)
}

fn current_policy(rule: &census_domain::model::ContractRefV1) -> ReportResult<()> {
    if rule
        != &participation_policy_v4().map_err(|_| invalid("V4 participation policy unavailable"))?
    {
        return Err(invalid("new participation capture requires the exact V4 policy").into());
    }
    Ok(())
}

fn replay_policy(rule: &census_domain::model::ContractRefV1) -> ReportResult<()> {
    let v2 =
        participation_policy_v2().map_err(|_| invalid("V2 participation policy unavailable"))?;
    let v3 =
        participation_policy_v3().map_err(|_| invalid("V3 participation policy unavailable"))?;
    let v4 =
        participation_policy_v4().map_err(|_| invalid("V4 participation policy unavailable"))?;
    if rule != &v2 && rule != &v3 && rule != &v4 {
        return Err(invalid("participation replay policy is not registered").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::ContractRefV1;
    use census_domain::participation_candidates::{
        PARTICIPATION_POLICY_V2_SHA256, PARTICIPATION_POLICY_V3_SHA256,
    };

    #[test]
    fn current_capture_requires_v4_and_historical_replay_preserves_exact_v2_v3(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let v2 = participation_policy_v2()?;
        let v3 = participation_policy_v3()?;
        let v4 = participation_policy_v4()?;
        assert!(current_policy(&v2).is_err());
        assert!(current_policy(&v3).is_err());
        assert!(current_policy(&v4).is_ok());
        assert!(replay_policy(&v4).is_ok());
        assert!(replay_policy(&v2).is_ok());
        assert!(replay_policy(&v3).is_ok());
        for (revision, digest) in [
            (3, PARTICIPATION_POLICY_V2_SHA256),
            (2, PARTICIPATION_POLICY_V3_SHA256),
            (4, PARTICIPATION_POLICY_V3_SHA256),
        ] {
            let unknown = ContractRefV1::new(1, revision, digest)?;
            assert!(current_policy(&unknown).is_err());
            assert!(replay_policy(&unknown).is_err());
        }
        Ok(())
    }
}
