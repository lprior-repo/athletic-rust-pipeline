use super::{MergePass, Row, RowKey};
use anyhow::{bail, Result};
use census_domain::model::{verify_contact_proof, ContactClaimEvidence, RawContactRow};
use census_service::coachverify;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

pub(super) fn read_fragment_claims(
    csv_files: &[PathBuf],
) -> Result<BTreeMap<PathBuf, Vec<ContactClaimEvidence>>> {
    let mut claims_by_file: BTreeMap<PathBuf, Vec<ContactClaimEvidence>> = BTreeMap::new();
    let mut bytes = 0u64;
    for path in csv_files {
        let size = std::fs::metadata(coachverify::evidence_path(path))?.len();
        bytes = bytes
            .checked_add(size)
            .ok_or_else(|| anyhow::anyhow!("merged evidence byte count overflow"))?;
        if bytes > 256 * 1024 * 1024 {
            bail!("merged evidence bytes {bytes} exceeds 268435456");
        }
    }
    for path in csv_files {
        let sidecar = coachverify::evidence_path(path);
        if !sidecar.is_file() {
            bail!(
                "fragment {} has no evidence sidecar {}",
                path.display(),
                sidecar.display()
            );
        }
        let claims = coachverify::read_evidence_jsonl(&sidecar)?;
        claims_by_file.insert(path.clone(), claims);
    }
    Ok(claims_by_file)
}

fn covering_proof(
    row: &Row,
    claims: &coachverify::ContactEvidenceIndex<'_>,
) -> Result<Result<Vec<ContactClaimEvidence>, census_domain::model::ContactProofError>> {
    let raw = raw_contact_row(row);
    let covering = claims.claims_for_row(&raw)?;
    Ok(verify_contact_proof(&raw, &covering, row.verified_proof_digest.trim()).map(|_| covering))
}

pub(super) fn verify_kept_proofs(
    pass: &mut MergePass,
    claims_by_file: &BTreeMap<PathBuf, Vec<ContactClaimEvidence>>,
) -> Result<Vec<ContactClaimEvidence>> {
    check_kept_budget(pass)?;
    let mut indexes = HashMap::new();
    indexes.try_reserve(claims_by_file.len())?;
    for (path, claims) in claims_by_file {
        indexes.insert(path, coachverify::ContactEvidenceIndex::build(claims)?);
    }
    let mut evidence: Vec<ContactClaimEvidence> = Vec::new();
    let mut unproven: Vec<(RowKey, String)> = Vec::new();
    unproven.try_reserve(pass.kept.len())?;
    let empty = coachverify::ContactEvidenceIndex::build(&[])?;
    for (key, row) in &pass.kept {
        let claims = match pass
            .origins
            .get(key)
            .and_then(|origin| indexes.get(&origin.file))
        {
            Some(claims) => claims,
            None => &empty,
        };
        match covering_proof(row, claims)? {
            Ok(covering) => {
                let count = evidence
                    .len()
                    .checked_add(covering.len())
                    .ok_or_else(|| anyhow::anyhow!("merged evidence count overflow"))?;
                if count > 1_000_000 {
                    bail!("merged evidence claims {count} exceeds 1000000");
                }
                evidence.try_reserve(covering.len())?;
                evidence.extend(covering);
            }
            Err(reason) => unproven.push((key.clone(), reason.to_string())),
        }
    }
    for (key, reason) in unproven {
        reject_unproven(pass, &key, &reason);
    }
    Ok(evidence)
}

fn check_kept_budget(pass: &MergePass) -> Result<()> {
    if pass.kept.len() > 1_000_000 {
        bail!("merged contact rows {} exceeds 1000000", pass.kept.len());
    }
    let mut bytes = 0usize;
    let mut work = 0usize;
    for row in pass.kept.values() {
        let record = row
            .to_fields()
            .into_iter()
            .fold(0usize, |count, field| count.saturating_add(field.len()));
        if record > 1_048_576 {
            bail!("merged contact record bytes {record} exceeds 1048576");
        }
        bytes = bytes.saturating_add(record);
        if bytes > 268_435_456 {
            bail!("merged contact bytes {bytes} exceeds 268435456");
        }
        work = work.saturating_add(row.source_url.split_whitespace().count().saturating_mul(6));
        if work > 8_000_000 {
            bail!("merged contact queries {work} exceeds 8000000");
        }
    }
    Ok(())
}

fn reject_unproven(pass: &mut MergePass, key: &RowKey, reason: &str) {
    let origin = pass.origins.remove(key);
    if let Some(row) = pass.kept.remove(key) {
        let counts = pass.per_state.entry(row.state.clone()).or_default();
        counts.kept = counts.kept.saturating_sub(1);
        counts.rejected = counts.rejected.saturating_add(1);
        pass.rejects.push(super::Rejection {
            state: row.state.clone(),
            line_no: origin.map_or(0, |origin| origin.line_no),
            reason: format!("proof not verified: {reason}"),
            school: row.school.clone(),
            coach_name: row.coach_name.clone(),
            role: row.role.clone(),
            source_url: row.source_url.clone(),
        });
    }
}

pub(super) fn raw_contact_row(row: &Row) -> RawContactRow {
    RawContactRow {
        school: row.school.clone(),
        city: row.city.clone(),
        state: row.state.clone(),
        sport: row.sport.clone(),
        role: row.role.clone(),
        coach_name: row.coach_name.clone(),
        public_professional_email: row.public_professional_email.clone(),
        ad_name: row.ad_name.clone(),
        ad_email: row.ad_email.clone(),
        source_urls: row
            .source_url
            .split_whitespace()
            .map(str::to_string)
            .collect(),
        last_observed: row.last_observed.clone(),
    }
}
