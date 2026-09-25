//! Kani proof harnesses for `published_email` and `normalize_name`.
//!
//! `kani::any::<String>()` has no `Arbitrary` impl, so the arbitrary address is built from a
//! bounded byte array. The address bytes are assumed printable ASCII: that is the input contract
//! exercised by the symbolic harness, and it keeps UTF-8 decoding out of CBMC. `normalize_name` is
//! exercised through value tables rather than symbolically: it calls `str::to_lowercase`, which
//! pulls the Unicode case-mapping tables into the harness, and the concrete harness whose only
//! input is `"Cafe"` was still inside CBMC after four minutes on this machine.

use crate::model::{
    published_email, CanonicalCoach, CanonicalSchool, CoachRole, Gender, MailboxKind,
    CONSUMER_MAIL_DOMAINS, normalize_name,
};

/// Address bytes handed to the symbolic harness: long enough for `local@domain.tld` and for the
/// `.<consumer>` subdomain case.
const ADDRESS_BYTES: usize = 12;

/// An arbitrary address over printable ASCII.
fn any_address() -> String {
    let bytes: [u8; ADDRESS_BYTES] = kani::any();
    let mut address = String::with_capacity(ADDRESS_BYTES);
    for byte in bytes {
        kani::assume(byte >= 32);
        kani::assume(byte <= 126);
        address.push(char::from(byte));
    }
    address
}

/// `published_email` never panics for printable ASCII and returns `Some` exactly when the trimmed
/// address has a non-empty local part and domain.
#[kani::proof]
#[kani::unwind(64)]
fn check_published_email_printable_ascii_contract() {
    let address = any_address();
    let trimmed = address.trim();
    let expected = match trimmed.split_once('@') {
        Some((local, domain)) => !local.is_empty() && !domain.is_empty(),
        None => false,
    };

    let published = published_email(&address);
    assert_eq!(
        published.is_some(),
        expected,
        "published_email accepted the wrong printable-ASCII shape: {address:?}"
    );
    if let Some((normalized, _)) = published {
        assert_eq!(normalized, trimmed, "published_email must trim its input");
    }
}

/// Every listed consumer domain is personal, while a domain outside the list is professional.
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
    ] {
        assert_eq!(
            published_email(address).map(|(_, kind)| kind),
            Some(MailboxKind::Personal),
            "{address} must classify as personal"
        );
    }
    for address in [
        "coach@school.edu",
        "admin@university.org",
        "user@xoutlook.com",
        "a@b.gmail.example",
    ] {
        assert_eq!(
            published_email(address).map(|(_, kind)| kind),
            Some(MailboxKind::Professional),
            "{address} must classify as professional"
        );
    }
    assert_eq!(
        CONSUMER_MAIL_DOMAINS.len(),
        12,
        "the concrete table covers the configured consumer domains"
    );
}

/// Empty local parts, empty domains and missing separators are malformed.
#[kani::proof]
#[kani::unwind(64)]
fn check_published_email_malformed() {
    for address in ["", "noatsign", "@nodomain", "local@", " @ ", "   "] {
        assert!(
            published_email(address).is_none(),
            "{address:?} is malformed"
        );
    }
}

fn proof_coach() -> CanonicalCoach {
    let school = CanonicalSchool::mint(crate::UsJurisdiction::Wisconsin, "Test High School", "test high school");
    CanonicalCoach::new(
        &school,
        "Coach",
        None,
        Gender::Boys,
        CoachRole::HeadCoach,
    )
}

/// A published address occupies exactly one field, selected by its domain rather than its caller's
/// initial field.
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
                assert_eq!(coach.professional_email.as_deref(), Some(normalized.as_str()));
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

/// Routing an address again is a no-op.
#[kani::proof]
#[kani::unwind(64)]
fn check_set_published_email_idempotent() {
    let address = any_address();
    let mut coach = proof_coach();
    coach.set_published_email(&address);
    let once = coach.clone();
    coach.set_published_email(&address);
    assert_eq!(coach, once, "routing the same address twice must be idempotent");
}

/// The comparison key folds case and diacritics: an accented and an unaccented spelling of the
/// same name produce the same key, and that key is the ASCII lowercase form.
#[kani::proof]
#[kani::unwind(64)]
fn check_normalize_diacritics() {
    assert_eq!(normalize_name("M\u{00fc}nchen"), normalize_name("Munchen"));
    assert_eq!(normalize_name("Caf\u{00e9}"), normalize_name("Cafe"));
    assert_eq!(normalize_name("\u{0141}\u{00f3}d\u{017a}"), normalize_name("Lodz"));

    assert_eq!(normalize_name("M\u{00fc}nchen"), "munchen");
    assert_eq!(normalize_name("Caf\u{00e9}"), "cafe");
    assert_eq!(normalize_name("MADRID"), "madrid");
}

/// The key is canonical in shape: lowercase ASCII alphanumerics separated by single spaces, with no
/// leading or trailing space.
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

/// Re-normalizing a key does not change it, for the name shapes the report carries.
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

/// A name whose normalized form ends in a second type suffix has to reach the same key as the name
/// without it: `"x school school"` normalizes to `"x"` in one call, so a second call is the
/// identity.
///
/// This is the repeated-suffix arm of the idempotency claim the pack publishes. It was a live
/// counterexample in the first pass (`"x school school"` normalized to `"x school"`, which
/// normalized again to `"x"`, and `SchoolId::mint` keys on that string); `normalize_name` now
/// strips type suffixes until none applies, which is the fixpoint this harness asserts.
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
