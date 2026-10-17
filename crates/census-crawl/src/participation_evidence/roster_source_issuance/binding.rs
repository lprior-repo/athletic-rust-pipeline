use super::{source::budget, RosterSourceEvidenceAssessmentV1};
use crate::participation_evidence::{hash, invalid, json};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{
    CanonicalSchool, CanonicalTeam, NaturalKey, RetainedConflict, SourceIdentity, SourceNamespace,
    SourceObservation, SourceSchoolObservation,
};
use census_domain::roster_affiliation_authority::{
    RosterAuthorityObjectRefV1, RosterExistingBindingClaimV1,
};
use census_store::{
    decode_physical_observation, Entity, RosterSourceEvidenceV1, StoreError, StoreResult, Table,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterBindingUnverifiedReferenceV1 {}

pub struct RosterExistingBindingAssessmentV1<'a, 'e> {
    source: &'a RosterSourceEvidenceAssessmentV1<'e>,
    school: CanonicalSchool,
    program: CanonicalTeam,
}

impl RosterExistingBindingAssessmentV1<'_, '_> {
    pub fn evidence(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&RosterSourceEvidenceV1> {
        self.source.evidence(grant)
    }

    pub fn school(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&CanonicalSchool> {
        self.source.require_original_grant(grant)?;
        Ok(&self.school)
    }

    pub fn program(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&CanonicalTeam> {
        self.source.require_original_grant(grant)?;
        Ok(&self.program)
    }

    pub fn conflict_policy(&self) -> &'static str {
        "all global conflicts absent; selected provider owners have no embedded conflicts"
    }

    pub fn unverified_references(&self) -> &'static [RosterBindingUnverifiedReferenceV1] {
        &[]
    }
}

pub fn assess_existing_roster_binding<'a, 'e>(
    source: &'a RosterSourceEvidenceAssessmentV1<'e>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RosterExistingBindingAssessmentV1<'a, 'e>> {
    source.require_original_grant(grant)?;
    let result = assess(source, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn assess<'a, 'e>(
    source: &'a RosterSourceEvidenceAssessmentV1<'e>,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RosterExistingBindingAssessmentV1<'a, 'e>> {
    let evidence = source.evidence(grant)?;
    let proposal = source.proposal(grant)?;
    let checked = source.checked_packet(grant)?;
    let binding = proposal.binding();
    let mut school = None;
    scan::<CanonicalSchool>(
        evidence,
        Table::Schools,
        grant,
        |row| school_row(row, binding, checked.school()),
        |row| select(&mut school, row, binding.school().as_str()),
    )?;
    let mut program = None;
    scan::<CanonicalTeam>(
        evidence,
        Table::Teams,
        grant,
        |row| team_row(row, binding),
        |row| select(&mut program, row, binding.program().as_str()),
    )?;
    scan::<RetainedConflict>(
        evidence,
        Table::Conflicts,
        grant,
        no_global_conflict,
        |_| Ok(()),
    )?;
    let mut observations = 0_usize;
    scan::<SourceObservation>(
        evidence,
        Table::SourceObservations,
        grant,
        |row| {
            if observation(row, checked.observation())? {
                observations = observations
                    .checked_add(1)
                    .ok_or(StoreError::CounterOverflow)?;
            }
            Ok(())
        },
        |row| observation(&row, checked.observation()).map(|_| ()),
    )?;
    if observations == 0 {
        return Err(invalid("roster binding exact packet observation missing").into());
    }
    let school = school.ok_or_else(|| invalid("roster binding existing school missing"))?;
    let program = program.ok_or_else(|| invalid("roster binding existing program missing"))?;
    school_row(&school, binding, checked.school())?;
    team_row(&program, binding)?;
    references(evidence, binding, &school, &program, grant)?;
    evidence.require_original_grant(grant)?;
    Ok(RosterExistingBindingAssessmentV1 {
        source,
        school,
        program,
    })
}

mod projection;
mod rows;
#[cfg(test)]
use projection::{projection_reference, require_reference};
use projection::{references, scan};
use rows::{no_global_conflict, observation, school_row, select, team_row};

#[cfg(test)]
#[path = "binding_tests.rs"]
mod tests;
