use crate::milesplit::{LocatedRosterAthlete, SourceRowLocator};
use crate::participation_evidence::roster_source_issuance::RosterSourceEvidenceAssessmentV1;
use crate::participation_evidence::{bound, hash, invalid, json};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::StoreResult;

use super::super::rows;
use super::super::wire;

fn roster_authority_error_to_store(
    error: census_domain::roster_affiliation_authority::RosterAuthorityError,
) -> census_store::StoreError {
    match error {
        census_domain::roster_affiliation_authority::RosterAuthorityError::Invalid(detail) => {
            census_store::StoreError::Refused { detail: detail.into() }
        }
        census_domain::roster_affiliation_authority::RosterAuthorityError::UnsupportedVersion => {
            invalid("unsupported roster authority version")
        }
        census_domain::roster_affiliation_authority::RosterAuthorityError::UnsupportedLifecycle => {
            invalid("unsupported roster authority lifecycle")
        }
        census_domain::roster_affiliation_authority::RosterAuthorityError::AcquisitionDayMismatch => {
            invalid("roster acquisition day mismatch")
        }
        census_domain::roster_affiliation_authority::RosterAuthorityError::BindingScopeMismatch => {
            invalid("roster binding scope mismatch")
        }
        census_domain::roster_affiliation_authority::RosterAuthorityError::ConflictingObjectReference => {
            invalid("roster conflicting object reference")
        }
        census_domain::roster_affiliation_authority::RosterAuthorityError::Capacity {
            resource,
            needed,
            available,
        } => census_store::StoreError::BudgetExceeded {
            what: resource,
            needed,
            available,
        },
        census_domain::roster_affiliation_authority::RosterAuthorityError::Overflow => {
            census_store::StoreError::CounterOverflow
        }
        census_domain::roster_affiliation_authority::RosterAuthorityError::Allocation => {
            invalid("roster member witness allocation refused")
        }
        census_domain::roster_affiliation_authority::RosterAuthorityError::Decode(source) => {
            json(source)
        }
        census_domain::roster_affiliation_authority::RosterAuthorityError::Encode(source) => {
            source.into()
        }
    }
}

fn roster_authority_limits(
) -> StoreResult<census_domain::roster_affiliation_authority::RosterAuthorityLimitsV1> {
    census_domain::roster_affiliation_authority::RosterAuthorityLimitsV1::new(
        64 * 1024,
        8 * 1024 * 1024,
    )
    .map_err(roster_authority_error_to_store)
}

impl RosterSourceEvidenceAssessmentV1<'_> {
    pub fn member_semantic_witness_v1(
        &self,
        row: SourceRowLocator,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Vec<u8>> {
        self.require_original_grant(grant)?;
        let checked = self.checked_packet(grant)?;
        let rows = rows::inventory(checked.roster_verdict(), grant)?;
        let index = member_find_index(&rows, row)?;
        let entry = rows
            .get(index)
            .ok_or_else(|| invalid("roster member row index out of range"))?;
        match &entry.outcome {
            wire::Outcome::Admitted { semantic_digest } => {
                let preimage = self.member_reconstruct_preimage(checked, entry, grant)?;
                let computed = hash(&preimage);
                if computed != *semantic_digest {
                    return Err(invalid("roster member witness preimage digest mismatch"));
                }
                let limits = roster_authority_limits()?;
                let witness =
                    census_domain::roster_affiliation_authority::encode_roster_member_witness_v1(
                        index, &preimage, limits,
                    )
                    .map_err(roster_authority_error_to_store)?;
                bound(witness.len(), 64 * 1024, "roster member witness")?;
                grant.charge_bytes(witness.len()).map_err(|e| {
                    census_store::StoreError::from(
                        census_store::EvidenceSubjectAttachmentStoreError::Domain(e),
                    )
                })?;
                Ok(witness)
            }
            wire::Outcome::Rejected { .. } => Err(invalid("roster member row is not admitted")),
        }
    }

    fn member_reconstruct_preimage(
        &self,
        checked: &crate::participation_evidence::provider_school::CheckedProviderSchoolPacketV1,
        entry: &wire::Row,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Vec<u8>> {
        let roster = checked
            .roster_verdict()
            .roster()
            .ok_or_else(|| invalid("roster not available for member preimage reconstruction"))?;
        for member in &roster.athletes {
            if member.locator() == entry.row {
                return member_encode_preimage(member, grant);
            }
        }
        Err(invalid(
            "roster member not found for preimage reconstruction",
        ))
    }
}

fn member_find_index(rows: &[wire::Row], row: SourceRowLocator) -> StoreResult<usize> {
    for (index, entry) in rows.iter().enumerate() {
        if entry.row == row {
            return Ok(index);
        }
    }
    Err(invalid("roster member row not found in semantic inventory"))
}

fn member_encode_preimage(
    member: &LocatedRosterAthlete,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let value = member.athlete();
    let current = match member.current_roster() {
        Some(current) => current,
        None => {
            return Err(invalid("roster member current roster missing"));
        }
    };
    let preimage = super::super::encode(
        &(
            "row-local-admitted-semantics-v2",
            &value.roster_name,
            &value.name,
            value.gender,
            value.grad_year,
            &value.athlete_id,
            &value.profile_url,
            [value.indoor, value.outdoor, value.xc],
            member.locator(),
            (
                current.owner(),
                current.member(),
                current.row(),
                current.section(),
                current.container(),
                current.heading(),
                current.heading_sha256(),
                [
                    current.supports(census_domain::model::Sport::IndoorTrack),
                    current.supports(census_domain::model::Sport::OutdoorTrack),
                    current.supports(census_domain::model::Sport::CrossCountry),
                ],
            ),
        ),
        grant,
    )?;
    Ok(preimage)
}
