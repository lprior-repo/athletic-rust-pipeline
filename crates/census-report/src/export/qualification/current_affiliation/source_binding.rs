use super::source_authority::SourceAuthorityFamilyInventoryV1;
use super::{invalid, MAX_OBSERVATIONS, MAX_OBSERVATION_BYTES};
use crate::export::qualification::physical_observations;
use crate::export::{proof_objects, ExportDataset};
use crate::report::ReportResult;
use census_domain::affiliation_candidates::{DeclaredFrozenView, ObjectRef};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalAthlete, QueryProfileV1};
use census_store::{
    CurrentRosterReviewedClaimsV1, ReadBudget, ReplayedRosterReviewedClaimsV1,
    ReviewedRosterSourceClaimV1, StoreError, StoreResult,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;
use std::path::Path;

mod capture;
mod object;
mod replay;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceBindingObjectV1 {
    pub(super) digest: String,
    pub(super) bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RosterSourceClosureV1 {
    version: u32,
    lineage: SourceBindingObjectV1,
    reviewed: census_store::RosterFrozenReviewedClaimsReferenceV1,
    athletes: physical_observations::PhysicalObservationDescriptor,
    captured_at: DateTime<Utc>,
    acquisition_cutoff: DateTime<Utc>,
}

enum Claims<'a> {
    Current(&'a CurrentRosterReviewedClaimsV1),
    Frozen(&'a ReplayedRosterReviewedClaimsV1),
}

pub(super) struct BoundRosterSourceViewV1<'a> {
    dataset: &'a ExportDataset,
    objects: &'a Path,
    claims: Claims<'a>,
    closure: RosterSourceClosureV1,
    closure_object: SourceBindingObjectV1,
    expected_family: Option<SourceBindingObjectV1>,
}

impl BoundRosterSourceViewV1<'_> {
    pub(super) fn require_binding(
        &self,
        dataset: &ExportDataset,
        profile: &QueryProfileV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        let result = (|| {
            self.require_original(grant)?;
            dataset
                .ensure_input_binding()
                .map_err(|_| invalid("reviewed roster dataset input binding differs"))?;
            super::validate_profile(dataset, profile)?;
            if dataset.lineage != self.dataset.lineage
                || self.closure.reviewed.store_identity().as_str() != dataset.lineage.store_identity
                || self.closure.reviewed.selected_snapshot_sequence()
                    != dataset.lineage.snapshot_sequence
                || self.closure.acquisition_cutoff > self.closure.captured_at
            {
                return Err(invalid("reviewed roster publication binding differs"));
            }
            if self.closure.athletes.table() != census_store::Table::Athletes {
                return Err(invalid("reviewed roster physical closure is not Athletes"));
            }
            let authority = dataset
                .selected_authority()
                .map_err(|_| invalid("reviewed roster selected authority is unavailable"))?;
            let expected = authority
                .fingerprints()
                .get(census_store::Table::Athletes.file())
                .ok_or_else(|| invalid("reviewed roster independent athlete root is absent"))?;
            self.closure
                .athletes
                .validate(dataset, expected, budget())?;
            self.require_original(grant)
        })();
        if result.is_err() {
            grant.poison();
        }
        result
    }

    pub(super) fn claims(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&[ReviewedRosterSourceClaimV1]> {
        match &self.claims {
            Claims::Current(value) => value.claims(grant),
            Claims::Frozen(value) => value.claims(grant),
        }
    }

    pub(super) fn visit_athletes(
        &self,
        grant: &mut AttachmentReplayGrantV1,
        mut visit: impl FnMut(CanonicalAthlete, &mut AttachmentReplayGrantV1) -> StoreResult<()>,
    ) -> ReportResult<()> {
        let result = (|| {
            self.require_original(grant)?;
            let counts = self.closure.athletes.counts();
            grant
                .charge_bytes(
                    usize::try_from(counts.encoded_bytes)
                        .map_err(|_| StoreError::CounterOverflow)?,
                )
                .map_err(domain)?;
            grant
                .charge_steps(
                    usize::try_from(counts.key_visits).map_err(|_| StoreError::CounterOverflow)?,
                )
                .map_err(domain)?;
            physical_observations::visit(
                self.dataset,
                self.objects,
                &self.closure.athletes,
                self.closure.athletes.fingerprint(),
                budget(),
                |row| {
                    self.require_original(grant)?;
                    visit(row, grant)?;
                    self.require_original(grant)
                },
            )?;
            self.require_original(grant)?;
            Ok(())
        })();
        if result.is_err() {
            grant.poison();
        }
        result
    }

    pub(super) fn declared_view<'a>(
        &'a self,
        family: &'a SourceAuthorityFamilyInventoryV1,
    ) -> StoreResult<DeclaredFrozenView<'a>> {
        if let Some(expected) = &self.expected_family {
            if expected.digest != family.digest()
                || expected.bytes
                    != u64::try_from(family.bytes().len())
                        .map_err(|_| StoreError::CounterOverflow)?
            {
                return Err(invalid("reviewed roster subject family inventory differs"));
            }
        }
        Ok(DeclaredFrozenView {
            lineage: reference(&self.closure.lineage.digest)?,
            revision: NonZeroU64::new(self.dataset.lineage.snapshot_sequence)
                .ok_or_else(|| invalid("reviewed roster selected revision is zero"))?,
            captured_at: self.closure.captured_at,
            closure_object: reference(&self.closure_object.digest)?,
            family_inventory: reference(family.digest())?,
        })
    }

    pub(super) fn acquisition_cutoff(&self) -> DateTime<Utc> {
        self.closure.acquisition_cutoff
    }

    fn require_original(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        match &self.claims {
            Claims::Current(value) => value.require_original_grant(grant),
            Claims::Frozen(value) => value.require_original_grant(grant),
        }
    }
}

fn reference(value: &str) -> StoreResult<ObjectRef<'_>> {
    ObjectRef::parse(value).map_err(|_| invalid("reviewed roster object reference is invalid"))
}

fn budget() -> ReadBudget {
    ReadBudget {
        rows: MAX_OBSERVATIONS,
        bytes: MAX_OBSERVATION_BYTES,
        record_bytes: 1024 * 1024,
    }
}

fn domain(
    error: census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError,
) -> StoreError {
    census_store::EvidenceSubjectAttachmentStoreError::from(error).into()
}
