use super::*;

#[path = "genuine/fixture.rs"]
mod original;

#[test]
fn genuine_partial_roster_controlled_review_selects_abbott_michigan_and_replays_publication_offline(
) -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source = original::install(&fixture)?;
    fixture.accept(&source.athlete.id)?;
    let approved = issuance::review_revision(
        &fixture,
        &source,
        "genuine-abbott-controlled-row-local-review",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let expected = Expected {
        athlete: source.athlete.id.clone(),
        current_school: source.school.id.clone(),
        school_name: "Detroit Catholic Central".into(),
        case: Case::Resolved,
    };
    let objects = tempfile::tempdir()?;
    let archive = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let dataset = capture(&fixture, objects.path())?;
    let before = witness(&dataset, &expected)?;
    let digest = dataset.archive_digest()?;
    let lineage = dataset.lineage.clone();
    let frozen = archive.path().join("genuine-abbott-controlled-review.json");
    dataset.save_frozen(&frozen)?;
    let inventory = retained_inventory(&frozen, &source, &approved)?;
    let workbook = publish(&fixture, &dataset, &output.path().join("publication"))?;
    let rows = artifacts::readback(&workbook, &dataset, &expected)?;
    let generation = workbook.parent().ok_or("genuine publication generation")?;
    check!(std::fs::read(generation.join("best-results-co2027.jsonl"))?.is_empty());
    let verified = published::verify_published(&workbook)?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    drop(dataset);
    remove_sources(fixture, objects)?;
    let reopened = ExportDataset::reopen_frozen_with_digest(&frozen, &digest)?;
    check!(eq; witness(&reopened, &expected)?, before);
    check!(eq; reopened.archive_digest()?, digest);
    check!(eq; retained_inventory(&frozen, &source, &approved)?, inventory);
    let publication =
        published::open_publication(&workbook, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; witness(&publication.dataset, &expected)?, before);
    check!(eq; artifacts::readback(&workbook, &publication.dataset, &expected)?, rows);
    check!(eq; published::verify_published(&workbook)?.generation_digest, verified.generation_digest);
    Ok(())
}

fn retained_inventory(
    frozen: &Path,
    source: &fixture::Capture,
    approved: &[u8],
) -> TestResult<Value> {
    let wire = inspect_wire(frozen)?;
    let directory = frozen.with_file_name("genuine-abbott-controlled-review.json.proofs");
    let closure_digest = wire
        .pointer("/selected_proofs/affiliation/archives/digest")
        .and_then(Value::as_str)
        .ok_or("genuine roster closure")?;
    let closure: Value = serde_json::from_slice(&std::fs::read(directory.join(closure_digest))?)?;
    let reference: census_store::RosterFrozenReviewedClaimsReferenceV1 = serde_json::from_value(
        closure
            .get("reviewed")
            .ok_or("genuine reviewed reference")?
            .clone(),
    )?;
    let raw = std::fs::read(directory.join(reference.digest()))?;
    let mut grant = AttachmentReplayGrantV1::new();
    let replay =
        census_store::replay_frozen_roster_reviewed_claims_v1(&raw, &reference, &mut grant)?;
    let claims = replay.claims(&mut grant)?;
    let [claim] = claims else {
        return Err("one genuine reviewed capture required".into());
    };
    check!(eq; claim.approval_bytes(&mut grant)?, approved);
    let proposal = claim.proposal(&mut grant)?;
    let interpretation = claim.object(
        proposal.capture().interpretation().digest().as_str(),
        &mut grant,
    )?;
    let value: Value = serde_json::from_slice(interpretation)?;
    check!(eq; value.get("schema").and_then(Value::as_str), Some("roster-retained-capture-interpretation-v3"));
    check!(eq; value.pointer("/semantic_inventory/disposition").and_then(Value::as_str), Some("partial"));
    let rows = value
        .pointer("/semantic_inventory/rows")
        .and_then(Value::as_array)
        .ok_or("complete disposition inventory")?;
    check!(eq; rows.len(), 164);
    check!(eq; rows.iter().filter(|row| row.pointer("/outcome/state").and_then(Value::as_str) == Some("admitted")).count(), 146);
    check!(eq; rows.iter().filter(|row| row.pointer("/outcome/state").and_then(Value::as_str) == Some("rejected")).count(), 18);
    let expected_reason =
        serde_json::to_value(census_crawl::milesplit::RosterRejectionKind::InvalidGraduationYear)?;
    for rejected in rows
        .iter()
        .filter(|row| row.pointer("/outcome/state").and_then(Value::as_str) == Some("rejected"))
    {
        check!(eq; rejected.pointer("/outcome/reason"), Some(&expected_reason));
    }
    let abbott = rows
        .iter()
        .find(|row| row.get("provider_id").and_then(Value::as_str) == Some("13749843"))
        .ok_or("retained Abbott disposition")?;
    check!(eq; abbott.pointer("/outcome/state").and_then(Value::as_str), Some("admitted"));
    let original_packet = claim.object(
        proposal.binding().provider_packet().digest().as_str(),
        &mut grant,
    )?;
    check!(eq; original_packet, source.packet.encode()?.as_slice());
    for (ordinal, row) in rows.iter().enumerate() {
        check!(eq; row.pointer("/row/ordinal").and_then(Value::as_u64), Some(u64::try_from(ordinal)?));
    }
    Ok(value)
}

#[test]
fn forged_candidate_for_rejected_original_row_cannot_obtain_current_affiliation_or_export(
) -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source = original::install(&fixture)?;
    let forged = original::forged_candidate_for_rejected_original_row(&fixture, &source)?;
    fixture.accept(&source.athlete.id)?;
    fixture.accept(&forged.id)?;
    let approved = issuance::review_revision(
        &fixture,
        &source,
        "genuine-rejected-target-controlled-review",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let expected = Expected {
        athlete: source.athlete.id.clone(),
        current_school: source.school.id.clone(),
        school_name: "Detroit Catholic Central".into(),
        case: Case::Resolved,
    };
    let objects = tempfile::tempdir()?;
    let archive = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let dataset = capture(&fixture, objects.path())?;
    witness(&dataset, &expected)?;
    rejected_target(&dataset, &forged.id)?;
    let digest = dataset.archive_digest()?;
    let frozen = archive.path().join("genuine-abbott-controlled-review.json");
    dataset.save_frozen(&frozen)?;
    retained_inventory(&frozen, &source, &approved)?;
    let workbook = publish(&fixture, &dataset, &output.path().join("publication"))?;
    let rows = artifacts::readback(&workbook, &dataset, &expected)?;
    let affiliations = rows
        .get("affiliations")
        .and_then(Value::as_array)
        .ok_or("affiliation export audit")?;
    let rejected = affiliations
        .iter()
        .find(|row| {
            row.get("retained_candidate_id").and_then(Value::as_str) == Some(forged.id.as_str())
        })
        .ok_or("rejected target audit row")?;
    check!(eq; rejected.get("resolution_status").and_then(Value::as_str), Some("current_affiliation_unknown"));
    check!(eq; rejected.get("current_school_id").and_then(Value::as_str), Some(""));
    check!(eq; rejected.get("unavailable_reasons").and_then(Value::as_str), Some("[\"archive_unavailable\"]"));
    check!(eq; published::verify_published(&workbook)?.workbook.mapped_athletes, 1);
    drop(dataset);
    remove_sources(fixture, objects)?;
    let reopened = ExportDataset::reopen_frozen_with_digest(&frozen, &digest)?;
    witness(&reopened, &expected)?;
    rejected_target(&reopened, &forged.id)?;
    check!(eq; artifacts::readback(&workbook, &reopened, &expected)?, rows);
    Ok(())
}

fn rejected_target(dataset: &ExportDataset, target: &AthleteId) -> TestResult {
    check!(dataset
        .verified_public_identities()?
        .public_id(target.as_str())
        .is_some());
    let affiliations = dataset.verified_current_affiliations()?;
    let resolution = affiliations.resolution(target);
    check!(eq; resolution.status(), "current_affiliation_unknown");
    check!(resolution.school().is_none());
    check!(eq; affiliations.audit(target).ok_or("rejected target audit")?.unavailable(), &["archive_unavailable".to_string()]);
    let selected = crate::report::Derivation::selected(
        dataset,
        Scope::AllSources,
        Some(2027),
        &[UsJurisdiction::Michigan],
        affiliations,
    )?;
    check!(!selected
        .athletes()
        .iter()
        .any(|athlete| &athlete.id == target));
    check!(eq; selected.affiliation_exclusions().get(target), Some(&crate::report::CurrentAffiliationExclusion::Unresolved));
    Ok(())
}
