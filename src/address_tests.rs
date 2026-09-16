use super::{parse_fields, AddressIssueSeverity};

#[test]
fn leading_zero_zip_and_raw_fields_are_preserved() {
    let address = parse_fields(
        Some(" 12 Maple Road "),
        Some(" 02108 "),
        Some(" Boston "),
        Some(" MA "),
    );
    assert_eq!(
        address.raw_street_combined.as_deref(),
        Some(" 12 Maple Road ")
    );
    assert_eq!(address.raw_postal.as_deref(), Some(" 02108 "));
    assert_eq!(address.raw_city.as_deref(), Some(" Boston "));
    assert_eq!(address.raw_state.as_deref(), Some(" MA "));
    assert_eq!(address.street_number.as_deref(), Some("12"));
    assert_eq!(address.road.as_deref(), Some("Maple Road"));
    assert_eq!(address.city.as_deref(), Some("Boston"));
    assert_eq!(address.postal.as_deref(), Some("02108"));
    assert_eq!(address.us_postal_syntax, Some(true));
    assert!(!address.requires_review());
}

#[test]
fn zip_plus_four_and_unit() {
    let address = parse_fields(
        Some("123 Main Street, Apt 04B"),
        Some("00501-0012"),
        Some("Holtsville"),
        Some("NY"),
    );
    assert_eq!(address.street_number.as_deref(), Some("123"));
    assert_eq!(address.road.as_deref(), Some("Main Street"));
    assert_eq!(address.unit.as_deref(), Some("04B"));
    assert_eq!(address.postal.as_deref(), Some("00501-0012"));
    assert_eq!(address.us_postal_syntax, Some(true));
    assert!(!address.requires_review());
}

#[test]
fn po_box_preserves_box_leading_zeros() {
    let address = parse_fields(Some("P.O. Box 007"), None, None, None);
    assert_eq!(address.po_box.as_deref(), Some("007"));
    assert_eq!(address.street_number, None);
    assert_eq!(address.road, None);
    assert!(!address.requires_review());
}

#[test]
fn foreign_region_and_postal_remain_partial() {
    let address = parse_fields(
        Some("22 King Street"),
        Some("K1A 0B1"),
        Some("Ottawa"),
        Some("Ontario"),
    );
    assert_eq!(address.region, None);
    assert_eq!(address.raw_state.as_deref(), Some("Ontario"));
    assert_eq!(address.postal.as_deref(), Some("K1A 0B1"));
    assert_eq!(address.us_postal_syntax, None);
    assert!(address
        .issues()
        .iter()
        .any(|issue| issue.code == "unknown_region"));
    assert!(!address.requires_review());
}

#[test]
fn missing_fields_are_advisory_and_distinguish_absence_from_blank() {
    let address = parse_fields(None, Some("  "), None, None);
    assert_eq!(address.raw_street_combined, None);
    assert_eq!(address.raw_postal.as_deref(), Some("  "));
    assert_eq!(address.postal, None);
    assert_eq!(address.issues().len(), 4);
    assert!(address
        .issues()
        .iter()
        .all(|issue| issue.severity == AddressIssueSeverity::Advisory));
    assert!(!address.requires_review());
}

#[test]
fn malformed_us_postal_is_not_discarded() {
    for postal in ["1234", "123456", "12345-123", "ABCDE", "１２３４５"] {
        let address = parse_fields(Some("10 Elm Road"), Some(postal), None, Some("WV"));
        assert_eq!(address.raw_postal.as_deref(), Some(postal));
        assert_eq!(address.postal.as_deref(), Some(postal));
        assert_eq!(address.us_postal_syntax, Some(false));
        assert!(address.requires_review());
        assert!(address
            .issues()
            .iter()
            .any(|issue| issue.code == "malformed_us_postal"));
    }
}

#[test]
fn ambiguous_or_malformed_streets_are_retained_without_guessing() {
    for street in [
        "123 Main St; 456 Oak St",
        "123 Main St\n456 Oak St",
        "Main Street",
        "123",
        "123 Main St Apt",
        "123 Main St & 456 Oak St",
    ] {
        let address = parse_fields(Some(street), None, None, None);
        assert_eq!(address.raw_street_combined.as_deref(), Some(street));
        assert_eq!(address.street_number, None);
        assert_eq!(address.road, None);
        assert!(address.requires_review());
        assert!(address
            .issues()
            .iter()
            .any(|issue| issue.code == "ambiguous_street"));
    }
}

#[test]
fn west_virginia_is_not_virginia() {
    let full = parse_fields(
        Some("1 Capitol Street"),
        Some("25301"),
        Some("Charleston"),
        Some("West Virginia"),
    );
    let abbreviated = parse_fields(None, None, None, Some("WV"));
    let virginia = parse_fields(None, None, None, Some("VA"));
    assert!(full.region.is_some());
    assert_eq!(full.region, abbreviated.region);
    assert_ne!(full.region, virginia.region);
    assert_eq!(full.raw_state.as_deref(), Some("West Virginia"));
    assert!(!full.requires_review());
}

#[test]
fn syntax_does_not_claim_zip_state_reference_validation() {
    let address = parse_fields(Some("1 Main Road"), Some("00000-0000"), None, Some("WV"));
    assert_eq!(address.us_postal_syntax, Some(true));
    assert!(!address.requires_review());
    assert!(!address
        .issues()
        .iter()
        .any(|issue| issue.code == "malformed_us_postal"));
}
