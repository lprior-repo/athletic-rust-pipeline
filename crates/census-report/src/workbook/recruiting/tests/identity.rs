use super::*;
use census_domain::model::{
    IdentityApplication, IdentityProjectionBuilder, IdentityStatus, ReviewCase, ReviewState,
    ReviewVerdictRecord, ATHLETE_IDENTITY_FAMILY,
};

fn member(school: &SchoolId, name: &str, gender: Gender, url: &str) -> CanonicalAthlete {
    let mut athlete = CanonicalAthlete::new(
        school,
        name,
        GradYear::CO2027,
        gender,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169").with_url(url),
    );
    athlete.public_profile_urls.push(url.to_owned());
    athlete.evidence = evidence("wiaa_results", Some(url));
    athlete
}

pub(super) fn accept(store: &Store, members: &[CanonicalAthlete]) {
    let index = store.athlete_identity_index().unwrap();
    let ids = members
        .iter()
        .map(|athlete| athlete.id.cast())
        .collect::<Vec<_>>();
    let subject = "Source-backed identity fixture";
    let detail = "One published provider identifier retained with alternate names";
    let evidence = index.case_evidence(subject, detail, &ids).unwrap();
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        members[0].id.as_str(),
        subject,
        detail,
        evidence,
    );
    case.member_ids = ids;
    case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "value_proposed".into(),
        field: "identity".into(),
        value: "same_person".into(),
        accepted: true,
        confidence: 100,
        rationale: detail.into(),
        reviewer: "deterministic fixture".into(),
        observed_at: DAY.into(),
        member_ids: case.member_ids.clone(),
    };
    store.replace(Table::ReviewCases, &case).unwrap();
    store.replace(Table::IdentityVerdicts, &verdict).unwrap();
    let cases = [case];
    let verdicts = [verdict];
    let builder = IdentityProjectionBuilder::new(index, &cases, &verdicts).unwrap();
    let decisions = builder
        .reviewed_applications(DAY)
        .map(|(_, result)| match result.unwrap() {
            IdentityApplication::Accepted(decision) => decision,
            IdentityApplication::Retained(issue) => {
                panic!("fixture acceptance rejected: {issue:?}")
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(store.apply_identity_decisions(&decisions).unwrap(), 1);
}

fn add_result(store: &Store, athlete: &CanonicalAthlete, kind: EventKind, value: i32) {
    let meet = meet(
        store,
        UsJurisdiction::Wisconsin,
        "Union Invitational",
        "2026-05-01",
        CompetitionLevel::Invitational,
        Sport::OutdoorTrack,
    );
    let event = event(store, &meet, kind.clone());
    performance(
        store,
        &PerformanceRow {
            athlete: &athlete.id,
            school: &athlete.school,
            meet: &meet,
            event: &event,
            kind: &kind,
            date: "2026-05-01",
        },
        Mark::TimeSeconds(CentiSeconds::new(value)),
        "wiaa_results",
        "https://wiaa.test/union/results",
    );
}

#[test]
fn accepted_aliases_publish_one_person_with_all_events_profiles_and_supported_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let school = school(&store, UsJurisdiction::Wisconsin, "Union High");
    let mut first = member(
        &school,
        "Synthetic Runner",
        Gender::Unknown,
        "https://wi.milesplit.com/athletes/accepted-person",
    );
    let mut second = member(
        &school,
        "S. Runner",
        Gender::Boys,
        "https://mn.milesplit.com/athletes/accepted-person",
    );
    first.public_profile_urls = vec!["https://profiles.test/first".into()];
    second.public_profile_urls = vec!["https://profiles.test/second".into()];
    first.sports = vec![Sport::OutdoorTrack];
    second.sports = vec![Sport::CrossCountry];
    first.observed_grades.push(ObservedGrade {
        grade: Grade::new(11).unwrap(),
        school_year: SchoolYear::new(2025).unwrap(),
        source: SourceRef::id("wiaa_results"),
    });
    second.observed_grades.push(ObservedGrade {
        grade: Grade::new(12).unwrap(),
        school_year: SchoolYear::new(2026).unwrap(),
        source: SourceRef::id("wiaa_results"),
    });
    second.add_identity(
        SourceIdentity::new(SourceNamespace::athletic_net("athlete"), "union")
            .with_url("https://www.athletic.net/athlete/union"),
    );
    store
        .append_many(Table::Athletes, &[first.clone(), second.clone()])
        .unwrap();
    add_result(&store, &first, EventKind::Track400m, 5000);
    add_result(&store, &second, EventKind::Track800m, 12000);
    coach(
        &store,
        &school,
        "Published Coach",
        Some(Sport::OutdoorTrack),
        CoachRole::HeadCoach,
        "coach@union.test",
    );
    accept(&store, &[first.clone(), second.clone()]);
    let dataset = crate::export::ExportDataset::load(&store).unwrap();
    let canonical = dataset
        .identities()
        .canonical_id(first.id.as_str())
        .to_owned();
    assert_eq!(
        dataset.identities().canonical_id(second.id.as_str()),
        canonical
    );
    let derivation = crate::report::Derivation::of(&dataset, Scope::AllSources, Some(2027));
    let [athlete] = derivation.athletes() else {
        panic!("one accepted identity required")
    };
    assert_eq!(athlete.id.as_str(), canonical);
    assert_eq!(athlete.gender, Gender::Boys);
    for member in [&first, &second] {
        assert!(athlete.known_names.contains(&member.canonical_name));
        assert!(athlete
            .public_profile_urls
            .contains(&member.public_profile_urls[0]));
        assert!(athlete.evidence.contains(&member.evidence[0]));
        assert!(athlete.observed_grades.contains(&member.observed_grades[0]));
    }
    assert_eq!(dataset.athletes.len(), 2);
    let coverage = crate::report::coverage_report(&dataset, Some(2027)).unwrap();
    let census = crate::report::build_census(&derivation, &store.out_dir());
    assert_eq!(coverage.read.athletes, 1);
    assert_eq!(coverage.read.performances, 2);
    assert_eq!(coverage.published_totals().athletes, census.totals.athletes);
    let jurisdiction = coverage
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction == UsJurisdiction::Wisconsin.into())
        .unwrap();
    assert_eq!(jurisdiction.with_performance, 1);
    assert_eq!(jurisdiction.performances, 2);
    assert_eq!(jurisdiction.athletes_core, 1);
    let projection = recruiting(&store, Scope::AllSources, Some(2027));
    let path = dir.path().join("accepted.xlsx");
    let mut book = Workbook::new();
    projection.write_athletes(&mut book, &path).unwrap();
    projection.write_prs(&mut book, &path).unwrap();
    book.save(&path).unwrap();
    let mut book = open_workbook(&path).unwrap();
    let range = sheet(&mut book, "Athletes");
    assert_eq!(range.height(), 2);
    let row = row_of(&range, &canonical);
    for (header, expected) in [
        ("TF", "yes"),
        ("XC", "yes"),
        ("Event list", "400m; 800m"),
        ("Observed School Year", "2026-27"),
        ("Performance count", "2"),
        ("Meet count", "1"),
        ("Identity Status", "verified"),
        ("Head TF Coach", "Published Coach"),
        ("School", "Union High"),
        ("Sources Count", "2"),
    ] {
        assert_eq!(
            text(&range, row, column_of(&range, header)),
            expected,
            "{header}"
        );
    }
    let summary = text(&range, row, column_of(&range, "Headline PR summary"));
    assert!(summary.contains("400m 50.00"));
    assert!(summary.contains("800m 2:00.00"));
    let profiles = ["MileSplit URL", "Other profile URLs", "Athletic.net URL"]
        .map(|header| text(&range, row, column_of(&range, header)))
        .join("; ");
    assert!(profiles.contains(&first.public_profile_urls[0]));
    assert!(profiles.contains(&second.public_profile_urls[0]));
    assert!(profiles.contains("https://wi.milesplit.com/athletes/accepted-person"));
    assert!(profiles.contains("https://mn.milesplit.com/athletes/accepted-person"));
    assert!(profiles.contains("https://www.athletic.net/athlete/union"));
    let prs = sheet(&mut book, "PRs");
    assert_eq!(prs.height(), 3);
    assert_eq!(
        text(&prs, row_of_event(&prs, &canonical, "Track400m"), 9),
        "50.00"
    );
    assert_eq!(
        text(&prs, row_of_event(&prs, &canonical, "Track800m"), 9),
        "2:00.00"
    );
    verified_publication(&store, 1);
}

#[test]
fn same_name_provider_owned_people_remain_separate_and_unresolved_status_is_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let school = school(&store, UsJurisdiction::Wisconsin, "Homonym High");
    let first = member(
        &school,
        "Synthetic Runner",
        Gender::Boys,
        "https://wi.milesplit.com/athletes/accepted-person",
    );
    let mut second = CanonicalAthlete::new(
        &school,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "other-person"),
    );
    second.evidence = evidence("wiaa_results", Some("https://wiaa.test/other/results"));
    store
        .append_many(Table::Athletes, &[first.clone(), second.clone()])
        .unwrap();
    store
        .replace(
            Table::ReviewCases,
            &ReviewCase::pending(
                ATHLETE_IDENTITY_FAMILY,
                second.id.as_str(),
                "Synthetic Runner",
                "Unresolved source ownership",
            ),
        )
        .unwrap();
    let dataset = crate::export::ExportDataset::load(&store).unwrap();
    assert!(dataset.canonical_aliases.is_empty());
    assert_eq!(
        dataset.identities().status(first.id.as_str()).unwrap(),
        IdentityStatus::Unverified
    );
    assert_eq!(
        dataset.identities().status(second.id.as_str()).unwrap(),
        IdentityStatus::Pending
    );
    let projection = recruiting(&store, Scope::AllSources, Some(2027));
    let path = dir.path().join("homonyms.xlsx");
    let mut book = Workbook::new();
    projection.write_athletes(&mut book, &path).unwrap();
    book.save(&path).unwrap();
    let mut book = open_workbook(&path).unwrap();
    let range = sheet(&mut book, "Athletes");
    assert_eq!(range.height(), 3);
    for (athlete, status) in [(&first, "unverified"), (&second, "pending")] {
        let row = row_of(&range, athlete.id.as_str());
        assert_eq!(
            text(&range, row, column_of(&range, "Identity Status")),
            status
        );
        assert_eq!(
            text(&range, row, column_of(&range, "Review Status")),
            "review"
        );
    }
}
