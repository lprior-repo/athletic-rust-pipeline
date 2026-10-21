use super::*;
use crate::participation_evidence::roster_source_issuance::policies::RosterInterpretationContractsV1;
use census_domain::roster_affiliation_authority::RosterParserClaimV1;
use census_store::RosterSourceEvidenceV1;
use serde_json::{json, Value};

fn object(bytes: &[u8]) -> Value {
    json!({"digest": crate::net::cache::content_digest(bytes), "bytes": bytes.len()})
}

pub(super) fn derive(
    school: &CanonicalSchool,
    program: &CanonicalTeam,
    objects: &SourceObjects,
    evidence: &RosterSourceEvidenceV1,
    parser: &RosterParserClaimV1,
    contracts: &RosterInterpretationContractsV1<'_>,
    occurrence: &str,
    acquired: &str,
    packet: &PublicSchoolIntakePacketV1,
) -> TestResult<Value> {
    let owner = SourceSubject::new(SourceNamespace::MilesplitSchool, TEAM.into())?;
    let program_source = SourceSubject::new(SourceNamespace::MilesplitTeam, TEAM.into())?;
    let day = chrono::DateTime::parse_from_rfc3339(acquired)?
        .date_naive()
        .to_string();
    let url = packet.roster().work().request().original_url();
    check!(eq; packet.roster().response().response_url(), url);
    Ok(json!({
        "version": 1,
        "scope": {
            "provider": "milesplit", "owner": owner,
            "sport": program.sport, "category": program.gender,
            "policy": contracts.scope_reference()?,
            "relationship": "represents_school_program", "temporal": "exact_utc_capture_day"
        },
        "parser": parser,
        "capture": capture(objects, occurrence, acquired, &day, url),
        "binding": binding(school, program, objects, evidence, contracts, owner, program_source)?
    }))
}

fn capture(
    objects: &SourceObjects,
    occurrence: &str,
    acquired: &str,
    day: &str,
    url: &str,
) -> Value {
    json!({
        "occurrence": occurrence, "body": object(&objects.body),
        "manifest": object(&objects.metadata), "interpretation": object(&objects.interpretation),
        "acquired_at": acquired, "observed_day": day, "requested_url": url,
        "final_url": {"status": "observed", "url": url},
        "retained_source_admission": object(&objects.admission),
        "verification_grant": object(&objects.verification)
    })
}

fn binding(
    school: &CanonicalSchool,
    program: &CanonicalTeam,
    objects: &SourceObjects,
    evidence: &RosterSourceEvidenceV1,
    contracts: &RosterInterpretationContractsV1<'_>,
    owner: SourceSubject,
    program_source: SourceSubject,
) -> TestResult<Value> {
    Ok(json!({
        "school": school.id, "program": program.id, "school_source": owner,
        "program_source": program_source, "sport": program.sport, "category": program.gender,
        "school_year": program.school_year, "year_convention": contracts.year_reference()?,
        "projection_merge_contract": contracts.projection_merge_reference()?,
        "school_projection": object(&objects.school), "program_projection": object(&objects.program),
        "provider_packet": object(&objects.packet),
        "physical_projection_inventory": object(&objects.inventory),
        "store": evidence.store_identity(), "snapshot_revision": evidence.snapshot_sequence()
    }))
}
