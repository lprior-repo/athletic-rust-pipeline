use super::{
    assemble_supplied, binding_value, capture_value, parents, parser_value, scope_value, Parents,
    SourceObjects,
};
use anyhow::{Context, Result};
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketV1;
use census_crawl::participation_evidence::recorded::read_retained_provider_roster_body_v1;
use census_crawl::participation_evidence::roster_source_issuance::{
    capture_objects::PreparedRosterCaptureObjectsV1,
    manifest::{
        assess_retained_roster_parser_manifest, compiled_roster_parser_manifest,
        CompiledRosterParserManifestV1,
    },
    policies::{compiled_roster_interpretation_contracts_v1, RosterInterpretationContractsV1},
    prepare_retained_roster_capture_objects_v3, RosterExistingBindingAssessmentV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::roster_affiliation_authority::{
    decode_roster_issuance_proposal_v1, LocallyCheckedRosterProposalV1, RosterAuthorityLimitsV1,
    RosterParserClaimV1,
};
use census_store::{
    RosterIssuanceInspectionV1, RosterSourceEvidenceV1, RosterSourceReviewObjectsV1,
};

pub(super) fn derive(
    packet: &ProviderSchoolPacketV1,
    inspection: &mut RosterIssuanceInspectionV1<'_>,
) -> Result<(LocallyCheckedRosterProposalV1, RosterSourceReviewObjectsV1)> {
    let (evidence, grant) = inspection
        .source_parts()
        .context("opening the inspection source parts")?;
    let (school_row, program_row) = RosterExistingBindingAssessmentV1::read_inspected_parents(
        evidence,
        evidence.requested_school(),
        evidence.requested_program(),
        grant,
    )
    .context("reading the inspected parents")?;
    let parents = parents(school_row, program_row)?;
    let encoded = packet.encode().context("encoding the retained packet")?;
    let roster = packet.roster();
    let observed_day = roster.acquired_at().date_naive().to_string();
    let requested_url = roster.requested_url();
    let compiled = compiled_roster_parser_manifest().context("compiling the parser manifest")?;
    let claim = compiled
        .parser_claim()
        .context("reading the parser claim")?;
    let checked = assess_retained_roster_parser_manifest(compiled.canonical_bytes(), &claim, grant)
        .context("assessing the parser manifest")?;
    let capture = prepare_retained_roster_capture_objects_v3(evidence, &checked, grant)
        .context("preparing the capture objects")?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let occurrence = capture
        .occurrence(grant)
        .context("reading the occurrence")?
        .to_owned();
    let capture_acquired = capture
        .acquired_at(grant)
        .context("reading the capture instant")?
        .to_owned();
    let objects = read_source_objects(encoded, &parents, &compiled, &capture, evidence, grant)?;
    let value = proposal_value(
        &parents,
        &contracts,
        &claim,
        &objects,
        evidence,
        &occurrence,
        &capture_acquired,
        &observed_day,
        requested_url,
    )?;
    finalize(value, &contracts, objects)
}

fn read_source_objects(
    encoded: Vec<u8>,
    parents: &Parents,
    compiled: &CompiledRosterParserManifestV1,
    capture: &PreparedRosterCaptureObjectsV1<'_>,
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<SourceObjects> {
    Ok(SourceObjects {
        canonical: compiled.canonical_bytes().to_vec(),
        schema: compiled
            .retained()
            .schema_bytes()
            .context("reading the schema object")?,
        configuration: compiled
            .retained()
            .configuration_bytes()
            .context("reading the configuration object")?,
        dependencies: compiled
            .retained()
            .dependencies_bytes()
            .context("reading the dependencies object")?,
        body: read_retained_provider_roster_body_v1(evidence, grant)
            .context("reading the validated roster body")?,
        metadata: capture
            .metadata_bytes(grant)
            .context("reading the metadata object")?
            .to_vec(),
        interpretation: capture
            .interpretation_bytes(grant)
            .context("reading the interpretation object")?
            .to_vec(),
        packet: encoded,
        school: serde_json::to_vec(&parents.school_row)
            .context("encoding the school projection")?,
        program: serde_json::to_vec(&parents.program_row)
            .context("encoding the program projection")?,
        inventory: evidence
            .physical_projection_manifest(grant)
            .context("reading the projection manifest")?,
        admission: evidence
            .retained_source_admission_manifest(grant)
            .context("reading the admission manifest")?,
        verification: evidence
            .verification_grant_manifest(grant)
            .context("reading the grant manifest")?,
    })
}

fn proposal_value(
    parents: &Parents,
    contracts: &RosterInterpretationContractsV1<'_>,
    claim: &RosterParserClaimV1,
    objects: &SourceObjects,
    evidence: &RosterSourceEvidenceV1,
    occurrence: &str,
    capture_acquired: &str,
    observed_day: &str,
    requested_url: &str,
) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "version": 1,
        "scope": scope_value(
            parents,
            &contracts.scope_reference().context("reading the scope reference")?,
        )?,
        "parser": parser_value(claim)?,
        "capture": capture_value(
            objects,
            occurrence,
            capture_acquired,
            observed_day,
            requested_url,
        )?,
        "binding": binding_value(
            parents,
            &contracts.year_reference().context("reading the year reference")?,
            &contracts
                .projection_merge_reference()
                .context("reading the merge reference")?,
            objects,
            evidence,
        )?,
    }))
}

fn finalize(
    value: serde_json::Value,
    contracts: &RosterInterpretationContractsV1<'_>,
    objects: SourceObjects,
) -> Result<(LocallyCheckedRosterProposalV1, RosterSourceReviewObjectsV1)> {
    let limits = RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024)
        .context("building the proposal limits")?;
    let bytes = serde_json::to_vec(&value).context("encoding the derived proposal")?;
    let proposal = decode_roster_issuance_proposal_v1(&bytes, limits)
        .context("decoding the derived proposal")?;
    let supplied = assemble_supplied(
        contracts.scope,
        contracts.year,
        contracts.projection_merge,
        objects,
    )?;
    Ok((proposal, supplied))
}
