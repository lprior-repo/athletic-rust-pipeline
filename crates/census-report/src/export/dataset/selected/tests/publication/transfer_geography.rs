use super::{
    publication, workbook, ExportDataset, Fixture, GradYear, Options, SchoolYear, Scope,
    TestResult, UsJurisdiction,
};
use census_domain::model::CanonicalPerformance;
use census_domain::public_identity_history::PublicAthleteId;
use std::collections::BTreeMap;

mod readback;

struct Expected {
    included: bool,
    public_id: String,
    canonical: String,
    current_candidate: String,
    current_school: String,
    old_school: String,
    profile: String,
    as_of: String,
    generation: String,
    performances: Vec<CanonicalPerformance>,
    participation_refs: BTreeMap<String, String>,
    affiliation_rows: BTreeMap<String, BTreeMap<String, String>>,
    retained: BTreeMap<&'static str, serde_json::Value>,
}

#[tokio::test]
async fn wisconsin_only_publication_includes_current_transfer_and_retains_alabama_performances(
) -> TestResult {
    exercise(UsJurisdiction::Wisconsin, true).await
}

#[tokio::test]
async fn alabama_only_publication_excludes_current_wisconsin_transfer_and_retains_audit(
) -> TestResult {
    exercise(UsJurisdiction::Alabama, false).await
}

async fn exercise(jurisdiction: UsJurisdiction, included: bool) -> TestResult {
    let (fixture, public_id) = Fixture::transferred_in(jurisdiction).await?;
    let objects = super::super::temporary()?;
    let publication_root = super::super::temporary()?;
    let dataset = fixture.capture(objects.path())?;
    let expected = capture(&dataset, &fixture, &public_id, included)?;
    check!(eq; dataset.selected_authority()?.profile().input().jurisdictions, vec![jurisdiction]);
    let original = serde_json::to_vec(&dataset)?;
    let witness = super::super::witness::capture(&dataset)?;
    let lineage = dataset.lineage.clone();
    let root = publication_root.path().join("publication");
    let options = Options {
        grad_year: Some(2027),
        out: Some(root.clone()),
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: vec![jurisdiction],
        school_year: SchoolYear::new(2026).ok_or("publication school year")?,
    };
    let censuses =
        workbook::Censuses::of(&dataset, &fixture.store.out_dir(), &options.jurisdictions)?;
    let path = workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
    let verified = publication::verify_published(&path)?;
    check!(eq; verified.workbook.mapped_athletes, u64::from(included));
    check!(eq; serde_json::to_vec(&dataset)?, original);
    let generation = path.parent().ok_or("published generation")?;
    readback::artifacts(generation, &path, &expected)?;
    let source_root = fixture.store.root().to_path_buf();
    let source_cache = fixture.store.http_cache_dir();
    let source_objects = objects.path().to_path_buf();
    fixture.remove_sources()?;
    objects.close()?;
    drop(dataset);
    check!(!source_root.exists());
    check!(!source_cache.exists());
    check!(!source_objects.exists());
    let reopened =
        publication::open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; super::super::witness::capture(&reopened.dataset)?, witness);
    check!(eq; reopened.dataset.selected_authority()?.profile().input().jurisdictions, vec![jurisdiction]);
    check!(eq; reopened.dataset.verified_public_identities()?.public_id(&expected.current_candidate), Some(&public_id));
    check_participation(&reopened.dataset)?;
    check!(eq; publication::verify_published(&path)?.generation_digest, verified.generation_digest);
    readback::artifacts(generation, &path, &expected)
}

fn capture(
    dataset: &ExportDataset,
    fixture: &Fixture,
    public_id: &PublicAthleteId,
    included: bool,
) -> TestResult<Expected> {
    check!(eq; dataset.athletes.len(), 2);
    check!(ne; fixture.performance.athlete, fixture.roster_athlete);
    let identities = dataset.verified_public_identities()?;
    check!(eq; identities.public_ids().count(), 1);
    for candidate in [&fixture.performance.athlete, &fixture.roster_athlete] {
        check!(eq; identities.public_id(candidate.as_str()), Some(public_id));
    }
    check!(eq; identities.group(fixture.roster_athlete.as_str()).ok_or("retained membership")?.canonical_candidate().as_str(), fixture.performance.athlete.as_str());
    let affiliations = dataset.verified_current_affiliations()?;
    check!(eq; affiliations.resolution(&fixture.performance.athlete).school(), Some(&fixture.roster_school));
    check!(!affiliations
        .resolution(&fixture.performance.athlete)
        .evidence_refs()
        .is_empty());
    check!(eq; dataset.schools.get(&fixture.roster_school).ok_or("current school")?.state, Some(UsJurisdiction::Wisconsin));
    let old_school = &dataset
        .teams
        .get(&fixture.performance.team)
        .ok_or("retained team")?
        .school;
    check!(ne; old_school, &fixture.roster_school);
    check!(eq; dataset.schools.get(old_school).ok_or("historical school")?.state, Some(UsJurisdiction::Alabama));
    for performance in &dataset.performances {
        check!(eq; performance.athlete, fixture.performance.athlete);
        check!(eq; &dataset.teams.get(&performance.team).ok_or("performance team")?.school, old_school);
    }
    let participation_refs = check_participation(dataset)?;
    let retained = BTreeMap::from([
        ("performances", serde_json::to_value(&dataset.performances)?),
        ("teams", serde_json::to_value(&dataset.teams)?),
        ("schools", serde_json::to_value(&dataset.schools)?),
        ("meets", serde_json::to_value(&dataset.meets)?),
    ]);
    Ok(Expected {
        included,
        public_id: public_id.as_str().to_owned(),
        canonical: fixture.performance.athlete.to_string(),
        current_candidate: fixture.roster_athlete.to_string(),
        current_school: fixture.roster_school.to_string(),
        old_school: old_school.to_string(),
        profile: affiliations.profile_id().as_str().to_owned(),
        as_of: affiliations.as_of().as_str().to_owned(),
        generation: dataset.lineage.input_generation.clone(),
        performances: dataset.performances.clone(),
        participation_refs,
        affiliation_rows: affiliation_rows(dataset)?,
        retained,
    })
}

fn check_participation(dataset: &ExportDataset) -> TestResult<BTreeMap<String, String>> {
    check!(eq; dataset.performances.len(), 2);
    let mut keys = Vec::new();
    let mut refs = BTreeMap::new();
    for performance in &dataset.performances {
        let assessment = dataset
            .verified_participation()?
            .assessment(&performance.id)
            .ok_or("retained participation")?;
        check!(eq; assessment.outcome(), crate::export::ParticipationOutcome::Unresolved);
        check!(eq; assessment.reasons().iter().collect::<Vec<_>>(), vec![
            census_domain::participation_candidates::ParticipationReason::HighSchoolStatusMissing,
            census_domain::participation_candidates::ParticipationReason::SummerBoundaryUnresolved,
        ]);
        check!(assessment.diagnostics().is_empty());
        check!(!assessment.evidence_refs().is_empty());
        check!(refs
            .insert(
                performance.id.to_string(),
                serde_json::to_string(assessment.evidence_refs())?
            )
            .is_none());
        keys.push(performance.source_key.as_str());
        if performance.source_key == "milesplit_result:201782264" {
            check!(eq; crate::bests::mark_text(&performance.mark), "14-9");
            check!(eq; performance.date, "2026-04-03");
        }
    }
    keys.sort_unstable();
    check!(eq; keys, vec!["milesplit_result:201782263", "milesplit_result:201782264"]);
    Ok(refs)
}

fn affiliation_rows(
    dataset: &ExportDataset,
) -> TestResult<BTreeMap<String, BTreeMap<String, String>>> {
    let affiliations = dataset.verified_current_affiliations()?;
    let mut rows = BTreeMap::new();
    for athlete in &dataset.athletes {
        let resolution = affiliations.resolution(&athlete.id);
        let audit = affiliations.audit(&athlete.id);
        let references = audit.map_or(&[][..], |row| row.references());
        let unavailable = audit.map_or_else(
            || vec!["missing_current_affiliation_evidence"],
            |row| row.unavailable().iter().map(String::as_str).collect(),
        );
        let row = BTreeMap::from([
            (
                "current_school_id".into(),
                resolution.school().map_or("", |id| id.as_str()).into(),
            ),
            ("resolution_status".into(), resolution.status().into()),
            ("evidence_refs".into(), serde_json::to_string(references)?),
            (
                "unavailable_reasons".into(),
                serde_json::to_string(&unavailable)?,
            ),
        ]);
        check!(rows.insert(athlete.id.to_string(), row).is_none());
    }
    Ok(rows)
}
