use super::super::AffiliationEvidenceAudit;
use super::{charge, invalid, UnavailableAffiliation as Gap};
use crate::export::qualification::physical_observations::OBSERVATION_BUDGET;
use census_domain::model::AthleteId;
use census_store::{StoreError, StoreResult};
use std::collections::{BTreeMap, BTreeSet};

const MAX_REFERENCES_PER_SUBJECT: u64 = 256;
const MAX_REFERENCE_BYTES: usize = 8192;

#[derive(Default)]
pub(super) struct RetainedAudit {
    pub(super) context: super::context::RosterEvidence,
    references: BTreeMap<AthleteId, BTreeSet<String>>,
    unavailable: BTreeMap<AthleteId, BTreeSet<Gap>>,
    reference_count: u64,
    reason_count: u64,
    bytes: u64,
}

impl RetainedAudit {
    pub(super) fn reference(&mut self, id: &AthleteId, reference: &str) -> StoreResult<()> {
        if reference.is_empty() || reference.len() > MAX_REFERENCE_BYTES {
            return Err(invalid(
                "current affiliation audit reference has invalid length",
            ));
        }
        let references = self.references.get(id);
        if references.is_some_and(|references| references.contains(reference)) {
            return Ok(());
        }
        let mut count = u64::try_from(references.map_or(0, BTreeSet::len))
            .map_err(|_| StoreError::CounterOverflow)?;
        charge(
            &mut count,
            1,
            MAX_REFERENCES_PER_SUBJECT,
            "current affiliation audit references per subject",
        )?;
        let mut reference_count = self.reference_count;
        let mut bytes = self.bytes;
        charge(
            &mut reference_count,
            1,
            OBSERVATION_BUDGET.rows,
            "current affiliation audit reference count",
        )?;
        charge(
            &mut bytes,
            reference.len(),
            OBSERVATION_BUDGET.bytes,
            "current affiliation audit bytes",
        )?;
        self.references
            .entry(id.clone())
            .or_default()
            .insert(reference.to_string());
        self.reference_count = reference_count;
        self.bytes = bytes;
        Ok(())
    }

    pub(super) fn gap(&mut self, id: AthleteId, reason: Gap) -> StoreResult<()> {
        if self
            .unavailable
            .get(&id)
            .is_some_and(|reasons| reasons.contains(&reason))
        {
            return Ok(());
        }
        let mut reason_count = self.reason_count;
        let mut bytes = self.bytes;
        charge(
            &mut reason_count,
            1,
            OBSERVATION_BUDGET.rows,
            "current affiliation audit reason count",
        )?;
        charge(
            &mut bytes,
            reason.label().len(),
            OBSERVATION_BUDGET.bytes,
            "current affiliation audit bytes",
        )?;
        self.unavailable.entry(id).or_default().insert(reason);
        self.reason_count = reason_count;
        self.bytes = bytes;
        Ok(())
    }

    pub(super) fn unavailable(&self) -> &BTreeMap<AthleteId, BTreeSet<Gap>> {
        &self.unavailable
    }

    pub(super) fn into_verified(self) -> BTreeMap<AthleteId, AffiliationEvidenceAudit> {
        let mut audit: BTreeMap<_, _> = self
            .references
            .into_iter()
            .map(|(id, references)| {
                (
                    id,
                    AffiliationEvidenceAudit {
                        references: references.into_iter().collect(),
                        unavailable: Vec::new(),
                    },
                )
            })
            .collect();
        for (id, reasons) in self.unavailable {
            let mut unavailable: Vec<_> = reasons
                .into_iter()
                .map(|reason| reason.label().to_string())
                .collect();
            unavailable.sort();
            audit.entry(id).or_default().unavailable = unavailable;
        }
        audit
    }
}

impl Gap {
    fn label(self) -> &'static str {
        match self {
            Self::NoRetainedRow => "no_retained_row",
            Self::IdentityUnaccepted => "identity_unaccepted",
            Self::OutsideCaptureDay => "outside_capture_day",
            Self::ArchiveUnavailable => "archive_unavailable",
            Self::SchoolUnresolved => "school_unresolved",
            Self::OutsideSelectedSport => "outside_selected_sport",
            Self::MissingTemporalAuthority => "missing_temporal_authority",
            Self::TemporalStatementAmbiguous => "temporal_statement_ambiguous",
            Self::TemporalMarkupMalformed => "temporal_markup_malformed",
            Self::TemporalResourceLimited => "temporal_resource_limited",
            Self::TemporalMemberInactive => "temporal_member_inactive",
        }
    }
}

#[cfg(test)]
mod tests;
