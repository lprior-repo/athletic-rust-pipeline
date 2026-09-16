use super::*;

#[test]
fn digest_requires_lowercase_sha256_hex() {
    let valid = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let invalid = valid[..62].to_owned() + "GG";
    let digest = WorkbookDigest::parse(valid).expect("digest");
    assert_eq!(digest.as_str(), valid);
    assert!(WorkbookDigest::parse(&valid[..63]).is_err());
    assert!(WorkbookDigest::parse(&invalid).is_err());
    assert!(EvidenceDigest::parse(&valid.to_uppercase()).is_err());
}

#[test]
fn source_row_key_uses_last_colon_and_preserves_sheet_spelling() {
    let key = SourceRowKey::parse("Meet:Indoor:2").expect("valid source key");
    assert_eq!(key.as_str(), "Meet:Indoor:2");
    assert_eq!(key.sheet(), "Meet:Indoor");
    assert_eq!(key.row(), 2);
    assert_ne!(
        SourceRowKey::parse("Export:2").expect("key").sheet(),
        SourceRowKey::parse("export:2").expect("key").sheet()
    );
    assert!(SourceRowKey::parse("Export:1").is_err());
    assert!(SourceRowKey::parse("Export:nope").is_err());
}

#[test]
fn identity_serde_roundtrips_only_valid_values() {
    let key = SourceRowKey::parse("Sheet1:27").expect("valid key");
    let encoded = serde_json::to_string(&key).expect("serialize key");
    assert_eq!(encoded, "\"Sheet1:27\"");
    let decoded: SourceRowKey = serde_json::from_str(&encoded).expect("deserialize key");
    assert_eq!(decoded, key);
    assert!(serde_json::from_str::<AthleteId>("0").is_err());
    assert!(serde_json::from_str::<SourceRowKey>("\"Sheet1:1\"").is_err());
}

#[test]
fn profile_url_canonicalizes_host_default_port_and_trailing_slash() {
    let base = ProfileUrl::parse("https://www.athletic.net:443/athlete/0007/").expect("base");
    assert_eq!(base.as_str(), "https://athletic.net/athlete/7");
    assert_eq!(base.athlete_id().get(), 7);
    let tf = ProfileUrl::parse("https://athletic.net/athlete/7/track-and-field/").expect("tf");
    assert_eq!(
        tf.as_str(),
        "https://athletic.net/athlete/7/track-and-field"
    );
    let xc = ProfileUrl::parse("https://athletic.net/athlete/7/cross-country").expect("xc");
    assert_eq!(xc.as_str(), "https://athletic.net/athlete/7/cross-country");
}

#[test]
fn profile_url_rejects_untrusted_provenance_and_unknown_routes() {
    for raw in [
        "http://athletic.net/athlete/7",
        "https://evil.example/athlete/7",
        "https://athletic.net:8443/athlete/7",
        "https://user@athletic.net/athlete/7",
        "https://athletic.net/athlete/0",
        "https://athletic.net/athlete/7/results",
        "https://athletic.net/athlete/7?tab=results",
        "https://athletic.net/athlete/7#results",
    ] {
        assert!(ProfileUrl::parse(raw).is_err(), "accepted {raw}");
    }
}
