use crate::school_directory::*;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

fn reject<T: DeserializeOwned + Debug>(json: &str, message: &str) {
    let decoded = serde_json::from_str::<T>(json);
    println!("{} {json} => {decoded:?}", std::any::type_name::<T>());
    let error = decoded.expect_err("invalid persisted value must reject");
    assert!(error.is_data());
    assert!(error.to_string().contains(message), "{error}");
}

fn preserve<T: DeserializeOwned + Serialize + Debug>(json: &str) {
    let value: T = serde_json::from_str(json).expect("valid persisted value");
    assert_eq!(serde_json::to_vec(&value).expect("encode"), json.as_bytes());
}

#[test]
fn grade_span_inversion_and_unrankable_endpoints_reject() {
    reject::<GradeSpan>(
        r#"{"low":{"Numbered":12},"high":{"Numbered":1}}"#,
        "inverted",
    );
    reject::<GradeSpan>(
        r#"{"low":"Ungraded","high":{"Numbered":12}}"#,
        "not a supported grade",
    );
    reject::<GradeSpan>(
        r#"{"low":"PreK","high":"AdultEducation"}"#,
        "not a supported grade",
    );
    preserve::<GradeSpan>(r#"{"low":"PreK","high":{"Numbered":12}}"#);
    preserve::<GradeSpan>(r#"{"low":{"Numbered":12},"high":{"Numbered":12}}"#);
}

#[test]
fn nces_identity_rejects_malformed_and_noncanonical_bytes() {
    for json in [r#""123""#, r#"" 010000500870""#, r#""01000050087A""#] {
        reject::<NcesSchoolId>(json, "NCES school id");
    }
    preserve::<NcesSchoolId>(r#""010000500870""#);
}

#[test]
fn pss_identity_rejects_malformed_and_noncanonical_bytes() {
    for json in [r#""A238000""#, r#""a2380006""#, r#"" A2380006""#] {
        reject::<PssId>(json, "PSS id");
    }
    preserve::<PssId>(r#""A2380006""#);
}

#[test]
fn state_identity_rejects_empty_whitespace_and_controls() {
    for json in [r#""""#, r#""a b""#, r#"" a""#, r#""a\u0000""#] {
        reject::<StateRecordId>(json, "state record id");
    }
    reject::<StateRecordId>(
        &serde_json::to_string(&"a".repeat(65)).expect("json"),
        "64-character",
    );
    preserve::<StateRecordId>(&serde_json::to_string(&"a".repeat(64)).expect("json"));
    preserve::<StateRecordId>(r#""800000038718""#);
}

#[test]
fn phone_rejects_empty_digit_bounds_and_noncanonical_whitespace() {
    for json in [
        r#""""#,
        r#""123456""#,
        r#""1234567890123456""#,
        r#""123  4567""#,
        r#""1234567\u0000""#,
    ] {
        reject::<Phone>(json, "phone");
    }
    preserve::<Phone>(r#""1234567""#);
    preserve::<Phone>(r#""123456789012345""#);
    preserve::<Phone>(r#""(256)878-2341""#);
    reject::<Phone>(
        &serde_json::to_string(&format!("{}1234567", "x".repeat(26))).expect("json"),
        "32-character",
    );
    preserve::<Phone>(&serde_json::to_string(&format!("{}1234567", "x".repeat(25))).expect("json"));
}

#[test]
fn website_rejects_empty_scheme_whitespace_and_length() {
    for json in [
        r#""""#,
        r#""ftp://school.edu""#,
        r#"" https://school.edu""#,
        r#""https://school .edu""#,
        r#""https://school.edu\u0000""#,
    ] {
        reject::<Website>(json, "website");
    }
    preserve::<Website>(r#""https://school.edu""#);
    preserve::<Website>(
        &serde_json::to_string(&format!("https://{}", "a".repeat(192))).expect("json"),
    );
    reject::<Website>(
        &serde_json::to_string(&format!("https://{}", "a".repeat(193))).expect("json"),
        "200-character",
    );
}

#[test]
fn latitude_rejects_beyond_both_geographic_limits() {
    for json in ["-900000001", "900000001", "2147483647"] {
        reject::<Latitude>(json, "latitude");
    }
    for json in ["-900000000", "0", "900000000"] {
        preserve::<Latitude>(json);
    }
}

#[test]
fn longitude_rejects_beyond_both_geographic_limits() {
    for json in ["-1800000001", "1800000001", "2147483647"] {
        reject::<Longitude>(json, "longitude");
    }
    for json in ["-1800000000", "0", "1800000000"] {
        preserve::<Longitude>(json);
    }
}

#[test]
fn zipcode_rejects_invalid_components_without_losing_leading_zero() {
    for json in [
        r#"{"code":"1234"}"#,
        r#"{"code":" 07030"}"#,
        r#"{"code":"07030","plus4":""}"#,
        r#"{"code":"07030","plus4":"123"}"#,
        r#"{"code":"07030","plus4":"12a4"}"#,
    ] {
        reject::<ZipCode>(json, "ZIP");
    }
    preserve::<ZipCode>(r#"{"code":"07030"}"#);
    preserve::<ZipCode>(r#"{"code":"07030","plus4":"0001"}"#);
}

#[test]
fn postal_address_rejects_present_empty_and_invalid_components() {
    reject::<PostalAddress>("{}", "postal address");
    reject::<PostalAddress>(r#"{"city":""}"#, "city");
    preserve::<PostalAddress>(r#"{"state":"NJ","zip":{"code":"07030"}}"#);
}

#[test]
fn coordinates_reject_invalid_required_components() {
    reject::<Coordinates>(r#"{"latitude":900000001,"longitude":0}"#, "latitude");
    reject::<Coordinates>(r#"{"latitude":0,"longitude":1800000001}"#, "longitude");
    reject::<Coordinates>(r#"{"latitude":0}"#, "missing field");
    preserve::<Coordinates>(r#"{"latitude":-900000000,"longitude":1800000000}"#);
}

#[test]
fn association_label_rejects_empty_controls_whitespace_and_length() {
    for json in [r#""""#, r#"" NAIS""#, r#""NAIS\u0000""#] {
        reject::<AssociationLabel>(json, "association");
    }
    reject::<AssociationLabel>(
        &serde_json::to_string(&"a".repeat(49)).expect("json"),
        "48-character",
    );
    preserve::<AssociationLabel>(&serde_json::to_string(&"a".repeat(48)).expect("json"));
    preserve::<AssociationLabel>(r#""NAIS""#);
}

#[test]
fn matching_form_rejects_noncanonical_bytes() {
    for json in [
        r#""School""#,
        r#""school!""#,
        r#"" school""#,
        r#""school  name""#,
    ] {
        reject::<MatchForm>(json, "matching form");
    }
    preserve::<MatchForm>(r#""school name""#);
    preserve::<MatchForm>(r#""""#);
}

#[test]
fn street_rejects_empty_noncanonical_and_overlong_bytes() {
    for json in [
        r#""""#,
        r#""123 MAIN ST""#,
        r#"" Main St""#,
        r#""Main\u0000""#,
    ] {
        reject::<StreetLine>(json, "street");
    }
    reject::<StreetLine>(
        &serde_json::to_string(&"a".repeat(121)).expect("json"),
        "120-character",
    );
    preserve::<StreetLine>(&serde_json::to_string(&"a".repeat(120)).expect("json"));
    preserve::<StreetLine>(r#""123 Main St""#);
}

#[test]
fn city_rejects_empty_noncanonical_and_overlong_bytes() {
    for json in [r#""""#, r#""NEW YORK""#, r#"" New York""#, r#""New\u0000""#] {
        reject::<CityName>(json, "city");
    }
    reject::<CityName>(
        &serde_json::to_string(&"a".repeat(65)).expect("json"),
        "64-character",
    );
    preserve::<CityName>(&serde_json::to_string(&"a".repeat(64)).expect("json"));
    preserve::<CityName>(r#""New York""#);
}

#[test]
fn weak_identity_rejects_empty_name_or_present_empty_city() {
    reject::<WeakKey>(r#"{"name":""}"#, "matching name");
    reject::<WeakKey>(r#"{"name":"school","city":""}"#, "matching city");
    preserve::<WeakKey>(r#"{"name":"school","city":"new york","state":"NY"}"#);
    preserve::<WeakKey>(r#"{"name":"school"}"#);
}

#[test]
fn canonical_matching_case_expansion_roundtrips_at_the_checked_boundary() {
    let form = MatchForm::of("İ");
    let form_json = serde_json::to_string(&form).expect("form");
    println!("case expansion form={form_json}");
    preserve::<MatchForm>(&form_json);
}

#[test]
fn canonical_presentation_case_expansion_roundtrips_at_the_checked_boundary() {
    let street = StreetLine::parse("ß").expect("street");
    let city = CityName::parse("ß").expect("city");
    let street_json = serde_json::to_string(&street).expect("street");
    let city_json = serde_json::to_string(&city).expect("city");
    println!("case expansion street={street_json}, city={city_json}");
    preserve::<StreetLine>(&street_json);
    preserve::<CityName>(&city_json);
}
