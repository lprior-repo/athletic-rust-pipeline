use super::{ExportDataset, TestResult, Value};
use crate::export::public_identity::output::ResolutionAudit;
use serde_json::json;

pub(super) fn capture(dataset: &ExportDataset) -> TestResult<Value> {
    let participation = dataset.verified_participation()?;
    let affiliations = dataset.verified_current_affiliations()?;
    let identities = dataset.verified_public_identities()?;
    let performances = dataset
        .performances
        .iter()
        .map(|performance| {
            let row = participation
                .assessment(&performance.id)
                .ok_or("participation row")?;
            Ok(json!({
                "performance": performance.id,
                "outcome": format!("{:?}", row.outcome()),
                "reasons": row.reasons().iter().collect::<Vec<_>>(),
                "evidence": row.evidence_refs(), "diagnostics": row.diagnostics(),
            }))
        })
        .collect::<TestResult<Vec<_>>>()?;
    let athletes = dataset.athletes.iter().map(|athlete| {
        let resolution = affiliations.resolution(&athlete.id);
        let audit = affiliations.audit(&athlete.id);
        let group = identities.group(athlete.id.as_str()).ok_or("identity group")?;
        Ok(json!({
            "athlete": athlete.id, "affiliation_status": resolution.status(),
            "school": resolution.school(), "affiliation_evidence": resolution.evidence_refs(),
            "audit_references": audit.map(|value| value.references()), "audit_unavailable": audit.map(|value| value.unavailable()),
            "public_id": identities.public_id(athlete.id.as_str()),
            "candidate_resolution": identities.candidate_resolution(athlete.id.as_str()).map(ResolutionAudit),
            "canonical_candidate": group.canonical_candidate(),
            "group_members": group.members().map(|(candidate, resolution)| (candidate, ResolutionAudit(resolution))).collect::<Vec<_>>(),
        }))
    }).collect::<TestResult<Vec<_>>>()?;
    let public = identities
        .public_ids()
        .map(|id| {
            let resolution = identities.resolve_public(id)?;
            Ok(
                json!({ "id": id, "resolution": ResolutionAudit(&resolution),
                "corrections": identities.correction_history(id) }),
            )
        })
        .collect::<TestResult<Vec<_>>>()?;
    let affected = (1..=identities.head().sequence).map(|sequence| {
        json!({ "sequence": sequence, "records": identities.affected_record_keys(sequence) })
    }).collect::<Vec<_>>();
    Ok(json!({
        "data": dataset,
        "participation_generation": participation.input_generation(),
        "participation_profile": participation.profile_id(),
        "participation_rule": participation.participation_rule(), "performances": performances,
        "affiliation_generation": affiliations.input_generation(),
        "affiliation_profile": affiliations.profile_id(), "as_of": affiliations.as_of(),
        "athletes": athletes, "identity_generation": identities.input_generation(),
        "identity_store": identities.store_identity(), "identity_sequence": identities.snapshot_sequence(),
        "identity_head": identities.head(), "public": public, "affected": affected,
    }))
}
