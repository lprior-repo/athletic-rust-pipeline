use crate::model::{CanonicalAthlete, Gender, GradYear, SchoolId, SourceIdentity, SourceNamespace};
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
    assert_eq!(decoded, expected);
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
    assert_eq!(decoded.source, Some(first));
    assert_eq!(decoded.source_links, vec![second]);
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
    assert_eq!(decoded.source, Some(first));
    assert!(decoded.source_links.is_empty());
    Ok(())
}

#[test]
fn a_row_without_any_identity_decodes_without_an_owner() -> TestResult {
    let encoded = persisted_before_the_cutover(&athlete(), Vec::new())?;
    let decoded: CanonicalAthlete = serde_json::from_value(encoded)?;
    assert_eq!(decoded.source, None);
    assert!(decoded.source_links.is_empty());
    assert_eq!(decoded.identities().count(), 0);
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
    assert!(object.contains_key("source"));
    assert!(object.contains_key("source_links"));
    assert!(!object.contains_key("source_identities"));
    assert_eq!(
        serde_json::from_value::<CanonicalAthlete>(encoded)?,
        expected
    );
    Ok(())
}
