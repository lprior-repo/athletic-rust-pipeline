use super::{CurrentAffiliationResolution, VerifiedCurrentAffiliations};
use crate::export::qualification::physical_observations::OBSERVATION_BUDGET;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::model::{AthleteId, ProfileDate, ProfileId};
#[cfg(test)]
use census_store::Table;
use census_store::{StoreError, StoreResult, StoreSnapshot};
use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;

mod archive;
mod audit;
mod capture;
mod context;
pub(crate) use context::VerifiedRosterContext;
mod frozen;
pub(crate) use frozen::FrozenAffiliationDescriptor;
pub(crate) use frozen::{capture as capture_frozen, replay as replay_frozen};

mod note;
mod observation;
mod source_authority;
mod source_binding;

#[cfg(test)]
mod tests;
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum UnavailableAffiliation {
    NoRetainedRow,
    IdentityUnaccepted,
    OutsideCaptureDay,
    ArchiveUnavailable,
    SchoolUnresolved,
    OutsideSelectedSport,
    MissingTemporalAuthority,
    TemporalStatementAmbiguous,
    TemporalMarkupMalformed,
    TemporalResourceLimited,
    TemporalMemberInactive,
}

pub(crate) struct PinnedCurrentAffiliations {
    input_generation: String,
    profile_id: ProfileId,
    as_of: ProfileDate,
    rows: BTreeMap<AthleteId, CurrentAffiliationResolution>,
    audit: audit::RetainedAudit,
}

impl PinnedCurrentAffiliations {
    #[cfg(test)]
    pub(crate) fn read(
        dataset: &ExportDataset,
        snapshot: &StoreSnapshot<'_>,
    ) -> ReportResult<Self> {
        let profile = pinned_profile(dataset, snapshot)?;
        let profile_id = profile.id().map_err(StoreError::from)?;
        let mut reader = observation::Reader::new(dataset, &profile, snapshot.root())?;
        snapshot.for_each_observation_bounded(
            Table::Athletes,
            OBSERVATION_BUDGET,
            |athlete, _| reader.accept(athlete),
        )?;
        let (rows, audit) = reader.finish();
        Ok(Self {
            input_generation: dataset.lineage.input_generation.clone(),
            profile_id: profile_id.clone(),
            as_of: profile.input().as_of.clone(),
            rows,
            audit,
        })
    }

    #[cfg(test)]
    pub(super) fn unavailable(&self) -> &BTreeMap<AthleteId, BTreeSet<UnavailableAffiliation>> {
        self.audit.unavailable()
    }

    pub(crate) fn into_verified(self) -> VerifiedCurrentAffiliations {
        VerifiedCurrentAffiliations {
            input_generation: self.input_generation,
            profile_id: self.profile_id,
            as_of: self.as_of,
            rows: self.rows,
            audit: self.audit.into_verified(),
        }
    }
}

fn invalid(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_string(),
    }
}

fn charge(current: &mut u64, added: usize, maximum: u64, what: &'static str) -> StoreResult<()> {
    let needed = current
        .checked_add(u64::try_from(added).map_err(|_| StoreError::CounterOverflow)?)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > maximum {
        return Err(StoreError::BudgetExceeded {
            what,
            needed,
            available: maximum,
        });
    }
    *current = needed;
    Ok(())
}

pub(super) fn aliases(dataset: &ExportDataset) -> StoreResult<()> {
    let athletes =
        u64::try_from(dataset.athletes.len()).map_err(|_| StoreError::CounterOverflow)?;
    if athletes > OBSERVATION_BUDGET.rows
        || dataset.canonical_aliases.len() > dataset.athletes.len()
    {
        return Err(invalid(
            "current affiliation alias inventory exceeds its ceiling",
        ));
    }
    let identities = dataset.identities();
    let mut count = 0usize;
    for athlete in &dataset.athletes {
        let canonical = identities.canonical_id(athlete.id.as_str());
        let expected = if canonical == athlete.id.as_str() {
            None
        } else {
            count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            Some(canonical)
        };
        if dataset
            .canonical_aliases
            .get(athlete.id.as_str())
            .map(String::as_str)
            != expected
        {
            return Err(invalid(
                "current affiliation alias map differs from accepted identity projection",
            ));
        }
    }
    if count != dataset.canonical_aliases.len() {
        return Err(invalid(
            "current affiliation alias map has foreign subjects",
        ));
    }
    Ok(())
}

fn pinned_profile(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
) -> ReportResult<census_domain::model::QueryProfileV1> {
    dataset.ensure_input_binding()?;
    let profile_id = dataset
        .selected_profile_id()
        .ok_or_else(|| invalid("current affiliation requires a selected profile"))?;
    let profile = snapshot.profile(profile_id)?;
    if snapshot.store_identity()?.as_str() != dataset.lineage.store_identity
        || snapshot.sequence() != dataset.lineage.snapshot_sequence
    {
        return Err(
            invalid("current affiliation requires the exact pinned source snapshot").into(),
        );
    }
    validate_profile(dataset, &profile)?;
    dataset.ensure_snapshot(snapshot)?;
    aliases(dataset)?;
    Ok(profile)
}

fn validate_profile(
    dataset: &ExportDataset,
    profile: &census_domain::model::QueryProfileV1,
) -> StoreResult<()> {
    if dataset.selected_as_of() != Some(&profile.input().as_of)
        || profile.input().include_provisional
            != census_domain::model::ProvisionalInclusionV1::Exclude
        || dataset.selected_profile_id() != Some(&profile.id()?)
        || census_domain::model::serialized_digest(dataset)? != dataset.lineage.input_digest
    {
        return Err(invalid(
            "current affiliation profile differs from its bound dataset",
        ));
    }
    Ok(())
}
