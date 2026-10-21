use super::super::source::TestResult;
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketV1;
use census_crawl::participation_evidence::roster_source_issuance::{
    assess_existing_roster_binding, assess_retained_roster_source_v3,
    capture_objects::PreparedRosterCaptureObjectsV1,
    manifest::{assess_retained_roster_parser_manifest, compiled_roster_parser_manifest},
    policies::compiled_roster_interpretation_contracts_v1,
    prepare_retained_roster_capture_objects_v3, RosterSourceEvidenceAssessmentV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalSchool, CanonicalTeam, SourceNamespace, SourceSubject};
use census_domain::roster_affiliation_authority::{
    decode_roster_issuance_claim, LocallyCheckedRosterIssuanceV1, RosterAuthorityLimitsV1,
};
use census_store::{
    RosterSourceEvidenceV1, RosterSourceReviewFieldsV1, RosterSourceReviewObjectsV1,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
pub(in super::super) fn prepare(
    school: &CanonicalSchool,
    program: &CanonicalTeam,
    evidence: &RosterSourceEvidenceV1,
    fields: &RosterSourceReviewFieldsV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<(LocallyCheckedRosterIssuanceV1, RosterSourceReviewObjectsV1)> {
    let retained = evidence.retained_root(grant)?;
    let packet: ProviderSchoolPacketV1 =
        serde_json::from_slice(retained.payload().get().as_bytes())?;
    let checked = packet.read()?;
    if checked.school().id != school.id || checked.reference().as_str() != retained.root().key {
        return Err("inspected current school packet differs from requested binding".into());
    }
    let parser = compiled_roster_parser_manifest()?;
    let checked_parser = assess_retained_roster_parser_manifest(
        parser.canonical_bytes(),
        &parser.parser_claim()?,
        grant,
    )?;
    let objects = prepare_retained_roster_capture_objects_v3(evidence, &checked_parser, grant)?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let semantic = serde_json::to_vec(fields)?;
    let owner = school_subject(school)?;
    let value = json!({
        "version": 1,
        "scope": {
            "provider": "milesplit", "owner": owner,
            "sport": program.sport, "category": program.gender,
            "policy": contracts.scope_reference()?,
            "relationship": "represents_school_program", "temporal": "exact_utc_capture_day"
        },
        "parser": parser.parser_claim()?,
        "capture": capture(&packet, &objects, evidence, grant)?,
        "binding": binding(school, program, &packet, evidence, grant)?,
        "semantic_review_evidence": object(&semantic)
    });
    let issuance = decode_roster_issuance_claim(
        &serde_json::to_vec(&value)?,
        RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024)?,
    )?;
    let source = assess_retained_roster_source_v3(
        evidence,
        parser.canonical_bytes(),
        &contracts,
        &issuance,
        grant,
    )?;
    let bound = assess_existing_roster_binding(&source, grant)?;
    if bound.school(grant)? != school || bound.program(grant)? != program {
        return Err("reviewed current school binding differs from existing projections".into());
    }
    let supplied = review_objects(&source, semantic, grant)?;
    Ok((issuance, supplied))
}

fn school_subject(school: &CanonicalSchool) -> TestResult<SourceSubject> {
    let mut identities = school
        .source_identities
        .iter()
        .filter(|identity| identity.namespace == SourceNamespace::MilesplitSchool);
    let identity = identities
        .next()
        .ok_or("current school provider identity missing")?;
    if identities.next().is_some() {
        return Err("current school provider identity ambiguous".into());
    }
    Ok(SourceSubject::new(
        SourceNamespace::MilesplitSchool,
        identity.id.clone(),
    )?)
}

fn program_subject(program: &CanonicalTeam) -> TestResult<SourceSubject> {
    let mut identities = program
        .source_identities
        .iter()
        .filter(|identity| identity.namespace == SourceNamespace::MilesplitTeam);
    let identity = identities
        .next()
        .ok_or("current program provider identity missing")?;
    if identities.next().is_some() {
        return Err("current program provider identity ambiguous".into());
    }
    Ok(SourceSubject::new(
        SourceNamespace::MilesplitTeam,
        identity.id.clone(),
    )?)
}

fn capture(
    packet: &ProviderSchoolPacketV1,
    objects: &PreparedRosterCaptureObjectsV1<'_>,
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<Value> {
    let metadata: Value = serde_json::from_slice(objects.metadata_bytes(grant)?)?;
    let body = metadata
        .get("body")
        .ok_or("retained roster body commitment missing")?;
    let url = packet.roster().requested_url();
    Ok(json!({
        "occurrence": objects.occurrence(grant)?,
        "body": body,
        "manifest": object(objects.metadata_bytes(grant)?),
        "interpretation": object(objects.interpretation_bytes(grant)?),
        "acquired_at": objects.acquired_at(grant)?,
        "observed_day": packet.roster().acquired_at().date_naive().to_string(),
        "requested_url": url,
        "final_url": {"status": "observed", "url": url},
        "retained_source_admission": object(&evidence.retained_source_admission_manifest(grant)?),
        "verification_grant": object(&evidence.verification_grant_manifest(grant)?)
    }))
}

fn binding(
    school: &CanonicalSchool,
    program: &CanonicalTeam,
    packet: &ProviderSchoolPacketV1,
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<Value> {
    let contracts = compiled_roster_interpretation_contracts_v1();
    Ok(json!({
        "school": school.id,
        "program": program.id,
        "school_source": school_subject(school)?,
        "program_source": program_subject(program)?,
        "sport": program.sport,
        "category": program.gender,
        "school_year": program.school_year,
        "year_convention": contracts.year_reference()?,
        "projection_merge_contract": contracts.projection_merge_reference()?,
        "school_projection": object(&serde_json::to_vec(school)?),
        "program_projection": object(&serde_json::to_vec(program)?),
        "provider_packet": object(&packet.encode()?),
        "physical_projection_inventory": object(&evidence.physical_projection_manifest(grant)?),
        "store": evidence.store_identity(),
        "snapshot_revision": evidence.snapshot_sequence()
    }))
}

fn review_objects(
    source: &RosterSourceEvidenceAssessmentV1<'_>,
    semantic: Vec<u8>,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<RosterSourceReviewObjectsV1> {
    let parser = source.parser_manifest(grant)?.retained();
    let binding = assess_existing_roster_binding(source, grant)?;
    let capture = source.capture_objects(grant)?;
    let evidence = source.evidence(grant)?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    Ok(RosterSourceReviewObjectsV1 {
        objects: vec![
            parser.canonical_bytes().to_vec(),
            parser.schema_bytes()?,
            parser.configuration_bytes()?,
            parser.dependencies_bytes()?,
            source.roster_body(grant)?.as_bytes().to_vec(),
            capture.metadata_bytes(grant)?.to_vec(),
            capture.interpretation_bytes(grant)?.to_vec(),
            source.retained_packet(grant)?.encode()?,
            serde_json::to_vec(binding.school(grant)?)?,
            serde_json::to_vec(binding.program(grant)?)?,
            evidence.physical_projection_manifest(grant)?,
            evidence.retained_source_admission_manifest(grant)?,
            evidence.verification_grant_manifest(grant)?,
            semantic,
        ],
        scope_policy: contracts.scope.to_vec(),
        year_convention: contracts.year.to_vec(),
        projection_merge_contract: contracts.projection_merge.to_vec(),
    })
}

fn object(bytes: &[u8]) -> Value {
    json!({"digest": format!("{:x}", Sha256::digest(bytes)), "bytes": bytes.len()})
}
