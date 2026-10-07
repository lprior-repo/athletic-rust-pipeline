use super::{MergePass, Row, RowKey};
use anyhow::{bail, Result};
use census_domain::model::{verify_contact_proof, ContactClaimEvidence, RawContactRow};
use census_service::coachverify;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub(super) fn read_fragment_claims(
    csv_files: &[PathBuf],
) -> Result<BTreeMap<PathBuf, Vec<ContactClaimEvidence>>> {
    let mut claims_by_file: BTreeMap<PathBuf, Vec<ContactClaimEvidence>> = BTreeMap::new();
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
    claims: &[ContactClaimEvidence],
) -> Result<Vec<ContactClaimEvidence>, String> {
    let raw = raw_contact_row(row);
    let covering = coachverify::claims_for_row(claims, &raw);
    verify_contact_proof(&raw, &covering, row.verified_proof_digest.trim())
        .map(|_| covering)
        .map_err(|error| error.to_string())
}

pub(super) fn verify_kept_proofs(
    pass: &mut MergePass,
    claims_by_file: &BTreeMap<PathBuf, Vec<ContactClaimEvidence>>,
) -> Vec<ContactClaimEvidence> {
    let mut evidence: Vec<ContactClaimEvidence> = Vec::new();
    let mut unproven: Vec<(RowKey, String)> = Vec::new();
    for (key, row) in &pass.kept {
        let claims = match pass
            .origins
            .get(key)
            .and_then(|origin| claims_by_file.get(&origin.file))
        {
            Some(claims) => claims.as_slice(),
            None => &[],
        };
        match covering_proof(row, claims) {
            Ok(covering) => evidence.extend(covering),
            Err(reason) => unproven.push((key.clone(), reason)),
        }
    }
    for (key, reason) in unproven {
        reject_unproven(pass, &key, &reason);
    }
    evidence
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
