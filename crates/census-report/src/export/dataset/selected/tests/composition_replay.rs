use super::{witness, ExportDataset, Fixture, ParticipationOutcome, Table, TestResult};
use crate::export::ParticipationDiagnostic;
use census_domain::model::{SourceIdentity, SourceNamespace};
use census_domain::participation_candidates::ParticipationReason;

#[tokio::test]
async fn retained_non_school_exclusions_survive_roster_composition_and_offline_replay() -> TestResult
{
    for (division, reason) in [
        ("Collegiate", ParticipationReason::CollegiateParticipation),
        (
            "Middle School",
            ParticipationReason::MiddleSchoolParticipation,
        ),
        (
            "Post Graduation",
            ParticipationReason::PostGraduationParticipation,
        ),
    ] {
        let fixture = Fixture::non_school(division).await?;
        let objects = super::temporary()?;
        let dataset = fixture.capture(objects.path())?;
        let assessment = dataset
            .verified_participation()?
            .assessment(&fixture.performance.id)
            .ok_or("non-school assessment")?;
        check!(eq; assessment.outcome(), ParticipationOutcome::Excluded);
        check!(eq; assessment.reasons().iter().collect::<Vec<_>>(), vec![ParticipationReason::SummerBoundaryUnresolved, reason]);
        check!(assessment.diagnostics().is_empty());
        check!(!assessment.evidence_refs().is_empty());
        check!(dataset
            .verified_public_identities()?
            .public_id(fixture.roster_athlete.as_str())
            .is_some());
        let bests = bests(&dataset)?;
        check!(bests.selections.is_empty());
        check!(eq; bests.participation_exclusions.len(), 1);
        let exclusion = bests
            .participation_exclusions
            .first()
            .ok_or("retained exclusion")?;
        check!(eq; exclusion.performance_id(), &fixture.performance.id);
        check!(eq; exclusion.outcome(), crate::bests::ParticipationExclusionOutcome::Excluded);
        replay(fixture, objects, dataset)?;
    }
    Ok(())
}

#[tokio::test]
async fn appended_date_and_owner_conflicts_stay_unresolved_after_roster_composition_and_replay(
) -> TestResult {
    for (field, family) in [
        ("missing date", "Canonical id collision"),
        ("wrong date", "Canonical id collision"),
        ("missing owner", "Individual result context"),
        ("wrong owner", "Individual result context"),
    ] {
        let fixture = Fixture::composed(false).await?;
        let objects = super::temporary()?;
        let baseline = fixture.capture(objects.path())?;
        check!(eq; baseline.verified_participation()?.assessment(&fixture.performance.id)
            .ok_or("baseline participation")?.outcome(), ParticipationOutcome::Unresolved);
        let baseline_assessment = baseline
            .verified_participation()?
            .assessment(&fixture.performance.id)
            .ok_or("baseline assessment")?;
        check!(eq; baseline_assessment.reasons().iter().collect::<Vec<_>>(), vec![
            ParticipationReason::HighSchoolStatusMissing,
            ParticipationReason::SummerBoundaryUnresolved,
        ]);
        check!(baseline_assessment.diagnostics().is_empty());
        drop(baseline);
        let mut changed = fixture.performance.clone();
        match field {
            "missing date" => changed.date.clear(),
            "wrong date" => changed.date = "2026-03-28".into(),
            "missing owner" => changed.source_athlete = None,
            "wrong owner" => {
                changed.source_athlete = Some(SourceIdentity::new(
                    SourceNamespace::MilesplitAthlete,
                    "14222593",
                ))
            }
            _ => return Err("unknown synthetic mutation".into()),
        }
        fixture.store.append(Table::Performances, &changed)?;
        let fresh_objects = super::temporary()?;
        let dataset = fixture.capture(fresh_objects.path())?;
        let retained = dataset
            .performances
            .iter()
            .find(|performance| performance.id == fixture.performance.id)
            .ok_or("retained canonical performance")?;
        check!(eq; retained.date, fixture.performance.date, "{field}");
        check!(eq; retained.source_athlete, fixture.performance.source_athlete, "{field}");
        check!(eq; retained.evidence, fixture.performance.evidence, "{field}");
        check!(eq; retained.retained_conflicts.len(), 1, "{field}: {:?}", retained.retained_conflicts);
        let conflict = retained
            .retained_conflicts
            .first()
            .ok_or("retained conflict")?;
        check!(eq; conflict.family, family, "{field}");
        check!(eq; conflict.subject_id, fixture.performance.id.as_str(), "{field}");
        let assessment = dataset
            .verified_participation()?
            .assessment(&fixture.performance.id)
            .ok_or("refused participation")?;
        check!(eq; assessment.outcome(), ParticipationOutcome::Unresolved, "{field}: {:?}", assessment.diagnostics());
        check!(
            assessment
                .diagnostics()
                .contains(&ParticipationDiagnostic::CanonicalConflict),
            "{field}: conflicts {:?}; diagnostics {:?}, reasons {:?}",
            retained.retained_conflicts,
            assessment.diagnostics(),
            assessment.reasons()
        );
        check!(
            !assessment
                .diagnostics()
                .contains(&ParticipationDiagnostic::SourceUnavailable),
            "{field}: {:?}",
            assessment.diagnostics()
        );
        check!(
            !assessment
                .diagnostics()
                .contains(&ParticipationDiagnostic::SourceBindingMismatch),
            "{field}: original source binding was preserved; {:?}",
            assessment.diagnostics()
        );
        check!(assessment
            .evidence_refs()
            .iter()
            .any(|reference| reference.starts_with("acquisition/")));
        check!(assessment
            .evidence_refs()
            .iter()
            .any(|reference| reference.starts_with("metadata/")));
        let bests = bests(&dataset)?;
        check!(bests.selections.is_empty());
        check!(eq; bests.participation_exclusions.len(), 1);
        check!(eq; bests.participation_exclusions.first().ok_or("unresolved exclusion")?.outcome(),
            crate::bests::ParticipationExclusionOutcome::Unresolved);
        objects.close()?;
        replay(fixture, fresh_objects, dataset)?;
    }
    Ok(())
}

fn bests(dataset: &ExportDataset) -> TestResult<crate::bests::SelectionOutcome> {
    Ok(crate::bests::build_with_exclusions(
        dataset,
        &crate::bests::Options {
            scope: crate::report::Scope::AllSources,
            ..crate::bests::Options::default()
        },
    )?)
}

fn replay(fixture: Fixture, objects: tempfile::TempDir, dataset: ExportDataset) -> TestResult {
    let expected = witness::capture(&dataset)?;
    let exclusions = bests(&dataset)?.participation_exclusions;
    let archive = super::temporary()?;
    let path = archive.path().join("composed-refusal.json");
    let digest = dataset.archive_digest()?;
    dataset.save_frozen(&path)?;
    let source_path = fixture.store.root().to_path_buf();
    let objects_path = objects.path().to_path_buf();
    fixture.remove_sources()?;
    objects.close()?;
    drop(dataset);
    check!(!source_path.exists());
    check!(!objects_path.exists());
    let reopened = ExportDataset::reopen_frozen_with_digest(&path, &digest)?;
    check!(eq; witness::capture(&reopened)?, expected);
    check!(eq; bests(&reopened)?.participation_exclusions, exclusions);
    check!(bests(&reopened)?.selections.is_empty());
    Ok(())
}
