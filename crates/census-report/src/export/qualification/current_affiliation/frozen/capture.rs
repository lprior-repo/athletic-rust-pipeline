use super::*;
use census_domain::model::CanonicalAthlete;
use census_store::Table;

pub(super) fn capture(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    objects_dir: &Path,
) -> ReportResult<(FrozenAffiliationDescriptor, PinnedCurrentAffiliations)> {
    let profile = super::super::pinned_profile(dataset, snapshot)?;
    let profile_object = encode(objects_dir, &profile, PROFILE_BYTES)?;
    let captures = captures(snapshot, objects_dir)?;
    let mut reader = super::super::observation::Reader::with_captures(dataset, &profile, captures)?;
    let mut writer = physical_observations::Writer::new(
        dataset,
        snapshot,
        Table::Athletes,
        objects_dir,
        observation_budget(),
    )?;
    let mut failure = None;
    let scanned = snapshot.for_each_physical_observation_bounded(
        Table::Athletes,
        observation_budget(),
        |athlete: CanonicalAthlete, key, raw, _| {
            if let Err(error) = writer.push(key, raw) {
                failure = Some(error);
                return Err(invalid("frozen affiliation physical row writer refused"));
            }
            reader.accept(athlete)
        },
    );
    if let Some(error) = failure {
        return Err(error);
    }
    let observations = writer.finish(scanned?)?;
    let (rows, audit, retention) = reader.finish_with_retention();
    let archives = retention
        .ok_or_else(|| invalid("frozen affiliation archive capture was not retained"))?
        .finish()?;
    let descriptor = FrozenAffiliationDescriptor {
        version: 1,
        store_identity: dataset.lineage.store_identity.clone(),
        snapshot_sequence: snapshot.sequence(),
        input_generation: dataset.lineage.input_generation.clone(),
        input_digest: dataset.lineage.input_digest.clone(),
        source_digest: dataset.lineage.source_digest.clone(),
        profile_id: profile.id().map_err(StoreError::from)?,
        as_of: profile.input().as_of.clone(),
        profile: profile_object,
        observations,
        archives,
        roster_family: None,
    };
    descriptor.validate(dataset, descriptor.observations.fingerprint())?;
    let pinned = PinnedCurrentAffiliations {
        input_generation: descriptor.input_generation.clone(),
        profile_id: descriptor.profile_id.clone(),
        as_of: descriptor.as_of.clone(),
        rows,
        audit,
    };
    Ok((descriptor, pinned))
}

fn captures(
    snapshot: &StoreSnapshot<'_>,
    objects_dir: &Path,
) -> ReportResult<super::super::capture::Captures> {
    let archive =
        super::super::archive::Archive::capturing(snapshot.root(), assets::Sink::new(objects_dir))?;
    Ok(super::super::capture::Captures::with_archive(archive))
}
