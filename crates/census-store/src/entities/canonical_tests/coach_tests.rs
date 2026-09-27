use super::*;

#[test]
fn coach_email_classifier_handles_edge_domains() {
    use census_domain::model::published_email;

    assert_eq!(
        published_email("COACH@GMAIL.COM").map(|(_, kind)| kind),
        Some(MailboxKind::Personal)
    );
    assert_eq!(
        published_email("user@sub.gmail.com").map(|(_, kind)| kind),
        Some(MailboxKind::Personal)
    );
    for address in ["ad@notgmail.com", "ad@gmail.com.evil.org", "ad@school"] {
        assert_eq!(
            published_email(address).map(|(_, kind)| kind),
            Some(MailboxKind::Professional),
            "{address}"
        );
    }
    assert_eq!(published_email("user@@gmail.com"), None);
}

#[test]
fn coach_merge_classifies_incoming_addresses_and_keeps_first_of_each_kind() {
    let mut first = CanonicalCoach::new(
        &school(),
        "Dana Reed",
        None,
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    first.personal_email = Some("first@gmail.com".to_string());
    let mut second = first.clone();
    second.personal_email = Some("second@gmail.com".to_string());
    second.professional_email = Some("coach@school.org".to_string());

    first.merge(second);

    assert_eq!(first.personal_email.as_deref(), Some("first@gmail.com"));
    assert_eq!(
        first.professional_email.as_deref(),
        Some("coach@school.org")
    );
}

#[test]
fn coach_merge_routes_addresses_even_when_source_fields_are_reversed() {
    let mut first = CanonicalCoach::new(
        &school(),
        "Dana Reed",
        None,
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    first.professional_email = Some("coach@school.org".to_string());
    let mut second = first.clone();
    second.professional_email = Some("second@gmail.com".to_string());
    second.personal_email = Some("other@district.org".to_string());

    first.merge(second);

    assert_eq!(
        first.professional_email.as_deref(),
        Some("coach@school.org")
    );
    assert_eq!(first.personal_email.as_deref(), Some("second@gmail.com"));
}
