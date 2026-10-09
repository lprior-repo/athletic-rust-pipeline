use super::*;

#[test]
fn mark_raw_reports_the_published_value_or_its_unit() -> Result<(), Box<dyn std::error::Error>> {
    let imperial = |feet: &str, metres: i32| Mark::FieldImperial {
        feet_mark: feet.into(),
        metres: CentiMetres::new(metres),
    };
    let cases: &[(Mark, &str)] = &[
        (Mark::Raw("41-06.5".into()), "41-06.5"),
        (Mark::Raw(String::new()), ""),
        (Mark::Raw(" ".into()), " "),
        (Mark::TimeSeconds(ExactSeconds::parse("10.94")?), "time"),
        (Mark::DistanceMetres(CentiMetres::new(173)), "distance"),
        (imperial("5' 4\"", 163), "field"),
        (Mark::Points(CentiPoints::new(842100)), "points"),
    ];
    for (mark, raw) in cases {
        check!(eq; mark.raw(), *raw);
    }
    Ok(())
}

#[test]
fn published_email_classifies_domains_and_keeps_personal_mailboxes() {
    assert_eq!(published_email("@ofsd.k12.wi.us"), None);
    assert_eq!(published_email("coach@"), None);
    assert_eq!(published_email(""), None);
    assert_eq!(published_email("no-at-sign"), None);
    for domain in ["gmail.com", "GMAIL.com", "sub.gmail.com", "proton.me"] {
        let mailbox = format!("coach@{domain}");
        let published = published_email(&mailbox);
        assert_eq!(
            published,
            Some((mailbox.clone(), MailboxKind::Personal)),
            "{domain}"
        );
    }
    for domain in ["notgmail.com", "llhs.org", "gmail.com.evil.org"] {
        let address = format!("ad@{domain}");
        assert_eq!(
            published_email(&address),
            Some((address.clone(), MailboxKind::Professional)),
            "{domain}"
        );
    }
    let trimmed = published_email(" jstoik@ofsd.k12.wi.us ");
    assert_eq!(
        trimmed,
        Some((
            "jstoik@ofsd.k12.wi.us".to_string(),
            MailboxKind::Professional
        ))
    );
    let caps = published_email("AD@LLHS.ORG");
    assert_eq!(
        caps,
        Some(("AD@LLHS.ORG".to_string(), MailboxKind::Professional))
    );
}

#[test]
fn normalize_name_turns_separators_into_one_space() {
    for (raw, expected) in [
        ("A B", "a b"),
        ("a-b", "a b"),
        ("a'b", "a b"),
        ("a.b", "a b"),
        ("a,b", "a b"),
        ("a/b", "a b"),
        ("a_b", "ab"),
        ("Plain ASCII 123", "plain ascii 123"),
        ("  Glencoe-Silver   Lake HS ", "glencoe silver lake"),
    ] {
        assert_eq!(normalize_name(raw), expected, "raw {raw:?}");
    }
}

#[test]
fn normalize_name_folds_every_accented_arm() {
    let accented = "áàâäãåāéèêëēęíìîïīóòôöõōøúùûüūñńçćšśžźżýÿłæœß";
    let folded = "aaaaaaaeeeeeeiiiiiooooooouuuuunnccsszzzyylaos";
    assert_eq!(accented.chars().count(), folded.chars().count());
    for (raw, expected) in accented.chars().zip(folded.chars()) {
        let name = raw.to_string();
        assert_eq!(normalize_name(&name), expected.to_string(), "{raw}");
    }
    assert_eq!(normalize_name("Řeřicha School"), "eicha");
    assert_eq!(normalize_name("César Chávez School"), "cesar chavez");
}

#[test]
fn normalize_name_strips_school_suffixes_only_from_longer_names() {
    for (raw, expected) in [
        ("Abbotsford High School", "abbotsford"),
        ("Abbotsford Highschool", "abbotsford"),
        ("Abbotsford HS", "abbotsford"),
        ("Abbotsford School", "abbotsford"),
        ("Glencoe Sr High", "glencoe"),
        ("Glencoe Senior High", "glencoe"),
        ("Abbotsford Academy", "abbotsford academy"),
        ("HS", "hs"),
        ("Sr High", "sr high"),
    ] {
        assert_eq!(normalize_name(raw), expected, "raw {raw:?}");
    }
}

#[test]
fn normalize_name_reaches_a_fixpoint_on_repeated_suffixes() {
    for (raw, expected) in [
        ("X School School", "x"),
        ("Center Grove High School High School", "center grove"),
        ("A B HS School", "a b"),
        ("A B School Sr High", "a b"),
    ] {
        assert_eq!(normalize_name(raw), expected, "raw {raw:?}");
    }
    for raw in [
        "Abbotsford High School",
        "X School School",
        "A B School Sr High",
        "HS",
        "",
    ] {
        let once = normalize_name(raw);
        assert_eq!(normalize_name(&once), once, "not idempotent on {raw:?}");
    }
}

#[test]
fn flip_last_first_handles_both_shapes() {
    assert_eq!(flip_last_first("Aguilera, Julian"), "Julian Aguilera");
    assert_eq!(flip_last_first(" Aguilera , Julian "), "Julian Aguilera");
    assert_eq!(flip_last_first("Julian Aguilera"), "Julian Aguilera");
    assert_eq!(flip_last_first("Aguilera,"), "Aguilera,");
    assert_eq!(flip_last_first(", Julian"), ", Julian");
    assert_eq!(flip_last_first(","), ",");
}
