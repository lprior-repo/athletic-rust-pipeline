use super::*;

#[test]
fn distinct_sources_yield_distinct_subjects() {
    let school = SchoolId::mint("sch", &["WI", "Madison West"]);
    let class = GradYear::new(2027).expect("a class");
    let source_a = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let source_b = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "98765432");

    let athlete_a = CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, source_a);
    let athlete_b = CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, source_b);

    assert_eq!(
        athlete_a.candidate_key().index_id(),
        athlete_b.candidate_key().index_id(),
        "candidate search bucket is shared"
    );
    assert_ne!(athlete_a.id, athlete_b.id, "different source owners produce different subjects");
}

#[test]
fn identical_source_inputs_are_idempotent() {
    let school = SchoolId::mint("sch", &["WI", "Madison West"]);
    let class = GradYear::new(2027).expect("a class");
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");

    let a = CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, source.clone());
    let b = CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, source.clone());

    assert_eq!(a.id, b.id, "idempotent mint with same source");
    assert_eq!(a.canonical_name, b.canonical_name);
    assert_eq!(a.school, b.school);
    assert_eq!(a.grad_year, b.grad_year);
    assert_eq!(a.gender, b.gender);
}

#[test]
fn shared_bucket_does_not_merge_subjects() {
    let school = SchoolId::mint("sch", &["WI", "Madison West"]);
    let class = GradYear::new(2027).expect("a class");
    let source_a = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let source_b = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "tfrrs-77777");

    let a = CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, source_a);
    let b = CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, source_b);

    assert_eq!(a.candidate_key().index_id(), b.candidate_key().index_id());
    assert_ne!(a.id, b.id);
    assert_ne!(a.source, b.source);
}

#[test]
fn grade_agreement_is_cohort_evidence_not_identity() {
    let school = SchoolId::mint("sch", &["WI", "Madison West"]);
    let class = GradYear::new(2027).expect("a class");
    let source_a = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let source_b = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "98765432");

    let mut athlete_a =
        CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, source_a);
    let mut athlete_b =
        CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, source_b);

    let obs = ObservedGrade {
        grade: Grade::new(11).expect("a grade"),
        school_year: SchoolYear::new(2025).expect("a season"),
        source: SourceRef::new("roster", None),
    };
    athlete_a.observed_grades.push(obs.clone());
    athlete_b.observed_grades.push(obs);

    assert_eq!(athlete_a.derived_cohort_confidence(), Some(Confidence::HIGH));
    assert_eq!(athlete_b.derived_cohort_confidence(), Some(Confidence::HIGH));
    assert_ne!(athlete_a.id, athlete_b.id);
}

fn observed_athlete(school: &str, source: SourceIdentity) -> CanonicalAthlete {
    let school = SchoolId::mint("sch", &["WI", school]);
    let mut athlete = CanonicalAthlete::new(&school, "Synthetic Runner", GradYear::CO2027,
        Gender::Girls, source);
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some("https://example.test/results".to_owned())), "2026-09-26"));
    athlete
}

#[test]
fn source_bound_identity_requires_a_checked_primary_person_identifier() {
    for id in ["", "0", "01", "-1", "１", "1 ", "18446744073709551616"] {
        let athlete = observed_athlete("School A", SourceIdentity::new(SourceNamespace::MilesplitAthlete, id));
        let mut index = AthleteIdentityIndex::default();
        index.observe(&athlete).expect("observation");
        assert!(!index.isolated_source(&athlete.id.cast()), "invalid primary identifier: {id:?}");
    }
    for id in ["1", "18446744073709551615"] {
        let athlete = observed_athlete("School A", SourceIdentity::new(SourceNamespace::MilesplitAthlete, id));
        let mut index = AthleteIdentityIndex::default();
        index.observe(&athlete).expect("observation");
        assert!(index.isolated_source(&athlete.id.cast()), "valid primary identifier: {id}");
    }
}

#[test]
fn advisory_person_links_cannot_promote_a_result_row_to_source_bound_identity() {
    let mut athlete = observed_athlete("School A",
        SourceIdentity::new(SourceNamespace::Other("timer_result_row".to_owned()), "meet:1:row:2"));
    athlete.add_identity(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "999"));
    let mut index = AthleteIdentityIndex::default();
    index.observe(&athlete).expect("observation");
    assert!(!index.isolated_source(&athlete.id.cast()));
}

#[test]
fn a_fetched_page_alone_does_not_establish_a_parsed_person_identity() {
    let mut athlete = observed_athlete("School A",
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"));
    athlete.evidence[0].method = EvidenceMethod::Fetched;
    let mut index = AthleteIdentityIndex::default();
    index.observe(&athlete).expect("observation");
    assert!(!index.isolated_source(&athlete.id.cast()));
}

#[test]
fn shared_advisory_links_do_not_authorize_homonym_merges() {
    let mut first = observed_athlete("School A",
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"));
    let mut second = observed_athlete("School A",
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "222"));
    first.add_identity(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "999"));
    second.add_identity(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "999"));
    let mut index = AthleteIdentityIndex::default();
    index.observe(&first).expect("first observation");
    index.observe(&second).expect("second observation");
    assert!(!index.supports_identity(AppliedIdentityKind::SamePerson, &[first.id.cast(), second.id.cast()]));
}

#[test]
fn the_same_primary_person_identifier_can_support_a_reviewed_transfer() {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111");
    let first = observed_athlete("School A", source.clone());
    let second = observed_athlete("School B", source);
    assert_ne!(first.id, second.id);
    let mut index = AthleteIdentityIndex::default();
    index.observe(&first).expect("first observation");
    index.observe(&second).expect("second observation");
    assert!(index.supports_identity(AppliedIdentityKind::SamePerson, &[first.id.cast(), second.id.cast()]));
}
