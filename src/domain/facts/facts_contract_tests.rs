use super::*;

#[test]
fn display_names_preserve_observed_text_without_controls() {
    let name = AthleteName::parse("  Ada Lovelace  ").expect("name");
    let school = SchoolName::parse("North\u{00a0}High").expect("school");
    assert_eq!(name.as_str(), "  Ada Lovelace  ");
    assert_eq!(school.as_str(), "North\u{00a0}High");
    assert!(CityName::parse("\n").is_err());
    assert!(RegionName::parse("\u{0000}West").is_err());
}

#[test]
fn display_names_have_bounded_input() {
    let long_name = "x".repeat(201);
    let long_school = "x".repeat(201);
    let long_city = "x".repeat(121);
    let long_region = "x".repeat(121);
    assert!(AthleteName::parse(&long_name).is_err());
    assert!(SchoolName::parse(&long_school).is_err());
    assert!(CityName::parse(&long_city).is_err());
    assert!(RegionName::parse(&long_region).is_err());
}

#[test]
fn numeric_facts_enforce_boundaries_and_retry_exhaustion() {
    assert!(GraduationYear::new(1899).is_err());
    assert_eq!(GraduationYear::new(1900).expect("year").get(), 1900);
    assert_eq!(GraduationYear::new(2200).expect("year").get(), 2200);
    assert!(GraduationYear::new(2201).is_err());
    assert!(ConfidenceScore::new(f64::NAN).is_err());
    assert!(ConfidenceScore::new(f64::INFINITY).is_err());
    assert!(ConfidenceScore::new(-0.1).is_err());
    assert_eq!(ConfidenceScore::new(1.0).expect("score").get(), 1.0);
    let retry = RetryCount::new(0).expect("retry");
    let retry = retry.next().expect("retry");
    let retry = retry.next().expect("retry");
    let retry = retry.next().expect("retry");
    assert_eq!(retry.get(), 3);
    assert!(retry.next().is_err());
    assert!(RetryCount::new(4).is_err());
}

#[test]
fn serde_deserialization_cannot_bypass_numeric_validation() {
    assert!(serde_json::from_str::<GraduationYear>("1899").is_err());
    assert!(serde_json::from_str::<ConfidenceScore>("null").is_err());
    assert!(serde_json::from_str::<ConfidenceScore>("1.1").is_err());
    assert!(serde_json::from_str::<RetryCount>("4").is_err());
    let location = Location::CityRegion {
        city: CityName::parse("Davis").expect("city"),
        region: RegionName::parse("CA").expect("region"),
    };
    let encoded = serde_json::to_string(&location).expect("location");
    let decoded: Location = serde_json::from_str(&encoded).expect("location");
    assert_eq!(decoded, location);
    assert!(serde_json::from_str::<Location>(r#"{"city_only":""}"#).is_err());
}
