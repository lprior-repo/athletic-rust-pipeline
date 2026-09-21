//! Kani proof harnesses for `professional_email` and `normalize_name`.
//!
//! `kani::any::<String>()` has no `Arbitrary` impl, so the arbitrary address is built from a
//! bounded byte array through `String::from_utf8_lossy` instead. `normalize_name` is exercised
//! through value tables rather than symbolically: it calls `str::to_lowercase`, which pulls the
//! Unicode case-mapping tables into the harness, and the concrete harness whose only input is
//! `"Cafe"` was still inside CBMC after four minutes on this machine. The tables state the rules
//! the function documents (case, diacritics, punctuation, whitespace, type suffixes) with the
//! values observed from the current model.

use crate::model::{CONSUMER_MAIL_DOMAINS, normalize_name, professional_email};

/// Address bytes handed to the symbolic harness: long enough for `local@domain.tld` and for the
/// `.<consumer>` subdomain case.
const ADDRESS_BYTES: usize = 12;

/// An arbitrary address over printable ASCII.
///
/// The contract compares ASCII domains and `CONSUMER_MAIL_DOMAINS` is an ASCII list, so restricting
/// the arbitrary address to printable ASCII loses nothing a mailbox can be - and it keeps
/// `str::from_utf8_lossy`'s byte-by-byte replacement decoding out of CBMC.
fn any_address() -> String {
    let bytes: [u8; ADDRESS_BYTES] = kani::any();
    let mut address = String::with_capacity(ADDRESS_BYTES);
    for byte in bytes {
        address.push(char::from((byte % 95) + 32));
    }
    address
}

/// Whatever `professional_email` publishes is the trimmed input, is stable under re-publication,
/// and its domain is not a consumer mailbox - not even as a `.<consumer>` suffix.
#[kani::proof]
#[kani::unwind(64)]
fn check_professional_email_never_publishes_consumer_mailbox() {
    let address = any_address();

    let Some(published) = professional_email(&address) else {
        return;
    };

    assert_eq!(
        published,
        address.trim(),
        "publish must return the trimmed address"
    );
    assert_eq!(
        professional_email(&published),
        Some(published.clone()),
        "publish must be closed under re-publication"
    );

    let (_, domain) = published
        .split_once('@')
        .expect("a published address carries a domain");
    let domain = domain.to_ascii_lowercase();
    for base in CONSUMER_MAIL_DOMAINS {
        let exact = domain == base;
        let subdomain = domain
            .strip_suffix(base)
            .is_some_and(|prefix| prefix.ends_with('.'));
        assert!(
            !exact && !subdomain,
            "a personal mailbox was published: {published}"
        );
    }
}

/// Every consumer mailbox in the crate's list is withheld, including subdomains of one and a
/// mixed-case spelling of one.
#[kani::proof]
#[kani::unwind(64)]
fn check_professional_email_known_consumer() {
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
        assert!(
            professional_email(address).is_none(),
            "{address} is a personal mailbox and must be withheld"
        );
    }
}

/// A school/association address survives, trimmed of surrounding whitespace, and a domain that
/// merely contains a consumer name is not one.
#[kani::proof]
#[kani::unwind(64)]
fn check_professional_email_known_professional() {
    for address in [
        "coach@school.edu",
        "admin@university.org",
        "user@xoutlook.com",
        "a@b.gmail.example",
    ] {
        assert_eq!(
            professional_email(address),
            Some(address.to_string()),
            "{address} is not a consumer mailbox and must be published"
        );
    }
    assert_eq!(
        professional_email("  coach@school.edu  "),
        Some("coach@school.edu".to_string()),
        "publish must trim the address"
    );
}

/// Malformed addresses are withheld rather than repaired.
#[kani::proof]
#[kani::unwind(64)]
fn check_professional_email_malformed() {
    for address in ["", "noatsign", "@nodomain", "local@", " @ ", "   "] {
        assert!(
            professional_email(address).is_none(),
            "{address:?} is malformed and must be withheld"
        );
    }
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

/// A name whose normalized form still ends in a dropped type suffix loses another token on the next
/// pass, so re-normalizing a key is not always the identity.
///
/// This harness asserts the universal claim the pack published ("`normalize_name` idempotency") and
/// is expected to fail on the current model: `"x school school"` normalizes to `"x school"`, and
/// that normalizes again to `"x"`. The repair belongs in `src/model.rs`, which this lane does not
/// own; the failure is reported verbatim in `docs/VERIFICATION-EVIDENCE.md`.
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
