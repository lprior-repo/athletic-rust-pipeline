use crate::model::{
    normalize_name, published_email, CanonicalCoach, CanonicalSchool, CoachRole, Gender,
    MailboxKind,
};

const ADDRESS_BYTES: usize = 12;

fn any_address() -> String {
    let bytes: [u8; ADDRESS_BYTES] = kani::any();
    for byte in bytes {
        kani::assume(byte >= 32);
        kani::assume(byte <= 126);
    }
    String::from_utf8(bytes.to_vec()).expect("the modeled bytes are printable ASCII")
}

#[kani::proof]
#[kani::unwind(64)]
fn check_published_email_printable_ascii_contract() {
    let address = any_address();
    let trimmed = address.trim();
    let expected = trimmed.bytes().filter(|byte| *byte == b'@').count() == 1
        && !trimmed.starts_with('@')
        && !trimmed.ends_with('@');

    let published = published_email(&address);
    assert!(published.is_some() == expected);
    if let Some((normalized, _)) = published {
        assert!(normalized == trimmed);
    }
}

#[kani::proof]
#[kani::unwind(64)]
fn check_published_email_classifies_domains() {
    for address in [
        "coach@gmail.com",
        "user@googlemail.com",
        "user@outlook.com",
        "user@hotmail.com",
        "user@live.com",
        "user@msn.com",
        "user@yahoo.com",
        "user@aol.com",
        "user@icloud.com",
        "user@me.com",
        "user@protonmail.com",
        "user@proton.me",
        "user@sub.gmail.com",
        "  coach@GMAIL.COM  ",
        "coach@COMCAST.NET",
        "coach@sub.frontier.com",
    ] {
        assert!(matches!(
            published_email(address),
            Some((_, MailboxKind::Personal))
        ));
    }
    for address in [
        "coach@school.edu",
        "admin@university.org",
        "user@xoutlook.com",
        "a@b.gmail.example",
    ] {
        assert!(matches!(
            published_email(address),
            Some((_, MailboxKind::Professional))
        ));
    }
}

#[kani::proof]
#[kani::unwind(16)]
fn check_published_email_malformed() {
    let addresses = ["", "noatsign", "@nodomain", "local@", " @ ", "   "];
    let choice = usize::from(kani::any::<u8>());
    kani::assume(choice < addresses.len());
    kani::cover!(choice == 0, "empty address");
    kani::cover!(choice == 1, "missing separator");
    kani::cover!(choice == 2, "missing local part");
    kani::cover!(choice == 3, "missing domain");
    kani::cover!(choice == 4, "whitespace around separator");
    kani::cover!(choice == 5, "whitespace only");
    assert!(published_email(addresses[choice]).is_none());
}

fn proof_coach() -> CanonicalCoach {
    let school = CanonicalSchool::mint(
        crate::UsJurisdiction::Wisconsin,
        "Test High School",
        "test high school",
    );
    CanonicalCoach::new(&school, "Coach", None, Gender::Boys, CoachRole::HeadCoach)
}

#[kani::proof]
#[kani::unwind(64)]
fn check_set_published_email_routes_by_kind() {
    for address in [
        "coach@gmail.com",
        "coach@school.edu",
        "  coach@GMAIL.COM  ",
        "noatsign",
    ] {
        let expected = published_email(address);
        let mut coach = proof_coach();
        coach.set_published_email(address);

        match expected {
            Some((normalized, MailboxKind::Professional)) => {
                assert_eq!(
                    coach.professional_email.as_deref(),
                    Some(normalized.as_str())
                );
                assert_eq!(coach.personal_email, None);
            }
            Some((normalized, MailboxKind::Personal)) => {
                assert_eq!(coach.personal_email.as_deref(), Some(normalized.as_str()));
                assert_eq!(coach.professional_email, None);
            }
            None => {
                assert_eq!(coach.professional_email, None);
                assert_eq!(coach.personal_email, None);
            }
        }
    }
}

#[kani::proof]
#[kani::unwind(64)]
fn check_set_published_email_idempotent() {
    let address = any_address();
    let mut coach = proof_coach();
    coach.set_published_email(&address);
    let once = coach.clone();
    coach.set_published_email(&address);
    assert_eq!(
        coach, once,
        "routing the same address twice must be idempotent"
    );
}

#[kani::proof]
#[kani::unwind(64)]
fn check_normalize_diacritics() {
    assert_eq!(normalize_name("M\u{00fc}nchen"), normalize_name("Munchen"));
    assert_eq!(normalize_name("Caf\u{00e9}"), normalize_name("Cafe"));
    assert_eq!(
        normalize_name("\u{0141}\u{00f3}d\u{017a}"),
        normalize_name("Lodz")
    );

    assert_eq!(normalize_name("M\u{00fc}nchen"), "munchen");
    assert_eq!(normalize_name("Caf\u{00e9}"), "cafe");
    assert_eq!(normalize_name("MADRID"), "madrid");
}

#[kani::proof]
#[kani::unwind(64)]
fn check_normalize_shape() {
    for raw in [
        "  John O'Brien, Jr.  ",
        "  St. Mary's High School  ",
        "  --Test--Name--  ",
        "Lady of Lourdes Academy",
        "M\u{00fc}nchen",
        "",
        "   ",
    ] {
        let key = normalize_name(raw);
        assert!(
            key.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == ' '),
            "{raw:?} normalized to {key:?}, which is not a lowercase-ASCII key"
        );
        assert_eq!(key, key.trim(), "normalized key has an edge space");
        assert!(
            !key.contains("  "),
            "{raw:?} normalized to {key:?} with a double space"
        );
    }
}

#[kani::proof]
#[kani::unwind(64)]
fn check_normalize_idempotent() {
    for raw in [
        "  John O'Brien, Jr.  ",
        "  St. Mary's High School  ",
        "MADRID",
        "M\u{00fc}nchen",
        "Caf\u{00e9}",
        "\u{0141}\u{00f3}d\u{017a}",
        "  --Test--Name--  ",
        "Appleton West High School",
        "Lady of Lourdes Academy",
        "",
        "   ",
    ] {
        let once = normalize_name(raw);
        let twice = normalize_name(&once);
        assert!(
            once == twice,
            "normalize_name not idempotent on {raw:?}: {once:?} then {twice:?}"
        );
    }
}

#[kani::proof]
#[kani::unwind(64)]
fn check_normalize_idempotent_repeated_suffix() {
    let once = normalize_name("x school school");
    let twice = normalize_name(&once);
    assert_eq!(
        once, twice,
        "normalize_name not idempotent: {once:?} then {twice:?}"
    );
}
