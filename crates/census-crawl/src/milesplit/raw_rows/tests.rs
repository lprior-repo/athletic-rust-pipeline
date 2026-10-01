use super::*;

#[test]
fn apostrophe_prefixes_carry_gender() {
    for (label, gender) in [
        ("Girls' Javelin 6A", Gender::Girls),
        ("Girls' 300 Hurdles", Gender::Girls),
        ("Women's Shot Put", Gender::Girls),
        ("Boys' Discus", Gender::Boys),
        ("Girls 100 Meter Dash", Gender::Girls),
        ("Flight 1 of 1", Gender::Unknown),
    ] {
        let section = section_of(label, None);
        assert_eq!(section.gender, gender, "label {label:?}");
    }
}

#[test]
fn qualified_field_labels_map_to_their_kind() {
    let section = section_of("Girls' Javelin 6A", None);
    assert_eq!(section.kind, EventKind::Javelin);
    assert_eq!(section.label, "Girls' Javelin 6A");
}

#[test]
fn escaped_entities_decode_in_labels() {
    assert_eq!(html_unescape("Girls&#039; Javelin"), "Girls' Javelin");
    assert_eq!(html_unescape("A&amp;B \u{2014} C"), "A&B \u{2014} C");
    assert_eq!(html_unescape("x&#x27;y"), "x'y");
    assert_eq!(html_unescape("100 &lt; 200"), "100 < 200");
    assert_eq!(html_unescape("bad &#zzz; entity"), "bad &#zzz; entity");
}

#[test]
fn numeric_references_decode_decimal_and_hex_forms() {
    for (encoded, decoded) in [
        ("Girls&#39; Javelin", "Girls' Javelin"),
        ("O&#039;Connell, Kenzie", "O'Connell, Kenzie"),
        ("18&#176; and rising", "18\u{b0} and rising"),
        ("200m&#x2014;final", "200m\u{2014}final"),
        ("R&amp;D &#38; Sons", "R&D & Sons"),
    ] {
        assert_eq!(html_unescape(encoded), decoded, "encoded {encoded:?}");
    }
}

#[test]
fn malformed_references_stay_verbatim() {
    for encoded in [
        "&#;",
        "&#zzz;",
        "&#1114112;",
        "&#xD800;",
        "&#123456789;",
        "&#x",
        "Fish & Chips",
    ] {
        assert_eq!(html_unescape(encoded), encoded, "encoded {encoded:?}");
    }
}
