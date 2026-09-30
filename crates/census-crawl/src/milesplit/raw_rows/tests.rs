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
