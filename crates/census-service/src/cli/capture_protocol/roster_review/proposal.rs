use anyhow::{ensure, Context, Result};
use census_crawl::participation_evidence::provider_school::{
    ProviderSchoolPacketRefV1, ProviderSchoolPacketV1,
};
use census_crawl::participation_evidence::recorded::read_retained_provider_roster_body_v1;
use census_crawl::participation_evidence::roster_source_issuance::{
    manifest::{assess_retained_roster_parser_manifest, compiled_roster_parser_manifest},
    policies::compiled_roster_interpretation_contracts_v1,
    prepare_retained_roster_capture_objects_v3, RosterExistingBindingAssessmentV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{
    CanonicalSchool, CanonicalTeam, ContractRefV1, SourceIdentity, SourceNamespace, SourceSubject,
};
use census_domain::roster_affiliation_authority::{
    decode_roster_issuance_proposal_v1, LocallyCheckedRosterProposalV1, RosterAuthorityLimitsV1,
    RosterParserClaimV1,
};
use census_store::{
    RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1, RosterIssuanceInspectionV1,
    RosterSourceEvidenceV1, RosterSourceReviewObjectsV1, Store,
};
use sha2::Digest;

struct Parents {
    school_row: CanonicalSchool,
    program_row: CanonicalTeam,
    owner: SourceSubject,
    program_subject: SourceSubject,
}

struct SourceObjects {
    canonical: Vec<u8>,
    schema: Vec<u8>,
    configuration: Vec<u8>,
    dependencies: Vec<u8>,
    body: Vec<u8>,
    metadata: Vec<u8>,
    interpretation: Vec<u8>,
    packet: Vec<u8>,
    school: Vec<u8>,
    program: Vec<u8>,
    inventory: Vec<u8>,
    admission: Vec<u8>,
    verification: Vec<u8>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", sha2::Sha256::digest(bytes))
}

fn object(bytes: &[u8]) -> serde_json::Value {
    serde_json::json!({"digest": sha256_hex(bytes), "bytes": bytes.len()})
}

pub(super) fn retained(
    store: &Store,
    reference: &ProviderSchoolPacketRefV1,
) -> Result<RetainedCaptureRootReadV1> {
    store
        .snapshot()
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
            reference.as_str(),
            &mut AttachmentReplayGrantV1::new(),
        )
        .context("reading the retained packet root")?
        .context("retained packet missing")
}

pub(super) fn packet(held: &RetainedCaptureRootReadV1) -> Result<ProviderSchoolPacketV1> {
    let packet: ProviderSchoolPacketV1 = serde_json::from_slice(held.payload().get().as_bytes())
        .context("decoding the retained packet")?;
    ensure!(
        packet
            .reference()
            .context("referencing the retained packet")?
            .as_str()
            == held.root().key,
        "retained packet key differs"
    );
    Ok(packet)
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
enum ProviderOwnerError {
    #[error("retained projection has multiple distinct provider owners")]
    Ambiguous,
    #[error("retained school and program provider owners disagree")]
    Mismatch,
}

fn unique_provider<'a>(
    identities: &'a [SourceIdentity],
    namespace: &SourceNamespace,
) -> Result<&'a SourceIdentity> {
    let mut candidates = identities
        .iter()
        .filter(|identity| &identity.namespace == namespace);
    let first = candidates
        .next()
        .context("retained projection lacks its provider owner")?;
    if candidates.any(|identity| identity.id != first.id) {
        return Err(ProviderOwnerError::Ambiguous.into());
    }
    Ok(first)
}

fn provider_subjects(
    school_owners: &[SourceIdentity],
    program_owners: &[SourceIdentity],
) -> Result<(SourceSubject, SourceSubject)> {
    let provider = unique_provider(school_owners, &SourceNamespace::MilesplitSchool)?;
    let program_provider = unique_provider(program_owners, &SourceNamespace::MilesplitTeam)?;
    if provider.id != program_provider.id {
        return Err(ProviderOwnerError::Mismatch.into());
    }
    let owner = SourceSubject::new(SourceNamespace::MilesplitSchool, provider.id.clone())
        .context("building the owner subject")?;
    let program = SourceSubject::new(SourceNamespace::MilesplitTeam, program_provider.id.clone())
        .context("building the program subject")?;
    Ok((owner, program))
}

fn parents(school_row: CanonicalSchool, program_row: CanonicalTeam) -> Result<Parents> {
    let (owner, program_subject) = provider_subjects(
        &school_row.source_identities,
        &program_row.source_identities,
    )?;
    Ok(Parents {
        school_row,
        program_row,
        owner,
        program_subject,
    })
}

fn scope_value(parents: &Parents, policy: &ContractRefV1) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "provider": "milesplit",
        "owner": parents.owner,
        "sport": parents.program_row.sport,
        "category": parents.program_row.gender,
        "policy": serde_json::to_value(policy).context("encoding the scope policy")?,
        "relationship": "represents_school_program",
        "temporal": "exact_utc_capture_day"
    }))
}

fn parser_value(claim: &RosterParserClaimV1) -> Result<serde_json::Value> {
    serde_json::to_value(claim).context("encoding the parser claim")
}

fn capture_value(
    objects: &SourceObjects,
    occurrence: &str,
    acquired: &str,
    day: &str,
    url: &str,
) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "occurrence": occurrence,
        "body": {"digest": sha256_hex(&objects.body), "bytes": objects.body.len()},
        "manifest": object(&objects.metadata),
        "interpretation": object(&objects.interpretation),
        "acquired_at": acquired,
        "observed_day": day,
        "requested_url": url,
        "final_url": {"status": "observed", "url": url},
        "retained_source_admission": object(&objects.admission),
        "verification_grant": object(&objects.verification)
    }))
}

fn binding_value(
    parents: &Parents,
    year: &ContractRefV1,
    merge: &ContractRefV1,
    objects: &SourceObjects,
    evidence: &RosterSourceEvidenceV1,
) -> Result<serde_json::Value> {
    Ok(serde_json::json!({
        "school": parents.school_row.id,
        "program": parents.program_row.id,
        "school_source": parents.owner,
        "program_source": parents.program_subject,
        "sport": parents.program_row.sport,
        "category": parents.program_row.gender,
        "school_year": parents.program_row.school_year,
        "year_convention": serde_json::to_value(year).context("encoding the year convention")?,
        "projection_merge_contract": serde_json::to_value(merge).context("encoding the merge contract")?,
        "school_projection": object(&objects.school),
        "program_projection": object(&objects.program),
        "provider_packet": {"digest": sha256_hex(&objects.packet), "bytes": objects.packet.len()},
        "physical_projection_inventory": object(&objects.inventory),
        "store": serde_json::to_value(evidence.store_identity()).context("encoding the store identity")?,
        "snapshot_revision": evidence.snapshot_sequence()
    }))
}

fn assemble_supplied(
    scope: &[u8],
    year: &[u8],
    merge: &[u8],
    objects: SourceObjects,
) -> Result<RosterSourceReviewObjectsV1> {
    Ok(RosterSourceReviewObjectsV1 {
        objects: vec![
            objects.canonical,
            objects.schema,
            objects.configuration,
            objects.dependencies,
            objects.body,
            objects.metadata,
            objects.interpretation,
            objects.packet,
            objects.school,
            objects.program,
            objects.inventory,
            objects.admission,
            objects.verification,
        ],
        scope_policy: scope.to_vec(),
        year_convention: year.to_vec(),
        projection_merge_contract: merge.to_vec(),
    })
}

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
    let objects = SourceObjects {
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
    };
    let value = serde_json::json!({
        "version": 1,
        "scope": scope_value(
            &parents,
            &contracts.scope_reference().context("reading the scope reference")?,
        )?,
        "parser": parser_value(&claim)?,
        "capture": capture_value(
            &objects,
            &occurrence,
            &capture_acquired,
            &observed_day,
            requested_url,
        )?,
        "binding": binding_value(
            &parents,
            &contracts.year_reference().context("reading the year reference")?,
            &contracts
                .projection_merge_reference()
                .context("reading the merge reference")?,
            &objects,
            evidence,
        )?,
    });
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

#[cfg(test)]
mod tests;
