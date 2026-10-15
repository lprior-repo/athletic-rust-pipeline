use crate::export::ExportDataset;
use crate::report::{Derivation, Scope};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalSchool, CoachRole,
    CompetitionLevel, EventIdentity, EventKind, EventSpecification, Evidence, Gender, GradYear,
    Grade, ObservedGrade, PublishedGraduation, SchoolYear, SourceIdentity, SourceNamespace,
    SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::Store;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn athlete(name: &str) -> TestResult<CanonicalAthlete> {
    let (_, school) = CanonicalSchool::new(UsJurisdiction::Michigan, "School", "school", None);
    let mut athlete = CanonicalAthlete::new(
        &school,
        name,
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, name),
    )?;
    athlete.evidence = vec![
        Evidence::parsed(SourceRef::new("milesplit_roster", None), "2026-10-15"),
        Evidence::parsed(SourceRef::new("athleticlive_athletes", None), "2026-10-15"),
    ];
    athlete.source_links.push(SourceIdentity::new(
        SourceNamespace::AthleticNet {
            kind: "athlete".into(),
        },
        name,
    ));
    Ok(athlete)
}

fn add_related_population(dataset: &mut ExportDataset) -> TestResult {
    let subject = athlete("Runner")?;
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Michigan),
        "Invite",
        "2026-05-01",
        CompetitionLevel::Invitational,
    );
    meet.evidence = subject.evidence.clone();
    let mut event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track100m,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    event.evidence = subject.evidence.clone();
    let coach = CanonicalCoach::new(
        &subject.school,
        "Coach",
        Some(Sport::OutdoorTrack),
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    dataset.meets.push(meet);
    dataset.events.push(event);
    dataset.coaches.push(coach.clone());
    dataset.coach_observations.push(coach);
    Ok(())
}

#[test]
fn core_derivation_owns_evidence_transform_without_mutating_dataset() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut dataset = ExportDataset::load(&store)?;
    let original = athlete("Runner")?;
    dataset.athletes.push(original.clone());
    add_related_population(&mut dataset)?;
    let original_meets = dataset.meets.clone();
    let original_events = dataset.events.clone();
    let derivation = Derivation::of(&dataset, Scope::Core, None, &[]);
    let expected_evidence = vec![Evidence::parsed(
        SourceRef::new("milesplit_roster", None),
        "2026-10-15",
    )];
    let mut expected = original.clone();
    expected.evidence = expected_evidence.clone();
    expected.source_links.clear();
    check!(eq; derivation.athletes(), &[expected]);
    check!(eq; dataset.athletes, vec![original]);
    check!(eq;
        derivation.meets.first().ok_or("core meet")?.evidence,
        expected_evidence
    );
    check!(eq;
        derivation.events.first().ok_or("core event")?.evidence,
        expected_evidence
    );
    check!(eq; dataset.meets, original_meets);
    check!(eq; dataset.events, original_events);
    Ok(())
}

#[test]
fn accepted_alias_union_preserves_explicit_facts_and_canonical_order() -> TestResult {
    let mut original = athlete("Runner")?;
    let mut alias = athlete("Alias")?;
    let other = athlete("Independent")?;
    let grade = ObservedGrade {
        grade: Grade::new(11).ok_or("grade")?,
        school_year: SchoolYear::new(2025).ok_or("school year")?,
        source: SourceRef::new("alias_grade", None),
    };
    let graduation = PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new("alias_graduation", None),
    };
    original.observed_grades.push(grade.clone());
    alias.observed_grades.push(grade.clone());
    alias.published_graduations.push(graduation.clone());
    alias
        .public_profile_urls
        .push("https://example.com/alias".into());
    let alias_evidence = Evidence::parsed(SourceRef::new("alias_capture", None), "2026-10-14");
    alias.evidence.push(alias_evidence.clone());
    let aliases = [(alias.id.to_string(), original.id.to_string())]
        .into_iter()
        .collect();
    let mut expected = original.clone();
    expected.known_names = vec!["Runner".into(), "Alias".into()];
    expected.observed_grades = vec![grade];
    expected.published_graduations = vec![graduation];
    expected.public_profile_urls = vec!["https://example.com/alias".into()];
    expected.evidence.push(alias_evidence);
    expected
        .source_links
        .push(alias.source.clone().ok_or("alias identity")?);
    expected.source_links.extend(alias.source_links.clone());
    let mut expected_rows = vec![expected, other.clone()];
    expected_rows.sort_by(|left, right| left.id.cmp(&right.id));
    let source = [alias, other, original];
    let untouched = source.clone();
    let collapsed = crate::report::derivation::borrowed_or_collapsed(&source, &aliases);
    check!(eq; collapsed.as_ref(), expected_rows.as_slice());
    check!(eq; source, untouched);
    Ok(())
}
