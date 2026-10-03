use super::{
    clean_text, is_varsity_level, is_vendor_contact, is_vendor_fixture, is_vendor_school,
    level_label, sanitize_person, sanitize_school, UNSTATED_LEVEL, VARSITY_LEVEL,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_executed_person_table_survives_the_port() -> TestResult {
    let cases: [(&str, Option<&str>); 23] = [
        ("Ann Greenfield (Athletic Director)", Some("Ann Greenfield")),
        ("Athletic Director", None),
        ("Principal Laurie Kolling", None),
        ("Dean Adams", Some("Dean Adams")),
        ("Dean of Students", None),
        ("Jane Doe, Head Coach", Some("Jane Doe")),
        ("Coach", None),
        ("John  Smith", Some("John Smith")),
        ("Ann Greenfield (assistant coach)", Some("Ann Greenfield")),
        ("Nurse Pat", None),
        ("Ann Greenfield (AD)", Some("Ann Greenfield (AD)")),
        ("Dean, Jane", None),
        ("Coach Bob Smith", Some("Coach Bob Smith")),
        ("John Smith Head Coach", Some("John Smith")),
        ("Head Coach", None),
        ("Assistant Coach", None),
        ("Mary Jones (Head Coach)", Some("Mary Jones")),
        ("Director of Athletics", None),
        ("The Coach", None),
        ("Bob Smith, Athletic Director", Some("Bob Smith")),
        ("Jose Ruiz (Assistant Coach)", Some("Jose Ruiz")),
        ("Jane Doe (Coach)", Some("Jane Doe")),
        ("Dean", None),
    ];
    for (input, expected) in cases {
        let actual = sanitize_person(input)?;
        if actual.as_deref() != expected {
            return Err(format!(
                "sanitize_person({input:?}): left: {:?}, right: {expected:?}",
                actual.as_deref()
            )
            .into());
        }
    }
    Ok(())
}

#[test]
fn the_executed_vendor_table_survives_the_port() -> TestResult {
    let cases: [(&str, &str, bool); 8] = [
        ("NC Test School 1", "", true),
        ("DF Test School 1", "", true),
        ("Test School", "x@a.org", true),
        ("Contest School", "", false),
        (
            "Madison West High School",
            "ad@dragonflyathletics.com",
            true,
        ),
        ("Madison West High School", "X@DRAGONFLYATHLETICS.COM", true),
        ("Madison West High School", "ad@school.org", false),
        ("Test  School 2", "", true),
    ];
    for (school, email, expected) in cases {
        let actual = is_vendor_fixture(school, email)?;
        if actual != expected {
            return Err(format!(
                "is_vendor_fixture({school:?}, {email:?}): left: {actual:?}, right: {expected:?}"
            )
            .into());
        }
    }
    if !is_vendor_contact("ad@dragonflyathletics.com") {
        return Err("is_vendor_contact(\"ad@dragonflyathletics.com\") must be true".into());
    }
    if is_vendor_contact("ad@school.org") {
        return Err("is_vendor_contact(\"ad@school.org\") must be false".into());
    }
    if !is_vendor_school("Megan's Test School")? {
        return Err("is_vendor_school(\"Megan's Test School\") must be true".into());
    }
    if is_vendor_school("Test Schoolhouse")? {
        return Err("is_vendor_school(\"Test Schoolhouse\") must be false".into());
    }
    Ok(())
}

#[test]
fn cleaning_collapses_every_whitespace_run_the_prototype_collapsed() {
    assert_eq!(clean_text("A\u{a0}B\u{a0}\u{a0}C"), "A B C");
    assert_eq!(clean_text("\u{a0}\u{a0}A\u{3000}B \t C\u{2003}"), "A B C");
    assert_eq!(clean_text("John  Smith"), "John Smith");
    assert_eq!(clean_text("  John Smith  "), "John Smith");
    assert_eq!(clean_text(""), "");
    assert_eq!(clean_text("\u{a0}\u{2009}"), "");
    assert_eq!(clean_text("A\nB\r\nC"), "A B C");
}

#[test]
fn a_school_name_is_dropped_when_it_is_empty_or_a_vendor_fixture() -> TestResult {
    check!(eq; sanitize_school("")?, None);
    check!(eq; sanitize_school("   ")?, None);
    check!(eq;
        sanitize_school("\u{a0}\u{a0}")?,
        None
    );
    check!(eq;
        sanitize_school("NC Test School 1")?,
        None
    );
    check!(eq;
        sanitize_school("  Madison  West High School ")?,
        Some("Madison West High School".to_string())
    );
    check!(eq; sanitize_person("  ")?, None);
    Ok(())
}

#[test]
fn only_an_unstated_or_varsity_level_survives_the_scope_filter() {
    assert!(is_varsity_level(None));
    assert!(is_varsity_level(Some("")));
    assert!(is_varsity_level(Some(VARSITY_LEVEL)));
    assert!(!is_varsity_level(Some("Junior High")));
    assert!(!is_varsity_level(Some("Junior Varsity")));
    assert!(!is_varsity_level(Some("JV")));
    assert!(!is_varsity_level(Some("Freshman")));
    assert!(!is_varsity_level(Some("varsity")));
    assert_eq!(level_label(None), UNSTATED_LEVEL);
    assert_eq!(level_label(Some("  ")), UNSTATED_LEVEL);
    assert_eq!(level_label(Some("Junior  High")), "Junior High");
}
