use crate::participation_evidence::{hash, invalid};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{ContractRefV1, SchoolYear};
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use census_store::{EvidenceSubjectAttachmentStoreError, RosterSourceEvidenceV1, StoreResult};

const SCOPE: &[u8] = br#"{"version":1,"revision":1,"contract":"roster-current-representation-v1","provider":"milesplit","statement":"scoped-current-roster-heading","relationship":"represents-school-program","temporal":"exact-utc-capture-day","enrollment":"unasserted","tenure":"unasserted"}"#;
const YEAR: &[u8] = br#"{"version":1,"revision":1,"contract":"roster-existing-program-key-year-v1","implementation":"SchoolYear::from_date","opening_month":8,"date":"utc-capture-day","authority":"existing-program-key-only","academic-season-coverage":"unasserted"}"#;
const MERGE: &[u8] = br#"{"version":1,"revision":1,"contract":"roster-existing-physical-projection-v1","tables":["Schools","Teams","Conflicts","SourceObservations"],"physical_inventory":"complete","implementation":"Entity::merge-and-publish","provider_binding":"unique-exact-existing","inspection":"each-physical-row-and-merged-row","global_conflicts":"refuse-all","selected_embedded_conflicts":"refuse","identity_minting":"forbidden"}"#;

pub struct RosterInterpretationContractsV1<'a> {
    pub scope: &'a [u8],
    pub year: &'a [u8],
    pub projection_merge: &'a [u8],
}

pub fn compiled_roster_interpretation_contracts_v1() -> RosterInterpretationContractsV1<'static> {
    RosterInterpretationContractsV1 { scope: SCOPE, year: YEAR, projection_merge: MERGE }
}

impl RosterInterpretationContractsV1<'_> {
    pub fn scope_reference(&self) -> StoreResult<ContractRefV1> { reference(self.scope) }
    pub fn year_reference(&self) -> StoreResult<ContractRefV1> { reference(self.year) }
    pub fn projection_merge_reference(&self) -> StoreResult<ContractRefV1> { reference(self.projection_merge) }
}

pub(super) fn assess(
    evidence: &RosterSourceEvidenceV1,
    retained: &RosterInterpretationContractsV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    evidence.require_original_grant(grant)?;
    let result = check(evidence, retained, proposal, grant);
    if result.is_err() { grant.poison(); }
    result
}

fn check(
    evidence: &RosterSourceEvidenceV1,
    retained: &RosterInterpretationContractsV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    for (actual, expected, claimed) in [
        (retained.scope, SCOPE, proposal.scope().policy()),
        (retained.year, YEAR, proposal.binding().year_convention()),
        (retained.projection_merge, MERGE, proposal.binding().projection_merge_contract()),
    ] {
        evidence.require_original_grant(grant)?;
        if actual.len() > 4096 { return Err(invalid("roster interpretation contract capacity")); }
        grant.charge_bytes(actual.len()).map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
        grant.charge_steps(1).map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
        if actual != expected || reference(actual)? != *claimed {
            return Err(invalid("roster retained interpretation contract mismatch"));
        }
    }
    if SchoolYear::from_date(proposal.capture().observed_day().as_str()) != Some(proposal.binding().school_year()) {
        return Err(invalid("roster existing program key year mismatch"));
    }
    evidence.require_original_grant(grant)
}

fn reference(bytes: &[u8]) -> StoreResult<ContractRefV1> {
    if bytes.len() > 4096 { return Err(invalid("roster interpretation contract capacity")); }
    ContractRefV1::new(1, 1, &hash(bytes)).map_err(|_| invalid("roster interpretation contract reference"))
}
