use super::super::fixture::{Expected, Fixture};
use super::super::TestResult;
use crate::export::{ExportDataset, ParticipationOutcome};
use crate::workbook::publication;
use serde_json::{to_value, Value};
use std::path::{Path, PathBuf};

const ROW0_DIGEST: &str = "bc7c7f83bcfc715c9c3343e09cf45f82bee7651ac86838f3427e1097de010b3d";

pub(super) fn require_publication(
    fixture: &Fixture,
    dataset: &ExportDataset,
    root: &Path,
) -> TestResult<PathBuf> {
    require_projected(dataset, accepted_source(fixture)?)?;
    let workbook = super::super::publish(fixture, dataset, root)?;
    let verified = publication::verify_published(&workbook)?;
    check!(eq; verified.workbook.mapped_athletes, 0);
    check!(eq; publication::current_workbook(root)?, workbook);
    let generation = workbook.parent().ok_or("published generation")?;
    require_manifest(generation, dataset, &verified.generation_digest)?;
    require_exclusions(fixture, dataset, generation)?;
    require_affiliation(fixture, dataset, generation)?;
    require_audit(fixture, dataset, generation)?;
    Ok(workbook)
}

pub(super) fn reopen(workbook: &Path, dataset: &ExportDataset) -> TestResult<ExportDataset> {
    let profile = dataset.selected_authority()?.profile();
    let [cohort] = profile.input().graduation_years.as_slice() else {
        return Err("one genuine selected cohort".into());
    };
    let bests = super::super::bests_options(dataset)?;
    let verified = publication::verify_published(workbook)?;
    let opened = publication::open_publication(workbook, &dataset.lineage, bests.scope, *cohort)?;
    check!(eq; opened.generation_digest, verified.generation_digest);
    check!(eq; opened.workbook.mapped_athletes, 0);
    check!(eq; opened.dataset.lineage, dataset.lineage);
    check!(eq; to_value(&opened.dataset)?, to_value(dataset)?);
    check!(eq; opened.dataset.verified_participation()?.documentary_rows().count(), 2);
    let [retained] = dataset.athletes.as_slice() else {
        return Err("one genuine retained athlete".into());
    };
    let before = dataset.verified_current_affiliations()?;
    let after = opened.dataset.verified_current_affiliations()?;
    check!(eq; after.resolution(&retained.id), before.resolution(&retained.id));
    let before_audit = before
        .audit(&retained.id)
        .ok_or("original affiliation audit")?;
    let after_audit = after
        .audit(&retained.id)
        .ok_or("reopened affiliation audit")?;
    check!(eq; after_audit.references(), before_audit.references());
    check!(eq; after_audit.unavailable(), before_audit.unavailable());
    Ok(opened.dataset)
}

pub(super) fn accepted_source(fixture: &Fixture) -> TestResult<&Expected> {
    fixture
        .expected
        .iter()
        .find(|item| item.references.iter().any(|digest| digest == ROW0_DIGEST))
        .ok_or_else(|| "genuine expected original result row absent".into())
}

fn require_projected(dataset: &ExportDataset, accepted: &Expected) -> TestResult {
    check!(eq; dataset.performances.len(), 1);
    dataset
        .performances
        .iter()
        .find(|row| row.id == accepted.performance)
        .ok_or("genuine performance projection absent")?;
    let view = dataset.verified_participation()?;
    let assessment = view
        .assessment(&accepted.performance)
        .ok_or("participation assessment absent for accepted row")?;
    check!(eq; assessment.outcome(), ParticipationOutcome::Eligible);
    check!(assessment.reasons().is_empty());
    check!(assessment.diagnostics().is_empty());
    check!(!assessment.evidence_refs().is_empty());
    check!(accepted
        .references
        .iter()
        .chain(&accepted.semantic_references)
        .all(|reference| assessment.evidence_refs().contains(reference)));
    check!(view.documentary(&accepted.performance).is_none());
    Ok(())
}

fn require_manifest(generation: &Path, dataset: &ExportDataset, digest: &str) -> TestResult {
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    check!(eq; manifest.pointer("/normalized_csv_rows/participation_exclusions.csv").and_then(Value::as_u64), Some(2));
    check!(eq; manifest.pointer("/normalized_csv_rows/current_affiliations.csv").and_then(Value::as_u64), Some(1));
    check!(eq; manifest.pointer("/normalized_csv_rows/best_marks.csv").and_then(Value::as_u64), Some(0));
    check!(eq; manifest.pointer("/normalized_csv_rows/athletes.csv").and_then(Value::as_u64), Some(0));
    check!(eq; manifest.get("generation_digest").and_then(Value::as_str).ok_or("manifest generation digest")?, digest);
    check!(eq; manifest.get("lineage").ok_or("manifest lineage")?, &to_value(&dataset.lineage)?);
    Ok(())
}

fn require_exclusions(fixture: &Fixture, dataset: &ExportDataset, generation: &Path) -> TestResult {
    let view = dataset.verified_participation()?;
    check!(view.unsupported_documentary().is_empty());
    check!(eq; view.documentary_rows().count(), 2);
    let accepted = &accepted_source(fixture)?.performance;
    let excluded: Vec<_> = fixture
        .expected
        .iter()
        .filter(|item| &item.performance != accepted)
        .collect();
    check!(eq; excluded.len(), 2);
    let mut reader = csv::Reader::from_path(generation.join("participation_exclusions.csv"))?;
    check!(eq; reader.headers()?.iter().collect::<Vec<_>>(), vec!["retained_candidate_id","performance_id","event_id","disposition","reasons","diagnostics","evidence_refs","input_generation"]);
    let rows = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(eq; rows.len(), excluded.len());
    rows.iter()
        .zip(excluded)
        .try_for_each(|(row, source)| require_exclusion(row, source, view))
}

fn require_exclusion(
    row: &csv::StringRecord,
    source: &Expected,
    view: &crate::export::VerifiedParticipationView,
) -> TestResult {
    let documentary = view
        .documentary(&source.performance)
        .ok_or("frozen documentary exclusion absent")?;
    let assessment = documentary.assessment();
    check!(eq; row.len(), 8);
    check!(eq; row.get(0), Some(source.athlete.as_str()));
    check!(eq; row.get(1), Some(source.performance.as_str()));
    check!(eq; row.get(2), Some(source.event.as_str()));
    check!(eq; documentary.athlete_id(), &source.athlete);
    check!(eq; documentary.event_id(), &source.event);
    check!(eq; assessment.outcome(), ParticipationOutcome::Unresolved);
    check!(eq; row.get(3), Some("unresolved"));
    check!(eq; row.get(4).ok_or("participation reasons")?, serde_json::to_string(&assessment.reasons().iter().collect::<Vec<_>>())?);
    check!(eq; row.get(5).ok_or("participation diagnostics")?, serde_json::to_string(assessment.diagnostics())?);
    let references = super::super::expected_references(source);
    check!(eq; assessment.evidence_refs(), references.as_slice());
    check!(eq; row.get(6).ok_or("participation references")?, serde_json::to_string(&references)?);
    check!(eq; row.get(7), Some(view.input_generation()));
    Ok(())
}

fn require_affiliation(
    fixture: &Fixture,
    dataset: &ExportDataset,
    generation: &Path,
) -> TestResult {
    let [retained] = dataset.athletes.as_slice() else {
        return Err("one genuine retained athlete".into());
    };
    check!(eq; retained.id, fixture.expected.first().ok_or("genuine expected athlete")?.athlete);
    let affiliations = dataset.verified_current_affiliations()?;
    check!(matches!(
        affiliations.resolution(&retained.id),
        crate::export::CurrentAffiliationResolution::Unknown
    ));
    let audit = affiliations
        .audit(&retained.id)
        .ok_or("genuine current affiliation audit absent")?;
    check!(!audit.unavailable().is_empty());
    let mut reader = csv::Reader::from_path(generation.join("current_affiliations.csv"))?;
    check!(eq; reader.headers()?.iter().collect::<Vec<_>>(), vec!["retained_candidate_id","current_school_id","resolution_status","evidence_refs","unavailable_reasons","profile_id","as_of","input_generation"]);
    let rows = reader.records().collect::<Result<Vec<_>, _>>()?;
    let [row] = rows.as_slice() else {
        return Err("one genuine retained affiliation audit row".into());
    };
    check!(eq; row.len(), 8);
    check!(eq; row.get(0), Some(retained.id.as_str()));
    check!(eq; row.get(1), Some(""));
    check!(eq; row.get(2), Some("current_affiliation_unknown"));
    check!(eq; row.get(3).ok_or("affiliation references")?, serde_json::to_string(audit.references())?);
    check!(eq; row.get(4).ok_or("affiliation unavailable reasons")?, serde_json::to_string(audit.unavailable())?);
    check!(eq; row.get(5), Some(affiliations.profile_id().as_str()));
    check!(eq; row.get(6), Some(affiliations.as_of().as_str()));
    check!(eq; row.get(7), Some(affiliations.input_generation()));
    Ok(())
}

fn require_audit(fixture: &Fixture, dataset: &ExportDataset, generation: &Path) -> TestResult {
    let bests = super::super::bests_options(dataset)?;
    let derivation =
        crate::report::Derivation::of(dataset, bests.scope, bests.grad_year, &bests.jurisdictions)?;
    let outcome = crate::bests::build_with_exclusions(dataset, &bests)?;
    check!(outcome.selections.is_empty());
    check!(derivation.athletes().is_empty());
    let audit: Value = serde_json::from_slice(&std::fs::read(generation.join("audit.json"))?)?;
    check!(eq; audit.get("input_generation").and_then(Value::as_str).ok_or("audit generation")?, dataset.lineage.input_generation.as_str());
    check!(eq; audit.get("scope").and_then(Value::as_str).ok_or("audit scope")?, bests.scope.as_str());
    check!(eq; audit.get("grad_year").and_then(Value::as_i64), bests.grad_year.map(i64::from));
    check!(eq; audit.get("school_year").and_then(Value::as_i64), Some(i64::from(fixture.profile.input().contact_school_year.get())));
    check!(eq; audit.get("athletes").ok_or("audit athletes")?, &to_value(Vec::<String>::new())?);
    let performance_ids: Vec<_> = derivation
        .performances()
        .iter()
        .map(|row| row.id.as_str())
        .collect();
    check!(eq; audit.get("performances").ok_or("audit performances")?, &to_value(performance_ids)?);
    ["best_marks.csv", "athletes.csv", "recruiting.csv"]
        .into_iter()
        .try_for_each(|name| -> TestResult {
            let mut empty = csv::Reader::from_path(generation.join(name))?;
            check!(empty.records().next().transpose()?.is_none());
            Ok(())
        })
}
