use super::*;
use census_domain::model::{EvidenceMethod, SourceNamespace};
use serde_json::json;

#[test]
fn genuine_tfrrs_matches_after_only_valid_review_references_change() -> TestResult {
    let original = fixture::tfrrs()?;
    let before = serde_json::to_vec(&original)?;
    let reviewed = fixture::note(&original, |note| {
        let target = note
            .get_mut("proposed_target")
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("retained target")?;
        target.insert(
            "proposed_attachment_reference".into(),
            json!(fixture::SCHOOL),
        );
        target.insert(
            "proposed_school_binding_reference".into(),
            json!(fixture::ATTACHMENT),
        );
        Ok(())
    })?;
    check!(matches_physical_observation(&original, &reviewed)?);
    check!(matches_physical_observation(&reviewed, &original)?);
    check!(eq; serde_json::to_vec(&original)?, before);
    let adjustment = original.adjustment.as_ref().ok_or("retained conversion")?;
    check!(eq; adjustment.original_mark, Some(Mark::TimeSeconds(ExactSeconds::parse("562.21")?)));
    check!(eq; original.source_athlete.as_ref().ok_or("native athlete")?.namespace, SourceNamespace::TfrrsAthlete);
    Ok(())
}

#[test]
fn physical_adjustment_and_native_identity_changes_cannot_match_tfrrs() -> TestResult {
    let original = fixture::tfrrs()?;
    let mut changed = original.clone();
    changed
        .adjustment
        .as_mut()
        .ok_or("retained conversion")?
        .original_mark = Some(Mark::TimeSeconds(ExactSeconds::parse("562.22")?));
    fixture::refused(&changed, &original)?;
    let mut changed = original.clone();
    changed.adjustment = None;
    fixture::refused(&changed, &original)?;
    let mut changed = original.clone();
    changed.mark = Mark::TimeSeconds(ExactSeconds::parse("555.74")?);
    fixture::refused(&changed, &original)?;
    let mut changed = original.clone();
    changed.source_athlete.as_mut().ok_or("native athlete")?.id = "8650212".into();
    fixture::refused(&changed, &original)?;
    let mut changed = original.clone();
    changed.date = "2026-03-20".into();
    fixture::refused(&changed, &original)?;
    Ok(())
}

#[test]
fn cross_source_foreign_provenance_and_empty_tfrrs_evidence_never_match() -> TestResult {
    let original = fixture::tfrrs()?;
    for index in 0..5 {
        let mut changed = original.clone();
        let evidence = changed.evidence.first_mut().ok_or("retained evidence")?;
        match index {
            0 => evidence.source.id = "milesplit_html_physical_projection_v1".into(),
            1 => evidence.source.url = Some("https://foreign.invalid/result".into()),
            2 => evidence.method = EvidenceMethod::Fetched,
            3 => evidence.observed_on = "2026-01-01T00:00:00Z".into(),
            _ => changed.evidence.clear(),
        }
        fixture::refused(&changed, &original)?;
    }
    let mut unknown = original.clone();
    unknown
        .evidence
        .first_mut()
        .ok_or("retained evidence")?
        .source
        .id = "unknown_source".into();
    fixture::refused(&unknown, &unknown)?;
    let mut forged_ms = original.clone();
    forged_ms
        .evidence
        .first_mut()
        .ok_or("retained evidence")?
        .source
        .id = "milesplit_html_physical_projection_v1".into();
    fixture::refused(&forged_ms, &forged_ms)?;
    Ok(())
}

#[test]
fn a_foreign_tfrrs_row_in_either_evidence_collection_refuses() -> TestResult {
    let original = fixture::tfrrs()?;
    let source = CheckedHtmlResultSourceV1::Tfrrs;
    let capture = common_fixture::capture(source)?;
    let document =
        read_retained_source_result_rows(&capture, &common_fixture::locators(source), source)?;
    let (_, foreign) = fixture::project(&document, 1)?;
    let mut changed = original.clone();
    changed
        .evidence
        .extend(foreign.proposed_performance().evidence.clone());
    fixture::refused(&changed, &original)?;
    Ok(())
}
