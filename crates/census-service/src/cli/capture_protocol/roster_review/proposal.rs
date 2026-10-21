use anyhow::{ensure, Context, Result};
use census_crawl::participation_evidence::provider_school::{
    ProviderSchoolPacketRefV1, RetainedProviderSchoolPacketV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{
    CanonicalSchool, CanonicalTeam, ContractRefV1, SourceIdentity, SourceNamespace, SourceSubject,
};
use census_domain::roster_affiliation_authority::{
    LocallyCheckedRosterProposalV1, RosterParserClaimV1,
};
use census_store::{
    RetainedCaptureRootReadV1, RetainedCaptureRootSchemaV1, RosterIssuanceInspectionV1,
    RosterSourceEvidenceV1, RosterSourceReviewObjectsV1, Store,
};
use sha2::Digest;

mod derivation;

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
    grant: &mut AttachmentReplayGrantV1,
) -> Result<RetainedCaptureRootReadV1> {
    let result = (|| {
        let view = store.snapshot();
        let raw = view
            .read_retained_capture_root_v1(
                RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
                reference.as_str(),
                grant,
            )
            .context("reading the retained Raw packet root")?;
        let owned = view
            .read_retained_capture_root_v1(
                RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
                reference.as_str(),
                grant,
            )
            .context("reading the retained observed packet root")?;
        match (raw, owned) {
            (Some(root), None) | (None, Some(root)) => Ok(root),
            (None, None) => anyhow::bail!("retained packet missing"),
            (Some(_), Some(_)) => anyhow::bail!("retained packet has multiple root schemas"),
        }
    })();
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn packet(
    held: &RetainedCaptureRootReadV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<RetainedProviderSchoolPacketV1> {
    decode_packet(
        held.root().schema,
        held.payload().get().as_bytes(),
        &held.root().key,
        grant,
    )
}

fn decode_packet(
    schema: RetainedCaptureRootSchemaV1,
    raw: &[u8],
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<RetainedProviderSchoolPacketV1> {
    let result = (|| {
        grant.ensure_available()?;
        if schema == RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1 {
            grant.charge_bytes(
                raw.len()
                    .checked_mul(3)
                    .ok_or(census_store::StoreError::CounterOverflow)?,
            )?;
            grant.charge_steps(64)?;
        }
        let packet = RetainedProviderSchoolPacketV1::from_root(schema, raw)
            .context("decoding the retained packet")?;
        ensure!(
            packet
                .reference()
                .context("referencing the retained packet")?
                .as_str()
                == key,
            "retained packet key differs"
        );
        Ok(packet)
    })();
    if result.is_err() {
        grant.poison();
    }
    result
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
    packet: &RetainedProviderSchoolPacketV1,
    inspection: &mut RosterIssuanceInspectionV1<'_>,
) -> Result<(LocallyCheckedRosterProposalV1, RosterSourceReviewObjectsV1)> {
    derivation::derive(packet, inspection)
}

#[cfg(test)]
mod tests;
