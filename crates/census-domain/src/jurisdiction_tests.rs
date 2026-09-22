//! Tests for the jurisdiction value: totality of the table, code/name round trips, and the
//! rejections that keep the census at exactly the 50 states plus DC.

use super::*;

#[test]
fn all_lists_every_variant_exactly_once_in_declaration_order() {
    assert_eq!(UsJurisdiction::ALL.len(), 51);
    let mut seen: Vec<UsJurisdiction> = UsJurisdiction::ALL.to_vec();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 51, "a variant is missing or duplicated in ALL");
    assert!(
        UsJurisdiction::ALL.is_sorted(),
        "ALL must follow declaration order so Ord reads alphabetically"
    );
    for jurisdiction in UsJurisdiction::ALL {
        assert!(!jurisdiction.code().is_empty());
        assert!(!jurisdiction.name().is_empty());
    }
}

#[test]
fn codes_and_names_are_unique() {
    for (index, jurisdiction) in UsJurisdiction::ALL.iter().enumerate() {
        for other in &UsJurisdiction::ALL[index.saturating_add(1)..] {
            assert_ne!(jurisdiction.code(), other.code(), "duplicate code");
            assert_ne!(jurisdiction.name(), other.name(), "duplicate name");
        }
    }
}

#[test]
fn every_jurisdiction_round_trips_through_code_and_name() {
    for jurisdiction in UsJurisdiction::ALL {
        assert_eq!(
            UsJurisdiction::from_code(jurisdiction.code()),
            Some(jurisdiction)
        );
        assert_eq!(
            UsJurisdiction::parse(jurisdiction.code()),
            Some(jurisdiction)
        );
        assert_eq!(
            UsJurisdiction::parse(jurisdiction.name()),
            Some(jurisdiction)
        );
    }
}

#[test]
fn parse_ignores_case_and_surrounding_whitespace() {
    assert_eq!(
        UsJurisdiction::parse(" wi "),
        Some(UsJurisdiction::Wisconsin)
    );
    assert_eq!(
        UsJurisdiction::parse("wisconsin"),
        Some(UsJurisdiction::Wisconsin)
    );
    assert_eq!(
        UsJurisdiction::parse("district of columbia"),
        Some(UsJurisdiction::DistrictOfColumbia)
    );
    assert_eq!(
        UsJurisdiction::from_code(" dc "),
        Some(UsJurisdiction::DistrictOfColumbia)
    );
}

#[test]
fn parse_rejects_territories_and_anything_else() {
    // Territories and freely associated states are explicit absences, not silent inclusions.
    for raw in ["PR", "GU", "VI", "AS", "MP", "FM", "MH", "PW"] {
        assert_eq!(UsJurisdiction::parse(raw), None, "{raw} must not parse");
    }
    for raw in [
        "",
        "   ",
        "XX",
        "ZZ",
        "Wisconsn",
        "W1",
        "Wisconsin, WI",
        "51",
    ] {
        assert_eq!(UsJurisdiction::parse(raw), None, "{raw:?} must not parse");
    }
    assert_eq!(UsJurisdiction::from_code(""), None);
}

#[test]
fn display_prints_the_code() {
    for jurisdiction in UsJurisdiction::ALL {
        assert_eq!(jurisdiction.to_string(), jurisdiction.code());
    }
}

#[test]
fn from_str_reports_the_domain_error() {
    use std::str::FromStr;
    assert_eq!(
        UsJurisdiction::from_str("WI"),
        Ok(UsJurisdiction::Wisconsin)
    );
    assert_eq!(
        UsJurisdiction::from_str("PR"),
        Err(DomainError::Unsupported {
            field: "jurisdiction"
        })
    );
}

#[test]
fn deserialization_validates_instead_of_retaining_strings() {
    use serde::de::value::{Error as DeError, StrDeserializer};
    use serde::de::IntoDeserializer;

    let code: StrDeserializer<DeError> = "WI".into_deserializer();
    assert!(matches!(
        UsJurisdiction::deserialize(code),
        Ok(UsJurisdiction::Wisconsin)
    ));

    let name: StrDeserializer<DeError> = "New Hampshire".into_deserializer();
    assert!(matches!(
        UsJurisdiction::deserialize(name),
        Ok(UsJurisdiction::NewHampshire)
    ));

    let territory: StrDeserializer<DeError> = "PR".into_deserializer();
    assert!(UsJurisdiction::deserialize(territory).is_err());
}
