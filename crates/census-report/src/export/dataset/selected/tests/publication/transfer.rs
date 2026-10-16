use super::{publication, publish, ExportDataset, Fixture, GradYear, Scope, TestResult};
use census_domain::public_identity_history::PublicAthleteId;

mod readback;

#[tokio::test]
async fn transferred_athlete_retains_public_birth_current_school_and_unresolved_old_school_marks_after_source_deletion(
) -> TestResult {
    let (fixture, public_id) = Fixture::transferred().await?;
    let objects = super::super::temporary()?;
    let publication_root = super::super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    assert_input(&dataset, &fixture, &public_id)?;
    let old_school = dataset
        .teams
        .get(&fixture.performance.team)
        .ok_or("historical team")?
        .school
        .clone();
    let retained_result = dataset
        .performances
        .iter()
        .find(|row| row.source_key == "milesplit_result:201782264")
        .ok_or("exact retained raw result")?
        .id
        .clone();
    let expected = super::super::witness::capture(&dataset)?;
    let lineage = dataset.lineage.clone();
    let root = publication_root.path().join("publication");
    let path = publish(&fixture, &dataset, &root)?;
    let verified = publication::verify_published(&path)?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    let generation = path.parent().ok_or("publication generation")?;
    let expected_ids = readback::Expected {
        public_id: public_id.as_str().to_owned(),
        canonical: fixture.performance.athlete.to_string(),
        current_candidate: fixture.roster_athlete.to_string(),
        old_school: old_school.to_string(),
        current_school: fixture.roster_school.to_string(),
        retained_result: retained_result.to_string(),
    };
    readback::artifacts(generation, &path, &expected_ids)?;
    let store_path = fixture.store.root().to_path_buf();
    let cache_path = fixture.store.http_cache_dir();
    let staging_path = objects.path().to_path_buf();
    fixture.remove_sources()?;
    objects.close()?;
    drop(dataset);
    check!(!store_path.exists());
    check!(!cache_path.exists());
    check!(!staging_path.exists());
    for entry in std::fs::read_dir(&root)? {
        let entry = entry?;
        check!(!entry.file_name().to_string_lossy().starts_with(".staging."));
    }
    let reopened =
        publication::open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; super::super::witness::capture(&reopened.dataset)?, expected);
    check!(eq; reopened.dataset.verified_public_identities()?.public_id(&expected_ids.current_candidate), Some(&public_id));
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    readback::artifacts(generation, &path, &expected_ids)?;
    Ok(())
}

fn assert_input(
    dataset: &ExportDataset,
    fixture: &Fixture,
    public_id: &PublicAthleteId,
) -> TestResult {
    check!(eq; dataset.athletes.len(), 2);
    check!(eq; dataset.performances.len(), 2);
    check!(ne; fixture.performance.athlete, fixture.roster_athlete);
    let identities = dataset.verified_public_identities()?;
    check!(eq; identities.public_ids().count(), 1);
    for candidate in [&fixture.performance.athlete, &fixture.roster_athlete] {
        check!(eq; identities.public_id(candidate.as_str()), Some(public_id));
    }
    check!(eq; identities.group(fixture.roster_athlete.as_str()).ok_or("membership group")?.canonical_candidate().as_str(), fixture.performance.athlete.as_str());
    check!(eq; dataset.verified_current_affiliations()?.resolution(&fixture.performance.athlete).school(), Some(&fixture.roster_school));
    let old_school = &dataset
        .teams
        .get(&fixture.performance.team)
        .ok_or("old school team")?
        .school;
    check!(ne; old_school, &fixture.roster_school);
    for performance in &dataset.performances {
        check!(eq; &performance.athlete, &fixture.performance.athlete);
        check!(eq; &dataset.teams.get(&performance.team).ok_or("performance team")?.school, old_school);
        check!(eq; dataset.verified_participation()?.assessment(&performance.id).ok_or("retained participation")?.outcome(), crate::export::ParticipationOutcome::Unresolved);
        check!(eq; dataset.verified_participation()?.assessment(&performance.id).ok_or("retained participation")?.reasons().iter().collect::<Vec<_>>(), vec![
            census_domain::participation_candidates::ParticipationReason::HighSchoolStatusMissing,
            census_domain::participation_candidates::ParticipationReason::SummerBoundaryUnresolved,
        ]);
    }
    Ok(())
}
