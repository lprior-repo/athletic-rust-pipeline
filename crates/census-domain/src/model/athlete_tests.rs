use crate::model::{
    CanonicalAthlete, Confidence, Evidence, Gender, GradYear, Grade, ObservedGrade,
    PublishedGraduation, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
};
use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn school() -> SchoolId {
    SchoolId::mint("sch", &["athlete-decode-fixture"])
}

fn athlete() -> CanonicalAthlete {
    CanonicalAthlete::new(
        &school(),
        "Cooper Robinson",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    )
}

fn persisted_before_the_cutover(
    athlete: &CanonicalAthlete,
    identities: Vec<Value>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut encoded = serde_json::to_value(athlete)?;
    let object = encoded
        .as_object_mut()
        .ok_or("an athlete encodes as an object")?;
    object.remove("source");
    object.remove("source_links");
    object.insert("source_identities".to_string(), Value::Array(identities));
    Ok(encoded)
}

fn identity(namespace: SourceNamespace, id: &str) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::to_value(SourceIdentity::new(namespace, id))?)
}

#[test]
fn a_row_persisted_before_the_source_cutover_decodes_with_its_first_identity_as_owner() -> TestResult
{
    let expected = athlete();
    let source = identity(SourceNamespace::MilesplitAthlete, "1001")?;
    let encoded = persisted_before_the_cutover(&expected, vec![source])?;
    let decoded: CanonicalAthlete = serde_json::from_value(encoded)?;
    check!(eq; decoded, expected);
    Ok(())
}

#[test]
fn further_legacy_identities_become_links() -> TestResult {
    let first = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001");
    let second = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "2002");
    let encoded = persisted_before_the_cutover(
        &athlete(),
        vec![
            serde_json::to_value(&first)?,
            serde_json::to_value(&second)?,
        ],
    )?;
    let decoded: CanonicalAthlete = serde_json::from_value(encoded)?;
    check!(eq; decoded.source, Some(first));
    check!(eq; decoded.source_links, vec![second]);
    Ok(())
}

#[test]
fn a_legacy_identity_repeated_in_the_links_is_listed_once() -> TestResult {
    let first = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001");
    let encoded = persisted_before_the_cutover(
        &athlete(),
        vec![serde_json::to_value(&first)?, serde_json::to_value(&first)?],
    )?;
    let decoded: CanonicalAthlete = serde_json::from_value(encoded)?;
    check!(eq; decoded.source, Some(first));
    check!(decoded.source_links.is_empty());
    Ok(())
}

#[test]
fn a_row_without_any_identity_decodes_without_an_owner() -> TestResult {
    let encoded = persisted_before_the_cutover(&athlete(), Vec::new())?;
    let decoded: CanonicalAthlete = serde_json::from_value(encoded)?;
    check!(eq; decoded.source, None);
    check!(decoded.source_links.is_empty());
    check!(eq; decoded.identities().count(), 0);
    Ok(())
}

#[test]
fn the_current_shape_encodes_the_owner_and_links_without_the_legacy_list() -> TestResult {
    let mut expected = athlete();
    expected.add_identity(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "2002"));
    let encoded = serde_json::to_value(&expected)?;
    let object = encoded
        .as_object()
        .ok_or("an athlete encodes as an object")?;
    check!(object.contains_key("source"));
    check!(object.contains_key("source_links"));
    check!(!object.contains_key("source_identities"));
    check!(eq; serde_json::from_value::<CanonicalAthlete>(encoded)?,
    expected);
    Ok(())
}

fn published(year: GradYear, source: &str) -> PublishedGraduation {
    PublishedGraduation {
        grad_year: year,
        source: SourceRef::new(source, Some(format!("https://source.example/{source}"))),
    }
}

fn grade(value: u8, year: i16) -> Result<ObservedGrade, Box<dyn std::error::Error>> {
    Ok(ObservedGrade {
        grade: Grade::new(value).ok_or("invalid fixture grade")?,
        school_year: SchoolYear::new(year).ok_or("invalid fixture school year")?,
        source: SourceRef::id("published-roster"),
    })
}

#[test]
fn matching_published_graduation_confirms_the_cohort_with_or_without_an_agreeing_grade(
) -> TestResult {
    let mut row = athlete();
    row.published_graduations
        .push(published(GradYear::CO2027, "owned-api"));
    check!(eq; row.observed_grades, Vec::new());
    check!(eq; row.derived_cohort_confidence(), Some(Confidence::HIGH));
    check!(eq; row.has_cohort_conflict(), false);
    row.observed_grades.push(grade(11, 2025)?);
    check!(eq; row.derived_cohort_confidence(), Some(Confidence::HIGH));
    check!(eq; row.has_cohort_conflict(), false);
    Ok(())
}

#[test]
fn a_disagreeing_direct_or_grade_claim_lowers_confidence_without_rewriting_the_cohort() -> TestResult
{
    let other_year = GradYear::new(2028).ok_or("invalid fixture graduation year")?;
    let mut matching_direct = athlete();
    matching_direct
        .published_graduations
        .push(published(GradYear::CO2027, "owned-api"));
    let mut grade_conflict = matching_direct.clone();
    grade_conflict.observed_grades.push(grade(10, 2025)?);
    let mut unsupported_grade = matching_direct.clone();
    unsupported_grade
        .observed_grades
        .push(grade(12, SchoolYear::MAX_START_YEAR)?);
    let mut direct_conflict = matching_direct;
    direct_conflict
        .published_graduations
        .push(published(other_year, "other-api"));
    let mut matching_grade = athlete();
    matching_grade.observed_grades.push(grade(11, 2025)?);
    matching_grade
        .published_graduations
        .push(published(other_year, "owned-api"));
    let mut only_disagreeing_direct = athlete();
    only_disagreeing_direct
        .published_graduations
        .push(published(other_year, "owned-api"));
    for row in [
        grade_conflict,
        unsupported_grade,
        direct_conflict,
        matching_grade,
        only_disagreeing_direct,
    ] {
        check!(eq; row.grad_year, GradYear::CO2027);
        check!(eq; row.has_cohort_conflict(), true);
        check!(eq; row.derived_cohort_confidence(), Some(Confidence::LOW));
    }
    Ok(())
}

#[test]
fn missing_typed_claims_leave_confidence_unknown_even_when_a_note_mentions_a_year(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut row = athlete();
    check!(eq; row.derived_cohort_confidence(), None);
    row.evidence.push(Evidence::derived(
        SourceRef::id("owned-api"),
        "2026-10-02",
        "gradYear2027",
    ));
    check!(eq; row.published_graduations, Vec::new());
    check!(eq; row.observed_grades, Vec::new());
    check!(eq; row.has_cohort_conflict(), false);
    check!(eq; row.derived_cohort_confidence(), None);
    Ok(())
}

#[test]
fn historical_rows_without_the_field_do_not_synthesize_claims_from_the_canonical_year_or_notes(
) -> TestResult {
    let mut row = athlete();
    row.evidence.push(Evidence::derived(
        SourceRef::id("owned-api"),
        "2026-10-02",
        "gradYear2027",
    ));
    row.published_graduations
        .push(published(GradYear::CO2027, "owned-api"));
    let mut encoded = serde_json::to_value(&row)?;
    encoded
        .as_object_mut()
        .ok_or("an athlete encodes as an object")?
        .remove("published_graduations");
    let decoded: CanonicalAthlete = serde_json::from_value(encoded)?;
    check!(eq; decoded.published_graduations, Vec::new());
    check!(eq; decoded.observed_grades, Vec::new());
    check!(eq; decoded.evidence, row.evidence);
    check!(eq; decoded.grad_year, GradYear::CO2027);
    check!(eq; decoded.derived_cohort_confidence(), None);
    Ok(())
}

#[test]
fn published_claims_roundtrip_with_their_conflicting_years_and_source_ownership() -> TestResult {
    let mut row = athlete();
    row.published_graduations = vec![
        published(GradYear::CO2027, "owned-api"),
        published(
            GradYear::new(2028).ok_or("invalid fixture year")?,
            "other-api",
        ),
    ];
    let decoded: CanonicalAthlete = serde_json::from_value(serde_json::to_value(&row)?)?;
    check!(eq; decoded, row);
    check!(eq; decoded.derived_cohort_confidence(), Some(Confidence::LOW));
    Ok(())
}
