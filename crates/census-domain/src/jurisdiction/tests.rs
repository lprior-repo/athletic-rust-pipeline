//! Tests for the jurisdiction value: totality of the table, code/name round trips, and the
//! rejections that keep the census at the modelled 50 states plus DC, of which all but Alaska and
//! Hawaii are admitted as the run scope.

use crate::error::DomainError;
use crate::jurisdiction::{
    bucket::JurisdictionBucket, meet_state::MeetState, table::UsJurisdiction,
};
use serde::de::IntoDeserializer;
use serde::Deserialize;

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
fn census_scope_is_the_contiguous_states_and_dc() {
    assert_eq!(UsJurisdiction::CENSUS_SCOPE.len(), 49);
    let mut scope: Vec<UsJurisdiction> = UsJurisdiction::CENSUS_SCOPE.to_vec();
    scope.sort_unstable();
    scope.dedup();
    assert_eq!(scope.len(), 49, "a state is missing or duplicated");
    assert!(
        UsJurisdiction::CENSUS_SCOPE.is_sorted(),
        "CENSUS_SCOPE reads alphabetically"
    );
    for state in UsJurisdiction::CENSUS_SCOPE {
        assert!(
            UsJurisdiction::ALL.contains(&state),
            "a run-scope state must be a modelled jurisdiction"
        );
    }
    // The constant and the admission check are two spellings of one rule. A state added to one and
    // not the other must fail here, not in a coverage denominator months later.
    for jurisdiction in UsJurisdiction::ALL {
        assert_eq!(
            jurisdiction.is_in_census_scope(),
            UsJurisdiction::CENSUS_SCOPE.contains(&jurisdiction),
            "{jurisdiction:?} disagrees between the scope constant and the admission check"
        );
    }
    for state in UsJurisdiction::CENSUS_SCOPE {
        assert!(
            state.require_census_scope().is_ok(),
            "{state:?} is in the scope and must be admitted"
        );
    }
    assert!(UsJurisdiction::Texas.require_census_scope().is_ok());
    assert!(
        UsJurisdiction::DistrictOfColumbia
            .require_census_scope()
            .is_ok(),
        "D.C. is part of the run scope"
    );
    for outside in [UsJurisdiction::Alaska, UsJurisdiction::Hawaii] {
        assert!(
            outside.require_census_scope().is_err(),
            "{outside:?} is outside the run scope and must be refused"
        );
    }
}

/// CENSUS_SCOPE must match ALL minus EXCLUDED name-for-name, in order — not just by set
/// membership. A one-for-one substitution would pass the count-and-sort check but silently
/// change which state is covered.
#[test]
fn census_scope_is_all_minus_excluded_in_order() {
    let expected: Vec<_> = UsJurisdiction::ALL
        .iter()
        .copied()
        .filter(|j| !UsJurisdiction::EXCLUDED_FROM_CENSUS.contains(j))
        .collect();
    assert_eq!(
        UsJurisdiction::CENSUS_SCOPE.to_vec(),
        expected,
        "CENSUS_SCOPE must match ALL minus EXCLUDED_FROM_CENSUS, in declaration order"
    );
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

#[test]
fn unplaced_bucket_sorts_after_every_jurisdiction() {
    let unplaced = JurisdictionBucket::Unplaced;
    for jurisdiction in UsJurisdiction::ALL {
        let placed = JurisdictionBucket::from(jurisdiction);
        assert!(
            placed < unplaced,
            "{} must sort before the unplaced row",
            jurisdiction
        );
        assert_eq!(placed.code(), jurisdiction.code());
        assert_eq!(placed.jurisdiction(), Some(jurisdiction));
    }
    assert_eq!(unplaced.code(), JurisdictionBucket::UNPLACED_CODE);
    assert_eq!(unplaced.jurisdiction(), None);
}

#[test]
fn bucket_codes_round_trip_and_reserved_labels_are_not_jurisdictions() {
    for jurisdiction in UsJurisdiction::ALL {
        let bucket = JurisdictionBucket::from(Some(jurisdiction));
        assert_eq!(JurisdictionBucket::from_code(bucket.code()), Some(bucket));
        assert_eq!(bucket.to_string(), jurisdiction.code());
    }
    assert_eq!(
        JurisdictionBucket::from_code("unknown"),
        Some(JurisdictionBucket::Unplaced)
    );
    assert_eq!(
        JurisdictionBucket::from_code(" wi "),
        Some(UsJurisdiction::Wisconsin.into())
    );
    // A territory is still refused, and a bucket never admits a name the jurisdiction itself does
    // not: the printed form is the code, so the name is not a bucket label.
    assert_eq!(JurisdictionBucket::from_code("PR"), None);
    assert_eq!(JurisdictionBucket::from_code("Wisconsin"), None);
    assert_eq!(JurisdictionBucket::from_code(""), None);

    let unplaced = JurisdictionBucket::Unplaced;
    assert_eq!(unplaced.to_string(), JurisdictionBucket::UNPLACED_CODE);
    assert_eq!(JurisdictionBucket::from(None::<UsJurisdiction>), unplaced);
}

#[test]
fn bucketed_deserialization_rejects_an_unknown_label() {
    use serde::de::value::{Error as DeError, StrDeserializer};

    let placed: StrDeserializer<DeError> = "co".into_deserializer();
    assert!(matches!(
        JurisdictionBucket::deserialize(placed),
        Ok(JurisdictionBucket::Jurisdiction(UsJurisdiction::Colorado))
    ));

    let unplaced: StrDeserializer<DeError> = "UNKNOWN".into_deserializer();
    assert!(matches!(
        JurisdictionBucket::deserialize(unplaced),
        Ok(JurisdictionBucket::Unplaced)
    ));

    let territory: StrDeserializer<DeError> = "PR".into_deserializer();
    assert!(JurisdictionBucket::deserialize(territory).is_err());
}

#[test]
fn meet_state_prints_the_code_or_the_store_sentinel() {
    let placed = MeetState::from(Some(UsJurisdiction::Wisconsin));
    assert_eq!(placed.code(), "WI");
    assert_eq!(placed.to_string(), "WI");
    assert_eq!(placed.jurisdiction(), Some(UsJurisdiction::Wisconsin));

    let unresolved = MeetState::from(None::<UsJurisdiction>);
    assert_eq!(unresolved, MeetState::Unresolved);
    assert_eq!(unresolved, MeetState::default());
    assert_eq!(unresolved.code(), crate::model::MEET_STATE_UNRESOLVED);
    assert_eq!(unresolved.jurisdiction(), None);
    assert!(placed < unresolved, "the unresolved label sorts last");

    // The bucket and the meet state are different vocabularies on purpose: a school with no state is
    // a missing fact (UNKNOWN), a meet with no venue state is the label the store always wrote (??).
    assert_ne!(JurisdictionBucket::Unplaced.code(), unresolved.code());
}

#[test]
fn meet_state_deserializes_the_sentinel_and_rejects_anything_else() {
    use serde::de::value::{Error as DeError, StrDeserializer};

    let sentinel: StrDeserializer<DeError> = "??".into_deserializer();
    assert!(matches!(
        MeetState::deserialize(sentinel),
        Ok(MeetState::Unresolved)
    ));

    let code: StrDeserializer<DeError> = "nv".into_deserializer();
    assert!(matches!(
        MeetState::deserialize(code),
        Ok(MeetState::Placed(UsJurisdiction::Nevada))
    ));

    let bucket_only: StrDeserializer<DeError> = "UNKNOWN".into_deserializer();
    assert!(MeetState::deserialize(bucket_only).is_err());
}
