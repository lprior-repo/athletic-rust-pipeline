use super::{aliases, invalid, PinnedCurrentAffiliations};
use crate::export::qualification::physical_observations;
use crate::export::{proof_objects, ExportDataset};
use crate::report::ReportResult;
use census_domain::model::{ProfileDate, ProfileId, QueryProfileV1};
use census_store::{StoreError, StoreSnapshot};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub(super) mod assets;
mod capture;
mod descriptor;
mod reviewed;

pub(crate) use descriptor::FrozenAffiliationDescriptor;
pub(super) use descriptor::ObjectRef;

const PROFILE_BYTES: u64 = 64 * 1024;
const INVENTORY_BYTES: u64 = 4 * 1024 * 1024;

pub(crate) fn capture(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    objects_dir: &Path,
) -> ReportResult<(FrozenAffiliationDescriptor, PinnedCurrentAffiliations)> {
    let profile = super::pinned_profile(dataset, snapshot)?;
    if census_domain::profile_execution::validate_selected_qualification_profile_v4(&profile)
        .is_ok()
    {
        return reviewed::capture(dataset, snapshot, objects_dir, &profile);
    }
    let _checked =
        census_domain::profile_execution::validate_selected_qualification_profile_v3(&profile)
            .map_err(|_| invalid("unsupported affiliation capture profile"))?;
    capture::capture(dataset, snapshot, objects_dir)
}

pub(crate) fn replay(
    dataset: &ExportDataset,
    objects_dir: &Path,
    descriptor: &FrozenAffiliationDescriptor,
    expected_athlete_table_digest: &str,
    expected_descriptor_digest: &str,
) -> ReportResult<PinnedCurrentAffiliations> {
    dataset.ensure_input_binding()?;
    descriptor.validate(dataset, expected_athlete_table_digest)?;
    descriptor.require_digest(expected_descriptor_digest)?;
    let profile: QueryProfileV1 = decode(objects_dir, &descriptor.profile, PROFILE_BYTES)?;
    super::validate_profile(dataset, &profile)?;
    aliases(dataset)?;
    if descriptor.reviewed_roster() {
        return reviewed::replay(dataset, objects_dir, descriptor, &profile);
    }
    let source = assets::Source::open(objects_dir, &descriptor.archives)?;
    let archive = super::archive::Archive::frozen(source);
    let captures = super::capture::Captures::with_archive(archive);
    let mut reader = super::observation::Reader::with_captures(dataset, &profile, captures)?;
    physical_observations::visit(
        dataset,
        objects_dir,
        &descriptor.observations,
        expected_athlete_table_digest,
        observation_budget(),
        |row| reader.accept(row),
    )?;
    let (rows, audit) = reader.finish();
    Ok(PinnedCurrentAffiliations {
        input_generation: descriptor.input_generation.clone(),
        profile_id: descriptor.profile_id.clone(),
        as_of: descriptor.as_of.clone(),
        rows,
        audit,
    })
}

#[cfg(test)]
pub(in crate::export::qualification) fn visit_athletes(
    dataset: &ExportDataset,
    objects_dir: &Path,
    descriptor: &FrozenAffiliationDescriptor,
    expected_athlete_table_digest: &str,
    expected_descriptor_digest: &str,
    visit: impl FnMut(census_domain::model::CanonicalAthlete) -> census_store::StoreResult<()>,
) -> ReportResult<()> {
    dataset.ensure_input_binding()?;
    descriptor.validate(dataset, expected_athlete_table_digest)?;
    descriptor.require_digest(expected_descriptor_digest)?;
    let profile: QueryProfileV1 = decode(objects_dir, &descriptor.profile, PROFILE_BYTES)?;
    super::validate_profile(dataset, &profile)?;
    aliases(dataset)?;
    physical_observations::visit(
        dataset,
        objects_dir,
        &descriptor.observations,
        expected_athlete_table_digest,
        observation_budget(),
        visit,
    )
    .map(|_| ())
}

fn observation_budget() -> census_store::ReadBudget {
    census_store::ReadBudget {
        rows: super::MAX_OBSERVATIONS,
        bytes: super::MAX_OBSERVATION_BYTES,
        record_bytes: 1024 * 1024,
    }
}

fn encode(objects_dir: &Path, value: &impl Serialize, ceiling: u64) -> ReportResult<ObjectRef> {
    let bytes = census_domain::model::serialized_bytes(value).map_err(StoreError::from)?;
    let length = u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)?;
    if length > ceiling {
        return Err(StoreError::BudgetExceeded {
            what: "frozen affiliation object bytes",
            needed: length,
            available: ceiling,
        }
        .into());
    }
    let digest = proof_objects::install(objects_dir, &bytes)?;
    Ok(ObjectRef {
        digest,
        bytes: length,
    })
}

fn decode<T: serde::de::DeserializeOwned>(
    objects_dir: &Path,
    reference: &ObjectRef,
    ceiling: u64,
) -> ReportResult<T> {
    let bytes = read(objects_dir, reference, ceiling)?;
    serde_json::from_slice(&bytes).map_err(|source| {
        StoreError::Decode {
            key: reference.digest.clone(),
            source,
        }
        .into()
    })
}

fn read(objects_dir: &Path, reference: &ObjectRef, ceiling: u64) -> ReportResult<Vec<u8>> {
    reference.validate(ceiling)?;
    let bytes = proof_objects::read(objects_dir, &reference.digest, ceiling)?;
    if u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)? != reference.bytes {
        return Err(invalid("frozen affiliation object differs from its exact byte count").into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
